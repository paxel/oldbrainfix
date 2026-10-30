/// Formatted text (ADR 0033): the subset of Markdown a Note's or a
/// Section's text in [TextFormats.markdown] is read in, the visible text
/// without its marks, and the edits the editors make, the same in both
/// cores (`markdown.rs`).
///
/// The text is kept as it was written; nothing here rewrites it, and
/// whatever the subset does not know shows as the characters it is.
/// Every line is a block of its own, so a line break always shows as
/// one. A line is, the first that fits:
///
/// - a heading: up to three spaces, `#` or `##`, then a space, a tab or
///   the line's end;
/// - a quote: up to three spaces and `>`, one space or tab after it
///   belonging to the mark;
/// - a bulleted item: any spaces and tabs, `-`, `*` or `+`, then a space,
///   a tab or the line's end;
/// - a numbered item: any spaces and tabs, one to nine digits and `.`,
///   then a space, a tab or the line's end;
/// - a paragraph line otherwise, its spaces at the start included.
///
/// Spaces count one and a tab four; an item indented by two or more is
/// one level in, deeper ones too. The spaces and tabs after an item's or
/// a heading's mark belong to the mark. A numbered item right after one
/// of its level, with only items further in between, shows the number
/// after that one's; any other shows its own.
///
/// Within a line: `\` before ASCII punctuation shows that character; a
/// run of backticks up to the next run of as many is code, shown as it
/// is, one space at both ends dropped when there is one at both; `[`
/// with its `]` and `(destination)` right after it is a link, its
/// destination without spaces, any parentheses in it balanced, and no
/// link inside; `*` and `_` runs are emphasis by CommonMark's flanking
/// rules, ASCII punctuation counting as punctuation and the start and
/// end of the line as space, one for italic and two for bold; a run of
/// exactly two `~` is strike-through, by the flanking rules of `*`. A
/// `[` right after a `!` is no link, so an image shows as written.
///
/// A table (GFM, #128) takes a line of its own per row: a header row,
/// the delimiter row under it, `| --- | :-: |`, with as many cells, and
/// the rows under those; [MarkdownTables] reads and edits them, every
/// line of one a [MarkdownBlockKind.table] block.
library;

import 'text_marks.dart';
import 'text_tags.dart';

part 'markdown_table.dart';

/// What a line of a formatted text is.
enum MarkdownBlockKind {
  paragraph,
  heading1,
  heading2,
  bullet,
  numbered,
  quote,

  /// A line of a table: its header row, its delimiter row, one of its
  /// rows, or the line marking its first column as headings.
  table,
}

/// What a part of a line is: [text] and [code] hold what shows, the
/// others the parts inside them.
enum MarkdownSpanKind { text, bold, italic, strike, code, link }

/// A part of a line: its [kind], where it lies in the text from [start]
/// to [end] with its marks, and its content between them, from
/// [contentStart] to [contentEnd]: a link's label, the characters
/// between a code part's backticks. Offsets are UTF-16 code units of
/// the whole text.
///
/// A [MarkdownSpanKind.text] part is either a run of characters shown as
/// written, [text] the same as the source from [start] to [end], or one
/// escaped character, `\*` showing `*`, its [contentStart] one after
/// [start].
class MarkdownSpan {
  final MarkdownSpanKind kind;
  final int start;
  final int end;
  final int contentStart;
  final int contentEnd;

  /// What a text or a code part shows; empty for the others.
  final String text;

  /// Where a link goes, its escapes resolved, as written by whoever wrote
  /// the text; empty for the others.
  final String url;

  /// The parts inside bold, italic, strike-through and a link's label.
  final List<MarkdownSpan> children;

  const MarkdownSpan(
    this.kind, {
    required this.start,
    required this.end,
    required this.contentStart,
    required this.contentEnd,
    this.text = '',
    this.url = '',
    this.children = const [],
  });

  /// The text this part shows, without its marks.
  String get visible =>
      kind == MarkdownSpanKind.text || kind == MarkdownSpanKind.code
      ? text
      : children.map((c) => c.visible).join();
}

/// A line of a formatted text: its [kind], the [line] it is, counted as
/// `TextMarks.lineAt` counts lines, where it lies in the text from
/// [start] to [end] without its line break, and where its content begins
/// after the mark, [contentStart]. Offsets are UTF-16 code units.
///
/// A line of a table holds the [table] it belongs to and, a row of it,
/// its [row] there, the header row 0; its [spans] are those of its cells,
/// left to right. The delimiter row and the line marking the first
/// column as headings have no [row].
class MarkdownBlock {
  final MarkdownBlockKind kind;
  final int line;

  /// How far in an item is: 0 or 1; 0 for any other line.
  final int level;

  /// The number a numbered item shows; 0 for any other line.
  final int number;
  final int start;
  final int end;
  final int contentStart;
  final List<MarkdownSpan> spans;
  final MarkdownTable? table;
  final int? row;

  const MarkdownBlock(
    this.kind, {
    required this.line,
    required this.level,
    required this.number,
    required this.start,
    required this.end,
    required this.contentStart,
    required this.spans,
    this.table,
    this.row,
  });

  /// The text this line shows, without its marks: a table's row its
  /// cells that show something, a space between them, its other lines
  /// nothing.
  String get visible => switch ((table, row)) {
    (final table?, final row?) => table.rows[row].visible,
    (_?, null) => '',
    _ => spans.map((s) => s.visible).join(),
  };
}

/// A status mark found in a formatted text: as `TextMarks.find` finds
/// it at the start of a line, [column] null, or at the start of the cell
/// in [column] of a table's row on the line [line]. Offsets are UTF-16
/// code units, a variation selector after the mark included.
typedef MarkdownMark = ({
  int line,
  int? column,
  TextMark mark,
  int start,
  int end,
});

/// A text after an edit and the selection in it, in UTF-16 code units;
/// [start] and [end] are equal for a cursor.
typedef MarkdownChange = ({String text, int start, int end});

