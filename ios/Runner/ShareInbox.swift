import Foundation
import UniformTypeIdentifiers

/// What the Share extension hands the app (#241), in both targets: the
/// App Group container both reach, the manifest of one share, which
/// attachments it takes, and the inbox the shares wait in until the app
/// takes them. No Flutter and no UIApplication here: the extension
/// compiles this file too.
///
/// A share extension cannot open its app (only a Today widget may, and
/// the responder-chain `openURL:` trick is undocumented), so a share
/// waits in the inbox and the app takes it the next time it opens or
/// comes back, as `capture_requests` takes a capture asked for on
/// Android.
enum ShareInbox {
  /// The App Group both targets are entitled to.
  static let group = "group.io.github.paxel.oldbrainfix"

  /// The inbox in the App Group container; nil where the entitlement is
  /// missing, as in an unsigned build.
  static func directory(_ files: FileManager = .default) -> URL? {
    files.containerURL(forSecurityApplicationGroupIdentifier: group)?
      .appendingPathComponent("Inbox", isDirectory: true)
  }

  /// The manifest's name in each share's folder.
  static let manifestName = "manifest.json"

  /// The prefix of a folder still being written; never taken.
  static let partialPrefix = "partial-"

  /// The name a finished share's folder gets: when it was written, in
  /// milliseconds and zero-padded so names sort as times do, then a
  /// UUID. The time lives in the name, so no file date is ever read.
  static func folderName(at date: Date, id: UUID = UUID()) -> String {
    let millis = max(0, Int64(date.timeIntervalSince1970 * 1000))
    return String(format: "%015lld-%@", millis, id.uuidString)
  }

  /// When the share in folder [name] was written, from its name; nil for
  /// a name that is none of [folderName]'s.
  static func writtenAt(_ name: String) -> Date? {
    let parts = name.split(separator: "-", maxSplits: 1)
    guard parts.count == 2, parts[0].count == 15, let millis = Int64(parts[0]) else { return nil }
    return Date(timeIntervalSince1970: Double(millis) / 1000)
  }

  /// Writes one share into [inbox]: [write] fills a folder of its own,
  /// which only then gets its final name, so the app never sees half a
  /// share. Returns the folder, or nil when nothing could be written.
  @discardableResult
  static func write(
    into inbox: URL, at date: Date = Date(), _ files: FileManager = .default,
    _ write: (URL) throws -> ShareManifest
  ) -> URL? {
    let id = UUID()
    let partial = inbox.appendingPathComponent(
      partialPrefix + folderName(at: date, id: id), isDirectory: true)
    do {
      try files.createDirectory(at: partial, withIntermediateDirectories: true)
      let manifest = try write(partial)
      guard !manifest.isEmpty else {
        try? files.removeItem(at: partial)
        return nil
      }
      try manifest.encoded().write(to: partial.appendingPathComponent(manifestName), options: .atomic)
      let done = inbox.appendingPathComponent(folderName(at: date, id: id), isDirectory: true)
      try files.moveItem(at: partial, to: done)
      return done
    } catch {
      try? files.removeItem(at: partial)
      return nil
    }
  }

  /// The finished shares in [inbox], oldest first.
  static func folders(in inbox: URL, _ files: FileManager = .default) -> [URL] {
    let names = (try? files.contentsOfDirectory(atPath: inbox.path)) ?? []
    return names.filter { writtenAt($0) != nil }.sorted()
      .map { inbox.appendingPathComponent($0, isDirectory: true) }
  }

  /// The manifest of the share in [folder]; nil when it has none that
  /// reads.
  static func readManifest(in folder: URL) -> ShareManifest? {
    guard let data = try? Data(contentsOf: folder.appendingPathComponent(manifestName)) else {
      return nil
    }
    return ShareManifest.decoded(data)
  }

  /// What the inbox holds: whether a share waits with text or pictures,
  /// and whether files of the app's own types wait.
  static func waiting(in inbox: URL, _ files: FileManager = .default) -> (shares: Bool, files: Bool) {
    var shares = false
    var documents = false
    for folder in folders(in: inbox, files) {
      guard let manifest = readManifest(in: folder) else { continue }
      shares = shares || manifest.hasShare
      documents = documents || !manifest.files.isEmpty
    }
    return (shares, documents)
  }

  /// One share taken out of the inbox: its subject, its text, links
  /// included, the bytes of its pictures as they were shared, and its
  /// other files as Attachments (#267), moved to where Dart attaches them.
  struct Taken: Equatable {
    var subject: String?
    var text: String
    var photos: [Data]
    var files: [TakenFile] = []
  }

