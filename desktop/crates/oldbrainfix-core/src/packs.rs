//! Packs (#164, ADR 0043), as the Dart core's `packs.dart` makes and
//! opens them: a file of copies of Notes picked from an Area, a Topic or
//! a Note, handed over through any messenger. The receiver keeps the
//! copies as their own, in new Areas and Topics; nothing links them
//! back, nothing updates or cancels them. A Pack is no Sent Item (one
//! Note, encrypted, owned by the sender) and no Share to an app.
//!
//! What a Note brings is what it is: its text and Sections, its kind
//! (the Lines of a Checklist unticked, a Shopping List's open with their
//! reappear times, a Tally's fields, a Session's pool, counts and rule),
//! its Routines and Reminders with their schedules, an open Frog, and
//! those of its photos, recordings, attached files (ADR 0047), Photo
//! Texts, Transcripts and Attachment Texts that are ticked. Never a Tally's entries, ticks, check-offs, its Capture
//! Context or its history.
//!
//! The file is one JSON object, keys sorted and no spaces, the same
//! bytes from both cores:
//! `{"pack":<payload>,"sig":<signature>,"type":"oldbrainfix-pack"}`.
//! `pack` is the payload as a string, `sig` is there only when the
//! payload carries the sender's Profile card ("Add my Profile"): the
//! signature of the Device that made the card over [`pack_bytes`] of the
//! payload. A later format writes `format`; only an absent one or the
//! whole number 1 is this version's (ADR 0034).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Timelike, Utc};
use serde_json::{Map, Value, json};

use crate::cards::check_profile_card;
use crate::checklists::{
    Checklists, NewLine, keys as checklist_keys, order as line_order, write_line,
};
use crate::entry::{Entry, dart_iso};
use crate::file_text;
use crate::frogs::{Frogs, keys as frog_keys};
use crate::note_kinds::NoteKind;
use crate::notes::{Attachment, Note, Notes, keys as note_keys};
use crate::person_settings::PersonSettings;
use crate::places::{Area, Places, keys as place_keys};
use crate::profile::Profile;
use crate::routines::{Routines, keys as routine_keys};
use crate::schedule::{Day, Schedule, WallClock};
use crate::sent::{baked, object, times};
use crate::sessions::{SessionRule, Sessions, keys as session_keys};
use crate::shopping_lists::{Reappear, ShoppingLists, keys as shopping_keys};
use crate::signing::{SigningKey, verify_signature};
use crate::store::{Fields, Store};
use crate::tallies::{Tallies, TallyPeriod, keys as tally_keys};
use crate::tally_units::{TallyRule, decode_number, encode_number};
use crate::text_format::{self, StoredText};
use crate::transcript_prompts::dart_trim;
use crate::{Error, Result};

/// The file ending of a Pack.
pub const EXTENSION: &str = "obfpack";

/// The type a Pack is shared as.
pub const MIME_TYPE: &str = "application/vnd.oldbrainfix.pack";

/// What a Pack file says it is.
const PACK_TYPE: &str = "oldbrainfix-pack";

/// The format of a Pack this version writes; never written out.
const FORMAT: i64 = 1;

/// Field keys a Pack leaves in the log.
pub mod keys {
    /// On a Note a Pack brought, kept with "Claim as mine" unticked:
    /// `{"id":<Profile id>,"name":<name>}`, the sender the "From `<name>`"
    /// badge names. Null once the receiver removed the badge.
    pub const FROM: &str = "packFrom";
}

/// The bytes a Pack's signature covers: the payload as sent.
pub fn pack_bytes(payload: &str) -> Vec<u8> {
    format!("oldbrainfix:pack:{payload}").into_bytes()
}

/// The name a Pack written on `day` goes by. It says nothing of what is
/// in it.
pub fn file_name(day: Day) -> String {
    format!(
        "oldbrainfix-pack-{}-{:02}-{:02}.{EXTENSION}",
        day.year, day.month, day.day
    )
}

/// Why a file was not opened as a Pack, the Dart core's exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackError {
    /// No Pack this app can read: no JSON, damaged, or carrying a Profile
    /// whose signature does not hold over what it holds, as when it was
    /// changed after it was signed.
    NotAPack,
    /// A Pack of a format a later version of the app writes (ADR 0034):
    /// this app needs an update to open it.
    NeedsUpdate,
    /// A Pack carrying the Profile of a person the user blocked: it is
    /// not opened (Q471).
    FromBlocked,
}

impl PackError {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            PackError::NotAPack => "NotAPack",
            PackError::NeedsUpdate => "PackNeedsUpdate",
            PackError::FromBlocked => "PackFromBlocked",
        }
    }
}

/// An Area of a Pack: its name and the names of its Topics the Pack's
/// Notes are filed into. Nothing else of the Area goes along (Q469).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackArea {
    pub name: String,
    pub topics: Vec<String>,
}

/// A Section of a Pack's Note: its text, the format it is in and the
/// version of its structure; not when it was captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSection {
    pub text: String,
    pub format: String,
    pub version: i64,
}

/// A Line of a Pack's Checklist or Shopping List: its text, whether it is
/// under the Line above (a Checklist's), and its reappear time (a
/// Shopping List's). It arrives unticked, open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackLine {
    pub text: String,
    pub indent: bool,
    pub reappear: Option<Reappear>,
}

/// A field of a Pack's Tally: as set up, without a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackField {
    pub name: String,
    pub unit: String,
    pub rule: TallyRule,
    /// The target as the log keeps it ([`encode_number`]), or none.
    pub target: Option<String>,
    pub hidden: bool,
    /// Whether it is a Check field (#264).
    pub check: bool,
    /// The index of the field among the Tally's fields this one
    /// multiplied (#200), or none.
    pub multiplies: Option<usize>,
}

/// A Session of a Pack's Note: its pool as the indices of the Pack's
/// Notes that are its Tallies, its counts and its rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSession {
    pub pool: Vec<usize>,
    pub count: i64,
    /// The count of a weekday (1 for Monday to 7 for Sunday) with its own.
    pub counts: BTreeMap<u32, i64>,
    pub rule: SessionRule,
}

/// A photo, a voice recording or an attached file of a Pack's Note: its
/// hash, whether its file goes along, its size then, and its Photo
/// Text, Transcript or Attachment Text; an attached file with its name,
/// media type and pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFile {
    pub hash: String,
    /// The file's bytes as the Pack carries them; none when the file
    /// stays out. On export they are read when the Pack is written.
    pub bytes: Option<Vec<u8>>,
    /// Whether the file goes along.
    pub has_file: bool,
    /// How many bytes the file has; 0 without it.
    pub size: usize,
    /// The Photo Text, Transcript or Attachment Text, or none when none
    /// goes along.
    pub text: Option<String>,
    /// An attached file's name and media type, as kept
    /// ([`crate::notes::clean_name`], [`crate::notes::clean_type`]);
    /// empty for a photo or a recording.
    pub name: String,
    pub media_type: String,
    /// How many pages an attached file has, when they were counted.
    pub pages: Option<u64>,
}

/// A Note of a Pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackNote {
    /// The index of its Area among the Pack's, none for none; and of its
    /// Topic among that Area's.
    pub area: Option<usize>,
    pub topic: Option<usize>,
    /// Whether a Session of the Pack brought it from outside what the
    /// Pack was made from: it shows under that Session.
    pub outside: bool,
    pub text: String,
    pub format: String,
    pub version: i64,
    pub sections: Vec<PackSection>,
    /// The Lines of its Checklist or Shopping List; none for neither.
    pub lines: Option<Vec<PackLine>>,
    pub shopping: bool,
    /// The fields of its Tally; empty for none.
    pub fields: Vec<PackField>,
    /// Its Tally's bar length (#264), none for a day as never set; and
    /// whether it counts in sets.
    pub bar: Option<TallyPeriod>,
    pub sets: bool,
    pub session: Option<PackSession>,
    /// Its Routines and Reminders, as their schedules.
    pub routines: Vec<Schedule>,
    /// Whether it carries an open Frog.
    pub frog: bool,
    pub photos: Vec<PackFile>,
    pub recordings: Vec<PackFile>,
    /// Its attached files (ADR 0047).
    pub attachments: Vec<PackFile>,
}

impl PackNote {
    /// The kind it is.
    pub fn kind(&self) -> NoteKind {
        if !self.fields.is_empty() {
            NoteKind::Tally
        } else if self.session.is_some() {
            NoteKind::Session
        } else if self.lines.is_none() {
            NoteKind::Plain
        } else if self.shopping {
            NoteKind::ShoppingList
        } else {
            NoteKind::Checklist
        }
    }
}

/// What a Pack holds: Areas, Notes, and the sender's Profile card when
/// it says who it is from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pack {
    pub areas: Vec<PackArea>,
    pub notes: Vec<PackNote>,
    /// The sender's Profile card, `<payload>.<signature>`, or none.
    pub card: Option<String>,
}

/// The keys of a Pack's items that are ticked.
pub type PackTicks = BTreeSet<String>;

/// What an item of a Pack's tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackItemKind {
    Area,
    Topic,
    Note,
    Photo,
    PhotoText,
    Recording,
    Transcript,
    /// An attached file (ADR 0047), and its Attachment Text.
    File,
    FileText,
    Routine,
    Reminder,
    Frog,
}

impl PackItemKind {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            PackItemKind::Area => "area",
            PackItemKind::Topic => "topic",
            PackItemKind::Note => "note",
            PackItemKind::Photo => "photo",
            PackItemKind::PhotoText => "photoText",
            PackItemKind::Recording => "recording",
            PackItemKind::Transcript => "transcript",
            PackItemKind::File => "file",
            PackItemKind::FileText => "fileText",
            PackItemKind::Routine => "routine",
            PackItemKind::Reminder => "reminder",
            PackItemKind::Frog => "frog",
        }
    }
}

/// One item of a Pack's tree, ticked or not (Q468). An Area or a Topic
/// only groups the items under it; a Note's photos, recordings, their
/// texts, Routines, Reminders and Frog lie under it, and the Tallies a
/// Session brought from outside lie under the Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackItem {
    /// The key it is ticked by ([`item_keys`]); `a<i>` and `a<i>.t<j>`
    /// for an Area and a Topic, which are never ticked.
    pub key: String,
    pub kind: PackItemKind,
    /// The Pack's Note it is or belongs to; none for an Area or a Topic.
    pub note: Option<usize>,
    /// The index of the photo, recording, Routine or Reminder.
    pub index: Option<usize>,
    /// How many bytes it adds to the Pack.
    pub size: usize,
    pub children: Vec<PackItem>,
}

