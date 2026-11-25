//! "Empty the trash now" (obf-1095): in the second row of the Trash's
//! band, greyed while nothing in the trash can be deleted; it asks once,
//! naming how many Notes and Routines go, with Delete for good on the
//! error colour and Cancel. Cancel changes nothing; Delete for good
//! deletes them, beside the window where it has the Import's opener,
//! listed on the page Running meanwhile. Driven headless by
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
use std::time::{Duration, Instant};

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{ClockTime, Day, Notes, Routines, Schedule, SigningKey, Store, Trash};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMPTY: &str = "Empty the trash now";
const FOR_GOOD: &str = "Delete for good";

fn mem() -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

fn open(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    harness.step();
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Whether "Empty the trash now" can be clicked.
fn offered(harness: &Harness<'_, App>) -> bool {
    !harness.get_by_label(EMPTY).accesskit_node().is_disabled()
}

/// Notes in the trash, the first `with_routines` with a Reminder that went
/// there with it; one Note outside it.
fn trashed(store: &Store, count: usize, with_routines: usize) -> TestResult {
    let notes = Notes::new(store);
    let routines = Routines::new(store);
    let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let once = Schedule::new(Day::new(2026, 3, 1), &eight, None).ok_or("a schedule")?;
    notes.capture("Kept", None, None)?;
    store.batch(|| {
        for n in 0..count {
            let note = notes.capture(&format!("Old chore {n}"), None, None)?;
            if n < with_routines {
                routines.spawn(&note.id, &once)?;
                routines.trash_note(&note.id)?;
            } else {
                notes.trash(&note.id)?;
            }
        }
        Ok(())
    })?;
    Ok(())
}

/// How many Notes the trash holds.
fn in_trash(store: &Store) -> usize {
    Notes::new(store)
        .trashed()
        .map(|t| t.len())
        .unwrap_or_default()
}

/// Clicks "Empty the trash now".
fn ask(harness: &mut Harness<'_, App>) {
    harness.get_by_label(EMPTY).click();
    harness.step();
    harness.step();
}

#[test]
fn the_trash_band_greys_it_while_nothing_can_be_deleted() -> TestResult {
    let store = Arc::new(mem()?);
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Trash);
    let mut harness = window(app);
    assert!(shows(&harness, EMPTY), "in the Trash's band");
    assert!(!offered(&harness), "nothing to delete");
    // A click on it asks nothing.
    harness.get_by_label(EMPTY).click_accesskit();
    harness.step();
    assert!(!shows(&harness, FOR_GOOD));

    // Not in another list.
    harness.state_mut().select(Selection::All);
    harness.step();
    harness.step();
    assert!(!shows(&harness, EMPTY), "only the Trash has it");

    trashed(&store, 1, 0)?;
    harness.state_mut().select(Selection::Trash);
    harness.step();
    harness.step();
    assert!(offered(&harness), "a Note to delete");
    Ok(())
}

#[test]
fn the_question_names_the_counts_and_cancel_changes_nothing() -> TestResult {
    let store = Arc::new(mem()?);
    trashed(&store, 3, 1)?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Trash);
    let mut harness = window(app);
    assert!(offered(&harness));

    ask(&mut harness);
    assert!(shows(&harness, "Delete 3 Notes and 1 Routine for good?"));
    assert!(shows(&harness, "They can't come back."));
    assert!(shows(&harness, FOR_GOOD));
    harness.get_by_label("Cancel").click_accesskit();
    harness.step();
    harness.step();

    assert!(!shows(&harness, FOR_GOOD), "the question is gone");
    assert_eq!(in_trash(&store), 3, "nothing deleted");
    let left = Trash::new(&store).to_empty()?;
    assert_eq!(left.routines.len(), 1, "the Reminder stays in the trash");
    assert!(offered(&harness), "still offered");
    Ok(())
}

