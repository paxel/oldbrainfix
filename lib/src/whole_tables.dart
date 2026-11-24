import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A part of a formatted text that a table moves past as a whole
/// (obf-1050): its lines from [first] to [last] and where it lies, from
/// [start] to [end] without the line break after it, and the [kind] of
/// its first line.
typedef TextBlock = ({
  int first,
  int last,
  int start,
  int end,
  MarkdownBlockKind kind,
});

/// A table of a formatted text deleted or moved as a whole from its ⋮
/// (obf-1050), every other character of the text kept.
///
/// The core reads every line as a block of its own ([Markdown.parse]);
/// a table moves past the block above or below it, and a block is:
///
/// - a table, its lines from the first to the last, the line marking its
///   first column as headings included;
/// - a fenced code block, from its opening fence to its closing one;
/// - a heading, its one line;
/// - a list, the bulleted and numbered items one right under the other;
/// - a quote, its lines one right under the other;
/// - a paragraph, the lines of words one right under the other.
///
/// A blank line, nothing on it but spaces and tabs, is no block: it
/// parts two of them and stays where it is.
abstract final class WholeTables {
  /// The blocks of [text], top first.
  static List<TextBlock> blocks(String text) {
    final lines = Markdown.parse(text);
    final blocks = <TextBlock>[];
    bool blank(MarkdownBlock line) =>
        line.kind == MarkdownBlockKind.paragraph &&
        text.substring(line.start, line.end).trim().isEmpty;
    bool joins(MarkdownBlock line, MarkdownBlockKind kind) =>
        line.table == null &&
        line.code == null &&
        !blank(line) &&
        switch (kind) {
          MarkdownBlockKind.bullet || MarkdownBlockKind.numbered =>
            line.kind == MarkdownBlockKind.bullet ||
                line.kind == MarkdownBlockKind.numbered,
          MarkdownBlockKind.quote ||
          MarkdownBlockKind.paragraph => line.kind == kind,
          _ => false,
        };
    var i = 0;
    while (i < lines.length) {
      final line = lines[i];
      if (blank(line)) {
        i++;
        continue;
      }
      var last = line.table?.lastLine ?? line.code?.lastLine ?? i;
      while (last + 1 < lines.length && joins(lines[last + 1], line.kind)) {
        last++;
      }
      blocks.add((
        first: i,
        last: last,
        start: line.start,
        end: lines[last].end,
        kind: line.kind,
      ));
      i = last + 1;
    }
    return blocks;
  }

  /// Whether the table at [at] of [text] has a block above it or, not
  /// [up], below it, to move past.
  static bool canMove(String text, int at, {required bool up}) =>
      _neighbours(text, at, up: up) != null;

  /// [text] with the table at [at] past the block above it or, not [up],
  /// below it; null where there is no table or no such block. The text
  /// between the two stays as it is, the blank lines and line breaks;
  /// where a table would stand right above a paragraph or another table,
  /// which would read as its rows, a blank line goes between them. The
  /// change selects the table where it lies then.
  static MarkdownChange? move(String text, int at, {required bool up}) {
    final found = _neighbours(text, at, up: up);
    if (found == null) return null;
    final (:table, :other, :before, :after) = found;
    final (above, below) = up ? (other, table) : (table, other);
    String gap(TextBlock? top, TextBlock? bottom, String between) {
      if (top == null || bottom == null) return between;
      final touches =
          top.kind == MarkdownBlockKind.table &&
          (bottom.kind == MarkdownBlockKind.paragraph ||
              bottom.kind == MarkdownBlockKind.table);
      // One line break and nothing else: no blank line between them.
      return touches && _breaks(between) == 1 ? '$between$between' : between;
    }

    final middle = text.substring(above.end, below.start);
    final tableText = text.substring(table.start, table.end);
    final otherText = text.substring(other.start, other.end);
    if (up) {
      // The block before the other one, the table, the other one.
      final cut = before?.end ?? above.start;
      final head = text.substring(0, cut);
      final lead = gap(before, table, text.substring(cut, above.start));
      final start = head.length + lead.length;
      final between = gap(table, other, middle);
      final cutAfter = after?.start ?? below.end;
      final trail = gap(other, after, text.substring(below.end, cutAfter));
      final tail = text.substring(cutAfter);
      return (
        text: '$head$lead$tableText$between$otherText$trail$tail',
        start: start,
        end: start + tableText.length,
      );
    }
    // The other block, the table, the block after the other one.
    final cutBefore = before?.end ?? above.start;
    final head = text.substring(0, cutBefore);
    final lead = gap(before, other, text.substring(cutBefore, above.start));
    final cut = after?.start ?? below.end;
    final between = gap(other, table, middle);
    final trail = gap(table, after, text.substring(below.end, cut));
    final start = head.length + lead.length + otherText.length + between.length;
    return (
      text:
          '$head$lead$otherText$between$tableText$trail${text.substring(cut)}',
      start: start,
      end: start + tableText.length,
    );
  }

  /// [text] without the table at [at] and the line break after it, or,
  /// last in the text, the one before it, as deleting its last row does;
  /// null where there is none. The cursor is where it was.
  static MarkdownChange? delete(String text, int at) {
    final table = MarkdownTables.at(text, at.clamp(0, text.length));
    if (table == null) return null;
    var start = table.start;
    var end = table.end;
    if (end < text.length) {
      end += text.startsWith('\r\n', end) ? 2 : 1;
    } else if (start > 0) {
      start -=
          text.codeUnitAt(start - 1) == 0x0A &&
              start >= 2 &&
              text.codeUnitAt(start - 2) == 0x0D
          ? 2
          : 1;
    }
    return (text: text.replaceRange(start, end, ''), start: start, end: start);
  }

  /// The table at [at] of [text] as a block, the block it moves past
  /// ([other]), and the blocks right before and after the two, if any.
  static ({
    TextBlock table,
    TextBlock other,
    TextBlock? before,
    TextBlock? after,
  })?
  _neighbours(String text, int at, {required bool up}) {
    final table = MarkdownTables.at(text, at.clamp(0, text.length));
    if (table == null) return null;
    final blocks = WholeTables.blocks(text);
    final index = blocks.indexWhere((b) => b.first == table.firstLine);
    final other = up ? index - 1 : index + 1;
    if (index < 0 || other < 0 || other >= blocks.length) return null;
    final (top, bottom) = up ? (other, index) : (index, other);
    return (
      table: blocks[index],
      other: blocks[other],
      before: top > 0 ? blocks[top - 1] : null,
      after: blocks.elementAtOrNull(bottom + 1),
    );
  }

  /// How many line breaks [text] holds, `\r\n` counting one.
  static int _breaks(String text) =>
      RegExp(r'\r\n|\r|\n').allMatches(text).length;
}
