import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'session_editor.dart';
import 'session_form_screen.dart';
import 'session_view.dart';
import 'setting_badges.dart';
import 'tally_form_screen.dart';
import 'tally_view.dart';
import 'unit_system.dart';

/// "Session" in the kind badge of the plain Note [noteId] (#191): the
/// setup's page, its pool with at least one Tally, how many it picks and
/// how; Save makes the Note a Session. Whether it did.
Future<bool> makeSession(
  BuildContext context,
  Notes notes,
  Places places,
  String noteId,
) async =>
    await Navigator.of(context).push<String>(
      MaterialPageRoute(
        builder: (_) =>
            SessionFormScreen(notes: notes, places: places, noteId: noteId),
      ),
    ) !=
    null;

/// "New Tally" in the pool of the Session [noteId]: the New Tally page,
/// filed where the Note is, its name starting with [text], the tags the
/// pool is filtered by (#220); the Tally made there comes first in the
/// pool, ticked. Whether one was made.
Future<bool> addPoolTally(
  BuildContext context,
  Notes notes,
  String noteId, {
  String text = '',
}) async {
  final made = await Navigator.of(context).push<String>(
    MaterialPageRoute(
      builder: (_) => TallyFormScreen(
        notes: notes,
        place: notes.byId(noteId)?.place,
        text: text,
      ),
    ),
  );
  if (made == null) return false;
  final sessions = Sessions(notes.store);
  final setup = sessions.setupOf(noteId);
  if (setup == null) return true;
  sessions.setPool(noteId, [made, ...setup.pool.where((id) => id != made)]);
  return true;
}

/// The setup of a Session Note in its editor (#191), after its name and
/// description: its pool, a checkbox per Tally it may pick, the last one
/// kept, with drag handles for Rolling and "New Tally" at the end; then
/// how many it picks each time with − and +. Each change is written at
/// once through [draft] ([SessionDraft.write]); where this Device may not
/// change the Note ([changes] false) it is only shown. The rule and the
/// weekdays' counts are badges among the Note's settings
/// ([sessionBadges]).
class SessionNoteSetup extends StatelessWidget {
  final SessionDraft draft;
  final bool changes;

  /// Runs after each change of [draft], with it.
  final ValueChanged<SessionDraft> onChanged;

  /// Opens "New Tally".
  final VoidCallback onNewTally;

  const SessionNoteSetup({
    super.key,
    required this.draft,
    required this.changes,
    required this.onChanged,
    required this.onNewTally,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        EditorPanel(
          key: const ValueKey('session-pool'),
          heading: t.sessionPool,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(t.sessionPoolExplain),
              SessionPoolList(
                draft: draft,
                onChanged: () => onChanged(draft),
                keepOne: true,
                enabled: changes,
              ),
              if (changes && draft.ordersByHand) Text(t.sessionPoolOrder),
              if (changes)
                // Making one stays inside the list, on this page.
                Material(
                  type: MaterialType.transparency,
                  child: ListTile(
                    key: const ValueKey('session-new-tally'),
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.add),
                    title: Text(t.newTally),
                    onTap: onNewTally,
                  ),
                ),
            ],
          ),
        ),
        EditorPanel(
          child: ControlBelow(
            padding: EdgeInsets.zero,
            title: Text(t.sessionCount),
            control: changes
                ? CountStepper(
                    key: const ValueKey('session-count'),
                    value: draft.count,
                    max: SessionKeys.maxCount,
                    onChanged: (n) {
                      draft.count = n;
                      onChanged(draft);
                    },
                  )
                : Text(
                    '${draft.count}',
                    key: const ValueKey('session-count'),
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
          ),
        ),
      ],
    );
  }
}

/// The badges of the Session [setup] among its Note's settings (#191):
/// how it picks and the weekdays' own counts, each opening its small
/// editor, which writes at once, where this Device may change the Note
/// ([changes]); [onChanged] runs after an editor closes.
List<Widget> sessionBadges(
  BuildContext context,
  Store store,
  SessionSetup setup, {
  required bool changes,
  required VoidCallback onChanged,
}) {
  final t = context.t;
  return [
    SettingBadge(
      key: const ValueKey('badge-session-rule'),
      setting: t.sessionRule,
      icon: sessionRuleBadgeIcon,
      value: sessionRuleName(t, setup.rule),
      valueIcon: sessionRuleIcon(setup.rule),
      onTap: changes
          ? (_) async {
              final picked = await pickSessionRule(context, setup.rule);
              if (!context.mounted) return;
              if (picked == null || picked == setup.rule) return;
              Sessions(store).setRule(setup.noteId, picked);
              onChanged();
            }
          : null,
    ),
    SettingBadge(
      key: const ValueKey('badge-session-weekdays'),
      setting: t.sessionWeekdayCounts,
      icon: Icons.calendar_view_week,
      value: t.sessionWeekdayCountsSummary(setup.counts.length),
      onTap: changes
          ? (_) async {
              await _editWeekdays(context, store, setup);
              if (!context.mounted) return;
              onChanged();
            }
          : null,
    ),
  ];
}

