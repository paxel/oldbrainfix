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
  final times = schedule.dayTimes;
  return [
    repeatText(t, locale, schedule),
    times.isEmpty
        ? t.allDay
        : [for (final time in times) clockText(locale, time)].join(', '),
  ].join(' · ');
}

/// The full name of [weekday], `DateTime.monday` to `DateTime.sunday`.
String weekdayName(String locale, int weekday) =>
    DateFormat.EEEE(locale).format(DateTime(2026, 10, 25 + weekday));

/// How [schedule] repeats, in words, without its times, with its end.
String repeatText(AppLocalizations t, String locale, Schedule schedule) {
  final rule = schedule.rule;
  final simple = SimpleRepeat.of(rule, schedule.start);
  if (simple == null) return t.ruleSummary(rule?.format() ?? '');
  final repeat = switch (simple.kind) {
    RepeatKind.once => t.onceOn(dayText(locale, schedule.start)),
    RepeatKind.every => switch (simple.unit) {
      RepeatUnit.days => t.everyNDays(simple.every),
      RepeatUnit.weeks => t.everyNWeeks(simple.every),
      RepeatUnit.months => t.everyNMonths(simple.every),
      RepeatUnit.years => t.everyNYears(simple.every),
    },
    RepeatKind.weekdays => t.onWeekdays(
      [for (final d in simple.weekdays.toList()..sort()) weekdayText(locale, d)]
          .join(', '),
    ),
    RepeatKind.monthDay => t.everyMonthOnDay(
      simple.every,
      simple.monthDay ?? schedule.start.day,
    ),
    RepeatKind.monthWeekday => t.everyMonthOnNth(
      simple.every,
      // `1` to `4`; anything else is the last.
      t.nthWord('${simple.nth?.ordinal ?? -1}'),
      weekdayName(locale, simple.nth?.weekday ?? schedule.start.weekday),
    ),
  };
  if (simple.count case final count?) return t.repeatCount(count, repeat);
  if (simple.until case final until?) {
    return t.repeatUntil(repeat, dayText(locale, until));
  }
  return repeat;
}
