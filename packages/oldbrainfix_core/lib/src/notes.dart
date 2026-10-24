import 'named_places.dart';
import 'places.dart';
import 'store.dart';

/// Field keys of a Note.
abstract final class NoteKeys {
  static const type = r'$type';
  static const typeNote = 'note';
  static const text = 'text';
  static const captured = 'captured';

  /// Where the Note was captured, as [Coordinates.encode] writes it;
  /// absent while unknown.
  static const location = 'location';

  /// The Area or Topic the Note is filed in; absent while Unsorted.
  static const place = 'place';

  /// The Note's [Importance] by name; absent means normal.
  static const importance = 'importance';

  /// When the Note was moved to the trash; absent while it is not there.
  static const trashed = 'trashed';

  /// The Note this one was merged into; absent while it stands alone.
  static const merged = 'merged';
}

/// Field keys of a Section.
abstract final class SectionKeys {
  static const type = r'$type';
  static const typeSection = 'section';

  /// The Note the Section belongs to.
  static const note = 'note';
  static const text = 'text';
  static const captured = 'captured';

  /// Where the Section was captured, as for [NoteKeys.location].
  static const location = 'location';
}

/// How much a Note matters (CONTEXT.md).
enum Importance { unimportant, normal, important }

/// What was true when a Note or Section was captured (CONTEXT.md). For
/// now that is the time and where it was; source app, calendar event and
/// Bluetooth device join later.
class CaptureContext {
  final DateTime captured;

  /// Where it was captured, or null when that is unknown: denied,
  /// unavailable, or not arrived yet.
  final Coordinates? location;

  /// The Named Place [location] lies in, resolved when read, so a
  /// renamed, deleted or new Named Place shows on Notes captured before.
  final NamedPlace? namedPlace;

  const CaptureContext({
    required this.captured,
    this.location,
    this.namedPlace,
  });
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

  /// The Note this one was merged into and now is a Section of, or null
  /// when it stands alone.
  final String? mergedInto;

  const Note({
    required this.id,
    required this.text,
    required this.context,
    this.place,
    this.importance = Importance.normal,
    this.trashed,
    this.mergedInto,
  });

  /// When the Note was captured.
  DateTime get captured => context.captured;

  bool get isTrashed => trashed != null;

  bool get isMerged => mergedInto != null;

  /// Not yet filed into an Area or Topic, not merged and not deleted
  /// (CONTEXT.md).
  bool get isUnsorted => place == null && !isTrashed && !isMerged;
}

/// A dated part of a Note, added by Amend or Merge (CONTEXT.md).
class Section {
  /// Entity id: `section:<id>` for an Amend, the merged Note's
  /// `note:<id>` for a Merge.
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

  /// When the text of the Note [noteId] was last set, by capture or edit;
  /// null when there is no such Note.
  DateTime? textSetAt(String noteId) {
    DateTime? latest;
    for (final e in store.history(noteId)) {
      if (e.field != NoteKeys.text) continue;
      if (latest == null || e.date.isAfter(latest)) latest = e.date;
    }
    return latest;
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

  /// Merges the Note [noteId] into the Note [intoId]: it becomes a dated
  /// Section of it, with its own Capture Context, and leaves every list
  /// and Unsorted.
  ///
  /// Nothing is copied: the merged Note keeps its entity and only points
  /// at its target, which shows it as a Section. Whatever belongs to it,
  /// its Sections and whatever reaches it from other Devices after the
  /// Merge included, travels along.
  void merge(String noteId, String intoId) {
    for (final (id, name) in [(noteId, 'noteId'), (intoId, 'intoId')]) {
      final note = byId(id);
      if (note == null || note.isTrashed || note.isMerged) {
        throw ArgumentError.value(id, name, 'not a Note in the lists');
      }
    }
    if (noteId == intoId) {
      throw ArgumentError.value(intoId, 'intoId', 'the Note itself');
    }
    store.append(noteId, NoteKeys.merged, intoId);
  }

  /// The Sections of the Note [noteId], oldest first: its Amends, the
  /// Notes merged into it, and their Amends.
  List<Section> sections(String noteId) {
    final merged = {
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (_mergedInto(id) == noteId) id,
    };
    final owners = {noteId, ...merged};
    final sections = [
      for (final id in merged)
        if (byId(id) case final note?)
          Section(
            id: id,
            noteId: noteId,
            text: note.text,
            context: note.context,
          ),
      for (final id in store.entitiesOfType(SectionKeys.typeSection))
        if (owners.contains(store.current(id, SectionKeys.note)))
          ?_section(id, noteId: noteId),
    ];
    sections.sort((a, b) => a.context.captured.compareTo(b.context.captured));
    return sections;
  }

  /// Records where the Note or Section [id] was captured. The position
  /// arrives after Capture, so it is attached later and never holds the
  /// Note back.
  void locate(String id, Coordinates location) {
    final field = switch (store.current(id, NoteKeys.type)) {
      NoteKeys.typeNote => NoteKeys.location,
      SectionKeys.typeSection => SectionKeys.location,
      _ => throw ArgumentError.value(id, 'id', 'not a Note or Section'),
    };
    store.append(id, field, location.encode());
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
          if (!note.isMerged)
            if (purgeAt(note) case final at? when now.isBefore(at)) note,
    ];
    notes.sort(
      (a, b) => (b.trashed ?? b.captured).compareTo(a.trashed ?? a.captured),
    );
    return notes;
  }

