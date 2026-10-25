//! Sent Items (CONTEXT.md, ADR 0009, 0015), as the Dart core's
//! `sent_items.dart` writes and reads them: one Note with its Routines
//! and Reminders, sent as a file through any messenger to another Old
//! Brain Fix user, signed by the sender's Device key under a stable id,
//! encrypted to the recipient's Devices when their keys are known here,
//! else signed only. The receiver previews it and accepts it into
//! Unsorted, or declines and nothing is kept.
//!
//! What travels is the Note's text, the texts of its Sections and the
//! schedules of its Routines and Reminders; never its Capture Context,
//! photos or voice recordings.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};

use crate::box_keys::{BOX_KEY_BYTES, box_public_of, box_secret_of, wrap_key, x25519};
use crate::entry::{dart_iso, parse_time};
use crate::keys::{AREA_KEY_BYTES, SEAL_NONCE_BYTES, seal, unseal};
use crate::notes::keys as note_keys;
use crate::routines::keys as routine_keys;
use crate::signing::{KeyRecord, KeyTrust, SigningKey, verify_signature};
use crate::store::{Fields, Store};
use crate::{Error, Notes, Result, Routine, Routines, Schedule};

/// The file ending of a Sent Item.
pub const EXTENSION: &str = "oldbrain";

/// The type a Sent Item is shared as.
pub const MIME_TYPE: &str = "application/vnd.oldbrainfix.item";

/// The format a file says it is in.
pub const FORMAT: i64 = 1;

/// A file larger than this is no Sent Item: it holds only texts.
pub const MAX_BYTES: usize = 1 << 20;

/// The kind a cancellation says it is; an item says none.
const CANCEL_KIND: &str = "cancel";

/// What the signature covers before the payload.
const SIGNED_PREFIX: &str = "oldbrainfix:sent:";
/// What the sealed file is bound to.
const SEALED_AAD: &str = "oldbrainfix:sent";
/// What the content key sealed for each recipient is bound to.
pub const SENT_KEY_AAD: &str = "oldbrainfix:sent-key";

/// Field keys a Sent Item leaves in the log.
pub mod keys {
    /// On the sender's Note, one per sending Device: `sent:<device>` =
    /// `{"id":…,"version":…}`.
    pub const SENT_PREFIX: &str = "sent:";
    /// On an accepted Note: the sender's key, the Sent Item's id, the
    /// sender's name and the version accepted; on an accepted Routine:
    /// the id the sender's Routine has.
    pub const RECEIVED: &str = "received";

    pub fn sent(device: &str) -> String {
        format!("{SENT_PREFIX}{device}")
    }
}

/// The name a Sent Item written on `day` (`YYYY-MM-DD`) goes by.
pub fn file_name(day: &str) -> String {
    format!("oldbrainfix-note-{day}.{EXTENSION}")
}

/// A Section of a Sent Item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentSection {
    pub text: String,
    pub captured: DateTime<Utc>,
}

/// A Routine or Reminder of a Sent Item: the sender's id and its
/// schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct SentRoutine {
    pub id: String,
    pub schedule: Schedule,
}

/// What a Sent Item holds, as signed.
#[derive(Debug, Clone, PartialEq)]
pub struct SentItem {
    pub id: String,
    pub version: i64,
    /// Whether it cancels the Sent Item of this id: the receiver may then
    /// move it to the trash. It still carries what the Note holds.
    pub cancel: bool,
    pub from: KeyRecord,
    pub name: String,
    pub text: String,
    pub sections: Vec<SentSection>,
    pub routines: Vec<SentRoutine>,
}

/// Builds a JSON object in key order, the same with or without
/// serde_json's `preserve_order`.
fn object(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}

impl SentItem {
    /// What is signed: keys sorted, no spaces, as the Dart core writes it;
    /// only a cancellation says its kind.
    pub fn encode(&self) -> String {
        let sections: Vec<Value> = self
            .sections
            .iter()
            .map(|s| {
                object(vec![
                    ("captured", json!(dart_iso(s.captured))),
                    ("text", json!(s.text)),
                ])
            })
            .collect();
        let routines: Vec<Value> = self
            .routines
            .iter()
            .map(|r| {
                let mut pairs = vec![("id", json!(r.id))];
                if let Some(rule) = &r.schedule.rule {
                    pairs.push(("rule", json!(rule.format())));
                }
                pairs.push(("start", json!(r.schedule.start.to_string())));
                pairs.push(("times", json!(times(&r.schedule))));
                object(pairs)
            })
            .collect();
        let mut pairs = vec![
            (
                "from",
                object(vec![
                    ("key", self.from.to_json()),
                    ("name", json!(self.name)),
                ]),
            ),
            ("id", json!(self.id)),
        ];
        if self.cancel {
            pairs.push(("kind", json!(CANCEL_KIND)));
        }
        pairs.extend([
            (
                "note",
                object(vec![
                    ("sections", Value::Array(sections)),
                    ("text", json!(self.text)),
                ]),
            ),
            ("routines", Value::Array(routines)),
            ("version", json!(self.version)),
        ]);
        object(pairs).to_string()
    }

