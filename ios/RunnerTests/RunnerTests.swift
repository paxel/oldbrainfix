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

  func testIos26AndNewer() {
    XCTAssertEqual(info("MinimumOSVersion") as? String, "26.0")
  }
}
