//! A new Reminder's chips under the When line (#271), driven headless by
//! egui_kittest over a store whose clock is set by the test: In 1 h,
//! Tonight, Tomorrow and In a week, each with its time, Tomorrow and In a
//! week when this Device's Quiet Hours end (09:00 while they are off),
//! Tonight only until 19:00. One click sets the schedule and Save writes
//! it; a chip is read again at the click. A new Reminder from a Note, a
//! new Routine set to Once and the quick note's alarm clock, in the
//! window and the popup, offer them; a saved Reminder, a Session, an
//! Advanced schedule and a repeat do not.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    ClockTime, Day, DeviceSettings, Note, Notes, OwnProfile, QuietHours, Routine, Routines,
    Schedule, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The wall clock shows UTC.
fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// Monday, 26 October 2026, 14:20.
const NOW: &str = "2026-10-26T14:20:00.000Z";

/// The alarm clock of a Note and of the quick note.
const CLOCK: &str = "Routine, Reminder, Frog or Session";

/// The starts of the chips' labels, in their order.
const CHIPS: [&str; 4] = ["In 1 h, ", "Tonight ", "Tomorrow ", "In a week, "];

/// An empty store with this Device's Profile whose clock shows `at` until
/// it is set again.
fn store(at: &str) -> Result<(Arc<Store>, Arc<AtomicI64>), Box<dyn std::error::Error>> {
    let now = Arc::new(AtomicI64::new(parse_time(at)?.timestamp_millis()));
    let held = now.clone();
    let clock =
        move || DateTime::from_timestamp_millis(held.load(Ordering::Relaxed)).unwrap_or_default();
    let store = Store::in_memory(Box::new(clock), || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok((Arc::new(store), now))
}

/// Sets the store's clock to `at`.
fn set(clock: &AtomicI64, at: &str) -> TestResult {
    clock.store(parse_time(at)?.timestamp_millis(), Ordering::Relaxed);
    Ok(())
}

fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(selection);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2000.0))
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

fn click<S>(harness: &mut Harness<'_, S>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

/// Picks the radio button `label`, as Once.
fn pick(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .click();
    harness.run();
}

/// The chips' labels, in their order.
fn chips<S>(harness: &Harness<'_, S>) -> Vec<String> {
    harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| CHIPS.iter().any(|c| l.starts_with(c)))
        .collect()
}

/// A shortcut, not a choice: no chip is shown as picked.
fn none_picked(harness: &Harness<'_, App>) -> bool {
    harness
        .query_all_by_role(Role::Button)
        .filter(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| CHIPS.iter().any(|c| l.starts_with(c)))
        })
        .all(|n| {
            n.accesskit_node().toggled().is_none()
                && !n.accesskit_node().is_selected().unwrap_or(false)
        })
}

/// The window on the Note "Call the dentist" with its new Reminder's
/// editor open, from the Note's alarm clock.
fn reminder(
    store: &Arc<Store>,
) -> Result<(Harness<'static, App>, Note), Box<dyn std::error::Error>> {
    let note = Notes::new(store).capture("Call the dentist", None, None)?;
    let mut harness = window(store, Selection::All);
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness.get_by_label(CLOCK).click();
    harness.run();
    click(&mut harness, "Reminder")?;
    assert!(harness.state().routine_editor().is_some());
    Ok((harness, note))
}

/// The schedule in the editor.
fn edited(harness: &Harness<'_, App>) -> Result<Schedule, Box<dyn std::error::Error>> {
    Ok(harness
        .state()
        .routine_editor()
        .ok_or("the editor")?
        .schedule()
        .ok_or("a schedule")?
        .clone())
}

/// Once on `day` at `hour`:`minute`.
fn once(day: Day, hour: u32, minute: u32) -> Result<Schedule, Box<dyn std::error::Error>> {
    let time = ClockTime::new(hour, minute).ok_or("a time")?;
    Ok(Schedule::new(day, &[time], None).ok_or("a schedule")?)
}

fn reminders_of(store: &Store, note: &str) -> Result<Vec<Routine>, Box<dyn std::error::Error>> {
    Ok(Routines::new(store).of_note(note)?)
}

