import Flutter
import Foundation
import UIKit
import UniformTypeIdentifiers

/// The folder channel on the iPhone (#240), answering the calls of
/// FolderChannel.kt and MainActivity.kt with the same arguments and
/// answers. The folder comes from the Files app's picker, so iCloud
/// Drive, Möbius Sync and other file providers work; it is kept as a
/// security-scoped bookmark under a key of its own ([FolderRules]),
/// resolved on every call. The background sync's engine answers the
/// same calls, all but the pickers ([picking]).
enum FolderChannel {
  static let name = "io.github.paxel.oldbrainfix/folder"

  private static let calls: Set<String> = [
    "name", "list", "read", "write", "put", "delete", "ensure", "autoBackupDir", "autoBackup",
    "files", "readDoc", "release", "zipNames", "unzip", "dropUnzipped",
  ]

  @discardableResult
  static func register(messenger: FlutterBinaryMessenger, picking: Bool) -> FlutterMethodChannel {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      if picking && call.method == "pickTree" {
        FolderPicker.shared.pickFolder(result)
      } else if picking && call.method == "pickZip" {
        FolderPicker.shared.pickZip(result)
      } else if calls.contains(call.method) {
        let args = call.arguments as? [String: Any] ?? [:]
        Folders.shared.answer(call.method, args) { answer in
          DispatchQueue.main.async {
            switch answer {
            case .success(let value):
              result(value)
            case .failure(let error):
              result(FlutterError(code: "folder", message: "\(error)", details: nil))
            }
          }
        }
      } else {
        result(FlutterMethodNotImplemented)
      }
    }
    return channel
  }
}

struct FolderFailure: Error, CustomStringConvertible {
  let description: String
  init(_ description: String) { self.description = description }
}

/// The folders and files themselves, on one queue, one call after the
/// other, as on Android. Each call resolves the folder's bookmark,
/// starts its access and stops it again; reads and writes go through
/// NSFileCoordinator, so a file provider such as iCloud Drive downloads
/// a file first and takes what was written as a whole.
final class Folders {
  static let shared = Folders()

  private let io = DispatchQueue(label: "io.github.paxel.oldbrainfix.folder")
  private let files = FileManager.default

  /// The bookmarks by key, read from [bookmarksFile] on first use.
  private var marks: [String: Data]?

  /// The zips the picker copied into the app, deleted with the files
  /// copied out of them.
  private var zips: [URL] = []

  private var support: URL {
    files.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
  }

  private var bookmarksFile: URL { support.appendingPathComponent("folders.json") }

  /// Where [unzip] puts the files it copies out of a zip.
  private var unzipped: URL {
    files.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent("import-zip")
  }

  func answer(
    _ method: String, _ args: [String: Any], done: @escaping (Result<Any?, Error>) -> Void
  ) {
    io.async { done(Result(catching: { try self.answerNow(method, args) })) }
  }

  /// Keeps the folder [url] the picker granted; [done] hears its key, or
  /// nil when no bookmark could be made. Run on the main queue, while the
  /// picker's grant is fresh.
  func keep(_ url: URL, done: @escaping (String?) -> Void) {
    let scoped = url.startAccessingSecurityScopedResource()
    let data = try? url.bookmarkData(
      options: .minimalBookmark, includingResourceValuesForKeys: [.nameKey], relativeTo: nil)
    if scoped { url.stopAccessingSecurityScopedResource() }
    guard let data else {
      done(nil)
      return
    }
    io.async {
      var marks = self.bookmarks()
      // The same folder picked again keeps its key: nothing is read anew.
      let path = url.standardizedFileURL.path
      let key =
        marks.keys.sorted().first { (try? self.resolve($0))?.standardizedFileURL.path == path }
        ?? FolderRules.newKey()
      marks[key] = data
      self.save(marks)
      done(key)
    }
  }

