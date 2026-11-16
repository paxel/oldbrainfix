//! Imports (ADR 0036), as the Dart core's `imports.dart`: Notes made
//! from the files of a folder, or from a Google Keep export, into a new
//! Area named after it, each dated by its file or its note, none of them
//! Nudged. The apps pick the folder or the export and read its files;
//! the core does the rest.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SubsecRound, Utc};
use serde_json::Value;

use crate::checklists::{
    Checklists, NewLine, keys as checklist_keys, line_text, order, write_line,
};
use crate::entry::dart_iso;
use crate::error::{Error, Result};
use crate::notes::keys;
use crate::places::Places;
use crate::search::lower_case;
use crate::shopping_lists::ListOnAgenda;
use crate::store::Store;
use crate::text_boxes;
use crate::text_format::{self, KNOWN, MARKDOWN, PLAIN, StoredText, VERSION};
use crate::text_tags;
use crate::transcript_prompts::dart_trim;

/// The largest file read; the apps need not read a larger one.
pub const MAX_FILE_BYTES: usize = 1 << 20;

/// The largest text kept, as stored with its format: a Backup reads an
/// entry of up to 1 MiB, its value escaped once more inside it.
pub const MAX_TEXT_BYTES: usize = 256 << 10;

/// The most text one Import keeps, as stored; the newest files are kept
/// first.
pub const MAX_TOTAL_BYTES: usize = 32 << 20;

/// The largest photo kept; the apps need not read a larger one.
pub const MAX_PHOTO_BYTES: usize = 16 << 20;

/// One file handed to an Import: its name, when it was last changed
/// (`None` when the app cannot tell) and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFile {
    /// The file's name; a path's folders before the last `/` or `\` are
    /// left out.
    pub name: String,
    pub modified: Option<DateTime<Utc>>,
    pub bytes: Vec<u8>,
}

/// One Line of a Checklist an Import makes: its `text`, and when it was
/// `checked`, `None` while it is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedLine {
    pub text: String,
    pub checked: Option<DateTime<Utc>>,
}

/// One Note an Import makes: its `text` in its `format`, when it was
/// `captured`, and the `name` of what it came from, which orders Notes
/// of the same date. With `lines` it is a Checklist of them, an empty
/// one too; it holds the `photos`, as their bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedNote {
    pub name: String,
    pub text: String,
    pub format: String,
    pub captured: DateTime<Utc>,
    pub lines: Option<Vec<ImportedLine>>,
    pub photos: Vec<Vec<u8>>,
}

/// What an Import made, for the apps to announce.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportSummary {
    /// The new Area the Notes landed in; `None` when there was nothing
    /// to import and no Area was made.
    pub area_id: Option<String>,
    /// How many Notes were made.
    pub notes: usize,
    /// Files skipped as no text: another kind of file, a hidden one, one
    /// holding a zero byte, or one with nothing in it or its name.
    pub not_text: usize,
    /// Files skipped as too large ([`MAX_FILE_BYTES`],
    /// [`MAX_TEXT_BYTES`], [`MAX_TOTAL_BYTES`], [`MAX_PHOTO_BYTES`]).
    pub too_large: usize,
    /// How many of the `notes` are Checklists.
    pub checklists: usize,
    /// How many photos the `notes` hold.
    pub photos: usize,
    /// Notes skipped as in the trash of the app they came from.
    pub trashed: usize,
    /// Files a note names that are not among those handed over.
    pub missing: usize,
    /// Files a note names that are no photo, such as a voice recording.
    pub not_photo: usize,
}

impl ImportSummary {
    /// How many files and notes were skipped.
    pub fn skipped(&self) -> usize {
        self.not_text + self.too_large + self.trashed + self.missing + self.not_photo
    }
}

/// Imports into one store.
pub struct Imports<'a> {
    store: &'a Store,
}

