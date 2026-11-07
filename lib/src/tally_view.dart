import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'browser.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'profile.dart';
import 'routine_editor_screen.dart';
import 'setting_badges.dart';
import 'tally_entry_screen.dart';
import 'tally_field_screen.dart';
import 'unit_system.dart';

/// The rule [rule] as the value icon of a field's rule badge (#192).
IconData tallyRuleIcon(TallyRule rule) => switch (rule) {
  TallyRule.sum => Icons.add_circle_outline,
  TallyRule.max => Icons.vertical_align_top,
};

/// The decimal sign of the app's language, for showing numbers.
String decimalSign(BuildContext context) =>
    NumberFormat.decimalPattern(Localizations.localeOf(context).toString())
        .symbols
        .DECIMAL_SEP;

/// [unit] as this Device shows it: a known one in [system], a typed one
/// as typed.
String shownUnit(String unit, UnitSystem system) =>
    TallyUnits.shownUnit(unit, system);

/// [base], stored in the base unit of [unit], as shown with its unit.
String shownValue(double base, String unit, UnitSystem system, String decimal) {
  final number = TallyUnits.formatShown(base, unit, system, decimal: decimal);
  final shown = shownUnit(unit, system);
  return shown.isEmpty ? number : '$number $shown';
}

/// [base] as typed into a field of [unit]: as shown, without the unit.
String typedValue(
  double? base,
  String unit,
  UnitSystem system,
  String decimal,
) => base == null
    ? ''
    : TallyUnits.formatShown(base, unit, system, decimal: decimal);

/// Puts the focus into the field of [node] and scrolls it into view, for
/// a field whose error text says what to fix.
void showField(FocusNode node) {
  node.requestFocus();
  if (node.context case final context?) {
    unawaited(Scrollable.ensureVisible(context, alignment: 0.2));
  }
}

/// Whose entries the table and the charts show in a Shared Area: the
/// user's own, everyone's or one person's, by the radio button's value.
abstract final class _Whose {
  static const mine = '';
  static const everyone = '*';

  static PersonFilter filter(String whose) => switch (whose) {
    mine => const PersonFilter(me: true),
    everyone => PersonFilter.everyone,
    final id => PersonFilter(people: {id}),
  };
}

/// An entry with who wrote it: null for the user's own (ADR 0017).
typedef _ShownEntry = ({TallyEntry entry, Profile? author, bool changes});

/// The name of [period] as a bar's length.
String tallyPeriodName(AppLocalizations t, TallyPeriod period) =>
    switch (period) {
      TallyPeriod.day => t.tallyPeriodDay,
      TallyPeriod.week => t.tallyPeriodWeek,
      TallyPeriod.month => t.tallyPeriodMonth,
      TallyPeriod.year => t.tallyPeriodYear,
    };

/// The setting of this Device that keeps all entries of the Tally of
/// [noteId] shown on its card (#190); nothing kept means folded after
/// [CardTally.shownEntries].
String tallyEntriesShownSetting(String noteId) => 'tallyEntriesShown:$noteId';

/// The person filter a Tally of [noteId] counts by unless one is picked:
/// the user's own entries in a Shared Area (ADR 0028), everyone's, which
/// are the user's own too, elsewhere. The compact view's totals and the
/// detailed view count alike (#190).
PersonFilter tallyDefaultFilter(Store store, String noteId) {
  final area = store.areaOf(noteId);
  return area != null && store.isShared(area)
      ? const PersonFilter(me: true)
      : PersonFilter.everyone;
}

/// The setup of a Note's Tally in its editor (#190), nothing while the
/// Note is no Tally: the Tally's own settings as badges (#264), the length
/// of a bar of its charts and whether it counts in sets; then a row per
/// field, its name and its settings as badges as on the New Tally page
/// (#161), how several sets or entries in a bar count, its unit and its
/// target, a Check field its kind and its target, a field that counted
/// sets as a multiplier did (#200) that it counts the sets of older
/// entries, "Hidden" while it is hidden; a tap on the
/// name or a badge opens the field's page for who may change the Note,
/// and "Add field" the page of a new one. The chart and the entries show
/// on the card, not here. It follows the log as it changes.
class TallySetup extends StatefulWidget {
  final Notes notes;
  final String noteId;

