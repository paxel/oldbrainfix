import 'notes.dart';
import 'store.dart';

/// Finding Notes by what they say. For now that is the Note text;
/// Transcripts, OCR text and Capture Context join later.
class Search {
  final Store store;

  Search(this.store);

  /// The Notes whose text holds [query], ignoring case, newest capture
  /// first. The query counts as typed, inner spaces included; only its
  /// ends are trimmed, and a blank one finds everything. With [within],
  /// only the Notes inside that Area or Topic count (an Area's Topics
  /// included).
  List<Note> notes(String query, {String? within}) {
    final notes = Notes(store);
    final scope = within == null ? notes.all() : notes.inside(within);
    final wanted = query.trim().toLowerCase();
    if (wanted.isEmpty) return scope;
    return [
      for (final note in scope)
        if (note.text.toLowerCase().contains(wanted)) note,
    ];
  }
}
