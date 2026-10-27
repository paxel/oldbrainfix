//! The speech model on this computer, as the phone's `SpeechModel` keeps
//! it: checked against the pinned checksums once per start before
//! anything is transcribed, by the checksum recorded when it was kept
//! unless the file changed since, fetched on request, the chosen one from the
//! phone's mirror or either from a file of the user's own, and kept only
//! when it is one of the pinned ones. A model fetched takes the place of
//! the one in use only once it is checked, so the one before stays in
//! use until then, and stays when the fetch fails. Checking, downloading
//! and copying run on a thread of their own; the Settings show how far
//! they came.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn quiet() -> Arc<dyn Fn() + Send + Sync> {
        Arc::new(|| {})
    }

    /// Waits for `model` to settle into a status that is not `busy`.
    fn settled(model: &SpeechModel) -> Status {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let status = model.status();
            let busy = matches!(status, Status::Checking | Status::Fetching { .. });
            if !busy || Instant::now() > until {
                return status;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_wrong_file_is_refused_and_a_download_that_breaks_keeps_nothing() {
        let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let broken: Download = Arc::new(|_, _, _, progress| {
            progress(5, Some(10));
            Outcome::Failed
        });
        let dir = ModelDir::new(tmp.path().join("whisper"));
        let model = SpeechModel::with_download(dir, broken, quiet());
        assert_eq!(settled(&model), Status::Missing);
        let file = tmp.path().join("not-whisper.bin");
        assert!(std::fs::write(&file, b"no model").is_ok());
        model.take_file(&file);
        assert_eq!(settled(&model), Status::Missing);
        assert_eq!(model.outcome(), Some(Outcome::Mismatch));
        model.download(WhisperModel::Small);
        assert_eq!(settled(&model), Status::Missing);
        assert_eq!(model.outcome(), Some(Outcome::Failed));
        assert_eq!(model.chosen(), None);
        assert!(!model.model().exists());
    }

    #[test]
    fn a_fetch_shows_how_far_it_came_and_can_be_cancelled() {
        let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let slow: Download = Arc::new(|_, _, stop, progress| {
            progress(1, Some(148));
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Outcome::Cancelled
        });
        let dir = ModelDir::new(tmp.path().join("whisper"));
        let model = SpeechModel::with_download(dir, slow, quiet());
        assert_eq!(settled(&model), Status::Missing);
        model.download(WhisperModel::Base);
        let until = Instant::now() + Duration::from_secs(10);
        while model.status()
            != (Status::Fetching {
                downloading: true,
                received: 1,
                total: Some(148),
            })
            && Instant::now() < until
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        model.cancel();
        assert_eq!(settled(&model), Status::Missing);
        assert_eq!(model.outcome(), Some(Outcome::Cancelled));
    }

    /// The SHA-256 of `bytes`, as lowercase hex.
    fn sha(bytes: &[u8]) -> String {
        let hash = oldbrainfix_whisper::model::copy_hashing(
            bytes,
            std::io::sink(),
            &AtomicBool::new(false),
            |_| {},
        );
        hash.ok().flatten().unwrap_or_default()
    }

    #[test]
    fn a_new_model_replaces_the_one_in_use_and_a_failed_one_keeps_it() {
        let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let dir = ModelDir::pinned_to(tmp.path().join("whisper"), &sha(b"base"), &sha(b"small"));
        // The server hands out "small" for the small model, the base
        // model's bytes for the base one, and breaks off at first.
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let served = calls.clone();
        let fake: Download = Arc::new(move |dir, model, stop, _| {
            if served.fetch_add(1, Ordering::SeqCst) == 1 {
                return Outcome::Failed;
            }
            let bytes: &[u8] = match model {
                WhisperModel::Base => b"base",
                WhisperModel::Small => b"small",
            };
            dir.take_in(bytes, None, Some(model), stop, |_, _| {})
        });
        let model = SpeechModel::with_download(dir.clone(), fake, quiet());
        assert_eq!(settled(&model), Status::Missing);
        model.download(WhisperModel::Base);
        assert_eq!(settled(&model), Status::Ready);
        assert_eq!(model.current(), Some(WhisperModel::Base));
        model.download(WhisperModel::Small);
        assert_eq!(settled(&model), Status::Ready);
        assert_eq!(model.outcome(), Some(Outcome::Failed));
        assert_eq!(
            model.current(),
            Some(WhisperModel::Base),
            "the old one stays"
        );
        assert_eq!(model.chosen(), Some(WhisperModel::Base));
        model.download(WhisperModel::Small);
        assert_eq!(settled(&model), Status::Ready);
        assert_eq!(
            model.outcome(),
            Some(Outcome::Installed(WhisperModel::Small))
        );
        assert_eq!(model.chosen(), Some(WhisperModel::Small));
        assert_eq!(std::fs::read(model.model()).unwrap_or_default(), b"small");
        // A file of the user's own may be either; a wrong one keeps the
        // model in use.
        let file = tmp.path().join("mine.bin");
        assert!(std::fs::write(&file, b"no model").is_ok());
        model.take_file(&file);
        assert_eq!(settled(&model), Status::Ready);
        assert_eq!(model.outcome(), Some(Outcome::Mismatch));
        assert_eq!(model.current(), Some(WhisperModel::Small));
        assert!(std::fs::write(&file, b"base").is_ok());
        model.take_file(&file);
        assert_eq!(settled(&model), Status::Ready);
        assert_eq!(model.current(), Some(WhisperModel::Base));
        // The next start finds the one kept.
        let again =
            SpeechModel::with_download(dir, Arc::new(|_, _, _, _| Outcome::Failed), quiet());
        assert_eq!(settled(&again), Status::Ready);
        assert_eq!(again.current(), Some(WhisperModel::Base));
    }
}
