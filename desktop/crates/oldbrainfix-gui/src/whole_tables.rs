//! A table of a formatted text deleted or moved as a whole from its ⋮
//! (obf-1050), every other character of the text kept, as the phone's
//! `whole_tables.dart` does. Offsets are bytes.
//!
//! The core reads every line as a block of its own
//! ([`markdown::parse`]); a table moves past the block above or below it,
//! and a block is:
//!
//! - a table, its lines from the first to the last, the line marking its
//!   first column as headings included;
//! - a fenced code block, from its opening fence to its closing one;
//! - a heading, its one line;
//! - a list, the bulleted and numbered items one right under the other;
//! - a quote, its lines one right under the other;
//! - a paragraph, the lines of words one right under the other.
//!
//! A blank line, nothing on it but white space, is no block: it parts two
//! of them and stays where it is.

use oldbrainfix_core::markdown::table::{self, Table};
use oldbrainfix_core::markdown::{self, Block, BlockKind, Change};

/// A part of a formatted text a table moves past as a whole: its lines
/// from `first` to `last`, where it lies, `start..end` without the line
/// break after it, and the kind of its first line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextBlock {
    pub first: usize,
    pub last: usize,
    pub start: usize,
    pub end: usize,
    pub kind: BlockKind,
}

/// Whether the line `line` of `text` is blank, no block.
pub fn blank(text: &str, line: &Block) -> bool {
    line.kind == BlockKind::Paragraph
        && text
            .get(line.start..line.end)
            .is_some_and(|l| l.trim().is_empty())
}

/// Whether there is a block of `text` above `table` and one below it to
/// move past: anything but white space before it and after it, as every
/// line that is not blank belongs to a block.
pub fn moves(text: &str, table: &Table) -> (bool, bool) {
    let shows = |part: Option<&str>| part.is_some_and(|p| !p.trim().is_empty());
    (shows(text.get(..table.start)), shows(text.get(table.end..)))
}

/// The blocks of `text`, top first.
pub fn blocks(text: &str) -> Vec<TextBlock> {
    let lines = markdown::parse(text);
    let joins = |line: &Block, kind: BlockKind| {
        line.table.is_none()
            && line.code.is_none()
            && !blank(text, line)
            && match kind {
                BlockKind::Bullet | BlockKind::Numbered => {
                    matches!(line.kind, BlockKind::Bullet | BlockKind::Numbered)
                }
                BlockKind::Quote | BlockKind::Paragraph => line.kind == kind,
                _ => false,
            }
    };
    let mut blocks = Vec::new();
    let mut i = 0;
    while let Some(line) = lines.get(i) {
        if blank(text, line) {
            i += 1;
            continue;
        }
        let mut last = line
            .table
            .as_ref()
            .map(|t| t.last_line)
            .or_else(|| line.code.as_ref().map(|c| c.last_line))
            .unwrap_or(i);
        while lines
            .get(last + 1)
            .is_some_and(|next| joins(next, line.kind))
        {
            last += 1;
        }
        let end = lines.get(last).map_or(line.end, |l| l.end);
        blocks.push(TextBlock {
            first: i,
            last,
            start: line.start,
            end,
            kind: line.kind,
        });
        i = last + 1;
    }
    blocks
}

/// Whether the table at `at` of `text` has a block above it or, not
/// `up`, below it, to move past.
pub fn can_move(text: &str, at: usize, up: bool) -> bool {
    neighbours(text, at, up).is_some()
}

