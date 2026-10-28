import 'notes.dart';
import 'profiles.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'store.dart';

/// Field keys of a Frog (ADR 0027).
abstract final class FrogKeys {
  static const type = r'$type';
  static const typeFrog = 'frog';

  /// The entity id's prefix.
  static const prefix = 'frog:';

  /// The Note the Frog was spawned from (ADR 0005).
  static const note = 'note';

  /// When the Frog was checked off; null when it was unchecked again.
  static const done = 'done';

  /// When the Frog was moved to the trash; absent while it is not there.
  static const trashed = 'trashed';

  /// The entity holding today's pick, outside every Area: sealed under
  /// the personal key, it goes to the user's own Devices only, and only
  /// what they wrote counts.
  static const pickEntity = 'frogs:pick';

  /// Today's pick: the day, then the ids of the Frogs it offers, joined
  /// by commas, e.g. `2026-10-28,frog:a,frog:b`.
  static const pick = 'pick';

  /// How many Frogs the Agenda offers a day.
  static const perDay = 2;
}

/// A Note's job with no time, done once, offered on the Agenda to eat one
/// a day (CONTEXT.md). It never rings.
class Frog {
  /// Entity id, `frog:<id>`.
  final String id;

  /// The Note it was spawned from, as stored; see [Frogs.note] for the
  /// Note it shows under after a Merge.
  final String noteId;

  /// When it was spawned.
  final DateTime spawned;

  /// When it was checked off, or null while it is open.
  final DateTime? done;

  /// When it was moved to the trash, or null when it is not there.
  final DateTime? trashed;

  const Frog({
    required this.id,
    required this.noteId,
    required this.spawned,
    this.done,
    this.trashed,
  });

  bool get isDone => done != null;

  bool get isTrashed => trashed != null;
}

/// The Agenda's "Eat a frog" section today.
class FrogsToday {
  final Day today;

  /// The Frogs the section shows, in order: those of today's pick
  /// checked off today when there are any, else the open ones of the
  /// pick, at most [FrogKeys.perDay].
  final List<Frog> shown;

  /// Whether a Frog of today's pick was eaten today: [shown] holds only
  /// those then, and the others stay hidden.
  final bool eaten;

  /// Whether "Another one" has an open Frog to swap in.
  final bool canSwap;

  /// Whether the section shows at all: while any Frog is open, or one
  /// eaten today is shown.
  final bool visible;

  const FrogsToday({
    required this.today,
    required this.shown,
    required this.eaten,
    required this.canSwap,
    required this.visible,
  });
}

/// Spawning, checking off and offering Frogs.
class Frogs {
  final Store store;

  Frogs(this.store);

  /// Spawns a Frog from the Note [noteId], which leaves Unsorted. A Note
  /// has one open Frog at a time: while it has one, that one is returned
  /// and nothing is written.
  Frog spawn(String noteId) {
    final note = Notes(store).byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
    }
    if (!store.mayManage(store.areaOf(noteId))) {
      throw StateError('not a Manager of the Area of $noteId');
    }
    for (final frog in ofNote(noteId)) {
      if (!frog.isDone) return frog;
    }
    final id = '${FrogKeys.prefix}${Store.newId()}';
    store.batch(() {
      store.append(id, FrogKeys.type, FrogKeys.typeFrog);
      store.append(id, FrogKeys.note, noteId);
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

  /// Checks off the Frog [id], at [at] or the store's now. Checking off
  /// one already Done writes nothing.
  void markDone(String id, {DateTime? at}) {
    final frog = _require(id);
    if (frog.isDone) return;
    store.append(
      id,
      FrogKeys.done,
      (at ?? store.clock()).toUtc().toIso8601String(),
    );
  }

  /// Unchecks the Frog [id] again; one still open writes nothing.
  void unmarkDone(String id) {
    final frog = _require(id);
    if (!frog.isDone) return;
    store.append(id, FrogKeys.done, null);
  }

  /// Moves the Frog [id] to the trash: it leaves every list. Its Note
  /// stays, and stays out of Unsorted.
  void trash(String id) {
    _require(id);
    store.append(id, FrogKeys.trashed, store.clock().toUtc().toIso8601String());
  }

  /// Brings the Frog [id] back from the trash.
  void restore(String id) {
    final frog = byId(id);
    if (frog == null || !frog.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not a Frog in the trash');
    }
    store.append(id, FrogKeys.trashed, null);
  }

  /// The Frog [id], trashed or not, or null when there is none.
  Frog? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[FrogKeys.type] != FrogKeys.typeFrog) return null;
    final noteId = fields[FrogKeys.note];
    if (noteId == null) return null;
    DateTime? spawned;
    for (final e in store.history(id)) {
      if (e.field != FrogKeys.type || e.value != FrogKeys.typeFrog) continue;
      if (spawned == null || e.date.isBefore(spawned)) spawned = e.date;
    }
    return Frog(
      id: id,
      noteId: noteId,
      spawned: spawned ?? _never,
      done: DateTime.tryParse(fields[FrogKeys.done] ?? ''),
      trashed: DateTime.tryParse(fields[FrogKeys.trashed] ?? ''),
    );
  }

