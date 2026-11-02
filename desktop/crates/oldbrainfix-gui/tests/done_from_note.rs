//! Done from the Note and from its cards (#215), driven headless by
//! egui_kittest: at the top of the Note dialog the Occurrence the Note
//! asks to check off, "Due 08:00" or "Missed" with its day, with Done,
//! and for a one-time Reminder, the Note's only one, "Done and move to
//! trash"; on its card a box for today's Occurrence of each of its
//! Routines and Reminders that ticks it as the Agenda row's does and opens
//! nothing.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    ClockTime, Day, Notes, Occurrence, Recurrence, Routines, Schedule, SigningKey, Store,
    parse_time,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// An empty store whose clock shows `at` until it is set again, a
/// millisecond on with every look.
fn store(at: &str) -> Result<(Arc<Store>, Arc<AtomicI64>), Box<dyn std::error::Error>> {
    let now = Arc::new(AtomicI64::new(parse_time(at)?.timestamp_millis()));
    let held = now.clone();
    let clock = move || {
        let ms = held.fetch_add(1, Ordering::Relaxed);
        DateTime::from_timestamp_millis(ms).unwrap_or_default()
    };
    let store = Store::in_memory(Box::new(clock), || Ok(SigningKey::from_seed([7; 32])))?;
    Ok((Arc::new(store), now))
}

fn set(now: &AtomicI64, at: &str) -> TestResult {
    now.store(parse_time(at)?.timestamp_millis(), Ordering::Relaxed);
    Ok(())
}

/// The window over `store` on All Notes.
fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
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

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Clicks the last button labelled `label`, the dialog's over the
/// window's.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

fn monday() -> Day {
    Day::new(2026, 10, 26)
}

fn at(day: Day, hour: u32) -> Occurrence {
    Occurrence::new(day, ClockTime::new(hour, 0))
}

fn reminder(day: Day, hour: u32) -> Result<Schedule, Box<dyn std::error::Error>> {
    let times: Vec<ClockTime> = ClockTime::new(hour, 0).into_iter().collect();
    Ok(Schedule::new(day, &times, None).ok_or("a schedule")?)
}

fn daily(hours: &[u32]) -> Result<Schedule, Box<dyn std::error::Error>> {
    let times: Vec<ClockTime> = hours.iter().filter_map(|h| ClockTime::new(*h, 0)).collect();
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e:?}"))?;
    Ok(Schedule::new(Day::new(2026, 10, 1), &times, Some(rule)).ok_or("a schedule")?)
}

#[test]
fn a_reminder_due_offers_done_in_the_note_dialog_which_checks_it_off() -> TestResult {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Call mum", None, None)?.id;
    let id = Routines::new(&store)
        .spawn(&note, &reminder(monday(), 8)?)?
        .id;
    set(&now, "2026-10-26T09:00:00.000Z")?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&note);
    harness.run();
    assert!(shows(&harness, "Due 08:00"));
    assert!(shows(&harness, "Done and move to trash"));
    click(&mut harness, "Done")?;
    let done = Routines::new(&store).by_id(&id)?.ok_or("the Reminder")?;
    assert!(done.done_at(&at(monday(), 8)).is_some());
    assert!(!shows(&harness, "Due 08:00"));
    assert!(harness.state().note_open(), "Done keeps the dialog open");
    Ok(())
}

#[test]
fn done_and_move_to_trash_trashes_the_note_with_its_reminder_and_closes_the_dialog() -> TestResult {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Call mum", None, None)?.id;
    let id = Routines::new(&store)
        .spawn(&note, &reminder(monday(), 8)?)?
        .id;
    set(&now, "2026-10-26T09:00:00.000Z")?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&note);
    harness.run();
    click(&mut harness, "Done and move to trash")?;
    assert!(!harness.state().note_open());
    let trashed = Notes::new(&store).by_id(&note)?.ok_or("the Note")?;
    assert!(trashed.is_trashed());
    let read = Routines::new(&store).by_id(&id)?.ok_or("the Reminder")?;
    assert!(read.done_at(&at(monday(), 8)).is_some());
    assert_eq!(read.trashed_with.as_deref(), Some(note.as_str()));
    Ok(())
}