    /// The item `text` holds; none when it is none.
    pub fn parse(text: &str) -> Option<SentItem> {
        let json: Value = serde_json::from_str(text).ok()?;
        let from = json.get("from")?;
        let note = json.get("note")?;
        let id = json.get("id")?.as_str()?;
        let version = json.get("version")?.as_i64()?;
        if id.is_empty() || version < 1 {
            return None;
        }
        let cancel = match json.get("kind") {
            None => false,
            Some(kind) if kind.as_str() == Some(CANCEL_KIND) => true,
            Some(_) => return None,
        };
        let mut sections = Vec::new();
        for s in note.get("sections")?.as_array()? {
            sections.push(SentSection {
                text: s.get("text")?.as_str()?.to_string(),
                captured: parse_time(s.get("captured")?.as_str()?).ok()?,
            });
        }
        let mut routines = Vec::new();
        for r in json.get("routines")?.as_array()? {
            let rule = match r.get("rule") {
                None | Some(Value::Null) => None,
                Some(rule) => Some(rule.as_str()?.to_string()),
            };
            let mut fields = vec![
                (
                    routine_keys::START.to_string(),
                    Some(r.get("start")?.as_str()?.to_string()),
                ),
                (
                    routine_keys::TIMES.to_string(),
                    Some(r.get("times")?.as_str()?.to_string()),
                ),
            ];
            if rule.is_some() {
                fields.push((routine_keys::RULE.to_string(), rule));
            }
            routines.push(SentRoutine {
                id: r.get("id")?.as_str()?.to_string(),
                schedule: Routines::schedule_of(&Fields::new(fields))?,
            });
        }
        Some(SentItem {
            id: id.to_string(),
            version,
            cancel,
            from: KeyRecord::from_json(from.get("key")?)?,
            name: from.get("name")?.as_str()?.to_string(),
            text: note.get("text")?.as_str()?.to_string(),
            sections,
            routines,
        })
    }
}

fn times(schedule: &Schedule) -> String {
    schedule
        .times
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Someone a Sent Item can be encrypted to, with the X25519 keys of
/// every Device of theirs known here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    /// The person's id, or the Device's when no Shared Area names one.
    pub id: String,
    pub name: String,
    pub keys: Vec<[u8; BOX_KEY_BYTES]>,
}

