//! Voice recordings, photos, Transcripts, Photo Texts and the search over
//! them in the window and the capture popup, driven headless by
//! egui_kittest with a fake microphone, speakers, clipboard and engines.
//! The real microphone, speakers, clipboard and Whisper are device-only.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::Duration;
use egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, SigningKey, Store, WireEntry, dart_iso, parse_time};
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::media::{self, Clipboard, Player, Recorder};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::speech::{Download, SpeechModel, Status};
use oldbrainfix_gui::{App, Selection};
use oldbrainfix_whisper::model::Outcome as ModelOutcome;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn empty_store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

/// The fixtures' `media` scenario with its photos and recordings, as
/// the Dart core wrote them.
fn media_store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/media");
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json"))?)?;
    let now = parse_time(expected["now"].as_str().ok_or("expected.json names now")?)?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    for line in std::fs::read_to_string(dir.join("log.jsonl"))?.lines() {
        let entry: WireEntry = serde_json::from_str(line)?;
        store.insert(&entry)?;
    }
    for entry in std::fs::read_dir(dir.join("photos"))? {
        store.put_photo(&std::fs::read(entry?.path())?)?;
    }
    for entry in std::fs::read_dir(dir.join("audio"))? {
        store.put_audio(&std::fs::read(entry?.path())?)?;
    }
    Ok(Arc::new(store))
}

/// What the fake devices were asked.
#[derive(Default)]
struct Devices {
    /// Whether the microphone may start.
    mic: bool,
    recording: bool,
    starts: u32,
    played: Vec<Vec<u8>>,
    playing: bool,
    stopped: u32,
    /// The clipboard's photo, if any.
    clipboard: Option<Vec<u8>>,
}

type Shared = Rc<RefCell<Devices>>;

struct FakeMic(Shared);

/// What the fake microphone hears: a tenth of a second of silence.
fn heard() -> Vec<u8> {
    media::wav(&[0.0; 1600])
}

impl Recorder for FakeMic {
    fn start(&mut self) -> bool {
        let mut d = self.0.borrow_mut();
        d.starts += 1;
        d.recording = d.mic;
        d.mic
    }

    fn stop(&mut self) -> Option<Vec<u8>> {
        let mut d = self.0.borrow_mut();
        std::mem::take(&mut d.recording).then(heard)
    }

    fn recording(&self) -> bool {
        self.0.borrow().recording
    }

    fn elapsed(&self) -> StdDuration {
        StdDuration::from_secs(75)
    }
}

struct FakeSpeakers(Shared);

impl Player for FakeSpeakers {
    fn play(&mut self, bytes: Vec<u8>) -> bool {
        let mut d = self.0.borrow_mut();
        d.played.push(bytes);
        d.playing = true;
        true
    }

    fn stop(&mut self) {
        let mut d = self.0.borrow_mut();
        d.stopped += 1;
        d.playing = false;
    }

    fn playing(&self) -> bool {
        self.0.borrow().playing
    }
}

struct FakeClipboard(Shared);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        self.0.borrow().clipboard.clone()
    }
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

/// The window over `store`, with the fake devices.
fn window(store: Arc<Store>, devices: &Shared) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.set_recorder(Box::new(FakeMic(devices.clone())));
    app.set_player(Box::new(FakeSpeakers(devices.clone())));
    app.set_clipboard(Box::new(FakeClipboard(devices.clone())));
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

/// The text field with the hint `hint`.
fn field<'a, S>(harness: &'a Harness<'_, S>, hint: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(|n| n.placeholder() == Some(hint))
}

/// Types `text` into the field with the hint `hint`.
fn type_into<S>(harness: &mut Harness<'_, S>, hint: &str, text: &str) {
    field(harness, hint).click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

#[test]
fn recording_starts_at_once_and_the_big_stop_button_keeps_the_note() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().mic = true;
    let mut harness = window(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    assert!(harness.state().recording());
    assert!(shows(&harness, "Recording… 1:15"));
    // While recording, the Nudge times keep it too.
    assert!(shows(&harness, "10 min"));
    harness.get_by_label("Stop and save").click();
    harness.run();
    assert!(!harness.state().recording());
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    let note = &notes[0];
    assert_eq!(note.text, "");
    assert_eq!(note.recordings.len(), 1);
    assert_eq!(store.audio(&note.recordings[0]), Some(heard()));
    assert_eq!(
        store.current(&note.id, "nudge")?,
        Some(dart_iso(store.now() + Duration::hours(1)))
    );
    // The new Note shows in the editor with its play button.
    assert_eq!(harness.state().selected_note(), Some(note.id.as_str()));
    harness.get_by_label("Play the voice recording").click();
    // While it plays, the window looks every quarter second.
    harness.run_steps(3);
    assert_eq!(devices.borrow().played, vec![heard()]);
    harness.get_by_label("Stop").click();
    harness.run();
    assert_eq!(devices.borrow().stopped, 2, "before playing, and asked");
    Ok(())
}

#[test]
fn a_recording_and_the_typed_text_go_into_one_note_with_the_picked_nudge() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().mic = true;
    let mut harness = window(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    type_into(&mut harness, "What's on your mind?", "Garage");
    harness.get_by_label("10 min").click();
    harness.run();
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Garage");
    assert_eq!(notes[0].recordings.len(), 1);
    assert_eq!(
        store.current(&notes[0].id, "nudge")?,
        Some(dart_iso(store.now() + Duration::minutes(10)))
    );
    Ok(())
}

#[test]
fn closing_the_window_while_recording_keeps_the_recording() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().mic = true;
    let mut harness = window(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    harness.state_mut().flush();
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].recordings.len(), 1);
    Ok(())
}

