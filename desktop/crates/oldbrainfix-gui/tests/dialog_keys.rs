//! Keys in the capture and in every dialog (#277, Q664, Q665, UI laws
//! "Desktop ▸ Menu bar and keys"): in the capture popup Enter saves and
//! Shift+Enter starts a new line; in the popup and the window's quick
//! note Ctrl+1, Ctrl+2 and Ctrl+3 save with the Nudge buttons' 10 min,
//! 30 min and 1 h, also with cards selected, whose Importance the list's
//! 1 to 3 would change; Ctrl+Enter runs a dialog's main action (a name
//! dialog's Create, the Routine editor's and the Session page's Save, a
//! Tally entry's and the New Tally page's Save, Send, Merge into… on the
//! first Note found, File into…'s first row and its naming step's
//! Create) and only the dialog's on top; a question that deletes or
//! discards keeps it from its destructive button. The first field of a
//! dialog takes the keyboard as it opens.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{
    Day, DeviceSettings, Importance, Note, Notes, OwnProfile, Places, QuickNoteControl, Recurrence,
    Routines, Schedule, Sessions, SigningKey, Store, Tallies, dart_iso,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock held still on Monday 2026-03-02 noon in Berlin,
/// with the Area "Household", its Notes "Water the plants", with a
/// Routine every day, and "Buy milk", and the Tally "Push-ups".
struct Device {
    store: Arc<Store>,
    household: String,
    plants: String,
    milk: String,
    routine: String,
    tally: String,
}

fn device() -> Result<Device, Box<dyn std::error::Error>> {
    let now = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    // A millisecond a read keeps the order of what is written.
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || now + Duration::milliseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    let notes = Notes::new(&store);
    let plants = notes
        .capture("Water the plants", Some(&household), None)?
        .id;
    let milk = notes.capture("Buy milk", Some(&household), None)?.id;
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let daily = Schedule::new(Day::new(2026, 3, 2), &[], Some(rule)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&plants, &daily)?.id;
    let tally = notes.capture("Push-ups", Some(&household), None)?.id;
    Tallies::new(&store).add_field(&tally, "Push-ups", "times", Some(20.0), None)?;
    Ok(Device {
        store: Arc::new(store),
        household,
        plants,
        milk,
        routine,
        tally,
    })
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

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The last widget labelled `label`: a dialog's, over the window's.
fn last<'a, S>(harness: &'a Harness<'_, S>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

fn click<S>(harness: &mut Harness<'_, S>, label: &str) {
    last(harness, label).click_accesskit();
    harness.run();
}

/// Types `text` where the keyboard is.
fn type_here<S>(harness: &mut Harness<'_, S>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Ctrl (Cmd on macOS) and `key`.
fn ctrl<S>(harness: &mut Harness<'_, S>, key: Key) {
    harness.key_press_modifiers(Modifiers::COMMAND, key);
    harness.run();
}

/// The label or placeholder of the widget holding the keyboard, if any.
fn focused<S>(harness: &Harness<'_, S>) -> Option<(Role, String)> {
    let node = harness.query_by(|n| n.is_focused())?;
    let node = node.accesskit_node();
    let name = node
        .label()
        .or_else(|| node.placeholder().map(str::to_string))
        .unwrap_or_default();
    Some((node.role(), name))
}

fn note_by_text(store: &Store, text: &str) -> Result<Option<Note>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .find(|n| n.text == text))
}

fn nudge(store: &Store, id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(store.current(id, "nudge")?)
}

const NUDGES: [(Key, i64); 3] = [(Key::Num1, 10), (Key::Num2, 30), (Key::Num3, 60)];

#[test]
fn shift_enter_starts_a_new_line_in_the_popup_and_enter_saves_it() -> TestResult {
    let device = device()?;
    let mut harness = popup(&device.store);
    type_here(&mut harness, "Pack the bag");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcome, Outcome::Open);
    type_here(&mut harness, "and the keys");
    assert_eq!(harness.state().popup.text(), "Pack the bag\nand the keys");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let note = note_by_text(&device.store, "Pack the bag\nand the keys")?;
    let note = note.ok_or("the Note of two lines")?;
    let in_an_hour = dart_iso(device.store.now() + Duration::hours(1));
    let nudged = nudge(&device.store, &note.id)?.ok_or("a Nudge")?;
    assert!(nudged <= in_an_hour, "{nudged} {in_an_hour}");
    Ok(())
}

