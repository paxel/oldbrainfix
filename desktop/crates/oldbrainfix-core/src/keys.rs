//! End-to-end encryption per Area, as the Dart core's `AreaKeys` keeps
//! it (ADR 0003, 0012): every Area has a random 256-bit key of its own,
//! and everything outside an Area goes under the Device group's personal
//! key. Keys travel only over an authenticated LAN session and sealed
//! into the shared folder under the personal key, never in plain text;
//! each is a local setting (ADR 0008). A scope may hold several keys:
//! every reader tries each, every writer seals with the lowest id, and
//! none is ever dropped.

use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::checklists::keys as checklist_keys;
use crate::frogs::keys as frog_keys;
use crate::notes::keys as note_keys;
use crate::places::keys as place_keys;
use crate::routines::keys as routine_keys;
use crate::store::Store;
use crate::tallies::keys as tally_keys;
use crate::{Error, Result};

/// The scope of everything outside an Area.
pub const PERSONAL_SCOPE: &str = "personal";

pub const AREA_KEY_BYTES: usize = 32;

/// XChaCha20's nonce and Poly1305's tag.
pub const SEAL_NONCE_BYTES: usize = 24;
pub const SEAL_TAG_BYTES: usize = 16;

const PREFIX: &str = "areakey:";

/// What a sealed message is bound to, by use, as the Dart core's
/// `SealedAs`.
pub mod sealed_as {
    pub const ENTRY: &str = "oldbrainfix:entry";

    pub fn keys(device: &str) -> String {
        format!("oldbrainfix:keys:{device}")
    }

    /// The Profiles of a Shared Area's people a Device publishes, sealed
    /// with that Area's key (ADR 0017).
    pub fn profiles(device: &str) -> String {
        format!("oldbrainfix:profiles:{device}")
    }

    pub fn blob(reference: &str) -> String {
        format!("oldbrainfix:blob:{reference}")
    }
}

/// One key of a scope: an Area's entity id, or [`PERSONAL_SCOPE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaKey {
    pub scope: String,
    pub key: [u8; AREA_KEY_BYTES],
}

impl AreaKey {
    /// The key's id: the first 16 hex digits of its SHA-256.
    pub fn id(&self) -> String {
        area_key_id(&self.key)
    }

    pub fn to_json(&self) -> Value {
        json!({"key": BASE64.encode(self.key), "scope": self.scope})
    }

    /// The key in `json`, or none: a scope that is no Area and not the
    /// personal one, or a key that is not 32 bytes.
    pub fn from_json(json: &Value) -> Option<AreaKey> {
        let scope = json.get("scope")?.as_str()?;
        if scope != PERSONAL_SCOPE && !scope.starts_with("area:") {
            return None;
        }
        let bytes = BASE64.decode(json.get("key")?.as_str()?).ok()?;
        Some(AreaKey {
            scope: scope.to_string(),
            key: bytes.try_into().ok()?,
        })
    }
}

/// The id of `key`.
pub fn area_key_id(key: &[u8]) -> String {
    hex::encode(Sha256::digest(key))[..16].to_string()
}

/// Fresh random bytes for a key or a nonce.
pub fn random_bytes<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|e| Error::Invalid(format!("random: {e}")))?;
    Ok(bytes)
}

/// `plain` sealed under `key` with XChaCha20-Poly1305, bound to `aad`:
/// the nonce, the cipher text and the tag, as the Dart core's `seal`.
pub fn seal(
    key: &[u8; AREA_KEY_BYTES],
    plain: &[u8],
    aad: &str,
    nonce: &[u8; SEAL_NONCE_BYTES],
) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let sealed = cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plain,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|e| Error::Invalid(format!("seal: {e}")))?;
    let mut out = nonce.to_vec();
    out.extend(sealed);
    Ok(out)
}

/// What `sealed` holds when `key` sealed it bound to `aad`; none for
/// another key, another `aad`, or bytes that were changed.
pub fn unseal(key: &[u8; AREA_KEY_BYTES], sealed: &[u8], aad: &str) -> Option<Vec<u8>> {
    if sealed.len() < SEAL_NONCE_BYTES + SEAL_TAG_BYTES {
        return None;
    }
    let (nonce, rest) = sealed.split_at(SEAL_NONCE_BYTES);
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: rest,
                aad: aad.as_bytes(),
            },
        )
        .ok()
}

