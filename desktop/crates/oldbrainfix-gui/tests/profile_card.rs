//! Profile cards in the window (#107, #144): "Send my Profile" saves the
//! own card as a file, "Show my QR" shows it as a QR, and "Add a person
//! from a link" in People opens a pasted card: a person added, updated
//! or already kept opens their Profile, not verified, a new one offering
//! "Send yours back"; every other card says why under the field. A card
//! file dropped onto the window or handed over opens the same way, and
//! one that adds nobody is announced at the top, why behind the click.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::cards::{self, LINK, card_bytes};
use oldbrainfix_core::profile::{SignedProfile, record_bytes};
use oldbrainfix_core::sent;
use oldbrainfix_core::signing::KeyRecord;
use oldbrainfix_core::{CardTaken, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A Device with its own Profile named `name`, on a clock of its own.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    let own = OwnProfile::new(&store);
    own.apply_defaults("Device", None)?;
    own.rename(name)?;
    Ok(store)
}

fn id_of(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

fn card(store: &Store, back: bool) -> Result<String, Box<dyn std::error::Error>> {
    Ok(store.profile_card(back)?.ok_or("a card")?)
}

/// The saves asked for: the dialog's title and the name it offered.
type Asked = Arc<Mutex<Vec<(String, String)>>>;

/// Ann's window on her Profile page.
fn window(ann: Store, locale: &str) -> Harness<'static, App> {
    window_of(App::new(ann, locale))
}

/// Ann's window on her Profile page, her card saved under `dir` as the
/// dialog offers it to be named, or as `named` when given.
fn window_saving(ann: Store, dir: &Path, named: Option<&str>) -> (Harness<'static, App>, Asked) {
    let asked = Asked::default();
    let mut app = App::new(ann, "en");
    let (record, dir, named) = (asked.clone(), dir.to_path_buf(), named.map(String::from));
    app.set_card_saver(Box::new(move |title, name| {
        if let Ok(mut asked) = record.lock() {
            asked.push((title.to_string(), name.to_string()));
        }
        Some(dir.join(named.as_deref().unwrap_or(name)))
    }));
    (window_of(app), asked)
}

fn window_of(mut app: App) -> Harness<'static, App> {
    app.select(Selection::Profile);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.run();
}

/// Pastes `link` into "Their Profile link" and clicks "Add a person
/// from a link".
fn add_from(harness: &mut Harness<'_, App>, link: &str) {
    let field = harness.get_by_role_and_label(Role::TextInput, "Their Profile link");
    field.focus();
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "Their Profile link")
        .type_text(link);
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    click(harness, "Add a person from a link");
}

/// Drops the file at `path` onto the window.
fn drop_file(harness: &mut Harness<'_, App>, path: &Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

/// Writes `bytes` as the file `name` under `dir`.
fn file(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(name);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

fn card_file(store: &Store, back: bool) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(store.profile_card_file(back)?.ok_or("a card file")?)
}

/// Drops the file `bytes` onto the window and opens why it added
/// nobody, announced at the top.
fn drop_refused(harness: &mut Harness<'_, App>, dir: &Path, bytes: &[u8]) -> TestResult {
    drop_file(harness, &file(dir, "card.obfcard", bytes)?);
    assert_eq!(
        harness.state().notice(),
        Some("Nobody was added from the card.")
    );
    assert!(!shows(harness, "Name and picture for me"));
    harness
        .get_by_label("Nobody was added from the card.")
        .click();
    harness.run();
    assert!(shows(harness, "Nobody added"));
    assert!(shows(harness, "Report"));
    Ok(())
}

#[test]
fn send_my_profile_saves_the_card_as_a_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ann = device(1, "Ann")?;
    let ann_id = id_of(&ann)?;
    let ben = device(2, "Ben")?;
    let (mut harness, asked) = window_saving(ann, dir.path(), None);
    assert!(!shows(&harness, "Saved."));
    click(&mut harness, "Send my Profile");
    let asked = asked.lock().map_err(|_| "the saves")?.clone();
    assert_eq!(
        asked,
        [(
            "Save your Profile card to send".to_string(),
            "Ann.obfcard".to_string()
        )]
    );
    assert!(shows(
        &harness,
        "Saved. Send the file through any messenger"
    ));
    // No announcement for a save the page shows.
    assert_eq!(harness.state().notice(), None);
    let bytes = std::fs::read(dir.path().join("Ann.obfcard"))?;
    assert!(cards::looks_like_profile_card(&bytes));
    let opened = ben.open_profile_card_file(&bytes)?;
    assert_eq!(opened.taken, CardTaken::Added);
    assert_eq!(opened.id, Some(ann_id));
    assert!(opened.offer_back);
    Ok(())
}

