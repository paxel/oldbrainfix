import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/home_shortcuts.dart';
import 'package:oldbrainfix/src/lock_screen.dart';
import 'package:oldbrainfix/src/notes_screen.dart';
import 'package:oldbrainfix/src/tally_entry_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_home_shortcuts.dart';
import 'fake_lock.dart';
import 'reveal.dart';

/// A Tally's shortcut on the home screen (#284): "Add to home screen" in
/// its card's ⋮ pins it, a tap on it opens the Tally's entry page, and
/// once that closes the app goes back to where the user was.
void main() {
  late Store store;
  late Notes notes;
  late Tallies tallies;
  late FakeHomeShortcuts shortcuts;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  const pin = '2468';

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    tallies = Tallies(store);
    shortcuts = FakeHomeShortcuts();
  });
  tearDown(() => store.close());

  /// "Weight", a Tally of one field with a target.
  String weight({String? place}) {
    final note = notes.capture('Weight\nevery morning', place: place);
    tallies.addField(note.id, name: 'Weight', unit: 'kg', target: 70);
    return note.id;
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    if (finder.evaluate().isEmpty) await reveal(tester, finder);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester, {HomeShortcuts? on}) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        shortcuts: on ?? shortcuts,
        lock: FakeBiometrics(can: false),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// The app goes to the background for [away] and comes back.
  Future<void> away(WidgetTester tester, Duration away) async {
    final binding = tester.binding;
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    now = now.add(away);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
  }

  /// Unlocks the App Lock's screen with the PIN.
  Future<void> unlock(WidgetTester tester) async {
    await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
    await tap(tester, find.byKey(const ValueKey('unlockPin')));
  }

  /// Starts the app and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> cardMenu(WidgetTester tester) => tap(tester, key('note-menu'));

  final entryPage = find.byType(TallyEntryScreen);

  group('"Add to home screen" in the card\'s ⋮', () {
    testWidgets("asks for a shortcut named as the Tally, the Android "
        "prompt doing the rest", (tester) async {
      final id = weight();
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(find.text('Add to home screen'), findsOneWidget);
      await tap(tester, key('note-add-to-home'));

      expect(shortcuts.pinned, [(id, 'Weight')]);
      // Android shows its own prompt: the app says nothing.
      expect(find.text('This home screen takes no shortcuts.'), findsNothing);
    });

    testWidgets('says at the top when the home screen takes none', (
      tester,
    ) async {
      shortcuts = FakeHomeShortcuts(takes: false);
      weight();
      await startAtNotes(tester);
      await cardMenu(tester);
      await tap(tester, key('note-add-to-home'));

      expect(find.text('This home screen takes no shortcuts.'), findsOneWidget);
    });

    testWidgets('names a Tally in a locked Area set to Masked "Private '
        'Tally"', (tester) async {
      final places = Places(store);
      final health = places.addArea('Health');
      final id = weight(place: health.id);
      store
        ..setPin(pin, iterations: 10)
        ..setAreaLocked(health.id, true)
        ..setAreaMasked(health.id, true);
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      await tap(tester, find.text('Health'));
      await tester.enterText(find.byKey(const ValueKey('pinField')), pin);
      await tap(tester, find.widgetWithText(TextButton, 'Unlock'));
      await cardMenu(tester);
      await tap(tester, key('note-add-to-home'));

      expect(shortcuts.pinned, [(id, 'Private Tally')]);
    });

    testWidgets('reads "Add to the app icon" on the iPhone and says at '
        'the top where the Tally went', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      final id = weight();
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(find.text('Add to home screen'), findsNothing);
      expect(find.text('Add to the app icon'), findsOneWidget);
      await tap(tester, key('note-add-to-home'));

      expect(shortcuts.pinned, [(id, 'Weight')]);
      expect(
        find.text(
          "Weight is in the app icon's menu: touch and hold the icon on the "
          'Home Screen.',
        ),
        findsOneWidget,
      );
      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('is offered only for a Tally, and only where the phone '
        'has shortcuts', (tester) async {
      notes.capture('Call the vet');
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(key('note-add-to-home'), findsNothing);
    });

    testWidgets('is not offered where the phone has no shortcuts', (
      tester,
    ) async {
      shortcuts = FakeHomeShortcuts(available: false);
      weight();
      await startAtNotes(tester);
      await cardMenu(tester);
      expect(key('note-add-entry'), findsOneWidget);
      expect(key('note-add-to-home'), findsNothing);
    });
  });

  group("a Tally's shortcut tapped", () {
    testWidgets('opens its entry page at start; Save records the entry '
        'and goes back to the home screen', (tester) async {
      final id = weight();
      shortcuts.waiting = id;
      await start(tester);

      expect(entryPage, findsOneWidget);
      expect(shortcuts.left, 0);
      await tap(tester, key('tally-entry-save'));

      expect(tallies.entries(id), hasLength(1));
      expect(entryPage, findsNothing);
      expect(shortcuts.left, 1);
    });

    testWidgets('while the app runs opens over what is open, which stays '
        'once the page closes, saved or not', (tester) async {
      final id = weight();
      await startAtNotes(tester);
      expect(find.byType(AllNotesScreen), findsOneWidget);

      shortcuts.tap(id);
      await tester.pumpAndSettle();
      expect(entryPage, findsOneWidget);
      // Left as it opened: nothing is written, and still back home.
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();

      expect(entryPage, findsNothing);
      expect(tallies.entries(id), isEmpty);
      expect(find.byType(AllNotesScreen), findsOneWidget);
      expect(shortcuts.left, 1);
    });

    testWidgets('behind the App Lock opens only after the unlock', (
      tester,
    ) async {
      store
        ..setPin(pin, iterations: 10)
        ..appLockOn = true;
      final id = weight();
      shortcuts.waiting = id;
      await start(tester);

      expect(find.byType(LockScreen), findsOneWidget);
      expect(entryPage, findsNothing);
      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));

      expect(find.byType(LockScreen), findsNothing);
      expect(entryPage, findsOneWidget);
    });

    testWidgets('tapped while locked and left without unlocking opens the '
        'page after a later unlock, and stays in the app once it closes', (
      tester,
    ) async {
      store
        ..setPin(pin, iterations: 10)
        ..appLockOn = true;
      final id = weight();
      shortcuts.waiting = id;
      await start(tester);
      expect(find.byType(LockScreen), findsOneWidget);

      // Back later from the app's own icon, not from the shortcut.
      await away(tester, const Duration(hours: 1));
      await unlock(tester);
      expect(entryPage, findsOneWidget);
      await tap(tester, key('tally-entry-save'));

      expect(tallies.entries(id), hasLength(1));
      expect(shortcuts.left, 0);
    });

    testWidgets('left open while the app locks again does not send the app '
        'away once it is back', (tester) async {
      store
        ..setPin(pin, iterations: 10)
        ..appLockOn = true;
      final id = weight();
      shortcuts.waiting = id;
      await start(tester);
      await unlock(tester);
      expect(entryPage, findsOneWidget);

      await away(tester, const Duration(hours: 1));

      expect(find.byType(LockScreen), findsOneWidget);
      expect(entryPage, findsNothing);
      expect(shortcuts.left, 0);
    });

    testWidgets('for a Tally in a locked Area asks to open it first; given '
        'up, it opens nothing', (tester) async {
      final health = Places(store).addArea('Health');
      final id = weight(place: health.id);
      store
        ..setPin(pin, iterations: 10)
        ..setAreaLocked(health.id, true);
      shortcuts.waiting = id;
      await start(tester);

      expect(find.byKey(const ValueKey('pinField')), findsOneWidget);
      expect(entryPage, findsNothing);
      await tap(tester, find.widgetWithText(TextButton, 'Cancel'));

      expect(entryPage, findsNothing);
      expect(shortcuts.left, 0);
    });

    testWidgets('for a Tally in a locked Area opens its page once the Area '
        'is open', (tester) async {
      final health = Places(store).addArea('Health');
      final id = weight(place: health.id);
      store
        ..setPin(pin, iterations: 10)
        ..setAreaLocked(health.id, true);
      shortcuts.waiting = id;
      await start(tester);

      await tester.enterText(find.byKey(const ValueKey('pinField')), pin);
      await tap(tester, find.widgetWithText(TextButton, 'Unlock'));
      expect(entryPage, findsOneWidget);
    });

    testWidgets('for a Tally in the trash opens only the app, and says so', (
      tester,
    ) async {
      final id = weight();
      notes.trash(id);
      shortcuts.waiting = id;
      await start(tester);

      expect(entryPage, findsNothing);
      expect(find.text('This Tally is no longer there.'), findsOneWidget);
      expect(shortcuts.left, 0);
    });

    testWidgets('for a Tally deleted for good opens only the app, and says '
        'so', (tester) async {
      shortcuts.waiting = 'gone';
      await start(tester);

      expect(entryPage, findsNothing);
      expect(find.text('This Tally is no longer there.'), findsOneWidget);
      expect(shortcuts.left, 0);
    });
  });

  testWidgets('the shortcuts channel as TallyShortcuts.kt and '
      'CaptureChannel.swift answer it: take, pinTally and leave', (
    tester,
  ) async {
    final id = weight();
    final calls = <MethodCall>[];
    final asked = [id];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
          const MethodChannel('io.github.paxel.oldbrainfix/shortcuts'),
          (call) async {
            calls.add(call);
            return switch (call.method) {
              'take' => asked.isEmpty ? null : asked.removeAt(0),
              'pinTally' => true,
              'leave' => null,
              _ => throw MissingPluginException(),
            };
          },
        );
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(
            const MethodChannel('io.github.paxel.oldbrainfix/shortcuts'),
            null,
          ),
    );
    await start(tester, on: ChannelHomeShortcuts());

    expect(entryPage, findsOneWidget);
    await tap(tester, key('tally-entry-save'));
    expect(entryPage, findsNothing);
    expect(calls.map((call) => call.method), ['take', 'leave']);

    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await cardMenu(tester);
    await tap(tester, key('note-add-to-home'));
    expect(calls.last.method, 'pinTally');
    expect(calls.last.arguments, {'note': id, 'label': 'Weight'});
  });
}