  const TallySetup({super.key, required this.notes, required this.noteId});

  @override
  State<TallySetup> createState() => _TallySetupState();
}

class _TallySetupState extends State<TallySetup> with ReloadsOnLogChange {
  late final _tallies = Tallies(widget.notes.store);
  late UnitSystem _system;
  Tally? _tally;

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => setState(_load);

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() {
    _system = deviceUnitSystem(widget.notes.store);
    _tally = _tallies.byNote(widget.noteId);
  }

  Future<void> _openField(String? fieldId) async {
    await openTallyField(context, widget.notes, widget.noteId, fieldId);
    if (!mounted) return;
    setState(_load);
  }

  /// The length of a bar: a dialog of the four lengths, the one chosen
  /// written at once.
  Future<void> _pickBar(Tally tally) => showDialog<void>(
    context: context,
    builder: (context) {
      final t = context.t;
      return BadgeDialog(
        setting: t.tallyPeriod,
        icon: Icons.bar_chart,
        content: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(t.tallyBarHint),
            RadioColumn<TallyPeriod>(
              key: const ValueKey('tally-bar'),
              value: tally.bar,
              onChanged: (bar) {
                _tallies.setBar(widget.noteId, bar);
                Navigator.of(context).pop();
              },
              options: [
                for (final period in TallyPeriod.values)
                  RadioOption(
                    key: ValueKey('tally-bar-${period.name}'),
                    value: period,
                    title: Text(tallyPeriodName(t, period)),
                  ),
              ],
            ),
          ],
        ),
      );
    },
  ).then((_) => mounted ? setState(_load) : null);

  /// Whether it counts in sets: a dialog with the switch, written at once.
  Future<void> _pickSets(Tally tally) => showDialog<void>(
    context: context,
    builder: (context) {
      final t = context.t;
      return BadgeDialog(
        setting: t.tallyCountsInSets,
        icon: Icons.repeat,
        content: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(t.tallyCountsInSetsDetail),
            const SizedBox(height: 8),
            BadgeSwitch(
              key: const ValueKey('tally-sets'),
              setting: t.tallyCountsInSets,
              on: tally.countsInSets,
              toggle: () => _tallies.setCountsInSets(
                widget.noteId,
                !(_tallies.byNote(widget.noteId)?.countsInSets ?? false),
              ),
            ),
          ],
        ),
      );
    },
  ).then((_) => mounted ? setState(_load) : null);

  Widget _fieldRow(Tally tally, TallyField field, String decimal) {
    final t = context.t;
    final unit = shownUnit(field.unit, _system);
    final target = field.target;
    final countsSets = field.multiplies != null;
    final open = widget.notes.store.mayChange(field.noteId)
        ? () => _openField(field.id)
        : null;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          InkWell(
            key: ValueKey('tally-field-${field.id}'),
            onTap: open,
            borderRadius: BorderRadius.circular(8),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: Text(
                field.name,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
          ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              if (field.check)
                SettingBadge(
                  key: ValueKey('tally-setup-kind-${field.id}'),
                  setting: t.tallyFieldKind,
                  icon: Icons.check_box_outlined,
                  value: t.tallyFieldKindCheck,
                  onTap: open == null ? null : (_) => open(),
                ),
              if (countsSets)
                SettingBadge(
                  key: ValueKey('tally-setup-counts-sets-${field.id}'),
                  setting: t.tallyCountsInSets,
                  icon: Icons.repeat,
                  value: t.tallySetCounter,
                ),
              // A field counting sets counts no value: no rule, no target.
              if (!countsSets && !field.check)
                SettingBadge(
                  key: ValueKey('tally-setup-rule-${field.id}'),
                  setting: t.tallyRule,
                  icon: Icons.functions,
                  value: field.rule == TallyRule.sum
                      ? t.tallyRuleSumShort
                      : t.tallyRuleMaxShort,
                  valueIcon: tallyRuleIcon(field.rule),
                  onTap: open == null ? null : (_) => open(),
                ),
              if (!field.check)
                SettingBadge(
                  key: ValueKey('tally-setup-unit-${field.id}'),
                  setting: t.tallyUnit,
                  icon: Icons.straighten,
                  value: unit.isEmpty ? t.tallyNoUnit : unit,
                  onTap: open == null ? null : (_) => open(),
                ),
              if (!countsSets)
                SettingBadge(
                  key: ValueKey('tally-setup-target-${field.id}'),
                  setting: t.tallyTargetBadge,
                  icon: Icons.flag_outlined,
                  value: target == null
                      ? t.tallyNoTarget
                      : shownValue(target, field.unit, _system, decimal),
                  onTap: open == null ? null : (_) => open(),
                ),
              if (field.hidden)
                SettingBadge(
                  key: ValueKey('tally-setup-hidden-${field.id}'),
                  setting: t.tallyHideField,
                  icon: Icons.visibility_off_outlined,
                  value: t.tallyFieldHidden,
                  valueIcon: switchIcon(true),
                  onTap: open == null ? null : (_) => open(),
                ),
            ],
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final tally = _tally;
    if (tally == null) return const SizedBox.shrink();
    final t = context.t;
    final decimal = decimalSign(context);
    final changes = widget.notes.store.mayChange(widget.noteId);
    return EditorPanel(
      key: const ValueKey('tally-fields'),
      heading: t.tallyFields,
      child: Material(
        type: MaterialType.transparency,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  SettingBadge(
                    key: const ValueKey('tally-setup-bar'),
                    setting: t.tallyPeriod,
                    icon: Icons.bar_chart,
                    value: tallyPeriodName(t, tally.bar),
                    onTap: changes ? (_) => _pickBar(tally) : null,
                  ),
                  SettingBadge(
                    key: const ValueKey('tally-setup-sets'),
                    setting: t.tallyCountsInSets,
                    icon: Icons.repeat,
                    value: tally.countsInSets ? t.settingOn : t.settingOff,
                    valueIcon: switchIcon(tally.countsInSets),
                    onTap: changes ? (_) => _pickSets(tally) : null,
                  ),
                ],
              ),
            ),
            for (final field in tally.fields) _fieldRow(tally, field, decimal),
            if (widget.notes.store.mayChange(widget.noteId))
              FilledButton.tonalIcon(
                key: const ValueKey('tally-add-field'),
                onPressed: () => _openField(null),
                icon: const Icon(Icons.playlist_add),
                label: Text(t.tallyAddField),
              ),
          ],
        ),
      ),
    );
  }
}

