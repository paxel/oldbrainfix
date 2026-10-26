import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/auto_sync.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/sync_platform.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';

/// Discovery and the periodic work as the test says.
class FakeSync implements SyncPlatform {
  final _found = StreamController<FoundHost>.broadcast();
  final _wanted = StreamController<void>.broadcast();
  var discovering = false;
  bool? background;

  /// What a background look finds, and how long it was asked to look.
  var nearby = <FoundHost>[];
  Duration? looked;

  void find(FoundHost host) => _found.add(host);
  void want() => _wanted.add(null);

  @override
  Stream<FoundHost> get found => _found.stream;

  @override
  Stream<void> get wanted => _wanted.stream;

  @override
  Future<void> startDiscovery() async => discovering = true;

  @override
  Future<void> stopDiscovery() async => discovering = false;

  @override
  Future<List<FoundHost>> discover(Duration within) async {
    looked = within;
    return nearby;
  }

  /// Whether the background work runs for a shared folder too.
  bool? folderBackground;

  /// Whether the Device is on Wi-Fi or Ethernet.
  var localNetwork = true;

  @override
  Future<void> setBackground(bool on, {bool folder = false}) async {
    background = on;
    folderBackground = folder;
  }

  @override
  Future<bool> onLocalNetwork() async => localNetwork;
}

