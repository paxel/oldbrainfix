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

  /// The Note's [Importance] by name; absent means normal.
  static const importance = 'importance';

  /// When the Note was moved to the trash; absent while it is not there.
  static const trashed = 'trashed';
}

/// Field keys of a Section.
abstract final class SectionKeys {
  static const type = r'$type';
  static const typeSection = 'section';

  /// The Note the Section belongs to.
  static const note = 'note';
  static const text = 'text';
  static const captured = 'captured';
}

/// How much a Note matters (CONTEXT.md).
enum Importance { unimportant, normal, important }

/// What was true when a Note or Section was captured (CONTEXT.md). For
/// now that is the time; place, source app, calendar event and
/// Bluetooth device join later.
class CaptureContext {
  final DateTime captured;

  const CaptureContext({required this.captured});
}

/// A Note as the screens want it (CONTEXT.md).
class Note {
  /// Entity id, `note:<id>`.
  final String id;
  final String text;
  final CaptureContext context;

  /// The Area or Topic the Note is filed in, or null when it has none.
  final String? place;
  final Importance importance;

  /// When the Note was moved to the trash, or null when it is not there.
  final DateTime? trashed;

  const Note({
    required this.id,
    required this.text,
    required this.context,
    this.place,
    this.importance = Importance.normal,
    this.trashed,
  });

  /// When the Note was captured.
  DateTime get captured => context.captured;

  bool get isTrashed => trashed != null;

  /// Not yet filed into an Area or Topic, and not deleted (CONTEXT.md).
  bool get isUnsorted => place == null && !isTrashed;
}

/// A dated part of a Note, added by Amend (CONTEXT.md).
class Section {
  /// Entity id, `section:<id>`.
  final String id;
  final String noteId;
  final String text;
  final CaptureContext context;

  const Section({
    required this.id,
    required this.noteId,
    required this.text,
    required this.context,
  });
}

/// How a diary of cards is ordered.
enum DiaryOrder {
  /// Newest capture first.
  time,

  /// Important first, unimportant last; newest capture first within each.
  importance,
}

/// [notes] as a diary in [order], without the unimportant ones when
/// [hideUnimportant] is set.
List<Note> diary(
  Iterable<Note> notes, {
  DiaryOrder order = DiaryOrder.time,
  bool hideUnimportant = false,
}) {
  final shown = [
    for (final note in notes)
      if (!hideUnimportant || note.importance != Importance.unimportant) note,
  ];
  shown.sort((a, b) {
    if (order == DiaryOrder.importance) {
      final byImportance = b.importance.index.compareTo(a.importance.index);
      if (byImportance != 0) return byImportance;
    }
    return b.captured.compareTo(a.captured);
  });
  return shown;
}

/// Capturing, changing and reading Notes.
class Notes {
  /// How long a deleted Note stays in the trash before it is purged.
  static const trashKeep = Duration(days: 30);

  final Store store;

  Notes(this.store);

  /// Captures a new text Note at the store's now, filed into the Area or
  /// Topic [place] when one is given.
  Note capture(String text, {String? place}) {
    if (place != null && !Places(store).isPlace(place)) {
      throw ArgumentError.value(place, 'place', 'not an Area or Topic');
    }
    final id = 'note:${Store.newId()}';
    final now = store.clock().toUtc();
    store.batch(() {
      store.append(id, NoteKeys.type, NoteKeys.typeNote, date: now);
      store.append(id, NoteKeys.captured, now.toIso8601String(), date: now);
      store.append(id, NoteKeys.text, text, date: now);
      if (place != null) store.append(id, NoteKeys.place, place, date: now);
    });
    return byId(id)!;
  }

  /// Files the Note [noteId] into the Area or Topic [placeId], out of
  /// wherever it was before.
  void file(String noteId, String placeId) {
    _requireNote(noteId);
    if (!Places(store).isPlace(placeId)) {
      throw ArgumentError.value(placeId, 'placeId', 'not an Area or Topic');
    }
    store.append(noteId, NoteKeys.place, placeId);
  }

  /// Fixes the text of the Note [noteId] in place. The old text stays in
  /// the log only.
  void edit(String noteId, String text) {
    _requireNote(noteId);
    store.append(noteId, NoteKeys.text, _text(text));
  }

