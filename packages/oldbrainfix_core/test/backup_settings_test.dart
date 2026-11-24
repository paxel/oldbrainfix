import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A cost tests can afford.
const cheap = BackupKdf(memory: 64, iterations: 1);

const passphrase = 'correct horse battery';

/// A clock one second further on every read.
DateTime Function() ticking([DateTime? from]) {
  var now = from ?? DateTime.utc(2026, 10, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

/// Every local setting [store] holds, by key.
Map<String, String> settings(Store store) => {
  for (final (k, v) in store.localSettingsWithPrefix('')) k: v,
};

/// The settings the carried list may write, sorted, as the stores keep
/// them: none of the identity, the keys or the Profile.
Map<String, String> plain(Store store) => {
  for (final MapEntry(:key, :value) in settings(store).entries)
    if (!key.startsWith('areakey:') &&
        !key.startsWith('key:') &&
        !key.startsWith('backup:') &&
        !key.startsWith('profile') &&
        key != 'device' &&
        key != 'signseed')
      key: value,
};

void main() {
  late Directory dir;

  setUp(() => dir = Directory.systemTemp.createTempSync('backup_settings'));
  tearDown(() => dir.deleteSync(recursive: true));

  Store fresh(String name) {
    final home = Directory('${dir.path}/$name')..createSync();
    return Store.open('${home.path}/new.db', clock: ticking());
  }

  /// A Backup of [old], restored on a new Device.
  Future<Store> restored(Store old) async {
    final code = await setUpBackups(old, passphrase, kdf: cheap);
    final path = '${dir.path}/b.$backupExtension';
    await writeBackup(old, path);
    final store = fresh('new');
    addTearDown(store.close);
    await restoreBackup(store, path, decodeRecoveryCode(code)!);
    return store;
  }

  /// A Backup of nothing but [old]'s identity and [records], sealed as
  /// one last chunk, and its backup key.
  Future<(String, Uint8List)> crafted(
    Store old,
    List<Map<String, Object?>> records,
  ) async {
    final code = await setUpBackups(old, passphrase, kdf: cheap);
    final backupKey = decodeRecoveryCode(code)!;
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
      ...records,
    ].map((r) => '${jsonEncode(r)}\n').join();
    final hash = sha256.convert(line).toString();
    final file = File('${dir.path}/crafted.$backupExtension')
      ..writeAsBytesSync([
        ...line,
        0x0a,
        ...seal(
          contentKey,
          utf8.encode(payload),
          aad: SealedAs.backupChunk(hash, 0, last: true),
        ),
      ]);
    return (file.path, backupKey);
  }

  /// A Device with a setting of every kind carried, a desktop's among
  /// them, and those bound to it.
  Store everything() {
    final store = Store.open('${dir.path}/old.db', clock: ticking());
    addTearDown(store.close);
    store
      ..themeChoice = ThemeChoice.dark
      ..textSize = TextSize.largest
      ..fatFingers = true
      ..renagInterval = const Duration(minutes: 15)
      ..nudgeRepeat = const Duration(minutes: 30)
      ..quietHours = QuietHours(ClockTime(21, 30), ClockTime(7, 0))
      ..quietHoursOn = false
      ..allDayFrom = ClockTime(9, 5)
      ..streaksOn = false
      ..agendaHidesChecked = true
      ..twoRowEditorOn = false
      ..quickNoteNudgeButtonsOn = false
      ..chosenUnitSystem = UnitSystem.imperial
      ..playbackSpeed = 2.0
      ..speechModelChoice = (name: 'small', source: SpeechModelSource.file)
      ..setLocalSetting('tipsOff', 'true')
      ..setLocalSetting('recordingPauseSeconds', '5')
      // As the desktop keeps them.
      ..setLocalSetting('alertsOff', 'true')
      ..setLocalSetting('ask:on', 'true')
      ..setLocalSetting('ask:openai:url', 'https://example.org/v1')
      ..setLocalSetting('captureHotkey', 'Ctrl+Alt+M')
      ..setLocalSetting('autostartOff', 'true')
      // Bound to this Device.
      ..chooseSyncFolder('/somewhere/sync')
      ..backupFolder = '/somewhere/backups'
      ..autoBackupOn = true
      ..setPin('1234', iterations: 1)
      ..appLockOn = true
      ..biometricsOn = true
      ..alarmSound = 'content://media/internal/audio/media/42'
      ..calendarContextOn = true
      ..bluetoothContextOn = true
      ..agendaCalendarOn = true
      ..agendaCalendars = {'7'}
      ..askOn = true
      ..alarmSetupDone = true
      ..starterSetOffered = true
      ..lastShoppingList = 'n1'
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

  group('a Backup carries the settings', () {
    test('that make sense on a new phone, and its speech model as an offer, '
        'none bound to the old Device', () async {
      final store = await restored(everything());
      expect(plain(store), {
        'agendaHideChecked': 'true',
        'allDayFrom': '09:05',
        'fatFingers': 'true',
        'nudgeRepeatMinutes': '30',
        'playbackSpeed': '2.0',
        'quickNoteNudgeButtonsOff': 'true',
        'quietHours': '21:30-07:00',
        'quietHoursOff': 'true',
        'recordingPauseSeconds': '5',
        'renagMinutes': '15',
        'speechModelOffer': 'small:file',
        'streaksOff': 'true',
        'textSize': 'largest',
        'theme': 'dark',
        'tipsOff': 'true',
        'twoRowEditorOff': 'true',
        'unitSystem': 'imperial',
      });
      expect(store.themeChoice, ThemeChoice.dark);
      expect(store.quietHours.start, ClockTime(21, 30));
      expect(store.quietHoursOn, isFalse);
      // The model's file stayed behind: it is offered, not in use.
      expect(store.speechModelChoice, isNull);
      expect(store.speechModelOffer, (
        name: 'small',
        source: SpeechModelSource.file,
      ));
      expect(store.hasPin, isFalse);
      expect(store.appLockOn, isFalse);
      expect(store.syncFolder, isNull);
      expect(store.backupFolder, isNull);
      expect(store.autoBackupOn, isFalse);
      expect(store.pairedDevices(), isEmpty);
      expect(store.alarmSound, isNull);
      expect(store.askOn, isFalse);
    });

    test('in one record after the Profile, keys sorted, for either app', () {
      final store = everything();
      expect(carriedSettings(store), {
        'agendaHideChecked': 'true',
        'alertsOff': 'true',
        'allDayFrom': '09:05',
        'fatFingers': 'true',
        'nudgeRepeatMinutes': '30',
        'quickNoteNudgeButtonsOff': 'true',
        'quietHours': '21:30-07:00',
        'quietHoursOff': 'true',
        'recordingPauseSeconds': '5',
        'renagMinutes': '15',
        'speechModel': 'small:file',
        'streaksOff': 'true',
        'textSize': 'largest',
        'theme': 'dark',
        'tipsOff': 'true',
        'twoRowEditorOff': 'true',
        'unitSystem': 'imperial',
      });
      expect(
        jsonEncode(carriedSettings(store)),
        startsWith('{"agendaHideChecked":"true","alertsOff":"true",'),
      );
    });

    test('each app takes only those it has', () {
      final record = {'alertsOff': 'true', 'twoRowEditorOff': 'true'};
      expect(takenSettings(record, SettingsApp.phone), [
        ('twoRowEditorOff', 'true'),
      ]);
      expect(takenSettings(record, SettingsApp.desktop), [
        ('alertsOff', 'true'),
      ]);
    });

    test('a speech model still offered goes into the next Backup', () async {
      final old = Store.inMemory()
        ..speechModelChoice = (
          name: 'base',
          source: SpeechModelSource.download,
        );
      addTearDown(old.close);
      final once = await restored(old);
      expect(carriedSettings(once), {'speechModel': 'base:download'});
      once.dropSpeechModelOffer();
      expect(once.speechModelOffer, isNull);
      expect(carriedSettings(once), isEmpty);
    });

    test('a value not offered, written otherwise or not as text is passed '
        'over, also when written', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      for (final (key, value) in [
        ('theme', 'Dark'),
        ('textSize', 'huge'),
        ('renagMinutes', '0'),
        ('nudgeRepeatMinutes', '010'),
        ('quietHours', '22:00-8:00'),
        ('allDayFrom', '24:00'),
        ('recordingPauseSeconds', '4'),
        ('fatFingers', 'yes'),
        ('speechModel', 'large:download'),
        ('shoppingAgenda', 'all'),
        ('unitSystem', 'nautical'),
      ]) {
        store.setLocalSetting(key, value);
      }
      expect(carriedSettings(store), isEmpty);
      expect(
        takenSettings({
          'theme': 'Dark',
          'renagMinutes': '+5',
          'nudgeRepeatMinutes': '1441',
          'allDayFrom': '7:00',
          'fatFingers': true,
          'textSize': 1,
          'speechModel': 'base',
          'streaksOff': 'true',
        }, SettingsApp.phone),
        [('streaksOff', 'true')],
      );
      expect(takenSettings('theme', SettingsApp.phone), isEmpty);
    });

    test('a crafted record sets nothing but the settings carried', () async {
      final old = Store.open('${dir.path}/old.db', clock: ticking());
      addTearDown(old.close);
      final (path, key) = await crafted(old, [
        {
          'settings': {
            'signseed': '00' * 32,
            'device': 'a' * 32,
            'author': 'Mallory',
            'lockPin': 'pbkdf2-sha256:1:aa:bb',
            'appLock': 'true',
            'lockedAreas': 'area1',
            'backup:key': 'AAAA',
            'backup:folder': '/elsewhere',
            'backup:auto': 'on',
            'folder': '/elsewhere',
            'paired:x': '{}',
            'key:x': '{}',
            'askAllowed': '["anthropic"]',
            'ask:on': 'true',
            'ask:openai:url': 'https://example.org/v1',
            'speechModelOffer': 'small:file',
            'alarmSound': 'file:///elsewhere',
            'theme': 'light',
          },
        },
      ]);
      final store = fresh('new');
      addTearDown(store.close);
      final before = settings(store);
      await restoreBackup(store, path, key);
      final after = settings(store);
      expect(after['theme'], 'light');
      for (final kept in ['device', 'signseed', 'author']) {
        expect(after[kept], before[kept]);
      }
      expect(plain(store), {'theme': 'light'});
      expect(store.hasPin, isFalse);
      expect(store.backupFolder, isNull);
      expect(store.speechModelOffer, isNull);
    });

    test('a Backup without the record, or with one that is no map, '
        'restores as before', () async {
      final old = Store.open('${dir.path}/old.db', clock: ticking());
      addTearDown(old.close);
      final (path, key) = await crafted(old, [
        {'settings': 'theme=dark'},
      ]);
      final store = fresh('new');
      addTearDown(store.close);
      await restoreBackup(store, path, key);
      expect(plain(store), isEmpty);
    });

    // As both apps make the Profile at their first start, every Backup
    // carries one, and with it the old Device counts as one's own.
    test('Simple and the other settings of the person come back with the '
        'log', () async {
      final old = Store.open('${dir.path}/old.db', clock: ticking());
      addTearDown(old.close);
      old
        ..applyProfileDefaults(name: 'Ada')
        ..advanced = false
        ..trashesDoneReminder = true;
      final store = await restored(old);
      expect(store.advanced, isFalse);
      expect(store.trashesDoneReminder, isTrue);
    });

    test('opening a Backup takes none of them', () async {
      final old = everything();
      Notes(old).capture('Pills at eight');
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final path = '${dir.path}/b.$backupExtension';
      await writeBackup(old, path);
      final store = fresh('open');
      addTearDown(store.close);
      final before = settings(store);
      await openBackupNotes(store, path, decodeRecoveryCode(code)!);
      expect(settings(store), before);
    });
  });

  group('the speech model', () {
    test('is kept with how it came, and none is refused otherwise', () {
      final store = Store.inMemory();
      addTearDown(store.close);
      expect(store.speechModelChoice, isNull);
      store.speechModelChoice = (
        name: 'base',
        source: SpeechModelSource.download,
      );
      expect(store.localSetting('speechModel'), 'base:download');
      expect(store.speechModelChoice, (
        name: 'base',
        source: SpeechModelSource.download,
      ));
      expect(
        () => store.speechModelChoice = (
          name: 'large',
          source: SpeechModelSource.file,
        ),
        throwsArgumentError,
      );
      store.speechModelChoice = null;
      expect(store.localSetting('speechModel'), isNull);
      store.setLocalSetting('speechModel', 'base:cloud');
      expect(store.speechModelChoice, isNull);
    });
  });
}
