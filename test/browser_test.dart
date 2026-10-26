import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
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
      // The Topic's Note is marked with its Topic.
      expect(find.text('Household / Kitchen'), findsOneWidget);
      final kitchenCard = tester.widget<NoteCard>(
        find.ancestor(
          of: find.text('sauce in the kitchen'),
          matching: find.byType(NoteCard),
        ),
      );
      expect(kitchenCard.topic?.id, kitchen.id);

      await tester.tap(find.byTooltip('Clear search'));
      await tester.pumpAndSettle();
      await open(tester, 'Kitchen');
      await search(tester, 'sauce');
      expect(cardTexts(tester), ['sauce in the kitchen']);
    });
  });

  group('where a card says it is filed', () {
    /// The place lines on the cards shown, by Note text.
    Map<String, bool> placeShown(WidgetTester tester) => {
      for (final card in tester.widgetList<NoteCard>(find.byType(NoteCard)))
        card.note.text: card.showPlace,
    };

    Finder onCard(String text) =>
        find.descendant(of: find.byType(NoteCard), matching: find.text(text));

    testWidgets('a search across every Area names each hit\'s place', (
      tester,
    ) async {
      capture('sauce in the kitchen', kitchen.id);
      capture('sauce at work', work.id);
      capture('nees mor sauce');
      await openBrowser(tester);

      await search(tester, 'sauce');
      expect(placeShown(tester), {
        'nees mor sauce': true,
        'sauce at work': true,
        'sauce in the kitchen': true,
      });
      expect(onCard('Household / Kitchen'), findsOneWidget);
      expect(onCard('Work'), findsOneWidget);
      expect(onCard('Unsorted'), findsOneWidget);
    });

    testWidgets('an Area, a Topic and Unsorted name no place on the cards', (
      tester,
    ) async {
      capture('in the Area', home.id);
      capture('in the kitchen', kitchen.id);
      capture('not filed');
      await openBrowser(tester);
      await open(tester, 'Household');
      expect(placeShown(tester), {'in the Area': false});
      expect(onCard('Household'), findsNothing);

      // A search inside the Area names the Topic of a Topic's hit only.
      await search(tester, 'in the');
      expect(placeShown(tester), {
        'in the kitchen': true,
        'in the Area': false,
      });
      expect(onCard('Household / Kitchen'), findsOneWidget);
      expect(onCard('Household'), findsNothing);

      await tester.tap(find.byTooltip('Clear search'));
      await tester.pumpAndSettle();
      await open(tester, 'Kitchen');
      expect(placeShown(tester), {'in the kitchen': false});
      expect(onCard('Household / Kitchen'), findsNothing);
      await search(tester, 'kitchen');
      expect(placeShown(tester), {'in the kitchen': false});

      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: UnsortedScreen(notes: notes, places: places),
        ),
      );
      await tester.pumpAndSettle();
      expect(placeShown(tester), {'not filed': false});
      expect(onCard('Unsorted'), findsNothing);
    });
  });

  group('levels', () {
    testWidgets('an Area shows its Topics and only its Notes without one', (
      tester,
    ) async {
      capture('older, in the Area', home.id);
      capture('in the kitchen', kitchen.id);
      capture('newer, in the Area', home.id);
      capture('elsewhere', work.id);
      await openBrowser(tester);
      await open(tester, 'Household');

      expect(find.text('Kitchen'), findsOneWidget);
      expect(find.text('Garden'), findsOneWidget);
      expect(cardTexts(tester), ['newer, in the Area', 'older, in the Area']);

      await open(tester, 'Kitchen');
      expect(cardTexts(tester), ['in the kitchen']);
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

    testWidgets('+ adds an Area on the Areas level', (tester) async {
      await openBrowser(tester);
      expect(find.byTooltip('New Area'), findsOneWidget);
    });

    testWidgets('+ in an Area offers a Note filed into it or a Topic', (
      tester,
    ) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      final plus = tester.getCenter(find.byTooltip('Add here'));
      await tester.tap(find.byTooltip('Add here'));
      await tester.pumpAndSettle();
      // The menu opens where the finger is, not from the bottom edge.
      final menu = tester.getRect(
        find.ancestor(
          of: find.text('Note here'),
          matching: find.byWidgetPredicate((w) => w is PopupMenuItem),
        ),
      );
      expect((menu.center - plus).distance, lessThan(200));
      expect(find.text('Topic'), findsOneWidget);
      expect(find.byType(BottomSheet), findsNothing);

      await open(tester, 'Note here');
      await tester.enterText(find.byType(TextField), 'nees mor sauce');
      await open(tester, 'Save');
      expect(cardTexts(tester), ['nees mor sauce']);
      expect(notes.all().single.place, home.id);

      await tester.tap(find.byTooltip('Add here'));
      await tester.pumpAndSettle();
      await open(tester, 'Topic');
      await tester.enterText(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(TextField),
        ),
        'Cellar',
      );
      await open(tester, 'Create');
      expect(find.text('Cellar'), findsOneWidget);
      expect(places.topics(home.id).map((t) => t.name), contains('Cellar'));
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
    testWidgets('left on the Agenda opens the Areas list, right returns', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);

      // The Agenda is the top level: right leads nowhere.
      await swipe(tester, 400);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);

      await swipe(tester, -400);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await swipe(tester, 400);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('a vertical drag on the Agenda scrolls, it does not swipe', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.fling(
        find.byType(BrowserSwipes),
        const Offset(0, -400),
        1000,
      );
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
    });

    testWidgets('from the Agenda left goes on into the last Area and Topic', (
      tester,
    ) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      await open(tester, 'Kitchen');
      for (var i = 0; i < 3; i++) {
        await swipe(tester, 400);
      }
      expect(find.byType(AgendaScreen), findsOneWidget);

      await swipe(tester, -400);
      expect(find.byType(AreasScreen), findsOneWidget);
      await swipe(tester, -400);
      expect(find.byType(AreaScreen), findsOneWidget);
      await swipe(tester, -400);
      expect(find.byType(TopicScreen), findsOneWidget);
    });

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
      expect(find.byType(AgendaScreen), findsOneWidget);
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
