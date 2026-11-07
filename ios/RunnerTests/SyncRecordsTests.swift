import XCTest

@testable import Runner

/// Discovery's records, the background task's plan and the iPhone's name
/// in a sync (#240), as the Kotlin tests check NsdRecords, SyncWork and
/// Pairing on Android.
class SyncRecordsTests: XCTestCase {
  private let id = "0123456789abcdef0123456789abcdef"

  func testOnlyOurServiceTypeIsOurs() {
    XCTAssertTrue(SyncRecords.isOurs("_oldbrainfix._tcp."))
    XCTAssertTrue(SyncRecords.isOurs("_oldbrainfix._tcp"))
    XCTAssertTrue(SyncRecords.isOurs("._oldbrainfix._tcp.local."))
    XCTAssertFalse(SyncRecords.isOurs("_http._tcp."))
    XCTAssertFalse(SyncRecords.isOurs(nil))
  }

  func testTheDeviceIdIs32LowercaseHexDigits() {
    XCTAssertEqual(SyncRecords.deviceId(["id": Data(id.utf8)]), id)
    XCTAssertEqual(SyncRecords.deviceId(["ID": Data(id.utf8)]), id)
    XCTAssertNil(SyncRecords.deviceId(["id": Data(id.uppercased().utf8)]))
    XCTAssertNil(SyncRecords.deviceId(["id": Data("abc".utf8)]))
    XCTAssertNil(SyncRecords.deviceId([:]))
  }

  func testTheOwnRecordHoldsTheIdOnly() {
    XCTAssertEqual(SyncRecords.own(device: id, port: 4100), ["id": id])
    XCTAssertNil(SyncRecords.own(device: "Pixel", port: 4100))
    XCTAssertNil(SyncRecords.own(device: id, port: 0))
    XCTAssertNil(SyncRecords.own(device: id, port: 70000))
  }

  func testAHostIsFoundWithAnIdAnIpv4AddressAndAPort() {
    let entries = ["id": Data(id.utf8)]
    XCTAssertEqual(
      SyncRecords.found(entries, host: "192.168.1.5", port: 4100),
      FoundHost(device: id, host: "192.168.1.5", port: 4100))
    XCTAssertNil(SyncRecords.found(entries, host: "fe80::1", port: 4100))
    XCTAssertNil(SyncRecords.found(entries, host: nil, port: 4100))
    XCTAssertNil(SyncRecords.found(entries, host: "192.168.1.5", port: 0))
    XCTAssertNil(SyncRecords.found([:], host: "192.168.1.5", port: 4100))
  }

  func testTheTxtRecordIsReadAndMadeInDnsFormat() {
    let record = SyncRecords.txtRecord(["id": id])
    XCTAssertEqual(record.first, UInt8(35))
    XCTAssertEqual(SyncRecords.txtEntries(record), ["id": Data(id.utf8)])
    var mixed: [UInt8] = [3]
    mixed += Array("a=1".utf8)
    mixed += [0, 4]
    mixed += Array("flag".utf8)
    mixed += [3]
    mixed += Array("a=2".utf8)
    XCTAssertEqual(SyncRecords.txtEntries(Data(mixed)), ["a": Data("1".utf8), "flag": Data()])
    XCTAssertEqual(SyncRecords.txtEntries(Data([9, 0x61])), ["a": Data()], "cut short")
  }

  func testAnIpv4AddressIsReadFromItsSockaddr() {
    let head: [UInt8] = [16, UInt8(AF_INET), 0x10, 0x04, 192, 168, 1, 5]
    let ipv4 = Data(head + [UInt8](repeating: 0, count: 8))
    XCTAssertEqual(SyncRecords.ipv4(sockaddr: ipv4), "192.168.1.5")
    let family: [UInt8] = [28, UInt8(AF_INET6)]
    let ipv6 = Data(family + [UInt8](repeating: 0, count: 26))
    XCTAssertNil(SyncRecords.ipv4(sockaddr: ipv6))
    XCTAssertNil(SyncRecords.ipv4(sockaddr: Data([16])))
  }

  func testTheBackgroundTaskIsAskedForNoSoonerThanAndroidsPeriod() {
    let now = Date(timeIntervalSince1970: 1_000_000)
    XCTAssertEqual(BackgroundPlan.next(on: true, now: now), now.addingTimeInterval(15 * 60))
    XCTAssertNil(BackgroundPlan.next(on: false, now: now))
  }

  func testARunAlreadyAskedForKeepsItsRhythm() {
    XCTAssertTrue(BackgroundPlan.asks(on: true, pending: false))
    XCTAssertFalse(BackgroundPlan.asks(on: true, pending: true))
    XCTAssertFalse(BackgroundPlan.asks(on: false, pending: false))
  }

  func testARunGoesOnWithAFolderOrOnALocalNetwork() {
    XCTAssertTrue(BackgroundPlan.runs(folder: true, localNetwork: false))
    XCTAssertTrue(BackgroundPlan.runs(folder: false, localNetwork: true))
    XCTAssertFalse(BackgroundPlan.runs(folder: false, localNetwork: false))
  }

  func testTheBackgroundTaskIsPermittedInTheInfoPlist() {
    let permitted =
      Bundle.main.object(forInfoDictionaryKey: "BGTaskSchedulerPermittedIdentifiers") as? [String]
    XCTAssertEqual(permitted, [BackgroundPlan.identifier])
  }

  func testTheIphoneGoesByMakerAndModel() {
    XCTAssertEqual(DeviceNames.deviceName(maker: "Apple", model: "iPhone"), "Apple iPhone")
    XCTAssertEqual(DeviceNames.deviceName(maker: "apple", model: "Apple iPhone"), "Apple iPhone")
    XCTAssertEqual(DeviceNames.deviceName(maker: "Apple", model: ""), "Apple")
    XCTAssertEqual(DeviceNames.deviceName(maker: "", model: "iPhone"), "iPhone")
  }
}