impl Store {
    /// Who a Sent Item can be encrypted to: the people of the Shared Areas
    /// and the Devices paired by invite, never this Device's own group,
    /// each with the keys of their Devices known here; by name. A person
    /// none of whose keys is known is not listed.
    pub fn sent_item_recipients(&self) -> Result<Vec<Recipient>> {
        let me = self.device_id()?;
        let mut devices: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
            Default::default();
        let mut names: std::collections::HashMap<String, String> = Default::default();
        let areas: Vec<String> = self.judged()?.areas.keys().cloned().collect();
        for area in areas {
            'people: for person in self.people(&area)? {
                for d in &person.devices {
                    if *d == me || self.is_own_device(d)? {
                        continue 'people;
                    }
                }
                devices
                    .entry(person.id.clone())
                    .or_default()
                    .extend(person.devices.iter().cloned());
                names.entry(person.id.clone()).or_insert(person.name);
            }
        }
        let grouped: std::collections::HashSet<String> =
            devices.values().flatten().cloned().collect();
        for paired in self.paired_devices()? {
            if !paired.shared || grouped.contains(&paired.id) {
                continue;
            }
            devices
                .entry(paired.id.clone())
                .or_default()
                .insert(paired.id.clone());
            names.entry(paired.id.clone()).or_insert(paired.name);
        }
        let mut out = Vec::new();
        for (id, ds) in devices {
            let mut keys = Vec::new();
            for d in ds {
                if let Some(key) = self
                    .pinned_key(&d)?
                    .and_then(|p| p.record.verified_box_key())
                {
                    keys.push(key);
                }
            }
            if keys.is_empty() {
                continue;
            }
            let name = names.get(&id).cloned().unwrap_or_default();
            out.push(Recipient { id, name, keys });
        }
        out.sort_by(|a, b| crate::store::utf16_cmp(&a.name, &b.name).then_with(|| a.id.cmp(&b.id)));
        Ok(out)
    }

    /// The Sent Item the Note `note_id` goes out as now, under the id it
    /// went out with from this Device before and the next version, or a
    /// new id; records both on the Note when this Device may change it.
    /// With `cancel` it goes out as the cancellation of that id.
    pub fn sent_item_of(&self, note_id: &str, cancel: bool) -> Result<SentItem> {
        let notes = Notes::new(self);
        let note = notes
            .by_id(note_id)?
            .filter(|n| !n.is_trashed() && !n.is_merged())
            .ok_or_else(|| Error::Invalid(format!("not a Note in the lists: {note_id}")))?;
        let field = keys::sent(&self.device_id()?);
        let held = self
            .current(note_id, &field)?
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|json| {
                let id = json.get("id")?.as_str()?.to_string();
                let version = json.get("version")?.as_i64()?;
                (!id.is_empty()).then_some((id, version + 1))
            });
        let (id, version) = match held {
            Some(held) => held,
            None => (self.new_id()?, 1),
        };
        if self.may_change(note_id)? {
            let value = object(vec![("id", json!(id)), ("version", json!(version))]);
            self.append(note_id, &field, Some(&value.to_string()), None)?;
        }
        let mut routines = Routines::new(self).of_note(note_id)?;
        routines.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(SentItem {
            id,
            version,
            cancel,
            from: self.own_key_record()?,
            name: self.author()?,
            text: note.text,
            sections: notes
                .sections(note_id)?
                .into_iter()
                .filter(|s| !s.text.trim().is_empty())
                .map(|s| SentSection {
                    text: s.text,
                    captured: s.captured,
                })
                .collect(),
            routines: routines
                .into_iter()
                .map(|r| SentRoutine {
                    id: r.id,
                    schedule: r.schedule,
                })
                .collect(),
        })
    }

    /// Whether this Device sent the Note `note_id` before, so sending it
    /// again sends an update, or a cancellation.
    pub fn was_sent(&self, note_id: &str) -> Result<bool> {
        Ok(self
            .current(note_id, &keys::sent(&self.device_id()?))?
            .is_some())
    }

    /// Writes the file the Note `note_id` goes out as: encrypted to the
    /// keys of `to`, else signed only; with `cancel` as its cancellation.
    pub fn send_note(
        &self,
        note_id: &str,
        to: Option<&Recipient>,
        cancel: bool,
    ) -> Result<Vec<u8>> {
        let item = self.sent_item_of(note_id, cancel)?;
        let keys = to.map(|r| r.keys.clone()).unwrap_or_default();
        encode(&item, &self.signing_key()?, &keys, &mut |n| {
            let mut out = vec![0u8; n];
            getrandom::fill(&mut out).map_err(|e| Error::Invalid(format!("random: {e}")))?;
            Ok(out)
        })
    }

    /// The Note `item` was accepted as before, from the same key, in the
    /// trash or merged too: the newest of them.
    pub fn accepted_note(&self, item: &SentItem) -> Result<Option<String>> {
        Ok(self.held(item)?.0.map(|h| h.note))
    }

    /// Accepts `item` as [`Store::open_sent_item`] judged it
    /// ([`SentItemState`]): a new Note in Unsorted, captured now, with its
    /// Sections and its Routines and Reminders, alarms on, linked to the
    /// Sent Item; for an update, its changes applied to the Note accepted
    /// before ([`SentItemChanges`]); for a cancellation, that Note moved
    /// to the trash with its Routines and Reminders. Returns the Note it
    /// is held as; `None` when there was nothing to cancel. One whose id
    /// came here from another key only is refused.
    pub fn accept_sent_item(&self, item: &SentItem) -> Result<Option<String>> {
        let (held, foreign) = self.held(item)?;
        if held.is_none() && foreign {
            return Err(Error::Invalid(
                "a Sent Item of this id came from another key".into(),
            ));
        }
        let (state, changes) = self.judge(item, held.as_ref())?;
        let now = self.now();
        match (state, held, changes) {
            (SentItemState::Fresh, _, _) => self.accept_new(item).map(Some),
            (SentItemState::Update, Some(held), Some(changes)) => {
                self.apply(&held.note, item, &changes, now)?;
                Ok(Some(held.note))
            }
            (SentItemState::Cancel, Some(held), _) => {
                self.cancel(&held.note, item, now)?;
                Ok(Some(held.note))
            }
            (SentItemState::UpToDate, Some(held), _) => Ok(Some(held.note)),
            _ => Ok(None),
        }
    }

    /// The Note `item` was accepted as here from the same key, in the
    /// trash or merged too, the newest of them, with the version it holds;
    /// and whether another key's Sent Item has its id.
    fn held(&self, item: &SentItem) -> Result<(Option<Held>, bool)> {
        let from = BASE64.encode(&item.from.public_key);
        let notes = Notes::new(self);
        let mut found: Option<(DateTime<Utc>, Held)> = None;
        let mut foreign = false;
        for id in self.entities_of_type(note_keys::TYPE_NOTE)? {
            let Some(link) = self
                .current(&id, keys::RECEIVED)?
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            else {
                continue;
            };
            let (Some(link_from), Some(link_id), Some(version)) = (
                link.get("from").and_then(Value::as_str),
                link.get("id").and_then(Value::as_str),
                link.get("version").and_then(Value::as_i64),
            ) else {
                continue;
            };
            if link_id != item.id {
                continue;
            }
            if link_from != from {
                foreign = true;
                continue;
            }
            let Some(note) = notes.by_id(&id)? else {
                continue;
            };
            if found.as_ref().is_none_or(|(at, _)| note.captured > *at) {
                found = Some((note.captured, Held { note: id, version }));
            }
        }
        Ok((found.map(|(_, held)| held), foreign))
    }

    /// What accepting `item` comes to, with the Note it was accepted as
    /// before, and for an update what it changes.
    fn judge(
        &self,
        item: &SentItem,
        held: Option<&Held>,
    ) -> Result<(SentItemState, Option<SentItemChanges>)> {
        let Some(held) = held else {
            let state = if item.cancel {
                SentItemState::NothingToCancel
            } else {
                SentItemState::Fresh
            };
            return Ok((state, None));
        };
        if item.version <= held.version {
            return Ok((SentItemState::UpToDate, None));
        }
        let trashed = Notes::new(self)
            .by_id(&held.note)?
            .is_some_and(|n| n.is_trashed());
        if item.cancel {
            let state = if trashed {
                SentItemState::NothingToCancel
            } else {
                SentItemState::Cancel
            };
            return Ok((state, None));
        }
        let changes = self.changes(&held.note, item, trashed)?;
        Ok((SentItemState::Update, Some(changes)))
    }

    /// What the update `item` changes on the Note `note_id`, as the Dart
    /// core works it out: its text when the sender changed theirs, the
    /// Sections not here yet (by when they were captured), and the
    /// sender's Routines and Reminders, known by the sender's id for each,
    /// rescheduled, added or taken away. Those the receiver put in the
    /// trash on their own stay there; when the Note is in the trash
    /// (`restores`) those that went with it count.
    fn changes(&self, note_id: &str, item: &SentItem, restores: bool) -> Result<SentItemChanges> {
        let notes = Notes::new(self);
        let text = notes.by_id(note_id)?.map(|n| n.text).unwrap_or_default();
        let sent = self.last_sent_text(note_id)?;
        let changed = item.text != text && sent.as_ref().is_none_or(|s| item.text != *s);
        let here: std::collections::HashSet<DateTime<Utc>> = notes
            .sections(note_id)?
            .into_iter()
            .map(|s| s.captured)
            .collect();
        let routines = Routines::new(self);
        let mut theirs: Vec<(String, Routine)> = Vec::new();
        let mut ids = self.entities_of_type(routine_keys::TYPE_ROUTINE)?;
        ids.sort();
        for id in ids {
            let Some(routine) = routines.by_id(&id)? else {
                continue;
            };
            if routine.note_id != note_id {
                continue;
            }
            let Some(from) = self.current(&id, keys::RECEIVED)? else {
                continue;
            };
            if !theirs.iter().any(|(f, _)| *f == from) {
                theirs.push((from, routine));
            }
        }
        let live = |r: &Routine| {
            !r.is_trashed() || (restores && r.trashed_with.as_deref() == Some(note_id))
        };
        let mut rescheduled = Vec::new();
        let mut routines_added = Vec::new();
        for r in &item.routines {
            match theirs.iter().find(|(f, _)| *f == r.id) {
                None => routines_added.push(r.clone()),
                Some((_, held)) if live(held) && !same_schedule(&held.schedule, &r.schedule) => {
                    rescheduled.push(SentReschedule {
                        routine: held.id.clone(),
                        before: held.schedule.clone(),
                        after: r.schedule.clone(),
                    });
                }
                Some(_) => {}
            }
        }
        let routines_removed = theirs
            .into_iter()
            .filter(|(from, r)| !item.routines.iter().any(|s| s.id == *from) && live(r))
            .map(|(_, r)| r)
            .collect();
        Ok(SentItemChanges {
            text_before: changed.then_some(text.clone()),
            text_edited: changed && sent.as_ref().is_some_and(|s| *s != text),
            sections_added: item
                .sections
                .iter()
                .filter(|s| !here.contains(&s.captured))
                .cloned()
                .collect(),
            rescheduled,
            routines_added,
            routines_removed,
            restores,
        })
    }

    /// The text the sender last sent for the Note `note_id`: the newest
    /// text written with its link, as accepting and applying write both
    /// at once.
    fn last_sent_text(&self, note_id: &str) -> Result<Option<String>> {
        let history = self.history(note_id)?;
        let linked: std::collections::HashSet<DateTime<Utc>> = history
            .iter()
            .filter(|e| e.field == keys::RECEIVED)
            .map(|e| e.date_at)
            .collect();
        let mut last: Option<&crate::Entry> = None;
        for e in &history {
            if e.field != note_keys::TEXT || !linked.contains(&e.date_at) {
                continue;
            }
            if last.is_none_or(|l| e.date_at >= l.date_at) {
                last = Some(e);
            }
        }
        Ok(last.and_then(|e| e.value.clone()))
    }

    /// A new Note in Unsorted for `item`, captured now.
    fn accept_new(&self, item: &SentItem) -> Result<String> {
        let id = format!("note:{}", self.new_id()?);
        let now = self.now();
        let at = Some(now);
        self.batch(|| {
            self.append(&id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), at)?;
            self.append(&id, note_keys::CAPTURED, Some(&dart_iso(now)), at)?;
            self.append(&id, note_keys::TEXT, Some(&item.text), at)?;
            self.append(&id, keys::RECEIVED, Some(&link(item)), at)?;
            for s in &item.sections {
                self.write_section(&id, s)?;
            }
            // Written as they are, not spawned: the Note stays in Unsorted.
            for r in &item.routines {
                self.write_routine(&id, r)?;
            }
            Ok(())
        })?;
        Ok(id)
    }

    /// Applies the update `item` to the Note `note_id` as `changes` says,
    /// at `now`, in one go, in the Dart core's order.
    fn apply(
        &self,
        note_id: &str,
        item: &SentItem,
        changes: &SentItemChanges,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let at = dart_iso(now);
        let routines = Routines::new(self);
        self.batch(|| {
            if changes.restores {
                self.append(note_id, note_keys::TRASHED, None, None)?;
                let mut ids = self.entities_of_type(routine_keys::TYPE_ROUTINE)?;
                ids.sort();
                for id in ids {
                    if routines
                        .by_id(&id)?
                        .is_some_and(|r| r.trashed_with.as_deref() == Some(note_id))
                    {
                        self.append(&id, routine_keys::TRASHED, None, None)?;
                        self.append(&id, routine_keys::TRASHED_WITH, None, None)?;
                    }
                }
            }
            if changes.text_before.is_some() {
                self.append(note_id, note_keys::TEXT, Some(&item.text), Some(now))?;
            }
            self.append(note_id, keys::RECEIVED, Some(&link(item)), Some(now))?;
            for s in &changes.sections_added {
                self.write_section(note_id, s)?;
            }
            for r in &changes.rescheduled {
                routines.write_schedule(&r.routine, &r.after)?;
            }
            for r in &changes.routines_added {
                self.write_routine(note_id, r)?;
            }
            for r in &changes.routines_removed {
                self.append(&r.id, routine_keys::TRASHED, Some(&at), None)?;
            }
            Ok(())
        })
    }

    /// Moves the Note `note_id` to the trash for the cancellation `item`,
    /// at `now`, with its Routines and Reminders: they come back with it.
    /// Those of its own count even when the receiver merged it into
    /// another Note.
    fn cancel(&self, note_id: &str, item: &SentItem, now: DateTime<Utc>) -> Result<()> {
        let at = dart_iso(now);
        let routines = Routines::new(self);
        let mut ids: std::collections::BTreeSet<String> = routines
            .of_note(note_id)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        for id in self.entities_of_type(routine_keys::TYPE_ROUTINE)? {
            if routines
                .by_id(&id)?
                .is_some_and(|r| r.note_id == note_id && !r.is_trashed())
            {
                ids.insert(id);
            }
        }
        self.batch(|| {
            self.append(note_id, note_keys::TRASHED, Some(&at), None)?;
            for id in &ids {
                self.append(id, routine_keys::TRASHED, Some(&at), None)?;
                self.append(id, routine_keys::TRASHED_WITH, Some(note_id), None)?;
            }
            self.append(note_id, keys::RECEIVED, Some(&link(item)), Some(now))?;
            Ok(())
        })
    }

    /// A Section of the Note `note_id` holding `section`.
    fn write_section(&self, note_id: &str, section: &SentSection) -> Result<()> {
        let id = format!("section:{}", self.new_id()?);
        self.append(&id, note_keys::TYPE, Some(note_keys::TYPE_SECTION), None)?;
        self.append(&id, note_keys::NOTE, Some(note_id), None)?;
        self.append(
            &id,
            note_keys::CAPTURED,
            Some(&dart_iso(section.captured)),
            None,
        )?;
        self.append(&id, note_keys::TEXT, Some(&section.text), None)?;
        Ok(())
    }

    /// A Routine or Reminder of the Note `note_id` as `routine` is,
    /// knowing the sender's id for it; written, not spawned.
    fn write_routine(&self, note_id: &str, routine: &SentRoutine) -> Result<()> {
        let id = format!("routine:{}", self.new_id()?);
        let schedule = &routine.schedule;
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
        self.append(&id, keys::RECEIVED, Some(&routine.id), None)?;
        Ok(())
    }
}

