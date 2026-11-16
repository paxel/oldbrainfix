//! Leaving an edit dialog saves what is valid and asks only about what
//! isn't (#275, Q658, UI laws "Navigation" and "Desktop ▸ The window"):
//! the Routine editor, a Tally entry, the New Tally page and the Session
//! pages, left by Esc, a click beside them, Alt+Left or the mouse's back
//! button, close as they opened without writing, save a change that can
//! be saved as their Save does, and ask "Discard changes?" once about one
//! that can't: Discard closes without saving, Keep editing, Esc or a
//! click beside the question stay and say what to fix. Cancel still
//! closes without saving. The capture popup's Routine page does the same.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{
    ClockTime, Day, Notes, OwnProfile, Places, Recurrence, Routines, Schedule, Sessions,
    SigningKey, Store, Tallies, parse_time,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// A store on a clock that moves a second per read from Monday
/// 2026-03-02 noon, with the Area "Household", the Note "Water the
/// plants" and its Routine every day, all day, and the Tally "Push-ups"
/// of one field with the target 20: the store, the Area, the Routine and
/// the Tally's Note.
struct Device {
    store: Arc<Store>,
    household: String,
    routine: String,
    tally: String,
}

fn device() -> Result<Device, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([9; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    let notes = Notes::new(&store);
    let plants = notes.capture("Water the plants", Some(&household), None)?;
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let daily = Schedule::new(Day::new(2026, 3, 2), &[], Some(rule)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&plants.id, &daily)?.id;
    let tally = notes.capture("Push-ups", Some(&household), None)?.id;
    Tallies::new(&store).add_field(&tally, "Push-ups", "times", Some(20.0), None)?;
    Ok(Device {
        store: Arc::new(store),
        household,
        routine,
        tally,
    })
}

/// The window over `store` in English on Berlin's clock, showing
/// `selection`.
fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(berlin));
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

/// The window with the editor of the Routine open.
fn editing(device: &Device) -> Harness<'static, App> {
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().open_routine(&device.routine);
    harness.run();
    assert!(harness.state().routine_editor().is_some());
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

fn esc<S>(harness: &mut Harness<'_, S>) {
    harness.key_press(Key::Escape);
    harness.run();
}

