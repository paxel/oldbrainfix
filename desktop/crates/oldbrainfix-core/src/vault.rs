//! The vault (#267, ADR 0050): every Note this Device can read written
//! out as a folder of Markdown files, a folder per Area and a subfolder
//! per Topic, for the day the user leaves for another tool; and the way
//! back, a rebuild of such a folder into an empty Old Brain Fix. Only the
//! desktop has it, so there is no Dart twin.
//!
//! A Note's file holds YAML front matter with what the Markdown does not
//! show (its id, capture time, Area, Topic, Importance, author, kind,
//! Routines and Reminders with their check-offs, Frogs, attached files
//! and Sections), then its text as stored, each Section after a marker
//! line, and last what the app draws of its kind (a task list, a table,
//! links) and its files, after a marker of its own, for the other tool
//! only. Each property is one line, its value JSON, which YAML reads.
//!
//! A rebuild reads a file with these properties back with them, every
//! entity under a new id and every Area new; a file made elsewhere
//! becomes a Note as an Import makes one (ADR 0036), filed by its
//! folders. Everything is signed by this Device.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, SubsecRound, Utc};
use serde_json::{Map, Value, json};

use crate::app_link::{self, AppLink};
use crate::checklists::{Checklists, NewLine, keys as checklist_keys, order, write_line_as};
use crate::entry::{dart_iso, parse_time};
use crate::file_text;
use crate::frogs::{Frogs, keys as frog_keys};
use crate::imports;
use crate::markdown::{self, Span, SpanKind};
use crate::note_kinds::NoteKind;
use crate::notes::{self, Attachment, Note, Notes, Section, keys as note_keys};
use crate::people::PersonFilter;
use crate::places::{Area, Places, Topic, keys as place_keys};
use crate::profile::OwnProfile;
use crate::routines::{Routines, keys as routine_keys};
use crate::schedule::WallClock;
use crate::search::lower_case;
use crate::sessions::{Sessions, keys as session_keys};
use crate::shopping_lists::{ListOnAgenda, ShoppingLists, keys as shopping_keys};
use crate::store::Store;
use crate::tallies::{Tallies, TallyEntry, TallyPeriod, keys as tally_keys};
use crate::tally_units::encode_number;
use crate::text_boxes;
use crate::text_format::{self, KNOWN, StoredText, VERSION};
use crate::text_tags;
use crate::transcript_prompts::dart_trim;
use crate::{Error, Result};

/// The format of the vault this version writes and reads (ADR 0034): a
/// file of a later one needs an update.
pub const FORMAT: i64 = 1;

/// The property that marks a Note's file as Old Brain Fix's, its value
/// the [`FORMAT`].
const MARK: &str = "oldbrainfix";

/// The folder beside a Note's file holding its photos, recordings and
/// attached files; no Area or Topic is written under this name.
pub const ATTACHMENTS: &str = "attachments";

/// The line before what the app draws for the other tool only.
const SHOWN: &str = "<!-- oldbrainfix:shown -->";

/// The start of the line before a Section, its id and [`MARKER_END`]
/// after it.
const SECTION: &str = "<!-- oldbrainfix:section ";
const MARKER_END: &str = " -->";

/// The most characters, and bytes, of a name taken from a title or a
/// place, so a name with its date and number stays under the 255 bytes
/// file systems allow.
const NAME_CHARS: usize = 60;
const NAME_BYTES: usize = 180;

/// The largest Note's file read back: its text, its Sections and the
/// entries of a Tally in its properties.
const MAX_NOTE_FILE: u64 = 32 << 20;

/// Why a vault was not rebuilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultProblem {
    /// Something is written here already: a vault goes into an empty Old
    /// Brain Fix only, as a Backup restores.
    NotEmpty,
    /// The folder holds no Note: no Markdown or text file to read.
    NotAVault,
    /// A file of a vault a later version wrote (ADR 0034).
    NeedsUpdate,
}

fn problem(p: VaultProblem) -> Error {
    Error::Vault(p)
}

/// The names the export gives what the app names in its language: an
/// Area and a Topic as they show, the folder of the Notes filed nowhere,
/// and a Note without words or a file to be named by.
pub struct VaultNames<'a> {
    pub area: &'a dyn Fn(&Area) -> String,
    pub topic: &'a dyn Fn(&Topic) -> String,
    pub unsorted: &'a str,
    pub untitled: &'a str,
}

/// What an export wrote: the vault's folder, how many Notes, how many
/// files beside them, and how many of their files this Device does not
/// hold (not synced yet), named in their properties only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub folder: PathBuf,
    pub notes: usize,
    pub files: usize,
    pub missing: usize,
}

/// What a rebuild made: the Notes, of them those made elsewhere, the
/// files attached again, those named but not found in the folder, and
/// the Markdown and text files that could not be read as a Note.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rebuilt {
    pub notes: usize,
    pub elsewhere: usize,
    pub files: usize,
    pub missing: usize,
    pub skipped: usize,
}

// ------------------------------------------------------------- names

/// Names already taken in one folder, compared in lower case, as file
/// systems on Windows and macOS compare them.
#[derive(Default)]
struct Taken(HashSet<String>);

impl Taken {
    /// A folder whose names `reserved` are never handed out.
    fn reserving(reserved: &[&str]) -> Taken {
        Taken(reserved.iter().map(|n| lower_case(n)).collect())
    }

    /// `stem` with `extension`, or with " (2)", " (3)" and so on before
    /// it, the first not taken; taken from now on.
    fn take(&mut self, stem: &str, extension: &str) -> String {
        let mut n = 1;
        loop {
            let name = if n == 1 {
                format!("{stem}{extension}")
            } else {
                format!("{stem} ({n}){extension}")
            };
            if self.0.insert(lower_case(&name)) {
                return name;
            }
            n += 1;
        }
    }
}

/// `text` as a name every file system takes: no folder separators, no
/// `: * ? " < > |`, none of `# ^ [ ]` that break a link in Obsidian, no
/// control or direction character, blanks as one space, no dot or space
/// at either end (a hidden file, a name Windows refuses), at most
/// [`NAME_CHARS`] characters and [`NAME_BYTES`] bytes, a name Windows
/// keeps for a device with `_` after it; `fallback` when nothing is left.
pub fn safe_name(text: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in text.chars() {
        let bad = c.is_control()
            || c.is_whitespace()
            || matches!(
                c,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']'
            )
            || matches!(c, '\u{200e}' | '\u{200f}' | '\u{feff}')
            || ('\u{202a}'..='\u{202e}').contains(&c)
            || ('\u{2066}'..='\u{2069}').contains(&c);
        if bad {
            gap = true;
            continue;
        }
        if gap && !out.is_empty() {
            out.push(' ');
        }
        gap = false;
        out.push(c);
    }
    let mut cut = String::new();
    for c in out.trim_matches(['.', ' ']).chars() {
        if cut.chars().count() >= NAME_CHARS || cut.len() + c.len_utf8() > NAME_BYTES {
            break;
        }
        cut.push(c);
    }
    let name = cut.trim_matches(['.', ' ']).to_string();
    if name.is_empty() {
        return safe_name_or(fallback);
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.chars().last().is_some_and(|d| d.is_ascii_digit()));
    if device { format!("{name}_") } else { name }
}

/// `fallback` cleaned as [`safe_name`] cleans, `Note` when nothing is
/// left of it either.
fn safe_name_or(fallback: &str) -> String {
    let name = safe_name(fallback, "Note");
    if name.is_empty() { "Note".into() } else { name }
}

/// A file's or folder's name in a link: each ASCII character but letters,
/// digits and `-._~` as `%` and its hex, as the desktop's "Save as
/// Markdown…" writes links; other characters as they are.
fn link_part(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if !c.is_ascii() || c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", c as u32));
        }
    }
    out
}

/// `text` with each `%` and two hex digits as the byte they name, read as
/// UTF-8; `None` when that is no UTF-8.
fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (b, hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// The link from a file in the folder `from` to the file `file` in the
/// folder `to`, both by their names under the vault.
fn relative(from: &[String], to: &[String], file: &str) -> String {
    let common = from.iter().zip(to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = (common..from.len()).map(|_| "..".to_string()).collect();
    parts.extend(to.iter().skip(common).map(|p| link_part(p)));
    parts.push(link_part(file));
    parts.join("/")
}

/// The file ending of the photo `bytes`, as the desktop names one.
fn photo_extension(bytes: &[u8]) -> &'static str {
    let tag = |at: usize| bytes.get(at..at + 4).unwrap_or_default();
    match tag(0) {
        b"\x89PNG" => "png",
        b"GIF8" => "gif",
        b"RIFF" if tag(8) == b"WEBP" => "webp",
        _ => "jpg",
    }
}

/// The file ending of the voice recording `bytes`, as the desktop names
/// one: WAV, Ogg, MP3, else AAC in MPEG-4.
fn recording_extension(bytes: &[u8]) -> &'static str {
    let tag = |at: usize| bytes.get(at..at + 4).unwrap_or_default();
    let byte = |at: usize| bytes.get(at).copied().unwrap_or(0);
    if tag(0) == b"RIFF" && tag(8) == b"WAVE" {
        "wav"
    } else if tag(0) == b"OggS" {
        "ogg"
    } else if bytes.starts_with(b"ID3")
        || (byte(0) == 0xFF && byte(1) & 0xE0 == 0xE0 && byte(1) & 0x06 != 0)
    {
        "mp3"
    } else {
        "m4a"
    }
}

/// The first 12 characters of `hash`.
fn short(hash: &str) -> &str {
    hash.char_indices()
        .nth(12)
        .map_or(hash, |(at, _)| hash.get(..at).unwrap_or(hash))
}

// ------------------------------------------------------------- links

/// The source range of the destination of the link `span` in `text`:
/// between its `](` and its `)`.
fn destination(text: &str, span: &Span) -> Option<(usize, usize)> {
    let from = span.content_end.checked_add(2)?;
    let to = span.end.checked_sub(1)?;
    let opens = text.get(span.content_end..from)? == "](";
    let closes = text.get(to..span.end)? == ")";
    (opens && closes && from <= to && text.get(from..to).is_some()).then_some((from, to))
}

