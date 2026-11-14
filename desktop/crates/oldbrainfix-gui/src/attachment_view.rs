//! The Attachments of a Note on the desktop (#267, ADR 0047): any file
//! dropped onto the window or picked, held in the quick note until the
//! Note is saved, a row each on the card and in the Note dialog with the
//! icon of its kind, its name, its size and its pages, opened with the
//! system's program for it, and its Attachment Text dimmed under it.

use std::collections::BTreeMap;
use std::path::Path;

use egui::{Id, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::file_text::{self, FileKind};
use oldbrainfix_core::notes::MAX_FILE_BYTES;
use oldbrainfix_core::{Attachment, NewFile, pdf};

use crate::icons;
use crate::l10n::L10n;
use crate::media_view::{self, Generated, Offers};
use crate::pack_view;
use crate::theme;

/// A file held for the Note being captured or attached to the Note
/// open: its name, its bytes as they are and, for a PDF, how many pages
/// it has, so the core knows a computer reads its text (Q645).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub pages: Option<u64>,
}

impl HeldFile {
    /// The file `bytes` named `name`, a PDF's pages counted.
    pub fn new(name: String, bytes: Vec<u8>) -> HeldFile {
        let pages = match file_text::kind_of(&name, "") {
            FileKind::Pdf => pdf::page_count(&bytes),
            _ => None,
        };
        HeldFile { name, bytes, pages }
    }

    /// It as the core attaches it, its media type the one its name's
    /// ending says.
    pub fn new_file(&self) -> NewFile<'_> {
        NewFile {
            name: &self.name,
            media_type: "",
            bytes: &self.bytes,
            pages: self.pages,
        }
    }
}

/// The files read from where they lie, by name, and those left out with
/// why: `too_large` when one was larger than a file may be (256 MiB).
#[derive(Debug, Default)]
pub struct ReadFiles {
    pub files: Vec<(String, Vec<u8>)>,
    pub refused: Vec<String>,
    pub too_large: bool,
}

/// The files at `paths` (each named `name` gives), read whole; one
/// larger than [`MAX_FILE_BYTES`], told by its size before it is read,
/// or one that cannot be read is left out.
pub fn read_paths<'p>(paths: impl IntoIterator<Item = (String, &'p Path)>) -> ReadFiles {
    let mut read = ReadFiles::default();
    for (name, path) in paths {
        let size = std::fs::metadata(path).map(|m| m.len());
        match size {
            Ok(size) if size > MAX_FILE_BYTES => {
                read.too_large = true;
                read.refused.push(format!("{name}: {size} bytes"));
            }
            _ => match std::fs::read(path) {
                Ok(bytes) => read.files.push((name, bytes)),
                Err(e) => read.refused.push(format!("{name}: {e}")),
            },
        }
    }
    read
}

/// The endings of files the system's programs only show or play (#267):
/// documents without macros, pictures, sound, video and a `.zip`, whose
/// opening lists what it holds. Any other, an `.exe`, a script, a
/// shortcut, a web page or a picture that runs script (`.svg`) among
/// them, could run a program and is asked about first.
const SAFE_ENDINGS: &[&str] = &[
    "pdf", "txt", "md", "markdown", "csv", "odt", "ods", "odp", "docx", "xlsx", "pptx", "epub",
    "jpg", "jpeg", "png", "gif", "webp", "heic", "heif", "avif", "bmp", "tif", "tiff", "mp3",
    "m4a", "aac", "wav", "ogg", "oga", "opus", "flac", "mp4", "m4v", "mov", "webm", "mkv", "avi",
    "zip",
];

/// The ending of the file `name` as its copy to open gets it: what
/// follows its last dot, in lower case, when that is one to eight ASCII
/// letters and digits; none otherwise, as for `a.exe.`, `a.exe ` or a
/// name without a dot.
pub fn ending(name: &str) -> Option<String> {
    let (_, after) = name.rsplit_once('.')?;
    let fits = (1..=8).contains(&after.len()) && after.bytes().all(|b| b.is_ascii_alphanumeric());
    fits.then(|| after.to_ascii_lowercase())
}

/// Whether the file `name` opens without a question: its ending is one
/// of the [`SAFE_ENDINGS`]. Its media type does not count, as the
/// system picks the program by the ending its copy gets.
pub fn opens_safely(name: &str) -> bool {
    ending(name).is_some_and(|e| SAFE_ENDINGS.contains(&e.as_str()))
}

/// The name of the copy of the file `hash` named `name` that the
/// system's program opens: made of the hash's first sixteen hex digits
/// and the file's [`ending`], so no part of a name another person gave
/// reaches the system but that ending.
pub fn copy_name(hash: &str, name: &str) -> String {
    let stem: String = hash
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(16)
        .collect();
    let stem = if stem.is_empty() { "file".into() } else { stem };
    match ending(name) {
        Some(ending) => format!("{stem}.{ending}"),
        None => stem,
    }
}

/// The icon of a file of `kind`, as on the phone: a PDF, a text
/// document, a table, plain text or Markdown, any other file.
pub fn icon(kind: FileKind) -> char {
    match kind {
        FileKind::Pdf => icons::FILE_PDF,
        FileKind::Docx | FileKind::Odt => icons::FILE_DOCUMENT,
        FileKind::Xlsx | FileKind::Ods => icons::FILE_TABLE,
        FileKind::Text => icons::FILE_TEXT,
        FileKind::Other => icons::FILE_OTHER,
    }
}

/// The words a file goes by: its name, else the word for an Attachment.
pub fn title<'a>(t: &'a L10n, file: &'a Attachment) -> &'a str {
    if file.name.is_empty() {
        t.attachment()
    } else {
        &file.name
    }
}

