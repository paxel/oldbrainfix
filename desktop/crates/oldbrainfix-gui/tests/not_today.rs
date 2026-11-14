//! Snooze and Not today wherever the desktop answers an Occurrence
//! (#269), driven headless by egui_kittest: the Agenda row's ⋮ and
//! right-click with the snoozes, Skip and Tomorrow under their headings,
//! as far as the Occurrence allows and as the log still allows when
//! picked, a skipped Occurrence
//! struck as "Skipped" for the day and the one Tomorrow added "From the
//! day before", Ctrl+Z taking a skip back (#268); the Note dialog's Due
//! line with Snooze and Not today, Tomorrow only for an Occurrence of
//! today or later; the History's skips beside the Adherence and in the
//! Log; a card's box "08:00 · Skipped".

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{
    Alarms, ClockTime, Day, DeviceSettings, Notes, Occurrence, Recurrence, Routines, Schedule,
    SigningKey, Store, parse_time,
};
use oldbrainfix_gui::occurrence_answers::{Answer, answer as answer_now};
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

/// The window over `store` showing `selection`.
fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(selection);
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

fn offers(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next()
        .is_some()
}

/// Clicks the last button labelled `label`: a menu's over the window's,
/// "Tomorrow" the menu's and not the Agenda's day.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

/// Opens the ⋮ of the Agenda row of `title`.
fn more(harness: &mut Harness<'_, App>, title: &str) -> TestResult {
    click(harness, &format!("More for {title}"))
}

fn monday() -> Day {
    Day::new(2026, 10, 26)
}

fn at(day: Day, hour: u32) -> Occurrence {
    Occurrence::new(day, ClockTime::new(hour, 0))
}

/// The store, its clock and the Routine's id.
type Plants = (Arc<Store>, Arc<AtomicI64>, String);

/// "Water the plants" every Monday at 08:00, set up on Monday 26 October
/// 2026 at 07:00, the clock then at 08:30: its Occurrence is due.
fn plants() -> Result<Plants, Box<dyn std::error::Error>> {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store)
        .capture("Water the plants", None, None)?
        .id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=WEEKLY;BYDAY=MO").map_err(|e| format!("{e:?}"))?;
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(rule)).ok_or("a schedule")?;
    let id = Routines::new(&store).spawn(&note, &schedule)?.id;
    set(&now, "2026-10-26T08:30:00.000Z")?;
    Ok((store, now, id))
}

const PLANTS: &str = "Water the plants";