  /// A file of a share for an Attachment: where it lies now, the name it
  /// was shared under and its MIME type, empty when not known.
  struct TakenFile: Equatable {
    var path: String
    var name: String
    var type: String
  }

  /// Takes the oldest share with text, pictures or other files out of
  /// [inbox], those files moved into [attachments] when it is given and
  /// let go of otherwise; its files of the app's own types stay for
  /// [takeFiles]. Nil when none waits.
  static func takeShare(
    from inbox: URL, attachments destination: URL? = nil, _ files: FileManager = .default
  ) -> Taken? {
    for folder in folders(in: inbox, files) {
      guard var manifest = readManifest(in: folder) else {
        try? files.removeItem(at: folder)
        continue
      }
      guard manifest.hasShare else { continue }
      let photos = manifest.photos.compactMap { name -> Data? in
        guard let file = inside(folder, name) else { return nil }
        return try? Data(contentsOf: file)
      }
      var moved: [TakenFile] = []
      if let destination {
        try? files.createDirectory(at: destination, withIntermediateDirectories: true)
      }
      for shared in manifest.attachments ?? [] {
        guard let file = inside(folder, shared.file) else { continue }
        guard let destination else {
          try? files.removeItem(at: file)
          continue
        }
        let target = destination.appendingPathComponent(UUID().uuidString)
        if (try? files.moveItem(at: file, to: target)) != nil {
          moved.append(TakenFile(path: target.path, name: shared.name, type: shared.type))
        }
      }
      let taken = Taken(
        subject: manifest.subject, text: manifest.text, photos: photos, files: moved)
      for name in manifest.photos {
        if let file = inside(folder, name) { try? files.removeItem(at: file) }
      }
      manifest.subject = nil
      manifest.texts = []
      manifest.photos = []
      manifest.attachments = nil
      keep(manifest, in: folder, files)
      return taken
    }
    return nil
  }

  /// Moves every file of the app's own types out of [inbox] into
  /// [destination], oldest share first; their new paths.
  static func takeFiles(from inbox: URL, to destination: URL, _ files: FileManager = .default)
    -> [URL]
  {
    var moved: [URL] = []
    try? files.createDirectory(at: destination, withIntermediateDirectories: true)
    for folder in folders(in: inbox, files) {
      guard var manifest = readManifest(in: folder) else {
        try? files.removeItem(at: folder)
        continue
      }
      guard !manifest.files.isEmpty else { continue }
      for name in manifest.files {
        guard let file = inside(folder, name) else { continue }
        let ext = (name as NSString).pathExtension
        let target = destination.appendingPathComponent(
          ext.isEmpty ? UUID().uuidString : "\(UUID().uuidString).\(ext)")
        if (try? files.moveItem(at: file, to: target)) != nil { moved.append(target) }
      }
      manifest.files = []
      keep(manifest, in: folder, files)
    }
    return moved
  }

  /// Removes the folders a share extension began and never finished,
  /// such as when it was stopped: those begun more than [age] before
  /// [now], by the time in their name.
  static func sweep(
    _ inbox: URL, now: Date = Date(), age: TimeInterval = 3600, _ files: FileManager = .default
  ) {
    let names = (try? files.contentsOfDirectory(atPath: inbox.path)) ?? []
    for name in names where name.hasPrefix(partialPrefix) {
      let begun = writtenAt(String(name.dropFirst(partialPrefix.count)))
      if begun.map({ now.timeIntervalSince($0) > age }) ?? true {
        try? files.removeItem(at: inbox.appendingPathComponent(name))
      }
    }
  }

  /// [manifest] written back into [folder], or the folder removed once
  /// nothing is left in it.
  private static func keep(_ manifest: ShareManifest, in folder: URL, _ files: FileManager) {
    if manifest.isEmpty {
      try? files.removeItem(at: folder)
      return
    }
    try? manifest.encoded().write(to: folder.appendingPathComponent(manifestName), options: .atomic)
  }

  /// The file [name] directly in [folder]; nil for a name that would
  /// reach anywhere else.
  static func inside(_ folder: URL, _ name: String) -> URL? {
    guard !name.isEmpty, name != ".", name != "..", !name.contains("/"), !name.contains("\\"),
      name != manifestName
    else { return nil }
    return folder.appendingPathComponent(name)
  }
}

/// One share as the extension wrote it: its subject (the sharing app's
/// title), its texts and links in the order shared, and the names of
/// its files in the share's folder, pictures, the app's own files and
/// any other files (#267) apart. An older app reading it ignores the
/// other files; one written by an older extension has none.
struct ShareManifest: Codable, Equatable {
  var version = 1
  var subject: String?
  var texts: [String] = []
  var photos: [String] = []
  var files: [String] = []
  var attachments: [SharedAttachment]?

