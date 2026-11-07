import UIKit
import XCTest

@testable import Runner

/// Photo Texts on the iPhone (#239): RustO linked in, with the models
/// bundled as on Android, reading a photo through the C functions.
class OcrTests: XCTestCase {

  /// Reads [jpeg] on the reader's queue and waits for the answer.
  private func read(_ jpeg: Data, with ocr: Ocr = Ocr()) -> String? {
    let done = expectation(description: "read")
    var text: String?
    ocr.read(jpeg) {
      text = $0
      done.fulfill()
    }
    wait(for: [done], timeout: 120)
    ocr.idle()
    return text
  }

  /// A JPEG of [width] × [height] pixels in [color].
  private func jpeg(_ color: UIColor, width: Int, height: Int) -> Data {
    let format = UIGraphicsImageRendererFormat()
    format.scale = 1
    format.preferredRange = .standard
    let renderer = UIGraphicsImageRenderer(
      size: CGSize(width: width, height: height), format: format)
    let image = renderer.image { context in
      color.setFill()
      context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    }
    return image.jpegData(compressionQuality: 1) ?? Data()
  }

  func testTheModelsAreBundled() {
    XCTAssertTrue(Ocr().supported)
  }

  /// RunnerTests' german-receipt.jpg is android/ocr/testdata's, the
  /// crate's own test of the German letters and the euro sign.
  func testTheBundledModelsReadAReceipt() throws {
    let url = try XCTUnwrap(
      Bundle(for: OcrTests.self).url(forResource: "german-receipt", withExtension: "jpg"))
    let text = try XCTUnwrap(read(try Data(contentsOf: url)))
    XCTAssertTrue(text.contains("Bäckerei Müller GmbH"), text)
  }

  func testABlankPhotoHasNoText() {
    XCTAssertEqual(read(jpeg(.white, width: 64, height: 48)), "")
  }

  func testWhatIsNoPhotoIsNotRead() {
    XCTAssertNil(read(Data("not a photo".utf8)))
  }

  func testPixelsAreRgbaRowByRowFromTheTop() throws {
    let format = UIGraphicsImageRendererFormat()
    format.scale = 1
    format.preferredRange = .standard
    let renderer = UIGraphicsImageRenderer(size: CGSize(width: 2, height: 2), format: format)
    // Red on top, blue below: a PNG, so the colours stay exact.
    let png = renderer.pngData { context in
      UIColor.red.setFill()
      context.fill(CGRect(x: 0, y: 0, width: 2, height: 1))
      UIColor.blue.setFill()
      context.fill(CGRect(x: 0, y: 1, width: 2, height: 1))
    }
    let picture = try XCTUnwrap(Ocr.rgba(png))
    XCTAssertEqual(picture.width, 2)
    XCTAssertEqual(picture.height, 2)
    XCTAssertEqual(picture.pixels.count, 2 * 2 * 4)
    XCTAssertEqual(Array(picture.pixels[0..<3]), [255, 0, 0])
    XCTAssertEqual(Array(picture.pixels[8..<11]), [0, 0, 255])
  }
}
