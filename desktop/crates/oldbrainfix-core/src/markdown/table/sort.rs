//! Sorting the rows of a table by one of its columns (obf-1052), as the
//! Dart core's `markdown_table_sort.dart` sorts them. The header row stays
//! first; the rows under it are ordered by what their cells in the column
//! show ([`super::Cell::visible`]), without the ids of their boxes and
//! variation selectors and without spaces, tabs, U+00A0 and U+202F at
//! their ends. A column compares its cells by [`SortKind`].

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::findings;
use crate::markdown::{Change, floor, slice};

use super::Table;

/// How a column of a table compares its cells when the rows are sorted
/// by it: by number when every cell of it that shows something reads as
/// a number, else by date when every one reads as a date, else by text.
///
/// A number is: a sign (`-`, `+` or `−`), a currency sign (`$`, `€`,
/// `£`, `¥`, `¢`, `₹`, `₽`, `₩`, `₺`, `₪`, `₫`, `฿`, `₴`, `₦`, `₱`, `₿`)
/// with spaces after it and a sign again where none came before, each
/// when it is there, then ASCII digits, grouped by `.`, `,`, `'`, `’`,
/// U+00A0 or U+202F, and the rest, a unit, holding no digit: "12,50 €",
/// "€ 3", "3 kg", "1.5 bar", "1'234.5", "−2 %". A `.` or a `,` is the
/// decimal mark when it comes once and last; the groups after a mark that
/// groups have three digits, the first one one to three. The column
/// decides which of the two is the decimal mark: a cell holding both has
/// the last one; a cell holding one twice groups by it; a cell holding
/// one once and last has it as the decimal mark when fewer or more than
/// three digits follow it, more than three or a 0 first come before it,
/// or a `'`, `’`, U+00A0 or U+202F groups it too. Where no cell says, the
/// mark seen grouping makes the other one the decimal mark; where nothing
/// says, a cell as "1,234" or "1.234" reads 1234. Cells that disagree
/// make the column one of text. Numbers compare exactly as written, their
/// units not taken into account.
///
/// A date is written as the app's languages write it: `2026-11-24`
/// (ISO), `24.11.2026` (German) or `11/24/2026` (English), day and month
/// of one or two digits, the year of four, and in the calendar.
///
/// Text compares folded as search folds it ([`findings::fold`]), case,
/// accents and umlauts not counted, code point by code point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKind {
    Number,
    Date,
    Text,
}

impl SortKind {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            SortKind::Number => "number",
            SortKind::Date => "date",
            SortKind::Text => "text",
        }
    }
}

/// How the column `column` of `table` compares its cells when its rows
/// are sorted by it; [`SortKind::Text`] for a column it does not have.
pub fn sort_kind(table: &Table, column: usize) -> SortKind {
    sort_column(table, column).0
}

