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
//!
//! A kept model's SHA-256 is recorded beside it with its size and
//! modification time, as on the phone, so a start reads the whole model
//! again only when the file changed since.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, UNIX_EPOCH};

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

    /// The model in use's SHA-256 with its size and modification time
    /// then: one line, `<sha256> <bytes> <milliseconds>`, as on the phone.
    fn record(&self) -> PathBuf {
        self.dir.join(format!("{FILE_NAME}.checked"))
    }

    /// Which pinned model is here, if any. A model that is none of them
    /// is dropped, and so is a fetch cut off when the app was last
    /// closed. Reads the whole model unless its record still fits it:
    /// run it off the window's thread.
    pub fn check(&self) -> Option<WhisperModel> {
        self.check_reading(|path| File::open(path))
    }

    /// As [`ModelDir::check`], reading the model through `open`.
    fn check_reading<R: Read>(
        &self,
        open: impl FnOnce(&Path) -> io::Result<R>,
    ) -> Option<WhisperModel> {
        let _ = std::fs::remove_file(self.incoming());
        let model = self.model();
        let stamp = stamp(&model)?;
        // A record that is broken, stale or names no pinned model is
        // none: the model is hashed again rather than dropped for it.
        let recorded = read_record(&self.record())
            .filter(|r| r.size == stamp.size && r.modified == stamp.modified)
            .and_then(|r| self.model_by_hash(&r.sha256));
        if recorded.is_some() {
            return recorded;
        }
        // A model that cannot be read now, say while another program
        // holds it, is kept for the next start; only a wrong one goes.
        let hash = open(&model)
            .and_then(|file| copy_hashing(file, io::sink(), &AtomicBool::new(false), |_| {}))
            .ok()
            .flatten()?;
        let found = self.model_by_hash(&hash);
        if found.is_some() {
            // Stamped as before the hashing: a file changed meanwhile is
            // hashed again at the next start.
            self.write_record(&hash, stamp);
        } else {
            let _ = std::fs::remove_file(self.record());
            let _ = std::fs::remove_file(model);
        }
        found
    }

    /// Records `sha256` for the model in use with its `stamp`. Written to the side and renamed, so it is
    /// whole or not there; when it cannot be written, the next start
    /// hashes the model again.
    fn write_record(&self, sha256: &str, stamp: Stamp) {
        let record = self.record();
        let side = self.dir.join(format!("{FILE_NAME}.checked.part"));
        let written = (|| {
            let line = format!("{sha256} {} {}\n", stamp.size, stamp.modified);
            std::fs::write(&side, line)?;
            std::fs::rename(&side, &record)
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&side);
        }
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
                    // The record of the model before goes first, so it
                    // never stands for another file.
                    let _ = std::fs::remove_file(self.record());
                    std::fs::rename(&incoming, self.model())?;
                    if let Some(stamp) = stamp(&self.model()) {
                        self.write_record(&hash, stamp);
                    }
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

    /// Drops the model in use, and its record.
    pub fn remove(&self) {
        let _ = std::fs::remove_file(self.record());
        let _ = std::fs::remove_file(self.model());
    }
}

/// A file's size in bytes and modification time in milliseconds since
/// the epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    size: u64,
    modified: i128,
}

/// The stamp of the file at `path`, or `None` when there is no file.
/// A time the system cannot tell is recorded as 0, so such a model is
/// still hashed only once.
fn stamp(path: &Path) -> Option<Stamp> {
    let meta = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    let modified = meta
        .modified()
        .map_or(0, |time| match time.duration_since(UNIX_EPOCH) {
            Ok(after) => after.as_millis() as i128,
            Err(before) => -(before.duration().as_millis() as i128),
        });
    Some(Stamp {
        size: meta.len(),
        modified,
    })
}

/// A model's SHA-256 as recorded, with its stamp then.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    sha256: String,
    size: u64,
    modified: i128,
}

/// The record in `path`, or `None` when there is none or it is not one:
/// anything but a lowercase hex SHA-256, a size and a time is no record.
fn read_record(path: &Path) -> Option<Record> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut parts = text.trim().split(' ');
    let (sha256, size, modified) = (parts.next()?, parts.next()?, parts.next()?);
    let hex = sha256.len() == 64
        && sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !hex || parts.next().is_some() {
        return None;
    }
    Some(Record {
        sha256: sha256.to_string(),
        size: size.parse().ok()?,
        modified: modified.parse().ok()?,
    })
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
        // A reader answers at most the buffer's length; one that claims
        // more is an error, not a crash.
        let chunk = buffer.get(..read).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "read more than the buffer holds",
            )
        })?;
        digest.update(chunk);
        output.write_all(chunk)?;
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
mod tests;
