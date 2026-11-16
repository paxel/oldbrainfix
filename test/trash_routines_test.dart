import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/all_routines_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/trash_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

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

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  /// Starts on a page tall enough for the Agenda's whole week.
  Future<void> startWeek(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 4000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await start(tester);
  }

  /// Starts the app anew, as after it was closed.
  Future<void> restart(WidgetTester tester) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await start(tester);
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the Routine of the Agenda row [row] through its menu's Edit
  /// schedule, as a tap on the row opens its Note (#270).
  Future<void> editSchedule(WidgetTester tester, Finder row) async {
    await tester.ensureVisible(row);
    await tester.pumpAndSettle();
    await tester.longPress(row);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('row-schedule')));
    await tester.pumpAndSettle();
  }

  /// From the Agenda, opens the editor of the Note showing [text].
  Future<void> openNote(WidgetTester tester, String text) async {
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await tester.longPress(find.text(text));
    await tester.pumpAndSettle();
  }

  /// From the notes browser, back to the Agenda.
  Future<void> backToAgenda(WidgetTester tester) async {
    while (find.byType(Checkbox).evaluate().isEmpty &&
        find.byTooltip('Search and browse').evaluate().isEmpty) {
      await tester.pageBack();
      await tester.pumpAndSettle();
    }
  }

  Future<void> openTrash(WidgetTester tester) async {
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('Trash'));
  }

  group('deleting a Note that spawned Routines', () {
    testWidgets('asks nothing; the announcement at the top undoes it with '
        'its Routines (#268)', (tester) async {
      final pills = notes.capture('pills');
      routines.spawn(pills.id, daily([at(8)]));
      routines.spawn(pills.id, daily([at(20)]));
      await start(tester);
      await openNote(tester, 'pills');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(notes.byId(pills.id)?.isTrashed, isTrue);
      expect(routines.all(), isEmpty);
      expect(find.text('Moved to the Trash'), findsOneWidget);
      expect(find.text('Undo'), findsOneWidget);

      await tap(tester, find.text('Moved to the Trash'));
      expect(find.text('Moved to the Trash'), findsNothing);
      expect(notes.byId(pills.id)?.isTrashed, isFalse);
      expect(routines.all(), hasLength(2));
      // The list it was left for shows it again.
      expect(find.text('pills'), findsOneWidget);
    });

    testWidgets('left alone, the announcement goes by itself and the Note '
        'stays in the trash', (tester) async {
      final pills = notes.capture('pills');
      routines.spawn(pills.id, daily([at(8)]));
      await start(tester);
      await openNote(tester, 'pills');
      await tap(tester, find.byTooltip('Move to trash'));
      await tester.pump(const Duration(seconds: 9));
      expect(find.text('Moved to the Trash'), findsOneWidget);
      await tester.pump(const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(find.text('Moved to the Trash'), findsNothing);
      expect(notes.byId(pills.id)?.isTrashed, isTrue);
      expect(routines.all(), isEmpty);
    });

    testWidgets(
      'Delete: its Routines go to the trash with the Note and come back '
      'with it',
      (tester) async {
        final pills = notes.capture('pills');
        routines.spawn(pills.id, daily([at(20)]));
        await startWeek(tester);
        expect(find.byType(Checkbox), findsNWidgets(AgendaScreen.days));
        await openNote(tester, 'pills');
        await tap(tester, find.byTooltip('Move to trash'));
        expect(find.byType(NoteEditorScreen), findsNothing);
        expect(notes.byId(pills.id)?.isTrashed, isTrue);
        expect(routines.all(), isEmpty);
        await backToAgenda(tester);
        expect(find.byType(Checkbox), findsNothing);

        await openTrash(tester);
        // Only the Note shows: its Routine comes back with it.
        expect(find.text('pills'), findsOneWidget);
        expect(find.text('Restore'), findsOneWidget);
        await tap(tester, find.text('Restore'));
        expect(find.text('The trash is empty.'), findsOneWidget);
        await backToAgenda(tester);
        expect(find.byType(Checkbox), findsNWidgets(AgendaScreen.days));
      },
    );

    testWidgets(
      'one left behind before #262 links to the Note in the trash, then to '
      'a deleted Note, and All Routines and Reminders marks it',
      (tester) async {
        final pills = notes.capture('pills');
        routines.spawn(pills.id, daily([at(20)]));
        routines.create('water', daily([at(9)]));
        // As "Delete only the Note" left it.
        notes.trash(pills.id);
        await startWeek(tester);
        expect(find.byType(Checkbox), findsNWidgets(2 * AgendaScreen.days));

        await tap(tester, find.byKey(const ValueKey('allRoutines')));
        expect(find.text('The Note is in the trash'), findsOneWidget);
        await tester.pageBack();
        await tester.pumpAndSettle();

        await tap(tester, find.text('pills').first);
        expect(find.byType(RoutineEditorScreen), findsOneWidget);
        expect(find.text('The Note is in the trash'), findsOneWidget);
        // A Note in the trash is not opened from here.
        await tap(tester, find.text('pills'));
        expect(find.byType(NoteEditorScreen), findsNothing);
        // The Routine is still saved.
        await tap(tester, find.text('Save'));
        expect(find.byType(RoutineEditorScreen), findsNothing);

        now = now.add(Notes.trashKeep);
        await restart(tester);
        expect(notes.byId(pills.id), isNull);
        await tap(tester, find.byKey(const ValueKey('allRoutines')));
        expect(find.text('The Note was deleted'), findsOneWidget);
        expect(find.text('The Note is in the trash'), findsNothing);
        await tester.pageBack();
        await tester.pumpAndSettle();
        await tap(tester, find.text('The Note is gone').first);
        expect(find.text('The Note was deleted'), findsOneWidget);
        // A Routine without its Note is still changed and saved.
        await tap(tester, find.text('Save'));
        expect(find.byType(RoutineEditorScreen), findsNothing);
      },
    );

    testWidgets('Delete from the back-link closes the Routine\'s editor too', (
      tester,
    ) async {
      final pills = notes.capture('pills');
      routines.spawn(pills.id, daily([at(20)]));
      await start(tester);
      await editSchedule(tester, find.text('pills').first);
      await tap(tester, find.text('pills'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(find.byType(Checkbox), findsNothing);
    });

    testWidgets('a Merge from the back-link moves it to the Note it went '
        'into', (tester) async {
      notes.capture('dentist');
      final tuesday = notes.capture('dentist Tuesday');
      routines.spawn(tuesday.id, daily([at(20)]));
      await start(tester);
      await editSchedule(tester, find.text('dentist Tuesday').first);
      await tap(tester, find.text('dentist Tuesday'));
      await tap(tester, find.byTooltip('Merge into…'));
      await tap(tester, find.text('dentist'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('dentist'), findsOneWidget);
      expect(find.text('dentist Tuesday'), findsNothing);
    });

    testWidgets('a Routine of a merged Note goes with the Note it went '
        'into', (tester) async {
      final dentist = notes.capture('dentist');
      final tuesday = notes.capture('dentist Tuesday');
      routines.spawn(tuesday.id, daily([at(20)]));
      notes.merge(tuesday.id, dentist.id);
      await start(tester);
      await openNote(tester, 'dentist');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(notes.byId(dentist.id)?.isTrashed, isTrue);
      expect(routines.all(), isEmpty);
    });

    testWidgets('a Note without Routines goes at once too, like from its '
        'card', (tester) async {
      final milk = notes.capture('milk');
      await start(tester);
      await openNote(tester, 'milk');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(notes.byId(milk.id)?.isTrashed, isTrue);
      expect(find.text('Moved to the Trash'), findsOneWidget);
    });
  });

  group('deleting a Routine on its own', () {
    testWidgets('it goes to the trash without asking, the announcement '
        'undoes it, and the trash restores it', (tester) async {
      final routine = routines.create('pills', daily([at(20)]));
      await startWeek(tester);
      await editSchedule(tester, find.text('pills').first);
      await tap(tester, find.byTooltip('Delete'));
      expect(find.byType(AlertDialog), findsNothing);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(routines.byId(routine.id)?.isTrashed, isTrue);
      expect(find.byType(Checkbox), findsNothing);
      await tap(tester, find.text('Routine moved to the Trash'));
      expect(routines.byId(routine.id)?.isTrashed, isFalse);
      expect(find.byType(Checkbox), findsNWidgets(AgendaScreen.days));

      await editSchedule(tester, find.text('pills').first);
      await tap(tester, find.byTooltip('Delete'));
      expect(routines.byId(routine.id)?.isTrashed, isTrue);

      now = now.add(const Duration(days: 10));
      await openTrash(tester);
      expect(find.byType(TrashScreen), findsOneWidget);
      expect(find.text('pills'), findsOneWidget);
      expect(find.text('Every day · 20:00'), findsOneWidget);
      expect(find.text('Deleted for good in 20 days'), findsOneWidget);
      await tap(tester, find.text('Restore'));
      expect(find.text('The trash is empty.'), findsOneWidget);
      expect(routines.byId(routine.id)?.isTrashed, isFalse);
      await backToAgenda(tester);
      expect(find.byType(Checkbox), findsNWidgets(AgendaScreen.days));
    });

    testWidgets('the undo brings its badge back in the Note editor it was '
        'opened from', (tester) async {
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, daily([at(20)]));
      await start(tester);
      await openNote(tester, 'pills');
      final badge = find.byKey(ValueKey('badge-routine-${routine.id}'));
      await tester.ensureVisible(badge);
      await tester.pumpAndSettle();
      await tap(tester, badge);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      await tap(tester, find.byTooltip('Delete'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(badge, findsNothing);
      await tap(tester, find.text('Routine moved to the Trash'));
      expect(routines.byId(routine.id)?.isTrashed, isFalse);
      expect(badge, findsOneWidget);
    });

    testWidgets('All Routines and Reminders shows what the undo brought '
        'back', (tester) async {
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, daily([at(20)]));
      await start(tester);
      await tap(tester, find.byKey(const ValueKey('allRoutines')));
      final card = find.byKey(ValueKey('routine-${routine.id}'));
      await tap(tester, card);
      await tap(tester, find.byTooltip('Delete'));
      expect(find.byType(AllRoutinesScreen), findsOneWidget);
      expect(card, findsNothing);
      await tap(tester, find.text('Routine moved to the Trash'));
      expect(card, findsOneWidget);

      // Its Note deleted from there and brought back: no longer "in the
      // trash".
      await tap(tester, card);
      await tap(
        tester,
        find.descendant(
          of: find.byType(RoutineEditorScreen),
          matching: find.text('pills'),
        ),
      );
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(card, findsNothing);
      await tap(tester, find.text('Moved to the Trash'));
      expect(card, findsOneWidget);
      expect(find.text('The Note is in the trash'), findsNothing);
    });

    testWidgets('after 30 days the trash has purged it', (tester) async {
      final routine = routines.create('pills', daily([at(20)]));
      routines.trash(routine.id);
      now = now.add(Notes.trashKeep);
      await start(tester);
      expect(routines.byId(routine.id), isNull);
      await openTrash(tester);
      expect(find.text('The trash is empty.'), findsOneWidget);
    });
  });

  testWidgets('the announcement and the trash fit a phone in German', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1080, 2340);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final pills = notes.capture('Tabletten');
    routines.spawn(pills.id, daily([at(8)]));
    routines.spawn(pills.id, Schedule(start: Day(2026, 10, 27)));
    routines.trash(routines.create('Blumen gießen', daily([at(18)])).id);
    await start(tester);
    await tap(tester, find.byTooltip('Suchen und stöbern'));
    await tap(tester, find.byTooltip('Alle Notizen'));
    await tester.longPress(find.text('Tabletten'));
    await tester.pumpAndSettle();
    // On a phone the bar's last actions overflow into its ⋮.
    await tap(tester, find.byTooltip('Mehr').last);
    await tap(tester, find.text('In den Papierkorb'));
    expect(find.text('In den Papierkorb verschoben'), findsOneWidget);
    expect(find.text('Rückgängig'), findsOneWidget);
    await tap(tester, find.byType(BackButton).last);
    await tap(tester, find.byTooltip('Papierkorb'));
    expect(find.text('Blumen gießen'), findsOneWidget);
    expect(find.text('Tabletten'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