impl PackItem {
    fn group(key: String, kind: PackItemKind, children: Vec<PackItem>) -> PackItem {
        PackItem {
            key,
            kind,
            note: None,
            index: None,
            size: 0,
            children,
        }
    }

    fn of(key: String, kind: PackItemKind, note: usize, index: Option<usize>, size: usize) -> Self {
        PackItem {
            key,
            kind,
            note: Some(note),
            index,
            size,
            children: Vec::new(),
        }
    }
}

/// The keys of Pack items: `n<i>` for the Pack's Note i, `n<i>.p<j>` and
/// `n<i>.pt<j>` for its photo j and that photo's text, `n<i>.a<j>` and
/// `n<i>.at<j>` for a recording and its Transcript, `n<i>.d<j>` and
/// `n<i>.dt<j>` for an attached file and its Attachment Text,
/// `n<i>.r<j>` for its Routine or Reminder j, `n<i>.f` for its Frog.
pub mod item_keys {
    pub fn note(i: usize) -> String {
        format!("n{i}")
    }

    pub fn photo(i: usize, j: usize) -> String {
        format!("n{i}.p{j}")
    }

    pub fn photo_text(i: usize, j: usize) -> String {
        format!("n{i}.pt{j}")
    }

    pub fn recording(i: usize, j: usize) -> String {
        format!("n{i}.a{j}")
    }

    pub fn transcript(i: usize, j: usize) -> String {
        format!("n{i}.at{j}")
    }

    pub fn file(i: usize, j: usize) -> String {
        format!("n{i}.d{j}")
    }

    pub fn file_text(i: usize, j: usize) -> String {
        format!("n{i}.dt{j}")
    }

    pub fn routine(i: usize, j: usize) -> String {
        format!("n{i}.r{j}")
    }

    pub fn frog(i: usize) -> String {
        format!("n{i}.f")
    }
}

/// How many characters `bytes` take as base64.
fn base64_length(bytes: usize) -> usize {
    bytes.div_ceil(3) * 4
}

impl Pack {
    /// The tree of the Pack (Q468, Q459): its Areas, each with the Notes
    /// filed into it alone and then its Topics with theirs, then the
    /// Notes of no Area; under each Note its photos, each with its Photo
    /// Text, its recordings, each with its Transcript, its Routines and
    /// Reminders and its Frog; under a Session the Tallies it brought
    /// from outside, and under a Tally its own items. A Note shows where
    /// its Area and Topic say; one brought from outside only under the
    /// Sessions that pick it.
    pub fn tree(&self) -> Vec<PackItem> {
        let shown: Vec<(usize, &PackNote)> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.outside)
            .collect();
        let mut out = Vec::new();
        for (a, area) in self.areas.iter().enumerate() {
            if !shown.iter().any(|(_, n)| n.area == Some(a)) {
                continue;
            }
            let mut children: Vec<PackItem> = shown
                .iter()
                .filter(|(_, n)| n.area == Some(a) && n.topic.is_none())
                .map(|&(i, n)| self.note_item(i, n, &HashSet::new()))
                .collect();
            for t in 0..area.topics.len() {
                let of_topic: Vec<PackItem> = shown
                    .iter()
                    .filter(|(_, n)| n.area == Some(a) && n.topic == Some(t))
                    .map(|&(i, n)| self.note_item(i, n, &HashSet::new()))
                    .collect();
                if !of_topic.is_empty() {
                    children.push(PackItem::group(
                        format!("a{a}.t{t}"),
                        PackItemKind::Topic,
                        of_topic,
                    ));
                }
            }
            out.push(PackItem::group(
                format!("a{a}"),
                PackItemKind::Area,
                children,
            ));
        }
        for (i, n) in shown {
            if n.area.is_none() {
                out.push(self.note_item(i, n, &HashSet::new()));
            }
        }
        out
    }

    /// The item of the Note `i`, `note`, with the items under it; a Tally
    /// its Session brought from outside under it unless `seen` holds it.
    fn note_item(&self, i: usize, note: &PackNote, seen: &HashSet<usize>) -> PackItem {
        let mut children = Vec::new();
        for (j, p) in note.photos.iter().enumerate() {
            if p.has_file {
                let key = item_keys::photo(i, j);
                let size = base64_length(p.size);
                children.push(PackItem::of(key, PackItemKind::Photo, i, Some(j), size));
            }
            if let Some(text) = &p.text {
                let key = item_keys::photo_text(i, j);
                children.push(PackItem::of(
                    key,
                    PackItemKind::PhotoText,
                    i,
                    Some(j),
                    text.len(),
                ));
            }
        }
        for (j, a) in note.recordings.iter().enumerate() {
            if a.has_file {
                let key = item_keys::recording(i, j);
                let size = base64_length(a.size);
                children.push(PackItem::of(key, PackItemKind::Recording, i, Some(j), size));
            }
            if let Some(text) = &a.text {
                let key = item_keys::transcript(i, j);
                children.push(PackItem::of(
                    key,
                    PackItemKind::Transcript,
                    i,
                    Some(j),
                    text.len(),
                ));
            }
        }
        for (j, d) in note.attachments.iter().enumerate() {
            if d.has_file {
                let key = item_keys::file(i, j);
                let size = base64_length(d.size);
                children.push(PackItem::of(key, PackItemKind::File, i, Some(j), size));
            }
            if let Some(text) = &d.text {
                let key = item_keys::file_text(i, j);
                children.push(PackItem::of(
                    key,
                    PackItemKind::FileText,
                    i,
                    Some(j),
                    text.len(),
                ));
            }
        }
        for (j, r) in note.routines.iter().enumerate() {
            let kind = if r.once() {
                PackItemKind::Reminder
            } else {
                PackItemKind::Routine
            };
            let size = routine_json(r).to_string().len();
            children.push(PackItem::of(
                item_keys::routine(i, j),
                kind,
                i,
                Some(j),
                size,
            ));
        }
        if note.frog {
            children.push(PackItem::of(
                item_keys::frog(i),
                PackItemKind::Frog,
                i,
                None,
                FROG_JSON.len(),
            ));
        }
        if let Some(session) = &note.session {
            for t in &session.pool {
                if let Some(tally) = self.notes.get(*t)
                    && tally.outside
                    && !seen.contains(t)
                {
                    let mut further = seen.clone();
                    further.insert(i);
                    children.push(self.note_item(*t, tally, &further));
                }
            }
        }
        PackItem {
            key: item_keys::note(i),
            kind: PackItemKind::Note,
            note: Some(i),
            index: None,
            size: note_json(note, None).map_or(0, |n| n.to_string().len()),
            children,
        }
    }

    /// The keys of every item that can be ticked: "Tick all".
    pub fn all_keys(&self) -> PackTicks {
        let mut out = PackTicks::new();
        for (i, note) in self.notes.iter().enumerate() {
            out.insert(item_keys::note(i));
            for (j, p) in note.photos.iter().enumerate() {
                if p.has_file {
                    out.insert(item_keys::photo(i, j));
                }
                if p.text.is_some() {
                    out.insert(item_keys::photo_text(i, j));
                }
            }
            for (j, a) in note.recordings.iter().enumerate() {
                if a.has_file {
                    out.insert(item_keys::recording(i, j));
                }
                if a.text.is_some() {
                    out.insert(item_keys::transcript(i, j));
                }
            }
            for (j, d) in note.attachments.iter().enumerate() {
                if d.has_file {
                    out.insert(item_keys::file(i, j));
                }
                if d.text.is_some() {
                    out.insert(item_keys::file_text(i, j));
                }
            }
            for j in 0..note.routines.len() {
                out.insert(item_keys::routine(i, j));
            }
            if note.frog {
                out.insert(item_keys::frog(i));
            }
        }
        out
    }

    /// What is ticked when an export starts (Q468): everything but the
    /// Reminders, the photos, the recordings and the attached files.
    pub fn export_ticks(&self) -> PackTicks {
        let mut out = PackTicks::new();
        for (i, note) in self.notes.iter().enumerate() {
            out.insert(item_keys::note(i));
            for (j, p) in note.photos.iter().enumerate() {
                if p.text.is_some() {
                    out.insert(item_keys::photo_text(i, j));
                }
            }
            for (j, a) in note.recordings.iter().enumerate() {
                if a.text.is_some() {
                    out.insert(item_keys::transcript(i, j));
                }
            }
            for (j, d) in note.attachments.iter().enumerate() {
                if d.text.is_some() {
                    out.insert(item_keys::file_text(i, j));
                }
            }
            for (j, r) in note.routines.iter().enumerate() {
                if !r.once() {
                    out.insert(item_keys::routine(i, j));
                }
            }
            if note.frog {
                out.insert(item_keys::frog(i));
            }
        }
        out
    }

    /// What is ticked when a Backup is opened to take Notes back from it
    /// (#285), as the Dart core's `backupTicks`: no Note, and everything
    /// under each, so a Note ticked comes back whole, its photos and
    /// recordings with it.
    pub fn backup_ticks(&self) -> PackTicks {
        let mut out = self.all_keys();
        for i in 0..self.notes.len() {
            out.remove(&item_keys::note(i));
        }
        out
    }

    /// `ticks` with the item `key` ticked `on`, as the Session rule says
    /// (Q458): ticking a Session ticks the Tallies of its pool; unticking
    /// a Tally takes it out of the pool of each Session, and a Session
    /// left with none of its pool ticked is unticked too.
    pub fn tick(&self, ticks: &PackTicks, key: &str, on: bool) -> PackTicks {
        let mut out = ticks.clone();
        let i = self.note_of(key);
        if on {
            out.insert(key.to_string());
            if let Some(session) = i
                .and_then(|i| self.notes.get(i))
                .and_then(|n| n.session.as_ref())
            {
                for t in &session.pool {
                    out.insert(item_keys::note(*t));
                }
            }
            return out;
        }
        out.remove(key);
        let Some(i) = i else {
            return out;
        };
        for (s, note) in self.notes.iter().enumerate() {
            let Some(session) = &note.session else {
                continue;
            };
            if !session.pool.contains(&i) {
                continue;
            }
            if !session
                .pool
                .iter()
                .any(|t| out.contains(&item_keys::note(*t)))
            {
                out.remove(&item_keys::note(s));
            }
        }
        out
    }

    /// The Note whose own item `key` is, `n<i>`; none for anything else.
    fn note_of(&self, key: &str) -> Option<usize> {
        let digits = key.strip_prefix('n')?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let i: usize = digits.parse().ok()?;
        (i < self.notes.len()).then_some(i)
    }

    /// How many bytes the Pack has with `ticks` (Q470): each ticked Note
    /// with its ticked items, as [`Pack::tree`] sizes them.
    pub fn size(&self, ticks: &PackTicks) -> usize {
        fn walk(
            item: &PackItem,
            counts: bool,
            ticks: &PackTicks,
            counted: &mut HashSet<String>,
            total: &mut usize,
        ) {
            // A Note under two Sessions counts once.
            if item.kind == PackItemKind::Note && !counted.insert(item.key.clone()) {
                return;
            }
            let ticked = match item.kind {
                PackItemKind::Area | PackItemKind::Topic => true,
                PackItemKind::Note => ticks.contains(&item.key),
                _ => counts && ticks.contains(&item.key),
            };
            if ticked {
                *total += item.size;
            }
            let under = if item.kind == PackItemKind::Note {
                ticked
            } else {
                counts
            };
            for child in &item.children {
                walk(child, under, ticks, counted, total);
            }
        }
        let mut total = 0;
        let mut counted = HashSet::new();
        for item in self.tree() {
            walk(&item, true, ticks, &mut counted, &mut total);
        }
        total
    }

    /// The Pack as `ticks` picks from it: the ticked Notes, each with its
    /// ticked items, a file without its text or a text without its file
    /// as ticked; a Session's pool only its ticked Tallies, and a Session
    /// with none left out (the core needs at least one); the Areas and
    /// Topics the Notes kept are filed into, in their order.
    pub fn picked(&self, ticks: &PackTicks) -> Pack {
        let ticked = |i: usize| ticks.contains(&item_keys::note(i));
        // A Session with none of its pool ticked goes neither.
        let kept: Vec<(usize, &PackNote)> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(i, _)| ticked(*i))
            .filter(|(_, n)| {
                n.session
                    .as_ref()
                    .is_none_or(|s| s.pool.iter().any(|t| ticked(*t)))
            })
            .collect();
        let new_index: HashMap<usize, usize> =
            kept.iter().enumerate().map(|(n, (i, _))| (*i, n)).collect();
        let mut area_index: HashMap<usize, usize> = HashMap::new();
        let mut topic_index: HashMap<(usize, usize), usize> = HashMap::new();
        let mut areas: Vec<PackArea> = Vec::new();
        for (_, note) in &kept {
            // Opening a Pack refuses a Note filed into an Area or Topic it
            // does not hold; were one here, it would go filed nowhere.
            let Some((a, area)) = note.area.and_then(|a| Some((a, self.areas.get(a)?))) else {
                continue;
            };
            let at = *area_index.entry(a).or_insert_with(|| {
                areas.push(PackArea {
                    name: area.name.clone(),
                    topics: Vec::new(),
                });
                areas.len() - 1
            });
            if let Some(t) = note.topic
                && let Some(topic) = area.topics.get(t)
                && let Some(kept_area) = areas.get_mut(at)
            {
                topic_index.entry((a, t)).or_insert_with(|| {
                    kept_area.topics.push(topic.clone());
                    kept_area.topics.len() - 1
                });
            }
        }
        let pooled: HashSet<usize> = kept
            .iter()
            .filter_map(|(_, n)| n.session.as_ref())
            .flat_map(|s| s.pool.iter().copied())
            .filter(|t| new_index.contains_key(t))
            .collect();
        let file = |f: &PackFile, with_file: bool, with_text: bool| -> Option<PackFile> {
            let has_file = with_file && f.has_file;
            let out = PackFile {
                hash: f.hash.clone(),
                bytes: if with_file { f.bytes.clone() } else { None },
                has_file,
                size: if has_file { f.size } else { 0 },
                text: if with_text { f.text.clone() } else { None },
                name: f.name.clone(),
                media_type: f.media_type.clone(),
                pages: f.pages,
            };
            (out.has_file || out.text.is_some()).then_some(out)
        };
        let notes = kept
            .iter()
            .map(|&(i, n)| PackNote {
                area: n.area.and_then(|a| area_index.get(&a).copied()),
                topic: match (n.area, n.topic) {
                    (Some(a), Some(t)) => topic_index.get(&(a, t)).copied(),
                    _ => None,
                },
                outside: n.outside && pooled.contains(&i),
                text: n.text.clone(),
                format: n.format.clone(),
                version: n.version,
                sections: n.sections.clone(),
                lines: n.lines.clone(),
                shopping: n.shopping,
                fields: n.fields.clone(),
                bar: n.bar,
                sets: n.sets,
                session: n.session.as_ref().map(|s| PackSession {
                    pool: s
                        .pool
                        .iter()
                        .filter_map(|t| new_index.get(t).copied())
                        .collect(),
                    count: s.count,
                    counts: s.counts.clone(),
                    rule: s.rule,
                }),
                routines: n
                    .routines
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| ticks.contains(&item_keys::routine(i, *j)))
                    .map(|(_, r)| r.clone())
                    .collect(),
                frog: n.frog && ticks.contains(&item_keys::frog(i)),
                photos: n
                    .photos
                    .iter()
                    .enumerate()
                    .filter_map(|(j, p)| {
                        file(
                            p,
                            ticks.contains(&item_keys::photo(i, j)),
                            ticks.contains(&item_keys::photo_text(i, j)),
                        )
                    })
                    .collect(),
                recordings: n
                    .recordings
                    .iter()
                    .enumerate()
                    .filter_map(|(j, a)| {
                        file(
                            a,
                            ticks.contains(&item_keys::recording(i, j)),
                            ticks.contains(&item_keys::transcript(i, j)),
                        )
                    })
                    .collect(),
                attachments: n
                    .attachments
                    .iter()
                    .enumerate()
                    .filter_map(|(j, d)| {
                        file(
                            d,
                            ticks.contains(&item_keys::file(i, j)),
                            ticks.contains(&item_keys::file_text(i, j)),
                        )
                    })
                    .collect(),
            })
            .collect();
        Pack {
            areas,
            notes,
            card: self.card.clone(),
        }
    }
}

