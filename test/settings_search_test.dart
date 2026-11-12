import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations_de.dart';
import 'package:oldbrainfix/l10n/app_localizations_en.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'open_settings.dart';

/// The search field on top of Settings (#282): it filters to the
/// matching settings, by their labels and detail lines in any case, with
/// their headings; a matching heading shows its section whole, and a
/// search that finds nothing says so.
void main() {
  late Store store;

  setUp(() {
    store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
  });
  tearDown(() => store.close());

  final en = AppLocalizationsEn();
  final field = find.descendant(
    of: find.byKey(const ValueKey('settingsSearch')),
    matching: find.byType(TextField),
  );
  final nothing = find.byKey(const ValueKey('settingsSearchNothing'));

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    await openDeviceSettings(tester);
  }

  Future<void> search(WidgetTester tester, String text) async {
    await tester.enterText(field, text);
    await tester.pumpAndSettle();
  }

  /// Whether [text] is on the page, scrolled out of view or not.
  Finder shown(String text) => find.text(text, skipOffstage: false);
  Finder keyed(String key) => find.byKey(ValueKey(key), skipOffstage: false);

  testWidgets('sits at the top, above the sections, and finds nothing '
      'while empty', (tester) async {
    await start(tester);

    expect(field, findsOneWidget);
    expect(
      tester.getRect(field).bottom,
      lessThan(tester.getRect(shown(en.settingsIntro)).top),
    );
    expect(
      tester.widget<TextField>(field).decoration?.hintText,
      'Search Settings',
    );
    for (final heading in [
      en.settingsLook,
      en.settingsAlarms,
      en.settingsNudges,
      en.settingsPrivacy,
      en.features,
    ]) {
      expect(shown(heading), findsOneWidget);
    }
    expect(nothing, findsNothing);
  });

  testWidgets('shows only the matching row under its heading, in any case', (
    tester,
  ) async {
    await start(tester);
    await search(tester, 'AREA locks');

    expect(shown(en.settingsPrivacy), findsOneWidget);
    expect(keyed('areaLocks'), findsOneWidget);
    expect(keyed('appLock'), findsNothing);
    expect(shown(en.relockAfter), findsNothing);
    for (final heading in [
      en.settingsLook,
      en.settingsAlarms,
      en.settingsNudges,
      en.settingsQuickNote,
      en.features,
    ]) {
      expect(shown(heading), findsNothing);
    }
    expect(shown(en.settingsIntro), findsNothing);
    expect(nothing, findsNothing);
  });

  testWidgets('finds a row by its detail line and by its options', (
    tester,
  ) async {
    await start(tester);
    // "Fat fingers" by its detail line alone.
    await search(tester, "system's font size");
    expect(shown(en.settingsLook), findsOneWidget);
    expect(keyed('fatFingers'), findsOneWidget);
    expect(keyed('theme'), findsNothing);

    await search(tester, en.themeDark.toLowerCase());
    expect(shown(en.settingsLook), findsOneWidget);
    expect(keyed('theme'), findsOneWidget);
    expect(keyed('fatFingers'), findsNothing);
    // No card is left without its rows.
    expect(
      find.descendant(
        of: find.byType(SettingsScreen),
        matching: find.byType(Embossed),
      ),
      findsNothing,
    );

    // "Default on this phone" by "ignored alarm rings again".
    await search(tester, 'ignored alarm');
    expect(shown(en.settingsAlarms), findsOneWidget);
    expect(shown(en.renagDevice), findsOneWidget);
    expect(keyed('allDayFrom'), findsNothing);
  });

  testWidgets('a matching heading shows its section whole', (tester) async {
    await start(tester);
    await search(tester, 'nudges');

    expect(shown(en.settingsNudges), findsOneWidget);
    expect(keyed('nudgeRepeat'), findsOneWidget);
    expect(keyed('quietHours'), findsOneWidget);
    expect(shown(en.nudgeSettingsExplain), findsOneWidget);
    expect(shown(en.settingsAlarms), findsNothing);
  });

  testWidgets('a row this Device does not show finds nothing, and says so', (
    tester,
  ) async {
    await start(tester);
    // Without a PIN there is no Change PIN, nor biometrics, whose detail
    // line this is.
    expect(en.biometricsDetail, 'The PIN always works as well.');
    await search(tester, 'always works');

    expect(shown(en.settingsPrivacy), findsNothing);
    expect(nothing, findsOneWidget);
    expect(
      tester.widget<Text>(nothing).data,
      'No setting matches “always works”.',
    );

    // With a PIN, Change PIN is the only row under Privacy.
    store.setPin('2468', iterations: 1);
    // Back to the Profile page, and Settings again.
    await tester.pageBack();
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('deviceSettings')));
    await tester.pumpAndSettle();
    await search(tester, 'change pin');
    expect(shown(en.settingsPrivacy), findsOneWidget);
    expect(keyed('changePin'), findsOneWidget);
    expect(keyed('appLock'), findsNothing);
    expect(nothing, findsNothing);
  });

  testWidgets('Quiet Hours\' times are found with them, once switched on', (
    tester,
  ) async {
    store.quietHoursOn = true;
    await start(tester);
    await search(tester, 'quiet hours');

    expect(keyed('quietHours'), findsOneWidget);
    expect(keyed('quietFrom'), findsOneWidget);
    expect(keyed('quietUntil'), findsOneWidget);
    expect(keyed('nudgeRepeat'), findsNothing);
  });

  testWidgets('finds nothing, says so, and shows everything again once '
      'cleared', (tester) async {
    await start(tester);
    await search(tester, 'xyzzy');

    expect(nothing, findsOneWidget);
    expect(shown(en.settingsLook), findsNothing);
    expect(shown(en.aboutTitle), findsNothing);

    await tester.tap(find.byTooltip(en.clearSearch));
    await tester.pumpAndSettle();
    expect(tester.widget<TextField>(field).controller?.text, isEmpty);
    expect(nothing, findsNothing);
    expect(shown(en.settingsIntro), findsOneWidget);
    expect(shown(en.settingsLook), findsOneWidget);
    expect(keyed('fatFingers'), findsOneWidget);
    expect(shown(en.aboutTitle), findsOneWidget);
  });

  testWidgets('speaks German and finds the German words', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final de = AppLocalizationsDe();
    await start(tester);

    expect(
      tester.widget<TextField>(field).decoration?.hintText,
      de.settingsSearchHint,
    );
    await search(tester, 'ruhezeit');
    expect(shown(de.settingsNudges), findsOneWidget);
    expect(keyed('quietHours'), findsOneWidget);
    expect(keyed('nudgeRepeat'), findsNothing);
    // From when all-day items ring, whose detail line names it.
    expect(keyed('allDayFrom'), findsOneWidget);
    expect(keyed('renagDevice'), findsNothing);

    await search(tester, 'dunkel');
    expect(shown(de.settingsLook), findsOneWidget);
    expect(keyed('theme'), findsOneWidget);

    await search(tester, 'Wecker klingelt');
    expect(nothing, findsOneWidget);
    expect(
      tester.widget<Text>(nothing).data,
      'Keine Einstellung passt zu „Wecker klingelt“.',
    );
  });
}
