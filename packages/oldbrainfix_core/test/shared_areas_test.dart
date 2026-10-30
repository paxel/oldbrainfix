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
      hostProfile: invite == null ? null : profileAnswer(host)['profile'],
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
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    annDesk = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
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
    final area = ann.share(house.id);
    exchange(ben, ann, inviting: Invite(area, role: role));
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
      expect(ann.share(house.id), house.id);
      expect(ann.isShared(house.id), isTrue);
      expect(ann.accessIn(house.id), AreaAccess.manager);
      expect(
        ann.accessIn(house.id, device: annDesk.deviceId),
        AreaAccess.manager,
      );
      final people = ann.people(house.id);
      expect(ann.personShown(people.single.id).name, 'Ann');
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
      final id = ann.share(starter);
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
      final id = ann.share(starter);
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
      expect(
        ben.people(h.area).map((p) => (ben.personShown(p.id).name, p.role)),
        [('Ben', AreaRole.member), ('Ann', AreaRole.manager)],
      );
    });

    test('a host invited sends nothing of its own; an inviting joiner '
        'only the Area', () {
      Notes(ben).capture('Ben private');
      final house = Places(ann).addArea('Household');
      Notes(ann).capture('Ann private');
      Notes(ann).capture('Shopping', place: house.id);
      final area = ann.share(house.id);
      final done = exchange(
        ann,
        ben,
        invite: Invite(area, role: AreaRole.member),
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
      final area = ann.share(house.id);
      final done = exchange(
        ben,
        ann,
        inviting: Invite(area, role: AreaRole.member),
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
      final area = ann.share(house.id);
      expect(area, house.id);
      expect(ben.refusesInvite(area), isTrue);
      final request = wire(
        syncRequest(
          ann,
          ben.versionVector(),
          name: 'Phone',
          hostDevice: ben.deviceId,
          invite: Invite(area, role: AreaRole.member),
          hostProfile: profileAnswer(ben)['profile'],
        ),
      );
      expect(answerSync(ben, request, name: 'Desk'), isNull);
    });

    test('an own Device is not invited, either way', () async {
      final house = Places(ann).addArea('Household');
      exchange(annDesk, ann);
      final area = ann.share(house.id);
      final invite = Invite(area, role: AreaRole.member);
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
          hostProfile: profileAnswer(annDesk)['profile'],
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
      expect(ann.personShown(ann.people(area).single.id).name, 'Ann');
    });

    test('a host inviting answers only a joiner that knows it', () {
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id);
      final request = wire(
        syncRequest(ben, ann.versionVector(), name: 'Phone'),
      );
      expect(
        answerSync(
          ann,
          request,
          name: 'Desk',
          inviting: Invite(area, role: AreaRole.member),
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

    test('a Member changes the status marks of their own Notes only, as '
        'any text (ADR 0031)', () {
      final h = household(AreaRole.member);
      Notes(ann).edit(h.note, 'Buy milk\n☐ oat');
      exchange(ben, ann);
      final own = Notes(ben).capture('Bread\n☐ rye', place: h.topic);
      expect(ben.mayChange(h.note), isFalse);
      for (final (id, text) in [
        (h.note, Notes(ben).byId(h.note)!.text),
        (own.id, own.text),
      ]) {
        final swapped = TextMarks.swap(text, 1, TextMark.checked);
        expect(swapped, isNotNull);
        Notes(ben).edit(id, swapped ?? text);
      }
      exchange(ben, ann);
      for (final store in [ann, ben]) {
        expect(Notes(store).byId(h.note)!.shownText, 'Buy milk\n☐ oat');
        expect(Notes(store).byId(own.id)!.shownText, 'Bread\n✅ rye');
      }
    });

    test('a tick another person\'s Device wrote on a Note in no Shared '
        'Area counts for nothing (ADR 0035)', () {
      final h = household(AreaRole.member);
      Notes(ann).edit(h.secret, '☐ pills');
      final id = Notes(ann).byId(h.secret)!.boxes.single.id;
      // As a modified app would.
      final forged = ben.append(h.secret, BoxKeys.tick(id), 'done');
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(Notes(ann).byId(h.secret)!.shownText, '☐ pills');
      expect(Boxes(ann).history(h.secret, id), isEmpty);
    });

    test('a Member ticks the boxes of any Note and Section, while their '
        'texts stay the author\'s (ADR 0035)', () {
      final h = household(AreaRole.member);
      Notes(ann).edit(h.note, 'Buy milk\n☐ oat');
      final eggs = Notes(ann).amend(h.note, '☐ ask about eggs');
      exchange(ben, ann);
      final box = Notes(ben).byId(h.note)!.boxes.single;
      final inSection = Notes(ben).section(eggs.id)!.boxes.single;
      expect(ben.mayChange(h.note), isFalse);
      expect(Boxes(ben).mayTick(h.note), isTrue);
      expect(Boxes(ben).mayTick(eggs.id), isTrue);
      Boxes(ben).tick(h.note, box.id, BoxState.done);
      Boxes(ben).tick(eggs.id, inSection.id, BoxState.failed);
      exchange(ben, ann);
      for (final store in [ann, ben]) {
        final note = Notes(store).byId(h.note)!;
        expect(note.shownText, 'Buy milk\n✅ oat');
        expect(note.boxes.single.starting, BoxState.open);
        expect(note.boxes.single.last?.device, ben.deviceId);
        expect(Notes(store).section(eggs.id)!.shownText, '❌ ask about eggs');
        expect(Boxes(store).history(eggs.id, inSection.id).length, 1);
      }
      final tick = Notes(ann).byId(h.note)!.boxes.single.last!;
      expect(tick.own, isFalse);
      expect(ann.tickedBy(h.note, tick)?.name, 'Ben');
      expect(Notes(ben).byId(h.note)!.boxes.single.last!.own, isTrue);
      expect(ben.tickedBy(h.note, tick), isNull);
    });

    test('a Manager does everything, and Managers change roles', () {
      final h = household(AreaRole.member);
      Notes(ben).edit(h.note, 'Too early');
      final person = ann
          .people(h.area)
          .firstWhere((p) => ann.personShown(p.id).name == 'Ben');
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
      final mine = ben
          .people(h.area)
          .firstWhere((p) => ben.personShown(p.id).name == 'Ann');
      ben.setRole(h.area, mine.id, AreaRole.member);
      exchange(ben, ann);
      expect(ann.accessIn(h.area), AreaAccess.manager);
    });

    test('removing a person keeps what they wrote and stops sending', () {
      final h = household(AreaRole.member);
      final own = Notes(ben).capture('Ben was here', place: h.area);
      exchange(ben, ann);
      final person = ann
          .people(h.area)
          .firstWhere((p) => ann.personShown(p.id).name == 'Ben');
      ann.removePerson(h.area, person.id);
      expect(ann.people(h.area).map((p) => ann.personShown(p.id).name), [
        'Ann',
      ]);
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
          ben
              .people(h.area)
              .firstWhere((p) => ben.personShown(p.id).name == 'Ben')
              .devices,
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
      final area = ann.share(house.id);
      final host = TestHost(ann)
        ..inviting = Invite(area, role: AreaRole.member);
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
      final garden = ann.share(other.id);
      Notes(ann).capture('Mow', place: garden);
      final benHost = TestHost(ben);
      await benHost.start();
      try {
        await lanSync(
          ann,
          decodePairCode(benHost.code())!,
          name: 'Phone',
          invite: Invite(garden, role: AreaRole.manager),
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
      final area = ann.share(house.id);
      final host = TestHost(ben)
        ..inviting = Invite('area:x', role: AreaRole.member);
      await host.start();
      try {
        await expectLater(
          lanSync(
            ann,
            decodePairCode(host.code())!,
            name: 'Phone',
            invite: Invite(area, role: AreaRole.member),
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

  group('grants of a later version (ADR 0034)', () {
    test('a grant reads as newer by its version, its keys or its role', () {
      for (final (value, role) in [
        ('{"profile":"p","role":"member"}', AreaRole.member),
        ('{"profile":"p","role":"manager","v":1}', AreaRole.manager),
      ]) {
        expect(Grant.parse(value)?.role, role, reason: value);
      }
      for (final value in [
        '{"profile":"p","role":"viewer"}',
        '{"profile":"p","role":"member","v":2}',
        '{"profile":"p","role":"member","v":1.5}',
        '{"profile":"p","role":"member","v":"1"}',
        '{"profile":"p","role":"member","v":0}',
        '{"profile":"p","role":"member","v":null}',
        '{"profile":"p","role":"member","until":"2027"}',
        '{"profile":"p","role":3}',
        '{"profile":"p"}',
      ]) {
        final grant = Grant.parse(value);
        expect(grant?.newer, isTrue, reason: value);
        expect(grant?.profile, 'p', reason: value);
      }
      for (final value in [
        '{"profile":"","role":"member"}',
        '{"profile":3,"role":"member"}',
        '{"role":"member"}',
        '["p","member"]',
        'member',
        '',
      ]) {
        expect(Grant.parse(value), isNull, reason: value);
      }
      expect(() => const Grant(null, 'p').encode(), throwsStateError);
    });

    test('a person whose grant is newer keeps getting the Area, may write '
        'nothing into it, and nothing here changes or ends the grant', () {
      final h = household(AreaRole.member);
      final benId = ben.ownProfile!.id;
      // As Ann's Device of a later version writes it.
      ann.append(
        h.area,
        ShareKeys.role(ben.deviceId),
        jsonEncode({'profile': benId, 'role': 'viewer', 'v': 2}),
      );
      exchange(ben, ann);
      final own = Notes(ben).byId(h.note)!;
      ben.batch(() {
        ben.append(own.id, NoteKeys.text, StoredText.plain('Beer').encode());
        ben.append('note:ben', NoteKeys.type, NoteKeys.typeNote);
        ben.append('note:ben', NoteKeys.place, h.area);
      });
      exchange(ben, ann);
      for (final store in [ann, ben]) {
        final person = store.people(h.area).firstWhere((p) => p.id == benId);
        expect(person.newer, isTrue);
        expect(person.role, isNull);
        expect(store.areasSharedWith(ben.deviceId), {h.area});
        expect(store.accessIn(h.area, device: ben.deviceId), AreaAccess.none);
        expect(Notes(store).byId(h.note)!.text, 'Buy milk');
        expect(Notes(store).byId('note:ben'), isNull);
      }
      expect(ben.mayWriteIn(h.area), isFalse);
      final before = ann.history(h.area).length;
      expect(
        () => ann.setRole(h.area, benId, AreaRole.manager),
        throwsStateError,
      );
      expect(() => ann.removePerson(h.area, benId), throwsStateError);
      ann.invite(h.area, ben.deviceId, role: AreaRole.member, profile: benId);
      expect(ann.history(h.area), hasLength(before));
      // He still gets what Ann writes.
      Notes(ann).capture('For Ben to read', place: h.area);
      exchange(ben, ann);
      expect(Notes(ben).all().map((n) => n.text), contains('For Ben to read'));
    });

    test('a person whose grant is newer adds no Device of theirs, and an own '
        'Device met in a sync is given nothing', () {
      final h = household(AreaRole.member);
      final benId = ben.ownProfile!.id;
      ann.append(
        h.area,
        ShareKeys.role(ben.deviceId),
        jsonEncode({'profile': benId, 'role': 'viewer'}),
      );
      exchange(ben, ann);
      // As a Member adds an own Device, by hand as a modified app would.
      ben.append(
        h.area,
        ShareKeys.role('laptop'),
        Grant(AreaRole.member, benId).encode(),
      );
      exchange(ben, ann);
      for (final store in [ann, ben]) {
        expect(store.grantsOf(h.area)?.grantOf('laptop'), isNull);
        expect(store.accessIn(h.area, device: 'laptop'), AreaAccess.none);
      }
      final benLaptop = Store.inMemory(clock: clock);
      try {
        exchange(benLaptop, ben);
        expect(ben.grantsOf(h.area)?.everGranted(benLaptop.deviceId), isFalse);
      } finally {
        benLaptop.close();
      }
    });
  });
}
