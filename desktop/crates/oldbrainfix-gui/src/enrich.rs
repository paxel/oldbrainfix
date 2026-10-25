//! Transcripts and Photo Texts made in the background (ADR 0007): one
//! thread each, one recording or photo at a time, while the window stays
//! responsive. Each new one is taken after its Note is saved, and those
//! that came from the phone before they were read there. A recording or
//! photo that cannot be read, or whose bytes are not on this computer
//! yet, is left alone until the next start; it is never lost, as the
//! recording or photo itself is always kept.
//!
//! The engines sit behind a trait: Whisper and ocrs in the app, fakes in
//! the tests.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use oldbrainfix_core::{Blob, Notes, Store};
use parking_lot::Mutex;

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

    /// Keeps `text` for `blob`, unless its Note let go of it meanwhile.
    fn keep(self, notes: &Notes, blob: &Blob, text: &str) -> oldbrainfix_core::Result<()> {
        let Some(note) = notes.by_id(&blob.note_id)? else {
            return Ok(());
        };
        match self {
            Kind::Transcripts if note.recordings.contains(&blob.hash) => {
                notes.set_transcript(&blob.note_id, &blob.hash, text)
            }
            Kind::PhotoTexts if note.photos.contains(&blob.hash) => {
                notes.set_photo_text(&blob.note_id, &blob.hash, text)
            }
            _ => Ok(()),
        }
    }
}

/// One background thread reading recordings or photos into text; its
/// clones all tell the same thread.
#[derive(Clone)]
pub struct Enricher {
    kick: Sender<()>,
    current: Arc<Mutex<Option<String>>>,
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
        let shared = current.clone();
        std::thread::spawn(move || run(&store, kind, engine, &kicked, &shared, &*changed));
        Enricher { kick, current }
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
    kicked: &Receiver<()>,
    current: &Mutex<Option<String>>,
    changed: &dyn Fn(),
) {
    let notes = Notes::new(store);
    let mut skipped: HashSet<String> = HashSet::new();
    // Gone when the window is.
    while kicked.recv().is_ok() {
        while kicked.try_recv().is_ok() {}
        loop {
            let next = kind
                .waiting(&notes)
                .ok()
                .and_then(|w| w.into_iter().find(|b| !skipped.contains(&b.hash)));
            let Some(blob) = next else {
                break;
            };
            let Some(bytes) = kind.bytes(&notes, &blob.hash) else {
                skipped.insert(blob.hash);
                continue;
            };
            *current.lock() = Some(blob.hash.clone());
            changed();
            let text = engine.read(bytes);
            *current.lock() = None;
            let kept = text.is_some_and(|text| kind.keep(&notes, &blob, &text).is_ok());
            if !kept {
                skipped.insert(blob.hash);
            }
            changed();
        }
        engine.idle();
    }
}

/// Whisper with the speech model at `model`, loaded on first use and let
/// go when idle. Device-only beyond a missing model.
pub struct WhisperEngine {
    model: PathBuf,
    whisper: Option<oldbrainfix_whisper::Whisper>,
}

impl WhisperEngine {
    pub fn new(model: PathBuf) -> WhisperEngine {
        WhisperEngine {
            model,
            whisper: None,
        }
    }
}

impl Engine for WhisperEngine {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        let samples = oldbrainfix_whisper::decode(bytes)?;
        if self.whisper.is_none() {
            self.whisper = oldbrainfix_whisper::Whisper::open(&self.model);
        }
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        self.whisper
            .as_mut()?
            .transcribe(&samples, oldbrainfix_whisper::threads_for(cores))
    }

    fn idle(&mut self) {
        self.whisper = None;
    }
}

/// The text-detection and text-recognition models the phone bundles
/// (android/app/src/main/assets/ocrs, CC BY-SA 4.0, see its NOTICE), the
/// same files, built in.
static DETECTION: &[u8] =
    include_bytes!("../../../../android/app/src/main/assets/ocrs/text-detection.rten");
static RECOGNITION: &[u8] =
    include_bytes!("../../../../android/app/src/main/assets/ocrs/text-recognition.rten");

/// ocrs with the phone's models, loaded on first use and let go when
/// idle, as android/ocr reads photos.
#[derive(Default)]
pub struct OcrEngine {
    engine: Option<ocrs::OcrEngine>,
}

impl OcrEngine {
    fn engine() -> Option<ocrs::OcrEngine> {
        let detection = rten::Model::load(DETECTION.to_vec()).ok()?;
        let recognition = rten::Model::load(RECOGNITION.to_vec()).ok()?;
        ocrs::OcrEngine::new(ocrs::OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })
        .ok()
    }
}

impl Engine for OcrEngine {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        let pixels = image::load_from_memory(&bytes).ok()?.into_rgba8();
        if self.engine.is_none() {
            self.engine = Self::engine();
        }
        let engine = self.engine.as_ref()?;
        let source = ocrs::ImageSource::from_bytes(pixels.as_raw(), pixels.dimensions()).ok()?;
        let input = engine.prepare_input(source).ok()?;
        let words = engine.detect_words(&input).ok()?;
        let lines = engine.find_text_lines(&input, &words);
        let texts = engine.recognize_text(&input, &lines).ok()?;
        Some(join_lines(
            texts.into_iter().flatten().map(|line| line.to_string()),
        ))
    }

    fn idle(&mut self) {
        self.engine = None;
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

    #[test]
    fn what_is_no_picture_is_not_read() {
        assert_eq!(OcrEngine::default().read(b"no picture".to_vec()), None);
    }

    #[test]
    fn without_its_model_whisper_reads_nothing() {
        let mut engine = WhisperEngine::new(PathBuf::from("/nonexistent/ggml-base.bin"));
        assert_eq!(engine.read(crate::media::wav(&[0.0; 1600])), None);
        assert_eq!(engine.read(b"no recording".to_vec()), None);
    }
}
