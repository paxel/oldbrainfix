// Every page with a search bar standing over its list, the Settings,
// File into…, Merge into…, Add to Note… and Link to…, sets it on the band
// of the list pages (#351, obf-1041), the page's top bar of its colour,
// with one line under it, so what scrolls under it goes under a line and
// not under an edge in the page's own colour (obf-1088).
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_links.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_settings.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
    final home = places.addArea('Household');
    places.addTopic(home.id, 'Kitchen');
    for (var i = 1; i <= 20; i++) {
      notes.capture('Note $i', place: home.id);
    }
  });
  tearDown(() => store.close());

  /// [page] pushed over a page, as the app opens it, in the look of the
  /// system's brightness.
  Future<void> push(WidgetTester tester, Widget page) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: lookTheme(Brightness.light),
        darkTheme: lookTheme(Brightness.dark),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () =>
                Navigator.of(context)
                    .push(MaterialPageRoute<void>(builder: (_) => page)),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  final pages = <String, Future<void> Function(WidgetTester)>{
    'the Settings': (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openDeviceSettings(tester);
    },
    'File into…': (tester) =>
        push(tester, FileIntoScreen(places: places, current: null)),
    'Merge into…': (tester) =>
        push(tester, MergeIntoScreen(notes: notes, places: places)),
    'Add to Note…': (tester) => push(
      tester,
      MergeIntoScreen(notes: notes, places: places, adding: true),
    ),
    'Link to…': (tester) =>
        push(tester, LinkToScreen(notes: notes, places: places)),
  };

  final band = find.byType(SearchBand);
  final field = find.byType(TextField);

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

  /// The contrast of two colours, 1 for the same, as WCAG reckons it.
  double contrast(Color a, Color b) {
    final (light, dark) = a.computeLuminance() > b.computeLuminance()
        ? (a.computeLuminance(), b.computeLuminance())
        : (b.computeLuminance(), a.computeLuminance());
    return (light + 0.05) / (dark + 0.05);
  }

  for (final MapEntry(key: page, value: open) in pages.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('$page in the $brightness look: the search bar sits on a '
          'band of the top bar\'s colour, the list under its line', (
        tester,
      ) async {
        tester.platformDispatcher.platformBrightnessTestValue = brightness;
        addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
        await open(tester);

        expect(band, findsOneWidget);
        final box = find
            .descendant(of: band, matching: find.byType(DecoratedBox))
            .first;
        final decoration =
            tester.widget<DecoratedBox>(box).decoration as BoxDecoration;
        final colour = decoration.color;
        final ground = LookTones.of(Theme.of(tester.element(band)).brightness)
            .background;
        expect(Theme.of(tester.element(band)).brightness, brightness);
        expect(colour, isNotNull);
        expect(colour, barColour(tester));
        expect(colour, isNot(ground));
        // The tone of the question marks, not Material's (obf-1081).
        expect(colour, LookTones.of(brightness).mark);
        expect(colour?.a, 1.0);
        final border = decoration.border;
        expect(border, isA<Border>());
        if (border is! Border || colour == null) return;
        expect(border.top, BorderSide.none);
        expect(border.left, BorderSide.none);
        expect(border.right, BorderSide.none);
        expect(border.bottom.width, greaterThan(0));
        expect(contrast(border.bottom.color, colour), greaterThan(1.2));

        // The band holds the whole field, the line at its bottom, and the
        // list starts under it.
        final shown = tester.getRect(box);
        final fieldRect = tester.getRect(
          find.descendant(of: band, matching: field).first,
        );
        expect(shown.top, lessThan(fieldRect.top));
        expect(shown.bottom, greaterThan(fieldRect.bottom));
        // The page's own list, not one of the pages under it.
        final list = find
            .byWidgetPredicate(
              (w) => w is ScrollView || w is SingleChildScrollView,
            )
            .hitTestable()
            .first;
        expect(tester.getRect(list).top, greaterThanOrEqualTo(shown.bottom));
      });
    }
  }
}
