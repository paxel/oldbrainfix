use super::*;

thread_local! {
    /// How many records [`Profile::parse`] read on this thread: a
    /// received record is not read before its signature holds.
    pub(super) static PARSED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// A `width` × `height` picture, red on the left half, blue on the
/// right.
fn halves(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let image = RgbImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut out, format)
        .expect("encodes");
    out.into_inner()
}

/// The marker bytes of `jpeg`'s segments before the scan.
fn markers(jpeg: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 2;
    while i + 4 <= jpeg.len() && jpeg[i] == 0xFF {
        let marker = jpeg[i + 1];
        if marker == 0xDA {
            break;
        }
        out.push(marker);
        i += 2 + ((usize::from(jpeg[i + 2]) << 8) | usize::from(jpeg[i + 3]));
    }
    out
}

fn assert_profile_picture(picture: Option<&[u8]>) {
    let picture = picture.expect("a picture");
    assert_eq!(&picture[..2], &[0xFF, 0xD8]);
    let decoded = image::load_from_memory(picture).expect("decodes");
    assert_eq!((decoded.width(), decoded.height()), (256, 256));
    assert!(
        markers(picture)
            .iter()
            .all(|m| !(0xE1..=0xEF).contains(m) && *m != 0xFE)
    );
}

#[test]
fn names_keep_every_script_and_emoji() {
    assert_eq!(sanitize_name("  Ada  "), "Ada");
    assert_eq!(sanitize_name("李小龍"), "李小龍");
    assert_eq!(
        sanitize_name("Mum \u{1F469}\u{200D}\u{1F467} \u{2764}\u{FE0F}"),
        "Mum \u{1F469}\u{200D}\u{1F467} \u{2764}\u{FE0F}"
    );
}

#[test]
fn names_lose_control_bidi_and_zero_width_characters() {
    assert_eq!(sanitize_name("A\u{0}d\u{7}a\n\tB\u{7F}\u{85}"), "AdaB");
    assert_eq!(
        sanitize_name("\u{202E}Ada\u{202C} \u{2066}Bo\u{2069}\u{200E}\u{200F}\u{61C}"),
        "Ada Bo"
    );
    assert_eq!(sanitize_name("\u{200B} Ada\u{2060}\u{FEFF}\u{AD} "), "Ada");
    assert_eq!(sanitize_name("\u{200D}Ada\u{200D}"), "Ada");
    assert_eq!(sanitize_name("A\u{200D}\u{200D}d"), "A\u{200D}d");
    assert_eq!(sanitize_name("\u{FE0F}Ada"), "Ada");
    assert_eq!(sanitize_name("Ada\u{E0041}\u{E0042}"), "Ada");
}

#[test]
fn names_are_cut_to_40_code_points_and_may_be_empty() {
    assert_eq!(sanitize_name(&"x".repeat(50)), "x".repeat(40));
    assert_eq!(sanitize_name(&"😀".repeat(45)), "😀".repeat(40));
    assert_eq!(
        sanitize_name(&format!("{} bc", "a".repeat(39))),
        "a".repeat(39)
    );
    assert_eq!(sanitize_name(" \u{200B}\u{202E}\n "), "");
}

#[test]
fn the_placeholder_has_initials_and_a_colour_from_the_id() {
    assert_eq!(initials("ada lovelace"), "AL");
    assert_eq!(initials("Ada King Lovelace"), "AL");
    assert_eq!(initials("Ada"), "A");
    assert_eq!(initials(""), "");
    assert_eq!(
        initials("\u{1F469}\u{200D}\u{1F467} Mum"),
        "\u{1F469}\u{200D}\u{1F467}M"
    );
    assert_eq!(color("abc"), color("abc"));
    assert!(COLORS.contains(&color("abc")));
}

#[test]
fn the_short_code_depends_on_id_and_key() {
    let key = crate::SigningKey::from_seed([0; 32]).public_key();
    let mut seed = [0; 32];
    seed[0] = 1;
    let other = crate::SigningKey::from_seed(seed).public_key();
    let code = short_code("p1", &key);
    assert_eq!(code.len(), 9);
    assert_eq!(&code[4..5], "-");
    assert_eq!(code, short_code("p1", &key));
    assert_ne!(code, short_code("p1", &other));
    assert_ne!(code, short_code("p2", &key));
}