/// A click with the pointer at the window's left edge, beside a dialog.
fn click_beside<S>(harness: &mut Harness<'_, S>) {
    let pos = Pos2::new(20.0, 700.0);
    harness.event(Event::PointerMoved(pos));
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

/// Tab out of the field holding the keyboard: the editor opens in its
/// When line (#277), where Alt+Left rests, and a click by accessibility
/// leaves the keyboard there.
fn leave_field<S>(harness: &mut Harness<'_, S>) {
    harness.key_press(Key::Tab);
    harness.run();
    assert!(
        harness
            .query_by(|n| n.is_focused()
                && matches!(n.role(), Role::MultilineTextInput | Role::TextInput))
            .is_none()
    );
}

/// Types `text` over what the field labelled `label` holds.
fn type_into<S>(harness: &mut Harness<'_, S>, label: &str, text: &str) {
    harness
        .query_all_by_role_and_label(Role::TextInput, label)
        .next_back()
        .unwrap_or_else(|| panic!("no field {label}"))
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// The Routine's rule as written.
fn rule_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let routine = Routines::new(store).by_id(id)?.ok_or("the Routine")?;
    Ok(routine
        .schedule
        .rule
        .as_ref()
        .map(Recurrence::format)
        .unwrap_or_default())
}

/// Weekdays, Monday ticked as the start's weekday: a change that can be
/// saved.
fn pick_weekdays(harness: &mut Harness<'_, App>, t: &L10n) {
    click(harness, t.repeat_weekdays());
}

/// Weekdays with none ticked: no schedule, a change that can't be saved.
fn pick_no_weekday(harness: &mut Harness<'_, App>, t: &L10n) {
    pick_weekdays(harness, t);
    click(harness, t.weekday_mon());
}

#[test]
fn the_routine_editor_left_unchanged_writes_nothing() -> TestResult {
    let device = device()?;
    let mut harness = editing(&device);
    let entries = device.store.all_entries()?.len();
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert!(!shows(&harness, "Discard changes?"));
    assert_eq!(device.store.all_entries()?.len(), entries);
    Ok(())
}

#[test]
fn leaving_the_routine_editor_saves_a_change_by_esc_a_click_beside_or_back() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = editing(&device);
    pick_weekdays(&mut harness, &t);
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert!(!shows(&harness, "Discard changes?"));
    assert!(rule_of(&device.store, &device.routine)?.contains("BYDAY=MO"));

    // A click beside it.
    harness.state_mut().open_routine(&device.routine);
    harness.run();
    click(&mut harness, t.repeat_every());
    click_beside(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert_eq!(rule_of(&device.store, &device.routine)?, "FREQ=DAILY");

    // Alt+Left, once the cursor left the When line it starts in (#277),
    // where Alt+Left rests.
    harness.state_mut().open_routine(&device.routine);
    harness.run();
    pick_weekdays(&mut harness, &t);
    leave_field(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert!(harness.state().routine_editor().is_none());
    assert!(rule_of(&device.store, &device.routine)?.contains("BYDAY=MO"));

    // The mouse's back button.
    harness.state_mut().open_routine(&device.routine);
    harness.run();
    click(&mut harness, t.repeat_every());
    mouse_back(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert_eq!(rule_of(&device.store, &device.routine)?, "FREQ=DAILY");
    Ok(())
}

/// The mouse's back button, pressed and let go.
fn mouse_back<S>(harness: &mut Harness<'_, S>) {
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: Pos2::new(640.0, 700.0),
            button: PointerButton::Extra1,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

#[test]
fn a_changed_schedule_that_never_comes_up_asks_but_a_past_ones_own_saves() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    // A Reminder of yesterday at 9:00, gone by.
    let call = Notes::new(&device.store).capture("Call the bank", None, None)?;
    let nine = ClockTime::new(9, 0).ok_or("a time")?;
    let yesterday = Schedule::new(Day::new(2026, 3, 1), &[nine], None).ok_or("a schedule")?;
    let id = Routines::new(&device.store).spawn(&call.id, &yesterday)?.id;
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().open_routine(&id);
    harness.run();
    // Its schedule as it was, only the Re-nag changed: saved.
    click(&mut harness, &t.renag_minutes(5));
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    let routine = Routines::new(&device.store)
        .by_id(&id)?
        .ok_or("the Reminder")?;
    assert_eq!(routine.renag, Some(Duration::minutes(5)));
    // A day further back, it never comes up: asked.
    harness.state_mut().open_routine(&id);
    harness.run();
    click(&mut harness, t.day_before());
    esc(&mut harness);
    assert!(shows(&harness, "Discard changes?"));
    click(&mut harness, t.leave_discard());
    assert!(harness.state().routine_editor().is_none());
    let routine = Routines::new(&device.store)
        .by_id(&id)?
        .ok_or("the Reminder")?;
    assert_eq!(routine.schedule, yesterday);
    Ok(())
}

#[test]
fn making_a_note_a_session_saves_on_leaving_once_a_tally_is_ticked() -> TestResult {
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
    // Left as it opened, it makes nothing.
    make(&mut harness);
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert!(!Sessions::new(&device.store).is_session_note(&legs.id)?);
    // A Tally ticked, leaving makes it a Session.
    make(&mut harness);
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert!(Sessions::new(&device.store).is_session_note(&legs.id)?);
    let setup = Sessions::new(&device.store)
        .setup_of(&legs.id)?
        .ok_or("a Session")?;
    assert_eq!(setup.pool, vec![device.tally.clone()]);
    Ok(())
}

#[test]
fn cancel_still_drops_a_change_of_the_routine_editor() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = editing(&device);
    pick_weekdays(&mut harness, &t);
    click(&mut harness, t.cancel());
    assert!(harness.state().routine_editor().is_none());
    assert!(!shows(&harness, "Discard changes?"));
    assert_eq!(rule_of(&device.store, &device.routine)?, "FREQ=DAILY");
    Ok(())
}

#[test]
fn a_change_that_cant_be_saved_asks_once_and_keep_editing_stays() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = editing(&device);
    pick_no_weekday(&mut harness, &t);
    esc(&mut harness);
    assert!(shows(&harness, "Discard changes?"));
    assert!(shows(&harness, t.leave_discard_body()));
    assert!(harness.state().routine_editor().is_some());
    // Esc on the question keeps editing; so do a click beside it and
    // Keep editing.
    esc(&mut harness);
    assert!(!shows(&harness, "Discard changes?"));
    assert!(harness.state().routine_editor().is_some());
    click_beside(&mut harness);
    assert!(shows(&harness, "Discard changes?"));
    click_beside(&mut harness);
    assert!(!shows(&harness, "Discard changes?"));
    assert!(harness.state().routine_editor().is_some());
    esc(&mut harness);
    click(&mut harness, t.leave_keep_editing());
    assert!(!shows(&harness, "Discard changes?"));
    assert!(harness.state().routine_editor().is_some());
    assert_eq!(rule_of(&device.store, &device.routine)?, "FREQ=DAILY");
    // Fixed, leaving saves it.
    click(&mut harness, t.weekday_tue());
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    assert!(rule_of(&device.store, &device.routine)?.contains("BYDAY=TU"));
    Ok(())
}

#[test]
fn discard_closes_the_routine_editor_without_saving() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = editing(&device);
    pick_no_weekday(&mut harness, &t);
    leave_field(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert!(shows(&harness, "Discard changes?"));
    click(&mut harness, t.leave_discard());
    assert!(harness.state().routine_editor().is_none());
    assert!(!shows(&harness, "Discard changes?"));
    assert_eq!(rule_of(&device.store, &device.routine)?, "FREQ=DAILY");
    Ok(())
}

#[test]
fn a_new_session_page_asks_until_a_tally_is_ticked_then_leaving_saves_it() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::Place(device.household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, t.session());
    assert!(shows(&harness, t.new_session()));
    // Left as it opened, it makes nothing.
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, t.session());
    type_into(&mut harness, t.kind_name(), "Workout");
    // No Tally ticked: asked, and Keep editing says what is missing.
    esc(&mut harness);
    assert!(shows(&harness, "Discard changes?"));
    click(&mut harness, t.leave_keep_editing());
    assert!(shows(&harness, t.session_pool_missing()));
    assert!(Sessions::new(&device.store).all()?.is_empty());
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    esc(&mut harness);
    assert!(harness.state().routine_editor().is_none());
    let sessions = Sessions::new(&device.store).all()?;
    let [session] = sessions.as_slice() else {
        return Err(format!("one Session: {sessions:?}").into());
    };
    assert_eq!(session.setup.pool, vec![device.tally.clone()]);
    let note = Notes::new(&device.store)
        .by_id(&session.routine.note_id)?
        .ok_or("its Note")?;
    assert_eq!(note.text, "Workout");
    Ok(())
}

