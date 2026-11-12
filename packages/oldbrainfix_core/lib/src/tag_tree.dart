import 'markdown.dart';
import 'notes.dart';
import 'store.dart';
import 'text_format.dart';
import 'text_tags.dart';

/// One tag of the tag tree (#267, ADR 0032): a part of the nested tags
/// written in the Notes, such as "health" of "#life/health/back".
class TagNode {
  /// The part as it is written most often, such as "Health".
  final String name;

  /// The tag down to here, `#` and each part as it is written most
  /// often, such as "#Life/Health": what a tap on it searches for.
  final String tag;

  /// How many Notes hold this tag or one under it, each counted once.
  final int notes;

  /// The tags one level under it, by name.
  final List<TagNode> children;

  const TagNode({
    required this.name,
    required this.tag,
    required this.notes,
    this.children = const [],
  });
}

/// The tag tree, the same in both cores (`tag_tree.rs`): every tag in the
/// texts of the Notes neither in the trash nor merged and of their
/// Sections, as the texts draw them, by its levels. Nothing is stored for
/// it; it is read from the texts.
class TagTree {
  final Store store;

  TagTree(this.store);

  /// The tags of the first level, each with those under it, by name:
  /// in lower case, by UTF-16 code units.
  List<TagNode> tree() => store.workedOut(TagTree, _build);

  /// Every tag of the tree whose tag begins with [typed], ignoring case,
  /// each before those under it: what is offered when a `#` is typed in
  /// the search. None when [typed] does not begin with a `#`.
  List<TagNode> suggestions(String typed) {
    final wanted = typed.trim().toLowerCase();
    if (!wanted.startsWith('#')) return const [];
    final out = <TagNode>[];
    void walk(List<TagNode> level) {
      for (final node in level) {
        if (node.tag.toLowerCase().startsWith(wanted)) out.add(node);
        walk(node.children);
      }
    }

    walk(tree());
    return out;
  }

  List<TagNode> _build() {
    final notes = Notes(store);
    final all = notes.all();
    final sections = notes.sectionsOf([for (final n in all) n.id]);
    final root = _Level();
    for (final note in all) {
      final texts = [
        (note.textFormat, note.shownText),
        for (final s in sections[note.id] ?? const <Section>[])
          (s.textFormat, s.shownText),
      ];
      for (final (format, text) in texts) {
        for (final found in tagsIn(format, text)) {
          final written = text.substring(found.start + 1, found.end).split('/');
          final parts = found.tag.substring(1).split('/');
          var level = root;
          for (var i = 0; i < parts.length; i++) {
            final part = level.parts.putIfAbsent(parts[i], _Part.new);
            part.spellings.update(written[i], (n) => n + 1, ifAbsent: () => 1);
            part.notes.add(note.id);
            level = part.under;
          }
        }
      }
    }
    return root.nodes('#');
  }

  /// The tags of [text] in [format] as it is drawn: none in a Code text,
  /// those outside code and links in a formatted one, those outside web
  /// addresses in any other.
  static List<FoundTag> tagsIn(String format, String text) => switch (format) {
    TextFormats.code => const [],
    TextFormats.markdown => Markdown.tags(text),
    _ => Markdown.plainTags(text),
  };
}

/// The parts of one level of the tree being read, by their lower case.
class _Level {
  final parts = <String, _Part>{};

  List<TagNode> nodes(String above) {
    final keys = parts.keys.toList()..sort();
    return [
      for (final key in keys)
        if (parts[key] case final part?) part.node(above),
    ];
  }
}

class _Part {
  final spellings = <String, int>{};
  final notes = <String>{};
  final under = _Level();

  TagNode node(String above) {
    String? name;
    var most = 0;
    for (final MapEntry(key: spelling, value: count) in spellings.entries) {
      if (name == null ||
          count > most ||
          (count == most && spelling.compareTo(name) < 0)) {
        name = spelling;
        most = count;
      }
    }
    final shown = name ?? '';
    final tag = above == '#' ? '#$shown' : '$above/$shown';
    return TagNode(
      name: shown,
      tag: tag,
      notes: notes.length,
      children: under.nodes(tag),
    );
  }
}
