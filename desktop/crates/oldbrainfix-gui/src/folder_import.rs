//! The Import of a folder (ADR 0036), as the phone's `folder_import.dart`:
//! the files of the folder picked and of its folders, but not of hidden
//! ones (`.git`, `.obsidian`), read off the window's thread and made into
//! Notes in a new Area by the core. Only what may become a Note is read,
//! and none larger than [`MAX_FILE_BYTES`]; every other file is handed
//! over unread, so the core counts it as skipped. A file that cannot be
//! read fails the Import, and then nothing is written. The import from
//! Google Keep (`keep_import`) runs the same way, one Import at a time.
//! It runs beside the window on a connection of its own (obf-1061), each
//! Note written in a transaction of its own and a rest after it, so the
//! window reads and writes on meanwhile; how many of its Notes are in
//! shows as it goes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use parking_lot::Mutex;

use chrono::{DateTime, Utc};
use oldbrainfix_core::Store;
use oldbrainfix_core::import_formats::ImportFormat;
use oldbrainfix_core::imports::{ImportFile, ImportSummary, Imports, MAX_FILE_BYTES};

use crate::keep_import::{self, KeepSource};

/// How an Import ended: what it made, or why it failed, in full.
pub type Imported = Result<ImportSummary, String>;

/// Which Import runs: of the folder of a name for people, or of a
/// Google Keep export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Importing {
    Folder(String),
    Keep,
}

/// Hands the Import's thread its store: a connection of its own to the
/// window's database, or the window's own.
pub type ImportStore = Box<dyn FnOnce() -> Arc<Store> + Send>;

/// How far an Import is: `None` while it reads, then how many of how
/// many Notes are in.
type Progress = Arc<Mutex<Option<(usize, usize)>>>;

/// The Import running off the window's thread, until it answers, with
/// how far it is.
#[derive(Default)]
pub struct FolderImport {
    running: Option<(Importing, Receiver<Imported>, Progress)>,
}

impl FolderImport {
    /// Imports the `folder` on a thread of its own into the store `open`
    /// hands it there, its Notes into a new Area named `area_name`;
    /// `name` is the folder's name for people. Nothing while one runs.
    pub fn start(&mut self, open: ImportStore, folder: PathBuf, name: String, area_name: String) {
        self.run(Importing::Folder(name), open, move |store, tell| {
            read_folder(&folder)
                .map_err(|e| format!("{}: {e}", folder.display()))
                .and_then(|files| {
                    Imports::new(store)
                        .reporting(tell)
                        .folder(&area_name, &files)
                        .map_err(|e| e.to_string())
                })
        });
    }

    /// Imports the Google Keep notes of the Takeout export `source` on a
    /// thread of its own into the store `open` hands it there, into a
    /// new Area named `area_name` (ADR 0036, Google Keep). Nothing while
    /// an Import runs.
    pub fn start_keep(&mut self, open: ImportStore, source: KeepSource, area_name: String) {
        self.run(Importing::Keep, open, move |store, tell| {
            keep_import::read(&source).and_then(|(files, refused)| {
                let mut summary = Imports::new(store)
                    .reporting(tell)
                    .keep(&area_name, &files)
                    .map_err(|e| e.to_string())?;
                summary.not_text += refused;
                Ok(summary)
            })
        });
    }

