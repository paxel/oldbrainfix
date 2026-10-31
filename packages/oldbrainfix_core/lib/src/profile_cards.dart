part of 'profiles.dart';

/// Profile cards (ADR 0017, #107, #144): a person sends their own Profile
/// as a small file through any messenger, or shows it as a QR, and
/// whoever opens it has them in People. The card is
/// `<payload>.<signature>`, both base64url without padding; the QR holds
/// it after the `#` of [profileCardLink], which never reaches a server.
/// The payload is one JSON object, keys sorted and no spaces, the same
/// bytes from both cores:
/// `{"back":true,"format":1,"key":<key record>,"profile":<signed Profile>}`.
/// `key` is the key record of the Device that made the card (ADR 0015),
/// `profile` the own Profile as it goes to other people with no aliases,
/// its record signed again by that Device: the record as kept in a file,
/// without its picture in the QR; `back` is there when the card answers
/// one the user opened. The signature is that Device's, over
/// [profileCardBytes] of the payload as sent, and its key must be bound to
/// the Profile. The file ([profileCardFile]) is
/// `{"card":<card>,"type":"oldbrainfix-card"}`, keys sorted and no spaces.
/// A card adds a person and nothing else: no Area, no grant, no key but
/// the public ones.

/// The address a QR's card goes out under; the card follows the `#`.
const profileCardLink = 'https://paxel.github.io/oldbrainfix/p#';

/// The most characters of a card's text that are read at all.
const profileCardMaxChars = 16 * 1024;

/// The file ending of a Profile card file.
const profileCardExtension = 'obfcard';

/// The type a Profile card file is shared as.
const profileCardMimeType = 'application/vnd.oldbrainfix.card';

/// A file larger than this is no Profile card: a picture received is at
/// most 256 KB.
const maxProfileCardFileBytes = 1 << 20;

const _cardFormat = 1;

/// What a Profile card file says it is.
const _cardFileType = 'oldbrainfix-card';

/// The characters no file name may hold on any of the apps' systems.
final _notInFileNames = RegExp(r'[/\\:*?"<>|]');

/// The names Windows keeps for its devices, with any ending.
final _reservedFileNames = RegExp(
  r'^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$',
  caseSensitive: false,
);

/// The name the Profile card file of a person called [name] goes by,
/// `Ann.obfcard`: the name sanitised as a Profile's, each character a
/// file name cannot hold made `_`, without blanks and full stops at
/// either end, `_` before a name Windows keeps for a device, and
/// `Profile` for a name with nothing left.
String profileCardFileName(String name) {
  var base = sanitizeProfileName(name)
      .replaceAll(_notInFileNames, '_')
      .replaceAll(RegExp(r'^[ .]+|[ .]+$'), '');
  if (base.isEmpty) base = 'Profile';
  final stem = base.split('.').first.replaceAll(RegExp(r' +$'), '');
  if (_reservedFileNames.hasMatch(stem)) base = '_$base';
  return '$base.$profileCardExtension';
}

/// The JSON object of the Profile card file [bytes]: one that says it is
/// one, of any format; null for anything else or one too large.
Map<String, Object?>? _cardFileJson(List<int> bytes) {
  if (bytes.length > maxProfileCardFileBytes) return null;
  final Object? json;
  try {
    json = jsonDecode(utf8.decode(bytes));
  } on FormatException {
    return null;
  }
  return json is Map<String, Object?> && json['type'] == _cardFileType
      ? json
      : null;
}

/// Whether [bytes] are a Profile card file rather than a Sent Item or a
/// bundle, in a format this version reads or a later one.
bool looksLikeProfileCard(List<int> bytes) => _cardFileJson(bytes) != null;

final _base64Url = RegExp(r'^[A-Za-z0-9_-]+$');
final _cardRun = RegExp(r'^[A-Za-z0-9_.-]*');

/// The bytes a card's signature covers: the payload as sent.
Uint8List profileCardBytes(String payload) =>
    utf8.encode('oldbrainfix:profile-card:$payload');

String _toBase64Url(List<int> bytes) =>
    base64Url.encode(bytes).replaceAll('=', '');

/// The bytes of [text], base64url without padding as written; null for
/// anything else.
Uint8List? _fromBase64Url(String text) {
  if (!_base64Url.hasMatch(text)) return null;
  try {
    final bytes = base64Url.decode(base64Url.normalize(text));
    return _toBase64Url(bytes) == text ? bytes : null;
  } on FormatException {
    return null;
  }
}

