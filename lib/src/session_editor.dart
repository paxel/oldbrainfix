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

  /// The tags picked to filter the pool picker by (#220), a view only:
  /// it shows the Tallies carrying any of them. Kept while the page is
  /// open, never stored.
  final Set<String> picked = {};

  SessionDraft._(
    this.store, {
    required this.order,
    required this.chosen,
    required this.notes,
    required this.rule,
    required this.count,
    required this.counts,
  });

  /// The draft of the Session [setup], or of a new Session spawned from
  /// the Note [entity]: its pool as held, then the other Tallies it may
  /// pick.
  factory SessionDraft.of(Store store, String entity, [SessionSetup? setup]) {
    final held = setup?.pool ?? const <String>[];
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
      rule: setup?.rule ?? SessionRule.rolling,
      count: setup?.count ?? 1,
      counts: {...?setup?.counts},
    );
  }

  void dispose() => weekdays.dispose();

  /// The tags in the text of the Tally [id], each once.
  Set<String> tagsOf(String id) => {
    for (final found in TextTags.find(notes[id]?.text ?? '')) found.tag,
  };

  /// The tags of the Tallies of [order], those most of them carry first,
  /// then by name.
  List<String> get tags {
    final counts = <String, int>{};
    for (final id in order) {
      for (final tag in tagsOf(id)) {
        counts[tag] = (counts[tag] ?? 0) + 1;
      }
    }
    return counts.keys.toList()..sort(
      (a, b) => switch ((counts[b] ?? 0).compareTo(counts[a] ?? 0)) {
        0 => a.compareTo(b),
        final by => by,
      },
    );
  }

  /// The tags [picked] that a Tally still carries, in the order of
  /// [tags].
  List<String> get filter => [
    for (final tag in tags)
      if (picked.contains(tag)) tag,
  ];

  /// The Tallies of [order] the picker shows: with a [filter] those
  /// carrying any of its tags, else all.
  List<String> get shown {
    final filter = this.filter;
    if (filter.isEmpty) return order;
    return [
      for (final id in order)
        if (tagsOf(id).any(filter.contains)) id,
    ];
  }

  /// How many Tallies ticked the [filter] hides.
  int get hiddenTicked {
    final shown = this.shown.toSet();
    return order
        .where((id) => chosen.contains(id) && !shown.contains(id))
        .length;
  }

  /// What a Tally made from the filtered picker starts its text with: the
  /// [filter]'s tags, each followed by a space; empty without one.
  String get newTallyText => [for (final tag in filter) '$tag '].join();

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

  /// Whether the picker is ordered by hand for Rolling: with more than
  /// one Tally and no [filter].
  bool get ordersByHand =>
      rule == SessionRule.rolling && order.length > 1 && filter.isEmpty;

  /// What the draft holds as plain values, kept after it is disposed;
  /// read while it is [valid].
  SessionChoice get choice =>
      (pool: pool, count: count, counts: {...counts}, rule: rule);

  /// Spawns the Session from the Note [noteId] on [schedule]; its id.
  String spawn(String noteId, Schedule schedule) =>
      spawnSession(store, noteId, schedule, choice);

  /// Writes what changed of the pool and the count of [held] onto its
  /// Note, as the Session Note's page changes them (#191).
  void write(SessionSetup held) {
    final sessions = Sessions(store);
    final pool = this.pool;
    if (pool.join(',') != held.pool.join(',')) {
      sessions.setPool(held.noteId, pool);
    }
    if (count != held.count) sessions.setCount(held.noteId, count);
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
                SessionPoolList(draft: draft, onChanged: onChanged),
              if (draft.ordersByHand) Text(t.sessionPoolOrder),
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
                  SessionWeekdayRow(
                    day: day,
                    own: draft.counts[day],
                    onOwn: (on) => _change(() => draft.setOwn(day, on)),
                    onCount: (n) => _change(() => draft.counts[day] = n),
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
}

/// A weekday [day]'s own count (1 for Monday to 7 for Sunday): a switch
/// for an own count, "As every day" while it is off, and the count's
/// stepper once it is on ([own]).
class SessionWeekdayRow extends StatelessWidget {
  final int day;
  final int? own;
  final ValueChanged<bool> onOwn;
  final ValueChanged<int> onCount;

  const SessionWeekdayRow({
    super.key,
    required this.day,
    required this.own,
    required this.onOwn,
    required this.onCount,
  });

  @override
  Widget build(BuildContext context) {
    final locale = Localizations.localeOf(context).toString();
    final own = this.own;
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
          onChanged: onOwn,
        ),
        if (own != null)
          CountStepper(
            key: ValueKey('session-count-$day'),
            value: own,
            max: SessionKeys.maxCount,
            onChanged: onCount,
          ),
      ],
    );
  }
}

