part of 'markdown.dart';

/// How a column of a table lines up its cells, as its cell of the
/// delimiter row says: `---` none, `:--` left, `:-:` center, `--:` right.
enum MarkdownAlign { none, left, center, right }

/// A cell of a table: where its content lies in the text, from [start]
/// to [end] in UTF-16 code units, without the spaces and tabs around it,
/// and its parts, one line of bold, italic, strike-through, code and
/// links. A cell a short row lacks is empty, both offsets at the end of
/// its row.
class MarkdownTableCell {
  final int start;
  final int end;
  final List<MarkdownSpan> spans;

  const MarkdownTableCell({
    required this.start,
    required this.end,
    required this.spans,
  });

  /// The text this cell shows, without its marks.
  String get visible => spans.map((s) => s.visible).join();
}

/// A row of a table: the [line] it is, where it lies in the text, from
/// [start] to [end] without its line break, and its [cells], one per
/// column of the table.
class MarkdownTableRow {
  final int line;
  final int start;
  final int end;
  final List<MarkdownTableCell> cells;

  /// The cells written beyond the table's columns, from where to where:
  /// shown nowhere, as GFM has it, and kept when the table is written
  /// anew.
  final List<(int, int)> _beyond;

  const MarkdownTableRow._(
    this._beyond, {
    required this.line,
    required this.start,
    required this.end,
    required this.cells,
  });

  /// What the row reads as: the cells that show something, a space
  /// between them.
  String get visible => [
    for (final cell in cells)
      if (cell.visible case final shown when shown.isNotEmpty) shown,
  ].join(' ');
}

/// A table (GFM) of a formatted text: where it lies, from [start] to
/// [end] in UTF-16 code units; its lines, from [firstLine] to
/// [lastLine], the line saying its first column holds headings first
/// when [headingColumn]; the [delimiterLine] under the header row; how
/// each column lines up its cells ([aligns]); and its [rows], the header
/// row first.
class MarkdownTable {
  final int start;
  final int end;
  final int firstLine;
  final int delimiterLine;
  final int lastLine;

  /// Whether the first column holds headings, said by the line
  /// [MarkdownTables.headingColumnLine] right before the header row.
  final bool headingColumn;
  final List<MarkdownAlign> aligns;
  final List<MarkdownTableRow> rows;

  const MarkdownTable._({
    required this.start,
    required this.end,
    required this.firstLine,
    required this.delimiterLine,
    required this.lastLine,
    required this.headingColumn,
    required this.aligns,
    required this.rows,
  });

  int get columns => aligns.length;
}

/// Reading the tables of a formatted text and the edits the table editor
/// makes. Every edit writes the whole table anew in one form, every other
/// character of the text kept: each row on a line of its own between
/// pipes, `| a | b |`, the delimiter row `| --- | :-: |`, a cell's
/// content as it was written, and the line break that followed the
/// header row. The change it gives selects the table in the new text, so
/// [at] finds it again; an edit that cannot be made gives the text as it
/// was and its cursor where it was asked for.
abstract final class MarkdownTables {
  /// The line right before a header row that marks the first column of
  /// its table as headings. GFM has no such flag; a comment shows nowhere
  /// in a Markdown viewer and reads as what it says where the source
  /// shows.
  static const headingColumnLine = '<!-- first column headings -->';

  /// Every table of [text], top first.
  static List<MarkdownTable> all(String text) => [
    for (final block in Markdown.parse(text))
      if (block.table case final table? when block.line == table.firstLine)
        table,
  ];

  /// The table the line of the UTF-16 offset [offset] belongs to, or
  /// null.
  static MarkdownTable? at(String text, int offset) =>
      Markdown.parse(text)[Markdown.lineAt(text, offset)].table;

  /// What the table editor edits of the cell in [row] and [column] of the
  /// table at [at]: its content as written, `\|` read as `|`. Null when
  /// there is no such table or cell.
  static String? cell(String text, int at, int row, int column) {
    final table = MarkdownTables.at(text, at);
    if (table == null ||
        row < 0 ||
        row >= table.rows.length ||
        column < 0 ||
        column >= table.columns) {
      return null;
    }
    final cell = table.rows[row].cells[column];
    return text.substring(cell.start, cell.end).replaceAll(r'\|', '|');
  }

