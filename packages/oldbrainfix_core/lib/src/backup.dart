import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:cryptography/cryptography.dart' show SecretKey;
import 'package:cryptography/dart.dart' show DartArgon2id;

import 'area_keys.dart';
import 'device_settings.dart';
import 'entry.dart';
import 'profiles.dart';
import 'recovery_sheet.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// Backups (ADR 0010, 0016): an encrypted file holding everything on
/// this Device but its settings, the playback speed aside, written by
/// hand or weekly into a folder the user picks, and restored on a fresh
/// install with the passphrase or the recovery key of the Recovery
/// Sheet.
///
/// Setting Backups up makes a random backup key, which the Recovery
/// Sheet shows, and seals it once with a key the passphrase derives
/// (Argon2id). The Device keeps both, never the passphrase, so a
/// scheduled Backup needs nobody to type anything. The key belongs to
/// the own Profile: the user's other Devices receive it with the sealed
/// copy and write their own Backups with it. Every file is sealed with a
/// content key of its own, sealed with the backup key.

/// The file extension of a Backup.
const backupExtension = 'obfbackup';

/// What the header line says a Backup is.
const backupType = 'oldbrainfix-backup';

/// The format a Backup says it is in, in its header and its first
/// record: `format`, 1 when missing, and 1 is never written (ADR 0019).
const backupFormat = 1;

/// Whether [json] is in the format this version reads.
bool _inFormat(Object? json) =>
    json is Map &&
    (json.containsKey('format') ? json['format'] : 1) == backupFormat;

/// How many payload bytes each sealed chunk holds.
const backupChunkBytes = 65536;

/// How many Backups of this Device a folder keeps; older ones go.
const backupsKept = 4;

/// How often a scheduled Backup is written.
const backupInterval = Duration(days: 7);

/// The most Android's Auto Backup takes of an app; a larger copy is not
/// sent at all.
const autoBackupQuotaBytes = 25 << 20;

/// The shortest passphrase taken: the Recovery Sheet is there for a
/// forgotten one, a guessable one has no remedy.
const minPassphraseLength = 8;

const backupKeyBytes = 32;
const _saltBytes = 16;

/// The longest header line read: far more than any header needs.
const _maxHeaderBytes = 4096;

/// The longest record line read; a blob's bytes follow their line.
const _maxRecordBytes = 1 << 20;

/// The largest blob taken from a Backup, as a bundle takes one.
const _maxBlobBytes = 256 << 20;

/// The Argon2id cost of the passphrase: memory in KiB, passes and
/// lanes. [standard] takes a 2020 phone a few seconds; tests and the
/// fixtures use less, and every file says what it was made with.
class BackupKdf {
  final int memory;
  final int iterations;
  final int parallelism;

  const BackupKdf({
    required this.memory,
    required this.iterations,
    this.parallelism = 1,
  });

  /// 64 MiB, three passes, one lane.
  static const standard = BackupKdf(memory: 65536, iterations: 3);

  /// Whether a file's cost is one to work through: a crafted file must
  /// not ask for gigabytes or hours.
  bool get sane =>
      parallelism >= 1 &&
      parallelism <= 4 &&
      memory >= 8 * parallelism &&
      memory <= 262144 &&
      iterations >= 1 &&
      iterations <= 16;
}

/// The key [passphrase] derives with [kdf] and [salt]: Argon2id, 32
/// bytes, of the passphrase without the blanks around it. Needs nothing
/// but its arguments, so the app runs it in an isolate of its own.
Future<Uint8List> backupPassKey(
  String passphrase,
  BackupKdf kdf,
  List<int> salt,
) async {
  final algorithm = DartArgon2id(
    parallelism: kdf.parallelism,
    memory: kdf.memory,
    iterations: kdf.iterations,
    hashLength: backupKeyBytes,
  );
  final key = await algorithm.deriveKey(
    secretKey: SecretKey(utf8.encode(passphrase.trim())),
    nonce: salt,
  );
  return Uint8List.fromList(await key.extractBytes());
}

/// How the passphrase's key is derived; the app hands in one that runs
/// [backupPassKey] in an isolate.
typedef PassKeyDeriver = Future<Uint8List> Function(
  String passphrase,
  BackupKdf kdf,
  List<int> salt,
);

/// The backup key sealed with the passphrase's key, with the cost and
/// the salt that derive it: kept on the Device and written into every
/// Backup.
class BackupPass {
  final BackupKdf kdf;
  final Uint8List salt;
  final Uint8List sealed;

  const BackupPass(this.kdf, this.salt, this.sealed);

