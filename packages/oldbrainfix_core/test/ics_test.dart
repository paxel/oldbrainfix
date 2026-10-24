import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

ClockTime at(int hour, [int minute = 0]) => ClockTime(hour, minute);

Routine routine(Schedule schedule, {String id = 'routine:r1'}) =>
    Routine(id: id, noteId: 'note:n1', schedule: schedule);

Schedule schedule(
  String? rule, {
  Day? start,
  List<ClockTime> times = const [],
}) => Schedule(
  start: start ?? Day(2026, 10, 24),
  times: times,
  rule: rule == null ? null : Recurrence.parse(rule),
);

final stamp = DateTime.utc(2026, 10, 24, 7, 5, 9);

/// The file's logical lines: folds undone, CRLFs gone.
List<String> unfolded(String ics) =>
    ics.replaceAll('\r\n ', '').split('\r\n')..removeLast();

/// The lines of the one VEVENT in [ics] that has [uid].
List<String> event(String ics, String uid) {
  final lines = unfolded(ics);
  final at = lines.indexOf('UID:$uid');
  expect(at, isNot(-1), reason: 'no event $uid');
  final begin = lines.lastIndexOf('BEGIN:VEVENT', at);
  final end = lines.indexOf('END:VEVENT', at);
  return lines.sublist(begin, end + 1);
}

