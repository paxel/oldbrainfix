import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'profile.dart';
import 'routine_editor_screen.dart';
import 'tally_entry_screen.dart';
import 'tally_field_screen.dart';

/// The system this Device shows the known units of Tallies in: the one
/// chosen in Settings, else the one of the region the system reports
/// (ADR 0028).
UnitSystem deviceUnitSystem(Store store) => store.unitSystem(
  WidgetsBinding.instance.platformDispatcher.locale.countryCode,
);

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

/// The Tally part of a Note's editor (#103), nothing while the Note is no
/// Tally: its fields, each opening the field page for who may change the
/// Note, with "Add entry" and "Add field"; in a Shared Area the choice of
/// whose entries show, the user's own by default; the length of a bar;
/// one chart per field shown; the entries, newest first, each tapped to
/// edit it and long-pressed for a menu at the finger; and a folded
/// "Removed entries" with Restore, closed until opened, remembered on
/// this Device. It follows the log as it changes.
class TallySection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  const TallySection({super.key, required this.notes, required this.noteId});

  @override
  State<TallySection> createState() => _TallySectionState();
}

class _TallySectionState extends State<TallySection> with ReloadsOnLogChange {
  /// The local setting that keeps "Removed entries" open.
  static const _removedOpenKey = 'tallyRemovedOpen';

  /// How many entries show before "Show more".
  static const _page = 30;

  late final _tallies = Tallies(widget.notes.store);
  late UnitSystem _system;
  Tally? _tally;
  var _shared = false;
  var _whose = _Whose.mine;
  var _period = TallyPeriod.day;
  var _shownCount = _page;
  List<_ShownEntry> _entries = const [];
  List<_ShownEntry> _removed = const [];

  /// The other people who wrote entries, for "Show entries".
  List<Profile> _authors = const [];
  Map<String, List<TallyBucket>> _charts = const {};
  late bool _removedOpen =
      widget.notes.store.localSetting(_removedOpenKey) == 'true';

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => setState(_load);

  @override
  void initState() {
    super.initState();
    final store = widget.notes.store;
    final area = store.areaOf(widget.noteId);
    _shared = area != null && store.isShared(area);
    // Outside a Shared Area every entry is the user's own.
    _whose = _shared ? _Whose.mine : _Whose.everyone;
    _load();
  }

