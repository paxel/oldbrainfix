//! The text of an attached file (ADR 0047), as the Dart core's
//! `file_text.dart` reads it, character for character: what a file is by
//! its name and media type, who reads its text (Q645), and the text of
//! `.docx`, `.xlsx`, `.odt` and `.ods` files (zip and XML, text only)
//! and of plain text and Markdown. A PDF is read by [`crate::pdf`] on
//! the desktop and by the phone's own platform; their page texts, a
//! scanned page's read by the OCR, are joined by [`join_pages`]. Any
//! other file is kept without text.
//!
//! Nothing here fails: a file that does not read as its kind says gives
//! an empty text, which records that nothing was found.

use std::io::Read;

use crate::transcript_prompts::dart_trim;

/// What an attached file is, as far as its text goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Pdf,
    /// A Word document, `.docx`.
    Docx,
    /// An Excel workbook, `.xlsx`.
    Xlsx,
    /// An OpenDocument text, `.odt`.
    Odt,
    /// An OpenDocument spreadsheet, `.ods`.
    Ods,
    /// Plain text or Markdown.
    Text,
    /// Any other file: kept without text.
    Other,
}

impl FileKind {
    /// Its name, as the Dart core's enum has it.
    pub fn name(self) -> &'static str {
        match self {
            FileKind::Pdf => "pdf",
            FileKind::Docx => "docx",
            FileKind::Xlsx => "xlsx",
            FileKind::Odt => "odt",
            FileKind::Ods => "ods",
            FileKind::Text => "text",
            FileKind::Other => "other",
        }
    }

    /// Whether a text is read out of a file of this kind.
    pub fn has_text(self) -> bool {
        self != FileKind::Other
    }
}

/// The most pages of a PDF the phone reads itself (Q645); a longer one
/// is left to a computer.
pub const PHONE_PAGES: u64 = 10;

/// The largest file of another kind with text the phone reads itself.
pub const PHONE_BYTES: u64 = 10 << 20;

/// The most an Attachment Text holds, in characters; the rest of a
/// longer one is cut off.
pub const MAX_TEXT_CHARS: usize = 1_000_000;

/// The most bytes a part of a zip is read to.
const MAX_PART_BYTES: u64 = 32 << 20;

/// The most sheets of a workbook read.
const MAX_SHEETS: usize = 256;

/// The most times a repeated cell or row of an OpenDocument spreadsheet
/// is written out.
const MAX_REPEAT: usize = 100;

/// The kind of the file `name` with the media type `media_type`: by the
/// ending of its name in any case of ASCII letters, else by its media
/// type.
pub fn kind_of(name: &str, media_type: &str) -> FileKind {
    let ending = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ending.as_str() {
        "pdf" => return FileKind::Pdf,
        "docx" => return FileKind::Docx,
        "xlsx" => return FileKind::Xlsx,
        "odt" => return FileKind::Odt,
        "ods" => return FileKind::Ods,
        "txt" | "text" | "md" | "markdown" | "csv" | "tsv" => return FileKind::Text,
        _ => {}
    }
    match media_type {
        "application/pdf" => FileKind::Pdf,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => FileKind::Docx,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => FileKind::Xlsx,
        "application/vnd.oasis.opendocument.text" => FileKind::Odt,
        "application/vnd.oasis.opendocument.spreadsheet" => FileKind::Ods,
        "text/plain" | "text/markdown" | "text/csv" | "text/tab-separated-values" => FileKind::Text,
        _ => FileKind::Other,
    }
}

/// The media type of the file `name` by the ending of its name, for a
/// file no app named one for; `application/octet-stream` when not known.
pub fn media_type_of(name: &str) -> &'static str {
    let ending = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ending.as_str() {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "txt" | "text" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "csv" => "text/csv",
        "tsv" => "text/tab-separated-values",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// Whether only a computer reads the text of a file of `kind`, `size`
/// bytes and `pages` pages (Q645): a PDF of more than [`PHONE_PAGES`]
/// pages or whose pages were not counted, any other file with text of
/// more than [`PHONE_BYTES`]. The attaching Device writes it down, so
/// every phone leaves the file alone until a computer has read it.
pub fn computer_reads(kind: FileKind, size: u64, pages: Option<u64>) -> bool {
    match kind {
        FileKind::Other => false,
        FileKind::Pdf => pages.is_none_or(|p| p > PHONE_PAGES),
        _ => size > PHONE_BYTES,
    }
}

/// What reading a file gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileReading {
    /// Its text, ready to keep; empty when none was found.
    Text(String),
    /// A PDF: read by [`crate::pdf::read`], its scanned pages by the OCR,
    /// and the page texts joined by [`join_pages`].
    Pdf,
    /// No text is read out of a file of this kind.
    Nothing,
}