/// Reading formatted text and the edits the editors make on it.
abstract final class Markdown {
  /// Every line of [text] as a block, top line first.
  static List<MarkdownBlock> parse(String text) {
    final blocks = <MarkdownBlock>[];
    final lines = _lines(text);
    for (var i = 0; i < lines.length; i++) {
      if (_tableAt(text, lines, i) case final table?) {
        final header = table.rows.first.line;
        for (; i <= table.lastLine; i++) {
          final row = i == header
              ? 0
              : i > table.delimiterLine
              ? i - table.delimiterLine
              : null;
          blocks.add(
            MarkdownBlock(
              MarkdownBlockKind.table,
              line: i,
              level: 0,
              number: 0,
              start: lines[i].$1,
              end: lines[i].$2,
              contentStart: lines[i].$1,
              spans: [
                if (row != null)
                  for (final cell in table.rows[row].cells) ...cell.spans,
              ],
              table: table,
              row: row,
            ),
          );
        }
        i--;
        continue;
      }
      final (start, end) = lines[i];
      final (kind, level, written, content) = _block(text, start, end);
      blocks.add(
        MarkdownBlock(
          kind,
          line: i,
          level: level,
          number: kind == MarkdownBlockKind.numbered
              ? _shown(blocks, level, written)
              : 0,
          start: start,
          end: end,
          contentStart: content,
          spans: _Inline(text, content, end, links: true).parse(),
        ),
      );
    }
    return blocks;
  }

  /// [text] as it reads without its marks, its line breaks as they are:
  /// what search looks in and what a preview shows. A table's delimiter
  /// row and the line marking its first column as headings go with a
  /// line break; each of its rows reads as its cells that show
  /// something, a space between them.
  static String plain(String text) {
    final out = StringBuffer();
    var at = 0;
    final blocks = parse(text);
    for (final (i, block) in blocks.indexed) {
      if (block.table != null && block.row == null) {
        // The line break before it goes, or the one after the first
        // line, which is followed by its table's header row.
        at = i == 0 ? blocks[1].start : block.end;
        continue;
      }
      out
        ..write(text.substring(at, block.start))
        ..write(block.visible);
      at = block.end;
    }
    return out.toString();
  }

  /// [text] as HTML, for an app that shows formatted text another app
  /// hands it (#129), the same in both cores. Lines in a row of
  /// paragraph lines are one `<p>`, of quote lines one `<blockquote>`,
  /// with `<br>` between them; a line of only spaces and tabs ends either
  /// and shows nothing. Headings are `<h1>` and `<h2>`. Items in a row
  /// are one `<ul>` or `<ol>`, `start` its first number unless that is
  /// 1, those one level in a list inside the item above them; another
  /// kind of item starts a list of its own. A table is a `<table>`, its
  /// header row in `<thead>` with `<th>` cells, the first cell of each
  /// other row a `<th>` too when its first column holds headings, a
  /// column's alignment as `align`. Bold is `<strong>`, italic `<em>`,
  /// strike-through `<del>`, code `<code>`; a link is an `<a>` only when
  /// [mayOpen] allows its destination, else its label alone. Every
  /// character of the text is escaped, so nothing in it is markup;
  /// status marks and tags are text. A line break follows each part at
  /// the top.
  static String html(String text) {
    final out = StringBuffer();
    // The paragraph or quote open, and the kinds of the lists open, the
    // outer one first, each with an item open.
    String? open;
    final lists = <MarkdownBlockKind>[];
    void closeLists(int keep) {
      while (lists.length > keep) {
        final kind = lists.removeLast();
        out.write(
          kind == MarkdownBlockKind.numbered ? '</li></ol>' : '</li></ul>',
        );
        if (lists.isEmpty) out.write('\n');
      }
    }

    void close() {
      if (open != null) out.write('</$open>\n');
      open = null;
      closeLists(0);
    }

    void openAs(String tag) {
      if (open == tag) {
        out.write('<br>');
      } else {
        close();
        out.write('<$tag>');
        open = tag;
      }
    }

    for (final block in parse(text)) {
      switch (block.kind) {
        case MarkdownBlockKind.table:
          final table = block.table;
          if (table == null || block.line != table.firstLine) continue;
          close();
          _htmlTable(out, table);
        case MarkdownBlockKind.paragraph:
          if (_skipBlank(text, block.start, block.end) == block.end) {
            close();
            continue;
          }
          openAs('p');
          _htmlSpans(out, block.spans);
        case MarkdownBlockKind.quote:
          openAs('blockquote');
          _htmlSpans(out, block.spans);
        case MarkdownBlockKind.heading1 || MarkdownBlockKind.heading2:
          close();
          final tag = block.kind == MarkdownBlockKind.heading1 ? 'h1' : 'h2';
          out.write('<$tag>');
          _htmlSpans(out, block.spans);
          out.write('</$tag>\n');
        case MarkdownBlockKind.bullet || MarkdownBlockKind.numbered:
          if (open != null) close();
          // One level in only inside an item.
          final level = block.level < lists.length ? block.level : lists.length;
          closeLists(level + 1);
          if (lists.length == level + 1 && lists[level] == block.kind) {
            out.write('</li><li>');
          } else {
            closeLists(level);
            out.write(switch (block.kind) {
              MarkdownBlockKind.numbered when block.number != 1 =>
                '<ol start="${block.number}"><li>',
              MarkdownBlockKind.numbered => '<ol><li>',
              _ => '<ul><li>',
            });
            lists.add(block.kind);
          }
          _htmlSpans(out, block.spans);
      }
    }
    close();
    return out.toString();
  }

  static void _htmlTable(StringBuffer out, MarkdownTable table) {
    void row(MarkdownTableRow row, bool header) {
      out.write('<tr>');
      for (final (c, cell) in row.cells.indexed) {
        final tag = header || (c == 0 && table.headingColumn) ? 'th' : 'td';
        final align = c < table.aligns.length
            ? table.aligns[c]
            : MarkdownAlign.none;
        out.write(
          align == MarkdownAlign.none
              ? '<$tag>'
              : '<$tag align="${align.name}">',
        );
        _htmlSpans(out, cell.spans);
        out.write('</$tag>');
      }
      out.write('</tr>');
    }

    out.write('<table><thead>');
    row(table.rows.first, true);
    out.write('</thead>');
    if (table.rows.length > 1) {
      out.write('<tbody>');
      for (final r in table.rows.skip(1)) {
        row(r, false);
      }
      out.write('</tbody>');
    }
    out.write('</table>\n');
  }