#[test]
fn a_chosen_square_becomes_a_256_jpeg_without_metadata() {
    let source = halves(600, 400, ImageFormat::Png);
    assert_eq!(picture_size(&source), Some((600, 400)));
    let picture = make_picture(&source, 300, 0, 400);
    assert_profile_picture(picture.as_deref());
    let pixel = image::load_from_memory(&picture.unwrap_or_default())
        .map(|i| i.to_rgb8().get_pixel(128, 128).0)
        .unwrap_or_default();
    assert!(pixel[2] > 200 && pixel[0] < 60, "{pixel:?}");
    assert_profile_picture(make_picture(&source, 900, -40, 2000).as_deref());
}

#[test]
fn a_picture_received_is_made_again_and_refused_when_too_big_or_none() {
    let jpeg = halves(512, 512, ImageFormat::Jpeg);
    // An Exif segment that must not come through.
    let mut with_exif = jpeg[..2].to_vec();
    with_exif.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x08, b'E', b'x', b'i', b'f', 0, 0]);
    with_exif.extend_from_slice(&jpeg[2..]);
    assert_profile_picture(received_picture(&with_exif).as_deref());
    let mut big = halves(64, 64, ImageFormat::Png);
    big.resize(PICTURE_MAX_BYTES + 1, 0);
    assert_eq!(received_picture(&big), None);
    assert_eq!(received_picture(b"hi"), None);
    assert_eq!(received_picture(&[0xFF, 0xD8, 0xFF, 0, 1, 2]), None);
    let png = halves(64, 64, ImageFormat::Png);
    assert_eq!(received_picture(&png[..png.len() / 2]), None);
    // A format left out, even when it would decode.
    assert_eq!(
        received_picture(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"),
        None
    );
}

#[test]
fn a_canvas_wider_than_the_decoder_takes_is_refused() {
    let wide = |width| halves(width, 1, ImageFormat::Png);
    assert_eq!(picture_size(&wide(8192)), Some((8192, 1)));
    assert_eq!(picture_size(&wide(8193)), None);
    assert_eq!(square_picture(&wide(8193)), None);
    // Pictures from others are made at 256: anything above 1024 is
    // refused.
    assert_profile_picture(received_picture(&wide(1024)).as_deref());
    assert_eq!(received_picture(&wide(1025)), None);
}

#[test]
fn records_are_written_sorted_and_malformed_ones_refused() {
    let profile = Profile {
        id: "p1".into(),
        name: "Ada".into(),
        picture: Some(vec![1, 2, 3]),
        version: 2,
    };
    assert_eq!(
        profile.encode(),
        r#"{"id":"p1","name":"Ada","picture":"AQID","version":2}"#
    );
    assert_eq!(Profile::parse(&profile.encode()), Some(profile));
    for bad in [
        "nope",
        r#"{"id":"","name":"A","version":1}"#,
        r#"{"id":"p","name":"A","version":1.5}"#,
        r#"{"id":"p","name":"A","version":0}"#,
        r#"{"id":"p","name":3,"version":1}"#,
        r#"{"id":"p","name":"A","picture":"%%","version":1}"#,
        r#"{"id":"p","name":"A","picture":"AQ-_","version":1}"#,
        r#"{"id":"p","name":"A","picture":"AQJ=","version":1}"#,
        r#"{"id":"p","name":"\ud800A","version":1}"#,
    ] {
        assert_eq!(Profile::parse(bad), None, "{bad}");
    }
}

#[test]
fn the_own_profile_follows_the_defaults_until_the_user_chose() -> TestResult {
    let store = crate::store::tests::store();
    let own = OwnProfile::new(&store);
    assert_eq!(own.get()?, None);
    assert!(own.rename("Ada").is_err());
    let made = own.apply_defaults(" desk ", None)?;
    assert_eq!((made.name.as_str(), made.version), ("desk", 1));
    assert_eq!(
        own.first_key()?,
        Some(store.signing_key()?.public_key().to_vec())
    );
    assert_eq!(
        own.code()?,
        Some(short_code(&made.id, &store.signing_key()?.public_key()))
    );
    assert_eq!(own.apply_defaults("desk", None)?.version, 1);
    assert_eq!(own.apply_defaults("Ada's desk", None)?.version, 2);
    own.rename("Ada")?;
    assert_eq!(own.apply_defaults("other", None)?.name, "Ada");
    assert_eq!(own.rename("Ada\u{200B}")?.version, 3);
    own.set_picture(None)?;
    let picture = square_picture(&halves(64, 64, ImageFormat::Png));
    assert_eq!(own.apply_defaults("other", picture)?.picture, None);
    Ok(())
}

