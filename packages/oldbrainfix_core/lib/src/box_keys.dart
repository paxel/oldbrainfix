import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:cryptography/cryptography.dart'
    show KeyPairType, SecretKeyData, SimpleKeyPairData, SimplePublicKey;
import 'package:cryptography/dart.dart' show DartX25519;

/// The keys a Sent Item is encrypted to (ADR 0015): every Device has an
/// X25519 key of its own beside its Ed25519 signing key, derived from
/// the signing key's seed so there is nothing more to keep, and
/// published in its key record signed by the signing key.
const boxKeyBytes = 32;

/// HKDF with HMAC-SHA256 (RFC 5869): [length] bytes from [ikm], bound
/// to [salt] and [info].
Uint8List hkdfSha256(
  List<int> ikm, {
  List<int> salt = const [],
  required List<int> info,
  int length = 32,
}) {
  final prk = Hmac(
    sha256,
    salt.isEmpty ? Uint8List(32) : salt,
  ).convert(ikm).bytes;
  final out = BytesBuilder(copy: false);
  var block = <int>[];
  for (var i = 1; out.length < length; i++) {
    block = Hmac(sha256, prk).convert([...block, ...info, i]).bytes;
    out.add(block);
  }
  return Uint8List.sublistView(out.takeBytes(), 0, length);
}

/// The X25519 secret of the Device whose signing key has [seed].
Uint8List boxSecretOf(List<int> seed) =>
    hkdfSha256(seed, info: utf8.encode('oldbrainfix:box'));

const _x25519 = DartX25519();

/// X25519 of [secret] and [public] (RFC 7748); null when the result is
/// all zeros, as for a public key of low order.
Uint8List? x25519(List<int> secret, List<int> public) {
  if (secret.length != boxKeyBytes || public.length != boxKeyBytes) {
    return null;
  }
  final shared = _x25519.sharedSecretSync(
    keyPairData: SimpleKeyPairData(
      secret,
      // Not read: only the secret goes into the ladder.
      publicKey: SimplePublicKey(_base, type: KeyPairType.x25519),
      type: KeyPairType.x25519,
    ),
    remotePublicKey: SimplePublicKey(public, type: KeyPairType.x25519),
  );
  final bytes = Uint8List.fromList((shared as SecretKeyData).bytes);
  return bytes.every((b) => b == 0) ? null : bytes;
}

/// The u-coordinate 9 of the curve's base point.
final _base = Uint8List(boxKeyBytes)..[0] = 9;

/// The public X25519 key of [secret]. The base point has prime order and
/// a clamped secret is never zero, so the result never is either.
Uint8List boxPublicOf(List<int> secret) => x25519(secret, _base)!;
