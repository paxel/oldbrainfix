//! Deprecated: replaced by the boxes of [`crate::text_boxes`] (ADR 0035,
//! which supersedes ADR 0031); kept only until the apps have switched
//! over, then removed.
//!
//! Status marks in user text (ADR 0031), as the Dart core's
//! `text_marks.dart` finds and changes them: a ☐, ✅ or ❌ that begins a
//! line of a Note's or a Section's text shows that line open, checked or
//! failed, and a click changes it. The marks are part of the text, so
//! changing one is an edit of the text like any other; a mark elsewhere
//! in a line is plain text.
//!
//! A mark counts when it is the first character of a line after any
//! spaces and tabs, and after a list item's `-`, `*` or `+` followed by
//! a space or a tab. One variation selector after it (U+FE0E or U+FE0F)
//! belongs to the mark; a zero-width joiner after it makes it part of
//! another emoji, which is no mark. Offsets are in UTF-8 bytes here,
//! where the Dart core counts UTF-16 code units.

/// The state a status mark shows, and the character it is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextMark {
    /// ☐: not done yet.
    Open,
    /// ✅: done.
    Checked,
    /// ❌: failed, or not to be done.
    Failed,
}

impl TextMark {
    /// Every state, in the Dart core's order.
    pub const ALL: [TextMark; 3] = [TextMark::Open, TextMark::Checked, TextMark::Failed];

    /// The character the mark is written as, without a variation
    /// selector.
    pub fn glyph(self) -> char {
        match self {
            TextMark::Open => '\u{2610}',
            TextMark::Checked => '\u{2705}',
            TextMark::Failed => '\u{274C}',
        }
    }

    /// Its name, as the Dart core's enum names it.
    pub fn name(self) -> &'static str {
        match self {
            TextMark::Open => "open",
            TextMark::Checked => "checked",
            TextMark::Failed => "failed",
        }
    }
}

/// A status mark found in a text: the line it begins, counted from 0 at
/// every line break as [`crate::checklists::split_lines`] counts them,
/// the mark it shows, and where it lies in the text, `start..end` in
/// bytes, a variation selector after it included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkedLine {
    pub line: usize,
    pub mark: TextMark,
    pub start: usize,
    pub end: usize,
}

const TEXT_STYLE: char = '\u{FE0E}';
const EMOJI_STYLE: char = '\u{FE0F}';
const JOINER: char = '\u{200D}';

/// Every status mark in `text`, top line first.
pub fn find(text: &str) -> Vec<MarkedLine> {
    lines(text)
        .into_iter()
        .enumerate()
        .filter_map(|(line, (start, end))| {
            mark_in(text, start, end).map(|(mark, start, end)| MarkedLine {
                line,
                mark,
                start,
                end,
            })
        })
        .collect()
}

/// `text` with the status mark of the line `line` changed to `to`,
/// everything else as it was; a variation selector after the old mark
/// goes with it. `None` when there is nothing to change: no such line, no
/// mark on it, or the mark is `to` already.
pub fn swap(text: &str, line: usize, to: TextMark) -> Option<String> {
    let (start, end) = *lines(text).get(line)?;
    let (mark, from, until) = mark_in(text, start, end)?;
    if mark == to {
        return None;
    }
    Some(format!(
        "{}{}{}",
        text.get(..from)?,
        to.glyph(),
        text.get(until..)?
    ))
}

/// `text` with "☐ " put at the start of the line `line`, after its
/// spaces and tabs and a list item's `- `, where [`find`] then finds it.
/// `None` when the line has a mark already or there is no such line. The
/// editor moves its cursor by the length the text grew.
pub fn insert_open(text: &str, line: usize) -> Option<String> {
    let (start, end) = *lines(text).get(line)?;
    if mark_in(text, start, end).is_some() {
        return None;
    }
    let at = mark_start(text.as_bytes(), start, end);
    Some(format!(
        "{}{} {}",
        text.get(..at)?,
        TextMark::Open.glyph(),
        text.get(at..)?
    ))
}

/// The line the byte offset `offset` of `text` lies on, as [`find`]
/// counts lines: a cursor between the `\r` and the `\n` of a line break
/// is still on the line before. An offset past the text is taken as its
/// end.
pub fn line_at(text: &str, offset: usize) -> usize {
    let bytes = text.as_bytes();
    let stop = offset.min(bytes.len());
    let mut line = 0;
    let mut i = 0;
    while i < stop {
        match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                if i + 1 < stop {
                    line += 1;
                }
                i += 1;
            }
            b'\n' | b'\r' => line += 1,
            _ => {}
        }
        i += 1;
    }
    line
}

