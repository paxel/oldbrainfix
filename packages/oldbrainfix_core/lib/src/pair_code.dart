import 'dart:io';
import 'dart:typed_data';

/// The pair code (Catlog's, copied and adapted): host, port, PIN and the
/// host certificate's fingerprint packed into Crockford base32
/// (lowercase, no i/l/o/u), grouped in fives. The QR carries the whole
/// fingerprint, the typed code its first [typedFingerprintBytes] bytes.
const _alphabet = '0123456789abcdefghjkmnpqrstvwxyz';

/// What a pair code names.
class PairInfo {
  final String host;
  final int port;
  final String pin;

  /// The host certificate's SHA-256 fingerprint, or its first bytes from
  /// a typed code.
  final List<int> fingerprint;

  const PairInfo(this.host, this.port, this.pin, this.fingerprint);
}

/// How much of the fingerprint a typed code carries: enough against a
/// sniffer on the same network, short enough to type.
const typedFingerprintBytes = 8;

/// A whole SHA-256 fingerprint, as the QR carries it.
const fullFingerprintBytes = 32;

/// How many base32 characters a code of [addressBytes] + port + PIN +
/// [fingerprintBytes] takes: five bits per character, rounded up.
int pairCodeLength(int addressBytes, int fingerprintBytes) =>
    ((addressBytes + 5 + fingerprintBytes) * 8 + 4) ~/ 5;

/// Character count → (address bytes, fingerprint bytes) for every shape
/// a code of version 1 can have, the only version written: it carries no
/// version and is told by its length alone.
final Map<int, (int, int)> _shapes = {
  for (final address in const [4, 16])
    for (final fp in const [typedFingerprintBytes, fullFingerprintBytes])
      pairCodeLength(address, fp): (address, fp),
};

/// The characters a code of a later version starts with, its version
/// (ADR 0034): `2` to `9`. No code of version 1 on a local network starts
/// with one: those of 10.x start with `1`, the others with a letter.
const _laterVersions = '23456789';

/// Whether [host] is an address on a local network, the only place a
/// Device to pair with in person can be. A code naming a public address
/// would send every entry to a stranger before any answer came back.
bool isPrivateHost(String host) {
  final parts = host.split('.');
  if (parts.length != 4) return false;
  final o = parts.map(int.tryParse).toList();
  if (o.any((n) => n == null || n < 0 || n > 255)) return false;
  final a = o[0]!, b = o[1]!;
  return a == 10 ||
      (a == 172 && b >= 16 && b <= 31) ||
      (a == 192 && b == 168) ||
      (a == 169 && b == 254) ||
      a == 127;
}

/// The code for [host], [port], the six-digit [pin] and the certificate
/// [fingerprint]: all of it for a QR, its first bytes when [typed].
String encodePairCode(
  String host,
  int port,
  String pin,
  List<int> fingerprint, {
  bool typed = false,
}) {
  final raw = InternetAddress(host).rawAddress;
  final pinNumber = int.parse(pin);
  final fp = typed
      ? fingerprint.take(typedFingerprintBytes).toList()
      : fingerprint;
  final bytes = Uint8List(raw.length + 5 + fp.length)
    ..setAll(0, raw)
    ..[raw.length] = port >> 8
    ..[raw.length + 1] = port & 0xff
    ..[raw.length + 2] = pinNumber >> 16
    ..[raw.length + 3] = (pinNumber >> 8) & 0xff
    ..[raw.length + 4] = pinNumber & 0xff
    ..setAll(raw.length + 5, fp);
  return _group(_toBase32(bytes));
}

/// Reads a code as the user typed it: underscores, spaces, dashes, case
/// and the classic confusions (i/l→1, o→0, u→v) are all forgiven.
/// Returns null when it is not a code, or one of a later version
/// ([pairCodeNeedsUpdate]).
PairInfo? decodePairCode(String input) {
  final cleaned = _clean(input);
  final bytes = _fromBase32(cleaned);
  final shape = _shapes[cleaned.length];
  if (bytes == null || shape == null) return null;
  final (addressLength, fpLength) = shape;
  if (bytes.length < addressLength + 5 + fpLength) return null;
  final address = InternetAddress.fromRawAddress(
    Uint8List.fromList(bytes.sublist(0, addressLength)),
  );
  final port = (bytes[addressLength] << 8) | bytes[addressLength + 1];
  final pin =
      (bytes[addressLength + 2] << 16) |
      (bytes[addressLength + 3] << 8) |
      bytes[addressLength + 4];
  if (port == 0 || pin > 999999) return null;
  return PairInfo(
    address.address,
    port,
    pin.toString().padLeft(6, '0'),
    bytes.sublist(addressLength + 5, addressLength + 5 + fpLength),
  );
}

/// Whether [input], read as [decodePairCode] reads it, is a code of a
/// later version (ADR 0034): not of a length version 1 has, at least as
/// long as its shortest, and starting with its version, `2` to `9`. This
/// version cannot join with it: the app needs an update.
bool pairCodeNeedsUpdate(String input) {
  final cleaned = _clean(input);
  return !_shapes.containsKey(cleaned.length) &&
      cleaned.length >= pairCodeLength(4, typedFingerprintBytes) &&
      _laterVersions.contains(cleaned[0]) &&
      _fromBase32(cleaned) != null;
}

/// [input] as a code is read: underscores, spaces and dashes out, lower
/// case, i/l→1, o→0, u→v.
String _clean(String input) => input
    .toLowerCase()
    .replaceAll(RegExp(r'[\s_\-]'), '')
    .replaceAll('i', '1')
    .replaceAll('l', '1')
    .replaceAll('o', '0')
    .replaceAll('u', 'v');

/// The code's groups in lines of three, as the screens show them.
List<String> pairCodeLines(String code) {
  final groups = code.split('_');
  return [
    for (var i = 0; i < groups.length; i += 3)
      groups
          .sublist(i, i + 3 > groups.length ? groups.length : i + 3)
          .join('_'),
  ];
}

String _group(String s) {
  final parts = <String>[];
  for (var i = 0; i < s.length; i += 5) {
    parts.add(s.substring(i, i + 5 > s.length ? s.length : i + 5));
  }
  return parts.join('_');
}

String _toBase32(Uint8List bytes) {
  final out = StringBuffer();
  var buffer = 0;
  var bits = 0;
  for (final byte in bytes) {
    buffer = ((buffer << 8) | byte) & 0xffff;
    bits += 8;
    while (bits >= 5) {
      out.write(_alphabet[(buffer >> (bits - 5)) & 31]);
      bits -= 5;
    }
  }
  if (bits > 0) out.write(_alphabet[(buffer << (5 - bits)) & 31]);
  return out.toString();
}

List<int>? _fromBase32(String s) {
  final out = <int>[];
  var buffer = 0;
  var bits = 0;
  for (final rune in s.runes) {
    final value = _alphabet.indexOf(String.fromCharCode(rune));
    if (value < 0) return null;
    buffer = ((buffer << 5) | value) & 0xffff;
    bits += 5;
    if (bits >= 8) {
      out.add((buffer >> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }
  return out;
}
