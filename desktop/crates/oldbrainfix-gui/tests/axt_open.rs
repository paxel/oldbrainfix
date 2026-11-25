//! An AXT file opened in the window (obf-1146, ADR 0056): dropped onto
//! it, picked with File ▸ Open ▸ AXT file… or handed over from outside,
//! it shows its records in a dialog, every one it can take ticked, those
//! it cannot named and greyed, who sent it, and where they go, Unsorted
//! first; "Take it" makes them Notes there, Cancel takes nothing, and a
//! file that is no AXT file is announced.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::schedule::Schedule;
use oldbrainfix_core::{ClockTime, Day, Notes, OwnProfile, Places, Routines, SigningKey, Store};
use oldbrainfix_gui::App;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    store.set_local_setting("author", name)?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// Desk, with the Area Health and its Topic Running; the id of Health.
fn desk() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let desk = store(4, "Desk")?;
    let places = Places::new(&desk);
    let health = places.add_area("Health")?;
    places.add_topic(&health.id, "Running")?;
    Ok((Arc::new(desk), health.id))
}

/// Ben's AXT file in `dir`: "Buy milk", and "Call the plumber" with a
/// Reminder.
fn bens_file(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let ben = store(5, "Ben")?;
    let notes = Notes::new(&ben);
    let milk = notes.capture("Buy milk", None, None)?;
    let plumber = notes.capture("Call the plumber", None, None)?;
    let at: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let once = Schedule::new(Day::new(2099, 6, 1), &at, None).ok_or("once")?;
    Routines::new(&ben).spawn(&plumber.id, &once)?;
    let path = dir.join("from-ben.axt");
    std::fs::write(&path, ben.axt_write(&[milk.id, plumber.id])?)?;
    Ok(path)
}

/// The core's file of another app, cat-a-log: an Appointment, a Task and
/// a record of a type Old Brain Fix does not take, `org.example.Memo`.
fn catalog() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../oldbrainfix-core/tests/axt/catalog.axt")
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1100.0))
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

fn shows(harness: &Harness<'_, App>, text: &str) -> bool {
    harness.query_all_by_label_contains(text).next().is_some()
        || harness
            .query_all_by(|n| n.value().is_some_and(|v| v.contains(text)))
            .next()
            .is_some()
}

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

fn drop_file(harness: &mut Harness<'_, App>, path: &Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

fn record<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    harness.get_by_role_and_label(Role::CheckBox, label)
}

fn ticked(harness: &Harness<'_, App>, label: &str) -> bool {
    record(harness, label).accesskit_node().toggled() == Some(Toggled::True)
}

fn greyed(harness: &Harness<'_, App>, label: &str) -> bool {
    last(harness, label).accesskit_node().is_disabled()
}

fn texts_in(store: &Store, place: Option<&str>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .filter(|n| n.place.as_deref() == place)
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(texts)
}

#[test]
fn a_file_dropped_shows_its_records_ticked_and_takes_them_into_unsorted() -> TestResult {
    let (desk, _) = desk()?;
    let dir = tempfile::tempdir()?;
    let path = bens_file(dir.path())?;
    let mut harness = window(App::new(desk.clone(), "en"));
    drop_file(&mut harness, &path);

    assert!(shows(&harness, "AXT file"));
    // The app its manifest names, the key to compare under it.
    assert!(shows(&harness, "From Old Brain Fix"));
    assert!(shows(&harness, "Key "));
    assert!(!shows(&harness, "From your own Device"));
    assert!(ticked(&harness, "Buy milk"));
    assert!(ticked(&harness, "Call the plumber"));
    // Its type under each, a Reminder with it.
    assert!(shows(&harness, "Note"));
    assert!(shows(&harness, "Task · "));
    assert!(!shows(&harness, "Not taken"));
    // Unsorted first and picked; then the Areas and Topics.
    let unsorted = harness.get_by_role_and_label(Role::RadioButton, "Unsorted");
    assert_eq!(unsorted.accesskit_node().toggled(), Some(Toggled::True));
    assert!(
        harness
            .query_by_role_and_label(Role::RadioButton, "Health / Running")
            .is_some()
    );

    click(&mut harness, "Take it");
    assert!(!shows(&harness, "Take it"), "the dialog closed");
    assert_eq!(harness.state().notice(), Some("2 Notes taken"));
    assert_eq!(texts_in(&desk, None)?, ["Buy milk", "Call the plumber"]);
    Ok(())
}