// ------------------------------------------------------------ the bytes

const FROG_JSON: &str = "true";

/// Reads the bytes of a `kind` (`photo`, `audio` or `file`) file by its
/// hash.
pub type BytesOf<'a> = dyn Fn(&str, &str) -> Option<Vec<u8>> + 'a;

/// The size of the bytes of the `kind` (`photo`, `audio` or `file`) file
/// `hash` held, none for none.
pub type SizeOf<'a> = dyn Fn(&str, &str) -> Option<usize> + 'a;

/// [`Store::pack_of_everything`]'s Pack, as the Dart core's record.
#[derive(Debug, Clone)]
pub struct EverythingPack {
    pub pack: Pack,
    /// The indices of the Notes that lay in the trash.
    pub trashed: BTreeSet<usize>,
    /// The Areas of `closed` whose Notes stayed out.
    pub left_out: BTreeSet<String>,
}

fn routine_json(s: &Schedule) -> Value {
    let mut pairs = Vec::new();
    if let Some(rule) = &s.rule {
        pairs.push(("rule", json!(rule.format())));
    }
    pairs.push(("start", json!(s.start.to_string())));
    pairs.push(("times", json!(times(s))));
    object(pairs)
}

/// A Note as the payload holds it, keys sorted; with `files` its photos
/// and recordings, its Routines, Frog and whether it is from outside,
/// the bytes of its files read by `files` when the Pack holds none yet.
fn note_json(n: &PackNote, files: Option<&BytesOf>) -> Result<Value> {
    let files_json = |list: &[PackFile], kind: &str, bytes_of: &BytesOf| -> Result<Value> {
        let mut out = Vec::new();
        for f in list {
            let mut pairs = Vec::new();
            if f.has_file {
                let bytes = match &f.bytes {
                    Some(bytes) => bytes.clone(),
                    None => bytes_of(kind, &f.hash)
                        .ok_or_else(|| Error::Invalid(format!("no bytes of {kind} {}", f.hash)))?,
                };
                pairs.push(("bytes", json!(BASE64.encode(bytes))));
            }
            pairs.push(("hash", json!(f.hash)));
            if kind == "file" {
                pairs.push(("name", json!(f.name)));
                if let Some(pages) = f.pages {
                    pairs.push(("pages", json!(pages)));
                }
            }
            if let Some(text) = &f.text {
                pairs.push(("text", json!(text)));
            }
            if kind == "file" {
                pairs.push(("type", json!(f.media_type)));
            }
            out.push(object(pairs));
        }
        Ok(Value::Array(out))
    };
    let mut pairs = Vec::new();
    if let Some(area) = n.area {
        pairs.push(("area", json!(area)));
    }
    if let Some(bytes_of) = files
        && !n.attachments.is_empty()
    {
        pairs.push(("attachments", files_json(&n.attachments, "file", bytes_of)?));
    }
    if !n.fields.is_empty()
        && let Some(bar) = n.bar
    {
        pairs.push(("bar", json!(bar.name())));
    }
    if !n.fields.is_empty() {
        let fields = n
            .fields
            .iter()
            .map(|f| {
                let mut field = Vec::new();
                if f.check {
                    field.push(("check", json!(true)));
                }
                if f.hidden {
                    field.push(("hidden", json!(true)));
                }
                if let Some(m) = f.multiplies {
                    field.push(("multiplies", json!(m)));
                }
                field.push(("name", json!(f.name)));
                field.push(("rule", json!(f.rule.name())));
                if let Some(target) = &f.target {
                    field.push(("target", json!(target)));
                }
                field.push(("unit", json!(f.unit)));
                object(field)
            })
            .collect();
        pairs.push(("fields", Value::Array(fields)));
    }
    pairs.push(("format", json!(n.format)));
    if files.is_some() && n.frog {
        pairs.push(("frog", json!(true)));
    }
    if let Some(lines) = &n.lines {
        let lines = lines
            .iter()
            .map(|l| {
                let mut line = Vec::new();
                if l.indent {
                    line.push(("indent", json!(true)));
                }
                if let Some(r) = l.reappear {
                    line.push(("reappear", json!(r.code())));
                }
                line.push(("text", json!(l.text)));
                object(line)
            })
            .collect();
        pairs.push(("lines", Value::Array(lines)));
        let list = if n.shopping {
            shopping_keys::SHOPPING
        } else {
            checklist_keys::CHECKLIST
        };
        pairs.push(("list", json!(list)));
    }
    if let Some(bytes_of) = files {
        if n.outside {
            pairs.push(("outside", json!(true)));
        }
        if !n.photos.is_empty() {
            pairs.push(("photos", files_json(&n.photos, "photo", bytes_of)?));
        }
        if !n.recordings.is_empty() {
            pairs.push(("recordings", files_json(&n.recordings, "audio", bytes_of)?));
        }
        if !n.routines.is_empty() {
            let routines = n.routines.iter().map(routine_json).collect();
            pairs.push(("routines", Value::Array(routines)));
        }
    }
    if !n.sections.is_empty() {
        let sections = n
            .sections
            .iter()
            .map(|s| {
                let mut section = vec![("format", json!(s.format)), ("text", json!(s.text))];
                if s.version != text_format::VERSION {
                    section.push(("v", json!(s.version)));
                }
                object(section)
            })
            .collect();
        pairs.push(("sections", Value::Array(sections)));
    }
    if let Some(s) = &n.session {
        let mut session = vec![("count", json!(s.count))];
        if !s.counts.is_empty() {
            let mut counts = Map::new();
            for (d, c) in &s.counts {
                counts.insert(d.to_string(), json!(c));
            }
            session.push(("counts", Value::Object(counts)));
        }
        session.push(("pool", json!(s.pool)));
        session.push(("rule", json!(s.rule.name())));
        pairs.push(("session", object(session)));
    }
    if !n.fields.is_empty() && n.sets {
        pairs.push(("sets", json!(true)));
    }
    pairs.push(("text", json!(n.text)));
    if let Some(topic) = n.topic {
        pairs.push(("topic", json!(topic)));
    }
    if n.version != text_format::VERSION {
        pairs.push(("v", json!(n.version)));
    }
    Ok(object(pairs))
}

