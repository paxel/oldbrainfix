//! Finishing a Note while capturing it (#113), in the capture popup and
//! the window's capture field, driven headless by egui_kittest over a
//! store with its clock held still: the Note card's buttons show, a
//! choice shows at the top and is not written, leaving or saving writes
//! the Note and all of it together, a sorted Note gets one Save and no
//! Nudge, an empty Note writes nothing and drops its choices, and a write
//! failing on the way leaves nothing written and everything on screen.

use std::sync::Arc;

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    Frogs, Importance, Note, Notes, OwnProfile, Places, Routines, Sessions, SigningKey, Store,
    Tallies, parse_time,
};
use oldbrainfix_gui::App;
use oldbrainfix_gui::capture_choices::CaptureChoices;
use oldbrainfix_gui::popup::{CapturePopup, Outcome};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store with the Area "Household" and the Tally "Push-ups"; the
/// store, the Area's id and the Tally's Note.
fn store() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    let tally = Notes::new(&store).capture("Push-ups", None, None)?.id;
    Tallies::new(&store).add_field(&tally, "Push-ups", "times", Some(20.0), None)?;
    Ok((Arc::new(store), household, tally))
}

/// The Note with `text`, if one was written.
fn note(store: &Store, text: &str) -> Result<Option<Note>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .find(|n| n.text == text))
}

fn nudge(store: &Store, id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(store.current(id, "nudge")?)
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn click<S>(harness: &mut Harness<'_, S>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.run();
}

struct Popup {
    popup: CapturePopup,
    closed: bool,
}

/// A popup over `store`, as tall as a page needs.
fn popup(store: &Arc<Store>) -> Harness<'static, Popup> {
    let popup = CapturePopup::new(store.clone(), "en");
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

fn type_in_popup(harness: &mut Harness<'_, Popup>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Sets up a Session picking "Push-ups" every day in the Routine editor
/// on the page shown, and saves it there.
fn choose_session<S>(harness: &mut Harness<'_, S>) {
    assert!(shows(harness, "New Session"));
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    click(harness, "Save");
}

#[test]
fn the_popup_files_the_note_and_saves_it_without_a_nudge() -> TestResult {
    let (store, household, _) = store()?;
    let mut harness = popup(&store);
    for label in [
        "File into…",
        "Routine",
        "Reminder",
        "Frog",
        "Session",
        "More",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(shows(&harness, "10 min"));
    type_in_popup(&mut harness, "Descale the kettle");
    click(&mut harness, "File into…");
    click(&mut harness, "Household");
    // Shown at the top, not written.
    assert!(shows(&harness, "Into: Household"));
    assert_eq!(
        harness.state().popup.choices().place,
        Some(household.clone())
    );
    assert_eq!(note(&store, "Descale the kettle")?, None);
    // A sorted Note needs no Nudge: one Save.
    assert!(!shows(&harness, "10 min"));
    assert!(shows(&harness, "Save"));
    assert!(shows(
        &harness,
        "Enter saves it. Esc keeps what is typed the same way; empty, it just closes."
    ));
    // Leaving saves all of it.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    let kettle = note(&store, "Descale the kettle")?.ok_or("the Note")?;
    assert_eq!(kettle.place, Some(household));
    assert_eq!(nudge(&store, &kettle.id)?, None);
    Ok(())
}

#[test]
fn a_reminder_set_up_in_the_popup_is_written_with_the_note() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Call the dentist");
    click(&mut harness, "Reminder");
    assert!(harness.state().popup.page_open());
    assert!(shows(&harness, "New Reminder"));
    // The Note it is for, by the text typed so far.
    assert!(shows(&harness, "Call the dentist"));
    click(&mut harness, "Save");
    assert!(!harness.state().popup.page_open());
    assert_eq!(harness.state().popup.text(), "Call the dentist");
    assert_eq!(harness.state().popup.choices().schedules.len(), 1);
    assert!(
        harness
            .query_all_by_label_contains("Reminder: ")
            .next()
            .is_some()
    );
    assert!(Routines::new(&store).all()?.is_empty());
    assert_eq!(note(&store, "Call the dentist")?, None);
    // Dropped and set up again, then Esc on the page takes nothing.
    let chip = harness
        .query_all_by_label_contains("Remove Reminder: ")
        .next()
        .ok_or("the chip's Remove")?;
    chip.click_accesskit();
    harness.run();
    assert!(harness.state().popup.choices().schedules.is_empty());
    assert!(shows(&harness, "10 min"));
    click(&mut harness, "Reminder");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().popup.page_open());
    assert!(!harness.state().closed);
    assert!(harness.state().popup.choices().schedules.is_empty());
    // Alt+Left goes back from the page the same way.
    click(&mut harness, "Reminder");
    harness.key_press_modifiers(egui::Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().popup.page_open());
    assert!(harness.state().popup.choices().schedules.is_empty());
    click(&mut harness, "Reminder");
    click(&mut harness, "Save");
    click(&mut harness, "Save");
    assert!(harness.state().closed);
    let dentist = note(&store, "Call the dentist")?.ok_or("the Note")?;
    let routines = Routines::new(&store).of_note(&dentist.id)?;
    assert_eq!(routines.len(), 1);
    assert!(routines[0].is_reminder());
    assert_eq!(nudge(&store, &dentist.id)?, None);
    Ok(())
}

#[test]
fn an_empty_note_writes_nothing_and_drops_its_choices() -> TestResult {
    let (store, _, _) = store()?;
    let entries = store.all_entries()?.len();
    let mut harness = popup(&store);
    click(&mut harness, "Frog");
    assert!(shows(&harness, "Frog"));
    assert!(harness.state().popup.choices().frog);
    // Nothing typed: the Save rests.
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Save")
            .accesskit_node()
            .is_disabled()
    );
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    assert_eq!(store.all_entries()?.len(), entries);
    assert_eq!(harness.state().popup.choices(), &CaptureChoices::default());
    Ok(())
}