/// What opening a card did.
enum CardTaken {
  /// The person was not kept here before and is now, not verified.
  added,

  /// The person was kept here; the card brought something new.
  updated,

  /// The person was kept here as the card has them.
  unchanged,

  /// The card is the user's own Profile: nothing was kept.
  own,

  /// The person is blocked, or a Device of theirs made the card.
  blocked,

  /// The card names a Profile kept here under another first key, or one
  /// its signer is not bound to: somebody else.
  refused,

  /// No card, or its signatures do not hold.
  unreadable,

  /// A Profile card file of a format a later version writes (ADR 0034):
  /// this app needs an update to open it.
  needsUpdate,
}

/// What opening a card did, the Profile id it names (null when
/// unreadable), and whether to offer "Send yours back": the person was
/// new here and the card answers none of the user's.
typedef CardOpened = ({CardTaken taken, String? id, bool offerBack});

/// A card whose signatures held.
typedef _Card = ({SignedProfile profile, KeyRecord key, bool back, String id});

/// The payload and signature of the card in [text]: the link, or the
/// card alone. Everything but printable ASCII is left out first, so the
/// blanks and line breaks a messenger puts in do no harm; then the card
/// is what follows the last `#` up to the first character no card has,
/// without a full stop after it.
(String, String)? _cardParts(String text) {
  final ascii = String.fromCharCodes(
    text.codeUnits.where((c) => c > 0x20 && c < 0x7F),
  );
  final after = ascii.substring(ascii.lastIndexOf('#') + 1);
  var run = _cardRun.stringMatch(after) ?? '';
  while (run.endsWith('.')) {
    run = run.substring(0, run.length - 1);
  }
  if (run.length > profileCardMaxChars) return null;
  final parts = run.split('.');
  if (parts.length != 2) return null;
  return (parts[0], parts[1]);
}

/// The card [text] holds ([_cardParts]), checked ([_checkCard]).
_Card? _readCard(String text) {
  final parts = _cardParts(text);
  return parts == null ? null : _checkCard(parts.$1, parts.$2);
}

/// The card of [payloadText] and [sigText], checked: its key record valid
/// and its signature over the payload as sent, before anything else of
/// it is read; then its Profile's signatures, and its key bound to that
/// Profile. Null when any of it fails.
_Card? _checkCard(String payloadText, String sigText) {
  final payloadBytes = _fromBase64Url(payloadText);
  final sigBytes = _fromBase64Url(sigText);
  if (payloadBytes == null || sigBytes == null) return null;
  final String payload;
  final Object? json;
  try {
    payload = utf8.decode(payloadBytes);
    json = jsonDecode(payload);
  } on FormatException {
    return null;
  }
  if (json
      case {
        'format': final int format,
        'key': final Object keyJson,
        'profile': final Object profileJson,
      }
      when format == _cardFormat) {
    final back = json['back'];
    if (back != null && back != true) return null;
    final key = KeyRecord.fromJson(keyJson);
    if (key == null || !key.valid) return null;
    if (!verifySignature(
      key.publicKey,
      profileCardBytes(payload),
      base64.encode(sigBytes),
    )) {
      return null;
    }
    final profile = SignedProfile.fromJson(profileJson);
    final checked = profile == null ? null : _check(profile);
    if (profile == null || checked == null) return null;
    final id = checked.profile.id;
    if (!_chain(
      id,
      profile.first,
      profile.keys,
    ).$1.contains(base64.encode(key.publicKey))) {
      return null;
    }
    return (profile: profile, key: key, back: back == true, id: id);
  }
  return null;
}

extension ProfileCards on Store {
  /// The own Profile as a card's link, for the QR: the record without its
  /// picture, signed by this Device, with the binding and this Device's
  /// key record, the whole signed by this Device; with [back] when it
  /// answers a card the user opened. Null before the own Profile exists,
  /// or while this Device's key is not bound to it yet.
  String? profileCard({bool back = false}) {
    final card = _card(back: back, picture: false);
    return card == null ? null : '$profileCardLink$card';
  }

