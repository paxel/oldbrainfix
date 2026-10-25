import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:ed25519_edwards/ed25519_edwards.dart' as ed;

/// Signed entries (ADR 0002): every Device signs what it writes with an
/// Ed25519 key made once per install and bound to its device id, so
/// nobody can write in someone else's name from a hand-made file.
class SigningKey {
  final Uint8List seed;
  final ed.PrivateKey _private;
  final Uint8List publicKey;

  SigningKey._(this.seed, this._private, this.publicKey);

  factory SigningKey.fromSeed(Uint8List seed) {
    final private = ed.newKeyFromSeed(seed);
    return SigningKey._(
      seed,
      private,
      Uint8List.fromList(ed.public(private).bytes),
    );
  }

  factory SigningKey.generate() {
    final pair = ed.generateKey();
    return SigningKey.fromSeed(Uint8List.fromList(ed.seed(pair.privateKey)));
  }

  String get seedHex => hex(seed);
  String get publicBase64 => base64.encode(publicKey);

  /// The key's fingerprint: SHA-256 of the public key, hex.
  String get fingerprint => keyFingerprint(publicKey);

  /// What the UI shows next to a name: `7f3a-c21e`.
  String get code => keyCode(publicKey);

  String sign(Uint8List message) => base64.encode(ed.sign(_private, message));
}

String hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

Uint8List fromHex(String s) => Uint8List.fromList([
  for (var i = 0; i + 1 < s.length; i += 2)
    int.parse(s.substring(i, i + 2), radix: 16),
]);

String keyFingerprint(List<int> publicKey) =>
    hex(sha256.convert(publicKey).bytes);

/// The short form of a fingerprint, for people: eight hex digits with a
/// dash, `7f3a-c21e`.
String keyCode(List<int> publicKey) {
  final f = keyFingerprint(publicKey);
  return '${f.substring(0, 4)}-${f.substring(4, 8)}';
}

/// A device id derived from its key: the first 16 bytes of the
/// fingerprint as hex.
String deviceIdFromKey(List<int> publicKey) =>
    keyFingerprint(publicKey).substring(0, 32);

/// Whether [signature] (base64) signs [message] under [publicKey]. Any
/// malformed input is simply invalid.
bool verifySignature(
  List<int> publicKey,
  Uint8List message,
  String? signature,
) {
  if (signature == null || publicKey.length != ed.PublicKeySize) return false;
  try {
    final sig = base64.decode(signature);
    if (sig.length != ed.SignatureSize) return false;
    return ed.verify(
      ed.PublicKey(Uint8List.fromList(publicKey)),
      message,
      Uint8List.fromList(sig),
    );
  } catch (_) {
    return false;
  }
}

/// The bytes an entry's signature covers: a fixed-order JSON list, so
/// a null value and an empty one differ and free text needs no escaping
/// rules of its own. Dates go in as the store writes them.
Uint8List entryBytes({
  required String device,
  required int dseq,
  required String entity,
  required String field,
  required String? value,
  required String dateIso,
  required String author,
  required String recordedIso,
}) => Uint8List.fromList(
  utf8.encode(
    jsonEncode([
      device,
      dseq,
      entity,
      field,
      value,
      dateIso,
      author,
      recordedIso,
    ]),
  ),
);

/// How far a pinned key is trusted: met in a payload some other Device
/// carried (trusted on first use), or met in person over a session the
/// pair code authenticated.
enum KeyTrust { tofu, verified }

/// A Device's public key as it travels with the entries, so a Device can
/// check the signatures of Devices it never met directly. The record
/// signs itself, and the device id is the key's own fingerprint
/// ([deviceIdFromKey]): nobody can publish a key for a device id they do
/// not hold the key of. Copied from Catlog and narrowed: every entry here
/// is signed from the first one, so there is no "signed since".
class KeyRecord {
  final String device;
  final Uint8List publicKey;
  final String signature;

  const KeyRecord(this.device, this.publicKey, this.signature);

  static Uint8List _bytes(String device, List<int> publicKey) =>
      Uint8List.fromList(
        utf8.encode(jsonEncode([device, base64.encode(publicKey)])),
      );

  /// This Device's record, signed by its [key].
  factory KeyRecord.make(SigningKey key) {
    final device = deviceIdFromKey(key.publicKey);
    return KeyRecord(
      device,
      key.publicKey,
      key.sign(_bytes(device, key.publicKey)),
    );
  }

  /// True when the record names the device its key belongs to and was
  /// signed by that key.
  bool get valid =>
      publicKey.length == 32 &&
      deviceIdFromKey(publicKey) == device &&
      verifySignature(publicKey, _bytes(device, publicKey), signature);

  /// What the UI shows for the key: `7f3a-c21e`.
  String get code => keyCode(publicKey);

  Map<String, dynamic> toJson() => {
    'device': device,
    'key': base64.encode(publicKey),
    'sig': signature,
  };

  /// The record in [json], or null when it is not one.
  static KeyRecord? fromJson(Object? json) {
    if (json is! Map) return null;
    final device = json['device'];
    final key = json['key'];
    final sig = json['sig'];
    if (device is! String || key is! String || sig is! String) return null;
    try {
      return KeyRecord(device, base64.decode(key), sig);
    } on FormatException {
      return null;
    }
  }
}

/// A key this Device holds for another Device, with how far it is
/// trusted.
class PinnedKey {
  final KeyRecord record;
  final KeyTrust trust;

  const PinnedKey(this.record, this.trust);

  Map<String, dynamic> toJson() => {...record.toJson(), 'trust': trust.name};

  static PinnedKey? fromJson(Object? json) {
    final record = KeyRecord.fromJson(json);
    if (record == null || json is! Map) return null;
    return PinnedKey(
      record,
      KeyTrust.values.asNameMap()[json['trust']] ?? KeyTrust.tofu,
    );
  }
}

/// What an import found out: the entries it refused for a missing or
/// wrong signature, and the keys it met for the first time.
class ImportReport {
  /// Refused entries by (author, device): how many.
  final Map<(String author, String device), int> refused = {};

  /// Keys pinned by this import.
  final List<PinnedKey> newKeys = [];

  int get refusedCount => refused.values.fold(0, (a, b) => a + b);

  void refuse(String author, String device) =>
      refused.update((author, device), (n) => n + 1, ifAbsent: () => 1);
}
