import 'dart:async';

import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_rows.dart';
import 'agenda_widget.dart';
import 'alarm_platform.dart';
import 'app_lock.dart';
import 'l10n.dart';
import 'profile.dart';
import 'schedule_text.dart';
import 'sync_runner.dart';

/// One read of the Agenda as plain data (#207): its days without the
/// calendar's appointments, today's Frogs, the Missed Summary and the
/// Unsorted count, read at the log's mark [seq].
class AgendaLoad {
  final AgendaDays days;
  final FrogsToday frogs;
  final MissedSummary missed;
  final int unsorted;

  /// The log's mark the rows were read at, after the picks the read
  /// wrote: a result whose store has moved on since is not shown.
  final int seq;

  /// Whether the load wrote today's picks. On a connection of its own,
  /// the store the app reads heard of them before the load arrived
  /// ([runIsolated]).
  final bool wrote;

  const AgendaLoad({
    required this.days,
    required this.frogs,
    required this.missed,
    required this.unsorted,
    required this.seq,
    this.wrote = false,
  });
}

/// The alarms' plan with the texts its notifications show, read at the
/// log's mark [seq] (#207); all [AlarmSync] hands the platform but the
/// labels and the sound.
class AlarmPlanRead {
  /// Whether any Routine escalates.
  final bool escalating;
  final List<ArmedAlarm> armed;
  final List<(String, Occurrence)> done;
  final ArmedNudge? nudge;
  final ArmedChangesNudge? changes;
  final int seq;

  const AlarmPlanRead({
    required this.escalating,
    required this.armed,
    required this.done,
    required this.nudge,
    required this.changes,
    required this.seq,
  });
}

/// Where the Agenda's reads and the alarms' plan run (#207): the app
/// runs them in an isolate of their own ([IsolateAgendaReader]), so the
/// screen draws on while they read; tests on the store they read
/// ([InlineAgendaReader]), whose answers come at once. Each is handed
/// the store the app reads: the inline reader reads it, the isolate's
/// tells it what its own connection wrote.
abstract interface class AgendaReader {
  /// The Agenda as it loads, with the [system] its targets show in: it
  /// writes today's Session and Frog picks when they are new, then reads.
  FutureOr<AgendaLoad> load(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  );

  /// The Agenda as the widget's snapshot reads it without the Agenda,
  /// writing nothing: a pick not made yet shows as the Agenda will make
  /// it.
  FutureOr<AgendaLoad> read(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  );

  /// The alarms' plan with its texts in [t] for [locale].
  FutureOr<AlarmPlanRead> plan(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  );
}

/// Reads on the store the app reads, in its isolate; the answers come
/// at once.
class InlineAgendaReader implements AgendaReader {
  const InlineAgendaReader();

  @override
  AgendaLoad load(Store store, WallClock wallClock, UnitSystem system) =>
      loadAgenda(store, wallClock, system);

  @override
  AgendaLoad read(Store store, WallClock wallClock, UnitSystem system) =>
      AgendaSnapshot.readLog(store, wallClock, system);

  @override
  AlarmPlanRead plan(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) => planAlarms(store, wallClock, t, locale);
}

/// Reads in an isolate of its own, on a connection of its own to the
/// database at [path] (SQLite WAL), as the sync does (#225): the app
/// keeps drawing meanwhile and gets plain data back. The picks a load
/// writes there, the store handed in hears of before the load arrives
/// ([runIsolated]), whether anyone still shows it or not.
class IsolateAgendaReader implements AgendaReader {
  final String path;

  const IsolateAgendaReader(this.path);

  @override
  Future<AgendaLoad> load(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) => _load(store, path, wallClock, system, writes: true);

  @override
  Future<AgendaLoad> read(
    Store store,
    WallClock wallClock,
    UnitSystem system,
  ) => _load(store, path, wallClock, system, writes: false);

  @override
  Future<AlarmPlanRead> plan(
    Store store,
    WallClock wallClock,
    AppLocalizations t,
    String locale,
  ) => _plan(store, path, wallClock, t, locale);
}

// Static, so the closures that cross into the isolate hold only what
// they are handed.

Future<AgendaLoad> _load(
  Store store,
  String path,
  WallClock wallClock,
  UnitSystem system, {
  required bool writes,
}) => runIsolated(
  store,
  path,
  (own) => writes
      ? loadAgenda(own, wallClock, system)
      : readAgenda(own, wallClock, system),
);

