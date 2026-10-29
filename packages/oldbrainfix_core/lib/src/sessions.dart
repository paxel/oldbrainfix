import 'dart:convert';

import 'package:crypto/crypto.dart';

import 'entry.dart';
import 'notes.dart';
import 'profiles.dart';
import 'routines.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'tallies.dart';

/// Field keys of a Session (ADR 0029), on the entity of its Routine.
abstract final class SessionKeys {
  /// The pool: the Notes of its Tallies, in the order Rolling goes
  /// through them, joined by commas. A Routine with a pool is a Session.
  static const pool = 'session:pool';

  /// How many Tallies an Occurrence picks.
  static const count = 'session:count';

  /// How many an Occurrence on [weekday] picks (1 for Monday to 7 for
  /// Sunday), instead of [count]; null for none.
  static String countOn(int weekday) => '$count:$weekday';

  /// The picking rule, a [SessionRule]'s name.
  static const rule = 'session:rule';

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

/// A Routine that picks Tallies from a pool for each Occurrence
/// (CONTEXT.md).
class Session {
  final Routine routine;

  /// The Notes of its Tallies as stored, in the order Rolling goes
  /// through them; see [Sessions.pool] for those it picks from.
  final List<String> pool;

  /// How many Tallies an Occurrence picks.
  final int count;

  /// The count of a weekday (1 for Monday to 7 for Sunday) that has its
  /// own.
  final Map<int, int> counts;
  final SessionRule rule;

  const Session({
    required this.routine,
    required this.pool,
    required this.count,
    required this.counts,
    required this.rule,
  });

  String get id => routine.id;

  /// How many Tallies an Occurrence on [day] picks.
  int countOn(Day day) => counts[day.weekday] ?? count;
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
  /// The current pick of each Occurrence.
  final Map<String, List<String>> picks;

  /// Every Tally a pick of each Occurrence ever held.
  final Map<String, Set<String>> offered;

  /// The entries recorded for each Occurrence: the Tally's Note by entry
  /// id.
  final Map<String, Map<String, String>> entries;

  const SessionMarks._(this.picks, this.offered, this.entries);

