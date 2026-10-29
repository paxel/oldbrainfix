import 'notes.dart';
import 'schedule.dart';
import 'sessions.dart';
import 'shared_areas.dart';
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

  /// The Routine's own Re-nag interval in minutes; absent for this
  /// Device's default.
  static const renag = 'renag';

  /// After how many ignored Re-nags the Routine's Occurrences escalate
  /// to a full-screen alarm; absent when Escalation is off.
  static const escalation = 'escalation';

  /// When the Routine was moved to the trash; absent while it is not
  /// there.
  static const trashed = 'trashed';

  /// The Note the Routine went to the trash with, and comes back with;
  /// absent when it was trashed on its own.
  static const trashedWith = 'trashedWith';

  /// The fields that make up the schedule.
  static const scheduleFields = [start, times, rule];
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

  /// How often an ignored Occurrence fires again; null for the Device's
  /// default.
  final Duration? renag;

  /// After how many ignored Re-nags an Occurrence becomes a full-screen
  /// alarm with sound (CONTEXT.md, Escalation); null when it is off.
  final int? escalation;

  /// When the schedule was last written or the Routine last left the
  /// trash: Occurrences due before then were never asked for, so they
  /// neither Re-nag nor count as Missed. Null when unknown, which bounds
  /// nothing.
  final DateTime? scheduledAt;

  /// When the Routine was moved to the trash, or null when it is not
  /// there.
  final DateTime? trashed;

  /// The Note it went to the trash with, or null when it went on its own
  /// or is not in the trash.
  final String? trashedWith;

  const Routine({
    required this.id,
    required this.noteId,
    required this.schedule,
    this.done = const {},
    this.renag,
    this.escalation,
    this.scheduledAt,
    this.trashed,
    this.trashedWith,
  });

  bool get isReminder => schedule.once;

  bool get isTrashed => trashed != null;

  /// When [occurrence] was checked off, or null while it is not Done.
  DateTime? doneAt(Occurrence occurrence) => done[occurrence.key];
}

/// [routines] in alphabetical order of their names, as the list of all
/// Routines and Reminders shows them; [nameOf] gives the title the app
/// shows for one. A name counts by its first line, trimmed, in lower
/// case, compared by UTF-16 code units; equal names go by id, so the
/// order is the same every time and on every Device.
List<Routine> routinesByName(
  Iterable<Routine> routines,
  String Function(Routine routine) nameOf,
) {
  final keyed = [for (final r in routines) (r, _nameKey(nameOf(r)))];
  keyed.sort((a, b) {
    final byName = a.$2.compareTo(b.$2);
    return byName != 0 ? byName : a.$1.id.compareTo(b.$1.id);
  });
  return [for (final (routine, _) in keyed) routine];
}

String _nameKey(String name) =>
    name.trim().split(RegExp(r'\r?\n')).first.trim().toLowerCase();

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
/// first and then by time, tomorrow's the same way, and those of the
/// days after when asked for.
class Agenda {
  final Day today;
  final List<AgendaRow> todayRows;
  final List<AgendaRow> tomorrowRows;

  /// The rows of each day after tomorrow, one list per day, in order.
  final List<List<AgendaRow>> laterRows;

  const Agenda({
    required this.today,
    required this.todayRows,
    required this.tomorrowRows,
    this.laterRows = const [],
  });

  Day get tomorrow => today.addDays(1);

  /// The day of `laterRows[index]`.
  Day later(int index) => today.addDays(2 + index);
}

/// Spawning, changing and checking off Routines and Reminders.
class Routines {
  final Store store;

  Routines(this.store);

