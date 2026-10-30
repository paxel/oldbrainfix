//! Tables (GFM, #128) of a formatted text, as the Dart core's
//! `markdown_table.dart` reads and edits them: a header row, the
//! delimiter row under it with as many cells, and every line under those
//! that is a row too, each row a line of its own. Offsets are bytes.
//!
//! Every edit writes the whole table anew in one form, every other byte
//! of the text kept: each row on a line of its own between pipes,
//! `| a | b |`, the delimiter row `| --- | :-: |`, a cell's content as it
//! was written, and the line break that followed the header row. The
//! change it gives selects the table in the new text, so [`at`] finds it
//! again; an edit that cannot be made gives the text as it was and its
//! cursor where it was asked for.

use std::sync::Arc;

use super::{Block, BlockKind, Change, Inline, Span, blank, floor, line_at, skip_blank, slice};

/// The line right before a header row that marks the first column of
/// its table as headings. GFM has no such flag; a comment shows nowhere
/// in a Markdown viewer and reads as what it says where the source shows.
pub const HEADING_COLUMN_LINE: &str = "<!-- first column headings -->";

/// How a column of a table lines up its cells, as its cell of the
/// delimiter row says: `---` none, `:--` left, `:-:` center, `--:` right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

impl Align {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            Align::None => "none",
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
        }
    }
}

/// A cell of a table: where its content lies in the text, `start..end`,
/// without the spaces and tabs around it, and its parts, one line of
/// bold, italic, strike-through, code and links. A cell a short row
/// lacks is empty, both offsets at the end of its row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub start: usize,
    pub end: usize,
    pub spans: Vec<Span>,
}

impl Cell {
    /// The text this cell shows, without its marks.
    pub fn visible(&self) -> String {
        self.spans.iter().map(Span::visible).collect()
    }
}

/// A row of a table: the line it is, where it lies in the text,
/// `start..end` without its line break, and its cells, one per column of
/// the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub cells: Vec<Cell>,
    /// The cells written beyond the table's columns: shown nowhere, as GFM
    /// has it, and kept when the table is written anew.
    beyond: Vec<(usize, usize)>,
}

