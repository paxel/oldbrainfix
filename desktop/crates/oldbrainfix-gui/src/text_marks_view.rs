//! Boxes in text on the desktop's screen (ADR 0035), as the phone's
//! `text_marks_view.dart` draws them: a ☐, ✅ or ❌ anywhere in a text
//! that carries an id of its own shows the state of its latest tick and
//! is read by a screen reader as that state; where this Device may tick
//! it, a click on it opens Check, Fail and Open at the pointer, the
//! current state left out, and its History once it has been ticked. A
//! tick is an entry beside the text, never an edit of it. The ids are
//! never drawn: a text is laid out from its source, each box as the
//! glyph of its state, its id left out (or, in an edit field, kept but
//! drawn invisibly small, so the field's cursor counts every character).
//! With Show ticks on, a box says after it who ticked it last and when.
//!
//! The boxes are found in the text as it is drawn and taken as widgets
//! of their own afterwards, over whatever lies under them (a card, the
//! editor's field), so that they get the click.
//!
//! A `#tag` in a text (#126, ADR 0032) is drawn in the accent colour,
//! and where the text is read, on a card or in a Section, a click on it
//! opens the search with the tag as written; a screen reader reads it as
//! a link. In the editor's field it is only coloured: a click there
//! places the cursor.
//!
//! A formatted text (#127, ADR 0033) is drawn as it reads, without its
//! marks: bold, italic, struck through, two sizes of heading, bulleted
//! and numbered items, quotes, links and code, line by line as the core
//! reads it. Its boxes and tags are found in its source and take clicks
//! as in a plain text; a link to a web or e-mail address opens in the
//! system's browser or mail program on a click, and any other stays
//! text.
//!
//! A bare web address, `http://` or `https://` (#222), in a plain or a
//! formatted text is drawn and opens as such a link does, but none in
//! code, a code block or a link ([`markdown::autolinks`]); a tag or a box
//! inside one is none. Nothing about it is stored.
//!
//! A table of a formatted text (#128, ADR 0033) is drawn as a grid,
//! scaled down to the width there is: its header row bold on grey, its
//! first column bold while it holds headings, each cell one line of the
//! formats above with its boxes and tags taking clicks. A click
//! elsewhere on it opens it in the full-screen table view.
//!
//! Code is coloured by its language (obf-1036, ADR 0054): a Code text by
//! the language its Note keeps, a fenced code block by the one its fence
//! names, each part in the colour of its kind ([`theme::code_colour`]);
//! only the glyphs' colour changes, the font, the tint and the sizes
//! stay. Without a language, or with one not known here, it shows in one
//! colour as before. The grammars are the person's (obf-1037): the
//! built-in ones and those they added, which the app hands in each frame
//! ([`set_grammars`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Local, Utc};
use egui::cache::{CacheTrait, ComputerMut, FrameCache};
use egui::text::{CCursor, CharIndex, Galley, LayoutJob, TextFormat};
use egui::{
    Color32, FontId, FontSelection, Id, Modifiers, Popup, Pos2, Rect, RichText, Sense, Stroke,
    TextStyle, TextWrapMode, Ui, Vec2, WidgetInfo, WidgetText, WidgetType,
};
use oldbrainfix_core::app_link::{self, AppLink};
use oldbrainfix_core::highlight::{CodeSpan, Grammar, Grammars};
use oldbrainfix_core::markdown::table::{Align, Cell, Table};
use oldbrainfix_core::markdown::{self, Block, BlockKind, CodeBlock, Span, SpanKind};
use oldbrainfix_core::text_boxes::{self, FoundBox};
use oldbrainfix_core::{BoxState, BoxTick, FoundTag, ShownBox, text_format};

use crate::l10n::L10n;
use crate::{schedule_text, theme};

/// How far apart the lines of a formatted text lie (#327, obf-1020), as
/// on the phone: a line's height in parts of its font's size.
pub const LINE_HEIGHT: f32 = 1.5;

/// A code block's line height, in parts of its font's size.
pub const CODE_LINE_HEIGHT: f32 = 1.3;

/// Code's font size, in parts of the words' around it.
pub const CODE_SCALE: f32 = 0.9;

/// How far code's tint reaches past its words, in parts of the font's
/// size.
pub const CODE_PAD: f32 = 0.12;

/// A first heading's size, in parts of the words' size.
pub const HEADING1: f32 = 1.3;

/// A second heading's size, in parts of the words' size.
pub const HEADING2: f32 = 1.15;

/// The room a blank line between two lines of a formatted text makes as
/// it reads, in parts of the font's size, instead of the room between
/// the two.
pub const BLANK_GAP: f32 = 1.0;

/// The room before a line of a formatted text of the kind `next` after
/// one of `prev`, besides the line height, in parts of the font's size:
/// more before a heading, less after one, little between two items or
/// two quote lines, half the size between all others (#327).
pub fn line_gap(prev: BlockKind, next: BlockKind) -> f32 {
    let item = |k| matches!(k, BlockKind::Bullet | BlockKind::Numbered);
    match (prev, next) {
        (_, BlockKind::Heading1 | BlockKind::Heading2) => 0.8,
        (BlockKind::Heading1 | BlockKind::Heading2, _) => 0.3,
        (BlockKind::Quote, BlockKind::Quote) => 0.2,
        (a, b) if item(a) && item(b) => 0.2,
        _ => 0.5,
    }
}

/// How much farther apart two rows of `font` lie than the font's own
/// row height for a line height of `height` parts of its size.
pub fn line_extra(ui: &Ui, font: &FontId, height: f32) -> f32 {
    let row = ui.fonts_mut(|f| f.row_height(font));
    (height * font.size - row).max(0.0)
}

/// `galley` with each row after the first moved down by what `before`
/// gives for it: the line it is on (counted from 0) and whether it is
/// that line's first row, rounded to whole pixels. egui lays a taller
/// line height out with all its room under the words, which takes an
/// underline and a strike-through along, so rows are moved apart instead;
/// a click still finds its character, as rows are found by where they
/// lie.
pub fn spread(mut galley: Arc<Galley>, before: impl Fn(usize, bool) -> f32) -> Arc<Galley> {
    if galley.rows.len() < 2 {
        return galley;
    }
    let laid = Arc::make_mut(&mut galley);
    let pixels = laid.pixels_per_point.max(1.0);
    let (mut line, mut first, mut down) = (0, true, 0.0);
    for (i, placed) in laid.rows.iter_mut().enumerate() {
        if i > 0 {
            down += (before(line, first) * pixels).round() / pixels;
        }
        placed.pos.y += down;
        first = placed.ends_with_newline;
        if first {
            line += 1;
        }
    }
    laid.rect.max.y += down;
    laid.mesh_bounds.max.y += down;
    galley
}

/// A box as it is drawn: its id, the state it shows and where it lies
/// on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxSpot {
    pub id: String,
    pub state: BoxState,
    pub rect: Rect,
}

/// How the boxes of a text show (ADR 0035): the state each shows now, by
/// its id, a box not among them in the state its glyph was written in;
/// and, with Show ticks on, what each ticked box says after it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoxLook {
    pub states: BTreeMap<String, BoxState>,
    pub after: BTreeMap<String, String>,
}

impl BoxLook {
    /// The states of `boxes`, a Note's or a Section's, nothing said after
    /// them.
    pub fn of(boxes: &[ShownBox]) -> BoxLook {
        BoxLook {
            states: boxes.iter().map(|b| (b.id.clone(), b.state)).collect(),
            after: BTreeMap::new(),
        }
    }

    /// The state the box `found` shows.
    pub fn state(&self, found: &FoundBox) -> BoxState {
        self.states.get(&found.id).copied().unwrap_or(found.state)
    }
}

/// A tag as it is drawn: where it begins in the whole text, in bytes,
/// the tag as written ("#Garten") and where it lies on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct TagSpot {
    pub start: usize,
    pub written: String,
    pub rect: Rect,
}

/// A link of a formatted text that may be opened, as it is drawn: where
/// it begins in the whole text, in bytes, where it goes, the words it
/// shows and where they lie on the screen, a rectangle for each row.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkSpot {
    pub start: usize,
    pub url: String,
    pub label: String,
    pub rects: Vec<Rect>,
}

/// A table of a formatted text as it is drawn: where it begins in the
/// whole text, in bytes, and where it lies on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableSpot {
    pub at: usize,
    pub rect: Rect,
}

/// A cell of a table of a formatted text as it is drawn: the table's
/// [`TableSpot::at`], the cell's row and column, and where it lies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellSpot {
    pub at: usize,
    pub cell: (usize, usize),
    pub rect: Rect,
}

/// The boxes, the tags, the links and the tables of a text as it is
/// drawn, and the tables' cells.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spots {
    pub boxes: Vec<BoxSpot>,
    pub tags: Vec<TagSpot>,
    pub links: Vec<LinkSpot>,
    pub tables: Vec<TableSpot>,
    pub cells: Vec<CellSpot>,
}

impl Spots {
    /// Adds the spots of another text drawn.
    pub fn extend(&mut self, other: Spots) {
        self.boxes.extend(other.boxes);
        self.tags.extend(other.tags);
        self.links.extend(other.links);
        self.tables.extend(other.tables);
        self.cells.extend(other.cells);
    }

    /// The cell of the table at `at` drawn under `pos`, by row and
    /// column.
    pub fn cell_at(&self, at: usize, pos: Option<Pos2>) -> Option<(usize, usize)> {
        let pos = pos?;
        self.cells
            .iter()
            .find(|spot| spot.at == at && spot.rect.contains(pos))
            .map(|spot| spot.cell)
    }
}

/// The bytes `from..to` of `text` laid out in `font` and `colour`, each
/// tag in it in `accent`, unwrapped.
pub fn tag_job(
    text: &str,
    (from, to): (usize, usize),
    font: FontId,
    (colour, accent): (Color32, Color32),
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat::simple(font.clone(), colour);
    let tagged = TextFormat::simple(font, accent);
    let mut at = from;
    for tag in markdown::plain_tags(text) {
        if tag.start < from || tag.end > to {
            continue;
        }
        if let (Some(before), Some(written)) =
            (text.get(at..tag.start), text.get(tag.start..tag.end))
        {
            job.append(before, 0.0, plain.clone());
            job.append(written, 0.0, tagged.clone());
            at = tag.end;
        }
    }
    job.append(text.get(at..to).unwrap_or_default(), 0.0, plain);
    job
}

