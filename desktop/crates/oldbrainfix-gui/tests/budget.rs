//! The time budget on a large store (obf-1069, C12): with 5000 Notes,
//! sized as a Keep import leaves them (a few hundred characters, some in
//! Markdown, some Checklists, some located in a Named Place, some in
//! Areas and Topics, a few with a photo), opening All Notes takes at most
//! the 500 ms of a page change, a click on a card and a switch of its
//! view at most the 100 ms of a tap, and every letter typed into the
//! search, with its results, at most 500 ms (ui-laws.md, obf-1032;
//! obf-1068): the letter shows within a tap's time, its search runs once
//! typing stopped and is timed from its start, the pause before it left
//! out. Each step is timed from the event to the frame that shows its
//! effect, a few times, and its median held against the budget.
//!
//! Timed in a release build only, as the app ships; CI runs it in a step
//! of its own:
//!
//! cargo test -p oldbrainfix-gui --test budget --release -- --ignored --nocapture

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::time::{Duration as Elapsed, Instant};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::named_places::keys as place_keys;
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::schedule::Recurrence;
use oldbrainfix_core::text_format::{MARKDOWN, PLAIN};
use oldbrainfix_core::{
    Checklists, ClockTime, Day, Notes, OwnProfile, Places, Routines, Schedule, SigningKey, Store,
};
use oldbrainfix_gui::alerts::Alerts;
use oldbrainfix_gui::app::local_wall;
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::notify::RecordingNotifier;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// How many Notes the store holds (C12).
const NOTES: usize = 5000;
/// How many daily Reminders the alarms plan in the timing of a click
/// beside them (obf-1091).
const ROUTINES: usize = 300;
/// How often each step is timed; its median counts.
const REPEATS: usize = 5;
/// A tap or a click, and a page change, from the event to the frame
/// showing it (ui-laws.md).
const TAP: Elapsed = Elapsed::from_millis(100);
const PAGE: Elapsed = Elapsed::from_millis(500);
/// A search with its results (obf-1068, C11).
const SEARCH: Elapsed = Elapsed::from_millis(500);

/// The words the texts are made of; "milk", "mild" and "mint" for the
/// search to narrow.
const WORDS: [&str; 24] = [
    "milk", "mild", "mint", "bread", "garden", "call", "mother", "about", "the", "trip", "plan",
    "remember", "book", "tomorrow", "and", "a", "window", "repair", "meeting", "list", "idea",
    "letter", "monday", "price",
];

/// `count` words picked from [`WORDS`] after `seed`, the same for the
/// same seed.
fn words(seed: usize, count: usize) -> String {
    let mut x = seed as u64 * 6364136223846793005 + 1442695040888963407;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.push(WORDS[(x >> 33) as usize % WORDS.len()]);
    }
    out.join(" ")
}

/// Whether Note `n` is plain text, neither a Checklist nor Markdown: the
/// cards clicked, where a click lands on no Line, link or tag.
fn plain(n: usize) -> bool {
    n % 10 >= 3
}

/// A small photo of its own colour for Note `n`.
fn photo(n: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let colour = [(n % 251) as u8, (n / 251 % 251) as u8, 128];
    let image = image::RgbImage::from_pixel(64, 48, image::Rgb(colour));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

/// The store of [`NOTES`] Notes, "Note 0" the oldest, written in one
/// batch, on a clock that stands still after them.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    store_at(None, 0)
}

