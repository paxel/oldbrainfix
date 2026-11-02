import 'dart:convert';

import 'package:crypto/crypto.dart';

import 'entry.dart';
import 'note_kinds.dart';
import 'notes.dart';
import 'profiles.dart';
import 'routines.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'tallies.dart';

/// Field keys of a Session (ADR 0029, 0039): its setup on the entity of
/// its Note, written there since #191 and read from the Routine as an
/// older version wrote it; its picks on the entity of each Routine that
/// runs it.
abstract final class SessionKeys {
  /// The pool: the Notes of its Tallies, in the order Rolling goes
  /// through them, joined by commas. A Note with a pool is a Session.
  static const pool = 'session:pool';

  /// How many Tallies an Occurrence picks.
  static const count = 'session:count';

  /// How many an Occurrence on [weekday] picks (1 for Monday to 7 for
  /// Sunday), instead of [count]; null for none.
  static String countOn(int weekday) => '$count:$weekday';

  /// The picking rule, a [SessionRule]'s name.
  static const rule = 'session:rule';

  /// Every field of the setup.
  static final setup = [
    pool,
    count,
    for (var weekday = 1; weekday <= 7; weekday++) countOn(weekday),
    rule,
  ];

  /// The pick of an Occurrence: the Notes of its Tallies joined by
  /// commas. Keyed by the wall-clock Occurrence, as `done@` is.
  static String pick(Occurrence occurrence) => '$pickPrefix${occurrence.key}';

  static const pickPrefix = 'pick@';

  /// An entry recorded for a pick of [occurrence], its value the Note of
  /// the Tally it went to.
  static String entry(Occurrence occurrence, String entryId) =>
      '$entryPrefix${occurrence.key}@$entryId';

  static const entryPrefix = 'entry@';

  /// The highest count a Session takes.
  static const maxCount = 99;
}

/// How a Session picks its Tallies (CONTEXT.md).
enum SessionRule {
  /// A new random pick each time, the same on every Device.
  random,

  /// Through the pool in its order, continuing where it stopped.
  rolling,

  /// The Tallies with the fewest entries first.
  least,
}

/// A Session as its Note holds it (#191): the pool, the counts and the
/// rule each Routine of the Note runs.
class SessionSetup {
  /// The Note that is the Session.
  final String noteId;

  /// The Notes of its Tallies as stored, in the order Rolling goes
  /// through them; see [Sessions.pool] for those it picks from.
  final List<String> pool;

  /// How many Tallies an Occurrence picks.
  final int count;

  /// The count of a weekday (1 for Monday to 7 for Sunday) that has its
  /// own.
  final Map<int, int> counts;
  final SessionRule rule;

  /// The Routine an older version wrote the setup on, while the Note
  /// holds none of its own (ADR 0039); null once it does.
  final String? heldBy;

  const SessionSetup({
    required this.noteId,
    required this.pool,
    required this.count,
    required this.counts,
    required this.rule,
    this.heldBy,
  });

  /// How many Tallies an Occurrence on [day] picks.
  int countOn(Day day) => counts[day.weekday] ?? count;
}

/// A Routine that runs the Session of its Note: it picks Tallies from
/// the Note's pool for each Occurrence (CONTEXT.md).
class Session {
  final Routine routine;
  final SessionSetup setup;

  const Session({required this.routine, required this.setup});

  /// The Routine's id: its picks lie on it.
  String get id => routine.id;

  /// The Note whose Session it runs.
  String get noteId => setup.noteId;

  List<String> get pool => setup.pool;
  int get count => setup.count;
  Map<int, int> get counts => setup.counts;
  SessionRule get rule => setup.rule;

  /// How many Tallies an Occurrence on [day] picks.
  int countOn(Day day) => setup.countOn(day);
}

/// One Tally a Session's Occurrence picked.
class SessionPick {
  /// The Tally's Note.
  final String noteId;

  /// The Note, or null when this Device holds none.
  final Note? note;

  /// The Tally with its fields and targets, or null when the Note is
  /// none any more.
  final Tally? tally;

  /// The entries recorded for this pick of the Occurrence, not removed,
  /// oldest first, anyone's.
  final List<TallyEntry> entries;

  const SessionPick({
    required this.noteId,
    required this.note,
    required this.tally,
    required this.entries,
  });

  bool get done => entries.isNotEmpty;
}

/// A Session's Occurrence with its picks.
class SessionOccurrence {
  final Session session;
  final Occurrence occurrence;
  final List<SessionPick> picks;

  /// Whether "Another one" has a Tally to swap in.
  final bool canSwap;

  const SessionOccurrence({
    required this.session,
    required this.occurrence,
    required this.picks,
    required this.canSwap,
  });

