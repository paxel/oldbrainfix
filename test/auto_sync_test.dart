import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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
import 'package:oldbrainfix/src/sync_line.dart';
import 'package:oldbrainfix/src/sync_runner.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announced.dart';
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

  /// The port the paired host is advertised on, while it is.
  int? advertised;

  /// How often the periodic work heard that its sync is over.
  var finishes = 0;

  /// The device id it is advertised under.
  String? advertisedAs;

  @override
  Future<void> advertise({required String device, required int port}) async {
    advertisedAs = device;
    advertised = port;
  }

  @override
  Future<void> unadvertise() async => advertised = null;

  @override
  Future<void> finished() async => finishes++;

  /// False as on the iPhone (#236).
  @override
  var hostsOutOfSight = true;
}

/// The paired host as the test says: started on a port, and the session
/// a paired Device's join would bring, handed in by the test.
class FakeHosts {
  var starts = 0;
  FakeHost? running;

  Future<PairedHosting> start(
    Store store, {
    required String name,
    required void Function(SyncSession session) onSession,
    PairedMerge? merging,
  }) async {
    starts++;
    return running = FakeHost(4700 + starts, onSession, merging);
  }
}

class FakeHost implements PairedHosting {
  @override
  final int port;
  final void Function(SyncSession session) onSession;
  final PairedMerge? merging;
  var stopped = false;

  FakeHost(this.port, this.onSession, this.merging);

  @override
  Future<void> stop() async => stopped = true;
}

/// Where a test's jobs run: here, the paired host's merges recorded.
class RecordingRunner extends InlineSyncRunner {
  final hosted = <(HostRequest, String)>[];

  RecordingRunner()
    : super(folders: const NoFolders(), joining: quickPairedSync);

  @override
  Future<(HostResponse, SyncSession?)> host(
    Store store,
    HostRequest request, {
    required String name,
  }) async {
    hosted.add((request, name));
    return (const HostResponse.status(204), null);
  }
}

