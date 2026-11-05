import Flutter
import Foundation
import UIKit

/// The `device` and `profile` channels on the iPhone (#243), answering
/// lib/src/feedback_screen.dart and lib/src/profile_platform.dart as
/// MainActivity.kt does: the facts Send feedback tells of this iPhone,
/// and the name and the picture the user's own Profile starts with.
enum DeviceChannel {
  static let deviceName = "io.github.paxel.oldbrainfix/device"
  static let profileName = "io.github.paxel.oldbrainfix/profile"

  static func register(messenger: FlutterBinaryMessenger) {
    let device = FlutterMethodChannel(name: deviceName, binaryMessenger: messenger)
    device.setMethodCallHandler { call, result in
      switch call.method {
      case "facts":
        result(
          DeviceFacts.facts(
            release: UIDevice.current.systemVersion, model: DeviceFacts.model(),
            installer: DeviceFacts.installer()))
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    let profile = FlutterMethodChannel(name: profileName, binaryMessenger: messenger)
    profile.setMethodCallHandler { call, result in
      switch call.method {
      case "deviceName":
        result(
          DeviceFacts.profileName(name: UIDevice.current.name, model: UIDevice.current.model))
      case "pick":
        PhotoPicker.shared.pick(limit: 1) { photos in
          result(photos.first.map { FlutterStandardTypedData(bytes: $0) })
        }
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }
}
