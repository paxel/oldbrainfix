import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// The wall clock [days] days after the store's now: the tests' clock
/// moves by seconds, the Agenda by days. Day 0 is 2026-03-01.
WallClock onDay(int days) =>
    (utc) => utc.add(Duration(days: days));

Schedule _daily([List<ClockTime> times = const []]) => Schedule(
  start: Day(2026, 1, 1),
  times: times,
  rule: Recurrence(frequency: Frequency.daily),
);

final _eight = ClockTime(8, 0);
final _eightPm = ClockTime(20, 0);

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  /// A Checklist made from a new Note with [text]; returns the Note.
  String checklist(Store store, String text, {String? place}) {
    final id = Notes(store).capture(text, place: place).id;
    Checklists(store).make(id);
    return id;
  }

  String idOf(Store store, String note, String text) =>
      Checklists(store)
          .byNote(note)!
          .lines
          .firstWhere((l) => l.text == text)
          .id;

  /// The Occurrence of [routine] on [days] days on, at [time].
  RoutineOccurrence on(String routine, int days, [ClockTime? time]) => (
    routineId: routine,
    occurrence: Occurrence(Day(2026, 3, 1).addDays(days), time),
  );

  /// The Lines as shown for [at], `[x]` for one checked.
  List<String> shown(Store store, RoutineOccurrence at) => [
    for (final l
        in Checklists(store).forOccurrence(at)?.lines ?? const <Line>[])
      '${l.inGroup ? '  ' : ''}${l.checked ? '[x] ' : ''}${l.text}',
  ];

  DateTime? doneAt(Store store, RoutineOccurrence at) =>
      Routines(store).byId(at.routineId)?.doneAt(at.occurrence);

  group('each Occurrence', () {
    test('starts with every Line unchecked, and is Done once every Line is '
        'checked', () {
      final note = checklist(ann, 'Gym bag\nTowel\nShoes\n  Laces\nBottle');
      final lists = Checklists(ann);
      lists.check(idOf(ann, note, 'Towel'));
      final routine = Routines(ann).spawn(note, _daily()).id;
      final today = on(routine, 0);
      // Its own checks show for no Occurrence.
      expect(shown(ann, today), ['Towel', 'Shoes', '  Laces', 'Bottle']);
      lists.check(idOf(ann, note, 'Towel'), on: today);
      lists.check(idOf(ann, note, 'Laces'), on: today);
      expect(shown(ann, today), [
        '[x] Towel',
        '[x] Shoes',
        '  [x] Laces',
        'Bottle',
      ]);
      expect(doneAt(ann, today), isNull);
      lists.check(idOf(ann, note, 'Bottle'), on: today);
      final done = doneAt(ann, today);
      expect(done, isNotNull);
      final row = Routines(ann).agenda(onDay(0)).todayRows.single;
      expect(row.done, done);
      expect(lists.forOccurrence(today)!.on, today);
      // The next one starts afresh, with nothing written for it.
      final before = ann.allEntries().length;
      final tomorrow = on(routine, 1);
      expect(shown(ann, tomorrow), ['Towel', 'Shoes', '  Laces', 'Bottle']);
      expect(doneAt(ann, tomorrow), isNull);
      expect(ann.allEntries().length, before);
      // Every check lies on the Routine; no Line's own check moved.
      expect(lists.byNote(note)!.checkedCount, 1);
      // Unchecking a Line undoes the Done; checking it off by hand still
      // works, and keeps its moment.
      lists.uncheck(idOf(ann, note, 'Laces'), on: today);
      expect(shown(ann, today), [
        '[x] Towel',
        'Shoes',
        '  Laces',
        '[x] Bottle',
      ]);
      expect(doneAt(ann, today), isNull);
      Routines(ann).markDone(routine, tomorrow.occurrence);
      expect(doneAt(ann, tomorrow), isNotNull);
      expect(shown(ann, tomorrow), ['Towel', 'Shoes', '  Laces', 'Bottle']);
      // "Uncheck all" for one Occurrence.
      lists.check(idOf(ann, note, 'Shoes'), on: today);
      expect(doneAt(ann, today), isNotNull);
      lists.uncheckAll(note, on: today);
      expect(shown(ann, today), ['Towel', 'Shoes', '  Laces', 'Bottle']);
      expect(doneAt(ann, today), isNull);
    });

    test('a Reminder from a Checklist is Done the same way', () {
      final note = checklist(ann, 'Passport\nPhoto\nForm');
      final reminder = Routines(ann)
          .spawn(note, Schedule(start: Day(2026, 3, 4), times: [_eight]))
          .id;
      final at = on(reminder, 3, _eight);
      final lists = Checklists(ann);
      lists.check(idOf(ann, note, 'Photo'), on: at);
      lists.check(idOf(ann, note, 'Form'), on: at);
      expect(doneAt(ann, at), isNotNull);
      expect(lists.occurrenceNow(note, onDay(0)), at);
      expect(lists.occurrenceNow(note, onDay(9)), at);
    });

    test('a Line added after it was Done leaves it Done; a deleted one no '
        'longer counts; a list without Lines is never Done', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs');
      final routine = Routines(ann).spawn(note, _daily()).id;
      final lists = Checklists(ann);
      final today = on(routine, 0);
      lists.check(idOf(ann, note, 'Milk'), on: today);
      lists.check(idOf(ann, note, 'Eggs'), on: today);
      final done = doneAt(ann, today);
      expect(done, isNotNull);
      final bread = lists.add(note, 'Bread');
      expect(doneAt(ann, today), done);
      expect(shown(ann, today), ['[x] Milk', '[x] Eggs', 'Bread']);
      // Checked again since, the Line added counts.
      lists.uncheck(idOf(ann, note, 'Eggs'), on: today);
      lists.check(idOf(ann, note, 'Eggs'), on: today);
      expect(doneAt(ann, today), isNull);
      lists.delete(bread);
      expect(doneAt(ann, today), isNotNull);
      lists.restore(bread);
      expect(doneAt(ann, today), isNull);
      // No Line, no Done, whatever a modified app marks.
      final empty = checklist(ann, 'Empty');
      final idle = Routines(ann).spawn(empty, _daily()).id;
      ann.append(idle, ChecklistKeys.checkFor(today.occurrence, 'line:x'), '1');
      expect(doneAt(ann, on(idle, 0)), isNull);
      expect(
        () => lists.check(idOf(ann, note, 'Milk'), on: on(idle, 0)),
        throwsArgumentError,
      );
    });

    test('back to a plain Note and made again, what was Done stays', () {
      final note = checklist(ann, 'Tidy\nDesk\nBin');
      final routine = Routines(ann).spawn(note, _daily()).id;
      final today = on(routine, 0);
      Checklists(ann).check(idOf(ann, note, 'Desk'), on: today);
      Checklists(ann).check(idOf(ann, note, 'Bin'), on: today);
      final done = doneAt(ann, today);
      Checklists(ann).toPlain(note);
      expect(doneAt(ann, today), done);
      Checklists(ann).make(note);
      expect(doneAt(ann, today), done);
      expect(shown(ann, today), ['Desk', 'Bin']);
    });

    test('checks no app writes are passed over', () {
      final note = checklist(ann, 'Shop\nMilk');
      final other = checklist(ann, 'Other\nTea');
      final routine = Routines(ann).spawn(note, _daily()).id;
      final today = on(routine, 0);
      final key = today.occurrence.key;
      final milk = idOf(ann, note, 'Milk');
      // As a modified app would.
      for (final field in [
        '${ChecklistKeys.checkPrefix}$key',
        '${ChecklistKeys.checkPrefix}@$milk',
        '${ChecklistKeys.checkPrefix}2026-3-1@$milk',
        '${ChecklistKeys.checkPrefix}$key@line:',
        '${ChecklistKeys.checkPrefix}$key@$milk@x',
        '${ChecklistKeys.checkPrefix}$key@note:x',
        ChecklistKeys.checkFor(today.occurrence, idOf(ann, other, 'Tea')),
      ]) {
        ann.append(routine, field, '2026-03-01T09:00:00.000Z');
      }
      expect(shown(ann, today), ['Milk']);
      expect(doneAt(ann, today), isNull);
      expect(LineChecks.read(ann, routine).byOccurrence.keys, [key]);
      // A Routine's Occurrence that is not due, or of another Note, takes
      // no check.
      final lists = Checklists(ann);
      expect(
        () => lists.check(milk, on: on(routine, 0, _eight)),
        throwsArgumentError,
      );
      final elsewhere = Routines(ann).spawn(other, _daily()).id;
      expect(
        () => lists.check(milk, on: on(elsewhere, 0)),
        throwsArgumentError,
      );
      expect(Checklists(ann).forOccurrence(on('routine:none', 0)), isNull);
    });
  });

  group('shown now', () {
    test('the Occurrence of the latest day up to today: the first not Done, '
        'else the last; before the first day, the first', () {
      final note = checklist(ann, 'Pills\nBlue\nRed');
      final lists = Checklists(ann);
      final blue = idOf(ann, note, 'Blue');
      final red = idOf(ann, note, 'Red');
      lists.check(blue);
      // Without a Routine, its own checks.
      expect(lists.occurrenceNow(note, onDay(0)), isNull);
      expect(lists.shownNow(note, onDay(0))!.checkedCount, 1);
      expect(lists.shownNow(note, onDay(0))!.on, isNull);
      final routine = Routines(ann).spawn(note, _daily([_eight, _eightPm])).id;
      final morning = on(routine, 0, _eight);
      expect(lists.occurrenceNow(note, onDay(0)), morning);
      expect(lists.shownNow(note, onDay(0))!.checkedCount, 0);
      lists.check(blue, on: morning);
      lists.check(red, on: morning);
      final evening = on(routine, 0, _eightPm);
      expect(lists.occurrenceNow(note, onDay(0)), evening);
      expect(lists.shownNow(note, onDay(0))!.checkedCount, 0);
      Routines(ann).markDone(routine, evening.occurrence);
      expect(lists.occurrenceNow(note, onDay(0)), evening);
      expect(lists.occurrenceNow(note, onDay(1)), on(routine, 1, _eight));
      // A weekly one stays on its day until the next comes.
      final weekly = checklist(ann, 'Bins\nPaper');
      final mondays = Routines(ann)
          .spawn(
            weekly,
            Schedule(
              start: Day(2026, 1, 5),
              rule: Recurrence(
                frequency: Frequency.weekly,
                byDay: [WeekdayRule(DateTime.monday)],
              ),
            ),
          )
          .id;
      expect(lists.occurrenceNow(weekly, onDay(0)), on(mondays, -6));
      expect(lists.occurrenceNow(weekly, onDay(1)), on(mondays, 1));
      expect(lists.occurrenceNow(weekly, onDay(7)), on(mondays, 1));
      expect(lists.occurrenceNow(weekly, onDay(8)), on(mondays, 8));
      // Before its first day, the first; of two Routines, the earlier.
      final later = checklist(ann, 'Trip\nTent');
      final trip = Routines(ann)
          .spawn(later, Schedule(start: Day(2026, 3, 10)))
          .id;
      expect(lists.occurrenceNow(later, onDay(0)), on(trip, 9));
      final soon = Routines(ann)
          .spawn(later, Schedule(start: Day(2026, 3, 5)))
          .id;
      expect(lists.occurrenceNow(later, onDay(0)), on(soon, 4));
      expect(lists.occurrenceNow(later, onDay(6)), on(soon, 4));
      expect(lists.occurrenceNow(later, onDay(9)), on(trip, 9));
      // In the trash, the Routine leaves it to its own checks.
      Routines(ann).trash(routine);
      expect(lists.occurrenceNow(note, onDay(0)), isNull);
      expect(lists.shownNow(note, onDay(0))!.checkedCount, 1);
      // Many at once, as one by one.
      final ids = [note, weekly, later, Notes(ann).capture('Plain').id];
      final atOnce = lists.ofNotesNow(ids, onDay(1));
      expect(atOnce.keys, [note, weekly, later]);
      for (final id in atOnce.keys) {
        expect(atOnce[id]!.on, lists.shownNow(id, onDay(1))!.on);
      }
    });
  });

  group("between one's own Devices", () {
    test('checks travel; the next Occurrence starts afresh on both', () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      final note = checklist(ann, 'Gym bag\nTowel\nShoes');
      final routine = Routines(ann).spawn(note, _daily()).id;
      exchange(desk, ann);
      final today = on(routine, 0);
      Checklists(desk).check(idOf(desk, note, 'Towel'), on: today);
      Checklists(ann).check(idOf(ann, note, 'Shoes'), on: today);
      exchange(desk, ann);
      exchange(desk, ann);
      final done = doneAt(ann, today);
      expect(done, isNotNull);
      expect(doneAt(desk, today), done);
      expect(shown(desk, on(routine, 1)), ['Towel', 'Shoes']);
      expect(shown(ann, on(routine, 1)), ['Towel', 'Shoes']);
    });

    test('a check another person\'s Device slipped in counts for nothing', () {
      final ben = device(clock, 'Ben');
      final area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      final note = checklist(ann, 'Shop\nMilk');
      final routine = Routines(ann).spawn(note, _daily()).id;
      final today = on(routine, 0);
      // As a modified app would: Ben slips a check into Ann's own list.
      final forged = ben.append(
        routine,
        ChecklistKeys.checkFor(today.occurrence, idOf(ann, note, 'Milk')),
        '2026-03-01T09:00:00.000Z',
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(shown(ann, today), ['Milk']);
      expect(doneAt(ann, today), isNull);
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = device(clock, 'Ben');
      area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });

    test('a Member checks for everyone; the last Line makes it Done for '
        'all', () {
      final note = checklist(ann, 'Shop\nMilk\nEggs', place: area);
      final routine = Routines(ann).spawn(note, _daily()).id;
      exchange(ben, ann);
      expect(ben.areaOf(routine), area);
      final today = on(routine, 0);
      Checklists(ben).check(idOf(ben, note, 'Milk'), on: today);
      Checklists(ann).check(idOf(ann, note, 'Eggs'), on: today);
      expect(doneAt(ann, today), isNull);
      exchange(ben, ann);
      exchange(ben, ann);
      final done = doneAt(ann, today);
      expect(done, isNotNull);
      expect(doneAt(ben, today), done);
      expect(shown(ann, today), ['[x] Milk', '[x] Eggs']);
      // The next Occurrence starts afresh for everyone.
      expect(shown(ben, on(routine, 1)), ['Milk', 'Eggs']);
      expect(Checklists(ben).occurrenceNow(note, onDay(1)), on(routine, 1));
    });
  });
}
