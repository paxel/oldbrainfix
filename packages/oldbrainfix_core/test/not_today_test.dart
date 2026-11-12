import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

/// Skip and Not today ▸ Tomorrow (#269): an Occurrence answered without
/// being Done, and the schedule never changing (ADR 0006).
void main() {
  late DateTime now;
  late Store store;
  late Routines routines;
  late Alarms alarms;
  DateTime wall(DateTime utc) => utc;

  setUp(() {
    // 06:00 on Monday, 2 March 2026; the wall clock is UTC.
    now = DateTime.utc(2026, 3, 2, 6);
    store = Store.inMemory(clock: () => now);
    routines = Routines(store);
    alarms = Alarms(store);
  });
  tearDown(() => store.close());

  final monday = Day(2026, 3, 2);
  Occurrence on(int days, ClockTime time) =>
      Occurrence(monday.addDays(days), time);

  Routine daily(String text, ClockTime time) => routines.create(
    text,
    Schedule(
      start: monday,
      times: [time],
      rule: Recurrence.parse('FREQ=DAILY'),
    ),
  );

  Routine weekly(String text) => routines.create(
    text,
    Schedule(
      start: monday,
      times: [at(8)],
      rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
    ),
  );

  Routine read(Routine routine) => routines.byId(routine.id)!;

  String written(Schedule s) =>
      '${s.start} ${s.times.join(',')} ${s.rule?.format()}';

  List<String> planned(Routine routine) => [
    for (final a in alarms.plan(wall))
      if (a.routineId == routine.id) a.occurrence.key,
  ];

  group('Skip', () {
    test('a skipped Occurrence is neither Done nor Missed, rings no more '
        'and leaves the Adherence, counted beside it', () {
      final stretch = daily('Stretch', at(7));
      routines.markDone(stretch.id, on(0, at(7)));
      routines.skip(stretch.id, on(1, at(7)));
      expect(planned(stretch), isNot(contains(on(1, at(7)).key)));
      expect(planned(stretch), contains(on(2, at(7)).key));

      // Tuesday 07:30: the skipped one is due, and asks for nothing.
      now = DateTime.utc(2026, 3, 3, 7, 30);
      final row = routines.agenda(wall).todayRows.single;
      expect(row.skipped, isNotNull);
      expect(row.done, isNull);
      expect(alarms.openOf(stretch.noteId, wall), isEmpty);
      expect(alarms.recentlyDone(wall), contains((stretch.id, on(1, at(7)))));

      // Thursday 09:00: Wednesday's is Missed, Tuesday's never.
      now = DateTime.utc(2026, 3, 5, 9);
      final missed = alarms.missed(wall);
      expect(
        [for (final o in missed.routines.single.missed) o.occurrence],
        [on(2, at(7))],
      );
      final history = routines.history(read(stretch), wall);
      expect(
        [for (final e in history.entries) e.status],
        [
          OccurrenceStatus.done,
          OccurrenceStatus.skipped,
          OccurrenceStatus.missed,
          OccurrenceStatus.open,
        ],
      );
      expect(history.entries[1].skippedAt, DateTime.utc(2026, 3, 2, 6));
      expect(history.thisWeek.done, 1);
      expect(history.thisWeek.missed, 1);
      expect(history.thisWeek.skipped, 1);
      expect(history.thisWeek.share, 0.5);
      final tuesday = history.lastSevenDays[4];
      expect(tuesday.day, monday.addDays(1));
      expect((tuesday.skipped, tuesday.open, tuesday.empty), (1, 0, false));
    });

    test('a skip neither breaks a Streak nor counts in it', () {
      final stretch = daily('Stretch', at(7));
      routines.markDone(stretch.id, on(0, at(7)));
      routines.skip(stretch.id, on(1, at(7)));
      routines.markDone(stretch.id, on(2, at(7)));
      now = DateTime.utc(2026, 3, 5, 6);
      final history = routines.history(read(stretch), wall);
      expect(history.currentStreak, 2);
      expect(history.longestStreak, 2);
    });

    test('a skip writes no schedule: what was asked for stays so', () {
      final stretch = daily('Stretch', at(7));
      now = DateTime.utc(2026, 3, 3, 9);
      routines.skip(stretch.id, on(1, at(7)));
      final skipped = read(stretch);
      expect(skipped.scheduledAt, stretch.scheduledAt);
      expect(written(skipped.schedule), written(stretch.schedule));
      expect(
        [
          for (final o in alarms.missed(wall).routines.single.missed)
            o.occurrence,
        ],
        [on(0, at(7))],
      );
    });

    test('Done after a skip counts as Done', () {
      final stretch = daily('Stretch', at(7));
      routines.skip(stretch.id, on(0, at(7)));
      routines.markDone(stretch.id, on(0, at(7)));
      now = DateTime.utc(2026, 3, 2, 9);
      final history = routines.history(read(stretch), wall);
      expect(history.entries.single.status, OccurrenceStatus.done);
      expect(history.thisWeek.skipped, 0);
    });

    test('only an Occurrence that is due and not Done is skipped', () {
      final stretch = daily('Stretch', at(7));
      expect(
        () => routines.skip(stretch.id, on(0, at(8))),
        throwsArgumentError,
      );
      expect(
        () => routines.tomorrow(stretch.id, Occurrence(monday)),
        throwsArgumentError,
      );
      routines.markDone(stretch.id, on(0, at(7)));
      expect(() => routines.skip(stretch.id, on(0, at(7))), throwsStateError);
      expect(
        () => routines.tomorrow(stretch.id, on(0, at(7))),
        throwsStateError,
      );
      routines.trash(stretch.id);
      expect(
        () => routines.skip(stretch.id, on(1, at(7))),
        throwsArgumentError,
      );
    });

    test('unskipped, it is due again', () {
      final stretch = daily('Stretch', at(7));
      routines.skip(stretch.id, on(0, at(7)));
      routines.unskip(stretch.id, on(0, at(7)));
      expect(read(stretch).skippedAt(on(0, at(7))), isNull);
      expect(planned(stretch), contains(on(0, at(7)).key));
      final written = store.allEntries().length;
      routines.unskip(stretch.id, on(1, at(7)));
      expect(store.allEntries(), hasLength(written), reason: 'nothing held');
    });
  });

  group('Tomorrow', () {
    test('today\'s is skipped and a one-time Occurrence comes tomorrow at '
        'the same time; the schedule stays', () {
      final bins = weekly('Take out the bins');
      routines.tomorrow(bins.id, on(0, at(8)));
      final moved = read(bins);
      expect(written(moved.schedule), written(bins.schedule));
      expect(moved.scheduledAt, bins.scheduledAt);
      expect(moved.skippedAt(on(0, at(8))), DateTime.utc(2026, 3, 2, 6));
      expect(moved.tomorrow, {on(0, at(8)).key});
      expect(moved.moved, [on(1, at(8))]);
      expect(moved.has(on(1, at(8))), isTrue);
      expect(moved.between(monday, monday.addDays(7)), [
        on(0, at(8)),
        on(1, at(8)),
        on(7, at(8)),
      ]);

      final agenda = routines.agenda(wall);
      expect(agenda.todayRows.single.skipped, isNotNull);
      final tomorrow = agenda.tomorrowRows.single;
      expect(tomorrow.occurrence, on(1, at(8)));
      expect((tomorrow.moved, tomorrow.skipped), (true, null));

      // It rings, and Re-nags until the next Occurrence starts.
      final alarm = alarms.plan(wall).firstWhere((a) => a.routineId == bins.id);
      expect(alarm.occurrence, on(1, at(8)));
      expect(alarm.at, DateTime.utc(2026, 3, 3, 8));
      expect(alarm.endsAt, DateTime.utc(2026, 3, 9, 8));
      expect(planned(bins), isNot(contains(on(0, at(8)).key)));

      // Not Done, it is Missed; today's never.
      now = DateTime.utc(2026, 3, 9, 9);
      expect(
        [
          for (final o in alarms.missed(wall).routines.single.missed)
            o.occurrence,
        ],
        [on(1, at(8))],
      );
      final history = routines.history(read(bins), wall);
      expect(
        [for (final e in history.entries) (e.status, e.moved)],
        [
          (OccurrenceStatus.skipped, false),
          (OccurrenceStatus.missed, true),
          (OccurrenceStatus.open, false),
        ],
      );
    });

    test('the moved Occurrence is checked off like any other', () {
      final bins = weekly('Take out the bins');
      routines.tomorrow(bins.id, on(0, at(8)));
      now = DateTime.utc(2026, 3, 3, 8, 30);
      routines.markDone(bins.id, on(1, at(8)));
      expect(alarms.recentlyDone(wall), contains((bins.id, on(1, at(8)))));
      final history = routines.history(read(bins), wall);
      expect(history.thisWeek.done, 1);
      expect(history.thisWeek.skipped, 1);
      expect(history.thisWeek.share, 1.0);
    });

    test('a day that has the Occurrence already gets none added', () {
      final stretch = daily('Stretch', at(7));
      routines.tomorrow(stretch.id, on(0, at(7)));
      final moved = read(stretch);
      expect(moved.skippedAt(on(0, at(7))), isNotNull);
      expect(moved.moved, isEmpty);
      expect(routines.agenda(wall).tomorrowRows.single.moved, isFalse);
    });

    test('a moved Occurrence moves on again, and a schedule that no longer '
        'has the one skipped drops what was added for it', () {
      final bins = weekly('Take out the bins');
      routines.tomorrow(bins.id, on(0, at(8)));
      routines.tomorrow(bins.id, on(1, at(8)));
      expect(read(bins).moved, [on(1, at(8)), on(2, at(8))]);
      routines.reschedule(
        bins.id,
        Schedule(
          start: monday,
          times: [at(8)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=TH'),
        ),
      );
      expect(read(bins).moved, isEmpty);
      expect(read(bins).has(on(1, at(8))), isFalse);
    });

    test('unskipped, today\'s is due again and the added one is gone; '
        'what was Done on it stays Done', () {
      final bins = weekly('Take out the bins');
      routines.tomorrow(bins.id, on(0, at(8)));
      routines.markDone(bins.id, on(1, at(8)));
      routines.unskip(bins.id, on(0, at(8)));
      final back = read(bins);
      expect(back.skippedAt(on(0, at(8))), isNull);
      expect(back.moved, isEmpty);
      expect(routines.agenda(wall).tomorrowRows, isEmpty);
      now = DateTime.utc(2026, 3, 3, 9);
      final history = routines.history(back, wall);
      expect(
        [for (final e in history.entries) (e.occurrence, e.status)],
        [
          (on(0, at(8)), OccurrenceStatus.open),
          (on(1, at(8)), OccurrenceStatus.done),
        ],
      );
    });

    test('a Checklist\'s moved Occurrence is the one its Lines are '
        'checked for', () {
      final note = Notes(store).capture('Before the bins\nRinse\nSort').id;
      Checklists(store).make(note);
      final bins = routines.spawn(
        note,
        Schedule(
          start: monday,
          times: [at(8)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
        ),
      );
      routines.tomorrow(bins.id, on(0, at(8)));
      now = DateTime.utc(2026, 3, 3, 9);
      final occurrence = Checklists(store).occurrenceNow(note, wall);
      expect(occurrence?.occurrence, on(1, at(8)));
      for (final line in Checklists(store).byNote(note)!.lines) {
        Checklists(store).check(line.id, on: occurrence);
      }
      expect(read(bins).doneAt(on(1, at(8))), isNotNull);
    });
  });

  group('a Session', () {
    test('a moved Occurrence is one its picks are recorded for', () {
      final pushUps = Notes(store).capture('Push-ups').id;
      Tallies(store).addField(pushUps, name: 'Push-ups', target: 20);
      final sessions = Sessions(store);
      final sport = sessions.create(
        'Sport',
        Schedule(
          start: monday,
          times: [at(8)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
        ),
        pool: [pushUps],
        count: 1,
        rule: SessionRule.rolling,
      );
      expect(() => sessions.on(sport.id, on(1, at(8))), throwsArgumentError);
      routines.tomorrow(sport.id, on(0, at(8)));
      expect(sessions.on(sport.id, on(1, at(8))).picks, hasLength(1));
    });
  });

  group('a notification\'s Not today', () {
    test('Skip and Tomorrow keep when they were tapped; one already '
        'answered is left as it is', () {
      final stretch = daily('Stretch', at(7));
      final bins = weekly('Take out the bins');
      final tapped = DateTime.utc(2026, 3, 2, 7, 5);
      expect(alarms.applySkip(stretch.id, on(0, at(7)).key, tapped), isTrue);
      expect(read(stretch).skippedAt(on(0, at(7))), tapped);
      expect(alarms.applySkip(stretch.id, on(0, at(7)).key, now), isFalse);
      expect(alarms.applyTomorrow(stretch.id, on(0, at(7)).key, now), isFalse);
      expect(alarms.applyTomorrow(bins.id, on(0, at(8)).key, tapped), isTrue);
      expect(read(bins).moved, [on(1, at(8))]);
      routines.markDone(stretch.id, on(1, at(7)));
      expect(alarms.applySkip(stretch.id, on(1, at(7)).key, now), isFalse);
      expect(alarms.applySkip(stretch.id, 'Rude', now), isFalse);
      expect(alarms.applySkip(stretch.id, on(0, at(9)).key, now), isFalse);
      expect(alarms.applySkip('routine:gone', on(0, at(7)).key, now), isFalse);
    });
  });

  group('the field as others may write it', () {
    test('keys and values this version cannot read skip nothing and throw '
        'nothing; a later word after the moment is a plain skip', () {
      final bins = weekly('Take out the bins');
      final key = on(0, at(8)).key;
      void raw(String field, String value) =>
          store.append(bins.id, field, value);
      raw('${RoutineKeys.skipPrefix}Rude', '2026-03-02T06:00:00.000Z tomorrow');
      raw(
        '${RoutineKeys.skipPrefix}2026-3-9T08:00',
        '2026-03-02T06:00:00.000Z tomorrow',
      );
      for (final value in [
        'Rude',
        '2026-02-30T06:00:00.000Z tomorrow',
        '2026-03-02T24:00:00Z',
        '2026-03-02T06:00:60Z',
        '2026-03-02T06:00:00+01:00',
        '2026-03-02 06:00:00Z',
        ' tomorrow',
      ]) {
        raw(RoutineKeys.skip(on(0, at(8))), value);
        final routine = read(bins);
        expect(routine.skipped, isEmpty, reason: value);
        expect(routine.moved, isEmpty, reason: value);
      }
      raw(RoutineKeys.skip(on(0, at(8))), '2026-03-02T06:00:00.000123Z later');
      final later = read(bins);
      expect(later.skipped, {key: DateTime.utc(2026, 3, 2, 6, 0, 0, 0, 123)});
      expect(later.moved, isEmpty);
      // Tomorrow on what is no Occurrence adds none.
      raw(
        '${RoutineKeys.skipPrefix}2026-03-04T03:00',
        '2026-03-02T06:00:00.000Z tomorrow',
      );
      expect(read(bins).moved, isEmpty);
      expect(routines.agenda(wall, days: 7).laterRows[1], isEmpty);
    });
  });

  group("between one's own Devices", () {
    test('a Skip and a Tomorrow of the same Occurrence go by latest wins', () {
      final clock = ticking();
      final ann = device(clock, 'Ann');
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      final bins = Routines(ann).create(
        'Take out the bins',
        Schedule(
          start: Day(2026, 3, 2),
          times: [at(8)],
          rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
        ),
      );
      final monday = Occurrence(Day(2026, 3, 2), at(8));
      exchange(desk, ann);
      Routines(ann).skip(bins.id, monday);
      Routines(desk).tomorrow(bins.id, monday);
      exchange(desk, ann);
      for (final store in [ann, desk]) {
        final routine = Routines(store).byId(bins.id)!;
        expect(routine.skippedAt(monday), isNotNull);
        expect(routine.moved, [Occurrence(Day(2026, 3, 3), at(8))]);
      }
      Routines(ann).unskip(bins.id, monday);
      exchange(desk, ann);
      for (final store in [ann, desk]) {
        final routine = Routines(store).byId(bins.id)!;
        expect(routine.skippedAt(monday), isNull);
        expect(routine.moved, isEmpty);
      }
    });
  });
}
