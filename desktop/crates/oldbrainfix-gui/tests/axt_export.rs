//! Export as AXT file… in the window (obf-1145, ADR 0056): beside every
//! Export as Pack…, on a card's Share ▸ with the cards selected along, in
//! the Note dialog's Share group, in a Topic's menu and under an Area's
//! Export ▸, in the sidebar and in the band's ⋮; it asks where to save
//! the file, proposed as `<title>.axt` as the phone names it, and writes
//! what it names; a save backed out of writes nothing, a failed one is
//! announced, and a place without Notes says so and asks nothing.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::Modifiers;
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Health with "Water" and its Topic Running with "Laps", the empty Area
/// Garden, and "Buy milk" and "Call the plumber" in Unsorted; the id of
/// "Water".
fn store() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(Utc::now), || Ok(SigningKey::from_seed([3; 32])))?;
    let places = Places::new(&store);
    let health = places.add_area("Health")?;
    let running = places.add_topic(&health.id, "Running")?;
    places.add_area("Garden")?;
    let notes = Notes::new(&store);
    let water = notes.capture("Water", Some(&health.id), None)?;
    notes.capture("Laps", Some(&running.id), None)?;
    notes.capture("Buy milk", None, None)?;
    notes.capture("Call the plumber", None, None)?;
    Ok((Arc::new(store), water.id))
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
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

/// The saves asked for: the dialog's title and the name it offered.
type Asked = Arc<Mutex<Vec<(String, String)>>>;

/// Saves the AXT file into `dir` by the name the dialog offers, or backs
/// out with `None`.
fn saving(app: &mut App, dir: Option<&Path>) -> Asked {
    let asked = Asked::default();
    let (record, dir) = (asked.clone(), dir.map(Path::to_path_buf));
    app.set_axt_saver(Box::new(move |title, name| {
        if let Ok(mut asked) = record.lock() {
            asked.push((title.to_string(), name.to_string()));
        }
        dir.as_ref().map(|dir| dir.join(name))
    }));
    asked
}

/// The last widget labelled `label`: a menu's entry, drawn over what
/// lies under it.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

/// Opens the submenus of `path`, such as "Share ▸ Export as AXT file…",
/// in the menu open, and picks its last entry (obf-1014).
fn pick(harness: &mut Harness<'_, App>, path: &str) {
    let mut parts: Vec<&str> = path.split(" ▸ ").collect();
    let entry = parts.pop().unwrap_or_default();
    for part in parts {
        last(harness, &format!("{part} ⏵")).click();
        harness.run();
    }
    last(harness, entry).click_accesskit();
    harness.run();
}

/// Right-clicks the sidebar's Area or Topic, or the card, `name` and
/// picks `path` in its menu.
fn right_pick(harness: &mut Harness<'_, App>, name: &str, path: &str) {
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, name)
        .click_secondary();
    harness.run();
    pick(harness, path);
}

fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// The texts of the records of the AXT file at `path`, in its order.
fn texts(store: &Store, path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let file = store.axt_read(&bytes, &utc)?.map_err(|_| "no AXT file")?;
    Ok(file
        .records
        .iter()
        .filter_map(|r| r.draft.as_ref().map(|d| d.text.clone()))
        .collect())
}

fn saved(dir: &Path, name: &str) -> PathBuf {
    dir.join(name)
}

#[test]
fn a_cards_share_menu_saves_its_note_as_an_axt_file() -> TestResult {
    let (store, _) = store()?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let asked = saving(&mut app, Some(dir.path()));
    let mut harness = window(app);
    right_pick(&mut harness, "Buy milk", "Share ▸ Export as AXT file…");
    assert_eq!(
        asked.lock().unwrap().as_slice(),
        [("Save the AXT file".to_string(), "Buy milk.axt".to_string())]
    );
    assert_eq!(
        texts(&store, &saved(dir.path(), "Buy milk.axt"))?,
        ["Buy milk"]
    );
    assert!(harness.state().notice().is_none());

    // Its ⋮ offers the same, after Export as Pack….
    last(&harness, "More: Laps").click();
    harness.run();
    last(&harness, "Share ⏵").click();
    harness.run();
    let pack = last(&harness, "Export as Pack…").rect();
    let axt = last(&harness, "Export as AXT file…").rect();
    assert!(axt.top() >= pack.bottom(), "{axt:?} under {pack:?}");
    last(&harness, "Export as AXT file…").click_accesskit();
    harness.run();
    assert_eq!(texts(&store, &saved(dir.path(), "Laps.axt"))?, ["Laps"]);
    Ok(())
}

#[test]
fn the_cards_selected_go_along_into_one_file() -> TestResult {
    let (store, _) = store()?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Unsorted);
    let asked = saving(&mut app, Some(dir.path()));
    let mut harness = window(app);
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, "Buy milk")
        .click();
    harness.run();
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, "Call the plumber")
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert_eq!(harness.state().selected_cards().len(), 2);
    right_pick(&mut harness, "Buy milk", "Share ▸ Export as AXT file…");
    // Several go under the day's name, as on the phone.
    let name = asked.lock().unwrap()[0].1.clone();
    let today = Utc::now().with_timezone(&chrono::Local).date_naive();
    assert_eq!(name, format!("oldbrainfix-notes-{today}.axt"));
    let mut written = texts(&store, &saved(dir.path(), &name))?;
    written.sort();
    assert_eq!(written, ["Buy milk", "Call the plumber"]);
    Ok(())
}