/// The weekdays' badge's editor: a switch per weekday for its own count,
/// starting from every day's, and its − / + once on; each change written
/// at once.
Future<void> _editWeekdays(
  BuildContext context,
  Store store,
  SessionSetup setup,
) {
  final counts = {...setup.counts};
  final sessions = Sessions(store);
  return showDialog<void>(
    context: context,
    builder: (context) => BadgeDialog(
      setting: context.t.sessionWeekdayCounts,
      icon: Icons.calendar_view_week,
      content: StatefulBuilder(
        builder: (context, setInner) {
          void set(int day, int? own) {
            sessions.setCountOn(setup.noteId, day, own);
            setInner(
              () => own == null ? counts.remove(day) : counts[day] = own,
            );
          }

          return Column(
            key: const ValueKey('session-weekdays'),
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(context.t.sessionWeekdayCountsExplain),
              for (var day = DateTime.monday; day <= DateTime.sunday; day++)
                SessionWeekdayRow(
                  day: day,
                  own: counts[day],
                  onOwn: (on) => set(day, on ? setup.count : null),
                  onCount: (n) => set(day, n),
                ),
            ],
          );
        },
      ),
    ),
  );
}

/// A week of a Session's history on its card: the day it starts, and how
/// many runs of all its Routines were Done and Missed in it.
typedef SessionWeek = ({Day from, int done, int missed});

/// How often the Routines [runs] of a Session were Done and Missed in
/// each of the last [weeks] weeks, this week first, up to [today], weeks
/// starting on [weekStart].
List<SessionWeek> sessionWeeks(
  Routines routines,
  List<Session> runs, {
  required Day today,
  required int weekStart,
  int weeks = 4,
}) {
  final histories = [
    for (final run in runs)
      routines.history(run.routine, deviceWallClock, weekStart: weekStart),
  ];
  final start = today.addDays(-((today.weekday - weekStart) % 7));
  return [
    for (var i = 0; i < weeks; i++)
      () {
        final from = start.addDays(-7 * i);
        final to = i == 0 ? today : from.addDays(6);
        var done = 0;
        var missed = 0;
        for (final history in histories) {
          final week = history.adherence(from, to);
          done += week.done;
          missed += week.missed;
        }
        return (from: from, done: done, missed: missed);
      }(),
  ];
}

/// A Session's past runs (#264), newest first: each Occurrence of its
/// Routines before today that holds a pick, with its date and time and
/// each picked Tally done or not done. At most [limit] when given, the
/// older ones counted under them as on the Session's page; "No past runs
/// yet." while there are none. It reads them when it is built anew,
/// never while building.
class SessionRuns extends StatefulWidget {
  final Notes notes;
  final String noteId;
  final int? limit;

  /// How many runs the card shows.
  static const onCard = 5;

  const SessionRuns({
    super.key,
    required this.notes,
    required this.noteId,
    this.limit,
  });

  @override
  State<SessionRuns> createState() => _SessionRunsState();
}

class _SessionRunsState extends State<SessionRuns> {
  List<SessionRun> _runs = const [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(SessionRuns old) {
    super.didUpdateWidget(old);
    _load();
  }

  void _load() =>
      _runs = Sessions(widget.notes.store).runs(widget.noteId, deviceWallClock);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final locale = Localizations.localeOf(context).toString();
    final limit = widget.limit;
    final shown = limit == null || _runs.length <= limit
        ? _runs
        : _runs.take(limit).toList();
    final older = _runs.length - shown.length;
    return Column(
      key: const ValueKey('session-runs'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (_runs.isEmpty) Text(t.sessionRunsNone, style: text.bodyMedium),
        for (final (i, run) in shown.indexed)
          Padding(
            key: ValueKey('session-run-$i'),
            padding: const EdgeInsets.only(top: 6),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '${dayText(locale, run.occurrence.day)} · '
                  '${occurrenceTime(t, locale, run.occurrence)}',
                  style: text.labelLarge,
                ),
                for (final pick in run.picks)
                  Row(
                    children: [
                      Icon(
                        pick.done
                            ? Icons.check_circle_outline
                            : Icons.radio_button_unchecked,
                        size: 18,
                        color: pick.done
                            ? scheme.primary
                            : scheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          switch (widget.notes.byId(pick.noteId)) {
                            final note? when pick.done => t.sessionRunDone(
                              noteText(t, note),
                            ),
                            final note? => t.sessionRunNotDone(
                              noteText(t, note),
                            ),
                            null when pick.done => t.sessionRunDone(t.noteGone),
                            null => t.sessionRunNotDone(t.noteGone),
                          },
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
              ],
            ),
          ),
        if (older > 0)
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text(
              t.sessionOlderRuns(older),
              style: text.bodySmall?.copyWith(color: scheme.onSurfaceVariant),
            ),
          ),
      ],
    );
  }
}

