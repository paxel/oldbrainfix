//! Voice recordings, photos, Transcripts, Photo Texts and the search over
//! them in the window and the capture popup, driven headless by
//! egui_kittest with a fake microphone, speakers, clipboard and engines.
//! The real microphone, speakers, clipboard and Whisper are device-only.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::Duration;
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, SigningKey, Store, WireEntry, dart_iso, parse_time};
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::media::{self, Clipboard, Player, Recorder};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::speech::{Download, SpeechModel, Status};
use oldbrainfix_gui::{App, Selection};
use oldbrainfix_whisper::model::{ModelDir, Outcome as ModelOutcome, WhisperModel, copy_hashing};

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
    /// Every playback speed the speakers were given.
    speeds: Vec<f64>,
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

    fn set_speed(&mut self, speed: f64) {
        self.0.borrow_mut().speeds.push(speed);
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
    window_in(store, devices, "en")
}

/// The window over `store` speaking `locale`, with the fake devices.
fn window_in(store: Arc<Store>, devices: &Shared, locale: &str) -> Harness<'static, App> {
    let mut app = App::new(store, locale);
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

/// Where `label` shows furthest right: in the editor, when the card in
/// the middle shows it too (#114).
fn rightmost<S>(harness: &Harness<'_, S>, label: &str) -> egui::Rect {
    harness
        .query_all_by_label(label)
        .map(|n| n.rect())
        .max_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap_or_else(|| panic!("no {label:?}"))
}

#[test]
fn recording_starts_at_once_and_the_big_stop_button_holds_it_until_saved() -> TestResult {
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
    harness.get_by_label("Stop").click();
    harness.run();
    assert!(!harness.state().recording());
    // Stopped, it stays on the capture field with the buttons, unsaved.
    assert!(Notes::new(&store).all()?.is_empty());
    assert!(shows(&harness, "Voice recording"));
    assert!(shows(&harness, "Routine"));
    harness.get_by_label("1 h").click();
    harness.run();
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
        harness.state().notice(),
        Some("Voice Notes need the microphone.")
    );
    assert!(Notes::new(&store).all()?.is_empty());
    // The click opens what to do, with Close and Report.
    harness
        .get_by_label("Voice Notes need the microphone.")
        .click();
    harness.run();
    assert_eq!(harness.state().notice(), None);
    assert!(shows(&harness, "Not done"));
    assert!(shows(&harness, "Report"));
    Ok(())
}

