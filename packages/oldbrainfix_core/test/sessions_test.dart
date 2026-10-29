import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// The wall clock [days] days after the store's now: the tests' clock
/// moves by seconds, the Agenda by days. Day 0 is a Sunday.
WallClock onDay(int days) =>
    (utc) => utc.add(Duration(days: days));

final _daily = Schedule(
  start: Day(2026, 1, 1),
  rule: Recurrence(frequency: Frequency.daily),
);

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  /// A Tally with one field, Times with a target of 20, as its Note.
  String tally(Store store, String name, {String? place}) {
    final note = Notes(store).capture(name, place: place).id;
    Tallies(store).addField(note, name: 'Times', unit: 'times', target: 20);
    return note;
  }

  List<String> tallies(Store store, List<String> names, {String? place}) => [
    for (final name in names) tally(store, name, place: place),
  ];

  Session session(
    Store store,
    List<String> pool, {
    int count = 2,
    SessionRule rule = SessionRule.rolling,
    String? place,
  }) => Sessions(store).spawn(
    Notes(store).capture('Sport', place: place).id,
    _daily,
    pool: pool,
    count: count,
    rule: rule,
  );

  /// Today's Occurrence of [s] on [days] days on.
  Occurrence day(Store store, int days) =>
      Occurrence(Day.today(store.clock(), onDay(days)));

  /// The picks of [s] on [days] days on, written as the Agenda does.
  List<String> picked(Store store, Session s, int days) => [
    for (final today in Sessions(store).pickToday(onDay(days)))
      if (today.session.id == s.id)
        for (final pick in today.picks) pick.noteId,
  ];

  /// Records the targets for [note], picked for [s] [days] days on.
  TallyEntry record(Store store, Session s, int days, String note) =>
      Sessions(store).record(
        s.id,
        day(store, days),
        note,
        values: Tallies(store).byNote(note)?.targets ?? const {},
      );

  /// The picks of [s] on [days] days on, written nowhere.
  List<String> shown(Store store, Session s, int days) => [
    for (final pick in Sessions(store).on(s.id, day(store, days)).picks)
      pick.noteId,
  ];

  group('the Session', () {
    test('is a Routine from a Note, with a pool, a count and a rule', () {
      final notes = Notes(ann);
      final sessions = Sessions(ann);
      final [a, b, c] = tallies(ann, ['Push-ups', 'Sit-ups', 'Wall sit']);
      final milk = notes.capture('Milk').id;
      final s = session(ann, [a, b, a, c], rule: SessionRule.least);
      expect(s.pool, [a, b, c]);
      expect(s.count, 2);
      expect(s.rule, SessionRule.least);
      expect(Routines(ann).byId(s.id)?.noteId, s.routine.noteId);
      expect(notes.unsorted().map((n) => n.id), [milk]);
      expect(sessions.choices(s.routine.noteId), [c, b, a]);
      expect(sessions.pool(s), [a, b, c]);
      // A pool of Tallies, at least one; a count of 1 to 99.
      final note = notes.capture('Learn').id;
      Session spawn(List<String> pool, int count) => sessions.spawn(
        note,
        _daily,
        pool: pool,
        count: count,
        rule: SessionRule.random,
      );
      expect(() => spawn([milk], 1), throwsArgumentError);
      expect(() => spawn([], 1), throwsArgumentError);
      expect(() => spawn([a], 0), throwsArgumentError);
      expect(() => spawn([a], 100), throwsArgumentError);
      // Changed: what is the same writes nothing.
      sessions.setCount(s.id, 3);
      sessions.setCountOn(s.id, DateTime.saturday, 1);
      sessions.setRule(s.id, SessionRule.random);
      sessions.setPool(s.id, [c, a]);
      final changed = sessions.byId(s.id)!;
      expect(changed.count, 3);
      expect(changed.counts, {DateTime.saturday: 1});
      expect(changed.rule, SessionRule.random);
      expect(changed.pool, [c, a]);
      final before = ann.allEntries().length;
      sessions.setCount(s.id, 3);
      sessions.setPool(s.id, [c, a]);
      expect(ann.allEntries().length, before);
      sessions.setCountOn(s.id, DateTime.saturday, null);
      expect(sessions.byId(s.id)?.counts, isEmpty);
      // A plain Routine is none.
      final walk = Routines(ann).create('Walk', _daily);
      expect(sessions.byId(walk.id), isNull);
      expect([for (final s in sessions.all()) s.id], [s.id]);
    });

    test('what no app writes is passed over', () {
      final [a, b, c] = tallies(ann, ['A', 'B', 'C']);
      final s = session(ann, [a, b, c]);
      final today = day(ann, 0);
      // As a modified app would.
      ann.append(s.id, SessionKeys.count, '+3');
      ann.append(s.id, SessionKeys.countOn(8), '5');
      ann.append(s.id, '${SessionKeys.count}:0', '5');
      ann.append(s.id, SessionKeys.countOn(DateTime.sunday), '07');
      ann.append(s.id, SessionKeys.rule, 'sometimes');
      ann.append(s.id, SessionKeys.pickPrefix, a);
      ann.append(s.id, '${SessionKeys.pickPrefix}2026-3-1', a);
      ann.append(s.id, SessionKeys.pick(today), 'x,$c,$c,note:,note:a@b,');
      ann.append(s.id, '${SessionKeys.entryPrefix}${today.key}', c);
      ann.append(s.id, '${SessionKeys.entryPrefix}${today.key}@', c);
      ann.append(s.id, '${SessionKeys.entryPrefix}@tallyEntry:x', c);
      ann.append(s.id, SessionKeys.entry(today, 'note:x'), c);
      final read = Sessions(ann).byId(s.id)!;
      expect(read.count, 1);
      expect(read.counts, isEmpty);
      expect(read.rule, SessionRule.rolling);
      final marks = SessionMarks.read(ann, s.id);
      expect(marks.picks, {
        today.key: [c],
      });
      expect(marks.entries, isEmpty);
      expect(shown(ann, s, 0), [c]);
      expect(Routines(ann).byId(s.id)?.done, isEmpty);
    });
  });

  group('picking', () {
    test('Rolling goes through the pool, continuing where it stopped', () {
      final [a, b, c, d, e] = tallies(ann, ['A', 'B', 'C', 'D', 'E']);
      final s = session(ann, [a, b, c, d, e]);
      expect(picked(ann, s, 0), [a, b]);
      expect(picked(ann, s, 1), [c, d]);
      expect(picked(ann, s, 2), [e, a]);
      // A day nobody looked picks nothing.
      expect(picked(ann, s, 4), [b, c]);
      expect(picked(ann, s, 5), [d, e]);
      expect(picked(ann, s, 6), [a, b]);
      // Called again the same day, it writes nothing.
      final before = ann.allEntries().length;
      expect(picked(ann, s, 6), [a, b]);
      expect(ann.allEntries().length, before);
    });

    test('Least selected picks those with the fewest entries first', () {
      final [a, b] = tallies(ann, ['A', 'B']);
      final s = session(ann, [a, b], count: 1, rule: SessionRule.least);
      expect(picked(ann, s, 0), [a]);
      // A skipped, B recorded: A comes back.
      expect(picked(ann, s, 1), [b]);
      record(ann, s, 1, b);
      expect(picked(ann, s, 2), [a]);
      // Entries from anywhere count.
      Tallies(ann).add(a, line: 'Morning');
      Tallies(ann).add(a, line: 'Evening');
      expect(picked(ann, s, 3), [b]);
    });

    test('Random draws by a hash, the same on every Device', () {
      final pool = tallies(ann, ['A', 'B', 'C', 'D', 'E']);
      final s = session(ann, pool, count: 3, rule: SessionRule.random);
      List<String> drawn(int days) {
        final key = day(ann, days).key;
        String hash(String id) =>
            sha256.convert(utf8.encode('${s.id}\n$key\n$id')).toString();
        return ([
          ...pool,
        ]..sort((x, y) => hash(x).compareTo(hash(y)))).take(3).toList();
      }

      for (var days = 0; days < 4; days++) {
        expect(shown(ann, s, days), drawn(days));
        expect(picked(ann, s, days), drawn(days));
      }
    });

    test('a weekday has a count of its own', () {
      final pool = tallies(ann, ['A', 'B', 'C', 'D']);
      final s = session(ann, pool, count: 1);
      Sessions(ann).setCountOn(s.id, DateTime.saturday, 3);
      expect(day(ann, 6).day.weekday, DateTime.saturday);
      expect(shown(ann, s, 5), hasLength(1));
      expect(shown(ann, s, 6), hasLength(3));
      // More than the pool holds picks the pool.
      Sessions(ann).setCountOn(s.id, DateTime.saturday, 9);
      expect(shown(ann, s, 6), hasLength(4));
    });

    test('"Another one" swaps in place; the one swapped keeps its place', () {
      final sessions = Sessions(ann);
      final [a, b, c, d] = tallies(ann, ['A', 'B', 'C', 'D']);
      final s = session(ann, [a, b, c, d]);
      final today = day(ann, 0);
      expect(picked(ann, s, 0), [a, b]);
      sessions.another(s.id, today, a);
      expect(shown(ann, s, 0), [c, b]);
      sessions.another(s.id, today, c);
      expect(shown(ann, s, 0), [d, b]);
      // Every one offered: round again.
      sessions.another(s.id, today, d);
      expect(shown(ann, s, 0), [a, b]);
      expect(() => sessions.another(s.id, today, c), throwsArgumentError);
      // Swapped out, C and D come first the next day.
      expect(picked(ann, s, 1), [c, d]);
      // Nothing to swap in, nothing written.
      final [x, y] = tallies(ann, ['X', 'Y']);
      final two = session(ann, [x, y]);
      final pick = Sessions(ann).pickToday(onDay(0));
      expect(pick.firstWhere((o) => o.session.id == two.id).canSwap, isFalse);
      final before = ann.allEntries().length;
      sessions.another(two.id, today, x);
      expect(ann.allEntries().length, before);
    });

    test('a Tally gone from the pool is replaced, unless it has an entry', () {
      final [a, b, c] = tallies(ann, ['A', 'B', 'C']);
      final s = session(ann, [a, b, c]);
      expect(picked(ann, s, 0), [a, b]);
      record(ann, s, 0, b);
      Notes(ann).trash(b);
      Notes(ann).trash(a);
      expect(picked(ann, s, 0), [c, b]);
      expect(Sessions(ann).pool(Sessions(ann).byId(s.id)!), [c]);
    });
  });

  group('Done', () {
    test('once every pick has an entry, or when checked off', () {
      final routines = Routines(ann);
      final [a, b, c] = tallies(ann, ['A', 'B', 'C']);
      final s = session(ann, [a, b, c]);
      final today = day(ann, 0);
      expect(picked(ann, s, 0), [a, b]);
      final first = Sessions(ann).record(s.id, today, a, line: 'Morning');
      expect(first.values, isEmpty);
      expect(first.line, 'Morning');
      expect(routines.byId(s.id)?.doneAt(today), isNull);
      // An entry made on the Tally itself is no pick's.
      Tallies(ann).add(b, line: 'Elsewhere');
      expect(routines.byId(s.id)?.doneAt(today), isNull);
      final fields = Tallies(ann).byNote(b)!.targets;
      final last = Sessions(ann)
          .record(s.id, today, b, values: fields, line: 'Done');
      expect(last.values, fields);
      expect(routines.byId(s.id)?.doneAt(today), last.at);
      final row = routines.agenda(onDay(0)).todayRows.single;
      expect(row.done, last.at);
      final occurrence = Sessions(ann).on(s.id, today);
      expect(occurrence.complete, isTrue);
      expect(
        [for (final p in occurrence.picks) p.entries.single.id],
        [first.id, last.id],
      );
      // Removed, the entry takes Done with it.
      Tallies(ann).trashEntry(last.id);
      expect(routines.byId(s.id)?.doneAt(today), isNull);
      // A check-off makes it Done, and keeps its moment.
      final at = DateTime.utc(2026, 3, 1, 12);
      routines.markDone(s.id, today, at: at);
      Tallies(ann).restoreEntry(last.id);
      expect(routines.byId(s.id)?.doneAt(today), at);
      // Only a pick takes an entry.
      expect(() => record(ann, s, 0, c), throwsArgumentError);
    });
  });

  group("between one's own Devices", () {
    test('picks, swaps and entries travel', () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      final [a, b, c] = tallies(ann, ['A', 'B', 'C']);
      final s = session(ann, [a, b, c]);
      final today = day(ann, 0);
      expect(picked(ann, s, 0), [a, b]);
      exchange(desk, ann);
      expect(shown(desk, s, 0), [a, b]);
      final before = desk.allEntries().length;
      expect(picked(desk, s, 0), [a, b]);
      expect(desk.allEntries().length, before);
      Sessions(desk).another(s.id, today, a);
      record(desk, s, 0, c);
      exchange(desk, ann);
      expect(shown(ann, s, 0), [c, b]);
      record(ann, s, 0, b);
      exchange(desk, ann);
      final done = Routines(ann).byId(s.id)?.doneAt(today);
      expect(done, isNotNull);
      expect(Routines(desk).byId(s.id)?.doneAt(today), done);
      // The next day goes on alike.
      expect(shown(desk, s, 1), shown(ann, s, 1));
    });

    test('a pick another person\'s Device slipped in counts for nothing', () {
      final ben = device(clock, 'Ben');
      final area = ann.share(Places(ann).addArea('Household').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
      final [a, b, c] = tallies(ann, ['A', 'B', 'C']);
      final s = session(ann, [a, b, c]);
      final today = day(ann, 0);
      expect(picked(ann, s, 0), [a, b]);
      // As a modified app would: Ben slips marks into Ann's own Session.
      final forged = [
        ben.append(s.id, SessionKeys.pick(today), c),
        ben.append(s.id, SessionKeys.entry(today, 'tallyEntry:x'), c),
      ];
      ann.applyEntries(forged, verifiedDevice: ben.deviceId);
      final held = ann.allEntries().map((e) => e.id);
      expect(held, containsAll([for (final e in forged) e.id]));
      expect(shown(ann, s, 0), [a, b]);
      expect(SessionMarks.read(ann, s.id).entries, isEmpty);
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

    test('everyone gets the same picks; Done is shared, from anyone', () {
      final [a, b, c] = tallies(ann, ['A', 'B', 'C'], place: area);
      final s = session(ann, [a, b, c], rule: SessionRule.least, place: area);
      final today = day(ann, 0);
      expect(picked(ann, s, 0), [a, b]);
      exchange(ben, ann);
      expect(ben.areaOf(s.id), area);
      final before = ben.allEntries().length;
      expect(picked(ben, s, 0), [a, b]);
      expect(ben.allEntries().length, before);
      // One entry each: one Done for both.
      record(ben, s, 0, a);
      record(ann, s, 0, b);
      expect(Routines(ann).byId(s.id)?.doneAt(today), isNull);
      exchange(ben, ann);
      exchange(ben, ann);
      final done = Routines(ann).byId(s.id)?.doneAt(today);
      expect(done, isNotNull);
      expect(Routines(ben).byId(s.id)?.doneAt(today), done);
      // A Member records and swaps, but changes no Session.
      expect(() => Sessions(ben).setCount(s.id, 1), throwsA(isA<StateError>()));
      final own = Notes(ben).capture('Gym', place: area).id;
      expect(
        () => Sessions(ben)
            .spawn(own, _daily, pool: [a], count: 1, rule: SessionRule.rolling),
        throwsA(isA<StateError>()),
      );
    });

    test('a Member swaps for everyone, but writes nothing else of it', () {
      final [a, b, c] = tallies(ann, ['A', 'B', 'C'], place: area);
      final s = session(ann, [a, b, c], place: area);
      final today = day(ann, 0);
      expect(picked(ann, s, 0), [a, b]);
      exchange(ben, ann);
      Sessions(ben).another(s.id, today, a);
      // As a modified app would.
      final forged = [
        ben.append(s.id, SessionKeys.count, '3'),
        ben.append(s.id, SessionKeys.pool, a),
      ];
      exchange(ben, ann);
      final held = ann.allEntries().map((e) => e.id);
      expect(held, containsAll([for (final e in forged) e.id]));
      expect(shown(ann, s, 0), [c, b]);
      final read = Sessions(ann).byId(s.id)!;
      expect(read.count, 2);
      expect(read.pool, [a, b, c]);
    });

    test('only Tallies of its Area count while it is shared', () {
      final mine = tally(ann, 'Mine');
      final [a] = tallies(ann, ['A'], place: area);
      final note = Notes(ann).capture('Sport', place: area).id;
      expect(Sessions(ann).choices(note), [a]);
      expect(
        () => Sessions(ann).spawn(
          note,
          _daily,
          pool: [a, mine],
          count: 2,
          rule: SessionRule.rolling,
        ),
        throwsArgumentError,
      );
      // A Session of one's own picks from every Area.
      final private = session(ann, [mine, a]);
      expect(Sessions(ann).pool(private), [mine, a]);
      // Filed into the Area, it leaves the others out.
      Notes(ann).file(private.routine.noteId, area);
      expect(Sessions(ann).pool(private), [a]);
      expect(picked(ann, private, 0), [a]);
    });
  });
}
