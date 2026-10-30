//! Delta exchange between Devices, as the Dart core's `Sync` does it
//! (ADR 0002, 0003): every Device tells the other the newest entry it
//! holds from each Device, only what is missing travels, the photo and
//! voice recording bytes after it, and keys travel along so every entry
//! is checked against the key of the Device that signed it. Local
//! settings never travel (ADR 0008).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::Result;
use crate::blocks::wire_kept_from_blocked;
use crate::entry::{Entry, WireEntry};
use crate::notes::keys as note_keys;
use crate::signing::{
    ImportReport, KeyRecord, KeyTrust, PinnedKey, entry_bytes_with, verify_signature,
};
use crate::store::Store;

const KEY_PREFIX: &str = "key:";

/// The blob kinds and the field prefixes naming them.
pub const BLOB_KINDS: [(&str, &str); 2] = [
    ("photo", note_keys::PHOTO_PREFIX),
    ("audio", note_keys::AUDIO_PREFIX),
];

/// The kind and hash of a blob reference `photo/<hash>`.
pub fn split_blob_ref(reference: &str) -> (&str, &str) {
    reference.split_once('/').unwrap_or(("", reference))
}

/// Why a Moment was recorded.
pub mod cause {
    pub const SYNC: &str = "sync";
    /// A folder sync brought it.
    pub const FOLDER: &str = "folder";
    /// A bundle file brought it.
    pub const BUNDLE: &str = "bundle";
}

/// A state of the log a Device can be returned to: the log's high-water
/// mark before a change. Recorded before every sync that brings
/// anything; going back is not offered yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moment {
    pub id: i64,
    pub seq: i64,
    pub cause: String,
    pub label: Option<String>,
    pub at: String,
}

impl Store {
    /// This Device's own key record.
    pub fn own_key_record(&self) -> Result<KeyRecord> {
        Ok(KeyRecord::make(&self.signing_key()?))
    }

    /// The key pinned for `device`, if it was ever met.
    pub fn pinned_key(&self, device: &str) -> Result<Option<PinnedKey>> {
        Ok(self
            .local_setting(&format!("{KEY_PREFIX}{device}"))?
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .and_then(|json| PinnedKey::from_json(&json)))
    }

    /// Every key this Device holds for other Devices, by device.
    pub fn pinned_keys(&self) -> Result<Vec<PinnedKey>> {
        Ok(self
            .local_settings_with_prefix(KEY_PREFIX)?
            .into_iter()
            .filter_map(|(_, raw)| serde_json::from_str(&raw).ok())
            .filter_map(|json| PinnedKey::from_json(&json))
            .collect())
    }

    /// The keys that go out with the entries: this Device's own and every
    /// one it holds, by device.
    pub fn key_records(&self) -> Result<Vec<KeyRecord>> {
        let mut keys = vec![self.own_key_record()?];
        keys.extend(self.pinned_keys()?.into_iter().map(|k| k.record));
        keys.sort_by(|a, b| a.device.cmp(&b.device));
        Ok(keys)
    }

    fn pin(&self, record: &KeyRecord, trust: KeyTrust) -> Result<()> {
        let pinned = PinnedKey {
            record: record.clone(),
            trust,
        };
        self.set_local_setting(
            &format!("{KEY_PREFIX}{}", record.device),
            &pinned.to_json().to_string(),
        )
    }