#[test]
fn ctrl_1_2_and_3_in_the_popup_save_with_the_nudge_buttons_times() -> TestResult {
    let device = device()?;
    for (key, minutes) in NUDGES {
        let mut harness = popup(&device.store);
        assert!(shows(
            &harness,
            "Enter saves it with a Nudge in 1 h. Ctrl+1, Ctrl+2 and Ctrl+3 save it with one in \
             10 min, 30 min or 1 h. Shift+Enter starts a new line. Esc keeps what is typed the \
             same way; empty, it just closes."
        ));
        // Nothing typed: nothing saved, the popup stays.
        ctrl(&mut harness, key);
        assert_eq!(harness.state().outcome, Outcome::Open);
        let text = format!("Call Anna {minutes}");
        type_here(&mut harness, &text);
        let before = device.store.now();
        ctrl(&mut harness, key);
        assert_eq!(harness.state().outcome, Outcome::Closed, "{key:?}");
        let note = note_by_text(&device.store, &text)?.ok_or("the Note")?;
        let nudged = nudge(&device.store, &note.id)?.ok_or("a Nudge")?;
        let earliest = dart_iso(before + Duration::minutes(minutes));
        let latest = dart_iso(device.store.now() + Duration::minutes(minutes));
        assert!(earliest <= nudged && nudged <= latest, "{key:?} {nudged}");
    }
    // Ctrl+Enter as Enter: 1 h.
    let mut harness = popup(&device.store);
    type_here(&mut harness, "Order stamps");
    ctrl(&mut harness, Key::Enter);
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let note = note_by_text(&device.store, "Order stamps")?.ok_or("the Note")?;
    assert!(nudge(&device.store, &note.id)?.is_some());
    Ok(())
}

#[test]
fn without_the_nudge_buttons_ctrl_1_saves_nothing_in_the_popup() -> TestResult {
    let device = device()?;
    DeviceSettings::new(&device.store).set_quick_note_nudge_buttons_on(false)?;
    let mut harness = popup(&device.store);
    type_here(&mut harness, "Buy stamps");
    ctrl(&mut harness, Key::Num1);
    assert_eq!(harness.state().outcome, Outcome::Open);
    assert!(!shows(&harness, "10 min"));
    assert!(note_by_text(&device.store, "Buy stamps")?.is_none());
    Ok(())
}

/// The window's quick note, opened and `text` typed into it.
fn quick_note(harness: &mut Harness<'_, App>, text: &str) {
    harness.state_mut().new_note();
    harness.run();
    assert!(harness.state().quick_note_open());
    // Its field took the keyboard as it opened.
    type_here(harness, text);
}

#[test]
fn ctrl_1_2_and_3_in_the_quick_note_save_with_their_nudge() -> TestResult {
    let device = device()?;
    let mut harness = window(&device.store, Selection::Agenda);
    for (key, minutes) in NUDGES {
        let text = format!("Ring the plumber {minutes}");
        quick_note(&mut harness, &text);
        assert!(shows(
            &harness,
            "Ctrl+Enter saves it with a Nudge in 1 h. Ctrl+1, Ctrl+2 and Ctrl+3 save it with one \
             in 10 min, 30 min or 1 h."
        ));
        let before = device.store.now();
        ctrl(&mut harness, key);
        assert!(!harness.state().quick_note_open(), "{key:?}");
        let note = note_by_text(&device.store, &text)?.ok_or("the Note")?;
        let nudged = nudge(&device.store, &note.id)?.ok_or("a Nudge")?;
        let earliest = dart_iso(before + Duration::minutes(minutes));
        let latest = dart_iso(device.store.now() + Duration::minutes(minutes));
        assert!(earliest <= nudged && nudged <= latest, "{key:?} {nudged}");
    }
    Ok(())
}

