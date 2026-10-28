import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:pointycastle/digests/sha256.dart';
import 'package:pointycastle/ecc/api.dart';
import 'package:pointycastle/ecc/curves/secp256r1.dart';
import 'package:pointycastle/macs/hmac.dart';
import 'package:pointycastle/signers/ecdsa_signer.dart';
import 'package:pointycastle/api.dart' show PrivateKeyParameter;

import 'store.dart';

/// The certificate a LAN host serves with and the key behind it (ADR
/// 0003), made once per install as the desktop core's `tls_identity`
/// makes it: a self-signed ECDSA P-256 certificate whose SHA-256
/// fingerprint the pair code carries and paired Devices pin. Kept as DER,
/// the key as PKCS#8, under the same local settings in both cores.
class HostIdentity {
  final Uint8List certDer;
  final Uint8List keyDer;

  const HostIdentity(this.certDer, this.keyDer);

  /// SHA-256 over the certificate's DER.
  List<int> get fingerprint => sha256.convert(certDer).bytes;

  /// A context serving with this certificate.
  SecurityContext securityContext() => SecurityContext()
    ..useCertificateChainBytes(utf8.encode(_pem('CERTIFICATE', certDer)))
    ..usePrivateKeyBytes(utf8.encode(_pem('PRIVATE KEY', keyDer)));
}

/// The host identity of this Device, made the first time it is asked
/// for and kept, so a paired Device knows it again.
extension HostIdentities on Store {
  static const certKey = 'tls:cert';
  static const keyKey = 'tls:key';

  /// This Device's identity; made and kept when there is none yet, with
  /// [random] bytes (secure ones unless handed in) and the store's clock.
  HostIdentity hostIdentity({Uint8List Function(int length)? random}) {
    if (heldHostIdentity case final held?) return held;
    final made = makeHostIdentity(now: clock(), random: random);
    setLocalSetting(certKey, base64.encode(made.certDer));
    setLocalSetting(keyKey, base64.encode(made.keyDer));
    return made;
  }

  /// The identity kept here, or null before this Device first hosted or
  /// joined.
  HostIdentity? get heldHostIdentity {
    final cert = localSetting(certKey);
    final key = localSetting(keyKey);
    if (cert == null || key == null) return null;
    try {
      return HostIdentity(base64.decode(cert), base64.decode(key));
    } on FormatException {
      return null;
    }
  }
}

/// A fresh self-signed certificate, ECDSA P-256 over SHA-256, valid from
/// [now] with no end (RFC 5280's 9999-12-31), for the name
/// `oldbrainfix`, as the desktop core's `rcgen` makes one. The private
/// key and the serial come from [random]; the signature is deterministic
/// (RFC 6979), so the same bytes make the same certificate.
HostIdentity makeHostIdentity({
  required DateTime now,
  Uint8List Function(int length)? random,
}) {
  final bytes = random ?? _secureBytes;
  final curve = ECCurve_secp256r1();
  final n = curve.n;
  final d = _unsigned(bytes(32)) % (n - BigInt.one) + BigInt.one;
  final point = curve.G * d;
  if (point == null) throw StateError('no public key');
  final public = point.getEncoded(false);
  final serial = bytes(16);
  // Positive, and with no leading zero byte to strip.
  serial[0] = (serial[0] & 0x7f) | 0x40;

  final name = _seq([
    _set([
      _seq([_oid(_commonName), _tlv(0x0c, utf8.encode('oldbrainfix'))]),
    ]),
  ]);
  final algorithm = _seq([_oid(_ecdsaWithSha256)]);
  final tbs = _seq([
    _tlv(0xa0, _int(Uint8List.fromList([2]))),
    _int(serial),
    algorithm,
    name,
    _seq([_time(now.toUtc()), _tlv(0x18, ascii.encode('99991231235959Z'))]),
    name,
    _seq([
      _seq([_oid(_ecPublicKey), _oid(_prime256v1)]),
      _bitString(public),
    ]),
    _tlv(
      0xa3,
      _seq([
        _seq([
          _oid(_subjectAltName),
          _tlv(0x04, _seq([_tlv(0x82, ascii.encode('oldbrainfix'))])),
        ]),
      ]),
    ),
  ]);

  final signer = ECDSASigner(SHA256Digest(), HMac(SHA256Digest(), 64))
    ..init(true, PrivateKeyParameter<ECPrivateKey>(ECPrivateKey(d, curve)));
  final signature = signer.generateSignature(tbs) as ECSignature;
  final cert = _seq([
    tbs,
    algorithm,
    _bitString(
      _seq([_int(_bytesOf(signature.r)), _int(_bytesOf(signature.s))]),
    ),
  ]);

  // PKCS#8 around an RFC 5915 key, its curve named outside and its
  // public key inside, as ring reads it.
  final key = _seq([
    _int(Uint8List.fromList([0])),
    _seq([_oid(_ecPublicKey), _oid(_prime256v1)]),
    _tlv(
      0x04,
      _seq([
        _int(Uint8List.fromList([1])),
        _tlv(0x04, _bytesOf(d, length: 32)),
        _tlv(0xa1, _bitString(public)),
      ]),
    ),
  ]);
  return HostIdentity(cert, key);
}

