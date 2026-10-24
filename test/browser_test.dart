import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/notes_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  late Area home;
  late Topic kitchen;
  late Area work;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
    home = places.addArea('Household');
    kitchen = places.addTopic(home.id, 'Kitchen');
    places.addTopic(home.id, 'Garden');
    work = places.addArea('Work');
  });
  tearDown(() => store.close());

  /// Captures [text] a minute after the last one, filed into [place].
  void capture(String text, [String? place]) {
    now = now.add(const Duration(minutes: 1));
    notes.capture(text, place: place);
  }

  Future<void> openBrowser(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
  }

  Future<void> open(WidgetTester tester, String row) async {
    await tester.tap(find.text(row));
    await tester.pumpAndSettle();
  }

  Future<void> search(WidgetTester tester, String query) async {
    await tester.enterText(find.byType(TextField), query);
    await tester.pumpAndSettle();
  }

  Future<void> swipe(WidgetTester tester, double dx) async {
    await tester.fling(find.byType(BrowserSwipes), Offset(dx, 0), 1000);
    await tester.pumpAndSettle();
  }

  List<String> cardTexts(WidgetTester tester) => [
    for (final card in tester.widgetList<NoteCard>(find.byType(NoteCard)))
      card.note.text,
  ];

  /// Whether the row [title] is ringed as the last selected one.
  bool highlighted(WidgetTester tester, String title) => tester
      .widgetList<DecoratedBox>(
        find.ancestor(
          of: find.text(title),
          matching: find.byType(DecoratedBox),
        ),
      )
      .map((box) => box.decoration)
      .whereType<BoxDecoration>()
      .any((d) => d.border != null);

  group('search', () {
    testWidgets('finds Note text everywhere on one page; ✕ clears', (
      tester,
    ) async {
      capture('sauce in the kitchen', kitchen.id);
      capture('sauce at work', work.id);
      capture('nees mor sauce');
      capture('call the garage', home.id);
      await openBrowser(tester);
      expect(find.text('Household'), findsOneWidget);

      await search(tester, 'SAUCE');
      expect(cardTexts(tester), [
        'nees mor sauce',
        'sauce at work',
        'sauce in the kitchen',
      ]);
      expect(find.text('Household'), findsNothing);

      await search(tester, 'nothing like it');
      expect(find.text('No Note matches.'), findsOneWidget);

      await tester.tap(find.byTooltip('Clear search'));
      await tester.pumpAndSettle();
      expect(find.byType(NoteCard), findsNothing);
      expect(find.text('Household'), findsOneWidget);
      expect(find.text('Work'), findsOneWidget);
      expect(
        tester.widget<TextField>(find.byType(TextField)).controller?.text,
        isEmpty,
      );
    });

    testWidgets('inside an Area or Topic only its own cards are searched', (
      tester,
    ) async {
      capture('sauce in the kitchen', kitchen.id);
      capture('sauce at home', home.id);
      capture('sauce at work', work.id);
      capture('unsorted sauce');
      await openBrowser(tester);
      await open(tester, 'Household');

      await search(tester, 'sauce');
      expect(cardTexts(tester), ['sauce at home', 'sauce in the kitchen']);
      expect(find.text('Kitchen'), findsNothing);

      await tester.tap(find.byTooltip('Clear search'));
      await tester.pumpAndSettle();
      await open(tester, 'Kitchen');
      await search(tester, 'sauce');
      expect(cardTexts(tester), ['sauce in the kitchen']);
    });
  });

  group('levels', () {
    testWidgets('an Area shows its Topics and its diary, newest first', (
      tester,
    ) async {
      capture('older, in the kitchen', kitchen.id);
      capture('newer, in the Area', home.id);
      capture('elsewhere', work.id);
      await openBrowser(tester);
      await open(tester, 'Household');

      expect(find.text('Kitchen'), findsOneWidget);
      expect(find.text('Garden'), findsOneWidget);
      expect(cardTexts(tester), [
        'newer, in the Area',
        'older, in the kitchen',
      ]);

      await open(tester, 'Kitchen');
      expect(cardTexts(tester), ['older, in the kitchen']);
    });

    testWidgets('an empty Topic says so', (tester) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      await open(tester, 'Garden');
      expect(find.text('No Notes here yet. Tap + to add one.'), findsOneWidget);
    });

    testWidgets('+ in a Topic captures a Note filed into it', (tester) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      await open(tester, 'Kitchen');

      await tester.tap(find.byIcon(Icons.add));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'nees mor sauce');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(cardTexts(tester), ['nees mor sauce']);
      expect(notes.all().single.place, kitchen.id);
    });

    testWidgets('+ adds an Area on the Areas level, a Topic in an Area', (
      tester,
    ) async {
      await openBrowser(tester);
      expect(find.byTooltip('New Area'), findsOneWidget);
      await open(tester, 'Household');
      expect(find.byTooltip('New Topic'), findsOneWidget);
    });

    testWidgets('no level has a bottom bar', (tester) async {
      Future<void> noBottomBar() async {
        expect(find.byType(BottomNavigationBar), findsNothing);
        expect(find.byType(NavigationBar), findsNothing);
        expect(find.byType(BottomAppBar), findsNothing);
        for (final scaffold in tester.widgetList<Scaffold>(
          find.byType(Scaffold),
        )) {
          expect(scaffold.bottomNavigationBar, isNull);
          expect(scaffold.bottomSheet, isNull);
        }
      }

      await openBrowser(tester);
      await noBottomBar();
      await open(tester, 'Household');
      await noBottomBar();
      await open(tester, 'Kitchen');
      await noBottomBar();
    });
  });

  group('swipes', () {
    testWidgets('right goes back: Topic, Area, Areas, then the home screen', (
      tester,
    ) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      await open(tester, 'Kitchen');
      expect(find.byType(TopicScreen), findsOneWidget);

      await swipe(tester, 400);
      expect(find.byType(TopicScreen), findsNothing);
      expect(find.byType(AreaScreen), findsOneWidget);
      await swipe(tester, 400);
      expect(find.byType(AreaScreen), findsNothing);
      expect(find.byType(AreasScreen), findsOneWidget);
      await swipe(tester, 400);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(NotesScreen), findsOneWidget);
    });

    testWidgets('left reopens the highlighted last Area and Topic', (
      tester,
    ) async {
      await openBrowser(tester);
      expect(highlighted(tester, 'Household'), isFalse);
      // Nothing selected yet: swipe left stays.
      await swipe(tester, -400);
      expect(find.byType(AreaScreen), findsNothing);

      await open(tester, 'Household');
      await open(tester, 'Kitchen');
      await swipe(tester, 400);
      expect(highlighted(tester, 'Kitchen'), isTrue);
      expect(highlighted(tester, 'Garden'), isFalse);
      await swipe(tester, 400);
      expect(highlighted(tester, 'Household'), isTrue);
      expect(highlighted(tester, 'Work'), isFalse);
      await swipe(tester, 400);

      // Leaving the browser keeps the way back in.
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      expect(highlighted(tester, 'Household'), isTrue);
      await swipe(tester, -400);
      expect(find.text('Household'), findsOneWidget);
      expect(find.byType(AreaScreen), findsOneWidget);
      await swipe(tester, -400);
      expect(find.byType(TopicScreen), findsOneWidget);
      expect(find.text('Kitchen'), findsOneWidget);
    });

    testWidgets('a last Topic of another Area is not where left leads', (
      tester,
    ) async {
      places.addTopic(work.id, 'Meetings');
      await openBrowser(tester);
      await open(tester, 'Household');
      await open(tester, 'Kitchen');
      await swipe(tester, 400);
      await swipe(tester, 400);
      await open(tester, 'Work');

      expect(highlighted(tester, 'Meetings'), isFalse);
      await swipe(tester, -400);
      expect(find.byType(TopicScreen), findsNothing);
    });
  });
}
