//! The Note dialog's Add to a Shopping List (#352), which took over the
//! quick note's (#124), driven headless by egui_kittest: with one list
//! its entry names it, "Add to Groceries", with several they show as
//! radio buttons, the one used last at the top (#276); a pick makes each
//! line of the Note a Line, a name the list has bringing its Line back,
//! keeps no Note and closes the Note dialog; a Note recorded into in the
//! Note dialog stays on the list as a Section with its recording, and the
//! items said in its Transcript become Lines once it is transcribed
//! (#311); it is not offered without a list, nor for a Note with a photo.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, Note, Notes, OwnProfile, ShoppingLists, SigningKey, Store};
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::media::{self, Recorder};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 07:00 on Saturday, 24 October 2026, the wall clock on UTC.
const NOW: &str = "2026-10-24T07:00:00Z";

/// What the tests say, in English.
fn t() -> L10n {
    L10n::new("en")
}

/// A store on a clock that moves a second per read, with the Shopping
/// List "Groceries" holding "Milk": the store and the list's Note.
fn store() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339(NOW)?.timestamp();
    let moment = Arc::new(AtomicI64::new(start));
    let clock = Box::new(move || {
        DateTime::<Utc>::from_timestamp(moment.fetch_add(1, Ordering::SeqCst), 0)
            .unwrap_or_default()
    });
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let list = Notes::new(&store)
        .capture("Groceries\nMilk", None, None)?
        .id;
    ShoppingLists::new(&store).make(&list)?;
    Ok((Arc::new(store), list))
}

/// The days of the tests, on UTC.
fn utc(utc: DateTime<Utc>) -> chrono::NaiveDateTime {
    utc.naive_utc()
}

/// The texts of the Lines of the list `list`, in alphabetical order.
fn lines(store: &Store, list: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut texts: Vec<String> = ShoppingLists::new(store)
        .by_note(list, &utc)?
        .ok_or("a Shopping List")?
        .lines
        .iter()
        .map(|l| l.text.clone())
        .collect();
    texts.sort();
    Ok(texts)
}

/// The Notes besides the Shopping List `list`.
fn others(store: &Store, list: &str) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .filter(|n| n.id != list)
        .collect())
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn click<S>(harness: &mut Harness<'_, S>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.run();
}

/// Opens the Note dialog's Add to a Shopping List.
fn open_menu<S>(harness: &mut Harness<'_, S>) {
    click(harness, t().add_to_shopping_list());
}

/// Waits until `done` holds, for at most ten seconds.
fn eventually(mut done: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + StdDuration::from_secs(10);
    while !done() {
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(StdDuration::from_millis(5));
    }
    true
}

/// The window over `app` on All Notes, its days on UTC, with the Note
/// `note` open in the Note dialog.
fn window(app: App, note: &str) -> Harness<'static, App> {
    let mut app = app;
    app.set_wall_clock(Arc::new(utc));
    app.select(Selection::All);
    app.select_note(note);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
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

#[test]
fn each_line_of_the_note_becomes_a_line_without_duplicates_and_no_note_stays() -> TestResult {
    let (store, list) = store()?;
    let note = Notes::new(&store)
        .capture("Eggs\nmilk\nButter", None, None)?
        .id;
    let mut harness = window(App::new(store.clone(), "en"), &note);
    open_menu(&mut harness);
    // One list to add to: its entry names it (#276).
    harness
        .get_by_label(&t().add_to_named_shopping_list("Groceries"))
        .click_accesskit();
    harness.run();
    // "milk" brings the Line there back instead of a second one.
    assert_eq!(lines(&store, &list)?, ["Butter", "Eggs", "Milk"]);
    assert_eq!(others(&store, &list)?, Vec::<Note>::new());
    assert_eq!(harness.state().selected_note(), None);
    assert_eq!(
        DeviceSettings::new(&store).last_shopping_list()?.as_deref(),
        Some(list.as_str())
    );
    Ok(())
}

