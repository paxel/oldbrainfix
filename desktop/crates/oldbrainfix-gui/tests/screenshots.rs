//! Desktop screenshots (#45), not a test suite: the
//! window over a small demo store, drawn by egui_kittest through wgpu and
//! written as PNG files to `$SCREENSHOTS_DIR` (else
//! `target/screenshots`). The release workflow renders them and
//! attaches them to the GitHub Release, which the metainfo points at.
//! Ignored: they need a GPU or a software Vulkan such as lavapipe.
//!
//! ```sh
//! SCREENSHOTS_DIR=/tmp/shots cargo test -p oldbrainfix-gui --test screenshots -- --ignored render_the_screenshots
//! ```
//!
//! The user guide's desktop frames (#134) come from the same demo store:
//! each screen in English and German, light and dark, under
//! `site/assets/frames/desktop/<language>/<theme>/`. They are committed; a
//! run compares each frame with the committed one and fails when they
//! differ, and `UPDATE_FRAMES=1` writes them instead
//! (`tool/guide-frames.sh`):
//!
//! ```sh
//! cargo test -p oldbrainfix-gui --test screenshots -- --ignored guide_frames
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
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

/// The demo store's words in one language.
struct Texts {
    household: &'static str,
    repairs: &'static str,
    work: &'static str,
    car: &'static str,
    /// The filed Notes: the tap, the bike lock, Dana, the parking, the
    /// tyres, and two Unsorted.
    filed: [&'static str; 7],
    washers: &'static str,
    snack: &'static str,
    bins: &'static str,
    garage: &'static str,
    /// The capture field's hint, and a thought typed into it.
    hint: &'static str,
    capture: &'static str,
    /// A Note with a typo, and its text as an AI Task fixes it.
    typo: &'static str,
    fixed: &'static str,
}

const EN: Texts = Texts {
    household: "Household",
    repairs: "Repairs",
    work: "Work",
    car: "Car",
    filed: [
        "The kitchen tap drips again. Washer size is 1/2 inch, spare ones are in the blue box in the cellar.",
        "Bike lock code changed to 4-7-1-9",
        "Ask Dana about the slides for Thursday's review",
        "Parking at the new office: level 2, row F",
        "Winter tyres are in the cellar, left shelf, with the wheel bolts",
        "Where did I put the warranty for the washing machine?",
        "Book title Lena recommended: The Overstory",
    ],
    washers: "Bought two washers, 2.40 at the hardware store.",
    snack: "Snack: an apple and a handful of nuts",
    bins: "Put the bins out",
    garage: "Call the garage about the winter tyres",
    hint: "What's on your mind?",
    capture: "Ask Dana if she still needs the ladder",
    typo: "Pick up the parcel for Dana at the post office tomorow",
    fixed: "Pick up the parcel for Dana at the post office tomorrow",
};

const DE: Texts = Texts {
    household: "Haushalt",
    repairs: "Reparaturen",
    work: "Arbeit",
    car: "Auto",
    filed: [
        "Der Küchenhahn tropft wieder. Dichtung ist 1/2 Zoll, Ersatz liegt in der blauen Kiste im Keller.",
        "Code vom Fahrradschloss geändert auf 4-7-1-9",
        "Dana wegen der Folien für die Besprechung am Donnerstag fragen",
        "Parken am neuen Büro: Ebene 2, Reihe F",
        "Winterreifen liegen im Keller, linkes Regal, mit den Radschrauben",
        "Wo ist die Garantie für die Waschmaschine?",
        "Buchtipp von Lena: Die Wurzeln des Lebens",
    ],
    washers: "Zwei Dichtungen gekauft, 2,40 im Baumarkt.",
    snack: "Snack: ein Apfel und eine Handvoll Nüsse",
    bins: "Mülltonnen rausstellen",
    garage: "Werkstatt wegen der Winterreifen anrufen",
    hint: "Was geht dir durch den Kopf?",
    capture: "Dana fragen, ob sie die Leiter noch braucht",
    typo: "Packet für Dana morgen bei der Post abhohlen",
    fixed: "Paket für Dana morgen bei der Post abholen",
};

/// A store with a few days of a household's and a job's Notes and
/// Routines in `x`'s words, its clock at Monday 26 October 2026, 08:30 UTC.
fn demo(x: &Texts) -> Result<(Store, Vec<String>), Box<dyn std::error::Error>> {
    let now = Arc::new(Mutex::new(parse_time("2026-10-22T17:40:00.000Z")?));
    let clock = now.clone();
    let store = Store::in_memory(Box::new(move || *clock.lock()), || {
        Ok(SigningKey::from_seed([45; 32]))
    })?;
    store.set_local_setting("author", "Sam")?;
    let places = Places::new(&store);
    let household = places.add_area(x.household)?;
    places.set_icon(&household.id, Some("home"))?;
    places.set_colour(&household.id, Some("2E7D6B"))?;
    let repairs = places.add_topic(&household.id, x.repairs)?;
    let work = places.add_area(x.work)?;
    places.set_icon(&work.id, Some("work"))?;
    places.set_colour(&work.id, Some("5C6BC0"))?;
    let car = places.add_area(x.car)?;
    places.set_icon(&car.id, Some("car"))?;
    places.set_colour(&car.id, Some("C0504D"))?;

    let notes = Notes::new(&store);
    let mut ids = Vec::new();
    let later = |minutes: i64| {
        let mut held = now.lock();
        *held += Duration::minutes(minutes);
    };
    let places_of = [
        Some(repairs.id.as_str()),
        Some(household.id.as_str()),
        Some(work.id.as_str()),
        Some(work.id.as_str()),
        Some(car.id.as_str()),
        None,
        None,
    ];
    let filed = x.filed.into_iter().zip(places_of);
    for (text, place) in filed {
        later(95);
        ids.push(notes.capture(text, place, None)?.id);
    }
    notes.set_importance(&ids[0], Importance::Important)?;
    later(30);
    notes.amend(&ids[0], x.washers)?;

    let routines = Routines::new(&store);
    let monday = Day::new(2026, 10, 26);
    let at = |h, m| ClockTime::new(h, m).ok_or("a time");
    let daily = oldbrainfix_core::Recurrence::parse("FREQ=DAILY")?;
    let weekly = oldbrainfix_core::Recurrence::parse("FREQ=WEEKLY;BYDAY=MO")?;
    let snack = notes.capture(x.snack, Some(&household.id), None)?;
    let schedule = Schedule::new(monday, &[at(8, 0)?, at(15, 0)?], Some(daily));
    routines.spawn(&snack.id, &schedule.ok_or("schedule")?)?;
    let bins = notes.capture(x.bins, Some(&household.id), None)?;
    let schedule = Schedule::new(monday, &[at(19, 0)?], Some(weekly));
    routines.spawn(&bins.id, &schedule.ok_or("schedule")?)?;
    let schedule = Schedule::new(monday, &[at(11, 30)?], None);
    routines.spawn(&ids[2], &schedule.ok_or("schedule")?)?;
    let schedule = Schedule::new(monday.add_days(1), &[at(9, 0)?], None);
    routines.create(x.garage, &schedule.ok_or("schedule")?)?;

    *now.lock() = parse_time("2026-10-26T08:30:00.000Z")?;
    let snacks = routines.of_note(&snack.id)?;
    if let Some(routine) = snacks.first() {
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
    let (store, ids) = demo(&EN)?;
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
        .find(|a| a.name == EN.household)
        .ok_or("the Household Area")?;
    harness.state_mut().select(Selection::Place(household.id));
    harness.state_mut().select_note(&ids[0]);
    shoot(&mut harness, FILES[1])?;

    harness.state_mut().select(Selection::Settings);
    shoot(&mut harness, FILES[2])?;
    Ok(())
}

/// How far a guide frame may drift from the committed one before it
/// counts as changed: a pixel differs when a channel is more than
/// `CHANNEL_SLACK` off, and a frame fails with more than `PIXEL_SLACK`
/// differing pixels. The slack absorbs the edges another GPU or Mesa
/// version rasterises a shade differently; a changed word or button
/// exceeds it. The same rule as the phone's frames.
const CHANNEL_SLACK: u8 = 24;
const PIXEL_SLACK: usize = 200;

/// Where the repository keeps the guide frame `path`.
fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path)
}