  /// A zip the picker copied in: deleted again with [dropUnzipped].
  func pickedZip(_ url: URL) {
    io.async { self.zips.append(url) }
  }

  private func answerNow(_ method: String, _ args: [String: Any]) throws -> Any? {
    switch method {
    // The iPhone has no Auto Backup of the app's own (ADR 0010): none.
    case "autoBackupDir", "autoBackup":
      return nil
    case "name":
      return name(try string(args, "tree"))
    case "files":
      return try importFiles(of: try string(args, "tree"))
    case "release":
      let tree = try string(args, "tree")
      if FolderRules.releases(tree, keep: args["keep"] as? [String] ?? []) {
        var marks = bookmarks()
        if marks.removeValue(forKey: tree) != nil { save(marks) }
      }
      return nil
    case "readDoc":
      let limit = (args["limit"] as? NSNumber)?.intValue ?? 0
      return try readDoc(try string(args, "doc"), limit: limit).map {
        FlutterStandardTypedData(bytes: $0)
      }
    case "zipNames":
      let zip = try zipFile(args)
      defer { try? zip.close() }
      return try ZipReader.entries(zip).filter { !$0.isDirectory }.map(\.name)
    case "unzip":
      return try unzip(args)
    case "dropUnzipped":
      try? files.removeItem(at: unzipped)
      for zip in zips { try? files.removeItem(at: zip) }
      zips = []
      return nil
    default:
      return try inFolder(try string(args, "tree")) { folder in
        try answerInFolder(method, args, folder)
      }
    }
  }

  /// The calls on `oldbrainfix-sync` (or the `root` named) inside the
  /// picked [folder], the Backups' too.
  private func answerInFolder(_ method: String, _ args: [String: Any], _ folder: URL) throws
    -> Any?
  {
    let root = args["root"] as? String ?? FolderRules.root
    let dirPath = args["dir"] as? String ?? ""
    switch method {
    case "list":
      guard let dir = try subfolder(folder, root, dirPath, create: false) else { return [String]() }
      let listing = try files.contentsOfDirectory(
        at: dir, includingPropertiesForKeys: [.isRegularFileKey], options: [])
      let names = listing.filter {
        (try? $0.resourceValues(forKeys: [.isRegularFileKey]))?.isRegularFile == true
      }.map(\.lastPathComponent)
      return FolderRules.shownNames(names)
    case "read":
      guard let dir = try subfolder(folder, root, dirPath, create: false) else { return nil }
      let file = dir.appendingPathComponent(try fileName(args))
      guard exists(file) else { return nil }
      return FlutterStandardTypedData(bytes: try coordinatedRead(file) { try Data(contentsOf: $0) })
    case "write":
      guard let dir = try subfolder(folder, root, dirPath, create: true) else {
        throw FolderFailure("no folder")
      }
      let bytes = (args["bytes"] as? FlutterStandardTypedData)?.data ?? Data()
      try coordinatedWrite(dir.appendingPathComponent(try fileName(args)), .forReplacing) {
        try bytes.write(to: $0, options: .atomic)
      }
      return nil
    case "put":
      guard let dir = try subfolder(folder, root, dirPath, create: true) else {
        throw FolderFailure("no folder")
      }
      try put(URL(fileURLWithPath: try string(args, "path")), into: dir, as: try fileName(args))
      return nil
    case "delete":
      guard let dir = try subfolder(folder, root, dirPath, create: false) else { return nil }
      let name = try fileName(args)
      let file = dir.appendingPathComponent(name)
      let placeholder = dir.appendingPathComponent(".\(name).icloud")
      for target in [file, placeholder] where files.fileExists(atPath: target.path) {
        try coordinatedWrite(target, .forDeleting) { try self.files.removeItem(at: $0) }
      }
      return nil
    default:  // "ensure"
      _ = try subfolder(folder, root, dirPath, create: true)
      return nil
    }
  }

  // MARK: The bookmarks

