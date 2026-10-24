import 'notes.dart';
import 'schedule.dart';
import 'store.dart';

/// Field keys of a Routine.
abstract final class RoutineKeys {
  static const type = r'$type';
  static const typeRoutine = 'routine';

  /// The Note the Routine was spawned from (ADR 0005).
  static const note = 'note';

  /// The [Schedule.start] day, `YYYY-MM-DD`.
  static const start = 'start';

  /// The [Schedule.times] as `HH:MM` joined by commas; empty for all day.
  static const times = 'times';

  /// The [Schedule.rule] as an RRULE value; absent for a Reminder.
  static const rule = 'rule';

  /// The field marking [occurrence] Done: its value is when it was
  /// checked off, null when it was unchecked again. Keyed by the
  /// wall-clock Occurrence, so two Devices checking off the same one
  /// meet in one field.
  static String done(Occurrence occurrence) => '$donePrefix${occurrence.key}';

  static const donePrefix = 'done@';
}

/// Something to be done on a schedule, spawned from a Note (CONTEXT.md).
/// A Reminder is a Routine whose schedule happens once (ADR 0005).
class Routine {
  /// Entity id, `routine:<id>`.
  final String id;

  /// The Note it was spawned from, as stored; see [Routines.note] for
  /// the Note it shows under after a Merge.
  final String noteId;
  final Schedule schedule;

  /// When each Done Occurrence was checked off, by [Occurrence.key].
  final Map<String, DateTime> done;

  const Routine({
    required this.id,
    required this.noteId,
    required this.schedule,
    this.done = const {},
  });

  bool get isReminder => schedule.once;

  /// When [occurrence] was checked off, or null while it is not Done.
  DateTime? doneAt(Occurrence occurrence) => done[occurrence.key];
}

/// One row of the Agenda: an Occurrence of a Routine, with the Note it
/// comes from and whether it is Done.
class AgendaRow {
  final Routine routine;
  final Occurrence occurrence;

  /// The Note the Routine links back to, or null when it is gone.
  final Note? note;

  const AgendaRow({
    required this.routine,
    required this.occurrence,
    required this.note,
  });

  DateTime? get done => routine.doneAt(occurrence);
}

/// The home screen's content (CONTEXT.md): today's Occurrences, all-day
/// first and then by time, and tomorrow's the same way.
class Agenda {
  final Day today;
  final List<AgendaRow> todayRows;
  final List<AgendaRow> tomorrowRows;

  const Agenda({
    required this.today,
    required this.todayRows,
    required this.tomorrowRows,
  });

  Day get tomorrow => today.addDays(1);
}

/// Spawning, changing and checking off Routines and Reminders.
class Routines {
  final Store store;

  Routines(this.store);