/// What a screen reader says for a box showing `state`.
pub fn box_label(t: &L10n, state: BoxState) -> &'static str {
    match state {
        BoxState::Open => t.text_mark_open(),
        BoxState::Done => t.text_mark_checked(),
        BoxState::Failed => t.text_mark_failed(),
    }
}

/// The menu entry that ticks a box to `to`, its glyph first.
pub fn menu_entry(t: &L10n, to: BoxState) -> String {
    let word = match to {
        BoxState::Done => t.text_mark_check(),
        BoxState::Failed => t.text_mark_fail(),
        BoxState::Open => t.text_mark_reopen(),
    };
    format!("{} {word}", to.glyph())
}

/// Who wrote `tick` and when, as "Ann, Tue 14:02", the phone's
/// `tickText`: "You" for the user's own, the time alone when the person
/// is not known (`by` none) or blocked. The weekday stands for a time
/// within six days of `now`, the date before; `full`, as the History
/// lists them, always the weekday and the date.
pub fn tick_text(
    t: &L10n,
    tick: &BoxTick,
    by: Option<&str>,
    (now, full): (DateTime<Utc>, bool),
) -> String {
    let at = tick.at.with_timezone(&Local);
    let day = oldbrainfix_core::Day {
        year: i64::from(chrono::Datelike::year(&at)),
        month: chrono::Datelike::month(&at),
        day: chrono::Datelike::day(&at),
    };
    let clock = at.format("%H:%M");
    let when = if full {
        format!("{} {clock}", schedule_text::day_text(t, day))
    } else if now - tick.at < chrono::Duration::days(6) {
        let weekday = schedule_text::weekday_text(t, day.weekday());
        format!("{weekday} {clock}")
    } else {
        format!("{} {clock}", schedule_text::short_day_text(t, day))
    };
    let name = if tick.own { Some(t.box_tick_you()) } else { by };
    match name {
        Some(name) => t.box_tick_by(name, &when),
        None => when,
    }
}

/// Draws the bytes `shown` of the source `text` as a label of `size`
/// (the body's while `None`) in `colour`, its tags in the accent colour,
/// its boxes as `boxes` says without their ids, cut off at the edge with
/// `truncate` and wrapped otherwise, and gives where its boxes and tags
/// lie. Without either it is the plain label it was.
pub fn marked_label(
    ui: &mut Ui,
    (text, boxes): (&str, &BoxLook),
    shown: (usize, usize),
    look: (Option<f32>, Color32),
    truncate: bool,
) -> Spots {
    let (from, to) = shown;
    let Some(part) = text.get(from..to) else {
        return Spots::default();
    };
    let found = Found::in_parsed(&parsed(ui.ctx(), text, false), boxes);
    let marked = found
        .boxes
        .iter()
        .any(|b| b.0.start >= from && b.0.end <= to);
    let tagged = found.tags.iter().any(|t| t.start >= from && t.end <= to);
    let linked = found.links.iter().any(|&(s, e)| s >= from && e <= to);
    if !marked && !tagged && !linked {
        // At the Device's text size, as [`Look::of`] has it (#326,
        // obf-1019).
        let size = look
            .0
            .unwrap_or_else(|| TextStyle::Body.resolve(ui.style()).size);
        let rich = RichText::new(part)
            .color(look.1)
            .size(crate::theme::content(ui, size));
        let label = egui::Label::new(rich);
        crate::read_text::label(ui, if truncate { label.truncate() } else { label });
        return Spots::default();
    }
    let wrap = if truncate {
        TextWrapMode::Truncate
    } else {
        TextWrapMode::Wrap
    };
    let look = Look::of(ui, look);
    let mut line = Line::default();
    add_written(&mut line, text, shown, Style::default(), (&found, &look));
    let galley = WidgetText::from(std::mem::take(&mut line.job)).into_galley(
        ui,
        Some(wrap),
        ui.available_width(),
        FontSelection::Default,
    );
    let response = crate::read_text::label(ui, egui::Label::new(galley.clone()));
    line_spots(&galley, response.rect.min, &line)
}

/// Draws the bytes `shown` of the source `text`, in `format`, its boxes
/// as `boxes` says, as [`marked_label`] draws them: a formatted text as
/// it reads, line by line, a table as a grid (a row's cells one after
/// another while cut off), a code block as its lines in the fixed-width
/// font on the background of code (its lines without the fences while
/// cut off), a plain one as it is written. Gives where its boxes, tags,
/// links and tables lie.
pub fn formatted_label(
    ui: &mut Ui,
    text: (&str, &str, &BoxLook),
    shown: (usize, usize),
    look: (Option<f32>, Color32),
    truncate: bool,
) -> Spots {
    note_label(ui, text, None, shown, look, truncate)
}

/// As [`formatted_label`], a Code text coloured by `language`, the scope
/// of the grammar its Note keeps (obf-1036).
pub fn note_label(
    ui: &mut Ui,
    (text, format, boxes): (&str, &str, &BoxLook),
    language: Option<&str>,
    shown: (usize, usize),
    look: (Option<f32>, Color32),
    truncate: bool,
) -> Spots {
    if format == text_format::CODE {
        code_text_label(ui, (text, language), shown, look, (truncate, false));
        return Spots::default();
    }
    if format != text_format::MARKDOWN {
        return marked_label(ui, (text, boxes), shown, look, truncate);
    }
    let (from, to) = shown;
    if text.get(from..to).is_none() {
        return Spots::default();
    }
    let look = Look::of(ui, look);
    let parsed = parsed(ui.ctx(), text, true);
    let found = Found::in_parsed(&parsed, boxes);
    let blocks = &parsed.blocks;
    let (first, last) = (line_at(text, from), line_at(text, to));
    let mut spots = Spots::default();
    let size = look.font.size;
    let extra = line_extra(ui, &look.font, LINE_HEIGHT);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        // The kind of the line drawn last and the blank lines since.
        let mut drawn: Option<BlockKind> = None;
        let mut blank = 0;
        // Room before each line drawn (#327): by the kinds of the two,
        // or a blank line's; cut off, only the line height.
        let mut room = |ui: &mut Ui, kind: BlockKind, blank: &mut usize| {
            let gap = match drawn {
                _ if truncate => 0.0,
                _ if *blank > 0 => *blank as f32 * BLANK_GAP,
                Some(prev) => line_gap(prev, kind),
                None => 0.0,
            };
            if drawn.is_some() || *blank > 0 {
                ui.add_space(extra + gap * size);
            }
            *blank = 0;
            drawn = Some(kind);
        };
        for block in blocks.get(first..=last).unwrap_or_default() {
            if let Some(code) = &block.code {
                if !truncate {
                    if block.line == code.first_line.max(first) {
                        room(ui, BlockKind::Code, &mut blank);
                        code_label(ui, text, (blocks, code), (first, last), &look);
                    }
                } else if !code.is_fence(block.line) {
                    room(ui, BlockKind::Code, &mut blank);
                    let line = (block.line, block.line);
                    let colours = block_colours(ui.ctx(), text, (blocks, code), line);
                    let drawn = block_label(ui, (text, block), &found, &look, (truncate, &colours));
                    spots.extend(drawn);
                }
                continue;
            }
            match &block.table {
                // A table is drawn once, at its first line shown; cut
                // off, each line as its row reads.
                Some(table) if !truncate => {
                    if block.line == table.first_line.max(first) {
                        room(ui, BlockKind::Table, &mut blank);
                        spots.extend(table_label(ui, text, table, (&found, &look)));
                    }
                }
                // A blank line reads as room, not as a line.
                None if !truncate
                    && block.kind == BlockKind::Paragraph
                    && text
                        .get(block.start..block.end)
                        .is_some_and(|l| l.trim().is_empty()) =>
                {
                    blank += 1;
                }
                _ => {
                    room(ui, block.kind, &mut blank);
                    let drawn = block_label(ui, (text, block), &found, &look, (truncate, &[]));
                    spots.extend(drawn);
                }
            }
        }
        // Blank lines at the end keep their room.
        if blank > 0 {
            ui.add_space(blank as f32 * BLANK_GAP * size);
        }
    });
    spots
}

/// Draws the bytes `shown` of a Code text (#213) as written in the
/// fixed-width font, unwrapped, nothing in it a format, a tag or a box:
/// on the background of code, cut off at the edge, or moved sideways
/// with `sideways`; one line cut off without the background with
/// `truncate`, as a compact card shows it. Its parts are coloured by
/// `language`, the scope of its Note's grammar (obf-1036).
pub fn code_text_label(
    ui: &mut Ui,
    (text, language): (&str, Option<&str>),
    (from, to): (usize, usize),
    look: (Option<f32>, Color32),
    (truncate, sideways): (bool, bool),
) {
    let Some(part) = text.get(from..to) else {
        return;
    };
    let look = Look::of(ui, look);
    let format = TextFormat::simple(FontId::monospace(look.font.size * CODE_SCALE), look.colour);
    let shown = text_boxes::strip(part);
    let grammars = grammars(ui.ctx());
    let colours = match grammars.by_scope(language) {
        Some(grammar) => code_text_colours(ui.ctx(), grammar, text, (from, &shown)),
        None => Vec::new(),
    };
    let mut job = LayoutJob::single_section(shown, format);
    paint_code(&mut job, 0, &colours, look.dark);
    if truncate {
        crate::read_text::label(ui, egui::Label::new(job).truncate());
        return;
    }
    egui::Frame::NONE
        .fill(look.code)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if sideways {
                egui::ScrollArea::horizontal()
                    .id_salt(("code text", from))
                    .show(ui, |ui| {
                        crate::read_text::label(ui, egui::Label::new(job).extend())
                    });
                return;
            }
            // Cut off at the edge, every line kept.
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            let size = Vec2::new(ui.available_width(), galley.size().y);
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            let said = galley.text().to_string();
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
            ui.painter()
                .with_clip_rect(rect.intersect(ui.clip_rect()))
                .galley(rect.min, galley, look.colour);
        });
}