#[test]
fn a_routine_shows_its_latest_missed_one_with_done_only() -> TestResult {
    let (store, now) = store("2026-10-24T06:00:00.000Z")?;
    let note = Notes::new(&store).capture("Pills", None, None)?.id;
    let routines = Routines::new(&store);
    let id = routines.spawn(&note, &daily(&[8])?)?.id;
    set(&now, "2026-10-26T09:00:00.000Z")?;
    routines.mark_done(&id, &at(monday(), 8), None)?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&note);
    harness.run();
    // A label's words, as the screen reader reads them.
    let missed = harness
        .query_all_by(|_| true)
        .filter_map(|n| {
            let node = n.accesskit_node();
            node.label().or(node.value())
        })
        .find(|l| l.starts_with("Missed "))
        .ok_or("no Missed line")?;
    assert_eq!(missed, "Missed Sun, 10/25/2026 08:00");
    assert!(!shows(&harness, "Done and move to trash"), "it repeats");
    click(&mut harness, "Done")?;
    let read = routines.by_id(&id)?.ok_or("the Routine")?;
    assert!(read.done_at(&at(Day::new(2026, 10, 25), 8)).is_some());
    Ok(())
}

#[test]
fn a_reminder_beside_another_routine_has_no_trash_and_one_not_due_shows_nothing() -> TestResult {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let notes = Notes::new(&store);
    let routines = Routines::new(&store);
    let dentist = notes.capture("Dentist", None, None)?.id;
    routines.spawn(&dentist, &reminder(monday(), 8)?)?;
    routines.spawn(&dentist, &daily(&[15])?)?;
    let later = notes.capture("Later", None, None)?.id;
    routines.spawn(&later, &reminder(monday(), 15)?)?;
    set(&now, "2026-10-26T09:00:00.000Z")?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&dentist);
    harness.run();
    assert!(shows(&harness, "Due 08:00"));
    assert!(!shows(&harness, "Done and move to trash"));
    harness.state_mut().select_note(&later);
    harness.run();
    assert!(harness.state().note_open());
    assert!(!shows(&harness, "Due 15:00"));
    Ok(())
}

fn ticked(harness: &Harness<'_, App>, label: &str) -> Option<bool> {
    harness
        .query_all_by(|n| n.role() == Role::CheckBox && n.label().as_deref() == Some(label))
        .next()
        .map(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

fn tick(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by(|n| n.role() == Role::CheckBox && n.label().as_deref() == Some(label))
        .next()
        .ok_or(format!("no box {label}"))?
        .click();
    harness.run();
    Ok(())
}

#[test]
fn a_cards_box_ticks_todays_open_occurrence_and_unchecks_it_again() -> TestResult {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Pills", None, None)?.id;
    let routines = Routines::new(&store);
    let id = routines.spawn(&note, &daily(&[8, 20])?)?.id;
    set(&now, "2026-10-26T09:00:00.000Z")?;
    let mut harness = window(&store);
    assert_eq!(ticked(&harness, "Done: 08:00"), Some(false));
    tick(&mut harness, "Done: 08:00")?;
    let read = routines.by_id(&id)?.ok_or("the Routine")?;
    assert!(read.done_at(&at(monday(), 8)).is_some());
    assert!(!harness.state().note_open(), "it opens nothing");
    // The next one today is open now.
    assert_eq!(ticked(&harness, "Done: 20:00"), Some(false));
    tick(&mut harness, "Done: 20:00")?;
    assert_eq!(
        ticked(&harness, "Done: 20:00"),
        Some(true),
        "Done today, there all day"
    );
    tick(&mut harness, "Done: 20:00")?;
    let read = routines.by_id(&id)?.ok_or("the Routine")?;
    assert!(read.done_at(&at(monday(), 20)).is_none());
    Ok(())
}
