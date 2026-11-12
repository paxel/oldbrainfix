import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/alarm_sync.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_links.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/lock_settings.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

/// The Areas in the person's own order (#280): the Areas list moves
/// them by their handles, and every list of Areas follows it.
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

  List<String> stored() => [for (final a in store.areasInOrder()) a.name];

  const names = ['Household', 'Work', 'Car'];

  testWidgets('the Areas list shows the person\'s own order', (tester) async {
    store.setAreaOrder([work.id, car.id, home.id]);
    await openAreas(tester);
    expect(shown(tester, AreasScreen, names), ['Work', 'Car', 'Household']);
  });

  testWidgets('dragging a handle moves the Area, for every Device', (
    tester,
  ) async {
    await openAreas(tester);
    expect(shown(tester, AreasScreen, names), ['Car', 'Household', 'Work']);

    final from = tester.getCenter(key('area-row-${car.id}'));
    final past = tester.getCenter(key('area-row-${home.id}'));
    final drag = await tester.startGesture(
      tester.getCenter(key('area-move-${car.id}')),
    );
    await tester.pump(const Duration(milliseconds: 100));
    final step = (past.dy - from.dy + 20) / 10;
    for (var i = 0; i < 10; i++) {
      await drag.moveBy(Offset(0, step));
      await tester.pump(const Duration(milliseconds: 16));
    }
    await drag.up();
    await tester.pumpAndSettle();

    expect(stored(), ['Household', 'Car', 'Work']);
    expect(shown(tester, AreasScreen, names), ['Household', 'Car', 'Work']);
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

  testWidgets('a swipe right from a handle still goes back to the Agenda', (
    tester,
  ) async {
    await openAreas(tester);
    await tester.flingFrom(
      tester.getCenter(key('area-move-${home.id}')),
      const Offset(600, 0),
      1000,
    );
    await tester.pumpAndSettle();
    expect(stored(), ['Car', 'Household', 'Work']);
    expect(find.byType(AgendaScreen), findsOneWidget);
    expect(find.byType(AreasScreen), findsNothing);
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

  testWidgets('no handles while searching', (tester) async {
    await openAreas(tester);
    expect(key('area-move-${car.id}'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'milk');
    await tester.pumpAndSettle();
    expect(key('area-move-${car.id}'), findsNothing);
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
