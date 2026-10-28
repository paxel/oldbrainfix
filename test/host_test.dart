import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/host_screen.dart';
import 'package:oldbrainfix/src/pairing_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';

class FakePairing implements PairingPlatform {
  @override
  Future<String?> readQr(Uint8List photo) async => null;

  @override
  Future<String> deviceName() async => 'Pixel';
}

/// A pairing host as the test says: its code, whether it has a network,
/// and whether it is locked.
class FakeHosting implements Hosting {
  final String pin;
  @override
  var offline = false;
  @override
  var lockedOut = false;
  var stopped = false;

  FakeHosting(this.pin);

  @override
  String pairCode({required bool typed}) => encodePairCode(
    '192.168.1.40',
    45231,
    pin,
    List.filled(32, 7),
    typed: typed,
  );

  @override
  Future<void> stop() async => stopped = true;
}

/// Starts [FakeHosting]s and keeps what the page handed over: the name,
/// the invite, the question to ask and where sessions go.
class FakeHosts {
  final started = <FakeHosting>[];
  var offline = false;
  var fails = false;
  String? name;
  Invite? inviting;
  late Future<bool> Function(JoinAsk ask, ProfileChoice? profile) ask;
  late void Function(SyncSession session) onSession;

  FakeHosting get last => started.last;

  Future<Hosting> start(
    Store store, {
    required String name,
    required Future<bool> Function(JoinAsk ask, ProfileChoice? profile) ask,
    Invite? inviting,
    required void Function(SyncSession session) onSession,
  }) async {
    if (fails) throw StateError('no host');
    this.name = name;
    this.inviting = inviting;
    this.ask = ask;
    this.onSession = onSession;
    final hosting = FakeHosting('04281${started.length}')..offline = offline;
    started.add(hosting);
    return hosting;
  }
}

SyncSession sessionWith(String name) => SyncSession(
  peer: 'tablet',
  name: name,
  applied: const [],
  moment: null,
  report: ImportReport(),
);