fn link_edits(
    text: &str,
    spans: &[Span],
    to: &dyn Fn(&str) -> Option<String>,
    edits: &mut Vec<(usize, usize, String)>,
) {
    for span in spans {
        if span.kind == SpanKind::Link
            && let Some((from, end)) = destination(text, span)
            && let Some(new) = to(&span.url)
        {
            edits.push((from, end, new));
        }
        link_edits(text, &span.children, to, edits);
    }
}

/// The formatted `text` with each link whose destination `to` gives
/// another one pointing there instead, the rest byte for byte.
fn relinked(text: &str, to: &dyn Fn(&str) -> Option<String>) -> String {
    let mut edits = Vec::new();
    for block in markdown::parse(text) {
        link_edits(text, &block.spans, to, &mut edits);
    }
    edits.sort_by_key(|(from, _, _)| *from);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (from, end, new) in edits {
        let Some(before) = text.get(at..from) else {
            continue;
        };
        out.push_str(before);
        out.push_str(&new);
        at = end;
    }
    out.push_str(text.get(at..).unwrap_or_default());
    out
}

// ---------------------------------------------------------- properties

/// Writes JSON on one line with a space after each `,` and `:`, which
/// every YAML reader takes as a flow value.
struct Spaced;

impl serde_json::ser::Formatter for Spaced {
    fn begin_array_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_key<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if first {
            Ok(())
        } else {
            writer.write_all(b", ")
        }
    }

    fn begin_object_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        writer.write_all(b": ")
    }
}

/// `value` as one line of JSON, each character YAML does not allow as it
/// is (delete, the C1 controls, the byte order mark, the line and
/// paragraph separators, the two non-characters) as `\u` and its hex.
fn property_value(value: &Value) -> Result<String> {
    let mut bytes = Vec::new();
    let mut writer = serde_json::Serializer::with_formatter(&mut bytes, Spaced);
    serde::Serialize::serialize(value, &mut writer)
        .map_err(|e| Error::Invalid(format!("vault: {e}")))?;
    let text = String::from_utf8(bytes).map_err(|e| Error::Invalid(format!("vault: {e}")))?;
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let escaped = c == '\u{7f}'
            || ('\u{80}'..='\u{9f}').contains(&c)
            || matches!(
                c,
                '\u{feff}' | '\u{2028}' | '\u{2029}' | '\u{fffe}' | '\u{ffff}'
            );
        if escaped {
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Whether `key` is written as a property is: ASCII letters and digits,
/// a letter first.
fn property_key(key: &str) -> bool {
    key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && key.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Each property of a front matter by its key, `None` for one that
/// cannot be read.
type Properties = Vec<(String, Option<Value>)>;

/// The front matter of `text`, when it begins with one: each property's
/// key and its value, `None` for a value that is no JSON and for the
/// property a line that is no property follows; and the text after it.
fn front_matter(text: &str) -> Option<(Properties, &str)> {
    let mut rest = text.strip_prefix("---\n")?;
    let mut props: Vec<(String, Option<Value>)> = Vec::new();
    loop {
        let (line, after) = match rest.split_once('\n') {
            Some(split) => split,
            None if rest == "---" => (rest, ""),
            None => return None,
        };
        if line == "---" {
            return Some((props, after));
        }
        let parsed = line
            .split_once(':')
            .filter(|(key, _)| property_key(key))
            .map(|(key, value)| (key.to_string(), serde_json::from_str(value.trim()).ok()));
        match parsed {
            Some(prop) => props.push(prop),
            None => match props.last_mut() {
                Some(last) => last.1 = None,
                None => props.push((String::new(), None)),
            },
        }
        rest = after;
    }
}

// --------------------------------------------------------------- export

/// A Note to write: where, under which name, with its Sections.
struct Planned {
    note: Note,
    sections: Vec<Section>,
    dir: Vec<String>,
    file: String,
}

/// The attached files of one folder: their names, and the name each file
/// was written under, so a file two Notes hold is written once.
#[derive(Default)]
struct Beside {
    taken: Taken,
    written: HashMap<(&'static str, String), String>,
}

/// What a Note's file says of one photo, recording or file, and the
/// Markdown that draws it, when its bytes went along.
struct Written {
    property: Value,
    drawn: Option<String>,
}

/// Writes every Note this Device reads, but those in the trash and the
/// help Notes, as a vault into a new folder in `parent`, named
/// `oldbrainfix-vault-<date>` and a number when that is taken: nothing
/// already in `parent` is touched. `names` names what the app names,
/// `wall` gives the dates of the file names and Sections, and
/// `progress` hears how many Notes of how many are written. When writing
/// fails, the new folder goes again.
pub fn export(
    store: &Store,
    parent: &Path,
    names: &VaultNames,
    wall: &WallClock,
    progress: &dyn Fn(usize, usize),
) -> Result<Exported> {
    let plan = plan(store, names, wall)?;
    let folder = new_folder(parent, &wall(store.now()).format("%Y-%m-%d").to_string())?;
    match write_plan(store, &folder, &plan, wall, progress) {
        Ok((files, missing)) => Ok(Exported {
            folder,
            notes: plan.len(),
            files,
            missing,
        }),
        Err(e) => {
            drop(std::fs::remove_dir_all(&folder));
            Err(e)
        }
    }
}

/// A new folder in `parent` for a vault of `day`.
fn new_folder(parent: &Path, day: &str) -> Result<PathBuf> {
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("oldbrainfix-vault-{day}")
        } else {
            format!("oldbrainfix-vault-{day}-{n}")
        };
        let path = parent.join(name);
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e.into()),
        }
    }
}

/// Whether `id` is one of the help Notes the app makes itself (ADR
/// 0042); a rebuild's start makes them again.
fn is_help(id: &str) -> bool {
    id.starts_with("note:help-")
}

/// The first line with a letter or digit of `note` as it reads, else the
/// name of its file, else `untitled`.
fn title(note: &Note, untitled: &str) -> String {
    let visible = note.visible_text();
    let worded = |l: &&str| l.chars().any(char::is_alphanumeric);
    if let Some(line) = visible.lines().map(dart_trim).find(worded) {
        return line.to_string();
    }
    let name = note.wordless_name();
    if name.is_empty() {
        untitled.to_string()
    } else {
        name
    }
}

/// Where each Note goes: a folder per Area by the name it shows, a
/// subfolder per Topic, the Notes filed nowhere in a folder of their
/// own; each Note's file by its title and capture date. Areas and
/// Topics go by name, then id; Notes by capture time, then id.
fn plan(store: &Store, names: &VaultNames, wall: &WallClock) -> Result<Vec<Planned>> {
    let places = Places::new(store);
    let notes = Notes::new(store);
    let mut root = Taken::reserving(&[ATTACHMENTS]);
    let mut folders: HashMap<String, Vec<String>> = HashMap::new();
    let mut areas = places.areas()?;
    areas.sort_by_cached_key(|a| (lower_case(&(names.area)(a)), a.id.clone()));
    for area in &areas {
        let dir = root.take(&safe_name(&(names.area)(area), "Area"), "");
        let mut topics = places.topics(&area.id)?;
        topics.sort_by_cached_key(|t| (lower_case(&(names.topic)(t)), t.id.clone()));
        let mut inside = Taken::reserving(&[ATTACHMENTS]);
        for topic in &topics {
            let sub = inside.take(&safe_name(&(names.topic)(topic), "Topic"), "");
            folders.insert(topic.id.clone(), vec![dir.clone(), sub]);
        }
        folders.insert(area.id.clone(), vec![dir]);
    }
    let unsorted = root.take(&safe_name(names.unsorted, "Unsorted"), "");
    let mut all: Vec<Note> = notes
        .all()?
        .into_iter()
        .filter(|n| !is_help(&n.id))
        .collect();
    all.sort_by(|a, b| (a.captured, &a.id).cmp(&(b.captured, &b.id)));
    let mut in_folder: HashMap<Vec<String>, Taken> = HashMap::new();
    let mut plan = Vec::new();
    for note in all {
        let dir = note
            .place
            .as_ref()
            .and_then(|p| folders.get(p))
            .cloned()
            .unwrap_or_else(|| vec![unsorted.clone()]);
        let taken = in_folder
            .entry(dir.clone())
            .or_insert_with(|| Taken::reserving(&[ATTACHMENTS]));
        let date = wall(note.captured).format("%Y-%m-%d");
        let stem = format!(
            "{} {date}",
            safe_name(&title(&note, names.untitled), "Note")
        );
        let file = taken.take(&stem, ".md");
        let sections = notes.sections(&note.id)?;
        plan.push(Planned {
            note,
            sections,
            dir,
            file,
        });
    }
    Ok(plan)
}

