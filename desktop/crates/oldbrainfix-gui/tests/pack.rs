//! Packs in the window (#165, ADR 0043): "Export as Pack…" in the menu
//! of an Area and a Topic, on a card's ⋮ and right-click, in the Note
//! menu and on the Note dialog opens the tree as a dialog, its default
//! ticks and its size as on the phone, and saves the file where the user
//! says; File ▸ Open Pack… and a Pack dropped onto the window open the
//! same tree, all ticked, who it is from with "Add … to People" and
//! "Claim as mine", and "Add" puts the copies into new Areas. A Pack the
//! Dart core made opens, unclaimed Notes say "From Ada" on their card and
//! as a badge in the Note dialog until removed, and a changed Pack, one
//! of a later format and one from a blocked person do not open.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::packs::{self, EXTENSION};
use oldbrainfix_core::schedule::{Recurrence, Schedule};
use oldbrainfix_core::{ClockTime, Day, Notes, OwnProfile, Places, Routines, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store of its own Profile `name`.
fn store(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    store.set_local_setting("author", name)?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// Desk's Health: "Water" with a daily Routine and a Reminder a year
/// ahead, and in its Topic Running "Laps". The ids of Health, Running
/// and "Water".
fn health(store: &Store) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let places = Places::new(store);
    let health = places.add_area("Health")?;
    let running = places.add_topic(&health.id, "Running")?;
    let notes = Notes::new(store);
    let water = notes.capture("Water", Some(&health.id), None)?;
    notes.capture("Laps", Some(&running.id), None)?;
    let at: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let daily = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let routines = Routines::new(store);
    let start = Day::new(2026, 1, 1);
    routines.spawn(
        &water.id,
        &Schedule::new(start, &at, Some(daily)).ok_or("daily")?,
    )?;
    let ahead = Day::new(2099, 6, 1);
    routines.spawn(&water.id, &Schedule::new(ahead, &at, None).ok_or("once")?)?;
    Ok((health.id, running.id, water.id))
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1200.0))
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

/// The last widget labelled `label`: a menu's entry, drawn over what
/// lies under it.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    last(harness, label).click_accesskit();
    harness.run();
}

/// Opens the menu of the sidebar's Area or Topic `name` and picks `entry`.
fn place_menu(harness: &mut Harness<'_, App>, name: &str, entry: &str) {
    harness
        .get_by_role_and_label(Role::Button, name)
        .click_secondary();
    harness.run();
    click(harness, entry);
}

/// The tree's box `label`.
fn tick<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    harness.get_by_role_and_label(Role::CheckBox, label)
}

fn ticked(harness: &Harness<'_, App>, label: &str) -> bool {
    tick(harness, label).accesskit_node().toggled() == Some(Toggled::True)
}

fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    !tick(harness, label).accesskit_node().is_disabled()
}

/// The label of the tree's box that starts with `prefix`.
fn starting(harness: &Harness<'_, App>, prefix: &str) -> Result<String, String> {
    harness
        .query_all_by_role(Role::CheckBox)
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| l.starts_with(prefix))
        .ok_or(format!("no {prefix}"))
}

/// The saves asked for: the dialog's title and the name it offered.
type Asked = Arc<Mutex<Vec<(String, String)>>>;

/// Saves the Pack under `dir` by the name the dialog offers.
fn saving(app: &mut App, dir: &Path) -> Asked {
    let asked = Asked::default();
    let (record, dir) = (asked.clone(), dir.to_path_buf());
    app.set_pack_saver(Box::new(move |title, name| {
        if let Ok(mut asked) = record.lock() {
            asked.push((title.to_string(), name.to_string()));
        }
        Some(dir.join(name))
    }));
    asked
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/pack")
        .join(format!("{name}.{EXTENSION}"))
}

