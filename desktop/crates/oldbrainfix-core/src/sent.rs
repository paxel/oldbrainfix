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
use crate::{Error, Notes, Result, Routines, Schedule};

/// The file ending of a Sent Item.
pub const EXTENSION: &str = "oldbrain";

/// The type a Sent Item is shared as.
pub const MIME_TYPE: &str = "application/vnd.oldbrainfix.item";

/// The format a file says it is in.
pub const FORMAT: i64 = 1;

/// A file larger than this is no Sent Item: it holds only texts.
pub const MAX_BYTES: usize = 1 << 20;

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
    /// What is signed: keys sorted, no spaces, as the Dart core writes it.
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
        object(vec![
            (
                "from",
                object(vec![
                    ("key", self.from.to_json()),
                    ("name", json!(self.name)),
                ]),
            ),
            ("id", json!(self.id)),
            (
                "note",
                object(vec![
                    ("sections", Value::Array(sections)),
                    ("text", json!(self.text)),
                ]),
            ),
            ("routines", Value::Array(routines)),
            ("version", json!(self.version)),
        ])
        .to_string()
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
    pub fn sent_item_of(&self, note_id: &str) -> Result<SentItem> {
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

    /// Writes the file the Note `note_id` goes out as: encrypted to the
    /// keys of `to`, else signed only.
    pub fn send_note(&self, note_id: &str, to: Option<&Recipient>) -> Result<Vec<u8>> {
        let item = self.sent_item_of(note_id)?;
        let keys = to.map(|r| r.keys.clone()).unwrap_or_default();
        encode(&item, &self.signing_key()?, &keys, &mut |n| {
            let mut out = vec![0u8; n];
            getrandom::fill(&mut out).map_err(|e| Error::Invalid(format!("random: {e}")))?;
            Ok(out)
        })
    }

    /// The Note `item` was accepted as before, from the same key: the
    /// newest not in the trash.
    pub fn accepted_note(&self, item: &SentItem) -> Result<Option<String>> {
        let from = BASE64.encode(&item.from.public_key);
        let notes = Notes::new(self);
        let mut found: Option<(DateTime<Utc>, String)> = None;
        for id in self.entities_of_type(note_keys::TYPE_NOTE)? {
            let Some(link) = self
                .current(&id, keys::RECEIVED)?
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            else {
                continue;
            };
            if link.get("from").and_then(Value::as_str) != Some(from.as_str())
                || link.get("id").and_then(Value::as_str) != Some(item.id.as_str())
            {
                continue;
            }
            let Some(note) = notes.by_id(&id)? else {
                continue;
            };
            if note.is_trashed() || note.is_merged() {
                continue;
            }
            if found.as_ref().is_none_or(|(at, _)| note.captured > *at) {
                found = Some((note.captured, id));
            }
        }
        Ok(found.map(|(_, id)| id))
    }

    /// Accepts `item`: a new Note in Unsorted, captured now, with its
    /// Sections and its Routines and Reminders, alarms on, linked to the
    /// Sent Item. One accepted before is not taken again.
    pub fn accept_sent_item(&self, item: &SentItem) -> Result<String> {
        if let Some(held) = self.accepted_note(item)? {
            return Ok(held);
        }
        let id = format!("note:{}", self.new_id()?);
        let now = self.now();
        let at = Some(now);
        let link = object(vec![
            ("from", json!(BASE64.encode(&item.from.public_key))),
            ("id", json!(item.id)),
            ("name", json!(item.name)),
            ("version", json!(item.version)),
        ])
        .to_string();
        self.batch(|| {
            self.append(&id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), at)?;
            self.append(&id, note_keys::CAPTURED, Some(&dart_iso(now)), at)?;
            self.append(&id, note_keys::TEXT, Some(&item.text), at)?;
            self.append(&id, keys::RECEIVED, Some(&link), at)?;
            for s in &item.sections {
                let section = format!("section:{}", self.new_id()?);
                self.append(
                    &section,
                    note_keys::TYPE,
                    Some(note_keys::TYPE_SECTION),
                    None,
                )?;
                self.append(&section, note_keys::NOTE, Some(&id), None)?;
                self.append(
                    &section,
                    note_keys::CAPTURED,
                    Some(&dart_iso(s.captured)),
                    None,
                )?;
                self.append(&section, note_keys::TEXT, Some(&s.text), None)?;
            }
            // Written as they are, not spawned: the Note stays in Unsorted.
            for r in &item.routines {
                let routine = format!("routine:{}", self.new_id()?);
                self.append(
                    &routine,
                    routine_keys::TYPE,
                    Some(routine_keys::TYPE_ROUTINE),
                    None,
                )?;
                self.append(&routine, routine_keys::NOTE, Some(&id), None)?;
                self.append(
                    &routine,
                    routine_keys::START,
                    Some(&r.schedule.start.to_string()),
                    None,
                )?;
                self.append(
                    &routine,
                    routine_keys::TIMES,
                    Some(&times(&r.schedule)),
                    None,
                )?;
                if let Some(rule) = &r.schedule.rule {
                    self.append(&routine, routine_keys::RULE, Some(&rule.format()), None)?;
                }
                self.append(&routine, keys::RECEIVED, Some(&r.id), None)?;
            }
            Ok(())
        })?;
        Ok(id)
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
    /// The Note it was accepted as before, if it was.
    pub accepted: Option<String>,
}

/// Why a file could not be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    /// No Sent Item: no JSON, damaged, too large, or not signed by the
    /// key it names.
    NotASentItem,
    /// Encrypted to other Devices, not to this one.
    NotForUs,
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
    /// Reads the Sent Item in `bytes`: decrypts it when it is encrypted
    /// and checks the signature over it.
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
        let accepted = self.accepted_note(&item)?;
        Ok(Ok(Opened {
            item,
            encrypted,
            trust,
            accepted,
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

        let file = ann.send_note(&note.id, None)?;
        assert!(looks_like_sent_item(&file));
        let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
        assert!(!opened.encrypted);
        assert_eq!(opened.trust, SenderTrust::Unknown);
        assert_eq!(opened.item.name, "Ann");
        assert_eq!(opened.item.version, 1);
        let again = ben
            .open_sent_item(&ann.send_note(&note.id, None)?)?
            .map_err(|_| "again")?;
        assert_eq!(again.item.id, opened.item.id);
        assert_eq!(again.item.version, 2);

        let id = ben.accept_sent_item(&opened.item)?;
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
        assert_eq!(ben.accept_sent_item(&again.item)?, id);

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
    fn a_person_met_gets_it_encrypted_and_nobody_else_reads_it() -> TestResult {
        let ann = device(1);
        let ben = device(2);
        let eve = device(3);
        invite(&ann, &ben, "Ben")?;
        let to = ann.sent_item_recipients()?;
        assert_eq!(to.len(), 1);
        assert_eq!(to[0].name, "Ben");
        let note = Notes::new(&ann).capture("Secret plan", None, None)?;
        let file = ann.send_note(&note.id, Some(&to[0]))?;
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
