import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/backup_screen.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/restore_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_photos.dart';

/// A stand-in for Argon2id, quick in a widget test: the real one is the
/// core's to test.
Future<Uint8List> quickKey(
  String passphrase,
  BackupKdf kdf,
  List<int> salt,
) async {
  final seed = utf8
      .encode(passphrase.trim())
      .fold(7, (a, b) => (a * 31 + b) & 0xffffff);
  return Uint8List.fromList([
    for (var i = 0; i < 32; i++)
      (seed >> (i % 3 * 8)) + salt[i % salt.length] + i & 0xff,
  ]);
}

/// Backups go into a directory the test made; the picker answers it.
class DirFolders implements FolderPlatform {
  final String path;
  var picked = 0;

  DirFolders(this.path);

  @override
  bool get available => true;

  @override
  Future<String?> pick() async {
    picked++;
    return path;
  }

  @override
  Future<String> displayName(String folder) async => 'Backups';

  @override
  SyncFolder open(String folder) => LocalSyncFolder(folder);

  @override
  BackupFolder openBackups(String folder) => LocalBackupFolder(folder);
}

/// Reads the QR the test says is in the photo.
class FakePairing implements PairingPlatform {
  String? code;

  @override
  Future<String?> readQr(Uint8List photo) async => code;

  @override
  Future<String> deviceName() async => 'Pixel';
}