#[test]
fn a_profile_without_a_name_saves_its_card_as_no_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ann = device(1, "Ann")?;
    OwnProfile::new(&ann).rename("")?;
    let (mut harness, asked) = window_saving(ann, dir.path(), None);
    click(&mut harness, "Send my Profile");
    let asked = asked.lock().map_err(|_| "the saves")?.clone();
    assert_eq!(asked.first().map(|a| a.1.as_str()), Some("No name.obfcard"));
    assert!(dir.path().join("No name.obfcard").exists());
    Ok(())
}

#[test]
fn a_save_dialog_closed_saves_nothing_and_says_nothing() -> TestResult {
    let mut app = App::new(device(1, "Ann")?, "en");
    app.set_card_saver(Box::new(|_, _| None));
    let mut harness = window_of(app);
    click(&mut harness, "Send my Profile");
    assert!(!shows(&harness, "Saved."));
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn a_card_file_that_cannot_be_written_is_announced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (mut harness, _) = window_saving(device(1, "Ann")?, dir.path(), Some("gone/card"));
    click(&mut harness, "Send my Profile");
    assert!(!shows(&harness, "Saved."));
    assert_eq!(
        harness.state().notice(),
        Some("Your Profile card was not sent.")
    );
    harness
        .get_by_label("Your Profile card was not sent.")
        .click();
    harness.run();
    assert!(shows(&harness, "Profile card not sent"));
    assert!(shows(&harness, "could not be written"));
    Ok(())
}

#[test]
fn a_card_file_dropped_adds_the_person_and_sends_yours_back_as_a_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ann = device(1, "Ann")?;
    let ann_id = id_of(&ann)?;
    let ben = device(2, "Ben")?;
    let ben_id = id_of(&ben)?;
    let bytes = card_file(&ben, false)?;
    // A card file reads as a Sent Item too: it is asked for first.
    assert!(sent::looks_like_sent_item(&bytes));
    let (mut harness, _) = window_saving(ann, dir.path(), Some("back"));
    drop_file(&mut harness, &file(dir.path(), "Ben.obfcard", &bytes)?);
    assert_eq!(harness.state().notice(), None);
    assert_eq!(harness.state().store().person_theirs(&ben_id)?.name, "Ben");
    assert!(shows(&harness, "Name and picture for me"));
    assert!(shows(&harness, "Not verified"));
    assert!(!shows(&harness, "Accept"));
    click(&mut harness, "Send yours back");
    assert!(!shows(&harness, "So Ben can add you too."));
    assert!(shows(
        &harness,
        "Saved. Send the file to Ben through any messenger"
    ));
    // Saved under the name given, with the card's ending; it answers
    // Ben's and asks nothing back.
    let back = ben.open_profile_card_file(&std::fs::read(dir.path().join("back.obfcard"))?)?;
    assert_eq!(back.taken, CardTaken::Added);
    assert_eq!(back.id, Some(ann_id));
    assert!(!back.offer_back);
    Ok(())
}

#[test]
fn a_card_file_handed_over_opens_the_person() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ben = device(2, "Ben")?;
    let path = file(dir.path(), "Ben.obfcard", &card_file(&ben, true)?)?;
    let handed = Arc::new(Mutex::new(vec![path]));
    let mut app = App::new(device(1, "Ann")?, "en");
    app.set_handed_files(handed);
    let harness = window_of(app);
    assert!(shows(&harness, "Name and picture for me"));
    assert!(!shows(&harness, "Send yours back"));
    assert_eq!(
        harness.state().store().person_theirs(&id_of(&ben)?)?.name,
        "Ben"
    );
    Ok(())
}

#[test]
fn ones_own_card_file_adds_nobody_and_says_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ann = device(1, "Ann")?;
    let bytes = card_file(&ann, false)?;
    let mut harness = window(ann, "en");
    drop_refused(&mut harness, dir.path(), &bytes)?;
    assert!(shows(&harness, "This is your own Profile card."));
    assert!(shows(&harness, "open their card, not yours"));
    assert!(harness.state().store().kept_people()?.is_empty());
    Ok(())
}

