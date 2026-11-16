// The search bar of a list page scrolls away with the list and comes back
// as soon as the list is scrolled up a little (#336, Q749).
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  /// Two digits, so the names sort as numbered.
  String nn(int i) => i.toString().padLeft(2, '0');

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    // Enough of each that every page scrolls far.
    for (var i = 1; i <= 30; i++) {
      places.addArea('Zone ${nn(i)}');
      now = now.add(const Duration(minutes: 1));
      notes.capture('Note ${nn(i)}', place: home.id);
      now = now.add(const Duration(minutes: 1));
      notes.capture('Kitchen note ${nn(i)}', place: kitchen.id);
    }
  });
  tearDown(() => store.close());

  Future<void> openBrowser(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
  }

  Future<void> open(WidgetTester tester, String row) async {
    await tester.tap(find.text(row));
    await tester.pumpAndSettle();
  }

  /// The pages with the bar, each with the rows its list shows.
  final pages = <String, (List<String>, Future<void> Function(WidgetTester))>{
    'the Areas list': (
      [for (var i = 1; i <= 30; i++) 'Zone ${nn(i)}'],
      openBrowser,
    ),
    'an Area': (
      [for (var i = 1; i <= 30; i++) 'Note ${nn(i)}'],
      (tester) async {
        await openBrowser(tester);
        await open(tester, 'Household');
      },
    ),
    'a Topic': (
      [for (var i = 1; i <= 30; i++) 'Kitchen note ${nn(i)}'],
      (tester) async {
        await openBrowser(tester);
        await open(tester, 'Household');
        await open(tester, 'Kitchen');
      },
    ),
  };

  final field = find.byType(TextField);

  /// The bar's field on the page on top, there while it is scrolled
  /// away; the pages under it keep theirs.
  final kept = find.byType(TextField, skipOffstage: false).last;

  /// The list the bar heads.
  final list = find
      .ancestor(of: kept, matching: find.byType(CustomScrollView))
      .first;

  double listTop(WidgetTester tester) => tester.getRect(list).top;

  ScrollPosition position(WidgetTester tester) => tester
      .state<ScrollableState>(
        find.descendant(of: list, matching: find.byType(Scrollable)).first,
      )
      .position;

  /// The bar stands fully shown at the top of the list, as at rest.
  void barShown(WidgetTester tester) {
    expect(tester.getRect(field).top, listTop(tester) + 8);
    expect(field.hitTestable(), findsOneWidget);
  }

  /// The bar is gone: nothing of it covers the list's top edge, and its
  /// field is still there, offstage, with what it holds.
  void barGone(WidgetTester tester) {
    expect(field.hitTestable(), findsNothing);
    expect(tester.getRect(kept).bottom, lessThanOrEqualTo(listTop(tester)));
  }

  /// Drags the list [dy] in small steps and lets go standing still: no
  /// fling, only the distance counts.
  Future<void> slowScroll(WidgetTester tester, double dy) async {
    final finger = await tester.startGesture(
      tester.getCenter(list) + const Offset(0, 40),
    );
    const steps = 10;
    for (var i = 1; i <= steps; i++) {
      await finger.moveBy(
        Offset(0, dy / steps),
        timeStamp: Duration(milliseconds: 20 * i),
      );
      await tester.pump();
    }
    await finger.up(timeStamp: const Duration(milliseconds: 20 * steps + 500));
    await tester.pumpAndSettle();
  }

  /// The first of [rows] well inside the list, away from both edges.
  Finder shownRow(WidgetTester tester, List<String> rows) {
    final shown = tester.getRect(list).deflate(100);
    for (final row in rows) {
      final found = find.text(row);
      if (found.evaluate().isEmpty) continue;
      final rect = tester.getRect(found.first);
      if (rect.top > shown.top && rect.bottom < shown.bottom) {
        return found.first;
      }
    }
    throw StateError('no row in sight');
  }

  for (final MapEntry(key: page, value: (rows, openPage)) in pages.entries) {
    group(page, () {
      testWidgets('the bar scrolls away with the list and comes back over '
          'it once the list goes up a little, the list staying put', (
        tester,
      ) async {
        await openPage(tester);
        barShown(tester);

        await slowScroll(tester, -400);
        barGone(tester);

        final row = shownRow(tester, rows);
        final rowTop = tester.getRect(row).top;
        final pixels = position(tester).pixels;
        // Less than the bar is high: it snaps fully shown.
        await slowScroll(tester, 40);
        barShown(tester);
        final moved = pixels - position(tester).pixels;
        expect(moved, greaterThan(0));
        // The bar comes over the list; the list moves only with the finger.
        expect(tester.getRect(row).top, rowTop + moved);

        // Down again a little: it snaps away.
        await slowScroll(tester, -40);
        barGone(tester);
      });

      testWidgets('the bar stays while its field has the focus, which '
          'moves nothing and keeps the keyboard', (tester) async {
        await openPage(tester);
        await slowScroll(tester, -400);
        await slowScroll(tester, 40);
        barShown(tester);

        final pixels = position(tester).pixels;
        await tester.tap(field);
        await tester.pumpAndSettle();
        // Focusing does not pull the list back to the top to reveal it.
        expect(position(tester).pixels, pixels);
        expect(tester.testTextInput.isVisible, isTrue);

        await slowScroll(tester, -400);
        barShown(tester);
        expect(position(tester).pixels, greaterThan(pixels));

        // The keyboard still types into it, and typing moves nothing.
        final scrolled = position(tester).pixels;
        tester.testTextInput.enterText(' ');
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(field).controller!.text, ' ');
        expect(position(tester).pixels, scrolled);
        barShown(tester);
      });

      testWidgets('the bar stays while it holds a search, and scrolls away '
          'again once it is cleared', (tester) async {
        await openPage(tester);
        await tester.tap(field);
        await tester.pumpAndSettle();
        tester.testTextInput.enterText('note');
        await tester.pumpAndSettle();
        // Search on the keyboard: the field lets go of the focus.
        await tester.testTextInput.receiveAction(TextInputAction.search);
        await tester.pumpAndSettle();
        expect(tester.testTextInput.isVisible, isFalse);

        await slowScroll(tester, -400);
        barShown(tester);
        expect(tester.widget<TextField>(field).controller!.text, 'note');

        // Cleared down in the list, the bar goes as a bar without a search
        // does there, and a little up brings it back.
        await tester.tap(find.byTooltip('Clear search'));
        await tester.pumpAndSettle();
        barGone(tester);
        await slowScroll(tester, 40);
        barShown(tester);
        expect(tester.widget<TextField>(field).controller!.text, isEmpty);
      });
    });
  }

  group('pages without a search bar keep their list as it is', () {
    testWidgets('All Notes and the Trash', (tester) async {
      await openBrowser(tester);
      for (final page in ['All Notes', 'Trash']) {
        await tester.tap(find.byTooltip(page));
        await tester.pumpAndSettle();
        expect(find.byType(SearchHeaderSliver), findsNothing);
        expect(field, findsNothing);
        await tester.pageBack();
        await tester.pumpAndSettle();
      }
    });

    testWidgets('Unsorted', (tester) async {
      notes.capture('Not filed');
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: UnsortedScreen(notes: notes, places: places),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(SearchHeaderSliver), findsNothing);
      expect(field, findsNothing);
    });
  });
}