/// A Tally on its Note's detailed card (#190), nothing while the Note is
/// no Tally: "Add entry" for who may add one; in a Shared Area the choice
/// of whose entries show, the user's own by default; one chart per field
/// shown but those counting sets, a bar per the Tally's bar length
/// (#264), a Check field's of its ticks; the entries, newest first, each
/// with its ticks and its sets, the latest [shownEntries] and the older folded under
/// how many they are until a tap shows them, kept per Note on this
/// Device; each tapped to edit it and long-pressed for a menu at the
/// finger; and a folded "Removed entries" with Restore, closed until
/// opened, remembered on this Device. It reads the Tally when the card is
/// built anew and after a change of its own, never while building.
class CardTally extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// How many entries show before the older ones fold.
  static const shownEntries = 5;

  const CardTally({super.key, required this.notes, required this.noteId});

  @override
  State<CardTally> createState() => _CardTallyState();
}

class _CardTallyState extends State<CardTally> {
  /// The local setting that keeps "Removed entries" open.
  static const _removedOpenKey = 'tallyRemovedOpen';

  late final _tallies = Tallies(widget.notes.store);
  late UnitSystem _system;
  Tally? _tally;
  var _shared = false;
  var _whose = _Whose.mine;
  var _allShown = false;
  List<_ShownEntry> _entries = const [];
  List<_ShownEntry> _removed = const [];

