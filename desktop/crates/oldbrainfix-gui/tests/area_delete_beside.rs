//! Deleting an Area beside the window (obf-1093): once its dialog is
//! answered, its Notes go on a thread of its own with a connection of its
//! own, the line under the menu bar moving; the Area's row says how many
//! of its Notes have gone and takes no click, and a second start of it is
//! not made. At the end the Area is gone and its Notes are where the
//! dialog said; a failure is announced at the top. The window draws and
//! reads on meanwhile. Driven headless by egui_kittest over a store on
//! disk, the deletion's connection held at an entry by its clock.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Duration;

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Notes, NotesGo, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn open(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

/// The Area Household on the database at `path`, its Topic Repairs, and
/// `count` Notes in them, every third in the Topic; one Note elsewhere.
/// The Area's id.
fn plant(path: &Path, count: usize) -> Result<String, Box<dyn std::error::Error>> {
    let store = open(path)?;
    let places = Places::new(&store);
    let household = places.add_area("Household")?;
    let repairs = places.add_topic(&household.id, "Repairs")?;
    let notes = Notes::new(&store);
    store.batch(|| {
        for n in 0..count {
            let place = if n % 3 == 2 {
                &repairs.id
            } else {
                &household.id
            };
            notes.capture(&format!("Chore {n}"), Some(place), None)?;
        }
        Ok(())
    })?;
    notes.capture("Call the bank", None, None)?;
    Ok(household.id)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
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

fn step_until(
    harness: &mut Harness<'_, App>,
    done: impl Fn(&Harness<'_, App>) -> bool,
) -> TestResult {
    for _ in 0..400 {
        if done(harness) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

/// Picks Area ▸ Delete, by keyboard or screen reader, for the Area shown.
fn area_delete(harness: &mut Harness<'_, App>) {
    harness
        .get_all_by_label("Area")
        .next()
        .expect("the Area menu")
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
}

/// Answers the open dialog's question with Move to trash.
fn to_trash(harness: &mut Harness<'_, App>) {
    harness.get_by_label("Move to trash").click_accesskit();
    harness.step();
    harness.step();
}

/// The deletion's own connection: it waits for the test to let it on
/// before it opens, and again each time its clock is asked, which every
/// entry written does, until the test lets go of it.
struct Held {
    on: Option<SyncSender<()>>,
    /// The threads the connection was opened on, one per deletion.
    opened: Arc<Mutex<Vec<ThreadId>>>,
}

impl Held {
    /// The window over the database at `path` showing the Area `area`,
    /// each deletion on a connection of its own whose clock `Held` holds.
    fn window(
        path: &Path,
        area: &str,
    ) -> Result<(Held, Harness<'static, App>), Box<dyn std::error::Error>> {
        let (on, waits) = sync_channel::<()>(0);
        let waits = Arc::new(Mutex::new(waits));
        let opened = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new(open(path)?, "en");
        app.select(Selection::Place(area.to_string()));
        // Opened here, its clock waiting only once the deletion has it.
        let armed = Arc::new(AtomicBool::new(false));
        let (ticks, gate) = (armed.clone(), waits.clone());
        let clock = move || {
            // Let go of: the clock waits no more.
            if ticks.load(Ordering::SeqCst)
                && let Ok(gate) = gate.lock()
            {
                let _ = gate.recv();
            }
            Utc::now()
        };
        let own = Arc::new(Store::open(path, Box::new(clock), || {
            Ok(SigningKey::from_seed([3; 32]))
        })?);
        let seen = opened.clone();
        app.set_import_store(move || {
            if let Ok(mut seen) = seen.lock() {
                seen.push(std::thread::current().id());
            }
            let waits: Arc<Mutex<Receiver<()>>> = waits.clone();
            if let Ok(waits) = waits.lock() {
                let _ = waits.recv();
            }
            armed.store(true, Ordering::SeqCst);
            own.clone()
        });
        Ok((
            Held {
                on: Some(on),
                opened,
            },
            window(app),
        ))
    }

    /// Lets the deletion on one entry at a time until the window shows
    /// `label`; there it waits again.
    fn until(&self, harness: &mut Harness<'_, App>, label: &str) -> TestResult {
        let on = self.on.as_ref().ok_or("let go already")?;
        for _ in 0..2000 {
            harness.step();
            if shows(harness, label) {
                return Ok(());
            }
            // Back once the deletion asked its clock the next time.
            on.send(())?;
        }
        Err(format!("the window never showed {label}").into())
    }

    /// Lets go of the deletion's clock.
    fn let_go(&mut self) {
        self.on = None;
    }
}

#[test]
fn an_area_is_deleted_beside_the_window_its_row_counting_the_notes_moved() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("log.db");
    let household = plant(&path, 3)?;
    let (mut held, mut harness) = Held::window(&path, &household)?;

    area_delete(&mut harness);
    assert!(shows(&harness, "3 Notes inside."), "counted for the dialog");
    to_trash(&mut harness);
    // The dialog closed; held before its connection opens, the row says
    // it is being deleted, the line moves.
    assert!(!shows(&harness, "Delete the Area"));
    assert!(harness.state().deleting_area());
    assert!(shows(&harness, "Being deleted…"));
    assert!(
        harness.query_by_label("Deleting an Area…").is_some(),
        "the line moves"
    );
    let opened = held.opened.lock().map_err(|_| "opened")?.clone();
    assert_eq!(opened.len(), 1, "one connection of its own");
    assert_ne!(
        opened[0],
        std::thread::current().id(),
        "on a thread of its own"
    );

    // The window draws and reads on meanwhile.
    harness.state_mut().select(Selection::All);
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert!(shows(&harness, "Call the bank"));
    assert!(shows(&harness, "Being deleted…"), "still held");

    // Started again, from the menu bar or the row, it is not asked for.
    harness
        .state_mut()
        .select(Selection::Place(household.clone()));
    harness.step();
    area_delete(&mut harness);
    assert!(!shows(&harness, "Delete the Area"), "not asked again");
    harness
        .get_all_by_label("Household")
        .next()
        .expect("its row")
        .click_secondary();
    harness.step();
    harness.step();
    assert!(
        harness.query_by_label("Rename").is_none(),
        "the row opens no menu"
    );

    // Held inside a Note's transaction from here, it counts its Notes as
    // they go.
    held.until(&mut harness, "Being deleted: 0 of 3 Notes moved")?;
    held.until(&mut harness, "Being deleted: 1 of 3 Notes moved")?;
    held.until(&mut harness, "Being deleted: 2 of 3 Notes moved")?;

    held.let_go();
    step_until(&mut harness, |h| !h.state().deleting_area())?;
    harness.step();
    harness.step();
    assert_eq!(held.opened.lock().map_err(|_| "opened")?.len(), 1);
    assert!(!shows(&harness, "Being deleted"));
    assert!(
        harness.query_by_label("Deleting an Area…").is_none(),
        "gone with it"
    );
    // Done without a word; the Area shown is gone, All Notes shows.
    assert_eq!(harness.state().notice(), None);
    assert_eq!(harness.state().selection(), &Selection::All);
    assert!(!shows(&harness, "Household"));
    assert!(!shows(&harness, "Repairs"));
    // The window's own connection sees what the deletion's wrote.
    let desk = harness.state().store();
    assert!(Places::new(desk).area_by_id(&household)?.is_none());
    let trashed = Notes::new(desk).trashed()?;
    assert_eq!(trashed.len(), 3);
    assert!(trashed.iter().all(|n| n.text.starts_with("Chore")));
    assert_eq!(Notes::new(desk).all()?.len(), 1);
    Ok(())
}

#[test]
fn an_area_that_could_not_be_deleted_is_announced_at_the_top() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("log.db");
    let household = plant(&path, 2)?;
    let mut app = App::new(open(&path)?, "en");
    app.select(Selection::Place(household.clone()));
    let own = Arc::new(open(&path)?);
    app.set_import_store(move || own.clone());
    let mut harness = window(app);
    area_delete(&mut harness);
    // Gone another way while the dialog asked.
    Places::new(&open(&path)?).delete_area(&household, NotesGo::Unsorted)?;
    to_trash(&mut harness);
    step_until(&mut harness, |h| !h.state().deleting_area())?;
    harness.step();
    harness.step();
    assert_eq!(
        harness.state().notice(),
        Some("The Area could not be deleted.")
    );
    harness
        .get_by_label("The Area could not be deleted.")
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Area not deleted"));
    assert!(shows(&harness, "Not all of its Notes could be moved."));
    Ok(())
}

/// The count the Area's row shows, "Being deleted: 12 of 1000 Notes
/// moved" as 12.
fn count_shown(harness: &Harness<'_, App>, of: &str) -> Option<usize> {
    harness
        .query_all_by_label_contains("Being deleted: ")
        .find_map(|n| {
            let node = n.accesskit_node();
            let text = node.label().or_else(|| node.value())?;
            text.strip_prefix("Being deleted: ")?
                .strip_suffix(of)?
                .trim()
                .parse()
                .ok()
        })
}

#[test]
fn the_window_reads_and_counts_on_while_an_area_is_deleted() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("log.db");
    let household = plant(&path, 1000)?;
    let mut app = App::new(open(&path)?, "en");
    app.select(Selection::Place(household.clone()));
    let own = Arc::new(open(&path)?);
    app.set_import_store(move || own.clone());
    let mut harness = window(app);
    area_delete(&mut harness);
    to_trash(&mut harness);
    // Each frame reads the list anew, which catches the search up on
    // the window's own connection: a write, which the deletion's Notes,
    // a transaction each with a pause after it, leave room for.
    let mut counts = Vec::new();
    for n in 0..4000 {
        if !harness.state().deleting_area() {
            break;
        }
        let place = if n % 2 == 0 {
            Selection::All
        } else {
            Selection::Agenda
        };
        harness.state_mut().select(place);
        harness.step();
        assert_eq!(harness.state().error(), None);
        if let Some(count) = count_shown(&harness, " of 1000 Notes moved")
            && counts.last() != Some(&count)
        {
            counts.push(count);
        }
    }
    assert!(!harness.state().deleting_area(), "it ended");
    assert!(
        counts.len() >= 5 && counts.windows(2).all(|w| w[0] < w[1]),
        "the window drew on, counting: {counts:?}"
    );
    let desk = harness.state().store();
    assert!(Places::new(desk).area_by_id(&household)?.is_none());
    assert_eq!(Notes::new(desk).trashed()?.len(), 1000);
    Ok(())
}