// ------------------------------------- between one's own Devices

use crate::folder::{MemoryFolder, folder_sync, random_nonce};
use crate::lan::{Session, answer_sync, apply_answer, sync_request};
use crate::signing::SigningKey;
use crate::store::tests::ticking;

/// A Device with its own Profile made from the default `name`.
fn device(seed: u8, name: &str) -> std::result::Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// One LAN session without the network: `joiner` joins `host`.
fn exchange(
    joiner: &Store,
    host: &Store,
) -> std::result::Result<(Value, Value, Session, Session), Box<dyn std::error::Error>> {
    let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
    let (answer, served) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
    let back = apply_answer(joiner, &answer)?;
    Ok((request, answer, served, back))
}

/// The Profile `store` holds, as both Devices must agree on it.
fn held(store: &Store) -> Result<(Option<String>, Option<i64>, Option<String>)> {
    let own = OwnProfile::new(store);
    let profile = own.get()?;
    Ok((
        profile.as_ref().map(|p| p.id.clone()),
        profile.map(|p| p.version),
        own.code()?,
    ))
}

fn id(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

#[test]
fn both_with_the_defaults_keep_the_lower_id() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Ada Lovelace")?;
    let mut ids = [id(&phone)?, id(&desk)?];
    ids.sort();
    let (request, answer, _, _) = exchange(&phone, &desk)?;
    assert!(request.get("profile").is_some());
    assert!(answer.get("profile").is_some());
    assert_eq!(id(&phone)?, ids[0]);
    assert_eq!(held(&phone)?, held(&desk)?);
    exchange(&phone, &desk)?;
    for store in [&phone, &desk] {
        assert_eq!(OwnProfile::new(store).aliases()?, vec![ids[1].clone()]);
        assert_eq!(OwnProfile::new(store).question()?, None);
    }
    Ok(())
}

#[test]
fn the_defaults_take_a_changed_profile_over_without_a_question() -> TestResult {
    for joiner_chose in [true, false] {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Ada Lovelace")?;
        let chosen = if joiner_chose { &phone } else { &desk };
        OwnProfile::new(chosen).rename("Ada")?;
        let kept = id(chosen)?;
        let (_, _, served, back) = exchange(&phone, &desk)?;
        assert_eq!(id(&phone)?, kept);
        assert_eq!(held(&phone)?, held(&desk)?);
        let expected = if joiner_chose {
            (ProfileTaken::Adopted, ProfileTaken::Updated)
        } else {
            (ProfileTaken::Unchanged, ProfileTaken::Adopted)
        };
        assert_eq!((served.profile, back.profile), expected);
        assert_eq!(OwnProfile::new(&phone).question()?, None);
        assert_eq!(OwnProfile::new(&desk).question()?, None);
    }
    Ok(())
}

#[test]
fn both_changed_ask_and_the_first_answer_settles_it() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    OwnProfile::new(&phone).rename("Ada L.")?;
    OwnProfile::new(&desk).rename("Ada")?;
    let (phone_id, desk_id) = (id(&phone)?, id(&desk)?);
    let (_, _, served, back) = exchange(&phone, &desk)?;
    assert_eq!(
        (served.profile, back.profile),
        (ProfileTaken::Asked, ProfileTaken::Asked)
    );
    let asked = OwnProfile::new(&phone).question()?.ok_or("a question")?;
    assert_eq!(asked.profile.name, "Ada");
    assert_eq!(Some(asked.code), OwnProfile::new(&desk).code()?);
    assert_eq!(id(&phone)?, phone_id);

    OwnProfile::new(&phone).answer(false)?;
    assert_eq!(OwnProfile::new(&phone).question()?, None);
    assert_eq!(id(&phone)?, desk_id);
    exchange(&phone, &desk)?;
    assert_eq!(OwnProfile::new(&desk).question()?, None);
    assert_eq!(held(&phone)?, held(&desk)?);
    assert_eq!(OwnProfile::new(&desk).aliases()?, vec![phone_id]);
    Ok(())
}

