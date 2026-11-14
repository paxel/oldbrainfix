import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'leave_saves.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'system_bars.dart';
import 'tally_view.dart';
import 'unit_system.dart';

/// "Add entry" (#103): the entry page for a new entry of the Tally of
/// [noteId]; whether one was saved.
Future<bool> addTallyEntry(BuildContext context, Notes notes, String noteId) =>
    _open(context, TallyEntryScreen(notes: notes, noteId: noteId));

/// The entry page for the entry [entryId] of the Tally of [noteId];
/// whether it was saved or deleted.
Future<bool> editTallyEntry(
  BuildContext context,
  Notes notes,
  String noteId,
  String entryId,
) => _open(
  context,
  TallyEntryScreen(notes: notes, noteId: noteId, entryId: entryId),
);

/// The entry page for a new entry of the Tally of [noteId] that [record]
/// saves, such as for a Session's pick; whether one was saved.
Future<bool> recordTallyEntry(
  BuildContext context,
  Notes notes,
  String noteId,
  TallyRecord record,
) => _open(
  context,
  TallyEntryScreen(notes: notes, noteId: noteId, record: record),
);

/// Saves a new entry: its first set's values and its Check fields' ticks
/// by field id in the base unit, its further sets (#264) and its line.
typedef TallyRecord = void Function(
  Map<String, double> values,
  List<Map<String, double>> more,
  String line,
);

Future<bool> _open(BuildContext context, Widget screen) async =>
    await Navigator.of(context)
        .push<bool>(MaterialPageRoute(builder: (_) => screen)) ??
    false;

/// One set of the entry page as it is typed: a value per number field.
class _SetDraft {
  final Map<String, TextEditingController> values;
  final Map<String, FocusNode> focus;

  _SetDraft(List<TallyField> fields, Map<String, String> texts)
    : values = {
        for (final field in fields)
          field.id: TextEditingController(text: texts[field.id] ?? ''),
      },
      focus = {for (final field in fields) field.id: FocusNode()};

  Map<String, String> get texts => {
    for (final MapEntry(:key, :value) in values.entries) key: value.text,
  };

  void dispose() {
    for (final controller in values.values) {
      controller.dispose();
    }
    for (final node in focus.values) {
      node.dispose();
    }
  }
}

/// One entry of a Tally on a full-screen page, Save in the top bar: a box
/// per Check field (#264), ticked once per entry above the sets; a value
/// per number field shown, a new entry's prefilled with the field's
/// target, so that saving the targets is one tap, or for a field without
/// one with the value of the user's latest entry that has one, selected
/// as the field takes the cursor so typing replaces it (#276). A Tally
/// that counts in sets shows each set as a row of those values with "Add
/// set" under them, which adds one that starts with the values of the set before;
/// a set after the first can be removed. Then the date it counts for,
/// today for a new one, and one line of text. A value left empty is
/// none. An entry that exists has Delete in the bar, which asks once; it
/// can be restored. Leaving by Back saves a
/// change as Save does (#275); one that can't be saved, a value that is
/// no number or a new entry left empty, asks once whether to discard it.
/// Left as it opened, targets and all, it writes nothing.
///
/// With [record], a new entry is saved through it instead, such as for a
/// Session's pick (ADR 0029); it counts for now, so the date does not
/// show.
class TallyEntryScreen extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// The entry shown; null for a new one.
  final String? entryId;

  /// Saves a new entry; null to add it to the Tally as any.
  final TallyRecord? record;

  const TallyEntryScreen({
    super.key,
    required this.notes,
    required this.noteId,
    this.entryId,
    this.record,
  });

  @override
  State<TallyEntryScreen> createState() => _TallyEntryScreenState();
}

class _TallyEntryScreenState extends State<TallyEntryScreen> {
  late final _tallies = Tallies(widget.notes.store);
  late final UnitSystem _system = deviceUnitSystem(widget.notes.store);
  late final Tally? _tally = _tallies.byNote(widget.noteId);
  late final TallyEntry? _entry = switch (widget.entryId) {
    final id? => _tallies.entryById(id),
    null => null,
  };
  late final List<TallyField> _fields = _tally?.numbers ?? const [];
  late final List<TallyField> _checks = _tally?.checks ?? const [];
  late final _ticked = {
    for (final field in _checks)
      field.id: _entry?.values.containsKey(field.id) ?? false,
  };
  final _sets = <_SetDraft>[];

