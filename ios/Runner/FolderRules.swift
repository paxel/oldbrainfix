import Foundation

/// The rules of FolderChannel.kt on the iPhone (#240), free of Flutter
/// and UIKit: the key a picked folder is kept by and the bookmarks kept
/// under it, the folders and names a call may reach, what an Import
/// takes, and an iCloud Drive file not downloaded yet shown by its name.
enum FolderRules {
  /// The folder the sync writes into, inside the one picked.
  static let root = "oldbrainfix-sync"

  /// What the folder the picker returned is kept as: this and a UUID. The
  /// app keeps that string as its setting and compares it, so it never
  /// changes, while the bookmark behind it may be made anew.
  static let keyPrefix = "ios-folder:"

  /// A new key for a folder just picked.
  static func newKey(_ id: UUID = UUID()) -> String {
    keyPrefix + id.uuidString
  }

  /// Whether [key] is one [newKey] made.
  static func isKey(_ key: String) -> Bool {
    key.hasPrefix(keyPrefix) && UUID(uuidString: String(key.dropFirst(keyPrefix.count))) != nil
  }

  /// The bookmarks by key as they are kept on disk: JSON, each in base64.
  static func encode(_ bookmarks: [String: Data]) -> Data {
    let strings = bookmarks.mapValues { $0.base64EncodedString() }
    return (try? JSONSerialization.data(withJSONObject: strings, options: [.sortedKeys])) ?? Data()
  }

  /// The bookmarks [encode] kept; none from what is missing or broken.
  static func decode(_ data: Data?) -> [String: Data] {
    guard let data,
      let strings = (try? JSONSerialization.jsonObject(with: data)) as? [String: String]
    else { return [:] }
    return strings.compactMapValues { Data(base64Encoded: $0) }
  }

  /// The folders from the picked one down to [dir] inside [root]; a
  /// segment that would leave its folder is dropped, as on Android.
  static func segments(root: String, dir: String) -> [String] {
    ([root] + dir.components(separatedBy: "/")).filter { !$0.isEmpty && safeName($0) }
  }

  /// A name that stays in its folder.
  static func safeName(_ name: String) -> Bool {
    !name.isEmpty && name != "." && name != ".." && !name.contains("/") && !name.contains("\\")
  }

  /// The path of [name] in the folder at [parent], "" for the picked one.
  static func path(_ parent: String, _ name: String) -> String {
    parent.isEmpty ? name : "\(parent)/\(name)"
  }

  /// Whether an Import takes the files of the folder [name]: not of a
  /// hidden one, such as `.git` or `.obsidian`.
  static func descends(_ name: String) -> Bool {
    !name.hasPrefix(".")
  }

  /// Whether the picked [folder]'s bookmark is given up after an Import:
  /// always, unless it is a folder the app keeps, in [keep].
  static func releases(_ folder: String, keep: [String]) -> Bool {
    !keep.contains(folder)
  }

  /// The name a file goes by: iCloud Drive keeps one not downloaded yet
  /// as `.name.icloud`, which stands for `name`.
  static func shownName(_ file: String) -> String {
    let suffix = ".icloud"
    guard file.hasPrefix("."), file.hasSuffix(suffix), file.count > suffix.count + 1 else {
      return file
    }
    return String(file.dropFirst().dropLast(suffix.count))
  }

  /// The names in a folder's [listing] as they are shown, each once.
  static func shownNames(_ listing: [String]) -> [String] {
    var seen = Set<String>()
    return listing.map(shownName).filter { seen.insert($0).inserted }
  }

  /// What an Import reads a file of the picked [folder] by: its key and
  /// the [path] inside it.
  static func ref(folder: String, path: String) -> String {
    "\(folder)|\(path)"
  }

  /// The folder's key and the path inside it [ref] names, or nil when it
  /// names none or a path that would leave the folder.
  static func parseRef(_ ref: String) -> (folder: String, path: [String])? {
    guard let bar = ref.firstIndex(of: "|") else { return nil }
    let folder = String(ref[..<bar])
    let path = ref[ref.index(after: bar)...].components(separatedBy: "/")
    guard isKey(folder), !path.isEmpty, path.allSatisfy(safeName) else { return nil }
    return (folder, path)
  }

  /// The bytes of [read], or nil as soon as there are more than [limit]:
  /// a larger file is never read whole. [read] answers the next chunk,
  /// empty at the end.
  static func readAtMost(limit: Int, _ read: () throws -> Data) rethrows -> Data? {
    var out = Data()
    while true {
      let chunk = try read()
      if chunk.isEmpty { return out }
      if out.count + chunk.count > limit { return nil }
      out.append(chunk)
    }
  }

  /// The time [date] in milliseconds since 1970, 0 when unknown, as
  /// Android says it.
  static func millis(_ date: Date?) -> Int64 {
    guard let date else { return 0 }
    return Int64((date.timeIntervalSince1970 * 1000).rounded())
  }
}
