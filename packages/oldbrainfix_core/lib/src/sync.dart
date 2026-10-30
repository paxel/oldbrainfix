import 'dart:convert';
import 'dart:typed_data';

import 'area_keys.dart';
import 'entry.dart';
import 'notes.dart';
import 'profiles.dart'
    show BlockKeys, Blocks, OverrideKeys, Overrides, keptFromBlocked;
import 'shared_areas.dart';
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
  /// as verified. A key held without the X25519 key Sent Items are
  /// encrypted to (ADR 0015) takes it from a record that carries it, the
  /// same key signed: the trust stays as it was.
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
      } else if (held.record.boxKey == null && record.boxKey != null) {
        _pin(record, verified ? KeyTrust.verified : held.trust);
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
  /// too. An entry of a later structure ([Entry.applies]) is kept and
  /// passed on, but not applied nor returned. What a blocked person's
  /// Device wrote is refused but their check-offs, its dseq noted so it
  /// is not asked for again (ADR 0021), and what a Device found to be
  /// theirs only now is deleted afterwards ([Blocks.enforceBlocks]).
  List<Entry> applyEntries(
    List<Entry> entries, {
    Iterable<KeyRecord> keys = const [],
    String? verifiedDevice,
    ImportReport? report,
  }) {
    var applied = _applyEntries(
      entries,
      keys: keys,
      verifiedDevice: verifiedDevice,
      report: report,
    );
    // A block from another of the user's Devices, or a grant naming a
    // blocked person, takes effect at once (ADR 0021); a photo or
    // recording removed elsewhere goes here too (ADR 0020). Anything new
    // may decide either, a grant or a filing as well.
    if (applied.isNotEmpty) {
      if (enforceBlocks()) {
        applied = [
          for (final e in applied)
            if (holds(e.device, e.dseq)) e,
        ];
      }
      dropRemovedBlobs();
    }
    // An Override or a block from another of the user's Devices shows
    // at once wherever a person shows (#52, #90).
    if (applied.any(
      (e) =>
          e.entity.startsWith(OverrideKeys.prefix) ||
          e.entity.startsWith(BlockKeys.prefix),
    )) {
      profileChanged();
    }
    return applied;
  }

  List<Entry> _applyEntries(
    List<Entry> entries, {
    required Iterable<KeyRecord> keys,
    required String? verifiedDevice,
    required ImportReport? report,
  }) => batch(() {
    learnKeys(keys, verifiedDevice: verifiedDevice, report: report);
    final self = deviceId;
    final ownMax = versionVector()[self] ?? 0;
    final own = signingKey.publicKey;
    final known = <String, List<int>?>{};
    final refusing = blockedDevices();
    final refused = <Entry>[];
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
      // Checked first, so nobody moves the mark with a forged entry.
      if (refusing.contains(e.device) && !keptFromBlocked(e)) {
        refused.add(e);
        continue;
      }
      // One of a later structure is kept and passed on, not applied.
      if (insert(e) && e.applies) applied.add(e);
    }
    // Where each refused entity lay, once what is kept of this batch is
    // in: a Note by its filing, a Section by the Note it names.
    final links = {
      for (final e in refused)
        if (e.field == SectionKeys.note) e.entity: ?e.value,
    };
    for (final e in refused) {
      noteRefused(e.device, e.dseq, scopeOf(links[e.entity] ?? e.entity));
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
    extra: e.extra,
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

  /// Whether the log refers to the blob [ref] (`photo/<hash>` or
  /// `audio/<hash>`) now: only those are taken from another Device,
  /// anything else would be storage filled by whoever holds the PIN. A
  /// removed one stays out (ADR 0020).
  bool knowsBlob(String ref) {
    final (kind, hash) = splitBlobRef(ref);
    for (final (k, prefix) in blobKinds) {
      if (k != kind) continue;
      final field = '$prefix$hash';
      for (final (entity, f) in fieldsWithPrefix(field)) {
        if (f == field && current(entity, f) != null) return true;
      }
    }
    return false;
  }

  /// The blobs the log referred to and lets go of now, as `photo/<hash>`
  /// and `audio/<hash>`, sorted (ADR 0020): every field that named one
  /// was last set to null by a removal that may let the bytes go. That
  /// is one written by the user's own Devices ([writtenByOwn]), or one on
  /// a Note that lies in a Shared Area, where its role was judged. A
  /// removal another person wrote anywhere else hides the file but
  /// keeps its bytes.
  List<String> removedBlobs() {
    final own = <String, bool>{};
    final histories = <String, List<Entry>>{};
    final removed = <String, bool>{};
    for (final (kind, prefix) in blobKinds) {
      for (final (entity, field) in fieldsWithPrefix(prefix)) {
        final ref = '$kind/${field.substring(prefix.length)}';
        if (removed[ref] == false) continue;
        Entry? last;
        for (final e in histories.putIfAbsent(entity, () => history(entity))) {
          if (e.field != field) continue;
          if (last == null || Store.latestWins(e, last) > 0) last = e;
        }
        removed[ref] = last != null && _letsGo(last, own);
      }
    }
    return [
      for (final MapEntry(:key, :value) in removed.entries)
        if (value) key,
    ]..sort();
  }

  /// Whether [last], the entry a blob's field holds, lets its bytes go:
  /// a removal by one of the user's own Devices, whose answers [own]
  /// keeps, or one on a Note in a Shared Area.
  bool _letsGo(Entry last, Map<String, bool> own) {
    if (last.value != null) return false;
    if (own.putIfAbsent(last.device, () => writtenByOwn(last.device))) {
      return true;
    }
    final area = areaOf(last.entity);
    return area != null && isShared(area);
  }

  /// Lets go of the bytes of every blob in [removedBlobs] held here;
  /// returns those that were here.
  List<String> dropRemovedBlobs() => [
    for (final ref in removedBlobs())
      if (splitBlobRef(ref) case (final kind, final hash)
          when dropBlob(kind, hash))
        ref,
  ];

  /// The blob kinds and the field prefixes naming them.
  static const blobKinds = [
    ('photo', NoteKeys.photoPrefix),
    ('audio', NoteKeys.audioPrefix),
  ];
}

/// Which Device needs a newer version of the app before two Devices can
/// sync (ADR 0019): this one, or the other one.
enum UpdateNeeded { here, there }

/// Syncs that need an update, kept so each is announced once and not at
/// every automatic try (ADR 0019): by the Device synced with, or
/// [folderSource] for the shared folder. Local settings, never synced.
extension UpdateNotices on Store {
  static const _prefix = 'update:';
  static const _seen = ' seen';

  /// The source of the shared folder's notice.
  static const folderSource = 'folder';

  /// The source of the notice that a local setting a later version of the
  /// app wrote was kept as it is ([laterSetting]).
  static const settingsSource = 'settings';

  /// Keeps that syncing with [source] needs an update [where], to be
  /// announced ([takeUpdateNotices]) unless that was known already.
  void noteUpdateNeeded(String source, UpdateNeeded where) {
    final held = localSetting('$_prefix$source');
    if (held == where.name || held == '${where.name}$_seen') return;
    setLocalSetting('$_prefix$source', where.name);
  }

  /// A sync with [source] went through: what it needed is forgotten.
  void updateNoLongerNeeded(String source) {
    if (localSetting('$_prefix$source') != null) {
      removeLocalSetting('$_prefix$source');
    }
  }

  /// Keeps whether the shared folder holds data of a newer format than
  /// this version reads.
  void noteFolderNewer({required bool newer}) => newer
      ? noteUpdateNeeded(folderSource, UpdateNeeded.here)
      : updateNoLongerNeeded(folderSource);

  /// The notices not announced yet, by source; taking them marks them
  /// announced.
  List<(String source, UpdateNeeded where)> takeUpdateNotices() {
    final out = <(String, UpdateNeeded)>[];
    for (final (key, value) in localSettingsWithPrefix(_prefix)) {
      final where = UpdateNeeded.values.where((w) => w.name == value);
      if (where.isEmpty) continue;
      out.add((key.substring(_prefix.length), where.first));
      setLocalSetting(key, '$value$_seen');
    }
    return out;
  }
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

  /// A bundle file brought it.
  static const bundle = 'bundle';
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
