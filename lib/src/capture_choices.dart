import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The Note being captured as an editor opened from a capture screen
/// shows it (#113): its text so far, and the Area or Topic picked for it,
/// null while it is Unsorted. It is not written yet.
typedef CaptureDraft = ({String text, String? place});

/// A Session's pool, count, weekdays' counts and rule as plain values,
/// kept after its editor is gone.
typedef SessionChoice = ({
  List<String> pool,
  int count,
  Map<int, int> counts,
  SessionRule rule,
});

/// A Tally's first field as its page saves it, held on a capture screen
/// until the Note is written.
typedef TallyFieldChoice = ({
  String name,
  String unit,
  double? target,
  TallyRule rule,
});

/// A Routine, Reminder or Session set up on a capture screen before its
/// Note is written (#113): what its editor saves, held until Capture
/// writes it together with the Note.
class ScheduleChoice {
  final Schedule schedule;

  /// Its own Re-nag interval; null for the Device's default.
  final Duration? renag;

  /// After how many ignored Re-nags it escalates; null when it does not.
  final int? escalation;

  /// The Session's part; null for a plain Routine or Reminder.
  final SessionChoice? session;

  const ScheduleChoice(
    this.schedule, {
    this.renag,
    this.escalation,
    this.session,
  });

  /// Spawns it from the Note [noteId] in [store]; the Routine's id.
  String spawnFrom(Store store, String noteId) {
    final session = this.session;
    final routines = Routines(store);
    final id = session == null
        ? routines.spawn(noteId, schedule).id
        : spawnSession(store, noteId, schedule, session);
    routines.setRenag(id, renag);
    routines.setEscalation(id, escalation);
    return id;
  }
}

/// Spawns the Session [choice] from the Note [noteId] on [schedule]; its
/// id.
String spawnSession(
  Store store,
  String noteId,
  Schedule schedule,
  SessionChoice choice,
) {
  final sessions = Sessions(store);
  final id = sessions
      .spawn(
        noteId,
        schedule,
        pool: choice.pool,
        count: choice.count,
        rule: choice.rule,
      )
      .id;
  for (final MapEntry(key: day, value: own) in choice.counts.entries) {
    sessions.setCountOn(id, day, own);
  }
  return id;
}
