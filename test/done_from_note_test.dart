import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Done from the Note and from its cards (#215).
void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    routines = Routines(store);
  });
  tearDown(() => store.close());

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 1),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  /// From the Agenda, shows All Notes as cards.
  Future<void> allNotes(WidgetTester tester) async {
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// From the Agenda, opens the editor of the Note showing [text].
  Future<void> openNote(WidgetTester tester, String text) async {
    await allNotes(tester);
    await tester.longPress(find.text(text).first);
    await tester.pumpAndSettle();
  }

  final done = find.byKey(const ValueKey('open-done'));
  final doneAndTrash = find.byKey(const ValueKey('open-done-trash'));

  group('the Note editor', () {
    testWidgets('a Reminder due offers Done, which checks it off', (
      tester,
    ) async {
      // Set up at 07:00, before it was due.
      now = DateTime(2026, 10, 24, 7);
      final note = notes.capture('call mum').id;
      final reminder = routines.spawn(
        note,
        Schedule(start: today, times: [at(8)]),
      );
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      await openNote(tester, 'call mum');
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(find.text('Due 08:00'), findsOneWidget);
      expect(doneAndTrash, findsOneWidget);
      await tap(tester, done);
      expect(
        routines.byId(reminder.id)?.doneAt(Occurrence(today, at(8))),
        isNotNull,
      );
      expect(find.text('Due 08:00'), findsNothing);
      expect(done, findsNothing);
      expect(notes.byId(note)?.isTrashed, isFalse);
    });

    testWidgets('"Done and move to trash" checks it off, trashes the Note '
        'with it and closes the editor', (tester) async {
      // Set up at 07:00, before it was due.
      now = DateTime(2026, 10, 24, 7);
      final note = notes.capture('call mum').id;
      final reminder = routines.spawn(
        note,
        Schedule(start: today, times: [at(8)]),
      );
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      await openNote(tester, 'call mum');
      await tap(tester, doneAndTrash);
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(notes.byId(note)?.isTrashed, isTrue);
      final read = routines.byId(reminder.id);
      expect(read?.doneAt(Occurrence(today, at(8))), isNotNull);
      expect(read?.trashedWith, note, reason: 'it comes back with its Note');
    });

    testWidgets('a Routine shows its latest Missed one, with Done only', (
      tester,
    ) async {
      now = DateTime(2026, 10, 22, 6);
      final note = notes.capture('pills').id;
      final pills = routines.spawn(note, daily([at(8)]));
      now = DateTime(2026, 10, 24, 9);
      // Today's is Done; yesterday's became Missed when it started.
      routines.markDone(pills.id, Occurrence(today, at(8)));
      await start(tester);
      await openNote(tester, 'pills');
      expect(find.text('Missed Fri, 10/23/2026 08:00'), findsOneWidget);
      expect(doneAndTrash, findsNothing, reason: 'it repeats');
      await tap(tester, done);
      expect(
        routines.byId(pills.id)?.doneAt(Occurrence(Day(2026, 10, 23), at(8))),
        isNotNull,
      );
    });

    testWidgets('a Reminder beside another Routine of the Note offers no '
        'trash; one not due yet shows nothing', (tester) async {
      now = DateTime(2026, 10, 24, 7);
      final note = notes.capture('dentist').id;
      routines.spawn(note, Schedule(start: today, times: [at(8)]));
      routines.spawn(note, daily([at(15)]));
      final later = notes.capture('later').id;
      routines.spawn(later, Schedule(start: today, times: [at(15)]));
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      await openNote(tester, 'dentist');
      expect(find.text('Due 08:00'), findsOneWidget);
      expect(doneAndTrash, findsNothing);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.longPress(find.text('later').first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(done, findsNothing);
    });
  });

  group('the cards', () {
    testWidgets("a box ticks today's open Occurrence and unchecks it again", (
      tester,
    ) async {
      final note = notes.capture('pills').id;
      final pills = routines.spawn(note, daily([at(8), at(20)]));
      await start(tester);
      await allNotes(tester);
      Finder box(String key) => find.descendant(
        of: find.byKey(ValueKey('card-occurrence-${pills.id}@$key')),
        matching: find.byType(Checkbox),
      );
      final morning = Occurrence(today, at(8));
      expect(box(morning.key), findsOneWidget);
      await tap(tester, box(morning.key));
      expect(routines.byId(pills.id)?.doneAt(morning), isNotNull);
      expect(find.byType(NoteEditorScreen), findsNothing, reason: 'opens none');
      // The next one today is open now.
      final evening = Occurrence(today, at(20));
      expect(box(evening.key), findsOneWidget);
      await tap(tester, box(evening.key));
      expect(routines.byId(pills.id)?.doneAt(evening), isNotNull);
      expect(
        tester.widget<Checkbox>(box(evening.key)).value,
        isTrue,
        reason: 'Done today, there all day',
      );
      await tap(tester, box(evening.key));
      expect(routines.byId(pills.id)?.doneAt(evening), isNull);
    });

    testWidgets('a long press on a card still opens the editor', (
      tester,
    ) async {
      final note = notes.capture('pills').id;
      routines.spawn(note, daily([at(8)]));
      await start(tester);
      await allNotes(tester);
      expect(find.byType(Checkbox), findsOneWidget);
      await tester.longPress(find.text('pills').first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });
  });
}
