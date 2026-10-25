import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A cost tests can afford; files say what they were made with.
const cheap = BackupKdf(memory: 64, iterations: 1);

const passphrase = 'correct horse battery';

/// A clock one second further on every read.
DateTime Function() ticking([DateTime? from]) {
  var now = from ?? DateTime.utc(2026, 10, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

/// Bytes that are no JPEG, so they are kept as they are, and more than
/// a chunk holds.
Uint8List bigPhoto() =>
    Uint8List.fromList(List.generate(150000, (i) => (i * 7 + 3) & 0xff));

/// A Device with something of everything: an Area with a Note, a photo
/// and a voice recording, a Routine checked off once, a Moment, Area
/// keys, another Device's pinned key and Device settings of all kinds.
Store fullDevice(Directory dir) {
  final store = Store.open('${dir.path}/old.db', clock: ticking());
  final places = Places(store);
  final notes = Notes(store);
  final area = places.addArea('Doctor');
  final mark = store.currentSeq();
  final note = notes.capture(
    'Pills at eight',
    place: area.id,
    photos: [bigPhoto()],
    recordings: [Uint8List.fromList(List.filled(900, 42))],
  );
  store.addMoment(seq: mark, cause: MomentCause.sync, label: 'Desk');
  final routines = Routines(store);
  final routine = routines.spawn(
    note.id,
    Schedule(
      start: Day(2026, 10, 1),
      times: [ClockTime(8, 0)],
      rule: Recurrence.parse('FREQ=DAILY'),
    ),
  );
  routines.markDone(routine.id, Occurrence(Day(2026, 10, 1), ClockTime(8, 0)));
  store.ensureAreaKeys();
  final other = Store.inMemory();
  store.learnKeys([other.ownKeyRecord], verifiedDevice: other.deviceId);
  other.close();
  // Device settings, none of which may travel.
  store
    ..themeChoice = ThemeChoice.dark
    ..renagInterval = const Duration(minutes: 20)
    ..chooseSyncFolder('/somewhere/sync')
    ..backupFolder = '/somewhere/backups'
    ..setPin('1234', iterations: 1)
    ..appLockOn = true
    ..rememberDevice(
      PairedDevice(
        id: 'd' * 32,
        name: 'Desk',
        fingerprint: 'f' * 64,
        host: null,
        port: null,
        lastSync: DateTime.utc(2026, 9, 1),
      ),
    );
  return store;
}

/// Every local setting [store] holds, by key.
Map<String, String> settings(Store store) => {
  for (final (k, v) in store.localSettingsWithPrefix('')) k: v,
};

void main() {
  late Directory dir;

  setUp(() => dir = Directory.systemTemp.createTempSync('backup'));
  tearDown(() => dir.deleteSync(recursive: true));

  Future<(Store, String, String)> backedUp() async {
    final store = fullDevice(dir);
    final code = await setUpBackups(store, passphrase, kdf: cheap);
    final path = '${dir.path}/b.$backupExtension';
    await writeBackup(store, path);
    return (store, code, path);
  }

  Store fresh(String name) {
    final home = Directory('${dir.path}/$name')..createSync();
    return Store.open('${home.path}/new.db', clock: ticking());
  }

  group('the recovery key', () {
    test('is 52 characters in groups of four and reads back', () {
      final key = Uint8List.fromList(List.generate(32, (i) => i * 9));
      final code = encodeRecoveryCode(key);
      expect(
        code,
        matches(RegExp(r'^([0-9A-HJKMNP-TV-Z]{4}-){12}[0-9A-Z]{4}$')),
      );
      expect(decodeRecoveryCode(code), key);
    });

    test('forgives case, blanks, missing dashes and O, I, L', () {
      final key = Uint8List.fromList(List.generate(32, (i) => 255 - i));
      final code = encodeRecoveryCode(key);
      final typed = code
          .toLowerCase()
          .replaceAll('-', ' ')
          .replaceAll('0', 'o')
          .replaceAll('1', 'l');
      expect(decodeRecoveryCode(typed), key);
    });

    test('a typo or a wrong length is no recovery key', () {
      final key = Uint8List.fromList(List.generate(32, (i) => i));
      final code = encodeRecoveryCode(key);
      final typo =
          '${code.substring(0, 5)}${code[5] == 'X' ? 'Y' : 'X'}'
          '${code.substring(6)}';
      expect(decodeRecoveryCode(typo), isNull);
      expect(decodeRecoveryCode(code.substring(1)), isNull);
      expect(decodeRecoveryCode('hello'), isNull);
    });
  });

  group('the passphrase', () {
    test('derives with Argon2id as RustCrypto\'s argon2 does', () async {
      final key = await backupPassKey(
        '  password ',
        BackupKdf.standard,
        'somesaltsomesalt'.codeUnits,
      );
      expect(
        hex(key),
        '7664ad4ba1a3c999fcdd0991ffc2270f78302d2383233db5e7befc85d1bb1819',
      );
    });

    test('must not be too short', () async {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(
        () => setUpBackups(store, ' short  ', kdf: cheap),
        throwsArgumentError,
      );
      expect(store.backupsSetUp, isFalse);
    });

    test('setting up keeps the backup key and its sealed copy, never the '
        'passphrase', () async {
      final store = Store.inMemory();
      addTearDown(store.close);
      final code = await setUpBackups(store, passphrase, kdf: cheap);
      expect(store.backupsSetUp, isTrue);
      expect(store.recoveryCode, code);
      expect(
        settings(store).values.any((v) => v.contains(passphrase)),
        isFalse,
      );
    });
  });

  group('a Backup', () {
    test('restores everything but the settings onto a new identity, with '
        'the passphrase', () async {
      final (old, code, path) = await backedUp();
      addTearDown(old.close);
      final header = readBackupHeader(path)!;
      final key = await unlockBackup(header, passphrase: passphrase);
      expect(key, isNotNull);
      final store = fresh('a');
      addTearDown(store.close);
      final result = await restoreBackup(store, path, key!);

      expect(result.from, old.deviceId);
      expect(store.deviceId, isNot(old.deviceId));
      expect(store.signingKey.seedHex, isNot(old.signingKey.seedHex));
      // The log, entry for entry, and what it reads as.
      expect(
        [for (final e in store.allEntries()) e.toJson()],
        [for (final e in old.allEntries()) e.toJson()],
      );
      expect(result.entries, old.allEntries().length);
      final note = Notes(store).unsorted();
      expect(note, isEmpty);
      final area = Places(store).areas().single;
      expect(area.name, 'Doctor');
      final notes = Notes(store).inside(area.id);
      expect(notes.single.text, 'Pills at eight');
      // The blobs.
      expect(heldBlobs(store), heldBlobs(old));
      expect(result.blobs, 2);
      for (final ref in heldBlobs(old)) {
        final (kind, hash) = splitBlobRef(ref);
        expect(store.blob(kind, hash), old.blob(kind, hash));
      }
      // The Moment at the same place in the log, at the same time.
      final moment = store.momentRows().single;
      final was = old.momentRows().single;
      expect(moment.seq, was.seq);
      expect(moment.at, was.at);
      expect(moment.label, 'Desk');
      // The keys.
      expect(
        [for (final k in store.areaKeys()) k.id],
        [for (final k in old.areaKeys()) k.id],
      );
      final pinned = {for (final p in store.pinnedKeys()) p.record.device: p};
      expect(
        pinned.keys,
        containsAll(old.pinnedKeys().map((p) => p.record.device)),
      );
      expect(pinned[old.deviceId]?.trust, KeyTrust.verified);
      // None of the old Device's settings came along.
      expect(store.themeChoice, ThemeChoice.system);
      expect(store.renagInterval, DeviceSettings.defaultRenag);
      expect(store.syncFolder, isNull);
      expect(store.backupFolder, isNull);
      expect(store.hasPin, isFalse);
      expect(store.appLockOn, isFalse);
      expect(store.pairedDevices(), isEmpty);
      final keys = settings(store).keys.where(
        (k) =>
            !k.startsWith('areakey:') &&
            !k.startsWith('key:') &&
            k != 'device' &&
            k != 'signseed',
      );
      expect(keys.toSet(), {'backup:key', 'backup:pass'});
      // The Recovery Sheet stays good.
      expect(store.recoveryCode, code);
      // What the new Device writes signs under its own key.
      Notes(store).capture('After the restore');
      expect(store.versionVector()[store.deviceId], greaterThan(0));
    });

    test('restores with the recovery key as well', () async {
      final (old, code, path) = await backedUp();
      addTearDown(old.close);
      final key = await unlockBackup(
        readBackupHeader(path)!,
        recoveryCode: code.toLowerCase(),
      );
      expect(key, isNotNull);
      final store = fresh('b');
      addTearDown(store.close);
      final result = await restoreBackup(store, path, key!);
      expect(result.entries, old.allEntries().length);
    });

    test('a wrong passphrase or recovery key opens nothing', () async {
      final (old, _, path) = await backedUp();
      addTearDown(old.close);
      final header = readBackupHeader(path)!;
      expect(
        await unlockBackup(header, passphrase: 'wrong horse battery'),
        isNull,
      );
      final other = encodeRecoveryCode(Uint8List(32));
      expect(await unlockBackup(header, recoveryCode: other), isNull);
      expect(await unlockBackup(header, recoveryCode: 'nonsense'), isNull);
      final store = fresh('c');
      addTearDown(store.close);
      await expectLater(
        restoreBackup(store, path, Uint8List(32)),
        throwsA(
          isA<BackupException>().having(
            (e) => e.problem,
            'problem',
            BackupProblem.wrongSecret,
          ),
        ),
      );
      expect(store.currentSeq(), 0);
    });

    test('a changed, moved or cut off byte is found and nothing is '
        'restored', () async {
      final (old, code, path) = await backedUp();
      addTearDown(old.close);
      final key = decodeRecoveryCode(code)!;
      final bytes = File(path).readAsBytesSync();
      final headerLength = readBackupHeader(path)!.line.length + 1;
      final whole = backupChunkBytes + 40;
      final variants = {
        'a byte flipped': Uint8List.fromList(bytes)..[headerLength + 5000] ^= 1,
        'the last chunk gone': bytes.sublist(0, headerLength + 2 * whole),
        'cut short': bytes.sublist(0, bytes.length - 3),
        'chunks swapped': Uint8List.fromList([
          ...bytes.sublist(0, headerLength),
          ...bytes.sublist(headerLength + whole, headerLength + 2 * whole),
          ...bytes.sublist(headerLength, headerLength + whole),
          ...bytes.sublist(headerLength + 2 * whole),
        ]),
        'the header changed': Uint8List.fromList(
          String.fromCharCodes(bytes)
              .replaceFirst('"created":"2026', '"created":"2027')
              .codeUnits,
        ),
      };
      var n = 0;
      for (final MapEntry(key: what, value: changed) in variants.entries) {
        final file = File('${dir.path}/changed${n++}.$backupExtension')
          ..writeAsBytesSync(changed);
        final store = fresh('t$n');
        addTearDown(store.close);
        await expectLater(
          restoreBackup(store, file.path, key),
          throwsA(
            isA<BackupException>().having(
              (e) => e.problem,
              'problem',
              BackupProblem.damaged,
            ),
          ),
          reason: what,
        );
        expect(store.currentSeq(), 0, reason: what);
        expect(store.backupsSetUp, isFalse, reason: what);
      }
    });

    test('a file that is no Backup is told apart', () async {
      final file = File('${dir.path}/x.$backupExtension')
        ..writeAsStringSync('PK\u0003\u0004 a zip');
      expect(readBackupHeader(file.path), isNull);
      final store = fresh('d');
      addTearDown(store.close);
      await expectLater(
        restoreBackup(store, file.path, Uint8List(32)),
        throwsA(
          isA<BackupException>().having(
            (e) => e.problem,
            'problem',
            BackupProblem.notABackup,
          ),
        ),
      );
    });

    test('goes only into an empty log', () async {
      final (old, code, path) = await backedUp();
      addTearDown(old.close);
      final store = fresh('e');
      addTearDown(store.close);
      Notes(store).capture('Already here');
      await expectLater(
        restoreBackup(store, path, decodeRecoveryCode(code)!),
        throwsA(
          isA<BackupException>().having(
            (e) => e.problem,
            'problem',
            BackupProblem.notEmpty,
          ),
        ),
      );
    });

    test('a Device without Backups set up writes none', () async {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(() => writeBackup(store, '${dir.path}/none'), throwsStateError);
    });

    test('the new Device takes over the old one\'s role in a Shared '
        'Area', () async {
      final old = Store.open('${dir.path}/ann.db', clock: ticking());
      addTearDown(old.close);
      final area = Places(old).addArea('Family');
      old.ensureAreaKeys();
      final shared = old.share(area.id, name: 'Ann');
      expect(old.accessIn(shared), AreaAccess.manager);
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final path = '${dir.path}/ann.$backupExtension';
      await writeBackup(old, path);

      final store = fresh('f');
      addTearDown(store.close);
      final result = await restoreBackup(
        store,
        path,
        decodeRecoveryCode(code)!,
      );
      expect(result.roles, 1);
      expect(store.accessIn(shared), AreaAccess.manager);
      // Signed as the old Device, next in its own count.
      final handed = store.allEntries().last;
      expect(handed.device, old.deviceId);
      expect(handed.dseq, old.versionVector()[old.deviceId]! + 1);
      expect(handed.field, ShareKeys.role(store.deviceId));
      // And what the new Device writes there counts.
      final note = Notes(store).capture('Groceries', place: shared);
      expect(Notes(store).inside(shared).map((n) => n.id), [note.id]);
    });
  });

  group('the Backup folder', () {
    test('keeps the last four Backups of this Device and leaves the '
        'others alone', () async {
      var now = DateTime.utc(2026, 10, 1, 9);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      await setUpBackups(store, passphrase, kdf: cheap);
      final folder = Directory('${dir.path}/folder')..createSync();
      File('${folder.path}/oldbrainfix-20200101-000000-ffffffff.obfbackup')
          .writeAsStringSync('another Device');
      File('${folder.path}/notes.txt').writeAsStringSync('mine');
      final work = Directory('${dir.path}/work')..createSync();
      final names = <String>[];
      for (var i = 0; i < 6; i++) {
        now = now.add(const Duration(days: 1));
        names.add(
          await backUpNow(store, LocalBackupFolder(folder.path), work.path),
        );
      }
      final left = await LocalBackupFolder(folder.path).list();
      expect(left, [
        'notes.txt',
        'oldbrainfix-20200101-000000-ffffffff.obfbackup',
        ...names.skip(2),
      ]);
      expect(names.first, startsWith('oldbrainfix-20261002-090000-'));
      expect(work.listSync(), isEmpty);
      expect(store.lastBackup, now);
      expect(readBackupHeader('${folder.path}/${names.last}')?.created, now);
    });

    test('a scheduled Backup is due weekly once set up and a folder is '
        'chosen', () async {
      var now = DateTime.utc(2026, 10, 1, 9);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      final folder = Directory('${dir.path}/weekly')..createSync();
      final work = dir.path;
      BackupFolder open(String path) => LocalBackupFolder(path);
      expect(store.backupDue(now), isFalse);
      await setUpBackups(store, passphrase, kdf: cheap);
      expect(store.backupDue(now), isFalse);
      store.backupFolder = folder.path;
      expect(store.backupsScheduled, isTrue);
      expect(await backUpIfDue(store, open, work), isNotNull);
      now = now.add(const Duration(days: 6));
      expect(await backUpIfDue(store, open, work), isNull);
      now = now.add(const Duration(days: 1));
      expect(await backUpIfDue(store, open, work), isNotNull);
      now = now.add(const Duration(days: 8));
      store.backupsWeekly = false;
      expect(await backUpIfDue(store, open, work), isNull);
    });

    test('a failure is kept for the screen until a Backup is '
        'written', () async {
      final now = DateTime.utc(2026, 10, 1, 9);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      await setUpBackups(store, passphrase, kdf: cheap);
      final gone = LocalBackupFolder('${dir.path}/not/there');
      await expectLater(
        backUpNow(store, gone, dir.path),
        throwsA(isA<FileSystemException>()),
      );
      expect(store.backupFailed, now);
      expect(store.lastBackup, isNull);
      final there = Directory('${dir.path}/there')..createSync();
      await backUpNow(store, LocalBackupFolder(there.path), dir.path);
      expect(store.backupFailed, isNull);
      expect(store.lastBackup, now);
    });
  });

  group('the Recovery Sheet', () {
    const sheet = RecoverySheet(
      title: 'Wiederherstellungsblatt',
      about: ['1. Oktober 2026', 'Gerät: Annas Telefon (Küche)'],
      intro: [
        'Mit diesem Schlüssel öffnen Sie Ihre Sicherungen, auch '
            'wenn Sie die Passphrase vergessen haben. Bewahren Sie ihn gut '
            'auf – zum Beispiel in einer Schublade.',
      ],
      codeLabel: 'Wiederherstellungsschlüssel',
      steps: [
        'Neu installieren, „Aus einer Sicherung wiederherstellen“ '
            'wählen und den Schlüssel eingeben oder den QR-Code scannen. 🧠',
      ],
    );
    final code = encodeRecoveryCode(List.generate(32, (i) => i));

    test('is a one-page PDF whose cross references hold', () {
      final pdf = recoverySheetPdf(sheet, code);
      final text = String.fromCharCodes(pdf);
      expect(text, startsWith('%PDF-1.4\n'));
      expect(text, endsWith('%%EOF\n'));
      final xref = int.parse(
        RegExp(r'startxref\n(\d+)\n').firstMatch(text)!.group(1)!,
      );
      expect(text.substring(xref), startsWith('xref\n0 8\n'));
      final offsets = RegExp(
        r'^(\d{10}) 00000 n $',
        multiLine: true,
      ).allMatches(text).map((m) => int.parse(m.group(1)!)).toList();
      expect(offsets, hasLength(7));
      for (var i = 0; i < offsets.length; i++) {
        expect(text.substring(offsets[i]), startsWith('${i + 1} 0 obj\n'));
      }
      final length = int.parse(
        RegExp(r'/Length (\d+) >>\nstream\n').firstMatch(text)!.group(1)!,
      );
      final start = text.indexOf('stream\n') + 'stream\n'.length;
      expect(text.substring(start + length), startsWith('endstream'));
    });

    test('carries the recovery key, the text in WinAnsi and the QR', () {
      final pdf = recoverySheetPdf(sheet, code);
      final text = String.fromCharCodes(pdf);
      final groups = code.split('-');
      expect(text, contains('(${groups.take(7).join('-')}) Tj'));
      expect(text, contains('(${groups.skip(7).join('-')}) Tj'));
      // ä as WinAnsi 0xE4, „ as 0x84, the brackets escaped, the emoji
      // unknown to WinAnsi as ?.
      expect(text, contains(r'Ger\344t: Annas Telefon \(K\374che\)'));
      expect(text, contains(r'\204Aus einer Sicherung'));
      expect(text, contains('scannen. ?'));
      final dark = recoveryQr(code).expand((row) => row).where((d) => d);
      expect(' re\n'.allMatches(text), hasLength(dark.length));
      expect(pdf.every((b) => b < 0x80), isTrue);
    });

    test('the same text gives the same bytes', () {
      expect(recoverySheetPdf(sheet, code), recoverySheetPdf(sheet, code));
    });
  });
}