#[test]
fn file_open_takes_the_ticked_records_into_the_place_picked() -> TestResult {
    let (desk, health) = desk()?;
    let dir = tempfile::tempdir()?;
    let path = bens_file(dir.path())?;
    let mut app = App::new(desk.clone(), "en");
    let asked = Arc::new(Mutex::new(Vec::new()));
    let record_asked = asked.clone();
    app.set_axt_opener(Box::new(move |title| {
        record_asked.lock().unwrap().push(title.to_string());
        Some(path.clone())
    }));
    let mut harness = window(app);
    last(&harness, "File").click();
    harness.run();
    last(&harness, "Open ⏵").click();
    harness.run();
    click(&mut harness, "AXT file…");
    assert_eq!(asked.lock().unwrap().as_slice(), ["Open an AXT file"]);

    record(&harness, "Call the plumber").click();
    harness.run();
    assert!(!ticked(&harness, "Call the plumber"));
    harness
        .get_by_role_and_label(Role::RadioButton, "Health")
        .click();
    harness.run();
    click(&mut harness, "Take it");
    assert_eq!(harness.state().notice(), Some("1 Note taken"));
    assert_eq!(texts_in(&desk, Some(&health))?, ["Buy milk"]);
    assert!(texts_in(&desk, None)?.is_empty());
    Ok(())
}

#[test]
fn another_apps_file_names_and_greys_what_it_does_not_take() -> TestResult {
    let (desk, _) = desk()?;
    let mut harness = window(App::new(desk.clone(), "en"));
    drop_file(&mut harness, &catalog());
    assert!(shows(&harness, "From cat-a-log"));
    assert!(ticked(&harness, "Vet: Mia"));
    assert!(ticked(&harness, "Order food"));
    assert!(shows(&harness, "Not taken"));
    assert!(!ticked(&harness, "org.example.Memo"));
    assert!(greyed(&harness, "org.example.Memo"));
    // Untick all leaves the Memo as it is and greys Take it.
    click(&mut harness, "Untick all");
    assert!(greyed(&harness, "Take it"));
    assert!(shows(&harness, "Tick at least one Note."));
    click(&mut harness, "Tick all");
    assert!(!greyed(&harness, "Take it"));
    click(&mut harness, "Take it");
    assert_eq!(harness.state().notice(), Some("2 Notes taken"));
    let texts = texts_in(&desk, None)?;
    assert_eq!(texts.len(), 2, "{texts:?}");
    assert!(texts.iter().all(|t| !t.is_empty()));
    Ok(())
}

#[test]
fn a_file_of_this_device_says_so_and_cancel_takes_nothing() -> TestResult {
    let (desk, _) = desk()?;
    let note = Notes::new(&desk).capture("Water the plants", None, None)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("mine.axt");
    std::fs::write(&path, desk.axt_write(&[note.id])?)?;
    let mut app = App::new(desk.clone(), "en");
    let handed = Arc::new(Mutex::new(vec![path]));
    app.set_handed_files(handed);
    let mut harness = window(app);
    harness.run();
    assert!(shows(&harness, "From your own Device"));
    assert!(!shows(&harness, "From Old Brain Fix"));
    assert!(ticked(&harness, "Water the plants"));
    click(&mut harness, "Cancel");
    assert!(!shows(&harness, "Take it"));
    assert_eq!(texts_in(&desk, None)?, ["Water the plants"]);
    assert!(harness.state().notice().is_none());
    Ok(())
}

#[test]
fn a_file_that_is_no_axt_file_is_announced() -> TestResult {
    let (desk, _) = desk()?;
    let dir = tempfile::tempdir()?;
    // Named as one, and starting as one by its content.
    let named = dir.path().join("broken.axt");
    std::fs::write(&named, b"not a file of records")?;
    let started = dir.path().join("records.bin");
    std::fs::write(&started, b"Obj\x01 and then nothing")?;
    let mut harness = window(App::new(desk, "en"));
    for path in [named, started] {
        drop_file(&mut harness, &path);
        assert_eq!(
            harness.state().notice(),
            Some("The AXT file was not opened."),
            "{path:?}"
        );
        assert!(!shows(&harness, "Take it"));
    }
    Ok(())
}
