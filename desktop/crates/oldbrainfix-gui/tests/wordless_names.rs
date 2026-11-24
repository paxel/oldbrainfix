//! A Note without text of its own named by its first Transcript or Photo
//! Text line (obf-1049), driven headless by egui_kittest: on the Agenda,
//! on All Routines and Reminders, in the calendar file, whose entry is
//! described by the whole, and on a photo Note's compact card. Nothing
//! is copied into the Note.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{ClockTime, Day, Notes, Routines, Schedule, SigningKey, Store, parse_time};
use oldbrainfix_gui::media;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// Wednesday noon in Europe.
const NOW: &str = "2026-10-28T11:00:00.000Z";

/// What the voice Note's recording says.
const PILLS: &str = "Pills\nthe blue ones";

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time(NOW)?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

/// A picture as a PNG file.
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([20, 120, 200, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    let written =
        image::DynamicImage::ImageRgba8(image).write_to(&mut out, image::ImageFormat::Png);
    assert!(written.is_ok());
    out.into_inner()
}

/// A Reminder today at `hour` of the Note `id`.
fn remind(store: &Store, id: &str, hour: u32) -> TestResult {
    let times: Vec<ClockTime> = ClockTime::new(hour, 0).into_iter().collect();
    let schedule = Schedule::new(Day::new(2026, 10, 28), &times, None).ok_or("a schedule")?;
    Routines::new(store).spawn(id, &schedule)?;
    Ok(())
}

/// A voice Note whose Transcript reads [`PILLS`] and one whose recording
/// is not transcribed, each with a Reminder this evening; their ids.
fn voice_notes(store: &Store) -> Result<(String, String), Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let pills = notes.capture_with("", None, None, &[], &[media::wav(&[0.0; 160])])?;
    notes.set_transcript(&pills.id, &pills.recordings[0], PILLS)?;
    let silent = notes.capture_with("", None, None, &[], &[media::wav(&[0.1; 160])])?;
    remind(store, &pills.id, 18)?;
    remind(store, &silent.id, 19)?;
    Ok((pills.id, silent.id))
}

/// The window over `app` on `selection`.
fn window(mut app: App, selection: Selection) -> Harness<'static, App> {
    app.set_wall_clock(Arc::new(berlin));
    app.select(selection);
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

/// Whether a node not a button is labelled or reads exactly `text`.
fn reads(harness: &Harness<'_, App>, text: &str) -> bool {
    named(harness, text).is_some()
}

/// Where the node not a button labelled or reading exactly `text` lies.
fn named(harness: &Harness<'_, App>, text: &str) -> Option<egui::Rect> {
    harness
        .query_all_by(|n| {
            n.role() != Role::Button
                && (n.label().as_deref() == Some(text) || n.value().as_deref() == Some(text))
        })
        .next()
        .map(|n| n.rect())
}

/// Whether anything in the window is labelled or reads with `text` in it.
fn shows(harness: &Harness<'_, App>, text: &str) -> bool {
    harness
        .query_all_by(|n| {
            [n.label(), n.value()]
                .into_iter()
                .flatten()
                .any(|s| s.contains(text))
        })
        .next()
        .is_some()
}

#[test]
fn a_voice_note_is_named_by_its_transcript_on_the_agenda_and_all_routines() -> TestResult {
    let store = store()?;
    let (pills, _) = voice_notes(&store)?;
    let harness = window(App::new(store.clone(), "en"), Selection::Agenda);
    assert!(shows(&harness, "Pills"));
    assert!(!shows(&harness, "the blue ones"), "only its first line");
    assert!(shows(&harness, "Voice recording"), "the untranscribed one");
    let harness = window(App::new(store.clone(), "en"), Selection::AllRoutines);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Pills")
            .is_some()
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Voice recording")
            .is_some()
    );
    // Nothing of it is copied into the Note.
    let note = Notes::new(&store).by_id(&pills)?.ok_or("the Note")?;
    assert_eq!(note.text, "");
    Ok(())
}

#[test]
fn the_calendar_file_titles_a_voice_note_by_its_transcript_and_describes_it_whole() -> TestResult {
    let store = store()?;
    voice_notes(&store)?;
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("routines.ics");
    let mut app = App::new(store, "en");
    let path = saved.clone();
    app.set_calendar_saver(Box::new(move |_, _| Some(path.clone())));
    let mut harness = window(app, Selection::AllRoutines);
    harness.get_by_label("Export").click_accesskit();
    harness.run();
    let written = read(&saved)?;
    assert!(written.contains("SUMMARY:Pills\r\n"), "{written}");
    assert!(
        written.contains("DESCRIPTION:Pills\\nthe blue ones\r\n"),
        "{written}"
    );
    assert!(written.contains("SUMMARY:Voice recording\r\n"), "{written}");
    Ok(())
}

/// The calendar file at `path`, its folded lines unfolded.
fn read(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(path)?.replace("\r\n ", ""))
}

#[test]
fn a_photo_note_without_words_is_named_by_its_photo_text_on_its_compact_card() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let receipt = notes.capture_with("", None, None, &[png(40, 30)], &[])?;
    notes.set_photo_text(&receipt.id, &receipt.photos[0], "  RECEIPT\nTotal 12,50")?;
    // A Transcript with words names it instead, and shows under its
    // recording already.
    let both = notes.capture_with("", None, None, &[png(30, 40)], &[media::wav(&[0.0; 160])])?;
    notes.set_photo_text(&both.id, &both.photos[0], "SALE")?;
    notes.set_transcript(&both.id, &both.recordings[0], PILLS)?;
    let mut harness = window(App::new(store, "en"), Selection::All);
    harness
        .get_by_role_and_label(Role::Button, "Show compact views")
        .click();
    harness.run();
    assert!(harness.state().compact());
    // Its card goes by the name, which shows above its picture.
    let card = harness
        .get_by_role_and_label(Role::Button, "RECEIPT")
        .rect();
    let name = named(&harness, "RECEIPT").ok_or("the name")?;
    assert!(card.contains_rect(name));
    let photo = harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .map(|n| n.rect())
        .find(|r| card.contains_rect(*r))
        .ok_or("its photo")?;
    assert!(name.bottom() <= photo.top());
    assert!(!shows(&harness, "Total 12,50"), "only its first line");
    // The one named by its Transcript gets no name line of its own.
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Pills")
            .is_some()
    );
    assert!(!reads(&harness, "Pills"));
    assert!(!reads(&harness, "SALE"));
    Ok(())
}
