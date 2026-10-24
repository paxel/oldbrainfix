//! Signed entries (ADR 0002): every Device signs what it writes with an
//! Ed25519 key made once per install and bound to its device id. The
//! signed bytes are the Dart core's `entryBytes`, byte for byte.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signature, Signer, SigningKey as DalekKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

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
/// them.
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
) -> Vec<u8> {
    serde_json::json!([
        device,
        dseq,
        entity,
        field,
        value,
        date_iso,
        author,
        recorded_iso
    ])
    .to_string()
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_verifies_only_for_its_bytes_and_key() {
        let key = SigningKey::from_seed([7; 32]);
        let other = SigningKey::from_seed([8; 32]);
        let bytes = entry_bytes("d", 1, "note:a", "text", Some("hi"), "x", "", "y");
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
        let null = entry_bytes("d", 1, "e", "f", None, "x", "", "y");
        let empty = entry_bytes("d", 1, "e", "f", Some(""), "x", "", "y");
        assert_eq!(
            String::from_utf8_lossy(&null),
            r#"["d",1,"e","f",null,"x","","y"]"#
        );
        assert_ne!(null, empty);
    }
}
