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
use crate::import_formats::{self, ImportFormat};
use crate::notes::keys;
use crate::obsidian;
use crate::places::{Places, Stopping};
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
    /// How many Notes it was to write, when it was stopped before all of
    /// them were in (obf-1078): the `notes` written stay, the others are
    /// not made. `None` when it was not stopped.
    pub stopped: Option<usize>,
}

impl ImportSummary {
    /// How many files and notes were skipped.
    pub fn skipped(&self) -> usize {
        self.not_text + self.too_large + self.trashed + self.missing + self.not_photo
    }
}

/// How far an Import is (obf-1061): `written` of the `of` Notes it
/// writes are in, told before the first and after each.
pub type ImportProgress<'a> = &'a dyn Fn(usize, usize);

/// Imports into one store.
pub struct Imports<'a> {
    store: &'a Store,
    progress: Option<ImportProgress<'a>>,
    stop: Option<Stopping<'a>>,
}

impl<'a> Imports<'a> {
    pub fn new(store: &'a Store) -> Imports<'a> {
        Imports {
            store,
            progress: None,
            stop: None,
        }
    }

    /// These Imports, `progress` hearing how far the writing is, as the
    /// Dart core's `progress:`.
    pub fn reporting(self, progress: ImportProgress<'a>) -> Imports<'a> {
        Imports {
            progress: Some(progress),
            ..self
        }
    }

    /// These Imports, asking `stop` before each Note they write
    /// (obf-1078): once it says so, the Notes written stay and the
    /// summary says how many were to be ([`ImportSummary::stopped`]).
    pub fn stopping(self, stop: Stopping<'a>) -> Imports<'a> {
        Imports {
            stop: Some(stop),
            ..self
        }
    }

    /// Whether the hook, when there is one, says to stop.
    fn stop_asked(&self) -> bool {
        self.stop.is_some_and(|stop| stop())
    }

    /// Tells the hook, when there is one, that `written` of `of` are in.
    fn tell(&self, written: usize, of: usize) {
        if let Some(progress) = self.progress {
            progress(written, of);
        }
    }

    /// Makes one Note of each Markdown (`.md`, `.markdown`), text
    /// (`.txt`), AsciiDoc (`.adoc`, `.asciidoc`, `.asc`), Org (`.org`),
    /// reStructuredText (`.rst`) and HTML (`.html`, `.htm`) file of
    /// `files` in a new Area named `area_name`, as [`Imports::notes`]
    /// writes them; any other file is skipped and counted. A Markdown
    /// file's Note is formatted, its Markdown as written; a text file's
    /// is plain; the others' are formatted, read into the Markdown subset
    /// ([`import_formats`]). A Note is dated by its file's modification
    /// time, and by now when that is unknown or later.
    pub fn folder(&self, area_name: &str, files: &[ImportFile]) -> Result<ImportSummary> {
        let name = area(area_name)?;
        let now = self.store.now();
        let mut made = Vec::new();
        let (mut not_text, mut too_large) = (0, 0);
        for file in files {
            let base = base(&file.name);
            let document = document_of(base);
            let Some((stem, format)) = document
                .map(|(stem, _)| (stem, MARKDOWN))
                .or_else(|| format_of(base))
            else {
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
            let text = match document {
                // A page's own title stands under the file's name, unless
                // the text begins with it, as the name does.
                Some((_, kind)) => {
                    let page = import_formats::read(kind, &read);
                    titled(stem, &titled(&page.title, &page.text, format), format)
                }
                None => titled(stem, &read, format),
            };
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

    /// Makes one Note of each Markdown file of `files` in a new Area
    /// named `area_name`, as [`Imports::folder`] makes it (obf-1056): the
    /// file opened in the desktop's viewer and the files it links to, by
    /// their paths in the folder their links are found in
    /// ([`crate::obsidian`]). A wiki link to another of them becomes an
    /// app link to its Note, `[[Name|label]]` showing its label; one to
    /// a file not among them, or skipped, stays as written. A picture it
    /// embeds, `![[picture.png]]` or `![alt](picture.png)`, that is among
    /// `pictures`, by its path there, becomes a photo of the Note, the
    /// embed left out of its text; one larger than [`MAX_PHOTO_BYTES`] is
    /// counted as too large and stays as written. Any other file of
    /// `files` is skipped and counted.
    pub fn linked(
        &self,
        area_name: &str,
        files: &[ImportFile],
        pictures: &[ImportFile],
    ) -> Result<ImportSummary> {
        let name = area(area_name)?;
        let now = self.store.now();
        let mut skipped = ImportSummary::default();
        let picture_paths: Vec<String> = pictures.iter().map(|p| slashed(&p.name)).collect();
        let mut read: Vec<LinkedRead> = Vec::new();
        for file in files {
            let path = slashed(&file.name);
            let base = base(&path);
            let Some((stem, MARKDOWN)) = format_of(base) else {
                skipped.not_text += 1;
                continue;
            };
            if file.bytes.len() > MAX_FILE_BYTES {
                skipped.too_large += 1;
                continue;
            }
            let Some(text) = decode(&file.bytes) else {
                skipped.not_text += 1;
                continue;
            };
            let (text, photos, large) = taken_pictures(&text, &path, pictures, &picture_paths);
            skipped.too_large += large;
            let at = match file.modified.map(|at| at.trunc_subsecs(6)) {
                Some(at) if at > DateTime::UNIX_EPOCH && at <= now => at,
                _ => now,
            };
            read.push((path.clone(), stem.to_string(), text, at, photos));
        }
        let paths: Vec<String> = read.iter().map(|r| r.0.clone()).collect();
        let ids = read
            .iter()
            .map(|_| Ok(format!("note:{}", self.store.new_id()?)))
            .collect::<Result<Vec<String>>>()?;
        // Links only to the Notes kept: those dropped as too large are
        // left as written, and the rest is linked anew without them.
        let mut linkable = vec![true; read.len()];
        loop {
            let notes: Vec<ImportedNote> = read
                .iter()
                .map(|(path, stem, text, at, photos)| {
                    let linked = with_links(text, (&paths, &linkable, &ids));
                    let text = titled(stem, &linked, MARKDOWN);
                    ImportedNote {
                        name: base(path).to_string(),
                        text,
                        format: MARKDOWN.to_string(),
                        captured: *at,
                        lines: None,
                        photos: photos.clone(),
                    }
                })
                .collect();
            let (kept, _) = kept(&notes);
            // Only ever fewer, so this ends: once every Note linked to is
            // kept, the texts are written as they are.
            let now_linkable: Vec<bool> = linkable
                .iter()
                .enumerate()
                .map(|(i, &l)| l && kept.iter().any(|(k, _)| *k == i))
                .collect();
            // Every file has a name, so no Note is empty.
            if now_linkable == linkable {
                return self.write_as(name, &notes, &skipped, &ids);
            }
            linkable = now_linkable;
        }
    }

    /// Writes `notes` into a new Area named `area_name`, oldest first,
    /// filed there so none of them is Unsorted or Nudged, each captured
    /// and dated when it says, with its photos and, for a Checklist, its
    /// Lines, checked when they say. An Area of that name already there gets a
    /// new one beside it, named with a number after it. A text larger
    /// than [`MAX_TEXT_BYTES`], and the oldest beyond
    /// [`MAX_TOTAL_BYTES`], are skipped as too large, counted on top of
    /// `not_text` and `too_large`; with nothing left, no Area is made.
    /// The Area and each Note are a transaction of their own, so another
    /// connection to the database, such as the window's while the Import
    /// runs beside it (obf-1061), waits at most for one Note; the hook of
    /// [`Imports::reporting`] hears how many are in.
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
        self.write_as(area_name, notes, skipped, &[])
    }

    /// As [`Imports::write`], each note getting the id `ids` holds at its
    /// place, a new one where it holds none (obf-1056).
    fn write_as(
        &self,
        area_name: &str,
        notes: &[ImportedNote],
        skipped: &ImportSummary,
        ids: &[String],
    ) -> Result<ImportSummary> {
        let name = area(area_name)?;
        if let Some(note) = notes.iter().find(|n| !KNOWN.contains(&n.format.as_str())) {
            return Err(Error::Invalid(format!(
                "not a format known here: {}",
                note.format
            )));
        }
        let (kept, large) = kept(notes);
        let summary = ImportSummary {
            area_id: None,
            notes: 0,
            too_large: skipped.too_large + large,
            checklists: 0,
            photos: 0,
            ..skipped.clone()
        };
        if kept.is_empty() {
            return Ok(summary);
        }
        // Stopped before anything was written: no Area either.
        if self.stop_asked() {
            return Ok(ImportSummary {
                stopped: Some(kept.len()),
                ..summary
            });
        }
        let kept: Vec<(&ImportedNote, DateTime<Utc>, Option<&String>)> = kept
            .into_iter()
            .filter_map(|(i, at)| Some((notes.get(i)?, at, ids.get(i))))
            .collect();
        let s = self.store;
        // The bytes are kept before the log names them, each photo once.
        let mut hashes = Vec::new();
        for (note, _, _) in &kept {
            let mut held: Vec<String> = Vec::new();
            for bytes in &note.photos {
                let hash = s.put_photo(bytes)?;
                if !held.contains(&hash) {
                    held.push(hash);
                }
            }
            hashes.push(held);
        }
        let area_id = s.batch(|| Ok(Places::new(s).add_area(&self.unused(name)?)?.id))?;
        self.tell(0, kept.len());
        let mut written = kept.len();
        for (i, ((note, at, given), held)) in kept.iter().zip(&hashes).enumerate() {
            if self.stop_asked() {
                written = i;
                break;
            }
            s.batch(|| {
                let id = match given {
                    Some(id) => (*id).clone(),
                    None => format!("note:{}", s.new_id()?),
                };
                let date = Some(*at);
                s.append(&id, keys::TYPE, Some(keys::TYPE_NOTE), date)?;
                s.append(&id, keys::CAPTURED, Some(&dart_iso(*at)), date)?;
                let boxed = text_boxes::convert(&note.text, &note.format, || s.new_id())?;
                let text = StoredText::of(&note.format, &boxed, VERSION).encode();
                s.append(&id, keys::TEXT, Some(&text), date)?;
                s.append(&id, keys::PLACE, Some(&area_id), date)?;
                for hash in held {
                    s.append(&id, &keys::photo(hash), Some(keys::ADDED), date)?;
                }
                match &note.lines {
                    Some(lines) => self.write_lines(&id, lines, *at),
                    None => Ok(()),
                }
            })?;
            self.tell(i + 1, kept.len());
        }
        let made = kept.get(..written).unwrap_or_default();
        Ok(ImportSummary {
            area_id: Some(area_id),
            notes: written,
            checklists: made.iter().filter(|(n, _, _)| n.lines.is_some()).count(),
            photos: hashes.iter().take(written).map(Vec::len).sum(),
            stopped: (written < kept.len()).then_some(kept.len()),
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

/// A Markdown file read for [`Imports::linked`]: its path, its name
/// without the ending, its text without the pictures taken, when it is
/// dated and its photos.
type LinkedRead = (String, String, String, DateTime<Utc>, Vec<Vec<u8>>);

/// `path` with its folders separated by `/`.
fn slashed(path: &str) -> String {
    path.replace('\\', "/")
}

/// `text`, of the file at `path`, without the pictures it embeds that are
/// among `pictures` (by their `paths`), with their bytes, each once; and
/// how many of them were too large to take, which stay as written.
fn taken_pictures(
    text: &str,
    path: &str,
    pictures: &[ImportFile],
    paths: &[String],
) -> (String, Vec<Vec<u8>>, usize) {
    let mut cuts: Vec<(usize, usize, usize)> = Vec::new();
    for link in obsidian::wiki_links(text).into_iter().filter(|l| l.embed) {
        if let Some(found) = obsidian::file_target(&link.target, path, paths) {
            cuts.push((link.start, link.end, found));
        }
    }
    for picture in obsidian::pictures(text) {
        if let Some(found) = obsidian::picture_target(&picture.path, path, paths) {
            cuts.push((picture.start, picture.end, found));
        }
    }
    cuts.sort_unstable();
    let mut out = String::with_capacity(text.len());
    let mut photos: Vec<Vec<u8>> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    let mut large = 0;
    let mut at = 0;
    for (start, end, found) in cuts {
        let Some(picture) = pictures.get(found) else {
            continue;
        };
        if start < at {
            continue;
        }
        if picture.bytes.len() > MAX_PHOTO_BYTES {
            large += 1;
            continue;
        }
        out.push_str(text.get(at..start).unwrap_or_default());
        at = end;
        if !taken.contains(&found) {
            taken.push(found);
            photos.push(picture.bytes.clone());
        }
    }
    out.push_str(text.get(at..).unwrap_or_default());
    (out, photos, large)
}

/// `text` with each wiki link to one of the files at `paths` that is
/// `linkable` as a link to its Note, by its id in `ids`: `[shown](app
/// link)`. Embeds and links to any other file stay as written.
fn with_links(text: &str, (paths, linkable, ids): (&[String], &[bool], &[String])) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for link in obsidian::wiki_links(text) {
        if link.embed || link.target.is_empty() {
            continue;
        }
        let Some(found) = obsidian::note_target(&link.target, paths) else {
            continue;
        };
        let (Some(true), Some(id)) = (linkable.get(found).copied(), ids.get(found)) else {
            continue;
        };
        out.push_str(text.get(at..link.start).unwrap_or_default());
        let url = crate::app_link::AppLink::Note(id.clone()).url();
        out.push_str(&format!("[{}]({url})", label_escaped(&link.shown())));
        at = link.end;
    }
    out.push_str(text.get(at..).unwrap_or_default());
    out
}

/// `label` as a link's words that read as written: a `\\` before each
/// mark that could end the link or start a format in it.
fn label_escaped(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for c in label.chars() {
        if matches!(c, '\\' | '[' | ']' | '*' | '_' | '~' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Which of `notes` an Import keeps, oldest first, by their places in
/// `notes`, each with when it was captured; and how many it skips as too
/// large: a text larger than [`MAX_TEXT_BYTES`], and the oldest beyond
/// [`MAX_TOTAL_BYTES`].
fn kept(notes: &[ImportedNote]) -> (Vec<(usize, DateTime<Utc>)>, usize) {
    let mut order: Vec<(usize, &ImportedNote, DateTime<Utc>)> = notes
        .iter()
        .enumerate()
        .map(|(i, n)| (i, n, n.captured.trunc_subsecs(6)))
        .collect();
    order.sort_by(|a, b| (a.2, &a.1.name, a.0).cmp(&(b.2, &b.1.name, b.0)));
    let mut kept = Vec::new();
    let mut large = 0;
    let mut total = 0;
    for (i, note, at) in order.iter().rev() {
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
        kept.push((*i, *at));
    }
    kept.reverse();
    (kept, large)
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

/// The name without its extension of the file `base`, and the format of
/// a document of another tool it is, by its extension in any case of
/// ASCII letters; `None` for a hidden file and any other kind.
fn document_of(base: &str) -> Option<(&str, ImportFormat)> {
    if base.starts_with('.') {
        return None;
    }
    let (stem, extension) = base.rsplit_once('.')?;
    ImportFormat::of_extension(&extension.to_ascii_lowercase()).map(|kind| (stem, kind))
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
