import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'shared_areas_test.dart' show exchange;

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
/// keys, another Device's pinned key and Device settings of all kinds,
/// a playback speed among them.
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
  // The one that does.
  store.playbackSpeed = 1.5;
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
    test('restores everything but the settings, the playback speed aside, '
        'onto a new identity, with the passphrase', () async {
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
      // None of the old Device's settings came along but the playback
      // speed.
      expect(store.playbackSpeed, 1.5);
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
      expect(keys.toSet(), {
        'backup:key',
        'backup:pass',
        'backup:version',
        'playbackSpeed',
      });
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

    test('a Device without a playback speed chosen leaves the new one at '
        'normal speed', () async {
      final old = fullDevice(dir)..setLocalSetting('playbackSpeed', '7');
      addTearDown(old.close);
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final path = '${dir.path}/b.$backupExtension';
      await writeBackup(old, path);
      final store = fresh('b');
      addTearDown(store.close);
      await restoreBackup(store, path, decodeRecoveryCode(code)!);
      expect(store.localSetting('playbackSpeed'), isNull);
      expect(store.playbackSpeed, 1);
    });

    test('a playback speed not offered here, or not written as text, is '
        'passed over', () async {
      final old = fullDevice(dir);
      addTearDown(old.close);
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final backupKey = decodeRecoveryCode(code)!;
      // A Backup of nothing but the old Device and those speeds, sealed
      // as one last chunk.
      final contentKey = secretBytes(backupKeyBytes);
      final line = BackupHeader.encode(
        created: DateTime.utc(2026, 10, 2),
        pass: old.backupPass!,
        sealedKey: seal(backupKey, contentKey, aad: SealedAs.backupKey),
      );
      final payload = [
        {
          'about': {'device': old.deviceId, 'seed': old.signingKey.seedHex},
        },
        {'playbackSpeed': '1.25'},
        {'playbackSpeed': 2},
      ].map((r) => '${jsonEncode(r)}\n').join();
      final hash = sha256.convert(line).toString();
      final file = File('${dir.path}/speeds.$backupExtension')
        ..writeAsBytesSync([
          ...line,
          0x0a,
          ...seal(
            contentKey,
            utf8.encode(payload),
            aad: SealedAs.backupChunk(hash, 0, last: true),
          ),
        ]);
      final store = fresh('b');
      addTearDown(store.close);
      await restoreBackup(store, file.path, backupKey);
      expect(store.localSetting('playbackSpeed'), isNull);
      expect(store.playbackSpeed, 1);
    });

    test('a text in a format not known here comes back as it was', () async {
      final old = fullDevice(dir);
      addTearDown(old.close);
      const future = '{"format":"x-future","text":"# Pills"}';
      final note = Notes(old).all().single;
      old.append(note.id, NoteKeys.text, future);
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final path = '${dir.path}/b.$backupExtension';
      await writeBackup(old, path);
      final key = await unlockBackup(
        readBackupHeader(path)!,
        recoveryCode: code,
      );
      final store = fresh('b');
      addTearDown(store.close);
      await restoreBackup(store, path, key!);
      expect(store.current(note.id, NoteKeys.text), future);
      expect(Notes(store).byId(note.id)?.textEditable, isFalse);
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
      final shared = old.share(area.id);
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

  group('one backup key per Profile', () {
    /// A Device of its own with a Profile: named by the user unless
    /// [named] is null.
    Store own(String? named, {String device = 'Device'}) {
      final store = Store.inMemory(clock: ticking());
      store.applyProfileDefaults(name: device);
      if (named != null) store.renameOwnProfile(named);
      return store;
    }

    /// Writes a Backup of [store] and restores it into a new Device, once
    /// with the recovery key [code] and once with the passphrase; returns
    /// the second.
    Future<Store> restoredFrom(Store store, String code, String name) async {
      final path = '${dir.path}/$name.$backupExtension';
      await writeBackup(store, path);
      final header = readBackupHeader(path)!;
      final byCode = await unlockBackup(header, recoveryCode: code);
      final byPass = await unlockBackup(header, passphrase: passphrase);
      expect(byCode, isNotNull, reason: '$name opens with the recovery key');
      expect(byPass, isNotNull, reason: '$name opens with the passphrase');
      final first = fresh('$name-code');
      addTearDown(first.close);
      await restoreBackup(first, path, byCode!);
      final second = fresh('$name-pass');
      addTearDown(second.close);
      await restoreBackup(second, path, byPass!);
      return second;
    }

    test('a Device paired as one\'s own receives it with its sealed copy, '
        'and one Recovery Sheet opens the Backups of both', () async {
      final phone = own(null, device: 'Pixel');
      final desk = own('Ada');
      addTearDown(phone.close);
      addTearDown(desk.close);
      exchange(phone, desk);
      expect(phone.ownProfile?.id, desk.ownProfile?.id);
      final code = await setUpBackups(phone, passphrase, kdf: cheap);
      final session = exchange(phone, desk);
      expect(session.request['backupKey'], phone.backupKeyToSend);
      expect(desk.backupsSetUp, isTrue);
      expect(desk.recoveryCode, code);
      expect(desk.backupPass?.toJson(), phone.backupPass?.toJson());
      // Each writes its own Backups, all opened with the same key.
      final fromPhone = await restoredFrom(phone, code, 'phone');
      final fromDesk = await restoredFrom(desk, code, 'desk');
      expect(fromPhone.recoveryCode, code);
      expect(fromDesk.recoveryCode, code);

      // Set up again on the desk: the newer key reaches the phone.
      final again = await setUpBackups(desk, passphrase, kdf: cheap);
      expect(desk.backupKeyVersion, 2);
      exchange(phone, desk);
      expect(phone.recoveryCode, again);
      expect(phone.backupKeyVersion, 2);
    });

    test('two Devices that set it up apart end with the same key', () async {
      final phone = own(null, device: 'Pixel');
      final desk = own('Ada');
      addTearDown(phone.close);
      addTearDown(desk.close);
      exchange(phone, desk);
      await setUpBackups(phone, passphrase, kdf: cheap);
      await setUpBackups(desk, passphrase, kdf: cheap);
      final larger = [
        phone.backupKeyToSend!['key']! as String,
        desk.backupKeyToSend!['key']! as String,
      ]..sort();
      exchange(phone, desk);
      exchange(phone, desk);
      expect(phone.recoveryCode, desk.recoveryCode);
      expect(base64.encode(phone.backupKey!), larger.last);
    });

    test('pairing two Devices with Backups of their own: the kept '
        'Profile\'s key wins, the other\'s old Backups open with its old '
        'Recovery Sheet', () async {
      final phone = own('Ada', device: 'Pixel');
      final desk = own('Ada L.');
      addTearDown(phone.close);
      addTearDown(desk.close);
      final phoneCode = await setUpBackups(phone, passphrase, kdf: cheap);
      final oldPath = '${dir.path}/phone-old.$backupExtension';
      await writeBackup(phone, oldPath);
      final deskCode = await setUpBackups(
        desk,
        'another passphrase',
        kdf: cheap,
      );
      exchange(phone, desk);
      // "Which one is you?" waits: no key changes Profile meanwhile.
      expect(phone.profileQuestion, isNotNull);
      expect(phone.recoveryCode, phoneCode);
      expect(desk.recoveryCode, deskCode);
      // The phone takes the desk's Profile over.
      phone.answerProfileQuestion(keepMine: false);
      expect(phone.backupKeyVersion, 0);
      exchange(phone, desk);
      expect(phone.recoveryCode, deskCode);
      expect(desk.recoveryCode, deskCode);
      expect(phone.backupPass?.toJson(), desk.backupPass?.toJson());
      // What the phone wrote before still opens with its old sheet.
      expect(
        await unlockBackup(readBackupHeader(oldPath)!, recoveryCode: phoneCode),
        isNotNull,
      );
    });

    test('a kept Profile without Backups takes the other\'s key', () async {
      final phone = own(null, device: 'Pixel');
      final desk = own('Ada');
      addTearDown(phone.close);
      addTearDown(desk.close);
      final code = await setUpBackups(phone, passphrase, kdf: cheap);
      exchange(phone, desk);
      // The phone, still with the defaults, took the desk's Profile.
      expect(phone.ownProfile?.id, desk.ownProfile?.id);
      expect(phone.recoveryCode, code);
      exchange(phone, desk);
      expect(desk.recoveryCode, code);
      expect(desk.backupKeyVersion, 0);
    });

    test('a new Device added as one\'s own before it made a Profile takes '
        'the Profile, the places and the key in its first sync, joining or '
        'hosting', () async {
      for (final joins in [true, false]) {
        // Still with the defaults, as most are: the other's id may be the
        // lower one, and still its Profile stays.
        final desk = own(null, device: 'Desk');
        final code = await setUpBackups(desk, passphrase, kdf: cheap);
        final places = Places(desk);
        final home = places.addArea('Home');
        places.addTopic(home.id, 'Kitchen');
        NamedPlaces(desk).add('Office', Coordinates(52.5, 13.4), radius: 100);
        final kept = (desk.ownProfile?.encode(), desk.ownProfileCode);
        final phone = Store.inMemory(clock: ticking());
        addTearDown(phone.close);
        addTearDown(desk.close);
        joins ? exchange(phone, desk) : exchange(desk, phone);
        expect((desk.ownProfile?.encode(), desk.ownProfileCode), kept);
        expect((phone.ownProfile?.encode(), phone.ownProfileCode), kept);
        expect(phone.recoveryCode, code);
        expect(phone.backupsSetUp, isTrue);
        expect(Places(phone).areas().map((a) => a.name), ['Home']);
        expect(Places(phone).topics(home.id).map((t) => t.name), ['Kitchen']);
        expect(NamedPlaces(phone).all().map((p) => p.name), ['Office']);
        expect(Places(phone).starterSetDue, isFalse);
        // The phone's key is bound to the Profile on the desk at once.
        expect(desk.ownSignedProfile?.keys.map((k) => k.key), [
          phone.signingKey.publicBase64,
        ]);
      }
    });

    test('another person\'s Device neither gets nor gives one', () async {
      final ann = own('Ann')..setLocalSetting('author', 'Ann');
      final ben = own('Ben')..setLocalSetting('author', 'Ben');
      addTearDown(ann.close);
      addTearDown(ben.close);
      final annCode = await setUpBackups(ann, passphrase, kdf: cheap);
      final benCode = await setUpBackups(ben, passphrase, kdf: cheap);
      final house = Places(ann).addArea('Household');
      final area = ann.share(house.id);
      final invited = exchange(
        ben,
        ann,
        inviting: Invite(area, role: AreaRole.member),
      );
      final later = exchange(ben, ann);
      final inviting = exchange(
        ann,
        ben,
        invite: Invite(
          ann.share(Places(ann).addArea('Garden').id),
          role: AreaRole.member,
        ),
      );
      for (final e in [invited, later, inviting]) {
        expect(e.request.containsKey('backupKey'), isFalse);
        expect(e.answer.containsKey('backupKey'), isFalse);
      }
      expect(ann.recoveryCode, annCode);
      expect(ben.recoveryCode, benCode);
    });

    test('a key is taken only for the own Profile, only whole and only '
        'when newer', () async {
      final store = own('Ada');
      addTearDown(store.close);
      final other = own('Ada');
      addTearDown(other.close);
      await setUpBackups(other, passphrase, kdf: cheap);
      final sent = other.backupKeyToSend!;
      final id = store.ownProfile!.id;
      Map<String, Object?> as(Map<String, Object?> changes) => {
        ...sent,
        'profile': id,
        ...changes,
      };
      expect(store.takeBackupKey(sent), isFalse, reason: 'another Profile');
      expect(store.takeBackupKey(null), isFalse);
      expect(store.takeBackupKey(as({'key': 'no base64!'})), isFalse);
      expect(
        store.takeBackupKey(
          as({
            'key': base64.encode([1, 2, 3]),
          }),
        ),
        isFalse,
      );
      expect(store.takeBackupKey(as({'pass': 'none'})), isFalse);
      expect(store.takeBackupKey(as({'version': -1})), isFalse);
      expect(store.takeBackupKey(as({'version': '2'})), isFalse);
      expect(store.backupsSetUp, isFalse);
      final low = base64.encode(List.filled(backupKeyBytes, 1));
      final high = base64.encode(List.filled(backupKeyBytes, 2));
      expect(store.takeBackupKey(as({'key': high, 'version': 2})), isTrue);
      expect(store.takeBackupKey(as({'key': high, 'version': 2})), isFalse);
      expect(store.takeBackupKey(as({'key': low, 'version': 2})), isFalse);
      expect(store.takeBackupKey(as({'key': low, 'version': 1})), isFalse);
      expect(store.takeBackupKey(as({'key': low, 'version': 3})), isTrue);
      expect(base64.encode(store.backupKey!), low);
      expect(store.backupKeyVersion, 3);
      expect(store.backupsSetUp, isTrue);
    });

    test('the shared folder carries it to a Device of the own group, in '
        'the keys file sealed with the personal key', () async {
      final phone = own(null, device: 'Pixel');
      final desk = own('Ada');
      addTearDown(phone.close);
      addTearDown(desk.close);
      exchange(phone, desk);
      final code = await setUpBackups(phone, passphrase, kdf: cheap);
      final folder = MemorySyncFolder();
      await folderSync(phone, folder);
      expect(desk.backupsSetUp, isFalse);
      await folderSync(desk, folder);
      expect(desk.recoveryCode, code);
      // Nothing of it in plain text.
      final key = phone.backupKeyToSend!['key']! as String;
      for (final files in folder.dirs.values) {
        for (final bytes in files.values) {
          expect(
            latin1.decode(bytes, allowInvalid: true),
            isNot(contains(key)),
          );
        }
      }
    });

    test('a bundle for a Device of the own group carries it too', () async {
      final phone = own(null, device: 'Pixel');
      final desk = own('Ada');
      addTearDown(phone.close);
      addTearDown(desk.close);
      exchange(phone, desk);
      final code = await setUpBackups(phone, passphrase, kdf: cheap);
      Notes(phone).capture('Written since');
      final path = '${dir.path}/phone.$bundleExtension';
      expect(
        await writeBundle(phone, path, forDevice: desk.deviceId),
        isNotNull,
      );
      await importBundle(desk, path);
      expect(desk.recoveryCode, code);
    });

    test('restoring a Backup restores the Profile, the new Device bound '
        'to it by the old one', () async {
      final phone = own('Ada', device: 'Pixel');
      final desk = own(null);
      addTearDown(phone.close);
      addTearDown(desk.close);
      exchange(desk, phone);
      final code = await setUpBackups(phone, passphrase, kdf: cheap);
      exchange(desk, phone);
      final id = phone.ownProfile!.id;
      final shortCode = phone.ownProfileCode;
      final path = '${dir.path}/phone.$backupExtension';
      await writeBackup(phone, path);

      // The new phone made a Profile of its own at its first start.
      final store = fresh('new');
      addTearDown(store.close);
      store.applyProfileDefaults(name: 'Pixel 9');
      var told = 0;
      store.addProfileListener(() => told++);
      await restoreBackup(store, path, decodeRecoveryCode(code)!);
      expect(told, greaterThan(0));
      expect(store.ownProfile?.id, id);
      expect(store.ownProfile?.name, 'Ada');
      expect(store.ownProfileCode, shortCode);
      expect(store.recoveryCode, code);
      // Its defaults no longer replace the name.
      store.applyProfileDefaults(name: 'Pixel 9');
      expect(store.ownProfile?.name, 'Ada');
      // Its key is bound to the Profile by the old phone's.
      final bound = store.ownSignedProfile!.keys.singleWhere(
        (k) => k.key == store.signingKey.publicBase64,
      );
      expect(bound.by, phone.signingKey.publicBase64);
      // What it changes counts on the desk.
      store.renameOwnProfile('Ada Lovelace');
      exchange(store, desk);
      expect(desk.ownProfile?.name, 'Ada Lovelace');
      expect(desk.ownProfile?.id, id);
      expect(desk.recoveryCode, code);
    });
  });

  group('Auto Backup', () {
    test('is off until opted in, then the weekly Backup goes there alone, '
        'without a folder, the newest only', () async {
      var now = DateTime.utc(2026, 10, 1, 9);
      final store = Store.inMemory(clock: () => now);
      addTearDown(store.close);
      final auto = Directory('${dir.path}/auto')..createSync();
      final work = Directory('${dir.path}/work')..createSync();
      final copy = LocalBackupFolder(auto.path);
      BackupFolder open(String path) => LocalBackupFolder(path);
      await setUpBackups(store, passphrase, kdf: cheap);
      expect(store.autoBackupOn, isFalse);
      expect(store.backupsScheduled, isFalse);
      expect(
        await backUpIfDue(store, open, work.path, autoBackup: copy),
        isNull,
      );
      expect(auto.listSync(), isEmpty);

      store.autoBackupOn = true;
      expect(store.backupsScheduled, isTrue);
      // A Backup of an older Device, restored from there, makes way.
      File('${auto.path}/oldbrainfix-20200101-000000-ffffffff.obfbackup')
          .writeAsStringSync('an older Device');
      final first = await backUpIfDue(store, open, work.path, autoBackup: copy);
      expect(await copy.list(), [first]);
      now = now.add(const Duration(days: 7));
      final second = await backUpIfDue(
        store,
        open,
        work.path,
        autoBackup: copy,
      );
      expect(second, isNot(first));
      expect(await copy.list(), [second]);
      expect(readBackupHeader('${auto.path}/$second')?.created, now);

      // With a folder too, both get it.
      final folder = Directory('${dir.path}/folder')..createSync();
      store.backupFolder = folder.path;
      now = now.add(const Duration(days: 7));
      final third = await backUpIfDue(store, open, work.path, autoBackup: copy);
      expect(await LocalBackupFolder(folder.path).list(), [third]);
      expect(await copy.list(), [third]);

      // Opted out: the folder alone.
      store.autoBackupOn = false;
      now = now.add(const Duration(days: 7));
      final fourth = await backUpIfDue(
        store,
        open,
        work.path,
        autoBackup: copy,
      );
      expect(await copy.list(), [third]);
      expect(await LocalBackupFolder(folder.path).list(), [third, fourth]);
      expect(store.backupFolder, isNotNull);
    });

    test('a Backup needs somewhere to go', () async {
      final store = Store.inMemory(clock: ticking());
      addTearDown(store.close);
      await setUpBackups(store, passphrase, kdf: cheap);
      expect(() => backUpNow(store, null, dir.path), throwsArgumentError);
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
