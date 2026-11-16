import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';

/// A tap on an Occurrence opens its Note (#270): on the Agenda, a
/// Checklist Routine's Line, and its notification; its schedule opens
/// from the row's ⋮ and the Note editor's badge.
void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  late FakeAlarms alarms;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    routines = Routines(store);
    alarms = FakeAlarms();
  });
  tearDown(() => store.close());

  Schedule daily(int hour) => Schedule(
    start: Day(2026, 10, 1),
    times: [ClockTime(hour, 0)],
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  /// "pills" every day at 08:00, made before today's was due.
  Routine pills() {
    final saved = now;
    now = DateTime(2026, 10, 24, 6);
    final routine = routines.create('pills', daily(8));
    now = saved;
    return routine;
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1200, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Finder key(String name) => find.byKey(ValueKey(name));
  final done = key('open-done');

  group('on the Agenda', () {
    testWidgets('a tap on an Occurrence opens its Note with Done, Snooze '
        'and Not today at the top', (tester) async {
      final routine = pills();
      await start(tester);

      await tap(tester, find.text('pills').first);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(key('open-${routine.id}@2026-10-24T08:00'), findsOneWidget);
      expect(done, findsOneWidget);
      expect(key('open-snooze'), findsOneWidget);
      expect(key('open-not-today'), findsOneWidget);

      await tap(tester, done);
      expect(
        routines
            .byId(routine.id)
            ?.doneAt(Occurrence(Day(2026, 10, 24), ClockTime(8, 0))),
        isNotNull,
      );
    });

    testWidgets("Edit schedule in the row's ⋮ opens the Routine, as the "
        "Note editor's badge does", (tester) async {
      final routine = pills();
      await start(tester);

      await tap(tester, key('row-menu-${routine.id}').first);
      expect(find.text('Edit schedule'), findsOneWidget);
      await tap(tester, key('row-schedule'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.byType(NoteEditorScreen), findsNothing);

      await tester.pageBack();
      await tester.pumpAndSettle();
      await tap(tester, find.text('pills').first);
      await tap(tester, key('badge-routine-${routine.id}'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
    });

    testWidgets('a Done Occurrence still offers Edit schedule', (tester) async {
      final routine = pills();
      routines.checkOff(
        routine.id,
        Occurrence(Day(2026, 10, 24), ClockTime(8, 0)),
      );
      await start(tester);

      await tap(tester, key('row-menu-${routine.id}').first);
      expect(key('answer-skip'), findsNothing);
      await tap(tester, key('row-schedule'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
    });

    testWidgets("a Checklist Routine's Line opens the list's Note", (
      tester,
    ) async {
      final note = notes.capture('Packing\nBath\nTent').id;
      final lists = Checklists(store)..make(note);
      routines.spawn(
        note,
        Schedule(
          start: Day(2026, 10, 1),
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      final tent = lists.byNote(note)!.lines.last.id;
      await start(tester);

      await tap(tester, key('checklist-line-$tent').first);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(find.byType(RoutineEditorScreen), findsNothing);
    });

    testWidgets('one whose Note is in the trash opens its Routine', (
      tester,
    ) async {
      final pills = notes.capture('pills');
      routines.spawn(pills.id, daily(20));
      notes.trash(pills.id);
      await start(tester);

      await tap(tester, find.text('pills').first);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.byType(NoteEditorScreen), findsNothing);
    });
  });

  group('its notification', () {
    testWidgets('a tap opens its Note over the Agenda', (tester) async {
      final routine = pills();
      await start(tester);
      expect(find.byType(NoteEditorScreen), findsNothing);

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();

      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(done, findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('a tap starts the app on its Note', (tester) async {
      final routine = pills();
      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await start(tester);

      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(done, findsOneWidget);
    });

    testWidgets('one of a Routine gone, or of a Note in the trash, opens '
        'nothing', (tester) async {
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, daily(8));
      await start(tester);

      alarms.tapOccurrence((
        routineId: 'routine:gone',
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsNothing);

      notes.trash(pills.id);
      await tester.pumpAndSettle();
      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });
  });
}
