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
      // 09:00, due as they were made, still Re-nags; 10:00 comes next.
      expect(planned.first.at, DateTime.utc(2026, 10, 24, 7));
      expect(planned[16].at, DateTime.utc(2026, 10, 24, 8));
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
      routines.trash(gone);
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

    test('a snooze rings at its time and Re-nags from there until Done', () {
      // Made at 07:00, so this morning's 08:00 was asked for.
      now = DateTime.utc(2026, 10, 24, 5);
      final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
      now = DateTime.utc(2026, 10, 24, 7);
      final morning = Occurrence(Day(2026, 10, 24), at(8));
      alarms.applySnooze(id, morning.key, now.add(const Duration(minutes: 30)));
      expect(
        plan().first,
        'snooze 2026-10-24T08:00 2026-10-24T07:30:00.000Z pills',
      );
      expect(alarms.plan(europe).first.id, 'snooze:$id@2026-10-24T08:00');
      expect(
        alarms.plan(europe).where((a) => a.occurrence == morning),
        hasLength(1),
        reason: 'the snooze takes the place of the Occurrence alarm',
      );

      now = now.add(const Duration(minutes: 31));
      final [snooze, ...] = alarms.plan(europe);
      expect(snooze.kind, AlarmKind.snooze);
      expect(
        snooze.firesAfter(now),
        DateTime.utc(2026, 10, 24, 7, 40),
        reason: 'ignored, it Re-nags from the snooze on',
      );

      routines.markDone(id, morning);
      expect(plan().first, startsWith('occurrence 2026-10-25T08:00'));
    });

    test('a snooze past the next Occurrence is dropped', () {
      now = DateTime.utc(2026, 10, 24, 5);
      final id = routines
          .spawn(notes.capture('pills').id, daily([at(8), at(9)]))
          .id;
      now = DateTime.utc(2026, 10, 24, 6, 50);
      alarms.applySnooze(
        id,
        '2026-10-24T08:00',
        now.add(const Duration(minutes: 30)),
      );
      expect(alarms.plan(europe).first.id, 'occurrence:$id@2026-10-24T09:00');
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
      routines.trash(id);
      expect(plan(), isEmpty);
    });

    test('snoozes over for longer than the plan looks back are dropped', () {
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
      expect(alarms.snoozes, hasLength(2));
      now = now.add(const Duration(days: alarmHorizonDays + 2));
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
      routines.trash(id);
      expect(alarms.applyDone(id, '2026-10-24T08:00', now), isFalse);
    });

    group('Re-nag', () {
      /// A Routine made at 06:00 on the 24th, before its Occurrences.
      String early(String text, Schedule schedule) {
        final saved = now;
        now = DateTime.utc(2026, 10, 24, 4);
        final id = routines.spawn(notes.capture(text).id, schedule).id;
        now = saved;
        return id;
      }

      PlannedAlarm alarmOf(String id, Occurrence occurrence) => alarms
          .plan(europe)
          .singleWhere((a) => a.routineId == id && a.occurrence == occurrence);

      test('an ignored Occurrence fires again every 10 min by default', () {
        final id = early('pills', daily([at(8)]));
        final morning = alarmOf(id, Occurrence(Day(2026, 10, 24), at(8)));
        expect(morning.at, DateTime.utc(2026, 10, 24, 6));
        expect(morning.renag, const Duration(minutes: 10));
        expect(
          morning.firesAfter(now),
          DateTime.utc(2026, 10, 24, 7, 10),
          reason: 'rung at 08:00 and still Re-nagging at 09:00',
        );
      });

      test('the Device interval and a Routine\'s own one', () {
        final pills = early('pills', daily([at(8)]));
        final plants = early('plants', daily([at(8)]));
        store.renagInterval = const Duration(minutes: 30);
        routines.setRenag(pills, const Duration(minutes: 5));
        final morning = Occurrence(Day(2026, 10, 24), at(8));
        expect(alarmOf(pills, morning).renag, const Duration(minutes: 5));
        expect(alarmOf(plants, morning).renag, const Duration(minutes: 30));
        expect(
          alarmOf(pills, morning).firesAfter(now),
          DateTime.utc(2026, 10, 24, 7, 5),
        );
        routines.setRenag(pills, null);
        expect(alarmOf(pills, morning).renag, const Duration(minutes: 30));
      });

      test('stops when the next Occurrence starts; then it is Missed', () {
        final id = early('pills', daily([at(8), at(14)]));
        final morning = alarmOf(id, Occurrence(Day(2026, 10, 24), at(8)));
        expect(morning.ends, (day: Day(2026, 10, 24), time: at(14)));
        expect(morning.endsAt, DateTime.utc(2026, 10, 24, 12));
        expect(
          morning.firesAfter(DateTime.utc(2026, 10, 24, 11, 49)),
          DateTime.utc(2026, 10, 24, 11, 50),
        );
        expect(morning.firesAfter(DateTime.utc(2026, 10, 24, 11, 50)), isNull);

        now = DateTime.utc(2026, 10, 24, 12);
        expect(
          alarms
              .plan(europe)
              .where(
                (a) => a.occurrence == Occurrence(Day(2026, 10, 24), at(8)),
              ),
          isEmpty,
          reason: 'Missed now, so no longer planned',
        );
      });

      test('a Reminder stops at midnight', () {
        final id = early(
          'call mum',
          Schedule(start: Day(2026, 10, 24), times: [at(8)]),
        );
        final call = alarmOf(id, Occurrence(Day(2026, 10, 24), at(8)));
        expect(call.ends, (day: Day(2026, 10, 25), time: at(0)));
        expect(call.endsAt, DateTime.utc(2026, 10, 24, 22));
        expect(call.firesAfter(DateTime.utc(2026, 10, 24, 21, 55)), isNull);
        now = DateTime.utc(2026, 10, 24, 22);
        expect(alarms.plan(europe), isEmpty);
      });

      test('an all-day Occurrence nags until its next day begins', () {
        final id = early(
          'water plants',
          Schedule(
            start: Day(2026, 10, 1),
            rule: Recurrence.parse('FREQ=DAILY'),
          ),
        );
        final today = alarmOf(id, Occurrence(Day(2026, 10, 24)));
        expect(today.at, DateTime.utc(2026, 10, 24, 7));
        expect(today.ends, (day: Day(2026, 10, 25), time: at(0)));
      });

      test('the last Occurrence of a rule stops at midnight', () {
        final id = early(
          'antibiotics',
          Schedule(
            start: Day(2026, 10, 22),
            times: [at(8)],
            rule: Recurrence.parse('FREQ=DAILY;COUNT=3'),
          ),
        );
        final last = alarmOf(id, Occurrence(Day(2026, 10, 24), at(8)));
        expect(last.ends, (day: Day(2026, 10, 25), time: at(0)));
      });

      test('Escalation: after the ring and X ignored Re-nags, full screen', () {
        final pills = early('pills', daily([at(8)]));
        final morning = Occurrence(Day(2026, 10, 24), at(8));
        expect(alarmOf(pills, morning).escalation, isNull);
        expect(alarmOf(pills, morning).escalatesAt, isNull, reason: 'opt-in');

        routines.setEscalation(pills, defaultEscalation);
        final alarm = alarmOf(pills, morning);
        expect(alarm.escalation, 3);
        expect(
          alarm.escalatesAt,
          DateTime.utc(2026, 10, 24, 6, 40),
          reason: 'rung at 08:00, Re-nags at 08:10, 08:20, 08:30 ignored',
        );
        expect(
          alarm.toString(),
          contains('escalating after 3'),
          reason: 'a changed Escalation is armed anew',
        );

        routines.setRenag(pills, const Duration(minutes: 5));
        routines.setEscalation(pills, 1);
        expect(
          alarmOf(pills, morning).escalatesAt,
          DateTime.utc(2026, 10, 24, 6, 10),
        );
      });

      test('a snooze starts the count anew', () {
        final pills = early('pills', daily([at(8)]));
        routines.setEscalation(pills, 2);
        final morning = Occurrence(Day(2026, 10, 24), at(8));
        alarms.applySnooze(
          pills,
          morning.key,
          now.add(const Duration(minutes: 30)),
        );
        final snooze = alarmOf(pills, morning);
        expect(snooze.kind, AlarmKind.snooze);
        expect(snooze.escalation, 2);
        expect(snooze.escalatesAt, DateTime.utc(2026, 10, 24, 8));
      });

      test('no Escalation when the Occurrence is Missed before it', () {
        // 08:00 and 08:30: the morning one stops at 08:30.
        final pills = early('pills', daily([at(8), at(8, 30)]));
        now = DateTime.utc(2026, 10, 24, 5);
        routines.setEscalation(pills, 3);
        final morning = Occurrence(Day(2026, 10, 24), at(8));
        expect(alarmOf(pills, morning).escalatesAt, isNull);
        routines.setEscalation(pills, 1);
        expect(
          alarmOf(pills, morning).escalatesAt,
          DateTime.utc(2026, 10, 24, 6, 20),
        );
      });

      test('nothing due before the Routine was made Re-nags', () {
        // Made at 09:00, after this morning's 08:00.
        routines.spawn(notes.capture('pills').id, daily([at(8)]));
        expect(plan().first, startsWith('occurrence 2026-10-25T08:00'));
      });
    });

    group('Missed', () {
      List<String> summary() => [
        for (final r in alarms.missed(europe).routines)
          '${r.note?.text} ${r.count}: '
              '${[for (final m in r.missed) m.occurrence.key].join(', ')}',
      ];

      test('an Occurrence not Done is Missed when the next one starts', () {
        now = DateTime.utc(2026, 10, 20, 4);
        final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
        routines.markDone(id, Occurrence(Day(2026, 10, 21), at(8)));
        now = DateTime.utc(2026, 10, 24, 7);
        // 20th, 22nd and 23rd were Missed; the 24th is still open.
        expect(summary(), [
          'pills 3: 2026-10-23T08:00, 2026-10-22T08:00, 2026-10-20T08:00',
        ]);
        expect(alarms.missed(europe).count, 3);
        expect(
          alarms.missed(europe).nextChange,
          DateTime.utc(2026, 10, 25, 7),
          reason: 'the 24th becomes Missed when the 25th starts',
        );
      });

      test('a Reminder is Missed at midnight', () {
        now = DateTime.utc(2026, 10, 23, 4);
        routines.spawn(
          notes.capture('call mum').id,
          Schedule(start: Day(2026, 10, 24), times: [at(8)]),
        );
        now = DateTime.utc(2026, 10, 24, 21, 59);
        expect(summary(), isEmpty);
        now = DateTime.utc(2026, 10, 24, 22);
        expect(summary(), ['call mum 1: 2026-10-24T08:00']);
      });

      test('"seen" starts it over; later ones show again', () {
        now = DateTime.utc(2026, 10, 22, 4);
        routines.spawn(notes.capture('pills').id, daily([at(8)]));
        now = DateTime.utc(2026, 10, 24, 7);
        expect(alarms.missed(europe).count, 2);
        alarms.markSeen();
        expect(store.missedSeen, now);
        expect(summary(), isEmpty);
        now = DateTime.utc(2026, 10, 25, 8);
        expect(summary(), ['pills 1: 2026-10-24T08:00']);
      });

      test('checking one off late takes it out', () {
        now = DateTime.utc(2026, 10, 22, 4);
        final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
        now = DateTime.utc(2026, 10, 24, 7);
        final missed = Occurrence(Day(2026, 10, 22), at(8));
        routines.markDone(id, missed);
        expect(routines.byId(id)?.doneAt(missed), now, reason: 'Done late');
        expect(summary(), ['pills 1: 2026-10-23T08:00']);
      });

      test('the latest Missed Routine first; nothing from before it was '
          'made', () {
        now = DateTime.utc(2026, 10, 23, 4);
        routines.spawn(
          notes.capture('dentist').id,
          Schedule(start: Day(2026, 10, 23), times: [at(10)]),
        );
        // Daily since the 1st, but made on the 23rd at 06:00.
        routines.spawn(notes.capture('pills').id, daily([at(7)]));
        now = DateTime.utc(2026, 10, 24, 7);
        expect(summary(), [
          // Missed at 07:00 on the 24th.
          'pills 1: 2026-10-23T07:00',
          // Missed at midnight before.
          'dentist 1: 2026-10-23T10:00',
        ]);
      });

      test('saving the same schedule again keeps what was Missed', () {
        now = DateTime.utc(2026, 10, 22, 4);
        final id = routines.spawn(notes.capture('pills').id, daily([at(8)])).id;
        now = DateTime.utc(2026, 10, 24, 7);
        routines.reschedule(id, daily([at(8)]));
        routines.setRenag(id, const Duration(minutes: 5));
        expect(alarms.missed(europe).count, 2);
        routines.reschedule(id, daily([at(8), at(20)]));
        expect(
          summary(),
          isEmpty,
          reason: 'a new schedule was not asked for before now',
        );
      });

      test('looks back no further than the plan does', () {
        now = DateTime.utc(2026, 1, 1);
        routines.spawn(
          notes.capture('pills').id,
          daily([at(8)], start: Day(2026, 1, 1)),
        );
        now = DateTime.utc(2026, 10, 24, 7);
        expect(alarms.missed(europe).count, alarmHorizonDays);
      });
    });
  });
}
