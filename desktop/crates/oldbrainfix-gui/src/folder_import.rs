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
//! shows as it goes. Aborted on the page Running (obf-1078), it stops
//! before its next Note, and the Notes it wrote stay. Before it reads
//! a file it lists them, and when an earlier Import of the same source
//! made Notes of any of them, it asks the window whether to fill that
//! one in and waits for the answer (obf-1078).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Instant;

use parking_lot::Mutex;

use chrono::{DateTime, Utc};
use oldbrainfix_core::Store;
use oldbrainfix_core::import_formats::ImportFormat;
use oldbrainfix_core::imports::{
    EarlierImport, ImportFile, ImportSourceKind, ImportSummary, Imports, MAX_FILE_BYTES,
};

use crate::keep_import::{self, KeepSource};

/// How an Import ended: what it made, or why it failed, in full.
pub type Imported = Result<ImportSummary, String>;

/// Which Import runs: of the folder of a name for people, or of a
/// Google Keep export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Importing {
    Folder(String),
    Keep,
    /// Of a Markdown file shown in its viewer (obf-1056), by its name for
    /// people.
    Markdown(String),
}

/// Hands the Import's thread its store: a connection of its own to the
/// window's database, or the window's own.
pub type ImportStore = Box<dyn FnOnce() -> Arc<Store> + Send>;

/// How far an Import is: `None` while it reads, then how many of how
/// many Notes are in.
type Progress = Arc<Mutex<Option<(usize, usize)>>>;

/// `tell` hearing how many of how many Notes are done, at low priority
/// (obf-1061): after each Note the thread rests as long as the Note
/// took, so the window's reads and writes, which wait for its
/// transaction, get the database between two Notes. Deleting an Area
/// beside the window rests the same way (obf-1093).
pub(crate) fn low_priority(tell: impl Fn(usize, usize)) -> impl Fn(usize, usize) {
    let since = std::cell::Cell::new(std::time::Instant::now());
    move |done, of| {
        tell(done, of);
        if done > 0 {
            std::thread::sleep(since.get().elapsed());
        }
        since.set(std::time::Instant::now());
    }
}

/// The answer to "Fill in the Import of … into …?" (obf-1078): the
/// missing items into the earlier Import's Area, every item into a new
/// Area, or nothing at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillIn {
    Fill,
    New,
    Cancel,
}

/// How an Import ended, with the Area it was asked to fill in, if any.
pub type Ended = (Importing, Imported, Option<String>);

/// Where an Import writes, once asked: into a new Area, into the Area of
/// the earlier Import it fills in, or nowhere, cancelled.
enum Target {
    New,
    FillIn(String),
    Cancelled,
}

/// What an Import's job is handed on its thread: the store, who hears
/// how far it is, whether it is to stop, and who answers the question
/// about an earlier Import.
struct Hooks<'a> {
    store: &'a Store,
    tell: &'a dyn Fn(usize, usize),
    stop: &'a dyn Fn() -> bool,
    ask: &'a dyn Fn(EarlierImport) -> FillIn,
}

impl Hooks<'_> {
    /// Where the Import of `kind` from `from` writes (obf-1078): when an
    /// earlier Import of it made Notes of any of `paths`, as the window
    /// answers about it; else into a new Area.
    fn target<I>(&self, kind: ImportSourceKind, from: &str, paths: I) -> Result<Target, String>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let earlier = Imports::new(self.store)
            .earlier(kind, from, paths)
            .map_err(|e| e.to_string())?;
        Ok(match earlier {
            None => Target::New,
            Some(earlier) => {
                let area = earlier.area_id.clone();
                match (self.ask)(earlier) {
                    FillIn::Fill => Target::FillIn(area),
                    FillIn::New => Target::New,
                    FillIn::Cancel => Target::Cancelled,
                }
            }
        })
    }

    /// The Imports from `from`, filling in the Area `into` when one is
    /// named, telling how far they are and asking whether to stop.
    fn imports<'b>(&'b self, from: &'b str, into: Option<&'b str>) -> Imports<'b> {
        let imports = Imports::new(self.store)
            .reporting(self.tell)
            .stopping(self.stop)
            .from_source(from);
        match into {
            Some(area) => imports.filling_in(area),
            None => imports,
        }
    }
}

