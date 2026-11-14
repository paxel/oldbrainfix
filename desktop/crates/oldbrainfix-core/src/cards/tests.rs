use super::*;
use crate::sent::{OpenError, SenderTrust};
use crate::store::tests::ticking;
use crate::{Notes, SigningKey};
use serde_json::json;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn device(seed: u8, name: &str) -> std::result::Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    OwnProfile::new(&store).rename(name)?;
    Ok(store)
}

fn id_of(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

fn card(store: &Store, back: bool) -> std::result::Result<String, Box<dyn std::error::Error>> {
    Ok(store.profile_card(back)?.ok_or("a card")?)
}

fn payload_of(link: &str) -> std::result::Result<Value, Box<dyn std::error::Error>> {
    let (payload, _) = card_parts(link).ok_or("a card")?;
    Ok(serde_json::from_slice(&BASE64_URL.decode(payload)?)?)
}

/// A card made by hand: `payload` signed by `key`.
fn card_of(key: &SigningKey, payload: &Value) -> String {
    let text = payload.to_string();
    let sig = BASE64
        .decode(key.sign(&card_bytes(&text)))
        .unwrap_or_default();
    format!(
        "{LINK}{}.{}",
        BASE64_URL.encode(text.as_bytes()),
        BASE64_URL.encode(sig)
    )
}

/// A Profile made by hand with `key` as its first key.
fn forged(key: &SigningKey, record: &Value) -> Value {
    let text = record.to_string();
    SignedProfile {
        sig: key.sign(&record_bytes(&text)),
        record: text,
        signer: BASE64.encode(key.public_key()),
        first: BASE64.encode(key.public_key()),
        keys: Vec::new(),
        aliases: Vec::new(),
        fresh: false,
    }
    .to_json()
}

#[test]
fn a_link_adds_the_person_not_verified_and_the_answer_offers_none() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let opened = ben.open_profile_card(&card(&ann, false)?, false)?;
    assert_eq!(opened.taken, CardTaken::Added);
    assert_eq!(opened.id, Some(id_of(&ann)?));
    assert!(opened.offer_back);
    assert_eq!(ben.person_shown(&id_of(&ann)?)?.name, "Ann");
    assert_eq!(
        ben.person_code(&id_of(&ann)?)?,
        OwnProfile::new(&ann).code()?
    );
    assert!(!ben.person_verified(&id_of(&ann)?)?);
    let back = ann.open_profile_card(&card(&ben, true)?, false)?;
    assert_eq!(back.taken, CardTaken::Added);
    assert!(!back.offer_back);
    let again = ben.open_profile_card(&card(&ann, false)?, true)?;
    assert_eq!(again.taken, CardTaken::Unchanged);
    assert!(ben.person_verified(&id_of(&ann)?)?);
    Ok(())
}

#[test]
fn the_picture_stays_out_and_comes_with_their_profile_at_the_same_version() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    // A Profile picture the Dart core made.
    let picture = include_bytes!("../../../../fixtures/shared/override.jpg");
    OwnProfile::new(&ann).set_picture(Some(picture.to_vec()))?;
    let link = card(&ann, false)?;
    assert!(!payload_of(&link)?.to_string().contains("picture"));
    ben.open_profile_card(&link, false)?;
    assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_none());
    let full = ann.own_profile_for_others()?.ok_or("her Profile")?;
    assert_eq!(ben.take_person(&full, None)?, PersonTaken::Updated);
    assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_some());
    assert_eq!(
        ben.open_profile_card(&link, false)?.taken,
        CardTaken::Unchanged
    );
    assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_some());
    Ok(())
}

#[test]
fn a_card_changed_after_it_was_signed_is_unreadable() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let link = card(&ann, false)?;
    let (_, sig) = card_parts(&link).ok_or("a card")?;
    let mut flagged = payload_of(&link)?;
    flagged
        .as_object_mut()
        .ok_or("an object")?
        .insert("back".into(), Value::Bool(true));
    let changed = format!(
        "{LINK}{}.{sig}",
        BASE64_URL.encode(flagged.to_string().as_bytes())
    );
    assert_eq!(
        ben.open_profile_card(&changed, false)?,
        CardOpened::only(CardTaken::Unreadable, None)
    );
    assert_eq!(
        ben.open_profile_card("hello", false)?.taken,
        CardTaken::Unreadable
    );
    assert!(ben.kept_people()?.is_empty());
    Ok(())
}

