/// A Routine's history (CONTEXT.md): each Occurrence Done, Missed or
/// still open, its Streak and its Adherence, as a pure function of the
/// schedules it had, what was Done, this Device's snoozes, now and the
/// wall clock. Missed follows the same rules as the Missed Summary.
library;

import 'alarms.dart';
import 'entry.dart';
import 'routines.dart';
import 'schedule.dart';
import 'store.dart';

/// A stretch of time in which a Routine had [schedule] and was out of
/// the trash: what fell due in it was asked for.
class ScheduleSpan {
  final Schedule schedule;

  /// When the schedule was written, or the Routine left the trash.
  final DateTime from;

  /// When the schedule was changed or the Routine went to the trash;
  /// null while it still holds.
  final DateTime? until;

  const ScheduleSpan({required this.schedule, required this.from, this.until});
}

/// The [ScheduleSpan]s a Routine's log [entries] tell, oldest first.
/// Saving a schedule writes its fields one after the other, a moment
/// apart: a state that stood for less than a minute never stood, as no
/// Occurrence can fall in it.
List<ScheduleSpan> scheduleSpans(List<Entry> entries) {
  final changes = [
    for (final entry in entries)
      if (RoutineKeys.scheduleFields.contains(entry.field) ||
          entry.field == RoutineKeys.trashed)
        entry,
  ]..sort(Store.latestWins);
  final fields = <String, String?>{};
  final states = <(DateTime, Schedule?)>[];
  for (final entry in changes) {
    fields[entry.field] = entry.value;
    final trashed = fields[RoutineKeys.trashed] != null;
    states.add((entry.date, trashed ? null : Routines.scheduleOf(fields)));
  }
  const settled = Duration(minutes: 1);
  return [
    for (var i = 0; i < states.length; i++)
      if (states[i] case (final from, final schedule?))
        if (i + 1 == states.length)
          ScheduleSpan(schedule: schedule, from: from)
        else if (states[i + 1].$1.difference(from) >= settled)
          ScheduleSpan(schedule: schedule, from: from, until: states[i + 1].$1),
  ];
}

/// Where an Occurrence of a Routine stands.
enum OccurrenceStatus {
  /// Checked off.
  done,

  /// Never Done, and its time is over (CONTEXT.md).
  missed,

  /// Due, not Done, not Missed yet.
  open,

  /// Not due yet.
  coming,
}

/// When a Done came, against the Occurrence's time.
enum DoneTiming {
  /// Before the Occurrence started.
  early,

  /// While it was due.
  onTime,

  /// After it was Missed: checked off late.
  late,
}

/// One Occurrence in a Routine's history.
class HistoryEntry {
  final Occurrence occurrence;
  final OccurrenceStatus status;

  /// When it starts, as an instant.
  final DateTime start;

  /// When it counts as Missed unless Done, as an instant.
  final DateTime missedAt;

  /// When it was checked off; null unless Done.
  final DateTime? doneAt;

  /// Until when it was last snoozed on this Device, if this Device still
  /// knows; snoozes are never synced (ADR 0008).
  final DateTime? snoozedUntil;

  const HistoryEntry({
    required this.occurrence,
    required this.status,
    required this.start,
    required this.missedAt,
    this.doneAt,
    this.snoozedUntil,
  });

  /// Whether it was Done early, on time or late; null unless Done.
  DoneTiming? get timing {
    final done = doneAt;
    if (done == null) return null;
    if (done.isBefore(start)) return DoneTiming.early;
    return done.isBefore(missedAt) ? DoneTiming.onTime : DoneTiming.late;
  }
}

/// The Done and Missed Occurrences over a period; those still open do
/// not count.
class Adherence {
  final int done;
  final int missed;

  const Adherence({required this.done, required this.missed});

  /// The share Done, from 0 to 1; null when nothing counts yet.
  double? get share => done + missed == 0 ? null : done / (done + missed);
}

/// What one day of a Routine came to.
class DayTally {
  final Day day;
  final int done;
  final int missed;

  /// Not Done and not Missed yet.
  final int open;