/// Drops the file at `path` onto the window.
fn drop_file(harness: &mut Harness<'_, App>, path: &Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

#[test]
fn an_area_exports_as_a_pack_with_the_phones_ticks_and_size_and_saves_it() -> TestResult {
    let desk = store(4, "Desk")?;
    let (health, _, _) = health(&desk)?;
    let pack = desk.pack_of(&health, &HashSet::new())?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(desk, "en");
    let asked = saving(&mut app, dir.path());
    let mut harness = window(app);
    place_menu(&mut harness, "Health", "Export as Pack…");

    // Everything ticked but the Reminder, "Add my Profile" off, the size
    // of what is ticked over the tree.
    assert!(shows(&harness, "Export as Pack"));
    let routine = starting(&harness, "Routine: ")?;
    let reminder = starting(&harness, "Reminder: ")?;
    for note in ["Water", "Laps", routine.as_str()] {
        assert!(ticked(&harness, note), "{note}");
    }
    assert!(!ticked(&harness, &reminder));
    assert!(!ticked(&harness, "Add my Profile"));
    let size = pack.size(&pack.export_ticks());
    assert!(size < 1000, "{size}");
    assert!(shows(&harness, &format!("Size: {size} B")));

    // A Note unticked takes its items along and they wait greyed.
    tick(&harness, "Water").click();
    harness.run();
    assert!(!ticked(&harness, "Water"));
    assert!(!enabled(&harness, &routine));
    assert!(!enabled(&harness, &reminder));
    tick(&harness, "Water").click();
    harness.run();
    assert!(enabled(&harness, &routine));

    // Untick all leaves nothing to save; Tick all ticks the Reminder too.
    click(&mut harness, "Untick all");
    assert!(shows(&harness, "Tick at least one Note."));
    assert!(last(&harness, "Save…").accesskit_node().is_disabled());
    assert!(shows(&harness, "Size: 0 B"));
    click(&mut harness, "Tick all");
    assert!(ticked(&harness, &reminder));
    assert!(!shows(&harness, "Tick at least one Note."));

    click(&mut harness, "Save…");
    let day = Day::today(chrono::Utc::now(), &oldbrainfix_gui::app::local_wall);
    let name = packs::file_name(day);
    assert_eq!(
        asked.lock().map_err(|e| e.to_string())?.as_slice(),
        [("Save the Pack to send".to_string(), name.clone())]
    );
    // Saved, the dialog closes and nothing is said.
    assert!(!shows(&harness, "Export as Pack"));
    assert_eq!(harness.state().notice(), None);
    let bytes = std::fs::read(dir.path().join(&name))?;
    let ben = store(5, "Ben")?;
    let opened = ben.open_pack(&bytes)?.map_err(|e| e.name())?;
    let texts: Vec<&str> = opened.pack.notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(texts, ["Water", "Laps"]);
    assert_eq!(opened.pack.notes[0].routines.len(), 2);
    assert!(opened.from.is_none());
    Ok(())
}

#[test]
fn a_topic_a_card_the_note_menu_and_the_note_dialog_export_what_they_hold() -> TestResult {
    let desk = store(4, "Desk")?;
    let (_, _, water) = health(&desk)?;
    let mut app = App::new(desk, "en");
    app.select(Selection::All);
    let mut harness = window(app);

    // From a Topic: its Notes.
    place_menu(&mut harness, "Running", "Export as Pack…");
    assert!(ticked(&harness, "Laps"));
    assert!(
        harness
            .query_by_role_and_label(Role::CheckBox, "Water")
            .is_none()
    );
    click(&mut harness, "Cancel");
    assert!(!shows(&harness, "Export as Pack"));

    // From a card's ⋮ and its right-click: that Note.
    click(&mut harness, "More: Water");
    click(&mut harness, "Export as Pack…");
    assert!(ticked(&harness, "Water"));
    assert!(
        harness
            .query_by_role_and_label(Role::CheckBox, "Laps")
            .is_none()
    );
    click(&mut harness, "Cancel");
    harness
        .get_by_role_and_label(Role::Button, "Laps")
        .click_secondary();
    harness.run();
    click(&mut harness, "Export as Pack…");
    assert!(ticked(&harness, "Laps"));
    click(&mut harness, "Cancel");

    // From the Note dialog's bar and the Note menu.
    harness.state_mut().select_note(&water);
    harness.run();
    click(&mut harness, "Export as Pack…");
    assert!(ticked(&harness, "Water"));
    click(&mut harness, "Cancel");
    harness
        .get_all_by_label("Note")
        .next()
        .ok_or("Note")?
        .click();
    harness.run();
    click(&mut harness, "Export as Pack…");
    assert!(ticked(&harness, "Water"));
    assert!(
        harness
            .query_by_role_and_label(Role::CheckBox, "Laps")
            .is_none()
    );
    Ok(())
}

#[test]
fn a_pack_the_dart_core_made_opens_from_the_file_menu_and_adds_its_notes() -> TestResult {
    let ben = store(5, "Ben")?;
    let bytes = std::fs::read(fixture("signed"))?;
    let expected = ben.open_pack(&bytes)?.map_err(|e| e.name())?;
    let count = expected.pack.picked(&expected.pack.all_keys()).notes.len();
    let mut app = App::new(ben, "en");
    let path = fixture("signed");
    app.set_pack_opener(Box::new(move |title| {
        assert_eq!(title, "Open a Pack");
        Some(path.clone())
    }));
    let mut harness = window(app);
    harness
        .get_all_by_label("File")
        .next()
        .ok_or("File")?
        .click();
    harness.run();
    click(&mut harness, "Open Pack…");

    // Who it is from, everything ticked, Claim as mine ticked.
    assert!(shows(&harness, "From Ada"));
    assert!(shows(&harness, "Add Ada to People"));
    for note in ["Buy shoes", "Fix the bike", "Training", "Digging"] {
        assert!(ticked(&harness, note), "{note}");
    }
    assert!(ticked(&harness, "Claim as mine"));
    click(&mut harness, "Add");
    assert_eq!(
        harness.state().notice(),
        Some(format!("{count} Notes added").as_str())
    );
    let store = harness.state().store();
    let areas: Vec<String> = Places::new(store)
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert!(areas.contains(&"Health".to_string()), "{areas:?}");
    assert!(areas.contains(&"Garden".to_string()), "{areas:?}");
    // Claimed: no Note says whom it came from.
    for note in Notes::new(store).all()? {
        assert_eq!(store.pack_from(&note.id)?, None, "{}", note.text);
    }
    Ok(())
}

#[test]
fn a_pack_without_a_profile_dropped_onto_the_window_asks_no_claim() -> TestResult {
    let mut harness = window(App::new(store(5, "Ben")?, "en"));
    drop_file(&mut harness, &fixture("pack"));
    assert!(ticked(&harness, "Buy shoes"));
    assert!(!shows(&harness, "From "));
    assert!(
        harness
            .query_by_role_and_label(Role::CheckBox, "Claim as mine")
            .is_none()
    );
    Ok(())
}

#[test]
fn unclaimed_notes_say_from_ada_on_the_card_and_in_the_dialog_until_removed() -> TestResult {
    let mut app = App::new(store(5, "Ben")?, "en");
    app.select(Selection::All);
    let mut harness = window(app);
    drop_file(&mut harness, &fixture("signed"));
    tick(&harness, "Claim as mine").click();
    harness.run();
    assert!(!ticked(&harness, "Claim as mine"));
    click(&mut harness, "Add");
    let note_of = |harness: &Harness<'_, App>, text: &str| -> Result<String, String> {
        Notes::new(harness.state().store())
            .all()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|n| n.text == text)
            .map(|n| n.id)
            .ok_or(format!("no {text}"))
    };
    let shoes = note_of(&harness, "Buy shoes")?;
    let bike = note_of(&harness, "Fix the bike")?;
    assert!(harness.state().store().pack_from(&shoes)?.is_some());
    assert!(shows(&harness, "From Ada"));

    // The card's ⋮ removes it at once.
    click(&mut harness, "More: Buy shoes");
    click(&mut harness, "Remove “From Ada”");
    assert_eq!(harness.state().error(), None);
    assert_eq!(harness.state().store().pack_from(&shoes)?, None);
    assert!(harness.state().store().pack_from(&bike)?.is_some());

    // Ctrl+Z brings it back (#268).
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert!(harness.state().store().pack_from(&shoes)?.is_some());

    // The badge in the Note dialog offers it in its menu, which asks
    // nothing either.
    harness.state_mut().select_note(&bike);
    harness.run();
    click(&mut harness, "Pack: From Ada");
    assert!(harness.state().store().pack_from(&bike)?.is_some());
    click(&mut harness, "Remove “From Ada”");
    assert_eq!(harness.state().store().pack_from(&bike)?, None);
    assert!(!shows(&harness, "Pack: From Ada"));
    Ok(())
}