#[test]
fn the_agenda_rows_menu_snoozes_and_skips_and_ctrl_z_takes_the_skip_back() -> TestResult {
    let (store, _now, id) = plants()?;
    let mut harness = window(&store, Selection::Agenda);
    more(&mut harness, PLANTS)?;
    // Each under its heading, before History and Share.
    assert!(shows(&harness, "Snooze"));
    assert!(shows(&harness, "Not today"));
    for label in [
        "10 min", "30 min", "1 h", "Skip", "Tomorrow", "History", "Share",
    ] {
        assert!(offers(&harness, label), "{label} is offered");
    }
    let before = store.now();
    click(&mut harness, "30 min")?;
    let key = oldbrainfix_core::alarms::snooze_key(&id, &at(monday(), 8));
    let until = *Alarms::new(&store).snoozes()?.get(&key).ok_or("snoozed")?;
    assert!(until >= before + Duration::minutes(30));
    assert!(until < before + Duration::minutes(31));
    // Skip: struck for the day as skipped.
    more(&mut harness, PLANTS)?;
    click(&mut harness, "Skip")?;
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Routine")?;
    assert!(routine.skipped_at(&at(monday(), 8)).is_some());
    assert!(shows(&harness, "Skipped"));
    assert!(shows(&harness, PLANTS), "it stays on the Agenda");
    // Skipped, there is nothing more to answer.
    more(&mut harness, PLANTS)?;
    assert!(!offers(&harness, "Skip"));
    assert!(!offers(&harness, "10 min"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    // Ctrl+Z takes the skip back (#268).
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Routine")?;
    assert!(routine.skipped_at(&at(monday(), 8)).is_none());
    assert!(!shows(&harness, "Skipped"));
    Ok(())
}

#[test]
fn tomorrow_from_the_agenda_adds_the_occurrence_the_day_after() -> TestResult {
    let (store, _now, id) = plants()?;
    let mut harness = window(&store, Selection::Agenda);
    more(&mut harness, PLANTS)?;
    click(&mut harness, "Tomorrow")?;
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Routine")?;
    assert!(routine.tomorrow.contains(&at(monday(), 8).key()));
    assert_eq!(routine.moved, [at(monday().add_days(1), 8)]);
    assert!(shows(&harness, "The day after instead"));
    assert!(shows(&harness, "From the day before"));
    // Tuesday's has not started: Not today, but no Snooze yet.
    harness
        .get_all_by_role_and_label(Role::Button, &format!("More for {PLANTS}"))
        .last()
        .ok_or("Tuesday's ⋮")?
        .click();
    harness.run();
    assert!(offers(&harness, "Skip"));
    assert!(!offers(&harness, "10 min"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    // Ctrl+Z takes it back, and Tuesday's goes with it.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Routine")?;
    assert!(routine.moved.is_empty());
    assert!(!shows(&harness, "From the day before"));
    Ok(())
}

#[test]
fn a_right_click_on_a_row_opens_the_same_answers() -> TestResult {
    let (store, _now, id) = plants()?;
    let mut harness = window(&store, Selection::Agenda);
    harness
        .get_by_role_and_label(Role::Button, PLANTS)
        .click_secondary();
    harness.run();
    assert!(shows(&harness, "Not today"));
    click(&mut harness, "Skip")?;
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Routine")?;
    assert!(routine.skipped_at(&at(monday(), 8)).is_some());
    Ok(())
}

#[test]
fn a_pick_from_a_menu_read_before_a_check_off_does_nothing() -> TestResult {
    let (store, _now, id) = plants()?;
    let eight = at(monday(), 8);
    Routines::new(&store).mark_done(&id, &eight, None)?;
    let wall: &oldbrainfix_core::WallClock = &utc;
    for answer in [Answer::Skip, Answer::Tomorrow, Answer::Snooze(10)] {
        assert_eq!(answer_now(&store, wall, (&id, &eight), answer)?, None);
    }
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Routine")?;
    assert!(routine.skipped_at(&eight).is_none());
    assert!(Alarms::new(&store).snoozes()?.is_empty());
    Ok(())
}

#[test]
fn with_checked_items_hidden_a_skipped_occurrence_leaves_the_agenda() -> TestResult {
    let (store, _now, id) = plants()?;
    DeviceSettings::new(&store).set_agenda_hides_checked(true)?;
    Routines::new(&store).skip(&id, &at(monday(), 8), None)?;
    let harness = window(&store, Selection::Agenda);
    assert!(!shows(&harness, PLANTS));
    Ok(())
}

#[test]
fn the_note_dialogs_due_line_offers_snooze_and_not_today() -> TestResult {
    let (store, _now, id) = plants()?;
    let note = Routines::new(&store)
        .by_id(&id)?
        .map(|r| r.note_id)
        .ok_or("the Note")?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&note);
    harness.run();
    assert!(shows(&harness, "Due 08:00"));
    click(&mut harness, "Snooze")?;
    for label in ["10 min", "30 min", "1 h"] {
        assert!(offers(&harness, label), "{label} is offered");
    }
    assert!(!offers(&harness, "Skip"));
    click(&mut harness, "1 h")?;
    let key = oldbrainfix_core::alarms::snooze_key(&id, &at(monday(), 8));
    assert!(Alarms::new(&store).snoozes()?.contains_key(&key));
    click(&mut harness, "Not today")?;
    assert!(offers(&harness, "Tomorrow"));
    click(&mut harness, "Skip")?;
    assert!(
        !shows(&harness, "Due 08:00"),
        "skipped, it asks for nothing"
    );
    assert!(harness.state().note_open());
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert!(shows(&harness, "Due 08:00"));
    Ok(())
}

#[test]
fn a_missed_occurrence_of_an_earlier_day_offers_skip_only() -> TestResult {
    let (store, now) = store("2026-10-25T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Call mum", None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let once = Schedule::new(monday().add_days(-1), &times, None).ok_or("a schedule")?;
    let id = Routines::new(&store).spawn(&note, &once)?.id;
    set(&now, "2026-10-26T08:30:00.000Z")?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&note);
    harness.run();
    assert!(!offers(&harness, "Snooze"), "a snooze rings from now on");
    click(&mut harness, "Not today")?;
    assert!(offers(&harness, "Skip"));
    assert!(!offers(&harness, "Tomorrow"));
    click(&mut harness, "Skip")?;
    let routine = Routines::new(&store).by_id(&id)?.ok_or("the Reminder")?;
    assert!(routine.skipped_at(&at(monday().add_days(-1), 8)).is_some());
    Ok(())
}

#[test]
fn the_history_counts_skips_beside_the_adherence_and_logs_them() -> TestResult {
    let (store, now, id) = plants()?;
    let routines = Routines::new(&store);
    routines.tomorrow(&id, &at(monday(), 8), None)?;
    set(&now, "2026-10-27T09:00:00.000Z")?;
    routines.skip(&id, &at(monday().add_days(1), 8), None)?;
    let harness = window(&store, Selection::History(id.clone()));
    assert!(shows(&harness, "Skipped, the day after instead"));
    assert!(shows(&harness, "Skipped"));
    assert!(shows(&harness, "From the day before"));
    // This week, this month and the last 30 days, never in the share.
    assert_eq!(harness.query_all_by_label("· 2 skipped").count(), 3);
    assert!(!shows(&harness, "0 of 0 Done"));
    // The last seven days: Monday's and Tuesday's dots, all skipped.
    assert_eq!(harness.query_all_by_label_contains(": skipped").count(), 2);
    Ok(())
}

#[test]
fn a_cards_box_says_its_occurrence_was_skipped() -> TestResult {
    let (store, _now, id) = plants()?;
    Routines::new(&store).skip(&id, &at(monday(), 8), None)?;
    let harness = window(&store, Selection::All);
    assert!(shows(&harness, "08:00 · Skipped"));
    Ok(())
}
