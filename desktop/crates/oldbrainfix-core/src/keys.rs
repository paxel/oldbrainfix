//! End-to-end encryption per Area, as the Dart core's `AreaKeys` keeps
//! it (ADR 0003, 0012): every Area has a random 256-bit key of its own,
//! and everything outside an Area goes under the Device group's personal
//! key. Keys travel only over an authenticated LAN session and sealed
//! into the shared folder under the personal key, never in plain text;
//! each is a local setting (ADR 0008). A scope may hold several keys:
//! every reader tries each, every writer seals with the lowest id, and
//! none is ever dropped.

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
        let field = match entity.split_once(':').map(|(kind, _)| kind) {
            Some("topic") => place_keys::AREA,
            Some("note") => note_keys::PLACE,
            Some("section") => note_keys::NOTE,
            Some("routine") => routine_keys::NOTE,
            Some("frog") => frog_keys::NOTE,
            Some(tally_keys::TYPE_ENTRY) => tally_keys::NOTE,
            Some(checklist_keys::TYPE_LINE) => checklist_keys::NOTE,
            _ => return Ok(PERSONAL_SCOPE.to_string()),
        };
        match self.logged(entity, field)? {
            Some(next) if rank(&next) < rank(entity) => self.scope_of(&next),
            _ => Ok(PERSONAL_SCOPE.to_string()),
        }
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
mod tests {
    use super::*;
    use crate::store::tests::store;
    use crate::{Notes, Places};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn counted(from: u8) -> [u8; 32] {
        std::array::from_fn(|i| from.wrapping_add(i as u8))
    }

    #[test]
    fn xchacha20_poly1305_gives_the_draft_standards_vector() -> TestResult {
        // draft-irtf-cfrg-xchacha-03, A.3.1.
        let key = counted(0x80);
        let nonce: [u8; 24] = std::array::from_fn(|i| 0x40 + i as u8);
        let plain = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let sealed = XChaCha20Poly1305::new((&key).into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plain,
                    aad: &hex::decode("50515253c0c1c2c3c4c5c6c7")?,
                },
            )
            .map_err(|e| e.to_string())?;
        assert_eq!(
            hex::encode(&sealed),
            "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb\
             731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452\
             2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9\
             21f9664c97637da9768812f615c68b13b52e\
             c0875924c1c7987947deafd8780acf49"
        );
        Ok(())
    }

    #[test]
    fn a_sealed_message_opens_only_with_its_key_and_use() -> TestResult {
        let key = counted(1);
        let sealed = seal(&key, b"Pills", sealed_as::ENTRY, &[7; 24])?;
        assert_eq!(
            unseal(&key, &sealed, sealed_as::ENTRY).as_deref(),
            Some(&b"Pills"[..])
        );
        assert_eq!(unseal(&counted(2), &sealed, sealed_as::ENTRY), None);
        assert_eq!(unseal(&key, &sealed, &sealed_as::blob("photo/x")), None);
        let mut changed = sealed.clone();
        changed[30] ^= 1;
        assert_eq!(unseal(&key, &changed, sealed_as::ENTRY), None);
        assert_eq!(unseal(&key, &sealed[..20], sealed_as::ENTRY), None);
        Ok(())
    }

    #[test]
    fn entities_are_sealed_with_the_key_of_their_area_now() -> TestResult {
        let s = store();
        let places = Places::new(&s);
        let house = places.add_area("Household")?;
        let kitchen = places.add_topic(&house.id, "Kitchen")?;
        let work = places.add_area("Work")?;
        let notes = Notes::new(&s);
        let loose = notes.capture("Loose", None, None)?;
        let pan = notes.capture("Pan", Some(&kitchen.id), None)?;
        s.ensure_area_keys()?;
        let scopes: Vec<String> = s.area_keys()?.into_iter().map(|k| k.scope).collect();
        assert_eq!(scopes.len(), 3);
        assert!(scopes.contains(&PERSONAL_SCOPE.to_string()));
        assert_eq!(s.scope_of(&kitchen.id)?, house.id);
        assert_eq!(s.scope_of(&pan.id)?, house.id);
        assert_eq!(s.scope_of(&loose.id)?, PERSONAL_SCOPE);
        assert_eq!(s.scope_of("namedplace:x")?, PERSONAL_SCOPE);
        notes.file(&pan.id, &work.id)?;
        assert_eq!(
            s.sealing_key(&pan.id)?.map(|k| k.scope),
            Some(work.id.clone())
        );
        s.append("note:stray", note_keys::PLACE, Some("area:unknown"), None)?;
        assert_eq!(
            s.sealing_key("note:stray")?.map(|k| k.scope).as_deref(),
            Some(PERSONAL_SCOPE)
        );
        s.ensure_area_keys()?;
        assert_eq!(s.area_keys()?.len(), 3);
        Ok(())
    }

    #[test]
    fn of_two_keys_for_a_scope_the_lower_id_writes() -> TestResult {
        let s = store();
        let a = AreaKey {
            scope: PERSONAL_SCOPE.into(),
            key: counted(1),
        };
        let b = AreaKey {
            scope: PERSONAL_SCOPE.into(),
            key: counted(2),
        };
        assert_eq!(s.learn_area_keys(&[a.clone(), b.clone(), a.clone()])?, 2);
        let low = if a.id() < b.id() { a } else { b };
        assert_eq!(
            s.writing_key(PERSONAL_SCOPE)?.map(|k| k.id()),
            Some(low.id())
        );
        assert_eq!(AreaKey::from_json(&low.to_json()), Some(low));
        assert_eq!(
            AreaKey::from_json(&json!({"scope": "topic:x", "key": BASE64.encode([0; 32])})),
            None
        );
        assert_eq!(
            AreaKey::from_json(&json!({"scope": "personal", "key": BASE64.encode([0; 16])})),
            None
        );
        Ok(())
    }
}