  /// The `kdf` of a header: keys sorted.
  Map<String, Object> kdfJson() => {
    'm': kdf.memory,
    'p': kdf.parallelism,
    'salt': base64.encode(salt),
    't': kdf.iterations,
  };

  /// As the Device keeps it: keys sorted.
  Map<String, Object> toJson() => {
    'm': kdf.memory,
    'p': kdf.parallelism,
    'salt': base64.encode(salt),
    'sealed': base64.encode(sealed),
    't': kdf.iterations,
  };

  /// The stanza in [kdf] with [sealed] (base64), or null when it is none
  /// or asks for a cost that is not [BackupKdf.sane].
  static BackupPass? read(Object? kdf, Object? sealed) {
    if (kdf
        case {
          'm': final int m,
          'p': final int p,
          'salt': final String salt,
          't': final int t,
        }
        when sealed is String) {
      final cost = BackupKdf(memory: m, iterations: t, parallelism: p);
      if (!cost.sane) return null;
      try {
        return BackupPass(cost, base64.decode(salt), base64.decode(sealed));
      } on FormatException {
        return null;
      }
    }
    return null;
  }

  static BackupPass? fromJson(Object? json) =>
      json is Map ? read(json, json['sealed']) : null;
}

/// Why a Backup could not be restored.
enum BackupProblem {
  /// The file is no Backup, or one of a later format.
  notABackup,

  /// Neither the passphrase nor the recovery key opens it.
  wrongSecret,

  /// It opened, but was changed or cut off.
  damaged,

  /// This Device holds a log already: Backups go only into a fresh one.
  notEmpty,
}

class BackupException implements Exception {
  final BackupProblem problem;

  const BackupException(this.problem);

  @override
  String toString() => 'BackupException(${problem.name})';
}

/// The plain first line of a Backup: when it was made, how the
/// passphrase opens it and its content key, sealed.
class BackupHeader {
  final DateTime created;
  final BackupPass pass;
  final Uint8List sealedKey;
  final int chunk;

  /// The line's bytes, without its line break: what every chunk is
  /// bound to.
  final Uint8List line;

  BackupHeader._(
    this.created,
    this.pass,
    this.sealedKey,
    this.chunk,
    this.line,
  );

  /// The SHA-256 of [line], hex.
  String get hash => sha256.convert(line).toString();

  /// The header line of a Backup made at [created], keys sorted, no
  /// spaces.
  static Uint8List encode({
    required DateTime created,
    required BackupPass pass,
    required List<int> sealedKey,
    int chunk = backupChunkBytes,
  }) => utf8.encode(
    jsonEncode({
      'chunk': chunk,
      'created': created.toUtc().toIso8601String(),
      'kdf': pass.kdfJson(),
      'key': base64.encode(sealedKey),
      'pass': base64.encode(pass.sealed),
      'type': backupType,
    }),
  );

  /// The header in [line], or null when it is none this format reads.
  static BackupHeader? parse(List<int> line) {
    try {
      final json = jsonDecode(utf8.decode(line));
      if (json
          case {
            'chunk': final int chunk,
            'created': final String created,
            'kdf': final Object kdf,
            'key': final String key,
            'pass': final String sealed,
            'type': backupType,
          }
          when chunk >= 1024 && chunk <= (1 << 20) && _inFormat(json)) {
        final pass = BackupPass.read(kdf, sealed);
        final at = DateTime.tryParse(created);
        if (pass == null || at == null) return null;
        return BackupHeader._(
          at.toUtc(),
          pass,
          base64.decode(key),
          chunk,
          Uint8List.fromList(line),
        );
      }
    } on FormatException {
      return null;
    }
    return null;
  }
}

/// The header of the Backup at [path], or null when it is none.
BackupHeader? readBackupHeader(String path) {
  final file = File(path);
  if (!file.existsSync()) return null;
  final raf = file.openSync();
  try {
    final head = raf.readSync(_maxHeaderBytes);
    final end = head.indexOf(0x0a);
    return end < 0 ? null : BackupHeader.parse(head.sublist(0, end));
  } finally {
    raf.closeSync();
  }
}

/// The backup key that opens [header]: through [passphrase], or the
/// [recoveryCode] as typed or scanned; null when neither does.
Future<Uint8List?> unlockBackup(
  BackupHeader header, {
  String? passphrase,
  String? recoveryCode,
  PassKeyDeriver derive = backupPassKey,
}) async {
  Uint8List? key;
  if (recoveryCode != null) {
    key = decodeRecoveryCode(recoveryCode);
  } else if (passphrase != null) {
    final pass = header.pass;
    final derived = await derive(passphrase, pass.kdf, pass.salt);
    key = unseal(derived, pass.sealed, aad: SealedAs.backupPass);
  }
  if (key == null) return null;
  return unseal(key, header.sealedKey, aad: SealedAs.backupKey) == null
      ? null
      : key;
}