  /// Whether every pick has an entry: the Occurrence is Done then.
  bool get complete => picks.isNotEmpty && picks.every((p) => p.done);
}

/// What a Session's Routine holds of its picks and of the entries
/// recorded for them, by Occurrence key, only what counts: in a Shared
/// Area what the roles allow (ADR 0013), anywhere else what the user's
/// own Devices wrote.
class SessionMarks {
  /// The current pick of each Occurrence: the one its field holds, with
  /// every Tally that has an entry recorded for it in it.
  final Map<String, List<String>> picks;

  /// The pick each Occurrence's field holds, as written.
  final Map<String, List<String>> stored;

  /// Every Tally a pick of each Occurrence ever held.
  final Map<String, Set<String>> offered;

  /// The entries recorded for each Occurrence: the Tally's Note by entry
  /// id.
  final Map<String, Map<String, String>> entries;

  /// The entries of [entries] not removed, by Occurrence key and the
  /// Tally's Note, oldest first: read with the marks, in one query.
  final Map<String, Map<String, List<TallyEntry>>> _recorded;

  SessionMarks._(this.stored, this.offered, this.entries, this._recorded)
    : picks = {for (final MapEntry(:key, :value) in stored.entries) key: value};

  /// The marks of the Routine [id], read once until the log changes
  /// (#204) and shared by every caller, so nobody changes them.
  static SessionMarks of(Store store, String id) =>
      store.workedOut((SessionMarks, id), () => SessionMarks.read(store, id));

  /// Reads the marks of the Routine [id] from [history], its own by
  /// default.
  factory SessionMarks.read(Store store, String id, [List<Entry>? history]) {
    final area = store.areaOf(id);
    final shared = area != null && store.isShared(area);
    final own = <String, bool>{};
    final latest = <String, Entry>{};
    final offered = <String, Set<String>>{};
    // The place each Tally had in the latest pick that held it.
    final places = <String, Map<String, (Entry, int)>>{};
    for (final e in history ?? store.history(id)) {
      final pick = e.field.startsWith(SessionKeys.pickPrefix);
      if (!pick && !e.field.startsWith(SessionKeys.entryPrefix)) continue;
      if (!shared &&
          !own.putIfAbsent(e.device, () => store.writtenByOwn(e.device))) {
        continue;
      }
      final held = latest[e.field];
      if (held == null || Store.latestWins(e, held) > 0) latest[e.field] = e;
      final key = pick
          ? _occurrenceKey(e.field.substring(SessionKeys.pickPrefix.length))
          : null;
      if (key != null) {
        final ids = _noteIds(e.value);
        offered.putIfAbsent(key, () => {}).addAll(ids);
        final placed = places.putIfAbsent(key, () => {});
        for (final (i, noteId) in ids.indexed) {
          final was = placed[noteId];
          if (was == null || Store.latestWins(e, was.$1) > 0) {
            placed[noteId] = (e, i);
          }
        }
      }
    }
    final stored = <String, List<String>>{};
    final entries = <String, Map<String, String>>{};
    for (final MapEntry(key: field, value: e) in latest.entries) {
      if (field.startsWith(SessionKeys.pickPrefix)) {
        final key = _occurrenceKey(
          field.substring(SessionKeys.pickPrefix.length),
        );
        final ids = _noteIds(e.value);
        if (key != null && ids.isNotEmpty) stored[key] = ids;
        continue;
      }
      final rest = field.substring(SessionKeys.entryPrefix.length);
      final at = rest.indexOf('@');
      if (at <= 0) continue;
      final key = _occurrenceKey(rest.substring(0, at));
      final entryId = rest.substring(at + 1);
      final note = e.value;
      if (key == null ||
          !_isId(entryId, TallyKeys.entryPrefix) ||
          note == null ||
          !_isId(note, _notePrefix)) {
        continue;
      }
      entries.putIfAbsent(key, () => {})[entryId] = note;
    }
    final found = Tallies(store)
        .entriesById({for (final recorded in entries.values) ...recorded.keys});
    final recorded = <String, Map<String, List<TallyEntry>>>{};
    for (final MapEntry(:key, value: byEntry) in entries.entries) {
      for (final MapEntry(key: entryId, value: note) in byEntry.entries) {
        if (found[entryId] case final entry? when !entry.isTrashed) {
          recorded
              .putIfAbsent(key, () => {})
              .putIfAbsent(note, () => [])
              .add(entry);
        }
      }
    }
    for (final byNote in recorded.values) {
      for (final list in byNote.values) {
        list.sort((a, b) {
          final byAt = a.at.compareTo(b.at);
          return byAt != 0 ? byAt : a.id.compareTo(b.id);
        });
      }
    }
    return SessionMarks._(stored, offered, entries, recorded)
      .._heal(store, places);
  }

