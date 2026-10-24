import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'routine_history_screen.dart';
import 'schedule_text.dart';

/// The Device's wall clock: what its own clock shows for an instant.
DateTime deviceWallClock(DateTime utc) => utc.toLocal();

/// Opens the editor for a new Routine, or a Reminder when [reminder],
/// spawned from [note]; true when one was made.
Future<bool> spawnFromNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note, {
  required bool reminder,
}) => _push(
  context,
  RoutineEditorScreen(
    notes: notes,
    places: places,
    noteId: note.id,
    reminder: reminder,
  ),
);

/// Opens the editor for a new Routine whose Note is made behind the
/// scenes; true when one was made.
Future<bool> newRoutine(BuildContext context, Notes notes, Places places) =>
    _push(context, RoutineEditorScreen(notes: notes, places: places));

/// Opens the editor of [routine]; true when it changed or went.
Future<bool> openRoutine(
  BuildContext context,
  Notes notes,
  Places places,
  Routine routine,
) => _push(
  context,
  RoutineEditorScreen(notes: notes, places: places, routineId: routine.id),
);

Future<bool> _push(BuildContext context, Widget screen) async =>
    await Navigator.of(context)
        .push(MaterialPageRoute<bool>(builder: (_) => screen)) ??
    false;

/// Makes or changes a Routine or Reminder: the Note it comes from (or a
/// text for a new Note), and its schedule in a simple editor (once, every
/// N days, weeks, months or years, chosen weekdays) or as an RRULE in the
/// advanced mode, with its times of day. The next Occurrences show while
/// it is edited. A folded Re-nag section sets how often it rings again
/// when ignored: its own interval or this Device's default, which is
/// set there too, for now (it moves to Settings later). A folded
/// Escalation section below it turns on the full-screen alarm after a
/// number of ignored Re-nags (3 when first turned on). Save writes the
/// Routine; leaving without saving drops the edit, but not a changed
/// Device default, which applies at once. A saved Routine's history
/// opens from the top bar.
class RoutineEditorScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  /// The Routine edited; null for a new one.
  final String? routineId;

  /// The Note a new Routine is spawned from; null to make its Note from
  /// the text typed here.
  final String? noteId;

  /// A new one starts as a Reminder: once, at the next full hour.
  final bool reminder;

  const RoutineEditorScreen({
    super.key,
    required this.notes,
    required this.places,
    this.routineId,
    this.noteId,
    this.reminder = false,
  });

  @override
  State<RoutineEditorScreen> createState() => _RoutineEditorScreenState();
}

class _RoutineEditorScreenState extends State<RoutineEditorScreen> {
  /// How many coming Occurrences the editor shows.
  static const _preview = 3;

  /// The Re-nag intervals offered, in minutes.
  static const _renagChoices = [5, 10, 15, 30, 60];

  /// The numbers of ignored Re-nags offered before Escalation.
  static const _escalationChoices = [1, 2, 3, 5, 10];

  late final _routines = Routines(widget.notes.store);
  final _text = TextEditingController();
  final _rule = TextEditingController();
  late final Day _today;
  Routine? _routine;
  Note? _note;

  bool _advanced = false;
  RepeatKind _kind = RepeatKind.every;
  int _every = 1;
  RepeatUnit _unit = RepeatUnit.days;
  Set<int> _weekdays = {};
  late Day _start;
  List<ClockTime> _times = [];

  /// The Routine's own Re-nag interval; null for the Device's default.
  Duration? _renag;
  late Duration _deviceRenag = widget.notes.store.renagInterval;

  /// After how many ignored Re-nags it escalates; null when it does not.
  int? _escalation;

  /// What is edited now, or null while the advanced rule is no rule.
  Schedule? _schedule;
  String? _ruleError;
  List<Occurrence> _next = const [];

