//! Signed entries (ADR 0002): every Device signs what it writes with an
//! Ed25519 key made once per install and bound to its device id. The
//! signed bytes are the Dart core's `entryBytes`, byte for byte.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signature, Signer, SigningKey as DalekKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::box_keys::{BOX_KEY_BYTES, box_public_of, box_secret_of};
use crate::{Error, Result};

/// A Device's own key.
pub struct SigningKey {
    seed: [u8; 32],
    key: DalekKey,
}

impl SigningKey {
    pub fn from_seed(seed: [u8; 32]) -> SigningKey {
        SigningKey {
            seed,
            key: DalekKey::from_bytes(&seed),
        }
    }

    /// The key whose seed is `hex`, as the store keeps it.
    pub fn from_seed_hex(hex: &str) -> Result<SigningKey> {
        let bytes = hex::decode(hex).map_err(|e| Error::Invalid(format!("key seed: {e}")))?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| Error::Invalid("key seed: not 32 bytes".into()))?;
        Ok(Self::from_seed(seed))
    }

    pub fn generate() -> Result<SigningKey> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed)
            .map_err(|e| Error::Invalid(format!("no randomness for a key: {e}")))?;
        Ok(Self::from_seed(seed))
    }

    pub fn seed_hex(&self) -> String {
        hex::encode(self.seed)
    }

    /// The seed, which the X25519 key is derived from.
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.key.verifying_key().to_bytes()
    }

    /// The key's fingerprint: SHA-256 of the public key, hex.
    pub fn fingerprint(&self) -> String {
        key_fingerprint(&self.public_key())
    }

    /// What the UI shows next to a name: `7f3a-c21e`.
    pub fn code(&self) -> String {
        key_code(&self.public_key())
    }

    /// The signature over `message`, base64.
    pub fn sign(&self, message: &[u8]) -> String {
        BASE64.encode(self.key.sign(message).to_bytes())
    }
}

pub fn key_fingerprint(public_key: &[u8]) -> String {
    hex::encode(Sha256::digest(public_key))
}

/// The short form of a fingerprint, for people: eight hex digits with a
/// dash.
pub fn key_code(public_key: &[u8]) -> String {
    let f = key_fingerprint(public_key);
    format!("{}-{}", &f[..4], &f[4..8])
}

/// A device id derived from its key: the first 16 bytes of the
/// fingerprint as hex.
pub fn device_id_from_key(public_key: &[u8]) -> String {
    key_fingerprint(public_key)[..32].to_string()
}

/// Whether `signature` (base64) signs `message` under `public_key`. Any
/// malformed input is simply invalid.
pub fn verify_signature(public_key: &[u8], message: &[u8], signature: Option<&str>) -> bool {
    let Some(signature) = signature else {
        return false;
    };
    let Ok(key) = <[u8; 32]>::try_from(public_key) else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(&key) else {
        return false;
    };
    let Ok(sig) = BASE64.decode(signature) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(&sig) else {
        return false;
    };
    key.verify(message, &sig).is_ok()
}

/// The bytes an entry's signature covers: a fixed-order JSON list, so a
/// null value and an empty one differ. Dates go in as the store writes
/// them. The causal context goes last, and only when there is one:
/// entries without it keep the bytes they were signed with.
#[allow(clippy::too_many_arguments)]
pub fn entry_bytes(
    device: &str,
    dseq: i64,
    entity: &str,
    field: &str,
    value: Option<&str>,
    date_iso: &str,
    author: &str,
    recorded_iso: &str,
    ctx: Option<&str>,
) -> Vec<u8> {
    entry_bytes_with(
        device,
        dseq,
        entity,
        field,
        value,
        date_iso,
        author,
        recorded_iso,
        ctx,
        &serde_json::Map::new(),
    )
}

