import Foundation
import UniformTypeIdentifiers

/// Where the app writes a file before it goes out (#243), as
/// FileShare.kt and Bundles.kt keep them on Android: one folder, the
/// calendar file's too (CalendarScreens.share), and nothing outside it
/// is ever shared, printed, saved or copied.
enum Exports {
  /// The folder the files wait in.
  static func dir(_ files: FileManager = .default) -> URL {
    files.temporaryDirectory.appendingPathComponent("exports", isDirectory: true)
  }

  /// [name] without any directory part, so it stays in the folder;
  /// "export" for a name that is none.
  static func safeName(_ name: String) -> String {
    let last = (name as NSString).lastPathComponent
    let blank = last.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    return blank || last == "." || last == ".." || last == "/" ? "export" : last
  }

  /// Where a file named [name] is written to go out.
  static func path(for name: String, in dir: URL = Exports.dir()) -> String {
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    return dir.appendingPathComponent(safeName(name)).path
  }

  /// Whether [path] lies directly in [dir], as Bundles.inside says it.
  static func inside(_ dir: URL, _ path: String) -> Bool {
    let file = URL(fileURLWithPath: path).standardizedFileURL
    let name = file.lastPathComponent
    // The folders, which are there, compared with their links resolved;
    // the file may not be written yet.
    let parent = file.deletingLastPathComponent().resolvingSymlinksInPath()
    let folder = dir.standardizedFileURL.resolvingSymlinksInPath()
    return !name.isEmpty && name != "/" && parent.path == folder.path
  }

  /// The file at [path] when it lies in [dir] and is there; nil
  /// otherwise.
  static func exported(
    _ path: String, in dir: URL = Exports.dir(), _ files: FileManager = .default
  ) -> URL? {
    guard inside(dir, path) else { return nil }
    var folder: ObjCBool = false
    guard files.fileExists(atPath: path, isDirectory: &folder), !folder.boolValue else {
      return nil
    }
    return URL(fileURLWithPath: path)
  }

  /// The type a file of MIME [type] goes on the clipboard as; JPEG for
  /// a type iOS does not know.
  static func pasteboardType(mime type: String) -> String {
    UTType(mimeType: type)?.identifier ?? UTType.jpeg.identifier
  }
}

/// What Paste takes from the clipboard (#243), as Clipboards.kt takes
/// it: the text of each item, one per line and blank ones left out, and
/// the pictures. Read only when Paste is tapped, as iOS asks the user
/// for every read by another app's copy.
enum Clipboard {
  /// The text and the picture data of [items], each item the clipboard's
  /// representations by type identifier. An item's text is its plain
  /// text, else its link; its picture the first representation of an
  /// image type.
  static func taken(_ items: [[String: Any]]) -> (text: String, pictures: [Data]) {
    var texts: [String] = []
    var pictures: [Data] = []
    for item in items {
      if let text = text(of: item), !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
        texts.append(text)
      }
      if let picture = picture(of: item) { pictures.append(picture) }
    }
    return (texts.joined(separator: "\n"), pictures)
  }

  private static func text(of item: [String: Any]) -> String? {
    let plain = item.first { key, value in
      (UTType(key)?.conforms(to: .plainText) ?? false) && (value is String || value is Data)
    }
    if let value = plain?.value {
      if let string = value as? String { return string }
      if let data = value as? Data { return String(data: data, encoding: .utf8) }
    }
    let link = item.first { key, _ in
      guard let type = UTType(key) else { return false }
      return type.conforms(to: .url) && !type.conforms(to: .fileURL)
    }
    if let value = link?.value {
      if let url = value as? URL { return url.absoluteString }
      if let string = value as? String { return string }
      if let data = value as? Data { return String(data: data, encoding: .utf8) }
    }
    return nil
  }

  private static func picture(of item: [String: Any]) -> Data? {
    for (key, value) in item.sorted(by: { $0.key < $1.key }) {
      guard UTType(key)?.conforms(to: .image) ?? false, let data = value as? Data else { continue }
      return data
    }
    return nil
  }
}