/// The Note a Sent Item was accepted as here and the version it holds.
struct Held {
    note: String,
    version: i64,
}

fn same_schedule(a: &Schedule, b: &Schedule) -> bool {
    a.start == b.start
        && times(a) == times(b)
        && a.rule.as_ref().map(|r| r.format()) == b.rule.as_ref().map(|r| r.format())
}

/// The link an accepted Note keeps to `item`.
fn link(item: &SentItem) -> String {
    object(vec![
        ("from", json!(BASE64.encode(&item.from.public_key))),
        ("id", json!(item.id)),
        ("name", json!(item.name)),
        ("version", json!(item.version)),
    ])
    .to_string()
}

/// What accepting a Sent Item opened here comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentItemState {
    /// Never accepted here: accepting makes a new Note in Unsorted.
    Fresh,
    /// A newer version of one accepted here: accepting applies its
    /// [`SentItemChanges`] to that Note, wherever it is filed.
    Update,
    /// The cancellation of one accepted here: accepting moves that Note
    /// to the trash with its Routines and Reminders, alarms off.
    Cancel,
    /// This version or a newer one was accepted here: nothing to do.
    UpToDate,
    /// The cancellation of nothing accepted here, or of a Note already in
    /// the trash: nothing to do.
    NothingToCancel,
}