#[test]
fn ctrl_1_in_the_quick_note_leaves_the_cards_selected_as_they_are() -> TestResult {
    let device = device()?;
    let mut harness = window(&device.store, Selection::All);
    // The list's Down selects the first card.
    harness.key_press(Key::ArrowDown);
    harness.run();
    let selected = harness.state().selected_cards();
    assert_eq!(selected.len(), 1);
    quick_note(&mut harness, "Fix the bike");
    ctrl(&mut harness, Key::Num1);
    assert!(!harness.state().quick_note_open());
    assert!(note_by_text(&device.store, "Fix the bike")?.is_some());
    let card = Notes::new(&device.store)
        .by_id(&selected[0])?
        .ok_or("the card's Note")?;
    assert_eq!(card.importance, Importance::Normal);
    Ok(())
}

#[test]
fn a_name_dialog_opens_with_its_field_taking_the_keyboard_and_ctrl_enter_creates() -> TestResult {
    let device = device()?;
    let mut harness = window(&device.store, Selection::Agenda);
    harness
        .get_by_role_and_label(Role::Button, "New Area")
        .click();
    harness.run();
    assert_eq!(
        focused(&harness).map(|(role, _)| role),
        Some(Role::TextInput)
    );
    // Empty, Create waits, and so does Ctrl+Enter.
    ctrl(&mut harness, Key::Enter);
    assert!(shows(&harness, "Create"));
    type_here(&mut harness, "Errands");
    ctrl(&mut harness, Key::Enter);
    assert!(!shows(&harness, "Create"));
    let areas = Places::new(&device.store).areas()?;
    assert!(areas.iter().any(|a| a.name == "Errands"));
    Ok(())
}

#[test]
fn the_routine_editor_opens_in_its_first_field_and_ctrl_enter_saves_it() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().open_routine(&device.routine);
    harness.run();
    assert!(harness.state().routine_editor().is_some());
    // From a Note, the sentence is its first field.
    assert_eq!(
        focused(&harness),
        Some((
            Role::MultilineTextInput,
            t.schedule_sentence_label().to_string()
        ))
    );
    click(&mut harness, t.repeat_weekdays());
    ctrl(&mut harness, Key::Enter);
    assert!(harness.state().routine_editor().is_none());
    let routine = Routines::new(&device.store)
        .by_id(&device.routine)?
        .ok_or("the Routine")?;
    let rule = routine.schedule.rule.as_ref().map(Recurrence::format);
    assert!(rule.is_some_and(|r| r.contains("BYDAY=MO")));

    // A new one made from its text: the text first.
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, t.new_routine_tooltip())
        .click();
    harness.run();
    assert!(harness.state().routine_editor().is_some());
    assert_eq!(
        focused(&harness).map(|(_, name)| name),
        Some(t.routine_what_hint().to_string())
    );
    type_here(&mut harness, "Stretch");
    ctrl(&mut harness, Key::Enter);
    assert!(harness.state().routine_editor().is_none());
    assert!(note_by_text(&device.store, "Stretch")?.is_some());
    Ok(())
}

#[test]
fn a_discard_question_keeps_ctrl_enter_from_its_answers() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().open_routine(&device.routine);
    harness.run();
    // Weekdays with none ticked: a change that can't be saved.
    click(&mut harness, t.repeat_weekdays());
    click(&mut harness, t.weekday_mon());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(shows(&harness, "Discard changes?"));
    ctrl(&mut harness, Key::Enter);
    assert!(shows(&harness, "Discard changes?"));
    assert!(harness.state().routine_editor().is_some());
    click(&mut harness, t.leave_keep_editing());
    assert!(harness.state().routine_editor().is_some());
    Ok(())
}

#[test]
fn ctrl_enter_saves_a_session_page() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let legs = Notes::new(&device.store).capture("Leg day", Some(&device.household), None)?;
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().select_note(&legs.id);
    harness.run();
    let make = |harness: &mut Harness<'_, App>| {
        click(
            harness,
            &t.setting_badge_label(t.badge_kind(), t.kind_plain()),
        );
        harness
            .get_by_role_and_label(Role::RadioButton, t.session())
            .click_accesskit();
        harness.run();
        assert!(
            harness
                .state()
                .routine_editor()
                .is_some_and(|e| e.makes_session())
        );
    };
    make(&mut harness);
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    ctrl(&mut harness, Key::Enter);
    assert!(harness.state().routine_editor().is_none());
    assert!(Sessions::new(&device.store).is_session_note(&legs.id)?);
    Ok(())
}

