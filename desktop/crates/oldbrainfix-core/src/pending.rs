//! Pending Changes (CONTEXT.md, ADR 0022, #111), as the Dart core's
//! `pending.dart`: changes the user sees as if they were stored, which
//! reach the log only when accepted.
//!
//! A pending set lies in a file of its own next to the store
//! (`pending/<id>.json`), so it survives a restart and the decision waits
//! on the next start. It holds two kinds of change:
//!
//! - drafts this Device makes ([`PendingDraft`]): no dseq, no signature;
//!   Accept writes them as this Device's entries, numbered and signed
//!   only then, so its sequence never has holes;
//! - entries other Devices wrote and signed ([`PendingEntry`]), checked
//!   when they come in and taken in at Accept as any import is.
//!
//! A [`PendingSet::shown`] set is read through: every read of the store
//! sees it on top of the log, but nothing that is passed on, numbered,
//! trusted or deleted does. Decline deletes the file; the log never saw
//! anything.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};

use crate::blocks::wire_kept_from_blocked;
use crate::conflicts::{conflict_unit, conflict_unit_fields};
use crate::entry::{Entry, WireEntry, dart_iso, parse_time};
use crate::shared::{JudgedLog, ROLE_PREFIX, judge_log};
use crate::signing::{ImportReport, KeyRecord, entry_bytes_with, verify_signature};
use crate::store::{ACCEPTED_PENDING_PREFIX, Store, entry_of};
use crate::{Error, Result};

/// A change this Device makes, unsigned until Accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDraft {
    pub entity: String,
    pub field: String,
    pub value: Option<String>,
    /// The date the entry is written with; `None` for `made`.
    pub date: Option<DateTime<Utc>>,
    /// When the draft was made.
    pub made: DateTime<Utc>,
    /// The log's mark ([`Store::current_seq`]) when it was made: an edit
    /// one of the user's own Devices wrote after it wins at Accept.
    pub mark: i64,
}

impl PendingDraft {
    /// The draft as the reads show it: this Device's entry `dseq`, below
    /// zero, so it is numbered nowhere, by `author`.
    pub(crate) fn as_entry(&self, device: &str, author: &str, dseq: i64) -> Entry {
        let date = self.date.unwrap_or(self.made);
        Entry {
            seq: 0,
            device: device.to_string(),
            dseq,
            entity: self.entity.clone(),
            field: self.field.clone(),
            value: self.value.clone(),
            date: dart_iso(date),
            author: author.to_string(),
            recorded: dart_iso(self.made),
            sig: None,
            ctx: None,
            extra: Map::new(),
            raw: None,
            date_at: date,
            recorded_at: self.made,
        }
    }
}

/// An entry another Device wrote and signed, kept as it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEntry {
    pub entry: WireEntry,
    /// The log's mark when it came.
    pub mark: i64,
}

/// A change in a pending set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChange {
    Draft(PendingDraft),
    Entry(Box<PendingEntry>),
}

impl PendingChange {
    fn to_json(&self) -> Result<Value> {
        Ok(match self {
            PendingChange::Draft(d) => json!({
                "date": d.date.map(dart_iso),
                "entity": d.entity,
                "field": d.field,
                "made": dart_iso(d.made),
                "mark": d.mark,
                "value": d.value,
            }),
            PendingChange::Entry(r) => json!({"entry": r.entry.line()?, "mark": r.mark}),
        })
    }

    fn parse(json: &Value) -> Option<PendingChange> {
        let mark = json.get("mark")?.as_i64()?;
        if let Some(line) = json.get("entry") {
            return Some(PendingChange::Entry(Box::new(PendingEntry {
                entry: WireEntry::parse(line.as_str()?)?,
                mark,
            })));
        }
        let text = |key: &str| -> Option<Option<String>> {
            match json.get(key) {
                None | Some(Value::Null) => Some(None),
                Some(Value::String(s)) => Some(Some(s.clone())),
                Some(_) => None,
            }
        };
        Some(PendingChange::Draft(PendingDraft {
            entity: json.get("entity")?.as_str()?.to_string(),
            field: json.get("field")?.as_str()?.to_string(),
            value: text("value")?,
            date: match text("date")? {
                Some(date) => Some(parse_time(&date).ok()?),
                None => None,
            },
            made: parse_time(json.get("made")?.as_str()?).ok()?,
            mark,
        }))
    }
}

