//! Formatted text (ADR 0033), as the Dart core's `markdown.dart` reads
//! and edits it: the subset of Markdown a Note's or a Section's text in
//! [`crate::text_format::MARKDOWN`] is read in, the visible text without
//! its marks, and the edits the editors make. Offsets are in UTF-8 bytes
//! here, where the Dart core counts UTF-16 code units.
//!
//! The text is kept as it was written; nothing here rewrites it, and
//! whatever the subset does not know shows as the characters it is.
//! Every line is a block of its own, so a line break always shows as
//! one. A line is, the first that fits:
//!
//! - a heading: up to three spaces, `#` or `##`, then a space, a tab or
//!   the line's end;
//! - a quote: up to three spaces and `>`, one space or tab after it
//!   belonging to the mark;
//! - a bulleted item: any spaces and tabs, `-`, `*` or `+`, then a space,
//!   a tab or the line's end;
//! - a numbered item: any spaces and tabs, one to nine digits and `.`,
//!   then a space, a tab or the line's end;
//! - a paragraph line otherwise, its spaces at the start included.
//!
//! Spaces count one and a tab four; an item indented by two or more is
//! one level in, deeper ones too. The spaces and tabs after an item's or
//! a heading's mark belong to the mark. A numbered item right after one
//! of its level, with only items further in between, shows the number
//! after that one's; any other shows its own.
//!
//! Within a line: `\` before ASCII punctuation shows that character; a
//! run of backticks up to the next run of as many is code, shown as it
//! is, one space at both ends dropped when there is one at both; `[`
//! with its `]` and `(destination)` right after it is a link, its
//! destination without spaces, any parentheses in it balanced, and no
//! link inside; `*` and `_` runs are emphasis by CommonMark's flanking
//! rules, ASCII punctuation counting as punctuation and the start and
//! end of the line as space, one for italic and two for bold; a run of
//! exactly two `~` is strike-through, by the flanking rules of `*`. A
//! `[` right after a `!` is no link, so an image shows as written.
//!
//! A table (GFM, #128) takes a line of its own per row: a header row,
//! the delimiter row under it, `| --- | :-: |`, with as many cells, and
//! the rows under those; [`table`] reads and edits them, every line of
//! one a [`BlockKind::Table`] block.
//!
//! A fenced code block (#212) is CommonMark's: a line of up to three
//! spaces and at least three backticks or tildes opens it, an info string
//! after them, whose first word is the language; it runs to the first
//! line of up to three spaces and at least as many of the same character
//! with only spaces and tabs after them, or to the end of the text. Its
//! lines are [`BlockKind::Code`] blocks, shown as written; nothing in
//! them is read as a format, a table, a tag or a box.

pub mod table;

use std::sync::Arc;

use crate::text_tags::{self, FoundTag};
use crate::{Error, Result};
use table::Table;

/// What a line of a formatted text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading1,
    Heading2,
    Bullet,
    Numbered,
    Quote,
    /// A line of a table: its header row, its delimiter row, one of its
    /// rows, or the line marking its first column as headings.
    Table,
    /// A line of a fenced code block: one of its fences or a line between
    /// them, shown as written.
    Code,
}

impl BlockKind {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            BlockKind::Paragraph => "paragraph",
            BlockKind::Heading1 => "heading1",
            BlockKind::Heading2 => "heading2",
            BlockKind::Bullet => "bullet",
            BlockKind::Numbered => "numbered",
            BlockKind::Quote => "quote",
            BlockKind::Table => "table",
            BlockKind::Code => "code",
        }
    }

    fn listed(self) -> bool {
        matches!(self, BlockKind::Bullet | BlockKind::Numbered)
    }
}

/// What a part of a line is: `Text` and `Code` hold what shows, the
/// others the parts inside them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    Text,
    Bold,
    Italic,
    Strike,
    Code,
    Link,
}

impl SpanKind {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            SpanKind::Text => "text",
            SpanKind::Bold => "bold",
            SpanKind::Italic => "italic",
            SpanKind::Strike => "strike",
            SpanKind::Code => "code",
            SpanKind::Link => "link",
        }
    }
}

/// A part of a line: its kind, where it lies in the text, `start..end`
/// with its marks, and its content between them,
/// `content_start..content_end`: a link's label, the characters between
/// a code part's backticks. Offsets are bytes of the whole text.
///
/// A `Text` part is either a run of characters shown as written, `text`
/// the same as the source `start..end`, or one escaped character, `\*`
/// showing `*`, its `content_start` one after `start`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub kind: SpanKind,
    pub start: usize,
    pub end: usize,
    pub content_start: usize,
    pub content_end: usize,
    /// What a text or a code part shows; empty for the others.
    pub text: String,
    /// Where a link goes, its escapes resolved, as written by whoever
    /// wrote the text; empty for the others.
    pub url: String,
    /// The parts inside bold, italic, strike-through and a link's label.
    pub children: Vec<Span>,
}

impl Span {
    /// The text this part shows, without its marks.
    pub fn visible(&self) -> String {
        match self.kind {
            SpanKind::Text | SpanKind::Code => self.text.clone(),
            _ => self.children.iter().map(Span::visible).collect(),
        }
    }

    fn literal(text: &str, start: usize, end: usize) -> Span {
        Span {
            kind: SpanKind::Text,
            start,
            end,
            content_start: start,
            content_end: end,
            text: text.get(start..end).unwrap_or_default().to_string(),
            url: String::new(),
            children: Vec::new(),
        }
    }

    /// Whether it is a run of characters shown as written.
    fn written(&self) -> bool {
        self.kind == SpanKind::Text && self.content_start == self.start
    }
}

/// A fenced code block: where it lies in the text, `start..end`, its
/// fences included, on the lines from `first_line`, its opening fence, to
/// `last_line`, its closing fence when `closed`, else the text's last
/// line. Its `fence` is the run of backticks or tildes that opened it,
/// its `info` the info string after that without the spaces and tabs
/// around it, as written. Offsets are bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub start: usize,
    pub end: usize,
    pub first_line: usize,
    pub last_line: usize,
    pub fence: String,
    pub info: String,
    pub closed: bool,
}

impl CodeBlock {
    /// The first word of `info`, what a highlighter would go by; empty
    /// when there is none.
    pub fn language(&self) -> &str {
        self.info.split([' ', '\t']).next().unwrap_or_default()
    }

    /// Whether the line `line` is one of its fences rather than a line of
    /// its content.
    pub fn is_fence(&self, line: usize) -> bool {
        line == self.first_line || (self.closed && line == self.last_line)
    }
}

/// A line of a formatted text: its kind, the line it is, counted as
/// [`line_at`] counts lines, where it lies in the text, `start..end`
/// without its line break, and where its content begins after the mark,
/// `content_start`. Offsets are bytes.
///
/// A line of a table holds the table it belongs to and, a row of it, its
/// row there, the header row 0; its spans are those of its cells, left to
/// right. The delimiter row and the line marking the first column as
/// headings have no row.
///
/// A line of a fenced code block holds the code block it belongs to; a
/// line of its content is one part shown as written, a fence none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub line: usize,
    /// How far in an item is: 0 or 1; 0 for any other line.
    pub level: usize,
    /// The number a numbered item shows; 0 for any other line.
    pub number: u64,
    pub start: usize,
    pub end: usize,
    pub content_start: usize,
    pub spans: Vec<Span>,
    pub table: Option<Arc<Table>>,
    pub row: Option<usize>,
    pub code: Option<Arc<CodeBlock>>,
}

impl Block {
    /// The text this line shows, without its marks: a table's row its
    /// cells that show something, a space between them, its other lines
    /// nothing; a code block's fence nothing.
    pub fn visible(&self) -> String {
        match (&self.table, self.row) {
            (Some(table), Some(row)) => table
                .rows
                .get(row)
                .map(table::Row::visible)
                .unwrap_or_default(),
            (Some(_), None) => String::new(),
            _ if self.is_fence() => String::new(),
            _ => self.spans.iter().map(Span::visible).collect(),
        }
    }

    /// Whether it is one of a code block's fences.
    fn is_fence(&self) -> bool {
        self.code.as_ref().is_some_and(|c| c.is_fence(self.line))
    }

    /// Whether it shows nothing and goes with a line break: a table's
    /// delimiter row and the line marking its first column as headings,
    /// and a code block's fence.
    fn hidden(&self) -> bool {
        (self.table.is_some() && self.row.is_none()) || self.is_fence()
    }
}

/// A text after an edit and the selection in it, in bytes; `start` and
/// `end` are equal for a cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Every line of `text` as a block, top line first.
pub fn parse(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let lines = lines(text);
    let mut i = 0;
    while let Some(&(start, end)) = lines.get(i) {
        if let Some(code) = code_at(text, &lines, i) {
            i = code.last_line + 1;
            blocks.extend(code_blocks(text, code, &lines));
            continue;
        }
        if let Some(table) = table::table_at(text, &lines, i) {
            i = table.last_line + 1;
            blocks.extend(table::blocks(table, &lines));
            continue;
        }
        let (kind, level, written, content) = block(text, start, end);
        let number = if kind == BlockKind::Numbered {
            match number_after(&blocks, blocks.len(), level) {
                0 => written,
                n => n,
            }
        } else {
            0
        };
        let spans = Inline {
            text,
            from: content,
            to: end,
            links: true,
            cell: false,
        }
        .parse();
        blocks.push(Block {
            kind,
            line: i,
            level,
            number,
            start,
            end,
            content_start: content,
            spans,
            table: None,
            row: None,
            code: None,
        });
        i += 1;
    }
    blocks
}