#[test]
fn a_count_of_none_is_left_out_of_the_question() -> TestResult {
    let store = Arc::new(mem()?);
    trashed(&store, 1, 0)?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Trash);
    let mut harness = window(app);
    ask(&mut harness);
    assert!(shows(&harness, "Delete 1 Note for good?"));
    assert!(shows(&harness, "It can't come back."));
    Ok(())
}

#[test]
fn the_question_names_it_in_german() -> TestResult {
    let store = Arc::new(mem()?);
    trashed(&store, 2, 2)?;
    let mut app = App::new(store.clone(), "de");
    app.select(Selection::Trash);
    let mut harness = window(app);
    harness.get_by_label("Papierkorb jetzt leeren").click();
    harness.step();
    harness.step();
    assert!(shows(
        &harness,
        "2 Notizen und 2 Routinen endgültig löschen?"
    ));
    assert!(shows(&harness, "Sie lassen sich nicht zurückholen."));
    assert!(shows(&harness, "Endgültig löschen"));
    Ok(())
}

#[test]
fn delete_for_good_empties_the_trash() -> TestResult {
    let store = Arc::new(mem()?);
    trashed(&store, 3, 1)?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Trash);
    let mut harness = window(app);
    assert!(shows(&harness, "Old chore 2"));

    ask(&mut harness);
    harness.get_by_label(FOR_GOOD).click_accesskit();
    harness.step();
    harness.step();

    assert!(!shows(&harness, FOR_GOOD), "the question is gone");
    assert!(
        !shows(&harness, "The trash could not be emptied."),
        "no failure"
    );
    assert_eq!(in_trash(&store), 0, "the Notes are gone");
    assert!(
        Trash::new(&store).to_empty()?.is_empty(),
        "and the Reminder"
    );
    assert_eq!(Notes::new(&store).all()?.len(), 1, "the rest stays");
    assert!(!shows(&harness, "Old chore 2"), "the Trash shows it");
    assert!(!offered(&harness), "nothing left to delete");
    Ok(())
}

/// Steps the window until `done` holds, for at most ten seconds; the
/// longest step, for the window staying usable meanwhile.
fn step_until(
    harness: &mut Harness<'_, App>,
    done: impl Fn(&Harness<'_, App>) -> bool,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let until = Instant::now() + Duration::from_secs(10);
    let mut longest = Duration::ZERO;
    while !done(harness) {
        if Instant::now() > until {
            return Err("the window never got there".into());
        }
        let at = Instant::now();
        harness.step();
        longest = longest.max(at.elapsed());
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(longest)
}

#[test]
fn beside_the_window_it_shows_on_the_page_running_until_done() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("log.db");
    trashed(&open(&path)?, 400, 0)?;
    let mut app = App::new(open(&path)?, "en");
    let own = Arc::new(open(&path)?);
    app.set_import_store(move || own.clone());
    // The counts are read on a thread of their own too.
    let reads = Arc::new(open(&path)?);
    app.set_agenda_store(move || reads.clone());
    app.select(Selection::Trash);
    let mut harness = window(app);
    step_until(&mut harness, offered)?;

    ask(&mut harness);
    assert!(shows(&harness, "Delete 400 Notes for good?"));
    harness.get_by_label(FOR_GOOD).click_accesskit();
    harness.step();
    harness.step();
    assert!(!offered(&harness), "greyed while it runs");
    // The line under the menu bar opens the page Running.
    harness.get_by_label("Emptying the trash…").click();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Emptying the trash"));
    assert!(shows(&harness, "400 Notes"));
    assert!(!shows(&harness, "Abort"), "one transaction: no Abort");

    let longest = step_until(&mut harness, |h| !shows(h, "Emptying the trash…"))?;
    harness.step();
    assert!(
        longest < Duration::from_millis(500),
        "the window stays usable: {longest:?}"
    );
    assert_eq!(in_trash(&open(&path)?), 0, "the Notes are gone");
    assert!(shows(&harness, "Nothing runs in the background."));
    Ok(())
}