#[test]
fn a_pasted_picture_becomes_a_photo_note_kept_the_phones_way() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.get_by_label("Paste").click();
    harness.run();
    // Nothing there is said in grey, not announced (#130).
    assert_eq!(harness.state().notice(), None);
    assert!(shows(
        &harness,
        "Nothing to paste: the clipboard holds no text and no picture."
    ));
    let photo = media::photo_from_file(&png(40, 30)).ok_or("a photo")?;
    devices.borrow_mut().clipboard = Some(photo.clone());
    type_into(&mut harness, "What's on your mind?", "Parking spot");
    harness.get_by_label("Paste").click();
    harness.run();
    assert_eq!(harness.state().error(), None);
    // The picture waits on the field until the capture is saved.
    assert!(Notes::new(&store).all()?.is_empty());
    assert!(shows(&harness, "Photo"));
    harness
        .get_by(|n| {
            n.role() == Role::MultilineTextInput && n.value().as_deref() == Some("Parking spot")
        })
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::Enter);
    harness.run();
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
    // Both pictures wait on the empty field until the capture is saved.
    assert!(Notes::new(&store).all()?.is_empty());
    assert_eq!(harness.query_all_by_label("Photo").count(), 2);
    field(&harness, "What's on your mind?").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::Enter);
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
    assert_eq!(harness.state().shown_notes().len(), 6);
    // Another level starts with a search of its own.
    harness.state_mut().set_query("receipt");
    harness.state_mut().select(Selection::Unsorted);
    harness.state_mut().select(Selection::All);
    harness.run();
    assert_eq!(harness.state().shown_notes().len(), 6);
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
    let wrong: Download = Arc::new(|_, _, _, _| ModelOutcome::Mismatch);
    let dir = ModelDir::new(tmp.path().join("whisper"));
    let speech = SpeechModel::with_download(dir, wrong, Arc::new(|| {}));
    assert!(eventually(|| speech.status() == Status::Missing));
    let transcripts = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(FakeEngine { fail_over: 1000 }),
        Box::new(|| {}),
    );
    let mut app = App::new(store.clone(), "en");
    app.set_enrichers(Some(transcripts), None, Some(speech.clone()));
    // The system's dialog hands back the file of the user's own.
    let file = tmp.path().join("ggml-base.bin");
    std::fs::write(&file, b"not whisper")?;
    let picked = file.clone();
    app.set_model_picker(Box::new(move |title| {
        assert_eq!(title, "Use a file of my own");
        Some(picked.clone())
    }));
    app.select(Selection::Settings);
    let mut harness = settings(app);
    harness.run();
    std::thread::sleep(StdDuration::from_millis(100));
    assert_eq!(Notes::new(&store).untranscribed()?.len(), 2, "no model yet");
    assert!(shows(&harness, "Speech model"));
    assert!(shows(&harness, "No speech model yet"));
    assert!(shows(
        &harness,
        "The speech models are the base and small models of Whisper by OpenAI, Copyright (c) 2022 OpenAI, under the MIT License."
    ));
    assert!(shows(
        &harness,
        "Better with English words in German speech, but slower, and it needs more memory."
    ));
    harness.get_by_label("Small, 488 MB").click();
    harness.run();
    assert!(eventually(|| speech.outcome().is_some()));
    harness.run();
    assert!(shows(
        &harness,
        "That is not the expected speech model, so it was not kept."
    ));
    // A file of the user's own that is not the model is refused alike.
    harness.get_by_label("Use a file of my own").click();
    harness.run();
    assert!(eventually(|| speech.status() == Status::Missing));
    assert_eq!(Notes::new(&store).untranscribed()?.len(), 2);
    Ok(())
}

/// The Settings of `app`, tall enough for the speech model's card.
fn settings(app: App) -> Harness<'static, App> {
    Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        )
}

/// The SHA-256 of `bytes`, as the model directory reads it.
fn sha(bytes: &[u8]) -> String {
    let hash = copy_hashing(bytes, std::io::sink(), &AtomicBool::new(false), |_| {});
    hash.ok().flatten().unwrap_or_default()
}