/// One pending set: what it is for (`kind`), whether the reads show it
/// (`shown`), the changes in the order they came, the keys of the
/// Devices that signed its entries, and what the ticket that made it
/// applies itself at Accept (`payload`, opaque here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSet {
    pub id: String,
    pub kind: String,
    pub shown: bool,
    pub made: DateTime<Utc>,
    pub payload: Option<String>,
    pub keys: Vec<KeyRecord>,
    pub changes: Vec<PendingChange>,
}

impl PendingSet {
    /// The drafts, in the order they were made.
    pub fn drafts(&self) -> Vec<&PendingDraft> {
        self.changes
            .iter()
            .filter_map(|c| match c {
                PendingChange::Draft(d) => Some(d),
                PendingChange::Entry(_) => None,
            })
            .collect()
    }

    /// The entries other Devices wrote, in the order they came.
    pub fn received(&self) -> Vec<&PendingEntry> {
        self.changes
            .iter()
            .filter_map(|c| match c {
                PendingChange::Entry(r) => Some(r.as_ref()),
                PendingChange::Draft(_) => None,
            })
            .collect()
    }

    /// The file's text: keys sorted, no spaces, as both cores write it.
    pub fn encode(&self) -> Result<String> {
        let changes = self
            .changes
            .iter()
            .map(PendingChange::to_json)
            .collect::<Result<Vec<_>>>()?;
        let mut map = Map::new();
        map.insert("changes".into(), Value::Array(changes));
        map.insert("id".into(), json!(self.id));
        map.insert(
            "keys".into(),
            Value::Array(self.keys.iter().map(KeyRecord::to_json).collect()),
        );
        map.insert("kind".into(), json!(self.kind));
        map.insert("made".into(), json!(dart_iso(self.made)));
        map.insert("payload".into(), json!(self.payload));
        map.insert("shown".into(), json!(self.shown));
        serde_json::to_string(&Value::Object(map)).map_err(|e| Error::Invalid(e.to_string()))
    }

    /// The set `text` holds; `None` when it is none, or of a later
    /// format (`v` above 1; a missing `v` is 1, ADR 0019).
    pub fn parse(text: &str) -> Option<PendingSet> {
        let json: Value = serde_json::from_str(text).ok()?;
        if let Some(v) = json.get("v")
            && v.as_i64() != Some(1)
        {
            return None;
        }
        let payload = match json.get("payload") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => return None,
        };
        let keys = json
            .get("keys")?
            .as_array()?
            .iter()
            .map(KeyRecord::from_json)
            .collect::<Option<Vec<_>>>()?;
        let changes = json
            .get("changes")?
            .as_array()?
            .iter()
            .map(PendingChange::parse)
            .collect::<Option<Vec<_>>>()?;
        Some(PendingSet {
            id: json.get("id")?.as_str()?.to_string(),
            kind: json.get("kind")?.as_str()?.to_string(),
            shown: json.get("shown")?.as_bool()?,
            made: parse_time(json.get("made")?.as_str()?).ok()?,
            payload,
            keys,
            changes,
        })
    }
}

/// A field where the user's own edit, made after the pending change, was
/// kept at Accept: `pending` is what the set held, `kept` the value that
/// stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptEdit {
    pub entity: String,
    pub field: String,
    pub pending: Option<String>,
    pub kept: Option<String>,
}

/// What Accept wrote: the received entries taken in (`taken`), the
/// entries this Device wrote (`written`: the drafts, and the user's own
/// values written again over what was received), and the fields where
/// the user's own edit stayed (`kept`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AcceptedChanges {
    pub taken: Vec<WireEntry>,
    pub written: Vec<Entry>,
    pub kept: Vec<KeptEdit>,
}

/// What deciding whether pending changes give way to the user's own
/// edits remembers ([`Store::gives_way`]): up to which mark the edits
/// count, the set's own entries, which Devices are one's own, and the
/// entities' histories.
#[derive(Default)]
pub(crate) struct GiveWay {
    pub(crate) up_to: Option<i64>,
    pub(crate) ids: HashSet<String>,
    pub(crate) own: HashMap<String, bool>,
    pub(crate) histories: HashMap<String, Vec<Entry>>,
}