#[test]
fn a_write_failing_in_the_popup_leaves_nothing_written() -> TestResult {
    let (store, _, tally) = store()?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Workout");
    click(&mut harness, "Session");
    choose_session(&mut harness);
    assert!(
        harness
            .query_all_by_label_contains("Session: ")
            .next()
            .is_some()
    );
    // The Tally goes meanwhile: the Session cannot pick it any more.
    Notes::new(&store).trash(&tally)?;
    let entries = store.all_entries()?.len();
    click(&mut harness, "Save");
    assert!(!harness.state().closed);
    assert!(harness.state().popup.error().is_some());
    assert_eq!(store.all_entries()?.len(), entries);
    assert_eq!(note(&store, "Workout")?, None);
    assert!(Sessions::new(&store).all()?.is_empty());
    // Everything stays, to change and save again.
    assert_eq!(harness.state().popup.text(), "Workout");
    assert_eq!(harness.state().popup.choices().schedules.len(), 1);
    let chip = harness
        .query_all_by_label_contains("Remove Session: ")
        .next()
        .ok_or("the chip's Remove")?;
    chip.click_accesskit();
    harness.run();
    click(&mut harness, "1 h");
    assert!(harness.state().closed);
    let workout = note(&store, "Workout")?.ok_or("the Note")?;
    assert!(nudge(&store, &workout.id)?.is_some());
    Ok(())
}

/// The window over `store` in English, on the Agenda.
fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
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

fn type_in_window(harness: &mut Harness<'_, App>, text: &str) {
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

#[test]
fn the_windows_capture_holds_its_choices_until_saved() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(&store);
    // The buttons show once something is typed.
    assert!(!shows(&harness, "Frog"));
    type_in_window(&mut harness, "Water the plants");
    for label in [
        "File into…",
        "Routine",
        "Reminder",
        "Frog",
        "Session",
        "More",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(shows(&harness, "10 min"));
    click(&mut harness, "Frog");
    click(&mut harness, "More");
    harness.get_by_label("Important").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Remove Frog"));
    assert!(shows(&harness, "Remove Important"));
    assert_eq!(note(&store, "Water the plants")?, None);
    assert!(!shows(&harness, "10 min"));
    assert!(shows(&harness, "Ctrl+Enter saves it."));
    click(&mut harness, "Save");
    let plants = note(&store, "Water the plants")?.ok_or("the Note")?;
    assert_eq!(plants.importance, Importance::Important);
    assert_eq!(Frogs::new(&store).of_note(&plants.id)?.len(), 1);
    assert_eq!(nudge(&store, &plants.id)?, None);
    assert_eq!(harness.state().selected_note(), Some(plants.id.as_str()));
    assert!(!shows(&harness, "Remove Frog"));
    Ok(())
}

#[test]
fn a_tally_set_up_in_the_window_is_made_with_the_note() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(&store);
    type_in_window(&mut harness, "Glasses of water");
    click(&mut harness, "More");
    click(&mut harness, "Make it a Tally");
    harness
        .get_by_role_and_label(Role::TextInput, "Name")
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Glasses");
    harness.run();
    click(&mut harness, "Save");
    assert!(shows(&harness, "Tally: Glasses"));
    assert_eq!(note(&store, "Glasses of water")?, None);
    click(&mut harness, "Save");
    let water = note(&store, "Glasses of water")?.ok_or("the Note")?;
    let tally = Tallies::new(&store)
        .by_note(&water.id)?
        .ok_or("its Tally")?;
    assert_eq!(tally.fields.len(), 1);
    assert_eq!(tally.fields[0].name, "Glasses");
    assert_eq!(nudge(&store, &water.id)?, None);
    Ok(())
}

#[test]
fn a_write_failing_in_the_window_leaves_nothing_written() -> TestResult {
    let (store, _, tally) = store()?;
    let mut harness = window(&store);
    type_in_window(&mut harness, "Workout");
    click(&mut harness, "Session");
    choose_session(&mut harness);
    assert!(
        harness
            .query_all_by_label_contains("Session: ")
            .next()
            .is_some()
    );
    assert!(Sessions::new(&store).all()?.is_empty());
    Notes::new(&store).trash(&tally)?;
    let entries = store.all_entries()?.len();
    click(&mut harness, "Save");
    assert!(harness.state().error().is_some());
    assert_eq!(store.all_entries()?.len(), entries);
    assert_eq!(note(&store, "Workout")?, None);
    // The text and the Session stay on the capture.
    assert!(
        harness
            .query_by(
                |n| n.role() == Role::MultilineTextInput && n.value().as_deref() == Some("Workout")
            )
            .is_some()
    );
    assert!(
        harness
            .query_all_by_label_contains("Session: ")
            .next()
            .is_some()
    );
    Ok(())
}
