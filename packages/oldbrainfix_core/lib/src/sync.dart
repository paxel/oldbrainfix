import 'dart:convert';
import 'dart:typed_data';

import 'entry.dart';
import 'notes.dart';
import 'signing.dart';
import 'store.dart';

/// Delta exchange between Devices (ADR 0002, 0003; Catlog's ADR 0002):
/// every Device tells the other the newest entry it holds from each
/// Device, and only what is missing travels, the photo and voice
/// recording bytes after it. Keys travel along, so every entry is
/// checked against the key of the Device that signed it.
///
/// Local settings never travel: they are this Device's (ADR 0008).
extension Sync on Store {
  static const _keyPrefix = 'key:';

  /// This Device's own key record.
  KeyRecord get ownKeyRecord => KeyRecord.make(signingKey);

  /// The key pinned for [device], if it was ever met.
  PinnedKey? pinnedKey(String device) {
    final raw = localSetting('$_keyPrefix$device');
    if (raw == null) return null;
    try {
      return PinnedKey.fromJson(jsonDecode(raw));
    } on FormatException {
      return null;
    }
  }

  /// Every key this Device holds for other Devices, by device.
  List<PinnedKey> pinnedKeys() => [
    for (final (key, _) in localSettingsWithPrefix(_keyPrefix))
      ?pinnedKey(key.substring(_keyPrefix.length)),
  ];

  /// The keys that go out with the entries: this Device's own and every
  /// one it holds, by device, so keys reach Devices that never met their
  /// owner.
  List<KeyRecord> keyRecords() =>
      [ownKeyRecord, for (final k in pinnedKeys()) k.record]
        ..sort((a, b) => a.device.compareTo(b.device));

  void _pin(KeyRecord record, KeyTrust trust) => setLocalSetting(
    '$_keyPrefix${record.device}',
    jsonEncode(PinnedKey(record, trust).toJson()),
  );

  /// Takes in the keys a payload carried. A record that is not valid is
  /// ignored; the first key of a Device is pinned on trust, the one of
  /// [verifiedDevice], met over a session the pair code authenticated,
  /// as verified.
  void learnKeys(
    Iterable<KeyRecord> keys, {
    String? verifiedDevice,
    ImportReport? report,
  }) {
    final self = deviceId;
    for (final record in keys) {
      if (record.device == self || !record.valid) continue;
      final verified = record.device == verifiedDevice;
      final held = pinnedKey(record.device);
      if (held == null) {
        final trust = verified ? KeyTrust.verified : KeyTrust.tofu;
        _pin(record, trust);
        report?.newKeys.add(PinnedKey(record, trust));
      } else if (verified && held.trust == KeyTrust.tofu) {
        _pin(held.record, KeyTrust.verified);
      }
    }
  }

  /// Takes in entries another Device sent, in one transaction, and
  /// returns the ones that were new here. [keys] are learned first (see
  /// [learnKeys]). An entry already held is skipped; one whose signature
  /// does not verify under its Device's key, or whose Device has no key
  /// here, is refused and counted in [report], never recorded, so it
  /// cannot move the version vector past the real entry. An entry under
  /// this Device's own id beyond its own counter is forged and refused
  /// too.
  List<Entry> applyEntries(
    List<Entry> entries, {
    Iterable<KeyRecord> keys = const [],
    String? verifiedDevice,
    ImportReport? report,
  }) => batch(() {
    learnKeys(keys, verifiedDevice: verifiedDevice, report: report);
    final self = deviceId;
    final ownMax = versionVector()[self] ?? 0;
    final own = signingKey.publicKey;
    final known = <String, List<int>?>{};
    final applied = <Entry>[];
    for (final e in entries) {
      if (holds(e.device, e.dseq)) continue;
      final key = e.device == self
          ? (e.dseq > ownMax ? null : own)
          : known.putIfAbsent(
              e.device,
              () => pinnedKey(e.device)?.record.publicKey,
            );
      if (key == null || !verifySignature(key, _bytes(e), e.sig)) {
        report?.refuse(e.author, e.device);
        continue;
      }
      if (insert(e)) applied.add(e);
    }
    return applied;
  });

  static Uint8List _bytes(Entry e) => entryBytes(
    device: e.device,
    dseq: e.dseq,
    entity: e.entity,
    field: e.field,
    value: e.value,
    dateIso: e.date.toUtc().toIso8601String(),
    author: e.author,
    recordedIso: e.recorded.toUtc().toIso8601String(),
    ctx: e.ctx,
  );

  /// The blobs the log refers to whose bytes are not here, as
  /// `photo/<hash>` and `audio/<hash>`, sorted.
  List<String> missingBlobs() {
    final missing = <String>{};
    for (final (kind, prefix) in blobKinds) {
      for (final (entity, field) in fieldsWithPrefix(prefix)) {
        final hash = field.substring(prefix.length);
        if (current(entity, field) == null) continue;
        if (blob(kind, hash) == null) missing.add('$kind/$hash');
      }
    }
    return missing.toList()..sort();
  }

  /// Whether the log ever referred to the blob [ref] (`photo/<hash>` or
  /// `audio/<hash>`): only those are taken from another Device, anything
  /// else would be storage filled by whoever holds the PIN.
  bool knowsBlob(String ref) {
    final (kind, hash) = splitBlobRef(ref);
    for (final (k, prefix) in blobKinds) {
      if (k == kind) return mentions('$prefix$hash');
    }
    return false;
  }

  /// The blob kinds and the field prefixes naming them.
  static const blobKinds = [
    ('photo', NoteKeys.photoPrefix),
    ('audio', NoteKeys.audioPrefix),
  ];
}

/// The kind and hash of a blob reference `photo/<hash>`.
(String kind, String hash) splitBlobRef(String ref) {
  final slash = ref.indexOf('/');
  if (slash < 0) return ('', ref);
  return (ref.substring(0, slash), ref.substring(slash + 1));
}

/// Why a Moment was recorded.
abstract final class MomentCause {
  static const sync = 'sync';

  /// A folder sync brought it.
  static const folder = 'folder';
}

/// A state of the log a Device can be returned to (Catlog's ADR 0009):
/// nothing more than the log's high-water mark before a change, so the
/// entries after it are the change. Recorded before every sync that
/// brings anything; going back is not offered yet.
class Moment {
  final int id;

  /// The mark: the entries after it came later.
  final int seq;

  /// One of [MomentCause].
  final String cause;

  /// With whom or from what: the Device synced with.
  final String? label;
  final DateTime at;

  const Moment({
    required this.id,
    required this.seq,
    required this.cause,
    required this.label,
    required this.at,
  });
}

extension Moments on Store {
  /// Every recorded Moment, newest first.
  List<Moment> moments() => [
    for (final r in momentRows())
      Moment(id: r.id, seq: r.seq, cause: r.cause, label: r.label, at: r.at),
  ];

  /// Records the Moment before a change that already happened, given the
  /// mark [before] taken before it. A change that brought nothing records
  /// nothing: a sync finding nothing new must not fill the list.
  Moment? momentFor({
    required int before,
    required bool changed,
    required String cause,
    String? label,
  }) {
    if (!changed) return null;
    final id = addMoment(seq: before, cause: cause, label: label);
    return moments().firstWhere((m) => m.id == id);
  }
}
