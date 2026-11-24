//! A Markdown file opened in the desktop's viewer (obf-1056), read to be
//! drawn: all of CommonMark with GitHub's tables, task lists,
//! strike-through and footnotes, the YAML front matter as properties,
//! Obsidian's wiki links and embeds, callouts (`> [!note] Title`) and
//! highlights (`==text==`). Nothing of it is run or fetched: HTML shows
//! as written, a picture is only named here, and only web and e-mail
//! addresses are links out.
//!
//! A text to be drawn is an [`Inline`]: what it shows and its runs, each
//! of one look and link. Tables and code blocks keep their source, which
//! the app's formatted text draws as it draws a Note's (obf-1048,
//! obf-1036): a table rebuilt from its cells, its wiki links as their
//! words; a code block as a fenced one.

use oldbrainfix_core::obsidian;
use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, MetadataBlockKind, Options, Parser,
    Tag, TagEnd,
};

/// A Markdown file read: the properties of its front matter, by their
/// keys as written, its blocks and its footnotes, numbered in the order
/// they are first referred to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Doc {
    pub properties: Vec<(String, String)>,
    pub blocks: Vec<Block>,
    pub footnotes: Vec<Footnote>,
}

/// A footnote: its label as written, its number and what it says.
#[derive(Debug, Clone, PartialEq)]
pub struct Footnote {
    pub label: String,
    pub number: usize,
    pub blocks: Vec<Block>,
}

/// A block of a Markdown file.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// A heading of level 1 to 6.
    Heading {
        level: usize,
        text: Inline,
    },
    Paragraph(Inline),
    Quote(Vec<Block>),
    /// An Obsidian callout: its kind in lower case, its title and what it
    /// says.
    Callout {
        kind: String,
        title: Inline,
        blocks: Vec<Block>,
    },
    /// A list, numbered from `start` when it has one.
    List {
        start: Option<u64>,
        items: Vec<Item>,
    },
    /// A code block or an HTML block, as the source of a fenced code
    /// block in the app's formatted text.
    Code(String),
    /// A table, as the source of a table in the app's formatted text.
    Table(String),
    Rule,
    Picture(Picture),
}

/// An item of a list: its task box, ticked or not, when it has one, and
/// what it says.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

/// A picture a file shows: what it names as written, whether by a wiki
/// embed (`![[picture.png]]`) or in Markdown's own way, and its words.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Picture {
    pub target: String,
    pub wiki: bool,
    pub alt: String,
}

/// Where a link goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A web or e-mail address.
    Web(String),
    /// Another Markdown file, by a wiki link's name or a path as written,
    /// at a heading in it when it names one.
    Note {
        name: String,
        heading: Option<String>,
    },
    /// A heading of this file.
    Heading(String),
    /// A footnote of this file, by its label.
    Footnote(String),
    /// Anything else: shown as a link, opening nothing.
    Nowhere,
}

/// How a run of an [`Inline`] looks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub highlight: bool,
    /// A footnote's number where it is referred to.
    pub note: bool,
}

/// A run of an [`Inline`]: the bytes `start..end` of its text, in one
/// look and going to one link.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub start: usize,
    pub end: usize,
    pub style: Style,
    pub link: Option<Target>,
}

/// A text as it shows, in runs that cover all of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inline {
    pub text: String,
    pub runs: Vec<Run>,
    /// Where its soft line breaks are, each a `\n` until [`Inline::flat`]
    /// makes it a space; a callout's title ends at the first.
    soft: Vec<usize>,
}

impl Inline {
    /// A text of one run in `style`.
    pub fn plain(text: &str, style: Style) -> Inline {
        let mut inline = Inline::default();
        inline.push(text, style, None);
        inline
    }

