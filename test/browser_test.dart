import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
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

  /// The app's whole screen.
  final screen = find.byType(MaterialApp);

  /// The middle of the screen, where a swipe starts.
  Offset middle(WidgetTester tester) => tester.getCenter(screen);

  Future<void> swipe(WidgetTester tester, double dx) async {
    await tester.flingFrom(middle(tester), Offset(dx, 0), 1000);
    await tester.pumpAndSettle();
  }

  /// Drags [dx] sideways in small steps and lets go standing still: no
  /// fling, only the distance counts.
  Future<void> slowDrag(WidgetTester tester, double dx) async {
    final finger = await tester.startGesture(middle(tester));
    const steps = 20;
    for (var i = 1; i <= steps; i++) {
      await finger.moveBy(
        Offset(dx / steps, 0),
        timeStamp: Duration(milliseconds: 50 * i),
      );
      await tester.pump();
    }
    await finger.up(timeStamp: const Duration(milliseconds: 50 * steps + 500));
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
  /// route of the page it sits beside, the Areas list the Agenda's.
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
      await tester.tap(find.byKey(const ValueKey('addFan-note')));
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

    testWidgets('+ in an Area fans out a Note filed into it, the other '
        'kinds and a Topic', (tester) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      final plus = tester.getRect(find.byTooltip('Add here'));
      await tester.tap(find.byTooltip('Add here'));
      await tester.pumpAndSettle();
      // The fan opens upward from the button, not from the bottom edge.
      final note = find.byKey(const ValueKey('addFan-note'));
      expect(tester.getRect(note).bottom, lessThanOrEqualTo(plus.top));
      expect((tester.getCenter(note) - plus.center).distance, lessThan(200));
      expect(find.text('Topic'), findsOneWidget);
      expect(find.byType(BottomSheet), findsNothing);

      await tester.tap(note);
      await tester.pumpAndSettle();
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

    testWidgets('the Areas list slides in with the finger, beside the '
        'Agenda', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final width = tester.getSize(screen).width;

      final finger = await tester.startGesture(middle(tester));
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
      // Two pages of one route: nothing is pushed.
      expect(routes(tester), 1);

      await finger.moveBy(const Offset(100, 0));
      await tester.pump();
      expect(pageLeft(tester, AreasScreen), closeTo(areas + 100, 1));

      // Letting go there, before the middle, springs back.
      await finger.up(timeStamp: const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);
      expect(routes(tester), 1);
    });

    testWidgets('the pushed level takes over where the preview was', (
      tester,
    ) async {
      await openBrowser(tester);
      await open(tester, 'Household');
      await swipe(tester, 400);
      expect(find.byType(AreasScreen), findsOneWidget);
      final width = tester.getSize(screen).width;

      final finger = await tester.startGesture(middle(tester));
      await finger.moveBy(const Offset(-30, 0));
      await finger.moveBy(const Offset(-500, 0));
      await tester.pump();
      final area = pageLeft(tester, AreaScreen);
      final areas = pageLeft(tester, AreasScreen);
      expect(area, lessThan(width / 2));
      await finger.up();
      // The frame of the push, then the frame of the take-over.
      for (var i = 0; i < 2; i++) {
        await tester.pump();
        expect(find.byType(AreaScreen), findsOneWidget);
        expect(pageLeft(tester, AreaScreen), closeTo(area, 1));
        expect(pageLeft(tester, AreasScreen), closeTo(areas, 1));
      }
      await tester.pump(const Duration(milliseconds: 50));
      expect(pageLeft(tester, AreaScreen), lessThan(area));
      await tester.pumpAndSettle();
      expect(find.byType(AreaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AreaScreen), 0);
      expect(routes(tester), 2);
    });

    testWidgets('going back the page and the level under it follow', (
      tester,
    ) async {
      await openBrowser(tester);
      final width = tester.getSize(screen).width;

      final finger = await tester.startGesture(middle(tester));
      await finger.moveBy(const Offset(30, 0));
      await finger.moveBy(const Offset(200, 0));
      await tester.pump();
      final areas = pageLeft(tester, AreasScreen);
      final agenda = pageLeft(tester, AgendaScreen);
      expect(areas, greaterThan(100));
      expect(agenda, closeTo(areas - width, 1));
      await finger.up(timeStamp: const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(pageLeft(tester, AreasScreen), 0);
    });

    testWidgets('past the middle completes, forward and back', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      await slowDrag(tester, -560);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await slowDrag(tester, 560);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('a slow drag turns past the middle, springs back short of '
        'it', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final half = tester.getSize(screen).width / 2;
      // The pages move only once the finger moved past the touch slop.
      final past = half + kTouchSlop + 10;
      final short = half - 10;

      await slowDrag(tester, -short);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);

      await slowDrag(tester, -past);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await slowDrag(tester, short);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);
      expect(pageLeft(tester, AreasScreen), 0);

      await slowDrag(tester, past);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('a short slow drag springs back, forward and back', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      await slowDrag(tester, -200);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);

      // Taps still work afterwards.
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await open(tester, 'Household');
      expect(find.byType(AreaScreen), findsOneWidget);

      await slowDrag(tester, 200);
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

    testWidgets('a fast short flick switches at once, forward and back', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      // 40 pixels in 16 ms, past the touch slop only with the last move,
      // as a quick thumb sends it.
      Future<void> flick(double dx) async {
        final finger = await tester.startGesture(middle(tester));
        await finger.moveBy(
          Offset(dx / 4, 0),
          timeStamp: const Duration(milliseconds: 8),
        );
        await finger.moveBy(
          Offset(dx * 3 / 4, 0),
          timeStamp: const Duration(milliseconds: 16),
        );
        await finger.up(timeStamp: const Duration(milliseconds: 24));
        await tester.pumpAndSettle();
      }

      await flick(-40);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await flick(40);
      expect(find.byType(AreasScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);

      // A short fling of the test's own, 60 pixels in 30 ms: its last
      // twenty moves must pass the touch slop to count as a fling.
      await tester.flingFrom(middle(tester), const Offset(-60, 0), 2000);
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
    });

    testWidgets('neither page is built while the other comes', (tester) async {
      final routines = Routines(store);
      for (var i = 0; i < 5; i++) {
        routines.create(
          'routine $i',
          Schedule(
            start: Day(2026, 10, 1),
            times: [ClockTime(9 + i, 0)],
            rule: Recurrence.parse('FREQ=DAILY'),
          ),
        );
      }
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final built = <Type, int>{};
      debugOnRebuildDirtyWidget = (element, _) {
        final type = element.widget.runtimeType;
        built[type] = (built[type] ?? 0) + 1;
      };
      addTearDown(() => debugOnRebuildDirtyWidget = null);

      // The Areas list was built ahead: going there builds neither page,
      // not while the finger moves, not as it glides in.
      await swipe(tester, -150);
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(built[AgendaScreen] ?? 0, 0);
      expect(built[AreasScreen] ?? 0, 0);

      // Nor coming back, until the Agenda is in place: then it reads its
      // rows once.
      final finger = await tester.startGesture(middle(tester));
      for (var i = 0; i < 10; i++) {
        await finger.moveBy(
          const Offset(30, 0),
          timeStamp: Duration(milliseconds: 8 * i),
        );
        await tester.pump(const Duration(milliseconds: 8));
      }
      await finger.up(timeStamp: const Duration(milliseconds: 88));
      while (pageLeft(tester, AgendaScreen) < 0) {
        expect(built[AgendaScreen] ?? 0, 0);
        await tester.pump(const Duration(milliseconds: 16));
      }
      await tester.pumpAndSettle();
      debugOnRebuildDirtyWidget = null;
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(built[AgendaScreen], 1);
      expect(built[AreasScreen] ?? 0, 0);

      // An Area pushed from the Areas list does not build the Agenda
      // under it either (BrowserHomeRoute, #210).
      await swipe(tester, -150);
      built.clear();
      debugOnRebuildDirtyWidget = (element, _) {
        final type = element.widget.runtimeType;
        built[type] = (built[type] ?? 0) + 1;
      };
      await open(tester, 'Household');
      debugOnRebuildDirtyWidget = null;
      expect(find.byType(AreaScreen), findsOneWidget);
      expect(built[AgendaScreen] ?? 0, 0);
    });

    testWidgets('a screen reader reads only the page shown', (tester) async {
      final semantics = tester.ensureSemantics();
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      // Not even as the hidden page beside it.
      expect(
        find.bySemanticsLabel('Household', skipOffstage: false),
        findsNothing,
      );

      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      expect(find.bySemanticsLabel('Household'), findsOneWidget);
      expect(
        find.bySemanticsLabel('Search and browse', skipOffstage: false),
        findsNothing,
      );
      semantics.dispose();
    });

    testWidgets('with animations off letting go lands at once', (tester) async {
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(disableAnimations: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      var finger = await tester.startGesture(middle(tester));
      await finger.moveBy(const Offset(-30, 0));
      await finger.moveBy(const Offset(-500, 0));
      await tester.pump();
      await finger.up();
      await tester.pump();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);
      expect(pageLeft(tester, AreasScreen), 0);

      finger = await tester.startGesture(middle(tester));
      await finger.moveBy(const Offset(30, 0));
      await finger.moveBy(const Offset(500, 0));
      await tester.pump();
      await finger.up();
      await tester.pump();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);

      // The folder changes at once too.
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pump();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(pageLeft(tester, AreasScreen), 0);
    });

    testWidgets('a vertical drag on the Agenda scrolls, it does not swipe', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.flingFrom(middle(tester), const Offset(0, -400), 1000);
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
    });

    testWidgets('the folder and back go between the Agenda and the Areas '
        'list', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();

      // The folder slides the Areas list in.
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));
      expect(pageLeft(tester, AreasScreen), greaterThan(0));
      expect(pageLeft(tester, AgendaScreen), lessThan(0));
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);
      expect(routes(tester), 1);

      // The bar's back on the Areas list.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      // The Agenda has no way back in its bar.
      expect(find.byType(BackButton), findsNothing);

      // The system's back, from an Area to the Areas list to the Agenda.
      await tester.tap(find.byTooltip('Search and browse'));
      await tester.pumpAndSettle();
      await open(tester, 'Household');
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
    });

    testWidgets('only a swipe, the folder and back change the page', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      // Revealing the bar's last button moves the pages towards the
      // Areas list; they go back to the Agenda.
      await tester.ensureVisible(find.byKey(const ValueKey('profile')));
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);
      expect(pageLeft(tester, AgendaScreen), 0);
    });

    testWidgets('on iOS a drag from the left edge moves neither page', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final height = tester.getSize(screen).height;

      await tester.flingFrom(
        Offset(5, height / 2),
        const Offset(-150, 0),
        1000,
      );
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(AreasScreen), findsNothing);

      await swipe(tester, -150);
      expect(find.byType(AreasScreen), findsOneWidget);

      await tester.flingFrom(Offset(5, height / 2), const Offset(300, 0), 1000);
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AgendaScreen), findsNothing);

      await swipe(tester, 150);
      expect(find.byType(AgendaScreen), findsOneWidget);
      debugDefaultTargetPlatformOverride = null;
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
      expect(find.byType(AreasScreen), findsOneWidget);

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
