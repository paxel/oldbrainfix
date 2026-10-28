//! Profile cards in the window (#107): "Copy my Profile link" puts the
//! own card on the clipboard, "Show my QR" shows it as a QR, and "Add a
//! person from a link" in People opens a pasted card: a person added,
//! updated or already kept opens their Profile, not verified, a new one
//! offering "Send yours back"; every other card says why under the field.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::cards::{LINK, card_bytes};
use oldbrainfix_core::profile::{SignedProfile, record_bytes};
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

/// Ann's window on her Profile page.
fn window(ann: Store, locale: &str) -> Harness<'static, App> {
    let mut app = App::new(ann, locale);
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

/// Clicks the button `label` and returns what went to the clipboard.
fn click_copying(harness: &mut Harness<'_, App>, label: &str) -> Option<String> {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    let mut copied = None;
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                copied = Some(text.clone());
            }
        }
    }
    harness.run();
    copied
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

#[test]
fn copy_my_profile_link_puts_the_card_on_the_clipboard() -> TestResult {
    let ann = device(1, "Ann")?;
    let ann_id = id_of(&ann)?;
    let ben = device(2, "Ben")?;
    let mut harness = window(ann, "en");
    assert!(!shows(&harness, "Copied."));
    let copied = click_copying(&mut harness, "Copy my Profile link").ok_or("a copy")?;
    // The link last, after a line saying what it is.
    assert!(copied.starts_with("My Profile for Old Brain Fix:\n"));
    assert!(copied.contains(LINK));
    assert!(shows(&harness, "Copied. Paste it into a message"));
    // No announcement for a copy the page shows.
    assert_eq!(harness.state().notice(), None);
    let opened = ben.open_profile_card(&copied, false)?;
    assert_eq!(opened.taken, CardTaken::Added);
    assert_eq!(opened.id, Some(ann_id));
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
    let ann = device(1, "Ann")?;
    let ann_id = id_of(&ann)?;
    let ben = device(2, "Ben")?;
    let ben_id = id_of(&ben)?;
    let mut harness = window(ann, "en");
    add_from(&mut harness, &card(&ben, false)?);
    assert!(harness.state().store().person_theirs(&ben_id)?.name == "Ben");
    assert!(shows(&harness, "Name and picture for me"));
    assert!(shows(&harness, "Not verified"));
    assert!(shows(&harness, "So Ben can add you too."));
    let copied = click_copying(&mut harness, "Send yours back").ok_or("a copy")?;
    assert!(!shows(&harness, "So Ben can add you too."));
    assert!(shows(&harness, "Copied. Paste it into a message to Ben."));
    // Ann's card answers Ben's: it asks nothing back.
    let back = ben.open_profile_card(&copied, false)?;
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
    assert!(shows(&harness, "Meinen Profil-Link kopieren"));
    assert!(shows(&harness, "Meinen QR-Code zeigen"));
    assert!(shows(&harness, "Person über einen Link hinzufügen"));
    assert!(shows(&harness, "Profil-Link der Person"));
    Ok(())
}
