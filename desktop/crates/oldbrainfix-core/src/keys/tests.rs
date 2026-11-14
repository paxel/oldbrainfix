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