  /// The own Profile as a card file, `<Name>.obfcard`
  /// ([profileCardFileName]): the card as [profileCard] makes it, but
  /// with the record as kept, picture and all, so the person shows with
  /// it at once. Null when [profileCard] is.
  Uint8List? profileCardFile({bool back = false}) {
    final card = _card(back: back, picture: true);
    return card == null
        ? null
        : utf8.encode(jsonEncode({'card': card, 'type': _cardFileType}));
  }

  /// The own card, `<payload>.<signature>`, with the record as kept when
  /// [picture], else without its picture.
  String? _card({required bool back, required bool picture}) {
    final own = ownSignedProfile;
    final record = own == null ? null : Profile.parse(own.record);
    if (own == null || record == null) return null;
    final me = signingKey.publicBase64;
    if (!_chain(record.id, own.first, own.keys).$1.contains(me)) return null;
    final sent = picture
        ? own.record
        : Profile(
            id: record.id,
            name: record.name,
            version: record.version,
          ).encode();
    final payload = jsonEncode({
      if (back) 'back': true,
      'format': _cardFormat,
      'key': ownKeyRecord.toJson(),
      'profile': SignedProfile(
        record: sent,
        signer: me,
        sig: signingKey.sign(profileRecordBytes(sent)),
        first: own.first,
        keys: own.keys,
      ).toJson(),
    });
    final sig = base64.decode(signingKey.sign(profileCardBytes(payload)));
    return '${_toBase64Url(utf8.encode(payload))}.${_toBase64Url(sig)}';
  }

  /// Opens the card [text] (a link tapped, scanned or pasted): the
  /// person it names is kept as another person's Profile is (ADR 0017),
  /// not verified, and the key of the Device that made it is pinned, so
  /// Sent Items to them are sealed for it. Scanned [inPerson], their
  /// short code is the one the card binds and the person is verified at
  /// once. The user's own card, a blocked person's, one a Device of a
  /// blocked person made, and one that names a Profile kept here under
  /// another first key keep nothing.
  CardOpened openProfileCard(String text, {bool inPerson = false}) =>
      _takeCard(_readCard(text), inPerson: inPerson);

  /// Opens the Profile card file [bytes] (a file tapped, opened or
  /// dropped), as [openProfileCard] opens a link: a file that is none,
  /// is larger than [maxProfileCardFileBytes] or holds a card that is
  /// changed or not bound is unreadable, one of a later format
  /// ([CardTaken.needsUpdate]) is not read further. Only an absent
  /// `format` or the whole number 1 is this version's (ADR 0034). A file
  /// is never in person: the person stays not verified.
  CardOpened openProfileCardFile(List<int> bytes) {
    final json = _cardFileJson(bytes);
    // `1.0 == 1` in Dart: only the whole number 1 is version 1.
    final format = json?['format'];
    if (json != null &&
        json.containsKey('format') &&
        !(format is int && format == 1)) {
      return (taken: CardTaken.needsUpdate, id: null, offerBack: false);
    }
    final parts = switch (json?['card']) {
      final String card => card.split('.'),
      _ => const <String>[],
    };
    return _takeCard(
      parts.length == 2 ? _checkCard(parts[0], parts[1]) : null,
      inPerson: false,
    );
  }

  /// Keeps the person of [card], checked, as [openProfileCard] says.
  CardOpened _takeCard(_Card? card, {required bool inPerson}) {
    if (card == null) {
      return (taken: CardTaken.unreadable, id: null, offerBack: false);
    }
    final id = card.id;
    if (isOwnPerson(id)) {
      return (taken: CardTaken.own, id: id, offerBack: false);
    }
    if (isBlocked(id) || blockedDevices().contains(card.key.device)) {
      return (taken: CardTaken.blocked, id: id, offerBack: false);
    }
    final taken = switch (takePerson(card.profile.toJson())) {
      PersonTaken.added => CardTaken.added,
      PersonTaken.updated => CardTaken.updated,
      PersonTaken.unchanged => CardTaken.unchanged,
      PersonTaken.refused => CardTaken.refused,
    };
    if (taken == CardTaken.refused) {
      return (taken: taken, id: id, offerBack: false);
    }
    learnKeys([card.key]);
    if (inPerson) {
      verifyPerson(id, profileShortCode(id, base64.decode(card.profile.first)));
    }
    profileChanged();
    return (
      taken: taken,
      id: id,
      offerBack: taken == CardTaken.added && !card.back,
    );
  }
}
