import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/setting_badges.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// Shopping Lists on today's Agenda (#123, #186), each as this person
/// chose for it with "Show on the Agenda" in the ⋮ of its card or as the
/// badge in the editor: off, one row with the number of its open Lines
/// (the default), or each open Line on its own row; a list without open
/// Lines never shows.
void main() {
  late Store store;
  late Notes notes;
  late ShoppingLists lists;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  late String groceries;
  late String hardware;

  String lineId(String note, String text, [Store? on]) =>
      ShoppingLists(on ?? store)
          .byNote(note, deviceWallClock)!
          .lines
          .firstWhere((l) => l.text == text)
          .id;

  /// "Groceries", each Line a week: Milk and Eggs checked a week ago,
  /// back today; Tea
  /// checked on Tuesday, struck still; Rice never checked. "Hardware":
  /// Nails, never checked.
  void seed() {
    now = DateTime(2026, 10, 17, 10);
    groceries = notes.capture('Groceries\nMilk\nEggs\nTea\nRice').id;
    lists.make(groceries);
    // Each Line a week (#195).
    for (final line in lists.byNote(groceries, deviceWallClock)!.lines) {
      lists.setReappear(line.id, Reappear.after(1, ReappearUnit.week));
    }
    lists.check(lineId(groceries, 'Milk'), deviceWallClock);
    lists.check(lineId(groceries, 'Eggs'), deviceWallClock);
    now = DateTime(2026, 10, 20, 18);
    lists.check(lineId(groceries, 'Tea'), deviceWallClock);
    hardware = notes.capture('Hardware\nNails').id;
    lists.make(hardware);
    now = DateTime(2026, 10, 24, 9);
  }

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    lists = ShoppingLists(store);
    seed();
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    if (finder.evaluate().isEmpty) await reveal(tester, finder);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester, [Store? on]) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: on ?? store));
    await tester.pumpAndSettle();
  }

  /// Starts the app and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester, [Store? on]) async {
    await start(tester, on);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// Every Shopping List row on the Agenda, by key.
  Iterable<String> shoppingRows(WidgetTester tester) => [
    for (final element
        in find
            .byWidgetPredicate(
              (w) => switch (w.key) {
                ValueKey<String>(:final value) =>
                  value.startsWith('shopping-row-') ||
                      value.startsWith('shopping-line-'),
                _ => false,
              },
            )
            .evaluate())
      (element.widget.key! as ValueKey<String>).value,
  ];

  /// Which of the three choices of the ⋮ open shows chosen.
  List<bool> menuChosen(WidgetTester tester) => [
    for (final way in ShoppingOnAgenda.values)
      find
          .descendant(
            of: key('note-show-on-agenda-${way.name}'),
            matching: find.byIcon(Icons.radio_button_checked),
          )
          .evaluate()
          .isNotEmpty,
  ];

  testWidgets('by default one row per list with open Lines, how many are '
      'open; a tap opens the list', (tester) async {
    expect(lists.onAgenda(groceries), ShoppingOnAgenda.count);
    await start(tester);
    expect(shoppingRows(tester), [
      'shopping-row-$groceries',
      'shopping-row-$hardware',
    ]);
    expect(
      find.descendant(
        of: key('shopping-row-$groceries'),
        matching: find.text('Groceries: 3 open'),
      ),
      findsOneWidget,
    );
    expect(find.text('Hardware: 1 open'), findsOneWidget);
    expect(find.byType(Checkbox), findsNothing, reason: 'no box on a list');
    await tap(tester, key('shopping-row-$groceries'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(key('shopping'), findsOneWidget);
  });

  testWidgets('each open Line indented under the list\'s name, each with '
      'its box, checked off there (#208)', (tester) async {
    lists.setOnAgenda(groceries, ShoppingOnAgenda.lines);
    final milk = lineId(groceries, 'Milk');
    final eggs = lineId(groceries, 'Eggs');
    final rice = lineId(groceries, 'Rice');
    await start(tester);
    expect(shoppingRows(tester), [
      'shopping-row-$groceries',
      'shopping-line-$milk',
      'shopping-line-$eggs',
      'shopping-line-$rice',
      'shopping-row-$hardware',
    ]);
    Finder inList(Finder finder) =>
        find.descendant(of: key('shopping-row-$groceries'), matching: finder);
    // The heading is the list's name alone, without a box.
    expect(inList(find.text('Groceries')), findsOneWidget);
    expect(inList(find.byType(Checkbox)), findsNWidgets(3));
    expect(inList(key('shopping-line-$milk')), findsOneWidget);
    expect(
      tester.getTopLeft(key('shopping-check-$milk')).dx,
      greaterThanOrEqualTo(tester.getTopLeft(find.text('Groceries')).dx),
      reason: 'indented under the heading',
    );
    expect(
      tester.getTopLeft(key('shopping-check-$milk')).dy,
      greaterThan(tester.getTopLeft(find.text('Groceries')).dy),
    );
    await tap(tester, key('shopping-check-$milk'));
    expect(
      lists.byNote(groceries, deviceWallClock)!.line(milk)!.struck,
      isTrue,
    );
    expect(shoppingRows(tester), [
      'shopping-row-$groceries',
      'shopping-line-$eggs',
      'shopping-line-$rice',
      'shopping-row-$hardware',
    ]);
    await tap(tester, key('shopping-line-$eggs'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  });

  testWidgets('a list chosen off stays off the Agenda', (tester) async {
    lists.setOnAgenda(groceries, ShoppingOnAgenda.off);
    lists.setOnAgenda(hardware, ShoppingOnAgenda.off);
    await start(tester);
    expect(shoppingRows(tester), isEmpty);
    expect(
      find.text('Nothing due today. Tap + to capture a thought.'),
      findsOneWidget,
    );
  });

  testWidgets('a list without open Lines never shows, whatever the choice', (
    tester,
  ) async {
    lists.setOnAgenda(hardware, ShoppingOnAgenda.lines);
    lists.check(lineId(hardware, 'Nails'), deviceWallClock);
    for (final line in ['Milk', 'Eggs', 'Rice']) {
      lists.check(lineId(groceries, line), deviceWallClock);
    }
    await start(tester);
    expect(shoppingRows(tester), isEmpty);
  });

  testWidgets('a list nobody chose for shows as this Device showed '
      'Shopping Lists before', (tester) async {
    store.setLocalSetting('shoppingAgenda', ShoppingAgenda.off.name);
    lists.setOnAgenda(hardware, ShoppingOnAgenda.count);
    await start(tester);
    expect(shoppingRows(tester), ['shopping-row-$hardware']);
    store.setLocalSetting('shoppingAgenda', ShoppingAgenda.lines.name);
    await tester.pumpWidget(const SizedBox());
    await start(tester);
    expect(shoppingRows(tester), [
      'shopping-row-$groceries',
      for (final line in ['Milk', 'Eggs', 'Rice'])
        'shopping-line-${lineId(groceries, line)}',
      'shopping-row-$hardware',
    ]);
  });

  testWidgets('a list of a Masked Area is one row that tells nothing', (
    tester,
  ) async {
    store.setPin('2468', iterations: 10);
    final places = Places(store);
    final home = places.addArea('Home');
    notes.file(groceries, home.id);
    store.setAreaLocked(home.id, true);
    store.setAreaMasked(home.id, true);
    lists.setOnAgenda(groceries, ShoppingOnAgenda.lines);
    await start(tester);
    expect(shoppingRows(tester), [
      'shopping-row-$groceries',
      'shopping-row-$hardware',
    ]);
    expect(find.text('Private Shopping List: 3 open'), findsOneWidget);
    expect(find.text('Milk'), findsNothing);
    expect(find.text('Groceries'), findsNothing);
  });

  group('the choice', () {
    // One list, so the browser shows one card and one ⋮.
    setUp(() => notes.trash(groceries));

    testWidgets('the card\'s ⋮ offers the three ways as radio buttons, '
        'the one it has marked', (tester) async {
      await startAtNotes(tester);
      await tap(tester, key('note-menu'));
      expect(find.text('Show on the Agenda'), findsOneWidget);
      expect(key('note-show-on-agenda'), findsNothing, reason: 'no switch');
      expect(menuChosen(tester), [false, true, false]);
      await tap(tester, key('note-show-on-agenda-lines'));
      expect(lists.onAgenda(hardware), ShoppingOnAgenda.lines);
      await tap(tester, key('note-menu'));
      expect(menuChosen(tester), [false, false, true]);
      await tap(tester, key('note-show-on-agenda-off'));
      expect(lists.onAgenda(hardware), ShoppingOnAgenda.off);
    });

    testWidgets('it is offered while no Line is open', (tester) async {
      lists.check(lineId(hardware, 'Nails'), deviceWallClock);
      await startAtNotes(tester);
      await tap(tester, key('note-menu'));
      await tap(tester, key('note-show-on-agenda-lines'));
      expect(lists.onAgenda(hardware), ShoppingOnAgenda.lines);
    });

    testWidgets('the editor\'s badge names it and chooses it', (tester) async {
      await start(tester);
      await tap(tester, key('shopping-row-$hardware'));
      expect(
        tester.widget<SettingBadge>(key('badge-agenda')).value,
        'One row with the number of open Lines',
      );
      await tap(tester, key('badge-agenda'));
      expect(
        tester
            .widget<RadioGroup<ShoppingOnAgenda>>(
              find.byType(RadioGroup<ShoppingOnAgenda>),
            )
            .groupValue,
        ShoppingOnAgenda.count,
      );
      await tap(tester, key('shopping-agenda-lines'));
      expect(find.byType(RadioGroup<ShoppingOnAgenda>), findsNothing);
      expect(lists.onAgenda(hardware), ShoppingOnAgenda.lines);
      expect(
        tester.widget<SettingBadge>(key('badge-agenda')).value,
        'Each open Line on its own row',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(shoppingRows(tester), [
        'shopping-row-$hardware',
        'shopping-line-${lineId(hardware, 'Nails')}',
      ]);
    });

    testWidgets('a Checklist offers the same three ways, kept as its own '
        'choice (#219)', (tester) async {
      notes.trash(hardware);
      final trip = notes.capture('Before the trip\nPassport').id;
      final checklists = Checklists(store)..make(trip);
      await startAtNotes(tester);
      await tap(tester, key('note-menu'));
      expect(key('note-show-on-agenda'), findsNothing, reason: 'no switch');
      expect(menuChosen(tester), [false, false, true]);
      await tap(tester, key('note-show-on-agenda-count'));
      expect(checklists.onAgenda(trip), ListOnAgenda.count);
      expect(lists.mayShowOnAgenda(trip), isFalse, reason: 'no Shopping List');
    });
  });

  group('per person', () {
    late Store ben;
    late String note;
    var tick = DateTime.utc(2026, 10, 24, 8);
    DateTime clock() => tick = tick.add(const Duration(seconds: 1));

    /// Ben's phone syncs with Ann's Device as a peer of her Shared Area.
    void sync({Invite? inviting}) {
      final done = answerSync(
        store,
        wire(
          syncRequest(
            ben,
            store.versionVector(),
            name: 'Phone',
            hostDevice: store.deviceId,
            invited: inviting != null,
          ),
        ),
        name: 'Desk',
        inviting: inviting,
      )!;
      applyAnswer(ben, wire(done.answer), shared: true);
      // Both remember each other as peers of the Shared Area, as the
      // apps do.
      final at = DateTime.utc(2026, 10, 24);
      store.rememberDevice(
        PairedDevice(
          id: ben.deviceId,
          name: 'Phone',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: at,
          shared: true,
        ),
      );
      ben.rememberDevice(
        PairedDevice(
          id: store.deviceId,
          name: 'Desk',
          fingerprint: 'ab' * 32,
          host: '192.168.1.2',
          port: 4000,
          lastSync: at,
          shared: true,
        ),
      );
    }

    setUp(() {
      store
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ann');
      ben = Store.inMemory(clock: clock)
        ..starterSetOffered = true
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ben');
      notes.trash(groceries);
      notes.trash(hardware);
      final house = Places(store).addArea('Household');
      note = notes.capture('Shop\nMilk\nEggs', place: house.id).id;
      lists.make(note);
      final area = store.share(house.id);
      sync(inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    testWidgets('a Member chooses for his own Agenda alone', (tester) async {
      await startAtNotes(tester, ben);
      await tap(tester, key('note-menu'));
      await tap(tester, key('note-show-on-agenda-lines'));
      expect(ShoppingLists(ben).onAgenda(note), ShoppingOnAgenda.lines);
      sync();
      expect(
        ShoppingLists(ben).byNote(note, deviceWallClock),
        isNotNull,
        reason: 'the list came over',
      );
      expect(lists.onAgenda(note), ShoppingOnAgenda.count);
      expect(lists.agendaRows(deviceWallClock).map((row) => row.shows), [
        ShoppingOnAgenda.count,
      ]);
    });
  });
}