    fn push(&mut self, text: &str, style: Style, link: Option<Target>) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        let end = self.text.len();
        if let Some(last) = self.runs.last_mut()
            && last.style == style
            && last.link == link
            && last.end == start
        {
            last.end = end;
            return;
        }
        self.runs.push(Run {
            start,
            end,
            style,
            link,
        });
    }

    /// It without its text from the byte `at` on.
    fn cut(&mut self, at: usize) {
        self.text.truncate(at);
        self.runs.retain(|r| r.start < at);
        if let Some(last) = self.runs.last_mut() {
            last.end = last.end.min(at);
        }
        self.soft.retain(|&s| s < at);
    }

    /// Whether it shows nothing but blanks.
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Its soft line breaks as spaces.
    fn flat(mut self) -> Inline {
        for at in std::mem::take(&mut self.soft) {
            if self.text.get(at..=at) == Some("\n") {
                self.text.replace_range(at..=at, " ");
            }
        }
        self
    }

    /// The bytes `from..to` of it, with their runs.
    fn slice(&self, from: usize, to: usize) -> Inline {
        let mut out = Inline::default();
        for run in &self.runs {
            let (a, b) = (run.start.max(from), run.end.min(to));
            if a < b
                && let Some(text) = self.text.get(a..b)
            {
                out.push(text, run.style, run.link.clone());
            }
        }
        out.soft = self
            .soft
            .iter()
            .filter(|&&at| at >= from && at < to)
            .map(|at| at - from)
            .collect();
        out
    }

    /// It with each pair of `==` outside code marking what lies between
    /// as highlighted, the marks left out; a `==` without a pair stays.
    fn highlighted(self) -> Inline {
        let marks: Vec<usize> = self
            .runs
            .iter()
            .filter(|r| !r.style.code)
            .flat_map(|r| {
                let text = self.text.get(r.start..r.end).unwrap_or_default();
                let mut found = Vec::new();
                let mut at = 0;
                while let Some(i) = text.get(at..).and_then(|t| t.find("==")) {
                    found.push(r.start + at + i);
                    at += i + 2;
                }
                found
            })
            .collect();
        let pairs = marks.len() / 2 * 2;
        if pairs == 0 {
            return self;
        }
        let marks = marks.get(..pairs).unwrap_or_default();
        let mut out = Inline::default();
        let mut on = false;
        for run in &self.runs {
            let mut at = run.start;
            for &mark in marks.iter().filter(|&&m| m >= run.start && m < run.end) {
                let style = Style {
                    highlight: on,
                    ..run.style
                };
                out.push(
                    self.text.get(at..mark).unwrap_or_default(),
                    style,
                    run.link.clone(),
                );
                on = !on;
                at = mark + 2;
            }
            let style = Style {
                highlight: on,
                ..run.style
            };
            out.push(
                self.text.get(at..run.end).unwrap_or_default(),
                style,
                run.link.clone(),
            );
        }
        out.soft = self.soft;
        out
    }
}

/// A container being read.
enum Frame {
    Root(Vec<Block>),
    Quote(Vec<Block>),
    List(Option<u64>, Vec<Item>),
    Item(Option<bool>, Vec<Block>),
    Footnote(String, Vec<Block>),
}

impl Frame {
    fn blocks(&mut self) -> Option<&mut Vec<Block>> {
        match self {
            Frame::Root(b) | Frame::Quote(b) | Frame::Item(_, b) | Frame::Footnote(_, b) => Some(b),
            Frame::List(..) => None,
        }
    }
}

/// A text being read: its kind, what it says so far, its look and links
/// open, and the pictures in it.
struct Leaf {
    heading: Option<usize>,
    inline: Inline,
    pictures: Vec<Picture>,
}

/// What reads a file's events into a [`Doc`].
struct Reader<'a> {
    source: &'a str,
    frames: Vec<Frame>,
    leaf: Option<Leaf>,
    bold: usize,
    italic: usize,
    strike: usize,
    links: Vec<Target>,
    /// Where each link open began in the text being read, and the words
    /// a wiki link without a label shows in place of what it names, as
    /// Obsidian shows them.
    link_starts: Vec<(usize, Option<String>)>,
    /// The picture being read, its words gathered.
    picture: Option<Picture>,
    /// The table being read: its alignments, its rows of cells' sources,
    /// and whether a cell is being read, whose events are skipped.
    table: Option<(Vec<Alignment>, Vec<Vec<String>>, bool)>,
    /// The code or HTML block being read: its language and its text.
    code: Option<(String, String)>,
    metadata: Option<String>,
    properties: Vec<(String, String)>,
    footnotes: Vec<(String, Vec<Block>)>,
    referred: Vec<String>,
}

