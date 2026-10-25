import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/bundle_platform.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_photos.dart';

/// Bundle files in a directory of their own; what was shared is kept.
class FakeBundles implements BundlePlatform {
  final Directory dir;
  final shared = <String>[];
  var shares = true;

  /// What the picker answers; null backs out.
  String? picks;
  final waiting = <String>[];
  final _opened = StreamController<void>.broadcast();

  FakeBundles(this.dir);

  @override
  bool get available => true;

  @override
  Future<String?> exportPath(String name) async => '${dir.path}/$name';

  @override
  Future<bool> share(String path) async {
    shared.add(path);
    return shares;
  }

  @override
  Future<String?> pick() async => picks;

  @override
  Future<List<String>> takeOpened() async {
    final taken = [...waiting];
    waiting.clear();
    return taken;
  }

  @override
  Stream<void> get opened => _opened.stream;

  /// Another app opens [path] with this one.
  void open(String path) {
    waiting.add(path);
    _opened.add(null);
  }
}

void main() {
  final now = DateTime(2026, 10, 24, 9);
  late Store phone;
  late Store desk;
  late Directory dir;
  late FakeBundles bundles;
  late Announcements announcements;

  /// One LAN session: Pairing hands the keys over.
  void pair() => applyAnswer(
    phone,
    answerSync(
      desk,
      syncRequest(phone, desk.versionVector(), name: 'Pixel'),
      name: 'Desk',
    )!.answer,
  );

  /// A copy of [path], as the platform hands over what it imports.
  String copy(String path) {
    final to = '${dir.path}/in-${DateTime.now().microsecondsSinceEpoch}';
    File(path).copySync(to);
    return to;
  }

  setUp(() {
    phone = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    desk = Store.inMemory(clock: () => now);
    dir = Directory.systemTemp.createTempSync('bundle_widget_');
    bundles = FakeBundles(dir);
    announcements = Announcements();
  });
  tearDown(() {
    phone.close();
    desk.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  Future<void> page(WidgetTester tester) async {
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
            child: BundleScope(
              bundles: bundles,
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        ),
        home: SyncScreen(store: phone),
      ),
    );
    await tester.pumpAndSettle();
  }

  group('the Sync page', () {
    testWidgets('sends a bundle of everything, or nothing when nothing is '
        'new for a paired Device', (tester) async {
      pair();
      phone.rememberDevice(
        PairedDevice(
          id: desk.deviceId,
          name: 'Desk',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: now,
        ),
      );
      Notes(phone).capture('From the phone');
      await page(tester);
      expect(find.text('Sync bundle'), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('send-bundle')));
      await tester.pumpAndSettle();
      expect(bundles.shared.single, endsWith('oldbrainfix-2026-10-24.obfsync'));
      await importBundle(desk, bundles.shared.single);
      expect(Notes(desk).all().map((n) => n.text), ['From the phone']);

      // For the desk: what it holds since the session is not sent again.
      await tester.tap(find.byKey(const ValueKey('bundle-for')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Desk').last);
      await tester.pumpAndSettle();
      phone.noteHeldBy(desk.deviceId, phone.versionVector());
      await tester.tap(find.byKey(const ValueKey('send-bundle')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'Nothing new to send to Desk.');
      expect(bundles.shared, hasLength(1));

      // No app to take it.
      Notes(phone).capture('More');
      bundles.shares = false;
      announcements.dismiss();
      await tester.tap(find.byKey(const ValueKey('send-bundle')));
      await tester.pumpAndSettle();
      expect(announcements.visible, 'No app here can take the bundle.');
    });

    testWidgets('imports a picked bundle and says what came in', (
      tester,
    ) async {
      pair();
      Notes(desk).capture('From the desk');
      final file = '${dir.path}/desk.obfsync';
      await writeBundle(desk, file);
      await page(tester);

      // Backing out of the picker changes nothing.
      await tester.tap(find.byKey(const ValueKey('import-bundle')));
      await tester.pumpAndSettle();
      expect(announcements.visible, isNull);

      bundles.picks = copy(file);
      await tester.tap(find.byKey(const ValueKey('import-bundle')));
      await tester.pumpAndSettle();
      expect(announcements.visible, contains('came in with the bundle'));
      expect(Notes(phone).all().map((n) => n.text), ['From the desk']);
      expect(File(bundles.picks!).existsSync(), isFalse, reason: 'the copy');

      announcements.dismiss();
      final junk = '${dir.path}/junk';
      File(junk).writeAsStringSync('no zip');
      bundles.picks = junk;
      await tester.tap(find.byKey(const ValueKey('import-bundle')));
      await tester.pumpAndSettle();
      expect(
        announcements.visible,
        'This file is no sync bundle this app can read.',
      );
    });

    testWidgets('a bundle from a Device never paired says to pair first', (
      tester,
    ) async {
      Notes(desk).capture('Private');
      final file = '${dir.path}/desk.obfsync';
      await writeBundle(desk, file);
      await page(tester);
      bundles.picks = copy(file);
      await tester.tap(find.byKey(const ValueKey('import-bundle')));
      await tester.pumpAndSettle();
      expect(announcements.visible, contains('Pair them first'));
      expect(Notes(phone).all(), isEmpty);
    });
  });

  testWidgets('a bundle opened with the app is imported, at the start and '
      'while it runs', (tester) async {
    pair();
    Notes(desk).capture('Opened at the start');
    final first = '${dir.path}/first.obfsync';
    await writeBundle(desk, first);
    bundles.waiting.add(copy(first));
    await tester.pumpWidget(
      OldBrainFixApp(
        store: phone,
        alarms: FakeAlarms(),
        announcements: announcements,
        bundles: bundles,
      ),
    );
    await tester.pumpAndSettle();
    expect(announcements.visible, contains('came in with the bundle'));
    expect(Notes(phone).all().map((n) => n.text), ['Opened at the start']);

    announcements.dismiss();
    Notes(desk).capture('Opened later');
    final second = '${dir.path}/second.obfsync';
    await writeBundle(desk, second);
    bundles.open(copy(second));
    await tester.pumpAndSettle();
    expect(announcements.visible, contains('came in with the bundle'));
    expect(Notes(phone).all().map((n) => n.text).toSet(), {
      'Opened at the start',
      'Opened later',
    });
  });

  testWidgets('Conflicts a bundle brings behind the App Lock wait for the '
      'unlock', (tester) async {
    final note = Notes(phone).capture('Buy milk');
    pair();
    Notes(phone).edit(note.id, 'Buy oat milk');
    Notes(desk).edit(note.id, 'Buy milk and eggs');
    final file = '${dir.path}/desk.obfsync';
    await writeBundle(desk, file);
    phone
      ..setPin('2468', iterations: 10)
      ..appLockOn = true;
    bundles.waiting.add(copy(file));
    await tester.pumpWidget(
      OldBrainFixApp(
        store: phone,
        alarms: FakeAlarms(),
        announcements: announcements,
        bundles: bundles,
      ),
    );
    await tester.pumpAndSettle();
    expect(announcements.visible, contains('came in with the bundle'));
    announcements.dismiss();
    expect(announcements.visible, isNull, reason: 'not past the lock');
    expect(find.byType(ConflictsScreen), findsNothing);

    await tester.enterText(find.byKey(const ValueKey('lockPin')), '2468');
    await tester.tap(find.byKey(const ValueKey('unlockPin')));
    await tester.pumpAndSettle();
    expect(announcements.visible, contains('changed on both Devices'));
  });
}