  /// Whether there is text, a picture or another file for Capture.
  var hasShare: Bool {
    !(subject ?? "").trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !text.isEmpty
      || !photos.isEmpty || !(attachments ?? []).isEmpty
  }

  var isEmpty: Bool { !hasShare && files.isEmpty }

  /// The texts as one: trimmed, each once, and a text another one
  /// already holds, such as a link inside the text shared with it,
  /// left out.
  var text: String {
    var kept: [String] = []
    for part in texts.map({ $0.trimmingCharacters(in: .whitespacesAndNewlines) })
    where !part.isEmpty && !kept.contains(part) {
      kept.append(part)
    }
    let unique = kept.enumerated().filter { index, part in
      !kept.enumerated().contains { other, longer in other != index && longer.contains(part) }
    }
    return unique.map(\.element).joined(separator: "\n")
  }

  func encoded() throws -> Data {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return try encoder.encode(self)
  }

  /// The manifest in [data]; nil when it is none, or of a version this
  /// app does not know.
  static func decoded(_ data: Data) -> ShareManifest? {
    guard let manifest = try? JSONDecoder().decode(ShareManifest.self, from: data),
      manifest.version == 1
    else { return nil }
    return manifest
  }
}

/// A file shared for an Attachment (#267): its [file] in the share's
/// folder, the [name] it was shared under and its MIME [type], empty
/// when not known.
struct SharedAttachment: Codable, Equatable {
  var file: String
  var name: String
  var type: String
}

/// What one shared attachment is to the app.
enum ShareKind: Equatable {
  /// A file of one of the app's own types: a bundle, Sent Item, Profile
  /// card or Pack, opened as "Open with" opens it.
  case file(ext: String)
  /// A picture, made into a photo by the app.
  case photo
  /// A web link, kept as text.
  case link
  /// Plain text.
  case text
  /// Any other file, kept as an Attachment of the Note (#267).
  case attachment
}

/// The app's own document types: the UTIs the Runner's Info.plist
/// exports, each with its file extension and the MIME type Android's
/// manifest names.
enum ShareTypes {
  static let documents: [(uti: String, ext: String, mime: String)] = [
    ("io.github.paxel.oldbrainfix.sync", "obfsync", "application/vnd.oldbrainfix.sync"),
    ("io.github.paxel.oldbrainfix.item", "oldbrain", "application/vnd.oldbrainfix.item"),
    ("io.github.paxel.oldbrainfix.card", "obfcard", "application/vnd.oldbrainfix.card"),
    ("io.github.paxel.oldbrainfix.pack", "obfpack", "application/vnd.oldbrainfix.pack"),
  ]

  /// The extension of the app's own type [uti], or of the file [name];
  /// nil for anything else.
  static func documentExtension(uti: String?, name: String?) -> String? {
    if let uti, let match = documents.first(where: { $0.uti == uti }) { return match.ext }
    let ext = ((name ?? "") as NSString).pathExtension.lowercased()
    return documents.first { $0.ext == ext }?.ext
  }

  /// What an attachment registered as [identifiers], suggesting the
  /// name [name], is to the app; the type to load it as; nil for what
  /// the app does not take. The app's own files go first, then
  /// pictures, links and text; a file of any other type is an
  /// Attachment (#267).
  static func kind(of identifiers: [String], name: String? = nil) -> (ShareKind, String)? {
    for id in identifiers {
      if let ext = documentExtension(uti: id, name: nil) { return (.file(ext: ext), id) }
    }
    let types = identifiers.compactMap { id in UTType(id).map { (id: id, type: $0) } }
    if let ext = documentExtension(uti: nil, name: name),
      let match = types.first(where: { $0.type.conforms(to: .data) || $0.type.conforms(to: .fileURL) })
    {
      return (.file(ext: ext), match.id)
    }
    if let match = types.first(where: { $0.type.conforms(to: .image) }) {
      return (.photo, match.id)
    }
    if let match = types.first(where: { $0.type.conforms(to: .url) && !$0.type.conforms(to: .fileURL) }) {
      return (.link, match.id)
    }
    if let match = types.first(where: { $0.type.conforms(to: .plainText) }) {
      return (.text, match.id)
    }
    // A URL is data too; a file URL alone is nothing to load.
    if let match = types.first(where: { $0.type.conforms(to: .data) && !$0.type.conforms(to: .url) }) {
      return (.attachment, match.id)
    }
    return nil
  }
}