/// Writes the Notes of `plan` into `root`; how many files went beside
/// them and how many were not here.
fn write_plan(
    store: &Store,
    root: &Path,
    plan: &[Planned],
    wall: &WallClock,
    progress: &dyn Fn(usize, usize),
) -> Result<(usize, usize)> {
    let mut at: HashMap<&str, usize> = HashMap::new();
    for (i, p) in plan.iter().enumerate() {
        at.insert(&p.note.id, i);
        for s in p.sections.iter().filter(|s| s.id.starts_with("note:")) {
            at.entry(&s.id).or_insert(i);
        }
    }
    let own = OwnProfile::new(store).get()?.map(|p| p.name);
    let mut beside: HashMap<Vec<String>, Beside> = HashMap::new();
    let (mut files, mut missing) = (0, 0);
    for (i, p) in plan.iter().enumerate() {
        let dir = p.dir.iter().fold(root.to_path_buf(), |d, c| d.join(c));
        std::fs::create_dir_all(&dir)?;
        let link = |url: &str| -> Option<String> {
            let Some(AppLink::Note(id)) = app_link::parse(url) else {
                return None;
            };
            let target = plan.get(*at.get(id.as_str())?)?;
            Some(relative(&p.dir, &target.dir, &target.file))
        };
        let held = beside.entry(p.dir.clone()).or_default();
        let mut write = |kind: &'static str,
                         hash: &str,
                         text: Option<&String>,
                         file: Option<&Attachment>,
                         removed: bool| {
            let w = write_blob(store, &dir, held, (kind, hash), text, file, removed)?;
            if w.drawn.is_some() {
                files += 1;
            } else if !removed {
                missing += 1;
            }
            Ok::<Written, Error>(w)
        };
        let mut drawn = Vec::new();
        let mut attachments = Vec::new();
        for w in held_files(&p.note, &mut write)? {
            drawn.extend(w.drawn.clone());
            attachments.push(w.property);
        }
        let mut sections = Vec::new();
        let mut body = shown_text(&p.note.shown_text(), &p.note.text_format, &link);
        for s in &p.sections {
            let mut entry = Map::new();
            entry.insert("id".into(), json!(s.id));
            entry.insert("captured".into(), json!(dart_iso(s.captured)));
            entry.insert("format".into(), json!(s.text_format));
            if s.text_version != VERSION {
                entry.insert("textVersion".into(), json!(s.text_version));
            }
            if s.id.starts_with("note:") {
                entry.insert("merged".into(), json!(true));
                let mut list = Vec::new();
                for w in section_files(s, &mut write)? {
                    drawn.extend(w.drawn.clone());
                    list.push(w.property);
                }
                if !list.is_empty() {
                    entry.insert("attachments".into(), Value::Array(list));
                }
            }
            sections.push(Value::Object(entry));
            let date = wall(s.captured).format("%Y-%m-%d %H:%M");
            body.push_str(&format!("\n\n{SECTION}{}{MARKER_END}\n*{date}*\n\n", s.id));
            body.push_str(&shown_text(&s.shown_text(), &s.text_format, &link));
        }
        let pool_link = |id: &str| {
            let t = plan.get(*at.get(id)?)?;
            let name = t.file.strip_suffix(".md").unwrap_or(&t.file).to_string();
            Some((name, relative(&p.dir, &t.dir, &t.file)))
        };
        let (mut props, kind_drawn) = properties(store, p, own.as_deref(), &pool_link)?;
        if !attachments.is_empty() {
            props.push(("attachments", Value::Array(attachments)));
        }
        if !sections.is_empty() {
            props.push(("sections", Value::Array(sections)));
        }
        let mut shown: Vec<String> = kind_drawn.into_iter().collect();
        if !drawn.is_empty() {
            shown.push(drawn.join("\n"));
        }
        if shows(&props) {
            body.push_str(&format!("\n\n{SHOWN}\n\n{}", shown.join("\n\n")));
        }
        let mut out = String::from("---\n");
        for (key, value) in &props {
            out.push_str(&format!("{key}: {}\n", property_value(value)?));
        }
        out.push_str("---\n");
        out.push_str(&body);
        out.push('\n');
        std::fs::write(dir.join(&p.file), out)?;
        progress(i + 1, plan.len());
    }
    Ok((files, missing))
}

/// `text` in `format` as the file holds it: as stored, a formatted
/// text's links to Notes written to their files by `link`.
fn shown_text(text: &str, format: &str, link: &dyn Fn(&str) -> Option<String>) -> String {
    if format == text_format::MARKDOWN {
        relinked(text, link)
    } else {
        text.to_string()
    }
}

type WriteBlob<'w> = dyn FnMut(&'static str, &str, Option<&String>, Option<&Attachment>, bool) -> Result<Written>
    + 'w;

/// The photos, recordings and files of `note`, removed ones included, in
/// that order, each written by `write`.
fn held_files(note: &Note, write: &mut WriteBlob) -> Result<Vec<Written>> {
    files_of(
        (&note.photos, &note.removed_photos, &note.photo_texts),
        (
            &note.recordings,
            &note.removed_recordings,
            &note.transcripts,
        ),
        (
            &note.attachments,
            &note.removed_attachments,
            &note.attachment_texts,
        ),
        write,
    )
}

/// [`held_files`] for a merged Note's Section.
fn section_files(s: &Section, write: &mut WriteBlob) -> Result<Vec<Written>> {
    files_of(
        (&s.photos, &s.removed_photos, &s.photo_texts),
        (&s.recordings, &s.removed_recordings, &s.transcripts),
        (&s.attachments, &s.removed_attachments, &s.attachment_texts),
        write,
    )
}

type Hashes<'n> = (
    &'n Vec<String>,
    &'n Vec<String>,
    &'n std::collections::BTreeMap<String, String>,
);
type Files<'n> = (
    &'n Vec<Attachment>,
    &'n Vec<Attachment>,
    &'n std::collections::BTreeMap<String, String>,
);

fn files_of(
    photos: Hashes,
    recordings: Hashes,
    files: Files,
    write: &mut WriteBlob,
) -> Result<Vec<Written>> {
    let mut out = Vec::new();
    for (kind, (held, removed, texts)) in [("photo", photos), ("audio", recordings)] {
        for hash in held {
            out.push(write(kind, hash, texts.get(hash), None, false)?);
        }
        for hash in removed {
            out.push(write(kind, hash, texts.get(hash), None, true)?);
        }
    }
    let (held, removed, texts) = files;
    for a in held {
        out.push(write("file", &a.hash, texts.get(&a.hash), Some(a), false)?);
    }
    for a in removed {
        out.push(write("file", &a.hash, texts.get(&a.hash), Some(a), true)?);
    }
    Ok(out)
}

