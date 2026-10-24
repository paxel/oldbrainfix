/// The Nudge (CONTEXT.md): one reminder covering every Unsorted Note, as
/// a pure function of the Unsorted Notes, now, the wall clock and this
/// Device's snooze, Quiet Hours and repeat interval. The platform's
/// alarm layer follows the same rule (android/…/NudgePlan.kt), so
/// Nudges keep coming while the app is closed.
library;

import 'alarms.dart';
import 'notes.dart';
import 'schedule.dart';

/// A daily window in which Nudges wait (CONTEXT.md). It runs over
/// midnight when [start] is later than [end]; with both the same it is
/// empty.
class QuietHours {
  final ClockTime start;
  final ClockTime end;

  const QuietHours(this.start, this.end);

  /// Whether [time] lies in the window: from [start], up to [end].
  bool contains(ClockTime time) {
    final (s, e, t) = (_minutes(start), _minutes(end), _minutes(time));
    return s <= e ? s <= t && t < e : t >= s || t < e;
  }

  /// When a Nudge due at [instant] fires: at [instant], or at the end of
  /// the window [instant] lies in, on [wallClock].
  DateTime defer(DateTime instant, WallClock wallClock) {
    final wall = wallClock(instant.toUtc());
    final time = ClockTime(wall.hour, wall.minute);
    if (!contains(time)) return instant;
    // In a window over midnight, before midnight it ends tomorrow.
    final day = Day.of(wall).addDays(
      _minutes(start) > _minutes(end) && time.compareTo(start) >= 0 ? 1 : 0,
    );
    final ends = instantOf(day, end, wallClock);
    return ends.isAfter(instant) ? ends : instant;
  }

  @override
  bool operator ==(Object other) =>
      other is QuietHours && other.start == start && other.end == end;

  @override
  int get hashCode => Object.hash(start, end);

  @override
  String toString() => '$start-$end';

  static int _minutes(ClockTime t) => t.hour * 60 + t.minute;
}

/// A snooze of the Nudge, tapped at [at] for the Notes Unsorted then: it
/// fires again at [until].
typedef NudgeSnooze = ({DateTime at, DateTime until});

/// One Unsorted Note's own Nudge time, and when it was captured.
typedef NudgeAnchor = ({DateTime due, DateTime captured});

/// The one Nudge that should exist while Notes are Unsorted.
///
/// It fires first at the earliest own time of the Unsorted Notes and
/// covers them all; while ignored it comes again every [repeat] after it
/// last fired. A Note captured after it last fired brings it forward to
/// that Note's own time when that is sooner, so several captures still
/// make one Nudge. A [snooze] holds back the Notes Unsorted when it was
/// tapped until it ends; Notes captured after it keep their own time. A
/// firing in [quiet] waits for its end, and the repeats count from
/// there.
class PlannedNudge {
  /// The Unsorted Notes, oldest first.
  final List<Note> notes;

  /// Their own Nudge times, by capture, the oldest first.
  final List<NudgeAnchor> anchors;
  final NudgeSnooze? snooze;
  final Duration repeat;

  /// This Device's Quiet Hours; null when they are off.
  final QuietHours? quiet;

  /// When it last fired up to the plan's now, by this rule; null when it
  /// has not yet. Later firings follow from it alone, so the platform
  /// starts counting there.
  final DateTime? since;

  /// When it fires next after the plan's now.
  final DateTime at;

  const PlannedNudge._({
    required this.notes,
    required this.anchors,
    required this.snooze,
    required this.repeat,
    required this.quiet,
    required this.since,
    required this.at,
  });

  /// How many Notes are Unsorted.
  int get count => notes.length;

  /// When it fires first after [after], on [wallClock]. The platform's
  /// alarm layer follows the same rule.
  DateTime firesAfter(DateTime after, WallClock wallClock) {
    final from = since;
    return _firings(
      anchors,
      snooze,
      repeat,
      quiet,
      from != null && !after.isBefore(from) ? from : null,
      after,
      wallClock,
    ).next;
  }