#[test]
fn a_tally_entry_opens_in_its_value_and_ctrl_enter_saves_it() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().select_note(&device.tally);
    harness.run();
    click(&mut harness, "Add entry");
    assert!(shows(&harness, t.tally_new_entry()));
    assert_eq!(
        focused(&harness),
        Some((Role::TextInput, "Push-ups".to_string()))
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    type_here(&mut harness, "25");
    ctrl(&mut harness, Key::Enter);
    assert!(!shows(&harness, t.tally_new_entry()));
    let entries = Tallies::new(&device.store).entries(&device.tally, &Default::default())?;
    let values: Vec<f64> = entries
        .iter()
        .flat_map(|e| e.values.values().copied())
        .collect();
    assert_eq!(values, [25.0]);
    Ok(())
}

#[test]
fn the_new_tally_page_saves_by_ctrl_enter_but_not_from_under_its_badge_dialog() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::Place(device.household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Tally");
    assert!(shows(&harness, t.new_tally()));
    type_here(&mut harness, "Morning");
    harness
        .get_all_by_role_and_label(Role::TextInput, t.kind_name())
        .last()
        .ok_or("the field's name")?
        .click();
    harness.run();
    type_here(&mut harness, "Steps");
    // The field's Rule badge's dialog over the page keeps Ctrl+Enter
    // from it.
    let rule = format!("{}: ", t.tally_rule());
    harness
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with(&rule))
        })
        .last()
        .ok_or("the Rule badge")?
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, t.close()));
    ctrl(&mut harness, Key::Enter);
    assert!(
        shows(&harness, t.new_tally()),
        "the page stays under its badge"
    );
    assert!(note_by_text(&device.store, "Morning")?.is_none());
    click(&mut harness, t.close());
    ctrl(&mut harness, Key::Enter);
    assert!(!shows(&harness, t.new_tally()));
    let morning = note_by_text(&device.store, "Morning")?.ok_or("the Tally's Note")?;
    let tally = Tallies::new(&device.store)
        .by_note(&morning.id)?
        .ok_or("the Tally")?;
    assert_eq!(tally.fields[0].name, "Steps");
    Ok(())
}

#[test]
fn merge_into_takes_the_first_note_found_by_ctrl_enter_once_something_is_typed() -> TestResult {
    let device = device()?;
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().select_note(&device.milk);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::M);
    harness.run();
    assert!(shows(&harness, "Merge into…"));
    assert_eq!(
        focused(&harness).map(|(role, _)| role),
        Some(Role::TextInput)
    );
    // Nothing typed: nothing merged.
    ctrl(&mut harness, Key::Enter);
    let milk = Notes::new(&device.store)
        .by_id(&device.milk)?
        .ok_or("the milk")?;
    assert!(!milk.is_merged());
    type_here(&mut harness, "plants");
    ctrl(&mut harness, Key::Enter);
    let milk = Notes::new(&device.store)
        .by_id(&device.milk)?
        .ok_or("the milk")?;
    assert_eq!(milk.merged_into.as_deref(), Some(device.plants.as_str()));
    Ok(())
}

#[test]
fn send_goes_by_ctrl_enter_from_the_note_dialog() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("for-anyone");
    let mut app = App::new(device.store.clone(), "en");
    app.select(Selection::All);
    app.edit_note(&device.milk);
    {
        let save = saved.clone();
        app.set_item_saver(Box::new(move |_, _| Some(save.clone())));
    }
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
    assert_eq!(
        focused(&harness).map(|(role, _)| role),
        Some(Role::MultilineTextInput)
    );
    click(&mut harness, "Send…");
    assert!(shows(&harness, t.send_title()));
    click(&mut harness, t.send_anyone());
    ctrl(&mut harness, Key::Enter);
    assert!(!shows(&harness, t.send_title()));
    assert!(
        harness
            .query_all_by_label_contains("Saved. Send the file")
            .next()
            .is_some()
    );
    assert!(saved.with_extension("oldbrain").exists());
    Ok(())
}