/// Compares the window as it stands with the committed guide frame `path`,
/// or writes it there with `UPDATE_FRAMES=1`. A changed frame is written to
/// `build/guide-frames/` for a look and returned as the problem.
fn match_frame(
    harness: &mut Harness<'static, App>,
    path: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    harness.run_steps(4);
    let image = harness.render()?;
    let shot = image::imageops::crop_imm(&image, MARGIN, MARGIN, SIZE.0, SIZE.1).to_image();
    let file = repo(path);
    if std::env::var("UPDATE_FRAMES").as_deref() == Ok("1") {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        shot.save(&file)?;
        return Ok(None);
    }
    let problem = match image::open(&file) {
        Err(err) => Some(format!("no committed frame ({err})")),
        Ok(old) => {
            let old = old.to_rgba8();
            if old.dimensions() == shot.dimensions() {
                let differing = old
                    .pixels()
                    .zip(shot.pixels())
                    .filter(|(a, b)| {
                        a.0.iter()
                            .zip(b.0)
                            .any(|(x, y)| x.abs_diff(y) > CHANNEL_SLACK)
                    })
                    .count();
                (differing > PIXEL_SLACK).then(|| format!("{differing} pixels differ"))
            } else {
                Some(format!(
                    "size {:?}, committed {:?}",
                    shot.dimensions(),
                    old.dimensions()
                ))
            }
        }
    };
    let Some(problem) = problem else {
        return Ok(None);
    };
    let fresh = repo("build/guide-frames").join(path);
    if let Some(dir) = fresh.parent() {
        std::fs::create_dir_all(dir)?;
    }
    shot.save(&fresh)?;
    Ok(Some(format!(
        "{path} no longer matches its screen ({problem}); the new frame is build/guide-frames/{path}"
    )))
}

