import 'dart:convert';

import 'pair_code.dart' show isPrivateHost;
import 'store.dart';

/// A Device this one has synced with in person (CONTEXT.md, Pairing),
/// kept as a local setting of this Device: what automatic sync needs to
/// find it again and know it is the same one.
class PairedDevice {
  /// Its device id.
  final String id;

  /// The name it goes by, as it said.
  final String name;

  /// The SHA-256 fingerprint of the certificate it hosts with, hex;
  /// empty for a Device that only ever joined.
  final String fingerprint;

  /// Where it was hosting last, if it hosted.
  final String? host;
  final int? port;

  /// When the last sync with it ended.
  final DateTime lastSync;

  /// Whether it is another person's Device, paired by invite to a Shared
  /// Area: it gets only what it shares (ADR 0013). Once one, always one.
  final bool shared;

  const PairedDevice({
    required this.id,
    required this.name,
    required this.fingerprint,
    required this.host,
    required this.port,
    required this.lastSync,
    this.shared = false,
  });

  Map<String, Object?> toJson() => {
    'id': id,
    'name': name,
    'fingerprint': fingerprint,
    'host': host,
    'port': port,
    'lastSync': lastSync.toUtc().toIso8601String(),
    if (shared) 'shared': true,
  };

  static PairedDevice? fromJson(Object? json) {
    if (json is! Map) return null;
    final id = json['id'];
    final name = json['name'];
    final fingerprint = json['fingerprint'];
    final host = json['host'];
    final port = json['port'];
    final lastSync = DateTime.tryParse('${json['lastSync']}');
    if (id is! String ||
        name is! String ||
        fingerprint is! String ||
        lastSync == null) {
      return null;
    }
    return PairedDevice(
      id: id,
      name: name,
      fingerprint: fingerprint,
      host: host is String ? host : null,
      port: port is int ? port : null,
      lastSync: lastSync,
      shared: json['shared'] == true,
    );
  }
}

/// Where a paired Device may be reached: a host and its port.
typedef PairedAddress = ({String host, int port});

/// An address typed by hand for a paired Device (#267, ADR 0003), such as
/// a Tailscale name or an address of a VPN, where it is tried when mDNS
/// finds nothing; its port, when none is typed, is the one the Device was
/// last found at. The pinned certificate still decides whether the Device
/// answering there is the one paired.
class HandAddress {
  /// A name or an IPv4 address in lower case, or an IPv6 address without
  /// its brackets.
  final String host;
  final int? port;

  const HandAddress(this.host, [this.port]);

  /// The address [typed] names, as the desktop core's
  /// `HandAddress::parse`: a host, `host:port`, `[v6]` or `[v6]:port`, or
  /// a bare IPv6 address; a pasted `http://` or `https://` and a path
  /// after the host are dropped. Null for anything else, a port that is
  /// not 1 to 65535 in digits included.
  static HandAddress? parse(String typed) {
    var text = typed.trim();
    final lower = _asciiLower(text);
    if (lower.startsWith('https://')) {
      text = text.substring('https://'.length);
    } else if (lower.startsWith('http://')) {
      text = text.substring('http://'.length);
    }
    text = _asciiLower(text.split(RegExp('[/?#]')).first);
    bool v6(String host) => host.contains(':') && _v6.hasMatch(host);
    if (text.startsWith('[')) {
      final end = text.indexOf(']');
      if (end < 0) return null;
      final host = text.substring(1, end);
      if (!v6(host)) return null;
      final rest = text.substring(end + 1);
      if (rest.isEmpty) return HandAddress(host);
      if (!rest.startsWith(':')) return null;
      final port = _portOf(rest.substring(1));
      return port == null ? null : HandAddress(host, port);
    }
    if (':'.allMatches(text).length > 1) {
      return v6(text) ? HandAddress(text) : null;
    }
    final colon = text.indexOf(':');
    final host = colon < 0 ? text : text.substring(0, colon);
    if (!_name.hasMatch(host)) return null;
    if (colon < 0) return HandAddress(host);
    final port = _portOf(text.substring(colon + 1));
    return port == null ? null : HandAddress(host, port);
  }

  /// The address as it is kept and shown: an IPv6 address in brackets.
  String get text {
    final shown = host.contains(':') ? '[$host]' : host;
    return port == null ? shown : '$shown:$port';
  }

  @override
  bool operator ==(Object other) =>
      other is HandAddress && other.host == host && other.port == port;

  @override
  int get hashCode => Object.hash(host, port);

  @override
  String toString() => 'HandAddress($text)';

  static final _v6 = RegExp(r'^[0-9a-f:.]+$');
  static final _name = RegExp(r'^[a-z0-9.-]+$');
  static final _digits = RegExp(r'^[0-9]{1,5}$');

  /// A port in digits only, 1 to 65535: no sign, no `0x`.
  static int? _portOf(String text) {
    if (!_digits.hasMatch(text)) return null;
    final port = int.parse(text);
    return port >= 1 && port <= 65535 ? port : null;
  }

  /// [text] with A to Z in lower case and nothing else changed, as the
  /// desktop core's `to_ascii_lowercase`.
  static String _asciiLower(String text) => String.fromCharCodes([
    for (final unit in text.codeUnits)
      unit >= 0x41 && unit <= 0x5a ? unit + 0x20 : unit,
  ]);
}

/// The paired Devices of this Device, never synced (ADR 0008).
extension Pairing on Store {
  static const _prefix = 'paired:';
  static const _autoPrefix = 'autosync:';
  static const _heldPrefix = 'held:';
  static const _addressPrefix = 'address:';

