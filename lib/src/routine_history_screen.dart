import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'l10n.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';

/// Opens the history of [routine].
Future<void> openHistory(BuildContext context, Notes notes, Routine routine) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            RoutineHistoryScreen(notes: notes, routineId: routine.id),
      ),
    );

/// A Routine's history (CONTEXT.md): its Streak, current and longest,
/// with the last seven days as dots; its Adherence this week, this month
/// and over the last 30 days; and a log of every Occurrence, the latest
/// first, Done (when, early or late), Missed or still due, with the
/// snoozes this Device recorded. Streaks can be switched off on this
/// Device here, for now (it moves to Settings later); Adherence stays.
class RoutineHistoryScreen extends StatefulWidget {
  final Notes notes;
  final String routineId;

  const RoutineHistoryScreen({
    super.key,
    required this.notes,
    required this.routineId,
  });

  @override
  State<RoutineHistoryScreen> createState() => _RoutineHistoryScreenState();
}

class _RoutineHistoryScreenState extends State<RoutineHistoryScreen> {
  late final _routines = Routines(widget.notes.store);
  late bool _streaksOn = widget.notes.store.streaksOn;
  Routine? _routine;
  Note? _note;
  RoutineHistory? _history;

  /// The log, the latest first.
  List<HistoryEntry> _log = const [];

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // The week starts where the reader's locale starts it.
    final first = MaterialLocalizations.of(context).firstDayOfWeekIndex;
    final weekStart = first == 0 ? DateTime.sunday : first;
    if (_history?.weekStart == weekStart) return;
    final routine = _routine = _routines.byId(widget.routineId);
    if (routine == null) return;
    _note = _routines.note(routine);
    final history = _history = _routines.history(
      routine,
      deviceWallClock,
      weekStart: weekStart,
    );
    _log = history.entries.reversed.toList();
  }

  void _setStreaks(bool on) {
    widget.notes.store.streaksOn = on;
    setState(() => _streaksOn = on);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final routine = _routine;
    final history = _history;
    final note = _note;
    final header = <Widget>[
      if (routine != null) ...[
        Text(
          note == null ? t.noteGone : noteText(t, note),
          maxLines: 3,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        Text(scheduleText(t, locale, routine.schedule)),
        const SizedBox(height: 16),
      ],
      if (history != null) ...[
        if (_streaksOn) _StreakPanel(history: history),
        _AdherencePanel(history: history),
      ],
      SwitchListTile(
        value: _streaksOn,
        onChanged: _setStreaks,
        title: Text(t.showStreaks),
        subtitle: Text(t.showStreaksDetail),
        contentPadding: EdgeInsets.zero,
      ),
      const SizedBox(height: 8),
      Text(t.historyLog, style: Theme.of(context).textTheme.titleSmall),
      Text(
        t.snoozesHere,
        style: Theme.of(context).textTheme.bodySmall
            ?.copyWith(color: Theme.of(context).colorScheme.onSurfaceVariant),
      ),
      if (_log.isEmpty)
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 16),
          child: Text(t.historyEmpty),
        ),
    ];
    return Scaffold(
      appBar: AppBar(title: Text(t.historyTitle)),
      body: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: header.length + _log.length,
        itemBuilder: (context, i) => i < header.length
            ? header[i]
            : _LogRow(entry: _log[i - header.length]),
      ),
    );
  }
}

/// The current and longest Streak, and the last seven days as dots.
class _StreakPanel extends StatelessWidget {
  final RoutineHistory history;

  const _StreakPanel({required this.history});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme;
    Widget figure(String label, int count) => Expanded(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('$count', style: text.headlineMedium),
          Text(label),
        ],
      ),
    );
    return EditorPanel(
      heading: t.streakHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              figure(t.streakNow, history.currentStreak),
              figure(t.streakLongest, history.longestStreak),
            ],
          ),
          const SizedBox(height: 16),
          Text(t.lastSevenDays, style: text.labelLarge),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              for (final tally in history.lastSevenDays)
                _DayDot(tally: tally, locale: locale),
            ],
          ),
        ],
      ),
    );
  }
}

