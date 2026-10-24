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
