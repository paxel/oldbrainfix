import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'calendar_platform.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'routine_history_screen.dart';
import 'schedule_text.dart';
import 'session_editor.dart';
import 'system_bars.dart';

/// The Device's wall clock: what its own clock shows for an instant.
DateTime deviceWallClock(DateTime utc) => utc.toLocal();

/// Spawns [spawn] from [note]: a Routine, Reminder or Session through its
/// editor, a Frog at once, which needs nothing more; true when one was
/// made.
Future<bool> spawnFromNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note, {
  required Spawn spawn,
}) async {
  if (spawn == Spawn.frog) {
    Frogs(notes.store).spawn(note.id);
    return true;
  }
  return _push(
    context,
    RoutineEditorScreen(
      notes: notes,
      places: places,
      noteId: note.id,
      reminder: spawn == Spawn.reminder,
      session: spawn == Spawn.session,
    ),
  );
}

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
/// N days, weeks, months or years, chosen weekdays, every N months on a
/// day or on the Nth or last weekday, each with an end: never, on a day
/// or after a number of times) or as an RRULE in the advanced mode, with
/// its times of day. The next Occurrences show while
/// it is edited. A folded Re-nag section sets how often it rings again
/// when ignored: its own interval or this Device's default from
/// Settings. A folded Escalation section below it turns on the
/// full-screen alarm after a number of ignored Re-nags (3 when first
/// turned on). Save writes the Routine; leaving without saving drops the
/// edit. A saved Routine's history
/// opens from the top bar, and its calendar button adds it, as saved,
/// to the calendar or shares it as a calendar file.
///
/// A Session (#104) has its pool, count, weekdays' counts and rule on top
/// of the schedule ([SessionFields]); Save says under each field what to
/// fix there.
class RoutineEditorScreen extends StatefulWidget {
  /// The Re-nag intervals offered, in minutes.
  static const renagChoices = [5, 10, 15, 30, 60];

  final Notes notes;
  final Places places;

  /// The Routine edited; null for a new one.
  final String? routineId;

  /// The Note a new Routine is spawned from; null to make its Note from
  /// the text typed here.
  final String? noteId;

  /// A new one starts as a Reminder: once, at the next full hour.
  final bool reminder;

  /// A new one is a Session, spawned from [noteId].
  final bool session;

  const RoutineEditorScreen({
    super.key,
    required this.notes,
    required this.places,
    this.routineId,
    this.noteId,
    this.reminder = false,
    this.session = false,
  });

  @override
  State<RoutineEditorScreen> createState() => _RoutineEditorScreenState();
}

class _RoutineEditorScreenState extends State<RoutineEditorScreen> {
  /// How many coming Occurrences the editor shows.
  static const _preview = 3;

  /// The numbers of ignored Re-nags offered before Escalation.
  static const _escalationChoices = [1, 2, 3, 5, 10];

  late final _routines = Routines(widget.notes.store);
  final _text = TextEditingController();
  final _rule = TextEditingController();
  late final Day _today;
  Routine? _routine;
  Note? _note;

  /// The Session edited, when it is one.
  Session? _session;
  SessionDraft? _draft;

  bool _advanced = false;
  RepeatKind _kind = RepeatKind.every;
  int _every = 1;
  RepeatUnit _unit = RepeatUnit.days;
  Set<int> _weekdays = {};

  /// N in every N months, for the month kinds.
  int _months = 1;

  /// The day of the month; the start's until one is chosen.
  int? _monthDay;

  /// 1 to 4 or -1 for the last, and the weekday; the start's position in
  /// its month until they are chosen.
  int? _ordinal;
  int? _nthWeekday;

  RepeatEnd _end = RepeatEnd.never;

  /// The end day and number, kept while another end is chosen.
  Day? _until;
  int _count = 10;
  late Day _start;
  List<ClockTime> _times = [];

  /// The Routine's own Re-nag interval; null for the Device's default.
  Duration? _renag;
  late final _deviceRenag = widget.notes.store.renagInterval;

  /// After how many ignored Re-nags it escalates; null when it does not.
  int? _escalation;

