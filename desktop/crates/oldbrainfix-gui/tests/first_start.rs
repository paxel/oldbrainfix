//! The first start driven headless (#106): its view asks whether the user
//! is new, adds a Device of their own or restores a Backup, and makes no
//! Profile before the answer. "Add a Device of mine" pairs either way
//! round over the loopback and syncs, taking the other Device's Profile
//! and Notes and skipping the setup: no Starter Set, and the Profile is
//! the other Device's, not one from this Device's defaults; a refusal, a failed join, Cancel and
//! Back return to the three choices with nothing answered.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::join::{SyncFailure, lan_sync};
use oldbrainfix_core::lan::{Host, PairInfo, decode_pair_code};
use oldbrainfix_core::{Error, Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::backup_view::restore_due;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

/// The user's other Device: their Profile "Ada", an Area and a Note.
fn phone() -> Result<Store, Box<dyn std::error::Error>> {
    let phone = store(2)?;
    OwnProfile::new(&phone).apply_defaults("Ada", None)?;
    Places::new(&phone).add_area("Doctor")?;
    Notes::new(&phone).capture("From the phone", None, None)?;
    Ok(phone)
}

/// A fresh computer on its first start, hosting on the loopback, its
/// defaults naming the Profile "Desk".
fn first_start() -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store(1)?, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.set_profile_defaults(Box::new(|store| {
        OwnProfile::new(store).apply_defaults("Desk", None)
    }));
    app.offer_restore();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    Ok(harness)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_label(label).click_accesskit();
    harness.step();
    harness.step();
}

/// Steps the window until `done` holds, at most a few seconds.
fn step_until(harness: &mut Harness<'_, App>, done: impl Fn(&App) -> bool) -> TestResult {
    for _ in 0..400 {
        if done(harness.state()) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

/// Types `code` into the pair code field and clicks "Sync with this code".
fn type_code(harness: &mut Harness<'_, App>, code: &str) {
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .click();
    harness.step();
    harness.get_by(|n| n.is_focused()).type_text(code);
    harness.step();
    harness
        .get_by_label("Sync with this code")
        .click_accesskit();
    harness.step();
}

fn own_name(store: &Store) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.map(|p| p.name))
}

fn texts(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect())
}

/// After the sync no Profile step came: the Profile is the other
/// Device's, its id `theirs` and its name, not one made from this Device's
/// defaults, and nothing asks which one is the user's. No Starter Set
/// came either: only the other Device's Area is here.
fn the_other_devices_profile(harness: &mut Harness<'_, App>, theirs: &str) -> TestResult {
    for _ in 0..3 {
        harness.step();
    }
    let desk = harness.state().store();
    let own = OwnProfile::new(desk).get()?.ok_or("a Profile")?;
    assert_eq!((own.id.as_str(), own.name.as_str()), (theirs, "Ada"));
    assert!(OwnProfile::new(desk).question()?.is_none());
    assert!(!shows(harness, "Which one is you?"));
    assert_ne!(harness.state().selection(), &Selection::Profile);
    let areas: Vec<String> = Places::new(desk)
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(areas, ["Doctor"]);
    Ok(())
}

/// Still on the three choices, nothing answered and no Profile made.
fn untouched(harness: &Harness<'_, App>) -> TestResult {
    let app = harness.state();
    assert!(app.welcoming());
    assert!(!app.sync_host().hosting());
    assert!(shows(harness, "New to Old Brain Fix"));
    assert!(shows(harness, "Add a Device of mine"));
    assert!(shows(harness, "Restore from a Backup"));
    assert!(restore_due(app.store())?);
    assert!(own_name(app.store())?.is_none());
    Ok(())
}

#[test]
fn the_three_choices_show_and_no_profile_is_made_before_one() -> TestResult {
    let harness = first_start()?;
    assert!(shows(&harness, "Welcome to Old Brain Fix"));
    untouched(&harness)
}

