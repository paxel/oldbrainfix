import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

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
  final TextEditingController count;

  /// The count typed for each weekday, 1 for Monday to 7 for Sunday;
  /// empty for none.
  final Map<int, TextEditingController> counts;
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
    required int count,
    required Map<int, int> counts,
  }) : count = TextEditingController(text: '$count'),
       counts = {
         for (var day = 1; day <= 7; day++)
           day: TextEditingController(text: counts[day]?.toString() ?? ''),
       };

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
      counts: session?.counts ?? const {},
    );
  }

  void dispose() {
    count.dispose();
    for (final controller in counts.values) {
      controller.dispose();
    }
    weekdays.dispose();
  }

  /// The chosen Tallies in their order.
  List<String> get pool => [
    for (final id in order)
      if (chosen.contains(id)) id,
  ];

  /// [text] as a count, 1 to 99; null when it is none.
  static int? countOf(String text) {
    final typed = text.trim();
    if (!RegExp(r'^\d{1,2}$').hasMatch(typed)) return null;
    final count = int.parse(typed);
    return count < 1 || count > SessionKeys.maxCount ? null : count;
  }

  bool get countValid => countOf(count.text) != null;

  /// Whether the weekday [day]'s count is empty or a count.
  bool weekdayValid(int day) {
    final text = counts[day]?.text.trim() ?? '';
    return text.isEmpty || countOf(text) != null;
  }

  bool get weekdaysValid =>
      [for (var d = 1; d <= 7; d++) d].every(weekdayValid);

  /// How many weekdays have their own count typed.
  int get ownCounts => [
    for (final controller in counts.values)
      if (countOf(controller.text) != null) controller,
  ].length;

  bool get valid => pool.isNotEmpty && countValid && weekdaysValid;

  /// Spawns the Session from the Note [noteId] on [schedule]; its id.
  String spawn(String noteId, Schedule schedule) {
    final sessions = Sessions(store);
    final id = sessions
        .spawn(
          noteId,
          schedule,
          pool: pool,
          count: countOf(count.text) ?? 1,
          rule: rule,
        )
        .id;
    for (var day = 1; day <= 7; day++) {
      if (countOf(counts[day]?.text ?? '') case final own?) {
        sessions.setCountOn(id, day, own);
      }
    }
    return id;
  }

  /// Writes what changed of [session].
  void save(Session session) {
    final sessions = Sessions(store);
    final id = session.id;
    final pool = this.pool;
    if (pool.join(',') != session.pool.join(',')) sessions.setPool(id, pool);
    final count = countOf(this.count.text) ?? session.count;
    if (count != session.count) sessions.setCount(id, count);
    for (var day = 1; day <= 7; day++) {
      final own = countOf(counts[day]?.text ?? '');
      if (own != session.counts[day]) sessions.setCountOn(id, day, own);
    }
    if (rule != session.rule) sessions.setRule(id, rule);
  }
}

/// The Session's part of the Routine editor, on top of its schedule: the
/// pool picker, a checkbox per Tally, with drag handles to order it for
/// Rolling; the count; the folded weekdays' own counts, open as last
/// left on this Device; and the rule as radio buttons. [onChanged] runs
/// after each change of [draft].
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
          child: TextField(
            key: const ValueKey('session-count'),
            controller: draft.count,
            keyboardType: TextInputType.number,
            onChanged: (_) => onChanged(),
            decoration: InputDecoration(
              labelText: t.sessionCount,
              errorText: draft.countValid ? null : t.sessionCountInvalid,
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
              subtitle: draft.weekdaysValid
                  ? Text(t.sessionWeekdayCountsSummary(draft.ownCounts))
                  : Text(t.sessionCountInvalid, style: error),
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
                  TextField(
                    key: ValueKey('session-count-$day'),
                    controller: draft.counts[day],
                    keyboardType: TextInputType.number,
                    onChanged: (_) => onChanged(),
                    decoration: InputDecoration(
                      labelText: weekdayName(locale, day),
                      hintText: t.sessionWeekdayCountHint,
                      errorText: draft.weekdayValid(day)
                          ? null
                          : t.sessionCountInvalid,
                    ),
                  ),
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
