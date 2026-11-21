//! AsciiDoc, Org, reStructuredText and HTML read into the app's Markdown
//! subset for an Import (#348, ADR 0033, ADR 0036), as the Dart core's
//! `import_formats.dart`, by hand and to the same bytes. What the subset
//! has becomes its Markdown: two heading levels, bold, italic,
//! strike-through, inline code, links, lists one level in, quotes, code
//! blocks and tables. What it lacks stays as the characters it is,
//! escaped so that it reads as itself; in HTML, whose marks are no text,
//! its words stay. Nothing of a document is run or fetched: an include
//! stays the line it is, a script or a style goes with its content, an
//! image is its description, and no other file is read.
//!
//! Every format is read by its `char`s, as the Dart core reads its code
//! points, and only ASCII characters decide anything, so both cores
//! agree on every text.

use std::collections::HashSet;

use crate::markdown;

/// A kind of document of another tool an Import takes besides Markdown
/// and text: each becomes a formatted Note in the Markdown subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportFormat {
    AsciiDoc,
    Org,
    RestructuredText,
    Html,
}

impl ImportFormat {
    /// The format of a file whose extension is `extension`, in lower
    /// case ASCII letters; `None` for any other.
    pub fn of_extension(extension: &str) -> Option<ImportFormat> {
        match extension {
            "adoc" | "asciidoc" | "asc" => Some(ImportFormat::AsciiDoc),
            "org" => Some(ImportFormat::Org),
            "rst" => Some(ImportFormat::RestructuredText),
            "html" | "htm" => Some(ImportFormat::Html),
            _ => None,
        }
    }
}

/// A document read for an Import: the title its page names apart from
/// its text, HTML's `<title>`, empty for none, and its text in the
/// Markdown subset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedDocument {
    pub title: String,
    pub text: String,
}

/// `text`, a document in `format` with its line breaks as `\n`, in the
/// Markdown subset.
pub fn read(format: ImportFormat, text: &str) -> ImportedDocument {
    let plain = |text: String| ImportedDocument {
        title: String::new(),
        text,
    };
    match format {
        ImportFormat::AsciiDoc => plain(ascii_doc(text)),
        ImportFormat::Org => plain(org(text)),
        ImportFormat::RestructuredText => plain(rst(text)),
        ImportFormat::Html => Html::new(text).read(),
    }
}

// ------------------------------------------------------------ characters

/// A space or a tab, the only blanks the lightweight formats know.
fn ws(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// A character of a word: an ASCII letter or digit, `_`, or any beyond
/// ASCII.
fn word(c: char) -> bool {
    alnum(c) || c == '_' || !c.is_ascii()
}

/// The part of `v` from `from` to `to`; empty where it would run past
/// its end, which the readers never ask for.
fn sub<T>(v: &[T], from: usize, to: usize) -> &[T] {
    v.get(from..to).unwrap_or_default()
}

/// The part of `v` from `from` on.
fn tail<T>(v: &[T], from: usize) -> &[T] {
    v.get(from..).unwrap_or_default()
}

/// The character at `i` of `c`; `'\0'` past its end, which the readers
/// never reach.
fn ch_at(c: &[char], i: usize) -> char {
    c.get(i).copied().unwrap_or('\0')
}

/// The line at `i` of `lines`; empty past their end.
fn line_at<'a>(lines: &[&'a str], i: usize) -> &'a str {
    lines.get(i).copied().unwrap_or("")
}

fn cps(text: &str) -> Vec<char> {
    text.chars().collect()
}

fn s(c: &[char], from: usize, to: usize) -> String {
    c.get(from..to).unwrap_or_default().iter().collect()
}

fn rest(c: &[char], from: usize) -> String {
    c.get(from..).unwrap_or_default().iter().collect()
}

/// Whether `c` holds the ASCII `s` at `i`, in any case of ASCII letters
/// with `any_case`.
fn at(c: &[char], i: usize, s: &str, any_case: bool) -> bool {
    let n = s.len();
    if i + n > c.len() {
        return false;
    }
    s.chars().zip(sub(c, i, i + n)).all(|(a, &d)| {
        let d = if any_case { d.to_ascii_lowercase() } else { d };
        a == d
    })
}

/// How many spaces and tabs `c` starts with.
fn indent(c: &[char]) -> usize {
    c.iter().take_while(|&&d| ws(d)).count()
}

fn trim_start(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

fn trim_end(s: &str) -> &str {
    s.trim_end_matches([' ', '\t'])
}

fn trim(s: &str) -> &str {
    trim_start(trim_end(s))
}

fn blank(s: &str) -> bool {
    trim(s).is_empty()
}

/// The first word of `s`, up to a space or a tab.
fn first_word(s: &str) -> &str {
    let t = trim_start(s);
    t.split([' ', '\t']).next().unwrap_or("")
}

/// The first `k` from `from` up to `to` that is `ok`.
fn find(from: usize, to: usize, ok: impl Fn(usize) -> bool) -> Option<usize> {
    (from..to).find(|&k| ok(k))
}

fn lower_str(c: &[char], from: usize, to: usize) -> String {
    c.get(from..to)
        .unwrap_or_default()
        .iter()
        .map(char::to_ascii_lowercase)
        .collect()
}

/// `lines` without the spaces and tabs they all start with, a blank one
/// empty.
fn dedent(lines: &[&str]) -> Vec<String> {
    let least = lines
        .iter()
        .filter(|line| !blank(line))
        .map(|line| indent(&cps(line)))
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| {
            if blank(line) {
                String::new()
            } else {
                line.chars().skip(least).collect()
            }
        })
        .collect()
}

// ----------------------------------------------------------------- spans

const BOLD: u8 = 1;
const ITALIC: u8 = 2;
const STRIKE: u8 = 4;
const CODE: u8 = 8;

/// A run of text in one style, part of a link to `link` when set.
#[derive(Clone, Debug)]
struct Span {
    text: String,
    style: u8,
    link: Option<String>,
}

/// `href` as a link's destination: when the subset can write it, without
/// spaces, angle brackets or backslashes and with its parentheses
/// balanced, and the apps may open it ([`markdown::may_open`]); else
/// `None` and its label is text.
fn target(href: &str) -> Option<String> {
    if href.is_empty() {
        return None;
    }
    let mut depth = 0i32;
    for c in href.chars() {
        if c <= ' ' || c == '\u{7f}' || c == '<' || c == '>' || c == '\\' {
            return None;
        }
        if c == '(' {
            depth += 1;
        }
        if c == ')' {
            depth -= 1;
            if depth < 0 {
                return None;
            }
        }
    }
    (depth == 0 && markdown::may_open(href)).then(|| href.to_string())
}

/// Whether `c`, beside a `_`, keeps it from reading as a mark: an ASCII
/// letter or digit, or a character beyond ASCII that is no space.
fn plain_beside(c: char) -> bool {
    alnum(c)
        || (!c.is_ascii()
            && !matches!(
                c,
                '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                    ..='\u{200a}'
                        | '\u{2028}'
                        | '\u{2029}'
                        | '\u{202f}'
                        | '\u{205f}'
                        | '\u{3000}'
                        | '\u{feff}'
            ))
}

/// Where the mark lies that the line `c` would start with in the subset:
/// a heading's `#`, a quote's `>`, a bulleted item's `-`, `*` or `+`, a
/// numbered item's `.`.
fn line_mark(c: &[char]) -> Option<usize> {
    let i = indent(c);
    let width: usize = sub(c, 0, i)
        .iter()
        .map(|&d| if d == '\t' { 4 } else { 1 })
        .sum();
    let mark = *c.get(i)?;
    let ends = |k: usize| c.get(k).is_none_or(|&d| ws(d));
    if width <= 3 && mark == '#' && (ends(i + 1) || (ch_at(c, i + 1) == '#' && ends(i + 2))) {
        return Some(i);
    }
    if width <= 3 && mark == '>' {
        return Some(i);
    }
    if matches!(mark, '-' | '*' | '+') && ends(i + 1) {
        return Some(i);
    }
    let k = i + tail(c, i).iter().take_while(|&&d| digit(d)).count();
    if k > i && k - i <= 9 && c.get(k) == Some(&'.') && ends(k + 1) {
        return Some(k);
    }
    None
}