#[test]
fn a_blocked_persons_card_file_adds_nobody_and_says_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    ann.open_profile_card(&card(&ben, false)?, false)?;
    ann.block_person(&id_of(&ben)?, false)?;
    let mut harness = window(ann, "en");
    drop_refused(&mut harness, dir.path(), &card_file(&ben, false)?)?;
    assert!(shows(&harness, "This card is from a person you blocked."));
    assert!(shows(&harness, "unblock them"));
    Ok(())
}

#[test]
fn a_changed_card_file_adds_nobody_and_says_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ben = device(2, "Ben")?;
    let mut changed: serde_json::Value = serde_json::from_slice(&card_file(&ben, false)?)?;
    let card = changed["card"].as_str().ok_or("a card")?.to_string();
    let last = card.chars().last().ok_or("a character")?;
    changed["card"] = json!(format!(
        "{}{}",
        &card[..card.len() - 1],
        if last == 'A' { 'B' } else { 'A' }
    ));
    for bytes in [
        changed.to_string().into_bytes(),
        br#"{"card":"x.y","type":"oldbrainfix-card"}"#.to_vec(),
    ] {
        let mut harness = window(device(1, "Ann")?, "en");
        drop_refused(&mut harness, dir.path(), &bytes)?;
        assert!(shows(&harness, "This is not a Profile card"));
        assert!(shows(
            &harness,
            "Ask the person to send their Profile again."
        ));
        assert!(harness.state().store().kept_people()?.is_empty());
    }
    Ok(())
}

#[test]
fn a_card_file_of_a_later_format_says_an_update_is_needed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ben = device(2, "Ben")?;
    let mut later: serde_json::Value = serde_json::from_slice(&card_file(&ben, false)?)?;
    later["format"] = json!(2);
    let mut harness = window(device(1, "Ann")?, "en");
    drop_refused(&mut harness, dir.path(), later.to_string().as_bytes())?;
    assert!(shows(
        &harness,
        "This file comes from a newer Old Brain Fix."
    ));
    assert!(shows(&harness, "Update Old Brain Fix on this Device"));
    assert!(harness.state().store().kept_people()?.is_empty());
    Ok(())
}

#[test]
fn show_my_qr_shows_the_card_as_a_large_qr() -> TestResult {
    let ann = device(1, "Ann")?;
    let mut harness = window(ann, "en");
    click(&mut harness, "Show my QR");
    let qr = harness.get_by_role_and_label(Role::Image, "My Profile QR");
    assert!(qr.rect().width() >= 400.0);
    assert!(shows(&harness, "The other person scans this"));
    assert!(!shows(&harness, "too many Devices"));
    click(&mut harness, "Close");
    assert!(harness.query_by_label("My Profile QR").is_none());
    Ok(())
}

#[test]
fn a_pasted_link_adds_the_person_not_verified_and_sends_yours_back() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ann = device(1, "Ann")?;
    let ann_id = id_of(&ann)?;
    let ben = device(2, "Ben")?;
    let ben_id = id_of(&ben)?;
    let (mut harness, _) = window_saving(ann, dir.path(), None);
    add_from(&mut harness, &card(&ben, false)?);
    assert!(harness.state().store().person_theirs(&ben_id)?.name == "Ben");
    assert!(shows(&harness, "Name and picture for me"));
    assert!(shows(&harness, "Not verified"));
    assert!(shows(&harness, "So Ben can add you too."));
    click(&mut harness, "Send yours back");
    assert!(!shows(&harness, "So Ben can add you too."));
    assert!(shows(&harness, "Saved. Send the file to Ben"));
    // Ann's card answers Ben's: it asks nothing back.
    let back = ben.open_profile_card_file(&std::fs::read(dir.path().join("Ann.obfcard"))?)?;
    assert_eq!(back.taken, CardTaken::Added);
    assert_eq!(back.id, Some(ann_id));
    assert!(!back.offer_back);
    // The field was emptied for the next one.
    click(&mut harness, "Close");
    let field = harness.get_by_role_and_label(Role::TextInput, "Their Profile link");
    assert_eq!(field.value().unwrap_or_default(), "");
    Ok(())
}

#[test]
fn an_answering_card_opens_the_profile_without_asking_back() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let mut harness = window(ann, "en");
    add_from(&mut harness, &card(&ben, true)?);
    assert!(shows(&harness, "Not verified"));
    assert!(!shows(&harness, "Send yours back"));
    Ok(())
}

