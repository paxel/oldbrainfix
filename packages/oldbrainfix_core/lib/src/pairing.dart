import 'dart:convert';

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

/// The paired Devices of this Device, never synced (ADR 0008).
extension Pairing on Store {
  static const _prefix = 'paired:';
  static const _autoPrefix = 'autosync:';
  static const _heldPrefix = 'held:';

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
  /// joined keeps the fingerprint and address it had when it hosted; one
  /// paired by invite stays [PairedDevice.shared].
  void rememberDevice(PairedDevice device) {
    final held = pairedDevice(device.id);
    final hosted = held == null || device.fingerprint.isNotEmpty;
    final kept = PairedDevice(
      id: device.id,
      name: device.name,
      fingerprint: hosted ? device.fingerprint : held.fingerprint,
      host: hosted ? device.host : held.host,
      port: hosted ? device.port : held.port,
      lastSync: device.lastSync,
      shared: device.shared || (held?.shared ?? false),
    );
    setLocalSetting('$_prefix${device.id}', jsonEncode(kept.toJson()));
  }

  /// Forgets the paired Device [id]. Its key stays: the entries it wrote
  /// are still checked against it.
  void forgetDevice(String id) {
    removeLocalSetting('$_prefix$id');
    removeLocalSetting('$_autoPrefix$id');
    removeLocalSetting('$_heldPrefix$id');
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
