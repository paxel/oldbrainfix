import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/setup_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The iPhone's alarms (#238): [IosAlarms] on the same channel as
/// Android's, answered here as ios/Runner/AlarmsChannel.swift answers.
void main() {
  const channel = MethodChannel('io.github.paxel.oldbrainfix/alarms');
  late Store store;
  late Routines routines;
  late IosAlarms alarms;
  late Map<String, bool> granted;
  late List<Map<Object?, Object?>> pending;
  late List<MethodCall> calls;
  var now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    routines = Routines(store);
    // What the Swift side reports: its two permissions only.
    granted = {'notifications': true, 'alarms': true};
    pending = [];
    calls = [];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          switch (call.method) {
            case 'permissions':
              return Map.of(granted);
            case 'request':
              granted[call.arguments as String] = true;
              return null;
            case 'pending':
              return List.of(pending);
            case 'ack':
              final seq = call.arguments as int;
              pending.removeWhere((m) => (m['seq'] as int) <= seq);
              return null;
            case 'ringing':
            case 'pickOpened':
              return null;
            case 'nudgeOpened':
            case 'changesOpened':
            case 'unsortedOpened':
              return false;
            case 'arm':
              return null;
          }
          // Not on the iPhone yet, as the Agenda widget (#242).
          throw MissingPluginException();
        });
  });
  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
    store.close();
  });

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  test('offer no choice of sound: iOS has no sound picker (#244)', () {
    expect(IosAlarms().choosesSound, isFalse);
    expect(AndroidAlarms().choosesSound, isTrue);
  });

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  Future<void> start(WidgetTester tester) async {
    alarms = IosAlarms();
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  List<MethodCall> named(String method) => [
    for (final c in calls)
      if (c.method == method) c,
  ];

  final banner = find.textContaining('a permission is missing');

  testWidgets('the setup asks for notifications and alarms only', (
    tester,
  ) async {
    store.alarmSetupDone = false;
    granted = {'notifications': false, 'alarms': false};
    await start(tester);

    expect(find.byType(SetupScreen), findsOneWidget);
    expect(find.text('Step 1 of 2'), findsOneWidget);
    expect(find.text('Notifications'), findsOneWidget);
    await tap(tester, find.text('Allow'));
    expect([for (final c in named('request')) c.arguments], ['notifications']);
    expect(find.text('Allowed'), findsOneWidget);
    await tap(tester, find.text('Next'));

    expect(find.text('Step 2 of 2'), findsOneWidget);
    expect(find.text('Alarms'), findsOneWidget);
    expect(find.text('Alarms on the minute'), findsNothing);
    expect(find.text('Full-screen alarms'), findsNothing);
    // Passed by: escalated Occurrences fall back to notifications.
    await tap(tester, find.text('Done'));

    expect(find.byType(AgendaScreen), findsOneWidget);
    expect(store.alarmSetupDone, isTrue);
    expect(
      banner,
      findsNothing,
      reason: 'the alarms only matter while a Routine escalates',
    );
  });

  testWidgets('a missing alarms permission raises the banner once a '
      'Routine escalates', (tester) async {
    final pills = routines.create('pills', daily([at(20)]));
    granted['alarms'] = false;
    await start(tester);
    expect(banner, findsNothing);

    routines.setEscalation(pills.id, 3);
    await tester.pumpAndSettle();
    expect(banner, findsOneWidget);

    await tap(tester, banner);
    expect(find.text('Step 1 of 2'), findsOneWidget);
    await tap(tester, find.text('Next'));
    await tap(tester, find.text('Allow'));
    expect([for (final c in named('request')) c.arguments], ['alarms']);
    await tap(tester, find.text('Done'));
    expect(banner, findsNothing);
  });

  testWidgets('the plan reaches the channel', (tester) async {
    final pills = routines.create('pills\ntwo of them', daily([at(20)]));
    routines.setEscalation(pills.id, 2);
    await start(tester);

    final arm = named('arm').last.arguments as Map<Object?, Object?>;
    final armed = arm['alarms'] as List<Object?>;
    final first = armed.first as Map<Object?, Object?>;
    expect(first['routine'], pills.id);
    expect(first['occurrence'], '2026-10-24T20:00');
    expect(first['local'], [2026, 10, 24, 20, 0]);
    expect(first['escalate'], 2);
    expect(first['title'], 'pills');
    final labels = arm['labels'] as Map<Object?, Object?>;
    expect(labels['done'], 'Done');
    expect(
      [labels['snooze10'], labels['snooze30'], labels['snooze60']],
      ['10 min', '30 min', '1 h'],
    );
  });

  testWidgets('a Done and a snooze tapped on the lock screen reach the log '
      'and are acknowledged', (tester) async {
    final pills = routines.create('pills', daily([at(8), at(20)]));
    final dentist = routines.create(
      'dentist',
      Schedule(start: today, times: [at(15)]),
    );
    final tapped = DateTime.utc(2026, 10, 24, 6, 5);
    pending.addAll([
      {
        'seq': 4,
        'kind': 'done',
        'routine': pills.id,
        'occurrence': '2026-10-24T20:00',
        'at': tapped.millisecondsSinceEpoch,
        'until': 0,
        'line': '',
      },
      {
        'seq': 5,
        'kind': 'snooze',
        'routine': dentist.id,
        'occurrence': '2026-10-24T15:00',
        'at': tapped.millisecondsSinceEpoch,
        'until': DateTime(2026, 10, 24, 15, 10).millisecondsSinceEpoch,
        'line': '',
      },
    ]);
    await start(tester);

    final evening = Occurrence(today, at(20));
    expect(routines.byId(pills.id)?.doneAt(evening), tapped);
    expect(named('ack').last.arguments, 5);
    expect(pending, isEmpty);
    final arm = named('arm').last.arguments as Map<Object?, Object?>;
    final ids = [
      for (final a in arm['alarms'] as List<Object?>)
        (a as Map<Object?, Object?>)['id'],
    ];
    expect(ids, contains('snooze:${dentist.id}@2026-10-24T15:00'));
  });

  testWidgets('Skip and Tomorrow tapped on the lock screen reach the log '
      '(#269)', (tester) async {
    final pills = routines.create('pills', daily([at(8), at(20)]));
    final tapped = DateTime.utc(2026, 10, 24, 6, 5);
    pending.addAll([
      for (final (seq, kind, occurrence) in [
        (1, 'skip', '2026-10-24T08:00'),
        (2, 'tomorrow', '2026-10-24T20:00'),
      ])
        {
          'seq': seq,
          'kind': kind,
          'routine': pills.id,
          'occurrence': occurrence,
          'at': tapped.millisecondsSinceEpoch,
          'until': 0,
          'line': '',
        },
    ]);
    await start(tester);

    final routine = routines.byId(pills.id)!;
    expect(routine.skippedAt(Occurrence(today, at(8))), tapped);
    expect(routine.skippedAt(Occurrence(today, at(20))), tapped);
    expect(routine.tomorrow, contains('2026-10-24T20:00'));
    expect(named('ack').last.arguments, 2);
  });

  testWidgets('a tap while the app runs arrives through "actions"', (
    tester,
  ) async {
    final pills = routines.create('pills', daily([at(20)]));
    await start(tester);
    expect(tester.widget<Checkbox>(find.byType(Checkbox).first).value, isFalse);

    pending.add({
      'seq': 1,
      'kind': 'done',
      'routine': pills.id,
      'occurrence': '2026-10-24T20:00',
      'at': now.toUtc().millisecondsSinceEpoch,
      'until': 0,
      'line': '',
    });
    await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .handlePlatformMessage(
          channel.name,
          channel.codec.encodeMethodCall(const MethodCall('actions')),
          (_) {},
        );
    await tester.pumpAndSettle();

    expect(tester.widget<Checkbox>(find.byType(Checkbox).first).value, isTrue);
    expect(named('ack').last.arguments, 1);
  });

  testWidgets('nothing rings as a full-screen alarm on the iPhone', (
    tester,
  ) async {
    await start(tester);
    expect(await alarms.ringing(), isNull);
    expect(alarms.steps, isNot(contains(AlarmPermission.fullScreen)));
    expect(alarms.steps, isNot(contains(AlarmPermission.exactAlarms)));
  });
}