/// Reads the Markdown `source`.
pub fn read(source: &str) -> Doc {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_WIKILINKS;
    let mut reader = Reader {
        source,
        frames: vec![Frame::Root(Vec::new())],
        leaf: None,
        bold: 0,
        italic: 0,
        strike: 0,
        links: Vec::new(),
        link_starts: Vec::new(),
        picture: None,
        table: None,
        code: None,
        metadata: None,
        properties: Vec::new(),
        footnotes: Vec::new(),
        referred: Vec::new(),
    };
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        reader.event(event, range);
    }
    reader.end_leaf();
    reader.done()
}

impl Reader<'_> {
    fn style(&self) -> Style {
        Style {
            bold: self.bold > 0,
            italic: self.italic > 0,
            strike: self.strike > 0,
            ..Style::default()
        }
    }

    /// The text being read, a paragraph of a tight list's item where none
    /// was started.
    fn leaf(&mut self) -> &mut Leaf {
        self.leaf.get_or_insert_with(|| Leaf {
            heading: None,
            inline: Inline::default(),
            pictures: Vec::new(),
        })
    }

    /// Adds `block` to the container being read.
    fn add(&mut self, block: Block) {
        if let Some(blocks) = self.frames.last_mut().and_then(Frame::blocks) {
            blocks.push(block);
        }
    }

    /// Ends the text being read: a heading or a paragraph, the pictures
    /// in it after it. A paragraph that opens a quote keeps its soft line
    /// breaks, for a callout's title.
    fn end_leaf(&mut self) {
        let Some(leaf) = self.leaf.take() else {
            return;
        };
        let opens_quote = matches!(self.frames.last(), Some(Frame::Quote(b)) if b.is_empty());
        let inline = leaf.inline.highlighted();
        let inline = if opens_quote { inline } else { inline.flat() };
        match leaf.heading {
            Some(level) => self.add(Block::Heading {
                level,
                text: inline,
            }),
            None if !inline.is_blank() => self.add(Block::Paragraph(inline)),
            None => {}
        }
        for picture in leaf.pictures {
            self.add(Block::Picture(picture));
        }
    }

    fn text(&mut self, text: &str) {
        if let Some((_, code)) = &mut self.code {
            code.push_str(text);
            return;
        }
        if let Some(meta) = &mut self.metadata {
            meta.push_str(text);
            return;
        }
        if let Some(picture) = &mut self.picture {
            picture.alt.push_str(text);
            return;
        }
        let (style, link) = (self.style(), self.links.last().cloned());
        self.leaf().inline.push(text, style, link);
    }

    fn event(&mut self, event: Event, range: std::ops::Range<usize>) {
        if let Some((_, _, true)) = &self.table {
            if matches!(event, Event::End(TagEnd::TableCell))
                && let Some((_, _, cell)) = &mut self.table
            {
                *cell = false;
            }
            return;
        }
        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(end) => self.end(end),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => {
                if self.picture.is_some() {
                    self.text(&code);
                    return;
                }
                let style = Style {
                    code: true,
                    ..self.style()
                };
                let link = self.links.last().cloned();
                self.leaf().inline.push(&code, style, link);
            }
            Event::Html(html) | Event::InlineHtml(html) => self.text(&html),
            Event::InlineMath(math) | Event::DisplayMath(math) => self.text(&math),
            Event::SoftBreak => {
                if self.picture.is_some() {
                    self.text(" ");
                    return;
                }
                let leaf = self.leaf();
                let at = leaf.inline.text.len();
                leaf.inline.soft.push(at);
                let style = Style::default();
                leaf.inline.push("\n", style, None);
            }
            Event::HardBreak => self.text("\n"),
            Event::Rule => {
                self.end_leaf();
                self.add(Block::Rule);
            }
            Event::TaskListMarker(ticked) => {
                if let Some(Frame::Item(task, _)) = self.frames.last_mut() {
                    *task = Some(ticked);
                }
            }
            Event::FootnoteReference(label) => {
                let label = label.to_string();
                let number = match self.referred.iter().position(|l| *l == label) {
                    Some(i) => i + 1,
                    None => {
                        self.referred.push(label.clone());
                        self.referred.len()
                    }
                };
                let style = Style {
                    note: true,
                    ..Style::default()
                };
                self.leaf().inline.push(
                    &format!("[{number}]"),
                    style,
                    Some(Target::Footnote(label)),
                );
            }
        }
    }

    fn start(&mut self, tag: Tag, range: std::ops::Range<usize>) {
        match tag {
            Tag::Paragraph => {
                self.end_leaf();
                self.leaf();
            }
            Tag::Heading { level, .. } => {
                self.end_leaf();
                self.leaf().heading = Some(heading_level(level));
            }
            Tag::BlockQuote(_) => {
                self.end_leaf();
                self.frames.push(Frame::Quote(Vec::new()));
            }
            Tag::CodeBlock(kind) => {
                self.end_leaf();
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((language, String::new()));
            }
            Tag::HtmlBlock => {
                self.end_leaf();
                self.code = Some(("html".to_string(), String::new()));
            }
            Tag::List(start) => {
                self.end_leaf();
                self.frames.push(Frame::List(start, Vec::new()));
            }
            Tag::Item => {
                self.end_leaf();
                self.frames.push(Frame::Item(None, Vec::new()));
            }
            Tag::FootnoteDefinition(label) => {
                self.end_leaf();
                self.frames
                    .push(Frame::Footnote(label.to_string(), Vec::new()));
            }
            Tag::Table(aligns) => {
                self.end_leaf();
                self.table = Some((aligns, Vec::new(), false));
            }
            Tag::TableHead | Tag::TableRow => {
                if let Some((_, rows, _)) = &mut self.table {
                    rows.push(Vec::new());
                }
            }
            Tag::TableCell => {
                let cell = self.source.get(range).unwrap_or_default();
                if let Some((_, rows, reading)) = &mut self.table {
                    if let Some(row) = rows.last_mut() {
                        row.push(table_cell(cell));
                    }
                    *reading = true;
                }
            }
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Superscript | Tag::Subscript => {}
            Tag::Link {
                link_type,
                dest_url,
                ..
            } => {
                let wiki = matches!(link_type, LinkType::WikiLink { .. });
                let shown = match link_type {
                    LinkType::WikiLink { has_pothole: false } => {
                        obsidian::wiki_links(&format!("[[{dest_url}]]"))
                            .first()
                            .map(obsidian::WikiLink::shown)
                    }
                    _ => None,
                };
                let at = self.leaf().inline.text.len();
                self.link_starts.push((at, shown));
                self.links.push(target(&dest_url, wiki));
            }
            Tag::Image {
                link_type,
                dest_url,
                ..
            } => {
                self.picture = Some(Picture {
                    target: dest_url.to_string(),
                    wiki: matches!(link_type, LinkType::WikiLink { .. }),
                    alt: String::new(),
                });
            }
            Tag::MetadataBlock(MetadataBlockKind::YamlStyle) => {
                self.metadata = Some(String::new());
            }
            Tag::MetadataBlock(MetadataBlockKind::PlusesStyle) => {}
            Tag::DefinitionList | Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {}
        }
    }

    fn end(&mut self, end: TagEnd) {
        match end {
            TagEnd::Paragraph | TagEnd::Heading(_) => self.end_leaf(),
            TagEnd::BlockQuote(_) => {
                self.end_leaf();
                if let Some(Frame::Quote(blocks)) = self.frames.pop() {
                    self.add(quote(blocks));
                }
            }
            TagEnd::CodeBlock | TagEnd::HtmlBlock => {
                if let Some((language, text)) = self.code.take() {
                    self.add(Block::Code(fenced(&language, &text)));
                }
            }
            TagEnd::List(_) => {
                self.end_leaf();
                if let Some(Frame::List(start, items)) = self.frames.pop() {
                    self.add(Block::List { start, items });
                }
            }
            TagEnd::Item => {
                self.end_leaf();
                if let Some(Frame::Item(task, blocks)) = self.frames.pop()
                    && let Some(Frame::List(_, items)) = self.frames.last_mut()
                {
                    items.push(Item { task, blocks });
                }
            }
            TagEnd::FootnoteDefinition => {
                self.end_leaf();
                if let Some(Frame::Footnote(label, blocks)) = self.frames.pop() {
                    self.footnotes.push((label, blocks));
                }
            }
            TagEnd::Table => {
                if let Some((aligns, rows, _)) = self.table.take() {
                    self.add(Block::Table(table_source(&aligns, &rows)));
                }
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => {
                if let Some((at, Some(shown))) = self.link_starts.pop() {
                    let (style, link) = (self.style(), self.links.last().cloned());
                    let inline = &mut self.leaf().inline;
                    inline.cut(at);
                    inline.push(&shown, style, link);
                }
                self.links.pop();
            }
            TagEnd::Image => {
                if let Some(picture) = self.picture.take() {
                    self.leaf().pictures.push(picture);
                }
            }
            TagEnd::MetadataBlock(_) => {
                if let Some(meta) = self.metadata.take() {
                    self.properties = properties(&meta);
                }
            }
            _ => {}
        }
    }

    fn done(mut self) -> Doc {
        let blocks = match self.frames.drain(..).next() {
            Some(Frame::Root(blocks)) => blocks,
            _ => Vec::new(),
        };
        let mut footnotes: Vec<Footnote> = Vec::new();
        let mut defined = std::mem::take(&mut self.footnotes);
        for (i, label) in self.referred.iter().enumerate() {
            if let Some(at) = defined.iter().position(|(l, _)| l == label) {
                let (label, blocks) = defined.remove(at);
                footnotes.push(Footnote {
                    label,
                    number: i + 1,
                    blocks,
                });
            }
        }
        // Those never referred to, after the others.
        for (label, blocks) in defined {
            let number = footnotes.iter().map(|f| f.number).max().unwrap_or(0) + 1;
            footnotes.push(Footnote {
                label,
                number,
                blocks,
            });
        }
        Doc {
            properties: self.properties,
            blocks,
            footnotes,
        }
    }
}

