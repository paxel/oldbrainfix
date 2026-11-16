import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'remove_media_test.dart' show syncAll;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
  });
  tearDown(() => ann.close());

  final daily = Schedule(
    start: Day(2026, 3, 1),
    times: [ClockTime(8, 0)],
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  List<String> ids(List<Note> list) => [for (final n in list) n.id];

  group('undoing a Merge', () {
    test('brings the Note back where it was, with all it holds', () {
      final home = Places(ann).addArea('Home').id;
      final keys = notes.capture('Spare keys at the neighbours', place: home);
      final amend = notes.amend(keys.id, 'Back by Friday');
      final routine = Routines(ann).spawn(keys.id, daily);
      final frog = Frogs(ann).spawn(keys.id);
      final loose = notes.capture('Ask about the plants');
      final into = notes.capture('Neighbours on holiday');
      notes.merge(keys.id, into.id);
      notes.merge(loose.id, into.id);
      expect(ids(notes.all()), [into.id]);
      expect(Routines(ann).ofNote(into.id).map((r) => r.id), [routine.id]);
      expect(Frogs(ann).ofNote(into.id).map((f) => f.id), [frog.id]);

      final mark = ann.currentSeq();
      expect(notes.mayUndoMerge(keys.id), isTrue);
      notes.undoMerge(keys.id);
      notes.undoMerge(loose.id);
      final written = ann.entriesAfter(mark);
      expect(written.map((e) => (e.entity, e.field, e.value)), [
        (keys.id, NoteKeys.merged, null),
        (loose.id, NoteKeys.merged, null),
      ]);
      expect(ids(notes.all()), [into.id, loose.id, keys.id]);
      expect(ids(notes.inside(home)), [keys.id]);
      expect(ids(notes.unsorted()), [loose.id, into.id]);
      expect(notes.byId(keys.id)?.mergedInto, isNull);
      expect(notes.byId(keys.id)?.place, home);
      expect(notes.sections(into.id), isEmpty);
      expect(notes.sections(keys.id).map((s) => s.id), [amend.id]);
      expect(Routines(ann).ofNote(keys.id).map((r) => r.id), [routine.id]);
      expect(Routines(ann).ofNote(into.id), isEmpty);
      expect(Frogs(ann).ofNote(keys.id).map((f) => f.id), [frog.id]);
      expect(Frogs(ann).ofNote(into.id), isEmpty);
      // Nothing is left to undo.
      expect(notes.mayUndoMerge(keys.id), isFalse);
      expect(() => notes.undoMerge(keys.id), throwsArgumentError);
    });

    test('is refused where nothing shows merged, and writes nothing', () {
      final alone = notes.capture('alone');
      final stranger = notes.capture('merged into a stranger');
      ann.append(stranger.id, NoteKeys.merged, 'note:never-synced');
      final mark = ann.currentSeq();
      for (final id in [alone.id, stranger.id, 'note:none', 'section:x']) {
        expect(notes.mayUndoMerge(id), isFalse, reason: id);
        expect(() => notes.undoMerge(id), throwsArgumentError, reason: id);
      }
      expect(ann.entriesAfter(mark), isEmpty);
    });

    test('of a chain, takes out only the one Note and what is in it', () {
      final a = notes.capture('a');
      final b = notes.capture('b');
      final c = notes.capture('c');
      notes.merge(a.id, b.id);
      notes.merge(b.id, c.id);
      notes.undoMerge(b.id);
      expect(ids(notes.all()), [c.id, b.id]);
      expect(notes.byId(a.id)?.mergedInto, b.id);
      expect(notes.sections(b.id).map((s) => s.id), [a.id]);
      expect(notes.sections(c.id), isEmpty);
    });

    test('of two Notes merged into each other leaves the other Merge', () {
      final x = notes.capture('x');
      final y = notes.capture('y');
      // Both merges arrive, as after a sync of two Devices.
      ann.append(x.id, NoteKeys.merged, y.id);
      ann.append(y.id, NoteKeys.merged, x.id);
      final (low, high) = x.id.compareTo(y.id) < 0 ? (x, y) : (y, x);
      expect(notes.mayUndoMerge(low.id), isFalse);
      notes.undoMerge(high.id);
      expect(ids(notes.all()), [high.id]);
      expect(notes.byId(low.id)?.mergedInto, high.id);
    });

    test('reaches the user\'s other Device, and a later Merge wins', () {
      final desk = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(desk.close);
      final a = notes.capture('a');
      final b = notes.capture('b');
      notes.merge(a.id, b.id);
      syncAll(desk, ann);
      Notes(desk).undoMerge(a.id);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        expect(Notes(store).byId(a.id)?.mergedInto, isNull);
      }
      notes.merge(a.id, b.id);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        expect(Notes(store).byId(a.id)?.mergedInto, b.id);
      }
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
      area = ann.share(Places(ann).addArea('Office').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    test(
      'a Member undoes the Merge of their own Note, a Manager anyone\'s',
      () {
        final into = notes.capture('Toner', place: area);
        final own = Notes(ben).capture('Ben\'s toner', place: area);
        syncAll(ben, ann);
        Notes(ben).merge(own.id, into.id);
        expect(Notes(ben).mayUndoMerge(own.id), isTrue);
        Notes(ben).undoMerge(own.id);
        syncAll(ben, ann);
        for (final store in [ann, ben]) {
          expect(Notes(store).byId(own.id)?.mergedInto, isNull);
        }
        Notes(ben).merge(own.id, into.id);
        syncAll(ben, ann);
        expect(notes.mayUndoMerge(own.id), isTrue);
        notes.undoMerge(own.id);
        syncAll(ben, ann);
        for (final store in [ann, ben]) {
          expect(Notes(store).byId(own.id)?.mergedInto, isNull);
        }
      },
    );

    test('a Member\'s undo on another\'s Note is refused and, written all '
        'the same, ignored everywhere', () {
      final into = notes.capture('Toner', place: area);
      final merged = notes.capture('Order more', place: area);
      notes.merge(merged.id, into.id);
      syncAll(ben, ann);
      final mark = ben.currentSeq();
      expect(Notes(ben).mayUndoMerge(merged.id), isFalse);
      expect(() => Notes(ben).undoMerge(merged.id), throwsStateError);
      expect(ben.entriesAfter(mark), isEmpty);
      // As a modified app would write it.
      ben.append(merged.id, NoteKeys.merged, null);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        expect(Notes(store).byId(merged.id)?.mergedInto, into.id);
      }
    });
  });
}
