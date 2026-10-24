/// The alarm plan (ADR 0004): which alarms should exist on this Device,
/// as a pure function of the Routines, now, the wall clock and this
/// Device's snoozes. The platform's alarm layer only arms what it is
/// handed.
library;

import 'dart:convert';

import 'device_settings.dart';
import 'notes.dart';
import 'routines.dart';
import 'schedule.dart';
import 'store.dart';

/// When an all-day Occurrence's alarm fires, on the wall clock.
final allDayAlarmTime = ClockTime(9, 0);

/// How many days ahead the plan looks. The platform keeps the whole
/// plan and arms one alarm after the other, so alarms keep coming for
/// this long even when the app is never opened.
const alarmHorizonDays = 30;

/// At most this many alarms per Routine, so a Routine due every few
/// minutes cannot crowd out the others.
const alarmsPerRoutine = 128;

/// At most this many alarms in all, the soonest first.
const alarmLimit = 2000;

/// After how many ignored Re-nags Escalation starts when it is turned on
/// for a Routine.
const defaultEscalation = 3;

/// The instant [day] at [time] is on [wallClock]. A time the clock shows
/// twice (when daylight saving ends) fires once, at the first; a time it
/// skips (when daylight saving starts) fires at the next valid instant,
/// the moment the clock jumps past it.
DateTime instantOf(Day day, ClockTime time, WallClock wallClock) {
  final target = DateTime.utc(
    day.year,
    day.month,
    day.day,
    time.hour,
    time.minute,
  );
  DateTime wallOf(DateTime utc) {
    final wall = wallClock(utc.toUtc());
    return DateTime.utc(
      wall.year,
      wall.month,
      wall.day,
      wall.hour,
      wall.minute,
      wall.second,
      wall.millisecond,
    );
  }

  Duration offsetAt(DateTime utc) => wallOf(utc).difference(utc);
  // The offsets a day before and a day after: equal on most days, the
  // two sides of a switch on the day of one.
  const day1 = Duration(days: 1);
  final before = offsetAt(target.subtract(day1));
  final after = offsetAt(target.add(day1));
  final hits = {
    for (final offset in [before, after]) target.subtract(offset),
  }.where((t) => wallOf(t) == target).toList()..sort();
  if (hits.isNotEmpty) return hits.first;
  // Skipped: find where the clock jumps, between the instant that still
  // shows an earlier time and the one that already shows a later one.
  var lo = target.subtract(after).millisecondsSinceEpoch;
  var hi = target.subtract(before).millisecondsSinceEpoch;
  if (lo >= hi) return DateTime.fromMillisecondsSinceEpoch(hi, isUtc: true);
  while (hi - lo > 1) {
    final mid = lo + (hi - lo) ~/ 2;
    final shown = wallOf(DateTime.fromMillisecondsSinceEpoch(mid, isUtc: true));
    if (shown.isBefore(target)) {
      lo = mid;
    } else {
      hi = mid;
    }
  }
  return DateTime.fromMillisecondsSinceEpoch(hi, isUtc: true);
}

/// A moment on the wall clock.
typedef WallMoment = ({Day day, ClockTime time});

/// When [occurrence] starts: at its time, an all-day one at
/// [allDayAlarmTime].
WallMoment startOf(Occurrence occurrence) =>
    (day: occurrence.day, time: occurrence.time ?? allDayAlarmTime);

/// When [occurrence] counts as Missed unless it is Done (CONTEXT.md):
/// when [next], the Occurrence after it, starts (an all-day one at the
/// start of its day, so it never nags through the night); at midnight
/// after its day when there is no next one, as for a Reminder.
WallMoment missedFrom(Occurrence occurrence, Occurrence? next) => next == null
    ? (day: occurrence.day.addDays(1), time: ClockTime(0, 0))
    : (day: next.day, time: next.time ?? ClockTime(0, 0));

/// One Occurrence of a Routine placed in time: when it starts and when
/// it counts as Missed.
class _Timed {
  final Occurrence occurrence;
  final DateTime start;
  final WallMoment ends;
  final DateTime endsAt;

  _Timed(this.occurrence, this.start, this.ends, this.endsAt);
}