  /// The other people who wrote entries, for "Show entries".
  List<Profile> _authors = const [];
  Map<String, List<TallyBucket>> _charts = const {};
  late bool _removedOpen =
      widget.notes.store.localSetting(_removedOpenKey) == 'true';

  Store get _store => widget.notes.store;

  @override
  void initState() {
    super.initState();
    final store = _store;
    final area = store.areaOf(widget.noteId);
    _shared = area != null && store.isShared(area);
    // Outside a Shared Area every entry is the user's own.
    _whose = _shared ? _Whose.mine : _Whose.everyone;
    _allShown =
        store.localSetting(tallyEntriesShownSetting(widget.noteId)) == 'true';
    _load();
  }

  @override
  void didUpdateWidget(CardTally old) {
    super.didUpdateWidget(old);
    // An entry changes no Note, so this reads every time the card is
    // built anew.
    _load();
  }

  void _load() {
    final store = _store;
    _system = deviceUnitSystem(store);
    final tally = _tally = _tallies.byNote(widget.noteId);
    if (tally == null) return;
    _ShownEntry shown(TallyEntry entry) => (
      entry: entry,
      author: store.authorOf(entry.id),
      changes: store.mayChange(entry.id),
    );
    if (_shared) {
      final authors = <String, Profile>{};
      for (final entry in _tallies.entries(widget.noteId)) {
        if (store.authorOf(entry.id) case final author?) {
          authors.putIfAbsent(author.id, () => author);
        }
      }
      _authors = authors.values.toList();
      // The person picked may have no entries any more.
      if (_whose != _Whose.mine &&
          _whose != _Whose.everyone &&
          !authors.containsKey(_whose)) {
        _whose = _Whose.mine;
      }
    }
    final filter = _Whose.filter(_whose);
    _entries = [
      for (final entry in _tallies.entries(widget.noteId, filter: filter))
        shown(entry),
    ];
    _charts = {
      for (final field in tally.counting)
        field.id: _tallies.chart(
          widget.noteId,
          field.id,
          deviceWallClock,
          filter: filter,
        ),
    };
    // The last removed first.
    _removed =
        [
          for (final entry in _tallies.removedEntries(widget.noteId))
            shown(entry),
        ]..sort(
          (a, b) => (b.entry.trashed ?? b.entry.at).compareTo(
            a.entry.trashed ?? a.entry.at,
          ),
        );
  }

  void _setWhose(String whose) => setState(() {
    _whose = whose;
    _load();
  });

  void _toggleAll() {
    final setting = tallyEntriesShownSetting(widget.noteId);
    setState(() => _allShown = !_allShown);
    _allShown
        ? _store.setLocalSetting(setting, 'true')
        : _store.removeLocalSetting(setting);
  }

  void _setRemovedOpen(bool open) {
    _removedOpen = open;
    _store.setLocalSetting(_removedOpenKey, '$open');
  }

  Future<void> _addEntry() async {
    await addTallyEntry(context, widget.notes, widget.noteId);
    if (!mounted) return;
    setState(_load);
  }

  Future<void> _editEntry(TallyEntry entry) async {
    await editTallyEntry(context, widget.notes, widget.noteId, entry.id);
    if (!mounted) return;
    setState(_load);
  }

  Future<void> _deleteEntry(TallyEntry entry) async {
    if (!await deleteTallyEntry(context, _tallies, entry.id)) return;
    if (!mounted) return;
    setState(_load);
  }

  void _restore(TallyEntry entry) {
    _tallies.restoreEntry(entry.id);
    setState(_load);
  }