#[test]
fn it_is_not_offered_without_a_list_nor_for_a_note_with_a_photo() -> TestResult {
    let (store, list) = store()?;
    let notes = Notes::new(&store);
    let bread = notes.capture("Bread", None, None)?.id;
    let harness = window(App::new(store.clone(), "en"), &bread);
    assert!(shows(&harness, t().add_to_shopping_list()));
    drop(harness);
    // No Line holds a photo.
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([20, 120, 200, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut png, image::ImageFormat::Png)?;
    let pictured = notes.capture("Paint", None, None)?.id;
    notes.add_photo(&pictured, &png.into_inner())?;
    let harness = window(App::new(store.clone(), "en"), &pictured);
    assert!(!shows(&harness, t().add_to_shopping_list()));
    drop(harness);
    // Nor the list itself, nor without a list.
    let harness = window(App::new(store.clone(), "en"), &list);
    assert!(!shows(&harness, t().add_to_shopping_list()));
    drop(harness);
    notes.trash(&list)?;
    let harness = window(App::new(store.clone(), "en"), &bread);
    assert!(!shows(&harness, t().add_to_shopping_list()));
    Ok(())
}

/// A microphone that hears a tenth of a second of silence.
#[derive(Default)]
struct Mic {
    recording: bool,
}

impl Recorder for Mic {
    fn start(&mut self) -> bool {
        self.recording = true;
        true
    }

    fn stop(&mut self) -> Option<Vec<u8>> {
        std::mem::take(&mut self.recording).then(|| media::wav(&[0.0; 1600]))
    }

    fn recording(&self) -> bool {
        self.recording
    }

    fn elapsed(&self) -> StdDuration {
        StdDuration::from_secs(3)
    }
}

/// Hears `heard` in every recording; `None` when it cannot read one.
struct Hears(Option<&'static str>);

impl Engine for Hears {
    fn read(&mut self, _: Vec<u8>) -> Option<String> {
        self.0.map(str::to_string)
    }
}

/// The thread making Transcripts of `store`, hearing `heard`.
fn transcriber(store: &Arc<Store>, heard: Option<&'static str>) -> Enricher {
    Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(Hears(heard)),
        Box::new(|| {}),
    )
}

/// The voice capture's Note, merged into a list or not.
fn voice_note(store: &Store) -> Result<Note, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    for id in store.entities_of_type("note")? {
        if let Some(note) = notes.by_id(&id)?
            && !note.recordings.is_empty()
        {
            return Ok(note);
        }
    }
    Err("no voice capture".into())
}

#[test]
fn a_note_recorded_into_stays_on_the_list_and_the_items_said_become_lines() -> TestResult {
    let (store, list) = store()?;
    let bread = Notes::new(&store).capture("Bread", None, None)?.id;
    let mut app = App::new(store.clone(), "en");
    app.set_recorder(Box::new(Mic::default()));
    app.set_enrichers(
        Some(transcriber(&store, Some("Eggs, milk und Butter."))),
        None,
        None,
    );
    let mut harness = window(app, &bread);
    // Voice in the Note dialog goes onto its Note (#352).
    click(&mut harness, t().voice());
    click(&mut harness, t().stop_recording());
    let recorded = Notes::new(&store).by_id(&bread)?.ok_or("Bread")?;
    assert_eq!(recorded.recordings.len(), 1);
    open_menu(&mut harness);
    harness
        .get_by_label(&t().add_to_named_shopping_list("Groceries"))
        .click_accesskit();
    harness.run();
    // The typed line at once, the items said once transcribed; Milk once.
    let added = eventually(|| lines(&store, &list).is_ok_and(|l| l.len() == 4));
    assert!(added, "{:?}", lines(&store, &list)?);
    assert_eq!(lines(&store, &list)?, ["Bread", "Butter", "Eggs", "Milk"]);
    let voice = voice_note(&store)?;
    assert_eq!(voice.id, bread);
    assert_eq!(voice.merged_into.as_deref(), Some(list.as_str()));
    let sections = Notes::new(&store).sections(&list)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].recordings, voice.recordings);
    assert_eq!(others(&store, &list)?, Vec::<Note>::new());
    Ok(())
}

#[test]
fn of_several_lists_the_one_used_last_is_at_the_top() -> TestResult {
    let (store, groceries) = store()?;
    let notes = Notes::new(&store);
    let hardware = notes.capture("Hardware\nNails", None, None)?.id;
    ShoppingLists::new(&store).make(&hardware)?;
    let top = |harness: &Harness<'_, App>, title: &str| {
        harness
            .get_by_role_and_label(Role::RadioButton, title)
            .rect()
            .top()
    };
    let glue = notes.capture("Glue", None, None)?.id;
    let mut harness = window(App::new(store.clone(), "en"), &glue);
    open_menu(&mut harness);
    // Oldest capture first while none was used.
    assert!(top(&harness, "Groceries") < top(&harness, "Hardware"));
    harness
        .get_by_role_and_label(Role::RadioButton, "Hardware")
        .click_accesskit();
    harness.run();
    assert_eq!(lines(&store, &hardware)?, ["Glue", "Nails"]);
    let tea = notes.capture("Tea", None, None)?.id;
    let mut harness = window(App::new(store.clone(), "en"), &tea);
    open_menu(&mut harness);
    assert!(top(&harness, "Hardware") < top(&harness, "Groceries"));
    assert_eq!(lines(&store, &groceries)?, ["Milk"]);
    Ok(())
}