/// The payload of `pack`, keys sorted and no spaces.
fn payload(pack: &Pack, bytes_of: &BytesOf) -> Result<String> {
    let areas = pack
        .areas
        .iter()
        .map(|a| object(vec![("name", json!(a.name)), ("topics", json!(a.topics))]))
        .collect();
    let notes = pack
        .notes
        .iter()
        .map(|n| note_json(n, Some(bytes_of)))
        .collect::<Result<Vec<_>>>()?;
    let mut pairs = vec![
        ("areas", Value::Array(areas)),
        ("notes", Value::Array(notes)),
    ];
    if let Some(card) = &pack.card {
        pairs.push(("profile", json!(card)));
    }
    Ok(object(pairs).to_string())
}

/// The file of `pack`, as it is: signed by `key` when it carries a
/// Profile card, the card `key`'s Device made. The bytes of its files
/// are read with `bytes_of` where the Pack holds none.
pub fn encode_pack(pack: &Pack, key: Option<&SigningKey>, bytes_of: &BytesOf) -> Result<Vec<u8>> {
    let payload = payload(pack, bytes_of)?;
    let mut pairs = vec![("pack", json!(payload))];
    if pack.card.is_some() {
        let key = key.ok_or_else(|| Error::Invalid("a Pack with a Profile is signed".into()))?;
        pairs.push(("sig", json!(key.sign(&pack_bytes(&payload)))));
    }
    pairs.push(("type", json!(PACK_TYPE)));
    Ok(object(pairs).to_string().into_bytes())
}

/// The JSON object of the file `bytes` when it says it is a Pack, of any
/// format; none for anything else.
fn pack_json(bytes: &[u8]) -> Option<Map<String, Value>> {
    if !starts_like_json(bytes) {
        return None;
    }
    match serde_json::from_slice(bytes).ok()? {
        Value::Object(map) if map.get("type").and_then(Value::as_str) == Some(PACK_TYPE) => {
            Some(map)
        }
        _ => None,
    }
}

/// Whether the first byte of `bytes` that is no blank opens a JSON
/// object, as a Pack's file does: its first bytes are enough to tell.
pub fn starts_like_json(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .find(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .is_some_and(|b| *b == b'{')
}

/// Whether `bytes` are a Pack rather than a Sent Item, a Profile card
/// file or a bundle, in a format this version reads or a later one.
pub fn looks_like_pack(bytes: &[u8]) -> bool {
    pack_json(bytes).is_some()
}

// What follows reads a payload as the Dart core's `_parse` does: `None`
// is "no Pack". A value that is JSON `null` counts as absent, one of
// another type than asked is no Pack.

/// The value of `key` in `json`; none when absent or null.
fn present<'a>(json: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    json.get(key).filter(|v| !v.is_null())
}

/// The text of `key`: `Some(None)` when absent, `None` when no text.
fn optional_str<'a>(json: &'a Map<String, Value>, key: &str) -> Option<Option<&'a str>> {
    match present(json, key) {
        None => Some(None),
        Some(v) => v.as_str().map(Some),
    }
}

fn optional_bool(json: &Map<String, Value>, key: &str) -> Option<Option<bool>> {
    match present(json, key) {
        None => Some(None),
        Some(v) => v.as_bool().map(Some),
    }
}

/// The whole number of `key`, as Dart's `is int` takes it.
fn optional_int(json: &Map<String, Value>, key: &str) -> Option<Option<i64>> {
    match present(json, key) {
        None => Some(None),
        Some(v) => v.as_i64().map(Some),
    }
}

fn optional_list<'a>(json: &'a Map<String, Value>, key: &str) -> Option<&'a [Value]> {
    match present(json, key) {
        None => Some(&[]),
        Some(v) => v.as_array().map(Vec::as_slice),
    }
}

/// An index of `key`: none when absent, no Pack when negative.
fn index(json: &Map<String, Value>, key: &str) -> Option<Option<usize>> {
    match optional_int(json, key)? {
        None => Some(None),
        Some(v) => usize::try_from(v).ok().map(Some),
    }
}

fn format_of(json: &Map<String, Value>) -> Option<String> {
    Some(
        optional_str(json, "format")?
            .unwrap_or(text_format::PLAIN)
            .to_string(),
    )
}

fn version_of(json: &Map<String, Value>) -> Option<i64> {
    let v = optional_int(json, "v")?.unwrap_or(text_format::VERSION);
    (v >= 1).then_some(v)
}

/// A number as Dart's `int.tryParse` reads it: blanks around it, an
/// optional sign, then decimal digits, or hexadecimal ones after `0x`.
fn dart_int(text: &str) -> Option<i64> {
    let text = dart_trim(text);
    let (negative, rest) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (radix, digits) = match rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        Some(hex) => (16, hex),
        None => (10, rest),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let value = i64::from_str_radix(digits, radix).ok()?;
    Some(if negative { -value } else { value })
}

/// The bytes of `text` as Dart's `base64.decode` reads it: the standard
/// or the URL-safe alphabet, padded with `=` or `%3D` to a multiple of
/// four; none for anything else.
fn dart_base64(text: &str) -> Option<Vec<u8>> {
    if !text.is_ascii() {
        return None;
    }
    let normal = text
        .replace("%3D", "=")
        .replace("%3d", "=")
        .replace('-', "+")
        .replace('_', "/");
    BASE64.decode(normal).ok()
}

/// The Pack the payload `text` holds; none when it is none.
#[cfg(test)]
fn parse(text: &str) -> Option<Pack> {
    let json = payload_json(text)?;
    pack_of(&json, optional_str(&json, "profile")?)
}