void main() {
  group('the file', () {
    test('a calendar around the events, every line ending in CRLF', () {
      final ics = writeIcs(
        routineEvents(routine(schedule(null, times: [at(8)])), 'Pills'),
        stamp: stamp,
      );
      expect(ics, startsWith('BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:'));
      expect(ics, endsWith('END:VEVENT\r\nEND:VCALENDAR\r\n'));
      expect(ics.replaceAll('\r\n', ''), isNot(contains('\n')));
      expect(ics.replaceAll('\r\n', ''), isNot(contains('\r')));
      expect(unfolded(ics), contains('DTSTAMP:20261024T070509Z'));
    });

    test('no events is an empty calendar', () {
      expect(
        writeIcs(const [], stamp: stamp),
        startsWith('BEGIN:VCALENDAR\r\n'),
      );
      expect(writeIcs(const [], stamp: stamp), isNot(contains('VEVENT')));
    });

    test('an event without a start is left out', () {
      final ics = writeIcs([
        CalendarEvent.ofText('note:n1', 'Call Mum'),
      ], stamp: stamp);
      expect(ics, isNot(contains('VEVENT')));
    });
  });

  group('escaping and folding', () {
    test('backslash, semicolon, comma and line breaks are escaped', () {
      final ics = writeIcs(
        routineEvents(
          routine(schedule(null, times: [at(8)])),
          'Buy a\\b; milk, eggs\r\nand bread\nnow',
        ),
        stamp: stamp,
      );
      final lines = event(ics, 'routine:r1-0800@oldbrainfix');
      expect(lines, contains(r'SUMMARY:Buy a\\b\; milk\, eggs'));
      expect(
        lines,
        contains(r'DESCRIPTION:Buy a\\b\; milk\, eggs\nand bread\nnow'),
      );
    });

    test('long lines fold at 75 octets without cutting a character', () {
      final text = '${'Medikament für Oma ü 💊 ' * 8}Ende';
      final ics = writeIcs(
        routineEvents(routine(schedule(null, times: [at(8)])), text),
        stamp: stamp,
      );
      final physical = ics.split('\r\n')..removeLast();
      for (final line in physical) {
        expect(utf8.encode(line).length, lessThanOrEqualTo(75), reason: line);
      }
      expect(physical.where((l) => l.startsWith(' ')), isNotEmpty);
      // Unfolding gives the text back whole: no character was split.
      expect(
        event(ics, 'routine:r1-0800@oldbrainfix'),
        contains('SUMMARY:${text.replaceAll(',', r'\,')}'),
      );
    });

    test('a line of exactly 75 octets is not folded', () {
      // "SUMMARY:" is 8 octets.
      final text = 'x' * 67;
      final ics = writeIcs(
        routineEvents(routine(schedule(null, times: [at(8)])), text),
        stamp: stamp,
      );
      expect(ics, contains('\r\nSUMMARY:$text\r\nBEGIN:VALARM'));
    });
  });

  group('Reminders', () {
    test('a timed Reminder is one floating event with an alarm at start', () {
      final events = routineEvents(
        routine(schedule(null, start: Day(2026, 11, 3), times: [at(14, 30)])),
        'Dentist',
      );
      expect(events, hasLength(1));
      expect(events.single.rule, isNull);
      final lines = event(
        writeIcs(events, stamp: stamp),
        'routine:r1-1430@oldbrainfix',
      );
      expect(lines, [
        'BEGIN:VEVENT',
        'UID:routine:r1-1430@oldbrainfix',
        'DTSTAMP:20261024T070509Z',
        'DTSTART:20261103T143000',
        'DURATION:PT15M',
        'SUMMARY:Dentist',
        'BEGIN:VALARM',
        'ACTION:DISPLAY',
        'DESCRIPTION:Dentist',
        'TRIGGER:PT0M',
        'END:VALARM',
        'END:VEVENT',
      ]);
    });

    test('an all-day Reminder is a date with its alarm at 09:00', () {
      final ics = writeIcs(
        routineEvents(
          routine(schedule(null, start: Day(2026, 12, 31))),
          'Party',
        ),
        stamp: stamp,
      );
      final lines = event(ics, 'routine:r1@oldbrainfix');
      expect(lines, contains('DTSTART;VALUE=DATE:20261231'));
      expect(lines, contains('DTEND;VALUE=DATE:20270101'));
      expect(lines, contains('TRIGGER:PT540M'));
      expect(lines.where((l) => l.startsWith('RRULE')), isEmpty);
    });

    test('several lines: the first is the summary, all is the description', () {
      final events = routineEvents(
        routine(schedule(null, times: [at(8)])),
        '  Call the bank\nabout the card  ',
      );
      expect(events.single.summary, 'Call the bank');
      expect(events.single.description, 'Call the bank\nabout the card');
    });
  });

  group('Routines', () {
    test('a Routine carries its RRULE', () {
      final events = routineEvents(
        routine(
          schedule(
            'FREQ=WEEKLY;BYDAY=MO,WE',
            start: Day(2026, 10, 26),
            times: [at(7)],
          ),
        ),
        'Bins',
      );
      expect(events.single.rule, 'FREQ=WEEKLY;BYDAY=MO,WE');
      final lines = event(
        writeIcs(events, stamp: stamp),
        'routine:r1-0700@oldbrainfix',
      );
      expect(lines, contains('DTSTART:20261026T070000'));
      expect(lines, contains('RRULE:FREQ=WEEKLY;BYDAY=MO,WE'));
    });

    test('DTSTART is the first Occurrence, not a start the rule skips', () {
      // Saturday the 24th; the first Monday is the 26th.
      final events = routineEvents(
        routine(schedule('FREQ=WEEKLY;BYDAY=MO', times: [at(7)])),
        'Bins',
      );
      expect(events.single.start, Occurrence(Day(2026, 10, 26), at(7)));
    });

    test('several times per day: one event each with the same rule', () {
      final events = routineEvents(
        routine(schedule('FREQ=DAILY', times: [at(20), at(8), at(13, 15)])),
        'Pills',
      );
      expect(
        [for (final e in events) e.uid],
        [
          'routine:r1-0800@oldbrainfix',
          'routine:r1-1315@oldbrainfix',
          'routine:r1-2000@oldbrainfix',
        ],
      );
      expect({for (final e in events) e.rule}, {'FREQ=DAILY'});
      final ics = writeIcs(events, stamp: stamp);
      expect(
        event(ics, 'routine:r1-1315@oldbrainfix'),
        contains('DTSTART:20261024T131500'),
      );
      expect('BEGIN:VEVENT'.allMatches(ics), hasLength(3));
    });

    test('times from BYHOUR become events, the rule loses them', () {
      final events = routineEvents(
        routine(schedule('FREQ=DAILY;BYHOUR=8,20;BYMINUTE=30')),
        'Pills',
      );
      expect(
        [for (final e in events) e.start?.key],
        ['2026-10-24T08:30', '2026-10-24T20:30'],
      );
      expect({for (final e in events) e.rule}, {'FREQ=DAILY'});
    });

    test('COUNT over several times becomes the last Occurrence as UNTIL', () {
      // 08:00 and 20:00, five in all: the last is the 26th at 08:00.
      final events = routineEvents(
        routine(schedule('FREQ=DAILY;COUNT=5', times: [at(8), at(20)])),
        'Antibiotics',
      );
      expect(
        {for (final e in events) e.rule},
        {'FREQ=DAILY;UNTIL=20261026T080000'},
      );
    });

    test('COUNT with one time stays COUNT', () {
      final events = routineEvents(
        routine(schedule('FREQ=WEEKLY;COUNT=4', times: [at(8)])),
        'Physio',
      );
      expect(events.single.rule, 'FREQ=WEEKLY;COUNT=4');
    });

    test('a time COUNT cuts off on the first day gets no event', () {
      final events = routineEvents(
        routine(schedule('FREQ=DAILY;COUNT=1', times: [at(8), at(20)])),
        'Once really',
      );
      expect([for (final e in events) e.start?.key], ['2026-10-24T08:00']);
    });

    test('a date UNTIL on a timed event becomes the end of that day', () {
      final events = routineEvents(
        routine(schedule('FREQ=DAILY;UNTIL=20261031', times: [at(8)])),
        'Pills',
      );
      expect(events.single.rule, 'FREQ=DAILY;UNTIL=20261031T235900');
    });

    test('UNTIL on an all-day event is a date', () {
      final events = routineEvents(
        routine(schedule('FREQ=DAILY;UNTIL=20261031T120000Z')),
        'Water plants',
      );
      expect(events.single.rule, 'FREQ=DAILY;UNTIL=20261031');
      final lines = event(
        writeIcs(events, stamp: stamp),
        'routine:r1@oldbrainfix',
      );
      expect(lines, contains('DTSTART;VALUE=DATE:20261024'));
      expect(lines, contains('RRULE:FREQ=DAILY;UNTIL=20261031'));
      expect(lines, contains('TRIGGER:PT540M'));
    });

    test('ordinals, month days and the week start are kept', () {
      final events = routineEvents(
        routine(
          schedule('FREQ=MONTHLY;INTERVAL=2;BYDAY=1MO;WKST=SU', times: [at(9)]),
        ),
        'Bills',
      );
      expect(events.single.rule, 'FREQ=MONTHLY;INTERVAL=2;BYDAY=1MO;WKST=SU');
      expect(events.single.start, Occurrence(Day(2026, 12, 7), at(9)));
    });

    test('a schedule that never happens has no events', () {
      final events = routineEvents(
        routine(schedule('FREQ=DAILY;UNTIL=20261001', times: [at(8)])),
        'Gone',
      );
      expect(events, isEmpty);
    });

    test('the same Routine gives the same UIDs every time', () {
      List<String> uids() => [
        for (final e in routineEvents(
          routine(schedule('FREQ=DAILY', times: [at(8), at(20)])),
          'Pills',
        ))
          e.uid,
      ];
      expect(uids(), uids());
    });
  });

  group('a Note on its own', () {
    test('has no start and no rule', () {
      final event = CalendarEvent.ofText('note:n1', 'Ring the plumber');
      expect(event.start, isNull);
      expect(event.rule, isNull);
      expect(event.summary, 'Ring the plumber');
      expect(event.description, '');
    });
  });
}