  const DayTally({
    required this.day,
    required this.done,
    required this.missed,
    required this.open,
  });

  /// Whether nothing was due that day.
  bool get empty => done + missed + open == 0;
}

/// A Routine's history up to the end of today.
class RoutineHistory {
  /// Every Occurrence that was asked for or Done, oldest first. One that
  /// fell due while the Routine was in the trash, or under a schedule
  /// changed before it was Missed, was never asked for and is left out,
  /// unless it was Done.
  final List<HistoryEntry> entries;
  final Day today;

  /// The day weeks start on, `DateTime.monday` to `DateTime.sunday`.
  final int weekStart;

  /// Works out the Streaks, the last seven days and the Adherence
  /// figures once, up front.
  RoutineHistory({
    required this.entries,
    required this.today,
    this.weekStart = DateTime.monday,
  }) {
    currentStreak = _currentStreak();
    longestStreak = _longestStreak();
    lastSevenDays = [for (var i = 6; i >= 0; i--) _tally(today.addDays(-i))];
    thisWeek = adherence(
      today.addDays(-((today.weekday - weekStart) % 7)),
      today,
    );
    thisMonth = adherence(Day(today.year, today.month, 1), today);
    last30Days = adherence(today.addDays(-29), today);
  }

  /// The [entries] from [from] to [to], both days included.
  List<HistoryEntry> between(Day from, Day to) => [
    for (final entry in entries)
      if (!(entry.occurrence.day < from) && entry.occurrence.day <= to) entry,
  ];

  /// The run of Done Occurrences up to the latest that is no longer
  /// open (CONTEXT.md, Streak): one still due does not break it.
  late final int currentStreak;

  int _currentStreak() {
    var run = 0;
    for (final entry in entries.reversed) {
      switch (entry.status) {
        case OccurrenceStatus.done:
          run++;
        case OccurrenceStatus.missed:
          return run;
        case OccurrenceStatus.open || OccurrenceStatus.coming:
          continue;
      }
    }
    return run;
  }

  /// The longest run of Done Occurrences ever.
  late final int longestStreak;

  int _longestStreak() {
    var run = 0;
    var longest = 0;
    for (final entry in entries) {
      switch (entry.status) {
        case OccurrenceStatus.done:
          run++;
          if (run > longest) longest = run;
        case OccurrenceStatus.missed:
          run = 0;
        case OccurrenceStatus.open || OccurrenceStatus.coming:
          continue;
      }
    }
    return longest;
  }

  /// The last seven days, today last.
  late final List<DayTally> lastSevenDays;

  DayTally _tally(Day day) {
    var done = 0;
    var missed = 0;
    var open = 0;
    for (final entry in between(day, day)) {
      switch (entry.status) {
        case OccurrenceStatus.done:
          done++;
        case OccurrenceStatus.missed:
          missed++;
        case OccurrenceStatus.open || OccurrenceStatus.coming:
          open++;
      }
    }
    return DayTally(day: day, done: done, missed: missed, open: open);
  }

  /// The Adherence from [from] to [to], both days included.
  Adherence adherence(Day from, Day to) {
    var done = 0;
    var missed = 0;
    for (final entry in between(from, to)) {
      if (entry.status == OccurrenceStatus.done) done++;
      if (entry.status == OccurrenceStatus.missed) missed++;
    }
    return Adherence(done: done, missed: missed);
  }

  /// Since the week began, on [weekStart].
  late final Adherence thisWeek;

  /// Since the 1st of the month.
  late final Adherence thisMonth;

  /// The last 30 days, today included.
  late final Adherence last30Days;
}

