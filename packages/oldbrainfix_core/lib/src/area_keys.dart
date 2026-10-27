import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:cryptography/cryptography.dart'
    show Mac, SecretBox, SecretBoxAuthenticationError, SecretKeyData;
import 'package:cryptography/dart.dart' show DartXchacha20;

import 'notes.dart';
import 'places.dart';
import 'routines.dart';
import 'signing.dart';
import 'store.dart';

/// End-to-end encryption per Area (ADR 0003, 0012): every Area has a
/// random 256-bit key of its own, and everything outside an Area
/// (Unsorted Notes, Named Places) goes under the Device group's personal
/// key. The keys travel only over the authenticated LAN session of a
/// Pairing or a paired sync, and sealed into the shared folder under the
/// personal key; never in plain text. Each is a local setting of this
/// Device (ADR 0008).
///
/// A scope (an Area, or [personalScope]) may hold several keys: two
/// Devices that both made one before they met keep both, every reader
/// tries each, and every writer seals with the one of the lowest id, so
/// paired Devices end up writing with the same key. A key is never
/// replaced or dropped, or older files could no longer be read.
class AreaKey {
  /// The Area's entity id, or [personalScope].
  final String scope;

  /// The 32 key bytes.
  final Uint8List key;

  const AreaKey(this.scope, this.key);

  /// The key's id, the first 16 hex digits of its SHA-256: what folder
  /// file names carry, saying nothing about the Area.
  String get id => areaKeyId(key);

  Map<String, Object?> toJson() => {'key': base64.encode(key), 'scope': scope};

  /// The key in [json], or null when it is none: a scope that is no Area
  /// and not the personal one, or a key that is not 32 bytes.
  static AreaKey? fromJson(Object? json) {
    if (json is! Map) return null;
    final scope = json['scope'];
    final key = json['key'];
    if (scope is! String || key is! String) return null;
    if (scope != personalScope && !scope.startsWith('area:')) return null;
    try {
      final bytes = base64.decode(key);
      return bytes.length == areaKeyBytes ? AreaKey(scope, bytes) : null;
    } on FormatException {
      return null;
    }
  }
}

/// The scope of everything outside an Area.
const personalScope = 'personal';

const areaKeyBytes = 32;

/// The id of [key]: the first 16 hex digits of its SHA-256.
String areaKeyId(List<int> key) =>
    sha256.convert(key).toString().substring(0, 16);

final _random = Random.secure();

Uint8List _secureBytes(int length) =>
    Uint8List.fromList(List.generate(length, (_) => _random.nextInt(256)));

/// Makes the bytes of new keys and nonces: random, unless the fixture
/// generator hands in a counter so its files come out the same on every
/// run.
Uint8List Function(int length) secretBytes = _secureBytes;

// -------------------------------------------------------------- sealing

/// The nonce of a sealed message: XChaCha20's 24 bytes, random.
const sealNonceBytes = 24;

/// Poly1305's tag.
const sealTagBytes = 16;

const _aead = DartXchacha20.poly1305Aead();

/// [plain] sealed under [key] with XChaCha20-Poly1305, bound to [aad]:
/// the nonce, the cipher text and the tag, in this order. [nonce]
/// defaults to fresh random bytes.
Uint8List seal(
  List<int> key,
  List<int> plain, {
  required String aad,
  Uint8List? nonce,
}) {
  final box = _aead.encryptSync(
    plain,
    secretKey: SecretKeyData(key),
    nonce: nonce ?? secretBytes(sealNonceBytes),
    aad: utf8.encode(aad),
  );
  // Filled in place: a long recording must not pass through a list of
  // boxed ints.
  final cipher = box.cipherText;
  return Uint8List(sealNonceBytes + cipher.length + sealTagBytes)
    ..setAll(0, box.nonce)
    ..setAll(sealNonceBytes, cipher)
    ..setAll(sealNonceBytes + cipher.length, box.mac.bytes);
}

/// What [sealed] holds when [key] sealed it bound to [aad]; null for
/// another key, another [aad], or bytes that were changed.
Uint8List? unseal(List<int> key, List<int> sealed, {required String aad}) {
  if (key.length != areaKeyBytes ||
      sealed.length < sealNonceBytes + sealTagBytes) {
    return null;
  }
  final tag = sealed.length - sealTagBytes;
  try {
    return Uint8List.fromList(
      _aead.decryptSync(
        SecretBox(
          sealed.sublist(sealNonceBytes, tag),
          nonce: sealed.sublist(0, sealNonceBytes),
          mac: Mac(sealed.sublist(tag)),
        ),
        secretKey: SecretKeyData(key),
        aad: utf8.encode(aad),
      ),
    );
  } on SecretBoxAuthenticationError {
    return null;
  }
}

