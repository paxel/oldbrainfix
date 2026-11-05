import Compression
import Foundation

/// One file or folder inside a zip, as its central directory names it.
struct ZipEntry: Equatable {
  let name: String
  let method: UInt16
  let compressedSize: UInt64
  let size: UInt64
  let localOffset: UInt64

  var isDirectory: Bool { name.hasSuffix("/") }
}

enum ZipError: Error {
  case notAZip
  case unsupported(UInt16)
}

/// Reading a zip, such as a Google Takeout export, on the iPhone (#240),
/// where Foundation has no ZipInputStream: the names from the central
/// directory (ZIP64 too), and a file copied out stored or deflated, at
/// most so many bytes, so a large one is never copied whole. Pure, so
/// XCTest reads zips made by hand.
enum ZipReader {
  /// Every entry of the zip [file], in the zip's order.
  static func entries(_ file: FileHandle) throws -> [ZipEntry] {
    let size = try file.seekToEnd()
    let tail = min(size, 22 + 65_535)
    try file.seek(toOffset: size - tail)
    let end = try file.read(upToCount: Int(tail)) ?? Data()
    guard let eocd = lastIndex(of: 0x0605_4b50, in: end) else { throw ZipError.notAZip }
    var count = UInt64(u16(end, eocd + 10))
    var directorySize = UInt64(u32(end, eocd + 12))
    var directoryOffset = UInt64(u32(end, eocd + 16))
    if count == 0xFFFF || directorySize == 0xFFFF_FFFF || directoryOffset == 0xFFFF_FFFF {
      // ZIP64: the locator just before says where the real record is.
      guard eocd >= 20, u32(end, eocd - 20) == 0x0706_4b50 else { throw ZipError.notAZip }
      try file.seek(toOffset: u64(end, eocd - 20 + 8))
      let record = try file.read(upToCount: 56) ?? Data()
      guard record.count == 56, u32(record, 0) == 0x0606_4b50 else { throw ZipError.notAZip }
      count = u64(record, 32)
      directorySize = u64(record, 40)
      directoryOffset = u64(record, 48)
    }
    try file.seek(toOffset: directoryOffset)
    let directory = try file.read(upToCount: Int(directorySize)) ?? Data()
    var entries: [ZipEntry] = []
    var at = 0
    while UInt64(entries.count) < count, at + 46 <= directory.count, u32(directory, at) == 0x0201_4b50 {
      let nameLength = Int(u16(directory, at + 28))
      let extraLength = Int(u16(directory, at + 30))
      let commentLength = Int(u16(directory, at + 32))
      guard at + 46 + nameLength + extraLength <= directory.count else { throw ZipError.notAZip }
      let nameBytes = slice(directory, at + 46, nameLength)
      let name =
        String(data: nameBytes, encoding: .utf8) ?? String(decoding: nameBytes, as: UTF8.self)
      var compressed = UInt64(u32(directory, at + 20))
      var plain = UInt64(u32(directory, at + 24))
      var offset = UInt64(u32(directory, at + 42))
      let extra = slice(directory, at + 46 + nameLength, extraLength)
      if let zip64 = field(0x0001, in: extra) {
        // Only the values that did not fit are there, in this order.
        var next = 0
        func take(_ value: inout UInt64) {
          guard value == 0xFFFF_FFFF, next + 8 <= zip64.count else { return }
          value = u64(zip64, next)
          next += 8
        }
        take(&plain)
        take(&compressed)
        take(&offset)
      }
      entries.append(
        ZipEntry(
          name: name, method: u16(directory, at + 10), compressedSize: compressed, size: plain,
          localOffset: offset))
      at += 46 + nameLength + extraLength + commentLength
    }
    return entries
  }

  /// Copies [entry] of the zip [file] into [write], until it ends or
  /// [most] bytes are copied; answers how many were.
  static func extract(
    _ file: FileHandle, _ entry: ZipEntry, most: Int64, into write: (Data) throws -> Void
  ) throws -> Int64 {
    try file.seek(toOffset: entry.localOffset)
    let header = try file.read(upToCount: 30) ?? Data()
    guard header.count == 30, u32(header, 0) == 0x0403_4b50 else { throw ZipError.notAZip }
    let skip = UInt64(u16(header, 26)) + UInt64(u16(header, 28))
    try file.seek(toOffset: entry.localOffset + 30 + skip)
    var left = entry.compressedSize
    let readCompressed: (Int) -> Data? = { wanted in
      guard left > 0 else { return nil }
      let chunk = (try? file.read(upToCount: Int(min(UInt64(wanted), left)))) ?? Data()
      left -= UInt64(chunk.count)
      if chunk.isEmpty { left = 0 }
      return chunk.isEmpty ? nil : chunk
    }
    let next: () throws -> Data?
    switch entry.method {
    case 0:
      next = { readCompressed(64 * 1024) }
    case 8:
      // Raw DEFLATE, which Compression's zlib is.
      let inflating = try InputFilter(.decompress, using: .zlib, bufferCapacity: 64 * 1024) {
        readCompressed($0)
      }
      next = { try inflating.readData(ofLength: 64 * 1024) }
    default:
      throw ZipError.unsupported(entry.method)
    }
    var copied: Int64 = 0
    while copied < most, let chunk = try next(), !chunk.isEmpty {
      let room = Int(min(Int64(chunk.count), most - copied))
      try write(chunk.prefix(room))
      copied += Int64(room)
    }
    return copied
  }

  /// The data of the extra field [id] in [extra], if it is there.
  static func field(_ id: UInt16, in extra: Data) -> Data? {
    var at = 0
    while at + 4 <= extra.count {
      let length = Int(u16(extra, at + 2))
      if u16(extra, at) == id { return slice(extra, at + 4, min(length, extra.count - at - 4)) }
      at += 4 + length
    }
    return nil
  }

  private static func lastIndex(of signature: UInt32, in data: Data) -> Int? {
    guard data.count >= 22 else { return nil }
    for at in stride(from: data.count - 22, through: 0, by: -1) where u32(data, at) == signature {
      return at
    }
    return nil
  }

  private static func slice(_ data: Data, _ at: Int, _ count: Int) -> Data {
    let start = data.startIndex + at
    return data.subdata(in: start..<(start + count))
  }

  /// Little-endian numbers at [at] in [data].
  static func u16(_ data: Data, _ at: Int) -> UInt16 {
    let start = data.startIndex + at
    guard at >= 0, at + 2 <= data.count else { return 0 }
    return UInt16(data[start]) | UInt16(data[start + 1]) << 8
  }

  static func u32(_ data: Data, _ at: Int) -> UInt32 {
    UInt32(u16(data, at)) | UInt32(u16(data, at + 2)) << 16
  }

  static func u64(_ data: Data, _ at: Int) -> UInt64 {
    UInt64(u32(data, at)) | UInt64(u32(data, at + 4)) << 32
  }
}
