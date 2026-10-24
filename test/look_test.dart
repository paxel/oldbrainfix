import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/question_mark_background.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  Store openStore() {
    final store = Store.inMemory()..starterSetOffered = true;
    addTearDown(store.close);
    return store;
  }

  void systemBrightness(WidgetTester tester, Brightness brightness) {
    tester.platformDispatcher.platformBrightnessTestValue = brightness;
    addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
  }

  Brightness shownBrightness(WidgetTester tester) =>
      Theme.of(tester.element(find.byType(AgendaScreen))).brightness;

  Finder background() => find.byWidgetPredicate(
    (w) => w is CustomPaint && w.painter is QuestionMarkPainter,
  );

  QuestionMarkPainter painter(WidgetTester tester) =>
      tester.widget<CustomPaint>(background()).painter as QuestionMarkPainter;

  group('theme', () {
    for (final brightness in Brightness.values) {
      testWidgets('follows the system when it is $brightness', (tester) async {
        systemBrightness(tester, brightness);
        await tester.pumpWidget(OldBrainFixApp(store: openStore()));
        expect(shownBrightness(tester), brightness);
        expect(painter(tester).tones, LookTones.of(brightness));
      });
    }

    testWidgets('a Device override to light beats a dark system', (
      tester,
    ) async {
      systemBrightness(tester, Brightness.dark);
      final store = openStore()..themeChoice = ThemeChoice.light;
      await tester.pumpWidget(OldBrainFixApp(store: store));
      expect(shownBrightness(tester), Brightness.light);
    });

    testWidgets('a Device override to dark beats a light system', (
      tester,
    ) async {
      systemBrightness(tester, Brightness.light);
      final store = openStore()..themeChoice = ThemeChoice.dark;
      await tester.pumpWidget(OldBrainFixApp(store: store));
      expect(shownBrightness(tester), Brightness.dark);
    });

    testWidgets('the accent is teal in both', (tester) async {
      for (final brightness in Brightness.values) {
        final accent = lookTheme(brightness).colorScheme.primary;
        final hue = HSLColor.fromColor(accent).hue;
        expect(hue, inInclusiveRange(165, 185), reason: '$brightness');
      }
    });
  });

  group('colours', () {
    test('light mode: a light teal ground, marks a touch darker', () {
      const t = LookTones.light;
      expect(HSLColor.fromColor(t.background).hue, inInclusiveRange(160, 190));
      expect(t.background.computeLuminance(), greaterThan(0.6));
      expect(
        t.mark.computeLuminance(),
        lessThan(t.background.computeLuminance()),
      );
      expect(contrast(t.mark, t.background), lessThan(1.2));
    });

    test('dark mode: an anthracite ground, marks a touch brighter', () {
      const t = LookTones.dark;
      expect(t.background.computeLuminance(), lessThan(0.05));
      expect(
        t.mark.computeLuminance(),
        greaterThan(t.background.computeLuminance()),
      );
      expect(contrast(t.mark, t.background), lessThan(1.2));
    });

    test('cards stand out from the ground and the marks (ratio >= 1.2)', () {
      for (final t in [LookTones.light, LookTones.dark]) {
        expect(contrast(t.card, t.background), greaterThanOrEqualTo(1.2));
        expect(contrast(t.card, t.mark), greaterThanOrEqualTo(1.2));
      }
    });
  });

  testWidgets('the + and the announcement strip stand out (ratio >= 3)', (
    tester,
  ) async {
    for (final brightness in Brightness.values) {
      final tones = LookTones.of(brightness);
      final theme = lookTheme(brightness);
      final fills = {
        'fab': theme.floatingActionButtonTheme.backgroundColor,
        'strip': theme.colorScheme.primary,
      };
      for (final MapEntry(:key, :value) in fills.entries) {
        expect(value, isNotNull, reason: key);
        if (value == null) continue;
        expect(
          contrast(value, tones.background),
          greaterThanOrEqualTo(3),
          reason: '$key on the $brightness ground',
        );
        expect(
          contrast(value, tones.mark),
          greaterThanOrEqualTo(3),
          reason: '$key on the $brightness marks',
        );
      }
    }
  });

  testWidgets('the background paints the ground and question marks', (
    tester,
  ) async {
    systemBrightness(tester, Brightness.light);
    await tester.pumpWidget(OldBrainFixApp(store: openStore()));
    expect(
      tester.renderObject(background()),
      paints
        ..rect(color: LookTones.light.background)
        ..path(color: LookTones.light.mark),
    );
  });

  testWidgets('a Note is an embossed card, not a list tile', (tester) async {
    systemBrightness(tester, Brightness.light);
    final store = openStore();
    Notes(store).capture('nees mor sauce');
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();

    expect(find.byType(ListTile), findsNothing);
    final card = find.ancestor(
      of: find.text('nees mor sauce'),
      matching: find.byType(NoteCard),
    );
    expect(card, findsOneWidget);
    final box = tester.widget<DecoratedBox>(
      find.descendant(of: card, matching: find.byType(DecoratedBox)).first,
    );
    final decoration = box.decoration as BoxDecoration;
    expect(decoration.color, LookTones.light.card);
    expect(decoration.boxShadow, hasLength(2));
  });

  group('parallax', () {
    /// Scrolls an Agenda long enough to scroll; returns by how much.
    Future<double> scrollNotes(WidgetTester tester) async {
      final store = openStore();
      final routines = Routines(store);
      for (var i = 0; i < 40; i++) {
        routines.create(
          'thought $i',
          Schedule(
            start: Day(2000, 1, 1),
            rule: Recurrence.parse('FREQ=DAILY'),
          ),
        );
      }
      await tester.pumpWidget(OldBrainFixApp(store: store));
      expect(painter(tester).shift, 0);
      await tester.drag(find.byType(ListView), const Offset(0, -600));
      await tester.pumpAndSettle();
      return tester
          .state<ScrollableState>(find.byType(Scrollable))
          .position
          .pixels;
    }

    testWidgets('the marks move slower than the page', (tester) async {
      final scrolled = await scrollNotes(tester);
      expect(scrolled, greaterThan(0));
      expect(painter(tester).shift, closeTo(scrolled * parallaxFactor, 0.01));
      expect(painter(tester).shift, lessThan(scrolled));
    });

    testWidgets('stops when the system removes animations', (tester) async {
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(disableAnimations: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      final scrolled = await scrollNotes(tester);
      expect(scrolled, greaterThan(0));
      expect(painter(tester).shift, 0);
    });
  });
}

/// WCAG contrast ratio of two colours.
double contrast(Color a, Color b) {
  final la = a.computeLuminance(), lb = b.computeLuminance();
  return (la > lb ? la + 0.05 : lb + 0.05) / (la > lb ? lb + 0.05 : la + 0.05);
}
