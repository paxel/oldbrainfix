//! Where a search finds what it finds (obf-1047), as the Dart core's
//! `findings.dart` has it: the folding both search and the find bar
//! match by, and the places a query is found in a Note's texts as they
//! show.
//!
//! A text and a query are matched folded ([`fold`]): letter by letter in
//! lower case, by a table both cores carry rather than by their
//! platforms' Unicode, which differ and change with an update, a letter
//! with accents or an umlaut as its letter without them, "ß" as "ss", "æ" as "ae", "œ" as "oe",
//! and the combining accents (U+0300 to U+036F) dropped. So "muller"
//! finds "Müller" and "strasse" "Straße"; "mueller" does not find
//! "Müller", and nothing allows for a typo.
//!
//! A formatted text is matched as it reads without its marks, as search
//! reads it ([`crate::text_format::visible`]), line by line: "Müller"
//! written `**Mül**ler` is found, and a place covers the marks inside it.
//! Offsets are in UTF-8 bytes here, where the Dart core counts UTF-16
//! code units.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use crate::markdown::{self, Span, SpanKind};
use crate::notes::{Attachment, Note, Section};
use crate::text_format;

/// How many characters a [`Finding::context`] holds at most, its "…" not
/// counted, unless the place is longer.
pub const CONTEXT_LENGTH: usize = 80;

/// How many characters a cut [`Finding::context`] shows before the place
/// at most.
pub const CONTEXT_BEFORE: usize = 20;

/// Which of the texts of a Note or of one of its Sections a [`Finding`]
/// lies in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingPart {
    /// The text of the Note or the Section.
    Text,
    /// The Transcript of a voice recording.
    Transcript,
    /// The Photo Text of a photo.
    PhotoText,
    /// The name of an attached file.
    FileName,
    /// The Attachment Text of a file.
    AttachmentText,
}

impl FindingPart {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            FindingPart::Text => "text",
            FindingPart::Transcript => "transcript",
            FindingPart::PhotoText => "photoText",
            FindingPart::FileName => "fileName",
            FindingPart::AttachmentText => "attachmentText",
        }
    }
}

/// The cell of a table a [`Finding`] begins in: the `table` it is among
/// the tables of its text, the top one 0, which begins at the byte
/// `table_start` of the text; its `row`, the header row 0, as
/// [`markdown::Block::row`] counts them; and its `column`, the first 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindingCell {
    pub table: usize,
    pub table_start: usize,
    pub row: usize,
    pub column: usize,
}

/// A place a query is found in a text as it shows: in its `part`, of the
/// recording, photo or file whose hash is `key` (empty for a text), of
/// the Section `section` or, `None`, of the Note itself. It lies from
/// `start` to `end` of that text as it shows (a Note's or a Section's
/// `shown_text()`, a Transcript, Photo Text, Attachment Text or file name
/// as it is), marks inside it included, in bytes, on its `line` counted
/// from 0 as [`markdown::line_at`] counts them; in a table's `cell`,
/// `None` elsewhere. Its `context` is that line as it reads, or a cut of
/// it around the place marked with "…", the place lying in it from the
/// byte `context_start` to `context_end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub part: FindingPart,
    pub key: String,
    pub section: Option<String>,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub cell: Option<FindingCell>,
    pub context: String,
    pub context_start: usize,
    pub context_end: usize,
}

/// `text` folded for matching: each character in lower case, the first
/// of its lower case by [`LOWER_RUNS`], then a letter with accents or
/// an umlaut as its letter without them, "ß" as "ss", "æ" as "ae", "œ" as
/// "oe", and the combining accents dropped.
pub fn fold(text: &str) -> String {
    if text.is_ascii() {
        return text.to_ascii_lowercase();
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        push_folded(&mut out, c);
    }
    out
}