#[test]
fn a_card_signed_by_a_key_its_profile_does_not_bind_is_unreadable() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let eve = SigningKey::from_seed([3; 32]);
    let mut payload = payload_of(&card(&ann, false)?)?;
    payload
        .as_object_mut()
        .ok_or("an object")?
        .insert("key".into(), KeyRecord::make(&eve).to_json());
    assert_eq!(
        ben.open_profile_card(&card_of(&eve, &payload), false)?
            .taken,
        CardTaken::Unreadable
    );
    assert!(ben.kept_people()?.is_empty());
    Ok(())
}

#[test]
fn a_copied_id_is_somebody_else_and_verifies_nobody() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    ben.open_profile_card(&card(&ann, false)?, false)?;
    let eve = SigningKey::from_seed([3; 32]);
    let copied = card_of(
        &eve,
        &json!({
            "format": 1,
            "key": KeyRecord::make(&eve).to_json(),
            "profile": forged(&eve, &json!({"id": id_of(&ann)?, "name": "Ann", "version": 9})),
        }),
    );
    assert_eq!(
        ben.open_profile_card(&copied, true)?.taken,
        CardTaken::Refused
    );
    assert!(!ben.person_verified(&id_of(&ann)?)?);
    Ok(())
}

#[test]
fn a_blocked_persons_card_is_refused() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    ben.open_profile_card(&card(&ann, false)?, false)?;
    ben.block_person(&id_of(&ann)?, false)?;
    assert_eq!(
        ben.open_profile_card(&card(&ann, false)?, false)?.taken,
        CardTaken::Blocked
    );
    assert!(!ben.person_verified(&id_of(&ann)?)?);
    Ok(())
}

fn file(store: &Store, back: bool) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(store.profile_card_file(back)?.ok_or("a card file")?)
}

/// The JSON of the card file `bytes`.
fn file_json(bytes: &[u8]) -> std::result::Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(bytes)?)
}

#[test]
fn a_card_file_adds_the_person_with_the_picture() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let picture = include_bytes!("../../../../fixtures/shared/override.jpg");
    OwnProfile::new(&ann).set_picture(Some(picture.to_vec()))?;
    let bytes = file(&ann, false)?;
    let json = file_json(&bytes)?;
    assert_eq!(
        json.as_object()
            .ok_or("an object")?
            .keys()
            .collect::<Vec<_>>(),
        ["card", "type"]
    );
    assert_eq!(json["type"], "oldbrainfix-card");
    assert!(looks_like_profile_card(&bytes));
    let opened = ben.open_profile_card_file(&bytes)?;
    assert_eq!(opened.taken, CardTaken::Added);
    assert_eq!(opened.id, Some(id_of(&ann)?));
    assert!(opened.offer_back);
    assert!(!ben.person_verified(&id_of(&ann)?)?);
    assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_some());
    // The QR's card after it changes nothing; the full Profile neither.
    assert_eq!(
        ben.open_profile_card(&card(&ann, false)?, false)?.taken,
        CardTaken::Unchanged
    );
    let full = ann.own_profile_for_others()?.ok_or("her Profile")?;
    assert_eq!(ben.take_person(&full, None)?, PersonTaken::Unchanged);
    // Opened after the QR's card, the file brings the picture.
    let cara = device(3, "Cara")?;
    cara.open_profile_card(&card(&ann, false)?, false)?;
    assert!(cara.person_shown(&id_of(&ann)?)?.picture.is_none());
    assert_eq!(
        cara.open_profile_card_file(&bytes)?.taken,
        CardTaken::Updated
    );
    assert!(cara.person_shown(&id_of(&ann)?)?.picture.is_some());
    // The answer offers none; one's own and a blocked person's keep
    // nothing.
    let back = ann.open_profile_card_file(&file(&ben, true)?)?;
    assert_eq!(back.taken, CardTaken::Added);
    assert!(!back.offer_back);
    assert_eq!(ann.open_profile_card_file(&bytes)?.taken, CardTaken::Own);
    ben.block_person(&id_of(&ann)?, false)?;
    assert_eq!(
        ben.open_profile_card_file(&bytes)?.taken,
        CardTaken::Blocked
    );
    Ok(())
}