impl SentItemState {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            SentItemState::Fresh => "fresh",
            SentItemState::Update => "update",
            SentItemState::Cancel => "cancel",
            SentItemState::UpToDate => "upToDate",
            SentItemState::NothingToCancel => "nothingToCancel",
        }
    }
}

/// A Routine or Reminder an update gives another schedule: the one here,
/// its schedule now and the one it gets.
#[derive(Debug, Clone, PartialEq)]
pub struct SentReschedule {
    pub routine: String,
    pub before: Schedule,
    pub after: Schedule,
}

/// What applying an update changes on the Note accepted before. What the
/// receiver added stays: their Sections, Routines and Reminders, Done
/// marks, Importance and where the Note is filed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SentItemChanges {
    /// The Note's text here, when the update replaces it: only when the
    /// sender changed theirs since.
    pub text_before: Option<String>,
    /// Whether the text here was changed here since it came.
    pub text_edited: bool,
    /// Sections the sender added since; none is ever taken away.
    pub sections_added: Vec<SentSection>,
    pub rescheduled: Vec<SentReschedule>,
    pub routines_added: Vec<SentRoutine>,
    /// The sender's Routines and Reminders here that they took away: they
    /// go to the trash, restorable.
    pub routines_removed: Vec<Routine>,
    /// Whether the Note is in the trash here: applying brings it back.
    pub restores: bool,
}

