import 'store.dart';

/// Field keys of a Note.
abstract final class NoteKeys {
  static const type = r'$type';
  static const typeNote = 'note';
  static const text = 'text';
  static const captured = 'captured';
}

/// A Note as the screens want it (CONTEXT.md).
class Note {
  /// Entity id, `note:<id>`.
  final String id;
  final String text;

  /// When the Note was captured.
  final DateTime captured;

  const Note({required this.id, required this.text, required this.captured});
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

  /// The Note [id], or null when there is none.
  Note? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final captured = fields[NoteKeys.captured];
    if (captured == null) return null;
    return Note(
      id: id,
      text: fields[NoteKeys.text] ?? '',
      captured: DateTime.parse(captured),
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