#[test]
fn answered_apart_against_each_other_the_lower_id_stays() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    OwnProfile::new(&phone).rename("Ada L.")?;
    OwnProfile::new(&desk).rename("Ada")?;
    let lower = id(&phone)?.min(id(&desk)?);
    exchange(&phone, &desk)?;
    OwnProfile::new(&phone).answer(true)?;
    OwnProfile::new(&desk).answer(true)?;
    exchange(&phone, &desk)?;
    exchange(&phone, &desk)?;
    exchange(&phone, &desk)?;
    assert_eq!(id(&phone)?, lower);
    assert_eq!(held(&phone)?, held(&desk)?);
    Ok(())
}

#[test]
fn a_shared_profile_keeps_its_name_from_either_devices_defaults() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    exchange(&phone, &desk)?;
    for store in [&phone, &desk] {
        let own = OwnProfile::new(store);
        let version = own.get()?.ok_or("a Profile")?.version;
        assert_eq!(own.apply_defaults("Renamed", None)?.version, version);
    }
    // A name the user typed on one shows on the other at once.
    OwnProfile::new(&desk).rename("Ada")?;
    let (_, _, _, back) = exchange(&phone, &desk)?;
    assert_eq!(back.profile, ProfileTaken::Updated);
    assert_eq!(
        OwnProfile::new(&phone).get()?.ok_or("a Profile")?.name,
        "Ada"
    );
    Ok(())
}

#[test]
fn changed_apart_at_the_same_version_the_one_with_a_picture_stays() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    OwnProfile::new(&desk).rename("Ada")?;
    exchange(&phone, &desk)?;
    // "Bea" sorts after "Ada": the larger bytes alone would keep the
    // phone's.
    OwnProfile::new(&phone).rename("Bea")?;
    OwnProfile::new(&desk).set_picture(square_picture(&halves(64, 64, ImageFormat::Png)))?;
    exchange(&phone, &desk)?;
    let kept = OwnProfile::new(&phone).get()?.ok_or("a Profile")?;
    assert_eq!(kept.name, "Ada");
    assert!(kept.picture.is_some());
    assert_eq!(held(&phone)?, held(&desk)?);
    Ok(())
}

#[test]
fn an_update_signed_by_a_key_not_bound_is_refused() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    OwnProfile::new(&desk).rename("Ada")?;
    exchange(&phone, &desk)?;
    let before = OwnProfile::new(&phone).get()?;
    let sent = OwnProfile::new(&desk).signed()?.ok_or("signed")?;
    let mine = OwnProfile::new(&desk).get()?.ok_or("a Profile")?;
    let stranger = SigningKey::from_seed([9; 32]);
    let stranger_b64 = BASE64.encode(stranger.public_key());
    let forged = Profile {
        id: mine.id.clone(),
        name: "Mallory".into(),
        picture: None,
        version: mine.version + 5,
    }
    .encode();
    let signed_by = |key: &SigningKey, keys: Vec<ProfileKey>| SignedProfile {
        record: forged.clone(),
        signer: BASE64.encode(key.public_key()),
        sig: key.sign(&record_bytes(&forged)),
        first: sent.first.clone(),
        keys,
        aliases: Vec::new(),
        fresh: false,
    };
    let desk_key = SigningKey::from_seed([2; 32]);
    let from = desk_key.public_key();
    let own = OwnProfile::new(&phone);
    let taken = own.take(
        Some(&signed_by(&stranger, sent.keys.clone()).to_json()),
        &from,
    )?;
    assert_eq!(taken, ProfileTaken::Refused);
    let mut self_bound = sent.keys.clone();
    self_bound.push(ProfileKey {
        key: stranger_b64.clone(),
        by: stranger_b64.clone(),
        sig: stranger.sign(&key_bytes(&mine.id, &stranger_b64)),
    });
    let taken = own.take(Some(&signed_by(&stranger, self_bound).to_json()), &from)?;
    assert_eq!(taken, ProfileTaken::Refused);
    let mut changed = sent.to_json();
    changed["record"] = json!(forged);
    assert_eq!(own.take(Some(&changed), &from)?, ProfileTaken::Refused);
    let copy = SignedProfile {
        first: stranger_b64.clone(),
        ..signed_by(&stranger, Vec::new())
    };
    assert_eq!(
        own.take(Some(&copy.to_json()), &from)?,
        ProfileTaken::Refused
    );
    assert_eq!(own.get()?, before);
    // Signed by a bound key, it counts.
    let good = signed_by(&desk_key, sent.keys.clone()).to_json();
    assert_eq!(own.take(Some(&good), &from)?, ProfileTaken::Updated);
    assert_eq!(own.get()?.ok_or("a Profile")?.name, "Mallory");
    Ok(())
}

