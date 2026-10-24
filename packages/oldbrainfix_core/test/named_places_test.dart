import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  late Store store;
  late NamedPlaces named;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  // Marienplatz, Munich, and spots around it.
  final home = Coordinates(48.137154, 11.576124);
  // About 55 m north of home.
  final nextDoor = Coordinates(48.137654, 11.576124);
  // About 1.1 km north of home.
  final faraway = Coordinates(48.147154, 11.576124);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now);
    named = NamedPlaces(store);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  group('coordinates', () {
    test('survive being stored', () {
      expect(Coordinates.decode(home.encode()), home);
    });

    test('anything else reads as unknown', () {
      for (final bad in [null, '', 'x', '1', '1,2,3', 'a,b', '91,0', '0,181']) {
        expect(Coordinates.decode(bad), isNull, reason: '$bad');
      }
    });

    test('outside the Earth are refused', () {
      expect(() => Coordinates(90.5, 0), throwsArgumentError);
      expect(() => Coordinates(0, -180.5), throwsArgumentError);
      expect(() => Coordinates(double.nan, 0), throwsArgumentError);
    });

    test('are measured apart in metres', () {
      expect(distanceInMetres(home, home), 0);
      expect(distanceInMetres(home, nextDoor), closeTo(55.6, 1));
      expect(distanceInMetres(home, faraway), closeTo(1112, 5));
      // Munich to Berlin, about 504 km.
      expect(
        distanceInMetres(home, Coordinates(52.520008, 13.404954)),
        closeTo(504000, 2000),
      );
    });
  });

  group('Named Places', () {
    test('are made with a name and a radius, and listed by name', () {
      named.add(' Work ', faraway, radius: 200);
      final made = named.add('home', home, radius: 100);
      expect(made.id, startsWith('namedplace:'));
      expect(made.centre, home);
      expect(made.radius, 100);
      expect(named.all().map((p) => p.name), ['home', 'Work']);
    });

    test('need a name and a radius above zero', () {
      expect(() => named.add('  ', home, radius: 100), throwsArgumentError);
      expect(() => named.add('Home', home, radius: 0), throwsArgumentError);
      expect(() => named.add('Home', home, radius: -5), throwsArgumentError);
      expect(named.all(), isEmpty);
    });

    test('are renamed, given another radius and deleted', () {
      final place = named.add('Home', home, radius: 100);
      named.rename(place.id, "Mum's");
      named.setRadius(place.id, 250);
      expect(named.byId(place.id)?.name, "Mum's");
      expect(named.byId(place.id)?.radius, 250);
      named.delete(place.id);
      expect(named.byId(place.id), isNull);
      expect(named.all(), isEmpty);
      expect(() => named.rename(place.id, 'x'), throwsArgumentError);
    });

    test('a point inside the radius is at the place, outside it is not', () {
      final place = named.add('Home', home, radius: 100);
      expect(named.at(home)?.id, place.id);
      expect(named.at(nextDoor)?.id, place.id);
      expect(named.at(faraway), isNull);
      named.setRadius(place.id, 50);
      expect(named.at(nextDoor), isNull);
    });

    test('where they overlap, the smallest one wins', () {
      named.add('Town', home, radius: 5000);
      named.add('Garden', nextDoor, radius: 60);
      named.add('Home', home, radius: 100);
      expect(named.at(nextDoor)?.name, 'Garden');
      expect(named.at(home)?.name, 'Garden');
      expect(named.at(faraway)?.name, 'Town');
    });

    test('of equal size, the nearest centre wins', () {
      named.add('Home', home, radius: 100);
      named.add('Next door', nextDoor, radius: 100);
      expect(namedPlaceAt(named.all(), home)?.name, 'Home');
      expect(namedPlaceAt(named.all(), nextDoor)?.name, 'Next door');
    });
  });

  group('Capture Context', () {
    test('a Note has no location until one is attached', () {
      final note = notes.capture('nees mor sauce');
      expect(note.context.location, isNull);
      expect(note.context.namedPlace, isNull);
      now = now.add(const Duration(seconds: 20));
      notes.locate(note.id, faraway);
      final located = notes.byId(note.id)?.context;
      expect(located?.location, faraway);
      expect(located?.namedPlace, isNull);
      expect(located?.captured, DateTime.utc(2026, 10, 24, 8));
    });

    test('a Note near a Named Place carries its name', () {
      named.add('Home', home, radius: 100);
      final note = notes.capture('nees mor sauce');
      notes.locate(note.id, nextDoor);
      expect(notes.byId(note.id)?.context.namedPlace?.name, 'Home');
    });

    test('naming follows the Named Places as they change', () {
      final note = notes.capture('nees mor sauce');
      notes.locate(note.id, nextDoor);
      String? shown() => notes.byId(note.id)?.context.namedPlace?.name;
      expect(shown(), isNull);
      final place = named.add('Home', home, radius: 100);
      expect(shown(), 'Home', reason: 'a new Named Place names old Notes');
      named.rename(place.id, "Mum's");
      expect(shown(), "Mum's");
      named.delete(place.id);
      expect(shown(), isNull);
      expect(notes.byId(note.id)?.context.location, nextDoor);
    });

    test('an Amend carries its own location', () {
      named.add('Home', home, radius: 100);
      final note = notes.capture('nees mor sauce');
      notes.locate(note.id, faraway);
      now = now.add(const Duration(hours: 1));
      final section = notes.amend(note.id, 'Update: sauce bought');
      notes.locate(section.id, home);
      expect(notes.byId(note.id)?.context.namedPlace, isNull);
      expect(notes.sections(note.id).single.context.namedPlace?.name, 'Home');
    });

    test('a merged Note keeps its location as a Section', () {
      named.add('Home', home, radius: 100);
      final target = notes.capture('shopping');
      now = now.add(const Duration(minutes: 1));
      final merged = notes.capture('nees mor sauce');
      notes.locate(merged.id, home);
      notes.merge(merged.id, target.id);
      final section = notes.sections(target.id).single;
      expect(section.context.location, home);
      expect(section.context.namedPlace?.name, 'Home');
    });

    test('only Notes and Sections can be located', () {
      final place = named.add('Home', home, radius: 100);
      expect(() => notes.locate(place.id, home), throwsArgumentError);
      expect(() => notes.locate('note:gone', home), throwsArgumentError);
    });
  });
}