/// Writes the bytes of the `kind` blob `hash` into the folder `dir`'s
/// attachments, once per folder, when this Device holds them (one at a
/// time, never more in memory) and it was not `removed` while its text
/// stays; what the Note's file says of it.
fn write_blob(
    store: &Store,
    dir: &Path,
    held: &mut Beside,
    (kind, hash): (&'static str, &str),
    text: Option<&String>,
    file: Option<&Attachment>,
    removed: bool,
) -> Result<Written> {
    let name = match held.written.get(&(kind, hash.to_string())) {
        _ if removed => None,
        Some(name) => Some(name.clone()),
        None => match store.blob(kind, hash) {
            Some(bytes) => {
                let name = match (kind, file) {
                    ("photo", _) => held.taken.take(
                        &format!("photo-{}", short(hash)),
                        &format!(".{}", photo_extension(&bytes)),
                    ),
                    ("audio", _) => held.taken.take(
                        &format!("recording-{}", short(hash)),
                        &format!(".{}", recording_extension(&bytes)),
                    ),
                    (_, Some(a)) => {
                        let clean = safe_name(&a.name, "file");
                        match clean.rsplit_once('.') {
                            Some((stem, ext)) if !stem.is_empty() => {
                                held.taken.take(stem, &format!(".{ext}"))
                            }
                            _ => held.taken.take(&clean, ""),
                        }
                    }
                    _ => held.taken.take(&format!("file-{}", short(hash)), ""),
                };
                let folder = dir.join(ATTACHMENTS);
                std::fs::create_dir_all(&folder)?;
                std::fs::write(folder.join(&name), &bytes)?;
                held.written.insert((kind, hash.to_string()), name.clone());
                Some(name)
            }
            None => None,
        },
    };
    let path = name.as_ref().map(|n| format!("{ATTACHMENTS}/{n}"));
    let mut property = Map::new();
    property.insert(
        "kind".into(),
        json!(match kind {
            "audio" => "recording",
            other => other,
        }),
    );
    property.insert("path".into(), json!(path));
    property.insert("hash".into(), json!(hash));
    if let Some(a) = file {
        property.insert("name".into(), json!(a.name));
        property.insert("type".into(), json!(a.media_type));
        if let Some(pages) = a.pages {
            property.insert("pages".into(), json!(pages));
        }
    }
    if let Some(text) = text {
        property.insert("text".into(), json!(text));
    }
    if removed {
        property.insert("removed".into(), json!(true));
    }
    let drawn = name.as_ref().map(|name| {
        let link = format!("{ATTACHMENTS}/{}", link_part(name));
        let bang = if kind == "photo" { "!" } else { "" };
        format!("{bang}[{}]({link})", markdown::escape(name))
    });
    Ok(Written {
        property: Value::Object(property),
        drawn,
    })
}

/// Whether a Note's file has the part drawn for the other tool, by its
/// properties: a kind's Lines, Tally or Session, or a file that went
/// along, its own or a merged Note's.
fn shows<K: AsRef<str>>(props: &[(K, Value)]) -> bool {
    let has_path = |list: Option<&Value>| {
        list.and_then(Value::as_array).is_some_and(|l| {
            l.iter()
                .any(|a| a.get("path").is_some_and(Value::is_string))
        })
    };
    props.iter().any(|(key, value)| match key.as_ref() {
        "lines" | "tally" | "session" => true,
        "attachments" => has_path(Some(value)),
        "sections" => value
            .as_array()
            .is_some_and(|s| s.iter().any(|s| has_path(s.get("attachments")))),
        _ => false,
    })
}

/// The properties a Note's file writes, by their keys, in their order.
type Named = Vec<(&'static str, Value)>;

/// The properties of the Note `p` but its attachments and Sections, and
/// the Markdown drawn of its kind.
fn properties(
    store: &Store,
    p: &Planned,
    own: Option<&str>,
    links: &PoolLink,
) -> Result<(Named, Option<String>)> {
    let note = &p.note;
    let id = &note.id;
    let places = Places::new(store);
    let mut props: Vec<(&'static str, Value)> = vec![
        (MARK, json!(FORMAT)),
        ("id", json!(id)),
        ("captured", json!(dart_iso(note.captured))),
        ("format", json!(note.text_format)),
    ];
    if note.text_version != VERSION {
        props.push(("textVersion", json!(note.text_version)));
    }
    if let Some(place) = &note.place {
        let topic = places.topic_by_id(place)?;
        let area_id = topic.as_ref().map_or(place.clone(), |t| t.area_id.clone());
        if let Some(area) = places.area_by_id(&area_id)? {
            props.push(("area", json!(area.name)));
            props.push(("areaId", json!(area.id)));
            if let Some(starter) = &area.starter {
                props.push(("areaStarter", json!(starter)));
            }
            if let Some(colour) = &area.colour {
                props.push(("areaColour", json!(colour)));
            }
            if let Some(icon) = &area.icon {
                props.push(("areaIcon", json!(icon)));
            }
            if store.is_marked_hidden_from_ai(&area.id)? {
                props.push(("areaHiddenFromAi", json!(true)));
            }
            if let Some(topic) = topic {
                props.push(("topic", json!(topic.name)));
                props.push(("topicId", json!(topic.id)));
                if let Some(starter) = &topic.starter {
                    props.push(("topicStarter", json!(starter)));
                }
            }
        }
    }
    if note.importance != notes::Importance::Normal {
        props.push(("importance", json!(note.importance.name())));
    }
    let author = match store.author_of(id)? {
        Some(p) => Some(p.name),
        None => own.map(str::to_string),
    };
    if let Some(author) = author.filter(|a| !a.is_empty()) {
        props.push(("author", json!(author)));
    }
    if store.is_marked_hidden_from_ai(id)? {
        props.push(("hiddenFromAi", json!(true)));
    }
    if note.spawned {
        props.push(("spawned", json!(true)));
    }
    let drawn = kind_properties(store, note, links, &mut props)?;
    let mut routines = Vec::new();
    let mut reminders = Vec::new();
    let mut all = Routines::new(store).of_note(id)?;
    all.retain(|r| !r.is_trashed());
    all.sort_by(|a, b| (a.scheduled_at, &a.id).cmp(&(b.scheduled_at, &b.id)));
    for r in &all {
        let fields = store.current_fields(&r.id)?;
        let mut out = Map::new();
        out.insert("id".into(), json!(r.id));
        for key in [
            routine_keys::START,
            routine_keys::TIMES,
            routine_keys::RULE,
            routine_keys::SCHEDULE_VERSION,
            routine_keys::RENAG,
            routine_keys::ESCALATION,
        ] {
            if let Some(value) = fields.get(key) {
                out.insert(key.into(), json!(value));
            }
        }
        if let Some(at) = r.scheduled_at {
            out.insert("scheduled".into(), json!(dart_iso(at)));
        }
        for (name, prefix) in [
            ("done", routine_keys::DONE_PREFIX),
            ("skipped", routine_keys::SKIP_PREFIX),
            ("checks", checklist_keys::CHECK_PREFIX),
            ("picks", session_keys::PICK_PREFIX),
            ("entries", session_keys::ENTRY_PREFIX),
        ] {
            let marks: Map<String, Value> = fields
                .iter()
                .filter_map(|(k, v)| Some((k.strip_prefix(prefix)?.to_string(), json!(v?))))
                .collect();
            if !marks.is_empty() {
                out.insert(name.into(), Value::Object(marks));
            }
        }
        if r.is_reminder() {
            reminders.push(Value::Object(out));
        } else {
            routines.push(Value::Object(out));
        }
    }
    if !routines.is_empty() {
        props.push(("routines", Value::Array(routines)));
    }
    if !reminders.is_empty() {
        props.push(("reminders", Value::Array(reminders)));
    }
    let frogs: Vec<Value> = Frogs::new(store)
        .of_note(id)?
        .iter()
        .filter(|f| !f.is_trashed())
        .map(|f| match f.done {
            Some(done) => json!({"done": dart_iso(done)}),
            None => json!({}),
        })
        .collect();
    if !frogs.is_empty() {
        props.push(("frogs", Value::Array(frogs)));
    }
    Ok((props, drawn))
}

/// A Note's file's name without `.md` and its link from the Note being
/// written, by the Note's id; `None` for a Note not in the vault.
type PoolLink<'p> = dyn Fn(&str) -> Option<(String, String)> + 'p;

/// `text` escaped for a table's cell.
fn cell(text: &str) -> String {
    markdown::escape(&text.replace(['\n', '\r'], " "))
}

/// The properties of `note`'s kind, and the Markdown drawn of it: a
/// Checklist's or Shopping List's Lines as a task list, a Tally's
/// fields and entries as a table, a Session's pool as links.
fn kind_properties(
    store: &Store,
    note: &Note,
    links: &PoolLink,
    props: &mut Vec<(&'static str, Value)>,
) -> Result<Option<String>> {
    let id = &note.id;
    match NoteKind::of(store, id)? {
        NoteKind::Plain => Ok(None),
        NoteKind::Checklist => {
            props.push(("kind", json!("checklist")));
            let lines = Checklists::new(store)
                .by_note(id)?
                .map(|c| c.lines)
                .unwrap_or_default();
            let mut list = Vec::new();
            let mut drawn = Vec::new();
            for l in &lines {
                let mut out = Map::new();
                out.insert("id".into(), json!(l.id));
                out.insert("text".into(), json!(l.text));
                if l.in_group() {
                    out.insert("indent".into(), json!(true));
                }
                if l.checked
                    && let Some(at) = l.checked_at
                {
                    out.insert("checked".into(), json!(dart_iso(at)));
                }
                list.push(Value::Object(out));
                let indent = if l.in_group() { "  " } else { "" };
                let box_ = if l.checked { "[x]" } else { "[ ]" };
                drawn.push(format!("{indent}- {box_} {}", cell(&l.text)));
            }
            props.push(("lines", Value::Array(list)));
            // The own choice, where it is not the default.
            let shows = Checklists::new(store).on_agenda(id)?;
            if shows != ListOnAgenda::Lines {
                props.push(("agenda", json!(shows.code())));
            }
            Ok(Some(drawn.join("\n")))
        }
        NoteKind::ShoppingList => {
            props.push(("kind", json!("shopping")));
            let utc = |at: DateTime<Utc>| at.naive_utc();
            let Some(list) = ShoppingLists::new(store).by_note(id, &utc)? else {
                props.push(("lines", json!([])));
                return Ok(Some(String::new()));
            };
            let mut lines: Vec<_> = list.lines.iter().chain(list.gone()).collect();
            lines.sort_by(|a, b| (&a.order, &a.id).cmp(&(&b.order, &b.id)));
            let mut out = Vec::new();
            for l in &lines {
                let mut line = Map::new();
                line.insert("id".into(), json!(l.id));
                line.insert("text".into(), json!(l.text));
                line.insert("reappear".into(), json!(l.reappear.code()));
                if let Some(at) = l.checked_at {
                    line.insert("checked".into(), json!(dart_iso(at)));
                }
                out.push(Value::Object(line));
            }
            props.push(("lines", Value::Array(out)));
            // As it shows here: a Device's default may differ.
            let shows = ShoppingLists::new(store).on_agenda(id)?;
            props.push(("agenda", json!(shows.code())));
            let drawn: Vec<String> = list
                .lines
                .iter()
                .map(|l| {
                    let box_ = if l.struck { "[x]" } else { "[ ]" };
                    format!("- {box_} {}", cell(&l.text))
                })
                .collect();
            Ok(Some(drawn.join("\n")))
        }
        NoteKind::Tally => {
            props.push(("kind", json!("tally")));
            let tallies = Tallies::new(store);
            let Some(tally) = tallies.by_note(id)? else {
                return Ok(None);
            };
            let mut entries = tallies.entries(id, &PersonFilter::default())?;
            entries.extend(tallies.removed_entries(id)?);
            entries.sort_by(|a, b| (a.at, &a.id).cmp(&(b.at, &b.id)));
            let fields: Vec<Value> = tally
                .fields
                .iter()
                .map(|f| {
                    let mut out = Map::new();
                    out.insert("id".into(), json!(f.id));
                    out.insert("name".into(), json!(f.name));
                    out.insert("unit".into(), json!(f.unit));
                    out.insert("rule".into(), json!(f.rule.name()));
                    if let Some(target) = f.target.and_then(encode_number) {
                        out.insert("target".into(), json!(target));
                    }
                    if f.hidden {
                        out.insert("hidden".into(), json!(true));
                    }
                    if f.check {
                        out.insert("check".into(), json!(true));
                    }
                    if let Some(m) = &f.multiplies {
                        out.insert("multiplies".into(), json!(m));
                    }
                    Value::Object(out)
                })
                .collect();
            let numbers = |values: &std::collections::BTreeMap<String, f64>| -> Value {
                Value::Object(
                    values
                        .iter()
                        .filter_map(|(k, v)| Some((k.clone(), json!(encode_number(*v)?))))
                        .collect(),
                )
            };
            let list: Vec<Value> = entries
                .iter()
                .map(|e| {
                    let mut out = Map::new();
                    out.insert("id".into(), json!(e.id));
                    out.insert("at".into(), json!(dart_iso(e.at)));
                    out.insert("values".into(), numbers(&e.values));
                    if !e.more.is_empty() {
                        out.insert(
                            "sets".into(),
                            Value::Array(e.more.iter().map(numbers).collect()),
                        );
                    }
                    if !e.line.is_empty() {
                        out.insert("line".into(), json!(e.line));
                    }
                    if let Some(at) = e.trashed {
                        out.insert("trashed".into(), json!(dart_iso(at)));
                    }
                    Value::Object(out)
                })
                .collect();
            let mut t = Map::new();
            if tally.bar != TallyPeriod::Day {
                t.insert("bar".into(), json!(tally.bar.name()));
            }
            t.insert("sets".into(), json!(tally.counts_in_sets));
            t.insert("fields".into(), Value::Array(fields));
            t.insert("entries".into(), Value::Array(list));
            props.push(("tally", Value::Object(t)));
            Ok(Some(tally_table(&tally, &entries)))
        }
        NoteKind::Session => {
            props.push(("kind", json!("session")));
            let sessions = Sessions::new(store);
            let Some(setup) = sessions.setup_of(id)? else {
                return Ok(None);
            };
            let pool = sessions.pool(&setup)?;
            let counts: Map<String, Value> = setup
                .counts
                .iter()
                .map(|(d, c)| (d.to_string(), json!(c)))
                .collect();
            props.push((
                "session",
                json!({
                    "pool": pool,
                    "count": setup.count,
                    "counts": counts,
                    "rule": setup.rule.name(),
                }),
            ));
            let drawn: Vec<String> = pool
                .iter()
                .filter_map(|t| links(t))
                .map(|(name, link)| format!("- [{}]({link})", markdown::escape(&name)))
                .collect();
            Ok(Some(drawn.join("\n")))
        }
    }
}

/// A Tally's entries that count as a table: when, each field shown, and
/// the entry's line; a Check field as ✓, further sets after a `/`.
fn tally_table(tally: &crate::tallies::Tally, entries: &[TallyEntry]) -> String {
    let shown = tally.shown();
    let mut head = String::from("| |");
    let mut rule = String::from("| --- |");
    for f in &shown {
        let name = if f.unit.is_empty() {
            f.name.clone()
        } else {
            format!("{} ({})", f.name, f.unit)
        };
        head.push_str(&format!(" {} |", cell(&name)));
        rule.push_str(" --- |");
    }
    head.push_str(" |");
    rule.push_str(" --- |");
    let mut rows = vec![head, rule];
    for e in entries.iter().filter(|e| !e.is_trashed()) {
        let mut row = format!("| {} |", dart_iso(e.at));
        for f in &shown {
            let value = |values: &std::collections::BTreeMap<String, f64>| {
                values.get(&f.id).map(|v| {
                    if f.check {
                        if *v > 0.0 {
                            "✓".to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        encode_number(*v).unwrap_or_default()
                    }
                })
            };
            let all: Vec<String> = std::iter::once(&e.values)
                .chain(&e.more)
                .filter_map(value)
                .collect();
            row.push_str(&format!(" {} |", cell(&all.join(" / "))));
        }
        row.push_str(&format!(" {} |", cell(&e.line)));
        rows.push(row);
    }
    rows.join("\n")
}

// -------------------------------------------------------------- rebuild

/// A Note's file read back with Old Brain Fix's properties.
struct Ours {
    dir: Vec<String>,
    file: String,
    props: Map<String, Value>,
    text: String,
    /// Each Section's properties and text, in their order.
    sections: Vec<(Map<String, Value>, String)>,
}

/// A file made elsewhere, read as an Import reads one.
struct Elsewhere {
    dir: Vec<String>,
    file: String,
    text: String,
    format: &'static str,
    captured: DateTime<Utc>,
}

enum Read {
    Ours(Box<Ours>),
    Elsewhere(Elsewhere),
    Skipped,
}

/// A photo, recording or file of a rebuilt Note: its kind, the hash it
/// is kept under now when its file was found, the hash it had, its text,
/// and a file's name, type and pages.
struct Held {
    kind: &'static str,
    hash: Option<String>,
    size: u64,
    old: String,
    text: Option<String>,
    removed: bool,
    name: String,
    media_type: String,
    pages: Option<u64>,
}

/// Rebuilds the vault in `folder` into the empty log of `store`: a file
/// with Old Brain Fix's properties comes back with them, under new ids
/// in new Areas; one made elsewhere becomes a Note with its text, the
/// tags of its front matter and its file's date, in the Area of its
/// first folder and the Topic of its second, one at the top in the Area
/// `root_area`. Hidden files and folders and the attachments folders are
/// passed over. The files of the Notes are kept first, one at a time,
/// then the log is written in one transaction. `progress` hears how many
/// files of how many were read.
pub fn rebuild(
    store: &Store,
    folder: &Path,
    root_area: &str,
    progress: &dyn Fn(usize, usize),
) -> Result<Rebuilt> {
    if store.current_seq()? != 0 {
        return Err(problem(VaultProblem::NotEmpty));
    }
    let root = folder.canonicalize()?;
    let mut found = Vec::new();
    walk(&root, &[], &mut found)?;
    let now = store.now().trunc_subsecs(6);
    let mut read = Vec::new();
    for (i, (dir, file, path)) in found.iter().enumerate() {
        read.push(read_file(path, dir, file, now)?);
        progress(i + 1, found.len());
    }
    let skipped = read.iter().filter(|r| matches!(r, Read::Skipped)).count();
    let mut ours: Vec<Ours> = Vec::new();
    let mut elsewhere: Vec<Elsewhere> = Vec::new();
    for r in read {
        match r {
            Read::Ours(o) => ours.push(*o),
            Read::Elsewhere(e) => elsewhere.push(e),
            Read::Skipped => {}
        }
    }
    if ours.is_empty() && elsewhere.is_empty() {
        return Err(problem(VaultProblem::NotAVault));
    }
    let mut ids = Ids::default();
    for o in &ours {
        ids.plan(store, o)?;
    }
    let mut paths: HashMap<String, String> = HashMap::new();
    for (k, o) in ours.iter().enumerate() {
        if let Some(new) = ids.notes.get(k) {
            paths.insert(path_key(&o.dir, &o.file), new.clone());
        }
    }
    let mut foreign_ids = Vec::new();
    for e in &elsewhere {
        let id = format!("note:{}", store.new_id()?);
        paths.insert(path_key(&e.dir, &e.file), id.clone());
        foreign_ids.push(id);
    }
    let lower_paths: HashMap<String, String> = paths
        .iter()
        .map(|(k, v)| (lower_case(k), v.clone()))
        .collect();
    // The files are kept before the log names them, one at a time.
    let mut held: Vec<Vec<Vec<Held>>> = Vec::new();
    let (mut files, mut missing) = (0, 0);
    for o in &ours {
        let mut lists = vec![held_of(store, &root, &o.dir, o.props.get("attachments"))?];
        for (s, _) in &o.sections {
            lists.push(held_of(store, &root, &o.dir, s.get("attachments"))?);
        }
        for h in lists.iter().flatten() {
            if h.hash.is_some() {
                files += 1;
            } else if !h.removed {
                missing += 1;
            }
        }
        held.push(lists);
    }
    let made = store.batch(|| {
        if store.current_seq()? != 0 {
            return Err(problem(VaultProblem::NotEmpty));
        }
        let mut places = Made::default();
        for (k, o) in ours.iter().enumerate() {
            let note = ids.notes.get(k).cloned().unwrap_or_default();
            let place = places.of_ours(store, &o.props)?;
            let relink = Relink {
                ids: &ids,
                local: ids.local.get(k),
                places: &places,
                paths: &paths,
                lower_paths: &lower_paths,
                dir: &o.dir,
            };
            let lists = held.get(k).map(Vec::as_slice).unwrap_or_default();
            write_ours(store, o, &note, place.as_deref(), &relink, lists, now)?;
        }
        for (e, id) in elsewhere.iter().zip(&foreign_ids) {
            let area = e.dir.first().map_or(root_area, String::as_str);
            let place = places.by_name(store, area, e.dir.get(1).map(String::as_str))?;
            let relink = Relink {
                ids: &ids,
                local: None,
                places: &places,
                paths: &paths,
                lower_paths: &lower_paths,
                dir: &e.dir,
            };
            write_elsewhere(store, e, id, &place, &relink)?;
        }
        for area in &places.hidden {
            store.set_hidden_from_ai(area, true)?;
        }
        Ok(())
    });
    made?;
    Ok(Rebuilt {
        notes: ours.len() + elsewhere.len(),
        elsewhere: elsewhere.len(),
        files,
        missing,
        skipped,
    })
}

/// The key a Note's file is found by from a link: its folders and its
/// name joined by `/`.
fn path_key(dir: &[String], file: &str) -> String {
    let mut parts: Vec<&str> = dir.iter().map(String::as_str).collect();
    parts.push(file);
    parts.join("/")
}

/// Every Markdown and text file in `dir` and its folders, by its folders
/// under the vault, its name and its path: no hidden file or folder, no
/// attachments folder, no link followed.
fn walk(
    dir: &Path,
    under: &[String],
    found: &mut Vec<(Vec<String>, String, PathBuf)>,
) -> Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if !name.eq_ignore_ascii_case(ATTACHMENTS) {
                let mut inner = under.to_vec();
                inner.push(name);
                walk(&entry.path(), &inner, found)?;
            }
        } else if kind.is_file() && imports::format_of(&name).is_some() {
            found.push((under.to_vec(), name, entry.path()));
        }
    }
    Ok(())
}

/// The file at `path` read as a Note: with Old Brain Fix's properties
/// when it has them, all readable, of this format; else as made
/// elsewhere. An error for a file of a later format.
fn read_file(path: &Path, dir: &[String], file: &str, now: DateTime<Utc>) -> Result<Read> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_NOTE_FILE {
        return Ok(Read::Skipped);
    }
    let bytes = std::fs::read(path)?;
    let utf8 = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    if let Ok(text) = std::str::from_utf8(utf8) {
        let text = if text.starts_with("---\r\n") {
            std::borrow::Cow::Owned(text.replace("\r\n", "\n"))
        } else {
            std::borrow::Cow::Borrowed(text)
        };
        if let Some((props, body)) = front_matter(&text)
            && props.iter().any(|(k, _)| k == MARK)
        {
            if let Some((_, Some(Value::Number(n)))) = props.iter().find(|(k, _)| k == MARK)
                && n.as_i64().is_some_and(|v| v > FORMAT)
            {
                return Err(problem(VaultProblem::NeedsUpdate));
            }
            if let Some(ours) = ours(dir, file, &props, body) {
                return Ok(Read::Ours(Box::new(ours)));
            }
        }
    }
    let modified = meta.modified().ok().map(DateTime::<Utc>::from);
    Ok(match elsewhere(dir, file, &bytes, modified, now) {
        Some(e) => Read::Elsewhere(e),
        None => Read::Skipped,
    })
}

