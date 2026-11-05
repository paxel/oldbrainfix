import XCTest

@testable import Runner

/// The shared folder's rules on the iPhone (#240), as FolderChannelTest.kt
/// checks them on Android: the key a folder is kept by and its bookmarks,
/// the folders and names a call reaches, and what an Import takes.
class FolderRulesTests: XCTestCase {
  func testAKeyIsAPrefixAndAUuidAndStaysTheSame() {
    let id = UUID()
    let key = FolderRules.newKey(id)
    XCTAssertEqual(key, "ios-folder:\(id.uuidString)")
    XCTAssertTrue(FolderRules.isKey(key))
    XCTAssertFalse(FolderRules.isKey("content://com.android.externalstorage/tree/primary%3ASync"))
    XCTAssertFalse(FolderRules.isKey("ios-folder:not-a-uuid"))
    XCTAssertNotEqual(FolderRules.newKey(), FolderRules.newKey())
  }

  func testBookmarksAreKeptAndReadBack() {
    let marks = [
      FolderRules.newKey(): Data([0, 1, 2, 255]),
      FolderRules.newKey(): Data("bookmark".utf8),
    ]
    XCTAssertEqual(FolderRules.decode(FolderRules.encode(marks)), marks)
    XCTAssertEqual(FolderRules.decode(FolderRules.encode([:])), [:])
  }

  func testNothingMissingOrBrokenIsABookmark() {
    XCTAssertEqual(FolderRules.decode(nil), [:])
    XCTAssertEqual(FolderRules.decode(Data("not json".utf8)), [:])
    XCTAssertEqual(FolderRules.decode(Data("[1, 2]".utf8)), [:])
  }

  func testTheSyncFolderAndItsFoldersStayInside() {
    XCTAssertEqual(FolderRules.segments(root: "oldbrainfix-sync", dir: ""), ["oldbrainfix-sync"])
    XCTAssertEqual(
      FolderRules.segments(root: "oldbrainfix-sync", dir: "blobs/ab"),
      ["oldbrainfix-sync", "blobs", "ab"])
    XCTAssertEqual(FolderRules.segments(root: "", dir: ""), [], "the Backups' folder itself")
    XCTAssertEqual(
      FolderRules.segments(root: "oldbrainfix-sync", dir: "../keys"), ["oldbrainfix-sync", "keys"])
  }

  func testOnlyNamesThatStayInTheirFolder() {
    XCTAssertTrue(FolderRules.safeName("a1b2.seg"))
    XCTAssertTrue(FolderRules.safeName(".hidden"))
    for name in ["", ".", "..", "a/b", "a\\b"] {
      XCTAssertFalse(FolderRules.safeName(name), name)
    }
  }

  func testAnImportSkipsHiddenFoldersAndGivesUpWhatTheAppDoesNotKeep() {
    XCTAssertEqual(FolderRules.path("", "a.md"), "a.md")
    XCTAssertEqual(FolderRules.path("Notes/2024", "a.md"), "Notes/2024/a.md")
    XCTAssertTrue(FolderRules.descends("Notes"))
    XCTAssertFalse(FolderRules.descends(".git"))
    XCTAssertFalse(FolderRules.descends(".obsidian"))
    XCTAssertTrue(FolderRules.releases("ios-folder:a", keep: ["ios-folder:b"]))
    XCTAssertFalse(FolderRules.releases("ios-folder:a", keep: ["ios-folder:a"]))
  }

  func testAnICloudFileNotDownloadedGoesByItsName() {
    XCTAssertEqual(FolderRules.shownName(".a1b2.seg.icloud"), "a1b2.seg")
    XCTAssertEqual(FolderRules.shownName("a1b2.seg"), "a1b2.seg")
    XCTAssertEqual(FolderRules.shownName(".icloud"), ".icloud")
    XCTAssertEqual(FolderRules.shownName("..icloud"), "..icloud")
    XCTAssertEqual(FolderRules.shownName("notes.icloud"), "notes.icloud")
    XCTAssertEqual(
      FolderRules.shownNames(["a.seg", ".a.seg.icloud", ".b.seg.icloud"]), ["a.seg", "b.seg"])
  }

  func testAnImportReadsAFileByTheFolderAndItsPath() throws {
    let key = FolderRules.newKey()
    let ref = FolderRules.ref(folder: key, path: "Notes/a.md")
    let parsed = try XCTUnwrap(FolderRules.parseRef(ref))
    XCTAssertEqual(parsed.folder, key)
    XCTAssertEqual(parsed.path, ["Notes", "a.md"])
    XCTAssertNil(FolderRules.parseRef("\(key)|../secret"))
    XCTAssertNil(FolderRules.parseRef("\(key)|Notes//a.md"))
    XCTAssertNil(FolderRules.parseRef("file:///tmp/import-zip/0"))
    XCTAssertNil(FolderRules.parseRef("other|a.md"))
  }

  func testAFileIsReadWholeOrNotAtAll() {
    func chunks(_ parts: [String]) -> () -> Data {
      var left = parts.map { Data($0.utf8) }
      return { left.isEmpty ? Data() : left.removeFirst() }
    }
    XCTAssertEqual(FolderRules.readAtMost(limit: 6, chunks(["abc", "def"])), Data("abcdef".utf8))
    XCTAssertNil(FolderRules.readAtMost(limit: 5, chunks(["abc", "def"])))
    XCTAssertEqual(FolderRules.readAtMost(limit: 0, chunks([])), Data())
  }

  func testTimesAreMillisecondsAndZeroWhenUnknown() {
    XCTAssertEqual(FolderRules.millis(nil), 0)
    XCTAssertEqual(FolderRules.millis(Date(timeIntervalSince1970: 1.5)), 1500)
  }
}