impl Row {
    /// What the row reads as: the cells that show something, a space
    /// between them.
    pub fn visible(&self) -> String {
        self.cells
            .iter()
            .map(Cell::visible)
            .filter(|shown| !shown.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A table of a formatted text: where it lies, `start..end`; its lines,
/// `first_line..=last_line`, the line saying its first column holds
/// headings first when `heading_column`; the `delimiter_line` under the
/// header row; how each column lines up its cells; and its rows, the
/// header row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub start: usize,
    pub end: usize,
    pub first_line: usize,
    pub delimiter_line: usize,
    pub last_line: usize,
    /// Whether the first column holds headings, said by
    /// [`HEADING_COLUMN_LINE`] right before the header row.
    pub heading_column: bool,
    pub aligns: Vec<Align>,
    pub rows: Vec<Row>,
}

impl Table {
    pub fn columns(&self) -> usize {
        self.aligns.len()
    }
}

/// Every table of `text`, top first.
pub fn all(text: &str) -> Vec<Arc<Table>> {
    super::parse(text)
        .into_iter()
        .filter_map(|b| b.table.filter(|t| t.first_line == b.line))
        .collect()
}

/// The table the line of the byte offset `offset` belongs to.
pub fn at(text: &str, offset: usize) -> Option<Arc<Table>> {
    let blocks = super::parse(text);
    blocks.get(line_at(text, offset))?.table.clone()
}

/// What the table editor edits of the cell in `row` and `column` of the
/// table at `at`: its content as written, `\|` read as `|`. `None` when
/// there is no such table or cell.
pub fn cell(text: &str, at: usize, row: usize, column: usize) -> Option<String> {
    let table = self::at(text, at)?;
    let cell = table.rows.get(row)?.cells.get(column)?;
    Some(slice(text, cell.start, cell.end).replace("\\|", "|"))
}

/// `text` with the cell in `row` and `column` of the table at `at`
/// holding `cell`, as [`cell`] reads it: each line break becomes a space
/// and each `|` is written `\|`, so the cell stays one cell of one line;
/// the spaces and tabs at its ends go.
pub fn set_cell(text: &str, at: usize, row: usize, column: usize, cell: &str) -> Change {
    change(text, at, |grid| {
        if column >= grid.aligns.len() {
            return false;
        }
        let Some(cells) = grid.rows.get_mut(row) else {
            return false;
        };
        let flat = cell.replace("\r\n", " ").replace(['\r', '\n'], " ");
        let trimmed = flat.trim_matches([' ', '\t']);
        match cells.get_mut(column) {
            Some(slot) => {
                *slot = trimmed.replace('|', "\\|");
                true
            }
            None => false,
        }
    })
}

/// `text` with an empty row put in at `index` of the rows of the table at
/// `at`: 0 makes it the header row, the rows' count puts it last.
pub fn insert_row(text: &str, at: usize, index: usize) -> Change {
    change(text, at, |grid| {
        if index > grid.rows.len() {
            return false;
        }
        grid.rows
            .insert(index, vec![String::new(); grid.aligns.len()]);
        true
    })
}

/// `text` with an empty column put in at `index` of the columns of the
/// table at `at`, lining up as GFM does by default.
pub fn insert_column(text: &str, at: usize, index: usize) -> Change {
    change(text, at, |grid| {
        if index > grid.aligns.len() {
            return false;
        }
        for cells in &mut grid.rows {
            cells.insert(index, String::new());
        }
        grid.aligns.insert(index, Align::None);
        true
    })
}

/// `text` with the row `from` of the table at `at` moved to `to`; a row
/// moved to 0 becomes the header row.
pub fn move_row(text: &str, at: usize, from: usize, to: usize) -> Change {
    change(text, at, |grid| {
        let count = grid.rows.len();
        if from >= count || to >= count || from == to {
            return false;
        }
        let row = grid.rows.remove(from);
        grid.rows.insert(to, row);
        true
    })
}

/// `text` with the column `from` of the table at `at` moved to `to`, its
/// alignment with it.
pub fn move_column(text: &str, at: usize, from: usize, to: usize) -> Change {
    change(text, at, |grid| {
        let count = grid.aligns.len();
        if from >= count || to >= count || from == to {
            return false;
        }
        for cells in &mut grid.rows {
            let cell = cells.remove(from);
            cells.insert(to, cell);
        }
        let align = grid.aligns.remove(from);
        grid.aligns.insert(to, align);
        true
    })
}

/// `text` with the row `row` of the table at `at` deleted; the header
/// row's going makes the next row the header row, and the last row's
/// going takes the table with it.
pub fn delete_row(text: &str, at: usize, row: usize) -> Change {
    change(text, at, |grid| {
        if row >= grid.rows.len() {
            return false;
        }
        grid.rows.remove(row);
        true
    })
}

/// `text` with the column `column` of the table at `at` deleted; the
/// last column's going takes the table with it.
pub fn delete_column(text: &str, at: usize, column: usize) -> Change {
    change(text, at, |grid| {
        if column >= grid.aligns.len() {
            return false;
        }
        for cells in &mut grid.rows {
            cells.remove(column);
        }
        grid.aligns.remove(column);
        true
    })
}

/// `text` with its table at `at` marking its first column as headings, or
/// no longer when it does.
pub fn toggle_heading_column(text: &str, at: usize) -> Change {
    change(text, at, |grid| {
        grid.headings = !grid.headings;
        true
    })
}

/// `text` with a new empty table of two columns, a header row and one
/// more row, on lines of its own, a blank line between it and the lines
/// around it: at a blank line in place of it, at the start of a line
/// before that line, anywhere else in a line after that line, and in a
/// table after that table. It takes the text's first kind of line break,
/// `\n` when there is none.
pub fn insert(text: &str, at: usize) -> Change {
    let offset = floor(text, at);
    let blocks = super::parse(text);
    let Some(block) = blocks.get(line_at(text, offset)) else {
        return unchanged(text, offset);
    };
    let br = first_break(text);
    let table = Grid {
        rows: vec![vec![String::new(); 2]; 2],
        aligns: vec![Align::None; 2],
        headings: false,
    }
    .write(br);
    let bytes = text.as_bytes();
    let blank = |line: usize| {
        blocks
            .get(line)
            .is_some_and(|b| skip_blank(bytes, b.start, b.end) == b.end)
    };
    let line = block.table.as_ref().map_or(block.line, |t| t.last_line);
    let next_blank = |line: usize| line + 1 >= blocks.len() || blank(line + 1);
    let before_blank = |line: usize| line == 0 || blank(line - 1);
    let (split, before, after) = if block.table.is_none() && blank(line) {
        let before = if before_blank(line) { "" } else { br };
        let after = if next_blank(line) { "" } else { br };
        (
            (block.start, block.end),
            before.to_string(),
            after.to_string(),
        )
    } else if block.table.is_none() && offset <= skip_blank(bytes, block.start, block.end) {
        let before = if before_blank(line) { "" } else { br };
        (
            (block.start, block.start),
            before.to_string(),
            format!("{br}{br}"),
        )
    } else {
        let end = blocks.get(line).map_or(text.len(), |b| b.end);
        let after = if next_blank(line) { "" } else { br };
        ((end, end), format!("{br}{br}"), after.to_string())
    };
    let head = slice(text, 0, split.0);
    let start = head.len() + before.len();
    Change {
        text: format!(
            "{head}{before}{table}{after}{}",
            slice(text, split.1, text.len())
        ),
        start,
        end: start + table.len(),
    }
}

fn unchanged(text: &str, offset: usize) -> Change {
    Change {
        text: text.to_string(),
        start: offset,
        end: offset,
    }
}

/// `text` with the table at `at` edited by `edit`, or as it was when
/// `edit` says it cannot be made.
fn change(text: &str, at: usize, edit: impl FnOnce(&mut Grid) -> bool) -> Change {
    let offset = floor(text, at);
    let Some(table) = self::at(text, offset) else {
        return unchanged(text, offset);
    };
    let mut grid = Grid::of(text, &table);
    if !edit(&mut grid) {
        return unchanged(text, offset);
    }
    let bytes = text.as_bytes();
    if grid.rows.is_empty() || grid.aligns.is_empty() {
        let (mut start, mut end) = (table.start, table.end);
        if end < text.len() {
            end += break_at(text, end).len();
        } else if start > 0 {
            start -= if start >= 2 && bytes.get(start - 2..start) == Some(b"\r\n") {
                2
            } else {
                1
            };
        }
        return Change {
            text: format!("{}{}", slice(text, 0, start), slice(text, end, text.len())),
            start,
            end: start,
        };
    }
    let header_end = table.rows.first().map_or(table.end, |r| r.end);
    let written = grid.write(break_at(text, header_end));
    Change {
        text: format!(
            "{}{written}{}",
            slice(text, 0, table.start),
            slice(text, table.end, text.len())
        ),
        start: table.start,
        end: table.start + written.len(),
    }
}

/// The line break at `at` of `text`.
fn break_at(text: &str, at: usize) -> &'static str {
    let bytes = text.as_bytes();
    match bytes.get(at) {
        Some(b'\r') if bytes.get(at + 1) == Some(&b'\n') => "\r\n",
        Some(b'\r') => "\r",
        _ => "\n",
    }
}

fn first_break(text: &str) -> &'static str {
    text.bytes()
        .position(|c| c == b'\n' || c == b'\r')
        .map_or("\n", |i| break_at(text, i))
}