#[test]
fn a_file_that_is_no_card_file_or_holds_a_changed_card_is_unreadable() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let bytes = file(&ann, false)?;
    let json = file_json(&bytes)?;
    let card_text = json["card"].as_str().ok_or("a card")?;
    let (payload, sig) = card_parts(card_text).ok_or("parts")?;
    let mut flagged: Value = serde_json::from_slice(&BASE64_URL.decode(&payload)?)?;
    flagged
        .as_object_mut()
        .ok_or("an object")?
        .insert("back".into(), Value::Bool(true));
    let changed = format!(
        "{}.{sig}",
        BASE64_URL.encode(flagged.to_string().as_bytes())
    );
    let mut padded = bytes.clone();
    padded.resize(MAX_FILE_BYTES + 1, b' ');
    let mut fits = bytes.clone();
    fits.resize(MAX_FILE_BYTES, b' ');
    let unreadable = [
        b"hello".to_vec(),
        Vec::new(),
        json!({"card": card_text}).to_string().into_bytes(),
        json!({"card": card_text, "type": "oldbrainfix-backup"})
            .to_string()
            .into_bytes(),
        json!({"card": changed, "type": "oldbrainfix-card"})
            .to_string()
            .into_bytes(),
        json!({"card": format!("{LINK}{card_text}"), "type": "oldbrainfix-card"})
            .to_string()
            .into_bytes(),
        json!({"card": 1, "type": "oldbrainfix-card"})
            .to_string()
            .into_bytes(),
        padded,
    ];
    for wrong in &unreadable {
        assert_eq!(
            ben.open_profile_card_file(wrong)?,
            CardOpened::only(CardTaken::Unreadable, None)
        );
    }
    assert!(!looks_like_profile_card(&unreadable[0]));
    assert!(!looks_like_profile_card(&unreadable[2]));
    assert!(!looks_like_profile_card(&unreadable[7]));
    assert!(ben.kept_people()?.is_empty());
    assert_eq!(ben.open_profile_card_file(&fits)?.taken, CardTaken::Added);
    Ok(())
}

#[test]
fn a_card_file_of_a_later_format_needs_an_update() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let json = file_json(&file(&ann, false)?)?;
    for format in [json!(2), json!("1"), json!(1.0), Value::Null, json!(0)] {
        let mut later = json.clone();
        later
            .as_object_mut()
            .ok_or("an object")?
            .insert("format".into(), format.clone());
        let bytes = later.to_string().into_bytes();
        assert!(looks_like_profile_card(&bytes));
        assert_eq!(
            ben.open_profile_card_file(&bytes)?,
            CardOpened::only(CardTaken::NeedsUpdate, None),
            "{format}"
        );
    }
    assert!(ben.kept_people()?.is_empty());
    let mut one = json.clone();
    one.as_object_mut()
        .ok_or("an object")?
        .insert("format".into(), json!(1));
    assert_eq!(
        ben.open_profile_card_file(one.to_string().as_bytes())?
            .taken,
        CardTaken::Added
    );
    Ok(())
}

#[test]
fn a_card_file_is_named_for_the_person() {
    for (name, file) in [
        ("Ann", "Ann.obfcard"),
        ("Ann Berg", "Ann Berg.obfcard"),
        ("a/b\\c:d*e?f\"g<h>i|j", "a_b_c_d_e_f_g_h_i_j.obfcard"),
        ("  ..Ann..  ", "Ann.obfcard"),
        ("...", "Profile.obfcard"),
        ("", "Profile.obfcard"),
        ("\u{200B}", "Profile.obfcard"),
        ("con", "_con.obfcard"),
        ("COM1", "_COM1.obfcard"),
        ("Nul .x", "_Nul .x.obfcard"),
        ("COM0", "COM0.obfcard"),
        ("Connie", "Connie.obfcard"),
        ("Zoë 😀", "Zoë 😀.obfcard"),
    ] {
        assert_eq!(file_name(name), file, "{name:?}");
    }
}

#[test]
fn a_person_added_from_a_card_is_sent_to_sealed_and_theirs_preview_as_known() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    assert!(ben.sent_item_recipients()?.is_empty());
    ben.open_profile_card(&card(&ann, false)?, false)?;
    let recipients = ben.sent_item_recipients()?;
    assert_eq!(recipients.len(), 1);
    assert_eq!(recipients[0].name, "Ann");
    assert!(ben.paired_devices()?.is_empty());
    let note = Notes::new(&ben).capture("For Ann", None, None)?;
    let file = ben.send_note(&note.id, Some(&recipients[0]), false)?;
    assert!(
        ann.open_sent_item(&file)?
            .map(|o| o.encrypted)
            .unwrap_or(false)
    );
    let mine = Notes::new(&ann).capture("For Ben", None, None)?;
    let from_ann = ann.send_note(&mine.id, None, false)?;
    let opened = ben
        .open_sent_item(&from_ann)?
        .map_err(|e: OpenError| format!("{e:?}"))?;
    assert_eq!(opened.trust, SenderTrust::Known);
    ben.block_person(&id_of(&ann)?, false)?;
    assert!(ben.sent_item_recipients()?.is_empty());
    Ok(())
}
