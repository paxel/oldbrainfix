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
