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
    let again = SpeechModel::with_download(dir, Arc::new(|_, _, _, _| Outcome::Failed), quiet());
    assert_eq!(settled(&again), Status::Ready);
    assert_eq!(again.current(), Some(WhisperModel::Base));
}
