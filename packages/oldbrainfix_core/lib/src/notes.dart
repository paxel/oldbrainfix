import 'places.dart';
import 'store.dart';

/// Field keys of a Note.
abstract final class NoteKeys {
  static const type = r'$type';
  static const typeNote = 'note';
  static const text = 'text';
  static const captured = 'captured';

  /// The Area or Topic the Note is filed in; absent while Unsorted.
  static const place = 'place';
}

/// A Note as the screens want it (CONTEXT.md).
class Note {
  /// Entity id, `note:<id>`.
  final String id;
  final String text;

  /// When the Note was captured.
  final DateTime captured;

  /// The Area or Topic the Note is filed in, or null when it has none.
  final String? place;

  const Note({
    required this.id,
    required this.text,
    required this.captured,
    this.place,
  });

  /// Not yet filed into an Area or Topic (CONTEXT.md).
  bool get isUnsorted => place == null;
}

/// Capturing and reading Notes.
class Notes {
  final Store store;

  Notes(this.store);

  /// Captures a new text Note at the store's now.
  Note capture(String text) {
    final id = 'note:${Store.newId()}';
    final now = store.clock().toUtc();
    store.batch(() {
      store.append(id, NoteKeys.type, NoteKeys.typeNote, date: now);
      store.append(id, NoteKeys.captured, now.toIso8601String(), date: now);
      store.append(id, NoteKeys.text, text, date: now);
    });
    return byId(id)!;
  }

  /// Files the Note [noteId] into the Area or Topic [placeId], out of
  /// wherever it was before.
  void file(String noteId, String placeId) {
    if (byId(noteId) == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note');
    }
    if (!Places(store).isPlace(placeId)) {
      throw ArgumentError.value(placeId, 'placeId', 'not an Area or Topic');
    }
    store.append(noteId, NoteKeys.place, placeId);
  }

  /// The Note [id], or null when there is none.
  Note? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final captured = fields[NoteKeys.captured];
    if (captured == null) return null;
    // A place that is gone leaves the Note Unsorted.
    final place = fields[NoteKeys.place];
    return Note(
      id: id,
      text: fields[NoteKeys.text] ?? '',
      captured: DateTime.parse(captured),
      place: place != null && Places(store).isPlace(place) ? place : null,
    );
  }

  /// Every Note, newest capture first.
  List<Note> all() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote)) ?byId(id),
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return notes;
  }
}