  static void _htmlSpans(StringBuffer out, List<MarkdownSpan> spans) {
    for (final s in spans) {
      switch (s.kind) {
        case MarkdownSpanKind.text:
          out.write(_htmlEscape(s.text));
        case MarkdownSpanKind.code:
          out.write('<code>${_htmlEscape(s.text)}</code>');
        case MarkdownSpanKind.link when mayOpen(s.url):
          out.write('<a href="${_htmlEscape(s.url)}">');
          _htmlSpans(out, s.children);
          out.write('</a>');
        case MarkdownSpanKind.link:
          _htmlSpans(out, s.children);
        case MarkdownSpanKind.bold ||
            MarkdownSpanKind.italic ||
            MarkdownSpanKind.strike:
          final tag = switch (s.kind) {
            MarkdownSpanKind.bold => 'strong',
            MarkdownSpanKind.italic => 'em',
            _ => 'del',
          };
          out.write('<$tag>');
          _htmlSpans(out, s.children);
          out.write('</$tag>');
      }
    }
  }

  /// [text] with `&`, `<`, `>`, `"` and `'` written as HTML references.
  static String _htmlEscape(String text) {
    final out = StringBuffer();
    for (final c in text.codeUnits) {
      out.write(switch (c) {
        0x26 => '&amp;',
        0x3C => '&lt;',
        0x3E => '&gt;',
        0x22 => '&quot;',
        0x27 => '&#39;',
        _ => String.fromCharCode(c),
      });
    }
    return out.toString();
  }

  /// Whether [text] uses any format: a line other than a paragraph line,
  /// or bold, italic, strike-through, code or a link. An escaped
  /// character is none.
  static bool uses(String text) => parse(text).any(
    (b) =>
        b.kind != MarkdownBlockKind.paragraph ||
        b.spans.any((s) => s.kind != MarkdownSpanKind.text),
  );

