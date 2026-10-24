import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

const zone = 'Europe/Berlin';

CalendarEvent event(String uid, {String summary = 'Pills', String? rule}) =>
    CalendarEvent(
      uid: uid,
      summary: summary,
      start: Occurrence(Day(2026, 10, 24), ClockTime(8, 0)),
      rule: rule,
    );

MirroredEvent known(String id, CalendarEvent event, [String z = zone]) =>
    MirroredEvent(id, mirrorPrint(event, z));

void main() {
  group('planMirror', () {
    test('an empty calendar gets every event', () {
      final a = event('a@oldbrainfix');
      final b = event('b@oldbrainfix');
      final plan = planMirror(
        desired: [a, b],
        remembered: const {},
        marked: const {},
        zone: zone,
      );
      expect(plan.create, [a, b]);
      expect(plan.update, isEmpty);
      expect(plan.delete, isEmpty);
      expect(plan.isEmpty, isFalse);
    });

    test('an event as it was written stays as it is', () {
      final a = event('a@oldbrainfix');
      final plan = planMirror(
        desired: [a],
        remembered: {a.uid: known('7', a)},
        marked: {'7': a.uid},
        zone: zone,
      );
      expect(plan.isEmpty, isTrue);
      expect(plan.keep, {a.uid: known('7', a)});
    });

    test('a changed event is written again in place', () {
      final before = event('a@oldbrainfix');
      final after = event('a@oldbrainfix', summary: 'Vitamins');
      final plan = planMirror(
        desired: [after],
        remembered: {before.uid: known('7', before)},
        marked: {'7': before.uid},
        zone: zone,
      );
      expect(plan.update, [('7', after)]);
      expect(plan.create, isEmpty);
      expect(plan.delete, isEmpty);
    });

    test('a changed repeat rule is a change', () {
      final before = event('a@oldbrainfix', rule: 'FREQ=DAILY');
      final after = event('a@oldbrainfix', rule: 'FREQ=WEEKLY');
      final plan = planMirror(
        desired: [after],
        remembered: {before.uid: known('7', before)},
        marked: {'7': before.uid},
        zone: zone,
      );
      expect(plan.update, [('7', after)]);
    });

    test('a new Device zone writes every event again', () {
      final a = event('a@oldbrainfix');
      final plan = planMirror(
        desired: [a],
        remembered: {a.uid: known('7', a, 'Europe/London')},
        marked: {'7': a.uid},
        zone: zone,
      );
      expect(plan.update, [('7', a)]);
    });

    test('an event no longer desired is removed', () {
      final a = event('a@oldbrainfix');
      final plan = planMirror(
        desired: const [],
        remembered: {a.uid: known('7', a)},
        marked: {'7': a.uid},
        zone: zone,
      );
      expect(plan.delete, ['7']);
      expect(plan.keep, isEmpty);
    });

    test('a remembered event gone from the calendar is written anew', () {
      final a = event('a@oldbrainfix');
      final plan = planMirror(
        desired: [a],
        remembered: {a.uid: known('7', a)},
        marked: const {},
        zone: zone,
      );
      expect(plan.create, [a]);
      expect(plan.delete, isEmpty);
    });

    test('a marked event nothing remembers is taken over', () {
      final a = event('a@oldbrainfix');
      final plan = planMirror(
        desired: [a],
        remembered: const {},
        marked: {'9': a.uid},
        zone: zone,
      );
      expect(plan.update, [('9', a)]);
      expect(plan.create, isEmpty);
    });

    test('a second marked event of the same UID goes', () {
      final a = event('a@oldbrainfix');
      final plan = planMirror(
        desired: [a],
        remembered: {a.uid: known('7', a)},
        marked: {'6': a.uid, '7': a.uid, '8': a.uid},
        zone: zone,
      );
      expect(plan.keep.keys, [a.uid]);
      expect(plan.delete, unorderedEquals(['6', '8']));
    });

    test('only marked events are ever updated or removed', () {
      final a = event('a@oldbrainfix');
      final b = event('b@oldbrainfix');
      // "7" is remembered, but the calendar no longer marks it as the
      // mirror's: it is not touched, the event is written anew.
      final plan = planMirror(
        desired: [a],
        remembered: {a.uid: known('7', a), b.uid: known('8', b)},
        marked: const {},
        zone: zone,
      );
      expect(plan.create, [a]);
      expect(plan.update, isEmpty);
      expect(plan.delete, isEmpty);
    });

    test('nothing desired removes every marked event', () {
      final plan = planMirror(
        desired: const [],
        remembered: const {},
        marked: {'1': 'a@oldbrainfix', '2': 'b@oldbrainfix'},
        zone: zone,
      );
      expect(plan.delete, unorderedEquals(['1', '2']));
    });

    test('an event without a start is left out', () {
      final plan = planMirror(
        desired: [const CalendarEvent(uid: 'x@oldbrainfix', summary: 'x')],
        remembered: const {},
        marked: const {},
        zone: zone,
      );
      expect(plan.isEmpty, isTrue);
    });
  });

  group('remembered', () {
    test('kept, created and updated events under the ids they got', () {
      final a = event('a@oldbrainfix');
      final b = event('b@oldbrainfix');
      final c = event('c@oldbrainfix');
      final d = event('d@oldbrainfix');
      final plan = MirrorPlan(
        create: [b, d],
        update: [('3', c)],
        keep: {a.uid: known('1', a)},
      );
      expect(
        plan.remembered(created: ['2', null], updated: ['4'], zone: zone),
        {a.uid: known('1', a), b.uid: known('2', b), c.uid: known('4', c)},
      );
    });
  });

  group('mirrorEvents', () {
    late Store store;
    late Routines routines;

    setUp(() {
      store = Store.inMemory(clock: () => DateTime(2026, 10, 24, 9));
      routines = Routines(store);
    });
    tearDown(() => store.close());

    Schedule daily(List<ClockTime> times) => Schedule(
      start: Day(2026, 10, 24),
      times: times,
      rule: Recurrence.parse('FREQ=DAILY'),
    );

    test('every time of every Routine and Reminder outside the trash', () {
      final pills = routines.create('pills', daily([ClockTime(8, 0)]));
      final walk = routines.create('walk', daily([ClockTime(7, 0)]));
      final dentist = routines.create(
        'dentist',
        Schedule(start: Day(2026, 10, 30), times: [ClockTime(14, 0)]),
      );
      routines.trash(walk.id);
      final events = {
        for (final e in mirrorEvents(routines, (r) => 'text of ${r.id}'))
          e.uid: e,
      };
      expect(
        events.keys,
        unorderedEquals([
          '${pills.id}-0800@oldbrainfix',
          '${dentist.id}-1400@oldbrainfix',
        ]),
      );
      final pillsEvent = events['${pills.id}-0800@oldbrainfix'];
      expect(pillsEvent?.summary, 'text of ${pills.id}');
      expect(pillsEvent?.rule, 'FREQ=DAILY');
      expect(events['${dentist.id}-1400@oldbrainfix']?.rule, isNull);
    });
  });

  group('settings', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    test('the mirror is off until a calendar is chosen', () {
      expect(store.mirrorCalendar, isNull);
      store.mirrorCalendar = (id: '3', name: 'Home');
      expect(store.mirrorCalendar, (id: '3', name: 'Home'));
      store.mirrorCalendar = null;
      expect(store.mirrorCalendar, isNull);
    });

    test('the mirrored events are remembered', () {
      expect(store.mirroredEvents, isEmpty);
      store.mirroredEvents = {'a@oldbrainfix': const MirroredEvent('7', 'p')};
      expect(store.mirroredEvents, {
        'a@oldbrainfix': const MirroredEvent('7', 'p'),
      });
    });

    test('broken settings read as none', () {
      store.setLocalSetting('calendarMirror', '{nope');
      store.setLocalSetting('calendarMirror:events', '[1]');
      expect(store.mirrorCalendar, isNull);
      expect(store.mirroredEvents, isEmpty);
    });
  });
}