Future<AlarmPlanRead> _plan(
  Store store,
  String path,
  WallClock wallClock,
  AppLocalizations t,
  String locale,
) => runIsolated(store, path, (own) async {
  // The isolate's own dates in the reader's language, as the app's
  // localizations load them there.
  await initializeDateFormatting(locale);
  return planAlarms(own, wallClock, t, locale);
});

/// The Agenda's load of [store]: today's Session and Frog picks written
/// when they are new, then its days, Frogs, Missed Summary and Unsorted
/// count read, at the mark after the picks.
AgendaLoad loadAgenda(Store store, WallClock wallClock, UnitSystem system) {
  final before = store.currentSeq();
  // Written before the rows read their Done.
  final sessions = Sessions(store).pickToday(wallClock);
  final frogs = Frogs(store).pickToday(wallClock);
  return _read(store, wallClock, system, sessions, frogs, before: before);
}

/// The Agenda of [store] as [loadAgenda] reads it, writing nothing.
AgendaLoad readAgenda(Store store, WallClock wallClock, UnitSystem system) =>
    _read(
      store,
      wallClock,
      system,
      Sessions(store).today(wallClock),
      Frogs(store).today(wallClock),
    );

AgendaLoad _read(
  Store store,
  WallClock wallClock,
  UnitSystem system,
  List<SessionOccurrence> sessions,
  FrogsToday frogs, {
  int? before,
}) {
  final seq = store.currentSeq();
  return AgendaLoad(
    days: AgendaDays.read(
      store: store,
      places: Places(store),
      wallClock: wallClock,
      days: AgendaDays.shown,
      sessions: sessions,
      frogsToday: frogs,
      system: system,
    ),
    frogs: frogs,
    missed: Alarms(store).missed(wallClock),
    unsorted: Notes(store).unsorted().length,
    seq: seq,
    wrote: before != null && before != seq,
  );
}

/// The alarms' plan of [store] with the texts its notifications show in
/// [t] for [locale]: a Routine from a locked Area set to Masked shows
/// its masked text, as on the lock screen.
AlarmPlanRead planAlarms(
  Store store,
  WallClock wallClock,
  AppLocalizations t,
  String locale,
) {
  final seq = store.currentSeq();
  // Read once for the plan, its titles and its Done (#228), where each
  // alarm read its Routine again.
  final routines = Routines(store).all();
  final byId = {for (final routine in routines) routine.id: routine};
  final locks = AreaLocks(store);
  final masked = <String, bool>{};
  final alarms = Alarms(store);
  String title(PlannedAlarm alarm) {
    final routine = byId[alarm.routineId];
    // From a locked Area set to Masked, on the lock screen too.
    if (routine != null &&
        masked.putIfAbsent(routine.id, () => locks.masks(routine))) {
      return maskedText(t, locale, routine, alarm.occurrence);
    }
    // A photo or voice Note without words is still there.
    return _title(alarm.text) ??
        (alarm.text == null ? t.noteGone : wordlessTitle(t, alarm.wordless));
  }

  final planned = alarms.nudge(wallClock);
  final changes = alarms.changesNudge(wallClock);
  return AlarmPlanRead(
    escalating: routines.any((routine) => routine.escalation != null),
    armed: [
      for (final alarm in alarms.plan(wallClock, routines: routines))
        ArmedAlarm(
          alarm: alarm,
          title: title(alarm),
          body: occurrenceTime(t, locale, alarm.occurrence),
        ),
    ],
    done: alarms.recentlyDone(wallClock, routines: routines),
    nudge: planned == null
        ? null
        : ArmedNudge(
            nudge: planned,
            title: t.nudgeTitle(planned.count),
            body: t.nudgeBody,
          ),
    changes: changes == null
        ? null
        : ArmedChangesNudge(
            nudge: changes,
            title: t.changesNudgeTitle(
              namesText(t, [
                for (final person in changes.people)
                  profileDisplayName(t, store.personShown(person).name),
              ]),
            ),
            body: t.changesNudgeBody,
            public: t.changesNudgePublic,
          ),
    seq: seq,
  );
}

/// The first line of a Note's text, or null when there is none.
String? _title(String? text) {
  final line = text?.trim().split('\n').first.trim();
  return line == null || line.isEmpty ? null : line;
}