/// The Note open in the Note dialog, the Tally's.
fn tally_window(device: &Device) -> Harness<'static, App> {
    let mut harness = window(&device.store, Selection::All);
    harness.state_mut().select_note(&device.tally);
    harness.run();
    harness
}

fn add_entry(harness: &mut Harness<'_, App>, t: &L10n) {
    click(harness, "Add entry");
    assert!(shows(harness, t.tally_new_entry()));
}

fn entries(device: &Device) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let entries = Tallies::new(&device.store).entries(&device.tally, &Default::default())?;
    Ok(entries
        .iter()
        .flat_map(|e| e.values.values().copied())
        .collect())
}

#[test]
fn a_tally_entry_left_as_it_opened_targets_and_all_is_not_saved() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = tally_window(&device);
    add_entry(&mut harness, &t);
    esc(&mut harness);
    assert!(!shows(&harness, t.tally_new_entry()));
    assert!(!shows(&harness, "Discard changes?"));
    assert!(entries(&device)?.is_empty());
    Ok(())
}

#[test]
fn leaving_a_tally_entry_saves_a_change_and_cancel_does_not() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = tally_window(&device);
    add_entry(&mut harness, &t);
    type_into(&mut harness, "Push-ups", "25");
    click(&mut harness, t.cancel());
    assert!(entries(&device)?.is_empty());
    add_entry(&mut harness, &t);
    type_into(&mut harness, "Push-ups", "25");
    click_beside(&mut harness);
    assert!(!shows(&harness, t.tally_new_entry()));
    assert_eq!(entries(&device)?, [25.0]);
    add_entry(&mut harness, &t);
    type_into(&mut harness, "Push-ups", "30");
    esc(&mut harness);
    assert!(!shows(&harness, t.tally_new_entry()));
    assert_eq!(entries(&device)?.len(), 2);
    Ok(())
}