/// The rows under the header row of `table` sorted by the column
/// `column`, ascending or `descending`, as [`SortKind`] compares them:
/// each by its index in [`Table::rows`], the header row 0, so the first
/// row under it is 1. A cell that shows nothing goes last both ways; rows
/// that compare equal keep the order they are written in, both ways. A
/// column the table does not have gives them as written.
pub fn sorted_rows(table: &Table, column: usize, descending: bool) -> Vec<usize> {
    let (_, keys) = sort_column(table, column);
    let mut order: Vec<usize> = (1..table.rows.len()).collect();
    order.sort_by(|&x, &y| {
        let a = keys.get(x - 1).and_then(Option::as_ref);
        let b = keys.get(y - 1).and_then(Option::as_ref);
        match (a, b) {
            (Some(a), Some(b)) => {
                let c = a.compare(b);
                let c = if descending { c.reverse() } else { c };
                c.then(x.cmp(&y))
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => x.cmp(&y),
        }
    });
    order
}

/// `text` with the rows under the header row of its table at `at` in the
/// order [`sorted_rows`] gives: each row's line as it was written moved,
/// the line breaks where they were, and the header row, the delimiter row
/// and [`super::HEADING_COLUMN_LINE`] untouched. The change selects the
/// table; rows already in that order, a column the table does not have or
/// no table give the text as it was.
pub fn sort_rows(text: &str, at: usize, column: usize, descending: bool) -> Change {
    let offset = floor(text, at);
    let Some(table) = super::at(text, offset) else {
        return super::unchanged(text, offset);
    };
    if column >= table.columns() {
        return super::unchanged(text, offset);
    }
    let order = sorted_rows(&table, column, descending);
    if order.iter().enumerate().all(|(i, &row)| row == i + 1) {
        return super::unchanged(text, offset);
    }
    let body = table.rows.get(1..).unwrap_or_default();
    let (Some(first), Some(last)) = (body.first(), body.last()) else {
        return super::unchanged(text, offset);
    };
    let mut out = String::new();
    for (i, &index) in order.iter().enumerate() {
        if let Some(row) = table.rows.get(index) {
            out.push_str(slice(text, row.start, row.end));
        }
        if let (Some(this), Some(next)) = (body.get(i), body.get(i + 1)) {
            out.push_str(slice(text, this.end, next.start));
        }
    }
    Change {
        text: format!(
            "{}{out}{}",
            slice(text, 0, first.start),
            slice(text, last.end, text.len())
        ),
        start: table.start,
        end: table.end,
    }
}

/// What a row is sorted by: its cell's date as year, month and day in
/// one number, its number, or its text folded.
enum Key {
    Date(u32),
    Number(Number),
    Text(String),
}

impl Key {
    fn compare(&self, other: &Key) -> Ordering {
        match (self, other) {
            (Key::Date(a), Key::Date(b)) => a.cmp(b),
            (Key::Number(a), Key::Number(b)) => a.compare(b),
            (Key::Text(a), Key::Text(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}

/// The column `column` of `table` as [`sorted_rows`] sorts by it: its
/// kind and the key of each row under the header row, `None` for a cell
/// that shows nothing.
fn sort_column(table: &Table, column: usize) -> (SortKind, Vec<Option<Key>>) {
    let shown: Vec<String> = table
        .rows
        .iter()
        .skip(1)
        .map(|row| {
            row.cells
                .get(column)
                .map(|cell| sort_text(&cell.visible()))
                .unwrap_or_default()
        })
        .collect();
    if shown.iter().any(|s| !s.is_empty()) {
        let dates: Vec<Option<u32>> = shown
            .iter()
            .map(|s| if s.is_empty() { None } else { date(s) })
            .collect();
        if shown
            .iter()
            .zip(&dates)
            .all(|(s, d)| s.is_empty() || d.is_some())
        {
            return (
                SortKind::Date,
                dates.into_iter().map(|d| d.map(Key::Date)).collect(),
            );
        }
        if let Some(numbers) = numbers(&shown) {
            return (
                SortKind::Number,
                numbers.into_iter().map(|n| n.map(Key::Number)).collect(),
            );
        }
    }
    let keys = shown
        .iter()
        .map(|s| (!s.is_empty()).then(|| Key::Text(findings::fold(s))))
        .collect();
    (SortKind::Text, keys)
}

/// `visible` as the sorting reads it: without the ids of boxes (tag
/// characters) and variation selectors, and without spaces, tabs, U+00A0
/// and U+202F at its ends.
fn sort_text(visible: &str) -> String {
    let kept: String = visible
        .chars()
        .filter(|c| !matches!(c, '\u{E0000}'..='\u{E007F}' | '\u{FE00}'..='\u{FE0F}'))
        .collect();
    kept.trim_matches([' ', '\t', '\u{a0}', '\u{202f}'])
        .to_string()
}

/// The date `s` is, as year, month and day in one number.
fn date(s: &str) -> Option<u32> {
    let mut runs: Vec<&str> = Vec::new();
    let mut mark = None;
    let mut rest = s;
    loop {
        let len = rest.bytes().take_while(u8::is_ascii_digit).count();
        if len == 0 {
            return None;
        }
        runs.push(rest.get(..len)?);
        rest = rest.get(len..)?;
        let Some(c) = rest.chars().next() else {
            break;
        };
        if runs.len() == 3 || !matches!(c, '-' | '.' | '/') || mark.is_some_and(|m| m != c) {
            return None;
        }
        mark = Some(c);
        rest = rest.get(c.len_utf8()..)?;
    }
    let (year, month, day) = match (mark, runs.as_slice()) {
        (Some('-'), &[y, m, d]) if y.len() == 4 => (y, m, d),
        (Some('.'), &[d, m, y]) if y.len() == 4 => (y, m, d),
        (Some('/'), &[m, d, y]) if y.len() == 4 => (y, m, d),
        _ => return None,
    };
    if month.len() > 2 || day.len() > 2 {
        return None;
    }
    let y: u32 = year.parse().ok()?;
    let m: u32 = month.parse().ok()?;
    let d: u32 = day.parse().ok()?;
    let leap = y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let days = match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=12).contains(&m) || d < 1 || d > days {
        return None;
    }
    Some(y * 10000 + m * 100 + d)
}

/// The mark that only groups digits, `'`, `’`, U+00A0 and U+202F alike.
const GROUP_MARK: char = '\'';

const CURRENCIES: [char; 16] = [
    '$', '€', '£', '¥', '¢', '₹', '₽', '₩', '₺', '₪', '₫', '฿', '₴', '₦', '₱', '₿',
];

/// A cell as a number is written: its sign, its runs of digits and the
/// marks between them, `.`, `,` or [`GROUP_MARK`].
struct Written {
    negative: bool,
    groups: Vec<String>,
    marks: Vec<char>,
}

fn written(s: &str) -> Option<Written> {
    let chars: Vec<char> = s.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let digit = |i: usize| at(i).is_some_and(|c| c.is_ascii_digit());
    let mut i = 0;
    let mut negative = false;
    let mut signed = false;
    let mut sign = |i: &mut usize| {
        if signed {
            return;
        }
        if let Some(c @ ('-' | '+' | '−')) = at(*i) {
            negative = c != '+';
            signed = true;
            *i += 1;
        }
    };
    sign(&mut i);
    if at(i).is_some_and(|c| CURRENCIES.contains(&c)) {
        i += 1;
        while matches!(at(i), Some(' ' | '\u{a0}' | '\u{202f}')) {
            i += 1;
        }
        sign(&mut i);
    }
    let digits = |i: &mut usize| -> String {
        let start = *i;
        while digit(*i) {
            *i += 1;
        }
        chars.get(start..*i).unwrap_or_default().iter().collect()
    };
    let first = digits(&mut i);
    if first.is_empty() {
        return None;
    }
    let mut groups = vec![first];
    let mut marks = Vec::new();
    while digit(i + 1) {
        let mark = match at(i) {
            Some('.') => '.',
            Some(',') => ',',
            Some('\'' | '’' | '\u{a0}' | '\u{202f}') => GROUP_MARK,
            _ => break,
        };
        i += 1;
        marks.push(mark);
        groups.push(digits(&mut i));
    }
    if chars
        .get(i..)
        .unwrap_or_default()
        .iter()
        .any(char::is_ascii_digit)
    {
        return None;
    }
    Some(Written {
        negative,
        groups,
        marks,
    })
}

/// The cells `shown` as numbers, `None` for those that show nothing, or
/// `None` when one of them is no number or they disagree on the decimal
/// mark.
fn numbers(shown: &[String]) -> Option<Vec<Option<Number>>> {
    let written: Vec<Option<Written>> = shown
        .iter()
        .map(|s| {
            if s.is_empty() {
                Some(None)
            } else {
                written(s).map(Some)
            }
        })
        .collect::<Option<_>>()?;
    let mut decimal = BTreeSet::new();
    let mut grouping = BTreeSet::new();
    for w in written.iter().flatten() {
        let marks: Vec<char> = w
            .marks
            .iter()
            .copied()
            .filter(|&m| m != GROUP_MARK)
            .collect();
        let Some(&last) = marks.last() else {
            continue;
        };
        if marks.contains(&'.') && marks.contains(&',') {
            decimal.insert(last);
        } else if marks.len() > 1 {
            grouping.insert(last);
        } else {
            let at = w.marks.iter().position(|&m| m == last).unwrap_or_default();
            let before = w.groups.get(at).map_or("", String::as_str);
            if at + 1 != w.marks.len() {
                grouping.insert(last);
            } else if w.groups.get(at + 1).map_or(0, String::len) != 3
                || before.len() > 3
                || before.starts_with('0')
                || w.marks.contains(&GROUP_MARK)
            {
                decimal.insert(last);
            }
        }
    }
    if decimal.len() > 1 {
        return None;
    }
    let mark = match (decimal.first(), grouping.first()) {
        (Some(&d), _) => Some(d),
        (None, Some(&g)) if grouping.len() == 1 => Some(if g == '.' { ',' } else { '.' }),
        _ => None,
    };
    if mark.is_some_and(|m| grouping.contains(&m)) {
        return None;
    }
    written
        .iter()
        .map(|w| match w {
            None => Some(None),
            Some(w) => Number::of(w, mark).map(Some),
        })
        .collect()
}

/// A number exactly as written: its sign, its whole digits without the
/// zeros before them and its fraction's without the zeros after them.
struct Number {
    negative: bool,
    whole: String,
    fraction: String,
}

impl Number {
    /// `w` with `mark` as its decimal mark, or `None` when it does not
    /// read so.
    fn of(w: &Written, mark: Option<char>) -> Option<Number> {
        let mut cut = w.marks.len();
        if let Some(at) = mark.and_then(|m| w.marks.iter().position(|&c| c == m)) {
            if at + 1 != w.marks.len() {
                return None;
            }
            cut = at;
        }
        let whole_groups = w.groups.get(..=cut)?;
        if cut > 0
            && (whole_groups.first().map_or(0, String::len) > 3
                || whole_groups.iter().skip(1).any(|g| g.len() != 3))
        {
            return None;
        }
        let whole = whole_groups.concat().trim_start_matches('0').to_string();
        let fraction = if cut < w.marks.len() {
            w.groups.get(cut + 1).map_or("", String::as_str)
        } else {
            ""
        };
        let fraction = fraction.trim_end_matches('0').to_string();
        Some(Number {
            negative: w.negative && !(whole.is_empty() && fraction.is_empty()),
            whole,
            fraction,
        })
    }

    fn compare(&self, other: &Number) -> Ordering {
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let c = self
            .whole
            .len()
            .cmp(&other.whole.len())
            .then_with(|| self.whole.cmp(&other.whole))
            .then_with(|| self.fraction.cmp(&other.fraction));
        if self.negative { c.reverse() } else { c }
    }
}

#[cfg(test)]
mod tests;