/// A table as the edits change it: the content of each cell as written,
/// each row as long as the columns and with the cells written beyond them
/// after, the columns' alignment, and whether the first column holds
/// headings.
struct Grid {
    rows: Vec<Vec<String>>,
    aligns: Vec<Align>,
    headings: bool,
}

impl Grid {
    fn of(text: &str, table: &Table) -> Grid {
        let rows = table
            .rows
            .iter()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|c| (c.start, c.end))
                    .chain(row.beyond.iter().copied())
                    .map(|(s, e)| slice(text, s, e).to_string())
                    .collect()
            })
            .collect();
        Grid {
            rows,
            aligns: table.aligns.clone(),
            headings: table.heading_column,
        }
    }

    /// The table's lines, `br` between them.
    fn write(&self, br: &str) -> String {
        let line = |cells: &[String]| format!("| {} |", cells.join(" | "));
        let delimiter: Vec<String> = self
            .aligns
            .iter()
            .map(|a| {
                match a {
                    Align::None => "---",
                    Align::Left => ":--",
                    Align::Center => ":-:",
                    Align::Right => "--:",
                }
                .to_string()
            })
            .collect();
        let mut lines = Vec::new();
        if self.headings {
            lines.push(HEADING_COLUMN_LINE.to_string());
        }
        let mut rows = self.rows.iter();
        if let Some(header) = rows.next() {
            lines.push(line(header));
        }
        lines.push(line(&delimiter));
        lines.extend(rows.map(|r| line(r)));
        lines.join(br)
    }
}

