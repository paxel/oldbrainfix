//! Transcripts and Photo Texts made in the background (ADR 0007): one
//! thread each, one recording or photo at a time, while the window stays
//! responsive. Each new one is taken after its Note is saved, and those
//! that came from the phone before they were read there. A recording or
//! photo that cannot be read, or whose bytes are not on this computer
//! yet, is left alone until the next start; it is never lost, as the
//! recording or photo itself is always kept. Asked to, one reads a
//! recording or photo again, ahead of the others and whether or not it
//! has a text, as "Transcribe again" and "Read the text again" do
//! (ADR 0025). What waits for the recordings of a Note, as a voice
//! capture going into a Shopping List does (#124), is told once each has
//! a Transcript or one could not be read.
//!
//! The engines sit behind a trait: Whisper and RustO in the app, fakes in
//! the tests. Whisper is led in with the Area and Topic names as the user
//! reads them, and for German speech with the English words German is
//! full of (`TranscriptPrompts`); what it only repeats of that lead-in
//! is taken out of the Transcript.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use ndarray::ArrayD;
use oldbrainfix_core::{Blob, Notes, Places, Store, TranscriptPrompts};
use oldbrainfix_whisper::Prompts;
use parking_lot::Mutex;
use rten_tensor::prelude::*;
use rusto::engine::{EngineError, InferenceSession};

use crate::l10n::L10n;
use crate::names;

/// Reads the text out of a recording or photo.
pub trait Engine: Send {
    /// The text in `bytes`, empty when there is none; `None` when they
    /// cannot be read.
    fn read(&mut self, bytes: Vec<u8>) -> Option<String>;

    /// Nothing more to read for now: whatever memory the engine holds
    /// can go.
    fn idle(&mut self) {}
}

/// What an [`Enricher`] works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The voice recordings, into Transcripts.
    Transcripts,
    /// The photos, into Photo Texts.
    PhotoTexts,
}

impl Kind {
    fn waiting(self, notes: &Notes) -> oldbrainfix_core::Result<Vec<Blob>> {
        match self {
            Kind::Transcripts => notes.untranscribed(),
            Kind::PhotoTexts => notes.unread_photos(),
        }
    }

    fn bytes(self, notes: &Notes, hash: &str) -> Option<Vec<u8>> {
        match self {
            Kind::Transcripts => notes.recording_bytes(hash),
            Kind::PhotoTexts => notes.photo_bytes(hash),
        }
    }