/// `text`, a part of a line, as it reads in the subset as itself, as
/// [`markdown::escape`] writes it but in a time that grows with its
/// length alone: a `\` before the mark a line would start with, before
/// every `*`, `` ` ``, `[` and `|`, before a `~` next to another, before
/// a `_` not between two characters that are neither space nor
/// punctuation, and before a `\` before ASCII punctuation. A `\` also
/// goes before what a mark beside the part could join: a `~` at either
/// end, a `!` or a `\` at its end; in a link's `label` also before each
/// `]`.
fn escape(text: &str, label: bool) -> String {
    let c = cps(text);
    let c = c.as_slice();
    let mark = line_mark(c);
    let mut out = String::with_capacity(text.len());
    let last = c.len().saturating_sub(1);
    for (i, &ch) in c.iter().enumerate() {
        let escaped = Some(i) == mark
            || match ch {
                '\\' => i == last || ch_at(c, i + 1).is_ascii_punctuation(),
                '*' | '`' | '[' | '|' => true,
                '~' => i == 0 || i == last || ch_at(c, i - 1) == '~' || ch_at(c, i + 1) == '~',
                '_' => {
                    i == 0
                        || i == last
                        || !plain_beside(ch_at(c, i - 1))
                        || !plain_beside(ch_at(c, i + 1))
                }
                '!' => i == last,
                ']' => label,
                _ => false,
            };
        if escaped {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Whether a web address starts at `i` of `c`, as the apps find one: not
/// right after an ASCII letter or digit, and something after its scheme.
fn address_at(c: &[char], i: usize) -> bool {
    if i > 0 && alnum(ch_at(c, i - 1)) {
        return false;
    }
    let after = if at(c, i, "https://", true) {
        i + 8
    } else if at(c, i, "http://", true) {
        i + 7
    } else {
        return false;
    };
    c.get(after).is_some_and(|&d| alnum(d) || !d.is_ascii())
}

fn address_ends(c: char) -> bool {
    c <= ' ' || c == '\u{7f}' || c == '<' || c == '>'
}

/// The longest run of backticks in `text`.
fn longest_backticks(text: &str) -> usize {
    let (mut longest, mut run) = (0, 0);
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}

/// `text` as inline code, between backticks more than any run in it.
fn code_span(text: &str, cell: bool) -> String {
    let content = if cell {
        text.replace('|', "\\|")
    } else {
        text.to_string()
    };
    let fence = "`".repeat(longest_backticks(&content) + 1);
    let pad = if content.starts_with('`') || content.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{content}{pad}{fence}")
}

fn mark(style: u8) -> &'static str {
    match style {
        STRIKE => "~~",
        BOLD => "**",
        _ => "*",
    }
}

/// `text` split into the spaces and tabs it starts with, its core and the
/// spaces and tabs it ends with.
fn edges(text: &str) -> (&str, &str, &str) {
    let start = text.len() - trim_start(text).len();
    let end = trim_end(text).len().max(start);
    (&text[..start], &text[start..end], &text[end..])
}

/// `spans` of no link, or of one link's label, with their marks: each
/// mark opened where its style starts and closed where it ends, in the
/// order they nest, and the blanks at a span's ends outside them.
fn styled(spans: &[Span], cell: bool, label: bool) -> String {
    let mut out = String::new();
    let mut open: Vec<u8> = Vec::new();
    let mut pending = String::new();
    for span in spans {
        let (lead, core, trail) = edges(&span.text);
        if core.is_empty() {
            pending.push_str(&span.text);
            continue;
        }
        let want = span.style & (STRIKE | BOLD | ITALIC);
        let keep = open.iter().take_while(|&&o| want & o != 0).count();
        for &o in tail(&open, keep).iter().rev() {
            out.push_str(mark(o));
        }
        open.truncate(keep);
        out.push_str(&pending);
        out.push_str(lead);
        for style in [STRIKE, BOLD, ITALIC] {
            if want & style != 0 && !open.contains(&style) {
                out.push_str(mark(style));
                open.push(style);
            }
        }
        if span.style & CODE != 0 {
            out.push_str(&code_span(core, cell));
        } else {
            out.push_str(&escape(core, label));
        }
        pending = trail.to_string();
    }
    for &o in open.iter().rev() {
        out.push_str(mark(o));
    }
    out.push_str(&pending);
    out
}

/// `spans` as one line of the subset, without the blanks at its end; in
/// a table's `cell` every `|` escaped.
fn inline(spans: &[Span], cell: bool) -> String {
    let mut runs: Vec<Span> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        match runs.last_mut() {
            Some(last) if last.style == span.style && last.link == span.link => {
                last.text.push_str(&span.text);
            }
            _ => runs.push(span.clone()),
        }
    }
    let mut out = String::new();
    let mut i = 0;
    while i < runs.len() {
        let link = runs.get(i).and_then(|r| r.link.clone());
        let mut j = i + 1;
        while runs.get(j).is_some_and(|r| r.link == link) {
            j += 1;
        }
        let group = sub(&runs, i, j);
        match link {
            Some(link) if !(cell && link.contains('|')) => {
                let label = styled(group, cell, true);
                let (lead, core, trail) = edges(&label);
                let shown = if core.is_empty() {
                    escape(&link, true)
                } else {
                    core.to_string()
                };
                out.push_str(&format!("{lead}[{shown}]({link}){trail}"));
            }
            link => out.push_str(&styled(group, cell, link.is_some())),
        }
        i = j;
    }
    trim_end(&out).to_string()
}

/// The lines of the Markdown a document becomes.
#[derive(Default)]
struct Out {
    lines: Vec<String>,
}

impl Out {
    /// A blank line, unless one is there already or nothing is.
    fn blank(&mut self) {
        if self.lines.last().is_some_and(|l| !l.is_empty()) {
            self.lines.push(String::new());
        }
    }

    fn heading(&mut self, level: usize, spans: &[Span]) {
        let text = inline(spans, false);
        let text = trim_start(&text);
        if !text.is_empty() {
            let marks = if level == 1 { "#" } else { "##" };
            self.lines.push(format!("{marks} {text}"));
        }
    }

    /// A bulleted item, or a numbered one with `number`; `level` 1 is one
    /// level in.
    fn item(&mut self, level: usize, number: Option<u64>, spans: &[Span]) {
        let text = inline(spans, false);
        let text = trim_start(&text);
        if text.is_empty() {
            return;
        }
        let mark = match number {
            None => "-".to_string(),
            Some(n) => format!("{}.", n.min(999_999_999)),
        };
        let indent = if level == 0 { "" } else { "  " };
        self.lines.push(format!("{indent}{mark} {text}"));
    }

    fn quote(&mut self, spans: &[Span]) {
        let text = inline(spans, false);
        let text = trim_start(&text);
        self.lines.push(if text.is_empty() {
            ">".to_string()
        } else {
            format!("> {text}")
        });
    }

    fn paragraph(&mut self, spans: &[Span]) {
        let text = inline(spans, false);
        if trim_start(&text).is_empty() {
            self.blank();
        } else {
            self.lines.push(text);
        }
    }

    /// `line` as it is written, escaped.
    fn written(&mut self, line: &str) {
        self.paragraph(&[Span {
            text: line.to_string(),
            style: 0,
            link: None,
        }]);
    }

    /// A code block of `lines`, fenced by more backticks than any run in
    /// them, at least three; `language` its info string.
    fn code<S: AsRef<str>>(&mut self, language: &str, lines: &[S]) {
        let longest = lines
            .iter()
            .map(|l| longest_backticks(l.as_ref()))
            .max()
            .unwrap_or(0);
        let fence = "`".repeat(if longest < 3 { 3 } else { longest + 1 });
        let word = first_word(language);
        self.lines.push(if word.contains('`') {
            fence.clone()
        } else {
            format!("{fence}{word}")
        });
        self.lines
            .extend(lines.iter().map(|l| l.as_ref().to_string()));
        self.lines.push(fence);
    }

    /// A table of `rows`, the first one its header row, as many columns
    /// as the widest; on lines of its own, a blank line before and after
    /// it.
    fn table(&mut self, rows: &[Vec<Vec<Span>>]) {
        let Some(header) = rows.first() else {
            return;
        };
        let width = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let line = |row: &Vec<Vec<Span>>| {
            let cells: Vec<String> = (0..width)
                .map(|i| {
                    row.get(i)
                        .map(|cell| trim_start(&inline(cell, true)).to_string())
                        .unwrap_or_default()
                })
                .collect();
            format!("| {} |", cells.join(" | "))
        };
        self.blank();
        self.lines.push(line(header));
        self.lines
            .push(format!("| {} |", vec!["---"; width].join(" | ")));
        for row in tail(rows, 1) {
            self.lines.push(line(row));
        }
        self.lines.push(String::new());
    }

    fn text(mut self) -> String {
        while self.lines.last().is_some_and(String::is_empty) {
            self.lines.pop();
        }
        self.lines.join("\n")
    }
}

/// Collects the spans an inline reader finds, its plain characters in a
/// run until a span of another kind comes.
#[derive(Default)]
struct Inline {
    out: Vec<Span>,
    run: String,
    style: u8,
    link: Option<String>,
}

impl Inline {
    fn char(&mut self, c: char, style: u8, link: Option<&str>) {
        if style != self.style || link != self.link.as_deref() {
            self.flush();
        }
        self.style = style;
        self.link = link.map(str::to_string);
        self.run.push(c);
    }

    fn text(&mut self, text: &str, style: u8, link: Option<&str>) {
        for c in text.chars() {
            self.char(c, style, link);
        }
    }

    fn span(&mut self, text: &str, style: u8, link: Option<&str>) {
        self.flush();
        if !text.is_empty() {
            self.out.push(Span {
                text: text.to_string(),
                style,
                link: link.map(str::to_string),
            });
        }
    }

    fn flush(&mut self) {
        if self.run.is_empty() {
            return;
        }
        self.out.push(Span {
            text: std::mem::take(&mut self.run),
            style: self.style,
            link: self.link.clone(),
        });
    }

    fn is_empty(&self) -> bool {
        self.out.is_empty() && self.run.is_empty()
    }

    fn take(&mut self) -> Vec<Span> {
        self.flush();
        std::mem::take(&mut self.out)
    }
}

/// Where a closing mark was looked for and none found: none comes after
/// either, so it is not looked for again, and a line is read in a time
/// that grows with its length alone.
#[derive(Default)]
struct Missing {
    keys: HashSet<String>,
}

impl Missing {
    fn find(
        &mut self,
        key: &str,
        from: usize,
        to: usize,
        ok: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        if self.keys.contains(key) {
            return None;
        }
        let k = find(from, to, ok);
        if k.is_none() {
            self.keys.insert(key.to_string());
        }
        k
    }
}

// -------------------------------------------------------------- AsciiDoc

/// The delimiter `bare` is, a line without blanks at its end: four or
/// more `-` (listing), `.` (literal), `+` (passthrough), `/` (comment) or
/// `_` (quote), or `|===` (table), by that character; `None` for any
/// other line.
fn adoc_delimiter(bare: &str) -> Option<char> {
    let c = cps(bare);
    let c = c.as_slice();
    if c.len() >= 4 && ch_at(c, 0) == '|' && tail(c, 1).iter().all(|&d| d == '=') {
        return Some('|');
    }
    if c.len() < 4 || !c.iter().all(|&d| d == ch_at(c, 0)) {
        return None;
    }
    ['-', '.', '+', '/', '_']
        .contains(&ch_at(c, 0))
        .then_some(ch_at(c, 0))
}

/// `bare` without AsciiDoc's line break ` +` at its end.
fn adoc_break(bare: &str) -> String {
    match bare.strip_suffix(" +") {
        Some(before) => trim_end(before).to_string(),
        None => bare.to_string(),
    }
}

/// The delimited block opening at `start` in `lines`, a listing or
/// literal one in `language`; the index after it.
fn adoc_block(
    out: &mut Out,
    lines: &[&str],
    start: usize,
    language: &str,
    counts: &mut [u64; 2],
) -> usize {
    let open = trim_end(line_at(lines, start));
    let kind = adoc_delimiter(open);
    let mut end = start + 1;
    while end < lines.len() && trim_end(line_at(lines, end)) != open {
        end += 1;
    }
    let inside = sub(lines, start + 1, end);
    match kind {
        Some('-' | '.') => out.code(language, inside),
        Some('_') => {
            for line in inside {
                if blank(line) {
                    out.blank();
                } else {
                    out.quote(&adoc_spans(&adoc_break(trim(line))));
                }
            }
            out.blank();
        }
        Some('|') => out.table(&adoc_table(inside)),
        _ => {
            out.written(line_at(lines, start));
            for line in inside {
                out.written(line);
            }
            if end < lines.len() {
                out.written(line_at(lines, end));
            }
        }
    }
    *counts = [0, 0];
    end + 1
}

fn ascii_doc(text: &str) -> String {
    let mut out = Out::default();
    let lines: Vec<&str> = text.split('\n').collect();
    let lines = lines.as_slice();
    let mut counts = [0u64; 2];
    let mut i = 0;
    while i < lines.len() {
        let line = line_at(lines, i);
        let bare = trim_end(line);
        let c = cps(bare);
        let c = c.as_slice();
        if adoc_delimiter(bare).is_some() {
            i = adoc_block(&mut out, lines, i, "", &mut counts);
            continue;
        }
        if bare.is_empty() {
            out.blank();
            i += 1;
            continue;
        }
        // A block's attributes, such as `[source,python]` over a listing,
        // which names its language.
        if ch_at(c, 0) == '[' && c.last() == Some(&']') {
            let next = lines
                .get(i + 1)
                .and_then(|next| adoc_delimiter(trim_end(next)));
            if matches!(next, Some('-' | '.')) {
                let attributes: Vec<&str> = bare[1..bare.len() - 1].split(',').collect();
                let language = match attributes.as_slice() {
                    [first, second, ..] if trim(first) == "source" => trim(second),
                    _ => "",
                };
                i = adoc_block(&mut out, lines, i + 1, language, &mut counts);
                continue;
            }
            out.written(line);
            i += 1;
            continue;
        }
        if let Some(item) = adoc_item(c, &mut counts) {
            out.item(
                item.level,
                item.number,
                &adoc_spans(&adoc_break(&rest(c, item.start))),
            );
            i += 1;
            continue;
        }
        counts = [0, 0];
        let level = c.iter().take_while(|&&d| d == '=').count();
        if (1..=6).contains(&level) && c.get(level).is_some_and(|&d| ws(d)) {
            if level > 2 {
                out.written(line);
            } else {
                let mut title = trim(&rest(c, level)).to_string();
                // A title may close with its marks again.
                if let Some(closing) = title.rfind(" =")
                    && title[closing + 1..].chars().all(|d| d == '=')
                {
                    title = trim_end(&title[..closing]).to_string();
                }
                out.heading(level, &adoc_spans(&title));
            }
        } else if adoc_written(c) {
            out.written(line);
        } else if ch_at(c, 0) == '>' {
            out.quote(&adoc_spans(&adoc_break(&rest(c, 1))));
        } else {
            out.paragraph(&adoc_spans(&adoc_break(bare)));
        }
        i += 1;
    }
    out.text()
}

/// Whether the AsciiDoc line `c` stays as it is written: a comment, an
/// attribute entry, a block macro such as `include::` or `image::`, a
/// block title, or a literal line, indented.
fn adoc_written(c: &[char]) -> bool {
    if ws(ch_at(c, 0)) || at(c, 0, "//", false) {
        return true;
    }
    if ch_at(c, 0) == ':'
        && let Some(k) = find(1, c.len(), |k| ch_at(c, k) == ':')
        && k > 1
        && sub(c, 1, k)
            .iter()
            .all(|&d| word(d) || d == '-' || d == '!')
        && (k + 1 == c.len() || ws(ch_at(c, k + 1)))
    {
        return true;
    }
    if letter(ch_at(c, 0)) {
        let k = c
            .iter()
            .take_while(|&&d| alnum(d) || d == '-' || d == '_')
            .count();
        if at(c, k, "::", false) && c.last() == Some(&']') {
            return true;
        }
    }
    ch_at(c, 0) == '.' && c.len() > 1 && !ws(ch_at(c, 1)) && ch_at(c, 1) != '.'
}

/// A list item a line is: its level, its number for a numbered one, and
/// where its text starts.
struct Item {
    level: usize,
    number: Option<u64>,
    start: usize,
}

/// The list item the AsciiDoc line `c` is: `*` to `*****` or `-` for a
/// bulleted one, `.` to `.....` or a number and `.` for a numbered one,
/// each with a blank after it, `counts` counting the numbered ones of
/// each level; `None` for any other line.
fn adoc_item(c: &[char], counts: &mut [u64; 2]) -> Option<Item> {
    let start = indent(c);
    let mark = *c.get(start)?;
    let run = start + tail(c, start).iter().take_while(|&&d| d == mark).count();
    let n = run - start;
    let spaced = c.get(run).is_some_and(|&d| ws(d));
    if mark == '*' && n <= 5 && spaced {
        let level = usize::from(n > 1);
        return Some(Item {
            level,
            number: None,
            start: run + 1,
        });
    }
    if mark == '-' && n == 1 && spaced {
        return Some(Item {
            level: 0,
            number: None,
            start: run + 1,
        });
    }
    if mark == '.' && n <= 5 && spaced {
        let level = usize::from(n > 1);
        let number = counts.get(level).copied().unwrap_or(0) + 1;
        if let Some(count) = counts.get_mut(level) {
            *count = number;
        }
        if level == 0 {
            counts[1] = 0;
        }
        return Some(Item {
            level,
            number: Some(number),
            start: run + 1,
        });
    }
    let digits = start + tail(c, start).iter().take_while(|&&d| digit(d)).count();
    if digits > start
        && digits - start <= 9
        && at(c, digits, ".", false)
        && c.get(digits + 1).is_some_and(|&d| ws(d))
    {
        let number = s(c, start, digits).parse::<u64>().ok()?;
        *counts = [number, 0];
        return Some(Item {
            level: 0,
            number: Some(number),
            start: digits + 2,
        });
    }
    None
}

/// The cells of an AsciiDoc table's `lines`, in rows as many as the first
/// line holding cells has; a line that holds none goes on with the cell
/// before it.
fn adoc_table(lines: &[&str]) -> Vec<Vec<Vec<Span>>> {
    let mut cells: Vec<String> = Vec::new();
    let mut width = 0;
    for line in lines {
        let t = trim(line);
        if t.is_empty() {
            continue;
        }
        let c = cps(t);
        let c = c.as_slice();
        let bar = find(0, c.len(), |k| ch_at(c, k) == '|');
        let spec = bar.filter(|&bar| {
            bar <= 6
                && sub(c, 0, bar)
                    .iter()
                    .all(|&d| alnum(d) || matches!(d, '.' | '+' | '*' | '<' | '>' | '^'))
        });
        let Some(bar) = spec else {
            match cells.last_mut() {
                Some(last) => {
                    last.push(' ');
                    last.push_str(t);
                }
                None => cells.push(t.to_string()),
            }
            continue;
        };
        let mut found = 0;
        let mut from = bar + 1;
        for k in bar + 1..=c.len() {
            if k < c.len() && (ch_at(c, k) != '|' || ch_at(c, k - 1) == '\\') {
                continue;
            }
            cells.push(trim(&s(c, from, k).replace("\\|", "|")).to_string());
            found += 1;
            from = k + 1;
        }
        if width == 0 {
            width = found;
        }
    }
    let width = width.max(1);
    cells
        .chunks(width)
        .map(|row| row.iter().map(|cell| adoc_spans(cell)).collect())
        .collect()
}

fn adoc_spans(text: &str) -> Vec<Span> {
    let mut into = Inline::default();
    let c = cps(text);
    let c = c.as_slice();
    adoc_inline(c, 0, c.len(), 0, None, &mut into);
    into.take()
}

/// The characters a `\` before them shows as themselves in AsciiDoc.
fn adoc_escapes(c: char) -> bool {
    matches!(
        c,
        '*' | '_' | '`' | '#' | '+' | '[' | '<' | '{' | '\\' | '^' | '~'
    )
}

/// `text` without the `+` around it that AsciiDoc's `` `+text+` `` keeps
/// as written.
fn unplus(text: &str) -> &str {
    if text.len() >= 2 && text.starts_with('+') && text.ends_with('+') {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// Reads `c` from `from` to `to` in `style`, a link's label when `link`
/// is set: `*bold*` and `**bold**`, `_italic_` and `__italic__`,
/// `` `code` `` and ``` ``code`` ```, `+text+` and `++text++` as written,
/// `[.line-through]#text#`, and web addresses, `link:` and `mailto:`
/// with a label in `[…]`; anything else as written.
fn adoc_inline(
    c: &[char],
    from: usize,
    to: usize,
    style: u8,
    link: Option<&str>,
    into: &mut Inline,
) {
    let mut missing = Missing::default();
    let opens =
        |i: usize| (i == from || !word(ch_at(c, i - 1))) && i + 1 < to && !ws(ch_at(c, i + 1));
    let closes = |k: usize| k + 1 >= to || !word(ch_at(c, k + 1));
    let mut i = from;
    while i < to {
        let ch = ch_at(c, i);
        if ch == '\\' && i + 1 < to && adoc_escapes(ch_at(c, i + 1)) {
            into.char(ch_at(c, i + 1), style, link);
            i += 2;
            continue;
        }
        if link.is_none() && (i == from || !word(ch_at(c, i - 1))) {
            let address = address_at(c, i);
            let macro_ = if at(c, i, "link:", false) {
                Some(5)
            } else if at(c, i, "mailto:", false) {
                Some(0)
            } else {
                None
            };
            if address || macro_.is_some() {
                let start = i + macro_.unwrap_or(0);
                let mut end = start;
                while end < to && !address_ends(ch_at(c, end)) && ch_at(c, end) != '[' {
                    end += 1;
                }
                let close = if end < to && ch_at(c, end) == '[' {
                    missing.find("]", end + 1, to, |k| ch_at(c, k) == ']')
                } else {
                    None
                };
                if let Some(close) = close
                    && end > start
                {
                    let href = s(c, start, end);
                    let dest = target(&href);
                    let mut label_end = close;
                    if label_end > end + 1 && ch_at(c, label_end - 1) == '^' {
                        label_end -= 1;
                    }
                    if label_end > end + 1 {
                        adoc_inline(c, end + 1, label_end, style, dest.as_deref(), into);
                    } else {
                        into.span(&href, style, dest.as_deref());
                    }
                    i = close + 1;
                    continue;
                }
                if address {
                    into.text(&s(c, i, end), style, link);
                    i = end;
                    continue;
                }
            }
        }
        if ch == '`' && i + 1 < to && ch_at(c, i + 1) == '`' {
            if let Some(k) = missing.find("``", i + 3, to.saturating_sub(1), |k| {
                ch_at(c, k) == '`' && ch_at(c, k + 1) == '`'
            }) {
                into.span(unplus(&s(c, i + 2, k)), style | CODE, link);
                i = k + 2;
                continue;
            }
        } else if ch == '`'
            && opens(i)
            && let Some(k) = missing.find("`", i + 2, to, |k| {
                ch_at(c, k) == '`' && !ws(ch_at(c, k - 1)) && closes(k)
            })
        {
            into.span(unplus(&s(c, i + 1, k)), style | CODE, link);
            i = k + 1;
            continue;
        }
        let bit = match ch {
            '*' => BOLD,
            '_' => ITALIC,
            _ => 0,
        };
        if bit != 0 && style & bit == 0 {
            if i + 1 < to && ch_at(c, i + 1) == ch {
                if i + 2 < to
                    && !ws(ch_at(c, i + 2))
                    && let Some(k) =
                        missing.find(&format!("{ch}{ch}"), i + 3, to.saturating_sub(1), |k| {
                            ch_at(c, k) == ch && ch_at(c, k + 1) == ch && !ws(ch_at(c, k - 1))
                        })
                {
                    adoc_inline(c, i + 2, k, style | bit, link, into);
                    i = k + 2;
                    continue;
                }
            } else if opens(i)
                && let Some(k) = missing.find(&ch.to_string(), i + 2, to, |k| {
                    ch_at(c, k) == ch && !ws(ch_at(c, k - 1)) && closes(k)
                })
            {
                adoc_inline(c, i + 1, k, style | bit, link, into);
                i = k + 1;
                continue;
            }
        }
        if ch == '+' {
            let double = i + 1 < to && ch_at(c, i + 1) == '+';
            let k = if double {
                missing.find("++", i + 3, to.saturating_sub(1), |k| {
                    ch_at(c, k) == '+' && ch_at(c, k + 1) == '+'
                })
            } else if opens(i) {
                missing.find("+", i + 2, to, |k| {
                    ch_at(c, k) == '+' && !ws(ch_at(c, k - 1)) && closes(k)
                })
            } else {
                None
            };
            if let Some(k) = k {
                let skip = if double { 2 } else { 1 };
                into.text(&s(c, i + skip, k), style, link);
                i = k + skip;
                continue;
            }
        }
        if ch == '['
            && style & STRIKE == 0
            && at(c, i, "[.line-through]#", false)
            && let Some(k) = missing.find("#", i + 17, to, |k| ch_at(c, k) == '#')
        {
            adoc_inline(c, i + 16, k, style | STRIKE, link, into);
            i = k + 1;
            continue;
        }
        into.char(ch, style, link);
        i += 1;
    }
}

// ------------------------------------------------------------------- Org

/// `text` without Org's line break `\\` at its end.
fn org_break(text: &str) -> String {
    let bare = trim_end(text);
    match bare.strip_suffix("\\\\") {
        Some(before) => trim_end(before).to_string(),
        None => bare.to_string(),
    }
}

/// The list item the Org line `c`, indented by `start`, is: `-` or `+`,
/// or `*` when indented, for a bulleted one, a number and `.` or `)` for
/// a numbered one, each with a blank or the line's end after it; `None`
/// for any other line.
fn org_item(c: &[char], start: usize) -> Option<Item> {
    let mark = ch_at(c, start);
    let ends = |k: usize| k == c.len() || c.get(k).is_some_and(|&d| ws(d));
    if (mark == '-' || mark == '+' || (mark == '*' && start > 0)) && ends(start + 1) {
        return Some(Item {
            level: 0,
            number: None,
            start: start + 1,
        });
    }
    let digits = start + tail(c, start).iter().take_while(|&&d| digit(d)).count();
    if digits > start
        && digits - start <= 9
        && c.get(digits).is_some_and(|&d| d == '.' || d == ')')
        && ends(digits + 1)
    {
        return Some(Item {
            level: 0,
            number: Some(s(c, start, digits).parse().ok()?),
            start: digits + 1,
        });
    }
    None
}

/// The lines of an Org source or example block: without the indent they
/// all have, a `,` that keeps a `*` or `#+` at a line's start from
/// reading as Org taken off.
fn org_code(lines: &[&str]) -> Vec<String> {
    dedent(lines)
        .into_iter()
        .map(|line| {
            let c = cps(&line);
            let c = c.as_slice();
            let k = indent(c);
            if at(c, k, ",*", false) || at(c, k, ",#+", false) {
                format!("{}{}", s(c, 0, k), rest(c, k + 1))
            } else {
                line
            }
        })
        .collect()
}

/// The cells of the Org table row `row`, a line between `|`.
fn org_row(row: &str) -> Vec<Vec<Span>> {
    let inner = &row[1..];
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').map(|cell| org_spans(trim(cell))).collect()
}

fn org(text: &str) -> String {
    let mut out = Out::default();
    let lines: Vec<&str> = text.split('\n').collect();
    let lines = lines.as_slice();
    let mut base: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = line_at(lines, i);
        let c = cps(trim_end(line));
        let c = c.as_slice();
        let start = indent(c);
        if start == c.len() {
            out.blank();
            i += 1;
            continue;
        }
        let stars = c.iter().take_while(|&&d| d == '*').count();
        if stars > 0 && c.get(stars).is_some_and(|&d| ws(d)) {
            base = None;
            if stars > 2 {
                out.written(line);
            } else {
                out.heading(stars, &org_spans(trim(&rest(c, stars))));
            }
            i += 1;
            continue;
        }
        if at(c, start, "#+begin_", true) {
            let k = start + 8 + tail(c, start + 8).iter().take_while(|&&d| !ws(d)).count();
            let name = lower_str(c, start + 8, k);
            let args = rest(c, k);
            let close = format!("#+end_{name}");
            let mut end = i + 1;
            while end < lines.len() && !at(&cps(trim_start(line_at(lines, end))), 0, &close, true) {
                end += 1;
            }
            let inside = sub(lines, i + 1, end);
            match name.as_str() {
                "src" => out.code(first_word(&args), &org_code(inside)),
                "example" => out.code("", &org_code(inside)),
                "quote" => {
                    for line in inside {
                        if blank(line) {
                            out.blank();
                        } else {
                            out.quote(&org_spans(&org_break(trim(line))));
                        }
                    }
                    out.blank();
                }
                _ => {
                    out.written(line);
                    for line in inside {
                        out.written(line);
                    }
                    if end < lines.len() {
                        out.written(line_at(lines, end));
                    }
                }
            }
            base = None;
            i = end + 1;
            continue;
        }
        if at(c, start, "#+title:", true) && !blank(&rest(c, start + 8)) {
            out.heading(1, &org_spans(trim(&rest(c, start + 8))));
            base = None;
            i += 1;
            continue;
        }
        // Keywords, comments, drawers and fixed-width lines.
        if (ch_at(c, start) == '#' && c.get(start + 1).is_none_or(|&d| ws(d) || d == '+'))
            || ch_at(c, start) == ':'
        {
            out.written(line);
            i += 1;
            continue;
        }
        if ch_at(c, start) == '|' {
            let mut rows = Vec::new();
            while i < lines.len() {
                let row = trim(line_at(lines, i));
                if !row.starts_with('|') {
                    break;
                }
                if !row.starts_with("|-") {
                    rows.push(org_row(row));
                }
                i += 1;
            }
            out.table(&rows);
            base = None;
            continue;
        }
        if let Some(item) = org_item(c, start) {
            let first = *base.get_or_insert(start);
            out.item(
                usize::from(start > first),
                item.number,
                &org_spans(&org_break(&rest(c, item.start))),
            );
            i += 1;
            continue;
        }
        if base.is_some_and(|b| start <= b) {
            base = None;
        }
        out.paragraph(&org_spans(&org_break(&rest(c, start))));
        i += 1;
    }
    out.text()
}

fn org_spans(text: &str) -> Vec<Span> {
    let mut into = Inline::default();
    let c = cps(text);
    let c = c.as_slice();
    org_inline(c, 0, c.len(), 0, None, &mut into);
    into.take()
}

/// What may stand before an Org mark that opens.
fn org_before(c: char) -> bool {
    ws(c) || matches!(c, '-' | '(' | '{' | '\'' | '"')
}

/// What may stand after an Org mark that closes.
fn org_after(c: char) -> bool {
    ws(c)
        || matches!(
            c,
            '-' | '.' | ',' | ';' | ':' | '!' | '?' | '\'' | ')' | '}' | '[' | '"' | '\\'
        )
}

/// Reads `c` from `from` to `to` in `style`, a link's label when `link`
/// is set: `*bold*`, `/italic/`, `+strike-through+`, `=code=` and
/// `~code~`, links `[[address][label]]` and `[[address]]`, and web
/// addresses; anything else, `_underline_` among it, as written.
fn org_inline(
    c: &[char],
    from: usize,
    to: usize,
    style: u8,
    link: Option<&str>,
    into: &mut Inline,
) {
    let mut missing = Missing::default();
    let mut i = from;
    while i < to {
        let ch = ch_at(c, i);
        if link.is_none()
            && ch == '['
            && i + 1 < to
            && ch_at(c, i + 1) == '['
            && let Some(k) = missing.find("]]", i + 2, to.saturating_sub(1), |k| {
                ch_at(c, k) == ']' && ch_at(c, k + 1) == ']'
            })
        {
            let m = find(i + 2, k - 1, |m| {
                ch_at(c, m) == ']' && ch_at(c, m + 1) == '['
            });
            let href = s(c, i + 2, m.unwrap_or(k));
            let dest = target(&href);
            match m {
                Some(m) if m + 2 < k => org_inline(c, m + 2, k, style, dest.as_deref(), into),
                _ => into.span(&href, style, dest.as_deref()),
            }
            i = k + 2;
            continue;
        }
        if link.is_none() && address_at(c, i) {
            let mut end = i;
            while end < to && !address_ends(ch_at(c, end)) {
                end += 1;
            }
            into.text(&s(c, i, end), style, link);
            i = end;
            continue;
        }
        let bit = match ch {
            '*' => BOLD,
            '/' => ITALIC,
            '+' => STRIKE,
            '=' | '~' => CODE,
            _ => 0,
        };
        if bit != 0
            && style & bit == 0
            && (i == from || org_before(ch_at(c, i - 1)))
            && i + 1 < to
            && !ws(ch_at(c, i + 1))
            && let Some(k) = missing.find(&ch.to_string(), i + 2, to, |k| {
                ch_at(c, k) == ch
                    && !ws(ch_at(c, k - 1))
                    && (k + 1 == to || org_after(ch_at(c, k + 1)))
            })
        {
            if bit == CODE {
                into.span(&s(c, i + 1, k), style | CODE, link);
            } else {
                org_inline(c, i + 1, k, style | bit, link, into);
            }
            i = k + 1;
            continue;
        }
        into.char(ch, style, link);
        i += 1;
    }
}

// ------------------------------------------------------ reStructuredText

/// Whether `bare` is an adornment: two or more of one ASCII punctuation
/// character.
fn adornment(bare: &str) -> bool {
    let c = cps(bare);
    let c = c.as_slice();
    c.len() >= 2 && ch_at(c, 0).is_ascii_punctuation() && c.iter().all(|&d| d == ch_at(c, 0))
}

/// The name, in lower case ASCII letters, and the arguments of the
/// directive `name:: arguments` in `c` after its `..` at `from`; `None`
/// when it is none, such as a comment or a target.
fn rst_directive(c: &[char], from: usize) -> Option<(String, String)> {
    let start = from + c.get(from..)?.iter().take_while(|&&d| ws(d)).count();
    let mut k = start;
    while k < c.len() && !ws(ch_at(c, k)) && !at(c, k, "::", false) {
        k += 1;
    }
    if k == start || !at(c, k, "::", false) {
        return None;
    }
    Some((lower_str(c, start, k), trim(&rest(c, k + 2)).to_string()))
}

/// A list item of reStructuredText: its number, whether `#` counts on,
/// and where its text starts.
struct RstItem {
    number: Option<u64>,
    auto: bool,
    start: usize,
}

/// The list item the reStructuredText line `c`, indented by `start`, is:
/// `-`, `*` or `+` for a bulleted one; a number or `#` with `.` or `)`
/// after it, or between `(` and `)`, for a numbered one, `#` counting on;
/// each with a blank or the line's end after it. `None` for any other
/// line.
fn rst_item(c: &[char], start: usize) -> Option<RstItem> {
    let spaced = |k: usize| k == c.len() || c.get(k).is_some_and(|&d| ws(d));
    let mark = ch_at(c, start);
    if matches!(mark, '-' | '*' | '+') && spaced(start + 1) {
        return Some(RstItem {
            number: None,
            auto: false,
            start: start + 1,
        });
    }
    let paren = mark == '(';
    let mut k = if paren { start + 1 } else { start };
    let mut number = None;
    let mut auto = false;
    if c.get(k) == Some(&'#') {
        auto = true;
        k += 1;
    } else {
        let digits = k;
        while k < c.len() && digit(ch_at(c, k)) {
            k += 1;
        }
        if k == digits || k - digits > 9 {
            return None;
        }
        number = Some(s(c, digits, k).parse().ok()?);
    }
    let close = *c.get(k)?;
    if if paren {
        close != ')'
    } else {
        close != '.' && close != ')'
    } {
        return None;
    }
    if !spaced(k + 1) {
        return None;
    }
    Some(RstItem {
        number,
        auto,
        start: k + 1,
    })
}

/// The rows of a grid table's `lines`, each row's lines joined cell by
/// cell, the first row its header.
fn grid_rows(lines: &[&str]) -> Vec<Vec<Vec<Span>>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let end = |row: &mut Vec<String>, rows: &mut Vec<Vec<Vec<Span>>>| {
        if row.iter().any(|cell| !cell.is_empty()) {
            rows.push(row.iter().map(|cell| rst_spans(cell)).collect());
        }
        row.clear();
    };
    for line in lines {
        let t = trim(line);
        if t.starts_with('+') {
            end(&mut row, &mut rows);
            continue;
        }
        let inner = t.get(1..).unwrap_or("");
        let inner = inner.strip_suffix('|').unwrap_or(inner);
        for (k, part) in inner.split('|').enumerate() {
            let part = trim(part);
            while row.len() <= k {
                row.push(String::new());
            }
            if !part.is_empty()
                && let Some(cell) = row.get_mut(k)
            {
                if !cell.is_empty() {
                    cell.push(' ');
                }
                cell.push_str(part);
            }
        }
    }
    end(&mut row, &mut rows);
    rows
}

/// A section title in `style`, as the subset's heading for the first two
/// styles met and as `written` for those under them.
fn rst_title(
    out: &mut Out,
    styles: &mut Vec<String>,
    style: String,
    title: &str,
    written: &[&str],
) {
    let level = match styles.iter().position(|s| *s == style) {
        Some(p) => p + 1,
        None => {
            styles.push(style);
            styles.len()
        }
    };
    if level <= 2 {
        out.heading(level, &rst_spans(title));
    } else {
        for line in written {
            out.written(line);
        }
    }
}

/// The index after the lines of `lines` from `from` on that are blank or
/// indented more than `by`, without the blank ones at their end.
fn block_end(lines: &[&str], from: usize, by: usize) -> usize {
    let mut end = from;
    while end < lines.len()
        && (blank(line_at(lines, end)) || indent(&cps(line_at(lines, end))) > by)
    {
        end += 1;
    }
    while end > from && blank(line_at(lines, end - 1)) {
        end -= 1;
    }
    end
}

fn rst(text: &str) -> String {
    let mut out = Out::default();
    let lines: Vec<&str> = text.split('\n').collect();
    let lines = lines.as_slice();
    let mut styles: Vec<String> = Vec::new();
    let mut counts = [0u64; 2];
    let mut base: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = line_at(lines, i);
        let bare = trim_end(line);
        let c = cps(bare);
        let c = c.as_slice();
        let start = indent(c);
        if start == c.len() {
            out.blank();
            i += 1;
            continue;
        }
        // A directive, a comment or a target, with what is indented under it.
        if at(c, start, "..", false) && c.get(start + 2).is_none_or(|&d| ws(d)) {
            let end = block_end(lines, i + 1, start);
            let directive = rst_directive(c, start + 2);
            match directive {
                Some((name, args))
                    if ["code", "code-block", "sourcecode"].contains(&name.as_str()) =>
                {
                    let body = sub(lines, i + 1, end);
                    let options = body
                        .iter()
                        .take_while(|line| trim_start(line).starts_with(':'))
                        .count();
                    let code = dedent(tail(body, options));
                    let first = code.iter().take_while(|line| line.is_empty()).count();
                    out.code(first_word(&args), tail(&code, first));
                }
                _ => {
                    for line in sub(lines, i, end) {
                        out.written(line);
                    }
                }
            }
            base = None;
            i = end;
            continue;
        }
        let adorned = |line: &str| line.chars().next().map(String::from).unwrap_or_default();
        if start == 0 && adornment(bare) {
            if i + 2 < lines.len()
                && !blank(line_at(lines, i + 1))
                && trim_end(line_at(lines, i + 2)) == bare
            {
                rst_title(
                    &mut out,
                    &mut styles,
                    format!("o{}", adorned(bare)),
                    trim(line_at(lines, i + 1)),
                    sub(lines, i, i + 3),
                );
                base = None;
                i += 3;
            } else {
                // A transition.
                out.written(line);
                i += 1;
            }
            continue;
        }
        if start == 0 && i + 1 < lines.len() {
            let under = trim_end(line_at(lines, i + 1));
            if adornment(under) && under.chars().count() >= c.len() {
                rst_title(
                    &mut out,
                    &mut styles,
                    format!("u{}", adorned(under)),
                    bare,
                    sub(lines, i, i + 2),
                );
                base = None;
                i += 2;
                continue;
            }
        }
        if at(c, start, "+-", false) || at(c, start, "+=", false) {
            let mut end = i;
            while end < lines.len() {
                let t = trim_start(line_at(lines, end));
                if !t.starts_with('+') && !t.starts_with('|') {
                    break;
                }
                end += 1;
            }
            out.table(&grid_rows(sub(lines, i, end)));
            base = None;
            i = end;
            continue;
        }
        // A field, such as `:Author: Ada`.
        if ch_at(c, start) == ':' {
            out.written(line);
            i += 1;
            continue;
        }
        if let Some(item) = rst_item(c, start) {
            let first = *base.get_or_insert(start);
            let level = usize::from(start > first);
            let number = item.number.or_else(|| {
                item.auto
                    .then(|| counts.get(level).copied().unwrap_or(0) + 1)
            });
            if let Some(number) = number {
                if let Some(count) = counts.get_mut(level) {
                    *count = number;
                }
                if level == 0 {
                    counts[1] = 0;
                }
            }
            out.item(level, number, &rst_spans(&rest(c, item.start)));
            i += 1;
            continue;
        }
        if base.is_some_and(|b| start <= b) {
            base = None;
            counts = [0, 0];
        }
        let mut content = rest(c, start);
        if let Some(before) = content.strip_suffix("::") {
            // A literal block follows; its `::` shows as one `:`, or as
            // nothing after a blank or alone.
            content = if before.is_empty() {
                String::new()
            } else if before.chars().last().is_some_and(ws) {
                trim_end(before).to_string()
            } else {
                format!("{before}:")
            };
            if !content.is_empty() {
                out.paragraph(&rst_spans(&content));
            }
            let mut from = i + 1;
            while from < lines.len() && blank(line_at(lines, from)) {
                from += 1;
            }
            let end = block_end(lines, from, start);
            if end > from {
                out.code("", &dedent(sub(lines, from, end)));
                i = end;
            } else {
                i += 1;
            }
            continue;
        }
        out.paragraph(&rst_spans(&content));
        i += 1;
    }
    out.text()
}

