import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/first_start_screen.dart';
import 'package:oldbrainfix/src/host_screen.dart';
import 'package:oldbrainfix/src/profile.dart';
import 'package:oldbrainfix/src/profile_screen.dart';
import 'package:oldbrainfix/src/setup_screen.dart';
import 'package:oldbrainfix/src/speech_offer_screen.dart';
import 'package:oldbrainfix/src/starter_set_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_alarms.dart';
import 'fake_profile.dart';
import 'fake_speech.dart';
import 'host_test.dart' show FakeHosts, FakePairing, sessionWith;

/// A code for the user's other Device at 192.168.1.23.
final code = encodePairCode(
  '192.168.1.23',
  45231,
  '042817',
  List.filled(32, 7),
  typed: true,
);

void main() {
  late Store store;
  late Store desk;
  late FakeHosts hosts;
  late Announcements announcements;
  late Directory dir;
  final joined = <PairInfo>[];
  SyncFailure? failure;

  setUp(() {
    store = Store.inMemory();
    desk = Store.inMemory()
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    Places(desk).plantStarterSet({'work'});
    Notes(desk).capture('From the desk');
    hosts = FakeHosts();
    announcements = Announcements();
    dir = Directory.systemTemp.createTempSync('first-start');
    joined.clear();
    failure = null;
  });
  tearDown(() {
    store.close();
    desk.close();
    dir.deleteSync(recursive: true);
  });

  /// What the other Device holds comes in, as a sync brings it, its
  /// Profile included.
  void takeDesk() {
    store.applyEntries(desk.entriesSince(const {}), keys: [desk.ownKeyRecord]);
    store.takeOwnProfile(desk.ownProfileJson, from: desk.signingKey.publicKey);
  }

  Future<SyncResult> join(
    Store into,
    PairInfo info, {
    required String name,
  }) async {
    joined.add(info);
    if (failure case final failed?) throw SyncException(failed);
    takeDesk();
    return SyncResult(
      session: sessionWith('Desk'),
      entriesSent: 0,
      blobsSent: 0,
      blobsReceived: 0,
    );
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        announcements: announcements,
        alarms: FakeAlarms(),
        speech: FakeSpeech(),
        bundles: FakeBundles(dir),
        pairing: FakePairing(),
        profile: FakeProfilePlatform(name: 'Pixel'),
        join: join,
        hosting: hosts.start,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  final newUser = find.byKey(const ValueKey('first-new'));
  final addDevice = find.byKey(const ValueKey('first-device'));
  final restore = find.byKey(const ValueKey('restore-offer'));

  /// After the sync: the speech model offer and the alarm setup still
  /// come, then the Agenda with the other Device's Notes. Neither the
  /// Starter Set nor the Profile's name and picture are asked: the
  /// Profile is the other Device's, not this Device's defaults.
  Future<void> setUpAfterSync(WidgetTester tester) async {
    void nothingAsked() {
      expect(find.byType(StarterSetScreen), findsNothing);
      expect(find.byType(ProfileScreen), findsNothing);
      expect(find.byType(ProfileQuestionDialog), findsNothing);
    }

    nothingAsked();
    expect(find.byType(SpeechOfferScreen), findsOneWidget);
    await tap(tester, find.text('Not now'));
    nothingAsked();
    expect(find.byType(SetupScreen), findsOneWidget);
    expect(store.starterSetOffered, isTrue);
    expect(store.ownProfile?.id, desk.ownProfile!.id);
    expect(store.ownProfile?.name, 'Ann');
    expect(store.profileQuestion, isNull);
    expect(Places(store).areas().map((a) => a.name), ['Work']);
  }

  testWidgets('a fresh install shows the three choices and makes no '
      'Profile before one is picked', (tester) async {
    await start(tester);
    expect(find.byType(FirstStartScreen), findsOneWidget);
    expect(find.text('New to Old Brain Fix'), findsOneWidget);
    expect(find.text('Add a Device of mine'), findsOneWidget);
    expect(find.text('Restore from a Backup'), findsOneWidget);
    expect(restore, findsOneWidget);
    expect(find.byType(StarterSetScreen), findsNothing);
    expect(store.ownProfile, isNull);
  });

  testWidgets('New shows the Starter Set, Back the first screen again, and '
      'the Profile is made once it is planted', (tester) async {
    await start(tester);
    await tap(tester, newUser);
    expect(find.byType(StarterSetScreen), findsOneWidget);
    expect(store.ownProfile, isNull);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.byType(FirstStartScreen), findsOneWidget);

    await tap(tester, newUser);
    await tap(tester, find.text('Start'));
    expect(find.byType(SpeechOfferScreen), findsOneWidget);
    expect(store.ownProfile?.name, 'Pixel');
  });

  testWidgets('Restore a Backup opens the restore', (tester) async {
    await start(tester);
    await tap(tester, restore);
    expect(find.text('Restore from a Backup'), findsWidgets);
    expect(find.byType(FirstStartScreen, skipOffstage: false), findsOne);
    expect(store.ownProfile, isNull);
  });

  testWidgets('joining the other Device skips the Starter Set and goes on '
      'with the setup', (tester) async {
    await start(tester);
    await tap(tester, addDevice);
    expect(find.byType(SyncScreen), findsOneWidget);
    expect(find.text('Add a Device of mine'), findsOneWidget);
    // Only the ways to pair; the rest of Sync is for later.
    expect(find.byKey(const ValueKey('conflicts')), findsNothing);
    expect(store.ownProfile, isNull);

    await tester.enterText(find.byKey(const ValueKey('code')), code);
    await tap(tester, find.byKey(const ValueKey('join')));
    expect(joined.single.pin, '042817');
    await setUpAfterSync(tester);

    while (find.text('Done').evaluate().isEmpty) {
      await tap(tester, find.text('Next'));
    }
    await tap(tester, find.text('Done'));
    expect(find.byType(AgendaScreen), findsOneWidget);
    expect(find.byType(StarterSetScreen), findsNothing);
    expect(find.byType(ProfileScreen), findsNothing);
    expect(store.ownProfile?.name, 'Ann');
    // The other Device's Note, Unsorted there and here.
    expect(Notes(store).all().single.text, 'From the desk');
    expect(find.textContaining('1 Unsorted'), findsOneWidget);
  });

  testWidgets('a failed join says why and returns to the first screen with '
      'nothing written', (tester) async {
    failure = SyncFailure.wrongPin;
    await start(tester);
    await tap(tester, addDevice);
    await tester.enterText(find.byKey(const ValueKey('code')), code);
    await tap(tester, find.byKey(const ValueKey('join')));

    expect(find.byType(FirstStartScreen), findsOneWidget);
    expect(announcements.visible, isNotNull);
    expect(store.currentSeq(), 0);
    expect(store.starterSetOffered, isFalse);
    expect(store.ownProfile, isNull);
  });

  testWidgets('a typed code that is none stays on the page', (tester) async {
    await start(tester);
    await tap(tester, addDevice);
    await tester.enterText(find.byKey(const ValueKey('code')), 'nonsense');
    await tap(tester, find.byKey(const ValueKey('join')));
    expect(find.byType(SyncScreen), findsOneWidget);
    expect(joined, isEmpty);
  });

  testWidgets('hosting for the other Device: once it synced, Continue goes '
      'on with the setup and the host stops', (tester) async {
    await start(tester);
    await tap(tester, addDevice);
    await tap(tester, find.byKey(const ValueKey('show-code')));
    expect(find.byType(HostScreen), findsOneWidget);
    expect(find.byKey(const ValueKey('host-continue')), findsNothing);

    takeDesk();
    hosts.onSession(sessionWith('Desk'));
    await tester.pumpAndSettle();
    await tap(tester, find.byKey(const ValueKey('host-continue')));

    expect(hosts.last.stopped, isTrue);
    await setUpAfterSync(tester);
  });

  testWidgets('turning the Device asking away stops the host and returns '
      'to the first screen', (tester) async {
    await start(tester);
    await tap(tester, addDevice);
    await tap(tester, find.byKey(const ValueKey('show-code')));

    final asked = hosts.ask(
      const JoinAsk(device: 'tablet', name: 'Tablet'),
      null,
    );
    await tester.pumpAndSettle();
    await tap(tester, find.byKey(const ValueKey('host-decline')));
    expect(await asked, isFalse);

    expect(find.byType(FirstStartScreen), findsOneWidget);
    expect(hosts.last.stopped, isTrue);
    expect(store.starterSetOffered, isFalse);
    expect(store.ownProfile, isNull);
  });

  testWidgets('Back from the pairing page returns to the first screen', (
    tester,
  ) async {
    await start(tester);
    await tap(tester, addDevice);
    await tap(tester, find.byKey(const ValueKey('show-code')));
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(hosts.last.stopped, isTrue);
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(FirstStartScreen), findsOneWidget);
    expect(store.ownProfile, isNull);
  });

  testWidgets('a later start, past the first screen, makes the Profile at '
      'once', (tester) async {
    store
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    await start(tester);
    expect(find.byType(FirstStartScreen), findsNothing);
    expect(store.ownProfile?.name, 'Pixel');
  });
}