void main() {
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  late Store phone;
  late Store desk;
  late FakeSync sync;
  final joined = <(String, int, String)>[];
  SyncFailure? failure;

  /// One session between the phone and the desk, as LAN sync runs it.
  SyncSession session() {
    final done = answerSync(
      desk,
      syncRequest(phone, desk.versionVector(), name: 'Pixel'),
      name: 'Desk',
    )!;
    return applyAnswer(phone, done.answer);
  }

  /// The automatic sync, run against [desk] without the network.
  Future<SyncResult> join(
    Store store,
    PairedDevice device, {
    required String host,
    required int port,
    required String name,
  }) async {
    joined.add((host, port, name));
    if (failure case final failed?) throw SyncException(failed);
    return SyncResult(
      session: session(),
      entriesSent: 0,
      blobsSent: 0,
      blobsReceived: 0,
    );
  }

  void pairDesk() => phone.rememberDevice(
    PairedDevice(
      id: desk.deviceId,
      name: 'Desk',
      fingerprint: 'ab' * 32,
      host: '192.168.1.5',
      port: 4000,
      lastSync: DateTime.utc(2026, 10, 24, 6),
    ),
  );

  AutoSync autoSync() => AutoSync(
    store: phone,
    platform: sync,
    deviceName: () async => 'Pixel',
    join: join,
    clock: () => now,
  );

  /// Lets [by] pass on the clock and the timers alike.
  Future<void> wait(WidgetTester tester, Duration by) async {
    now = now.add(by);
    await tester.pump(by);
  }

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    phone = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    desk = Store.inMemory(clock: () => now);
    sync = FakeSync();
    joined.clear();
    failure = null;
  });
  tearDown(() {
    phone.close();
    desk.close();
  });

  group('the Sync page', () {
    testWidgets('a paired computer syncs on its own until switched off', (
      tester,
    ) async {
      pairDesk();
      final auto = autoSync();
      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) => PhotoScope(
            photos: FakePhotos(),
            bytes: (_) => null,
            child: AutoSyncScope(
              sync: auto,
              child: child ?? const SizedBox.shrink(),
            ),
          ),
          home: SyncScreen(store: phone),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining('sync on their own'), findsOneWidget);
      expect(find.textContaining('Last synced'), findsOneWidget);
      final toggle = find.byKey(ValueKey('auto-${desk.deviceId}'));
      await tester.scrollUntilVisible(
        toggle,
        200,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(tester.widget<Switch>(toggle).value, isTrue);

      await tester.tap(toggle);
      await tester.pumpAndSettle();
      expect(phone.autoSyncOn(desk.deviceId), isFalse);
      expect(tester.widget<Switch>(toggle).value, isFalse);
      expect(sync.background, isFalse, reason: 'nobody to sync with');

      await tester.tap(toggle);
      await tester.pumpAndSettle();
      expect(phone.autoSyncOn(desk.deviceId), isTrue);
      expect(sync.background, isTrue);
      auto.dispose();
    });
  });

  group('while the app is open', () {
    testWidgets('syncs at once, where the computer was last, and keeps '
        'looking', (tester) async {
      pairDesk();
      Notes(desk).capture('From the desk');
      final auto = autoSync();
      auto.shown();
      await tester.pump();
      expect(joined, [('192.168.1.5', 4000, 'Pixel')]);
      expect(Notes(phone).all().map((n) => n.text), ['From the desk']);
      expect(sync.discovering, isTrue);
      expect(sync.background, isTrue);

      // Every minute while open, so a Done on the desk arrives soon.
      await wait(tester, const Duration(seconds: 59));
      expect(joined, hasLength(1));
      await wait(tester, const Duration(seconds: 1));
      expect(joined, hasLength(2));
      auto.dispose();
    });

    testWidgets('a computer showing up is synced where it was found', (
      tester,
    ) async {
      pairDesk();
      final auto = autoSync();
      failure = SyncFailure.unreachable;
      auto.shown();
      await tester.pump();
      expect(joined, hasLength(1));

      failure = null;
      sync.find(FoundHost(desk.deviceId, '192.168.1.9', 4100));
      await tester.pump();
      expect(joined.last, ('192.168.1.9', 4100, 'Pixel'));
      // Discovery repeats itself: not again right after.
      sync.find(FoundHost(desk.deviceId, '192.168.1.9', 4100));
      await tester.pump();
      expect(joined, hasLength(2));
      // Somebody else's host is none of its business.
      sync.find(const FoundHost('stranger', '192.168.1.7', 4000));
      await tester.pump();
      expect(joined, hasLength(2));
      auto.dispose();
    });

    testWidgets('a change here goes out 5 s after the last one; what came '
        'in sends nothing', (tester) async {
      pairDesk();
      final auto = autoSync();
      auto.shown();
      await tester.pump();
      expect(joined, hasLength(1));

      Notes(phone).capture('One');
      await wait(tester, const Duration(seconds: 3));
      Notes(phone).capture('Two');
      await wait(tester, const Duration(seconds: 3));
      expect(joined, hasLength(1));
      await wait(tester, const Duration(seconds: 2));
      expect(joined, hasLength(2));
      expect(Notes(desk).all(), hasLength(2));

      // What the desk wrote comes in with the next sync and sends none.
      Notes(desk).capture('From the desk');
      sync.want();
      await tester.pump();
      expect(joined, hasLength(3));
      await wait(tester, const Duration(seconds: 10));
      expect(joined, hasLength(3));
      auto.dispose();
    });

    testWidgets('the first change goes out 5 s later even when nothing came '
        'in before', (tester) async {
      pairDesk();
      final auto = autoSync();
      failure = SyncFailure.unreachable;
      auto.shown();
      await tester.pump();
      expect(joined, hasLength(1));
      failure = null;
      Notes(phone).capture('First thing');
      await wait(tester, const Duration(seconds: 5));
      expect(joined, hasLength(2));
      expect(Notes(desk).all().map((n) => n.text), ['First thing']);
      auto.dispose();
    });

    testWidgets('leaving sends what changed at once and stops looking', (
      tester,
    ) async {
      pairDesk();
      final auto = autoSync();
      auto.shown();
      await tester.pump();
      Notes(phone).capture('Just before leaving');
      auto.hidden();
      await tester.pump();
      expect(joined, hasLength(2));
      expect(Notes(desk).all().map((n) => n.text), ['Just before leaving']);
      expect(sync.discovering, isFalse);
      // Nothing more until the app shows again.
      await wait(tester, const Duration(minutes: 5));
      expect(joined, hasLength(2));
      auto.dispose();
    });

    testWidgets('a computer switched off, or one that never hosted, is left '
        'alone', (tester) async {
      pairDesk();
      phone.setAutoSync(desk.deviceId, false);
      phone.rememberDevice(
        PairedDevice(
          id: 'joined-only',
          name: 'Laptop',
          fingerprint: '',
          host: '192.168.1.6',
          port: 4000,
          lastSync: DateTime.utc(2026),
        ),
      );
      final auto = autoSync();
      auto.shown();
      await tester.pump();
      Notes(phone).capture('Stays here');
      await wait(tester, const Duration(minutes: 2));
      expect(joined, isEmpty);
      expect(sync.background, isFalse);
      auto.dispose();
    });
  });

  group('the whole app', () {
    late Announcements announcements;
    setUp(() => announcements = Announcements());

    Future<void> start(WidgetTester tester) async {
      await tester.pumpWidget(
        OldBrainFixApp(
          store: phone,
          announcements: announcements,
          sync: sync,
          autoJoin: join,
        ),
      );
      await tester.pumpAndSettle();
    }

    /// The same Note edited apart on both Devices.
    String editApart() {
      final note = Notes(desk).capture('nees mor sauce');
      session();
      Notes(phone).edit(note.id, 'From the phone');
      Notes(desk).edit(note.id, 'Edited at the desk');
      return note.id;
    }

    const found =
        '1 thing was changed on both Devices. Tap to pick which version stays.';

    testWidgets('Conflicts a sync brought are announced at the top and lead '
        'to them', (tester) async {
      pairDesk();
      editApart();
      await start(tester);
      expect(joined, hasLength(1));
      expect(announcements.visible, found);
      await tester.tap(find.text(found));
      await tester.pumpAndSettle();
      expect(find.byType(ConflictsScreen), findsOneWidget);
      expect(find.text('Edited at the desk'), findsOneWidget);
    });

    testWidgets('a computer that needs an update is announced at the top '
        'once, not at every try (ADR 0019)', (tester) async {
      pairDesk();
      failure = SyncFailure.updateThere;
      await start(tester);
      expect(joined, hasLength(1));
      const needed =
          'Old Brain Fix on Desk needs an update to sync with this Device.';
      expect(announcements.visible, needed);
      announcements.dismiss();
      // The periodic work asks again.
      sync.want();
      await tester.pumpAndSettle();
      expect(joined, hasLength(2), reason: 'tried again');
      expect(announcements.visible, isNull);
    });

    testWidgets('Conflicts a background sync brought are announced when the '
        'app opens', (tester) async {
      editApart();
      session();
      phone.conflictsArrived = true;
      await start(tester);
      expect(joined, isEmpty);
      expect(announcements.visible, found);
      expect(phone.conflictsArrived, isFalse);
    });
  });

  group('in the background', () {
    testWidgets('a Done tapped here goes out, and a Done on the computer '
        'silences the alarm here', (tester) async {
      final pills = Routines(phone).create(
        'pills',
        Schedule(
          start: Day(2026, 10, 1),
          times: [ClockTime(8, 0), ClockTime(20, 0)],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      pairDesk();
      session();
      final alarms = FakeAlarms();
      final tapped = DateTime.utc(2026, 10, 24, 6, 30);
      alarms.pending.add(DoneTapped(3, pills.id, '2026-10-24T08:00', tapped));
      Alarms(desk).applyDone(pills.id, '2026-10-24T20:00', now.toUtc());
      sync.nearby = [FoundHost(desk.deviceId, '192.168.1.9', 4100)];
      final armed = AlarmSync(
        store: phone,
        platform: alarms,
        wallClock: deviceWallClock,
      );
      addTearDown(armed.dispose);
      armed.setTexts(lookupAppLocalizations(const Locale('en')), 'en');

      await backgroundSync(
        store: phone,
        platform: sync,
        alarms: armed,
        name: 'Pixel',
        join: join,
      );
      expect(joined, [('192.168.1.9', 4100, 'Pixel')]);
      final today = Day(2026, 10, 24);
      expect(
        Routines(desk)
            .byId(pills.id)
            ?.doneAt(Occurrence(today, ClockTime(8, 0))),
        tapped,
      );
      expect(alarms.acknowledged, 3);
      expect(
        alarms.ids.where((id) => id.contains('@2026-10-24T')),
        isEmpty,
        reason: 'both of today are Done',
      );
      expect(alarms.ids.first, 'occurrence:${pills.id}@2026-10-25T08:00');
      expect(phone.conflictsArrived, isFalse);
    });

    testWidgets('Conflicts it brought wait for the app; a computer that is '
        'not there is no trouble', (tester) async {
      pairDesk();
      final note = Notes(desk).capture('nees mor sauce');
      session();
      Notes(phone).edit(note.id, 'From the phone');
      Notes(desk).edit(note.id, 'Edited at the desk');
      final armed = AlarmSync(
        store: phone,
        platform: FakeAlarms(),
        wallClock: deviceWallClock,
      );
      addTearDown(armed.dispose);
      armed.setTexts(lookupAppLocalizations(const Locale('en')), 'en');
      await backgroundSync(
        store: phone,
        platform: sync,
        alarms: armed,
        name: 'Pixel',
        join: join,
      );
      expect(joined.single.$1, '192.168.1.5', reason: 'where it was last');
      expect(phone.conflictsArrived, isTrue);

      failure = SyncFailure.unreachable;
      await backgroundSync(
        store: phone,
        platform: sync,
        alarms: armed,
        name: 'Pixel',
        join: join,
      );
      expect(joined, hasLength(2));
    });
  });

  group('weekly Backups', () {
    const cheap = BackupKdf(memory: 64, iterations: 1);

    test('keep the background work on without a network, and one due is '
        'written when the app shows', () async {
      final folder = Directory.systemTemp.createTempSync('weekly_');
      addTearDown(() => folder.deleteSync(recursive: true));
      await setUpBackups(phone, 'correct horse', kdf: cheap);
      phone.backupFolder = folder.path;
      final auto = AutoSync(
        store: phone,
        platform: sync,
        deviceName: () async => 'Pixel',
        join: join,
        clock: () => now,
        folders: const NoFolders(),
        workDir: Directory.systemTemp.path,
      );
      addTearDown(auto.dispose);
      auto.devicesChanged();
      expect(sync.background, isTrue);
      expect(sync.folderBackground, isTrue);

      auto.shown();
      await auto.backUpWhenDue();
      expect(folder.listSync(), hasLength(1));
      expect(phone.lastBackup, now.toUtc());
    });

    test('the background work writes one when it is due, and only '
        'then', () async {
      final folder = Directory.systemTemp.createTempSync('weekly_');
      addTearDown(() => folder.deleteSync(recursive: true));
      await setUpBackups(phone, 'correct horse', kdf: cheap);
      phone.backupFolder = folder.path;
      final armed = AlarmSync(
        store: phone,
        platform: FakeAlarms(),
        wallClock: deviceWallClock,
      );
      addTearDown(armed.dispose);
      armed.setTexts(lookupAppLocalizations(const Locale('en')), 'en');
      Future<void> run() => backgroundSync(
        store: phone,
        platform: sync,
        alarms: armed,
        name: 'Pixel',
        join: join,
        workDir: Directory.systemTemp.path,
      );
      await run();
      expect(folder.listSync(), hasLength(1));
      now = now.add(const Duration(days: 3));
      await run();
      expect(folder.listSync(), hasLength(1));
      now = now.add(const Duration(days: 4));
      await run();
      expect(folder.listSync(), hasLength(2));
    });
  });
}