fn rst_spans(text: &str) -> Vec<Span> {
    let mut into = Inline::default();
    let c = cps(text);
    let c = c.as_slice();
    rst_inline(c, 0, c.len(), 0, None, &mut into);
    into.take()
}

/// What may stand before reStructuredText's inline markup.
fn rst_before(c: char) -> bool {
    ws(c) || matches!(c, '-' | ':' | '/' | '\'' | '"' | '<' | '(' | '[' | '{')
}

/// What may stand after reStructuredText's inline markup.
fn rst_after(c: char) -> bool {
    ws(c)
        || matches!(
            c,
            '-' | '.'
                | ','
                | ':'
                | ';'
                | '!'
                | '?'
                | '\\'
                | '/'
                | '\''
                | '"'
                | ')'
                | ']'
                | '}'
                | '>'
        )
}

/// The index after the one or two `_` of a reference from `from` on.
fn underscores(c: &[char], from: usize, to: usize) -> usize {
    let mut k = from;
    while k < to && k - from < 2 && ch_at(c, k) == '_' {
        k += 1;
    }
    k
}

/// The label and the address of the reference `c` holds from `from` to
/// `to`, `label <address>`; the address `None` when it names none.
fn rst_reference(c: &[char], from: usize, to: usize) -> (String, Option<String>) {
    if to - from >= 2
        && ch_at(c, to - 1) == '>'
        && let Some(lt) = (from..to - 1).rev().find(|&k| ch_at(c, k) == '<')
        && (lt == from || ws(ch_at(c, lt - 1)))
    {
        return (
            trim(&s(c, from, lt)).to_string(),
            Some(s(c, lt + 1, to - 1)),
        );
    }
    (s(c, from, to), None)
}

