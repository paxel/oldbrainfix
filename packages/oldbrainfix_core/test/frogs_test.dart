import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

/// The wall clock [days] days after the store's now: the tests' clock
/// moves by seconds, the Agenda by days.
WallClock onDay(int days) =>
    (utc) => utc.add(Duration(days: days));

void main() {
  late DateTime Function() clock;
  late Store ann;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann');
  });

  /// Frogs spawned from new Notes with [texts], in that order.
  List<Frog> spawned(Store store, List<String> texts, {String? place}) => [
    for (final text in texts)
      Frogs(store).spawn(Notes(store).capture(text, place: place).id),
  ];

  List<String> ids(Iterable<Frog> frogs) => [for (final f in frogs) f.id];

  group('the Frog', () {
    test('spawning takes the Note out of Unsorted, once per Note', () {
      final notes = Notes(ann);
      final frogs = Frogs(ann);
      final note = notes.capture('Wash the windows');
      expect(notes.unsorted().map((n) => n.id), [note.id]);
      final frog = frogs.spawn(note.id);
      expect(notes.unsorted(), isEmpty);
      expect(frog.noteId, note.id);
      expect(frogs.note(frog)?.text, 'Wash the windows');
      // The Note has its open Frog: spawning again writes nothing.
      final before = ann.allEntries().length;
      expect(frogs.spawn(note.id).id, frog.id);
      expect(ann.allEntries().length, before);
      // Never rings: no Routine, no alarm.
      expect(Routines(ann).all(), isEmpty);
      expect(Alarms(ann).plan((utc) => utc), isEmpty);
      // Eaten, the Note may spawn the next one.
      frogs.markDone(frog.id);
      expect(frogs.spawn(note.id).id, isNot(frog.id));
    });

    test('only a Note in the lists spawns one', () {
      final notes = Notes(ann);
      final frogs = Frogs(ann);
      final gone = notes.capture('Gone');
      notes.trash(gone.id);
      expect(() => frogs.spawn(gone.id), throwsArgumentError);
      final merged = notes.capture('Merged');
      final into = notes.capture('Into');
      notes.merge(merged.id, into.id);
      expect(() => frogs.spawn(merged.id), throwsArgumentError);
      expect(() => frogs.spawn('note:none'), throwsArgumentError);
    });

    test('Done and undone; open oldest first, Done newest first', () {
      final frogs = Frogs(ann);
      final [cupboard, windows, dvds] = spawned(ann, [
        'Clean out the cupboard',
        'Wash the windows',
        'Sort out the old DVDs',
      ]);
      expect(ids(frogs.open()), [cupboard.id, windows.id, dvds.id]);
      frogs.markDone(windows.id);
      final at = DateTime.utc(2026, 2, 1, 12);
      frogs.markDone(cupboard.id, at: at);
      expect(ids(frogs.open()), [dvds.id]);
      expect(ids(frogs.done()), [windows.id, cupboard.id]);
      expect(frogs.byId(cupboard.id)?.done, at);
      // Checked off again, it keeps when it was eaten.
      final before = ann.allEntries().length;
      frogs.markDone(cupboard.id);
      expect(ann.allEntries().length, before);
      frogs.unmarkDone(windows.id);
      expect(ids(frogs.open()), [windows.id, dvds.id]);
      expect(frogs.byId(windows.id)?.done, isNull);
    });

    test('the trash, its own or its Note\'s, takes it out of every list', () {
      final notes = Notes(ann);
      final frogs = Frogs(ann);
      final [a, b] = spawned(ann, ['A', 'B']);
      frogs.trash(a.id);
      expect(ids(frogs.open()), [b.id]);
      frogs.restore(a.id);
      expect(ids(frogs.open()), [a.id, b.id]);
      notes.trash(b.noteId);
      expect(ids(frogs.open()), [a.id]);
      // The Note stays out of Unsorted.
      notes.restore(b.noteId);
      expect(notes.unsorted(), isEmpty);
      expect(ids(frogs.open()), [a.id, b.id]);
      // Merged, it shows under the Note it went into.
      final into = notes.capture('Into');
      notes.merge(b.noteId, into.id);
      expect(frogs.note(b)?.id, into.id);
      expect(ids(frogs.ofNote(into.id)), [b.id]);
    });
  });

  group('on the Agenda', () {
    test('two a day, those offered the longest ago first', () {
      final frogs = Frogs(ann);
      final [a, b, c, d, e] = spawned(ann, ['A', 'B', 'C', 'D', 'E']);
      // Never offered: the oldest first.
      expect(ids(frogs.rotation()), [a.id, b.id, c.id, d.id, e.id]);
      final first = frogs.pickToday(onDay(0));
      expect(ids(first.shown), [a.id, b.id]);
      expect(first.visible, isTrue);
      expect(first.eaten, isFalse);
      expect(first.canSwap, isTrue);
      // Called again the same day, it writes nothing.
      final before = ann.allEntries().length;
      expect(ids(frogs.pickToday(onDay(0)).shown), [a.id, b.id]);
      expect(ann.allEntries().length, before);
      // Not eaten, they rotate on.
      expect(ids(frogs.rotation()), [c.id, d.id, e.id, a.id, b.id]);
      expect(ids(frogs.pickToday(onDay(1)).shown), [c.id, d.id]);
      expect(ids(frogs.pickToday(onDay(2)).shown), [e.id, a.id]);
      expect(ids(frogs.pickToday(onDay(3)).shown), [b.id, c.id]);
    });

    test('today shows what the pick will be, writing nothing', () {
      final frogs = Frogs(ann);
      final [a, b, _] = spawned(ann, ['A', 'B', 'C']);
      final before = ann.allEntries().length;
      expect(ids(frogs.today(onDay(0)).shown), [a.id, b.id]);
      expect(ann.allEntries().length, before);
    });

    test('"Another one" swaps in place, next in the rotation', () {
      final frogs = Frogs(ann);
      final [a, b, c] = spawned(ann, ['A', 'B', 'C']);
      frogs.pickToday(onDay(0));
      frogs.another(onDay(0), a.id);
      expect(ids(frogs.today(onDay(0)).shown), [c.id, b.id]);
      // The one swapped out was offered before the one swapped in.
      frogs.another(onDay(0), c.id);
      expect(ids(frogs.today(onDay(0)).shown), [a.id, b.id]);
      expect(() => frogs.another(onDay(0), c.id), throwsArgumentError);
      // Without one to swap in, nothing happens.
      frogs.trash(c.id);
      final today = frogs.today(onDay(0));
      expect(today.canSwap, isFalse);
      final before = ann.allEntries().length;
      frogs.another(onDay(0), a.id);
      expect(ann.allEntries().length, before);
    });

    test('checking one off hides the other, all day', () {
      final frogs = Frogs(ann);
      final [a, b, _] = spawned(ann, ['A', 'B', 'C']);
      frogs.pickToday(onDay(0));
      frogs.markDone(b.id);
      final eaten = frogs.pickToday(onDay(0));
      expect(ids(eaten.shown), [b.id]);
      expect(eaten.eaten, isTrue);
      expect(eaten.canSwap, isFalse);
      expect(() => frogs.another(onDay(0), a.id), throwsArgumentError);
      // Unchecked, both are back.
      frogs.unmarkDone(b.id);
      expect(ids(frogs.pickToday(onDay(0)).shown), [a.id, b.id]);
    });

    test('a Frog gone from the pick is replaced in its place', () {
      final frogs = Frogs(ann);
      final [a, b, c] = spawned(ann, ['A', 'B', 'C']);
      frogs.pickToday(onDay(0));
      frogs.trash(a.id);
      expect(ids(frogs.pickToday(onDay(0)).shown), [c.id, b.id]);
      // One new while only one is offered tops it up.
      frogs.trash(c.id);
      expect(ids(frogs.pickToday(onDay(0)).shown), [b.id]);
      final [d] = spawned(ann, ['D']);
      expect(ids(frogs.pickToday(onDay(0)).shown), [b.id, d.id]);
    });

    test('without open Frogs the section is gone, but for one eaten today', () {
      final frogs = Frogs(ann);
      final none = frogs.pickToday(onDay(0));
      expect(none.visible, isFalse);
      expect(none.shown, isEmpty);
      final [a] = spawned(ann, ['A']);
      expect(ids(frogs.pickToday(onDay(0)).shown), [a.id]);
      frogs.markDone(a.id);
      final eaten = frogs.pickToday(onDay(0));
      expect(eaten.visible, isTrue);
      expect(ids(eaten.shown), [a.id]);
      final tomorrow = frogs.pickToday(onDay(1));
      expect(tomorrow.visible, isFalse);
      expect(tomorrow.shown, isEmpty);
    });
  });

  group("between one's own Devices", () {
    test('today\'s pick and "Another one" travel', () {
      final desk = device(clock, 'Ann at the desk');
      exchange(desk, ann);
      final [a, b, c] = spawned(ann, ['A', 'B', 'C']);
      Frogs(ann).pickToday(onDay(0));
      exchange(desk, ann);
      expect(ids(Frogs(desk).today(onDay(0)).shown), [a.id, b.id]);
      // The desk picks nothing new.
      final before = desk.allEntries().length;
      Frogs(desk).pickToday(onDay(0));
      expect(desk.allEntries().length, before);
      Frogs(desk).another(onDay(0), b.id);
      exchange(desk, ann);
      expect(ids(Frogs(ann).today(onDay(0)).shown), [a.id, c.id]);
      // Their rotation goes on alike.
      expect(ids(Frogs(ann).rotation()), ids(Frogs(desk).rotation()));
      expect(
        ids(Frogs(ann).today(onDay(1)).shown),
        ids(Frogs(desk).today(onDay(1)).shown),
      );
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

    test('the Frog is shared like its Note; Done is shared', () {
      final [a, b] = spawned(ann, ['A', 'B'], place: area);
      exchange(ben, ann);
      expect(ids(Frogs(ben).open()), [a.id, b.id]);
      expect(ben.areaOf(a.id), area);
      // A Member checks off, but spawns nothing.
      Frogs(ben).markDone(a.id);
      final own = Notes(ben).capture('Bread', place: area);
      expect(() => Frogs(ben).spawn(own.id), throwsStateError);
      exchange(ben, ann);
      expect(Frogs(ann).byId(a.id)?.isDone, isTrue);
      Frogs(ann).unmarkDone(a.id);
      exchange(ben, ann);
      expect(Frogs(ben).byId(a.id)?.isDone, isFalse);
    });

    test('a Frog spawned before its Note was filed in goes along', () {
      final [a] = spawned(ann, ['A']);
      // Written in the Area since, so Ben's vector passes the Frog.
      Notes(ann).capture('Milk', place: area);
      exchange(ben, ann);
      expect(Frogs(ben).byId(a.id), isNull);
      Notes(ann).file(a.noteId, area);
      exchange(ben, ann);
      expect(ids(Frogs(ben).open()), [a.id]);
    });

    test('a Member writes nothing of the Frog but its check-off', () {
      final [a] = spawned(ann, ['A'], place: area);
      exchange(ben, ann);
      // As a modified app would.
      final forged = ben.append(
        a.id,
        FrogKeys.trashed,
        '2026-03-01T10:00:00.000Z',
      );
      exchange(ben, ann);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(Frogs(ann).byId(a.id)?.isTrashed, isFalse);
      expect(ids(Frogs(ann).open()), [a.id]);
    });

    test('each person keeps their own rotation', () {
      final [a, b, c] = spawned(ann, ['A', 'B', 'C'], place: area);
      Frogs(ann).pickToday(onDay(0));
      Frogs(ann).another(onDay(0), a.id);
      exchange(ben, ann);
      // Ann's picks never reach Ben: his are his own.
      expect(ben.loggedHistory(FrogKeys.pickEntity), isEmpty);
      expect(ids(Frogs(ben).pickToday(onDay(0)).shown), [a.id, b.id]);
      expect(ids(Frogs(ann).today(onDay(0)).shown), [c.id, b.id]);
      exchange(ben, ann);
      expect(ids(Frogs(ann).today(onDay(0)).shown), [c.id, b.id]);
    });

    test('a pick another person\'s Device wrote counts for nothing', () {
      final [a, b, c] = spawned(ann, ['A', 'B', 'C'], place: area);
      exchange(ben, ann);
      Frogs(ann).pickToday(onDay(0));
      // As a modified app would: Ben slips a pick into Ann's log.
      final day = Day.today(clock(), onDay(0));
      final forged = ben.append(
        FrogKeys.pickEntity,
        FrogKeys.pick,
        '$day,${c.id}',
      );
      ann.applyEntries([forged], verifiedDevice: ben.deviceId);
      expect(ann.allEntries().map((e) => e.id), contains(forged.id));
      expect(ids(Frogs(ann).today(onDay(0)).shown), [a.id, b.id]);
      expect(ids(Frogs(ann).rotation()), [c.id, a.id, b.id]);
    });
  });
}
