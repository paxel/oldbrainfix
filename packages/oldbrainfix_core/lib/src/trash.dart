import 'notes.dart';
import 'routines.dart';
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

  /// "Empty the trash now": purges what [toEmpty] names in one batch, as
  /// the 30-day purge does, then lets go of the bytes of the photos,
  /// recordings and files of those Notes, their Sections and the Notes
  /// merged into them, on this Device, unless a Note not purged still
  /// shows them. The log keeps every entry, so sync is unchanged; a file
  /// only a purged Note holds is no longer wanted from another Device.
  /// It cannot be undone. Frogs stay, as the 30-day purge leaves them
  /// (ADR 0027).
  TrashEmptied empty() {
    final notes = Notes(store);
    final routines = Routines(store);
    final refs = <String>{};
    final (gone, goneRoutines) = store.batch(() {
      final gone = notes.purgeable();
      // Read for all of them at once, before anything is written: one
      // pass, not one per Note.
      final sections = notes.sectionsOf(gone);
      final purged = [
        for (final id in gone) ...notes.purge(id, sections: sections[id]),
      ];
      for (final fields in store.currentFieldsOf(purged).values) {
        for (final field in fields.keys) {
          for (final (kind, prefix) in Sync.blobKinds) {
            if (field.startsWith(prefix)) {
              refs.add('$kind/${field.substring(prefix.length)}');
            }
          }
        }
      }
      final goneRoutines = routines.purgeable();
      for (final id in goneRoutines) {
        store.append(id, RoutineKeys.type, null);
      }
      return (gone, goneRoutines);
    });
    // Once the purge is in: the bytes are not part of the transaction.
    final known = store.knownBlobs();
    final dropped = <String>[];
    for (final ref in refs.toList()..sort()) {
      if (known.contains(ref)) continue;
      final (kind, hash) = splitBlobRef(ref);
      if (store.dropBlob(kind, hash)) dropped.add(ref);
    }
    return TrashEmptied(notes: gone, routines: goneRoutines, blobs: dropped);
  }
}