  /// The sets as the page opened, to write only what changed.
  var _asOpened = const <Map<String, String>>[];
  late final Map<String, bool> _ticksAsOpened;
  late final _line = TextEditingController(text: _entry?.line ?? '');
  final _lineFocus = FocusNode();

  /// The day the entry counts for, on this Device's clock.
  late DateTime _day = _dayOf(
    (_entry?.at ?? widget.notes.store.clock()).toLocal(),
  );
  late final DateTime _dayAsOpened;

  /// The values that are no number, as `<set>:<field id>`.
  var _invalid = <String>{};
  var _empty = false;
  var _started = false;

  static DateTime _dayOf(DateTime wall) =>
      DateTime(wall.year, wall.month, wall.day);

  @override
  void initState() {
    super.initState();
    _dayAsOpened = _day;
    _ticksAsOpened = {..._ticked};
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    final decimal = decimalSign(context);
    final entry = _entry;
    final tally = _tally;
    final rows = entry == null || tally == null
        ? [_startValues(tally)]
        : tally.setsOf(entry);
    _asOpened = [
      for (final row in rows.isEmpty ? [const <String, double>{}] : rows)
        {
          for (final field in _fields)
            field.id: typedValue(row[field.id], field.unit, _system, decimal),
        },
    ];
    _sets.addAll([for (final texts in _asOpened) _SetDraft(_fields, texts)]);
    if (entry != null) return;
    final first = _sets.first;
    for (final field in _fields) {
      if (field.target != null) continue;
      final shown = _asOpened.first[field.id] ?? '';
      final controller = first.values[field.id];
      final node = first.focus[field.id];
      if (shown.isEmpty || controller == null || node == null) continue;
      node.addListener(() {
        if (node.hasFocus && controller.text == shown) {
          controller.selection = TextSelection(
            baseOffset: 0,
            extentOffset: shown.length,
          );
        }
      });
    }
  }

  /// What a new entry starts with (#276): each number field's target,
  /// and for one without, the value of the user's latest entry that has
  /// one, of its first set.
  Map<String, double> _startValues(Tally? tally) {
    if (tally == null) return const {};
    final values = tally.targets;
    final wanted = {
      for (final field in _fields)
        if (field.target == null) field.id,
    };
    if (wanted.isEmpty) return values;
    final latest = <String, double>{};
    for (final entry in _tallies.entries(
      widget.noteId,
      filter: const PersonFilter(me: true),
    )) {
      final sets = tally.setsOf(entry);
      if (sets.isEmpty) continue;
      for (final id in wanted) {
        if (sets.first[id] case final value?) {
          latest.putIfAbsent(id, () => value);
        }
      }
      if (latest.length == wanted.length) break;
    }
    return {...latest, ...values};
  }

  @override
  void dispose() {
    for (final set in _sets) {
      set.dispose();
    }
    _line.dispose();
    _lineFocus.dispose();
    super.dispose();
  }