  void _load() {
    final store = widget.notes.store;
    _system = deviceUnitSystem(store);
    final tally = _tally = _tallies.byNote(widget.noteId);
    if (tally == null) return;
    _ShownEntry shown(TallyEntry entry) => (
      entry: entry,
      author: store.authorOf(entry.id),
      changes: store.mayChange(entry.id),
    );
    final filter = _Whose.filter(_whose);
    _entries = [
      for (final entry in _tallies.entries(widget.noteId, filter: filter))
        shown(entry),
    ];
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
        _load();
        return;
      }
    }
    _charts = {
      for (final field in tally.shown)
        field.id: _tallies.chart(
          widget.noteId,
          field.id,
          _period,
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

  void _setPeriod(TallyPeriod period) => setState(() {
    _period = period;
    _load();
  });

  void _setRemovedOpen(bool open) {
    _removedOpen = open;
    widget.notes.store.setLocalSetting(_removedOpenKey, '$open');
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

  Future<void> _openField(String? fieldId) async {
    await openTallyField(context, widget.notes, widget.noteId, fieldId);
    if (!mounted) return;
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

  /// What [entry] holds for the fields shown, one after the other.
  String _values(Tally tally, TallyEntry entry, String decimal) => [
    for (final field in tally.shown)
      if (entry.values[field.id] case final value?)
        '${field.name} ${shownValue(value, field.unit, _system, decimal)}',
  ].join(' · ');

  Widget _entryRow(
    Tally tally,
    _ShownEntry shown,
    DateFormat when,
    String decimal, {
    bool removed = false,
  }) {
    final t = context.t;
    final (:entry, :author, :changes) = shown;
    final values = _values(tally, entry, decimal);
    final lines = [
      if (author != null) profileDisplayName(t, author.name),
      if (values.isNotEmpty) values,
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

  @override
  Widget build(BuildContext context) {
    final tally = _tally;
    if (tally == null) return const SizedBox.shrink();
    final t = context.t;
    final store = widget.notes.store;
    final decimal = decimalSign(context);
    final when = DateFormat.yMd(Localizations.localeOf(context).toString())
        .add_Hm();
    final adds = store.mayWriteIn(widget.noteId);
    final defines = store.mayChange(widget.noteId);
    final entries = _entries.take(_shownCount).toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        EditorPanel(
          key: const ValueKey('tally-fields'),
          heading: t.tallyHeading,
          child: Material(
            type: MaterialType.transparency,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final field in tally.fields)
                  ListTile(
                    key: ValueKey('tally-field-${field.id}'),
                    contentPadding: EdgeInsets.zero,
                    title: Text(field.name),
                    subtitle: Text(
                      [
                        if (shownUnit(field.unit, _system) case final unit
                            when unit.isNotEmpty)
                          unit,
                        if (field.target case final target?)
                          t.tallyTargetValue(
                            shownValue(target, field.unit, _system, decimal),
                          ),
                        if (field.hidden) t.tallyFieldHidden,
                      ].join(' · '),
                    ),
                    onTap: store.mayChange(field.noteId)
                        ? () => _openField(field.id)
                        : null,
                  ),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    if (adds)
                      FilledButton.tonalIcon(
                        key: const ValueKey('tally-add-entry'),
                        onPressed: _addEntry,
                        icon: const Icon(Icons.add),
                        label: Text(t.addTallyEntry),
                      ),
                    if (defines)
                      FilledButton.tonalIcon(
                        key: const ValueKey('tally-add-field'),
                        onPressed: () => _openField(null),
                        icon: const Icon(Icons.playlist_add),
                        label: Text(t.tallyAddField),
                      ),
                  ],
                ),
              ],
            ),
          ),
        ),
        if (_shared)
          EditorPanel(
            heading: t.tallyShowEntries,
            child: RadioColumn<String>(
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
          ),
        EditorPanel(
          heading: t.tallyPeriod,
          child: RadioColumn<TallyPeriod>(
            key: const ValueKey('tally-period'),
            value: _period,
            onChanged: _setPeriod,
            options: [
              for (final (period, label) in [
                (TallyPeriod.day, t.tallyPeriodDay),
                (TallyPeriod.week, t.tallyPeriodWeek),
                (TallyPeriod.month, t.tallyPeriodMonth),
                (TallyPeriod.year, t.tallyPeriodYear),
              ])
                RadioOption(
                  key: ValueKey('tally-period-${period.name}'),
                  value: period,
                  title: Text(label),
                ),
            ],
          ),
        ),
        for (final field in tally.shown)
          EditorPanel(
            key: ValueKey('tally-chart-${field.id}'),
            heading: switch (shownUnit(field.unit, _system)) {
              '' => field.name,
              final unit => '${field.name} ($unit)',
            },
            child: TallyChart(
              name: field.name,
              buckets: _charts[field.id] ?? const [],
              target: field.target,
              period: _period,
              format: (value) =>
                  shownValue(value, field.unit, _system, decimal),
            ),
          ),
        EditorPanel(
          key: const ValueKey('tally-entries'),
          heading: t.tallyEntries,
          child: Material(
            type: MaterialType.transparency,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (entries.isEmpty) Text(t.tallyNoEntries),
                for (final shown in entries)
                  _entryRow(tally, shown, when, decimal),
                if (_entries.length > entries.length)
                  TextButton(
                    key: const ValueKey('tally-more'),
                    onPressed: () => setState(() => _shownCount += _page),
                    child: Text(t.tallyMoreEntries),
                  ),
              ],
            ),
          ),
        ),
        if (_removed.isNotEmpty)
          EditorPanel(
            child: Material(
              type: MaterialType.transparency,
              child: ExpansionTile(
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
            ),
          ),
      ],
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

/// One field's chart: a bar per day, week, month or year, the latest
/// that fit the width, never scrolled sideways (a horizontal swipe only
/// navigates); the target as a line; under it the days the bars cover
/// and the highest bar's value.
class TallyChart extends StatelessWidget {
  /// The field's name, for the chart's label.
  final String name;
  final List<TallyBucket> buckets;
  final double? target;
  final TallyPeriod period;

  /// A value in the base unit as shown, with its unit.
  final String Function(double value) format;

  /// How wide a bar is at least, with the gap after it.
  static const slot = 10.0;

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
                    slots: math.max(shown.length, 7),
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

/// Bars from a zero line, scaled to the largest of the values and the
/// target, negative ones below it.
class _BarsPainter extends CustomPainter {
  final List<double?> values;
  final double? target;

  /// How many bars the width is cut into; the bars sit at the start.
  final int slots;
  final Color bar;
  final Color line;
  final Color axis;

  _BarsPainter({
    required this.values,
    required this.target,
    required this.slots,
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
    final width = size.width / slots;
    final fill = Paint()..color = bar;
    for (final (i, value) in values.indexed) {
      if (value == null) continue;
      final left = i * width;
      canvas.drawRect(
        Rect.fromLTRB(
          left + 1,
          math.min(zero, y(value)),
          left + width - 1,
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
          ..strokeWidth = 2,
      );
    }
  }

  @override
  bool shouldRepaint(_BarsPainter old) =>
      old.target != target ||
      old.slots != slots ||
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