  /// A long press on an entry: Edit and Delete in a menu where the finger
  /// is, for who may change it.
  Future<void> _entryMenu(TallyEntry entry, Offset at) async {
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final run = await showMenu<Future<void> Function()>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        PopupMenuItem(
          key: const ValueKey('tally-entry-edit'),
          value: () => _editEntry(entry),
          child: Row(
            children: [
              const Icon(Icons.edit_outlined),
              const SizedBox(width: 12),
              Flexible(child: Text(t.edit)),
            ],
          ),
        ),
        PopupMenuItem(
          key: const ValueKey('tally-entry-delete'),
          value: () => _deleteEntry(entry),
          child: Row(
            children: [
              const Icon(Icons.delete_outline),
              const SizedBox(width: 12),
              Flexible(child: Text(t.delete)),
            ],
          ),
        ),
      ],
    );
    if (run == null || !mounted) return;
    await run();
  }

  /// What [entry] holds for the fields shown (#264): the Check fields it
  /// ticks, then its sets, each a line of its values one after the other,
  /// numbered while there are several ("1. Weight 60 kg · Reps 10").
  List<String> _values(Tally tally, TallyEntry entry, String decimal) {
    final ticked = [
      for (final field in tally.checks)
        if (entry.values.containsKey(field.id)) '✓ ${field.name}',
    ];
    final sets = [
      for (final set in tally.setsOf(entry))
        [
          for (final field in tally.numbers)
            if (set[field.id] case final value?)
              '${field.name} ${shownValue(value, field.unit, _system, decimal)}',
        ].join(' · '),
    ].where((line) => line.isNotEmpty).toList();
    return [
      if (ticked.isNotEmpty) ticked.join(' · '),
      if (sets.length == 1)
        sets.single
      else
        for (final (i, line) in sets.indexed) '${i + 1}. $line',
    ];
  }

  Widget _entryRow(
    Tally tally,
    _ShownEntry shown,
    DateFormat when,
    String decimal, {
    bool removed = false,
  }) {
    final t = context.t;
    final (:entry, :author, :changes) = shown;
    final lines = [
      if (author != null) profileDisplayName(t, author.name),
      ..._values(tally, entry, decimal),
      if (entry.line.isNotEmpty) entry.line,
    ];
    final tile = ListTile(
      key: ValueKey('tally-entry-${entry.id}'),
      contentPadding: EdgeInsets.zero,
      leading: author == null ? null : ProfileAvatar(profile: author, size: 32),
      title: Text(when.format(entry.at.toLocal())),
      subtitle: lines.isEmpty ? null : Text(lines.join('\n')),
      onTap: !removed && changes ? () => _editEntry(entry) : null,
    );
    if (removed) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          tile,
          // The control goes below its text.
          if (changes)
            TextButton.icon(
              key: ValueKey('tally-restore-${entry.id}'),
              onPressed: () => _restore(entry),
              icon: const Icon(Icons.restore),
              label: Text(t.restore),
            ),
        ],
      );
    }
    if (!changes) return tile;
    return GestureDetector(
      onLongPressStart: (details) => _entryMenu(entry, details.globalPosition),
      child: tile,
    );
  }

  /// A heading of a part of the card, a setting's or a chart's.
  Widget _heading(String text) => Padding(
    padding: const EdgeInsets.only(top: 16, bottom: 4),
    child: Text(text, style: Theme.of(context).textTheme.titleSmall),
  );

  @override
  Widget build(BuildContext context) {
    final tally = _tally;
    if (tally == null) return const SizedBox.shrink();
    final t = context.t;
    final scheme = Theme.of(context).colorScheme;
    final decimal = decimalSign(context);
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    final older = _entries.length - CardTally.shownEntries;
    final entries = older > 0 && !_allShown
        ? _entries.take(CardTally.shownEntries).toList()
        : _entries;
    return Material(
      key: const ValueKey('tally-card'),
      type: MaterialType.transparency,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_store.mayWriteIn(widget.noteId))
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  key: const ValueKey('tally-add-entry'),
                  onPressed: _addEntry,
                  icon: const Icon(Icons.add),
                  label: Text(t.addTallyEntry),
                ),
              ),
            ),
          if (_shared) ...[
            _heading(t.tallyShowEntries),
            RadioColumn<String>(
              key: const ValueKey('tally-whose'),
              value: _whose,
              onChanged: _setWhose,
              options: [
                RadioOption(
                  key: const ValueKey('tally-whose-mine'),
                  value: _Whose.mine,
                  title: Text(t.tallyEntriesMine),
                ),
                RadioOption(
                  key: const ValueKey('tally-whose-everyone'),
                  value: _Whose.everyone,
                  title: Text(t.tallyEntriesEveryone),
                ),
                for (final person in _authors)
                  RadioOption(
                    key: ValueKey('tally-whose-${person.id}'),
                    value: person.id,
                    title: Text(profileDisplayName(t, person.name)),
                    secondary: ProfileAvatar(profile: person, size: 24),
                  ),
              ],
            ),
          ],
          for (final field in tally.counting)
            Column(
              key: ValueKey('tally-chart-${field.id}'),
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _heading(switch (shownUnit(field.unit, _system)) {
                  '' => field.name,
                  final unit => '${field.name} ($unit)',
                }),
                TallyChart(
                  name: field.name,
                  buckets: _charts[field.id] ?? const [],
                  target: field.target,
                  period: tally.bar,
                  format: (value) =>
                      shownValue(value, field.unit, _system, decimal),
                ),
              ],
            ),
          Column(
            key: const ValueKey('tally-entries'),
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _heading(t.tallyEntries),
              if (entries.isEmpty) Text(t.tallyNoEntries),
              for (final shown in entries)
                _entryRow(tally, shown, when, decimal),
              // The older ones folded under the latest (#190).
              if (older > 0)
                Semantics(
                  button: true,
                  expanded: _allShown,
                  child: InkWell(
                    key: const ValueKey('tally-older'),
                    borderRadius: BorderRadius.circular(8),
                    onTap: _toggleAll,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                      child: Row(
                        children: [
                          Icon(
                            _allShown ? Icons.expand_less : Icons.expand_more,
                            color: scheme.onSurfaceVariant,
                          ),
                          const SizedBox(width: 8),
                          Flexible(
                            child: Text(
                              t.tallyOlderEntries(older),
                              style: Theme.of(context).textTheme.bodyMedium
                                  ?.copyWith(color: scheme.onSurfaceVariant),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
            ],
          ),
          if (_removed.isNotEmpty)
            ExpansionTile(
              key: const ValueKey('tally-removed'),
              shape: const Border(),
              collapsedShape: const Border(),
              tilePadding: EdgeInsets.zero,
              childrenPadding: EdgeInsets.zero,
              expandedCrossAxisAlignment: CrossAxisAlignment.start,
              leading: const Icon(Icons.delete_outline),
              title: Text(t.tallyRemovedEntries),
              initiallyExpanded: _removedOpen,
              onExpansionChanged: _setRemovedOpen,
              children: [
                for (final shown in _removed)
                  _entryRow(tally, shown, when, decimal, removed: true),
              ],
            ),
        ],
      ),
    );
  }
}