#[test]
fn choosing_a_model_downloads_it_and_keeps_only_that_one() -> TestResult {
    let store = media_store()?;
    let tmp = tempfile::tempdir()?;
    let whisper = tmp.path().join("whisper");
    // Stand-ins for the models: the bytes "base" and "small".
    let dir = ModelDir::pinned_to(&whisper, &sha(b"base"), &sha(b"small"));
    let gate = Arc::new(AtomicBool::new(false));
    let open = gate.clone();
    let mirror: Download = Arc::new(move |dir, model, stop, _| {
        let bytes: &[u8] = match model {
            WhisperModel::Base => b"base",
            WhisperModel::Small => {
                while !open.load(Ordering::SeqCst) {
                    std::thread::sleep(StdDuration::from_millis(1));
                }
                b"small"
            }
        };
        dir.take_in(bytes, None, Some(model), stop, |_, _| {})
    });
    let speech = SpeechModel::with_download(dir, mirror, Arc::new(|| {}));
    assert!(eventually(|| speech.status() == Status::Missing));
    let transcripts = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(FakeEngine {
            fail_over: usize::MAX,
        }),
        Box::new(|| {}),
    );
    let mut app = App::new(store.clone(), "en");
    app.set_enrichers(Some(transcripts), None, Some(speech.clone()));
    app.select(Selection::Settings);
    let mut harness = settings(app);
    harness.run();
    harness.get_by_label("Base, 148 MB").click();
    // A download in progress repaints, so step instead of waiting for stillness.
    harness.run_steps(4);
    assert!(eventually(|| speech.current() == Some(WhisperModel::Base)));
    harness.run_steps(4);
    assert!(shows(
        &harness,
        "The speech model is ready. Voice Notes are transcribed on this computer."
    ));
    // The model came: what waited is transcribed.
    assert!(eventually(|| Notes::new(&store)
        .untranscribed()
        .is_ok_and(|w| w.is_empty())));
    // The small one downloads while the base one stays in use.
    harness.get_by_label("Small, 488 MB").click();
    // A download repaints to show how far it came.
    harness.run_steps(3);
    assert!(matches!(
        speech.status(),
        Status::Fetching {
            downloading: true,
            ..
        }
    ));
    assert_eq!(speech.current(), Some(WhisperModel::Base));
    assert_eq!(speech.chosen(), Some(WhisperModel::Small));
    assert!(shows(&harness, "Downloading: 0 of 488 MB"));
    gate.store(true, Ordering::SeqCst);
    assert!(eventually(|| speech.status() == Status::Ready));
    harness.run();
    assert_eq!(speech.current(), Some(WhisperModel::Small));
    assert_eq!(std::fs::read(speech.model())?, b"small");
    let mut kept: Vec<_> = std::fs::read_dir(&whisper)?
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();
    kept.sort();
    // The model, with the record of its checksum.
    assert_eq!(
        kept,
        ["ggml-base.bin", "ggml-base.bin.checked"],
        "only the chosen model is kept"
    );
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
        .with_size(egui::vec2(480.0, 340.0))
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
fn the_popup_records_and_keeps_the_recording_stopped_until_left() -> TestResult {
    let store = empty_store()?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().mic = true;
    let mut harness = popup(store.clone(), &devices);
    harness.get_by_label("Record voice").click();
    harness.run();
    assert!(harness.state().popup.recording());
    // All of it fits the popup's window while recording.
    let hint = harness.get_by_label_contains("Enter saves it with a Nudge in 1 h.");
    assert!(hint.rect().bottom() <= 340.0, "{:?}", hint.rect());
    harness.get_by_label("Stop").click();
    harness.run();
    // Stopped, the popup stays with the recording and room to type.
    assert!(!harness.state().closed);
    assert!(!harness.state().popup.recording());
    assert!(Notes::new(&store).all()?.is_empty());
    assert!(harness.query_by_label("Voice recording").is_some());
    harness.get_by(|n| n.is_focused()).type_text("Tyres");
    harness.run();
    // Leaving saves it all together.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Tyres");
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
    harness.get_by_label("Merge into…").click_accesskit();
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

const SHED: &str = "note:00000000000000000000000000000003";
const DENTIST: &str = "note:00000000000000000000000000000004";

#[test]
fn only_a_photo_or_recording_with_a_text_offers_its_removal() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store, &devices);
    // Shed: a photo with a Photo Text, one with an empty one and one
    // without; a recording with an empty Transcript and one without.
    harness.state_mut().select_note(SHED);
    harness.run();
    assert_eq!(
        harness
            .query_all_by_label("Remove photo, keep text")
            .count(),
        1
    );
    assert!(!shows(&harness, "Remove recording, keep text"));
    // Groceries: a photo without a Photo Text, its recording removed and
    // the Transcript on the card and in the editor.
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000002");
    harness.run();
    assert!(!shows(&harness, "Remove photo, keep text"));
    assert!(harness.query_all_by_label("Milk, eggs and bread").count() >= 2);
    // A Note whose only photo was removed shows its Photo Text alone.
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000001");
    harness.run();
    assert!(harness.query_all_by_label("RECEIPT\nTotal 12,50").count() >= 2);
    // Dentist: the removed photo of a merged Note, in its Section.
    harness.state_mut().select_note(DENTIST);
    harness.run();
    assert!(shows(&harness, "Dr. Müller, Tuesday 9:00"));
    assert!(!shows(&harness, "Remove recording, keep text"));
    Ok(())
}

