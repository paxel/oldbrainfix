import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'alarms_test.dart' show at, europe;
import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  group('Heads-ups (obf-1046)', () {
    late DateTime now;
    late Store store;
    late Notes notes;
    late Routines routines;
    late Alarms alarms;

    setUp(() {
      // 09:00 on Saturday, 24 October 2026, in Europe; the clock goes
      // back from summer to winter time in the night to Sunday.
      now = DateTime.utc(2026, 10, 24, 7);
      store = Store.inMemory(clock: () => now);
      notes = Notes(store);
      routines = Routines(store);
      alarms = Alarms(store);
    });
    tearDown(() => store.close());

    String reminder(String text, Day day, [ClockTime? time]) => routines
        .spawn(notes.capture(text).id, Schedule(start: day, times: [?time]))
        .id;

    List<String> planned() => [
      for (final h in alarms.headsUps(europe))
        '${h.minutes} ${h.occurrence.key} ${h.at.toIso8601String()} '
            '${h.wall.day}T${h.wall.time}',
    ];

    group('kept', () {
      test('are read leniently, ascending and each once', () {
        final id = reminder('dentist', Day(2026, 10, 26), at(10));
        expect(routines.headsUps(id), isEmpty);
        // As another version or a modified app might write them.
        store.append(
          HeadsUpKeys.of(id),
          HeadsUpKeys.headsUps,
          '1440,020,+5, 5,abc,0,43201,20,1440,,43200,-3,0x10',
        );
        expect(routines.headsUps(id), [20, 1440, 43200]);
        expect(HeadsUpKeys.read(null), isEmpty);
        expect(HeadsUpKeys.read(''), isEmpty);
      });

      test('are written as canonical numbers; the same again writes '
          'nothing; none clears them', () {
        final id = reminder('dentist', Day(2026, 10, 26), at(10));
        List<Entry> written() => [
          for (final e in store.allEntries())
            if (e.entity == HeadsUpKeys.of(id)) e,
        ];
        routines.setHeadsUps(id, [1440, 20, 20, 0, -5, 50000]);
        expect(written().single.value, '20,1440');
        expect(routines.headsUps(id), [20, 1440]);
        routines.setHeadsUps(id, [20, 1440]);
        expect(written(), hasLength(1), reason: 'the same again');
        routines.setHeadsUps(id, []);
        expect(written().last.value, isNull);
        expect(routines.headsUps(id), isEmpty);
        expect(
          () => routines.setHeadsUps('routine:none', [20]),
          throwsArgumentError,
        );
      });

      test('a Routine in the trash keeps them and plans none', () {
        final id = reminder('dentist', Day(2026, 10, 26), at(10));
        routines.trash(id);
        routines.setHeadsUps(id, [20]);
        expect(routines.headsUps(id), [20]);
        expect(planned(), isEmpty);
        routines.restore(id);
        expect(planned(), hasLength(1));
      });
    });

    group('planned', () {
      test('before a timed Occurrence, a day back on the calendar across '
          'the switch', () {
        final id = reminder('dentist', Day(2026, 10, 25), at(10));
        routines.setHeadsUps(id, [20, 1440]);
        // 10:00 on Sunday is 09:00 UTC in winter time; 10:00 the day
        // before is 08:00 UTC in summer time, 25 hours earlier.
        expect(planned(), [
          '1440 2026-10-25T10:00 2026-10-24T08:00:00.000Z 2026-10-24T10:00',
          '20 2026-10-25T10:00 2026-10-25T08:40:00.000Z 2026-10-25T09:40',
        ]);
        final first = alarms.headsUps(europe).first;
        expect(first.id, 'headsUp:1440:$id@2026-10-25T10:00');
        expect(first.start, (day: Day(2026, 10, 25), time: at(10)));
        expect(first.text, 'dentist');
        expect(alarms.plan(europe), hasLength(1), reason: 'it still rings');
      });

      test('only those still ahead', () {
        final id = reminder('dentist', Day(2026, 10, 25), at(10));
        routines.setHeadsUps(id, [20, 1440]);
        now = DateTime.utc(2026, 10, 24, 8);
        expect(planned(), [
          '20 2026-10-25T10:00 2026-10-25T08:40:00.000Z 2026-10-25T09:40',
        ], reason: 'the one due now has rung');
        now = DateTime.utc(2026, 10, 25, 8, 50);
        expect(planned(), isEmpty);
      });

      test('none at or after the Occurrence starts', () {
        now = DateTime.utc(2026, 3, 28, 12);
        // On 29 March the clock jumps from 02:00 to 03:00: 02:10 is
        // skipped and fires at 03:00, as the Occurrence itself does.
        final id = reminder('night train', Day(2026, 3, 29), at(2, 30));
        routines.setHeadsUps(id, [20, 60]);
        expect(planned(), [
          '60 2026-03-29T02:30 2026-03-29T00:30:00.000Z 2026-03-29T01:30',
        ]);
      });

      test('an all-day Occurrence takes whole days, when its ring would '
          'ring', () {
        final id = reminder('bins out', Day(2026, 10, 27));
        routines.setHeadsUps(id, [30, 1440, 2880]);
        expect(planned(), [
          '2880 2026-10-27 2026-10-25T14:00:00.000Z 2026-10-25T15:00',
          '1440 2026-10-27 2026-10-26T14:00:00.000Z 2026-10-26T15:00',
        ]);
        // From 07:00, in the Quiet Hours: once they end, at 08:00.
        store.allDayFrom = at(7);
        expect(planned(), [
          '2880 2026-10-27 2026-10-25T07:00:00.000Z 2026-10-25T08:00',
          '1440 2026-10-27 2026-10-26T07:00:00.000Z 2026-10-26T08:00',
        ]);
        store.quietHoursOn = false;
        expect(planned(), [
          '2880 2026-10-27 2026-10-25T06:00:00.000Z 2026-10-25T07:00',
          '1440 2026-10-27 2026-10-26T06:00:00.000Z 2026-10-26T07:00',
        ]);
      });

      test('in the Quiet Hours without sound, whether it escalates or not', () {
        final id = reminder('pills', Day(2026, 10, 25), at(8, 10));
        routines.setHeadsUps(id, [5, 20]);
        final [early, late] = alarms.headsUps(europe);
        // 07:50 lies in 22:00-08:00; 08:05 does not.
        expect(early.minutes, 20);
        expect(early.quiet, QuietHours(at(22), at(8)));
        expect(early.silentAt(europe), isTrue);
        expect(early.toString(), contains('silent in 22:00-08:00'));
        expect(late.silentAt(europe), isFalse);
        routines.setEscalation(id, 3);
        expect(alarms.headsUps(europe).first.silentAt(europe), isTrue);
        store.quietHoursOn = false;
        expect(alarms.headsUps(europe).first.quiet, isNull);
        expect(alarms.headsUps(europe).first.silentAt(europe), isFalse);
      });

      test('Done, Skip or a snooze of the Occurrence cancels its own', () {
        final id = routines
            .spawn(
              notes.capture('feed the cat').id,
              Schedule(
                start: Day(2026, 10, 1),
                times: [at(18)],
                rule: Recurrence.parse('FREQ=DAILY'),
              ),
            )
            .id;
        routines.setHeadsUps(id, [20]);
        Occurrence on(int day) => Occurrence(Day(2026, 10, day), at(18));
        List<String> days() => [
          for (final h in alarms.headsUps(europe).take(2)) h.occurrence.key,
        ];
        expect(days(), ['2026-10-24T18:00', '2026-10-25T18:00']);
        routines.markDone(id, on(24));
        routines.skip(id, on(25));
        alarms.applySnooze(id, on(26).key, DateTime.utc(2026, 10, 26, 18));
        expect(days(), ['2026-10-27T18:00', '2026-10-28T18:00']);
      });

      test('at most so many per Routine', () {
        final id = routines
            .spawn(
              notes.capture('water').id,
              Schedule(
                start: Day(2026, 10, 1),
                times: [for (var h = 1; h < 24; h += 4) at(h)],
                rule: Recurrence.parse('FREQ=DAILY'),
              ),
            )
            .id;
        routines.setHeadsUps(id, [10]);
        expect(alarms.headsUps(europe), hasLength(alarmsPerRoutine));
      });
    });
  });

  group('Heads-ups are each person\'s own (obf-1046)', () {
    late DateTime Function() clock;
    late Store ann;
    late Store ben;

    setUp(() {
      clock = ticking();
      ann = device(clock, 'Ann');
      ben = device(clock, 'Ben');
    });

    /// Ann's Household shared with Ben as a Member, with her Reminder in
    /// it on 10 March 2026 at 10:00; returns the Reminder.
    String shared() {
      final area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      final id = Routines(ann)
          .spawn(
            Notes(ann).capture('Dentist', place: area).id,
            Schedule(start: Day(2026, 3, 10), times: [at(10)]),
          )
          .id;
      exchange(ben, ann);
      return id;
    }

    List<Entry> kept(Store store) => [
      for (final e in store.allEntries())
        if (e.entity.startsWith(HeadsUpKeys.prefix)) e,
    ];

    test('a Member sets their own; nobody else gets them', () {
      final id = shared();
      expect(ben.mayChangeRoutine(id), isFalse);
      Routines(ben).setHeadsUps(id, [20]);
      expect(Alarms(ben).headsUps(europe), hasLength(1));
      exchange(ben, ann);
      exchange(ann, ben);
      expect(kept(ann), isEmpty);
      expect(Routines(ann).headsUps(id), isEmpty);
      expect(Alarms(ann).headsUps(europe), isEmpty);
    });

    test('what another person\'s Device wrote counts for nothing', () {
      final id = shared();
      // As a modified app would: Ben slips Heads-ups into Ann's log.
      final forged = ben.append(HeadsUpKeys.of(id), HeadsUpKeys.headsUps, '20');
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(kept(ann).map((e) => e.id), contains(forged.id));
      expect(Routines(ann).headsUps(id), isEmpty);
      expect(Alarms(ann).headsUps(europe), isEmpty);
      // Nor does it stop her setting her own.
      Routines(ann).setHeadsUps(id, [20]);
      expect(Routines(ann).headsUps(id), [20]);
      expect(Alarms(ann).headsUps(europe), hasLength(1));
    });
  });
}
