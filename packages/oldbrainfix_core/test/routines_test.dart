import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// Central European time: summer time (+2) ends on 2026-10-25 at 01:00
/// UTC, winter time (+1) after it.
DateTime europe(DateTime utc) => utc.add(
  Duration(hours: utc.isBefore(DateTime.utc(2026, 10, 25, 1)) ? 2 : 1),
);

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

void main() {
  late DateTime now;
  late Store store;
  late Notes notes;
  late Routines routines;
  final today = Day(2026, 10, 24);
  final tomorrow = Day(2026, 10, 25);

  setUp(() {
    // 09:00 on Saturday, 24 October 2026, in Europe.
    now = DateTime.utc(2026, 10, 24, 7);
    store = Store.inMemory(clock: () => now);
    notes = Notes(store);
    routines = Routines(store);
  });
  tearDown(() => store.close());

  Schedule daily(List<ClockTime> times, {Day? start}) => Schedule(
    start: start ?? today,
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  List<String> rows(List<AgendaRow> rows) => [
    for (final row in rows) '${row.occurrence.key} ${row.note?.text}',
  ];

  group('spawning', () {
    test('a Note spawns several Routines and Reminders, each linked back', () {
      final dentist = notes.capture('dentist Tuesday 10:00');
      final dayBefore = routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 10, 26), times: [at(18)]),
      );
      final hourBefore = routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 10, 27), times: [at(9)]),
      );
      expect(dayBefore.isReminder, isTrue);
      expect(routines.note(dayBefore)?.id, dentist.id);
      expect(routines.note(hourBefore)?.id, dentist.id);
      expect(routines.ofNote(dentist.id).map((r) => r.id).toSet(), {
        dayBefore.id,
        hourBefore.id,
      });
    });

    test('a Note that spawns a Routine leaves Unsorted, and stays out', () {
      final pills = notes.capture('pills');
      expect(notes.byId(pills.id)?.isUnsorted, isTrue);
      final routine = routines.spawn(pills.id, daily([at(8)]));
      expect(notes.byId(pills.id)?.isUnsorted, isFalse);
      routines.delete(routine.id);
      expect(routines.byId(routine.id), isNull);
      expect(notes.byId(pills.id)?.isUnsorted, isFalse);
    });

    test('a deleted place sends a spawning Note back to Unsorted', () {
      final places = Places(store);
      final health = places.addArea('Health');
      final pills = notes.capture('pills', place: health.id);
      routines.spawn(pills.id, daily([at(8)]));
      places.deleteArea(health.id, NotesGo.unsorted);
      expect(notes.byId(pills.id)?.isUnsorted, isTrue);
      expect(routines.ofNote(pills.id), hasLength(1));
    });

    test('creating a Routine directly captures its Note, not Unsorted', () {
      final routine = routines.create('  water the plants ', daily([]));
      final note = routines.note(routine);
      expect(note?.text, 'water the plants');
      expect(note?.captured, now);
      expect(note?.isUnsorted, isFalse);
      expect(notes.all().single.id, note?.id);
      expect(() => routines.create(' ', daily([])), throwsArgumentError);
    });

    test('only a Note in the lists spawns', () {
      final gone = notes.capture('gone');
      notes.trash(gone.id);
      expect(
        () => routines.spawn(gone.id, daily([at(8)])),
        throwsArgumentError,
      );
      expect(
        () => routines.spawn('note:none', daily([at(8)])),
        throwsArgumentError,
      );
      expect(routines.all(), isEmpty);
    });

    test('a merged Note\'s Routines link to the Note it went into', () {
      final pills = notes.capture('pills');
      final meds = notes.capture('medication');
      final routine = routines.spawn(pills.id, daily([at(8)]));
      notes.merge(pills.id, meds.id);
      expect(routines.note(routine)?.id, meds.id);
      expect(routines.ofNote(meds.id).single.id, routine.id);
    });

    test('the schedule survives the log, an advanced rule included', () {
      final note = notes.capture('first Monday: rent');
      final schedule = Schedule(
        start: Day(2026, 11, 1),
        times: [at(9, 30)],
        rule: Recurrence.parse('FREQ=MONTHLY;BYDAY=1MO;COUNT=12'),
      );
      final id = routines.spawn(note.id, schedule).id;
      final read = Routines(store).byId(id)?.schedule;
      expect(read?.start, Day(2026, 11, 1));
      expect(read?.times, [at(9, 30)]);
      expect(read?.rule?.format(), 'FREQ=MONTHLY;COUNT=12;BYDAY=1MO');
      final rescheduled = Schedule(start: today, times: [at(7)]);
      routines.reschedule(id, rescheduled);
      expect(routines.byId(id)?.isReminder, isTrue);
      expect(routines.byId(id)?.schedule.times, [at(7)]);
    });
  });

  group('Done', () {
    test('several times a day are checked off one by one', () {
      final note = notes.capture('pills');
      final id = routines.spawn(note.id, daily([at(8), at(14), at(20)])).id;
      final [morning, noon, evening] = routines.byId(id)!.schedule.on(today);
      routines.markDone(id, noon);
      final routine = routines.byId(id);
      expect(routine?.doneAt(noon), now);
      expect(routine?.doneAt(morning), isNull);
      expect(routine?.doneAt(evening), isNull);
      routines.unmarkDone(id, noon);
      expect(routines.byId(id)?.doneAt(noon), isNull);
    });

    test('Done keeps when it was tapped when it is applied later', () {
      final note = notes.capture('pills');
      final id = routines.spawn(note.id, daily([at(8)])).id;
      final [morning] = routines.byId(id)!.schedule.on(today);
      final tapped = now.subtract(const Duration(hours: 2));
      routines.markDone(id, morning, at: tapped);
      expect(routines.byId(id)?.doneAt(morning), tapped.toUtc());
    });

    test('Done early or late never moves the schedule (ADR 0006)', () {
      final note = notes.capture('water the plants');
      final id = routines
          .spawn(
            note.id,
            Schedule(
              start: Day(2026, 10, 19),
              times: [at(8)],
              rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=MO,TH,SA'),
            ),
          )
          .id;
      final before = routines
          .byId(id)!
          .schedule
          .between(Day(2026, 10, 19), Day(2026, 11, 30));
      // Thursday's, late on Saturday; next Monday's, early on Saturday.
      routines.markDone(id, Occurrence(Day(2026, 10, 22), at(8)));
      routines.markDone(id, Occurrence(Day(2026, 10, 26), at(8)));
      final after = routines
          .byId(id)!
          .schedule
          .between(Day(2026, 10, 19), Day(2026, 11, 30));
      expect(after, before);
      expect(
        routines.byId(id)?.doneAt(Occurrence(Day(2026, 10, 26), at(8))),
        now,
      );
    });

    test('only an Occurrence that is due can be Done', () {
      final note = notes.capture('pills');
      final id = routines.spawn(note.id, daily([at(8)])).id;
      expect(
        () => routines.markDone(id, Occurrence(today, at(9))),
        throwsArgumentError,
      );
      expect(
        () => routines.markDone(id, Occurrence(today.addDays(-1), at(8))),
        throwsArgumentError,
        reason: 'before the start',
      );
    });

    test('Done is kept when the schedule changes', () {
      final note = notes.capture('pills');
      final id = routines.spawn(note.id, daily([at(8)])).id;
      routines.markDone(id, Occurrence(today, at(8)));
      routines.reschedule(id, daily([at(8), at(20)]));
      expect(routines.byId(id)?.doneAt(Occurrence(today, at(8))), now);
    });
  });

  group('Re-nag override', () {
    test('a Routine has its own Re-nag interval, or the Device\'s', () {
      final id = routines.create('pills', daily([at(8)])).id;
      expect(routines.byId(id)?.renag, isNull);
      routines.setRenag(id, const Duration(minutes: 5));
      expect(routines.byId(id)?.renag, const Duration(minutes: 5));
      expect(
        () => routines.setRenag(id, const Duration(seconds: 30)),
        throwsArgumentError,
      );
      routines.setRenag(id, null);
      expect(routines.byId(id)?.renag, isNull);
    });

    test('the same schedule again writes nothing', () {
      final id = routines.create('pills', daily([at(8)])).id;
      final entries = store.allEntries().length;
      routines.reschedule(id, daily([at(8)]));
      expect(store.allEntries(), hasLength(entries));
      expect(routines.byId(id)?.scheduledAt, now);
      now = now.add(const Duration(hours: 1));
      routines.reschedule(id, daily([at(9)]));
      expect(store.allEntries(), hasLength(entries + 1));
      expect(routines.byId(id)?.scheduledAt, now);
    });
  });

  group('Agenda', () {
    test('today all-day first, then timed by time; then tomorrow', () {
      routines.create('pills', daily([at(20), at(8)]));
      routines.create('tidy up', daily([]));
      routines.create('dentist', Schedule(start: tomorrow, times: [at(10)]));
      routines.create('call Mum', Schedule(start: today, times: [at(12)]));
      final agenda = routines.agenda(europe);
      expect(agenda.today, today);
      expect(rows(agenda.todayRows), [
        '2026-10-24 tidy up',
        '2026-10-24T08:00 pills',
        '2026-10-24T12:00 call Mum',
        '2026-10-24T20:00 pills',
      ]);
      expect(rows(agenda.tomorrowRows), [
        '2026-10-25 tidy up',
        '2026-10-25T08:00 pills',
        '2026-10-25T10:00 dentist',
        '2026-10-25T20:00 pills',
      ]);
    });

    test('tomorrow\'s Occurrences can be checked off today', () {
      final routine = routines.create(
        'weekly report',
        Schedule(start: tomorrow, rule: Recurrence.parse('FREQ=WEEKLY')),
      );
      final row = routines.agenda(europe).tomorrowRows.single;
      routines.markDone(routine.id, row.occurrence);
      expect(routines.agenda(europe).tomorrowRows.single.done, now);
      expect(routines.byId(routine.id)?.schedule.next(tomorrow.addDays(1), 1), [
        Occurrence(Day(2026, 11, 1)),
      ]);
    });

    test('the day turns at the wall clock\'s midnight, across DST', () {
      final routine = routines.create('pills', daily([at(8)]));
      routines.markDone(routine.id, Occurrence(tomorrow, at(8)));
      // 00:30 on Sunday, before the clocks go back: Sunday's 08:00, Done.
      now = DateTime.utc(2026, 10, 24, 22, 30);
      var agenda = routines.agenda(europe);
      expect(agenda.today, tomorrow);
      expect(rows(agenda.todayRows), ['2026-10-25T08:00 pills']);
      expect(agenda.todayRows.single.done, isNotNull);
      // 23:30 on Sunday, winter time: still Sunday.
      now = DateTime.utc(2026, 10, 25, 22, 30);
      agenda = routines.agenda(europe);
      expect(agenda.today, tomorrow);
      expect(rows(agenda.tomorrowRows), ['2026-10-26T08:00 pills']);
    });

    test('a Reminder shows on its day only', () {
      routines.create('dentist', Schedule(start: Day(2026, 10, 27)));
      expect(routines.agenda(europe).todayRows, isEmpty);
      now = DateTime.utc(2026, 10, 26, 7);
      expect(rows(routines.agenda(europe).tomorrowRows), [
        '2026-10-27 dentist',
      ]);
      now = DateTime.utc(2026, 10, 28, 7);
      final agenda = routines.agenda(europe);
      expect(agenda.todayRows, isEmpty);
      expect(agenda.tomorrowRows, isEmpty);
    });

    test('a deleted Routine leaves the Agenda', () {
      final routine = routines.create('pills', daily([at(8)]));
      routines.delete(routine.id);
      expect(routines.agenda(europe).todayRows, isEmpty);
    });
  });
}