  /// Spawns a Routine (or a Reminder, when [schedule] happens once) from
  /// the Note [noteId]. The Note leaves Unsorted; it can spawn several.
  Routine spawn(String noteId, Schedule schedule) {
    final note = Notes(store).byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
    }
    final id = 'routine:${Store.newId()}';
    store.batch(() {
      store.append(id, RoutineKeys.type, RoutineKeys.typeRoutine);
      store.append(id, RoutineKeys.note, noteId);
      _writeSchedule(id, schedule);
      if (!note.spawned) {
        store.append(
          noteId,
          NoteKeys.spawned,
          store.clock().toUtc().toIso8601String(),
        );
      }
    });
    return byId(id)!;
  }

  /// Creates a Routine without a Note at hand: its Note, holding [text],
  /// is captured behind the scenes and is not Unsorted (ADR 0005).
  Routine create(String text, Schedule schedule) =>
      spawn(Notes(store).capture(_text(text)).id, schedule);

  /// Gives the Routine [id] a new schedule. What was Done stays Done.
  void reschedule(String id, Schedule schedule) {
    _require(id);
    store.batch(() => _writeSchedule(id, schedule));
  }

  /// Deletes the Routine [id]. Its Note stays, and stays out of Unsorted.
  void delete(String id) {
    _require(id);
    store.append(id, RoutineKeys.type, null);
  }

  /// Checks off [occurrence] of the Routine [id] as Done, at [at] or the
  /// store's now: [at] is when it was tapped on a notification, applied
  /// later. Early or late, the schedule stays as it is (ADR 0006).
  void markDone(String id, Occurrence occurrence, {DateTime? at}) {
    final routine = _require(id);
    if (!routine.schedule.has(occurrence)) {
      throw ArgumentError.value(occurrence, 'occurrence', 'not due then');
    }
    store.append(
      id,
      RoutineKeys.done(occurrence),
      (at ?? store.clock()).toUtc().toIso8601String(),
    );
  }

  /// Unchecks [occurrence] of the Routine [id] again.
  void unmarkDone(String id, Occurrence occurrence) {
    _require(id);
    store.append(id, RoutineKeys.done(occurrence), null);
  }

  /// The Routine [id], or null when there is none.
  Routine? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[RoutineKeys.type] != RoutineKeys.typeRoutine) return null;
    final noteId = fields[RoutineKeys.note];
    final start = Day.tryParse(fields[RoutineKeys.start] ?? '');
    if (noteId == null || start == null) return null;
    final ruleText = fields[RoutineKeys.rule];
    final rule = ruleText == null ? null : Recurrence.tryParse(ruleText);
    if (ruleText != null && rule == null) return null;
    final times = [
      for (final text in (fields[RoutineKeys.times] ?? '').split(','))
        ?ClockTime.tryParse(text),
    ];
    final Schedule schedule;
    try {
      schedule = Schedule(start: start, times: times, rule: rule);
    } on ArgumentError {
      return null;
    }
    return Routine(
      id: id,
      noteId: noteId,
      schedule: schedule,
      done: {
        for (final MapEntry(:key, :value) in fields.entries)
          if (key.startsWith(RoutineKeys.donePrefix) && value != null)
            key.substring(RoutineKeys.donePrefix.length): DateTime.parse(value),
      },
    );
  }

  /// Every Routine and Reminder.
  List<Routine> all() => [
    for (final id in store.entitiesOfType(RoutineKeys.typeRoutine)) ?byId(id),
  ];

  /// The Note [routine] links back to: the Note it was spawned from, or
  /// the one that Note was merged into. Null when it is gone for good.
  Note? note(Routine routine) {
    final notes = Notes(store);
    final note = notes.byId(routine.noteId);
    final into = note?.mergedInto;
    return into == null ? note : notes.byId(into);
  }

  /// The Routines and Reminders the Note [noteId] spawned, those of the
  /// Notes merged into it included.
  List<Routine> ofNote(String noteId) => [
    for (final routine in all())
      if (note(routine)?.id == noteId) routine,
  ];

  /// The Agenda at the store's now, with days as [wallClock] shows them.
  Agenda agenda(WallClock wallClock) {
    final today = Day.today(store.clock(), wallClock);
    final routines = [for (final r in all()) (r, note(r))];
    List<AgendaRow> rowsOn(Day day) {
      final rows = [
        for (final (routine, note) in routines)
          for (final occurrence in routine.schedule.on(day))
            AgendaRow(routine: routine, occurrence: occurrence, note: note),
      ];
      rows.sort((a, b) {
        final byTime = a.occurrence.compareTo(b.occurrence);
        if (byTime != 0) return byTime;
        final byText = (a.note?.text ?? '').compareTo(b.note?.text ?? '');
        if (byText != 0) return byText;
        return a.routine.id.compareTo(b.routine.id);
      });
      return rows;
    }

    return Agenda(
      today: today,
      todayRows: rowsOn(today),
      tomorrowRows: rowsOn(today.addDays(1)),
    );
  }

  void _writeSchedule(String id, Schedule schedule) {
    store.append(id, RoutineKeys.start, schedule.start.toString());
    store.append(id, RoutineKeys.times, schedule.times.join(','));
    store.append(id, RoutineKeys.rule, schedule.rule?.format());
  }

  Routine _require(String id) {
    final routine = byId(id);
    if (routine == null) {
      throw ArgumentError.value(id, 'id', 'not a Routine');
    }
    return routine;
  }

  static String _text(String text) {
    final trimmed = text.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(text, 'text', 'must not be empty');
    }
    return trimmed;
  }
}