#[test]
fn a_new_reminder_offers_the_four_chips_each_with_its_time() -> TestResult {
    let (store, _) = store(NOW)?;
    let (harness, _) = reminder(&store)?;
    // The default Quiet Hours end at 08:00.
    assert_eq!(
        chips(&harness),
        [
            "In 1 h, 15:20",
            "Tonight 19:00",
            "Tomorrow 08:00",
            "In a week, 08:00"
        ]
    );
    assert!(none_picked(&harness));
    Ok(())
}

#[test]
fn a_click_on_tomorrow_sets_it_and_save_writes_it() -> TestResult {
    let (store, _) = store(NOW)?;
    let (mut harness, note) = reminder(&store)?;
    click(&mut harness, "Tomorrow 08:00")?;
    assert_eq!(edited(&harness)?, once(Day::new(2026, 10, 27), 8, 0)?);
    // The click only sets it.
    assert!(reminders_of(&store, &note.id)?.is_empty());
    assert!(none_picked(&harness));
    click(&mut harness, "Save")?;
    let saved = reminders_of(&store, &note.id)?;
    assert_eq!(saved.len(), 1);
    assert!(saved[0].is_reminder());
    assert_eq!(saved[0].schedule, once(Day::new(2026, 10, 27), 8, 0)?);
    Ok(())
}

#[test]
fn tomorrow_and_in_a_week_follow_this_devices_quiet_hours() -> TestResult {
    let (store, _) = store(NOW)?;
    let quiet = QuietHours {
        start: ClockTime::new(22, 0).ok_or("a time")?,
        end: ClockTime::new(7, 30).ok_or("a time")?,
    };
    DeviceSettings::new(&store).set_quiet_hours(quiet)?;
    let (mut harness, _) = reminder(&store)?;
    let shown = chips(&harness);
    assert!(shown.contains(&"Tomorrow 07:30".to_string()), "{shown:?}");
    assert!(shown.contains(&"In a week, 07:30".to_string()), "{shown:?}");
    click(&mut harness, "In a week, 07:30")?;
    assert_eq!(edited(&harness)?, once(Day::new(2026, 11, 2), 7, 30)?);

    // While they are off, the day starts at 09:00.
    DeviceSettings::new(&store).set_quiet_hours_on(false)?;
    harness.run();
    let shown = chips(&harness);
    assert!(shown.contains(&"Tomorrow 09:00".to_string()), "{shown:?}");
    click(&mut harness, "Tomorrow 09:00")?;
    assert_eq!(edited(&harness)?, once(Day::new(2026, 10, 27), 9, 0)?);
    Ok(())
}

#[test]
fn in_1_h_carries_into_the_next_day_and_tonight_goes_at_19() -> TestResult {
    let (store, _) = store("2026-10-26T23:30:00.000Z")?;
    let (mut harness, _) = reminder(&store)?;
    assert_eq!(
        chips(&harness),
        ["In 1 h, 00:30", "Tomorrow 08:00", "In a week, 08:00"]
    );
    click(&mut harness, "In 1 h, 00:30")?;
    assert_eq!(edited(&harness)?, once(Day::new(2026, 10, 27), 0, 30)?);
    Ok(())
}

#[test]
fn a_chip_is_read_again_at_the_click() -> TestResult {
    let (afternoon, clock) = store(NOW)?;
    let (mut harness, _) = reminder(&afternoon)?;
    assert_eq!(chips(&harness)[0], "In 1 h, 15:20");
    // A quarter of an hour passes before the click.
    set(&clock, "2026-10-26T14:35:00.000Z")?;
    click(&mut harness, "In 1 h, 15:20")?;
    assert_eq!(edited(&harness)?, once(Day::new(2026, 10, 26), 15, 35)?);
    assert_eq!(chips(&harness)[0], "In 1 h, 15:35");

    // Tonight clicked after 19:00 only goes and sets nothing.
    let (evening, clock) = store("2026-10-26T18:50:00.000Z")?;
    let (mut harness, _) = reminder(&evening)?;
    let before = edited(&harness)?;
    assert_eq!(
        before,
        once(Day::new(2026, 10, 26), 19, 0)?,
        "the next full hour"
    );
    set(&clock, "2026-10-26T19:05:00.000Z")?;
    click(&mut harness, "Tonight 19:00")?;
    assert_eq!(edited(&harness)?, before);
    assert!(!chips(&harness).iter().any(|c| c.starts_with("Tonight")));
    Ok(())
}

