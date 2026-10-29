/// Routines and Reminders as calendar events: the iCalendar file (RFC
/// 5545) for an export of all of them or to share one, and the same
/// events one by one for a calendar app's insert screen (adapted from
/// Catlog's ics.dart). The desktop core writes the same file byte for
/// byte (`ics.rs`).
///
/// Times are floating: the Device's wall clock without a zone, as the
/// schedule itself is (ADR 0006). An 08:00 Routine is 08:00 wherever the
/// calendar is read, as the app's own alarm is. A TZID would need a
/// VTIMEZONE with daylight-saving rules the core does not have.
library;

import 'alarms.dart';
import 'routines.dart';
import 'schedule.dart';

/// One calendar event: a Reminder, or one time of day of a Routine.
class CalendarEvent {
  /// How long a timed event lasts in the calendar.
  static const length = Duration(minutes: 15);

  /// Stable per Routine and time of day, so importing an updated export
  /// replaces events instead of doubling them.
  final String uid;

  /// One line: what shows in the calendar's grid.
  final String summary;

  /// The full text; empty when [summary] already says it all.
  final String description;

  /// The first Occurrence; null for an event without a time, whose time
  /// the calendar app proposes.
  final Occurrence? start;

  /// The RRULE value for the days after [start], in step with it;
  /// null for one time only.
  final String? rule;

  const CalendarEvent({
    required this.uid,
    required this.summary,
    this.description = '',
    this.start,
    this.rule,
  });

  /// An event for [text] at no particular time: a Note on its own.
  factory CalendarEvent.ofText(String uid, String text) {
    final (summary, description) = _split(text);
    return CalendarEvent(uid: uid, summary: summary, description: description);
  }
}

/// The events of [routine], whose Note reads [text]: one per time of day,
/// each repeating on the Routine's days, or one all-day event; none when
/// the schedule has no Occurrence at all.
List<CalendarEvent> routineEvents(Routine routine, String text) {
  final schedule = routine.schedule;
  final first = schedule.next(schedule.start, 1).firstOrNull;
  if (first == null) return const [];
  final (summary, description) = _split(text);
  final times = schedule.dayTimes;
  final rule = schedule.rule;
  String? ruleFor(ClockTime? time) {
    if (rule == null) return null;
    var count = rule.count;
    var until = rule.until;
    if (count != null && times.length > 1) {
      // COUNT counts every time of day; per event the same last
      // Occurrence as UNTIL says it once for all of them.
      final last = schedule.next(first.day, count).last;
      count = null;
      until = (day: last.day, time: last.time);
    }
    if (until != null) {
      // UNTIL has DTSTART's type: a date for all day, a time otherwise.
      until = time == null
          ? (day: until.day, time: null)
          : (day: until.day, time: until.time ?? ClockTime(23, 59));
    }
    return Recurrence(
      frequency: rule.frequency,
      interval: rule.interval,
      byDay: rule.byDay,
      byMonthDay: rule.byMonthDay,
      byMonth: rule.byMonth,
      count: count,
      until: until,
      weekStart: rule.weekStart,
    ).format();
  }

  // A time cut off on the first day by COUNT or UNTIL never comes; the
  // others start there, as DTSTART always counts as an Occurrence.
  return [
    for (final start in schedule.on(first.day))
      CalendarEvent(
        uid: switch (start.time) {
          null => '${routine.id}@oldbrainfix',
          final time =>
            '${routine.id}-${_two(time.hour)}${_two(time.minute)}@oldbrainfix',
        },
        summary: summary,
        description: description,
        start: start,
        rule: ruleFor(start.time),
      ),
  ];
}

/// [routines] as one iCalendar file, every event of each in turn, each
/// titled by [textOf]: the export of all of them, or one Routine with
/// its whole series and its end, or one Reminder, to share.
String routinesIcs(
  Iterable<Routine> routines,
  String Function(Routine routine) textOf, {
  required DateTime stamp,
}) => writeIcs([
  for (final routine in routines) ...routineEvents(routine, textOf(routine)),
], stamp: stamp);