#[test]
fn a_from_line_another_persons_device_wrote_does_not_show() -> TestResult {
    let ben = store(5, "Ben")?;
    let signed = std::fs::read(fixture("signed"))?;
    let opened = ben.open_pack(&signed)?.map_err(|e| e.name())?;
    let wall = |at: chrono::DateTime<chrono::Utc>| at.naive_utc();
    ben.add_pack(
        &opened,
        &opened.pack.all_keys(),
        false,
        &wall,
        "Pack",
        "From a Pack",
    )?;
    let notes = Notes::new(&ben);
    let shoes = notes
        .all()?
        .into_iter()
        .find(|n| n.text == "Buy shoes")
        .ok_or("Buy shoes")?
        .id;
    let milk = notes.capture("Milk", None, None)?.id;
    // As a modified app in a Shared Area would (Q554 (a)): Cy's Device
    // writes the line on a badged Note and on Ben's own, later.
    let cy = store(6, "Cy")?;
    let from = r#"{"id":"cy","name":"Cy"}"#;
    let later = Some(chrono::Utc::now() + chrono::Duration::days(1));
    for note in [&shoes, &milk] {
        ben.insert(
            &cy.append(note, packs::keys::FROM, Some(from), later)?
                .wire(),
        )?;
    }
    let mut app = App::new(ben, "en");
    app.select(Selection::All);
    let harness = window(app);
    assert!(shows(&harness, "Milk"));
    assert!(shows(&harness, "From Ada"));
    assert!(!shows(&harness, "From Cy"));
    Ok(())
}