/// The text of the file `bytes` of `kind`.
pub fn read(kind: FileKind, bytes: &[u8]) -> FileReading {
    let text = match kind {
        FileKind::Pdf => return FileReading::Pdf,
        FileKind::Other => return FileReading::Nothing,
        FileKind::Text => crate::imports::decode(bytes).unwrap_or_default(),
        FileKind::Docx => zipped(bytes, docx).unwrap_or_default(),
        FileKind::Xlsx => zipped(bytes, xlsx).unwrap_or_default(),
        FileKind::Odt => zipped(bytes, odt).unwrap_or_default(),
        FileKind::Ods => zipped(bytes, ods).unwrap_or_default(),
    };
    FileReading::Text(tidy(&text))
}

/// The texts of a PDF's `pages`, in their order, as one Attachment
/// Text: a blank line between them, blank pages left out.
pub fn join_pages(pages: &[String]) -> String {
    let kept: Vec<String> = pages
        .iter()
        .map(|p| tidy(p))
        .filter(|p| !p.is_empty())
        .collect();
    tidy(&kept.join("\n\n"))
}

/// `text` as an Attachment Text keeps it: line breaks as `\n`, no
/// control characters but tabs and line breaks, no spaces or tabs at
/// the ends of lines, at most one blank line in a row, no blank lines
/// or blanks at its ends, and at most [`MAX_TEXT_CHARS`] characters.
pub fn tidy(text: &str) -> String {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    let mut breaks = 0;
    for line in unified.split('\n') {
        let line: String = line
            .chars()
            .filter(|&c| c == '\t' || !is_control(c))
            .collect();
        let line = line.trim_end_matches([' ', '\t']);
        if line.is_empty() {
            breaks += 1;
            continue;
        }
        if !out.is_empty() {
            out.push_str(if breaks > 1 { "\n\n" } else { "\n" });
        }
        breaks = 1;
        out.push_str(line);
    }
    let trimmed = out.trim_start_matches([' ', '\t']);
    match trimmed.char_indices().nth(MAX_TEXT_CHARS) {
        Some((cut, _)) => trimmed
            .get(..cut)
            .unwrap_or(trimmed)
            .trim_end_matches([' ', '\t', '\n'])
            .to_string(),
        None => trimmed.to_string(),
    }
}

/// A control character: C0, DEL and C1.
fn is_control(c: char) -> bool {
    c < ' ' || ('\u{7f}'..='\u{9f}').contains(&c)
}

/// The text `read` makes of the zip `bytes`; `None` for a zip that does
/// not open.
fn zipped(bytes: &[u8], read: fn(&mut Parts) -> String) -> Option<String> {
    let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    Some(read(&mut Parts { archive }))
}

/// The parts of a zip, read by name.
struct Parts<'b> {
    archive: zip::ZipArchive<std::io::Cursor<&'b [u8]>>,
}

impl Parts<'_> {
    /// The part `name` as UTF-8 text; `None` when it is not there, too
    /// large or not UTF-8.
    fn text(&mut self, name: &str) -> Option<String> {
        let part = self.archive.by_name(name).ok()?;
        if part.size() > MAX_PART_BYTES {
            return None;
        }
        let mut bytes = Vec::new();
        part.take(MAX_PART_BYTES).read_to_end(&mut bytes).ok()?;
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        String::from_utf8(bytes.to_vec()).ok()
    }
}

/// The text of a Word document: each paragraph on a line, tabs and line
/// breaks as written, deleted text and field codes left out.
fn docx(parts: &mut Parts) -> String {
    let Some(xml) = parts.text("word/document.xml") else {
        return String::new();
    };
    let mut out = String::new();
    let mut in_text = 0usize;
    for event in Xml::new(&xml) {
        match event {
            Event::Start(tag) => match local(tag.name) {
                "t" if !tag.empty => in_text += 1,
                "tab" if tag.empty => out.push('\t'),
                "br" | "cr" | "p" if tag.empty => out.push('\n'),
                _ => {}
            },
            Event::End(name) => match local(name) {
                "t" => in_text = in_text.saturating_sub(1),
                "p" => out.push('\n'),
                _ => {}
            },
            Event::Text(text) if in_text > 0 => out.push_str(&text),
            Event::Text(_) => {}
        }
    }
    out
}

