import 'package:meta/meta.dart';

import 'checklists.dart';
import 'notes.dart';
import 'places.dart';
import 'search_index.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'tallies.dart';
import 'text_format.dart';

/// Finding Notes by what they say: their text, the Transcripts of their
/// voice recordings, the Photo Texts of their photos, the names and
/// Attachment Texts of their attached files (ADR 0047) and the same of
/// their Sections (Amends and merged Notes), the text lines of the
/// entries of a Tally and the Lines of a Checklist or a Shopping List,
/// but those gone. Capture Context joins later.
/// A formatted text counts as it reads without its marks (ADR 0033), a
/// text in a format not known here as its source text.
///
/// The full-text index (ADR 0048) names the entities that may hold a
/// query, and only those are read; without it, or for a query holding a
/// box's glyph, which a tick changes, every Note is read as before. Both
/// find the same Notes.
class Search {
  final Store store;

  /// Whether the index is asked; else every Note is read.
  final bool _withIndex;

  Search(this.store) : _withIndex = true;

  /// A search that reads every Note, as before the index: what tests and
  /// measurements compare the index with.
  @visibleForTesting
  Search.scanning(this.store) : _withIndex = false;

  /// The Notes whose text, Transcripts, Photo Texts, file names or
  /// Attachment Texts, or those of one of
  /// their Sections, or the text line of an entry of their Tally, or a
  /// Line of their Checklist or Shopping List, hold
  /// [query], ignoring case, newest capture first.
  /// The query counts as typed, inner spaces included; only its ends are
  /// trimmed, and a blank one finds everything. With [within], only the
  /// Notes inside that Area or Topic count (an Area's Topics included).
  List<Note> notes(String query, {String? within}) {
    final notes = Notes(store);
    final wanted = query.trim().toLowerCase();
    final read = <String, Note?>{};
    final found = wanted.isEmpty ? null : _indexed(wanted, read);
    if (found == null) {
      final scope = within == null ? notes.all() : notes.inside(within);
      return wanted.isEmpty ? scope : _scanned(wanted, scope);
    }
    final places = within == null
        ? null
        : {within, for (final topic in Places(store).topics(within)) topic.id};
    // As [Notes.all] and [Notes.inside] list them.
    final hits = [
      for (final id in found)
        if (read.putIfAbsent(id, () => notes.byId(id)) case final note?
            when !note.isTrashed &&
                !note.isMerged &&
                (places == null || places.contains(note.place)))
          note,
    ];
    if (hits.length < 2) return hits;
    Map<String, int>? order;
    int at(String id) =>
        (order ??= {
          for (final (i, id) in store.everOfType(NoteKeys.typeNote).indexed)
            id: i,
        })[id] ??
        -1;
    // Newest capture first; at the same time in the order the store lists
    // the Notes in, as a stable sort of them keeps it.
    hits.sort((a, b) {
      final byTime = b.captured.compareTo(a.captured);
      return byTime != 0 ? byTime : at(a.id).compareTo(at(b.id));
    });
    return hits;
  }

  /// Those of [scope] that [query] finds, as [notes] finds them, in the
  /// order given: for a list the search does not read itself, such as the
  /// trash (#274). A blank query keeps them all.
  List<Note> among(String query, List<Note> scope) {
    final wanted = query.trim().toLowerCase();
    if (wanted.isEmpty) return scope;
    final found = _indexed(wanted, {});
    if (found == null) return _scanned(wanted, scope);
    return [
      for (final note in scope)
        if (found.contains(note.id)) note,
    ];
  }

  /// The glyphs of a box, which shows the state it is ticked to rather
  /// than the one it was written with (ADR 0035).
  static const _boxGlyphs = ['\u2610', '\u2705', '\u274C'];

