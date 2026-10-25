import 'dart:convert';
import 'dart:typed_data';

import 'package:characters/characters.dart';
import 'package:crypto/crypto.dart';
import 'package:image/image.dart' as img;

import 'photos.dart';
import 'store.dart';

/// Profiles (ADR 0017): every person has one, with an id, a name and a
/// picture. This Device keeps the user's own as local settings; nothing
/// about it leaves the Device yet (#48 syncs it, signed).

/// The most characters (Unicode code points) a Profile name keeps.
const profileNameMax = 40;

/// The edge of a Profile picture, in pixels; it is square.
const profilePictureEdge = 256;

/// The largest picture taken from someone else, in bytes.
const profilePictureMaxBytes = 256 * 1024;

/// The longest edge of a picture this core decodes at all, so a small
/// file declaring a huge canvas cannot fill the memory.
const pictureDecodeMaxEdge = 8192;

/// The longest edge of a picture taken from someone else: theirs were
/// made at [profilePictureEdge].
const receivedPictureMaxEdge = 1024;

const _quality = 85;

/// The colours of the placeholder, ARGB, each dark enough for white
/// initials on it. Both cores and every screen use this list.
const profileColors = [
  0xFF00796B, // teal
  0xFF1565C0, // blue
  0xFF6A1B9A, // purple
  0xFFAD1457, // pink
  0xFFC62828, // red
  0xFFBF360C, // deep orange
  0xFF2E7D32, // green
  0xFF4E342E, // brown
  0xFF37474F, // blue grey
  0xFF283593, // indigo
  0xFF006064, // cyan
  0xFF4527A0, // deep purple
];

/// One Profile record: its [id], [name], [picture] (a JPEG of
/// [profilePictureEdge] pixels, or null for the placeholder) and its
/// [version], counted up with every change.
class Profile {
  final String id;
  final String name;
  final Uint8List? picture;
  final int version;

  const Profile({
    required this.id,
    required this.name,
    this.picture,
    required this.version,
  });

  /// The record as it is kept and, from #48 on, signed: keys sorted, no
  /// spaces, the same bytes from both cores; no picture, no key.
  String encode() => jsonEncode({
    'id': id,
    'name': name,
    if (picture case final p?) 'picture': base64.encode(p),
    'version': version,
  });

  /// The record [text] holds, its name sanitised again; null when it is
  /// none.
  static Profile? parse(String text) {
    try {
      final json = jsonDecode(text);
      if (json is! Map<String, Object?>) return null;
      final id = json['id'];
      final name = json['name'];
      final picture = json['picture'];
      final version = json['version'];
      if (id is! String || id.isEmpty || name is! String) return null;
      // Half a surrogate pair is no text; the desktop's JSON refuses it.
      if (_halfPair(id) || _halfPair(name)) return null;
      if (version is! int || version < 1) return null;
      if (picture != null && picture is! String) return null;
      final bytes = picture is String ? base64.decode(picture) : null;
      // Only standard base64 as written, the same the desktop takes.
      if (bytes != null && base64.encode(bytes) != picture) return null;
      return Profile(
        id: id,
        name: sanitizeProfileName(name),
        picture: bytes,
        version: version,
      );
    } on FormatException {
      return null;
    }
  }

  Profile _with({String? name, Uint8List? Function()? picture}) => Profile(
    id: id,
    name: name ?? this.name,
    picture: picture == null ? this.picture : picture(),
    version: version + 1,
  );
}

// ------------------------------------------------------------- names

bool _halfPair(String text) =>
    text.runes.any((c) => c >= 0xD800 && c <= 0xDFFF);

bool _isWhitespace(int c) =>
    c == 0x20 ||
    c == 0xA0 ||
    c == 0x1680 ||
    (c >= 0x2000 && c <= 0x200A) ||
    c == 0x202F ||
    c == 0x205F ||
    c == 0x3000;