void main() {
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  late Store phone;
  late Store desk;
  late FakeSync sync;
  final joined = <(String, int, String)>[];
  SyncFailure? failure;

  /// Holds the join until the test lets it go.
  Completer<void>? parked;

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
    if (parked case final held?) await held.future;
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

  AutoSync hostingAutoSync(FakeHosts hosts) => AutoSync(
    store: phone,
    platform: sync,
    deviceName: () async => 'Pixel',
    join: join,
    clock: () => now,
    host: hosts.start,
  );

  AlarmSync alarmSync() {
    final armed = AlarmSync(
      store: phone,
      platform: FakeAlarms(),
      wallClock: deviceWallClock,
    );
    addTearDown(armed.dispose);
    armed.setTexts(lookupAppLocalizations(const Locale('en')), 'en');
    return armed;
  }

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
    parked = null;
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

  group('hosting while the app is open', () {
    testWidgets('hosts for paired Devices while it shows, says so on the '
        'network, and stops on leaving', (tester) async {
      pairDesk();
      final hosts = FakeHosts();
      final auto = hostingAutoSync(hosts);
      auto.shown();
      await tester.pump();
      expect(hosts.starts, 1);
      expect(sync.advertised, 4701);
      expect(sync.advertisedAs, phone.deviceId);
      auto.devicesChanged();
      await tester.pump();
      expect(hosts.starts, 1, reason: 'one host at a time');

      auto.hidden();
      await tester.pump();
      expect(hosts.running?.stopped, isTrue);
      expect(sync.advertised, isNull);

      // Nobody syncs on their own any more: nothing to host for.
      phone.setAutoSync(desk.deviceId, false);
      auto.shown();
      await tester.pump();
      expect(hosts.starts, 1);
      auto.dispose();
    });

    testWidgets('a joiner\'s /sync merges where the runner runs the joins '
        '(#225)', (tester) async {
      pairDesk();
      final hosts = FakeHosts();
      final runner = RecordingRunner();
      final auto = AutoSync(
        store: phone,
        platform: sync,
        deviceName: () async => 'Pixel',
        join: join,
        clock: () => now,
        host: hosts.start,
        runner: runner,
      );
      auto.shown();
      await tester.pump();
      final request = HostRequest(
        method: 'POST',
        path: '/sync',
        body: Uint8List(0),
      );
      final (response, session) = await hosts.running!.merging!(request);
      expect(response.status, 204);
      expect(session, isNull);
      expect(runner.hosted.single.$1, same(request));
      expect(runner.hosted.single.$2, 'Pixel');
      auto.dispose();
    });

    testWidgets('away from Wi-Fi, nothing is hosted', (tester) async {
      pairDesk();
      sync.localNetwork = false;
      final hosts = FakeHosts();
      final auto = hostingAutoSync(hosts);
      auto.shown();
      await tester.pump();
      expect(hosts.starts, 0);
      expect(sync.advertised, isNull);
      auto.dispose();
    });

    testWidgets('the periodic work out of sight hosts and looks for its '
        'window, tries again, then says it is over', (tester) async {
      pairDesk();
      final hosts = FakeHosts();
      final auto = hostingAutoSync(hosts);
      auto.shown();
      await tester.pump();
      auto.hidden();
      await tester.pump();
      final before = joined.length;
      failure = SyncFailure.unreachable;

      sync.want();
      await tester.pump();
      expect(hosts.starts, 2);
      expect(sync.advertised, 4702);
      expect(sync.discovering, isTrue);
      expect(joined, hasLength(before + 1));

      await wait(tester, retryEvery + const Duration(seconds: 1));
      expect(joined, hasLength(before + 2), reason: 'tried again');
      expect(sync.finishes, 0);

      await wait(tester, hostWindow);
      expect(sync.finishes, 1);
      expect(hosts.running?.stopped, isTrue);
      expect(sync.advertised, isNull);
      expect(sync.discovering, isFalse);
      auto.dispose();
    });

    testWidgets('the window ends once every Device synced, also by joining '
        'this one', (tester) async {
      pairDesk();
      final hosts = FakeHosts();
      final auto = hostingAutoSync(hosts);
      failure = SyncFailure.unreachable;
      sync.want();
      await tester.pump();
      expect(hosts.starts, 1);
      hosts.running?.onSession(session());
      await wait(tester, const Duration(seconds: 1));
      expect(sync.finishes, 1);
      expect(hosts.running?.stopped, isTrue);
      auto.dispose();
    });

    testWidgets('while it shows, the periodic work is told at once', (
      tester,
    ) async {
      pairDesk();
      final hosts = FakeHosts();
      final auto = hostingAutoSync(hosts);
      auto.shown();
      await tester.pump();
      sync.want();
      await tester.pump();
      expect(sync.finishes, 1);
      expect(hosts.starts, 1);
      expect(hosts.running?.stopped, isFalse);
      auto.dispose();
    });
  });

  group('on the iPhone, hosting only while the app is open (#240)', () {
    testWidgets('the periodic work out of sight joins once without '
        'hosting, then says it is over', (tester) async {
      pairDesk();
      sync.hostsOutOfSight = false;
      final hosts = FakeHosts();
      final auto = hostingAutoSync(hosts);
      auto.shown();
      await tester.pump();
      expect(hosts.starts, 1, reason: 'hosts while it shows');
      auto.hidden();
      await tester.pump();
      expect(hosts.running?.stopped, isTrue, reason: 'stops on leaving');
      expect(sync.advertised, isNull);
      final before = joined.length;
      failure = SyncFailure.unreachable;
      parked = Completer<void>();

      sync.want();
      await tester.pump();
      expect(hosts.starts, 1, reason: 'no host out of sight');
      expect(sync.advertised, isNull);
      expect(joined, hasLength(before + 1));
      expect(sync.finishes, 0, reason: 'the join still runs');

      parked?.complete();
      await wait(tester, const Duration(milliseconds: 200));
      expect(sync.finishes, 1);
      await wait(tester, retryEvery + const Duration(seconds: 1));
      expect(joined, hasLength(before + 1), reason: 'not tried again');
      expect(sync.discovering, isFalse);
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
        'Something was changed on both Devices. Tap to pick which version '
        'stays.';

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
      // Which Device behind the tap.
      await expectFailureAnnounced(
        tester,
        announcements,
        'An update is needed to sync.',
        title: 'Update needed',
        cause:
            'Old Brain Fix on Desk needs an update to sync with this Device.',
        fix:
            'Install the latest Old Brain Fix on the Device named, then sync '
            'again.',
      );
      // The periodic work asks again.
      sync.want();
      await tester.pumpAndSettle();
      expect(joined, hasLength(2), reason: 'tried again');
      expect(announcements.visible, isNull);
    });

    testWidgets('a setting a newer version saved is kept and said once '
        '(ADR 0034)', (tester) async {
      const later = '{"generation":3,"v":2}';
      phone
        ..setLocalSetting('folderRead:x', later)
        ..forgetFolderReads();
      expect(phone.localSetting('folderRead:x'), later);
      await start(tester);
      await expectFailureAnnounced(
        tester,
        announcements,
        'An update of Old Brain Fix is needed.',
        title: 'Update needed',
        cause:
            'A setting a newer Old Brain Fix saved on this Device is kept as '
            'it is.',
        fix: 'Install the latest Old Brain Fix on this Device.',
      );
    });

    testWidgets('while a sync runs, a line moves under the bar and goes when '
        'it ends; a screen reader hears "Syncing" once (#225)', (tester) async {
      final spoken = <String>[];
      tester.binding.defaultBinaryMessenger
          .setMockDecodedMessageHandler<dynamic>(SystemChannels.accessibility, (
            message,
          ) async {
            final data = (message as Map)['data'] as Map?;
            if (message['type'] == 'announce') {
              spoken.add('${data?['message']}');
            }
            return null;
          });
      addTearDown(
        () => tester.binding.defaultBinaryMessenger
            .setMockDecodedMessageHandler<dynamic>(
              SystemChannels.accessibility,
              null,
            ),
      );
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(accessibleNavigation: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      pairDesk();
      parked = Completer<void>();
      await tester.pumpWidget(
        OldBrainFixApp(
          store: phone,
          announcements: announcements,
          sync: sync,
          autoJoin: join,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(joined, hasLength(1));
      final moving = find.byKey(const Key('syncLineMoving'));
      expect(moving, findsOneWidget);
      // Under the Agenda's bar, across its width.
      final bar = tester.getRect(find.byType(AppBar));
      final line = tester.getRect(
        find.ancestor(of: moving, matching: find.byType(SizedBox)).first,
      );
      expect(line.bottom, bar.bottom);
      expect(line.width, bar.width);
      // It moves while the app keeps drawing.
      final before = tester.getRect(moving).left;
      await tester.pump(syncLineSweep ~/ 2);
      expect(tester.getRect(moving).left, isNot(before));
      expect(spoken, ['Syncing…']);
      parked!.complete();
      parked = null;
      await tester.pumpAndSettle();
      expect(moving, findsNothing);
      // The next sync shows the line again, but says nothing.
      parked = Completer<void>();
      sync.want();
      await tester.pump();
      await tester.pump();
      expect(joined, hasLength(2));
      expect(moving, findsOneWidget);
      parked!.complete();
      await tester.pumpAndSettle();
      expect(moving, findsNothing);
      expect(spoken, ['Syncing…']);
    });

    testWidgets('with "remove animations" the line stands still (#225)', (
      tester,
    ) async {
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(disableAnimations: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      pairDesk();
      parked = Completer<void>();
      await tester.pumpWidget(
        OldBrainFixApp(
          store: phone,
          announcements: announcements,
          sync: sync,
          autoJoin: join,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(find.byKey(const Key('syncLineMoving')), findsNothing);
      final still = find.byKey(const Key('syncLineStill'));
      expect(still, findsOneWidget);
      expect(
        tester.getRect(still).width,
        tester.getRect(find.byType(AppBar)).width,
      );
      parked!.complete();
      await tester.pumpAndSettle();
      expect(still, findsNothing);
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
        window: Duration.zero,
      );
      expect(joined, hasLength(2));
    });
  });

  group('hosting in the background', () {
    testWidgets('hosts for its window, tries again, and stops once every '
        'Device synced', (tester) async {
      pairDesk();
      final hosts = FakeHosts();
      failure = SyncFailure.unreachable;
      final run = backgroundSync(
        store: phone,
        platform: sync,
        alarms: alarmSync(),
        name: 'Pixel',
        join: join,
        host: hosts.start,
        clock: () => now,
      );
      await tester.pump();
      expect(sync.advertised, 4701);
      expect(sync.advertisedAs, phone.deviceId);
      expect(joined, hasLength(1));

      failure = null;
      await wait(tester, retryEvery);
      expect(joined, hasLength(2));
      await run;
      expect(hosts.running?.stopped, isTrue);
      expect(sync.advertised, isNull);
    });

    testWidgets('a Device that only joins this one is waited for until it '
        'did', (tester) async {
      phone.rememberDevice(
        PairedDevice(
          id: desk.deviceId,
          name: 'Tablet',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: DateTime.utc(2026, 10, 24, 6),
        ),
      );
      final hosts = FakeHosts();
      var over = false;
      final run = backgroundSync(
        store: phone,
        platform: sync,
        alarms: alarmSync(),
        name: 'Pixel',
        join: join,
        host: hosts.start,
        clock: () => now,
      ).then((_) => over = true);
      await tester.pump();
      expect(joined, isEmpty, reason: 'nowhere to join it');
      await wait(tester, retryEvery);
      expect(over, isFalse);

      hosts.running?.onSession(session());
      await wait(tester, retryEvery);
      await run;
      expect(hosts.running?.stopped, isTrue);
    });

    testWidgets('gives up when the window is over', (tester) async {
      pairDesk();
      final hosts = FakeHosts();
      failure = SyncFailure.unreachable;
      var over = false;
      final run = backgroundSync(
        store: phone,
        platform: sync,
        alarms: alarmSync(),
        name: 'Pixel',
        join: join,
        host: hosts.start,
        clock: () => now,
      ).then((_) => over = true);
      await tester.pump();
      for (var i = 0; i < 8; i++) {
        await wait(tester, retryEvery);
      }
      await run;
      expect(over, isTrue);
      expect(joined, hasLength(9));
      expect(hosts.running?.stopped, isTrue);
      expect(sync.advertised, isNull);
    });
  });

  group('in the background on the iPhone (#240)', () {
    testWidgets('joins once and hosts nothing', (tester) async {
      pairDesk();
      sync.hostsOutOfSight = false;
      final hosts = FakeHosts();
      failure = SyncFailure.unreachable;
      await backgroundSync(
        store: phone,
        platform: sync,
        alarms: alarmSync(),
        name: 'Pixel',
        join: join,
        host: hosts.start,
        clock: () => now,
      );
      expect(hosts.starts, 0);
      expect(sync.advertised, isNull);
      expect(joined, hasLength(1));
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
