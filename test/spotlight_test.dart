import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/spotlight.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_profile.dart';
import 'open_settings.dart';

/// The tips of 0.7.0's new things (obf-1043): never in [before070].
const _new070 = {
  'agenda': ['agenda-quiet'],
  'capture': ['capture-more'],
  'editor': ['editor-remind', 'editor-code-language', 'editor-linked-from'],
  'routine': ['routine-heads-ups'],
  'settings': ['settings-show-tips', 'settings-grammar'],
};

Set<String> _seen(Store store, String screen) => {
  for (final id in seenSpotlights(store, screen).split(','))
    if (id.isNotEmpty) id,
};

/// A word holds a letter or a digit; "+" and "…" are none.
int _words(String text) => text
    .split(RegExp(r'\s+'))
    .where((w) => w.contains(RegExp(r'[\p{L}\p{N}]', unicode: true)))
    .length;

void main() {
  final en = lookupAppLocalizations(const Locale('en'));
  final de = lookupAppLocalizations(const Locale('de'));

  group('the manifest', () {
    test('holds each id once, with one short sentence in English and '
        'German', () {
      final ids = [
        for (final items in spotlightManifest.values)
          for (final item in items) item.id,
      ];
      expect(ids.toSet(), hasLength(ids.length));
      for (final items in spotlightManifest.values) {
        for (final item in items) {
          for (final t in [en, de]) {
            final text = item.text(t);
            expect(text, isNotEmpty, reason: item.id);
            expect(_words(text), lessThanOrEqualTo(15), reason: text);
          }
        }
      }
    });

    test('before070 names only tips its screen has, none of 0.7.0', () {
      for (final MapEntry(key: screen, value: ids) in before070.entries) {
        final items = {
          for (final item in spotlightManifest[screen] ?? <SpotlightItem>[])
            item.id,
        };
        for (final id in ids) {
          expect(items, contains(id), reason: '$screen: $id');
        }
      }
      for (final MapEntry(key: screen, value: ids) in _new070.entries) {
        for (final id in ids) {
          expect(spotlightScreenOf(id), screen, reason: id);
          expect(before070[screen] ?? const [], isNot(contains(id)));
        }
      }
    });

    test('due is what was not shown, in the manifest order', () {
      final items = spotlightManifest['agenda']!;
      expect(
        [
          for (final item in dueSpotlights('agenda-today,agenda-plus', items))
            item.id,
        ],
        ['agenda-unsorted', 'agenda-browse', 'agenda-quiet', 'agenda-profile'],
      );
      expect(dueSpotlights('', items), hasLength(items.length));
    });
  });

  group('marks', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    test('a fresh Device keeps every tip due', () {
      seedSpotlights(store, firstStartDue: true);
      for (final screen in spotlightManifest.keys) {
        expect(_seen(store, screen), isEmpty, reason: screen);
      }
      expect(store.localSetting(spotlightsStartedKey), '1');
    });

    test('an upgraded Device counts what 0.6.1 had as shown; the tips of '
        '0.7.0 stay due', () {
      seedSpotlights(store, firstStartDue: false);
      for (final MapEntry(key: screen, value: ids) in before070.entries) {
        expect(_seen(store, screen), ids.toSet(), reason: screen);
      }
      for (final MapEntry(key: screen, value: ids) in _new070.entries) {
        final due = {
          for (final item in dueSpotlights(
            seenSpotlights(store, screen),
            spotlightManifest[screen]!,
          ))
            item.id,
        };
        expect(due, containsAll(ids), reason: screen);
      }
      expect(store.localSetting(spotlightsStartedKey), '1');
    });

    test('seeding runs once: Show tips again is not undone by a '
        'restart', () {
      seedSpotlights(store, firstStartDue: false);
      resetSpotlights(store);
      expect(store.localSetting(spotlightsStartedKey), '1');
      seedSpotlights(store, firstStartDue: false);
      for (final screen in spotlightManifest.keys) {
        expect(_seen(store, screen), isEmpty, reason: screen);
      }
    });

    test('the harness marks every tip of every screen shown', () {
      markAllSpotlightsSeen(store);
      for (final MapEntry(key: screen, value: items)
          in spotlightManifest.entries) {
        expect(
          dueSpotlights(seenSpotlights(store, screen), items),
          isEmpty,
          reason: screen,
        );
      }
    });

    test('marks add to those there', () {
      markSpotlightsSeen(store, 'agenda', ['agenda-plus']);
      markSpotlightsSeen(store, 'agenda', ['agenda-today', 'agenda-plus']);
      expect(_seen(store, 'agenda'), {'agenda-plus', 'agenda-today'});
    });
  });

  group('placement', () {
    const screen = Size(400, 800);

    void inside(Rect target) {
      final place = tipPlacement(target, screen);
      final top = place.top ?? screen.height - place.bottom! - tipMinHeight;
      final card = Rect.fromLTWH(place.left, top, place.width, tipMinHeight);
      expect(place.left, greaterThanOrEqualTo(tipMargin - 0.001));
      expect(
        place.left + place.width,
        lessThanOrEqualTo(screen.width - tipMargin + 0.001),
      );
      expect(card.top, greaterThanOrEqualTo(tipMargin - 0.001));
      expect(card.bottom, lessThanOrEqualTo(screen.height - tipMargin + 0.001));
      expect(card.overlaps(target), isFalse, reason: '$target');
    }

    test('a card stays inside the screen and off a small target', () {
      inside(const Rect.fromLTWH(340, 8, 48, 48));
      inside(const Rect.fromLTWH(0, 0, 48, 48));
      inside(const Rect.fromLTWH(330, 720, 56, 56));
      inside(const Rect.fromLTWH(16, 380, 368, 48));
      inside(const Rect.fromLTWH(-20, 790, 60, 40));
    });

    test('a card is no wider than 400 on a wide screen', () {
      final place = tipPlacement(
        const Rect.fromLTWH(600, 100, 48, 48),
        const Size(1200, 800),
      );
      expect(place.width, tipMaxWidth);
    });
  });

  group('a tour', () {
    late Store store;
    late ValueNotifier<bool> unsorted;

    setUp(() {
      store = Store.inMemory();
      unsorted = ValueNotifier(false);
    });
    tearDown(() {
      unsorted.dispose();
      store.close();
    });

    /// A page with four of the Agenda's anchors; the Unsorted count only
    /// while [unsorted] says so, Quiet today never.
    Widget host({bool scope = true, bool held = false}) => MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      navigatorObservers: [spotlightRoutes],
      builder: scope
          ? (context, child) => SpotlightScope(
              store: store,
              child: child ?? const SizedBox.shrink(),
            )
          : null,
      home: SpotlightHold(
        held: held,
        child: Scaffold(
          appBar: AppBar(
            actions: [
              Spotlight(
                id: 'agenda-browse',
                child: IconButton(
                  onPressed: () {},
                  icon: const Icon(Icons.folder_outlined),
                ),
              ),
              Spotlight(
                id: 'agenda-profile',
                child: IconButton(
                  onPressed: () {},
                  icon: const Icon(Icons.person_outline),
                ),
              ),
            ],
          ),
          body: ListView(
            children: [
              ValueListenableBuilder<bool>(
                valueListenable: unsorted,
                builder: (_, on, _) => on
                    ? const Spotlight(
                        id: 'agenda-unsorted',
                        child: SizedBox(height: 48, child: Text('3 Unsorted')),
                      )
                    : const SizedBox.shrink(),
              ),
              const Spotlight(
                id: 'agenda-today',
                child: Padding(
                  padding: EdgeInsets.all(16),
                  child: Text('Today'),
                ),
              ),
            ],
          ),
          floatingActionButton: Spotlight(
            id: 'agenda-plus',
            child: FloatingActionButton(
              onPressed: () {},
              child: const Icon(Icons.add),
            ),
          ),
        ),
      ),
    );

    final tip = find.byType(SpotlightTip);
    final next = find.byKey(const ValueKey('spotNext'));
    final skip = find.byKey(const ValueKey('spotSkip'));

    testWidgets('none runs without the scope', (tester) async {
      await tester.pumpWidget(host(scope: false));
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
      expect(_seen(store, 'agenda'), isEmpty);
    });

    testWidgets('each tip is marked before it shows; Next chains, the '
        'last says Got it, an absent element stays due', (tester) async {
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaPlus), findsOneWidget);
      expect(_seen(store, 'agenda'), {'agenda-plus'});
      expect(find.text(en.spotNext), findsOneWidget);
      expect(find.text(en.spotSkip), findsOneWidget);

      await tester.tap(next);
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaToday), findsOneWidget);
      expect(_seen(store, 'agenda'), {'agenda-plus', 'agenda-today'});

      await tester.tap(next);
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaBrowse), findsOneWidget);

      await tester.tap(next);
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaProfile), findsOneWidget);
      expect(find.text(en.spotDone), findsOneWidget);

      await tester.tap(next);
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
      expect(_seen(store, 'agenda'), {
        'agenda-plus',
        'agenda-today',
        'agenda-browse',
        'agenda-profile',
      });
    });

    testWidgets('an element that shows later gets its tip then, once', (
      tester,
    ) async {
      markSpotlightsSeen(store, 'agenda', [
        'agenda-plus',
        'agenda-today',
        'agenda-browse',
        'agenda-profile',
      ]);
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      expect(tip, findsNothing);

      unsorted.value = true;
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaUnsorted), findsOneWidget);
      await tester.tap(next);
      await tester.pumpAndSettle();
      expect(tip, findsNothing);

      unsorted.value = false;
      await tester.pumpAndSettle();
      unsorted.value = true;
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
    });

    testWidgets('Skip ends the tour and marks the rest shown; an absent '
        'element stays due', (tester) async {
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      await tester.tap(skip);
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
      expect(_seen(store, 'agenda'), {
        'agenda-plus',
        'agenda-today',
        'agenda-browse',
        'agenda-profile',
      });
    });

    testWidgets('a tap outside the card ends the tour', (tester) async {
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      expect(tip, findsOneWidget);
      // Left of the card, which keeps 24 from the edge.
      await tester.tapAt(const Offset(4, 300));
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
      expect(_seen(store, 'agenda'), hasLength(4));
    });

    testWidgets('Back ends the tour', (tester) async {
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      expect(tip, findsOneWidget);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
      expect(_seen(store, 'agenda'), hasLength(4));
    });

    testWidgets('the card sits beside its element, inside the screen', (
      tester,
    ) async {
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      final card = tester.getRect(find.byKey(const ValueKey('spotCard')));
      final plus = tester.getRect(find.byType(FloatingActionButton));
      final size = tester.getSize(find.byType(Scaffold));
      expect(card.overlaps(plus), isFalse);
      expect(card.left, greaterThanOrEqualTo(0));
      expect(card.top, greaterThanOrEqualTo(0));
      expect(card.right, lessThanOrEqualTo(size.width));
      expect(card.bottom, lessThanOrEqualTo(size.height));
    });

    testWidgets('a held page shows its tips once it is let go', (tester) async {
      await tester.pumpWidget(host(held: true));
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
      expect(_seen(store, 'agenda'), isEmpty);

      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaPlus), findsOneWidget);
    });

    testWidgets('all marked, nothing shows', (tester) async {
      markAllSpotlightsSeen(store);
      unsorted.value = true;
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      expect(tip, findsNothing);
    });

    testWidgets('a page on top holds the tour of the one under it', (
      tester,
    ) async {
      markAllSpotlightsSeen(store);
      await tester.pumpWidget(host());
      await tester.pumpAndSettle();
      final navigator = tester.state<NavigatorState>(find.byType(Navigator));
      unawaitedPush(navigator);
      await tester.pumpAndSettle();
      resetSpotlights(store);
      unsorted.value = true;
      await tester.pumpAndSettle();
      expect(tip, findsNothing);

      // Back on top, its tips still due show.
      navigator.pop();
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaPlus), findsOneWidget);
    });
  });

  group('the app', () {
    late Store store;

    setUp(() => store = Store.inMemory()..alarmSetupDone = true);
    tearDown(() => store.close());

    Future<void> start(WidgetTester tester, {bool spotlights = true}) async {
      tester.platformDispatcher.localesTestValue = [const Locale('en')];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: FakeAlarms(),
          profile: FakeProfilePlatform(name: 'Pixel'),
          spotlights: spotlights,
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('shows no tip unless main.dart switches them on', (
      tester,
    ) async {
      store.starterSetOffered = true;
      await start(tester, spotlights: false);
      expect(find.byType(SpotlightTip), findsNothing);
      expect(store.localSetting(spotlightsStartedKey), isNull);
    });

    testWidgets('an upgraded Device sees on the Agenda only the tip of '
        'what is new', (tester) async {
      store.starterSetOffered = true;
      await start(tester);
      expect(find.text(en.spotAgendaQuiet), findsOneWidget);
      expect(find.text(en.spotDone), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('spotNext')));
      await tester.pumpAndSettle();
      expect(find.byType(SpotlightTip), findsNothing);
    });

    testWidgets('a new person gets the Agenda\'s whole tour after the '
        'first start', (tester) async {
      await start(tester);
      expect(find.byType(SpotlightTip), findsNothing);
      await tester.tap(find.byKey(const ValueKey('first-new')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Start'));
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaPlus), findsOneWidget);
      expect(store.localSetting(spotlightsStartedKey), '1');
    });

    testWidgets('Show tips again brings them back on this Device, without '
        'seeding again', (tester) async {
      store.starterSetOffered = true;
      seedSpotlights(store, firstStartDue: false);
      markAllSpotlightsSeen(store);
      await start(tester);
      expect(find.byType(SpotlightTip), findsNothing);
      await openDeviceSettings(tester);
      final row = find.byKey(const ValueKey('showTips'));
      await tester.ensureVisible(row);
      await tester.pumpAndSettle();
      await tester.tap(row);
      await tester.pumpAndSettle();
      expect(find.text(en.settingsShowTipsDone), findsOneWidget);
      for (final screen in spotlightManifest.keys) {
        expect(_seen(store, screen), isEmpty, reason: screen);
      }
      expect(store.localSetting(spotlightsStartedKey), '1');

      // Back on the Agenda, its whole tour.
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.text(en.spotAgendaPlus), findsOneWidget);
    });
  });
}

/// A plain page on top, its Future left to run.
void unawaitedPush(NavigatorState navigator) {
  navigator
      .push(MaterialPageRoute<void>(builder: (_) => const Scaffold()))
      .ignore();
}