  /// [text], taken as plain text, written so that it reads as itself:
  /// [plain] of the result is [text], and it uses no format. For a
  /// Transcript or Photo Text put into a formatted text. A `\` goes
  /// before a line's mark, every `*`, `` ` ``, `[` and `|`, a `~` next to
  /// another, a `_` not between two characters that are neither space
  /// nor punctuation, and a `\` before ASCII punctuation. With every `|`
  /// escaped no line is a table's, so a line's mark is the one it has
  /// as a line of its own.
  static String escape(String text) {
    final out = StringBuffer();
    var at = 0;
    for (final block in parse(text)) {
      out.write(text.substring(at, block.start));
      final mark = switch (_block(text, block.start, block.end).$1) {
        MarkdownBlockKind.paragraph => -1,
        MarkdownBlockKind.numbered => text.indexOf('.', block.start),
        _ => _skipBlank(text, block.start, block.end),
      };
      var from = block.start;
      for (var i = block.start; i < block.end; i++) {
        if (i == mark || _needsEscape(text, i, block.start, block.end)) {
          out
            ..write(text.substring(from, i))
            ..write(r'\');
          from = i;
        }
      }
      out.write(text.substring(from, block.end));
      at = block.end;
    }
    return out.toString();
  }

  /// The tags of [text] as [TextTags.find] finds them, but those in code
  /// or in a link's destination.
  static List<FoundTag> tags(String text) {
    final hidden = <(int, int)>[];
    void hide(List<MarkdownSpan> spans) {
      for (final s in spans) {
        if (s.kind == MarkdownSpanKind.code) hidden.add((s.start, s.end));
        if (s.kind == MarkdownSpanKind.link) hidden.add((s.contentEnd, s.end));
        hide(s.children);
      }
    }

    for (final block in parse(text)) {
      hide(block.spans);
    }
    return [
      for (final tag in TextTags.find(text))
        if (!hidden.any((h) => tag.start < h.$2 && h.$1 < tag.end)) tag,
    ];
  }

  /// Deprecated: replaced by the boxes of `text_boxes.dart` (ADR 0035);
  /// kept only until the apps have switched over.
  ///
  /// Every status mark of [text], top line first and left to right: those
  /// `TextMarks.find` finds on a line that is no table's, and one at the
  /// start of a cell of a table's row.
  static List<MarkdownMark> marks(String text) {
    final blocks = parse(text);
    final marks = <MarkdownMark>[];
    for (final m in TextMarks.find(text)) {
      if (blocks[m.line].table != null) continue;
      marks.add((
        line: m.line,
        column: null,
        mark: m.mark,
        start: m.start,
        end: m.end,
      ));
    }
    for (final block in blocks) {
      final (table, row) = (block.table, block.row);
      if (table == null || row == null) continue;
      for (final (column, cell) in table.rows[row].cells.indexed) {
        if (_cellMark(text, cell.start, cell.end) case (
          final mark,
          final start,
          final end,
        )) {
          marks.add((
            line: block.line,
            column: column,
            mark: mark,
            start: start,
            end: end,
          ));
        }
      }
    }
    marks.sort((a, b) => a.start.compareTo(b.start));
    return marks;
  }

  /// Deprecated: replaced by the boxes of `text_boxes.dart` (ADR 0035);
  /// kept only until the apps have switched over.
  ///
  /// [text] with the status mark [marks] finds on the line [line], in the
  /// cell in [column] of a table's row or, [column] null, at the line's
  /// start, changed to [to], everything else as it was. Null when there
  /// is nothing to change: no such mark, or it is [to] already.
  static String? swapMark(String text, int line, TextMark to, {int? column}) {
    for (final m in marks(text)) {
      if (m.line != line || m.column != column) continue;
      if (m.mark == to) return null;
      return text.replaceRange(m.start, m.end, to.glyph);
    }
    return null;
  }

  /// Whether a link to [url] may be opened: a web address, `http://` or
  /// `https://`, or an e-mail address, `mailto:`, in any case of ASCII
  /// letters. A text may come from another person's Device, so nothing
  /// else is opened from it: no file, no app's own address, no script.
  static bool mayOpen(String url) {
    final lower = String.fromCharCodes([
      for (final c in url.codeUnits) c >= 0x41 && c <= 0x5A ? c | 0x20 : c,
    ]);
    return lower.startsWith('https://') ||
        lower.startsWith('http://') ||
        lower.startsWith('mailto:');
  }

  /// The word at the UTF-16 offset [offset] of [text], from where to
  /// where: letters, digits and `_`, and `'`, `’` and `-` between them,
  /// with a tag's `#` before it. A word ending at [offset] counts. Both
  /// are [offset] when there is none.
  static (int, int) wordAt(String text, int offset) {
    var at = offset.clamp(0, text.length);
    if (at > 0 &&
        at < text.length &&
        _low(text.codeUnitAt(at)) &&
        _high(text.codeUnitAt(at - 1))) {
      at--;
    }
    bool wordAfter(int i) => i < text.length && _word(_at(text, i));
    bool wordBefore(int i) => i > 0 && _word(_before(text, i));
    var end = at;
    while (true) {
      if (wordAfter(end)) {
        end += _at(text, end) > 0xFFFF ? 2 : 1;
      } else if (end < text.length &&
          _joiner(text.codeUnitAt(end)) &&
          wordBefore(end) &&
          wordAfter(end + 1)) {
        end++;
      } else {
        break;
      }
    }
    var start = at;
    while (true) {
      if (wordBefore(start)) {
        start -= _before(text, start) > 0xFFFF ? 2 : 1;
      } else if (start > 0 &&
          _joiner(text.codeUnitAt(start - 1)) &&
          wordAfter(start) &&
          wordBefore(start - 1)) {
        start--;
      } else {
        break;
      }
    }
    if (start == end) return (at, at);
    if (start > 0 &&
        text.codeUnitAt(start - 1) == _hash &&
        TextTags.find(text).any((t) => t.start == start - 1)) {
      start--;
    }
    return (start, end);
  }

  /// [text] with bold, italic, strike-through or code ([kind]) put on
  /// the selection from [start] to [end], or taken off when it has it.
  ///
  /// A cursor takes the format off the part its content holds the cursor
  /// in, puts it on the word at the cursor, or, without a word, puts both
  /// marks there with the cursor between them. A selection is taken line
  /// by line without the spaces at its ends: when every such piece lies
  /// in a part of that kind, marks included, those parts lose their
  /// marks; otherwise every piece that does not gets them. Code gets more
  /// backticks than any run inside it, and a space inside when it starts
  /// or ends with one. The selection keeps what it selected.
  static MarkdownChange toggle(
    String text,
    int start,
    int end,
    MarkdownSpanKind kind,
  ) {
    if (!_inline.contains(kind)) {
      throw ArgumentError.value(
        kind,
        'kind',
        'not bold, italic, strike or code',
      );
    }
    final (s, e) = _ordered(text, start, end);
    final blocks = parse(text);
    if (s == e) {
      final block = blocks[TextMarks.lineAt(text, s)];
      final inside = _deepest(
        block.spans,
        kind,
        (span) => span.contentStart <= s && s <= span.contentEnd,
      );
      if (inside != null) return _apply(text, _unwrap(inside), s, e);
      final (from, to) = wordAt(text, s);
      if (from == to) {
        final (open, close) = _marks(kind, '');
        return (
          text: text.replaceRange(s, s, '$open$close'),
          start: s + open.length,
          end: s + open.length,
        );
      }
      return _apply(text, _wrap(text, from, to, kind), s, e);
    }
    final pieces = _pieces(text, blocks, s, e);
    if (pieces.isEmpty) return (text: text, start: s, end: e);
    final within = [
      for (final (block, from, to) in pieces)
        _deepest(
          block.spans,
          kind,
          (span) => span.start <= from && to <= span.end,
        ),
    ];
    if (within.every((span) => span != null)) {
      return _apply(text, [for (final span in within) ..._unwrap(span!)], s, e);
    }
    return _apply(
      text,
      [
        for (final (i, (_, from, to)) in pieces.indexed)
          if (within[i] == null) ..._wrap(text, from, to, kind),
      ],
      s,
      e,
    );
  }

  /// [text] with a link to [url] on the selection from [start] to [end]
  /// on its first line, or on the word at a cursor. In a link already,
  /// its destination becomes [url], and a blank [url] takes the link
  /// off. Without a word, the link is put at the cursor with [url] as
  /// its label, selected. Spaces and control characters in [url] are
  /// written %-encoded, `(`, `)` and `\` escaped.
  static MarkdownChange link(String text, int start, int end, String url) {
    var (s, e) = _ordered(text, start, end);
    final block = parse(text)[TextMarks.lineAt(text, s)];
    if (e > block.end) e = block.end;
    final address = _trimmedUrl(url);
    final destination = _destination(address);
    final inside = _deepest(
      block.spans,
      MarkdownSpanKind.link,
      (span) => span.start <= s && e <= span.end,
    );
    if (inside != null) {
      if (destination.isEmpty) return _apply(text, _unwrap(inside), s, e);
      return _apply(
        text,
        [
          _Edit(
            inside.contentEnd,
            inside.end - inside.contentEnd,
            ']($destination)',
          ),
        ],
        s,
        e,
      );
    }
    if (destination.isEmpty) return (text: text, start: s, end: e);
    final (from, to) = s == e ? wordAt(text, s) : _trimmed(text, s, e);
    if (from == to) {
      // A `]` would end the label.
      final label = escape(address).replaceAll(']', r'\]');
      return (
        text: text.replaceRange(s, s, '[$label]($destination)'),
        start: s + 1,
        end: s + 1 + label.length,
      );
    }
    return _apply(
      text,
      [_Edit(from, 0, '[', after: true), _Edit(to, 0, ']($destination)')],
      s,
      e,
    );
  }

  /// [text] with every line the selection from [start] to [end] touches
  /// made a [kind] line, or made paragraph lines when they all are of
  /// [kind] already. A selection ending at a line's start leaves that
  /// line out. The old mark goes and the new one comes at the line's
  /// start, an item keeping how far in it is; numbered items count on
  /// from one of their level right above, or from 1. A cursor in a
  /// line's mark goes after the new one. A table's lines stay as they
  /// are, and [MarkdownBlockKind.table] changes nothing.
  static MarkdownChange toggleLine(
    String text,
    int start,
    int end,
    MarkdownBlockKind kind,
  ) {
    final (s, e) = _ordered(text, start, end);
    if (kind == MarkdownBlockKind.table) return (text: text, start: s, end: e);
    final blocks = parse(text);
    final (first, last) = _touched(text, blocks, s, e);
    // A table's lines stay its own.
    final lines = [
      for (final b in blocks.sublist(first, last + 1))
        if (b.kind != MarkdownBlockKind.table) b,
    ];
    final target =
        kind != MarkdownBlockKind.paragraph &&
            lines.every((b) => b.kind == kind)
        ? MarkdownBlockKind.paragraph
        : kind;
    final next = [
      for (final level in [0, 1])
        switch (_numberAfter(blocks, first, level)) {
          0 => 1,
          final n => n,
        },
    ];
    final edits = <_Edit>[];
    for (final b in lines) {
      final listed = _listed(b.kind);
      final level = listed ? b.level : 0;
      final lead = listed
          ? text.substring(b.start, _skipBlank(text, b.start, b.end))
          : '';
      final prefix = switch (target) {
        MarkdownBlockKind.paragraph || MarkdownBlockKind.table => '',
        MarkdownBlockKind.heading1 => '# ',
        MarkdownBlockKind.heading2 => '## ',
        MarkdownBlockKind.quote => '> ',
        MarkdownBlockKind.bullet => '$lead- ',
        MarkdownBlockKind.numbered => '$lead${next[level]++}. ',
      };
      if (level == 0) next[1] = 1;
      if (target == MarkdownBlockKind.paragraph &&
          b.kind == MarkdownBlockKind.paragraph) {
        continue;
      }
      final markEnd = b.kind == MarkdownBlockKind.paragraph
          ? _skipBlank(text, b.start, b.end)
          : b.contentStart;
      if (text.substring(b.start, markEnd) == prefix) continue;
      edits.add(_Edit(b.start, markEnd - b.start, prefix, after: true));
    }
    return _apply(text, edits, s, e);
  }

  /// [text] with every item the selection from [start] to [end] touches
  /// one level in, or one level out with [out]; other lines stay as
  /// they are.
  static MarkdownChange indent(
    String text,
    int start,
    int end, {
    bool out = false,
  }) {
    final (s, e) = _ordered(text, start, end);
    final blocks = parse(text);
    final (first, last) = _touched(text, blocks, s, e);
    final edits = <_Edit>[
      for (final b in blocks.sublist(first, last + 1))
        if (_listed(b.kind) && b.level == (out ? 1 : 0))
          _Edit(
            b.start,
            _skipBlank(text, b.start, b.end) - b.start,
            out ? '' : '  ',
            after: !out,
          ),
    ];
    return _apply(text, edits, s, e);
  }

  /// Whether a shortcut ends at the UTF-16 offset [cursor] of [text]: a
  /// line's mark with its space typed just before it, such as "# ",
  /// "- " or "1. ", or the closing mark of bold, italic,
  /// strike-through, code or a link. A plain text becomes formatted
  /// when one is typed.
  static bool shortcutAt(String text, int cursor) {
    final at = cursor.clamp(0, text.length);
    final block = parse(text)[TextMarks.lineAt(text, at)];
    if (block.kind != MarkdownBlockKind.paragraph &&
        at == block.contentStart &&
        at > block.start &&
        _blank(text.codeUnitAt(at - 1))) {
      return true;
    }
    bool ends(List<MarkdownSpan> spans) => spans.any(
      (s) =>
          (s.kind != MarkdownSpanKind.text && s.end == at) || ends(s.children),
    );
    return ends(block.spans);
  }

  /// The status mark at the start of a table's cell from [start] to
  /// [end], as `TextMarks` reads one at the start of a line, or null.
  static (TextMark, int, int)? _cellMark(String text, int start, int end) {
    if (start >= end) return null;
    final c = text.codeUnitAt(start);
    for (final mark in TextMark.values) {
      if (mark.glyph.codeUnitAt(0) != c) continue;
      var stop = start + 1;
      if (stop < end) {
        final next = text.codeUnitAt(stop);
        if (next == 0xFE0E || next == 0xFE0F) stop++;
      }
      if (stop < end && text.codeUnitAt(stop) == 0x200D) return null;
      return (mark, start, stop);
    }
    return null;
  }

  static const _inline = {
    MarkdownSpanKind.bold,
    MarkdownSpanKind.italic,
    MarkdownSpanKind.strike,
    MarkdownSpanKind.code,
  };

  static const _lf = 0x0A;
  static const _cr = 0x0D;
  static const _space = 0x20;
  static const _tab = 0x09;
  static const _hash = 0x23;
  static const _backslash = 0x5C;
  static const _backtick = 0x60;
  static const _star = 0x2A;
  static const _underscore = 0x5F;
  static const _tilde = 0x7E;

  static final _alphabetic = RegExp(r'^\p{Alphabetic}$', unicode: true);
  static final _number = RegExp(r'^\p{N}$', unicode: true);

  /// The lines of [text] as offsets, as `TextMarks` splits them.
  static List<(int, int)> _lines(String text) {
    final lines = <(int, int)>[];
    var start = 0;
    var i = 0;
    while (i < text.length) {
      final c = text.codeUnitAt(i);
      if (c != _lf && c != _cr) {
        i++;
        continue;
      }
      lines.add((start, i));
      final pair =
          c == _cr && i + 1 < text.length && text.codeUnitAt(i + 1) == _lf;
      i += pair ? 2 : 1;
      start = i;
    }
    lines.add((start, text.length));
    return lines;
  }

  static bool _blank(int c) => c == _space || c == _tab;

  static int _skipBlank(String text, int at, int end) {
    while (at < end && _blank(text.codeUnitAt(at))) {
      at++;
    }
    return at;
  }

  /// A line's kind, level, the number written for a numbered item and
  /// where its content starts.
  static (MarkdownBlockKind, int, int, int) _block(
    String text,
    int start,
    int end,
  ) {
    var at = start;
    var width = 0;
    while (at < end && _blank(text.codeUnitAt(at))) {
      width += text.codeUnitAt(at) == _tab ? 4 : 1;
      at++;
    }
    bool endsMark(int i) => i == end || _blank(text.codeUnitAt(i));
    if (at < end && width <= 3) {
      final c = text.codeUnitAt(at);
      if (c == _hash) {
        var hashes = 1;
        while (at + hashes < end && text.codeUnitAt(at + hashes) == _hash) {
          hashes++;
        }
        if (hashes <= 2 && endsMark(at + hashes)) {
          final kind = hashes == 1
              ? MarkdownBlockKind.heading1
              : MarkdownBlockKind.heading2;
          return (kind, 0, 0, _skipBlank(text, at + hashes, end));
        }
      }
      if (c == 0x3E) {
        final content = at + 1 < end && _blank(text.codeUnitAt(at + 1))
            ? at + 2
            : at + 1;
        return (MarkdownBlockKind.quote, 0, 0, content);
      }
    }
    final level = width >= 2 ? 1 : 0;
    if (at < end && '-*+'.contains(text[at]) && endsMark(at + 1)) {
      final content = _skipBlank(text, at + 1, end);
      return (MarkdownBlockKind.bullet, level, 0, content);
    }
    var digits = 0;
    while (at + digits < end && _digit(text.codeUnitAt(at + digits))) {
      digits++;
    }
    if (digits >= 1 &&
        digits <= 9 &&
        at + digits < end &&
        text.codeUnitAt(at + digits) == 0x2E &&
        endsMark(at + digits + 1)) {
      final number = int.parse(text.substring(at, at + digits));
      final content = _skipBlank(text, at + digits + 1, end);
      return (MarkdownBlockKind.numbered, level, number, content);
    }
    return (MarkdownBlockKind.paragraph, 0, 0, start);
  }

  static bool _digit(int c) => c >= 0x30 && c <= 0x39;

  static bool _listed(MarkdownBlockKind kind) =>
      kind == MarkdownBlockKind.bullet || kind == MarkdownBlockKind.numbered;

  /// The number a numbered item of [level] written as [written] shows
  /// after [blocks].
  static int _shown(List<MarkdownBlock> blocks, int level, int written) {
    final after = _numberAfter(blocks, blocks.length, level);
    return after == 0 ? written : after;
  }

  /// One more than the number of the numbered item of [level] above the
  /// line [line], only items further in between; 0 when there is none.
  static int _numberAfter(List<MarkdownBlock> blocks, int line, int level) {
    for (var i = line - 1; i >= 0; i--) {
      final b = blocks[i];
      if (_listed(b.kind) && b.level > level) continue;
      if (b.kind == MarkdownBlockKind.numbered && b.level == level) {
        return b.number + 1;
      }
      return 0;
    }
    return 0;
  }

  /// Whether [c] counts as space for the flanking rules.
  static bool _white(int c) =>
      c == _space ||
      c == _tab ||
      c == 0x0C ||
      c == 0xA0 ||
      c == 0x1680 ||
      (c >= 0x2000 && c <= 0x200A) ||
      c == 0x202F ||
      c == 0x205F ||
      c == 0x3000;

  static bool _punct(int c) =>
      (c >= 0x21 && c <= 0x2F) ||
      (c >= 0x3A && c <= 0x40) ||
      (c >= 0x5B && c <= 0x60) ||
      (c >= 0x7B && c <= 0x7E);

  static bool _needsEscape(String text, int i, int start, int end) {
    final c = text.codeUnitAt(i);
    switch (c) {
      case _backslash:
        return i + 1 < end && _punct(text.codeUnitAt(i + 1));
      case _star || _backtick || 0x5B || 0x7C:
        return true;
      case _tilde:
        return (i > start && text.codeUnitAt(i - 1) == _tilde) ||
            (i + 1 < end && text.codeUnitAt(i + 1) == _tilde);
      case _underscore:
        bool plain(int j) {
          final d = text.codeUnitAt(j);
          return !_white(d) && !_punct(d);
        }
        return !(i > start && i + 1 < end && plain(i - 1) && plain(i + 1));
      default:
        return false;
    }
  }

  static bool _high(int c) => c >= 0xD800 && c < 0xDC00;
  static bool _low(int c) => c >= 0xDC00 && c < 0xE000;

  /// The code point at the UTF-16 offset [i] of [text].
  static int _at(String text, int i) {
    final c = text.codeUnitAt(i);
    if (_high(c) && i + 1 < text.length && _low(text.codeUnitAt(i + 1))) {
      return 0x10000 + ((c - 0xD800) << 10) + (text.codeUnitAt(i + 1) - 0xDC00);
    }
    return c;
  }

  /// The code point before the UTF-16 offset [i] of [text], [i] above 0.
  static int _before(String text, int i) {
    final c = text.codeUnitAt(i - 1);
    if (_low(c) && i >= 2 && _high(text.codeUnitAt(i - 2))) {
      return _at(text, i - 2);
    }
    return c;
  }

  static bool _word(int c) {
    if (c < 0x80) {
      return (c | 0x20) >= 0x61 && (c | 0x20) <= 0x7A ||
          _digit(c) ||
          c == _underscore;
    }
    final s = String.fromCharCode(c);
    return _alphabetic.hasMatch(s) || _number.hasMatch(s);
  }

  static bool _joiner(int c) => c == 0x27 || c == 0x2019 || c == 0x2D;

  static (int, int) _ordered(String text, int start, int end) {
    final a = start.clamp(0, text.length);
    final b = end.clamp(0, text.length);
    return a <= b ? (a, b) : (b, a);
  }

  /// [start] and [end] moved in past the spaces at both ends.
  static (int, int) _trimmed(String text, int start, int end) {
    var s = start;
    var e = end;
    while (s < e && _white(text.codeUnitAt(s))) {
      s++;
    }
    while (e > s && _white(text.codeUnitAt(e - 1))) {
      e--;
    }
    return (s, e);
  }

  /// The first and the last line the selection from [s] to [e] touches.
  static (int, int) _touched(
    String text,
    List<MarkdownBlock> blocks,
    int s,
    int e,
  ) {
    final first = TextMarks.lineAt(text, s);
    var last = TextMarks.lineAt(text, e);
    if (last > first && e == blocks[last].start) last--;
    return (first, last);
  }

  /// The pieces of the selection from [s] to [e], one per line, without
  /// the spaces at their ends; blank ones left out.
  static List<(MarkdownBlock, int, int)> _pieces(
    String text,
    List<MarkdownBlock> blocks,
    int s,
    int e,
  ) {
    final (first, last) = _touched(text, blocks, s, e);
    return [
      for (final b in blocks.sublist(first, last + 1))
        if (_trimmed(text, s > b.start ? s : b.start, e < b.end ? e : b.end)
            case (final from, final to) when from < to)
          (b, from, to),
    ];
  }

  /// The innermost part of [kind] among [spans] that [test] holds for.
  static MarkdownSpan? _deepest(
    List<MarkdownSpan> spans,
    MarkdownSpanKind kind,
    bool Function(MarkdownSpan) test,
  ) {
    for (final span in spans) {
      if (_deepest(span.children, kind, test) case final inner?) return inner;
      if (span.kind == kind && test(span)) return span;
    }
    return null;
  }

  /// The marks [kind] puts around [content].
  static (String, String) _marks(MarkdownSpanKind kind, String content) {
    switch (kind) {
      case MarkdownSpanKind.bold:
        return ('**', '**');
      case MarkdownSpanKind.strike:
        return ('~~', '~~');
      case MarkdownSpanKind.code:
        var longest = 0;
        var run = 0;
        for (final c in content.codeUnits) {
          run = c == _backtick ? run + 1 : 0;
          if (run > longest) longest = run;
        }
        final ticks = '`' * (longest + 1);
        final pad = content.startsWith('`') || content.endsWith('`') ? ' ' : '';
        return ('$ticks$pad', '$pad$ticks');
      default:
        return ('*', '*');
    }
  }

  static List<_Edit> _wrap(
    String text,
    int from,
    int to,
    MarkdownSpanKind kind,
  ) {
    final (open, close) = _marks(kind, text.substring(from, to));
    return [_Edit(from, 0, open, after: true), _Edit(to, 0, close)];
  }

  static List<_Edit> _unwrap(MarkdownSpan span) => [
    _Edit(span.start, span.contentStart - span.start, ''),
    _Edit(span.contentEnd, span.end - span.contentEnd, ''),
  ];

  /// [text] with [edits] made, and the selection from [s] to [e] moved
  /// along.
  static MarkdownChange _apply(String text, List<_Edit> edits, int s, int e) {
    edits.sort((a, b) => a.at.compareTo(b.at));
    final out = StringBuffer();
    var at = 0;
    for (final edit in edits) {
      out
        ..write(text.substring(at, edit.at))
        ..write(edit.insert);
      at = edit.at + edit.remove;
    }
    out.write(text.substring(at));
    int moved(int p) {
      var shift = 0;
      for (final edit in edits) {
        if (p < edit.at) break;
        if (p > edit.at + edit.remove) {
          shift += edit.insert.length - edit.remove;
          continue;
        }
        return edit.at + shift + (edit.after ? edit.insert.length : 0);
      }
      return p + shift;
    }

    return (text: out.toString(), start: moved(s), end: moved(e));
  }

  /// [url] without the spaces and control characters at its ends.
  static String _trimmedUrl(String url) {
    bool blank(int c) => c <= _space || c == 0x7F || _white(c);
    var s = 0;
    var e = url.length;
    while (s < e && blank(url.codeUnitAt(s))) {
      s++;
    }
    while (e > s && blank(url.codeUnitAt(e - 1))) {
      e--;
    }
    return url.substring(s, e);
  }

  /// [url] as a link's destination.
  static String _destination(String url) {
    final out = StringBuffer();
    for (final c in url.runes) {
      if (c <= 0x20 || c == 0x7F || _white(c)) {
        for (final byte in _utf8(c)) {
          out.write('%${byte.toRadixString(16).toUpperCase().padLeft(2, '0')}');
        }
      } else if (c == 0x28 || c == 0x29 || c == _backslash) {
        out.write('\\${String.fromCharCode(c)}');
      } else {
        out.writeCharCode(c);
      }
    }
    return out.toString();
  }

  static List<int> _utf8(int c) {
    if (c < 0x80) return [c];
    if (c < 0x800) return [0xC0 | (c >> 6), 0x80 | (c & 0x3F)];
    return [0xE0 | (c >> 12), 0x80 | ((c >> 6) & 0x3F), 0x80 | (c & 0x3F)];
  }
}

/// An edit of a text: [remove] code units at [at] replaced by [insert];
/// an offset within it goes after [insert] when [after], else before.
class _Edit {
  final int at;
  final int remove;
  final String insert;
  final bool after;