// -------------------------------------------------------------- settings

extension Backups on Store {
  static const _key = 'backup:key';
  static const _pass = 'backup:pass';
  static const _folder = 'backup:folder';
  static const _weekly = 'backup:weekly';
  static const _last = 'backup:last';
  static const _failed = 'backup:failed';
  static const _auto = 'backup:auto';
  static const _version = 'backup:version';

  /// The backup key, once Backups are set up.
  Uint8List? get backupKey {
    final raw = localSetting(_key);
    if (raw == null) return null;
    try {
      final key = base64.decode(raw);
      return key.length == backupKeyBytes ? key : null;
    } on FormatException {
      return null;
    }
  }

  /// The backup key sealed with the passphrase's key.
  BackupPass? get backupPass {
    final raw = localSetting(_pass);
    if (raw == null) return null;
    try {
      return BackupPass.fromJson(jsonDecode(raw));
    } on FormatException {
      return null;
    }
  }

  bool get backupsSetUp => backupKey != null && backupPass != null;

  /// The recovery key the Recovery Sheet shows, once set up.
  String? get recoveryCode => switch (backupKey) {
    final key? => encodeRecoveryCode(key),
    null => null,
  };

  /// Keeps the backup [key] and [pass], what a scheduled Backup needs,
  /// with the key's [version].
  void keepBackupKeys(Uint8List key, BackupPass pass, {required int version}) {
    setLocalSetting(_key, base64.encode(key));
    setLocalSetting(_pass, jsonEncode(pass.toJson()));
    setLocalSetting(_version, '$version');
  }

  /// The folder Backups go into, as the platform keeps it.
  String? get backupFolder => localSetting(_folder);

  set backupFolder(String? folder) => folder == null
      ? removeLocalSetting(_folder)
      : setLocalSetting(_folder, folder);

  /// Whether a Backup is written every week; on unless switched off.
  bool get backupsWeekly => localSetting(_weekly) != 'off';

  set backupsWeekly(bool on) =>
      on ? removeLocalSetting(_weekly) : setLocalSetting(_weekly, 'off');

  /// Whether the newest Backup also goes to the platform's own backup
  /// (Android's Auto Backup to the user's Google account, ADR 0010); off
  /// unless the user opted in.
  bool get autoBackupOn => localSetting(_auto) == 'on';

  set autoBackupOn(bool on) =>
      on ? setLocalSetting(_auto, 'on') : removeLocalSetting(_auto);

  /// Whether Backups are written on their own: set up, a folder chosen
  /// or Auto Backup on, and weekly on.
  bool get backupsScheduled =>
      backupsSetUp && (backupFolder != null || autoBackupOn) && backupsWeekly;

  /// When the last Backup was written here.
  DateTime? get lastBackup =>
      DateTime.tryParse(localSetting(_last) ?? '')?.toUtc();

  /// When writing a Backup last failed, unless one was written since.
  DateTime? get backupFailed =>
      DateTime.tryParse(localSetting(_failed) ?? '')?.toUtc();

  /// Whether a scheduled Backup is due at [now].
  bool backupDue(DateTime now) {
    if (!backupsScheduled) return false;
    final last = lastBackup;
    return last == null || now.toUtc().difference(last) >= backupInterval;
  }

  void _wrote(DateTime at) {
    setLocalSetting(_last, at.toUtc().toIso8601String());
    removeLocalSetting(_failed);
  }

  void _failedAt(DateTime at) =>
      setLocalSetting(_failed, at.toUtc().toIso8601String());

  /// The backup key's version: 1 when Backups were first set up, one
  /// more each time they are set up again, 0 once the Profile it was
  /// made for was given up for another (ADR 0016, 0017).
  int get backupKeyVersion => int.tryParse(localSetting(_version) ?? '') ?? 1;

  /// The backup key as it travels to the user's other Devices (ADR
  /// 0016): the key, its sealed copy, the own Profile it belongs to and
  /// its version, keys sorted; null before Backups are set up or before
  /// the own Profile exists.
  Map<String, Object?>? get backupKeyToSend {
    final key = backupKey;
    final pass = backupPass;
    final profile = ownProfile;
    if (key == null || pass == null || profile == null) return null;
    return {
      'key': base64.encode(key),
      'pass': pass.toJson(),
      'profile': profile.id,
      'version': backupKeyVersion,
    };
  }

