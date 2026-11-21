//! A quiet day on the desktop's Agenda (obf-1044), driven headless by
//! egui_kittest: "Quiet today" in the Agenda's ⋮, the line "Quiet day
//! until midnight" first on the Agenda with End, today's Occurrences
//! still there, and End taking the line away and bringing "Quiet today"
//! back.

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
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{
    ClockTime, Day, Notes, PersonSettings, Recurrence, Routines, Schedule, SigningKey, Store,
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

/// The window over `store` on the Agenda.
fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
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

/// Whether anything says `text`: a button as its label, a Label as its
/// value (egui gives a Label its text as the value, not the label).
fn says(harness: &Harness<'_, App>, text: &str) -> bool {
    harness
        .query_all_by(|n| n.label().as_deref() == Some(text) || n.value().as_deref() == Some(text))
        .next()
        .is_some()
}

/// The top of the Label that says `text`.
fn top_of(harness: &Harness<'_, App>, text: &str) -> Option<f32> {
    harness
        .query_all_by_value(text)
        .next()
        .map(|n| n.rect().top())
}

fn offers(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next()
        .is_some()
}

/// Clicks the last button labelled `label`: a menu's over the window's.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

/// The Agenda's ⋮.
const MENU: &str = "More for Agenda";
const LINE: &str = "Quiet day until midnight";
const PLANTS: &str = "Water the plants";

/// "Water the plants" every Monday at 08:00, set up on Monday 26 October
/// 2026 at 07:00, the clock then at 08:30: its Occurrence is due.
fn plants() -> Result<(Arc<Store>, Arc<AtomicI64>), Box<dyn std::error::Error>> {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture(PLANTS, None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=WEEKLY;BYDAY=MO").map_err(|e| format!("{e:?}"))?;
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(rule)).ok_or("a schedule")?;
    Routines::new(&store).spawn(&note, &schedule)?;
    set(&now, "2026-10-26T08:30:00.000Z")?;
    Ok((store, now))
}

#[test]
fn quiet_today_shows_its_line_on_top_and_end_brings_the_menu_entry_back() -> TestResult {
    let (store, _now) = plants()?;
    let mut harness = window(&store);
    let wall = utc;
    assert!(!says(&harness, LINE));
    assert!(says(&harness, PLANTS));

    // "Quiet today" in the Agenda's ⋮.
    click(&mut harness, MENU)?;
    assert!(offers(&harness, "Quiet today"));
    click(&mut harness, "Quiet today")?;
    assert!(PersonSettings::new(&store).quiet_until(&wall)?.is_some());

    // The line first of all, today's Occurrence still there, and no ⋮
    // with nothing left to offer.
    assert!(says(&harness, LINE));
    assert!(offers(&harness, "End"));
    let line = top_of(&harness, LINE).ok_or("the line")?;
    let today = top_of(&harness, "Today").ok_or("Today")?;
    assert!(line < today, "the line is above today's rows");
    assert!(says(&harness, PLANTS), "today's Occurrence stays");
    assert!(!offers(&harness, MENU));

    // End: the line goes, "Quiet today" is back.
    click(&mut harness, "End")?;
    assert!(PersonSettings::new(&store).quiet_until(&wall)?.is_none());
    assert!(!says(&harness, LINE));
    assert!(says(&harness, PLANTS));
    click(&mut harness, MENU)?;
    assert!(offers(&harness, "Quiet today"));
    Ok(())
}

#[test]
fn a_quiet_day_set_elsewhere_shows_its_line_and_it_goes_at_midnight() -> TestResult {
    let (store, now) = plants()?;
    let mut harness = window(&store);
    assert!(!says(&harness, LINE));
    // As if it arrived by sync: the Agenda shows it once it reads again.
    PersonSettings::new(&store).quiet_today(&utc)?;
    set(&now, "2026-10-26T08:31:00.000Z")?;
    harness.run();
    assert!(says(&harness, LINE));
    assert!(!offers(&harness, MENU));
    // Midnight ends it by itself.
    set(&now, "2026-10-27T00:00:30.000Z")?;
    harness.run();
    assert!(!says(&harness, LINE));
    assert!(offers(&harness, MENU));
    Ok(())
}
