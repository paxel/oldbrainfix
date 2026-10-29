import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'system_bars.dart';
import 'tally_view.dart';

/// "Make it a Tally" (#103): the page for the first field of the Note
/// [noteId]; whether it was saved, which makes the Note a Tally.
Future<bool> makeTally(BuildContext context, Notes notes, String noteId) =>
    openTallyField(context, notes, noteId, null);

/// The page of the field [fieldId] of the Tally of [noteId], or of a new
/// one when null; whether it saved or hid something.
Future<bool> openTallyField(
  BuildContext context,
  Notes notes,
  String noteId,
  String? fieldId,
) async =>
    await Navigator.of(context).push<bool>(
      MaterialPageRoute(
        builder: (_) =>
            TallyFieldScreen(notes: notes, noteId: noteId, fieldId: fieldId),
      ),
    ) ??
    false;

/// One field of a Tally on a full-screen page, Save in the top bar: its
/// name; its unit, a known one shown in this Device's system or one as
/// typed, only of its kind once entries hold values for it; its target,
/// optional, prefilled in each new entry; and how a day with several
/// entries counts, their sum or the largest, which follows the kind of
/// unit until it is picked. A field that exists has "Hide field" at the
/// bottom, or "Show field again" when hidden; its values stay stored.
/// Leaving without Save changes nothing.
class TallyFieldScreen extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// The field shown; null for a new one.
  final String? fieldId;

  const TallyFieldScreen({
    super.key,
    required this.notes,
    required this.noteId,
    this.fieldId,
  });

  @override
  State<TallyFieldScreen> createState() => _TallyFieldScreenState();
}

class _TallyFieldScreenState extends State<TallyFieldScreen> {
  /// The unit option of a unit as typed.
  static const _typedChoice = '\u0000typed';

  late final _tallies = Tallies(widget.notes.store);
  late final UnitSystem _system = deviceUnitSystem(widget.notes.store);
  late final TallyField? _field = widget.fieldId == null
      ? null
      : _tallies.byNote(widget.noteId)?.field(widget.fieldId ?? '');
  late final _name = TextEditingController(text: _field?.name ?? '');
  late String _unit = switch (_field) {
    final field? when TallyUnits.byCode(field.unit) != null => field.unit,
    _ => _typedChoice,
  };
  late final _typed = TextEditingController(
    text: _unit == _typedChoice ? _field?.unit ?? '' : '',
  );
  late final _target = TextEditingController();
  final _nameFocus = FocusNode();
  final _targetFocus = FocusNode();
  final _typedFocus = FocusNode();
  late TallyRule _rule = _field?.rule ?? TallyRule.sum;

  /// Whether the rule was picked here; until then it follows the unit.
  var _rulePicked = false;

  /// The target as the page opened, to write it only when it changed.
  var _targetAsOpened = '';
  var _nameMissing = false;
  var _targetInvalid = false;