/// One day as a dot under its weekday: all Done, partly, Missed, still
/// due, or nothing due.
class _DayDot extends StatelessWidget {
  final DayTally tally;
  final String locale;

  const _DayDot({required this.tally, required this.locale});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final scheme = Theme.of(context).colorScheme;
    final day = dayText(locale, tally.day);
    final (icon, colour, label) = tally.empty
        ? (Icons.remove, scheme.outline, t.dayNothing(day))
        : tally.missed > 0
        ? tally.done > 0
              ? (Icons.timelapse, scheme.tertiary, t.dayPartlyDone(day))
              : (Icons.cancel, scheme.error, t.dayMissed(day))
        : tally.open > 0
        ? (Icons.radio_button_unchecked, scheme.primary, t.dayOpen(day))
        : (Icons.check_circle, scheme.primary, t.dayAllDone(day));
    return Semantics(
      label: label,
      excludeSemantics: true,
      child: Column(
        children: [
          Text(weekdayText(locale, tally.day.weekday)),
          const SizedBox(height: 4),
          Icon(icon, color: colour, size: 28),
        ],
      ),
    );
  }
}

/// The Adherence this week, this month and over the last 30 days.
class _AdherencePanel extends StatelessWidget {
  final RoutineHistory history;

  const _AdherencePanel({required this.history});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final percent = NumberFormat.percentPattern(locale);
    final text = Theme.of(context).textTheme;
    Widget row(String label, Adherence adherence) {
      final share = adherence.share;
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            Expanded(child: Text(label)),
            if (share == null)
              Text(t.adherenceNone)
            else ...[
              Text(
                t.adherenceOf(
                  adherence.done,
                  adherence.done + adherence.missed,
                ),
                style: text.bodySmall,
              ),
              const SizedBox(width: 12),
              Text(percent.format(share), style: text.titleMedium),
            ],
          ],
        ),
      );
    }

    return EditorPanel(
      heading: t.adherenceHeading,
      child: Column(
        children: [
          row(t.adherenceThisWeek, history.thisWeek),
          row(t.adherenceThisMonth, history.thisMonth),
          row(t.adherenceLast30Days, history.last30Days),
        ],
      ),
    );
  }
}

/// One Occurrence in the log: when it was due, and what came of it.
class _LogRow extends StatelessWidget {
  final HistoryEntry entry;

  const _LogRow({required this.entry});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final scheme = Theme.of(context).colorScheme;
    final occurrence = entry.occurrence;
    final (icon, colour) = switch (entry.status) {
      OccurrenceStatus.done => (Icons.check_circle, scheme.primary),
      OccurrenceStatus.missed => (Icons.cancel, scheme.error),
      OccurrenceStatus.open => (Icons.radio_button_unchecked, scheme.primary),
      OccurrenceStatus.coming => (Icons.schedule, scheme.outline),
    };
    final snoozed = entry.snoozedUntil;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(icon, color: colour),
      title: Text(
        '${dayText(locale, occurrence.day)} · '
        '${occurrenceTime(t, locale, occurrence)}',
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_outcome(t, locale)),
          if (snoozed != null)
            Text(t.logSnoozed(_when(locale, snoozed, occurrence.day))),
        ],
      ),
    );
  }

  String _outcome(AppLocalizations t, String locale) {
    final doneAt = entry.doneAt;
    if (doneAt == null) {
      return switch (entry.status) {
        OccurrenceStatus.missed => t.logMissed,
        OccurrenceStatus.coming => t.logComing,
        _ => t.logOpen,
      };
    }
    final when = _when(locale, doneAt, entry.occurrence.day);
    return switch (entry.timing) {
      DoneTiming.early => t.logDoneEarly(when),
      DoneTiming.late => t.logDoneLate(when),
      _ => t.logDone(when),
    };
  }

  /// [instant] on this Device's clock: its time, with the day when that
  /// is not [day].
  static String _when(String locale, DateTime instant, Day day) {
    final wall = deviceWallClock(instant.toUtc());
    final time = clockText(locale, ClockTime(wall.hour, wall.minute));
    return Day.of(wall) == day
        ? time
        : '${dayText(locale, Day.of(wall))} $time';
  }
}
