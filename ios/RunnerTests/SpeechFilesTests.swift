import XCTest

@testable import Runner

/// The speech model's checksum on the iPhone (#239), as Speech.kt keeps
/// it: hashed while copied, recorded with the file's size and time, and
/// hashed again when the file changed; and the samples Whisper wants.
class SpeechFilesTests: XCTestCase {
  private var dir: URL!

  override func setUpWithError() throws {
    dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
  }

  override func tearDownWithError() throws {
    try? FileManager.default.removeItem(at: dir)
  }

  private func file(_ name: String, _ text: String) throws -> URL {
    let url = dir.appendingPathComponent(name)
    try Data(text.utf8).write(to: url)
    return url
  }

  /// SHA-256 of "abc" (FIPS 180-2).
  private let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

  func testTheHashIsSha256AsLowercaseHex() throws {
    XCTAssertEqual(SpeechFiles.hash(try file("model", "abc")), abc)
    XCTAssertNil(SpeechFiles.hash(dir.appendingPathComponent("none")))
  }

  func testACopyIsHashedAndWrittenWhole() throws {
    let bytes = String(repeating: "x", count: 3 << 20)
    let input = try FileHandle(forReadingFrom: try file("in", bytes))
    let out = dir.appendingPathComponent("out")
    FileManager.default.createFile(atPath: out.path, contents: nil)
    let output = try FileHandle(forWritingTo: out)
    var told: [Int64] = []
    let hash = SpeechFiles.copyHashing(input, to: output, copied: { told.append($0) })
    try output.close()
    XCTAssertEqual(hash, SpeechFiles.hash(out))
    XCTAssertEqual(try Data(contentsOf: out).count, 3 << 20)
    XCTAssertEqual(told.last, 3 << 20)
    XCTAssertGreaterThanOrEqual(told.count, 3)
  }

  func testAStoppedCopyHasNoHash() throws {
    let input = try FileHandle(forReadingFrom: try file("in", "abc"))
    XCTAssertNil(SpeechFiles.copyHashing(input, stop: { true }))
  }

  func testTheRecordIsTheAndroidLine() throws {
    let model = try file("ggml-base.bin", "abc")
    let record = dir.appendingPathComponent("ggml-base.bin.checked")
    SpeechFiles.writeRecord(model: model, record: record, sha256: abc)
    let stamp = try XCTUnwrap(SpeechFiles.stamp(model))
    XCTAssertEqual(
      try String(contentsOf: record, encoding: .utf8), "\(abc) 3 \(stamp.modified)\n")
    XCTAssertEqual(
      SpeechFiles.readRecord(record),
      SpeechFiles.ModelRecord(sha256: abc, size: 3, modified: stamp.modified))
  }

  func testWhatIsNoRecordIsNone() throws {
    for text in [
      "", "\(abc) 3", "\(abc.uppercased()) 3 4", "abc 3 4", "\(abc) three 4", "\(abc) 3 4 5",
    ] {
      XCTAssertNil(SpeechFiles.readRecord(try file("record", text)), text)
    }
    XCTAssertNil(SpeechFiles.readRecord(dir.appendingPathComponent("none")))
  }

  /// The record spares hashing the model while the file is unchanged:
  /// its hash is taken as it stands, here one that is not the file's.
  func testTheRecordedHashIsTakenWhileTheFileIsUnchanged() throws {
    let model = try file("ggml-base.bin", "abc")
    let record = dir.appendingPathComponent("ggml-base.bin.checked")
    let recorded = String(repeating: "0", count: 64)
    SpeechFiles.writeRecord(model: model, record: record, sha256: recorded)
    XCTAssertEqual(SpeechFiles.checkedHash(model: model, record: record), recorded)
  }

  func testAChangedFileIsHashedAgainAndRecorded() throws {
    let model = try file("ggml-base.bin", "abc")
    let record = dir.appendingPathComponent("ggml-base.bin.checked")
    SpeechFiles.writeRecord(model: model, record: record, sha256: String(repeating: "0", count: 64))
    try Data("abcd".utf8).write(to: model)
    let hash = SpeechFiles.checkedHash(model: model, record: record)
    XCTAssertEqual(hash, SpeechFiles.hash(model))
    XCTAssertNotEqual(hash, abc)
    XCTAssertEqual(SpeechFiles.readRecord(record)?.sha256, hash)
  }

