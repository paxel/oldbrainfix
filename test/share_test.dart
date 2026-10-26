import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'reveal.dart';

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
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    ben = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Pixel')
      ..renameOwnProfile('Ben');
    final house = Places(ann).addArea('Household');
    milk = Notes(ann).capture('Buy milk', place: house.id).id;
    Routines(ann).spawn(milk, Schedule(start: Day(2026, 3, 1)));
    area = ann.share(house.id);
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
      inviting: Invite(area, role: role),
    )!;
    applyAnswer(ben, wire(done.answer), shared: true);
    for (final (store, peer) in [(ann, ben), (ben, ann)]) {
      store.rememberDevice(
        PairedDevice(
          id: peer.deviceId,
          name: '',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: DateTime.utc(2026, 3, 1),
          shared: true,
        ),
      );
    }
  }

  /// Ben's phone and Ann's computer sync once more, as paired.
  void sync() {
    final done = answerSync(
      ann,
      wire(
        syncRequest(
          ben,
          ann.versionVector(),
          name: 'Phone',
          hostDevice: ann.deviceId,
        ),
      ),
      name: 'Desk',
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
    await reveal(tester, find.text('Add to Note'));
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
    expect(ann.people(area).map((p) => ann.personShown(p.id).name), ['Ann']);
    expect(find.text('Ben'), findsNothing);
  });

  testWidgets('other people\'s Notes show their picture; one\'s own none', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    invite(AreaRole.member);
    Notes(ben).capture('Ben buys bread', place: area);
    sync();
    for (final (store, mine, theirs, who) in [
      (ben, 'Ben buys bread', 'Buy milk', 'Ann'),
      (ann, 'Buy milk', 'Ben buys bread', 'Ben'),
    ]) {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.search));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Household'));
      await tester.pumpAndSettle();
      final cards = find.byType(NoteCard);
      expect(cards, findsNWidgets(2));
      Finder cardOf(String text) =>
          find.ancestor(of: find.text(text), matching: cards);
      expect(
        find.descendant(
          of: cardOf(theirs),
          matching: find.byKey(const ValueKey('author')),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: cardOf(mine),
          matching: find.byKey(const ValueKey('author')),
        ),
        findsNothing,
      );
      expect(find.bySemanticsLabel(RegExp('Written by $who')), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
    }
    semantics.dispose();
  });

  testWidgets('sharing shows the other person\'s Profile and invites only '
      'once it is confirmed', (tester) async {
    // Tall enough for the whole form.
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final announcements = Announcements();
    final house = Places(ben).addArea('Garden');
    final invites = <Invite?>[];
    // Ann's computer, as the core answers when invited: its Profile
    // first, then the session once the user confirmed it.
    Future<SyncResult> join(
      Store store,
      PairInfo info, {
      required String name,
      Invite? invite,
      Future<bool> Function(ProfileChoice other)? confirm,
    }) async {
      final hostProfile = profileAnswer(ann)['profile'];
      final choice = store.sentProfileOf(hostProfile, device: ann.deviceId)!;
      if (confirm != null && !await confirm(choice)) {
        throw const SyncException(SyncFailure.cancelled);
      }
      invites.add(invite);
      final request = wire(
        syncRequest(
          store,
          ann.versionVector(),
          name: name,
          hostDevice: ann.deviceId,
          invite: invite,
          hostProfile: hostProfile,
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
    // Nobody types a name.
    expect(find.text('Your name'), findsNothing);
    expect(find.text('Their name'), findsNothing);
    await tester.tap(find.byKey(const ValueKey('role-manager')));
    await tester.enterText(find.byKey(const ValueKey('share-code')), code);
    await tester.tap(find.byKey(const ValueKey('invite')));
    // The progress turns behind the question: no settling meanwhile.
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    // Ann's Profile shows to confirm; put off, nothing is shared.
    expect(find.text('Share Garden with this person?'), findsOneWidget);
    expect(find.text('Ann'), findsOneWidget);
    expect(find.text(ann.ownProfileCode!), findsOneWidget);
    expect(find.byKey(const ValueKey('inviteAvatar')), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(invites, isEmpty);
    expect(ann.accessIn(house.id), AreaAccess.owner);
    expect(announcements.visible, 'Nothing was shared.');
    announcements.dismiss();
    await tester.tap(find.byKey(const ValueKey('invite')));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await tester.tap(find.byKey(const ValueKey('inviteConfirm')));
    await tester.pumpAndSettle();
    final sent = invites.single!;
    expect((sent.areaId, sent.role), (house.id, AreaRole.manager));
    expect(ben.isShared(house.id), isTrue);
    expect(ann.accessIn(house.id), AreaAccess.manager);
    expect(ben.people(house.id).map((p) => ben.personShown(p.id).name), [
      'Ben',
      'Ann',
    ]);
    expect(announcements.visible, 'Garden is now shared with Ann.');
  });
}
