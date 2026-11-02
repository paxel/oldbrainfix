import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'capture_choices.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'system_bars.dart';
import 'tally_unit_match.dart';
import 'tally_view.dart';

/// "Make it a Tally" (#103): the page for the first field of the Note
/// [noteId]; whether it was saved, which makes the Note a Tally.
Future<bool> makeTally(BuildContext context, Notes notes, String noteId) =>
    openTallyField(context, notes, noteId, null);

/// "Make it a Tally" on a capture screen (#113): the page for the first
/// field of the Note being captured, which is not written yet; the field
/// as saved, for Capture to write with the Note, or null when it was
/// left without Save.
Future<TallyFieldChoice?> chooseTallyField(BuildContext context, Notes notes) =>
    Navigator.of(context).push(
      MaterialPageRoute<TallyFieldChoice>(
        builder: (_) => TallyFieldScreen(notes: notes),
      ),
    );

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
/// name; its unit, typed into one field under which the known units it
/// may mean show as chips in this Device's system, a known one or one as
/// typed, only of its kind once entries hold values for it; its target,
/// optional, prefilled in each new entry; and how a day with several
/// entries counts, their sum or the largest, which follows the kind of
/// unit until it is picked. In a Tally with another field it may multiply
/// (#200), "Multiplies" offers Nothing and those fields; a multiplier has
/// no target and no rule, so they leave the page while one is picked. A
/// field that exists has "Hide field" at the
/// bottom, or "Show field again" when hidden; its values stay stored.
/// Leaving without Save changes nothing.
class TallyFieldScreen extends StatefulWidget {
  final Notes notes;

  /// The Note whose Tally it is; null for the Note being captured, not
  /// written yet, whose new field Save pops as a [TallyFieldChoice].
  final String? noteId;

  /// The field shown; null for a new one.
  final String? fieldId;

  const TallyFieldScreen({
    super.key,
    required this.notes,
    this.noteId,
    this.fieldId,
  });

  @override
  State<TallyFieldScreen> createState() => _TallyFieldScreenState();
}

class _TallyFieldScreenState extends State<TallyFieldScreen> {
  late final _tallies = Tallies(widget.notes.store);
  late final UnitSystem _system = deviceUnitSystem(widget.notes.store);
  late final TallyField? _field = switch ((widget.noteId, widget.fieldId)) {
    (final noteId?, final fieldId?) => _tallies.byNote(noteId)?.field(fieldId),
    _ => null,
  };
  late final _name = TextEditingController(text: _field?.name ?? '');

  /// The unit as the page opened, shown in this Device's system.
  late final String _unitAsOpened = switch (_field) {
    final field? => shownUnit(field.unit, _system),
    null => '',
  };
  late final _unitText = TextEditingController(text: _unitAsOpened);

  /// Once entries hold values, only units of the field's kind are left.
  late final UnitKind? _fixed = switch (_field) {
    final field? when field.hasEntries => field.kind,
    _ => null,
  };

  /// The names the known units are matched by, in every language.
  late final _unitNames = unitNames(
    AppLocalizations.supportedLocales.map(lookupAppLocalizations),
    _system,
  );

  /// The known units offered for the unit typed so far.
  late var _matches = _match();

  /// The code of the known unit a chip put into the field, until it is
  /// typed in again.
  String? _picked;
  late final _target = TextEditingController();
  final _nameFocus = FocusNode();
  final _targetFocus = FocusNode();
  final _unitFocus = FocusNode();
  late TallyRule _rule = _field?.rule ?? TallyRule.sum;

  /// The fields this one may multiply, those of a written Note's Tally.
  late final List<TallyField> _multipliable = switch (widget.noteId) {
    final noteId? =>
      _tallies.byNote(noteId)?.multipliable(widget.fieldId) ?? const [],
    null => const [],
  };

  /// The id of the field it multiplies, or [_nothing].
  late String _multiplies = _field?.multiplies ?? _nothing;

