import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';
import 'open_settings.dart';

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

    await tester.ensureVisible(find.text('Forget'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Forget'));
    await tester.pumpAndSettle();
    expect(phone.pairedDevices(), isEmpty);
    expect(find.text('No paired Devices yet.'), findsOneWidget);
  });

  group('"Which one is you?"', () {
    Future<void> pair(WidgetTester tester) async {
      await open(tester);
      await tester.enterText(find.byKey(const ValueKey('code')), code);
      await tester.tap(find.byKey(const ValueKey('join')));
      await tester.pumpAndSettle();
    }

    Finder inChoice(String key, Finder finder) =>
        find.descendant(of: find.byKey(ValueKey(key)), matching: finder);

    testWidgets('pairing two Devices whose Profiles were both named asks '
        'once, showing both, and keeps the one picked', (tester) async {
      phone
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ada L.');
      desk
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ada');
      final deskId = desk.ownProfile!.id;
      final phoneId = phone.ownProfile!.id;
      await pair(tester);
      expect(find.text('Which one is you?'), findsOneWidget);
      expect(inChoice('whichMine', find.text('Ada L.')), findsOneWidget);
      expect(
        inChoice('whichMine', find.text(phone.ownProfileCode!)),
        findsOneWidget,
      );
      expect(
        inChoice('whichMine', find.text('On this Device')),
        findsOneWidget,
      );
      expect(inChoice('whichOther', find.text('Ada')), findsOneWidget);
      expect(
        inChoice('whichOther', find.text(desk.ownProfileCode!)),
        findsOneWidget,
      );
      await tester.tap(inChoice('whichOther', find.text('This is me')));
      await tester.pumpAndSettle();
      expect(find.text('Which one is you?'), findsNothing);
      expect(phone.ownProfile!.id, deskId);
      expect(phone.ownProfileCode, desk.ownProfileCode);
      expect(phone.profileQuestion, isNull);

      // The next sync carries the answer: the computer asks no more, and
      // neither does the phone.
      await tester.enterText(find.byKey(const ValueKey('code')), code);
      await tester.tap(find.byKey(const ValueKey('join')));
      await tester.pumpAndSettle();
      expect(find.text('Which one is you?'), findsNothing);
      expect(desk.profileQuestion, isNull);
      expect(desk.ownProfileAliases, {phoneId});
    });

    testWidgets('keeping this Device\'s Profile keeps it, and the computer '
        'takes it over at the next sync', (tester) async {
      phone
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ada L.');
      desk
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ada');
      final phoneId = phone.ownProfile!.id;
      await pair(tester);
      await tester.tap(inChoice('whichMine', find.text('This is me')));
      await tester.pumpAndSettle();
      expect(phone.ownProfile!.id, phoneId);
      await tester.enterText(find.byKey(const ValueKey('code')), code);
      await tester.tap(find.byKey(const ValueKey('join')));
      await tester.pumpAndSettle();
      expect(desk.ownProfile!.id, phoneId);
      expect(desk.ownProfile!.name, 'Ada L.');
    });

    testWidgets('a phone with the defaults takes the computer\'s Profile '
        'over without a question', (tester) async {
      phone.applyProfileDefaults(name: 'Pixel');
      desk
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ada');
      await pair(tester);
      expect(find.text('Which one is you?'), findsNothing);
      expect(find.byType(Dialog), findsNothing);
      expect(phone.ownProfile!.id, desk.ownProfile!.id);
      expect(phone.ownProfile!.name, 'Ada');
    });
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

  Future<void> syncNow(WidgetTester tester) async {
    await tester.enterText(find.byKey(const ValueKey('code')), code);
    await tester.tap(find.byKey(const ValueKey('join')));
    await tester.pumpAndSettle();
  }

  testWidgets('a sync that finds Conflicts leads to them from the top', (
    tester,
  ) async {
    await open(tester);
    await syncNow(tester);
    final note = Notes(phone).all().single;
    Notes(phone).edit(note.id, 'From the phone');
    Notes(desk).edit(note.id, 'Edited at the desk');
    announcements.dismiss();
    await syncNow(tester);
    expect(announcements.visible, startsWith('Synced with Desk: '));
    announcements.dismiss();
    await tester.pumpAndSettle();
    const found =
        '1 thing was changed on both Devices. Tap to pick which version stays.';
    expect(announcements.visible, found);
    expect(find.text('1 conflict to settle'), findsOneWidget);

    await tester.tap(find.text(found));
    await tester.pumpAndSettle();
    expect(find.byType(ConflictsScreen), findsOneWidget);
    expect(announcements.visible, isNull);
    expect(find.text('Note text'), findsOneWidget);
    expect(find.text('Edited at the desk'), findsOneWidget);
    expect(find.text('From the phone'), findsOneWidget);
    expect(find.textContaining('On Desk, '), findsOneWidget);
    expect(find.textContaining('On this Device, '), findsOneWidget);
    // Meanwhile the newer one shows.
    expect(Notes(phone).byId(note.id)?.text, 'Edited at the desk');

    await tester.tap(find.text('From the phone'));
    await tester.pumpAndSettle();
    expect(find.text('No conflicts. Every Device agrees.'), findsOneWidget);
    expect(Notes(phone).byId(note.id)?.text, 'From the phone');
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.text('No conflicts'), findsOneWidget);

    // The next sync carries the pick to the desk; nothing is open there.
    await syncNow(tester);
    expect(desk.conflicts(), isEmpty);
    expect(Notes(desk).byId(note.id)?.text, 'From the phone');
  });

  testWidgets('the Conflicts show every kind of thing in words', (
    tester,
  ) async {
    final routine = Routines(desk).create(
      'Pills',
      Schedule(
        start: Day(2026, 3, 2),
        times: [ClockTime(8, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    final home = NamedPlaces(desk)
        .add('Home', Coordinates(52.5, 13.4), radius: 100);
    await open(tester);
    await syncNow(tester);
    Routines(phone).reschedule(
      routine.id,
      Schedule(
        start: Day(2026, 3, 2),
        times: [ClockTime(9, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    Routines(desk).reschedule(
      routine.id,
      Schedule(start: Day(2026, 3, 2), times: [ClockTime(8, 0)]),
    );
    NamedPlaces(phone).setRadius(home.id, 250);
    NamedPlaces(desk).setRadius(home.id, 50);
    await syncNow(tester);
    await tester.tap(find.byKey(const ValueKey('conflicts')));
    await tester.pumpAndSettle();
    expect(find.text('Routine schedule'), findsOneWidget);
    expect(find.text('Pills'), findsOneWidget);
    expect(find.textContaining('09:00'), findsOneWidget);
    expect(find.text('Named Place radius'), findsOneWidget);
    expect(find.text('250 m'), findsOneWidget);
    expect(find.text('50 m'), findsOneWidget);
  });

  testWidgets('a Conflict about a closed locked Area stays out', (
    tester,
  ) async {
    final area = Places(phone).addArea('Doctor');
    final note = Notes(phone).capture('Blood test', place: area.id);
    desk.applyEntries(phone.entriesSince(const {}), keys: [phone.ownKeyRecord]);
    Notes(phone).edit(note.id, 'Blood test on Monday');
    Notes(desk).edit(note.id, 'Blood test on Tuesday');
    phone.applyEntries(desk.entriesSince(const {}), keys: [desk.ownKeyRecord]);
    expect(phone.conflicts(), hasLength(1));
    phone.setPin('1234', iterations: 1);
    phone.setAreaLocked(area.id, true);
    final lock = AppLock(store: phone);
    addTearDown(lock.dispose);

    Future<void> show(Key key) async {
      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: LockScope(
            lock: lock,
            child: ConflictsScreen(key: key, store: phone),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    await show(const ValueKey('closed'));
    expect(find.text('Note text'), findsNothing);
    expect(find.text('No conflicts. Every Device agrees.'), findsOneWidget);
    lock.openArea(area.id);
    await show(const ValueKey('open'));
    expect(find.text('Note text'), findsOneWidget);
    expect(find.text('Blood test on Tuesday'), findsOneWidget);
  });

  testWidgets('Sync opens from the Profile page', (tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: phone, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
    await openProfilePage(tester);
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
