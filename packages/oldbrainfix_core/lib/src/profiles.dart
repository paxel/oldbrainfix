import 'dart:convert';
import 'dart:typed_data';

import 'package:characters/characters.dart';
import 'package:crypto/crypto.dart';
import 'package:image/image.dart' as img;

import 'photos.dart';
import 'signing.dart';
import 'store.dart';

/// Profiles (ADR 0017): every person has one, with an id, a name and a
/// picture. This Device keeps the user's own as local settings, signed
/// and bound to the keys of the user's own Devices, and syncs it with
/// them (#48).

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

  /// The record as it is kept and signed: keys sorted, no spaces, the
  /// same bytes from both cores; no picture, no key.
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

// ------------------------------------------------ signing and binding

/// The bytes a Profile record's signature covers: the record as sent.
Uint8List profileRecordBytes(String record) =>
    utf8.encode('oldbrainfix:profile:$record');

/// The bytes that bind the key [key] (base64) to the Profile [id].
Uint8List profileKeyBytes(String id, String key) =>
    utf8.encode('oldbrainfix:profile-key:$id:$key');

/// The bytes that make the Profile id [alias] an alias of [to].
Uint8List profileAliasBytes(String alias, String to) =>
    utf8.encode('oldbrainfix:profile-alias:$alias:$to');

/// Whether [text] is a public key in standard base64, as written.
bool _isKey(String text) {
  try {
    final bytes = base64.decode(text);
    return bytes.length == 32 && base64.encode(bytes) == text;
  } on FormatException {
    return false;
  }
}

/// A key joined to a Profile's binding: [key], and [sig] over
/// [profileKeyBytes] by [by], a key bound before it (both base64).
class ProfileKey {
  final String key;
  final String by;
  final String sig;

  const ProfileKey({required this.key, required this.by, required this.sig});

  Map<String, Object?> toJson() => {'by': by, 'key': key, 'sig': sig};

  static ProfileKey? fromJson(Object? json) {
    if (json
        case {
          'by': final String by,
          'key': final String key,
          'sig': final String sig,
        }
        when _isKey(by) && _isKey(key)) {
      return ProfileKey(key: key, by: by, sig: sig);
    }
    return null;
  }
}

/// The Profile id [alias] given up for [to]: [sig] over
/// [profileAliasBytes] by [by], a key bound to the Profile given up.
class ProfileAlias {
  final String alias;
  final String to;
  final String by;
  final String sig;

  const ProfileAlias({
    required this.alias,
    required this.to,
    required this.by,
    required this.sig,
  });

  Map<String, Object?> toJson() => {
    'alias': alias,
    'by': by,
    'sig': sig,
    'to': to,
  };

  static ProfileAlias? fromJson(Object? json) {
    if (json
        case {
          'alias': final String alias,
          'by': final String by,
          'sig': final String sig,
          'to': final String to,
        }
        when alias.isNotEmpty && to.isNotEmpty && _isKey(by)) {
      return ProfileAlias(alias: alias, to: to, by: by, sig: sig);
    }
    return null;
  }

  /// Whether [by] made [sig].
  bool get signed =>
      verifySignature(base64.decode(by), profileAliasBytes(alias, to), sig);
}

/// A Profile as it travels between Devices (ADR 0017): the [record]
/// as its signer sent it, signed by [signer] ([sig]); its binding, the
/// [first] key and the [keys] joined since; the ids it replaced
/// ([aliases]); and, between one's own Devices only, whether it still
/// has the defaults ([fresh]).
class SignedProfile {
  final String record;
  final String signer;
  final String sig;
  final String first;
  final List<ProfileKey> keys;
  final List<ProfileAlias> aliases;
  final bool fresh;

  const SignedProfile({
    required this.record,
    required this.signer,
    required this.sig,
    required this.first,
    this.keys = const [],
    this.aliases = const [],
    this.fresh = false,
  });

