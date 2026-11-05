import Foundation

/// A paired Device's host as its record names it: its device id and
/// where it answers.
struct FoundHost: Equatable {
  let device: String
  let host: String
  let port: Int

  var channelMap: [String: Any] { ["device": device, "host": host, "port": port] }
}

/// What an mDNS/DNS-SD record of a paired host says, as NsdRecords in
/// Discovery.kt reads and makes it (#240): the service type, and the
/// device id as the instance name and its only TXT entry. Pure, for
/// XCTest.
enum SyncRecords {
  static let serviceType = "_oldbrainfix._tcp"
  static let idKey = "id"

  /// Whether [type], as Bonjour reports it ("_oldbrainfix._tcp." and the
  /// like), is ours.
  static func isOurs(_ type: String?) -> Bool {
    guard var trimmed = type?.trimmingCharacters(in: CharacterSet(charactersIn: ".")) else {
      return false
    }
    if trimmed.hasSuffix(".local") { trimmed = String(trimmed.dropLast(".local".count)) }
    return trimmed.trimmingCharacters(in: CharacterSet(charactersIn: ".")) == serviceType
  }

  /// The device id in the TXT [entries]: 32 lowercase hex digits, or nil.
  static func deviceId(_ entries: [String: Data]) -> String? {
    guard let raw = entries.first(where: { $0.key.lowercased() == idKey })?.value,
      let id = String(data: raw, encoding: .ascii), isDeviceId(id)
    else { return nil }
    return id
  }

  /// The TXT entries of this Device's own record: its [device] id only, no
  /// name, since anyone on the network hears it. Nil when [device] is no
  /// device id or [port] no real port: then nothing is advertised.
  static func own(device: String, port: Int) -> [String: String]? {
    isDeviceId(device) && (1...65535).contains(port) ? [idKey: device] : nil
  }

  /// The host the record names, when it gives a device id, an IPv4 [host]
  /// and a [port].
  static func found(_ entries: [String: Data], host: String?, port: Int) -> FoundHost? {
    guard let device = deviceId(entries), let host, isIPv4(host), (1...65535).contains(port)
    else { return nil }
    return FoundHost(device: device, host: host, port: port)
  }

  /// The entries of a TXT record in DNS's wire format: each a length byte
  /// and `key=value`; a key without `=` has no value.
  static func txtEntries(_ record: Data) -> [String: Data] {
    var entries: [String: Data] = [:]
    let bytes = [UInt8](record)
    var at = 0
    while at < bytes.count {
      let length = Int(bytes[at])
      let end = min(at + 1 + length, bytes.count)
      let entry = bytes[(at + 1)..<end]
      at = end
      guard !entry.isEmpty else { continue }
      if let equals = entry.firstIndex(of: UInt8(ascii: "=")) {
        let key = String(decoding: entry[entry.startIndex..<equals], as: UTF8.self)
        if entries[key] == nil { entries[key] = Data(entry[(equals + 1)...]) }
      } else {
        let key = String(decoding: entry, as: UTF8.self)
        if entries[key] == nil { entries[key] = Data() }
      }
    }
    return entries
  }

  /// [entries] as a TXT record in DNS's wire format.
  static func txtRecord(_ entries: [String: String]) -> Data {
    var record = Data()
    for (key, value) in entries.sorted(by: { $0.key < $1.key }) {
      let entry = Array("\(key)=\(value)".utf8.prefix(255))
      record.append(UInt8(entry.count))
      record.append(contentsOf: entry)
    }
    return record
  }

  static func isDeviceId(_ id: String) -> Bool {
    id.count == 32 && id.unicodeScalars.allSatisfy { "0123456789abcdef".unicodeScalars.contains($0) }
  }

  static func isIPv4(_ host: String) -> Bool {
    let parts = host.split(separator: ".", omittingEmptySubsequences: false)
    return parts.count == 4
      && parts.allSatisfy { part in
        (1...3).contains(part.count) && part.allSatisfy { $0.isASCII && $0.isNumber }
      }
  }

  /// The dotted IPv4 address in a `sockaddr_in` as Bonjour hands it
  /// (length, family, port, address), or nil for any other family.
  static func ipv4(sockaddr: Data) -> String? {
    let bytes = [UInt8](sockaddr)
    guard bytes.count >= 8, bytes[1] == UInt8(AF_INET) else { return nil }
    return bytes[4..<8].map { String($0) }.joined(separator: ".")
  }
}

/// When the background sync runs on the iPhone (#240), as SyncWork.kt
/// and SyncWorker.kt decide it: the task is asked for again each time,
/// no sooner than Android's period, and it goes on only with a shared
/// folder or on Wi-Fi or Ethernet. iOS decides when it really runs, if
/// at all; nothing promises a time.
enum BackgroundPlan {
  static let identifier = "io.github.paxel.oldbrainfix.sync"

  /// Android's shortest period: no sooner than this is asked for.
  static let period: TimeInterval = 15 * 60

  /// When the next run is asked for, while the work is [on]; nil when it
  /// is off and the task is cancelled.
  static func next(on: Bool, now: Date) -> Date? {
    on ? now.addingTimeInterval(period) : nil
  }

  /// Whether a run is asked for: while the work is [on] and none is
  /// [pending] already, so its rhythm stays, as on Android.
  static func asks(on: Bool, pending: Bool) -> Bool {
    on && !pending
  }

  /// Whether a run goes on: with a shared folder anywhere, else only on
  /// a local network, where paired Devices can be.
  static func runs(folder: Bool, localNetwork: Bool) -> Bool {
    folder || localNetwork
  }
}

/// The name this iPhone goes by in a sync, as Pairing.kt makes it: the
/// model, with the maker in front unless the model names it already.
/// iOS gives apps the model ("iPhone"), not the name the user gave it.
enum DeviceNames {
  static func deviceName(maker: String, model: String) -> String {
    let maker = maker.trimmingCharacters(in: .whitespaces)
    let model = model.trimmingCharacters(in: .whitespaces)
    if model.isEmpty { return maker }
    if maker.isEmpty || model.lowercased().hasPrefix(maker.lowercased()) { return model }
    return "\(maker.prefix(1).uppercased())\(maker.dropFirst()) \(model)"
  }
}
