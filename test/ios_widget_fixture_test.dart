import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_widget.dart';

/// The Agenda widget's snapshot as the iPhone's widget reads it (#242):
/// this test writes what the app hands over into
/// ios/RunnerTests/agenda_snapshot.json, and the XCTests
/// (AgendaWidgetPlanTests.swift) decode the same file, so both sides
/// agree on its shape. `UPDATE_FIXTURES=1` writes it anew.
void main() {
  const fixture = 'ios/RunnerTests/agenda_snapshot.json';

  /// Every kind of row the snapshot holds, as the Agenda's groups give
  /// them (#209), a Masked one among them, on two days.
  final snapshot = AgendaSnapshot(
    days: {
      '2026-10-24': [
        const WidgetRow(
          kind: WidgetRow.occurrence,
          title: 'Water the plants',
          time: 'All day',
          checks: true,
          routineId: 'water',
          occurrenceKey: '2026-10-24',
          block: WidgetRow.allDay,
        ),
        const WidgetRow(
          kind: WidgetRow.shopping,
          title: 'Groceries',
          block: WidgetRow.lists,
        ),
        const WidgetRow(
          kind: WidgetRow.shopping,
          title: 'Milk',
          checks: true,
          indent: 1,
          lineId: 'milk',
          block: WidgetRow.lists,
        ),
        const WidgetRow(
          kind: WidgetRow.checklist,
          title: 'Packing',
          block: WidgetRow.lists,
        ),
        const WidgetRow(
          kind: WidgetRow.checklistLine,
          title: 'Passport',
          done: true,
          checks: true,
          indent: 1,
          lineId: 'passport',
          block: WidgetRow.lists,
        ),
        const WidgetRow(
          kind: WidgetRow.occurrence,
          title: 'Something in Private',
          time: '08:00',
          checks: true,
          routineId: 'masked',
          occurrenceKey: '2026-10-24T08:00',
          block: WidgetRow.timed,
          at: 480,
        ),
        const WidgetRow(
          kind: WidgetRow.occurrence,
          title: 'Morning',
          time: '09:00',
          checks: true,
          routineId: 'morning',
          occurrenceKey: '2026-10-24T09:00',
          block: WidgetRow.timed,
          at: 540,
        ),
        const WidgetRow(
          kind: WidgetRow.line,
          title: 'Teeth',
          checks: true,
          indent: 1,
          routineId: 'morning',
          occurrenceKey: '2026-10-24T09:00',
          lineId: 'teeth',
          block: WidgetRow.timed,
          at: 540,
        ),
        const WidgetRow(
          kind: WidgetRow.line,
          title: 'Floss',
          checks: true,
          indent: 2,
          routineId: 'morning',
          occurrenceKey: '2026-10-24T09:00',
          lineId: 'floss',
          block: WidgetRow.timed,
          at: 540,
        ),
        const WidgetRow(
          kind: WidgetRow.session,
          title: 'Workout',
          time: '18:00',
          routineId: 'workout',
          occurrenceKey: '2026-10-24T18:00',
          block: WidgetRow.timed,
          at: 1080,
        ),
        const WidgetRow(
          kind: WidgetRow.pick,
          title: 'Push-ups',
          checks: true,
          indent: 1,
          routineId: 'workout',
          occurrenceKey: '2026-10-24T18:00',
          noteId: 'pushups',
          block: WidgetRow.timed,
          at: 1080,
        ),
        const WidgetRow(
          kind: WidgetRow.heading,
          title: 'Eat a frog',
          block: WidgetRow.frogs,
        ),
        const WidgetRow(
          kind: WidgetRow.frog,
          title: 'Call the bank',
          checks: true,
          frogId: 'bank',
          block: WidgetRow.frogs,
        ),
      ],
      '2026-10-25': [
        const WidgetRow(
          kind: WidgetRow.occurrence,
          title: 'Morning',
          time: '09:00',
          checks: true,
          routineId: 'morning',
          occurrenceKey: '2026-10-25T09:00',
          block: WidgetRow.timed,
          at: 540,
        ),
        const WidgetRow(
          kind: WidgetRow.text,
          title: 'Picks 2',
          indent: 1,
          block: WidgetRow.timed,
          at: 540,
        ),
      ],
      '2026-10-26': const [],
    },
    headings: const {
      '2026-10-24': 'Saturday, 24 October',
      '2026-10-25': 'Sunday, 25 October',
      '2026-10-26': 'Monday, 26 October',
    },
    missed: const [
      WidgetRow(kind: WidgetRow.heading, title: 'Missed'),
      WidgetRow(kind: WidgetRow.text, title: 'Pills · 2 times'),
      WidgetRow(
        kind: WidgetRow.missed,
        title: 'Fri, 23 Oct',
        time: '08:00',
        checks: true,
        routineId: 'pills',
        occurrenceKey: '2026-10-23T08:00',
      ),
    ],
    unsorted: 3,
    calendars: const ['calendar-1'],
    today: 'Today',
    tomorrow: 'Tomorrow',
    empty: 'Nothing due today.',
    planned: 'Nothing planned',
    unsortedText: '3 Unsorted',
    done: 'Done',
    allDay: 'All day',
    noTitle: '(No title)',
    since: 'from {time}',
    until: 'until {time}',
    fatFingers: true,
  );

  test('the snapshot the iPhone widget reads is the one the app hands '
      'over (#242)', () {
    final written =
        '${const JsonEncoder.withIndent('  ').convert(snapshot.toMap())}\n';
    final file = File(fixture);
    if (Platform.environment['UPDATE_FIXTURES'] == '1') {
      file.writeAsStringSync(written);
    }
    expect(file.readAsStringSync(), written);
    // Version 4 (#194), as the iPhone's widget reads it.
    expect(jsonDecode(written), containsPair('v', 4));
  });
}