#[test]
fn a_tally_entry_that_is_no_number_asks_and_keep_editing_says_so() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = tally_window(&device);
    add_entry(&mut harness, &t);
    type_into(&mut harness, "Push-ups", "lots");
    esc(&mut harness);
    assert!(shows(&harness, "Discard changes?"));
    assert!(!shows(&harness, t.tally_not_a_number()));
    click(&mut harness, t.leave_keep_editing());
    assert!(shows(&harness, t.tally_new_entry()));
    assert!(shows(&harness, t.tally_not_a_number()));
    esc(&mut harness);
    click(&mut harness, t.leave_discard());
    assert!(!shows(&harness, t.tally_new_entry()));
    assert!(entries(&device)?.is_empty());
    Ok(())
}

#[test]
fn the_new_tally_page_saves_on_leaving_and_asks_without_a_field() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Selection::Place(device.household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Tally");
    assert!(shows(&harness, t.new_tally()));
    // Unchanged, it closes.
    esc(&mut harness);
    assert!(!shows(&harness, t.new_tally()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Tally");
    harness.get_by(|n| n.is_focused()).type_text("Morning");
    harness.run();
    // Without a field with a name: asked.
    esc(&mut harness);
    assert!(shows(&harness, "Discard changes?"));
    click(&mut harness, t.leave_keep_editing());
    assert!(shows(&harness, t.tally_fields_missing()));
    harness
        .get_all_by_role_and_label(Role::TextInput, t.kind_name())
        .last()
        .ok_or("the field's name")?
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Steps");
    harness.run();
    click_beside(&mut harness);
    assert!(!shows(&harness, t.new_tally()));
    let morning = Notes::new(&device.store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Morning")
        .ok_or("the Tally's Note")?;
    assert_eq!(morning.place.as_deref(), Some(device.household.as_str()));
    let tally = Tallies::new(&device.store)
        .by_note(&morning.id)?
        .ok_or("the Tally")?;
    assert_eq!(tally.fields[0].name, "Steps");
    Ok(())
}

struct Popup {
    popup: CapturePopup,
    closed: bool,
}

/// A capture popup over `store`, "Repot the fern" typed.
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
        .get_by(|n| n.is_focused())
        .type_text("Repot the fern");
    harness.run();
    harness
}

/// The popup's page of a new Routine, from the alarm clock's menu.
fn routine_page(harness: &mut Harness<'_, Popup>, t: &L10n) {
    harness
        .get_by_role_and_label(Role::Button, "Routine, Reminder, Frog or Session")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, t.routine())
        .click_accesskit();
    harness.run();
    assert!(harness.state().popup.page_open());
}

#[test]
fn the_popups_routine_page_hands_back_a_change_on_leaving_and_asks_about_none() -> TestResult {
    let store = Arc::new({
        let now = parse_time("2026-03-02T11:00:00.000Z")?;
        let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
        OwnProfile::new(&store).apply_defaults("Desk", None)?;
        store
    });
    let t = L10n::new("en");
    let mut harness = popup(&store);
    // Unchanged, Esc takes nothing.
    routine_page(&mut harness, &t);
    esc(&mut harness);
    assert!(!harness.state().popup.page_open());
    assert!(harness.state().popup.choices().schedules.is_empty());
    // A change is handed back as Save does.
    routine_page(&mut harness, &t);
    click(&mut harness, t.repeat_weekdays());
    esc(&mut harness);
    assert!(!harness.state().popup.page_open());
    assert!(!harness.state().closed);
    assert_eq!(harness.state().popup.choices().schedules.len(), 1);
    // No weekday: asked; Esc keeps editing, Discard takes nothing.
    routine_page(&mut harness, &t);
    click(&mut harness, t.repeat_weekdays());
    click(&mut harness, t.weekday_mon());
    leave_field(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert!(shows(&harness, "Discard changes?"));
    esc(&mut harness);
    assert!(!shows(&harness, "Discard changes?"));
    assert!(harness.state().popup.page_open());
    esc(&mut harness);
    click(&mut harness, t.leave_discard());
    assert!(!harness.state().popup.page_open());
    assert_eq!(harness.state().popup.choices().schedules.len(), 1);
    Ok(())
}