/// The Occurrences of [routine] from [from] to [to] that were asked for
/// (due at or after [Routine.scheduledAt]), placed on [wallClock].
List<_Timed> _timed(Routine routine, Day from, Day to, WallClock wallClock) {
  final schedule = routine.schedule;
  final occurrences = schedule.between(from, to);
  final after = schedule.once ? null : schedule.next(to.addDays(1), 1);
  final since = routine.scheduledAt;
  final timed = <_Timed>[];
  for (var i = 0; i < occurrences.length; i++) {
    final next = i + 1 < occurrences.length
        ? occurrences[i + 1]
        : after?.firstOrNull;
    final t = _place(occurrences[i], next, wallClock);
    if (since == null || !t.start.isBefore(since)) timed.add(t);
  }
  return timed;
}

_Timed _place(Occurrence occurrence, Occurrence? next, WallClock wallClock) {
  final start = startOf(occurrence);
  final ends = missedFrom(occurrence, next);
  return _Timed(
    occurrence,
    instantOf(start.day, start.time, wallClock),
    ends,
    instantOf(ends.day, ends.time, wallClock),
  );
}

/// Why an alarm fires.
enum AlarmKind {
  /// An Occurrence is due.
  occurrence,

  /// An Occurrence snoozed from its notification is due again.
  snooze,
}

/// One alarm that should exist. While its Occurrence is not Done it
/// fires at [at] and then Re-nags every [renag], up to [endsAt], when
/// the Occurrence counts as Missed. With [escalation] on, every firing
/// from [escalatesAt] on is a full-screen alarm with sound.
class PlannedAlarm {
  final AlarmKind kind;
  final String routineId;
  final Occurrence occurrence;

  /// When it fires first, in UTC; past for an Occurrence still Re-nagging.
  final DateTime at;

  /// The wall-clock moment an Occurrence alarm fires at, so the platform
  /// re-arms it at the same local time after a time-zone change; null
  /// for a snooze, which waits a fixed time.
  final WallMoment? wall;

  /// How often it fires again while ignored.
  final Duration renag;

  /// After how many ignored Re-nags it becomes a full-screen alarm; null
  /// when its Routine does not escalate.
  final int? escalation;

  /// When the Occurrence counts as Missed and the alarm stops, on the
  /// wall clock and as the instant [endsAt].
  final WallMoment ends;
  final DateTime endsAt;

  /// The text of the Note the Routine links back to; null when it is
  /// gone.
  final String? text;

  const PlannedAlarm({
    required this.kind,
    required this.routineId,
    required this.occurrence,
    required this.at,
    required this.wall,
    required this.renag,
    this.escalation,
    required this.ends,
    required this.endsAt,
    required this.text,
  });

  /// Names the alarm for good: one per kind and Occurrence.
  String get id => '${kind.name}:${snoozeKey(routineId, occurrence)}';

  /// When it fires first after [after]: at [at], or at a Re-nag; null
  /// when it has stopped by then. The platform's alarm layer follows the
  /// same rule.
  DateTime? firesAfter(DateTime after) {
    if (at.isAfter(after)) return at.isBefore(endsAt) ? at : null;
    final step = renag.inMilliseconds;
    final passed = after.difference(at).inMilliseconds;
    final next = at.add(Duration(milliseconds: (passed ~/ step + 1) * step));
    return next.isBefore(endsAt) ? next : null;
  }

  /// When it first fires as a full-screen alarm: the firing after the
  /// first one and [escalation] Re-nags went ignored, counted from [at],
  /// so a snooze starts the count anew. Null when it does not escalate
  /// or stops before. The platform's alarm layer follows the same rule.
  DateTime? get escalatesAt {
    final after = escalation;
    if (after == null) return null;
    final first = at.add(renag * (after + 1));
    return first.isBefore(endsAt) ? first : null;
  }

  @override
  String toString() =>
      '$id at ${at.toIso8601String()} every ${renag.inMinutes} min '
      'until ${endsAt.toIso8601String()}'
      '${escalation == null ? '' : ' escalating after $escalation'}';
}

/// The key a snooze of [occurrence] of the Routine [routineId] is kept
/// under.
String snoozeKey(String routineId, Occurrence occurrence) =>
    '$routineId@${occurrence.key}';