  /// Takes the backup key [sent] by another of the user's own Devices,
  /// after its Profile was taken in: only one made for the own Profile,
  /// and only when it is newer than the one held (a higher version, or
  /// at the same version the larger key), so Devices that set Backups up
  /// apart end with the same key. Returns whether it was kept.
  bool takeBackupKey(Object? sent) {
    if (sent
        case {
          'key': final String key,
          'pass': final Object pass,
          'profile': final String profile,
          'version': final int version,
        }
        when version >= 0 && profile == ownProfile?.id) {
      final sealed = BackupPass.fromJson(pass);
      final Uint8List bytes;
      try {
        bytes = base64.decode(key);
      } on FormatException {
        return false;
      }
      if (sealed == null || bytes.length != backupKeyBytes) return false;
      final held = backupKey;
      if (held != null && backupPass != null) {
        final mine = backupKeyVersion;
        final ours = base64.encode(held);
        if (version < mine || (version == mine && key.compareTo(ours) <= 0)) {
          return false;
        }
      }
      keepBackupKeys(bytes, sealed, version: version);
      return true;
    }
    return false;
  }

  /// Gives the backup key held up to whatever key the own Profile
  /// brings: the Profile it was made for was given up for another.
  void yieldBackupKey() {
    if (backupKey != null) setLocalSetting(_version, '0');
  }
}

/// Sets Backups up with [passphrase]: a new backup key for the own
/// Profile, sealed with the key [passphrase] derives, one version above
/// the one held, so it reaches the user's other Devices. Returns the
/// recovery key for the Recovery Sheet. Backups set up before keep
/// opening with their own passphrase and recovery key.
Future<String> setUpBackups(
  Store store,
  String passphrase, {
  BackupKdf kdf = BackupKdf.standard,
  PassKeyDeriver derive = backupPassKey,
}) async {
  if (passphrase.trim().length < minPassphraseLength) {
    throw ArgumentError.value(passphrase, 'passphrase', 'too short');
  }
  final version = store.backupsSetUp ? store.backupKeyVersion + 1 : 1;
  final key = secretBytes(backupKeyBytes);
  final salt = secretBytes(_saltBytes);
  final derived = await derive(passphrase, kdf, salt);
  final pass = BackupPass(
    kdf,
    salt,
    seal(derived, key, aad: SealedAs.backupPass),
  );
  store.keepBackupKeys(key, pass, version: version);
  return encodeRecoveryCode(key);
}

// --------------------------------------------------------------- writing

/// What a Backup holds, by count.
class BackupCounts {
  final int entries;
  final int moments;
  final int keys;
  final int pinned;
  final int blobs;

  const BackupCounts({
    required this.entries,
    required this.moments,
    required this.keys,
    required this.pinned,
    required this.blobs,
  });
}

/// Seals the payload in chunks into a file as it comes, never holding
/// more than one chunk.
class _ChunkWriter {
  final RandomAccessFile _out;
  final Uint8List _key;
  final String _header;
  final Uint8List _buffer;
  var _filled = 0;
  var _index = 0;

  _ChunkWriter(this._out, this._key, this._header, int chunk)
    : _buffer = Uint8List(chunk);

  Future<void> add(List<int> bytes) async {
    var from = 0;
    while (from < bytes.length) {
      final take = (bytes.length - from).clamp(0, _buffer.length - _filled);
      _buffer.setRange(_filled, _filled + take, bytes, from);
      _filled += take;
      from += take;
      if (_filled == _buffer.length) {
        _seal(_buffer, last: false);
        _filled = 0;
        // The screen gets a turn between chunks.
        await Future<void>.delayed(Duration.zero);
      }
    }
  }

  /// The last chunk: what is left, shorter than a whole one, maybe
  /// empty.
  void close() => _seal(Uint8List.sublistView(_buffer, 0, _filled), last: true);

  void _seal(Uint8List plain, {required bool last}) {
    _out.writeFromSync(
      seal(_key, plain, aad: SealedAs.backupChunk(_header, _index, last: last)),
    );
    _index++;
  }
}

/// A record line: the JSON and its line break.
List<int> _line(String json) => utf8.encode('$json\n');