#[test]
fn file_into_takes_its_first_row_and_names_a_new_area_by_ctrl_enter_over_the_quick_note()
-> TestResult {
    let device = device()?;
    DeviceSettings::new(&device.store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    let mut harness = window(&device.store, Selection::Agenda);
    quick_note(&mut harness, "Buy screws");
    click(&mut harness, "File into…");
    assert!(shows(&harness, "Filter Areas and Topics"));
    type_here(&mut harness, "Cellar");
    // Nothing matches: Enter asks the name.
    harness.key_press(Key::Enter);
    harness.run();
    assert!(shows(&harness, "New Area"));
    // Create, and the capture under it keeps its Ctrl+Enter.
    ctrl(&mut harness, Key::Enter);
    assert!(!shows(&harness, "Filter Areas and Topics"));
    assert!(harness.state().quick_note_open());
    assert!(note_by_text(&device.store, "Buy screws")?.is_none());
    let areas = Places::new(&device.store).areas()?;
    let cellar = areas
        .iter()
        .find(|a| a.name == "Cellar")
        .ok_or("the Area")?;
    // The list's first row by Ctrl+Enter: the Area filed into last.
    click(&mut harness, "File into…");
    ctrl(&mut harness, Key::Enter);
    assert!(!shows(&harness, "Filter Areas and Topics"));
    ctrl(&mut harness, Key::Enter);
    assert!(!harness.state().quick_note_open());
    let screws = note_by_text(&device.store, "Buy screws")?.ok_or("the Note")?;
    assert_eq!(screws.place.as_deref(), Some(cellar.id.as_str()));
    Ok(())
}

#[test]
fn the_feedback_page_opens_in_its_text() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::Agenda);
    harness.state_mut().open(Page::Feedback);
    harness.run();
    assert_eq!(
        focused(&harness),
        Some((
            Role::MultilineTextInput,
            t.feedback_text_label().to_string()
        ))
    );
    Ok(())
}

#[test]
fn a_tally_fields_page_opens_in_its_name_and_ctrl_enter_saves_it() -> TestResult {
    let device = device()?;
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().select_note(&device.tally);
    harness.run();
    harness
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("Target: "))
        })
        .last()
        .ok_or("the field's Target badge")?
        .click_accesskit();
    harness.run();
    // An existing field's page opens in its name, too.
    assert_eq!(
        focused(&harness),
        Some((Role::TextInput, "Name".to_string()))
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    type_here(&mut harness, "Sit-ups");
    ctrl(&mut harness, Key::Enter);
    let tally = Tallies::new(&device.store)
        .by_note(&device.tally)?
        .ok_or("the Tally")?;
    assert_eq!(tally.fields[0].name, "Sit-ups");
    Ok(())
}

#[test]
fn ctrl_enter_takes_a_page_of_the_popup_and_file_intos_naming_step() -> TestResult {
    let device = device()?;
    DeviceSettings::new(&device.store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    let mut harness = popup(&device.store);
    type_here(&mut harness, "Call the dentist");
    // The Reminder's page: Ctrl+Enter is its Save.
    harness
        .get_by_role_and_label(Role::Button, "Routine, Reminder, Frog or Session")
        .click();
    harness.run();
    click(&mut harness, "Reminder");
    assert!(harness.state().popup.page_open());
    ctrl(&mut harness, Key::Enter);
    assert!(!harness.state().popup.page_open());
    assert_eq!(harness.state().outcome, Outcome::Open);
    assert_eq!(harness.state().popup.choices().schedules.len(), 1);
    // File into…'s naming step: Ctrl+Enter is its Create.
    click(&mut harness, "File into…");
    assert!(harness.state().popup.page_open());
    type_here(&mut harness, "Dentist");
    harness.key_press(Key::Enter);
    harness.run();
    assert!(shows(&harness, "New Area"));
    ctrl(&mut harness, Key::Enter);
    assert!(!harness.state().popup.page_open());
    assert_eq!(harness.state().outcome, Outcome::Open);
    let areas = Places::new(&device.store).areas()?;
    let dentist = areas
        .iter()
        .find(|a| a.name == "Dentist")
        .ok_or("the Area")?;
    assert_eq!(
        harness.state().popup.choices().place.as_deref(),
        Some(dentist.id.as_str())
    );
    assert!(note_by_text(&device.store, "Call the dentist")?.is_none());
    Ok(())
}
