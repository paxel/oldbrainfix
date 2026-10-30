//! Sorting a capture into a Shopping List (#124), in the capture popup
//! and the window's capture field, driven headless by egui_kittest: "Add
//! to a Shopping List" in "More" lists the lists, the one picked shows at
//! the top alone and the other choices come back when it is dropped;
//! saving makes each line a Line, a name the list has bringing its Line
//! back, and keeps no Note; a recording's Transcript gives its lines once
//! it is there, and without one the Note stays, announced at the top; a
//! photo keeps it a Note; and it is not offered without a list nor with
//! a recording that gets no Transcript here.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Utc};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Note, Notes, OwnProfile, Places, ShoppingLists, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::media::{self, Clipboard, Recorder};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 07:00 on Saturday, 24 October 2026, the wall clock on UTC.
const NOW: &str = "2026-10-24T07:00:00Z";

/// What the tests say, in English.
fn t() -> L10n {
    L10n::new("en")
}

/// A store on a clock that moves a second per read, with the Area
/// "Household" and the Shopping List "Groceries" holding "Milk": the
/// store, the Area's id and the list's Note.
fn store() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339(NOW)?.timestamp();
    let moment = Arc::new(AtomicI64::new(start));
    let clock = Box::new(move || {
        DateTime::<Utc>::from_timestamp(moment.fetch_add(1, Ordering::SeqCst), 0)
            .unwrap_or_default()
    });
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    let list = Notes::new(&store)
        .capture("Groceries\nMilk", None, None)?
        .id;
    ShoppingLists::new(&store).make(&list)?;
    Ok((Arc::new(store), household, list))
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

/// Picks the Shopping List `title` in "More".
fn pick<S>(harness: &mut Harness<'_, S>, title: &str) {
    click(harness, "More");
    assert!(shows(harness, t().add_to_shopping_list()));
    harness
        .get_by_role_and_label(Role::RadioButton, title)
        .click_accesskit();
    harness.run();
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

struct Popup {
    popup: CapturePopup,
    closed: bool,
}

fn popup(store: &Arc<Store>) -> Harness<'static, Popup> {
    let mut popup = CapturePopup::new(store.clone(), "en");
    popup.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 900.0))
        .build_ui_state(
            |ui, state: &mut Popup| {
                App::install(ui.ctx());
                if state.popup.show(ui) == Outcome::Closed {
                    state.closed = true;
                }
            },
            Popup {
                popup,
                closed: false,
            },
        );
    harness.run();
    harness
}

#[test]
fn the_popup_adds_the_line_to_the_list_and_keeps_no_note() -> TestResult {
    let (store, household, list) = store()?;
    let mut harness = popup(&store);
    harness.get_by(|n| n.is_focused()).type_text("Bread");
    harness.run();
    click(&mut harness, "File into…");
    click(&mut harness, "Household");
    pick(&mut harness, "Groceries");
    // Only the list shows; the rest waits unshown.
    assert!(shows(&harness, &t().capture_choice_shopping("Groceries")));
    assert!(!shows(&harness, "Into: Household"));
    for label in ["File into…", "Routine", "Frog", "10 min"] {
        assert!(!shows(&harness, label), "{label}");
    }
    assert!(shows(&harness, "Save"));
    // Dropped, the list gives the other choices back.
    let chip = t().capture_choice_shopping("Groceries");
    click(&mut harness, &t().remove_choice(&chip));
    assert!(shows(&harness, "Into: Household"));
    assert!(shows(&harness, "File into…"));
    assert_eq!(
        harness.state().popup.choices().place,
        Some(household.clone())
    );
    pick(&mut harness, "Groceries");
    click(&mut harness, "Save");
    assert!(harness.state().closed);
    assert_eq!(lines(&store, &list)?, ["Bread", "Milk"]);
    assert_eq!(others(&store, &list)?, Vec::<Note>::new());
    Ok(())
}

/// The window of `app` on the Agenda, its days on UTC.
fn window(app: App) -> Harness<'static, App> {
    let mut app = app;
    app.set_wall_clock(Arc::new(utc));
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

/// Types `rows` into the capture field, one line each.
fn type_rows(harness: &mut Harness<'_, App>, rows: &[&str]) {
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            harness.key_press(Key::Enter);
            harness.run();
        }
        harness.get_by(|n| n.is_focused()).type_text(row);
        harness.run();
    }
}

#[test]
fn each_line_typed_in_the_window_becomes_a_line_without_duplicates() -> TestResult {
    let (store, _, list) = store()?;
    let mut harness = window(App::new(store.clone(), "en"));
    type_rows(&mut harness, &["Eggs", "milk", "Butter"]);
    pick(&mut harness, "Groceries");
    assert!(shows(&harness, &t().capture_choice_shopping("Groceries")));
    click(&mut harness, "Save");
    // "milk" brings the Line there back instead of a second one.
    assert_eq!(lines(&store, &list)?, ["Butter", "Eggs", "Milk"]);
    assert_eq!(others(&store, &list)?, Vec::<Note>::new());
    assert!(!shows(&harness, &t().capture_choice_shopping("Groceries")));
    Ok(())
}