#[test]
fn a_record_whose_signature_fails_is_refused_before_it_is_read() -> TestResult {
    let key = SigningKey::from_seed([9; 32]);
    let key_b64 = BASE64.encode(key.public_key());
    let record = json!({
        "id": "e".repeat(32),
        "name": "Eve",
        "picture": format!("*not base64*{}", "A".repeat(1000)),
        "version": 1,
    })
    .to_string();
    let sent = |sig: String| SignedProfile {
        record: record.clone(),
        signer: key_b64.clone(),
        sig,
        first: key_b64.clone(),
        keys: Vec::new(),
        aliases: Vec::new(),
        fresh: false,
    };
    let before = PARSED.with(|n| n.get());
    assert!(check(&sent(key.sign(&record_bytes("{}")))).is_none());
    assert_eq!(PARSED.with(|n| n.get()), before, "nothing of it was read");
    // Signed, the same record is read, and refused for its picture.
    assert!(check(&sent(key.sign(&record_bytes(&record)))).is_none());
    assert!(PARSED.with(|n| n.get()) > before);
    Ok(())
}

#[test]
fn taking_over_or_restoring_keeps_only_aliases_a_bound_key_made() -> TestResult {
    let desk = device(2, "Desk")?;
    OwnProfile::new(&desk).rename("Ada")?;
    let desk_id = id(&desk)?;
    let stranger = SigningKey::from_seed([9; 32]);
    let stranger_id = "a".repeat(32);
    let forged = ProfileAlias {
        alias: stranger_id.clone(),
        to: desk_id.clone(),
        by: BASE64.encode(stranger.public_key()),
        sig: stranger.sign(&alias_bytes(&stranger_id, &desk_id)),
    };
    assert!(forged.signed());
    desk.set_local_setting(ALIASES, &Value::Array(vec![forged.to_json()]).to_string())?;
    // A Device with the defaults takes the computer's over.
    let phone = device(1, "Pixel 7")?;
    exchange(&phone, &desk)?;
    assert_eq!(id(&phone)?, desk_id);
    assert!(!OwnProfile::new(&phone).aliases()?.contains(&stranger_id));
    // A Backup carrying it restores none either.
    let pad = device(3, "Pad")?;
    let sent = OwnProfile::new(&desk).signed()?.ok_or("signed")?.to_json();
    assert!(OwnProfile::new(&pad).restore(Some(&sent), &desk.signing_key()?)?);
    assert_eq!(id(&pad)?, desk_id);
    assert!(OwnProfile::new(&pad).aliases()?.is_empty());
    Ok(())
}

