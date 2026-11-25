//! The viewer of a Markdown file (obf-1056): a file opened from the
//! command line, with "Open with" or File ▸ Open ▸ Markdown file… shows in
//! a window of its own as a Note's reading view shows its Note (#322,
//! obf-1015): drawn as it reads, in a column in the middle, at the
//! Device's text size. Nothing of it goes into the store or syncs; it is
//! read again as it changes on disk ([`crate::md_source`]).
//!
//! Its wiki links open the file they name in the same window, at the
//! heading they name; a name not found shows dimmed. Back and forward
//! (the arrows, Alt+Left and Alt+Right, the mouse's back and forward
//! buttons) go through the files it showed. It has the find bar
//! (Ctrl+F, obf-1047), selects to be copied with Copy and Search in the
//! right-click menu (obf-1055), and Ctrl+Plus and Ctrl+Minus change the
//! text size (obf-1019). "Import as Note…" imports the file, asking
//! first whether the files it links to come along; ⋮ also opens it in
//! the system's own program for it. Inside the Flatpak, where only the
//! file is shared, a line under its bar, "Open the folder to follow its
//! links", asks for the folder its links are found in (Q13).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{
    Align, Color32, FontId, FontSelection, Frame, Id, Key, Margin, Modifiers, PointerButton,
    RichText, Sense, Stroke, TextFormat, TextWrapMode, TextureHandle, Ui, Vec2, ViewportId,
    WidgetInfo, WidgetText, WidgetType,
};
use oldbrainfix_core::findings::{self, Finding, FindingPart};
use oldbrainfix_core::markdown::table::{self, TableCopy};
use oldbrainfix_core::text_format;

use crate::app::{READ_EDGE, READ_SIZE, icon_button};
use crate::design;
use crate::find_bar::{self, FindBar};
use crate::find_marks::{self, Current};
use crate::l10n::L10n;
use crate::md_doc::{Block, Doc, Inline, Picture, Run, Target};
use crate::md_source::{Read, Watch};
use crate::note_window::{self, Scrolled};
use crate::text_marks_view::{self, BoxLook, CODE_SCALE, LINE_HEIGHT, TableAct, TableRoom};
use crate::{icons, read_text, text_menu, theme};

/// The sizes of the six levels of heading, in parts of the text's size.
pub const HEADINGS: [f32; 6] = [1.6, 1.4, 1.25, 1.1, 1.0, 0.95];

/// The viewport of the `serial`th viewer opened while the app runs.
pub fn viewport_id(serial: u64) -> ViewportId {
    ViewportId::from_hash_of(("markdown viewer", serial))
}

/// What the viewer asked of the main window.
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    /// The text one step larger or smaller (obf-1019).
    TextStep(bool),
    /// Import as Note: the file, the folder its links are found in and
    /// the other files to import with it.
    Import {
        file: PathBuf,
        root: PathBuf,
        linked: Vec<PathBuf>,
    },
    /// Open in the default app.
    OpenElsewhere(PathBuf),
    /// The find bar's Search all Notes, with its words.
    SearchAll(String),
    /// A table's ⋮ Copy (obf-1050): the table on the clipboard as its
    /// HTML beside its Markdown.
    Copy(TableCopy),
    /// Open the folder to follow its links (Q13): the folder its links
    /// are found in, asked through the folder portal for the viewer of
    /// this viewport.
    OpenFolder(ViewportId),
}

/// Where the view is to scroll to once drawn: a heading, by its words or
/// their slug, or a footnote, by its label.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Goal {
    Heading(String),
    Footnote(String),
}

/// A text of the file the find bar finds in, in the order they show: its
/// text as drawn and its format, plain for the viewer's own texts, the
/// app's formatted text for a table's or a code block's source; and the
/// part of the file it is drawn in ([`units`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub text: String,
    pub format: &'static str,
    pub unit: usize,
}

/// The parts of `doc` drawn one by one, each only near the view: its
/// properties first, then each of its blocks, then its footnotes; how
/// many characters and pictures each holds, to guess its height before
/// it was drawn.
pub fn units(doc: &Doc) -> Vec<(usize, usize)> {
    let mut out = vec![(0, 0); doc.blocks.len() + 2];
    for piece in pieces(doc) {
        if let Some(unit) = out.get_mut(piece.unit) {
            unit.0 += piece.text.len();
        }
    }
    let pictures = |blocks: &[&Block]| {
        blocks
            .iter()
            .filter(|b| matches!(b, Block::Picture(_)))
            .count()
    };
    for (i, block) in doc.blocks.iter().enumerate() {
        let mut inner = Vec::new();
        crate::md_source::walk(std::slice::from_ref(block), &mut inner);
        if let Some(unit) = out.get_mut(i + 1) {
            unit.1 = pictures(&inner);
        }
    }
    out
}