/// The table whose first line is the line `first` of `lines`.
///
/// A table is a header row, the delimiter row right under it with as
/// many cells, and every line under those that is a row too; before its
/// header row may come [`HEADING_COLUMN_LINE`]. A row is a line of at
/// least one cell that would be a paragraph line, indented by at most
/// three, not blank, no comment (`<!--`), no fence of three backticks or
/// tildes and no rule (three `-`, `*` or `_`). The delimiter row holds a
/// `|` and cells of `-`, a `:` at either end telling the alignment.
pub(super) fn table_at(text: &str, lines: &[(usize, usize)], first: usize) -> Option<Table> {
    let flagged = is_heading_column_line(text, *lines.get(first)?);
    let header = if flagged { first + 1 } else { first };
    let &(hs, he) = lines.get(header)?;
    if !is_row(text, hs, he) {
        return None;
    }
    let heads = cells(text, hs, he);
    let aligns = delimiter(text, *lines.get(header + 1)?)?;
    if aligns.len() != heads.len() {
        return None;
    }
    let mut last = header + 1;
    while let Some(&(s, e)) = lines.get(last + 1)
        && is_row(text, s, e)
    {
        last += 1;
    }
    let row = |line: usize| -> Row {
        let (start, end) = lines.get(line).copied().unwrap_or_default();
        let found = cells(text, start, end);
        let cells = (0..aligns.len())
            .map(|i| match found.get(i) {
                Some(&(s, e)) => Cell {
                    start: s,
                    end: e,
                    spans: Inline {
                        text,
                        from: s,
                        to: e,
                        links: true,
                        cell: true,
                    }
                    .parse(),
                },
                None => Cell {
                    start: end,
                    end,
                    spans: Vec::new(),
                },
            })
            .collect();
        Row {
            line,
            start,
            end,
            cells,
            beyond: found.get(aligns.len()..).unwrap_or_default().to_vec(),
        }
    };
    let rows = std::iter::once(row(header))
        .chain((header + 2..=last).map(row))
        .collect();
    Some(Table {
        start: lines.get(first)?.0,
        end: lines.get(last)?.1,
        first_line: first,
        delimiter_line: header + 1,
        last_line: last,
        heading_column: flagged,
        aligns,
        rows,
    })
}

/// The blocks of the lines of `table`, each with the table and, a row of
/// it, its row there.
pub(super) fn blocks(table: Table, lines: &[(usize, usize)]) -> Vec<Block> {
    let table = Arc::new(table);
    let header = table.rows.first().map_or(0, |r| r.line);
    (table.first_line..=table.last_line)
        .map(|i| {
            let row = if i == header {
                Some(0)
            } else if i > table.delimiter_line {
                Some(i - table.delimiter_line)
            } else {
                None
            };
            let spans = row
                .and_then(|r| table.rows.get(r))
                .map(|r| r.cells.iter().flat_map(|c| c.spans.clone()).collect())
                .unwrap_or_default();
            let (start, end) = lines.get(i).copied().unwrap_or_default();
            Block {
                kind: BlockKind::Table,
                line: i,
                level: 0,
                number: 0,
                start,
                end,
                content_start: start,
                spans,
                table: Some(Arc::clone(&table)),
                row,
            }
        })
        .collect()
}

fn is_heading_column_line(text: &str, (start, end): (usize, usize)) -> bool {
    slice(text, start, end).trim_matches([' ', '\t']) == HEADING_COLUMN_LINE
}

/// Whether the line `start..end` may be a row of a table.
fn is_row(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut at = start;
    let mut width = 0;
    while at < end && blank(byte(at)) {
        width += if byte(at) == b'\t' { 4 } else { 1 };
        at += 1;
    }
    if at == end || width > 3 {
        return false;
    }
    if super::block(text, start, end).0 != BlockKind::Paragraph {
        return false;
    }
    let rest = slice(text, at, end);
    if rest.starts_with("<!--") || rest.starts_with("```") || rest.starts_with("~~~") {
        return false;
    }
    let c = byte(at);
    if matches!(c, b'-' | b'*' | b'_') {
        let rule = rest.bytes().all(|d| d == c || blank(d));
        if rule && rest.bytes().filter(|&d| d == c).count() >= 3 {
            return false;
        }
    }
    !cells(text, start, end).is_empty()
}

/// The cells of the line `start..end`, without the spaces and tabs
/// around them: split at each `|` that does not follow a `\`, a `|` at
/// the line's start or end opening or closing the row, not a cell.
fn cells(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut cells = Vec::new();
    let mut at = skip_blank(bytes, start, end);
    if at < end && byte(at) == b'|' {
        at = skip_blank(bytes, at + 1, end);
    }
    while at < end {
        let mut stop = at;
        while stop < end && !(byte(stop) == b'|' && !(stop > start && byte(stop - 1) == b'\\')) {
            stop += 1;
        }
        let mut to = stop;
        while to > at && blank(byte(to - 1)) {
            to -= 1;
        }
        cells.push((at, to));
        if stop >= end {
            break;
        }
        at = skip_blank(bytes, stop + 1, end);
    }
    cells
}

