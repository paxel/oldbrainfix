/// A Note's History (#193): every change of the Note, its Amends and the
/// Notes merged into it, and the Lines of its lists, read from the log
/// alone, newest first, each with who made it and when.
library;

import 'checklists.dart';
import 'entry.dart';
import 'notes.dart';
import 'profiles.dart';
import 'shared_areas.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'text_boxes.dart';
import 'text_format.dart';

/// What a change in a Note's History did.
enum NoteChangeKind {
  /// The Note was captured with [NoteChange.after] as its text.
  captured,

  /// The Note's text went from [NoteChange.before] to [NoteChange.after].
  text,

  /// The Note's text was put in another format, its words the same:
  /// [NoteChange.before] and [NoteChange.after] are the formats.
  format,

  /// An Amend added a Section of the text [NoteChange.after].
  amended,

  /// The text of an Amend, or of a Note merged in, went from
  /// [NoteChange.before] to [NoteChange.after].
  amendEdited,

  /// A Note was merged in; [NoteChange.after] is its text now.
  mergedIn,

  /// A Line was added, of the text [NoteChange.after].
  lineAdded,

  /// A Line was renamed from [NoteChange.before] to [NoteChange.after].
  lineRenamed,

  /// The Line [NoteChange.after] was checked.
  lineChecked,

  /// The Line [NoteChange.after] was unchecked.
  lineUnchecked,

  /// The Line [NoteChange.after] was deleted.
  lineRemoved,

  /// The deleted Line [NoteChange.after] was brought back.
  lineRestored,

  /// The Importance went from [NoteChange.before] to [NoteChange.after],
  /// by name; null is normal.
  importance,

  /// The Note was filed from the Area or Topic [NoteChange.before] into
  /// [NoteChange.after], by id; null is Unsorted.
  filed,

  /// The Note became another kind of list: [NoteChange.before] and
  /// [NoteChange.after] are `checklist`, `shopping` or null for a plain
  /// Note.
  kind,

  /// The Note went to the trash.
  trashed,

  /// The Note came back from the trash.
  restored,

  photoAdded,
  photoRemoved,
  recordingAdded,
  recordingRemoved,
}

/// One change in a Note's History.
class NoteChange {
  /// When it was made: its entry's date.
  final DateTime at;

  /// What it changed: the Note, a Section of it (`section:` for an
  /// Amend, `note:` for a Note merged in) or a Line (`line:`).
  final String entity;
  final NoteChangeKind kind;

  /// The value before and after, as [kind] says: a text as it shows,
  /// each box in the glyph it was typed with and without its id.
  final String? before;
  final String? after;

  /// The format of a text [after]: what "Use this version" writes it in;
  /// null for a text in a format or structure not known here, which no
  /// version here may write.
  final String? format;

  /// Who made it, as their Profile shows ([People.editorOf]); null for a
  /// writer not known and a blocked person.
  final Profile? by;

  const NoteChange({
    required this.at,
    required this.entity,
    required this.kind,
    this.before,
    this.after,
    this.format,
    this.by,
  });

  /// Whether [after] is a text this change set: one "Use this version"
  /// can bring back.
  bool get setsText => switch (kind) {
    NoteChangeKind.captured ||
    NoteChangeKind.text ||
    NoteChangeKind.amended ||
    NoteChangeKind.amendEdited => true,
    _ => false,
  };
}

/// Reads a Note's History and brings back an old text of it.
class NoteHistory {
  final Store store;

  NoteHistory(this.store);