#[test]
fn a_record_of_a_later_structure_is_read_as_far_as_it_can_be() {
    for known in [
        r#"{"id":"p","name":"A","version":1}"#,
        r#"{"id":"p","name":"A","v":1,"version":1}"#,
        "nope",
    ] {
        assert!(!Profile::record_is_newer(known), "{known}");
    }
    for (newer, name, picture) in [
        (r#"{"id":"p","name":"A","v":2,"version":3}"#, "A", false),
        (r#"{"id":"p","name":"A","v":"1","version":3}"#, "A", false),
        (r#"{"id":"p","name":"A","v":1.5,"version":3}"#, "A", false),
        (r#"{"id":"p","name":"A","v":0,"version":3}"#, "A", false),
        (r#"{"id":"p","name":"A","v":null,"version":3}"#, "A", false),
        (
            r#"{"id":"p","name":"A","pronouns":"she","version":3}"#,
            "A",
            false,
        ),
        (
            r#"{"id":"p","name":{"en":"A"},"v":2,"version":3}"#,
            "",
            false,
        ),
        (
            r#"{"id":"p","name":"A","picture":"%%","v":2,"version":3}"#,
            "A",
            false,
        ),
        (
            r#"{"id":"p","name":"A","picture":"AQID","v":2,"version":3}"#,
            "A",
            true,
        ),
    ] {
        assert!(Profile::record_is_newer(newer), "{newer}");
        let profile = Profile::parse(newer);
        assert_eq!(
            profile.as_ref().map(|p| p.id.as_str()),
            Some("p"),
            "{newer}"
        );
        assert_eq!(profile.as_ref().map(|p| p.version), Some(3), "{newer}");
        assert_eq!(
            profile.as_ref().map(|p| p.name.as_str()),
            Some(name),
            "{newer}"
        );
        assert_eq!(
            profile.as_ref().map(|p| p.picture.is_some()),
            Some(picture),
            "{newer}"
        );
    }
    // Its id and version stay what they always were.
    for bad in [
        r#"{"name":"A","v":2,"version":3}"#,
        r#"{"id":"p","name":"A","v":2}"#,
        r#"{"id":"p","name":"A","v":2,"version":"3"}"#,
    ] {
        assert_eq!(Profile::parse(bad), None, "{bad}");
    }
}

#[test]
fn a_record_of_a_later_structure_is_taken_shown_and_never_written_anew() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    OwnProfile::new(&desk).rename("Ada")?;
    exchange(&phone, &desk)?;
    let key = SigningKey::from_seed([2; 32]);
    let mine = OwnProfile::new(&desk).get()?.ok_or("a Profile")?;
    // As the desk's later version writes it, its picture none here.
    let later = format!(
        r#"{{"id":"{}","name":"Ada L.","picture":"{}","pronouns":"she","v":2,"version":{}}}"#,
        mine.id,
        BASE64.encode(b"a picture of later"),
        mine.version + 1
    );
    let mut sent = OwnProfile::new(&desk).to_send()?.ok_or("sent")?;
    sent["record"] = json!(later);
    sent["sig"] = json!(key.sign(&record_bytes(&later)));
    let own = OwnProfile::new(&phone);
    assert_eq!(
        own.take(Some(&sent), &key.public_key())?,
        ProfileTaken::Updated
    );
    assert!(own.is_newer()?);
    assert!(phone.person_is_newer(&mine.id)?);
    let shown = own.get()?.ok_or("a Profile")?;
    assert_eq!(shown.name, "Ada L.");
    assert_eq!(shown.picture, None);
    assert_eq!(own.signed()?.map(|s| s.record), Some(later.clone()));
    assert!(own.rename("Ada").is_err());
    // The same name again, as a page left unchanged keeps it, is fine.
    own.rename("Ada L.")?;
    own.set_picture(None)?;
    let picture = square_picture(&halves(300, 300, ImageFormat::Png));
    assert!(own.set_picture(picture).is_err());
    own.apply_defaults("Pixel 8", None)?;
    assert_eq!(own.signed()?.map(|s| s.record), Some(later));
    assert!(!OwnProfile::new(&desk).is_newer()?);
    assert!(!desk.person_is_newer(&mine.id)?);
    Ok(())
}

#[test]
fn a_received_picture_shows_made_again_and_none_is_refused() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    let picture = square_picture(&halves(300, 300, ImageFormat::Png));
    OwnProfile::new(&desk).set_picture(picture)?;
    exchange(&phone, &desk)?;
    assert_eq!(id(&phone)?, id(&desk)?);
    assert_profile_picture(
        OwnProfile::new(&phone)
            .get()?
            .and_then(|p| p.picture)
            .as_deref(),
    );
    let key = SigningKey::from_seed([2; 32]);
    let mine = OwnProfile::new(&desk).get()?.ok_or("a Profile")?;
    let broken = json!({
        "id": mine.id,
        "name": mine.name,
        "picture": BASE64.encode(b"no picture at all"),
        "version": mine.version + 1,
    })
    .to_string();
    let mut sent = OwnProfile::new(&desk).to_send()?.ok_or("sent")?;
    sent["record"] = json!(broken);
    sent["sig"] = json!(key.sign(&record_bytes(&broken)));
    let taken = OwnProfile::new(&phone).take(Some(&sent), &key.public_key())?;
    assert_eq!(taken, ProfileTaken::Refused);
    Ok(())
}

#[test]
fn the_shared_folder_carries_it_sealed() -> TestResult {
    let phone = device(1, "Pixel 7")?;
    let desk = device(2, "Desk")?;
    OwnProfile::new(&desk).rename("Ada")?;
    exchange(&phone, &desk)?;
    OwnProfile::new(&desk).rename("Ada Lovelace")?;
    let mut folder = MemoryFolder::default();
    folder_sync(&desk, &mut folder, &mut random_nonce)?;
    let read = folder_sync(&phone, &mut folder, &mut random_nonce)?;
    assert!(read.profile_changed);
    assert_eq!(
        OwnProfile::new(&phone).get()?.ok_or("a Profile")?.name,
        "Ada Lovelace"
    );
    assert_eq!(held(&phone)?, held(&desk)?);
    Ok(())
}