#[test]
fn without_a_microphone_the_top_says_so_and_nothing_is_kept() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    assert_eq!(devices.borrow().starts, 1);
    assert_eq!(
        harness.state().error(),
        Some("Voice Notes need the microphone.")
    );
    assert!(Notes::new(&store).all()?.is_empty());
    Ok(())
}

#[test]
fn a_pasted_picture_becomes_a_photo_note_kept_the_phones_way() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.get_by_label("Paste a picture").click();
    harness.run();
    assert_eq!(
        harness.state().error(),
        Some("There is no picture on the clipboard.")
    );
    let photo = media::photo_from_file(&png(40, 30)).ok_or("a photo")?;
    devices.borrow_mut().clipboard = Some(photo.clone());
    type_into(&mut harness, "What's on your mind?", "Parking spot");
    harness.get_by_label("Paste a picture").click();
    harness.run();
    assert_eq!(harness.state().error(), None);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Parking spot");
    assert_eq!(notes[0].photos.len(), 1);
    assert_eq!(store.photo(&notes[0].photos[0]), Some(photo));
    // The card and the editor show it.
    assert!(harness.query_all_by_label("Photo").count() >= 2);
    Ok(())
}

#[test]
fn pictures_dropped_onto_the_window_become_one_photo_note() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("receipt.png");
    std::fs::write(&path, png(3000, 1000))?;
    harness.input_mut().dropped_files = vec![
        egui::DroppedFile {
            path: Some(path),
            ..Default::default()
        },
        egui::DroppedFile {
            bytes: Some(Arc::from(png(10, 10))),
            ..Default::default()
        },
        // Not a picture: left alone.
        egui::DroppedFile {
            bytes: Some(Arc::from(&b"plain text"[..])),
            ..Default::default()
        },
    ];
    harness.run();
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].photos.len(), 2);
    let kept = store.photo(&notes[0].photos[0]).ok_or("the photo")?;
    let size = image::load_from_memory(&kept).map(|i| (i.width(), i.height()))?;
    assert_eq!(size, (2048, 683));
    Ok(())
}

#[test]
fn the_editor_shows_photos_recordings_and_their_transcripts() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store, &devices);
    // Shed: two Photos, an empty Transcript; Groceries: a Transcript.
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000003");
    harness.run();
    assert!(shows(&harness, "No words heard."));
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000002");
    harness.run();
    assert!(harness.query_all_by_label("Milk, eggs and bread").count() >= 2);
    // Dentist: a photo from another Device whose bytes are not here, and
    // the merged Notes' photo and recording in its Sections.
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000004");
    harness.run();
    assert!(shows(&harness, "Photo not on this device"));
    assert!(
        harness
            .query_all_by_label("Play the voice recording")
            .count()
            >= 1
    );
    Ok(())
}

#[test]
fn search_finds_transcripts_photo_texts_and_sections() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store, &devices);
    type_into(&mut harness, "Search all Notes", "milk");
    assert_eq!(
        harness.state().shown_notes(),
        vec!["note:00000000000000000000000000000002"]
    );
    // The Photo Text of a Note merged into another, in its Section.
    harness.state_mut().set_query("MÜLLER");
    harness.run();
    assert_eq!(
        harness.state().shown_notes(),
        vec!["note:00000000000000000000000000000004"]
    );
    harness.get_by_label("Clear search").click();
    harness.run();
    assert_eq!(harness.state().shown_notes().len(), 4);
    // Another level starts with a search of its own.
    harness.state_mut().set_query("receipt");
    harness.state_mut().select(Selection::Unsorted);
    harness.state_mut().select(Selection::All);
    harness.run();
    assert_eq!(harness.state().shown_notes().len(), 4);
    Ok(())
}

/// Reads every recording or photo as "heard <length>", and fails on
/// what is longer than `fail_over` bytes.
struct FakeEngine {
    fail_over: usize,
}

impl Engine for FakeEngine {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        (bytes.len() <= self.fail_over).then(|| format!(" heard {} ", bytes.len()))
    }
}

/// Waits until `done` holds, for at most ten seconds.
fn eventually(done: impl Fn() -> bool) -> bool {
    let until = Instant::now() + StdDuration::from_secs(10);
    while !done() {
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(StdDuration::from_millis(5));
    }
    true
}

