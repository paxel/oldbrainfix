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

  List<String> ids(List<Note> list) => [for (final n in list) n.id];

  group('taking a Note out of its place', () {
    test('sends it back to Unsorted with one entry', () {
      final home = Places(ann).addArea('Home').id;
      final keys = notes.capture('Spare keys at the neighbours');
      notes.file(keys.id, home);
      expect(notes.unsorted(), isEmpty);

      final mark = ann.currentSeq();
      expect(notes.mayUnfile(keys.id), isTrue);
      notes.unfile(keys.id);
      final written = ann.entriesAfter(mark);
      expect(written.map((e) => (e.entity, e.field, e.value)), [
        (keys.id, NoteKeys.place, null),
      ]);
      expect(notes.byId(keys.id)?.place, isNull);
      expect(ids(notes.unsorted()), [keys.id]);
      expect(notes.inside(home), isEmpty);
      // Nothing is left to take out.
      expect(notes.mayUnfile(keys.id), isFalse);
      expect(() => notes.unfile(keys.id), throwsArgumentError);
    });

    test('leaves a Note that spawned a Routine out of Unsorted', () {
      final home = Places(ann).addArea('Home').id;
      final bins = notes.capture('Bins out', place: home);
      Routines(ann).spawn(
        bins.id,
        Schedule(
          start: Day(2026, 3, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      final mark = ann.currentSeq();
      notes.unfile(bins.id);
      expect(ann.entriesAfter(mark).map((e) => e.field), [NoteKeys.place]);
      final back = notes.byId(bins.id);
      expect(back?.place, isNull);
      expect(back?.spawned, isTrue);
      expect(notes.unsorted(), isEmpty);
      expect(ids(notes.all()), [bins.id]);
    });

    test('is refused where nothing shows filed, and writes nothing', () {
      final places = Places(ann);
      final loose = notes.capture('never filed');
      final gone = places.addArea('Gone').id;
      final stranded = notes.capture('filed where nothing is', place: gone);
      // The place deleted elsewhere, its Notes not moved here yet.
      ann.append(gone, PlaceKeys.type, null);
      final mark = ann.currentSeq();
      for (final id in [loose.id, stranded.id, 'note:none', gone]) {
        expect(notes.mayUnfile(id), isFalse, reason: id);
        expect(() => notes.unfile(id), throwsArgumentError, reason: id);
      }
      expect(ann.entriesAfter(mark), isEmpty);
    });

    test('reaches the user\'s other Device, and a later filing wins', () {
      final desk = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(desk.close);
      final home = Places(ann).addArea('Home').id;
      final keys = notes.capture('Spare keys', place: home);
      syncAll(desk, ann);
      Notes(desk).unfile(keys.id);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        expect(ids(Notes(store).unsorted()), [keys.id]);
      }
      notes.file(keys.id, home);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        expect(Notes(store).byId(keys.id)?.place, home);
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

    test('a Member takes out their own Note, a Manager anyone\'s', () {
      final own = Notes(ben).capture('Ben\'s toner', place: area);
      final more = Notes(ben).capture('Ben\'s stapler', place: area);
      syncAll(ben, ann);
      expect(Notes(ben).mayUnfile(own.id), isTrue);
      Notes(ben).unfile(own.id);
      expect(ids(Notes(ben).unsorted()), [own.id]);
      expect(notes.mayUnfile(more.id), isTrue);
      notes.unfile(more.id);
      expect(ids(notes.unsorted()), [more.id]);
      // Out of the Area, neither leaves for the other person: each
      // stays filed there for them (ADR 0013).
      syncAll(ben, ann);
      expect(notes.byId(own.id)?.place, area);
      expect(Notes(ben).byId(more.id)?.place, area);
    });

    test('a Member\'s try on another\'s Note is refused and, written all '
        'the same, ignored everywhere', () {
      final theirs = notes.capture('Order paper', place: area);
      syncAll(ben, ann);
      final mark = ben.currentSeq();
      expect(Notes(ben).mayUnfile(theirs.id), isFalse);
      expect(() => Notes(ben).unfile(theirs.id), throwsStateError);
      expect(ben.entriesAfter(mark), isEmpty);
      // As a modified app would write it.
      ben.append(theirs.id, NoteKeys.place, null);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        expect(Notes(store).byId(theirs.id)?.place, area);
        expect(Notes(store).unsorted(), isEmpty);
      }
    });
  });
}