  /// Whether the unit typed is of another kind than the entries fix.
  var _unitInvalid = false;
  var _started = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    final field = _field;
    if (field == null) return;
    _targetAsOpened = typedValue(
      field.target,
      field.unit,
      _system,
      decimalSign(context),
    );
    _target.text = _targetAsOpened;
  }

  @override
  void dispose() {
    _name.dispose();
    _typed.dispose();
    _target.dispose();
    _nameFocus.dispose();
    _targetFocus.dispose();
    _typedFocus.dispose();
    super.dispose();
  }

  String get _chosenUnit => _unit == _typedChoice ? _typed.text.trim() : _unit;

  void _pickUnit(String unit) => setState(() {
    _unit = unit;
    if (!_rulePicked && _field == null) {
      _rule = TallyUnits.defaultRule(TallyUnits.kindOf(_chosenUnit));
    }
  });

  void _pickRule(TallyRule rule) => setState(() {
    _rule = rule;
    _rulePicked = true;
  });

  void _save() {
    final name = _name.text.trim();
    final unit = _chosenUnit;
    final targetText = _target.text.trim();
    final target = targetText.isEmpty
        ? null
        : TallyUnits.parseShown(targetText, unit, _system);
    final field = _field;
    setState(() {
      _nameMissing = name.isEmpty;
      _targetInvalid = targetText.isNotEmpty && target == null;
      _unitInvalid =
          field != null &&
          field.hasEntries &&
          TallyUnits.kindOf(unit) != field.kind;
    });
    // The field to fix comes into view.
    if (_nameMissing) {
      showField(_nameFocus);
      return;
    }
    if (_unitInvalid) {
      showField(_typedFocus);
      return;
    }
    if (_targetInvalid) {
      showField(_targetFocus);
      return;
    }
    if (field == null) {
      _tallies.addField(
        widget.noteId,
        name: name,
        unit: unit,
        target: target,
        rule: _rule,
      );
    } else {
      final noteId = widget.noteId;
      if (name != field.name) _tallies.renameField(noteId, field.id, name);
      final unitChanged = unit != field.unit;
      if (unitChanged) _tallies.setUnit(noteId, field.id, unit);
      // Read again in the unit now shown next to it.
      if (unitChanged || targetText != _targetAsOpened) {
        _tallies.setTarget(noteId, field.id, target);
      }
      if (_rule != field.rule) _tallies.setRule(noteId, field.id, _rule);
    }
    Navigator.of(context).pop(true);
  }

  void _setHidden(TallyField field, bool hidden) {
    if (hidden) {
      _tallies.hideField(widget.noteId, field.id);
    } else {
      _tallies.restoreField(widget.noteId, field.id);
    }
    Navigator.of(context).pop(true);
  }

  String _kindName(UnitKind kind) {
    final t = context.t;
    return switch (kind) {
      UnitKind.mass => t.unitKindMass,
      UnitKind.distance => t.unitKindDistance,
      UnitKind.speed => t.unitKindSpeed,
      UnitKind.temperature => t.unitKindTemperature,
      UnitKind.angle => t.unitKindAngle,
      UnitKind.time => t.unitKindTime,
      UnitKind.typed => t.tallyUnitTyped,
    };
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final field = _field;
    // Once entries hold values, only units of the field's kind are left.
    final fixed = field != null && field.hasEntries ? field.kind : null;
    return Scaffold(
      appBar: AppBar(
        title: Text(field == null ? t.tallyNewField : t.tallyField),
        actions: [
          TextButton(
            key: const ValueKey('tally-field-save'),
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
            EditorPanel(
              child: TextField(
                key: const ValueKey('tally-field-name'),
                controller: _name,
                focusNode: _nameFocus,
                autofocus: field == null,
                textCapitalization: TextCapitalization.sentences,
                decoration: InputDecoration(
                  labelText: t.tallyFieldName,
                  hintText: t.tallyFieldNameHint,
                  errorText: _nameMissing ? t.tallyFieldNameMissing : null,
                ),
              ),
            ),
            EditorPanel(
              heading: t.tallyUnit,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (fixed != null) Text(t.tallyUnitFixed),
                  RadioColumn<String>(
                    key: const ValueKey('tally-unit'),
                    value: _unit,
                    onChanged: _pickUnit,
                    options: [
                      for (final known in TallyUnits.known)
                        if (fixed == null || known.kind == fixed)
                          RadioOption(
                            key: ValueKey('tally-unit-${known.code}'),
                            value: known.code,
                            title: Text(shownUnit(known.code, _system)),
                            subtitle: Text(_kindName(known.kind)),
                          ),
                      if (fixed == null || fixed == UnitKind.typed)
                        RadioOption(
                          key: const ValueKey('tally-unit-typed'),
                          value: _typedChoice,
                          title: Text(t.tallyUnitTyped),
                        ),
                    ],
                  ),
                  if (_unit == _typedChoice)
                    TextField(
                      key: const ValueKey('tally-unit-text'),
                      controller: _typed,
                      focusNode: _typedFocus,
                      decoration: InputDecoration(
                        labelText: t.tallyUnitTyped,
                        hintText: t.tallyUnitTypedHint,
                        errorText: _unitInvalid ? t.tallyUnitFixed : null,
                      ),
                      // A typed known code is that unit; its kind may
                      // change the rule.
                      onChanged: (_) => _pickUnit(_typedChoice),
                    ),
                ],
              ),
            ),
            EditorPanel(
              child: TextField(
                key: const ValueKey('tally-field-target'),
                controller: _target,
                focusNode: _targetFocus,
                keyboardType: const TextInputType.numberWithOptions(
                  signed: true,
                  decimal: true,
                ),
                decoration: InputDecoration(
                  labelText: t.tallyTarget,
                  helperText: t.tallyTargetHint,
                  suffixText: shownUnit(_chosenUnit, _system),
                  errorText: _targetInvalid ? t.tallyNotANumber : null,
                ),
              ),
            ),
            EditorPanel(
              heading: t.tallyRule,
              child: RadioColumn<TallyRule>(
                key: const ValueKey('tally-rule'),
                value: _rule,
                onChanged: _pickRule,
                options: [
                  RadioOption(
                    key: const ValueKey('tally-rule-sum'),
                    value: TallyRule.sum,
                    title: Text(t.tallyRuleSum),
                  ),
                  RadioOption(
                    key: const ValueKey('tally-rule-max'),
                    value: TallyRule.max,
                    title: Text(t.tallyRuleMax),
                  ),
                ],
              ),
            ),
            // A secondary state change: a row at the bottom.
            if (field != null)
              ListTile(
                key: const ValueKey('tally-field-hide'),
                leading: Icon(
                  field.hidden
                      ? Icons.visibility_outlined
                      : Icons.visibility_off_outlined,
                ),
                title: Text(field.hidden ? t.tallyShowField : t.tallyHideField),
                subtitle: Text(t.tallyHideFieldDetail),
                onTap: () => _setHidden(field, !field.hidden),
              ),
          ],
        ),
      ),
    );
  }
}