  /// Puts each Tally with an entry recorded for an Occurrence back into
  /// its current pick when the field no longer holds it, as when a pick
  /// another Device wrote later wins over the one it was recorded for:
  /// in the place it had in the latest pick that held it, while the
  /// Tally there has no entry, else at the end.
  void _heal(Store store, Map<String, Map<String, (Entry, int)>> places) {
    for (final MapEntry(:key, value: recorded) in entries.entries) {
      final pick = [...?picks[key]];
      final missing = {
        for (final noteId in recorded.values)
          if (!pick.contains(noteId)) noteId,
      }.toList()..sort();
      var healed = false;
      for (final noteId in missing) {
        if (entriesFor(store, key, noteId).isEmpty) continue;
        final place = places[key]?[noteId]?.$2;
        if (place != null &&
            place < pick.length &&
            entriesFor(store, key, pick[place]).isEmpty) {
          pick[place] = noteId;
        } else {
          pick.add(noteId);
        }
        healed = true;
      }
      if (healed) picks[key] = pick;
    }
  }

  /// The entries recorded for [noteId] in the Occurrence [key], not
  /// removed, oldest first, as they were read with the marks.
  List<TallyEntry> entriesFor(Store store, String key, String noteId) => [
    ...?_recorded[key]?[noteId],
  ];

  /// The Occurrences whose every pick has an entry, by key, each with
  /// the moment the last of them got its first.
  Map<String, DateTime> doneFromEntries(Store store) {
    final done = <String, DateTime>{};
    for (final MapEntry(:key, value: ids) in picks.entries) {
      DateTime? last;
      var whole = true;
      for (final id in ids) {
        final first = entriesFor(store, key, id).firstOrNull?.at;
        if (first == null) {
          whole = false;
          break;
        }
        if (last == null || first.isAfter(last)) last = first;
      }
      if (whole && last != null) done[key] = last;
    }
    return done;
  }

  /// [text] when it is an Occurrence's key as the apps write it.
  static String? _occurrenceKey(String text) =>
      Occurrence.tryParse(text)?.key == text ? text : null;

  /// The Notes [value] lists, each once, skipping what is no Note id.
  static List<String> _noteIds(String? value) {
    final ids = <String>[];
    for (final id in (value ?? '').split(',')) {
      if (_isId(id, _notePrefix) && !ids.contains(id)) ids.add(id);
    }
    return ids;
  }

  static bool _isId(String id, String prefix) =>
      id.length > prefix.length &&
      id.startsWith(prefix) &&
      !id.contains(',') &&
      !id.contains('@');

  static const _notePrefix = 'note:';
}

/// Setting up Sessions on their Notes, and picking their Tallies for
/// the Routines that run them.
class Sessions {
  final Store store;

  Sessions(this.store);

  /// Makes the Note [noteId] a Session (#191): it picks [count] Tallies
  /// of [pool] (their Notes, in the order Rolling goes through them) by
  /// [rule] for each Occurrence of every Routine of the Note. Only a
  /// plain Note in the lists becomes one; the pool takes [choices] only.
  /// It leaves Unsorted, as a Tally does.
  void make(
    String noteId, {
    required List<String> pool,
    required int count,
    required SessionRule rule,
  }) {
    _requireChangeable(noteId);
    if (NoteKinds(store).of(noteId) != NoteKind.plain) {
      throw ArgumentError.value(noteId, 'noteId', 'not a plain Note');
    }
    final chosen = _chosen(store.areaOf(noteId), pool);
    _checkCount(count);
    store.batch(() {
      _writeSetup(noteId, chosen, count, rule);
      if (!Notes(store).byId(noteId)!.spawned) {
        store.append(
          noteId,
          NoteKeys.spawned,
          store.clock().toUtc().toIso8601String(),
        );
      }
    });
  }

  /// Makes the Session [noteId] a plain Note again: its pool is taken
  /// away, so its Routines run as plain ones; their picks and entries
  /// stay in the log.
  void unmake(String noteId) {
    _requireSetup(noteId);
    _requireChangeable(noteId);
    store.append(noteId, SessionKeys.pool, null);
  }