/// The texts of `doc` the find bar finds in, top first: its properties,
/// its blocks and its footnotes.
pub fn pieces(doc: &Doc) -> Vec<Piece> {
    fn plain(out: &mut Vec<Piece>, text: &str, unit: usize) {
        out.push(Piece {
            text: text.to_string(),
            format: text_format::PLAIN,
            unit,
        });
    }
    fn walk(out: &mut Vec<Piece>, blocks: &[Block], unit: usize) {
        for block in blocks {
            match block {
                Block::Heading { text, .. } | Block::Paragraph(text) => {
                    plain(out, &text.text, unit);
                }
                Block::Quote(inner) => walk(out, inner, unit),
                Block::Callout { title, blocks, .. } => {
                    plain(out, &title.text, unit);
                    walk(out, blocks, unit);
                }
                Block::List { items, .. } => {
                    for item in items {
                        walk(out, &item.blocks, unit);
                    }
                }
                Block::Code(source) | Block::Table(source) => out.push(Piece {
                    text: source.clone(),
                    format: text_format::MARKDOWN,
                    unit,
                }),
                Block::Rule | Block::Picture(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    for (key, value) in &doc.properties {
        plain(&mut out, key, 0);
        plain(&mut out, value, 0);
    }
    for (i, block) in doc.blocks.iter().enumerate() {
        walk(&mut out, std::slice::from_ref(block), i + 1);
    }
    let last = doc.blocks.len() + 1;
    for note in &doc.footnotes {
        walk(&mut out, &note.blocks, last);
    }
    out
}

/// The part of `doc` ([`units`]) that holds the heading or footnote
/// `goal`.
fn goal_unit(doc: &Doc, goal: &Goal) -> Option<usize> {
    match goal {
        Goal::Footnote(_) => Some(doc.blocks.len() + 1),
        Goal::Heading(wanted) => doc
            .blocks
            .iter()
            .position(|block| {
                let mut inner = Vec::new();
                crate::md_source::walk(std::slice::from_ref(block), &mut inner);
                inner.iter().any(|b| match b {
                    Block::Heading { text, .. } => heading_is(&text.text, wanted),
                    _ => false,
                })
            })
            .map(|i| i + 1),
    }
}

/// Whether a heading of `words` is the one `wanted` names, by its words
/// in any case or by its slug.
fn heading_is(words: &str, wanted: &str) -> bool {
    words.trim().to_lowercase() == wanted.trim().to_lowercase()
        || crate::md_doc::slug(words) == crate::md_doc::slug(wanted)
}

/// A Markdown file in a window of its own.
pub struct Viewer {
    pub viewport: ViewportId,
    watch: Watch,
    /// The file shown, and as it was read last.
    file: PathBuf,
    read: Option<Read>,
    pieces: Arc<Vec<Piece>>,
    /// What the pieces hash to, for the find bar to tell a change by.
    pieces_key: u64,
    /// The size of each part of the file ([`units`]) and each one's
    /// height as drawn last, while drawn at the same width, text size and
    /// reading; the part with the find bar's current place to show.
    unit_sizes: Vec<(usize, usize)>,
    heights: Vec<Option<f32>>,
    heights_key: (u32, u32, u64),
    readings: u64,
    reveal_unit: Option<usize>,
    /// Whether nothing wakes the window as the file is read, so it looks
    /// again itself, as in tests.
    polls: bool,
    textures: HashMap<Picture, TextureHandle>,
    /// The files shown before this one and after it.
    back: Vec<PathBuf>,
    ahead: Vec<PathBuf>,
    goal: Option<Goal>,
    find: Option<FindBar>,
    scrolled: Scrolled,
    scroll_to: Option<f32>,
    /// Whether "Also import the files it links to?" is asked.
    asking: bool,
    /// Whether it is to come forward.
    pub forward: bool,
    /// Its size as drawn last, in points.
    pub size: Vec2,
    /// Whether the system's cross was clicked.
    closed: bool,
    /// A table copied from its ⋮ in this frame (obf-1050).
    copied: Option<TableCopy>,
    /// Whether it runs inside the Flatpak, where it asks for the folder
    /// its links are found in, and whether one was granted since (Q13).
    asks_folder: bool,
    granting: bool,
}

impl Viewer {
    /// A viewer of the Markdown file `file`, its viewport the `serial`th's,
    /// `size` large; `wake` wakes the window as the file is read, and
    /// without one the window looks again every so often.
    pub fn open(
        file: PathBuf,
        serial: u64,
        size: Vec2,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Viewer {
        Viewer::opened(file, serial, size, wake, None)
    }

    /// [`Viewer::open`] inside the Flatpak (Q13): its links are found in
    /// the first of the folders `granted` while the app runs that holds
    /// a copy of the file; without one it asks for the folder.
    pub fn open_granted(
        file: PathBuf,
        serial: u64,
        size: Vec2,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
        granted: Vec<PathBuf>,
    ) -> Viewer {
        Viewer::opened(file, serial, size, wake, Some(granted))
    }

    fn opened(
        file: PathBuf,
        serial: u64,
        size: Vec2,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
        granted: Option<Vec<PathBuf>>,
    ) -> Viewer {
        let polls = wake.is_none();
        let wake = wake.unwrap_or_else(|| Arc::new(|| {}));
        let asks_folder = granted.is_some();
        let watch = match granted {
            Some(granted) => Watch::start_granted(file.clone(), granted, wake),
            None => Watch::start(file.clone(), wake),
        };
        Viewer {
            viewport: viewport_id(serial),
            watch,
            file,
            read: None,
            pieces: Arc::default(),
            pieces_key: 0,
            unit_sizes: Vec::new(),
            heights: Vec::new(),
            heights_key: (0, 0, 0),
            readings: 0,
            reveal_unit: None,
            polls,
            textures: HashMap::new(),
            back: Vec::new(),
            ahead: Vec::new(),
            goal: None,
            find: None,
            scrolled: Scrolled::default(),
            scroll_to: None,
            asking: false,
            forward: false,
            size,
            closed: false,
            copied: None,
            asks_folder,
            granting: false,
        }
    }

    /// Finds its links in `folder`, granted through the folder portal
    /// (Q13), and reads the file again.
    pub fn grant(&mut self, folder: PathBuf) {
        self.granting = true;
        self.watch.grant(folder);
    }

    /// Whether it shows "Open the folder to follow its links": inside the
    /// Flatpak, for a file with links or pictures, until a folder is
    /// granted.
    pub fn asks_for_folder(&self) -> bool {
        self.asks_folder
            && !self.granting
            && self
                .read
                .as_ref()
                .is_some_and(|r| !r.granted && (!r.notes.is_empty() || !r.pictures.is_empty()))
    }

    /// The file it shows.
    pub fn file(&self) -> &std::path::Path {
        &self.file
    }

    /// Whether it shows `file`.
    pub fn shows(&self, file: &std::path::Path) -> bool {
        let same = |a: &std::path::Path| a.canonicalize().ok();
        self.file == file || (same(&self.file).is_some() && same(&self.file) == same(file))
    }

    /// Where its view was scrolled as it was drawn last, for tests.
    pub fn scrolled(&self) -> Scrolled {
        self.scrolled
    }

    /// Whether the file shown has been read, for tests.
    pub fn loaded(&self) -> bool {
        self.read.as_ref().is_some_and(|r| r.file == self.file)
    }

    /// The window's title: the file's name.
    pub fn title(&self, app: &str) -> String {
        match self.file.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => app.to_string(),
        }
    }

    /// Takes in a reading of the file, its pictures as textures.
    fn take_read(&mut self, ctx: &egui::Context) {
        let Some(read) = self.watch.arrived() else {
            return;
        };
        // One of the file shown before, read as it was left, is dropped.
        if !self.shows(&read.file) {
            return;
        }
        self.textures.clear();
        for (picture, shown) in read.pictures.iter() {
            if let Some(image) = shown {
                let name = format!("markdown picture {}", picture.target);
                let texture = ctx.load_texture(name, image.clone(), egui::TextureOptions::LINEAR);
                self.textures.insert(picture.clone(), texture);
            }
        }
        self.pieces = Arc::new(match &read.doc {
            Ok(doc) => pieces(doc),
            Err(_) => Vec::new(),
        });
        self.unit_sizes = match &read.doc {
            Ok(doc) => units(doc),
            Err(_) => Vec::new(),
        };
        self.readings += 1;
        self.pieces_key = self.pieces.iter().fold(0u64, |h, p| {
            h.rotate_left(5) ^ find_marks::text_key(&p.text)
        });
        self.file = read.file.clone();
        // Granted or not, the folder asked for has been looked at.
        self.granting = false;
        self.read = Some(read);
        ctx.request_repaint();
    }

    /// Shows `file` in place of the one shown, at `goal` in it; what was
    /// shown goes back, what lay ahead is forgotten.
    fn go(&mut self, file: PathBuf, goal: Option<Goal>) {
        if self.shows(&file) {
            self.goal = goal;
            return;
        }
        self.back
            .push(std::mem::replace(&mut self.file, file.clone()));
        self.ahead.clear();
        self.goal = goal;
        self.find = None;
        self.scroll_to = Some(0.0);
        self.watch.show(file);
    }

    /// Back (`forward` false) or forward through the files shown.
    fn step(&mut self, forward: bool) {
        let (from, to) = if forward {
            (&mut self.ahead, &mut self.back)
        } else {
            (&mut self.back, &mut self.ahead)
        };
        let Some(file) = from.pop() else {
            return;
        };
        to.push(std::mem::replace(&mut self.file, file.clone()));
        self.goal = None;
        self.find = None;
        self.scroll_to = Some(0.0);
        self.watch.show(file);
    }

    /// A frame of its window: the bar, the find bar while open, the file
    /// as it reads; and the question of the Import over it. Whether it
    /// stays open, with what it asked of the main window. `importing`
    /// says an Import runs, which greys Import as Note.
    pub fn frame(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        importing: bool,
        native: bool,
    ) -> (bool, Vec<Asked>) {
        let ctx = ui.ctx().clone();
        self.take_read(&ctx);
        let mut asked = Vec::new();
        if native && ui.input(|i| i.viewport().close_requested()) {
            self.closed = true;
        }
        // The reading wakes the window; without a way to, it looks again.
        if self.polls {
            ctx.request_repaint_after(crate::md_source::POLL);
        }
        self.keys(ui, &mut asked);
        let tones = theme::tones(ui.visuals());
        Frame::NONE
            .fill(tones.background)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.bar(ui, t, importing, &mut asked);
                if self.asks_for_folder() {
                    self.folder_line(ui, t, &mut asked);
                }
                let marks = self.find_row(ui, t, &mut asked);
                let before = find_marks::set(&ctx, marks);
                let searched = text_menu::searches(&ctx, true);
                text_menu::finds_in_file(&ctx);
                self.content(ui, t);
                text_menu::searches(&ctx, searched);
                find_marks::set(&ctx, before);
            });
        if self.asking {
            self.ask_import(&ctx, t, &mut asked);
        }
        read_text::menu(&ctx, t);
        asked.extend(self.copied.take().map(Asked::Copy));
        // The cells of a table picked and copied (obf-1054).
        asked.extend(crate::cell_pick::take_copy(&ctx).map(Asked::Copy));
        if let Some(words) = text_menu::take_searched(&ctx) {
            self.open_find(Some(&words));
        }
        if native {
            let maximized = ui.input(|i| {
                let viewport = i.viewport();
                viewport.maximized == Some(true) || viewport.fullscreen == Some(true)
            });
            if !maximized {
                self.size = ctx.viewport_rect().size();
            }
        }
        (!self.closed, asked)
    }

    /// Opens the find bar, or puts the keyboard back into it, with
    /// `words` in its field when there are any.
    fn open_find(&mut self, words: Option<&str>) {
        match (&mut self.find, words) {
            (Some(bar), None) => bar.focus(),
            (_, words) => self.find = Some(FindBar::new(words.unwrap_or_default(), true)),
        }
    }

    /// The viewer's keys, while no dialog lies over it.
    fn keys(&mut self, ui: &mut Ui, asked: &mut Vec<Asked>) {
        if self.asking || egui::Popup::is_any_open(ui.ctx()) {
            return;
        }
        let typing = ui.ctx().text_edit_focused();
        let (find, close, larger, smaller, back, forward) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::F),
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.consume_key(Modifiers::COMMAND, Key::Plus)
                    || i.consume_key(Modifiers::COMMAND, Key::Equals),
                i.consume_key(Modifiers::COMMAND, Key::Minus),
                i.pointer.button_pressed(PointerButton::Extra1)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowLeft)),
                i.pointer.button_pressed(PointerButton::Extra2)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowRight)),
            )
        });
        if find {
            self.open_find(None);
        }
        if close {
            self.find = None;
        }
        if larger {
            asked.push(Asked::TextStep(true));
        }
        if smaller {
            asked.push(Asked::TextStep(false));
        }
        if back {
            self.step(false);
        }
        if forward {
            self.step(true);
        }
        let free = ui.memory(|m| m.focused().is_none());
        if let Some(key) = ui.input_mut(|i| note_window::scroll_key(i, free)) {
            self.scroll_to = Some(self.scrolled.to(key));
            ui.ctx().request_repaint();
        }
    }

    /// The bar over the file: back and forward, the file's name, Import as
    /// Note… and ⋮ with Find, Open in the default app and Import as
    /// Note….
    fn bar(&mut self, ui: &mut Ui, t: &L10n, importing: bool, asked: &mut Vec<Asked>) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let readable = self.read.as_ref().is_some_and(|r| r.doc.is_ok());
        let mut import = false;
        ui.horizontal(|ui| {
            for (forward, icon, label) in [
                (false, icons::ARROW_BACK, t.nav_back()),
                (true, icons::ARROW_FORWARD, t.nav_forward()),
            ] {
                let some = if forward {
                    !self.ahead.is_empty()
                } else {
                    !self.back.is_empty()
                };
                let shade = if some { text } else { weak };
                let button = ui
                    .add_enabled_ui(some, |ui| icon_button(ui, icon, label, shade))
                    .inner;
                if button.clicked() {
                    self.step(forward);
                }
            }
            ui.add_space(8.0);
            let name = self.title("");
            ui.label(RichText::new(name).strong().color(text));
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let more = icon_button(ui, icons::MORE_VERT, t.desk_markdown_more(), text);
                egui::Popup::menu(&more).show(|ui| {
                    let entry = |ui: &mut Ui, label: &str, enabled: bool| {
                        let button = egui::Button::new(RichText::new(label).color(text));
                        ui.add_enabled(enabled, button).clicked()
                    };
                    if entry(ui, t.menu_find(), readable) {
                        self.open_find(None);
                    }
                    if entry(ui, t.open_in_other_app(), true) {
                        asked.push(Asked::OpenElsewhere(self.file.clone()));
                    }
                    if entry(ui, t.desk_markdown_import(), readable && !importing) {
                        import = true;
                    }
                });
                if importing {
                    ui.label(RichText::new(t.desk_markdown_importing()).color(weak));
                } else {
                    let button =
                        egui::Button::new(RichText::new(t.desk_markdown_import()).color(text));
                    if ui.add_enabled(readable, button).clicked() {
                        import = true;
                    }
                }
            });
        });
        if import {
            match self.read.as_ref() {
                Some(read) if !read.linked.is_empty() => self.asking = true,
                Some(read) => asked.push(Asked::Import {
                    file: read.to_import(),
                    root: read.root.clone(),
                    linked: Vec::new(),
                }),
                None => {}
            }
        }
    }

    /// "Open the folder to follow its links", under the bar inside the
    /// Flatpak (Q13).
    fn folder_line(&mut self, ui: &mut Ui, t: &L10n, asked: &mut Vec<Asked>) {
        let text = ui.visuals().text_color();
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let button =
                egui::Button::new(RichText::new(t.desk_markdown_open_folder()).color(text));
            if ui.add(button).clicked() {
                asked.push(Asked::OpenFolder(self.viewport));
            }
        });
        ui.add_space(4.0);
    }

    /// The find bar while it is open, finding in the file's texts; the
    /// marks it sets for what is drawn.
    fn find_row(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        asked: &mut Vec<Asked>,
    ) -> Option<Arc<find_marks::FindMarks>> {
        self.reveal_unit = None;
        let bar = self.find.as_mut()?;
        let pieces = self.pieces.clone();
        bar.find(self.pieces_key, |query| {
            pieces
                .iter()
                .enumerate()
                .flat_map(|(i, p)| {
                    findings::in_text_of(
                        &p.text,
                        p.format,
                        query,
                        FindingPart::Text,
                        &i.to_string(),
                        None,
                    )
                })
                .collect()
        });
        let said = (t.desk_markdown_find_hint(), t.desk_markdown_find_none());
        match bar.show_with(ui, t, said) {
            Some(find_bar::Asked::Close) => {
                self.find = None;
                return None;
            }
            Some(find_bar::Asked::SearchAll) => {
                asked.push(Asked::SearchAll(bar.query().to_string()));
            }
            None => {}
        }
        ui.add_space(4.0);
        let drawn_at = |f: &Finding| {
            let piece = f.key.parse::<usize>().ok().and_then(|i| pieces.get(i))?;
            Some(Current {
                text: find_marks::text_key(&piece.text),
                start: f.start,
            })
        };
        let marks = bar.marks(ui.ctx(), drawn_at);
        // The part holding the current place is drawn, to come into view.
        self.reveal_unit = bar
            .current()
            .filter(|_| marks.reveal)
            .and_then(|f| f.key.parse::<usize>().ok())
            .and_then(|i| pieces.get(i))
            .map(|p| p.unit);
        Some(marks)
    }

    /// The file as it reads, in a column in the middle, or why it cannot
    /// be read.
    fn content(&mut self, ui: &mut Ui, t: &L10n) {
        let Some(read) = self.read.clone() else {
            return;
        };
        let doc = match &read.doc {
            Ok(doc) => doc.clone(),
            Err(details) => {
                let text = ui.visuals().text_color();
                let weak = ui.visuals().weak_text_color();
                ui.add_space(16.0);
                ui.label(RichText::new(t.desk_markdown_unreadable()).color(text));
                ui.label(RichText::new(details).small().color(weak));
                return;
            }
        };
        let column = note_window::measure(theme::content(ui, READ_SIZE));
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt(("markdown", self.viewport))
            .auto_shrink([false, false]);
        if let Some(to) = self.scroll_to.take() {
            scroll = scroll.vertical_scroll_offset(to);
        }
        // A heading or footnote to scroll to is looked for in the file it
        // is in, once that is read; not in the one shown before it.
        let loaded = read.file == self.file;
        // The part to draw wherever it lies: the one with the heading or
        // footnote gone to, else the one with the find's current place.
        let force = match &self.goal {
            Some(goal) if loaded => goal_unit(&doc, goal),
            _ => None,
        }
        .or(self.reveal_unit);
        let size = theme::content(ui, READ_SIZE);
        let width = ui.available_width().min(column);
        let key = (width.to_bits(), size.to_bits(), self.readings);
        if self.heights_key != key || self.heights.len() != self.unit_sizes.len() {
            self.heights = vec![None; self.unit_sizes.len()];
            self.heights_key = key;
        }
        let mut waiting = None;
        let goal = if loaded { &mut self.goal } else { &mut waiting };
        let mut cull = Cull {
            view: egui::Rangef::new(0.0, 0.0),
            sizes: &self.unit_sizes,
            heights: &mut self.heights,
            force,
            width,
        };
        let mut draw = Draw {
            t,
            read: &read,
            textures: &self.textures,
            size: theme::content(ui, READ_SIZE),
            dark: ui.visuals().dark_mode,
            wide: egui::Rangef::new(0.0, 0.0),
            clicked: None,
            copied: None,
            goal,
        };
        let output = scroll.show_viewport(ui, |ui, viewport| {
            let room = ui.available_width();
            let width = room.min(column);
            draw.wide = ui.max_rect().x_range().shrink(READ_EDGE);
            cull.view = viewport.y_range();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(((room - width) / 2.0).max(0.0));
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.add_space(8.0);
                    read_text::shown(ui, |ui| {
                        ui.style_mut().interaction.selectable_labels = true;
                        draw.doc(ui, &doc, &mut cull);
                    });
                    ui.add_space(24.0);
                });
            });
        });
        let clicked = draw.clicked.take();
        let copied = draw.copied.take();
        if copied.is_some() {
            self.copied = copied;
        }
        // One the file does not hold is dropped once it was drawn.
        if loaded {
            self.goal = None;
        }
        self.scrolled = Scrolled {
            offset: output.state.offset.y,
            page: output.inner_rect.height(),
            height: output.content_size.y,
        };
        if clicked.is_some() {
            ui.ctx().request_repaint();
        }
        match clicked {
            Some(Target::Note { name, heading }) => {
                if let Some(Some(file)) = read.notes.get(&name) {
                    self.go(file.clone(), heading.map(Goal::Heading));
                }
            }
            Some(Target::Heading(heading)) => self.goal = Some(Goal::Heading(heading)),
            Some(Target::Footnote(label)) => self.goal = Some(Goal::Footnote(label)),
            Some(Target::Web(url)) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
            Some(Target::Nowhere) | None => {}
        }
    }

    /// "Also import the N files it links to?", with Import them too, Only
    /// this file and Cancel.
    fn ask_import(&mut self, ctx: &egui::Context, t: &L10n, asked: &mut Vec<Asked>) {
        let Some(read) = self.read.clone() else {
            self.asking = false;
            return;
        };
        let count = read.linked.len();
        let modal = egui::Modal::new(Id::new(("markdown import", self.viewport))).show(ctx, |ui| {
            ui.set_max_width(420.0);
            let text = ui.visuals().text_color();
            let weak = ui.visuals().weak_text_color();
            ui.label(
                RichText::new(t.desk_markdown_import_title())
                    .size(18.0)
                    .strong()
                    .color(text),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(t.desk_markdown_ask_links(count as i64)).color(text));
            ui.label(
                RichText::new(t.desk_markdown_ask_detail())
                    .small()
                    .color(weak),
            );
            ui.add_space(12.0);
            let mut picked = None;
            ui.horizontal(|ui| {
                if theme::primary(ui, true, t.desk_markdown_with_links()).clicked() {
                    picked = Some(true);
                }
                let alone = egui::Button::new(RichText::new(t.desk_markdown_alone()).color(text));
                if ui.add(alone).clicked() {
                    picked = Some(false);
                }
                if design::row_button(ui, t.cancel()).clicked() {
                    self.asking = false;
                }
            });
            picked
        });
        if let Some(with_links) = modal.inner {
            self.asking = false;
            asked.push(Asked::Import {
                file: read.to_import(),
                root: read.root.clone(),
                linked: if with_links {
                    read.linked.clone()
                } else {
                    Vec::new()
                },
            });
        }
        let back = ctx.input_mut(|i| {
            i.pointer.button_pressed(PointerButton::Extra1)
                || i.consume_key(Modifiers::ALT, Key::ArrowLeft)
        });
        if modal.should_close() || back {
            self.asking = false;
        }
    }
}