  /// Spawns a Routine (or a Reminder, when [schedule] happens once) from
  /// the Note [noteId]. The Note leaves Unsorted; it can spawn several.
  /// [within] writes more of it in the same batch, given its id.
  Routine spawn(
    String noteId,
    Schedule schedule, {
    void Function(String id)? within,
  }) {
    final note = Notes(store).byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
    }
    if (!store.mayManage(store.areaOf(noteId))) {
      throw StateError('not a Manager of the Area of $noteId');
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
      within?.call(id);
    });
    return byId(id)!;
  }

  /// Creates a Routine without a Note at hand: its Note, holding [text],
  /// is captured behind the scenes and is not Unsorted (ADR 0005).
  Routine create(String text, Schedule schedule) =>
      spawn(Notes(store).capture(_text(text)).id, schedule);

  /// Gives the Routine [id] a new schedule. What was Done stays Done.
  /// The same schedule again writes nothing, so it keeps what was Missed.
  void reschedule(String id, Schedule schedule) {
    _require(id);
    store.batch(() => _writeSchedule(id, schedule));
  }

  /// Gives the Routine [id] its own Re-nag interval, or back the
  /// Device's default with null.
  void setRenag(String id, Duration? renag) {
    final routine = _require(id);
    if (routine.renag == renag) return;
    if (renag != null && renag.inMinutes < 1) {
      throw ArgumentError.value(renag, 'renag', 'at least a minute');
    }
    store.append(id, RoutineKeys.renag, renag?.inMinutes.toString());
  }

  /// Turns Escalation on for the Routine [id], after [after] ignored
  /// Re-nags, or off with null.
  void setEscalation(String id, int? after) {
    final routine = _require(id);
    if (routine.escalation == after) return;
    if (after != null && after < 1) {
      throw ArgumentError.value(after, 'after', 'at least one Re-nag');
    }
    store.append(id, RoutineKeys.escalation, after?.toString());
  }

  /// Moves the Routine [id] to the trash: it leaves the Agenda, fires
  /// no more and is never Missed, and can be restored until
  /// [Notes.trashKeep] has passed. Its Note stays, and stays out of
  /// Unsorted.
  void trash(String id) {
    _require(id);
    store.append(id, RoutineKeys.trashed, _now());
  }

  /// Brings the Routine [id] back from the trash. Its schedule goes on
  /// where it always was (ADR 0006); what fell due while it was in the
  /// trash is not Missed.
  void restore(String id) {
    final routine = byId(id);
    if (routine == null || !routine.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not a Routine in the trash');
    }
    store.batch(() => _untrash(routine));
  }

  /// Moves the Note [noteId] to the trash, and with [withRoutines] the
  /// Routines and Reminders it spawned too, those of the Notes merged
  /// into it included: they come back with it ([restoreNote]). Without,
  /// they stay and link back to a Note in the trash.
  void trashNote(String noteId, {required bool withRoutines}) {
    final routines = withRoutines ? ofNote(noteId) : const <Routine>[];
    store.batch(() {
      Notes(store).trash(noteId);
      // The same moment as the Note's, so both are purged together.
      final at = store.current(noteId, NoteKeys.trashed) ?? _now();
      for (final routine in routines) {
        store.append(routine.id, RoutineKeys.trashed, at);
        store.append(routine.id, RoutineKeys.trashedWith, noteId);
      }
    });
  }

  /// Brings the Note [noteId] back from the trash, with the Routines and
  /// Reminders that went there with it.
  void restoreNote(String noteId) {
    store.batch(() {
      Notes(store).restore(noteId);
      for (final id in store.entitiesOfType(RoutineKeys.typeRoutine)) {
        final routine = byId(id);
        if (routine != null && routine.trashedWith == noteId) {
          _untrash(routine);
        }
      }
    });
  }

  /// When the trashed [routine] is purged; null when it is not in the
  /// trash.
  DateTime? purgeAt(Routine routine) => routine.trashed?.add(Notes.trashKeep);

  /// The Routines and Reminders trashed on their own that can still be
  /// restored, most recently deleted first. Those trashed with their
  /// Note come back with it and are not listed.
  List<Routine> trashed() {
    final now = store.clock().toUtc();
    final routines = [
      for (final id in store.entitiesOfType(RoutineKeys.typeRoutine))
        if (byId(id) case final routine? when routine.trashedWith == null)
          if (purgeAt(routine) case final at? when now.isBefore(at)) routine,
    ];
    routines.sort(
      (a, b) => (b.trashed ?? DateTime(0)).compareTo(a.trashed ?? DateTime(0)),
    );
    return routines;
  }

  /// Purges every Routine and Reminder that has been in the trash for
  /// [Notes.trashKeep]; those trashed with a Note go with it, on the same
  /// day. The log keeps their entries; nothing shows them again.
  void purgeTrash() {
    final now = store.clock().toUtc();
    store.batch(() {
      for (final id in store.entitiesOfType(RoutineKeys.typeRoutine)) {
        final routine = byId(id);
        final at = routine == null ? null : purgeAt(routine);
        // Only where this Device may change it (ADR 0013).
        if (at == null || now.isBefore(at) || !store.mayChangeRoutine(id)) {
          continue;
        }
        store.append(id, RoutineKeys.type, null);
      }
    });
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

  /// The Routine [id], trashed or not, or null when there is none.
  Routine? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[RoutineKeys.type] != RoutineKeys.typeRoutine) return null;
    final noteId = fields[RoutineKeys.note];
    final schedule = scheduleOf(fields);
    if (noteId == null || schedule == null) return null;
    final renag = int.tryParse(fields[RoutineKeys.renag] ?? '');
    final escalation = int.tryParse(fields[RoutineKeys.escalation] ?? '');
    final trashed = fields[RoutineKeys.trashed];
    DateTime? scheduledAt;
    final history = store.history(id);
    for (final entry in history) {
      // Leaving the trash bounds what was asked for like a new schedule.
      if (!RoutineKeys.scheduleFields.contains(entry.field) &&
          entry.field != RoutineKeys.trashed) {
        continue;
      }
      if (scheduledAt == null || entry.date.isAfter(scheduledAt)) {
        scheduledAt = entry.date;
      }
    }
    return Routine(
      id: id,
      noteId: noteId,
      schedule: schedule,
      renag: renag == null || renag < 1 ? null : Duration(minutes: renag),
      escalation: escalation == null || escalation < 1 ? null : escalation,
      scheduledAt: scheduledAt,
      trashed: trashed == null ? null : DateTime.parse(trashed),
      trashedWith: trashed == null ? null : fields[RoutineKeys.trashedWith],
      done: {
        // A Session's Occurrence is Done too once every pick has an
        // entry (ADR 0029); a check-off, written below, keeps its moment.
        if (fields[SessionKeys.pool] != null)
          ...SessionMarks.read(store, id, history).doneFromEntries(store),
        for (final MapEntry(:key, :value) in fields.entries)
          if (key.startsWith(RoutineKeys.donePrefix) && value != null)
            key.substring(RoutineKeys.donePrefix.length): DateTime.parse(value),
      },
    );
  }

  /// The schedule the Routine [fields] hold, or null when they hold none.
  static Schedule? scheduleOf(Map<String, String?> fields) {
    final start = Day.tryParse(fields[RoutineKeys.start] ?? '');
    if (start == null) return null;
    final ruleText = fields[RoutineKeys.rule];
    final rule = ruleText == null ? null : Recurrence.tryParse(ruleText);
    if (ruleText != null && rule == null) return null;
    final times = [
      for (final text in (fields[RoutineKeys.times] ?? '').split(','))
        ?ClockTime.tryParse(text),
    ];
    try {
      return Schedule(start: start, times: times, rule: rule);
    } on ArgumentError {
      return null;
    }
  }

  /// Every Routine and Reminder not in the trash.
  List<Routine> all() => [
    for (final id in store.entitiesOfType(RoutineKeys.typeRoutine))
      if (byId(id) case final routine? when !routine.isTrashed) routine,
  ];

  /// The Note [routine] links back to: the Note it was spawned from, or
  /// the one that Note was merged into; it may be in the trash. Null when
  /// it is gone for good.
  Note? note(Routine routine) {
    final notes = Notes(store);
    final note = notes.byId(routine.noteId);
    final into = note?.mergedInto;
    return into == null ? note : notes.byId(into);
  }

  /// The Routines and Reminders the Note [noteId] spawned, those of the
  /// Notes merged into it included, without those in the trash.
  List<Routine> ofNote(String noteId) => [
    for (final routine in all())
      if (note(routine)?.id == noteId) routine,
  ];

  /// The Agenda at the store's now, with days as [wallClock] shows them,
  /// over [days] days from today; never fewer than today and tomorrow.
  Agenda agenda(WallClock wallClock, {int days = 2}) {
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
      laterRows: [for (var i = 2; i < days; i++) rowsOn(today.addDays(i))],
    );
  }

  /// Writes the fields of [schedule] that differ from what is there.
  void _writeSchedule(String id, Schedule schedule) {
    final current = store.currentFields(id);
    void write(String field, String? value) {
      if (current.containsKey(field) && current[field] == value) return;
      store.append(id, field, value);
    }

    write(RoutineKeys.start, schedule.start.toString());
    write(RoutineKeys.times, schedule.times.join(','));
    write(RoutineKeys.rule, schedule.rule?.format());
  }

  // The writes behind restore, for use inside one batch.
  void _untrash(Routine routine) {
    store.append(routine.id, RoutineKeys.trashed, null);
    if (routine.trashedWith != null) {
      store.append(routine.id, RoutineKeys.trashedWith, null);
    }
  }

  String _now() => store.clock().toUtc().toIso8601String();

  /// The Routine [id], which must not be in the trash.
  Routine _require(String id) {
    final routine = byId(id);
    if (routine == null || routine.isTrashed) {
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