  func testNoModelHasNoHash() {
    let record = dir.appendingPathComponent("ggml-base.bin.checked")
    XCTAssertNil(SpeechFiles.checkedHash(model: dir.appendingPathComponent("none"), record: record))
    XCTAssertNil(SpeechFiles.checkedHash(model: dir, record: record))
  }

  func testAKeptModelReplacesTheOneBeforeWithItsRecord() throws {
    let model = try file("ggml-base.bin", "old")
    let record = dir.appendingPathComponent("ggml-base.bin.checked")
    SpeechFiles.writeRecord(model: model, record: record, sha256: String(repeating: "1", count: 64))
    let incoming = try file("ggml-base.bin.part", "abc")
    XCTAssertTrue(SpeechFiles.keep(incoming: incoming, model: model, record: record, hash: abc))
    XCTAssertFalse(FileManager.default.fileExists(atPath: incoming.path))
    XCTAssertEqual(try String(contentsOf: model, encoding: .utf8), "abc")
    XCTAssertEqual(SpeechFiles.checkedHash(model: model, record: record), abc)
  }

  func testAKeptModelWithoutAHashHasNoRecord() throws {
    let model = dir.appendingPathComponent("ggml-base.bin")
    let record = try file("ggml-base.bin.checked", "\(abc) 3 4")
    let incoming = try file("ggml-base.bin.part", "abc")
    XCTAssertTrue(SpeechFiles.keep(incoming: incoming, model: model, record: record, hash: nil))
    XCTAssertFalse(FileManager.default.fileExists(atPath: record.path))
    XCTAssertFalse(
      SpeechFiles.keep(
        incoming: dir.appendingPathComponent("none"), model: model, record: record, hash: abc))
  }

  /// The checksums the app pins (speech_platform.dart) are what this
  /// hashes to: lowercase hex of 64 digits, as a record holds them.
  func testThePinnedChecksumsAreRecords() throws {
    for pinned in [
      "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
      "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    ] {
      XCTAssertEqual(SpeechFiles.readRecord(try file("record", "\(pinned) 1 2"))?.sha256, pinned)
    }
  }

  func testWhisperGetsOneThreadFewerThanTheCoresAtMostFour() {
    XCTAssertEqual(SpeechFiles.threads(cores: 1), 1)
    XCTAssertEqual(SpeechFiles.threads(cores: 2), 1)
    XCTAssertEqual(SpeechFiles.threads(cores: 4), 3)
    XCTAssertEqual(SpeechFiles.threads(cores: 6), 4)
  }

  func testChannelsAreAveragedAndResampledTo16kHz() {
    XCTAssertEqual(SpeechFiles.downmix([[1, 0.5], [0, -0.5]]), [0.5, 0])
    XCTAssertEqual(SpeechFiles.downmix([[0.25, 0.75]]), [0.25, 0.75])
    XCTAssertEqual(SpeechFiles.downmix([]), [])
    XCTAssertEqual(SpeechFiles.resample([0, 1, 0, 1], from: 32_000, to: 16_000), [0, 0])
    XCTAssertEqual(SpeechFiles.resample([0, 1], from: 8_000, to: 16_000), [0, 0.5, 1, 1])
    XCTAssertEqual(SpeechFiles.toWhisper([[0.1, 0.2]], rate: 16_000), [0.1, 0.2])
  }

  func testRecordingsAreNamedByTheirBytes() {
    func named(_ bytes: [UInt8]) -> String { VoiceFiles.fileExtension(Data(bytes)) }
    XCTAssertEqual(named(Array("RIFF\0\0\0\0WAVE".utf8)), "wav")
    XCTAssertEqual(named(Array("OggS".utf8)), "ogg")
    XCTAssertEqual(named(Array("ID3\u{3}".utf8)), "mp3")
    XCTAssertEqual(named([0xFF, 0xFB, 0x90, 0x00]), "mp3")
    // ADTS AAC: the same sync, no layer.
    XCTAssertEqual(named([0xFF, 0xF1, 0x50, 0x80]), "m4a")
    XCTAssertEqual(named(Array("\0\0\0\u{18}ftypM4A ".utf8)), "m4a")
    XCTAssertEqual(named([]), "m4a")
  }
}
