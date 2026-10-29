import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
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
    testWidgets('asks first, with their count; cancel keeps everything', (
      tester,
    ) async {
      final pills = notes.capture('pills');
      routines.spawn(pills.id, daily([at(8)]));
      routines.spawn(pills.id, daily([at(20)]));
      await start(tester);
      await openNote(tester, 'pills');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('Delete this Note?'), findsOneWidget);
      expect(find.text('Delete only the Note'), findsOneWidget);
      expect(
        find.text('Delete with its 2 Routines and Reminders'),
        findsOneWidget,
      );

      await tap(tester, find.text('Cancel'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(notes.byId(pills.id)?.isTrashed, isFalse);
      expect(routines.all(), hasLength(2));
    });

    testWidgets('counts a Reminder as linked too', (tester) async {
      final dentist = notes.capture('dentist');
      routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 10, 27), times: [at(9)]),
      );
      await start(tester);
      await openNote(tester, 'dentist');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.text('Delete with its Routine or Reminder'), findsOneWidget);
      await tap(tester, find.text('Cancel'));

      routines.spawn(dentist.id, daily([at(8)]));
      await tester.pageBack();
      await tester.pumpAndSettle();
      await tester.longPress(find.text('dentist'));
      await tester.pumpAndSettle();
      await tap(tester, find.byTooltip('Move to trash'));
      expect(
        find.text('Delete with its 2 Routines and Reminders'),
        findsOneWidget,
      );
    });

    testWidgets(
      'With its Routines: they go to the trash with the Note and come back '
      'with it',
      (tester) async {
        final pills = notes.capture('pills');
        routines.spawn(pills.id, daily([at(20)]));
        await startWeek(tester);
        expect(find.byType(Checkbox), findsNWidgets(AgendaScreen.days));
        await openNote(tester, 'pills');
        await tap(tester, find.byTooltip('Move to trash'));
        await tap(tester, find.text('Delete with its Routine or Reminder'));
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
      'Only the Note: they stay, linking to the Note in the trash, then to '
      'a deleted Note',
      (tester) async {
        final pills = notes.capture('pills');
        routines.spawn(pills.id, daily([at(20)]));
        await startWeek(tester);
        await openNote(tester, 'pills');
        await tap(tester, find.byTooltip('Move to trash'));
        await tap(tester, find.text('Delete only the Note'));
        expect(notes.byId(pills.id)?.isTrashed, isTrue);
        await backToAgenda(tester);
        expect(find.byType(Checkbox), findsNWidgets(AgendaScreen.days));

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
        await tap(tester, find.text('The Note is gone').first);
        expect(find.text('The Note was deleted'), findsOneWidget);
        // A Routine without its Note is still changed and saved.
        await tap(tester, find.text('Save'));
        expect(find.byType(RoutineEditorScreen), findsNothing);
      },
    );

    testWidgets(
      'With its Routine from the back-link closes the Routine\'s editor too',
      (tester) async {
        final pills = notes.capture('pills');
        routines.spawn(pills.id, daily([at(20)]));
        await start(tester);
        await tap(tester, find.text('pills').first);
        await tap(tester, find.text('pills'));
        expect(find.byType(NoteEditorScreen), findsOneWidget);
        await tap(tester, find.byTooltip('Move to trash'));
        await tap(tester, find.text('Delete with its Routine or Reminder'));
        expect(find.byType(NoteEditorScreen), findsNothing);
        expect(find.byType(RoutineEditorScreen), findsNothing);
        expect(find.byType(Checkbox), findsNothing);
      },
    );

    testWidgets('a Merge from the back-link moves it to the Note it went '
        'into', (tester) async {
      notes.capture('dentist');
      final tuesday = notes.capture('dentist Tuesday');
      routines.spawn(tuesday.id, daily([at(20)]));
      await start(tester);
      await tap(tester, find.text('dentist Tuesday').first);
      await tap(tester, find.text('dentist Tuesday'));
      await tap(tester, find.byTooltip('Merge into…'));
      await tap(tester, find.text('dentist'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('dentist'), findsOneWidget);
      expect(find.text('dentist Tuesday'), findsNothing);
    });

    testWidgets('a Routine of a merged Note links to the Note it went into', (
      tester,
    ) async {
      final dentist = notes.capture('dentist');
      final tuesday = notes.capture('dentist Tuesday');
      routines.spawn(tuesday.id, daily([at(20)]));
      notes.merge(tuesday.id, dentist.id);
      await start(tester);
      await openNote(tester, 'dentist');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.text('Delete with its Routine or Reminder'), findsOneWidget);
      await tap(tester, find.text('Delete only the Note'));
      await backToAgenda(tester);
      await tap(tester, find.text('dentist').first);
      expect(find.text('The Note is in the trash'), findsOneWidget);
    });

    testWidgets('a Note without Routines asks too, like its card; cancel '
        'keeps it', (tester) async {
      final milk = notes.capture('milk');
      await start(tester);
      await openNote(tester, 'milk');
      await tap(tester, find.byTooltip('Move to trash'));
      expect(find.text('Delete this Note?'), findsOneWidget);
      expect(find.text('It stays in the trash for 30 days.'), findsOneWidget);
      final delete = tester.widget<TextButton>(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.widgetWithText(TextButton, 'Delete'),
        ),
      );
      expect(
        delete.style?.foregroundColor?.resolve({}),
        Theme.of(tester.element(find.byType(AlertDialog))).colorScheme.error,
      );

      await tap(tester, find.text('Cancel'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(notes.byId(milk.id)?.isTrashed, isFalse);

      await tap(tester, find.byTooltip('Move to trash'));
      await tap(tester, find.text('Delete'));
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(notes.byId(milk.id)?.isTrashed, isTrue);
    });
  });

  group('deleting a Routine on its own', () {
    testWidgets('it goes to the trash after a confirm and can be restored', (
      tester,
    ) async {
      final routine = routines.create('pills', daily([at(20)]));
      await startWeek(tester);
      await tap(tester, find.text('pills').first);
      await tap(tester, find.byTooltip('Delete'));
      expect(
        find.text(
          'Delete this Routine? It stays in the trash for 30 days. '
          'Its Note stays.',
        ),
        findsOneWidget,
      );
      await tap(tester, find.text('Delete').last);
      expect(routines.byId(routine.id)?.isTrashed, isTrue);
      expect(find.byType(Checkbox), findsNothing);

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

  testWidgets('the question and the trash fit a phone in German', (
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
    await tap(tester, find.byTooltip('In den Papierkorb'));
    await tap(
      tester,
      find.text('Mit ihren 2 Routinen und Erinnerungen löschen'),
    );
    await tap(tester, find.byType(BackButton).last);
    await tap(tester, find.byTooltip('Papierkorb'));
    expect(find.text('Blumen gießen'), findsOneWidget);
    expect(find.text('Tabletten'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
