import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

/// A Checklist with a Routine (#121): each Occurrence's row on the Agenda
/// shows the Lines as checked for it, checkable there; the last Line
/// checked makes it Done; the card shows the Occurrence it is on.
void main() {
  late Store store;
  late Notes notes;
  late Checklists lists;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Occurrence(Day(2026, 10, 24));
  final tomorrow = Occurrence(Day(2026, 10, 25));

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    lists = Checklists(store);
    routines = Routines(store);
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

  /// "Packing" with Bath, Tent and Map, a daily all-day Routine from it;
  /// the Routine's id.
  String packing() {
    final note = notes.capture('Packing\nBath\nTent\nMap').id;
    lists.make(note);
    return routines
        .spawn(
          note,
          Schedule(
            start: Day(2026, 10, 1),
            rule: Recurrence(frequency: Frequency.daily),
          ),
        )
        .id;
  }

  String lineId(String text) => lists
      .ofNotes([for (final n in notes.all()) n.id])
      .values
      .single
      .lines
      .firstWhere((l) => l.text == text)
      .id;

  /// The Lines of [routine]'s row for [occurrence].
  Finder row(String routine, Occurrence occurrence) =>
      key('row-checklist-$routine@${occurrence.key}');

  /// The box of the Line [text] on [routine]'s row for [occurrence].
  Finder lineBox(String routine, Occurrence occurrence, String text) =>
      find.descendant(
        of: row(routine, occurrence),
        matching: key('checklist-check-${lineId(text)}'),
      );

  bool? lineChecked(
    WidgetTester tester,
    String routine,
    Occurrence occurrence,
    String text,
  ) => tester.widget<Checkbox>(lineBox(routine, occurrence, text)).value;

  /// The row's own box, which checks the Occurrence off.
  Finder rowBox(String routine, Occurrence occurrence) => find.descendant(
    of: find.ancestor(of: row(routine, occurrence), matching: find.byType(Row)),
    matching: find.byWidgetPredicate(
      (w) => w is Checkbox && w.semanticLabel == 'Done',
    ),
  );

  Checkbox rowCheckbox(
    WidgetTester tester,
    String routine,
    Occurrence occurrence,
  ) => tester.widget<Checkbox>(rowBox(routine, occurrence).first);

  DateTime? doneAt(String routine, Occurrence occurrence) =>
      routines.byId(routine)?.doneAt(occurrence);

  testWidgets('an Occurrence row lists its Lines; a check counts for that '
      'Occurrence only, and a new one starts unchecked', (tester) async {
    final routine = packing();
    await start(tester);
    for (final text in ['Bath', 'Tent', 'Map']) {
      expect(lineBox(routine, today, text), findsOneWidget);
      expect(lineBox(routine, tomorrow, text), findsOneWidget);
    }

    await tap(tester, lineBox(routine, today, 'Bath'));
    expect(lineChecked(tester, routine, today, 'Bath'), isTrue);
    expect(lineChecked(tester, routine, tomorrow, 'Bath'), isFalse);
    // The Note's own Lines stay as they were.
    expect(lists.byNote(notes.all().single.id)!.checkedCount, 0);

    // Tomorrow's row is checked from today, for tomorrow.
    await tap(tester, lineBox(routine, tomorrow, 'Tent'));
    expect(lineChecked(tester, routine, tomorrow, 'Tent'), isTrue);
    expect(lineChecked(tester, routine, today, 'Tent'), isFalse);

    // The next day, today's row starts afresh.
    now = DateTime(2026, 10, 25, 9);
    await tester.pumpWidget(const SizedBox());
    await start(tester);
    final next = Occurrence(Day(2026, 10, 26));
    expect(lineChecked(tester, routine, tomorrow, 'Bath'), isFalse);
    expect(lineChecked(tester, routine, tomorrow, 'Tent'), isTrue);
    expect(lineChecked(tester, routine, next, 'Bath'), isFalse);
    expect(lineChecked(tester, routine, next, 'Tent'), isFalse);
  });

  testWidgets('the last Line checked marks the row Done and keeps its box '
      'checked; unchecking a Line undoes it', (tester) async {
    final routine = packing();
    await start(tester);
    await tap(tester, lineBox(routine, today, 'Bath'));
    await tap(tester, lineBox(routine, today, 'Tent'));
    expect(doneAt(routine, today), isNull);
    expect(rowCheckbox(tester, routine, today).value, isFalse);

    await tap(tester, lineBox(routine, today, 'Map'));
    expect(doneAt(routine, today), isNotNull);
    expect(rowCheckbox(tester, routine, today).value, isTrue);
    // Done from its Lines, it stays checked.
    expect(rowCheckbox(tester, routine, today).onChanged, isNull);
    expect(doneAt(routine, tomorrow), isNull);

    await tap(tester, lineBox(routine, today, 'Tent'));
    expect(doneAt(routine, today), isNull);
    expect(rowCheckbox(tester, routine, today).value, isFalse);
    expect(rowCheckbox(tester, routine, today).onChanged, isNotNull);
  });

  testWidgets('the row is still checked off by hand, its Lines unchecked', (
    tester,
  ) async {
    final routine = packing();
    await start(tester);
    await tap(tester, rowBox(routine, today).first);
    expect(doneAt(routine, today), isNotNull);
    expect(rowCheckbox(tester, routine, today).value, isTrue);
    expect(lineChecked(tester, routine, today, 'Bath'), isFalse);

    await tap(tester, rowBox(routine, today).first);
    expect(doneAt(routine, today), isNull);
    expect(rowCheckbox(tester, routine, today).value, isFalse);
  });

  testWidgets('a check on the card counts for the Occurrence it is on and '
      'shows on its Agenda row', (tester) async {
    final routine = packing();
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    final bath = key('checklist-check-${lineId('Bath')}');
    expect(bath, findsOneWidget);
    await tap(tester, bath);
    expect(tester.widget<Checkbox>(bath).value, isTrue);
    expect(lists.byNote(notes.all().single.id)!.checkedCount, 0);

    while (find.byType(AgendaScreen).hitTestable().evaluate().isEmpty) {
      await tester.pageBack();
      await tester.pumpAndSettle();
    }
    expect(lineChecked(tester, routine, today, 'Bath'), isTrue);
    expect(lineChecked(tester, routine, tomorrow, 'Bath'), isFalse);
  });

  testWidgets('the editor checks and unchecks all for the Occurrence it is '
      'on', (tester) async {
    final routine = packing();
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await tester.longPress(find.text('Packing').first);
    await tester.pumpAndSettle();
    final on = (routineId: routine, occurrence: today);
    final bath = find.descendant(
      of: key('checklist'),
      matching: key('checklist-check-${lineId('Bath')}'),
    );
    await tap(tester, bath);
    expect(tester.widget<Checkbox>(bath).value, isTrue);
    expect(lists.forOccurrence(on)!.checkedCount, 1);
    expect(lists.byNote(notes.all().single.id)!.checkedCount, 0);

    await tap(tester, key('checklist-uncheck-all'));
    expect(tester.widget<Checkbox>(bath).value, isFalse);
    expect(lists.forOccurrence(on)!.checkedCount, 0);
  });
}
