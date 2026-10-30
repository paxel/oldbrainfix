//! Notes made from files in the window: "From file…" with the dialog
//! handed in and files dropped onto the window, held on the capture
//! field until it is saved with Save or on leaving, driven headless by
//! egui_kittest over an empty store with its clock held still. The
//! pictures are made here; the sound file is the phone's test voice
//! message, an Ogg Opus file as messengers send. The system's dialog and
//! Whisper itself are device-only.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Note, Notes, SigningKey, Store, parse_time};
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-27T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

/// The phone's test voice message: 0.2 s of a 440 Hz tone, 16 kHz mono,
/// encoded with libopus into an Ogg file.
fn voice_message() -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../oldbrainfix-whisper/testdata/voice-message.ogg"),
    )
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

/// The window over `store` speaking `locale`, its "From file…" dialog
/// answering `picked`.
fn window(store: Arc<Store>, locale: &str, picked: Vec<PathBuf>) -> Harness<'static, App> {
    let mut app = App::new(store, locale);
    app.set_files_picker(Box::new(move |_| picked.clone()));
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
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

fn notes(store: &Store) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).all()?)
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

#[test]
fn from_file_makes_one_note_of_a_picture_and_a_voice_message() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let (picture, voice) = (dir.path().join("receipt.png"), dir.path().join("Ben.ogg"));
    std::fs::write(&picture, png(40, 30))?;
    std::fs::write(&voice, voice_message()?)?;
    let mut harness = window(store.clone(), "en", vec![picture, voice]);
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("From Ben");
    harness.run();
    harness.get_by_label("From file…").click();
    harness.run();
    // Held on the field with what is typed, nothing saved yet.
    assert!(notes(&store)?.is_empty());
    assert!(shows(&harness, "Photo"));
    assert!(shows(&harness, "Voice recording"));
    harness.get_by_label("30 min").click();
    harness.run();
    let notes = notes(&store)?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "From Ben");
    assert_eq!(notes[0].photos.len(), 1);
    assert_eq!(notes[0].recordings.len(), 1);
    // The voice message is kept as it is.
    assert_eq!(store.audio(&notes[0].recordings[0]), Some(voice_message()?));
    // It decodes: nothing to announce.
    assert_eq!(harness.state().error(), None);
    assert!(!shows(
        &harness,
        "A recording from a file cannot be transcribed."
    ));
    Ok(())
}

#[test]
fn closing_the_dialog_without_a_file_changes_nothing() -> TestResult {
    let store = store()?;
    let mut harness = window(store.clone(), "en", Vec::new());
    harness.get_by_label("From file…").click();
    harness.run();
    assert!(notes(&store)?.is_empty());
    // Nothing held, nothing typed: leaving saves nothing.
    harness.state_mut().flush();
    assert!(notes(&store)?.is_empty());
    Ok(())
}

/// Reads a recording as Whisper would hear it, telling how much it
/// heard; what does not decode is not read.
struct Decoding;

impl Engine for Decoding {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        let samples = oldbrainfix_whisper::decode(bytes)?;
        Some(format!("{} samples", samples.len()))
    }
}

/// Waits until `done` holds, for at most ten seconds.
fn eventually(done: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(10);
    while !done() {
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

#[test]
fn a_dropped_voice_message_becomes_a_voice_note_and_is_transcribed() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let voice = dir.path().join("PTT-20261027-WA0001.opus");
    std::fs::write(&voice, voice_message()?)?;
    let mut harness = window(store.clone(), "en", Vec::new());
    let transcriber = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(Decoding),
        Box::new(|| {}),
    );
    harness
        .state_mut()
        .set_enrichers(Some(transcriber), None, None);
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(voice),
        ..Default::default()
    }];
    harness.run();
    assert!(notes(&store)?.is_empty());
    assert!(shows(&harness, "Voice recording"));
    harness.get_by_label("1 h").click();
    harness.run();
    let notes_now = notes(&store)?;
    assert_eq!(notes_now.len(), 1);
    assert!(notes_now[0].photos.is_empty());
    assert_eq!(notes_now[0].recordings.len(), 1);
    let hash = notes_now[0].recordings[0].clone();
    let heard = || {
        notes(&store)
            .ok()
            .and_then(|n| n.first().and_then(|n| n.transcripts.get(&hash).cloned()))
    };
    assert!(eventually(|| heard().is_some()), "never transcribed");
    // 0.2 s at Whisper's 16 kHz, give or take a frame.
    let count: usize = heard()
        .unwrap_or_default()
        .trim_end_matches(" samples")
        .parse()?;
    assert!((2_880..=3_520).contains(&count), "{count}");
    Ok(())
}

#[test]
fn a_dropped_sound_file_that_does_not_decode_is_kept_and_says_why() -> TestResult {
    let store = store()?;
    let mut harness = window(store.clone(), "en", Vec::new());
    harness.input_mut().dropped_files = vec![
        egui::DroppedFile {
            name: "voice.ogg".into(),
            bytes: Some(Arc::from(&b"not a recording"[..])),
            ..Default::default()
        },
        // Neither a picture nor a sound file: left alone.
        egui::DroppedFile {
            name: "notes.txt".into(),
            bytes: Some(Arc::from(&b"plain text"[..])),
            ..Default::default()
        },
    ];
    harness.run();
    // Said once the Note is saved, not while it is held.
    assert!(notes(&store)?.is_empty());
    assert!(!shows(
        &harness,
        "A recording from a file cannot be transcribed."
    ));
    harness.state_mut().flush();
    harness.run();
    let notes = notes(&store)?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].recordings.len(), 1);
    assert_eq!(
        store.audio(&notes[0].recordings[0]),
        Some(b"not a recording".to_vec())
    );
    // Announced at the top; the click shows why.
    harness
        .get_by_label("A recording from a file cannot be transcribed.")
        .click();
    harness.run();
    assert!(!shows(
        &harness,
        "A recording from a file cannot be transcribed."
    ));
    assert!(shows(&harness, "Recording not transcribed"));
    assert!(
        harness
            .query_all_by_label_contains("No sound was found in the file")
            .next()
            .is_some()
    );
    assert!(shows(&harness, "unreadable: .ogg"));
    assert!(shows(&harness, "Report"));
    harness.get_by_label("Close").click();
    harness.run();
    assert!(!shows(&harness, "Recording not transcribed"));
    Ok(())
}

#[test]
fn from_file_and_its_announcement_speak_german() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let broken = dir.path().join("Sprachnachricht.mp3");
    std::fs::write(&broken, b"ID3 but nothing after it")?;
    let mut harness = window(store.clone(), "de", vec![broken]);
    assert!(shows(&harness, "Aus Datei…"));
    harness.get_by_label("Aus Datei…").click();
    harness.run();
    assert!(notes(&store)?.is_empty());
    harness.get_by_label("1 Std.").click();
    harness.run();
    assert_eq!(notes(&store)?.len(), 1);
    harness
        .get_by_label("Eine Aufnahme aus einer Datei kann nicht transkribiert werden.")
        .click();
    harness.run();
    assert!(shows(&harness, "Aufnahme nicht transkribiert"));
    assert!(shows(&harness, "Schließen"));
    Ok(())
}
