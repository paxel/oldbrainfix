//! Delta exchange between Devices, as the Dart core's `Sync` does it
//! (ADR 0002, 0003): every Device tells the other the newest entry it
//! holds from each Device, only what is missing travels, the photo and
//! voice recording bytes after it, and keys travel along so every entry
//! is checked against the key of the Device that signed it. Local
//! settings never travel (ADR 0008).

use std::collections::{BTreeSet, HashMap};

use crate::Result;
use crate::entry::{Entry, WireEntry};
use crate::notes::keys as note_keys;
use crate::signing::{ImportReport, KeyRecord, KeyTrust, PinnedKey, entry_bytes, verify_signature};
use crate::store::Store;

const KEY_PREFIX: &str = "key:";

/// The blob kinds and the field prefixes naming them.
const BLOB_KINDS: [(&str, &str); 2] = [
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
    /// `verified_device` as verified.
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
    /// counted, never recorded.
    pub fn apply_entries(
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
                let bytes = entry_bytes(
                    &e.device,
                    e.dseq,
                    &e.entity,
                    &e.field,
                    e.value.as_deref(),
                    &e.date,
                    &e.author,
                    &e.recorded,
                );
                let signed = key.is_some_and(|k| verify_signature(&k, &bytes, e.sig.as_deref()));
                if !signed {
                    report.refuse(&e.author, &e.device);
                    continue;
                }
                if self.insert(e)? {
                    applied.push(e.clone());
                }
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

    /// Whether the log ever referred to the blob `reference`.
    pub fn knows_blob(&self, reference: &str) -> Result<bool> {
        let (kind, hash) = split_blob_ref(reference);
        for (k, prefix) in BLOB_KINDS {
            if k == kind {
                return self.mentions(&format!("{prefix}{hash}"));
            }
        }
        Ok(false)
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
        assert_eq!(b.current(&note.id, "text")?.as_deref(), Some("Real"));
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