/// The pending sets the store holds, and what is worked out from the
/// shown ones after every change.
#[derive(Default)]
pub(crate) struct PendingState {
    pub(crate) sets: BTreeMap<String, PendingSet>,
    pub(crate) overlay: Option<Arc<Vec<Entry>>>,
    pub(crate) judged: Option<Arc<JudgedLog>>,
}

/// Where the pending sets' files lie: a directory next to the database,
/// or memory for in-memory stores.
pub trait PendingFiles: Send + Sync {
    /// Every file's text, by set id.
    fn read_all(&self) -> Result<BTreeMap<String, String>>;
    /// The text of the file of set `id`; `None` when there is none.
    fn read(&self, id: &str) -> Result<Option<String>>;
    /// Writes the file of set `id`; a kill midway leaves the old one.
    fn write(&self, id: &str, text: &str) -> Result<()>;
    /// Deletes the file of set `id`, if there is one.
    fn remove(&self, id: &str) -> Result<()>;
}

/// A set id, which names its file: nothing that could leave the
/// directory.
fn is_set_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// [`PendingFiles`] as `<id>.json` files in a directory, made when the
/// first set is.
pub struct FilePendingFiles {
    dir: PathBuf,
}

impl FilePendingFiles {
    pub fn new(dir: PathBuf) -> FilePendingFiles {
        FilePendingFiles { dir }
    }
}

impl PendingFiles for FilePendingFiles {
    fn read_all(&self) -> Result<BTreeMap<String, String>> {
        let mut out = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Ok(out);
        };
        for entry in entries {
            let path = entry?.path();
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".json"))
            else {
                continue;
            };
            if !is_set_id(id) || !path.is_file() {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                out.insert(id.to_string(), text);
            }
        }
        Ok(out)
    }

    fn read(&self, id: &str) -> Result<Option<String>> {
        if !is_set_id(id) {
            return Ok(None);
        }
        let file = self.dir.join(format!("{id}.json"));
        if !file.is_file() {
            return Ok(None);
        }
        Ok(std::fs::read_to_string(file).ok())
    }

    fn write(&self, id: &str, text: &str) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let file = self.dir.join(format!("{id}.json"));
        // Written beside, then renamed, as a blob is.
        let tmp = self.dir.join(format!("{id}.json.tmp"));
        {
            use std::io::Write;
            let mut out = std::fs::File::create(&tmp)?;
            out.write_all(text.as_bytes())?;
            out.sync_all()?;
        }
        std::fs::rename(&tmp, &file)?;
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<()> {
        let file = self.dir.join(format!("{id}.json"));
        if file.exists() {
            std::fs::remove_file(file)?;
        }
        Ok(())
    }
}

/// [`PendingFiles`] in memory: gone with the store. For tests.
#[derive(Default)]
pub struct MemoryPendingFiles {
    held: parking_lot::Mutex<BTreeMap<String, String>>,
}

impl PendingFiles for MemoryPendingFiles {
    fn read_all(&self) -> Result<BTreeMap<String, String>> {
        Ok(self.held.lock().clone())
    }

    fn read(&self, id: &str) -> Result<Option<String>> {
        Ok(self.held.lock().get(id).cloned())
    }

    fn write(&self, id: &str, text: &str) -> Result<()> {
        self.held.lock().insert(id.to_string(), text.to_string());
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<()> {
        self.held.lock().remove(id);
        Ok(())
    }
}

/// The fields `field` stands or falls with: its whole schedule for a
/// Routine's, else itself.
fn unit_fields(field: &str) -> Vec<String> {
    match conflict_unit(field) {
        Some(unit) => conflict_unit_fields(unit)
            .into_iter()
            .map(str::to_string)
            .collect(),
        None => vec![field.to_string()],
    }
}

/// The entry of `field` in `history` that wins among those `take` keeps.
fn latest<'a>(
    history: &'a [Entry],
    field: &str,
    take: impl Fn(&Entry) -> bool,
) -> Option<&'a Entry> {
    let mut held: Option<&Entry> = None;
    for e in history {
        if e.field != field || !take(e) {
            continue;
        }
        if held.is_none_or(|h| Store::latest_wins(e, h) == Ordering::Greater) {
            held = Some(e);
        }
    }
    held
}