/// The pool picker of [draft]: its tags as filter chips (#220), then a
/// checkbox per Tally the filter shows, and how many ticked it hides; for
/// Rolling, while nothing is filtered, each with a handle to drag it to
/// its place, never a long press, which only opens menus. With [keepOne] the last Tally ticked cannot be unticked;
/// with [enabled] false it is only shown. [onChanged] runs after each
/// change of [draft].
class SessionPoolList extends StatelessWidget {
  final SessionDraft draft;
  final VoidCallback onChanged;
  final bool keepOne;
  final bool enabled;

  const SessionPoolList({
    super.key,
    required this.draft,
    required this.onChanged,
    this.keepOne = false,
    this.enabled = true,
  });

  void _change(VoidCallback change) {
    change();
    onChanged();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    // Rolling's order is dragged in the whole list, never a filtered one.
    final rolling = enabled && draft.ordersByHand;
    final rows = [
      for (final (index, id) in draft.shown.indexed)
        _poolRow(t, index, id, handle: rolling),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SessionPoolTags(draft: draft, onChanged: onChanged),
        Material(
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
        ),
        SessionPoolHidden(draft: draft),
      ],
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
    final chosen = draft.chosen.contains(id);
    final locked = !enabled || (keepOne && chosen && draft.pool.length == 1);
    return CheckboxListTile(
      key: ValueKey('session-pool-$id'),
      contentPadding: EdgeInsets.zero,
      controlAffinity: ListTileControlAffinity.leading,
      value: chosen,
      title: Text(name),
      onChanged: locked
          ? null
          : (on) => _change(
              () =>
                  on ?? false ? draft.chosen.add(id) : draft.chosen.remove(id),
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

/// The tags of the Tallies of [draft]'s pool picker as filter chips
/// (#220), those most of them carry first; picking one or more shows only
/// the Tallies carrying any of them, and a tap again takes a tag back.
/// Nothing without a tag. [onChanged] runs after each pick.
class SessionPoolTags extends StatelessWidget {
  final SessionDraft draft;
  final VoidCallback onChanged;

  const SessionPoolTags({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final tags = draft.tags;
    if (tags.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Wrap(
        spacing: 4,
        runSpacing: 4,
        children: [
          for (final tag in tags)
            FilterChip(
              key: ValueKey('session-tag-$tag'),
              label: Text(tag),
              selected: draft.picked.contains(tag),
              onSelected: (on) {
                on ? draft.picked.add(tag) : draft.picked.remove(tag);
                onChanged();
              },
            ),
        ],
      ),
    );
  }
}

/// How many Tallies ticked the filter of [draft] hides (#220); they stay
/// ticked. Nothing while it hides none.
class SessionPoolHidden extends StatelessWidget {
  final SessionDraft draft;

  const SessionPoolHidden({super.key, required this.draft});

  @override
  Widget build(BuildContext context) {
    final hidden = draft.hiddenTicked;
    if (hidden == 0) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: 4, bottom: 4),
      child: Text(
        context.t.sessionPoolHidden(hidden),
        key: const ValueKey('session-pool-hidden'),
      ),
    );
  }
}