  @override
  String toString() {
    final s = snooze;
    return 'nudge of $count at ${at.toIso8601String()} every '
        '${repeat.inMinutes} min since ${since?.toIso8601String()}, '
        'quiet ${quiet ?? 'off'}, snoozed '
        '${s == null ? 'never' : '${s.at.toIso8601String()}-${s.until.toIso8601String()}'}, '
        'own ${[for (final a in anchors) a.due.toIso8601String()]}';
  }
}

/// When the Nudge fires next after it last fired at [fired] (null:
/// never).
DateTime? _nextFiring(
  List<NudgeAnchor> anchors,
  NudgeSnooze? snooze,
  Duration repeat,
  QuietHours? quiet,
  DateTime? fired,
  WallClock wallClock,
) {
  DateTime? earliest(Iterable<DateTime> times) =>
      times.fold<DateTime?>(null, (a, b) => a == null || b.isBefore(a) ? b : a);
  // The own times of the Notes the last firing did not cover.
  DateTime? ownAfter(DateTime? from) => earliest([
    for (final (:due, :captured) in anchors)
      if (from == null || captured.isAfter(from))
        if (fired == null || due.isAfter(fired)) due,
  ]);
  var raw = earliest([?fired?.add(repeat), ?ownAfter(fired)]);
  if (snooze != null) {
    final (at: tapped, :until) = snooze;
    if (fired == null || fired.isBefore(tapped)) {
      // From the tap to its end only Notes captured after it fire.
      if (raw != null && !raw.isBefore(tapped)) {
        raw = earliest([until, ?ownAfter(tapped)]);
      }
    } else if (fired.isBefore(until)) {
      raw = earliest([?raw, until]);
    }
  }
  if (raw == null) return null;
  return quiet?.defer(raw, wallClock) ?? raw;
}

/// Follows the Nudge from [from], when it fired (null: from the start),
/// to its last firing up to [after] and the next one after it.
({DateTime? last, DateTime next}) _firings(
  List<NudgeAnchor> anchors,
  NudgeSnooze? snooze,
  Duration repeat,
  QuietHours? quiet,
  DateTime? from,
  DateTime after,
  WallClock wallClock,
) {
  var last = from;
  while (true) {
    // Never null: there is a Note, and each firing is followed by a
    // repeat.
    final next =
        _nextFiring(anchors, snooze, repeat, quiet, last, wallClock) ??
        after.add(repeat);
    if (next.isAfter(after)) return (last: last, next: next);
    last = next;
  }
}

/// The Nudge at [now] over the [unsorted] Notes, null when there are
/// none. It repeats every [repeat] while ignored and waits out [quiet]
/// (null when Quiet Hours are off); [snooze] is this Device's latest
/// (ADR 0008).
PlannedNudge? nudgePlan({
  required List<Note> unsorted,
  required NudgeSnooze? snooze,
  required DateTime now,
  required WallClock wallClock,
  required Duration repeat,
  QuietHours? quiet,
}) {
  if (unsorted.isEmpty) return null;
  if (repeat.inMinutes < 1) {
    throw ArgumentError.value(repeat, 'repeat', 'at least a minute');
  }
  final notes = List.of(unsorted)
    ..sort((a, b) => a.captured.compareTo(b.captured));
  final anchors = [
    for (final note in notes)
      (due: note.nudgeAt.toUtc(), captured: note.captured.toUtc()),
  ];
  final snoozed = snooze == null
      ? null
      : (at: snooze.at.toUtc(), until: snooze.until.toUtc());
  final (:last, :next) = _firings(
    anchors,
    snoozed,
    repeat,
    quiet,
    null,
    now.toUtc(),
    wallClock,
  );
  return PlannedNudge._(
    notes: notes,
    anchors: anchors,
    snooze: snoozed,
    repeat: repeat,
    quiet: quiet,
    since: last,
    at: next,
  );
}
