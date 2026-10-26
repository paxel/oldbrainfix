//! The speech models Transcripts can be made with (ADR 0007), the
//! phone's: whisper.cpp's multilingual `base` and `small` models, one
//! fetched on request from Old Brain Fix's own GitHub release, never from
//! anywhere else, or taken from a file of the user's own; kept only when
//! its SHA-256 is one pinned here, the phone's (`WhisperModel` in
//! speech_platform.dart). Only the chosen one is kept.
//!
//! A model on its way in is written beside the one in use and only takes
//! its place once it is checked, so a broken download never leaves a
//! half model behind, and the one before stays in use until then.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// One of the speech models, as on the phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhisperModel {
    /// `base`: the default.
    Base,
    /// `small`: better with English words in German speech, but slower.
    Small,
}

impl WhisperModel {
    pub const ALL: [WhisperModel; 2] = [WhisperModel::Base, WhisperModel::Small];

    /// Its file name, on the release and in whisper.cpp's model list.
    pub fn file(self) -> &'static str {
        match self {
            WhisperModel::Base => "ggml-base.bin",
            WhisperModel::Small => "ggml-small.bin",
        }
    }

    /// The SHA-256 a model must have to be kept as this one.
    pub fn sha256(self) -> &'static str {
        match self {
            // SHA-1 465707469ff3a37a2b9b8d8f89f2f99de7299dac in
            // whisper.cpp's model list; 147,951,465 bytes.
            WhisperModel::Base => {
                "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"
            }
            // As on Hugging Face (ggerganov/whisper.cpp); 487,601,967 bytes.
            WhisperModel::Small => {
                "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b"
            }
        }
    }

    /// Its size in megabytes, as told before the download.
    pub fn megabytes(self) -> i64 {
        match self {
            WhisperModel::Base => 148,
            WhisperModel::Small => 488,
        }
    }

    /// The release asset; GitHub may redirect it to its download host.
    pub fn url(self) -> String {
        format!(
            "https://github.com/paxel/oldbrainfix/releases/download/whisper-model-v1/{}",
            self.file()
        )
    }
}

/// The model in use, whichever it is, under the name of the first one
/// there was, as on the phone.
pub const FILE_NAME: &str = "ggml-base.bin";

/// How fetching a model ended, as on the phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Checked and in use, in place of the one before.
    Installed(WhisperModel),
    /// Stopped by the user.
    Cancelled,
    /// The download or copy broke off.
    Failed,
    /// Not the expected model: not kept.
    Mismatch,
}

/// Where the model lives: one directory of this computer's data.
#[derive(Debug, Clone)]
pub struct ModelDir {
    dir: PathBuf,
    /// Each model with the SHA-256 it must have.
    pins: [(WhisperModel, String); 2],
}

impl ModelDir {
    pub fn new(dir: impl Into<PathBuf>) -> ModelDir {
        ModelDir {
            dir: dir.into(),
            pins: WhisperModel::ALL.map(|m| (m, m.sha256().to_string())),
        }
    }

    /// As [`ModelDir::new`], the models pinned to `base` and `small`
    /// instead: for tests, which cannot hold the real models.
    pub fn pinned_to(dir: impl Into<PathBuf>, base: &str, small: &str) -> ModelDir {
        ModelDir {
            dir: dir.into(),
            pins: [
                (WhisperModel::Base, base.to_string()),
                (WhisperModel::Small, small.to_string()),
            ],
        }
    }

    /// The model whose SHA-256 is `hash`, if it is one.
    pub fn model_by_hash(&self, hash: &str) -> Option<WhisperModel> {
        self.pins
            .iter()
            .find(|(_, pinned)| pinned == hash)
            .map(|(model, _)| *model)
    }