  _Edit(this.at, this.remove, this.insert, {this.after = false});
}

/// A run of `*`, `_` or `~` that may open or close emphasis, as far as
/// it is not used yet: from [start] to [end].
class _Delim {
  final int char;
  final int length;
  final bool open;
  final bool close;
  int start;
  int end;

  _Delim(
    this.char,
    this.start,
    this.end, {
    required this.open,
    required this.close,
  }) : length = end - start;

  int get count => end - start;
}

/// The parts of the content of a line from [from] to [to].
class _Inline {
  final String text;
  final int from;
  final int to;
  final bool links;

  /// Whether it is a table's cell, where `\|` in code shows `|`, as GFM
  /// takes a cell's escaped pipes out before reading it.
  final bool cell;

  _Inline(
    this.text,
    this.from,
    this.to, {
    required this.links,
    this.cell = false,
  });

  List<MarkdownSpan> parse() {
    final items = <Object>[];
    var literal = -1;
    void flush(int p) {
      if (literal >= 0 && literal < p) items.add(_literal(literal, p));
      literal = -1;
    }

    // Where the last `!` shown as written is, before [from] when none.
    var bang = from - 2;
    var p = from;
    while (p < to) {
      final c = text.codeUnitAt(p);
      if (c == Markdown._backslash &&
          p + 1 < to &&
          Markdown._punct(text.codeUnitAt(p + 1))) {
        flush(p);
        items.add(
          MarkdownSpan(
            MarkdownSpanKind.text,
            start: p,
            end: p + 2,
            contentStart: p + 1,
            contentEnd: p + 2,
            text: text[p + 1],
          ),
        );
        p += 2;
        continue;
      }
      if (c == Markdown._backtick) {
        final n = _run(p);
        final close = _closing(p + n, n);
        if (close >= 0) {
          flush(p);
          items.add(_code(p, n, close));
          p = close + n;
        } else {
          if (literal < 0) literal = p;
          p += n;
        }
        continue;
      }
      if (c == 0x5B && links && bang != p - 1) {
        if (_link(p) case final link?) {
          flush(p);
          items.add(link);
          p = link.end;
          continue;
        }
      }
      if (c == Markdown._star ||
          c == Markdown._underscore ||
          c == Markdown._tilde) {
        final n = _run(p);
        if (c != Markdown._tilde || n == 2) {
          flush(p);
          items.add(_delim(c, p, p + n));
        } else if (literal < 0) {
          literal = p;
        }
        p += n;
        continue;
      }
      if (c == 0x21) bang = p;
      if (literal < 0) literal = p;
      p++;
    }
    flush(to);
    _emphasis(items);
    return _merged([for (final item in items) _span(item)]);
  }