  /// Spawns a Routine on [schedule] from the Note [noteId] and sets the
  /// Note up as a Session of [pool], [count] and [rule], as [make] does,
  /// in one batch; a Session already gets that setup. The Session the
  /// new Routine runs.
  Session spawn(
    String noteId,
    Schedule schedule, {
    required List<String> pool,
    required int count,
    required SessionRule rule,
  }) {
    final held = setupOf(noteId);
    if (held == null && NoteKinds(store).of(noteId) != NoteKind.plain) {
      throw ArgumentError.value(noteId, 'noteId', 'not a plain Note');
    }
    final chosen = _chosen(
      store.areaOf(noteId),
      pool,
      held: held?.pool ?? const [],
    );
    _checkCount(count);
    if (held != null) _requireChangeable(noteId);
    final routine = Routines(store).spawn(
      noteId,
      schedule,
      within: (_) {
        if (held == null) {
          _writeSetup(noteId, chosen, count, rule);
        } else {
          _adopt(held);
          _write(noteId, SessionKeys.pool, chosen.join(','));
          _write(noteId, SessionKeys.count, '$count');
          _write(noteId, SessionKeys.rule, rule.name);
        }
      },
    );
    return byId(routine.id)!;
  }

  /// Creates a Session without a Note at hand: its Note, holding [text],
  /// is captured behind the scenes (ADR 0005) and set up with a Routine
  /// on [schedule], as [spawn] does.
  Session create(
    String text,
    Schedule schedule, {
    required List<String> pool,
    required int count,
    required SessionRule rule,
  }) {
    _chosen(null, pool);
    _checkCount(count);
    final trimmed = text.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(text, 'text', 'must not be empty');
    }
    return spawn(
      Notes(store).capture(trimmed).id,
      schedule,
      pool: pool,
      count: count,
      rule: rule,
    );
  }

  /// Gives the Session [noteId] the pool [pool]: [choices], and those it
  /// holds already.
  void setPool(String noteId, List<String> pool) {
    final setup = _requireManaged(noteId);
    final chosen = _chosen(store.areaOf(noteId), pool, held: setup.pool);
    _change(setup, SessionKeys.pool, chosen.join(','));
  }

  /// Sets how many Tallies an Occurrence of the Session [noteId] picks.
  void setCount(String noteId, int count) {
    final setup = _requireManaged(noteId);
    _checkCount(count);
    _change(setup, SessionKeys.count, '$count');
  }

  /// Sets how many an Occurrence on [weekday] (1 for Monday to 7 for
  /// Sunday) picks; null takes the weekday's own count away.
  void setCountOn(String noteId, int weekday, int? count) {
    final setup = _requireManaged(noteId);
    if (weekday < 1 || weekday > 7) {
      throw ArgumentError.value(weekday, 'weekday', '1 to 7');
    }
    if (count != null) _checkCount(count);
    _change(setup, SessionKeys.countOn(weekday), count?.toString());
  }

  /// Sets how the Session [noteId] picks.
  void setRule(String noteId, SessionRule rule) {
    final setup = _requireManaged(noteId);
    _change(setup, SessionKeys.rule, rule.name);
  }

  /// The Session the Note [noteId] is, or null when it is none: the setup
  /// on the Note, or, while the Note holds none, on the Routine of it an
  /// older version wrote it on (ADR 0039), of several the one whose
  /// setup was written last. A Note made plain again holds a pool of
  /// null and is none.
  SessionSetup? setupOf(String noteId) {
    final setups = store.workedOut(_Setups, () => _Setups.read(store));
    if (setups.own[noteId] case final fields?) {
      return _setupFrom(noteId, fields, null);
    }
    if (setups.left[noteId] case (final heldBy, final fields)) {
      return _setupFrom(noteId, fields, heldBy);
    }
    return null;
  }

  /// Whether the Note [noteId] is a Session.
  bool isSessionNote(String noteId) => setupOf(noteId) != null;

  /// The Routine [id] running the Session of its Note, trashed or not, or
  /// null when it is no Routine or its Note is no Session.
  Session? byId(String id) {
    final routine = Routines(store).byId(id);
    if (routine == null) return null;
    final setup = setupOf(routine.noteId);
    if (setup == null) return null;
    return Session(routine: routine, setup: setup);
  }

  /// Whether the Routine [id] runs a Session.
  bool isSession(String id) => byId(id) != null;

  /// Every Routine not in the trash that runs a Session, by id.
  List<Session> all() {
    final setups = <String, SessionSetup?>{};
    final sessions = [
      for (final routine in Routines(store).all())
        if (setups.putIfAbsent(routine.noteId, () => setupOf(routine.noteId))
            case final setup?)
          Session(routine: routine, setup: setup),
    ];
    sessions.sort((a, b) => a.id.compareTo(b.id));
    return sessions;
  }

  /// The Routines not in the trash spawned from the Session [noteId],
  /// each running it, by id; none while the Note is no Session.
  List<Session> ofNote(String noteId) {
    final setup = setupOf(noteId);
    if (setup == null) return const [];
    final sessions = [
      for (final routine in Routines(store).all())
        if (routine.noteId == noteId) Session(routine: routine, setup: setup),
    ];
    sessions.sort((a, b) => a.id.compareTo(b.id));
    return sessions;
  }

  /// The Tallies a Session of [entity] may pick, as their Notes, newest
  /// capture first: [entity] is the Note of the Session, or one of its
  /// Routines. Every Tally in the lists, but those outside its Area while
  /// that is a Shared Area, which not everyone in it holds.
  List<String> choices(String entity) {
    final area = store.areaOf(entity);
    return [
      for (final tally in Tallies(store).all())
        if (_fits(area, tally.noteId)) tally.noteId,
    ];
  }

  /// The Tallies [setup] picks from, as their Notes, in its order: those
  /// of its pool that are Tallies in the lists, a merged one as the Note
  /// it went into, each once, and in a Shared Area only those of that
  /// Area.
  List<String> pool(SessionSetup setup) {
    final area = store.areaOf(setup.noteId);
    final notes = Notes(store);
    final tallies = Tallies(store);
    final out = <String>[];
    for (final id in setup.pool) {
      final shown = notes.byId(id)?.mergedInto ?? id;
      final note = notes.byId(shown);
      if (note == null || note.isTrashed || note.isMerged) continue;
      if (out.contains(shown) || !_fits(area, shown)) continue;
      if (tallies.isTally(shown)) out.add(shown);
    }
    return out;
  }

  /// The Occurrence [occurrence] of the Session [id] with its picks, as
  /// [pickToday] would write them when there are none yet: written
  /// nowhere.
  SessionOccurrence on(String id, Occurrence occurrence) {
    final session = _requireDue(id, occurrence);
    final marks = SessionMarks.of(store, id);
    final pool = this.pool(session.setup);
    final picks = _wanted(session, occurrence, pool, marks, _runners(session));
    return _shown(session, occurrence, picks, pool, marks);
  }

  /// Today's Occurrences of every Session on [wallClock] with their
  /// picks, by time, then by Session; written nowhere.
  List<SessionOccurrence> today(WallClock wallClock) =>
      _today(wallClock, write: false);

  /// Today's Occurrences of every Session on [wallClock] with their
  /// picks, each pick written for everyone the Session reaches when it
  /// is new: the first, or one that replaces Tallies gone since. The
  /// Agenda calls it as it shows; called again, it writes nothing.
  List<SessionOccurrence> pickToday(WallClock wallClock) =>
      _today(wallClock, write: true);

  /// "Another one": swaps the Tally of [noteId] picked for [occurrence]
  /// of the Session [id] for the next by the Session's rule, in its
  /// place: one this Occurrence has not offered yet, else the first not
  /// picked. The one swapped out keeps its place in the rule. Nothing
  /// happens when the pool holds no other, or when the pick has an entry.
  void another(String id, Occurrence occurrence, String noteId) {
    final session = _requireDue(id, occurrence);
    _requireCheckOff(id);
    final marks = SessionMarks.of(store, id);
    final pool = this.pool(session.setup);
    final runners = _runners(session);
    final wanted = _wanted(session, occurrence, pool, marks, runners);
    if (!wanted.contains(noteId)) {
      throw ArgumentError.value(noteId, 'noteId', 'not picked then');
    }
    // A pick with an entry stays in the pick.
    if (marks.entriesFor(store, occurrence.key, noteId).isNotEmpty) return;
    final offered = {...?marks.offered[occurrence.key], ...wanted};
    final order = _order(session, occurrence, pool, marks, runners);
    final next =
        order.where((t) => !offered.contains(t)).firstOrNull ??
        order.where((t) => !wanted.contains(t)).firstOrNull;
    if (next == null) return;
    store.batch(() {
      // The pick it swaps from counts as offered.
      _writePick(id, occurrence, wanted, marks);
      store.append(
        id,
        SessionKeys.pick(occurrence),
        [for (final picked in wanted) picked == noteId ? next : picked]
            .join(','),
      );
    });
  }

  /// Records an entry in the Tally of [noteId] picked for [occurrence]
  /// of the Session [id], with [values] by field id in the base unit and
  /// a text [line]. Once every pick has one, the Occurrence is Done for
  /// everyone the Session reaches.
  TallyEntry record(
    String id,
    Occurrence occurrence,
    String noteId, {
    Map<String, double> values = const {},
    String line = '',
  }) {
    final session = _requireDue(id, occurrence);
    _requireCheckOff(id);
    final marks = SessionMarks.of(store, id);
    final pool = this.pool(session.setup);
    final wanted = _wanted(session, occurrence, pool, marks, _runners(session));
    if (!wanted.contains(noteId)) {
      throw ArgumentError.value(noteId, 'noteId', 'not picked then');
    }
    return Tallies(store).add(
      noteId,
      values: values,
      line: line,
      within: (entryId) {
        _writePick(id, occurrence, wanted, marks);
        store.append(id, SessionKeys.entry(occurrence, entryId), noteId);
      },
    );
  }

  List<SessionOccurrence> _today(WallClock wallClock, {required bool write}) {
    final day = Day.today(store.clock(), wallClock);
    // By time, then by Routine, so that Rolling over several Routines of
    // a Note goes on from the pick written before (ADR 0039).
    final sessions = all();
    final routines = [for (final session in sessions) session.routine];
    final due = [
      for (final session in sessions)
        for (final occurrence in session.routine.schedule.on(day))
          (session, occurrence),
    ];
    due.sort((a, b) {
      final byTime = a.$2.compareTo(b.$2);
      return byTime != 0 ? byTime : a.$1.id.compareTo(b.$1.id);
    });
    final shown = <SessionOccurrence>[];
    for (final (session, occurrence) in due) {
      final pool = this.pool(session.setup);
      var marks = SessionMarks.of(store, session.id);
      final runners = _runners(session, routines);
      final picks = _wanted(session, occurrence, pool, marks, runners);
      if (write &&
          store.mayCheckOff(session.id) &&
          _writePick(session.id, occurrence, picks, marks)) {
        marks = SessionMarks.of(store, session.id);
      }
      shown.add(_shown(session, occurrence, picks, pool, marks));
    }
    return shown;
  }

  SessionOccurrence _shown(
    Session session,
    Occurrence occurrence,
    List<String> picks,
    List<String> pool,
    SessionMarks marks,
  ) {
    final notes = Notes(store);
    final tallies = Tallies(store);
    return SessionOccurrence(
      session: session,
      occurrence: occurrence,
      picks: [
        for (final id in picks)
          SessionPick(
            noteId: id,
            note: notes.byId(id),
            tally: tallies.byNote(id),
            entries: marks.entriesFor(store, occurrence.key, id),
          ),
      ],
      canSwap: pool.any((id) => !picks.contains(id)),
    );
  }

  /// The pick [occurrence] should have: the one held, with what left the
  /// pool replaced in its place, unless it has an entry, and brought to
  /// the day's count; with none held, the first of the rule's order.
  List<String> _wanted(
    Session session,
    Occurrence occurrence,
    List<String> pool,
    SessionMarks marks,
    List<String> runners,
  ) {
    final count = session.countOn(occurrence.day);
    final order = _order(session, occurrence, pool, marks, runners);
    final held = marks.picks[occurrence.key];
    if (held == null) return order.take(count).toList();
    final offered = marks.offered[occurrence.key] ?? const <String>{};
    final next = [
      for (final id in order)
        if (!offered.contains(id)) id,
      for (final id in order)
        if (offered.contains(id) && !held.contains(id)) id,
    ].iterator;
    bool recorded(String id) =>
        marks.entriesFor(store, occurrence.key, id).isNotEmpty;
    final wanted = <String>[];
    for (final id in held) {
      if (pool.contains(id) || recorded(id)) {
        wanted.add(id);
      } else if (next.moveNext()) {
        wanted.add(next.current);
      }
    }
    while (wanted.length < count && next.moveNext()) {
      wanted.add(next.current);
    }
    for (var i = wanted.length - 1; i >= 0 && wanted.length > count; i--) {
      if (!recorded(wanted[i])) wanted.removeAt(i);
    }
    return wanted;
  }

  /// [pool] in the order [session]'s rule picks for [occurrence], with
  /// [runners] the Routines that run it ([_runners]).
  /// Rolling: those picked longest ago first, by the Occurrences before
  /// this one and their place in its pick, those never picked before
  /// them in pool order. Least selected: those with the fewest entries
  /// first, anyone's, then as Rolling. Random: by a hash of the Session,
  /// the Occurrence and the Tally, so every Device draws the same.
  List<String> _order(
    Session session,
    Occurrence occurrence,
    List<String> pool,
    SessionMarks marks,
    List<String> runners,
  ) {
    final index = {for (var i = 0; i < pool.length; i++) pool[i]: i};
    if (session.rule == SessionRule.random) {
      final draw = {
        for (final id in pool)
          id: sha256
              .convert(utf8.encode('${session.id}\n${occurrence.key}\n$id'))
              .toString(),
      };
      return [...pool]..sort((a, b) {
        final byDraw = (draw[a] ?? '').compareTo(draw[b] ?? '');
        return byDraw != 0 ? byDraw : a.compareTo(b);
      });
    }
    // The picks of the Occurrences before this one of every Routine that
    // runs the Session, in schedule order, those at the same moment by
    // their Routine (ADR 0039).
    final before = <(Occurrence, String, List<String>)>[];
    for (final runner in runners) {
      final held = runner == session.id
          ? marks
          : SessionMarks.of(store, runner);
      for (final MapEntry(:key, value: ids) in held.picks.entries) {
        final o = Occurrence.tryParse(key);
        if (o == null) continue;
        final byTime = o.compareTo(occurrence);
        if (byTime < 0 || byTime == 0 && runner.compareTo(session.id) < 0) {
          before.add((o, runner, ids));
        }
      }
    }
    before.sort((a, b) {
      final byTime = a.$1.compareTo(b.$1);
      return byTime != 0 ? byTime : a.$2.compareTo(b.$2);
    });
    // When each was picked last: later Occurrences count higher, and
    // within one, those further on in its pick.
    final last = <String, int>{};
    var picked = 0;
    for (final (_, _, ids) in before) {
      for (final id in ids) {
        last[id] = picked++;
      }
    }
    final tallies = Tallies(store);
    final entries = {
      if (session.rule == SessionRule.least)
        for (final id in pool) id: tallies.entries(id).length,
    };
    return [...pool]..sort((a, b) {
      final byEntries = (entries[a] ?? 0).compareTo(entries[b] ?? 0);
      if (byEntries != 0) return byEntries;
      final byLast = (last[a] ?? -1).compareTo(last[b] ?? -1);
      if (byLast != 0) return byLast;
      return (index[a] ?? 0).compareTo(index[b] ?? 0);
    });
  }

  /// Writes [ids] as the pick of [occurrence], unless its field holds
  /// that pick already or there is nothing to pick; whether it wrote.
  bool _writePick(
    String id,
    Occurrence occurrence,
    List<String> ids,
    SessionMarks marks,
  ) {
    if (ids.isEmpty) return false;
    final held = marks.stored[occurrence.key];
    if (held != null &&
        held.length == ids.length &&
        Iterable<int>.generate(ids.length).every((i) => held[i] == ids[i])) {
      return false;
    }
    store.append(id, SessionKeys.pick(occurrence), ids.join(','));
    return true;
  }

  /// [pool] without repeats, each a Tally a Session in [area] may pick or
  /// one of [held]; at least one.
  List<String> _chosen(
    String? area,
    List<String> pool, {
    List<String> held = const [],
  }) {
    final allowed = {
      for (final tally in Tallies(store).all())
        if (_fits(area, tally.noteId)) tally.noteId,
    };
    final chosen = <String>[];
    for (final id in pool) {
      if (chosen.contains(id)) continue;
      if (!allowed.contains(id) && !held.contains(id)) {
        throw ArgumentError.value(id, 'pool', 'not a Tally to pick');
      }
      chosen.add(id);
    }
    if (chosen.isEmpty) {
      throw ArgumentError.value(pool, 'pool', 'at least one Tally');
    }
    return chosen;
  }

  /// Whether a Session in [area] picks the Tally of [noteId]: in no
  /// Shared Area any Tally, in one only those of that Area.
  bool _fits(String? area, String noteId) =>
      area == null || !store.isShared(area) || store.areaOf(noteId) == area;

  void _write(String id, String field, String? value) {
    final current = store.currentFields(id);
    if (current.containsKey(field) && current[field] == value) return;
    store.append(id, field, value);
  }

  static void _checkCount(int count) {
    if (count < 1 || count > SessionKeys.maxCount) {
      throw ArgumentError.value(count, 'count', '1 to ${SessionKeys.maxCount}');
    }
  }

  /// The count [value] holds when written as the apps write one.
  static int? _countOf(String? value) {
    if (value == null || !RegExp(r'^[1-9][0-9]?$').hasMatch(value)) {
      return null;
    }
    return int.parse(value);
  }

  /// The Routine [id] running a Session, not in the trash.
  Session _require(String id) {
    final session = byId(id);
    if (session == null || session.routine.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not a Session');
    }
    return session;
  }

  /// The Routines that run [session]: those not in the trash spawned
  /// from its Note, and its own, by id. [routines] are those not in the
  /// trash, read once for many; null reads them.
  List<String> _runners(Session session, [List<Routine>? routines]) => {
    session.id,
    for (final routine in routines ?? Routines(store).all())
      if (routine.noteId == session.noteId) routine.id,
  }.toList()..sort();

  /// The setup of the Session [noteId].
  SessionSetup _requireSetup(String noteId) {
    final setup = setupOf(noteId);
    if (setup == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Session');
    }
    return setup;
  }

  /// The setup of the Session [noteId], in the lists, which this Device
  /// may change as it may change the Note (ADR 0013).
  SessionSetup _requireManaged(String noteId) {
    final setup = _requireSetup(noteId);
    _requireChangeable(noteId);
    return setup;
  }

  /// Requires the Note [noteId] in the lists, which this Device may
  /// change.
  void _requireChangeable(String noteId) {
    final note = Notes(store).byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
    }
    if (!store.mayChange(noteId)) {
      throw StateError('may not change $noteId');
    }
  }

  /// Writes a new setup on the Note [noteId], taking away the weekdays'
  /// counts an earlier one left.
  void _writeSetup(
    String noteId,
    List<String> pool,
    int count,
    SessionRule rule,
  ) {
    store.append(noteId, SessionKeys.pool, pool.join(','));
    store.append(noteId, SessionKeys.count, '$count');
    store.append(noteId, SessionKeys.rule, rule.name);
    final fields = store.currentFields(noteId);
    for (var weekday = 1; weekday <= 7; weekday++) {
      if (fields[SessionKeys.countOn(weekday)] != null) {
        store.append(noteId, SessionKeys.countOn(weekday), null);
      }
    }
  }

  /// Writes [field] of the Session [setup] as [value] on its Note, with
  /// the rest of a setup an older version left on a Routine in the same
  /// batch.
  void _change(SessionSetup setup, String field, String? value) {
    store.batch(() {
      _adopt(setup);
      _write(setup.noteId, field, value);
    });
  }

  /// Writes the setup an older version left on a Routine onto its Note,
  /// as it reads, so the Note holds it from now on (ADR 0039); nothing
  /// when the Note holds its own.
  void _adopt(SessionSetup setup) {
    if (setup.heldBy == null) return;
    final id = setup.noteId;
    store.append(id, SessionKeys.pool, setup.pool.join(','));
    store.append(id, SessionKeys.count, '${setup.count}');
    for (final MapEntry(key: weekday, value: count) in setup.counts.entries) {
      store.append(id, SessionKeys.countOn(weekday), '$count');
    }
    store.append(id, SessionKeys.rule, setup.rule.name);
  }

  /// The setup [fields] of the Note [noteId] hold, or null without a
  /// pool; [heldBy] is the Routine they lie on, if not on the Note.
  static SessionSetup? _setupFrom(
    String noteId,
    Map<String, String?> fields,
    String? heldBy,
  ) {
    final pool = fields[SessionKeys.pool];
    if (pool == null) return null;
    return SessionSetup(
      noteId: noteId,
      pool: SessionMarks._noteIds(pool),
      count: _countOf(fields[SessionKeys.count]) ?? 1,
      counts: {
        for (var weekday = 1; weekday <= 7; weekday++)
          weekday: ?_countOf(fields[SessionKeys.countOn(weekday)]),
      },
      rule:
          SessionRule.values.asNameMap()[fields[SessionKeys.rule]] ??
          SessionRule.rolling,
      heldBy: heldBy,
    );
  }

  /// The Session [id], due at [occurrence].
  Session _requireDue(String id, Occurrence occurrence) {
    final session = _require(id);
    if (!session.routine.schedule.has(occurrence)) {
      throw ArgumentError.value(occurrence, 'occurrence', 'not due then');
    }
    return session;
  }

  void _requireCheckOff(String id) {
    if (!store.mayCheckOff(id)) {
      throw StateError('no role in the Shared Area of $id');
    }
  }
}

