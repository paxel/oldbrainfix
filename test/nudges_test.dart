import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/missed_summary_card.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'open_settings.dart';
import 'open_capture.dart';

void main() {
  late Store store;
  late Notes notes;
  late FakeAlarms alarms;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    alarms = FakeAlarms();
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    // Tall enough for five cards with their action rows.
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Captures [count] Unsorted Notes a minute apart, "note 1" the oldest.
  void captureMany(int count) {
    final saved = now;
    for (var i = 1; i <= count; i++) {
      now = saved.subtract(Duration(minutes: count - i + 1));
      notes.capture('note $i');
    }
    now = saved;
  }

  /// The texts of the Note cards shown, top to bottom.
  List<String> cards(WidgetTester tester) => [
    for (final card in tester.widgetList<NoteCard>(find.byType(NoteCard)))
      card.note.text,
  ];

  group('capture', () {
    testWidgets('shows "remind me in" 10 min, 30 min and 1 h at the bottom, '
        'not as a bar', (tester) async {
      await start(tester);
      await openTextCapture(tester);

      expect(find.text('Remind me to sort it in'), findsOneWidget);
      for (final key in ['nudge10', 'nudge30', 'nudge60']) {
        expect(find.byKey(ValueKey(key)), findsOneWidget);
      }
      expect(find.text('10 min'), findsOneWidget);
      expect(find.text('30 min'), findsOneWidget);
      expect(find.text('1 h'), findsOneWidget);
      final screen = tester.getRect(find.byType(CaptureScreen));
      final buttons = tester.getRect(find.byKey(const ValueKey('nudge10')));
      expect(buttons.top, greaterThan(screen.center.dy));
      expect(
        tester.getRect(find.byType(TextField)).bottom,
        lessThanOrEqualTo(buttons.top),
      );
      final scaffold = tester.widget<Scaffold>(
        find.descendant(
          of: find.byType(CaptureScreen),
          matching: find.byType(Scaffold),
        ),
      );
      expect(scaffold.bottomNavigationBar, isNull);
      expect(scaffold.bottomSheet, isNull);
      expect(scaffold.persistentFooterButtons, isNull);
    });

    testWidgets('a tapped choice saves the Note and arms the Nudge for it', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'nees mor sauce');
      await tap(tester, find.byKey(const ValueKey('nudge10')));

      final note = notes.all().single;
      expect(note.text, 'nees mor sauce');
      expect(note.nudge, now.toUtc().add(const Duration(minutes: 10)));
      expect(find.byType(CaptureScreen), findsNothing);
      expect(
        alarms.nudge?.nudge.at,
        now.toUtc().add(const Duration(minutes: 10)),
      );
      expect(alarms.nudge?.title, '1 Note is Unsorted');
      expect(alarms.nudge?.body, 'Tap to sort them.');
      expect(alarms.labels?.nudgeChannel, 'Nudges');
    });

    testWidgets('Save without a choice means 1 h', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'call the plumber');
      await tap(tester, find.text('Save'));

      expect(
        notes.all().single.nudgeAt,
        now.toUtc().add(const Duration(hours: 1)),
      );
      expect(alarms.nudge?.nudge.at, now.toUtc().add(const Duration(hours: 1)));
    });

    testWidgets('shows the choices when capturing into an Area too; the '
        'filed Note is not nudged', (tester) async {
      final area = Places(store).addArea('Household');
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          home: Navigator(
            onGenerateRoute: (_) => MaterialPageRoute<void>(
              builder: (_) => CaptureScreen(notes: notes, place: area.id),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('nudge10')), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'descale the kettle');
      await tap(tester, find.byKey(const ValueKey('nudge30')));
      expect(notes.all().single.place, area.id);
      expect(notes.unsorted(), isEmpty);
      expect(Alarms(store).nudge(deviceWallClock), isNull);
    });
  });

  group('leaving capture', () {
    Future<void> background(WidgetTester tester, {bool back = true}) async {
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
        if (back) ...[
          AppLifecycleState.hidden,
          AppLifecycleState.inactive,
          AppLifecycleState.resumed,
        ],
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
    }

    testWidgets('by Back saves the Note, nudged in 1 h', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'nees mor sauce');
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(find.byType(CaptureScreen), findsNothing);
      final note = notes.all().single;
      expect(note.text, 'nees mor sauce');
      expect(note.nudgeAt, now.toUtc().add(const Duration(hours: 1)));
      expect(find.text('nees mor sauce'), findsOneWidget);
    });

    testWidgets('by Back with nothing typed captures nothing', (tester) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(CaptureScreen), findsNothing);
      expect(notes.all(), isEmpty);
    });

    testWidgets('to the background saves once; typing on changes that Note', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await background(tester);
      expect(notes.all(), isEmpty, reason: 'nothing typed yet');

      await tester.enterText(find.byType(TextField), 'nees mor');
      await background(tester);
      expect(notes.all().single.text, 'nees mor');
      expect(find.byType(CaptureScreen), findsOneWidget);
      await background(tester);
      expect(notes.all(), hasLength(1));

      await tester.enterText(find.byType(TextField), 'nees mor sauce');
      await tap(tester, find.byKey(const ValueKey('nudge10')));
      final note = notes.all().single;
      expect(note.text, 'nees mor sauce');
      expect(note.nudgeAt, now.toUtc().add(const Duration(minutes: 10)));
    });
  });

  group('the "x Unsorted" banner', () {
    testWidgets('sits under the Missed Summary and opens all Unsorted Notes, '
        'oldest first', (tester) async {
      final saved = now;
      now = DateTime(2026, 10, 22, 6);
      Routines(store).create(
        'pills',
        Schedule(
          start: Day(2026, 10, 22),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      now = saved;
      captureMany(5);
      await start(tester);

      final banner = find.byKey(const ValueKey('unsortedBanner'));
      expect(find.text('5 Unsorted'), findsOneWidget);
      expect(
        tester.getRect(banner).top,
        greaterThanOrEqualTo(
          tester.getRect(find.byType(MissedSummaryCard)).bottom,
        ),
      );
      expect(
        tester.getRect(banner).bottom,
        lessThan(tester.getRect(find.text('Today')).top),
      );

      await tap(tester, banner);
      expect(find.byType(UnsortedScreen), findsOneWidget);
      expect(cards(tester), ['note 1', 'note 2', 'note 3', 'note 4', 'note 5']);
    });

    testWidgets('is gone while nothing is Unsorted', (tester) async {
      await start(tester);
      expect(find.byKey(const ValueKey('unsortedBanner')), findsNothing);
      expect(alarms.nudge, isNull);
    });
  });

  group('the Nudge screen', () {
    testWidgets('opens from the notification with the three oldest cards and '
        '"+x"', (tester) async {
      captureMany(5);
      await start(tester);
      alarms.tapNudge();
      await tester.pumpAndSettle();

      expect(find.byType(NudgeScreen), findsOneWidget);
      expect(find.text('Waiting to be sorted'), findsOneWidget);
      expect(cards(tester), ['note 1', 'note 2', 'note 3']);
      expect(find.text('+2'), findsOneWidget);

      await tap(tester, find.byKey(const ValueKey('nudgeMore')));
      expect(find.byType(UnsortedScreen), findsOneWidget);
      expect(cards(tester), hasLength(5));
    });

    testWidgets('a card opens its Note; a Note sorted meanwhile leaves', (
      tester,
    ) async {
      captureMany(3);
      await start(tester);
      alarms.tapNudge();
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('nudgeMore')), findsNothing);

      await tap(tester, find.text('note 2'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      // Filed while the editor is open.
      final area = Places(store).addArea('Household');
      notes.file(notes.all().firstWhere((n) => n.text == 'note 2').id, area.id);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(cards(tester), ['note 1', 'note 3']);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.text('2 Unsorted'), findsOneWidget);
    });

    testWidgets('opened over the Unsorted list: Notes filed there are gone '
        'from the list once it shows again', (tester) async {
      final places = Places(store);
      final household = places.addArea('Household');
      places.addTopic(household.id, 'Bills');
      captureMany(2);
      await start(tester);
      await tap(tester, find.byKey(const ValueKey('unsortedBanner')));
      expect(find.byType(UnsortedScreen), findsOneWidget);
      expect(cards(tester), ['note 1', 'note 2']);

      // The notification, tapped while the Unsorted list is open.
      alarms.tapNudge();
      await tester.pumpAndSettle();
      expect(find.byType(NudgeScreen), findsOneWidget);
      for (final text in ['note 1', 'note 2']) {
        await tap(tester, find.byTooltip('File into…').first);
        await tap(tester, find.text('Bills'));
        expect(cards(tester), isNot(contains(text)));
      }
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(find.byType(UnsortedScreen), findsOneWidget);
      expect(cards(tester), isEmpty);
      expect(find.text('Nothing is Unsorted. Well done!'), findsOneWidget);
    });

    testWidgets('says so when nothing is left', (tester) async {
      await start(tester);
      alarms.tapNudge();
      await tester.pumpAndSettle();
      expect(find.text('Nothing is Unsorted. Well done!'), findsOneWidget);
    });
  });

  group('the Nudge settings', () {
    testWidgets('change the repeat and Quiet Hours of this Device and are '
        'armed at once', (tester) async {
      captureMany(1);
      await start(tester);
      await tap(tester, find.byKey(const ValueKey('unsortedBanner')));
      expect(find.byKey(const ValueKey('nudgeRepeat')), findsNothing);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(
        alarms.nudge?.nudge.quiet,
        QuietHours(ClockTime(22, 0), ClockTime(8, 0)),
      );

      await openDeviceSettings(tester);
      expect(find.text('22:00'), findsOneWidget);
      expect(find.text('08:00'), findsOneWidget);
      await tap(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('nudgeRepeat')),
          matching: find.text('10 min'),
        ),
      );
      expect(store.nudgeRepeat, const Duration(minutes: 10));
      expect(alarms.nudge?.nudge.repeat, const Duration(minutes: 10));

      await tap(tester, find.byKey(const ValueKey('quietHours')));
      expect(store.quietHoursOn, isFalse);
      expect(alarms.nudge?.nudge.quiet, isNull);
      expect(find.byKey(const ValueKey('quietFrom')), findsNothing);
    });
  });

  testWidgets('a snooze tapped on the Nudge is applied for the Device', (
    tester,
  ) async {
    captureMany(1);
    await start(tester);
    final until = now.toUtc().add(const Duration(minutes: 30));
    alarms.tap(NudgeSnoozed(1, now.toUtc(), until));
    await tester.pumpAndSettle();

    expect(Alarms(store).nudgeSnooze, (at: now.toUtc(), until: until));
    expect(alarms.acknowledged, 1);
    expect(alarms.nudge?.nudge.snooze?.until, until);
    expect(
      store.allEntries().map((e) => e.field),
      isNot(contains('nudgeSnooze')),
    );
  });
}