  /// The length of the run of the character at [p].
  int _run(int p) {
    final c = text.codeUnitAt(p);
    var q = p;
    while (q < to && text.codeUnitAt(q) == c) {
      q++;
    }
    return q - p;
  }

  /// Where the next run of exactly [n] backticks from [p] on starts, or
  /// -1.
  int _closing(int p, int n) {
    var q = p;
    while (q < to) {
      if (text.codeUnitAt(q) != Markdown._backtick) {
        q++;
        continue;
      }
      final m = _run(q);
      if (m == n) return q;
      q += m;
    }
    return -1;
  }

  MarkdownSpan _code(int p, int n, int close) {
    var content = text.substring(p + n, close);
    if (cell) content = content.replaceAll(r'\|', '|');
    if (content.length >= 2 &&
        content.startsWith(' ') &&
        content.endsWith(' ') &&
        content.codeUnits.any((c) => c != Markdown._space)) {
      content = content.substring(1, content.length - 1);
    }
    return MarkdownSpan(
      MarkdownSpanKind.code,
      start: p,
      end: close + n,
      contentStart: p + n,
      contentEnd: close,
      text: content,
    );
  }

  /// The link at the `[` at [p], or null when there is none.
  MarkdownSpan? _link(int p) {
    var depth = 1;
    var q = p + 1;
    while (q < to) {
      final c = text.codeUnitAt(q);
      if (c == Markdown._backslash &&
          q + 1 < to &&
          Markdown._punct(text.codeUnitAt(q + 1))) {
        q += 2;
        continue;
      }
      if (c == Markdown._backtick) {
        final n = _run(q);
        final close = _closing(q + n, n);
        q = close >= 0 ? close + n : q + n;
        continue;
      }
      if (c == 0x5B) depth++;
      if (c == 0x5D && --depth == 0) break;
      q++;
    }
    if (q >= to) return null;
    final labelEnd = q;
    q++;
    if (q >= to || text.codeUnitAt(q) != 0x28) return null;
    q = Markdown._skipBlank(text, q + 1, to);
    final url = StringBuffer();
    var parens = 0;
    while (q < to) {
      final c = text.codeUnitAt(q);
      if (c == Markdown._backslash &&
          q + 1 < to &&
          Markdown._punct(text.codeUnitAt(q + 1))) {
        url.write(text[q + 1]);
        q += 2;
        continue;
      }
      if (Markdown._white(c)) break;
      if (c < 0x20 || c == 0x7F) return null;
      if (c == 0x28) parens++;
      if (c == 0x29) {
        if (parens == 0) break;
        parens--;
      }
      url.write(text[q]);
      q++;
    }
    if (parens != 0) return null;
    q = Markdown._skipBlank(text, q, to);
    if (q >= to || text.codeUnitAt(q) != 0x29) return null;
    return MarkdownSpan(
      MarkdownSpanKind.link,
      start: p,
      end: q + 1,
      contentStart: p + 1,
      contentEnd: labelEnd,
      url: url.toString(),
      children: _Inline(
        text,
        p + 1,
        labelEnd,
        links: false,
        cell: cell,
      ).parse(),
    );
  }

