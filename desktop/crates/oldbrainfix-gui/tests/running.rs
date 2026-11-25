//! The page Running (obf-1078): View ▸ Running and a click on the line
//! under the menu bar open it; each job running beside the window shows
//! there with what it does, how far it is, since when and Abort. Abort
//! stops a sync, an Import, deleting an Area, the speech model's download
//! and the reading of Photo Texts at their next safe point; what was done
//! stays, and the page says how far it came. Driven headless by
//! egui_kittest.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::auto_sync::Syncing;
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::speech::{Download, SpeechModel, Status};
use oldbrainfix_gui::{App, Page, Selection};
use oldbrainfix_whisper::model::{ModelDir, Outcome, WhisperModel};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn open(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

fn mem() -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
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
    harness.step();
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The label of the first node whose label holds `part`.
fn label_with(harness: &Harness<'_, App>, part: &str) -> Option<String> {
    harness
        .query_all_by_label_contains(part)
        .next()
        .and_then(|n| {
            let node = n.accesskit_node();
            node.label().or_else(|| node.value())
        })
}

/// Whether a label holding `count`, "120 of 340 Notes", says that more
/// than none are done.
fn under_way(harness: &Harness<'_, App>, count: &str) -> bool {
    harness.query_all_by_label_contains(count).any(|n| {
        let node = n.accesskit_node();
        let text = node.label().or_else(|| node.value()).unwrap_or_default();
        let done = text
            .split(" of ")
            .next()
            .and_then(|before| before.rsplit(' ').next())
            .and_then(|n| n.parse::<usize>().ok());
        done.is_some_and(|n| n > 0)
    })
}

fn step_until(
    harness: &mut Harness<'_, App>,
    done: impl Fn(&Harness<'_, App>) -> bool,
) -> TestResult {
    for _ in 0..1000 {
        if done(harness) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

/// Waits until `done` holds, for at most ten seconds.
fn eventually(done: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(10);
    while !done() {
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

/// Opens View ▸ Running.
fn view_running(harness: &mut Harness<'_, App>) {
    harness.get_by_label("View").click();
    harness.step();
    harness
        .get_all_by_label("Running")
        .last()
        .expect("View ▸ Running")
        .click_accesskit();
    harness.step();
    harness.step();
}

#[test]
fn view_running_opens_the_page_which_says_when_nothing_runs() -> TestResult {
    let mut harness = window(App::new(mem()?, "en"));
    view_running(&mut harness);
    assert_eq!(harness.state().page(), Some(Page::Running));
    assert!(shows(&harness, "Nothing runs in the background."));
    // Nothing runs: no line to click either.
    assert!(harness.query_by_label("Syncing…").is_none());
    Ok(())
}

#[test]
fn a_click_on_the_line_shows_the_sync_whose_abort_pauses_it_until_go_on() -> TestResult {
    let mut app = App::new(mem()?, "en");
    let syncing = Syncing::new(|| {});
    app.set_syncing(syncing.clone());
    let mut harness = window(app);
    // A sync of this Device running, until let go.
    let (go, held) = channel::<()>();
    let running = syncing.clone();
    let sync = std::thread::spawn(move || running.during(|| held.recv()));
    assert!(eventually(|| syncing.running()));
    harness.step();

    // The menus above the line still take their clicks.
    view_running(&mut harness);
    assert!(shows(&harness, "Sync with a Device"));
    harness.state_mut().close_page();
    harness.step();
    harness.step();
    assert_eq!(harness.state().page(), None);

    harness.get_by_label("Syncing…").click();
    harness.step();
    harness.step();
    assert_eq!(harness.state().page(), Some(Page::Running));
    assert!(shows(&harness, "Sync with a Device"));
    assert!(shows(&harness, "Since "), "since when");
    assert!(shows(&harness, "The sync under way ends first."));

    harness.get_by_label("Abort").click_accesskit();
    harness.step();
    harness.step();
    assert!(syncing.paused());
    assert!(shows(&harness, "Stopping…"), "the round under way ends");
    go.send(())?;
    let _ = sync.join();
    harness.step();
    harness.step();
    assert!(!shows(&harness, "Stopping…"));
    assert!(shows(&harness, "Nothing runs in the background."));
    assert!(shows(
        &harness,
        "Paused: this Device syncs only by Sync now"
    ));

    harness.get_by_label("Go on").click_accesskit();
    harness.step();
    harness.step();
    assert!(!syncing.paused());
    assert!(!shows(&harness, "Paused:"));

    // Sync now ends a pause too.
    syncing.pause();
    harness.state_mut().close_page();
    harness.step();
    harness.get_by_label("Sync").click();
    harness.step();
    harness.get_by_label("Sync now").click_accesskit();
    harness.step();
    assert!(!syncing.paused());
    Ok(())
}

#[test]
fn an_import_aborted_keeps_the_notes_it_made_and_says_how_many() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Notes");
    std::fs::create_dir_all(&dir)?;
    // Enough that the Import, resting after each Note, outlasts many
    // frames.
    for n in 0..1000 {
        std::fs::write(dir.join(format!("Note {n:04}.txt")), format!("Text {n}"))?;
    }
    let path = root.path().join("log.db");
    let mut app = App::new(open(&path)?, "en");
    app.set_import_picker(Box::new(move |_| Some(dir.clone())));
    let own = Arc::new(open(&path)?);
    app.set_import_store(move || own.clone());
    let mut harness = window(app);

    harness.get_by_label("File").click();
    harness.step();
    harness.get_by_label("Import…").click();
    harness.step();
    harness.step();
    harness.get_by_label("A folder…").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "You can close this dialog."));
    harness.get_by_label("Close").click_accesskit();
    harness.step();
    harness.step();
    assert!(!shows(&harness, "You can close this dialog."));

    harness.get_by_label("Importing…").click();
    harness.step();
    harness.step();
    assert_eq!(harness.state().page(), Some(Page::Running));
    assert!(shows(&harness, "Import of Notes"));
    // Once a Note is in: before the first, "0 of 1000 Notes" is told.
    step_until(&mut harness, |h| under_way(h, "of 1000 Notes"))?;
    assert!(shows(&harness, "The Notes imported so far stay."));
    harness.get_by_label("Abort").click_accesskit();
    harness.step();
    step_until(&mut harness, |h| h.state().notice().is_some())?;
    assert_eq!(harness.state().notice(), Some("Import stopped."));
    harness.step();

    let said = label_with(&harness, "Stopped: ").ok_or("the page says so")?;
    let check = open(&path)?;
    let area = Places::new(&check)
        .areas()?
        .into_iter()
        .find(|a| a.name.contains("Notes"))
        .ok_or("the new Area")?;
    let made = Notes::new(&check).inside(&area.id)?.len();
    assert!(made > 0 && made < 1000, "stopped on the way: {made}");
    assert_eq!(said, format!("Stopped: {made} of 1000 imported"));
    assert!(!harness.state().importing());
    Ok(())
}

/// The Area Household on the database at `path`, with `count` Notes.
fn plant(path: &Path, count: usize) -> Result<String, Box<dyn std::error::Error>> {
    let store = open(path)?;
    let household = Places::new(&store).add_area("Household")?;
    let notes = Notes::new(&store);
    store.batch(|| {
        for n in 0..count {
            notes.capture(&format!("Chore {n}"), Some(&household.id), None)?;
        }
        Ok(())
    })?;
    Ok(household.id)
}

#[test]
fn deleting_an_area_aborted_keeps_the_area_with_the_notes_not_gone() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("log.db");
    let area = plant(&path, 1000)?;
    let mut app = App::new(open(&path)?, "en");
    let own = Arc::new(open(&path)?);
    app.set_import_store(move || own.clone());
    app.select(Selection::Place(area.clone()));
    let mut harness = window(app);

    harness
        .get_all_by_label("More for this Area")
        .next()
        .expect("the Area's ⋮")
        .click();
    harness.step();
    harness.step();
    harness
        .get_all_by_label("Delete")
        .last()
        .expect("Delete")
        .click_accesskit();
    harness.step();
    harness.step();
    harness.get_by_label("Move to trash").click_accesskit();
    harness.step();
    harness.step();
    assert!(harness.state().deleting_area());

    harness.get_by_label("Deleting an Area…").click();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Deleting the Area Household"));
    step_until(&mut harness, |h| under_way(h, "of 1000 Notes moved"))?;
    harness.get_by_label("Abort").click_accesskit();
    harness.step();
    step_until(&mut harness, |h| !h.state().deleting_area())?;
    harness.step();

    let check = open(&path)?;
    assert!(
        Places::new(&check).area_by_id(&area)?.is_some(),
        "the Area stays"
    );
    let left = Notes::new(&check).inside(&area)?.len();
    assert!(left > 0 && left < 1000, "stopped on the way: {left}");
    let said = label_with(&harness, "Stopped: ").ok_or("the page says so")?;
    assert_eq!(
        said,
        format!("Stopped: {} of 1000 Notes moved", 1000 - left)
    );
    Ok(())
}

#[test]
fn the_speech_models_download_aborted_stops_and_is_offered_again() -> TestResult {
    let tmp = tempfile::tempdir()?;
    // Downloads until told to stop.
    let slow: Download = Arc::new(|_, _, stop: &AtomicBool, progress| {
        progress(1_000_000, Some(140_000_000));
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(5));
        }
        Outcome::Cancelled
    });
    let speech = SpeechModel::with_download(
        ModelDir::new(tmp.path().join("whisper")),
        slow,
        Arc::new(|| {}),
    );
    assert!(eventually(|| speech.status() == Status::Missing));
    let mut app = App::new(mem()?, "en");
    app.set_enrichers(None, None, Some(speech.clone()));
    speech.download(WhisperModel::Base);
    let mut harness = window(app);
    harness.state_mut().open(Page::Running);
    step_until(&mut harness, |h| shows(h, "Downloading: 1 of 140 MB"))?;
    assert!(shows(&harness, "Downloading the speech model"));

    harness.get_by_label("Abort").click_accesskit();
    harness.step();
    assert!(eventually(|| speech.status() == Status::Missing));
    assert_eq!(speech.outcome(), Some(Outcome::Cancelled));
    harness.step();
    assert!(shows(&harness, "Nothing runs in the background."));
    assert!(shows(
        &harness,
        "The Settings offer the speech model again."
    ));
    Ok(())
}