  /// The Note [frog] links back to: the Note it was spawned from, or the
  /// one that Note was merged into; it may be in the trash. Null when it
  /// is gone for good.
  Note? note(Frog frog) {
    final notes = Notes(store);
    final note = notes.byId(frog.noteId);
    final into = note?.mergedInto;
    return into == null ? note : notes.byId(into);
  }

  /// The open Frogs, oldest first: not checked off, not in the trash,
  /// their Note neither.
  List<Frog> open() => [
    for (final frog in _listed())
      if (!frog.isDone) frog,
  ];

  /// The Frogs checked off, most recently first, but those in the trash
  /// or whose Note is.
  List<Frog> done() {
    final done = [
      for (final frog in _listed())
        if (frog.isDone) frog,
    ];
    done.sort((a, b) {
      final byDone = (b.done ?? _never).compareTo(a.done ?? _never);
      return byDone != 0 ? byDone : a.id.compareTo(b.id);
    });
    return done;
  }

  /// The Frogs the Note [noteId] spawned, those of the Notes merged into
  /// it included, oldest first, without those in the trash.
  List<Frog> ofNote(String noteId) => [
    for (final frog in _all())
      if (!frog.isTrashed && note(frog)?.id == noteId) frog,
  ];

  /// The open Frogs in the order the Agenda offers them: those offered
  /// the longest ago first, those never offered before them, oldest
  /// first among equals.
  List<Frog> rotation() {
    final last = <String, int>{};
    final picks = _picks();
    for (var i = 0; i < picks.length; i++) {
      for (final id in picks[i].ids) {
        last[id] = i;
      }
    }
    final open = this.open();
    final order = {for (var i = 0; i < open.length; i++) open[i].id: i};
    return open..sort((a, b) {
      final byShown = (last[a.id] ?? -1).compareTo(last[b.id] ?? -1);
      return byShown != 0
          ? byShown
          : (order[a.id] ?? 0).compareTo(order[b.id] ?? 0);
    });
  }

  /// Today's "Eat a frog" section, on [wallClock], as [pickToday] would
  /// write it: written nowhere.
  FrogsToday today(WallClock wallClock) {
    final day = Day.today(store.clock(), wallClock);
    return _today(day, wallClock, _wanted(day, wallClock));
  }

  /// Today's "Eat a frog" section, on [wallClock], its pick written for
  /// the user's own Devices when it is new: the first of the day, or one
  /// that replaces Frogs gone since. The Agenda calls it as it shows;
  /// called again, it writes nothing.
  FrogsToday pickToday(WallClock wallClock) {
    final day = Day.today(store.clock(), wallClock);
    final wanted = _wanted(day, wallClock);
    _writePick(day, wanted);
    return _today(day, wallClock, wanted);
  }

  /// "Another one": swaps the Frog [id] offered today for the next one in
  /// the [rotation], in its place. Nothing happens when there is none
  /// left to offer.
  void another(WallClock wallClock, String id) {
    final day = Day.today(store.clock(), wallClock);
    final wanted = _wanted(day, wallClock);
    final shown = _today(day, wallClock, wanted);
    if (shown.eaten || !shown.shown.any((f) => f.id == id)) {
      throw ArgumentError.value(id, 'id', 'not a Frog offered today');
    }
    store.batch(() {
      // The pick it swaps from counts as offered.
      _writePick(day, wanted);
      for (final frog in rotation()) {
        if (wanted.contains(frog.id)) continue;
        _writePick(day, [
          for (final picked in wanted) picked == id ? frog.id : picked,
        ]);
        return;
      }
    });
  }

  /// The section for [day] with the Frogs [picked].
  FrogsToday _today(Day day, WallClock wallClock, List<String> picked) {
    final frogs = [for (final id in picked) ?_shownFrog(id)];
    final eaten = [
      for (final frog in frogs)
        if (_doneOn(frog, day, wallClock)) frog,
    ];
    final open = this.open();
    return FrogsToday(
      today: day,
      shown: eaten.isNotEmpty ? eaten : [...frogs.where((f) => !f.isDone)],
      eaten: eaten.isNotEmpty,
      canSwap: eaten.isEmpty && open.any((f) => !picked.contains(f.id)),
      visible: open.isNotEmpty || eaten.isNotEmpty,
    );
  }