/// The text of an Excel workbook: each sheet under its name, in the
/// workbook's order, each row on a line with its cells' values between
/// tabs, a blank line between sheets.
fn xlsx(parts: &mut Parts) -> String {
    let shared = parts
        .text("xl/sharedStrings.xml")
        .map(|xml| shared_strings(&xml))
        .unwrap_or_default();
    let targets = parts
        .text("xl/_rels/workbook.xml.rels")
        .map(|xml| relationships(&xml))
        .unwrap_or_default();
    let Some(workbook) = parts.text("xl/workbook.xml") else {
        return String::new();
    };
    let mut sheets = Vec::new();
    for event in Xml::new(&workbook) {
        if let Event::Start(tag) = event
            && local(tag.name) == "sheet"
        {
            let name = tag.attribute("name").unwrap_or_default();
            let id = tag.attribute("id").unwrap_or_default();
            if let Some((_, target)) = targets.iter().find(|(i, _)| *i == id) {
                sheets.push((name, part_path(target)));
            }
        }
    }
    let mut out = Vec::new();
    for (name, path) in sheets.into_iter().take(MAX_SHEETS) {
        let Some(xml) = parts.text(&path) else {
            continue;
        };
        let rows = sheet_rows(&xml, &shared);
        if !rows.is_empty() {
            out.push(format!("{name}\n{}", rows.join("\n")));
        }
    }
    out.join("\n\n")
}

/// The path in the zip of the workbook's relationship target `target`.
fn part_path(target: &str) -> String {
    match target.strip_prefix('/') {
        Some(absolute) => absolute.to_string(),
        None => format!("xl/{}", target.trim_start_matches("./")),
    }
}

/// The shared strings of a workbook, in their order: each one's text
/// runs, its phonetic guides left out.
fn shared_strings(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let (mut in_text, mut in_guide) = (0usize, 0usize);
    for event in Xml::new(xml) {
        match event {
            Event::Start(tag) => match local(tag.name) {
                "si" if tag.empty => out.push(String::new()),
                "si" => current = Some(String::new()),
                "t" if !tag.empty => in_text += 1,
                "rPh" if !tag.empty => in_guide += 1,
                _ => {}
            },
            Event::End(name) => match local(name) {
                "si" => out.push(current.take().unwrap_or_default()),
                "t" => in_text = in_text.saturating_sub(1),
                "rPh" => in_guide = in_guide.saturating_sub(1),
                _ => {}
            },
            Event::Text(text) => {
                if in_text > 0
                    && in_guide == 0
                    && let Some(current) = current.as_mut()
                {
                    current.push_str(&text);
                }
            }
        }
    }
    out
}

/// The relationships of a workbook: each id with its target.
fn relationships(xml: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for event in Xml::new(xml) {
        if let Event::Start(tag) = event
            && local(tag.name) == "Relationship"
            && let (Some(id), Some(target)) = (tag.attribute("Id"), tag.attribute("Target"))
        {
            out.push((id, target));
        }
    }
    out
}

/// The rows of a worksheet with a value, each its cells' values between
/// tabs, the empty ones at the end left out.
fn sheet_rows(xml: &str, shared: &[String]) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell: Option<(String, String)> = None;
    let (mut in_value, mut in_text) = (0usize, 0usize);
    let mut inline = String::new();
    for event in Xml::new(xml) {
        match event {
            Event::Start(tag) => match local(tag.name) {
                "row" if tag.empty => {}
                "row" => row.clear(),
                "c" if tag.empty => row.push(String::new()),
                "c" => {
                    cell = Some((tag.attribute("t").unwrap_or_default(), String::new()));
                    inline.clear();
                }
                "v" if !tag.empty => in_value += 1,
                "t" if !tag.empty => in_text += 1,
                _ => {}
            },
            Event::End(name) => match local(name) {
                "row" => {
                    while row.last().is_some_and(String::is_empty) {
                        row.pop();
                    }
                    if !row.is_empty() {
                        rows.push(row.join("\t"));
                    }
                    row.clear();
                }
                "c" => {
                    let (kind, value) = cell.take().unwrap_or_default();
                    row.push(cell_value(&kind, &value, &inline, shared));
                }
                "v" => in_value = in_value.saturating_sub(1),
                "t" => in_text = in_text.saturating_sub(1),
                _ => {}
            },
            Event::Text(text) => {
                if let Some((_, value)) = cell.as_mut() {
                    if in_value > 0 {
                        value.push_str(&text);
                    } else if in_text > 0 {
                        inline.push_str(&text);
                    }
                }
            }
        }
    }
    rows
}