#[test]
fn add_ada_to_people_keeps_her_and_the_pack_open() -> TestResult {
    let mut harness = window(App::new(store(5, "Ben")?, "en"));
    drop_file(&mut harness, &fixture("signed"));
    click(&mut harness, "Add Ada to People");
    assert_eq!(harness.state().notice(), Some("Ada was added to People."));
    assert!(!shows(&harness, "Add Ada to People"));
    assert!(ticked(&harness, "Claim as mine"));
    let kept = harness.state().store().kept_people()?;
    assert!(kept.iter().any(|p| p.name == "Ada"), "{kept:?}");
    Ok(())
}

/// Drops `name` and opens why it did not open: `cause`.
fn refused(harness: &mut Harness<'_, App>, name: &str, cause: &str) {
    harness.state_mut().close_page();
    drop_file(harness, &fixture(name));
    assert_eq!(harness.state().notice(), Some("The Pack was not opened."));
    assert!(!shows(harness, "Claim as mine"));
    harness.get_by_label("The Pack was not opened.").click();
    harness.run();
    assert!(shows(harness, "Pack not opened"));
    assert!(shows(harness, cause), "{cause}");
    click(harness, "Close");
}

#[test]
fn a_changed_pack_a_later_one_and_one_from_a_blocked_person_do_not_open() -> TestResult {
    let ben = store(5, "Ben")?;
    // Ada's card, as her Pack carries it, and Ada blocked.
    let opened = ben
        .open_pack(&std::fs::read(fixture("signed"))?)?
        .map_err(|e| e.name())?;
    let card = opened.pack.card.clone().ok_or("her card")?;
    ben.open_profile_card(&card, false)?;
    let ada = opened.from.ok_or("from Ada")?.id;
    let mut harness = window(App::new(ben, "en"));
    refused(
        &mut harness,
        "changed",
        "This file is no Pack, or it was changed after it was made.",
    );
    refused(
        &mut harness,
        "later",
        "This file comes from a newer Old Brain Fix.",
    );
    harness.state().store().block_person(&ada, false)?;
    refused(
        &mut harness,
        "signed",
        "This Pack is from a person you blocked.",
    );
    Ok(())
}

/// Hands the window's threads a connection to `store` once `held` is
/// let go: while the test holds it, a Pack's work waits there.
fn gated(app: &mut App, store: &Arc<Store>) -> Arc<Mutex<()>> {
    let held = Arc::new(Mutex::new(()));
    let (gate, store) = (held.clone(), store.clone());
    app.set_agenda_store(move || {
        drop(gate.lock());
        store.clone()
    });
    held
}

