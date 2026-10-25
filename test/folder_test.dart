import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/auto_sync.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'auto_sync_test.dart' show FakeSync;
import 'fake_alarms.dart';
import 'fake_photos.dart';

/// One shared folder in memory, picked as the test says.
class FakeFolders implements FolderPlatform {
  final folder = MemorySyncFolder();

  /// What the picker answers; null backs out.
  String? picks = 'content://tree/primary%3ASync';
  var picked = 0;

  @override
  bool get available => true;

  @override
  Future<String?> pick() async {
    picked++;
    return picks;
  }

  @override
  Future<String> displayName(String folder) async => 'Syncthing';

  @override
  SyncFolder open(String folder) =>
      folder.isEmpty ? throw ArgumentError('no folder') : this.folder;
}

void main() {
  var now = DateTime(2026, 10, 24, 9);
  late Store phone;
  late Store desk;
  late FakeFolders folders;
  late FakeSync sync;

  /// One LAN session: Pairing hands the keys over.
  void pair() => applyAnswer(
    phone,
    answerSync(
      desk,
      syncRequest(phone, desk.versionVector(), name: 'Pixel'),
      name: 'Desk',
    )!.answer,
  );

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    phone = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    desk = Store.inMemory(clock: () => now);
    folders = FakeFolders();
    sync = FakeSync();
  });
  tearDown(() {
    phone.close();
    desk.close();
  });

  AutoSync autoSync() => AutoSync(
    store: phone,
    platform: sync,
    deviceName: () async => 'Pixel',
    folders: folders,
    clock: () => now,
  );

  Future<void> page(WidgetTester tester, AutoSync auto) async {
    // Tall enough for the whole page.
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
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
            child: FolderScope(
              folders: folders,
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        ),
        home: SyncScreen(store: phone),
      ),
    );
    await tester.pumpAndSettle();
  }

  group('the Sync page', () {
    testWidgets('a shared folder is picked, synced at once and forgotten', (
      tester,
    ) async {
      final doctor = Places(desk).addArea('Doctor');
      pair();
      Notes(desk).capture('Blood test', place: doctor.id);
      await folderSync(desk, folders.folder);
      final auto = autoSync();
      await page(tester, auto);
      expect(find.text('Shared folder'), findsOneWidget);

      // Backing out of the picker changes nothing.
      folders.picks = null;
      await tester.tap(find.byKey(const ValueKey('pick-folder')));
      await tester.pumpAndSettle();
      expect(phone.syncFolder, isNull);

      folders.picks = 'content://tree/primary%3ASync';
      await tester.tap(find.byKey(const ValueKey('pick-folder')));
      await tester.pumpAndSettle();
      expect(phone.syncFolder, 'content://tree/primary%3ASync');
      expect(find.text('Syncthing'), findsOneWidget);
      expect(find.textContaining('1 change came in'), findsNothing);
      expect(find.textContaining('changes came in'), findsOneWidget);
      expect(Notes(phone).all().map((n) => n.text), ['Blood test']);
      expect(sync.background, isTrue);
      expect(sync.folderBackground, isTrue);

      await tester.tap(find.byKey(const ValueKey('forget-folder')));
      await tester.pumpAndSettle();
      expect(phone.syncFolder, isNull);
      expect(find.byKey(const ValueKey('pick-folder')), findsOneWidget);
      expect(sync.background, isFalse);
      auto.dispose();
    });

    testWidgets('a Device never paired is told to pair first', (tester) async {
      Notes(desk).capture('Private');
      await folderSync(desk, folders.folder);
      final auto = autoSync();
      await page(tester, auto);
      await tester.tap(find.byKey(const ValueKey('pick-folder')));
      await tester.pumpAndSettle();
      expect(Notes(phone).all(), isEmpty);
      expect(find.textContaining('not paired with'), findsOneWidget);
      auto.dispose();
    });

    testWidgets('a folder that cannot be reached says so', (tester) async {
      phone.chooseSyncFolder('');
      await folderRound(phone, _Gone());
      final auto = autoSync();
      await page(tester, auto);
      expect(find.textContaining('could not be reached'), findsOneWidget);
      auto.dispose();
    });
  });

  group('automatic sync', () {
    testWidgets('the open app syncs through the folder at once and every '
        'minute', (tester) async {
      pair();
      phone.chooseSyncFolder('content://tree/x');
      final auto = autoSync();
      auto.shown();
      await tester.pump();
      expect(phone.folderStatus, isNotNull);
      expect(sync.folderBackground, isTrue);

      Notes(desk).capture('From the desk');
      await folderSync(desk, folders.folder);
      now = now.add(const Duration(minutes: 1));
      await tester.pump(const Duration(minutes: 1));
      expect(Notes(phone).all().map((n) => n.text), ['From the desk']);

      // A change here goes out 5 s later.
      Notes(phone).capture('From the phone');
      now = now.add(const Duration(seconds: 5));
      await tester.pump(const Duration(seconds: 5));
      await folderSync(desk, folders.folder);
      expect(Notes(desk).all().map((n) => n.text), contains('From the phone'));
      auto.dispose();
    });

    testWidgets('the background syncs the folder away from any Wi-Fi', (
      tester,
    ) async {
      pair();
      Notes(desk).capture('Done on the desk');
      await folderSync(desk, folders.folder);
      phone.chooseSyncFolder('content://tree/x');
      sync.localNetwork = false;
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
        folders: folders,
      );
      expect(Notes(phone).all().map((n) => n.text), ['Done on the desk']);
      expect(sync.looked, isNull, reason: 'no paired host to look for');
    });
  });
}

/// A folder that is gone.
class _Gone extends MemorySyncFolder {
  @override
  Future<void> ensure(String dir) async =>
      throw const FileSystemException('gone');
}