  /// [text] with the cell in [row] and [column] of the table at [at]
  /// holding [cell], as [MarkdownTables.cell] reads it: each line break
  /// becomes a space and each `|` is written `\|`, so the cell stays one
  /// cell of one line; the spaces and tabs at its ends go.
  static MarkdownChange setCell(
    String text,
    int at,
    int row,
    int column,
    String cell,
  ) => _change(text, at, (grid) {
    if (row < 0 ||
        row >= grid.rows.length ||
        column < 0 ||
        column >= grid.aligns.length) {
      return false;
    }
    final flat = cell
        .replaceAll('\r\n', ' ')
        .replaceAll('\r', ' ')
        .replaceAll('\n', ' ');
    final from = Markdown._skipBlank(flat, 0, flat.length);
    var to = flat.length;
    while (to > from && Markdown._blank(flat.codeUnitAt(to - 1))) {
      to--;
    }
    grid.rows[row][column] = flat.substring(from, to).replaceAll('|', r'\|');
    return true;
  });

  /// [text] with an empty row put in at [index] of the rows of the table
  /// at [at]: 0 makes it the header row, the rows' count puts it last.
  static MarkdownChange insertRow(String text, int at, int index) =>
      _change(text, at, (grid) {
        if (index < 0 || index > grid.rows.length) return false;
        grid.rows.insert(index, List.filled(grid.aligns.length, ''));
        return true;
      });

  /// [text] with an empty column put in at [index] of the columns of the
  /// table at [at], lining up as GFM does by default.
  static MarkdownChange insertColumn(String text, int at, int index) =>
      _change(text, at, (grid) {
        if (index < 0 || index > grid.aligns.length) return false;
        for (final cells in grid.rows) {
          cells.insert(index, '');
        }
        grid.aligns.insert(index, MarkdownAlign.none);
        return true;
      });

  /// [text] with the row [from] of the table at [at] moved to [to]; a row
  /// moved to 0 becomes the header row.
  static MarkdownChange moveRow(String text, int at, int from, int to) =>
      _change(text, at, (grid) {
        final count = grid.rows.length;
        if (from < 0 || from >= count || to < 0 || to >= count || from == to) {
          return false;
        }
        grid.rows.insert(to, grid.rows.removeAt(from));
        return true;
      });

  /// [text] with the column [from] of the table at [at] moved to [to],
  /// its alignment with it.
  static MarkdownChange moveColumn(String text, int at, int from, int to) =>
      _change(text, at, (grid) {
        final count = grid.aligns.length;
        if (from < 0 || from >= count || to < 0 || to >= count || from == to) {
          return false;
        }
        for (final cells in grid.rows) {
          cells.insert(to, cells.removeAt(from));
        }
        grid.aligns.insert(to, grid.aligns.removeAt(from));
        return true;
      });

  /// [text] with the row [row] of the table at [at] deleted; the header
  /// row's going makes the next row the header row, and the last row's
  /// going takes the table with it.
  static MarkdownChange deleteRow(String text, int at, int row) =>
      _change(text, at, (grid) {
        if (row < 0 || row >= grid.rows.length) return false;
        grid.rows.removeAt(row);
        return true;
      });

  /// [text] with the column [column] of the table at [at] deleted; the
  /// last column's going takes the table with it.
  static MarkdownChange deleteColumn(String text, int at, int column) =>
      _change(text, at, (grid) {
        if (column < 0 || column >= grid.aligns.length) return false;
        for (final cells in grid.rows) {
          cells.removeAt(column);
        }
        grid.aligns.removeAt(column);
        return true;
      });

  /// [text] with its table at [at] marking its first column as headings,
  /// or no longer when it does.
  static MarkdownChange toggleHeadingColumn(String text, int at) =>
      _change(text, at, (grid) {
        grid.headings = !grid.headings;
        return true;
      });

  /// [text] with a new empty table of two columns, a header row and one
  /// more row, on lines of its own, a blank line between it and the
  /// lines around it: at a blank line in place of it, at the start of a
  /// line before that line, anywhere else in a line after that line, and
  /// in a table after that table. It takes the text's first kind of line
  /// break, `\n` when there is none.
  static MarkdownChange insert(String text, int at) {
    final offset = at.clamp(0, text.length);
    final blocks = Markdown.parse(text);
    final block = blocks[Markdown.lineAt(text, offset)];
    final br = _firstBreak(text);
    final table = _Grid(
      [
        ['', ''],
        ['', ''],
      ],
      [MarkdownAlign.none, MarkdownAlign.none],
      headings: false,
    ).write(br);
    bool blank(int line) =>
        line >= 0 &&
        line < blocks.length &&
        Markdown._skipBlank(text, blocks[line].start, blocks[line].end) ==
            blocks[line].end;
    final String head;
    final String tail;
    final String before;
    final String after;
    final line = block.table?.lastLine ?? block.line;
    if (block.table == null && blank(line)) {
      head = text.substring(0, block.start);
      tail = text.substring(block.end);
      before = line > 0 && !blank(line - 1) ? br : '';
      after = line + 1 < blocks.length && !blank(line + 1) ? br : '';
    } else if (block.table == null &&
        offset <= Markdown._skipBlank(text, block.start, block.end)) {
      head = text.substring(0, block.start);
      tail = text.substring(block.start);
      before = line > 0 && !blank(line - 1) ? br : '';
      after = '$br$br';
    } else {
      final end = blocks[line].end;
      head = text.substring(0, end);
      tail = text.substring(end);
      before = '$br$br';
      after = line + 1 < blocks.length && !blank(line + 1) ? br : '';
    }
    final start = head.length + before.length;
    return (
      text: '$head$before$table$after$tail',
      start: start,
      end: start + table.length,
    );
  }