/// The places `query`, trimmed, is found in `shown`, a text as it shows
/// in `format`, top first; none for a query that is blank or folds to
/// nothing. Each is of `part`, `key` and `section`, as the caller says
/// which text `shown` is.
pub fn in_text_of(
    shown: &str,
    format: &str,
    query: &str,
    part: FindingPart,
    key: &str,
    section: Option<&str>,
) -> Vec<Finding> {
    let wanted = fold(query.trim());
    if wanted.is_empty() {
        return Vec::new();
    }
    let lines = if format == text_format::MARKDOWN {
        formatted_lines(shown)
    } else {
        plain_lines(shown)
    };
    let mut found = Vec::new();
    for line in &lines {
        for (first, last) in matches(line, &wanted) {
            found.push(finding(line, first, last, part, key, section));
        }
    }
    found
}

/// The places `query` is found in `shown`, a Note's text as it shows in
/// `format`: [`in_text_of`] for the Note's own text.
pub fn in_text(shown: &str, format: &str, query: &str) -> Vec<Finding> {
    in_text_of(shown, format, query, FindingPart::Text, "", None)
}

/// The places `query` is found in `note`: in its text, its Transcripts
/// (its recordings' order, those removed after them), its Photo Texts
/// (likewise), its files' names and their Attachment Texts (the files'
/// order, those removed after them), then the same of each of
/// `sections`, in their order, as [`in_text_of`] finds them. A text left
/// over in no order goes last, by its hash.
pub fn of_note(note: &Note, query: &str, sections: &[Section]) -> Vec<Finding> {
    let mut found = in_text(&note.shown_text(), &note.text_format, query);
    found.extend(of_media(
        query,
        None,
        Media {
            recordings: [note.recordings.as_slice(), &note.removed_recordings].concat(),
            transcripts: &note.transcripts,
            photos: [note.photos.as_slice(), &note.removed_photos].concat(),
            photo_texts: &note.photo_texts,
            attachments: &note.attachments,
            removed_attachments: &note.removed_attachments,
            attachment_texts: &note.attachment_texts,
        },
    ));
    for s in sections {
        found.extend(in_text_of(
            &s.shown_text(),
            &s.text_format,
            query,
            FindingPart::Text,
            "",
            Some(&s.id),
        ));
        found.extend(of_media(
            query,
            Some(&s.id),
            Media {
                recordings: [s.recordings.as_slice(), &s.removed_recordings].concat(),
                transcripts: &s.transcripts,
                photos: [s.photos.as_slice(), &s.removed_photos].concat(),
                photo_texts: &s.photo_texts,
                attachments: &s.attachments,
                removed_attachments: &s.removed_attachments,
                attachment_texts: &s.attachment_texts,
            },
        ));
    }
    found
}

/// The recordings, photos and files of a Note or a Section with their
/// texts.
struct Media<'a> {
    recordings: Vec<String>,
    transcripts: &'a BTreeMap<String, String>,
    photos: Vec<String>,
    photo_texts: &'a BTreeMap<String, String>,
    attachments: &'a [Attachment],
    removed_attachments: &'a [Attachment],
    attachment_texts: &'a BTreeMap<String, String>,
}

fn of_media(query: &str, section: Option<&str>, media: Media) -> Vec<Finding> {
    let find = |part, key: &str, text: &str| {
        in_text_of(text, text_format::PLAIN, query, part, key, section)
    };
    let mut found = Vec::new();
    for hash in ordered(&media.recordings, media.transcripts) {
        let text = media.transcripts.get(hash).map_or("", String::as_str);
        found.extend(find(FindingPart::Transcript, hash, text));
    }
    for hash in ordered(&media.photos, media.photo_texts) {
        let text = media.photo_texts.get(hash).map_or("", String::as_str);
        found.extend(find(FindingPart::PhotoText, hash, text));
    }
    for file in media.attachments {
        found.extend(find(FindingPart::FileName, &file.hash, &file.name));
    }
    let files: Vec<String> = media
        .attachments
        .iter()
        .chain(media.removed_attachments)
        .map(|f| f.hash.clone())
        .collect();
    for hash in ordered(&files, media.attachment_texts) {
        let text = media.attachment_texts.get(hash).map_or("", String::as_str);
        found.extend(find(FindingPart::AttachmentText, hash, text));
    }
    found
}

