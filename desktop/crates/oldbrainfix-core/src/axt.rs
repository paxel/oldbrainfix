//! AXT files (obf-1144, ADR 0056), as the Dart core's `axt.dart`:
//! Notes, with their Reminders and Routines, as records of the open
//! standard AXT for small apps to exchange typed records, written and
//! read through the library `axt-protocol`. A Note goes as an AXT
//! `Note`, a Note with a Reminder or Routine as a `Task`; a `Note`, a
//! `Task` and an `Appointment` that come in become Notes, every other
//! type is listed as not taken. Both cores write the same records; the
//! file's bytes differ, as the library draws its sync marker and the low
//! bits of each version at random.

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use axt_protocol::examples as ax;
use axt_protocol::{
    AxtNode, EncryptionSeed, NodeConfig, Outcome, OutgoingRecord, RecordChange, RecordResult,
    SigningSeed, Store as AxtStore, TypeDescription, Value,
};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::box_keys::box_secret_of;
use crate::entry::{dart_iso, parse_time};
use crate::notes::{Note, Notes, keys as note_keys};
use crate::places::Places;
use crate::routines::{Routine, Routines, heads_up_keys, keys as routine_keys};
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule, Until, WallClock};
use crate::signing::device_id_from_key;
use crate::store::Store;
use crate::text_format;
use crate::text_tags;
use crate::{Error, Result};

/// The full names of the AXT types Old Brain Fix writes and takes.
pub mod types {
    pub const NOTE: &str = "axt.example.v1.Note";
    pub const TASK: &str = "axt.example.v1.Task";
    pub const APPOINTMENT: &str = "axt.example.v1.Appointment";

    /// The types a file's records are taken from.
    pub const TAKEN: [&str; 3] = [NOTE, TASK, APPOINTMENT];
}

pub mod keys {
    /// On a Note taken from an AXT file: `{"id":…,"sender":…,"type":…,
    /// "version":…}` ([`super::AxtFrom`]). Only what the user's own
    /// Devices wrote counts.
    pub const FROM: &str = "axtFrom";
}

/// A file that is no AXT file this app can read: no Avro container, a
/// signature that does not hold, a header of another format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAnAxtFile;

/// Where a Note taken from an AXT file came from: the record `id` as its
/// `sender` wrote it, the sender's key as hexadecimal text, the record's
/// `type_name` and the `version` taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxtFrom {
    pub sender: String,
    pub id: String,
    pub type_name: String,
    pub version: i64,
}

impl AxtFrom {
    /// Keys sorted, no spaces, the same bytes from both cores.
    pub fn encode(&self) -> String {
        json!({
            "id": self.id,
            "sender": self.sender,
            "type": self.type_name,
            "version": self.version,
        })
        .to_string()
    }

    /// What `raw` holds, or `None` when it is none.
    pub fn parse(raw: Option<&str>) -> Option<AxtFrom> {
        let json: serde_json::Value = serde_json::from_str(raw?).ok()?;
        let sender = json.get("sender")?.as_str()?;
        let key = sender.len() == 64
            && sender
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !key {
            return None;
        }
        Some(AxtFrom {
            sender: sender.to_string(),
            id: json.get("id")?.as_str()?.to_string(),
            type_name: json.get("type")?.as_str()?.to_string(),
            version: json.get("version")?.as_i64()?,
        })
    }
}

/// A Note as a record of an AXT file makes it, before it is taken, as
/// the Dart core's `AxtNoteDraft`: its `text` in `format`, and for a
/// `Task` with a due date or an `Appointment` its Reminder or Routine,
/// the first Occurrence Done or not, and the Heads-ups of an
/// `Appointment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxtNoteDraft {
    pub text: String,
    pub format: String,
    pub schedule: Option<Schedule>,
    /// Whether the first Occurrence of `schedule` is Done.
    pub done: bool,
    /// When it was checked off, or `None` when not known: it is then
    /// taken as checked off when it is taken.
    pub done_at: Option<DateTime<Utc>>,
    /// Minutes before each Occurrence, ascending.
    pub heads_ups: Vec<i64>,
}

/// What a record of a file is to the Notes taken from that sender before
/// (obf-1147), as the Dart core's `AxtRecordKind`, by the record id and
/// the sender's key their [`AxtFrom`] names, as the user's own Devices
/// wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxtRecordKind {
    /// A record no Note was taken from: a `Note`, `Task` or `Appointment`
    /// that came in whole, to become a new Note.
    New,
    /// A later version of a record a Note was taken from: it rewrites the
    /// Note's text and its Reminder or Routine.
    Update,
    /// A later deletion of such a record: it moves the Note
    /// to the trash.
    Deletion,
    /// A record that may not change the Note taken under its id: one of
    /// another sender, one of another type than the Note was taken as, or
    /// one the library refused.
    Refused,
    /// A version of the record no later than the one taken: nothing to do.
    Stale,
    /// Nothing to take: a record of a type not taken, one the library
    /// refused, or a deletion of a record no Note was taken from.
    None,
}

impl AxtRecordKind {
    /// How the fixtures and the Dart core name it.
    pub fn code(self) -> &'static str {
        match self {
            AxtRecordKind::New => "new",
            AxtRecordKind::Update => "update",
            AxtRecordKind::Deletion => "deletion",
            AxtRecordKind::Refused => "refused",
            AxtRecordKind::Stale => "stale",
            AxtRecordKind::None => "none",
        }
    }
}

/// One record of an AXT file, as its last message the library applied
/// leaves it: its `id` as its sender wrote it, its `version`, its
/// `type_name` when it came in whole, and `result`, what the library made
/// of that message (`APPLIED`, or why not). A record of a type this app
/// takes has its `draft`; a deletion has none. `kind` says what it is to
/// the Notes taken before, `note_id` which Note it updates, deletes or
/// may not change, and `kept` whether the user changed that Note since it
/// was taken or last updated, so that nothing changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxtRecord {
    pub id: String,
    pub version: i64,
    pub type_name: Option<String>,
    pub result: String,
    pub draft: Option<AxtNoteDraft>,
    pub kind: AxtRecordKind,
    pub note_id: Option<String>,
    pub kept: bool,
}

