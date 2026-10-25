//! The keys a Sent Item is encrypted to (ADR 0015), as the Dart core's
//! `box_keys.dart`: every Device has an X25519 key of its own beside its
//! Ed25519 signing key, derived from the signing key's seed so there is
//! nothing more to keep, and published in its key record signed by the
//! signing key.

use curve25519_dalek::montgomery::MontgomeryPoint;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const BOX_KEY_BYTES: usize = 32;

fn hmac(key: &[u8], parts: &[&[u8]]) -> Vec<u8> {
    // HMAC takes a key of any length, so this never fails.
    let mut mac = match <Hmac<Sha256> as Mac>::new_from_slice(key) {
        Ok(mac) => mac,
        Err(_) => return Vec::new(),
    };
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().to_vec()
}

/// HKDF with HMAC-SHA256 (RFC 5869): `length` bytes from `ikm`, bound to
/// `salt` and `info`.
pub fn hkdf_sha256(ikm: &[u8], salt: &[u8], info: &[u8], length: usize) -> Vec<u8> {
    let zeros = [0u8; 32];
    let prk = hmac(if salt.is_empty() { &zeros } else { salt }, &[ikm]);
    let mut out = Vec::with_capacity(length + 32);
    let mut block = Vec::new();
    let mut i: u8 = 1;
    while out.len() < length {
        block = hmac(&prk, &[&block, info, &[i]]);
        if block.is_empty() {
            break;
        }
        out.extend_from_slice(&block);
        i = i.wrapping_add(1);
    }
    out.resize(length, 0);
    out
}

/// The 32 bytes of HKDF-SHA256 over `ikm`.
fn hkdf32(ikm: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&hkdf_sha256(ikm, salt, info, 32));
    out
}

/// The X25519 secret of the Device whose signing key has `seed`.
pub fn box_secret_of(seed: &[u8; 32]) -> [u8; BOX_KEY_BYTES] {
    hkdf32(seed, &[], b"oldbrainfix:box")
}

/// X25519 of `secret` and `public` (RFC 7748); none when the result is
/// all zeros, as for a public key of low order.
pub fn x25519(secret: &[u8; BOX_KEY_BYTES], public: &[u8; BOX_KEY_BYTES]) -> Option<[u8; 32]> {
    let shared = MontgomeryPoint(*public).mul_clamped(*secret).to_bytes();
    (shared != [0u8; 32]).then_some(shared)
}

/// The public X25519 key of `secret`.
pub fn box_public_of(secret: &[u8; BOX_KEY_BYTES]) -> [u8; BOX_KEY_BYTES] {
    MontgomeryPoint::mul_base_clamped(*secret).to_bytes()
}

/// The key a Sent Item's content key is sealed with for one recipient:
/// HKDF over the X25519 secret, bound to both public keys.
pub fn wrap_key(shared: &[u8; 32], ephemeral: &[u8; 32], to: &[u8; 32]) -> [u8; 32] {
    let mut salt = ephemeral.to_vec();
    salt.extend_from_slice(to);
    hkdf32(shared, &salt, crate::sent::SENT_KEY_AAD.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes32(text: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(&hex::decode(text).unwrap_or_default());
        out
    }

    #[test]
    fn hkdf_gives_rfc_5869_test_case_1() {
        let okm = hkdf_sha256(
            &[0x0b; 22],
            &hex::decode("000102030405060708090a0b0c").unwrap_or_default(),
            &hex::decode("f0f1f2f3f4f5f6f7f8f9").unwrap_or_default(),
            42,
        );
        assert_eq!(
            hex::encode(okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    #[test]
    fn x25519_gives_the_rfc_7748_shared_secret() {
        let alice = bytes32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        let bob = bytes32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
        assert_eq!(
            hex::encode(box_public_of(&alice)),
            "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
        );
        assert_eq!(
            x25519(&alice, &box_public_of(&bob)).map(hex::encode),
            Some("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742".into())
        );
        assert_eq!(x25519(&alice, &[0; 32]), None);
    }
}