  /// A Routine in a Shared Area that only a Manager may change: shown,
  /// not edited (ADR 0013).
  late final bool _locked =
      widget.routineId != null &&
      !widget.notes.store.mayChangeRoutine(widget.routineId ?? '');

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
      final session = _session = Sessions(widget.notes.store).byId(routine.id);
      if (session != null) {
        _draft = SessionDraft.of(widget.notes.store, routine.id, session);
      }
      _note = _routines.note(routine);
      _renag = routine.renag;
      _escalation = routine.escalation;
      _load(routine.schedule);
    } else {
      final noteId = widget.noteId;
      _note = noteId == null ? null : widget.notes.byId(noteId);
      if (widget.session && noteId != null) {
        _draft = SessionDraft.of(widget.notes.store, noteId);
      }
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
    _draft?.dispose();
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
    if (simple.kind == RepeatKind.monthDay) {
      _months = simple.every;
      _monthDay = simple.monthDay;
    }
    if (simple.nth case final nth?
        when simple.kind == RepeatKind.monthWeekday) {
      _months = simple.every;
      _ordinal = nth.ordinal;
      _nthWeekday = nth.weekday;
    }
    _end = simple.end;
    if (simple.until case final until?) _until = until;
    if (simple.count case final count?) _count = count;
  }

  /// Where [day] lies in its month: the 1st to 4th of its weekday, or -1
  /// for a 5th, which is the last.
  static int _ordinalOf(Day day) {
    final nth = (day.day - 1) ~/ 7 + 1;
    return nth > 4 ? -1 : nth;
  }

  /// An end day not chosen yet: a month after the start.
  Day get _untilDay => _until ?? Day(_start.year, _start.month + 1, _start.day);

  /// The rule the simple editor shows.
  Recurrence? _simpleRule() {
    final count = _end == RepeatEnd.count ? _count : null;
    final until = _end == RepeatEnd.until ? _untilDay : null;
    return switch (_kind) {
      RepeatKind.once => null,
      RepeatKind.every => SimpleRepeat.every(
        _every,
        _unit,
        count: count,
        until: until,
      ).toRule(),
      RepeatKind.weekdays => SimpleRepeat.weekdays(
        _weekdays,
        count: count,
        until: until,
      ).toRule(),
      RepeatKind.monthDay => SimpleRepeat.monthDay(
        _monthDay ?? _start.day,
        every: _months,
        count: count,
        until: until,
      ).toRule(),
      RepeatKind.monthWeekday => SimpleRepeat.monthWeekday(
        WeekdayRule(
          _nthWeekday ?? _start.weekday,
          _ordinal ?? _ordinalOf(_start),
        ),
        every: _months,
        count: count,
        until: until,
      ).toRule(),
    };
  }

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

  Future<void> _pickUntil() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _untilDay.wall,
      firstDate: DateTime(2000),
      lastDate: DateTime(2100, 12, 31),
    );
    if (!mounted || picked == null) return;
    _change(() => _until = Day.of(picked));
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
    final draft = _draft;
    if (draft != null && !draft.valid) {
      setState(() => draft.checked = true);
      // The weekday to fix comes into view.
      if (!draft.weekdaysValid) draft.weekdays.expand();
      return;
    }
    final routine = _routine;
    final note = _note;
    final session = _session;
    final String id;
    if (routine != null) {
      _routines.reschedule(routine.id, schedule);
      if (draft != null && session != null) draft.save(session);
      id = routine.id;
    } else if (note != null) {
      id = draft == null
          ? _routines.spawn(note.id, schedule).id
          : draft.spawn(note.id, schedule);
    } else {
      final text = _text.text.trim();
      if (text.isEmpty) return;
      id = _routines.create(text, schedule).id;
    }
    _routines.setRenag(id, _renag);
    _routines.setEscalation(id, _escalation);
    Navigator.of(context).pop(true);
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

  /// What the calendar button offers for [routine] as saved: each of its
  /// events, and all of them as one calendar file.
  List<CalendarChoice> _calendarChoices(Routine routine) {
    final add = addChoices(context, _routines, routine);
    if (add.isEmpty) return const [];
    return [
      ...add,
      (
        label: context.t.shareCalendarFile,
        run: () => shareCalendarFile(
          context,
          widget.notes.store,
          eventsOf(context, _routines, routine),
        ),
      ),
    ];
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
    final title = _draft != null
        ? (routine == null ? t.newSession : t.session)
        : routine == null
        ? (once ? t.newReminder : t.newRoutine)
        : (once ? t.reminder : t.routine);
    final note = _note;
    return Scaffold(
      appBar: AppBar(
        title: Text(title),
        actions: [
          if (routine != null)
            CalendarButton(choices: () => _calendarChoices(routine)),
          if (routine != null && !routine.isReminder)
            IconButton(
              tooltip: t.historyTitle,
              onPressed: () => openHistory(context, widget.notes, routine),
              icon: const Icon(Icons.history),
            ),
          if (routine != null && !_locked)
            IconButton(
              tooltip: t.delete,
              onPressed: _delete,
              icon: const Icon(Icons.delete_outline),
            ),
          if (!_locked)
            TextButton(onPressed: _canSave ? _save : null, child: Text(t.save)),
        ],
      ),
      body: AbsorbPointer(
        absorbing: _locked,
        child: _body(context, t, locale, routine, note, once: once),
      ),
    );
  }

  Widget _body(
    BuildContext context,
    AppLocalizations t,
    String locale,
    Routine? routine,
    Note? note, {
    required bool once,
  }) {
    return ListView(
      padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
      children: [
        if (_locked)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: Text(t.onlyManagers),
          ),
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
        if (_draft case final draft?)
          SessionFields(draft: draft, onChanged: () => setState(() {})),
        EditorPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              RadioColumn<bool>(
                value: _advanced,
                options: [
                  RadioOption(value: false, title: Text(t.scheduleSimple)),
                  RadioOption(value: true, title: Text(t.scheduleAdvanced)),
                ],
                onChanged: _switchMode,
              ),
              const SizedBox(height: 16),
              if (_advanced) _advancedEditor(t) else ..._simpleEditor(t),
              const SizedBox(height: 8),
              ControlBelow(
                padding: EdgeInsets.zero,
                title: Text(once ? t.onDay : t.startsOn),
                control: TextButton.icon(
                  onPressed: _pickStart,
                  icon: const Icon(Icons.event),
                  label: Text(dayText(locale, _start)),
                ),
              ),
              if (!_advanced && _kind != RepeatKind.once) ..._endEditor(t),
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
    );
  }

  /// The folded Re-nag section: this one's interval, or the Device's.
  Widget _renagPanel(AppLocalizations t) {
    final own = _renag?.inMinutes;
    final device = _deviceRenag.inMinutes;
    // A value set elsewhere stays pickable.
    final choices = {...RoutineEditorScreen.renagChoices, ?own}.toList()
      ..sort();
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
          ControlBelow(
            padding: EdgeInsets.zero,
            title: Text(t.renagThis),
            // 0 stands for the Device default: no Re-nag lasts 0 min.
            control: RadioColumn<int>(
              key: const ValueKey('renagThis'),
              value: own ?? 0,
              onChanged: (minutes) => setState(
                () => _renag = minutes == 0 ? null : Duration(minutes: minutes),
              ),
              options: [
                RadioOption(
                  value: 0,
                  title: Text(t.renagDeviceDefault(device)),
                ),
                for (final m in choices)
                  RadioOption(value: m, title: Text(t.renagMinutes(m))),
              ],
            ),
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
          SwitchBelow(
            switchKey: const ValueKey('escalationOn'),
            padding: const EdgeInsets.symmetric(vertical: 8),
            title: Text(t.escalationSwitch),
            value: after != null,
            onChanged: (on) =>
                setState(() => _escalation = on ? defaultEscalation : null),
          ),
          if (after != null)
            ControlBelow(
              padding: EdgeInsets.zero,
              title: Text(t.escalationAfter),
              control: RadioColumn<int>(
                key: const ValueKey('escalationAfter'),
                value: after,
                onChanged: (count) => setState(() => _escalation = count),
                options: [
                  // A value set elsewhere stays pickable.
                  for (final n in {
                    ..._escalationChoices,
                    after,
                  }.toList()..sort())
                    RadioOption(value: n, title: Text('$n')),
                ],
              ),
            ),
        ],
      ),
    );
  }

  List<Widget> _simpleEditor(AppLocalizations t) {
    final locale = Localizations.localeOf(context).toString();
    return [
      RadioColumn<RepeatKind>(
        value: _kind,
        options: [
          RadioOption(value: RepeatKind.once, title: Text(t.repeatOnce)),
          RadioOption(value: RepeatKind.every, title: Text(t.repeatEvery)),
          RadioOption(
            value: RepeatKind.weekdays,
            title: Text(t.repeatWeekdays),
          ),
          RadioOption(
            value: RepeatKind.monthDay,
            title: Text(t.repeatMonthDay),
          ),
          RadioOption(
            value: RepeatKind.monthWeekday,
            title: Text(t.repeatMonthWeekday),
          ),
        ],
        onChanged: (picked) => _change(() {
          _kind = picked;
          if (_kind == RepeatKind.weekdays && _weekdays.isEmpty) {
            _weekdays = {_start.weekday};
          }
          if (_kind == RepeatKind.monthDay) _monthDay ??= _start.day;
          if (_kind == RepeatKind.monthWeekday) {
            _ordinal ??= _ordinalOf(_start);
            _nthWeekday ??= _start.weekday;
          }
        }),
      ),
      if (_kind == RepeatKind.every) ...[
        const SizedBox(height: 8),
        ControlBelow(
          padding: EdgeInsets.zero,
          title: Text(t.everyLabel),
          control: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _stepper(_every, onChanged: (n) => _every = n),
              RadioColumn<RepeatUnit>(
                key: const ValueKey('everyUnit'),
                value: _unit,
                onChanged: (unit) => _change(() => _unit = unit),
                options: [
                  for (final unit in RepeatUnit.values)
                    RadioOption(
                      value: unit,
                      title: Text(switch (unit) {
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
      if (_kind == RepeatKind.monthDay || _kind == RepeatKind.monthWeekday)
        ..._monthEditor(t, locale),
    ];
  }

  /// A number with a minus and a plus button, and its unit after it.
  Widget _stepper(
    int value, {
    required ValueChanged<int> onChanged,
    int min = 1,
    int? max,
    String? unit,
  }) => Wrap(
    crossAxisAlignment: WrapCrossAlignment.center,
    children: [
      IconButton(
        tooltip: context.t.fewer,
        onPressed: value > min
            ? () => _change(() => onChanged(value - 1))
            : null,
        icon: const Icon(Icons.remove),
      ),
      Text('$value', style: Theme.of(context).textTheme.titleMedium),
      IconButton(
        tooltip: context.t.more,
        onPressed: max == null || value < max
            ? () => _change(() => onChanged(value + 1))
            : null,
        icon: const Icon(Icons.add),
      ),
      if (unit != null) Text(unit),
    ],
  );

  /// A single choice under its text (UI laws, "Dialogs and pages").
  Widget _choice<T>(
    String title,
    T value,
    List<RadioOption<T>> options,
    ValueChanged<T> onChanged,
  ) => ControlBelow(
    padding: const EdgeInsets.only(top: 8),
    title: Text(title),
    control: RadioColumn<T>(
      value: value,
      options: options,
      onChanged: onChanged,
    ),
  );

  /// The controls of the month kinds: every N months, and the day or the
  /// Nth weekday.
  List<Widget> _monthEditor(AppLocalizations t, String locale) {
    final monthDay = _monthDay ?? _start.day;
    return [
      const SizedBox(height: 8),
      ControlBelow(
        padding: EdgeInsets.zero,
        title: Text(t.everyLabel),
        control: _stepper(
          _months,
          onChanged: (n) => _months = n,
          unit: t.unitMonths(_months),
        ),
      ),
      if (_kind == RepeatKind.monthDay)
        ControlBelow(
          padding: EdgeInsets.zero,
          title: Text(t.dayOfMonthLabel),
          subtitle: monthDay > 28 ? Text(t.monthDaySkipped) : null,
          control: _stepper(
            monthDay,
            max: 31,
            onChanged: (day) => _monthDay = day,
          ),
        )
      else ...[
        _choice<int>(t.whichInMonthLabel, _ordinal ?? _ordinalOf(_start), [
          for (final ordinal in const [1, 2, 3, 4, -1])
            RadioOption(value: ordinal, title: Text(t.nthOption('$ordinal'))),
        ], (ordinal) => _change(() => _ordinal = ordinal)),
        _choice<int>(t.weekdayLabel, _nthWeekday ?? _start.weekday, [
          for (var day = DateTime.monday; day <= DateTime.sunday; day++)
            RadioOption(value: day, title: Text(weekdayName(locale, day))),
        ], (day) => _change(() => _nthWeekday = day)),
      ],
    ];
  }

  /// How a repeat ends: never, on a day, or after a number of times.
  List<Widget> _endEditor(AppLocalizations t) {
    final locale = Localizations.localeOf(context).toString();
    return [
      _choice<RepeatEnd>(t.endsLabel, _end, [
        RadioOption(value: RepeatEnd.never, title: Text(t.endNever)),
        RadioOption(value: RepeatEnd.until, title: Text(t.endOnDay)),
        RadioOption(value: RepeatEnd.count, title: Text(t.endAfterCount)),
      ], (end) => _change(() => _end = end)),
      if (_end == RepeatEnd.until)
        ControlBelow(
          padding: EdgeInsets.zero,
          title: Text(t.endLastDay),
          control: TextButton.icon(
            onPressed: _pickUntil,
            icon: const Icon(Icons.event_busy),
            label: Text(dayText(locale, _untilDay)),
          ),
        ),
      if (_end == RepeatEnd.count)
        ControlBelow(
          padding: EdgeInsets.zero,
          title: Text(t.endCountLabel),
          subtitle: Text(t.endCountExplain),
          control: _stepper(
            _count,
            max: 99999,
            onChanged: (n) => _count = n,
            unit: t.unitTimes(_count),
          ),
        ),
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