  private func bookmarks() -> [String: Data] {
    if let marks { return marks }
    let read = FolderRules.decode(try? Data(contentsOf: bookmarksFile))
    marks = read
    return read
  }

  private func save(_ marks: [String: Data]) {
    self.marks = marks
    try? files.createDirectory(at: support, withIntermediateDirectories: true)
    try? FolderRules.encode(marks).write(to: bookmarksFile, options: .atomic)
  }

  /// The folder kept under [key]; a stale bookmark is made anew under the
  /// same key, as Apple asks.
  private func resolve(_ key: String) throws -> URL {
    guard let data = bookmarks()[key] else { throw FolderFailure("no folder kept for \(key)") }
    var stale = false
    let url = try URL(
      resolvingBookmarkData: data, options: [], relativeTo: nil, bookmarkDataIsStale: &stale)
    if stale {
      let scoped = url.startAccessingSecurityScopedResource()
      defer { if scoped { url.stopAccessingSecurityScopedResource() } }
      if let fresh = try? url.bookmarkData(
        options: .minimalBookmark, includingResourceValuesForKeys: [.nameKey], relativeTo: nil)
      {
        var marks = bookmarks()
        marks[key] = fresh
        save(marks)
      }
    }
    return url
  }

  /// Runs [body] on the folder kept under [key], with its access started.
  private func inFolder<T>(_ key: String, _ body: (URL) throws -> T) throws -> T {
    let url = try resolve(key)
    let scoped = url.startAccessingSecurityScopedResource()
    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
    return try body(url)
  }

  /// The folder's own name; from the bookmark itself when it no longer
  /// resolves.
  private func name(_ key: String) -> String? {
    if let url = try? resolve(key) {
      let scoped = url.startAccessingSecurityScopedResource()
      defer { if scoped { url.stopAccessingSecurityScopedResource() } }
      return (try? url.resourceValues(forKeys: [.localizedNameKey]))?.localizedName
        ?? url.lastPathComponent
    }
    guard let data = bookmarks()[key] else { return nil }
    return URL.resourceValues(forKeys: [.nameKey], fromBookmarkData: data)?.name
  }

  // MARK: Folders and files

  /// `root`, or a folder in it, inside [folder]; made on the way when
  /// [create], else nil when it is not there.
  private func subfolder(_ folder: URL, _ root: String, _ path: String, create: Bool) throws
    -> URL?
  {
    var dir = folder
    for segment in FolderRules.segments(root: root, dir: path) {
      dir = dir.appendingPathComponent(segment, isDirectory: true)
      var isDir: ObjCBool = false
      if files.fileExists(atPath: dir.path, isDirectory: &isDir) {
        if isDir.boolValue { continue }
        if !create { return nil }
        throw FolderFailure("cannot create \(segment)")
      }
      if !create { return nil }
      try coordinatedWrite(dir, []) {
        try self.files.createDirectory(at: $0, withIntermediateDirectories: true)
      }
    }
    return dir
  }

  /// Whether [file] is there, downloaded or not.
  private func exists(_ file: URL) -> Bool {
    let placeholder = file.deletingLastPathComponent()
      .appendingPathComponent(".\(file.lastPathComponent).icloud")
    return files.fileExists(atPath: file.path) || files.fileExists(atPath: placeholder.path)
  }

  /// Copies the app's own file [from] into [dir] as [name], a Backup too
  /// big to cross the channel: copied in beside, then moved over, so a
  /// cloud client never takes half a file for the whole.
  private func put(_ from: URL, into dir: URL, as name: String) throws {
    let beside = dir.appendingPathComponent("\(name).tmp")
    let target = dir.appendingPathComponent(name)
    try coordinatedWrite(beside, .forReplacing) { at in
      if self.files.fileExists(atPath: at.path) { try self.files.removeItem(at: at) }
      try self.files.copyItem(at: from, to: at)
    }
    var failure: NSError?
    var moved: Error?
    NSFileCoordinator().coordinate(
      writingItemAt: beside, options: .forMoving, writingItemAt: target, options: .forReplacing,
      error: &failure
    ) { source, destination in
      do {
        if self.files.fileExists(atPath: destination.path) {
          _ = try self.files.replaceItemAt(destination, withItemAt: source)
        } else {
          try self.files.moveItem(at: source, to: destination)
        }
      } catch {
        moved = error
      }
    }
    if let failure { throw failure }
    if let moved { throw moved }
  }

