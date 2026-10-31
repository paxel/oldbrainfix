//! The uncluttered quick note (#142) in the capture popup and the
//! window's capture field, driven headless by egui_kittest over a store
//! with its clock held still: by default only the text, its buttons for
//! recordings, pictures and files, the Nudge buttons and "To full
//! editor" show; each switch of Settings ▸ Quick note brings its control
//! back and takes it away again; with the Nudge buttons off one Save
//! nudges in 1 h; a filed capture shows Save; "To full editor" saves the
//! Note and opens it in the Note editor.

use std::sync::Arc;

use chrono::Duration;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    DeviceSettings, Note, Notes, OwnProfile, Places, QuickNoteControl, SigningKey, Store, dart_iso,
    parse_time,
};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What only a switch brings to the capture: "File into…", Routine and
/// the rest, and "More" with the Importance, Make it a Tally and the
/// Shopping Lists.
const SWITCHED: [&str; 6] = [
    "File into…",
    "Routine",
    "Reminder",
    "Frog",
    "Session",
    "More",
];

/// A store with the Area "Household"; the store and the Area's id.
fn store() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([5; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    Ok((Arc::new(store), household))
}

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

/// Whether the button `label` can be clicked.
fn enabled<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    !harness
        .get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .is_disabled()
}

struct Popup {
    popup: CapturePopup,
    outcome: Outcome,
}

fn popup(store: &Arc<Store>) -> Harness<'static, Popup> {
    let popup = CapturePopup::new(store.clone(), "en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 900.0))
        .build_ui_state(
            |ui, state: &mut Popup| {
                App::install(ui.ctx());
                if state.outcome == Outcome::Open {
                    state.outcome = state.popup.show(ui);
                }
            },
            Popup {
                popup,
                outcome: Outcome::Open,
            },
        );
    harness.run();
    harness
}