  /// Keys sorted, no spaces, the same bytes from both cores.
  Map<String, Object?> toJson() => {
    'aliases': [for (final a in aliases) a.toJson()],
    'first': first,
    if (fresh) 'fresh': true,
    'keys': [for (final k in keys) k.toJson()],
    'record': record,
    'sig': sig,
    'signer': signer,
  };

  /// The signed Profile [json] holds, or null when it is none. Keys and
  /// aliases that are no such things are left out.
  static SignedProfile? fromJson(Object? json) {
    if (json
        case {
          'record': final String record,
          'signer': final String signer,
          'sig': final String sig,
          'first': final String first,
          'keys': final List keys,
          'aliases': final List aliases,
        }
        when _isKey(signer) && _isKey(first)) {
      final fresh = json['fresh'];
      if (fresh != null && fresh is! bool) return null;
      return SignedProfile(
        record: record,
        signer: signer,
        sig: sig,
        first: first,
        keys: [for (final k in keys) ?ProfileKey.fromJson(k)],
        aliases: [for (final a in aliases) ?ProfileAlias.fromJson(a)],
        fresh: fresh == true,
      );
    }
    return null;
  }
}

/// The keys bound to the Profile [id] from [first] on: every key of
/// [held], then of [offered], signed by one bound before it, until no
/// more join; with the statements that bound them, sorted by key.
(Set<String>, List<ProfileKey>) _chain(
  String id,
  String first,
  Iterable<ProfileKey> offered, {
  Iterable<ProfileKey> held = const [],
}) {
  final bound = {first};
  final kept = <ProfileKey>[];
  final pending = [...held, ...offered];
  var grew = true;
  while (grew) {
    grew = false;
    for (final k in pending) {
      if (bound.contains(k.key) || !bound.contains(k.by)) continue;
      if (!verifySignature(
        base64.decode(k.by),
        profileKeyBytes(id, k.key),
        k.sig,
      )) {
        continue;
      }
      bound.add(k.key);
      kept.add(k);
      grew = true;
    }
  }
  kept.sort((a, b) => a.key.compareTo(b.key));
  return (bound, kept);
}

/// A received Profile whose signatures held: its record read, the keys
/// joined to its binding, and its picture made again for the screen.
typedef _Checked = ({Profile profile, List<ProfileKey> keys, Uint8List? shown});

/// [sent] checked: its binding from the first key, its signer bound,
/// the signature over its record, the record, and its picture made
/// again; null when any of it fails.
_Checked? _check(SignedProfile sent) {
  final profile = Profile.parse(sent.record);
  if (profile == null) return null;
  final (bound, keys) = _chain(profile.id, sent.first, sent.keys);
  if (!bound.contains(sent.signer)) return null;
  if (!verifySignature(
    base64.decode(sent.signer),
    profileRecordBytes(sent.record),
    sent.sig,
  )) {
    return null;
  }
  Uint8List? shown;
  if (profile.picture case final picture?) {
    shown = receivedProfilePicture(picture);
    if (shown == null) return null;
  }
  return (profile: profile, keys: keys, shown: shown);
}

/// [a] against [b] by their UTF-8 bytes, as the desktop core compares.
int _compareUtf8(String a, String b) {
  final x = utf8.encode(a), y = utf8.encode(b);
  for (var i = 0; i < x.length && i < y.length; i++) {
    if (x[i] != y[i]) return x[i] - y[i];
  }
  return x.length - y.length;
}

/// Whether the record [a] replaces [b]: a higher version, or at the same
/// version the larger bytes, so two Devices that changed it apart end up
/// with the same one.
bool _replaces(String a, Profile pa, String b, Profile pb) =>
    pa.version != pb.version ? pa.version > pb.version : _compareUtf8(a, b) > 0;

/// What became of a Profile another of the user's Devices sent.
enum ProfileTaken {
  /// Nothing new, or none sent.
  unchanged,

  /// Its signatures did not hold, or a copied id: nothing was kept.
  refused,

  /// A newer record, keys or aliases of the own Profile were kept.
  updated,

  /// This Device took the other Profile over.
  adopted,

  /// Both Profiles were changed by the user: "Which one is you?" waits.
  asked,
}

