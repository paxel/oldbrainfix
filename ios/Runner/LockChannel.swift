import Flutter
import Foundation
import LocalAuthentication

/// The `lock` channel on the iPhone (#243), answering
/// lib/src/lock_platform.dart as Biometrics.kt does: Face ID or Touch
/// ID, with the passcode as fallback, unlocks the App Lock and the Area
/// Locks. They guard the screens only, as on Android (#42): nothing is
/// encrypted with it, and the app's own PIN always unlocks besides.
enum LockChannel {
  static let name = "io.github.paxel.oldbrainfix/lock"

  /// The prompt showing, so a second one is turned down.
  private static var showing: LAContext?

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      switch call.method {
      case "available":
        result(available())
      case "authenticate":
        let args = call.arguments as? [String: Any] ?? [:]
        authenticate(title: args["title"] as? String ?? "", result)
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  /// Whether Face ID, Touch ID or the passcode can unlock: true where
  /// the iPhone has a passcode, which every biometric needs.
  static func available() -> Bool {
    LAContext().canEvaluatePolicy(.deviceOwnerAuthentication, error: nil)
  }

  /// Shows the system's prompt with [title]; answers whether the user
  /// unlocked. Given up, failed or locked out answers false, as Android's
  /// prompt does; the passcode is offered in it, so it needs no button
  /// of its own.
  private static func authenticate(title: String, _ result: @escaping FlutterResult) {
    guard showing == nil else {
      result(false)
      return
    }
    let context = LAContext()
    guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: nil) else {
      result(false)
      return
    }
    showing = context
    // iOS refuses an empty reason.
    let reason = LockReason.of(title)
    context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason) { ok, _ in
      DispatchQueue.main.async {
        showing = nil
        result(ok)
      }
    }
  }
}

/// The reason the prompt shows under Face ID's name and over the
/// passcode: the app's title, never empty.
enum LockReason {
  static func of(_ title: String) -> String {
    let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmed.isEmpty ? "Old Brain Fix" : trimmed
  }
}