  /// Every file of the folder kept under [key] and of its folders but
  /// hidden ones, for an Import, as FolderChannel.kt lists them.
  private func importFiles(of key: String) throws -> [[String: Any]] {
    try inFolder(key) { folder in
      var found: [[String: Any]] = []
      var folders: [(URL, String)] = [(folder, "")]
      let keys: [URLResourceKey] = [.isDirectoryKey, .contentModificationDateKey, .fileSizeKey]
      while !folders.isEmpty {
        let (dir, path) = folders.removeFirst()
        let listing = try files.contentsOfDirectory(
          at: dir, includingPropertiesForKeys: keys, options: [])
        for child in listing {
          let values = try? child.resourceValues(forKeys: Set(keys))
          let raw = child.lastPathComponent
          if values?.isDirectory == true {
            if FolderRules.descends(raw) {
              folders.append((child, FolderRules.path(path, raw)))
            }
            continue
          }
          let shown = FolderRules.shownName(raw)
          let inside = FolderRules.path(path, shown)
          // A file not downloaded yet: its time and size are the
          // placeholder's, so unknown.
          let placeholder = shown != raw
          found.append([
            "path": inside,
            "doc": FolderRules.ref(folder: key, path: inside),
            "modified": placeholder ? 0 : FolderRules.millis(values?.contentModificationDate),
            "size": placeholder ? -1 : Int64(values?.fileSize ?? -1),
          ])
        }
      }
      return found
    }
  }

  /// The bytes of the file [ref] names, or nil when it holds more than
  /// [limit]; one that cannot be read fails the Import.
  private func readDoc(_ ref: String, limit: Int) throws -> Data? {
    func atMost(_ url: URL) throws -> Data? {
      let handle = try FileHandle(forReadingFrom: url)
      defer { try? handle.close() }
      return try FolderRules.readAtMost(limit: limit) {
        try handle.read(upToCount: 64 * 1024) ?? Data()
      }
    }
    if let parsed = FolderRules.parseRef(ref) {
      return try inFolder(parsed.folder) { folder in
        let file = parsed.path.reduce(folder) { $0.appendingPathComponent($1) }
        return try coordinatedRead(file, atMost)
      }
    }
    // A file [unzip] copied out, in the app's own cache.
    guard let url = URL(string: ref), url.isFileURL,
      url.standardizedFileURL.path.hasPrefix(unzipped.standardizedFileURL.path + "/")
    else { throw FolderFailure("not a file to import: \(ref)") }
    return try atMost(url)
  }

  private func zipFile(_ args: [String: Any]) throws -> FileHandle {
    let zip = try string(args, "zip")
    guard let url = URL(string: zip), url.isFileURL else { throw FolderFailure("no zip \(zip)") }
    return try FileHandle(forReadingFrom: url)
  }

