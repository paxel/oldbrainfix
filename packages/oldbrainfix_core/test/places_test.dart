import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now);
    places = Places(store);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  /// Moves the clock on so a later write is clearly the latest.
  void tick() => now = now.add(const Duration(minutes: 1));

  group('Areas and Topics', () {
    test('an Area is created, renamed and listed by name', () {
      final work = places.addArea('Work');
      places.addArea('Household');
      tick();
      places.rename(work.id, 'Job');
      expect(places.areas().map((a) => a.name), ['Household', 'Job']);
      expect(work.id, startsWith('area:'));
    });

    test('a Topic lives inside its Area', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      places.addTopic(home.id, 'Garden');
      expect(kitchen.id, startsWith('topic:'));
      expect(kitchen.areaId, home.id);
      expect(places.topics(home.id).map((t) => t.name), ['Garden', 'Kitchen']);
      tick();
      places.rename(kitchen.id, 'Cooking');
      expect(places.topicById(kitchen.id)?.name, 'Cooking');
    });

    test('there are exactly two levels: no Topic inside a Topic', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      expect(() => places.addTopic(kitchen.id, 'Fridge'), throwsArgumentError);
      expect(() => places.addTopic('area:none', 'X'), throwsArgumentError);
    });

    test('names are trimmed and never empty', () {
      final home = places.addArea('  Household ');
      expect(home.name, 'Household');
      expect(() => places.addArea('   '), throwsArgumentError);
      expect(() => places.rename(home.id, ''), throwsArgumentError);
    });

    test('an Area carries an optional colour and icon', () {
      final car = places.addArea('Car');
      expect(car.colour, isNull);
      expect(car.icon, isNull);
      tick();
      places.setColour(car.id, 'C62828');
      places.setIcon(car.id, 'car');
      final shown = places.areaById(car.id);
      expect(shown?.colour, 'C62828');
      expect(shown?.icon, 'car');
      tick();
      places.setColour(car.id, null);
      places.setIcon(car.id, null);
      expect(places.areaById(car.id)?.colour, isNull);
      expect(places.areaById(car.id)?.icon, isNull);
    });

    test('an unknown icon key is refused', () {
      final car = places.addArea('Car');
      expect(() => places.setIcon(car.id, 'rocket-ship'), throwsArgumentError);
    });
  });

  group('filing and Unsorted', () {
    test('a new Note is Unsorted', () {
      final note = notes.capture('nees mor sauce');
      expect(note.isUnsorted, isTrue);
      expect(note.place, isNull);
    });

    test('filing a Note into an Area removes it from Unsorted', () {
      final home = places.addArea('Household');
      final note = notes.capture('nees mor sauce');
      tick();
      notes.file(note.id, home.id);
      final filed = notes.byId(note.id);
      expect(filed?.isUnsorted, isFalse);
      expect(filed?.place, home.id);
    });

    test('a Note sits in exactly one place: filing again moves it', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final work = places.addArea('Work');
      final note = notes.capture('nees mor sauce');
      tick();
      notes.file(note.id, kitchen.id);
      expect(notes.byId(note.id)?.place, kitchen.id);
      tick();
      notes.file(note.id, work.id);
      expect(notes.byId(note.id)?.place, work.id);
      expect(places.notesInside(home.id), 0);
      expect(places.notesInside(work.id), 1);
    });

    test('a Note cannot be filed into something that is not a place', () {
      final note = notes.capture('x');
      expect(() => notes.file(note.id, 'area:none'), throwsArgumentError);
      expect(() => notes.file(note.id, note.id), throwsArgumentError);
    });

    test('a place that disappeared leaves its Note Unsorted', () {
      final home = places.addArea('Household');
      final note = notes.capture('x');
      tick();
      notes.file(note.id, home.id);
      tick();
      // As if a deletion arrived without moving this Note.
      store.append(home.id, r'$type', null);
      expect(notes.byId(note.id)?.isUnsorted, isTrue);
    });

    test('an Area counts the Notes in it and in its Topics', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      tick();
      notes.file(notes.capture('a').id, home.id);
      notes.file(notes.capture('b').id, kitchen.id);
      notes.capture('c');
      expect(places.notesInside(home.id), 2);
      expect(places.notesInside(kitchen.id), 1);
    });
  });

  group('deleting', () {
    test('an empty Topic goes', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      tick();
      places.deleteTopic(kitchen.id, NotesGo.unsorted);
      expect(places.topics(home.id), isEmpty);
      expect(places.topicById(kitchen.id), isNull);
    });

    test('deleting a Topic can move its Notes to Unsorted', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final note = notes.capture('sauce');
      tick();
      notes.file(note.id, kitchen.id);
      tick();
      places.deleteTopic(kitchen.id, NotesGo.unsorted);
      final moved = notes.byId(note.id);
      expect(moved?.isUnsorted, isTrue);
      expect(store.current(note.id, NoteKeys.place), isNull);
    });

    test('deleting a Topic can move its Notes to its Area', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final note = notes.capture('sauce');
      tick();
      notes.file(note.id, kitchen.id);
      tick();
      places.deleteTopic(kitchen.id, NotesGo.area);
      expect(notes.byId(note.id)?.place, home.id);
    });

    test('deleting an Area takes its Topics, their Notes go Unsorted', () {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final work = places.addArea('Work');
      final inArea = notes.capture('a');
      final inTopic = notes.capture('b');
      final elsewhere = notes.capture('c');
      tick();
      notes.file(inArea.id, home.id);
      notes.file(inTopic.id, kitchen.id);
      notes.file(elsewhere.id, work.id);
      tick();
      places.deleteArea(home.id);
      expect(places.areas().map((a) => a.id), [work.id]);
      expect(places.topicById(kitchen.id), isNull);
      expect(notes.byId(inArea.id)?.isUnsorted, isTrue);
      expect(notes.byId(inTopic.id)?.isUnsorted, isTrue);
      expect(notes.byId(elsewhere.id)?.place, work.id);
    });

    test('a Note moved out by a deletion can be filed again', () {
      final home = places.addArea('Household');
      final work = places.addArea('Work');
      final note = notes.capture('a');
      tick();
      notes.file(note.id, home.id);
      tick();
      places.deleteArea(home.id);
      tick();
      notes.file(note.id, work.id);
      expect(notes.byId(note.id)?.place, work.id);
    });
  });

  group('Starter Set', () {
    test('holds the agreed Areas and Topics', () {
      expect(
        {
          for (final area in starterSet)
            area.name: [for (final topic in area.topics) topic.name],
        },
        {
          'Household': ['Kitchen', 'Garden', 'Cleaning'],
          'Work': ['Meetings', 'To sort out'],
          'Car': ['Service', 'Repairs'],
          'Health': ['Doctor', 'Medication'],
        },
      );
    });

    test('plants the picked Areas and Topics with their starter slugs', () {
      places.plantStarterSet({'household', 'kitchen', 'garden', 'car'});
      final areas = places.areas();
      expect(areas.map((a) => a.name), ['Car', 'Household']);
      final home = areas.firstWhere((a) => a.name == 'Household');
      expect(home.starter, 'household');
      expect(places.topics(home.id).map((t) => t.starter), [
        'garden',
        'kitchen',
      ]);
      final car = areas.firstWhere((a) => a.name == 'Car');
      expect(places.topics(car.id), isEmpty);
    });

    test('a Topic is not planted without its Area', () {
      places.plantStarterSet({'kitchen', 'doctor'});
      expect(places.areas(), isEmpty);
    });

    test('planted starters are editable like any other', () {
      places.plantStarterSet({'work', 'meetings'});
      final work = places.areas().single;
      final meetings = places.topics(work.id).single;
      tick();
      places.rename(work.id, 'Büro');
      places.setIcon(work.id, 'work');
      places.deleteTopic(meetings.id, NotesGo.unsorted);
      final edited = places.areaById(work.id);
      expect(edited?.name, 'Büro');
      expect(edited?.starter, 'work');
      expect(edited?.icon, 'work');
      expect(places.topics(work.id), isEmpty);
    });

    test('user-made places have no starter slug', () {
      final mine = places.addArea('Misc');
      expect(mine.starter, isNull);
      expect(places.addTopic(mine.id, 'Stuff').starter, isNull);
    });
  });
}