/// Asks once whether the entry [id] goes, with the verb on the button in
/// the error colour, and removes it; whether it did. It can be restored.
Future<bool> deleteTallyEntry(
  BuildContext context,
  Tallies tallies,
  String id,
) async {
  final sure = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        content: Text(t.deleteEntryQuestion),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.cancel),
          ),
          TextButton(
            key: const ValueKey('tally-delete-confirm'),
            style: TextButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.delete),
          ),
        ],
      );
    },
  );
  // A sync while the dialog was open may have taken it already.
  final entry = tallies.entryById(id);
  if (sure != true || entry == null || entry.isTrashed) return false;
  if (!tallies.store.mayChange(id)) return false;
  tallies.trashEntry(id);
  return true;
}

/// One field's chart (#264): a slim bar of a fixed width per day, week,
/// month or year, with gaps, the newest on the right and as many of the
/// latest as fit the width, never scrolled sideways (a horizontal swipe
/// only navigates; older values are in the entries); the target per bar
/// as a thin line; under it the days the bars cover and the highest
/// bar's value.
class TallyChart extends StatelessWidget {
  /// The field's name, for the chart's label.
  final String name;
  final List<TallyBucket> buckets;
  final double? target;
  final TallyPeriod period;

  /// A value in the base unit as shown, with its unit.
  final String Function(double value) format;

