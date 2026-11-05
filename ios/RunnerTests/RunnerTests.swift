import Flutter
import UIKit
import XCTest

/// The Runner app as the tests' host sees it: the ids and names the
/// App Store record and the Android package share (#237). The platform
/// channels' bridges get their own tests as their tickets land.
class RunnerTests: XCTestCase {

  private func info(_ key: String) -> Any? {
    Bundle.main.object(forInfoDictionaryKey: key)
  }

  func testBundleIdIsTheAndroidPackage() {
    XCTAssertEqual(Bundle.main.bundleIdentifier, "io.github.paxel.oldbrainfix")
  }

  func testDisplayName() {
    XCTAssertEqual(info("CFBundleDisplayName") as? String, "Old Brain Fix")
  }

  func testEnglishAndGerman() {
    let languages = info("CFBundleLocalizations") as? [String] ?? []
    XCTAssertTrue(languages.contains("en"))
    XCTAssertTrue(languages.contains("de"))
  }

  /// The microphone's reason for voice Notes (#239), in English here and
  /// in German in de.lproj.
  func testTheMicrophoneIsExplained() {
    XCTAssertNotNil(info("NSMicrophoneUsageDescription") as? String)
  }

  /// Face ID's reason for the App Lock and the Area Locks, the camera's
  /// for photos and adding to the Photos library's for Save (#243), in
  /// English here and in German in de.lproj.
  func testFaceIdTheCameraAndSavingPhotosAreExplained() {
    XCTAssertNotNil(info("NSFaceIDUsageDescription") as? String)
    XCTAssertNotNil(info("NSCameraUsageDescription") as? String)
    XCTAssertNotNil(info("NSPhotoLibraryAddUsageDescription") as? String)
    // Adding only: the app never reads the library, the picker needs no
    // access.
    XCTAssertNil(info("NSPhotoLibraryUsageDescription"))
  }

  /// The local network's reason, and the Bonjour type the sync looks
  /// for, without which iOS finds no paired host (#240). A Debug build,
  /// as the tests run, also carries Flutter's own `_dartVmService._tcp`,
  /// which flutter's xcode_backend adds for the debugger.
  func testTheLocalNetworkIsExplainedAndOurServiceNamed() {
    XCTAssertNotNil(info("NSLocalNetworkUsageDescription") as? String)
    let services = info("NSBonjourServices") as? [String] ?? []
    XCTAssertTrue(services.contains("_oldbrainfix._tcp"), "\(services)")
    XCTAssertTrue(
      services.allSatisfy { ["_oldbrainfix._tcp", "_dartVmService._tcp"].contains($0) },
      "\(services)")
  }

  /// The background sync's app refresh task (#240).
  func testBackgroundFetchIsOn() {
    XCTAssertEqual(info("UIBackgroundModes") as? [String], ["fetch"])
  }

  /// iPhone only (#236, #244): no iPad, so the App Store asks for no
  /// iPad screenshots. Xcode writes TARGETED_DEVICE_FAMILY as this key.
  func testIphoneOnly() {
    XCTAssertEqual(info("UIDeviceFamily") as? [Int], [1])
  }

  func testIos26AndNewer() {
    XCTAssertEqual(info("MinimumOSVersion") as? String, "26.0")
  }
}
