import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'capture_choices.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'schedule_text.dart';

/// What the Routine editor holds of a Session while it is edited (#104):
/// the Tallies of its pool picker, in the order Rolling goes through
/// them, those chosen, the count, the weekdays' own counts and the rule.
/// Nothing is written before [save].
class SessionDraft {
  /// The local setting that keeps the weekdays' counts open.
  static const weekdaysOpenKey = 'sessionWeekdaysOpen';

  final Store store;

  /// The Tallies the pool picker shows, the chosen ones first in pool
  /// order.
  final List<String> order;
  final Set<String> chosen;

  /// The Note of each Tally of [order], null when it is gone.
  final Map<String, Note?> notes;
  SessionRule rule;

  /// How many it picks each time, 1 to 99.
  int count;

  /// The own count of each weekday that has one, 1 for Monday to 7 for
  /// Sunday, 1 to 99.
  final Map<int, int> counts;
  final weekdays = ExpansibleController();

  /// Whether the weekdays' counts were left open on this Device.
  late final bool weekdaysOpen = store.localSetting(weekdaysOpenKey) == 'true';

  /// Set by a Save that found something to fix: the pool's error shows
  /// from then on.
  var checked = false;

  SessionDraft._(
    this.store, {
    required this.order,
    required this.chosen,
    required this.notes,
    required this.rule,
    required this.count,
    required this.counts,
  });

  /// The draft of [session], or of a new Session spawned from the Note
  /// [entity]: its pool as held, then the other Tallies it may pick.
  factory SessionDraft.of(Store store, String entity, [Session? session]) {
    final held = session?.pool ?? const <String>[];
    final order = [
      ...held,
      for (final id in Sessions(store).choices(entity))
        if (!held.contains(id)) id,
    ];
    final notes = Notes(store);
    return SessionDraft._(
      store,
      order: order,
      chosen: {...held},
      notes: {for (final id in order) id: notes.byId(id)},
      rule: session?.rule ?? SessionRule.rolling,
      count: session?.count ?? 1,
      counts: {...?session?.counts},
    );
  }

  void dispose() => weekdays.dispose();

  /// The chosen Tallies in their order.
  List<String> get pool => [
    for (final id in order)
      if (chosen.contains(id)) id,
  ];

  /// Gives the weekday [day] its own count, starting from every day's,
  /// or takes it away when [own] is false.
  void setOwn(int day, bool own) =>
      own ? counts[day] = count : counts.remove(day);

  bool get valid => pool.isNotEmpty;

  /// What the draft holds as plain values, kept after it is disposed;
  /// read while it is [valid].
  SessionChoice get choice =>
      (pool: pool, count: count, counts: {...counts}, rule: rule);

  /// Spawns the Session from the Note [noteId] on [schedule]; its id.
  String spawn(String noteId, Schedule schedule) =>
      spawnSession(store, noteId, schedule, choice);

  /// Writes what changed of [session].
  void save(Session session) {
    final sessions = Sessions(store);
    final id = session.id;
    final pool = this.pool;
    if (pool.join(',') != session.pool.join(',')) sessions.setPool(id, pool);
    if (count != session.count) sessions.setCount(id, count);
    for (var day = 1; day <= 7; day++) {
      final own = counts[day];
      if (own != session.counts[day]) sessions.setCountOn(id, day, own);
    }
    if (rule != session.rule) sessions.setRule(id, rule);
  }
}

/// The Session's part of the Routine editor, on top of its schedule: the
/// pool picker, a checkbox per Tally, with drag handles to order it for
/// Rolling; the count's − / + stepper; the folded weekdays, each with a
/// switch for its own count and then its stepper, open as last left on
/// this Device; and the rule as radio buttons. [onChanged] runs after
/// each change of [draft].
class SessionFields extends StatelessWidget {
  final SessionDraft draft;
  final VoidCallback onChanged;

