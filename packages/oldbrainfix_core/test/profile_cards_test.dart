import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device, forged;
import 'profiles_test.dart' show halves;
import 'shared_areas_test.dart' show exchange, ticking;

String _b64(List<int> bytes) => base64Url.encode(bytes).replaceAll('=', '');

/// The payload of the card [link], as signed.
String payloadOf(String link) {
  final body = link.substring(link.indexOf('#') + 1).split('.').first;
  return utf8.decode(base64Url.decode(base64Url.normalize(body)));
}

/// The signature part of the card [link].
String sigOf(String link) => link.split('.').last;

/// A card made by hand: [payload] signed by [key].
String cardOf(SigningKey key, Object? payload) {
  final text = payload is String ? payload : jsonEncode(payload);
  final sig = base64.decode(key.sign(profileCardBytes(text)));
  return '$profileCardLink${_b64(utf8.encode(text))}.${_b64(sig)}';
}

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    ben = device(clock, 'Ben')..renameOwnProfile('Ben');
  });

  String idOf(Store store) => store.ownProfile!.id;

  group('opening a card', () {
    test('a link adds the person, not verified, and offers to send one '
        'back; the answer offers none', () {
      final opened = ben.openProfileCard(ann.profileCard()!);
      expect(opened.taken, CardTaken.added);
      expect(opened.id, idOf(ann));
      expect(opened.offerBack, isTrue);
      expect(ben.personShown(idOf(ann)).name, 'Ann');
      expect(ben.personCode(idOf(ann)), ann.ownProfileCode);
      expect(ben.personVerified(idOf(ann)), isFalse);

      final back = ann.openProfileCard(ben.profileCard(back: true)!);
      expect(back.taken, CardTaken.added);
      expect(back.offerBack, isFalse);
      expect(ann.personShown(idOf(ben)).name, 'Ben');

      final again = ben.openProfileCard(ann.profileCard()!);
      expect(again.taken, CardTaken.unchanged);
      expect(again.offerBack, isFalse);
    });

    test('scanned in person, the person is verified at once', () {
      final opened = ben.openProfileCard(ann.profileCard()!, inPerson: true);
      expect(opened.taken, CardTaken.added);
      expect(ben.personVerified(idOf(ann)), isTrue);
    });

    test('blanks, text around the link and the card alone still open', () {
      final link = ann.profileCard()!;
      final wrapped = StringBuffer('Here is my card:\n');
      for (var i = 0; i < link.length; i += 30) {
        wrapped.writeln(link.substring(i, (i + 30).clamp(0, link.length)));
      }
      expect(
        ben.openProfileCard('$wrapped\u200B(see you).').taken,
        CardTaken.added,
      );
      final bare = link.substring(link.indexOf('#') + 1);
      expect(ben.openProfileCard(' $bare ').taken, CardTaken.unchanged);
    });

    test('the user\'s own card keeps nothing', () {
      final opened = ann.openProfileCard(ann.profileCard()!);
      expect(opened.taken, CardTaken.own);
      expect(ann.keptPeople, isEmpty);
    });

    test('a newer card brings the new name', () {
      ben.openProfileCard(ann.profileCard()!);
      ann.renameOwnProfile('Ann B.');
      expect(ben.openProfileCard(ann.profileCard()!).taken, CardTaken.updated);
      expect(ben.personShown(idOf(ann)).name, 'Ann B.');
    });
  });

  group('the picture', () {
    test('stays out of the card and comes with their Profile at the same '
        'version', () {
      final picture = squareProfilePicture(halves(64, 64))!;
      ann.setOwnProfilePicture(picture);
      final link = ann.profileCard()!;
      expect(payloadOf(link), isNot(contains('picture')));
      ben.openProfileCard(link);
      expect(ben.personShown(idOf(ann)).picture, isNull);

      expect(ben.takePerson(ann.ownProfileForOthers), PersonTaken.updated);
      expect(ben.personShown(idOf(ann)).picture, isNotNull);
      // The card again changes nothing: the record with the picture wins
      // at the same version.
      expect(ben.openProfileCard(link).taken, CardTaken.unchanged);
      expect(ben.personShown(idOf(ann)).picture, isNotNull);
    });
  });

  group('refused', () {
    test('a card changed after it was signed is unreadable', () {
      final link = ann.profileCard()!;
      final payload = jsonDecode(payloadOf(link)) as Map<String, Object?>;
      // Only the card's own signature covers the answer flag.
      final flagged = jsonEncode({'back': true, ...payload});
      expect(
        ben.openProfileCard(
          '$profileCardLink${_b64(utf8.encode(flagged))}.${sigOf(link)}',
        ),
        (taken: CardTaken.unreadable, id: null, offerBack: false),
      );
      final sig = sigOf(link);
      final flipped = '${sig[0] == 'A' ? 'B' : 'A'}${sig.substring(1)}';
      expect(
        ben.openProfileCard('${link.split('.').first}.$flipped').taken,
        CardTaken.unreadable,
      );
      expect(
        ben.openProfileCard(link.substring(0, link.length - 3)).taken,
        CardTaken.unreadable,
      );
      expect(ben.openProfileCard(profileCardLink).taken, CardTaken.unreadable);
      expect(ben.openProfileCard('hello').taken, CardTaken.unreadable);
      // Another format, signed as it should be, is not read.
      expect(
        ben
            .openProfileCard(cardOf(ann.signingKey, {...payload, 'format': 2}))
            .taken,
        CardTaken.unreadable,
      );
      expect(ben.keptPeople, isEmpty);
    });

    test('a card signed by a key its Profile does not bind is unreadable', () {
      final eve = device(clock, 'Eve');
      final payload = jsonDecode(payloadOf(ann.profileCard()!)) as Map;
      final signedByEve = cardOf(eve.signingKey, {
        ...payload,
        'key': eve.ownKeyRecord.toJson(),
      });
      expect(ben.openProfileCard(signedByEve).taken, CardTaken.unreadable);
      expect(ben.keptPeople, isEmpty);
    });

    test('a card naming a kept id under another first key is somebody '
        'else, and verifies nobody', () {
      ben.openProfileCard(ann.profileCard()!);
      final eve = device(clock, 'Eve');
      final copied = cardOf(eve.signingKey, {
        'format': 1,
        'key': eve.ownKeyRecord.toJson(),
        'profile': forged(eve.signingKey, {
          'id': idOf(ann),
          'name': 'Ann',
          'version': 9,
        }),
      });
      final opened = ben.openProfileCard(copied, inPerson: true);
      expect(opened.taken, CardTaken.refused);
      expect(ben.personVerified(idOf(ann)), isFalse);
      expect(ben.personCode(idOf(ann)), ann.ownProfileCode);
      expect(ben.pinnedKey(eve.deviceId), isNull);
    });

    test('a blocked person\'s card is refused', () {
      ben.openProfileCard(ann.profileCard()!);
      ben.blockPerson(idOf(ann));
      final opened = ben.openProfileCard(ann.profileCard()!);
      expect(opened.taken, CardTaken.blocked);
      expect(ben.isBlocked(idOf(ann)), isTrue);
      expect(ben.personVerified(idOf(ann)), isFalse);
    });
  });

  test('a card a blocked person\'s Device made under a new id is refused', () {
    final carol = device(clock, 'Carol')..renameOwnProfile('Carol');
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(carol, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(carol, ann);
    carol.blockPerson(idOf(ben));
    final renamed = cardOf(ben.signingKey, {
      'format': 1,
      'key': ben.ownKeyRecord.toJson(),
      'profile': forged(ben.signingKey, {
        'id': '0123456789abcdef0123456789abcdef',
        'name': 'Not Ben',
        'version': 1,
      }),
    });
    expect(carol.openProfileCard(renamed).taken, CardTaken.blocked);
    expect(carol.personCode('0123456789abcdef0123456789abcdef'), isNull);
  });

  group('what a card shares', () {
    test('the Profile and the Device\'s public keys, nothing else', () {
      final payload = jsonDecode(payloadOf(ann.profileCard()!)) as Map;
      expect(payload.keys, ['format', 'key', 'profile']);
      expect(payload['key'], ann.ownKeyRecord.toJson());
      final profile = payload['profile'] as Map;
      expect(profile.keys, [
        'aliases',
        'first',
        'keys',
        'record',
        'sig',
        'signer',
      ]);
      ben.openProfileCard(ann.profileCard()!);
      expect(ben.pairedDevices(), isEmpty);
      expect(ben.isOwnDevice(ann.deviceId), isFalse);
      expect(ben.judged.areas, isEmpty);
    });

    test('a Sent Item to a person added from a card is sealed for their '
        'Device, and theirs previews as known', () {
      expect(sentItemRecipients(ben), isEmpty);
      ben.openProfileCard(ann.profileCard()!);
      final recipients = sentItemRecipients(ben);
      expect([for (final r in recipients) r.name], ['Ann']);
      expect(recipients.single.keys.single, ann.ownKeyRecord.boxKey);
      final note = Notes(ben).capture('For Ann');
      final opened = openSentItem(
        ann,
        sendNote(ben, note.id, to: recipients.single),
      );
      expect(opened.encrypted, isTrue);
      expect(opened.trust, SenderTrust.unknown);

      final mine = Notes(ann).capture('For Ben');
      expect(
        openSentItem(ben, sendNote(ann, mine.id)).trust,
        SenderTrust.known,
      );
      ben.blockPerson(idOf(ann));
      expect(sentItemRecipients(ben), isEmpty);
    });

    test('a one-Device Profile with the longest name fits a QR', () {
      ann.renameOwnProfile('\u{1F600}' * 50);
      final link = ann.profileCard()!;
      expect(link, startsWith('https://paxel.github.io/oldbrainfix/p#'));
      expect(link.length, lessThanOrEqualTo(1280));
    });

    test('each further Device of the Profile makes the link longer, and '
        'the card from either opens', () {
      final desk = device(clock, 'Ann at the desk');
      final laptop = device(clock, 'Ann on the go');
      exchange(desk, ann);
      exchange(laptop, ann);
      exchange(desk, ann);
      ann.renameOwnProfile('\u{1F600}' * 50);
      exchange(desk, ann);
      final link = desk.profileCard()!;
      expect(link.length, lessThanOrEqualTo(1830));
      expect(ben.openProfileCard(link).taken, CardTaken.added);
      expect(
        ben.openProfileCard(ann.profileCard()!).taken,
        CardTaken.unchanged,
      );
    });

    test('no card before the own Profile exists', () {
      final fresh = Store.inMemory(clock: clock);
      addTearDown(fresh.close);
      expect(fresh.profileCard(), isNull);
    });
  });
}
