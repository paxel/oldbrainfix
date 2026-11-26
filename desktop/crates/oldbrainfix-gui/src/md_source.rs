//! What the desktop's viewer of a Markdown file reads (obf-1056), all of
//! it off the window's thread: the file, read again whenever it changes
//! on disk; the folder its links are found in, the nearest one above it
//! holding `.obsidian`, else its own, with that folder's files by their
//! paths in it; the files its wiki links name there and the pictures it
//! shows, decoded. Nothing outside that folder is read: hidden folders
//! and links to files or folders are not followed, and every path found
//! is checked to lie inside it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

use oldbrainfix_core::obsidian;

use crate::md_doc::{self, Block, Doc, Picture, Target};

/// The largest Markdown file the viewer reads: the largest an Import
/// takes, so every file shown can be imported.
pub const MAX_FILE_BYTES: u64 = oldbrainfix_core::imports::MAX_FILE_BYTES as u64;

/// The largest picture the viewer shows.
pub const MAX_PICTURE_BYTES: u64 = 16 << 20;

/// The most files of the folder its links are found in that the viewer
/// looks at; a larger folder's others are not found.
pub const MAX_FILES: usize = 20_000;

/// How deep into the folder's folders the viewer looks.
pub const MAX_DEPTH: usize = 16;

/// The widest and highest a picture is shown, in pixels; a larger one is
/// made smaller as it is read.
pub const MAX_PICTURE_SIDE: u32 = 2048;

/// How often the file is looked at for a change.
pub const POLL: Duration = Duration::from_secs(1);

/// The folder the links of the Markdown file at `file` are found in: the
/// nearest one above it holding a folder `.obsidian`, else its own; as
/// the system names it, links worked out.
pub fn link_root(file: &Path) -> PathBuf {
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let own = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut at = Some(own.as_path());
    while let Some(folder) = at {
        if folder.join(".obsidian").is_dir() {
            return folder.to_path_buf();
        }
        at = folder.parent();
    }
    own
}

/// The files of `root` and of its folders, at most [`MAX_FILES`] and
/// [`MAX_DEPTH`] deep, by their paths inside it, folders separated by
/// `/`, the shallowest first and then by name: not hidden ones, none in a
/// hidden folder, and no link to a file or a folder.
pub fn index(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![(root.to_path_buf(), String::new(), 0)];
    let mut at = 0;
    while let Some((folder, prefix, depth)) = folders.get(at).cloned() {
        at += 1;
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if found.len() >= MAX_FILES {
                break;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if name.starts_with('.') || kind.is_symlink() {
                continue;
            }
            let path = format!("{prefix}{name}");
            if kind.is_dir() {
                if depth + 1 < MAX_DEPTH {
                    folders.push((entry.path(), format!("{path}/"), depth + 1));
                }
            } else if kind.is_file() {
                found.push(path);
            }
        }
    }
    // Shallowest first, as Obsidian finds a name nearest the top.
    found.sort_by(|a, b| {
        let depth = |p: &str| p.matches('/').count();
        (depth(a), a).cmp(&(depth(b), b))
    });
    found
}

/// The file at `path` inside `root`, while it lies there as the system
/// names it, links worked out; `None` for a path out of it.
pub fn inside(root: &Path, path: &str) -> Option<PathBuf> {
    let full = root.join(obsidian::normal(path)?).canonicalize().ok()?;
    full.starts_with(root).then_some(full)
}

/// `file`'s path inside `root`, folders separated by `/`.
pub fn path_in(root: &Path, file: &Path) -> Option<String> {
    let file = file.canonicalize().ok()?;
    let rest = file.strip_prefix(root).ok()?;
    let parts: Vec<String> = rest
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// When the file at `path` was last changed and how large it is, to tell
/// a change by; `None` while it is not there.
pub fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// The text of the bytes of a Markdown file: UTF-8 without the byte
/// order mark before it, any other byte as the character it is in
/// Windows-1252's Latin part; line breaks as `\n`.
pub fn text_of(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    };
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// A picture read for the viewer.
pub type Shown = Option<egui::ColorImage>;

/// The Markdown file the viewer shows, as read last.
#[derive(Clone)]
pub struct Read {
    /// The file and its name.
    pub file: PathBuf,
    pub name: String,
    /// The folder its links are found in.
    pub root: PathBuf,
    /// The file read, or why it could not be.
    pub doc: Result<Arc<Doc>, String>,
    /// The file each wiki link's name finds, by its name as written,
    /// none where it finds none.
    pub notes: Arc<HashMap<String, Option<PathBuf>>>,
    /// Each picture it shows, decoded, none where it was not found or
    /// read.
    pub pictures: Arc<HashMap<Picture, Shown>>,
    /// The other Markdown files of the folder it links to, each once, in
    /// the order it first does.
    pub linked: Vec<PathBuf>,
}

/// Reads the Markdown file at `file` for the viewer, with what it links
/// to and shows in `root`, the folder its links are found in, or in its
/// own [`link_root`] without one. A viewer keeps the folder of the file
/// it opened first while it follows links, so a link back from a file
/// in a folder further in still finds its file.
pub fn read_file(file: &Path, root: Option<&Path>) -> Read {
    let root = root.map_or_else(|| link_root(file), Path::to_path_buf);
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let name = file
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let bytes = match std::fs::metadata(&file) {
        Ok(meta) if meta.len() > MAX_FILE_BYTES => Err(format!(
            "{}: larger than {} KiB",
            file.display(),
            MAX_FILE_BYTES >> 10
        )),
        Ok(_) => std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display())),
        Err(e) => Err(format!("{}: {e}", file.display())),
    };
    let doc = bytes.map(|bytes| Arc::new(md_doc::read(&text_of(&bytes))));
    let mut read = Read {
        file: file.clone(),
        name,
        root: root.clone(),
        doc: doc.clone(),
        notes: Arc::default(),
        pictures: Arc::default(),
        linked: Vec::new(),
    };
    let Ok(doc) = doc else {
        return read;
    };
    let paths = index(&root);
    let from = path_in(&root, &file).unwrap_or_default();
    let mut notes = HashMap::new();
    let mut linked = Vec::new();
    for name in note_names(&doc) {
        let found = obsidian::note_target(&name, &paths)
            .or_else(|| {
                // A path as written in Markdown's own way, from the file's
                // folder.
                let near = format!("{}{name}", folder_of(&from));
                obsidian::note_target(&obsidian::normal(&near)?, &paths)
            })
            .and_then(|i| paths.get(i))
            .and_then(|p| inside(&root, p));
        if let Some(found) = &found
            && *found != file
            && !linked.contains(found)
        {
            linked.push(found.clone());
        }
        notes.insert(name, found);
    }
    let mut pictures = HashMap::new();
    for picture in doc_pictures(&doc) {
        let index = if picture.wiki {
            obsidian::file_target(&picture.target, &from, &paths)
        } else {
            obsidian::picture_target(&picture.target, &from, &paths)
        };
        let shown = index
            .and_then(|i| paths.get(i))
            .and_then(|p| inside(&root, p))
            .and_then(|p| decoded(&p));
        pictures.insert(picture, shown);
    }
    read.notes = Arc::new(notes);
    read.pictures = Arc::new(pictures);
    read.linked = linked;
    read
}

