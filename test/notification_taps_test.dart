import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/notification_taps.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

/// The trace of the notification taps (obf-1039): what each tap brought,
/// where it went, how the app was and when, without a Note's text,
/// bounded and kept in a file of its own.
void main() {
  setUp(notificationTaps.clear);

  group('the trace', () {
    test('keeps the last ten taps, oldest first, in its file', () {
      final dir = Directory.systemTemp.createTempSync('taps');
      addTearDown(() => dir.deleteSync(recursive: true));
      final taps = NotificationTaps()..keepIn(dir);
      for (var i = 0; i < 12; i++) {
        taps.record(
          kind: 'nudge',
          arrival: (
            kind: 'nudge',
            from: TapFrom.background,
            at: DateTime.utc(2026, 10, 24, 9, i),
          ),
          went: TapWent.nudge,
        );
      }
      expect(taps.count, NotificationTaps.kept);
      final lines = taps.text?.split('\n') ?? const [];
      expect(lines.first, startsWith('2026-10-24T09:02:00.000Z nudge'));
      expect(
        lines.last,
        '2026-10-24T09:11:00.000Z nudge from=background -> nudge page',
      );

      final again = NotificationTaps()..keepIn(dir);
      expect(again.text, taps.text, reason: 'a later run reads them');
    });

    test('reads what the platform tells, and nothing it cannot', () {
      expect(tapArrivalOf({'kind': 'headsUp', 'from': 'cold', 'at': 0}), (
        kind: 'headsUp',
        from: TapFrom.cold,
        at: DateTime.utc(1970),
      ));
      expect(
        tapArrivalOf({'kind': 'ring', 'from': 'sideways', 'at': 0})?.from,
        TapFrom.unknown,
      );
      expect(tapArrivalOf({'kind': 'ring'}), isNull);
      expect(tapArrivalOf(null), isNull);
    });
  });

  group('a tap', () {
    late Store store;
    late Routines routines;
    late FakeAlarms alarms;
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

    Future<void> start(WidgetTester tester) async {
      tester.view.physicalSize = const Size(1200, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
      await tester.pumpAndSettle();
    }

    testWidgets("on an Occurrence's Heads-up is noted with its ids, how "
        "the app was and where it went, never the Note's text", (tester) async {
      now = DateTime(2026, 10, 24, 6);
      final routine = routines.create(
        'pills for the heart',
        Schedule(
          start: Day(2026, 10, 1),
          times: [ClockTime(8, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      now = DateTime(2026, 10, 24, 9);
      await start(tester);

      alarms.tapOccurrence(
        (routineId: routine.id, occurrenceKey: '2026-10-24T08:00'),
        kind: 'headsUp',
        from: TapFrom.background,
      );
      await tester.pumpAndSettle();

      expect(
        notificationTaps.text,
        '2026-10-24T09:00:00.000Z headsUp routine=${routine.id} '
        'occurrence=2026-10-24T08:00 from=background '
        '-> ${TapWent.agendaToday}',
      );
      expect(notificationTaps.text, isNot(contains('pills')));
    });

    testWidgets('on the Nudge is noted as it opens the Nudge page', (
      tester,
    ) async {
      Notes(store).capture('call the plumber');
      await start(tester);

      alarms.tapNudge();
      await tester.pumpAndSettle();

      expect(find.byType(NudgeScreen), findsOneWidget);
      expect(
        notificationTaps.text,
        '2026-10-24T09:00:00.000Z nudge from=running -> ${TapWent.nudge}',
      );
    });

    testWidgets('on the Catch-up Notification is noted as it shows the '
        'Agenda', (tester) async {
      await start(tester);

      alarms.tapCatchUp();
      await tester.pumpAndSettle();

      expect(
        notificationTaps.text,
        '2026-10-24T09:00:00.000Z catchUp from=running -> ${TapWent.agenda}',
      );
    });
  });
}
