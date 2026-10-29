import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/first_start_screen.dart';
import 'package:oldbrainfix/src/lock_platform.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix/src/profile_card.dart';
import 'package:oldbrainfix/src/profile_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_lock.dart';
import 'fake_photos.dart';
import 'fake_profile.dart';
import 'open_settings.dart';

/// Reads the QR the test says is in any picture.
class FakePairing implements PairingPlatform {
  String? text;
  final read = <Uint8List>[];

  @override
  Future<String?> readQr(Uint8List photo) async {
    read.add(photo);
    return text;
  }

  @override
  Future<String> deviceName() async => 'Pixel';
}

void main() {
  late Store store;
  late Store ann;
  late FakeProfilePlatform platform;
  late FakePhotos photos;
  late FakePairing pairing;

  setUp(() {
    store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Ben');
    ann = Store.inMemory()..applyProfileDefaults(name: 'Ann');
    platform = FakeProfilePlatform(name: 'Ben');
    photos = FakePhotos(
      nextPhoto: Uint8List.fromList([1]),
      nextPicked: [
        Uint8List.fromList([2]),
      ],
    );
    pairing = FakePairing();
  });
  tearDown(() {
    store.close();
    ann.close();
  });

  String annId() => ann.ownProfile!.id;

  Future<void> start(WidgetTester tester, {LockPlatform? lock}) async {
    tester.view.physicalSize = const Size(1080, 2600);
    tester.view.devicePixelRatio = 2;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        alarms: FakeAlarms(),
        profile: platform,
        photos: photos,
        pairing: pairing,
        lock: lock ?? const NoBiometrics(),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> openAddPerson(WidgetTester tester) async {
    await openProfilePage(tester);
    await tap(tester, find.byKey(const ValueKey('addPerson')));
    expect(find.byType(AddPersonScreen), findsOneWidget);
  }

  Future<void> paste(WidgetTester tester, String text) async {
    await tester.enterText(find.byKey(const ValueKey('add-person-link')), text);
    await tap(tester, find.byKey(const ValueKey('add-person-open')));
  }

  /// Taps the failure announced as [announced] and expects its dialog to
  /// say [cause] and [fix], with Close and Report; closes it.
  Future<void> expectFailure(
    WidgetTester tester,
    String announced, {
    required String title,
    required String cause,
    required String fix,
  }) async {
    await tap(tester, find.text(announced));
    expect(find.text(title), findsOneWidget);
    expect(find.text('$cause $fix'), findsOneWidget);
    expect(find.byKey(const ValueKey('failureDetails')), findsOneWidget);
    expect(find.text('Report'), findsOneWidget);
    await tap(tester, find.text('Close'));
    expect(find.byType(AlertDialog), findsNothing);
  }

  /// The person page on top.
  PersonScreen shownPerson(WidgetTester tester) =>
      tester.widget<PersonScreen>(find.byType(PersonScreen));

  group('the own card', () {
    testWidgets('"Send my Profile" shares the link last, nothing after it', (
      tester,
    ) async {
      await start(tester);
      await openProfilePage(tester);
      await tap(tester, find.byKey(const ValueKey('sendProfile')));
      expect(platform.shared, hasLength(1));
      final shared = platform.shared.single;
      expect(shared, endsWith(store.profileCard()!));
      expect(shared, startsWith('My Profile for Old Brain Fix:\n'));
      expect(ann.openProfileCard(shared).taken, CardTaken.added);
    });

    testWidgets('no app to take it is announced, why and what to do '
        'behind the tap', (tester) async {
      platform.shares = false;
      await start(tester);
      await openProfilePage(tester);
      await tap(tester, find.byKey(const ValueKey('sendProfile')));
      expect(find.text('No app here can take the link.'), findsNothing);
      await expectFailure(
        tester,
        'Your Profile card was not sent.',
        title: 'Profile card not sent',
        cause: 'No app here can take the link.',
        fix:
            'Show your QR for them to scan instead, or install a messenger '
            'and send it again.',
      );
    });

    testWidgets('"Show my QR" shows the link as a QR that holds it', (
      tester,
    ) async {
      await start(tester);
      await openProfilePage(tester);
      await tap(tester, find.byKey(const ValueKey('showProfileQr')));
      expect(find.byType(ProfileQrScreen), findsOneWidget);
      final screen = tester.widget<ProfileQrScreen>(
        find.byType(ProfileQrScreen),
      );
      expect(screen.link, store.profileCard());
      expect(find.byKey(const ValueKey('profile-qr')), findsOneWidget);
      // At error correction L, a one-Device card is far from the limit.
      final modules = profileCardQr(screen.link)!;
      expect(modules.length, lessThan(40 * 4 + 17));
      expect(modules.length, greaterThan(21));
    });

    testWidgets('a QR too long for any version says so', (tester) async {
      expect(profileCardQr('x' * 3000), isNull);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: ProfileQrScreen(link: 'x' * 3000),
        ),
      );
      expect(find.byKey(const ValueKey('profile-qr-long')), findsOneWidget);
    });

    testWidgets('without a bound key there is no card yet: sync first', (
      tester,
    ) async {
      store.close();
      // A Device with no Profile of its own yet makes no card.
      store = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true;
      final announcements = Announcements();
      addTearDown(announcements.dispose);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) => AnnouncementStrip(
            announcements: announcements,
            child: AnnouncementsScope(
              announcements: announcements,
              child: child!,
            ),
          ),
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () => sendProfileCard(context, store, platform),
                child: const Text('send'),
              ),
            ),
          ),
        ),
      );
      expect(store.profileCard(), isNull);
      await tester.tap(find.text('send'));
      await tester.pumpAndSettle();
      expect(platform.shared, isEmpty);
      expect(announcements.visible, 'Your Profile card could not be made.');
      await tester.tap(find.text('Your Profile card could not be made.'));
      await tester.pumpAndSettle();
      expect(find.text('No Profile card yet'), findsOneWidget);
      expect(
        find.text(
          'This Device joins your Profile only once it has synced with '
          'another of your Devices. Sync with your other Device first, '
          'then try again.',
        ),
        findsOneWidget,
      );
      expect(find.text('Report'), findsOneWidget);
    });
  });

  group('adding a person', () {
    testWidgets('a pasted link adds them, not verified, and offers "Send '
        'yours back", which shares the answer', (tester) async {
      await start(tester);
      await openAddPerson(tester);
      await paste(tester, 'Here: ${ann.profileCard()!}');
      expect(find.byType(AddPersonScreen), findsNothing);
      expect(shownPerson(tester).id, annId());
      expect(shownPerson(tester).offerBack, isTrue);
      expect(store.personVerified(annId()), isFalse);
      expect(
        find.text('Not verified: compare codes in person to be sure.'),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('compare-codes')), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('send-back')));
      expect(platform.shared.single, endsWith(store.profileCard(back: true)!));
      expect(find.byKey(const ValueKey('send-back')), findsNothing);
      // The answer adds Ben on Ann's side and asks nothing back.
      final back = ann.openProfileCard(platform.shared.single);
      expect(back.taken, CardTaken.added);
      expect(back.offerBack, isFalse);
    });

    testWidgets('a QR scanned with the camera adds them verified', (
      tester,
    ) async {
      pairing.text = ann.profileCard();
      await start(tester);
      await openAddPerson(tester);
      await tap(tester, find.byKey(const ValueKey('add-person-scan')));
      expect(photos.taken, 1);
      expect(shownPerson(tester).id, annId());
      expect(store.personVerified(annId()), isTrue);
      expect(find.byKey(const ValueKey('compare-codes')), findsNothing);
    });

    testWidgets('a QR in a picture adds them, not verified', (tester) async {
      pairing.text = ann.profileCard();
      await start(tester);
      await openAddPerson(tester);
      await tap(tester, find.byKey(const ValueKey('add-person-picture')));
      expect(photos.picks, 1);
      expect(shownPerson(tester).id, annId());
      expect(store.personVerified(annId()), isFalse);
    });

    testWidgets('a picture with no QR says so and stays', (tester) async {
      await start(tester);
      await openAddPerson(tester);
      await tap(tester, find.byKey(const ValueKey('add-person-scan')));
      await expectFailure(
        tester,
        'The picture could not be read.',
        title: 'No QR code found',
        cause: 'No QR code was found in the picture.',
        fix: 'Try a picture taken a little closer, or ask for the link.',
      );
      expect(find.byType(AddPersonScreen), findsOneWidget);
    });

    testWidgets('a card opened again changes nothing and offers nothing', (
      tester,
    ) async {
      store.openProfileCard(ann.profileCard()!);
      await start(tester);
      await openAddPerson(tester);
      await paste(tester, ann.profileCard()!);
      expect(shownPerson(tester).id, annId());
      expect(shownPerson(tester).offerBack, isFalse);
      expect(find.byKey(const ValueKey('send-back')), findsNothing);
    });

    testWidgets('a newer card brings the new name', (tester) async {
      store.openProfileCard(ann.profileCard()!);
      ann.renameOwnProfile('Annie');
      await start(tester);
      await openAddPerson(tester);
      await paste(tester, ann.profileCard()!);
      expect(shownPerson(tester).id, annId());
      expect(find.byKey(const ValueKey('personName')), findsOneWidget);
      expect(find.text('Annie'), findsWidgets);
      expect(shownPerson(tester).offerBack, isFalse);
    });

    testWidgets('the own card adds nothing and says so', (tester) async {
      await start(tester);
      await openAddPerson(tester);
      await paste(tester, store.profileCard()!);
      await expectFailure(
        tester,
        'Nobody was added from the card.',
        title: 'Nobody added',
        cause: 'This is your own Profile card.',
        fix: 'To add someone, open their card, not yours.',
      );
      expect(find.byType(PersonScreen), findsNothing);
      expect(store.peopleShown, isEmpty);
    });

    testWidgets('a blocked person\'s card adds nothing and says so', (
      tester,
    ) async {
      store.openProfileCard(ann.profileCard()!);
      store.blockPerson(annId(), leaveAreas: false);
      await start(tester);
      await openAddPerson(tester);
      await paste(tester, ann.profileCard()!);
      await expectFailure(
        tester,
        'Nobody was added from the card.',
        title: 'Nobody added',
        cause: 'This card is from a person you blocked.',
        fix:
            'To take what they send, unblock them on their Profile under '
            'People, then ask them to send it again.',
      );
      expect(find.byType(PersonScreen), findsNothing);
    });

    testWidgets('a card that is somebody else\'s says so', (tester) async {
      await start(tester);
      await openAddPerson(tester);
      final context = tester.element(find.byType(AddPersonScreen));
      await showCardOpened(context, (
        taken: CardTaken.refused,
        id: annId(),
        offerBack: false,
      ), replace: true);
      await tester.pumpAndSettle();
      await expectFailure(
        tester,
        'Nobody was added from the card.',
        title: 'Nobody added',
        cause: 'This card names a person in People but was not made by them.',
        fix: 'Ask the person for their card again, in person if you can.',
      );
      expect(find.byType(PersonScreen), findsNothing);
    });

    testWidgets('pasted text that is no card is the field\'s error', (
      tester,
    ) async {
      await start(tester);
      await openAddPerson(tester);
      final link = ann.profileCard()!;
      await paste(tester, '${link.substring(0, link.length - 4)}AAAA');
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('add-person-link')),
          matching: find.text(
            'This is not a Profile card, or it was changed on the way. Ask '
            'for the link again.',
          ),
        ),
        findsOneWidget,
      );
      expect(find.byType(PersonScreen), findsNothing);
      expect(store.peopleShown, isEmpty);
    });
  });

  group('a link tapped in another app', () {
    testWidgets('opens the person\'s Profile, not verified', (tester) async {
      await start(tester);
      platform.tapCard(ann.profileCard()!);
      await tester.pumpAndSettle();
      expect(shownPerson(tester).id, annId());
      expect(shownPerson(tester).offerBack, isTrue);
      expect(store.personVerified(annId()), isFalse);
    });

    testWidgets('that started the app opens once it shows', (tester) async {
      platform.tappedBefore(ann.profileCard()!);
      await start(tester);
      expect(shownPerson(tester).id, annId());
    });

    testWidgets('that is no card says so', (tester) async {
      await start(tester);
      platform.tapCard('$profileCardLink.');
      await tester.pumpAndSettle();
      await expectFailure(
        tester,
        'Nobody was added from the card.',
        title: 'Nobody added',
        cause: 'This is not a Profile card, or it was changed on the way.',
        fix: 'Ask for the link again.',
      );
    });

    testWidgets('waits for the first start to make the own Profile', (
      tester,
    ) async {
      store.close();
      store = Store.inMemory();
      platform.tappedBefore(ann.profileCard()!);
      await start(tester);
      expect(find.byType(FirstStartScreen), findsOneWidget);
      expect(find.byType(PersonScreen), findsNothing);
      expect(store.currentSeq(), 0, reason: 'nothing kept before the choice');

      await tap(tester, find.byKey(const ValueKey('first-new')));
      await tap(tester, find.text('Start'));
      expect(store.ownProfile, isNotNull);
      expect(shownPerson(tester).id, annId());
    });

    testWidgets('waits behind the App Lock, then opens', (tester) async {
      store
        ..setPin('2468', iterations: 10)
        ..appLockOn = true;
      await start(tester, lock: FakeBiometrics(can: false));
      platform.tapCard(ann.profileCard()!);
      await tester.pumpAndSettle();
      expect(find.byType(PersonScreen), findsNothing);
      expect(store.peopleShown, isEmpty);

      await tester.enterText(find.byKey(const ValueKey('lockPin')), '2468');
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      expect(shownPerson(tester).id, annId());
    });
  });

  testWidgets('the Profile page lists the card actions under the code', (
    tester,
  ) async {
    await start(tester);
    await openProfilePage(tester);
    expect(find.byType(ProfileScreen), findsOneWidget);
    final code = tester.getTopLeft(find.byKey(const ValueKey('profileCode')));
    final send = tester.getTopLeft(find.byKey(const ValueKey('sendProfile')));
    final qr = tester.getTopLeft(find.byKey(const ValueKey('showProfileQr')));
    expect(code.dy, lessThan(send.dy));
    expect(send.dy, lessThan(qr.dy));
  });
}
