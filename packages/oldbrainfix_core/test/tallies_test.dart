import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// The wall clock on UTC.
DateTime utcWall(DateTime utc) => utc;

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  /// A Tally made from a new Note with [text] and a field of each of
  /// [units], named after its unit; returns the Note and the field ids.
  (String, List<String>) tally(
    Store store,
    String text,
    List<String> units, {
    String? place,
  }) {
    final note = Notes(store).capture(text, place: place);
    final tallies = Tallies(store);
    return (
      note.id,
      [
        for (final unit in units)
          tallies.addField(note.id, name: unit, unit: unit),
      ],
    );
  }

  List<double?> values(List<TallyBucket> chart) => [
    for (final bucket in chart) bucket.value,
  ];

  group('units', () {
    test('known units are stored in metric and shown in either system', () {
      expect(TallyUnits.kindOf('kg'), UnitKind.mass);
      expect(TallyUnits.kindOf('pages'), UnitKind.typed);
      expect(TallyUnits.shownUnit('km', UnitSystem.imperial), 'mi');
      expect(TallyUnits.shownUnit('km', UnitSystem.metric), 'km');
      expect(TallyUnits.shownUnit('pages', UnitSystem.imperial), 'pages');
      expect(TallyUnits.toBase(5, 'km', UnitSystem.metric), 5000);
      expect(TallyUnits.toBase(1, 'lb', UnitSystem.imperial), 1);
      expect(TallyUnits.toBase(1, 'kg', UnitSystem.imperial), 0.45359237);
      expect(TallyUnits.toBase(212, '°C', UnitSystem.imperial), 100);
      expect(TallyUnits.fromBase(100, '°C', UnitSystem.imperial), 212);
      expect(TallyUnits.toBase(90, 'min', UnitSystem.imperial), 5400);
      expect(TallyUnits.toBase(12, 'pages', UnitSystem.imperial), 12);
      expect(TallyUnits.formatShown(8046.72, 'km', UnitSystem.imperial), '5');
      expect(
        TallyUnits.formatShown(82.5, 'kg', UnitSystem.metric, decimal: ','),
        '82,5',
      );
      expect(TallyUnits.formatShown(60, 'min', UnitSystem.metric), '1:00');
      expect(TallyUnits.formatShown(3725, 's', UnitSystem.metric), '1:02:05');
      expect(TallyUnits.parseShown('1:30', 'min', UnitSystem.metric), 90);
      expect(TallyUnits.parseShown('1,5', 'min', UnitSystem.metric), 90);
      expect(TallyUnits.parseShown('1:00:00', 's', UnitSystem.metric), 3600);
      expect(TallyUnits.parseShown('1:75', 'min', UnitSystem.metric), isNull);
      expect(TallyUnits.parseShown('two', 'kg', UnitSystem.metric), isNull);
      expect(TallyUnits.parseShown('1e3', 'kg', UnitSystem.metric), isNull);
      expect(TallyUnits.parseShown('-4', '°C', UnitSystem.metric), -4);
    });

    test('numbers are kept as millionths, written plain', () {
      expect(encodeTallyNumber(20), '20');
      expect(encodeTallyNumber(-3.5), '-3.5');
      expect(encodeTallyNumber(5 * 1609.344), '8046.72');
      expect(encodeTallyNumber(0.1 + 0.2), '0.3');
      expect(encodeTallyNumber(0.0000004), '0');
      expect(encodeTallyNumber(0.0000006), '0.000001');
      expect(() => encodeTallyNumber(double.nan), throwsArgumentError);
      expect(() => encodeTallyNumber(2e9), throwsArgumentError);
      expect(decodeTallyNumber('8046.72'), 8046.72);
      for (final bad in ['1.', '01', '1.50', '1e3', ' 1', '1.0000001', '']) {
        expect(decodeTallyNumber(bad), isNull, reason: bad);
      }
    });

    test('the system follows the region until one is chosen', () {
      expect(ann.unitSystem('US'), UnitSystem.imperial);
      expect(ann.unitSystem('de'), UnitSystem.metric);
      expect(ann.unitSystem(null), UnitSystem.metric);
      ann.chosenUnitSystem = UnitSystem.metric;
      expect(ann.unitSystem('US'), UnitSystem.metric);
      ann.chosenUnitSystem = null;
      expect(ann.unitSystem('US'), UnitSystem.imperial);
      expect(TallyUnits.defaultRule(UnitKind.mass), TallyRule.max);
      expect(TallyUnits.defaultRule(UnitKind.typed), TallyRule.sum);
    });
  });

  group('the Tally', () {
    test('a first field makes a Note a Tally and takes it out of Unsorted', () {
      final notes = Notes(ann);
      final tallies = Tallies(ann);
      final note = notes.capture('Push-ups');
      expect(tallies.byNote(note.id), isNull);
      expect(notes.unsorted().map((n) => n.id), [note.id]);
      final reps = tallies.addField(
        note.id,
        name: ' Push-ups ',
        unit: 'times',
        target: 20,
      );
      final weight = tallies.addField(note.id, name: 'Weight', unit: 'kg');
      expect(notes.unsorted(), isEmpty);
      final made = tallies.byNote(note.id)!;
      expect([for (final f in made.fields) f.id], [reps, weight]);
      expect(made.fields.first.name, 'Push-ups');
      expect(made.fields.first.rule, TallyRule.sum);
      expect(made.fields.last.rule, TallyRule.max);
      expect(made.targets, {reps: 20});
      expect([for (final t in tallies.all()) t.noteId], [note.id]);
      final gone = notes.capture('Gone');
      notes.trash(gone.id);
      expect(() => tallies.addField(gone.id, name: 'X'), throwsArgumentError);
      expect(() => tallies.addField(note.id, name: ' '), throwsArgumentError);
    });

    test('fields are renamed and hidden; hidden values return on restore', () {
      final tallies = Tallies(ann);
      final (note, [reps, minutes]) = tally(ann, 'Reading', ['pages', 'min']);
      tallies.add(note, values: {reps: 12, minutes: 1800});
      tallies.renameField(note, reps, 'Pages read');
      final before = ann.allEntries().length;
      tallies.renameField(note, reps, 'Pages read');
      expect(ann.allEntries().length, before);
      tallies.hideField(note, minutes);
      final hidden = tallies.byNote(note)!;
      expect([for (final f in hidden.shown) f.name], ['Pages read']);
      expect(hidden.field(minutes)?.hidden, isTrue);
      expect(tallies.entries(note).single.values[minutes], 1800);
      tallies.restoreField(note, minutes);
      expect(tallies.byNote(note)!.shown.length, 2);
      expect(values(tallies.chart(note, minutes, TallyPeriod.day, utcWall)), [
        1800,
      ]);
      // A field added later has no value in older entries.
      final added = tallies.addField(note, name: 'Chapters');
      expect(tallies.entries(note).single.values.containsKey(added), isFalse);
    });

    test('a field\'s kind of unit is fixed once it has entries', () {
      final tallies = Tallies(ann);
      final (note, [weight, other]) = tally(ann, 'Lifting', ['kg', 'reps']);
      // Without entries anything goes.
      tallies.setUnit(note, other, 'min');
      tallies.setUnit(note, other, 'reps');
      tallies.add(note, values: {weight: 80});
      tallies.setUnit(note, weight, 'g');
      expect(tallies.byNote(note)!.field(weight)?.unit, 'g');
      expect(() => tallies.setUnit(note, weight, 'km'), throwsStateError);
      expect(() => tallies.setUnit(note, weight, 'bags'), throwsStateError);
      // Stored in the base unit, the value stays what it was.
      expect(tallies.entries(note).single.values[weight], 80);
      // A removed entry keeps the kind fixed too.
      final entry = tallies.entries(note).single;
      tallies.trashEntry(entry.id);
      expect(() => tallies.setUnit(note, weight, 'km'), throwsStateError);
      tallies.setTarget(note, weight, 100);
      tallies.setRule(note, weight, TallyRule.sum);
      final field = tallies.byNote(note)!.field(weight)!;
      expect(field.target, 100);
      expect(field.rule, TallyRule.sum);
      tallies.setTarget(note, weight, null);
      expect(tallies.byNote(note)!.field(weight)?.target, isNull);
    });

    test('keys that are no field of a Tally are passed over', () {
      final (note, [reps]) = tally(ann, 'Push-ups', ['times']);
      // As a modified app would.
      ann.append(note, 'tally:name', 'Short');
      ann.append(note, 'tally::name', 'Empty');
      ann.append(note, 'tally:a:b:name', 'Colon');
      expect([for (final f in Tallies(ann).byNote(note)!.fields) f.id], [reps]);
      expect(Tallies(ann).all().length, 1);
    });
  });

  group('entries', () {
    test('are added, changed, removed and found by their line', () {
      final tallies = Tallies(ann);
      final (note, [reps]) = tally(ann, 'Push-ups', ['times']);
      final first = tallies.add(
        note,
        values: {reps: 20},
        line: ' after work ',
        at: DateTime.utc(2026, 2, 27, 18),
      );
      expect(first.line, 'after work');
      expect(first.values, {reps: 20});
      final second = tallies.add(note, values: {reps: 25});
      expect(
        [for (final e in tallies.entries(note)) e.id],
        [second.id, first.id],
      );
      expect(() => tallies.add(note), throwsArgumentError);
      expect(() => tallies.add(note, values: {'none': 1}), throwsArgumentError);
      expect(Search(ann).notes('AFTER').map((n) => n.id), [note]);
      tallies.editEntry(first.id, values: {reps: 22}, line: 'before work');
      expect(tallies.entryById(first.id)?.values, {reps: 22});
      final before = ann.allEntries().length;
      tallies.editEntry(first.id, values: {reps: 22}, line: 'before work');
      expect(ann.allEntries().length, before);
      tallies.trashEntry(first.id);
      expect([for (final e in tallies.entries(note)) e.id], [second.id]);
      expect(Search(ann).notes('before'), isEmpty);
      tallies.restoreEntry(first.id);
      expect(tallies.entries(note).length, 2);
      // The trash of the Note takes the Tally out of the lists.
      Notes(ann).trash(note);
      expect(tallies.all(), isEmpty);
      expect(() => tallies.add(note, values: {reps: 1}), throwsArgumentError);
    });

    test('a Tally merged into another Note shows there, with its entries', () {
      final notes = Notes(ann);
      final tallies = Tallies(ann);
      final (note, [reps]) = tally(ann, 'Push-ups', ['times']);
      tallies.add(note, values: {reps: 20});
      final into = notes.capture('Sport');
      notes.merge(note, into.id);
      final merged = tallies.byNote(into.id)!;
      expect(merged.fields.single.noteId, note);
      expect(tallies.entries(into.id).single.values, {reps: 20});
      tallies.add(into.id, values: {reps: 5});
      tallies.renameField(into.id, reps, 'Reps');
      expect(tallies.byNote(into.id)!.field(reps)?.name, 'Reps');
      expect(values(tallies.chart(into.id, reps, TallyPeriod.day, utcWall)), [
        25,
      ]);
      expect([for (final t in tallies.all()) t.noteId], [into.id]);
    });

    test('removed ones are listed apart, a merged Note\'s included', () {
      final notes = Notes(ann);
      final tallies = Tallies(ann);
      final (note, [reps]) = tally(ann, 'Push-ups', ['times']);
      final (other, [pages]) = tally(ann, 'Reading', ['pages']);
      final older = tallies.add(
        note,
        values: {reps: 20},
        at: DateTime.utc(2026, 2, 27, 18),
      );
      final kept = tallies.add(
        note,
        values: {reps: 25},
        at: DateTime.utc(2026, 2, 28, 18),
      );
      final elsewhere = tallies.add(other, values: {pages: 12});
      final into = notes.capture('Sport');
      notes.merge(note, into.id);
      final newer = tallies.add(
        into.id,
        values: {reps: 5},
        at: DateTime.utc(2026, 3, 1, 18),
      );
      tallies.trashEntry(newer.id);
      tallies.trashEntry(older.id);
      tallies.trashEntry(elsewhere.id);
      expect(
        [for (final e in tallies.removedEntries(into.id)) e.id],
        [newer.id, older.id],
      );
      expect([for (final e in tallies.entries(into.id)) e.id], [kept.id]);
      expect(tallies.removedEntries(note), isEmpty);
      tallies.restoreEntry(older.id);
      expect(
        [for (final e in tallies.removedEntries(into.id)) e.id],
        [newer.id],
      );
    });
  });

  group('charts', () {
    test('sum or largest per day, week, month and year', () {
      final tallies = Tallies(ann);
      final (note, [reps, weight]) = tally(ann, 'Gym', ['times', 'kg']);
      void add(DateTime at, double r, double w) =>
          tallies.add(note, values: {reps: r, weight: w}, at: at);
      // The clock is at Sunday 2026-03-01; its week began on the 23rd.
      add(DateTime.utc(2026, 1, 30, 8), 5, 70);
      add(DateTime.utc(2026, 2, 23, 8), 10, 80);
      add(DateTime.utc(2026, 2, 23, 18), 15, 82.5);
      add(DateTime.utc(2026, 2, 25, 8), 20, 81);
      add(DateTime.utc(2026, 3, 1, 8), 5, 79);
      // After today: in the table, not in the charts.
      add(DateTime.utc(2026, 3, 5, 8), 99, 99);
      final days = tallies.chart(note, reps, TallyPeriod.day, utcWall);
      expect(days.first.start, Day(2026, 1, 30));
      expect(days.last.start, Day(2026, 3, 1));
      expect(values(days).sublist(days.length - 7), [
        25,
        null,
        20,
        null,
        null,
        null,
        5,
      ]);
      expect(days[days.length - 7].entries, 2);
      expect(
        values(tallies.chart(note, weight, TallyPeriod.day, utcWall)).last,
        79,
      );
      final weeks = tallies.chart(note, reps, TallyPeriod.week, utcWall);
      expect(weeks.last.start, Day(2026, 2, 23));
      expect(weeks.last.end, Day(2026, 3, 1));
      expect(weeks.first.start, Day(2026, 1, 26));
      expect(values(weeks), [5, null, null, null, 50]);
      final months = tallies.chart(note, reps, TallyPeriod.month, utcWall);
      expect(
        [for (final b in months) b.start],
        [Day(2026, 1, 1), Day(2026, 2, 1), Day(2026, 3, 1)],
      );
      expect(months[1].end, Day(2026, 2, 28));
      expect(values(months), [5, 45, 5]);
      expect(values(tallies.chart(note, weight, TallyPeriod.month, utcWall)), [
        70,
        82.5,
        79,
      ]);
      expect(values(tallies.chart(note, reps, TallyPeriod.year, utcWall)), [
        55,
      ]);
      expect(tallies.chart(note, 'none', TallyPeriod.day, utcWall), isEmpty);
    });

    test('at most the latest thousand bars', () {
      final tallies = Tallies(ann);
      final (note, [reps]) = tally(ann, 'Push-ups', ['times']);
      tallies.add(note, values: {reps: 1}, at: DateTime.utc(2020, 1, 1));
      final days = tallies.chart(note, reps, TallyPeriod.day, utcWall);
      expect(days.length, Tallies.maxBuckets);
      expect(days.last.start, Day(2026, 3, 1));
      expect(days.every((b) => b.value == null), isTrue);
      expect(values(tallies.chart(note, reps, TallyPeriod.year, utcWall)), [
        1,
        null,
        null,
        null,
        null,
        null,
        null,
      ]);
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

    test('everyone records entries; each carries its author', () {
      final (note, [reps]) = tally(ann, 'Push-ups', ['times'], place: area);
      final own = Tallies(ann).add(note, values: {reps: 20});
      exchange(ben, ann);
      expect(ben.areaOf(own.id), area);
      expect(Tallies(ben).entries(note).single.id, own.id);
      final his = Tallies(ben).add(note, values: {reps: 30}, line: 'Ben');
      exchange(ben, ann);
      final tallies = Tallies(ann);
      expect([for (final e in tallies.entries(note)) e.id], [his.id, own.id]);
      expect(ann.authorOf(his.id)?.name, 'Ben');
      expect(ann.authorOf(own.id), isNull);
      // The user's own by default; everyone's, or one person's.
      const mine = PersonFilter(me: true);
      expect(
        [for (final e in tallies.entries(note, filter: mine)) e.id],
        [own.id],
      );
      final benId = ann.authorOf(his.id)!.id;
      expect(
        [
          for (final e in tallies.entries(
            note,
            filter: PersonFilter(people: {benId}),
          ))
            e.id,
        ],
        [his.id],
      );
      expect(
        values(
          tallies.chart(note, reps, TallyPeriod.day, utcWall, filter: mine),
        ),
        [20],
      );
      expect(values(tallies.chart(note, reps, TallyPeriod.day, utcWall)), [50]);
    });

    test('a Member changes only their own entries, never the fields', () {
      final (note, [reps]) = tally(ann, 'Push-ups', ['times'], place: area);
      final own = Tallies(ann).add(note, values: {reps: 20});
      exchange(ben, ann);
      expect(
        () => Tallies(ben).editEntry(own.id, values: {reps: 1}),
        throwsStateError,
      );
      expect(
        () => Tallies(ben).renameField(note, reps, 'Sit-ups'),
        throwsStateError,
      );
      // As a modified app would.
      final forged = [
        ben.append(own.id, TallyKeys.value(reps), '1'),
        ben.append(note, TallyKeys.field(reps, TallyKeys.name), 'Sit-ups'),
        ben.append(note, TallyKeys.field(reps, TallyKeys.hidden), 'true'),
      ];
      exchange(ben, ann);
      final held = ann.allEntries().map((e) => e.id).toSet();
      expect(held.containsAll([for (final e in forged) e.id]), isTrue);
      expect(Tallies(ann).entryById(own.id)?.values, {reps: 20});
      final field = Tallies(ann).byNote(note)!.field(reps)!;
      expect(field.name, 'times');
      expect(field.hidden, isFalse);
      // Their own Tally is theirs to change.
      final (his, [laps]) = tally(ben, 'Laps', ['m'], place: area);
      Tallies(ben).renameField(his, laps, 'Lanes');
      exchange(ben, ann);
      expect(Tallies(ann).byNote(his)?.field(laps)?.name, 'Lanes');
    });

    test('a Tally recorded in before its Note was filed in goes along', () {
      final (note, [reps]) = tally(ann, 'Push-ups', ['times']);
      final entry = Tallies(ann).add(note, values: {reps: 20});
      // Written in the Area since, so Ben's vector passes the entry.
      Notes(ann).capture('Milk', place: area);
      exchange(ben, ann);
      expect(Tallies(ben).entryById(entry.id), isNull);
      Notes(ann).file(note, area);
      exchange(ben, ann);
      expect(Tallies(ben).entries(note).single.id, entry.id);
      expect(Tallies(ben).byNote(note)?.field(reps)?.name, 'times');
    });
  });
}