#[test]
fn removing_a_photo_asks_once_and_keeps_its_photo_text() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.state_mut().select_note(SHED);
    harness.run();
    let photo = "0b6a0c7fce2f928971ccdff4431e34871dabb53e11455a29afc2fd5bed4d2e90";
    // The editor shows the Photo Text under its photo, above its button
    // (#97); the full card shows it under the photo too (#114).
    let spare = "Spare key under the pot";
    assert_eq!(harness.query_all_by_label(spare).count(), 2);
    let button = harness.get_by_label("Remove photo, keep text").rect();
    assert!(rightmost(&harness, spare).bottom() <= button.top());
    // Cancel keeps it.
    harness.get_by_label("Remove photo, keep text").click();
    harness.run();
    assert!(shows(
        &harness,
        "Remove this photo and keep its Photo Text?"
    ));
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(!shows(
        &harness,
        "Remove this photo and keep its Photo Text?"
    ));
    let shed = Notes::new(&store).by_id(SHED)?.ok_or("Shed")?;
    assert_eq!(shed.photos.len(), 3);
    assert!(shed.removed_photos.is_empty());
    // Remove takes it; the Photo Text stays where it was.
    harness.get_by_label("Remove photo, keep text").click();
    harness.run();
    harness.get_by_label("Remove").click();
    harness.run();
    assert!(!shows(
        &harness,
        "Remove this photo and keep its Photo Text?"
    ));
    assert_eq!(harness.state().error(), None);
    let shed = Notes::new(&store).by_id(SHED)?.ok_or("Shed")?;
    assert_eq!(shed.photos.len(), 2);
    assert_eq!(shed.removed_photos, vec![photo.to_string()]);
    assert!(store.photo(photo).is_none(), "its bytes go");
    // On the card and in the editor.
    assert_eq!(harness.query_all_by_label(spare).count(), 2);
    assert!(!shows(&harness, "Remove photo, keep text"));
    // Search still finds it.
    type_into(&mut harness, "Search all Notes", "spare key");
    assert_eq!(harness.state().shown_notes(), vec![SHED]);
    Ok(())
}

#[test]
fn removing_a_recording_of_a_merged_note_keeps_its_transcript() -> TestResult {
    let store = media_store()?;
    let merged = "note:00000000000000000000000000000006";
    let recording = "f30aa31ce85ada5e75454198abfd7a73294f261d712e5ec3dbe7a8846606879d";
    Notes::new(&store).set_transcript(merged, recording, "Bring the X-ray")?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.state_mut().select_note(DENTIST);
    harness.run();
    // The Capture Context above takes room: it lies at the bottom.
    harness
        .get_by_label("Remove recording, keep text")
        .scroll_to_me();
    harness.run();
    harness.get_by_label("Remove recording, keep text").click();
    harness.run();
    assert!(shows(
        &harness,
        "Remove this recording and keep its Transcript?"
    ));
    harness.get_by_label("Cancel").click();
    harness.run();
    let note = Notes::new(&store).by_id(merged)?.ok_or("merged")?;
    assert_eq!(note.recordings, vec![recording.to_string()]);
    harness.get_by_label("Remove recording, keep text").click();
    harness.run();
    harness.get_by_label("Remove").click();
    harness.run();
    let note = Notes::new(&store).by_id(merged)?.ok_or("merged")?;
    assert!(note.recordings.is_empty());
    assert_eq!(note.removed_recordings, vec![recording.to_string()]);
    assert!(shows(&harness, "Bring the X-ray"));
    assert!(!shows(&harness, "Remove recording, keep text"));
    Ok(())
}

/// A store on disk in `dir`, opened anew on every call.
fn disk_store(dir: &std::path::Path) -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::open(&dir.join("log.db"), Box::new(move || now), || {
        Ok(SigningKey::from_seed([9; 32]))
    })?;
    Ok(Arc::new(store))
}

/// Records a voice Note through the fake microphone; it opens in the
/// editor with its play button.
fn record_note<S>(harness: &mut Harness<'_, S>, devices: &Shared) {
    devices.borrow_mut().mic = true;
    harness.get_by_label("Record voice").click();
    harness.run();
    harness.get_by_label("Stop").click();
    harness.run();
    harness.get_by_label("1 h").click();
    harness.run();
}

