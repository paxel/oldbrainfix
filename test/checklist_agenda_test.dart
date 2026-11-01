import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

/// Checklists on the Agenda (#143): each without a Routine shows today as
/// one row with how many of its Lines are open, after the Shopping Lists'
/// rows, until every Line is checked or the user switches "Show on the
/// Agenda" off in the ⋮ of its card or editor.
void main() {
  late Store store;
  late Notes notes;
  late Checklists lists;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    lists = Checklists(store);
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

  /// Starts the app and opens every Note's card from the browser.
  Future<void> startAtNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  String lineId(String note, String text) =>
      lists.byNote(note)!.lines.firstWhere((l) => l.text == text).id;

  /// "Before the trip" with Passport, Tickets and Charger, Passport
  /// checked, captured at [at].
  String trip([DateTime? at]) {
    now = at ?? DateTime(2026, 10, 20, 18);
    final id = notes.capture('Before the trip\nPassport\nTickets\nCharger').id;
    lists.make(id);
    lists.check(lineId(id, 'Passport'));
    now = DateTime(2026, 10, 24, 9);
    return id;
  }

  /// Every Checklist row on the Agenda, by key.
  Iterable<String> checklistRows() => [
    for (final element
        in find
            .byWidgetPredicate(
              (w) => switch (w.key) {
                ValueKey<String>(:final value) => value.startsWith(
                  'checklist-row-',
                ),
                _ => false,
              },
            )
            .evaluate())
      (element.widget.key! as ValueKey<String>).value,
  ];

  /// Whether the "Show on the Agenda" entry of the menu open shows on.
  bool switchOn(WidgetTester tester) => tester
      .widget<Switch>(
        find.descendant(
          of: key('note-show-on-agenda'),
          matching: find.byType(Switch),
        ),
      )
      .value;

  testWidgets('a new Checklist shows today with its open Lines; '
      'a tap opens the list', (tester) async {
    final id = trip();
    await start(tester);
    expect(checklistRows(), ['checklist-row-$id']);
    expect(find.text('Before the trip: 2 of 3 open'), findsOneWidget);
    expect(find.byType(Checkbox), findsNothing, reason: 'no box on the row');
    await tap(tester, key('checklist-row-$id'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(key('checklist'), findsOneWidget);
  });

  testWidgets('oldest capture first, after the all-day Occurrences and the '
      'Shopping Lists, before the timed Occurrences', (tester) async {
    final later = trip(DateTime(2026, 10, 22, 8));
    now = DateTime(2026, 10, 10, 8);
    final first = notes.capture('Garage\nSort screws').id;
    lists.make(first);
    final groceries = notes.capture('Groceries\nMilk').id;
    ShoppingLists(store).make(groceries);
    final routines = Routines(store);
    routines.create(
      'Bins out',
      Schedule(start: Day(2026, 10, 1), rule: Recurrence.parse('FREQ=DAILY')),
    );
    routines.create(
      'Evening pills',
      Schedule(
        start: Day(2026, 10, 1),
        times: [ClockTime(20, 0)],
        rule: Recurrence.parse('FREQ=DAILY'),
      ),
    );
    now = DateTime(2026, 10, 24, 9);
    await start(tester);
    expect(checklistRows(), ['checklist-row-$first', 'checklist-row-$later']);
    double top(Finder finder) => tester.getTopLeft(finder.first).dy;
    final order = [
      top(find.text('Bins out')),
      top(key('shopping-row-$groceries')),
      top(key('checklist-row-$first')),
      top(key('checklist-row-$later')),
      top(find.text('Evening pills')),
    ];
    expect(order, [...order]..sort());
    expect(find.text('Garage: 1 of 1 open'), findsOneWidget);
  });

  testWidgets('every Line checked, it is gone', (tester) async {
    final id = trip();
    lists.check(lineId(id, 'Tickets'));
    lists.check(lineId(id, 'Charger'));
    await start(tester);
    expect(checklistRows(), isEmpty);
    expect(
      find.text('Nothing due today. Tap + to capture a thought.'),
      findsOneWidget,
    );
  });

  testWidgets('switched off, it stays off the Agenda', (tester) async {
    final id = trip();
    lists.setShowOnAgenda(id, false);
    await start(tester);
    expect(checklistRows(), isEmpty);
  });

  testWidgets('the card\'s ⋮ switches "Show on the Agenda" off and on', (
    tester,
  ) async {
    final id = trip();
    await startAtNotes(tester);
    await tap(tester, key('note-menu'));
    expect(find.text('Show on the Agenda'), findsOneWidget);
    expect(switchOn(tester), isTrue);
    await tap(tester, key('note-show-on-agenda'));
    expect(lists.showsOnAgenda(id), isFalse);
    await tap(tester, key('note-menu'));
    expect(switchOn(tester), isFalse);
    await tap(tester, key('note-show-on-agenda'));
    expect(lists.showsOnAgenda(id), isTrue);
  });

  testWidgets('the editor\'s badge has the switch, also without boxes', (
    tester,
  ) async {
    final id = trip();
    await start(tester);
    await tap(tester, key('checklist-row-$id'));
    expect(key('badge-ticks'), findsNothing);
    await tap(tester, key('badge-agenda'));
    expect(tester.widget<Switch>(key('badge-switch')).value, isTrue);
    await tap(tester, key('badge-switch'));
    expect(lists.showsOnAgenda(id), isFalse);
    await tap(tester, key('badge-close'));
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(checklistRows(), isEmpty);
  });

  testWidgets('a Checklist with a Routine shows as its Occurrence only, '
      'without the switch', (tester) async {
    final id = trip();
    Routines(store).spawn(
      id,
      Schedule(start: Day(2026, 10, 1), rule: Recurrence.parse('FREQ=DAILY')),
    );
    await startAtNotes(tester);
    await tap(tester, key('note-menu'));
    expect(key('note-show-on-agenda'), findsNothing);
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
    await tester.longPress(find.text('Before the trip').first);
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(key('badge-agenda'), findsNothing);
  });

  testWidgets('no row for a Checklist with a Routine', (tester) async {
    final id = trip();
    Routines(store).spawn(
      id,
      Schedule(start: Day(2026, 10, 1), rule: Recurrence.parse('FREQ=DAILY')),
    );
    await start(tester);
    expect(checklistRows(), isEmpty);
    expect(find.text('Before the trip'), findsWidgets, reason: 'its rows');
  });

  testWidgets('a Checklist of a Masked Area is a row that tells nothing', (
    tester,
  ) async {
    final id = trip();
    store.setPin('2468', iterations: 10);
    final home = Places(store).addArea('Home');
    notes.file(id, home.id);
    store.setAreaLocked(home.id, true);
    store.setAreaMasked(home.id, true);
    await start(tester);
    expect(checklistRows(), ['checklist-row-$id']);
    expect(find.text('Private Checklist: 2 of 3 open'), findsOneWidget);
    expect(find.textContaining('Before the trip'), findsNothing);
  });
}
