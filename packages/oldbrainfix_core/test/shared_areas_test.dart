import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'test_host.dart';

/// A clock shared by every store of a test, one second per read, so
/// Devices write in a known order.
DateTime Function() ticking() {
  var now = DateTime.utc(2026, 3, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// What one LAN session put on the wire, and what each side took in.
typedef Exchange = ({
  Map<String, Object?> request,
  Map<String, Object?> answer,
  SyncSession host,
  SyncSession joiner,
});

/// One LAN session as the joiner and the desktop host run it, without
/// the network: [joiner] joins [host]; with [invite] the joiner invites
/// the host, with [inviting] the host invites the joiner. Both remember
/// each other afterwards, as the apps do.
Exchange exchange(
  Store joiner,
  Store host, {
  Invite? invite,
  Invite? inviting,
}) {
  final invited = inviting != null;
  final shared =
      invited || invite != null || joiner.isSharedPeer(host.deviceId);
  final hostShared =
      invited || invite != null || host.isSharedPeer(joiner.deviceId);
  final request = wire(
    syncRequest(
      joiner,
      host.versionVector(),
      name: 'Phone',
      hostDevice: host.deviceId,
      invited: invited,
      invite: invite,
    ),
  );
  final done = answerSync(host, request, name: 'Desk', inviting: inviting)!;
  final answer = wire(done.answer);
  final back = applyAnswer(
    joiner,
    answer,
    shared: shared,
    invitedBy: invite?.areaId,
  );
  final at = DateTime.utc(2026, 3, 1);
  host.rememberDevice(
    PairedDevice(
      id: joiner.deviceId,
      name: 'Phone',
      fingerprint: '',
      host: null,
      port: null,
      lastSync: at,
      shared: hostShared,
    ),
  );
  joiner.rememberDevice(
    PairedDevice(
      id: host.deviceId,
      name: 'Desk',
      fingerprint: 'ab' * 32,
      host: '192.168.1.2',
      port: 4000,
      lastSync: at,
      shared: shared,
    ),
  );
  return (request: request, answer: answer, host: done.session, joiner: back);
}

List<String> entities(Map<String, Object?> payload) =>
    {for (final e in payload['entries'] as List) (e as Map)['entity'] as String}
        .toList()
      ..sort();

List<String> scopes(Map<String, Object?> payload) => [
  for (final k in payload['areaKeys'] as List) (k as Map)['scope'] as String,
];

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store annDesk;
  late Store ben;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock);
    annDesk = Store.inMemory(clock: clock);
    ben = Store.inMemory(clock: clock);
  });

  tearDown(() {
    ann.close();
    annDesk.close();
    ben.close();
  });

  /// Ann's household, with a Topic, a Note in it and a Routine, and a
  /// private Area beside it; shared with Ben as [role].
  ({String area, String topic, String note, String routine, String secret})
  household(AreaRole role) {
    final places = Places(ann);
    final house = places.addArea('Household');
    final kitchen = places.addTopic(house.id, 'Kitchen');
    final milk = Notes(ann).capture('Buy milk', place: kitchen.id);
    final bins = Routines(ann).spawn(
      milk.id,
      Schedule(
        start: Day(2026, 3, 1),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    final health = places.addArea('Health');
    final secret = Notes(ann).capture('Blood results', place: health.id);
    Notes(ann).capture('Unsorted thought');
    final area = ann.share(house.id, name: 'Ann');
    exchange(
      ben,
      ann,
      inviting: Invite(area, role: role, name: 'Ben'),
    );
    return (
      area: area,
      topic: kitchen.id,
      note: milk.id,
      routine: bins.id,
      secret: secret.id,
    );
  }

  group('sharing', () {
    test('the first to share is its first Manager, with its own Devices', () {
      final house = Places(ann).addArea('Household');
      exchange(annDesk, ann);
      expect(ann.isShared(house.id), isFalse);
      expect(ann.accessIn(house.id), AreaAccess.owner);
      expect(ann.share(house.id, name: 'Ann'), house.id);
      expect(ann.isShared(house.id), isTrue);
      expect(ann.accessIn(house.id), AreaAccess.manager);
      expect(
        ann.accessIn(house.id, device: annDesk.deviceId),
        AreaAccess.manager,
      );
      final people = ann.people(house.id);
      expect(people.single.name, 'Ann');
      expect(
        people.single.devices,
        unorderedEquals([ann.deviceId, annDesk.deviceId]),
      );
    });

    test('a Starter Set Area moves to fresh ids before it is shared', () {
      Places(ann).plantStarterSet({'household', 'kitchen'});
      final starter = Places.starterId(PlaceKeys.typeArea, 'household');
      final kitchen = Places.starterId(PlaceKeys.typeTopic, 'kitchen');
      final note = Notes(ann).capture('Descale the kettle', place: kitchen);
      final trashed = Notes(ann).capture('Old', place: starter);
      Notes(ann).trash(trashed.id);
      final id = ann.share(starter, name: 'Ann');
      expect(id, isNot(starter));
      expect(Places(ann).areaById(starter), isNull);
      final area = Places(ann).areaById(id)!;
      expect(area.starter, 'household');
      final topic = Places(ann).topics(id).single;
      expect(topic.starter, 'kitchen');
      expect(topic.id, isNot(kitchen));
      expect(Notes(ann).byId(note.id)!.place, topic.id);
      expect(Notes(ann).byId(trashed.id)!.place, id);
      // Planting again leaves the old ids alone.
      Places(ann).plantStarterSet({'household'});
      expect(Places(ann).areas().map((a) => a.id), [id]);
    });

    test('an Area Lock follows the Area to its fresh id', () {
      ann.setPin('1234', iterations: 1);
      Places(ann).plantStarterSet({'health'});
      final starter = Places.starterId(PlaceKeys.typeArea, 'health');
      ann.setAreaLocked(starter, true);
      final id = ann.share(starter, name: 'Ann');
      expect(ann.isAreaLocked(id), isTrue);
      expect(ann.isAreaMasked(id), isTrue);
    });

    test('the invited Device gets that Area, its key, and nothing else', () {
      final h = household(AreaRole.member);
      expect(ben.isSharedPeer(ann.deviceId), isTrue);
      expect(ann.isSharedPeer(ben.deviceId), isTrue);
      expect(Places(ben).areas().map((a) => a.name), ['Household']);
      expect(Notes(ben).byId(h.note)!.text, 'Buy milk');
      expect(Routines(ben).byId(h.routine), isNotNull);
      expect(Notes(ben).byId(h.secret), isNull);
      expect(Notes(ben).all().map((n) => n.text), ['Buy milk']);
      expect([for (final k in ben.areaKeys()) k.scope], contains(h.area));
      expect([
        for (final k in ben.areaKeys()) k.id,
      ], isNot(contains(ann.writingKey(personalScope)!.id)));
      expect(ben.accessIn(h.area), AreaAccess.member);
      expect(ben.people(h.area).map((p) => (p.name, p.role)), [
        ('Ben', AreaRole.member),
        ('Ann', AreaRole.manager),
      ]);
    });

    test('a host invited sends nothing of its own; an inviting joiner '
        'only the Area', () {
      Notes(ben).capture('Ben private');
      final house = Places(ann).addArea('Household');
      Notes(ann).capture('Ann private');
      Notes(ann).capture('Shopping', place: house.id);
      final area = ann.share(house.id, name: 'Ann');
      final done = exchange(
        ann,
        ben,
        invite: Invite(area, role: AreaRole.member, name: 'Ben'),
      );
      expect(done.request['invite'], {'area': area});
      expect(
        entities(done.request),
        [
          area,
          ...{for (final n in Notes(ann).inside(area)) n.id},
        ]..sort(),
      );
      expect(scopes(done.request), [area]);
      // Ben answers with nothing of his: he holds nothing of the Area.
      expect(done.answer['entries'], isEmpty);
      expect(scopes(done.answer), [area]);
      expect(done.host.sharedArea, area);
      expect(done.joiner.sharedArea, area);
      expect(
        Notes(ben).all().map((n) => n.text),
        unorderedEquals(['Ben private', 'Shopping']),
      );
      expect(
        Notes(ann).all().map((n) => n.text),
        unorderedEquals(['Ann private', 'Shopping']),
      );
    });

    test('the other way round the joiner sends nothing at all', () {
      Notes(ben).capture('Ben private');
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id, name: 'Ann');
      final done = exchange(
        ben,
        ann,
        inviting: Invite(area, role: AreaRole.member, name: 'Ben'),
      );
      expect(done.request['invited'], isTrue);
      expect(done.request['entries'], isEmpty);
      expect(done.request['areaKeys'], isEmpty);
      expect(Notes(ann).all(), isEmpty);
      expect(scopes(done.answer), [area]);
    });

    test('an invite to an Area held as one\'s own is refused', () {
      final house = Places(ann).addArea('Household');
      for (final e in ann.allEntries()) {
        ben.applyEntries([e], keys: ann.keyRecords());
      }
      final area = ann.share(house.id, name: 'Ann');
      expect(area, house.id);
      expect(ben.refusesInvite(area), isTrue);
      final request = wire(
        syncRequest(
          ann,
          ben.versionVector(),
          name: 'Phone',
          hostDevice: ben.deviceId,
          invite: Invite(area, role: AreaRole.member, name: 'Ben'),
        ),
      );
      expect(answerSync(ben, request, name: 'Desk'), isNull);
    });

    test('an own Device is not invited, either way', () async {
      final house = Places(ann).addArea('Household');
      exchange(annDesk, ann);
      final area = ann.share(house.id, name: 'Ann');
      final invite = Invite(area, role: AreaRole.member, name: 'Ben');
      final asked = wire(
        syncRequest(
          annDesk,
          ann.versionVector(),
          name: 'Desk',
          hostDevice: ann.deviceId,
          invited: true,
        ),
      );
      expect(answerSync(ann, asked, name: 'Phone', inviting: invite), isNull);
      final inviting = wire(
        syncRequest(
          ann,
          annDesk.versionVector(),
          name: 'Phone',
          hostDevice: annDesk.deviceId,
          invite: invite,
        ),
      );
      expect(answerSync(annDesk, inviting, name: 'Desk'), isNull);
      // The joiner refuses before anything is sent.
      final host = TestHost(annDesk);
      await host.start();
      try {
        await expectLater(
          lanSync(
            ann,
            decodePairCode(host.code())!,
            name: 'Phone',
            invite: invite,
          ),
          throwsA(
            isA<SyncException>().having(
              (e) => e.failure,
              'failure',
              SyncFailure.declined,
            ),
          ),
        );
        expect(host.bodies, isEmpty);
      } finally {
        await host.stop();
      }
      expect(ann.isSharedPeer(annDesk.deviceId), isFalse);
      expect(ann.people(area).single.name, 'Ann');
    });

    test('a host inviting answers only a joiner that knows it', () {
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id, name: 'Ann');
      final request = wire(
        syncRequest(ben, ann.versionVector(), name: 'Phone'),
      );
      expect(
        answerSync(
          ann,
          request,
          name: 'Desk',
          inviting: Invite(area, role: AreaRole.member, name: 'Ben'),
        ),
        isNull,
      );
    });
  });

  group('roles', () {
    test('a Member adds and changes their own Notes, Amends any, checks '
        'off; everything else of theirs is ignored on every Device', () {
      final h = household(AreaRole.member);
      final own = Notes(ben).capture('Ben buys bread', place: h.topic);
      Notes(ben).edit(own.id, 'Ben buys rye bread');
      Notes(ben).amend(h.note, 'Oat milk please');
      final occurrence = Occurrence(Day(2026, 3, 2), ClockTime(8, 0));
      Routines(ben).markDone(h.routine, occurrence);
      // Not his to do: none of it counts.
      Notes(ben).edit(h.note, 'Buy beer');
      Notes(ben).trash(h.note);
      Routines(ben).reschedule(
        h.routine,
        Schedule(start: Day(2026, 3, 1), times: [ClockTime(6, 0)]),
      );
      Places(ben).rename(h.area, 'Ben\'s');
      expect(() => Places(ben).addTopic(h.area, 'Garage'), throwsStateError);
      expect(
        () => Routines(ben).spawn(own.id, Schedule(start: Day(2026, 3, 5))),
        throwsStateError,
      );
      // As a modified app would write them all the same.
      ben.batch(() {
        ben.append('topic:garage', PlaceKeys.type, PlaceKeys.typeTopic);
        ben.append('topic:garage', PlaceKeys.area, h.area);
        ben.append('topic:garage', PlaceKeys.name, 'Garage');
        ben.append('routine:ben', RoutineKeys.type, RoutineKeys.typeRoutine);
        ben.append('routine:ben', RoutineKeys.note, own.id);
        ben.append('routine:ben', RoutineKeys.start, '2026-03-05');
      });
      exchange(ben, ann);
      for (final store in [ann, ben]) {
        final milk = Notes(store).byId(h.note)!;
        expect(milk.text, 'Buy milk');
        expect(milk.isTrashed, isFalse);
        expect(Notes(store).byId(own.id)!.text, 'Ben buys rye bread');
        expect(Notes(store).sections(h.note).map((s) => s.text), [
          'Oat milk please',
        ]);
        final routine = Routines(store).byId(h.routine)!;
        expect(routine.schedule.times, [ClockTime(8, 0)]);
        expect(routine.doneAt(occurrence), isNotNull);
        expect(Places(store).areaById(h.area)!.name, 'Household');
        expect(Places(store).topics(h.area).map((t) => t.name), ['Kitchen']);
        expect(Routines(store).ofNote(own.id), isEmpty);
      }
      expect(ben.mayChange(own.id), isTrue);
      expect(ben.mayChange(h.note), isFalse);
      expect(ben.mayChangeRoutine(h.routine), isFalse);
      expect(ben.mayCheckOff(h.routine), isTrue);
      expect(ben.mayManage(h.area), isFalse);
      expect(ann.mayChange(own.id), isTrue);
      expect(ann.mayManage(h.area), isTrue);
    });

    test('a Manager does everything, and Managers change roles', () {
      final h = household(AreaRole.member);
      Notes(ben).edit(h.note, 'Too early');
      final person = ann.people(h.area).firstWhere((p) => p.name == 'Ben');
      exchange(ben, ann);
      ann.setRole(h.area, person.id, AreaRole.manager);
      exchange(ben, ann);
      expect(ben.accessIn(h.area), AreaAccess.manager);
      Notes(ben).edit(h.note, 'Buy oat milk');
      Places(ben).rename(h.area, 'Home');
      exchange(ben, ann);
      for (final store in [ann, ben]) {
        expect(Notes(store).byId(h.note)!.text, 'Buy oat milk');
        expect(Places(store).areaById(h.area)!.name, 'Home');
      }
      // What he wrote as a Member stays ignored.
      expect(
        ann.history(h.note).map((e) => e.value),
        isNot(contains('Too early')),
      );
      // A Member cannot hand out roles.
      ann.setRole(h.area, person.id, AreaRole.member);
      exchange(ben, ann);
      final mine = ben.people(h.area).firstWhere((p) => p.name == 'Ann');
      ben.setRole(h.area, mine.id, AreaRole.member);
      exchange(ben, ann);
      expect(ann.accessIn(h.area), AreaAccess.manager);
    });

    test('removing a person keeps what they wrote and stops sending', () {
      final h = household(AreaRole.member);
      final own = Notes(ben).capture('Ben was here', place: h.area);
      exchange(ben, ann);
      final person = ann.people(h.area).firstWhere((p) => p.name == 'Ben');
      ann.removePerson(h.area, person.id);
      expect(ann.people(h.area).map((p) => p.name), ['Ann']);
      expect(ann.areasSharedWith(ben.deviceId), isEmpty);
      Notes(ann).capture('After Ben', place: h.area);
      Notes(ben).edit(own.id, 'Ben after removal');
      final done = exchange(ben, ann);
      expect(done.answer['entries'], isEmpty);
      expect(done.answer['areaKeys'], isEmpty);
      expect(Notes(ann).byId(own.id)!.text, 'Ben was here');
    });

    test('an own Device met in a sync gets the same role', () {
      final h = household(AreaRole.member);
      final benLaptop = Store.inMemory(clock: clock);
      try {
        exchange(benLaptop, ben);
        expect(
          ben.accessIn(h.area, device: benLaptop.deviceId),
          AreaAccess.member,
        );
        expect(
          ben.people(h.area).firstWhere((p) => p.name == 'Ben').devices,
          hasLength(2),
        );
        // Gone with the next sync; Ann takes it from either.
        exchange(benLaptop, ben);
        Notes(benLaptop).amend(h.note, 'From the laptop');
        exchange(benLaptop, ben);
        exchange(ben, ann);
        expect(Notes(ann).sections(h.note).map((s) => s.text), [
          'From the laptop',
        ]);
      } finally {
        benLaptop.close();
      }
    });
  });

  group('sending', () {
    test('a Note filed into the Area later goes over whole', () {
      final h = household(AreaRole.member);
      final loose = Notes(ann).capture('Fix the tap');
      Notes(ann).amend(loose.id, 'The hot one');
      exchange(ben, ann);
      expect(Notes(ben).byId(loose.id), isNull);
      Notes(ann).file(loose.id, h.topic);
      Notes(ann).capture('Still private');
      exchange(ben, ann);
      expect(Notes(ben).byId(loose.id)!.text, 'Fix the tap');
      expect(Notes(ben).sections(loose.id).map((s) => s.text), ['The hot one']);
      expect(
        Notes(ben).all().map((n) => n.text),
        unorderedEquals(['Buy milk', 'Fix the tap']),
      );
    });

    test('a bundle for the other person holds only the Area', () async {
      final h = household(AreaRole.member);
      Notes(ann).capture('New private');
      final blob = Notes(ann).capture(
        'Photo inside',
        place: h.area,
        photos: [Uint8List.fromList(List.filled(40, 3))],
      );
      final dir = Directory.systemTemp.createTempSync('shared_bundle_');
      try {
        final path = '${dir.path}/b.$bundleExtension';
        final wrote = await writeBundle(ann, path, forDevice: ben.deviceId);
        expect(wrote!.blobs, 1);
        final read = await importBundle(ben, path);
        // Nothing in it is sealed with a key Ben lacks: it holds the Area
        // only.
        expect(read.unreadable, 0);
        expect(Notes(ben).byId(blob.id)!.photos, hasLength(1));
        expect(
          Notes(ben).all().map((n) => n.text),
          unorderedEquals(['Buy milk', 'Photo inside']),
        );
      } finally {
        dir.deleteSync(recursive: true);
      }
    });

    test('through the shared folder the other person reads only the Area, '
        'and a Note filed later whole', () async {
      final h = household(AreaRole.member);
      final loose = Notes(ann).capture('Fix the tap');
      final folder = MemorySyncFolder();
      await folderSync(ann, folder);
      Notes(ann).file(loose.id, h.topic);
      await folderSync(ann, folder);
      final read = await folderSync(ben, folder);
      expect(read.unreadable, greaterThan(0));
      expect(Notes(ben).byId(loose.id)!.text, 'Fix the tap');
      expect(Notes(ben).byId(h.secret), isNull);
      expect(
        Notes(ben).all().map((n) => n.text),
        unorderedEquals(['Buy milk', 'Fix the tap']),
      );
      // And what Ben writes reaches Ann the same way.
      Notes(ben).amend(h.note, 'From Ben');
      await folderSync(ben, folder);
      await folderSync(ann, folder);
      expect(Notes(ann).sections(h.note).map((s) => s.text), ['From Ben']);
    });

    test('blobs go only to and from the Area', () {
      final h = household(AreaRole.member);
      final inside = Notes(ann).capture(
        'In',
        place: h.area,
        photos: [Uint8List.fromList(List.filled(10, 1))],
      );
      final outside = Notes(ann)
          .capture('Out', photos: [Uint8List.fromList(List.filled(10, 2))]);
      final areas = ann.areasSharedWith(ben.deviceId);
      expect(ann.blobIn('photo/${inside.photos.single}', areas), isTrue);
      expect(ann.blobIn('photo/${outside.photos.single}', areas), isFalse);
    });
  });

  group('over the network', () {
    test('a host invites a joiner, and a joiner invites a host', () async {
      final house = Places(ann).addArea('Household');
      Notes(ann).capture('Shopping', place: house.id);
      Notes(ann).capture('Ann private');
      Notes(ben).capture('Ben private');
      final area = ann.share(house.id, name: 'Ann');
      final host = TestHost(ann)
        ..inviting = Invite(area, role: AreaRole.member, name: 'Ben');
      await host.start();
      try {
        final result = await lanSync(
          ben,
          decodePairCode(host.code())!,
          name: 'Phone',
        );
        expect(result.session.sharedArea, area);
        expect(result.entriesSent, 0);
        expect(ben.isSharedPeer(ann.deviceId), isTrue);
        expect(ann.isSharedPeer(ben.deviceId), isTrue);
        expect(
          Notes(ben).all().map((n) => n.text),
          unorderedEquals(['Ben private', 'Shopping']),
        );
        expect(
          Notes(ann).all().map((n) => n.text),
          unorderedEquals(['Ann private', 'Shopping']),
        );
      } finally {
        await host.stop();
      }

      final other = Places(ann).addArea('Garden');
      final garden = ann.share(other.id, name: 'Ann');
      Notes(ann).capture('Mow', place: garden);
      final benHost = TestHost(ben);
      await benHost.start();
      try {
        await lanSync(
          ann,
          decodePairCode(benHost.code())!,
          name: 'Phone',
          invite: Invite(garden, role: AreaRole.manager, name: 'Ben'),
        );
        expect(ben.accessIn(garden), AreaAccess.manager);
        expect(
          Notes(ben).all().map((n) => n.text),
          unorderedEquals(['Ben private', 'Shopping', 'Mow']),
        );
        expect(
          [for (final k in ben.areaKeys()) k.scope]..sort(),
          unorderedEquals([personalScope, area, garden]),
        );
      } finally {
        await benHost.stop();
      }
    });

    test('two Devices inviting each other get nowhere', () async {
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id, name: 'Ann');
      final host = TestHost(ben)
        ..inviting = Invite('area:x', role: AreaRole.member, name: 'Ann');
      await host.start();
      try {
        await expectLater(
          lanSync(
            ann,
            decodePairCode(host.code())!,
            name: 'Phone',
            invite: Invite(area, role: AreaRole.member, name: 'Ben'),
          ),
          throwsA(
            isA<SyncException>().having(
              (e) => e.failure,
              'failure',
              SyncFailure.declined,
            ),
          ),
        );
        expect(host.bodies, isEmpty);
      } finally {
        await host.stop();
      }
    });
  });
}