    /// Runs `job` on a thread of its own as the Import `kind`, on the
    /// store `open` hands it there, telling it how far it is, unless one
    /// runs.
    fn run(
        &mut self,
        kind: Importing,
        open: ImportStore,
        job: impl FnOnce(&Store, &dyn Fn(usize, usize)) -> Imported + Send + 'static,
    ) {
        if self.running.is_some() {
            return;
        }
        let progress = Progress::default();
        let shared = progress.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let store = open();
            // At low priority: after each Note it rests as long as the
            // Note took, so the window's reads and writes, which wait
            // for its transaction, get the database between two Notes.
            let since = std::cell::Cell::new(std::time::Instant::now());
            let tell = |written: usize, of: usize| {
                *shared.lock() = Some((written, of));
                if written > 0 {
                    std::thread::sleep(since.get().elapsed());
                }
                since.set(std::time::Instant::now());
            };
            // A window gone meanwhile is nobody's problem.
            drop(tx.send(job(&store, &tell)));
        });
        self.running = Some((kind, rx, progress));
    }

    /// How many of how many Notes the Import running has written; `None`
    /// while it reads, or while none runs.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.running
            .as_ref()
            .and_then(|(_, _, progress)| *progress.lock())
    }

    /// Whether an Import is reading its folder or zip.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// Which Import runs, `None` while none does.
    pub fn kind(&self) -> Option<&Importing> {
        self.running.as_ref().map(|(kind, ..)| kind)
    }

    /// Takes in the Import's answer once there, with which Import it
    /// was: `None` while it runs.
    pub fn finished(&mut self) -> Option<(Importing, Imported)> {
        let (_, rx, _) = self.running.as_ref()?;
        let answer = match rx.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return None,
            // The thread broke off without an answer.
            Err(TryRecvError::Disconnected) => Err("the import broke off".to_string()),
        };
        let (kind, ..) = self.running.take()?;
        Some((kind, answer))
    }
}

/// The name of `folder` for people: its last part, or all of it for a
/// drive's or the file system's root.
pub fn folder_name(folder: &Path) -> String {
    folder.file_name().map_or_else(
        || folder.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Every file of `folder` and of its folders, but not of hidden ones, by
/// its path inside the folder, folders separated by `/`. A link to a
/// folder is not followed.
pub fn read_folder(folder: &Path) -> std::io::Result<Vec<ImportFile>> {
    let mut files = Vec::new();
    read_into(folder, "", &mut files)?;
    Ok(files)
}

fn read_into(dir: &Path, prefix: &str, files: &mut Vec<ImportFile>) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        if entry.file_type()?.is_dir() {
            if !name.starts_with('.') {
                read_into(&entry.path(), &format!("{path}/"), files)?;
            }
            continue;
        }
        if !may_become_note(&name) {
            // A link to a folder is no file; any other is handed over
            // unread, to be counted.
            if !std::fs::metadata(entry.path()).is_ok_and(|m| m.is_dir()) {
                files.push(ImportFile {
                    name: path,
                    modified: None,
                    bytes: Vec::new(),
                });
            }
            continue;
        }
        let metadata = std::fs::metadata(entry.path())?;
        if metadata.is_dir() {
            continue;
        }
        let modified = metadata.modified().ok().map(DateTime::<Utc>::from);
        let bytes = if metadata.len() > MAX_FILE_BYTES as u64 {
            // What the core takes as larger than it reads: only its
            // length counts, and its pages are never touched.
            vec![0; MAX_FILE_BYTES + 1]
        } else {
            std::fs::read(entry.path())?
        };
        files.push(ImportFile {
            name: path,
            modified,
            bytes,
        });
    }
    Ok(())
}

/// Whether the file `name` may become a Note, as the core takes it: a
/// Markdown (`.md`, `.markdown`), text (`.txt`), AsciiDoc (`.adoc`,
/// `.asciidoc`, `.asc`), Org (`.org`), reStructuredText (`.rst`) or HTML
/// (`.html`, `.htm`) file, in any case of ASCII letters, not a hidden one.
/// Others are not worth reading.
pub fn may_become_note(name: &str) -> bool {
    if name.starts_with('.') {
        return false;
    }
    name.rsplit_once('.').is_some_and(|(_, ending)| {
        ["md", "markdown", "txt"]
            .iter()
            .any(|known| ending.eq_ignore_ascii_case(known))
            || ImportFormat::of_extension(&ending.to_ascii_lowercase()).is_some()
    })
}

#[cfg(test)]
mod tests;
