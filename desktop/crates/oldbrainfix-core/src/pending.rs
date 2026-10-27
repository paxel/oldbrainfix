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
mod tests {
    use super::*;
    use crate::blocks::{BLOCKED, PREFIX as BLOCK_PREFIX};
    use crate::keys::PERSONAL_SCOPE;
    use crate::notes::keys as note_keys;
    use crate::routines::keys as routine_keys;
    use crate::signing::SigningKey;
    use crate::sync::wire;
    use crate::text_format::StoredText;
    use crate::{ClockTime, Day, Notes, Places, Recurrence, Routines, Schedule};
    use chrono::TimeZone;
    use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    fn clock_of(ticks: &Arc<AtomicI64>) -> crate::Clock {
        let ticks = ticks.clone();
        Box::new(move || {
            let n = ticks.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).single();
            start.unwrap_or_default() + chrono::Duration::seconds(n)
        })
    }

    fn device(ticks: &Arc<AtomicI64>, seed: u8, author: &str) -> Made<Store> {
        let store = Store::in_memory(clock_of(ticks), move || {
            Ok(SigningKey::from_seed([seed; 32]))
        })?;
        store.set_local_setting("author", author)?;
        Ok(store)
    }

    struct Two {
        ann: Store,
        bob: Store,
    }

    fn two() -> Made<Two> {
        let ticks = Arc::new(AtomicI64::new(0));
        let ann = device(&ticks, 1, "Ann")?;
        let bob = device(&ticks, 2, "Bob")?;
        Ok(Two { ann, bob })
    }

    fn text(plain: &str) -> String {
        StoredText::plain(plain).encode()
    }

    fn text_of(store: &Store, note: &str) -> Made<Option<String>> {
        Ok(Notes::new(store).by_id(note)?.map(|n| n.text))
    }

    /// What the screens read of `store`: every Note's text, by id.
    fn texts(store: &Store) -> Made<Vec<(String, String)>> {
        Ok(Notes::new(store)
            .all()?
            .into_iter()
            .map(|n| (n.id, n.text))
            .collect())
    }

    fn pills() -> Made<Schedule> {
        Ok(Schedule::new(
            Day::new(2026, 3, 1),
            &[ClockTime::new(8, 0).ok_or("a time")?],
            Some(Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e:?}"))?),
        )
        .ok_or("a schedule")?)
    }

    /// `to` takes in every entry `from` holds, with its keys.
    fn hand_over(from: &Store, to: &Store) -> TestResult {
        to.apply_entries(
            &wire(&from.entries_since(&HashMap::new())?),
            &from.key_records()?,
            None,
            &mut ImportReport::default(),
        )?;
        Ok(())
    }

    fn wires(store: &Store) -> Made<Vec<WireEntry>> {
        Ok(wire(&store.all_entries()?))
    }

    #[test]
    fn a_shown_set_reads_as_stored_and_the_log_sees_none_of_it() -> TestResult {
        let Two { ann, .. } = two()?;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        let entries = ann.all_entries()?.len();
        let vector = ann.version_vector()?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;
        let made = format!("note:{}", ann.new_id()?);
        let captured = dart_iso(ann.now());
        for (field, value) in [
            (note_keys::TYPE, note_keys::TYPE_NOTE.to_string()),
            (note_keys::CAPTURED, captured),
            (note_keys::TEXT, text("Bread")),
        ] {
            ann.draft_pending(&id, &made, field, Some(&value), None)?;
        }

        assert_eq!(text_of(&ann, &milk.id)?.as_deref(), Some("Oat milk"));
        assert_eq!(text_of(&ann, &made)?.as_deref(), Some("Bread"));
        assert_eq!(ann.all_entries()?.len(), entries);
        assert_eq!(ann.version_vector()?, vector);
        assert!(ann.entries_since(&vector)?.is_empty());
        assert_eq!(ann.logged(&milk.id, note_keys::TEXT)?, Some(text("Milk")));
        Ok(())
    }

    #[test]
    fn a_set_not_shown_is_read_only_through_itself() -> TestResult {
        let Two { ann, .. } = two()?;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        let id = ann.open_pending("sent-item", false, Some("{}"))?;
        ann.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;

        assert_eq!(text_of(&ann, &milk.id)?.as_deref(), Some("Milk"));
        let set = ann.pending_set(&id)?.ok_or("the set")?;
        assert_eq!(set.kind, "sent-item");
        assert_eq!(set.payload.as_deref(), Some("{}"));
        assert_eq!(set.drafts()[0].value, Some(text("Oat milk")));
        Ok(())
    }

    #[test]
    fn accept_writes_them_numbered_and_signed_then_dated_when_drafted() -> TestResult {
        let Two { ann, bob, .. } = two()?;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        let routine = Routines::new(&ann).spawn(&milk.id, &pills()?)?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;
        ann.draft_pending(&id, &routine.id, routine_keys::RENAG, Some("15"), None)?;
        let drafted: Vec<PendingDraft> = ann
            .pending_set(&id)?
            .ok_or("the set")?
            .drafts()
            .into_iter()
            .cloned()
            .collect();
        let before = texts(&ann)?;
        let renag = ann.current_fields(&routine.id)?;
        let own = ann.device_id()?;
        let max = ann.version_vector()?.get(&own).copied().unwrap_or(0);

        let accepted = ann.accept_pending(&id)?;

        assert!(accepted.kept.is_empty());
        assert!(accepted.taken.is_empty());
        let dseqs: Vec<i64> = accepted.written.iter().map(|e| e.dseq).collect();
        assert_eq!(dseqs, [max + 1, max + 2]);
        for (e, d) in accepted.written.iter().zip(&drafted) {
            assert_eq!(e.device, own);
            assert_eq!(e.date_at, d.made);
            assert!(e.sig.is_some());
        }
        assert_eq!(texts(&ann)?, before);
        assert_eq!(ann.current_fields(&routine.id)?, renag);
        assert!(ann.pending_set(&id)?.is_none());
        assert!(ann.pending_sets()?.is_empty());
        // What the log now holds verifies on another Device.
        hand_over(&ann, &bob)?;
        assert_eq!(text_of(&bob, &milk.id)?.as_deref(), Some("Oat milk"));
        Ok(())
    }

    #[test]
    fn decline_leaves_the_log_as_it_was() -> TestResult {
        let Two { ann, .. } = two()?;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        let entries = wires(&ann)?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;

        assert!(ann.decline_pending(&id)?);

        assert_eq!(wires(&ann)?, entries);
        assert_eq!(text_of(&ann, &milk.id)?.as_deref(), Some("Milk"));
        assert!(ann.pending_sets()?.is_empty());
        assert!(!ann.decline_pending(&id)?);
        Ok(())
    }

    #[test]
    fn own_edits_made_meanwhile_win_a_whole_schedule_too_and_the_review_lists_them() -> TestResult {
        let Two { ann, .. } = two()?;
        let notes = Notes::new(&ann);
        let milk = notes.capture("Milk", None, None)?;
        let bread = notes.capture("Bread", None, None)?;
        let routine = Routines::new(&ann).spawn(&milk.id, &pills()?)?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;
        ann.draft_pending(
            &id,
            &routine.id,
            routine_keys::RULE,
            Some("FREQ=WEEKLY"),
            None,
        )?;
        // The user edits the Note and the Routine's start, then the AI
        // edits the other Note, after the user did.
        notes.edit(&milk.id, "Milk, two litres")?;
        notes.edit(&bread.id, "Brown bread")?;
        ann.append(&routine.id, routine_keys::START, Some("2026-03-02"), None)?;
        ann.draft_pending(
            &id,
            &bread.id,
            note_keys::TEXT,
            Some(&text("Rye bread")),
            None,
        )?;

        let accepted = ann.accept_pending(&id)?;

        assert_eq!(
            accepted.kept,
            [
                KeptEdit {
                    entity: milk.id.clone(),
                    field: note_keys::TEXT.into(),
                    pending: Some(text("Oat milk")),
                    kept: Some(text("Milk, two litres")),
                },
                KeptEdit {
                    entity: routine.id.clone(),
                    field: routine_keys::RULE.into(),
                    pending: Some("FREQ=WEEKLY".into()),
                    kept: Some("FREQ=DAILY".into()),
                },
            ]
        );
        let entities: Vec<&str> = accepted.written.iter().map(|e| e.entity.as_str()).collect();
        assert_eq!(entities, [bread.id.as_str()]);
        assert_eq!(
            text_of(&ann, &milk.id)?.as_deref(),
            Some("Milk, two litres")
        );
        assert_eq!(text_of(&ann, &bread.id)?.as_deref(), Some("Rye bread"));
        assert_eq!(
            ann.current(&routine.id, routine_keys::RULE)?.as_deref(),
            Some("FREQ=DAILY")
        );
        Ok(())
    }

    #[test]
    fn the_reads_show_what_accept_keeps_not_a_rule_the_user_changed_the_schedule_after()
    -> TestResult {
        let Two { ann, .. } = two()?;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        let routine = Routines::new(&ann).spawn(&milk.id, &pills()?)?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &routine.id,
            routine_keys::RULE,
            Some("FREQ=WEEKLY"),
            None,
        )?;
        assert_eq!(
            ann.current(&routine.id, routine_keys::RULE)?.as_deref(),
            Some("FREQ=WEEKLY")
        );
        ann.append(&routine.id, routine_keys::START, Some("2026-03-02"), None)?;
        let shown = ann.current_fields(&routine.id)?;

        assert_eq!(shown.get(routine_keys::RULE), Some("FREQ=DAILY"));
        ann.accept_pending(&id)?;
        assert_eq!(ann.current_fields(&routine.id)?, shown);
        Ok(())
    }

    #[test]
    fn an_edit_another_person_wrote_meanwhile_does_not_stop_a_draft() -> TestResult {
        let Two { ann, bob, .. } = two()?;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        hand_over(&ann, &bob)?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;
        Notes::new(&bob).edit(&milk.id, "Milk from Bob")?;
        hand_over(&bob, &ann)?;

        let accepted = ann.accept_pending(&id)?;

        assert!(accepted.kept.is_empty());
        assert_eq!(accepted.written.len(), 1);
        Ok(())
    }

    fn reopen(dir: &std::path::Path, ticks: &Arc<AtomicI64>) -> Made<Store> {
        let store = Store::open(&dir.join("log.db"), clock_of(ticks), || {
            Ok(SigningKey::from_seed([1; 32]))
        })?;
        store.set_local_setting("author", "Ann")?;
        Ok(store)
    }

    #[test]
    fn a_restart_finds_the_set_waiting_as_it_was() -> TestResult {
        let dir = tempfile::tempdir()?;
        let ticks = Arc::new(AtomicI64::new(0));
        let store = reopen(dir.path(), &ticks)?;
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        let id = store.open_pending("ai-task", true, None)?;
        store.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;
        let file = dir.path().join("pending").join(format!("{id}.json"));
        let written = std::fs::read_to_string(&file)?;
        drop(store);

        let again = reopen(dir.path(), &ticks)?;
        assert_eq!(again.pending_set(&id)?.ok_or("the set")?.encode()?, written);
        assert_eq!(text_of(&again, &milk.id)?.as_deref(), Some("Oat milk"));
        again.accept_pending(&id)?;
        assert!(!file.exists());
        assert_eq!(
            again.logged(&milk.id, note_keys::TEXT)?,
            Some(text("Oat milk"))
        );
        Ok(())
    }

    #[test]
    fn a_restart_after_an_accept_cut_short_before_its_file_went_finds_none() -> TestResult {
        let dir = tempfile::tempdir()?;
        let ticks = Arc::new(AtomicI64::new(0));
        let store = reopen(dir.path(), &ticks)?;
        let id = store.open_pending("ai-task", true, None)?;
        store.draft_pending(&id, "note:x", note_keys::TEXT, Some(&text("Lost?")), None)?;
        // What an Accept writes with its entries, before the file goes.
        store.set_local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"), "accepted")?;
        drop(store);

        let again = reopen(dir.path(), &ticks)?;
        assert!(again.pending_sets()?.is_empty());
        assert!(
            !dir.path()
                .join("pending")
                .join(format!("{id}.json"))
                .exists()
        );
        assert!(
            again
                .local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"))?
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn a_second_store_on_the_same_file_sees_what_the_other_did_to_a_set() -> TestResult {
        let dir = tempfile::tempdir()?;
        let ticks = Arc::new(AtomicI64::new(0));
        let app = reopen(dir.path(), &ticks)?;
        let milk = Notes::new(&app).capture("Milk", None, None)?;
        let id = app.open_pending("ai-task", true, None)?;
        app.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Oat milk")),
            None,
        )?;
        let background = reopen(dir.path(), &ticks)?;
        // The app adds a draft the background store never read.
        app.draft_pending(
            &id,
            &milk.id,
            note_keys::IMPORTANCE,
            Some("important"),
            None,
        )?;
        background.draft_pending(
            &id,
            &milk.id,
            note_keys::TEXT,
            Some(&text("Rye milk")),
            None,
        )?;
        assert_eq!(app.pending_set(&id)?.ok_or("the set")?.drafts().len(), 3);

        app.accept_pending(&id)?;

        assert!(background.pending_set(&id)?.is_none());
        assert_eq!(text_of(&background, &milk.id)?.as_deref(), Some("Rye milk"));
        assert!(
            background
                .draft_pending(&id, &milk.id, note_keys::TEXT, Some(&text("No")), None)
                .is_err()
        );
        assert!(
            !dir.path()
                .join("pending")
                .join(format!("{id}.json"))
                .exists()
        );
        Ok(())
    }

    #[test]
    fn a_restart_leaves_a_file_it_cannot_read_where_it_is_unread() -> TestResult {
        let dir = tempfile::tempdir()?;
        let ticks = Arc::new(AtomicI64::new(0));
        drop(reopen(dir.path(), &ticks)?);
        let pending = dir.path().join("pending");
        std::fs::create_dir_all(&pending)?;
        let file = pending.join("abc.json");
        std::fs::write(&file, r#"{"changes":[],"v":2}"#)?;

        let again = reopen(dir.path(), &ticks)?;
        assert!(again.pending_sets()?.is_empty());
        assert!(file.exists());
        Ok(())
    }

    #[test]
    fn received_entries_are_checked_as_they_come_pinned_nothing_and_taken_in_at_accept()
    -> TestResult {
        let Two { ann, bob, .. } = two()?;
        let note = Notes::new(&bob).capture("From Bob", None, None)?;
        let entries = wire(&bob.entries_since(&HashMap::new())?);
        let (last, rest) = entries.split_last().ok_or("entries")?;
        let mut forged = last.clone();
        forged.value = Some(text("Forged"));
        let mut mine = entries[0].clone();
        mine.device = ann.device_id()?;
        mine.dseq = 99;
        let mut report = ImportReport::default();
        let id = ann.open_pending("bundle", true, None)?;
        let mut first = rest.to_vec();
        first.extend([forged, mine]);

        let taken = ann.receive_pending(&id, &first, &[bob.own_key_record()?], &mut report)?;

        assert_eq!(taken.len(), rest.len());
        assert_eq!(report.refused_count(), 2);
        assert!(ann.pinned_key(&bob.device_id()?)?.is_none());
        assert!(!ann.holds(&bob.device_id()?, 1)?);
        assert!(text_of(&ann, &note.id)?.is_some());
        // The rest comes later; one already in the set is not taken twice.
        let again = ann.receive_pending(
            &id,
            &entries,
            &[bob.own_key_record()?],
            &mut ImportReport::default(),
        )?;
        assert_eq!(again.len(), 1);

        let accepted = ann.accept_pending(&id)?;

        assert_eq!(accepted.taken.len(), entries.len());
        assert!(accepted.written.is_empty());
        assert!(ann.pinned_key(&bob.device_id()?)?.is_some());
        assert_eq!(text_of(&ann, &note.id)?.as_deref(), Some("From Bob"));
        assert_eq!(
            ann.version_vector()?.get(&bob.device_id()?).copied(),
            Some(last.dseq)
        );
        Ok(())
    }

    /// Bob edits Ann's Note; Ann takes his edit into a set, shown or not,
    /// and edits the Note herself.
    fn edited_over(shown: bool) -> Made<(Two, String, AcceptedChanges)> {
        let two = two()?;
        let (ann, bob) = (&two.ann, &two.bob);
        let milk = Notes::new(ann).capture("Milk", None, None)?;
        hand_over(ann, bob)?;
        Notes::new(bob).edit(&milk.id, "Milk from Bob")?;
        let id = ann.open_pending(if shown { "bundle" } else { "area-offer" }, shown, None)?;
        ann.receive_pending(
            &id,
            &wire(&bob.entries_since(&ann.version_vector()?)?),
            &bob.key_records()?,
            &mut ImportReport::default(),
        )?;
        if shown {
            assert_eq!(text_of(ann, &milk.id)?.as_deref(), Some("Milk from Bob"));
        }
        Notes::new(ann).edit(&milk.id, "Milk, two litres")?;
        assert!(ann.conflicts()?.is_empty());
        let accepted = ann.accept_pending(&id)?;
        assert!(ann.conflicts()?.is_empty());
        assert_eq!(
            accepted.kept,
            [KeptEdit {
                entity: milk.id.clone(),
                field: note_keys::TEXT.into(),
                pending: Some(text("Milk from Bob")),
                kept: Some(text("Milk, two litres")),
            }]
        );
        assert_eq!(text_of(ann, &milk.id)?.as_deref(), Some("Milk, two litres"));
        Ok((two, milk.id, accepted))
    }

    #[test]
    fn received_entries_give_way_to_an_own_edit_made_over_them_which_saw_them() -> TestResult {
        let (_, _, accepted) = edited_over(true)?;
        assert!(accepted.written.is_empty());
        Ok(())
    }

    #[test]
    fn received_entries_of_a_set_not_shown_give_way_written_again_so_no_conflict_stands()
    -> TestResult {
        let (_, _, accepted) = edited_over(false)?;
        assert_eq!(accepted.written.len(), 1);
        assert_eq!(accepted.written[0].value, Some(text("Milk, two litres")));
        Ok(())
    }

    #[test]
    fn a_pending_change_is_trusted_for_nothing_no_block_no_area() -> TestResult {
        let Two { ann, .. } = two()?;
        let carol = ann.new_id()?;
        let area = Places::new(&ann).add_area("Household")?.id;
        let milk = Notes::new(&ann).capture("Milk", None, None)?;
        let id = ann.open_pending("ai-task", true, None)?;
        ann.draft_pending(
            &id,
            &format!("{BLOCK_PREFIX}{carol}"),
            BLOCKED,
            Some("Carol"),
            None,
        )?;
        ann.draft_pending(&id, &milk.id, note_keys::PLACE, Some(&area), None)?;

        assert!(ann.blocked_ids()?.is_empty());
        assert_eq!(ann.scope_of(&milk.id)?, PERSONAL_SCOPE);
        assert_eq!(
            Notes::new(&ann).by_id(&milk.id)?.and_then(|n| n.place),
            Some(area)
        );
        Ok(())
    }
}
