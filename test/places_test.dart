import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  Store openStore({bool offered = true}) {
    final store = Store.inMemory()..starterSetOffered = offered;
    addTearDown(store.close);
    return store;
  }

  void speakGerman(WidgetTester tester) {
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
  }

  Future<void> openAreas(WidgetTester tester) async {
    await tester.tap(find.byIcon(Icons.search));
    await tester.pumpAndSettle();
  }

  Future<void> openAllNotes(WidgetTester tester) async {
    await openAreas(tester);
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
  }

  Future<void> rowMenu(WidgetTester tester, String row, String item) async {
    await tester.tap(
      find.descendant(
        of: find.ancestor(of: find.text(row), matching: find.byType(Row)).first,
        matching: find.byWidgetPredicate((w) => w is PopupMenuButton),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(item).last);
    await tester.pumpAndSettle();
  }

  Future<void> typeName(WidgetTester tester, String name, String button) async {
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      name,
    );
    await tester.tap(find.text(button).last);
    await tester.pumpAndSettle();
  }

  Color? buttonColour(WidgetTester tester, String label) {
    final button = tester.widget<TextButton>(
      find.ancestor(of: find.text(label), matching: find.byType(TextButton)),
    );
    return button.style?.foregroundColor?.resolve({});
  }

  Color errorColour(WidgetTester tester) =>
      Theme.of(tester.element(find.byType(AlertDialog))).colorScheme.error;

  group('Starter Set', () {
    testWidgets('first start offers it with everything ticked', (tester) async {
      final store = openStore(offered: false);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      expect(find.byType(StarterSetScreen), findsOneWidget);
      final boxes = tester.widgetList<CheckboxListTile>(
        find.byType(CheckboxListTile, skipOffstage: false),
      );
      expect(boxes.length, greaterThan(4));
      expect(boxes.every((box) => box.value == true), isTrue);
      for (final name in ['Household', 'Kitchen', 'Work', 'To sort out']) {
        expect(find.text(name), findsOneWidget);
      }
    });

    testWidgets('keeps what stays ticked and is not offered again', (
      tester,
    ) async {
      final store = openStore(offered: false);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Kitchen'));
      await tester.tap(find.text('Work'));
      await tester.pump();
      // A Topic stays unticked when its Area goes.
      expect(
        tester
            .widget<CheckboxListTile>(
              find.ancestor(
                of: find.text('Meetings'),
                matching: find.byType(CheckboxListTile),
              ),
            )
            .value,
        isFalse,
      );
      await tester.tap(find.text('Start'));
      await tester.pumpAndSettle();

      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(store.starterSetOffered, isTrue);
      final places = Places(store);
      expect(places.areas().map((a) => a.name), ['Car', 'Health', 'Household']);
      final home = places.areas().firstWhere((a) => a.name == 'Household');
      expect(places.topics(home.id).map((t) => t.name), ['Cleaning', 'Garden']);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      expect(find.byType(StarterSetScreen), findsNothing);
    });

    testWidgets('names show in German; renamed and own ones as typed', (
      tester,
    ) async {
      speakGerman(tester);
      final store = openStore(offered: false);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      expect(find.text('Haushalt'), findsOneWidget);
      expect(find.text('Zu klären'), findsOneWidget);
      await tester.tap(find.text("Los geht's"));
      await tester.pumpAndSettle();

      final places = Places(store);
      final car = places.areas().firstWhere((a) => a.starter == 'car');
      places.rename(car.id, 'Karre');
      places.addArea('Misc');
      await openAreas(tester);

      expect(find.text('Haushalt'), findsOneWidget);
      expect(find.text('Gesundheit'), findsOneWidget);
      expect(find.text('Karre'), findsOneWidget);
      expect(find.text('Auto'), findsNothing);
      expect(find.text('Misc'), findsOneWidget);
      expect(find.text('Household'), findsNothing);
    });
  });

  group('Areas and Topics', () {
    testWidgets('an Area is created, renamed and given Topics', (tester) async {
      final store = openStore();
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);
      expect(find.text('No Areas yet. Tap + to make one.'), findsOneWidget);

      await tester.tap(find.byIcon(Icons.add));
      await tester.pumpAndSettle();
      await typeName(tester, 'Hobby', 'Create');
      expect(find.text('Hobby'), findsOneWidget);

      await rowMenu(tester, 'Hobby', 'Rename');
      await typeName(tester, 'Hobbies', 'Rename');
      expect(find.text('Hobbies'), findsOneWidget);
      expect(Places(store).areas().single.name, 'Hobbies');

      await tester.tap(find.text('Hobbies'));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.add));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Topic'));
      await tester.pumpAndSettle();
      await typeName(tester, 'Model trains', 'Create');
      expect(find.text('Model trains'), findsOneWidget);
      final area = Places(store).areas().single;
      expect(Places(store).topics(area.id).single.name, 'Model trains');
    });

    testWidgets('an empty Topic deletes after one red confirm', (tester) async {
      final store = openStore();
      final places = Places(store);
      final home = places.addArea('Household');
      places.addTopic(home.id, 'Kitchen');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);
      await tester.tap(find.text('Household'));
      await tester.pumpAndSettle();

      await rowMenu(tester, 'Kitchen', 'Delete');
      expect(find.text('Delete the Topic “Kitchen”?'), findsOneWidget);
      expect(buttonColour(tester, 'Delete'), errorColour(tester));
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.text('Kitchen'), findsNothing);
      expect(places.topics(home.id), isEmpty);
    });

    testWidgets('an empty Area deletes with its Topics after one confirm', (
      tester,
    ) async {
      final store = openStore();
      final places = Places(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);

      await rowMenu(tester, 'Household', 'Delete');
      expect(find.text('Delete the Area “Household”?'), findsOneWidget);
      expect(find.text('Its Topics are deleted with it.'), findsOneWidget);
      expect(buttonColour(tester, 'Delete'), errorColour(tester));
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.text('Household'), findsNothing);
      expect(places.areas(), isEmpty);
      expect(places.topicById(kitchen.id), isNull);
    });

    testWidgets('a Topic with Notes asks where they go', (tester) async {
      final store = openStore();
      final places = Places(store);
      final notes = Notes(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final note = notes.capture('nees mor sauce');
      notes.file(note.id, kitchen.id);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);
      await tester.tap(find.text('Household'));
      await tester.pumpAndSettle();

      await rowMenu(tester, 'Kitchen', 'Delete');
      expect(find.textContaining('1 Note inside.'), findsOneWidget);
      expect(find.text('Move to Unsorted'), findsOneWidget);
      expect(buttonColour(tester, 'Move to “Household”'), errorColour(tester));
      await tester.tap(find.text('Move to “Household”'));
      await tester.pumpAndSettle();

      expect(places.topics(home.id), isEmpty);
      expect(notes.byId(note.id)?.place, home.id);
    });

    testWidgets('an Area with Notes sends them back to Unsorted', (
      tester,
    ) async {
      final store = openStore();
      final places = Places(store);
      final notes = Notes(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      notes.file(notes.capture('a').id, home.id);
      notes.file(notes.capture('b').id, kitchen.id);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);

      await rowMenu(tester, 'Household', 'Delete');
      expect(find.text('Delete the Area “Household”?'), findsOneWidget);
      expect(find.textContaining('2 Notes inside.'), findsOneWidget);
      expect(find.textContaining('Its Topics are deleted'), findsOneWidget);
      expect(find.textContaining('Where should they go?'), findsOneWidget);
      // Cancel, Unsorted, trash: an Area has no Area to move them into.
      final actions = find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextButton),
      );
      expect(actions, findsNWidgets(3));
      expect(buttonColour(tester, 'Move to trash'), errorColour(tester));
      await tester.tap(find.text('Move to Unsorted'));
      await tester.pumpAndSettle();

      expect(places.areas(), isEmpty);
      expect(notes.all().every((n) => n.isUnsorted), isTrue);
      await tester.tap(find.byTooltip('All Notes'));
      await tester.pumpAndSettle();
      expect(find.text('Unsorted'), findsNWidgets(2));
    });

    testWidgets('an Area with Notes can trash them with its Topics', (
      tester,
    ) async {
      final store = openStore();
      final places = Places(store);
      final notes = Notes(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      notes.capture('a', place: home.id);
      notes.capture('b', place: kitchen.id);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);

      await rowMenu(tester, 'Household', 'Delete');
      await tester.tap(find.text('Move to trash'));
      await tester.pumpAndSettle();

      expect(places.areas(), isEmpty);
      expect(notes.all(), isEmpty);
      expect(notes.trashed(), hasLength(2));
    });

    testWidgets('a Topic with Notes can trash them', (tester) async {
      final store = openStore();
      final places = Places(store);
      final notes = Notes(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final note = notes.capture('nees mor sauce', place: kitchen.id);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);
      await tester.tap(find.text('Household'));
      await tester.pumpAndSettle();

      await rowMenu(tester, 'Kitchen', 'Delete');
      expect(buttonColour(tester, 'Move to trash'), errorColour(tester));
      await tester.tap(find.text('Move to trash'));
      await tester.pumpAndSettle();

      expect(places.topics(home.id), isEmpty);
      expect(notes.byId(note.id)?.isTrashed, isTrue);
      expect(find.text('nees mor sauce'), findsNothing);
    });

    testWidgets('an Area gets a colour and an icon', (tester) async {
      final store = openStore();
      final places = Places(store);
      final car = places.addArea('Car');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAreas(tester);

      await rowMenu(tester, 'Car', 'Colour and icon');
      await tester.tap(find.byType(CircleAvatar).first);
      await tester.tap(find.byIcon(Icons.directions_car));
      await tester.pumpAndSettle();

      final looked = places.areaById(car.id);
      expect(looked?.colour, 'C62828');
      expect(looked?.icon, 'car');
    });
  });

  group('filing', () {
    testWidgets('a new Note shows as Unsorted, filing moves it out', (
      tester,
    ) async {
      final store = openStore();
      final places = Places(store);
      final home = places.addArea('Household');
      places.setColour(home.id, '2E7D32');
      places.setIcon(home.id, 'home');
      places.addTopic(home.id, 'Kitchen');
      final note = Notes(store).capture('nees mor sauce');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAllNotes(tester);
      expect(find.text('Unsorted'), findsOneWidget);

      await tester.tap(find.byTooltip('File into…'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Kitchen'));
      await tester.pumpAndSettle();

      expect(find.text('Unsorted'), findsNothing);
      expect(find.text('Household / Kitchen'), findsOneWidget);
      expect(Notes(store).byId(note.id)?.isUnsorted, isFalse);
      // The Area's colour and icon mark the card.
      final card = find.byType(NoteCard);
      final icon = tester.widget<Icon>(
        find.descendant(of: card, matching: find.byIcon(Icons.home)),
      );
      expect(icon.color, const Color(0xFF2E7D32));
      final edge = tester
          .widgetList<Container>(
            find.descendant(of: card, matching: find.byType(Container)),
          )
          .map((c) => c.decoration)
          .whereType<BoxDecoration>()
          .map((d) => d.border)
          .whereType<Border>()
          .single;
      expect(edge.left.color, const Color(0xFF2E7D32));
    });

    testWidgets('a Note can be filed straight into an Area', (tester) async {
      final store = openStore();
      final places = Places(store);
      final home = places.addArea('Household');
      places.addTopic(home.id, 'Kitchen');
      final note = Notes(store).capture('nees mor sauce');
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await openAllNotes(tester);

      await tester.tap(find.byTooltip('File into…'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Household'));
      await tester.pumpAndSettle();

      expect(find.text('Household'), findsOneWidget);
      expect(Notes(store).byId(note.id)?.place, home.id);
    });
  });
}