/// Writes a Backup of [store] to [path]: this Device's identity, the
/// own Profile with its binding and the backup key's version, the
/// playback speed chosen here, every entry, Moment, Area key, pinned key
/// and blob; none of its other settings. Backups must be set up. Keys
/// and nonces come from [secretBytes].
Future<BackupCounts> writeBackup(Store store, String path) async {
  final backupKey = store.backupKey;
  final pass = store.backupPass;
  if (backupKey == null || pass == null) {
    throw StateError('Backups are not set up');
  }
  final contentKey = secretBytes(backupKeyBytes);
  final line = BackupHeader.encode(
    created: store.clock(),
    pass: pass,
    sealedKey: seal(backupKey, contentKey, aad: SealedAs.backupKey),
  );
  final raf = File(path).openSync(mode: FileMode.write);
  try {
    raf
      ..writeFromSync(line)
      ..writeByteSync(0x0a);
    final out = _ChunkWriter(
      raf,
      contentKey,
      sha256.convert(line).toString(),
      backupChunkBytes,
    );
    await out.add(
      _line(
        jsonEncode({
          'about': {'device': store.deviceId, 'seed': store.signingKey.seedHex},
        }),
      ),
    );
    if (store.ownProfileJson case final profile?) {
      await out.add(
        _line(
          jsonEncode({
            'backupVersion': store.backupKeyVersion,
            'profile': profile,
          }),
        ),
      );
    }
    // As the setting is written, never as a number: the cores need not
    // write fractions alike.
    if (store.chosenPlaybackSpeed case final speed?) {
      await out.add(_line(jsonEncode({'playbackSpeed': '$speed'})));
    }
    final entries = store.allEntries();
    for (final e in entries) {
      await out.add(_line('{"entry":${e.wire}}'));
    }
    final seqs = [for (final e in entries) e.seq];
    final moments = store.momentRows().reversed.toList();
    for (final m in moments) {
      await out.add(
        _line(
          jsonEncode({
            'moment': {
              'at': m.at.toUtc().toIso8601String(),
              'cause': m.cause,
              'label': m.label,
              'seq': _countUpTo(seqs, m.seq),
            },
          }),
        ),
      );
    }
    final keys = store.areaKeys();
    for (final k in keys) {
      await out.add(_line(jsonEncode({'key': k.toJson()})));
    }
    final pinned = store.pinnedKeys();
    for (final p in pinned) {
      await out.add(_line(jsonEncode({'pinned': p.toJson()})));
    }
    final blobs = heldBlobs(store);
    for (final ref in blobs) {
      final (kind, hash) = splitBlobRef(ref);
      final bytes = store.blob(kind, hash);
      if (bytes == null) continue;
      await out.add(_line(jsonEncode({'blob': ref, 'size': bytes.length})));
      await out.add(bytes);
    }
    out.close();
    raf.flushSync();
    return BackupCounts(
      entries: entries.length,
      moments: moments.length,
      keys: keys.length,
      pinned: pinned.length,
      blobs: blobs.length,
    );
  } finally {
    raf.closeSync();
  }
}

/// Every blob the log ever named whose bytes are here, as
/// `photo/<hash>` and `audio/<hash>`, sorted.
List<String> heldBlobs(Store store) {
  final refs = <String>{};
  for (final (kind, prefix) in Sync.blobKinds) {
    for (final (_, field) in store.fieldsWithPrefix(prefix)) {
      final hash = field.substring(prefix.length);
      if (store.blob(kind, hash) != null) refs.add('$kind/$hash');
    }
  }
  return refs.toList()..sort();
}