  @override
  void initState() {
    super.initState();
    final now = widget.notes.store.clock();
    _today = Day.today(now, deviceWallClock);
    _start = _today;
    final routine = widget.routineId == null
        ? null
        : _routines.byId(widget.routineId ?? '');
    _routine = routine;
    if (routine != null) {
      _note = _routines.note(routine);
      _renag = routine.renag;
      _escalation = routine.escalation;
      _load(routine.schedule);
    } else {
      final noteId = widget.noteId;
      _note = noteId == null ? null : widget.notes.byId(noteId);
      if (widget.reminder) {
        // The next full hour, today or just after midnight.
        final wall = deviceWallClock(now.toUtc());
        _kind = RepeatKind.once;
        _start = Day.of(wall.add(const Duration(hours: 1)));
        _times = [ClockTime((wall.hour + 1) % 24, 0)];
      } else {
        _times = [ClockTime(8, 0)];
      }
    }
    _text.addListener(_typed);
    _update();
  }

  @override
  void dispose() {
    _text.dispose();
    _rule.dispose();
    super.dispose();
  }

  void _typed() => setState(() {});

  /// Shows [schedule]: in the simple editor when it can, else advanced.
  void _load(Schedule schedule) {
    _start = schedule.start;
    _times = [...schedule.times];
    final rule = schedule.rule;
    _rule.text = rule?.format() ?? '';
    final simple = SimpleRepeat.of(rule, schedule.start);
    if (simple == null) {
      _advanced = true;
    } else {
      _takeSimple(simple);
    }
  }

  void _takeSimple(SimpleRepeat simple) {
    _kind = simple.kind;
    if (simple.kind == RepeatKind.every) {
      _every = simple.every;
      _unit = simple.unit;
    }
    if (simple.kind == RepeatKind.weekdays) _weekdays = {...simple.weekdays};
  }

  /// The rule the simple editor shows.
  Recurrence? _simpleRule() => switch (_kind) {
    RepeatKind.once => null,
    RepeatKind.every => SimpleRepeat.every(_every, _unit).toRule(),
    RepeatKind.weekdays => SimpleRepeat.weekdays(_weekdays).toRule(),
  };

  /// Reads the edit into [_schedule] and its coming Occurrences.
  void _update() {
    Recurrence? rule;
    _ruleError = null;
    if (_advanced) {
      final text = _rule.text.trim();
      try {
        rule = text.isEmpty ? null : Recurrence.parse(text);
      } on FormatException catch (e) {
        // The offending part; the core's message is not translated.
        _ruleError = '${e.source ?? ''}';
      }
    } else {
      rule = _simpleRule();
    }
    final once = _advanced
        ? _rule.text.trim().isEmpty
        : _kind == RepeatKind.once;
    if (once && _times.length > 1) _times = [_times.first];
    final weekdaysMissing =
        !_advanced && _kind == RepeatKind.weekdays && _weekdays.isEmpty;
    _schedule = _ruleError != null || weekdaysMissing
        ? null
        : Schedule(start: _start, times: _times, rule: rule);
    _next = _schedule?.next(_today, _preview) ?? const [];
  }

  void _change(VoidCallback change) => setState(() {
    change();
    _update();
  });

  void _switchMode(bool advanced) => _change(() {
    if (advanced) {
      _rule.text = _simpleRule()?.format() ?? '';
    } else {
      final text = _rule.text.trim();
      final rule = text.isEmpty ? null : Recurrence.tryParse(text);
      final simple = text.isEmpty
          ? const SimpleRepeat.once()
          : rule == null
          ? null
          : SimpleRepeat.of(rule, _start);
      if (simple != null) _takeSimple(simple);
    }
    _advanced = advanced;
  });

