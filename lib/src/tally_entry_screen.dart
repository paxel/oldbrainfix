import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'note_editor_screen.dart';
import 'system_bars.dart';
import 'tally_view.dart';

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
  void Function(Map<String, double> values, String line) record,
) => _open(
  context,
  TallyEntryScreen(notes: notes, noteId: noteId, record: record),
);

Future<bool> _open(BuildContext context, Widget screen) async =>
    await Navigator.of(context)
        .push<bool>(MaterialPageRoute(builder: (_) => screen)) ??
    false;

/// One entry of a Tally on a full-screen page, Save in the top bar: a
/// value per field shown, a new entry's prefilled with the field's
/// target, so that saving the targets is one tap; the date it counts
/// for, today for a new one; and one line of text. A value left empty is
/// none. An entry that exists has Delete in the bar, which asks once; it
/// can be restored. Leaving without Save changes nothing.
///
/// With [record], a new entry is saved through it instead, such as for a
/// Session's pick (ADR 0029); it counts for now, so the date does not
/// show.
class TallyEntryScreen extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// The entry shown; null for a new one.
  final String? entryId;

  /// Saves a new entry's values, by field id in the base unit, and its
  /// line; null to add it to the Tally as any.
  final void Function(Map<String, double> values, String line)? record;

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
  late final List<TallyField> _fields = _tally?.shown ?? const [];
  late final _values = {
    for (final field in _fields) field.id: TextEditingController(),
  };

  /// The values as the page opened, to write only those that changed.
  final _asOpened = <String, String>{};
  late final _line = TextEditingController(text: _entry?.line ?? '');
  late final _focus = {for (final field in _fields) field.id: FocusNode()};
  final _lineFocus = FocusNode();

  /// The day the entry counts for, on this Device's clock.
  late DateTime _day = _dayOf(
    (_entry?.at ?? widget.notes.store.clock()).toLocal(),
  );
  late final DateTime _dayAsOpened;
  var _invalid = <String>{};
  var _empty = false;
  var _started = false;

  static DateTime _dayOf(DateTime wall) =>
      DateTime(wall.year, wall.month, wall.day);

  @override
  void initState() {
    super.initState();
    _dayAsOpened = _day;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    final decimal = decimalSign(context);
    final entry = _entry;
    for (final field in _fields) {
      final base = entry == null ? field.target : entry.values[field.id];
      final text = typedValue(base, field.unit, _system, decimal);
      _asOpened[field.id] = text;
      _values[field.id]?.text = text;
    }
  }

  @override
  void dispose() {
    for (final controller in _values.values) {
      controller.dispose();
    }
    for (final node in _focus.values) {
      node.dispose();
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

  void _save() {
    final parsed = <String, double?>{};
    final invalid = <String>{};
    for (final field in _fields) {
      final text = _values[field.id]?.text.trim() ?? '';
      if (text.isEmpty) {
        parsed[field.id] = null;
        continue;
      }
      final value = TallyUnits.parseShown(text, field.unit, _system);
      if (value == null) invalid.add(field.id);
      parsed[field.id] = value;
    }
    final line = _line.text.trim();
    final entry = _entry;
    final empty =
        entry == null &&
        line.isEmpty &&
        parsed.values.every((value) => value == null);
    setState(() {
      _invalid = invalid;
      _empty = empty && invalid.isEmpty;
    });
    // The field to fix comes into view.
    if (invalid.isNotEmpty) {
      if (_focus[_fields.firstWhere((f) => invalid.contains(f.id)).id]
          case final node?) {
        showField(node);
      }
      return;
    }
    if (empty) {
      showField(_lineFocus);
      return;
    }
    final values = {
      for (final MapEntry(:key, :value) in parsed.entries) key: ?value,
    };
    if (entry == null) {
      if (widget.record case final record?) {
        record(values, line);
      } else {
        _tallies.add(widget.noteId, values: values, line: line, at: _at());
      }
    } else {
      _tallies.editEntry(
        entry.id,
        // Only those typed anew: one shown rounded stays as it is.
        values: {
          for (final MapEntry(:key, :value) in parsed.entries)
            if ((_values[key]?.text.trim() ?? '') != _asOpened[key]) key: value,
        },
        line: line,
        at: _at(),
      );
    }
    Navigator.of(context).pop(true);
  }

  Future<void> _delete(TallyEntry entry) async {
    if (!await deleteTallyEntry(context, _tallies, entry.id)) return;
    if (!mounted) return;
    Navigator.of(context).pop(true);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final entry = _entry;
    final store = widget.notes.store;
    final date = DateFormat.yMMMEd(Localizations.localeOf(context).toString());
    return Scaffold(
      appBar: AppBar(
        title: Text(entry == null ? t.tallyNewEntry : t.tallyEntry),
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
            for (final field in _fields)
              EditorPanel(
                child: TextField(
                  key: ValueKey('tally-value-${field.id}'),
                  controller: _values[field.id],
                  focusNode: _focus[field.id],
                  keyboardType: const TextInputType.numberWithOptions(
                    signed: true,
                    decimal: true,
                  ),
                  decoration: InputDecoration(
                    labelText: field.name,
                    suffixText: shownUnit(field.unit, _system),
                    errorText: _invalid.contains(field.id)
                        ? t.tallyNotANumber
                        : null,
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
    );
  }
}
