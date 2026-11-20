import '../l10n/app_localizations.dart';

/// The Heads-ups the Routine editor offers (obf-1046), in minutes before
/// the start: for a timed schedule 5, 10, 15 and 30 min, 1 h, 2 h, 1 day,
/// 2 days and 1 week.
const timedHeadsUps = [5, 10, 15, 30, 60, 120, 1440, 2880, 10080];

/// Those for an all-day schedule, which takes whole days only: 1, 2 and
/// 3 days and 1 week.
const allDayHeadsUps = [1440, 2880, 4320, 10080];

/// Whether [minutes] is a whole number of days, the only Heads-ups an
/// all-day Occurrence takes.
bool wholeDays(int minutes) => minutes % 1440 == 0;

/// "20 min before", "1 day before": a Heads-up as the editor lists it.
String headsUpBefore(AppLocalizations t, int minutes) => switch (_span(
  minutes,
)) {
  (_Unit.weeks, final n, _) => t.headsUpWeeksBefore(n),
  (_Unit.days, final n, _) => t.headsUpDaysBefore(n),
  (_Unit.hours, final n, _) => t.headsUpHoursBefore(n),
  (_Unit.hoursMinutes, final h, final m) => t.headsUpHoursMinutesBefore(h, m),
  (_Unit.minutes, final n, _) => t.headsUpMinutesBefore(n),
};

/// "in 20 min", "in 1 day": how long until the Occurrence starts, as its
/// Heads-up's notification says it.
String headsUpIn(AppLocalizations t, int minutes) => switch (_span(minutes)) {
  (_Unit.weeks, final n, _) => t.headsUpInWeeks(n),
  (_Unit.days, final n, _) => t.headsUpInDays(n),
  (_Unit.hours, final n, _) => t.headsUpInHours(n),
  (_Unit.hoursMinutes, final h, final m) => t.headsUpInHoursMinutes(h, m),
  (_Unit.minutes, final n, _) => t.headsUpInMinutes(n),
};

enum _Unit { weeks, days, hours, hoursMinutes, minutes }

/// [minutes] in the largest unit it is a whole number of; past an hour
/// and not a whole one, in hours and minutes.
(_Unit, int, int) _span(int minutes) {
  if (minutes % 10080 == 0) return (_Unit.weeks, minutes ~/ 10080, 0);
  if (minutes % 1440 == 0) return (_Unit.days, minutes ~/ 1440, 0);
  if (minutes % 60 == 0) return (_Unit.hours, minutes ~/ 60, 0);
  if (minutes > 60) return (_Unit.hoursMinutes, minutes ~/ 60, minutes % 60);
  return (_Unit.minutes, minutes, 0);
}