/// The keys of `texts` in the order of `order`, then those it lacks by
/// their order as text.
fn ordered<'a>(order: &'a [String], texts: &'a BTreeMap<String, String>) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    let mut keys: Vec<&str> = order
        .iter()
        .filter(|k| texts.contains_key(*k) && seen.insert(k.as_str()))
        .map(String::as_str)
        .collect();
    keys.extend(
        texts
            .keys()
            .filter(|k| !seen.contains(k.as_str()))
            .map(String::as_str),
    );
    keys
}

/// A line of a text as it reads, each of its characters with where it is
/// written, `from` to `to`, and the column of a table's row it lies in.
struct Line {
    line: usize,
    table: Option<usize>,
    table_start: usize,
    row: usize,
    chars: Vec<char>,
    from: Vec<usize>,
    to: Vec<usize>,
    columns: Vec<usize>,
}

impl Line {
    fn new(line: usize) -> Line {
        Line {
            line,
            table: None,
            table_start: 0,
            row: 0,
            chars: Vec::new(),
            from: Vec::new(),
            to: Vec::new(),
            columns: Vec::new(),
        }
    }

    fn add(&mut self, c: char, start: usize, end: usize, column: usize) {
        self.chars.push(c);
        self.from.push(start);
        self.to.push(end);
        self.columns.push(column);
    }
}

/// The runs of `line` that `wanted`, folded and not empty, is found in,
/// as the first and the last of its characters, left to right and none
/// inside another: a match on half of a character's folding takes the
/// whole character, the accents dropped right after its end are its own,
/// and two that would take the same character are one.
fn matches(line: &Line, wanted: &str) -> Vec<(usize, usize)> {
    let mut folded = String::new();
    let mut owner = Vec::new();
    let mut empty = Vec::new();
    for (i, &c) in line.chars.iter().enumerate() {
        let before = folded.len();
        push_folded(&mut folded, c);
        owner.extend(std::iter::repeat_n(i, folded.len() - before));
        empty.push(folded.len() == before);
    }
    let mut found: Vec<(usize, usize)> = Vec::new();
    for (m, _) in folded.match_indices(wanted) {
        let (Some(&first), Some(&end)) = (owner.get(m), owner.get(m + wanted.len() - 1)) else {
            continue;
        };
        let mut last = end;
        while empty.get(last + 1).copied().unwrap_or(false) {
            last += 1;
        }
        match found.last_mut() {
            Some(prev) if first <= prev.1 => prev.1 = prev.1.max(last),
            _ => found.push((first, last)),
        }
    }
    found
}