/// [events] as an iCalendar file (RFC 5545), each with an alarm when it
/// starts (an all-day one at [allDayAlarmTime], as the app rings it).
/// Events without a start are left out. [stamp] is when it was made.
String writeIcs(Iterable<CalendarEvent> events, {required DateTime stamp}) {
  final lines = <String>[
    'BEGIN:VCALENDAR',
    'VERSION:2.0',
    'PRODID:-//Old Brain Fix//Routines//EN',
    'CALSCALE:GREGORIAN',
    'METHOD:PUBLISH',
  ];
  final dtstamp = _utcStamp(stamp);
  for (final event in events) {
    final start = event.start;
    if (start == null) continue;
    lines
      ..add('BEGIN:VEVENT')
      ..add('UID:${_escape(event.uid)}')
      ..add('DTSTAMP:$dtstamp');
    switch (start.time) {
      case null:
        lines
          ..add('DTSTART;VALUE=DATE:${_date(start.day)}')
          ..add('DTEND;VALUE=DATE:${_date(start.day.addDays(1))}');
      case final time:
        lines
          ..add('DTSTART:${_date(start.day)}T${_clock(time)}')
          ..add('DURATION:PT${CalendarEvent.length.inMinutes}M');
    }
    if (event.rule case final rule?) lines.add('RRULE:$rule');
    lines.add('SUMMARY:${_escape(event.summary)}');
    if (event.description.isNotEmpty) {
      lines.add('DESCRIPTION:${_escape(event.description)}');
    }
    final trigger = start.time == null
        ? 'PT${allDayAlarmTime.hour * 60 + allDayAlarmTime.minute}M'
        : 'PT0M';
    lines
      ..add('BEGIN:VALARM')
      ..add('ACTION:DISPLAY')
      ..add('DESCRIPTION:${_escape(event.summary)}')
      ..add('TRIGGER:$trigger')
      ..add('END:VALARM')
      ..add('END:VEVENT');
  }
  lines.add('END:VCALENDAR');
  return [for (final line in lines) _fold(line)].join();
}

/// The first line of [text] as the summary, and the whole text as the
/// description when there is more than that line.
(String, String) _split(String text) {
  final trimmed = text.trim();
  final lines = trimmed.split(RegExp(r'\r?\n'));
  final summary = lines.first.trim();
  return (summary, lines.length > 1 ? trimmed : '');
}

String _two(int n) => n.toString().padLeft(2, '0');

String _date(Day day) =>
    '${day.year.toString().padLeft(4, '0')}${_two(day.month)}${_two(day.day)}';

String _clock(ClockTime time) => '${_two(time.hour)}${_two(time.minute)}00';

String _utcStamp(DateTime instant) {
  final u = instant.toUtc();
  return '${_date(Day(u.year, u.month, u.day))}T'
      '${_two(u.hour)}${_two(u.minute)}${_two(u.second)}Z';
}

/// TEXT escaping (RFC 5545, 3.3.11).
String _escape(String text) => text
    .replaceAll('\\', '\\\\')
    .replaceAll(';', '\\;')
    .replaceAll(',', '\\,')
    .replaceAll(RegExp(r'\r\n|\r|\n'), '\\n');

/// [line] with its CRLF, folded so no physical line is longer than 75
/// octets (RFC 5545, 3.1); a continuation starts with a space, and no
/// UTF-8 sequence is cut.
String _fold(String line) {
  const max = 75;
  final out = StringBuffer();
  var octets = 0;
  for (final rune in line.runes) {
    final size = _utf8Length(rune);
    if (octets + size > max) {
      out.write('\r\n ');
      octets = 1;
    }
    out.writeCharCode(rune);
    octets += size;
  }
  out.write('\r\n');
  return out.toString();
}

int _utf8Length(int rune) => rune < 0x80
    ? 1
    : rune < 0x800
    ? 2
    : rune < 0x10000
    ? 3
    : 4;