/// How the columns of the delimiter row `start..end` line up, or `None`
/// when it is none.
fn delimiter(text: &str, (start, end): (usize, usize)) -> Option<Vec<Align>> {
    if !is_row(text, start, end) || !slice(text, start, end).contains('|') {
        return None;
    }
    cells(text, start, end)
        .into_iter()
        .map(|(from, to)| {
            let cell = slice(text, from, to);
            let (left, inner) = match cell.strip_prefix(':') {
                Some(rest) => (true, rest),
                None => (false, cell),
            };
            let (right, inner) = match inner.strip_suffix(':') {
                Some(rest) => (true, rest),
                None => (false, inner),
            };
            if inner.is_empty() || !inner.bytes().all(|c| c == b'-') {
                return None;
            }
            Some(match (left, right) {
                (true, true) => Align::Center,
                (true, false) => Align::Left,
                (false, true) => Align::Right,
                (false, false) => Align::None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str) -> Vec<Vec<String>> {
        all(text)
            .first()
            .map(|t| {
                t.rows
                    .iter()
                    .map(|r| r.cells.iter().map(Cell::visible).collect())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn a_table_is_a_header_row_a_delimiter_row_and_the_rows_under() {
        let text = "Shop\n| Item | **Qty** |\n|:--|--:|\n| milk |\n| eggs | 6 | beyond |\nbread | `a\\|b` \\| c\n\nafter";
        let blocks = super::super::parse(text);
        let kinds: Vec<&str> = blocks.iter().map(|b| b.kind.name()).collect();
        assert_eq!(
            kinds,
            [
                "paragraph",
                "table",
                "table",
                "table",
                "table",
                "table",
                "paragraph",
                "paragraph"
            ]
        );
        let rows_of: Vec<Option<usize>> = blocks.iter().map(|b| b.row).collect();
        assert_eq!(
            rows_of,
            [None, Some(0), None, Some(1), Some(2), Some(3), None, None]
        );
        assert_eq!(
            rows(text),
            [
                vec!["Item", "Qty"],
                vec!["milk", ""],
                vec!["eggs", "6"],
                vec!["bread", "a|b | c"],
            ]
        );
        let table = all(text).first().cloned().ok_or("a table");
        assert_eq!(
            table.map(|t| t.aligns.clone()),
            Ok(vec![Align::Left, Align::Right])
        );
        // As GFM splits a row, a `\` before a pipe always keeps it in.
        assert_eq!(rows("a \\\\| b | c\n-|-"), [vec!["a \\| b", "c"]]);
        for none in [
            "a | b\n---",
            "a | b\n- | -",
            "# a | b\n--|--",
            "a | b\n|--|--|--|",
        ] {
            assert!(all(none).is_empty(), "{none:?}");
        }
    }

    #[test]
    fn a_cell_is_written_so_that_it_stays_one_cell_of_one_line() {
        let text = "Before\n\n| a | b \\| c |\n|---|--:|\n| d |\n\nAfter";
        let at = text.find("| a").unwrap_or_default();
        assert_eq!(cell(text, at, 0, 1).as_deref(), Some("b | c"));
        assert_eq!(cell(text, at, 2, 0), None);
        for (value, written) in [
            ("# x", "# x"),
            ("- x", "- x"),
            ("a|b", "a\\|b"),
            ("`a|b`", "`a\\|b`"),
            ("a\r\nb\nc\rd", "a b c d"),
            ("  **x**\t", "**x**"),
        ] {
            let change = set_cell(text, at, 1, 1, value);
            assert_eq!(
                change.text,
                format!("Before\n\n| a | b \\| c |\n| --- | --: |\n| d | {written} |\n\nAfter"),
                "{value:?}"
            );
            assert_eq!(all(&change.text).first().map(|t| t.rows.len()), Some(2));
        }
        assert_eq!(set_cell(text, at, 2, 0, "x").text, text);
    }

    #[test]
    fn the_last_row_or_column_going_takes_the_table_with_it() {
        let text = "x\n| a |\n|---|\n\ny";
        assert_eq!(delete_row(text, 2, 0).text, "x\n\ny");
        let last = delete_column("x\r\n| a |\r\n|-|", 4, 0);
        assert_eq!((last.text.as_str(), last.start), ("x", 1));
        // Never a panic, whatever the offsets.
        assert_eq!(insert_row("ü | b\n-|-", 1, 9).text, "ü | b\n-|-");
        assert_eq!(insert("ü", 2).text, "ü\n\n|  |  |\n| --- | --- |\n|  |  |");
    }
}