/// `text` as it reads without its marks, its line breaks as they are:
/// what search looks in and what a preview shows. A table's delimiter row
/// and the line marking its first column as headings go with a line
/// break; each of its rows reads as its cells that show something, a
/// space between them. So does a code block's fence, its other lines
/// reading as written.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    let blocks = parse(text);
    for (i, block) in blocks.iter().enumerate() {
        if block.hidden() {
            // The line break before it goes, or the one after the first
            // line.
            at = if i == 0 {
                blocks.get(1).map_or(block.end, |b| b.start)
            } else {
                block.end
            };
            continue;
        }
        out.push_str(slice(text, at, block.start));
        out.push_str(&block.visible());
        at = block.end;
    }
    out
}

/// `text` as HTML, for an app that shows formatted text another app
/// hands it (#129), the same in both cores. Lines in a row of paragraph
/// lines are one `<p>`, of quote lines one `<blockquote>`, with `<br>`
/// between them; a line of only spaces and tabs ends either and shows
/// nothing. Headings are `<h1>` and `<h2>`. Items in a row are one `<ul>`
/// or `<ol>`, `start` its first number unless that is 1, those one level
/// in a list inside the item above them; another kind of item starts a
/// list of its own. A table is a `<table>`, its header row in `<thead>`
/// with `<th>` cells, the first cell of each other row a `<th>` too when
/// its first column holds headings, a column's alignment as `align`. A
/// code block is a `<pre><code>`, `class="language-…"` its language when
/// it has one, each line of its content followed by a line break. Bold
/// is `<strong>`, italic `<em>`, strike-through `<del>`, code
/// `<code>`; a link is an `<a>` only when [`may_open`] allows its
/// destination, else its label alone. Every character of the text is
/// escaped, so nothing in it is markup; status marks and tags are text. A
/// line break follows each part at the top.
pub fn html(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut html = Html::default();
    let blocks = parse(text);
    for block in &blocks {
        match block.kind {
            BlockKind::Code => {
                let Some(code) = &block.code else { continue };
                if block.line != code.first_line {
                    continue;
                }
                html.close();
                match code.language() {
                    "" => html.out.push_str("<pre><code>"),
                    language => {
                        html.out.push_str("<pre><code class=\"language-");
                        html_escape(&mut html.out, language);
                        html.out.push_str("\">");
                    }
                }
                for line in code.first_line + 1..=code.last_line {
                    if code.is_fence(line) {
                        continue;
                    }
                    if let Some(b) = blocks.get(line) {
                        html_escape(&mut html.out, slice(text, b.start, b.end));
                    }
                    html.out.push('\n');
                }
                html.out.push_str("</code></pre>\n");
            }
            BlockKind::Table => {
                let Some(table) = &block.table else { continue };
                if block.line != table.first_line {
                    continue;
                }
                html.close();
                html_table(&mut html.out, table);
            }
            BlockKind::Paragraph => {
                if skip_blank(bytes, block.start, block.end) == block.end {
                    html.close();
                    continue;
                }
                html.open_as("p");
                html_spans(&mut html.out, &block.spans);
            }
            BlockKind::Quote => {
                html.open_as("blockquote");
                html_spans(&mut html.out, &block.spans);
            }
            BlockKind::Heading1 | BlockKind::Heading2 => {
                html.close();
                let tag = if block.kind == BlockKind::Heading1 {
                    "h1"
                } else {
                    "h2"
                };
                html.out.push_str(&format!("<{tag}>"));
                html_spans(&mut html.out, &block.spans);
                html.out.push_str(&format!("</{tag}>\n"));
            }
            BlockKind::Bullet | BlockKind::Numbered => {
                if html.open.is_some() {
                    html.close();
                }
                // One level in only inside an item.
                let level = block.level.min(html.lists.len());
                html.close_lists(level + 1);
                if html.lists.len() == level + 1 && html.lists[level] == block.kind {
                    html.out.push_str("</li><li>");
                } else {
                    html.close_lists(level);
                    match block.kind {
                        BlockKind::Numbered if block.number != 1 => {
                            html.out
                                .push_str(&format!("<ol start=\"{}\"><li>", block.number));
                        }
                        BlockKind::Numbered => html.out.push_str("<ol><li>"),
                        _ => html.out.push_str("<ul><li>"),
                    }
                    html.lists.push(block.kind);
                }
                html_spans(&mut html.out, &block.spans);
            }
        }
    }
    html.close();
    html.out
}

/// What [`html`] has written and has open: the paragraph or quote, and
/// the kinds of the lists, the outer one first, each with an item open.
#[derive(Default)]
struct Html {
    out: String,
    open: Option<&'static str>,
    lists: Vec<BlockKind>,
}

impl Html {
    fn close_lists(&mut self, keep: usize) {
        while self.lists.len() > keep {
            let kind = self.lists.pop();
            self.out.push_str(if kind == Some(BlockKind::Numbered) {
                "</li></ol>"
            } else {
                "</li></ul>"
            });
            if self.lists.is_empty() {
                self.out.push('\n');
            }
        }
    }

    fn close(&mut self) {
        if let Some(tag) = self.open.take() {
            self.out.push_str(&format!("</{tag}>\n"));
        }
        self.close_lists(0);
    }

    fn open_as(&mut self, tag: &'static str) {
        if self.open == Some(tag) {
            self.out.push_str("<br>");
        } else {
            self.close();
            self.out.push_str(&format!("<{tag}>"));
            self.open = Some(tag);
        }
    }
}

fn html_table(out: &mut String, table: &Table) {
    let row = |out: &mut String, row: &table::Row, header: bool| {
        out.push_str("<tr>");
        for (c, cell) in row.cells.iter().enumerate() {
            let tag = if header || (c == 0 && table.heading_column) {
                "th"
            } else {
                "td"
            };
            match table.aligns.get(c).copied().unwrap_or(table::Align::None) {
                table::Align::None => out.push_str(&format!("<{tag}>")),
                align => out.push_str(&format!("<{tag} align=\"{}\">", align.name())),
            }
            html_spans(out, &cell.spans);
            out.push_str(&format!("</{tag}>"));
        }
        out.push_str("</tr>");
    };
    out.push_str("<table><thead>");
    if let Some(header) = table.rows.first() {
        row(&mut *out, header, true);
    }
    out.push_str("</thead>");
    if table.rows.len() > 1 {
        out.push_str("<tbody>");
        for r in table.rows.iter().skip(1) {
            row(&mut *out, r, false);
        }
        out.push_str("</tbody>");
    }
    out.push_str("</table>\n");
}

fn html_spans(out: &mut String, spans: &[Span]) {
    for s in spans {
        match s.kind {
            SpanKind::Text => html_escape(out, &s.text),
            SpanKind::Code => {
                out.push_str("<code>");
                html_escape(out, &s.text);
                out.push_str("</code>");
            }
            SpanKind::Link if may_open(&s.url) => {
                out.push_str("<a href=\"");
                html_escape(out, &s.url);
                out.push_str("\">");
                html_spans(out, &s.children);
                out.push_str("</a>");
            }
            SpanKind::Link => html_spans(out, &s.children),
            SpanKind::Bold | SpanKind::Italic | SpanKind::Strike => {
                let tag = match s.kind {
                    SpanKind::Bold => "strong",
                    SpanKind::Italic => "em",
                    _ => "del",
                };
                out.push_str(&format!("<{tag}>"));
                html_spans(out, &s.children);
                out.push_str(&format!("</{tag}>"));
            }
        }
    }
}

/// `text` with `&`, `<`, `>`, `"` and `'` written as HTML references.
fn html_escape(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

/// Whether `text` uses any format: a line other than a paragraph line,
/// or bold, italic, strike-through, code or a link. An escaped character
/// is none.
pub fn uses(text: &str) -> bool {
    parse(text)
        .iter()
        .any(|b| b.kind != BlockKind::Paragraph || b.spans.iter().any(|s| s.kind != SpanKind::Text))
}

/// `text`, taken as plain text, written so that it reads as itself:
/// [`plain`] of the result is `text`, and it uses no format. For a
/// Transcript or Photo Text put into a formatted text. A `\` goes before
/// a line's mark, every `*`, `` ` ``, `[` and `|`, a `~` next to another,
/// a `_` not between two characters that are neither space nor
/// punctuation, and a `\` before ASCII punctuation. With every `|`
/// escaped no line is a table's, so a line's mark is the one it has as a
/// line of its own.
pub fn escape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut at = 0;
    for block in parse(text) {
        out.push_str(slice(text, at, block.start));
        let mark = match self::block(text, block.start, block.end).0 {
            BlockKind::Paragraph => None,
            BlockKind::Numbered => text
                .get(block.start..)
                .and_then(|rest| rest.find('.'))
                .map(|i| block.start + i),
            _ => Some(skip_blank(bytes, block.start, block.end)),
        };
        let mut from = block.start;
        for i in block.start..block.end {
            if !text.is_char_boundary(i) {
                continue;
            }
            if Some(i) == mark || needs_escape(text, i, block.start, block.end) {
                out.push_str(slice(text, from, i));
                out.push('\\');
                from = i;
            }
        }
        out.push_str(slice(text, from, block.end));
        at = block.end;
    }
    out
}

/// The tags of `text` as [`text_tags::find`] finds them, but those in
/// code, in a code block, in a link's destination or in a web address
/// ([`autolinks`]).
pub fn tags(text: &str) -> Vec<FoundTag> {
    outside(&literal_ranges(text), text)
}

/// The tags of the plain text `text` as [`text_tags::find`] finds them,
/// but those in a web address ([`plain_autolinks`]).
pub fn plain_tags(text: &str) -> Vec<FoundTag> {
    outside(&plain_autolinks(text), text)
}

fn outside(hidden: &[(usize, usize)], text: &str) -> Vec<FoundTag> {
    text_tags::find(text)
        .into_iter()
        .filter(|t| !hidden.iter().any(|&(a, b)| t.start < b && a < t.end))
        .collect()
}

