import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/routine_history_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

void main() {
  late Store store;
  late Routines routines;
  late FakeAlarms alarms;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    routines = Routines(store);
    alarms = FakeAlarms();
  });
  tearDown(() => store.close());

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  /// Makes a Routine as of [made], before its Occurrences were due.
  Routine madeAt(DateTime made, String text, Schedule schedule) {
    final saved = now;
    now = made;
    final routine = routines.create(text, schedule);
    now = saved;
    return routine;
  }

  /// Pills at 08:00 since the 20th: Missed on the 20th, snoozed that
  /// morning; Done on the 21st to the 23rd; today's still due.
  Routine pills() {
    final pills = madeAt(DateTime(2026, 10, 20, 6), 'pills', daily([at(8)]));
    Alarms(
      store,
    ).applySnooze(pills.id, '2026-10-20T08:00', DateTime(2026, 10, 20, 8, 10));
    for (final day in [21, 22, 23]) {
      routines.markDone(
        pills.id,
        Occurrence(Day(2026, 10, day), at(8)),
        at: DateTime(2026, 10, day, 8, 5),
      );
    }
    return pills;
  }

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  final page = find.byType(RoutineHistoryScreen);

  /// The dots of the days that came to [outcome].
  Finder dot(String outcome) => find.byWidgetPredicate(
    (w) =>
        w is Semantics && (w.properties.label?.endsWith(': $outcome') ?? false),
  );
  final historyButton = find.byTooltip('History');
  final streak = find.text('Streak');

  /// The figure over [label] in the Streak panel.
  String? figureOver(String label) {
    final column = find.ancestor(
      of: find.text(label),
      matching: find.byType(Column),
    );
    final texts = find.descendant(
      of: column.first,
      matching: find.byType(Text),
    );
    return (texts.evaluate().first.widget as Text).data;
  }

  group('the history page', () {
    testWidgets('opens from the Routine editor with Streak, Adherence and '
        'the log', (tester) async {
      pills();
      await start(tester);
      await tap(tester, find.text('pills').first);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      await tap(tester, historyButton);
      expect(page, findsOneWidget);

      expect(streak, findsOneWidget);
      expect(figureOver('Now'), '3');
      expect(figureOver('Longest'), '3');
      expect(dot('all Done'), findsNWidgets(3));
      expect(dot('Missed'), findsOneWidget);
      expect(dot('still due'), findsOneWidget);
      expect(dot('nothing due'), findsNWidgets(2));

      // Week, month and 30 days alike: 3 of 4; today's does not count.
      expect(find.text('Adherence'), findsOneWidget);
      expect(find.text('75%'), findsNWidgets(3));
      expect(find.text('3 of 4 Done'), findsNWidgets(3));

      final scrollable = find
          .descendant(of: page, matching: find.byType(Scrollable))
          .first;
      await tester.scrollUntilVisible(
        find.text('Snoozed until 08:10'),
        200,
        scrollable: scrollable,
      );
      expect(find.text('Due, not Done yet'), findsOneWidget);
      expect(find.text('Done: 08:05'), findsNWidgets(3));
      expect(find.text('Missed'), findsOneWidget);
      expect(find.text('Snoozed until 08:10'), findsOneWidget);
      expect(
        find.text(
          'Snoozes show only as this phone recorded them, for about the '
          'last month.',
        ),
        findsOneWidget,
      );
    });

    testWidgets('opens from a long press on a Routine on the Agenda', (
      tester,
    ) async {
      pills();
      await start(tester);
      await tester.longPress(find.text('pills').first);
      await tester.pumpAndSettle();
      await tap(tester, find.text('History'));
      expect(page, findsOneWidget);
      expect(figureOver('Now'), '3');
    });

    testWidgets('a late Done shows as late, with its day', (tester) async {
      final pills = madeAt(DateTime(2026, 10, 22, 6), 'pills', daily([at(8)]));
      routines.markDone(
        pills.id,
        Occurrence(Day(2026, 10, 22), at(8)),
        at: DateTime(2026, 10, 23, 8, 30),
      );
      await start(tester);
      await tap(tester, find.text('pills').first);
      await tap(tester, historyButton);
      await tester.scrollUntilVisible(
        find.textContaining('Done late: '),
        200,
        scrollable: find
            .descendant(of: page, matching: find.byType(Scrollable))
            .first,
      );
      expect(find.textContaining('Done late: '), findsOneWidget);
      expect(find.textContaining('08:30'), findsOneWidget);
    });

    testWidgets('Streaks switched off show nowhere; Adherence stays', (
      tester,
    ) async {
      pills();
      await start(tester);
      await tap(tester, find.text('pills').first);
      await tap(tester, historyButton);
      expect(streak, findsOneWidget);

      expect(find.text('Show Streaks on this phone'), findsNothing);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('Settings'));
      await tap(tester, find.text('Show Streaks on this phone'));
      expect(store.streaksOn, isFalse);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.text('pills').first);
      await tap(tester, historyButton);
      expect(streak, findsNothing);
      expect(find.text('Now'), findsNothing);
      expect(find.text('Last 7 days'), findsNothing);
      expect(dot('all Done'), findsNothing);
      expect(find.text('75%'), findsNWidgets(3));

      // Still off the next time.
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, historyButton);
      expect(streak, findsNothing);
      expect(find.text('Adherence'), findsOneWidget);

      store.streaksOn = true;
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, historyButton);
      expect(streak, findsOneWidget);
    });

    testWidgets('a Reminder has no history', (tester) async {
      madeAt(
        DateTime(2026, 10, 24, 6),
        'call mum',
        Schedule(start: Day(2026, 10, 24), times: [at(10)]),
      );
      await start(tester);
      // No menu: the press opens the Reminder as a tap does.
      await tester.longPress(find.text('call mum'));
      await tester.pumpAndSettle();
      expect(find.text('History'), findsNothing);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(historyButton, findsNothing);
    });
  });
}