  Future<void> _pickStart() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _start.wall,
      firstDate: DateTime(2000),
      lastDate: DateTime(2100, 12, 31),
    );
    if (!mounted || picked == null) return;
    _change(() => _start = Day.of(picked));
  }

  Future<void> _addTime() async {
    final picked = await showTimePicker(
      context: context,
      initialTime: const TimeOfDay(hour: 8, minute: 0),
    );
    if (!mounted || picked == null) return;
    _change(() => _times = [..._times, ClockTime(picked.hour, picked.minute)]);
  }

  /// A Routine saves whatever became of its Note; a new one needs a Note
  /// in the lists to spawn from, or a text for one.
  bool get _canSave {
    if (_schedule == null) return false;
    if (_routine != null) return true;
    final note = _note;
    return note == null ? _text.text.trim().isNotEmpty : !note.isTrashed;
  }

  void _save() {
    final schedule = _schedule;
    if (schedule == null) return;
    final routine = _routine;
    final note = _note;
    final String id;
    if (routine != null) {
      _routines.reschedule(routine.id, schedule);
      id = routine.id;
    } else if (note != null) {
      id = _routines.spawn(note.id, schedule).id;
    } else {
      final text = _text.text.trim();
      if (text.isEmpty) return;
      id = _routines.create(text, schedule).id;
    }
    _routines.setRenag(id, _renag);
    _routines.setEscalation(id, _escalation);
    Navigator.of(context).pop(true);
  }

  void _setDeviceRenag(Duration interval) {
    widget.notes.store.renagInterval = interval;
    setState(() => _deviceRenag = interval);
  }

  Future<void> _delete() async {
    final routine = _routine;
    if (routine == null) return;
    final t = context.t;
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        content: Text(
          routine.isReminder
              ? t.deleteReminderQuestion
              : t.deleteRoutineQuestion,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.cancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.delete),
          ),
        ],
      ),
    );
    if (!mounted || sure != true) return;
    _routines.trash(routine.id);
    Navigator.of(context).pop(true);
  }

  Future<void> _openNote() async {
    final note = _note;
    if (note == null || note.isTrashed) return;
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    // Deleting the Note there may have taken this Routine along.
    final routine = _routine;
    if (routine != null && (_routines.byId(routine.id)?.isTrashed ?? true)) {
      Navigator.of(context).pop(true);
      return;
    }
    // A Merge there moves the back-link to the Note it went into.
    setState(
      () => _note = routine == null
          ? widget.notes.byId(note.id)
          : _routines.note(routine),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final once =
        _schedule?.once ??
        (_advanced ? _rule.text.trim().isEmpty : _kind == RepeatKind.once);
    final routine = _routine;
    final title = routine == null
        ? (once ? t.newReminder : t.newRoutine)
        : (once ? t.reminder : t.routine);
    final note = _note;
    return Scaffold(
      appBar: AppBar(
        title: Text(title),
        actions: [
          if (routine != null && !routine.isReminder)
            IconButton(
              tooltip: t.historyTitle,
              onPressed: () => openHistory(context, widget.notes, routine),
              icon: const Icon(Icons.history),
            ),
          if (routine != null)
            IconButton(
              tooltip: t.delete,
              onPressed: _delete,
              icon: const Icon(Icons.delete_outline),
            ),
          TextButton(onPressed: _canSave ? _save : null, child: Text(t.save)),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          if (widget.noteId == null && routine == null)
            Embossed(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: TextField(
                  controller: _text,
                  autofocus: true,
                  maxLines: null,
                  textCapitalization: TextCapitalization.sentences,
                  decoration: InputDecoration(
                    hintText: t.routineWhatHint,
                    filled: false,
                    border: InputBorder.none,
                  ),
                ),
              ),
            )
          else
            EditorPanel(
              heading: t.fromNote,
              child: note == null
                  ? Text(t.noteDeleted)
                  : note.isTrashed
                  ? Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          noteText(t, note),
                          style: Theme.of(context).textTheme.bodyLarge,
                        ),
                        Text(t.noteInTrash),
                      ],
                    )
                  : Material(
                      // The ink shows on the panel, not under it.
                      type: MaterialType.transparency,
                      child: InkWell(
                        onTap: _openNote,
                        child: Row(
                          children: [
                            const Icon(Icons.sticky_note_2_outlined),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                noteText(t, note),
                                style: Theme.of(context).textTheme.bodyLarge,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
            ),
          const SizedBox(height: 16),
          EditorPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SegmentedButton<bool>(
                  segments: [
                    ButtonSegment(value: false, label: Text(t.scheduleSimple)),
                    ButtonSegment(value: true, label: Text(t.scheduleAdvanced)),
                  ],
                  selected: {_advanced},
                  showSelectedIcon: false,
                  onSelectionChanged: (picked) => _switchMode(picked.single),
                ),
                const SizedBox(height: 16),
                if (_advanced) _advancedEditor(t) else ..._simpleEditor(t),
                const SizedBox(height: 8),
                Wrap(
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    Text(once ? t.onDay : t.startsOn),
                    const SizedBox(width: 8),
                    TextButton.icon(
                      onPressed: _pickStart,
                      icon: const Icon(Icons.event),
                      label: Text(dayText(locale, _start)),
                    ),
                  ],
                ),
              ],
            ),
          ),
          EditorPanel(
            heading: t.timesHeading,
            child: Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                if (_times.isEmpty) Chip(label: Text(t.allDay)),
                for (final time in _times)
                  InputChip(
                    label: Text(clockText(locale, time)),
                    deleteButtonTooltipMessage: t.removeTime(
                      clockText(locale, time),
                    ),
                    onDeleted: () => _change(
                      () => _times = [
                        for (final kept in _times)
                          if (kept != time) kept,
                      ],
                    ),
                  ),
                if (!once || _times.isEmpty)
                  ActionChip(
                    avatar: const Icon(Icons.add),
                    label: Text(t.addTime),
                    onPressed: _addTime,
                  ),
              ],
            ),
          ),
          EditorPanel(
            heading: t.nextOccurrences,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (_schedule != null && _next.isEmpty) Text(t.neverDue),
                for (final o in _next)
                  Text(
                    '${dayText(locale, o.day)}  '
                    '${occurrenceTime(t, locale, o)}',
                  ),
              ],
            ),
          ),
          _renagPanel(t),
          _escalationPanel(t),
        ],
      ),
    );
  }

  /// The folded Re-nag section: this one's interval and the Device's.
  Widget _renagPanel(AppLocalizations t) {
    final own = _renag?.inMinutes;
    final device = _deviceRenag.inMinutes;
    // A value set elsewhere stays pickable.
    List<int> choices(int? kept) => {..._renagChoices, ?kept}.toList()..sort();
    return Embossed(
      child: ExpansionTile(
        key: const ValueKey('renag'),
        shape: const Border(),
        collapsedShape: const Border(),
        title: Text(t.renagHeading),
        subtitle: Text(t.renagSummary(own ?? device)),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        expandedCrossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(t.renagExplain),
          const SizedBox(height: 8),
          Row(
            children: [
              Expanded(child: Text(t.renagThis)),
              DropdownButton<int?>(
                key: const ValueKey('renagThis'),
                value: own,
                onChanged: (minutes) => setState(
                  () => _renag = minutes == null
                      ? null
                      : Duration(minutes: minutes),
                ),
                items: [
                  DropdownMenuItem(
                    value: null,
                    child: Text(t.renagDeviceDefault(device)),
                  ),
                  for (final m in choices(own))
                    DropdownMenuItem(value: m, child: Text(t.renagMinutes(m))),
                ],
              ),
            ],
          ),
          Row(
            children: [
              Expanded(child: Text(t.renagDevice)),
              DropdownButton<int>(
                key: const ValueKey('renagDevice'),
                value: device,
                onChanged: (minutes) {
                  if (minutes != null) {
                    _setDeviceRenag(Duration(minutes: minutes));
                  }
                },
                items: [
                  for (final m in choices(device))
                    DropdownMenuItem(value: m, child: Text(t.renagMinutes(m))),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  /// The folded Escalation section: off, or after how many ignored
  /// Re-nags it becomes a full-screen alarm.
  Widget _escalationPanel(AppLocalizations t) {
    final after = _escalation;
    return Embossed(
      child: ExpansionTile(
        key: const ValueKey('escalation'),
        shape: const Border(),
        collapsedShape: const Border(),
        title: Text(t.escalationHeading),
        subtitle: Text(
          after == null ? t.escalationOff : t.escalationSummary(after),
        ),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        expandedCrossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(t.escalationExplain),
          SwitchListTile(
            key: const ValueKey('escalationOn'),
            contentPadding: EdgeInsets.zero,
            title: Text(t.escalationSwitch),
            value: after != null,
            onChanged: (on) =>
                setState(() => _escalation = on ? defaultEscalation : null),
          ),
          if (after != null)
            Row(
              children: [
                Expanded(child: Text(t.escalationAfter)),
                DropdownButton<int>(
                  key: const ValueKey('escalationAfter'),
                  value: after,
                  onChanged: (count) {
                    if (count != null) setState(() => _escalation = count);
                  },
                  items: [
                    // A value set elsewhere stays pickable.
                    for (final n in {
                      ..._escalationChoices,
                      after,
                    }.toList()..sort())
                      DropdownMenuItem(value: n, child: Text('$n')),
                  ],
                ),
              ],
            ),
        ],
      ),
    );
  }

  List<Widget> _simpleEditor(AppLocalizations t) {
    final locale = Localizations.localeOf(context).toString();
    return [
      SegmentedButton<RepeatKind>(
        segments: [
          ButtonSegment(value: RepeatKind.once, label: Text(t.repeatOnce)),
          ButtonSegment(value: RepeatKind.every, label: Text(t.repeatEvery)),
          ButtonSegment(
            value: RepeatKind.weekdays,
            label: Text(t.repeatWeekdays),
          ),
        ],
        selected: {_kind},
        showSelectedIcon: false,
        onSelectionChanged: (picked) => _change(() {
          _kind = picked.single;
          if (_kind == RepeatKind.weekdays && _weekdays.isEmpty) {
            _weekdays = {_start.weekday};
          }
        }),
      ),
      if (_kind == RepeatKind.every) ...[
        const SizedBox(height: 8),
        Wrap(
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            Text(t.everyLabel),
            IconButton(
              tooltip: t.fewer,
              onPressed: _every > 1 ? () => _change(() => _every--) : null,
              icon: const Icon(Icons.remove),
            ),
            Text('$_every', style: Theme.of(context).textTheme.titleMedium),
            IconButton(
              tooltip: t.more,
              onPressed: () => _change(() => _every++),
              icon: const Icon(Icons.add),
            ),
            DropdownButton<RepeatUnit>(
              value: _unit,
              onChanged: (unit) {
                if (unit != null) _change(() => _unit = unit);
              },
              items: [
                for (final unit in RepeatUnit.values)
                  DropdownMenuItem(
                    value: unit,
                    child: Text(switch (unit) {
                      RepeatUnit.days => t.unitDays(_every),
                      RepeatUnit.weeks => t.unitWeeks(_every),
                      RepeatUnit.months => t.unitMonths(_every),
                      RepeatUnit.years => t.unitYears(_every),
                    }),
                  ),
              ],
            ),
          ],
        ),
      ],
      if (_kind == RepeatKind.weekdays) ...[
        const SizedBox(height: 8),
        Wrap(
          spacing: 4,
          runSpacing: 4,
          children: [
            for (var day = DateTime.monday; day <= DateTime.sunday; day++)
              FilterChip(
                label: Text(weekdayText(locale, day)),
                selected: _weekdays.contains(day),
                onSelected: (on) => _change(
                  () => _weekdays = on
                      ? {..._weekdays, day}
                      : ({..._weekdays}..remove(day)),
                ),
              ),
          ],
        ),
      ],
    ];
  }

  Widget _advancedEditor(AppLocalizations t) => TextField(
    controller: _rule,
    autocorrect: false,
    enableSuggestions: false,
    textCapitalization: TextCapitalization.characters,
    onChanged: (_) => _change(() {}),
    decoration: InputDecoration(
      labelText: t.rruleLabel,
      helperText: t.rruleHelper,
      helperMaxLines: 2,
      errorText: _ruleError == null ? null : t.rruleInvalid(_ruleError ?? ''),
      errorMaxLines: 3,
    ),
  );
}
