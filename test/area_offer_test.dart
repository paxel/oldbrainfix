import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/area_offer_screen.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/folder_platform.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_alarms.dart';
import 'fake_photos.dart';
import 'fake_profile.dart';
import 'folder_test.dart' show FakeFolders;

/// Sharing an Area with a person from afar (#108): the share page asks
/// whom, as what and how changes travel, and hands the offer to the app
/// the user picks; the other side previews it and accepts or declines.
void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store ben;
  late Directory dir;
  late FakeBundles files;
  late FakeFolders folders;
  late Announcements announcements;
  var n = 0;

  setUp(() {
    var now = DateTime.utc(2026, 10, 24, 9);
    clock = () => now = now.add(const Duration(seconds: 1));
    ann = Store.inMemory(clock: clock)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Ann')
      ..renameOwnProfile('Ann');
    ben = Store.inMemory(clock: clock)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Ben')
      ..renameOwnProfile('Ben');
    // Each sent the other their Profile card through a messenger.
    ben.openProfileCard(ann.profileCard()!);
    ann.openProfileCard(ben.profileCard(back: true)!);
    dir = Directory.systemTemp.createTempSync('area_offer_widget_');
    files = FakeBundles(dir);
    folders = FakeFolders()..picks = 'content://tree/primary%3ASync';
    announcements = Announcements();
  });
  tearDown(() {
    ann.close();
    ben.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  String idOf(Store store) => store.ownProfile!.id;

  /// A copy of [path], as the platform hands over what it imports.
  String copy(String path) {
    final to = '${dir.path}/in-${n++}';
    File(path).copySync(to);
    return to;
  }

  /// Ann's Household with a Note, shared with Ben as [role]; changes
  /// travel as [transport]. Returns the Area and the offer file.
  Future<({String area, String file})> offered({
    AreaRole role = AreaRole.member,
    Transport transport = Transport.bundle,
  }) async {
    final house = Places(ann).addArea('Household');
    Notes(ann).capture('Buy milk', place: house.id);
    final area = shareAreaWith(
      ann,
      house.id,
      idOf(ben),
      role: role,
      transport: transport,
    );
    final file = '${dir.path}/offer-${n++}.$bundleExtension';
    await writeAreaOffer(ann, file, area, idOf(ben));
    return (area: area, file: file);
  }

  /// The share page of [area] on [store], as the Areas list opens it.
  Future<void> sharePage(WidgetTester tester, Store store, String area) async {
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
            photos: FakePhotos(),
            bytes: (_) => null,
            child: FolderScope(
              folders: folders,
              child: BundleScope(
                bundles: files,
                child: child ?? const SizedBox.shrink(),
              ),
            ),
          ),
        ),
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () => Navigator.of(context).push(
                MaterialPageRoute<bool>(
                  builder: (_) => ShareAreaScreen(store: store, areaId: area),
                ),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  /// The phone's app on Ben's store, with [waiting] opened with it.
  Future<void> app(
    WidgetTester tester, [
    List<String> waiting = const [],
  ]) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    files.waiting.addAll(waiting);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: ben,
        alarms: FakeAlarms(),
        announcements: announcements,
        bundles: files,
        folders: folders,
      ),
    );
    await tester.pumpAndSettle();
  }

  group('sharing', () {
    testWidgets('by bundles: Ben is picked, unverified, and gets the offer; '
        'the next Area does not ask again', (tester) async {
      final house = Places(ann).addArea('Household');
      Notes(ann).capture('Buy milk', place: house.id);
      await sharePage(tester, ann, house.id);
      expect(find.text('Someone not nearby'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('share-afar')));
      await tester.pumpAndSettle();
      expect(find.text('Share Household with…'), findsOneWidget);
      expect(find.text('Not verified'), findsOneWidget);
      await tester.tap(find.byKey(ValueKey('share-to-${idOf(ben)}')));
      await tester.pumpAndSettle();
      expect(find.text('How do changes travel with Ben?'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('transport-bundle')));
      await tester.tap(find.byKey(const ValueKey('transport-share')));
      await tester.pumpAndSettle();
      expect(find.byType(ShareAreaScreen), findsNothing);
      expect(ann.transportWith(idOf(ben)), Transport.bundle);
      expect(folders.picked, 0);
      final held = (await importBundle(ben, files.shared.single)).offer!;
      expect(held.areaName, 'Household');
      expect(held.role, AreaRole.member);
      expect(held.transport, Transport.bundle);

      // Asked once per person: the next Area goes the same way.
      final garden = Places(ann).addArea('Garden');
      await sharePage(tester, ann, garden.id);
      await tester.tap(find.byKey(const ValueKey('role-manager')));
      await tester.tap(find.byKey(const ValueKey('share-afar')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey('share-to-${idOf(ben)}')));
      await tester.pumpAndSettle();
      expect(find.byType(TransportDialog), findsNothing);
      expect(files.shared, hasLength(2));
      final second = (await importBundle(ben, files.shared.last)).offer!;
      expect(second.areaName, 'Garden');
      expect(second.role, AreaRole.manager);
    });

    testWidgets('by a folder: the folder is picked before anything is '
        'written, and backing out shares nothing', (tester) async {
      final house = Places(ann).addArea('Household');
      await sharePage(tester, ann, house.id);
      folders.picks = null;
      await tester.tap(find.byKey(const ValueKey('share-afar')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey('share-to-${idOf(ben)}')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('transport-folder')));
      await tester.tap(find.byKey(const ValueKey('transport-share')));
      await tester.pumpAndSettle();
      expect(folders.picked, 1);
      expect(files.shared, isEmpty);
      expect(ann.transportWith(idOf(ben)), isNull);
      expect(ann.isShared(house.id), isFalse);

      folders.picks = 'content://tree/primary%3ASync';
      await tester.tap(find.byKey(const ValueKey('share-afar')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey('share-to-${idOf(ben)}')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('transport-folder')));
      await tester.tap(find.byKey(const ValueKey('transport-share')));
      await tester.pumpAndSettle();
      expect(ann.syncFolder, 'content://tree/primary%3ASync');
      expect(ann.transportWith(idOf(ben)), Transport.folder);
      final held = (await importBundle(ben, files.shared.single)).offer!;
      expect(held.transport, Transport.folder);
    });

    testWidgets('nobody to share with: a blocked person is no choice', (
      tester,
    ) async {
      final house = Places(ann).addArea('Household');
      ann.blockPerson(idOf(ben));
      await sharePage(tester, ann, house.id);
      expect(find.byKey(const ValueKey('share-afar')), findsNothing);
      expect(
        find.textContaining('Add the person from their Profile card'),
        findsOneWidget,
      );
    });

    testWidgets('no app to take the file is said', (tester) async {
      final house = Places(ann).addArea('Household');
      files.shares = false;
      await sharePage(tester, ann, house.id);
      await tester.tap(find.byKey(const ValueKey('share-afar')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey('share-to-${idOf(ben)}')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('transport-share')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'No app here can take the bundle.');
      expect(find.byType(ShareAreaScreen), findsOneWidget);
    });
  });

  group('receiving', () {
    testWidgets('the preview names who shares what as what; codes compared '
        'verify Ann on Accept', (tester) async {
      final o = await offered();
      await app(tester, [copy(o.file)]);
      expect(find.byType(AreaOfferScreen), findsOneWidget);
      expect(
        find.text('Ann shares Household with you as Member'),
        findsOneWidget,
      );
      expect(find.textContaining('Not verified: known here'), findsOneWidget);
      expect(Notes(ben).all(), isEmpty, reason: 'nothing before Accept');
      await tester.tap(find.byKey(const ValueKey('compare-codes')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('codes-match')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('accept-offer')));
      await tester.pumpAndSettle();
      expect(find.byType(AreaOfferScreen), findsNothing);
      expect(announcements.visible, 'Shared Area accepted.');
      expect(Notes(ben).all().map((n) => n.text), ['Buy milk']);
      expect(ben.accessIn(o.area), AreaAccess.member);
      expect(ben.personVerified(idOf(ann)), isTrue);
      expect(heldAreaOffers(ben), isEmpty);
    });

    testWidgets('accepting an Area that travels by a folder asks for the '
        'folder when none is chosen', (tester) async {
      final o = await offered(transport: Transport.folder);
      await app(tester, [copy(o.file)]);
      expect(find.textContaining('through a shared folder'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('accept-offer')));
      await tester.pumpAndSettle();
      expect(find.text('Choose the shared folder?'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('offer-folder-pick')));
      await tester.pumpAndSettle();
      expect(ben.syncFolder, 'content://tree/primary%3ASync');
      expect(ben.transportWith(idOf(ann)), Transport.folder);
      expect(find.byType(AreaOfferScreen), findsNothing);
    });

    testWidgets('Decline keeps nothing', (tester) async {
      final o = await offered();
      final before = ben.allEntries().length;
      await app(tester, [copy(o.file)]);
      await tester.tap(find.byKey(const ValueKey('decline-offer')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'Declined: nothing was kept.');
      expect(ben.allEntries(), hasLength(before));
      expect(ben.pendingSets, isEmpty);
      expect(ben.transportWith(idOf(ann)), isNull);
      expect(ben.writingKey(o.area), isNull);
    });

    testWidgets('"Decline and block" keeps nothing and blocks Ann', (
      tester,
    ) async {
      final o = await offered();
      await app(tester, [copy(o.file)]);
      await tester.tap(find.byKey(const ValueKey('block-offer')));
      await tester.pumpAndSettle();
      expect(find.text('Block Ann?'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('block-confirm')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'Declined and blocked: nothing was kept.');
      expect(ben.isBlocked(idOf(ann)), isTrue);
      expect(ben.pendingSets, isEmpty);
      expect(Notes(ben).all(), isEmpty);
    });

    testWidgets('an offer left undecided waits on the Agenda after a '
        'restart', (tester) async {
      final o = await offered();
      await importBundle(ben, o.file);
      await app(tester);
      expect(find.text('1 shared Area waiting'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('areaOffersWaiting')));
      await tester.pumpAndSettle();
      expect(find.text('Shared Areas waiting'), findsOneWidget);
      await tester.tap(find.text('Ann shares Household with you as Member'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('accept-offer')));
      await tester.pumpAndSettle();
      expect(find.text('No shared Areas are waiting.'), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('areaOffersWaiting')), findsNothing);
    });

    testWidgets('behind the App Lock it only says that it waits', (
      tester,
    ) async {
      final o = await offered();
      ben
        ..setPin('2468', iterations: 10)
        ..appLockOn = true;
      await app(tester, [copy(o.file)]);
      expect(announcements.visible, 'A shared Area is waiting on the Agenda.');
      expect(find.byType(AreaOfferScreen), findsNothing);
      expect(heldAreaOffers(ben), hasLength(1));
      await tester.enterText(find.byKey(const ValueKey('lockPin')), '2468');
      await tester.tap(find.byKey(const ValueKey('unlockPin')));
      await tester.pumpAndSettle();
      expect(find.text('1 shared Area waiting'), findsOneWidget);
    });

    group('refused', () {
      testWidgets('from a blocked person', (tester) async {
        final o = await offered();
        ben.blockPerson(idOf(ann));
        await app(tester, [copy(o.file)]);
        expect(
          announcements.visible,
          'Refused: this shared Area comes from a person you blocked.',
        );
        expect(ben.pendingSets, isEmpty);
      });

      testWidgets('of an Area that is the own already', (tester) async {
        final o = await offered();
        ben.append(o.area, PlaceKeys.type, PlaceKeys.typeArea);
        await app(tester, [copy(o.file)]);
        expect(announcements.visible, contains('your own on this Device'));
        expect(ben.pendingSets, isEmpty);
      });

      testWidgets('meant for somebody else', (tester) async {
        final o = await offered();
        final cat = Store.inMemory(clock: clock)
          ..starterSetOffered = true
          ..alarmSetupDone = true
          ..applyProfileDefaults(name: 'Cat');
        ben.close();
        ben = cat;
        await app(tester, [copy(o.file)]);
        expect(announcements.visible, contains('meant for another person'));
        expect(cat.pendingSets, isEmpty);
      });

      testWidgets('damaged', (tester) async {
        final o = await offered();
        // The offer inside, its key renamed at the same length.
        final bytes = File(o.file).readAsBytesSync();
        final at = latin1.decode(bytes).indexOf('{"sealed":"');
        expect(at, isNonNegative);
        final broken = [...bytes]..[at + 7] = 0x58;
        final file = '${dir.path}/broken';
        File(file).writeAsBytesSync(broken);
        await app(tester, [file]);
        expect(
          announcements.visible,
          'This file holds no shared Area this app can read. Ask the sender '
          'to share it again.',
        );
        expect(ben.pendingSets, isEmpty);
      });
    });
  });

  testWidgets('"Send changes to Ann" sends what is new in the Areas shared '
      'with her', (tester) async {
    final o = await offered();
    acceptAreaOffer(ben, (await importBundle(ben, o.file)).offer!.id);
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
          child: BundleScope(
            bundles: files,
            child: child ?? const SizedBox.shrink(),
          ),
        ),
        home: PersonScreen(
          store: ben,
          platform: FakeProfilePlatform(),
          id: idOf(ann),
        ),
      ),
    );
    await tester.pumpAndSettle();
    Notes(ben).capture('From Ben', place: o.area);
    await tester.tap(find.text('Send changes to Ann'));
    await tester.pumpAndSettle();
    final sent = files.shared.single;
    await importBundle(ann, sent);
    expect(Notes(ann).all().map((n) => n.text).toSet(), {
      'Buy milk',
      'From Ben',
    });

    // Nothing new once she holds it all.
    ben.noteHeldBy(ann.deviceId, ann.versionVector());
    await tester.tap(find.text('Send changes to Ann'));
    await tester.pumpAndSettle();
    expect(announcements.visible, 'Nothing new to send to Ann.');
    expect(files.shared, hasLength(1));
  });
}
