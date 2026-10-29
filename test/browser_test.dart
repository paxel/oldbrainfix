import 'package:flutter/gestures.dart';
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

  /// Where the page of [screen] starts, as the eye sees it.
  double pageLeft(WidgetTester tester, Type screen) => tester
      .getTopLeft(
        find
            .descendant(
              of: find.byType(screen),
              matching: find.byType(Scaffold),
            )
            .first,
      )
      .dx;

  /// How many pages the app's navigator holds: a preview shares the
  /// route of the page it sits beside.
  int routes(WidgetTester tester) => {
    for (final page in find.byType(Scaffold, skipOffstage: false).evaluate())
      ModalRoute.of(page),
  }.length;

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
      await tester.tap(find.byKey(const ValueKey('captureSave')));
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
      await tester.tap(find.byKey(const ValueKey('captureSave')));
      await tester.pumpAndSettle();
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

    testWidgets('the next level slides in with the finger', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final width = tester.getSize(find.byType(BrowserSwipes)).width;

      final finger = await tester.startGesture(
        tester.getCenter(find.byType(BrowserSwipes)),
      );
      await finger.moveBy(const Offset(-30, 0));
      await finger.moveBy(const Offset(-200, 0));
      await tester.pump();
      final agenda = pageLeft(tester, AgendaScreen);
      final areas = pageLeft(tester, AreasScreen);
      expect(agenda, lessThan(-100));
      expect(areas, greaterThan(width / 2));
      expect(areas, lessThan(width - 100));
      // Side by side: the Areas list starts where the Agenda ends.
      expect(areas, closeTo(agenda + width, 1));
      // Only a preview while the finger is down: nothing pushed yet.
      expect(routes(tester), 1);

      await finger.moveBy(const Offset(100, 0));
      await tester.pump();
      expect(pageLeft(tester, AreasScreen), closeTo(areas + 100, 1));

      // Letting go there, before the middle, springs back.
      await finger.up();
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);
      expect(routes(tester), 1);
    });

    testWidgets('the pushed level takes over where the preview was', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final width = tester.getSize(find.byType(BrowserSwipes)).width;

      final finger = await tester.startGesture(
        tester.getCenter(find.byType(BrowserSwipes)),
      );
      await finger.moveBy(const Offset(-30, 0));
      await finger.moveBy(const Offset(-500, 0));
      await tester.pump();
      final areas = pageLeft(tester, AreasScreen);
      final agenda = pageLeft(tester, AgendaScreen);
      expect(areas, lessThan(width / 2));
      await finger.up();
      // The frame of the push, then the frame of the take-over.
      for (var i = 0; i < 2; i++) {
        await tester.pump();
        expect(find.byType(AreasScreen), findsOneWidget);
        expect(pageLeft(tester, AreasScreen), closeTo(areas, 1));
        expect(pageLeft(tester, AgendaScreen), closeTo(agenda, 1));
      }
      await tester.pump(const Duration(milliseconds: 50));
      expect(pageLeft(tester, AreasScreen), lessThan(areas));
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);
      expect(pageLeft(tester, AreasScreen), 0);
      expect(routes(tester), 2);
    });

    testWidgets('going back the page and the level under it follow', (
      tester,
    ) async {
      await openBrowser(tester);
      final width = tester.getSize(find.byType(BrowserSwipes)).width;

      final finger = await tester.startGesture(
        tester.getCenter(find.byType(BrowserSwipes)),
      );
      await finger.moveBy(const Offset(30, 0));
      await finger.moveBy(const Offset(200, 0));
      await tester.pump();
      final areas = pageLeft(tester, AreasScreen);
      final agenda = pageLeft(tester, AgendaScreen);
      expect(areas, greaterThan(100));
      expect(agenda, closeTo(areas - width, 1));
      await finger.up();
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(pageLeft(tester, AreasScreen), 0);
    });

    testWidgets('past the middle completes, forward and back', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      // Slow: no fling, only the distance counts.
      await tester.timedDrag(
        find.byType(BrowserSwipes),
        const Offset(-560, 0),
        const Duration(seconds: 3),
      );
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await tester.timedDrag(
        find.byType(BrowserSwipes),
        const Offset(560, 0),
        const Duration(seconds: 3),
      );
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('a slow drag turns past a third, springs back short of it', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final third = tester.getSize(find.byType(BrowserSwipes)).width / 3;
      // The drag only counts once the finger moved past the touch slop.
      final past = third + kTouchSlop + 10;
      final short = third - 10;

      Future<void> drag(double dx) async {
        await tester.timedDrag(
          find.byType(BrowserSwipes),
          Offset(dx, 0),
          const Duration(seconds: 3),
        );
        await tester.pumpAndSettle();
      }

      await drag(-short);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);

      await drag(-past);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await drag(short);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);
      expect(pageLeft(tester, AreasScreen), 0);

      await drag(past);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('a short slow drag springs back, forward and back', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      await tester.timedDrag(
        find.byType(BrowserSwipes),
        const Offset(-200, 0),
        const Duration(seconds: 2),
      );
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);

      // Taps still work afterwards.
      await openBrowser(tester);
      await open(tester, 'Household');
      expect(find.byType(AreaScreen), findsOneWidget);

      await tester.timedDrag(
        find.byType(BrowserSwipes),
        const Offset(200, 0),
        const Duration(seconds: 2),
      );
      await tester.pumpAndSettle();
      expect(find.byType(AreaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AreaScreen), 0);

      await open(tester, 'Kitchen');
      expect(find.byType(TopicScreen), findsOneWidget);
    });

    testWidgets('a short fling completes, forward and back', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      await swipe(tester, -150);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await swipe(tester, 150);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('with animations off nothing follows; letting go changes '
        'at once', (tester) async {
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(disableAnimations: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      var finger = await tester.startGesture(
        tester.getCenter(find.byType(BrowserSwipes)),
      );
      await finger.moveBy(const Offset(-30, 0));
      await finger.moveBy(const Offset(-500, 0));
      await tester.pump();
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);
      await finger.up();
      await tester.pump();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);
      expect(pageLeft(tester, AreasScreen), 0);

      finger = await tester.startGesture(
        tester.getCenter(find.byType(BrowserSwipes)),
      );
      await finger.moveBy(const Offset(30, 0));
      await finger.moveBy(const Offset(500, 0));
      await tester.pump();
      expect(find.byType(AgendaScreen), findsNothing);
      expect(pageLeft(tester, AreasScreen), 0);
      await finger.up();
      await tester.pump();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
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
