//! The Routine editor's Heads-ups (obf-1046), driven headless by
//! egui_kittest: under the schedule each one set, the earliest warning
//! first, with its ✕, and "Add heads-up" offering 5, 10, 15 and 30 min,
//! 1 h, 2 h, 1 day, 2 days and 1 week before, an all-day schedule whole
//! days only, which it also keeps alone. Save writes them, of a new
//! Reminder too; a Member who may not change the Routine gets the editor
//! with the rest not editable, and Save writes only the Heads-ups.

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
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, ClockTime, Day, Invite, Notes, OwnProfile, PairedDevice, Places, Routines, Schedule,
    SigningKey, Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The wall clock shows UTC.
fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// The alarm clock of a Note.
const CLOCK: &str = "Routine, Reminder, Frog or Session";

/// Every choice "Add heads-up" offers on a schedule with times.
const TIMED: [&str; 9] = [
    "5 min before",
    "10 min before",
    "15 min before",
    "30 min before",
    "1 h before",
    "2 h before",
    "1 day before",
    "2 days before",
    "1 week before",
];

/// A store with this Device's Profile, `seed` its key, whose clock runs
/// a second on with each read from 26 October 2026, 09:00.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-10-26T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(Selection::All);
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

/// The buttons whose label is among `labels`, in the order shown.
fn offered<S>(harness: &Harness<'_, S>, labels: &[&str]) -> Vec<String> {
    harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| labels.contains(&l.as_str()))
        .collect()
}

/// The Heads-ups the editor lists, top to bottom.
fn listed<S>(harness: &Harness<'_, S>) -> Vec<String> {
    let mut rows: Vec<(f32, String)> = harness
        .query_all_by_role(Role::Label)
        .filter_map(|n| {
            let label = n.accesskit_node().label()?;
            label.ends_with(" before").then(|| (n.rect().top(), label))
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, l)| l).collect()
}

/// Adds the Heads-up `label` through "Add heads-up".
fn add(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    click(harness, "Add heads-up")?;
    click(harness, label)
}

/// A Routine of a new Note `text` on `schedule`: its id.
fn routine(
    store: &Store,
    text: &str,
    schedule: &Schedule,
) -> Result<String, Box<dyn std::error::Error>> {
    let note = Notes::new(store).capture(text, None, None)?;
    Ok(Routines::new(store).spawn(&note.id, schedule)?.id)
}

/// Every day from 26 October 2026, at `times`.
fn daily(times: &[ClockTime]) -> Result<Schedule, Box<dyn std::error::Error>> {
    let rule = oldbrainfix_core::Recurrence::parse("FREQ=DAILY")?;
    Ok(Schedule::new(Day::new(2026, 10, 26), times, Some(rule)).ok_or("a schedule")?)
}

fn ten() -> Result<ClockTime, Box<dyn std::error::Error>> {
    Ok(ClockTime::new(10, 0).ok_or("10:00")?)
}

#[test]
fn a_timed_routine_offers_every_heads_up_and_saves_those_set() -> TestResult {
    let store = Arc::new(device(1, "Ann")?);
    let id = routine(&store, "Dentist", &daily(&[ten()?])?)?;
    let mut harness = window(&store);
    harness.state_mut().open_routine(&id);
    harness.run();
    assert!(harness.query_by_label("Heads-ups").is_some());
    click(&mut harness, "Add heads-up")?;
    assert_eq!(offered(&harness, &TIMED), TIMED);
    click(&mut harness, "30 min before")?;
    add(&mut harness, "1 day before")?;
    // The earliest warning first; nothing saved before Save.
    assert_eq!(listed(&harness), ["1 day before", "30 min before"]);
    assert!(Routines::new(&store).heads_ups(&id)?.is_empty());
    // One set is not offered again.
    click(&mut harness, "Add heads-up")?;
    let left = offered(&harness, &TIMED);
    assert!(
        !left
            .iter()
            .any(|l| l == "30 min before" || l == "1 day before")
    );
    assert_eq!(left.len(), 7);
    // Closed again by its button: Esc would leave the editor.
    click(&mut harness, "Add heads-up")?;
    // The ✕ of the first row, 1 day before.
    harness
        .get_all_by_role_and_label(Role::Button, "Remove heads-up")
        .next()
        .ok_or("a ✕")?
        .click();
    harness.run();
    assert_eq!(listed(&harness), ["30 min before"]);
    assert!(
        harness
            .state()
            .routine_editor()
            .is_some_and(|e| e.changed())
    );
    click(&mut harness, "Save")?;
    assert!(harness.state().routine_editor().is_none());
    assert_eq!(Routines::new(&store).heads_ups(&id)?, [30]);
    // Opened again, it lists what was saved.
    harness.state_mut().open_routine(&id);
    harness.run();
    assert_eq!(listed(&harness), ["30 min before"]);
    Ok(())
}

