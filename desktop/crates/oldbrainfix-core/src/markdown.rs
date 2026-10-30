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

/// A line of a formatted text: its kind, the line it is, counted as
/// [`line_at`] counts lines, where it lies in the text, `start..end`
/// without its line break, and where its content begins after the mark,
/// `content_start`. Offsets are bytes.
///
/// A line of a table holds the table it belongs to and, a row of it, its
/// row there, the header row 0; its spans are those of its cells, left to
/// right. The delimiter row and the line marking the first column as
/// headings have no row.
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
}

impl Block {
    /// The text this line shows, without its marks: a table's row its
    /// cells that show something, a space between them, its other lines
    /// nothing.
    pub fn visible(&self) -> String {
        match (&self.table, self.row) {
            (Some(table), Some(row)) => table
                .rows
                .get(row)
                .map(table::Row::visible)
                .unwrap_or_default(),
            (Some(_), None) => String::new(),
            _ => self.spans.iter().map(Span::visible).collect(),
        }
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
        });
        i += 1;
    }
    blocks
}

/// `text` as it reads without its marks, its line breaks as they are:
/// what search looks in and what a preview shows. A table's delimiter row
/// and the line marking its first column as headings go with a line
/// break; each of its rows reads as its cells that show something, a
/// space between them.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    let blocks = parse(text);
    for (i, block) in blocks.iter().enumerate() {
        if block.table.is_some() && block.row.is_none() {
            // The line break before it goes, or the one after the first
            // line, which is followed by its table's header row.
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
/// its first column holds headings, a column's alignment as `align`.
/// Bold is `<strong>`, italic `<em>`, strike-through `<del>`, code
/// `<code>`; a link is an `<a>` only when [`may_open`] allows its
/// destination, else its label alone. Every character of the text is
/// escaped, so nothing in it is markup; status marks and tags are text. A
/// line break follows each part at the top.
pub fn html(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut html = Html::default();
    for block in parse(text) {
        match block.kind {
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
/// code or in a link's destination.
pub fn tags(text: &str) -> Vec<FoundTag> {
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
    let mut hidden = Vec::new();
    for block in parse(text) {
        hide(&block.spans, &mut hidden);
    }
    text_tags::find(text)
        .into_iter()
        .filter(|t| !hidden.iter().any(|&(a, b)| t.start < b && a < t.end))
        .collect()
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
/// or ends with one. The selection keeps what it selected.
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
    let pieces = pieces(text, &blocks, s, e);
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
/// %-encoded, `(`, `)` and `\` escaped.
pub fn link(text: &str, start: usize, end: usize, url: &str) -> Change {
    let (s, e) = ordered(text, start, end);
    let blocks = parse(text);
    let Some(block) = blocks.get(line_at(text, s)) else {
        return unchanged(text, s, e);
    };
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
/// the new one. A table's lines stay as they are, and
/// [`BlockKind::Table`] changes nothing.
pub fn toggle_line(text: &str, start: usize, end: usize, kind: BlockKind) -> Change {
    let (s, e) = ordered(text, start, end);
    if kind == BlockKind::Table {
        return unchanged(text, s, e);
    }
    let bytes = text.as_bytes();
    let blocks = parse(text);
    let (first, last) = touched(text, &blocks, s, e);
    let Some(lines) = blocks.get(first..=last) else {
        return unchanged(text, s, e);
    };
    // A table's lines stay its own.
    let lines: Vec<&Block> = lines
        .iter()
        .filter(|b| b.kind != BlockKind::Table)
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
            BlockKind::Paragraph | BlockKind::Table => String::new(),
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

/// Whether a shortcut ends at the byte offset `cursor` of `text`: a
/// line's mark with its space typed just before it, such as "# ", "- "
/// or "1. ", or the closing mark of bold, italic, strike-through, code
/// or a link. A plain text becomes formatted when one is typed.
pub fn shortcut_at(text: &str, cursor: usize) -> bool {
    let at = floor(text, cursor);
    let blocks = parse(text);
    let Some(block) = blocks.get(line_at(text, at)) else {
        return false;
    };
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
}