/// Steps the window until `done`, up to half a minute.
fn until(harness: &mut Harness<'_, App>, done: impl Fn(&Harness<'_, App>) -> bool) -> TestResult {
    for _ in 0..6000 {
        if done(harness) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Err("never done".into())
}

#[test]
fn save_writes_the_pack_on_a_thread_of_its_own_greyed_meanwhile() -> TestResult {
    let desk = store(4, "Desk")?;
    health(&desk)?;
    let desk = Arc::new(desk);
    let dir = tempfile::tempdir()?;
    let mut app = App::new(desk.clone(), "en");
    app.select(Selection::All);
    let held = gated(&mut app, &desk);
    saving(&mut app, dir.path());
    let mut harness = window(app);
    place_menu(&mut harness, "Health", "Export as Pack…");
    let guard = held.lock().map_err(|e| e.to_string())?;
    // Stepped, not run: the window looks again while the thread runs.
    last(&harness, "Save…").click_accesskit();
    // The window draws on, Save… greyed, the dialog open, nothing saved.
    for _ in 0..3 {
        harness.step();
    }
    assert!(shows(&harness, "Export as Pack"));
    assert!(last(&harness, "Save…").accesskit_node().is_disabled());
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 0);
    drop(guard);
    until(&mut harness, |h| !shows(h, "Export as Pack"))?;
    let saved = std::fs::read_dir(dir.path())?
        .next()
        .ok_or("a Pack saved")??;
    let opened = store(5, "Ben")?
        .open_pack(&std::fs::read(saved.path())?)?
        .map_err(|e| e.name())?;
    let texts: Vec<&str> = opened.pack.notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(texts, ["Water", "Laps"]);
    Ok(())
}

#[test]
fn a_large_pack_is_read_and_opened_on_a_thread_the_line_showing_meanwhile() -> TestResult {
    let ada = store(6, "Ada")?;
    let (health, _, water) = health(&ada)?;
    Notes::new(&ada).add_photo(&water, &vec![7u8; (1 << 20) + 1000])?;
    let pack = ada.pack_of(&health, &HashSet::new())?;
    let bytes = ada.write_pack(&pack, &pack.all_keys(), false)?;
    assert!(bytes.len() > 1 << 20);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(format!("big.{EXTENSION}"));
    std::fs::write(&path, bytes)?;
    let ben = Arc::new(store(5, "Ben")?);
    let mut app = App::new(ben.clone(), "en");
    app.select(Selection::All);
    let held = gated(&mut app, &ben);
    let mut harness = window(app);
    let guard = held.lock().map_err(|e| e.to_string())?;
    // Stepped, not run: the line moves while the thread runs.
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.clone()),
        ..Default::default()
    }];
    for _ in 0..3 {
        harness.step();
    }
    assert!(shows(&harness, "Opening the Pack…"));
    assert!(!shows(&harness, "Claim as mine"));
    assert!(!shows(&harness, "Water"));
    drop(guard);
    until(&mut harness, |h| shows(h, "Water"))?;
    assert!(!shows(&harness, "Opening the Pack…"));
    assert!(ticked(&harness, "Water"));
    assert!(ticked(&harness, "Laps"));
    Ok(())
}

#[test]
fn what_the_ticks_pick_and_the_size_are_worked_out_when_they_change() -> TestResult {
    let desk = store(4, "Desk")?;
    health(&desk)?;
    let mut harness = window(App::new(desk, "en"));
    place_menu(&mut harness, "Health", "Export as Pack…");
    let opened = oldbrainfix_gui::pack_view::worked_out();
    // Frames that change no tick work out nothing again.
    for _ in 0..5 {
        harness.step();
    }
    last(&harness, "Add my Profile").click();
    harness.run();
    assert_eq!(oldbrainfix_gui::pack_view::worked_out(), opened);
    tick(&harness, "Water").click();
    harness.run();
    assert_eq!(oldbrainfix_gui::pack_view::worked_out(), opened + 1);
    assert!(!ticked(&harness, "Water"));
    Ok(())
}