  /// What [_multiplies] holds while it multiplies nothing.
  static const _nothing = '';

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
    _unitText.dispose();
    _target.dispose();
    _nameFocus.dispose();
    _targetFocus.dispose();
    _unitFocus.dispose();
    super.dispose();
  }

  /// The unit a chip picked, else the unit typed: the field's own while
  /// the text is as it opened, so a unit of the user's own that reads
  /// like a known symbol stays theirs.
  String get _chosenUnit =>
      _picked ??
      switch (_field) {
        final field? when _unitText.text == _unitAsOpened => field.unit,
        _ => unitOfTyped(_unitText.text, _system),
      };

  /// Every known unit left while the unit is as the page opened, so
  /// another one is a tap away; then those the text typed may mean.
  List<KnownUnit> _match() => matchingUnits(
    _unitText.text == _unitAsOpened ? '' : _unitText.text,
    _system,
    _unitNames,
    kind: _fixed,
  );

  /// The unit typed changed; its kind may change the rule.
  void _typedUnit() => setState(() {
    _matches = _match();
    if (!_rulePicked && _field == null) {
      _rule = TallyUnits.defaultRule(TallyUnits.kindOf(_chosenUnit));
    }
  });

  /// A chip picks the known unit [known], shown in this Device's system.
  void _pickUnit(KnownUnit known) {
    final shown = shownUnit(known.code, _system);
    _picked = known.code;
    _unitText.value = TextEditingValue(
      text: shown,
      selection: TextSelection.collapsed(offset: shown.length),
    );
    _typedUnit();
  }

  void _pickRule(TallyRule rule) => setState(() {
    _rule = rule;
    _rulePicked = true;
  });

  void _save() {
    final name = _name.text.trim();
    final unit = _chosenUnit;
    // A multiplier has no target (#200).
    final multiplies = _multiplies == _nothing ? null : _multiplies;
    final targetText = multiplies == null ? _target.text.trim() : '';
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
      showField(_unitFocus);
      return;
    }
    if (_targetInvalid) {
      showField(_targetFocus);
      return;
    }
    final noteId = widget.noteId;
    if (noteId == null) {
      Navigator.of(context).pop<TallyFieldChoice>((
        name: name,
        unit: unit,
        target: target,
        rule: _rule,
      ));
      return;
    }
    if (field == null) {
      _tallies.addField(
        noteId,
        name: name,
        unit: unit,
        target: target,
        rule: _rule,
        multiplies: multiplies,
      );
    } else {
      if (name != field.name) _tallies.renameField(noteId, field.id, name);
      final unitChanged = unit != field.unit;
      if (unitChanged) _tallies.setUnit(noteId, field.id, unit);
      if (multiplies != field.multiplies) {
        _tallies.setMultiplies(noteId, field.id, multiplies);
      }
      // A multiplier's target and rule stay as they were, for when it is
      // a field of its own again.
      if (multiplies == null) {
        // Read again in the unit now shown next to it.
        if (unitChanged || targetText != _targetAsOpened) {
          _tallies.setTarget(noteId, field.id, target);
        }
        if (_rule != field.rule) _tallies.setRule(noteId, field.id, _rule);
      }
    }
    Navigator.of(context).pop(true);
  }

  void _setHidden(TallyField field, bool hidden) {
    // A field shown is one of a written Note.
    final noteId = widget.noteId;
    if (noteId == null) return;
    if (hidden) {
      _tallies.hideField(noteId, field.id);
    } else {
      _tallies.restoreField(noteId, field.id);
    }
    Navigator.of(context).pop(true);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final field = _field;
    final fixed = _fixed;
    return Scaffold(
      appBar: pageBar(
        context,
        title: field == null ? t.tallyNewField : t.tallyField,
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
                  TextField(
                    key: const ValueKey('tally-unit-text'),
                    controller: _unitText,
                    focusNode: _unitFocus,
                    decoration: InputDecoration(
                      hintText: t.tallyUnitHint,
                      // A unit of the user's own takes no known one.
                      helperText: fixed == UnitKind.typed
                          ? null
                          : t.tallyUnitHelper,
                      helperMaxLines: 3,
                      errorText: _unitInvalid ? t.tallyUnitFixed : null,
                      errorMaxLines: 3,
                    ),
                    onChanged: (_) {
                      _picked = null;
                      _typedUnit();
                    },
                  ),
                  UnitSuggestions(
                    matches: _matches,
                    system: _system,
                    onPick: _pickUnit,
                  ),
                ],
              ),
            ),
            if (_multipliable.isNotEmpty || _field?.multiplies != null)
              EditorPanel(
                heading: t.tallyMultiplies,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    RadioColumn<String>(
                      key: const ValueKey('tally-multiplies'),
                      value: _multiplies,
                      onChanged: (id) => setState(() => _multiplies = id),
                      options: [
                        RadioOption(
                          key: const ValueKey('tally-multiplies-nothing'),
                          value: _nothing,
                          title: Text(t.tallyMultipliesNothing),
                        ),
                        for (final other in _multipliable)
                          RadioOption(
                            key: ValueKey('tally-multiplies-${other.id}'),
                            value: other.id,
                            title: Text(other.name),
                          ),
                      ],
                    ),
                    Text(
                      t.tallyMultipliesDetail,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
            if (_multiplies == _nothing) ...[
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
            ],
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

/// The known units [matches] under a Tally field's unit (#159), each a
/// chip in this Device's [system] that fills in the field through
/// [onPick]; suggestions, not a choice of their own. Nothing for none.
class UnitSuggestions extends StatelessWidget {
  final List<KnownUnit> matches;
  final UnitSystem system;
  final ValueChanged<KnownUnit> onPick;

  const UnitSuggestions({
    super.key,
    required this.matches,
    required this.system,
    required this.onPick,
  });

  String _kindName(AppLocalizations t, UnitKind kind) => switch (kind) {
    UnitKind.mass => t.unitKindMass,
    UnitKind.distance => t.unitKindDistance,
    UnitKind.speed => t.unitKindSpeed,
    UnitKind.temperature => t.unitKindTemperature,
    UnitKind.angle => t.unitKindAngle,
    UnitKind.time => t.unitKindTime,
    UnitKind.typed => t.tallyUnitTyped,
  };

  @override
  Widget build(BuildContext context) {
    if (matches.isEmpty) return const SizedBox.shrink();
    final t = context.t;
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Wrap(
        key: const ValueKey('tally-unit'),
        spacing: 8,
        runSpacing: 4,
        children: [
          for (final known in matches)
            ActionChip(
              key: ValueKey('tally-unit-${known.code}'),
              label: Text(shownUnit(known.code, system)),
              tooltip: _kindName(t, known.kind),
              onPressed: () => onPick(known),
            ),
        ],
      ),
    );
  }
}
