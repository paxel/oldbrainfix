//! A click on an Occurrence opens its Note (#270), driven headless by
//! egui_kittest: a click on an Agenda row's text, or on a Checklist
//! Routine's Line's words, opens the Note dialog of its Note with its
//! Due line, not the Routine editor; "Edit schedule" in the row's ⋮ and
//! right-click opens the Routine editor; a Note in the trash leaves the
//! Routine editor to open. A click on an Occurrence's notification opens
//! the Note dialog too, keeping a Routine editor open over it, and opens
//! nothing for a Routine or Note no longer there.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::schedule::Frequency;
use oldbrainfix_core::{
    Checklists, ClockTime, Day, Notes, Recurrence, Routines, Schedule, SigningKey, Store,
    parse_time,
};
use oldbrainfix_gui::alerts::OccurrenceWanted;
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

/// The window over `store` on the Agenda, the click on a notification
/// coming through `wanted`.
fn window(store: &Arc<Store>, wanted: &OccurrenceWanted) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.set_occurrence_wanted(wanted.clone());
    app.select(Selection::Agenda);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1800.0))
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

/// Clicks the first button labelled `label`, today's row.
fn click_first(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .next()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

/// Clicks the last button labelled `label`, a menu's over the window's.
fn click_last(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

const PLANTS: &str = "Water the plants";

/// "Water the plants" every Monday at 08:00, set up on Monday 26 October
/// 2026 at 07:00, the clock then at 08:30: due. The store, the Note's id
/// and the Routine's id.
fn plants() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture(PLANTS, None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=WEEKLY;BYDAY=MO").map_err(|e| format!("{e:?}"))?;
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(rule)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&note, &schedule)?.id;
    now.store(
        parse_time("2026-10-26T08:30:00.000Z")?.timestamp_millis(),
        Ordering::Relaxed,
    );
    Ok((store, note, routine))
}

fn slot() -> OccurrenceWanted {
    Arc::new(Mutex::new(None))
}

#[test]
fn a_click_on_an_agenda_row_opens_its_note_with_the_due_line() -> TestResult {
    let (store, note, _) = plants()?;
    let mut harness = window(&store, &slot());
    click_first(&mut harness, PLANTS)?;
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    assert!(harness.state().note_open());
    assert!(harness.state().routine_editor().is_none());
    // The Due line answers it at the top.
    assert!(shows(&harness, "Due 08:00"));
    Ok(())
}

#[test]
fn edit_schedule_in_the_rows_menu_opens_the_routine_editor() -> TestResult {
    let (store, _, routine) = plants()?;
    let mut harness = window(&store, &slot());
    click_last(&mut harness, &format!("More for {PLANTS}"))?;
    // After the answers, before History and Share.
    let labels: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    let at = |label: &str| labels.iter().rposition(|l| l == label);
    let schedule = at("Edit schedule").ok_or("Edit schedule")?;
    assert!(at("Tomorrow").is_some_and(|i| i < schedule));
    assert!(at("History").is_some_and(|i| schedule < i));
    click_last(&mut harness, "Edit schedule")?;
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert_eq!(editor.routine.as_deref(), Some(routine.as_str()));

    // The right-click offers it too.
    let mut harness = window(&store, &slot());
    harness
        .get_all_by_role_and_label(Role::Button, PLANTS)
        .next()
        .ok_or("the row")?
        .click_secondary();
    harness.run();
    click_last(&mut harness, "Edit schedule")?;
    assert!(harness.state().routine_editor().is_some());
    Ok(())
}

#[test]
fn a_row_whose_note_is_in_the_trash_opens_its_routine() -> TestResult {
    let (store, note, routine) = plants()?;
    Notes::new(&store).trash(&note)?;
    let mut harness = window(&store, &slot());
    click_first(&mut harness, PLANTS)?;
    assert!(!harness.state().note_open());
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert_eq!(editor.routine.as_deref(), Some(routine.as_str()));
    Ok(())
}

#[test]
fn a_checklist_routines_line_opens_its_note() -> TestResult {
    let (store, _) = store("2026-10-24T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Packing\nBath\nTent\nMap", None, None)?;
    Checklists::new(&store).make(&note.id)?;
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 24), &[], Some(daily)).ok_or("a schedule")?;
    Routines::new(&store).spawn(&note.id, &schedule)?;
    let mut harness = window(&store, &slot());
    click_first(&mut harness, "Tent")?;
    assert_eq!(harness.state().selected_note(), Some(note.id.as_str()));
    assert!(harness.state().routine_editor().is_none());
    Ok(())
}

#[test]
fn a_click_on_an_occurrences_notification_opens_its_note() -> TestResult {
    let (store, note, routine) = plants()?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    assert!(!harness.state().note_open());
    *wanted.lock().map_err(|e| e.to_string())? = Some(routine);
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    assert!(shows(&harness, "Due 08:00"));
    assert!(wanted.lock().map_err(|e| e.to_string())?.is_none());
    Ok(())
}

#[test]
fn a_notification_keeps_an_open_routine_editor_over_the_note() -> TestResult {
    let (store, note, routine) = plants()?;
    let other = Notes::new(&store).capture("Feed the cat", None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(18, 0).into_iter().collect();
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(daily)).ok_or("a schedule")?;
    let cat = Routines::new(&store).spawn(&other, &schedule)?.id;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    harness.state_mut().open_routine(&cat);
    harness.run();
    *wanted.lock().map_err(|e| e.to_string())? = Some(routine);
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    let editor = harness.state().routine_editor().ok_or("the editor kept")?;
    assert_eq!(editor.routine.as_deref(), Some(cat.as_str()));
    Ok(())
}

#[test]
fn a_notification_of_a_note_in_the_trash_or_gone_opens_nothing() -> TestResult {
    let (store, note, routine) = plants()?;
    Notes::new(&store).trash(&note)?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    *wanted.lock().map_err(|e| e.to_string())? = Some(routine);
    harness.run();
    assert!(!harness.state().note_open());
    assert!(harness.state().routine_editor().is_none());
    *wanted.lock().map_err(|e| e.to_string())? = Some("gone".into());
    harness.run();
    assert!(!harness.state().note_open());
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    Ok(())
}
