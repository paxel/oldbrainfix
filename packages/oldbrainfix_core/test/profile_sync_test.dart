import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'profiles_test.dart' show halves;
import 'shared_areas_test.dart' show exchange, ticking;

/// A Device with its own Profile made from the default [name].
Store device(DateTime Function() clock, String name) {
  final store = Store.inMemory(clock: clock);
  addTearDown(store.close);
  store.applyProfileDefaults(name: name);
  return store;
}

/// The Profile [store] holds, as both Devices must agree on it.
(String?, int?, String?) held(Store store) =>
    (store.ownProfile?.id, store.ownProfile?.version, store.ownProfileCode);

void main() {
  group('two Devices paired as one\'s own', () {
    test('both with the defaults: the lower id stays on both, the other '
        'becomes its alias', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Ada Lovelace');
      final ids = [phone.ownProfile!.id, desk.ownProfile!.id]..sort();
      final done = exchange(phone, desk);
      expect(done.request['profile'], isNotNull);
      expect(done.answer['profile'], isNotNull);
      expect(phone.ownProfile!.id, ids.first);
      expect(held(phone), held(desk));
      // The Device that gave its id up holds the alias at once, the other
      // one after the next sync at the latest.
      exchange(phone, desk);
      for (final store in [phone, desk]) {
        expect(store.ownProfileAliases, {ids.last});
      }
      expect(phone.profileQuestion, isNull);
      expect(desk.profileQuestion, isNull);
      // Nothing more changes when they meet again.
      exchange(phone, desk);
      expect(held(phone), held(desk));
      expect(phone.ownProfileAliases, desk.ownProfileAliases);
    });

    test('a Device with the defaults takes over a changed Profile without '
        'a question, as joiner or as host', () {
      for (final joinerChose in [true, false]) {
        final clock = ticking();
        final phone = device(clock, 'Pixel 7');
        final desk = device(clock, 'Ada Lovelace');
        final chosen = joinerChose ? phone : desk;
        chosen.renameOwnProfile('Ada');
        final kept = chosen.ownProfile!.id;
        final given = (joinerChose ? desk : phone).ownProfile!.id;
        final done = exchange(phone, desk);
        expect(phone.ownProfile!.id, kept);
        expect(phone.ownProfile!.name, 'Ada');
        expect(held(phone), held(desk));
        expect(phone.profileQuestion, isNull);
        expect(desk.profileQuestion, isNull);
        expect(
          [done.host.profile, done.joiner.profile],
          joinerChose
              ? [ProfileTaken.adopted, ProfileTaken.updated]
              : [ProfileTaken.unchanged, ProfileTaken.adopted],
        );
        exchange(phone, desk);
        expect(phone.ownProfileAliases, {given});
        expect(desk.ownProfileAliases, {given});
      }
    });

    test('both changed: "Which one is you?" waits on both, and the first '
        'answer settles it on both', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7')..renameOwnProfile('Ada L.');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      final phoneId = phone.ownProfile!.id, deskId = desk.ownProfile!.id;
      final done = exchange(phone, desk);
      expect(done.host.profile, ProfileTaken.asked);
      expect(done.joiner.profile, ProfileTaken.asked);
      expect(phone.profileQuestion!.profile.name, 'Ada');
      expect(phone.profileQuestion!.code, desk.ownProfileCode);
      expect(desk.profileQuestion!.profile.name, 'Ada L.');
      // Nothing is taken over while the question waits.
      expect(phone.ownProfile!.id, phoneId);
      expect(desk.ownProfile!.id, deskId);

      // On the phone: the computer's Profile is me.
      phone.answerProfileQuestion(keepMine: false);
      expect(phone.profileQuestion, isNull);
      expect(phone.ownProfile!.id, deskId);
      expect(phone.ownProfile!.name, 'Ada');
      exchange(phone, desk);
      expect(desk.profileQuestion, isNull, reason: 'answered on the phone');
      expect(held(phone), held(desk));
      expect(desk.ownProfileAliases, {phoneId});
      expect(phone.ownProfileAliases, {phoneId});
    });

    test('kept on the host, the joiner takes it over at the next sync', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7')..renameOwnProfile('Ada L.');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      final phoneId = phone.ownProfile!.id, deskId = desk.ownProfile!.id;
      exchange(phone, desk);
      desk.answerProfileQuestion(keepMine: true);
      expect(desk.ownProfile!.id, deskId);
      expect(desk.ownProfileAliases, {phoneId});
      final done = exchange(phone, desk);
      expect(done.joiner.profile, ProfileTaken.adopted);
      expect(phone.profileQuestion, isNull);
      expect(held(phone), held(desk));
      expect(phone.ownProfile!.name, 'Ada');
    });

    test('answered apart and against each other, the lower id stays on '
        'both', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7')..renameOwnProfile('Ada L.');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      final ids = [phone.ownProfile!.id, desk.ownProfile!.id]..sort();
      exchange(phone, desk);
      phone.answerProfileQuestion(keepMine: true);
      desk.answerProfileQuestion(keepMine: true);
      exchange(phone, desk);
      exchange(phone, desk);
      expect(phone.ownProfile!.id, ids.first);
      expect(held(phone), held(desk));
      exchange(phone, desk);
      expect(phone.ownProfile!.id, ids.first, reason: 'and stays');
    });

    test('a third Device follows the Profile its own Device gave up', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final tablet = device(clock, 'Tab');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(tablet, phone);
      final shared = phone.ownProfile!.id;
      expect(tablet.ownProfile!.id, shared);
      // The phone still has the defaults: it takes the computer's.
      exchange(phone, desk);
      expect(phone.ownProfile!.id, desk.ownProfile!.id);
      exchange(tablet, phone);
      expect(held(tablet), held(desk));
      expect(tablet.ownProfileAliases, contains(shared));
    });
    test('"Keep mine" reaches the people who knew the Profile given up, '
        'answered on the host or on the joiner', () {
      for (final onHost in [true, false]) {
        final clock = ticking();
        final phone = device(clock, 'Pixel 7')..renameOwnProfile('Ada L.');
        final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
        final cid = device(clock, 'Cid');
        // Cid shares an Area with the phone and keeps its Profile.
        final area = phone.share(Places(phone).addArea('Garden').id);
        exchange(cid, phone, inviting: Invite(area, role: AreaRole.member));
        final phoneId = phone.ownProfile!.id, deskId = desk.ownProfile!.id;
        expect(cid.resolvePerson(phoneId), phoneId);
        // The phone gives its Profile up for the computer's.
        if (onHost) {
          exchange(phone, desk);
        } else {
          exchange(desk, phone);
        }
        desk.answerProfileQuestion(keepMine: true);
        for (var i = 0; i < 3; i++) {
          exchange(phone, desk);
        }
        expect(phone.ownProfile!.id, deskId);
        expect(
          phone.localSetting('profile:aliases'),
          desk.localSetting('profile:aliases'),
        );
        exchange(cid, phone);
        expect(cid.resolvePerson(phoneId), deskId, reason: 'onHost: $onHost');
        expect(cid.personShown(phoneId).name, 'Ada');
      }
    });
  });

  group('aliases', () {
    test('taking a Profile over or restoring it keeps only the aliases a '
        'key bound to it made', () {
      final clock = ticking();
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      final deskId = desk.ownProfile!.id;
      final stranger = SigningKey.generate();
      final strangerId = 'a' * 32;
      final forgedAlias = ProfileAlias(
        alias: strangerId,
        to: deskId,
        by: stranger.publicBase64,
        sig: stranger.sign(profileAliasBytes(strangerId, deskId)),
      );
      expect(forgedAlias.signed, isTrue);
      desk.setLocalSetting(
        'profile:aliases',
        jsonEncode([forgedAlias.toJson()]),
      );
      // A Device with the defaults takes the computer's over.
      final phone = device(clock, 'Pixel 7');
      exchange(phone, desk);
      expect(phone.ownProfile!.id, deskId);
      expect(phone.ownProfileAliases, isNot(contains(strangerId)));
      // A Backup carrying it restores none either.
      final restored = device(clock, 'Pad');
      expect(
        restored.restoreOwnProfile(desk.ownProfileJson, old: desk.signingKey),
        isTrue,
      );
      expect(restored.ownProfile!.id, deskId);
      expect(restored.ownProfileAliases, isEmpty);
    });
  });

  group('updates', () {
    test('a newer validly signed Profile replaces the stored one, and the '
        'listeners hear of it', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(phone, desk);
      var heard = 0;
      phone.addProfileListener(() => heard++);
      desk.renameOwnProfile('Ada Lovelace');
      final done = exchange(phone, desk);
      expect(done.joiner.profile, ProfileTaken.updated);
      expect(heard, 1);
      expect(phone.ownProfile!.name, 'Ada Lovelace');
      expect(held(phone), held(desk));
      // Changed on the phone, which did not make it: the computer takes it.
      phone.renameOwnProfile('Ada');
      exchange(phone, desk);
      expect(desk.ownProfile!.name, 'Ada');
      expect(desk.ownProfile!.version, phone.ownProfile!.version);
    });

    test('a name changed on another Device is not replaced by this one\'s '
        'defaults, and a shared Profile no longer follows them', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(phone, desk);
      expect(phone.ownProfile!.name, 'Ada');
      // The adopter's own name does not replace it.
      phone.applyProfileDefaults(name: 'Pixel 8');
      expect(phone.ownProfile!.name, 'Ada');
      // Nor does the maker's, once another Device holds it.
      final fresh = device(clock, 'Old laptop');
      final other = device(clock, 'New phone');
      exchange(other, fresh);
      for (final store in [fresh, other]) {
        final version = store.ownProfile!.version;
        store.applyProfileDefaults(name: 'Renamed computer');
        expect(store.ownProfile!.version, version);
      }
    });

    test('changed apart at the same version, both end with the same '
        'one', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(phone, desk);
      phone.renameOwnProfile('Ada from the phone');
      desk.renameOwnProfile('Ada from the desk');
      expect(phone.ownProfile!.version, desk.ownProfile!.version);
      exchange(phone, desk);
      expect(phone.ownProfile!.name, desk.ownProfile!.name);
      expect(held(phone), held(desk));
    });

    test('an update signed by a key not bound to the Profile is refused', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(phone, desk);
      final before = phone.ownProfile!.encode();
      final stranger = SigningKey.fromSeed(
        Uint8List.fromList(List.filled(32, 9)),
      );
      final sent = desk.ownSignedProfile!;
      final profile = desk.ownProfile!;
      final forged = jsonEncode({
        'id': profile.id,
        'name': 'Mallory',
        'version': profile.version + 5,
      });
      SignedProfile signedBy(SigningKey key, {List<ProfileKey>? keys}) =>
          SignedProfile(
            record: forged,
            signer: key.publicBase64,
            sig: key.sign(profileRecordBytes(forged)),
            first: sent.first,
            keys: keys ?? sent.keys,
            aliases: sent.aliases,
          );
      // Relayed by the computer, one of the own Devices: still refused.
      final from = desk.signingKey.publicKey;
      expect(
        phone.takeOwnProfile(signedBy(stranger).toJson(), from: from),
        ProfileTaken.refused,
      );
      // A key bound by itself is no key bound.
      final selfBound = [
        ...sent.keys,
        ProfileKey(
          key: stranger.publicBase64,
          by: stranger.publicBase64,
          sig: stranger.sign(
            profileKeyBytes(profile.id, stranger.publicBase64),
          ),
        ),
      ];
      expect(
        phone.takeOwnProfile(
          signedBy(stranger, keys: selfBound).toJson(),
          from: from,
        ),
        ProfileTaken.refused,
      );
      // A record changed after it was signed.
      final changed = sent.toJson()..['record'] = forged;
      expect(phone.takeOwnProfile(changed, from: from), ProfileTaken.refused);
      // The same id with another first key: a copy.
      final copy = SignedProfile(
        record: forged,
        signer: stranger.publicBase64,
        sig: stranger.sign(profileRecordBytes(forged)),
        first: stranger.publicBase64,
      );
      expect(
        phone.takeOwnProfile(copy.toJson(), from: from),
        ProfileTaken.refused,
      );
      expect(phone.ownProfile!.encode(), before);
      // Signed by a bound key, it counts.
      expect(
        phone.takeOwnProfile(signedBy(desk.signingKey).toJson(), from: from),
        ProfileTaken.updated,
      );
      expect(phone.ownProfile!.name, 'Mallory');
    });

    test('a received picture shows made again; one that is none is '
        'refused', () {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk');
      final picture = squareProfilePicture(halves(300, 300))!;
      desk.setOwnProfilePicture(picture);
      exchange(phone, desk);
      expect(phone.ownProfile!.id, desk.ownProfile!.id);
      expect(phone.ownProfile!.picture, isNotNull);
      expect(phone.ownSignedProfile!.record, desk.ownSignedProfile!.record);

      final key = desk.signingKey;
      final profile = desk.ownProfile!;
      final broken = jsonEncode({
        'id': profile.id,
        'name': profile.name,
        'picture': base64.encode(utf8.encode('no picture at all')),
        'version': profile.version + 1,
      });
      final sent = desk.ownSignedProfile!.toJson()
        ..['record'] = broken
        ..['sig'] = key.sign(profileRecordBytes(broken))
        ..['signer'] = key.publicBase64;
      expect(
        phone.takeOwnProfile(sent, from: key.publicKey),
        ProfileTaken.refused,
      );
    });
  });

  group('transports', () {
    test('the shared folder carries it, sealed in the keys file', () async {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(phone, desk);
      desk.renameOwnProfile('Ada Lovelace');
      final folder = MemorySyncFolder();
      await folderSync(desk, folder);
      final read = await folderSync(phone, folder);
      expect(read.profileChanged, isTrue);
      expect(phone.ownProfile!.name, 'Ada Lovelace');
      expect(held(phone), held(desk));
      for (final files in folder.dirs.values) {
        for (final bytes in files.values) {
          expect(
            latin1.decode(bytes, allowInvalid: true),
            isNot(contains('Lovelace')),
          );
        }
      }
    });

    test('a bundle carries it', () async {
      final clock = ticking();
      final phone = device(clock, 'Pixel 7');
      final desk = device(clock, 'Desk')..renameOwnProfile('Ada');
      exchange(phone, desk);
      desk.renameOwnProfile('Ada Lovelace');
      Notes(desk).capture('Written since');
      final dir = Directory.systemTemp.createTempSync('profile');
      addTearDown(() => dir.deleteSync(recursive: true));
      final path = '${dir.path}/desk.$bundleExtension';
      await writeBundle(desk, path, forDevice: phone.deviceId);
      final read = await importBundle(phone, path);
      expect(read.profileChanged, isTrue);
      expect(phone.ownProfile!.name, 'Ada Lovelace');
    });

    test('another person\'s Device gets none, and sends none that is '
        'taken', () {
      final clock = ticking();
      final ann = device(clock, 'Ann')..setLocalSetting('author', 'Ann');
      final ben = device(clock, 'Ben')..setLocalSetting('author', 'Ben');
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id);
      final annBefore = held(ann), benBefore = held(ben);
      final invited = exchange(
        ben,
        ann,
        inviting: Invite(area, role: AreaRole.member),
      );
      final later = exchange(ben, ann);
      // Invited, Ben's phone sends its Profile as it goes to other
      // people, for Ann's desk to show; never what only his own Devices
      // are told.
      expect(invited.request['profile'], ben.ownProfileForOthers);
      expect(invited.request['profile'] as Map, isNot(contains('fresh')));
      expect(later.request.containsKey('profile'), isFalse);
      for (final e in [invited, later]) {
        expect(e.answer.containsKey('profile'), isFalse);
      }
      expect(held(ann), annBefore);
      expect(held(ben), benBefore);
    });

    test('a Device without a Profile yet takes the other one', () {
      final clock = ticking();
      final phone = Store.inMemory(clock: clock);
      addTearDown(phone.close);
      final desk = device(clock, 'Ada');
      exchange(phone, desk);
      expect(held(phone), held(desk));
    });
  });
}