/// `text` with the table at `at` past the block above it or, not `up`,
/// below it; `None` where there is no table or no such block. The text
/// between the two stays as it is, the blank lines and line breaks; where
/// a table would stand right above a paragraph or another table, which
/// would read as its rows, a blank line goes between them. The change
/// selects the table where it lies then.
pub fn move_table(text: &str, at: usize, up: bool) -> Option<Change> {
    let found = neighbours(text, at, up)?;
    let (table, other) = (found.table, found.other);
    let (above, below) = if up { (other, table) } else { (table, other) };
    let part = |from: usize, to: usize| text.get(from..to).unwrap_or_default();
    let middle = part(above.end, below.start);
    let table_text = part(table.start, table.end);
    let other_text = part(other.start, other.end);
    if up {
        // The block before the other one, the table, the other one, the
        // block after the table.
        let cut = found.before.map_or(above.start, |b| b.end);
        let head = part(0, cut);
        let lead = gap(found.before, Some(table), part(cut, above.start));
        let start = head.len() + lead.len();
        let between = gap(Some(table), Some(other), middle);
        let cut_after = found.after.map_or(below.end, |b| b.start);
        let trail = gap(Some(other), found.after, part(below.end, cut_after));
        let tail = part(cut_after, text.len());
        return Some(Change {
            text: format!("{head}{lead}{table_text}{between}{other_text}{trail}{tail}"),
            start,
            end: start + table_text.len(),
        });
    }
    // The block before the table, the other block, the table, the block
    // after the other one.
    let cut_before = found.before.map_or(above.start, |b| b.end);
    let head = part(0, cut_before);
    let lead = gap(found.before, Some(other), part(cut_before, above.start));
    let cut = found.after.map_or(below.end, |b| b.start);
    let between = gap(Some(other), Some(table), middle);
    let trail = gap(Some(table), found.after, part(below.end, cut));
    let start = head.len() + lead.len() + other_text.len() + between.len();
    let rest = part(cut, text.len());
    Some(Change {
        text: format!("{head}{lead}{other_text}{between}{table_text}{trail}{rest}"),
        start,
        end: start + table_text.len(),
    })
}

/// `text` without the table at `at` and the line break after it, or,
/// last in the text, the one before it, as deleting its last row does;
/// `None` where there is none. The cursor is where it was.
pub fn delete_table(text: &str, at: usize) -> Option<Change> {
    let table = table::at(text, at.min(text.len()))?;
    let (mut start, mut end) = (table.start, table.end);
    let bytes = text.as_bytes();
    if end < text.len() {
        end += if text.get(end..)?.starts_with("\r\n") {
            2
        } else {
            1
        };
    } else if start > 0 {
        let crlf = bytes.get(start - 1) == Some(&b'\n')
            && start >= 2
            && bytes.get(start - 2) == Some(&b'\r');
        start -= if crlf { 2 } else { 1 };
    }
    let text = format!("{}{}", text.get(..start)?, text.get(end..)?);
    Some(Change {
        text,
        start,
        end: start,
    })
}

/// Whether the table at `at` of `text` holds words in any cell.
pub fn holds_words(text: &str, at: usize) -> bool {
    table::at(text, at.min(text.len())).is_some_and(|table| {
        table
            .rows
            .iter()
            .any(|row| row.cells.iter().any(|cell| cell.end > cell.start))
    })
}

/// What lies between `top` and `bottom`, `between`: one line break more
/// where `top` is a table and `bottom` a paragraph or a table with only
/// one line break between them, which would join them.
fn gap(top: Option<TextBlock>, bottom: Option<TextBlock>, between: &str) -> String {
    let (Some(top), Some(bottom)) = (top, bottom) else {
        return between.to_string();
    };
    let touches = top.kind == BlockKind::Table
        && matches!(bottom.kind, BlockKind::Paragraph | BlockKind::Table);
    if touches && breaks(between) == 1 {
        format!("{between}{between}")
    } else {
        between.to_string()
    }
}

/// How many line breaks `text` holds, `\r\n` counting one.
fn breaks(text: &str) -> usize {
    text.replace("\r\n", "\n").matches(['\n', '\r']).count()
}

/// The table at `at` of `text` as a block, the block it moves past, and
/// the blocks right before and after the two, if any.
struct Neighbours {
    table: TextBlock,
    other: TextBlock,
    before: Option<TextBlock>,
    after: Option<TextBlock>,
}

fn neighbours(text: &str, at: usize, up: bool) -> Option<Neighbours> {
    let table = table::at(text, at.min(text.len()))?;
    let blocks = blocks(text);
    let index = blocks.iter().position(|b| b.first == table.first_line)?;
    let other = if up { index.checked_sub(1)? } else { index + 1 };
    let (top, bottom) = if up { (other, index) } else { (index, other) };
    Some(Neighbours {
        table: *blocks.get(index)?,
        other: *blocks.get(other)?,
        before: top.checked_sub(1).and_then(|b| blocks.get(b).copied()),
        after: blocks.get(bottom + 1).copied(),
    })
}

#[cfg(test)]
mod tests;