/// The Note of a file with Old Brain Fix's properties `props` and the
/// `body` after them; `None` when a property cannot be read, or the id,
/// the capture time or the format is missing.
fn ours(dir: &[String], file: &str, props: &[(String, Option<Value>)], body: &str) -> Option<Ours> {
    let mut map = Map::new();
    for (key, value) in props {
        map.insert(key.clone(), value.clone()?);
    }
    if map.get(MARK).and_then(Value::as_i64) != Some(FORMAT) {
        return None;
    }
    let id = map.get("id").and_then(Value::as_str)?;
    if !id.starts_with("note:") {
        return None;
    }
    parse_time(map.get("captured").and_then(Value::as_str)?).ok()?;
    map.get("format").and_then(Value::as_str)?;
    let section_props: Vec<Map<String, Value>> = match map.get("sections") {
        None => Vec::new(),
        Some(Value::Array(list)) => list
            .iter()
            .map(|s| s.as_object().cloned())
            .collect::<Option<Vec<_>>>()?,
        Some(_) => return None,
    };
    let pairs: Vec<(&str, Value)> = map.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    let mut rest = body.strip_suffix('\n').unwrap_or(body);
    if shows(&pairs) {
        let marker = format!("\n\n{SHOWN}\n");
        if let Some(at) = rest.rfind(&marker) {
            rest = rest.get(..at).unwrap_or(rest);
        }
    }
    let mut sections: Vec<(Map<String, Value>, String)> = Vec::new();
    for s in section_props.iter().rev() {
        let Some(sid) = s.get("id").and_then(Value::as_str) else {
            continue;
        };
        let marker = format!("\n\n{SECTION}{sid}{MARKER_END}\n");
        let Some(at) = rest.rfind(&marker) else {
            continue;
        };
        let after = rest.get(at + marker.len()..).unwrap_or_default();
        // The date line, then the blank line, as written.
        let after = after.split_once('\n').map_or("", |(_, a)| a);
        let after = after.strip_prefix('\n').unwrap_or(after);
        sections.push((s.clone(), after.to_string()));
        rest = rest.get(..at).unwrap_or_default();
    }
    sections.reverse();
    Some(Ours {
        dir: dir.to_vec(),
        file: file.to_string(),
        props: map,
        text: rest.to_string(),
        sections,
    })
}

