import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'profiles_test.dart' show halves;
import 'shared_areas_test.dart' show exchange, ticking;
import 'test_host.dart';

/// A Device with its own Profile made from the default [name].
Store device(DateTime Function() clock, String name) {
  final store = Store.inMemory(clock: clock);
  addTearDown(store.close);
  store.applyProfileDefaults(name: name);
  return store;
}

/// A Profile made by hand with the key [key]: the record [record] (any
/// JSON, so names and pictures no app would write fit in) signed by
/// [signer], or by [key] itself.
Map<String, Object?> forged(
  SigningKey key,
  Map<String, Object?> record, {
  SigningKey? signer,
  List<Map<String, Object?>> keys = const [],
}) {
  final text = jsonEncode(record);
  final by = signer ?? key;
  return SignedProfile(
    record: text,
    signer: by.publicBase64,
    sig: by.sign(profileRecordBytes(text)),
    first: key.publicBase64,
    keys: [for (final k in keys) ProfileKey.fromJson(k)!],
  ).toJson();
}

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store annDesk;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    annDesk = device(clock, 'Ann at the desk');
    ben = device(clock, 'Ben');
  });

  /// Ann's household shared with Ben as a Member, Ann's desk one of her
  /// own Devices.
  String household() {
    exchange(annDesk, ann);
    final house = Places(ann).addArea('Household');
    final area = ann.share(house.id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(annDesk, ann);
    return area;
  }

  String idOf(Store store) => store.ownProfile!.id;

  group('travel', () {
    test('an invite carries both Profiles, each bound to the keys it came '
        'with', () {
      final area = household();
      expect(ann.personShown(idOf(ben)).name, 'Ben');
      expect(ben.personShown(idOf(ann)).name, 'Ann');
      expect(ann.people(area).map((p) => ann.personShown(p.id).name), [
        'Ann',
        'Ben',
      ]);
      expect(ben.personCode(idOf(ann)), ann.ownProfileCode);
      // One's own Devices pass the people's Profiles on.
      expect(annDesk.personShown(idOf(ben)).name, 'Ben');
      // Ann's desk took Ann's Profile over: it is no other person to Ben.
      expect(annDesk.ownProfile!.id, idOf(ann));
    });

    test('a renamed Profile reaches the others with the next sync', () {
      household();
      ben.renameOwnProfile('Benjamin');
      exchange(ben, ann);
      expect(ann.personShown(idOf(ben)).name, 'Benjamin');
      exchange(annDesk, ann);
      expect(annDesk.personShown(idOf(ben)).name, 'Benjamin');
    });

    test('another person gets only the Profiles of the people of the '
        'Areas shared with them', () {
      household();
      final cat = device(clock, 'Cat');
      final garden = ann.share(Places(ann).addArea('Garden').id);
      exchange(cat, ann, inviting: Invite(garden, role: AreaRole.member));
      expect(ann.personShown(idOf(cat)).name, 'Cat');
      final toBen = exchange(ben, ann).answer['profiles'] as List;
      expect([
        for (final p in toBen) Profile.parse((p as Map)['record'])!.name,
      ], unorderedEquals(['Ann', 'Ben']));
      expect(ben.personShown(idOf(cat)).name, isEmpty);
      // And takes no others from them either.
      final taken = ann.takePeople([
        cat.ownProfileForOthers,
      ], allowed: ann.peopleFrom(ben.deviceId));
      expect(taken, isFalse);
    });

    test('a Device of another person names its Profiles under the Area\'s '
        'grants only', () {
      household();
      final dan = device(clock, 'Dan');
      expect(
        ann.takePerson(
          dan.ownProfileForOthers,
          allowed: ann.peopleFrom(ben.deviceId),
        ),
        PersonTaken.refused,
      );
      expect(ann.keptPeople.map((p) => p.name), ['Ben']);
    });
  });

  group('binding', () {
    test('an update signed by a key not bound to the Profile is refused', () {
      household();
      final sent = SignedProfile.fromJson(ben.ownProfileForOthers)!;
      final record = Profile.parse(sent.record)!;
      final mallory = SigningKey.generate();
      final text = jsonEncode({
        'id': record.id,
        'name': 'Mallory',
        'version': record.version + 1,
      });
      // Same id, same first key, signed by a key nobody bound.
      final update = SignedProfile(
        record: text,
        signer: mallory.publicBase64,
        sig: mallory.sign(profileRecordBytes(text)),
        first: sent.first,
        keys: [
          ...sent.keys,
          ProfileKey(
            key: mallory.publicBase64,
            by: mallory.publicBase64,
            sig: mallory.sign(profileKeyBytes(record.id, mallory.publicBase64)),
          ),
        ],
      ).toJson();
      expect(ann.takePerson(update), PersonTaken.refused);
      expect(ann.personShown(record.id).name, 'Ben');
      // A copied id under another first key is somebody else.
      expect(
        ann.takePerson(
          forged(mallory, {'id': record.id, 'name': 'Ben', 'version': 9}),
        ),
        PersonTaken.refused,
      );
      expect(ann.personShown(record.id).version, record.version);
      // Ben's own update counts.
      ben.renameOwnProfile('Ben B.');
      expect(ann.takePerson(ben.ownProfileForOthers), PersonTaken.updated);
      expect(ann.personShown(record.id).name, 'Ben B.');
    });

    test('a key joins when a bound key signed it', () {
      household();
      final benDesk = device(clock, 'Ben at the desk');
      ben.renameOwnProfile('Ben');
      exchange(ben, benDesk);
      exchange(ben, benDesk);
      exchange(ben, ann);
      final benDeskKey = base64.encode(benDesk.signingKey.publicKey);
      final kept = SignedProfile.fromJson(ann.profilesToSend().single)!;
      expect(kept.keys.map((k) => k.key), contains(benDeskKey));
    });

    test('names and pictures are sanitised again on receipt', () {
      final key = SigningKey.generate();
      final id = 'f' * 32;
      expect(
        ann.takePerson(
          forged(key, {
            'id': id,
            'name': ' \u202eAnn\u200b\u0007 Lee ',
            'version': 1,
          }),
        ),
        PersonTaken.added,
      );
      expect(ann.personShown(id).name, 'Ann Lee');
      final big = halves(600, 400);
      final made = ann.takePerson(
        forged(key, {
          'id': id,
          'name': 'Ann',
          'picture': base64.encode(big),
          'version': 2,
        }),
      );
      expect(made, PersonTaken.updated);
      final shown = ann.personShown(id).picture!;
      expect(pictureSize(shown), (width: 256, height: 256));
      expect(shown, isNot(big));
      final notAPicture = Uint8List.fromList(utf8.encode('not a picture'));
      final tooLarge = Uint8List(profilePictureMaxBytes + 1)
        ..setAll(0, halves(64, 64));
      for (final bytes in [notAPicture, tooLarge]) {
        expect(
          ann.takePerson(
            forged(key, {
              'id': id,
              'name': 'Ann',
              'picture': base64.encode(bytes),
              'version': 3,
            }),
          ),
          PersonTaken.refused,
        );
      }
      expect(ann.personShown(id).version, 2);
    });

    test('a Profile shown before an invite must hold the key of the Device '
        'that sent it', () {
      expect(
        ann.sentProfileOf(ben.ownProfileForOthers, device: ben.deviceId),
        isNotNull,
      );
      expect(
        ann.sentProfileOf(ben.ownProfileForOthers, device: annDesk.deviceId),
        isNull,
      );
      expect(ann.sentProfileOf(null, device: ben.deviceId), isNull);
    });
  });

  group('authors', () {
    test('a Note is marked with the Profile of the Device that wrote it', () {
      final area = household();
      final mine = Notes(ann).capture('Milk', place: area);
      final desk = Notes(annDesk).capture('Bread', place: area);
      exchange(annDesk, ann);
      final theirs = Notes(ben).capture('Eggs', place: area);
      exchange(ben, ann);
      exchange(annDesk, ann);
      expect(ann.authorOf(mine.id), isNull);
      expect(ann.authorOf(desk.id), isNull);
      expect(ann.authorOf(theirs.id)!.name, 'Ben');
      expect(annDesk.authorOf(theirs.id)!.name, 'Ben');
      expect(ben.authorOf(theirs.id), isNull);
      expect(ben.authorOf(mine.id)!.name, 'Ann');
      expect(ben.authorOf(desk.id)!.id, idOf(ann));
    });

    test('a Note is marked through the aliases of its writer\'s Profile', () {
      final area = household();
      // Ben's desk was invited on its own, under its own Profile.
      final benDesk = device(clock, 'Ben at the desk');
      final deskId = idOf(benDesk);
      final house = area;
      exchange(benDesk, ann, inviting: Invite(house, role: AreaRole.member));
      final note = Notes(benDesk).capture('Fix the tap', place: house);
      exchange(benDesk, ann);
      expect(ann.authorOf(note.id)!.name, 'Ben at the desk');
      // Then Ben paired it with his phone, whose Profile it took over.
      ben.renameOwnProfile('Ben');
      exchange(ben, benDesk);
      exchange(ben, benDesk);
      expect(benDesk.ownProfile!.id, idOf(ben));
      expect(benDesk.ownProfileAliases, contains(deskId));
      exchange(benDesk, ann);
      expect(ann.resolvePerson(deskId), idOf(ben));
      expect(ann.authorOf(note.id)!.id, idOf(ben));
      expect(ann.authorOf(note.id)!.name, 'Ben');
    });

    test('a Profile claiming another person\'s key marks none of their '
        'Notes', () {
      final area = household();
      final theirs = Notes(ben).capture('Eggs', place: area);
      exchange(ben, ann);
      // Somebody binds Ben's key into a Profile of their own.
      final mallory = SigningKey.generate();
      final id = '0' * 32;
      final benKey = base64.encode(ben.signingKey.publicKey);
      ann.takePerson(
        forged(
          mallory,
          {'id': id, 'name': 'Mallory', 'version': 1},
          keys: [
            ProfileKey(
              key: benKey,
              by: mallory.publicBase64,
              sig: mallory.sign(profileKeyBytes(id, benKey)),
            ).toJson(),
          ],
        ),
      );
      expect(ann.personShown(id).name, 'Mallory');
      expect(ann.authorOf(theirs.id)!.name, 'Ben');
    });

    test('a Profile naming somebody else\'s id as its alias takes nobody\'s '
        'place', () {
      final area = household();
      final theirs = Notes(ben).capture('Eggs', place: area);
      exchange(ben, ann);
      final mallory = SigningKey.generate();
      final id = '0' * 32;
      final benId = idOf(ben);
      final claim = SignedProfile.fromJson(
        forged(mallory, {'id': id, 'name': 'Mallory', 'version': 1}),
      )!;
      final json = SignedProfile(
        record: claim.record,
        signer: claim.signer,
        sig: claim.sig,
        first: claim.first,
        aliases: [
          ProfileAlias(
            alias: benId,
            to: id,
            by: mallory.publicBase64,
            sig: mallory.sign(profileAliasBytes(benId, id)),
          ),
        ],
      ).toJson();
      // Not let in through Ben's id from another person's Device.
      expect(
        ann.takePerson(json, allowed: ann.peopleFrom(ben.deviceId)),
        PersonTaken.refused,
      );
      // Kept all the same, as one's own Devices pass everything on.
      expect(ann.takePerson(json), PersonTaken.added);
      expect(ann.resolvePerson(benId), benId);
      expect(ann.personShown(benId).name, 'Ben');
      expect(ann.authorOf(theirs.id)!.name, 'Ben');
      expect(ann.people(area).map((p) => ann.personShown(p.id).name), [
        'Ann',
        'Ben',
      ]);
    });

    test('until the writer\'s Profile arrives, the placeholder shows', () {
      final area = household();
      final theirs = Notes(ben).capture('Eggs', place: area);
      exchange(ben, ann);
      // Another Device of Ann's, met without the Profiles.
      final pad = device(clock, 'Ann on the pad');
      final request = syncRequest(
        pad,
        ann.versionVector(),
        name: 'Pad',
        hostDevice: ann.deviceId,
      );
      final done = answerSync(
        ann,
        jsonDecode(jsonEncode(request)) as Map<String, Object?>,
        name: 'Desk',
      )!;
      applyAnswer(
        pad,
        jsonDecode(jsonEncode(done.answer..remove('profiles')))
            as Map<String, Object?>,
      );
      expect(pad.keptPeople, isEmpty);
      final author = pad.authorOf(theirs.id)!;
      expect((author.id, author.name, author.picture), (idOf(ben), '', null));
    });
  });

  group('the shared folder and bundles', () {
    test('the people\'s Profiles go sealed under the Area\'s key', () async {
      final area = household();
      final folder = MemorySyncFolder();
      await folderSync(ann, folder);
      final key = ann.areaKeys().firstWhere((k) => k.scope == area);
      final names = await folder.list('keys');
      expect(names, contains(profilesFileName(ann.deviceId, key.id)));
      // No Profiles file for the personal key or a private Area.
      expect(names.where((n) => n.endsWith('.profiles')), hasLength(1));
      final plain = unseal(
        key.key,
        (await folder.read('keys', profilesFileName(ann.deviceId, key.id)))!,
        aad: SealedAs.profiles(ann.deviceId),
      )!;
      final sent = jsonDecode(utf8.decode(plain))['profiles'] as List;
      expect(sent, hasLength(2));
      // Another reader of Ben's: it learns Ann's renamed Profile from the
      // folder.
      ann.renameOwnProfile('Ann A.');
      await folderSync(ann, folder);
      final result = await folderSync(ben, folder);
      expect(result.profileChanged, isTrue);
      expect(ben.personShown(idOf(ann)).name, 'Ann A.');
    });

    test('a bundle for the other person carries the Profiles', () async {
      final area = household();
      ann.renameOwnProfile('Ann A.');
      Notes(ann).capture('Milk', place: area);
      final dir = Directory.systemTemp.createTempSync('people');
      addTearDown(() => dir.deleteSync(recursive: true));
      final path = '${dir.path}/ann.$bundleExtension';
      await writeBundle(ann, path, forDevice: ben.deviceId);
      final read = await importBundle(ben, path);
      expect(read.profileChanged, isTrue);
      expect(ben.personShown(idOf(ann)).name, 'Ann A.');
    });
  });

  group('inviting', () {
    test('the joiner sees the host\'s Profile and invites only once the '
        'user confirms', () async {
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id);
      final host = TestHost(ben);
      await host.start();
      try {
        final seen = <String>[];
        await expectLater(
          lanSync(
            ann,
            decodePairCode(host.code())!,
            name: 'Phone',
            invite: Invite(area, role: AreaRole.member),
            confirm: (other) async {
              seen.add('${other.profile.name} ${other.code}');
              return false;
            },
          ),
          throwsA(
            isA<SyncException>().having(
              (e) => e.failure,
              'failure',
              SyncFailure.cancelled,
            ),
          ),
        );
        expect(seen, ['Ben ${ben.ownProfileCode}']);
        expect(host.bodies, isEmpty);
        expect(ann.people(area), hasLength(1));
        await lanSync(
          ann,
          decodePairCode(host.code())!,
          name: 'Phone',
          invite: Invite(area, role: AreaRole.member),
          confirm: (_) async => true,
        );
        expect(ann.people(area).map((p) => ann.personShown(p.id).name), [
          'Ann',
          'Ben',
        ]);
        expect(ben.personShown(idOf(ann)).name, 'Ann');
      } finally {
        await host.stop();
      }
    });

    test('a host inviting refuses a joiner without a Profile', () {
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id);
      final bare = Store.inMemory(clock: clock);
      addTearDown(bare.close);
      final request = syncRequest(
        bare,
        ann.versionVector(),
        name: 'Phone',
        hostDevice: ann.deviceId,
        invited: true,
      );
      expect(
        answerSync(
          ann,
          jsonDecode(jsonEncode(request)) as Map<String, Object?>,
          name: 'Desk',
          inviting: Invite(area, role: AreaRole.member),
        ),
        isNull,
      );
      expect(ann.people(area), hasLength(1));
    });
  });
}