/// The bare web addresses of the plain text `text`, each drawn as a link
/// and opened as one (#222), as (start, end) in bytes; nothing about
/// them is stored. One starts with `http://` or `https://`, in any case
/// of ASCII letters, not right after an ASCII letter or digit, and an
/// ASCII letter or digit or a character beyond ASCII must follow. It
/// runs up to a space of any kind (Unicode's White_Space), a `<`, a `>`
/// or the end, then, as GitHub's autolinks do, loses its last character
/// while that is one of `.,:;!?`, or a `)` with more `)` than `(` in it.
/// The same as the Dart core's `Markdown.plainAutolinks`.
pub fn plain_autolinks(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    addresses(text, (0, text.len()), &mut found);
    found
}

/// The bare web addresses of the formatted `text` as
/// [`plain_autolinks`] finds them, each within the characters a part
/// shows as written: none in code, in a code block or in a link, its
/// label included.
pub fn autolinks(text: &str) -> Vec<(usize, usize)> {
    fn walk(text: &str, spans: &[Span], found: &mut Vec<(usize, usize)>) {
        for s in spans {
            match s.kind {
                // An escaped character starts none.
                SpanKind::Text => addresses(text, (s.start, s.end), found),
                SpanKind::Bold | SpanKind::Italic | SpanKind::Strike => {
                    walk(text, &s.children, found);
                }
                SpanKind::Code | SpanKind::Link => {}
            }
        }
    }
    let mut found = Vec::new();
    for block in parse(text) {
        if block.code.is_none() {
            walk(text, &block.spans, &mut found);
        }
    }
    found
}