/// A file made elsewhere as an Import reads it (ADR 0036): UTF-8 or
/// Windows-1252, under its name unless it begins with it, the tags of its
/// front matter on a last line, dated by its file; `None` when it is too
/// large or no text.
fn elsewhere(
    dir: &[String],
    file: &str,
    bytes: &[u8],
    modified: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<Elsewhere> {
    let (stem, format) = imports::format_of(file)?;
    if bytes.len() > imports::MAX_FILE_BYTES {
        return None;
    }
    let read = imports::decode(bytes)?;
    let mut text = imports::titled(stem, &read, format);
    let tags = front_matter_tags(&read, &text);
    if !tags.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&tags);
    }
    if text.is_empty()
        || StoredText::of(format, &text, VERSION).encode().len() > imports::MAX_TEXT_BYTES
    {
        return None;
    }
    let captured = match modified.map(|at| at.trunc_subsecs(6)) {
        Some(at) if at > DateTime::UNIX_EPOCH && at <= now => at,
        _ => now,
    };
    Some(Elsewhere {
        dir: dir.to_vec(),
        file: file.to_string(),
        text,
        format,
        captured,
    })
}

/// The tags another tool keeps in the front matter of `text` (`tags:` as
/// a list in brackets, a list of lines under it, or words), as tags of
/// the text ([`text_tags::from_label`], each level of a nested tag on its
/// own), those `body` holds already left out; apart by spaces.
fn front_matter_tags(text: &str, body: &str) -> String {
    let Some(rest) = text.strip_prefix("---\n") else {
        return String::new();
    };
    let Some(end) = rest.find("\n---") else {
        return String::new();
    };
    let head = rest.get(..end).unwrap_or_default();
    let mut labels: Vec<String> = Vec::new();
    let mut lines = head.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(value) = line
            .strip_prefix("tags:")
            .or_else(|| line.strip_prefix("tag:"))
        else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            while let Some(item) = lines.peek().and_then(|l| l.trim_start().strip_prefix('-')) {
                labels.push(item.trim().to_string());
                lines.next();
            }
        } else {
            match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                Some(inner) => labels.extend(inner.split(',').map(str::to_string)),
                None if value.contains(',') => {
                    labels.extend(value.split(',').map(str::to_string));
                }
                None => labels.extend(value.split_whitespace().map(str::to_string)),
            }
        }
    }
    let mut seen: HashSet<String> = text_tags::find(body).into_iter().map(|t| t.tag).collect();
    let mut tags = Vec::new();
    for label in labels {
        let label = label
            .trim()
            .trim_matches(['"', '\''])
            .trim_start_matches('#');
        let parts: Option<Vec<String>> = label
            .split('/')
            .map(|p| text_tags::from_label(p).map(|t| t.trim_start_matches('#').to_string()))
            .collect();
        let Some(parts) = parts.filter(|p| !p.is_empty()) else {
            continue;
        };
        let tag = format!("#{}", parts.join("/"));
        if seen.insert(lower_case(&tag)) {
            tags.push(tag);
        }
    }
    tags.join(" ")
}

/// The new ids of what the files with Old Brain Fix's properties hold:
/// each Note's, and of the Sections, Lines, Tally fields and entries,
/// Routines and the list it holds, by the id it had. The first file
/// holding an id gives it; a later copy of that file keeps its own.
#[derive(Default)]
struct Ids {
    notes: Vec<String>,
    local: Vec<HashMap<String, String>>,
    all: HashMap<String, String>,
}

impl Ids {
    fn plan(&mut self, store: &Store, o: &Ours) -> Result<()> {
        let mut local: HashMap<String, String> = HashMap::new();
        let mut fresh = |old: &str, prefix: &str| -> Result<()> {
            local.insert(old.to_string(), format!("{prefix}{}", store.new_id()?));
            Ok(())
        };
        let note = o
            .props
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        fresh(note, "note:")?;
        for (s, _) in &o.sections {
            if let Some(id) = s.get("id").and_then(Value::as_str) {
                let prefix = if id.starts_with("note:") {
                    "note:"
                } else {
                    "section:"
                };
                fresh(id, prefix)?;
            }
        }
        for line in list(o.props.get("lines")) {
            if let Some(id) = line.get("id").and_then(Value::as_str) {
                fresh(id, checklist_keys::LINE_PREFIX)?;
            }
        }
        if let Some(tally) = o.props.get("tally") {
            for f in list(tally.get("fields")) {
                if let Some(id) = f.get("id").and_then(Value::as_str) {
                    fresh(id, "")?;
                }
            }
            for e in list(tally.get("entries")) {
                if let Some(id) = e.get("id").and_then(Value::as_str) {
                    fresh(id, tally_keys::ENTRY_PREFIX)?;
                }
            }
        }
        for r in list(o.props.get("routines")).chain(list(o.props.get("reminders"))) {
            if let Some(id) = r.get("id").and_then(Value::as_str) {
                fresh(id, "routine:")?;
            }
        }
        self.notes
            .push(local.get(note).cloned().unwrap_or_default());
        for (old, new) in &local {
            self.all.entry(old.clone()).or_insert_with(|| new.clone());
        }
        self.local.push(local);
        Ok(())
    }
}

/// The items of the JSON list `value`; none for anything else.
fn list(value: Option<&Value>) -> impl Iterator<Item = &Value> {
    value
        .and_then(Value::as_array)
        .map(|l| l.iter())
        .into_iter()
        .flatten()
}

/// The Areas and Topics a rebuild made: by the id they had, by name for
/// the files made elsewhere, and the hidden marks to set.
#[derive(Default)]
struct Made {
    areas: HashMap<String, String>,
    topics: HashMap<String, String>,
    area_names: HashMap<String, String>,
    topic_names: HashMap<(String, String), String>,
    hidden: Vec<String>,
}

impl Made {
    /// The new place of a Note with the properties `props`, made at the
    /// first Note that names it; `None` for one filed nowhere.
    fn of_ours(&mut self, store: &Store, props: &Map<String, Value>) -> Result<Option<String>> {
        let text = |key: &str| props.get(key).and_then(Value::as_str);
        let Some(name) = text("area").map(dart_trim).filter(|n| !n.is_empty()) else {
            return Ok(None);
        };
        let old = text("areaId").unwrap_or(name).to_string();
        let area = match self.areas.get(&old) {
            Some(id) => id.clone(),
            None => {
                let id = self.new_area(store, name, text("areaStarter"))?;
                if let Some(colour) = text("areaColour") {
                    store.append(&id, place_keys::COLOUR, Some(colour), None)?;
                }
                if let Some(icon) = text("areaIcon") {
                    store.append(&id, place_keys::ICON, Some(icon), None)?;
                }
                if props.get("areaHiddenFromAi") == Some(&json!(true)) {
                    self.hidden.push(id.clone());
                }
                self.areas.insert(old, id.clone());
                id
            }
        };
        let Some(topic) = text("topic").map(dart_trim).filter(|n| !n.is_empty()) else {
            return Ok(Some(area));
        };
        let old = text("topicId").map_or_else(|| format!("{area}/{topic}"), str::to_string);
        if let Some(id) = self.topics.get(&old) {
            return Ok(Some(id.clone()));
        }
        let id = self.new_topic(store, &area, topic, text("topicStarter"))?;
        self.topics.insert(old, id.clone());
        Ok(Some(id))
    }

    /// The place of a file made elsewhere in the folder `area` and, when
    /// it lies deeper, `topic`: an Area of that name a rebuild made, else
    /// a new one; the same for its Topic.
    fn by_name(&mut self, store: &Store, area: &str, topic: Option<&str>) -> Result<String> {
        let area = match self.area_names.get(area) {
            Some(id) => id.clone(),
            None => self.new_area(store, area, None)?,
        };
        let Some(topic) = topic else {
            return Ok(area);
        };
        match self.topic_names.get(&(area.clone(), topic.to_string())) {
            Some(id) => Ok(id.clone()),
            None => self.new_topic(store, &area, topic, None),
        }
    }

    fn new_area(&mut self, store: &Store, name: &str, starter: Option<&str>) -> Result<String> {
        let id = format!("area:{}", store.new_id()?);
        let name = dart_trim(name);
        store.append(&id, place_keys::TYPE, Some(place_keys::TYPE_AREA), None)?;
        store.append(&id, place_keys::NAME, Some(name), None)?;
        if let Some(starter) = starter {
            store.append(&id, place_keys::STARTER, Some(starter), None)?;
        }
        self.area_names
            .entry(name.to_string())
            .or_insert_with(|| id.clone());
        Ok(id)
    }

    fn new_topic(
        &mut self,
        store: &Store,
        area: &str,
        name: &str,
        starter: Option<&str>,
    ) -> Result<String> {
        let id = format!("topic:{}", store.new_id()?);
        let name = dart_trim(name);
        store.append(&id, place_keys::TYPE, Some(place_keys::TYPE_TOPIC), None)?;
        store.append(&id, place_keys::AREA, Some(area), None)?;
        store.append(&id, place_keys::NAME, Some(name), None)?;
        if let Some(starter) = starter {
            store.append(&id, place_keys::STARTER, Some(starter), None)?;
        }
        self.topic_names
            .entry((area.to_string(), name.to_string()))
            .or_insert_with(|| id.clone());
        Ok(id)
    }
}

/// What a rebuilt text's links and ids point at now.
struct Relink<'a> {
    ids: &'a Ids,
    local: Option<&'a HashMap<String, String>>,
    places: &'a Made,
    paths: &'a HashMap<String, String>,
    lower_paths: &'a HashMap<String, String>,
    dir: &'a [String],
}

