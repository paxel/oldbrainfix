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
mod tests {
    use super::*;

    #[test]
    fn a_file_says_its_size_and_its_pages() {
        let t = L10n::new("en");
        let mut file = oldbrainfix_core::Attachment::read(
            "abc",
            r#"{"name":"Lease.pdf","pages":3,"size":2500000,"type":"application/pdf"}"#,
        );
        assert_eq!(facts(&t, &file), "2.5 MB · 3 pages");
        assert_eq!(title(&t, &file), "Lease.pdf");
        assert_eq!(icon(file.kind()), icons::FILE_PDF);
        file.pages = None;
        file.name = String::new();
        assert_eq!(facts(&t, &file), "2.5 MB");
        assert_eq!(title(&t, &file), "Attachment");
    }

    #[test]
    fn a_file_too_large_is_left_out_unread() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let small = dir.path().join("a.txt");
        std::fs::write(&small, b"milk")?;
        let large = dir.path().join("b.bin");
        std::fs::File::create(&large)?.set_len(MAX_FILE_BYTES + 1)?;
        let gone = dir.path().join("c.txt");
        let read = read_paths([
            ("a.txt".to_string(), small.as_path()),
            ("b.bin".to_string(), large.as_path()),
            ("c.txt".to_string(), gone.as_path()),
        ]);
        assert_eq!(read.files, [("a.txt".to_string(), b"milk".to_vec())]);
        assert!(read.too_large);
        assert_eq!(read.refused.len(), 2);
        assert!(read.refused[0].starts_with("b.bin: "));
        Ok(())
    }

    #[test]
    fn a_pdf_held_counts_its_pages_and_other_files_do_not() {
        let held = HeldFile::new("notes.txt".into(), b"milk".to_vec());
        assert_eq!(held.pages, None);
        assert_eq!(held.new_file().name, "notes.txt");
        let broken = HeldFile::new("Scan.PDF".into(), b"no pdf".to_vec());
        assert_eq!(broken.pages, None);
    }
}