/// Reads each photo only once the test lets it.
struct Gated(Receiver<()>);

impl Engine for Gated {
    fn read(&mut self, _bytes: Vec<u8>) -> Option<String> {
        self.0.recv().ok().map(|()| "read".to_string())
    }
}

#[test]
fn reading_photo_texts_aborted_ends_the_one_read_and_goes_on_when_asked() -> TestResult {
    let store = Arc::new(mem()?);
    let notes = Notes::new(&store);
    for n in 0..3u8 {
        let note = notes.capture(&format!("Receipt {n}"), None, None)?;
        notes.add_photo(&note.id, &[n; 16])?;
    }
    let (read, gate) = channel::<()>();
    let reader = Enricher::spawn(
        store.clone(),
        Kind::PhotoTexts,
        Box::new(Gated(gate)),
        Box::new(|| {}),
    );
    let mut app = App::new(store.clone(), "en");
    app.set_enrichers(None, Some(reader.clone()), None);
    assert!(eventually(|| reader.current().is_some()));
    let mut harness = window(app);
    harness.state_mut().open(Page::Running);
    harness.step();
    assert!(shows(&harness, "Reading the text in photos"));
    assert!(shows(&harness, "0 of 3"));

    harness.get_by_label("Abort").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Stopping…"));
    // The one being read ends and is kept; the others wait.
    read.send(())?;
    assert!(eventually(|| reader.busy().is_none()));
    harness.step();
    assert_eq!(notes.unread_photos()?.len(), 2);
    assert!(shows(&harness, "Stopped: the others wait"));

    harness.get_by_label("Go on").click_accesskit();
    harness.step();
    harness.step();
    read.send(())?;
    read.send(())?;
    assert!(eventually(|| notes
        .unread_photos()
        .is_ok_and(|w| w.is_empty())));
    harness.step();
    assert!(!shows(&harness, "Stopped: the others wait"));
    Ok(())
}