  const SessionFields({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  void _change(VoidCallback change) {
    change();
    onChanged();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final error = TextStyle(color: Theme.of(context).colorScheme.error);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        EditorPanel(
          key: const ValueKey('session-pool'),
          heading: t.sessionPool,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(t.sessionPoolExplain),
              if (draft.order.isEmpty)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(t.sessionPoolNone),
                )
              else
                _pool(context),
              if (draft.rule == SessionRule.rolling && draft.order.length > 1)
                Text(t.sessionPoolOrder),
              if (draft.checked && draft.pool.isEmpty)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    t.sessionPoolMissing,
                    key: const ValueKey('session-pool-error'),
                    style: error,
                  ),
                ),
            ],
          ),
        ),
        EditorPanel(
          child: ControlBelow(
            padding: EdgeInsets.zero,
            title: Text(t.sessionCount),
            control: CountStepper(
              key: const ValueKey('session-count'),
              value: draft.count,
              max: SessionKeys.maxCount,
              onChanged: (n) => _change(() => draft.count = n),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(bottom: 16),
          child: Embossed(
            child: ExpansionTile(
              key: const ValueKey('session-weekdays'),
              controller: draft.weekdays,
              shape: const Border(),
              collapsedShape: const Border(),
              title: Text(t.sessionWeekdayCounts),
              subtitle: Text(
                t.sessionWeekdayCountsSummary(draft.counts.length),
              ),
              initiallyExpanded: draft.weekdaysOpen,
              onExpansionChanged: (open) => draft.store.setLocalSetting(
                SessionDraft.weekdaysOpenKey,
                '$open',
              ),
              childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
              expandedCrossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(t.sessionWeekdayCountsExplain),
                for (var day = DateTime.monday; day <= DateTime.sunday; day++)
                  _weekday(context, locale, day),
              ],
            ),
          ),
        ),
        EditorPanel(
          heading: t.sessionRule,
          child: RadioColumn<SessionRule>(
            key: const ValueKey('session-rule'),
            value: draft.rule,
            onChanged: (rule) => _change(() => draft.rule = rule),
            options: [
              RadioOption(
                value: SessionRule.random,
                title: Text(t.sessionRuleRandom),
                subtitle: Text(t.sessionRuleRandomDetail),
              ),
              RadioOption(
                value: SessionRule.rolling,
                title: Text(t.sessionRuleRolling),
                subtitle: Text(t.sessionRuleRollingDetail),
              ),
              RadioOption(
                value: SessionRule.least,
                title: Text(t.sessionRuleLeast),
                subtitle: Text(t.sessionRuleLeastDetail),
              ),
            ],
          ),
        ),
      ],
    );
  }

  /// The weekday [day]'s row: a switch for an own count, "As every day"
  /// while it is off, and the count's stepper once it is on.
  Widget _weekday(BuildContext context, String locale, int day) {
    final own = draft.counts[day];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SwitchBelow(
          switchKey: ValueKey('session-own-$day'),
          padding: const EdgeInsets.only(top: 8),
          title: Text(weekdayName(locale, day)),
          subtitle: own == null
              ? Text(context.t.sessionWeekdayCountHint)
              : null,
          value: own != null,
          onChanged: (on) => _change(() => draft.setOwn(day, on)),
        ),
        if (own != null)
          CountStepper(
            key: ValueKey('session-count-$day'),
            value: own,
            max: SessionKeys.maxCount,
            onChanged: (n) => _change(() => draft.counts[day] = n),
          ),
      ],
    );
  }

  /// A checkbox per Tally; for Rolling each with a handle to drag it to
  /// its place, never a long press, which only opens menus.
  Widget _pool(BuildContext context) {
    final t = context.t;
    final rolling = draft.rule == SessionRule.rolling;
    final rows = [
      for (final (index, id) in draft.order.indexed)
        _poolRow(t, index, id, handle: rolling),
    ];
    return Material(
      type: MaterialType.transparency,
      child: rolling
          ? ReorderableListView(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              buildDefaultDragHandles: false,
              onReorderItem: (from, to) => _change(() {
                final id = draft.order.removeAt(from);
                draft.order.insert(to, id);
              }),
              children: rows,
            )
          : Column(children: rows),
    );
  }

  Widget _poolRow(
    AppLocalizations t,
    int index,
    String id, {
    required bool handle,
  }) {
    final note = draft.notes[id];
    final name = note == null ? t.noteGone : noteText(t, note);
    return CheckboxListTile(
      key: ValueKey('session-pool-$id'),
      contentPadding: EdgeInsets.zero,
      controlAffinity: ListTileControlAffinity.leading,
      value: draft.chosen.contains(id),
      title: Text(name),
      onChanged: (on) => _change(
        () => on ?? false ? draft.chosen.add(id) : draft.chosen.remove(id),
      ),
      secondary: handle
          ? ReorderableDragStartListener(
              key: ValueKey('session-move-$id'),
              index: index,
              child: Icon(
                Icons.drag_handle,
                semanticLabel: t.sessionMove(name),
              ),
            )
          : null,
    );
  }
}
