import UIKit
import XCTest

@testable import Runner

/// What the Share extension hands the app (#241): the manifest, which
/// attachments it takes, the inbox the shares wait in, and the photos
/// made of shared pictures.
class ShareInboxTests: XCTestCase {
  private var inbox: URL!

  override func setUpWithError() throws {
    inbox = FileManager.default.temporaryDirectory
      .appendingPathComponent("ShareInboxTests-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: inbox, withIntermediateDirectories: true)
  }

  override func tearDownWithError() throws {
    try? FileManager.default.removeItem(at: inbox)
  }

  // MARK: The manifest

  func testAManifestReadsBackAsWritten() throws {
    let manifest = ShareManifest(
      subject: "A page", texts: ["Look", "https://example.org"], photos: ["a.jpg"],
      files: ["b.obfpack"])
    XCTAssertEqual(ShareManifest.decoded(try manifest.encoded()), manifest)
  }

  func testNothingBrokenOrOfAnotherVersionIsAManifest() throws {
    XCTAssertNil(ShareManifest.decoded(Data("not json".utf8)))
    XCTAssertNil(ShareManifest.decoded(Data(#"{"version": 2, "texts": [], "photos": [], "files": []}"#.utf8)))
    XCTAssertEqual(
      ShareManifest.decoded(Data(#"{"version": 1, "texts": ["x"], "photos": [], "files": []}"#.utf8))?.texts,
      ["x"])
  }

  func testTheTextIsEachPartOnceAndALinkInsideTheTextNotAgain() {
    let manifest = ShareManifest(
      texts: ["  Read this: https://example.org/a ", "https://example.org/a", "", "Read this: https://example.org/a", "Second"])
    XCTAssertEqual(manifest.text, "Read this: https://example.org/a\nSecond")
  }

  func testOnlyBlankTextIsNoShare() {
    XCTAssertTrue(ShareManifest(texts: [" ", "\n"]).isEmpty)
    XCTAssertFalse(ShareManifest(subject: "Title").isEmpty)
    XCTAssertFalse(ShareManifest(files: ["a.obfcard"]).isEmpty)
    XCTAssertFalse(ShareManifest(files: ["a.obfcard"]).hasShare)
  }

  // MARK: What is taken

  func testTheAppsOwnFilesByTypeOrByName() {
    XCTAssertEqual(ShareTypes.kind(of: ["io.github.paxel.oldbrainfix.pack"])?.0, .file(ext: "obfpack"))
    XCTAssertEqual(ShareTypes.kind(of: ["io.github.paxel.oldbrainfix.card"])?.0, .file(ext: "obfcard"))
    XCTAssertEqual(ShareTypes.kind(of: ["io.github.paxel.oldbrainfix.sync"])?.0, .file(ext: "obfsync"))
    XCTAssertEqual(ShareTypes.kind(of: ["io.github.paxel.oldbrainfix.item"])?.0, .file(ext: "oldbrain"))
    // A messenger that only knows "some data" still names the file.
    XCTAssertEqual(ShareTypes.kind(of: ["public.data"], name: "Ann.OBFCARD")?.0, .file(ext: "obfcard"))
    XCTAssertEqual(
      ShareTypes.kind(of: ["public.file-url"], name: "Trip.obfpack")?.0, .file(ext: "obfpack"))
  }

  func testPicturesLinksAndText() {
    XCTAssertEqual(ShareTypes.kind(of: ["public.jpeg"])?.0, .photo)
    XCTAssertEqual(ShareTypes.kind(of: ["public.heic"])?.0, .photo)
    XCTAssertEqual(ShareTypes.kind(of: ["public.png"])?.0, .photo)
    XCTAssertEqual(ShareTypes.kind(of: ["public.url"])?.0, .link)
    XCTAssertEqual(ShareTypes.kind(of: ["public.plain-text"])?.0, .text)
    XCTAssertEqual(ShareTypes.kind(of: ["public.utf8-plain-text"])?.0, .text)
    // Loaded as the type that matched.
    XCTAssertEqual(ShareTypes.kind(of: ["public.plain-text", "public.jpeg"])?.1, "public.jpeg")
  }

  func testAnyOtherFileIsAnAttachment() {
    XCTAssertEqual(ShareTypes.kind(of: ["com.adobe.pdf"], name: "Bill.pdf")?.0, .attachment)
    XCTAssertEqual(ShareTypes.kind(of: ["public.zip-archive"])?.0, .attachment)
    XCTAssertEqual(ShareTypes.kind(of: ["public.data"])?.0, .attachment)
    // A picture is still a photo, and a file URL alone is nothing to load.
    XCTAssertEqual(ShareTypes.kind(of: ["com.adobe.pdf", "public.jpeg"])?.0, .photo)
    XCTAssertNil(ShareTypes.kind(of: ["public.file-url"], name: "notes.zip"))
  }

  func testTheExportedTypesAreTheOnesTheAppDeclares() throws {
    let declared = Bundle.main.object(forInfoDictionaryKey: "UTExportedTypeDeclarations") as? [[String: Any]] ?? []
    let ids = declared.compactMap { $0["UTTypeIdentifier"] as? String }
    XCTAssertEqual(Set(ids), Set(ShareTypes.documents.map(\.uti)))
    for type in declared {
      let tags = type["UTTypeTagSpecification"] as? [String: Any]
      let id = type["UTTypeIdentifier"] as? String
      let document = try XCTUnwrap(ShareTypes.documents.first { $0.uti == id })
      XCTAssertEqual(tags?["public.filename-extension"] as? [String], [document.ext])
      XCTAssertEqual(tags?["public.mime-type"] as? [String], [document.mime])
      XCTAssertEqual(type["UTTypeConformsTo"] as? [String], ["public.data"])
    }
    let documents = Bundle.main.object(forInfoDictionaryKey: "CFBundleDocumentTypes") as? [[String: Any]]
    XCTAssertEqual(Set(documents?.first?["LSItemContentTypes"] as? [String] ?? []), Set(ids))
    XCTAssertEqual(documents?.first?["LSHandlerRank"] as? String, "Owner")
  }

  // MARK: The inbox

  /// Writes a share of [texts], one picture and one Pack into the inbox.
  @discardableResult
  private func share(_ texts: [String], at date: Date, picture: Bool = true, pack: Bool = true)
    -> URL?
  {
    ShareInbox.write(into: inbox, at: date) { folder in
      var manifest = ShareManifest(subject: nil, texts: texts)
      if picture {
        try Data("picture".utf8).write(to: folder.appendingPathComponent("p.jpg"))
        manifest.photos = ["p.jpg"]
      }
      if pack {
        try Data("pack".utf8).write(to: folder.appendingPathComponent("f.obfpack"))
        manifest.files = ["f.obfpack"]
      }
      return manifest
    }
  }

  func testFolderNamesSortAsTimesAndReadBack() {
    let early = ShareInbox.folderName(at: Date(timeIntervalSince1970: 9))
    let late = ShareInbox.folderName(at: Date(timeIntervalSince1970: 1_800_000_000))
    XCTAssertLessThan(early, late)
    XCTAssertEqual(ShareInbox.writtenAt(late), Date(timeIntervalSince1970: 1_800_000_000))
    XCTAssertNil(ShareInbox.writtenAt("partial-" + late))
    XCTAssertNil(ShareInbox.writtenAt("manifest.json"))
  }

  func testSharesAreTakenOldestFirstAndTheirFilesApart() throws {
    share(["second"], at: Date(timeIntervalSince1970: 2000))
    share(["first"], at: Date(timeIntervalSince1970: 1000))
    XCTAssertEqual(ShareInbox.waiting(in: inbox).shares, true)
    XCTAssertEqual(ShareInbox.waiting(in: inbox).files, true)

    let one = try XCTUnwrap(ShareInbox.takeShare(from: inbox))
    XCTAssertEqual(one.text, "first")
    XCTAssertEqual(one.photos, [Data("picture".utf8)])
    XCTAssertEqual(ShareInbox.takeShare(from: inbox)?.text, "second")
    XCTAssertNil(ShareInbox.takeShare(from: inbox))
    XCTAssertEqual(ShareInbox.waiting(in: inbox).shares, false)

    // The Packs waited for the files' turn, oldest first.
    let destination = inbox.appendingPathComponent("taken", isDirectory: true)
    let files = ShareInbox.takeFiles(from: inbox, to: destination)
    XCTAssertEqual(files.count, 2)
    XCTAssertTrue(files.allSatisfy { $0.pathExtension == "obfpack" })
    XCTAssertEqual(try Data(contentsOf: files[0]), Data("pack".utf8))
    // Nothing is left of either share.
    XCTAssertEqual(ShareInbox.folders(in: inbox), [])
    XCTAssertEqual(ShareInbox.waiting(in: inbox).files, false)
  }

  func testAShareWithoutAnythingIsNotWritten() {
    XCTAssertNil(share([" "], at: Date(), picture: false, pack: false))
    XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: inbox.path), [])
  }

  func testAFailedShareLeavesNothing() {
    struct Broken: Error {}
    XCTAssertNil(ShareInbox.write(into: inbox) { _ in throw Broken() })
    XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: inbox.path), [])
  }

  func testAHalfWrittenShareIsNeverTakenAndSweptOnceOld() throws {
    let begun = Date(timeIntervalSince1970: 1_800_000_000)
    let partial = inbox.appendingPathComponent(ShareInbox.partialPrefix + ShareInbox.folderName(at: begun))
    try FileManager.default.createDirectory(at: partial, withIntermediateDirectories: true)
    try ShareManifest(texts: ["half"]).encoded()
      .write(to: partial.appendingPathComponent(ShareInbox.manifestName))
    XCTAssertNil(ShareInbox.takeShare(from: inbox))

    ShareInbox.sweep(inbox, now: begun.addingTimeInterval(60))
    XCTAssertTrue(FileManager.default.fileExists(atPath: partial.path), "still being written")
    ShareInbox.sweep(inbox, now: begun.addingTimeInterval(2 * 3600))
    XCTAssertFalse(FileManager.default.fileExists(atPath: partial.path))
  }

  func testAManifestNeverReachesOutOfItsFolder() throws {
    let folder = inbox.appendingPathComponent(ShareInbox.folderName(at: Date()), isDirectory: true)
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    let outside = inbox.appendingPathComponent("secret.obfpack")
    try Data("secret".utf8).write(to: outside)
    try ShareManifest(photos: ["../secret.obfpack"], files: ["../secret.obfpack", "manifest.json"])
      .encoded().write(to: folder.appendingPathComponent(ShareInbox.manifestName))

    XCTAssertEqual(ShareInbox.takeShare(from: inbox)?.photos, [])
    XCTAssertEqual(ShareInbox.takeFiles(from: inbox, to: inbox.appendingPathComponent("taken")), [])
    XCTAssertTrue(FileManager.default.fileExists(atPath: outside.path))
    XCTAssertNil(ShareInbox.inside(folder, ".."))
    XCTAssertNil(ShareInbox.inside(folder, "a/b"))
    XCTAssertNotNil(ShareInbox.inside(folder, "a.jpg"))
  }

  // MARK: Photos

  func testASharedPictureBecomesAnUprightJpegOfAtMost2048() throws {
    let url = try XCTUnwrap(
      Bundle(for: ShareInboxTests.self).url(forResource: "german-receipt", withExtension: "jpg"))
    let photo = try XCTUnwrap(PhotoShape.normalize(try Data(contentsOf: url)))
    let image = try XCTUnwrap(UIImage(data: photo))
    XCTAssertLessThanOrEqual(max(image.size.width, image.size.height), 2048)
    XCTAssertEqual(Array(photo.prefix(2)), [0xFF, 0xD8], "a JPEG")
  }

  func testATransparentPictureGoesOnWhiteAndALargeOneShrinks() throws {
    let size = CGSize(width: 4000, height: 1000)
    let format = UIGraphicsImageRendererFormat()
    format.scale = 1
    format.opaque = false
    let png = try XCTUnwrap(
      UIGraphicsImageRenderer(size: size, format: format).image { _ in }.pngData())
    let photo = try XCTUnwrap(PhotoShape.normalize(png))
    let image = try XCTUnwrap(UIImage(data: photo)?.cgImage)
    XCTAssertEqual(image.width, 2048)
    XCTAssertEqual(image.height, 512)
    // A see-through pixel is white now.
    let pixel = try XCTUnwrap(image.cropping(to: CGRect(x: 10, y: 10, width: 1, height: 1)))
    var rgba = [UInt8](repeating: 0, count: 4)
    let context = try XCTUnwrap(
      CGContext(
        data: &rgba, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
        space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
    context.draw(pixel, in: CGRect(x: 0, y: 0, width: 1, height: 1))
    XCTAssertGreaterThan(rgba[0], 245)
    XCTAssertGreaterThan(rgba[1], 245)
    XCTAssertGreaterThan(rgba[2], 245)
  }

  func testNoPictureIsNoPhoto() {
    XCTAssertNil(PhotoShape.normalize(Data("not a picture".utf8)))
  }
}