  /// [text] with the table at [at] edited by [edit], or as it was when
  /// [edit] says it cannot be made.
  static MarkdownChange _change(
    String text,
    int at,
    bool Function(_Grid grid) edit,
  ) {
    final offset = at.clamp(0, text.length);
    final table = MarkdownTables.at(text, offset);
    if (table == null) return (text: text, start: offset, end: offset);
    final grid = _Grid.of(text, table);
    if (!edit(grid)) return (text: text, start: offset, end: offset);
    if (grid.rows.isEmpty || grid.aligns.isEmpty) {
      var start = table.start;
      var end = table.end;
      if (end < text.length) {
        end += _breakAt(text, end).length;
      } else if (start > 0) {
        start -=
            text.codeUnitAt(start - 1) == Markdown._lf &&
                start >= 2 &&
                text.codeUnitAt(start - 2) == Markdown._cr
            ? 2
            : 1;
      }
      return (
        text: text.replaceRange(start, end, ''),
        start: start,
        end: start,
      );
    }
    final written = grid.write(_breakAt(text, table.rows.first.end));
    return (
      text: text.replaceRange(table.start, table.end, written),
      start: table.start,
      end: table.start + written.length,
    );
  }

  /// The line break at [at] of [text].
  static String _breakAt(String text, int at) {
    if (at >= text.length) return '\n';
    if (text.codeUnitAt(at) == Markdown._cr) {
      return at + 1 < text.length && text.codeUnitAt(at + 1) == Markdown._lf
          ? '\r\n'
          : '\r';
    }
    return '\n';
  }

  static String _firstBreak(String text) {
    for (var i = 0; i < text.length; i++) {
      final c = text.codeUnitAt(i);
      if (c == Markdown._lf || c == Markdown._cr) return _breakAt(text, i);
    }
    return '\n';
  }
}

/// A table as the edits change it: the content of each cell as written,
/// each row as long as the columns and with the cells written beyond
/// them after, the columns' alignment, and whether the first column holds
/// headings.
class _Grid {
  final List<List<String>> rows;
  final List<MarkdownAlign> aligns;
  bool headings;

  _Grid(this.rows, this.aligns, {required this.headings});

  factory _Grid.of(String text, MarkdownTable table) => _Grid(
    [
      for (final row in table.rows)
        [
          for (final cell in row.cells) text.substring(cell.start, cell.end),
          for (final (start, end) in row._beyond) text.substring(start, end),
        ],
    ],
    [...table.aligns],
    headings: table.headingColumn,
  );

  /// The table's lines, [br] between them.
  String write(String br) {
    String line(List<String> cells) => '| ${cells.join(' | ')} |';
    return [
      if (headings) MarkdownTables.headingColumnLine,
      line(rows.first),
      line([
        for (final align in aligns)
          switch (align) {
            MarkdownAlign.none => '---',
            MarkdownAlign.left => ':--',
            MarkdownAlign.center => ':-:',
            MarkdownAlign.right => '--:',
          },
      ]),
      for (final row in rows.skip(1)) line(row),
    ].join(br);
  }
}

const _pipe = 0x7C;
const _colon = 0x3A;
const _dash = 0x2D;