  _Delim _delim(int c, int start, int end) {
    final before = start == from ? Markdown._space : text.codeUnitAt(start - 1);
    final after = end == to ? Markdown._space : text.codeUnitAt(end);
    final left =
        !Markdown._white(after) &&
        (!Markdown._punct(after) ||
            Markdown._white(before) ||
            Markdown._punct(before));
    final right =
        !Markdown._white(before) &&
        (!Markdown._punct(before) ||
            Markdown._white(after) ||
            Markdown._punct(after));
    if (c == Markdown._underscore) {
      return _Delim(
        c,
        start,
        end,
        open: left && (!right || Markdown._punct(before)),
        close: right && (!left || Markdown._punct(after)),
      );
    }
    return _Delim(c, start, end, open: left, close: right);
  }

  /// Matches openers and closers, as CommonMark's emphasis does: each
  /// closer, left to right, with the nearest opener of its character
  /// before it, two of each for bold when both have two, a pair whose
  /// lengths add up to a multiple of three left alone when one of them
  /// can both open and close, unless both are multiples of three.
  void _emphasis(List<Object> items) {
    var c = 0;
    while (c < items.length) {
      final closer = items[c];
      if (closer is! _Delim || !closer.close || closer.count == 0) {
        c++;
        continue;
      }
      var o = c - 1;
      for (; o >= 0; o--) {
        final d = items[o];
        if (d is _Delim &&
            d.char == closer.char &&
            d.open &&
            d.count > 0 &&
            !_third(d, closer)) {
          break;
        }
      }
      if (o < 0) {
        c++;
        continue;
      }
      final opener = items[o] as _Delim;
      final use =
          closer.char == Markdown._tilde ||
              (opener.count >= 2 && closer.count >= 2)
          ? 2
          : 1;
      final kind = closer.char == Markdown._tilde
          ? MarkdownSpanKind.strike
          : use == 2
          ? MarkdownSpanKind.bold
          : MarkdownSpanKind.italic;
      final span = MarkdownSpan(
        kind,
        start: opener.end - use,
        end: closer.start + use,
        contentStart: opener.end,
        contentEnd: closer.start,
        children: _merged([
          for (final item in items.sublist(o + 1, c)) _span(item),
        ]),
      );
      opener.end -= use;
      closer.start += use;
      items.replaceRange(o + 1, c, [span]);
      c = o + 2;
      if (opener.count == 0) {
        items.removeAt(o);
        c--;
      }
      if (closer.count == 0) items.removeAt(c);
    }
  }

  static bool _third(_Delim opener, _Delim closer) {
    if (opener.char == Markdown._tilde) return false;
    final sum = opener.length + closer.length;
    return (opener.close || closer.open) &&
        sum % 3 == 0 &&
        !(opener.length % 3 == 0 && closer.length % 3 == 0);
  }

  MarkdownSpan _literal(int start, int end) => MarkdownSpan(
    MarkdownSpanKind.text,
    start: start,
    end: end,
    contentStart: start,
    contentEnd: end,
    text: text.substring(start, end),
  );

  MarkdownSpan _span(Object item) =>
      item is _Delim ? _literal(item.start, item.end) : item as MarkdownSpan;

  /// [spans] with neighbouring runs of characters shown as written
  /// joined into one.
  List<MarkdownSpan> _merged(List<MarkdownSpan> spans) {
    final out = <MarkdownSpan>[];
    for (final span in spans) {
      if (out.isNotEmpty &&
          _written(out.last) &&
          _written(span) &&
          out.last.end == span.start) {
        out.last = _literal(out.last.start, span.end);
      } else {
        out.add(span);
      }
    }
    return out;
  }

  static bool _written(MarkdownSpan span) =>
      span.kind == MarkdownSpanKind.text && span.contentStart == span.start;
}
