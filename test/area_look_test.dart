import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/area_look.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The WCAG relative luminance of an opaque colour, worked out here and
/// not taken from the app.
double luminance(Color colour) {
  double linear(double c) {
    final v = (c * 255).round() / 255;
    return v <= 0.04045
        ? v / 12.92
        : math.pow((v + 0.055) / 1.055, 2.4).toDouble();
  }

  return 0.2126 * linear(colour.r) +
      0.7152 * linear(colour.g) +
      0.0722 * linear(colour.b);
}

/// The WCAG contrast of two opaque colours.
double contrast(Color a, Color b) {
  final x = luminance(a);
  final y = luminance(b);
  return (math.max(x, y) + 0.05) / (math.min(x, y) + 0.05);
}

String hex(Color colour) => [colour.r, colour.g, colour.b]
    .map((c) => (c * 255).round().toRadixString(16).padLeft(2, '0'))
    .join()
    .toUpperCase();

/// An Area's colour and a Topic's icon (obf-1156).
void main() {
  group('the Area colours', () {
    test('eighteen, today\'s nine first and none twice', () {
      expect(areaPalette, hasLength(18));
      expect(areaPalette.take(9), [
        'C62828',
        'EF6C00',
        'F9A825',
        '2E7D32',
        '00838F',
        '1565C0',
        '6A1B9A',
        '8D6E63',
        '546E7A',
      ]);
      expect(areaPalette.toSet(), hasLength(18));
    });

    for (final brightness in Brightness.values) {
      test('each stands out 3:1 from every surface of the '
          '${brightness.name} look', () {
        final tones = LookTones.of(brightness);
        for (final stored in areaPalette) {
          final drawn = areaColour(stored, brightness);
          expect(drawn, isNotNull, reason: stored);
          if (drawn == null) continue;
          for (final surface in [tones.background, tones.mark, tones.card]) {
            expect(
              contrast(drawn, surface),
              greaterThanOrEqualTo(3.0),
              reason: '$stored as ${hex(drawn)} on $surface',
            );
          }
        }
        // Still eighteen colours to tell apart.
        expect({
          for (final stored in areaPalette) areaColour(stored, brightness),
        }, hasLength(18));
      });
    }

    test('a colour readable already is drawn as stored, the others shaded '
        'as the desktop shades them', () {
      Color? light(String stored) => areaColour(stored, Brightness.light);
      Color? dark(String stored) => areaColour(stored, Brightness.dark);
      expect(light('C62828'), const Color(0xFFC62828));
      expect(dark('F9A825'), const Color(0xFFF9A825));
      // The same values as the desktop's `theme` tests.
      expect(light('F9A825'), const Color(0xFFA26D18));
      expect(light('EF6C00'), const Color(0xFFBF5600));
      expect(dark('6A1B9A'), const Color(0xFFAD82C7));
      expect(dark('283593'), const Color(0xFF8990C4));
      // Not only the palette's: any stored colour.
      expect(dark('000000'), isNotNull);
      expect(light('nope'), isNull);
      expect(areaColour(null, Brightness.light), isNull);
    });
  });

  test('every icon key of the core has a Material icon', () {
    for (final key in areaIcons) {
      expect(areaIconData(key), isNotNull, reason: key);
    }
    expect(areaIconData('rocket'), isNull);
    expect(areaIconData(null), isNull);
  });

  group('a Topic\'s icon', () {
    late Store store;
    late Places places;
    late Area home;
    late Topic post;

    setUp(() {
      store = Store.inMemory()..starterSetOffered = true;
      places = Places(store);
      home = places.addArea('Home');
      places.setColour(home.id, '1565C0');
      post = places.addTopic(home.id, 'Post');
    });
    tearDown(() => store.close());

    Future<void> show(WidgetTester tester, Widget screen) async {
      tester.view.physicalSize = const Size(2400, 3600);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      final lock = AppLock(store: store);
      addTearDown(lock.dispose);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: LockScope(lock: lock, child: screen),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('is picked without a colour row, in its Area\'s colour', (
      tester,
    ) async {
      await show(tester, AreaLookScreen(places: places, placeId: post.id));
      expect(find.text('Post'), findsOneWidget);
      expect(find.text('Colour'), findsNothing);
      expect(find.text('Icon'), findsOneWidget);
      final mail = find.byIcon(Icons.mail);
      expect(
        tester.widget<Icon>(mail).color,
        areaColour('1565C0', Brightness.light),
      );
      await tester.tap(mail);
      await tester.pumpAndSettle();
      expect(places.topicById(post.id)?.icon, 'mail');
      expect(places.areaById(home.id)?.icon, isNull);
      await tester.tap(find.bySemanticsLabel('No icon'));
      await tester.pumpAndSettle();
      expect(places.topicById(post.id)?.icon, isNull);
    });

    testWidgets('an Area\'s page still has its colours', (tester) async {
      await show(tester, AreaLookScreen(places: places, placeId: home.id));
      expect(find.text('Colour'), findsOneWidget);
      await tester.tap(find.byIcon(Icons.mail));
      await tester.pumpAndSettle();
      expect(places.areaById(home.id)?.icon, 'mail');
      expect(places.topicById(post.id)?.icon, isNull);
    });

    testWidgets('shows in File into, a Topic without one as before', (
      tester,
    ) async {
      places.setTopicIcon(post.id, 'mail');
      places.addTopic(home.id, 'Bills');
      await show(tester, FileIntoScreen(places: places));
      final mail = find.byIcon(Icons.mail);
      expect(mail, findsOneWidget);
      expect(
        tester.widget<Icon>(mail).color,
        areaColour('1565C0', Brightness.light),
      );
    });
  });
}