#[test]
fn the_playback_speed_changes_what_plays_and_stays_on_this_device() -> TestResult {
    let dir = tempfile::tempdir()?;
    let devices: Shared = Rc::default();
    {
        let mut harness = window(disk_store(dir.path())?, &devices);
        record_note(&mut harness, &devices);
        // Normal speed beside the play button until another is chosen.
        assert!(shows(&harness, "1×"));
        harness.get_by_label("Play the voice recording").click();
        harness.run_steps(3);
        assert_eq!(devices.borrow().speeds.last(), Some(&1.0));
        harness.get_by_label("1×").click();
        harness.run_steps(3);
        assert!(shows(&harness, "Playback speed"));
        for option in ["×0.5", "×0.75", "×1", "×2"] {
            assert!(shows(&harness, option), "{option}");
        }
        // Chosen while playing: the recording plays on, faster.
        harness.get_by_label("×1.5").click();
        harness.run_steps(3);
        assert!(shows(&harness, "1.5×"));
        assert!(!shows(&harness, "1×"));
        assert!(!shows(&harness, "×2"), "the menu closed");
        let d = devices.borrow();
        assert_eq!(d.speeds.last(), Some(&1.5));
        assert_eq!(d.played.len(), 1, "not started again");
        assert!(d.playing);
    }
    // The next start of the window plays at the speed chosen.
    devices.borrow_mut().speeds.clear();
    let store = disk_store(dir.path())?;
    assert_eq!(
        store.local_setting("playbackSpeed")?.as_deref(),
        Some("1.5")
    );
    let mut harness = window(store, &devices);
    assert_eq!(devices.borrow().speeds, vec![1.5]);
    let notes = Notes::new(harness.state().store()).all()?;
    harness.state_mut().select_note(&notes[0].id);
    harness.run();
    assert!(shows(&harness, "1.5×"));
    harness.get_by_label("Play the voice recording").click();
    harness.run_steps(3);
    assert_eq!(devices.borrow().speeds.last(), Some(&1.5));
    Ok(())
}

#[test]
fn an_unknown_playback_speed_plays_at_normal_speed() -> TestResult {
    let devices: Shared = Rc::default();
    for kept in ["3", "fast", "1.25"] {
        let store = empty_store()?;
        store.set_local_setting("playbackSpeed", kept)?;
        let mut harness = window(store, &devices);
        record_note(&mut harness, &devices);
        assert!(shows(&harness, "1×"), "{kept}");
    }
    Ok(())
}

#[test]
fn the_playback_speed_is_written_the_german_way_in_german() -> TestResult {
    let store = empty_store()?;
    store.set_local_setting("playbackSpeed", "1.5")?;
    let devices: Shared = Rc::default();
    let mut harness = window_in(store, &devices, "de");
    devices.borrow_mut().mic = true;
    harness.get_by_label("Sprache aufnehmen").click();
    harness.run();
    harness.get_by_label("Beenden").click();
    harness.run();
    harness.get_by_label("1 Std.").click();
    harness.run();
    assert!(shows(&harness, "1,5×"));
    harness.get_by_label("1,5×").click();
    harness.run_steps(3);
    assert!(shows(&harness, "Wiedergabegeschwindigkeit"));
    assert!(shows(&harness, "×0,75"));
    Ok(())
}