const _commonName = '2.5.4.3';
const _subjectAltName = '2.5.29.17';
const _ecPublicKey = '1.2.840.10045.2.1';
const _prime256v1 = '1.2.840.10045.3.1.7';
const _ecdsaWithSha256 = '1.2.840.10045.4.3.2';

Uint8List _secureBytes(int length) {
  final random = Random.secure();
  return Uint8List.fromList([
    for (var i = 0; i < length; i++) random.nextInt(256),
  ]);
}

BigInt _unsigned(List<int> bytes) {
  var value = BigInt.zero;
  for (final b in bytes) {
    value = (value << 8) | BigInt.from(b);
  }
  return value;
}

/// [value] big-endian, unsigned, in at least [length] bytes.
Uint8List _bytesOf(BigInt value, {int length = 0}) {
  final out = <int>[];
  var rest = value;
  while (rest > BigInt.zero) {
    out.add((rest & BigInt.from(0xff)).toInt());
    rest = rest >> 8;
  }
  while (out.length < length) {
    out.add(0);
  }
  return Uint8List.fromList(out.reversed.toList());
}

Uint8List _tlv(int tag, List<int> content) {
  final length = content.length;
  final head = length < 0x80
      ? [length]
      : length < 0x100
      ? [0x81, length]
      : [0x82, length >> 8, length & 0xff];
  return Uint8List.fromList([tag, ...head, ...content]);
}

Uint8List _seq(List<List<int>> parts) =>
    _tlv(0x30, [for (final p in parts) ...p]);

Uint8List _set(List<List<int>> parts) =>
    _tlv(0x31, [for (final p in parts) ...p]);

/// A positive INTEGER from its unsigned big-endian [bytes], in the
/// fewest bytes DER allows.
Uint8List _int(List<int> bytes) {
  var start = 0;
  while (start < bytes.length - 1 && bytes[start] == 0) {
    start++;
  }
  final trimmed = bytes.sublist(start);
  return _tlv(0x02, [if (trimmed.isEmpty || trimmed[0] >= 0x80) 0, ...trimmed]);
}

Uint8List _bitString(List<int> bytes) => _tlv(0x03, [0, ...bytes]);

Uint8List _oid(String dotted) {
  final arcs = [for (final a in dotted.split('.')) int.parse(a)];
  final out = <int>[arcs[0] * 40 + arcs[1]];
  for (final arc in arcs.skip(2)) {
    final groups = <int>[arc & 0x7f];
    var rest = arc >> 7;
    while (rest > 0) {
      groups.add((rest & 0x7f) | 0x80);
      rest >>= 7;
    }
    out.addAll(groups.reversed);
  }
  return _tlv(0x06, out);
}

/// UTCTime before 2050, GeneralizedTime from then on (RFC 5280).
Uint8List _time(DateTime at) {
  String two(int n) => n.toString().padLeft(2, '0');
  final rest =
      '${two(at.month)}${two(at.day)}${two(at.hour)}${two(at.minute)}'
      '${two(at.second)}Z';
  return at.year < 2050
      ? _tlv(0x17, ascii.encode('${two(at.year % 100)}$rest'))
      : _tlv(0x18, ascii.encode('${at.year.toString().padLeft(4, '0')}$rest'));
}

String _pem(String label, List<int> der) {
  final text = base64.encode(der);
  final lines = [
    for (var i = 0; i < text.length; i += 64)
      text.substring(i, min(i + 64, text.length)),
  ];
  return '-----BEGIN $label-----\n${lines.join('\n')}\n-----END $label-----\n';
}
