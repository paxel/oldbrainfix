import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/l10n.dart';
import 'package:oldbrainfix/src/person_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announced.dart';
import 'bundle_test.dart' show FakeBundles;
import 'fake_alarms.dart';
import 'fake_profile.dart';

/// The Changes Nudge on the phone (#109): planned while a person synced
/// by bundles lacks changes, gone once a bundle went to them; its tap and
/// "Send changes to …" count a bundle as sent only once an app took it.
void main() {
  late Store ann;
  late Store ben;
  late Directory dir;
  late FakeBundles files;
  late FakeAlarms alarms;
  late Announcements announcements;
  late String area;
  var n = 0;

  String idOf(Store store) => store.ownProfile!.id;

  setUp(() async {
    var now = DateTime.utc(2026, 10, 24, 9);
    DateTime clock() => now = now.add(const Duration(seconds: 1));
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
    ben.openProfileCard(ann.profileCard()!);
    ann.openProfileCard(ben.profileCard(back: true)!);
    dir = Directory.systemTemp.createTempSync('changes_nudge_widget_');
    files = FakeBundles(dir);
    alarms = FakeAlarms();
    announcements = Announcements();
    // Ann shares her Household with Ben by bundles; Ben takes it.
    final house = Places(ann).addArea('Household');
    area = shareAreaWith(
      ann,
      house.id,
      idOf(ben),
      role: AreaRole.member,
      transport: Transport.bundle,
    );
    final file = '${dir.path}/offer-${n++}.$bundleExtension';
    await writeAreaOffer(ann, file, area, idOf(ben));
    acceptAreaOffer(ben, (await importBundle(ben, file)).offer!.id);
  });

  tearDown(() {
    ann.close();
    ben.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  Future<void> app(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: ben,
        alarms: alarms,
        announcements: announcements,
        bundles: files,
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('a change for Ann plans "Changes for Ann" 30 minutes after '
      'it; nothing waiting plans none', (tester) async {
    await app(tester);
    expect(alarms.changes, isNull);
    Notes(ben).capture('From Ben', place: area);
    await tester.pumpAndSettle();
    final changes = alarms.changes!;
    expect(changes.title, 'Changes for Ann');
    expect(changes.body, 'Tap to send them.');
    expect(changes.public, 'Changes are waiting to be sent');
    expect(changes.nudge.people, [idOf(ann)]);
    expect(
      changes.nudge.at,
      ben.waitingForChanges()[idOf(ann)]!.add(const Duration(minutes: 30)),
    );
  });

  testWidgets('its tap marks nothing while the share sheet is left without '
      'an app, and ends it once an app took the bundle', (tester) async {
    await app(tester);
    Notes(ben).capture('From Ben', place: area);
    await tester.pumpAndSettle();
    expect(alarms.changes, isNotNull);

    files.dismisses = true;
    alarms.tapChanges();
    await tester.pumpAndSettle();
    expect(files.shared, hasLength(1));
    expect(ben.changesSentTo(idOf(ann)), {area: anything});
    expect(ben.waitingForChanges().keys, [idOf(ann)]);
    expect(alarms.changes, isNotNull);
    expect(announcements.visible, isNull);

    files.dismisses = false;
    final seq = ben.currentSeq();
    alarms.tapChanges();
    await tester.pumpAndSettle();
    expect(files.shared, hasLength(2));
    expect(ben.changesSentTo(idOf(ann))[area], seq);
    expect(ben.waitingForChanges(), isEmpty);
    expect(alarms.changes, isNull);
    await importBundle(ann, files.shared.last);
    expect(Notes(ann).all().map((n) => n.text), contains('From Ben'));
  });

  testWidgets('"Send changes to Ann" marks them sent only once an app took '
      'the bundle, and says so when no app can', (tester) async {
    var replans = 0;
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    Notes(ben).capture('From Ben', place: area);
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
          onChangesSent: () => replans++,
        ),
      ),
    );
    await tester.pumpAndSettle();

    files.shares = false;
    await tester.tap(find.text('Send changes to Ann'));
    await tester.pumpAndSettle();
    await expectFailureAnnounced(
      tester,
      announcements,
      'The bundle was not sent.',
      cause: 'No app here can take the bundle.',
      fix:
          'Install a messenger or a mail app, or choose a shared folder, '
          'then send it again.',
    );
    expect(ben.waitingForChanges().keys, [idOf(ann)]);

    files
      ..shares = true
      ..dismisses = true;
    await tester.tap(find.text('Send changes to Ann'));
    await tester.pumpAndSettle();
    expect(ben.waitingForChanges().keys, [idOf(ann)]);
    expect(replans, 0);

    files.dismisses = false;
    await tester.tap(find.text('Send changes to Ann'));
    await tester.pumpAndSettle();
    expect(ben.waitingForChanges(), isEmpty);
    expect(replans, 1);
    // Each bundle is named for its person, so one for another person
    // never overwrites it.
    expect(files.shared.last, endsWith('-Ann.$bundleExtension'));
  });

  testWidgets('"Send changes" shows only while changes travel by bundles', (
    tester,
  ) async {
    ben.setTransportWith(idOf(ann), Transport.folder);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: PersonScreen(
          store: ben,
          platform: FakeProfilePlatform(),
          id: idOf(ann),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('sendChanges')), findsNothing);
  });

  test('the names of the Changes Nudge read as a sentence', () {
    final en = lookupAppLocalizations(const Locale('en'));
    final de = lookupAppLocalizations(const Locale('de'));
    expect(namesText(en, ['Bob']), 'Bob');
    expect(namesText(en, ['Bob', 'Carol']), 'Bob and Carol');
    expect(namesText(en, ['Bob', 'Carol', 'Dave']), 'Bob, Carol and Dave');
    expect(
      de.changesNudgeTitle(namesText(de, ['Bob', 'Carol'])),
      'Änderungen für Bob und Carol',
    );
  });
}