/// The JSON object of the payload `text`; none when it is none.
fn payload_json(text: &str) -> Option<Map<String, Value>> {
    match serde_json::from_str(text).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// The Pack the payload `json` holds, its card `card` read already;
/// none when it is none.
fn pack_of(json: &Map<String, Value>, card: Option<&str>) -> Option<Pack> {
    let areas = json.get("areas")?.as_array()?;
    let notes = json.get("notes")?.as_array()?;
    let out = Pack {
        areas: areas.iter().map(area).collect::<Option<_>>()?,
        notes: notes.iter().map(note).collect::<Option<_>>()?,
        card: card.map(str::to_string),
    };
    check(&out).then_some(out)
}

/// Whether every index of `pack` points at something.
fn check(pack: &Pack) -> bool {
    pack.notes.iter().all(|n| {
        let area = match n.area {
            Some(a) => pack.areas.get(a),
            None => None,
        };
        let area_ok = n.area.is_none() || area.is_some();
        let topic_ok = match n.topic {
            None => true,
            Some(t) => area.is_some_and(|a| t < a.topics.len()),
        };
        let pool_ok = n.session.as_ref().is_none_or(|s| {
            s.pool
                .iter()
                .all(|i| pack.notes.get(*i).is_some_and(|t| !t.fields.is_empty()))
        });
        let fields_ok = n
            .fields
            .iter()
            .all(|f| f.multiplies.is_none_or(|m| m < n.fields.len()));
        area_ok && topic_ok && pool_ok && fields_ok
    })
}

fn area(json: &Value) -> Option<PackArea> {
    let name = json.get("name")?.as_str()?;
    let topics = json.get("topics")?.as_array()?;
    if dart_trim(name).is_empty() {
        return None;
    }
    let topics = topics
        .iter()
        .map(|t| {
            t.as_str()
                .filter(|t| !dart_trim(t).is_empty())
                .map(str::to_string)
        })
        .collect::<Option<_>>()?;
    Some(PackArea {
        name: name.to_string(),
        topics,
    })
}

fn note(json: &Value) -> Option<PackNote> {
    let json = json.as_object()?;
    let text = json.get("text")?.as_str()?;
    let list = optional_str(json, "list")?;
    let lines_json = match present(json, "lines") {
        None => None,
        Some(v) => Some(v.as_array()?),
    };
    if list.is_none() != lines_json.is_none() {
        return None;
    }
    if list.is_some_and(|l| l != shopping_keys::SHOPPING && l != checklist_keys::CHECKLIST) {
        return None;
    }
    let shopping = list == Some(shopping_keys::SHOPPING);
    let area = index(json, "area")?;
    let topic = index(json, "topic")?;
    let outside = optional_bool(json, "outside")?.unwrap_or(false);
    let format = format_of(json)?;
    let version = version_of(json)?;
    let sections = optional_list(json, "sections")?
        .iter()
        .map(|s| {
            let s = s.as_object()?;
            Some(PackSection {
                text: s.get("text")?.as_str()?.to_string(),
                format: format_of(s)?,
                version: version_of(s)?,
            })
        })
        .collect::<Option<_>>()?;
    let lines = match lines_json {
        None => None,
        Some(lines) => Some(
            lines
                .iter()
                .map(|l| {
                    let l = l.as_object()?;
                    let text = l.get("text")?.as_str()?.to_string();
                    // Only a Checklist's Line is under another, only a
                    // Shopping List's has a reappear time.
                    let indent = !shopping && optional_bool(l, "indent")?.unwrap_or(false);
                    let reappear = if shopping {
                        Some(Reappear::read(optional_str(l, "reappear")?))
                    } else {
                        None
                    };
                    Some(PackLine {
                        text,
                        indent,
                        reappear,
                    })
                })
                .collect::<Option<_>>()?,
        ),
    };
    let fields = optional_list(json, "fields")?
        .iter()
        .map(field)
        .collect::<Option<_>>()?;
    let bar = match optional_str(json, "bar")? {
        None => None,
        Some(bar) => Some(TallyPeriod::parse(bar)?),
    };
    let sets = optional_bool(json, "sets")?.unwrap_or(false);
    let session = match present(json, "session") {
        None => None,
        Some(s) => Some(session(s)?),
    };
    let routines = optional_list(json, "routines")?
        .iter()
        .map(routine)
        .collect::<Option<_>>()?;
    let frog = optional_bool(json, "frog")?.unwrap_or(false);
    let photos = optional_list(json, "photos")?
        .iter()
        .map(file)
        .collect::<Option<_>>()?;
    let recordings = optional_list(json, "recordings")?
        .iter()
        .map(file)
        .collect::<Option<_>>()?;
    let attachments = optional_list(json, "attachments")?
        .iter()
        .map(attached)
        .collect::<Option<_>>()?;
    Some(PackNote {
        area,
        topic,
        outside,
        text: text.to_string(),
        format,
        version,
        sections,
        lines,
        shopping,
        fields,
        bar,
        sets,
        session,
        routines,
        frog,
        photos,
        recordings,
        attachments,
    })
}

fn field(json: &Value) -> Option<PackField> {
    let json = json.as_object()?;
    let name = json.get("name")?.as_str()?;
    let rule = json.get("rule")?.as_str()?;
    if dart_trim(name).is_empty() {
        return None;
    }
    let target = optional_str(json, "target")?;
    if target.is_some_and(|t| decode_number(Some(t)).is_none()) {
        return None;
    }
    Some(PackField {
        name: name.to_string(),
        unit: optional_str(json, "unit")?.unwrap_or("").to_string(),
        rule: TallyRule::parse(rule)?,
        target: target.map(str::to_string),
        hidden: optional_bool(json, "hidden")?.unwrap_or(false),
        check: optional_bool(json, "check")?.unwrap_or(false),
        multiplies: index(json, "multiplies")?,
    })
}

fn session(json: &Value) -> Option<PackSession> {
    let json = json.as_object()?;
    let count = json.get("count")?.as_i64()?;
    let pool = json.get("pool")?.as_array()?;
    let rule = json.get("rule")?.as_str()?;
    if pool.is_empty() {
        return None;
    }
    let mut counts = BTreeMap::new();
    let counts_json = match present(json, "counts") {
        None => None,
        Some(v) => Some(v.as_object()?),
    };
    for (key, value) in counts_json.into_iter().flatten() {
        let day = dart_int(key).filter(|d| (1..=7).contains(d))?;
        let c = value.as_i64()?;
        counts.insert(u32::try_from(day).ok()?, c);
    }
    if std::iter::once(&count)
        .chain(counts.values())
        .any(|c| *c < 1 || *c > session_keys::MAX_COUNT)
    {
        return None;
    }
    Some(PackSession {
        pool: pool
            .iter()
            .map(|i| i.as_i64().and_then(|i| usize::try_from(i).ok()))
            .collect::<Option<_>>()?,
        count,
        counts,
        rule: SessionRule::parse(rule)?,
    })
}

fn routine(json: &Value) -> Option<Schedule> {
    let map = json.as_object()?;
    let start = map.get("start")?.as_str()?;
    let times = map.get("times")?.as_str()?;
    let rule = optional_str(map, "rule")?;
    Routines::schedule_of(&Fields::new(vec![
        (routine_keys::START.to_string(), Some(start.to_string())),
        (routine_keys::TIMES.to_string(), Some(times.to_string())),
        (routine_keys::RULE.to_string(), rule.map(str::to_string)),
    ]))
}

fn file(json: &Value) -> Option<PackFile> {
    let json = json.as_object()?;
    let hash = json.get("hash")?.as_str()?;
    if hash.len() != 64 || !hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let bytes = match optional_str(json, "bytes")? {
        None => None,
        Some(b) => Some(dart_base64(b)?),
    };
    let text = optional_str(json, "text")?.map(str::to_string);
    if bytes.is_none() && text.is_none() {
        return None;
    }
    Some(PackFile {
        hash: hash.to_string(),
        has_file: bytes.is_some(),
        size: bytes.as_ref().map_or(0, Vec::len),
        bytes,
        text,
        name: String::new(),
        media_type: String::new(),
        pages: None,
    })
}

/// An attached file of a Pack's Note: as [`file`], with its name, media
/// type and pages, each read as an attached file's field reads them
/// ([`Attachment::read`]); a name or type that is there but no string,
/// or pages that are no whole number, make the Pack none.
fn attached(json: &Value) -> Option<PackFile> {
    let map = json.as_object()?;
    let name = optional_str(map, "name")?.unwrap_or("");
    let media_type = optional_str(map, "type")?.unwrap_or("");
    let pages = match present(map, "pages") {
        None => None,
        Some(p) => Some(p.as_i64().and_then(|p| u64::try_from(p).ok())?),
    };
    Some(PackFile {
        name: crate::notes::clean_name(name),
        media_type: crate::notes::clean_type(media_type),
        pages: pages.filter(|p| *p <= 1 << 53),
        ..file(json)?
    })
}

// --------------------------------------------------------------- export

/// The wall clock a Shopping List is read on for a Pack: UTC, the same
/// on every Device and in both cores; it decides only the order of its
/// Lines, which all arrive open.
fn utc(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// What a Pack does with a Note Hidden from AI (#267), as the Dart
/// core's: one picked for export is refused, the feedback of everything
/// leaves it out, a Backup opened here keeps it, as it is the user's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hidden {
    Refused,
    LeftOut,
    Kept,
}

impl Store {
    /// The Pack of everything the tree may offer when exporting from the
    /// Area, Topic or Note `from` (Q468), as the Dart core's `packOf`:
    /// from a Note, that Note; from a Topic, its Notes; from an Area, its
    /// Notes and its Topics' Notes, newest capture first; Notes of other
    /// people in a Shared Area too (Q466). Each with what it is (Q455),
    /// its photos and recordings with their texts (Q456, Q457), its
    /// Routines and Reminders (Q465) and an open Frog (Q473); the Tallies
    /// its Session picks from that lie elsewhere after them, marked as
    /// brought from outside, with their own Area and Topic, but for those
    /// filed into an Area of `closed`: the locked Areas not open now,
    /// whose Tallies the app does not show. Only the names of the Areas
    /// and Topics go along (Q469). Nothing in the trash or merged into
    /// another Note. Refused when a Note it would hold is Hidden from AI
    /// ([`Error::HiddenFromAi`], #267).
    pub fn pack_of(&self, from: &str, closed: &HashSet<String>) -> Result<Pack> {
        let notes = Notes::new(self);
        let places = Places::new(self);
        let is_area = places.area_by_id(from)?.is_some();
        let picked: Vec<Note> = if is_area || places.topic_by_id(from)?.is_some() {
            // The Area's own Notes first, then each Topic's, by name.
            let mut ids = vec![from.to_string()];
            if is_area {
                ids.extend(places.topics(from)?.into_iter().map(|t| t.id));
            }
            let all = notes.all()?;
            ids.iter()
                .flat_map(|id| {
                    all.iter()
                        .filter(move |n| n.place.as_deref() == Some(id.as_str()))
                })
                .cloned()
                .collect()
        } else {
            let note = notes
                .by_id(from)?
                .filter(|n| !n.is_trashed() && !n.is_merged())
                .ok_or_else(|| Error::Invalid(format!("not an Area, Topic or Note: {from}")))?;
            vec![note]
        };
        self.pack_of_notes(&picked, closed, Hidden::Refused)
    }

    /// The Pack of everything the tree may offer for feedback, as
    /// [`Store::pack_of_all_by`] with the Areas missing from the person's
    /// own order by stored name.
    pub fn pack_of_all(&self, closed: &HashSet<String>) -> Result<Pack> {
        self.pack_of_all_by(closed, |a| a.name.clone())
    }

    /// The Pack of everything the tree may offer for feedback (#214), as
    /// the Dart core's `packOfAll`: each Area in the person's own order,
    /// as [`PersonSettings::areas_in_order_by`] by `shown` lists them
    /// (#280), but those of `closed`, with its own Notes and then its
    /// Topics' Notes as [`Store::pack_of`] lists them, then the Notes
    /// filed into no Area, newest capture first; nothing in the trash or
    /// merged, and nothing Hidden from AI (#267), which the user did not
    /// pick.
    pub fn pack_of_all_by(
        &self,
        closed: &HashSet<String>,
        shown: impl Fn(&Area) -> String,
    ) -> Result<Pack> {
        let places = Places::new(self);
        let all = Notes::new(self).all()?;
        let mut ids = Vec::new();
        for area in PersonSettings::new(self).areas_in_order_by(shown)? {
            if closed.contains(&area.id) {
                continue;
            }
            let topics = places.topics(&area.id)?;
            ids.push(area.id);
            ids.extend(topics.into_iter().map(|t| t.id));
        }
        let mut picked: Vec<Note> = ids
            .iter()
            .flat_map(|id| {
                all.iter()
                    .filter(move |n| n.place.as_deref() == Some(id.as_str()))
            })
            .cloned()
            .collect();
        picked.extend(all.iter().filter(|n| n.place.is_none()).cloned());
        self.pack_of_notes(&picked, closed, Hidden::LeftOut)
    }

    /// The Pack of a Backup opened to take Notes back from it (#285), as
    /// the Dart core's `packOfEverything`: every Note this Store holds but
    /// those merged into another, those in the trash among them, each
    /// Area in the person's own order by `shown` with its own Notes and
    /// then its Topics' Notes, but those of `closed`, then the Notes filed
    /// into no Area, newest capture first, as [`Store::pack_of_all_by`]
    /// lists them; the Tallies a Session picks from that are not among
    /// them after them, but for those filed into an Area of `closed`. The
    /// files' sizes come from `size_of`, `photo` or `audio` and a hash to
    /// the size of the bytes held, none for none.
    pub fn pack_of_everything(
        &self,
        closed: &HashSet<String>,
        shown: impl Fn(&Area) -> String,
        size_of: &SizeOf,
    ) -> Result<EverythingPack> {
        let places = Places::new(self);
        let notes = Notes::new(self);
        let mut kept = Vec::new();
        for id in self.entities_of_type(note_keys::TYPE_NOTE)? {
            if let Some(note) = notes.by_id(&id)?
                && !note.is_merged()
            {
                kept.push(note);
            }
        }
        kept.sort_by(|a, b| b.captured.cmp(&a.captured).then_with(|| a.id.cmp(&b.id)));
        let mut ids = Vec::new();
        let mut left_out = BTreeSet::new();
        for area in PersonSettings::new(self).areas_in_order_by(shown)? {
            let topics = places.topics(&area.id)?;
            let mut here = vec![area.id.clone()];
            here.extend(topics.into_iter().map(|t| t.id));
            if !closed.contains(&area.id) {
                ids.extend(here);
            } else if kept
                .iter()
                .any(|n| n.place.as_ref().is_some_and(|p| here.contains(p)))
            {
                left_out.insert(area.id);
            }
        }
        let mut picked: Vec<Note> = ids
            .iter()
            .flat_map(|id| {
                kept.iter()
                    .filter(move |n| n.place.as_deref() == Some(id.as_str()))
            })
            .cloned()
            .collect();
        picked.extend(kept.iter().filter(|n| n.place.is_none()).cloned());
        let (pack, order) = self.pack_of_notes_by(&picked, closed, size_of, Hidden::Kept)?;
        let mut trashed = BTreeSet::new();
        for (i, id) in order.iter().enumerate() {
            if notes.by_id(id)?.is_some_and(|n| n.is_trashed()) {
                trashed.insert(i);
            }
        }
        Ok(EverythingPack {
            pack,
            trashed,
            left_out,
        })
    }

    /// The Pack of `picked`, in their order, and the Tallies their
    /// Sessions pick from that are not among them after them, but for
    /// those filed into an Area of `closed`; what is Hidden from AI goes
    /// as `hidden` says.
    fn pack_of_notes(
        &self,
        picked: &[Note],
        closed: &HashSet<String>,
        hidden: Hidden,
    ) -> Result<Pack> {
        let held = |kind: &str, hash: &str| self.blob(kind, hash).map(|b| b.len());
        Ok(self.pack_of_notes_by(picked, closed, &held, hidden)?.0)
    }

    /// [`Store::pack_of_notes`] with the files' sizes from `size_of`, and
    /// the ids of the Notes in the order the Pack holds them.
    fn pack_of_notes_by(
        &self,
        picked: &[Note],
        closed: &HashSet<String>,
        size_of: &SizeOf,
        hidden: Hidden,
    ) -> Result<(Pack, Vec<String>)> {
        let notes = Notes::new(self);
        let places = Places::new(self);
        let marks = match hidden {
            Hidden::Kept => BTreeSet::new(),
            _ => self.hidden_from_ai_marks()?,
        };
        let marks_of = |id: &str| self.hidden_by(id, &marks);
        let left_out = |id: &str| -> Result<bool> {
            Ok(hidden == Hidden::LeftOut && !marks_of(id)?.is_empty())
        };
        let mut order: Vec<String> = Vec::new();
        for n in picked {
            if !left_out(&n.id)? {
                order.push(n.id.clone());
            }
        }
        let sessions = Sessions::new(self);
        // The Tallies the Sessions pick from that are not among them.
        let mut k = 0;
        while let Some(id) = order.get(k).cloned() {
            if let Some(setup) = sessions.setup_of(&id)? {
                for t in sessions.pool(&setup)? {
                    if order.contains(&t) || left_out(&t)? {
                        continue;
                    }
                    let place = notes.by_id(&t)?.and_then(|n| n.place);
                    let area = match place {
                        Some(place) => Some(match places.topic_by_id(&place)? {
                            Some(topic) => topic.area_id,
                            None => place,
                        }),
                        None => None,
                    };
                    if area.is_some_and(|a| closed.contains(&a)) {
                        continue;
                    }
                    order.push(t);
                }
            }
            k += 1;
        }
        if hidden == Hidden::Refused {
            let mut found: BTreeSet<String> = BTreeSet::new();
            for id in &order {
                found.extend(marks_of(id)?);
            }
            let mut found: Vec<String> = found.into_iter().collect();
            found.sort_by(|a, b| crate::store::utf16_cmp(a, b));
            Store::refuse_hidden_from_ai(found)?;
        }
        let index: HashMap<String, usize> = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();
        let mut areas: HashMap<String, usize> = HashMap::new();
        let mut topics: HashMap<String, usize> = HashMap::new();
        let mut area_names: Vec<PackArea> = Vec::new();
        let mut out = Vec::new();
        for (i, id) in order.iter().enumerate() {
            let note = notes
                .by_id(id)?
                .ok_or_else(|| Error::Invalid(format!("not a Note: {id}")))?;
            let (mut area, mut topic) = (None, None);
            if let Some(place) = &note.place {
                let in_topic = places.topic_by_id(place)?;
                let area_id = in_topic
                    .as_ref()
                    .map_or(place.clone(), |t| t.area_id.clone());
                if let Some(found) = places.area_by_id(&area_id)? {
                    let a = *areas.entry(area_id).or_insert_with(|| {
                        area_names.push(PackArea {
                            name: found.name.clone(),
                            topics: Vec::new(),
                        });
                        area_names.len() - 1
                    });
                    area = Some(a);
                    if let Some(t) = in_topic
                        && let Some(named) = area_names.get_mut(a)
                    {
                        topic = Some(*topics.entry(t.id).or_insert_with(|| {
                            named.topics.push(t.name.clone());
                            named.topics.len() - 1
                        }));
                    }
                }
            }
            out.push(self.pack_note(&note, area, topic, i >= picked.len(), &index, size_of)?);
        }
        let pack = Pack {
            areas: area_names,
            notes: out,
            card: None,
        };
        Ok((pack, order))
    }

    /// The Note `note` as a Pack carries it, filed into the Pack's Area
    /// `area` and Topic `topic`, its Session's pool by `index`, the sizes
    /// of its files from `size_of`.
    fn pack_note(
        &self,
        note: &Note,
        area: Option<usize>,
        topic: Option<usize>,
        outside: bool,
        index: &HashMap<String, usize>,
        size_of: &SizeOf,
    ) -> Result<PackNote> {
        let id = &note.id;
        let kind = NoteKind::of(self, id)?;
        let sessions = Sessions::new(self);
        let setup = match kind {
            NoteKind::Session => sessions.setup_of(id)?,
            _ => None,
        };
        let tally = match kind {
            NoteKind::Tally => Tallies::new(self).by_note(id)?,
            _ => None,
        };
        let bar = tally
            .as_ref()
            .map(|t| t.bar)
            .filter(|b| *b != TallyPeriod::Day);
        let sets = tally.as_ref().is_some_and(|t| t.counts_in_sets);
        let fields = tally.map(|t| t.fields).unwrap_or_default();
        let field_index: HashMap<&str, usize> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id.as_str(), i))
            .collect();
        let file = |kind: &str, hash: &str, text: Option<&String>| {
            let size = size_of(kind, hash);
            PackFile {
                hash: hash.to_string(),
                bytes: None,
                has_file: size.is_some(),
                size: size.unwrap_or(0),
                text: text.cloned(),
                name: String::new(),
                media_type: String::new(),
                pages: None,
            }
        };
        let mut routines = Routines::new(self).of_note(id)?;
        routines.sort_by(|a, b| a.id.cmp(&b.id));
        let lines = match kind {
            NoteKind::Checklist => Some(
                Checklists::new(self)
                    .by_note(id)?
                    .map(|c| c.lines)
                    .unwrap_or_default()
                    .iter()
                    .map(|l| PackLine {
                        text: l.text.clone(),
                        indent: l.in_group(),
                        reappear: None,
                    })
                    .collect(),
            ),
            NoteKind::ShoppingList => Some(
                ShoppingLists::new(self)
                    .by_note(id, &utc)?
                    .map(|s| s.lines)
                    .unwrap_or_default()
                    .iter()
                    .map(|l| PackLine {
                        text: l.text.clone(),
                        indent: false,
                        reappear: Some(l.reappear),
                    })
                    .collect(),
            ),
            _ => None,
        };
        let session = match &setup {
            Some(setup) => Some(PackSession {
                pool: sessions
                    .pool(setup)?
                    .iter()
                    .filter_map(|t| index.get(t).copied())
                    .collect(),
                count: setup.count,
                counts: setup.counts.clone(),
                rule: setup.rule,
            }),
            None => None,
        };
        Ok(PackNote {
            area,
            topic,
            outside,
            text: baked(&note.text, note.text_editable, &note.boxes),
            format: note.text_format.clone(),
            version: note.text_version,
            sections: Notes::new(self)
                .sections(id)?
                .iter()
                .filter(|s| !dart_trim(&s.text).is_empty())
                .map(|s| PackSection {
                    text: baked(&s.text, s.text_editable, &s.boxes),
                    format: s.text_format.clone(),
                    version: s.text_version,
                })
                .collect(),
            lines,
            shopping: kind == NoteKind::ShoppingList,
            fields: fields
                .iter()
                .map(|f| PackField {
                    name: f.name.clone(),
                    unit: f.unit.clone(),
                    rule: f.rule,
                    target: f.target.and_then(encode_number),
                    hidden: f.hidden,
                    check: f.check,
                    multiplies: f
                        .multiplies
                        .as_deref()
                        .and_then(|m| field_index.get(m).copied()),
                })
                .collect(),
            bar,
            sets,
            session,
            routines: routines.into_iter().map(|r| r.schedule).collect(),
            frog: Frogs::new(self).of_note(id)?.iter().any(|f| !f.is_done()),
            photos: note
                .photos
                .iter()
                .chain(&note.removed_photos)
                .map(|hash| file("photo", hash, note.photo_texts.get(hash)))
                .collect(),
            recordings: note
                .recordings
                .iter()
                .chain(&note.removed_recordings)
                .map(|hash| file("audio", hash, note.transcripts.get(hash)))
                .collect(),
            attachments: note
                .attachments
                .iter()
                .chain(&note.removed_attachments)
                .map(|a| PackFile {
                    name: a.name.clone(),
                    media_type: a.media_type.clone(),
                    pages: a.pages,
                    ..file("file", &a.hash, note.attachment_texts.get(&a.hash))
                })
                .collect(),
        })
    }

    /// The file of `pack` as `ticks` picks from it ([`Pack::picked`]):
    /// with `add_profile` it carries the own Profile card and is signed
    /// by this Device (Q462). An error when `add_profile` is asked before
    /// the own Profile can make a card.
    pub fn write_pack(&self, pack: &Pack, ticks: &PackTicks, add_profile: bool) -> Result<Vec<u8>> {
        let picked = pack.picked(ticks);
        let card = if add_profile {
            Some(
                self.profile_card_text()?
                    .ok_or_else(|| Error::Invalid("no own Profile card".into()))?,
            )
        } else {
            None
        };
        let key = match card {
            Some(_) => Some(self.signing_key()?),
            None => None,
        };
        let out = Pack {
            areas: picked.areas,
            notes: picked.notes,
            card,
        };
        encode_pack(&out, key.as_ref(), &|kind, hash| self.blob(kind, hash))
    }
}

