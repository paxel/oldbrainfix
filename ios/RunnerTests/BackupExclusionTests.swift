import XCTest

@testable import Runner

/// The app's data left out of the iPhone's own backups (#244, ADR 0010).
class BackupExclusionTests: XCTestCase {
  private var dir: URL!

  override func setUpWithError() throws {
    dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
  }

  override func tearDownWithError() throws {
    try? FileManager.default.removeItem(at: dir)
  }

  func testAMissingFolderIsMadeAndLeftOut() throws {
    let folder = dir.appendingPathComponent("Application Support", isDirectory: true)
    XCTAssertFalse(FileManager.default.fileExists(atPath: folder.path))
    try BackupExclusion.exclude(folder)
    var isDirectory: ObjCBool = false
    XCTAssertTrue(FileManager.default.fileExists(atPath: folder.path, isDirectory: &isDirectory))
    XCTAssertTrue(isDirectory.boolValue)
    XCTAssertTrue(BackupExclusion.isExcluded(folder))
  }

  func testAFolderWithTheDatabaseKeepsItAndIsLeftOut() throws {
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    let database = dir.appendingPathComponent("oldbrainfix.db")
    try Data("db".utf8).write(to: database)
    XCTAssertFalse(BackupExclusion.isExcluded(dir))
    try BackupExclusion.exclude(dir)
    XCTAssertTrue(BackupExclusion.isExcluded(dir))
    XCTAssertEqual(try Data(contentsOf: database), Data("db".utf8))
  }

  func testLeavingOutAgainChangesNothing() throws {
    try BackupExclusion.exclude(dir)
    try BackupExclusion.exclude(dir)
    XCTAssertTrue(BackupExclusion.isExcluded(dir))
  }

  func testTheAppDataStartsWithApplicationSupport() throws {
    let support = try XCTUnwrap(
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first)
    XCTAssertEqual(BackupExclusion.appDataDirectories().first, support)
  }

  func testTheAppDataIsLeftOutOnStart() throws {
    BackupExclusion.excludeAppData()
    for directory in BackupExclusion.appDataDirectories() {
      XCTAssertTrue(BackupExclusion.isExcluded(directory), directory.path)
    }
  }
}
