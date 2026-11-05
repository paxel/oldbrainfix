import CoreImage
import Flutter
import Foundation
import UIKit
import Vision

/// The pairing channel on the iPhone (#240), answering the calls of
/// MainActivity.kt: the pair code read from a photo of the computer's
/// screen, with Vision's barcode reader on the phone, and the name the
/// iPhone goes by in a sync. The photo itself comes from the photos
/// channel; nothing here opens the camera.
enum PairingChannel {
  static let name = "io.github.paxel.oldbrainfix/pairing"

  private static let reading = DispatchQueue(label: "io.github.paxel.oldbrainfix.qr")

  @discardableResult
  static func register(messenger: FlutterBinaryMessenger) -> FlutterMethodChannel {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      switch call.method {
      case "readQr":
        guard let photo = (call.arguments as? FlutterStandardTypedData)?.data else {
          result(nil)
          return
        }
        reading.async {
          let text = QrReader.read(photo)
          DispatchQueue.main.async { result(text) }
        }
      case "deviceName":
        result(DeviceNames.deviceName(maker: "Apple", model: UIDevice.current.model))
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    return channel
  }
}

/// The QR code's text in a JPEG, as Pairing.kt reads it: the picture as
/// it is, then inverted, since a dark screen shows light modules.
enum QrReader {
  static func read(_ photo: Data) -> String? {
    guard let picture = CIImage(data: photo, options: [.applyOrientationProperty: true]) else {
      return nil
    }
    if let text = read(picture) { return text }
    let inverted = picture.applyingFilter("CIColorInvert")
    return read(inverted)
  }

  private static func read(_ picture: CIImage) -> String? {
    let request = VNDetectBarcodesRequest()
    request.symbologies = [.qr]
    do {
      try VNImageRequestHandler(ciImage: picture, options: [:]).perform([request])
    } catch {
      return nil
    }
    return request.results?.lazy.compactMap(\.payloadStringValue).first
  }
}