#[test]
fn an_all_day_routine_offers_and_keeps_whole_days_only() -> TestResult {
    let store = Arc::new(device(1, "Ann")?);
    let id = routine(&store, "Bins out", &daily(&[])?)?;
    // Set elsewhere: 30 min does not count on an all-day schedule.
    Routines::new(&store).set_heads_ups(&id, &[30, 1440])?;
    let mut harness = window(&store);
    harness.state_mut().open_routine(&id);
    harness.run();
    assert_eq!(listed(&harness), ["1 day before"]);
    click(&mut harness, "Add heads-up")?;
    let days = [
        "1 day before",
        "2 days before",
        "3 days before",
        "1 week before",
    ];
    assert_eq!(offered(&harness, &TIMED).len(), 2);
    assert_eq!(
        offered(&harness, &days),
        ["2 days before", "3 days before", "1 week before"]
    );
    assert!(offered(&harness, &["5 min before", "1 h before"]).is_empty());
    click(&mut harness, "3 days before")?;
    assert_eq!(listed(&harness), ["3 days before", "1 day before"]);
    click(&mut harness, "Save")?;
    assert_eq!(Routines::new(&store).heads_ups(&id)?, [1440, 4320]);
    Ok(())
}

#[test]
fn a_new_reminder_saves_its_heads_ups_once_made() -> TestResult {
    let store = Arc::new(device(1, "Ann")?);
    let note = Notes::new(&store).capture("Call the dentist", None, None)?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness.get_by_label(CLOCK).click();
    harness.run();
    click(&mut harness, "Remind me…")?;
    add(&mut harness, "15 min before")?;
    assert_eq!(listed(&harness), ["15 min before"]);
    click(&mut harness, "Save")?;
    let made = Routines::new(&store).of_note(&note.id)?;
    assert_eq!(made.len(), 1);
    assert_eq!(Routines::new(&store).heads_ups(&made[0].id)?, [15]);
    Ok(())
}

/// Ann's Household with her Routine "Water the plants" every day at
/// 10:00, shared with Ben as a Member: Ben's store and the Routine's id.
fn shared() -> Result<(Store, String), Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let note = Notes::new(&ann).capture("Water the plants", Some(&house.id), None)?;
    let id = Routines::new(&ann).spawn(&note.id, &daily(&[ten()?])?)?.id;
    Routines::new(&ann).set_renag(&id, Some(Duration::minutes(15)))?;
    let area = ann.share(&house.id)?;
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let invite = Invite {
        area,
        role: AreaRole::Member,
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    ben.remember_device(&PairedDevice {
        id: ann.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: ben.now(),
        shared: true,
    })?;
    Ok((ben, id))
}

#[test]
fn a_member_sets_only_their_own_heads_ups() -> TestResult {
    let (ben, id) = shared()?;
    assert!(!ben.may_change_routine(&id)?);
    let before = Routines::new(&ben).by_id(&id)?.ok_or("the Routine")?;
    let ben = Arc::new(ben);
    let mut harness = window(&ben);
    harness.state_mut().open_routine(&id);
    harness.run();
    // The editor, not the Note: the rest shown, not editable.
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert!(editor.locked());
    assert!(harness.query_by_label("Delete").is_none());
    let add_time = harness.get_by_role_and_label(Role::Button, "Add a time");
    assert!(add_time.accesskit_node().is_disabled());
    add(&mut harness, "2 h before")?;
    assert_eq!(listed(&harness), ["2 h before"]);
    click(&mut harness, "Save")?;
    assert!(harness.state().routine_editor().is_none());
    let routines = Routines::new(&ben);
    assert_eq!(routines.heads_ups(&id)?, [120]);
    // Schedule, Re-nag and Escalation as they were.
    let after = routines.by_id(&id)?.ok_or("the Routine")?;
    assert_eq!(after.schedule, before.schedule);
    assert_eq!(after.renag, before.renag);
    assert_eq!(after.escalation, before.escalation);
    Ok(())
}