/// Draws the lines of `code`, among `blocks` of `text`, from the line
/// `first` to `last` as written, in the fixed-width font on the
/// background of code: nothing in them is a format, a tag or a box.
fn code_label(
    ui: &mut Ui,
    text: &str,
    (blocks, code): (&[Block], &CodeBlock),
    (first, last): (usize, usize),
    look: &Look,
) {
    let lines: Vec<String> = (code.first_line.max(first)..=code.last_line.min(last))
        .filter(|&l| !code.is_fence(l))
        .filter_map(|l| blocks.get(l))
        .map(|b| text_boxes::strip(text.get(b.start..b.end).unwrap_or_default()))
        .collect();
    // Only its closing fence shown: nothing to draw.
    if lines.is_empty() && code.first_line < first {
        return;
    }
    let font = FontId::monospace(look.font.size * CODE_SCALE);
    let extra = line_extra(ui, &font, CODE_LINE_HEIGHT);
    let format = TextFormat::simple(font, look.colour);
    let colours = block_colours(ui.ctx(), text, (blocks, code), (first, last));
    let mut job = LayoutJob::single_section(lines.join("\n"), format);
    paint_code(&mut job, 0, &colours, look.dark);
    egui::Frame::NONE
        .fill(look.code)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let galley = WidgetText::from(job).into_galley(
                ui,
                Some(TextWrapMode::Wrap),
                ui.available_width(),
                FontSelection::Default,
            );
            let galley = spread(galley, |_, _| extra);
            crate::read_text::label(ui, egui::Label::new(galley));
        });
}

/// How a formatted text is drawn: its font, its colour, the accent for
/// links, tags and a quote's bar, the dimmer colour of a quote's words,
/// the background of code and whether the colours of code's parts are
/// the dark look's.
#[derive(Clone)]
pub(crate) struct Look {
    font: FontId,
    colour: Color32,
    accent: Color32,
    weak: Color32,
    code: Color32,
    dark: bool,
}

impl Look {
    /// The body's font of `size` (its own while `None`) in `colour`, at
    /// the Device's text size (#326, obf-1019), the rest as `ui`'s
    /// visuals have it.
    pub(crate) fn of(ui: &Ui, (size, colour): (Option<f32>, Color32)) -> Look {
        let mut font = TextStyle::Body.resolve(ui.style());
        if let Some(size) = size {
            font.size = size;
        }
        font.size = crate::theme::content(ui, font.size);
        let visuals = ui.visuals();
        Look {
            font,
            colour,
            accent: visuals.hyperlink_color,
            weak: visuals.weak_text_color(),
            code: visuals.code_bg_color,
            dark: visuals.dark_mode,
        }
    }

    /// Its font, at the size it shows.
    pub(crate) fn font(&self) -> FontId {
        self.font.clone()
    }

    /// The same, `scale` times as large.
    fn scaled(&self, scale: f32) -> Look {
        let mut look = self.clone();
        look.font.size *= scale;
        look
    }
}

/// The grammars code is coloured with around a context (obf-1037) and
/// how often the person's own ones changed, so colours worked out with
/// one that is gone are not taken again.
#[derive(Clone)]
struct Held {
    grammars: Arc<Grammars>,
    generation: u64,
}

/// Where the context keeps the [`Held`] grammars.
fn grammars_id() -> Id {
    Id::new("oldbrainfix-grammars")
}

/// The built-in grammars alone, as the person's grammars before any are
/// handed in.
fn built_in() -> Arc<Grammars> {
    static BUILT_IN: std::sync::OnceLock<Arc<Grammars>> = std::sync::OnceLock::new();
    BUILT_IN
        .get_or_init(|| Arc::new(Grammars::with_added(Vec::new())))
        .clone()
}

/// Colours code around `ctx` with `grammars`, the person's (obf-1037),
/// for every window; set each frame, so one added, removed or arriving
/// by sync colours at once.
pub fn set_grammars(ctx: &egui::Context, grammars: Arc<Grammars>) {
    ctx.data_mut(|d| {
        let held = d.get_temp::<Held>(grammars_id());
        let generation = match held {
            Some(held) if Arc::ptr_eq(&held.grammars, &grammars) => return,
            Some(held) => {
                let same = held.grammars.added.len() == grammars.added.len()
                    && held
                        .grammars
                        .added
                        .iter()
                        .zip(&grammars.added)
                        .all(|(a, b)| Arc::ptr_eq(a, b));
                held.generation + u64::from(!same)
            }
            None => 0,
        };
        d.insert_temp(
            grammars_id(),
            Held {
                grammars,
                generation,
            },
        );
    });
}

/// The grammars code is coloured with around `ctx`: the person's as last
/// handed in, the built-in ones until then.
pub fn grammars(ctx: &egui::Context) -> Arc<Grammars> {
    ctx.data(|d| d.get_temp::<Held>(grammars_id()))
        .map_or_else(built_in, |held| held.grammars)
}

/// How often the person's own grammars changed around `ctx`.
fn generation(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<Held>(grammars_id()))
        .map_or(0, |held| held.generation)
}

/// A text to colour in a grammar, for [`ColourCache`]: told apart by the
/// grammar, the generation of the person's grammars it is one of, and
/// the text.
#[derive(Clone, Copy)]
struct Colouring<'a> {
    grammar: &'a Grammar,
    generation: u64,
    text: &'a str,
}

impl std::hash::Hash for Colouring<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.grammar).hash(state);
        self.grammar.scope.hash(state);
        self.generation.hash(state);
        self.text.hash(state);
    }
}

/// Colours code by its grammar, for [`ColourCache`].
#[derive(Default)]
struct Colourer;

impl ComputerMut<Colouring<'_>, Arc<Vec<CodeSpan>>> for Colourer {
    fn compute(&mut self, key: Colouring<'_>) -> Arc<Vec<CodeSpan>> {
        Arc::new(key.grammar.colour(key.text))
    }
}

/// The coloured parts of the texts drawn, by their grammar and the text,
/// kept while they are drawn frame after frame.
type ColourCache = FrameCache<Arc<Vec<CodeSpan>>, Colourer>;

/// The coloured parts of `text` in `grammar` (obf-1036), in bytes of
/// `text`: from the context's cache while the same text was drawn in the
/// frame before, so a frame does not colour it again.
pub fn code_colours(ctx: &egui::Context, grammar: &Grammar, text: &str) -> Arc<Vec<CodeSpan>> {
    if text.is_empty() {
        return Arc::default();
    }
    let key = Colouring {
        grammar,
        generation: generation(ctx),
        text,
    };
    ctx.memory_mut(|m| m.caches.cache::<ColourCache>().get(key).clone())
}

/// The parts of `spans` within the bytes `from..to`, counted from `from`.
fn clipped(spans: &[CodeSpan], (from, to): (usize, usize)) -> Vec<CodeSpan> {
    spans
        .iter()
        .filter(|s| s.end > from && s.start < to)
        .map(|s| CodeSpan {
            start: s.start.max(from) - from,
            end: s.end.min(to) - from,
            colour: s.colour,
        })
        .collect()
}

/// The coloured parts of `shown`, the bytes from `from` of the Code text
/// `text` as they show, without the ids of boxes: the whole text coloured
/// in `grammar`, so a part in the middle of a comment or a string reads
/// as one; `shown` coloured alone where the ids taken out move it.
fn code_text_colours(
    ctx: &egui::Context,
    grammar: &Grammar,
    text: &str,
    (from, shown): (usize, &str),
) -> Vec<CodeSpan> {
    let whole = text_boxes::strip(text);
    let at = text_boxes::strip(text.get(..from).unwrap_or_default()).len();
    let end = at + shown.len();
    if whole.get(at..end) == Some(shown) {
        clipped(&code_colours(ctx, grammar, &whole), (at, end))
    } else {
        code_colours(ctx, grammar, shown).to_vec()
    }
}

/// The grammar among `grammars` the fence of `code` names (obf-1036);
/// none for a fence naming none or one not known here.
fn fence_grammar<'a>(grammars: &'a Grammars, code: &CodeBlock) -> Option<&'a Grammar> {
    grammars.for_fence(code.language())
}

/// The coloured parts of the lines `first..=last` of `code`, among
/// `blocks` of `text`, as they show, without its fences and the ids of
/// boxes and joined by line breaks: all its lines coloured as one text by
/// the language its fence names, so what spans lines reads as one; none
/// for a fence naming none or one not known.
fn block_colours(
    ctx: &egui::Context,
    text: &str,
    (blocks, code): (&[Block], &CodeBlock),
    (first, last): (usize, usize),
) -> Vec<CodeSpan> {
    let grammars = grammars(ctx);
    let Some(grammar) = fence_grammar(&grammars, code) else {
        return Vec::new();
    };
    let (mut whole, mut from, mut to) = (String::new(), None, 0);
    let lines = (code.first_line..=code.last_line).filter(|&l| !code.is_fence(l));
    for (i, l) in lines.enumerate() {
        let Some(block) = blocks.get(l) else {
            continue;
        };
        if i > 0 {
            whole.push('\n');
        }
        if l >= first && from.is_none() {
            from = Some(whole.len());
        }
        whole.push_str(&text_boxes::strip(
            text.get(block.start..block.end).unwrap_or_default(),
        ));
        if l <= last {
            to = whole.len();
        }
    }
    let Some(from) = from.filter(|&f| f <= to) else {
        return Vec::new();
    };
    clipped(&code_colours(ctx, grammar, &whole), (from, to))
}