/// Where the setups of the Sessions lie (ADR 0039), worked out in one
/// pass over what wrote a pool and kept until the log changes: a Note
/// is read once for many, and the Routines that never held a setup not
/// at all.
class _Setups {
  /// The fields of every entity that holds a pool of its own, of null
  /// too, by entity.
  final Map<String, Map<String, String?>> own;

  /// The setup an older version left on a Routine not in the trash, by
  /// its Note: the Routine and its fields; of several the one whose
  /// setup was written last.
  final Map<String, (String, Map<String, String?>)> left;

  _Setups(this.own, this.left);

  static _Setups read(Store store) {
    final own = <String, Map<String, String?>>{};
    final left = <String, (String, Map<String, String?>)>{};
    final newest = <String, Entry>{};
    final ids = {
      for (final (id, field) in store.fieldsWithPrefix(SessionKeys.pool))
        if (field == SessionKeys.pool) id,
    };
    for (final id in ids) {
      final fields = store.currentFields(id);
      if (!fields.containsKey(SessionKeys.pool)) continue;
      own[id] = fields;
      final noteId = fields[RoutineKeys.note];
      if (fields[RoutineKeys.type] != RoutineKeys.typeRoutine ||
          noteId == null ||
          fields[RoutineKeys.trashed] != null ||
          fields[SessionKeys.pool] == null) {
        continue;
      }
      for (final e in store.history(id)) {
        if (!SessionKeys.setup.contains(e.field)) continue;
        final held = newest[noteId];
        if (held == null || Store.latestWins(e, held) > 0) {
          newest[noteId] = e;
          left[noteId] = (id, fields);
        }
      }
    }
    return _Setups(own, left);
  }
}