void main() {
  final now = DateTime.utc(2026, 10, 24, 9);
  late Directory dir;
  late FakeBundles bundles;
  late Announcements announcements;

  setUp(() {
    dir = Directory.systemTemp.createTempSync('backup_widget_');
    bundles = FakeBundles(dir);
    announcements = Announcements();
  });
  tearDown(() {
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  /// A Backup of a Device with a Note in an Area, and its recovery key.
  Future<(String, String)> backup() async {
    final old = Store.inMemory(clock: () => now);
    addTearDown(old.close);
    final area = Places(old).addArea('Doctor');
    Notes(old).capture('Pills at eight', place: area.id);
    final code = await setUpBackups(old, 'correct horse', derive: quickKey);
    final path = '${dir.path}/old.$backupExtension';
    await writeBackup(old, path);
    return (path, code);
  }

  /// A copy of [path], as the platform hands over what was picked.
  String copy(String path) {
    final to = '${dir.path}/picked-${DateTime.now().microsecondsSinceEpoch}';
    File(path).copySync(to);
    return to;
  }

  Future<void> page(
    WidgetTester tester,
    Widget home, {
    FolderPlatform folders = const NoFolders(),
    PhotoPlatform? photos,
    PairingPlatform? pairing,
  }) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => AnnouncementsScope(
          announcements: announcements,
          child: PhotoScope(
            photos: photos ?? FakePhotos(),
            bytes: (_) => null,
            child: PairingScope(
              pairing: pairing ?? FakePairing(),
              child: FolderScope(
                folders: folders,
                child: BundleScope(
                  bundles: bundles,
                  child: child ?? const SizedBox.shrink(),
                ),
              ),
            ),
          ),
        ),
        home: home,
      ),
    );
    await tester.pumpAndSettle();
  }

  group('the Backups page', () {
    testWidgets('sets Backups up with a passphrase typed twice and offers '
        'the Recovery Sheet at once', (tester) async {
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      await page(tester, BackupScreen(store: store, derive: quickKey));

      await tester.enterText(find.byKey(const ValueKey('passphrase')), 'short');
      await tester.tap(find.byKey(const ValueKey('set-up')));
      await tester.pumpAndSettle();
      expect(find.text('At least 8 characters, please.'), findsOneWidget);

      await tester.enterText(
        find.byKey(const ValueKey('passphrase')),
        'correct horse',
      );
      await tester.enterText(
        find.byKey(const ValueKey('passphrase-again')),
        'correct hose',
      );
      await tester.tap(find.byKey(const ValueKey('set-up')));
      await tester.pumpAndSettle();
      expect(find.text('The two do not match.'), findsOneWidget);
      expect(store.backupsSetUp, isFalse);

      await tester.enterText(
        find.byKey(const ValueKey('passphrase-again')),
        'correct horse',
      );
      await tester.tap(find.byKey(const ValueKey('set-up')));
      await tester.pumpAndSettle();
      expect(store.backupsSetUp, isTrue);
      expect(find.text('Recovery Sheet'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('sheet-print')));
      await tester.pumpAndSettle();
      final sheet = File(bundles.printed.single).readAsStringSync();
      expect(sheet, startsWith('%PDF-1.4'));
      final groups = store.recoveryCode!.split('-');
      expect(sheet, contains('(${groups.take(7).join('-')}) Tj'));
      expect(sheet, contains('Device: Pixel'));

      await tester.tap(find.byKey(const ValueKey('sheet-share')));
      await tester.pumpAndSettle();
      expect(bundles.types.single, 'application/pdf');
      expect(bundles.shared.single, endsWith(recoverySheetName));
    });

    testWidgets('a chosen folder gets a Backup at once, then weekly', (
      tester,
    ) async {
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      await setUpBackups(store, 'correct horse', derive: quickKey);
      final folder = Directory('${dir.path}/backups')..createSync();
      await page(
        tester,
        BackupScreen(store: store, derive: quickKey),
        folders: DirFolders(folder.path),
      );
      expect(find.textContaining('No folder chosen yet'), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('backup-folder')));
      await tester.pumpAndSettle();
      expect(store.backupFolder, folder.path);
      expect(find.text('Backups go into Backups.'), findsOneWidget);
      expect(announcements.visible, 'Backup written.');
      final written = folder.listSync().whereType<File>().single;
      expect(written.path, endsWith('.$backupExtension'));
      expect(readBackupHeader(written.path)?.created, now);
      expect(find.textContaining('Last Backup:'), findsOneWidget);
      expect(store.backupsScheduled, isTrue);

      await tester.tap(find.byKey(const ValueKey('backup-weekly')));
      await tester.pumpAndSettle();
      expect(store.backupsWeekly, isFalse);

      // A folder gone: the failure shows.
      folder.deleteSync(recursive: true);
      announcements.dismiss();
      await tester.tap(find.byKey(const ValueKey('backup-now')));
      await tester.pumpAndSettle();
      expect(announcements.visible, startsWith('The Backup could not'));
      expect(find.textContaining('could not be written'), findsWidgets);
    });
  });

  group('restoring', () {
    testWidgets('a wrong passphrase opens nothing, the right one restores', (
      tester,
    ) async {
      final (path, _) = await backup();
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      RestoreResult? popped;
      await page(
        tester,
        Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              popped = await Navigator.of(context).push<RestoreResult>(
                MaterialPageRoute(
                  builder: (_) => RestoreScreen(store: store, derive: quickKey),
                ),
              );
            },
            child: const Text('open'),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();

      // A file that is no Backup.
      bundles.picks = copy('${Directory.current.path}/pubspec.yaml');
      await tester.tap(find.byKey(const ValueKey('restore-pick')));
      await tester.pumpAndSettle();
      expect(
        announcements.visible,
        'This file is not a Backup of Old Brain Fix.',
      );
      expect(find.byKey(const ValueKey('restore')), findsNothing);

      announcements.dismiss();
      bundles.picks = copy(path);
      await tester.tap(find.byKey(const ValueKey('restore-pick')));
      await tester.pumpAndSettle();
      expect(find.textContaining('Backup of October 24, 2026'), findsOneWidget);
      await tester.enterText(
        find.byKey(const ValueKey('restore-passphrase')),
        'wrong horse',
      );
      await tester.tap(find.byKey(const ValueKey('restore')));
      await tester.pumpAndSettle();
      expect(
        announcements.visible,
        'This passphrase or recovery key does not open the Backup.',
      );
      expect(store.currentSeq(), 0);

      await tester.enterText(
        find.byKey(const ValueKey('restore-passphrase')),
        'correct horse',
      );
      await tester.tap(find.byKey(const ValueKey('restore')));
      await tester.pumpAndSettle();
      expect(popped?.entries, greaterThan(0));
      expect(Notes(store).all().single.text, 'Pills at eight');
    });

    testWidgets('on first start, before the Starter Set, with the recovery '
        'key scanned from the sheet', (tester) async {
      final (path, code) = await backup();
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      bundles.picks = copy(path);
      final pairing = FakePairing()..code = 'not a key';
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          announcements: announcements,
          bundles: bundles,
          photos: FakePhotos(nextPhoto: onePixelPng),
          pairing: pairing,
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Pick your Areas'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('restore-offer')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('restore-pick')));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('restore-scan')));
      await tester.pumpAndSettle();
      expect(
        announcements.visible,
        startsWith('No recovery key found in the photo'),
      );
      announcements.dismiss();
      pairing.code = code;
      await tester.tap(find.byKey(const ValueKey('restore-scan')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('restore')));
      await tester.pumpAndSettle();

      expect(announcements.visible, startsWith('Restored: '));
      expect(find.text('Pick your Areas'), findsNothing);
      expect(store.starterSetOffered, isTrue);
      expect(Places(store).areas().single.name, 'Doctor');
      expect(store.recoveryCode, code);
    });

    testWidgets('is not offered once the log holds anything', (tester) async {
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      Notes(store).capture('Captured before');
      await tester.pumpWidget(OldBrainFixApp(store: store, bundles: bundles));
      await tester.pumpAndSettle();
      expect(find.text('Pick your Areas'), findsOneWidget);
      expect(find.byKey(const ValueKey('restore-offer')), findsNothing);
    });
  });
}