impl AxtRecord {
    /// Whether it becomes a new Note when taken: a `Note`, `Task` or
    /// `Appointment` that came in whole, under an id no Note was taken
    /// from.
    pub fn taken(&self) -> bool {
        self.kind == AxtRecordKind::New
    }

    /// Whether taking it updates or deletes the Note it was taken as,
    /// unless the user changed that Note since (`kept`).
    pub fn changes(&self) -> bool {
        matches!(self.kind, AxtRecordKind::Update | AxtRecordKind::Deletion)
    }

    fn classified(&self, kind: AxtRecordKind, note_id: Option<&str>, kept: bool) -> AxtRecord {
        AxtRecord {
            kind,
            note_id: note_id.map(str::to_string),
            kept,
            ..self.clone()
        }
    }
}

/// A Note taken from a record: its id, where it came from, and the entry
/// of the user's own Devices that says so last.
struct Taken {
    note_id: String,
    from: AxtFrom,
    entry: crate::entry::Entry,
}

/// What an AXT file holds, checked as a whole and read, nothing taken
/// yet, as the Dart core's `AxtFile`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxtFile {
    /// The key that signed it, as hexadecimal text.
    pub sender: String,
    /// Its short fingerprint, for the user to compare.
    pub short_fingerprint: String,
    /// The app name its manifest gives.
    pub app_name: Option<String>,
    /// Whether the key is one of the user's own Devices'.
    pub from_own_device: bool,
    /// Each record once, where its last message lies in the file.
    pub records: Vec<AxtRecord>,
}

impl AxtFile {
    /// The records that become new Notes when taken, in the file's order.
    pub fn taken(&self) -> Vec<&AxtRecord> {
        self.records.iter().filter(|r| r.taken()).collect()
    }

    /// The records that update or delete a Note taken before, in the
    /// file's order, those the user changed since among them (`kept`).
    pub fn changes(&self) -> Vec<&AxtRecord> {
        self.records.iter().filter(|r| r.changes()).collect()
    }

    /// The others, in the file's order: of another type, a deletion of a
    /// record never taken, one the library refused, one that may not
    /// change the Note taken under its id, or one no later than it.
    pub fn not_taken(&self) -> Vec<&AxtRecord> {
        self.records
            .iter()
            .filter(|r| !r.taken() && !r.changes())
            .collect()
    }
}

fn axt_error(e: axt_protocol::Error) -> Error {
    Error::Invalid(format!("axt: {e}"))
}

/// The three types as the library reads them: Note, Task, Appointment.
pub fn type_descriptions() -> Result<Vec<TypeDescription>> {
    Ok(vec![
        ax::Note::description().map_err(axt_error)?,
        ax::Task::description().map_err(axt_error)?,
        ax::Appointment::description().map_err(axt_error)?,
    ])
}

fn s(text: &str) -> Value {
    Value::String(text.to_string())
}