fn heading_level(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Where a link to `url` goes; a wiki link's `url` is what stands
/// between its brackets before its `|`.
fn target(url: &str, wiki: bool) -> Target {
    if wiki {
        return match obsidian::wiki_link(url) {
            Some((name, Some(heading), _)) if name.is_empty() => Target::Heading(heading),
            Some((name, heading, _)) => Target::Note { name, heading },
            None => Target::Nowhere,
        };
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
    {
        return Target::Web(url.to_string());
    }
    if let Some(anchor) = url.strip_prefix('#') {
        return Target::Heading(obsidian::percent_decoded(anchor));
    }
    let (path, heading) = match url.split_once('#') {
        Some((path, heading)) => (path, Some(obsidian::percent_decoded(heading))),
        None => (url, None),
    };
    let path = obsidian::percent_decoded(path);
    if !lower.contains(':') && obsidian::is_markdown(&path) {
        return Target::Note {
            name: path,
            heading,
        };
    }
    Target::Nowhere
}

/// A quote's `blocks` as it reads: a callout when its first line is
/// `[!kind]`, a `+` or `-` after it and its title.
fn quote(mut blocks: Vec<Block>) -> Block {
    let first = match blocks.first() {
        Some(Block::Paragraph(first)) => first.clone(),
        _ => return Block::Quote(flat(blocks)),
    };
    let line_end = first.text.find('\n').unwrap_or(first.text.len());
    let line = first.text.get(..line_end).unwrap_or_default();
    let kind = line
        .strip_prefix("[!")
        .and_then(|rest| rest.split_once(']'))
        .filter(|(kind, _)| {
            !kind.is_empty()
                && kind
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        });
    let Some((kind, after)) = kind else {
        return Block::Quote(flat(blocks));
    };
    let marked = 2 + kind.len() + 1;
    let fold = usize::from(after.starts_with(['+', '-']));
    let title_start = marked + fold;
    let title = first.slice(title_start, line_end);
    let lead = title.text.len() - title.text.trim_start().len();
    let title = title.slice(lead, title.text.len()).flat();
    let rest = first.slice((line_end + 1).min(first.text.len()), first.text.len());
    blocks.remove(0);
    if !rest.is_blank() {
        blocks.insert(0, Block::Paragraph(rest.flat()));
    }
    let kind = kind.to_lowercase();
    let title = if title.is_blank() {
        Inline::plain(&capitalized(&kind), Style::default())
    } else {
        title
    };
    Block::Callout {
        kind,
        title,
        blocks,
    }
}

/// `blocks` with the soft line breaks of the first paragraph as spaces.
fn flat(mut blocks: Vec<Block>) -> Vec<Block> {
    if let Some(Block::Paragraph(first)) = blocks.first_mut() {
        *first = std::mem::take(first).flat();
    }
    blocks
}

/// `word` with its first letter in upper case.
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A fenced code block of `text` in `language`, its fence longer than any
/// run of backticks in it.
fn fenced(language: &str, text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat((longest + 1).max(3));
    let body = if text.ends_with('\n') || text.is_empty() {
        text.to_string()
    } else {
        format!("{text}\n")
    };
    format!("{fence}{language}\n{body}{fence}")
}

/// A table cell's source, `cell` as written in the file, as the app's
/// table draws it: without the pipes and blanks around it, its wiki
/// links and embeds as the words they show.
fn table_cell(cell: &str) -> String {
    let cell = cell
        .trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .trim();
    let mut out = String::with_capacity(cell.len());
    let mut at = 0;
    for link in obsidian::wiki_links(cell) {
        out.push_str(cell.get(at..link.start).unwrap_or_default());
        out.push_str(&link.shown());
        at = link.end;
    }
    out.push_str(cell.get(at..).unwrap_or_default());
    out
}

/// The source of a table of `rows`, the first its header, aligned as
/// `aligns` says.
fn table_source(aligns: &[Alignment], rows: &[Vec<String>]) -> String {
    let columns = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(aligns.len())
        .max(1);
    let line = |cells: &[String]| {
        let mut out = String::from("|");
        for i in 0..columns {
            out.push(' ');
            out.push_str(cells.get(i).map_or("", String::as_str));
            out.push_str(" |");
        }
        out
    };
    let mut lines = Vec::new();
    let mut rows = rows.iter();
    lines.push(line(rows.next().map_or(&[][..], Vec::as_slice)));
    let delimiter: Vec<String> = (0..columns)
        .map(|i| {
            match aligns.get(i) {
                Some(Alignment::Left) => ":---",
                Some(Alignment::Center) => ":---:",
                Some(Alignment::Right) => "---:",
                _ => "---",
            }
            .to_string()
        })
        .collect();
    lines.push(line(&delimiter));
    for row in rows {
        lines.push(line(row));
    }
    lines.join("\n")
}

/// The properties of a YAML front matter `meta`: each `key: value` line,
/// a list under a key, `- item` lines or `[a, b]`, as its items apart by
/// commas, quotes around a value left out. Anything deeper shows as
/// written.
pub fn properties(meta: &str) -> Vec<(String, String)> {
    let unquoted = |v: &str| {
        let v = v.trim();
        let quoted = v.len() >= 2
            && ((v.starts_with('"') && v.ends_with('"'))
                || (v.starts_with('\'') && v.ends_with('\'')));
        if quoted {
            v.get(1..v.len() - 1).unwrap_or(v).to_string()
        } else {
            v.to_string()
        }
    };
    let mut out: Vec<(String, String)> = Vec::new();
    for line in meta.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indented = line.starts_with([' ', '\t']) || line.starts_with('-');
        if indented && let Some((_, value)) = out.last_mut() {
            let item = line.trim();
            let item = unquoted(item.strip_prefix('-').unwrap_or(item));
            if !value.is_empty() {
                value.push_str(", ");
            }
            value.push_str(&item);
            continue;
        }
        let (key, value) = match line.split_once(':') {
            Some((key, value)) => (key.trim(), value.trim()),
            None => (line.trim(), ""),
        };
        let value = match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
            Some(items) => items
                .split(',')
                .map(unquoted)
                .filter(|i| !i.is_empty())
                .collect::<Vec<_>>()
                .join(", "),
            None => unquoted(value),
        };
        out.push((unquoted(key), value));
    }
    out
}

/// The words a heading shows, as a link to it is matched: in lower case,
/// its blanks, `-` and `_` as one `-`, without any other mark, as GitHub
/// names a heading's place.
pub fn slug(heading: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in heading.trim().chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c);
        } else if c.is_whitespace() || c == '-' || c == '_' {
            gap = true;
        }
    }
    out
}

#[cfg(test)]
mod tests;