    /// Takes in the keys a payload carried. A record that is not valid is
    /// ignored; the first key of a Device is pinned on trust, the one of
    /// `verified_device` as verified. A key held without the X25519 key
    /// Sent Items are encrypted to (ADR 0015) takes it from a record that
    /// carries it, the trust kept.
    pub fn learn_keys(
        &self,
        keys: &[KeyRecord],
        verified_device: Option<&str>,
        report: &mut ImportReport,
    ) -> Result<()> {
        let own = self.device_id()?;
        for record in keys {
            if record.device == own || !record.valid() {
                continue;
            }
            let verified = verified_device == Some(record.device.as_str());
            match self.pinned_key(&record.device)? {
                None => {
                    let trust = if verified {
                        KeyTrust::Verified
                    } else {
                        KeyTrust::Tofu
                    };
                    self.pin(record, trust)?;
                    report.new_keys.push(PinnedKey {
                        record: record.clone(),
                        trust,
                    });
                }
                Some(held)
                    if held.record.verified_box_key().is_none()
                        && record.verified_box_key().is_some() =>
                {
                    let trust = if verified {
                        KeyTrust::Verified
                    } else {
                        held.trust
                    };
                    self.pin(record, trust)?;
                }
                Some(held) if verified && held.trust == KeyTrust::Tofu => {
                    self.pin(&held.record, KeyTrust::Verified)?;
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Takes in entries another Device sent, in one transaction, and
    /// returns the ones that were new here, as the Dart core's
    /// `applyEntries`: keys first, then every entry not held checked
    /// against its Device's key; what does not verify is refused and
    /// counted, never recorded. An entry of a later structure is kept and
    /// passed on, but not applied nor returned. What a blocked person's
    /// Device wrote is refused but their check-offs, its dseq noted so it
    /// is not asked for again (ADR 0021), and what a Device found to be
    /// theirs only now is deleted afterwards ([`Store::enforce_blocks`]).
    pub fn apply_entries(
        &self,
        entries: &[WireEntry],
        keys: &[KeyRecord],
        verified_device: Option<&str>,
        report: &mut ImportReport,
    ) -> Result<Vec<WireEntry>> {
        let mut applied = self.apply_entries_only(entries, keys, verified_device, report)?;
        // A block from another of the user's Devices, or a grant naming a
        // blocked person, takes effect at once (ADR 0021); a photo or
        // recording removed elsewhere goes here too (ADR 0020). Anything
        // new may decide either, a grant or a filing as well.
        if !applied.is_empty() {
            if self.enforce_blocks()? {
                let mut kept = Vec::new();
                for e in applied {
                    if self.holds(&e.device, e.dseq)? {
                        kept.push(e);
                    }
                }
                applied = kept;
            }
            self.drop_removed_blobs()?;
        }
        Ok(applied)
    }

    fn apply_entries_only(
        &self,
        entries: &[WireEntry],
        keys: &[KeyRecord],
        verified_device: Option<&str>,
        report: &mut ImportReport,
    ) -> Result<Vec<WireEntry>> {
        self.batch(|| {
            self.learn_keys(keys, verified_device, report)?;
            let own = self.device_id()?;
            let own_max = self.version_vector()?.get(&own).copied().unwrap_or(0);
            let own_key = self.signing_key()?.public_key().to_vec();
            let mut known: HashMap<String, Option<Vec<u8>>> = HashMap::new();
            let refusing = self.blocked_devices()?;
            let mut refused: Vec<&WireEntry> = Vec::new();
            let mut applied = Vec::new();
            for e in entries {
                if self.holds(&e.device, e.dseq)? {
                    continue;
                }
                let key = if e.device == own {
                    (e.dseq <= own_max).then(|| own_key.clone())
                } else {
                    match known.get(&e.device) {
                        Some(key) => key.clone(),
                        None => {
                            let key = self.pinned_key(&e.device)?.map(|k| k.record.public_key);
                            known.insert(e.device.clone(), key.clone());
                            key
                        }
                    }
                };
                let bytes = entry_bytes_with(
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
                );
                let signed = key.is_some_and(|k| verify_signature(&k, &bytes, e.sig.as_deref()));
                if !signed {
                    report.refuse(&e.author, &e.device);
                    continue;
                }
                // Checked first, so nobody moves the mark with a forged
                // entry.
                if refusing.contains(&e.device) && !wire_kept_from_blocked(e) {
                    refused.push(e);
                    continue;
                }
                // One of a later structure is kept and passed on, not
                // applied.
                if self.insert(e)? && e.applies() {
                    applied.push(e.clone());
                }
            }
            // Where each refused entity lay, once what is kept of this
            // batch is in: a Note by its filing, a Section by the Note it
            // names.
            let mut links: HashMap<&str, &str> = HashMap::new();
            for e in &refused {
                if e.field == note_keys::NOTE
                    && let Some(note) = e.value.as_deref()
                {
                    links.insert(&e.entity, note);
                }
            }
            for e in &refused {
                let of = links.get(e.entity.as_str()).copied().unwrap_or(&e.entity);
                self.note_refused(&e.device, e.dseq, &self.scope_of(of)?)?;
            }
            Ok(applied)
        })
    }

    /// The blobs the log refers to whose bytes are not here, as
    /// `photo/<hash>` and `audio/<hash>`, sorted.
    pub fn missing_blobs(&self) -> Result<Vec<String>> {
        let mut missing = BTreeSet::new();
        for (kind, prefix) in BLOB_KINDS {
            for (entity, field) in self.fields_with_prefix(prefix)? {
                let hash = &field[prefix.len()..];
                if self.current(&entity, &field)?.is_none() {
                    continue;
                }
                if self.blob(kind, hash).is_none() {
                    missing.insert(format!("{kind}/{hash}"));
                }
            }
        }
        Ok(missing.into_iter().collect())
    }

    /// Whether the log refers to the blob `reference` now: only those
    /// are taken from another Device. A removed one stays out (ADR 0020).
    pub fn knows_blob(&self, reference: &str) -> Result<bool> {
        let (kind, hash) = split_blob_ref(reference);
        for (k, prefix) in BLOB_KINDS {
            if k != kind {
                continue;
            }
            let field = format!("{prefix}{hash}");
            for (entity, f) in self.fields_with_prefix(&field)? {
                if f == field && self.current(&entity, &f)?.is_some() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// The blobs the log referred to and lets go of now, as the Dart
    /// core's `removedBlobs`, sorted (ADR 0020): every field that named
    /// one was last set to null by a removal that may let the bytes go,
    /// one written by the user's own Devices or one on a Note that lies
    /// in a Shared Area, where its role was judged. A removal another
    /// person wrote anywhere else hides the file but keeps its bytes.
    pub fn removed_blobs(&self) -> Result<Vec<String>> {
        let mut own: HashMap<String, bool> = HashMap::new();
        let mut histories: HashMap<String, Vec<Entry>> = HashMap::new();
        let mut removed: BTreeMap<String, bool> = BTreeMap::new();
        for (kind, prefix) in BLOB_KINDS {
            for (entity, field) in self.fields_with_prefix(prefix)? {
                let reference = format!("{kind}/{}", &field[prefix.len()..]);
                if removed.get(&reference) == Some(&false) {
                    continue;
                }
                if !histories.contains_key(&entity) {
                    histories.insert(entity.clone(), self.history(&entity)?);
                }
                let mut last: Option<&Entry> = None;
                for e in histories.get(&entity).into_iter().flatten() {
                    if e.field != field {
                        continue;
                    }
                    if last.is_none_or(|held| Self::latest_wins(e, held) == Ordering::Greater) {
                        last = Some(e);
                    }
                }
                let lets_go = match last {
                    Some(last) => self.lets_go(last, &mut own)?,
                    None => false,
                };
                removed.insert(reference, lets_go);
            }
        }
        Ok(removed
            .into_iter()
            .filter_map(|(reference, gone)| gone.then_some(reference))
            .collect())
    }

    /// Whether `last`, the entry a blob's field holds, lets its bytes go:
    /// a removal by one of the user's own Devices, whose answers `own`
    /// keeps, or one on a Note in a Shared Area.
    fn lets_go(&self, last: &Entry, own: &mut HashMap<String, bool>) -> Result<bool> {
        if last.value.is_some() {
            return Ok(false);
        }
        let by_own = match own.get(&last.device) {
            Some(known) => *known,
            None => {
                let known = self.written_by_own(&last.device)?;
                own.insert(last.device.clone(), known);
                known
            }
        };
        if by_own {
            return Ok(true);
        }
        match self.area_of(&last.entity)? {
            Some(area) => self.is_shared(&area),
            None => Ok(false),
        }
    }

    /// Lets go of the bytes of every blob in [`Store::removed_blobs`]
    /// held here; returns those that were here.
    pub fn drop_removed_blobs(&self) -> Result<Vec<String>> {
        let mut dropped = Vec::new();
        for reference in self.removed_blobs()? {
            let (kind, hash) = split_blob_ref(&reference);
            if self.drop_blob(kind, hash)? {
                dropped.push(reference);
            }
        }
        Ok(dropped)
    }

    /// Every recorded Moment, newest first.
    pub fn moments(&self) -> Result<Vec<Moment>> {
        Ok(self
            .moment_rows()?
            .into_iter()
            .map(|r| Moment {
                id: r.id,
                seq: r.seq,
                cause: r.cause,
                label: r.label,
                at: r.at,
            })
            .collect())
    }

    /// Records the Moment before a change that already happened, given
    /// the mark `before`; a change that brought nothing records nothing.
    pub fn moment_for(
        &self,
        before: i64,
        changed: bool,
        cause: &str,
        label: Option<&str>,
    ) -> Result<Option<Moment>> {
        if !changed {
            return Ok(None);
        }
        let id = self.add_moment(before, cause, label)?;
        Ok(self.moments()?.into_iter().find(|m| m.id == id))
    }
}

/// Which Device needs a newer version of the app before two Devices can
/// sync (ADR 0019), as the Dart core's `UpdateNeeded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateNeeded {
    /// This one.
    Here,
    /// The other one.
    There,
}

impl UpdateNeeded {
    pub fn name(self) -> &'static str {
        match self {
            UpdateNeeded::Here => "here",
            UpdateNeeded::There => "there",
        }
    }

    fn from_name(name: &str) -> Option<UpdateNeeded> {
        match name {
            "here" => Some(UpdateNeeded::Here),
            "there" => Some(UpdateNeeded::There),
            _ => None,
        }
    }
}

const UPDATE_PREFIX: &str = "update:";
const UPDATE_SEEN: &str = " seen";

/// The source of the shared folder's update notice.
pub const FOLDER_SOURCE: &str = "folder";

/// The source of the notice that a local setting a later version of the
/// app wrote was kept as it is ([`crate::store::later_setting`]).
pub const SETTINGS_SOURCE: &str = "settings";

/// Syncs that need an update, kept so each is announced once and not at
/// every automatic try, as the Dart core's `UpdateNotices`: by the Device
/// synced with, or [`FOLDER_SOURCE`]. Local settings, never synced.
impl Store {
    /// Keeps that syncing with `source` needs an update `where_`, to be
    /// announced ([`Store::take_update_notices`]) unless known already.
    pub fn note_update_needed(&self, source: &str, where_: UpdateNeeded) -> Result<()> {
        let key = format!("{UPDATE_PREFIX}{source}");
        let held = self.local_setting(&key)?;
        let seen = format!("{}{UPDATE_SEEN}", where_.name());
        if held.as_deref() == Some(where_.name()) || held.as_deref() == Some(seen.as_str()) {
            return Ok(());
        }
        self.set_local_setting(&key, where_.name())
    }

    /// A sync with `source` went through: what it needed is forgotten.
    pub fn update_no_longer_needed(&self, source: &str) -> Result<()> {
        let key = format!("{UPDATE_PREFIX}{source}");
        if self.local_setting(&key)?.is_some() {
            self.remove_local_setting(&key)?;
        }
        Ok(())
    }

    /// Keeps whether the shared folder holds data of a newer format than
    /// this version reads.
    pub fn note_folder_newer(&self, newer: bool) -> Result<()> {
        if newer {
            self.note_update_needed(FOLDER_SOURCE, UpdateNeeded::Here)
        } else {
            self.update_no_longer_needed(FOLDER_SOURCE)
        }
    }

    /// The notices not announced yet, by source; taking them marks them
    /// announced.
    pub fn take_update_notices(&self) -> Result<Vec<(String, UpdateNeeded)>> {
        let mut out = Vec::new();
        for (key, value) in self.local_settings_with_prefix(UPDATE_PREFIX)? {
            let Some(where_) = UpdateNeeded::from_name(&value) else {
                continue;
            };
            out.push((key[UPDATE_PREFIX.len()..].to_string(), where_));
            self.set_local_setting(&key, &format!("{value}{UPDATE_SEEN}"))?;
        }
        Ok(out)
    }
}

/// The wire form of `entries`.
pub fn wire(entries: &[Entry]) -> Vec<WireEntry> {
    entries.iter().map(Entry::wire).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Notes;
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;

    fn device(seed: u8) -> Result<Store> {
        Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
    }

    #[test]
    fn signed_entries_pass_and_forged_ones_are_refused() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("Real", None, None)?;
        let mut entries = wire(&a.entries_since(&HashMap::new())?);
        let mut forged = entries[0].clone();
        forged.dseq = 99;
        forged.value = Some("forged".into());
        let mut stranger = entries[0].clone();
        stranger.device = "stranger".into();
        entries.push(forged);
        entries.push(stranger);
        let mut report = ImportReport::default();
        let applied = b.apply_entries(&entries, &[a.own_key_record()?], None, &mut report)?;
        assert_eq!(applied.len(), entries.len() - 2);
        assert_eq!(report.refused_count(), 2);
        assert_eq!(report.new_keys.len(), 1);
        let real = crate::Notes::new(&b).by_id(&note.id)?.map(|n| n.text);
        assert_eq!(real.as_deref(), Some("Real"));
        // Taken twice, nothing is new.
        let again = b.apply_entries(&entries, &[], None, &mut ImportReport::default())?;
        assert!(again.is_empty());
        // Without the key, nothing passes.
        let c = device(3)?;
        let mut report = ImportReport::default();
        assert!(
            c.apply_entries(&entries, &[], None, &mut report)?
                .is_empty()
        );
        assert_eq!(report.refused_count(), entries.len());
        Ok(())
    }

    #[test]
    fn keys_are_valid_only_for_their_device_and_trusted_as_met() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let c = device(3)?;
        let record = a.own_key_record()?;
        assert!(record.valid());
        let claimed = KeyRecord {
            public_key: c.signing_key()?.public_key().to_vec(),
            ..record.clone()
        };
        assert!(!claimed.valid());
        let mut report = ImportReport::default();
        b.learn_keys(
            &[record.clone(), c.own_key_record()?, claimed],
            Some(&a.device_id()?),
            &mut report,
        )?;
        let trust = |s: &Store, d: &str| -> Result<Option<KeyTrust>> {
            Ok(s.pinned_key(d)?.map(|k| k.trust))
        };
        assert_eq!(trust(&b, &a.device_id()?)?, Some(KeyTrust::Verified));
        assert_eq!(trust(&b, &c.device_id()?)?, Some(KeyTrust::Tofu));
        b.learn_keys(&[], Some(&c.device_id()?), &mut report)?;
        b.learn_keys(&[c.own_key_record()?], Some(&c.device_id()?), &mut report)?;
        assert_eq!(trust(&b, &c.device_id()?)?, Some(KeyTrust::Verified));
        assert_eq!(b.key_records()?.len(), 3);
        assert_eq!(KeyRecord::from_json(&record.to_json()), Some(record));
        Ok(())
    }

    #[test]
    fn blobs_follow_the_entries_checked_by_hash() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture_with("Photo", None, None, &[b"pic".to_vec()], &[])?;
        let hash = note.photos[0].clone();
        b.apply_entries(
            &wire(&a.entries_since(&HashMap::new())?),
            &[a.own_key_record()?],
            None,
            &mut ImportReport::default(),
        )?;
        assert_eq!(b.missing_blobs()?, vec![format!("photo/{hash}")]);
        assert!(b.knows_blob(&format!("photo/{hash}"))?);
        assert!(!b.knows_blob(&format!("audio/{hash}"))?);
        assert!(!b.put_blob("photo", &hash, b"other")?);
        assert!(b.put_blob("photo", &hash, b"pic")?);
        assert!(b.missing_blobs()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_moment_is_recorded_only_when_something_changed() -> Result<()> {
        let a = device(1)?;
        assert_eq!(a.moment_for(0, false, cause::SYNC, None)?, None);
        a.append("note:a", "text", Some("x"), None)?;
        let moment = a.moment_for(0, true, cause::SYNC, Some("Phone"))?;
        assert_eq!(moment.as_ref().map(|m| m.seq), Some(0));
        assert_eq!(a.moments()?.len(), 1);
        assert_eq!(a.entries_after(0)?.len(), 1);
        assert_eq!(a.current_seq()?, 1);
        Ok(())
    }
}