/// What a cell of the type `kind` with `value` and its inline text
/// `inline` shows: a shared string by its index, a truth value as
/// TRUE or FALSE, anything else as written. Tabs and line breaks in it
/// become spaces, so it stays in its row.
fn cell_value(kind: &str, value: &str, inline: &str, shared: &[String]) -> String {
    let shown = match kind {
        "s" => whole(value)
            .and_then(|i| shared.get(i))
            .cloned()
            .unwrap_or_default(),
        "inlineStr" => inline.to_string(),
        "b" => match dart_trim(value) {
            "1" => "TRUE".to_string(),
            "0" => "FALSE".to_string(),
            other => other.to_string(),
        },
        _ => value.to_string(),
    };
    one_line(&shown)
}

/// `text` with its tabs and line breaks as spaces, its ends trimmed.
fn one_line(text: &str) -> String {
    let spaced: String = text
        .chars()
        .map(|c| {
            if matches!(c, '\t' | '\n' | '\r') {
                ' '
            } else {
                c
            }
        })
        .collect();
    spaced.trim_matches(' ').to_string()
}

/// The text of an OpenDocument text: each paragraph and heading on a
/// line, its spaces, tabs and line breaks as written, deleted text left
/// out.
fn odt(parts: &mut Parts) -> String {
    let Some(xml) = parts.text("content.xml") else {
        return String::new();
    };
    let mut out = String::new();
    let (mut in_body, mut in_paragraph, mut in_deleted) = (0usize, 0usize, 0usize);
    for event in Xml::new(&xml) {
        match event {
            Event::Start(tag) => {
                let name = local(tag.name);
                if tag.empty {
                    if in_body > 0 && in_paragraph > 0 && in_deleted == 0 {
                        inline_mark(&tag, &mut out);
                    }
                    if matches!(name, "p" | "h") && in_body > 0 && in_deleted == 0 {
                        out.push('\n');
                    }
                    continue;
                }
                match name {
                    "body" => in_body += 1,
                    "p" | "h" => in_paragraph += 1,
                    "tracked-changes" => in_deleted += 1,
                    _ => {}
                }
            }
            Event::End(name) => match local(name) {
                "body" => in_body = in_body.saturating_sub(1),
                "p" | "h" => {
                    in_paragraph = in_paragraph.saturating_sub(1);
                    if in_body > 0 && in_deleted == 0 {
                        out.push('\n');
                    }
                }
                "tracked-changes" => in_deleted = in_deleted.saturating_sub(1),
                _ => {}
            },
            Event::Text(text) => {
                if in_body > 0 && in_paragraph > 0 && in_deleted == 0 {
                    out.push_str(&text);
                }
            }
        }
    }
    out
}

/// Writes what the empty OpenDocument element `tag` inside a paragraph
/// stands for: spaces, a tab or a line break.
fn inline_mark(tag: &Tag, out: &mut String) {
    match local(tag.name) {
        "s" => {
            let count = tag
                .attribute("c")
                .and_then(|c| whole(&c))
                .unwrap_or(1)
                .min(MAX_REPEAT);
            out.push_str(&" ".repeat(count));
        }
        "tab" => out.push('\t'),
        "line-break" => out.push('\n'),
        _ => {}
    }
}

