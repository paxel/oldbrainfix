import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'setting_badges.dart';
import 'system_bars.dart';
import 'tally_field_screen.dart';
import 'tally_unit_match.dart';
import 'tally_view.dart';

/// One field of the New Tally page as it is typed: its name, its unit as
/// typed and the known one a chip picked, its rule, which follows the
/// unit until it is picked, and its target as typed.
class _FieldDraft {
  final name = TextEditingController();
  final focus = FocusNode();
  var unitText = '';

  /// The code of the known unit a chip put into the unit, until it is
  /// typed in again.
  String? picked;
  var rule = TallyRule.sum;
  var rulePicked = false;
  var targetText = '';

  /// Set by a Save that found the target no number.
  var targetInvalid = false;

  /// The unit as it is stored.
  String unit(UnitSystem system) => picked ?? unitOfTyped(unitText, system);

  /// The target in the base unit; null for none or no number.
  double? target(UnitSystem system) {
    final text = targetText.trim();
    return text.isEmpty
        ? null
        : TallyUnits.parseShown(text, unit(system), system);
  }

  void dispose() {
    name.dispose();
    focus.dispose();
  }
}

/// A new Tally made from the browser's "+" (#161), on one page with
/// Save in the top bar: its name, then a row per field, one at first,
/// and "Add field" for the next. A field's row is its name and its
/// settings as badges, how a day with several entries counts ("Σ Sum"),
/// its unit and its target, each opening its small editor; the unit is
/// typed with the known units it may mean as suggestions (#159), and the
/// rule follows its kind until it is picked. Save files the Tally into
/// [place], Unsorted for none, with every row that has a name; rows left
/// without one are dropped. It pops the new Tally's Note id; leaving
/// without Save makes nothing.
class TallyFormScreen extends StatefulWidget {
  final Notes notes;

  /// The Area or Topic it goes into; null for none.
  final String? place;

  /// What its name starts with, such as the tags a Session's pool is
  /// filtered by (#220).
  final String text;

  const TallyFormScreen({
    super.key,
    required this.notes,
    this.place,
    this.text = '',
  });

  @override
  State<TallyFormScreen> createState() => _TallyFormScreenState();
}

class _TallyFormScreenState extends State<TallyFormScreen> {
  late final UnitSystem _system = deviceUnitSystem(widget.notes.store);

  /// The names the known units are matched by, in every language.
  late final _unitNames = unitNames(
    AppLocalizations.supportedLocales.map(lookupAppLocalizations),
    _system,
  );
  late final _name = TextEditingController(text: widget.text);
  final _nameFocus = FocusNode();
  final _fields = [_FieldDraft()];
  var _nameMissing = false;
  var _fieldsMissing = false;

  @override
  void dispose() {
    _name.dispose();
    _nameFocus.dispose();
    for (final field in _fields) {
      field.dispose();
    }
    super.dispose();
  }

