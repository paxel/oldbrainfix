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
//! `site/assets/frames/desktop/<language>/<theme>/`, a guide box's frame
//! in the folder of its page as `<page>/<box>.png` (#171). They are committed; a
//! run compares each frame with the committed one and fails when they
//! differ, and `UPDATE_FRAMES=1` writes them instead
//! (`tool/guide-frames.sh`):
//!
//! ```sh
//! cargo test -p oldbrainfix-gui --test screenshots -- --ignored guide_frames
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Duration, TimeZone, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{
    Checklists, ClockTime, Day, DeviceSettings, Frogs, Importance, Notes, PersonSettings, Places,
    QuickNoteControl, Routines, Schedule, ShoppingLists, SigningKey, Store, Tallies, parse_time,
};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Page, Selection};
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
    /// The quick note's hint, and a thought typed into it.
    hint: &'static str,
    capture: &'static str,
    /// The "+" at the top of a list (#161).
    add_here: &'static str,
    /// A Note with a typo, and its text as an AI Task fixes it.
    typo: &'static str,
    fixed: &'static str,
    /// The menu bar's Note menu.
    note_menu: &'static str,
    /// The first start's "New to Old Brain Fix".
    new_here: &'static str,
    /// The menu bar's File menu, its Import…, and Google Keep… in the
    /// Import dialog.
    file_menu: &'static str,
    import: &'static str,
    keep: &'static str,
    /// The Note menu's History.
    history: &'static str,
    /// The Agenda's "New Routine or Reminder", the hint of its text, the
    /// schedule's Weekdays and the Escalation's switch.
    new_routine: &'static str,
    what_hint: &'static str,
    weekdays: &'static str,
    escalate: &'static str,
    /// The start of an Agenda row's ⋮, "More for {title}".
    more_for: &'static str,
    /// The Household's lists (#174): a packing Checklist, its first line
    /// its name, a group under one of its Lines; the weekly Shopping
    /// List; a Tally of the pages read in the evenings, with its field
    /// and unit; and the Frogs put off for a while.
    packing: &'static str,
    groceries: &'static str,
    reading: &'static str,
    pages_read: &'static str,
    pages: &'static str,
    frogs: [&'static str; 3],
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
    add_here: "Add here",
    typo: "Pick up the parcel for Dana at the post office tomorow",
    fixed: "Pick up the parcel for Dana at the post office tomorrow",
    note_menu: "Note",
    new_here: "New to Old Brain Fix",
    file_menu: "File",
    import: "Import…",
    keep: "Google Keep…",
    history: "History",
    new_routine: "New Routine or Reminder",
    what_hint: "What is to be done?",
    weekdays: "Weekdays",
    escalate: "Escalate to an urgent alarm with sound",
    more_for: "More for",
    packing: "Packing for the weekend at the lake\n✓ Toothbrush\nPhone charger\nHiking gear\n  ✓ Boots\n  Rain jacket",
    groceries: "Weekly shopping\nMilk\nBread\nCoffee\nApples\nWashing-up liquid",
    reading: "Reading before bed",
    pages_read: "Pages read",
    pages: "pages",
    frogs: [
        "Sort the receipts for the tax return",
        "Clear out the shelf in the cellar",
        "Cancel the old gym membership",
    ],
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
    add_here: "Hier hinzufügen",
    typo: "Packet für Dana morgen bei der Post abhohlen",
    fixed: "Paket für Dana morgen bei der Post abholen",
    note_menu: "Notiz",
    new_here: "Neu bei Old Brain Fix",
    file_menu: "Datei",
    import: "Importieren …",
    keep: "Google Keep …",
    history: "Verlauf",
    new_routine: "Neue Routine oder Erinnerung",
    what_hint: "Was ist zu tun?",
    weekdays: "Wochentage",
    escalate: "Zum dringenden Alarm mit Ton eskalieren",
    more_for: "Mehr zu",
    packing: "Packen fürs Wochenende am See\n✓ Zahnbürste\nLadekabel fürs Handy\nWandersachen\n  ✓ Wanderschuhe\n  Regenjacke",
    groceries: "Wocheneinkauf\nMilch\nBrot\nKaffee\nÄpfel\nSpülmittel",
    reading: "Lesen vor dem Schlafen",
    pages_read: "Gelesene Seiten",
    pages: "Seiten",
    frogs: [
        "Belege für die Steuererklärung sortieren",
        "Das Regal im Keller ausmisten",
        "Den alten Fitnessstudio-Vertrag kündigen",
    ],
};

