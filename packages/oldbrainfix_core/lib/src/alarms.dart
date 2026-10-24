/// The alarm plan (ADR 0004): which alarms should exist on this Device,
/// as a pure function of the Routines, now, the wall clock and this
/// Device's snoozes. The platform's alarm layer only arms what it is
/// handed.
library;

import 'dart:convert';

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

/// Why an alarm fires.
enum AlarmKind {
  /// An Occurrence is due.
  occurrence,

  /// An Occurrence snoozed from its notification is due again.
  snooze,
}

/// One alarm that should exist.
class PlannedAlarm {
  final AlarmKind kind;
  final String routineId;
  final Occurrence occurrence;

  /// When it fires, in UTC.
  final DateTime at;

  /// The wall-clock moment an Occurrence alarm fires at, so the platform
  /// re-arms it at the same local time after a time-zone change; null
  /// for a snooze, which waits a fixed time.
  final ({Day day, ClockTime time})? wall;

  /// The text of the Note the Routine links back to; null when it is
  /// gone.
  final String? text;

  const PlannedAlarm({
    required this.kind,
    required this.routineId,
    required this.occurrence,
    required this.at,
    required this.wall,
    required this.text,
  });

  /// Names the alarm for good: one per kind and Occurrence.
  String get id => '${kind.name}:${snoozeKey(routineId, occurrence)}';

  @override
  String toString() => '$id at ${at.toIso8601String()}';
}

/// The key a snooze of [occurrence] of the Routine [routineId] is kept
/// under.
String snoozeKey(String routineId, Occurrence occurrence) =>
    '$routineId@${occurrence.key}';

/// The alarms that should exist at [now]: for each Routine its next
/// Occurrences within [alarmHorizonDays] that are not Done, at most
/// [alarmsPerRoutine], and every snooze still ahead whose Occurrence is
/// not Done, the soonest first and at most [alarmLimit]. [snoozes] are
/// this Device's (ADR 0008), keyed by [snoozeKey].
List<PlannedAlarm> alarmPlan({
  required List<(Routine, Note?)> routines,
  required Map<String, DateTime> snoozes,
  required DateTime now,
  required WallClock wallClock,
}) {
  final today = Day.today(now, wallClock);
  final last = today.addDays(alarmHorizonDays);
  final alarms = <PlannedAlarm>[];
  final byId = <String, (Routine, Note?)>{};
  for (final (routine, note) in routines) {
    byId[routine.id] = (routine, note);
    var planned = 0;
    for (final occurrence in routine.schedule.between(today, last)) {
      if (planned >= alarmsPerRoutine) break;
      if (routine.doneAt(occurrence) != null) continue;
      final time = occurrence.time ?? allDayAlarmTime;
      final at = instantOf(occurrence.day, time, wallClock);
      if (!at.isAfter(now)) continue;
      alarms.add(
        PlannedAlarm(
          kind: AlarmKind.occurrence,
          routineId: routine.id,
          occurrence: occurrence,
          at: at,
          wall: (day: occurrence.day, time: time),
          text: note?.text,
        ),
      );
      planned++;
    }
  }
  for (final MapEntry(:key, value: until) in snoozes.entries) {
    if (!until.isAfter(now)) continue;
    final at = key.lastIndexOf('@');
    if (at < 0) continue;
    final found = byId[key.substring(0, at)];
    final occurrence = Occurrence.tryParse(key.substring(at + 1));
    if (found == null || occurrence == null) continue;
    final (routine, note) = found;
    if (!routine.schedule.has(occurrence)) continue;
    if (routine.doneAt(occurrence) != null) continue;
    alarms.add(
      PlannedAlarm(
        kind: AlarmKind.snooze,
        routineId: routine.id,
        occurrence: occurrence,
        at: until.toUtc(),
        wall: null,
        text: note?.text,
      ),
    );
  }
  alarms.sort((a, b) {
    final byTime = a.at.compareTo(b.at);
    return byTime != 0 ? byTime : a.id.compareTo(b.id);
  });
  return alarms.length > alarmLimit ? alarms.sublist(0, alarmLimit) : alarms;
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
    );
  }

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
  /// Routine [routineId] fires again at [until]. Snoozes already over
  /// are dropped on the way.
  void applySnooze(String routineId, String occurrenceKey, DateTime until) {
    final occurrence = Occurrence.tryParse(occurrenceKey);
    if (occurrence == null) return;
    final now = store.clock();
    final kept = {
      for (final MapEntry(:key, :value) in snoozes.entries)
        if (value.isAfter(now)) key: value,
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
