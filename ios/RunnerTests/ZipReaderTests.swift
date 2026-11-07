import XCTest

@testable import Runner

/// Reading a Google Takeout zip on the iPhone (#240): the names, and a
/// file copied out stored or deflated, never more than asked for.
class ZipReaderTests: XCTestCase {
  private var dir: URL!

  override func setUpWithError() throws {
    dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
  }

  override func tearDownWithError() throws {
    try? FileManager.default.removeItem(at: dir)
  }

  private func le16(_ value: Int) -> [UInt8] { [UInt8(value & 0xFF), UInt8(value >> 8 & 0xFF)] }
  private func le32(_ value: Int) -> [UInt8] { le16(value & 0xFFFF) + le16(value >> 16 & 0xFFFF) }

  /// A zip of [files] (name, bytes, deflated), as a zip tool writes one:
  /// local headers with the data, the central directory, its end.
  private func zip(_ files: [(String, Data, Bool)]) throws -> FileHandle {
    var out: [UInt8] = []
    var central: [UInt8] = []
    for (name, plain, deflate) in files {
      var data = plain
      if deflate { data = try (plain as NSData).compressed(using: .zlib) as Data }
      let nameBytes = Array(name.utf8)
      let offset = out.count
      let method = deflate ? 8 : 0
      out += le32(0x0403_4b50) + le16(20) + le16(0x0800) + le16(method) + le32(0) + le32(0)
      out += le32(data.count) + le32(plain.count) + le16(nameBytes.count) + le16(0)
      out += nameBytes + [UInt8](data)
      central += le32(0x0201_4b50) + le16(20) + le16(20) + le16(0x0800) + le16(method)
      central += le32(0) + le32(0) + le32(data.count) + le32(plain.count)
      central += le16(nameBytes.count) + le16(0) + le16(0) + le16(0) + le16(0) + le32(0)
      central += le32(offset) + nameBytes
    }
    let directoryOffset = out.count
    out += central
    out += le32(0x0605_4b50) + le16(0) + le16(0) + le16(files.count) + le16(files.count)
    out += le32(central.count) + le32(directoryOffset) + le16(0)
    let url = dir.appendingPathComponent("takeout.zip")
    try Data(out).write(to: url)
    return try FileHandle(forReadingFrom: url)
  }

  private func copy(_ file: FileHandle, _ entry: ZipEntry, most: Int64) throws -> Data {
    var copied = Data()
    let count = try ZipReader.extract(file, entry, most: most) { copied.append($0) }
    XCTAssertEqual(Int64(copied.count), count)
    return copied
  }

  func testTheNamesComeInTheZipsOrder() throws {
    let file = try zip([
      ("Takeout/Keep/", Data(), false),
      ("Takeout/Keep/Shopping.json", Data("{}".utf8), false),
      ("Takeout/Keep/Bild ä.png", Data([1, 2, 3]), true),
    ])
    let entries = try ZipReader.entries(file)
    XCTAssertEqual(
      entries.map(\.name),
      ["Takeout/Keep/", "Takeout/Keep/Shopping.json", "Takeout/Keep/Bild ä.png"])
    XCTAssertEqual(entries.map(\.isDirectory), [true, false, false])
  }

  func testStoredAndDeflatedFilesComeOutWhole() throws {
    let text = Data(String(repeating: "Milk, eggs, bread. ", count: 5000).utf8)
    let file = try zip([("a.json", text, false), ("b.json", text, true)])
    let entries = try ZipReader.entries(file)
    XCTAssertLessThan(entries[1].compressedSize, entries[1].size)
    XCTAssertEqual(try copy(file, entries[0], most: 1 << 20), text)
    XCTAssertEqual(try copy(file, entries[1], most: 1 << 20), text)
  }

  func testNoMoreThanAskedForIsCopied() throws {
    let text = Data(String(repeating: "x", count: 300_000).utf8)
    let file = try zip([("a.json", text, false), ("b.json", text, true)])
    let entries = try ZipReader.entries(file)
    XCTAssertEqual(try copy(file, entries[0], most: 1001), text.prefix(1001))
    XCTAssertEqual(try copy(file, entries[1], most: 1001), text.prefix(1001))
  }

  func testAFileThatIsNoZipFails() throws {
    let url = dir.appendingPathComponent("no.zip")
    try Data("just text".utf8).write(to: url)
    XCTAssertThrowsError(try ZipReader.entries(try FileHandle(forReadingFrom: url)))
  }

  func testZip64SizesAreReadFromTheExtraField() {
    let bytes: [UInt8] =
      [0x01, 0x00, 0x10, 0x00] + [UInt8](repeating: 0xAB, count: 8) + [5, 0, 0, 0, 1, 0, 0, 0]
    let extra = Data(bytes)
    let field = ZipReader.field(0x0001, in: extra)
    XCTAssertEqual(field?.count, 16)
    XCTAssertEqual(field.map { ZipReader.u64($0, 8) }, 0x1_0000_0005)
    XCTAssertNil(ZipReader.field(0x5455, in: extra))
  }
}
