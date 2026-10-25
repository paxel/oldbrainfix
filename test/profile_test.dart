import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/backup_screen.dart';
import 'package:oldbrainfix/src/profile.dart';
import 'package:oldbrainfix/src/profile_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_profile.dart';
import 'open_settings.dart';

/// Reads and makes pictures right here, as the tests run on one isolate.
final inline = PictureWork(
  size: (bytes) async => pictureSize(bytes),
  make: (bytes, left, top, size) async =>
      makeProfilePicture(bytes, left: left, top: top, size: size),
);

void main() {
  late Store store;
  late FakeProfilePlatform platform;

  setUp(() {
    store = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    platform = FakeProfilePlatform(name: 'Ada Pixel');
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, alarms: FakeAlarms(), profile: platform),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Finder inAgendaBar(Finder finder) => find.descendant(
    of: find.byType(AppBar),
    matching: find.descendant(of: profileButton, matching: finder),
  );

  group('the own Profile', () {
    testWidgets('is made from the phone\'s name at the first start, asking '
        'nothing', (tester) async {
      await start(tester);
      final profile = store.ownProfile!;
      expect(profile.name, 'Ada Pixel');
      expect(profile.picture, isNull);
      expect(find.byType(Dialog), findsNothing);
      expect(store.allEntries(), isEmpty, reason: 'nothing leaves the Device');
      // Its initials on its colour, where the gear was.
      expect(inAgendaBar(find.text('AP')), findsOneWidget);
      final circle = tester.widget<Container>(
        inAgendaBar(find.byKey(const ValueKey('profilePlaceholder'))),
      );
      expect(
        (circle.decoration! as BoxDecoration).color,
        Color(profileColor(profile.id)),
      );
    });

    testWidgets('follows a renamed phone until the user typed a name', (
      tester,
    ) async {
      await start(tester);
      platform.name = 'Kitchen phone';
      await tester.pumpWidget(const SizedBox());
      await start(tester);
      expect(store.ownProfile!.name, 'Kitchen phone');
      expect(inAgendaBar(find.text('KP')), findsOneWidget);
    });

    testWidgets('shows a person until the defaults are known', (tester) async {
      await tester.pumpWidget(MaterialApp(home: ProfileAvatar(profile: null)));
      expect(find.byIcon(Icons.person), findsOneWidget);
    });
  });

  group('the Profile page', () {
    testWidgets('shows picture, name and code, then Backups, Devices, '
        'People and Settings, in that order', (tester) async {
      tester.view.physicalSize = const Size(1080, 2600);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.reset);
      await start(tester);
      await openProfilePage(tester);
      expect(find.byType(ProfileScreen), findsOneWidget);
      expect(find.text('Profile'), findsOneWidget);
      final code = store.ownProfileCode!;
      expect(code, matches(RegExp(r'^[0-9A-Z]{4}-[0-9A-Z]{4}$')));
      final order = [
        find.byKey(const ValueKey('profilePicture')),
        find.byKey(const ValueKey('profileName')),
        find.text(code),
        find.text('Backups and recovery'),
        find.text('Your Devices'),
        find.text('People'),
        find.text('Settings of this Device'),
      ];
      final tops = [for (final f in order) tester.getTopLeft(f).dy];
      expect(tops, [...tops]..sort());
      expect(tops.toSet(), hasLength(tops.length));
      expect(find.text('Ada Pixel'), findsOneWidget);
    });

    testWidgets('leads to Backups, Sync and this Device\'s settings', (
      tester,
    ) async {
      await start(tester);
      for (final (key, screen) in [
        ('backups', BackupScreen),
        ('sync', SyncScreen),
        ('deviceSettings', SettingsScreen),
      ]) {
        await openProfilePage(tester);
        await tap(tester, find.byKey(ValueKey(key)));
        expect(find.byType(screen), findsOneWidget);
        await tester.binding.handlePopRoute();
        await tester.pumpAndSettle();
        await tester.binding.handlePopRoute();
        await tester.pumpAndSettle();
      }
      // Settings no longer hold Sync and Backups.
      await openDeviceSettings(tester);
      expect(find.byKey(const ValueKey('sync')), findsNothing);
      expect(find.byKey(const ValueKey('backups')), findsNothing);
    });

    testWidgets('keeps a typed name, sanitised, and the Agenda shows it', (
      tester,
    ) async {
      await start(tester);
      await openProfilePage(tester);
      await tester.enterText(
        find.byKey(const ValueKey('profileName')),
        '  \u202EGrace\u200B Hopper ',
      );
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();
      expect(store.ownProfile!.name, 'Grace Hopper');
      expect(store.ownProfile!.version, 2);
      expect(find.text('Grace Hopper'), findsOneWidget);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(inAgendaBar(find.text('GH')), findsOneWidget);

      // A changed phone name no longer replaces it.
      platform.name = 'Other';
      await tester.pumpWidget(const SizedBox());
      await start(tester);
      expect(store.ownProfile!.name, 'Grace Hopper');
    });

    testWidgets('keeps the name typed last when the page is left', (
      tester,
    ) async {
      await start(tester);
      await openProfilePage(tester);
      await tester.enterText(find.byKey(const ValueKey('profileName')), 'Ada');
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(store.ownProfile!.name, 'Ada');
    });

    testWidgets('shows "No name" for an empty name', (tester) async {
      await start(tester);
      await openProfilePage(tester);
      await tester.enterText(find.byKey(const ValueKey('profileName')), ' ');
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();
      expect(store.ownProfile!.name, '');
      expect(find.text('No name'), findsOneWidget);
    });

    testWidgets('fits a phone in German', (tester) async {
      tester.view.physicalSize = const Size(1080, 2340);
      tester.view.devicePixelRatio = 3;
      addTearDown(tester.view.reset);
      tester.platformDispatcher.localesTestValue = const [Locale('de')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await start(tester);
      expect(find.byTooltip('Profil'), findsOneWidget);
      await openProfilePage(tester);
      expect(find.text('Dein Code'), findsOneWidget);
      await tester.ensureVisible(find.text('Einstellungen dieses Geräts'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });
  });

  group('another of the own Devices', () {
    late Store desk;

    setUp(() {
      desk = Store.inMemory();
    });
    tearDown(() => desk.close());

    /// One LAN session with [desk] as the phone runs it.
    void sync() {
      final done = answerSync(
        desk,
        syncRequest(store, desk.versionVector(), name: 'Pixel'),
        name: 'Desk',
      )!;
      applyAnswer(store, done.answer);
    }

    testWidgets('changes the name and picture, and every view shows it at '
        'once', (tester) async {
      desk
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ada');
      await start(tester);
      sync();
      await tester.pumpAndSettle();
      expect(inAgendaBar(find.text('A')), findsOneWidget);
      await openProfilePage(tester);
      expect(find.text('Ada'), findsOneWidget);

      desk.renameOwnProfile('Ada Lovelace');
      desk.setOwnProfilePicture(squareProfilePicture(halvesPng));
      sync();
      await tester.pumpAndSettle();
      expect(find.text('Ada Lovelace'), findsOneWidget);
      expect(store.ownProfile!.picture, isNotNull);
      expect(find.byKey(const ValueKey('profilePlaceholder')), findsNothing);
    });

    testWidgets('with a Profile of its own leaves "Which one is you?" on the '
        'Profile page until it is answered', (tester) async {
      desk
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ada');
      await start(tester);
      store.renameOwnProfile('Ada L.');
      sync();
      await tester.pumpAndSettle();
      await openProfilePage(tester);
      await tap(tester, find.byKey(const ValueKey('profileQuestion')));
      expect(find.byType(ProfileQuestionDialog), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('whichLater')));
      expect(store.profileQuestion, isNotNull);
      expect(find.byKey(const ValueKey('profileQuestion')), findsOneWidget);

      await tap(tester, find.byKey(const ValueKey('profileQuestion')));
      await tap(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('whichMine')),
          matching: find.text('This is me'),
        ),
      );
      expect(store.profileQuestion, isNull);
      expect(find.byKey(const ValueKey('profileQuestion')), findsNothing);
      expect(store.ownProfile!.name, 'Ada L.');
      expect(store.ownProfileAliases, {desk.ownProfile!.id});
    });
  });

  group('the Profile picture', () {
    late OwnProfileModel model;
    late AlarmSync alarms;

    setUp(() {
      store.applyProfileDefaults(name: 'Ada');
      model = OwnProfileModel(store: store, platform: platform);
      alarms = AlarmSync(
        store: store,
        platform: FakeAlarms(),
        wallClock: (t) => t,
      );
    });
    tearDown(() {
      model.dispose();
      alarms.dispose();
    });

    Future<void> page(WidgetTester tester) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: ProfileScreen(
            store: store,
            alarms: alarms,
            model: model,
            work: inline,
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('is a square the user picks, kept as a 256 JPEG', (
      tester,
    ) async {
      platform.picked = halvesPng;
      await page(tester);
      await tap(tester, find.byKey(const ValueKey('profilePicture')));
      await tap(tester, find.text('Choose a picture'));
      expect(find.byType(ProfileCropScreen), findsOneWidget);
      expect(find.text('Choose the square'), findsOneWidget);
      // The square starts in the middle at full height; it goes to the
      // right edge, into the blue.
      await tester.drag(
        find.byKey(const ValueKey('cropArea')),
        const Offset(400, 0),
      );
      await tester.pumpAndSettle();
      await tap(tester, find.byKey(const ValueKey('cropUse')));
      expect(find.byType(ProfileCropScreen), findsNothing);
      final picture = store.ownProfile!.picture!;
      expect(picture.sublist(0, 2), [0xFF, 0xD8]);
      expect(pictureSize(picture), (width: 256, height: 256));
      expect(store.ownProfile!.version, 2);
      expect(find.byKey(const ValueKey('profilePlaceholder')), findsNothing);
    });

    testWidgets('can go back to the initials', (tester) async {
      store.setOwnProfilePicture(squareProfilePicture(halvesPng));
      await page(tester);
      await tap(tester, find.byKey(const ValueKey('profilePicture')));
      await tap(tester, find.text('Show initials instead'));
      expect(store.ownProfile!.picture, isNull);
      expect(find.text('A'), findsOneWidget);
    });

    testWidgets('nothing picked changes nothing', (tester) async {
      await page(tester);
      await tap(tester, find.byKey(const ValueKey('profilePicture')));
      expect(find.text('Show initials instead'), findsNothing);
      await tap(tester, find.text('Choose a picture'));
      expect(platform.picks, 1);
      expect(find.byType(ProfileCropScreen), findsNothing);
      expect(store.ownProfile!.version, 1);
    });

    testWidgets('a file that is no picture is refused', (tester) async {
      platform.picked = Uint8List.fromList([1, 2, 3, 4]);
      await page(tester);
      await tap(tester, find.byKey(const ValueKey('profilePicture')));
      await tap(tester, find.text('Choose a picture'));
      expect(find.byType(ProfileCropScreen), findsNothing);
      expect(store.ownProfile!.picture, isNull);
    });
  });
}