impl SentItemChanges {
    /// Whether nothing but the version changes.
    pub fn is_empty(&self) -> bool {
        self.text_before.is_none()
            && self.sections_added.is_empty()
            && self.rescheduled.is_empty()
            && self.routines_added.is_empty()
            && self.routines_removed.is_empty()
            && !self.restores
    }
}

/// The file `item` goes out as, signed by `key`: signed only when `to`
/// is empty, else encrypted to each X25519 key of `to`. The bytes come
/// from `random` as the Dart core draws them: the content key, the
/// content's nonce, then per recipient its ephemeral secret and nonce.
pub fn encode(
    item: &SentItem,
    key: &SigningKey,
    to: &[[u8; BOX_KEY_BYTES]],
    random: &mut dyn FnMut(usize) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let payload = item.encode();
    let signed = object(vec![
        ("format", json!(FORMAT)),
        ("item", json!(payload)),
        ("sig", json!(key.sign(&signed_bytes(&payload)))),
    ])
    .to_string()
    .into_bytes();
    if to.is_empty() {
        return Ok(signed);
    }
    let content: [u8; AREA_KEY_BYTES] = array(random(AREA_KEY_BYTES)?)?;
    let nonce: [u8; SEAL_NONCE_BYTES] = array(random(SEAL_NONCE_BYTES)?)?;
    let sealed = seal(&content, &signed, SEALED_AAD, &nonce)?;
    let mut stanzas = Vec::new();
    for recipient in to {
        let ephemeral: [u8; BOX_KEY_BYTES] = array(random(BOX_KEY_BYTES)?)?;
        let public = box_public_of(&ephemeral);
        let shared = x25519(&ephemeral, recipient)
            .ok_or_else(|| Error::Invalid("not a key to encrypt to".into()))?;
        let nonce: [u8; SEAL_NONCE_BYTES] = array(random(SEAL_NONCE_BYTES)?)?;
        let wrapped = seal(
            &wrap_key(&shared, &public, recipient),
            &content,
            SENT_KEY_AAD,
            &nonce,
        )?;
        stanzas.push(object(vec![
            ("eph", json!(BASE64.encode(public))),
            ("key", json!(BASE64.encode(wrapped))),
        ]));
    }
    Ok(object(vec![
        ("format", json!(FORMAT)),
        ("sealed", json!(BASE64.encode(sealed))),
        ("to", Value::Array(stanzas)),
    ])
    .to_string()
    .into_bytes())
}

fn array<const N: usize>(bytes: Vec<u8>) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::Invalid("random: wrong length".into()))
}

fn signed_bytes(payload: &str) -> Vec<u8> {
    format!("{SIGNED_PREFIX}{payload}").into_bytes()
}

/// Who a Sent Item opened here comes from, as far as this Device knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderTrust {
    /// Their key was met in person.
    Verified,
    /// Their key came along with a sync, never met in person.
    Known,
    /// Never met.
    Unknown,
}

impl SenderTrust {
    pub fn name(self) -> &'static str {
        match self {
            SenderTrust::Verified => "verified",
            SenderTrust::Known => "known",
            SenderTrust::Unknown => "unknown",
        }
    }
}

/// A Sent Item opened here, to preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Opened {
    pub item: SentItem,
    /// Whether it came encrypted to this Device.
    pub encrypted: bool,
    pub trust: SenderTrust,
    /// The Note it was accepted as before, if it was, in the trash too.
    pub accepted: Option<String>,
    /// What accepting it comes to.
    pub state: SentItemState,
    /// What an update changes; `None` for anything else.
    pub changes: Option<SentItemChanges>,
}

/// Why a file could not be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    /// No Sent Item: no JSON, damaged, too large, or not signed by the
    /// key it names.
    NotASentItem,
    /// Encrypted to other Devices, not to this one.
    NotForUs,
    /// Its id came here from another key: only its sender may update or
    /// cancel it (ADR 0009).
    NotFromSender,
}