/// Control, bidi and invisible characters a name never keeps.
bool _isDropped(int c) =>
    c <= 0x1F ||
    (c >= 0x7F && c <= 0x9F) ||
    c == 0xAD ||
    c == 0x34F ||
    c == 0x61C ||
    c == 0x115F ||
    c == 0x1160 ||
    c == 0x17B4 ||
    c == 0x17B5 ||
    (c >= 0xD800 && c <= 0xDFFF) ||
    c == 0x180E ||
    c == 0x200B ||
    c == 0x200E ||
    c == 0x200F ||
    (c >= 0x2028 && c <= 0x202E) ||
    (c >= 0x2060 && c <= 0x206F) ||
    c == 0x3164 ||
    c == 0xFEFF ||
    c == 0xFFA0 ||
    (c >= 0xFFF9 && c <= 0xFFFB) ||
    (c >= 0x1D173 && c <= 0x1D17A) ||
    c == 0xE0000 ||
    c == 0xE0001 ||
    (c >= 0xE0002 && c <= 0xE001F);

bool _isJoiner(int c) => c == 0x200C || c == 0x200D;

bool _isSelector(int c) =>
    (c >= 0xFE00 && c <= 0xFE0F) || (c >= 0xE0100 && c <= 0xE01EF);

bool _isTag(int c) => c >= 0xE0020 && c <= 0xE007F;

const _blackFlag = 0x1F3F4;

/// [raw] as a Profile name: control, bidi and invisible characters
/// removed, trimmed, cut to [profileNameMax] code points and trimmed
/// again; empty when nothing is left, which every screen shows as "No
/// name". Emoji and every script stay: a zero-width joiner or non-joiner
/// stays only between two visible characters, a variation selector only
/// right after one, and tag characters only in a flag after the black
/// flag. The same in both cores, code point for code point.
String sanitizeProfileName(String raw) {
  final kept = <int>[];
  for (final c in raw.runes) {
    if (_isDropped(c)) continue;
    if (_isTag(c)) {
      final last = kept.isEmpty ? null : kept.last;
      final inFlag =
          last == _blackFlag ||
          (last != null && _isTag(last) && last != 0xE007F);
      if (!inFlag) continue;
    }
    kept.add(c);
  }
  final out = <int>[];
  for (var i = 0; i < kept.length; i++) {
    final c = kept[i];
    final before = out.isEmpty ? null : out.last;
    final visibleBefore =
        before != null &&
        !_isWhitespace(before) &&
        !_isJoiner(before) &&
        !_isSelector(before);
    if (_isJoiner(c)) {
      final after = i + 1 < kept.length ? kept[i + 1] : null;
      final visibleAfter =
          after != null &&
          !_isWhitespace(after) &&
          !_isJoiner(after) &&
          !_isSelector(after);
      if (!visibleBefore || !visibleAfter) continue;
    } else if (_isSelector(c)) {
      if (!visibleBefore) continue;
    }
    out.add(c);
  }
  var cut = _trim(out);
  if (cut.length > profileNameMax) {
    cut = cut.sublist(0, profileNameMax);
    // No joiner or half a flag left hanging at the end.
    while (cut.isNotEmpty && (_isJoiner(cut.last) || _isTag(cut.last))) {
      cut.removeLast();
    }
    cut = _trim(cut);
  }
  return String.fromCharCodes(cut);
}

List<int> _trim(List<int> runes) {
  var start = 0, end = runes.length;
  while (start < end && _isWhitespace(runes[start])) {
    start++;
  }
  while (end > start && _isWhitespace(runes[end - 1])) {
    end--;
  }
  return runes.sublist(start, end);
}

/// The placeholder's initials for [name]: the first character of its
/// first and of its last word, upper case; one for a single word, none
/// for no name.
String profileInitials(String name) {
  final words = name
      .split(RegExp('[    -   　]+'))
      .where((w) => w.isNotEmpty)
      .toList();
  if (words.isEmpty) return '';
  String first(String word) => word.characters.first.toUpperCase();
  return words.length == 1
      ? first(words.single)
      : '${first(words.first)}${first(words.last)}';
}

/// The placeholder's colour for the Profile [id], ARGB, from
/// [profileColors].
int profileColor(String id) {
  final hash = sha256.convert(utf8.encode('oldbrainfix:profile-colour:$id'));
  return profileColors[hash.bytes.first % profileColors.length];
}

const _codeAlphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

