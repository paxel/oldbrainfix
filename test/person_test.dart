import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/notes_screen.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix/src/profile_screen.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_profile.dart';
import 'open_settings.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// Reads and makes pictures right here, as the tests run on one isolate.
final inline = PictureWork(
  size: (bytes) async => pictureSize(bytes),
  make: (bytes, left, top, size) async =>
      makeProfilePicture(bytes, left: left, top: top, size: size),
);

void main() {
  late Store ann;
  late Store ben;
  late String area;
  late String benId;
  late FakeProfilePlatform platform;

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
    benId = ben.ownProfile!.id;
    platform = FakeProfilePlatform(name: 'Desk', picked: halvesPng);
    area = ann.share(Places(ann).addArea('Household').id);
    // Ann's computer invites Ben's phone, as a LAN session would.
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
      inviting: Invite(area, role: AreaRole.member),
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
  });
  tearDown(() {
    ann.close();
    ben.close();
  });

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

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// [home] inside the app's localizations and a [PersonScope] of Ann's.
  Future<void> page(
    WidgetTester tester,
    Widget home, {
    Locale locale = const Locale('en'),
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: locale,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => PersonScope(
          store: ann,
          platform: platform,
          work: inline,
          child: child ?? const SizedBox.shrink(),
        ),
        home: home,
      ),
    );
    await tester.pumpAndSettle();
  }

  PersonScreen person() =>
      PersonScreen(store: ann, platform: platform, work: inline, id: benId);

  testWidgets('a picture on a card opens the person\'s Profile, and the '
      'name for me shows on the card', (tester) async {
    final semantics = tester.ensureSemantics();
    Notes(ben).capture('Ben buys bread', place: area);
    sync();
    await tester.pumpWidget(
      OldBrainFixApp(store: ann, alarms: FakeAlarms(), profile: platform),
    );
    await tester.pumpAndSettle();
    await tap(tester, find.byIcon(Icons.search));
    await tap(tester, find.text('Household'));
    await tap(tester, find.byKey(const ValueKey('author-open')));
    expect(find.byType(PersonScreen), findsOneWidget);
    expect(find.byType(NoteCard), findsNothing);
    expect(find.text('Ben'), findsWidgets);
    expect(find.text(ann.personCode(benId)!), findsOneWidget);
    expect(find.textContaining('Not verified'), findsOneWidget);
    await tester.enterText(find.byKey(const ValueKey('overrideName')), 'Mum');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pumpAndSettle();
    expect(ann.personShown(benId).name, 'Mum');
    // Their own name stays on their Profile.
    expect(find.byKey(const ValueKey('personName')), findsOneWidget);
    expect(
      tester.widget<Text>(find.byKey(const ValueKey('personName'))).data,
      'Ben',
    );
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.byType(NoteCard), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp('Written by Mum')), findsOneWidget);
    semantics.dispose();
  });

  testWidgets('codes compared there mark the Profile verified', (tester) async {
    await page(tester, person());
    await tap(tester, find.byKey(const ValueKey('compare-codes')));
    expect(find.textContaining(ann.personCode(benId)!), findsWidgets);
    await tap(tester, find.byKey(const ValueKey('codes-match')));
    expect(ann.personVerified(benId), isTrue);
    expect(find.textContaining('Verified'), findsOneWidget);
    expect(find.byKey(const ValueKey('compare-codes')), findsNothing);
  });

  testWidgets('codes that differ verify nobody', (tester) async {
    await page(tester, person());
    await tap(tester, find.byKey(const ValueKey('compare-codes')));
    await tap(tester, find.byKey(const ValueKey('codes-differ')));
    expect(ann.personVerified(benId), isFalse);
  });

  testWidgets('the picture for me is a square the user picks; their own '
      'comes back', (tester) async {
    await page(tester, person());
    await tap(tester, find.byKey(const ValueKey('overridePicture')));
    await tap(tester, find.text('Choose a picture'));
    expect(find.byType(ProfileCropScreen), findsOneWidget);
    await tap(tester, find.byKey(const ValueKey('cropUse')));
    expect(find.byType(PersonScreen), findsOneWidget);
    final picture = ann.personOverride(benId)!.picture!;
    expect(pictureSize(picture), (width: 256, height: 256));
    expect(ann.personShown(benId).picture, picture);
    // Their own picture, the placeholder, still leads their Profile.
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('personPicture')),
        matching: find.byKey(const ValueKey('profilePlaceholder')),
      ),
      findsOneWidget,
    );
    await tap(tester, find.byKey(const ValueKey('overridePicture')));
    await tap(tester, find.text('Use their own picture'));
    expect(ann.personOverride(benId), isNull);
  });

  testWidgets('the name for me is kept when the page is left', (tester) async {
    await page(tester, const SizedBox.shrink());
    final context = tester.element(find.byType(SizedBox));
    final opened = openPerson(context, benId);
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const ValueKey('overrideName')), 'Mum');
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    await opened;
    expect(ann.personShown(benId).name, 'Mum');
  });

  testWidgets('the people of a Shared Area open their Profile, the own '
      'line not', (tester) async {
    ann.setOverrideName(benId, 'Mum');
    await page(tester, PeopleScreen(store: ann, areaId: area));
    expect(find.text('Mum'), findsOneWidget);
    await tap(tester, find.text('Ann (you)'));
    expect(find.byType(PersonScreen), findsNothing);
    await tap(tester, find.text('Mum'));
    expect(find.byType(PersonScreen), findsOneWidget);
  });

  testWidgets('a Sent Item\'s sender opens their Profile once kept', (
    tester,
  ) async {
    final file = sendNote(ben, Notes(ben).capture('Dentist').id);
    final cid = Store.inMemory()..applyProfileDefaults(name: 'Cid');
    addTearDown(cid.close);
    // Cid never met Ben: nothing to open before accepting.
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => PersonScope(
          store: cid,
          platform: platform,
          child: child ?? const SizedBox.shrink(),
        ),
        home: SentItemScreen(store: cid, opened: openSentItem(cid, file)),
      ),
    );
    await tester.pumpAndSettle();
    await tap(tester, find.byKey(const ValueKey('sent-from-picture')));
    expect(find.byType(PersonScreen), findsNothing);
    // Ann keeps Ben's Profile: it opens, and the name for me set there
    // shows in the preview after.
    await tester.pumpWidget(const SizedBox.shrink());
    await page(
      tester,
      SentItemScreen(
        store: ann,
        opened: openSentItem(ann, file),
        reopen: () => openSentItem(ann, file),
      ),
    );
    await tap(tester, find.byKey(const ValueKey('sent-from-picture')));
    expect(find.byType(PersonScreen), findsOneWidget);
    await tester.enterText(find.byKey(const ValueKey('overrideName')), 'Mum');
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(find.text('From Mum'), findsOneWidget);
  });

  testWidgets('People on the Profile page lists every stored Profile, the '
      'name for me among them', (tester) async {
    tester.view.physicalSize = const Size(1080, 2600);
    tester.view.devicePixelRatio = 2;
    addTearDown(tester.view.reset);
    ann.setOverrideName(benId, 'Mum');
    await tester.pumpWidget(
      OldBrainFixApp(store: ann, alarms: FakeAlarms(), profile: platform),
    );
    await tester.pumpAndSettle();
    await openProfilePage(tester);
    expect(find.byType(ProfileScreen), findsOneWidget);
    await tap(tester, find.byKey(ValueKey('people-$benId')));
    expect(find.byType(PersonScreen), findsOneWidget);
    expect(find.text('Mum'), findsWidgets);
  });

  testWidgets('People says so while nobody is kept', (tester) async {
    final alone = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    addTearDown(alone.close);
    await tester.pumpWidget(
      OldBrainFixApp(store: alone, alarms: FakeAlarms(), profile: platform),
    );
    await tester.pumpAndSettle();
    await openProfilePage(tester);
    final empty = find.byKey(const ValueKey('peopleEmpty'));
    await tester.ensureVisible(empty);
    expect(empty, findsOneWidget);
  });

  group('pictures read on change, not on every build', () {
    /// The image the picture under [key] shows: the same as before when
    /// its bytes are the same, so it is not decoded again.
    ImageProvider imageUnder(WidgetTester tester, Key key) => tester
        .widget<Image>(
          find.descendant(of: find.byKey(key), matching: find.byType(Image)),
        )
        .image;

    /// [child] under a parent that builds it again when [rebuild] is
    /// called, with nothing changed.
    late void Function() rebuild;
    Widget again(Widget Function() child) => StatefulBuilder(
      builder: (context, setState) {
        rebuild = () => setState(() {});
        return child();
      },
    );

    Future<Uint8List> forMe() async {
      final size = pictureSize(halvesPng)!;
      return makeProfilePicture(
        halvesPng,
        left: 0,
        top: 0,
        size: size.height < size.width ? size.height : size.width,
      )!;
    }

    testWidgets('on a person\'s Profile', (tester) async {
      ann.setOverridePicture(benId, await forMe());
      await page(tester, again(person));
      const key = ValueKey('overridePicture');
      final before = imageUnder(tester, key);
      rebuild();
      await tester.pump();
      expect(imageUnder(tester, key), before);
      // A change shows at once.
      ann.setOverrideName(benId, 'Mum');
      await tester.pump();
      expect(find.text('Mum'), findsWidgets);
    });

    testWidgets('in the chips of the person filter', (tester) async {
      ann.setOverridePicture(benId, await forMe());
      await page(
        tester,
        again(
          () => Scaffold(
            body: PersonFilterChips(
              store: ann,
              filter: PersonFilter(people: {benId}),
              onChanged: (_) {},
            ),
          ),
        ),
      );
      final key = ValueKey('personChip-$benId');
      final before = imageUnder(tester, key);
      rebuild();
      await tester.pump();
      expect(imageUnder(tester, key), before);
      ann.setOverrideName(benId, 'Mum');
      await tester.pump();
      expect(find.text('Mum'), findsOneWidget);
    });

    testWidgets('in People', (tester) async {
      ann.setOverridePicture(benId, await forMe());
      await page(
        tester,
        again(() => Scaffold(body: PeopleSection(store: ann))),
      );
      final key = ValueKey('people-$benId');
      final before = imageUnder(tester, key);
      rebuild();
      await tester.pump();
      expect(imageUnder(tester, key), before);
      ann.setOverrideName(benId, 'Mum');
      await tester.pump();
      expect(find.text('Mum'), findsOneWidget);
    });
  });

  testWidgets('fits a phone in German', (tester) async {
    tester.view.physicalSize = const Size(720, 1280);
    tester.view.devicePixelRatio = 2;
    addTearDown(tester.view.reset);
    ann.setOverrideName(benId, 'Mama');
    await page(tester, person(), locale: const Locale('de'));
    expect(tester.takeException(), isNull);
    expect(find.text('Name und Bild für mich'), findsOneWidget);
    expect(find.text('Codes vergleichen'), findsOneWidget);
  });

  group('person filter', () {
    Finder chip(String id) => find.byKey(ValueKey('personChip-$id'));

    Future<void> pick(WidgetTester tester, String item) async {
      await tap(tester, find.byKey(const ValueKey('personFilter')).first);
      await tap(tester, find.byKey(ValueKey('personFilter-$item')));
    }

    testWidgets('"Their Notes" on a Profile opens All Notes with only that '
        'person\'s, until their chip is taken away', (tester) async {
      Notes(ann).capture('Ann buys milk', place: area);
      Notes(ben).capture('Ben buys bread', place: area);
      sync();
      await page(tester, person());
      await tap(tester, find.byKey(const ValueKey('theirNotes')));
      expect(find.byType(AllNotesScreen), findsOneWidget);
      expect(find.text('Ben buys bread'), findsOneWidget);
      // The screen names no Area, so the card does.
      expect(
        find.descendant(
          of: find.byType(NoteCard),
          matching: find.text('Household'),
        ),
        findsOneWidget,
      );
      expect(find.text('Ann buys milk'), findsNothing);
      expect(chip(benId), findsOneWidget);
      expect(find.text('Only Notes by'), findsOneWidget);
      await tap(
        tester,
        find.descendant(of: chip(benId), matching: find.byTooltip('Delete')),
      );
      expect(chip(benId), findsNothing);
      expect(find.text('Ben buys bread'), findsOneWidget);
      expect(find.text('Ann buys milk'), findsOneWidget);
    });

    testWidgets('in an Area "Me" and a person add up, and the filter '
        'combines with the search', (tester) async {
      // Tall enough for all three cards.
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      Notes(ann).capture('Ann buys milk', place: area);
      Notes(ann).capture('Ann bakes bread', place: area);
      Notes(ben).capture('Ben buys bread', place: area);
      sync();
      await tester.pumpWidget(
        OldBrainFixApp(store: ann, alarms: FakeAlarms(), profile: platform),
      );
      await tester.pumpAndSettle();
      await tap(tester, find.byIcon(Icons.search));
      await tap(tester, find.text('Household'));
      expect(find.byType(NoteCard), findsNWidgets(3));
      await pick(tester, 'me');
      expect(chip('me'), findsOneWidget);
      expect(find.text('Ben buys bread'), findsNothing);
      expect(find.byType(NoteCard), findsNWidgets(2));
      await pick(tester, benId);
      expect(find.byType(NoteCard), findsNWidgets(3));
      await pick(tester, 'me');
      expect(chip('me'), findsNothing);
      expect(find.byType(NoteCard), findsOneWidget);
      expect(find.text('Ben buys bread'), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'milk');
      await tester.pumpAndSettle();
      expect(find.text('No Note matches.'), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'bread');
      await tester.pumpAndSettle();
      expect(find.byType(NoteCard), findsOneWidget);
      expect(find.text('Ben buys bread'), findsOneWidget);
      await pick(tester, 'everyone');
      expect(chip(benId), findsNothing);
      expect(find.byType(NoteCard), findsNWidgets(2));
    });

    testWidgets('the search over every Note offers the filter while it '
        'searches', (tester) async {
      Notes(ann).capture('Ann buys bread', place: area);
      Notes(ben).capture('Ben buys bread', place: area);
      sync();
      await tester.pumpWidget(
        OldBrainFixApp(store: ann, alarms: FakeAlarms(), profile: platform),
      );
      await tester.pumpAndSettle();
      await tap(tester, find.byIcon(Icons.search));
      expect(find.byKey(const ValueKey('personFilter')), findsNothing);
      await tester.enterText(find.byType(TextField), 'bread');
      await tester.pumpAndSettle();
      expect(find.byType(NoteCard), findsNWidgets(2));
      await pick(tester, benId);
      expect(find.byType(NoteCard), findsOneWidget);
      expect(find.text('Ben buys bread'), findsOneWidget);
    });

    testWidgets('Unsorted keeps only the own Notes with "Me"', (tester) async {
      Notes(ann).capture('Call the bank');
      await page(
        tester,
        UnsortedScreen(notes: Notes(ann), places: Places(ann)),
      );
      expect(find.text('Call the bank'), findsOneWidget);
      await pick(tester, 'me');
      expect(find.text('Call the bank'), findsOneWidget);
      await pick(tester, benId);
      await pick(tester, 'me');
      expect(find.text('Call the bank'), findsNothing);
      expect(find.text('No Note matches.'), findsOneWidget);
    });

    testWidgets('fits a phone in German', (tester) async {
      tester.view.physicalSize = const Size(720, 1280);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.reset);
      Notes(ben).capture('Ben kauft Brot', place: area);
      sync();
      await page(tester, person(), locale: const Locale('de'));
      await tap(tester, find.byKey(const ValueKey('theirNotes')));
      expect(tester.takeException(), isNull);
      expect(find.text('Nur Notizen von'), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('personFilter')));
      expect(find.text('Mir'), findsOneWidget);
      expect(find.text('Allen'), findsOneWidget);
    });
  });

  group('blocking', () {
    testWidgets('Block on the Profile page asks once; Unblock ends it', (
      tester,
    ) async {
      await page(tester, person());
      await tap(tester, find.byKey(const ValueKey('block-person')));
      expect(find.text('Block Ben?'), findsOneWidget);
      expect(find.textContaining('A shared folder can still'), findsOneWidget);
      await tap(tester, find.text('Cancel'));
      expect(ann.isBlocked(benId), isFalse);

      await tap(tester, find.byKey(const ValueKey('block-person')));
      final confirm = find.byKey(const ValueKey('block-confirm'));
      expect(
        tester
            .widget<Text>(
              find.descendant(of: confirm, matching: find.byType(Text)),
            )
            .data,
        'Block',
      );
      await tap(tester, confirm);
      expect(ann.isBlocked(benId), isTrue);
      expect(find.byKey(const ValueKey('personBlocked')), findsOneWidget);
      expect(find.byKey(const ValueKey('block-person')), findsNothing);

      await tap(tester, find.byKey(const ValueKey('unblock-person')));
      expect(find.text('Unblock Ben?'), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('unblock-confirm')));
      expect(ann.isBlocked(benId), isFalse);
      expect(find.byKey(const ValueKey('personBlocked')), findsNothing);
      expect(find.byKey(const ValueKey('block-person')), findsOneWidget);
    });

    testWidgets('blocking a Manager names the Area it leaves', (tester) async {
      final annId = ann.ownProfile!.id;
      await page(
        tester,
        PersonScreen(store: ben, platform: platform, work: inline, id: annId),
      );
      await tap(tester, find.byKey(const ValueKey('block-person')));
      expect(find.textContaining('you leave Household'), findsOneWidget);
      expect(find.text('Block and leave'), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('block-confirm')));
      expect(ben.isBlocked(annId), isTrue);
      expect(ben.grantsOf(area)?.grantOf(ben.deviceId), isNull);
    });

    testWidgets('"Decline and block" leaves the Area of a Manager too', (
      tester,
    ) async {
      final annId = ann.ownProfile!.id;
      final file = sendNote(ann, Notes(ann).capture('Party').id);
      await page(tester, const SizedBox.shrink());
      final context = tester.element(find.byType(SizedBox));
      unawaited(
        Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (_) =>
                SentItemScreen(store: ben, opened: openSentItem(ben, file)),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tap(tester, find.byKey(const ValueKey('block-sent')));
      expect(find.textContaining('you leave Household'), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('block-confirm')));
      expect(find.byType(SentItemScreen), findsNothing);
      expect(ben.isBlocked(annId), isTrue);
      expect(ben.grantsOf(area)?.grantOf(ben.deviceId), isNull);
      expect(Notes(ben).all().where((n) => n.text == 'Party'), isEmpty);
    });

    testWidgets('People folds the blocked away, the fold kept per Device', (
      tester,
    ) async {
      ann.blockPerson(benId);
      Future<void> people() => page(
        tester,
        Scaffold(
          body: ListView(children: [PeopleSection(store: ann)]),
        ),
      );
      await people();
      expect(find.byKey(ValueKey('people-$benId')), findsNothing);
      expect(find.byKey(const ValueKey('peopleEmpty')), findsNothing);
      expect(find.text('Blocked'), findsOneWidget);
      expect(find.byKey(ValueKey('blocked-$benId')), findsNothing);
      await tap(tester, find.text('Blocked'));
      expect(find.byKey(ValueKey('blocked-$benId')), findsOneWidget);
      expect(find.text('Ben'), findsOneWidget);

      await tester.pumpWidget(const SizedBox.shrink());
      await people();
      expect(find.byKey(ValueKey('blocked-$benId')), findsOneWidget);
      await tap(tester, find.byKey(ValueKey('blocked-$benId')));
      expect(find.byType(PersonScreen), findsOneWidget);
      expect(find.text(ann.personCode(benId)!), findsOneWidget);
      expect(find.byKey(const ValueKey('unblock-person')), findsOneWidget);
      await tap(tester, find.byKey(const ValueKey('unblock-person')));
      await tap(tester, find.byKey(const ValueKey('unblock-confirm')));
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.byKey(ValueKey('people-$benId')), findsOneWidget);
      expect(find.text('Blocked'), findsNothing);
    });

    testWidgets('the block dialog fits a phone in German', (tester) async {
      tester.view.physicalSize = const Size(720, 1280);
      tester.view.devicePixelRatio = 2;
      addTearDown(tester.view.reset);
      await page(
        tester,
        PersonScreen(
          store: ben,
          platform: platform,
          work: inline,
          id: ann.ownProfile!.id,
        ),
        locale: const Locale('de'),
      );
      await tap(tester, find.byKey(const ValueKey('block-person')));
      expect(tester.takeException(), isNull);
      expect(find.text('Blockieren und verlassen'), findsOneWidget);
    });
  });
}