/// [`store`]'s Notes, in the database at `path` when given (obf-1091),
/// else in memory, after `routines` daily Reminders through the day.
fn store_at(path: Option<&Path>, routines: usize) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let at = Arc::new(AtomicI64::new(0));
    let read = at.clone();
    let clock = Box::new(move || start + Duration::seconds(read.load(Ordering::SeqCst)));
    let key = || Ok(SigningKey::from_seed([3; 32]));
    let store = match path {
        Some(path) => Store::open(path, clock, key)?,
        None => Store::in_memory(clock, key)?,
    };
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let photos = (0..NOTES)
        .filter(|n| n % 25 == 0)
        .map(|n| photo(n).map(|p| (n, p)))
        .collect::<Result<std::collections::HashMap<_, _>, _>>()?;
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let from = Day::today(start, &local_wall).add_days(-3);
    store.batch(|| {
        let made = Routines::new(&store);
        for r in 0..routines {
            let time = ClockTime::new((r % 24) as u32, (r * 7 % 60) as u32);
            let schedule = time
                .and_then(|t| Schedule::new(from, &[t], Some(rule.clone())))
                .ok_or_else(|| oldbrainfix_core::Error::Invalid("a schedule".into()))?;
            made.create(&format!("Reminder {r}"), &schedule)?;
        }
        let places = Places::new(&store);
        let mut homes = Vec::new();
        for a in 0..6 {
            let area = places.add_area(&format!("Area {a}"))?;
            homes.push(area.id.clone());
            for t in 0..3 {
                homes.push(places.add_topic(&area.id, &format!("Topic {a}.{t}"))?.id);
            }
        }
        let home = "namedPlace:home";
        store.append(
            home,
            place_keys::TYPE,
            Some(place_keys::TYPE_NAMED_PLACE),
            None,
        )?;
        store.append(home, place_keys::NAME, Some("Home"), None)?;
        store.append(home, place_keys::CENTRE, Some("52.52,13.405"), None)?;
        store.append(home, place_keys::RADIUS, Some("300"), None)?;
        let notes = Notes::new(&store);
        let lists = Checklists::new(&store);
        for n in 0..NOTES {
            at.fetch_add(60, Ordering::SeqCst);
            // Two of three filed, the others Unsorted.
            let place = match n % 3 {
                2 => None,
                _ => homes.get(n % homes.len()).map(String::as_str),
            };
            let (text, format) = match n % 10 {
                0 => {
                    let lines = (0..6)
                        .map(|l| words(n * 7 + l, 3))
                        .collect::<Vec<_>>()
                        .join("\n");
                    (format!("Note {n}\n{lines}"), PLAIN)
                }
                1 | 2 => (
                    format!(
                        "# Note {n}\n\n{} **{}** #list{}\n\n- {}\n- {}\n\n{}",
                        words(n, 25),
                        words(n + 1, 2),
                        n % 7,
                        words(n + 2, 5),
                        words(n + 3, 5),
                        words(n + 4, 20),
                    ),
                    MARKDOWN,
                ),
                _ => (format!("Note {n} {}", words(n, 20 + n % 60)), PLAIN),
            };
            let photo: Vec<Vec<u8>> = photos.get(&n).cloned().into_iter().collect();
            let id = if photo.is_empty() {
                notes.capture_as(&text, format, place, None)?.id
            } else {
                notes.capture_with(&text, place, None, &photo, &[])?.id
            };
            if n % 10 == 0 {
                lists.make(&id)?;
            }
            if n % 4 == 0 {
                let location = format!("52.52{},13.405", n % 9);
                store.append(&id, note_keys::LOCATION, Some(&location), None)?;
            }
        }
        Ok(())
    })?;
    Ok(store)
}

/// Steps frames until `done`, and once more to the frame that shows it;
/// how long from `start` that took.
fn until(
    harness: &mut Harness<'_, App>,
    start: Instant,
    done: impl Fn(&App) -> bool,
) -> Result<Elapsed, Box<dyn std::error::Error>> {
    for _ in 0..20 {
        harness.step();
        if done(harness.state()) {
            harness.step();
            return Ok(start.elapsed());
        }
    }
    Err("the step never took effect".into())
}

/// Steps frames until the search of `letters` starts on its thread, the
/// typing rested, and on to the frame that shows what it found; how long
/// that took from the frame that started it (obf-1068).
fn searched(
    harness: &mut Harness<'_, App>,
    letters: &str,
) -> Result<Elapsed, Box<dyn std::error::Error>> {
    let mut start = None;
    for _ in 0..20 {
        let before = Instant::now();
        harness.step();
        if harness.state().searching() {
            start = Some(before);
            break;
        }
    }
    let start = start.ok_or("the search never started")?;
    // The window looks every 30 ms while it runs, as the app does.
    while harness.state().searched() != letters {
        if start.elapsed() > Elapsed::from_secs(30) {
            return Err("the search never came".into());
        }
        std::thread::sleep(Elapsed::from_millis(30));
        harness.step();
    }
    harness.step();
    Ok(start.elapsed())
}

/// Lets more than a double-click's time pass, untimed.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// What one step took, its times sorted.
struct Step {
    name: &'static str,
    budget: Elapsed,
    times: Vec<Elapsed>,
}

impl Step {
    fn median(&self) -> Elapsed {
        self.times[self.times.len() / 2]
    }
}

/// The plain Notes whose cards are in view, newest first: their ids and
/// what their cards say.
fn plain_in_view(harness: &Harness<'_, App>) -> Vec<(String, String)> {
    let notes = Notes::new(harness.state().store());
    harness
        .state()
        .shown_notes()
        .into_iter()
        .take(40)
        .filter_map(|id| notes.by_id(&id).ok().flatten())
        .filter(|note| {
            note.text
                .strip_prefix("Note ")
                .and_then(|rest| rest.split(' ').next())
                .and_then(|n| n.parse::<usize>().ok())
                .is_some_and(plain)
        })
        .map(|note| (note.id.clone(), note.visible_text()))
        .filter(|(_, label)| {
            harness
                .query_all_by_role_and_label(Role::Button, label)
                .next()
                .is_some()
        })
        .collect()
}

