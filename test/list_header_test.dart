// The search bar of a list page scrolls away with the list and comes back
// as soon as the list is scrolled up a little (#336, Q749), on a band of
// the top bar's colour, set off from the page, with one line under it
// (#351, obf-1041).
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/look.dart';
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

  /// The band the bar sits on, on the page on top.
  final band = find
      .descendant(
        of: find.byType(SearchHeaderSliver),
        matching: find.byWidgetPredicate(
          (w) => w is DecoratedBox && w.decoration is BoxDecoration,
        ),
      )
      .first;

  /// The contrast of two colours, 1 for the same, as WCAG reckons it.
  double contrast(Color a, Color b) {
    final (light, dark) = a.computeLuminance() > b.computeLuminance()
        ? (a.computeLuminance(), b.computeLuminance())
        : (b.computeLuminance(), a.computeLuminance());
    return (light + 0.05) / (dark + 0.05);
  }

  /// The fill of the top bar of the page on top.
  Color? barColour(WidgetTester tester) => tester
      .widget<Material>(
        find
            .descendant(
              of: find.byType(AppBar).last,
              matching: find.byType(Material),
            )
            .first,
      )
      .color;

  /// The bar sits on an opaque band of the top bar's colour, top bar and
  /// band one header set off from the page's ground, with one line along
  /// the band's bottom that reads against it, and no line between bar and
  /// band.
  void onBand(WidgetTester tester) {
    final decoration =
        tester.widget<DecoratedBox>(band).decoration as BoxDecoration;
    final colour = decoration.color;
    final ground = LookTones.of(Theme.of(tester.element(band)).brightness)
        .background;
    expect(colour, isNotNull);
    expect(colour, barColour(tester));
    expect(colour, isNot(ground));
    expect(colour?.a, 1.0);
    final border = decoration.border;
    expect(border, isA<Border>());
    if (border is! Border || colour == null) return;
    expect(border.top, BorderSide.none);
    expect(border.left, BorderSide.none);
    expect(border.right, BorderSide.none);
    expect(border.bottom.width, greaterThan(0));
    expect(contrast(border.bottom.color, colour), greaterThan(1.2));
    // The band holds the whole bar, the line at its bottom.
    final shown = tester.getRect(band);
    expect(shown.top, lessThan(tester.getRect(field).top));
    expect(shown.bottom, greaterThan(tester.getRect(field).bottom));
  }

  void systemBrightness(WidgetTester tester, Brightness brightness) {
    tester.platformDispatcher.platformBrightnessTestValue = brightness;
    addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
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

      for (final brightness in Brightness.values) {
        testWidgets('in the $brightness look the bar sits on a band of the '
            'top bar\'s colour, over the list and with a search', (
          tester,
        ) async {
          systemBrightness(tester, brightness);
          await openPage(tester);
          onBand(tester);

          // Floated back over the list.
          await slowScroll(tester, -400);
          await slowScroll(tester, 40);
          barShown(tester);
          onBand(tester);

          // Kept over the list by a search.
          await tester.tap(field);
          await tester.pumpAndSettle();
          tester.testTextInput.enterText('note');
          await tester.pumpAndSettle();
          await tester.testTextInput.receiveAction(TextInputAction.search);
          await tester.pumpAndSettle();
          await slowScroll(tester, -400);
          barShown(tester);
          onBand(tester);
        });
      }

      testWidgets('pulled down, the bar stays on its band at the top and '
          'the list moves away under it', (tester) async {
        await openPage(tester);
        final row = shownRow(tester, rows);
        final rowTop = tester.getRect(row).top;
        final finger = await tester.startGesture(
          tester.getCenter(list) - const Offset(0, 100),
        );
        for (var i = 0; i < 10; i++) {
          await finger.moveBy(const Offset(0, 20));
          await tester.pump();
        }
        // Still pulled: the band stays fixed at the list's top.
        expect(tester.getRect(band).top, listTop(tester));
        onBand(tester);
        expect(tester.getRect(row).top, greaterThan(rowTop));
        expect(
          tester.getRect(row).top,
          greaterThanOrEqualTo(tester.getRect(band).bottom),
        );
        await finger.up();
        await tester.pumpAndSettle();
        barShown(tester);
        onBand(tester);
      }, variant: TargetPlatformVariant.only(TargetPlatform.iOS));
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
        // Their bar keeps the page's colour.
        expect(
          barColour(tester),
          LookTones.of(
            Theme.of(tester.element(find.byType(AppBar).last)).brightness,
          ).background,
        );
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