/// The text of an OpenDocument spreadsheet: each sheet under its name,
/// each row with a value on a line with its cells' texts between tabs,
/// a blank line between sheets.
fn ods(parts: &mut Parts) -> String {
    let Some(xml) = parts.text("content.xml") else {
        return String::new();
    };
    let mut sheets: Vec<String> = Vec::new();
    let mut sheet: Option<(String, Vec<String>)> = None;
    let mut row: Option<(usize, Vec<String>)> = None;
    let mut cell: Option<(usize, String)> = None;
    let mut in_paragraph = 0usize;
    let mut paragraphs = 0usize;
    for event in Xml::new(&xml) {
        match event {
            Event::Start(tag) => {
                let name = local(tag.name);
                match name {
                    "table" if !tag.empty => {
                        sheet = Some((tag.attribute("name").unwrap_or_default(), Vec::new()));
                    }
                    "table-row" => {
                        let repeat = repeated(&tag, "number-rows-repeated");
                        row = Some((repeat, Vec::new()));
                        if tag.empty {
                            end_row(&mut row, &mut sheet);
                        }
                    }
                    "table-cell" | "covered-table-cell" => {
                        let repeat = repeated(&tag, "number-columns-repeated");
                        cell = Some((repeat, String::new()));
                        paragraphs = 0;
                        if tag.empty {
                            end_cell(&mut cell, &mut row);
                        }
                    }
                    "p" | "h" if cell.is_some() => {
                        if paragraphs > 0
                            && let Some((_, text)) = cell.as_mut()
                        {
                            text.push(' ');
                        }
                        paragraphs += 1;
                        if !tag.empty {
                            in_paragraph += 1;
                        }
                    }
                    _ if in_paragraph > 0 && tag.empty => {
                        if let Some((_, text)) = cell.as_mut() {
                            inline_mark(&tag, text);
                        }
                    }
                    _ => {}
                }
            }
            Event::End(name) => match local(name) {
                "table" => {
                    if let Some((name, rows)) = sheet.take()
                        && !rows.is_empty()
                    {
                        sheets.push(format!("{name}\n{}", rows.join("\n")));
                    }
                }
                "table-row" => end_row(&mut row, &mut sheet),
                "table-cell" | "covered-table-cell" => end_cell(&mut cell, &mut row),
                "p" | "h" => in_paragraph = in_paragraph.saturating_sub(1),
                _ => {}
            },
            Event::Text(text) => {
                if in_paragraph > 0
                    && let Some((_, cell)) = cell.as_mut()
                {
                    cell.push_str(&text);
                }
            }
        }
    }
    sheets.join("\n\n")
}

/// How often `tag` is repeated by its attribute `attribute`, at least
/// once and at most [`MAX_REPEAT`] times.
fn repeated(tag: &Tag, attribute: &str) -> usize {
    tag.attribute(attribute)
        .and_then(|n| whole(&n))
        .unwrap_or(1)
        .clamp(1, MAX_REPEAT)
}

/// Puts the cell held into its row, as often as it is repeated.
fn end_cell(cell: &mut Option<(usize, String)>, row: &mut Option<(usize, Vec<String>)>) {
    if let Some((repeat, text)) = cell.take()
        && let Some((_, cells)) = row.as_mut()
    {
        let text = one_line(&text);
        for _ in 0..repeat {
            cells.push(text.clone());
        }
    }
}

/// Puts the row held into its sheet, as often as it is repeated, unless
/// it holds no value.
fn end_row(row: &mut Option<(usize, Vec<String>)>, sheet: &mut Option<(String, Vec<String>)>) {
    if let Some((repeat, mut cells)) = row.take() {
        while cells.last().is_some_and(String::is_empty) {
            cells.pop();
        }
        if let Some((_, rows)) = sheet.as_mut()
            && !cells.is_empty()
        {
            let line = cells.join("\t");
            for _ in 0..repeat {
                rows.push(line.clone());
            }
        }
    }
}

/// The whole number `text` writes in decimal digits alone, blanks around
/// them trimmed, at most nine of them.
fn whole(text: &str) -> Option<usize> {
    let digits = dart_trim(text);
    if digits.is_empty() || digits.len() > 9 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The name of an element or attribute without its namespace prefix.
fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

/// A start tag of an XML element.
struct Tag<'x> {
    name: &'x str,
    /// The raw attributes, between the name and the end of the tag.
    attributes: &'x str,
    /// Whether it closes itself (`<a/>`); no end follows.
    empty: bool,
}

impl Tag<'_> {
    /// The value of the attribute `wanted`, by its name without its
    /// prefix, its references resolved.
    fn attribute(&self, wanted: &str) -> Option<String> {
        let mut rest = self.attributes;
        loop {
            rest = rest.trim_start_matches(is_xml_space);
            let equals = rest.find('=')?;
            let name = rest.get(..equals)?.trim_end_matches(is_xml_space);
            rest = rest.get(equals + 1..)?.trim_start_matches(is_xml_space);
            let quote = rest.chars().next().filter(|q| *q == '"' || *q == '\'')?;
            let value_end = rest.get(1..)?.find(quote)? + 1;
            let value = rest.get(1..value_end)?;
            if local(name) == wanted {
                return Some(unescape(value));
            }
            rest = rest.get(value_end + 1..)?;
        }
    }
}