/// Which parts of a file ([`units`]) are drawn in a frame: those near
/// `view`, the part of it the scroll area shows, by their heights as
/// drawn last, guessed by their `sizes` before; and the part `force`
/// wherever it lies. Each part drawn has its height kept.
struct Cull<'a> {
    view: egui::Rangef,
    sizes: &'a [(usize, usize)],
    heights: &'a mut Vec<Option<f32>>,
    force: Option<usize>,
    /// The column's width, which a guess counts lines by.
    width: f32,
}

impl Cull<'_> {
    /// How high the part `unit` is, as drawn last, else as guessed for
    /// text at `size`: its characters in lines of the column's width,
    /// a picture as a third of a page.
    fn height(&self, unit: usize, size: f32) -> f32 {
        if let Some(Some(height)) = self.heights.get(unit) {
            return *height;
        }
        let (chars, pictures) = self.sizes.get(unit).copied().unwrap_or_default();
        if chars == 0 && pictures == 0 {
            return 0.0;
        }
        let per_line = (self.width / (size * 0.5)).max(20.0);
        let lines = (chars as f32 / per_line).ceil().max(1.0);
        lines * size * LINE_HEIGHT + size + pictures as f32 * 300.0
    }

    /// Whether the part `unit`, `height` high from `top` down the content,
    /// is drawn: near the view, a page above and below it, or forced.
    fn draws(&self, unit: usize, top: f32, height: f32) -> bool {
        let page = self.view.span().max(200.0);
        self.force == Some(unit)
            || (top + height >= self.view.min - page && top <= self.view.max + page)
    }
}