impl<'a> Imports<'a> {
    pub fn new(store: &'a Store) -> Imports<'a> {
        Imports { store }
    }

    /// Makes one Note of each Markdown (`.md`, `.markdown`) and text
    /// (`.txt`) file of `files` in a new Area named `area_name`, as
    /// [`Imports::notes`] writes them; any other file is skipped and
    /// counted. A Markdown file's Note is formatted, its Markdown as
    /// written; a text file's is plain. A Note is dated by its file's
    /// modification time, and by now when that is unknown or later.
    pub fn folder(&self, area_name: &str, files: &[ImportFile]) -> Result<ImportSummary> {
        let name = area(area_name)?;
        let now = self.store.now();
        let mut made = Vec::new();
        let (mut not_text, mut too_large) = (0, 0);
        for file in files {
            let base = base(&file.name);
            let Some((stem, format)) = format_of(base) else {
                not_text += 1;
                continue;
            };
            if file.bytes.len() > MAX_FILE_BYTES {
                too_large += 1;
                continue;
            }
            let Some(read) = decode(&file.bytes) else {
                not_text += 1;
                continue;
            };
            let text = titled(stem, &read, format);
            if text.is_empty() {
                not_text += 1;
                continue;
            }
            let at = match file.modified.map(|at| at.trunc_subsecs(6)) {
                Some(at) if at > DateTime::UNIX_EPOCH && at <= now => at,
                _ => now,
            };
            made.push(ImportedNote {
                name: base.to_string(),
                text,
                format: format.to_string(),
                captured: at,
                lines: None,
                photos: Vec::new(),
            });
        }
        self.notes(name, &made, not_text, too_large)
    }

    /// Writes `notes` into a new Area named `area_name`, oldest first,
    /// filed there so none of them is Unsorted or Nudged, each captured
    /// and dated when it says, with its photos and, for a Checklist, its
    /// Lines, checked when they say. An Area of that name already there gets a
    /// new one beside it, named with a number after it. A text larger
    /// than [`MAX_TEXT_BYTES`], and the oldest beyond
    /// [`MAX_TOTAL_BYTES`], are skipped as too large, counted on top of
    /// `not_text` and `too_large`; with nothing left, no Area is made.
    pub fn notes(
        &self,
        area_name: &str,
        notes: &[ImportedNote],
        not_text: usize,
        too_large: usize,
    ) -> Result<ImportSummary> {
        let skipped = ImportSummary {
            not_text,
            too_large,
            ..ImportSummary::default()
        };
        self.write(area_name, notes, &skipped)
    }

    /// `notes` with the files and notes `skipped` before them counted.
    fn write(
        &self,
        area_name: &str,
        notes: &[ImportedNote],
        skipped: &ImportSummary,
    ) -> Result<ImportSummary> {
        let name = area(area_name)?;
        if let Some(note) = notes.iter().find(|n| !KNOWN.contains(&n.format.as_str())) {
            return Err(Error::Invalid(format!(
                "not a format known here: {}",
                note.format
            )));
        }
        let mut order: Vec<(usize, &ImportedNote, DateTime<Utc>)> = notes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n, n.captured.trunc_subsecs(6)))
            .collect();
        order.sort_by(|a, b| (a.2, &a.1.name, a.0).cmp(&(b.2, &b.1.name, b.0)));
        let mut kept = Vec::new();
        let mut large = skipped.too_large;
        let mut total = 0;
        for (_, note, at) in order.iter().rev() {
            let mut size = StoredText::of(&note.format, &note.text, VERSION)
                .encode()
                .len();
            for line in note.lines.iter().flatten() {
                size += StoredText::plain(&line.text).encode().len();
            }
            if size > MAX_TEXT_BYTES || total + size > MAX_TOTAL_BYTES {
                large += 1;
                continue;
            }
            total += size;
            kept.push((*note, *at));
        }
        let summary = ImportSummary {
            area_id: None,
            notes: 0,
            too_large: large,
            checklists: 0,
            photos: 0,
            ..skipped.clone()
        };
        if kept.is_empty() {
            return Ok(summary);
        }
        kept.reverse();
        let s = self.store;
        // The bytes are kept before the log names them, each photo once.
        let mut hashes = Vec::new();
        for (note, _) in &kept {
            let mut held: Vec<String> = Vec::new();
            for bytes in &note.photos {
                let hash = s.put_photo(bytes)?;
                if !held.contains(&hash) {
                    held.push(hash);
                }
            }
            hashes.push(held);
        }
        let area_id = s.batch(|| {
            let area = Places::new(s).add_area(&self.unused(name)?)?;
            for ((note, at), held) in kept.iter().zip(&hashes) {
                let id = format!("note:{}", s.new_id()?);
                let date = Some(*at);
                s.append(&id, keys::TYPE, Some(keys::TYPE_NOTE), date)?;
                s.append(&id, keys::CAPTURED, Some(&dart_iso(*at)), date)?;
                let boxed = text_boxes::convert(&note.text, &note.format, || s.new_id())?;
                let text = StoredText::of(&note.format, &boxed, VERSION).encode();
                s.append(&id, keys::TEXT, Some(&text), date)?;
                s.append(&id, keys::PLACE, Some(&area.id), date)?;
                for hash in held {
                    s.append(&id, &keys::photo(hash), Some(keys::ADDED), date)?;
                }
                if let Some(lines) = &note.lines {
                    self.write_lines(&id, lines, *at)?;
                }
            }
            Ok(area.id)
        })?;
        Ok(ImportSummary {
            area_id: Some(area_id),
            notes: kept.len(),
            checklists: kept.iter().filter(|(n, _)| n.lines.is_some()).count(),
            photos: hashes.iter().map(Vec::len).sum(),
            ..summary
        })
    }

    /// Makes the new Note `note_id` a Checklist of `lines`, not spawned,
    /// as a Sent Item does, since it is filed already: each Line written
    /// at `at`, its check, when it has one, when it says but not before.
    /// Lines with no text are left out. It starts off today's Agenda for
    /// this person, so an Import of many lists does not fill it.
    fn write_lines(&self, note_id: &str, lines: &[ImportedLine], at: DateTime<Utc>) -> Result<()> {
        let s = self.store;
        let making = s.new_id()?;
        let kind = checklist_keys::kind_of(&making);
        s.append(note_id, checklist_keys::KIND, Some(&kind), Some(at))?;
        let mut key = String::new();
        for line in lines {
            if dart_trim(&line.text).is_empty() {
                continue;
            }
            key = order::after(&key);
            let text = line_text(&line.text)?;
            let new = NewLine {
                text: &text,
                order: &key,
                indent: false,
                checked: false,
                received: None,
                at: Some(at),
            };
            let id = write_line(s, note_id, &making, &new)?;
            if let Some(checked) = line.checked {
                let on = checked.trunc_subsecs(6).max(at);
                s.append(&id, checklist_keys::CHECKED, Some(&dart_iso(on)), Some(on))?;
            }
        }
        Checklists::new(s).set_on_agenda(note_id, ListOnAgenda::Off)
    }

    /// Makes one Note of each note of a Google Keep export in a new Area
    /// named `area_name`, as [`Imports::notes`] writes them (ADR 0036).
    /// `entries` are the files of its Takeout, a zip's or a folder's, by
    /// their paths: each note's `.json`, which is all that is read of it,
    /// and the photos they name. When any of them lies in a folder named
    /// `Keep`, only those do. A note in Keep's trash is skipped; an
    /// archived one is made as any other. Its title is its first line,
    /// its labels are tags in a last line, a list is a Checklist with its
    /// Lines checked as in Keep, its photos are its photos. It is
    /// captured when Keep says it was made, and its checks are dated when
    /// it was last changed.
    pub fn keep(&self, area_name: &str, entries: &[ImportFile]) -> Result<ImportSummary> {
        let name = area(area_name)?;
        let now = self.store.now();
        let in_keep = entries.iter().any(|e| in_keep_folder(&e.name));
        let taken: Vec<&ImportFile> = entries
            .iter()
            .filter(|e| !in_keep || in_keep_folder(&e.name))
            .collect();
        let mut by_name: HashMap<&str, &ImportFile> = HashMap::new();
        for e in &taken {
            by_name.entry(base(&e.name)).or_insert(*e);
        }
        let mut made = Vec::new();
        let mut skipped = ImportSummary::default();
        for entry in taken {
            let file_name = base(&entry.name);
            if file_name.starts_with('.') || extension(file_name) != "json" {
                continue;
            }
            if entry.bytes.len() > MAX_FILE_BYTES {
                skipped.too_large += 1;
                continue;
            }
            let Some(note) = keep_note(&entry.bytes) else {
                skipped.not_text += 1;
                continue;
            };
            if note.get("isTrashed").and_then(Value::as_bool) == Some(true) {
                skipped.trashed += 1;
                continue;
            }
            let created = usec(note.get("createdTimestampUsec"), now);
            let edited = usec(note.get("userEditedTimestampUsec"), now);
            let at = created.or(edited).unwrap_or(now);
            let checked_at = match edited {
                Some(edited) if edited >= at => edited,
                _ => at,
            };
            let body = titled(
                &string(note.get("title")),
                &string(note.get("textContent")),
                PLAIN,
            );
            let lines = match note.get("listContent") {
                Some(Value::Array(items)) => Some(
                    items
                        .iter()
                        .filter_map(|item| keep_line(item, checked_at))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            };
            let mut photos = Vec::new();
            if let Some(Value::Array(attachments)) = note.get("attachments") {
                for attachment in attachments {
                    let path = attachment
                        .get("filePath")
                        .and_then(Value::as_str)
                        .map_or("", base);
                    if !PHOTO_KINDS.contains(&extension(path).as_str()) {
                        if path.is_empty() {
                            skipped.missing += 1;
                        } else {
                            skipped.not_photo += 1;
                        }
                        continue;
                    }
                    let file = by_name
                        .get(path)
                        .or_else(|| other_jpeg(path).and_then(|other| by_name.get(other.as_str())));
                    match file {
                        None => skipped.missing += 1,
                        Some(f) if f.bytes.len() > MAX_PHOTO_BYTES => skipped.too_large += 1,
                        Some(f) => photos.push(f.bytes.clone()),
                    }
                }
            }
            if body.is_empty() && lines.as_ref().is_none_or(Vec::is_empty) && photos.is_empty() {
                skipped.not_text += 1;
                continue;
            }
            let tags = keep_tags(note.get("labels"), &body);
            let text = if tags.is_empty() {
                body
            } else if body.is_empty() {
                tags
            } else {
                format!("{body}\n{tags}")
            };
            made.push(ImportedNote {
                name: file_name.to_string(),
                text,
                format: PLAIN.to_string(),
                captured: at,
                lines,
                photos,
            });
        }
        self.write(name, &made, &skipped)
    }

    /// `name`, or with " (2)", " (3)" and so on after it when an Area is
    /// called so already.
    fn unused(&self, name: &str) -> Result<String> {
        let taken: Vec<String> = Places::new(self.store)
            .areas()?
            .into_iter()
            .map(|a| a.name)
            .collect();
        if !taken.iter().any(|t| t == name) {
            return Ok(name.to_string());
        }
        let mut n = 2;
        loop {
            let candidate = format!("{name} ({n})");
            if !taken.contains(&candidate) {
                return Ok(candidate);
            }
            n += 1;
        }
    }
}