/// What a sealed message is bound to, by use: an entry line, the keys a
/// Device publishes, the bytes of a blob.
abstract final class SealedAs {
  static const entry = 'oldbrainfix:entry';
  static String keys(String device) => 'oldbrainfix:keys:$device';

  /// The Profiles of a Shared Area's people a Device publishes, sealed
  /// with that Area's key (ADR 0017).
  static String profiles(String device) => 'oldbrainfix:profiles:$device';
  static String blob(String ref) => 'oldbrainfix:blob:$ref';

  /// The backup key, sealed with the key the passphrase derives.
  static const backupPass = 'oldbrainfix:backup-pass';

  /// A Backup's content key, sealed with the backup key.
  static const backupKey = 'oldbrainfix:backup-key';

  /// Chunk [index] of the Backup whose header line hashes to [header].
  static String backupChunk(String header, int index, {required bool last}) =>
      'oldbrainfix:backup:$header:$index${last ? ':last' : ''}';
}

// ---------------------------------------------------------------- store

extension AreaKeys on Store {
  static const _prefix = 'areakey:';

  /// Every key this Device holds, by id.
  List<AreaKey> areaKeys() => [
    for (final (_, raw) in localSettingsWithPrefix(_prefix)) ?_read(raw),
  ];

  /// The key with [id], if held.
  AreaKey? areaKey(String id) {
    final raw = localSetting('$_prefix$id');
    return raw == null ? null : _read(raw);
  }

  /// The key a writer seals [scope] with: of the keys held for it, the
  /// one of the lowest id; null when none is held.
  AreaKey? writingKey(String scope) {
    AreaKey? pick;
    for (final key in areaKeys()) {
      if (key.scope == scope &&
          (pick == null || key.id.compareTo(pick.id) < 0)) {
        pick = key;
      }
    }
    return pick;
  }

  /// Keeps [keys] another Device handed over; a key already held stays
  /// as it is. Returns how many were new.
  int learnAreaKeys(Iterable<AreaKey> keys) {
    var learned = 0;
    for (final key in keys) {
      if (areaKey(key.id) != null) continue;
      setLocalSetting('$_prefix${key.id}', jsonEncode(key.toJson()));
      learned++;
    }
    return learned;
  }

  /// Makes the keys still missing: the personal one, and one for every
  /// Area that has none here. Run before anything leaves this Device.
  void ensureAreaKeys() {
    final scopes = {for (final k in areaKeys()) k.scope};
    final wanted = [
      personalScope,
      ...entitiesOfType(PlaceKeys.typeArea)..sort(),
    ];
    for (final scope in wanted) {
      if (scopes.contains(scope)) continue;
      learnAreaKeys([AreaKey(scope, secretBytes(areaKeyBytes))]);
    }
  }

  /// The scope [entity] is sealed under, from where it is now: an Area
  /// itself; a Topic by its Area; a Note by its Area or Topic; a Section
  /// or a Routine by its Note; everything else, and whatever is in no
  /// Area, [personalScope]. Read from the log alone: a pending change
  /// (ADR 0022) moves nothing into an Area before it is accepted.
  String scopeOf(String entity) {
    if (entity.startsWith('area:')) return entity;
    String? next;
    if (entity.startsWith('topic:')) {
      next = logged(entity, PlaceKeys.area);
    } else if (entity.startsWith('note:')) {
      next = logged(entity, NoteKeys.place);
    } else if (entity.startsWith('section:')) {
      next = logged(entity, SectionKeys.note);
    } else if (entity.startsWith('routine:')) {
      next = logged(entity, RoutineKeys.note);
    }
    // Each step goes to a kind further up, so the walk ends.
    if (next == null || !_above(entity, next)) return personalScope;
    return scopeOf(next);
  }

  /// The key [entity] is sealed with: its scope's writing key, or the
  /// personal one when its Area has no key here.
  AreaKey? sealingKey(String entity) =>
      writingKey(scopeOf(entity)) ?? writingKey(personalScope);

  static bool _above(String entity, String next) {
    int rank(String id) => switch (id.substring(0, max(0, id.indexOf(':')))) {
      'area' => 0,
      'topic' => 1,
      'note' => 2,
      'section' || 'routine' => 3,
      _ => 9,
    };
    return rank(next) < rank(entity);
  }

  static AreaKey? _read(String raw) {
    try {
      return AreaKey.fromJson(jsonDecode(raw));
    } on FormatException {
      return null;
    }
  }
}

/// The keys of [json], skipping what is no key.
List<AreaKey> areaKeysOf(Object? json) => [
  if (json is List)
    for (final k in json) ?AreaKey.fromJson(k),
];

/// The name a blob [ref] (`photo/<hash>`) goes by in the shared folder
/// when sealed with [key]: an HMAC of the reference, so nobody holding
/// the photo can tell it is there.
String blobName(List<int> key, String ref) =>
    hex(Hmac(sha256, key).convert(utf8.encode(ref)).bytes);
