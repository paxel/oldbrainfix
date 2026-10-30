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

    test('a record whose signature fails is refused before it is read', () {
      final key = SigningKey.generate();
      final record = jsonEncode({
        'id': 'e' * 32,
        'name': 'Eve',
        'picture': '*not base64*${'A' * 1000}',
        'version': 1,
      });
      final bad = SignedProfile(
        record: record,
        signer: key.publicBase64,
        sig: key.sign(profileRecordBytes('{}')),
        first: key.publicBase64,
      ).toJson();
      expect(ann.isOwnPerson(''), isFalse, reason: 'the own Profile read');
      final before = Profile.parsed;
      expect(ann.takePerson(bad), PersonTaken.refused);
      expect(Profile.parsed, before, reason: 'nothing of it was read');
      // Signed, the same record is read, and refused for its picture.
      final signed = SignedProfile(
        record: record,
        signer: key.publicBase64,
        sig: key.sign(profileRecordBytes(record)),
        first: key.publicBase64,
      ).toJson();
      expect(ann.takePerson(signed), PersonTaken.refused);
      expect(Profile.parsed, greaterThan(before));
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

  group('person filter', () {
    List<String> ids(List<Note> notes) => [for (final n in notes) n.id];

    test('"Me" keeps the Notes of every own Device, a person theirs, and '
        'both together both', () {
      final area = household();
      final mine = Notes(ann).capture('Milk', place: area);
      final desk = Notes(annDesk).capture('Bread', place: area);
      final unsorted = Notes(ann).capture('Call the bank');
      exchange(annDesk, ann);
      final theirs = Notes(ben).capture('Eggs', place: area);
      exchange(ben, ann);
      final all = Notes(ann).all();
      const me = PersonFilter(me: true);
      final benOnly = PersonFilter(people: {idOf(ben)});
      expect(ann.notesBy(all, PersonFilter.everyone), all);
      expect(ids(ann.notesBy(all, me)), [unsorted.id, desk.id, mine.id]);
      expect(ids(ann.notesBy(all, benOnly)), [theirs.id]);
      expect(ids(ann.notesBy(all, me.toggled(idOf(ben)))), ids(all));
      // The own Profile's id among the people counts as "Me".
      expect(
        ids(ann.notesBy(all, PersonFilter(people: {idOf(ann)}))),
        ids(ann.notesBy(all, me)),
      );
      // Ben sees it the other way round, Ann's desk as Ann.
      final onBen = Notes(ben).all();
      expect(ids(ben.notesBy(onBen, me)), [theirs.id]);
      expect(ids(ben.notesBy(onBen, PersonFilter(people: {idOf(ann)}))), [
        desk.id,
        mine.id,
      ]);
    });

    test('the filter combines with the search and an Area', () {
      final area = household();
      Notes(ann).capture('Milk', place: area);
      final eggs = Notes(ann).capture('Eggs', place: area);
      Notes(ann).capture('Eggs for the cake');
      final theirs = Notes(ben).capture('Eggs and milk', place: area);
      exchange(ben, ann);
      final found = Search(ann).notes('eggs', within: area);
      expect(ids(found), [theirs.id, eggs.id]);
      expect(ids(ann.notesBy(found, const PersonFilter(me: true))), [eggs.id]);
      expect(ids(ann.notesBy(found, PersonFilter(people: {idOf(ben)}))), [
        theirs.id,
      ]);
    });

    test('a person is found through the aliases of their Profile, from '
        'either side', () {
      final area = household();
      final benDesk = device(clock, 'Ben at the desk');
      final deskId = idOf(benDesk);
      exchange(benDesk, ann, inviting: Invite(area, role: AreaRole.member));
      final note = Notes(benDesk).capture('Fix the tap', place: area);
      exchange(benDesk, ann);
      ben.renameOwnProfile('Ben');
      exchange(ben, benDesk);
      exchange(ben, benDesk);
      exchange(benDesk, ann);
      final all = Notes(ann).all();
      expect(ids(ann.notesBy(all, PersonFilter(people: {deskId}))), [note.id]);
      expect(ids(ann.notesBy(all, PersonFilter(people: {idOf(ben)}))), [
        note.id,
      ]);
      // On Ben's phone the desk's Note is his own, under his old id too.
      exchange(ben, benDesk);
      expect(ben.wroteBy(note.id, const PersonFilter(me: true)), isTrue);
      expect(ben.wroteBy(note.id, PersonFilter(people: {deskId})), isTrue);
    });

    test('a Note whose writer is not known is nobody\'s; one whose Profile '
        'has not arrived is the person\'s the Area names', () {
      final area = household();
      final theirs = Notes(ben).capture('Eggs', place: area);
      final moved = Notes(ben).capture('Flour', place: area);
      exchange(ben, ann);
      // Ann takes one of Ben's Notes into an Area of her own.
      Notes(ann).file(moved.id, Places(ann).addArea('Baking').id);
      expect(ann.authorOf(moved.id)?.id, idOf(ben));
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
      expect(pad.wroteBy(theirs.id, const PersonFilter(me: true)), isFalse);
      expect(pad.wroteBy(theirs.id, PersonFilter(people: {idOf(ben)})), isTrue);
      // Outside the Area no grant names him, and no Profile binds his key
      // here.
      expect(pad.authorOf(moved.id), isNull);
      expect(pad.wroteBy(moved.id, const PersonFilter(me: true)), isFalse);
      expect(pad.wroteBy(moved.id, PersonFilter(people: {idOf(ben)})), isFalse);
      // Nothing in the log says who wrote it.
      expect(pad.wroteBy('0' * 32, const PersonFilter(me: true)), isFalse);
      expect(pad.wroteBy('0' * 32, PersonFilter(people: {idOf(ben)})), isFalse);
    });

    test('a Note accepted from a Sent Item is its sender\'s', () {
      final file = sendNote(ann, Notes(ann).capture('Dentist').id);
      final note = acceptSentItem(ben, openSentItem(ben, file).item)!;
      expect(ben.wroteBy(note, PersonFilter(people: {idOf(ann)})), isTrue);
      expect(ben.wroteBy(note, const PersonFilter(me: true)), isFalse);
    });

    test('a received link another person\'s Device wrote marks nothing', () {
      final area = household();
      final cid = device(clock, 'Cid');
      final cids = acceptSentItem(
        ann,
        openSentItem(ann, sendNote(cid, Notes(cid).capture('Hi').id)).item,
      )!;
      final cidKey = base64.encode(cid.signingKey.publicKey);
      expect(ann.authorOf(cids)!.id, idOf(cid));
      // Ben's modified app marks his Notes as Ann's and as Cid's.
      final asAnn = Notes(ben).capture('Eggs', place: area);
      final asCid = Notes(ben).capture('Flour', place: area);
      String link(String key, String person) =>
          jsonEncode({'from': key, 'id': 'x', 'profile': person, 'version': 1});
      ben.append(
        asAnn.id,
        'received',
        link(base64.encode(ann.signingKey.publicKey), idOf(ann)),
      );
      ben.append(asCid.id, 'received', link(cidKey, idOf(cid)));
      exchange(ben, ann);
      expect(ann.authorOf(asAnn.id)!.id, idOf(ben));
      expect(ann.wroteBy(asAnn.id, const PersonFilter(me: true)), isFalse);
      expect(ann.authorOf(asCid.id)!.id, idOf(ben));
    });

    test('a newer received link another person\'s Device wrote does not '
        'hide the one accepting wrote', () {
      final area = household();
      final cid = device(clock, 'Cid');
      final cids = acceptSentItem(
        ann,
        openSentItem(ann, sendNote(cid, Notes(cid).capture('Hi').id)).item,
      )!;
      // A Manager can write on Ann's Note too.
      ann.setRole(area, idOf(ben), AreaRole.manager);
      exchange(ben, ann);
      Notes(ann).file(cids, area);
      exchange(ben, ann);
      final forged = jsonEncode({
        'from': base64.encode(ann.signingKey.publicKey),
        'id': 'x',
        'profile': idOf(ann),
        'version': 1,
      });
      ben.append(cids, 'received', forged);
      exchange(ben, ann);
      expect(ann.current(cids, 'received'), forged, reason: 'it landed');
      expect(ann.authorOf(cids)!.id, idOf(cid));
    });

    test('a Sent Item carrying the receiver\'s Profile id, or a third '
        'person\'s, shows its sender by the key', () {
      final eve = device(clock, 'Eve');
      final cid = device(clock, 'Cid');
      for (final from in [eve, cid]) {
        acceptSentItem(
          ben,
          openSentItem(ben, sendNote(from, Notes(from).capture('Hi').id)).item,
        );
      }
      String forgedAs(String id, String name) {
        final record = Profile(id: id, name: name, version: 9).encode();
        final copy = SignedProfile(
          record: record,
          signer: eve.signingKey.publicBase64,
          sig: eve.signingKey.sign(profileRecordBytes(record)),
          first: eve.signingKey.publicBase64,
        );
        final file = encodeSentItem(
          SentItem(
            id: 'as-$name',
            version: 1,
            from: eve.ownKeyRecord,
            profile: copy,
            text: 'Hello from $name',
          ),
          eve.signingKey,
        );
        return acceptSentItem(ben, openSentItem(ben, file).item)!;
      }

      final asBen = forgedAs(idOf(ben), 'Ben');
      expect(ben.authorOf(asBen)?.id, idOf(eve));
      expect(ben.wroteBy(asBen, const PersonFilter(me: true)), isFalse);
      final asCid = forgedAs(idOf(cid), 'Cid');
      expect(ben.authorOf(asCid)?.id, idOf(eve));
      expect(ben.wroteBy(asCid, PersonFilter(people: {idOf(cid)})), isFalse);
    });
  });

  group('Sent Item links', () {
    /// Cid's Note "Hi" with a Reminder, accepted on Ann's phone: Cid's
    /// Note and the Note Ann holds it as.
    ({Store cid, String sent, String held}) fromCid() {
      final cid = device(clock, 'Cid');
      final sent = Notes(cid).capture('Hi').id;
      Routines(cid).spawn(
        sent,
        Schedule(start: Day(2026, 3, 3), times: [ClockTime(10, 0)]),
      );
      final held = acceptSentItem(
        ann,
        openSentItem(ann, sendNote(cid, sent)).item,
      )!;
      return (cid: cid, sent: sent, held: held);
    }

    /// The `received` link Ann's phone keeps on [held], with [version].
    String relinked(String held, int version) => jsonEncode({
      ...jsonDecode(ann.current(held, 'received')!) as Map<String, Object?>,
      'version': version,
    });

    test('a Sent Item\'s link another person\'s Device wrote on Ann\'s Note '
        'is no Note its update or cancellation reaches', () {
      final area = household();
      final (:cid, :sent, :held) = fromCid();
      // A Manager can write on Ann's Note too.
      ann.setRole(area, idOf(ben), AreaRole.manager);
      exchange(ben, ann);
      final rent = Notes(ann).capture('Pay the rent', place: area).id;
      exchange(ben, ann);
      ben.append(rent, 'received', relinked(held, 1));
      exchange(ben, ann);
      expect(ann.current(rent, 'received'), isNotNull, reason: 'it landed');

      Notes(cid).edit(sent, 'Hi again');
      final update = openSentItem(ann, sendNote(cid, sent));
      expect(update.state, SentItemState.update);
      expect(update.accepted, held);
      expect(acceptSentItem(ann, update.item), held);
      expect(Notes(ann).byId(held)!.text, 'Hi again');
      expect(Notes(ann).byId(rent)!.text, 'Pay the rent');

      final cancel = openSentItem(ann, sendNote(cid, sent, cancel: true));
      expect(cancel.state, SentItemState.cancel);
      expect(acceptSentItem(ann, cancel.item), held);
      expect(Notes(ann).byId(held)!.isTrashed, isTrue);
      expect(Notes(ann).byId(rent)!.isTrashed, isFalse);
    });

    test('a Member\'s own Note linked to a Sent Item never accepted here '
        'is neither cancelled nor updated', () {
      final area = household();
      final cid = device(clock, 'Cid');
      final sent = Notes(cid).capture('Hi').id;
      final item = openSentItem(ann, sendNote(cid, sent)).item;
      final eggs = Notes(ben).capture('Eggs', place: area).id;
      ben.append(
        eggs,
        'received',
        jsonEncode({
          'from': base64.encode(cid.signingKey.publicKey),
          'id': item.id,
          'profile': idOf(cid),
          'version': 1,
        }),
      );
      exchange(ben, ann);
      expect(ann.current(eggs, 'received'), isNotNull, reason: 'it landed');

      final cancel = openSentItem(ann, sendNote(cid, sent, cancel: true));
      expect(cancel.state, SentItemState.nothingToCancel);
      expect(acceptSentItem(ann, cancel.item), isNull);
      expect(Notes(ann).byId(eggs)!.isTrashed, isFalse);

      final update = openSentItem(ann, sendNote(cid, sent));
      expect(update.state, SentItemState.fresh);
      expect(acceptSentItem(ann, update.item), isNot(eggs));
      expect(Notes(ann).byId(eggs)!.text, 'Eggs');
    });

    test('the link one\'s own phone wrote carries an update applied on the '
        'desk and a cancellation back on the phone', () {
      household();
      final (:cid, :sent, :held) = fromCid();
      exchange(annDesk, ann);
      Notes(cid).edit(sent, 'Hi again');
      final update = openSentItem(annDesk, sendNote(cid, sent));
      expect(update.state, SentItemState.update);
      expect(update.changes!.routinesAdded, isEmpty);
      expect(acceptSentItem(annDesk, update.item), held);
      expect(Notes(annDesk).byId(held)!.text, 'Hi again');
      exchange(annDesk, ann);

      final cancel = openSentItem(ann, sendNote(cid, sent, cancel: true));
      expect(cancel.state, SentItemState.cancel);
      expect(acceptSentItem(ann, cancel.item), held);
      expect(Notes(ann).byId(held)!.isTrashed, isTrue);
      expect(Routines(ann).all(), isEmpty, reason: 'alarms off');
    });

    test('a newer link another person\'s Device wrote does not hide the one '
        'accepting wrote from an update', () {
      final area = household();
      final (:cid, :sent, :held) = fromCid();
      ann.setRole(area, idOf(ben), AreaRole.manager);
      exchange(ben, ann);
      Notes(ann).file(held, area);
      exchange(ben, ann);
      final routine = Routines(ann).ofNote(held).single.id;
      // Claims every version is held already, and the Reminder is
      // none of Cid's.
      ben.append(held, 'received', relinked(held, 99));
      ben.append(routine, 'received', 'routine:forged');
      exchange(ben, ann);
      expect(
        (jsonDecode(ann.current(held, 'received')!) as Map)['version'],
        99,
        reason: 'it landed',
      );
      expect(ann.current(routine, 'received'), 'routine:forged');

      Notes(cid).edit(sent, 'Hi again');
      final update = openSentItem(ann, sendNote(cid, sent));
      expect(update.state, SentItemState.update);
      expect(update.changes!.routinesAdded, isEmpty);
      expect(update.changes!.routinesRemoved, isEmpty);
      acceptSentItem(ann, update.item);
      expect(Notes(ann).byId(held)!.text, 'Hi again');
      expect(Routines(ann).ofNote(held), hasLength(1));
    });

    test('a text another person\'s Device wrote with a link is not the one '
        'the sender sent last', () {
      final area = household();
      final (:cid, :sent, :held) = fromCid();
      ann.setRole(area, idOf(ben), AreaRole.manager);
      exchange(ben, ann);
      Notes(ann).file(held, area);
      exchange(ben, ann);
      // Both at once, as accepting writes them.
      final at = clock().add(const Duration(hours: 1));
      ben.append(
        held,
        NoteKeys.text,
        StoredText.of('plain', 'Hi from Ben').encode(),
        date: at,
      );
      ben.append(held, 'received', relinked(held, 1), date: at);
      exchange(ben, ann);
      expect(Notes(ann).byId(held)!.text, 'Hi from Ben', reason: 'it landed');

      Notes(cid).edit(sent, 'Hi again');
      final update = openSentItem(ann, sendNote(cid, sent));
      expect(update.changes!.textBefore, 'Hi from Ben');
      expect(update.changes!.textEdited, isTrue);
    });
  });

  group('Sent Item links on the sending side', () {
    /// The link Ann's phone keeps on a Note it sent.
    String sentField() => SentKeys.sent(ann.deviceId);

    /// Ann's Note "Dentist" sent to Cid and accepted there: the Sent Item
    /// and the Note Cid holds it as.
    ({SentItem item, String held}) dentistAtCid(Store cid) {
      final item = openSentItem(
        cid,
        sendNote(ann, Notes(ann).capture('Dentist').id),
      ).item;
      return (item: item, held: acceptSentItem(cid, item)!);
    }

    test('a link another person\'s Device wrote on Ann\'s Note sends it as '
        'a new Sent Item, no update or cancellation of hers', () {
      final area = household();
      ann.setRole(area, idOf(ben), AreaRole.manager);
      exchange(ben, ann);
      final cid = device(clock, 'Cid');
      final dentist = dentistAtCid(cid);
      final rent = Notes(ann).capture('Pay the rent', place: area).id;
      exchange(ben, ann);
      ben.append(
        rent,
        sentField(),
        jsonEncode({'id': dentist.item.id, 'version': 1}),
      );
      exchange(ben, ann);
      expect(ann.current(rent, sentField()), isNotNull, reason: 'it landed');
      expect(wasSent(ann, rent), isFalse);

      final cancel = openSentItem(cid, sendNote(ann, rent, cancel: true));
      expect(cancel.item.id, isNot(dentist.item.id));
      expect(cancel.state, SentItemState.nothingToCancel);
      expect(acceptSentItem(cid, cancel.item), isNull);
      expect(Notes(cid).byId(dentist.held)!.isTrashed, isFalse);

      final again = openSentItem(cid, sendNote(ann, rent));
      expect(again.item.id, isNot(dentist.item.id));
      expect(again.state, SentItemState.fresh);
      expect(acceptSentItem(cid, again.item), isNot(dentist.held));
      expect(Notes(cid).byId(dentist.held)!.text, 'Dentist');
    });

    test('Ann\'s own link makes the second send an update, also after it '
        'went through her other own Device', () {
      household();
      final cid = device(clock, 'Cid');
      final hi = Notes(ann).capture('Hi').id;
      final first = openSentItem(cid, sendNote(ann, hi)).item;
      final held = acceptSentItem(cid, first)!;
      exchange(annDesk, ann);
      Notes(annDesk).edit(hi, 'Hi again');
      exchange(annDesk, ann);
      expect(wasSent(ann, hi), isTrue);

      final update = openSentItem(cid, sendNote(ann, hi));
      expect(update.item.id, first.id);
      expect(update.item.version, 2);
      expect(update.state, SentItemState.update);
      expect(acceptSentItem(cid, update.item), held);
      expect(Notes(cid).byId(held)!.text, 'Hi again');
      // Another Device of Ann's sends under an id of its own (ADR 0015).
      expect(wasSent(annDesk, hi), isFalse);
      expect(sentItemOf(annDesk, hi).id, isNot(first.id));
    });

    test('a newer link another person\'s Device wrote does not hide Ann\'s '
        'own', () {
      final area = household();
      ann.setRole(area, idOf(ben), AreaRole.manager);
      exchange(ben, ann);
      final cid = device(clock, 'Cid');
      final dentist = dentistAtCid(cid);
      final rent = Notes(ann).capture('Pay the rent', place: area).id;
      final first = openSentItem(cid, sendNote(ann, rent)).item;
      final held = acceptSentItem(cid, first)!;
      exchange(ben, ann);
      ben.append(
        rent,
        sentField(),
        jsonEncode({'id': dentist.item.id, 'version': 99}),
      );
      exchange(ben, ann);
      expect(
        (jsonDecode(ann.current(rent, sentField())!) as Map)['version'],
        99,
        reason: 'it landed',
      );

      final update = openSentItem(cid, sendNote(ann, rent));
      expect(update.item.id, first.id);
      expect(update.item.version, 2);
      expect(update.state, SentItemState.update);
      expect(acceptSentItem(cid, update.item), held);
      expect(Notes(cid).byId(dentist.held)!.text, 'Dentist');
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

    test('a Profiles file of a later version is not read (ADR 0034)', () async {
      final area = household();
      final folder = MemorySyncFolder();
      ann.renameOwnProfile('Ann A.');
      await folderSync(ann, folder);
      final key = ann.areaKeys().firstWhere((k) => k.scope == area);
      final name = profilesFileName(ann.deviceId, key.id);
      final aad = SealedAs.profiles(ann.deviceId);
      final plain = unseal(key.key, folder.dirs['keys']![name]!, aad: aad);
      final json = jsonDecode(utf8.decode(plain!)) as Map<String, Object?>;
      folder.dirs['keys']![name] = seal(
        key.key,
        utf8.encode(jsonEncode({...json, 'v': 2})),
        aad: aad,
      );
      final result = await folderSync(ben, folder);
      expect(result.newer, 1);
      expect(result.profileChanged, isFalse);
      expect(ben.personShown(idOf(ann)).name, 'Ann');
      // Anyone holding the Area's key could have written that one: the
      // own is written over as before.
      await folderSync(ann, folder);
      final again = unseal(key.key, folder.dirs['keys']![name]!, aad: aad);
      expect(jsonDecode(utf8.decode(again!)), json);
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