/// Whether `bytes` look like a Sent Item rather than a bundle: a JSON
/// object, where a bundle is a zip.
pub fn looks_like_sent_item(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .find(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
        .is_some_and(|b| *b == b'{')
}

impl Store {
    /// Reads the Sent Item in `bytes`: decrypts it when it is encrypted,
    /// checks the signature over it and what accepting it would do here.
    pub fn open_sent_item(&self, bytes: &[u8]) -> Result<std::result::Result<Opened, OpenError>> {
        if bytes.len() > MAX_BYTES {
            return Ok(Err(OpenError::NotASentItem));
        }
        let Ok(outer) = serde_json::from_slice::<Value>(bytes) else {
            return Ok(Err(OpenError::NotASentItem));
        };
        let mut signed = outer.clone();
        let mut encrypted = false;
        if outer.get("format").and_then(Value::as_i64) == Some(FORMAT)
            && let (Some(sealed), Some(to)) = (
                outer.get("sealed").and_then(Value::as_str),
                outer.get("to").and_then(Value::as_array),
            )
        {
            let Ok(body) = BASE64.decode(sealed) else {
                return Ok(Err(OpenError::NotASentItem));
            };
            let Some(inner) = self.open_sealed(&body, to)? else {
                return Ok(Err(OpenError::NotForUs));
            };
            let Ok(inner) = serde_json::from_slice::<Value>(&inner) else {
                return Ok(Err(OpenError::NotASentItem));
            };
            signed = inner;
            encrypted = true;
        }
        let (Some(FORMAT), Some(payload), Some(sig)) = (
            signed.get("format").and_then(Value::as_i64),
            signed.get("item").and_then(Value::as_str),
            signed.get("sig").and_then(Value::as_str),
        ) else {
            return Ok(Err(OpenError::NotASentItem));
        };
        let Some(item) = SentItem::parse(payload) else {
            return Ok(Err(OpenError::NotASentItem));
        };
        if !item.from.valid()
            || !verify_signature(&item.from.public_key, &signed_bytes(payload), Some(sig))
        {
            return Ok(Err(OpenError::NotASentItem));
        }
        let pinned = self.pinned_key(&item.from.device)?;
        let own = item.from.device == self.device_id()?;
        let trust = match pinned {
            _ if own => SenderTrust::Verified,
            Some(p) if p.trust == KeyTrust::Verified => SenderTrust::Verified,
            Some(_) => SenderTrust::Known,
            None => SenderTrust::Unknown,
        };
        let (held, foreign) = self.held(&item)?;
        if held.is_none() && foreign {
            return Ok(Err(OpenError::NotFromSender));
        }
        let (state, changes) = self.judge(&item, held.as_ref())?;
        Ok(Ok(Opened {
            item,
            encrypted,
            trust,
            accepted: held.map(|h| h.note),
            state,
            changes,
        }))
    }

    /// The signed file inside `body`, opened with the first stanza of
    /// `to` sealed for this Device's X25519 key.
    fn open_sealed(&self, body: &[u8], to: &[Value]) -> Result<Option<Vec<u8>>> {
        let secret = box_secret_of(self.signing_key()?.seed());
        let mine = box_public_of(&secret);
        for stanza in to {
            let (Some(eph), Some(wrapped)) = (
                stanza.get("eph").and_then(Value::as_str),
                stanza.get("key").and_then(Value::as_str),
            ) else {
                continue;
            };
            let Some(public) = BASE64
                .decode(eph)
                .ok()
                .and_then(|p| <[u8; BOX_KEY_BYTES]>::try_from(p).ok())
            else {
                continue;
            };
            let Ok(wrapped) = BASE64.decode(wrapped) else {
                continue;
            };
            let Some(shared) = x25519(&secret, &public) else {
                continue;
            };
            let Some(content) = unseal(&wrap_key(&shared, &public, &mine), &wrapped, SENT_KEY_AAD)
                .and_then(|c| <[u8; AREA_KEY_BYTES]>::try_from(c).ok())
            else {
                continue;
            };
            return Ok(unseal(&content, body, SEALED_AAD));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pairing::PairedDevice;
    use crate::store::tests::ticking;
    use crate::{ClockTime, Day, Recurrence};

    fn device(seed: u8) -> Store {
        match Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32]))) {
            Ok(s) => s,
            Err(e) => panic!("store: {e}"),
        }
    }

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn invite(store: &Store, other: &Store, name: &str) -> TestResult {
        store.remember_device(&PairedDevice {
            id: other.device_id()?,
            name: name.into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: parse_time("2026-01-01T00:00:00.000Z")?,
            shared: true,
        })?;
        let mut report = crate::ImportReport::default();
        let id = other.device_id()?;
        store.learn_keys(&[other.own_key_record()?], Some(&id), &mut report)?;
        Ok(())
    }

    #[test]
    fn a_note_goes_out_signed_under_one_id_and_lands_in_unsorted() -> TestResult {
        let ann = device(1);
        ann.set_local_setting("author", "Ann")?;
        let ben = device(2);
        let note = Notes::new(&ann).capture("Dentist", None, None)?;
        Notes::new(&ann).amend(&note.id, "Room 3")?;
        let schedule = Schedule::new(
            Day::parse("2026-03-02").ok_or("day")?,
            &[ClockTime::parse("08:00").ok_or("time")?],
            Some(Recurrence::parse("FREQ=DAILY").map_err(|_| "rule")?),
        )
        .ok_or("schedule")?;
        let sent = Routines::new(&ann).spawn(&note.id, &schedule)?;

        let file = ann.send_note(&note.id, None, false)?;
        assert!(looks_like_sent_item(&file));
        let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
        assert!(!opened.encrypted);
        assert_eq!(opened.trust, SenderTrust::Unknown);
        assert_eq!(opened.item.name, "Ann");
        assert_eq!(opened.item.version, 1);
        let again = ben
            .open_sent_item(&ann.send_note(&note.id, None, false)?)?
            .map_err(|_| "again")?;
        assert_eq!(again.item.id, opened.item.id);
        assert_eq!(again.item.version, 2);

        let id = ben.accept_sent_item(&opened.item)?.ok_or("accepted")?;
        let got = Notes::new(&ben).by_id(&id)?.ok_or("note")?;
        assert!(got.is_unsorted());
        assert_eq!(got.text, "Dentist");
        let routines = Routines::new(&ben).of_note(&id)?;
        assert_eq!(routines.len(), 1);
        assert_eq!(routines[0].schedule, schedule);
        assert_eq!(
            ben.current(&routines[0].id, keys::RECEIVED)?.as_deref(),
            Some(sent.id.as_str())
        );
        assert_eq!(
            ben.open_sent_item(&file)?.map(|o| o.state),
            Ok(SentItemState::UpToDate)
        );
        // Sent again unchanged: only the version moves on.
        let resent = ben
            .open_sent_item(&ann.send_note(&note.id, None, false)?)?
            .map_err(|_| "resent")?;
        assert_eq!(resent.state, SentItemState::Update);
        assert!(
            resent
                .changes
                .as_ref()
                .is_some_and(SentItemChanges::is_empty)
        );
        assert_eq!(
            ben.accept_sent_item(&resent.item)?.as_deref(),
            Some(id.as_str())
        );

        let mut changed = String::from_utf8(file)?;
        changed = changed.replacen("Dentist", "Dentlst", 1);
        assert_eq!(
            ben.open_sent_item(changed.as_bytes())?.err(),
            Some(OpenError::NotASentItem)
        );
        assert_eq!(
            ben.open_sent_item(b"PK\x03\x04")?.err(),
            Some(OpenError::NotASentItem)
        );
        Ok(())
    }

    #[test]
    fn an_update_keeps_the_receivers_text_and_brings_a_trashed_note_back() -> TestResult {
        let ann = device(1);
        let ben = device(2);
        let notes = Notes::new(&ann);
        let note = notes.capture("Dentist", None, None)?;
        let schedule = Schedule::new(
            Day::parse("2026-03-03").ok_or("day")?,
            &[ClockTime::parse("10:00").ok_or("time")?],
            None,
        )
        .ok_or("schedule")?;
        Routines::new(&ann).spawn(&note.id, &schedule)?;
        let first = ben
            .open_sent_item(&ann.send_note(&note.id, None, false)?)?
            .map_err(|_| "first")?;
        let got = ben.accept_sent_item(&first.item)?.ok_or("accepted")?;
        assert!(ann.was_sent(&note.id)?);

        // His own words stay while hers do not change.
        Notes::new(&ben).edit(&got, "Dentist (mine)")?;
        let same = ben
            .open_sent_item(&ann.send_note(&note.id, None, false)?)?
            .map_err(|_| "same")?;
        let changes = same.changes.clone().ok_or("changes")?;
        assert_eq!(changes.text_before, None);
        ben.accept_sent_item(&same.item)?;
        assert_eq!(
            Notes::new(&ben).by_id(&got)?.ok_or("note")?.text,
            "Dentist (mine)"
        );

        // In his trash: the cancellation finds nothing, an update brings
        // it back, its Reminder with it.
        Routines::new(&ben).trash_note(&got, true)?;
        let cancel = ben
            .open_sent_item(&ann.send_note(&note.id, None, true)?)?
            .map_err(|_| "cancel")?;
        assert_eq!(cancel.state, SentItemState::NothingToCancel);
        assert_eq!(ben.accept_sent_item(&cancel.item)?, None);
        notes.edit(&note.id, "Dentist moved")?;
        let update = ben
            .open_sent_item(&ann.send_note(&note.id, None, false)?)?
            .map_err(|_| "update")?;
        let changes = update.changes.clone().ok_or("changes")?;
        assert!(changes.restores);
        assert!(changes.text_edited);
        assert_eq!(changes.text_before.as_deref(), Some("Dentist (mine)"));
        ben.accept_sent_item(&update.item)?;
        let back = Notes::new(&ben).by_id(&got)?.ok_or("note")?;
        assert!(!back.is_trashed());
        assert_eq!(back.text, "Dentist moved");
        assert_eq!(Routines::new(&ben).of_note(&got)?.len(), 1);

        // Cancelled for real: in the trash, alarms off.
        let cancel = ben
            .open_sent_item(&ann.send_note(&note.id, None, true)?)?
            .map_err(|_| "cancel")?;
        assert_eq!(cancel.state, SentItemState::Cancel);
        ben.accept_sent_item(&cancel.item)?;
        assert!(Notes::new(&ben).by_id(&got)?.ok_or("note")?.is_trashed());
        assert!(Routines::new(&ben).all()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_person_met_gets_it_encrypted_and_nobody_else_reads_it() -> TestResult {
        let ann = device(1);
        let ben = device(2);
        let eve = device(3);
        invite(&ann, &ben, "Ben")?;
        let to = ann.sent_item_recipients()?;
        assert_eq!(to.len(), 1);
        assert_eq!(to[0].name, "Ben");
        let note = Notes::new(&ann).capture("Secret plan", None, None)?;
        let file = ann.send_note(&note.id, Some(&to[0]), false)?;
        assert!(!String::from_utf8_lossy(&file).contains("Secret"));
        let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
        assert!(opened.encrypted);
        assert_eq!(opened.item.text, "Secret plan");
        assert_eq!(eve.open_sent_item(&file)?.err(), Some(OpenError::NotForUs));

        // A key pinned without its X25519 key is not offered, until a
        // record with it comes along.
        let cid = device(4);
        let record = cid.own_key_record()?.bare();
        let mut report = crate::ImportReport::default();
        let other = device(5);
        other.learn_keys(&[record], None, &mut report)?;
        invite(&other, &cid, "Cid")?;
        assert_eq!(other.sent_item_recipients()?.len(), 1, "taken on learning");
        Ok(())
    }
}