/// Reads `c` from `from` to `to` in `style`: `**bold**`, `*italic*`,
/// ``` ``code`` ```, the reference `` `label <address>`_ ``, a `\` before
/// a character, and web addresses; anything else, interpreted text and
/// roles among it, as written.
fn rst_inline(
    c: &[char],
    from: usize,
    to: usize,
    style: u8,
    link: Option<&str>,
    into: &mut Inline,
) {
    let mut missing = Missing::default();
    let after = |p: usize| p >= to || rst_after(ch_at(c, p));
    let mut i = from;
    while i < to {
        let ch = ch_at(c, i);
        if ch == '\\' {
            if i + 1 < to && !ws(ch_at(c, i + 1)) {
                into.char(ch_at(c, i + 1), style, link);
            }
            i += 2;
            continue;
        }
        if link.is_none() && address_at(c, i) {
            let mut end = i;
            while end < to && !address_ends(ch_at(c, end)) {
                end += 1;
            }
            into.text(&s(c, i, end), style, link);
            i = end;
            continue;
        }
        let opens = i == from || rst_before(ch_at(c, i - 1));
        if opens && ch == '`' && i + 2 < to && ch_at(c, i + 1) == '`' && !ws(ch_at(c, i + 2)) {
            if let Some(k) = missing.find("``", i + 3, to.saturating_sub(1), |k| {
                ch_at(c, k) == '`' && ch_at(c, k + 1) == '`' && !ws(ch_at(c, k - 1)) && after(k + 2)
            }) {
                into.span(&s(c, i + 2, k), style | CODE, link);
                i = k + 2;
                continue;
            }
        } else if opens
            && ch == '`'
            && i + 1 < to
            && !ws(ch_at(c, i + 1))
            && let Some(k) = missing.find("`", i + 2, to, |k| {
                ch_at(c, k) == '`' && !ws(ch_at(c, k - 1)) && after(underscores(c, k + 1, to))
            })
        {
            let end = underscores(c, k + 1, to);
            if end > k + 1 {
                let (label, href) = rst_reference(c, i + 1, k);
                let dest = href.as_deref().and_then(target);
                let shown = if label.is_empty() {
                    href.unwrap_or_default()
                } else {
                    label
                };
                into.span(&shown, style, dest.as_deref());
            } else {
                into.text(&s(c, i, end), style, link);
            }
            i = end;
            continue;
        }
        if opens
            && ch == '*'
            && style & BOLD == 0
            && i + 2 < to
            && ch_at(c, i + 1) == '*'
            && !ws(ch_at(c, i + 2))
        {
            if let Some(k) = missing.find("**", i + 3, to.saturating_sub(1), |k| {
                ch_at(c, k) == '*' && ch_at(c, k + 1) == '*' && !ws(ch_at(c, k - 1)) && after(k + 2)
            }) {
                rst_inline(c, i + 2, k, style | BOLD, link, into);
                i = k + 2;
                continue;
            }
        } else if opens
            && ch == '*'
            && style & ITALIC == 0
            && i + 1 < to
            && ch_at(c, i + 1) != '*'
            && !ws(ch_at(c, i + 1))
            && let Some(k) = missing.find("*", i + 2, to, |k| {
                ch_at(c, k) == '*' && !ws(ch_at(c, k - 1)) && after(k + 1)
            })
        {
            rst_inline(c, i + 1, k, style | ITALIC, link, into);
            i = k + 1;
            continue;
        }
        into.char(ch, style, link);
        i += 1;
    }
}

