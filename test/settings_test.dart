import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/about_screen.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/setup_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'fake_alarms.dart';
import 'open_settings.dart';

void main() {
  late Store store;
  late FakeAlarms alarms;

  setUp(() {
    store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    alarms = FakeAlarms();
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    if (finder.evaluate().isEmpty) {
      await tester.scrollUntilVisible(
        finder,
        200,
        scrollable: find.byType(Scrollable).last,
      );
    }
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  group('the Profile picture', () {
    testWidgets('sits in the Agenda\'s top bar where the gear was and leads '
        'to Settings through the Profile page; no drawer, no ⋮ menu', (
      tester,
    ) async {
      await start(tester);
      expect(
        find.descendant(of: find.byType(AppBar), matching: profileButton),
        findsOneWidget,
      );
      expect(find.byTooltip('Profile'), findsOneWidget);
      expect(find.byIcon(Icons.settings_outlined), findsNothing);
      final agenda = find.descendant(
        of: find.byType(AgendaScreen),
        matching: find.byType(Scaffold),
      );
      expect(tester.widget<Scaffold>(agenda.first).drawer, isNull);
      expect(find.byIcon(Icons.more_vert), findsNothing);
      expect(find.byWidgetPredicate((w) => w is PopupMenuButton), findsNothing);

      await openDeviceSettings(tester);
      expect(find.byType(SettingsScreen), findsOneWidget);
      expect(
        find.text('These settings belong to this phone and are never synced.'),
        findsOneWidget,
      );
    });

    testWidgets('is the only way to Settings: the notes browser keeps just '
        'All Notes and the trash', (tester) async {
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      expect(find.byType(AreasScreen), findsOneWidget);
      final actions = find.descendant(
        of: find.byType(AppBar),
        matching: find.byType(IconButton),
      );
      expect(
        [for (final b in tester.widgetList<IconButton>(actions)) b.tooltip]
            .whereType<String>(),
        ['Back', 'All Notes', 'Trash'],
      );
      expect(profileButton, findsNothing);
    });
  });

  group('Settings', () {
    testWidgets('the theme changes at once', (tester) async {
      tester.platformDispatcher.platformBrightnessTestValue = Brightness.light;
      addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);
      await start(tester);
      await openDeviceSettings(tester);
      Brightness shown() =>
          Theme.of(tester.element(find.byType(SettingsScreen))).brightness;
      expect(shown(), Brightness.light);

      await tap(tester, find.text('Dark'));
      expect(store.themeChoice, ThemeChoice.dark);
      expect(shown(), Brightness.dark);

      await tap(tester, find.text('System'));
      expect(store.themeChoice, ThemeChoice.system);
      expect(shown(), Brightness.light);
    });

    testWidgets('are kept on this Device and never logged', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      await tap(tester, find.text('Light'));
      await tap(tester, find.byKey(const ValueKey('renagDevice')));
      await tap(tester, find.text('30 min').last);
      await tap(tester, find.byKey(const ValueKey('nudgeRepeat')));
      await tap(tester, find.text('10 min').last);
      await tap(tester, find.byKey(const ValueKey('quietHours')));
      await tap(tester, find.byKey(const ValueKey('streaks')));

      expect(store.themeChoice, ThemeChoice.light);
      expect(store.renagInterval, const Duration(minutes: 30));
      expect(store.nudgeRepeat, const Duration(minutes: 10));
      expect(store.quietHoursOn, isFalse);
      expect(store.streaksOn, isFalse);
      expect(store.allEntries(), isEmpty, reason: 'settings are never synced');
    });

    testWidgets('lead to the alarm setup, which comes back to them', (
      tester,
    ) async {
      await start(tester);
      await openDeviceSettings(tester);
      await tap(tester, find.text('Set up alarms'));
      expect(find.byType(SetupScreen), findsOneWidget);
      for (var step = 0; step < 2; step++) {
        await tap(tester, find.text('Next'));
      }
      await tap(tester, find.text('Done'));
      expect(find.byType(SettingsScreen), findsOneWidget);
    });

    testWidgets('offer no alarm setup where there are no alarms', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openDeviceSettings(tester);
      expect(find.text('Set up alarms'), findsNothing);
      expect(find.byKey(const ValueKey('renagDevice')), findsOneWidget);
    });

    testWidgets('and About fit a phone in German', (tester) async {
      tester.view.physicalSize = const Size(1080, 2340);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await start(tester);
      await openDeviceSettings(tester);
      expect(find.text('Wie das System'), findsOneWidget);
      await tester.ensureVisible(find.text('Über Old Brain Fix'));
      await tap(tester, find.text('Über Old Brain Fix'));
      expect(find.byType(AboutScreen), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('lead to About', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      await tap(tester, find.text('About Old Brain Fix'));
      expect(find.byType(AboutScreen), findsOneWidget);
    });
  });

  group('About', () {
    late List<Uri> opened;

    setUp(() => opened = []);

    Future<void> about(
      WidgetTester tester, {
      Locale? locale,
      Future<void> Function(Uri url)? open,
    }) async {
      await tester.pumpWidget(
        MaterialApp(
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: AboutScreen(
            version: Future.value(
              PackageInfo(
                appName: 'Old Brain Fix',
                packageName: 'oldbrainfix',
                version: '0.1.0',
                buildNumber: '1',
              ),
            ),
            open: open ?? (url) async => opened.add(url),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('shows the name, version and tagline', (tester) async {
      await about(tester);
      expect(find.text('Old Brain Fix'), findsWidgets);
      expect(find.text('Version 0.1.0 (1)'), findsOneWidget);
      expect(
        find.text(
          'Capture a thought in one tap. '
          'Get nagged until it is sorted or done.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('opens the docs, policy, source, issues, mail and Ko-fi', (
      tester,
    ) async {
      await about(tester);
      for (final title in [
        'Documentation',
        'Privacy policy',
        'Source code',
        'Report a problem or an idea',
        'Write to the developer',
        'Buy me a coffee',
      ]) {
        await tap(tester, find.text(title));
      }
      expect(opened.map((u) => u.toString()), [
        'https://paxel.github.io/oldbrainfix/en/',
        'https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html',
        'https://github.com/paxel/oldbrainfix',
        'https://github.com/paxel/oldbrainfix/issues',
        'mailto:taum@tuta.io?subject=Old%20Brain%20Fix%20feedback',
        'https://ko-fi.com/paxel7',
      ]);
      expect(
        find.text('https://github.com/paxel/oldbrainfix — Apache-2.0 / MIT'),
        findsOneWidget,
      );
    });

    testWidgets('links the German docs and privacy policy in German', (
      tester,
    ) async {
      await about(tester, locale: const Locale('de'));
      await tap(tester, find.text('Dokumentation'));
      await tap(tester, find.text('Datenschutzerklärung'));
      expect(opened.map((u) => u.toString()), [
        'https://paxel.github.io/oldbrainfix/de/',
        'https://paxel.github.io/oldbrainfix/de/reference/privacy-policy.html',
      ]);
    });

    testWidgets('a link nothing can open does nothing', (tester) async {
      await about(tester, open: (_) async => throw Exception('no app'));
      await tap(tester, find.text('Source code'));
      expect(tester.takeException(), isNull);
      expect(find.byType(AboutScreen), findsOneWidget);
    });

    testWidgets('opens the open-source licences', (tester) async {
      await about(tester);
      await tap(tester, find.text('Open-source licenses'));
      expect(find.byType(LicensePage), findsOneWidget);
    });

    test('the licences name everything that is not a Dart package', () async {
      registerLicenses();
      registerLicenses();
      final entries = await LicenseRegistry.licenses.toList();
      String of(String package) {
        final found = [
          for (final e in entries)
            if (e.packages.contains(package))
              e.paragraphs.map((p) => p.text).join('\n'),
        ];
        expect(found, hasLength(1), reason: package);
        return found.single;
      }

      final models = of('ocrs models');
      for (final tasl in [
        'Ocrs pre-trained models',
        'Robert Knight',
        'https://huggingface.co/robertknight/ocrs',
        'https://creativecommons.org/licenses/by-sa/4.0/',
        'bundled unmodified',
        'Attribution-ShareAlike 4.0 International',
      ]) {
        expect(models, contains(tasl));
      }
      expect(of('whisper.cpp'), contains('The ggml authors'));
      expect(
        of('OpenAI Whisper base model'),
        contains('Copyright (c) 2022 OpenAI'),
      );
      expect(of('Material Icons'), contains('Attribution 4.0 International'));
      expect(of('Rust standard library'), contains('MIT'));
      expect(of('androidx.work:work-runtime'), contains('Apache License'));
      expect(of('org.jetbrains.kotlin:kotlin-stdlib'), contains('JetBrains'));
      expect(of('com.google.zxing:core'), contains('ZXing authors'));
      // A crate by its name, whatever its version.
      String ofCrate(String name) => of(
        entries
            .expand((e) => e.packages)
            .firstWhere((p) => RegExp('^$name [0-9]').hasMatch(p)),
      );
      expect(ofCrate('ocrs'), contains('Ocrs project contributors'));
      expect(ofCrate('rten'), contains('RTen project contributors'));
    });

    testWidgets('the licence page lists them', (tester) async {
      registerLicenses();
      await about(tester);
      await tester.scrollUntilVisible(
        find.text('Open-source licenses'),
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.tap(find.text('Open-source licenses'));
      // The page reads the lists from the asset bundle, off the clock.
      for (
        var i = 0;
        i < 50 && find.text('androidx.activity:activity').evaluate().isEmpty;
        i++
      ) {
        await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 100)),
        );
        await tester.pump(const Duration(milliseconds: 100));
      }
      await tester.pumpAndSettle();
      for (final name in [
        'ocrs models',
        'OpenAI Whisper base model',
        'whisper.cpp',
        'com.google.zxing:core',
      ]) {
        tester
            .state<ScrollableState>(find.byType(Scrollable).last)
            .position
            .jumpTo(0);
        await tester.pump();
        await tester.scrollUntilVisible(
          find.text(name),
          200,
          scrollable: find.byType(Scrollable).last,
        );
        expect(find.text(name), findsOneWidget);
      }
    });
  });
}