fn record(fields: Vec<(&str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn take(type_name: &str, purpose: &str, de: &str) -> Value {
    record(vec![
        ("type", s(type_name)),
        ("min_version", Value::Int(1)),
        ("max_version", Value::Null),
        ("purpose", s(purpose)),
        (
            "purpose_translations",
            Value::Map(BTreeMap::from([("de".to_string(), s(de))])),
        ),
    ])
}

/// The manifest Old Brain Fix writes into every AXT file, without its
/// revision, as the Dart core's `axtManifest`; with `any_type`, the one
/// a file is read with, so that the records of a type not taken are
/// listed by their names.
fn manifest(any_type: bool) -> Value {
    record(vec![
        ("app_name", s("Old Brain Fix")),
        ("app_url", Value::Null),
        ("formats", Value::Array(vec![Value::Int(1)])),
        (
            "takes",
            Value::Array(vec![
                take(
                    types::APPOINTMENT,
                    "An Appointment becomes a Note with a Reminder at its start.",
                    "Ein Termin wird zu einer Notiz mit einer Erinnerung zu seinem Beginn.",
                ),
                take(
                    types::TASK,
                    "A Task becomes a Note with a Reminder or Routine.",
                    "Eine Aufgabe wird zu einer Notiz mit einer Erinnerung oder Routine.",
                ),
                take(
                    types::NOTE,
                    "A Note becomes a Note.",
                    "Eine Notiz wird zu einer Notiz.",
                ),
            ]),
        ),
        (
            "any_type",
            if any_type {
                record(vec![
                    ("purpose", s("Any other type is listed as not taken.")),
                    ("purpose_translations", Value::Map(BTreeMap::new())),
                ])
            } else {
                Value::Null
            },
        ),
        (
            "sends",
            Value::Array(
                [types::NOTE, types::TASK]
                    .into_iter()
                    .map(|t| {
                        record(vec![
                            ("type", s(t)),
                            ("min_version", Value::Int(1)),
                            ("max_version", Value::Int(1)),
                            ("proposals", Value::Boolean(false)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("encryption_key", Value::Null),
        ("relay", Value::Null),
    ])
}

/// The manifest Old Brain Fix writes into every AXT file.
pub fn axt_manifest() -> Value {
    manifest(false)
}

/// The AXT id of the Note `note_id` as a record of `type_name`, as the
/// Dart core's `axtRecordId`: as a `Note`, its 32 hexadecimal digits as a
/// UUID of version 4, the bits of the version and the variant set; an id
/// of another form, and the Note as a record of another type, through
/// SHA-256 first. A record keeps its type for its whole life, so a Note
/// that gains a Reminder goes as a `Task` of an id of its own.
pub fn record_id(note_id: &str, type_name: &str) -> String {
    let digits = note_id.strip_prefix("note:").unwrap_or("");
    let note = type_name == types::NOTE;
    let hex32 = note
        && digits.len() == 32
        && digits
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    let seed = if note {
        format!("oldbrainfix:axt:{note_id}")
    } else {
        format!("oldbrainfix:axt:{type_name}:{note_id}")
    };
    let mut bytes: Vec<u8> = match hex::decode(digits) {
        Ok(bytes) if hex32 => bytes,
        _ => Sha256::digest(seed.as_bytes())
            .iter()
            .take(16)
            .copied()
            .collect(),
    };
    if let Some(b) = bytes.get_mut(6) {
        *b = 0x40 | (*b & 0x0f);
    }
    if let Some(b) = bytes.get_mut(8) {
        *b = 0x80 | (*b & 0x3f);
    }
    let part = |from: usize, to: usize| bytes.get(from..to).map(hex::encode).unwrap_or_default();
    format!(
        "{}-{}-{}-{}-{}",
        part(0, 4),
        part(4, 6),
        part(6, 8),
        part(8, 10),
        part(10, 16)
    )
}

/// The key-value store the library keeps its state in for one file: in
/// memory only, as nothing of it outlives the file (ADR 0056).
#[derive(Default)]
struct MemoryStore {
    values: BTreeMap<String, Vec<u8>>,
}

impl AxtStore for MemoryStore {
    type Error = Infallible;

    fn read(&self, key: &str) -> std::result::Result<Option<Vec<u8>>, Infallible> {
        Ok(self.values.get(key).cloned())
    }

    fn write(&mut self, key: &str, value: &[u8]) -> std::result::Result<(), Infallible> {
        self.values.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, key: &str) -> std::result::Result<(), Infallible> {
        self.values.remove(key);
        Ok(())
    }

    fn list(&self, prefix: &str) -> std::result::Result<Vec<String>, Infallible> {
        Ok(self
            .values
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }

    fn transaction<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> std::result::Result<T, axt_protocol::Error>,
    ) -> std::result::Result<T, axt_protocol::Error> {
        let before = self.values.clone();
        let out = body(self);
        if out.is_err() {
            self.values = before;
        }
        out
    }
}

fn system_time(at: DateTime<Utc>) -> SystemTime {
    match u64::try_from(at.timestamp_millis()) {
        Ok(ms) => UNIX_EPOCH + StdDuration::from_millis(ms),
        Err(_) => UNIX_EPOCH,
    }
}

impl Store {
    /// The AXT file of the Notes `note_ids`, in their order, signed with
    /// this Device's key, as the Dart core's `AxtFiles.write`: each as an
    /// AXT `Note`, or as a `Task` when it has a Reminder or Routine. An
    /// id that is no Note, or one in the trash or merged into another, is
    /// left out. Its record id is [`record_id`], its version a stamp of
    /// the store's now, above every version a file of it had before.
    pub fn axt_write(&self, note_ids: &[String]) -> Result<Vec<u8>> {
        let descriptions = type_descriptions()?;
        let notes = Notes::new(self);
        let by_note = self.axt_routines_by_note()?;
        let mut values = Vec::new();
        for id in note_ids {
            let Some(note) = notes.by_id(id)? else {
                continue;
            };
            if note.is_trashed() || note.is_merged() {
                continue;
            }
            let routines = by_note.get(id).map(Vec::as_slice).unwrap_or(&[]);
            let changed = self.axt_latest_change(&note, routines)?;
            let (type_name, value) = record_of(&note, routines, changed);
            values.push((record_id(id, type_name), type_name, value));
        }
        let at = self.now();
        let seed = *self.signing_key()?.seed();
        let mut config = NodeConfig::new(manifest(false));
        config.readers = descriptions.clone();
        config.clock = Box::new(move || system_time(at));
        let mut node = AxtNode::open(
            MemoryStore::default(),
            SigningSeed::new(seed),
            EncryptionSeed::new(box_secret_of(&seed)),
            config,
        );
        let mut records = Vec::new();
        for (id, type_name, value) in values {
            let description = descriptions
                .iter()
                .find(|d| d.name() == type_name)
                .ok_or_else(|| Error::Invalid(format!("axt: no type {type_name}")))?;
            records.push(OutgoingRecord::new(id, description, value));
        }
        node.write_file(&records, None, false).map_err(axt_error)
    }

    /// The AXT type and value the Note `note_id` goes as
    /// ([`Store::axt_write`]); `None` when it is no Note in the lists.
    pub fn axt_record_of(&self, note_id: &str) -> Result<Option<(&'static str, Value)>> {
        let Some(note) = Notes::new(self).by_id(note_id)? else {
            return Ok(None);
        };
        if note.is_trashed() || note.is_merged() {
            return Ok(None);
        }
        let by_note = self.axt_routines_by_note()?;
        let routines = by_note.get(note_id).map(Vec::as_slice).unwrap_or(&[]);
        let changed = self.axt_latest_change(&note, routines)?;
        Ok(Some(record_of(&note, routines, changed)))
    }

    /// The Routines and Reminders of each Note, those of the Notes merged
    /// into it included, read once.
    fn axt_routines_by_note(&self) -> Result<HashMap<String, Vec<Routine>>> {
        let mut by_note: HashMap<String, Vec<Routine>> = HashMap::new();
        for (routine, note) in Routines::new(self).all_with_notes()? {
            if let Some(note) = note {
                by_note.entry(note.id).or_default().push(routine);
            }
        }
        Ok(by_note)
    }

    /// The date of the latest entry of `note` and of its `routines`.
    fn axt_latest_change(&self, note: &Note, routines: &[Routine]) -> Result<DateTime<Utc>> {
        let mut latest = note.captured;
        let ids = std::iter::once(note.id.as_str()).chain(routines.iter().map(|r| r.id.as_str()));
        for id in ids {
            for e in self.logged_history(id)? {
                let date = parse_time(&e.date)?;
                if date > latest {
                    latest = date;
                }
            }
        }
        Ok(latest)
    }

    /// Reads the AXT file `bytes`, as the Dart core's `AxtFiles.read`:
    /// checks it as a whole, signatures and all, and reads each of its
    /// records, applying nothing. Times in a zone are put on this
    /// Device's wall clock by `wall_clock`. [`NotAnAxtFile`] for a file
    /// the library rejects as a whole.
    pub fn axt_read(
        &self,
        bytes: &[u8],
        wall_clock: &WallClock,
    ) -> Result<std::result::Result<AxtFile, NotAnAxtFile>> {
        let mut config = NodeConfig::new(manifest(true));
        config.readers = type_descriptions()?;
        let now = self.now();
        config.clock = Box::new(move || system_time(now));
        // A key of its own: nothing is signed with it, and no file can
        // pass for one of this reader's own.
        let seed = crate::keys::random_bytes::<32>()?;
        let mut node = AxtNode::open(
            MemoryStore::default(),
            SigningSeed::new(seed),
            EncryptionSeed::new(box_secret_of(&seed)),
            config,
        );
        let Ok(preview) = node.inspect(bytes) else {
            return Ok(Err(NotAnAxtFile));
        };
        let mut changes: Vec<RecordChange> = Vec::new();
        let result = node
            .apply(
                &preview,
                true,
                |_, change| {
                    changes.push(change);
                    Ok(())
                },
                |_, _, _| Ok(None),
            )
            .map_err(axt_error)?;
        // Each record once, as the file leaves it: a later message of the
        // same id that the library applied replaces what came before, and
        // moves to where it lies.
        let mut records: Vec<AxtRecord> = Vec::new();
        let mut handed = changes.iter();
        for r in result.records {
            let result = r.result.as_str().to_string();
            let changed = matches!(
                r.result,
                RecordResult::Outcome(Outcome::Applied | Outcome::Merged | Outcome::Withdrawn)
            );
            let change = if changed { handed.next() } else { None };
            let held = records.iter().position(|h| h.id == r.id);
            if !changed && held.is_some() {
                continue;
            }
            if let Some(at) = held {
                records.remove(at);
            }
            let (type_name, draft) = match change {
                Some(RecordChange {
                    id,
                    type_name: Some(t),
                    value: Some(value),
                    ..
                }) if *id == r.id => (Some(t.clone()), draft_of(t, value, wall_clock)),
                _ => (None, None),
            };
            records.push(AxtRecord {
                id: r.id,
                version: r.version,
                type_name,
                result,
                draft,
                kind: AxtRecordKind::None,
                note_id: None,
                kept: false,
            });
        }
        let key = hex::decode(&preview.sender).unwrap_or_default();
        Ok(Ok(AxtFile {
            from_own_device: self.written_by_own(&device_id_from_key(&key))?,
            records: self.axt_classified(&preview.sender, &records)?,
            sender: preview.sender,
            short_fingerprint: preview.short_fingerprint,
            app_name: preview.app_name,
        }))
    }

    /// `records` of a file of `sender`, each with what it is to the Notes
    /// taken before, as the Dart core's `_classified`: the Note taken
    /// under its id from the same sender, by the [`AxtFrom`] the user's
    /// own Devices wrote last, the one of the highest version where two
    /// were, the first by id where they tie. A record from another
    /// sender under an id taken may change nothing.
    fn axt_classified(&self, sender: &str, records: &[AxtRecord]) -> Result<Vec<AxtRecord>> {
        let mut own = HashMap::new();
        let taken = self.axt_taken_by_record(&mut own)?;
        let mut out = Vec::with_capacity(records.len());
        for r in records {
            let held = taken.get(&r.id).map(Vec::as_slice).unwrap_or(&[]);
            out.push(self.axt_classify(r, sender, held, &mut own)?);
        }
        Ok(out)
    }

    /// Every Note taken from an AXT file, by the record id it came from,
    /// in the order of the Notes' ids.
    fn axt_taken_by_record(
        &self,
        own: &mut HashMap<String, bool>,
    ) -> Result<HashMap<String, Vec<Taken>>> {
        let mut ids = Vec::new();
        for (entity, _, device) in self.writers_of(&[keys::FROM])? {
            if self.written_by_own_in(&device, own)? {
                ids.push(entity);
            }
        }
        ids.sort();
        ids.dedup();
        let histories = self.logged_histories_of(ids.iter().map(String::as_str))?;
        let mut by_record: HashMap<String, Vec<Taken>> = HashMap::new();
        for id in &ids {
            let mut held: Option<&crate::entry::Entry> = None;
            for e in histories.get(id).map(Vec::as_slice).unwrap_or(&[]) {
                if e.field != keys::FROM || !self.written_by_own_in(&e.device, own)? {
                    continue;
                }
                if held.is_none_or(|h| Store::latest_wins(e, h) == std::cmp::Ordering::Greater) {
                    held = Some(e);
                }
            }
            let Some(entry) = held else {
                continue;
            };
            if let Some(from) = AxtFrom::parse(entry.value.as_deref()) {
                by_record.entry(from.id.clone()).or_default().push(Taken {
                    note_id: id.clone(),
                    from,
                    entry: entry.clone(),
                });
            }
        }
        Ok(by_record)
    }

    fn axt_classify(
        &self,
        r: &AxtRecord,
        sender: &str,
        taken: &[Taken],
        own: &mut HashMap<String, bool>,
    ) -> Result<AxtRecord> {
        let Some(first) = taken.first() else {
            let kind = if r.draft.is_some() {
                AxtRecordKind::New
            } else {
                AxtRecordKind::None
            };
            return Ok(r.classified(kind, None, false));
        };
        let mut target: Option<&Taken> = None;
        for t in taken {
            if t.from.sender != sender {
                continue;
            }
            if target.is_none_or(|h| t.from.version > h.from.version) {
                target = Some(t);
            }
        }
        let Some(target) = target else {
            return Ok(r.classified(AxtRecordKind::Refused, Some(&first.note_id), false));
        };
        let note_id = Some(target.note_id.as_str());
        let deletion =
            r.type_name.is_none() && matches!(r.result.as_str(), "APPLIED" | "WITHDRAWN");
        let data =
            r.draft.is_some() && r.type_name.as_deref() == Some(target.from.type_name.as_str());
        if !deletion && !data {
            return Ok(r.classified(AxtRecordKind::Refused, note_id, false));
        }
        if r.version <= target.from.version {
            return Ok(r.classified(AxtRecordKind::Stale, note_id, false));
        }
        let kind = if deletion {
            AxtRecordKind::Deletion
        } else {
            AxtRecordKind::Update
        };
        let kept = self.axt_changed_since(target, own)?;
        Ok(r.classified(kind, note_id, kept))
    }

    /// Whether the user changed the Note `taken` since it was taken or
    /// last updated, as the Dart core's `_changedSince`: an entry of one
    /// of their own Devices after the [`AxtFrom`] that says so, on the
    /// Note, on a Routine, Reminder or Section that ever was the Note's,
    /// on their Heads-ups, or on a Note ever merged into it. What another
    /// person's Device wrote is no change of the user's.
    fn axt_changed_since(&self, taken: &Taken, own: &mut HashMap<String, bool>) -> Result<bool> {
        let note_id = taken.note_id.as_str();
        let mut linked = self.ever_with(routine_keys::NOTE, note_id)?;
        linked.extend(self.ever_with(note_keys::MERGED, note_id)?);
        let mut entities: Vec<String> = vec![note_id.to_string()];
        for id in &linked {
            entities.push(id.clone());
            entities.push(heads_up_keys::of(id));
        }
        let histories = self.logged_histories_of(entities.iter().map(String::as_str))?;
        for (entity, entries) in &histories {
            for e in entries {
                if entity == note_id && e.field == keys::FROM {
                    continue;
                }
                if !self.written_by_own_in(&e.device, own)? {
                    continue;
                }
                if Store::latest_wins(e, &taken.entry) == std::cmp::Ordering::Greater {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Takes the records of `file` whose ids are among `ids`, as the Dart
    /// core's `AxtFiles.take`: in the file's order, all in one batch, each
    /// as it is to the Notes taken before now, whatever `file` said when
    /// it was read. A new one becomes a new Note of this Device's,
    /// captured now, filed into the Area or Topic `place` or left in
    /// Unsorted with `None`, with its Reminder or Routine written, not
    /// spawned, its first Occurrence Done when the record says so, its
    /// Heads-ups, and where it came from ([`AxtFrom`]); an update
    /// rewrites the text of the Note taken from it and its Reminder or
    /// Routine to the record's, bringing it back from the trash a
    /// deletion put it in; a deletion moves that Note to the trash with
    /// its Reminders and Routines; one whose Note the user changed since
    /// (`kept`), one refused and one stale change nothing. An update or a
    /// deletion keeps the record's version in [`AxtFrom`], written last.
    /// Returns the new Notes' ids. Refused before anything is written for
    /// a place that is no Area or Topic, where this Device may not write,
    /// or may not make Reminders and Routines, or may not change a Note to
    /// update or delete.
    pub fn axt_take(
        &self,
        file: &AxtFile,
        ids: &[String],
        place: Option<&str>,
    ) -> Result<Vec<String>> {
        let records: Vec<AxtRecord> = self
            .axt_classified(&file.sender, &file.records)?
            .into_iter()
            .filter(|r| ids.contains(&r.id))
            .collect();
        if let Some(place) = place {
            if !Places::new(self).is_place(place)? {
                return Err(Error::Invalid(format!("{place} is no Area or Topic")));
            }
            if !self.may_write_in(place)? {
                return Err(Error::Invalid(format!(
                    "no role in the Shared Area of {place}"
                )));
            }
        }
        let area = match place {
            Some(place) => self.area_of(place)?,
            None => None,
        };
        let scheduled = records
            .iter()
            .any(|r| r.taken() && r.draft.as_ref().is_some_and(|d| d.schedule.is_some()));
        if scheduled && !self.may_manage(area.as_deref())? {
            return Err(Error::Invalid(format!(
                "not a Manager of the Area of {}",
                place.unwrap_or_default()
            )));
        }
        for r in &records {
            let (true, false, Some(note_id)) = (r.changes(), r.kept, r.note_id.as_deref()) else {
                continue;
            };
            if !self.may_change(note_id)? {
                return Err(Error::Invalid(format!("{note_id} may not be changed here")));
            }
            let schedules = (r.kind == AxtRecordKind::Update
                && r.draft.as_ref().is_some_and(|d| d.schedule.is_some()))
                || !self.axt_routines_of(note_id)?.is_empty();
            if schedules && !self.may_manage(self.area_of(note_id)?.as_deref())? {
                return Err(Error::Invalid(format!(
                    "not a Manager of the Area of {note_id}"
                )));
            }
        }
        self.batch(|| {
            let notes = Notes::new(self);
            let mut made = Vec::new();
            for record in &records {
                let note_id = record.note_id.as_deref();
                match (record.kind, &record.draft, &record.type_name, note_id) {
                    (AxtRecordKind::New, Some(draft), Some(type_name), _) => {
                        let note = notes.capture_as(&draft.text, &draft.format, place, None)?;
                        if let Some(schedule) = &draft.schedule {
                            self.axt_write_routine(&note.id, schedule, draft)?;
                        }
                        // Last, so that what the record wrote lies before
                        // it and is no change of the user's.
                        let from = AxtFrom {
                            sender: file.sender.clone(),
                            id: record.id.clone(),
                            type_name: type_name.clone(),
                            version: record.version,
                        };
                        self.append(&note.id, keys::FROM, Some(&from.encode()), None)?;
                        made.push(note.id);
                    }
                    (AxtRecordKind::Update, Some(draft), _, Some(note_id)) if !record.kept => {
                        self.axt_update(note_id, draft, record.version)?;
                    }
                    (AxtRecordKind::Deletion, _, _, Some(note_id)) if !record.kept => {
                        self.axt_delete(note_id, record.version)?;
                    }
                    _ => {}
                }
            }
            Ok(made)
        })
    }

    /// The Reminders and Routines of the Note `note_id` not in the trash,
    /// by id, as the Dart core's `_routinesOf`: found by what ever named
    /// the Note rather than by reading every Routine.
    fn axt_routines_of(&self, note_id: &str) -> Result<Vec<Routine>> {
        let routines = Routines::new(self);
        let mut ids: Vec<String> = self
            .ever_with(routine_keys::NOTE, note_id)?
            .into_iter()
            .collect();
        ids.sort();
        let mut live = Vec::new();
        for id in ids {
            if let Some(r) = routines.by_id(&id)?
                && r.note_id == note_id
                && !r.is_trashed()
            {
                live.push(r);
            }
        }
        Ok(live)
    }

    /// Rewrites the Note `note_id` to `draft`, the record of `version`, as
    /// the Dart core's `_update`: its text when it differs, its first
    /// Reminder or Routine to the record's schedule, Heads-ups and first
    /// Occurrence Done or not, one written where it had none, every other
    /// one in the trash.
    fn axt_update(&self, note_id: &str, draft: &AxtNoteDraft, version: i64) -> Result<()> {
        let notes = Notes::new(self);
        let routines = Routines::new(self);
        let (Some(note), Some(from)) = (notes.by_id(note_id)?, self.axt_from(note_id)?) else {
            return Ok(());
        };
        if note.is_trashed() {
            routines.restore_note(note_id)?;
        }
        let text = draft.text.trim();
        let format = match note.text_format.as_str() {
            text_format::MARKDOWN | text_format::CODE => note.text_format.as_str(),
            _ => draft.format.as_str(),
        };
        if !text.is_empty() && (note.shown_text().trim() != text || note.text_format != format) {
            notes.edit_as(note_id, text, &draft.format)?;
        }
        let live = self.axt_routines_of(note_id)?;
        match (&draft.schedule, live.split_first()) {
            (None, _) => {
                for r in &live {
                    routines.trash(&r.id)?;
                }
            }
            (Some(schedule), None) => self.axt_write_routine(note_id, schedule, draft)?,
            (Some(schedule), Some((routine, others))) => {
                routines.reschedule(&routine.id, schedule)?;
                routines.set_heads_ups(&routine.id, &draft.heads_ups)?;
                if let Some(first) = schedule.next(schedule.start, 1).first() {
                    let done_at = routine.done_at(first);
                    if draft.done && done_at.is_none() {
                        let at = draft.done_at.unwrap_or_else(|| self.now());
                        self.append(
                            &routine.id,
                            &routine_keys::done(first),
                            Some(&dart_iso(at)),
                            None,
                        )?;
                    } else if !draft.done && done_at.is_some() {
                        self.append(&routine.id, &routine_keys::done(first), None, None)?;
                    }
                }
                for other in others {
                    routines.trash(&other.id)?;
                }
            }
        }
        self.axt_bump(note_id, &from, version)
    }

    /// Moves the Note `note_id` to the trash with its Reminders and
    /// Routines, as [`Routines::trash_note`] does, for the record's
    /// deletion of `version`, as the Dart core's `_delete`; one in the
    /// trash already stays there.
    fn axt_delete(&self, note_id: &str, version: i64) -> Result<()> {
        let notes = Notes::new(self);
        let (Some(note), Some(from)) = (notes.by_id(note_id)?, self.axt_from(note_id)?) else {
            return Ok(());
        };
        if !note.is_trashed() {
            let live = self.axt_routines_of(note_id)?;
            notes.trash(note_id)?;
            // The same moment as the Note's, so both are purged together.
            let at = match self.current(note_id, note_keys::TRASHED)? {
                Some(at) => at,
                None => dart_iso(self.now()),
            };
            for r in &live {
                self.append(&r.id, routine_keys::TRASHED, Some(&at), None)?;
                self.append(&r.id, routine_keys::TRASHED_WITH, Some(note_id), None)?;
            }
        }
        self.axt_bump(note_id, &from, version)
    }

    /// The [`AxtFrom`] of the Note `note_id` at `version`, written last,
    /// so that what the record changed lies before it.
    fn axt_bump(&self, note_id: &str, from: &AxtFrom, version: i64) -> Result<()> {
        let from = AxtFrom {
            version,
            ..from.clone()
        };
        self.append(note_id, keys::FROM, Some(&from.encode()), None)
            .map(drop)
    }

    /// A Reminder or Routine of the Note `note_id` on `schedule`, written,
    /// not spawned, as a Sent Item's is.
    fn axt_write_routine(
        &self,
        note_id: &str,
        schedule: &Schedule,
        draft: &AxtNoteDraft,
    ) -> Result<()> {
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
        self.append(
            &id,
            routine_keys::TIMES,
            Some(&crate::sent::times(schedule)),
            None,
        )?;
        if let Some(rule) = &schedule.rule {
            self.append(&id, routine_keys::RULE, Some(&rule.format()), None)?;
        }
        if let Some(heads_ups) = heads_up_keys::write(&draft.heads_ups) {
            self.append(
                &heads_up_keys::of(&id),
                heads_up_keys::HEADS_UPS,
                Some(&heads_ups),
                None,
            )?;
        }
        let first = schedule.next(schedule.start, 1).first().copied();
        if let (true, Some(first)) = (draft.done, first) {
            let at = draft.done_at.unwrap_or_else(|| self.now());
            self.append(&id, &routine_keys::done(&first), Some(&dart_iso(at)), None)?;
        }
        Ok(())
    }

    /// Where the Note `note_id` came from over AXT, as the user's own
    /// Devices last wrote it (`own_current`): another person's Device in a
    /// Shared Area can neither set nor change it. `None` when it came from
    /// no AXT file.
    pub fn axt_from(&self, note_id: &str) -> Result<Option<AxtFrom>> {
        let raw = self.own_current(note_id, keys::FROM, &mut HashMap::new())?;
        Ok(AxtFrom::parse(raw.as_deref()))
    }
}

fn record_of(note: &Note, routines: &[Routine], changed: DateTime<Utc>) -> (&'static str, Value) {
    let shown = note.shown_text();
    let visible = note.visible_text();
    let title = first_line(&visible);
    let Some(routine) = chosen(routines) else {
        let value = ax::Note {
            title,
            text: shown.clone(),
            format: if note.text_format == text_format::MARKDOWN {
                ax::NoteFormat::Markdown
            } else {
                ax::NoteFormat::Plain
            },
            created: Some(instant(note.captured)),
            updated: Some(instant(changed)),
            tags: tags_of(&shown),
            relations: Vec::new(),
            extensions: Vec::new(),
        };
        return (types::NOTE, value.into());
    };
    let schedule = &routine.schedule;
    let first = schedule.next(schedule.start, 1).first().copied();
    let done_at = first.and_then(|f| routine.done_at(&f));
    let value = ax::Task {
        title,
        notes: rest(&visible),
        due: first.map(floating),
        due_all_day: first.is_some_and(|f| f.time.is_none()),
        repeat: first.and_then(|f| repeat(schedule, f)),
        done: done_at.is_some(),
        done_at: done_at.map(instant),
        priority: ax::TaskPriority::None,
        relations: Vec::new(),
        extensions: Vec::new(),
    };
    (types::TASK, value.into())
}

/// The Routine or Reminder of a Note that goes as its `Task`: the one
/// whose first Occurrence comes first, by id when two start alike; one
/// without any Occurrence last.
fn chosen(routines: &[Routine]) -> Option<&Routine> {
    let first = |r: &Routine| r.schedule.next(r.schedule.start, 1).first().copied();
    routines.iter().min_by(|a, b| {
        let (fa, fb) = (first(a), first(b));
        let by_first = match (fa, fb) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(x), Some(y)) => x.cmp(&y),
        };
        by_first.then_with(|| a.id.cmp(&b.id))
    })
}

/// The RRULE of a `Task` due at `first` on `schedule`, as the Dart
/// core's `_repeat`.
fn repeat(schedule: &Schedule, first: Occurrence) -> Option<String> {
    let rule = schedule.rule.as_ref()?;
    let mut count = rule.count;
    let mut until = rule.until;
    let mut by_hour = rule.by_hour.clone();
    let mut by_minute = rule.by_minute.clone();
    let times = schedule.day_times();
    if rule.times().is_none() && times.len() > 1 {
        let mut hours: Vec<i64> = times.iter().map(|t| i64::from(t.hour)).collect();
        hours.sort_unstable();
        hours.dedup();
        let mut minutes: Vec<i64> = times.iter().map(|t| i64::from(t.minute)).collect();
        minutes.sort_unstable();
        minutes.dedup();
        if hours.len().saturating_mul(minutes.len()) == times.len() {
            by_hour = hours;
            by_minute = minutes;
        } else if let Some(n) = count {
            let n = usize::try_from(n).unwrap_or(0);
            if let Some(last) = schedule.next(first.day, n).last() {
                count = None;
                until = Some(Until {
                    day: last.day,
                    time: last.time,
                });
            }
        }
    }
    let until = until.map(|u| Until {
        day: u.day,
        time: first.time.map(|_| {
            u.time.unwrap_or(ClockTime {
                hour: 23,
                minute: 59,
            })
        }),
    });
    Some(
        Recurrence {
            frequency: rule.frequency,
            interval: rule.interval,
            by_day: rule.by_day.clone(),
            by_month_day: rule.by_month_day.clone(),
            by_month: rule.by_month.clone(),
            by_hour,
            by_minute,
            count,
            until,
            week_start: rule.week_start,
        }
        .format(),
    )
}

/// The first line of `text`, trimmed: a record's title.
fn first_line(text: &str) -> String {
    text.trim()
        .split('\n')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// The lines of `text` after its first, trimmed: a `Task`'s notes.
fn rest(text: &str) -> String {
    text.trim()
        .split('\n')
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// The Tags of `text` without their `#`, each once whatever its case, in
/// their order, as written.
fn tags_of(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut tags = Vec::new();
    for t in text_tags::find(text) {
        if seen.insert(t.tag.clone())
            && let Some(name) = text.get(t.start.saturating_add(1)..t.end)
        {
            tags.push(name.to_string());
        }
    }
    tags
}

fn instant(at: DateTime<Utc>) -> ax::AxtDateTime {
    let int = |n: u32| i32::try_from(n).unwrap_or(0);
    ax::AxtDateTime {
        year: at.year(),
        month: int(at.month()),
        day: int(at.day()),
        hour: int(at.hour()),
        minute: int(at.minute()),
        second: int(at.second()),
        millisecond: int(at.timestamp_subsec_millis()),
        zone: Some("Etc/UTC".to_string()),
        offset_seconds: Some(0),
        instant_ms: Some(at.timestamp_millis()),
    }
}

fn floating(o: Occurrence) -> ax::AxtDateTime {
    let int = |n: u32| i32::try_from(n).unwrap_or(0);
    ax::AxtDateTime {
        year: i32::try_from(o.day.year).unwrap_or(0),
        month: int(o.day.month),
        day: int(o.day.day),
        hour: int(o.time.map_or(0, |t| t.hour)),
        minute: int(o.time.map_or(0, |t| t.minute)),
        second: 0,
        millisecond: 0,
        zone: None,
        offset_seconds: None,
        instant_ms: None,
    }
}

/// The text of the field `name` of `value`, its line breaks as `\n`;
/// empty when it is none.
fn text_of(value: &Value, name: &str) -> String {
    match value.field(name) {
        Some(Value::String(s)) => s.replace("\r\n", "\n").replace('\r', "\n"),
        _ => String::new(),
    }
}

fn joined(parts: &[String]) -> String {
    parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The Note a record of `type_name` with `value` makes, as the Dart
/// core's `_draft`.
fn draft_of(type_name: &str, value: &Value, wall_clock: &WallClock) -> Option<AxtNoteDraft> {
    let plain = |text: String| AxtNoteDraft {
        text,
        format: text_format::PLAIN.to_string(),
        schedule: None,
        done: false,
        done_at: None,
        heads_ups: Vec::new(),
    };
    match type_name {
        types::NOTE => {
            let markdown = matches!(value.field("format"), Some(Value::Enum { symbol, .. }) if symbol == "MARKDOWN");
            let format = if markdown {
                text_format::MARKDOWN
            } else {
                text_format::PLAIN
            };
            let title = text_of(value, "title").trim().to_string();
            let mut body = text_of(value, "text");
            if !title.is_empty() && first_line(&text_format::visible(format, &body)) != title {
                body = if body.trim().is_empty() {
                    title
                } else {
                    format!("{title}\n{body}")
                };
            }
            let tags = missing_tags(value.field("tags"), &body);
            if !tags.is_empty() {
                body = if body.is_empty() {
                    tags
                } else {
                    format!("{body}\n{tags}")
                };
            }
            Some(AxtNoteDraft {
                format: format.to_string(),
                ..plain(body)
            })
        }
        types::TASK => {
            let body = joined(&[text_of(value, "title"), text_of(value, "notes")]);
            let Some(due @ Value::Record(_)) = value.field("due") else {
                return Some(plain(body));
            };
            let all_day = matches!(value.field("due_all_day"), Some(Value::Boolean(true)));
            let start = wall(due, wall_clock, all_day);
            let rule = match value.field("repeat") {
                Some(Value::String(r)) => Recurrence::parse(r).ok(),
                _ => None,
            };
            let times: Vec<ClockTime> = start.time.into_iter().collect();
            let schedule = Schedule::new(start.day, &times, rule)?;
            let done = matches!(value.field("done"), Some(Value::Boolean(true)));
            Some(AxtNoteDraft {
                schedule: Some(schedule),
                done,
                done_at: if done {
                    value.field("done_at").and_then(instant_of)
                } else {
                    None
                },
                ..plain(body)
            })
        }
        types::APPOINTMENT => {
            let body = joined(&[
                text_of(value, "title"),
                text_of(value, "location"),
                text_of(value, "notes"),
            ]);
            let Some(start @ Value::Record(_)) = value.field("start") else {
                return Some(plain(body));
            };
            let all_day = matches!(value.field("all_day"), Some(Value::Boolean(true)));
            let at = wall(start, wall_clock, all_day);
            let times: Vec<ClockTime> = at.time.into_iter().collect();
            Some(AxtNoteDraft {
                schedule: Schedule::new(at.day, &times, None),
                heads_ups: heads_ups(value.field("reminders"), all_day),
                ..plain(body)
            })
        }
        _ => None,
    }
}

/// The tags of `tags` ([`tag_of`]) not in `body` yet, each once whatever
/// its case, in their order, apart by spaces.
fn missing_tags(tags: Option<&Value>, body: &str) -> String {
    let Some(Value::Array(tags)) = tags else {
        return String::new();
    };
    let mut seen: std::collections::HashSet<String> =
        text_tags::find(body).into_iter().map(|t| t.tag).collect();
    let mut out = Vec::new();
    for name in tags {
        if let Value::String(name) = name
            && let Some(tag) = tag_of(name)
            && seen.insert(tag.to_lowercase())
        {
            out.push(tag);
        }
    }
    out.join(" ")
}

/// The tag a record's tag `name` is in a text: each part between `/` as
/// [`text_tags::from_label`] makes it, nested again by `/`; `None` when
/// nothing is left.
fn tag_of(name: &str) -> Option<String> {
    let parts: Vec<String> = name
        .split('/')
        .filter_map(text_tags::from_label)
        .map(|t| t.trim_start_matches('#').to_string())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(format!("#{}", parts.join("/")))
    }
}

fn int_field(value: &Value, name: &str) -> i64 {
    match value.field(name) {
        Some(Value::Int(n)) => i64::from(*n),
        Some(Value::Long(n)) => *n,
        _ => 0,
    }
}

/// The Occurrence an AXT date and time `value` is on this Device's wall
/// clock, as the Dart core's `_wall`.
fn wall(value: &Value, wall_clock: &WallClock, all_day: bool) -> Occurrence {
    let day = Day::new(
        int_field(value, "year"),
        int_field(value, "month"),
        int_field(value, "day"),
    );
    if all_day {
        return Occurrence::new(day, None);
    }
    if let (Some(Value::String(_)), Some(Value::Long(ms))) =
        (value.field("zone"), value.field("instant_ms"))
        && let Some(at) = DateTime::<Utc>::from_timestamp_millis(*ms)
    {
        let w = wall_clock(at);
        return Occurrence::new(
            Day::new(
                i64::from(w.year()),
                i64::from(w.month()),
                i64::from(w.day()),
            ),
            ClockTime::new(w.hour(), w.minute()),
        );
    }
    let clock = |n: i64| u32::try_from(n).unwrap_or(0);
    Occurrence::new(
        day,
        ClockTime::new(
            clock(int_field(value, "hour")),
            clock(int_field(value, "minute")),
        ),
    )
}

/// The instant of a zoned AXT date and time `value`; `None` for a
/// floating one, which names none.
fn instant_of(value: &Value) -> Option<DateTime<Utc>> {
    match (value.field("zone"), value.field("instant_ms")) {
        (Some(Value::String(_)), Some(Value::Long(ms))) => DateTime::from_timestamp_millis(*ms),
        _ => None,
    }
}

/// The Heads-ups of an `Appointment`'s `reminders`, as the Dart core's
/// `_headsUps`.
fn heads_ups(reminders: Option<&Value>, all_day: bool) -> Vec<i64> {
    let Some(Value::Array(reminders)) = reminders else {
        return Vec::new();
    };
    let mut minutes = Vec::new();
    for r in reminders {
        if int_field(r, "months") != 0 {
            continue;
        }
        let total = int_field(r, "days")
            .saturating_mul(86_400_000)
            .saturating_add(int_field(r, "millis"));
        if total >= 0 || total % 60_000 != 0 {
            continue;
        }
        let before = total.saturating_neg() / 60_000;
        if all_day && before % 1440 != 0 {
            continue;
        }
        minutes.push(before);
    }
    heads_up_keys::read(heads_up_keys::write(&minutes).as_deref())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod update_tests;