/// The alarms that should exist at [now]: for each Routine its
/// Occurrences that are not Done and not yet Missed, from
/// [alarmHorizonDays] back to as far ahead, at most [alarmsPerRoutine],
/// the soonest first and at most [alarmLimit]. An Occurrence already
/// rung stays while it Re-nags. A snoozed one rings at its snooze
/// instead and Re-nags from there; a snooze reaching past when the
/// Occurrence counts as Missed is dropped. Each Re-nags at its Routine's
/// interval, else at [renag], and escalates as its Routine says. [snoozes] are this Device's (ADR 0008),
/// keyed by [snoozeKey].
List<PlannedAlarm> alarmPlan({
  required List<(Routine, Note?)> routines,
  required Map<String, DateTime> snoozes,
  required DateTime now,
  required WallClock wallClock,
  Duration renag = DeviceSettings.defaultRenag,
}) {
  final today = Day.today(now, wallClock);
  final alarms = <PlannedAlarm>[];
  for (final (routine, note) in routines) {
    var planned = 0;
    final timed = _timed(
      routine,
      today.addDays(-alarmHorizonDays),
      today.addDays(alarmHorizonDays),
      wallClock,
    );
    for (final t in timed) {
      if (planned >= alarmsPerRoutine) break;
      final occurrence = t.occurrence;
      if (routine.doneAt(occurrence) != null) continue;
      if (!t.endsAt.isAfter(now)) continue;
      final snoozed = snoozes[snoozeKey(routine.id, occurrence)]?.toUtc();
      if (snoozed != null && !snoozed.isBefore(t.endsAt)) continue;
      alarms.add(
        PlannedAlarm(
          kind: snoozed == null ? AlarmKind.occurrence : AlarmKind.snooze,
          routineId: routine.id,
          occurrence: occurrence,
          at: snoozed ?? t.start,
          wall: snoozed == null ? startOf(occurrence) : null,
          renag: routine.renag ?? renag,
          escalation: routine.escalation,
          ends: t.ends,
          endsAt: t.endsAt,
          text: note?.text,
        ),
      );
      planned++;
    }
  }
  alarms.sort((a, b) {
    final byTime = a.at.compareTo(b.at);
    return byTime != 0 ? byTime : a.id.compareTo(b.id);
  });
  return alarms.length > alarmLimit ? alarms.sublist(0, alarmLimit) : alarms;
}

/// One Missed Occurrence and when it became Missed.
typedef MissedOccurrence = ({Occurrence occurrence, DateTime missedAt});

/// What one Routine Missed, the latest first.
class MissedRoutine {
  final Routine routine;

  /// The Note it links back to; null when it is gone.
  final Note? note;
  final List<MissedOccurrence> missed;

  const MissedRoutine({
    required this.routine,
    required this.note,
    required this.missed,
  });

  /// How often it was Missed.
  int get count => missed.length;
}

/// The Missed Summary (CONTEXT.md): what was Missed since the user last
/// tapped "seen", by Routine, the one Missed latest first.
class MissedSummary {
  final List<MissedRoutine> routines;

  /// When the next Occurrence not Done becomes Missed, so a screen
  /// showing the summary knows when to look again; null when none will.
  final DateTime? nextChange;

  const MissedSummary({required this.routines, required this.nextChange});

  bool get isEmpty => routines.isEmpty;

  /// How many Occurrences were Missed in all.
  int get count => routines.fold(0, (sum, r) => sum + r.count);
}

/// The Missed Summary at [now]: every Occurrence not Done that became
/// Missed after [seen] (or within [alarmHorizonDays] when never seen, and
/// never further back than that), by Routine.
MissedSummary missedSummary({
  required List<(Routine, Note?)> routines,
  required DateTime? seen,
  required DateTime now,
  required WallClock wallClock,
}) {
  final today = Day.today(now, wallClock);
  final from = today.addDays(-alarmHorizonDays);
  final result = <MissedRoutine>[];
  DateTime? nextChange;
  for (final (routine, note) in routines) {
    final missed = <MissedOccurrence>[];
    for (final t in _timed(routine, from, today, wallClock)) {
      if (routine.doneAt(t.occurrence) != null) continue;
      if (t.endsAt.isAfter(now)) {
        if (nextChange == null || t.endsAt.isBefore(nextChange)) {
          nextChange = t.endsAt;
        }
        continue;
      }
      if (seen != null && !t.endsAt.isAfter(seen)) continue;
      missed.add((occurrence: t.occurrence, missedAt: t.endsAt));
    }
    if (missed.isEmpty) continue;
    result.add(
      MissedRoutine(
        routine: routine,
        note: note,
        missed: missed.reversed.toList(),
      ),
    );
  }
  result.sort((a, b) {
    final byTime = b.missed.first.missedAt.compareTo(a.missed.first.missedAt);
    return byTime != 0 ? byTime : a.routine.id.compareTo(b.routine.id);
  });
  return MissedSummary(routines: result, nextChange: nextChange);
}

