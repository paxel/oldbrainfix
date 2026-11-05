import Flutter
import Foundation
import ImageIO
import UIKit
import UniformTypeIdentifiers

/// What other apps hand the iPhone app (#241), answering MainActivity.kt's
/// calls with the same answers: on the `photos` channel `shared`, the
/// oldest share the Share extension kept with text or pictures, the
/// pictures made into photos; on the `bundle` channel `take`, the files
/// of the app's own types the extension kept or "Open with" opened.
/// Each channel is told (`shared`, `opened`) when something waits as the
/// app comes back; `take` itself always reads the inbox, so nothing
/// depends on which comes first.
///
/// Only those methods so far: every other one of the two channels
/// answers not implemented, which Dart takes as "not on this phone",
/// until the camera, picker, clipboard and share sheet come (#243),
/// which extend these handlers rather than register new ones.
enum SharesChannel {
  static let photosName = "io.github.paxel.oldbrainfix/photos"
  static let bundleName = "io.github.paxel.oldbrainfix/bundle"

  private static var photos: FlutterMethodChannel?
  private static var bundle: FlutterMethodChannel?

  /// Taking shares and files one after the other, off the main thread.
  private static let work = DispatchQueue(label: "io.github.paxel.oldbrainfix.shares")

  /// Where taken files wait for Dart to import them; Dart removes each.
  static var bundles: URL {
    FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
      .appendingPathComponent("bundles", isDirectory: true)
  }

  /// The files "Open with" opened, kept until Dart takes them.
  private static var opened: [URL] = []

  static func register(messenger: FlutterBinaryMessenger) {
    let photos = FlutterMethodChannel(name: photosName, binaryMessenger: messenger)
    photos.setMethodCallHandler { call, result in
      switch call.method {
      case "shared":
        work.async {
          let answer = takeShared()
          DispatchQueue.main.async { result(answer) }
        }
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    let bundle = FlutterMethodChannel(name: bundleName, binaryMessenger: messenger)
    bundle.setMethodCallHandler { call, result in
      switch call.method {
      case "take":
        let early = opened
        opened = []
        work.async {
          let paths = (early + takeFiles()).map(\.path)
          DispatchQueue.main.async { result(paths) }
        }
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    self.photos = photos
    self.bundle = bundle
    NotificationCenter.default.addObserver(
      forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main
    ) { _ in tellWaiting() }
  }

  /// Tells Dart what waits in the inbox: a share for Capture, files to
  /// open. Also clears what a stopped extension left half-written.
  static func tellWaiting() {
    work.async {
      guard let inbox = ShareInbox.directory() else { return }
      ShareInbox.sweep(inbox)
      let (shares, files) = ShareInbox.waiting(in: inbox)
      DispatchQueue.main.async {
        if shares { photos?.invokeMethod("shared", arguments: nil) }
        if files || !opened.isEmpty { bundle?.invokeMethod("opened", arguments: nil) }
      }
    }
  }

  /// Keeps the files [urls] another app opened with this one: copied at
  /// once, as the copy iOS made in Documents/Inbox or the file reached
  /// through a security scope may go, then Dart is told. Only files of
  /// the app's own types are kept.
  static func open(_ urls: [URL]) {
    var kept: [URL] = []
    try? FileManager.default.createDirectory(at: bundles, withIntermediateDirectories: true)
    for url in urls where url.isFileURL {
      guard let ext = ShareTypes.documentExtension(uti: nil, name: url.lastPathComponent) else {
        continue
      }
      let scoped = url.startAccessingSecurityScopedResource()
      defer { if scoped { url.stopAccessingSecurityScopedResource() } }
      let target = bundles.appendingPathComponent("\(UUID().uuidString).\(ext)")
      if (try? FileManager.default.copyItem(at: url, to: target)) != nil { kept.append(target) }
      // iOS's own copy in Documents/Inbox is the app's to remove.
      if url.path.contains("/Documents/Inbox/") { try? FileManager.default.removeItem(at: url) }
    }
    guard !kept.isEmpty else { return }
    DispatchQueue.main.async {
      opened.append(contentsOf: kept)
      bundle?.invokeMethod("opened", arguments: nil)
    }
  }

  /// The oldest share as the `shared` answer, or nil.
  private static func takeShared() -> [String: Any]? {
    guard let inbox = ShareInbox.directory(),
      let taken = ShareInbox.takeShare(from: inbox)
    else { return nil }
    var answer: [String: Any] = [
      "text": taken.text,
      "photos": taken.photos.compactMap(PhotoShape.normalize).map {
        FlutterStandardTypedData(bytes: $0)
      },
      "recordings": [Any](),
    ]
    if let subject = taken.subject { answer["subject"] = subject }
    return answer
  }

  private static func takeFiles() -> [URL] {
    guard let inbox = ShareInbox.directory() else { return [] }
    return ShareInbox.takeFiles(from: inbox, to: bundles)
  }
}

/// Photos as the app keeps them, as Photos.kt makes them on Android:
/// upright, at most [maxEdge] pixels on the long edge, on white where
/// the picture is see-through, as a JPEG without the original's
/// metadata, which re-encoding drops.
enum PhotoShape {
  static let maxEdge = 2048
  static let quality: CGFloat = 0.85

  /// The picture in [data] as such a JPEG; nil when it is no picture
  /// iOS can decode (HEIC included).
  static func normalize(_ data: Data) -> Data? {
    guard let source = CGImageSourceCreateWithData(data as CFData, nil),
      CGImageSourceGetCount(source) > 0
    else { return nil }
    let options: [CFString: Any] = [
      kCGImageSourceCreateThumbnailFromImageAlways: true,
      kCGImageSourceCreateThumbnailWithTransform: true,
      kCGImageSourceThumbnailMaxPixelSize: maxEdge,
      kCGImageSourceShouldCacheImmediately: true,
    ]
    guard let picture = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary)
    else { return nil }
    let size = CGSize(width: picture.width, height: picture.height)
    let format = UIGraphicsImageRendererFormat()
    format.scale = 1
    format.opaque = true
    let drawn = UIGraphicsImageRenderer(size: size, format: format).image { context in
      UIColor.white.setFill()
      context.fill(CGRect(origin: .zero, size: size))
      // UIKit's context is flipped against Core Graphics'.
      context.cgContext.translateBy(x: 0, y: size.height)
      context.cgContext.scaleBy(x: 1, y: -1)
      context.cgContext.draw(picture, in: CGRect(origin: .zero, size: size))
    }
    return drawn.jpegData(compressionQuality: quality)
  }
}
