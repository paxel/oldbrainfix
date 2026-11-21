import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'alarms_test.dart' show at, europe;
import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  group('A quiet day (obf-1044)', () {
    late DateTime now;
    late Store store;
    late Notes notes;
    late Routines routines;
    late Alarms alarms;

    setUp(() {
      // 09:00 on Tuesday, 6 January 2026, in Europe (winter time, +1).
      now = DateTime.utc(2026, 1, 6, 8);
      store = Store.inMemory(clock: () => now);
      notes = Notes(store);
      routines = Routines(store);
      alarms = Alarms(store);
    });
    tearDown(() => store.close());

    String daily(String text, List<ClockTime> times, {Day? from}) => routines
        .spawn(
          notes.capture(text).id,
          Schedule(
            start: from ?? Day(2026, 1, 1),
            times: times,
            rule: Recurrence.parse('FREQ=DAILY'),
          ),
        )
        .id;

    List<String> planned() => [
      for (final a in alarms.plan(europe))
        '${a.occurrence.key} ${a.at.toIso8601String()}',
    ];

    group('kept', () {
      test('is not there until Quiet today; again writes nothing', () {
        expect(store.quietUntil(europe), isNull);
        store.quietToday(europe);
        // Midnight in Europe.
        expect(store.quietUntil(europe), DateTime.utc(2026, 1, 6, 23));
        final written = store.allEntries().length;
        store.quietToday(europe);
        expect(store.allEntries().length, written);
        expect(
          store.current(
            PersonSettingKeys.entity,
            PersonSettingKeys.quietDay(Day(2026, 1, 6)),
          ),
          PersonSettingKeys.on,
        );
      });

      test('End keeps the moment it was tapped; it ends nothing twice', () {
        store.quietToday(europe);
        now = DateTime.utc(2026, 1, 6, 13);
        store.endQuietDay(europe);
        expect(store.quietUntil(europe), isNull);
        expect(store.quietDays.ends, {
          Day(2026, 1, 6): DateTime.utc(2026, 1, 6, 13),
        });
        final written = store.allEntries().length;
        store.endQuietDay(europe);
        expect(store.allEntries().length, written);
        // Quiet today again keeps the whole day quiet once more.
        store.quietToday(europe);
        expect(store.quietUntil(europe), DateTime.utc(2026, 1, 6, 23));
      });

      test('ends at midnight by itself, and the next day is not quiet', () {
        store.quietToday(europe);
        now = DateTime.utc(2026, 1, 6, 23);
        expect(store.quietUntil(europe), isNull);
        expect(store.quietDays.ends.keys, [Day(2026, 1, 6)]);
      });

      test('a day or a value no app writes is not quiet', () {
        void raw(String field, String value) =>
            store.append(PersonSettingKeys.entity, field, value);
        raw('${PersonSettingKeys.quietDayPrefix}2026-1-6', 'on');
        raw(PersonSettingKeys.quietDay(Day(2026, 1, 7)), 'yes');
        raw(
          PersonSettingKeys.quietDay(Day(2026, 1, 8)),
          '2026-01-08T12:00:00.000Z later',
        );
        raw(PersonSettingKeys.quietDay(Day(2026, 1, 9)), '2026-02-30T12:00Z');
        expect(store.quietDays.ends, isEmpty);
      });

      test('each Day keeps its own, so an earlier one stays quiet', () {
        store.quietToday(europe);
        now = DateTime.utc(2026, 1, 8, 8);
        store.quietToday(europe);
        expect(store.quietDays.ends.keys, [Day(2026, 1, 6), Day(2026, 1, 8)]);
      });
    });

    group('rings nothing', () {
      test("today's Occurrences have no alarm; tomorrow's ring", () {
        daily('pills', [at(8), at(20)]);
        store.quietToday(europe);
        expect(planned().first, startsWith('2026-01-07T08:00 '));
        expect(planned().where((p) => p.startsWith('2026-01-06')), isEmpty);
      });

      test('what began before it Re-nags again from midnight', () {
        // Set up before Monday's 09:00, so that one was asked for.
        now = DateTime.utc(2026, 1, 5, 7);
        final weekly = routines.spawn(
          notes.capture('stretch').id,
          Schedule(
            start: Day(2026, 1, 5),
            times: [at(9)],
            rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO'),
          ),
        );
        now = DateTime.utc(2026, 1, 6, 8);
        store.quietToday(europe);
        final alarm = alarms
            .plan(europe)
            .singleWhere(
              (a) => a.routineId == weekly.id && a.occurrence.day.day == 5,
            );
        expect(alarm.at, DateTime.utc(2026, 1, 6, 23));
        expect('${alarm.wall!.day}T${alarm.wall!.time}', '2026-01-07T00:00');
      });

      test('End brings back what is still to come, not what passed', () {
        daily('pills', [at(8), at(20)]);
        store.quietToday(europe);
        now = DateTime.utc(2026, 1, 6, 13);
        store.endQuietDay(europe);
        expect(planned().first, '2026-01-06T20:00 2026-01-06T19:00:00.000Z');
      });

      test('no Heads-up warns of it or fires on it', () {
        final dentist = routines.spawn(
          notes.capture('dentist').id,
          Schedule(start: Day(2026, 1, 7), times: [at(10)]),
        );
        final today = routines.spawn(
          notes.capture('call').id,
          Schedule(start: Day(2026, 1, 6), times: [at(18)]),
        );
        routines.setHeadsUps(dentist.id, [60, 1440]);
        routines.setHeadsUps(today.id, [60]);
        store.quietToday(europe);
        expect(
          [for (final h in alarms.headsUps(europe)) h.id],
          ['headsUp:60:${dentist.id}@2026-01-07T10:00'],
        );
      });

      test('the Nudge waits for the day to end', () {
        notes.capture('Unsorted');
        store.quietToday(europe);
        final nudge = alarms.nudge(europe)!;
        // Midnight, then the Quiet Hours until 08:00.
        expect(nudge.at, DateTime.utc(2026, 1, 7, 7));
        expect(nudge.firesAfter(now, europe), DateTime.utc(2026, 1, 7, 7));
      });

      test('the Changes Nudge waits for the day to end', () {
        final plan = changesNudgePlan(
          waiting: {'profile:bob': now},
          now: now,
          wallClock: europe,
          repeat: const Duration(hours: 1),
          quietUntil: DateTime.utc(2026, 1, 6, 23),
        )!;
        expect(plan.at, DateTime.utc(2026, 1, 6, 23));
      });

      test('its notifications showing may close', () {
        final id = daily('pills', [at(8)]);
        store.quietToday(europe);
        expect(alarms.recentlyDone(europe), [
          (id, Occurrence(Day(2026, 1, 6), at(8))),
        ]);
      });
    });

    group('counts what is left undone as skipped', () {
      test('never Missed, and skipped in the history after midnight', () {
        // Set up before today's 08:00, so that one was asked for.
        now = DateTime.utc(2026, 1, 6, 6);
        final id = daily('pills', [at(8), at(14)]);
        store.quietToday(europe);
        now = DateTime.utc(2026, 1, 6, 15);
        // 08:00 would be Missed at 14:00.
        expect(alarms.missed(europe).routines, isEmpty);
        expect(
          [
            for (final o in alarms.openOf(notes.all().single.id, europe))
              '${o.occurrence.key} ${o.missed}',
          ],
          ['2026-01-06T14:00 false'],
        );
        now = DateTime.utc(2026, 1, 7, 9);
        expect(alarms.missed(europe).routines, isEmpty);
        final history = routines.history(routines.byId(id)!, europe);
        final quiet = [
          for (final e in history.entries)
            if (e.occurrence.day == Day(2026, 1, 6)) e,
        ];
        expect(quiet.map((e) => e.status), [
          OccurrenceStatus.skipped,
          OccurrenceStatus.skipped,
        ]);
        expect(quiet.first.skippedAt, DateTime.utc(2026, 1, 6, 23));
        expect(history.thisWeek.skipped, 2);
      });

      test('what is Done on it stays Done', () {
        final id = daily('pills', [at(8)]);
        store.quietToday(europe);
        routines.markDone(id, Occurrence(Day(2026, 1, 6), at(8)));
        now = DateTime.utc(2026, 1, 7, 9);
        final entry = routines
            .history(routines.byId(id)!, europe)
            .entries
            .singleWhere((e) => e.occurrence.day == Day(2026, 1, 6));
        expect(entry.status, OccurrenceStatus.done);
      });
    });
  });

  group('A quiet day between Devices (obf-1044)', () {
    late DateTime Function() clock;
    late Store ann;

    setUp(() {
      clock = ticking();
      ann = device(clock, 'Ann');
    });

    test("it travels between one's own Devices", () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      ann.quietToday(europe);
      exchange(desk, ann);
      expect(desk.quietDays.ends, ann.quietDays.ends);
      expect(desk.quietDays.ends, isNotEmpty);
    });

    test("what another person's Device wrote counts for nothing", () {
      final ben = device(clock, 'Ben');
      final area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      // As a modified app would: Ben slips a quiet day into Ann's log.
      final forged = ben.append(
        PersonSettingKeys.entity,
        PersonSettingKeys.quietDay(Day.today(clock(), europe)),
        PersonSettingKeys.on,
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.quietDays.ends, isEmpty);
    });

    test('it never reaches another person', () {
      final ben = device(clock, 'Ben');
      final area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      ann.quietToday(europe);
      Notes(ann).capture('Milk', place: area);
      exchange(ben, ann);
      expect(ben.loggedHistory(PersonSettingKeys.entity), isEmpty);
      expect(ben.quietDays.ends, isEmpty);
    });
  });
}