fn finding(
    line: &Line,
    first: usize,
    last: usize,
    part: FindingPart,
    key: &str,
    section: Option<&str>,
) -> Finding {
    let chars = &line.chars;
    let n = chars.len();
    let (a, b) = (first, last + 1);
    // Whether the character at `i` is a space or a tab.
    let blank_at = |i: usize| chars.get(i).copied().is_some_and(blank);
    let mut lo = 0;
    let mut hi = n;
    while lo < a && blank_at(lo) {
        lo += 1;
    }
    while hi > b && blank_at(hi - 1) {
        hi -= 1;
    }
    let mut from = lo;
    let mut to = hi;
    if hi - lo > CONTEXT_LENGTH {
        from = lo.max(a.saturating_sub(CONTEXT_BEFORE));
        to = hi.min(b.max(from + CONTEXT_LENGTH));
        if to - from < CONTEXT_LENGTH {
            from = lo.max(to.saturating_sub(CONTEXT_LENGTH));
        }
        // Whole words at a cut.
        if from > lo
            && !blank_at(from - 1)
            && let Some(i) = (from..a).find(|&i| blank_at(i))
        {
            from = i + 1;
        }
        if to < hi
            && !blank_at(to)
            && let Some(i) = (b..to).rev().find(|&i| blank_at(i))
        {
            to = i;
        }
        while from < a && blank_at(from) {
            from += 1;
        }
        while to > b && blank_at(to - 1) {
            to -= 1;
        }
    }
    let text = |range: std::ops::Range<usize>| -> String {
        chars.get(range).unwrap_or_default().iter().collect()
    };
    let before = text(from..a);
    let hit = text(a..b);
    let after = text(b..to);
    let open = if from > lo { "…" } else { "" };
    let close = if to < hi { "…" } else { "" };
    let start = line.from.get(a).copied().unwrap_or_default();
    Finding {
        part,
        key: key.to_string(),
        section: section.map(str::to_string),
        start,
        end: start.max(line.to.get(last).copied().unwrap_or_default()),
        line: line.line,
        cell: line.table.map(|table| FindingCell {
            table,
            table_start: line.table_start,
            row: line.row,
            column: line.columns.get(a).copied().unwrap_or_default(),
        }),
        context_start: open.len() + before.len(),
        context_end: open.len() + before.len() + hit.len(),
        context: format!("{open}{before}{hit}{after}{close}"),
    }
}

fn blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The lines of a text that is not formatted, each character as it is.
fn plain_lines(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut line = Line::new(0);
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek().is_some_and(|&(_, d)| d == '\n') {
                chars.next();
            }
            lines.push(line);
            line = Line::new(lines.len());
            continue;
        }
        line.add(c, i, i + c.len_utf8(), 0);
    }
    lines.push(line);
    lines
}

/// The lines of a formatted text as they read without their marks,
/// [`markdown::Block::visible`], each character with where it is
/// written: a table's row its cells that show something, a space between
/// them, that space lying between the two cells; no line for a table's
/// delimiter row, the line marking its first column as headings, or a
/// code block's fence.
fn formatted_lines(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut tables: HashMap<usize, usize> = HashMap::new();
    for block in markdown::parse(text) {
        if let Some(table) = &block.table {
            let Some(row) = block.row else {
                continue;
            };
            let count = tables.len();
            let mut line = Line::new(block.line);
            line.table = Some(*tables.entry(table.start).or_insert(count));
            line.table_start = table.start;
            line.row = row;
            let mut after = None;
            for (column, cell) in table
                .rows
                .get(row)
                .map_or(&[][..], |r| r.cells.as_slice())
                .iter()
                .enumerate()
            {
                if cell.visible().is_empty() {
                    continue;
                }
                if let Some(after) = after {
                    line.add(' ', after, cell.start, column);
                }
                for span in &cell.spans {
                    span_chars(text, span, &mut line, column);
                }
                after = Some(cell.end);
            }
            lines.push(line);
            continue;
        }
        if block
            .code
            .as_ref()
            .is_some_and(|code| code.is_fence(block.line))
        {
            continue;
        }
        let mut line = Line::new(block.line);
        for span in &block.spans {
            span_chars(text, span, &mut line, 0);
        }
        lines.push(line);
    }
    lines
}