/// What an Import's job ends with: what it made, why it failed, or
/// `None` once cancelled at the question.
type Job = Result<Option<ImportSummary>, String>;

/// Where `path` was picked, as the Area an Import makes keeps it
/// ([`Imports::from_source`]): its absolute path.
pub fn source_of(path: &Path) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// The Import running off the window's thread, until it answers.
struct Running {
    kind: Importing,
    /// `None` once cancelled at the question.
    answer: Receiver<Option<Imported>>,
    progress: Progress,
    /// When it started, for the page Running (obf-1078).
    since: Instant,
    /// Raised by Abort there: it stops before its next Note.
    stop: Arc<AtomicBool>,
    /// The earlier Import it asks about while it waits for the answer
    /// (obf-1078).
    question: Arc<Mutex<Option<EarlierImport>>>,
    /// Takes the answer to it.
    answers: Sender<FillIn>,
    /// The Area it was told to fill in.
    filling_in: Option<String>,
}

/// The Import running off the window's thread, until it answers, with
/// how far it is.
#[derive(Default)]
pub struct FolderImport {
    running: Option<Running>,
}

impl FolderImport {
    /// Imports the `folder` on a thread of its own into the store `open`
    /// hands it there, its Notes into a new Area named `area_name`;
    /// `name` is the folder's name for people. Nothing while one runs.
    pub fn start(&mut self, open: ImportStore, folder: PathBuf, name: String, area_name: String) {
        self.run(Importing::Folder(name), open, move |hooks| {
            let failed = |e: std::io::Error| format!("{}: {e}", folder.display());
            let from = source_of(&folder);
            let listed = list_folder(&folder).map_err(failed)?;
            let paths = listed.iter().map(|file| file.name.as_str());
            let into = match hooks.target(ImportSourceKind::Folder, &from, paths)? {
                Target::Cancelled => return Ok(None),
                Target::New => None,
                Target::FillIn(area) => Some(area),
            };
            let files = read_listed(&listed).map_err(failed)?;
            hooks
                .imports(&from, into.as_deref())
                .folder(&area_name, &files)
                .map(Some)
                .map_err(|e| e.to_string())
        });
    }

    /// Imports the Markdown file `file` and the files `linked` it links
    /// to in the folder `root` its links are found in on a thread of its
    /// own (obf-1056), into the store `open` hands it there, into a new
    /// Area named `area_name`, with the pictures they show as photos
    /// ([`Imports::linked`]); `name` is the file's name for people.
    /// Nothing while an Import runs.
    pub fn start_linked(
        &mut self,
        open: ImportStore,
        (file, root, linked): (PathBuf, PathBuf, Vec<PathBuf>),
        name: String,
        area_name: String,
    ) {
        self.run(Importing::Markdown(name), open, move |hooks| {
            let from = source_of(&file);
            let paths = std::iter::once(&file)
                .chain(&linked)
                .filter_map(|path| crate::md_source::path_in(&root, path));
            let into = match hooks.target(ImportSourceKind::Markdown, &from, paths)? {
                Target::Cancelled => return Ok(None),
                Target::New => None,
                Target::FillIn(area) => Some(area),
            };
            let (files, pictures) = read_linked(&root, &file, &linked)?;
            hooks
                .imports(&from, into.as_deref())
                .linked(&area_name, &files, &pictures)
                .map(Some)
                .map_err(|e| e.to_string())
        });
    }

    /// Imports the Google Keep notes of the Takeout export `source` on a
    /// thread of its own into the store `open` hands it there, into a
    /// new Area named `area_name` (ADR 0036, Google Keep). Nothing while
    /// an Import runs.
    pub fn start_keep(&mut self, open: ImportStore, source: KeepSource, area_name: String) {
        self.run(Importing::Keep, open, move |hooks| {
            let from = source_of(source.path());
            let paths = keep_import::paths(&source)?;
            let into = match hooks.target(ImportSourceKind::Keep, &from, &paths)? {
                Target::Cancelled => return Ok(None),
                Target::New => None,
                Target::FillIn(area) => Some(area),
            };
            let (files, refused) = keep_import::read(&source)?;
            let mut summary = hooks
                .imports(&from, into.as_deref())
                .keep(&area_name, &files)
                .map_err(|e| e.to_string())?;
            summary.not_text += refused;
            Ok(Some(summary))
        });
    }

