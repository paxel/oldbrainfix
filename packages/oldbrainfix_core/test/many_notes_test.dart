import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// The Notes [Notes.all] lists, each made on its own by [Notes.byId] from
/// the entities whose current `$type` is a Note's, as before obf-1069.
List<Note> _oneByOne(Store store) {
  final notes = Notes(store);
  final listed = [
    for (final id in store.everOfType(NoteKeys.typeNote))
      if (store.current(id, NoteKeys.type) == NoteKeys.typeNote)
        if (notes.byId(id) case final note?
            when !note.isTrashed && !note.isMerged)
          note,
  ];
  listed.sort((a, b) => b.captured.compareTo(a.captured));
  return listed;
}

/// What a list shows of [note].
String _shown(Note note) => [
  note.id,
  note.text,
  note.place,
  note.mergedInto,
  note.trashed,
  note.captured,
  note.spawned,
  note.isUnsorted,
  note.context.location,
  note.context.namedPlace?.id,
  for (final box in note.boxes) '${box.id} ${box.state}',
].join(' | ');

void main() {
  late Store store;
  late Notes notes;
  late String home;
  late String gone;
  late String desk;
  late String orphan;

  setUp(() {
    var now = DateTime.utc(2026, 11, 22, 8);
    store = Store.inMemory(
      clock: () => now = now.add(const Duration(seconds: 1)),
    );
    addTearDown(store.close);
    notes = Notes(store);
    final places = Places(store);
    home = places.addArea('Home').id;
    gone = places.addArea('Gone').id;
    final work = places.addArea('Work').id;
    desk = places.addTopic(work, 'Desk').id;
    final old = places.addArea('Old').id;
    orphan = places.addTopic(old, 'Orphan').id;
    // An Area that is gone leaves its Notes and its Topics' Notes
    // Unsorted.
    store.append(gone, PlaceKeys.type, null);
    store.append(old, PlaceKeys.type, null);
    final centre = Coordinates(52.5, 13.4);
    NamedPlaces(store).add('Home', centre, radius: 500);
    NamedPlaces(store).add('Kitchen', centre, radius: 50);
    final boxed = notes.capture('☐ milk');
    Boxes(store).tick(boxed.id, boxed.boxes.single.id, BoxState.done);
  });

  /// Adds [rounds] times one Note of every kind a list tells apart.
  void seed(int rounds) {
    for (var i = 0; i < rounds; i++) {
      notes.capture('plain $i');
      notes.capture('home $i', place: home);
      notes.capture('desk $i', place: desk);
      store.append(notes.capture('gone $i').id, NoteKeys.place, gone);
      store.append(notes.capture('orphan $i').id, NoteKeys.place, orphan);
      final located = notes.capture('located $i').id;
      notes.locate(located, Coordinates(52.5, 13.4001));
      final far = notes.capture('far $i').id;
      notes.locate(far, Coordinates(48.1, 11.6));
      store.append(notes.capture('spawned $i').id, NoteKeys.spawned, 'x');
      notes.trash(notes.capture('trashed $i').id);
      final a = notes.capture('a $i').id;
      final b = notes.capture('b $i').id;
      final c = notes.capture('c $i').id;
      notes.merge(a, b);
      notes.merge(b, c);
      // Merged both ways on two Devices before they synced.
      final x = notes.capture('x $i').id;
      final y = notes.capture('y $i').id;
      store.append(x, NoteKeys.merged, y);
      store.append(y, NoteKeys.merged, x);
      // Merged into a Note this Device does not hold, and a Merge undone.
      store.append(notes.capture('z $i').id, NoteKeys.merged, 'note:away$i');
      final undone = notes.capture('undone $i').id;
      store.append(undone, NoteKeys.merged, c);
      store.append(undone, NoteKeys.merged, null);
    }
  }

  int readsOf(void Function() read) {
    final before = store.historyReads;
    read();
    return store.historyReads - before;
  }

  test('lists the same Notes as made one by one (obf-1069)', () {
    seed(3);
    final listed = notes.all();
    expect(listed.map(_shown), _oneByOne(store).map(_shown));
    // The smaller of the two Named Places holding it.
    for (final note in listed) {
      final name = note.context.namedPlace?.name;
      expect(name, note.text.startsWith('located') ? 'Kitchen' : isNull);
    }
    expect(listed.where((n) => n.place != null), isNotEmpty);
    expect(listed.where((n) => n.boxes.isNotEmpty), isNotEmpty);
  });

  test('reads Notes by their ids as made one by one (obf-1069)', () {
    seed(3);
    final ids = [
      ...store.everOfType(NoteKeys.typeNote),
      home,
      desk,
      'note:none',
    ];
    final oneByOne = {
      for (final id in ids)
        if (notes.byId(id) case final note?) id: _shown(note),
    };
    Map<String, String> shown(Map<String, Note> read) => {
      for (final MapEntry(key: id, value: note) in read.entries)
        id: _shown(note),
    };
    expect(oneByOne.keys, containsAll([for (final n in notes.all()) n.id]));
    expect(oneByOne.length, greaterThan(notes.all().length));
    store.changedElsewhere();
    // Read anew, with some of their fields read already, and from the
    // pass over every Note a list keeps.
    expect(shown(notes.byIds(ids)), oneByOne);
    store.changedElsewhere();
    expect(
      shown(notes.byIds(ids, read: store.currentFieldsOf(ids.take(9)))),
      oneByOne,
    );
    notes.all();
    expect(shown(notes.byIds(ids)), oneByOne);
  });

  test('counts the Unsorted Notes as it lists them (obf-1069)', () {
    seed(3);
    expect(notes.unsortedCount(), notes.unsorted().length);
    expect(notes.unsortedCount(), greaterThan(0));
  });

  test('reads no more for many Notes than for a few (obf-1069)', () {
    seed(1);
    final few = readsOf(notes.all);
    final fewCount = readsOf(notes.unsortedCount);
    seed(4);
    expect(readsOf(notes.all), few);
    expect(readsOf(notes.unsortedCount), fewCount);
  });
}