/// [`entry_bytes`] of an entry carrying keys a later version added
/// (ADR 0019), as the Dart core's `entryBytes` with `extra`: they follow
/// as one object, keys sorted, after the context or a null in its place.
#[allow(clippy::too_many_arguments)]
pub fn entry_bytes_with(
    device: &str,
    dseq: i64,
    entity: &str,
    field: &str,
    value: Option<&str>,
    date_iso: &str,
    author: &str,
    recorded_iso: &str,
    ctx: Option<&str>,
    extra: &serde_json::Map<String, serde_json::Value>,
) -> Vec<u8> {
    let mut list = vec![
        serde_json::json!(device),
        serde_json::json!(dseq),
        serde_json::json!(entity),
        serde_json::json!(field),
        serde_json::json!(value),
        serde_json::json!(date_iso),
        serde_json::json!(author),
        serde_json::json!(recorded_iso),
    ];
    if !extra.is_empty() {
        list.push(serde_json::json!(ctx));
        list.push(serde_json::Value::Object(extra.clone()));
    } else if let Some(ctx) = ctx {
        list.push(serde_json::json!(ctx));
    }
    serde_json::Value::Array(list).to_string().into_bytes()
}

/// How far a pinned key is trusted: met in a payload some other Device
/// carried, or met in person over a session the pair code authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTrust {
    Tofu,
    Verified,
}

impl KeyTrust {
    pub fn name(self) -> &'static str {
        match self {
            KeyTrust::Tofu => "tofu",
            KeyTrust::Verified => "verified",
        }
    }
}

/// A Device's public key as it travels with the entries, as the Dart
/// core's `KeyRecord`: it signs itself, and the device id is the key's
/// own fingerprint. It carries the Device's X25519 key too, which Sent
/// Items are encrypted to (ADR 0015), signed by the same key on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRecord {
    pub device: String,
    pub public_key: Vec<u8>,
    pub signature: String,
    /// The X25519 key and the signature over it; none in a record made
    /// without one.
    pub box_key: Option<Vec<u8>>,
    pub box_signature: Option<String>,
}

impl KeyRecord {
    fn bytes(device: &str, public_key: &[u8]) -> Vec<u8> {
        serde_json::json!([device, BASE64.encode(public_key)])
            .to_string()
            .into_bytes()
    }

    fn box_bytes(device: &str, box_key: &[u8]) -> Vec<u8> {
        serde_json::json!(["box", device, BASE64.encode(box_key)])
            .to_string()
            .into_bytes()
    }

    /// The record of the Device whose key is `key`.
    pub fn make(key: &SigningKey) -> KeyRecord {
        let public_key = key.public_key();
        let device = device_id_from_key(&public_key);
        let signature = key.sign(&Self::bytes(&device, &public_key));
        let box_key = box_public_of(&box_secret_of(key.seed()));
        let box_signature = key.sign(&Self::box_bytes(&device, &box_key));
        KeyRecord {
            device,
            public_key: public_key.to_vec(),
            signature,
            box_key: Some(box_key.to_vec()),
            box_signature: Some(box_signature),
        }
    }

    /// A record without an X25519 key, as a build before them made it.
    pub fn bare(&self) -> KeyRecord {
        KeyRecord {
            box_key: None,
            box_signature: None,
            ..self.clone()
        }
    }

    /// True when the record names the device its key belongs to and was
    /// signed by that key.
    pub fn valid(&self) -> bool {
        self.public_key.len() == 32
            && device_id_from_key(&self.public_key) == self.device
            && verify_signature(
                &self.public_key,
                &Self::bytes(&self.device, &self.public_key),
                Some(&self.signature),
            )
    }

    /// The X25519 key to encrypt to this Device, when the record is
    /// valid and its key signed it.
    pub fn verified_box_key(&self) -> Option<[u8; BOX_KEY_BYTES]> {
        let key: [u8; BOX_KEY_BYTES] = self.box_key.as_deref()?.try_into().ok()?;
        (self.valid()
            && verify_signature(
                &self.public_key,
                &Self::box_bytes(&self.device, &key),
                self.box_signature.as_deref(),
            ))
        .then_some(key)
    }

    /// What the UI shows for the key.
    pub fn code(&self) -> String {
        key_code(&self.public_key)
    }