// ------------------------------------------------------------------ HTML

fn html_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{c}' | '\r')
}

fn tag_char(c: char) -> bool {
    alnum(c) || c == '-' || c == ':'
}

/// `s` without the HTML blanks around it, those inside it one space.
fn collapse(s: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in s.chars() {
        if html_space(c) {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    out
}

/// The elements that end a line of text where they start and end.
const HTML_BLOCKS: [&str; 25] = [
    "address",
    "article",
    "aside",
    "body",
    "caption",
    "center",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "header",
    "hgroup",
    "html",
    "legend",
    "main",
    "nav",
    "section",
    "summary",
];

/// The elements whose content is never text: a script, a style, a
/// template.
const HTML_DROPPED: [&str; 3] = ["script", "style", "template"];

/// The named character references read, the Latin-1 ones by their place
/// from U+00A0 on.
const LATIN1: [&str; 96] = [
    "nbsp", "iexcl", "cent", "pound", "curren", "yen", "brvbar", "sect", "uml", "copy", "ordf",
    "laquo", "not", "shy", "reg", "macr", "deg", "plusmn", "sup2", "sup3", "acute", "micro",
    "para", "middot", "cedil", "sup1", "ordm", "raquo", "frac14", "frac12", "frac34", "iquest",
    "Agrave", "Aacute", "Acirc", "Atilde", "Auml", "Aring", "AElig", "Ccedil", "Egrave", "Eacute",
    "Ecirc", "Euml", "Igrave", "Iacute", "Icirc", "Iuml", "ETH", "Ntilde", "Ograve", "Oacute",
    "Ocirc", "Otilde", "Ouml", "times", "Oslash", "Ugrave", "Uacute", "Ucirc", "Uuml", "Yacute",
    "THORN", "szlig", "agrave", "aacute", "acirc", "atilde", "auml", "aring", "aelig", "ccedil",
    "egrave", "eacute", "ecirc", "euml", "igrave", "iacute", "icirc", "iuml", "eth", "ntilde",
    "ograve", "oacute", "ocirc", "otilde", "ouml", "divide", "oslash", "ugrave", "uacute", "ucirc",
    "uuml", "yacute", "thorn", "yuml",
];

/// The code point of the named character reference `name`.
fn entity(name: &str) -> Option<u32> {
    if let Some(p) = LATIN1.iter().position(|&n| n == name) {
        return u32::try_from(p).ok().map(|p| 0xa0 + p);
    }
    Some(match name {
        "quot" => 0x22,
        "amp" => 0x26,
        "apos" => 0x27,
        "lt" => 0x3c,
        "gt" => 0x3e,
        "OElig" => 0x152,
        "oelig" => 0x153,
        "Scaron" => 0x160,
        "scaron" => 0x161,
        "Yuml" => 0x178,
        "fnof" => 0x192,
        "circ" => 0x2c6,
        "tilde" => 0x2dc,
        "ensp" => 0x2002,
        "emsp" => 0x2003,
        "thinsp" => 0x2009,
        "zwnj" => 0x200c,
        "zwj" => 0x200d,
        "lrm" => 0x200e,
        "rlm" => 0x200f,
        "ndash" => 0x2013,
        "mdash" => 0x2014,
        "lsquo" => 0x2018,
        "rsquo" => 0x2019,
        "sbquo" => 0x201a,
        "ldquo" => 0x201c,
        "rdquo" => 0x201d,
        "bdquo" => 0x201e,
        "dagger" => 0x2020,
        "Dagger" => 0x2021,
        "bull" => 0x2022,
        "hellip" => 0x2026,
        "permil" => 0x2030,
        "prime" => 0x2032,
        "Prime" => 0x2033,
        "lsaquo" => 0x2039,
        "rsaquo" => 0x203a,
        "euro" => 0x20ac,
        "trade" => 0x2122,
        "larr" => 0x2190,
        "uarr" => 0x2191,
        "rarr" => 0x2192,
        "darr" => 0x2193,
        "harr" => 0x2194,
        "minus" => 0x2212,
        _ => return None,
    })
}

/// `s` with its character references read: `&#…;` and `&#x…;` of a
/// character, and the names above; any other `&` as written.
fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let c = cps(text);
    let c = c.as_slice();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if ch_at(c, i) != '&' {
            out.push(ch_at(c, i));
            i += 1;
            continue;
        }
        let read = if c.get(i + 1) == Some(&'#') {
            let hex = c.get(i + 2).is_some_and(|&d| d.eq_ignore_ascii_case(&'x'));
            let start = i + if hex { 3 } else { 2 };
            let mut j = start;
            while j < c.len()
                && j - start < if hex { 6 } else { 7 }
                && (if hex {
                    ch_at(c, j).is_ascii_hexdigit()
                } else {
                    digit(ch_at(c, j))
                })
            {
                j += 1;
            }
            (j > start && c.get(j) == Some(&';'))
                .then(|| u32::from_str_radix(&s(c, start, j), if hex { 16 } else { 10 }))
                .and_then(Result::ok)
                .filter(|&v| v > 0)
                .and_then(char::from_u32)
                .map(|ch| (ch, j + 1))
        } else {
            let mut j = i + 1;
            while j < c.len() && j - i - 1 < 8 && alnum(ch_at(c, j)) {
                j += 1;
            }
            (j > i + 1 && c.get(j) == Some(&';'))
                .then(|| entity(&s(c, i + 1, j)))
                .flatten()
                .and_then(char::from_u32)
                .map(|ch| (ch, j + 1))
        };
        match read {
            Some((ch, next)) => {
                out.push(ch);
                i = next;
            }
            None => {
                out.push('&');
                i += 1;
            }
        }
    }
    out
}