/// The folder of the path `path`, with its `/`, empty at the top.
fn folder_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(at) => path.get(..=at).unwrap_or_default(),
        None => "",
    }
}

/// The picture at `path`, made no larger than [`MAX_PICTURE_SIDE`];
/// `None` when it is too large to read or no picture.
pub fn decoded(path: &Path) -> Shown {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_PICTURE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let image = if image.width() > MAX_PICTURE_SIDE || image.height() > MAX_PICTURE_SIDE {
        image.thumbnail(MAX_PICTURE_SIDE, MAX_PICTURE_SIDE)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}

/// Each block of `blocks` and of the blocks inside them, first first.
pub fn walk<'a>(blocks: &'a [Block], out: &mut Vec<&'a Block>) {
    for block in blocks {
        out.push(block);
        match block {
            Block::Quote(inner) => walk(inner, out),
            Block::Callout { blocks, .. } => walk(blocks, out),
            Block::List { items, .. } => {
                for item in items {
                    walk(&item.blocks, out);
                }
            }
            _ => {}
        }
    }
}

/// Every block of `doc`, its footnotes' after the others.
pub fn all_blocks(doc: &Doc) -> Vec<&Block> {
    let mut out = Vec::new();
    walk(&doc.blocks, &mut out);
    for note in &doc.footnotes {
        walk(&note.blocks, &mut out);
    }
    out
}

/// The names of the other files `doc`'s links name, each once.
pub fn note_names(doc: &Doc) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for block in all_blocks(doc) {
        let texts = match block {
            Block::Heading { text, .. } | Block::Paragraph(text) => vec![text],
            Block::Callout { title, .. } => vec![title],
            _ => Vec::new(),
        };
        for text in texts {
            for run in &text.runs {
                if let Some(Target::Note { name, .. }) = &run.link
                    && !names.contains(name)
                {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}

/// The pictures `doc` shows, each once.
pub fn doc_pictures(doc: &Doc) -> Vec<Picture> {
    let mut found: Vec<Picture> = Vec::new();
    for block in all_blocks(doc) {
        if let Block::Picture(picture) = block
            && !found.contains(picture)
        {
            found.push(picture.clone());
        }
    }
    found
}

/// Reads the file the viewer shows on a thread of its own, and again each
/// time it changes or the viewer shows another, all in the folder the
/// links of the first are found in; each reading arrives
/// with [`Watch::arrived`], `wake` waking the window. It ends with the
/// viewer.
pub struct Watch {
    /// Another file to show.
    wanted: Sender<PathBuf>,
    arrives: Receiver<Read>,
}

impl Watch {
    /// Starts reading `file`.
    pub fn start(file: PathBuf, wake: Arc<dyn Fn() + Send + Sync>) -> Watch {
        let (wanted, asked) = std::sync::mpsc::channel::<PathBuf>();
        let (sent, arrives) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut file = file;
            let mut seen = None;
            let mut root: Option<PathBuf> = None;
            loop {
                let now = (file.clone(), stamp(&file));
                if seen.as_ref() != Some(&now) {
                    let read = read_file(&file, root.as_deref());
                    root.get_or_insert_with(|| read.root.clone());
                    if sent.send(read).is_err() {
                        return;
                    }
                    wake();
                    seen = Some(now);
                }
                match asked.recv_timeout(POLL) {
                    Ok(next) => file = next,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        });
        Watch { wanted, arrives }
    }

    /// Shows `file` in place of the one shown.
    pub fn show(&self, file: PathBuf) {
        drop(self.wanted.send(file));
    }

    /// The last reading arrived since asked last.
    pub fn arrived(&self) -> Option<Read> {
        self.arrives.try_iter().last()
    }
}

#[cfg(test)]
mod tests;
