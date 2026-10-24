import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Central European time in 2026: summer time (+2) from 29 March 01:00
/// UTC to 25 October 01:00 UTC, winter time (+1) around it.
DateTime europe(DateTime utc) {
  final summer =
      !utc.isBefore(DateTime.utc(2026, 3, 29, 1)) &&
      utc.isBefore(DateTime.utc(2026, 10, 25, 1));
  final wall = utc.add(Duration(hours: summer ? 2 : 1));
  return DateTime.utc(
    wall.year,
    wall.month,
    wall.day,
    wall.hour,
    wall.minute,
    wall.second,
    wall.millisecond,
  );
}

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

/// The instant the clock in Europe shows [hour]:[minute] on [month]/[day].
DateTime wall(int month, int day, int hour, [int minute = 0]) =>
    instantOf(Day(2026, month, day), at(hour, minute), europe);

void main() {
  late DateTime now;
  late Store store;
  late Routines routines;

  setUp(() {
    // 09:00 on Saturday, 24 October 2026, in Europe.
    now = wall(10, 24, 9);
    store = Store.inMemory(clock: () => now);
    routines = Routines(store);
  });
  tearDown(() => store.close());

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 9, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  /// Runs [body] as of [then].
  T asOf<T>(DateTime then, T Function() body) {
    final saved = now;
    now = then;
    try {
      return body();
    } finally {
      now = saved;
    }
  }

  /// Makes a Routine as of [made], before its Occurrences were due.
  Routine madeAt(DateTime made, Schedule schedule) =>
      asOf(made, () => routines.create('pills', schedule));

  void done(Routine routine, int month, int day, ClockTime time, DateTime at) =>
      routines.markDone(
        routine.id,
        Occurrence(Day(2026, month, day), time),
        at: at,
      );

  RoutineHistory historyOf(
    Routine routine, {
    int weekStart = DateTime.monday,
  }) => routines.history(
    routines.byId(routine.id) ?? routine,
    europe,
    weekStart: weekStart,
  );

  List<String> log(RoutineHistory history) => [
    for (final e in history.entries) '${e.occurrence.key} ${e.status.name}',
  ];

  group('the log', () {
    test('several times a day: each Occurrence on its own, with when it was '
        'Done; one still due neither counts nor breaks the Streak', () {
      final pills = madeAt(wall(10, 20, 6), daily([at(8), at(20)]));
      done(pills, 10, 20, at(8), wall(10, 20, 8, 5));
      for (final day in [21, 22, 23]) {
        done(pills, 10, day, at(8), wall(10, day, 8, 5));
        done(pills, 10, day, at(20), wall(10, day, 20, 15));
      }
      final history = historyOf(pills);

      expect(log(history), [
        '2026-10-20T08:00 done',
        '2026-10-20T20:00 missed',
        '2026-10-21T08:00 done',
        '2026-10-21T20:00 done',
        '2026-10-22T08:00 done',
        '2026-10-22T20:00 done',
        '2026-10-23T08:00 done',
        '2026-10-23T20:00 done',
        '2026-10-24T08:00 open',
        '2026-10-24T20:00 coming',
      ]);
      final first = history.entries.first;
      expect(first.start, wall(10, 20, 8));
      expect(first.doneAt, wall(10, 20, 8, 5));
      expect(first.timing, DoneTiming.onTime);
      // Missed when the next one started.
      expect(history.entries[1].missedAt, wall(10, 21, 8));
      expect(history.entries[1].doneAt, isNull);
      expect(history.currentStreak, 6);
      expect(history.longestStreak, 6);
    });

    test('Done early, on time or late, and late Done keeps the Streak', () {
      final pills = madeAt(wall(10, 22, 6), daily([at(8), at(20)]));
      done(pills, 10, 22, at(8), wall(10, 22, 7, 50));
      done(pills, 10, 22, at(20), wall(10, 22, 21));
      // From the Missed Summary, the next morning.
      done(pills, 10, 23, at(8), wall(10, 23, 8, 10));
      done(pills, 10, 23, at(20), wall(10, 24, 8, 30));
      done(pills, 10, 24, at(8), wall(10, 24, 8, 1));
      done(pills, 10, 24, at(20), wall(10, 24, 9));
      final history = historyOf(pills);

      expect(
        [for (final e in history.entries) e.timing],
        [
          DoneTiming.early,
          DoneTiming.onTime,
          DoneTiming.onTime,
          DoneTiming.late,
          DoneTiming.onTime,
          DoneTiming.early,
        ],
      );
      expect(history.currentStreak, 6);
    });

    test('the longest Streak outlasts a Missed; the current one restarts', () {
      final pills = madeAt(wall(10, 18, 6), daily([at(8)]));
      for (final day in [18, 19, 20, 22, 23]) {
        done(pills, 10, day, at(8), wall(10, day, 8));
      }
      final history = historyOf(pills);
      expect(history.longestStreak, 3);
      expect(history.currentStreak, 2);

      routines.unmarkDone(pills.id, Occurrence(Day(2026, 10, 23), at(8)));
      expect(historyOf(pills).currentStreak, 0);
    });

    test('nothing from before the Routine was made counts', () {
      final pills = madeAt(wall(10, 24, 8, 30), daily([at(8)]));
      final history = historyOf(pills);
      expect(history.entries, isEmpty);
      expect(history.currentStreak, 0);
      expect(history.longestStreak, 0);
      expect(history.thisWeek.share, isNull);
    });

    test('the last seven days, day by day', () {
      final pills = madeAt(wall(10, 20, 6), daily([at(8), at(20)]));
      done(pills, 10, 20, at(8), wall(10, 20, 8));
      done(pills, 10, 21, at(8), wall(10, 21, 8));
      done(pills, 10, 21, at(20), wall(10, 21, 20));
      final days = historyOf(pills).lastSevenDays;
      expect(
        [for (final d in days) d.day],
        [for (var day = 18; day <= 24; day++) Day(2026, 10, day)],
      );
      expect(
        [for (final d in days) (d.done, d.missed, d.open)],
        [
          (0, 0, 0),
          (0, 0, 0),
          (1, 1, 0),
          (2, 0, 0),
          (0, 2, 0),
          (0, 2, 0),
          (0, 0, 2),
        ],
      );
      expect(days.first.empty, isTrue);
    });

    test('snoozes this Device knows of show, those of others do not', () {
      final pills = madeAt(wall(10, 22, 6), daily([at(8)]));
      final other = madeAt(wall(10, 22, 6), daily([at(8)]));
      final alarms = Alarms(store);
      alarms.applySnooze(pills.id, '2026-10-23T08:00', wall(10, 23, 8, 30));
      alarms.applySnooze(other.id, '2026-10-22T08:00', wall(10, 22, 9));
      final history = historyOf(pills);
      expect(
        [for (final e in history.entries) e.snoozedUntil],
        [null, wall(10, 23, 8, 30), null],
      );
      expect(
        store.allEntries().where((e) => e.field.contains('snooze')),
        isEmpty,
      );
    });
  });

  group('Missed', () {
    test('agrees with the Missed Summary', () {
      final pills = madeAt(wall(10, 10, 6), daily([at(8), at(13), at(20)]));
      for (var day = 10; day <= 24; day++) {
        if (day % 3 == 0) continue;
        done(pills, 10, day, at(13), wall(10, day, 13));
      }
      done(pills, 10, 15, at(8), wall(10, 15, 8));
      final summary = missedSummary(
        routines: [(routines.byId(pills.id) ?? pills, null)],
        seen: null,
        now: now,
        wallClock: europe,
      );
      final history = historyOf(pills);
      expect(
        {
          for (final e in history.entries)
            if (e.status == OccurrenceStatus.missed) e.occurrence,
        },
        {for (final m in summary.routines.single.missed) m.occurrence},
      );
      expect(
        {
          for (final e in history.entries)
            if (e.status == OccurrenceStatus.missed) e.missedAt,
        },
        {for (final m in summary.routines.single.missed) m.missedAt},
      );
    });

    test('across the end of summer time, by the wall clock', () {
      // 09:00 on Tuesday, 27 October, in winter time.
      now = wall(10, 27, 9);
      // 02:30 comes twice on the 25th; it is due at the first.
      final pills = madeAt(wall(10, 22, 6), daily([at(2, 30)]));
      done(pills, 10, 23, at(2, 30), wall(10, 23, 2, 40));
      done(pills, 10, 25, at(2, 30), DateTime.utc(2026, 10, 25, 1, 15));
      done(pills, 10, 27, at(2, 30), wall(10, 27, 3));
      final history = historyOf(pills);

      expect(log(history), [
        '2026-10-23T02:30 done',
        '2026-10-24T02:30 missed',
        '2026-10-25T02:30 done',
        '2026-10-26T02:30 missed',
        '2026-10-27T02:30 done',
      ]);
      final sunday = history.entries[2];
      expect(sunday.start, DateTime.utc(2026, 10, 25, 0, 30));
      // 02:15 the second time round, still before the next one.
      expect(sunday.timing, DoneTiming.onTime);
      expect(history.entries[3].missedAt, DateTime.utc(2026, 10, 27, 1, 30));
      expect(
        [for (final d in history.lastSevenDays) d.done + d.missed],
        [0, 0, 1, 1, 1, 1, 1],
      );
      expect(history.last30Days.done, 3);
      expect(history.last30Days.missed, 2);
    });

    test('a schedule change keeps what was Missed before it; the Occurrence '
        'running across the change counts neither way', () {
      final pills = madeAt(wall(10, 20, 6), daily([at(8)]));
      done(pills, 10, 22, at(8), wall(10, 22, 8));
      // At noon on the 23rd, 08:00 that day still not Done, to 10:00.
      asOf(
        wall(10, 23, 12),
        () => routines.reschedule(pills.id, daily([at(10)])),
      );
      final history = historyOf(pills);

      expect(log(history), [
        '2026-10-20T08:00 missed',
        '2026-10-21T08:00 missed',
        '2026-10-22T08:00 done',
        '2026-10-24T10:00 coming',
      ]);
      expect(history.last30Days.done, 1);
      expect(history.last30Days.missed, 2);
      // The Missed Summary only looks at the schedule as it is now.
      expect(Alarms(store).missed(europe).isEmpty, isTrue);
    });

    test('nothing from the trash period counts; the Streak goes on across '
        'it', () {
      final pills = madeAt(wall(10, 18, 20), daily([at(8)]));
      done(pills, 10, 20, at(8), wall(10, 20, 8));
      done(pills, 10, 21, at(8), wall(10, 21, 8));
      asOf(wall(10, 21, 12), () => routines.trash(pills.id));
      asOf(wall(10, 23, 9, 30), () => routines.restore(pills.id));
      done(pills, 10, 24, at(8), wall(10, 24, 8));
      final history = historyOf(pills);

      expect(log(history), [
        '2026-10-19T08:00 missed',
        '2026-10-20T08:00 done',
        '2026-10-21T08:00 done',
        '2026-10-24T08:00 done',
      ]);
      expect(history.currentStreak, 3);
      expect(history.longestStreak, 3);
      expect(history.last30Days.missed, 1);
    });

    test('a Routine trashed before its Occurrence was Missed', () {
      final pills = madeAt(wall(10, 22, 6), daily([at(8)]));
      asOf(wall(10, 22, 12), () => routines.trash(pills.id));
      expect(log(historyOf(pills)), isEmpty);
    });
  });

  group('Adherence', () {
    late Routine pills;
    setUp(() {
      pills = madeAt(wall(9, 27, 6), daily([at(8)]));
      const missed = [(9, 30), (10, 5), (10, 20)];
      for (var d = Day(2026, 9, 27); d < Day(2026, 10, 24); d = d.addDays(1)) {
        if (missed.contains((d.month, d.day))) continue;
        done(pills, d.month, d.day, at(8), wall(d.month, d.day, 8, 5));
      }
    });

    test('this week, this month and the last 30 days; one still due does '
        'not count', () {
      final history = historyOf(pills);
      // Monday the 19th to today; today's 08:00 is still due.
      expect((history.thisWeek.done, history.thisWeek.missed), (4, 1));
      expect(history.thisWeek.share, 0.8);
      expect((history.thisMonth.done, history.thisMonth.missed), (21, 2));
      expect((history.last30Days.done, history.last30Days.missed), (24, 3));
    });

    test('a week can start on Sunday', () {
      final history = historyOf(pills, weekStart: DateTime.sunday);
      expect((history.thisWeek.done, history.thisWeek.missed), (5, 1));
    });

    test('any period', () {
      final history = historyOf(pills);
      final september = history.adherence(Day(2026, 9, 1), Day(2026, 9, 30));
      expect((september.done, september.missed), (3, 1));
      expect(september.share, 0.75);
    });
  });

  group('schedule spans', () {
    test('a schedule written a moment apart is one change, as of its last '
        'write', () {
      // Every read of the clock moves it on a little.
      var tick = wall(10, 20, 8).subtract(const Duration(milliseconds: 5));
      store.close();
      store = Store.inMemory(
        clock: () => tick = tick.add(const Duration(milliseconds: 1)),
      );
      routines = Routines(store);
      final pills = routines.create('pills', daily([at(8)]));
      final spans = scheduleSpans(store.history(pills.id));
      expect(spans, hasLength(1));
      expect(spans.single.from, routines.byId(pills.id)?.scheduledAt);
      expect(spans.single.until, isNull);
      expect(spans.single.schedule.dayTimes, [at(8)]);
    });

    test('the trash and a new schedule end a span', () {
      final pills = madeAt(wall(10, 18, 6), daily([at(8)]));
      asOf(wall(10, 19, 12), () => routines.trash(pills.id));
      asOf(wall(10, 20, 12), () => routines.restore(pills.id));
      asOf(
        wall(10, 21, 12),
        () => routines.reschedule(pills.id, daily([at(9)])),
      );
      final spans = scheduleSpans(store.history(pills.id));
      expect(
        [for (final s in spans) (s.from, s.until)],
        [
          (wall(10, 18, 6), wall(10, 19, 12)),
          (wall(10, 20, 12), wall(10, 21, 12)),
          (wall(10, 21, 12), null),
        ],
      );
      expect(
        [for (final s in spans) s.schedule.dayTimes],
        [
          [at(8)],
          [at(8)],
          [at(9)],
        ],
      );
    });
  });
}
