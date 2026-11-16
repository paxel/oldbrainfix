import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/sync_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The address by hand (#267, ADR 0003): a field under each paired
/// Device of the user's own that hosts, on the Sync page, kept as typed
/// once it is an address, and saying why under the field when it is
/// none.
void main() {
  late Store store;
  final now = DateTime.utc(2026, 11, 13, 9);

  setUp(() {
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..askOn = true;
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  void remember(String id, String name, {bool shared = false, String? f}) =>
      store.rememberDevice(
        PairedDevice(
          id: id,
          name: name,
          fingerprint: f ?? 'ab' * 32,
          host: '192.168.1.5',
          port: 4000,
          lastSync: now,
          shared: shared,
        ),
      );

  Future<void> page(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1200, 3600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: SyncScreen(store: store),
      ),
    );
    await tester.pumpAndSettle();
  }

  Finder field(String id) =>
      find.descendant(of: key('address-$id'), matching: find.byType(TextField));

  String? errorOf(WidgetTester tester, String id) =>
      tester.widget<TextField>(field(id)).decoration?.errorText;

  testWidgets('is offered for one\'s own Devices that host only', (
    tester,
  ) async {
    remember('device:desk', 'Desk');
    remember('device:theirs', 'Theirs', shared: true);
    remember('device:joiner', 'Joiner', f: '');
    await page(tester);
    expect(field('device:desk'), findsOneWidget);
    expect(find.text('Address by hand'), findsOneWidget);
    expect(find.textContaining('such as Tailscale'), findsOneWidget);
    expect(field('device:theirs'), findsNothing);
    expect(field('device:joiner'), findsNothing);
  });

  testWidgets('shows only while Ask is switched on (#333), and an address '
      'kept meanwhile is still tried by sync', (tester) async {
    remember('device:desk', 'Desk');
    store
      ..setHandAddress('device:desk', 'desk.tail1234.ts.net')
      ..askOn = false;
    await page(tester);
    expect(find.text('Desk'), findsOneWidget, reason: 'still paired');
    expect(field('device:desk'), findsNothing);
    expect(find.text('Address by hand'), findsNothing);
    final desk = store.pairedDevices().single;
    expect(store.handAddress('device:desk')?.text, 'desk.tail1234.ts.net');
    expect(
      store.pairedAddresses(desk).map((a) => a.host),
      contains('desk.tail1234.ts.net'),
    );

    store.askOn = true;
    await tester.pumpWidget(const SizedBox.shrink());
    await page(tester);
    expect(field('device:desk'), findsOneWidget);
    expect(find.text('desk.tail1234.ts.net'), findsOneWidget);
  });

  testWidgets('keeps an address as typed, says why one is none, and an '
      'empty field removes it', (tester) async {
    remember('device:desk', 'Desk');
    store.setHandAddress('device:desk', 'old.example');
    await page(tester);
    expect(
      tester.widget<TextField>(field('device:desk')).controller?.text,
      'old.example',
    );

    await tester.enterText(field('device:desk'), 'Desk.Tail1234.ts.net:4711');
    await tester.pump();
    expect(
      store.handAddress('device:desk'),
      const HandAddress('desk.tail1234.ts.net', 4711),
    );
    expect(errorOf(tester, 'device:desk'), isNull);

    // A port that is none: the address before stays.
    await tester.enterText(field('device:desk'), 'desk:99999');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pump();
    expect(
      errorOf(tester, 'device:desk'),
      'The port must be a number from 1 to 65535.',
    );
    expect(store.handAddress('device:desk')?.text, 'desk.tail1234.ts.net:4711');

    await tester.enterText(field('device:desk'), 'not an address');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pump();
    expect(
      errorOf(tester, 'device:desk'),
      'Not an address. Type a name or an IP address, with :port if needed.',
    );

    // Typing an address again takes the error away.
    await tester.enterText(field('device:desk'), '[fd7a:115c::1]:4711');
    await tester.pump();
    expect(errorOf(tester, 'device:desk'), isNull);
    expect(store.handAddress('device:desk')?.text, '[fd7a:115c::1]:4711');

    await tester.enterText(field('device:desk'), '');
    await tester.pump();
    expect(store.handAddress('device:desk'), isNull);
    expect(store.pairedAddresses(store.pairedDevice('device:desk')!), [
      (host: '192.168.1.5', port: 4000),
    ]);
  });

  testWidgets('says why when the field is left', (tester) async {
    remember('device:desk', 'Desk');
    await page(tester);
    await tester.enterText(field('device:desk'), 'desk:0');
    await tester.pump();
    expect(errorOf(tester, 'device:desk'), isNull, reason: 'still typing');
    // Leaving the field, as a tap elsewhere does.
    FocusManager.instance.primaryFocus?.unfocus();
    await tester.pump();
    expect(
      errorOf(tester, 'device:desk'),
      'The port must be a number from 1 to 65535.',
    );
    expect(store.handAddress('device:desk'), isNull);
  });
}
