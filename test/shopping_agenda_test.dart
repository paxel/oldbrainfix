import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

/// Shopping Lists on the Agenda (#123), the way this Device chose: each
/// Line back today on its own row, one row per list with Lines back (the
/// default), one row per list with open Lines, or none; today only.
void main() {
  late Store store;
  late Notes notes;
  late ShoppingLists lists;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  late String groceries;
  late String hardware;

  String lineId(String note, String text) => lists
      .byNote(note, deviceWallClock)!
      .lines
      .firstWhere((l) => l.text == text)
      .id;

  /// "Groceries": Milk and Eggs checked a week ago, back today; Tea
  /// checked on Tuesday, struck still; Rice never checked. "Hardware":
  /// Nails, never checked.
  void seed() {
    now = DateTime(2026, 10, 17, 10);
    groceries = notes.capture('Groceries\nMilk\nEggs\nTea\nRice').id;
    lists.make(groceries);
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

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
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

  testWidgets('by default one row per list on a day when Lines came back; '
      'a tap opens the list', (tester) async {
    expect(store.shoppingAgenda, ShoppingAgenda.back);
    await start(tester);
    expect(shoppingRows(tester), ['shopping-row-$groceries']);
    expect(
      find.descendant(
        of: key('shopping-row-$groceries'),
        matching: find.text('Groceries: 2 back'),
      ),
      findsOneWidget,
    );
    expect(find.byType(Checkbox), findsNothing, reason: 'no box on a list');
    await tap(tester, key('shopping-row-$groceries'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(key('shopping'), findsOneWidget);
  });

  testWidgets('each Line back today on its own row, checked off there', (
    tester,
  ) async {
    store.shoppingAgenda = ShoppingAgenda.lines;
    final milk = lineId(groceries, 'Milk');
    final eggs = lineId(groceries, 'Eggs');
    await start(tester);
    expect(shoppingRows(tester), [
      'shopping-line-$milk',
      'shopping-line-$eggs',
    ]);
    expect(
      find.descendant(
        of: key('shopping-line-$milk'),
        matching: find.text('Groceries'),
      ),
      findsOneWidget,
    );
    await tap(tester, key('shopping-check-$milk'));
    expect(
      lists.byNote(groceries, deviceWallClock)!.line(milk)!.struck,
      isTrue,
    );
    expect(shoppingRows(tester), ['shopping-line-$eggs']);
    await tap(tester, key('shopping-line-$eggs'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  });

  testWidgets('one row per list every day it has open Lines', (tester) async {
    store.shoppingAgenda = ShoppingAgenda.open;
    await start(tester);
    expect(shoppingRows(tester), [
      'shopping-row-$groceries',
      'shopping-row-$hardware',
    ]);
    expect(find.text('Groceries: 3 open'), findsOneWidget);
    expect(find.text('Hardware: 1 open'), findsOneWidget);
  });

  testWidgets('not at all', (tester) async {
    store.shoppingAgenda = ShoppingAgenda.off;
    await start(tester);
    expect(shoppingRows(tester), isEmpty);
    expect(
      find.text('Nothing due today. Tap + to capture a thought.'),
      findsOneWidget,
    );
  });

  testWidgets('nothing on a day when no Line came back', (tester) async {
    now = DateTime(2026, 10, 25, 9);
    await start(tester);
    expect(shoppingRows(tester), isEmpty);
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
    store.shoppingAgenda = ShoppingAgenda.lines;
    await start(tester);
    expect(shoppingRows(tester), ['shopping-row-$groceries']);
    expect(find.text('Private Shopping List: 2 back'), findsOneWidget);
    expect(find.text('Milk'), findsNothing);
    expect(find.text('Groceries'), findsNothing);
  });
}