  /// Purges every Note that has been in the trash for [trashKeep], with
  /// its Sections, the Notes merged into it included. The log keeps their
  /// entries; nothing shows them again.
  void purgeTrash() {
    final now = store.clock().toUtc();
    store.batch(() {
      for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
        final note = byId(id);
        // A merged Note goes with its target, never on its own.
        final at = note == null || note.isMerged ? null : purgeAt(note);
        if (at == null || now.isBefore(at)) continue;
        // A merged Note's Section is the Note itself; both keep their
        // type under the same key.
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
      context: _context(captured, fields[NoteKeys.location]),
      place: place != null && Places(store).isPlace(place) ? place : null,
      importance:
          Importance.values.asNameMap()[fields[NoteKeys.importance]] ??
          Importance.normal,
      trashed: trashed == null ? null : DateTime.parse(trashed),
      mergedInto: _mergedInto(id),
    );
  }

  /// Every Note neither in the trash nor merged, newest capture first.
  List<Note> all() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed && !note.isMerged)
          note,
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

  /// The Note [id] ends up in at the end of its Merge chain, or null when
  /// it stands alone. A chain stops before a Note this Device does not
  /// hold. A loop, from Devices merging both ways before they synced,
  /// stops at its lowest id, so every Device keeps the same Note.
  String? _mergedInto(String id) {
    final chain = [id];
    while (true) {
      final next = store.current(chain.last, NoteKeys.merged);
      if (next == null ||
          store.current(next, NoteKeys.type) != NoteKeys.typeNote) {
        break;
      }
      final loop = chain.indexOf(next);
      if (loop >= 0) {
        final kept = chain
            .sublist(loop)
            .reduce((a, b) => a.compareTo(b) <= 0 ? a : b);
        return kept == id ? null : kept;
      }
      chain.add(next);
    }
    return chain.last == id ? null : chain.last;
  }

  /// The Section [id]; shown under [noteId] when given, for the Amends of
  /// a merged Note.
  Section? _section(String id, {String? noteId}) {
    final fields = store.currentFields(id);
    if (fields[SectionKeys.type] != SectionKeys.typeSection) return null;
    final owner = fields[SectionKeys.note];
    final captured = fields[SectionKeys.captured];
    if (owner == null || captured == null) return null;
    return Section(
      id: id,
      noteId: noteId ?? owner,
      text: fields[SectionKeys.text] ?? '',
      context: _context(captured, fields[SectionKeys.location]),
    );
  }

  /// The Capture Context from the stored capture time and location, with
  /// the Named Place the location lies in.
  CaptureContext _context(String captured, String? location) {
    final at = Coordinates.decode(location);
    return CaptureContext(
      captured: DateTime.parse(captured),
      location: at,
      namedPlace: at == null ? null : NamedPlaces(store).at(at),
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