fn signed_bytes(e: &WireEntry) -> Vec<u8> {
    entry_bytes_with(
        &e.device,
        e.dseq,
        &e.entity,
        &e.field,
        e.value.as_deref(),
        &e.date,
        &e.author,
        &e.recorded,
        e.ctx.as_deref(),
        &e.extra,
    )
}

/// Opening, filling, accepting and declining pending sets, as the Dart
/// core's `PendingChanges`.
impl Store {
    /// Opens an empty pending set for `kind` (what it is for, such as
    /// `ai-task` or `area-offer`) and returns its id. A `shown` set is
    /// read through at once; any other only through
    /// [`Store::pending_set`]. `payload` is kept as it is for the caller
    /// to apply at Accept.
    pub fn open_pending(&self, kind: &str, shown: bool, payload: Option<&str>) -> Result<String> {
        let id = self.new_id()?;
        if !is_set_id(&id) {
            return Err(Error::Invalid(format!("a set id names a file: {id}")));
        }
        self.keep_pending_set(PendingSet {
            id: id.clone(),
            kind: kind.to_string(),
            shown,
            made: self.now(),
            payload: payload.map(str::to_string),
            keys: Vec::new(),
            changes: Vec::new(),
        })?;
        Ok(id)
    }

    /// The pending set `id` as its file holds it now; `None` when there
    /// is none.
    pub fn pending_set(&self, id: &str) -> Result<Option<PendingSet>> {
        self.fresh_pending_set(id)
    }

    /// Every pending set as the files hold them now, the oldest first:
    /// those a restart left too.
    pub fn pending_sets(&self) -> Result<Vec<PendingSet>> {
        self.fresh_pending_sets()
    }

    /// The set `id` as its file holds it now: another store open on the
    /// same file may have changed or ended it.
    fn set_of(&self, id: &str) -> Result<PendingSet> {
        self.fresh_pending_set(id)?
            .ok_or_else(|| Error::Invalid(format!("no pending set {id}")))
    }

    /// Adds the change of `field` on `entity` to `value` to the set `id`,
    /// as this Device would write it: dated `date`, or when it is made.
    pub fn draft_pending(
        &self,
        id: &str,
        entity: &str,
        field: &str,
        value: Option<&str>,
        date: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let mut set = self.set_of(id)?;
        set.changes.push(PendingChange::Draft(PendingDraft {
            entity: entity.to_string(),
            field: field.to_string(),
            value: value.map(str::to_string),
            date,
            made: self.now(),
            mark: self.current_seq()?,
        }));
        self.keep_pending_set(set)
    }

    /// Adds the entries other Devices wrote to the set `id`, with the
    /// `keys` they came with, and returns those taken. Each is checked as
    /// an import checks it, but nothing is pinned or stored: one already
    /// held or in the set is skipped; one whose signature does not verify
    /// under a key held here or among the set's, or which claims this
    /// Device, is refused and counted in `report`; one a blocked person's
    /// Device wrote that an import would not keep is refused unseen (ADR
    /// 0021), her Devices judged by the log and the set together.
    pub fn receive_pending(
        &self,
        id: &str,
        entries: &[WireEntry],
        keys: &[KeyRecord],
        report: &mut ImportReport,
    ) -> Result<Vec<WireEntry>> {
        let mut set = self.set_of(id)?;
        let own = self.device_id()?;
        let mut records: BTreeMap<String, KeyRecord> = set
            .keys
            .iter()
            .map(|k| (k.device.clone(), k.clone()))
            .collect();
        for k in keys {
            if k.device != own && k.valid() && !records.contains_key(&k.device) {
                records.insert(k.device.clone(), k.clone());
            }
        }
        let mut in_set: HashSet<(String, i64)> = set
            .received()
            .iter()
            .map(|r| (r.entry.device.clone(), r.entry.dseq))
            .collect();
        let mut checked: Vec<WireEntry> = Vec::new();
        for e in entries {
            if in_set.contains(&(e.device.clone(), e.dseq)) || self.holds(&e.device, e.dseq)? {
                continue;
            }
            let key = if e.device == own {
                None
            } else {
                match self.pinned_key(&e.device)? {
                    Some(pinned) => Some(pinned.record.public_key),
                    None => records.get(&e.device).map(|k| k.public_key.clone()),
                }
            };
            let signed =
                key.is_some_and(|k| verify_signature(&k, &signed_bytes(e), e.sig.as_deref()));
            if !signed {
                report.refuse(&e.author, &e.device);
                continue;
            }
            in_set.insert((e.device.clone(), e.dseq));
            checked.push(e.clone());
        }
        let mut together: Vec<WireEntry> = set.received().iter().map(|r| r.entry.clone()).collect();
        together.extend(checked.iter().cloned());
        let blocked = self.blocked_for(&together)?;
        let taken: Vec<WireEntry> = checked
            .into_iter()
            .filter(|e| !blocked.contains(&e.device) || wire_kept_from_blocked(e))
            .collect();
        let used: HashSet<&str> = taken.iter().map(|e| e.device.as_str()).collect();
        let held: HashSet<String> = set.keys.iter().map(|k| k.device.clone()).collect();
        set.keys = records
            .into_values()
            .filter(|k| used.contains(k.device.as_str()) || held.contains(&k.device))
            .collect();
        let mark = self.current_seq()?;
        for e in &taken {
            set.changes
                .push(PendingChange::Entry(Box::new(PendingEntry {
                    entry: e.clone(),
                    mark,
                })));
        }
        self.keep_pending_set(set)?;
        Ok(taken)
    }