/// The number an `<ol>`'s `start` says, up to nine digits; 1 for none.
fn start_of(value: Option<&str>) -> u64 {
    let s = collapse(value.unwrap_or(""));
    if s.is_empty() || s.len() > 9 || !s.chars().all(digit) {
        return 1;
    }
    s.parse().unwrap_or(1)
}

/// A start tag read: its name in lower case ASCII letters, its
/// attributes, the first of each name, whether it closes itself and the
/// index after it.
struct Tag {
    name: String,
    attributes: Vec<(String, String)>,
    self_closing: bool,
    end: usize,
}

impl Tag {
    fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The language a `class` of `language-…` or `lang-…` names; empty
    /// for none.
    fn language(&self) -> String {
        for name in collapse(self.get("class").unwrap_or("")).split(' ') {
            for prefix in ["language-", "lang-"] {
                if let Some(language) = name.strip_prefix(prefix)
                    && !language.is_empty()
                {
                    return language.to_string();
                }
            }
        }
        String::new()
    }
}

/// HTML read as a browser shows it, into the subset: the text of its
/// elements, a run of blanks one space; `<h1>` and `<h2>` its headings,
/// the others a line of text; `<b>`, `<strong>`, `<i>`, `<em>`, `<s>`,
/// `<del>` and `<code>` their formats; `<a href>` a link; lists, quotes,
/// `<pre>` and tables as the subset has them; an image its `alt` text.
/// A script, a style and a template go with their content, and so do
/// comments; nothing is fetched.
struct Html {
    c: Vec<char>,
    out: Out,
    title: String,
    line: Inline,
    cell: Option<Inline>,
    blank_due: bool,
    bolds: usize,
    italics: usize,
    strikes: usize,
    codes: usize,
    links: Vec<Option<String>>,
    heading: usize,
    /// The lists open, each its next number, `None` for a bulleted one.
    lists: Vec<Option<u64>>,
    item: Option<(usize, Option<u64>)>,
    quotes: usize,
    pre: usize,
    pre_text: String,
    language: String,
    table: usize,
    rows: Vec<Vec<Vec<Span>>>,
    row: Option<Vec<Vec<Span>>>,
}