    /// Runs `job` on a thread of its own as the Import `kind`, on the
    /// store `open` hands it there, telling it how far it is and whether
    /// it is to stop, and asking the window about an earlier Import,
    /// unless one runs.
    fn run(
        &mut self,
        kind: Importing,
        open: ImportStore,
        job: impl FnOnce(Hooks<'_>) -> Job + Send + 'static,
    ) {
        if self.running.is_some() {
            return;
        }
        let progress = Progress::default();
        let shared = progress.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let asked = stop.clone();
        let question = Arc::new(Mutex::new(None));
        let asking = question.clone();
        let (answers, answered) = std::sync::mpsc::channel();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let store = open();
            let tell = low_priority(|written, of| *shared.lock() = Some((written, of)));
            let stopped = || asked.load(Ordering::SeqCst);
            let ask = |earlier: EarlierImport| {
                *asking.lock() = Some(earlier);
                // A window gone meanwhile answers nothing: nothing is
                // written.
                answered.recv().unwrap_or(FillIn::Cancel)
            };
            let hooks = Hooks {
                store: &store,
                tell: &tell,
                stop: &stopped,
                ask: &ask,
            };
            // A window gone meanwhile is nobody's problem.
            drop(tx.send(job(hooks).transpose()));
        });
        self.running = Some(Running {
            kind,
            answer: rx,
            progress,
            since: Instant::now(),
            stop,
            question,
            answers,
            filling_in: None,
        });
    }

    /// The earlier Import of the same source the Import running asks
    /// about (obf-1078), until it is answered.
    pub fn asking(&self) -> Option<EarlierImport> {
        self.running
            .as_ref()
            .and_then(|r| r.question.lock().clone())
    }

    /// Answers the question of the Import running, if it asks one.
    pub fn answer(&mut self, fill_in: FillIn) {
        let Some(running) = self.running.as_mut() else {
            return;
        };
        let Some(earlier) = running.question.lock().take() else {
            return;
        };
        if fill_in == FillIn::Fill {
            running.filling_in = Some(earlier.area_id);
        }
        // A thread gone meanwhile answers by itself.
        let _ = running.answers.send(fill_in);
    }

    /// How many of how many Notes the Import running has written; `None`
    /// while it reads, or while none runs.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.running.as_ref().and_then(|r| *r.progress.lock())
    }

    /// When the Import running started, `None` while none runs.
    pub fn since(&self) -> Option<Instant> {
        self.running.as_ref().map(|r| r.since)
    }

    /// Stops the Import running before its next Note (obf-1078): the
    /// Notes it wrote stay, and its answer says how many it was to write.
    /// One still asking about an earlier Import writes nothing.
    pub fn abort(&mut self) {
        if let Some(running) = &self.running {
            running.stop.store(true, Ordering::SeqCst);
        }
        self.answer(FillIn::Cancel);
    }

    /// Whether the Import running was aborted and has not answered yet.
    pub fn aborting(&self) -> bool {
        self.running
            .as_ref()
            .is_some_and(|r| r.stop.load(Ordering::SeqCst))
    }

    /// Whether an Import is reading its folder or zip.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// Which Import runs, `None` while none does.
    pub fn kind(&self) -> Option<&Importing> {
        self.running.as_ref().map(|r| &r.kind)
    }

    /// Takes in the Import's answer once there, with which Import it
    /// was and the Area it was told to fill in: `None` while it runs,
    /// and also once it was cancelled at its question, when none runs
    /// any more.
    pub fn finished(&mut self) -> Option<Ended> {
        let answer = match self.running.as_ref()?.answer.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return None,
            // The thread broke off without an answer.
            Err(TryRecvError::Disconnected) => Some(Err("the import broke off".to_string())),
        };
        let running = self.running.take()?;
        Some((running.kind, answer?, running.filling_in))
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
    read_listed(&list_folder(folder)?)
}

/// A file of a folder to import, listed before it is read (obf-1078):
/// its path inside the folder, where it lies, and whether it may become
/// a Note.
struct Listed {
    name: String,
    path: PathBuf,
    note: bool,
}

/// Every file [`read_folder`] reads, unread.
fn list_folder(folder: &Path) -> std::io::Result<Vec<Listed>> {
    let mut listed = Vec::new();
    list_into(folder, "", &mut listed)?;
    Ok(listed)
}