/// Adds the web addresses in the bytes `from..to` of `text` to `found`.
fn addresses(text: &str, (from, to): (usize, usize), found: &mut Vec<(usize, usize)>) {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < to {
        let rest = text.get(i..to).unwrap_or_default();
        let lower = |scheme: &str| {
            rest.get(..scheme.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(scheme))
        };
        let after = if lower("https://") {
            Some(i + 8)
        } else if lower("http://") {
            Some(i + 7)
        } else {
            None
        };
        let starts = after.filter(|&after| {
            !(i > 0 && bytes[i - 1].is_ascii_alphanumeric())
                && text
                    .get(after..to)
                    .and_then(|r| r.chars().next())
                    .is_some_and(|c| {
                        c.is_ascii_alphanumeric() || (!c.is_ascii() && !c.is_whitespace())
                    })
        });
        let Some(after) = starts else {
            i += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let mut end = text
            .get(after..to)
            .unwrap_or_default()
            .char_indices()
            .find(|&(_, c)| c == '<' || c == '>' || c.is_whitespace())
            .map_or(to, |(k, _)| after + k);
        loop {
            let link = &bytes[i..end];
            let count = |c: u8| link.iter().filter(|&&b| b == c).count();
            match link.last() {
                Some(b'.' | b',' | b':' | b';' | b'!' | b'?') => end -= 1,
                Some(b')') if count(b')') > count(b'(') => end -= 1,
                _ => break,
            }
        }
        found.push((i, end));
        i = end;
    }
}

/// Where `text` holds characters taken as written, so no tag and no box:
/// code, a code block, its fences included, a link's destination and a
/// web address ([`autolinks`]), as (start, end) in bytes.
pub fn literal_ranges(text: &str) -> Vec<(usize, usize)> {
    fn hide(spans: &[Span], hidden: &mut Vec<(usize, usize)>) {
        for s in spans {
            if s.kind == SpanKind::Code {
                hidden.push((s.start, s.end));
            }
            if s.kind == SpanKind::Link {
                hidden.push((s.content_end, s.end));
            }
            hide(&s.children, hidden);
        }
    }
    let mut hidden = autolinks(text);
    for block in parse(text) {
        if let Some(code) = &block.code {
            if block.line == code.first_line {
                hidden.push((code.start, code.end));
            }
            continue;
        }
        hide(&block.spans, &mut hidden);
    }
    hidden
}

/// Whether a link to `url` may be opened: a web address, `http://` or
/// `https://`, or an e-mail address, `mailto:`, in any case of ASCII
/// letters. A text may come from another person's Device, so nothing
/// else is opened from it: no file, no app's own address, no script.
pub fn may_open(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
}

/// The word at the byte offset `offset` of `text`, from where to where:
/// letters, digits and `_`, and `'`, `’` and `-` between them, with a
/// tag's `#` before it. A word ending at `offset` counts. Both are
/// `offset`, on a character's start, when there is none.
pub fn word_at(text: &str, offset: usize) -> (usize, usize) {
    let at = floor(text, offset);
    let after = |i: usize| text.get(i..).and_then(|r| r.chars().next());
    let before = |i: usize| text.get(..i).and_then(|r| r.chars().next_back());
    let word_after = |i: usize| after(i).is_some_and(word);
    let word_before = |i: usize| before(i).is_some_and(word);
    let mut end = at;
    loop {
        match after(end) {
            Some(c) if word(c) => end += c.len_utf8(),
            Some(c) if joiner(c) && word_before(end) && word_after(end + c.len_utf8()) => {
                end += c.len_utf8();
            }
            _ => break,
        }
    }
    let mut start = at;
    loop {
        match before(start) {
            Some(c) if word(c) => start -= c.len_utf8(),
            Some(c) if joiner(c) && word_after(start) && word_before(start - c.len_utf8()) => {
                start -= c.len_utf8();
            }
            _ => break,
        }
    }
    if start == end {
        return (at, at);
    }
    if start > 0
        && text.as_bytes().get(start - 1) == Some(&b'#')
        && text_tags::find(text).iter().any(|t| t.start == start - 1)
    {
        start -= 1;
    }
    (start, end)
}

/// `text` with bold, italic, strike-through or code (`kind`) put on the
/// selection `start..end`, or taken off when it has it; an error for any
/// other kind.
///
/// A cursor takes the format off the part its content holds the cursor
/// in, puts it on the word at the cursor, or, without a word, puts both
/// marks there with the cursor between them. A selection is taken line
/// by line without the spaces at its ends: when every such piece lies in
/// a part of that kind, marks included, those parts lose their marks;
/// otherwise every piece that does not gets them. Code gets more
/// backticks than any run inside it, and a space inside when it starts
/// or ends with one. The selection keeps what it selected. The lines of
/// a code block stay as they are.
pub fn toggle(text: &str, start: usize, end: usize, kind: SpanKind) -> Result<Change> {
    if !matches!(
        kind,
        SpanKind::Bold | SpanKind::Italic | SpanKind::Strike | SpanKind::Code
    ) {
        return Err(Error::Invalid(format!(
            "not bold, italic, strike or code: {}",
            kind.name()
        )));
    }
    let (s, e) = ordered(text, start, end);
    let blocks = parse(text);
    if s == e {
        let Some(block) = blocks.get(line_at(text, s)) else {
            return Ok(unchanged(text, s, e));
        };
        if block.code.is_some() {
            return Ok(unchanged(text, s, e));
        }
        if let Some(inside) = deepest(&block.spans, kind, &|span| {
            span.content_start <= s && s <= span.content_end
        }) {
            return Ok(apply(text, unwrap_span(inside), s, e));
        }
        let (from, to) = word_at(text, s);
        if from == to {
            let (open, close) = span_marks(kind, "");
            return Ok(Change {
                text: format!(
                    "{}{open}{close}{}",
                    slice(text, 0, s),
                    slice(text, s, text.len())
                ),
                start: s + open.len(),
                end: s + open.len(),
            });
        }
        return Ok(apply(text, wrap(text, from, to, kind), s, e));
    }
    let pieces: Vec<_> = pieces(text, &blocks, s, e)
        .into_iter()
        .filter(|(block, _, _)| block.code.is_none())
        .collect();
    if pieces.is_empty() {
        return Ok(unchanged(text, s, e));
    }
    let within: Vec<Option<&Span>> = pieces
        .iter()
        .map(|&(block, from, to)| {
            deepest(&block.spans, kind, &|span| {
                span.start <= from && to <= span.end
            })
        })
        .collect();
    if within.iter().all(Option::is_some) {
        let edits = within.into_iter().flatten().flat_map(unwrap_span).collect();
        return Ok(apply(text, edits, s, e));
    }
    let edits = pieces
        .iter()
        .zip(&within)
        .filter(|(_, w)| w.is_none())
        .flat_map(|(&(_, from, to), _)| wrap(text, from, to, kind))
        .collect();
    Ok(apply(text, edits, s, e))
}

/// `text` with a link to `url` on the selection `start..end` on its
/// first line, or on the word at a cursor. In a link already, its
/// destination becomes `url`, and a blank `url` takes the link off.
/// Without a word, the link is put at the cursor with `url` as its
/// label, selected. Spaces and control characters in `url` are written
/// %-encoded, `(`, `)` and `\` escaped. A code block's lines stay as
/// they are.
pub fn link(text: &str, start: usize, end: usize, url: &str) -> Change {
    let (s, e) = ordered(text, start, end);
    let blocks = parse(text);
    let Some(block) = blocks.get(line_at(text, s)) else {
        return unchanged(text, s, e);
    };
    if block.code.is_some() {
        return unchanged(text, s, e);
    }
    let e = e.min(block.end);
    let address = trimmed_url(url);
    let destination = destination(address);
    if let Some(inside) = deepest(&block.spans, SpanKind::Link, &|span| {
        span.start <= s && e <= span.end
    }) {
        if destination.is_empty() {
            return apply(text, unwrap_span(inside), s, e);
        }
        let edit = Edit {
            at: inside.content_end,
            remove: inside.end - inside.content_end,
            insert: format!("]({destination})"),
            after: false,
        };
        return apply(text, vec![edit], s, e);
    }
    if destination.is_empty() {
        return unchanged(text, s, e);
    }
    let (from, to) = if s == e {
        word_at(text, s)
    } else {
        trimmed(text, s, e)
    };
    if from == to {
        // A `]` would end the label.
        let label = escape(address).replace(']', "\\]");
        return Change {
            text: format!(
                "{}[{label}]({destination}){}",
                slice(text, 0, s),
                slice(text, s, text.len())
            ),
            start: s + 1,
            end: s + 1 + label.len(),
        };
    }
    let edits = vec![
        Edit {
            at: from,
            remove: 0,
            insert: "[".into(),
            after: true,
        },
        Edit {
            at: to,
            remove: 0,
            insert: format!("]({destination})"),
            after: false,
        },
    ];
    apply(text, edits, s, e)
}

/// `text` with every line the selection `start..end` touches made a
/// `kind` line, or made paragraph lines when they all are of `kind`
/// already. A selection ending at a line's start leaves that line out.
/// The old mark goes and the new one comes at the line's start, an item
/// keeping how far in it is; numbered items count on from one of their
/// level right above, or from 1. A cursor in a line's mark goes after
/// the new one. A table's and a code block's lines stay as they are, and
/// [`BlockKind::Table`] and [`BlockKind::Code`] change nothing;
/// [`code_block`] makes one.
pub fn toggle_line(text: &str, start: usize, end: usize, kind: BlockKind) -> Change {
    let (s, e) = ordered(text, start, end);
    if matches!(kind, BlockKind::Table | BlockKind::Code) {
        return unchanged(text, s, e);
    }
    let bytes = text.as_bytes();
    let blocks = parse(text);
    let (first, last) = touched(text, &blocks, s, e);
    let Some(lines) = blocks.get(first..=last) else {
        return unchanged(text, s, e);
    };
    // A table's and a code block's lines stay their own.
    let lines: Vec<&Block> = lines
        .iter()
        .filter(|b| !matches!(b.kind, BlockKind::Table | BlockKind::Code))
        .collect();
    let target = if kind != BlockKind::Paragraph && lines.iter().all(|b| b.kind == kind) {
        BlockKind::Paragraph
    } else {
        kind
    };
    let mut next = [0, 1].map(|level| match number_after(&blocks, first, level) {
        0 => 1,
        n => n,
    });
    let mut edits = Vec::new();
    for b in lines {
        let listed = b.kind.listed();
        let level = if listed { b.level } else { 0 };
        let lead = if listed {
            slice(text, b.start, skip_blank(bytes, b.start, b.end))
        } else {
            ""
        };
        let prefix = match target {
            BlockKind::Paragraph | BlockKind::Table | BlockKind::Code => String::new(),
            BlockKind::Heading1 => "# ".into(),
            BlockKind::Heading2 => "## ".into(),
            BlockKind::Quote => "> ".into(),
            BlockKind::Bullet => format!("{lead}- "),
            BlockKind::Numbered => {
                let n = next[level.min(1)];
                next[level.min(1)] += 1;
                format!("{lead}{n}. ")
            }
        };
        if level == 0 {
            next[1] = 1;
        }
        if target == BlockKind::Paragraph && b.kind == BlockKind::Paragraph {
            continue;
        }
        let mark_end = if b.kind == BlockKind::Paragraph {
            skip_blank(bytes, b.start, b.end)
        } else {
            b.content_start
        };
        if slice(text, b.start, mark_end) == prefix {
            continue;
        }
        edits.push(Edit {
            at: b.start,
            remove: mark_end - b.start,
            insert: prefix,
            after: true,
        });
    }
    apply(text, edits, s, e)
}

/// `text` with every item the selection `start..end` touches one level
/// in, or one level out with `out`; other lines stay as they are.
pub fn indent(text: &str, start: usize, end: usize, out: bool) -> Change {
    let (s, e) = ordered(text, start, end);
    let bytes = text.as_bytes();
    let blocks = parse(text);
    let (first, last) = touched(text, &blocks, s, e);
    let edits = blocks
        .get(first..=last)
        .unwrap_or_default()
        .iter()
        .filter(|b| b.kind.listed() && b.level == usize::from(out))
        .map(|b| Edit {
            at: b.start,
            remove: skip_blank(bytes, b.start, b.end) - b.start,
            insert: if out { String::new() } else { "  ".into() },
            after: !out,
        })
        .collect();
    apply(text, edits, s, e)
}

/// `text` with the lines the selection `start..end` touches put into a
/// fenced code block, or, when they all are lines of one block, that
/// block's fences taken off. A selection ending at a line's start leaves
/// that line out; a code block or a table it touches goes in whole. The
/// fences are backticks, more than any run of them inside and at least
/// three, on lines of their own with the text's first kind of line
/// break, `\n` when there is none. At an empty line the block is empty,
/// the cursor in it. The selection keeps what it selected.
pub fn code_block(text: &str, start: usize, end: usize) -> Change {
    let (s, e) = ordered(text, start, end);
    let blocks = parse(text);
    let (first, last) = touched(text, &blocks, s, e);
    let (Some(top), Some(bottom)) = (blocks.get(first), blocks.get(last)) else {
        return unchanged(text, s, e);
    };
    if let (Some(code), Some(other)) = (&top.code, &bottom.code)
        && Arc::ptr_eq(code, other)
    {
        return unfence(text, &blocks, code, s, e);
    }
    let first = top
        .table
        .as_ref()
        .map(|t| t.first_line)
        .or(top.code.as_ref().map(|c| c.first_line))
        .unwrap_or(first);
    let last = bottom
        .table
        .as_ref()
        .map(|t| t.last_line)
        .or(bottom.code.as_ref().map(|c| c.last_line))
        .unwrap_or(last);
    let (Some(top), Some(bottom)) = (blocks.get(first), blocks.get(last)) else {
        return unchanged(text, s, e);
    };
    let (from, to) = (top.start, bottom.end);
    let br = table::first_break(text);
    let (mut longest, mut run) = (0, 0);
    for c in slice(text, from, to).bytes() {
        run = if c == b'`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(if longest < 3 { 3 } else { longest + 1 });
    if from == to {
        let cursor = from + fence.len() + br.len();
        return Change {
            text: format!(
                "{}{fence}{br}{br}{fence}{}",
                slice(text, 0, from),
                slice(text, from, text.len())
            ),
            start: cursor,
            end: cursor,
        };
    }
    let edits = vec![
        Edit {
            at: from,
            remove: 0,
            insert: format!("{fence}{br}"),
            after: true,
        },
        Edit {
            at: to,
            remove: 0,
            insert: format!("{br}{fence}"),
            after: false,
        },
    ];
    apply(text, edits, s, e)
}

/// `text` with the fences of `code` taken off, each with the line break
/// between it and the content; a block without content goes whole.
fn unfence(text: &str, blocks: &[Block], code: &CodeBlock, s: usize, e: usize) -> Change {
    let last = if code.closed {
        code.last_line.saturating_sub(1)
    } else {
        code.last_line
    };
    let content = blocks
        .get(code.first_line + 1)
        .zip(blocks.get(last))
        .filter(|_| last > code.first_line)
        .map(|(a, b)| (a.start, b.end));
    let Some((content_start, content_end)) = content else {
        let edit = Edit {
            at: code.start,
            remove: code.end - code.start,
            insert: String::new(),
            after: false,
        };
        return apply(text, vec![edit], s, e);
    };
    let mut edits = vec![Edit {
        at: code.start,
        remove: content_start - code.start,
        insert: String::new(),
        after: true,
    }];
    if code.end > content_end {
        edits.push(Edit {
            at: content_end,
            remove: code.end - content_end,
            insert: String::new(),
            after: false,
        });
    }
    apply(text, edits, s, e)
}

/// Whether a shortcut ends at the byte offset `cursor` of `text`: a
/// line's mark with its space typed just before it, such as "# ", "- "
/// or "1. ", or the closing mark of bold, italic, strike-through, code
/// or a link, or a code block's closing fence. A plain text becomes
/// formatted when one is typed.
pub fn shortcut_at(text: &str, cursor: usize) -> bool {
    let at = floor(text, cursor);
    let blocks = parse(text);
    let Some(block) = blocks.get(line_at(text, at)) else {
        return false;
    };
    if let Some(code) = &block.code {
        if !code.closed || block.line != code.last_line {
            return false;
        }
        let bytes = text.as_bytes();
        let mark = code.fence.as_bytes().first().copied();
        let mut end = skip_blank(bytes, block.start, block.end);
        while end < block.end && bytes.get(end).copied() == mark {
            end += 1;
        }
        return at == end;
    }
    if block.kind != BlockKind::Paragraph
        && at == block.content_start
        && at > block.start
        && text.as_bytes().get(at - 1).is_some_and(|&c| blank(c))
    {
        return true;
    }
    fn ends(spans: &[Span], at: usize) -> bool {
        spans
            .iter()
            .any(|s| (s.kind != SpanKind::Text && s.end == at) || ends(&s.children, at))
    }
    ends(&block.spans, at)
}

/// `text` from `start` to `end`, empty where they are no character
/// boundaries.
fn slice(text: &str, start: usize, end: usize) -> &str {
    text.get(start..end).unwrap_or_default()
}

fn unchanged(text: &str, s: usize, e: usize) -> Change {
    Change {
        text: text.to_string(),
        start: s,
        end: e,
    }
}

/// `offset` within `text`, moved back to a character's start.
fn floor(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn ordered(text: &str, start: usize, end: usize) -> (usize, usize) {
    let (a, b) = (floor(text, start), floor(text, end));
    if a <= b { (a, b) } else { (b, a) }
}

/// The line the byte offset `offset` of `text` lies on, as [`parse`]
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

/// The lines of `text` as offsets, from the first character to the line
/// break: split at `\n`, `\r\n` and a lone `\r`.
fn lines(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let (mut start, mut i) = (0, 0);
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
    while at < end && bytes.get(at).is_some_and(|&c| blank(c)) {
        at += 1;
    }
    at
}

/// The run of backticks or tildes a fence `start..end` is written with,
/// and where it ends: up to three spaces, at least three of one of them;
/// a run of backticks with no backtick after it. None when the line is
/// no fence.
fn fence_at(text: &str, start: usize, end: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut at = start;
    while at < end && at - start < 4 && bytes.get(at) == Some(&b' ') {
        at += 1;
    }
    if at - start > 3 || at >= end {
        return None;
    }
    let c = *bytes.get(at)?;
    if c != b'`' && c != b'~' {
        return None;
    }
    let mut to = at;
    while to < end && bytes.get(to) == Some(&c) {
        to += 1;
    }
    if to - at < 3 || (c == b'`' && slice(text, to, end).contains('`')) {
        return None;
    }
    Some((slice(text, at, to), to))
}

/// The code block whose opening fence is the line `first` of `lines`:
/// up to the first line that closes it, a fence of its character at
/// least as long with only spaces and tabs after it, or to the last line.
fn code_at(text: &str, lines: &[(usize, usize)], first: usize) -> Option<CodeBlock> {
    let &(start, end) = lines.get(first)?;
    let (fence, after) = fence_at(text, start, end)?;
    let bytes = text.as_bytes();
    let closes = |&(s, e): &(usize, usize)| {
        fence_at(text, s, e).is_some_and(|(f, to)| {
            f.as_bytes().first() == fence.as_bytes().first()
                && f.len() >= fence.len()
                && skip_blank(bytes, to, e) == e
        })
    };
    let close = (first + 1..lines.len()).find(|&i| lines.get(i).is_some_and(closes));
    let last = close.unwrap_or(lines.len().saturating_sub(1));
    Some(CodeBlock {
        start,
        end: lines.get(last)?.1,
        first_line: first,
        last_line: last,
        fence: fence.to_string(),
        info: slice(text, after, end)
            .trim_matches([' ', '\t'])
            .to_string(),
        closed: close.is_some(),
    })
}

/// The blocks of the lines of `code`, each with the code block; a line of
/// its content one part shown as written.
fn code_blocks(text: &str, code: CodeBlock, lines: &[(usize, usize)]) -> Vec<Block> {
    let code = Arc::new(code);
    (code.first_line..=code.last_line)
        .map(|i| {
            let (start, end) = lines.get(i).copied().unwrap_or_default();
            let spans = if code.is_fence(i) || start == end {
                Vec::new()
            } else {
                vec![Span::literal(text, start, end)]
            };
            Block {
                kind: BlockKind::Code,
                line: i,
                level: 0,
                number: 0,
                start,
                end,
                content_start: start,
                spans,
                table: None,
                row: None,
                code: Some(Arc::clone(&code)),
            }
        })
        .collect()
}

/// A line's kind, level, the number written for a numbered item and
/// where its content starts.
fn block(text: &str, start: usize, end: usize) -> (BlockKind, usize, u64, usize) {
    let bytes = text.as_bytes();
    let byte = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut at = start;
    let mut width = 0;
    while at < end && blank(byte(at)) {
        width += if byte(at) == b'\t' { 4 } else { 1 };
        at += 1;
    }
    let ends_mark = |i: usize| i == end || blank(byte(i));
    if at < end && width <= 3 {
        let c = byte(at);
        if c == b'#' {
            let mut hashes = 1;
            while at + hashes < end && byte(at + hashes) == b'#' {
                hashes += 1;
            }
            if hashes <= 2 && ends_mark(at + hashes) {
                let kind = if hashes == 1 {
                    BlockKind::Heading1
                } else {
                    BlockKind::Heading2
                };
                return (kind, 0, 0, skip_blank(bytes, at + hashes, end));
            }
        }
        if c == b'>' {
            let content = if at + 1 < end && blank(byte(at + 1)) {
                at + 2
            } else {
                at + 1
            };
            return (BlockKind::Quote, 0, 0, content);
        }
    }
    let level = usize::from(width >= 2);
    if at < end && matches!(byte(at), b'-' | b'*' | b'+') && ends_mark(at + 1) {
        return (BlockKind::Bullet, level, 0, skip_blank(bytes, at + 1, end));
    }
    let mut digits = 0;
    while at + digits < end && byte(at + digits).is_ascii_digit() {
        digits += 1;
    }
    if (1..=9).contains(&digits)
        && at + digits < end
        && byte(at + digits) == b'.'
        && ends_mark(at + digits + 1)
    {
        let number = slice(text, at, at + digits).parse().unwrap_or(0);
        let content = skip_blank(bytes, at + digits + 1, end);
        return (BlockKind::Numbered, level, number, content);
    }
    (BlockKind::Paragraph, 0, 0, start)
}

/// One more than the number of the numbered item of `level` above the
/// line `line`, only items further in between; 0 when there is none.
fn number_after(blocks: &[Block], line: usize, level: usize) -> u64 {
    for b in blocks.get(..line).unwrap_or_default().iter().rev() {
        if b.kind.listed() && b.level > level {
            continue;
        }
        if b.kind == BlockKind::Numbered && b.level == level {
            return b.number + 1;
        }
        return 0;
    }
    0
}

/// Whether `c` counts as space for the flanking rules.
fn white(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\u{0C}' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

fn punct(c: char) -> bool {
    c.is_ascii_punctuation()
}

fn needs_escape(text: &str, i: usize, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    let byte = |j: usize| bytes.get(j).copied().unwrap_or(0);
    match byte(i) {
        b'\\' => i + 1 < end && byte(i + 1).is_ascii_punctuation(),
        b'*' | b'`' | b'[' | b'|' => true,
        b'~' => (i > start && byte(i - 1) == b'~') || (i + 1 < end && byte(i + 1) == b'~'),
        b'_' => {
            let plain = |c: Option<char>| c.is_some_and(|c| !white(c) && !punct(c));
            let before = slice(text, start, i).chars().next_back();
            let after = slice(text, i + 1, end).chars().next();
            !(plain(before) && plain(after))
        }
        _ => false,
    }
}

fn word(c: char) -> bool {
    c.is_alphabetic() || c.is_numeric() || c == '_'
}

fn joiner(c: char) -> bool {
    matches!(c, '\'' | '\u{2019}' | '-')
}

/// `start..end` moved in past the spaces at both ends.
fn trimmed(text: &str, start: usize, end: usize) -> (usize, usize) {
    let part = slice(text, start, end);
    let inner = part.trim_start_matches(white);
    let s = start + (part.len() - inner.len());
    let e = s + inner.trim_end_matches(white).len();
    (s, e)
}

/// The first and the last line the selection `s..e` touches.
fn touched(text: &str, blocks: &[Block], s: usize, e: usize) -> (usize, usize) {
    let first = line_at(text, s);
    let mut last = line_at(text, e);
    if last > first && blocks.get(last).is_some_and(|b| e == b.start) {
        last -= 1;
    }
    (first, last)
}

/// The pieces of the selection `s..e`, one per line, without the spaces
/// at their ends; blank ones left out.
fn pieces<'b>(
    text: &str,
    blocks: &'b [Block],
    s: usize,
    e: usize,
) -> Vec<(&'b Block, usize, usize)> {
    let (first, last) = touched(text, blocks, s, e);
    blocks
        .get(first..=last)
        .unwrap_or_default()
        .iter()
        .filter_map(|b| {
            let (from, to) = trimmed(text, s.max(b.start), e.min(b.end));
            (from < to).then_some((b, from, to))
        })
        .collect()
}

/// The innermost part of `kind` among `spans` that `test` holds for.
fn deepest<'s>(
    spans: &'s [Span],
    kind: SpanKind,
    test: &dyn Fn(&Span) -> bool,
) -> Option<&'s Span> {
    for span in spans {
        if let Some(inner) = deepest(&span.children, kind, test) {
            return Some(inner);
        }
        if span.kind == kind && test(span) {
            return Some(span);
        }
    }
    None
}

/// The marks `kind` puts around `content`.
fn span_marks(kind: SpanKind, content: &str) -> (String, String) {
    match kind {
        SpanKind::Bold => ("**".into(), "**".into()),
        SpanKind::Strike => ("~~".into(), "~~".into()),
        SpanKind::Code => {
            let (mut longest, mut run) = (0, 0);
            for c in content.bytes() {
                run = if c == b'`' { run + 1 } else { 0 };
                longest = longest.max(run);
            }
            let ticks = "`".repeat(longest + 1);
            let pad = if content.starts_with('`') || content.ends_with('`') {
                " "
            } else {
                ""
            };
            (format!("{ticks}{pad}"), format!("{pad}{ticks}"))
        }
        _ => ("*".into(), "*".into()),
    }
}

/// An edit of a text: `remove` bytes at `at` replaced by `insert`; an
/// offset within it goes after `insert` when `after`, else before.
struct Edit {
    at: usize,
    remove: usize,
    insert: String,
    after: bool,
}

fn wrap(text: &str, from: usize, to: usize, kind: SpanKind) -> Vec<Edit> {
    let (open, close) = span_marks(kind, slice(text, from, to));
    vec![
        Edit {
            at: from,
            remove: 0,
            insert: open,
            after: true,
        },
        Edit {
            at: to,
            remove: 0,
            insert: close,
            after: false,
        },
    ]
}

fn unwrap_span(span: &Span) -> Vec<Edit> {
    vec![
        Edit {
            at: span.start,
            remove: span.content_start - span.start,
            insert: String::new(),
            after: false,
        },
        Edit {
            at: span.content_end,
            remove: span.end - span.content_end,
            insert: String::new(),
            after: false,
        },
    ]
}

/// `text` with `edits` made, and the selection `s..e` moved along.
fn apply(text: &str, mut edits: Vec<Edit>, s: usize, e: usize) -> Change {
    edits.sort_by_key(|edit| edit.at);
    let mut out = String::new();
    let mut at = 0;
    for edit in &edits {
        out.push_str(slice(text, at, edit.at));
        out.push_str(&edit.insert);
        at = edit.at + edit.remove;
    }
    out.push_str(slice(text, at, text.len()));
    let moved = |p: usize| {
        let mut shift: isize = 0;
        for edit in &edits {
            if p < edit.at {
                break;
            }
            if p > edit.at + edit.remove {
                shift += edit.insert.len() as isize - edit.remove as isize;
                continue;
            }
            let inside = if edit.after { edit.insert.len() } else { 0 };
            return (edit.at as isize + shift) as usize + inside;
        }
        (p as isize + shift) as usize
    };
    Change {
        start: moved(s),
        end: moved(e),
        text: out,
    }
}

/// `url` without the spaces and control characters at its ends.
fn trimmed_url(url: &str) -> &str {
    let edge = |c: char| c <= ' ' || c == '\u{7F}' || white(c);
    url.trim_matches(edge)
}

/// `url` as a link's destination.
fn destination(url: &str) -> String {
    let mut out = String::new();
    for c in url.chars() {
        if c <= ' ' || c == '\u{7F}' || white(c) {
            let mut bytes = [0; 4];
            for byte in c.encode_utf8(&mut bytes).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        } else if matches!(c, '(' | ')' | '\\') {
            out.push('\\');
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}

/// A run of `*`, `_` or `~` that may open or close emphasis, as far as it
/// is not used yet: `start..end`.
struct Delim {
    char: u8,
    length: usize,
    open: bool,
    close: bool,
    start: usize,
    end: usize,
}

impl Delim {
    fn count(&self) -> usize {
        self.end - self.start
    }
}

enum Item {
    Span(Span),
    Delim(Delim),
}

/// The parts of the content of a line, `from..to`.
struct Inline<'t> {
    text: &'t str,
    from: usize,
    to: usize,
    links: bool,
    /// Whether it is a table's cell, where `\|` in code shows `|`, as GFM
    /// takes a cell's escaped pipes out before reading it.
    cell: bool,
}

impl Inline<'_> {
    fn byte(&self, i: usize) -> u8 {
        self.text.as_bytes().get(i).copied().unwrap_or(0)
    }

    fn escaped(&self, p: usize) -> bool {
        self.byte(p) == b'\\' && p + 1 < self.to && self.byte(p + 1).is_ascii_punctuation()
    }

    fn parse(&self) -> Vec<Span> {
        let mut items: Vec<Item> = Vec::new();
        let mut literal: Option<usize> = None;
        let flush = |items: &mut Vec<Item>, literal: &mut Option<usize>, p: usize| {
            if let Some(start) = literal.take()
                && start < p
            {
                items.push(Item::Span(Span::literal(self.text, start, p)));
            }
        };
        // Where the last `!` shown as written is, if any.
        let mut bang: Option<usize> = None;
        let mut p = self.from;
        while p < self.to {
            let c = self.byte(p);
            if self.escaped(p) {
                flush(&mut items, &mut literal, p);
                items.push(Item::Span(Span {
                    kind: SpanKind::Text,
                    start: p,
                    end: p + 2,
                    content_start: p + 1,
                    content_end: p + 2,
                    text: slice(self.text, p + 1, p + 2).to_string(),
                    url: String::new(),
                    children: Vec::new(),
                }));
                p += 2;
                continue;
            }
            if c == b'`' {
                let n = self.run(p);
                if let Some(close) = self.closing(p + n, n) {
                    flush(&mut items, &mut literal, p);
                    items.push(Item::Span(self.code(p, n, close)));
                    p = close + n;
                } else {
                    literal.get_or_insert(p);
                    p += n;
                }
                continue;
            }
            if c == b'['
                && self.links
                && bang.is_none_or(|b| b + 1 != p)
                && let Some(link) = self.link(p)
            {
                flush(&mut items, &mut literal, p);
                p = link.end;
                items.push(Item::Span(link));
                continue;
            }
            if matches!(c, b'*' | b'_' | b'~') {
                let n = self.run(p);
                if c != b'~' || n == 2 {
                    flush(&mut items, &mut literal, p);
                    items.push(Item::Delim(self.delim(c, p, p + n)));
                } else {
                    literal.get_or_insert(p);
                }
                p += n;
                continue;
            }
            if c == b'!' {
                bang = Some(p);
            }
            literal.get_or_insert(p);
            p += 1;
        }
        flush(&mut items, &mut literal, self.to);
        emphasis(self.text, &mut items);
        merged(
            self.text,
            items.into_iter().map(|i| span_of(self.text, i)).collect(),
        )
    }

    /// The length of the run of the byte at `p`.
    fn run(&self, p: usize) -> usize {
        let c = self.byte(p);
        let mut q = p;
        while q < self.to && self.byte(q) == c {
            q += 1;
        }
        q - p
    }

    /// Where the next run of exactly `n` backticks from `p` on starts.
    fn closing(&self, p: usize, n: usize) -> Option<usize> {
        let mut q = p;
        while q < self.to {
            if self.byte(q) != b'`' {
                q += 1;
                continue;
            }
            let m = self.run(q);
            if m == n {
                return Some(q);
            }
            q += m;
        }
        None
    }

    fn code(&self, p: usize, n: usize, close: usize) -> Span {
        let unescaped;
        let mut content = slice(self.text, p + n, close);
        if self.cell {
            unescaped = content.replace("\\|", "|");
            content = &unescaped;
        }
        if content.len() >= 2
            && content.starts_with(' ')
            && content.ends_with(' ')
            && content.bytes().any(|c| c != b' ')
        {
            content = slice(content, 1, content.len() - 1);
        }
        Span {
            kind: SpanKind::Code,
            start: p,
            end: close + n,
            content_start: p + n,
            content_end: close,
            text: content.to_string(),
            url: String::new(),
            children: Vec::new(),
        }
    }

    /// The link at the `[` at `p`, if there is one.
    fn link(&self, p: usize) -> Option<Span> {
        let mut depth = 1;
        let mut q = p + 1;
        while q < self.to {
            let c = self.byte(q);
            if self.escaped(q) {
                q += 2;
                continue;
            }
            if c == b'`' {
                let n = self.run(q);
                q = self.closing(q + n, n).map_or(q + n, |close| close + n);
                continue;
            }
            if c == b'[' {
                depth += 1;
            }
            if c == b']' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            q += 1;
        }
        if q >= self.to {
            return None;
        }
        let label_end = q;
        q += 1;
        if q >= self.to || self.byte(q) != b'(' {
            return None;
        }
        q = skip_blank(self.text.as_bytes(), q + 1, self.to);
        let mut url = String::new();
        let mut parens = 0;
        while q < self.to {
            if self.escaped(q) {
                url.push_str(slice(self.text, q + 1, q + 2));
                q += 2;
                continue;
            }
            let c = slice(self.text, q, self.to).chars().next()?;
            if white(c) {
                break;
            }
            if c < ' ' || c == '\u{7F}' {
                return None;
            }
            if c == '(' {
                parens += 1;
            }
            if c == ')' {
                if parens == 0 {
                    break;
                }
                parens -= 1;
            }
            url.push(c);
            q += c.len_utf8();
        }
        if parens != 0 {
            return None;
        }
        q = skip_blank(self.text.as_bytes(), q, self.to);
        if q >= self.to || self.byte(q) != b')' {
            return None;
        }
        let children = Inline {
            text: self.text,
            from: p + 1,
            to: label_end,
            links: false,
            cell: self.cell,
        }
        .parse();
        Some(Span {
            kind: SpanKind::Link,
            start: p,
            end: q + 1,
            content_start: p + 1,
            content_end: label_end,
            text: String::new(),
            url,
            children,
        })
    }

    fn delim(&self, c: u8, start: usize, end: usize) -> Delim {
        let before = if start == self.from {
            ' '
        } else {
            slice(self.text, self.from, start)
                .chars()
                .next_back()
                .unwrap_or(' ')
        };
        let after = if end == self.to {
            ' '
        } else {
            slice(self.text, end, self.to).chars().next().unwrap_or(' ')
        };
        let left = !white(after) && (!punct(after) || white(before) || punct(before));
        let right = !white(before) && (!punct(before) || white(after) || punct(after));
        let (open, close) = if c == b'_' {
            (
                left && (!right || punct(before)),
                right && (!left || punct(after)),
            )
        } else {
            (left, right)
        };
        Delim {
            char: c,
            length: end - start,
            open,
            close,
            start,
            end,
        }
    }
}

/// Matches openers and closers, as CommonMark's emphasis does: each
/// closer, left to right, with the nearest opener of its character
/// before it, two of each for bold when both have two, a pair whose
/// lengths add up to a multiple of three left alone when one of them can
/// both open and close, unless both are multiples of three.
fn emphasis(text: &str, items: &mut Vec<Item>) {
    let mut c = 0;
    while c < items.len() {
        let (char, closer_count, closer_length, closer_open) = match &items[c] {
            Item::Delim(d) if d.close && d.count() > 0 => (d.char, d.count(), d.length, d.open),
            _ => {
                c += 1;
                continue;
            }
        };
        let opener = (0..c).rev().find(|&o| match &items[o] {
            Item::Delim(d) => {
                d.char == char
                    && d.open
                    && d.count() > 0
                    && !third(d.char, d.length, d.close, closer_length, closer_open)
            }
            Item::Span(_) => false,
        });
        let Some(o) = opener else {
            c += 1;
            continue;
        };
        let opener_count = match &items[o] {
            Item::Delim(d) => d.count(),
            Item::Span(_) => 0,
        };
        let used = if char == b'~' || (opener_count >= 2 && closer_count >= 2) {
            2
        } else {
            1
        };
        let kind = match (char, used) {
            (b'~', _) => SpanKind::Strike,
            (_, 2) => SpanKind::Bold,
            _ => SpanKind::Italic,
        };
        let inner: Vec<Span> = items
            .drain(o + 1..c)
            .map(|item| span_of(text, item))
            .collect();
        let (mut start, mut content_start) = (0, 0);
        if let Item::Delim(d) = &mut items[o] {
            content_start = d.end;
            d.end -= used;
            start = d.end;
        }
        let (mut end, mut content_end) = (0, 0);
        if let Item::Delim(d) = &mut items[o + 1] {
            content_end = d.start;
            d.start += used;
            end = d.start;
        }
        let span = Span {
            kind,
            start,
            end,
            content_start,
            content_end,
            text: String::new(),
            url: String::new(),
            children: merged(text, inner),
        };
        items.insert(o + 1, Item::Span(span));
        c = o + 2;
        if matches!(&items[o], Item::Delim(d) if d.count() == 0) {
            items.remove(o);
            c -= 1;
        }
        if matches!(&items[c], Item::Delim(d) if d.count() == 0) {
            items.remove(c);
        }
    }
}

fn third(char: u8, opener: usize, opener_closes: bool, closer: usize, closer_opens: bool) -> bool {
    if char == b'~' {
        return false;
    }
    (opener_closes || closer_opens)
        && (opener + closer).is_multiple_of(3)
        && !(opener.is_multiple_of(3) && closer.is_multiple_of(3))
}

fn span_of(text: &str, item: Item) -> Span {
    match item {
        Item::Span(span) => span,
        Item::Delim(d) => Span::literal(text, d.start, d.end),
    }
}

/// `spans` with neighbouring runs of characters shown as written joined
/// into one.
fn merged(text: &str, spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        if let Some(last) = out.last_mut()
            && last.written()
            && span.written()
            && last.end == span.start
        {
            *last = Span::literal(text, last.start, span.end);
        } else {
            out.push(span);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `spans` as a short string, as the Dart core's test writes them.
    fn tree(spans: &[Span]) -> String {
        spans
            .iter()
            .map(|s| match s.kind {
                SpanKind::Text => format!("\"{}\"", s.text),
                SpanKind::Code => format!("code\"{}\"", s.text),
                SpanKind::Link => format!("link<{}>({})", s.url, tree(&s.children)),
                _ => format!("{}({})", s.kind.name(), tree(&s.children)),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn line(text: &str) -> String {
        tree(&parse(text)[0].spans)
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

    #[test]
    fn each_format_is_a_part_of_its_line_its_source_where_it_lies() {
        let text = "**bold** *it* ~~gone~~ `co de` [site](https://x.org)";
        assert_eq!(
            line(text),
            "bold(\"bold\") \" \" italic(\"it\") \" \" strike(\"gone\") \" \" code\"co de\" \
             \" \" link<https://x.org>(\"site\")"
        );
        let spans = &parse(text)[0].spans;
        let link = &spans[spans.len() - 1];
        assert_eq!(&text[spans[0].start..spans[0].end], "**bold**");
        assert_eq!(&text[link.content_end..link.end], "](https://x.org)");
        assert_eq!(line("***x***"), "italic(bold(\"x\"))");
        assert_eq!(line("2 * 3 * 4"), "\"2 * 3 * 4\"");
        assert_eq!(line("snake_case_name"), "\"snake_case_name\"");
        assert_eq!(line("[a [b](c)](d)"), "link<d>(\"a [b](c)\")");
        assert_eq!(line("![photo](u)"), "\"![photo](u)\"");
    }

    #[test]
    fn offsets_are_bytes_and_never_split_a_character() {
        let text = "ü **ö** 🧠";
        let bold = &parse(text)[0].spans[1];
        assert_eq!((bold.start, bold.end), (3, 9));
        assert_eq!(word_at(text, 1), (0, 2));
        // Inside a character, the offset goes back to its start.
        assert_eq!(word_at("🧠x", 2), (0, 0));
        assert_eq!(word_at("🧠x", 4), (4, 5));
        let change = toggle("ü", 1, 1, SpanKind::Bold).ok();
        assert_eq!(change.map(|c| c.text), Some("**ü**".to_string()));
        assert!(toggle("x", 0, 1, SpanKind::Link).is_err());
    }

    #[test]
    fn a_plain_text_escaped_reads_as_itself() {
        for text in [
            "# x\n1. y\n* z\n> q",
            "**b** `c` [l](u) ~~s~~ snake_case #my_tag \\* \\",
            "ü_ü _ü 🧠~~",
            "| a | b |\n|---|:-:|\n| c | d |",
            "- | -\n|-|\nx",
        ] {
            let escaped = escape(text);
            assert_eq!(plain(&escaped), text, "{escaped}");
            assert!(!uses(&escaped), "{escaped}");
        }
    }

    #[test]
    fn a_tag_in_code_or_in_a_links_destination_is_no_tag() {
        let text = "#a `#b` [#c](https://x.org/#d) #e\n# Heading #f";
        let found: Vec<String> = tags(text).into_iter().map(|t| t.tag).collect();
        assert_eq!(found, ["#a", "#c", "#e", "#f"]);
        // As plain text all count.
        assert_eq!(text_tags::find(text).len(), 5);
    }

    #[test]
    fn a_tag_inside_a_web_address_is_no_tag() {
        let text = "see https://x.org/?#frag and #a";
        let tags_of =
            |found: Vec<FoundTag>| -> Vec<String> { found.into_iter().map(|t| t.tag).collect() };
        assert_eq!(tags_of(text_tags::find(text)), ["#frag", "#a"]);
        assert_eq!(tags_of(plain_tags(text)), ["#a"]);
        assert_eq!(tags_of(tags(text)), ["#a"]);
    }

    fn plain_links(text: &str) -> Vec<&str> {
        plain_autolinks(text)
            .into_iter()
            .map(|(s, e)| &text[s..e])
            .collect()
    }

    #[test]
    fn a_bare_http_or_https_address_is_one_in_any_case() {
        assert_eq!(
            plain_links("see https://x.org/a?b=1 and HTTP://y.de now"),
            ["https://x.org/a?b=1", "HTTP://y.de"]
        );
        assert!(plain_links("www.x.org ftp://x.org mailto:a@x.org https:// x").is_empty());
        assert!(plain_links("xhttps://x.org 1https://x.org").is_empty());
        assert_eq!(plain_links("(https://x.org)"), ["https://x.org"]);
    }

    #[test]
    fn a_web_address_ends_as_githubs_autolinks_end() {
        assert_eq!(
            plain_links("https://x.org. https://x.org/a, https://x.org/b:;!?"),
            ["https://x.org", "https://x.org/a", "https://x.org/b"]
        );
        assert_eq!(
            plain_links("(see https://x.org/a_(b)) and https://x.org/c)."),
            ["https://x.org/a_(b)", "https://x.org/c"]
        );
        assert_eq!(
            plain_links("<https://x.org>\nhttps://y.org\u{a0}z https://z.org\u{3000}"),
            ["https://x.org", "https://y.org", "https://z.org"]
        );
        assert_eq!(plain_links("https://x.org/a.b.c"), ["https://x.org/a.b.c"]);
        assert_eq!(
            plain_links("https://x.org\u{202f}https://y.org\u{2028}"),
            ["https://x.org", "https://y.org"]
        );
    }

    #[test]
    fn a_formatted_text_has_no_web_address_in_code_a_code_block_or_a_link() {
        let text = "`https://a.org` [https://b.org](https://c.org) **https://d.org** \
                    *https://e.org*, https://f.org\\_x\n```\nhttps://g.org\n```\n\
                    | https://h.org | b |\n|---|---|";
        let found: Vec<&str> = autolinks(text)
            .into_iter()
            .map(|(s, e)| &text[s..e])
            .collect();
        assert_eq!(
            found,
            [
                "https://d.org",
                "https://e.org",
                "https://f.org",
                "https://h.org"
            ]
        );
        assert_eq!(plain_links(text).len(), 7);
    }

    #[test]
    fn only_a_web_or_e_mail_address_is_opened_from_a_link() {
        assert!(may_open("HTTPS://x.org") && may_open("mailto:a@x.org"));
        for url in [
            "javascript:x",
            "file:///etc",
            "intent://x",
            "x.org",
            "ʜttps://x",
        ] {
            assert!(!may_open(url), "{url}");
        }
    }

    #[test]
    fn html_groups_lines_escapes_and_links_only_what_may_open() {
        assert_eq!(
            html("one\ntwo\n\n  \nthree\n> a\n> **b**\n- a\n  - b\n  1. c\n- d\n\n3. f\n# H"),
            "<p>one<br>two</p>\n<p>three</p>\n<blockquote>a<br><strong>b</strong></blockquote>\n\
             <ul><li>a<ul><li>b</li></ul><ol><li>c</li></ol></li><li>d</li></ul>\n\
             <ol start=\"3\"><li>f</li></ol>\n<h1>H</h1>\n"
        );
        assert_eq!(
            html("<b>&</b> 'q' `<i>` [w](https://x.org/?a&b) [no](javascript:x)"),
            "<p>&lt;b&gt;&amp;&lt;/b&gt; &#39;q&#39; <code>&lt;i&gt;</code> \
             <a href=\"https://x.org/?a&amp;b\">w</a> no</p>\n"
        );
        assert_eq!(
            html("<!-- first column headings -->\n| A | B |\n|:-:|--:|\n| x | **y** |"),
            "<table><thead><tr><th align=\"center\">A</th><th align=\"right\">B</th></tr></thead>\
             <tbody><tr><th align=\"center\">x</th><td align=\"right\"><strong>y</strong></td></tr>\
             </tbody></table>\n"
        );
        assert_eq!(html(""), "");
    }

    // ------------------------------------------------------ Code blocks

    /// The kinds of the lines of `text`, `c` for a code block's.
    fn kinds(text: &str) -> String {
        parse(text)
            .iter()
            .map(|b| match b.kind {
                BlockKind::Code => 'c',
                kind => kind.name().chars().next().unwrap_or('?'),
            })
            .collect()
    }

    /// The content lines of the code blocks of `text`.
    fn content(text: &str) -> Vec<&str> {
        parse(text)
            .iter()
            .filter(|b| b.code.as_ref().is_some_and(|c| !c.is_fence(b.line)))
            .map(|b| slice(text, b.start, b.end))
            .collect()
    }

    fn code_of(text: &str, line: usize) -> Option<Arc<CodeBlock>> {
        parse(text).get(line).and_then(|b| b.code.clone())
    }

    #[test]
    fn a_fence_of_backticks_or_tildes_holds_lines_read_as_nothing() -> Result<()> {
        let text = "```\n# no *heading*\n- ☐ `x` [a](b)\n```\nafter";
        assert_eq!(kinds(text), "ccccp");
        let blocks = parse(text);
        let code = blocks[0]
            .code
            .clone()
            .ok_or(Error::Invalid("no block".into()))?;
        assert_eq!((code.first_line, code.last_line, code.closed), (0, 3, true));
        assert_eq!(
            (code.fence.as_str(), code.info.as_str(), code.language()),
            ("```", "", "")
        );
        assert_eq!(
            slice(text, code.start, code.end),
            "```\n# no *heading*\n- ☐ `x` [a](b)\n```"
        );
        assert_eq!(tree(&blocks[1].spans), "\"# no *heading*\"");
        assert_eq!(tree(&blocks[2].spans), "\"- ☐ `x` [a](b)\"");
        assert!(blocks[0].spans.is_empty() && blocks[3].spans.is_empty());
        assert!(blocks[4].code.is_none());
        assert_eq!(kinds("~~~\n**x**\n~~~"), "ccc");
        assert_eq!(
            code_of("~~~~ x", 0).map(|c| c.fence.clone()).as_deref(),
            Some("~~~~")
        );
        assert_eq!(kinds("``\nx\n``"), "ppp");
        assert_eq!(kinds("``` a`b\nx"), "pp");
        Ok(())
    }

    #[test]
    fn the_closing_fence_is_of_the_same_character_and_at_least_as_long() {
        let four = "````\n```\nx\n````\ny";
        assert_eq!(kinds(four), "ccccp");
        assert_eq!(content(four), ["```", "x"]);
        assert_eq!(kinds("```\nx\n`````\ny"), "cccp");
        assert_eq!(code_of("```\nx\n`````\ny", 0).map(|c| c.last_line), Some(2));
        assert_eq!(content("~~~\n```\n~~~"), ["```"]);
        assert_eq!(content("```\n~~~\n```"), ["~~~"]);
        assert_eq!(kinds("```\n~~~\n```\n~~~"), "cccc");
        assert_eq!(
            code_of("```\n~~~\n```\n~~~", 3).map(|c| c.first_line),
            Some(3)
        );
        let nested = "`````\n````\n```\nx\n```\n````\n`````\nz";
        assert_eq!(kinds(nested), "cccccccp");
        assert_eq!(content(nested), ["````", "```", "x", "```", "````"]);
    }

    #[test]
    fn a_closing_fence_is_up_to_three_spaces_in_with_only_blanks_after() {
        assert_eq!(kinds("```\nx\n   ```  \t\ny"), "cccp");
        assert_eq!(kinds("```\nx\n    ```\ny"), "cccc");
        assert_eq!(kinds("```\nx\n``` no\ny"), "cccc");
        assert_eq!(kinds("   ```\nx\n```"), "ccc");
        assert_eq!(kinds("    ```\nx\n```"), "ppc");
        assert_eq!(kinds("\t```\nx"), "pp");
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end_of_the_text() -> Result<()> {
        let text = "a\n```js\nb\n| c | d |\n|---|---|\n# e";
        assert_eq!(kinds(text), "pccccc");
        let code = code_of(text, 1).ok_or(Error::Invalid("no block".into()))?;
        assert_eq!(
            (code.first_line, code.last_line, code.closed),
            (1, 5, false)
        );
        assert!(!code.is_fence(5));
        assert!(table::all(text).is_empty());
        assert_eq!(plain(text), "a\nb\n| c | d |\n|---|---|\n# e");
        assert_eq!(kinds("```"), "c");
        assert_eq!(plain("```"), "");
        assert_eq!(plain("```\n```"), "");
        Ok(())
    }

    #[test]
    fn the_info_string_is_kept_its_first_word_the_language() -> Result<()> {
        let text = "```bash  -e  \necho hi\n```";
        let code = code_of(text, 0).ok_or(Error::Invalid("no block".into()))?;
        assert_eq!((code.info.as_str(), code.language()), ("bash  -e", "bash"));
        assert_eq!(plain(text), "echo hi");
        assert_eq!(
            code_of("~~~ py `x`\n", 0)
                .map(|c| c.language().to_string())
                .as_deref(),
            Some("py")
        );
        Ok(())
    }

    #[test]
    fn a_table_a_tag_and_a_box_in_a_block_are_text() -> Result<()> {
        let text = "#a\n```\n| x | y |\n|---|---|\n#b ☐ c\n```\n#c";
        assert!(table::all(text).is_empty());
        let found: Vec<String> = tags(text).into_iter().map(|t| t.tag).collect();
        assert_eq!(found, ["#a", "#c"]);
        assert!(tags("``` #x\ny").is_empty());
        let mut n = 0;
        let saved = crate::text_boxes::convert(
            "☐ a\n```\n☐ b\n```",
            crate::text_format::MARKDOWN,
            || {
                n += 1;
                Ok(n.to_string())
            },
        )?;
        assert_eq!(crate::text_boxes::find(&saved).len(), 1);
        assert!(saved.contains("☐ b"));
        Ok(())
    }

    #[test]
    fn a_fence_ends_a_table_and_a_table_may_follow_a_block() {
        let text = "a | b\n-|-\nc\n```\nx\n```\nd | e\n-|-";
        assert_eq!(kinds(text), "tttccctt");
        assert_eq!(table::all(text).len(), 2);
        assert_eq!(
            table::insert("```\nx\n```\ny", 5).text,
            "```\nx\n```\n\n|  |  |\n| --- | --- |\n|  |  |\n\ny"
        );
    }

    #[test]
    fn lines_of_any_line_break_are_lines_of_the_block() {
        let text = "```\r\na\rb\r\n```\r\nc";
        assert_eq!(kinds(text), "ccccp");
        assert_eq!(plain(text), "a\rb\r\nc");
    }

    #[test]
    fn html_of_a_block_is_a_pre_with_the_language_as_a_class() {
        assert_eq!(
            html("a\n```bash x\n<b> & \"q\"\n\n```\nb"),
            "<p>a</p>\n<pre><code class=\"language-bash\">&lt;b&gt; &amp; &quot;q&quot;\n\n</code></pre>\n<p>b</p>\n"
        );
        assert_eq!(html("~~~\n"), "<pre><code>\n</code></pre>\n");
        assert_eq!(html("```\nx"), "<pre><code>x\n</code></pre>\n");
    }

    #[test]
    fn a_text_with_a_block_uses_a_format_an_escaped_one_has_no_block() {
        assert!(uses("```\nx\n```"));
        assert!(uses("~~~"));
        for text in ["```\nx\n```", "~~~ sh\n# y\n~~~", "````"] {
            let escaped = escape(text);
            assert!(!uses(&escaped), "{escaped}");
            assert_eq!(plain(&escaped), text);
        }
    }

    #[test]
    fn formats_and_line_formats_leave_a_block_as_it_is() -> Result<()> {
        let text = "a\n```\nb c\n```";
        assert_eq!(toggle(text, 7, 7, SpanKind::Bold)?, unchanged(text, 7, 7));
        assert_eq!(
            toggle(text, 0, text.len(), SpanKind::Italic)?.text,
            "*a*\n```\nb c\n```"
        );
        assert_eq!(link(text, 6, 9, "https://x.org").text, text);
        assert_eq!(
            toggle_line(text, 0, text.len(), BlockKind::Quote).text,
            "> a\n```\nb c\n```"
        );
        assert_eq!(
            toggle_line(text, 0, 0, BlockKind::Code),
            unchanged(text, 0, 0)
        );
        Ok(())
    }

    #[test]
    fn a_closing_fence_typed_is_a_shortcut_an_opening_one_is_not() {
        let text = "x\n```\na\n````";
        let at: Vec<usize> = (0..=text.len()).filter(|&i| shortcut_at(text, i)).collect();
        assert_eq!(at, [text.len()]);
        assert!(!shortcut_at("```\na\n```", 3));
    }

    fn change(text: &str, start: usize, end: usize) -> Change {
        Change {
            text: text.into(),
            start,
            end,
        }
    }

    #[test]
    fn code_block_puts_the_lines_touched_into_a_fence() {
        assert_eq!(
            code_block("a\nb c\nd", 3, 4),
            change("a\n```\nb c\n```\nd", 7, 8)
        );
        assert_eq!(
            code_block("a\nb\nc", 0, 3),
            change("```\na\nb\n```\nc", 4, 7)
        );
        assert_eq!(code_block("a\n\nb", 2, 2), change("a\n```\n\n```\nb", 6, 6));
        assert_eq!(code_block("", 0, 0), change("```\n\n```", 4, 4));
        assert_eq!(
            code_block("x ```` y\r\nz", 0, 0).text,
            "`````\r\nx ```` y\r\n`````\r\nz"
        );
        assert_eq!(
            code_block("a\n```\nb\n```", 0, 3).text,
            "````\na\n```\nb\n```\n````"
        );
        assert_eq!(
            code_block("x | y\n-|-\nz", 0, 0).text,
            "```\nx | y\n-|-\nz\n```"
        );
    }

    #[test]
    fn code_block_in_a_block_takes_its_fences_off() {
        assert_eq!(
            code_block("a\n```sh\nb\nc\n```\nd", 9, 9),
            change("a\nb\nc\nd", 3, 3)
        );
        assert_eq!(code_block("```\nb", 5, 5), change("b", 1, 1));
        assert_eq!(code_block("```\n```", 0, 0), change("", 0, 0));
        assert_eq!(code_block("x\n```", 3, 3), change("x\n", 2, 2));
        assert_eq!(
            code_block("```\na\n```\nb", 4, 11).text,
            "````\n```\na\n```\nb\n````"
        );
    }
}
