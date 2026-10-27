import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/ring_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
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

  group('Escalation in the Routine editor', () {
    Future<void> openSection(WidgetTester tester) async {
      final section = find.text('Escalation');
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
    }

    testWidgets('is off until turned on, then after 3 ignored Re-nags, and '
        'is armed on Save', (tester) async {
      final pills = routines.create('pills', daily([at(20)]));
      await start(tester);
      expect(alarms.armed.first.alarm.escalation, isNull);
      await tap(tester, find.text('pills').first);
      await openSection(tester);
      expect(find.text('Off: it only Re-nags'), findsOneWidget);
      expect(find.byKey(const ValueKey('escalationAfter')), findsNothing);

      await tap(tester, find.byKey(const ValueKey('escalationOn')));
      expect(
        find.text('Full-screen alarm after 3 ignored Re-nags'),
        findsOneWidget,
      );
      await tap(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('escalationAfter')),
          matching: find.text('5'),
        ),
      );
      expect(
        find.text('Full-screen alarm after 5 ignored Re-nags'),
        findsOneWidget,
      );
      expect(
        routines.byId(pills.id)?.escalation,
        isNull,
        reason: 'the Routine changes on Save',
      );

      await tap(tester, find.text('Save'));
      expect(routines.byId(pills.id)?.escalation, 5);
      expect(alarms.armed.first.alarm.escalation, 5);
    });

    testWidgets('a count outside the choices stays chosen', (tester) async {
      final pills = routines.create('pills', daily([at(20)]));
      routines.setEscalation(pills.id, 4);
      await start(tester);
      await tap(tester, find.text('pills').first);
      await openSection(tester);
      final column = find.byKey(const ValueKey('escalationAfter'));
      expect(find.byType(DropdownButton<int>), findsNothing);
      expect(
        find.descendant(of: column, matching: find.byType(RadioListTile<int>)),
        findsNWidgets(6),
      );
      expect(
        tester
            .widget<RadioGroup<int>>(
              find.descendant(
                of: column,
                matching: find.byType(RadioGroup<int>),
              ),
            )
            .groupValue,
        4,
      );
    });

    testWidgets('turned off again, the Routine no longer escalates', (
      tester,
    ) async {
      final pills = routines.create('pills', daily([at(20)]));
      routines.setEscalation(pills.id, 2);
      await start(tester);
      await tap(tester, find.text('pills').first);
      await openSection(tester);
      expect(
        find.text('Full-screen alarm after 2 ignored Re-nags'),
        findsOneWidget,
      );
      await tap(tester, find.byKey(const ValueKey('escalationOn')));
      await tap(tester, find.text('Save'));
      expect(routines.byId(pills.id)?.escalation, isNull);
      expect(alarms.armed.first.alarm.escalation, isNull);
    });
  });

  group('the full-screen alarm', () {
    late Routine pills;
    final morning = Occurrence(Day(2026, 10, 24), ClockTime(8, 0));
    Ringing ringing() => Ringing(
      routineId: pills.id,
      occurrenceKey: morning.key,
      title: 'pills',
      body: '08:00',
    );

    setUp(() {
      pills = routines.create('pills', daily([at(8)]));
      routines.setEscalation(pills.id, 3);
    });

    testWidgets('shows on top while it rings and is ticked off with Done', (
      tester,
    ) async {
      alarms.ringingNow = ringing();
      await start(tester);
      expect(find.byType(RingScreen), findsOneWidget);
      expect(find.text('pills'), findsOneWidget);
      expect(find.text('08:00'), findsOneWidget);

      await tap(tester, find.byKey(const ValueKey('ringDone')));
      expect(alarms.answers, [null]);
      expect(find.byType(RingScreen), findsNothing);
      expect(
        routines.byId(pills.id)?.doneAt(morning),
        isNotNull,
        reason: 'applied through the queue at once',
      );
      expect(alarms.pending, isEmpty, reason: 'acknowledged');
    });

    testWidgets('its Done label takes the button\'s colour', (tester) async {
      alarms.ringingNow = ringing();
      await start(tester);
      final done = find.byKey(const ValueKey('ringDone'));
      final label = tester.widget<RichText>(
        find.descendant(of: done, matching: find.byType(RichText)).last,
      );
      final scheme = Theme.of(tester.element(done)).colorScheme;
      expect(label.text.style?.color, scheme.onPrimary);
    });

    testWidgets('a snooze from it rings again later', (tester) async {
      await start(tester);
      expect(find.byType(RingScreen), findsNothing);
      alarms.ring(ringing());
      await tester.pumpAndSettle();
      expect(find.byType(RingScreen), findsOneWidget);

      await tap(tester, find.text('30 min'));
      expect(alarms.answers, [const Duration(minutes: 30)]);
      expect(find.byType(RingScreen), findsNothing);
      expect(Alarms(store).snoozes, hasLength(1));
      expect(routines.byId(pills.id)?.doneAt(morning), isNull);
    });

    testWidgets('closes when the ringing stops by itself', (tester) async {
      alarms.ringingNow = ringing();
      await start(tester);
      expect(find.byType(RingScreen), findsOneWidget);
      alarms.ring(null);
      await tester.pumpAndSettle();
      expect(find.byType(RingScreen), findsNothing);
      expect(alarms.answers, isEmpty);
    });

    testWidgets('cannot be swiped away while it rings', (tester) async {
      alarms.ringingNow = ringing();
      await start(tester);
      final navigator = tester.state<NavigatorState>(find.byType(Navigator));
      await navigator.maybePop();
      await tester.pumpAndSettle();
      expect(find.byType(RingScreen), findsOneWidget);
    });
  });
}
