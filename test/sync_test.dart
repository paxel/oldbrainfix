import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';

/// Reads the QR the test says is in the photo.
class FakePairing implements PairingPlatform {
  String? code;
  Uint8List? read;

  @override
  Future<String?> readQr(Uint8List photo) async {
    read = photo;
    return code;
  }

  @override
  Future<String> deviceName() async => 'Pixel';
}

/// A code for a computer at 192.168.1.23.
final code = encodePairCode(
  '192.168.1.23',
  45231,
  '042817',
  List.filled(32, 7),
  typed: true,
);

void main() {
  late Store phone;
  late Store desk;
  late FakePairing pairing;
  late FakePhotos photos;
  late Announcements announcements;
  final joined = <(PairInfo, String)>[];
  SyncFailure? failure;
  Object? broken;

  setUp(() {
    phone = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    desk = Store.inMemory();
    Notes(desk).capture('From the desk');
    pairing = FakePairing();
    photos = FakePhotos();
    announcements = Announcements();
    joined.clear();
    failure = null;
    broken = null;
  });
  tearDown(() {
    phone.close();
    desk.close();
  });

  /// The LAN session, run against [desk] without the network.
  Future<SyncResult> join(
    Store store,
    PairInfo info, {
    required String name,
  }) async {
    joined.add((info, name));
    if (failure case final failed?) throw SyncException(failed);
    if (broken case final error?) throw error;
    final done = answerSync(
      desk,
      syncRequest(store, desk.versionVector(), name: name),
      name: 'Desk',
    )!;
    final session = applyAnswer(store, done.answer);
    store.rememberDevice(
      PairedDevice(
        id: desk.deviceId,
        name: 'Desk',
        fingerprint: '',
        host: info.host,
        port: info.port,
        lastSync: DateTime.utc(2026, 3, 1, 9),
      ),
    );
    return SyncResult(
      session: session,
      entriesSent: 0,
      blobsSent: 0,
      blobsReceived: 0,
    );
  }

  Future<void> open(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => AnnouncementStrip(
          announcements: announcements,
          child: AnnouncementsScope(
            announcements: announcements,
            child: PhotoScope(
              photos: photos,
              bytes: (_) => null,
              child: PairingScope(
                pairing: pairing,
                child: child ?? const SizedBox.shrink(),
              ),
            ),
          ),
        ),
        home: SyncScreen(store: phone, join: join),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('a typed code syncs, says what came in, and pairs', (
    tester,
  ) async {
    await open(tester);
    expect(find.text('No paired Devices yet.'), findsOneWidget);
    await tester.enterText(find.byKey(const ValueKey('code')), code);
    await tester.tap(find.byKey(const ValueKey('join')));
    await tester.pumpAndSettle();
    final (info, name) = joined.single;
    expect((info.host, info.port, info.pin), ('192.168.1.23', 45231, '042817'));
    expect(name, 'Pixel');
    expect(announcements.visible, startsWith('Synced with Desk: '));
    expect(find.text(announcements.visible!), findsOneWidget);
    expect([for (final n in Notes(phone).all()) n.text], ['From the desk']);
    expect(find.text('Desk'), findsOneWidget);

    await tester.tap(find.text('Forget'));
    await tester.pumpAndSettle();
    expect(phone.pairedDevices(), isEmpty);
    expect(find.text('No paired Devices yet.'), findsOneWidget);
  });

  testWidgets('what is no code is said so and nothing is tried', (
    tester,
  ) async {
    await open(tester);
    await tester.enterText(find.byKey(const ValueKey('code')), 'hello');
    await tester.tap(find.byKey(const ValueKey('join')));
    await tester.pumpAndSettle();
    expect(joined, isEmpty);
    expect(
      announcements.visible,
      "That is not a pair code. Check it against the computer's screen.",
    );
  });

  testWidgets('a refused sync says why at the top', (tester) async {
    failure = SyncFailure.wrongPin;
    await open(tester);
    await tester.enterText(find.byKey(const ValueKey('code')), code);
    await tester.tap(find.byKey(const ValueKey('join')));
    await tester.pumpAndSettle();
    expect(
      announcements.visible,
      'The PIN does not match. Type the code again.',
    );
    expect(phone.allEntries(), isEmpty);
  });

  testWidgets('anything else that breaks is said too', (tester) async {
    broken = StateError('cut off');
    await open(tester);
    await tester.enterText(find.byKey(const ValueKey('code')), code);
    await tester.tap(find.byKey(const ValueKey('join')));
    await tester.pumpAndSettle();
    expect(
      announcements.visible,
      'Sync broke off. Nothing was lost; try again.',
    );
    expect(find.byKey(const ValueKey('join')), findsOneWidget);
  });

  testWidgets('scanning reads the code in a photo of the screen', (
    tester,
  ) async {
    photos.nextPhoto = onePixelPng;
    pairing.code = code;
    await open(tester);
    await tester.tap(find.byKey(const ValueKey('scan')));
    await tester.pumpAndSettle();
    expect(pairing.read, onePixelPng);
    expect(joined.single.$1.pin, '042817');
    expect(announcements.visible, startsWith('Synced with Desk: '));
  });

  testWidgets('a photo without a code says so', (tester) async {
    photos.nextPhoto = onePixelPng;
    await open(tester);
    await tester.tap(find.byKey(const ValueKey('scan')));
    await tester.pumpAndSettle();
    expect(joined, isEmpty);
    expect(announcements.visible, startsWith('No code found in the photo.'));
  });

  testWidgets('Sync opens from Settings', (tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: phone, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Settings'));
    await tester.pumpAndSettle();
    final entry = find.byKey(const ValueKey('sync'));
    await tester.scrollUntilVisible(
      entry,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.tap(entry);
    await tester.pumpAndSettle();
    expect(find.byType(SyncScreen), findsOneWidget);
  });

  testWidgets('the Starter Set is not offered once Areas arrived by sync', (
    tester,
  ) async {
    final fresh = Store.inMemory()..alarmSetupDone = true;
    addTearDown(fresh.close);
    Places(desk).plantStarterSet({'work'});
    fresh.applyEntries(desk.entriesSince(const {}), keys: [desk.ownKeyRecord]);
    await tester.pumpWidget(OldBrainFixApp(store: fresh, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    expect(find.byType(StarterSetScreen), findsNothing);
  });
}