/// The table whose first line is the line [first] of [lines], or null.
///
/// A table is a header row, the delimiter row right under it with as
/// many cells, and every line under those that is a row too; before its
/// header row may come [MarkdownTables.headingColumnLine]. A row is a
/// line of at least one cell that would be a paragraph line, indented by
/// at most three, not blank, no comment (`<!--`), no fence of three
/// backticks or tildes and no rule (three `-`, `*` or `_`). The
/// delimiter row holds a `|` and cells of `-`, a `:` at either end
/// telling the alignment.
MarkdownTable? _tableAt(String text, List<(int, int)> lines, int first) {
  final flagged = _isHeadingColumnLine(text, lines[first]);
  final header = flagged ? first + 1 : first;
  if (header + 1 >= lines.length) return null;
  final (hs, he) = lines[header];
  if (!_isRow(text, hs, he)) return null;
  final heads = _cells(text, hs, he);
  final aligns = _delimiter(text, lines[header + 1]);
  if (aligns == null || aligns.length != heads.length) return null;
  var last = header + 1;
  while (last + 1 < lines.length &&
      _isRow(text, lines[last + 1].$1, lines[last + 1].$2)) {
    last++;
  }
  MarkdownTableRow row(int line) {
    final (start, end) = lines[line];
    final found = _cells(text, start, end);
    return MarkdownTableRow._(
      found.skip(aligns.length).toList(),
      line: line,
      start: start,
      end: end,
      cells: [
        for (var i = 0; i < aligns.length; i++)
          if (i < found.length)
            MarkdownTableCell(
              start: found[i].$1,
              end: found[i].$2,
              spans: _Inline(
                text,
                found[i].$1,
                found[i].$2,
                links: true,
                cell: true,
              ).parse(),
            )
          else
            MarkdownTableCell(start: end, end: end, spans: const []),
      ],
    );
  }

  return MarkdownTable._(
    start: lines[first].$1,
    end: lines[last].$2,
    firstLine: first,
    delimiterLine: header + 1,
    lastLine: last,
    headingColumn: flagged,
    aligns: aligns,
    rows: [row(header), for (var l = header + 2; l <= last; l++) row(l)],
  );
}

bool _isHeadingColumnLine(String text, (int, int) line) {
  final (start, end) = line;
  final from = Markdown._skipBlank(text, start, end);
  var to = end;
  while (to > from && Markdown._blank(text.codeUnitAt(to - 1))) {
    to--;
  }
  return text.substring(from, to) == MarkdownTables.headingColumnLine;
}

/// Whether the line from [start] to [end] may be a row of a table.
bool _isRow(String text, int start, int end) {
  var at = start;
  var width = 0;
  while (at < end && Markdown._blank(text.codeUnitAt(at))) {
    width += text.codeUnitAt(at) == Markdown._tab ? 4 : 1;
    at++;
  }
  if (at == end || width > 3) return false;
  if (Markdown._block(text, start, end).$1 != MarkdownBlockKind.paragraph) {
    return false;
  }
  if (text.startsWith('<!--', at)) return false;
  final c = text.codeUnitAt(at);
  if ((c == Markdown._backtick || c == Markdown._tilde) &&
      text.startsWith(String.fromCharCodes([c, c, c]), at)) {
    return false;
  }
  if (c == _dash || c == Markdown._star || c == Markdown._underscore) {
    var count = 0;
    var rule = true;
    for (var i = at; i < end && rule; i++) {
      final d = text.codeUnitAt(i);
      if (d == c) {
        count++;
      } else if (!Markdown._blank(d)) {
        rule = false;
      }
    }
    if (rule && count >= 3) return false;
  }
  return _cells(text, start, end).isNotEmpty;
}

/// The cells of the line from [start] to [end], from where to where
/// without the spaces and tabs around them: split at each `|` that does
/// not follow a `\`, a `|` at the line's start or end opening or closing
/// the row, not a cell.
List<(int, int)> _cells(String text, int start, int end) {
  final cells = <(int, int)>[];
  var at = Markdown._skipBlank(text, start, end);
  if (at < end && text.codeUnitAt(at) == _pipe) {
    at = Markdown._skipBlank(text, at + 1, end);
  }
  while (at < end) {
    var stop = at;
    while (stop < end &&
        !(text.codeUnitAt(stop) == _pipe &&
            !(stop > start &&
                text.codeUnitAt(stop - 1) == Markdown._backslash))) {
      stop++;
    }
    var to = stop;
    while (to > at && Markdown._blank(text.codeUnitAt(to - 1))) {
      to--;
    }
    cells.add((at, to));
    if (stop >= end) break;
    at = Markdown._skipBlank(text, stop + 1, end);
  }
  return cells;
}

/// How the columns of the delimiter row [line] line up, or null when it
/// is none.
List<MarkdownAlign>? _delimiter(String text, (int, int) line) {
  final (start, end) = line;
  if (!_isRow(text, start, end) || !text.substring(start, end).contains('|')) {
    return null;
  }
  final aligns = <MarkdownAlign>[];
  for (final (from, to) in _cells(text, start, end)) {
    var s = from;
    var e = to;
    final left = s < e && text.codeUnitAt(s) == _colon;
    if (left) s++;
    final right = e > s && text.codeUnitAt(e - 1) == _colon;
    if (right) e--;
    if (s >= e) return null;
    for (var i = s; i < e; i++) {
      if (text.codeUnitAt(i) != _dash) return null;
    }
    aligns.add(switch ((left, right)) {
      (true, true) => MarkdownAlign.center,
      (true, false) => MarkdownAlign.left,
      (false, true) => MarkdownAlign.right,
      (false, false) => MarkdownAlign.none,
    });
  }
  return aligns;
}