/// The Profile "Which one is you?" asks about: its record with the
/// picture made again, and its short code.
typedef ProfileChoice = ({Profile profile, String code});

// ------------------------------------------------------- own Profile

/// The user's own Profile on this Device (ADR 0017), kept as local
/// settings: `profile` holds the record, `profile:first` the public key
/// of the Device that made it (base64), and `profile:nameChosen` and
/// `profile:pictureChosen` say the user replaced a default. Since #48
/// `profile:signed` holds the record's signature and signer,
/// `profile:keys` the keys joined to the binding, `profile:aliases` the
/// ids it replaced, `profile:shown` a received picture made again, and
/// `profile:asked` the other Profile "Which one is you?" asks about.
extension OwnProfile on Store {
  static const _record = 'profile';
  static const _first = 'profile:first';
  static const _nameChosen = 'profile:nameChosen';
  static const _pictureChosen = 'profile:pictureChosen';
  static const _signed = 'profile:signed';
  static const _keys = 'profile:keys';
  static const _aliases = 'profile:aliases';
  static const _shown = 'profile:shown';
  static const _asked = 'profile:asked';

  /// The user's own Profile; null before [applyProfileDefaults] made it.
  /// A picture taken from another Device shows as made again here.
  Profile? get ownProfile {
    final text = localSetting(_record);
    final profile = text == null ? null : Profile.parse(text);
    final shown = localSetting(_shown);
    if (profile == null || shown == null || profile.picture == null) {
      return profile;
    }
    return Profile(
      id: profile.id,
      name: profile.name,
      picture: base64.decode(shown),
      version: profile.version,
    );
  }

  /// The first key of the own Profile: the signing key of the Device
  /// that made it.
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

  /// The ids the own Profile replaced, the kept Profile showing for each.
  Set<String> get ownProfileAliases => {for (final a in _heldAliases) a.alias};

