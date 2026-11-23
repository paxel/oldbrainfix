import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A clock that moves one second per read, as the fixtures' does: a
/// read more or less moves every later date.
DateTime Function() ticking([DateTime? from]) {
  var now = from ?? DateTime.utc(2026, 10, 24, 7);
  return () => now = now.add(const Duration(seconds: 1));
}

Schedule daily(int hour) => Schedule(
  start: Day(2026, 10, 24),
  times: [ClockTime(hour, 0)],
  rule: Recurrence.parse('FREQ=DAILY'),
);

/// Deleting the Area [areaId] as it was done before obf-1093: one batch,
/// [Routines.trashNote] for each Note, which read every Routine each time.
void deleteAreaAsBefore(Store store, String areaId, NotesGo notesGo) {
  final places = Places(store);
  final routines = Routines(store);
  store.batch(() {
    for (final note in Notes(store).inside(areaId)) {
      switch (notesGo) {
        case NotesGo.unsorted:
          store.append(note.id, NoteKeys.place, null);
          if (note.spawned) store.append(note.id, NoteKeys.spawned, null);
        case NotesGo.area:
          throw StateError('an Area has no Area');
        case NotesGo.trash:
          routines.trashNote(note.id);
      }
    }
    for (final topic in places.topics(areaId)) {
      store.append(topic.id, PlaceKeys.type, null);
    }
    store.append(areaId, PlaceKeys.type, null);
  });
}

/// Every entry of [store] with all it holds but its local order.
List<String> log(Store store) => [
  for (final e in store.allEntries())
    [
      e.device,
      e.dseq,
      e.entity,
      e.field,
      e.value,
      e.date.toIso8601String(),
      e.author,
      e.recorded.toIso8601String(),
      e.sig,
      e.ctx,
    ].join('|'),
];

/// An Area "Home" with a Topic, Notes in both, Routines on some of them,
/// one on a Note merged into one of them, one in the trash on its own,
/// and a Note and a Routine elsewhere; returns the Area's id.
String plant(Store store) {
  final places = Places(store);
  final notes = Notes(store);
  final routines = Routines(store);
  final home = places.addArea('Home');
  final kitchen = places.addTopic(home.id, 'Kitchen');
  final work = places.addArea('Work');
  final pills = notes.capture('pills', place: home.id);
  routines.spawn(pills.id, daily(8));
  routines.spawn(pills.id, daily(20));
  final sauce = notes.capture('sauce', place: kitchen.id);
  routines.spawn(sauce.id, daily(18));
  final vitamins = notes.capture('vitamins', place: home.id);
  routines.spawn(vitamins.id, daily(9));
  notes.merge(vitamins.id, pills.id);
  final plants = notes.capture('plants', place: home.id);
  routines.trash(routines.spawn(plants.id, daily(10)).id);
  notes.capture('no routine', place: kitchen.id);
  final report = notes.capture('report', place: work.id);
  routines.spawn(report.id, daily(11));
  return home.id;
}