#[test]
fn a_card_kept_or_newer_opens_the_profile_without_asking_back() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let ben_id = id_of(&ben)?;
    let link = card(&ben, false)?;
    let mut harness = window(ann, "en");
    add_from(&mut harness, &link);
    click(&mut harness, "Close");
    // The same card again: unchanged.
    add_from(&mut harness, &link);
    assert!(shows(&harness, "Name and picture for me"));
    assert!(!shows(&harness, "Send yours back"));
    click(&mut harness, "Close");
    // A newer one: updated.
    OwnProfile::new(&ben).rename("Benjamin")?;
    add_from(&mut harness, &card(&ben, false)?);
    assert!(shows(&harness, "Benjamin"));
    assert!(!shows(&harness, "Send yours back"));
    assert_eq!(
        harness.state().store().person_theirs(&ben_id)?.name,
        "Benjamin"
    );
    Ok(())
}

#[test]
fn ones_own_card_adds_nobody_and_says_so() -> TestResult {
    let ann = device(1, "Ann")?;
    let link = card(&ann, false)?;
    let mut harness = window(ann, "en");
    add_from(&mut harness, &link);
    assert!(shows(&harness, "This is your own Profile card."));
    assert!(!shows(&harness, "Name and picture for me"));
    assert!(harness.state().store().kept_people()?.is_empty());
    Ok(())
}

#[test]
fn a_blocked_persons_card_is_refused_and_says_so() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let link = card(&ben, false)?;
    ann.open_profile_card(&link, false)?;
    ann.block_person(&id_of(&ben)?, false)?;
    let mut harness = window(ann, "en");
    add_from(&mut harness, &link);
    assert!(shows(&harness, "This card is from a person you blocked."));
    assert!(!shows(&harness, "Name and picture for me"));
    Ok(())
}

#[test]
fn a_link_without_a_card_says_so_under_the_field_and_stays() -> TestResult {
    let ben = device(2, "Ben")?;
    let link = card(&ben, false)?;
    // One character of the signature changed.
    let last = link.chars().last().ok_or("a character")?;
    let changed = format!(
        "{}{}",
        &link[..link.len() - 1],
        if last == 'A' { 'B' } else { 'A' }
    );
    for text in ["hello", changed.as_str()] {
        let mut harness = window(device(1, "Ann")?, "en");
        add_from(&mut harness, text);
        assert!(shows(&harness, "This is not a Profile card"));
        let field = harness.get_by_role_and_label(Role::TextInput, "Their Profile link");
        assert_eq!(field.value().unwrap_or_default(), text);
        assert!(harness.state().store().kept_people()?.is_empty());
    }
    Ok(())
}

#[test]
fn a_card_naming_a_kept_person_under_other_keys_is_refused() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let ben_id = id_of(&ben)?;
    ann.open_profile_card(&card(&ben, false)?, false)?;
    // Eve copies Ben's id into a Profile of her own key.
    let eve = SigningKey::from_seed([3; 32]);
    let record = json!({"id": ben_id, "name": "Ben", "version": 9}).to_string();
    let profile = SignedProfile {
        sig: eve.sign(&record_bytes(&record)),
        record,
        signer: BASE64.encode(eve.public_key()),
        first: BASE64.encode(eve.public_key()),
        keys: Vec::new(),
        aliases: Vec::new(),
        fresh: false,
    };
    let payload = json!({
        "format": 1,
        "key": KeyRecord::make(&eve).to_json(),
        "profile": profile.to_json(),
    })
    .to_string();
    let sig = BASE64.decode(eve.sign(&card_bytes(&payload)))?;
    let copied = format!(
        "{LINK}{}.{}",
        BASE64_URL.encode(payload.as_bytes()),
        BASE64_URL.encode(sig)
    );
    let mut harness = window(ann, "en");
    add_from(&mut harness, &copied);
    assert!(shows(&harness, "This card names a person in People"));
    assert!(!shows(&harness, "Name and picture for me"));
    assert!(!harness.state().store().person_verified(&ben_id)?);
    Ok(())
}

#[test]
fn the_profile_card_speaks_german() -> TestResult {
    let ann = device(1, "Ann")?;
    let harness = window(ann, "de");
    assert!(shows(&harness, "Mein Profil senden"));
    assert!(shows(&harness, "Als kleine Datei"));
    assert!(shows(&harness, "Meinen QR-Code zeigen"));
    assert!(shows(&harness, "Person über einen Link hinzufügen"));
    assert!(shows(&harness, "Profil-Link der Person"));
    assert!(shows(
        &harness,
        "Eine Profilkartendatei, auf dieses Fenster gezogen"
    ));
    Ok(())
}