  /// Makes the own Profile the first time, with a new id, the default
  /// [name] and [picture] (already a Profile picture, or null for the
  /// placeholder) and this Device's key as its first key; later it takes
  /// the defaults the user has not replaced, while no other Device holds
  /// the Profile. Asks nothing; counts the version up only when something
  /// changed.
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
      _keep(made);
      return made;
    }
    // A Profile held by several Devices keeps the name and picture it
    // had: each Device's own defaults would replace each other's.
    if (!_alone) return current;
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
    _keep(next);
    return next;
  }

  /// Gives the own Profile the [name] the user typed, sanitised.
  Profile renameOwnProfile(String name) {
    final current = _own;
    setLocalSetting(_nameChosen, 'true');
    final clean = sanitizeProfileName(name);
    if (clean == current.name) return current;
    final next = current._with(name: clean);
    _keep(next);
    return next;
  }

  /// Gives the own Profile the [picture] the user chose (a Profile
  /// picture), or the placeholder for null.
  Profile setOwnProfilePicture(Uint8List? picture) {
    final current = _own;
    setLocalSetting(_pictureChosen, 'true');
    if (_same(picture, current.picture)) return current;
    final next = current._with(picture: () => picture);
    _keep(next);
    return next;
  }

  Profile get _own =>
      ownProfile ??
      (throw StateError('no own Profile yet: applyProfileDefaults first'));

  /// Keeps [profile] as made here, signed with this Device's key.
  void _keep(Profile profile) {
    final text = profile.encode();
    setLocalSetting(_record, text);
    _sign(text);
    removeLocalSetting(_shown);
    profileChanged();
  }

  void _sign(String record) => setLocalSetting(
    _signed,
    jsonEncode({
      'sig': signingKey.sign(profileRecordBytes(record)),
      'signer': signingKey.publicBase64,
    }),
  );

  bool _chosen(String key) => localSetting(key) == 'true';

  bool get _fresh => !_chosen(_nameChosen) && !_chosen(_pictureChosen);

  /// Whether only this Device holds the own Profile: it made it, and no
  /// other key or id joined it.
  bool get _alone =>
      localSetting(_first) == signingKey.publicBase64 &&
      _heldKeys.isEmpty &&
      _heldAliases.isEmpty;

  List<ProfileKey> get _heldKeys => _list(_keys, ProfileKey.fromJson);

  List<ProfileAlias> get _heldAliases => _list(_aliases, ProfileAlias.fromJson);

  List<T> _list<T>(String key, T? Function(Object?) read) {
    final text = localSetting(key);
    if (text == null) return const [];
    try {
      final json = jsonDecode(text);
      return [
        if (json is List)
          for (final item in json) ?read(item),
      ];
    } on FormatException {
      return const [];
    }
  }

  void _keepKeys(List<ProfileKey> keys) => keys.isEmpty
      ? removeLocalSetting(_keys)
      : setLocalSetting(_keys, jsonEncode([for (final k in keys) k.toJson()]));

  void _keepAliases(Iterable<ProfileAlias> aliases) {
    final sorted = aliases.toList()
      ..sort((a, b) => _compareUtf8(a.alias, b.alias));
    sorted.isEmpty
        ? removeLocalSetting(_aliases)
        : setLocalSetting(
            _aliases,
            jsonEncode([for (final a in sorted) a.toJson()]),
          );
  }

  /// The own Profile as it goes to the user's other Devices: the record
  /// with its signature (made here when the record has none that holds),
  /// its binding and aliases, and whether it still has the defaults; null
  /// before it exists.
  SignedProfile? get ownSignedProfile {
    final record = localSetting(_record);
    final first = localSetting(_first);
    if (record == null || first == null || !_isKey(first)) return null;
    var signed = _signature;
    if (signed == null ||
        !verifySignature(
          base64.decode(signed.signer),
          profileRecordBytes(record),
          signed.sig,
        )) {
      _sign(record);
      signed = _signature;
    }
    if (signed == null) return null;
    return SignedProfile(
      record: record,
      signer: signed.signer,
      sig: signed.sig,
      first: first,
      keys: _heldKeys,
      aliases: _heldAliases,
      fresh: _fresh,
    );
  }

  /// [ownSignedProfile] as JSON, for the sync bodies and the keys file.
  Map<String, Object?>? get ownProfileJson => ownSignedProfile?.toJson();

  ({String sig, String signer})? get _signature {
    final text = localSetting(_signed);
    if (text == null) return null;
    try {
      if (jsonDecode(text)
          case {'sig': final String sig, 'signer': final String signer}
          when _isKey(signer)) {
        return (sig: sig, signer: signer);
      }
    } on FormatException {
      // Read as none: the record is signed again.
    }
    return null;
  }

  /// Binds [key] (base64), the key of another of the user's own Devices
  /// met in a sync, to the own Profile, signed with this Device's key
  /// when that is bound: the user's own Devices join the binding through
  /// Pairing.
  void _bind(String key) {
    final record = localSetting(_record);
    final first = localSetting(_first);
    final profile = record == null ? null : Profile.parse(record);
    final me = signingKey.publicBase64;
    if (profile == null || first == null || key == me || !_isKey(key)) {
      return;
    }
    final (bound, keys) = _chain(profile.id, first, _heldKeys);
    if (!bound.contains(me) || bound.contains(key)) return;
    _keepKeys(
      [
        ...keys,
        ProfileKey(
          key: key,
          by: me,
          sig: signingKey.sign(profileKeyBytes(profile.id, key)),
        ),
      ]..sort((a, b) => a.key.compareTo(b.key)),
    );
  }

  /// Takes in the Profile [sent] by another of the user's own Devices,
  /// whose signing key [from] the session or the keys file vouches for.
  /// That Device's key is bound to the own Profile first. Then: the same
  /// Profile, newer and validly signed, replaces the record kept; an
  /// update signed by a key not bound to it is refused. Another Profile
  /// is taken over when this Device's still has the defaults and the
  /// other not, when both have them and the other's id is lower, or when
  /// it names this one's id as its alias; when both were changed by the
  /// user, "Which one is you?" waits ([profileQuestion]).
  ProfileTaken takeOwnProfile(Object? sent, {required List<int> from}) {
    _bind(base64.encode(from));
    if (sent == null) return ProfileTaken.unchanged;
    final theirs = SignedProfile.fromJson(sent);
    final checked = theirs == null ? null : _check(theirs);
    if (theirs == null || checked == null) return ProfileTaken.refused;
    final taken = _take(theirs, checked);
    if (taken != ProfileTaken.unchanged && taken != ProfileTaken.refused) {
      profileChanged();
    }
    return taken;
  }

  ProfileTaken _take(SignedProfile theirs, _Checked checked) {
    final record = localSetting(_record);
    final first = localSetting(_first);
    final mine = record == null ? null : Profile.parse(record);
    if (record == null || first == null || mine == null) {
      _adopt(theirs, checked, null);
      return ProfileTaken.adopted;
    }
    final id = checked.profile.id;
    if (id == mine.id) return _merge(theirs, checked, mine, record, first);
    final myBound = _chain(mine.id, first, _heldKeys).$1;
    final aliasesMe = theirs.aliases.any(
      (a) =>
          a.alias == mine.id &&
          myBound.contains(a.by) &&
          a.signed &&
          _reaches(a.to, id, theirs.aliases),
    );
    if (ownProfileAliases.contains(id)) {
      // Given up here, and this one given up there: answered apart, the
      // lower id stays.
      if (aliasesMe && _compareUtf8(id, mine.id) < 0) {
        _adopt(theirs, checked, mine);
        return ProfileTaken.adopted;
      }
      return ProfileTaken.unchanged;
    }
    final fresh = _fresh;
    if (aliasesMe ||
        (fresh && !theirs.fresh) ||
        (fresh && theirs.fresh && _compareUtf8(id, mine.id) < 0)) {
      _adopt(theirs, checked, mine);
      return ProfileTaken.adopted;
    }
    if (fresh || theirs.fresh) return ProfileTaken.unchanged;
    final text = jsonEncode(theirs.toJson());
    if (localSetting(_asked) == text) return ProfileTaken.unchanged;
    setLocalSetting(_asked, text);
    return ProfileTaken.asked;
  }

  /// Whether the alias chain [aliases] leads from [to] to [id].
  bool _reaches(String to, String id, List<ProfileAlias> aliases) {
    var at = to;
    for (var i = 0; i <= aliases.length; i++) {
      if (at == id) return true;
      final next = aliases.where((a) => a.alias == at).firstOrNull;
      if (next == null) return false;
      at = next.to;
    }
    return false;
  }

  ProfileTaken _merge(
    SignedProfile theirs,
    _Checked checked,
    Profile mine,
    String record,
    String first,
  ) {
    // A copied id with another first key is another person's Profile.
    if (theirs.first != first) return ProfileTaken.refused;
    final held = _heldKeys;
    final (bound, keys) = _chain(mine.id, first, theirs.keys, held: held);
    if (!bound.contains(theirs.signer)) return ProfileTaken.refused;
    var taken = ProfileTaken.unchanged;
    if (jsonEncode([for (final k in keys) k.toJson()]) !=
        jsonEncode([for (final k in held) k.toJson()])) {
      _keepKeys(keys);
    }
    final aliases = {for (final a in _heldAliases) a.alias: a};
    var more = false;
    for (final a in theirs.aliases) {
      if (aliases.containsKey(a.alias) || a.alias == mine.id) continue;
      if (!bound.contains(a.by) || !a.signed) continue;
      aliases[a.alias] = a;
      more = true;
    }
    if (more) {
      _keepAliases(aliases.values);
      taken = ProfileTaken.updated;
    }
    final asked = SignedProfile.fromJson(_askedJson);
    if (asked != null &&
        aliases.containsKey(Profile.parse(asked.record)?.id ?? '')) {
      removeLocalSetting(_asked);
      taken = ProfileTaken.updated;
    }
    if (!_replaces(theirs.record, checked.profile, record, mine)) return taken;
    // A change made on another Device was the user's: the defaults no
    // longer apply to it here either.
    if (!theirs.fresh) {
      if (checked.profile.name != mine.name) {
        setLocalSetting(_nameChosen, 'true');
      }
      if (!_same(checked.profile.picture, mine.picture)) {
        setLocalSetting(_pictureChosen, 'true');
      }
    }
    _keepReceived(theirs, checked);
    return ProfileTaken.updated;
  }

  /// Keeps [theirs] record as received, with its signature and its
  /// picture made again.
  void _keepReceived(SignedProfile theirs, _Checked checked) {
    setLocalSetting(_record, theirs.record);
    setLocalSetting(
      _signed,
      jsonEncode({'sig': theirs.sig, 'signer': theirs.signer}),
    );
    if (checked.shown case final shown?) {
      setLocalSetting(_shown, base64.encode(shown));
    } else {
      removeLocalSetting(_shown);
    }
  }

  /// Takes [theirs] over as the own Profile; the id of [mine], when there
  /// was one, becomes its alias, signed with this Device's key.
  void _adopt(SignedProfile theirs, _Checked checked, Profile? mine) {
    final id = checked.profile.id;
    final aliases = <String, ProfileAlias>{};
    for (final a in [..._heldAliases, ...theirs.aliases]) {
      if (a.alias != id && a.signed) aliases.putIfAbsent(a.alias, () => a);
    }
    if (mine != null && mine.id != id && !aliases.containsKey(mine.id)) {
      aliases[mine.id] = ProfileAlias(
        alias: mine.id,
        to: id,
        by: signingKey.publicBase64,
        sig: signingKey.sign(profileAliasBytes(mine.id, id)),
      );
    }
    _keepReceived(theirs, checked);
    setLocalSetting(_first, theirs.first);
    _keepKeys(checked.keys);
    _keepAliases(aliases.values);
    if (theirs.fresh) {
      removeLocalSetting(_nameChosen);
      removeLocalSetting(_pictureChosen);
    } else {
      setLocalSetting(_nameChosen, 'true');
      setLocalSetting(_pictureChosen, 'true');
    }
    removeLocalSetting(_asked);
  }

  Object? get _askedJson {
    final text = localSetting(_asked);
    if (text == null) return null;
    try {
      return jsonDecode(text);
    } on FormatException {
      return null;
    }
  }

  /// The other Profile "Which one is you?" asks about, or null when
  /// nothing is asked.
  ProfileChoice? get profileQuestion {
    final asked = SignedProfile.fromJson(_askedJson);
    final checked = asked == null ? null : _check(asked);
    if (asked == null || checked == null) return null;
    final p = checked.profile;
    return (
      profile: Profile(
        id: p.id,
        name: p.name,
        picture: checked.shown,
        version: p.version,
      ),
      code: profileShortCode(p.id, base64.decode(asked.first)),
    );
  }

  /// Answers "Which one is you?": [keepMine] keeps this Device's Profile
  /// and makes the other's id its alias, signed with this Device's key;
  /// otherwise this Device takes the other over. The answer reaches the
  /// other Devices with the next sync.
  void answerProfileQuestion({required bool keepMine}) {
    final asked = SignedProfile.fromJson(_askedJson);
    final checked = asked == null ? null : _check(asked);
    final record = localSetting(_record);
    final mine = record == null ? null : Profile.parse(record);
    removeLocalSetting(_asked);
    if (asked != null && checked != null && mine != null) {
      final id = checked.profile.id;
      if (!keepMine) {
        _adopt(asked, checked, mine);
      } else if (id != mine.id) {
        final aliases = {for (final a in _heldAliases) a.alias: a};
        aliases.putIfAbsent(
          id,
          () => ProfileAlias(
            alias: id,
            to: mine.id,
            by: signingKey.publicBase64,
            sig: signingKey.sign(profileAliasBytes(id, mine.id)),
          ),
        );
        _keepAliases(aliases.values);
      }
    }
    profileChanged();
  }
}

bool _same(Uint8List? a, Uint8List? b) {
  if (a == null || b == null) return a == b;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