void main() {
  late Directory dir;

  setUp(() => dir = Directory.systemTemp.createTempSync('area_trash_test_'));
  tearDown(() => dir.deleteSync(recursive: true));

  /// The log after deleting the planted Area the old way and the new
  /// way, each on a copy of the same database.
  (List<String>, List<String>) both(NotesGo notesGo) {
    final first = Store.open('${dir.path}/a.db', clock: ticking());
    final areaId = plant(first);
    first.close();
    File('${dir.path}/a.db').copySync('${dir.path}/b.db');
    final later = DateTime.utc(2026, 10, 25, 7);
    final before = Store.open('${dir.path}/a.db', clock: ticking(later));
    final now = Store.open('${dir.path}/b.db', clock: ticking(later));
    addTearDown(before.close);
    addTearDown(now.close);
    deleteAreaAsBefore(before, areaId, notesGo);
    Places(now).deleteArea(areaId, notesGo);
    return (log(before), log(now));
  }

  test('an Area trashed in one pass writes the log trashing each Note '
      'wrote (obf-1093)', () {
    final (before, now) = both(NotesGo.trash);
    expect(before.length, greaterThan(20));
    expect(now, before);
  });

  test('an Area whose Notes go back to Unsorted writes the log it wrote '
      'before', () {
    final (before, now) = both(NotesGo.unsorted);
    expect(now, before);
  });

  group('trashing an Area', () {
    late Store store;
    setUp(() => store = Store.inMemory(clock: ticking()));
    tearDown(() => store.close());

    test('takes every Routine of its Notes along, those merged in too, '
        'and leaves the others', () {
      final areaId = plant(store);
      final routines = Routines(store);
      Places(store).deleteArea(areaId, NotesGo.trash);
      expect(routines.all().map((r) => routines.note(r)?.text), ['report']);
      final pills = Notes(store)
          .trashed()
          .singleWhere((n) => n.text.startsWith('pills'));
      routines.restoreNote(pills.id);
      expect(routines.ofNote(pills.id), hasLength(3));
    });

    test('tells how many of its Notes are in the trash as it goes', () {
      final areaId = plant(store);
      final told = <(int, int)>[];
      Places(store).deleteArea(
        areaId,
        NotesGo.trash,
        progress: (n, of) => told.add((n, of)),
      );
      expect(told, [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)]);
    });

    test('tells how many went back to Unsorted as it goes', () {
      final areaId = plant(store);
      final told = <(int, int)>[];
      Places(store).deleteArea(
        areaId,
        NotesGo.unsorted,
        progress: (n, of) => told.add((n, of)),
      );
      expect(told.last, (4, 4));
      expect(told, hasLength(5));
    });

    /// The reads of deleting an Area of [count] Notes into the trash,
    /// beside the same Routines.
    int readsWith(int count) {
      final store = Store.inMemory(clock: ticking());
      addTearDown(store.close);
      final notes = Notes(store);
      final routines = Routines(store);
      final places = Places(store);
      final home = places.addArea('Home');
      final work = places.addArea('Work');
      for (var i = 0; i < 3; i++) {
        routines.spawn(
          notes.capture('elsewhere $i', place: work.id).id,
          daily(8),
        );
      }
      routines.spawn(notes.capture('pills', place: home.id).id, daily(9));
      for (var i = 1; i < count; i++) {
        notes.capture('note $i', place: home.id);
      }
      // What a first read of anything reads once.
      places.notesInside(home.id);
      final from = store.historyReads + store.loggedReads;
      places.deleteArea(home.id, NotesGo.trash);
      expect(notes.inside(home.id), isEmpty);
      return store.historyReads + store.loggedReads - from;
    }

    test('reads as much for many Notes as for a few', () {
      expect(readsWith(30), readsWith(3));
    });
  });

  group('the Notes inside a place, counted', () {
    late Store store;
    setUp(() => store = Store.inMemory(clock: ticking()));
    tearDown(() => store.close());

    test('are those listed inside it, without making a Note', () {
      final places = Places(store);
      final notes = Notes(store);
      final home = places.addArea('Home');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final work = places.addArea('Work');
      notes.capture('in the Area', place: home.id);
      notes.capture('in its Topic', place: kitchen.id);
      notes.capture('elsewhere', place: work.id);
      notes.trash(notes.capture('trashed', place: home.id).id);
      final kept = notes.capture('kept', place: home.id);
      notes.merge(notes.capture('merged', place: kitchen.id).id, kept.id);
      final gone = notes.capture('gone Topic');
      store.append(gone.id, NoteKeys.place, 'topic:gone');
      for (final id in [home.id, kitchen.id, work.id, 'topic:gone']) {
        expect(places.notesInside(id), notes.inside(id).length, reason: id);
      }
      expect(places.notesInside(home.id), 3);
      expect(places.notesInside(kitchen.id), 1);
    });
  });
}
