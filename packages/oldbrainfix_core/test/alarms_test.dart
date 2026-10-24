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

void main() {
  group('instantOf', () {
    test('an ordinary time is the instant the clock shows it', () {
      expect(
        instantOf(Day(2026, 10, 24), at(8), europe),
        DateTime.utc(2026, 10, 24, 6),
      );
      expect(
        instantOf(Day(2026, 10, 26), at(8), europe),
        DateTime.utc(2026, 10, 26, 7),
        reason: '08:00 stays 08:00 on the wall after the switch',
      );
    });

    test('a time the clock skips fires when the clock jumps past it', () {
      // On 29 March the clock jumps from 02:00 to 03:00.
      expect(
        instantOf(Day(2026, 3, 29), at(2, 30), europe),
        DateTime.utc(2026, 3, 29, 1),
      );
      expect(
        instantOf(Day(2026, 3, 29), at(2), europe),
        DateTime.utc(2026, 3, 29, 1),
      );
      expect(
        instantOf(Day(2026, 3, 29), at(3), europe),
        DateTime.utc(2026, 3, 29, 1),
      );
      expect(
        instantOf(Day(2026, 3, 29), at(1, 59), europe),
        DateTime.utc(2026, 3, 29, 0, 59),
      );
    });

    test('a time the clock shows twice fires once, at the first', () {
      // On 25 October the clock goes from 03:00 back to 02:00.
      expect(
        instantOf(Day(2026, 10, 25), at(2, 30), europe),
        DateTime.utc(2026, 10, 25, 0, 30),
      );
      expect(
        instantOf(Day(2026, 10, 25), at(3), europe),
        DateTime.utc(2026, 10, 25, 2),
      );
    });
  });

  group('the alarm plan', () {
    late DateTime now;
    late Store store;
    late Notes notes;
    late Routines routines;
    late Alarms alarms;

    setUp(() {
      // 09:00 on Saturday, 24 October 2026, in Europe.
      now = DateTime.utc(2026, 10, 24, 7);
      store = Store.inMemory(clock: () => now);
      notes = Notes(store);
      routines = Routines(store);
      alarms = Alarms(store);
    });
    tearDown(() => store.close());

    Schedule daily(List<ClockTime> times, {Day? start}) => Schedule(
      start: start ?? Day(2026, 10, 1),
      times: times,
      rule: Recurrence.parse('FREQ=DAILY'),
    );

    List<String> plan() => [
      for (final alarm in alarms.plan(europe))
        '${alarm.kind.name} ${alarm.occurrence.key} '
            '${alarm.at.toIso8601String()} ${alarm.text}',
    ];

    test('holds each Occurrence ahead, soonest first, with its Note', () {
      routines.spawn(notes.capture('pills').id, daily([at(8), at(20)]));
      routines.spawn(
        notes.capture('dentist').id,
        Schedule(start: Day(2026, 10, 25), times: [at(10)]),
      );
      expect(plan().take(4), [
        // 08:00 today is past; 20:00 today is the first.
        'occurrence 2026-10-24T20:00 2026-10-24T18:00:00.000Z pills',
        // After the switch to winter time 08:00 is 07:00 UTC.
        'occurrence 2026-10-25T08:00 2026-10-25T07:00:00.000Z pills',
        'occurrence 2026-10-25T10:00 2026-10-25T09:00:00.000Z dentist',
        'occurrence 2026-10-25T20:00 2026-10-25T19:00:00.000Z pills',
      ]);
    });

    test('looks ahead the horizon, so alarms go on without the app', () {
      routines.spawn(notes.capture('pills').id, daily([at(20)]));
      final planned = alarms.plan(europe);
      expect(planned, hasLength(alarmHorizonDays + 1));
      expect(
        planned.last.occurrence.day,
        Day(2026, 10, 24).addDays(alarmHorizonDays),
      );
    });

    test('caps a Routine due very often', () {
      routines.spawn(
        notes.capture('stretch').id,
        daily([for (var h = 0; h < 24; h++) at(h)]),
      );
      expect(alarms.plan(europe), hasLength(alarmsPerRoutine));
    });

    test('caps the plan as a whole, keeping the soonest', () {
      for (var i = 0; i < 16; i++) {
        routines.spawn(
          notes.capture('stretch $i').id,
          daily([for (var h = 0; h < 24; h++) at(h)]),
        );
      }
      final planned = alarms.plan(europe);
      expect(planned, hasLength(alarmLimit));
      expect(planned.first.at, DateTime.utc(2026, 10, 24, 8));
      for (var i = 1; i < planned.length; i++) {
        expect(planned[i].at.isBefore(planned[i - 1].at), isFalse);
      }
    });

    test('leaves out what is Done, tomorrow included', () {
      final id = routines.spawn(notes.capture('pills').id, daily([at(20)])).id;
      routines.markDone(id, Occurrence(Day(2026, 10, 25), at(20)));
      expect(plan().take(2), [
        'occurrence 2026-10-24T20:00 2026-10-24T18:00:00.000Z pills',
        'occurrence 2026-10-26T20:00 2026-10-26T19:00:00.000Z pills',
      ]);
    });

    test('a past Reminder has no alarm; a deleted Routine neither', () {
      routines.spawn(
        notes.capture('call mum').id,
        Schedule(start: Day(2026, 10, 24), times: [at(8)]),
      );
      final gone = routines
          .spawn(
            notes.capture('dentist').id,
            Schedule(start: Day(2026, 10, 25), times: [at(10)]),
          )
          .id;
      routines.delete(gone);
      expect(plan(), isEmpty);
    });

    test('an all-day Occurrence rings in the morning', () {
      routines.spawn(
        notes.capture('bins out').id,
        Schedule(start: Day(2026, 10, 25)),
      );
      final [alarm] = alarms.plan(europe);
      expect(alarm.occurrence, Occurrence(Day(2026, 10, 25)));
      expect(alarm.wall?.time, allDayAlarmTime);
      expect(alarm.at, DateTime.utc(2026, 10, 25, 8));
    });

    test('an Occurrence in the skipped hour rings once the clock jumps', () {
      now = DateTime.utc(2026, 3, 28, 12);
      routines.spawn(
        notes.capture('night pills').id,
        Schedule(start: Day(2026, 3, 29), times: [at(2, 30)]),
      );
      final [alarm] = alarms.plan(europe);
      expect(alarm.at, DateTime.utc(2026, 3, 29, 1));
      expect(alarm.wall, (day: Day(2026, 3, 29), time: at(2, 30)));
    });

    test('an Occurrence in the doubled hour rings once, at the first', () {
      routines.spawn(
        notes.capture('night pills').id,
        Schedule(start: Day(2026, 10, 25), times: [at(2, 30)]),
      );
      final planned = alarms.plan(europe);
      expect(planned, hasLength(1));
      expect(planned.single.at, DateTime.utc(2026, 10, 25, 0, 30));
    });

    test('a snooze rings again at its time until it is over or Done', () {
      final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
      final morning = Occurrence(Day(2026, 10, 24), at(8));
      alarms.applySnooze(id, morning.key, now.add(const Duration(minutes: 30)));
      expect(
        plan().first,
        'snooze 2026-10-24T08:00 2026-10-24T07:30:00.000Z pills',
      );
      expect(alarms.plan(europe).first.id, 'snooze:$id@2026-10-24T08:00');

      now = now.add(const Duration(minutes: 31));
      expect(plan().first, startsWith('occurrence 2026-10-25T08:00'));

      now = DateTime.utc(2026, 10, 24, 7);
      routines.markDone(id, morning);
      expect(plan().first, startsWith('occurrence 2026-10-25T08:00'));
    });

    test('a snooze of a deleted Routine is dropped; snoozes stay local', () {
      final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
      final entries = store.allEntries().length;
      alarms.applySnooze(
        id,
        '2026-10-24T08:00',
        now.add(const Duration(minutes: 10)),
      );
      expect(store.allEntries(), hasLength(entries), reason: 'never logged');
      routines.delete(id);
      expect(plan(), isEmpty);
    });

    test('snoozes that are over are dropped when the next one comes', () {
      final id = routines
          .spawn(notes.capture('pills').id, daily([at(8), at(9)]))
          .id;
      alarms.applySnooze(
        id,
        '2026-10-24T08:00',
        now.add(const Duration(minutes: 10)),
      );
      now = now.add(const Duration(hours: 1));
      alarms.applySnooze(
        id,
        '2026-10-24T09:00',
        now.add(const Duration(minutes: 10)),
      );
      expect(alarms.snoozes.keys, ['$id@2026-10-24T09:00']);
    });

    test('names what was Done from yesterday to tomorrow', () {
      final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
      for (final day in [22, 23, 24, 25, 26]) {
        routines.markDone(id, Occurrence(Day(2026, 10, day), at(8)));
      }
      expect(
        [for (final (r, o) in alarms.recentlyDone(europe)) '$r ${o.key}'],
        [
          '$id 2026-10-23T08:00',
          '$id 2026-10-24T08:00',
          '$id 2026-10-25T08:00',
        ],
      );
    });

    test('a Done from a notification is applied as of when it was tapped', () {
      final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
      final tapped = DateTime.utc(2026, 10, 24, 6, 5);
      expect(alarms.applyDone(id, '2026-10-24T08:00', tapped), isTrue);
      final morning = Occurrence(Day(2026, 10, 24), at(8));
      expect(routines.byId(id)?.doneAt(morning), tapped);
      expect(
        alarms.applyDone(id, '2026-10-24T08:00', now),
        isFalse,
        reason: 'already Done: the first tap counts',
      );
      expect(routines.byId(id)?.doneAt(morning), tapped);
    });

    test('a Done for something gone or not due is dropped', () {
      final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
      expect(alarms.applyDone(id, '2026-10-24T09:00', now), isFalse);
      expect(alarms.applyDone(id, 'nonsense', now), isFalse);
      expect(
        alarms.applyDone('routine:gone', '2026-10-24T08:00', now),
        isFalse,
      );
      routines.delete(id);
      expect(alarms.applyDone(id, '2026-10-24T08:00', now), isFalse);
    });
  });
}
