import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'profiles_test.dart' show halves;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store annDesk;
  late Store ben;
  late Store carol;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    annDesk = device(clock, 'Ann at the desk');
    ben = device(clock, 'Ben')..renameOwnProfile('Ben');
    carol = device(clock, 'Carol')..renameOwnProfile('Carol');
  });

  String idOf(Store store) => store.ownProfile!.id;

  final pills = Schedule(
    start: Day(2026, 3, 1),
    times: [ClockTime(8, 0)],
    rule: Recurrence.parse('FREQ=DAILY'),
  );
  final eight = Occurrence(Day(2026, 3, 1), ClockTime(8, 0));
  final nine = Occurrence(Day(2026, 3, 2), ClockTime(8, 0));
  final photo = halves(8, 8, png: false);

  /// Ann's Household shared with Ben and Carol as Members, Ann's desk one
  /// of her own Devices.
  String household() {
    exchange(annDesk, ann);
    final area = ann.share(Places(ann).addArea('Household').id);
    exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(carol, ann, inviting: Invite(area, role: AreaRole.member));
    exchange(annDesk, ann);
    return area;
  }

  /// The entries [store] holds that [writer] wrote.
  List<Entry> by(Store store, Store writer) => [
    for (final e in store.allEntries())
      if (e.device == writer.deviceId) e,
  ];

  /// Ben takes in what Carol holds, as her Device would hand it over.
  void relayToBen() => ben.applyEntries(
    carol.entriesSince(ben.versionVector()),
    keys: carol.keyRecords(),
  );

  group('blocking', () {
    test(
      'deletes what she wrote, her photo too, and keeps her check-offs',
      () async {
        final area = household();
        final milk = Notes(ann).capture('Milk', place: area);
        final bins = Routines(ann).spawn(milk.id, pills);
        exchange(carol, ann);
        final rude = Notes(carol).capture('Rude', place: area, photos: [photo]);
        Notes(carol).amend(milk.id, 'Rude too');
        Routines(carol).markDone(bins.id, eight);
        exchange(carol, ann);
        exchange(ben, ann);
        final hash = Notes(ann).byId(rude.id)!.photos.single;
        expect(ann.putBlob('photo', hash, carol.photo(hash)!), isTrue);
        // Ann made a Reminder of Carol's Note: it stays in the Area.
        final cake = Routines(ann).spawn(rude.id, pills);
        final carolMax = carol.versionVector()[carol.deviceId]!;

        expect(ann.blockPerson(idOf(carol)), isEmpty);

        expect(Notes(ann).byId(rude.id), isNull);
        expect(Notes(ann).sections(milk.id), isEmpty);
        expect(Notes(ann).all().map((n) => n.text), ['Milk']);
        expect(ann.photo(hash), isNull);
        expect(Routines(ann).byId(bins.id)!.done.keys, [eight.key]);
        expect(ann.areaOf(cake.id), area);
        expect(by(ann, carol), isNotEmpty);
        expect(by(ann, carol).every(keptFromBlocked), isTrue);
        expect(ann.versionVector()[carol.deviceId], carolMax);
        // Nothing of hers goes into the shared folder.
        final folder = MemorySyncFolder();
        await folderSync(ann, folder);
        expect(await folder.list('blobs'), isEmpty);
        // Ben keeps what he has.
        exchange(ben, ann);
        expect(Notes(ben).byId(rude.id)?.text, 'Rude');
      },
    );

    test('a sync relaying her Note through a third Device stores none of '
        'it, nor asks for her photo', () {
      final area = household();
      final milk = Notes(ann).capture('Milk', place: area);
      final bins = Routines(ann).spawn(milk.id, pills);
      exchange(carol, ann);
      exchange(ben, ann);
      ann.blockPerson(idOf(carol));
      final rude = Notes(carol).capture('Rude', place: area, photos: [photo]);
      Routines(carol).markDone(bins.id, eight);
      relayToBen();
      final hash = Notes(ben).byId(rude.id)!.photos.single;
      exchange(ben, ann);

      expect(Notes(ann).byId(rude.id), isNull);
      expect(by(ann, carol).every(keptFromBlocked), isTrue);
      expect(
        by(ann, carol).where((e) => e.field == RoutineKeys.done(eight)),
        hasLength(1),
      );
      expect(Routines(ann).byId(bins.id)!.done.keys, [eight.key]);
      expect(ann.missingBlobs(), isNot(contains('photo/$hash')));
      expect(ann.knowsBlob('photo/$hash'), isFalse);
      // Nobody is asked for it again.
      final count = ann.allEntries().length;
      final again = exchange(ben, ann);
      expect(ann.allEntries(), hasLength(count));
      expect(
        (again.request['entries']! as List).where(
          (e) => (e as Map)['device'] == carol.deviceId,
        ),
        isEmpty,
      );
    });

    test('her Device met only through a grant that comes later is cleared '
        'when it comes', () {
      final area = household();
      ann.blockPerson(idOf(carol));
      final rude = Notes(carol).capture('Rude', place: area);
      relayToBen();
      exchange(ben, ann);
      expect(Notes(ann).byId(rude.id), isNull);
    });

    test('what is refused counts as held; a forged entry of hers moves '
        'nothing', () {
      final area = household();
      ann.blockPerson(idOf(carol));
      Notes(carol).capture('Rude', place: area);
      final real = by(carol, carol);
      final last = real.lastWhere((e) => !keptFromBlocked(e));
      final forged = Entry.fromJson({...last.toJson(), 'dseq': 999});
      ann.applyEntries([...real, forged], keys: carol.keyRecords());
      expect(
        ann.versionVector()[carol.deviceId],
        carol.versionVector()[carol.deviceId],
      );
      expect(ann.holds(carol.deviceId, last.dseq), isFalse);
    });

    test('unblocking fetches her content again from a Device holding it', () {
      final area = household();
      final milk = Notes(ann).capture('Milk', place: area);
      final bins = Routines(ann).spawn(milk.id, pills);
      exchange(carol, ann);
      final rude = Notes(carol).capture('Rude', place: area, photos: [photo]);
      // A check-off of hers above her Note, a Note of her own nobody else
      // gets and an Amend above both.
      Routines(carol).markDone(bins.id, nine);
      Notes(carol).capture('Private');
      Notes(carol).amend(rude.id, 'Ruder');
      relayToBen();
      exchange(ben, ann);
      ann.blockPerson(idOf(carol));
      expect(Notes(ann).byId(rude.id), isNull);
      final first = by(ben, carol)
          .where((e) => e.entity == rude.id && !keptFromBlocked(e))
          .map((e) => e.dseq)
          .reduce((a, b) => a < b ? a : b);

      ann.unblockPerson(idOf(carol));
      expect(ann.isBlocked(idOf(carol)), isFalse);
      expect(ann.versionVector()[carol.deviceId] ?? 0, lessThan(first));
      exchange(ben, ann);

      expect(Notes(ann).byId(rude.id)?.text, 'Rude');
      final hash = Notes(ann).byId(rude.id)!.photos.single;
      expect(ann.missingBlobs(), contains('photo/$hash'));
      expect(Notes(ann).sections(rude.id).map((s) => s.text), ['Ruder']);
      expect(Routines(ann).byId(bins.id)!.done.keys, [nine.key]);
      expect(ann.refusalMarks(), isEmpty);
      // Her own Note is not waited for: nobody asks for her entries again.
      final again = exchange(ben, ann);
      expect(
        (again.request['entries']! as List).where(
          (e) => (e as Map)['device'] == carol.deviceId,
        ),
        isEmpty,
      );
    });

    test('her Profile loses its picture and goes nowhere; unblocking takes '
        'it in again', () {
      carol.setOwnProfilePicture(squareProfilePicture(halves(40, 30)));
      household();
      final id = idOf(carol);
      expect(ann.personShown(id).picture, isNotNull);
      ann.blockPerson(id);

      expect(ann.personShown(id).picture, isNull);
      expect(ann.personShown(id).name, 'Carol');
      expect(ann.peopleShown.map((p) => p.name), isNot(contains('Carol')));
      expect(ann.blockedPeople.map((p) => p.name), ['Carol']);
      expect(jsonEncode(ann.profilesToSend()), isNot(contains(id)));
      expect(
        jsonEncode(ann.profilesToSend(areas: ann.judged.areas.keys.toSet())),
        isNot(contains(id)),
      );
      expect(ann.takePerson(carol.ownProfileForOthers), PersonTaken.refused);

      ann.unblockPerson(id);
      expect(ann.blockedPeople, isEmpty);
      exchange(carol, ann);
      expect(ann.personShown(id).picture, isNotNull);
      expect(ann.peopleShown.map((p) => p.name), contains('Carol'));
    });

    test('the own Profile cannot be blocked', () {
      expect(() => ann.blockPerson(idOf(ann)), throwsArgumentError);
    });
  });

  group('own Devices', () {
    test('the block reaches the user\'s other Device, and no other person', () {
      final area = household();
      final rude = Notes(carol).capture('Rude', place: area);
      exchange(carol, ann);
      exchange(annDesk, ann);
      expect(Notes(annDesk).byId(rude.id)?.text, 'Rude');

      ann.blockPerson(idOf(carol));
      exchange(annDesk, ann);
      expect(annDesk.isBlocked(idOf(carol)), isTrue);
      expect(Notes(annDesk).byId(rude.id), isNull);
      expect(annDesk.blockedPeople.map((p) => p.name), ['Carol']);

      bool blocks(Store store) =>
          store.allEntries().any((e) => e.entity.startsWith(BlockKeys.prefix));
      final toBen = exchange(ben, ann);
      final toCarol = exchange(carol, ann);
      expect(blocks(ben), isFalse);
      expect(blocks(carol), isFalse);
      for (final wire in [toBen.answer, toCarol.answer]) {
        expect(jsonEncode(wire), isNot(contains(BlockKeys.prefix)));
      }
      final file = sendNote(ann, Notes(ann).capture('Dentist').id);
      expect(utf8.decode(file), isNot(contains(BlockKeys.prefix)));
    });

    test('the block reaches the other Device through the shared folder, and '
        'no other person', () async {
      household();
      ann.blockPerson(idOf(carol));
      final folder = MemorySyncFolder();
      await folderSync(ann, folder);
      await folderSync(annDesk, folder);
      await folderSync(ben, folder);
      expect(annDesk.isBlocked(idOf(carol)), isTrue);
      expect(
        ben.allEntries().any((e) => e.entity.startsWith(BlockKeys.prefix)),
        isFalse,
      );
    });

    test('a block another person\'s Device wrote counts for nothing', () {
      final area = household();
      final rude = Notes(carol).capture('Rude', place: area);
      exchange(carol, ann);
      // As a modified app would: Ben blocks Carol in Ann's log.
      final forged = ben.append(
        '${BlockKeys.prefix}${idOf(carol)}',
        BlockKeys.blocked,
        'Carol',
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(ann.isBlocked(idOf(carol)), isFalse);
      expect(Notes(ann).byId(rude.id)?.text, 'Rude');
    });

    test('a grant naming her for one of the user\'s own Devices blocks '
        'none of it', () {
      final area = household();
      final mine = Notes(annDesk).capture('Mine', place: area);
      exchange(annDesk, ann);
      // As a modified app of a Manager would: the desk named as Carol's.
      ann.append(
        area,
        ShareKeys.role(annDesk.deviceId),
        Grant(AreaRole.member, idOf(carol)).encode(),
      );
      expect(ann.grantsOf(area)!.personOf(annDesk.deviceId), idOf(carol));
      ann.blockPerson(idOf(carol));
      expect(ann.blockedDevices(), {carol.deviceId});
      expect(Notes(ann).byId(mine.id)?.text, 'Mine');
    });
  });

  group('shapes', () {
    Entry entry(String entity, String field, String? value, {String? ctx}) =>
        Entry(
          seq: 0,
          device: 'd',
          dseq: 1,
          entity: entity,
          field: field,
          value: value,
          date: DateTime.utc(2026, 3, 1),
          author: '',
          recorded: DateTime.utc(2026, 3, 1),
          ctx: ctx,
        );

    test('a check-off, a grant and a Note\'s place are kept as the apps '
        'write them', () {
      for (final e in [
        entry(
          'routine:ab12',
          'done@2026-03-01T08:00',
          '2026-03-01T08:00:00.000Z',
        ),
        entry('routine:ab12', 'done@2026-03-01', null),
        entry('frog:ab12', 'done', '2026-03-01T08:00:00.000Z'),
        entry('frog:ab12', 'done', null),
        entry('line:ab12', 'checked', '2026-03-01T08:00:00.000Z'),
        entry('line:ab12', 'checked', null),
        entry(
          'routine:ab12',
          'check@2026-03-01T08:00@line:cd34',
          '2026-03-01T08:00:00.000Z',
        ),
        entry('routine:ab12', 'check@2026-03-01@line:cd34', null),
        entry('area:starter-household', 'role:ab12', null),
        entry(
          'area:ab12',
          'role:ab12',
          const Grant(AreaRole.member, 'cd34').encode(),
        ),
        entry('note:ab12', r'$type', 'note'),
        entry('note:ab12', 'place', 'topic:ab12'),
        entry('note:ab12', 'place', null),
      ]) {
        expect(keptFromBlocked(e), isTrue, reason: '$e');
      }
    });

    test('nothing else, and nothing hidden in them', () {
      final big = 'x' * 65;
      for (final e in [
        entry('note:ab12', 'text', 'Rude'),
        entry('note:ab12', 'captured', '2026-03-01T08:00:00.000Z'),
        entry('note:ab12', 'photo:ab', 'added'),
        entry('note:ab12', 'transcript:ab', 'Rude'),
        entry('section:ab12', r'$type', 'section'),
        entry('routine:ab12', 'done@2026-03-01T08:00', 'Rude words'),
        entry('routine:ab12', 'done@Rude', null),
        entry('routine:ab12', 'done@2026-3-1', null),
        entry('routine:Rude words', 'done@2026-03-01', null),
        entry('routine:$big', 'done@2026-03-01', null),
        entry('frog:ab12', 'done', 'Rude words'),
        entry('frog:ab12', 'trashed', '2026-03-01T08:00:00.000Z'),
        entry('frog:ab12', r'$type', 'frog'),
        entry('frog:Rude words', 'done', null),
        entry('line:ab12', 'checked', 'Rude words'),
        entry('line:ab12', 'text', '{"format":"plain","text":"Rude"}'),
        entry('line:ab12', r'$type', 'line'),
        entry('line:Rude words', 'checked', null),
        entry('routine:ab12', 'check@2026-03-01@line:cd34', 'Rude words'),
        entry('routine:ab12', 'check@Rude@line:cd34', null),
        entry('routine:ab12', 'check@2026-03-01@line:Rude words', null),
        entry('routine:ab12', 'check@2026-03-01@note:cd34', null),
        entry('routine:ab12', 'check@2026-03-01@line:', null),
        entry('routine:ab12', 'check@2026-03-01', null),
        entry('routine:ab12', 'check@@line:cd34', null),
        entry('routine:Rude words', 'check@2026-03-01@line:cd34', null),
        entry(
          'area:ab12',
          'role:ab12',
          '{"profile":"cd34","role":"member","x":1}',
        ),
        entry(
          'area:ab12',
          'role:ab12',
          '{"profile":"cd34","role":"member","v":2}',
        ),
        entry(
          'area:ab12',
          'role:ab12',
          '{"profile":"Rude words","role":"member"}',
        ),
        entry('area:ab12', 'name', 'Rude'),
        entry('note:ab12', r'$type', 'Rude'),
        entry('note:ab12', 'place', 'area:Rude words'),
        entry('note:ab12', 'place', null, ctx: ''),
        Entry(
          seq: 0,
          device: 'd',
          dseq: 1,
          entity: 'note:ab12',
          field: 'place',
          value: null,
          date: DateTime.utc(2026, 3, 1),
          author: 'Rude ' * 10,
          recorded: DateTime.utc(2026, 3, 1),
        ),
        Entry.fromJson({
          'device': 'd',
          'dseq': 1,
          'entity': 'note:ab12',
          'field': 'place',
          'value': null,
          'date': '2026-03-01T00:00:00.000Z',
          'author': '',
          'recorded': '2026-03-01T00:00:00.000Z',
          'x': 'Rude',
        }),
      ]) {
        expect(keptFromBlocked(e), isFalse, reason: '$e');
      }
    });
  });

  group('Managers', () {
    test('blocking a Manager leaves the Area she manages, only when asked', () {
      // Carol shares her own Area with Ann as a Member.
      final school = carol.share(Places(carol).addArea('School').id);
      exchange(ann, carol, inviting: Invite(school, role: AreaRole.member));
      expect(ann.accessIn(school), AreaAccess.member);
      expect(ann.areasToLeaveForBlocking(idOf(carol)), [school]);
      final count = ann.allEntries().length;
      expect(() => ann.blockPerson(idOf(carol)), throwsStateError);
      expect(ann.allEntries(), hasLength(count));
      expect(ann.isBlocked(idOf(carol)), isFalse);

      expect(ann.blockPerson(idOf(carol), leaveAreas: true), [school]);
      expect(ann.isBlocked(idOf(carol)), isTrue);
      expect(ann.accessIn(school), AreaAccess.none);
      expect(ann.areasToLeaveForBlocking(idOf(carol)), isEmpty);
    });

    test('a Member leaves an Area and its Manager sees her gone', () {
      final area = household();
      ben.leaveArea(area);
      expect(ben.accessIn(area), AreaAccess.none);
      exchange(ben, ann);
      expect(ann.grantsOf(area)!.grantOf(ben.deviceId), isNull);
      expect(ann.people(area).map((p) => p.id), isNot(contains(idOf(ben))));
    });

    test('a Member ends no other person\'s grant', () {
      final area = household();
      // As a modified app would: Ben takes Carol out.
      ben.append(area, ShareKeys.role(carol.deviceId), null);
      exchange(ben, ann);
      expect(ann.grantsOf(area)!.grantOf(carol.deviceId), isNotNull);
    });
  });

  group('Sent Items', () {
    test('one from a blocked person is neither shown nor accepted', () {
      final file = sendNote(carol, Notes(carol).capture('Rude').id);
      final opened = openSentItem(ann, file);
      ann.blockPerson(idOf(carol));
      expect(
        () => openSentItem(ann, file),
        throwsA(isA<SentItemFromBlocked>()),
      );
      expect(
        () => acceptSentItem(ann, opened.item),
        throwsA(isA<SentItemFromBlocked>()),
      );
      expect(Notes(ann).all(), isEmpty);
    });

    test('"Decline and block" keeps nothing of it and blocks its sender', () {
      final file = sendNote(carol, Notes(carol).capture('Rude').id);
      final opened = openSentItem(ann, file);
      expect(blockSentItemSender(ann, opened), isTrue);
      expect(Notes(ann).all(), isEmpty);
      expect(ann.isBlocked(idOf(carol)), isTrue);
      expect(ann.blockedPeople.map((p) => p.name), ['Carol']);
      expect(ann.peopleShown, isEmpty);
      expect(
        () => openSentItem(ann, file),
        throwsA(isA<SentItemFromBlocked>()),
      );
    });

    test('a stranger\'s one held until "Show" goes with the block', () {
      final mallory = device(clock, 'Mallory');
      final file = sendNote(mallory, Notes(mallory).capture('Rude').id);
      final held = receiveSentItem(ann, file) as HeldSentItem;
      expect(ann.pendingSets, hasLength(1));
      ann.blockPerson(held.sender.id);
      expect(ann.pendingSets, isEmpty, reason: 'nothing of hers stored');
      expect(heldSentItems(ann), isEmpty);
      expect(
        () => receiveSentItem(ann, file),
        throwsA(isA<SentItemFromBlocked>()),
      );
    });

    test('"Decline and block" on a held one, before "Show"', () {
      final mallory = device(clock, 'Mallory');
      final file = sendNote(mallory, Notes(mallory).capture('Rude').id);
      final held = receiveSentItem(ann, file) as HeldSentItem;
      expect(blockHeldSentItemSender(ann, held.id), isTrue);
      expect(ann.isBlocked(held.sender.id), isTrue);
      expect(ann.blockedPeople.map((p) => p.name), ['Mallory']);
      expect(ann.pendingSets, isEmpty);
      expect(Notes(ann).all(), isEmpty);
    });

    test('"Decline and block" blocks nobody for a copied id', () {
      household();
      // Mallory sends under Carol's id with a key of her own.
      final mallory = device(clock, 'Mallory');
      final opened = openSentItem(
        ann,
        sendNote(carol, Notes(carol).capture('x').id),
      );
      final copied = OpenedSentItem(
        item: opened.item,
        encrypted: false,
        sender: opened.sender,
        code: mallory.ownProfileCode!,
        trust: SenderTrust.unknown,
        accepted: null,
      );
      expect(blockSentItemSender(ann, copied), isFalse);
      expect(ann.isBlocked(idOf(carol)), isFalse);
    });
  });
}