// --------------------------------------------------------------- import

/// A Pack opened here: what it holds and who it is from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPack {
    pub pack: Pack,
    /// The sender's Profile as the Pack's card carries it, sanitised,
    /// with the picture made again and the user's Override; none for a
    /// Pack without a Profile, or one the user made.
    pub from: Option<Profile>,
    /// Whether the sender's Profile is kept here, bound to the key that
    /// signed the Pack.
    pub from_kept: bool,
}

/// What adding a Pack made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackAdded {
    /// The new Areas, in the order they were made.
    pub areas: Vec<String>,
    /// How many Notes were made.
    pub notes: usize,
}

/// The "From `<name>`" badge of a Note a Pack brought (Q463).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFrom {
    /// The sender's Profile id.
    pub id: String,
    /// The name it shows with.
    pub name: String,
}

/// The hashes a Note's photos and recordings are kept under here.
struct Kept {
    photos: Vec<String>,
    recordings: Vec<String>,
    attachments: Vec<String>,
}

impl Store {
    /// Opens the Pack `bytes` (a file opened with the app), as the Dart
    /// core's `openPack`: checks it and says who it is from.
    /// [`PackError::NeedsUpdate`] for one of a later format,
    /// [`PackError::NotAPack`] for a file that is none, damaged, or whose
    /// signature does not hold over what it holds or carries a Profile not
    /// bound to the key that signed it, and [`PackError::FromBlocked`] for
    /// one carrying the Profile of a blocked person, or made by a Device
    /// of theirs (Q471). Nothing is kept.
    pub fn open_pack(&self, bytes: &[u8]) -> Result<std::result::Result<OpenedPack, PackError>> {
        let Some(json) = pack_json(bytes) else {
            return Ok(Err(PackError::NotAPack));
        };
        // Only the whole number 1 is version 1, never `1.0` or `"1"`.
        if json
            .get("format")
            .is_some_and(|format| format.as_i64() != Some(FORMAT))
        {
            return Ok(Err(PackError::NeedsUpdate));
        }
        let Some(payload) = json.get("pack").and_then(Value::as_str) else {
            return Ok(Err(PackError::NotAPack));
        };
        let sig = match present(&json, "sig") {
            None => None,
            Some(Value::String(sig)) => Some(sig.as_str()),
            Some(_) => return Ok(Err(PackError::NotAPack)),
        };
        // The card and the signature are checked, and a blocked sender
        // refused, before any Area or Note is read (ADR 0043).
        let Some(body) = payload_json(payload) else {
            return Ok(Err(PackError::NotAPack));
        };
        let Some(card) = optional_str(&body, "profile") else {
            return Ok(Err(PackError::NotAPack));
        };
        let (card, sig) = match (card, sig) {
            (None, None) => {
                return Ok(match pack_of(&body, None) {
                    Some(pack) => Ok(OpenedPack {
                        pack,
                        from: None,
                        from_kept: false,
                    }),
                    None => Err(PackError::NotAPack),
                });
            }
            (Some(card), Some(sig)) => (card, sig),
            _ => return Ok(Err(PackError::NotAPack)),
        };
        let Some(checked) = check_profile_card(card) else {
            return Ok(Err(PackError::NotAPack));
        };
        if !verify_signature(&checked.key.public_key, &pack_bytes(payload), Some(sig)) {
            return Ok(Err(PackError::NotAPack));
        }
        if self.is_blocked(&checked.id)? || self.blocked_devices()?.contains(&checked.key.device) {
            return Ok(Err(PackError::FromBlocked));
        }
        let Some(pack) = pack_of(&body, Some(card)) else {
            return Ok(Err(PackError::NotAPack));
        };
        if self.is_own_person(&checked.id)? {
            return Ok(Ok(OpenedPack {
                pack,
                from: None,
                from_kept: false,
            }));
        }
        let Some(sender) =
            self.sent_item_profile(&checked.profile.to_json(), &checked.key.public_key)?
        else {
            return Ok(Err(PackError::NotAPack));
        };
        Ok(Ok(OpenedPack {
            pack,
            from: Some(sender.profile),
            from_kept: sender.kept,
        }))
    }

