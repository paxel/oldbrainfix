//! The speech model on this computer, as the phone's `SpeechModel` keeps
//! it: checked against the pinned checksum once per start before anything
//! is transcribed, fetched on request, from the phone's mirror or a file
//! of the user's own, and kept only when it is the pinned one. Checking,
//! downloading and copying run on a thread of their own; the Settings
//! show how far they came.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use oldbrainfix_whisper::model::{ModelDir, Outcome};
use parking_lot::Mutex;

/// Where the speech model is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Being checked against the pinned checksum.
    Checking,
    /// None yet: it can be downloaded or taken from a file.
    Missing,
    /// Being downloaded, or copied in when `downloading` is false, then
    /// checked; so many bytes so far, of so many when known.
    Fetching {
        downloading: bool,
        received: u64,
        total: Option<u64>,
    },
    /// Checked and in use.
    Ready,
}

/// How the model is fetched, so tests can hand in their own.
pub type Download =
    Arc<dyn Fn(&ModelDir, &AtomicBool, &dyn Fn(u64, Option<u64>)) -> Outcome + Send + Sync>;

struct State {
    status: Status,
    /// How the last fetch ended, until the next one starts.
    outcome: Option<Outcome>,
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
        let download: Download = Arc::new(|dir, stop, progress| dir.download(stop, progress));
        Self::with_download(dir, download, changed)
    }

    /// As [`SpeechModel::new`], fetching with `download`.
    pub fn with_download(
        dir: impl Into<PathBuf>,
        download: Download,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> SpeechModel {
        let model = SpeechModel {
            dir: ModelDir::new(dir),
            state: Arc::new(Mutex::new(State {
                status: Status::Checking,
                outcome: None,
            })),
            stop: Arc::new(AtomicBool::new(false)),
            download,
            changed,
        };
        let checked = model.clone();
        std::thread::spawn(move || {
            let ready = checked.dir.check();
            checked.set(if ready {
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

    /// The model file, once it is ready.
    pub fn model(&self) -> PathBuf {
        self.dir.model()
    }

    /// Downloads the model from the phone's mirror.
    pub fn download(&self) {
        let download = self.download.clone();
        self.fetch(true, move |model| {
            let progress = |received, total| {
                model.set(Status::Fetching {
                    downloading: true,
                    received,
                    total,
                });
            };
            download(&model.dir, &model.stop, &progress)
        });
    }

    /// Takes the model from the file at `path`.
    pub fn take_file(&self, path: &Path) {
        let path = path.to_path_buf();
        self.fetch(false, move |model| model.dir.take_file(&path, &model.stop));
    }

    /// Stops the download or copy running.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn fetch(
        &self,
        downloading: bool,
        fetch: impl FnOnce(&SpeechModel) -> Outcome + Send + 'static,
    ) {
        {
            let mut state = self.state.lock();
            if state.status != Status::Missing {
                return;
            }
            state.outcome = None;
            state.status = Status::Fetching {
                downloading,
                received: 0,
                total: None,
            };
        }
        self.stop.store(false, Ordering::Relaxed);
        (self.changed)();
        let model = self.clone();
        std::thread::spawn(move || {
            let outcome = fetch(&model);
            model.state.lock().outcome = Some(outcome);
            model.set(if outcome == Outcome::Installed {
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
        let broken: Download = Arc::new(|_, _, progress| {
            progress(5, Some(10));
            Outcome::Failed
        });
        let model = SpeechModel::with_download(tmp.path().join("whisper"), broken, quiet());
        assert_eq!(settled(&model), Status::Missing);
        let file = tmp.path().join("not-whisper.bin");
        assert!(std::fs::write(&file, b"no model").is_ok());
        model.take_file(&file);
        assert_eq!(settled(&model), Status::Missing);
        assert_eq!(model.outcome(), Some(Outcome::Mismatch));
        model.download();
        assert_eq!(settled(&model), Status::Missing);
        assert_eq!(model.outcome(), Some(Outcome::Failed));
        assert!(!model.model().exists());
    }

    #[test]
    fn a_fetch_shows_how_far_it_came_and_can_be_cancelled() {
        let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let slow: Download = Arc::new(|_, stop, progress| {
            progress(1, Some(148));
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Outcome::Cancelled
        });
        let model = SpeechModel::with_download(tmp.path().join("whisper"), slow, quiet());
        assert_eq!(settled(&model), Status::Missing);
        model.download();
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
}
