import Foundation

/// What Send feedback says of this iPhone (#214, #243), as
/// MainActivity.kt's deviceFacts says it of an Android phone: the
/// system's version, the model and where the app came from. No Flutter
/// and no UIKit here, so the tests reach every rule.
enum DeviceFacts {
  /// Where the app came from, by the receipt the App Store puts into
  /// every app it or TestFlight installs: "App Store" for a `receipt`,
  /// "TestFlight" for a `sandboxReceipt`, "unknown" for anything else,
  /// such as a build Xcode installed, whose receipt URL names a sandbox
  /// receipt that is not there. [name] is the receipt URL's last part,
  /// [exists] whether its file is there.
  static func installer(receiptName name: String?, exists: Bool) -> String {
    guard exists else { return "unknown" }
    switch name {
    case "receipt": return "App Store"
    case "sandboxReceipt": return "TestFlight"
    default: return "unknown"
    }
  }

  /// This app's [installer], from its bundle's receipt URL.
  static func installer(_ bundle: Bundle = .main, _ files: FileManager = .default) -> String {
    // Deprecated for AppTransaction, which answers asynchronously and
    // only through StoreKit; the URL still names the receipt.
    let url = bundle.appStoreReceiptURL
    return installer(
      receiptName: url?.lastPathComponent, exists: url.map { files.fileExists(atPath: $0.path) } ?? false)
  }

  /// The model's identifier, "iPhone17,1": the simulator's own from its
  /// environment, as the machine there is the Mac's ("arm64").
  static func model(machine: String, environment: [String: String]) -> String {
    if let simulated = environment["SIMULATOR_MODEL_IDENTIFIER"], !simulated.isEmpty {
      return simulated
    }
    return machine
  }

  /// [model] of this device.
  static func model() -> String {
    var system = utsname()
    uname(&system)
    let machine = withUnsafeBytes(of: &system.machine) { raw in
      String(decoding: raw.prefix { $0 != 0 }, as: UTF8.self)
    }
    return model(machine: machine, environment: ProcessInfo.processInfo.environment)
  }

  /// The facts as the `device` channel's `facts` answers them: `os`
  /// "iOS" tells Dart this shape from Android's.
  static func facts(release: String, model: String, installer: String) -> [String: Any] {
    [
      "os": "iOS",
      "release": release,
      "manufacturer": "Apple",
      "model": model,
      "installer": installer,
      "installerKnown": installer != "unknown",
    ]
  }

  /// What the user's own Profile starts with (ADR 0017), as Profiles.kt
  /// says it: the device's [name] when it holds more than blanks, else
  /// its [model], else nothing. Without Apple's user-assigned name
  /// entitlement iOS names every iPhone just "iPhone".
  static func profileName(name: String?, model: String?) -> String {
    let named = (name ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
    if !named.isEmpty { return named }
    return (model ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
  }
}