impl Html {
    fn new(text: &str) -> Html {
        Html {
            c: cps(text),
            out: Out::default(),
            title: String::new(),
            line: Inline::default(),
            cell: None,
            blank_due: false,
            bolds: 0,
            italics: 0,
            strikes: 0,
            codes: 0,
            links: Vec::new(),
            heading: 0,
            lists: Vec::new(),
            item: None,
            quotes: 0,
            pre: 0,
            pre_text: String::new(),
            language: String::new(),
            table: 0,
            rows: Vec::new(),
            row: None,
        }
    }

    fn style(&self) -> u8 {
        (if self.bolds > 0 { BOLD } else { 0 })
            | (if self.italics > 0 { ITALIC } else { 0 })
            | (if self.strikes > 0 { STRIKE } else { 0 })
            | (if self.codes > 0 { CODE } else { 0 })
    }

    fn read(mut self) -> ImportedDocument {
        let n = self.c.len();
        let mut text = 0;
        let mut i = 0;
        while i < n {
            if ch_at(&self.c, i) != '<' || i + 1 >= n {
                i += 1;
                continue;
            }
            let next = ch_at(&self.c, i + 1);
            if at(&self.c, i, "<!--", false) {
                self.text_run(text, i);
                let c = &self.c;
                let k = find(i + 4, n.saturating_sub(2), |k| {
                    ch_at(c, k) == '-' && ch_at(c, k + 1) == '-' && ch_at(c, k + 2) == '>'
                });
                i = k.map_or(n, |k| k + 3);
                text = i;
            } else if next == '!'
                || next == '?'
                || (next == '/' && !self.c.get(i + 2).is_some_and(|&d| letter(d)))
            {
                self.text_run(text, i);
                let c = &self.c;
                i = find(i + 2, n, |k| ch_at(c, k) == '>').map_or(n, |k| k + 1);
                text = i;
            } else if next == '/' {
                self.text_run(text, i);
                let k = i
                    + 2
                    + tail(&self.c, i + 2)
                        .iter()
                        .take_while(|&&d| tag_char(d))
                        .count();
                let name = lower_str(&self.c, i + 2, k);
                self.close(&name);
                let c = &self.c;
                i = find(k, n, |k| ch_at(c, k) == '>').map_or(n, |k| k + 1);
                text = i;
            } else if letter(next) {
                self.text_run(text, i);
                let Some(tag) = self.tag(i) else {
                    i = n;
                    text = n;
                    continue;
                };
                i = tag.end;
                text = i;
                let name = tag.name.as_str();
                if HTML_DROPPED.contains(&name) || name == "title" || name == "textarea" {
                    let c = &self.c;
                    let closing = format!("/{name}");
                    let close = find(i, n, |k| ch_at(c, k) == '<' && at(c, k + 1, &closing, true));
                    let inner = s(c, i, close.unwrap_or(n));
                    if name == "title" {
                        if self.title.is_empty() {
                            self.title = collapse(&decode_entities(&inner));
                        }
                    } else if name == "textarea" {
                        self.text(&decode_entities(&inner));
                    }
                    let c = &self.c;
                    i = close
                        .and_then(|close| find(close, n, |k| ch_at(c, k) == '>'))
                        .map_or(n, |k| k + 1);
                    text = i;
                    continue;
                }
                self.open(&tag);
                if tag.self_closing {
                    self.close(&tag.name);
                }
            } else {
                i += 1;
            }
        }
        self.text_run(text, n);
        if self.pre > 0 {
            self.pre = 1;
            self.close("pre");
        }
        if self.table > 0 {
            self.table = 1;
            self.close("table");
        }
        self.flush();
        ImportedDocument {
            title: self.title,
            text: self.out.text(),
        }
    }