  Future<void> _pickDay() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _day,
      firstDate: DateTime(1970),
      lastDate: DateTime(2200),
    );
    if (picked == null || !mounted) return;
    setState(() => _day = _dayOf(picked));
  }

  /// "Add set": a set that starts with the values of the set before.
  void _addSet() {
    final set = _SetDraft(_fields, _sets.last.texts);
    setState(() => _sets.add(set));
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _fields.isEmpty) return;
      if (set.focus[_fields.first.id] case final node?) showField(node);
    });
  }

  void _removeSet(int index) {
    final set = _sets[index];
    setState(() => _sets.removeAt(index));
    set.dispose();
  }

  /// The instant the entry counts for: its own, or now for a new one, on
  /// the day picked when that changed, at the same time of day.
  DateTime? _at() {
    if (_day == _dayAsOpened) return null;
    final time = (_entry?.at ?? widget.notes.store.clock()).toLocal();
    return DateTime(
      _day.year,
      _day.month,
      _day.day,
      time.hour,
      time.minute,
      time.second,
    );
  }

  /// Whether anything differs from the page as it opened (#275).
  bool _changed() {
    if (_day != _dayAsOpened) return true;
    if (_line.text.trim() != (_entry?.line ?? '').trim()) return true;
    for (final MapEntry(:key, :value) in _ticked.entries) {
      if (_ticksAsOpened[key] != value) return true;
    }
    if (_sets.length != _asOpened.length) return true;
    for (final (i, set) in _sets.indexed) {
      for (final field in _fields) {
        final typed = set.values[field.id]?.text.trim() ?? '';
        if (typed != (_asOpened[i][field.id] ?? '').trim()) return true;
      }
    }
    return false;
  }

  /// Whether Save would save: every value a number or empty, and a new
  /// entry not empty.
  bool _valid() {
    final (:parsed, :invalid) = _parse();
    return invalid.isEmpty && !_isEmpty(parsed);
  }

  /// The values of each set, null where empty or no number, and the
  /// values that are no number, as `<set>:<field id>`.
  ({List<Map<String, double?>> parsed, Set<String> invalid}) _parse() {
    final parsed = <Map<String, double?>>[];
    final invalid = <String>{};
    for (final (i, set) in _sets.indexed) {
      final row = <String, double?>{};
      for (final field in _fields) {
        final text = set.values[field.id]?.text.trim() ?? '';
        if (text.isEmpty) {
          row[field.id] = null;
          continue;
        }
        final value = TallyUnits.parseShown(text, field.unit, _system);
        if (value == null) invalid.add('$i:${field.id}');
        row[field.id] = value;
      }
      parsed.add(row);
    }
    return (parsed: parsed, invalid: invalid);
  }

  /// A new entry with nothing in it.
  bool _isEmpty(List<Map<String, double?>> parsed) =>
      _entry == null &&
      _line.text.trim().isEmpty &&
      !_ticked.values.any((on) => on) &&
      parsed.every((row) => row.values.every((value) => value == null));

  void _save() {
    final (:parsed, :invalid) = _parse();
    final line = _line.text.trim();
    final entry = _entry;
    final empty = _isEmpty(parsed);
    setState(() {
      _invalid = invalid;
      _empty = empty && invalid.isEmpty;
    });
    // The field to fix comes into view.
    if (invalid.isNotEmpty) {
      for (final (i, set) in _sets.indexed) {
        for (final field in _fields) {
          if (!invalid.contains('$i:${field.id}')) continue;
          if (set.focus[field.id] case final node?) showField(node);
          return;
        }
      }
      return;
    }
    if (empty) {
      showField(_lineFocus);
      return;
    }
    Map<String, double> held(Map<String, double?> row) => {
      for (final MapEntry(:key, :value) in row.entries) key: ?value,
    };
    final first = parsed.first;
    final more = [for (final row in parsed.skip(1)) held(row)];
    final ticks = {
      for (final MapEntry(:key, :value) in _ticked.entries)
        if (value) key: 1.0,
    };
    if (entry == null) {
      final values = {...held(first), ...ticks};
      if (widget.record case final record?) {
        record(values, more, line);
      } else {
        _tallies.add(
          widget.noteId,
          values: values,
          more: more,
          line: line,
          at: _at(),
        );
      }
    } else {
      final opened = _asOpened.first;
      _tallies.editEntry(
        entry.id,
        values: {
          // Only those typed anew: one shown rounded stays as it is.
          for (final MapEntry(:key, :value) in first.entries)
            if ((_sets.first.values[key]?.text.trim() ?? '') != opened[key])
              key: value,
          for (final field in _checks)
            if (_ticked[field.id] != entry.values.containsKey(field.id))
              field.id: (_ticked[field.id] ?? false) ? 1 : null,
        },
        // The further sets as typed, while there were or are several.
        more: _sets.length > 1 || _asOpened.length > 1 ? more : null,
        line: line,
        at: _at(),
      );
    }
    Navigator.of(context).pop(true);
  }

  /// Removes the entry without asking and closes; the announcement at
  /// the top undoes it (#268).
  void _delete(TallyEntry entry) {
    if (!deleteTallyEntry(context, _tallies, entry.id)) return;
    Navigator.of(context).pop(true);
  }

  /// The values of the set [index], under its heading while the Tally
  /// counts in sets or the entry holds several.
  List<Widget> _setFields(int index, bool inSets) {
    final t = context.t;
    final set = _sets[index];
    return [
      if (inSets)
        Padding(
          padding: const EdgeInsets.only(top: 8),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  t.tallySet(index + 1),
                  style: Theme.of(context).textTheme.titleSmall,
                ),
              ),
              if (index > 0)
                IconButton(
                  key: ValueKey('tally-set-remove-$index'),
                  tooltip: t.tallyRemoveSet(index + 1),
                  onPressed: () => _removeSet(index),
                  icon: const Icon(Icons.remove_circle_outline),
                ),
            ],
          ),
        ),
      for (final field in _fields)
        EditorPanel(
          child: TextField(
            key: ValueKey(
              index == 0
                  ? 'tally-value-${field.id}'
                  : 'tally-value-$index-${field.id}',
            ),
            controller: set.values[field.id],
            focusNode: set.focus[field.id],
            keyboardType: const TextInputType.numberWithOptions(
              signed: true,
              decimal: true,
            ),
            decoration: InputDecoration(
              labelText: field.name,
              suffixText: shownUnit(field.unit, _system),
              errorText: _invalid.contains('$index:${field.id}')
                  ? t.tallyNotANumber
                  : null,
            ),
          ),
        ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final entry = _entry;
    final store = widget.notes.store;
    final date = DateFormat.yMMMEd(Localizations.localeOf(context).toString());
    final countsInSets = _tally?.countsInSets ?? false;
    final inSets = countsInSets || _sets.length > 1;
    return LeaveSaves(
      changed: _changed,
      valid: _valid,
      save: _save,
      // Save shows what to fix without saving.
      onKeep: _save,
      child: Scaffold(
        appBar: pageBar(
          context,
          title: entry == null ? t.tallyNewEntry : t.tallyEntry,
          actions: [
            if (entry != null && store.mayChange(entry.id))
              IconButton(
                key: const ValueKey('tally-entry-trash'),
                tooltip: t.delete,
                onPressed: () => _delete(entry),
                icon: const Icon(Icons.delete_outline),
              ),
            TextButton(
              key: const ValueKey('tally-entry-save'),
              onPressed: _save,
              child: Text(t.save),
            ),
          ],
        ),
        // A short page, built whole, so a field to fix can come into view.
        body: SingleChildScrollView(
          padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (_checks.isNotEmpty)
                EditorPanel(
                  child: Material(
                    type: MaterialType.transparency,
                    child: Column(
                      children: [
                        for (final field in _checks)
                          CheckboxListTile(
                            key: ValueKey('tally-check-${field.id}'),
                            contentPadding: EdgeInsets.zero,
                            controlAffinity: ListTileControlAffinity.leading,
                            title: Text(field.name),
                            value: _ticked[field.id] ?? false,
                            onChanged: (on) =>
                                setState(() => _ticked[field.id] = on ?? false),
                          ),
                      ],
                    ),
                  ),
                ),
              for (var i = 0; i < _sets.length; i++) ..._setFields(i, inSets),
              if (countsInSets && _fields.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.only(bottom: 16),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: FilledButton.tonalIcon(
                      key: const ValueKey('tally-add-set'),
                      onPressed: _sets.length < TallyKeys.maxSets
                          ? _addSet
                          : null,
                      icon: const Icon(Icons.add),
                      label: Text(t.tallyAddSet),
                    ),
                  ),
                ),
              if (entry != null || widget.record == null)
                EditorPanel(
                  heading: t.tallyEntryDate,
                  child: Material(
                    type: MaterialType.transparency,
                    child: ListTile(
                      key: const ValueKey('tally-entry-date'),
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.event_outlined),
                      title: Text(date.format(_day)),
                      onTap: _pickDay,
                    ),
                  ),
                ),
              EditorPanel(
                child: TextField(
                  key: const ValueKey('tally-entry-line'),
                  controller: _line,
                  focusNode: _lineFocus,
                  textCapitalization: TextCapitalization.sentences,
                  decoration: InputDecoration(
                    labelText: t.tallyEntryLine,
                    errorText: _empty ? t.tallyEntryEmpty : null,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