    /// "Add" on the Pack `opened` with `ticks` (Q459), as the Dart core's
    /// `addPack`: the ticked Notes, as [`Pack::picked`] picks them, as new
    /// Notes of this Device's, captured now, in new private Areas and
    /// Topics, never in an Area there already (Q460, Q461): each of the
    /// Pack's Areas becomes a new Area of its name, or of its name with
    /// ` (<suffix>)` when an Area has it, ` (<suffix> 2)` and so on when
    /// that is taken too; a Note of no Area lands in a new Area named
    /// `no_area` by the same rule. Routines start today by `wall_clock`,
    /// their times and rules as they are (Q464); a Reminder keeps its day
    /// and time, and one already past is left out (Q465). An open Frog is
    /// a new Frog. Without `claim`, each Note keeps a "From `<name>`"
    /// badge ([`keys::FROM`], Q463), when the Pack says who it is from.
    /// All of it is written in one batch.
    pub fn add_pack(
        &self,
        opened: &OpenedPack,
        ticks: &PackTicks,
        claim: bool,
        wall_clock: &WallClock,
        suffix: &str,
        no_area: &str,
    ) -> Result<PackAdded> {
        let pack = opened.pack.picked(ticks);
        if pack.notes.is_empty() {
            return Ok(PackAdded {
                areas: Vec::new(),
                notes: 0,
            });
        }
        let now = self.now();
        let today = Day::today(now, wall_clock);
        let wall = wall_clock(now);
        let wall_minute = wall.hour() * 60 + wall.minute();
        let badge = match (&opened.from, claim) {
            (Some(from), false) => {
                Some(object(vec![("id", json!(from.id)), ("name", json!(from.name))]).to_string())
            }
            _ => None,
        };
        // The bytes are kept before the log names them.
        let mut kept = Vec::new();
        for n in &pack.notes {
            let mut photos = Vec::new();
            for p in &n.photos {
                photos.push(match &p.bytes {
                    Some(bytes) => self.put_photo(bytes)?,
                    None => p.hash.clone(),
                });
            }
            let mut recordings = Vec::new();
            for a in &n.recordings {
                recordings.push(match &a.bytes {
                    Some(bytes) => self.put_audio(bytes)?,
                    None => a.hash.clone(),
                });
            }
            let mut attachments = Vec::new();
            for d in &n.attachments {
                attachments.push(match &d.bytes {
                    Some(bytes) => self.put_file(bytes)?,
                    None => d.hash.clone(),
                });
            }
            kept.push(Kept {
                photos,
                recordings,
                attachments,
            });
        }
        self.batch(|| {
            let mut taken: HashSet<String> = Places::new(self)
                .areas()?
                .into_iter()
                .map(|a| a.name)
                .collect();
            let mut new_area = |name: &str| -> Result<String> {
                let named = landing(dart_trim(name), suffix, &taken);
                taken.insert(named.clone());
                let id = format!("area:{}", self.new_id()?);
                self.append(&id, place_keys::TYPE, Some(place_keys::TYPE_AREA), None)?;
                self.append(&id, place_keys::NAME, Some(&named), None)?;
                Ok(id)
            };
            let mut made = Vec::new();
            let mut area_ids = Vec::new();
            for a in &pack.areas {
                let id = new_area(&a.name)?;
                made.push(id.clone());
                area_ids.push(id);
            }
            let mut topic_ids: Vec<Vec<String>> = Vec::new();
            for (pa, area_id) in pack.areas.iter().zip(&area_ids) {
                let mut ids = Vec::new();
                for t in &pa.topics {
                    let id = format!("topic:{}", self.new_id()?);
                    self.append(&id, place_keys::TYPE, Some(place_keys::TYPE_TOPIC), None)?;
                    self.append(&id, place_keys::AREA, Some(area_id), None)?;
                    self.append(&id, place_keys::NAME, Some(dart_trim(t)), None)?;
                    ids.push(id);
                }
                topic_ids.push(ids);
            }
            let mut none = None;
            if pack.notes.iter().any(|n| n.area.is_none()) {
                let id = new_area(no_area)?;
                made.push(id.clone());
                none = Some(id);
            }
            let unplaced = || Error::Invalid("a Pack's Note filed nowhere".into());
            let mut note_ids = Vec::new();
            for (n, held) in pack.notes.iter().zip(&kept) {
                let id = format!("note:{}", self.new_id()?);
                note_ids.push(id.clone());
                let place = match (n.area, n.topic) {
                    (Some(a), Some(t)) => topic_ids.get(a).and_then(|ts| ts.get(t)),
                    (Some(a), None) => area_ids.get(a),
                    _ => none.as_ref(),
                }
                .ok_or_else(unplaced)?;
                self.write_pack_note(&id, n, place, now, badge.as_deref(), held)?;
                for s in &n.routines {
                    if s.once() && past(s, today, wall_minute) {
                        continue;
                    }
                    if s.once() {
                        self.write_pack_routine(&id, s)?;
                    } else {
                        self.write_pack_routine(&id, &from_day(s, today))?;
                    }
                }
                if n.frog {
                    let frog = format!("{}{}", frog_keys::PREFIX, self.new_id()?);
                    self.append(&frog, frog_keys::TYPE, Some(frog_keys::TYPE_FROG), None)?;
                    self.append(&frog, frog_keys::NOTE, Some(&id), None)?;
                }
            }
            // The pools name the Notes made, so they come last.
            for (n, id) in pack.notes.iter().zip(&note_ids) {
                let Some(s) = &n.session else {
                    continue;
                };
                let pool = s
                    .pool
                    .iter()
                    .map(|t| note_ids.get(*t).cloned().ok_or_else(unplaced))
                    .collect::<Result<Vec<_>>>()?
                    .join(",");
                self.append(id, session_keys::POOL, Some(&pool), None)?;
                self.append(id, session_keys::COUNT, Some(&s.count.to_string()), None)?;
                for (d, c) in &s.counts {
                    self.append(id, &session_keys::count_on(*d), Some(&c.to_string()), None)?;
                }
                self.append(id, session_keys::RULE, Some(s.rule.name()), None)?;
            }
            Ok(PackAdded {
                areas: made,
                notes: pack.notes.len(),
            })
        })
    }