#[test]
fn the_note_dialogs_share_group_saves_its_note() -> TestResult {
    let (store, water) = store()?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let asked = saving(&mut app, Some(dir.path()));
    let mut harness = window(app);
    harness.state_mut().select_note(&water);
    harness.run();
    last(&harness, "Export as AXT file…").click_accesskit();
    harness.run();
    assert_eq!(asked.lock().unwrap()[0].1, "Water.axt");
    assert_eq!(texts(&store, &saved(dir.path(), "Water.axt"))?, ["Water"]);
    Ok(())
}

#[test]
fn an_area_saves_its_notes_and_its_topics_from_its_export_menu() -> TestResult {
    let (store, _) = store()?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let asked = saving(&mut app, Some(dir.path()));
    let mut harness = window(app);
    // The sidebar's right-click, Export as Pack… beside it.
    right_pick(&mut harness, "Health", "Export ▸ Export as AXT file…");
    assert_eq!(asked.lock().unwrap()[0].1, "Health.axt");
    assert_eq!(
        texts(&store, &saved(dir.path(), "Health.axt"))?,
        ["Water", "Laps"]
    );

    // The band's ⋮ of the Area shown.
    std::fs::remove_file(saved(dir.path(), "Health.axt"))?;
    let health = Places::new(&store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Health")
        .ok_or("Health")?;
    harness.state_mut().select(Selection::Place(health.id));
    harness.run();
    last(&harness, "More for this Area").click();
    harness.run();
    last(&harness, "Export ⏵").click();
    harness.run();
    assert!(
        harness
            .query_all_by_label("Export as Pack…")
            .next()
            .is_some()
    );
    last(&harness, "Export as AXT file…").click_accesskit();
    harness.run();
    assert_eq!(
        texts(&store, &saved(dir.path(), "Health.axt"))?,
        ["Water", "Laps"]
    );
    Ok(())
}

#[test]
fn a_topic_saves_its_notes_from_its_menus() -> TestResult {
    let (store, _) = store()?;
    let dir = tempfile::tempdir()?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let asked = saving(&mut app, Some(dir.path()));
    let mut harness = window(app);
    right_pick(&mut harness, "Running", "Export as AXT file…");
    assert_eq!(asked.lock().unwrap()[0].1, "Running.axt");
    assert_eq!(texts(&store, &saved(dir.path(), "Running.axt"))?, ["Laps"]);

    std::fs::remove_file(saved(dir.path(), "Running.axt"))?;
    let running = Places::new(&store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Health")
        .map(|a| Places::new(&store).topics(&a.id))
        .ok_or("Health")??
        .into_iter()
        .next()
        .ok_or("Running")?;
    harness.state_mut().select(Selection::Place(running.id));
    harness.run();
    last(&harness, "More for this Topic").click();
    harness.run();
    last(&harness, "Export as AXT file…").click_accesskit();
    harness.run();
    assert_eq!(texts(&store, &saved(dir.path(), "Running.axt"))?, ["Laps"]);
    Ok(())
}

#[test]
fn a_save_backed_out_of_writes_nothing_and_says_nothing() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let asked = saving(&mut app, None);
    let mut harness = window(app);
    right_pick(&mut harness, "Buy milk", "Share ▸ Export as AXT file…");
    assert_eq!(asked.lock().unwrap().len(), 1);
    assert!(harness.state().notice().is_none());
    Ok(())
}

#[test]
fn a_file_that_cannot_be_written_is_announced() -> TestResult {
    let (store, _) = store()?;
    let dir = tempfile::tempdir()?;
    let missing = dir.path().join("gone");
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    saving(&mut app, Some(&missing));
    let mut harness = window(app);
    right_pick(&mut harness, "Buy milk", "Share ▸ Export as AXT file…");
    assert_eq!(
        harness.state().notice(),
        Some("The AXT file was not saved.")
    );
    assert!(
        harness
            .query_all_by_label_contains("The AXT file was not saved.")
            .next()
            .is_some()
            || harness
                .query_all_by_value("The AXT file was not saved.")
                .next()
                .is_some()
    );
    Ok(())
}

#[test]
fn an_area_without_notes_says_so_and_asks_nothing() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let asked = saving(&mut app, None);
    let mut harness = window(app);
    right_pick(&mut harness, "Garden", "Export ▸ Export as AXT file…");
    assert!(asked.lock().unwrap().is_empty());
    assert_eq!(
        harness.state().notice(),
        Some("There are no Notes here to put in an AXT file.")
    );
    Ok(())
}

#[test]
fn a_files_name_is_the_phones() {
    use oldbrainfix_core::Day;
    use oldbrainfix_gui::axt_view::file_name;
    let day = Day::new(2026, 11, 25);
    assert_eq!(file_name(Some("Buy milk"), day), "Buy milk.axt");
    assert_eq!(file_name(Some("Call: 3pm?\nmore"), day), "Call_ 3pm_.axt");
    assert_eq!(file_name(Some(" ..Notes.. "), day), "Notes.axt");
    assert_eq!(file_name(Some("con"), day), "_con.axt");
    assert_eq!(file_name(Some("LPT1.txt"), day), "_LPT1.txt.axt");
    assert_eq!(file_name(Some("COM10"), day), "COM10.axt");
    assert_eq!(
        file_name(Some(&"a".repeat(70)), day),
        format!("{}.axt", "a".repeat(60))
    );
    assert_eq!(file_name(None, day), "oldbrainfix-notes-2026-11-25.axt");
    assert_eq!(
        file_name(Some(" "), day),
        "oldbrainfix-notes-2026-11-25.axt"
    );
}