/// The characters `span` shows, each with where it is written.
fn span_chars(text: &str, span: &Span, line: &mut Line, column: usize) {
    match span.kind {
        SpanKind::Text if span.content_start == span.start => {
            let mut at = span.start;
            for c in span.text.chars() {
                line.add(c, at, at + c.len_utf8(), column);
                at += c.len_utf8();
            }
        }
        SpanKind::Text => {
            // One escaped character: the backslash is its own.
            for c in span.text.chars() {
                line.add(c, span.start, span.end, column);
            }
        }
        SpanKind::Code => {
            // Its characters as written, but for a space dropped at both
            // ends and a `\|` in a cell showing `|`.
            let content = text.get(..span.content_end).unwrap_or_default();
            let mut at = span.content_start;
            for c in span.text.chars() {
                let mut q = at;
                let mut escaped = false;
                let mut found = false;
                while let Some(here) = content.get(q..).and_then(|t| t.chars().next()) {
                    if here == c {
                        found = true;
                        break;
                    }
                    if here == '\\'
                        && content
                            .get(q + 1..)
                            .and_then(|t| t.chars().next())
                            .is_some_and(|next| next == c)
                    {
                        escaped = true;
                        found = true;
                        break;
                    }
                    q += here.len_utf8();
                }
                if !found {
                    line.add(c, span.start, span.end, column);
                    continue;
                }
                let end = q + c.len_utf8() + usize::from(escaped);
                line.add(c, q, end, column);
                at = end;
            }
        }
        _ => {
            for child in &span.children {
                span_chars(text, child, line, column);
            }
        }
    }
}

/// Writes what `c` folds to.
fn push_folded(out: &mut String, c: char) {
    if c.is_ascii() {
        out.push(c.to_ascii_lowercase());
        return;
    }
    let lower = lower(c);
    if ('\u{300}'..='\u{36F}').contains(&lower) {
        return;
    }
    match lower {
        // The letters that fold to two.
        'ß' | 'ẞ' => out.push_str("ss"),
        'æ' | 'Æ' => out.push_str("ae"),
        'œ' | 'Œ' => out.push_str("oe"),
        _ => out.push(bases().get(&lower).copied().unwrap_or(lower)),
    }
}

/// The lower case of `c` by [`LOWER_RUNS`], `c` itself when it has none.
fn lower(c: char) -> char {
    let code = u32::from(c);
    let at = LOWER_RUNS.partition_point(|&(start, ..)| start <= code);
    match at.checked_sub(1).and_then(|i| LOWER_RUNS.get(i)) {
        Some(&(start, end, step, delta)) if code <= end && (code - start) % step == 0 => {
            char::from_u32(code.wrapping_add_signed(delta)).unwrap_or(c)
        }
        _ => c,
    }
}

/// The letter each letter with accents folds to, by [`FOLD_RUNS`].
fn bases() -> &'static HashMap<char, char> {
    static BASES: OnceLock<HashMap<char, char>> = OnceLock::new();
    BASES.get_or_init(|| {
        let mut bases = HashMap::new();
        for (start, letters) in FOLD_RUNS {
            for (i, letter) in (0u32..).zip(letters.chars()) {
                if letter != '.'
                    && let Some(c) = char::from_u32(start + i)
                {
                    bases.insert(c, letter);
                }
            }
        }
        bases
    })
}

/// The letters with accents and umlauts and the letters they fold to, the
/// same in both cores (`findings.dart`): Latin and Greek letters whose
/// canonical decomposition is a letter and combining accents, and the
/// letters with a stroke ø, đ, ħ, ł and ŧ and the dotless ı, each to a
/// letter in lower case, upper and lower case alike. Each run gives the
/// letter of each character from its start on, `.` for one folding to
/// itself.
const FOLD_RUNS: [(u32, &str); 3] = [
    (
        0xC0,
        concat!(
            "aaaaaa.ceeeeiiii.nooooo.ouuuuy..aaaaaa.ceeeeiiii.nooooo.ouuu",
            "uy.yaaaaaaccccccccddddeeeeeeeeeegggggggghhhhiiiiiiiiii..jjkk",
            ".llllll..llnnnnnn...oooooo..rrrrrrssssssssttttttuuuuuuuuuuuu",
            "wwyyyzzzzzz.................................oo.............u",
            "u............................aaiioouuuuuuuuuu.aaaa....ggkkoo",
            "oo..j...gg..nnaa....aaaaeeeeiiiioooorrrruuuusstt..hh......aa",
            "eeooooooooyy",
        ),
    ),
    (
        0x370,
        concat!(
            "......................α.εηι.ο.υωι.........................ιυ",
            "αεηιυ.........................ιυουω",
        ),
    ),
    (
        0x1E00,
        concat!(
            "aabbbbbbccddddddddddeeeeeeeeeeffgghhhhhhhhhhiiiikkkkkkllllll",
            "llmmmmmmnnnnnnnnoooooooopppprrrrrrrrssssssssssttttttttuuuuuu",
            "uuuuvvvvwwwwwwwwwwxxxxyyzzzzzzhtwy......aaaaaaaaaaaaaaaaaaaa",
            "aaaaeeeeeeeeeeeeeeeeiiiioooooooooooooooooooooooouuuuuuuuuuuu",
            "uuyyyyyyyy",
        ),
    ),
];