  /// Copies the files of the zip that `wanted` names into the app's
  /// cache, each named by its number, at most its limit and one byte
  /// more, as FolderChannel.kt does.
  private func unzip(_ args: [String: Any]) throws -> [[String: Any]] {
    let wanted = (args["wanted"] as? [String: Any] ?? [:]).compactMapValues {
      ($0 as? NSNumber)?.int64Value
    }
    let zip = try zipFile(args)
    defer { try? zip.close() }
    try? files.removeItem(at: unzipped)
    try files.createDirectory(at: unzipped, withIntermediateDirectories: true)
    var copied: [[String: Any]] = []
    for entry in try ZipReader.entries(zip) where !entry.isDirectory {
      guard let limit = wanted[entry.name] else { continue }
      let file = unzipped.appendingPathComponent(String(copied.count))
      guard files.createFile(atPath: file.path, contents: nil) else {
        throw FolderFailure("cannot make \(file.path)")
      }
      let out = try FileHandle(forWritingTo: file)
      defer { try? out.close() }
      let size = try ZipReader.extract(zip, entry, most: limit + 1) { try out.write(contentsOf: $0) }
      copied.append(["path": entry.name, "doc": file.absoluteString, "size": size])
    }
    return copied
  }

  // MARK: Coordination

  private func coordinatedRead<T>(_ url: URL, _ body: (URL) throws -> T) throws -> T {
    // A file of iCloud Drive not downloaded yet comes down first.
    try? files.startDownloadingUbiquitousItem(at: url)
    var failure: NSError?
    var answer: Result<T, Error> = .failure(FolderFailure("cannot read \(url.lastPathComponent)"))
    NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &failure) { at in
      answer = Result(catching: { try body(at) })
    }
    if let failure { throw failure }
    return try answer.get()
  }

  private func coordinatedWrite(
    _ url: URL, _ options: NSFileCoordinator.WritingOptions, _ body: (URL) throws -> Void
  ) throws {
    var failure: NSError?
    var written: Error?
    NSFileCoordinator().coordinate(writingItemAt: url, options: options, error: &failure) { at in
      do { try body(at) } catch { written = error }
    }
    if let failure { throw failure }
    if let written { throw written }
  }

  // MARK: Arguments

  private func string(_ args: [String: Any], _ key: String) throws -> String {
    guard let value = args[key] as? String else { throw FolderFailure("no \(key)") }
    return value
  }

  private func fileName(_ args: [String: Any]) throws -> String {
    let name = try string(args, "name")
    guard FolderRules.safeName(name) else { throw FolderFailure("not a folder file: \(name)") }
    return name
  }
}

/// The Files app's pickers: a folder for the sync, the Backups or an
/// Import, kept as a bookmark; a zip to import, copied into the app, so
/// no lasting access is taken. One answer at a time, as on Android.
final class FolderPicker: NSObject, UIDocumentPickerDelegate {
  static let shared = FolderPicker()

  private var answer: FlutterResult?
  private var zip = false

  func pickFolder(_ result: @escaping FlutterResult) {
    present(UIDocumentPickerViewController(forOpeningContentTypes: [.folder]), zip: false, result)
  }

  func pickZip(_ result: @escaping FlutterResult) {
    present(
      UIDocumentPickerViewController(forOpeningContentTypes: [.zip], asCopy: true), zip: true,
      result)
  }

  private func present(
    _ picker: UIDocumentPickerViewController, zip: Bool, _ result: @escaping FlutterResult
  ) {
    answer?(nil)
    answer = nil
    guard let top = FolderPicker.top() else {
      result(nil)
      return
    }
    answer = result
    self.zip = zip
    picker.delegate = self
    picker.allowsMultipleSelection = false
    top.present(picker, animated: true)
  }

  func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
    guard let answer else { return }
    self.answer = nil
    guard let url = urls.first else {
      answer(nil)
      return
    }
    if zip {
      Folders.shared.pickedZip(url)
      answer(url.absoluteString)
      return
    }
    Folders.shared.keep(url) { key in DispatchQueue.main.async { answer(key) } }
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    answer?(nil)
    answer = nil
  }

  /// The screen on top, to show a picker over.
  static func top() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    let windows = scenes.flatMap(\.windows)
    var top = (windows.first { $0.isKeyWindow } ?? windows.first)?.rootViewController
    while let shown = top?.presentedViewController { top = shown }
    return top
  }
}