#[test]
fn the_background_reads_what_waits_and_leaves_what_it_cannot() -> TestResult {
    let store = media_store()?;
    let changes = Arc::new(AtomicU64::new(0));
    let bumped = changes.clone();
    let photos = Enricher::spawn(
        store.clone(),
        Kind::PhotoTexts,
        Box::new(FakeEngine { fail_over: 1000 }),
        Box::new(move || {
            bumped.fetch_add(1, Ordering::SeqCst);
        }),
    );
    photos.kick();
    let notes = Notes::new(&store);
    // The photo from another Device is not here; it waits for its bytes.
    let left = || notes.unread_photos().map(|w| w.len()).unwrap_or(0);
    assert!(eventually(|| left() == 1), "{:?}", notes.unread_photos());
    let shed = notes
        .by_id("note:00000000000000000000000000000003")?
        .ok_or("shed")?;
    let read = shed.photo_texts.get(&shed.photos[1]).ok_or("read")?;
    assert!(read.starts_with("heard "), "trimmed: {read:?}");
    assert!(changes.load(Ordering::SeqCst) > 0);
    let transcripts = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(FakeEngine { fail_over: 0 }),
        Box::new(|| {}),
    );
    transcripts.kick();
    std::thread::sleep(StdDuration::from_millis(100));
    // None could be read: all still wait, none is lost.
    assert_eq!(notes.untranscribed()?.len(), 2);
    assert_eq!(transcripts.current(), None);
    Ok(())
}

#[test]
fn transcripts_wait_for_the_speech_model_and_the_settings_fetch_it() -> TestResult {
    let store = media_store()?;
    let tmp = tempfile::tempdir()?;
    let wrong: Download = Arc::new(|_, _, _| ModelOutcome::Mismatch);
    let speech = SpeechModel::with_download(tmp.path().join("whisper"), wrong, Arc::new(|| {}));
    assert!(eventually(|| speech.status() == Status::Missing));
    let transcripts = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(FakeEngine { fail_over: 1000 }),
        Box::new(|| {}),
    );
    let mut app = App::new(store.clone(), "en");
    app.set_enrichers(Some(transcripts), None, Some(speech.clone()));
    app.select(Selection::Settings);
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
    std::thread::sleep(StdDuration::from_millis(100));
    assert_eq!(Notes::new(&store).untranscribed()?.len(), 2, "no model yet");
    assert!(shows(&harness, "Speech model"));
    assert!(shows(&harness, "No speech model yet"));
    assert!(shows(
        &harness,
        "The speech model is the base model of Whisper by OpenAI, Copyright (c) 2022 OpenAI, under the MIT License."
    ));
    harness.get_by_label("Download (148 MB)").click();
    harness.run();
    assert!(eventually(|| speech.outcome().is_some()));
    harness.run();
    assert!(shows(
        &harness,
        "That is not the expected speech model, so it was not kept."
    ));
    // A file of the user's own that is not the model is refused alike.
    let file = tmp.path().join("ggml-base.bin");
    std::fs::write(&file, b"not whisper")?;
    type_into(
        &mut harness,
        "Path to ggml-base.bin",
        &file.display().to_string(),
    );
    harness.get_by_label("Use this file").click();
    harness.run();
    assert!(eventually(|| speech.status() == Status::Missing));
    assert_eq!(Notes::new(&store).untranscribed()?.len(), 2);
    Ok(())
}

/// A popup with the fake microphone.
struct Popup {
    popup: CapturePopup,
    closed: bool,
}

fn popup(store: Arc<Store>, devices: &Shared) -> Harness<'static, Popup> {
    let mut popup = CapturePopup::new(store, "en");
    popup.set_recorder(Box::new(FakeMic(devices.clone())));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(460.0, 280.0))
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
fn the_popup_records_and_its_stop_button_keeps_the_note() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().mic = true;
    let mut harness = popup(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    assert!(harness.state().popup.recording());
    let stop = harness.get_by_label("Stop and save");
    assert!(stop.rect().bottom() <= 280.0, "{:?}", stop.rect());
    stop.click();
    harness.run();
    assert!(harness.state().closed);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].recordings.len(), 1);
    Ok(())
}

#[test]
fn esc_while_the_popup_records_keeps_what_was_said_and_typed() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().mic = true;
    let mut harness = popup(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Call back");
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Call back");
    assert_eq!(notes[0].recordings.len(), 1);
    Ok(())
}

#[test]
fn the_merge_dialog_finds_notes_by_their_transcripts_too() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store, &devices);
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000001");
    harness.run();
    harness.get_by_label("Merge into…").click();
    harness.run();
    let count = |harness: &Harness<'_, App>, label| harness.query_all_by_label(label).count();
    let (groceries, shed) = (count(&harness, "Groceries"), count(&harness, "Shed"));
    // The dialog's field has the keyboard as it opens.
    harness.get_by(|n| n.is_focused()).type_text("bread");
    harness.run();
    // Its row in the dialog stays; the other's goes, its card stays.
    assert_eq!(count(&harness, "Groceries"), groceries);
    assert_eq!(count(&harness, "Shed"), shed - 1);
    Ok(())
}