/// A Session on its Note's detailed card (#191), nothing while the Note
/// is no Session: its pool, how many it picks and how; today's picks of
/// each of its Routines, each recorded or swapped there as on the
/// Agenda; how often it was Done and skipped in each of the last four
/// weeks; and its latest past runs (#264). A Session without Routines
/// shows its pool and its setup only.
/// It reads the Session when the card is built anew and after a change
/// of its own, never while building.
class CardSession extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// How many weeks of history it shows.
  static const weeks = 4;

  const CardSession({super.key, required this.notes, required this.noteId});

  @override
  State<CardSession> createState() => _CardSessionState();
}

class _CardSessionState extends State<CardSession> {
  late final _sessions = Sessions(widget.notes.store);
  SessionSetup? _setup;
  List<Note?> _pool = const [];
  List<SessionOccurrence> _today = const [];
  List<SessionWeek> _weeks = const [];
  UnitSystem _system = UnitSystem.metric;

  Store get _store => widget.notes.store;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _load();
  }

  @override
  void didUpdateWidget(CardSession old) {
    super.didUpdateWidget(old);
    // A pick or an entry changes no Note, so this reads every time the
    // card is built anew.
    _load();
  }

  void _load() {
    // An older Session spawned from a Tally stays a Tally (ADR 0039).
    final setup = _setup =
        NoteKinds(_store).of(widget.noteId) == NoteKind.session
        ? _sessions.setupOf(widget.noteId)
        : null;
    if (setup == null) return;
    _system = deviceUnitSystem(_store);
    _pool = [for (final id in _sessions.pool(setup)) widget.notes.byId(id)];
    final runs = _sessions.ofNote(widget.noteId);
    // As the Agenda shows them, by time and then by Routine; written
    // there, not here.
    _today = [
      for (final today in _sessions.today(deviceWallClock))
        if (today.session.noteId == widget.noteId) today,
    ];
    final first = MaterialLocalizations.of(context).firstDayOfWeekIndex;
    _weeks = runs.isEmpty
        ? const []
        : sessionWeeks(
            Routines(_store),
            runs,
            today: Day.today(_store.clock(), deviceWallClock),
            weekStart: first == 0 ? DateTime.sunday : first,
            weeks: CardSession.weeks,
          );
  }

  Future<void> _open(SessionOccurrence today, SessionPick pick) async {
    await openPick(context, widget.notes, today, pick);
    if (!mounted) return;
    setState(_load);
  }

  void _another(SessionOccurrence today, SessionPick pick) {
    _sessions.another(today.session.id, today.occurrence, pick.noteId);
    setState(_load);
  }

  Widget _occurrence(SessionOccurrence today, String locale, String decimal) {
    final t = context.t;
    final id = today.session.id;
    final swaps = today.canSwap && _store.mayCheckOff(id);
    return Padding(
      key: ValueKey('card-session-today-$id@${today.occurrence.key}'),
      padding: const EdgeInsets.only(top: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            occurrenceTime(t, locale, today.occurrence),
            style: Theme.of(context).textTheme.labelLarge,
          ),
          SessionPicks(
            today: today,
            texts: [
              for (final pick in today.picks)
                pickText(t, pick, _system, decimal),
            ],
            onOpen: (i) => opensPick(_store, id, today.picks[i])
                ? () => _open(today, today.picks[i])
                : null,
            onAnother: (i) =>
                swaps ? () => _another(today, today.picks[i]) : null,
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final setup = _setup;
    if (setup == null) return const SizedBox.shrink();
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final locale = Localizations.localeOf(context).toString();
    final decimal = decimalSign(context);
    return Padding(
      key: const ValueKey('card-session'),
      padding: const EdgeInsets.only(top: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(t.sessionPool, style: text.titleSmall),
          for (final (i, note) in _pool.indexed)
            Padding(
              key: ValueKey('card-session-pool-$i'),
              padding: const EdgeInsets.symmetric(vertical: 2),
              child: Row(
                children: [
                  const Icon(Icons.bar_chart, size: 18),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      note == null ? t.noteGone : noteText(t, note),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
            ),
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text(
              '${t.sessionPicks(setup.count)} · '
              '${sessionRuleName(t, setup.rule)}',
              key: const ValueKey('card-session-setup'),
              style: text.bodySmall,
            ),
          ),
          if (_today.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(t.agendaToday, style: text.titleSmall),
            for (final today in _today) _occurrence(today, locale, decimal),
          ],
          if (_weeks.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(t.sessionHistory, style: text.titleSmall),
            for (final (i, week) in _weeks.indexed)
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(
                  t.sessionHistoryWeek(
                    dayText(locale, week.from),
                    week.done,
                    week.missed,
                  ),
                  key: ValueKey('card-session-week-$i'),
                  style: text.bodyMedium,
                ),
              ),
          ],
          const SizedBox(height: 12),
          Text(t.sessionRuns, style: text.titleSmall),
          SessionRuns(
            notes: widget.notes,
            noteId: widget.noteId,
            limit: SessionRuns.onCard,
          ),
        ],
      ),
    );
  }
}