/// Draws the coloured parts `spans` of `job`'s text, counted from its
/// byte `from`, in their colours, the dark look's while `dark`
/// (obf-1036): its sections are cut where a part begins and ends, and
/// only their glyphs' colour changes; font, background and every byte
/// stay.
pub fn paint_code(job: &mut LayoutJob, from: usize, spans: &[CodeSpan], dark: bool) {
    if spans.is_empty() {
        return;
    }
    let sections = std::mem::take(&mut job.sections);
    let mut out = Vec::with_capacity(sections.len() + spans.len() * 2);
    for section in sections {
        let (start, end) = (section.byte_range.start.0, section.byte_range.end.0);
        let mut at = start;
        let mut piece = |a: usize, b: usize, format: TextFormat| {
            if a < b {
                let lead = if a == start {
                    section.leading_space
                } else {
                    0.0
                };
                out.push(egui::text::LayoutSection {
                    leading_space: lead,
                    byte_range: egui::text::ByteIndex(a)..egui::text::ByteIndex(b),
                    format,
                });
            }
        };
        for span in spans {
            let (a, b) = ((span.start + from).max(at), (span.end + from).min(end));
            if a >= b {
                continue;
            }
            piece(at, a, section.format.clone());
            let mut coloured = section.format.clone();
            coloured.color = theme::code_colour(dark, span.colour);
            piece(a, b, coloured);
            at = b;
        }
        piece(at, end, section.format);
    }
    job.sections = out;
}

/// The coloured parts of every fenced code block of the formatted source
/// `text` whose fence names a language known here (obf-1036), in bytes
/// of `text`, for its edit field: each block's lines between its fences
/// coloured as one text, as written.
pub fn fence_colours(ctx: &egui::Context, text: &str) -> Vec<CodeSpan> {
    let blocks = markdown::parse(text);
    let grammars = grammars(ctx);
    let mut spans = Vec::new();
    let mut done = None;
    for block in &blocks {
        let Some(code) = &block.code else {
            continue;
        };
        if done == Some(code.first_line) {
            continue;
        }
        done = Some(code.first_line);
        let Some(grammar) = fence_grammar(&grammars, code) else {
            continue;
        };
        let mut content = (code.first_line..=code.last_line)
            .filter(|&l| !code.is_fence(l))
            .filter_map(|l| blocks.get(l));
        let (Some(start), Some(end)) = (
            content.clone().next().map(|b| b.start),
            content.next_back().map(|b| b.end),
        ) else {
            continue;
        };
        let Some(written) = text.get(start..end) else {
            continue;
        };
        spans.extend(
            code_colours(ctx, grammar, written)
                .iter()
                .map(|s| CodeSpan {
                    start: s.start + start,
                    end: s.end + start,
                    colour: s.colour,
                }),
        );
    }
    spans
}

/// The line `at`, a byte of `text`, lies on, counted from 0.
pub(crate) fn line_at(text: &str, at: usize) -> usize {
    text.get(..at)
        .unwrap_or(text)
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
}

/// The boxes, the tags and the web addresses (#222) of a text, in its
/// source, a table's cells' among them: each box with the state it shows
/// and what it says after it. A box or a tag inside a web address is
/// none.
pub(crate) struct Found {
    boxes: Vec<(FoundBox, BoxState, Option<String>)>,
    tags: Vec<FoundTag>,
    links: Vec<(usize, usize)>,
}

impl Found {
    /// Those of the source `text` in `format`, its boxes as `look` says.
    pub(crate) fn of(text: &str, format: &str, look: &BoxLook) -> Found {
        let markdown = format == text_format::MARKDOWN;
        Found::in_parsed(&Parsed::of(text, markdown), look)
    }

    /// Those `parsed` holds, its boxes as `look` says.
    fn in_parsed(parsed: &Parsed, look: &BoxLook) -> Found {
        let links = parsed.links.clone();
        let boxes = parsed
            .boxes
            .iter()
            .filter(|b| !links.iter().any(|&(s, e)| b.start < e && s < b.end))
            .map(|b| {
                let state = look.state(b);
                let after = look.after.get(&b.id).cloned();
                (b.clone(), state, after)
            })
            .collect();
        Found {
            boxes,
            tags: parsed.tags.clone(),
            links,
        }
    }
}

/// What a text is parsed into before it is drawn: its lines, its web
/// addresses, boxes and tags, as Markdown or as plain text.
struct Parsed {
    blocks: Vec<Block>,
    links: Vec<(usize, usize)>,
    boxes: Vec<FoundBox>,
    tags: Vec<FoundTag>,
}

impl Parsed {
    fn of(text: &str, markdown: bool) -> Parsed {
        let (blocks, links, tags) = if markdown {
            (
                markdown::parse(text),
                markdown::autolinks(text),
                markdown::tags(text),
            )
        } else {
            (
                Vec::new(),
                markdown::plain_autolinks(text),
                markdown::plain_tags(text),
            )
        };
        Parsed {
            blocks,
            links,
            boxes: text_boxes::find(text),
            tags,
        }
    }
}

/// Parses each text once while it is drawn frame after frame (obf-1069),
/// forgotten the first frame it is not.
#[derive(Default)]
struct Parser;

impl ComputerMut<(&str, bool), Arc<Parsed>> for Parser {
    fn compute(&mut self, (text, markdown): (&str, bool)) -> Arc<Parsed> {
        Arc::new(Parsed::of(text, markdown))
    }
}

type ParsedTexts = FrameCache<Arc<Parsed>, Parser>;

/// Where [`parses`] counts.
const PARSES: &str = "text parses";

/// `text` parsed, as Markdown or plain, from what was drawn the frame
/// before where it was.
fn parsed(ctx: &egui::Context, text: &str, markdown: bool) -> Arc<Parsed> {
    ctx.memory_mut(|m| {
        let cache = m.caches.cache::<ParsedTexts>();
        let held = CacheTrait::len(cache);
        let parsed = cache.get((text, markdown)).clone();
        if CacheTrait::len(cache) > held {
            *m.data
                .get_temp_mut_or_default::<usize>(egui::Id::new(PARSES)) += 1;
        }
        parsed
    })
}

/// How often a text drawn was parsed in `ctx`: what a test counts by
/// (obf-1069).
pub fn parses(ctx: &egui::Context) -> usize {
    ctx.data(|d| d.get_temp::<usize>(egui::Id::new(PARSES)))
        .unwrap_or_default()
}

/// What a part of a line is inside of.
#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    /// Inside a link that may be opened.
    link: bool,
    quote: bool,
}

/// A line of a formatted text laid out: what it shows, in `job`; its bold
/// parts once more, in `ghost`, drawn a little to the right over them, as
/// egui's fonts have no bold; and where its boxes, tags and links lie in
/// it, in characters.
#[derive(Default)]
struct Line {
    job: LayoutJob,
    ghost: LayoutJob,
    bold: bool,
    chars: usize,
    boxes: Vec<(String, BoxState, (usize, usize))>,
    tags: Vec<(usize, String, (usize, usize))>,
    links: Vec<(usize, String, (usize, usize))>,
}

impl Line {
    /// Adds `shown` in `format`; gives where it lies, in characters.
    fn push(&mut self, shown: &str, format: TextFormat, bold: bool) -> (usize, usize) {
        let from = self.chars;
        if shown.is_empty() {
            return (from, from);
        }
        let mut ghost = format.clone();
        ghost.background = Color32::TRANSPARENT;
        ghost.underline = Stroke::NONE;
        ghost.strikethrough = Stroke::NONE;
        if !bold {
            ghost.color = Color32::TRANSPARENT;
        }
        self.bold |= bold;
        self.job.append(shown, 0.0, format);
        self.ghost.append(shown, 0.0, ghost);
        self.chars += shown.chars().count();
        (from, self.chars)
    }
}

/// The format of a part inside `style`, code in the fixed-width font on
/// its background.
fn part_format(look: &Look, style: Style, code: bool) -> TextFormat {
    let font = if code {
        FontId::monospace(look.font.size * CODE_SCALE)
    } else {
        look.font.clone()
    };
    let colour = if style.link {
        look.accent
    } else if style.quote {
        look.weak
    } else {
        look.colour
    };
    let mut format = TextFormat::simple(font, colour);
    format.italics = style.italic;
    if style.strike {
        format.strikethrough = Stroke::new(1.0, colour);
    }
    if style.link {
        format.underline = Stroke::new(1.0, colour);
    }
    if code {
        // A light tint behind the word, a little wider than it (#327).
        format.background = look.code;
        format.expand_bg = CODE_PAD * look.font.size;
    }
    format
}