/// A store with a few days of a household's and a job's Notes and
/// Routines in `x`'s words, its clock at Monday 26 October 2026, 08:30 UTC.
fn demo(x: &Texts) -> Result<(Store, Vec<String>), Box<dyn std::error::Error>> {
    let (store, ids, _) = demo_with_clock(x)?;
    Ok((store, ids))
}

/// The store's clock, which a scene may set back and forth.
type Clock = Arc<Mutex<DateTime<Utc>>>;

/// [`demo`] with the store's clock.
fn demo_with_clock(x: &Texts) -> Result<(Store, Vec<String>, Clock), Box<dyn std::error::Error>> {
    let now = Arc::new(Mutex::new(parse_time("2026-10-22T17:40:00.000Z")?));
    let clock = now.clone();
    let mut store = Store::in_memory(Box::new(move || *clock.lock()), || {
        Ok(SigningKey::from_seed([45; 32]))
    })?;
    // Counted ids, not random ones: Notes captured at the same moment
    // keep one order on every machine.
    let next = AtomicU64::new(0);
    store.set_id_maker(Box::new(move || {
        Ok(format!("{:032x}", next.fetch_add(1, Ordering::SeqCst) + 1))
    }));
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
    // The screens show Tallies, Sessions, Frogs, Topics, the format bar
    // and roles: Advanced, chosen (#197).
    PersonSettings::new(&store).set_advanced(true)?;
    Ok((store, ids, now))
}

