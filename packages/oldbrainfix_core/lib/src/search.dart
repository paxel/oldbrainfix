import 'checklists.dart';
import 'notes.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'tallies.dart';
import 'text_format.dart';

/// Finding Notes by what they say: their text, the Transcripts of their
/// voice recordings, the Photo Texts of their photos and the same of
/// their Sections (Amends and merged Notes), the text lines of the
/// entries of a Tally and the Lines of a Checklist or a Shopping List,
/// but those gone. Capture Context joins later.
/// A formatted text counts as it reads without its marks (ADR 0033), a
/// text in a format not known here as its source text.
class Search {
  final Store store;

  Search(this.store);

  /// The Notes whose text, Transcripts or Photo Texts, or those of one of
  /// their Sections, or the text line of an entry of their Tally, or a
  /// Line of their Checklist or Shopping List, hold
  /// [query], ignoring case, newest capture first.
  /// The query counts as typed, inner spaces included; only its ends are
  /// trimmed, and a blank one finds everything. With [within], only the
  /// Notes inside that Area or Topic count (an Area's Topics included).
  List<Note> notes(String query, {String? within}) {
    final notes = Notes(store);
    final scope = within == null ? notes.all() : notes.inside(within);
    final wanted = query.trim().toLowerCase();
    if (wanted.isEmpty) return scope;
    final sections = _sectionTexts(notes);
    bool holds(String text) => text.toLowerCase().contains(wanted);
    return [
      for (final note in scope)
        if (holds(note.visibleText) ||
            note.transcripts.values.any(holds) ||
            note.photoTexts.values.any(holds) ||
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
      final (:making, :shopping) = makings.putIfAbsent(owner, () {
        final listed = Checklists(store).byNote(owner)?.making;
        if (listed != null) return (making: listed, shopping: false);
        return (making: ShoppingLists(store).making(owner), shopping: true);
      });
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
