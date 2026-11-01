import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/lock_screen.dart';
import 'package:oldbrainfix/src/named_places_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Every page with a floating button scrolls its last row above it
/// (#158), on top of the room for the navigation bar (#61).
void main() {
  // The three-button navigation bar of a Pixel: 48 logical pixels.
  const bar = 48.0;

  late Store store;
  late Notes notes;
  late Places places;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  /// A phone [height] high whose bottom [bar] belongs to the system.
  void phoneWithNavigationBar(WidgetTester tester, {double height = 800}) {
    final ratio = tester.view.devicePixelRatio;
    tester.view.physicalSize = Size(400 * ratio, height * ratio);
    final inset = FakeViewPadding(bottom: bar * ratio);
    tester.view.padding = inset;
    tester.view.viewPadding = inset;
    addTearDown(tester.view.reset);
  }

  /// Captures [text] a minute after the last one, filed into [place].
  void capture(String text, [String? place]) {
    now = now.add(const Duration(minutes: 1));
    notes.capture(text, place: place);
  }

  Future<void> openBrowser(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
  }

  Future<void> open(WidgetTester tester, String row) async {
    await tester.tap(find.text(row));
    await tester.pumpAndSettle();
  }

  /// The page's own list, not a search field's.
  Finder list() => find
      .descendant(
        of: find.byWidgetPredicate(
          (w) => w is ScrollView || w is SingleChildScrollView,
        ),
        matching: find.byType(Scrollable),
      )
      .first;

  Future<void> scrollToEnd(WidgetTester tester) async {
    await tester.fling(list(), const Offset(0, -20000), 5000);
    await tester.pumpAndSettle();
  }

  /// Scrolled to the end, the last row of the list ends above every
  /// floating button.
  Future<void> expectLastRowAboveButtons(WidgetTester tester) async {
    await scrollToEnd(tester);
    final rows = find.descendant(of: list(), matching: find.byType(Embossed));
    expect(rows, findsWidgets);
    final buttons = find.byType(FloatingActionButton);
    expect(buttons, findsWidgets);
    final top = [
      for (var i = 0; i < buttons.evaluate().length; i++)
        tester.getRect(buttons.at(i)).top,
    ].reduce(math.min);
    expect(tester.getRect(rows.last).bottom, lessThanOrEqualTo(top));
  }

  testWidgets('the Agenda scrolls its last row above "+"', (tester) async {
    phoneWithNavigationBar(tester);
    final routines = Routines(store);
    for (var i = 0; i < 6; i++) {
      routines.create(
        'Routine $i',
        Schedule(
          start: Day(2026, 10, 1),
          times: [ClockTime(10 + i, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
    }
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('the Areas list scrolls its last Area above "+"', (tester) async {
    phoneWithNavigationBar(tester);
    for (var i = 0; i < 30; i++) {
      places.addArea('Area $i');
    }
    await openBrowser(tester);

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('search results scroll their last Note above "+"', (
    tester,
  ) async {
    phoneWithNavigationBar(tester);
    for (var i = 0; i < 30; i++) {
      capture('Milk $i');
    }
    await openBrowser(tester);
    await tester.enterText(find.byType(TextField), 'milk');
    await tester.pumpAndSettle();

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('All Notes scrolls its last Note above "+"', (tester) async {
    phoneWithNavigationBar(tester);
    for (var i = 0; i < 30; i++) {
      capture('Milk $i');
    }
    await openBrowser(tester);
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('an Area scrolls its last row above "+"', (tester) async {
    phoneWithNavigationBar(tester);
    final home = places.addArea('Household');
    for (var i = 0; i < 30; i++) {
      places.addTopic(home.id, 'Topic $i');
    }
    await openBrowser(tester);
    await open(tester, 'Household');

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('a Topic scrolls its last Note above "+"', (tester) async {
    phoneWithNavigationBar(tester);
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    for (var i = 0; i < 30; i++) {
      capture('Note $i', kitchen.id);
    }
    await openBrowser(tester);
    await open(tester, 'Household');
    await open(tester, 'Kitchen');

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('Named places scroll their last place above "+"', (tester) async {
    phoneWithNavigationBar(tester);
    final named = NamedPlaces(store);
    for (var i = 0; i < 30; i++) {
      named.add('Place $i', Coordinates(52, 13), radius: 100);
    }
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: NamedPlacesScreen(namedPlaces: named),
      ),
    );
    await tester.pumpAndSettle();

    await expectLastRowAboveButtons(tester);
  });

  testWidgets('the lock screen scrolls its card above "+"', (tester) async {
    // Short enough that the card has to scroll.
    phoneWithNavigationBar(tester, height: 420);
    final lock = AppLock(store: store);
    addTearDown(lock.dispose);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: LockScope(
          lock: lock,
          child: LockScreen(lock: lock, onCapture: (_) {}),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await expectLastRowAboveButtons(tester);
  });
}