#[test]
fn a_device_of_mine_joined_by_its_code_brings_everything_and_skips_the_setup() -> TestResult {
    let phone = Arc::new(phone()?);
    let theirs = OwnProfile::new(&phone).get()?.ok_or("profile")?.id;
    let identity = phone.tls_identity()?;
    let host = Host::start(&identity, "246810", Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
    let code = host.pair_code(true);
    let stop = Arc::new(AtomicBool::new(false));
    let served = {
        let (phone, stop) = (phone.clone(), stop.clone());
        std::thread::spawn(move || -> Result<(), String> {
            while !stop.load(Ordering::Relaxed) {
                let Some(request) = host.wait_request(Duration::from_millis(20)) else {
                    continue;
                };
                let (response, session) =
                    phone.serve(&request, "Phone").map_err(|e| e.to_string())?;
                request.reply(response);
                if let Some(session) = session {
                    phone.remember_joiner(&session).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        })
    };

    let mut harness = first_start()?;
    click(&mut harness, "Add a Device of mine");
    assert!(harness.state().sync_host().hosting());
    assert!(shows(&harness, "Sync with a Device that shows its code"));
    type_code(&mut harness, &code);
    step_until(&mut harness, |app| !app.welcoming())?;
    harness.step();
    stop.store(true, Ordering::Relaxed);
    served.join().map_err(|_| "host thread")??;

    // Straight to the panes, no setup in between, the host stopped.
    let desk = harness.state().store();
    assert!(!harness.state().sync_host().hosting());
    assert_eq!(texts(desk)?, ["From the phone"]);
    assert_eq!(own_name(desk)?.as_deref(), Some("Ada"));
    let areas: Vec<String> = Places::new(desk)
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(areas, ["Doctor"]);
    assert!(!restore_due(desk)?);
    harness.state_mut().offer_restore();
    assert!(!harness.state().welcoming());
    the_other_devices_profile(&mut harness, &theirs)
}

#[test]
fn a_device_of_mine_joining_this_one_brings_everything_after_continue() -> TestResult {
    let mut harness = first_start()?;
    click(&mut harness, "Add a Device of mine");
    let code = harness
        .state()
        .sync_host()
        .host()
        .ok_or("hosting")?
        .pair_code(true);
    assert!(harness.query_by_label("Continue").is_none());
    let phone = phone()?;
    let theirs = OwnProfile::new(&phone).get()?.ok_or("profile")?.id;
    let joined = std::thread::spawn(move || -> Result<(), String> {
        let fail = |e: &dyn std::fmt::Display| e.to_string();
        let info = decode_pair_code(&code).ok_or("a code")?;
        lan_sync(&phone, &info, "Pixel").map_err(|e| fail(&e))?;
        Ok(())
    });
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    click(&mut harness, "Trust");
    joined.join().map_err(|_| "joiner")??;
    harness.step();
    // The page stays until the user goes on, so the other Device can
    // fetch what it needs; Cancel is gone, as nothing is to undo.
    assert!(harness.state().welcoming());
    assert!(harness.query_by_label("Cancel").is_none());
    click(&mut harness, "Continue");

    let desk = harness.state().store();
    assert!(!harness.state().welcoming());
    assert!(!harness.state().sync_host().hosting());
    assert_eq!(texts(desk)?, ["From the phone"]);
    assert_eq!(own_name(desk)?.as_deref(), Some("Ada"));
    let areas: Vec<String> = Places::new(desk)
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(areas, ["Doctor"]);
    the_other_devices_profile(&mut harness, &theirs)
}

#[test]
fn a_failed_join_returns_to_the_choices_saying_why() -> TestResult {
    let mut harness = first_start()?;
    harness
        .state_mut()
        .set_joiner(Arc::new(|_: &Store, _: &PairInfo, _: &str| {
            Err(Error::Sync(SyncFailure::Declined))
        }));
    click(&mut harness, "Add a Device of mine");
    let code = harness
        .state()
        .sync_host()
        .host()
        .ok_or("hosting")?
        .pair_code(true);
    type_code(&mut harness, &code);
    step_until(&mut harness, |app| !app.sync_host().joining())?;
    harness.step();
    harness.step();
    untouched(&harness)?;
    // At the top only that it failed; the click opens why and what to do.
    assert_eq!(
        harness.state().notice(),
        Some("The sync did not go through.")
    );
    assert!(!shows(&harness, "The other Device said no to this one."));
    click(&mut harness, "The sync did not go through.");
    assert!(shows(&harness, "Sync failed"));
    assert!(shows(
        &harness,
        "The other Device said no to this one. Try again, and say yes on the other Device when it asks."
    ));
    assert!(shows(&harness, "sync: Declined"));
    assert!(harness.query_by_label("Report").is_some());
    Ok(())
}

#[test]
fn a_device_refused_returns_to_the_choices() -> TestResult {
    let mut harness = first_start()?;
    click(&mut harness, "Add a Device of mine");
    let code = harness
        .state()
        .sync_host()
        .host()
        .ok_or("hosting")?
        .pair_code(true);
    let joined = std::thread::spawn(move || -> Result<(), String> {
        let phone = phone().map_err(|e| e.to_string())?;
        let info = decode_pair_code(&code).ok_or("a code")?;
        match lan_sync(&phone, &info, "Pixel") {
            Err(Error::Sync(SyncFailure::Declined)) => Ok(()),
            other => Err(format!("{:?}", other.map(|_| ()))),
        }
    });
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    click(&mut harness, "Refuse");
    joined.join().map_err(|_| "joiner")??;
    untouched(&harness)
}

#[test]
fn cancel_and_back_return_to_the_choices() -> TestResult {
    let mut harness = first_start()?;
    click(&mut harness, "Add a Device of mine");
    assert!(harness.state().sync_host().hosting());
    click(&mut harness, "Cancel");
    untouched(&harness)?;

    click(&mut harness, "Add a Device of mine");
    assert!(harness.state().sync_host().hosting());
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.step();
    harness.step();
    untouched(&harness)
}
