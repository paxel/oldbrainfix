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
      routines.trash(routine.id);
      expect(routines.all(), isEmpty);
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

    test('Escalation is off until turned on, after a number of Re-nags', () {
      final id = routines.create('pills', daily([at(8)])).id;
      expect(routines.byId(id)?.escalation, isNull);
      routines.setEscalation(id, 3);
      expect(routines.byId(id)?.escalation, 3);
      final entries = store.allEntries().length;
      routines.setEscalation(id, 3);
      expect(store.allEntries(), hasLength(entries), reason: 'unchanged');
      expect(() => routines.setEscalation(id, 0), throwsArgumentError);
      routines.setEscalation(id, null);
      expect(routines.byId(id)?.escalation, isNull);
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

    test('over a week, the days after tomorrow cross a month end', () {
      routines.create('pills', daily([at(8)]));
      routines.create('dentist', Schedule(start: Day(2026, 11, 2)));
      expect(routines.agenda(europe).laterRows, isEmpty);
      now = DateTime.utc(2026, 10, 28, 11);
      final agenda = routines.agenda(europe, days: 7);
      expect(agenda.today, Day(2026, 10, 28));
      expect(agenda.laterRows, hasLength(5));
      expect(
        [for (var i = 0; i < 5; i++) '${agenda.later(i)}'],
        ['2026-10-30', '2026-10-31', '2026-11-01', '2026-11-02', '2026-11-03'],
      );
      expect(rows(agenda.laterRows[1]), ['2026-10-31T08:00 pills']);
      expect(rows(agenda.laterRows[3]), [
        '2026-11-02 dentist',
        '2026-11-02T08:00 pills',
      ]);
    });

    test('a trashed Routine leaves the Agenda', () {
      final routine = routines.create('pills', daily([at(8)]));
      routines.trash(routine.id);
      expect(routines.agenda(europe).todayRows, isEmpty);
    });
  });

  group('trash', () {
    late Alarms alarms;
    setUp(() => alarms = Alarms(store));

    Set<String> planned() => {for (final a in alarms.plan(europe)) a.routineId};

    Set<String> missed() => {
      for (final r in alarms.missed(europe).routines) r.routine.id,
    };

    test('a Routine deleted on its own goes to the trash, its Note stays', () {
      final pills = routines.create('pills', daily([at(8)]));
      final note = routines.note(pills);
      routines.trash(pills.id);
      expect(routines.all(), isEmpty);
      expect(routines.ofNote(note?.id ?? ''), isEmpty);
      expect(routines.byId(pills.id)?.isTrashed, isTrue);
      expect(routines.trashed().map((r) => r.id), [pills.id]);
      expect(planned(), isEmpty);
      expect(notes.all().single.isUnsorted, isFalse);
      // A trashed Routine is neither changed nor checked off.
      expect(
        () => routines.markDone(pills.id, Occurrence(today, at(8))),
        throwsArgumentError,
      );
      expect(alarms.applyDone(pills.id, '2026-10-24T08:00', now), isFalse);
    });

    test('the trash lists Routines on their own, latest deletion first', () {
      final a = routines.create('a', daily([at(8)]));
      final b = routines.create('b', daily([at(8)]));
      routines.trash(b.id);
      now = now.add(const Duration(minutes: 1));
      routines.trash(a.id);
      expect(routines.trashed().map((r) => r.id), [a.id, b.id]);
    });

    test('nothing is Missed or Re-nagged from its time in the trash; '
        'restored, it goes on as scheduled (ADR 0006)', () {
      final pills = routines.create('pills', daily([at(8), at(20)]));
      routines.trash(pills.id);
      // Two days in the trash, then restored at 09:00 on the 26th.
      now = DateTime.utc(2026, 10, 26, 8);
      expect(missed(), isEmpty);
      routines.restore(pills.id);
      expect(routines.byId(pills.id)?.isTrashed, isFalse);
      expect(routines.trashed(), isEmpty);
      expect(missed(), isEmpty);
      final next = alarms.plan(europe).first;
      expect(next.occurrence, Occurrence(Day(2026, 10, 26), at(20)));
      expect(routines.agenda(europe).todayRows, hasLength(2));
    });

    test('after 30 days in the trash a Routine is purged', () {
      final pills = routines.create('pills', daily([at(8)]));
      routines.trash(pills.id);
      final trashedAt = now;
      expect(
        routines.purgeAt(routines.trashed().single),
        trashedAt.add(Notes.trashKeep),
      );
      now = trashedAt.add(Notes.trashKeep - const Duration(minutes: 1));
      routines.purgeTrash();
      expect(routines.trashed().map((r) => r.id), [pills.id]);
      now = trashedAt.add(Notes.trashKeep);
      routines.purgeTrash();
      expect(routines.byId(pills.id), isNull);
      expect(routines.trashed(), isEmpty);
      expect(notes.all(), hasLength(1));
    });

    test('Yes: a Note goes to the trash with its Routines, and comes back '
        'with them', () {
      final dentist = notes.capture('dentist');
      final dayBefore = routines.spawn(
        dentist.id,
        Schedule(start: today, times: [at(18)]),
      );
      final hourBefore = routines.spawn(
        dentist.id,
        Schedule(start: tomorrow, times: [at(9)]),
      );
      routines.trashNote(dentist.id, withRoutines: true);
      expect(notes.byId(dentist.id)?.isTrashed, isTrue);
      expect(routines.all(), isEmpty);
      expect(planned(), isEmpty);
      expect(routines.agenda(europe).todayRows, isEmpty);
      // They are in the trash with the Note, not on their own.
      expect(routines.trashed(), isEmpty);
      expect(notes.trashed().map((n) => n.id), [dentist.id]);
      // The same moment as the Note's, so both are purged together.
      expect(routines.byId(dayBefore.id)?.trashed, isNotNull);
      expect(
        routines.byId(dayBefore.id)?.trashed,
        notes.byId(dentist.id)?.trashed,
      );

      routines.restoreNote(dentist.id);
      expect(notes.byId(dentist.id)?.isTrashed, isFalse);
      expect(routines.ofNote(dentist.id).map((r) => r.id).toSet(), {
        dayBefore.id,
        hourBefore.id,
      });
      expect(planned(), {dayBefore.id, hourBefore.id});
    });

    test('Yes: Note and Routines are purged together', () {
      final dentist = notes.capture('dentist');
      final reminder = routines.spawn(
        dentist.id,
        Schedule(start: tomorrow, times: [at(9)]),
      );
      routines.trashNote(dentist.id, withRoutines: true);
      now = now.add(Notes.trashKeep);
      notes.purgeTrash();
      routines.purgeTrash();
      expect(notes.byId(dentist.id), isNull);
      expect(routines.byId(reminder.id), isNull);
    });

    test('restoring a Note leaves a Routine trashed on its own in the '
        'trash', () {
      final pills = notes.capture('pills');
      final morning = routines.spawn(pills.id, daily([at(8)]));
      final evening = routines.spawn(pills.id, daily([at(20)]));
      routines.trash(morning.id);
      routines.trashNote(pills.id, withRoutines: true);
      routines.restoreNote(pills.id);
      expect(routines.ofNote(pills.id).map((r) => r.id), [evening.id]);
      expect(routines.trashed().map((r) => r.id), [morning.id]);
    });

    test('No: the Routines stay, linking to the Note in the trash and then '
        'to nothing once it is purged', () {
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, daily([at(20)]));
      routines.trashNote(pills.id, withRoutines: false);
      expect(routines.all().map((r) => r.id), [routine.id]);
      expect(routines.note(routine)?.isTrashed, isTrue);
      expect(planned(), {routine.id});

      now = now.add(Notes.trashKeep);
      notes.purgeTrash();
      routines.purgeTrash();
      expect(routines.all().map((r) => r.id), [routine.id]);
      expect(routines.note(routine), isNull);
    });

    test('No, then the Note comes back: the Routines were never away', () {
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, daily([at(20)]));
      routines.trashNote(pills.id, withRoutines: false);
      routines.restoreNote(pills.id);
      expect(routines.note(routine)?.isTrashed, isFalse);
      // A second deletion with Yes takes them along after all.
      routines.trashNote(pills.id, withRoutines: true);
      expect(routines.all(), isEmpty);
    });

    test('a merged Note\'s Routines go with the Note it went into', () {
      final target = notes.capture('dentist');
      final merged = notes.capture('dentist Tuesday');
      final routine = routines.spawn(merged.id, daily([at(8)]));
      notes.merge(merged.id, target.id);
      routines.trashNote(target.id, withRoutines: true);
      expect(routines.all(), isEmpty);
      routines.restoreNote(target.id);
      expect(routines.ofNote(target.id).map((r) => r.id), [routine.id]);
    });

    test('a merged Note\'s kept Routines link to the trashed target, then '
        'to nothing', () {
      final target = notes.capture('dentist');
      final merged = notes.capture('dentist Tuesday');
      final routine = routines.spawn(merged.id, daily([at(8)]));
      notes.merge(merged.id, target.id);
      routines.trashNote(target.id, withRoutines: false);
      final note = routines.note(routine);
      expect(note?.id, target.id);
      expect(note?.isTrashed, isTrue);
      now = now.add(Notes.trashKeep);
      notes.purgeTrash();
      expect(routines.note(routine), isNull);
      expect(routines.all().map((r) => r.id), [routine.id]);
    });
  });

  group('by name', () {
    Routine named(String id) => Routine(
      id: id,
      noteId: 'note:$id',
      schedule: Schedule(start: today),
    );
    List<String> order(Map<String, String> names) => [
      for (final r in routinesByName([
        for (final id in names.keys) named(id),
      ], (r) => names[r.id] ?? ''))
        r.id,
    ];

    test('alphabetical, whatever the case', () {
      expect(
        order({
          'routine:1': 'Cherry',
          'routine:2': 'apple',
          'routine:3': 'Banana',
        }),
        ['routine:2', 'routine:3', 'routine:1'],
      );
    });

    test('equal names go by id', () {
      expect(
        order({
          'routine:b': 'Pills',
          'routine:a': 'pills',
          'routine:c': 'PILLS',
        }),
        ['routine:a', 'routine:b', 'routine:c'],
      );
    });

    test('a name counts by its first line, trimmed', () {
      expect(
        order({
          'routine:1': '  Zebra',
          'routine:2': 'Mango\nAardvark',
          'routine:3': '\n Kiwi ',
        }),
        ['routine:3', 'routine:2', 'routine:1'],
      );
    });

    test('by code units, not a language: an umlaut after z', () {
      expect(
        order({'routine:1': 'Ärzte', 'routine:2': 'zoo', 'routine:3': 'Arzt'}),
        ['routine:3', 'routine:2', 'routine:1'],
      );
    });
  });
}