  /// The pick [day] should have: today's as it is while one of it was
  /// eaten today or it is whole; else with what is gone replaced in
  /// place from the [rotation] and topped up to [FrogKeys.perDay].
  List<String> _wanted(Day day, WallClock wallClock) {
    final held = _currentPick();
    final picked = held != null && held.day == day
        ? held.ids.take(FrogKeys.perDay).toList()
        : <String>[];
    final frogs = [for (final id in picked) _shownFrog(id)];
    if (frogs.any((f) => f != null && _doneOn(f, day, wallClock))) {
      return picked;
    }
    final next = [
      for (final frog in rotation())
        if (!picked.contains(frog.id)) frog.id,
    ].iterator;
    final wanted = <String>[];
    for (var i = 0; i < picked.length; i++) {
      final frog = frogs[i];
      if (frog != null && !frog.isDone) {
        wanted.add(picked[i]);
      } else if (next.moveNext()) {
        wanted.add(next.current);
      }
    }
    while (wanted.length < FrogKeys.perDay && next.moveNext()) {
      wanted.add(next.current);
    }
    return wanted;
  }

  /// Writes [ids] as the pick of [day], unless that is the pick already
  /// or there is nothing to pick.
  void _writePick(Day day, List<String> ids) {
    if (ids.isEmpty) return;
    final held = _currentPick();
    if (held != null &&
        held.day == day &&
        held.ids.length == ids.length &&
        Iterable<int>.generate(ids.length)
            .every((i) => held.ids[i] == ids[i])) {
      return;
    }
    store.append(FrogKeys.pickEntity, FrogKeys.pick, '$day,${ids.join(',')}');
  }

  /// Every pick one of the user's own Devices wrote, in latest-wins
  /// order; another person's Device has no say in them.
  List<_Pick> _picks() {
    final own = <String, bool>{};
    final entries = [
      for (final e in store.loggedHistory(FrogKeys.pickEntity))
        if (e.field == FrogKeys.pick &&
            own.putIfAbsent(e.device, () => store.writtenByOwn(e.device)))
          e,
    ]..sort(Store.latestWins);
    return [for (final e in entries) ?_Pick.parse(e.value)];
  }

  /// The newest pick of the user's own Devices, or null when there is
  /// none it can read.
  _Pick? _currentPick() {
    final picks = _picks();
    return picks.isEmpty ? null : picks.last;
  }

  /// Whether [frog] was checked off on [day] as [wallClock] shows it.
  static bool _doneOn(Frog frog, Day day, WallClock wallClock) {
    final done = frog.done;
    return done != null && Day.today(done, wallClock) == day;
  }

  /// The Frog [id] when it shows in the lists, Done or not.
  Frog? _shownFrog(String id) {
    final frog = byId(id);
    if (frog == null || frog.isTrashed) return null;
    final note = this.note(frog);
    return note == null || note.isTrashed ? null : frog;
  }

  /// Every Frog that shows in the lists, Done or not, oldest first.
  List<Frog> _listed() => [
    for (final frog in _all())
      if (_shownFrog(frog.id) != null) frog,
  ];

  /// Every Frog, trashed or not, oldest first.
  List<Frog> _all() {
    final frogs = [
      for (final id in store.entitiesOfType(FrogKeys.typeFrog)) ?byId(id),
    ];
    frogs.sort((a, b) {
      final bySpawned = a.spawned.compareTo(b.spawned);
      return bySpawned != 0 ? bySpawned : a.id.compareTo(b.id);
    });
    return frogs;
  }

  /// The Frog [id], which must not be in the trash.
  Frog _require(String id) {
    final frog = byId(id);
    if (frog == null || frog.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not a Frog');
    }
    return frog;
  }

  /// Stands in for a moment never written.
  static final _never = DateTime.utc(0);
}

/// A pick as written: its day and the Frogs it offers.
class _Pick {
  final Day day;
  final List<String> ids;

  const _Pick(this.day, this.ids);

  /// The pick [value] holds; null for anything else.
  static _Pick? parse(String? value) {
    if (value == null) return null;
    final parts = value.split(',');
    final day = Day.tryParse(parts.first);
    if (day == null) return null;
    final ids = parts.skip(1).toList();
    if (ids.any((id) => !id.startsWith(FrogKeys.prefix))) return null;
    return _Pick(day, ids);
  }
}
