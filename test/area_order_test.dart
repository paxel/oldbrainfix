import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_links.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/browser.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/lock_settings.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

/// The Areas in the person's own order (#280): the Areas list moves
/// them by their handles while rearranging (obf-1089), and every list of
/// Areas follows it.
void main() {
  late Store store;
  late Places places;
  late Area car;
  late Area home;
  late Area work;

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
    places = Places(store);
    home = places.addArea('Household');
    places.addTopic(home.id, 'Kitchen');
    work = places.addArea('Work');
    car = places.addArea('Car');
  });
  tearDown(() => store.close());

  Finder key(String k) => find.byKey(ValueKey(k));

  void tall(WidgetTester tester) {
    tester.view.physicalSize = const Size(2400, 3600);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
  }

  Future<void> openAreas(WidgetTester tester) async {
    tall(tester);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
  }

  Future<void> show(
    WidgetTester tester,
    Widget Function(AppLock lock) screen,
  ) async {
    tall(tester);
    final lock = AppLock(store: store);
    addTearDown(lock.dispose);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: LockScope(lock: lock, child: screen(lock)),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// The [names] as they stand from top to bottom inside [screen].
  List<String> shown(WidgetTester tester, Type screen, List<String> names) {
    double top(String name) => tester
        .getTopLeft(
          find.descendant(of: find.byType(screen), matching: find.text(name)),
        )
        .dy;
    return [...names]..sort((a, b) => top(a).compareTo(top(b)));
  }

  /// ⋮ ▸ Rearrange on the Areas list.
  Future<void> rearrange(WidgetTester tester) async {
    await tester.tap(key('areas-more'));
    await tester.pumpAndSettle();
    await tester.tap(key('areas-rearrange'));
    await tester.pumpAndSettle();
  }

  /// A slow drag from [start] to just past [past], down the list.
  Future<void> dragDown(
    WidgetTester tester,
    Offset start,
    Offset from,
    Offset past,
  ) async {
    final drag = await tester.startGesture(start);
    await tester.pump(const Duration(milliseconds: 100));
    final step = (past.dy - from.dy + 20) / 10;
    for (var i = 0; i < 10; i++) {
      await drag.moveBy(Offset(0, step));
      await tester.pump(const Duration(milliseconds: 16));
    }
    await drag.up();
    await tester.pumpAndSettle();
  }

  List<String> stored() => [for (final a in store.areasInOrder()) a.name];

  const names = ['Household', 'Work', 'Car'];

  testWidgets('the Areas list shows the person\'s own order', (tester) async {
    store.setAreaOrder([work.id, car.id, home.id]);
    await openAreas(tester);
    expect(shown(tester, AreasScreen, names), ['Work', 'Car', 'Household']);
  });

  testWidgets('no handles until Rearrange, and Done in the place of ⋮', (
    tester,
  ) async {
    await openAreas(tester);
    expect(key('area-move-${car.id}'), findsNothing);
    expect(key('area-menu-${car.id}'), findsOneWidget);
    expect(key('areas-rearrange-done'), findsNothing);

    await rearrange(tester);
    expect(key('area-move-${car.id}'), findsOneWidget);
    expect(key('area-menu-${car.id}'), findsNothing);
    expect(key('areas-more'), findsNothing);
    expect(find.text('Done'), findsOneWidget);

    await tester.tap(key('areas-rearrange-done'));
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsNothing);
    expect(key('area-menu-${car.id}'), findsOneWidget);
    expect(key('areas-more'), findsOneWidget);
  });

  testWidgets('dragging a handle moves the Area, for every Device', (
    tester,
  ) async {
    await openAreas(tester);
    expect(shown(tester, AreasScreen, names), ['Car', 'Household', 'Work']);
    await rearrange(tester);

    await dragDown(
      tester,
      tester.getCenter(key('area-move-${car.id}')),
      tester.getCenter(key('area-row-${car.id}')),
      tester.getCenter(key('area-row-${home.id}')),
    );

    expect(stored(), ['Household', 'Car', 'Work']);
    expect(shown(tester, AreasScreen, names), ['Household', 'Car', 'Work']);
  });

  testWidgets('in German the Areas go by the names shown until one is '
      'moved, and a drag moves the row dragged', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    store.close();
    store = Store.inMemory()..starterSetOffered = true;
    places = Places(store)..plantStarterSet({'household', 'work', 'car'});
    String id(String slug) => Places.starterId(PlaceKeys.typeArea, slug);
    tall(tester);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Suchen und stöbern'));
    await tester.pumpAndSettle();
    const german = ['Haushalt', 'Arbeit', 'Auto'];
    expect(shown(tester, AreasScreen, german), ['Arbeit', 'Auto', 'Haushalt']);
    await rearrange(tester);
    expect(find.text('Fertig'), findsOneWidget);

    await dragDown(
      tester,
      tester.getCenter(key('area-move-${id('car')}')),
      tester.getCenter(key('area-row-${id('car')}')),
      tester.getCenter(key('area-row-${id('household')}')),
    );

    expect(
      [for (final a in store.areasInOrder()) a.id],
      [id('work'), id('household'), id('car')],
    );
    expect(shown(tester, AreasScreen, german), ['Arbeit', 'Haushalt', 'Auto']);
  });

  testWidgets('in German Link to… lists the Areas by the names shown', (
    tester,
  ) async {
    store.close();
    store = Store.inMemory()..starterSetOffered = true;
    places = Places(store)..plantStarterSet({'household', 'work', 'car'});
    tall(tester);
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('de'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => pickLinkTarget(context, Notes(store), places),
            child: const Text('pick'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('pick'));
    await tester.pumpAndSettle();
    await tester.enterText(key('linkToSearch'), 'a');
    await tester.pumpAndSettle();
    const german = ['Haushalt', 'Arbeit', 'Auto'];
    final labels = [
      for (final w in tester.widgetList<Text>(find.byType(Text)))
        if (w.data case final data? when german.contains(data)) data,
    ];
    expect(labels, ['Arbeit', 'Auto', 'Haushalt']);
  });

  testWidgets('a long press on an Area moves nothing', (tester) async {
    await openAreas(tester);
    final row = key('area-row-${car.id}');
    final drag = await tester.startGesture(tester.getCenter(row));
    await tester.pump(const Duration(seconds: 1));
    await drag.moveBy(const Offset(0, 300));
    await tester.pump();
    await drag.up();
    await tester.pumpAndSettle();
    expect(stored(), ['Car', 'Household', 'Work']);
  });

  testWidgets('while rearranging a drag anywhere on a row moves it', (
    tester,
  ) async {
    await openAreas(tester);
    await rearrange(tester);
    final row = tester.getRect(key('area-row-${car.id}'));
    await dragDown(
      tester,
      row.centerLeft + const Offset(40, 0),
      row.center,
      tester.getCenter(key('area-row-${home.id}')),
    );
    expect(stored(), ['Household', 'Car', 'Work']);
  });

  testWidgets('while rearranging a tap on a row opens nothing', (tester) async {
    await openAreas(tester);
    await rearrange(tester);
    await tester.tap(key('area-row-${car.id}'));
    await tester.pumpAndSettle();
    expect(find.byType(AreaScreen), findsNothing);
    expect(key('area-move-${car.id}'), findsOneWidget);
    await tester.tap(key('tags-row'));
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsOneWidget);
  });

  testWidgets('a swipe right on a row goes back to the Agenda', (tester) async {
    await openAreas(tester);
    await tester.flingFrom(
      tester.getCenter(key('area-row-${home.id}')),
      const Offset(600, 0),
      1000,
    );
    await tester.pumpAndSettle();
    expect(stored(), ['Car', 'Household', 'Work']);
    expect(find.byType(AgendaScreen), findsOneWidget);
    expect(find.byType(AreasScreen), findsNothing);
  });

  testWidgets('while rearranging a swipe on a row stays on the page', (
    tester,
  ) async {
    await openAreas(tester);
    // A last chosen Area, which swipe left would open.
    await tester.tap(key('area-row-${car.id}'));
    await tester.pumpAndSettle();
    expect(find.byType(AreaScreen), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();
    await rearrange(tester);
    for (final way in [const Offset(600, 0), const Offset(-600, 0)]) {
      await tester.flingFrom(
        tester.getCenter(key('area-row-${home.id}')),
        way,
        1000,
      );
      await tester.pumpAndSettle();
      expect(find.byType(AreasScreen), findsOneWidget);
      expect(find.byType(AreaScreen), findsNothing);
      expect(key('area-move-${home.id}'), findsOneWidget);
    }
    expect(stored(), ['Car', 'Household', 'Work']);
  });

  testWidgets('leaving the page ends rearranging and keeps the order', (
    tester,
  ) async {
    await openAreas(tester);
    await rearrange(tester);
    await dragDown(
      tester,
      tester.getCenter(key('area-move-${car.id}')),
      tester.getCenter(key('area-row-${car.id}')),
      tester.getCenter(key('area-row-${home.id}')),
    );
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(AgendaScreen), findsOneWidget);
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsNothing);
    expect(key('areas-more'), findsOneWidget);
    expect(stored(), ['Household', 'Car', 'Work']);
    expect(shown(tester, AreasScreen, names), ['Household', 'Car', 'Work']);
  });

  testWidgets('Done fits a narrow bar with Fat fingers', (tester) async {
    store.fatFingers = true;
    tester.view.physicalSize = const Size(960, 2400);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    // Fat fingers grew the bar's buttons.
    expect(tester.getSize(find.byTooltip('All Notes')).width, greaterThan(48));
    await rearrange(tester);
    expect(key('areas-rearrange-done'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('All Notes ends rearranging', (tester) async {
    await openAreas(tester);
    await rearrange(tester);
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsNothing);
    expect(key('areas-more'), findsOneWidget);
  });

  testWidgets('a screen reader moves an Area down and up', (tester) async {
    await openAreas(tester);
    VoidCallback action(Area area, String label) {
      final semantics = tester
          .widgetList<Semantics>(
            find.ancestor(
              of: key('area-row-${area.id}'),
              matching: find.byType(Semantics),
            ),
          )
          .firstWhere((s) => s.properties.customSemanticsActions != null);
      return semantics.properties.customSemanticsActions!.entries
          .firstWhere((e) => e.key.label == label)
          .value;
    }

    action(car, 'Move down')();
    await tester.pumpAndSettle();
    expect(stored(), ['Household', 'Car', 'Work']);
    action(work, 'Move up')();
    await tester.pumpAndSettle();
    expect(stored(), ['Household', 'Work', 'Car']);
    expect(shown(tester, AreasScreen, names), ['Household', 'Work', 'Car']);
  });

  testWidgets('a new Area goes to the end of the order', (tester) async {
    store.setAreaOrder([work.id, car.id, home.id]);
    await openAreas(tester);
    await tester.tap(find.byTooltip('New Area'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      'Attic',
    );
    await tester.tap(find.text('Create'));
    await tester.pumpAndSettle();
    expect(shown(tester, AreasScreen, [...names, 'Attic']), [
      'Work',
      'Car',
      'Household',
      'Attic',
    ]);
  });

  testWidgets('a search ends rearranging and has no Rearrange', (tester) async {
    await openAreas(tester);
    await rearrange(tester);
    expect(key('area-move-${car.id}'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'milk');
    await tester.pump(BrowserSearchField.pause);
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsNothing);
    expect(key('areas-rearrange-done'), findsNothing);
    expect(key('areas-more'), findsNothing);
    await tester.enterText(find.byType(TextField), '');
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsNothing);
    expect(key('areas-more'), findsOneWidget);
  });

  testWidgets('File into… lists the Areas in the person\'s order', (
    tester,
  ) async {
    store.setAreaOrder([work.id, car.id, home.id]);
    await show(tester, (_) => FileIntoScreen(places: places));
    expect(shown(tester, FileIntoScreen, names), ['Work', 'Car', 'Household']);
  });

  testWidgets('the Area locks list the Areas in the person\'s order', (
    tester,
  ) async {
    store.setAreaOrder([work.id, car.id, home.id]);
    final alarms = AlarmSync(
      store: store,
      platform: FakeAlarms(),
      wallClock: deviceWallClock,
    );
    addTearDown(alarms.dispose);
    await show(
      tester,
      (lock) => AreaLocksScreen(lock: lock, alarms: alarms, places: places),
    );
    expect(shown(tester, AreaLocksScreen, names), ['Work', 'Car', 'Household']);
  });

  testWidgets('Link to… lists the Areas in the person\'s order', (
    tester,
  ) async {
    store.setAreaOrder([work.id, car.id, home.id]);
    final notes = Notes(store);
    tall(tester);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => pickLinkTarget(context, notes, places),
            child: const Text('pick'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('pick'));
    await tester.pumpAndSettle();
    // "o" keeps Work and the Household, by name the other way round.
    await tester.enterText(key('linkToSearch'), 'o');
    await tester.pumpAndSettle();
    final labels = [
      for (final w in tester.widgetList<Text>(find.byType(Text)))
        if (w.data case final data? when names.contains(data)) data,
    ];
    expect(labels, ['Work', 'Household']);
  });
}