#[test]
fn it_is_not_offered_without_a_shopping_list() -> TestResult {
    let (store, _, list) = store()?;
    Notes::new(&store).trash(&list)?;
    let mut harness = window(App::new(store.clone(), "en"));
    type_rows(&mut harness, &["Eggs"]);
    click(&mut harness, "More");
    assert!(shows(&harness, "Important"));
    assert!(!shows(&harness, t().add_to_shopping_list()));
    Ok(())
}

struct Pictures;

impl Clipboard for Pictures {
    fn photo(&mut self) -> Option<Vec<u8>> {
        let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([20, 120, 200, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut out, image::ImageFormat::Png)
            .ok()?;
        Some(out.into_inner())
    }
}

#[test]
fn a_photo_keeps_the_capture_a_note() -> TestResult {
    let (store, _, list) = store()?;
    let mut app = App::new(store.clone(), "en");
    app.set_clipboard(Box::new(Pictures));
    let mut harness = window(app);
    type_rows(&mut harness, &["Bread"]);
    pick(&mut harness, "Groceries");
    click(&mut harness, t().paste());
    // The picture waits on the field until the capture is saved.
    assert!(others(&store, &list)?.is_empty());
    harness
        .get_by(|n| n.role() == Role::MultilineTextInput && n.value().as_deref() == Some("Bread"))
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::Enter);
    harness.run();
    let kept = others(&store, &list)?;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].text, "Bread");
    assert_eq!(kept[0].photos.len(), 1);
    assert_eq!(lines(&store, &list)?, ["Milk"]);
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

/// The window with the microphone and a transcriber hearing `heard`,
/// with a recording stopped and the list "Groceries" picked.
fn voice(
    store: &Arc<Store>,
    heard: Option<&'static str>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store.clone(), "en");
    app.set_recorder(Box::new(Mic::default()));
    let transcriber = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(Hears(heard)),
        Box::new(|| {}),
    );
    app.set_enrichers(Some(transcriber), None, None);
    let mut harness = window(app);
    click(&mut harness, t().record_voice());
    click(&mut harness, t().stop_recording());
    pick(&mut harness, "Groceries");
    Ok(harness)
}

#[test]
fn a_recordings_transcript_goes_into_the_list_once_it_is_there() -> TestResult {
    let (store, _, list) = store()?;
    let mut harness = voice(&store, Some("Eggs\nmilk"))?;
    click(&mut harness, "Save");
    let poured = eventually(|| others(&store, &list).is_ok_and(|n| n.is_empty()));
    assert!(poured, "{:?}", others(&store, &list)?);
    assert_eq!(lines(&store, &list)?, ["Eggs", "Milk"]);
    harness.run();
    assert!(!shows(&harness, t().capture_shopping_untranscribed()));
    Ok(())
}

#[test]
fn without_a_transcript_the_recording_stays_a_note_announced_at_the_top() -> TestResult {
    let (store, _, list) = store()?;
    let mut harness = voice(&store, None)?;
    click(&mut harness, "Save");
    // It waits Unsorted, holding the recording.
    let kept = others(&store, &list)?;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].recordings.len(), 1);
    assert_eq!(kept[0].place, None);
    let said = eventually(|| {
        harness.run();
        shows(&harness, t().capture_shopping_untranscribed())
    });
    assert!(said);
    assert_eq!(others(&store, &list)?.len(), 1);
    assert_eq!(lines(&store, &list)?, ["Milk"]);
    Ok(())
}

#[test]
fn a_recording_without_a_transcriber_is_not_offered() -> TestResult {
    let (store, _, _) = store()?;
    let mut app = App::new(store.clone(), "en");
    app.set_recorder(Box::new(Mic::default()));
    let mut harness = window(app);
    click(&mut harness, t().record_voice());
    click(&mut harness, t().stop_recording());
    click(&mut harness, "More");
    assert!(shows(&harness, "Important"));
    assert!(!shows(&harness, t().add_to_shopping_list()));
    Ok(())
}

#[test]
fn a_recording_without_a_transcriber_keeps_the_popups_capture_a_note_as_said() -> TestResult {
    let (store, _, list) = store()?;
    let mut harness = popup(&store);
    harness
        .state_mut()
        .popup
        .set_recorder(Box::new(Mic::default()));
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Bread");
    harness.run();
    pick(&mut harness, "Groceries");
    click(&mut harness, t().record_voice());
    click(&mut harness, t().stop_recording());
    click(&mut harness, "Save");
    // Kept with its recording and said at the top, the popup still open.
    assert!(!harness.state().closed);
    assert!(shows(&harness, t().capture_shopping_untranscribed()));
    let kept = others(&store, &list)?;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].text, "Bread");
    assert_eq!(kept[0].recordings.len(), 1);
    assert_eq!(lines(&store, &list)?, ["Milk"]);
    Ok(())
}