  /// Every change of the Note [noteId], newest first by latest wins, so
  /// every Device lists them alike; empty when there is no such Note.
  /// Only what counts is read ([Store.history]): what no role in a
  /// Shared Area allows is left out.
  List<NoteChange> of(String noteId) {
    final notes = Notes(store);
    if (notes.byId(noteId) == null) return const [];
    final found = <(Entry, NoteChange)>[];
    void add(Entry e, NoteChangeKind kind, {String? before, String? after}) =>
        found.add((
          e,
          NoteChange(
            at: e.date,
            entity: e.entity,
            kind: kind,
            before: before,
            after: after,
            by: store.editorOf(noteId, e.device),
          ),
        ));
    void texts(
      String entity,
      NoteChangeKind first,
      NoteChangeKind later, {
      bool skipFirst = false,
    }) {
      StoredText? held;
      for (final e in _byField(entity, NoteKeys.text)) {
        final stored = StoredText.read(e.value);
        final shown = _shown(stored);
        final previous = held;
        held = stored;
        if (previous == null) {
          if (!skipFirst) {
            found.add((e, _textChange(e, noteId, first, null, stored)));
          }
          continue;
        }
        if (_shown(previous) == shown) {
          if (previous.format != stored.format) {
            add(
              e,
              NoteChangeKind.format,
              before: previous.format,
              after: stored.format,
            );
          }
          continue;
        }
        found.add((e, _textChange(e, noteId, later, previous, stored)));
      }
    }

    texts(noteId, NoteChangeKind.captured, NoteChangeKind.text);
    for (final (field, kind) in [
      (NoteKeys.importance, NoteChangeKind.importance),
      (NoteKeys.place, NoteChangeKind.filed),
      (ChecklistKeys.kind, NoteChangeKind.kind),
    ]) {
      String? held;
      for (final e in _byField(noteId, field)) {
        final value = kind == NoteChangeKind.kind ? _list(e.value) : e.value;
        if (value != held) add(e, kind, before: held, after: value);
        held = value;
      }
    }
    for (final e in _byField(noteId, NoteKeys.trashed)) {
      add(
        e,
        e.value == null ? NoteChangeKind.restored : NoteChangeKind.trashed,
      );
    }
    for (final e in store.history(noteId)) {
      final (added, removed) = e.field.startsWith(NoteKeys.photoPrefix)
          ? (NoteChangeKind.photoAdded, NoteChangeKind.photoRemoved)
          : e.field.startsWith(NoteKeys.audioPrefix)
          ? (NoteChangeKind.recordingAdded, NoteChangeKind.recordingRemoved)
          : (null, null);
      if (added == null || removed == null) continue;
      add(e, e.value == null ? removed : added);
    }
    for (final section in notes.sections(noteId)) {
      final id = section.id;
      if (id.startsWith('note:')) {
        final merged = _byField(id, NoteKeys.merged).lastOrNull;
        if (merged != null && merged.value != null) {
          add(merged, NoteChangeKind.mergedIn, after: _textOf(id));
        }
        texts(
          id,
          NoteChangeKind.amended,
          NoteChangeKind.amendEdited,
          skipFirst: true,
        );
      } else {
        texts(id, NoteChangeKind.amended, NoteChangeKind.amendEdited);
      }
    }
    for (final line in _lines(noteId)) {
      final name = StoredText.read(store.current(line, ChecklistKeys.text))
          .text;
      String? held;
      for (final e in _byField(line, ChecklistKeys.text)) {
        final text = StoredText.read(e.value).text;
        if (held == null) {
          add(e, NoteChangeKind.lineAdded, after: text);
        } else if (held != text) {
          add(e, NoteChangeKind.lineRenamed, before: held, after: text);
        }
        held = text;
      }
      for (final e in _byField(line, ChecklistKeys.checked)) {
        add(
          e,
          e.value == null
              ? NoteChangeKind.lineUnchecked
              : NoteChangeKind.lineChecked,
          after: name,
        );
      }
      for (final e in _byField(line, ChecklistKeys.trashed)) {
        add(
          e,
          e.value == null
              ? NoteChangeKind.lineRestored
              : NoteChangeKind.lineRemoved,
          after: name,
        );
      }
    }
    found.sort((a, b) => Store.latestWins(b.$1, a.$1));
    return [for (final (_, change) in found) change];
  }

  /// Whether this Device may bring back the text of [change] with
  /// "Use this version": it set a text that is not the one showing now,
  /// on the Note or a Section this Device may edit.
  bool mayUse(String noteId, NoteChange change) {
    final after = change.after;
    if (!change.setsText || after == null || change.format == null) {
      return false;
    }
    final id = change.entity;
    final notes = Notes(store);
    final may = id == noteId
        ? notes.byId(noteId)?.textEditable == true && store.mayChange(noteId)
        : notes.mayEditSection(id);
    return may && _textOf(id) != after;
  }

  /// Brings back the text of [change] as a new version of the Note
  /// [noteId] or its Section (#193): an edit like any other, nothing is
  /// deleted. Refused unless [mayUse].
  void use(String noteId, NoteChange change) {
    if (!mayUse(noteId, change)) {
      throw StateError('the text of ${change.entity} may not be used here');
    }
    final notes = Notes(store);
    if (change.entity == noteId) {
      notes.edit(noteId, change.after!, format: change.format);
    } else {
      notes.editSection(change.entity, change.after!, format: change.format);
    }
  }

  NoteChange _textChange(
    Entry e,
    String noteId,
    NoteChangeKind kind,
    StoredText? before,
    StoredText after,
  ) => NoteChange(
    at: e.date,
    entity: e.entity,
    kind: kind,
    before: before == null ? null : _shown(before),
    after: _shown(after),
    format: after.editable ? after.format : null,
    by: store.editorOf(noteId, e.device),
  );

  /// The entries of [entity] under [field] that count, oldest first by
  /// latest wins: the last is the value that shows.
  List<Entry> _byField(String entity, String field) => [
    for (final e in store.history(entity))
      if (e.field == field) e,
  ]..sort(Store.latestWins);

  /// The Lines of the Note [noteId], of every making, by id.
  List<String> _lines(String noteId) => [
    for (final id in store.entitiesOfType(ChecklistKeys.typeLine))
      if (store.current(id, ChecklistKeys.note) == noteId) id,
  ]..sort();

  /// The text of the Note or Section [id] as it is now, as [_shown].
  String _textOf(String id) =>
      _shown(StoredText.read(store.current(id, NoteKeys.text)));

  /// [text] as it shows: each box in its glyph without its id, in a
  /// format known here.
  static String _shown(StoredText text) =>
      text.editable ? TextBoxes.strip(text.text) : text.text;

  /// The list a value of [ChecklistKeys.kind] names; null for none.
  static String? _list(String? value) => switch (parseListKind(value)?.list) {
    final list?
        when list == ChecklistKeys.checklist || list == ShoppingKeys.shopping =>
      list,
    _ => null,
  };
}