fn utc(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// The Notes of the Household's lists (#174), as [`seed_lists`] made them.
#[derive(Default)]
struct Lists {
    packing: String,
    groceries: String,
}

/// Adds the Household's lists in `x`'s words to the demo store, each at
/// its own moment on `now`, which is set back after: a Tally of the pages
/// read most evenings since 13 October, 30 a day the target; the packing
/// Checklist, a Line and a grouped one checked; the weekly Shopping List,
/// two Lines bought on Saturday; and three Frogs, one of them at Work.
fn seed_lists(store: &Store, x: &Texts, now: &Clock) -> oldbrainfix_core::Result<Lists> {
    let back = *now.lock();
    let at = |day: u32, h: u32, m: u32| {
        *now.lock() = Utc
            .with_ymd_and_hms(2026, 10, day, h, m, 0)
            .single()
            .unwrap_or(back);
    };
    let places = Places::new(store);
    let area = |name: &str| -> oldbrainfix_core::Result<Option<String>> {
        Ok(places
            .areas()?
            .into_iter()
            .find(|a| a.name == name)
            .map(|a| a.id))
    };
    let household = area(x.household)?;
    let work = area(x.work)?;
    let notes = Notes::new(store);

    at(13, 22, 5);
    let reading = notes.capture(x.reading, household.as_deref(), None)?;
    let tallies = Tallies::new(store);
    let pages = tallies.add_field(&reading.id, x.pages_read, x.pages, Some(30.0), None)?;
    let read = [24, 31, 0, 18, 40, 35, 12, 0, 30, 27, 22, 45, 33];
    for (day, count) in (13..).zip(read) {
        if count == 0 {
            continue;
        }
        at(day, 22, 15);
        let values = BTreeMap::from([(pages.clone(), f64::from(count))]);
        tallies.add(&reading.id, &values, "", None)?;
    }

    at(24, 9, 30);
    let groceries = notes.capture(x.groceries, household.as_deref(), None)?;
    let shopping = ShoppingLists::new(store);
    shopping.make(&groceries.id)?;
    at(24, 11, 5);
    if let Some(list) = shopping.by_note(&groceries.id, &utc)? {
        for line in [&list.lines[1], &list.lines[3]] {
            shopping.check(&line.id, &utc)?;
        }
    }

    at(25, 19, 40);
    let packing = notes.capture(x.packing, household.as_deref(), None)?;
    Checklists::new(store).make(&packing.id)?;

    let frogs = Frogs::new(store);
    for (day, (text, place)) in (8..).step_by(3).zip([
        (x.frogs[0], &work),
        (x.frogs[1], &household),
        (x.frogs[2], &household),
    ]) {
        at(day, 20, 10);
        let note = notes.capture(text, place.as_deref(), None)?;
        frogs.spawn(&note.id)?;
    }
    *now.lock() = back;
    Ok(Lists {
        packing: packing.id,
        groceries: groceries.id,
    })
}

/// The window over the demo store with the Household's lists, in `x`'s
/// words, `locale` and `theme`, on the Agenda; with the lists' Notes.
fn window_with_lists(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<(Harness<'static, App>, Lists), Box<dyn std::error::Error>> {
    let mut lists = Lists::default();
    let harness = window_with_clock(x, locale, theme, |store, _, now| {
        lists = seed_lists(store, x, now)?;
        Ok(())
    })?;
    Ok((harness, lists))
}

/// Shows the Household's Notes in the window.
fn show_household(harness: &mut Harness<'static, App>, x: &Texts) -> TestResult {
    let household = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .find(|a| a.name == x.household)
        .ok_or("the Household Area")?;
    harness.state_mut().select(Selection::Place(household.id));
    harness.run();
    Ok(())
}

/// Clicks the last widget labelled `label`.
fn click_last(harness: &mut Harness<'static, App>, label: &str) -> TestResult {
    harness
        .get_all_by_label(label)
        .last()
        .ok_or_else(|| format!("{label:?}"))?
        .click();
    harness.run();
    Ok(())
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

    harness.state_mut().open(Page::Settings);
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
fn match_frame<S>(
    harness: &mut Harness<'static, S>,
    path: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    harness.run_steps(4);
    let image = harness.render()?;
    // The window, or the capture popup, without the harness's margin.
    let (width, height) = image.dimensions();
    let shot = image::imageops::crop_imm(
        &image,
        MARGIN,
        MARGIN,
        width - 2 * MARGIN,
        height - 2 * MARGIN,
    )
    .to_image();
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
    window_with_clock(x, locale, theme, |store, ids, _| prepare(store, ids))
}

/// [`window_with`], `prepare` also given the store's clock: what it sets
/// it to holds while the window shows.
fn window_with_clock(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
    prepare: impl FnOnce(&Store, &[String], &Clock) -> oldbrainfix_core::Result<()>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let (store, ids, now) = demo_with_clock(x)?;
    prepare(&store, &ids, &now)?;
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
    // The window follows the system's theme (#147).
    harness.input_mut().system_theme = Some(theme);
    harness.run_steps(4);
    Ok(harness)
}

/// The id of the Routine whose Note says `text`.
fn routine_of(store: &Store, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Routines::new(store)
        .all_with_notes()?
        .into_iter()
        .find(|(_, note)| note.as_ref().is_some_and(|n| n.text == text))
        .map(|(routine, _)| routine.id)
        .ok_or_else(|| format!("the Routine of {text}").into())
}

/// The capture popup's size, as its own window opens it.
const POPUP: (u32, u32) = (480, 340);

/// The capture popup over the demo store, as the hotkey opens it.
struct Popup {
    popup: CapturePopup,
    outcome: Outcome,
}

/// The capture popup the hotkey opens over all windows, over the demo
/// store in `x`'s words, in `locale` and `theme`.
fn popup(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<Harness<'static, Popup>, Box<dyn std::error::Error>> {
    let (store, _) = demo(x)?;
    let mut popup = CapturePopup::new(Arc::new(store), locale);
    popup.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (POPUP.0 + 2 * MARGIN) as f32,
            (POPUP.1 + 2 * MARGIN) as f32,
        ))
        .with_theme(theme)
        .wgpu()
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
    harness.input_mut().system_theme = Some(theme);
    harness.run_steps(4);
    Ok(harness)
}

/// The window of a computer on its first start, in `locale` and `theme`:
/// nothing written yet, so the first start's three choices show.
fn first_start(
    locale: &str,
    theme: egui::Theme,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T08:30:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || {
        Ok(SigningKey::from_seed([46; 32]))
    })?;
    let mut app = App::new(store, locale);
    app.set_wall_clock(Arc::new(utc));
    app.set_profile_defaults(Box::new(|store| {
        oldbrainfix_core::OwnProfile::new(store).apply_defaults("Sam", None)
    }));
    app.offer_restore();
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
    harness.input_mut().system_theme = Some(theme);
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
            // The frame of the box `name` on the guide page `page` (#171),
            // as the site's box names it: a page's ticket only adds its
            // scenes here.
            let box_frame = |page: &str, name: &str| frame(&format!("{page}/{name}"));
            let mut harness = window(texts, language, theme)?;
            problems.extend(match_frame(&mut harness, &frame("agenda"))?);
            // Getting started: the first start's three choices, for adding
            // a Device and for a restore, whose own pages show a code and
            // a file of this run; the Starter Set; the Import dialog and
            // its question about a Keep export; the Settings.
            let mut harness = first_start(language, theme)?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "add-device"),
            )?);
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "restore"),
            )?);
            harness.get_by_label(texts.new_here).click_accesskit();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "start-new"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness.get_by_label(texts.file_menu).click();
            harness.run();
            harness.get_by_label(texts.import).click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "import-folder"),
            )?);
            harness.get_by_label(texts.keep).click_accesskit();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "import-keep"),
            )?);
            // A thought typed into the quick note, opened over the Agenda
            // as File ▸ New Note opens it (#146): the action row and the
            // Nudge times show.
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().new_note();
            harness.run();
            harness
                .get_by(|n| n.placeholder() == Some(texts.hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "capture"),
            )?);
            // Capture and sort: the quick note with what Settings ▸ Quick
            // note switched on, the capture popup of the hotkey, the
            // Unsorted Notes, the Note dialog and its History.
            let mut harness = window_with(texts, language, theme, |store, _| {
                let device = DeviceSettings::new(store);
                for control in [
                    QuickNoteControl::FileInto,
                    QuickNoteControl::Schedule,
                    QuickNoteControl::Importance,
                ] {
                    device.set_quick_note_shows(control, true)?;
                }
                Ok(())
            })?;
            harness.state_mut().new_note();
            harness.run();
            harness
                .get_by(|n| n.placeholder() == Some(texts.hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "sort-while-capturing"),
            )?);
            let mut harness = popup(texts, language, theme)?;
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "capture-outside"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().select(Selection::Unsorted);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "sort-later"),
            )?);
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
            // The Household's Notes under the kitchen tap's, filed into
            // Repairs, open in the Note dialog with its badges (#146,
            // #160).
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
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "edit"),
            )?);
            // The menu bar's Note menu over it, for that Note.
            harness
                .get_all_by_label(texts.note_menu)
                .next()
                .ok_or("the Note menu")?
                .click();
            harness.run();
            problems.extend(match_frame(&mut harness, &frame("menu"))?);
            // Note ▸ History of that Note.
            harness
                .get_all_by_label(texts.history)
                .last()
                .ok_or("Note ▸ History")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "history"),
            )?);
            // Routines and the Agenda (#173): a new Routine typed from the
            // Agenda's bar, the snack's schedule on weekdays, its Re-nag
            // and Escalation, a Reminder due now in its Note's dialog, the
            // Missed Summary, the snack's History, the Agenda, All Routines
            // and Reminders, and an Occurrence's menu with Share.
            let mut harness = window(texts, language, theme)?;
            harness.get_by_label(texts.new_routine).click();
            harness.run();
            harness
                .get_by(|n| n.placeholder() == Some(texts.what_hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "set-up"),
            )?);
            let mut harness = window(texts, language, theme)?;
            let snack = routine_of(harness.state().store(), texts.snack)?;
            harness.state_mut().open_routine(&snack);
            harness.run();
            harness.get_by_label(texts.weekdays).click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "schedule"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().open_routine(&snack);
            harness.run();
            harness
                .get_all_by_label(texts.escalate)
                .next()
                .ok_or("the Escalation's switch")?
                .scroll_to_me();
            harness.run();
            harness
                .get_all_by_label(texts.escalate)
                .next()
                .ok_or("the Escalation's switch")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "re-nag"),
            )?);
            // The parking's Note with a Reminder due at 08:00, set up the
            // evening before.
            let mut parking = String::new();
            let mut harness = window_with_clock(texts, language, theme, |store, ids, now| {
                parking = ids[3].clone();
                let due = ClockTime::new(8, 0)
                    .and_then(|at| Schedule::new(Day::new(2026, 10, 26), &[at], None));
                let back = *now.lock();
                *now.lock() = back - Duration::hours(12);
                if let Some(schedule) = due {
                    Routines::new(store).spawn(&ids[3], &schedule)?;
                }
                *now.lock() = back;
                Ok(())
            })?;
            harness.state_mut().select_note(&parking);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "check-off"),
            )?);
            // The tyres to be checked every evening since Tuesday, set up
            // the day before and never Done: Missed.
            let mut harness = window_with_clock(texts, language, theme, |store, ids, now| {
                let daily = oldbrainfix_core::Recurrence::parse("FREQ=DAILY").ok();
                let evenings = ClockTime::new(18, 0)
                    .and_then(|at| Schedule::new(Day::new(2026, 10, 20), &[at], daily));
                let back = *now.lock();
                *now.lock() = back - Duration::days(7);
                if let Some(schedule) = evenings {
                    Routines::new(store).spawn(&ids[4], &schedule)?;
                }
                *now.lock() = back;
                Ok(())
            })?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "missed"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness
                .state_mut()
                .select(Selection::History(snack.clone()));
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "history"),
            )?);
            let mut harness = window(texts, language, theme)?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "agenda"),
            )?);
            harness.state_mut().select(Selection::AllRoutines);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "all-routines"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness
                .get_all_by_label(&format!("{} {}", texts.more_for, texts.snack))
                .nth(1)
                .ok_or("the snack's ⋮")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "calendar"),
            )?);
            // What "+" at the top of the Household's list makes (#161).
            let mut harness = window(texts, language, theme)?;
            let household = Places::new(harness.state().store())
                .areas()?
                .into_iter()
                .find(|a| a.name == texts.household)
                .ok_or("the Household Area")?;
            harness.state_mut().select(Selection::Place(household.id));
            harness.run();
            harness.get_by_label(texts.add_here).click();
            harness.run();
            problems.extend(match_frame(&mut harness, &frame("add-menu"))?);
            // Lists, Tallies, Sessions and Frogs (#174): the same "+", the
            // Household's cards compact, a plain Note's kinds, the packing
            // Checklist in the Note dialog, the Shopping List's Agenda
            // choices and its card, the New Tally page, a Tally's card,
            // the New Session page and the Frog view.
            const LISTS: &str = "lists-tallies-sessions-frogs";
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "make"))?);
            let (mut harness, _) = window_with_lists(texts, language, theme)?;
            show_household(&mut harness, texts)?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame(LISTS, "shopping-list"),
            )?);
            let add_entry = harness.state().t().add_tally_entry().to_string();
            harness
                .get_all_by_label(&add_entry)
                .next()
                .ok_or("the Tally's Add entry")?
                .scroll_to_me();
            harness.run();
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "tally-entry"))?);
            let (mut harness, _) = window_with_lists(texts, language, theme)?;
            show_household(&mut harness, texts)?;
            let view = harness.state().t().menu_view().to_string();
            let compact = harness.state().t().menu_compact_views().to_string();
            click_last(&mut harness, &view)?;
            click_last(&mut harness, &compact)?;
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "views"))?);
            let (mut harness, lists) = window_with_lists(texts, language, theme)?;
            harness.state_mut().select_note(&lists.packing);
            harness.run();
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "checklist"))?);
            let (mut harness, lists) = window_with_lists(texts, language, theme)?;
            harness.state_mut().select_note(&lists.groceries);
            harness.run();
            let agenda = harness.state().t().checklist_show_on_agenda().to_string();
            harness
                .query_all_by_label_contains(&format!("{agenda}: "))
                .next()
                .ok_or("the Show on the Agenda badge")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame(LISTS, "list-on-agenda"),
            )?);
            // The bike lock's plain Note in the Note dialog, its kind
            // badge open.
            let mut lock = String::new();
            let mut harness = window_with(texts, language, theme, |_, ids| {
                lock = ids[1].clone();
                Ok(())
            })?;
            harness.state_mut().select_note(&lock);
            harness.run();
            let kind = {
                let t = harness.state().t();
                format!("{}: {}", t.badge_kind(), t.kind_plain())
            };
            click_last(&mut harness, &kind)?;
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "kind"))?);
            for (page, label) in [
                ("tally", harness.state().t().tally_heading().to_string()),
                ("session", harness.state().t().session().to_string()),
            ] {
                let (mut harness, _) = window_with_lists(texts, language, theme)?;
                show_household(&mut harness, texts)?;
                harness.get_by_label(texts.add_here).click();
                harness.run();
                click_last(&mut harness, &label)?;
                problems.extend(match_frame(&mut harness, &box_frame(LISTS, page))?);
            }
            let (mut harness, _) = window_with_lists(texts, language, theme)?;
            harness.state_mut().select(Selection::Frogs);
            harness.run();
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "frog"))?);
            // The Settings dialog with this computer's switches, the AI
            // port and the hotkey, over the Agenda.
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().open(Page::Settings);
            problems.extend(match_frame(&mut harness, &frame("settings"))?);
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "set-up"),
            )?);
        }
    }
    assert!(
        problems.is_empty(),
        "{}\nRegenerate with tool/guide-frames.sh.",
        problems.join("\n")
    );
    Ok(())
}