/// How many of the ascending [seqs] are at most [mark]: a Moment's place
/// in the log, whatever the local handles.
int _countUpTo(List<int> seqs, int mark) {
  var low = 0, high = seqs.length;
  while (low < high) {
    final mid = (low + high) >> 1;
    if (seqs[mid] <= mark) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return low;
}

// ------------------------------------------------------------- restoring

/// What a restore took in.
class RestoreResult {
  /// The id of the Device the Backup was made on.
  final String from;
  final int entries;
  final int moments;
  final int keys;
  final int pinned;
  final int blobs;

  /// The Shared Areas whose role the old Device handed to this one.
  final int roles;

  const RestoreResult({
    required this.from,
    required this.entries,
    required this.moments,
    required this.keys,
    required this.pinned,
    required this.blobs,
    required this.roles,
  });
}

/// Opens the chunks of a Backup one after the other and hands out the
/// payload byte by byte, lines and blobs alike.
class _ChunkReader {
  final RandomAccessFile _in;
  final Uint8List _key;
  final String _header;
  final int _chunk;
  var _index = 0;
  var _ended = false;
  Uint8List _plain = Uint8List(0);
  var _at = 0;

  _ChunkReader(this._in, this._key, this._header, this._chunk);

  static const _damaged = BackupException(BackupProblem.damaged);

  /// Whether the next chunk is opened; false after the last one.
  Future<bool> _next() async {
    if (_ended) return false;
    final whole = _chunk + sealNonceBytes + sealTagBytes;
    final sealed = _in.readSync(whole);
    if (sealed.length < sealNonceBytes + sealTagBytes) throw _damaged;
    final last = sealed.length < whole;
    final plain = unseal(
      _key,
      sealed,
      aad: SealedAs.backupChunk(_header, _index, last: last),
    );
    if (plain == null) throw _damaged;
    _index++;
    _ended = last;
    _plain = plain;
    _at = 0;
    await Future<void>.delayed(Duration.zero);
    return true;
  }

  /// The next line, without its break; null at the end of the payload.
  Future<String?> line() async {
    final bytes = BytesBuilder(copy: false);
    while (true) {
      if (_at == _plain.length) {
        if (!await _next()) {
          if (bytes.isEmpty) return null;
          throw _damaged;
        }
        continue;
      }
      final end = _plain.indexOf(0x0a, _at);
      final stop = end < 0 ? _plain.length : end;
      bytes.add(Uint8List.sublistView(_plain, _at, stop));
      if (bytes.length > _maxRecordBytes) throw _damaged;
      _at = end < 0 ? stop : stop + 1;
      if (end >= 0) {
        try {
          return utf8.decode(bytes.takeBytes());
        } on FormatException {
          throw _damaged;
        }
      }
    }
  }

  /// The next [length] bytes.
  Future<Uint8List> bytes(int length) async {
    final out = Uint8List(length);
    var filled = 0;
    while (filled < length) {
      if (_at == _plain.length && !await _next()) throw _damaged;
      final take = (length - filled).clamp(0, _plain.length - _at);
      out.setRange(filled, filled + take, _plain, _at);
      filled += take;
      _at += take;
    }
    return out;
  }

  /// Whether the whole payload was read, up to its last chunk.
  Future<bool> atEnd() async =>
      _at == _plain.length && !await _next() && _ended;
}

/// Restores the Backup at [path], opened with [backupKey], into the
/// empty log of [store]. This Device keeps its own identity: it takes in
/// the entries, Moments, blobs, Area keys and pinned keys, pins the old
/// Device's key as verified, takes the own Profile back with its own key
/// bound to it by the old Device's ([OwnProfile.restoreOwnProfile]),
/// and keeps the backup key, so the Recovery Sheet stays good. In every
/// Shared Area where the old Device held a role, it signs the same role
/// for itself as the old Device. Of the old Device's settings only the
/// playback speed comes along, if it is one offered. The log is written in one
/// transaction, once the whole file proved sound.
Future<RestoreResult> restoreBackup(
  Store store,
  String path,
  Uint8List backupKey,
) async {
  if (store.currentSeq() != 0) {
    throw const BackupException(BackupProblem.notEmpty);
  }
  final header = readBackupHeader(path);
  if (header == null) throw const BackupException(BackupProblem.notABackup);
  final contentKey = unseal(
    backupKey,
    header.sealedKey,
    aad: SealedAs.backupKey,
  );
  if (contentKey == null) {
    throw const BackupException(BackupProblem.wrongSecret);
  }
  const damaged = BackupException(BackupProblem.damaged);
  final raf = File(path).openSync();
  SigningKey? old;
  final entries = <Entry>[];
  final moments = <({int seq, String cause, String? label, DateTime at})>[];
  final keys = <AreaKey>[];
  final pinned = <PinnedKey>[];
  Object? profile;
  int? version;
  double? speed;
  var blobs = 0;
  try {
    raf.setPositionSync(header.line.length + 1);
    final payload = _ChunkReader(raf, contentKey, header.hash, header.chunk);
    while (true) {
      final text = await payload.line();
      if (text == null) break;
      final Object? json;
      try {
        json = jsonDecode(text);
      } on FormatException {
        throw damaged;
      }
      if (json is! Map<String, dynamic>) throw damaged;
      if (old == null) {
        old = _about(json['about']);
        if (old == null) throw damaged;
        continue;
      }
      switch (json) {
        case {'entry': final Map<String, dynamic> e}:
          try {
            entries.add(Entry.fromJson(e, text: _entryText(text)));
          } on Object {
            throw damaged;
          }
        case {
          'moment': {
            'at': final String at,
            'cause': final String cause,
            'label': final String? label,
            'seq': final int seq,
          },
        }:
          final when = DateTime.tryParse(at);
          if (when == null) throw damaged;
          moments.add((seq: seq, cause: cause, label: label, at: when));
        case {'key': final Object k}:
          keys.add(AreaKey.fromJson(k) ?? (throw damaged));
        case {'pinned': final Object p}:
          pinned.add(PinnedKey.fromJson(p) ?? (throw damaged));
        case {'backupVersion': final int v, 'profile': final Object p}:
          if (v < 0) throw damaged;
          profile = p;
          version = v;
        case {'playbackSpeed': final String s}:
          // One not offered here is passed over, as normal speed.
          final kept = double.tryParse(s);
          if (kept != null && playbackSpeeds.contains(kept)) speed = kept;
        case {'blob': final String ref, 'size': final int size}:
          if (size < 0 || size > _maxBlobBytes) throw damaged;
          final bytes = await payload.bytes(size);
          final (kind, hash) = splitBlobRef(ref);
          if (!store.putBlob(kind, hash, bytes)) throw damaged;
          blobs++;
        case {'size': final int size}:
          // A record of a later kind: its bytes are passed over.
          if (size < 0 || size > _maxBlobBytes) throw damaged;
          await payload.bytes(size);
        default:
          // A record of a later kind.
          continue;
      }
    }
    if (old == null || !await payload.atEnd()) throw damaged;
  } finally {
    raf.closeSync();
  }
  final from = old;
  final roles = store.batch(() {
    if (store.currentSeq() != 0) {
      throw const BackupException(BackupProblem.notEmpty);
    }
    // Each entry's local handle as it lands: a Moment's place in the
    // order becomes a mark here.
    final marks = [store.currentSeq()];
    for (final e in entries) {
      if (store.insert(e)) marks.add(store.currentSeq());
    }
    for (final m in moments) {
      store.addMoment(
        seq: marks[m.seq.clamp(0, marks.length - 1)],
        cause: m.cause,
        label: m.label,
        at: m.at,
      );
    }
    store.learnAreaKeys(keys);
    final oldRecord = KeyRecord.make(from);
    store.learnKeys([oldRecord], verifiedDevice: oldRecord.device);
    for (final p in pinned) {
      store.learnKeys([
        p.record,
      ], verifiedDevice: p.trust == KeyTrust.verified ? p.record.device : null);
    }
    store
      ..restoreOwnProfile(profile, old: from)
      ..keepBackupKeys(backupKey, header.pass, version: version ?? 1);
    if (speed case final speed?) store.playbackSpeed = speed;
    return _handRoles(store, from);
  });
  store.profileChanged();
  return RestoreResult(
    from: deviceIdFromKey(from.publicKey),
    entries: entries.length,
    moments: moments.length,
    keys: keys.length,
    pinned: pinned.length,
    blobs: blobs,
    roles: roles,
  );
}

/// The entry in a record `{"entry":…}` as it was written, so it is kept
/// byte for byte (ADR 0019); null for a record written otherwise.
String? _entryText(String record) {
  const head = '{"entry":';
  return record.startsWith(head) && record.endsWith('}')
      ? record.substring(head.length, record.length - 1)
      : null;
}

/// The old Device's signing key in the `about` record, or null when it
/// is none or not the Device's own.
SigningKey? _about(Object? about) {
  if (about case {'device': final String device, 'seed': final String seed}
      when seed.length == 64 && _inFormat(about)) {
    try {
      final key = SigningKey.fromSeed(fromHex(seed));
      return deviceIdFromKey(key.publicKey) == device ? key : null;
    } on FormatException {
      return null;
    }
  }
  return null;
}

/// Signs, as the old Device [old], its role in every Shared Area for
/// this Device, as a Device of its group would be given it; returns how
/// many.
int _handRoles(Store store, SigningKey old) {
  final self = store.deviceId;
  final device = deviceIdFromKey(old.publicKey);
  final areas = store.judged.areas.entries.toList()
    ..sort((a, b) => a.key.compareTo(b.key));
  var dseq = store.versionVector()[device] ?? 0;
  var handed = 0;
  for (final MapEntry(key: area, value: grants) in areas) {
    final grant = grants.grantOf(device);
    // A grant not known here is not this version's to hand on.
    if (grant == null || grant.newer || grants.everGranted(self)) continue;
    dseq++;
    // After every grant there, or it would not be judged after them,
    // whatever this Device's clock says.
    var now = store.clock().toUtc();
    for (final e in store.history(area)) {
      if (e.field.startsWith(ShareKeys.rolePrefix) && !e.date.isBefore(now)) {
        now = e.date.toUtc().add(const Duration(milliseconds: 1));
      }
    }
    final iso = now.toIso8601String();
    final field = ShareKeys.role(self);
    final value = grant.encode();
    store.insert(
      Entry(
        seq: -1,
        device: device,
        dseq: dseq,
        entity: area,
        field: field,
        value: value,
        date: now,
        author: store.author,
        recorded: now,
        sig: old.sign(
          entryBytes(
            device: device,
            dseq: dseq,
            entity: area,
            field: field,
            value: value,
            dateIso: iso,
            author: store.author,
            recordedIso: iso,
          ),
        ),
      ),
    );
    handed++;
  }
  return handed;
}

// ---------------------------------------------------------------- folder

/// The folder Backups go into: a directory, or the document tree the
/// phone's picker granted.
abstract interface class BackupFolder {
  /// The file names in the folder, sorted.
  Future<List<String>> list();

  /// Copies the file at [path] into the folder as [name].
  Future<void> put(String name, String path);

  Future<void> delete(String name);
}

/// A [BackupFolder] that is a directory.
class LocalBackupFolder implements BackupFolder {
  final String path;

  const LocalBackupFolder(this.path);

  File _file(String name) {
    if (name.isEmpty ||
        name == '.' ||
        name == '..' ||
        name.contains('/') ||
        name.contains(r'\')) {
      throw FileSystemException('Not a folder file', name);
    }
    return File('$path/$name');
  }

  @override
  Future<List<String>> list() async {
    final dir = Directory(path);
    if (!dir.existsSync()) return const [];
    return [
      for (final f in dir.listSync().whereType<File>()) f.uri.pathSegments.last,
    ]..sort();
  }

  @override
  Future<void> put(String name, String from) async {
    // Beside, then renamed: a cloud client must never upload half a
    // file as the whole.
    final target = _file(name);
    final tmp = File('${target.path}.tmp');
    File(from).copySync(tmp.path);
    tmp.renameSync(target.path);
  }

  @override
  Future<void> delete(String name) async {
    final f = _file(name);
    if (f.existsSync()) f.deleteSync();
  }
}

String _two(int n) => n.toString().padLeft(2, '0');

/// The name of this Device's Backup made at [at]: the UTC time and the
/// first 8 digits of [device], so Backups sort by time and those of
/// other Devices are told apart.
String backupFileName(DateTime at, String device) {
  final u = at.toUtc();
  return 'oldbrainfix-${u.year.toString().padLeft(4, '0')}${_two(u.month)}'
      '${_two(u.day)}-${_two(u.hour)}${_two(u.minute)}${_two(u.second)}-'
      '${device.substring(0, 8)}.$backupExtension';
}

/// Whether [name] is a Backup of [device] as [backupFileName] names it.
bool isBackupOf(String name, String device) => RegExp(
  '^oldbrainfix-\\d{8}-\\d{6}-${RegExp.escape(device.substring(0, 8))}'
  '\\.$backupExtension\$',
).hasMatch(name);

/// Writes a Backup now, through a file in [workDir], into [folder] and,
/// when given, into [autoBackup], the directory the platform's own
/// backup takes it from; only the newest [backupsKept] of this Device
/// stay in [folder], and nothing but it in [autoBackup] (a Backup of an
/// older Device restored from there included). Returns its name; a
/// failure is kept for the screen and thrown on.
Future<String> backUpNow(
  Store store,
  BackupFolder? folder,
  String workDir, {
  BackupFolder? autoBackup,
}) async {
  if (folder == null && autoBackup == null) {
    throw ArgumentError('A Backup needs somewhere to go');
  }
  final now = store.clock().toUtc();
  final name = backupFileName(now, store.deviceId);
  final tmp = File('$workDir/$name');
  try {
    await writeBackup(store, tmp.path);
    if (folder != null) {
      await folder.put(name, tmp.path);
      final mine = [
        for (final n in await folder.list())
          if (isBackupOf(n, store.deviceId)) n,
      ]..sort();
      for (final n in mine.take(max(0, mine.length - backupsKept))) {
        await folder.delete(n);
      }
    }
    if (autoBackup != null) {
      await autoBackup.put(name, tmp.path);
      for (final n in await autoBackup.list()) {
        if (n != name) await autoBackup.delete(n);
      }
    }
  } on Object {
    store._failedAt(now);
    rethrow;
  } finally {
    if (tmp.existsSync()) tmp.deleteSync();
  }
  store._wrote(now);
  return name;
}

/// Writes the scheduled Backup when it is due, into the folder [open]
/// makes of the one chosen and, while Auto Backup is on, into
/// [autoBackup]; returns its name, or null when none was due.
Future<String?> backUpIfDue(
  Store store,
  BackupFolder Function(String folder) open,
  String workDir, {
  BackupFolder? autoBackup,
}) async {
  if (!store.backupDue(store.clock())) return null;
  final folder = store.backupFolder;
  final auto = store.autoBackupOn ? autoBackup : null;
  if (folder == null && auto == null) return null;
  return backUpNow(
    store,
    folder == null ? null : open(folder),
    workDir,
    autoBackup: auto,
  );
}
