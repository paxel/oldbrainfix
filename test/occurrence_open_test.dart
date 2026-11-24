import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'dart:async';

import 'package:oldbrainfix/src/agenda_reader.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/missed_summary_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/notification_taps.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';

import 'fake_alarms.dart';

/// Reads as [InlineAgendaReader] does, but hands the Agenda's rows over
/// a moment later, as the app's isolate does (#207).
class _LateReader implements AgendaReader {
  const _LateReader();

  static const _inline = InlineAgendaReader();

  @override
  FutureOr<AgendaLoad> load(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) {
    final load = _inline.load(store, wallClock, system);
    return Future.delayed(const Duration(milliseconds: 50), () => load);
  }

  @override
  FutureOr<AgendaLoad> read(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) => _inline.read(store, wallClock, system);

  @override
  FutureOr<AlarmPlanRead> plan(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) => _inline.plan(store, wallClock, t, locale);
}

/// A tap on an Occurrence opens its Note (#270): on the Agenda and a
/// Checklist Routine's Line; its schedule opens from the row's ⋮ and the
/// Note editor's badge. A tap on its notification opens the Agenda with
/// it marked where the Agenda lists it (obf-1039).
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
    final marked = key('agenda-marked');

    /// Whether the marked row lies within the screen, under the bar.
    void expectInView(WidgetTester tester) {
      expect(marked, findsOneWidget);
      final rect = tester.getRect(marked);
      final screen = tester.view.physicalSize / tester.view.devicePixelRatio;
      expect(rect.top, greaterThanOrEqualTo(0));
      expect(rect.bottom, lessThanOrEqualTo(screen.height));
    }

    Finder markedIn(Finder of) => find.descendant(of: of, matching: marked);

    testWidgets('a tap opens the Agenda with its row marked, not its Note, '
        'over whatever the app showed (obf-1039)', (tester) async {
      final routine = pills();
      await start(tester);
      await tap(tester, find.text('pills').first);
      expect(find.byType(NoteEditorScreen), findsOneWidget);

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();

      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(
        markedIn(key('row-${routine.id}@2026-10-24T08:00')),
        findsOneWidget,
      );
      // Done, Snooze and Not today stay on the row: its box and its ⋮.
      await tap(tester, key('row-menu-${routine.id}').first);
      expect(key('answer-snooze10'), findsOneWidget);
      expect(key('answer-skip'), findsOneWidget);
    });

    testWidgets('a tap starts the app on the Agenda, its row marked', (
      tester,
    ) async {
      final routine = pills();
      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await start(tester);

      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(
        markedIn(key('row-${routine.id}@2026-10-24T08:00')),
        findsOneWidget,
      );
    });

    testWidgets('a tap that starts the app waits for the rows read off '
        'the UI thread, then marks its row', (tester) async {
      final routine = pills();
      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ), from: TapFrom.cold);
      tester.view.physicalSize = const Size(1200, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: alarms,
          agendaReader: const _LateReader(),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        markedIn(key('row-${routine.id}@2026-10-24T08:00')),
        findsOneWidget,
      );
    });

    testWidgets('what was typed in the Note editor is saved, and a '
        'Routine editor stays open over the Agenda with its edit', (
      tester,
    ) async {
      final routine = pills();
      await start(tester);
      await tap(tester, find.text('pills').first);
      await tester.enterText(
        find
            .descendant(
              of: find.byType(NoteEditorScreen),
              matching: find.byType(TextField),
            )
            .first,
        'pills, two of them',
      );
      await tester.pump();

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(notes.byId(routine.noteId)?.text, contains('pills, two of them'));

      await tap(tester, key('row-menu-${routine.id}').first);
      await tap(tester, key('row-schedule'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      await tester.enterText(
        find
            .descendant(
              of: find.byType(RoutineEditorScreen),
              matching: find.byType(TextField),
            )
            .first,
        'pills at eight',
      );
      await tester.pump();

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();
      expect(
        find.byType(RoutineEditorScreen),
        findsOneWidget,
        reason: 'it saves or asks when left, never drops what was typed',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(RoutineEditorScreen), findsNothing);
      expect(marked, findsOneWidget);
    });

    testWidgets("one of a coming day is scrolled into view where the "
        'Agenda lists it, marked', (tester) async {
      tester.view.physicalSize = const Size(400, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final saved = now;
      now = DateTime(2026, 10, 24, 6);
      for (var i = 0; i < 14; i++) {
        routines.create('chore $i', daily(10 + i ~/ 2));
      }
      final routine = routines.create('pills', daily(8));
      now = saved;
      await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
      await tester.pumpAndSettle();
      final row = key('row-${routine.id}@2026-10-28T08:00');
      expect(row, findsNothing, reason: 'not built before the tap');

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-28T08:00',
      ));
      await tester.pumpAndSettle();

      expect(markedIn(row), findsOneWidget);
      expectInView(tester);
    });

    testWidgets('one Missed is marked in the Missed Summary, scrolled back '
        'into view', (tester) async {
      tester.view.physicalSize = const Size(400, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final saved = now;
      now = DateTime(2026, 10, 22, 6);
      final routine = routines.create('pills', daily(8));
      now = DateTime(2026, 10, 24, 6);
      for (var i = 0; i < 14; i++) {
        routines.create('chore $i', daily(10 + i ~/ 2));
      }
      now = saved;
      await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
      await tester.pumpAndSettle();
      await tester.drag(find.byType(Scrollable).first, const Offset(0, -2000));
      await tester.pumpAndSettle();
      expect(find.byType(MissedSummaryCard), findsNothing);

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-23T08:00',
      ));
      await tester.pumpAndSettle();

      expect(markedIn(find.byType(MissedSummaryCard)), findsOneWidget);
      expectInView(tester);
    });

    testWidgets('one Done or skipped meanwhile opens the Agenda with '
        '"Already done" or "Skipped" at the top', (tester) async {
      final announcements = Announcements();
      addTearDown(announcements.dispose);
      final saved = now;
      now = DateTime(2026, 10, 22, 6);
      final routine = routines.create('pills', daily(8));
      now = saved;
      routines.checkOff(
        routine.id,
        Occurrence(Day(2026, 10, 22), ClockTime(8, 0)),
      );
      routines.skip(routine.id, Occurrence(Day(2026, 10, 23), ClockTime(8, 0)));
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: alarms,
          announcements: announcements,
        ),
      );
      await tester.pumpAndSettle();

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-22T08:00',
      ));
      await tester.pumpAndSettle();
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(marked, findsNothing);
      expect(announcements.visible, 'Already done');
      announcements.dismiss();
      await tester.pumpAndSettle();

      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-23T08:00',
      ));
      await tester.pumpAndSettle();
      expect(marked, findsNothing);
      expect(announcements.visible, 'Skipped');
    });

    testWidgets('one of a Routine gone, or of a Note in the trash, opens '
        'the Agenda alone', (tester) async {
      final announcements = Announcements();
      addTearDown(announcements.dispose);
      final pills = notes.capture('pills');
      final routine = routines.spawn(pills.id, daily(8));
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: alarms,
          announcements: announcements,
        ),
      );
      await tester.pumpAndSettle();

      alarms.tapOccurrence((
        routineId: 'routine:gone',
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);

      notes.trash(pills.id);
      await tester.pumpAndSettle();
      alarms.tapOccurrence((
        routineId: routine.id,
        occurrenceKey: '2026-10-24T08:00',
      ));
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
      expect(marked, findsNothing);
      expect(announcements.visible, isNull);
    });
  });
}