    /// Keeps `text` for `blob`, unless its Note let go of it meanwhile;
    /// whether it did.
    fn keep(self, notes: &Notes, blob: &Blob, text: &str) -> oldbrainfix_core::Result<bool> {
        let Some(note) = notes.by_id(&blob.note_id)? else {
            return Ok(false);
        };
        match self {
            Kind::Transcripts if note.recordings.contains(&blob.hash) => {
                notes.set_transcript(&blob.note_id, &blob.hash, text)?;
            }
            Kind::PhotoTexts if note.photos.contains(&blob.hash) => {
                notes.set_photo_text(&blob.note_id, &blob.hash, text)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// What an announcement at the top says, a sentence without values, and
/// the full text behind its click.
pub type Said = (&'static str, String);

/// What runs on the thread once every recording of a Note has a
/// Transcript, told `true`, or one could not be read or the Note is gone,
/// told `false`; what it has to announce, if anything.
pub type Then = Box<dyn FnOnce(bool) -> Option<Said> + Send>;

/// One background thread reading recordings or photos into text; its
/// clones all tell the same thread.
#[derive(Clone)]
pub struct Enricher {
    kick: Sender<()>,
    current: Arc<Mutex<Option<String>>>,
    asked: Arc<Asked>,
}

/// The recordings or photos asked to be read again, oldest ask first,
/// and those of them that brought no text; what waits for the
/// recordings of a Note, by its id, and what that had to announce.
#[derive(Default)]
struct Asked {
    waiting: Mutex<VecDeque<Blob>>,
    failed: Mutex<Vec<Blob>>,
    waiters: Mutex<Vec<(String, Then)>>,
    said: Mutex<Vec<Said>>,
}

impl Enricher {
    /// Starts the thread reading `kind` from `store` with `engine`;
    /// `changed` runs on it after every text kept and whenever it starts
    /// or stops reading one.
    pub fn spawn(
        store: Arc<Store>,
        kind: Kind,
        engine: Box<dyn Engine>,
        changed: Box<dyn Fn() + Send>,
    ) -> Enricher {
        let (kick, kicked) = std::sync::mpsc::channel();
        let current = Arc::new(Mutex::new(None));
        let asked = Arc::new(Asked::default());
        let (shared, asks) = (current.clone(), asked.clone());
        std::thread::spawn(move || run(&store, kind, engine, (&kicked, &asks), &shared, &*changed));
        Enricher {
            kick,
            current,
            asked,
        }
    }

    /// Reads the recording or photo `blob` again, after the one being
    /// read if any, and keeps the result as its text in place of the one
    /// before or of a removed one (ADR 0025). One that could not be read
    /// before is tried again; one that brings no text now is among
    /// [`Enricher::take_failed`].
    pub fn again(&self, blob: Blob) {
        self.asked.waiting.lock().push_back(blob);
        self.kick();
    }

    /// Whether the recording or photo `hash` is being read, or waits to
    /// be read again.
    pub fn busy_with(&self, hash: &str) -> bool {
        self.current.lock().as_deref() == Some(hash)
            || self.asked.waiting.lock().iter().any(|b| b.hash == hash)
    }

    /// Those asked to be read again that brought no text since the last
    /// call.
    pub fn take_failed(&self) -> Vec<Blob> {
        std::mem::take(&mut *self.asked.failed.lock())
    }

    /// Runs `then` once every recording of the Note `note_id` has a
    /// Transcript or one could not be read; after the next look for
    /// something to read at the latest.
    pub fn when_read(&self, note_id: &str, then: Then) {
        self.asked.waiters.lock().push((note_id.to_string(), then));
        self.kick();
    }

    /// What the ones [`Enricher::when_read`] ran had to announce since
    /// the last call.
    pub fn take_said(&self) -> Vec<Said> {
        std::mem::take(&mut *self.asked.said.lock())
    }

    /// Looks for something to read, unless it reads already; it looks
    /// once more at the end.
    pub fn kick(&self) {
        self.kick.send(()).ok();
    }

    /// The hash of the recording or photo being read, if any.
    pub fn current(&self) -> Option<String> {
        self.current.lock().clone()
    }
}

fn run(
    store: &Store,
    kind: Kind,
    mut engine: Box<dyn Engine>,
    (kicked, asked): (&Receiver<()>, &Asked),
    current: &Mutex<Option<String>>,
    changed: &dyn Fn(),
) {
    let notes = Notes::new(store);
    let mut skipped: HashSet<String> = HashSet::new();
    // Gone when the window is.
    while kicked.recv().is_ok() {
        while kicked.try_recv().is_ok() {}
        loop {
            // What was asked for comes first, skipped before or not.
            let again = asked.waiting.lock().pop_front();
            let is_again = again.is_some();
            let next = again.or_else(|| {
                kind.waiting(&notes)
                    .ok()
                    .and_then(|w| w.into_iter().find(|b| !skipped.contains(&b.hash)))
            });
            let Some(blob) = next else {
                break;
            };
            let Some(bytes) = kind.bytes(&notes, &blob.hash) else {
                skipped.insert(blob.hash.clone());
                if is_again {
                    asked.failed.lock().push(blob);
                    changed();
                }
                continue;
            };
            *current.lock() = Some(blob.hash.clone());
            changed();
            let text = engine.read(bytes);
            *current.lock() = None;
            let kept = text.is_some_and(|text| kind.keep(&notes, &blob, &text).unwrap_or(false));
            if kept {
                skipped.remove(&blob.hash);
            } else {
                skipped.insert(blob.hash.clone());
                if is_again {
                    asked.failed.lock().push(blob);
                }
            }
            changed();
            settle(&notes, asked, &skipped, changed);
        }
        settle(&notes, asked, &skipped, changed);
        engine.idle();
    }
}

/// Runs what waits for the recordings of a Note that are all read now,
/// or of which one could not be read (`skipped`), keeping what they
/// have to announce; `changed` once any ran.
fn settle(notes: &Notes, asked: &Asked, skipped: &HashSet<String>, changed: &dyn Fn()) {
    let mut ready: Vec<(Then, bool)> = Vec::new();
    {
        let mut waiters = asked.waiters.lock();
        let mut i = 0;
        while i < waiters.len() {
            match read_all(notes, &waiters[i].0, skipped) {
                Some(read) => ready.push((waiters.remove(i).1, read)),
                None => i += 1,
            }
        }
    }
    if ready.is_empty() {
        return;
    }
    for (then, read) in ready {
        if let Some(said) = then(read) {
            asked.said.lock().push(said);
        }
    }
    changed();
}

/// Whether every recording of the Note `note_id` has a Transcript, or a
/// removed one: `Some(true)` when so, `Some(false)` when one could not be
/// read or the Note is gone, `None` while one still waits.
fn read_all(notes: &Notes, note_id: &str, skipped: &HashSet<String>) -> Option<bool> {
    let Ok(Some(note)) = notes.by_id(note_id) else {
        return Some(false);
    };
    let mut waiting = note.recordings.iter().filter(|hash| {
        !note.transcripts.contains_key(*hash) && !note.removed_transcripts.contains(hash)
    });
    match waiting.clone().next() {
        None => Some(true),
        Some(_) if waiting.any(|hash| skipped.contains(hash)) => Some(false),
        Some(_) => None,
    }
}

/// What Whisper is led in with for the next recording.
pub type PromptsNow = Box<dyn Fn() -> TranscriptPrompts + Send>;

/// Whisper with the speech model at `model`, loaded on first use and let
/// go when idle, led in for each recording with what `prompts` gives
/// then; its Transcript is guarded against those prompts. Device-only
/// beyond a missing model.
pub struct WhisperEngine {
    model: PathBuf,
    prompts: PromptsNow,
    whisper: Option<oldbrainfix_whisper::Whisper>,
}

impl WhisperEngine {
    pub fn new(model: PathBuf, prompts: PromptsNow) -> WhisperEngine {
        WhisperEngine {
            model,
            prompts,
            whisper: None,
        }
    }
}

/// The prompts for the next recording in `store`: its Areas and Topics as
/// they are now, as the user reads them in `t`, with the app's words
/// Unsorted, Routine and Agenda; none when the store cannot be read.
pub fn transcript_prompts(store: &Store, t: &L10n) -> TranscriptPrompts {
    TranscriptPrompts::for_places(
        &Places::new(store),
        [t.unsorted(), t.routine(), t.agenda()],
        &|area| names::area_name(t, area),
        &|topic| names::topic_name(t, topic),
    )
    .unwrap_or_default()
}

impl Engine for WhisperEngine {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        let samples = oldbrainfix_whisper::decode(bytes)?;
        if self.whisper.is_none() {
            self.whisper = oldbrainfix_whisper::Whisper::open(&self.model);
        }
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        let prompts = (self.prompts)();
        let lead_in = Prompts {
            english: prompts.english.clone(),
            german: prompts.german.clone(),
        };
        let heard = self.whisper.as_mut()?.transcribe(
            &samples,
            oldbrainfix_whisper::threads_for(cores),
            &lead_in,
        )?;
        Some(prompts.guard(&heard))
    }

    fn idle(&mut self) {
        self.whisper = None;
    }
}

/// The PP-OCRv6 Tiny text-detection and text-recognition models and
/// their character list that the phone bundles
/// (android/app/src/main/assets/rusto, Apache-2.0, see its NOTICE), the
/// same files, built in.
static DETECTION: &[u8] = include_bytes!("../../../../android/app/src/main/assets/rusto/det.rten");
static RECOGNITION: &[u8] =
    include_bytes!("../../../../android/app/src/main/assets/rusto/rec.rten");
static CHARACTERS: &str = include_str!("../../../../android/app/src/main/assets/rusto/dict.txt");

/// RustO with the phone's models, loaded on first use and let go when
/// idle, as android/ocr reads photos.
#[derive(Default)]
pub struct OcrEngine {
    engine: Option<rusto::RustO>,
}

impl OcrEngine {
    fn engine() -> Option<rusto::RustO> {
        let detection = ModelSession::new(DETECTION.to_vec(), None)?;
        let characters = CHARACTERS.lines().map(String::from).collect();
        let recognition = ModelSession::new(RECOGNITION.to_vec(), Some(characters))?;
        // The PP-OCRv6 settings; the paths are not read, the sessions
        // hold the models.
        let config = rusto::InitializeConfig::ppv6("det.rten", "rec.rten", "dict.txt");
        rusto::RustO::with_custom_engines(config, detection, recognition).ok()
    }
}

/// How RustO reads, as on the phone (ADR 0007): the text laid out as on
/// the picture, and a box kept from a probability of 0.3 instead of 0.6,
/// so the lines of a curved page are not dropped. The threshold is the
/// top-level `postprocess`; RustO 0.3.2 ignores `detection.postprocess`
/// (https://github.com/byrizki/rusto-rs/issues/21).
pub fn ocr_options() -> rusto::OcrRunOptions {
    rusto::OcrRunOptions {
        output: rusto::OutputGranularity::Spatial,
        postprocess: Some(rusto::PostprocessRunOptions {
            box_threshold: Some(0.3),
            ..Default::default()
        }),
        ..Default::default()
    }
}

impl Engine for OcrEngine {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        let pixels = image::load_from_memory(&bytes).ok()?.into_rgb8();
        if self.engine.is_none() {
            self.engine = Self::engine();
        }
        let engine = self.engine.as_mut()?;
        let (width, height) = pixels.dimensions();
        let result = engine
            .detect_text_rgb(
                pixels.as_raw(),
                width,
                height,
                width as usize * 3,
                &ocr_options(),
            )
            .ok()?;
        Some(match result {
            rusto::DetectTextResult::Spatial(text) => join_lines(text.lines().map(String::from)),
            rusto::DetectTextResult::Structured(lines) => {
                join_lines(lines.into_iter().map(|line| line.text))
            }
        })
    }

    fn idle(&mut self) {
        self.engine = None;
    }
}

/// One model run as RustO runs its own models from a file, but loaded
/// from the bytes built in, as on the phone: the first input in, the
/// first output out. The recognition model also holds its characters.
struct ModelSession {
    model: rten::Model,
    characters: Option<Vec<String>>,
}

impl ModelSession {
    fn new(bytes: Vec<u8>, characters: Option<Vec<String>>) -> Option<ModelSession> {
        let model = rten::Model::load(bytes).ok()?;
        Some(ModelSession { model, characters })
    }
}

impl InferenceSession for ModelSession {
    fn run(&mut self, input: ArrayD<f32>) -> Result<ArrayD<f32>, EngineError> {
        let shape = input.shape().to_vec();
        let tensor = rten_tensor::Tensor::<f32>::from_data(
            &shape,
            input.iter().copied().collect::<Vec<_>>(),
        );
        let input_id = *self
            .model
            .input_ids()
            .first()
            .ok_or_else(|| EngineError::OutputError("Model has no inputs".into()))?;
        let outputs = self.model.run(
            vec![(input_id, tensor.view().into())],
            self.model.output_ids(),
            None,
        )?;
        match outputs.first() {
            Some(rten::Value::FloatTensor(output)) => Ok(ArrayD::from_shape_vec(
                output.shape().to_vec(),
                output.iter().copied().collect(),
            )?),
            Some(_) => Err(EngineError::OutputError(
                "Unsupported output tensor type".into(),
            )),
            None => Err(EngineError::OutputError("Model produced no outputs".into())),
        }
    }

    fn run_all(
        &mut self,
        _input: ArrayD<f32>,
    ) -> Result<HashMap<String, ArrayD<f32>>, EngineError> {
        Err(EngineError::Engine("not used for text".into()))
    }

    fn run_with_inputs(
        &mut self,
        _inputs: HashMap<String, ArrayD<f32>>,
    ) -> Result<HashMap<String, ArrayD<f32>>, EngineError> {
        Err(EngineError::Engine("not used for text".into()))
    }

    fn get_character_list(&self, key: &str) -> Option<Vec<String>> {
        (key == "character")
            .then(|| self.characters.clone())
            .flatten()
    }

    fn have_key(&self, key: &str) -> bool {
        key == "character" && self.characters.is_some()
    }
}

/// The recognised `lines` as one text: each trimmed, blank ones left
/// out, one per line, as on the phone.
pub fn join_lines(lines: impl IntoIterator<Item = String>) -> String {
    lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_trimmed_and_blank_ones_left_out() {
        let lines = ["  Milk ", "", "   ", "eggs\t"].map(String::from);
        assert_eq!(join_lines(lines), "Milk\neggs");
        assert_eq!(join_lines(Vec::<String>::new()), "");
    }

    /// testdata/rust-book.jpg of android/ocr, copied there unchanged
    /// from the examples of the ocrs 0.13.1 crate.
    #[test]
    fn the_phones_models_read_a_photographed_page() {
        let photo = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../android/ocr/testdata/rust-book.jpg"
        ))
        .unwrap_or_default();
        let text = OcrEngine::default().read(photo).unwrap_or_default();
        assert!(text.contains("THE RUST\nPROGRAMMING\nLANGUAGE"), "{text}");
    }

    /// A photo of android/ocr's testdata, read as the desktop reads it.
    fn read_testdata(name: &str) -> String {
        let path = format!(
            "{}/../../../android/ocr/testdata/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let photo = std::fs::read(path).unwrap_or_default();
        OcrEngine::default().read(photo).unwrap_or_default()
    }

    #[test]
    fn german_letters_and_the_euro_sign_are_read() {
        let text = read_testdata("german-receipt.jpg");
        assert!(text.contains("Bäckerei Müller GmbH"), "{text}");
        assert!(text.contains("Hauptstraße 7, 80331 München"), "{text}");
        assert!(
            text.lines()
                .any(|line| line.starts_with("Käsekuchen") && line.ends_with("3,90 €")),
            "{text}"
        );
    }

    /// With RustO's own box threshold of 0.6 seven of the page's twelve
    /// lines are gone.
    #[test]
    fn every_line_of_a_curved_page_is_read() {
        let text = read_testdata("curved-page.jpg");
        assert_eq!(text.lines().count(), 12, "{text}");
        assert!(text.contains("once or twice"), "{text}");
        assert!(text.contains("queue full"), "{text}");
    }

    /// The box threshold must sit at the top level: RustO 0.3.2 ignores
    /// `detection.postprocess`.
    #[test]
    fn spatial_text_with_the_lower_box_threshold() {
        let options = ocr_options();
        assert_eq!(options.output, rusto::OutputGranularity::Spatial);
        assert!(options.detection.is_none());
        let box_threshold = options.postprocess.and_then(|p| p.box_threshold);
        assert_eq!(box_threshold, Some(0.3));
    }

    #[test]
    fn what_is_no_picture_is_not_read() {
        assert_eq!(OcrEngine::default().read(b"no picture".to_vec()), None);
    }

    /// The stores of the prompt fixture the Dart core wrote, seen with
    /// the app's own strings, give the prompts it wrote: the Area and
    /// Topic names as the user reads them, translated Starter Set places
    /// in German among them.
    #[test]
    fn the_prompts_name_the_places_as_the_app_shows_them() -> oldbrainfix_core::Result<()> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/transcript_prompts.json"
        );
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let fixture: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let views = fixture["views"].as_array().cloned().unwrap_or_default();
        assert_eq!(views.len(), 6);
        for view in views {
            let name = view["store"].as_str().unwrap_or_default();
            let t = L10n::new(view["locale"].as_str().unwrap_or_default());
            let store = Store::in_memory(Box::new(chrono::Utc::now), || {
                Ok(oldbrainfix_core::SigningKey::from_seed([1; 32]))
            })?;
            for entry in fixture["stores"][name].as_array().into_iter().flatten() {
                if let Some(entry) = oldbrainfix_core::WireEntry::from_value(entry) {
                    store.insert(&entry)?;
                }
            }
            let prompts = transcript_prompts(&store, &t);
            assert_eq!(prompts.english, view["english"], "{name}");
            assert_eq!(prompts.german, view["german"], "{name}");
        }
        let de = L10n::new("de");
        let store = Store::in_memory(Box::new(chrono::Utc::now), || {
            Ok(oldbrainfix_core::SigningKey::from_seed([1; 32]))
        })?;
        let places = Places::new(&store);
        places.plant_starter_set(&["household", "kitchen"])?;
        places.add_area("Fahrrad")?;
        let prompts = transcript_prompts(&store, &de);
        assert_eq!(prompts.english, "Fahrrad, Haushalt, Küche.");
        // What the Transcript is guarded against.
        assert_eq!(prompts.names, ["Fahrrad", "Haushalt", "Küche"]);
        assert_eq!(prompts.terms, ["Unsortiert", "Routine", "Agenda"]);
        assert_eq!(prompts.guard("Fahrrad, Haushalt, Küche."), "");
        assert!(
            prompts
                .german
                .ends_with("Unsortiert, Routine, Agenda. Fahrrad, Haushalt, Küche.")
        );
        Ok(())
    }

    #[test]
    fn without_its_model_whisper_reads_nothing() {
        let mut engine = WhisperEngine::new(
            PathBuf::from("/nonexistent/ggml-base.bin"),
            Box::new(TranscriptPrompts::default),
        );
        assert_eq!(engine.read(crate::media::wav(&[0.0; 1600])), None);
        assert_eq!(engine.read(b"no recording".to_vec()), None);
    }
}
