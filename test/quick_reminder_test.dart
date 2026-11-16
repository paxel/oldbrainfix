import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'open_capture.dart';

/// A new Reminder's chips over its schedule (#271): In 1 h, Tonight,
/// Tomorrow and In a week, each with its time, Tomorrow's when this
/// Device's Quiet Hours end; and the When line's relative words.
void main() {
  late Store store;
  late Routines routines;
  // 14:20 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 14, 20);
  final today = Day(2026, 10, 24);

  setUp(() {
    now = DateTime(2026, 10, 24, 14, 20);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    routines = Routines(store);
  });
  tearDown(() => store.close());

  final chips = find.byKey(const ValueKey('quickReminders'));
  final line = find.byKey(const ValueKey('scheduleSentence'));

  Finder chip(String name) => find.byKey(ValueKey('quick-$name'));

  List<String> labels(WidgetTester tester) => [
    for (final chip in tester.widgetList<ActionChip>(
      find.descendant(of: chips, matching: find.byType(ActionChip)),
    ))
      (chip.label as Text).data ?? '',
  ];

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  final save = find.descendant(
    of: find.byType(RoutineEditorScreen),
    matching: find.widgetWithText(TextButton, 'Save'),
  );

  /// The Agenda's New Routine or Reminder, with its text typed.
  Future<void> newRoutine(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('New Routine or Reminder'));
    await tester.enterText(find.byType(TextField).first, 'call the plumber');
    await tester.pumpAndSettle();
  }

  /// A Reminder from a captured Note's card.
  Future<void> cardReminder(WidgetTester tester) async {
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'call the plumber');
    await tap(tester, find.text('Save'));
    await tap(tester, find.byTooltip('Routine, Reminder, Frog or Session'));
    await tap(tester, find.text('Reminder'));
    expect(find.text('New Reminder'), findsOneWidget);
  }

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  group('A new Reminder', () {
    testWidgets('from a card offers the four chips, each with its time, '
        'Tomorrow at the end of the Quiet Hours', (tester) async {
      await cardReminder(tester);
      expect(labels(tester), [
        'In 1 h, 15:20',
        'Tonight 19:00',
        'Tomorrow 08:00',
        'In a week, 08:00',
      ]);
    });

    testWidgets('a tap on Tomorrow sets it, and Save writes it', (
      tester,
    ) async {
      await cardReminder(tester);
      await tap(tester, chip('tomorrow'));
      expect(
        tester.widget<TextField>(line).controller?.text,
        'Once on 10/25 at 08:00',
      );
      await tap(tester, save);

      final reminder = routines.all().single;
      expect(reminder.isReminder, isTrue);
      expect(reminder.schedule.start, today.addDays(1));
      expect(reminder.schedule.times, [at(8)]);
    });

    testWidgets('Tomorrow and In a week follow this Device\'s '
        'Quiet Hours', (tester) async {
      store.quietHours = QuietHours(at(23), at(6, 45));
      await cardReminder(tester);
      expect(labels(tester), contains('Tomorrow 06:45'));
      await tap(tester, chip('inAWeek'));
      await tap(tester, save);
      final week = routines.all().single.schedule;
      expect(week.start, today.addDays(7));
      expect(week.times, [at(6, 45)]);
    });

    testWidgets('Tomorrow is 09:00 while the Quiet Hours are off', (
      tester,
    ) async {
      store.quietHoursOn = false;
      await cardReminder(tester);
      expect(labels(tester), contains('Tomorrow 09:00'));
    });

    testWidgets('In 1 h carries into the next day, and from 19:00 on '
        'Tonight is gone', (tester) async {
      now = DateTime(2026, 10, 24, 23, 30);
      await cardReminder(tester);
      expect(labels(tester), [
        'In 1 h, 00:30',
        'Tomorrow 08:00',
        'In a week, 08:00',
      ]);
      await tap(tester, chip('inAnHour'));
      await tap(tester, save);
      final hour = routines.all().single.schedule;
      expect(hour.start, today.addDays(1));
      expect(hour.times, [at(0, 30)]);
    });

    testWidgets('a chip is read again at the tap', (tester) async {
      now = DateTime(2026, 10, 24, 17, 30);
      await cardReminder(tester);
      expect(labels(tester).first, 'In 1 h, 18:30');
      now = DateTime(2026, 10, 24, 17, 45);
      await tap(tester, chip('inAnHour'));
      expect(labels(tester).first, 'In 1 h, 18:45');
      await tap(tester, save);
      expect(routines.all().single.schedule.times, [at(18, 45)]);
    });

    testWidgets('Tonight tapped after 19:00 only goes and sets nothing', (
      tester,
    ) async {
      now = DateTime(2026, 10, 24, 17, 30);
      await cardReminder(tester);
      now = DateTime(2026, 10, 24, 19, 5);
      await tap(tester, chip('tonight'));
      expect(chip('tonight'), findsNothing);
      await tap(tester, save);
      final kept = routines.all().single.schedule;
      expect(kept.start, today);
      expect(kept.times, [at(18)], reason: 'the next full hour, as it was');
    });

    testWidgets('of the quick note\'s alarm clock takes a chip with the Note', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'ring the plumber');
      await tap(
        tester,
        find.descendant(
          of: find.byType(CaptureScreen),
          matching: find.byIcon(Icons.alarm_add_outlined),
        ),
      );
      await tap(tester, find.text('Reminder').last);
      await tap(tester, chip('tonight'));
      await tap(tester, save);
      await tap(tester, find.byKey(const ValueKey('captureSave')));

      final reminder = routines.all().single;
      expect(reminder.schedule.start, today);
      expect(reminder.schedule.times, [at(19)]);
    });
  });

  group('A new Routine', () {
    testWidgets('offers the chips once set to Once, and not before', (
      tester,
    ) async {
      await newRoutine(tester);
      expect(chips, findsNothing);
      await tap(tester, find.text('Once'));
      expect(chips, findsOneWidget);
      await tap(tester, find.text('Advanced'));
      expect(chips, findsNothing);
    });

    testWidgets('reads "tomorrow 7 pm" in the When line', (tester) async {
      await newRoutine(tester);
      await tester.enterText(line, 'tomorrow 7 pm');
      await tester.pumpAndSettle();
      expect(find.text('New Reminder'), findsOneWidget);
      await tap(tester, save);

      final reminder = routines.all().single;
      expect(reminder.isReminder, isTrue);
      expect(reminder.schedule.start, today.addDays(1));
      expect(reminder.schedule.times, [at(19)]);
    });
  });

  testWidgets('a saved Reminder offers no chips', (tester) async {
    routines.create(
      'call the plumber',
      Schedule(start: today, times: [at(16)]),
    );
    await start(tester);
    await tester.longPress(find.text('call the plumber').first);
    await tester.pumpAndSettle();
    await tap(tester, find.byKey(const ValueKey('row-schedule')));
    expect(find.byType(RoutineEditorScreen), findsOneWidget);
    expect(chips, findsNothing);
  });

  testWidgets('a new Session offers no chips, also set to Once', (
    tester,
  ) async {
    await start(tester);
    final context = tester.element(find.byType(Scaffold).first);
    unawaited(newSession(context, Notes(store), Places(store)));
    await tester.pumpAndSettle();
    expect(find.text('New Session'), findsOneWidget);
    await tester.scrollUntilVisible(
      find.text('Once'),
      200,
      scrollable: find
          .descendant(
            of: find.byType(RoutineEditorScreen),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    await tap(tester, find.text('Once'));
    expect(chips, findsNothing);
  });
}
