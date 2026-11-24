//! The speech model on this computer, as the phone's `SpeechModel` keeps
//! it: checked against the pinned checksums once per start before
//! anything is transcribed, by the checksum recorded when it was kept
//! unless the file changed since, fetched on request, the chosen one from the
//! phone's mirror or either from a file of the user's own, and kept only
//! when it is one of the pinned ones. A model fetched takes the place of
//! the one in use only once it is checked, so the one before stays in
//! use until then, and stays when the fetch fails. Checking, downloading
//! and copying run on a thread of their own; the Settings show how far
//! they came. What it came to is kept as the Device's choice of speech
//! model (obf-1098), which a Backup carries.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use oldbrainfix_core::{SpeechModelChoice, SpeechModelSource};
use oldbrainfix_whisper::model::{ModelDir, Outcome, WhisperModel};
use parking_lot::Mutex;

/// Where the speech model is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Being checked against the pinned checksums.
    Checking,
    /// None yet: one can be downloaded or taken from a file.
    Missing,
    /// One being downloaded, or copied in when `downloading` is false,
    /// then checked; so many bytes so far, of so many when known. The
    /// model in use before, if any, stays in use meanwhile.
    Fetching {
        downloading: bool,
        received: u64,
        total: Option<u64>,
    },
    /// Checked and in use.
    Ready,
}

/// How a model is downloaded, so tests can hand in their own.
pub type Download = Arc<
    dyn Fn(&ModelDir, WhisperModel, &AtomicBool, &dyn Fn(u64, Option<u64>)) -> Outcome
        + Send
        + Sync,
>;

struct State {
    status: Status,
    /// How the last fetch ended, until the next one starts.
    outcome: Option<Outcome>,
    /// The checked model in use.
    current: Option<WhisperModel>,
    /// The model being downloaded.
    target: Option<WhisperModel>,
    /// How the model in use came, once a fetch put it in place; `None`
    /// for the one found at start.
    source: Option<SpeechModelSource>,
}

/// The speech model of this computer.
#[derive(Clone)]
pub struct SpeechModel {
    dir: ModelDir,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    download: Download,
    changed: Arc<dyn Fn() + Send + Sync>,
}

impl SpeechModel {
    /// The model in `dir`, checked at once in the background; `changed`
    /// runs on that thread whenever its status moves.
    pub fn new(dir: impl Into<PathBuf>, changed: Arc<dyn Fn() + Send + Sync>) -> SpeechModel {
        let download: Download =
            Arc::new(|dir, model, stop, progress| dir.download(model, stop, progress));
        Self::with_download(ModelDir::new(dir), download, changed)
    }

    /// As [`SpeechModel::new`], in `dir`, downloading with `download`.
    pub fn with_download(
        dir: ModelDir,
        download: Download,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> SpeechModel {
        let model = SpeechModel {
            dir,
            state: Arc::new(Mutex::new(State {
                status: Status::Checking,
                outcome: None,
                current: None,
                target: None,
                source: None,
            })),
            stop: Arc::new(AtomicBool::new(false)),
            download,
            changed,
        };
        let checked = model.clone();
        std::thread::spawn(move || {
            let found = checked.dir.check();
            checked.state.lock().current = found;
            checked.set(if found.is_some() {
                Status::Ready
            } else {
                Status::Missing
            });
        });
        model
    }

    pub fn status(&self) -> Status {
        self.state.lock().status
    }

    /// How the last fetch ended, until the next one starts.
    pub fn outcome(&self) -> Option<Outcome> {
        self.state.lock().outcome
    }

    /// The checked model in use, or `None` while there is none; it stays
    /// in use while another one is fetched.
    pub fn current(&self) -> Option<WhisperModel> {
        self.state.lock().current
    }

    /// The model chosen: the one being downloaded, else the one in use.
    pub fn chosen(&self) -> Option<WhisperModel> {
        let state = self.state.lock();
        state.target.or(state.current)
    }

    /// The choice of speech model to keep (obf-1098), given the one
    /// `kept`: `None` while the model is checked or fetched, so nothing is
    /// decided yet; else the model in use and how it came, `Some(None)`
    /// for none. The model found at start keeps how it came as `kept`
    /// says when that names it, and counts as downloaded otherwise.
    pub fn choice(&self, kept: Option<&SpeechModelChoice>) -> Option<Option<SpeechModelChoice>> {
        let state = self.state.lock();
        if !matches!(state.status, Status::Missing | Status::Ready) {
            return None;
        }
        let Some(current) = state.current else {
            return Some(None);
        };
        let name = model_name(current);
        let source = match (state.source, kept) {
            (Some(source), _) => source,
            (None, Some(kept)) if kept.name == name => kept.source,
            (None, _) => SpeechModelSource::Download,
        };
        Some(Some(SpeechModelChoice {
            name: name.to_owned(),
            source,
        }))
    }

    /// The model file in use, once there is one.
    pub fn model(&self) -> PathBuf {
        self.dir.model()
    }

    /// Downloads `chosen` from the phone's mirror and, once it is
    /// checked, uses it instead of the one in use.
    pub fn download(&self, chosen: WhisperModel) {
        let download = self.download.clone();
        self.fetch(Some(chosen), move |model| {
            let progress = |received, total| {
                model.set(Status::Fetching {
                    downloading: true,
                    received,
                    total,
                });
            };
            download(&model.dir, chosen, &model.stop, &progress)
        });
    }

    /// Takes the model from the file at `path`: either model, told apart
    /// by its checksum.
    pub fn take_file(&self, path: &Path) {
        let path = path.to_path_buf();
        self.fetch(None, move |model| model.dir.take_file(&path, &model.stop));
    }

    /// Stops the download or copy running.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Fetches a model with `fetch`, a download of `target` or, for none,
    /// a file; whatever the outcome, the model in use before stays until
    /// a new one took its place.
    fn fetch(
        &self,
        target: Option<WhisperModel>,
        fetch: impl FnOnce(&SpeechModel) -> Outcome + Send + 'static,
    ) {
        {
            let mut state = self.state.lock();
            if !matches!(state.status, Status::Missing | Status::Ready) {
                return;
            }
            state.outcome = None;
            state.target = target;
            state.status = Status::Fetching {
                downloading: target.is_some(),
                received: 0,
                total: None,
            };
        }
        self.stop.store(false, Ordering::Relaxed);
        (self.changed)();
        let model = self.clone();
        std::thread::spawn(move || {
            let outcome = fetch(&model);
            let ready = {
                let mut state = model.state.lock();
                state.outcome = Some(outcome);
                state.target = None;
                if let Outcome::Installed(installed) = outcome {
                    state.current = Some(installed);
                    state.source = Some(if target.is_some() {
                        SpeechModelSource::Download
                    } else {
                        SpeechModelSource::File
                    });
                }
                state.current.is_some()
            };
            model.set(if ready {
                Status::Ready
            } else {
                Status::Missing
            });
        });
    }

    fn set(&self, status: Status) {
        self.state.lock().status = status;
        (self.changed)();
    }
}

/// The name both apps give `model` (`SPEECH_MODEL_NAMES`).
pub fn model_name(model: WhisperModel) -> &'static str {
    match model {
        WhisperModel::Base => "base",
        WhisperModel::Small => "small",
    }
}

/// The model named `name` as [`model_name`] names it.
pub fn named(name: &str) -> Option<WhisperModel> {
    WhisperModel::ALL
        .into_iter()
        .find(|m| model_name(*m) == name)
}

#[cfg(test)]
mod tests;