    /// The record as it travels, keys sorted as the Dart core writes them.
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        if let (Some(key), Some(sig)) = (&self.box_key, &self.box_signature) {
            map.insert("box".into(), BASE64.encode(key).into());
            map.insert("boxSig".into(), sig.clone().into());
        }
        map.insert("device".into(), self.device.clone().into());
        map.insert("key".into(), BASE64.encode(&self.public_key).into());
        map.insert("sig".into(), self.signature.clone().into());
        serde_json::Value::Object(map)
    }

    /// The record in `json`, or none when it is not one. A box key that
    /// is not base64 is left out.
    pub fn from_json(json: &serde_json::Value) -> Option<KeyRecord> {
        let box_key = json
            .get("box")
            .and_then(|b| b.as_str())
            .and_then(|b| BASE64.decode(b).ok());
        let box_signature = json
            .get("boxSig")
            .and_then(|s| s.as_str())
            .map(str::to_string);
        let (box_key, box_signature) = match (box_key, box_signature) {
            (Some(key), Some(sig)) => (Some(key), Some(sig)),
            _ => (None, None),
        };
        Some(KeyRecord {
            device: json.get("device")?.as_str()?.to_string(),
            public_key: BASE64.decode(json.get("key")?.as_str()?).ok()?,
            signature: json.get("sig")?.as_str()?.to_string(),
            box_key,
            box_signature,
        })
    }
}

/// A key this Device holds for another Device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedKey {
    pub record: KeyRecord,
    pub trust: KeyTrust,
}

impl PinnedKey {
    pub fn to_json(&self) -> serde_json::Value {
        let mut json = self.record.to_json();
        if let Some(map) = json.as_object_mut() {
            map.insert("trust".into(), self.trust.name().into());
        }
        json
    }

    pub fn from_json(json: &serde_json::Value) -> Option<PinnedKey> {
        let trust = match json.get("trust").and_then(|t| t.as_str()) {
            Some("verified") => KeyTrust::Verified,
            _ => KeyTrust::Tofu,
        };
        Some(PinnedKey {
            record: KeyRecord::from_json(json)?,
            trust,
        })
    }
}

/// What an import found out: the entries it refused for a missing or
/// wrong signature, by (author, device), and the keys it met first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub refused: std::collections::BTreeMap<(String, String), usize>,
    pub new_keys: Vec<PinnedKey>,
}

impl ImportReport {
    pub fn refused_count(&self) -> usize {
        self.refused.values().sum()
    }

    pub fn refuse(&mut self, author: &str, device: &str) {
        *self
            .refused
            .entry((author.to_string(), device.to_string()))
            .or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_verifies_only_for_its_bytes_and_key() {
        let key = SigningKey::from_seed([7; 32]);
        let other = SigningKey::from_seed([8; 32]);
        let bytes = entry_bytes("d", 1, "note:a", "text", Some("hi"), "x", "", "y", None);
        let sig = key.sign(&bytes);
        assert!(verify_signature(&key.public_key(), &bytes, Some(&sig)));
        assert!(!verify_signature(&other.public_key(), &bytes, Some(&sig)));
        assert!(!verify_signature(&key.public_key(), b"other", Some(&sig)));
        assert!(!verify_signature(&key.public_key(), &bytes, None));
        assert!(!verify_signature(&key.public_key(), &bytes, Some("!!")));
        assert!(!verify_signature(&[1, 2], &bytes, Some(&sig)));
        assert_eq!(key.code().len(), 9);
        assert_eq!(
            &key.fingerprint()[..32],
            device_id_from_key(&key.public_key())
        );
        let again = SigningKey::from_seed_hex(&key.seed_hex());
        assert_eq!(again.map(|k| k.public_key()).ok(), Some(key.public_key()));
        assert!(SigningKey::from_seed_hex("abcd").is_err());
        assert!(SigningKey::from_seed_hex("zz").is_err());
    }

    #[test]
    fn a_null_value_and_an_empty_one_sign_differently() {
        let null = entry_bytes("d", 1, "e", "f", None, "x", "", "y", None);
        let empty = entry_bytes("d", 1, "e", "f", Some(""), "x", "", "y", None);
        assert_eq!(
            String::from_utf8_lossy(&null),
            r#"["d",1,"e","f",null,"x","","y"]"#
        );
        assert_ne!(null, empty);
    }
}