/// The short code people compare in person, `4F2K-9QX7`: the first 40
/// bits of SHA-256 over `oldbrainfix:profile-code:<id>:` and the first
/// key's 32 public bytes, in Crockford base32. A copied id with another
/// key gives another code.
String profileShortCode(String id, List<int> firstKey) {
  final hash = sha256.convert([
    ...utf8.encode('oldbrainfix:profile-code:$id:'),
    ...firstKey,
  ]).bytes;
  var bits = 0;
  for (var i = 0; i < 5; i++) {
    bits = (bits << 8) | hash[i];
  }
  final out = StringBuffer();
  for (var i = 0; i < 8; i++) {
    if (i == 4) out.write('-');
    out.write(_codeAlphabet[(bits >> (35 - 5 * i)) & 31]);
  }
  return out.toString();
}

// ----------------------------------------------------------- pictures

img.Decoder? _decoderFor(Uint8List bytes) {
  bool starts(List<int> magic) =>
      bytes.length >= magic.length &&
      Iterable.generate(magic.length).every((i) => bytes[i] == magic[i]);
  if (starts(const [0xFF, 0xD8, 0xFF])) return img.JpegDecoder();
  if (starts(const [0x89, 0x50, 0x4E, 0x47])) return img.PngDecoder();
  if (starts(const [0x52, 0x49, 0x46, 0x46]) &&
      bytes.length >= 12 &&
      String.fromCharCodes(bytes.sublist(8, 12)) == 'WEBP') {
    return img.WebPDecoder();
  }
  return null;
}

/// [bytes] decoded and turned upright; null when they are no JPEG, PNG
/// or WebP, are broken, or declare a canvas wider or higher than
/// [maxEdge].
img.Image? _decode(Uint8List bytes, {int maxEdge = pictureDecodeMaxEdge}) {
  final decoder = _decoderFor(bytes);
  if (decoder == null) return null;
  try {
    final info = decoder.startDecode(bytes);
    if (info == null ||
        info.width < 1 ||
        info.height < 1 ||
        info.width > maxEdge ||
        info.height > maxEdge) {
      return null;
    }
    final image = decoder.decode(bytes, frame: 0);
    return image == null ? null : img.bakeOrientation(image);
  } catch (_) {
    // Broken data throws from deep inside the decoders, in any type.
    return null;
  }
}

/// The width and height of the upright picture [bytes], or null when it
/// is none this core reads.
({int width, int height})? pictureSize(Uint8List bytes) {
  final image = _decode(bytes);
  return image == null ? null : (width: image.width, height: image.height);
}

/// The square at [left], [top] with the edge [size] (pixels of the
/// upright picture [bytes], kept inside it) as a Profile picture: JPEG,
/// [profilePictureEdge] pixels, transparent parts white, no metadata.
/// Null when [bytes] are no picture.
Uint8List? makeProfilePicture(
  Uint8List bytes, {
  required int left,
  required int top,
  required int size,
}) {
  final image = _decode(bytes);
  if (image == null) return null;
  return _square(image, left: left, top: top, size: size);
}

/// The middle square of the picture [bytes] as a Profile picture, for a
/// picture nobody cropped, such as the login's account picture.
Uint8List? squareProfilePicture(Uint8List bytes) =>
    _middleSquare(bytes, pictureDecodeMaxEdge);

/// A Profile picture someone else sent, decoded and made again as
/// [squareProfilePicture] makes it; null when it is larger than
/// [profilePictureMaxBytes], wider or higher than
/// [receivedPictureMaxEdge], or no picture.
Uint8List? receivedProfilePicture(Uint8List bytes) =>
    bytes.length > profilePictureMaxBytes
    ? null
    : _middleSquare(bytes, receivedPictureMaxEdge);

Uint8List? _middleSquare(Uint8List bytes, int maxEdge) {
  final image = _decode(bytes, maxEdge: maxEdge);
  if (image == null) return null;
  final size = image.width < image.height ? image.width : image.height;
  return _square(
    image,
    left: (image.width - size) ~/ 2,
    top: (image.height - size) ~/ 2,
    size: size,
  );
}