/// Adds the parts `spans` of `text` to `line`.
fn add_spans(line: &mut Line, text: &str, spans: &[Span], style: Style, ctx: (&Found, &Look)) {
    let look = ctx.1;
    for span in spans {
        match span.kind {
            SpanKind::Text if span.content_start == span.start => {
                add_written(line, text, (span.start, span.end), style, ctx);
            }
            // Never an id drawn, of a box or of a glyph copied in.
            SpanKind::Text => {
                let shown = text_boxes::strip(&span.text);
                line.push(&shown, part_format(look, style, false), style.bold);
            }
            SpanKind::Code => {
                let shown = text_boxes::strip(&span.text);
                line.push(&shown, part_format(look, style, true), style.bold);
            }
            SpanKind::Bold => {
                let inner = Style {
                    bold: true,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
            }
            SpanKind::Italic => {
                let inner = Style {
                    italic: true,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
            }
            SpanKind::Strike => {
                let inner = Style {
                    strike: true,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
            }
            SpanKind::Link => {
                // Only to a web or e-mail address; any other is its words.
                let opens = !style.link && markdown::may_open(&span.url);
                let from = line.chars;
                let inner = Style {
                    link: style.link || opens,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
                if opens {
                    line.links
                        .push((span.start, span.url.clone(), (from, line.chars)));
                }
            }
        }
    }
}

/// What a run of characters shown as written holds: a box, by its place
/// among those found, a tag or a web address.
enum Cut {
    Box(usize),
    Tag,
    Link,
}

/// Adds the bytes `start..end` of `text`, shown as written, to `line`,
/// its boxes, tags and web addresses cut out of it: a box as the glyph of
/// the state it shows, its id left out, and with Show ticks what it says
/// after it, small and dim; a web address as a link that opens (#222); a
/// tag in a link's words is part of the link.
fn add_written(
    line: &mut Line,
    text: &str,
    (start, end): (usize, usize),
    style: Style,
    (found, look): (&Found, &Look),
) {
    let format = part_format(look, style, false);
    let mut cuts: Vec<(usize, usize, Cut)> = found
        .boxes
        .iter()
        .enumerate()
        .filter(|(_, b)| b.0.start >= start && b.0.end <= end)
        .map(|(i, b)| (b.0.start, b.0.end, Cut::Box(i)))
        .collect();
    if !style.link {
        cuts.extend(
            found
                .tags
                .iter()
                .filter(|t| t.start >= start && t.end <= end)
                .map(|t| (t.start, t.end, Cut::Tag)),
        );
        cuts.extend(
            found
                .links
                .iter()
                .filter(|&&(s, e)| s >= start && e <= end)
                .map(|&(s, e)| (s, e, Cut::Link)),
        );
    }
    cuts.sort_by_key(|c| c.0);
    let mut at = start;
    for (a, b, cut) in cuts {
        let (Some(before), Some(piece)) = (text.get(at..a), text.get(a..b)) else {
            continue;
        };
        line.push(before, format.clone(), style.bold);
        match cut {
            Cut::Box(i) => {
                let Some((found, state, after)) = found.boxes.get(i) else {
                    continue;
                };
                let glyph = state.glyph().to_string();
                let chars = line.push(&glyph, format.clone(), style.bold);
                line.boxes.push((found.id.clone(), *state, chars));
                if let Some(after) = after {
                    let small =
                        TextFormat::simple(FontId::proportional(look.font.size * 0.75), look.weak);
                    line.push(&format!(" {after} "), small, false);
                }
            }
            Cut::Tag => {
                let mut tagged = format.clone();
                tagged.color = look.accent;
                let chars = line.push(piece, tagged, style.bold);
                line.tags.push((a, piece.to_string(), chars));
            }
            Cut::Link => {
                // Never an id drawn, of a glyph copied in.
                let url = text_boxes::strip(piece);
                let linked = Style {
                    link: true,
                    ..style
                };
                let chars = line.push(&url, part_format(look, linked, false), style.bold);
                if markdown::may_open(&url) {
                    line.links.push((a, url, chars));
                }
            }
        }
        at = b;
    }
    line.push(text.get(at..end).unwrap_or_default(), format, style.bold);
}

/// What goes before an item's words: its bullet or its number.
fn item_prefix(block: &Block, look: &Look, colour: Color32) -> Option<LayoutJob> {
    let (shown, font) = match block.kind {
        // The proportional font has no "◦"; the fixed-width one has.
        BlockKind::Bullet if block.level > 0 => {
            ("◦".to_string(), FontId::monospace(look.font.size))
        }
        BlockKind::Bullet => ("•".to_string(), look.font.clone()),
        BlockKind::Numbered => (format!("{}.", block.number), look.font.clone()),
        _ => return None,
    };
    Some(LayoutJob::simple_singleline(shown, font, colour))
}

/// Draws the line `block` of `text`, cut off at the edge with `truncate`
/// and wrapped otherwise, and gives where its marks, tags and links lie.
/// A heading cut off keeps the size of the words around it; a line of a
/// code block takes the colours `colours` of its parts (obf-1036).
fn block_label(
    ui: &mut Ui,
    (text, block): (&str, &Block),
    found: &Found,
    look: &Look,
    (truncate, colours): (bool, &[CodeSpan]),
) -> Spots {
    let mut font = look.font.clone();
    let heading = match block.kind {
        BlockKind::Heading1 => Some(HEADING1),
        BlockKind::Heading2 => Some(HEADING2),
        _ => None,
    };
    if let Some(scale) = heading
        && !truncate
    {
        font.size *= scale;
    }
    let look = Look {
        font,
        colour: look.colour,
        accent: look.accent,
        weak: look.weak,
        code: look.code,
        dark: look.dark,
    };
    let style = Style {
        bold: heading.is_some(),
        quote: block.kind == BlockKind::Quote,
        ..Style::default()
    };
    let mut line = Line::default();
    match &block.table {
        // A row cut off: its cells that show something, one after
        // another; the line marking the first column as headings as the
        // header row, the delimiter row as nothing.
        Some(table) => {
            let shown = block
                .row
                .or((block.line + 2 == table.delimiter_line).then_some(0))
                .and_then(|row| Some((row, table.rows.get(row)?)));
            if let Some((number, row)) = shown {
                let cells = row.cells.iter().filter(|c| c.end > c.start);
                for (i, cell) in cells.enumerate() {
                    if i > 0 {
                        line.push(" ", part_format(&look, style, false), false);
                    }
                    let style = Style {
                        bold: number == 0,
                        ..style
                    };
                    add_spans(&mut line, text, &cell.spans, style, (found, &look));
                }
            }
        }
        // A line of a code block as written, in the fixed-width font.
        None if block.kind == BlockKind::Code => {
            let shown = text_boxes::strip(text.get(block.start..block.end).unwrap_or_default());
            line.push(&shown, part_format(&look, style, true), false);
            paint_code(&mut line.job, 0, colours, look.dark);
        }
        None => add_spans(&mut line, text, &block.spans, style, (found, &look)),
    }
    let quote = block.kind == BlockKind::Quote;
    let indent = match block.kind {
        BlockKind::Bullet | BlockKind::Numbered if block.level > 0 => 20.0,
        _ if quote => 12.0,
        _ => 0.0,
    };
    let wrap = if truncate {
        TextWrapMode::Truncate
    } else {
        TextWrapMode::Wrap
    };
    let row = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // The indent grows with the text size (#326, obf-1019).
        ui.add_space(crate::theme::content(ui, indent));
        if let Some(prefix) = item_prefix(block, &look, look.colour) {
            let galley = WidgetText::from(prefix).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                FontSelection::Default,
            );
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(galley.size().x.max(14.0) + 6.0, galley.size().y),
                Sense::hover(),
            );
            ui.painter().galley(rect.min, galley, look.colour);
        }
        if line.chars == 0 {
            // A blank line keeps its height.
            let height = ui.fonts_mut(|f| f.row_height(&look.font));
            ui.allocate_exact_size(egui::vec2(1.0, height), Sense::hover());
            return Spots::default();
        }
        let galley = WidgetText::from(std::mem::take(&mut line.job)).into_galley(
            ui,
            Some(wrap),
            ui.available_width(),
            FontSelection::Default,
        );
        // Its rows as far apart as the line height says (#327).
        let extra = line_extra(ui, &look.font, LINE_HEIGHT);
        let galley = spread(galley, |_, _| extra);
        let response = crate::read_text::label(ui, egui::Label::new(galley.clone()));
        let origin = response.rect.min;
        if line.bold && ui.is_rect_visible(response.rect) {
            let mut ghost = std::mem::take(&mut line.ghost);
            ghost.wrap = galley.job.wrap.clone();
            ghost.halign = galley.job.halign;
            let ghost = spread(ui.fonts_mut(|f| f.layout_job(ghost)), |_, _| extra);
            ui.painter()
                .galley(origin + egui::vec2(0.6, 0.0), ghost, look.colour);
        }
        line_spots(&galley, origin, &line)
    });
    if quote && ui.is_rect_visible(row.response.rect) {
        let rect = row.response.rect;
        let x = rect.left() + 4.0;
        ui.painter()
            .vline(x, rect.y_range(), Stroke::new(3.0, look.accent));
    }
    row.inner
}

/// A cell of a table laid out on one line: what it shows, its bold parts
/// once more for the ghost, and where its marks, tags and links lie in
/// it.
pub(crate) struct LaidCell {
    galley: Arc<Galley>,
    ghost: Option<Arc<Galley>>,
    line: Line,
}

impl LaidCell {
    /// How much room its words take, at least a row of `look`'s font.
    pub(crate) fn size(&self, ui: &Ui, look: &Look) -> Vec2 {
        let height = ui.fonts_mut(|f| f.row_height(&look.font));
        let ghost = if self.ghost.is_some() { 0.6 } else { 0.0 };
        let size = self.galley.size();
        egui::vec2(size.x + ghost, size.y.max(height))
    }

    /// Draws it with its top left at `pos`, as a label to a screen reader
    /// while `label`, and gives where its marks, tags and links lie.
    pub(crate) fn paint(&self, ui: &mut Ui, pos: Pos2, colour: Color32, label: bool) -> Spots {
        let rect = Rect::from_min_size(pos, self.galley.size());
        if label {
            ui.put(rect, egui::Label::new(self.galley.clone()));
        } else if ui.is_rect_visible(rect) {
            ui.painter().galley(pos, self.galley.clone(), colour);
        }
        if let Some(ghost) = &self.ghost
            && ui.is_rect_visible(rect)
        {
            ui.painter()
                .galley(pos + egui::vec2(0.6, 0.0), ghost.clone(), colour);
        }
        line_spots(&self.galley, pos, &self.line)
    }
}

/// Lays out `cell` of `text`, bold while `bold`, on one line.
pub(crate) fn lay_cell(
    ui: &Ui,
    (text, cell): (&str, &Cell),
    (found, look): (&Found, &Look),
    bold: bool,
) -> LaidCell {
    let mut line = Line::default();
    let style = Style {
        bold,
        ..Style::default()
    };
    add_spans(&mut line, text, &cell.spans, style, (found, look));
    let job = std::mem::take(&mut line.job);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let ghost = line.bold.then(|| {
        let ghost = std::mem::take(&mut line.ghost);
        ui.fonts_mut(|f| f.layout_job(ghost))
    });
    LaidCell {
        galley,
        ghost,
        line,
    }
}

/// Every cell of `table` laid out, row by row: the header row bold, and
/// the first column too while it holds headings.
pub(crate) fn lay_table(
    ui: &Ui,
    (text, table): (&str, &Table),
    ctx: (&Found, &Look),
) -> Vec<Vec<LaidCell>> {
    table
        .rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            row.cells
                .iter()
                .enumerate()
                .map(|(c, cell)| {
                    let bold = r == 0 || (table.heading_column && c == 0);
                    lay_cell(ui, (text, cell), ctx, bold)
                })
                .collect()
        })
        .collect()
}