#[test]
fn a_new_routine_offers_the_chips_once_set_to_once_in_simple() -> TestResult {
    let (store, _) = store(NOW)?;
    let mut harness = window(&store, Selection::Agenda);
    harness.get_by_label("New Routine or Reminder").click();
    harness.run();
    assert!(harness.state().routine_editor().is_some());
    // A new Routine starts every day: no chips.
    assert!(chips(&harness).is_empty());
    pick(&mut harness, "Once");
    assert_eq!(chips(&harness).len(), 4);
    pick(&mut harness, "Advanced");
    assert!(chips(&harness).is_empty());
    pick(&mut harness, "Simple");
    assert_eq!(chips(&harness).len(), 4);
    pick(&mut harness, "Every …");
    assert!(chips(&harness).is_empty());
    Ok(())
}

#[test]
fn a_saved_reminder_and_a_new_session_offer_none() -> TestResult {
    let (store, _) = store(NOW)?;
    let schedule = once(Day::new(2026, 10, 27), 8, 0)?;
    let saved = Routines::new(&store).create("Dentist", &schedule)?;
    let mut harness = window(&store, Selection::Agenda);
    harness.state_mut().open_routine(&saved.id);
    harness.run();
    assert!(harness.state().routine_editor().is_some());
    assert!(chips(&harness).is_empty());

    // A Session, set to Once too.
    let note = Notes::new(&store).capture("Leg day", None, None)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness.get_by_label(CLOCK).click();
    harness.run();
    click(&mut harness, "Session")?;
    assert!(harness.query_all_by_label("New Session").next().is_some());
    pick(&mut harness, "Once");
    assert!(chips(&harness).is_empty());
    Ok(())
}

#[test]
fn the_quick_notes_alarm_clock_takes_a_chip_with_the_note() -> TestResult {
    let (store, _) = store(NOW)?;
    let mut harness = window(&store, Selection::Agenda);
    harness.state_mut().new_note();
    harness.run();
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Buy stamps");
    harness.run();
    harness.get_by_role_and_label(Role::Button, CLOCK).click();
    harness.run();
    click(&mut harness, "Reminder")?;
    click(&mut harness, "Tonight 19:00")?;
    click(&mut harness, "Save")?;
    assert!(harness.state().routine_editor().is_none());
    assert!(Routines::new(&store).all()?.is_empty());
    // Back in the quick note's field, Ctrl+Enter saves both.
    harness
        .get_by(|n| {
            n.role() == Role::MultilineTextInput && n.value().as_deref() == Some("Buy stamps")
        })
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    harness.run();
    let note = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Buy stamps")
        .ok_or("the Note")?;
    let saved = reminders_of(&store, &note.id)?;
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].schedule, once(Day::new(2026, 10, 26), 19, 0)?);
    Ok(())
}

struct Popup {
    popup: CapturePopup,
    outcome: Outcome,
}

#[test]
fn the_popups_alarm_clock_takes_a_chip_with_the_note() -> TestResult {
    let (store, _) = store(NOW)?;
    let mut popup = CapturePopup::new(store.clone(), "en");
    popup.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 1400.0))
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
        .get_by(|n| n.is_focused())
        .type_text("Call the dentist");
    harness.run();
    harness.get_by_role_and_label(Role::Button, CLOCK).click();
    harness.run();
    click(&mut harness, "Reminder")?;
    assert_eq!(chips(&harness).len(), 4);
    click(&mut harness, "Tomorrow 08:00")?;
    click(&mut harness, "Save")?;
    assert!(!harness.state().popup.page_open());
    click(&mut harness, "Save")?;
    assert_eq!(harness.state().outcome, Outcome::Closed);
    let note = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Call the dentist")
        .ok_or("the Note")?;
    let saved = reminders_of(&store, &note.id)?;
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].schedule, once(Day::new(2026, 10, 27), 8, 0)?);
    Ok(())
}