/// The window over the demo store in `x`'s words, in `locale` and `theme`,
/// on the Agenda.
fn window(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    window_with(x, locale, theme, |_, _| Ok(()))
}

/// The window as [`window`] opens it, once `prepare` changed the demo
/// store, given with the ids of its filed Notes.
fn window_with(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
    prepare: impl FnOnce(&Store, &[String]) -> oldbrainfix_core::Result<()>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let (store, ids) = demo(x)?;
    prepare(&store, &ids)?;
    let mut app = App::new(store, locale);
    app.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (SIZE.0 + 2 * MARGIN) as f32,
            (SIZE.1 + 2 * MARGIN) as f32,
        ))
        .with_theme(theme)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run_steps(4);
    Ok(harness)
}

#[test]
#[ignore = "needs a GPU or a software Vulkan; run by tool/guide-frames.sh and CI"]
fn guide_frames() -> TestResult {
    let mut problems = Vec::new();
    for (language, texts) in [("en", &EN), ("de", &DE)] {
        for (theme, name) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let frame =
                |screen: &str| format!("site/assets/frames/desktop/{language}/{name}/{screen}.png");
            let mut harness = window(texts, language, theme)?;
            problems.extend(match_frame(&mut harness, &frame("agenda"))?);
            // A thought typed into the Agenda's capture field: the action
            // row and the Nudge times show.
            let mut harness = window(texts, language, theme)?;
            harness
                .get_by(|n| n.placeholder() == Some(texts.hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(&mut harness, &frame("capture"))?);
            // An AI Task that fixed a typo and filed two Unsorted Notes
            // into the Household waits for its review: since before the
            // window opened, so the review shows at once.
            let mut harness = window_with(texts, language, theme, |store, ids| {
                let notes = Notes::new(store);
                let parcel = notes.capture(texts.typo, None, None)?;
                let household = Places::new(store)
                    .areas()?
                    .into_iter()
                    .find(|a| a.name == texts.household)
                    .map(|a| a.id)
                    .unwrap_or_default();
                let task = store.open_ai_task()?;
                store.drafting(&task, || {
                    notes.edit(&parcel.id, texts.fixed)?;
                    notes.file(&parcel.id, &household)?;
                    notes.file(&ids[5], &household)
                })
            })?;
            problems.extend(match_frame(&mut harness, &frame("ai-review"))?);
            // The window's three panes: the Household's Notes, the
            // kitchen tap's, filed into Repairs, open in the editor.
            let mut tap = String::new();
            let mut harness = window_with(texts, language, theme, |_, ids| {
                tap = ids[0].clone();
                Ok(())
            })?;
            let household = Places::new(harness.state().store())
                .areas()?
                .into_iter()
                .find(|a| a.name == texts.household)
                .ok_or("the Household Area")?;
            harness.state_mut().select(Selection::Place(household.id));
            harness.state_mut().select_note(&tap);
            problems.extend(match_frame(&mut harness, &frame("window"))?);
            // The Settings with this computer's switches, the AI port
            // and the hotkey.
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().select(Selection::Settings);
            problems.extend(match_frame(&mut harness, &frame("settings"))?);
        }
    }
    assert!(
        problems.is_empty(),
        "{}\nRegenerate with tool/guide-frames.sh.",
        problems.join("\n")
    );
    Ok(())
}