/// How wide each column of `laid` is and how high each row, each with
/// `pad` around its words.
pub(crate) fn grid(
    ui: &Ui,
    laid: &[Vec<LaidCell>],
    look: &Look,
    pad: Vec2,
) -> (Vec<f32>, Vec<f32>) {
    let columns = laid.first().map_or(0, Vec::len);
    let mut widths = vec![0.0f32; columns];
    let mut heights = Vec::with_capacity(laid.len());
    for row in laid {
        let mut height = 0.0f32;
        for (c, cell) in row.iter().enumerate() {
            let size = cell.size(ui, look) + pad * 2.0;
            if let Some(width) = widths.get_mut(c) {
                *width = width.max(size.x);
            }
            height = height.max(size.y);
        }
        heights.push(height);
    }
    (widths, heights)
}

/// Where in a column `width` wide the words of a cell `size` wide begin,
/// from its left edge, as the column lines them up, `pad` in from either
/// edge.
pub(crate) fn aligned(align: Align, width: f32, size: f32, pad: f32) -> f32 {
    match align {
        Align::None | Align::Left => pad,
        Align::Center => (width - size) / 2.0,
        Align::Right => width - size - pad,
    }
}

/// How the lines of a table are drawn and its header row filled, in a
/// text drawn in `colour`.
pub(crate) fn table_tones(colour: Color32) -> (Stroke, Color32) {
    (
        Stroke::new(1.0, colour.gamma_multiply(0.35)),
        colour.gamma_multiply(0.12),
    )
}

/// The room around a cell's words in a table, at full size.
const CELL_PAD: Vec2 = egui::vec2(6.0, 3.0);

/// Draws `table` of `text` as a grid, scaled down to the width there is,
/// and gives where its marks, tags and links lie and where it does.
fn table_label(ui: &mut Ui, text: &str, table: &Table, (found, look): (&Found, &Look)) -> Spots {
    let room = ui.available_width().max(1.0);
    let mut scale = 1.0f32;
    let mut scaled = look.clone();
    let mut laid = lay_table(ui, (text, table), (found, &scaled));
    let (mut widths, mut heights) = grid(ui, &laid, &scaled, CELL_PAD);
    // Smaller till it fits; a font does not shrink quite in proportion.
    for _ in 0..3 {
        let wide: f32 = widths.iter().sum();
        if wide <= room {
            break;
        }
        scale = (scale * room / wide).max(0.2);
        scaled = look.scaled(scale);
        laid = lay_table(ui, (text, table), (found, &scaled));
        (widths, heights) = grid(ui, &laid, &scaled, CELL_PAD * scale);
    }
    let pad = CELL_PAD * scale;
    let size = egui::vec2(widths.iter().sum(), heights.iter().sum());
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    // The cells' labels in a part of their own, which moves nothing after
    // the table.
    let mut cells = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .id_salt(("table", table.start)),
    );
    let (stroke, header) = table_tones(look.colour);
    let mut spots = Spots::default();
    let mut y = rect.top();
    for (r, row) in laid.iter().enumerate() {
        let height = heights.get(r).copied().unwrap_or_default();
        let mut x = rect.left();
        for (c, cell) in row.iter().enumerate() {
            let width = widths.get(c).copied().unwrap_or_default();
            let at = Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));
            if r == 0 && ui.is_rect_visible(at) {
                ui.painter().rect_filled(at, 0.0, header);
            }
            let align = table.aligns.get(c).copied().unwrap_or(Align::None);
            let words = cell.size(ui, &scaled);
            let left = aligned(align, width, words.x, pad.x);
            let pos = egui::pos2(x + left, y + pad.y);
            spots.extend(cell.paint(&mut cells, pos, scaled.colour, true));
            spots.cells.push(CellSpot {
                at: table.start,
                cell: (r, c),
                rect: at,
            });
            if ui.is_rect_visible(at) {
                ui.painter()
                    .rect_stroke(at, 0.0, stroke, egui::StrokeKind::Inside);
            }
            x += width;
        }
        y += height;
    }
    spots.tables.push(TableSpot {
        at: table.start,
        rect,
    });
    spots
}

/// Takes the tables `spots` of the text `of`, by id, as buttons over
/// what was drawn of them, read as "Open the table". Gives where the
/// table clicked begins in the text. Taken before the marks, tags and
/// links in them, those keep their clicks.
pub fn table_clicks(ui: &mut Ui, t: &L10n, of: Id, spots: &[TableSpot]) -> Option<usize> {
    let mut picked = None;
    for spot in spots {
        let resp = ui.interact(spot.rect, of.with(("table", spot.at)), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, t.table_open()));
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(t.table_open())
            .clicked()
        {
            picked = Some(spot.at);
        }
    }
    picked
}

/// Where the boxes, tags and links of `line`, laid out in `galley` at
/// `origin`, lie on the screen.
fn line_spots(galley: &Galley, origin: Pos2, line: &Line) -> Spots {
    let at =
        |chars: (usize, usize)| place_chars(galley, chars).map(|r| r.translate(origin.to_vec2()));
    Spots {
        boxes: line
            .boxes
            .iter()
            .filter_map(|(id, state, chars)| {
                Some(BoxSpot {
                    id: id.clone(),
                    state: *state,
                    rect: at(*chars)?,
                })
            })
            .collect(),
        tags: line
            .tags
            .iter()
            .filter_map(|(start, written, chars)| {
                Some(TagSpot {
                    start: *start,
                    written: written.clone(),
                    rect: at(*chars)?,
                })
            })
            .collect(),
        links: line
            .links
            .iter()
            .filter_map(|(start, url, chars)| {
                let rects: Vec<Rect> = row_rects(galley, *chars)
                    .into_iter()
                    .map(|r| r.translate(origin.to_vec2()))
                    .collect();
                if rects.is_empty() {
                    return None;
                }
                let label = galley
                    .job
                    .text
                    .chars()
                    .skip(chars.0)
                    .take(chars.1 - chars.0)
                    .collect();
                Some(LinkSpot {
                    start: *start,
                    url: url.clone(),
                    label,
                    rects,
                })
            })
            .collect(),
        tables: Vec::new(),
        cells: Vec::new(),
    }
}

/// Where the characters `start..end` of `galley` lie, a rectangle on each
/// row they are on.
fn row_rects(galley: &Galley, (start, end): (usize, usize)) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut at = 0;
    for row in &galley.rows {
        let count = row.char_count_excluding_newline().0;
        let (a, b) = (start.max(at), end.min(at + count));
        if a < b {
            let left = row.pos.x + row.x_offset(CharIndex(a - at));
            let right = row.pos.x + row.x_offset(CharIndex(b - at));
            out.push(Rect::from_min_max(
                egui::pos2(left, row.min_y()),
                egui::pos2(right.max(left + 4.0), row.max_y()),
            ));
        }
        at += row.char_count_including_newline().0;
    }
    out
}

/// Where a clicked app link waits for the window to follow it (#261).
fn app_link_id() -> Id {
    Id::new("app link clicked")
}

/// The app link clicked this frame, which the window follows once;
/// `None` when there is none.
pub fn take_app_link(ctx: &egui::Context) -> Option<AppLink> {
    ctx.data_mut(|d| d.remove_temp::<Option<AppLink>>(app_link_id()))
        .flatten()
}

/// Hands the app link `link` to the window, as a click on one in a text
/// does: [`take_app_link`] gives it once.
pub fn hand_app_link(ctx: &egui::Context, link: AppLink) {
    ctx.data_mut(|d| d.insert_temp(app_link_id(), Some(link)));
}

/// Takes the links `spots` of the text `of`, by id, as links over what
/// was drawn under them; a click opens a web or e-mail address in the
/// system's browser or mail program, and hands an app link (ADR 0046) to
/// the window ([`take_app_link`]): these texts are drawn in many places
/// that know nothing of the window. Only what `markdown::may_open`
/// allows is ever among them.
pub fn link_clicks(ui: &mut Ui, of: Id, spots: &[LinkSpot]) {
    for spot in spots {
        for (i, rect) in spot.rects.iter().enumerate() {
            let resp = ui.interact(*rect, of.with(("link", spot.start, i)), Sense::click());
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &spot.label));
            if resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(&spot.url)
                .clicked()
                && markdown::may_open(&spot.url)
            {
                match app_link::parse(&spot.url) {
                    Some(link) => hand_app_link(ui.ctx(), link),
                    None => ui.ctx().open_url(egui::OpenUrl::new_tab(&spot.url)),
                }
            }
        }
    }
}