#[test]
fn replace_and_append_take_a_photo_text_over_into_the_notes_text() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.state_mut().select_note(SHED);
    harness.run();
    // Only the Photo Text that says something; one button per line, in
    // the phone's order, above the file's own.
    assert_eq!(harness.query_all_by_label("Replace Note text").count(), 1);
    assert_eq!(harness.query_all_by_label("Append to Note text").count(), 1);
    let order = [
        "Spare key under the pot",
        "Replace Note text",
        "Append to Note text",
        "Remove generated text",
        "Remove photo, keep text",
    ];
    for pair in order.windows(2) {
        let above = rightmost(&harness, pair[0]);
        let below = rightmost(&harness, pair[1]);
        assert!(above.bottom() <= below.top(), "{pair:?}");
    }
    let text = |store: &Store| -> Result<String, Box<dyn std::error::Error>> {
        Ok(Notes::new(store).by_id(SHED)?.ok_or("Shed")?.text)
    };
    harness.get_by_label("Append to Note text").click();
    harness.run();
    assert_eq!(text(&store)?, "Shed\nSpare key under the pot");
    // The editor's field took the new text: a second Append builds on it.
    harness.get_by_label("Append to Note text").click();
    harness.run();
    assert_eq!(
        text(&store)?,
        "Shed\nSpare key under the pot\nSpare key under the pot"
    );
    harness.get_by_label("Replace Note text").click();
    harness.run();
    assert_eq!(text(&store)?, "Spare key under the pot");
    assert_eq!(harness.state().error(), None);
    // The Photo Text itself stays as it was.
    let shed = Notes::new(&store).by_id(SHED)?.ok_or("Shed")?;
    let photo = "0b6a0c7fce2f928971ccdff4431e34871dabb53e11455a29afc2fd5bed4d2e90";
    assert_eq!(
        shed.photo_texts.get(photo).map(String::as_str),
        Some("Spare key under the pot")
    );
    Ok(())
}

#[test]
fn replace_and_append_take_a_transcript_over_into_the_notes_text() -> TestResult {
    let store = empty_store()?;
    let notes = Notes::new(&store);
    let note = notes.capture("Bike", None, None)?;
    let hash = notes.add_recording(&note.id, &heard())?;
    notes.set_transcript(&note.id, &hash, "Lock code 1234")?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.state_mut().select_note(&note.id);
    harness.run();
    assert_eq!(harness.query_all_by_label("Replace Note text").count(), 1);
    assert_eq!(harness.query_all_by_label("Append to Note text").count(), 1);
    let text = |store: &Store| -> Result<String, Box<dyn std::error::Error>> {
        Ok(Notes::new(store).by_id(&note.id)?.ok_or("Bike")?.text)
    };
    harness.get_by_label("Append to Note text").click();
    harness.run();
    assert_eq!(text(&store)?, "Bike\nLock code 1234");
    // The editor's field took the new text: a second Append builds on it.
    harness.get_by_label("Append to Note text").click();
    harness.run();
    assert_eq!(text(&store)?, "Bike\nLock code 1234\nLock code 1234");
    harness.get_by_label("Replace Note text").click();
    harness.run();
    assert_eq!(text(&store)?, "Lock code 1234");
    assert_eq!(harness.state().error(), None);
    // A Note text edit as any other: a signed entry of this Device, which
    // a sync carries.
    let device = store.device_id()?;
    let edits: Vec<_> = store
        .all_entries()?
        .into_iter()
        .filter(|e| e.entity == note.id && e.field == "text")
        .collect();
    assert!(edits.len() > 1, "captured, then edited");
    assert!(edits.iter().all(|e| e.device == device && e.sig.is_some()));
    // The Transcript itself stays as it was.
    let bike = Notes::new(&store).by_id(&note.id)?.ok_or("Bike")?;
    assert_eq!(
        bike.transcripts.get(&hash).map(String::as_str),
        Some("Lock code 1234")
    );
    assert!(shows(&harness, "Lock code 1234"));
    Ok(())
}