/// What an XML text gives, in its order.
enum Event<'x> {
    Start(Tag<'x>),
    End(&'x str),
    Text(String),
}

fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// A small reader of XML, the same in both cores: start tags, end tags
/// and text with its references resolved; comments, processing
/// instructions and declarations skipped, CDATA as text. It stops at
/// the first tag that does not end.
struct Xml<'x> {
    rest: &'x str,
}

impl<'x> Xml<'x> {
    fn new(xml: &'x str) -> Xml<'x> {
        Xml { rest: xml }
    }
}

impl<'x> Iterator for Xml<'x> {
    type Item = Event<'x>;

    fn next(&mut self) -> Option<Event<'x>> {
        loop {
            if self.rest.is_empty() {
                return None;
            }
            if !self.rest.starts_with('<') {
                let end = self.rest.find('<').unwrap_or(self.rest.len());
                let (text, rest) = self.rest.split_at(end);
                self.rest = rest;
                return Some(Event::Text(unescape(text)));
            }
            if let Some(rest) = self.rest.strip_prefix("<!--") {
                let end = rest.find("-->")?;
                self.rest = rest.get(end + 3..)?;
                continue;
            }
            if let Some(rest) = self.rest.strip_prefix("<![CDATA[") {
                let end = rest.find("]]>")?;
                let text = rest.get(..end)?;
                self.rest = rest.get(end + 3..)?;
                return Some(Event::Text(text.to_string()));
            }
            if let Some(rest) = self.rest.strip_prefix("<?") {
                let end = rest.find("?>")?;
                self.rest = rest.get(end + 2..)?;
                continue;
            }
            if let Some(rest) = self.rest.strip_prefix("<!") {
                let end = rest.find('>')?;
                self.rest = rest.get(end + 1..)?;
                continue;
            }
            let end = tag_end(self.rest)?;
            let inner = self.rest.get(1..end)?;
            self.rest = self.rest.get(end + 1..)?;
            if let Some(name) = inner.strip_prefix('/') {
                return Some(Event::End(name.trim_matches(is_xml_space)));
            }
            let (inner, empty) = match inner.strip_suffix('/') {
                Some(inner) => (inner, true),
                None => (inner, false),
            };
            let name_end = inner.find(is_xml_space).unwrap_or(inner.len());
            let (name, attributes) = inner.split_at(name_end);
            return Some(Event::Start(Tag {
                name,
                attributes,
                empty,
            }));
        }
    }
}

/// Where the tag at the start of `xml` ends: its `>` outside quotes.
fn tag_end(xml: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (i, c) in xml.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

/// `text` with its character and entity references resolved; one that
/// is not known, or names no character, stays as written.
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(rest.get(..at).unwrap_or_default());
        rest = rest.get(at..).unwrap_or_default();
        let resolved = rest.find(';').and_then(|end| {
            let name = rest.get(1..end)?;
            Some((reference(name)?, end))
        });
        match resolved {
            Some((c, end)) => {
                out.push(c);
                rest = rest.get(end + 1..).unwrap_or_default();
            }
            None => {
                out.push('&');
                rest = rest.get(1..).unwrap_or_default();
            }
        }
    }
    out.push_str(rest);
    out
}

/// The character the reference `name` (between `&` and `;`) stands for.
fn reference(name: &str) -> Option<char> {
    let code = match name {
        "lt" => return Some('<'),
        "gt" => return Some('>'),
        "amp" => return Some('&'),
        "quot" => return Some('"'),
        "apos" => return Some('\''),
        _ => {
            let number = name.strip_prefix('#')?;
            let (digits, radix) = match number.strip_prefix(['x', 'X']) {
                Some(hex) => (hex, 16),
                None => (number, 10),
            };
            if digits.is_empty() || digits.len() > 8 || !digits.chars().all(|c| c.is_digit(radix)) {
                return None;
            }
            u32::from_str_radix(digits, radix).ok()?
        }
    };
    if code == 0 {
        return None;
    }
    char::from_u32(code)
}

#[cfg(test)]
mod tests;