/// The source `text` of a formatted text laid out for its edit field in
/// `font` and `colour`: every character where it is, its marks in `dim`
/// and its tags in `accent`, unwrapped. A table's pipes, its delimiter
/// row and the line marking its first column as headings are marks; so
/// are a code block's fences, its lines in the fixed-width font on the
/// background `code`. Gives with it what each line is, for
/// [`source_spread`].
pub fn source_job(
    text: &str,
    font: FontId,
    (colour, accent, dim, code): (Color32, Color32, Color32, Color32),
) -> (LayoutJob, Vec<SourceLine>) {
    // What each byte is: 0 words, 1 a mark, 2 a tag, 3 code.
    let mut kind = vec![0u8; text.len()];
    let mut mark = |a: usize, b: usize, k: u8| {
        for slot in kind.get_mut(a..b).unwrap_or_default() {
            *slot = k;
        }
    };
    fn marks_of(spans: &[Span], out: &mut Vec<(usize, usize)>) {
        for s in spans {
            match s.kind {
                SpanKind::Text => out.push((s.start, s.content_start)),
                SpanKind::Code | SpanKind::Bold | SpanKind::Italic | SpanKind::Strike => {
                    out.push((s.start, s.content_start));
                    out.push((s.content_end, s.end));
                }
                SpanKind::Link => {
                    out.push((s.start, s.content_start));
                    out.push((s.content_end, s.end));
                }
            }
            marks_of(&s.children, out);
        }
    }
    // A line's mark, or a table's line whole; then its cells' words
    // again; then the marks of its words.
    let (mut lines, mut cells, mut inline) = (Vec::new(), Vec::new(), Vec::new());
    let mut code_lines = Vec::new();
    let mut kinds = Vec::new();
    for block in markdown::parse(text) {
        kinds.push(match (&block.code, &block.table) {
            (Some(_), _) => SourceLine::Code,
            (None, Some(table)) => SourceLine::Table {
                first: block.line == table.first_line,
            },
            _ if text
                .get(block.start..block.end)
                .is_some_and(|l| l.trim().is_empty()) =>
            {
                SourceLine::Blank
            }
            _ => SourceLine::Words(block.kind),
        });
        if let Some(c) = &block.code {
            if c.is_fence(block.line) {
                lines.push((block.start, block.end));
            } else {
                code_lines.push((block.start, block.end));
            }
            continue;
        }
        match (&block.table, block.row) {
            (Some(table), row) => {
                lines.push((block.start, block.end));
                let row = row.and_then(|r| table.rows.get(r));
                cells.extend(
                    row.iter()
                        .flat_map(|r| r.cells.iter().map(|c| (c.start, c.end))),
                );
            }
            (None, _) => lines.push((block.start, block.content_start)),
        }
        marks_of(&block.spans, &mut inline);
    }
    for (a, b) in lines {
        mark(a, b, 1);
    }
    for (a, b) in cells {
        mark(a, b, 0);
    }
    for (a, b) in inline {
        mark(a, b, 1);
    }
    for (a, b) in code_lines {
        mark(a, b, 3);
    }
    for tag in markdown::tags(text) {
        mark(tag.start, tag.end, 2);
    }
    let format = |k: u8| match k {
        1 => TextFormat::simple(font.clone(), dim),
        2 => TextFormat::simple(font.clone(), accent),
        3 => {
            let mut format = TextFormat::simple(FontId::monospace(font.size * CODE_SCALE), colour);
            format.background = code;
            format
        }
        _ => TextFormat::simple(font.clone(), colour),
    };
    let mut job = LayoutJob::default();
    let mut run = 0;
    let mut run_kind = kind.first().copied().unwrap_or(0);
    for (i, _) in text.char_indices() {
        let k = kind.get(i).copied().unwrap_or(0);
        if k != run_kind {
            job.append(text.get(run..i).unwrap_or_default(), 0.0, format(run_kind));
            run = i;
            run_kind = k;
        }
    }
    job.append(text.get(run..).unwrap_or_default(), 0.0, format(run_kind));
    (job, kinds)
}

/// What a line of a formatted text's source is, for the room before it
/// in its edit field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceLine {
    /// Words of this kind.
    Words(BlockKind),
    /// Nothing but spaces.
    Blank,
    /// A line of a code block, its fences too.
    Code,
    /// A line of a table, its first one or another.
    Table { first: bool },
}

/// `galley`, laid out from a formatted text's source whose lines are
/// `lines`, its rows as far apart as the text reads (#327): `extra`
/// more than its font's rows, and before a line the room [`line_gap`]
/// gives in parts of `size`. A blank line only has the line height;
/// a code block's rows lie together on their background, and so do the
/// rows of a table, of which only the first has room before it, so a
/// grid drawn over them keeps its place.
pub fn source_spread(
    galley: Arc<Galley>,
    lines: &[SourceLine],
    size: f32,
    extra: f32,
) -> Arc<Galley> {
    let kind = |line: SourceLine| match line {
        SourceLine::Words(kind) => kind,
        SourceLine::Code => BlockKind::Code,
        SourceLine::Table { .. } | SourceLine::Blank => BlockKind::Table,
    };
    spread(galley, |line, first| {
        let at = lines.get(line).copied().unwrap_or(SourceLine::Blank);
        if !first {
            return match at {
                SourceLine::Code | SourceLine::Table { .. } => 0.0,
                _ => extra,
            };
        }
        let prev = line
            .checked_sub(1)
            .and_then(|l| lines.get(l))
            .copied()
            .unwrap_or(SourceLine::Blank);
        match (prev, at) {
            (SourceLine::Code, SourceLine::Code)
            | (SourceLine::Table { .. }, SourceLine::Table { first: false }) => 0.0,
            (SourceLine::Blank, _) | (_, SourceLine::Blank) => extra,
            (prev, at) => extra + line_gap(kind(prev), kind(at)) * size,
        }
    })
}

/// Where the bytes `start..end` of `text`, laid out in `galley` from its
/// bytes `from..`, lie in it; none while a cut-off galley does not show
/// them.
fn place(galley: &Galley, text: &str, from: usize, (start, end): (usize, usize)) -> Option<Rect> {
    let start_at = text.get(from..start)?.chars().count();
    let end_at = start_at + text.get(start..end)?.chars().count();
    place_chars(galley, (start_at, end_at))
}

/// Where the characters `start_at..end_at` of `galley` lie in it, on the
/// row the first is on; none while a cut-off galley does not show them.
fn place_chars(galley: &Galley, (start_at, end_at): (usize, usize)) -> Option<Rect> {
    // What a cut-off galley still shows of it, in characters.
    let shown = galley.end().index.0;
    if end_at > shown || (galley.elided && end_at >= shown) {
        return None;
    }
    let at = |index: usize| {
        galley.pos_from_cursor(CCursor {
            index: CharIndex(index),
            prefer_next_row: true,
        })
    };
    let (a, b) = (at(start_at), at(end_at));
    if b.min.y > a.min.y {
        // Broken over two rows: the part on the first.
        return Some(Rect::from_min_max(
            a.min,
            egui::pos2(galley.rect.max.x.max(a.min.x + 4.0), a.max.y),
        ));
    }
    Some(Rect::from_min_max(
        a.min,
        egui::pos2(b.max.x.max(a.min.x + 4.0), a.max.y),
    ))
}

/// Where the tags of `text` lie in `galley`, laid out at `origin` from
/// its bytes `shown`.
pub fn tag_spots(
    galley: &Arc<Galley>,
    origin: egui::Pos2,
    text: &str,
    (from, to): (usize, usize),
) -> Vec<TagSpot> {
    markdown::plain_tags(text)
        .into_iter()
        .filter(|tag| tag.start >= from && tag.end <= to)
        .filter_map(|tag| {
            let rect = place(galley, text, from, (tag.start, tag.end))?;
            Some(TagSpot {
                start: tag.start,
                written: text.get(tag.start..tag.end)?.to_string(),
                rect: rect.translate(origin.to_vec2()),
            })
        })
        .collect()
}

/// Takes the tags `spots` of the text `of`, by id, as links over what was
/// drawn under them, each read as "Search for" the tag. Gives the tag
/// clicked, as written.
pub fn tag_clicks(ui: &mut Ui, t: &L10n, of: Id, spots: &[TagSpot]) -> Option<String> {
    let mut picked = None;
    for spot in spots {
        let resp = ui.interact(spot.rect, of.with(("tag", spot.start)), Sense::click());
        let label = t.search_for_tag(&spot.written);
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &label));
        if resp.hovered() && ui.is_rect_visible(spot.rect) {
            let stroke = egui::Stroke::new(1.0, ui.visuals().hyperlink_color);
            let y = spot.rect.bottom();
            ui.painter().hline(spot.rect.x_range(), y, stroke);
        }
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            picked = Some(spot.written.clone());
        }
    }
    picked
}

/// The format the characters of a box's id are drawn in, in an edit
/// field: too small and too faint to be seen, with room after them for
/// what the box says (`gap`, in points).
fn hidden_format(format: &TextFormat, gap: f32) -> TextFormat {
    let mut hidden = format.clone();
    hidden.font_id = FontId::new(1.0, format.font_id.family.clone());
    hidden.color = Color32::TRANSPARENT;
    hidden.background = Color32::TRANSPARENT;
    hidden.underline = Stroke::NONE;
    hidden.strikethrough = Stroke::NONE;
    hidden.extra_letter_spacing = gap;
    hidden
}

/// `job`, laid out for an edit field from the source text it holds, with
/// every box drawn as the glyph of the state it shows by `look` (each of
/// the three glyphs is three bytes, so every offset stays) and its id
/// drawn invisibly, every character still there for the field's cursor.
/// The last character of an id leaves `gap` of the text it says after
/// it room.
pub fn field_job(mut job: LayoutJob, look: &BoxLook, gap: &dyn Fn(&str) -> f32) -> LayoutJob {
    let found = text_boxes::find(&job.text);
    if found.is_empty() {
        return job;
    }
    // Where each id lies, with what its box says after it.
    let mut hidden = Vec::new();
    for b in &found {
        let glyph = look.state(b).glyph();
        let width = glyph.len_utf8();
        let written = job.text.get(b.start..).and_then(|s| s.chars().next());
        if written.is_some_and(|c| c.len_utf8() == width) {
            job.text
                .replace_range(b.start..b.start + width, glyph.encode_utf8(&mut [0; 4]));
        }
        let after = look.after.get(&b.id).map_or(0.0, |a| gap(a));
        hidden.push((b.start + width, b.end, after));
    }
    let sections = std::mem::take(&mut job.sections);
    let mut out = Vec::with_capacity(sections.len() + hidden.len() * 2);
    for section in sections {
        let (from, to) = (section.byte_range.start.0, section.byte_range.end.0);
        let mut at = from;
        let mut piece = |a: usize, b: usize, format: TextFormat, lead: f32| {
            if a < b {
                out.push(egui::text::LayoutSection {
                    leading_space: lead,
                    byte_range: egui::text::ByteIndex(a)..egui::text::ByteIndex(b),
                    format,
                });
            }
        };
        for &(h_from, h_to, after) in &hidden {
            if h_to <= at || h_from >= to {
                continue;
            }
            let lead = if at == from {
                section.leading_space
            } else {
                0.0
            };
            piece(at, h_from.max(at), section.format.clone(), lead);
            let (a, b) = (h_from.max(at), h_to.min(to));
            // The last character of the id leaves the room.
            let last = text_last_char(&job.text, b);
            if b == h_to && after > 0.0 && last > a {
                piece(a, last, hidden_format(&section.format, 0.0), 0.0);
                piece(last, b, hidden_format(&section.format, after), 0.0);
            } else {
                piece(a, b, hidden_format(&section.format, 0.0), 0.0);
            }
            at = b;
        }
        let lead = if at == from {
            section.leading_space
        } else {
            0.0
        };
        piece(at, to, section.format, lead);
    }
    job.sections = out;
    job
}