  /// How wide a bar is, with the gap after it.
  static const slot = 12.0;

  /// How wide the gap after a bar is.
  static const gap = 4.0;

  /// How tall the bars are at most.
  static const height = 120.0;

  const TallyChart({
    super.key,
    required this.name,
    required this.buckets,
    required this.target,
    required this.period,
    required this.format,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    if (buckets.isEmpty) return Text(t.tallyChartEmpty);
    final scheme = Theme.of(context).colorScheme;
    final locale = Localizations.localeOf(context).toString();
    final date = switch (period) {
      TallyPeriod.day || TallyPeriod.week => DateFormat.yMd(locale),
      TallyPeriod.month => DateFormat.yMMM(locale),
      TallyPeriod.year => DateFormat.y(locale),
    };
    return LayoutBuilder(
      builder: (context, constraints) {
        final fit = math.max(1, (constraints.maxWidth / slot).floor());
        final shown = buckets.length <= fit
            ? buckets
            : buckets.sublist(buckets.length - fit);
        final values = [for (final bucket in shown) ?bucket.value];
        final highest = values.isEmpty ? null : values.reduce(math.max);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Semantics(
              label: t.tallyChartLabel(name, shown.length),
              child: SizedBox(
                height: height,
                width: double.infinity,
                child: CustomPaint(
                  painter: _BarsPainter(
                    values: [for (final bucket in shown) bucket.value],
                    target: target,
                    bar: scheme.primary,
                    line: scheme.tertiary,
                    axis: scheme.outlineVariant,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              t.tallyChartRange(
                date.format(shown.first.start.wall),
                date.format(shown.last.end.wall),
              ),
              style: Theme.of(context).textTheme.bodySmall,
            ),
            if (highest != null)
              Text(
                t.tallyChartHighest(format(highest)),
                style: Theme.of(context).textTheme.bodySmall,
              ),
          ],
        );
      },
    );
  }
}

/// Bars of [TallyChart.slot] from a zero line, the last at the right
/// edge, scaled to the largest of the values and the target, negative
/// ones below it.
class _BarsPainter extends CustomPainter {
  final List<double?> values;
  final double? target;
  final Color bar;
  final Color line;
  final Color axis;

  _BarsPainter({
    required this.values,
    required this.target,
    required this.bar,
    required this.line,
    required this.axis,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final all = [...values.whereType<double>(), ?target];
    final top = all.fold(0.0, math.max);
    final bottom = all.fold(0.0, math.min);
    final span = top - bottom;
    double y(double value) =>
        span == 0 ? size.height : size.height * (top - value) / span;
    final zero = y(0);
    canvas.drawLine(
      Offset(0, zero),
      Offset(size.width, zero),
      Paint()
        ..color = axis
        ..strokeWidth = 1,
    );
    const slot = TallyChart.slot;
    final fill = Paint()..color = bar;
    for (final (i, value) in values.indexed) {
      if (value == null) continue;
      final right = size.width - (values.length - 1 - i) * slot;
      canvas.drawRect(
        Rect.fromLTRB(
          right - slot + TallyChart.gap,
          math.min(zero, y(value)),
          right,
          math.max(zero, y(value)),
        ),
        fill,
      );
    }
    if (target case final target?) {
      final at = y(target);
      canvas.drawLine(
        Offset(0, at),
        Offset(size.width, at),
        Paint()
          ..color = line
          ..strokeWidth = 1,
      );
    }
  }

  @override
  bool shouldRepaint(_BarsPainter old) =>
      old.target != target ||
      old.bar != bar ||
      old.line != line ||
      old.axis != axis ||
      !_same(old.values, values);

  static bool _same(List<double?> a, List<double?> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i] != b[i]) return false;
    }
    return true;
  }
}