  /// Amends the Note [noteId] with a new Section holding [text], captured
  /// at the store's now.
  Section amend(String noteId, String text) {
    _requireNote(noteId);
    final trimmed = _text(text);
    final id = 'section:${Store.newId()}';
    final now = store.clock().toUtc();
    store.batch(() {
      store.append(id, SectionKeys.type, SectionKeys.typeSection, date: now);
      store.append(id, SectionKeys.note, noteId, date: now);
      store.append(id, SectionKeys.captured, now.toIso8601String(), date: now);
      store.append(id, SectionKeys.text, trimmed, date: now);
    });
    return _section(id)!;
  }

  /// The Sections of the Note [noteId], oldest first.
  List<Section> sections(String noteId) {
    final sections = [
      for (final id in store.entitiesOfType(SectionKeys.typeSection))
        if (store.current(id, SectionKeys.note) == noteId) ?_section(id),
    ];
    sections.sort((a, b) => a.context.captured.compareTo(b.context.captured));
    return sections;
  }

  /// Sets the Importance of the Note [noteId].
  void setImportance(String noteId, Importance importance) {
    _requireNote(noteId);
    store.append(noteId, NoteKeys.importance, importance.name);
  }

  /// Moves the Note [noteId] to the trash: it leaves every list and
  /// Unsorted, and can be restored until [trashKeep] has passed.
  void trash(String noteId) {
    _requireNote(noteId);
    store.append(
      noteId,
      NoteKeys.trashed,
      store.clock().toUtc().toIso8601String(),
    );
  }

  /// Brings the Note [noteId] back from the trash, to where it was filed.
  void restore(String noteId) {
    _requireNote(noteId);
    store.append(noteId, NoteKeys.trashed, null);
  }

  /// When the trashed [note] is purged; null when it is not in the trash.
  DateTime? purgeAt(Note note) => note.trashed?.add(trashKeep);

  /// The Notes in the trash that can still be restored, most recently
  /// deleted first.
  List<Note> trashed() {
    final now = store.clock().toUtc();
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note?)
          if (purgeAt(note) case final at? when now.isBefore(at)) note,
    ];
    notes.sort(
      (a, b) => (b.trashed ?? b.captured).compareTo(a.trashed ?? a.captured),
    );
    return notes;
  }

  /// Purges every Note that has been in the trash for [trashKeep], with
  /// its Sections. The log keeps their entries; nothing shows them again.
  void purgeTrash() {
    final now = store.clock().toUtc();
    store.batch(() {
      for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
        final note = byId(id);
        final at = note == null ? null : purgeAt(note);
        if (at == null || now.isBefore(at)) continue;
        for (final section in sections(id)) {
          store.append(section.id, SectionKeys.type, null);
        }
        store.append(id, NoteKeys.type, null);
      }
    });
  }

  /// The Note [id], trashed or not, or null when there is none.
  Note? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final captured = fields[NoteKeys.captured];
    if (captured == null) return null;
    // A place that is gone leaves the Note Unsorted.
    final place = fields[NoteKeys.place];
    final trashed = fields[NoteKeys.trashed];
    return Note(
      id: id,
      text: fields[NoteKeys.text] ?? '',
      context: CaptureContext(captured: DateTime.parse(captured)),
      place: place != null && Places(store).isPlace(place) ? place : null,
      importance:
          Importance.values.asNameMap()[fields[NoteKeys.importance]] ??
          Importance.normal,
      trashed: trashed == null ? null : DateTime.parse(trashed),
    );
  }

  /// Every Note not in the trash, newest capture first.
  List<Note> all() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed) note,
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return notes;
  }

  /// The Notes filed in the Area or Topic [placeId], newest capture
  /// first; an Area holds its Topics' Notes too.
  List<Note> inside(String placeId) {
    final ids = {
      placeId,
      for (final topic in Places(store).topics(placeId)) topic.id,
    };
    return [
      for (final note in all())
        if (ids.contains(note.place)) note,
    ];
  }

  Section? _section(String id) {
    final fields = store.currentFields(id);
    if (fields[SectionKeys.type] != SectionKeys.typeSection) return null;
    final noteId = fields[SectionKeys.note];
    final captured = fields[SectionKeys.captured];
    if (noteId == null || captured == null) return null;
    return Section(
      id: id,
      noteId: noteId,
      text: fields[SectionKeys.text] ?? '',
      context: CaptureContext(captured: DateTime.parse(captured)),
    );
  }

  void _requireNote(String id) {
    if (byId(id) == null) throw ArgumentError.value(id, 'noteId', 'not a Note');
  }

  static String _text(String text) {
    final trimmed = text.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(text, 'text', 'must not be empty');
    }
    return trimmed;
  }
}
