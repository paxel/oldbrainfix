import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

/// [time] as the reader's clock shows it.
String clockText(String locale, ClockTime time) =>
    DateFormat.Hm(locale).format(DateTime(2000, 1, 1, time.hour, time.minute));

/// [day] as a short date with its weekday.
String dayText(String locale, Day day) =>
    DateFormat.yMEd(locale).format(day.wall);

/// The short name of [weekday], `DateTime.monday` to `DateTime.sunday`.
String weekdayText(String locale, int weekday) =>
    // 2026-10-26 is a Monday.
    DateFormat.E(locale).format(DateTime(2026, 10, 25 + weekday));

/// When an Occurrence is due: its time, or "All day".
String occurrenceTime(AppLocalizations t, String locale, Occurrence o) =>
    switch (o.time) {
      null => t.allDay,
      final time => clockText(locale, time),
    };

/// [schedule] in words: how it repeats, and at which times.
String scheduleText(AppLocalizations t, String locale, Schedule schedule) {
  final rule = schedule.rule;
  final simple = SimpleRepeat.of(rule, schedule.start);
  final repeat = switch (simple?.kind) {
    RepeatKind.once => t.onceOn(dayText(locale, schedule.start)),
    RepeatKind.every => switch (simple?.unit) {
      RepeatUnit.weeks => t.everyNWeeks(simple?.every ?? 1),
      RepeatUnit.months => t.everyNMonths(simple?.every ?? 1),
      RepeatUnit.years => t.everyNYears(simple?.every ?? 1),
      _ => t.everyNDays(simple?.every ?? 1),
    },
    RepeatKind.weekdays => t.onWeekdays(
      [
        for (final d in (simple?.weekdays.toList() ?? <int>[])..sort())
          weekdayText(locale, d),
      ].join(', '),
    ),
    null => t.ruleSummary(rule?.format() ?? ''),
  };
  final times = schedule.dayTimes;
  return [
    repeat,
    times.isEmpty
        ? t.allDay
        : [for (final time in times) clockText(locale, time)].join(', '),
  ].join(' · ');
}