impl Relink<'_> {
    /// The new id of what had the id `old`; `None` for one not rebuilt.
    fn id(&self, old: &str) -> Option<String> {
        self.local
            .and_then(|l| l.get(old))
            .or_else(|| self.ids.all.get(old))
            .or_else(|| self.places.areas.get(old))
            .or_else(|| self.places.topics.get(old))
            .cloned()
    }

    /// The new destination of a link to `url`: a Note's or place's app
    /// link by its new id, a relative link to a Note's file of the vault
    /// as an app link to it; `None` for any other.
    fn link(&self, url: &str) -> Option<String> {
        match app_link::parse(url) {
            Some(AppLink::Note(old)) => return Some(AppLink::Note(self.id(&old)?).url()),
            Some(AppLink::Place(old)) => return Some(AppLink::Place(self.id(&old)?).url()),
            Some(_) => return None,
            None => {}
        }
        let path = url.split('#').next().unwrap_or(url);
        if path.contains(':') || path.starts_with('/') {
            return None;
        }
        let lower = path.to_ascii_lowercase();
        if !(lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".txt")) {
            return None;
        }
        let mut parts: Vec<String> = self.dir.to_vec();
        for part in percent_decoded(path)?.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop()?;
                }
                name => parts.push(name.to_string()),
            }
        }
        let key = parts.join("/");
        let id = self
            .paths
            .get(&key)
            .or_else(|| self.lower_paths.get(&lower_case(&key)))?;
        Some(AppLink::Note(id.clone()).url())
    }

    /// `text` in `format` with its links pointing at the rebuilt Notes.
    fn text(&self, text: &str, format: &str) -> String {
        if format == text_format::MARKDOWN {
            relinked(text, &|url| self.link(url))
        } else {
            text.to_string()
        }
    }
}

/// The files `list` names, those found inside the vault kept, by the
/// folder `dir` of the Note's file.
fn held_of(
    store: &Store,
    root: &Path,
    dir: &[String],
    list_value: Option<&Value>,
) -> Result<Vec<Held>> {
    let mut out = Vec::new();
    for a in list(list_value) {
        let kind = match a.get("kind").and_then(Value::as_str) {
            Some("photo") => "photo",
            Some("recording") => "audio",
            Some("file") => "file",
            _ => continue,
        };
        let Some(old) = a.get("hash").and_then(Value::as_str) else {
            continue;
        };
        let text = a.get("text").and_then(Value::as_str).map(str::to_string);
        let removed = a.get("removed") == Some(&json!(true));
        let name = notes::clean_name(a.get("name").and_then(Value::as_str).unwrap_or_default());
        let media_type =
            notes::clean_type(a.get("type").and_then(Value::as_str).unwrap_or_default());
        let pages = a.get("pages").and_then(Value::as_u64);
        let mut hash = None;
        let mut size = 0;
        if !removed
            && let Some(path) = a.get("path").and_then(Value::as_str)
            && let Some(file) = inside(root, dir, path)
            && std::fs::metadata(&file)
                .is_ok_and(|m| m.is_file() && m.len() <= notes::MAX_FILE_BYTES)
        {
            let bytes = std::fs::read(&file)?;
            size = bytes.len() as u64;
            hash = Some(match kind {
                "photo" => store.put_photo(&bytes)?,
                "audio" => store.put_audio(&bytes)?,
                _ => store.put_file(&bytes)?,
            });
        }
        out.push(Held {
            kind,
            hash,
            size,
            old: old.to_string(),
            text,
            removed,
            name,
            media_type,
            pages,
        });
    }
    Ok(out)
}

/// The file `path` names from a Note's file in the folder `dir` of the
/// vault `root`, when it lies inside the vault, links followed; `None`
/// for a path out of it, absolute, with `..`, `\`, `:` or a control
/// character.
fn inside(root: &Path, dir: &[String], path: &str) -> Option<PathBuf> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
    {
        return None;
    }
    let mut full = dir.iter().fold(root.to_path_buf(), |p, d| p.join(d));
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
        full.push(part);
    }
    let real = full.canonicalize().ok()?;
    real.starts_with(root).then_some(real)
}

/// When the moment `value` says, not after `now`; `None` for none.
fn moment(value: Option<&Value>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let at = parse_time(value?.as_str()?).ok()?.trunc_subsecs(6);
    Some(at.min(now))
}

/// The stored text of `text` in `format` and `version`, its boxes given
/// new ids when this version writes that format (ADR 0035).
fn stored(store: &Store, text: &str, format: &str, version: i64) -> Result<String> {
    if version == VERSION && KNOWN.contains(&format) {
        let boxed = text_boxes::convert(text, format, || store.new_id())?;
        Ok(StoredText::of(format, &boxed, version).encode())
    } else {
        Ok(StoredText::of(format, text, version).encode())
    }
}

/// The key of a field by a blob's hash.
type KeyOf = fn(&str) -> String;

/// Writes the photos, recordings and files `held` onto the Note `id`,
/// dated `at`: each kept one under its new hash with its text; a removed
/// one, or one not found, with its text only, as removed while its text
/// stays (ADR 0020), when it has one.
fn write_held(store: &Store, id: &str, held: &[Held], at: DateTime<Utc>) -> Result<()> {
    let date = Some(at);
    let later = Some(at + chrono::Duration::microseconds(1));
    for h in held {
        let (field, text_field): (KeyOf, KeyOf) = match h.kind {
            "photo" => (note_keys::photo, note_keys::ocr),
            "audio" => (note_keys::audio, note_keys::transcript),
            _ => (note_keys::file, note_keys::file_text),
        };
        let value = |hash: &str, size: u64| {
            if h.kind == "file" {
                let kind = file_text::kind_of(&h.name, &h.media_type);
                Attachment {
                    hash: hash.to_string(),
                    name: h.name.clone(),
                    media_type: h.media_type.clone(),
                    size,
                    pages: h.pages,
                    computer: file_text::computer_reads(kind, size, h.pages),
                }
                .value()
            } else {
                note_keys::ADDED.to_string()
            }
        };
        match &h.hash {
            Some(hash) => {
                store.append(id, &field(hash), Some(&value(hash, h.size)), date)?;
                if let Some(text) = &h.text {
                    store.append(id, &text_field(hash), Some(text), date)?;
                }
            }
            None => {
                let Some(text) = &h.text else {
                    continue;
                };
                store.append(id, &field(&h.old), Some(&value(&h.old, 0)), date)?;
                store.append(id, &field(&h.old), None, later)?;
                store.append(id, &text_field(&h.old), Some(text), date)?;
            }
        }
    }
    Ok(())
}

/// Writes the Note of the file `o` as `note`, filed into `place`, with
/// its Sections, files, kind, Routines and Reminders, Frogs and mark,
/// each entry dated when it says but never after `now`.
fn write_ours(
    store: &Store,
    o: &Ours,
    note: &str,
    place: Option<&str>,
    relink: &Relink,
    held: &[Vec<Held>],
    now: DateTime<Utc>,
) -> Result<()> {
    let p = &o.props;
    let text_of = |key: &str| p.get(key).and_then(Value::as_str);
    let at = moment(p.get("captured"), now).unwrap_or(now);
    let date = Some(at);
    let format = text_of("format").unwrap_or(text_format::PLAIN);
    let version = p
        .get("textVersion")
        .and_then(Value::as_i64)
        .unwrap_or(VERSION);
    store.append(note, note_keys::TYPE, Some(note_keys::TYPE_NOTE), date)?;
    store.append(note, note_keys::CAPTURED, Some(&dart_iso(at)), date)?;
    let text = stored(store, &relink.text(&o.text, format), format, version)?;
    store.append(note, note_keys::TEXT, Some(&text), date)?;
    if let Some(place) = place {
        store.append(note, note_keys::PLACE, Some(place), date)?;
    }
    if let Some(importance) = text_of("importance") {
        let importance = notes::Importance::from_name(Some(importance));
        if importance != notes::Importance::Normal {
            store.append(note, note_keys::IMPORTANCE, Some(importance.name()), date)?;
        }
    }
    let routines: Vec<&Value> = list(p.get("routines"))
        .chain(list(p.get("reminders")))
        .collect();
    if p.get("spawned") == Some(&json!(true)) || !routines.is_empty() {
        store.append(note, note_keys::SPAWNED, Some(&dart_iso(at)), date)?;
    }
    write_held(
        store,
        note,
        held.first().map(Vec::as_slice).unwrap_or_default(),
        at,
    )?;
    for (k, (s, text)) in o.sections.iter().enumerate() {
        let Some(id) = s
            .get("id")
            .and_then(Value::as_str)
            .and_then(|old| relink.id(old))
        else {
            continue;
        };
        let s_at = moment(s.get("captured"), now).unwrap_or(at);
        let s_date = Some(s_at);
        let s_format = s
            .get("format")
            .and_then(Value::as_str)
            .unwrap_or(text_format::PLAIN);
        let s_version = s
            .get("textVersion")
            .and_then(Value::as_i64)
            .unwrap_or(VERSION);
        let s_text = stored(store, &relink.text(text, s_format), s_format, s_version)?;
        if s.get("merged") == Some(&json!(true)) {
            store.append(&id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), s_date)?;
            store.append(&id, note_keys::CAPTURED, Some(&dart_iso(s_at)), s_date)?;
            store.append(&id, note_keys::TEXT, Some(&s_text), s_date)?;
            let files = held.get(k + 1).map(Vec::as_slice).unwrap_or_default();
            write_held(store, &id, files, s_at)?;
            store.append(&id, note_keys::MERGED, Some(note), s_date)?;
        } else {
            store.append(&id, note_keys::TYPE, Some(note_keys::TYPE_SECTION), s_date)?;
            store.append(&id, note_keys::NOTE, Some(note), s_date)?;
            store.append(&id, note_keys::CAPTURED, Some(&dart_iso(s_at)), s_date)?;
            store.append(&id, note_keys::TEXT, Some(&s_text), s_date)?;
        }
    }
    write_kind(store, o, note, relink, at, now)?;
    for r in routines {
        write_routine(store, r, note, relink, at, now)?;
    }
    for f in list(p.get("frogs")) {
        let id = format!("{}{}", frog_keys::PREFIX, store.new_id()?);
        store.append(&id, frog_keys::TYPE, Some(frog_keys::TYPE_FROG), date)?;
        store.append(&id, frog_keys::NOTE, Some(note), date)?;
        if let Some(done) = moment(f.get("done"), now) {
            store.append(&id, frog_keys::DONE, Some(&dart_iso(done)), Some(done))?;
        }
    }
    if p.get("hiddenFromAi") == Some(&json!(true)) {
        store.set_hidden_from_ai(note, true)?;
    }
    Ok(())
}

