import CryptoKit
import Foundation

/// The rules of Speech.kt's companion on the iPhone (#239), free of
/// Flutter, AVFoundation and Whisper: the model's SHA-256 and the record
/// that spares hashing it on every start, and the samples Whisper wants.
/// The app's Dart side checks every hash against the checksums it pins
/// (WhisperModel in speech_platform.dart) before a model is kept or used.
enum SpeechFiles {
  /// The model in use, named after the first one there was, whichever it
  /// is now; the same name as on Android.
  static let modelName = "ggml-base.bin"

  /// Whisper's own rate.
  static let whisperRate = 16_000

  /// How many threads Whisper (and RustO) get on [cores] cores: one is
  /// left for the app, at most four.
  static func threads(cores: Int) -> Int {
    min(max(cores - 1, 1), 4)
  }

  /// [bytes] as lowercase hex.
  static func hex<D: Sequence>(_ bytes: D) -> String where D.Element == UInt8 {
    bytes.map { String(format: "%02x", $0) }.joined()
  }

  /// Copies [input] to [output], if there is one, and answers the SHA-256
  /// of what was copied, as lowercase hex; nil when [stop] said so on the
  /// way or it could not be read or written. [copied] hears the bytes so
  /// far, about every megabyte and at the end.
  static func copyHashing(
    _ input: FileHandle, to output: FileHandle? = nil, stop: () -> Bool = { false },
    copied: (Int64) -> Void = { _ in }
  ) -> String? {
    var hasher = SHA256()
    var total: Int64 = 0
    var told: Int64 = 0
    while true {
      if stop() { return nil }
      let chunk: Data
      do {
        guard let read = try input.read(upToCount: 64 * 1024), !read.isEmpty else { break }
        chunk = read
        try output?.write(contentsOf: chunk)
      } catch {
        return nil
      }
      hasher.update(data: chunk)
      total += Int64(chunk.count)
      if total - told >= 1 << 20 {
        told = total
        copied(total)
      }
    }
    copied(total)
    return hex(hasher.finalize())
  }

  /// The SHA-256 of the file at [url], or nil when it cannot be read.
  static func hash(_ url: URL, stop: () -> Bool = { false }) -> String? {
    guard let input = try? FileHandle(forReadingFrom: url) else { return nil }
    defer { try? input.close() }
    return copyHashing(input, stop: stop)
  }

  /// A model's SHA-256 as recorded, with its size in bytes and
  /// modification time in milliseconds then.
  struct ModelRecord: Equatable {
    let sha256: String
    let size: Int64
    let modified: Int64
  }

  /// The size and modification time in milliseconds of the file at
  /// [url], or nil when there is no file.
  static func stamp(_ url: URL) -> (size: Int64, modified: Int64)? {
    var directory: ObjCBool = false
    guard FileManager.default.fileExists(atPath: url.path, isDirectory: &directory),
      !directory.boolValue,
      let attributes = try? FileManager.default.attributesOfItem(atPath: url.path),
      let size = (attributes[.size] as? NSNumber)?.int64Value,
      let date = attributes[.modificationDate] as? Date
    else { return nil }
    return (size, Int64((date.timeIntervalSince1970 * 1000).rounded(.down)))
  }

  /// The SHA-256 of [model], or nil when there is none. It is taken from
  /// [record] while the file's size and modification time are still the
  /// ones recorded; else the whole file is hashed and the record written
  /// anew.
  static func checkedHash(model: URL, record: URL) -> String? {
    guard let stamp = stamp(model) else { return nil }
    if let recorded = readRecord(record), recorded.size == stamp.size,
      recorded.modified == stamp.modified
    {
      return recorded.sha256
    }
    guard let hash = hash(model) else { return nil }
    writeRecord(model: model, record: record, sha256: hash)
    return hash
  }

  /// Moves [incoming] to [model] and records [hash] for it, if there is
  /// one; whether it moved. The record of the model before goes first, so
  /// it never stands for another file.
  static func keep(incoming: URL, model: URL, record: URL, hash: String?) -> Bool {
    try? FileManager.default.removeItem(at: record)
    guard rename(incoming.path, model.path) == 0 else { return false }
    if let hash { writeRecord(model: model, record: record, sha256: hash) }
    return true
  }

  /// Records [sha256] for [model] with its size and modification time
  /// now, in [record]: one line, `<sha256> <bytes> <milliseconds>`, as on
  /// Android. Written to the side and renamed, so it is whole or not
  /// there.
  static func writeRecord(model: URL, record: URL, sha256: String) {
    guard let stamp = stamp(model) else { return }
    let side = URL(fileURLWithPath: record.path + ".part")
    do {
      try Data("\(sha256) \(stamp.size) \(stamp.modified)\n".utf8).write(to: side)
      if rename(side.path, record.path) != 0 { try? FileManager.default.removeItem(at: side) }
    } catch {
      try? FileManager.default.removeItem(at: side)
    }
  }

  /// The record in [record], or nil when there is none or it is not one:
  /// anything but a lowercase hex SHA-256, a size and a time is no
  /// record, so the model is hashed again rather than dropped.
  static func readRecord(_ record: URL) -> ModelRecord? {
    guard let data = try? Data(contentsOf: record), let text = String(data: data, encoding: .utf8)
    else { return nil }
    let parts = text.trimmingCharacters(in: .whitespacesAndNewlines).split(
      separator: " ", omittingEmptySubsequences: false)
    guard parts.count == 3, isSha256(parts[0]), let size = Int64(parts[1]),
      let modified = Int64(parts[2])
    else { return nil }
    return ModelRecord(sha256: String(parts[0]), size: size, modified: modified)
  }

  private static func isSha256(_ text: Substring) -> Bool {
    text.utf8.count == 64
      && text.utf8.allSatisfy { (0x30...0x39).contains($0) || (0x61...0x66).contains($0) }
  }

  /// The average of the [channels] of a recording, each its samples;
  /// a channel shorter than the first cuts the rest off.
  static func downmix(_ channels: [[Float]]) -> [Float] {
    guard let first = channels.first else { return [] }
    if channels.count == 1 { return first }
    let frames = channels.map(\.count).min() ?? 0
    let count = Float(channels.count)
    return (0..<frames).map { frame in
      channels.reduce(Float(0)) { $0 + $1[frame] } / count
    }
  }

  /// [samples] at [from] Hz linearly interpolated to [to] Hz, as on
  /// Android.
  static func resample(_ samples: [Float], from: Int, to: Int) -> [Float] {
    if from == to || samples.isEmpty || from <= 0 { return samples }
    let count = Int(Int64(samples.count) * Int64(to) / Int64(from))
    let step = Double(from) / Double(to)
    let last = samples.count - 1
    return (0..<count).map { i in
      let at = Double(i) * step
      let left = min(Int(at), last)
      let right = min(left + 1, last)
      let part = Float(at - Double(left))
      return samples[left] + (samples[right] - samples[left]) * part
    }
  }

  /// The channels of a recording at [rate] Hz as Whisper wants them:
  /// mono, the channels averaged, at 16 kHz. A recording made by Voice
  /// already is, and comes back as it is.
  static func toWhisper(_ channels: [[Float]], rate: Int) -> [Float] {
    resample(downmix(channels), from: rate, to: whisperRate)
  }
}