    /// The Devices of blocked people, by the grants of the log and of
    /// `entries` together: a set may bring the grant that names her.
    fn blocked_for(&self, entries: &[WireEntry]) -> Result<HashSet<String>> {
        if self.blocked_ids()?.is_empty() {
            return Ok(HashSet::new());
        }
        if !entries.iter().any(|e| e.field.starts_with(ROLE_PREFIX)) {
            return self.blocked_devices();
        }
        let mut all: Vec<Entry> = self
            .all_entries()?
            .into_iter()
            .filter(|e| e.v() <= crate::entry::ENTRY_VERSION)
            .collect();
        for e in entries {
            if e.applies() {
                all.push(entry_of(0, e.clone())?);
            }
        }
        self.blocked_devices_by(&judge_log(&all))
    }

    /// Takes out of every pending set what a blocked person's Device
    /// wrote that an import would not keep (ADR 0021); returns whether
    /// anything went.
    pub fn drop_blocked_from_pending(&self) -> Result<bool> {
        if self.blocked_ids()?.is_empty() {
            return Ok(false);
        }
        let sets = self.fresh_pending_sets()?;
        let mut dropped = false;
        for mut set in sets {
            let received: Vec<WireEntry> = set.received().iter().map(|r| r.entry.clone()).collect();
            if received.is_empty() {
                continue;
            }
            let blocked = self.blocked_for(&received)?;
            let before = set.changes.len();
            set.changes.retain(|c| match c {
                PendingChange::Entry(r) => {
                    !blocked.contains(&r.entry.device) || wire_kept_from_blocked(&r.entry)
                }
                PendingChange::Draft(_) => true,
            });
            if set.changes.len() == before {
                continue;
            }
            self.keep_pending_set(set)?;
            dropped = true;
        }
        Ok(dropped)
    }