/// Writes the kind of the Note `note` as the file `o` says: a
/// Checklist's or Shopping List's Lines, checked when they say; a
/// Tally's fields and entries; a Session's pool, counts and rule.
fn write_kind(
    store: &Store,
    o: &Ours,
    note: &str,
    relink: &Relink,
    at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<()> {
    let p = &o.props;
    let date = Some(at);
    match p.get("kind").and_then(Value::as_str) {
        Some(kind @ ("checklist" | "shopping")) => {
            let making = store.new_id()?;
            let shopping = kind == "shopping";
            let value = if shopping {
                shopping_keys::kind_of(&making)
            } else {
                checklist_keys::kind_of(&making)
            };
            store.append(note, checklist_keys::KIND, Some(&value), date)?;
            let mut key = String::new();
            for (j, line) in list(p.get("lines")).enumerate() {
                let text = line.get("text").and_then(Value::as_str).unwrap_or_default();
                let id = match line
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|old| relink.id(old))
                {
                    Some(id) => id,
                    None => format!("{}{}", checklist_keys::LINE_PREFIX, store.new_id()?),
                };
                key = order::after(&key);
                let indent = !shopping && j > 0 && line.get("indent") == Some(&json!(true));
                write_line_as(
                    store,
                    id.clone(),
                    note,
                    &making,
                    &NewLine {
                        text,
                        order: &key,
                        indent,
                        checked: false,
                        received: None,
                        at: date,
                    },
                )?;
                if shopping && let Some(reappear) = line.get("reappear").and_then(Value::as_str) {
                    store.append(&id, shopping_keys::REAPPEAR, Some(reappear), date)?;
                }
                if let Some(checked) = moment(line.get("checked"), now) {
                    let on = checked.max(at);
                    store.append(&id, checklist_keys::CHECKED, Some(&dart_iso(on)), Some(on))?;
                }
            }
            // The own choice how the list shows on the Agenda (ADR 0030).
            let shows = p
                .get("agenda")
                .and_then(Value::as_str)
                .and_then(ListOnAgenda::parse);
            match shows {
                Some(shows) if shopping => ShoppingLists::new(store).set_on_agenda(note, shows)?,
                Some(shows) => Checklists::new(store).set_on_agenda(note, shows)?,
                None => {}
            }
        }
        Some("tally") => {
            let Some(tally) = p.get("tally") else {
                return Ok(());
            };
            // Fields show in the order they were added, then by id: each a
            // moment after the one before, as the file lists them.
            for (i, f) in list(tally.get("fields")).enumerate() {
                let date = Some(at + chrono::Duration::microseconds(i as i64));
                let Some(field) = f
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|old| relink.id(old))
                else {
                    continue;
                };
                let key = |k: &str| tally_keys::field(&field, k);
                let text = |k: &str| f.get(k).and_then(Value::as_str);
                store.append(
                    note,
                    &key(tally_keys::NAME),
                    Some(text("name").unwrap_or_default()),
                    date,
                )?;
                store.append(
                    note,
                    &key(tally_keys::UNIT),
                    Some(text("unit").unwrap_or_default()),
                    date,
                )?;
                if let Some(rule) = text("rule") {
                    store.append(note, &key(tally_keys::RULE), Some(rule), date)?;
                }
                if let Some(target) = text("target") {
                    store.append(note, &key(tally_keys::TARGET), Some(target), date)?;
                }
                if f.get("hidden") == Some(&json!(true)) {
                    store.append(note, &key(tally_keys::HIDDEN), Some("true"), date)?;
                }
                if f.get("check") == Some(&json!(true)) {
                    store.append(
                        note,
                        &key(tally_keys::KIND),
                        Some(tally_keys::KIND_CHECK),
                        date,
                    )?;
                }
                if let Some(other) = text("multiplies").and_then(|old| relink.id(old)) {
                    store.append(note, &key(tally_keys::MULTIPLIES), Some(&other), date)?;
                }
            }
            if let Some(bar) = tally.get("bar").and_then(Value::as_str) {
                store.append(note, tally_keys::BAR, Some(bar), date)?;
            }
            let sets = tally.get("sets") == Some(&json!(true));
            store.append(
                note,
                tally_keys::SETS,
                Some(if sets { "true" } else { "false" }),
                date,
            )?;
            for e in list(tally.get("entries")) {
                let Some(id) = e
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|old| relink.id(old))
                else {
                    continue;
                };
                let e_at = moment(e.get("at"), now).unwrap_or(at);
                let e_on = e_at.max(at);
                let e_date = Some(e_on);
                store.append(&id, tally_keys::TYPE, Some(tally_keys::TYPE_ENTRY), e_date)?;
                store.append(&id, tally_keys::NOTE, Some(note), e_date)?;
                store.append(&id, tally_keys::AT, Some(&dart_iso(e_at)), e_date)?;
                let sets = std::iter::once(e.get("values")).chain(list(e.get("sets")).map(Some));
                for (i, set) in sets.enumerate() {
                    let Some(values) = set.and_then(Value::as_object) else {
                        continue;
                    };
                    for (old, v) in values {
                        let (Some(field), Some(v)) = (relink.id(old), v.as_str()) else {
                            continue;
                        };
                        store.append(
                            &id,
                            &tally_keys::set_value(i + 1, &field),
                            Some(v),
                            e_date,
                        )?;
                    }
                }
                if let Some(line) = e.get("line").and_then(Value::as_str) {
                    store.append(&id, tally_keys::LINE, Some(line), e_date)?;
                }
                if let Some(trashed) = moment(e.get("trashed"), now) {
                    let on = trashed.max(e_on);
                    store.append(&id, tally_keys::TRASHED, Some(&dart_iso(on)), Some(on))?;
                }
            }
        }
        Some("session") => {
            let Some(session) = p.get("session") else {
                return Ok(());
            };
            let pool: Vec<String> = list(session.get("pool"))
                .filter_map(Value::as_str)
                .filter_map(|old| relink.id(old))
                .collect();
            store.append(note, session_keys::POOL, Some(&pool.join(",")), date)?;
            if let Some(count) = session.get("count").and_then(Value::as_i64) {
                store.append(note, session_keys::COUNT, Some(&count.to_string()), date)?;
            }
            if let Some(counts) = session.get("counts").and_then(Value::as_object) {
                for (day, count) in counts {
                    let (Ok(day), Some(count)) = (day.parse::<u32>(), count.as_i64()) else {
                        continue;
                    };
                    if (1..=7).contains(&day) {
                        store.append(
                            note,
                            &session_keys::count_on(day),
                            Some(&count.to_string()),
                            date,
                        )?;
                    }
                }
            }
            if let Some(rule) = session.get("rule").and_then(Value::as_str) {
                store.append(note, session_keys::RULE, Some(rule), date)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Writes the Routine or Reminder `r` of the Note `note`: its schedule,
/// dated when it was last scheduled, its own Re-nag and escalation, and
/// its check-offs, skips, Line checks and a Session's picks and entries,
/// each by the new ids of what it names.
fn write_routine(
    store: &Store,
    r: &Value,
    note: &str,
    relink: &Relink,
    at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<()> {
    let Some(id) = r
        .get("id")
        .and_then(Value::as_str)
        .and_then(|old| relink.id(old))
    else {
        return Ok(());
    };
    let text = |k: &str| r.get(k).and_then(Value::as_str);
    let Some(start) = text(routine_keys::START) else {
        return Ok(());
    };
    let scheduled = moment(r.get("scheduled"), now).unwrap_or(at);
    let date = Some(scheduled);
    store.append(
        &id,
        routine_keys::TYPE,
        Some(routine_keys::TYPE_ROUTINE),
        date,
    )?;
    store.append(&id, routine_keys::NOTE, Some(note), date)?;
    store.append(&id, routine_keys::START, Some(start), date)?;
    store.append(
        &id,
        routine_keys::TIMES,
        Some(text(routine_keys::TIMES).unwrap_or_default()),
        date,
    )?;
    for key in [
        routine_keys::RULE,
        routine_keys::SCHEDULE_VERSION,
        routine_keys::RENAG,
        routine_keys::ESCALATION,
    ] {
        if let Some(value) = text(key) {
            store.append(&id, key, Some(value), date)?;
        }
    }
    let marks = |name: &str| {
        r.get(name)
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
    };
    let dated = |value: &str| {
        let head = value.split(' ').next().unwrap_or(value);
        parse_time(head)
            .ok()
            .map_or(scheduled, |t| t.trunc_subsecs(6).min(now))
    };
    for (key, value) in marks("done") {
        store.append(
            &id,
            &format!("{}{key}", routine_keys::DONE_PREFIX),
            Some(&value),
            Some(dated(&value)),
        )?;
    }
    for (key, value) in marks("skipped") {
        store.append(
            &id,
            &format!("{}{key}", routine_keys::SKIP_PREFIX),
            Some(&value),
            Some(dated(&value)),
        )?;
    }
    for (key, value) in marks("checks") {
        let Some((occurrence, line)) = key.split_once('@') else {
            continue;
        };
        let Some(line) = relink.id(line) else {
            continue;
        };
        let field = format!("{}{occurrence}@{line}", checklist_keys::CHECK_PREFIX);
        store.append(&id, &field, Some(&value), Some(dated(&value)))?;
    }
    for (key, value) in marks("picks") {
        let picked: Vec<String> = value.split(',').filter_map(|n| relink.id(n)).collect();
        let field = format!("{}{key}", session_keys::PICK_PREFIX);
        store.append(&id, &field, Some(&picked.join(",")), date)?;
    }
    for (key, value) in marks("entries") {
        let Some((occurrence, entry)) = key.split_once('@') else {
            continue;
        };
        let (Some(entry), Some(tally)) = (relink.id(entry), relink.id(&value)) else {
            continue;
        };
        let field = format!("{}{occurrence}@{entry}", session_keys::ENTRY_PREFIX);
        store.append(&id, &field, Some(&tally), date)?;
    }
    Ok(())
}

/// Writes the Note of a file made elsewhere as `id`, filed into `place`,
/// captured and dated when its file was last changed.
fn write_elsewhere(
    store: &Store,
    e: &Elsewhere,
    id: &str,
    place: &str,
    relink: &Relink,
) -> Result<()> {
    let date = Some(e.captured);
    store.append(id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), date)?;
    store.append(id, note_keys::CAPTURED, Some(&dart_iso(e.captured)), date)?;
    let text = stored(store, &relink.text(&e.text, e.format), e.format, VERSION)?;
    store.append(id, note_keys::TEXT, Some(&text), date)?;
    store.append(id, note_keys::PLACE, Some(place), date)?;
    Ok(())
}

#[cfg(test)]
mod tests;