/// What a file's row says under its name: its size, and its pages when
/// they were counted.
pub fn facts(t: &L10n, file: &Attachment) -> String {
    let size = usize::try_from(file.size).unwrap_or(usize::MAX);
    let mut said = pack_view::size_text(t, size);
    if let Some(pages) = file.pages {
        said.push_str(" · ");
        said.push_str(&t.attachment_pages(i64::try_from(pages).unwrap_or(i64::MAX)));
    }
    said
}

/// The row of `file`: the icon of its kind, its name and, under it, its
/// size and pages; `large` names it as a card's title, for a Note made
/// from the file alone. Where it lies, for its click.
pub fn row(ui: &mut Ui, t: &L10n, file: &Attachment, large: bool) -> egui::Rect {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let accent = theme::tones(ui.visuals()).accent;
    ui.horizontal(|ui| {
        icons::show(
            ui,
            icon(file.kind()),
            if large { 32.0 } else { 26.0 },
            accent,
        );
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let size = if large { 18.0 } else { 15.0 };
            ui.add(
                egui::Label::new(RichText::new(title(t, file)).size(size).color(text)).truncate(),
            );
            ui.label(RichText::new(facts(t, file)).size(12.0).color(weak));
        });
    })
    .response
    .rect
}

/// The click over the row of `file` at `rect` that opens it, which a
/// screen reader hears as "Open …".
pub fn open_click(ui: &mut Ui, t: &L10n, file: &Attachment, rect: egui::Rect, id: Id) -> bool {
    let said = t.open_file(title(t, file));
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &said));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(said)
        .clicked()
}

/// The Attachment Text `text` of a file on a card, dimmed so it does not
/// read as typed text: its first three lines in the compact view, whole
/// under the words "Attachment Text" in the detailed one. Nothing while
/// it is not read, or found nothing.
pub fn card_text(ui: &mut Ui, t: &L10n, text: Option<&String>, compact: bool) {
    let Some(text) = text.filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let weak = ui.visuals().weak_text_color();
    if compact {
        let lines: Vec<&str> = text.lines().take(3).collect();
        let cut = text.lines().count() > 3;
        let mut shown = lines.join("\n");
        if cut {
            shown.push('…');
        }
        ui.label(RichText::new(shown).color(weak));
        return;
    }
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(RichText::new(t.attachment_text_label()).small().color(weak));
        ui.label(RichText::new(text).color(weak));
    });
}

/// The files of a Note or Section on a card, `(shown, removed, texts)`:
/// each file's row with its Attachment Text under it, then the texts of
/// those removed while their texts stay; `compact` as in the compact
/// view, `large` naming the one file as the card's title, for a Note
/// made from it alone. Each file shown comes back with where its row
/// lies, for its click.
pub fn card_files(
    ui: &mut Ui,
    t: &L10n,
    (files, removed, texts): (&[Attachment], &[Attachment], &BTreeMap<String, String>),
    compact: bool,
    large: bool,
) -> Vec<(Attachment, egui::Rect)> {
    let mut shown = Vec::new();
    for file in files {
        shown.push((file.clone(), row(ui, t, file, large)));
        card_text(ui, t, texts.get(&file.hash), compact);
    }
    for file in removed {
        card_text(ui, t, texts.get(&file.hash), compact);
    }
    shown
}

/// What was asked of a file in the Note dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// Opening it with the system's program.
    Open(String),
    /// Removing it and keeping its Attachment Text (ADR 0020).
    Remove(String),
    /// Something of its Attachment Text.
    Text(Generated),
}

/// The files of a Note or Section in the Note dialog, `(shown, removed,
/// texts)`: each file's row, which opens it, its Attachment Text under
/// it, dimmed, labelled and selecting to be copied (#255), the buttons
/// `offers` gives for it by hash, and "Remove Attachment, keep text";
/// then the texts of the files removed while their texts stay. What was
/// asked comes back.
pub fn editor_files(
    ui: &mut Ui,
    t: &L10n,
    (files, removed, texts): (&[Attachment], &[Attachment], &BTreeMap<String, String>),
    of: &str,
    offers: &dyn Fn(&str) -> Offers,
) -> Option<Asked> {
    let weak = ui.visuals().weak_text_color();
    let mut asked = None;
    let shown_text = |ui: &mut Ui, said: &String| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(t.attachment_text_label()).small().color(weak));
            crate::read_text::shown(ui, |ui| {
                crate::read_text::label(ui, egui::Label::new(RichText::new(said).color(weak)))
            });
        });
    };
    for file in files {
        let offered = offers(&file.hash);
        let rect = row(ui, t, file, false);
        if open_click(ui, t, file, rect, Id::new(("file", of, &file.hash))) {
            asked = Some(Asked::Open(file.hash.clone()));
        }
        let said = texts.get(&file.hash);
        if let Some(said) = said.filter(|s| !s.trim().is_empty()) {
            shown_text(ui, said);
        }
        if let Some(text) = media_view::generated_buttons(ui, t, &file.hash, said, offered, true) {
            asked = Some(Asked::Text(text));
        }
        if offered.remove_file && theme::destructive(ui, true, t.remove_file_keep_text()).clicked()
        {
            asked = Some(Asked::Remove(file.hash.clone()));
        }
    }
    for file in removed {
        let Some(said) = texts.get(&file.hash).filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        shown_text(ui, said);
        let offered = offers(&file.hash);
        if let Some(text) =
            media_view::generated_buttons(ui, t, &file.hash, Some(said), offered, true)
        {
            asked = Some(Asked::Text(text));
        }
    }
    asked
}

#[cfg(test)]
mod tests;