#[test]
fn removing_a_photo_text_asks_once_and_keeps_the_photo() -> TestResult {
    let store = media_store()?;
    let devices: Shared = Rc::default();
    let mut harness = window(store.clone(), &devices);
    harness.state_mut().select_note(SHED);
    harness.run();
    let photo = "0b6a0c7fce2f928971ccdff4431e34871dabb53e11455a29afc2fd5bed4d2e90";
    harness.get_by_label("Remove generated text").click();
    harness.run();
    assert!(shows(&harness, "Remove this Photo Text?"));
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(!shows(&harness, "Remove this Photo Text?"));
    let shed = Notes::new(&store).by_id(SHED)?.ok_or("Shed")?;
    assert!(shed.removed_photo_texts.is_empty());
    harness.get_by_label("Remove generated text").click();
    harness.run();
    harness.get_by_label("Remove").click();
    harness.run();
    assert_eq!(harness.state().error(), None);
    let shed = Notes::new(&store).by_id(SHED)?.ok_or("Shed")?;
    assert_eq!(shed.removed_photo_texts, vec![photo.to_string()]);
    assert_eq!(shed.photos.len(), 3);
    assert!(store.photo(photo).is_some(), "the photo stays");
    // No text, nothing to take over, remove or remove the photo for.
    for gone in [
        "Spare key under the pot",
        "Replace Note text",
        "Remove generated text",
        "Remove photo, keep text",
    ] {
        assert!(!shows(&harness, gone), "{gone}");
    }
    type_into(&mut harness, "Search all Notes", "spare key");
    assert!(harness.state().shown_notes().is_empty());
    Ok(())
}

#[test]
fn a_removed_text_shows_only_again_and_comes_back_from_its_file() -> TestResult {
    let store = media_store()?;
    let bike = "note:0000000000000000000000000000000a";
    let photo = "557383277bf8ba9fdbe899439670c7a47a5e0abfb679f48025ce9d6cf1fd4872";
    let reader = Enricher::spawn(
        store.clone(),
        Kind::PhotoTexts,
        Box::new(FakeEngine {
            fail_over: usize::MAX,
        }),
        Box::new(|| {}),
    );
    let mut app = App::new(store.clone(), "en");
    app.set_enrichers(None, Some(reader), None);
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
    // The voice Note with Bike merged into it: Bike's photo lost its
    // Photo Text and shows only the button that reads it again; without
    // a transcriber nothing offers transcribing.
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000009");
    harness.run();
    assert_eq!(harness.query_all_by_label("Read the text again").count(), 1);
    assert!(!shows(&harness, "Transcribe again"));
    harness
        .get_by_label("Read the text again")
        .click_accesskit();
    harness.run();
    let notes = Notes::new(&store);
    let read = || {
        notes
            .by_id(bike)
            .ok()
            .flatten()
            .is_some_and(|n| n.photo_texts.contains_key(photo))
    };
    assert!(eventually(read));
    let section = notes.by_id(bike)?.ok_or("Bike")?;
    assert!(section.removed_photo_texts.is_empty());
    assert!(section.photo_texts[photo].starts_with("heard "));
    Ok(())
}

#[test]
fn transcribing_again_without_a_text_is_announced_at_the_top() -> TestResult {
    let store = media_store()?;
    let transcriber = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(FakeEngine { fail_over: 0 }),
        Box::new(|| {}),
    );
    let mut app = App::new(store.clone(), "en");
    app.set_enrichers(Some(transcriber), None, None);
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
    // Its own recording heard no words; Bike's has a Transcript.
    harness
        .state_mut()
        .select_note("note:00000000000000000000000000000009");
    harness.run();
    assert_eq!(harness.query_all_by_label("Transcribe again").count(), 2);
    assert!(shows(&harness, "Lock code 1234"));
    harness
        .query_all_by_label("Transcribe again")
        .next()
        .ok_or("again")?
        .click_accesskit();
    let announced = "A recording could not be transcribed.";
    let until = Instant::now() + StdDuration::from_secs(10);
    while !shows(&harness, announced) && Instant::now() < until {
        std::thread::sleep(StdDuration::from_millis(5));
        harness.run();
    }
    assert!(shows(&harness, announced));
    harness.get_by_label(announced).click();
    harness.run();
    assert!(shows(&harness, "Recording not transcribed"));
    // Cause and fix are one line.
    assert!(shows(
        &harness,
        "Whisper made no text from this recording. The file stays on the Note; try again later."
    ));
    // What was there before stays.
    let section = Notes::new(&store)
        .by_id("note:0000000000000000000000000000000a")?
        .ok_or("Bike")?;
    assert_eq!(
        section.transcripts.values().next().map(String::as_str),
        Some("Lock code 1234")
    );
    Ok(())
}