/// What draws a file in one frame, and what was clicked in it.
struct Draw<'a> {
    t: &'a L10n,
    read: &'a Read,
    textures: &'a HashMap<Picture, TextureHandle>,
    /// The size of its text, the Device's text size counted in.
    size: f32,
    dark: bool,
    /// How wide a table may be, beyond the column.
    wide: egui::Rangef,
    clicked: Option<Target>,
    /// A table copied from its ⋮ (obf-1050).
    copied: Option<TableCopy>,
    goal: &'a mut Option<Goal>,
}

impl Draw<'_> {
    /// Draws the parts of `doc` near the view, room in place of the
    /// others.
    fn doc(&mut self, ui: &mut Ui, doc: &Doc, cull: &mut Cull) {
        let text = ui.visuals().text_color();
        ui.spacing_mut().item_spacing.y = 0.0;
        let origin = ui.cursor().top();
        let last = doc.blocks.len() + 1;
        for unit in 0..=last {
            let top = ui.cursor().top() - origin;
            let guess = cull.height(unit, self.size);
            if !cull.draws(unit, top, guess) {
                ui.add_space(guess);
                continue;
            }
            match unit {
                0 if !doc.properties.is_empty() => {
                    self.properties(ui, &doc.properties);
                    ui.add_space(self.size);
                }
                0 => {}
                unit if unit == last => self.footnotes(ui, doc, text),
                unit => {
                    if let Some(block) = doc.blocks.get(unit - 1) {
                        if unit > 1 {
                            let gap = match block {
                                Block::Heading { .. } => 0.9,
                                _ => 0.5,
                            };
                            ui.add_space(self.size * gap);
                        }
                        self.block(ui, block, text);
                    }
                }
            }
            let drawn = ui.cursor().top() - origin - top;
            if let Some(height) = cull.heights.get_mut(unit) {
                *height = Some(drawn);
            }
        }
    }

    /// The footnotes, under a line, each with its number.
    fn footnotes(&mut self, ui: &mut Ui, doc: &Doc, text: Color32) {
        if doc.footnotes.is_empty() {
            return;
        }
        ui.add_space(self.size);
        ui.separator();
        ui.add_space(self.size * 0.5);
        let small = self.size * 0.9;
        for note in &doc.footnotes {
            let row = ui.horizontal_top(|ui| {
                let number = format!("{}.", note.number);
                let weak = ui.visuals().weak_text_color();
                marker(ui, &number, small, weak, small * 2.0);
                ui.vertical(|ui| {
                    let size = std::mem::replace(&mut self.size, small);
                    self.blocks(ui, &note.blocks, text);
                    self.size = size;
                });
            });
            if self.goal.as_ref() == Some(&Goal::Footnote(note.label.clone())) {
                ui.scroll_to_rect(row.response.rect, Some(Align::Center));
                *self.goal = None;
            }
            ui.add_space(self.size * 0.3);
        }
    }

    /// The front matter's properties, a small table at the top.
    fn properties(&mut self, ui: &mut Ui, properties: &[(String, String)]) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let small = self.size * 0.85;
        Frame::NONE
            .fill(theme::code_tint(text))
            .corner_radius(6)
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                egui::Grid::new("markdown properties")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for (key, value) in properties {
                            let style = crate::md_doc::Style::default();
                            self.inline(ui, &Inline::plain(key, style), (small, weak, false));
                            self.inline(ui, &Inline::plain(value, style), (small, text, false));
                            ui.end_row();
                        }
                    });
            });
    }

    fn blocks(&mut self, ui: &mut Ui, blocks: &[Block], colour: Color32) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                let gap = match block {
                    Block::Heading { .. } => 0.9,
                    _ => 0.5,
                };
                ui.add_space(self.size * gap);
            }
            self.block(ui, block, colour);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, colour: Color32) {
        let weak = ui.visuals().weak_text_color();
        match block {
            Block::Heading { level, text } => {
                let factor = HEADINGS
                    .get(level.saturating_sub(1))
                    .copied()
                    .unwrap_or(1.0);
                let colour = if *level >= 6 { weak } else { colour };
                let response = self.inline(ui, text, (self.size * factor, colour, true));
                let reached = match self.goal.as_ref() {
                    Some(Goal::Heading(wanted)) => heading_is(&text.text, wanted),
                    _ => false,
                };
                if reached {
                    ui.scroll_to_rect(response.rect, Some(Align::TOP));
                    *self.goal = None;
                }
            }
            Block::Paragraph(text) => {
                self.inline(ui, text, (self.size, colour, false));
            }
            Block::Quote(inner) => {
                let accent = ui.visuals().hyperlink_color;
                let row = ui.horizontal_top(|ui| {
                    ui.add_space(self.size * 0.8);
                    ui.vertical(|ui| self.blocks(ui, inner, weak));
                });
                let rect = row.response.rect;
                ui.painter()
                    .vline(rect.left() + 3.0, rect.y_range(), Stroke::new(3.0, accent));
            }
            Block::Callout {
                kind,
                title,
                blocks,
            } => {
                let tint = callout_colour(kind);
                let shown = Frame::NONE
                    .fill(tint.gamma_multiply(0.14))
                    .corner_radius(6)
                    .inner_margin(Margin {
                        left: 14,
                        right: 10,
                        top: 8,
                        bottom: 8,
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        self.inline(ui, title, (self.size, tint, true));
                        if !blocks.is_empty() {
                            ui.add_space(self.size * 0.4);
                            self.blocks(ui, blocks, colour);
                        }
                    });
                let rect = shown.response.rect;
                ui.painter()
                    .vline(rect.left() + 2.0, rect.y_range(), Stroke::new(3.0, tint));
            }
            Block::List { start, items } => {
                for (n, item) in items.iter().enumerate() {
                    if n > 0 {
                        ui.add_space(self.size * 0.2);
                    }
                    ui.horizontal_top(|ui| {
                        let room = self.size * 1.6;
                        match (item.task, start) {
                            (Some(ticked), _) => {
                                let icon = if ticked {
                                    icons::CHECKED
                                } else {
                                    icons::UNCHECKED
                                };
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(room, self.size * LINE_HEIGHT),
                                    Sense::hover(),
                                );
                                let side = self.size;
                                let at = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(0.0, (rect.height() - side) / 2.0),
                                    egui::vec2(side, side),
                                );
                                icons::paint(ui.painter(), at, icon, colour);
                            }
                            (None, Some(first)) => {
                                let number = format!("{}.", first + n as u64);
                                marker(ui, &number, self.size, colour, room);
                            }
                            (None, None) => marker(ui, "•", self.size, colour, room),
                        }
                        ui.vertical(|ui| self.blocks(ui, &item.blocks, colour));
                    });
                }
            }
            Block::Code(source) | Block::Table(source) => self.source(ui, source, colour),
            Block::Rule => {
                ui.add_space(self.size * 0.3);
                ui.separator();
                ui.add_space(self.size * 0.3);
            }
            Block::Picture(picture) => self.picture(ui, picture),
        }
    }

    /// A table or a code block, by its `source`, as the app's formatted
    /// text draws them in the reading view: a table at the text's size,
    /// as wide as the window allows (obf-1048), code coloured by its
    /// language (obf-1036).
    fn source(&mut self, ui: &mut Ui, source: &str, colour: Color32) {
        let room = TableRoom {
            card: None,
            wide: Some(self.wide),
        };
        let none = BoxLook::default();
        let text = (source, text_format::MARKDOWN, &none);
        let look = (Some(self.size / theme::text_factor(ui.ctx())), colour);
        let spots =
            text_marks_view::room_label(ui, text, None, (0, source.len()), look, false, room);
        let id = Id::new(("markdown source", find_marks::text_key(source)));
        // Its sideways bar is taken there; full screen it does not open.
        // Its ⋮ copies it (obf-1050).
        if let Some((at, TableAct::Copy)) =
            text_marks_view::table_clicks(ui, self.t, id, &spots.tables, false)
        {
            self.copied = table::copy(source, at, &none.states);
        }
        text_marks_view::link_clicks(ui, id, &spots.links);
    }

    /// A picture as wide as the column at most, or its name dimmed where
    /// it was not found or read.
    fn picture(&mut self, ui: &mut Ui, picture: &Picture) {
        match self.textures.get(picture) {
            Some(texture) => {
                let size = texture.size_vec2();
                let width = size.x.min(ui.available_width());
                let shown = egui::vec2(width, size.y * width / size.x.max(1.0));
                let image = egui::Image::new((texture.id(), shown)).alt_text(&picture.alt);
                ui.add(image);
            }
            None => {
                let weak = ui.visuals().weak_text_color();
                let said = self.t.desk_markdown_picture_missing(&picture.target);
                ui.label(RichText::new(said).italics().color(weak));
            }
        }
    }

    /// The colour of a link to `target`: the accent's, dimmed where it
    /// finds nothing.
    fn link_colour(&self, ui: &Ui, target: &Target) -> Color32 {
        let accent = ui.visuals().hyperlink_color;
        let weak = ui.visuals().weak_text_color();
        match target {
            Target::Note { name, .. } => match self.read.notes.get(name) {
                Some(Some(_)) => accent,
                _ => weak,
            },
            Target::Nowhere => weak,
            _ => accent,
        }
    }

    /// The look of `run` at `size` in `colour`.
    fn format(&self, ui: &Ui, run: &Run, (size, colour): (f32, Color32)) -> TextFormat {
        let style = run.style;
        let font = if style.code {
            FontId::monospace(size * CODE_SCALE)
        } else if style.note {
            FontId::proportional(size * 0.7)
        } else {
            FontId::proportional(size)
        };
        let colour = match &run.link {
            Some(target) => self.link_colour(ui, target),
            None => colour,
        };
        let mut format = TextFormat::simple(font, colour);
        format.italics = style.italic;
        if style.strike {
            format.strikethrough = Stroke::new(1.0, colour);
        }
        if run.link.is_some() && !style.note {
            format.underline = Stroke::new(1.0, colour);
        }
        if style.note {
            format.valign = Align::TOP;
        }
        if style.code {
            format.background = ui.visuals().code_bg_color;
            format.expand_bg = text_marks_view::CODE_PAD * size;
        } else if style.highlight {
            format.background = highlight_tint(self.dark);
            format.expand_bg = 1.0;
        }
        format
    }

    /// Draws `text` at `size` in `colour`, all of it bold with `bold`,
    /// selectable to be copied, the places the find found on their
    /// tints; its links take clicks.
    fn inline(
        &mut self,
        ui: &mut Ui,
        text: &Inline,
        (size, colour, bold): (f32, Color32, bool),
    ) -> egui::Response {
        let mut job = LayoutJob::default();
        let mut ghost = LayoutJob::default();
        let mut thick = false;
        for run in &text.runs {
            let part = text.text.get(run.start..run.end).unwrap_or_default();
            let format = self.format(ui, run, (size, colour));
            let strong = bold || run.style.bold;
            let mut again = format.clone();
            again.background = Color32::TRANSPARENT;
            again.underline = Stroke::NONE;
            again.strikethrough = Stroke::NONE;
            if !strong {
                again.color = Color32::TRANSPARENT;
            }
            thick |= strong;
            job.append(part, 0.0, format);
            ghost.append(part, 0.0, again);
        }
        // The places the find found in it (obf-1047).
        let mut current = None;
        if let Some((hits, marks)) = find_marks::hits(ui.ctx(), &text.text, text_format::PLAIN) {
            let at = hits.current(&marks);
            let ranges: Vec<(usize, usize, bool)> = hits
                .found
                .iter()
                .enumerate()
                .map(|(i, h)| (h.source.0, h.source.1, Some(i) == at))
                .collect();
            find_marks::mark_job(&mut job, &ranges, self.dark);
            if marks.reveal {
                current = at
                    .and_then(|i| hits.found.get(i))
                    .map(|h| (h.source, marks));
            }
        }
        let galley = WidgetText::from(job).into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            ui.available_width(),
            FontSelection::Default,
        );
        let extra = text_marks_view::line_extra(ui, &FontId::proportional(size), LINE_HEIGHT);
        let galley = text_marks_view::spread(galley, |_, _| extra);
        let response = read_text::label(ui, egui::Label::new(galley.clone()));
        let origin = response.rect.min;
        if thick && ui.is_rect_visible(response.rect) {
            ghost.wrap = galley.job.wrap.clone();
            ghost.halign = galley.job.halign;
            let ghost =
                text_marks_view::spread(ui.fonts_mut(|f| f.layout_job(ghost)), |_, _| extra);
            ui.painter()
                .galley(origin + egui::vec2(0.6, 0.0), ghost, colour);
        }
        let chars = |byte: usize| text.text.get(..byte).map_or(0, |s| s.chars().count());
        if let Some(((a, b), marks)) = current
            && let Some(rect) = text_marks_view::row_rects(&galley, (chars(a), chars(b))).first()
        {
            find_marks::reveal(ui, &marks, rect.translate(origin.to_vec2()));
        }
        for (i, run) in text.runs.iter().enumerate() {
            let Some(target) = &run.link else {
                continue;
            };
            let label = text.text.get(run.start..run.end).unwrap_or_default();
            let rects = text_marks_view::row_rects(&galley, (chars(run.start), chars(run.end)));
            for (j, rect) in rects.iter().enumerate() {
                let id = response.id.with(("link", i, j));
                let link = ui.interact(rect.translate(origin.to_vec2()), id, Sense::click());
                link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, label));
                let hint = match target {
                    Target::Web(url) => Some(url.clone()),
                    Target::Note { name, .. }
                        if self.read.notes.get(name).is_none_or(Option::is_none) =>
                    {
                        Some(self.t.desk_markdown_not_found().to_string())
                    }
                    _ => None,
                };
                let link = link.on_hover_cursor(egui::CursorIcon::PointingHand);
                let link = match hint {
                    Some(hint) => link.on_hover_text(hint),
                    None => link,
                };
                if link.clicked() {
                    self.clicked = Some(target.clone());
                }
            }
        }
        response
    }
}