void main() {
  late Store phone;
  late FakeHosts hosts;
  late Announcements announcements;

  setUp(() {
    phone = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Pixel')
      ..renameOwnProfile('Ben');
    hosts = FakeHosts();
    announcements = Announcements();
  });
  tearDown(() => phone.close());

  Future<void> open(WidgetTester tester, Widget home) async {
    // Tall enough for the whole page.
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
            child: PairingScope(
              pairing: FakePairing(),
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        ),
        home: home,
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Opens this Device's code from Sync.
  Future<void> showCode(WidgetTester tester) async {
    await open(tester, SyncScreen(store: phone, host: hosts.start));
    await tester.tap(find.byKey(const ValueKey('show-code')));
    await tester.pumpAndSettle();
  }

  testWidgets('Sync shows this Device\'s code as a QR with the typed code '
      'under it, and stops hosting when left', (tester) async {
    await showCode(tester);
    expect(find.byType(HostScreen), findsOneWidget);
    expect(hosts.name, 'Pixel');
    expect(hosts.inviting, isNull);
    expect(find.byKey(const ValueKey('host-qr')), findsOneWidget);
    final typed = pairCodeLines(hosts.last.pairCode(typed: true)).join('\n');
    expect(find.text(typed), findsOneWidget);
    expect(decodePairCode(typed)?.pin, '042810');
    expect(hosts.last.stopped, isFalse);

    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(HostScreen), findsNothing);
    expect(hosts.last.stopped, isTrue);
  });

  testWidgets('a Device asking to join gets in only when the user says yes', (
    tester,
  ) async {
    await showCode(tester);
    const tablet = JoinAsk(device: 'tablet', name: 'Tablet');

    final declined = hosts.ask(tablet, null);
    await tester.pumpAndSettle();
    expect(find.text('Let Tablet join?'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('host-decline')));
    await tester.pumpAndSettle();
    expect(await declined, isFalse);

    final allowed = hosts.ask(tablet, null);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('host-allow')));
    await tester.pumpAndSettle();
    expect(await allowed, isTrue);
    expect(find.byType(JoinAskDialog), findsNothing);

    hosts.onSession(sessionWith('Tablet'));
    await tester.pumpAndSettle();
    expect(announcements.visible, 'Synced with Tablet: nothing new came in.');
    expect(hosts.last.stopped, isFalse, reason: 'stays for the photos');
  });

  testWidgets('without a network it says so and tries again', (tester) async {
    hosts.offline = true;
    await showCode(tester);
    expect(find.byKey(const ValueKey('host-offline')), findsOneWidget);
    expect(find.byKey(const ValueKey('host-qr')), findsNothing);

    hosts.offline = false;
    await tester.tap(find.byKey(const ValueKey('host-again')));
    await tester.pumpAndSettle();
    expect(hosts.started.first.stopped, isTrue);
    expect(find.byKey(const ValueKey('host-qr')), findsOneWidget);
  });

  testWidgets('locked by wrong PINs it says so and makes a new code', (
    tester,
  ) async {
    await showCode(tester);
    final first = hosts.last;
    first.lockedOut = true;
    await tester.pump(const Duration(seconds: 1));
    expect(find.byKey(const ValueKey('host-locked')), findsOneWidget);
    expect(find.byKey(const ValueKey('host-qr')), findsNothing);

    await tester.tap(find.byKey(const ValueKey('host-again')));
    await tester.pumpAndSettle();
    expect(first.stopped, isTrue);
    expect(hosts.started, hasLength(2));
    expect(find.byKey(const ValueKey('host-qr')), findsOneWidget);
    final typed = pairCodeLines(hosts.last.pairCode(typed: true)).join('\n');
    expect(decodePairCode(typed)?.pin, '042811', reason: 'a new PIN');
  });

  testWidgets('a host that does not start says so, with no spinner left', (
    tester,
  ) async {
    hosts.fails = true;
    await showCode(tester);
    expect(find.byKey(const ValueKey('host-failed')), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    hosts.fails = false;
    await tester.tap(find.byKey(const ValueKey('host-again')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('host-qr')), findsOneWidget);
  });

  testWidgets('a phone shares an Area by showing a code: the joiner\'s '
      'Profile is confirmed first', (tester) async {
    final ann = Store.inMemory()
      ..applyProfileDefaults(name: 'Tablet')
      ..renameOwnProfile('Ann');
    addTearDown(ann.close);
    final garden = Places(phone).addArea('Garden');
    final shared = <String>[];
    var popped = false;
    await open(
      tester,
      Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            onPressed: () async {
              final done = await Navigator.of(context).push<bool>(
                MaterialPageRoute(
                  builder: (_) => ShareAreaScreen(
                    store: phone,
                    areaId: garden.id,
                    host: hosts.start,
                    onShared: shared.add,
                  ),
                ),
              );
              popped = done ?? false;
            },
            child: const Text('share'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('share'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('role-manager')));
    await tester.tap(find.byKey(const ValueKey('share-show')));
    await tester.pumpAndSettle();
    final invite = hosts.inviting!;
    expect((invite.areaId, invite.role), (shared.single, AreaRole.manager));
    expect(phone.isShared(invite.areaId), isTrue);
    expect(find.text('Share Garden'), findsOneWidget);

    const asking = JoinAsk(device: 'tablet', name: 'Tablet');
    // Without a Profile the core refuses it anyway: nobody is asked.
    expect(await hosts.ask(asking, null), isFalse);
    final profile = phone.sentProfileOf(
      profileAnswer(ann)['profile'],
      device: ann.deviceId,
    )!;
    final confirmed = hosts.ask(asking, profile);
    await tester.pumpAndSettle();
    expect(find.text('Share Garden with this person?'), findsOneWidget);
    expect(find.text('Ann'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('inviteConfirm')));
    await tester.pumpAndSettle();
    expect(await confirmed, isTrue);

    hosts.onSession(sessionWith('Tablet'));
    await tester.pumpAndSettle();
    expect(announcements.visible, 'Garden is now shared with Ann.');

    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(hosts.last.stopped, isTrue);
    expect(find.byType(ShareAreaScreen), findsNothing);
    expect(popped, isTrue);
  });
}