fn type_in_popup(harness: &mut Harness<'_, Popup>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// The window over `store` in English, showing `selection`.
fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(selection);
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
fn the_default_popup_shows_only_the_text_the_nudge_buttons_and_to_full_editor() -> TestResult {
    let (store, _) = store()?;
    let mut harness = popup(&store);
    for label in SWITCHED {
        assert!(!shows(&harness, label), "{label}");
    }
    assert!(!shows(&harness, "Bold"));
    for label in ["10 min", "30 min", "1 h", "To full editor"] {
        assert!(shows(&harness, label), "{label}");
    }
    // It waits while there is nothing to save.
    assert!(!enabled(&harness, "To full editor"));
    type_in_popup(&mut harness, "Buy stamps");
    assert!(enabled(&harness, "To full editor"));
    Ok(())
}

#[test]
fn the_default_window_capture_shows_only_the_text_the_nudge_buttons_and_to_full_editor()
-> TestResult {
    let (store, _) = store()?;
    let mut harness = window(&store, Selection::Agenda);
    type_in_window(&mut harness, "Buy stamps");
    for label in SWITCHED {
        assert!(!shows(&harness, label), "{label}");
    }
    assert!(!shows(&harness, "Make it a Tally"));
    for label in ["10 min", "30 min", "1 h", "To full editor", "From file…"] {
        assert!(shows(&harness, label), "{label}");
    }
    Ok(())
}

#[test]
fn each_switch_brings_its_control_back_and_takes_it_away_again() -> TestResult {
    let (store, _) = store()?;
    let settings = DeviceSettings::new(&store);
    let brought: [(QuickNoteControl, &[&str]); 3] = [
        (QuickNoteControl::FileInto, &["File into…"]),
        (
            QuickNoteControl::Schedule,
            &["Routine", "Reminder", "Frog", "Session", "More"],
        ),
        (QuickNoteControl::FormatBar, &["Bold", "Italic", "Quote"]),
    ];
    for (control, labels) in brought {
        settings.set_quick_note_shows(control, true)?;
        let harness = popup(&store);
        for label in labels {
            assert!(shows(&harness, label), "{control:?} {label}");
        }
        drop(harness);
        settings.set_quick_note_shows(control, false)?;
        let harness = popup(&store);
        for label in labels {
            assert!(!shows(&harness, label), "{control:?} {label}");
        }
    }
    // Make it a Tally goes with Routine and the rest (Q481).
    settings.set_quick_note_shows(QuickNoteControl::Schedule, true)?;
    let mut harness = popup(&store);
    click(&mut harness, "More");
    assert!(shows(&harness, "Make it a Tally"));
    assert!(!shows(&harness, "Important"));
    drop(harness);
    settings.set_quick_note_shows(QuickNoteControl::Schedule, false)?;
    // The Importance alone is in "More" too.
    settings.set_quick_note_shows(QuickNoteControl::Importance, true)?;
    let mut harness = popup(&store);
    click(&mut harness, "More");
    assert!(shows(&harness, "Important"));
    assert!(!shows(&harness, "Make it a Tally"));
    drop(harness);
    settings.set_quick_note_shows(QuickNoteControl::Importance, false)?;
    // Add to a Shopping List, while there is a list to add to.
    let list = Notes::new(&store).capture("Groceries", None, None)?.id;
    oldbrainfix_core::ShoppingLists::new(&store).make(&list)?;
    assert!(!shows(&popup(&store), "More"));
    settings.set_quick_note_shows(QuickNoteControl::Shopping, true)?;
    let mut harness = popup(&store);
    click(&mut harness, "More");
    assert!(shows(&harness, "Add to a Shopping List"));
    assert!(!shows(&harness, "Important"));
    Ok(())
}

#[test]
fn the_settings_switch_what_the_window_capture_shows() -> TestResult {
    let (store, _) = store()?;
    let mut harness = window(&store, Selection::Settings);
    assert!(shows(&harness, "Quick note"));
    harness
        .get_by_role_and_label(Role::CheckBox, "File into…")
        .click_accesskit();
    harness.run();
    let settings = DeviceSettings::new(&store);
    assert!(settings.quick_note_shows(QuickNoteControl::FileInto)?);
    assert_eq!(
        store.local_setting("quickNoteFileInto")?.as_deref(),
        Some("true")
    );
    harness
        .get_by_role_and_label(Role::CheckBox, "Nudge buttons")
        .click_accesskit();
    harness.run();
    assert!(!settings.quick_note_nudge_buttons_on()?);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    type_in_window(&mut harness, "Buy stamps");
    assert!(shows(&harness, "File into…"));
    assert!(!shows(&harness, "10 min"));
    assert!(shows(&harness, "Save"));
    // Off again, it goes.
    harness.state_mut().select(Selection::Settings);
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, "File into…")
        .click_accesskit();
    harness.run();
    assert!(!settings.quick_note_shows(QuickNoteControl::FileInto)?);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!shows(&harness, "File into…"));
    Ok(())
}

#[test]
fn without_the_nudge_buttons_one_save_nudges_in_an_hour() -> TestResult {
    let (store, _) = store()?;
    DeviceSettings::new(&store).set_quick_note_nudge_buttons_on(false)?;
    let in_an_hour = Some(dart_iso(store.now() + Duration::hours(1)));
    let mut harness = popup(&store);
    assert!(!shows(&harness, "10 min"));
    type_in_popup(&mut harness, "Buy stamps");
    click(&mut harness, "Save");
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let stamps = note(&store, "Buy stamps")?.ok_or("the Note")?;
    assert_eq!(nudge(&store, &stamps.id)?, in_an_hour);
    let mut harness = window(&store, Selection::Agenda);
    type_in_window(&mut harness, "Post the letter");
    assert!(!shows(&harness, "10 min"));
    click(&mut harness, "Save");
    let letter = note(&store, "Post the letter")?.ok_or("the Note")?;
    assert_eq!(letter.place, None);
    assert_eq!(nudge(&store, &letter.id)?, in_an_hour);
    Ok(())
}