  /// The ids of the Notes [wanted], in lower case and not blank, is found
  /// in, as [_scanned] finds them in a scope holding every Note: those of
  /// the candidates the index names, read as they are now. Each Note read
  /// is kept in [read]. Null when the index cannot answer.
  Set<String>? _indexed(String wanted, Map<String, Note?> read) {
    if (!_withIndex || _boxGlyphs.any(wanted.contains)) return null;
    final candidates = SearchIndex.candidates(store, wanted);
    if (candidates == null) return null;
    final notes = Notes(store);
    bool holds(String text) => text.toLowerCase().contains(wanted);
    Note? note(String id) => read.putIfAbsent(id, () => notes.byId(id));
    String shownUnder(String owner) => note(owner)?.mergedInto ?? owner;
    final found = <String>{};
    final makings = <String, ({String? making, bool shopping})>{};
    // Read in one query per few hundred rather than one each (#204).
    final fieldsOf = store.currentFieldsOf(candidates);
    for (final id in candidates) {
      final fields = fieldsOf[id] ?? const <String, String?>{};
      switch (fields[NoteKeys.type]) {
        case NoteKeys.typeNote:
          final own = note(id);
          if (own == null || !_holdsOwn(own, holds)) continue;
          found.add(id);
          if (own.mergedInto case final into?) found.add(into);
        case SectionKeys.typeSection:
          final owner = fields[SectionKeys.note];
          if (owner == null) continue;
          if (holds(StoredText.read(fields[SectionKeys.text]).visible)) {
            found.add(shownUnder(owner));
          }
        case TallyKeys.typeEntry:
          final owner = fields[TallyKeys.note];
          final line = fields[TallyKeys.line];
          if (owner == null ||
              line == null ||
              fields[TallyKeys.trashed] != null) {
            continue;
          }
          if (holds(line)) found.add(shownUnder(owner));
        case ChecklistKeys.typeLine:
          final owner = fields[ChecklistKeys.note];
          if (owner == null || fields[ChecklistKeys.trashed] != null) continue;
          final (:making, :shopping) = makings.putIfAbsent(
            owner,
            () => _making(owner),
          );
          if (making == null || fields[ChecklistKeys.made] != making) continue;
          if (shopping && ShoppingLists.isGone(fields)) continue;
          if (holds(StoredText.read(fields[ChecklistKeys.text]).text)) {
            found.add(shownUnder(owner));
          }
      }
    }
    return found;
  }

  /// Whether the Note's own text, Transcripts, Photo Texts, Attachment
  /// Texts or file names [holds] the query.
  static bool _holdsOwn(Note note, bool Function(String) holds) =>
      holds(note.visibleText) ||
      note.transcripts.values.any(holds) ||
      note.photoTexts.values.any(holds) ||
      note.attachmentTexts.values.any(holds) ||
      note.attachments.any((a) => holds(a.name));

  /// The making of the Checklist or Shopping List of the Note [owner]
  /// whose Lines count now, and whether it is a Shopping List.
  ({String? making, bool shopping}) _making(String owner) {
    final listed = Checklists(store).byNote(owner)?.making;
    if (listed != null) return (making: listed, shopping: false);
    return (making: ShoppingLists(store).making(owner), shopping: true);
  }

  /// Those of [scope] [wanted], in lower case and not blank, is found
  /// in, each Note read in full: the search without the index.
  List<Note> _scanned(String wanted, List<Note> scope) {
    final sections = _sectionTexts(Notes(store));
    bool holds(String text) => text.toLowerCase().contains(wanted);
    return [
      for (final note in scope)
        if (_holdsOwn(note, holds) ||
            (sections[note.id] ?? const []).any(holds))
          note,
    ];
  }

  /// The texts of every Section, Tally entry and Line, by the Note they
  /// show under, read in one pass: a merged Note's text, Transcripts and
  /// Photo Texts under the end of its Merge chain, an Amend, the text
  /// line of an entry not removed or a Line not deleted of the Checklist
  /// its Note is now under its Note, or where that Note was merged into.
  Map<String, List<String>> _sectionTexts(Notes notes) {
    final texts = <String, List<String>>{};
    for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
      if (notes.byId(id) case final note?) {
        final into = note.mergedInto;
        if (into == null) continue;
        (texts[into] ??= []).addAll([
          note.visibleText,
          ...note.transcripts.values,
          ...note.photoTexts.values,
          ...note.attachmentTexts.values,
          for (final file in note.attachments) file.name,
        ]);
      }
    }
    for (final id in store.entitiesOfType(SectionKeys.typeSection)) {
      final fields = store.currentFields(id);
      final owner = fields[SectionKeys.note];
      if (owner == null) continue;
      final shownUnder = notes.byId(owner)?.mergedInto ?? owner;
      (texts[shownUnder] ??= []).add(
        StoredText.read(fields[SectionKeys.text]).visible,
      );
    }
    for (final id in store.entitiesOfType(TallyKeys.typeEntry)) {
      final fields = store.currentFields(id);
      final owner = fields[TallyKeys.note];
      final line = fields[TallyKeys.line];
      if (owner == null || line == null || fields[TallyKeys.trashed] != null) {
        continue;
      }
      final shownUnder = notes.byId(owner)?.mergedInto ?? owner;
      (texts[shownUnder] ??= []).add(line);
    }
    final makings = <String, ({String? making, bool shopping})>{};
    for (final id in store.entitiesOfType(ChecklistKeys.typeLine)) {
      final fields = store.currentFields(id);
      final owner = fields[ChecklistKeys.note];
      if (owner == null || fields[ChecklistKeys.trashed] != null) continue;
      final (:making, :shopping) = makings.putIfAbsent(
        owner,
        () => _making(owner),
      );
      if (making == null || fields[ChecklistKeys.made] != making) continue;
      if (shopping && ShoppingLists.isGone(fields)) continue;
      final shownUnder = notes.byId(owner)?.mergedInto ?? owner;
      (texts[shownUnder] ??= []).add(
        StoredText.read(fields[ChecklistKeys.text]).text,
      );
    }
    return texts;
  }
}
