import 'dart:io' show sleep;
import 'dart:math' as math;

import 'notes.dart';
import 'places.dart';
import 'routines.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'sync.dart';

/// What "Empty the trash now" removes, or removed (obf-1095): the Notes
/// and the Routines and Reminders by id, in the order they are purged,
/// and the files of those Notes whose bytes left this Device.
final class TrashEmptied {
  const TrashEmptied({
    this.notes = const [],
    this.routines = const [],
    this.blobs = const [],
  });

  /// The Notes, merged ones not counted: they go with their target.
  final List<String> notes;

  /// The Routines and Reminders, those trashed with their Note included.
  final List<String> routines;

  /// The photos, recordings and attached files whose bytes were let go,
  /// as `photo/<hash>`, `audio/<hash>` and `file/<hash>`, sorted; none
  /// for [Trash.toEmpty].
  final List<String> blobs;

  /// Whether nothing is, or was, removed.
  bool get isEmpty => notes.isEmpty && routines.isEmpty;
}

/// The trash emptied at once (obf-1095).
final class Trash {
  Trash(this.store);

  final Store store;

  /// What [empty] removes now, for the question asked before it: every
  /// Note and every Routine or Reminder in the trash that this Device may
  /// change, as [Notes.purgeTrash] and [Routines.purgeTrash] remove them
  /// after 30 days, however long they have been there. Those it may not
  /// change in a Shared Area stay, and are not counted.
  TrashEmptied toEmpty() => TrashEmptied(
    notes: Notes(store).purgeable(),
    routines: Routines(store).purgeable(),
  );

  /// How many Notes, or Routines and Reminders, [empty] purges in one
  /// transaction: another connection to the database, such as the app's
  /// while it runs beside it, waits at most for so many.
  static const perBatch = 25;

  /// "Empty the trash now": purges what [toEmpty] names, as the 30-day
  /// purge does, [perBatch] at a time, each batch a transaction of its
  /// own, resting as long as it took before the next, as the desktop's
  /// Import does beside its window; then lets go of the bytes of the photos,
  /// recordings and files of those Notes, their Sections and the Notes
  /// merged into them, on this Device, unless a Note not purged still
  /// shows them. One brought back from the trash or purged meanwhile is
  /// left as it is. The log keeps every entry, so sync is unchanged; a
  /// file only a purged Note holds is no longer wanted from another
  /// Device. It cannot be undone. Frogs stay, as the 30-day purge leaves
  /// them (ADR 0027).
  ///
  /// [progress] hears how many of the Notes it has gone through, before
  /// the first batch and after each; [stop] is asked before each batch.
  /// Once it says so, or a batch fails, what was purged stays purged and
  /// its bytes go all the same.
  TrashEmptied empty({MoveProgress? progress, Stopping? stop}) {
    final refs = <String>{};
    final gone = <String>[];
    final goneRoutines = <String>[];
    try {
      _purge(refs, gone, goneRoutines, progress, stop);
    } catch (_) {
      // What went wrong is told; letting go of the bytes of what went
      // before is tried all the same.
      try {
        _drop(refs);
      } on Object catch (_) {
        // The purge's error is the one told.
      }
      rethrow;
    }
    final dropped = _drop(refs);
    return TrashEmptied(notes: gone, routines: goneRoutines, blobs: dropped);
  }

  /// The purge behind [empty]: adds the Notes and the Routines and
  /// Reminders it purged to [gone] and [goneRoutines], and the files
  /// they held to [refs], each once its batch is in.
  void _purge(
    Set<String> refs,
    List<String> gone,
    List<String> goneRoutines,
    MoveProgress? progress,
    Stopping? stop,
  ) {
    final notes = Notes(store);
    final routines = Routines(store);
    final watch = Stopwatch();
    // Rests as long as the batch before took, so the app's reads and
    // writes, which wait for its transaction, get the database between
    // two batches.
    bool next() {
      if (watch.isRunning) sleep(watch.elapsed);
      if (stop?.call() ?? false) return false;
      watch
        ..reset()
        ..start();
      return true;
    }

    final selected = notes.purgeable();
    // Read for all of them at once, before anything is written: one
    // pass, not one per Note.
    final sections = notes.sectionsOf(selected);
    progress?.call(0, selected.length);
    for (var at = 0; at < selected.length; at += perBatch) {
      if (!next()) return;
      final ids = selected.sublist(
        at,
        math.min(at + perBatch, selected.length),
      );
      final (purgedNotes, held) = store.batch(() {
        final purgedNotes = [
          for (final id in ids)
            if (_stillPurgeable(
              id,
              (NoteKeys.type, NoteKeys.typeNote),
              NoteKeys.trashed,
              store.mayChange,
            ))
              id,
        ];
        final purged = [
          for (final id in purgedNotes)
            ...notes.purge(id, sections: sections[id]),
        ];
        return (purgedNotes, _refsOf(purged));
      });
      gone.addAll(purgedNotes);
      refs.addAll(held);
      progress?.call(at + ids.length, selected.length);
    }
    final selectedRoutines = routines.purgeable();
    for (var at = 0; at < selectedRoutines.length; at += perBatch) {
      if (!next()) return;
      final ids = selectedRoutines.sublist(
        at,
        math.min(at + perBatch, selectedRoutines.length),
      );
      final purged = store.batch(() {
        final purged = [
          for (final id in ids)
            if (_stillPurgeable(
              id,
              (RoutineKeys.type, RoutineKeys.typeRoutine),
              RoutineKeys.trashed,
              store.mayChangeRoutine,
            ))
              id,
        ];
        for (final id in purged) {
          store.append(id, RoutineKeys.type, null);
        }
        return purged;
      });
      goneRoutines.addAll(purged);
    }
  }

  /// Whether [id], picked before an earlier batch, still has [type] and
  /// is in the trash under [trashed], also in the log, and this Device
  /// may still change it, as [mayChange] tells: brought back or purged
  /// meanwhile, it stays as it is.
  bool _stillPurgeable(
    String id,
    (String, String) type,
    String trashed,
    bool Function(String id) mayChange,
  ) {
    final fields = store.currentFields(id);
    return fields[type.$1] == type.$2 &&
        fields[trashed] != null &&
        store.logged(id, trashed) != null &&
        mayChange(id);
  }

  /// The photos, recordings and files the [purged] entities name, as
  /// `<kind>/<hash>`.
  Set<String> _refsOf(List<String> purged) => {
    for (final fields in store.currentFieldsOf(purged).values)
      for (final field in fields.keys)
        for (final (kind, prefix) in Sync.blobKinds)
          if (field.startsWith(prefix))
            '$kind/${field.substring(prefix.length)}',
  };

  /// Lets go of the bytes of [refs] on this Device, those a Note not
  /// purged still shows kept; returns those let go, sorted. Once the
  /// purge is in: the bytes are not part of a transaction.
  List<String> _drop(Set<String> refs) {
    if (refs.isEmpty) return [];
    final known = store.knownBlobs();
    final dropped = <String>[];
    for (final ref in refs.toList()..sort()) {
      if (known.contains(ref)) continue;
      final (kind, hash) = splitBlobRef(ref);
      if (store.dropBlob(kind, hash)) dropped.add(ref);
    }
    return dropped;
  }
}
