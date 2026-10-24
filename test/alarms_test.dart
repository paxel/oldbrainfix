import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/setup_screen.dart';
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

  Future<void> start(WidgetTester tester, {AlarmPlatform? platform}) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, alarms: platform ?? alarms),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> resume(WidgetTester tester) async {
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
  }

  final banner = find.textContaining('a permission is missing');

  group('syncing the alarm plan', () {
    testWidgets('arms the plan on start, with texts in the reader\'s '
        'language', (tester) async {
      final pills = routines.create('pills\ntwo of them', daily([at(20)]));
      await start(tester);

      expect(alarms.ids.first, 'occurrence:${pills.id}@2026-10-24T20:00');
      expect(alarms.armed.first.title, 'pills');
      expect(alarms.armed.first.body, '20:00');
      expect(alarms.armed.first.alarm.at, DateTime(2026, 10, 24, 20).toUtc());
      expect(alarms.labels?.done, 'Done');
      expect(alarms.labels?.snooze, 'Snooze');
      expect(
        [
          alarms.labels?.snooze10,
          alarms.labels?.snooze30,
          alarms.labels?.snooze60,
        ],
        ['10 min', '30 min', '1 h'],
      );
    });

    testWidgets('arms again on every change, without the Agenda', (
      tester,
    ) async {
      await start(tester);
      expect(alarms.armed, isEmpty);

      final dentist = routines.create(
        'dentist',
        Schedule(start: today, times: [at(15)]),
      );
      await tester.pump();
      expect(alarms.ids, ['occurrence:${dentist.id}@2026-10-24T15:00']);

      routines.reschedule(dentist.id, Schedule(start: today, times: [at(16)]));
      await tester.pump();
      expect(alarms.ids, ['occurrence:${dentist.id}@2026-10-24T16:00']);

      routines.delete(dentist.id);
      await tester.pump();
      expect(alarms.armed, isEmpty);
    });

    testWidgets('Done on the Agenda leaves the alarm out and takes its '
        'notification away', (tester) async {
      final pills = routines.create('pills', daily([at(20)]));
      await start(tester);
      expect(alarms.done, isEmpty);
      await tap(tester, find.byType(Checkbox).first);
      expect(alarms.ids.first, 'occurrence:${pills.id}@2026-10-25T20:00');
      expect(alarms.done, ['${pills.id}@2026-10-24T20:00']);
    });

    testWidgets('arms again when the app comes back', (tester) async {
      await start(tester);
      final before = alarms.arms;
      await resume(tester);
      expect(alarms.arms, greaterThan(before));
    });

    testWidgets('without an alarm layer there is no setup and no banner', (
      tester,
    ) async {
      store.alarmSetupDone = false;
      await start(tester, platform: const NoAlarms());
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(find.byType(SetupScreen), findsNothing);
      expect(banner, findsNothing);
    });
  });

  group('taps on notifications', () {
    testWidgets('a Done tapped while the app was closed reaches the log at '
        'start, as of when it was tapped', (tester) async {
      final pills = routines.create('pills', daily([at(8), at(20)]));
      final tapped = DateTime.utc(2026, 10, 24, 6, 5);
      alarms.pending.add(DoneTapped(7, pills.id, '2026-10-24T20:00', tapped));
      await start(tester);

      final evening = Occurrence(today, at(20));
      expect(routines.byId(pills.id)?.doneAt(evening), tapped);
      expect(alarms.acknowledged, 7);
      expect(alarms.pending, isEmpty);
      expect(alarms.ids.first, 'occurrence:${pills.id}@2026-10-25T08:00');
      final box = tester
          .widgetList<Checkbox>(find.byType(Checkbox))
          .elementAt(1);
      expect(box.value, isTrue);
    });

    testWidgets('a Done tapped while the app runs shows at once', (
      tester,
    ) async {
      final pills = routines.create('pills', daily([at(20)]));
      await start(tester);
      expect(
        tester.widget<Checkbox>(find.byType(Checkbox).first).value,
        isFalse,
      );

      alarms.tap(DoneTapped(1, pills.id, '2026-10-24T20:00', now.toUtc()));
      await tester.pumpAndSettle();
      expect(
        tester.widget<Checkbox>(find.byType(Checkbox).first).value,
        isTrue,
      );
      expect(alarms.acknowledged, 1);
    });

    testWidgets('a snooze rings again and outlives the next sync', (
      tester,
    ) async {
      // Made before 08:00, so this morning's rang.
      now = DateTime(2026, 10, 24, 6);
      final pills = routines.create('pills', daily([at(8)]));
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      final until = now.add(const Duration(minutes: 30)).toUtc();
      alarms.tap(
        SnoozeTapped(1, pills.id, '2026-10-24T08:00', now.toUtc(), until),
      );
      await tester.pumpAndSettle();
      expect(alarms.ids.first, 'snooze:${pills.id}@2026-10-24T08:00');
      expect(alarms.armed.first.alarm.at, until);

      routines.create('water', daily([at(21)]));
      await tester.pump();
      expect(alarms.ids, contains('snooze:${pills.id}@2026-10-24T08:00'));
    });

    testWidgets('a tap for a deleted Routine is dropped quietly', (
      tester,
    ) async {
      alarms.pending.add(
        DoneTapped(3, 'routine:gone', '2026-10-24T08:00', now.toUtc()),
      );
      await start(tester);
      expect(alarms.acknowledged, 3);
      expect(store.allEntries(), isEmpty);
    });
  });

  group('the setup', () {
    testWidgets('walks through each permission once, at first start', (
      tester,
    ) async {
      store.alarmSetupDone = false;
      alarms.granted = {for (final p in AlarmPermission.values) p: false};
      await start(tester);

      expect(find.byType(SetupScreen), findsOneWidget);
      expect(find.text('Step 1 of 3'), findsOneWidget);
      expect(find.text('Notifications'), findsOneWidget);
      await tap(tester, find.text('Allow'));
      expect(alarms.requested, [AlarmPermission.notifications]);
      expect(find.text('Allowed'), findsOneWidget);
      await tap(tester, find.text('Next'));

      expect(find.text('Step 2 of 3'), findsOneWidget);
      expect(find.text('Alarms on the minute'), findsOneWidget);
      // Passed by: the alarms come inexact.
      await tap(tester, find.text('Next'));

      expect(find.text('Step 3 of 3'), findsOneWidget);
      expect(find.text('Full-screen alarms'), findsOneWidget);
      await tap(tester, find.text('Done'));

      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(store.alarmSetupDone, isTrue);
      expect(banner, findsOneWidget, reason: 'exact alarms are missing');
    });

    testWidgets('a red banner shows while a permission is missing and leads '
        'to the setup', (tester) async {
      alarms.granted[AlarmPermission.exactAlarms] = false;
      await start(tester);
      expect(banner, findsOneWidget);
      final colour = tester
          .widget<Material>(
            find.ancestor(of: banner, matching: find.byType(Material)).first,
          )
          .color;
      expect(colour, Theme.of(tester.element(banner)).colorScheme.error);

      await tap(tester, banner);
      expect(find.byType(SetupScreen), findsOneWidget);
      await tap(tester, find.text('Next'));
      await tap(tester, find.text('Allow'));
      expect(alarms.requested, [AlarmPermission.exactAlarms]);
      await tap(tester, find.text('Next'));
      await tap(tester, find.text('Done'));

      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(banner, findsNothing);
    });

    testWidgets('the banner follows what is granted when the app comes back', (
      tester,
    ) async {
      await start(tester);
      expect(banner, findsNothing);
      alarms.granted[AlarmPermission.notifications] = false;
      await resume(tester);
      expect(banner, findsOneWidget);
      alarms.granted[AlarmPermission.notifications] = true;
      await resume(tester);
      expect(banner, findsNothing);
    });

    testWidgets('a missing full-screen permission raises no banner while no '
        'Routine escalates', (tester) async {
      routines.create('plants', daily([at(20)]));
      alarms.granted[AlarmPermission.fullScreen] = false;
      await start(tester);
      expect(banner, findsNothing);
    });

    testWidgets('a missing full-screen permission raises the banner once a '
        'Routine escalates, and lowers it when none does', (tester) async {
      final pills = routines.create('pills', daily([at(20)]));
      alarms.granted[AlarmPermission.fullScreen] = false;
      await start(tester);
      expect(banner, findsNothing);

      routines.setEscalation(pills.id, 3);
      await tester.pumpAndSettle();
      expect(banner, findsOneWidget);

      routines.setEscalation(pills.id, null);
      await tester.pumpAndSettle();
      expect(banner, findsNothing);

      routines.setEscalation(pills.id, 3);
      await tester.pumpAndSettle();
      alarms.granted[AlarmPermission.fullScreen] = true;
      await resume(tester);
      expect(banner, findsNothing, reason: 'granted in the settings');
    });
  });
}
