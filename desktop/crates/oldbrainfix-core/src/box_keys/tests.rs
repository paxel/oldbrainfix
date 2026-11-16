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
