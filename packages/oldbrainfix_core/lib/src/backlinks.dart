import 'app_link.dart';
import 'markdown.dart';
import 'notes.dart';
import 'places.dart';
import 'search_index.dart';
import 'store.dart';
import 'text_format.dart';

/// The Notes that link to a Note (obf-1035, ADR 0046), the same in both
/// cores (`backlinks.rs`): those neither in the trash nor merged whose
/// text, or the text of one of their Sections, holds an app link to it,
/// `oldbrainfix://note/<id>`, in a formatted text as it is drawn. Nothing
/// is stored for it; it is read from the texts and kept until the log
/// changes, the full-text index (ADR 0048) naming the texts that may hold
/// one, so a change reads only those. The Notes filed into an Area of
/// `closed`, the locked Areas not open now, are left out, as from every
/// list (Area Locks); a Note of a Shared Area the reader is no member of
/// is none this store reads (ADR 0013).
class Backlinks {
  final Store store;

  Backlinks(this.store);

  /// The Notes that link to the Note [noteId], newest capture first and
  /// by id within one capture time, each once; never [noteId] itself.
  /// Those filed into an Area of [closed] are left out.
  List<Note> to(String noteId, {Set<String> closed = const {}}) {
    final linking = store.workedOut<Map<String, List<_Linking>>>(
      Backlinks,
      _read,
    )[noteId];
    return [
      for (final l in linking ?? const <_Linking>[])
        if (l.area == null || !closed.contains(l.area)) l.note,
    ];
  }

  /// The ids of the Notes [text] in [format] links to, as it is drawn:
  /// only a formatted text has links; in it every link whose destination
  /// [AppLink.parse] reads as a Note's, in a line, a table's cell, bold,
  /// italic or struck through.
  static Set<String> linksIn(String format, String text) {
    if (format != TextFormats.markdown) return const {};
    final ids = <String>{};
    void walk(List<MarkdownSpan> spans) {
      for (final span in spans) {
        if (span.kind == MarkdownSpanKind.link) {
          final link = AppLink.parse(span.url);
          if (link != null && link.kind == AppLinkKind.note) {
            if (link.target case final id?) ids.add(id);
          }
        }
        walk(span.children);
      }
    }

    for (final block in Markdown.parse(text)) {
      walk(block.spans);
    }
    return ids;
  }

  /// The Notes linking to each Note, by the id linked to, with the Area
  /// each is filed into, whatever is closed: kept until the log changes.
  Map<String, List<_Linking>> _read() {
    final notes = Notes(store);
    final places = Places(store);
    final listed = [
      for (final note in _maybeLinking(notes))
        if (!note.isTrashed && !note.isMerged) note,
    ];
    listed.sort((a, b) {
      final by = b.captured.compareTo(a.captured);
      return by != 0 ? by : a.id.compareTo(b.id);
    });
    final out = <String, List<_Linking>>{};
    for (final note in listed) {
      final targets = {
        ...linksIn(note.textFormat, note.shownText),
        for (final section in notes.sections(note.id))
          ...linksIn(section.textFormat, section.shownText),
      }..remove(note.id);
      if (targets.isEmpty) continue;
      final place = note.place;
      final area = place == null
          ? null
          : places.topicById(place)?.areaId ?? place;
      for (final target in targets) {
        (out[target] ??= []).add(_Linking(note, area));
      }
    }
    return out;
  }

  /// The Notes whose text or Sections may hold an app link: those the
  /// full-text index names as holding the scheme once, by their own text
  /// or a Section's, and those they were merged into; every Note when the
  /// index cannot answer. Some may hold none, and none is missed.
  List<Note> _maybeLinking(Notes notes) {
    final found = SearchIndex.candidates(store, AppLink.scheme);
    if (found == null) return notes.all();
    final ids = <String>{};
    void add(String id) {
      if (ids.contains(id)) return;
      ids.add(id);
      if (notes.byId(id)?.mergedInto case final into?) ids.add(into);
    }

    final fieldsOf = store.currentFieldsOf(found);
    for (final id in found) {
      final fields = fieldsOf[id] ?? const <String, String?>{};
      switch (fields[NoteKeys.type]) {
        case NoteKeys.typeNote:
          add(id);
        case SectionKeys.typeSection:
          if (fields[SectionKeys.note] case final owner?) add(owner);
      }
    }
    return [for (final id in ids) ?notes.byId(id)];
  }
}

/// A Note linking to another, and the Area it is filed into, directly or
/// through a Topic.
class _Linking {
  final Note note;
  final String? area;

  const _Linking(this.note, this.area);
}
