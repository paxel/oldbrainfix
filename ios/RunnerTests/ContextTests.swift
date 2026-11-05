import AVFoundation
import XCTest

@testable import Runner

/// The Capture Context on the iPhone (#241): the connected device as
/// the audio route names it, and the reasons iOS shows when the app asks
/// for the calendars and the location.
class ContextTests: XCTestCase {
  func testBluetoothAndCarPlayAreConnectedDevicesEachOnce() {
    let ports: [(type: AVAudioSession.Port, name: String)] = [
      (.builtInSpeaker, "Speaker"),
      (.bluetoothA2DP, " Ann's AirPods "),
      (.bluetoothHFP, "Ann's AirPods"),
      (.bluetoothLE, "Hearing aid"),
      (.carAudio, "VW Golf"),
      (.headphones, "Headphones"),
      (.airPlay, "Living room"),
      (.bluetoothA2DP, "   "),
    ]
    XCTAssertEqual(RouteNames.pick(ports), ["Ann's AirPods", "Hearing aid", "VW Golf"])
  }

  func testNothingConnectedIsNoDevice() {
    let phoneOnly: [(type: AVAudioSession.Port, name: String)] = [
      (.builtInReceiver, "Receiver"), (.builtInMic, "Microphone"),
    ]
    XCTAssertEqual(RouteNames.pick(phoneOnly), [])
    XCTAssertEqual(RouteNames.pick([]), [])
  }

  private func info(_ key: String) -> String? {
    Bundle.main.object(forInfoDictionaryKey: key) as? String
  }

  /// Without these iOS ends the app as it asks (#241); German in
  /// de.lproj.
  func testTheCalendarsAndTheLocationAreExplained() {
    XCTAssertNotNil(info("NSCalendarsFullAccessUsageDescription"))
    XCTAssertNotNil(info("NSLocationWhenInUseUsageDescription"))
    let german = Bundle.main.path(forResource: "InfoPlist", ofType: "strings", inDirectory: nil, forLocalization: "de")
      .flatMap { NSDictionary(contentsOfFile: $0) as? [String: String] } ?? [:]
    XCTAssertNotNil(german["NSCalendarsFullAccessUsageDescription"])
    XCTAssertNotNil(german["NSLocationWhenInUseUsageDescription"])
  }

  /// The Share extension rides inside the app, and both reach the same
  /// App Group (#241).
  func testTheShareExtensionIsEmbedded() throws {
    let plugIns = try XCTUnwrap(Bundle.main.builtInPlugInsURL)
    let appex = plugIns.appendingPathComponent("ShareExtension.appex")
    let extensionBundle = try XCTUnwrap(Bundle(url: appex))
    XCTAssertEqual(extensionBundle.bundleIdentifier, "io.github.paxel.oldbrainfix.ShareExtension")
    XCTAssertEqual(extensionBundle.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String, "Old Brain Fix")
    XCTAssertEqual(
      extensionBundle.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String,
      Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String)
    XCTAssertEqual(
      extensionBundle.object(forInfoDictionaryKey: "CFBundleVersion") as? String,
      Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String)
    let point = (extensionBundle.object(forInfoDictionaryKey: "NSExtension") as? [String: Any])?[
      "NSExtensionPointIdentifier"] as? String
    XCTAssertEqual(point, "com.apple.share-services")
    XCTAssertEqual(ShareInbox.group, "group.io.github.paxel.oldbrainfix")
  }
}
