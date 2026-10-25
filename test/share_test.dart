import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';

/// Reads no QR; this Device goes by Pixel.
class FakePairing implements PairingPlatform {
  @override
  Future<String?> readQr(Uint8List photo) async => null;

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

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

void main() {
  late Store ann;
  late Store ben;
  late String area;
  late String milk;

  setUp(() {
    ann = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    ben = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    final house = Places(ann).addArea('Household');
    milk = Notes(ann).capture('Buy milk', place: house.id).id;
    Routines(ann).spawn(milk, Schedule(start: Day(2026, 3, 1)));
    area = ann.share(house.id, name: 'Ann');
  });
  tearDown(() {
    ann.close();
    ben.close();
  });

  /// Ann's computer invites Ben's phone, as a LAN session would.
  void invite(AreaRole role) {
    final done = answerSync(
      ann,
      wire(
        syncRequest(
          ben,
          ann.versionVector(),
          name: 'Phone',
          hostDevice: ann.deviceId,
          invited: true,
        ),
      ),
      name: 'Desk',
      inviting: Invite(area, role: role, name: 'Ben'),
    )!;
    applyAnswer(ben, wire(done.answer), shared: true);
  }

  Future<void> rowMenu(WidgetTester tester, String row) async {
    await tester.tap(
      find.descendant(
        of: find.ancestor(of: find.text(row), matching: find.byType(Row)).first,
        matching: find.byWidgetPredicate((w) => w is PopupMenuButton),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('a Shared Area shows who shares it; a Member only sees who '
      'is in', (tester) async {
    invite(AreaRole.member);
    await tester.pumpWidget(OldBrainFixApp(store: ben));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.search));
    await tester.pumpAndSettle();
    expect(find.text('Household'), findsOneWidget);
    expect(find.text('Ben, Ann'), findsOneWidget);
    expect(
      find.byWidgetPredicate((w) => w is Icon && w.semanticLabel == 'Shared'),
      findsOneWidget,
    );
    await rowMenu(tester, 'Household');
    expect(find.text('People'), findsOneWidget);
    expect(find.text('Rename'), findsNothing);
    expect(find.text('Share…'), findsNothing);
    expect(find.text('Delete'), findsNothing);
    await tester.tap(find.text('People'));
    await tester.pumpAndSettle();
    expect(find.text('Ben (you)'), findsOneWidget);
    expect(find.text('Ann'), findsOneWidget);
    expect(find.text('Manager'), findsOneWidget);
    expect(find.text('Member'), findsOneWidget);
    expect(find.byWidgetPredicate((w) => w is PopupMenuButton), findsNothing);
  });

  testWidgets('a Member may only add to someone else\'s Note', (tester) async {
    invite(AreaRole.member);
    await tester.pumpWidget(OldBrainFixApp(store: ben));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.search));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Household'));
    await tester.pumpAndSettle();
    expect(find.text('Buy milk'), findsOneWidget);
    expect(find.byTooltip('File into…'), findsNothing);
    await tester.tap(find.text('Buy milk'));
    await tester.pumpAndSettle();
    expect(find.byTooltip('Move to trash'), findsNothing);
    expect(find.byTooltip('Merge into…'), findsNothing);
    expect(find.byTooltip('File into…'), findsNothing);
    expect(
      find.text('Only a Manager of this Area can change this.'),
      findsOneWidget,
    );
    final field = tester.widget<TextField>(find.byType(TextField).first);
    expect(field.readOnly, isTrue);
    expect(find.text('Add to Note'), findsOneWidget);
  });

  testWidgets('a Manager sees everything as an owner does', (tester) async {
    invite(AreaRole.manager);
    await tester.pumpWidget(OldBrainFixApp(store: ben));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.search));
    await tester.pumpAndSettle();
    await rowMenu(tester, 'Household');
    for (final item in ['Rename', 'Share…', 'People', 'Delete']) {
      expect(find.text(item), findsOneWidget);
    }
  });

  testWidgets('a Manager changes roles and removes people', (tester) async {
    invite(AreaRole.member);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: PeopleScreen(store: ann, areaId: area),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Ann (you)'), findsOneWidget);
    final person = ann.people(area).last.id;
    await tester.tap(find.byKey(ValueKey('person-menu-$person')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Make Manager'));
    await tester.pumpAndSettle();
    expect(ann.people(area).last.role, AreaRole.manager);
    await tester.tap(find.byKey(ValueKey('person-menu-$person')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Remove'));
    await tester.pumpAndSettle();
    expect(find.text('Remove Ben from Household?'), findsOneWidget);
    await tester.tap(find.text('Remove').last);
    await tester.pumpAndSettle();
    expect(ann.people(area).map((p) => p.name), ['Ann']);
    expect(find.text('Ben'), findsNothing);
  });

  testWidgets('sharing joins the other computer inviting it to the Area', (
    tester,
  ) async {
    // Tall enough for the whole form.
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final announcements = Announcements();
    final house = Places(ben).addArea('Garden');
    final invites = <Invite?>[];
    // Ben's computer, as the core answers when invited.
    Future<SyncResult> join(
      Store store,
      PairInfo info, {
      required String name,
      Invite? invite,
    }) async {
      invites.add(invite);
      final request = wire(
        syncRequest(
          store,
          ann.versionVector(),
          name: name,
          hostDevice: ann.deviceId,
          invite: invite,
        ),
      );
      final done = answerSync(ann, request, name: 'Desk')!;
      final session = applyAnswer(
        store,
        wire(done.answer),
        shared: true,
        invitedBy: invite?.areaId,
      );
      return SyncResult(
        session: session,
        entriesSent: 0,
        blobsSent: 0,
        blobsReceived: 0,
      );
    }

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
            child: PairingScope(
              pairing: FakePairing(),
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        ),
        home: ShareAreaScreen(store: ben, areaId: house.id, join: join),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('invite')));
    await tester.pumpAndSettle();
    expect(announcements.visible, 'Fill in both names first.');
    expect(invites, isEmpty);
    announcements.dismiss();
    await tester.enterText(find.byKey(const ValueKey('own-name')), 'Ben');
    await tester.enterText(find.byKey(const ValueKey('their-name')), 'Ann');
    await tester.tap(find.byKey(const ValueKey('role-manager')));
    await tester.enterText(find.byKey(const ValueKey('share-code')), code);
    await tester.tap(find.byKey(const ValueKey('invite')));
    await tester.pumpAndSettle();
    final sent = invites.single!;
    expect(
      (sent.areaId, sent.role, sent.name),
      (house.id, AreaRole.manager, 'Ann'),
    );
    expect(ben.isShared(house.id), isTrue);
    expect(ann.accessIn(house.id), AreaAccess.manager);
    expect(ben.localSetting('shareName'), 'Ben');
    expect(announcements.visible, 'Garden is now shared with Ann.');
  });
}