  /// Reads the marks of the Routine [id] from [history], its own by
  /// default.
  factory SessionMarks.read(Store store, String id, [List<Entry>? history]) {
    final area = store.areaOf(id);
    final shared = area != null && store.isShared(area);
    final own = <String, bool>{};
    final latest = <String, Entry>{};
    final offered = <String, Set<String>>{};
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
        offered.putIfAbsent(key, () => {}).addAll(_noteIds(e.value));
      }
    }
    final picks = <String, List<String>>{};
    final entries = <String, Map<String, String>>{};
    for (final MapEntry(key: field, value: e) in latest.entries) {
      if (field.startsWith(SessionKeys.pickPrefix)) {
        final key = _occurrenceKey(
          field.substring(SessionKeys.pickPrefix.length),
        );
        final ids = _noteIds(e.value);
        if (key != null && ids.isNotEmpty) picks[key] = ids;
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
    return SessionMarks._(picks, offered, entries);
  }

  /// The entries recorded for [noteId] in the Occurrence [key], not
  /// removed, oldest first.
  List<TallyEntry> entriesFor(Store store, String key, String noteId) {
    final tallies = Tallies(store);
    final list = [
      for (final MapEntry(key: id, value: note)
          in (entries[key] ?? const <String, String>{}).entries)
        if (note == noteId)
          if (tallies.entryById(id) case final entry? when !entry.isTrashed)
            entry,
    ];
    list.sort((a, b) {
      final byAt = a.at.compareTo(b.at);
      return byAt != 0 ? byAt : a.id.compareTo(b.id);
    });
    return list;
  }

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

/// Spawning and changing Sessions, and picking their Tallies.
class Sessions {
  final Store store;

  Sessions(this.store);

  /// Spawns a Session from the Note [noteId], a Routine on [schedule]
  /// that picks [count] Tallies of [pool] (their Notes, in the order
  /// Rolling goes through them) by [rule] for each Occurrence. The pool
  /// takes [choices] only.
  Session spawn(
    String noteId,
    Schedule schedule, {
    required List<String> pool,
    required int count,
    required SessionRule rule,
  }) {
    final chosen = _chosen(store.areaOf(noteId), pool);
    _checkCount(count);
    final routine = Routines(store).spawn(
      noteId,
      schedule,
      within: (id) {
        store.append(id, SessionKeys.pool, chosen.join(','));
        store.append(id, SessionKeys.count, '$count');
        store.append(id, SessionKeys.rule, rule.name);
      },
    );
    return byId(routine.id)!;
  }

  /// Creates a Session without a Note at hand: its Note, holding [text],
  /// is captured behind the scenes (ADR 0005).
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

  /// Gives the Session [id] the pool [pool]: [choices], and those it
  /// holds already.
  void setPool(String id, List<String> pool) {
    final session = _requireManaged(id);
    final chosen = _chosen(store.areaOf(id), pool, held: session.pool);
    _write(id, SessionKeys.pool, chosen.join(','));
  }

  /// Sets how many Tallies an Occurrence of the Session [id] picks.
  void setCount(String id, int count) {
    _requireManaged(id);
    _checkCount(count);
    _write(id, SessionKeys.count, '$count');
  }

  /// Sets how many an Occurrence on [weekday] (1 for Monday to 7 for
  /// Sunday) picks; null takes the weekday's own count away.
  void setCountOn(String id, int weekday, int? count) {
    _requireManaged(id);
    if (weekday < 1 || weekday > 7) {
      throw ArgumentError.value(weekday, 'weekday', '1 to 7');
    }
    if (count != null) _checkCount(count);
    _write(id, SessionKeys.countOn(weekday), count?.toString());
  }

  /// Sets how the Session [id] picks.
  void setRule(String id, SessionRule rule) {
    _requireManaged(id);
    _write(id, SessionKeys.rule, rule.name);
  }

  /// The Session [id], trashed or not, or null when it is none.
  Session? byId(String id) {
    final routine = Routines(store).byId(id);
    if (routine == null) return null;
    final fields = store.currentFields(id);
    final pool = fields[SessionKeys.pool];
    if (pool == null) return null;
    return Session(
      routine: routine,
      pool: SessionMarks._noteIds(pool),
      count: _countOf(fields[SessionKeys.count]) ?? 1,
      counts: {
        for (var weekday = 1; weekday <= 7; weekday++)
          weekday: ?_countOf(fields[SessionKeys.countOn(weekday)]),
      },
      rule:
          SessionRule.values.asNameMap()[fields[SessionKeys.rule]] ??
          SessionRule.rolling,
    );
  }

  /// Whether the Routine [id] is a Session.
  bool isSession(String id) => byId(id) != null;

  /// Every Session not in the trash, by id.
  List<Session> all() {
    final sessions = [
      for (final routine in Routines(store).all()) ?byId(routine.id),
    ];
    sessions.sort((a, b) => a.id.compareTo(b.id));
    return sessions;
  }

  /// The Tallies a Session of [entity] may pick, as their Notes, newest
  /// capture first: [entity] is the Note to spawn it from, or the
  /// Session. Every Tally in the lists, but those outside its Area while
  /// that is a Shared Area, which not everyone in it holds.
  List<String> choices(String entity) {
    final area = store.areaOf(entity);
    return [
      for (final tally in Tallies(store).all())
        if (_fits(area, tally.noteId)) tally.noteId,
    ];
  }

  /// The Tallies [session] picks from, as their Notes, in its order:
  /// those of its pool that are Tallies in the lists, a merged one as
  /// the Note it went into, each once, and in a Shared Area only those
  /// of that Area.
  List<String> pool(Session session) {
    final area = store.areaOf(session.id);
    final notes = Notes(store);
    final tallies = Tallies(store);
    final out = <String>[];
    for (final id in session.pool) {
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
    final marks = SessionMarks.read(store, id);
    final pool = this.pool(session);
    final picks = _wanted(session, occurrence, pool, marks);
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
  /// happens when the pool holds no other.
  void another(String id, Occurrence occurrence, String noteId) {
    final session = _requireDue(id, occurrence);
    _requireCheckOff(id);
    final marks = SessionMarks.read(store, id);
    final pool = this.pool(session);
    final wanted = _wanted(session, occurrence, pool, marks);
    if (!wanted.contains(noteId)) {
      throw ArgumentError.value(noteId, 'noteId', 'not picked then');
    }
    final offered = {...?marks.offered[occurrence.key], ...wanted};
    final order = _order(session, occurrence, pool, marks);
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
    final marks = SessionMarks.read(store, id);
    final pool = this.pool(session);
    final wanted = _wanted(session, occurrence, pool, marks);
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
    final shown = <SessionOccurrence>[];
    for (final session in all()) {
      final pool = this.pool(session);
      final may = write && store.mayCheckOff(session.id);
      for (final occurrence in session.routine.schedule.on(day)) {
        var marks = SessionMarks.read(store, session.id);
        final picks = _wanted(session, occurrence, pool, marks);
        if (may && _writePick(session.id, occurrence, picks, marks)) {
          marks = SessionMarks.read(store, session.id);
        }
        shown.add(_shown(session, occurrence, picks, pool, marks));
      }
    }
    shown.sort((a, b) {
      final byTime = a.occurrence.compareTo(b.occurrence);
      return byTime != 0 ? byTime : a.session.id.compareTo(b.session.id);
    });
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
  ) {
    final count = session.countOn(occurrence.day);
    final order = _order(session, occurrence, pool, marks);
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

  /// [pool] in the order [session]'s rule picks for [occurrence].
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
    final before = [
      for (final key in marks.picks.keys)
        if (Occurrence.tryParse(key) case final o?
            when o.compareTo(occurrence) < 0)
          o,
    ]..sort();
    // When each was picked last: later Occurrences count higher, and
    // within one, those further on in its pick.
    final last = <String, int>{};
    var picked = 0;
    for (final o in before) {
      for (final id in marks.picks[o.key] ?? const <String>[]) {
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

  /// Writes [ids] as the pick of [occurrence], unless that is the pick
  /// already or there is nothing to pick; whether it wrote.
  bool _writePick(
    String id,
    Occurrence occurrence,
    List<String> ids,
    SessionMarks marks,
  ) {
    if (ids.isEmpty) return false;
    final held = marks.picks[occurrence.key];
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

  /// The Session [id], not in the trash.
  Session _require(String id) {
    final session = byId(id);
    if (session == null || session.routine.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not a Session');
    }
    return session;
  }

  /// The Session [id], which this Device may change.
  Session _requireManaged(String id) {
    final session = _require(id);
    if (!store.mayChangeRoutine(id)) {
      throw StateError('not a Manager of the Area of $id');
    }
    return session;
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