/// The lines of `text` as byte offsets, from the first character to the
/// line break: split at `\n`, `\r\n` and a lone `\r`.
fn lines(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c != b'\n' && c != b'\r' {
            i += 1;
            continue;
        }
        lines.push((start, i));
        let pair = c == b'\r' && bytes.get(i + 1) == Some(&b'\n');
        i += if pair { 2 } else { 1 };
        start = i;
    }
    lines.push((start, bytes.len()));
    lines
}

fn blank(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn skip_blank(bytes: &[u8], mut at: usize, end: usize) -> usize {
    while at < end && blank(bytes[at]) {
        at += 1;
    }
    at
}

/// Where a mark of the line `start..end` would lie: after its spaces and
/// tabs and a list item's bullet. Only ASCII is skipped, so it is a
/// character boundary.
fn mark_start(bytes: &[u8], start: usize, end: usize) -> usize {
    let at = skip_blank(bytes, start, end);
    if at + 1 < end && matches!(bytes[at], b'-' | b'*' | b'+') && blank(bytes[at + 1]) {
        return skip_blank(bytes, at + 1, end);
    }
    at
}

/// The mark of the line `start..end` with its byte offsets, or `None`.
fn mark_in(text: &str, start: usize, end: usize) -> Option<(TextMark, usize, usize)> {
    let at = mark_start(text.as_bytes(), start, end);
    let rest = text.get(at..end)?;
    let (mark, after) = TextMark::ALL
        .into_iter()
        .find_map(|m| rest.strip_prefix(m.glyph()).map(|after| (m, after)))?;
    let after = after
        .strip_prefix([TEXT_STYLE, EMOJI_STYLE])
        .unwrap_or(after);
    if after.starts_with(JOINER) {
        return None;
    }
    Some((mark, at, end - after.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checklists::split_lines;

    /// The marks of `text` as (line, mark, the mark as written).
    fn marks(text: &str) -> Vec<(usize, TextMark, &str)> {
        find(text)
            .into_iter()
            .map(|m| (m.line, m.mark, &text[m.start..m.end]))
            .collect()
    }

    #[test]
    fn a_mark_counts_as_the_first_character_of_a_line() {
        assert_eq!(
            marks("Release\n✅ v2 at customer\n❌ v1\n☐ v3"),
            [
                (1, TextMark::Checked, "✅"),
                (2, TextMark::Failed, "❌"),
                (3, TextMark::Open, "☐"),
            ]
        );
        assert_eq!(marks("☐"), [(0, TextMark::Open, "☐")]);
        assert_eq!(marks("✅done"), [(0, TextMark::Checked, "✅")]);
    }

    #[test]
    fn a_mark_elsewhere_in_a_line_is_plain_text() {
        assert!(marks("v2 supported at customer: ✅").is_empty());
        assert!(marks("😀 ☐ after an emoji").is_empty());
        assert!(marks("x ❌\ny ✅ z").is_empty());
    }

    #[test]
    fn spaces_tabs_and_a_list_items_bullet_may_come_before_it() {
        assert_eq!(
            marks("  ☐ a\n\t✅ b\n \t ❌ c\n- ☐ d\n* ✅ e\n+\t❌ f\n-   ☐ g")
                .iter()
                .map(|m| (m.0, m.1))
                .collect::<Vec<_>>(),
            [
                (0, TextMark::Open),
                (1, TextMark::Checked),
                (2, TextMark::Failed),
                (3, TextMark::Open),
                (4, TextMark::Checked),
                (5, TextMark::Failed),
                (6, TextMark::Open),
            ]
        );
        // Nothing else: no bullet without a space, no second bullet, no
        // numbered item, no other kind of space.
        assert!(marks("-☐ a\n- - ☐ b\n1. ☐ c\n\u{A0}☐ d\n\u{FEFF}☐ e\n> ☐ f").is_empty());
    }

    #[test]
    fn a_variation_selector_belongs_to_the_mark_a_joiner_makes_it_another_emoji() {
        assert_eq!(
            marks("✅\u{FE0F} a\n❌\u{FE0E} b\n☐\u{FE0F}\u{FE0F} c"),
            [
                (0, TextMark::Checked, "✅\u{FE0F}"),
                (1, TextMark::Failed, "❌\u{FE0E}"),
                (2, TextMark::Open, "☐\u{FE0F}"),
            ]
        );
        assert!(marks("✅\u{200D}🔥 a\n✅\u{FE0F}\u{200D}🔥 b\n\u{FE0F}☐ c").is_empty());
    }

    #[test]
    fn other_boxes_and_ticks_are_no_marks() {
        assert!(marks("☑ a\n☒ b\n✔ c\n✓ d\n❎ e\n✖ f\n✗ g").is_empty());
    }

    #[test]
    fn lines_break_at_lf_crlf_and_a_lone_cr_as_split_lines_does() {
        let text = "a\r\n☐ b\r❌ c\n\n✅";
        assert_eq!(
            marks(text).iter().map(|m| m.0).collect::<Vec<_>>(),
            [1, 2, 4]
        );
        assert_eq!(split_lines(text)[4], "✅");
        assert_eq!(marks("☐ a\n"), [(0, TextMark::Open, "☐")]);
        assert!(marks("").is_empty());
    }

    #[test]
    fn offsets_are_utf8_bytes() {
        let found = find("🧠\n☐ x");
        assert_eq!(
            found.iter().map(|m| (m.start, m.end)).collect::<Vec<_>>(),
            [(5, 8)]
        );
    }

    #[test]
    fn a_swap_changes_the_mark_alone_every_other_character_kept() {
        let text = "Release 🧠\r\n  - ☐ v2 at customer: ✅\n❌ v1\n";
        assert_eq!(
            swap(text, 1, TextMark::Checked).as_deref(),
            Some("Release 🧠\r\n  - ✅ v2 at customer: ✅\n❌ v1\n")
        );
        assert_eq!(
            swap(text, 2, TextMark::Open).as_deref(),
            Some("Release 🧠\r\n  - ☐ v2 at customer: ✅\n☐ v1\n")
        );
    }

    #[test]
    fn a_swap_drops_the_variation_selector_of_the_old_mark() {
        assert_eq!(
            swap("✅\u{FE0F} done", 0, TextMark::Failed).as_deref(),
            Some("❌ done")
        );
        assert_eq!(
            swap("a\n☐\u{FE0E}", 1, TextMark::Checked).as_deref(),
            Some("a\n✅")
        );
    }

    #[test]
    fn a_swap_gives_none_when_there_is_nothing_to_change() {
        let text = "a ✅\n☐ b";
        assert_eq!(swap(text, 0, TextMark::Open), None);
        assert_eq!(swap(text, 1, TextMark::Open), None);
        assert_eq!(swap(text, 2, TextMark::Checked), None);
        assert_eq!(swap("", 0, TextMark::Checked), None);
    }

    #[test]
    fn an_insert_puts_an_open_mark_at_the_start_of_the_line_where_it_is_found() {
        let text = "Title\n  - milk\n\tbread\n\n-tea\n- ";
        let expected = ["☐ Title", "  - ☐ milk", "\t☐ bread", "☐ ", "☐ -tea", "- ☐ "];
        for (i, want) in expected.iter().enumerate() {
            let inserted = insert_open(text, i).unwrap_or_default();
            assert_eq!(split_lines(&inserted)[i], *want);
            assert_eq!(
                find(&inserted).iter().map(|m| m.line).collect::<Vec<_>>(),
                [i]
            );
            assert_eq!(inserted.len(), text.len() + '☐'.len_utf8() + 1);
        }
    }

    #[test]
    fn an_insert_keeps_every_other_character_or_gives_none() {
        assert_eq!(insert_open("a\r\nb\rc", 1).as_deref(), Some("a\r\n☐ b\rc"));
        assert_eq!(insert_open("", 0).as_deref(), Some("☐ "));
        assert_eq!(insert_open("a\n  ✅ b", 1), None);
        assert_eq!(insert_open("a", 1), None);
        // A mark in the middle of the line is none.
        assert_eq!(insert_open("a ✅", 0).as_deref(), Some("☐ a ✅"));
    }

    #[test]
    fn the_line_of_an_offset_counts_every_line_break_once() {
        let text = "a\r\nb\rc\nd";
        assert_eq!(
            (0..=text.len())
                .map(|i| line_at(text, i))
                .collect::<Vec<_>>(),
            [0, 0, 0, 1, 1, 2, 2, 3, 3]
        );
        assert_eq!(line_at("a\nb", 99), 1);
        assert_eq!(line_at("", 0), 0);
        assert_eq!(line_at("a\n", 2), 1);
        // An offset inside a character is taken as it comes.
        assert_eq!(line_at("☐\n☐", 1), 0);
    }
}
