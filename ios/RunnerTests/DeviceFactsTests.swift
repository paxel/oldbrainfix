import XCTest

@testable import Runner

/// What Send feedback tells of this iPhone (#243) and the name a
/// Profile starts with.
class DeviceFactsTests: XCTestCase {
  func testTheReceiptTellsTheStoreFromTestFlight() {
    XCTAssertEqual(DeviceFacts.installer(receiptName: "receipt", exists: true), "App Store")
    XCTAssertEqual(DeviceFacts.installer(receiptName: "sandboxReceipt", exists: true), "TestFlight")
  }

  /// A build Xcode installed names a sandbox receipt that is not there.
  func testNoReceiptOrAnyOtherIsUnknown() {
    XCTAssertEqual(DeviceFacts.installer(receiptName: "sandboxReceipt", exists: false), "unknown")
    XCTAssertEqual(DeviceFacts.installer(receiptName: "receipt", exists: false), "unknown")
    XCTAssertEqual(DeviceFacts.installer(receiptName: nil, exists: false), "unknown")
    XCTAssertEqual(DeviceFacts.installer(receiptName: "other", exists: true), "unknown")
  }

  /// The tests run in the simulator, which Xcode installs.
  func testThisBuildIsUnknown() {
    XCTAssertEqual(DeviceFacts.installer(), "unknown")
  }

  func testTheSimulatorNamesTheModelItSimulates() {
    XCTAssertEqual(
      DeviceFacts.model(machine: "arm64", environment: ["SIMULATOR_MODEL_IDENTIFIER": "iPhone17,1"]),
      "iPhone17,1")
    XCTAssertEqual(DeviceFacts.model(machine: "iPhone17,1", environment: [:]), "iPhone17,1")
    XCTAssertFalse(DeviceFacts.model().isEmpty)
  }

  func testTheFactsTellDartTheyAreTheIPhones() {
    let facts = DeviceFacts.facts(release: "26.1", model: "iPhone17,1", installer: "TestFlight")
    XCTAssertEqual(facts["os"] as? String, "iOS")
    XCTAssertEqual(facts["release"] as? String, "26.1")
    XCTAssertEqual(facts["manufacturer"] as? String, "Apple")
    XCTAssertEqual(facts["model"] as? String, "iPhone17,1")
    XCTAssertEqual(facts["installer"] as? String, "TestFlight")
    XCTAssertEqual(facts["installerKnown"] as? Bool, true)
    let unknown = DeviceFacts.facts(release: "26.1", model: "iPhone17,1", installer: "unknown")
    XCTAssertEqual(unknown["installerKnown"] as? Bool, false)
  }

  func testAProfileStartsWithTheNameElseTheModel() {
    XCTAssertEqual(DeviceFacts.profileName(name: " Anna's iPhone ", model: "iPhone"), "Anna's iPhone")
    XCTAssertEqual(DeviceFacts.profileName(name: "  ", model: "iPhone"), "iPhone")
    XCTAssertEqual(DeviceFacts.profileName(name: nil, model: nil), "")
  }
}
