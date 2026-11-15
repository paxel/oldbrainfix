//! Add to Note… on the desktop (#308), driven headless by egui_kittest:
//! the quick note and the capture popup add what is typed, recorded or
//! held to the Note picked at once as a Section, written where that Note
//! lies and merged into it, so nothing is left in Unsorted; the picker's
//! filter has the keyboard, "Added to recently" shows the Notes added to
//! last on top while nothing is typed, and Enter takes the first row. An
//! Unsorted Note's card offers it in place of Merge into….

use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, Note, Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::media::{self, Recorder};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SAUCE: &str = "Tomato sauce";
const PASTA: &str = "Pasta for Friday";
const MILK: &str = "Buy milk";

/// "Tomato sauce" filed in Household, "Pasta for Friday" Unsorted, and
/// "Buy milk" in the trash.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let now = DateTime::parse_from_rfc3339("2026-11-15T09:00:00Z")?.with_timezone(&Utc);
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([8; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let house = Places::new(&store).add_area("Household")?;
    let notes = Notes::new(&store);
    notes.capture(SAUCE, Some(&house.id), None)?;
    notes.capture(PASTA, None, None)?;
    let milk = notes.capture(MILK, None, None)?;
    notes.trash(&milk.id)?;
    Ok(store)
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
    harness.run();
    harness
}

/// The window's quick note, open, with `text` typed.
fn quick_note(app: App, text: &str) -> Harness<'static, App> {
    let mut harness = window(app);
    harness.state_mut().new_note();
    harness.run();
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    type_here(&mut harness, text);
    harness
}

/// A microphone that hears a tenth of a second of silence.
#[derive(Default)]
struct Mic {
    recording: bool,
}

impl Recorder for Mic {
    fn start(&mut self) -> bool {
        self.recording = true;
        true
    }

    fn stop(&mut self) -> Option<Vec<u8>> {
        std::mem::take(&mut self.recording).then(|| media::wav(&[0.0; 1600]))
    }

    fn recording(&self) -> bool {
        self.recording
    }

    fn elapsed(&self) -> StdDuration {
        StdDuration::from_secs(3)
    }
}

/// Types `text` where the keyboard is.
fn type_here<S>(harness: &mut Harness<'_, S>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Clicks the last button named `label`, the one over the rest.
fn press<S>(harness: &mut Harness<'_, S>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next_back()
        .ok_or_else(|| format!("no {label:?}"))?
        .click_accesskit();
    harness.run_steps(8);
    Ok(())
}

/// Picks the Note `text` in Add to Note…, drawn after its card.
fn pick<S>(harness: &mut Harness<'_, S>, text: &str) -> TestResult {
    harness
        .query_all_by_label(text)
        .next_back()
        .ok_or_else(|| format!("no {text:?} to pick"))?
        .click();
    harness.run();
    Ok(())
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The Note `text` in the lists.
fn note(store: &Store, text: &str) -> Result<Note, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .find(|n| n.text == text)
        .ok_or_else(|| format!("no {text:?}"))?)
}

/// The texts of the Notes left Unsorted.
fn unsorted(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .filter(Note::is_unsorted)
        .map(|n| n.text)
        .collect())
}

#[test]
fn the_quick_note_adds_its_text_to_the_note_picked_as_a_section() -> TestResult {
    let store = store()?;
    let mut harness = quick_note(App::new(store, "en"), "more garlic");
    press(&mut harness, "Add to Note…")?;
    assert!(harness.state().quick_note_open());
    // The filter has the keyboard; the trash is not offered.
    assert!(!shows(&harness, MILK));
    type_here(&mut harness, "sauce");
    assert!(!shows(&harness, PASTA));
    harness.key_press(Key::Enter);
    harness.run();

    assert!(!harness.state().quick_note_open());
    let store = harness.state().store();
    let sauce = note(store, SAUCE)?;
    let sections = Notes::new(store).sections(&sauce.id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].text, "more garlic");
    // Where the Note lies, so it is sealed for the same people.
    let added = Notes::new(store)
        .by_id(&sections[0].id)?
        .ok_or("the Note added")?;
    assert_eq!(added.place, sauce.place);
    assert_eq!(unsorted(store)?, [PASTA]);
    assert_eq!(DeviceSettings::new(store).recently_added_to()?, [sauce.id]);
    Ok(())
}

#[test]
fn the_notes_added_to_last_show_on_top_and_enter_takes_the_latest() -> TestResult {
    let store = store()?;
    let sauce = note(&store, SAUCE)?.id;
    let pasta = note(&store, PASTA)?.id;
    let settings = DeviceSettings::new(&store);
    settings.added_to(&pasta)?;
    settings.added_to(&sauce)?;
    let mut harness = quick_note(App::new(store, "en"), "basil");
    press(&mut harness, "Add to Note…")?;
    assert!(shows(&harness, "Added to recently"));
    let heading = harness.get_by_label("Added to recently").rect().top();
    let below = harness
        .query_all_by_label(PASTA)
        .next_back()
        .ok_or("Pasta in the list")?
        .rect()
        .top();
    assert!(heading < below);
    // Typed, the recent ones go.
    type_here(&mut harness, "pas");
    assert!(!shows(&harness, "Added to recently"));
    for _ in 0..3 {
        harness.key_press(Key::Backspace);
    }
    harness.run_steps(8);
    // Nothing typed, Enter takes the Note added to last.
    harness.key_press(Key::Enter);
    harness.run();

    let store = harness.state().store();
    let sections = Notes::new(store).sections(&sauce)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].text, "basil");
    Ok(())
}

