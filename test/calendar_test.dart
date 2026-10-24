import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/calendar_export_screen.dart';
import 'package:oldbrainfix/src/calendar_platform.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_calendar.dart';

void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  late FakeCalendar calendar;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    routines = Routines(store);
    calendar = FakeCalendar();
  });
  tearDown(() => store.close());

  ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

  Schedule daily(List<ClockTime> times) => Schedule(
    start: Day(2026, 10, 24),
    times: times,
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  Future<void> start(WidgetTester tester, {CalendarPlatform? platform}) async {
    await tester.pumpWidget(
      OldBrainFixApp(store: store, calendar: platform ?? calendar),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor of the Note [id] from the Agenda.
  Future<void> edit(WidgetTester tester, String id) async {
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, Places(store), notes.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  final calendarButton = find.byTooltip('Calendar');
  final exportButton = find.byTooltip('Export to calendar');
  const share = 'Share as calendar file';

  group('Add to calendar', () {
    testWidgets('a Routine opens the calendar app filled in', (tester) async {
      routines.create('pills', daily([at(8)]));
      await start(tester);
      await tap(tester, find.text('pills').first);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);

      await tap(tester, calendarButton);
      expect(find.text(share), findsOneWidget);
      await tap(tester, find.text('Add to calendar: Every day · 08:00'));

      final event = calendar.inserted.single;
      expect(event.summary, 'pills');
      expect(event.start, Occurrence(Day(2026, 10, 24), at(8)));
      expect(event.rule, 'FREQ=DAILY');
    });

    testWidgets('several times per day: one choice per time', (tester) async {
      routines.create('pills', daily([at(8), at(20)]));
      await start(tester);
      await tap(tester, find.text('pills').first);

      await tap(tester, calendarButton);
      expect(find.text('Add to calendar: Every day · 08:00'), findsOneWidget);
      await tap(tester, find.text('Add to calendar: Every day · 20:00'));

      final event = calendar.inserted.single;
      expect(event.start, Occurrence(Day(2026, 10, 24), at(20)));
      expect(event.rule, 'FREQ=DAILY');
    });

    testWidgets('a Reminder is added once', (tester) async {
      routines.create(
        'dentist',
        Schedule(start: Day(2026, 10, 24), times: [at(14, 30)]),
      );
      await start(tester);
      await tap(tester, find.text('dentist').first);
      await tap(tester, calendarButton);
      await tap(tester, find.textContaining('Add to calendar: Once'));

      final event = calendar.inserted.single;
      expect(event.start, Occurrence(Day(2026, 10, 24), at(14, 30)));
      expect(event.rule, isNull);
    });

    testWidgets('a Note on its own leaves the time to the calendar app', (
      tester,
    ) async {
      final note = notes.capture('Ring the plumber\nabout the tap');
      await start(tester);
      await edit(tester, note.id);

      // Nothing else to offer: the button adds it at once.
      await tap(tester, calendarButton);
      final event = calendar.inserted.single;
      expect(event.summary, 'Ring the plumber');
      expect(event.description, 'Ring the plumber\nabout the tap');
      expect(event.start, isNull);
      expect(event.rule, isNull);
    });

    testWidgets('a Note offers itself, its Routines and their file', (
      tester,
    ) async {
      final note = notes.capture('pills');
      routines.spawn(note.id, daily([at(8)]));
      await start(tester);
      await edit(tester, note.id);

      await tap(tester, calendarButton);
      expect(find.text('Add to calendar at a time you pick'), findsOneWidget);
      expect(find.text('Add to calendar: Every day · 08:00'), findsOneWidget);
      await tap(tester, find.text(share));

      final (name, ics) = calendar.shared.single;
      expect(name, 'oldbrainfix.ics');
      expect(ics, contains('RRULE:FREQ=DAILY'));
      expect(ics, contains('SUMMARY:pills'));
    });

    testWidgets('without a calendar app it says so at the top', (tester) async {
      calendar.hasCalendarApp = false;
      final note = notes.capture('Ring the plumber');
      await start(tester);
      await edit(tester, note.id);
      await tap(tester, calendarButton);

      expect(
        find.text('There is no calendar app to add it to.'),
        findsOneWidget,
      );
    });

    testWidgets('without a calendar platform no calendar button shows', (
      tester,
    ) async {
      final note = notes.capture('pills');
      routines.spawn(note.id, daily([at(8)]));
      await start(tester, platform: const NoCalendar());
      expect(exportButton, findsNothing);
      await edit(tester, note.id);
      expect(calendarButton, findsNothing);
    });
  });

  group('ICS export', () {
    testWidgets('shares all Routines and Reminders with rules and alarms', (
      tester,
    ) async {
      final pills = routines.create('pills', daily([at(8), at(20)]));
      final dentist = routines.create(
        'dentist',
        Schedule(start: Day(2026, 11, 3), times: [at(14, 30)]),
      );
      await start(tester);
      await tap(tester, exportButton);
      expect(find.byType(CalendarExportScreen), findsOneWidget);
      expect(find.text('pills'), findsOneWidget);
      expect(find.text('dentist'), findsOneWidget);

      await tap(tester, find.byTooltip(share));
      final (name, ics) = calendar.shared.single;
      expect(name, 'oldbrainfix.ics');
      expect(ics, contains('UID:${pills.id}-0800@oldbrainfix'));
      expect(ics, contains('UID:${pills.id}-2000@oldbrainfix'));
      expect(ics, contains('UID:${dentist.id}-1430@oldbrainfix'));
      expect(ics, contains('RRULE:FREQ=DAILY'));
      expect('BEGIN:VALARM'.allMatches(ics), hasLength(3));
    });

    testWidgets('shares only what is picked', (tester) async {
      final pills = routines.create('pills', daily([at(8)]));
      final dentist = routines.create(
        'dentist',
        Schedule(start: Day(2026, 11, 3), times: [at(14, 30)]),
      );
      await start(tester);
      await tap(tester, exportButton);

      await tap(tester, find.text('pills'));
      await tap(tester, find.byTooltip(share));
      final (_, ics) = calendar.shared.single;
      expect(ics, isNot(contains(pills.id)));
      expect(ics, contains(dentist.id));
    });

    testWidgets('"All" drops and picks everything together', (tester) async {
      routines.create('pills', daily([at(8)]));
      await start(tester);
      await tap(tester, exportButton);

      await tap(tester, find.text('All Routines and Reminders'));
      final shareButton = find.byTooltip(share);
      expect(
        tester
            .widget<IconButton>(
              find.ancestor(of: shareButton, matching: find.byType(IconButton)),
            )
            .onPressed,
        isNull,
      );
      await tap(tester, find.text('All Routines and Reminders'));
      await tap(tester, shareButton);
      expect(calendar.shared, hasLength(1));
    });

    testWidgets('nothing to export says so', (tester) async {
      await start(tester);
      await tap(tester, exportButton);
      expect(find.text('No Routines or Reminders yet.'), findsOneWidget);
    });

    testWidgets('when no app takes the file it says so at the top', (
      tester,
    ) async {
      calendar.hasShareTarget = false;
      routines.create('pills', daily([at(8)]));
      await start(tester);
      await tap(tester, exportButton);
      await tap(tester, find.byTooltip(share));
      expect(find.text('No app can take the calendar file.'), findsOneWidget);
    });
  });
}