/// Where the last character before the byte `end` of `text` begins.
fn text_last_char(text: &str, end: usize) -> usize {
    text.get(..end)
        .and_then(|s| s.char_indices().next_back())
        .map_or(end, |(i, _)| i)
}

/// Where the boxes of the source `text`, laid out in an edit field's
/// `galley` at `origin` with [`field_job`], lie on the screen, each as
/// `look` says it shows.
pub fn field_spots(galley: &Arc<Galley>, origin: Pos2, text: &str, look: &BoxLook) -> Vec<BoxSpot> {
    text_boxes::find(text)
        .into_iter()
        .filter_map(|b| {
            let rect = place(galley, text, 0, (b.start, b.id_start))?;
            Some(BoxSpot {
                state: look.state(&b),
                id: b.id,
                rect: rect.translate(origin.to_vec2()),
            })
        })
        .collect()
}

/// Paints what each box of the source `text` says after it, with Show
/// ticks on, into the room [`field_job`] left for it in an edit field's
/// `galley` at `origin`, in `font` and `colour`.
pub fn paint_after(
    ui: &Ui,
    (galley, origin): (&Arc<Galley>, Pos2),
    (text, look): (&str, &BoxLook),
    (font, colour): (FontId, Color32),
) {
    for b in text_boxes::find(text) {
        let Some(after) = look.after.get(&b.id) else {
            continue;
        };
        let Some(glyph) = place(galley, text, 0, (b.start, b.id_start)) else {
            continue;
        };
        let pos = origin + egui::vec2(glyph.right() + 2.0, glyph.center().y);
        ui.painter()
            .text(pos, egui::Align2::LEFT_CENTER, after, font.clone(), colour);
    }
}

/// What a click on a box asks: to tick it to a state, or to show its
/// History; each with the box's id.
#[derive(Debug, Clone, PartialEq)]
pub enum BoxAsk {
    Tick(String, BoxState),
    History(String),
}

/// Takes the boxes `spots` of the text `of`, by id, as widgets over what
/// was drawn under them: read as their state, and each a button opening
/// at the box, where the pointer is and where a screen reader's click
/// lands, Check, Fail and Open, the current state left out, while
/// `ticks`, and History while it was ticked (`ticked` holds its id).
/// Gives what was picked.
pub fn box_clicks(
    ui: &mut Ui,
    t: &L10n,
    of: Id,
    spots: &[BoxSpot],
    (ticks, ticked): (bool, &dyn Fn(&str) -> bool),
) -> Option<BoxAsk> {
    let mut picked = None;
    for spot in spots {
        let history = ticked(&spot.id);
        let opens = ticks || history;
        let sense = if opens {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui.interact(spot.rect, of.with(("box", &spot.id)), sense);
        let label = box_label(t, spot.state);
        let typ = if opens {
            WidgetType::Button
        } else {
            WidgetType::Label
        };
        resp.widget_info(|| WidgetInfo::labeled(typ, true, label));
        if !opens {
            continue;
        }
        if resp.hovered() && ui.is_rect_visible(spot.rect) {
            let stroke = ui.visuals().widgets.hovered.fg_stroke;
            ui.painter().rect_stroke(
                spot.rect.expand(1.0),
                3.0,
                stroke,
                egui::StrokeKind::Outside,
            );
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        Popup::menu(&resp).show(|ui| {
            let text = ui.visuals().text_color();
            if ticks {
                for to in BoxState::ALL {
                    if to != spot.state
                        && ui
                            .button(RichText::new(menu_entry(t, to)).color(text))
                            .clicked()
                    {
                        picked = Some(BoxAsk::Tick(spot.id.clone(), to));
                        ui.close();
                    }
                }
            }
            if history
                && ui
                    .button(RichText::new(t.history_title()).color(text))
                    .clicked()
            {
                picked = Some(BoxAsk::History(spot.id.clone()));
                ui.close();
            }
        });
    }
    picked
}

/// Where each box of the source `text` lies, in characters: its glyph's
/// first and the one after its id.
pub fn box_chars(text: &str) -> Vec<(usize, usize)> {
    text_boxes::find(text)
        .into_iter()
        .filter_map(|b| {
            let start = text.get(..b.start)?.chars().count();
            let end = start + text.get(b.start..b.end)?.chars().count();
            Some((start, end))
        })
        .collect()
}

/// The cursor at the character `at` kept out of every box of `boxes`
/// ([`box_chars`]): one inside a box, behind its glyph, goes to its end
/// when it came from before it (`from`, where it was), and before its
/// glyph otherwise.
pub fn kept_out(boxes: &[(usize, usize)], at: usize, from: usize) -> usize {
    match boxes.iter().find(|&&(start, end)| start < at && at < end) {
        Some(&(start, end)) => {
            if from <= start {
                end
            } else {
                start
            }
        }
        None => at,
    }
}

/// Where the character `at` of `old` is in `new`, the text it became:
/// counted through what both share, what `new` put in between (the ids
/// saving gave its boxes) skipped; past a change that is no putting in,
/// as far from the end as it was.
pub fn followed(old: &str, new: &str, at: usize) -> usize {
    let (mut a, mut b) = (old.chars().peekable(), new.chars());
    let mut i = 0;
    let mut j = 0;
    while i < at {
        let Some(&c) = a.peek() else {
            break;
        };
        match b.next() {
            Some(d) if d == c => {
                a.next();
                i += 1;
                j += 1;
            }
            Some(_) => j += 1,
            None => {
                let (old_n, new_n) = (old.chars().count(), new.chars().count());
                return (at + new_n).saturating_sub(old_n).min(new_n);
            }
        }
    }
    j
}

/// `text` without the characters of an id that belongs to no box, as a
/// cut through one leaves them; those of a flag's emoji stay.
pub fn without_stray_ids(text: &str) -> Option<String> {
    let kept: Vec<(usize, usize)> = text_boxes::find(text)
        .iter()
        .map(|b| (b.id_start, b.end))
        .collect();
    let tag = |c: char| ('\u{E0020}'..='\u{E007F}').contains(&c);
    let mut out = String::with_capacity(text.len());
    let mut stray = false;
    let mut flag = false;
    for (i, c) in text.char_indices() {
        if tag(c) {
            if flag || kept.iter().any(|&(a, b)| a <= i && i < b) {
                out.push(c);
            } else {
                stray = true;
            }
            continue;
        }
        // A black flag begins the tags of a subdivision's flag.
        flag = c == '\u{1F3F4}';
        out.push(c);
    }
    stray.then_some(out)
}

/// Keeps the boxes of the source text `text` of the edit field `id` whole
/// before the field shows: Backspace right after a box, or Delete right
/// before one, takes the whole box, its id with it, and while `follow`,
/// a text changed since the field last showed it ([`remember_text`]; a
/// save gave its boxes ids) keeps the cursor where it was among its
/// characters. Whether `text` changed.
pub fn guard_field(ui: &mut Ui, id: Id, text: &mut String, follow: bool) -> bool {
    use egui::text::CCursorRange;
    use egui::widgets::text_edit::TextEditState;
    let seen_id = id.with("box text seen");
    let seen: Option<String> = ui.data(|d| d.get_temp(seen_id));
    let focused = ui.memory(|m| m.has_focus(id));
    let mut state = match TextEditState::load(ui.ctx(), id) {
        Some(state) => state,
        None if focused => TextEditState::default(),
        None => return false,
    };
    // A field with the keyboard and no cursor yet has it at its end.
    let end = CCursorRange::one(CCursor::new(text.chars().count()));
    let mut range = match state.cursor.char_range() {
        Some(range) => range,
        None if focused => end,
        None => return false,
    };
    if let Some(seen) = seen.filter(|s| follow && s != text) {
        let follow = |c: CCursor| CCursor::new(followed(&seen, text, c.index.0));
        range = CCursorRange::two(follow(range.secondary), follow(range.primary));
        state.cursor.set_char_range(Some(range));
        state.clone().store(ui.ctx(), id);
    }
    if !focused || range.primary != range.secondary {
        return false;
    }
    let at = range.primary.index.0;
    let boxes = text_boxes::find(text);
    let chars = box_chars(text);
    let (back, forward) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::Backspace) && i.modifiers.is_none(),
            i.key_pressed(egui::Key::Delete) && i.modifiers.is_none(),
        )
    });
    let hit = chars
        .iter()
        .zip(&boxes)
        .find(|((start, end), _)| (back && *end == at) || (forward && *start == at));
    let Some((&(start, _), found)) = hit else {
        return false;
    };
    let key = if back {
        egui::Key::Backspace
    } else {
        egui::Key::Delete
    };
    ui.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    text.replace_range(found.start..found.end, "");
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(start))));
    state.store(ui.ctx(), id);
    true
}

/// Remembers `text` as the edit field `id` holds it at the end of this
/// frame, for [`guard_field`].
pub fn remember_text(ui: &Ui, id: Id, text: &str) {
    ui.data_mut(|d| d.insert_temp(id.with("box text seen"), text.to_string()));
}

/// Keeps the cursor of the edit field `id`, after it showed the source
/// text `text`, out of the ids of its boxes, and cuts the stray parts of
/// an id a cut through a box left.
pub fn guard_cursor(ui: &mut Ui, id: Id, text: &mut String) {
    use egui::text::CCursorRange;
    use egui::widgets::text_edit::TextEditState;
    if let Some(cleaned) = without_stray_ids(text) {
        *text = cleaned;
    }
    let Some(mut state) = TextEditState::load(ui.ctx(), id) else {
        return;
    };
    let Some(range) = state.cursor.char_range() else {
        return;
    };
    let was_id = id.with("box cursor was");
    let was: usize = ui.data(|d| d.get_temp(was_id)).unwrap_or(0);
    let chars = box_chars(text);
    let primary = kept_out(&chars, range.primary.index.0, was);
    let secondary = kept_out(&chars, range.secondary.index.0, was);
    ui.data_mut(|d| d.insert_temp(was_id, primary));
    if (primary, secondary) != (range.primary.index.0, range.secondary.index.0) {
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(secondary),
            CCursor::new(primary),
        )));
        state.store(ui.ctx(), id);
    }
}

#[cfg(test)]
mod tests;