#[test]
#[ignore = "a timing test: run in release by CI's own step"]
fn all_notes_a_card_and_the_search_on_5000_notes_keep_the_budget() -> TestResult {
    if cfg!(debug_assertions) {
        return Err("timed in a release build only: cargo test --release".into());
    }
    let built = Instant::now();
    let store = Arc::new(store()?);
    println!("{NOTES} Notes written in {:?}", built.elapsed());
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    // The search's thread on the window's own connection: an in-memory
    // store has no other.
    let aside = store.clone();
    app.set_agenda_store(move || aside.clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_max_steps(200)
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    let all = harness.state().shown_notes().len();
    assert_eq!(all, NOTES, "All Notes shows every Note");
    let mut steps = Vec::new();

    // All Notes from another list, what the store worked out forgotten
    // as after a write, and as it is kept.
    for (name, cold) in [
        ("All Notes opened, read anew", true),
        ("All Notes opened again", false),
    ] {
        let mut times = Vec::new();
        for _ in 0..REPEATS {
            harness.state_mut().select(Selection::Unsorted);
            harness.run();
            assert!(harness.state().shown_notes().len() < all);
            if cold {
                store.changed_elsewhere();
            }
            let start = Instant::now();
            harness.get_by_label("All Notes").click();
            times.push(until(&mut harness, start, |app| {
                *app.selection() == Selection::All && app.shown_notes().len() == all
            })?);
            harness.run();
        }
        steps.push(Step {
            name,
            budget: PAGE,
            times,
        });
    }

    // A click on a card selects it, another card each time.
    pause(&mut harness);
    let cards = plain_in_view(&harness);
    assert!(
        cards.len() >= REPEATS,
        "{} plain cards in view",
        cards.len()
    );
    let mut times = Vec::new();
    for (id, label) in cards.iter().take(REPEATS) {
        let card = harness.get_by_role_and_label(Role::Button, label);
        let start = Instant::now();
        card.click();
        times.push(until(&mut harness, start, |app| {
            app.selected_cards() == [id.clone()]
        })?);
        pause(&mut harness);
    }
    steps.push(Step {
        name: "A card clicked",
        budget: TAP,
        times,
    });
    harness.key_press(Key::Escape);
    harness.run();

    // A card switched to compact and back.
    let (_, label) = cards.first().ok_or("no card")?;
    let mut times = Vec::new();
    for _ in 0..REPEATS {
        let setting = |app: &App| app.store().local_setting("switchedNotes:allNotes");
        let before = setting(harness.state())?;
        let card = harness.get_by_role_and_label(Role::Button, label).rect();
        let switch = harness
            .query_all_by_role(Role::Button)
            .find(|n| {
                n.accesskit_node()
                    .label()
                    .is_some_and(|l| l == "Show compact view" || l == "Show detailed view")
                    && card.contains_rect(n.rect())
            })
            .ok_or("no view button")?;
        let start = Instant::now();
        switch.click();
        times.push(until(&mut harness, start, |app| {
            setting(app).ok() != Some(before.clone())
        })?);
        pause(&mut harness);
    }
    steps.push(Step {
        name: "A card's view switched",
        budget: TAP,
        times,
    });

    // A letter typed into the search, its results shown: the first, the
    // second and the third.
    // Each letter shows at once; its search starts once typing stopped
    // and is timed from there to the frame showing what it found
    // (obf-1068).
    let mut letters_shown = Vec::new();
    let mut typed: [Vec<Elapsed>; 3] = Default::default();
    for _ in 0..REPEATS {
        harness.state_mut().set_query("");
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
        harness.run();
        for (letters, times) in ["m", "mi", "mil"].into_iter().zip(typed.iter_mut()) {
            let letter = &letters[letters.len() - 1..];
            let field = harness.get_by(|n| n.is_focused());
            assert_eq!(field.accesskit_node().role(), Role::TextInput);
            let start = Instant::now();
            field.type_text(letter);
            letters_shown.push(until(&mut harness, start, |app| app.query() == letters)?);
            times.push(searched(&mut harness, letters)?);
            harness.run();
        }
    }
    let found = harness.state().shown_notes().len();
    assert!(0 < found && found < all, "{found} Notes found for \"mil\"");
    steps.push(Step {
        name: "A letter shown",
        budget: TAP,
        times: letters_shown,
    });
    for (name, times) in [
        "A first letter searched",
        "A second letter searched",
        "A third letter searched",
    ]
    .into_iter()
    .zip(typed)
    {
        steps.push(Step {
            name,
            budget: SEARCH,
            times,
        });
    }

    let mut over = Vec::new();
    for step in &mut steps {
        step.times.sort();
        let median = step.median();
        println!(
            "{:<28} median {:>9.1?}  min {:>9.1?}  max {:>9.1?}  budget {:?}",
            step.name,
            median,
            step.times[0],
            step.times[step.times.len() - 1],
            step.budget
        );
        if median > step.budget {
            over.push(format!(
                "{}: {median:.1?} over {:?}",
                step.name, step.budget
            ));
        }
    }
    assert!(over.is_empty(), "over the budget: {over:?}");
    Ok(())
}

/// The window with All Notes on `store`, laid out.
fn all_notes(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_max_steps(200)
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

/// A click on a card and All Notes read anew while the alarms tick
/// again and again over daily Reminders, their plan, Heads-ups, Nudge
/// and Changes Nudge, on the window's connection as before obf-1091 and
/// on one of their own as the app runs them now, and with no alarms
/// running to compare: on their own the click keeps a tap's budget.
#[test]
#[ignore = "a timing test: run in release by CI's own step"]
fn a_click_and_a_page_wait_on_no_alarms_tick_on_5000_notes() -> TestResult {
    if cfg!(debug_assertions) {
        return Err("timed in a release build only: cargo test --release".into());
    }
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    let store = Arc::new(store_at(Some(&path), ROUTINES)?);
    let now = store.now();
    let own = Arc::new(Store::open(&path, Box::new(move || now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?);
    // What the Nudge built before obf-1091, every Note, and what it reads
    // now, each worked out anew.
    let notes = Notes::new(&store);
    store.changed_elsewhere();
    let start = Instant::now();
    let unsorted = notes.unsorted()?.len();
    let built = start.elapsed();
    store.changed_elsewhere();
    let start = Instant::now();
    assert_eq!(notes.nudge_anchors()?.len(), unsorted);
    println!(
        "The Nudge over {unsorted} Unsorted Notes: {built:.1?} with every Note made, {:.1?} now",
        start.elapsed()
    );
    let mut harness = all_notes(&store);
    pause(&mut harness);
    let cards = plain_in_view(&harness);
    assert!(
        cards.len() >= REPEATS,
        "{} plain cards in view",
        cards.len()
    );
    let all = harness.state().shown_notes().len();
    let mut click = Elapsed::ZERO;
    for (name, alarms) in [
        ("not running", None),
        ("on the window's connection", Some(store.clone())),
        ("on a connection of their own", Some(own.clone())),
    ] {
        let stop = Arc::new(AtomicBool::new(false));
        let ticking = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut alerts = Alerts::new(
                    Box::new(RecordingNotifier::default()),
                    L10n::new("en"),
                    Arc::new(AtomicU64::new(0)),
                );
                let mut ticks = Vec::new();
                let Some(alarms) = alarms else {
                    return ticks;
                };
                while !stop.load(Ordering::SeqCst) {
                    let start = Instant::now();
                    alerts.tick(&alarms, &local_wall).unwrap();
                    ticks.push(start.elapsed());
                }
                ticks
            })
        };
        let mut times = Vec::new();
        for _ in 0..3 {
            for (id, label) in cards.iter().take(REPEATS) {
                let card = harness.get_by_role_and_label(Role::Button, label);
                let start = Instant::now();
                card.click();
                times.push(until(&mut harness, start, |app| {
                    app.selected_cards() == [id.clone()]
                })?);
                pause(&mut harness);
            }
        }
        // All Notes from another list, read anew as after a write.
        let mut pages = Vec::new();
        for _ in 0..REPEATS {
            harness.state_mut().select(Selection::Unsorted);
            harness.run();
            store.changed_elsewhere();
            let start = Instant::now();
            harness.get_by_label("All Notes").click();
            pages.push(until(&mut harness, start, |app| {
                *app.selection() == Selection::All && app.shown_notes().len() == all
            })?);
            harness.run();
        }
        stop.store(true, Ordering::SeqCst);
        let mut ticks = ticking.join().map_err(|_| "the alarms thread")?;
        ticks.sort();
        let tick = ticks.get(ticks.len() / 2).copied().unwrap_or_default();
        println!(
            "The alarms {name}: {} ticks, median {tick:.1?}",
            ticks.len()
        );
        let mut medians = Vec::new();
        for (step, mut times) in [("A card clicked", times), ("All Notes read anew", pages)] {
            times.sort();
            let median = times[times.len() / 2];
            println!(
                "  {step:<20} median {median:>9.1?}  max {:>9.1?}",
                times[times.len() - 1],
            );
            medians.push(median);
        }
        click = medians[0];
    }
    // All Notes is only shown: on disk and beside the Reminders it is
    // read slower than the budget's own store in memory, alarms or not.
    assert!(click <= TAP, "a card clicked: {click:.1?} over {TAP:?}");
    Ok(())
}