#[test]
fn a_recording_in_the_quick_note_becomes_a_section_too() -> TestResult {
    let store = store()?;
    let mut app = App::new(store, "en");
    app.set_recorder(Box::new(Mic::default()));
    let mut harness = window(app);
    harness.state_mut().new_note();
    harness.run();
    press(&mut harness, "Record voice")?;
    // Still recording: Add to Note… ends it and keeps it.
    press(&mut harness, "Add to Note…")?;
    pick(&mut harness, SAUCE)?;

    let store = harness.state().store();
    let sauce = note(store, SAUCE)?;
    let sections = Notes::new(store).sections(&sauce.id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].recordings.len(), 1);
    assert_eq!(unsorted(store)?, [PASTA]);
    Ok(())
}

#[test]
fn cancel_in_add_to_note_keeps_the_quick_note_as_it_was() -> TestResult {
    let store = store()?;
    let mut harness = quick_note(App::new(store, "en"), "more garlic");
    press(&mut harness, "Add to Note…")?;
    press(&mut harness, "Cancel")?;

    assert!(harness.state().quick_note_open());
    let store = harness.state().store();
    assert!(
        Notes::new(store)
            .sections(&note(store, SAUCE)?.id)?
            .is_empty()
    );
    assert!(DeviceSettings::new(store).recently_added_to()?.is_empty());
    Ok(())
}

struct Popup {
    popup: CapturePopup,
    closed: bool,
}

fn popup(store: &Arc<Store>, recorder: bool) -> Harness<'static, Popup> {
    let mut popup = CapturePopup::new(store.clone(), "en");
    if recorder {
        popup.set_recorder(Box::new(Mic::default()));
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 900.0))
        .build_ui_state(
            |ui, state: &mut Popup| {
                App::install(ui.ctx());
                if state.popup.show(ui) == Outcome::Closed {
                    state.closed = true;
                }
            },
            Popup {
                popup,
                closed: false,
            },
        );
    harness.run();
    harness
}

#[test]
fn the_popup_adds_to_the_note_picked_on_its_page_and_closes() -> TestResult {
    let store = Arc::new(store()?);
    let mut harness = popup(&store, false);
    type_here(&mut harness, "more garlic");
    press(&mut harness, "Add to Note…")?;
    assert!(harness.state().popup.page_open());
    // Esc goes back to the capture, nothing added.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().popup.page_open());
    assert!(!harness.state().closed);
    press(&mut harness, "Add to Note…")?;
    pick(&mut harness, SAUCE)?;

    assert!(harness.state().closed);
    assert_eq!(harness.state().popup.text(), "");
    let sauce = note(&store, SAUCE)?;
    let sections = Notes::new(&store).sections(&sauce.id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].text, "more garlic");
    assert_eq!(unsorted(&store)?, [PASTA]);
    Ok(())
}

#[test]
fn the_popup_adds_a_recording_still_running_to_the_note_picked() -> TestResult {
    let store = Arc::new(store()?);
    let mut harness = popup(&store, true);
    press(&mut harness, "Record voice")?;
    press(&mut harness, "Add to Note…")?;
    pick(&mut harness, PASTA)?;

    assert!(harness.state().closed);
    let pasta = note(&store, PASTA)?;
    let sections = Notes::new(&store).sections(&pasta.id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].recordings.len(), 1);
    // Added to an Unsorted Note, the Section is in no Area either.
    let added = Notes::new(&store)
        .by_id(&sections[0].id)?
        .ok_or("the Note added")?;
    assert_eq!(added.place, None);
    assert_eq!(unsorted(&store)?, [PASTA]);
    Ok(())
}

#[test]
fn an_unsorted_cards_right_click_offers_add_to_note_in_place_of_merge_into() -> TestResult {
    let store = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let mut harness = window(app);
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, PASTA)
        .click_secondary();
    harness.run();
    assert!(shows(&harness, "Add to Note…"));
    assert!(!shows(&harness, "Merge into…"));
    harness
        .query_all_by_label("Add to Note…")
        .next_back()
        .ok_or("Add to Note… in the menu")?
        .click();
    harness.run_steps(8);
    pick(&mut harness, SAUCE)?;

    let store = harness.state().store();
    let sauce = note(store, SAUCE)?;
    // The core's Merge, as Merge into… does.
    let pasta = Notes::new(store)
        .sections(&sauce.id)?
        .into_iter()
        .next()
        .ok_or("Pasta merged")?;
    assert_eq!(pasta.text, PASTA);
    assert!(unsorted(store)?.is_empty());
    assert_eq!(DeviceSettings::new(store).recently_added_to()?, [sauce.id]);
    // A filed Note's card keeps Merge into….
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, SAUCE)
        .click_secondary();
    harness.run();
    assert!(shows(&harness, "Merge into…"));
    assert!(!shows(&harness, "Add to Note…"));
    Ok(())
}
