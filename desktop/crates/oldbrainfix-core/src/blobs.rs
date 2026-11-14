//! Photo and voice recording bytes live outside the log in
//! content-addressed storage, keyed by their SHA-256, as the Dart core's
//! `Blobs` keeps them: one file per blob in a directory next to the
//! database file, or memory for in-memory stores. The log only names a
//! blob by its hash, so whatever carries the log carries the reference,
//! and the bytes can follow on their own.

use std::collections::HashMap;
use std::path::PathBuf;

use parking_lot::Mutex;

use crate::Result;

/// Where blobs are kept.
pub trait Blobs: Send + Sync {
    /// Keeps `bytes` under `hash`; a hash already held stays as it is.
    fn put(&self, hash: &str, bytes: &[u8]) -> Result<()>;

    /// The bytes under `hash`, or `None` when they are not here (yet).
    fn get(&self, hash: &str) -> Option<Vec<u8>>;

    /// Whether the bytes under `hash` are here, without reading them.
    fn has(&self, hash: &str) -> bool {
        self.get(hash).is_some()
    }

    /// Lets go of the bytes under `hash`; returns whether they were here.
    fn remove(&self, hash: &str) -> Result<bool>;
}

/// A blob name is its SHA-256 as lowercase hex, nothing else: a hash
/// arriving from another Device ("../../x") must never leave the
/// directory.
pub fn is_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// [`Blobs`] as one file per blob in a directory.
pub struct FileBlobs {
    dir: PathBuf,
}

impl FileBlobs {
    /// The blobs in `dir`, made when it is not there yet.
    pub fn new(dir: impl Into<PathBuf>) -> Result<FileBlobs> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(FileBlobs { dir })
    }

    fn file(&self, hash: &str) -> Option<PathBuf> {
        is_hash(hash).then(|| self.dir.join(hash))
    }
}

impl Blobs for FileBlobs {
    fn put(&self, hash: &str, bytes: &[u8]) -> Result<()> {
        // Anything that is no hash is a blob that is not here, never an
        // error, as in the Dart core.
        let Some(file) = self.file(hash) else {
            return Ok(());
        };
        if file.exists() {
            return Ok(());
        }
        // Written beside, then renamed: a kill mid-write leaves a stray
        // .tmp, never a cut-off blob that counts as present.
        let tmp = self.dir.join(format!("{hash}.tmp"));
        {
            use std::io::Write;
            let mut out = std::fs::File::create(&tmp)?;
            out.write_all(bytes)?;
            out.sync_all()?;
        }
        std::fs::rename(&tmp, &file)?;
        Ok(())
    }

    fn get(&self, hash: &str) -> Option<Vec<u8>> {
        std::fs::read(self.file(hash)?).ok()
    }

    fn has(&self, hash: &str) -> bool {
        self.file(hash).is_some_and(|file| file.is_file())
    }

    fn remove(&self, hash: &str) -> Result<bool> {
        let Some(file) = self.file(hash) else {
            return Ok(false);
        };
        match std::fs::remove_file(file) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

/// [`Blobs`] in memory: gone with the store. For tests.
#[derive(Default)]
pub struct MemoryBlobs {
    held: Mutex<HashMap<String, Vec<u8>>>,
}

impl Blobs for MemoryBlobs {
    fn put(&self, hash: &str, bytes: &[u8]) -> Result<()> {
        self.held
            .lock()
            .entry(hash.to_string())
            .or_insert_with(|| bytes.to_vec());
        Ok(())
    }

    fn get(&self, hash: &str) -> Option<Vec<u8>> {
        self.held.lock().get(hash).cloned()
    }

    fn has(&self, hash: &str) -> bool {
        self.held.lock().contains_key(hash)
    }

    fn remove(&self, hash: &str) -> Result<bool> {
        Ok(self.held.lock().remove(hash).is_some())
    }
}

#[cfg(test)]
mod tests;
