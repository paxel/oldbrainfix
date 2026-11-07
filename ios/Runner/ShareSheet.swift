import Flutter
import Foundation
import Photos
import UIKit

/// What goes out of the app on the iPhone (#243), as Bundles.kt,
/// Printing.kt and PhotoOut.kt send it on Android: a file or a Note's
/// text through the share sheet, the Recovery Sheet to the print
/// dialog, a photo into the Photos library or onto the clipboard. Each
/// takes only a file the app wrote into [Exports].
enum ShareSheet {
  /// The share sheet's answer still owed, so a second sheet answers the
  /// first as left, as MainActivity.kt's shareWith does.
  private static var answer: FlutterResult?

  /// Offers the files [paths] and [text] on the share sheet, [subject]
  /// for the apps that take one, such as Mail. Answers "taken" once an
  /// app took them, "dismissed" when the sheet was left, "none" when one
  /// of the files is none the app wrote or there is no screen to show
  /// the sheet over.
  static func share(
    text: String, subject: String?, paths: [String], _ result: @escaping FlutterResult
  ) {
    let files = paths.map { Exports.exported($0) }
    guard !files.contains(where: { $0 == nil }), let top = FolderPicker.top() else {
      result("none")
      return
    }
    var items: [Any] = []
    if !text.isEmpty { items.append(ShareText(text: text, subject: subject)) }
    items.append(contentsOf: files.compactMap { $0 })
    guard !items.isEmpty else {
      result("none")
      return
    }
    answer?("dismissed")
    answer = result
    let sheet = UIActivityViewController(activityItems: items, applicationActivities: nil)
    sheet.completionWithItemsHandler = { activity, completed, _, _ in
      // Called again when a sheet an app showed closes; answered once.
      guard let owed = answer else { return }
      answer = nil
      owed(completed && activity != nil ? "taken" : "dismissed")
    }
    anchor(sheet, over: top)
    top.present(sheet, animated: true)
  }

  /// Opens the print dialog for the PDF at [path] as [name]; false when
  /// it is no PDF the app wrote or nothing can print it.
  static func print(path: String, name: String) -> Bool {
    guard let file = Exports.exported(path), UIPrintInteractionController.canPrint(file) else {
      return false
    }
    let info = UIPrintInfo.printInfo()
    info.jobName = name
    info.outputType = .general
    let printer = UIPrintInteractionController.shared
    printer.printInfo = info
    printer.printingItem = file
    return printer.present(animated: true, completionHandler: nil)
  }

  /// Saves the photo at [path] into the Photos library, which asks for
  /// adding only, never for reading; answers "saved" or "failed".
  /// Adding cannot make an album, so it lands among the recent photos.
  static func savePhoto(path: String, _ result: @escaping FlutterResult) {
    guard let file = Exports.exported(path) else {
      result("failed")
      return
    }
    PHPhotoLibrary.requestAuthorization(for: .addOnly) { status in
      guard status == .authorized || status == .limited else {
        DispatchQueue.main.async { result("failed") }
        return
      }
      PHPhotoLibrary.shared().performChanges({
        PHAssetCreationRequest.forAsset().addResource(with: .photo, fileURL: file, options: nil)
      }) { saved, _ in
        DispatchQueue.main.async { result(saved ? "saved" : "failed") }
      }
    }
  }

  /// Puts the photo at [path], of MIME [type], on the clipboard as a
  /// picture; whether it is there.
  static func copyPhoto(path: String, type: String) -> Bool {
    guard let file = Exports.exported(path), let data = try? Data(contentsOf: file) else {
      return false
    }
    UIPasteboard.general.setData(data, forPasteboardType: Exports.pasteboardType(mime: type))
    return true
  }

  /// Shows [sheet] as a popover in the middle where the screen is wide
  /// enough for one, as on an iPad.
  static func anchor(_ sheet: UIViewController, over top: UIViewController) {
    guard let popover = sheet.popoverPresentationController else { return }
    popover.sourceView = top.view
    popover.sourceRect = CGRect(x: top.view.bounds.midX, y: top.view.bounds.midY, width: 0, height: 0)
    popover.permittedArrowDirections = []
  }
}

/// A Note's text on the share sheet, with the subject an app such as
/// Mail fills in.
private final class ShareText: NSObject, UIActivityItemSource {
  let text: String
  let subject: String?

  init(text: String, subject: String?) {
    self.text = text
    self.subject = subject
  }

  func activityViewControllerPlaceholderItem(_ controller: UIActivityViewController) -> Any {
    text
  }

  func activityViewController(
    _ controller: UIActivityViewController, itemForActivityType activityType: UIActivity.ActivityType?
  ) -> Any? {
    text
  }

  func activityViewController(
    _ controller: UIActivityViewController, subjectForActivityType activityType: UIActivity.ActivityType?
  ) -> String {
    subject ?? ""
  }
}
