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
import 'package:oldbrainfix/src/ocr_platform.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/profile_platform.dart';
import 'package:oldbrainfix/src/speech_platform.dart';
import 'package:oldbrainfix/src/sync_platform.dart';
import 'package:oldbrainfix/src/voice_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';
import 'open_settings.dart';

/// Every method channel the app talks to. On the iPhone none of them
/// answers until its ticket brings the Swift side (#237): each call
/// fails with a [MissingPluginException], as here, where no handler is
/// set.
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
