import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/missed_summary_card.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

void main() {
  late Store store;
  late Routines routines;
  late FakeAlarms alarms;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);

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

  final card = find.byType(MissedSummaryCard);
  Finder inCard(Finder finder) => find.descendant(of: card, matching: finder);

  group('the Missed Summary', () {
    testWidgets('shows on top of the Agenda what was Missed and how often', (
      tester,
    ) async {
      madeAt(DateTime(2026, 10, 21, 6), 'pills\ntwo of them', daily([at(8)]));
      madeAt(
        DateTime(2026, 10, 22, 6),
        'call mum',
        Schedule(start: Day(2026, 10, 23), times: [at(10)]),
      );
      await start(tester);

      expect(card, findsOneWidget);
      expect(inCard(find.text('Missed')), findsOneWidget);
      // 21st, 22nd and 23rd; today's 08:00 still Re-nags.
      expect(inCard(find.text('pills · 3 times')), findsOneWidget);
      expect(inCard(find.text('call mum · once')), findsOneWidget);
      expect(inCard(find.byType(Checkbox)), findsNWidgets(4));
      final cardTop = tester.getTopLeft(card).dy;
      expect(cardTop, lessThan(tester.getTopLeft(find.text('Today')).dy));
    });

    testWidgets('lists the latest few to check off, and how many more', (
      tester,
    ) async {
      madeAt(DateTime(2026, 10, 18, 6), 'pills', daily([at(8)]));
      await start(tester);
      expect(inCard(find.text('pills · 6 times')), findsOneWidget);
      expect(
        inCard(find.byType(Checkbox)),
        findsNWidgets(MissedSummaryCard.shown),
      );
      expect(inCard(find.text('and 3 more')), findsOneWidget);
    });

    testWidgets('stays when the app opens again, until "seen"', (tester) async {
      madeAt(DateTime(2026, 10, 22, 6), 'pills', daily([at(8)]));
      await start(tester);
      expect(card, findsOneWidget);

      await tester.pumpWidget(const SizedBox());
      await start(tester);
      expect(card, findsOneWidget, reason: 'opening the app never clears it');

      await tap(tester, inCard(find.text('Seen')));
      expect(card, findsNothing);
      expect(store.missedSeen, now.toUtc());
      await tester.pumpWidget(const SizedBox());
      await start(tester);
      expect(card, findsNothing);
    });

    testWidgets('checking one off marks it Done late', (tester) async {
      final pills = madeAt(DateTime(2026, 10, 22, 6), 'pills', daily([at(8)]));
      await start(tester);
      expect(inCard(find.text('pills · 2 times')), findsOneWidget);

      // The latest first: the 23rd.
      await tap(tester, inCard(find.byType(Checkbox)).first);
      expect(
        routines.byId(pills.id)?.doneAt(Occurrence(Day(2026, 10, 23), at(8))),
        now.toUtc(),
      );
      expect(inCard(find.text('pills · once')), findsOneWidget);

      await tap(tester, inCard(find.byType(Checkbox)).first);
      expect(card, findsNothing, reason: 'nothing left Missed');
    });

    testWidgets('takes in what becomes Missed while the Agenda is open', (
      tester,
    ) async {
      madeAt(
        DateTime(2026, 10, 24, 6),
        'pills',
        Schedule(
          start: today,
          times: [at(8)],
          rule: Recurrence.parse('FREQ=DAILY;BYHOUR=8,10'),
        ),
      );
      await start(tester);
      expect(card, findsNothing);

      now = DateTime(2026, 10, 24, 10);
      await tester.pump(const Duration(hours: 1, seconds: 2));
      await tester.pumpAndSettle();
      expect(inCard(find.text('pills · once')), findsOneWidget);
    });
  });

  group('Re-nag', () {
    testWidgets('the plan hands the interval and its end to the platform', (
      tester,
    ) async {
      final pills = madeAt(DateTime(2026, 10, 24, 6), 'pills', daily([at(8)]));
      store.renagInterval = const Duration(minutes: 15);
      await start(tester);
      final morning = alarms.armed.firstWhere(
        (a) => a.alarm.id == 'occurrence:${pills.id}@2026-10-24T08:00',
      );
      expect(morning.alarm.renag, const Duration(minutes: 15));
      expect(morning.alarm.ends, (
        day: Day(2026, 10, 25),
        time: ClockTime(8, 0),
      ));
      expect(alarms.labels?.catchUpTitle, 'Missed while the phone was off');
      expect(
        alarms.labels?.catchUpMany,
        '{count} alarms were Missed. '
        'Open the app to see them.',
      );
    });

    testWidgets('a Routine sets its own interval and the Device\'s default '
        'in its Re-nag section', (tester) async {
      final pills = routines.create('pills', daily([at(8)]));
      await start(tester);
      await tap(tester, find.text('pills').first);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);

      final section = find.text('Re-nag');
      await tester.scrollUntilVisible(
        section,
        200,
        scrollable: find
            .descendant(
              of: find.byType(RoutineEditorScreen),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      await tap(tester, section);
      expect(find.text('Rings again every 10 min until Done'), findsOneWidget);

      await tap(tester, find.byKey(const ValueKey('renagDevice')));
      await tap(tester, find.text('30 min').last);
      expect(store.renagInterval, const Duration(minutes: 30));

      await tap(tester, find.byKey(const ValueKey('renagThis')));
      await tap(tester, find.text('5 min').last);
      expect(find.text('Rings again every 5 min until Done'), findsOneWidget);
      expect(
        routines.byId(pills.id)?.renag,
        isNull,
        reason: 'the Routine changes on Save',
      );

      await tap(tester, find.text('Save'));
      expect(routines.byId(pills.id)?.renag, const Duration(minutes: 5));
      final [armed, ...] = alarms.armed;
      expect(armed.alarm.renag, const Duration(minutes: 5));
    });

    testWidgets('a changed Device default is armed without a Save', (
      tester,
    ) async {
      routines.create('pills', daily([at(20)]));
      await start(tester);
      expect(alarms.armed.first.alarm.renag, const Duration(minutes: 10));
      await tap(tester, find.text('pills').first);
      final section = find.text('Re-nag');
      await tester.scrollUntilVisible(
        section,
        200,
        scrollable: find
            .descendant(
              of: find.byType(RoutineEditorScreen),
              matching: find.byType(Scrollable),
            )
            .first,
      );
      await tap(tester, section);
      await tap(tester, find.byKey(const ValueKey('renagDevice')));
      await tap(tester, find.text('15 min').last);
      await tap(tester, find.byType(BackButton));
      expect(alarms.armed.first.alarm.renag, const Duration(minutes: 15));
    });

    testWidgets('the app arms again on its way to the background', (
      tester,
    ) async {
      routines.create('pills', daily([at(20)]));
      await start(tester);
      store.renagInterval = const Duration(minutes: 30);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      await tester.pumpAndSettle();
      expect(alarms.armed.first.alarm.renag, const Duration(minutes: 30));
    });

    testWidgets('saving an unchanged schedule keeps what was Missed', (
      tester,
    ) async {
      madeAt(DateTime(2026, 10, 22, 6), 'pills', daily([at(8)]));
      await start(tester);
      await tap(tester, find.text('pills').last);
      await tap(tester, find.text('Save'));
      expect(inCard(find.text('pills · 2 times')), findsOneWidget);
    });
  });
}
