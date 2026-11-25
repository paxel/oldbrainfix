import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/about_screen.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/settings_search.dart';
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

  group('Simple or Advanced', () {
    testWidgets('reads Advanced where nothing was chosen; Simple is kept for '
        'the person', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      final choice = find.byKey(const ValueKey('features'));
      expect(tester.widget<RadioColumn<bool>>(choice).value, isTrue);
      // The quick note is the same in both (#352): no switch of its own.
      expect(
        find.byKey(const ValueKey('quickNote-formatBar'), skipOffstage: false),
        findsNothing,
      );
      await tap(tester, find.byKey(const ValueKey('features-simple')));
      expect(store.advanced, isFalse);
      expect(tester.widget<RadioColumn<bool>>(choice).value, isFalse);
      expect(
        find.byKey(const ValueKey('quickNote-formatBar'), skipOffstage: false),
        findsNothing,
      );
      await tap(tester, find.byKey(const ValueKey('features-advanced')));
      expect(store.advanced, isTrue);
    });
  });

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
      // ⋮ holds Running alone (obf-1078), never Settings.
      await tap(tester, find.byKey(const ValueKey('bar-more')));
      expect(find.byType(PopupMenuItem<VoidCallback>), findsOneWidget);
      expect(find.text('Running'), findsOneWidget);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await openDeviceSettings(tester);
      expect(find.byType(SettingsScreen), findsOneWidget);
      expect(
        find.text(
          'Simple or Advanced and the Trash setting sync to your own Devices; '
          'the other settings belong to this phone.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('is the only way to Settings: the notes browser keeps just '
        'All Notes, Frogs, the trash and ⋮ without Settings', (tester) async {
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
        ['Back', 'All Notes', 'Frogs', 'Trash', 'More'],
      );
      // The Trash opens with a bin and an arrow up, not the bin that
      // deletes (#166).
      final trash = tester.widget<IconButton>(
        find.widgetWithIcon(IconButton, Icons.restore_from_trash),
      );
      expect(trash.tooltip, 'Trash');
      expect(
        find.descendant(
          of: find.byType(AppBar),
          matching: find.byIcon(Icons.delete_outline),
        ),
        findsNothing,
      );
      expect(profileButton, findsNothing);
      // ⋮ stands with one Area too (obf-1096), Settings never in it.
      await tap(tester, find.byKey(const ValueKey('areas-more')));
      expect(find.text('Settings'), findsNothing);
      expect(find.text('Running'), findsOneWidget);
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

    testWidgets('show units as the region does until metric or imperial is '
        'picked as a radio button', (tester) async {
      tester.platformDispatcher.localeTestValue = const Locale('en', 'US');
      addTearDown(tester.platformDispatcher.clearLocaleTestValue);
      await start(tester);
      await openDeviceSettings(tester);
      final column = find.byKey(const ValueKey('unitSystem'));
      await tester.scrollUntilVisible(
        column,
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.pumpAndSettle();
      expect(
        find.descendant(
          of: column,
          matching: find.byWidgetPredicate((w) => w is RadioListTile),
        ),
        findsNWidgets(3),
      );
      expect(find.text('As the phone\'s region: imperial'), findsOneWidget);
      expect(store.unitSystem('US'), UnitSystem.imperial);

      await tap(tester, find.byKey(const ValueKey('unitSystem-metric')));
      expect(store.chosenUnitSystem, UnitSystem.metric);
      expect(store.unitSystem('US'), UnitSystem.metric);
      expect(store.allEntries(), isEmpty, reason: 'settings are never synced');

      await tap(tester, find.byKey(const ValueKey('unitSystem-region')));
      expect(store.chosenUnitSystem, isNull);
    });

    testWidgets('have no choice of how the Agenda shows Shopping Lists, '
        'each list has its own (#186)', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      // The section stood between the units and the alarms.
      await tester.scrollUntilVisible(
        find.text('Alarms'),
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('unitSystem-metric')), findsOneWidget);
      expect(find.text('Shopping Lists on the Agenda'), findsNothing);
      expect(find.byKey(const ValueKey('shoppingAgenda')), findsNothing);
    });

    testWidgets('are kept on this Device and never logged', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      await tap(tester, find.text('Light'));
      await tap(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('renagDevice')),
          matching: find.text('30 min'),
        ),
      );
      await tap(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('nudgeRepeat')),
          matching: find.text('10 min'),
        ),
      );
      await tap(tester, find.byKey(const ValueKey('quietHours')));
      await tap(tester, find.byKey(const ValueKey('streaks')));
      expect(store.twoRowEditorOn, isTrue);
      await tap(tester, find.byKey(const ValueKey('twoRowEditor')));

      expect(store.themeChoice, ThemeChoice.light);
      expect(store.renagInterval, const Duration(minutes: 30));
      expect(store.nudgeRepeat, const Duration(minutes: 10));
      expect(store.quietHoursOn, isFalse);
      expect(store.streaksOn, isFalse);
      expect(store.twoRowEditorOn, isFalse);
      expect(store.allEntries(), isEmpty, reason: 'settings are never synced');
    });

    testWidgets('strike through what is checked on the Agenda until it is '
        'hidden, which the Agenda follows on the way back (#217)', (
      tester,
    ) async {
      final routines = Routines(store);
      final today = Day.today(DateTime.now(), deviceWallClock);
      final pills = routines.create('pills', Schedule(start: today));
      routines.markDone(pills.id, Occurrence(today));
      await start(tester);
      expect(find.text('pills'), findsOneWidget);
      await openDeviceSettings(tester);
      expect(find.text('Agenda'), findsOneWidget);
      final hide = find.byKey(const ValueKey('agendaHideChecked'));
      if (hide.evaluate().isEmpty) {
        await tester.scrollUntilVisible(
          hide,
          200,
          scrollable: find.byType(Scrollable).last,
        );
      }
      expect(tester.widget<Switch>(hide).value, isFalse);
      final logged = store.allEntries().length;
      await tap(tester, hide);
      expect(store.agendaHidesChecked, isTrue);
      expect(store.allEntries(), hasLength(logged), reason: 'never logged');
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.text('pills'), findsNothing);
    });

    testWidgets('are radio columns that keep a value outside the choices', (
      tester,
    ) async {
      store
        ..renagInterval = const Duration(minutes: 20)
        ..nudgeRepeat = const Duration(minutes: 45);
      await start(tester);
      await openDeviceSettings(tester);
      for (final (key, count, chosen) in [
        ('renagDevice', 6, 20),
        ('nudgeRepeat', 4, 45),
      ]) {
        final column = find.byKey(ValueKey(key));
        await tester.scrollUntilVisible(
          column,
          200,
          scrollable: find.byType(Scrollable).last,
        );
        await tester.pumpAndSettle();
        expect(
          find.descendant(of: column, matching: find.byType(ChoiceChip)),
          findsNWidgets(count),
          reason: key,
        );
        expect(
          find.descendant(of: column, matching: find.text('$chosen min')),
          findsOneWidget,
          reason: key,
        );
        expect(
          tester.widget<ChoiceChips<int>>(column).value,
          chosen,
          reason: key,
        );
      }
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

    testWidgets('ring all-day items from 15:00 until another time is picked, '
        'which is kept on this Device and armed at once (#201)', (
      tester,
    ) async {
      final note = Notes(store).capture('water the plants');
      Routines(store).spawn(
        note.id,
        Schedule(
          start: Day.today(store.clock(), (utc) => utc.toLocal()),
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      await start(tester);
      List<ClockTime?> allDayTimes() => [
        for (final a in alarms.armed)
          if (a.alarm.occurrence.time == null) a.alarm.wall?.time,
      ];
      expect(allDayTimes(), isNotEmpty);
      expect(allDayTimes().toSet(), {ClockTime(15, 0)});

      await openDeviceSettings(tester);
      final tile = find.byKey(const ValueKey('allDayFrom'));
      await tap(tester, tile);
      expect(find.byType(TimePickerDialog), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(store.localSetting('allDayFrom'), isNull, reason: 'backed out');
      expect(
        find.descendant(of: tile, matching: find.text('15:00')),
        findsOneWidget,
      );

      await tap(tester, tile);
      await tester.tap(
        find.descendant(
          of: find.byType(TimePickerDialog),
          matching: find.byIcon(Icons.keyboard_outlined),
        ),
      );
      await tester.pumpAndSettle();
      final fields = find.descendant(
        of: find.byType(TimePickerDialog),
        matching: find.byType(TextField),
      );
      await tester.enterText(fields.at(0), '4');
      await tester.enterText(fields.at(1), '30');
      await tester.tap(find.text('OK'));
      await tester.pumpAndSettle();

      expect(store.allDayFrom, ClockTime(16, 30));
      expect(
        find.descendant(of: tile, matching: find.text('16:30')),
        findsOneWidget,
      );
      expect(allDayTimes().toSet(), {ClockTime(16, 30)});
      expect(store.allEntries().where((e) => e.field == 'allDayFrom'), isEmpty);
    });

    testWidgets('offer no alarm setup where there are no alarms', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await openDeviceSettings(tester);
      expect(find.text('Set up alarms'), findsNothing);
      expect(find.byKey(const ValueKey('alarmSound')), findsNothing);
      expect(find.byKey(const ValueKey('renagDevice')), findsOneWidget);
    });

    group('the alarm sound', () {
      const bell = 'content://media/internal/audio/media/42';
      final tile = find.byKey(const ValueKey('alarmSound'));

      testWidgets('is no choice on the iPhone, which has no sound picker', (
        tester,
      ) async {
        await start(tester);
        alarms.choosesSound = false;
        await openDeviceSettings(tester);
        expect(tile, findsNothing);
        expect(find.byKey(const ValueKey('renagDevice')), findsOneWidget);
      });

      testWidgets('is the app\'s own until another is picked, which is kept '
          'on this Device and armed', (tester) async {
        await start(tester);
        expect(alarms.sound, isNull, reason: 'the own sound');
        await openDeviceSettings(tester);
        expect(
          find.descendant(
            of: tile,
            matching: find.textContaining('own chime. Tap'),
          ),
          findsOneWidget,
        );

        alarms.soundPicked = (sound: bell);
        await tap(tester, tile);
        expect(alarms.pickedFrom, [null]);
        expect(store.alarmSound, bell);
        expect(alarms.sound, bell);
        expect(
          find.descendant(
            of: tile,
            matching: find.textContaining('A sound of the phone'),
          ),
          findsOneWidget,
        );
        expect(store.allEntries(), isEmpty, reason: 'never synced');
      });

      testWidgets('goes back to the app\'s own with the picker\'s default', (
        tester,
      ) async {
        store.alarmSound = bell;
        await start(tester);
        expect(alarms.sound, bell);
        await openDeviceSettings(tester);

        alarms.soundPicked = (sound: null);
        await tap(tester, tile);
        expect(alarms.pickedFrom, [bell]);
        expect(store.alarmSound, isNull);
        expect(alarms.sound, isNull);
      });

      testWidgets('stays as it was when the picker is left', (tester) async {
        store.alarmSound = bell;
        await start(tester);
        await openDeviceSettings(tester);
        final arms = alarms.arms;

        alarms.soundPicked = null;
        await tap(tester, tile);
        expect(alarms.pickedFrom, [bell]);
        expect(store.alarmSound, bell);
        expect(alarms.sound, bell);
        expect(alarms.arms, arms, reason: 'nothing to arm anew');
      });
    });

    testWidgets('draw every section as one card with its title on top', (
      tester,
    ) async {
      await start(tester);
      await openDeviceSettings(tester);
      Finder under(Finder of, Type type) => find.descendant(
        of: of,
        matching: find.byType(type, skipOffstage: false),
      );
      final page = find.byType(SettingsScreen);
      final rows = under(page, SettingsRow);
      expect(rows, findsWidgets);
      for (final row in rows.evaluate()) {
        expect(
          find.ancestor(
            of: find.byWidget(row.widget, skipOffstage: false),
            matching: find.byType(SettingsSection, skipOffstage: false),
          ),
          findsOneWidget,
          reason: '${row.widget.key ?? row.widget} sits on a card',
        );
      }
      final groups = under(page, SettingsGroup);
      expect(groups, findsAtLeastNWidgets(10));
      for (var i = 0; i < groups.evaluate().length; i++) {
        final group = groups.at(i);
        final heading = (tester.widget(group) as SettingsGroup).heading;
        final card = under(group, SettingsSection);
        expect(card, findsOneWidget, reason: '$heading is one card');
        if (heading == null) continue;
        final title = under(group, SettingsHeading);
        expect(title, findsOneWidget);
        expect(
          tester.getRect(title).bottom,
          lessThanOrEqualTo(tester.getRect(card).top),
          reason: '$heading is the title on top of its card',
        );
      }
      // The theme's chips start at the card's start, not in its middle.
      final theme = find.byKey(const ValueKey('theme'));
      expect(
        tester.getTopLeft(theme).dx,
        tester
                .getTopLeft(
                  find.ancestor(of: theme, matching: find.byType(Embossed)),
                )
                .dx +
            16,
      );
    });

    testWidgets('fit a phone in German', (tester) async {
      tester.view.physicalSize = const Size(1080, 2340);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await start(tester);
      await openDeviceSettings(tester);
      expect(find.text('Wie das System'), findsOneWidget);
      await tester.ensureVisible(find.text('Feedback senden'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });

    for (final (locale, labels) in [
      (
        const Locale('en'),
        {
          'renagDevice': 'Default on this phone',
          'nudgeRepeat': 'Nudge again after',
          'quietHours': 'Quiet Hours',
          'streaks': 'Adherence shows either way.',
        },
      ),
      (
        const Locale('de'),
        {
          'renagDevice': 'Standard auf diesem Handy',
          'nudgeRepeat': 'Erneut stupsen nach',
          'quietHours': 'Ruhezeit',
          'streaks': 'Die Einhaltung wird immer gezeigt.',
        },
      ),
    ]) {
      testWidgets('put each switch and single choice on its own line below its '
          'text (${locale.languageCode})', (tester) async {
        tester.platformDispatcher.localesTestValue = [locale];
        addTearDown(tester.platformDispatcher.clearLocalesTestValue);
        await start(tester);
        await openDeviceSettings(tester);
        for (final MapEntry(key: key, value: label) in labels.entries) {
          final control = find.byKey(ValueKey(key));
          await tester.scrollUntilVisible(
            control,
            200,
            scrollable: find.byType(Scrollable).last,
          );
          await tester.pumpAndSettle();
          final text = find.text(label);
          expect(
            tester.getTopLeft(control).dy,
            greaterThanOrEqualTo(tester.getBottomLeft(text).dy),
            reason: '$key sits below "$label"',
          );
          expect(
            tester.getTopLeft(control).dx,
            closeTo(tester.getTopLeft(text).dx, 0.5),
            reason: '$key starts where its text starts',
          );
        }
      });
    }

    testWidgets('name a switch by its text', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      final streaks = find.byKey(const ValueKey('streaks'));
      await tester.scrollUntilVisible(
        streaks,
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.pumpAndSettle();
      expect(
        tester.getSemantics(streaks),
        isSemantics(
          label: 'Show Streaks on this phone\nAdherence shows either way.',
          hasToggledState: true,
          isToggled: true,
        ),
      );
    });

    testWidgets('end with Send feedback; About is behind the Profile '
        'picture', (tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      await tester.scrollUntilVisible(
        find.text('Send feedback'),
        200,
        scrollable: find.byType(Scrollable).last,
      );
      await tester.pumpAndSettle();
      expect(
        find.descendant(
          of: find.byType(SettingsScreen),
          matching: find.text('About Old Brain Fix', skipOffstage: false),
        ),
        findsNothing,
      );
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

    testWidgets('offers no Ko-fi on the iPhone (Q591)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      try {
        await about(tester);
        // The row would sit right under this one: scroll it into view so
        // a Ko-fi row would be built if there were one.
        await tester.scrollUntilVisible(
          find.text('Write to the developer'),
          200,
          scrollable: find.byType(Scrollable).first,
        );
        await tester.drag(find.byType(Scrollable).first, const Offset(0, -400));
        await tester.pumpAndSettle();
        expect(find.text('Write to the developer'), findsOneWidget);
        expect(find.text('Buy me a coffee', skipOffstage: false), findsNothing);
      } finally {
        debugDefaultTargetPlatformOverride = null;
      }
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

    for (final (locale, button, thanks, done) in [
      (
        const Locale('en'),
        "DON'T PRESS.\nDANGER",
        'Thank you for using Old Brain Fix!',
        'Done',
      ),
      (
        const Locale('de'),
        'NICHT DRÜCKEN.\nGEFAHR',
        'Danke, dass du Old Brain Fix benutzt!',
        'Fertig',
      ),
    ]) {
      testWidgets('the red button thanks the user and Done closes the thanks '
          '(${locale.languageCode})', (tester) async {
        await about(tester, locale: locale);
        await tap(tester, find.text(button));
        expect(find.byType(AlertDialog), findsOneWidget);
        expect(find.text('\u{1F917}'), findsOneWidget);
        expect(find.text(thanks), findsOneWidget);
        await tester.tap(find.text(done));
        await tester.pumpAndSettle();
        expect(find.byType(AlertDialog), findsNothing);
        expect(find.text(thanks), findsNothing);
        expect(find.byType(AboutScreen), findsOneWidget);
      });
    }

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

      final models = of('PP-OCRv6 Tiny models');
      for (final tasl in [
        'PP-OCRv6 Tiny text detection and text recognition models',
        'PaddlePaddle Authors',
        'https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_det',
        'https://www.apache.org/licenses/LICENSE-2.0',
        "the only change to PaddlePaddle's models is RustO's conversion",
        'Copyright (c) 2016 PaddlePaddle Authors',
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
      expect(ofCrate('rusto-rs'), contains('RustO Contributors'));
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
        'PP-OCRv6 Tiny models',
        'OpenAI Whisper base model',
        'whisper.cpp',
        'com.google.zxing:core',
      ]) {
        tester
            .state<ScrollableState>(find.byType(Scrollable).last)
            .position
            .jumpTo(0);
        await tester.pump();
        // The OCR library's crates make the list long.
        await tester.scrollUntilVisible(
          find.text(name),
          200,
          scrollable: find.byType(Scrollable).last,
          maxScrolls: 200,
        );
        expect(find.text(name), findsOneWidget);
      }
    });
  });
}