  void _addField() {
    final field = _FieldDraft();
    setState(() => _fields.add(field));
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) field.focus.requestFocus();
    });
  }

  void _save() {
    final name = _name.text.trim();
    final kept = [
      for (final field in _fields)
        if (field.name.text.trim().isNotEmpty) field,
    ];
    setState(() {
      _nameMissing = name.isEmpty;
      _fieldsMissing = kept.isEmpty;
      for (final field in _fields) {
        field.targetInvalid =
            kept.contains(field) &&
            field.targetText.trim().isNotEmpty &&
            field.target(_system) == null;
      }
    });
    // The field to fix comes into view.
    if (_nameMissing) {
      showField(_nameFocus);
      return;
    }
    if (_fieldsMissing) {
      showField(_fields.first.focus);
      return;
    }
    for (final field in kept) {
      if (field.targetInvalid) {
        showField(field.focus);
        return;
      }
    }
    final notes = widget.notes;
    final tallies = Tallies(notes.store);
    final id = notes.store.batch(() {
      final note = notes.capture(name, place: widget.place);
      for (final field in kept) {
        tallies.addField(
          note.id,
          name: field.name.text.trim(),
          unit: field.unit(_system),
          target: field.target(_system),
          rule: field.rule,
        );
      }
      return note.id;
    });
    Navigator.of(context).pop(id);
  }

  /// The rule's editor: its two choices as radio buttons; a pick closes
  /// it.
  Future<void> _editRule(_FieldDraft field) async {
    final t = context.t;
    final picked = await showDialog<TallyRule>(
      context: context,
      builder: (context) => BadgeDialog(
        setting: t.tallyRule,
        icon: Icons.functions,
        content: ChoiceChips<TallyRule>(
          key: const ValueKey('tally-rule'),
          value: field.rule,
          onChanged: (rule) => Navigator.of(context).pop(rule),
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
    );
    if (!mounted || picked == null) return;
    setState(() {
      field.rule = picked;
      field.rulePicked = true;
    });
  }

  /// The unit's editor: the unit typed, with the known units it may mean
  /// as suggestions under it. It changes the field as it is typed.
  Future<void> _editUnit(_FieldDraft field) => showDialog<void>(
    context: context,
    builder: (context) => _UnitDialog(
      field: field,
      system: _system,
      names: _unitNames,
      onChanged: () => setState(() {
        if (!field.rulePicked) {
          field.rule = TallyUnits.defaultRule(
            TallyUnits.kindOf(field.unit(_system)),
          );
        }
      }),
    ),
  );

  /// The target's editor: a number in the field's unit, or nothing.
  Future<void> _editTarget(_FieldDraft field) => showDialog<void>(
    context: context,
    builder: (context) => _TargetDialog(
      field: field,
      system: _system,
      onChanged: () => setState(() => field.targetInvalid = false),
    ),
  );

  Widget _fieldRow(AppLocalizations t, int index, _FieldDraft field) {
    final unit = shownUnit(field.unit(_system), _system);
    final target = field.targetText.trim();
    final error = TextStyle(color: Theme.of(context).colorScheme.error);
    return EditorPanel(
      key: ValueKey('tally-form-field-$index'),
      heading: index == 0 ? t.tallyFields : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            key: ValueKey('tally-form-field-name-$index'),
            controller: field.name,
            focusNode: field.focus,
            textCapitalization: TextCapitalization.sentences,
            decoration: InputDecoration(
              labelText: t.tallyFieldName,
              hintText: t.tallyFieldNameHint,
              errorText: index == 0 && _fieldsMissing
                  ? t.tallyFieldsMissing
                  : null,
            ),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              SettingBadge(
                key: ValueKey('tally-form-rule-$index'),
                setting: t.tallyRule,
                icon: Icons.functions,
                value: field.rule == TallyRule.sum
                    ? t.tallyRuleSumShort
                    : t.tallyRuleMaxShort,
                valueIcon: tallyRuleIcon(field.rule),
                onTap: (_) => _editRule(field),
              ),
              SettingBadge(
                key: ValueKey('tally-form-unit-$index'),
                setting: t.tallyUnit,
                icon: Icons.straighten,
                value: unit.isEmpty ? t.tallyNoUnit : unit,
                onTap: (_) => _editUnit(field),
              ),
              SettingBadge(
                key: ValueKey('tally-form-target-$index'),
                setting: t.tallyTargetBadge,
                icon: Icons.flag_outlined,
                value: target.isEmpty
                    ? t.tallyNoTarget
                    : unit.isEmpty
                    ? target
                    : '$target $unit',
                onTap: (_) => _editTarget(field),
              ),
            ],
          ),
          if (field.targetInvalid)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(
                t.tallyNotANumber,
                key: ValueKey('tally-form-target-error-$index'),
                style: error,
              ),
            ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.newTally,
        actions: [
          TextButton(
            key: const ValueKey('tally-form-save'),
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
                key: const ValueKey('tally-form-name'),
                controller: _name,
                focusNode: _nameFocus,
                autofocus: true,
                textCapitalization: TextCapitalization.sentences,
                decoration: InputDecoration(
                  labelText: t.kindName,
                  errorText: _nameMissing ? t.kindNameMissing : null,
                ),
              ),
            ),
            for (final (index, field) in _fields.indexed)
              _fieldRow(t, index, field),
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.tonalIcon(
                key: const ValueKey('tally-form-add-field'),
                onPressed: _addField,
                icon: const Icon(Icons.add),
                label: Text(t.tallyAddField),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The unit badge's editor on the New Tally page.
class _UnitDialog extends StatefulWidget {
  final _FieldDraft field;
  final UnitSystem system;
  final Map<String, List<String>> names;
  final VoidCallback onChanged;

  const _UnitDialog({
    required this.field,
    required this.system,
    required this.names,
    required this.onChanged,
  });

  @override
  State<_UnitDialog> createState() => _UnitDialogState();
}

class _UnitDialogState extends State<_UnitDialog> {
  late final _text = TextEditingController(text: widget.field.unitText);
  late var _matches = _match();

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  List<KnownUnit> _match() =>
      matchingUnits(_text.text, widget.system, widget.names);

  void _typed() {
    widget.field.unitText = _text.text;
    setState(() => _matches = _match());
    widget.onChanged();
  }

  void _pick(KnownUnit known) {
    final shown = shownUnit(known.code, widget.system);
    widget.field.picked = known.code;
    _text.value = TextEditingValue(
      text: shown,
      selection: TextSelection.collapsed(offset: shown.length),
    );
    _typed();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return BadgeDialog(
      setting: t.tallyUnit,
      icon: Icons.straighten,
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            key: const ValueKey('tally-unit-text'),
            controller: _text,
            autofocus: true,
            decoration: InputDecoration(
              hintText: t.tallyUnitHint,
              helperText: t.tallyUnitHelper,
              helperMaxLines: 3,
            ),
            onChanged: (_) {
              widget.field.picked = null;
              _typed();
            },
            onSubmitted: (_) => Navigator.of(context).pop(),
          ),
          UnitSuggestions(
            matches: _matches,
            system: widget.system,
            onPick: _pick,
          ),
        ],
      ),
    );
  }
}

/// The target badge's editor on the New Tally page.
class _TargetDialog extends StatefulWidget {
  final _FieldDraft field;
  final UnitSystem system;
  final VoidCallback onChanged;

  const _TargetDialog({
    required this.field,
    required this.system,
    required this.onChanged,
  });

  @override
  State<_TargetDialog> createState() => _TargetDialogState();
}

class _TargetDialogState extends State<_TargetDialog> {
  late final _text = TextEditingController(text: widget.field.targetText);

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final field = widget.field;
    final typed = _text.text.trim();
    final invalid = typed.isNotEmpty && field.target(widget.system) == null;
    return BadgeDialog(
      setting: t.tallyTargetBadge,
      icon: Icons.flag_outlined,
      content: TextField(
        key: const ValueKey('tally-field-target'),
        controller: _text,
        autofocus: true,
        keyboardType: const TextInputType.numberWithOptions(
          signed: true,
          decimal: true,
        ),
        decoration: InputDecoration(
          labelText: t.tallyTarget,
          helperText: t.tallyTargetHint,
          suffixText: shownUnit(field.unit(widget.system), widget.system),
          errorText: invalid ? t.tallyNotANumber : null,
          errorMaxLines: 3,
        ),
        onChanged: (_) {
          field.targetText = _text.text;
          setState(() {});
          widget.onChanged();
        },
        onSubmitted: (_) => Navigator.of(context).pop(),
      ),
    );
  }
}