/// The most bytes the import from Google Keep reads of the file at
/// `path`, an entry of a Takeout export: [`MAX_FILE_BYTES`] of a note's
/// `.json`, [`MAX_PHOTO_BYTES`] of a photo. `None` for any other file,
/// which it does not read; the apps need not hand it over. A file larger
/// than that may be handed over as that many bytes and one more, unread,
/// to be counted as too large.
pub fn keep_reads(path: &str) -> Option<usize> {
    let base = base(path);
    if base.starts_with('.') {
        return None;
    }
    let extension = extension(base);
    if extension == "json" {
        Some(MAX_FILE_BYTES)
    } else if PHOTO_KINDS.contains(&extension.as_str()) {
        Some(MAX_PHOTO_BYTES)
    } else {
        None
    }
}

/// The extensions of the photos a Keep note may hold, in lower case.
const PHOTO_KINDS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

/// Whether the file at `path` lies in a folder named `Keep`, in any case
/// of ASCII letters.
fn in_keep_folder(path: &str) -> bool {
    let folders: Vec<&str> = path.split(['/', '\\']).collect();
    folders
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|folder| folder.eq_ignore_ascii_case("keep"))
}

/// The extension of the file `base` in lower case ASCII letters, empty
/// for none.
fn extension(base: &str) -> String {
    base.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

/// `base` with `.jpg` for `.jpeg` and the other way round, as a Keep
/// export may name a photo by one and hold it by the other; `None` for
/// any other file.
fn other_jpeg(base: &str) -> Option<String> {
    let (stem, _) = base.rsplit_once('.')?;
    match extension(base).as_str() {
        "jpeg" => Some(format!("{stem}.jpg")),
        "jpg" => Some(format!("{stem}.jpeg")),
        _ => None,
    }
}

/// The note a Keep `.json` holds: an object with Keep's time of making
/// or of the last change as a whole number; `None` for anything else.
fn keep_note(bytes: &[u8]) -> Option<serde_json::Map<String, Value>> {
    // As the Dart core's decoder, which drops a byte order mark.
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let Ok(Value::Object(note)) = serde_json::from_slice::<Value>(bytes) else {
        return None;
    };
    let whole = |key: &str| note.get(key).and_then(Value::as_i64).is_some();
    (whole("createdTimestampUsec") || whole("userEditedTimestampUsec")).then_some(note)
}

/// The moment `value` says in microseconds since 1970, when it is a
/// whole number after 1970 and not later than `now`; else `None`.
fn usec(value: Option<&Value>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let micros = value?.as_i64()?;
    if micros <= 0 || micros > now.timestamp_micros() {
        return None;
    }
    DateTime::from_timestamp_micros(micros)
}

/// `value` with its line breaks as `\n` when it is text; else empty.
fn string(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(|text| text.replace("\r\n", "\n").replace('\r', "\n"))
        .unwrap_or_default()
}

/// The Line a Keep list's `item` is, checked at `checked_at` when Keep
/// says it is; `None` for one without text.
fn keep_line(item: &Value, checked_at: DateTime<Utc>) -> Option<ImportedLine> {
    let text = item.get("text")?.as_str()?;
    if dart_trim(text).is_empty() {
        return None;
    }
    let checked = item.get("isChecked").and_then(Value::as_bool) == Some(true);
    Some(ImportedLine {
        text: text.to_string(),
        checked: checked.then_some(checked_at),
    })
}

/// The tags of the Keep `labels` ([`text_tags::from_label`]) not in
/// `body` yet, each once whatever its case, in their order, apart by
/// spaces.
fn keep_tags(labels: Option<&Value>, body: &str) -> String {
    let Some(Value::Array(labels)) = labels else {
        return String::new();
    };
    let mut seen: HashSet<String> = text_tags::find(body).into_iter().map(|t| t.tag).collect();
    let mut tags = Vec::new();
    for label in labels {
        let Some(name) = label.get("name").and_then(Value::as_str) else {
            continue;
        };
        if let Some(tag) = text_tags::from_label(name)
            && seen.insert(lower_case(&tag))
        {
            tags.push(tag);
        }
    }
    tags.join(" ")
}

fn area(name: &str) -> Result<&str> {
    let trimmed = dart_trim(name);
    if trimmed.is_empty() {
        return Err(Error::Invalid("the Area's name must not be empty".into()));
    }
    Ok(trimmed)
}

/// Unicode's White_Space and the byte order mark, as Dart's `trim` knows
/// them.
fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// The part of `name` after its last `/` or `\`.
fn base(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// The name without its extension of the file `base`, and the format it
/// becomes a Note in, by its extension in any case of ASCII letters;
/// `None` for a hidden file and any other kind.
pub(crate) fn format_of(base: &str) -> Option<(&str, &'static str)> {
    if base.starts_with('.') {
        return None;
    }
    let (stem, extension) = base.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "md" | "markdown" => Some((stem, MARKDOWN)),
        "txt" => Some((stem, PLAIN)),
        _ => None,
    }
}

/// The text of `bytes`: UTF-8 without the byte order mark before it,
/// else Windows-1252, the encoding of older text files on Windows,
/// which reads every byte; line breaks as `\n`. `None` for bytes holding
/// a zero, which no text file does.
pub(crate) fn decode(bytes: &[u8]) -> Option<String> {
    let utf8 = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = match std::str::from_utf8(utf8) {
        Ok(text) => text.to_string(),
        Err(_) => bytes
            .iter()
            .map(|&b| match b {
                // The table holds all 32 of them; were one missing, the
                // byte would read as Latin-1, as the rest do.
                0x80..=0x9f => CP1252
                    .get(usize::from(b - 0x80))
                    .copied()
                    .unwrap_or(char::from(b)),
                _ => char::from(b),
            })
            .collect(),
    };
    if text.contains('\0') {
        return None;
    }
    Some(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// What Windows-1252 has at 0x80 to 0x9F; the five bytes it leaves open
/// read as Latin-1 does.
const CP1252: [char; 32] = [
    '\u{20ac}', '\u{0081}', '\u{201a}', '\u{0192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02c6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008d}', '\u{017d}', '\u{008f}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02dc}', '\u{2122}', '\u{0161}', '\u{203a}', '\u{0153}', '\u{009d}', '\u{017e}', '\u{0178}',
];

/// `text` without its blank lines at the start and blanks at the end,
/// under the file's name `stem` as its first line, a heading in
/// Markdown, unless its first line with words already says it (the
/// Note's title, in any case).
pub(crate) fn titled(stem: &str, text: &str, format: &str) -> String {
    let trimmed = text.trim_end_matches(is_space);
    let lead = trimmed.len() - trimmed.trim_start_matches(is_space).len();
    let body = &trimmed[trimmed[..lead].rfind('\n').map_or(0, |i| i + 1)..];
    let name = stem.replace(['\r', '\n'], " ");
    let name = dart_trim(&name);
    if name.is_empty() {
        return body.to_string();
    }
    let visible = text_format::visible(format, body);
    let first = visible
        .split('\n')
        .map(dart_trim)
        .find(|words| !words.is_empty());
    if first.is_some_and(|words| lower_case(words) == lower_case(name)) {
        return body.to_string();
    }
    let title = if format == MARKDOWN {
        format!("# {}", text_format::literal(format, name))
    } else {
        name.to_string()
    };
    if body.is_empty() {
        title
    } else {
        format!("{title}\n{body}")
    }
}

#[cfg(test)]
mod tests;