/// The lower case of every character that has one, the first character
/// of it where it is more than one ("İ" to "i"), as Unicode 18 has it
/// (Rust 1.100), the same in both cores (`findings.dart`), rather than
/// by the platforms' Unicode, which differ and change with an update:
/// runs from a character to another, every one or every other, each
/// lowered by the same distance.
const LOWER_RUNS: [(u32, u32, u32, i32); 194] = [
    (0xC0, 0xD6, 1, 32),
    (0xD8, 0xDE, 1, 32),
    (0x100, 0x12E, 2, 1),
    (0x130, 0x130, 1, -199),
    (0x132, 0x136, 2, 1),
    (0x139, 0x147, 2, 1),
    (0x14A, 0x176, 2, 1),
    (0x178, 0x178, 1, -121),
    (0x179, 0x17D, 2, 1),
    (0x181, 0x181, 1, 210),
    (0x182, 0x184, 2, 1),
    (0x186, 0x186, 1, 206),
    (0x187, 0x187, 1, 1),
    (0x189, 0x18A, 1, 205),
    (0x18B, 0x18B, 1, 1),
    (0x18E, 0x18E, 1, 79),
    (0x18F, 0x18F, 1, 202),
    (0x190, 0x190, 1, 203),
    (0x191, 0x191, 1, 1),
    (0x193, 0x193, 1, 205),
    (0x194, 0x194, 1, 207),
    (0x196, 0x196, 1, 211),
    (0x197, 0x197, 1, 209),
    (0x198, 0x198, 1, 1),
    (0x19C, 0x19C, 1, 211),
    (0x19D, 0x19D, 1, 213),
    (0x19F, 0x19F, 1, 214),
    (0x1A0, 0x1A4, 2, 1),
    (0x1A6, 0x1A6, 1, 218),
    (0x1A7, 0x1A7, 1, 1),
    (0x1A9, 0x1A9, 1, 218),
    (0x1AC, 0x1AC, 1, 1),
    (0x1AE, 0x1AE, 1, 218),
    (0x1AF, 0x1AF, 1, 1),
    (0x1B1, 0x1B2, 1, 217),
    (0x1B3, 0x1B5, 2, 1),
    (0x1B7, 0x1B7, 1, 219),
    (0x1B8, 0x1B8, 1, 1),
    (0x1BC, 0x1BC, 1, 1),
    (0x1C4, 0x1C4, 1, 2),
    (0x1C5, 0x1C5, 1, 1),
    (0x1C7, 0x1C7, 1, 2),
    (0x1C8, 0x1C8, 1, 1),
    (0x1CA, 0x1CA, 1, 2),
    (0x1CB, 0x1DB, 2, 1),
    (0x1DE, 0x1EE, 2, 1),
    (0x1F1, 0x1F1, 1, 2),
    (0x1F2, 0x1F4, 2, 1),
    (0x1F6, 0x1F6, 1, -97),
    (0x1F7, 0x1F7, 1, -56),
    (0x1F8, 0x21E, 2, 1),
    (0x220, 0x220, 1, -130),
    (0x222, 0x232, 2, 1),
    (0x23A, 0x23A, 1, 10795),
    (0x23B, 0x23B, 1, 1),
    (0x23D, 0x23D, 1, -163),
    (0x23E, 0x23E, 1, 10792),
    (0x241, 0x241, 1, 1),
    (0x243, 0x243, 1, -195),
    (0x244, 0x244, 1, 69),
    (0x245, 0x245, 1, 71),
    (0x246, 0x24E, 2, 1),
    (0x370, 0x372, 2, 1),
    (0x376, 0x376, 1, 1),
    (0x37F, 0x37F, 1, 116),
    (0x386, 0x386, 1, 38),
    (0x388, 0x38A, 1, 37),
    (0x38C, 0x38C, 1, 64),
    (0x38E, 0x38F, 1, 63),
    (0x391, 0x3A1, 1, 32),
    (0x3A3, 0x3AB, 1, 32),
    (0x3CF, 0x3CF, 1, 8),
    (0x3D8, 0x3EE, 2, 1),
    (0x3F4, 0x3F4, 1, -60),
    (0x3F7, 0x3F7, 1, 1),
    (0x3F9, 0x3F9, 1, -7),
    (0x3FA, 0x3FA, 1, 1),
    (0x3FD, 0x3FF, 1, -130),
    (0x400, 0x40F, 1, 80),
    (0x410, 0x42F, 1, 32),
    (0x460, 0x480, 2, 1),
    (0x48A, 0x4BE, 2, 1),
    (0x4C0, 0x4C0, 1, 15),
    (0x4C1, 0x4CD, 2, 1),
    (0x4D0, 0x52E, 2, 1),
    (0x531, 0x556, 1, 48),
    (0x10A0, 0x10C5, 1, 7264),
    (0x10C7, 0x10C7, 1, 7264),
    (0x10CD, 0x10CD, 1, 7264),
    (0x13A0, 0x13EF, 1, 38864),
    (0x13F0, 0x13F5, 1, 8),
    (0x1C89, 0x1C89, 1, 1),
    (0x1C90, 0x1CBA, 1, -3008),
    (0x1CBD, 0x1CBF, 1, -3008),
    (0x1E00, 0x1E94, 2, 1),
    (0x1E9E, 0x1E9E, 1, -7615),
    (0x1EA0, 0x1EFE, 2, 1),
    (0x1F08, 0x1F0F, 1, -8),
    (0x1F18, 0x1F1D, 1, -8),
    (0x1F28, 0x1F2F, 1, -8),
    (0x1F38, 0x1F3F, 1, -8),
    (0x1F48, 0x1F4D, 1, -8),
    (0x1F59, 0x1F5F, 2, -8),
    (0x1F68, 0x1F6F, 1, -8),
    (0x1F88, 0x1F8F, 1, -8),
    (0x1F98, 0x1F9F, 1, -8),
    (0x1FA8, 0x1FAF, 1, -8),
    (0x1FB8, 0x1FB9, 1, -8),
    (0x1FBA, 0x1FBB, 1, -74),
    (0x1FBC, 0x1FBC, 1, -9),
    (0x1FC8, 0x1FCB, 1, -86),
    (0x1FCC, 0x1FCC, 1, -9),
    (0x1FD8, 0x1FD9, 1, -8),
    (0x1FDA, 0x1FDB, 1, -100),
    (0x1FE8, 0x1FE9, 1, -8),
    (0x1FEA, 0x1FEB, 1, -112),
    (0x1FEC, 0x1FEC, 1, -7),
    (0x1FF8, 0x1FF9, 1, -128),
    (0x1FFA, 0x1FFB, 1, -126),
    (0x1FFC, 0x1FFC, 1, -9),
    (0x2126, 0x2126, 1, -7517),
    (0x212A, 0x212A, 1, -8383),
    (0x212B, 0x212B, 1, -8262),
    (0x2132, 0x2132, 1, 28),
    (0x2160, 0x216F, 1, 16),
    (0x2183, 0x2183, 1, 1),
    (0x24B6, 0x24CF, 1, 26),
    (0x2C00, 0x2C2F, 1, 48),
    (0x2C60, 0x2C60, 1, 1),
    (0x2C62, 0x2C62, 1, -10743),
    (0x2C63, 0x2C63, 1, -3814),
    (0x2C64, 0x2C64, 1, -10727),
    (0x2C67, 0x2C6B, 2, 1),
    (0x2C6D, 0x2C6D, 1, -10780),
    (0x2C6E, 0x2C6E, 1, -10749),
    (0x2C6F, 0x2C6F, 1, -10783),
    (0x2C70, 0x2C70, 1, -10782),
    (0x2C72, 0x2C72, 1, 1),
    (0x2C75, 0x2C75, 1, 1),
    (0x2C7E, 0x2C7F, 1, -10815),
    (0x2C80, 0x2CE2, 2, 1),
    (0x2CEB, 0x2CED, 2, 1),
    (0x2CF2, 0x2CF2, 1, 1),
    (0xA640, 0xA66C, 2, 1),
    (0xA680, 0xA69A, 2, 1),
    (0xA722, 0xA72E, 2, 1),
    (0xA732, 0xA76E, 2, 1),
    (0xA779, 0xA77B, 2, 1),
    (0xA77D, 0xA77D, 1, -35332),
    (0xA77E, 0xA786, 2, 1),
    (0xA78B, 0xA78B, 1, 1),
    (0xA78D, 0xA78D, 1, -42280),
    (0xA790, 0xA792, 2, 1),
    (0xA796, 0xA7A8, 2, 1),
    (0xA7AA, 0xA7AA, 1, -42308),
    (0xA7AB, 0xA7AB, 1, -42319),
    (0xA7AC, 0xA7AC, 1, -42315),
    (0xA7AD, 0xA7AD, 1, -42305),
    (0xA7AE, 0xA7AE, 1, -42308),
    (0xA7B0, 0xA7B0, 1, -42258),
    (0xA7B1, 0xA7B1, 1, -42282),
    (0xA7B2, 0xA7B2, 1, -42261),
    (0xA7B3, 0xA7B3, 1, 928),
    (0xA7B4, 0xA7C2, 2, 1),
    (0xA7C4, 0xA7C4, 1, -48),
    (0xA7C5, 0xA7C5, 1, -42307),
    (0xA7C6, 0xA7C6, 1, -35384),
    (0xA7C7, 0xA7C9, 2, 1),
    (0xA7CB, 0xA7CB, 1, -42343),
    (0xA7CC, 0xA7DA, 2, 1),
    (0xA7DC, 0xA7DC, 1, -42561),
    (0xA7DD, 0xA7DD, 1, -42342),
    (0xA7E2, 0xA7E2, 1, -42342),
    (0xA7F5, 0xA7F5, 1, 1),
    (0xAB6C, 0xAB6D, 1, -33),
    (0xFF21, 0xFF3A, 1, 32),
    (0x10400, 0x10427, 1, 40),
    (0x104B0, 0x104D3, 1, 40),
    (0x10570, 0x1057A, 1, 39),
    (0x1057C, 0x1058A, 1, 39),
    (0x1058C, 0x10592, 1, 39),
    (0x10594, 0x10595, 1, 39),
    (0x10C80, 0x10CB2, 1, 64),
    (0x10D50, 0x10D65, 1, 32),
    (0x118A0, 0x118BF, 1, 32),
    (0x16E40, 0x16E5F, 1, 32),
    (0x16EA0, 0x16EB8, 1, 27),
    (0x1DF40, 0x1DF40, 1, 1),
    (0x1DF48, 0x1DF4A, 2, 1),
    (0x1DF4D, 0x1DF4D, 1, 1),
    (0x1DF51, 0x1DF51, 1, 1),
    (0x1DF68, 0x1DF6E, 2, 1),
    (0x1DF72, 0x1DF7E, 2, 1),
    (0x1E900, 0x1E921, 1, 34),
];

#[cfg(test)]
mod tests;
