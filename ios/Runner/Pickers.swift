import Flutter
import Foundation
import PhotosUI
import UIKit
import UniformTypeIdentifiers

/// The system's pickers the photos, profile and bundle channels open
/// (#243), as MainActivity.kt opens Android's: the camera, the photo
/// picker, which needs no access to the library, and the Files picker
/// for a file to import. One answer at a time per picker, as on
/// Android: a second one answers the first as given up.
final class CameraPicker: NSObject, UIImagePickerControllerDelegate,
  UINavigationControllerDelegate
{
  static let shared = CameraPicker()

  private var answer: FlutterResult?

  /// Takes a photo with the camera; answers it as photos are kept
  /// ([PhotoShape]), or nil when none was taken or there is no camera,
  /// as in the simulator.
  func take(_ result: @escaping FlutterResult) {
    answer?(nil)
    answer = nil
    guard UIImagePickerController.isSourceTypeAvailable(.camera), let top = FolderPicker.top()
    else {
      result(nil)
      return
    }
    answer = result
    let camera = UIImagePickerController()
    camera.sourceType = .camera
    camera.mediaTypes = [UTType.image.identifier]
    camera.delegate = self
    top.present(camera, animated: true)
  }

  func imagePickerController(
    _ picker: UIImagePickerController,
    didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]
  ) {
    picker.dismiss(animated: true)
    guard let answer else { return }
    self.answer = nil
    guard let image = info[.originalImage] as? UIImage else {
      answer(nil)
      return
    }
    DispatchQueue.global(qos: .userInitiated).async {
      let photo = PhotoShape.normalize(image: image)
      DispatchQueue.main.async { answer(photo.map { FlutterStandardTypedData(bytes: $0) }) }
    }
  }

  func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
    picker.dismiss(animated: true)
    answer?(nil)
    answer = nil
  }
}

/// The system's photo picker: several pictures for Capture, one for the
/// Profile picture, each made into a photo as the app keeps them.
final class PhotoPicker: NSObject, PHPickerViewControllerDelegate {
  static let shared = PhotoPicker()

  private var answer: (([Data]) -> Void)?

  /// Lets the user pick up to [limit] pictures, 0 for any number;
  /// [done] gets them as photos in the order picked, none when the
  /// picker was closed without picking.
  func pick(limit: Int, _ done: @escaping ([Data]) -> Void) {
    answer?([])
    answer = nil
    guard let top = FolderPicker.top() else {
      done([])
      return
    }
    answer = done
    var configuration = PHPickerConfiguration()
    configuration.filter = .images
    configuration.selectionLimit = limit
    configuration.selection = .ordered
    let picker = PHPickerViewController(configuration: configuration)
    picker.delegate = self
    top.present(picker, animated: true)
  }

  func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
    picker.dismiss(animated: true)
    guard let answer else { return }
    self.answer = nil
    var photos = [Data?](repeating: nil, count: results.count)
    let loaded = DispatchGroup()
    let lock = NSLock()
    for (index, picked) in results.enumerated() {
      let provider = picked.itemProvider
      guard provider.hasItemConformingToTypeIdentifier(UTType.image.identifier) else { continue }
      loaded.enter()
      provider.loadDataRepresentation(forTypeIdentifier: UTType.image.identifier) { data, _ in
        let photo = data.flatMap { PhotoShape.normalize($0) }
        lock.lock()
        photos[index] = photo
        lock.unlock()
        loaded.leave()
      }
    }
    loaded.notify(queue: .main) { answer(photos.compactMap { $0 }) }
  }
}

/// The Files picker for a file to import: a bundle, a Sent Item, a
/// Backup or a Pack. The copy iOS hands over is moved into the bundles'
/// folder, as Bundles.kt copies the file picked, and its path answered.
final class BundlePicker: NSObject, UIDocumentPickerDelegate {
  static let shared = BundlePicker()

  private var answer: FlutterResult?

  func pick(into folder: URL, _ result: @escaping FlutterResult) {
    answer?(nil)
    answer = nil
    guard let top = FolderPicker.top() else {
      result(nil)
      return
    }
    answer = result
    self.folder = folder
    let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.data], asCopy: true)
    picker.delegate = self
    picker.allowsMultipleSelection = false
    top.present(picker, animated: true)
  }

  private var folder: URL?

  func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
    guard let answer else { return }
    self.answer = nil
    guard let url = urls.first, let folder else {
      answer(nil)
      return
    }
    let files = FileManager.default
    try? files.createDirectory(at: folder, withIntermediateDirectories: true)
    // The same name Bundles.kt gives every copy; the app tells them
    // apart by what is in them.
    let target = folder.appendingPathComponent("\(UUID().uuidString).obfsync")
    do {
      try files.moveItem(at: url, to: target)
    } catch {
      guard (try? files.copyItem(at: url, to: target)) != nil else {
        answer(nil)
        return
      }
    }
    answer(target.path)
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    answer?(nil)
    answer = nil
  }
}