/// The history at [now] of a Routine that had the [spans] (see
/// [scheduleSpans]), was Done at [done] and was snoozed on this Device
/// until [snoozes], both by [Occurrence.key], with days as [wallClock]
/// shows them. An Occurrence is Missed as the Missed Summary says: when
/// the next one starts, unless the schedule changed or the Routine went
/// to the trash before. Whatever was Done counts, asked for or not.
RoutineHistory routineHistory({
  required List<ScheduleSpan> spans,
  required Map<String, DateTime> done,
  Map<String, DateTime> snoozes = const {},
  required DateTime now,
  required WallClock wallClock,
  int weekStart = DateTime.monday,
}) {
  final today = Day.today(now, wallClock);
  final asked = <String, HistoryEntry>{};
  // Done, but outside every span: kept unless asked for elsewhere.
  final doneAside = <String, HistoryEntry>{};
  HistoryEntry entry(
    Occurrence occurrence,
    OccurrenceStatus status,
    DateTime start,
    DateTime missedAt,
  ) => HistoryEntry(
    occurrence: occurrence,
    status: status,
    start: start,
    missedAt: missedAt,
    doneAt: done[occurrence.key],
    snoozedUntil: snoozes[occurrence.key],
  );

  for (final span in spans) {
    final schedule = span.schedule;
    final until = span.until;
    final from = Day.today(span.from, wallClock);
    final to = until == null ? today : Day.today(until, wallClock);
    if (to < from) continue;
    final occurrences = schedule.between(from, to);
    final after = schedule.once ? null : schedule.next(to.addDays(1), 1);
    for (var i = 0; i < occurrences.length; i++) {
      final occurrence = occurrences[i];
      if (occurrence.day > today) break;
      final next = i + 1 < occurrences.length
          ? occurrences[i + 1]
          : after?.firstOrNull;
      final wallStart = startOf(occurrence);
      final wallEnds = missedFrom(occurrence, next);
      final start = instantOf(wallStart.day, wallStart.time, wallClock);
      final missedAt = instantOf(wallEnds.day, wallEnds.time, wallClock);
      final inSpan =
          !start.isBefore(span.from) &&
          (until == null || start.isBefore(until));
      final key = occurrence.key;
      if (done[key] != null) {
        final e = entry(occurrence, OccurrenceStatus.done, start, missedAt);
        if (inSpan) {
          asked[key] = e;
        } else {
          doneAside.putIfAbsent(key, () => e);
        }
        continue;
      }
      if (!inSpan) continue;
      if (!missedAt.isAfter(now) &&
          (until == null || !missedAt.isAfter(until))) {
        asked[key] = entry(
          occurrence,
          OccurrenceStatus.missed,
          start,
          missedAt,
        );
      } else if (until == null) {
        asked[key] = entry(
          occurrence,
          start.isAfter(now) ? OccurrenceStatus.coming : OccurrenceStatus.open,
          start,
          missedAt,
        );
      }
    }
  }
  for (final MapEntry(:key, :value) in doneAside.entries) {
    asked.putIfAbsent(key, () => value);
  }
  // Done under a schedule no span shows, as from another Device.
  for (final key in done.keys) {
    if (asked.containsKey(key)) continue;
    final occurrence = Occurrence.tryParse(key);
    if (occurrence == null || occurrence.day > today) continue;
    final wallStart = startOf(occurrence);
    final wallEnds = missedFrom(occurrence, null);
    asked[key] = entry(
      occurrence,
      OccurrenceStatus.done,
      instantOf(wallStart.day, wallStart.time, wallClock),
      instantOf(wallEnds.day, wallEnds.time, wallClock),
    );
  }
  final entries = asked.values.toList()
    ..sort((a, b) => a.occurrence.compareTo(b.occurrence));
  return RoutineHistory(entries: entries, today: today, weekStart: weekStart);
}

/// The history of Routines in the store.
extension RoutineHistories on Routines {
  /// The history of [routine] at the store's now, on [wallClock], with
  /// the snoozes this Device knows of.
  RoutineHistory history(
    Routine routine,
    WallClock wallClock, {
    int weekStart = DateTime.monday,
  }) {
    final prefix = '${routine.id}@';
    return routineHistory(
      spans: scheduleSpans(store.history(routine.id)),
      done: routine.done,
      snoozes: {
        // Keyed by snoozeKey: the Routine's id, then the Occurrence's.
        for (final MapEntry(:key, :value) in Alarms(store).snoozes.entries)
          if (key.startsWith(prefix)) key.substring(prefix.length): value,
      },
      now: store.clock(),
      wallClock: wallClock,
      weekStart: weekStart,
    );
  }
}
