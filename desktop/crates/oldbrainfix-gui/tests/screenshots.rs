//! Desktop screenshots (#45), not a test suite: the
//! window over a small demo store, drawn by egui_kittest through wgpu and
//! written as PNG files to `$SCREENSHOTS_DIR` (else
//! `target/screenshots`). The release workflow renders them and
//! attaches them to the GitHub Release, which the metainfo points at.
//! Ignored: they need a GPU or a software Vulkan such as lavapipe.
//!
//! ```sh
//! SCREENSHOTS_DIR=/tmp/shots cargo test -p oldbrainfix-gui --test screenshots -- --ignored
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use egui_kittest::Harness;
use oldbrainfix_core::{
    ClockTime, Day, Importance, Notes, Places, Routines, Schedule, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::{App, Selection};
use parking_lot::Mutex;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The file names the metainfo lists, in its order.
pub const FILES: [&str; 3] = [
    "screenshot-agenda.png",
    "screenshot-notes.png",
    "screenshot-settings.png",
];

fn out_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::var_os("SCREENSHOTS_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// A store with a few days of a household's and a job's Notes and
/// Routines, its clock at Monday 26 October 2026, 08:30 UTC.
fn demo() -> Result<(Store, Vec<String>), Box<dyn std::error::Error>> {
    let now = Arc::new(Mutex::new(parse_time("2026-10-22T17:40:00.000Z")?));
    let clock = now.clone();
    let store = Store::in_memory(Box::new(move || *clock.lock()), || {
        Ok(SigningKey::from_seed([45; 32]))
    })?;
    store.set_local_setting("author", "Sam")?;
    let places = Places::new(&store);
    let household = places.add_area("Household")?;
    places.set_icon(&household.id, Some("home"))?;
    places.set_colour(&household.id, Some("2E7D6B"))?;
    let repairs = places.add_topic(&household.id, "Repairs")?;
    let work = places.add_area("Work")?;
    places.set_icon(&work.id, Some("work"))?;
    places.set_colour(&work.id, Some("5C6BC0"))?;
    let health = places.add_area("Health")?;
    places.set_icon(&health.id, Some("health"))?;
    places.set_colour(&health.id, Some("C0504D"))?;

    let notes = Notes::new(&store);
    let mut ids = Vec::new();
    let later = |minutes: i64| {
        let mut held = now.lock();
        *held += Duration::minutes(minutes);
    };
    let filed = [
        (
            "The kitchen tap drips again. Washer size is 1/2 inch, spare ones are in the blue box in the cellar.",
            Some(repairs.id.as_str()),
        ),
        (
            "Bike lock code changed to 4-7-1-9",
            Some(household.id.as_str()),
        ),
        (
            "Ask Dana about the slides for Thursday's review",
            Some(work.id.as_str()),
        ),
        (
            "Parking at the new office: level 2, row F",
            Some(work.id.as_str()),
        ),
        (
            "Dentist said to come back in six months, call in April",
            Some(health.id.as_str()),
        ),
        (
            "Where did I put the warranty for the washing machine?",
            None,
        ),
        ("Book title Lena recommended: The Overstory", None),
    ];
    for (text, place) in filed {
        later(95);
        ids.push(notes.capture(text, place, None)?.id);
    }
    notes.set_importance(&ids[0], Importance::Important)?;
    later(30);
    notes.amend(&ids[0], "Bought two washers, 2.40 at the hardware store.")?;

    let routines = Routines::new(&store);
    let monday = Day::new(2026, 10, 26);
    let at = |h, m| ClockTime::new(h, m).ok_or("a time");
    let daily = oldbrainfix_core::Recurrence::parse("FREQ=DAILY")?;
    let weekly = oldbrainfix_core::Recurrence::parse("FREQ=WEEKLY;BYDAY=MO")?;
    let pills = notes.capture("Take the blood pressure pill", Some(&health.id), None)?;
    let schedule = Schedule::new(monday, &[at(8, 0)?, at(20, 0)?], Some(daily));
    routines.spawn(&pills.id, &schedule.ok_or("schedule")?)?;
    let bins = notes.capture("Put the bins out", Some(&household.id), None)?;
    let schedule = Schedule::new(monday, &[at(19, 0)?], Some(weekly));
    routines.spawn(&bins.id, &schedule.ok_or("schedule")?)?;
    let schedule = Schedule::new(monday, &[at(11, 30)?], None);
    routines.spawn(&ids[2], &schedule.ok_or("schedule")?)?;
    let schedule = Schedule::new(monday.add_days(1), &[at(9, 0)?], None);
    routines.create(
        "Call the garage about the winter tyres",
        &schedule.ok_or("schedule")?,
    )?;

    *now.lock() = parse_time("2026-10-26T08:30:00.000Z")?;
    let pill = routines.of_note(&pills.id)?;
    if let Some(routine) = pill.first() {
        let morning = oldbrainfix_core::Occurrence::new(monday, Some(at(8, 0)?));
        routines.mark_done(&routine.id, &morning, None)?;
    }
    Ok((store, ids))
}

fn utc(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// The window's size in the screenshots.
const SIZE: (u32, u32) = (1280, 800);
/// The margin kittest draws around the window.
const MARGIN: u32 = 8;

/// Draws the window as it stands and writes it as `name`, without the
/// harness's margin.
fn shoot(harness: &mut Harness<'static, App>, name: &str) -> TestResult {
    harness.run_steps(4);
    let image = harness.render()?;
    let window = image::imageops::crop_imm(&image, MARGIN, MARGIN, SIZE.0, SIZE.1).to_image();
    window.save(out_dir()?.join(name))?;
    Ok(())
}

#[test]
#[ignore = "needs a GPU or a software Vulkan; run by the release workflow"]
fn render_the_screenshots() -> TestResult {
    let (store, ids) = demo()?;
    let mut app = App::new(store, "en");
    app.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (SIZE.0 + 2 * MARGIN) as f32,
            (SIZE.1 + 2 * MARGIN) as f32,
        ))
        .wgpu()
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run_steps(4);
    shoot(&mut harness, FILES[0])?;

    let household = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .find(|a| a.name == "Household")
        .ok_or("the Household Area")?;
    harness.state_mut().select(Selection::Place(household.id));
    harness.state_mut().select_note(&ids[0]);
    shoot(&mut harness, FILES[1])?;

    harness.state_mut().select(Selection::Settings);
    shoot(&mut harness, FILES[2])?;
    Ok(())
}