#[test]
fn a_filed_capture_shows_save_instead_of_the_nudge_buttons() -> TestResult {
    let (store, household) = store()?;
    // In the window, inside an Area, it goes there.
    let mut harness = window(&store, Selection::Place(household.clone()));
    type_in_window(&mut harness, "Fix the tap");
    assert!(!shows(&harness, "10 min"));
    assert!(shows(&harness, "Ctrl+Enter saves it."));
    click(&mut harness, "Save");
    let tap = note(&store, "Fix the tap")?.ok_or("the Note")?;
    assert_eq!(tap.place.as_deref(), Some(household.as_str()));
    assert_eq!(nudge(&store, &tap.id)?, None);
    // In the popup, filed with File into….
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Descale the kettle");
    click(&mut harness, "File into…");
    click(&mut harness, "Household");
    assert!(!shows(&harness, "10 min"));
    click(&mut harness, "Save");
    let kettle = note(&store, "Descale the kettle")?.ok_or("the Note")?;
    assert_eq!(kettle.place.as_deref(), Some(household.as_str()));
    assert_eq!(nudge(&store, &kettle.id)?, None);
    Ok(())
}

#[test]
fn to_full_editor_in_the_window_saves_unsorted_and_opens_the_note_editor() -> TestResult {
    let (store, household) = store()?;
    let mut harness = window(&store, Selection::Agenda);
    type_in_window(&mut harness, "Call the plumber");
    click(&mut harness, "To full editor");
    let plumber = note(&store, "Call the plumber")?.ok_or("the Note")?;
    assert_eq!(plumber.place, None);
    assert_eq!(
        nudge(&store, &plumber.id)?,
        Some(dart_iso(store.now() + Duration::hours(1)))
    );
    assert_eq!(harness.state().selected_note(), Some(plumber.id.as_str()));
    // Where it was opened from stays; its text has the keyboard.
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    let focused = harness.get_by(|n| n.is_focused());
    assert_eq!(focused.value().as_deref(), Some("Call the plumber"));
    // Inside an Area it is filed there and needs no Nudge.
    let mut harness = window(&store, Selection::Place(household.clone()));
    type_in_window(&mut harness, "Fix the tap");
    click(&mut harness, "To full editor");
    let tap = note(&store, "Fix the tap")?.ok_or("the Note")?;
    assert_eq!(tap.place.as_deref(), Some(household.as_str()));
    assert_eq!(nudge(&store, &tap.id)?, None);
    assert_eq!(harness.state().selected_note(), Some(tap.id.as_str()));
    Ok(())
}

#[test]
fn to_full_editor_in_the_popup_hands_the_note_to_the_window() -> TestResult {
    let (store, _) = store()?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Call the plumber");
    click(&mut harness, "To full editor");
    let plumber = note(&store, "Call the plumber")?.ok_or("the Note")?;
    assert_eq!(harness.state().outcome, Outcome::Edit(plumber.id.clone()));
    assert_eq!(plumber.place, None);
    assert_eq!(
        nudge(&store, &plumber.id)?,
        Some(dart_iso(store.now() + Duration::hours(1)))
    );
    // The window opens it in its Note editor, its text with the keyboard.
    let mut app = App::new(store.clone(), "en");
    app.edit_note(&plumber.id);
    let mut window = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    window.run();
    assert_eq!(window.state().selected_note(), Some(plumber.id.as_str()));
    let focused = window.get_by(|n| n.is_focused());
    assert_eq!(focused.value().as_deref(), Some("Call the plumber"));
    Ok(())
}

#[test]
fn to_full_editor_is_not_offered_while_a_shopping_list_is_picked() -> TestResult {
    let (store, _) = store()?;
    let list = Notes::new(&store).capture("Groceries", None, None)?.id;
    oldbrainfix_core::ShoppingLists::new(&store).make(&list)?;
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::Shopping, true)?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Eggs");
    click(&mut harness, "More");
    harness.get_by_label("Groceries").click_accesskit();
    harness.run();
    assert!(!shows(&harness, "To full editor"));
    Ok(())
}

#[test]
fn the_format_bar_formats_the_capture_where_it_is_switched_on() -> TestResult {
    let (store, _) = store()?;
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FormatBar, true)?;
    let mut harness = popup(&store);
    click(&mut harness, "Bulleted list");
    type_in_popup(&mut harness, "Eggs");
    click(&mut harness, "1 h");
    let eggs = note(&store, "- Eggs")?.ok_or("the Note")?;
    assert_eq!(eggs.text_format, oldbrainfix_core::text_format::MARKDOWN);
    Ok(())
}
