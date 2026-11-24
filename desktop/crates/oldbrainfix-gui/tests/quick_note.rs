//! The quick note (#142, #352) in the capture popup and the window's
//! quick note dialog (#146), driven headless by egui_kittest over a store
//! with its clock held still: it shows the text, the three Nudge buttons
//! and under them Open in the editor across the width (obf-1080), nothing
//! else, also where the five old switches of Settings ▸ Quick note are
//! on, whose rows are gone; with the Nudge buttons off one Save nudges in
//! 1 h; a filed capture shows Save; Esc saves as the 1 h button does, an
//! empty one nothing; Open in the editor saves the Note and opens it in
//! the Note dialog.

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
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The button that saves the Note and opens the Note dialog (obf-1080).
const OPEN: &str = "Open in the editor";

/// What the quick note no longer shows (#352): the alarm clock, the
/// buttons for recordings, pictures and files, To full editor, Add to
/// Note…, File into… and the format bar.
const GONE: [&str; 9] = [
    "Routine, Reminder, Frog or Session",
    "Record voice",
    "Paste",
    "From file…",
    "To full editor",
    "Add to Note…",
    "File into…",
    "Bold",
    "Make it a Tally",
];

/// The five old switches of Settings ▸ Quick note.
const SWITCHES: [QuickNoteControl; 5] = [
    QuickNoteControl::FileInto,
    QuickNoteControl::Schedule,
    QuickNoteControl::Shopping,
    QuickNoteControl::Importance,
    QuickNoteControl::FormatBar,
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

fn in_an_hour(store: &Store) -> Option<String> {
    Some(dart_iso(store.now() + Duration::hours(1)))
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

/// Whether the quick note shows the text, the three Nudge buttons and
/// Open in the editor, and nothing of [`GONE`] nor the Kind choice.
fn only_text_nudges_and_open<S>(harness: &Harness<'_, S>) {
    for label in ["10 min", "30 min", "1 h", OPEN] {
        assert!(shows(harness, label), "{label}");
    }
    for label in GONE {
        assert!(!shows(harness, label), "{label}");
    }
    assert!(
        harness
            .query_all_by_role_and_label(Role::RadioButton, "Checklist")
            .next()
            .is_none()
    );
}

/// Whether Open in the editor sits under the button `above`, as wide as
/// the text field, which has the keyboard (obf-1080).
fn under_and_across<S>(harness: &Harness<'_, S>, above: &str) {
    let open = harness.get_by_role_and_label(Role::Button, OPEN).rect();
    let above = harness.get_by_role_and_label(Role::Button, above).rect();
    let field = harness.get_by(|n| n.is_focused()).rect();
    assert!(open.min.y >= above.max.y, "{open:?} under {above:?}");
    assert!(
        open.width() >= field.width() - 1.0,
        "{open:?} across {field:?}"
    );
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
    // The quick note's dialog first (#146).
    if !harness.state().quick_note_open() {
        harness.state_mut().new_note();
        harness.run();
    }
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

#[test]
fn the_popup_shows_only_the_text_the_nudge_buttons_and_open_in_the_editor() -> TestResult {
    let (store, _) = store()?;
    let mut harness = popup(&store);
    only_text_nudges_and_open(&harness);
    // Open in the editor waits while there is nothing to save.
    assert!(!enabled(&harness, OPEN));
    type_in_popup(&mut harness, "Buy stamps");
    assert!(enabled(&harness, OPEN));
    only_text_nudges_and_open(&harness);
    under_and_across(&harness, "1 h");
    Ok(())
}

#[test]
fn the_window_quick_note_shows_only_the_text_the_nudge_buttons_and_open_in_the_editor() -> TestResult
{
    let (store, _) = store()?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().new_note();
    harness.run();
    // Nothing typed, nothing to save: no Open in the editor yet.
    assert!(!shows(&harness, OPEN));
    type_in_window(&mut harness, "Buy stamps");
    only_text_nudges_and_open(&harness);
    under_and_across(&harness, "1 h");
    Ok(())
}

#[test]
fn the_old_switches_bring_nothing_back() -> TestResult {
    let (store, _) = store()?;
    let settings = DeviceSettings::new(&store);
    for control in SWITCHES {
        settings.set_quick_note_shows(control, true)?;
    }
    // A list to add to, which the Shopping switch once offered.
    let list = Notes::new(&store).capture("Groceries", None, None)?.id;
    oldbrainfix_core::ShoppingLists::new(&store).make(&list)?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Buy stamps");
    only_text_nudges_and_open(&harness);
    let mut harness = window(&store, Selection::All);
    type_in_window(&mut harness, "Post the letter");
    only_text_nudges_and_open(&harness);
    // What they stored stays.
    for control in SWITCHES {
        assert!(settings.quick_note_shows(control)?, "{control:?}");
    }
    Ok(())
}

#[test]
fn the_settings_card_offers_only_the_nudge_buttons() -> TestResult {
    let (store, _) = store()?;
    let mut harness = window(&store, Selection::Agenda);
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert!(shows(&harness, "Quick note"));
    for label in [
        "File into…",
        "Frog, Session or Tally",
        "Add to a Shopping List",
        "Importance",
        "Format bar",
    ] {
        assert!(
            harness
                .query_all_by_role_and_label(Role::CheckBox, label)
                .next()
                .is_none(),
            "{label}"
        );
    }
    harness
        .get_by_role_and_label(Role::CheckBox, "Nudge buttons")
        .click_accesskit();
    harness.run();
    let settings = DeviceSettings::new(&store);
    assert!(!settings.quick_note_nudge_buttons_on()?);
    harness.state_mut().close_page();
    harness.run();
    type_in_window(&mut harness, "Buy stamps");
    assert!(!shows(&harness, "10 min"));
    assert!(shows(&harness, "Save"));
    assert!(shows(&harness, OPEN));
    under_and_across(&harness, "Save");
    Ok(())
}

#[test]
fn without_the_nudge_buttons_one_save_nudges_in_an_hour() -> TestResult {
    let (store, _) = store()?;
    DeviceSettings::new(&store).set_quick_note_nudge_buttons_on(false)?;
    let mut harness = popup(&store);
    assert!(!shows(&harness, "10 min"));
    type_in_popup(&mut harness, "Buy stamps");
    click(&mut harness, "Save");
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let stamps = note(&store, "Buy stamps")?.ok_or("the Note")?;
    assert_eq!(nudge(&store, &stamps.id)?, in_an_hour(&store));
    let mut harness = window(&store, Selection::Agenda);
    type_in_window(&mut harness, "Post the letter");
    assert!(!shows(&harness, "10 min"));
    click(&mut harness, "Save");
    let letter = note(&store, "Post the letter")?.ok_or("the Note")?;
    assert_eq!(letter.place, None);
    assert_eq!(nudge(&store, &letter.id)?, in_an_hour(&store));
    Ok(())
}

#[test]
fn a_filed_capture_shows_save_instead_of_the_nudge_buttons() -> TestResult {
    let (store, household) = store()?;
    // In the window, inside an Area, it goes there.
    let mut harness = window(&store, Selection::Place(household.clone()));
    type_in_window(&mut harness, "Fix the tap");
    assert!(!shows(&harness, "10 min"));
    assert!(shows(&harness, OPEN));
    under_and_across(&harness, "Save");
    assert!(shows(&harness, "Ctrl+Enter saves it."));
    click(&mut harness, "Save");
    let tap = note(&store, "Fix the tap")?.ok_or("the Note")?;
    assert_eq!(tap.place.as_deref(), Some(household.as_str()));
    assert_eq!(nudge(&store, &tap.id)?, None);
    Ok(())
}

#[test]
fn esc_saves_as_the_hour_button_does_and_an_empty_one_nothing() -> TestResult {
    let (store, _) = store()?;
    let entries = store.all_entries()?.len();
    // Empty, Esc closes both and writes nothing.
    let mut harness = popup(&store);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let mut harness = window(&store, Selection::Agenda);
    harness.state_mut().new_note();
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().quick_note_open());
    assert_eq!(store.all_entries()?.len(), entries);
    // Typed, Esc keeps it Unsorted, nudged in 1 h.
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Buy stamps");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let stamps = note(&store, "Buy stamps")?.ok_or("the Note")?;
    assert_eq!(stamps.place, None);
    assert_eq!(nudge(&store, &stamps.id)?, in_an_hour(&store));
    let mut harness = window(&store, Selection::Agenda);
    type_in_window(&mut harness, "Post the letter");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().quick_note_open());
    let letter = note(&store, "Post the letter")?.ok_or("the Note")?;
    assert_eq!(letter.place, None);
    assert_eq!(nudge(&store, &letter.id)?, in_an_hour(&store));
    Ok(())
}

#[test]
fn open_in_the_editor_in_the_window_saves_unsorted_and_opens_the_note_dialog() -> TestResult {
    let (store, household) = store()?;
    let mut harness = window(&store, Selection::Agenda);
    type_in_window(&mut harness, "Call the plumber");
    click(&mut harness, OPEN);
    let plumber = note(&store, "Call the plumber")?.ok_or("the Note")?;
    assert_eq!(plumber.place, None);
    assert_eq!(nudge(&store, &plumber.id)?, in_an_hour(&store));
    assert!(!harness.state().quick_note_open());
    assert_eq!(harness.state().selected_note(), Some(plumber.id.as_str()));
    // Where it was opened from stays; its text has the keyboard.
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    let focused = harness.get_by(|n| n.is_focused());
    assert_eq!(focused.value().as_deref(), Some("Call the plumber"));
    // Inside an Area it is filed there and needs no Nudge.
    let mut harness = window(&store, Selection::Place(household.clone()));
    type_in_window(&mut harness, "Fix the tap");
    click(&mut harness, OPEN);
    let tap = note(&store, "Fix the tap")?.ok_or("the Note")?;
    assert_eq!(tap.place.as_deref(), Some(household.as_str()));
    assert_eq!(nudge(&store, &tap.id)?, None);
    assert_eq!(harness.state().selected_note(), Some(tap.id.as_str()));
    Ok(())
}

#[test]
fn open_in_the_editor_in_the_popup_hands_the_note_to_the_window() -> TestResult {
    let (store, _) = store()?;
    let mut harness = popup(&store);
    type_in_popup(&mut harness, "Call the plumber");
    click(&mut harness, OPEN);
    let plumber = note(&store, "Call the plumber")?.ok_or("the Note")?;
    assert_eq!(harness.state().outcome, Outcome::Edit(plumber.id.clone()));
    assert_eq!(plumber.place, None);
    assert_eq!(nudge(&store, &plumber.id)?, in_an_hour(&store));
    // The window opens it in its Note dialog, its text with the keyboard.
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