fn list_into(dir: &Path, prefix: &str, listed: &mut Vec<Listed>) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        if entry.file_type()?.is_dir() {
            if !name.starts_with('.') {
                list_into(&entry.path(), &format!("{path}/"), listed)?;
            }
            continue;
        }
        let note = may_become_note(&name);
        // A link to a folder is no file.
        let folder = if note {
            std::fs::metadata(entry.path())?.is_dir()
        } else {
            std::fs::metadata(entry.path()).is_ok_and(|m| m.is_dir())
        };
        if !folder {
            listed.push(Listed {
                name: path,
                path: entry.path(),
                note,
            });
        }
    }
    Ok(())
}

/// The files `listed`, each that may become a Note read, any other
/// handed over unread, to be counted.
fn read_listed(listed: &[Listed]) -> std::io::Result<Vec<ImportFile>> {
    let mut files = Vec::new();
    for file in listed {
        if !file.note {
            files.push(ImportFile {
                name: file.name.clone(),
                modified: None,
                bytes: Vec::new(),
            });
            continue;
        }
        let metadata = std::fs::metadata(&file.path)?;
        let modified = metadata.modified().ok().map(DateTime::<Utc>::from);
        let bytes = if metadata.len() > MAX_FILE_BYTES as u64 {
            // What the core takes as larger than it reads: only its
            // length counts, and its pages are never touched.
            vec![0; MAX_FILE_BYTES + 1]
        } else {
            std::fs::read(&file.path)?
        };
        files.push(ImportFile {
            name: file.name.clone(),
            modified,
            bytes,
        });
    }
    Ok(files)
}

/// The Markdown file `file` and the files `linked` of the folder `root`,
/// by their paths in it, and the pictures they show there (obf-1056);
/// what fails to be read fails the Import. A file or picture larger than
/// the core takes is handed over as that many bytes and one more,
/// unread, to be counted as too large. Nothing outside `root` is read.
pub fn read_linked(
    root: &Path,
    file: &Path,
    linked: &[PathBuf],
) -> Result<(Vec<ImportFile>, Vec<ImportFile>), String> {
    use oldbrainfix_core::imports::MAX_PHOTO_BYTES;
    use oldbrainfix_core::obsidian;
    let read = |path: &Path, most: usize| -> Result<(Option<DateTime<Utc>>, Vec<u8>), String> {
        let failed = |e: std::io::Error| format!("{}: {e}", path.display());
        let metadata = std::fs::metadata(path).map_err(failed)?;
        let modified = metadata.modified().ok().map(DateTime::<Utc>::from);
        if metadata.len() > most as u64 {
            return Ok((modified, vec![0; most + 1]));
        }
        Ok((modified, std::fs::read(path).map_err(failed)?))
    };
    let index = crate::md_source::index(root);
    let mut files = Vec::new();
    let mut wanted: Vec<String> = Vec::new();
    for path in std::iter::once(file).chain(linked.iter().map(PathBuf::as_path)) {
        let Some(name) = crate::md_source::path_in(root, path) else {
            continue;
        };
        let (modified, bytes) = read(path, MAX_FILE_BYTES)?;
        let text = crate::md_source::text_of(&bytes);
        for link in obsidian::wiki_links(&text).into_iter().filter(|l| l.embed) {
            if let Some(found) = obsidian::file_target(&link.target, &name, &index)
                .and_then(|i| index.get(i))
                .filter(|p| !wanted.contains(p))
            {
                wanted.push(found.clone());
            }
        }
        for picture in obsidian::pictures(&text) {
            if let Some(found) = obsidian::picture_target(&picture.path, &name, &index)
                .and_then(|i| index.get(i))
                .filter(|p| !wanted.contains(p))
            {
                wanted.push(found.clone());
            }
        }
        files.push(ImportFile {
            name,
            modified,
            bytes,
        });
    }
    let mut pictures = Vec::new();
    for path in wanted {
        let Some(full) = crate::md_source::inside(root, &path) else {
            continue;
        };
        let (modified, bytes) = read(&full, MAX_PHOTO_BYTES)?;
        pictures.push(ImportFile {
            name: path,
            modified,
            bytes,
        });
    }
    Ok((files, pictures))
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
