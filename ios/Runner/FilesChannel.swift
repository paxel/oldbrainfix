import Flutter
import Foundation
import UIKit
import UniformTypeIdentifiers

/// Attachments on the iPhone (#267, ADR 0047), answering the `files`
/// channel as MainActivity.kt does: the Files picker for files of any
/// kind (`pick`), a Note's file opened with another app (`open`), and the
/// PDF reader of the app's native library, the desktop's
/// `oldbrainfix-pdf` in android/ocr's `pdf` module (`pdfPages`,
/// `pdfPage`), one page at a time.
enum FilesChannel {
  static let name = "io.github.paxel.oldbrainfix/files"

  /// Reading PDFs off the main thread, one at a time.
  private static let work = DispatchQueue(label: "io.github.paxel.oldbrainfix.files")

  /// Where picked files wait until Dart attaches them; Dart removes each.
  static var picked: URL {
    FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
      .appendingPathComponent("shared", isDirectory: true)
  }

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      let args = call.arguments as? [String: Any] ?? [:]
      switch call.method {
      case "pick":
        FilePicker.shared.pick(into: picked, result)
      case "open":
        result(
          AttachmentOpener.shared.open(
            path: args["path"] as? String ?? "", type: args["type"] as? String ?? ""))
      case "pdfPages":
        let path = call.arguments as? String ?? ""
        work.async {
          let pages = path.withCString { obf_pdf_pages($0) }
          DispatchQueue.main.async { result(pages < 0 ? -1 : Int(pages)) }
        }
      case "pdfPage":
        let path = args["path"] as? String ?? ""
        let index = (args["index"] as? NSNumber)?.int64Value ?? -1
        work.async {
          let page = readPage(path: path, index: index)
          DispatchQueue.main.async { result(page) }
        }
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  /// The page [index] of the PDF at [path] as Dart reads it: its `text`
  /// and the `scan` of a scanned page; nil when it cannot be read.
  private static func readPage(path: String, index: Int64) -> [String: Any]? {
    var len = 0
    guard let bytes = path.withCString({ obf_pdf_page($0, index, &len) }) else { return nil }
    let data = Data(bytes: bytes, count: len)
    obf_pdf_free(bytes, len)
    guard let page = decode(data) else { return nil }
    var answer: [String: Any] = ["text": page.text]
    // A scan as a photo is kept, upright and at most 2048 px, as the OCR
    // expects; one that does not decode as it lies.
    if let scan = page.scan {
      answer["scan"] = FlutterStandardTypedData(bytes: PhotoShape.normalize(scan) ?? scan)
    }
    return answer
  }

  /// A page as the native reader encodes it: the length of its text in
  /// UTF-8 as four bytes, most significant first, the text, then the
  /// picture of a scanned page; nil for bytes that are no page.
  static func decode(_ data: Data) -> (text: String, scan: Data?)? {
    let bytes = [UInt8](data)
    guard bytes.count >= 4 else { return nil }
    let len = bytes[0..<4].reduce(0) { ($0 << 8) | Int($1) }
    guard len <= bytes.count - 4,
      let text = String(bytes: bytes[4..<(4 + len)], encoding: .utf8)
    else { return nil }
    let scan = bytes.count > 4 + len ? Data(bytes[(4 + len)...]) : nil
    return (text, scan)
  }

  /// The MIME type of a file named [name], as the core reads one: empty
  /// when its ending says none.
  static func mimeType(of name: String) -> String {
    let ext = (name as NSString).pathExtension
    return UTType(filenameExtension: ext)?.preferredMIMEType ?? ""
  }
}

/// The Files picker for Attachments: several files of any kind, each
/// copied into a folder of the app with a name of its own, answered with
/// its `path`, the `name` it had and its `type`; none when cancelled.
final class FilePicker: NSObject, UIDocumentPickerDelegate {
  static let shared = FilePicker()

  private var answer: FlutterResult?
  private var folder: URL?

  func pick(into folder: URL, _ result: @escaping FlutterResult) {
    answer?([Any]())
    answer = nil
    guard let top = FolderPicker.top() else {
      result([Any]())
      return
    }
    answer = result
    self.folder = folder
    let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.item], asCopy: true)
    picker.delegate = self
    picker.allowsMultipleSelection = true
    top.present(picker, animated: true)
  }

  func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
    guard let answer else { return }
    self.answer = nil
    guard let folder else {
      answer([Any]())
      return
    }
    let files = FileManager.default
    try? files.createDirectory(at: folder, withIntermediateDirectories: true)
    var kept: [[String: String]] = []
    for url in urls {
      let target = folder.appendingPathComponent(UUID().uuidString)
      let moved =
        (try? files.moveItem(at: url, to: target)) != nil
        || (try? files.copyItem(at: url, to: target)) != nil
      guard moved else { continue }
      let name = url.lastPathComponent
      kept.append(["path": target.path, "name": name, "type": FilesChannel.mimeType(of: name)])
    }
    answer(kept)
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    answer?([Any]())
    answer = nil
  }
}

/// Opens a Note's file written into the exports with another app: iOS's
/// own menu of the apps that take it, beside a preview.
final class AttachmentOpener: NSObject, UIDocumentInteractionControllerDelegate {
  static let shared = AttachmentOpener()

  /// Held while its menu shows; iOS keeps no reference of its own.
  private var shown: UIDocumentInteractionController?

  func open(path: String, type: String) -> Bool {
    guard let url = Exports.exported(path), let top = FolderPicker.top() else { return false }
    let controller = UIDocumentInteractionController(url: url)
    controller.delegate = self
    if let uti = UTType(mimeType: type)?.identifier { controller.uti = uti }
    shown = controller
    if controller.presentPreview(animated: true) { return true }
    let opened = controller.presentOpenInMenu(from: top.view.bounds, in: top.view, animated: true)
    if !opened { shown = nil }
    return opened
  }

  func documentInteractionControllerViewControllerForPreview(
    _ controller: UIDocumentInteractionController
  ) -> UIViewController {
    FolderPicker.top() ?? UIViewController()
  }

  func documentInteractionControllerDidEndPreview(_ controller: UIDocumentInteractionController) {
    shown = nil
  }

  func documentInteractionControllerDidDismissOpenInMenu(
    _ controller: UIDocumentInteractionController
  ) {
    shown = nil
  }
}