    /// Writes the Note `id` as `n` is, filed into `place`, captured `now`,
    /// with its Sections, files, Lines and fields; with `badge` as its
    /// "From `<name>`".
    fn write_pack_note(
        &self,
        id: &str,
        n: &PackNote,
        place: &str,
        now: DateTime<Utc>,
        badge: Option<&str>,
        kept: &Kept,
    ) -> Result<()> {
        self.append(id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), None)?;
        self.append(id, note_keys::CAPTURED, Some(&dart_iso(now)), None)?;
        let text = StoredText::of(&n.format, &n.text, n.version).encode();
        self.append(id, note_keys::TEXT, Some(&text), None)?;
        self.append(id, note_keys::PLACE, Some(place), None)?;
        if let Some(badge) = badge {
            self.append(id, keys::FROM, Some(badge), None)?;
        }
        for (j, s) in n.sections.iter().enumerate() {
            // Captured with the Note, in their order.
            let section = format!("section:{}", self.new_id()?);
            let captured = now + chrono::Duration::microseconds(j as i64 + 1);
            self.append(
                &section,
                note_keys::TYPE,
                Some(note_keys::TYPE_SECTION),
                None,
            )?;
            self.append(&section, note_keys::NOTE, Some(id), None)?;
            self.append(
                &section,
                note_keys::CAPTURED,
                Some(&dart_iso(captured)),
                None,
            )?;
            let text = StoredText::of(&s.format, &s.text, s.version).encode();
            self.append(&section, note_keys::TEXT, Some(&text), None)?;
        }
        for (p, hash) in n.photos.iter().zip(&kept.photos) {
            let added = p.has_file.then_some(note_keys::ADDED);
            self.append(id, &note_keys::photo(hash), added, None)?;
            if let Some(text) = &p.text {
                self.append(id, &note_keys::ocr(hash), Some(text), None)?;
            }
        }
        for (a, hash) in n.recordings.iter().zip(&kept.recordings) {
            let added = a.has_file.then_some(note_keys::ADDED);
            self.append(id, &note_keys::audio(hash), added, None)?;
            if let Some(text) = &a.text {
                self.append(id, &note_keys::transcript(hash), Some(text), None)?;
            }
        }
        for (d, hash) in n.attachments.iter().zip(&kept.attachments) {
            let value = d.has_file.then(|| {
                let kind = file_text::kind_of(&d.name, &d.media_type);
                let size = d.size as u64;
                Attachment {
                    hash: hash.clone(),
                    name: d.name.clone(),
                    media_type: d.media_type.clone(),
                    size,
                    pages: d.pages,
                    computer: file_text::computer_reads(kind, size, d.pages),
                }
                .value()
            });
            self.append(id, &note_keys::file(hash), value.as_deref(), None)?;
            if let Some(text) = &d.text {
                self.append(id, &note_keys::file_text(hash), Some(text), None)?;
            }
        }
        if let Some(lines) = &n.lines {
            let making = self.new_id()?;
            let kind = if n.shopping {
                shopping_keys::kind_of(&making)
            } else {
                checklist_keys::kind_of(&making)
            };
            self.append(id, checklist_keys::KIND, Some(&kind), None)?;
            let mut key = String::new();
            for (j, l) in lines.iter().enumerate() {
                key = line_order::after(&key);
                let line = write_line(
                    self,
                    id,
                    &making,
                    &NewLine {
                        text: &l.text,
                        order: &key,
                        indent: !n.shopping && l.indent && j > 0,
                        checked: false,
                        received: None,
                        at: None,
                    },
                )?;
                if n.shopping {
                    let reappear = l.reappear.unwrap_or(Reappear::FALLBACK);
                    self.append(&line, shopping_keys::REAPPEAR, Some(&reappear.code()), None)?;
                }
            }
        }
        let field_ids = n
            .fields
            .iter()
            .map(|_| self.new_id())
            .collect::<Result<Vec<_>>>()?;
        for (f, field) in n.fields.iter().zip(&field_ids) {
            let key = |k: &str| tally_keys::field(field, k);
            self.append(id, &key(tally_keys::NAME), Some(dart_trim(&f.name)), None)?;
            self.append(id, &key(tally_keys::UNIT), Some(dart_trim(&f.unit)), None)?;
            self.append(id, &key(tally_keys::RULE), Some(f.rule.name()), None)?;
            if let Some(target) = &f.target {
                self.append(id, &key(tally_keys::TARGET), Some(target), None)?;
            }
            if f.hidden {
                self.append(id, &key(tally_keys::HIDDEN), Some("true"), None)?;
            }
            if f.check {
                self.append(
                    id,
                    &key(tally_keys::KIND),
                    Some(tally_keys::KIND_CHECK),
                    None,
                )?;
            }
        }
        for (j, (f, own)) in n.fields.iter().zip(&field_ids).enumerate() {
            let Some(m) = f.multiplies.filter(|m| *m != j) else {
                continue;
            };
            let Some(other) = field_ids.get(m) else {
                continue;
            };
            let key = tally_keys::field(own, tally_keys::MULTIPLIES);
            self.append(id, &key, Some(other), None)?;
        }
        if !n.fields.is_empty() {
            if let Some(bar) = n.bar {
                self.append(id, tally_keys::BAR, Some(bar.name()), None)?;
            }
            if n.sets {
                self.append(id, tally_keys::SETS, Some("true"), None)?;
            }
        }
        Ok(())
    }

    /// Writes a Routine or Reminder of the Note `note_id` on `schedule`.
    fn write_pack_routine(&self, note_id: &str, schedule: &Schedule) -> Result<()> {
        let id = format!("routine:{}", self.new_id()?);
        self.append(
            &id,
            routine_keys::TYPE,
            Some(routine_keys::TYPE_ROUTINE),
            None,
        )?;
        self.append(&id, routine_keys::NOTE, Some(note_id), None)?;
        self.append(
            &id,
            routine_keys::START,
            Some(&schedule.start.to_string()),
            None,
        )?;
        self.append(&id, routine_keys::TIMES, Some(&times(schedule)), None)?;
        if let Some(rule) = &schedule.rule {
            self.append(&id, routine_keys::RULE, Some(&rule.format()), None)?;
        }
        Ok(())
    }

    /// The "From `<name>`" badge of the Note `note_id` (Q463), as the Dart
    /// core's `packFrom`: the sender's Profile id and the name it shows
    /// with, the user's Override or the name kept here when their Profile
    /// is kept, else the name the Pack carried; none when it has none, or
    /// it was removed. Only what the user's own Devices wrote counts
    /// (`own_current`, Q554 (a)): another person's Device in a Shared
    /// Area can neither set nor change it.
    pub fn pack_from(&self, note_id: &str) -> Result<Option<PackFrom>> {
        let Some(raw) = self.own_current(note_id, keys::FROM, &mut HashMap::new())? else {
            return Ok(None);
        };
        let Ok(Value::Object(json)) = serde_json::from_str::<Value>(&raw) else {
            return Ok(None);
        };
        let (Some(id), Some(name)) = (
            json.get("id").and_then(Value::as_str),
            json.get("name").and_then(Value::as_str),
        ) else {
            return Ok(None);
        };
        let mut kept = false;
        for p in self.kept_people()? {
            if self.resolve_person(&p.id)? == id {
                kept = true;
                break;
            }
        }
        let name = if kept {
            self.person_shown(id)?.name
        } else {
            name.to_string()
        };
        Ok(Some(PackFrom {
            id: id.to_string(),
            name,
        }))
    }

    /// Removes the "From `<name>`" badge of the Note `note_id`.
    pub fn remove_pack_from(&self, note_id: &str) -> Result<()> {
        if self
            .own_current(note_id, keys::FROM, &mut HashMap::new())?
            .is_none()
        {
            return Ok(());
        }
        if !self.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        self.append(note_id, keys::FROM, None, None).map(drop)
    }

    /// Brings back the "From `<name>`" badge of the Note `note_id` that
    /// [`Store::remove_pack_from`] took away, as the user's own Devices
    /// last wrote it (#268). Nothing while it shows, or when it never had
    /// one.
    pub fn restore_pack_from(&self, note_id: &str) -> Result<()> {
        let mut own = HashMap::new();
        if self.own_current(note_id, keys::FROM, &mut own)?.is_some() {
            return Ok(());
        }
        let mut last: Option<Entry> = None;
        for e in self.logged_history(note_id)? {
            if e.field != keys::FROM
                || e.value.is_none()
                || !self.written_by_own_in(&e.device, &mut own)?
            {
                continue;
            }
            if last
                .as_ref()
                .is_none_or(|l| Store::latest_wins(&e, l) == Ordering::Greater)
            {
                last = Some(e);
            }
        }
        let Some(last) = last else {
            return Ok(());
        };
        if !self.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        self.append(note_id, keys::FROM, last.value.as_deref(), None)
            .map(drop)
    }
}

/// `name`, or with ` (<suffix>)` after it when `taken` holds it, or the
/// first of ` (<suffix> 2)`, ` (<suffix> 3)` and on that is free.
fn landing(name: &str, suffix: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(name) {
        return name.to_string();
    }
    let first = format!("{name} ({suffix})");
    if !taken.contains(&first) {
        return first;
    }
    (2..)
        .map(|n| format!("{name} ({suffix} {n})"))
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or_default()
}

/// Whether the Reminder `s` is past on `today` at the wall clock's
/// minute of the day `minute`: on a day before, or today at a time
/// before this minute.
fn past(s: &Schedule, today: Day, minute: u32) -> bool {
    if s.start < today {
        return true;
    }
    if today < s.start {
        return false;
    }
    s.times
        .first()
        .is_some_and(|time| time.hour * 60 + time.minute < minute)
}

/// The Routine `s` starting on `today` (Q464), its times and rule kept.
fn from_day(s: &Schedule, today: Day) -> Schedule {
    Schedule {
        start: today,
        times: s.times.clone(),
        rule: s.rule.clone(),
    }
}

#[cfg(test)]
mod tests;
