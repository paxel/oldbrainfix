import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/appointments.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/calendar_mirror.dart';
import 'package:oldbrainfix/src/calendar_platform.dart';
import 'package:oldbrainfix/src/capture_requests.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/context_platform.dart';
import 'package:oldbrainfix/src/first_start_screen.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/location.dart';
import 'package:oldbrainfix/src/lock_platform.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/ocr_platform.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/profile_platform.dart';
import 'package:oldbrainfix/src/profile_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix/src/sync_platform.dart';
import 'package:oldbrainfix/src/voice_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';
import 'open_settings.dart';

/// Every method channel the app talks to. On the iPhone each answers
/// since its ticket brought the Swift side (#237 to #243), but `exit`,
/// which iOS keeps no record for; the app must
/// still start and run where none answers: each call fails with a
/// [MissingPluginException], as here, where no handler is set.
const channels = [
  'alarms',
  'bundle',
  'calendar',
  'capture',
  'context',
  'device',
  'exit',
  'folder',
  'location',
  'lock',
  'ocr',
  'pairing',
  'photos',
  'profile',
  'speech',
  'sync',
  'voice',
];

void main() {
  late Store store;
  late IosAlarms alarms;

  setUp(() {
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    for (final name in channels) {
      messenger.setMockMethodCallHandler(
        MethodChannel('io.github.paxel.oldbrainfix/$name'),
        null,
      );
    }
    store = Store.inMemory();
  });
  tearDown(() => store.close());

  /// Lets the channels' answers arrive, then draws what they changed. A
  /// missing plugin answers through the real event loop, which the
  /// test's fake clock never reaches on its own.
  Future<void> settle(WidgetTester tester) async {
    // One round per call awaited after another, as the alarms make
    // several in a row.
    for (var i = 0; i < 20; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 10)),
      );
      await tester.pumpAndSettle();
    }
  }

  /// The app as main.dart starts it on the phone, with the wrappers of
  /// every channel, the iPhone's alarms, and none of them answering.
  Future<void> start(WidgetTester tester) async {
    alarms = IosAlarms();
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        location: const AndroidLocation(),
        alarms: alarms,
        photos: AndroidPhotos(),
        voice: AndroidVoice(),
        context: const AndroidContext(),
        requests: AndroidCaptureRequests(),
        speech: AndroidSpeech(),
        ocr: const AndroidOcr(),
        calendar: const AndroidCalendar(),
        mirror: const AndroidMirror(),
        appointments: const AndroidAppointments(),
        lock: const AndroidBiometrics(),
        pairing: const AndroidPairing(),
        sync: IosSync(),
        folders: const AndroidFolders(),
        bundles: AndroidBundles(),
        profile: AndroidProfile(),
      ),
    );
    await settle(tester);
  }

  test('the iPhone hosts only while the app is open, Android also out of '
      'sight (#240)', () {
    expect(IosSync().hostsOutOfSight, isFalse);
    expect(AndroidSync().hostsOutOfSight, isTrue);
  });

  testWidgets('a share the Share extension kept opens Capture on start, '
      'subject and text in one (#241)', (tester) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    // The photos channel as SharesChannel.swift answers it: the share
    // once, then none; every other method not implemented.
    final waiting = <Map<String, Object?>>[
      {
        'subject': 'Sauce recipe',
        'text': 'https://example.org/sauce',
        'photos': <Object?>[],
        'recordings': <Object?>[],
      },
    ];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
          const MethodChannel('io.github.paxel.oldbrainfix/photos'),
          (call) async {
            if (call.method != 'shared') {
              throw MissingPluginException(call.method);
            }
            return waiting.isEmpty ? null : waiting.removeAt(0);
          },
        );
    await start(tester);

    expect(tester.takeException(), isNull);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('Sauce recipe\nhttps://example.org/sauce'), findsOne);
    expect(waiting, isEmpty);
  });

  testWidgets('a capture a Capture widget or control asked for opens its '
      'screen on start (#242)', (tester) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    // The capture channel as CaptureChannel.swift answers it: the mode
    // the link asked for once, then none.
    final asked = ['text'];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
          const MethodChannel('io.github.paxel.oldbrainfix/capture'),
          (call) async {
            if (call.method != 'take') throw MissingPluginException();
            return asked.isEmpty ? null : asked.removeAt(0);
          },
        );
    await start(tester);

    expect(tester.takeException(), isNull);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(asked, isEmpty);
  });

  testWidgets('boxes ticked on the Agenda widget reach the log as the app '
      'refreshes, and its Unsorted count opens the Unsorted list (#242)', (
    tester,
  ) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    final notes = Notes(store);
    final frogs = Frogs(store);
    final taxes = frogs.spawn(notes.capture('taxes').id);
    final shopping = ShoppingLists(store);
    final groceries = notes.capture('Groceries\nMilk').id;
    shopping.make(groceries);
    notes.capture('nees mor sauce');
    final milk = shopping.byNote(groceries, deviceWallClock)!.lines.single;
    final at = DateTime(2026, 10, 24, 8).millisecondsSinceEpoch;
    // The alarms channel as AlarmsChannel.swift answers it: the widget's
    // ticks queued in the App Group, in QueuedAction.channelMap's shape,
    // until acknowledged; the Unsorted count's link once.
    var pending = <Map<String, Object?>>[
      {
        'seq': 1,
        'kind': 'shopping',
        'routine': '',
        'occurrence': '',
        'at': at,
        'until': 0,
        'line': milk.id,
      },
      {
        'seq': 2,
        'kind': 'frog',
        'routine': taxes.id,
        'occurrence': '',
        'at': at,
        'until': 0,
        'line': '',
      },
    ];
    var unsorted = true;
    final acknowledged = <int>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
          const MethodChannel('io.github.paxel.oldbrainfix/alarms'),
          (call) async {
            switch (call.method) {
              case 'pending':
                return pending;
              case 'ack':
                final seq = call.arguments as int;
                acknowledged.add(seq);
                pending = [
                  for (final p in pending)
                    if ((p['seq'] as int) > seq) p,
                ];
                return null;
              case 'unsortedOpened':
                final opened = unsorted;
                unsorted = false;
                return opened;
              case 'pickOpened':
              case 'ringing':
                return null;
              case 'nudgeOpened':
              case 'changesOpened':
                return false;
              case 'permissions':
                return {'notifications': true, 'alarms': true};
              default:
                return null;
            }
          },
        );
    await start(tester);

    expect(tester.takeException(), isNull);
    expect(
      shopping.byNote(groceries, deviceWallClock)?.lines.single.struck,
      isTrue,
    );
    expect(frogs.byId(taxes.id)?.isDone, isTrue);
    expect(acknowledged, isNotEmpty);
    expect(acknowledged.last, 2);
    expect(pending, isEmpty);
    expect(find.byType(UnsortedScreen), findsOneWidget);
  });

  testWidgets("a tap on an Occurrence's notification opens its Note, as "
      'AlarmsChannel.swift tells it once (#270)', (tester) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    final routine = Routines(store).create(
      'pills',
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    Map<String, String>? opened = {
      'routine': routine.id,
      'occurrence': '2026-10-24T08:00',
    };
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
          const MethodChannel('io.github.paxel.oldbrainfix/alarms'),
          (call) async {
            switch (call.method) {
              case 'occurrenceOpened':
                final answer = opened;
                opened = null;
                return answer;
              case 'pending':
                return const <Object?>[];
              case 'nudgeOpened':
              case 'changesOpened':
              case 'unsortedOpened':
                return false;
              case 'permissions':
                return {'notifications': true, 'alarms': true};
              default:
                return null;
            }
          },
        );
    await start(tester);

    expect(tester.takeException(), isNull);
    expect(opened, isNull);
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  });

  testWidgets('on the iPhone a page pushed on the Agenda goes back with '
      'the edge swipe (#243)', (tester) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
    try {
      await start(tester);
      await openProfilePage(tester);
      expect(find.byType(ProfileScreen), findsOneWidget);

      await tester.dragFrom(const Offset(5, 400), const Offset(600, 0));
      await settle(tester);

      expect(tester.takeException(), isNull);
      expect(find.byType(ProfileScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets('a first start without any channel shows the first screen', (
    tester,
  ) async {
    await start(tester);

    expect(tester.takeException(), isNull);
    expect(find.byType(FirstStartScreen), findsOneWidget);
  });

  testWidgets('without any channel the alarm setup can be walked through', (
    tester,
  ) async {
    store.starterSetOffered = true;
    await start(tester);

    // Nothing is allowed where no channel answers: every step offers
    // Allow, which asks in vain, and Next moves on, to the Agenda at the
    // end.
    for (final _ in alarms.steps) {
      await tester.tap(find.byType(FilledButton));
      await settle(tester);
      expect(find.byType(FilledButton), findsOneWidget);
      await tester.tap(find.byType(OutlinedButton));
      await settle(tester);
    }

    expect(tester.takeException(), isNull);
    expect(find.byType(AgendaScreen), findsOneWidget);
  });

  testWidgets('without any channel the Agenda shows, Capture and the '
      'settings open', (tester) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    await start(tester);
    expect(find.byType(AgendaScreen), findsOneWidget);

    await openTextCapture(tester);
    await settle(tester);
    await tester.pageBack();
    await settle(tester);
    expect(find.byType(AgendaScreen), findsOneWidget);

    await openDeviceSettings(tester);
    await settle(tester);
    await tester.pageBack();
    await settle(tester);
    await tester.pageBack();
    await settle(tester);

    expect(tester.takeException(), isNull);
    expect(find.byType(AgendaScreen), findsOneWidget);
  });
}