    /// The model in use, once it was checked and kept.
    pub fn model(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// A model on its way in, not yet checked.
    fn incoming(&self) -> PathBuf {
        self.dir.join(format!("{FILE_NAME}.part"))
    }

    /// Which pinned model is here, if any. A model that is none of them
    /// is dropped, and so is a fetch cut off when the app was last
    /// closed. Reads the whole model: run it off the window's thread.
    pub fn check(&self) -> Option<WhisperModel> {
        let _ = std::fs::remove_file(self.incoming());
        let model = self.model();
        let file = File::open(&model).ok()?;
        let hash = copy_hashing(file, io::sink(), &AtomicBool::new(false), |_| {});
        let found = hash.ok().flatten().and_then(|h| self.model_by_hash(&h));
        if found.is_none() {
            let _ = std::fs::remove_file(model);
        }
        found
    }

    /// Downloads `model` from its release asset, over HTTPS only, and
    /// keeps it in place of the one in use when it is that model.
    /// `progress` hears the bytes so far and the whole size when the
    /// server tells it; `stop` cancels.
    pub fn download(
        &self,
        model: WhisperModel,
        stop: &AtomicBool,
        progress: impl FnMut(u64, Option<u64>),
    ) -> Outcome {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .build()
            .into();
        let response = match agent.get(&model.url()).call() {
            Ok(response) => response,
            Err(_) => return Outcome::Failed,
        };
        let body = response.into_body();
        let total = body.content_length();
        self.take_in(body.into_reader(), total, Some(model), stop, progress)
    }

    /// Takes the model from the file at `path` the user picked, and keeps
    /// it in place of the one in use when it is either pinned model.
    pub fn take_file(&self, path: &Path, stop: &AtomicBool) -> Outcome {
        match File::open(path) {
            Ok(file) => {
                let total = file.metadata().ok().map(|m| m.len());
                self.take_in(file, total, None, stop, |_, _| {})
            }
            Err(_) => Outcome::Failed,
        }
    }

    /// Copies `input` to the side, checks it, and keeps it as the model
    /// in use when it is `expected`, or either pinned model when that is
    /// `None`; anything else is dropped, and the model in use stays.
    pub fn take_in(
        &self,
        input: impl Read,
        total: Option<u64>,
        expected: Option<WhisperModel>,
        stop: &AtomicBool,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Outcome {
        let incoming = self.incoming();
        let outcome = (|| {
            std::fs::create_dir_all(&self.dir)?;
            let mut out = File::create(&incoming)?;
            let hash = copy_hashing(input, &mut out, stop, |n| progress(n, total))?;
            out.sync_all()?;
            drop(out);
            let Some(hash) = hash else {
                return Ok(Outcome::Cancelled);
            };
            Ok::<_, io::Error>(match self.model_by_hash(&hash) {
                Some(model) if expected.is_none_or(|e| e == model) => {
                    std::fs::rename(&incoming, self.model())?;
                    Outcome::Installed(model)
                }
                _ => Outcome::Mismatch,
            })
        })()
        .unwrap_or(Outcome::Failed);
        if !matches!(outcome, Outcome::Installed(_)) {
            let _ = std::fs::remove_file(&incoming);
        }
        outcome
    }

    /// Drops the model in use.
    pub fn remove(&self) {
        let _ = std::fs::remove_file(self.model());
    }
}

/// Copies `input` to `output` and answers the SHA-256 of what was copied,
/// as lowercase hex; `None` when `stop` was set on the way. `copied`
/// hears the bytes so far, about every megabyte and at the end.
pub fn copy_hashing(
    mut input: impl Read,
    mut output: impl Write,
    stop: &AtomicBool,
    mut copied: impl FnMut(u64),
) -> io::Result<Option<String>> {
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    let mut told = 0u64;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let read = match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        digest.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
        total += read as u64;
        if total - told >= 1 << 20 {
            told = total;
            copied(total);
        }
    }
    copied(total);
    Ok(Some(hex::encode(digest.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that breaks off after `left` bytes.
    struct Broken {
        left: usize,
    }

    impl Read for Broken {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Err(io::Error::other("connection reset"));
            }
            let n = buf.len().min(self.left);
            buf[..n].fill(7);
            self.left -= n;
            Ok(n)
        }
    }

    fn dir() -> (tempfile::TempDir, ModelDir) {
        let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let dir = ModelDir::new(tmp.path().join("whisper"));
        (tmp, dir)
    }

    #[test]
    fn copying_hashes_what_it_copies() -> io::Result<()> {
        let mut out = Vec::new();
        let mut heard = Vec::new();
        let hash = copy_hashing(&b"abc"[..], &mut out, &AtomicBool::new(false), |n| {
            heard.push(n)
        })?;
        assert_eq!(
            hash.as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        assert_eq!(out, b"abc");
        assert_eq!(heard, vec![3]);
        let stopped = copy_hashing(&b"abc"[..], io::sink(), &AtomicBool::new(true), |_| {})?;
        assert_eq!(stopped, None);
        Ok(())
    }

    #[test]
    fn a_file_that_is_not_the_pinned_model_is_not_kept() {
        let (_tmp, dir) = dir();
        let outcome = dir.take_in(
            &b"not whisper"[..],
            None,
            None,
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(outcome, Outcome::Mismatch);
        assert!(!dir.model().exists());
        assert!(!dir.incoming().exists());
        assert_eq!(dir.check(), None);
    }

    #[test]
    fn a_broken_or_cancelled_fetch_leaves_nothing_behind() {
        let (_tmp, dir) = dir();
        let broken = dir.take_in(
            Broken { left: 3 << 20 },
            None,
            Some(WhisperModel::Small),
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(broken, Outcome::Failed);
        let cancelled = dir.take_in(&b"x"[..], Some(1), None, &AtomicBool::new(true), |_, _| {});
        assert_eq!(cancelled, Outcome::Cancelled);
        assert!(!dir.model().exists());
        assert!(!dir.incoming().exists());
        let missing = dir.take_file(
            Path::new("/nonexistent/ggml-base.bin"),
            &AtomicBool::new(false),
        );
        assert_eq!(missing, Outcome::Failed);
    }

    #[test]
    fn progress_tells_the_size_the_server_said() {
        let (_tmp, dir) = dir();
        let mut heard = Vec::new();
        let bytes = vec![1u8; (1 << 20) + 5];
        dir.take_in(
            &bytes[..],
            Some(bytes.len() as u64),
            None,
            &AtomicBool::new(false),
            |n, t| heard.push((n, t)),
        );
        let total = Some(bytes.len() as u64);
        assert_eq!(heard, vec![(1 << 20, total), ((1 << 20) + 5, total)]);
    }

    #[test]
    fn a_wrong_model_in_place_is_dropped_on_the_check() -> io::Result<()> {
        let (_tmp, dir) = dir();
        std::fs::create_dir_all(&dir.dir)?;
        std::fs::write(dir.model(), b"tampered")?;
        std::fs::write(dir.incoming(), b"cut off")?;
        assert_eq!(dir.check(), None);
        assert!(!dir.model().exists());
        assert!(!dir.incoming().exists());
        Ok(())
    }

    /// The SHA-256 of `bytes`, as lowercase hex.
    fn sha(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    /// A model directory where the bytes "base" and "small" are the
    /// pinned models.
    fn pinned() -> (tempfile::TempDir, ModelDir) {
        let tmp = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let dir = ModelDir::pinned_to(tmp.path().join("whisper"), &sha(b"base"), &sha(b"small"));
        (tmp, dir)
    }

    fn take(dir: &ModelDir, bytes: &[u8], expected: Option<WhisperModel>) -> Outcome {
        dir.take_in(bytes, None, expected, &AtomicBool::new(false), |_, _| {})
    }

    #[test]
    fn the_models_are_the_phones() {
        assert_eq!(
            WhisperModel::Small.url(),
            "https://github.com/paxel/oldbrainfix/releases/download/whisper-model-v1/ggml-small.bin"
        );
        assert_eq!(
            WhisperModel::Base.url(),
            "https://github.com/paxel/oldbrainfix/releases/download/whisper-model-v1/ggml-base.bin"
        );
        assert_eq!(
            (
                WhisperModel::Base.megabytes(),
                WhisperModel::Small.megabytes()
            ),
            (148, 488)
        );
        let dir = ModelDir::new("unused");
        for model in WhisperModel::ALL {
            assert_eq!(dir.model_by_hash(model.sha256()), Some(model));
        }
        assert_eq!(dir.model_by_hash(&sha(b"base")), None);
    }

    #[test]
    fn a_new_model_takes_the_place_of_the_one_before() -> io::Result<()> {
        let (_tmp, dir) = pinned();
        assert_eq!(
            take(&dir, b"base", Some(WhisperModel::Base)),
            Outcome::Installed(WhisperModel::Base)
        );
        assert_eq!(dir.check(), Some(WhisperModel::Base));
        assert_eq!(
            take(&dir, b"small", Some(WhisperModel::Small)),
            Outcome::Installed(WhisperModel::Small)
        );
        assert_eq!(dir.check(), Some(WhisperModel::Small));
        // Only the chosen one is kept.
        let files: Vec<_> = std::fs::read_dir(&dir.dir)?
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .collect();
        assert_eq!(files, vec![std::ffi::OsString::from(FILE_NAME)]);
        Ok(())
    }

    #[test]
    fn a_failed_or_wrong_download_keeps_the_model_in_use() -> io::Result<()> {
        let (_tmp, dir) = pinned();
        take(&dir, b"base", None);
        // The base model is not the small one asked for.
        assert_eq!(
            take(&dir, b"base", Some(WhisperModel::Small)),
            Outcome::Mismatch
        );
        assert_eq!(take(&dir, b"nothing", None), Outcome::Mismatch);
        let broken = dir.take_in(
            Broken { left: 10 },
            None,
            Some(WhisperModel::Small),
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(broken, Outcome::Failed);
        assert_eq!(std::fs::read(dir.model())?, b"base");
        assert!(!dir.incoming().exists());
        assert_eq!(dir.check(), Some(WhisperModel::Base));
        Ok(())
    }

    #[test]
    fn a_file_of_the_users_own_may_be_either_model() -> io::Result<()> {
        let (tmp, dir) = pinned();
        let file = tmp.path().join("ggml-small.bin");
        std::fs::write(&file, b"small")?;
        assert_eq!(
            dir.take_file(&file, &AtomicBool::new(false)),
            Outcome::Installed(WhisperModel::Small)
        );
        std::fs::write(&file, b"base")?;
        assert_eq!(
            dir.take_file(&file, &AtomicBool::new(false)),
            Outcome::Installed(WhisperModel::Base)
        );
        assert_eq!(dir.check(), Some(WhisperModel::Base));
        Ok(())
    }
}
