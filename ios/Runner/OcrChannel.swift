import CoreGraphics
import Flutter
import Foundation
import ImageIO

/// The ocr channel on the iPhone (#239), answering the calls of
/// MainActivity.kt's: whether RustO runs, the text in a photo, and
/// letting the models go.
enum OcrChannel {
  static let name = "io.github.paxel.oldbrainfix/ocr"
  private static var channel: FlutterMethodChannel?

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in handle(call, result) }
    self.channel = channel
  }

  private static func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
    switch call.method {
    case "supported":
      result(Ocr.shared.supported)
    case "read":
      guard let jpeg = (call.arguments as? FlutterStandardTypedData)?.data else {
        result(nil)
        return
      }
      Ocr.shared.read(jpeg) { text in DispatchQueue.main.async { result(text) } }
    case "idle":
      Ocr.shared.idle()
      result(nil)
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}

/// Photo Texts (ADR 0007) on the iPhone: the text in a photo, read on
/// the phone by RustO (the crate android/ocr, linked in) with the
/// PP-OCRv6 Tiny models and their character list the Android app
/// bundles, android/app/src/main/assets/rusto, copied into the app as
/// `rusto/`. Offline: nothing is fetched. The twin of Ocr.kt.
final class Ocr {
  static let shared = Ocr()

  static let folder = "rusto"
  static let detection = "det.rten"
  static let recognition = "rec.rten"
  static let characters = "dict.txt"

  /// The one queue reading: photos go one at a time.
  private let worker = DispatchQueue(label: "io.github.paxel.oldbrainfix.ocr")

  /// The models loaded on [worker], until [idle]; nil while none are.
  private var reader: OpaquePointer?

  private let bundle: Bundle

  init(bundle: Bundle = .main) {
    self.bundle = bundle
  }

  /// RustO reads with one thread fewer than the phone has cores, at most
  /// four, like Whisper; set once, before its thread pool starts.
  private static let threadsSet: Void = {
    let threads = SpeechFiles.threads(cores: ProcessInfo.processInfo.activeProcessorCount)
    setenv("RTEN_NUM_THREADS", String(threads), 1)
  }()

  /// Whether the models are in the app, so RustO can read.
  var supported: Bool {
    [Ocr.detection, Ocr.recognition, Ocr.characters].allSatisfy { model(named: $0) != nil }
  }

  /// The text in the photo [jpeg], on the reading queue: [done] hears it,
  /// empty when there is none, or nil when it could not be read.
  func read(_ jpeg: Data, done: @escaping (String?) -> Void) {
    worker.async { done(self.readNow(jpeg)) }
  }

  /// Nothing more to read for now: the models' memory goes.
  func idle() {
    worker.async { self.closeReader() }
  }

  private func readNow(_ jpeg: Data) -> String? {
    guard let picture = Ocr.rgba(jpeg) else { return nil }
    if reader == nil { reader = open() }
    guard let reader else { return nil }
    let text = picture.pixels.withUnsafeBufferPointer { pixels in
      obf_ocr_read(
        reader, pixels.baseAddress, pixels.count, UInt32(picture.width), UInt32(picture.height))
    }
    guard let text else { return nil }
    defer { obf_ocr_free_text(text) }
    return String(cString: text)
  }

  private func open() -> OpaquePointer? {
    _ = Ocr.threadsSet
    guard let detection = model(named: Ocr.detection),
      let recognition = model(named: Ocr.recognition),
      let characters = model(named: Ocr.characters),
      let detectionBytes = try? Data(contentsOf: detection),
      let recognitionBytes = try? Data(contentsOf: recognition),
      let characterBytes = try? Data(contentsOf: characters)
    else { return nil }
    let det = [UInt8](detectionBytes)
    let rec = [UInt8](recognitionBytes)
    let dict = [UInt8](characterBytes)
    return det.withUnsafeBufferPointer { det in
      rec.withUnsafeBufferPointer { rec in
        dict.withUnsafeBufferPointer { dict in
          obf_ocr_open(
            det.baseAddress, det.count, rec.baseAddress, rec.count, dict.baseAddress, dict.count)
        }
      }
    }
  }

  private func model(named name: String) -> URL? {
    let file = name as NSString
    return bundle.url(
      forResource: file.deletingPathExtension, withExtension: file.pathExtension,
      subdirectory: Ocr.folder)
  }

  private func closeReader() {
    if let reader { obf_ocr_close(reader) }
    reader = nil
  }

  /// The photo [jpeg] as the RGBA bytes RustO reads, row by row from the
  /// top, as decoded: the app hands in an upright photo. The fourth
  /// byte of each pixel is not read. Nil when it is no picture.
  static func rgba(_ jpeg: Data) -> (pixels: [UInt8], width: Int, height: Int)? {
    guard let source = CGImageSourceCreateWithData(jpeg as CFData, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
    else { return nil }
    let width = image.width
    let height = image.height
    guard width > 0, height > 0, let space = CGColorSpace(name: CGColorSpace.sRGB) else {
      return nil
    }
    var pixels = [UInt8](repeating: 0, count: width * height * 4)
    let drawn = pixels.withUnsafeMutableBytes { bytes -> Bool in
      guard
        let context = CGContext(
          data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
          bytesPerRow: width * 4, space: space,
          bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue
            | CGBitmapInfo.byteOrder32Big.rawValue)
      else { return false }
      context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
      return true
    }
    return drawn ? (pixels: pixels, width: width, height: height) : nil
  }
}