  /// Every paired Device, by name.
  List<PairedDevice> pairedDevices() =>
      [for (final (_, raw) in localSettingsWithPrefix(_prefix)) ?_read(raw)]
        ..sort((a, b) => a.name.compareTo(b.name));

  /// The paired Device [id], or null.
  PairedDevice? pairedDevice(String id) {
    final raw = localSetting('$_prefix$id');
    return raw == null ? null : _read(raw);
  }

  /// Remembers [device], or what changed about it. A Device that only
  /// joined keeps the fingerprint it had when it hosted, and a Device
  /// whose address is not known now the address it had with that
  /// fingerprint; one paired by invite stays [PairedDevice.shared].
  void rememberDevice(PairedDevice device) {
    final held = pairedDevice(device.id);
    final fingerprint = device.fingerprint.isNotEmpty
        ? device.fingerprint
        : held?.fingerprint ?? '';
    final same = held != null && held.fingerprint == fingerprint;
    final kept = PairedDevice(
      id: device.id,
      name: device.name,
      fingerprint: fingerprint,
      host: device.host ?? (same ? held.host : null),
      port: device.port ?? (same ? held.port : null),
      lastSync: device.lastSync,
      shared: device.shared || (held?.shared ?? false),
    );
    setLocalSetting('$_prefix${device.id}', jsonEncode(kept.toJson()));
  }

  /// Forgets the paired Device [id] and its address by hand. Its signing
  /// key stays: the entries it wrote are still checked against it.
  void forgetDevice(String id) {
    removeLocalSetting('$_prefix$id');
    removeLocalSetting('$_autoPrefix$id');
    removeLocalSetting('$_heldPrefix$id');
    removeLocalSetting('$_addressPrefix$id');
  }

  /// The address typed by hand for the paired Device [id], if one is.
  HandAddress? handAddress(String id) {
    final kept = localSetting('$_addressPrefix$id');
    return kept == null ? null : HandAddress.parse(kept);
  }

  /// Keeps [typed] as the address by hand of the paired Device [id], a
  /// local setting of this Device, never synced; an empty one removes it.
  /// Whether it was taken: an address [HandAddress.parse] refuses changes
  /// nothing.
  bool setHandAddress(String id, String typed) {
    if (typed.trim().isEmpty) {
      removeLocalSetting('$_addressPrefix$id');
      return true;
    }
    final address = HandAddress.parse(typed);
    if (address == null) return false;
    setLocalSetting('$_addressPrefix$id', address.text);
    return true;
  }

  /// Where to look for the paired [device], in turn: where mDNS [found]
  /// it, else its address by hand, with the port it was last found at
  /// when none is typed, then the address it was last found at. Each
  /// once.
  List<PairedAddress> pairedAddresses(
    PairedDevice device, {
    PairedAddress? found,
  }) {
    final hand = handAddress(device.id);
    final addresses = <PairedAddress>[];
    void add(String? host, int? port) {
      if (host == null || port == null) return;
      final address = (host: host, port: port);
      if (!addresses.contains(address)) addresses.add(address);
    }

    add(found?.host, found?.port);
    add(hand?.host, hand?.port ?? device.port);
    add(device.host, device.port);
    return addresses;
  }

  /// Whether this Device may reach the paired [device] at [host]: on the
  /// local network, or at the address typed for it by hand, which may be
  /// a VPN's anywhere. The pinned certificate decides the rest.
  bool mayReachPaired(PairedDevice device, String host) =>
      isPrivateHost(host) || isHandAddress(device, host);

  /// Whether [host] is the address typed by hand for [device].
  bool isHandAddress(PairedDevice device, String host) {
    final hand = handAddress(device.id);
    return hand != null && hand.host == HandAddress._asciiLower(host);
  }

  /// The version vector the Device [id] last said it holds, in a LAN
  /// session or a bundle it wrote; empty when it never said. What a
  /// bundle for it carries is what lies beyond.
  Map<String, int> heldBy(String id) {
    final raw = localSetting('$_heldPrefix$id');
    if (raw == null) return const {};
    try {
      final json = jsonDecode(raw);
      return {
        if (json is Map)
          for (final MapEntry(:key, :value) in json.entries)
            if (key is String && value is int) key: value,
      };
    } on FormatException {
      return const {};
    }
  }

  /// The Device [id] said it holds [vector]: kept, each counter only
  /// ever growing, so an older word never makes it seem to hold less.
  void noteHeldBy(String id, Map<String, int> vector) {
    if (id.isEmpty || id == deviceId) return;
    final held = Map.of(heldBy(id));
    for (final MapEntry(:key, :value) in vector.entries) {
      if (value > (held[key] ?? 0)) held[key] = value;
    }
    final sorted = {for (final d in held.keys.toList()..sort()) d: held[d]};
    setLocalSetting('$_heldPrefix$id', jsonEncode(sorted));
  }

  /// Whether this Device syncs with the paired Device [id] on its own;
  /// on unless switched off. Kept apart from the [PairedDevice], which
  /// every sync writes anew.
  bool autoSyncOn(String id) => localSetting('$_autoPrefix$id') != 'off';

  void setAutoSync(String id, bool on) => on
      ? removeLocalSetting('$_autoPrefix$id')
      : setLocalSetting('$_autoPrefix$id', 'off');

  static PairedDevice? _read(String raw) {
    try {
      return PairedDevice.fromJson(jsonDecode(raw));
    } on FormatException {
      return null;
    }
  }
}