    /// Whether one of the user's own Devices wrote `field` on `entity`
    /// after `mark`, as the Dart core's `givesWay`: an edit of the
    /// user's, made after the pending change, that the change gives way
    /// to in the reads and at Accept. A Routine's schedule counts as one
    /// field.
    pub(crate) fn gives_way(
        &self,
        entity: &str,
        field: &str,
        mark: i64,
        by: &mut GiveWay,
    ) -> Result<bool> {
        let fields = unit_fields(field);
        if !by.histories.contains_key(entity) {
            let history = self.logged_history(entity)?;
            by.histories.insert(entity.to_string(), history);
        }
        let Some(history) = by.histories.get(entity) else {
            return Ok(false);
        };
        for e in history {
            if e.seq > mark
                && by.up_to.is_none_or(|up_to| e.seq <= up_to)
                && fields.contains(&e.field)
                && !by.ids.contains(&e.id())
                && self.written_by_own_in(&e.device, &mut by.own)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Accepts the set `id`, as the Dart core's `acceptPending`: its
    /// received entries are taken in as any import is
    /// ([`Store::apply_entries`]), then its drafts are written as this
    /// Device's entries, numbered and signed now, each dated as it was
    /// drafted. Where one of the user's own Devices edited a field after
    /// the change came (its whole schedule, for a Routine's), the user's
    /// edit stays: the draft is not written, and a value received over it
    /// is written over again with the user's, which ends the Conflict
    /// too. The set is gone after; what the ticket that opened it keeps
    /// in its `payload` is its own to apply.
    pub fn accept_pending(&self, id: &str) -> Result<AcceptedChanges> {
        let set = self.set_of(id)?;
        let up_to = self.current_seq()?;
        let received: Vec<PendingEntry> = set.received().into_iter().cloned().collect();
        let ids: HashSet<String> = received
            .iter()
            .map(|r| format!("{}:{}", r.entry.device, r.entry.dseq))
            .collect();
        let taken = if received.is_empty() {
            Vec::new()
        } else {
            let entries: Vec<WireEntry> = received.iter().map(|r| r.entry.clone()).collect();
            self.apply_entries(&entries, &set.keys, None, &mut ImportReport::default())?
        };
        let mut by = GiveWay {
            up_to: Some(up_to),
            ids: ids.clone(),
            ..GiveWay::default()
        };
        let mut kept: Vec<KeptEdit> = Vec::new();
        let mut keep = |edit: KeptEdit| match kept
            .iter_mut()
            .find(|k| k.entity == edit.entity && k.field == edit.field)
        {
            Some(held) => *held = edit,
            None => kept.push(edit),
        };
        let mut written: Vec<Entry> = Vec::new();
        self.batch(|| {
            // What was received gives way to the user's own edits.
            let mut restored: HashSet<(String, String)> = HashSet::new();
            for r in &received {
                let e = &r.entry;
                if !e.applies() || !self.holds(&e.device, e.dseq)? {
                    continue;
                }
                if !self.gives_way(&e.entity, &e.field, r.mark, &mut by)? {
                    continue;
                }
                let fields = unit_fields(&e.field);
                let history = self.logged_history(&e.entity)?;
                for field in &fields {
                    if !restored.insert((e.entity.clone(), field.clone())) {
                        continue;
                    }
                    let before = latest(&history, field, |x| !ids.contains(&x.id()));
                    let theirs = latest(&history, field, |x| ids.contains(&x.id()));
                    let Some(theirs) = theirs else { continue };
                    let before_value = before.and_then(|b| b.value.clone());
                    if theirs.value == before_value {
                        continue;
                    }
                    keep(KeptEdit {
                        entity: e.entity.clone(),
                        field: field.clone(),
                        pending: theirs.value.clone(),
                        kept: before_value.clone(),
                    });
                    // Written again where theirs would show, or stand as
                    // a Conflict with the user's edit, which did not see
                    // it: the set was not shown then.
                    if self.logged(&e.entity, field)? == before_value
                        && self.conflicts_among(&[theirs.wire()])?.is_empty()
                    {
                        continue;
                    }
                    written.push(self.append(&e.entity, field, before_value.as_deref(), None)?);
                }
            }
            // Then the drafts, numbered and signed now.
            for d in set.drafts() {
                if self.gives_way(&d.entity, &d.field, d.mark, &mut by)? {
                    keep(KeptEdit {
                        entity: d.entity.clone(),
                        field: d.field.clone(),
                        pending: d.value.clone(),
                        kept: self.logged(&d.entity, &d.field)?,
                    });
                    continue;
                }
                written.push(self.append(
                    &d.entity,
                    &d.field,
                    d.value.as_deref(),
                    Some(d.date.unwrap_or(d.made)),
                )?);
            }
            // In the same transaction: a restart before the file is gone
            // finds the set accepted, not waiting.
            self.set_local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"), "accepted")
        })?;
        self.forget_pending_set(id)?;
        Ok(AcceptedChanges {
            taken,
            written,
            kept,
        })
    }

    /// Declines the set `id`: its file is deleted and the log never saw
    /// anything of it. Returns whether there was one.
    pub fn decline_pending(&self, id: &str) -> Result<bool> {
        if self.fresh_pending_set(id)?.is_none() {
            return Ok(false);
        }
        self.forget_pending_set(id)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