Uint8List _square(
  img.Image image, {
  required int left,
  required int top,
  required int size,
}) {
  final shortest = image.width < image.height ? image.width : image.height;
  final edge = size.clamp(1, shortest);
  final x = left.clamp(0, image.width - edge);
  final y = top.clamp(0, image.height - edge);
  final cropped = img.copyCrop(image, x: x, y: y, width: edge, height: edge);
  final resized = img.copyResize(
    cropped,
    width: profilePictureEdge,
    height: profilePictureEdge,
    interpolation: edge > profilePictureEdge
        ? img.Interpolation.average
        : img.Interpolation.linear,
  );
  final canvas = img.Image(
    width: profilePictureEdge,
    height: profilePictureEdge,
  )..clear(img.ColorRgb8(255, 255, 255));
  img.compositeImage(canvas, resized);
  return stripJpegMetadata(img.encodeJpg(canvas, quality: _quality));
}

// ------------------------------------------------------- own Profile

/// The user's own Profile on this Device (ADR 0017), kept as local
/// settings: `profile` holds the record, `profile:first` the public key
/// of the Device that made it (base64), and `profile:nameChosen` and
/// `profile:pictureChosen` say the user replaced a default.
extension OwnProfile on Store {
  static const _record = 'profile';
  static const _first = 'profile:first';
  static const _nameChosen = 'profile:nameChosen';
  static const _pictureChosen = 'profile:pictureChosen';

  /// The user's own Profile; null before [applyProfileDefaults] made it.
  Profile? get ownProfile {
    final text = localSetting(_record);
    return text == null ? null : Profile.parse(text);
  }

  /// The first key of the own Profile: this Device's signing key when it
  /// made the Profile.
  Uint8List? get ownProfileFirstKey {
    final text = localSetting(_first);
    return text == null ? null : base64.decode(text);
  }

  /// The own Profile's short code, or null before it exists.
  String? get ownProfileCode {
    final profile = ownProfile;
    final key = ownProfileFirstKey;
    return profile == null || key == null
        ? null
        : profileShortCode(profile.id, key);
  }

  /// Makes the own Profile the first time, with a new id, the default
  /// [name] and [picture] (already a Profile picture, or null for the
  /// placeholder) and this Device's key as its first key; later it takes
  /// the defaults the user has not replaced. Asks nothing; counts the
  /// version up only when something changed.
  Profile applyProfileDefaults({required String name, Uint8List? picture}) {
    final clean = sanitizeProfileName(name);
    final current = ownProfile;
    if (current == null) {
      final made = Profile(
        id: Store.newId(),
        name: clean,
        picture: picture,
        version: 1,
      );
      setLocalSetting(_first, base64.encode(signingKey.publicKey));
      setLocalSetting(_record, made.encode());
      return made;
    }
    final takeName =
        localSetting(_nameChosen) != 'true' && clean != current.name;
    final takePicture =
        localSetting(_pictureChosen) != 'true' &&
        !_same(picture, current.picture);
    if (!takeName && !takePicture) return current;
    final next = current._with(
      name: takeName ? clean : null,
      picture: takePicture ? () => picture : null,
    );
    setLocalSetting(_record, next.encode());
    return next;
  }

  /// Gives the own Profile the [name] the user typed, sanitised.
  Profile renameOwnProfile(String name) {
    final current = _own;
    setLocalSetting(_nameChosen, 'true');
    final clean = sanitizeProfileName(name);
    if (clean == current.name) return current;
    final next = current._with(name: clean);
    setLocalSetting(_record, next.encode());
    return next;
  }

  /// Gives the own Profile the [picture] the user chose (a Profile
  /// picture), or the placeholder for null.
  Profile setOwnProfilePicture(Uint8List? picture) {
    final current = _own;
    setLocalSetting(_pictureChosen, 'true');
    if (_same(picture, current.picture)) return current;
    final next = current._with(picture: () => picture);
    setLocalSetting(_record, next.encode());
    return next;
  }

  Profile get _own =>
      ownProfile ??
      (throw StateError('no own Profile yet: applyProfileDefaults first'));
}

bool _same(Uint8List? a, Uint8List? b) {
  if (a == null || b == null) return a == b;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