    /// The start tag at `i`; `None` when the text ends inside it.
    fn tag(&self, i: usize) -> Option<Tag> {
        let c = &self.c;
        let n = c.len();
        let mut j = i + 1;
        while j < n && tag_char(ch_at(c, j)) {
            j += 1;
        }
        let name = lower_str(c, i + 1, j);
        let mut attributes: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            while j < n && (html_space(ch_at(c, j)) || ch_at(c, j) == '/') {
                self_closing = ch_at(c, j) == '/';
                j += 1;
            }
            if j >= n {
                return None;
            }
            if ch_at(c, j) == '>' {
                return Some(Tag {
                    name,
                    attributes,
                    self_closing,
                    end: j + 1,
                });
            }
            self_closing = false;
            let from = j;
            while j < n
                && !html_space(ch_at(c, j))
                && ch_at(c, j) != '>'
                && ch_at(c, j) != '/'
                && (ch_at(c, j) != '=' || j == from)
            {
                j += 1;
            }
            let key = lower_str(c, from, j);
            while j < n && html_space(ch_at(c, j)) {
                j += 1;
            }
            let mut value = String::new();
            if j < n && ch_at(c, j) == '=' {
                j += 1;
                while j < n && html_space(ch_at(c, j)) {
                    j += 1;
                }
                if j < n && (ch_at(c, j) == '"' || ch_at(c, j) == '\'') {
                    let q = ch_at(c, j);
                    let k = find(j + 1, n, |k| ch_at(c, k) == q)?;
                    value = s(c, j + 1, k);
                    j = k + 1;
                } else {
                    let start = j;
                    while j < n && !html_space(ch_at(c, j)) && ch_at(c, j) != '>' {
                        j += 1;
                    }
                    value = s(c, start, j);
                }
            }
            if !attributes.iter().any(|(k, _)| *k == key) {
                attributes.push((key, decode_entities(&value)));
            }
        }
    }

    fn text_run(&mut self, from: usize, to: usize) {
        if to > from {
            let text = decode_entities(&s(&self.c, from, to));
            self.text(&text);
        }
    }

    /// Text as a browser shows it: a run of blanks one space, none at the
    /// start of a line; in `<pre>` as it is.
    fn text(&mut self, text: &str) {
        if self.pre > 0 {
            self.pre_text.push_str(text);
            return;
        }
        let style = self.style();
        let link = self.links.last().cloned().flatten();
        let into = match self.cell.as_mut() {
            Some(cell) => cell,
            None => &mut self.line,
        };
        for ch in text.chars() {
            if html_space(ch) {
                self.blank_due = true;
                continue;
            }
            if self.blank_due && !into.is_empty() {
                into.char(' ', style, link.as_deref());
            }
            self.blank_due = false;
            into.char(ch, style, link.as_deref());
        }
    }

    /// Ends the line, or in a table's cell puts a space.
    fn boundary(&mut self) {
        if self.cell.is_some() {
            self.blank_due = true;
        } else {
            self.flush();
        }
    }

    /// A blank line, but in a list or a table's cell.
    fn gap(&mut self) {
        if self.cell.is_none() && self.lists.is_empty() {
            self.out.blank();
        }
    }

    fn flush(&mut self) {
        let spans = self.line.take();
        self.blank_due = false;
        if spans.is_empty() {
            return;
        }
        if self.heading == 1 || self.heading == 2 {
            self.out.heading(self.heading, &spans);
        } else if let Some((level, number)) = self.item.take() {
            self.out.item(level, number, &spans);
        } else if self.quotes > 0 {
            self.out.quote(&spans);
        } else {
            self.out.paragraph(&spans);
        }
    }

    fn end_cell(&mut self) {
        if let Some(mut cell) = self.cell.take() {
            self.row.get_or_insert_with(Vec::new).push(cell.take());
            self.blank_due = false;
        }
    }

    fn end_row(&mut self) {
        if let Some(row) = self.row.take()
            && !row.is_empty()
        {
            self.rows.push(row);
        }
    }

    fn open(&mut self, tag: &Tag) {
        let name = tag.name.as_str();
        match name {
            "br" => {
                if self.pre > 0 {
                    self.pre_text.push('\n');
                } else {
                    self.boundary();
                }
            }
            "p" | "hr" => {
                self.boundary();
                self.gap();
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.boundary();
                self.gap();
                if self.cell.is_none() {
                    self.heading = match name {
                        "h1" => 1,
                        "h2" => 2,
                        _ => 3,
                    };
                }
            }
            "ul" | "ol" => {
                self.boundary();
                self.gap();
                self.lists
                    .push((name == "ol").then(|| start_of(tag.get("start"))));
            }
            "li" => {
                self.boundary();
                if self.cell.is_none() {
                    let next = self.lists.last().copied().flatten();
                    if let (Some(n), Some(last)) = (next, self.lists.last_mut()) {
                        *last = Some(n + 1);
                    }
                    self.item = Some((usize::from(self.lists.len() > 1), next));
                }
            }
            "blockquote" => {
                self.boundary();
                self.gap();
                self.quotes += 1;
            }
            "pre" => {
                self.boundary();
                if self.table == 0 {
                    if self.pre == 0 {
                        self.gap();
                        self.pre_text.clear();
                        self.language = tag.language();
                    }
                    self.pre += 1;
                }
            }
            "code" | "kbd" | "samp" | "tt" => {
                if self.pre > 0 {
                    if self.language.is_empty() {
                        self.language = tag.language();
                    }
                } else {
                    self.codes += 1;
                }
            }
            "b" | "strong" => self.bolds += 1,
            "i" | "em" => self.italics += 1,
            "s" | "strike" | "del" => self.strikes += 1,
            "a" => self
                .links
                .push(target(&collapse(tag.get("href").unwrap_or("")))),
            "img" => {
                let alt = tag.get("alt").unwrap_or("");
                if !alt.is_empty() {
                    let alt = format!(" {alt} ");
                    self.text(&alt);
                }
            }
            "table" => {
                if self.table == 0 {
                    self.boundary();
                    self.gap();
                    self.rows.clear();
                    self.row = None;
                } else {
                    self.boundary();
                }
                self.table += 1;
            }
            "tr" => {
                if self.table == 1 {
                    self.end_cell();
                    self.end_row();
                    self.row = Some(Vec::new());
                } else {
                    self.boundary();
                }
            }
            "td" | "th" => {
                if self.table == 1 {
                    self.end_cell();
                    self.row.get_or_insert_with(Vec::new);
                    self.cell = Some(Inline::default());
                    self.blank_due = false;
                } else {
                    self.boundary();
                }
            }
            _ => {
                if HTML_BLOCKS.contains(&name) {
                    self.boundary();
                }
            }
        }
    }

    fn close(&mut self, name: &str) {
        match name {
            "p" => {
                self.boundary();
                self.gap();
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.boundary();
                if self.cell.is_none() {
                    self.heading = 0;
                }
                self.gap();
            }
            "ul" | "ol" => {
                self.boundary();
                self.item = None;
                self.lists.pop();
                self.gap();
            }
            "li" => {
                self.boundary();
                self.item = None;
            }
            "blockquote" => {
                self.boundary();
                self.quotes = self.quotes.saturating_sub(1);
                self.gap();
            }
            "pre" => {
                if self.table > 0 || self.pre == 0 {
                    self.boundary();
                } else {
                    self.pre -= 1;
                    if self.pre == 0 {
                        let text = std::mem::take(&mut self.pre_text);
                        let text = text.strip_prefix('\n').unwrap_or(&text);
                        let text = text.strip_suffix('\n').unwrap_or(text);
                        let lines: Vec<&str> = text.split('\n').collect();
                        let lines = lines.as_slice();
                        let language = self.language.clone();
                        self.out.code(&language, lines);
                        self.gap();
                    }
                }
            }
            "code" | "kbd" | "samp" | "tt" => {
                if self.pre == 0 {
                    self.codes = self.codes.saturating_sub(1);
                }
            }
            "b" | "strong" => self.bolds = self.bolds.saturating_sub(1),
            "i" | "em" => self.italics = self.italics.saturating_sub(1),
            "s" | "strike" | "del" => self.strikes = self.strikes.saturating_sub(1),
            "a" => {
                self.links.pop();
            }
            "table" => {
                if self.table > 0 {
                    self.table -= 1;
                    if self.table == 0 {
                        self.end_cell();
                        self.end_row();
                        self.flush();
                        let rows = std::mem::take(&mut self.rows);
                        self.out.table(&rows);
                    }
                }
            }
            "tr" => {
                if self.table == 1 {
                    self.end_cell();
                    self.end_row();
                }
            }
            "td" | "th" => {
                if self.table == 1 {
                    self.end_cell();
                }
            }
            _ => {
                if HTML_BLOCKS.contains(&name) {
                    self.boundary();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
