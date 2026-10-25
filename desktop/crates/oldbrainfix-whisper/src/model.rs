//! The speech model Transcripts are made with (ADR 0007), the phone's:
//! whisper.cpp's multilingual `base` model, fetched once on request from
//! Old Brain Fix's own GitHub release, never from anywhere else, or taken
//! from a file of the user's own; kept only when its SHA-256 is the one
//! pinned here, the phone's (`WhisperModel` in speech_platform.dart).
//!
//! A model on its way in is written beside the one in use and only takes
//! its place once it is checked, so a broken download never leaves a
//! half model behind.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// The release asset; GitHub may redirect it to its download host.
pub const URL: &str =
    "https://github.com/paxel/oldbrainfix/releases/download/whisper-model-v1/ggml-base.bin";

/// The SHA-256 of whisper.cpp's `ggml-base.bin`.
pub const SHA256: &str = "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe";

/// Its size in megabytes (147,951,465 bytes), as told before the
/// download.
pub const MEGABYTES: i64 = 148;

/// The model's file name, the phone's.
pub const FILE_NAME: &str = "ggml-base.bin";

/// How fetching the model ended, as on the phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Checked and in use.
    Installed,
    /// Stopped by the user.
    Cancelled,
    /// The download or copy broke off.
    Failed,
    /// Not the pinned model: not kept.
    Mismatch,
}

/// Where the model lives: one directory of this computer's data.
#[derive(Debug, Clone)]
pub struct ModelDir {
    dir: PathBuf,
}

impl ModelDir {
    pub fn new(dir: impl Into<PathBuf>) -> ModelDir {
        ModelDir { dir: dir.into() }
    }

    /// The model in use, once it was checked and kept.
    pub fn model(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// A model on its way in, not yet checked.
    fn incoming(&self) -> PathBuf {
        self.dir.join(format!("{FILE_NAME}.part"))
    }

    /// Whether the pinned model is here. A model that is not the pinned
    /// one is dropped, and so is a fetch cut off when the app was last
    /// closed. Reads the whole model: run it off the window's thread.
    pub fn check(&self) -> bool {
        let _ = std::fs::remove_file(self.incoming());
        let model = self.model();
        let Ok(file) = File::open(&model) else {
            return false;
        };
        match copy_hashing(file, io::sink(), &AtomicBool::new(false), |_| {}) {
            Ok(Some(hash)) if hash == SHA256 => true,
            _ => {
                let _ = std::fs::remove_file(model);
                false
            }
        }
    }

    /// Downloads the model from [`URL`], over HTTPS only, and keeps it
    /// when it is the pinned one. `progress` hears the bytes so far and
    /// the whole size when the server tells it; `stop` cancels.
    pub fn download(&self, stop: &AtomicBool, progress: impl FnMut(u64, Option<u64>)) -> Outcome {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .build()
            .into();
        let response = match agent.get(URL).call() {
            Ok(response) => response,
            Err(_) => return Outcome::Failed,
        };
        let body = response.into_body();
        let total = body.content_length();
        self.take_in(body.into_reader(), total, stop, progress)
    }

    /// Takes the model from the file at `path` the user picked, and keeps
    /// it when it is the pinned one.
    pub fn take_file(&self, path: &Path, stop: &AtomicBool) -> Outcome {
        match File::open(path) {
            Ok(file) => {
                let total = file.metadata().ok().map(|m| m.len());
                self.take_in(file, total, stop, |_, _| {})
            }
            Err(_) => Outcome::Failed,
        }
    }

    /// Copies `input` to the side, checks it, and keeps it as the model
    /// in use when it is the pinned one; anything else is dropped.
    pub fn take_in(
        &self,
        input: impl Read,
        total: Option<u64>,
        stop: &AtomicBool,
        mut progress: impl FnMut(u64, Option<u64>),
    ) -> Outcome {
        let incoming = self.incoming();
        let outcome = (|| {
            std::fs::create_dir_all(&self.dir)?;
            let mut out = File::create(&incoming)?;
            let hash = copy_hashing(input, &mut out, stop, |n| progress(n, total))?;
            out.sync_all()?;
            Ok::<_, io::Error>(match hash {
                None => Outcome::Cancelled,
                Some(hash) if hash != SHA256 => Outcome::Mismatch,
                Some(_) => {
                    std::fs::rename(&incoming, self.model())?;
                    Outcome::Installed
                }
            })
        })()
        .unwrap_or(Outcome::Failed);
        if outcome != Outcome::Installed {
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
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(outcome, Outcome::Mismatch);
        assert!(!dir.model().exists());
        assert!(!dir.incoming().exists());
        assert!(!dir.check());
    }

    #[test]
    fn a_broken_or_cancelled_fetch_leaves_nothing_behind() {
        let (_tmp, dir) = dir();
        let broken = dir.take_in(
            Broken { left: 3 << 20 },
            None,
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert_eq!(broken, Outcome::Failed);
        let cancelled = dir.take_in(&b"x"[..], Some(1), &AtomicBool::new(true), |_, _| {});
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
        assert!(!dir.check());
        assert!(!dir.model().exists());
        assert!(!dir.incoming().exists());
        Ok(())
    }
}