/// A list's or footnote's mark, `said` at `size` in `colour`, in `room`
/// across.
fn marker(ui: &mut Ui, said: &str, size: f32, colour: Color32, room: f32) {
    let galley = WidgetText::from(RichText::new(said).size(size).color(colour)).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::Default,
    );
    let height = galley.size().y.max(size * LINE_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(room, height), Sense::hover());
    let top = rect.min + egui::vec2(0.0, (size * LINE_HEIGHT - galley.size().y).max(0.0) / 2.0);
    ui.painter().galley(top, galley, colour);
}

/// The colour of a callout of `kind`, as Obsidian tells its kinds apart:
/// blue to note, green for a tip or what is done, orange to warn, red
/// for danger and failure, violet for an example, grey for a quote.
pub fn callout_colour(kind: &str) -> Color32 {
    let rgb = match kind {
        "tip" | "hint" | "important" | "success" | "check" | "done" => 0x2E7D32,
        "warning" | "caution" | "attention" | "question" | "help" | "faq" => 0xEF6C00,
        "failure" | "fail" | "missing" | "danger" | "error" | "bug" => 0xC62828,
        "example" => 0x6A1B9A,
        "quote" | "cite" | "abstract" | "summary" | "tldr" => 0x546E7A,
        _ => 0x1565C0,
    };
    let [_, r, g, b] = u32::to_be_bytes(rgb);
    Color32::from_rgb(r, g, b)
}

/// The tint behind a highlight (`==text==`): violet, which a find's
/// amber never is.
pub fn highlight_tint(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x4A, 0x3A, 0x6B)
    } else {
        Color32::from_rgb(0xE6, 0xD5, 0xFF)
    }
}

#[cfg(test)]
mod tests;
