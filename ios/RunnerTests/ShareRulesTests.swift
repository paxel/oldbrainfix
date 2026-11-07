import ImageIO
import UIKit
import UniformTypeIdentifiers
import XCTest

@testable import Runner

/// What goes out of the app and what Paste takes (#243): only files in
/// the exports leave, the clipboard's text and pictures are taken as
/// Clipboards.kt takes them, the camera's photo comes upright, and the
/// lock's prompt always has a reason.
class ShareRulesTests: XCTestCase {
  private var dir: URL!

  override func setUpWithError() throws {
    dir = FileManager.default.temporaryDirectory
      .appendingPathComponent("share-rules-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
  }

  override func tearDownWithError() throws {
    try? FileManager.default.removeItem(at: dir)
  }

  // MARK: Exports

  func testANameKeepsNoDirectory() {
    XCTAssertEqual(Exports.safeName("../../etc/passwd"), "passwd")
    XCTAssertEqual(Exports.safeName("2026-11-05.obfsync"), "2026-11-05.obfsync")
    XCTAssertEqual(Exports.safeName(""), "export")
    XCTAssertEqual(Exports.safeName("  "), "export")
    XCTAssertEqual(Exports.safeName(".."), "export")
  }

  func testTheExportPathLiesInTheExports() {
    let path = Exports.path(for: "../sheet.pdf", in: dir)
    XCTAssertTrue(Exports.inside(dir, path))
    XCTAssertEqual((path as NSString).lastPathComponent, "sheet.pdf")
  }

  func testOnlyAFileDirectlyInTheExportsGoesOut() throws {
    let file = dir.appendingPathComponent("photo.jpg")
    try Data([1, 2, 3]).write(to: file)
    XCTAssertEqual(Exports.exported(file.path, in: dir)?.lastPathComponent, "photo.jpg")
    XCTAssertNil(Exports.exported(dir.appendingPathComponent("missing.jpg").path, in: dir))
    XCTAssertNil(Exports.exported(dir.path, in: dir))
    XCTAssertNil(Exports.exported(dir.appendingPathComponent("../photo.jpg").path, in: dir))
    let sub = dir.appendingPathComponent("sub", isDirectory: true)
    try FileManager.default.createDirectory(at: sub, withIntermediateDirectories: true)
    let deeper = sub.appendingPathComponent("photo.jpg")
    try Data([1]).write(to: deeper)
    XCTAssertNil(Exports.exported(deeper.path, in: dir))
    XCTAssertNil(Exports.exported("/etc/hosts", in: dir))
  }

  func testAPhotoGoesOnTheClipboardAsItsType() {
    XCTAssertEqual(Exports.pasteboardType(mime: "image/png"), UTType.png.identifier)
    XCTAssertEqual(Exports.pasteboardType(mime: "image/jpeg"), UTType.jpeg.identifier)
    XCTAssertEqual(Exports.pasteboardType(mime: "nonsense"), UTType.jpeg.identifier)
  }

  // MARK: Paste

  func testTextOnePerLineBlankOnesLeftOutAndLinksAsText() {
    let (text, pictures) = Clipboard.taken([
      [UTType.utf8PlainText.identifier: "milk"],
      [UTType.utf8PlainText.identifier: "   "],
      [UTType.url.identifier: URL(string: "https://example.org/sauce") as Any],
      [UTType.plainText.identifier: Data("eggs".utf8)],
    ])
    XCTAssertEqual(text, "milk\nhttps://example.org/sauce\neggs")
    XCTAssertTrue(pictures.isEmpty)
  }

  func testPicturesAreTakenOtherFilesNot() {
    let png = Data([0x89, 0x50, 0x4E, 0x47])
    let (text, pictures) = Clipboard.taken([
      [UTType.png.identifier: png],
      [UTType.vCard.identifier: Data("BEGIN:VCARD".utf8)],
      [UTType.fileURL.identifier: URL(fileURLWithPath: "/tmp/x") as Any],
    ])
    XCTAssertEqual(text, "")
    XCTAssertEqual(pictures, [png])
  }

  // MARK: Camera

  /// The camera hands a picture held sideways with its orientation
  /// beside it; the photo kept is upright, its pixels turned.
  func testTheCamerasPhotoComesUpright() throws {
    let size = CGSize(width: 300, height: 100)
    let format = UIGraphicsImageRendererFormat()
    format.scale = 1
    let drawn = UIGraphicsImageRenderer(size: size, format: format).image { context in
      UIColor.red.setFill()
      context.fill(CGRect(origin: .zero, size: size))
    }
    let sideways = UIImage(cgImage: try XCTUnwrap(drawn.cgImage), scale: 1, orientation: .right)
    let photo = try XCTUnwrap(PhotoShape.normalize(image: sideways))
    let source = try XCTUnwrap(CGImageSourceCreateWithData(photo as CFData, nil))
    let image = try XCTUnwrap(CGImageSourceCreateImageAtIndex(source, 0, nil))
    XCTAssertEqual(image.width, 100)
    XCTAssertEqual(image.height, 300)
    let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any] ?? [:]
    let orientation = properties[kCGImagePropertyOrientation] as? Int ?? 1
    XCTAssertEqual(orientation, 1, "no orientation left for a viewer to apply")
    XCTAssertNil(properties[kCGImagePropertyGPSDictionary])
  }

  // MARK: Lock

  func testThePromptAlwaysHasAReason() {
    XCTAssertEqual(LockReason.of("Unlock Old Brain Fix"), "Unlock Old Brain Fix")
    XCTAssertEqual(LockReason.of("  "), "Old Brain Fix")
  }
}