/// This Device's alarms: its snoozes, the plan over the store, and what
/// the notification's Done and Snooze leave to be applied.
class Alarms {
  static const _snoozes = 'snoozes';

  final Store store;

  Alarms(this.store);

  /// This Device's snoozes, by [snoozeKey]; never synced (ADR 0008).
  Map<String, DateTime> get snoozes {
    final text = store.localSetting(_snoozes);
    if (text == null) return {};
    try {
      final raw = jsonDecode(text);
      if (raw is! Map) return {};
      return {
        for (final MapEntry(:key, :value) in raw.entries)
          if (value is String) '$key': ?DateTime.tryParse(value),
      };
    } on FormatException {
      return {};
    }
  }

  /// The alarm plan at the store's now, on [wallClock].
  List<PlannedAlarm> plan(WallClock wallClock) {
    final routines = Routines(store);
    return alarmPlan(
      routines: [for (final r in routines.all()) (r, routines.note(r))],
      snoozes: snoozes,
      now: store.clock(),
      wallClock: wallClock,
      renag: store.renagInterval,
    );
  }

  /// The Missed Summary at the store's now, since this Device last
  /// marked it "seen".
  MissedSummary missed(WallClock wallClock) {
    final routines = Routines(store);
    return missedSummary(
      routines: [for (final r in routines.all()) (r, routines.note(r))],
      seen: store.missedSeen,
      now: store.clock(),
      wallClock: wallClock,
    );
  }

  /// The user tapped "seen": the Missed Summary starts over from now.
  void markSeen() => store.missedSeen = store.clock();

  /// The Occurrences Done from yesterday to tomorrow on [wallClock], by
  /// Routine id: their notifications, if still showing, can go.
  List<(String, Occurrence)> recentlyDone(WallClock wallClock) {
    final today = Day.today(store.clock(), wallClock);
    return [
      for (final routine in Routines(store).all())
        for (final occurrence in routine.schedule.between(
          today.addDays(-1),
          today.addDays(1),
        ))
          if (routine.doneAt(occurrence) != null) (routine.id, occurrence),
    ];
  }

  /// Applies a Done tapped on a notification at [at]: [occurrenceKey] of
  /// the Routine [routineId] is checked off as of then. Returns whether
  /// it changed anything; a Routine deleted or rescheduled meanwhile, or
  /// an Occurrence already Done, is left as it is.
  bool applyDone(String routineId, String occurrenceKey, DateTime at) {
    final occurrence = Occurrence.tryParse(occurrenceKey);
    final routine = Routines(store).byId(routineId);
    if (occurrence == null || routine == null) return false;
    if (routine.doneAt(occurrence) != null) return false;
    try {
      Routines(store).markDone(routineId, occurrence, at: at);
      return true;
    } on ArgumentError {
      return false;
    }
  }

  /// Applies a Snooze tapped on a notification: [occurrenceKey] of the
  /// Routine [routineId] fires again at [until] and Re-nags from there.
  /// Snoozes over for longer than the plan looks back are dropped on
  /// the way.
  void applySnooze(String routineId, String occurrenceKey, DateTime until) {
    final occurrence = Occurrence.tryParse(occurrenceKey);
    if (occurrence == null) return;
    final oldest = store.clock().subtract(
      const Duration(days: alarmHorizonDays + 1),
    );
    final kept = {
      for (final MapEntry(:key, :value) in snoozes.entries)
        if (value.isAfter(oldest)) key: value,
      snoozeKey(routineId, occurrence): until,
    };
    store.setLocalSetting(
      _snoozes,
      jsonEncode({
        for (final MapEntry(:key, :value) in kept.entries)
          key: value.toUtc().toIso8601String(),
      }),
    );
  }
}