/// The name a blob `reference` goes by in the shared folder when sealed
/// with `key`: an HMAC-SHA256 of the reference, hex.
pub fn blob_name(key: &[u8], reference: &str) -> String {
    match <Hmac<Sha256> as Mac>::new_from_slice(key) {
        Ok(mut mac) => {
            mac.update(reference.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        // HMAC takes a key of any length.
        Err(_) => String::new(),
    }
}

/// The keys of `json`, skipping what is no key.
pub fn area_keys_of(json: Option<&Value>) -> Vec<AreaKey> {
    json.and_then(Value::as_array)
        .map(|list| list.iter().filter_map(AreaKey::from_json).collect())
        .unwrap_or_default()
}

/// How far up an entity id's kind is, for the walk to its Area.
/// The field naming what `entity` lies in, one kind further up, as
/// [`Store::scope_of`] follows it; none for a kind that lies in nothing.
fn up_field(entity: &str) -> Option<&'static str> {
    match entity.split_once(':').map(|(kind, _)| kind) {
        Some("topic") => Some(place_keys::AREA),
        Some("note") => Some(note_keys::PLACE),
        Some("section") => Some(note_keys::NOTE),
        Some("routine") => Some(routine_keys::NOTE),
        Some("frog") => Some(frog_keys::NOTE),
        Some(tally_keys::TYPE_ENTRY) => Some(tally_keys::NOTE),
        Some(checklist_keys::TYPE_LINE) => Some(checklist_keys::NOTE),
        _ => None,
    }
}

fn rank(id: &str) -> u8 {
    match id.split_once(':').map(|(kind, _)| kind) {
        Some("area") => 0,
        Some("topic") => 1,
        Some("note") => 2,
        Some("section")
        | Some("routine")
        | Some("frog")
        | Some(tally_keys::TYPE_ENTRY)
        | Some(checklist_keys::TYPE_LINE) => 3,
        _ => 9,
    }
}

impl Store {
    /// Every key this Device holds, by id.
    pub fn area_keys(&self) -> Result<Vec<AreaKey>> {
        Ok(self
            .local_settings_with_prefix(PREFIX)?
            .into_iter()
            .filter_map(|(_, raw)| serde_json::from_str(&raw).ok())
            .filter_map(|json: Value| AreaKey::from_json(&json))
            .collect())
    }

    /// The key with `id`, if held.
    pub fn area_key(&self, id: &str) -> Result<Option<AreaKey>> {
        Ok(self
            .local_setting(&format!("{PREFIX}{id}"))?
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|json| AreaKey::from_json(&json)))
    }

    /// The key a writer seals `scope` with: the lowest id held for it.
    pub fn writing_key(&self, scope: &str) -> Result<Option<AreaKey>> {
        Ok(self
            .area_keys()?
            .into_iter()
            .filter(|k| k.scope == scope)
            .min_by_key(AreaKey::id))
    }

    /// Keeps `keys` another Device handed over; a key already held stays.
    /// Returns how many were new.
    pub fn learn_area_keys(&self, keys: &[AreaKey]) -> Result<usize> {
        let mut learned = 0;
        for key in keys {
            if self.area_key(&key.id())?.is_some() {
                continue;
            }
            self.set_local_setting(&format!("{PREFIX}{}", key.id()), &key.to_json().to_string())?;
            learned += 1;
        }
        Ok(learned)
    }

    /// Makes the keys still missing: the personal one, and one for every
    /// Area that has none here.
    pub fn ensure_area_keys(&self) -> Result<()> {
        let held: Vec<String> = self.area_keys()?.into_iter().map(|k| k.scope).collect();
        let mut wanted = vec![PERSONAL_SCOPE.to_string()];
        let mut areas = self.entities_of_type(place_keys::TYPE_AREA)?;
        areas.sort();
        wanted.extend(areas);
        for scope in wanted {
            if held.contains(&scope) {
                continue;
            }
            self.learn_area_keys(&[AreaKey {
                scope,
                key: random_bytes()?,
            }])?;
        }
        Ok(())
    }

    /// The scope `entity` is sealed under, from where it is now, as the
    /// Dart core's `scopeOf`. Read from the log alone: a pending change
    /// (ADR 0022) moves nothing into an Area before it is accepted.
    pub fn scope_of(&self, entity: &str) -> Result<String> {
        if entity.starts_with("area:") {
            return Ok(entity.to_string());
        }
        let Some(field) = up_field(entity) else {
            return Ok(PERSONAL_SCOPE.to_string());
        };
        match self.logged(entity, field)? {
            Some(next) if rank(&next) < rank(entity) => self.scope_of(&next),
            _ => Ok(PERSONAL_SCOPE.to_string()),
        }
    }

    /// What [`Store::scope_of`] gives for each of `entities`, walked up a
    /// kind at a time for all of them, one read of the log for each step
    /// rather than one per entity (obf-1069), as the Dart core's
    /// `scopesOf`.
    pub fn scopes_of(&self, entities: &[String]) -> Result<HashMap<String, String>> {
        // The entity one kind further up that each lies in, none for none.
        let mut up: HashMap<String, Option<String>> = HashMap::new();
        let mut step: Vec<String> = entities.to_vec();
        step.sort();
        step.dedup();
        while !step.is_empty() {
            let read = self.logged_histories_of(
                step.iter()
                    .filter(|e| !e.starts_with("area:") && up_field(e).is_some())
                    .map(String::as_str),
            )?;
            let mut next: Vec<String> = Vec::new();
            for e in &step {
                let field = if e.starts_with("area:") {
                    None
                } else {
                    up_field(e)
                };
                let value = match (field, read.get(e)) {
                    (Some(field), Some(history)) => Store::latest_fields(history.iter().cloned())
                        .get(field)
                        .map(str::to_string),
                    _ => None,
                };
                let above = value.filter(|v| rank(v) < rank(e));
                if let Some(above) = &above {
                    next.push(above.clone());
                }
                up.insert(e.clone(), above);
            }
            next.retain(|e| !up.contains_key(e));
            next.sort();
            next.dedup();
            step = next;
        }
        fn scope(up: &HashMap<String, Option<String>>, e: &str) -> String {
            if e.starts_with("area:") {
                return e.to_string();
            }
            match up.get(e) {
                Some(Some(above)) => scope(up, above),
                _ => PERSONAL_SCOPE.to_string(),
            }
        }
        Ok(entities
            .iter()
            .map(|e| (e.clone(), scope(&up, e)))
            .collect())
    }

    /// The key `entity` is sealed with: its scope's writing key, or the
    /// personal one when its Area has no key here.
    pub fn sealing_key(&self, entity: &str) -> Result<Option<AreaKey>> {
        match self.writing_key(&self.scope_of(entity)?)? {
            Some(key) => Ok(Some(key)),
            None => self.writing_key(PERSONAL_SCOPE),
        }
    }
}

#[cfg(test)]
mod tests;
