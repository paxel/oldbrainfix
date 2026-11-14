//! The tables of a formatted text edited as grids in place of their lines
//! (#257, ADR 0033), in the Note dialog's text fields and the Amend field
//! while they are edited. The field keeps the Markdown: a table's lines
//! are laid out invisibly, its first line as high as the grid, which is
//! then drawn and taken over them. The field's cursor never stays in a
//! table's lines: an arrow key that steps into them enters the grid, from
//! before at the header row's first cell, from after at the last cell's
//! end; anything else that puts it there edits the cell it is in.
//!
//! A click on a cell edits it in place on one line, its boxes as they
//! show and their ids kept out of reach (ADR 0035). Tab and Shift+Tab go
//! to the next and the previous cell row by row, Enter to the cell below,
//! Left and Right at a cell's edge and Up and Down to the cell beside it;
//! past the grid's edge the cursor goes into the text before or after the
//! table, Esc back into the field. The "+" under the rows' numbers and
//! after the columns' letters adds one; a row's number and a column's
//! letter open their menus, as in the table full screen, a column's also
//! with its alignment. A column's border drags it wider or narrower for
//! as long as the editor shows the table: Markdown has no widths. Every
//! change writes the whole table through the core at once, the cell's own
//! words the truth while it is edited. "Show as lines" shows a table's
//! lines as text in the field until the cursor leaves them or the field
//! the keyboard; "Show as grid" beside them brings the grid back.

use std::collections::BTreeMap;
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange, LayoutJob, LayoutSection};
use egui::widgets::text_edit::{TextEditOutput, TextEditState};
use egui::{
    Color32, FontId, Frame, Id, Key, Modifiers, Popup, Rect, Sense, Stroke, TextEdit, TextFormat,
    Ui, Vec2, WidgetInfo, WidgetType,
};
use oldbrainfix_core::markdown::Change;
use oldbrainfix_core::markdown::table::{self, Align, Table};
use oldbrainfix_core::text_format;

use crate::format_bar::{self, Format};
use crate::l10n::L10n;
use crate::table_view::{self, Deleting, Op};
use crate::text_marks_view::{self, BoxAsk, BoxLook, BoxSpot, Found, LaidCell, Look};
use crate::theme::Tones;

/// The room around a cell's words.
const PAD: Vec2 = egui::vec2(8.0, 4.0);
/// The narrowest and, unless dragged wider, the widest a column is drawn.
const NARROWEST: f32 = 48.0;
const WIDEST: f32 = 320.0;
/// The narrowest a column is dragged.
const DRAGGED: f32 = 24.0;
/// How wide the rows' numbers are, how high the columns' letters, and
/// how wide and high the "+" after them.
const NUMBERS: f32 = 30.0;
const LETTERS: f32 = 22.0;
const PLUS: f32 = 26.0;
/// The room above and under a grid.
const GAP: f32 = 4.0;
/// How far from a column's border a drag takes it.
const BORDER: f32 = 4.0;

/// Whether this Device ticks the boxes shown, and which were ticked.
pub type Ticks<'a> = (bool, &'a dyn Fn(&str) -> bool);

/// How a field's grids look: its text's size and colour, and how its
/// boxes show.
#[derive(Clone, Copy)]
pub struct GridLook<'a> {
    pub size: f32,
    pub colour: Color32,
    pub boxes: &'a BoxLook,
}

/// What the grids of a field kept between frames, in egui's memory under
/// the field's id.
#[derive(Clone, Default)]
struct State {
    editing: Option<Editing>,
    /// The tables shown as lines, by their place among the text's tables,
    /// each with whether the field has had the keyboard since.
    raw: BTreeMap<usize, bool>,
    /// The widths dragged, by table and column.
    widths: BTreeMap<usize, Vec<Option<f32>>>,
    /// The row or column of a table whose delete is asked about.
    deleting: Option<(usize, Deleting)>,
    /// The text as the grids last saw it.
    seen: String,
    /// Where the field's cursor was, in characters.
    was: Option<usize>,
    /// Whether a cell, a menu or the question had the keyboard.
    held: bool,
    /// Whether a menu of the grids was open.
    menu: bool,
    /// Whether the question was answered in the last frame: the field
    /// takes the keyboard back once the question is gone.
    answered: bool,
}

/// The cell edited: the table's place among the text's tables, its row
/// and column, its words as typed, the selection to give it once its
/// field shows, in characters, and the text as it was when it began.
#[derive(Clone)]
struct Editing {
    table: usize,
    row: usize,
    column: usize,
    draft: String,
    place: Option<(usize, usize)>,
    before: String,
}

/// What the grids did in a frame.
#[derive(Debug, Default)]
pub struct Outcome {
    /// The keyboard is in the field, a cell, a menu or the question of
    /// its grids: the field is still being edited.
    pub holds: bool,
    /// The keyboard left a cell or a menu of the grids for nothing of the
    /// field, as when the field loses it.
    pub left: bool,
    /// A box in a cell was clicked.
    pub boxes: Option<BoxAsk>,
    /// The link asked for on the words of the cell edited: their bytes in
    /// the text and the address of the link they are in.
    pub link: Option<(usize, usize, Option<String>)>,
    /// The format picked went to the cell edited.
    pub took: bool,
}

/// One table of a field's text this frame: as lines, or laid out as a
/// grid.
struct Shown {
    table: Arc<Table>,
    grid: Option<Laid>,
}

/// A table's cells laid out, each column's width and each row's height,
/// and how high the grid is.
struct Laid {
    cells: Vec<Vec<LaidCell>>,
    widths: Vec<f32>,
    heights: Vec<f32>,
    height: f32,
}

impl Laid {
    fn width(&self) -> f32 {
        NUMBERS + self.widths.iter().sum::<f32>() + PLUS
    }
}

/// The grids of a field's text in a frame.
pub struct Grids {
    id: Id,
    /// Where its state is kept: by the field and whose text it holds, so
    /// another Note's text in the same field starts afresh.
    key: Id,
    state: State,
    shown: Vec<Shown>,
    size: f32,
    colour: Color32,
    room: f32,
}

impl Grids {
    /// The grids of the field `id` holding the formatted text `text` of
    /// the Note or Section `of`, laid out in `look` for the width there
    /// is, before the field shows.
    pub fn prepare(ui: &mut Ui, (id, of): (Id, &str), text: &str, look: GridLook<'_>) -> Grids {
        let key = state_id(id, of);
        let mut state: State = ui.data(|d| d.get_temp(key)).unwrap_or_default();
        let focused = ui.memory(|m| m.has_focus(id));
        // A cell is edited while it has the keyboard or is about to; one
        // left behind, as the dialog closed, is no longer.
        let cell = cell_id(id);
        let keyboard = ui.memory(|m| m.has_focus(cell) || m.had_focus_last_frame(cell));
        if state.editing.as_ref().is_some_and(|e| e.place.is_none())
            && !keyboard
            && !state.menu
            && state.deleting.is_none()
        {
            state.editing = None;
        }
        // Changed from outside, say by a save giving its boxes ids or a
        // link set: the cell edited reads its words again, unless they
        // are those typed.
        if state.seen != text
            && let Some(editing) = &mut state.editing
        {
            let start = table::all(text).get(editing.table).map(|t| t.start);
            match start.and_then(|s| table::cell(text, s, editing.row, editing.column)) {
                Some(cell) if cell != written(&editing.draft) => editing.draft = cell,
                Some(_) => {}
                None => state.editing = None,
            }
        }
        state.seen = text.to_string();
        // A table shown as lines shows as a grid again once the cursor
        // left its lines or the field the keyboard.
        let tables = table::all(text);
        let cursor = field_cursor(ui, id);
        state.raw.retain(|&index, settled| {
            let Some(table) = tables.get(index) else {
                return false;
            };
            if !*settled {
                *settled = focused;
                return true;
            }
            let (from, to) = chars_of(text, table);
            focused
                && cursor.is_some_and(|(a, b)| (from..=to).contains(&a) && (from..=to).contains(&b))
        });
        let mut grids = Grids {
            id,
            key,
            state,
            shown: Vec::new(),
            size: look.size,
            colour: look.colour,
            room: ui.available_width(),
        };
        grids.lay(ui, text, look.boxes);
        grids
    }

    /// Lays out every table of `text` not shown as lines.
    fn lay(&mut self, ui: &Ui, text: &str, boxes: &BoxLook) {
        let look = Look::of(ui, (Some(self.size), self.colour));
        let found = Found::of(text, text_format::MARKDOWN, boxes);
        let lowest = ui.fonts_mut(|f| f.row_height(&FontId::proportional(self.size))) + PAD.y * 2.0;
        self.shown = table::all(text)
            .into_iter()
            .enumerate()
            .map(|(index, table)| {
                if self.state.raw.contains_key(&index) {
                    return Shown { table, grid: None };
                }
                let cells = text_marks_view::lay_table(ui, (text, &table), (&found, &look));
                let (mut widths, mut heights) = text_marks_view::grid(ui, &cells, &look, PAD);
                let dragged = self.state.widths.get(&index);
                let room = (self.room - NUMBERS - PLUS).max(NARROWEST);
                let natural: f32 = widths.iter().map(|w| w.clamp(NARROWEST, WIDEST)).sum();
                // Narrower to fit the field, but never narrower than the
                // narrowest; a width dragged stays as dragged.
                let fit = if natural > room { room / natural } else { 1.0 };
                for (c, width) in widths.iter_mut().enumerate() {
                    let drawn = (width.clamp(NARROWEST, WIDEST) * fit).max(NARROWEST);
                    *width = dragged
                        .and_then(|w| w.get(c).copied().flatten())
                        .unwrap_or(drawn);
                }
                for height in &mut heights {
                    *height = height.max(lowest);
                }
                let height = GAP + LETTERS + heights.iter().sum::<f32>() + PLUS + GAP;
                let grid = Laid {
                    cells,
                    widths,
                    heights,
                    height,
                };
                Shown {
                    table,
                    grid: Some(grid),
                }
            })
            .collect();
    }

    /// Whether a cell of these grids has the keyboard.
    pub fn cell_focused(&self, ui: &Ui) -> bool {
        self.state.editing.is_some() && ui.memory(|m| m.has_focus(cell_id(self.id)))
    }

    /// The words of the cell edited and their selection, in bytes, for
    /// the format bar.
    pub fn cell_selection(&self, ui: &Ui) -> Option<(String, (usize, usize))> {
        let editing = self.state.editing.as_ref()?;
        let selected = format_bar::selection(ui, cell_id(self.id), &editing.draft);
        Some((editing.draft.clone(), selected))
    }

    /// `job`, laid out from the source `typed`, with the lines of every
    /// table shown as a grid invisible and as high together as the grid,
    /// which is drawn over them.
    pub fn hide(&self, typed: &str, mut job: LayoutJob) -> LayoutJob {
        for (index, table) in table::all(typed).iter().enumerate() {
            let shown = self.shown.get(index);
            if shown.is_some_and(|s| s.grid.is_none()) {
                continue;
            }
            let height = shown
                .and_then(|s| s.grid.as_ref())
                .map_or(PLUS, |g| g.height);
            let first = typed
                .get(table.start..)
                .and_then(|s| s.chars().next())
                .map_or(table.start, |c| table.start + c.len_utf8());
            hide_range(&mut job, (table.start, first, table.end), height);
        }
        job
    }

    /// Whether `rect` lies on one of the grids drawn in `galley`.
    fn covers(&self, rects: &[Rect], rect: Rect) -> bool {
        rects.iter().any(|r| r.intersects(rect))
    }

    /// The boxes of the field drawn outside its grids, which take their
    /// own.
    pub fn outside(
        &self,
        output: &TextEditOutput,
        text: &str,
        spots: Vec<BoxSpot>,
    ) -> Vec<BoxSpot> {
        let rects = self.rects(output, text);
        spots
            .into_iter()
            .filter(|s| !self.covers(&rects, s.rect))
            .collect()
    }

    /// `look` without what the boxes in the grids' cells say after them,
    /// which the field would draw over the grids.
    pub fn look_outside(&self, text: &str, look: &BoxLook) -> BoxLook {
        let mut outside = look.clone();
        for shown in &self.shown {
            if shown.grid.is_none() {
                continue;
            }
            for found in oldbrainfix_core::text_boxes::find(text) {
                if shown.table.start <= found.start && found.end <= shown.table.end {
                    outside.after.remove(&found.id);
                }
            }
        }
        outside
    }

    /// Where each grid lies over the field `output` shows `text` in.
    fn rects(&self, output: &TextEditOutput, text: &str) -> Vec<Rect> {
        let field = output.response.response.rect;
        self.shown
            .iter()
            .map(|shown| match &shown.grid {
                Some(grid) => {
                    let (from, _) = chars_of(text, &shown.table);
                    let top = output.galley.pos_from_cursor(CCursor::new(from)).min.y;
                    Rect::from_min_size(
                        egui::pos2(field.left(), output.galley_pos.y + top),
                        egui::vec2(field.width().max(grid.width()), grid.height),
                    )
                }
                None => Rect::NOTHING,
            })
            .collect()
    }

    /// Draws the grids over the field `output` showed `text` in and takes
    /// what is done to them; `picked` is the format bar's button clicked,
    /// `ticks` whether this Device ticks boxes and which were ticked.
    pub fn show(
        mut self,
        ui: &mut Ui,
        (t, tones): (&L10n, Tones),
        (output, text): (&TextEditOutput, &mut String),
        (picked, boxes, ticks): (Option<Format>, &BoxLook, Ticks<'_>),
    ) -> Outcome {
        let mut outcome = Outcome::default();
        // What is typed or deleted around a table shown as a grid leaves
        // it whole, as on the phone (`keepTables`).
        if self.state.seen != *text {
            let raw: Vec<usize> = self.state.raw.keys().copied().collect();
            if let Some((kept, at)) = keep_tables(&self.state.seen, text, &raw) {
                *text = kept;
                format_bar::select(ui, self.id, text, (at, at));
            }
        }
        if self.state.seen != *text {
            // Typed into the field meanwhile.
            self.lay(ui, text, boxes);
            self.state.seen = text.clone();
        }
        self.snap(ui, text);
        if self.state.seen != *text {
            self.lay(ui, text, boxes);
            self.state.seen = text.clone();
            ui.ctx().request_repaint();
        }
        if let Some(format) = picked
            && self.state.editing.is_some()
        {
            outcome.took = true;
            outcome.link = self.format_cell(ui, text, format);
        }
        let rects = self.rects(output, text);
        let mut menu = false;
        for index in 0..self.shown.len() {
            let Some(&rect) = rects.get(index) else {
                continue;
            };
            if self.shown.get(index).is_some_and(|s| s.grid.is_none()) {
                self.raw_button(ui, t, (output, &*text), index);
                continue;
            }
            let drawn = self.grid(ui, (t, tones), (index, rect), text, (boxes, ticks));
            menu |= drawn.menu;
            if drawn.boxes.is_some() {
                outcome.boxes = drawn.boxes;
            }
        }
        let answered = std::mem::take(&mut self.state.answered);
        if answered {
            self.back_to_field(ui, text);
        }
        if let Some((index, deleting)) = self.state.deleting {
            match table_view::question(ui.ctx(), t, tones, deleting) {
                Some(true) => {
                    self.state.deleting = None;
                    self.state.answered = true;
                    self.delete(ui, text, index, deleting);
                }
                Some(false) => {
                    self.state.deleting = None;
                    self.state.answered = true;
                }
                None => {}
            }
        }
        // A menu closed with nothing picked gives the keyboard back.
        if self.state.menu && !menu && ui.memory(|m| m.focused().is_none()) {
            self.give_back(ui, text);
        }
        let focused = ui.memory(|m| m.focused());
        let held = self.state.deleting.is_some()
            || answered
            || self.state.answered
            || menu
            || self
                .state
                .editing
                .as_ref()
                .is_some_and(|e| e.place.is_some())
            || focused == Some(cell_id(self.id));
        outcome.holds = held || focused == Some(self.id);
        outcome.left = self.state.held && !outcome.holds && picked.is_none();
        self.state.held = held;
        self.state.menu = menu;
        self.state.was = field_cursor(ui, self.id).map(|(a, _)| a);
        self.state.seen = text.clone();
        ui.data_mut(|d| d.insert_temp(self.key, self.state));
        outcome
    }

    /// Keeps the field's cursor out of the lines of the tables shown as
    /// grids: an arrow key stepping into them enters the grid, anything
    /// else that puts it there edits the cell it is in; one end of a
    /// selection in them goes to the table's far edge.
    fn snap(&mut self, ui: &mut Ui, text: &mut String) {
        if !ui.memory(|m| m.has_focus(self.id)) {
            return;
        }
        let Some(mut state) = TextEditState::load(ui.ctx(), self.id) else {
            return;
        };
        let Some(range) = state.cursor.char_range() else {
            return;
        };
        let (primary, secondary) = (range.primary.index.0, range.secondary.index.0);
        let was = self.state.was.unwrap_or(primary);
        let arrows = ui.input(|i| {
            let down = i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowRight);
            let up = i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::ArrowLeft);
            (down, up)
        });
        let grids: Vec<(usize, Arc<Table>)> = self
            .shown
            .iter()
            .enumerate()
            .filter(|(_, s)| s.grid.is_some())
            .map(|(i, s)| (i, s.table.clone()))
            .collect();
        for (index, table) in grids {
            let (from, to) = chars_of(text, &table);
            if primary != secondary {
                // A selection takes the table whole or not at all.
                let out = |end: usize, other: usize| {
                    if from < end && end < to {
                        if other < from { to } else { from }
                    } else {
                        end
                    }
                };
                let (p, s) = (out(primary, secondary), out(secondary, primary));
                let one_cell = cell_of(text, &table, primary) == cell_of(text, &table, secondary);
                if (from..=to).contains(&p) && (from..=to).contains(&s) && one_cell {
                    // Both in one cell, as a link set there: edit it so.
                    let cell = cell_of(text, &table, primary);
                    let at = |c: usize| draft_char(text, &table, cell, c);
                    self.start(ui, text, index, cell, (at(secondary), at(primary)));
                    return;
                }
                if (p, s) != (primary, secondary) {
                    state
                        .cursor
                        .set_char_range(Some(CCursorRange::two(CCursor::new(s), CCursor::new(p))));
                    state.clone().store(ui.ctx(), self.id);
                }
                continue;
            }
            if !(from..=to).contains(&primary) {
                continue;
            }
            let last = last_cell(&table);
            let cell = if arrows.0 && was < from {
                (0, 0)
            } else if arrows.1 && was > to {
                last
            } else if primary == to {
                // Put at the table's end, as after a paste: after it.
                self.leave(ui, text, index, true);
                return;
            } else {
                cell_of(text, &table, primary)
            };
            // The key that stepped in moves nothing in the cell.
            ui.input_mut(|i| {
                for key in [
                    Key::ArrowUp,
                    Key::ArrowDown,
                    Key::ArrowLeft,
                    Key::ArrowRight,
                ] {
                    i.consume_key(Modifiers::NONE, key);
                }
            });
            // The field keeps its cursor where it came from, for the way
            // back into it.
            if !(from..=to).contains(&was) {
                state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(was))));
                state.clone().store(ui.ctx(), self.id);
            }
            let place = if cell == last && (arrows.1 || primary == to) {
                usize::MAX
            } else if cell == (0, 0) && arrows.0 {
                0
            } else {
                draft_char(text, &table, cell, primary)
            };
            self.start(ui, text, index, cell, (place, place));
            return;
        }
    }

    /// Edits the cell `(row, column)` of the table `index`, the cell
    /// edited before left, with the characters `place` of its words
    /// selected (`usize::MAX` its end).
    fn start(
        &mut self,
        ui: &Ui,
        text: &str,
        index: usize,
        (row, column): (usize, usize),
        place: (usize, usize),
    ) {
        self.finish(ui, text);
        let Some(table) = table::all(text).get(index).cloned() else {
            return;
        };
        let draft = table::cell(text, table.start, row, column).unwrap_or_default();
        let end = draft.chars().count();
        let place = (place.0.min(end), place.1.min(end));
        fresh_cell(ui, cell_id(self.id), &draft);
        self.state.editing = Some(Editing {
            table: index,
            row,
            column,
            draft,
            place: Some(place),
            before: text.to_string(),
        });
    }

    /// Ends editing the cell, the change it made one step back for Ctrl+Z
    /// in the field.
    fn finish(&mut self, ui: &Ui, text: &str) {
        if let Some(editing) = self.state.editing.take()
            && editing.before != text
        {
            undoable(ui, self.id, &editing.before);
        }
    }

    /// Leaves the grids for the field: its cursor before the table
    /// `index`, or after it while `after`, on a line where what is typed
    /// stays out of it (a line break or two put in where there is none).
    fn leave(&mut self, ui: &Ui, text: &mut String, index: usize, after: bool) {
        self.finish(ui, text);
        let Some(table) = table::all(text).get(index).cloned() else {
            select_chars(ui, self.id, (0, 0));
            return;
        };
        let (left, at) = if after {
            leave_after(text, &table)
        } else {
            leave_before(text, &table)
        };
        *text = left;
        self.state.was = Some(text.get(..at).map_or(0, |s| s.chars().count()));
        format_bar::select(ui, self.id, text, (at, at));
    }

    /// Gives the keyboard back to the cell edited, or else to the field.
    fn give_back(&mut self, ui: &Ui, text: &mut String) {
        match &mut self.state.editing {
            Some(editing) => {
                let end = editing.draft.chars().count();
                editing.place = Some((end, end));
            }
            None => self.back_to_field(ui, text),
        }
    }

    /// Gives the field the keyboard, its cursor where it was while that is
    /// outside the tables shown as grids; else at the text's end or start
    /// outside them, or after the last table.
    fn back_to_field(&mut self, ui: &Ui, text: &mut String) {
        let tables: Vec<(usize, usize)> = table::all(text)
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.state.raw.contains_key(i))
            .map(|(_, t)| chars_of(text, t))
            .collect();
        let outside = |at: usize| !tables.iter().any(|&(a, b)| (a..=b).contains(&at));
        let end = text.chars().count();
        let at = match field_cursor(ui, self.id) {
            Some((a, b)) if outside(a) && outside(b) => {
                ui.memory_mut(|m| m.request_focus(self.id));
                return;
            }
            _ if outside(end) => end,
            _ if outside(0) => 0,
            _ => {
                let last = table::all(text).len().saturating_sub(1);
                self.leave(ui, text, last, true);
                return;
            }
        };
        self.state.was = Some(at);
        select_chars(ui, self.id, (at, at));
    }

    /// Puts `format` on the words of the cell edited, as on the field's;
    /// a line format and a table do nothing there. Gives the link asked
    /// for.
    fn format_cell(
        &mut self,
        ui: &Ui,
        text: &mut String,
        format: Format,
    ) -> Option<(usize, usize, Option<String>)> {
        let cell = cell_id(self.id);
        let editing = self.state.editing.as_mut()?;
        let selected = format_bar::selection(ui, cell, &editing.draft);
        let end = editing.draft.chars().count();
        match format {
            Format::Mark | Format::Bold | Format::Italic | Format::Strike | Format::Code => {
                if let Some(changed) = format_bar::change(format, &editing.draft, selected) {
                    editing.draft = changed.text;
                    let chars =
                        |b: usize| editing.draft.get(..b).map_or(end, |s| s.chars().count());
                    editing.place = Some((chars(changed.start), chars(changed.end)));
                    let (index, row, column) = (editing.table, editing.row, editing.column);
                    let draft = editing.draft.clone();
                    write_cell(ui, self.id, text, (index, row, column), &draft);
                } else {
                    let at = format_bar::selection(ui, cell, &editing.draft);
                    let chars =
                        |b: usize| editing.draft.get(..b).map_or(end, |s| s.chars().count());
                    editing.place = Some((chars(at.0), chars(at.1)));
                }
                None
            }
            Format::Link => {
                let at = format_bar::At::of(&editing.draft, selected, true);
                let tables = table::all(text);
                let table = tables.get(editing.table)?;
                let cell = table.rows.get(editing.row)?.cells.get(editing.column)?;
                let (start, end) = (
                    text_byte(&editing.draft, cell.start, selected.0),
                    text_byte(&editing.draft, cell.start, selected.1),
                );
                // The cell is left for the link's dialog; setting it
                // selects the link, which edits the cell again.
                self.finish(ui, text);
                Some((start, end, at.url))
            }
            _ => {
                let at = |b: usize| editing.draft.get(..b).map_or(end, |s| s.chars().count());
                editing.place = Some((at(selected.0), at(selected.1)));
                None
            }
        }
    }

    /// The button "Show as grid" beside the first line of the table
    /// `index`, shown as lines in the field `output` shows `text` in.
    fn raw_button(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        (output, text): (&TextEditOutput, &str),
        index: usize,
    ) {
        let Some(table) = self.shown.get(index).map(|s| s.table.clone()) else {
            return;
        };
        let (from, _) = chars_of(text, &table);
        let row = output.galley.pos_from_cursor(CCursor::new(from));
        let field = output.response.response.rect;
        let y = output.galley_pos.y + row.center().y;
        let id = self.id.with(("table show grid", index));
        let words = t.table_show_grid();
        let size = small_size(ui, words);
        let rect = Rect::from_center_size(egui::pos2(field.right() - size.x / 2.0, y), size);
        if small_button(ui, rect, id, words).clicked() {
            self.state.raw.remove(&index);
            let cursor = field_cursor(ui, self.id).map_or(0, |(a, _)| a);
            let cell = cell_of(text, &table, cursor);
            let place = draft_char(text, &table, cell, cursor);
            self.start(ui, text, index, cell, (place, place));
        }
    }

    /// Draws the grid of the table `index` in `rect` and takes what is
    /// done to it.
    fn grid(
        &mut self,
        ui: &mut Ui,
        (t, tones): (&L10n, Tones),
        (index, rect): (usize, Rect),
        text: &mut String,
        (boxes, ticks): (&BoxLook, Ticks<'_>),
    ) -> Drawn {
        let mut drawn = Drawn::default();
        let Some((table, widths, heights)) = self.shown.get(index).and_then(|s| {
            let grid = s.grid.as_ref()?;
            Some((s.table.clone(), grid.widths.clone(), grid.heights.clone()))
        }) else {
            return drawn;
        };
        let (rows, columns) = (table.rows.len(), table.columns());
        let of = self.id.with(("table grid", index));
        let colour = self.colour;
        let look = Look::of(ui, (Some(self.size), colour));
        let origin = rect.min + egui::vec2(0.0, GAP);
        let xs = starts(NUMBERS, &widths);
        let ys = starts(LETTERS, &heights);
        let cell_rect = |r: usize, c: usize| {
            let x = xs.get(c).copied().unwrap_or(0.0);
            let y = ys.get(r).copied().unwrap_or(0.0);
            Rect::from_min_size(
                origin + egui::vec2(x, y),
                egui::vec2(
                    widths.get(c).copied().unwrap_or(0.0),
                    heights.get(r).copied().unwrap_or(0.0),
                ),
            )
        };
        let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        clipped.set_clip_rect(rect.intersect(ui.clip_rect()));
        let ui = &mut clipped;
        // A click beside the grid, in its rows, goes on after the table.
        let beside = ui.interact(rect, of.with("beside"), Sense::click());
        let mut out = beside.clicked().then_some(true);
        let (stroke, header) = text_marks_view::table_tones(colour);
        let editing = self
            .state
            .editing
            .as_ref()
            .filter(|e| e.table == index)
            .map(|e| (e.row, e.column));
        let mut picked: Option<(usize, usize)> = None;
        for r in 0..rows {
            for c in 0..columns {
                let at = cell_rect(r, c);
                if ui.is_rect_visible(at) {
                    ui.painter().rect_filled(at, 0.0, tones.card);
                    if r == 0 || (table.heading_column && c == 0) {
                        ui.painter().rect_filled(at, 0.0, header);
                    }
                }
                if editing == Some((r, c)) {
                    continue;
                }
                let align = table.aligns.get(c).copied().unwrap_or(Align::None);
                let laid = self
                    .shown
                    .get(index)
                    .and_then(|s| s.grid.as_ref())
                    .and_then(|g| g.cells.get(r))
                    .and_then(|row| row.get(c));
                let spots = laid
                    .map(|laid| paint_cell(ui, laid, at, align, &look))
                    .unwrap_or_default();
                if ui.is_rect_visible(at) {
                    ui.painter()
                        .rect_stroke(at, 0.0, stroke, egui::StrokeKind::Inside);
                }
                let resp = ui.interact(at, of.with(("cell", r, c)), Sense::click());
                let words = table
                    .rows
                    .get(r)
                    .and_then(|row| row.cells.get(c))
                    .map(|cell| oldbrainfix_core::text_boxes::strip(&cell.visible()))
                    .unwrap_or_default();
                let named = t.table_cell(r as i64 + 1, &table_view::letter(c));
                let label = if words.is_empty() {
                    named
                } else {
                    format!("{named}: {words}")
                };
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                if resp.on_hover_cursor(egui::CursorIcon::Text).clicked() {
                    picked = Some((r, c));
                }
                // Its boxes tick, over it.
                let marked = of.with(("cell marks", r, c));
                if let Some(asked) = text_marks_view::box_clicks(ui, t, marked, &spots.boxes, ticks)
                {
                    drawn.boxes = Some(asked);
                }
            }
        }
        // The rows' numbers, and the "+" under them.
        let weak = ui.visuals().weak_text_color();
        let mut op: Option<(Op, bool, usize)> = None;
        for r in 0..rows {
            let y = ys.get(r).copied().unwrap_or(0.0);
            let at = Rect::from_min_size(
                origin + egui::vec2(0.0, y),
                egui::vec2(NUMBERS, heights.get(r).copied().unwrap_or(0.0)),
            );
            let label = t.table_row(r as i64 + 1);
            let shown = (r + 1).to_string();
            let id = of.with(("row", r));
            let resp = table_view::heading(ui, at, (&shown, &label), id, (tones, weak));
            if let Some(picked) = table_view::menu(t, &resp, (false, r, rows), (false, None)) {
                op = Some((picked, false, r));
            }
            drawn.menu |= menu_open(ui, &resp);
        }
        let bottom = ys.last().copied().unwrap_or(LETTERS);
        let at = Rect::from_min_size(origin + egui::vec2(0.0, bottom), egui::vec2(NUMBERS, PLUS));
        let label = ("+", t.table_add_row());
        let add_row = table_view::heading(ui, at, label, of.with("add row"), (tones, weak));
        // The columns' letters, and the "+" after them.
        for c in 0..columns {
            let x = xs.get(c).copied().unwrap_or(0.0);
            let at = Rect::from_min_size(
                origin + egui::vec2(x, 0.0),
                egui::vec2(widths.get(c).copied().unwrap_or(0.0), LETTERS),
            );
            let letter = table_view::letter(c);
            let label = t.table_column(&letter);
            let id = of.with(("column", c));
            let resp = table_view::heading(ui, at, (&letter, &label), id, (tones, weak));
            let align = table.aligns.get(c).copied().unwrap_or(Align::None);
            let menu = (table.heading_column, Some(align));
            if let Some(picked) = table_view::menu(t, &resp, (true, c, columns), menu) {
                op = Some((picked, true, c));
            }
            drawn.menu |= menu_open(ui, &resp);
        }
        let right = xs.last().copied().unwrap_or(NUMBERS);
        let at = Rect::from_min_size(origin + egui::vec2(right, 0.0), egui::vec2(PLUS, LETTERS));
        let label = ("+", t.table_add_column());
        let add_column = table_view::heading(ui, at, label, of.with("add column"), (tones, weak));
        // "Show as lines" in the row of the "+" under the rows' numbers.
        let words = t.table_show_lines();
        let size = small_size(ui, words);
        let lines = Rect::from_min_size(
            origin + egui::vec2(NUMBERS + 4.0, bottom + (PLUS - size.y) / 2.0),
            size,
        );
        let raw = small_button(ui, lines, of.with("show lines"), words).clicked();
        // The columns' borders, which drag them wider or narrower.
        let (top, low) = (origin.y, origin.y + bottom);
        let mut dragged: Option<(usize, f32)> = None;
        for c in 0..columns {
            let x = origin.x + xs.get(c + 1).copied().unwrap_or(0.0);
            let border = Rect::from_x_y_ranges(x - BORDER..=x + BORDER, top..=low);
            let resp = ui
                .interact(border, of.with(("border", c)), Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeColumn);
            if resp.dragged() {
                let width = widths.get(c).copied().unwrap_or(NARROWEST);
                let delta = resp.drag_delta().x;
                if delta != 0.0 {
                    dragged = Some((c, (width + delta).max(DRAGGED)));
                }
            }
            if resp.hovered() || resp.dragged() {
                let accent = ui.visuals().hyperlink_color;
                ui.painter().vline(x, top..=low, Stroke::new(2.0, accent));
            }
        }
        // The cell edited, in place.
        if let Some((r, c)) = editing {
            let at = cell_rect(r, c);
            match self.cell(ui, t, (at, &table), text, boxes) {
                Some(Move::Cell(cell, place)) => self.start(ui, text, index, cell, place),
                Some(Move::Out(after)) => out = Some(after),
                Some(Move::Field) => {
                    self.finish(ui, text);
                    self.back_to_field(ui, text);
                }
                None => {}
            }
        }
        if let Some((c, width)) = dragged {
            let widths = self.state.widths.entry(index).or_default();
            widths.resize(columns, None);
            if let Some(slot) = widths.get_mut(c) {
                *slot = Some(width);
            }
            ui.ctx().request_repaint();
        }
        if let Some(cell) = picked {
            self.start(ui, text, index, cell, (usize::MAX, usize::MAX));
        }
        if add_row.clicked() {
            let last = rows.saturating_sub(1);
            self.operate(ui, text, index, (Op::After, false, last));
        }
        if add_column.clicked() {
            let last = columns.saturating_sub(1);
            self.operate(ui, text, index, (Op::After, true, last));
        }
        if let Some(op) = op {
            self.operate(ui, text, index, op);
        }
        if raw {
            self.finish(ui, text);
            self.state.raw.insert(index, false);
            // The field has the keyboard at the end of the table's first
            // line.
            if let Some(table) = table::all(text).get(index) {
                let first = line_end(text, table.start);
                format_bar::select(ui, self.id, text, (first, first));
            }
        }
        if let Some(after) = out {
            self.leave(ui, text, index, after);
        }
        drawn
    }

    /// The field of the cell edited, at `rect` in `table`: its keys first,
    /// then its words, written into `text` as they change. Gives where the
    /// keys move.
    fn cell(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        (rect, table): (Rect, &Table),
        text: &mut String,
        boxes: &BoxLook,
    ) -> Option<Move> {
        let id = cell_id(self.id);
        let editing = self.state.editing.as_mut()?;
        let (row, column) = (editing.row, editing.column);
        let focused = ui.memory(|m| m.has_focus(id));
        let had = ui.memory(|m| m.had_focus_last_frame(id));
        if focused {
            ui.data_mut(|d| d.insert_temp(Id::new("table grid cell"), id));
        }
        // Esc drops the keyboard before this frame: back into the field.
        if (focused || had) && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            return Some(Move::Field);
        }
        if focused && editing.place.is_none() {
            let end = editing.draft.chars().count();
            let cursor = TextEditState::load(ui.ctx(), id)
                .and_then(|s| s.cursor.char_range())
                .filter(|r| r.primary == r.secondary)
                .map(|r| r.primary.index.0);
            let moved = ui.input_mut(|i| {
                // Shift+Tab first: Tab alone would take it too.
                let plain = i.modifiers.is_none();
                if i.consume_key(Modifiers::SHIFT, Key::Tab) {
                    Some(Step::Previous)
                } else if !plain {
                    None
                } else if i.consume_key(Modifiers::NONE, Key::Tab) {
                    Some(Step::Next)
                } else if i.consume_key(Modifiers::NONE, Key::Enter)
                    || i.consume_key(Modifiers::NONE, Key::ArrowDown)
                {
                    Some(Step::Below)
                } else if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    Some(Step::Above)
                } else if cursor == Some(0) && i.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                    Some(Step::Back)
                } else if cursor == Some(end) && i.consume_key(Modifiers::NONE, Key::ArrowRight) {
                    Some(Step::On)
                } else {
                    None
                }
            });
            if let Some(step) = moved {
                return Some(step.from(table, (row, column)));
            }
        }
        let fill = ui.visuals().text_edit_bg_color();
        ui.painter().rect_filled(rect, 0.0, fill);
        if let Some((a, b)) = editing.place {
            select_chars(ui, id, (a, b));
        }
        let colour = self.colour;
        let accent = ui.visuals().hyperlink_color;
        let size = self.size;
        let mut layouter = |ui: &Ui, typed: &dyn egui::TextBuffer, width: f32| {
            let typed = typed.as_str();
            let font = FontId::proportional(size);
            let job = text_marks_view::tag_job(typed, (0, typed.len()), font, (colour, accent));
            let mut job = text_marks_view::field_job(job, boxes, &|_| 0.0);
            job.wrap.max_width = width;
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let before = editing.draft.clone();
        text_marks_view::guard_field(ui, id, &mut editing.draft, editing.place.is_none());
        let response = ui.put(
            rect.shrink2(egui::vec2(PAD.x, 0.0)),
            TextEdit::singleline(&mut editing.draft)
                .id(id)
                .font(FontId::proportional(size))
                .text_color(colour)
                .layouter(&mut layouter)
                .lock_focus(true)
                .return_key(None)
                .frame(Frame::NONE)
                .desired_width(f32::INFINITY)
                .vertical_align(egui::Align::Center),
        );
        crate::text_menu::show(ui, t, &response, &editing.draft);
        text_marks_view::guard_cursor(ui, id, &mut editing.draft);
        text_marks_view::remember_text(ui, id, &editing.draft);
        let named = t.table_cell(row as i64 + 1, &table_view::letter(column));
        crate::app::name_field(ui, &response, &named);
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(2.0, accent),
            egui::StrokeKind::Inside,
        );
        if editing.draft != before {
            let draft = editing.draft.clone();
            write_cell(ui, self.id, text, (editing.table, row, column), &draft);
        }
        // The cell takes the keyboard the moment it is edited; egui drops
        // that focus again while the field is new under the click that
        // started it, or in a dialog's first, unseen frame.
        if editing.place.is_some() {
            if response.has_focus() {
                editing.place = None;
            } else {
                ui.ctx().request_repaint();
            }
            return None;
        }
        if response.lost_focus() && !ui.memory(|m| m.focused().is_some_and(|f| f == id)) {
            // Clicked away: into the field, a menu of the grid, or out.
            self.finish(ui, text);
        }
        None
    }

    /// Does `op` to the row (or, the column) `index` of the table `table`.
    fn operate(
        &mut self,
        ui: &Ui,
        text: &mut String,
        table: usize,
        (op, column, index): (Op, bool, usize),
    ) {
        self.finish(ui, text);
        let Some(shown) = table::all(text).get(table).cloned() else {
            return;
        };
        let at = shown.start;
        let change = match (op, column) {
            (Op::Before, false) => table::insert_row(text, at, index),
            (Op::After, false) => table::insert_row(text, at, index + 1),
            (Op::Back, false) if index > 0 => table::move_row(text, at, index, index - 1),
            (Op::Forward, false) => table::move_row(text, at, index, index + 1),
            (Op::Before, true) => table::insert_column(text, at, index),
            (Op::After, true) => table::insert_column(text, at, index + 1),
            (Op::Back, true) if index > 0 => table::move_column(text, at, index, index - 1),
            (Op::Forward, true) => table::move_column(text, at, index, index + 1),
            (Op::Align(align), _) => table::set_align(text, at, index, align),
            (Op::Headings, _) => table::toggle_heading_column(text, at),
            (Op::Delete, _) => {
                let gone = if column {
                    shown.columns() == 1
                } else {
                    shown.rows.len() == 1
                };
                let deleting = Deleting {
                    column,
                    index,
                    gone,
                };
                if table_view::holds_words(&shown, deleting) {
                    self.state.deleting = Some((table, deleting));
                } else {
                    self.delete(ui, text, table, deleting);
                }
                return;
            }
            (Op::Back, _) => return,
        };
        self.take(ui, text, change);
        // The widths dragged go with their columns.
        if column && let Some(widths) = self.state.widths.get_mut(&table) {
            widths.resize(shown.columns(), None);
            match op {
                Op::Before => widths.insert(index, None),
                Op::After => widths.insert((index + 1).min(widths.len()), None),
                Op::Back if index > 0 => widths.swap(index, index - 1),
                Op::Forward if index + 1 < widths.len() => widths.swap(index, index + 1),
                _ => {}
            }
        }
        // A row or column put in is edited at once, in its first cell.
        match (op, column) {
            (Op::Before, false) => self.start(ui, text, table, (index, 0), (0, 0)),
            (Op::After, false) => self.start(ui, text, table, (index + 1, 0), (0, 0)),
            (Op::Before, true) => self.start(ui, text, table, (0, index), (0, 0)),
            (Op::After, true) => self.start(ui, text, table, (0, index + 1), (0, 0)),
            _ => self.back_to_field(ui, text),
        }
    }

    /// Deletes the row or column `deleting` says of the table `table`.
    fn delete(&mut self, ui: &Ui, text: &mut String, table: usize, deleting: Deleting) {
        self.finish(ui, text);
        let Some(shown) = table::all(text).get(table).cloned() else {
            return;
        };
        let change = if deleting.column {
            if let Some(widths) = self.state.widths.get_mut(&table)
                && deleting.index < widths.len()
            {
                widths.remove(deleting.index);
            }
            table::delete_column(text, shown.start, deleting.index)
        } else {
            table::delete_row(text, shown.start, deleting.index)
        };
        if deleting.gone {
            self.state.widths.remove(&table);
        }
        self.take(ui, text, change);
        if !self.state.answered {
            self.back_to_field(ui, text);
        }
    }

    /// Takes `change` as the text, one step back for Ctrl+Z in the field.
    fn take(&mut self, ui: &Ui, text: &mut String, change: Change) {
        if change.text != *text {
            undoable(ui, self.id, text);
            put(ui, self.id, text, change.text);
        }
    }
}

/// Edits the cell `(row, column)` of the table beginning at the byte `at`
/// of `text` in the grids of the field `id` holding the text of `of`, the
/// cursor at its end, the table shown as a grid: as the format bar puts a
/// new table in, or a click on a table read edits it.
pub fn edit(ui: &Ui, (id, of): (Id, &str), text: &str, at: usize, (row, column): (usize, usize)) {
    let Some(index) = table::all(text).iter().position(|t| t.start == at) else {
        return;
    };
    let key = state_id(id, of);
    let mut state: State = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    state.raw.remove(&index);
    let draft = table::cell(text, at, row, column).unwrap_or_default();
    let end = draft.chars().count();
    fresh_cell(ui, cell_id(id), &draft);
    state.editing = Some(Editing {
        table: index,
        row,
        column,
        draft,
        place: Some((end, end)),
        before: text.to_string(),
    });
    state.seen = text.to_string();
    ui.data_mut(|d| d.insert_temp(key, state));
}

/// The cell of `table` the byte `at` of `text` is in, by row and column.
pub fn cell_at(text: &str, table: &Table, at: usize) -> (usize, usize) {
    let chars = text.get(..at).map_or(0, |s| s.chars().count());
    cell_of(text, table, chars)
}

/// What a grid's drawing found.
#[derive(Default)]
struct Drawn {
    menu: bool,
    boxes: Option<BoxAsk>,
}

/// Where a key moves from a cell.
enum Move {
    /// To the cell, its characters selected.
    Cell((usize, usize), (usize, usize)),
    /// Out of the grid, into the text after the table while `true`, else
    /// before it.
    Out(bool),
    /// Back into the field, its cursor where it was.
    Field,
}

/// A key's step from a cell.
#[derive(Clone, Copy)]
enum Step {
    Next,
    Previous,
    Below,
    Above,
    Back,
    On,
}

impl Step {
    /// Where the step goes from the cell `(row, column)` of `table`.
    fn from(self, table: &Table, (row, column): (usize, usize)) -> Move {
        let (rows, columns) = (table.rows.len(), table.columns());
        let end = (usize::MAX, usize::MAX);
        match self {
            Step::Next | Step::On => match next(table, (row, column)) {
                Some(cell) => {
                    let place = if matches!(self, Step::On) {
                        (0, 0)
                    } else {
                        end
                    };
                    Move::Cell(cell, place)
                }
                None => Move::Out(true),
            },
            Step::Previous | Step::Back => match previous(table, (row, column)) {
                Some(cell) => Move::Cell(cell, end),
                None => Move::Out(false),
            },
            Step::Below if row + 1 < rows => Move::Cell((row + 1, column.min(columns)), end),
            Step::Below => Move::Out(true),
            Step::Above if row > 0 => Move::Cell((row - 1, column), end),
            Step::Above => Move::Out(false),
        }
    }
}

/// The cell after `(row, column)` of `table`, row by row; none after the
/// last.
fn next(table: &Table, (row, column): (usize, usize)) -> Option<(usize, usize)> {
    if column + 1 < table.columns() {
        Some((row, column + 1))
    } else if row + 1 < table.rows.len() {
        Some((row + 1, 0))
    } else {
        None
    }
}

/// The cell before `(row, column)` of `table`, row by row; none before
/// the first.
fn previous(table: &Table, (row, column): (usize, usize)) -> Option<(usize, usize)> {
    if column > 0 {
        Some((row, column - 1))
    } else if row > 0 {
        Some((row - 1, table.columns().saturating_sub(1)))
    } else {
        None
    }
}

/// The last cell of `table`.
fn last_cell(table: &Table) -> (usize, usize) {
    (
        table.rows.len().saturating_sub(1),
        table.columns().saturating_sub(1),
    )
}

/// Makes the field of the cell edited, `cell`, new for the words
/// `draft`: its typing undoes on its own, not another cell's, and its
/// cursor follows no change from another cell's words.
fn fresh_cell(ui: &Ui, cell: Id, draft: &str) {
    if let Some(mut state) = TextEditState::load(ui.ctx(), cell) {
        state.clear_undoer();
        state.store(ui.ctx(), cell);
    }
    text_marks_view::remember_text(ui, cell, draft);
}

/// Whether the field of a cell of a grid had the keyboard in the last
/// frame: Esc there leaves the cell, not the dialog around it.
pub fn cell_had_keyboard(ctx: &egui::Context) -> bool {
    let cell: Option<Id> = ctx.data(|d| d.get_temp(Id::new("table grid cell")));
    cell.is_some_and(|cell| ctx.memory(|m| m.had_focus_last_frame(cell) || m.has_focus(cell)))
}

/// The id of the grids' state of the field `id` holding the text of `of`.
fn state_id(id: Id, of: &str) -> Id {
    id.with(("table grids", of))
}

/// The id of the field of the cell edited in the grids of the field `id`.
pub fn cell_id(id: Id) -> Id {
    id.with("table cell")
}

/// The selection of the field `id`, in characters: its moving end first.
fn field_cursor(ui: &Ui, id: Id) -> Option<(usize, usize)> {
    let range = TextEditState::load(ui.ctx(), id)?.cursor.char_range()?;
    Some((range.primary.index.0, range.secondary.index.0))
}

/// Selects the characters `start..end` in the field `id` and gives it the
/// keyboard.
fn select_chars(ui: &Ui, id: Id, (start, end): (usize, usize)) {
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(start),
        CCursor::new(end),
    )));
    state.store(ui.ctx(), id);
    ui.memory_mut(|m| m.request_focus(id));
}

/// Where `table` lies in `text`, in characters: its first and the one
/// after its last.
fn chars_of(text: &str, table: &Table) -> (usize, usize) {
    let chars = |b: usize| text.get(..b).map_or(0, |s| s.chars().count());
    (chars(table.start), chars(table.end))
}

/// The cell of `table` the character `at` of `text` is in, the header
/// row's first for its delimiter row and the line before it.
fn cell_of(text: &str, table: &Table, at: usize) -> (usize, usize) {
    let byte = text.char_indices().nth(at).map_or(text.len(), |(b, _)| b);
    let Some((r, row)) = table
        .rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.start <= byte && byte <= row.end)
    else {
        return (0, 0);
    };
    let c = row
        .cells
        .iter()
        .rposition(|cell| cell.start <= byte)
        .unwrap_or(0);
    (r, c)
}

/// Where the character `at` of `text` is in the words of the cell `cell`
/// of `table` as edited, in characters.
fn draft_char(text: &str, table: &Table, (r, c): (usize, usize), at: usize) -> usize {
    let byte = text.char_indices().nth(at).map_or(text.len(), |(b, _)| b);
    let Some(cell) = table.rows.get(r).and_then(|row| row.cells.get(c)) else {
        return 0;
    };
    if byte <= cell.start {
        return 0;
    }
    let to = byte.min(cell.end);
    text.get(cell.start..to)
        .map_or(0, |s| s.replace("\\|", "|").chars().count())
}

/// Where the byte `at` of the words `draft` of a cell, written from the
/// byte `start` of the text, is in the text: without the spaces before
/// them, each `|` written `\|`.
fn text_byte(draft: &str, start: usize, at: usize) -> usize {
    let lead = draft.len() - draft.trim_start_matches([' ', '\t']).len();
    let kept = written(draft).len() + lead;
    let to = at.clamp(lead, kept);
    start
        + draft.get(lead..to).map_or(0, |s| {
            s.chars()
                .map(|c| if c == '|' { 2 } else { c.len_utf8() })
                .sum()
        })
}

/// What a cell's words read as once written: without the spaces and tabs
/// at their ends.
fn written(draft: &str) -> &str {
    draft.trim_matches([' ', '\t'])
}

/// Writes `draft` into the cell `(table, row, column)` of `text`, the
/// table found again by its place among the text's tables, for the field
/// `field`.
fn write_cell(
    ui: &Ui,
    field: Id,
    text: &mut String,
    (index, row, column): (usize, usize, usize),
    draft: &str,
) {
    let Some(start) = table::all(text).get(index).map(|t| t.start) else {
        return;
    };
    let change = table::set_cell(text, start, row, column, draft);
    put(ui, field, text, change.text);
}

/// Puts `new` into the field `field` holding `text`, its cursor kept
/// where it was among the characters around the change.
fn put(ui: &Ui, field: Id, text: &mut String, new: String) {
    if let Some(mut state) = TextEditState::load(ui.ctx(), field)
        && let Some(range) = state.cursor.char_range()
    {
        let follow = |c: CCursor| CCursor::new(text_marks_view::followed(text, &new, c.index.0));
        state.cursor.set_char_range(Some(CCursorRange::two(
            follow(range.secondary),
            follow(range.primary),
        )));
        state.store(ui.ctx(), field);
    }
    *text = new;
}

/// Makes `old` one step back for Ctrl+Z in the field `id`. The field's
/// own undoer takes the text as it is now when it next has the keyboard,
/// with the cursor it then has.
fn undoable(ui: &Ui, id: Id, old: &str) {
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    let range = state
        .cursor
        .char_range()
        .unwrap_or(CCursorRange::one(CCursor::new(0)));
    let mut undoer = state.undoer();
    undoer.add_undo(&(range, old.to_string()));
    state.set_undoer(undoer);
    state.store(ui.ctx(), id);
}

/// `after`, an edit of `before` typed outside its tables, changed so
/// that each table of `before` shown as a grid (not one of `raw`, by its
/// place among them) stays the table it was, as the phone's `keepTables`
/// does: what is typed goes on a line of its own after a line break or
/// a blank line, and a line break whose deletion would join a line to a
/// table is put back. Gives the text and the byte its cursor goes to;
/// `None` while nothing needs changing, or for an edit of a table's own
/// lines, which the grid makes.
pub(crate) fn keep_tables(before: &str, after: &str, raw: &[usize]) -> Option<(String, usize)> {
    let tables: Vec<Arc<Table>> = table::all(before)
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !raw.contains(index))
        .map(|(_, t)| t)
        .collect();
    if tables.is_empty() || before == after {
        return None;
    }
    // What `before` lost from `a` to `b`, and what took its place.
    let most = before.len().min(after.len());
    let mut a = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(x, y)| x == y)
        .count();
    while !before.is_char_boundary(a) || !after.is_char_boundary(a) {
        a -= 1;
    }
    let mut tail = before
        .bytes()
        .rev()
        .zip(after.bytes().rev())
        .take(most - a)
        .take_while(|(x, y)| x == y)
        .count();
    while !before.is_char_boundary(before.len() - tail)
        || !after.is_char_boundary(after.len() - tail)
    {
        tail -= 1;
    }
    let b = before.len() - tail;
    let typed = after.get(a..after.len() - tail)?;
    if tables.iter().any(|t| a < t.end && b > t.start) {
        return None;
    }
    // Whether each table is still one where it went, `grew` bytes later
    // when it lay after the edit.
    let whole = |text: &str, grew: isize| {
        let now = table::all(text);
        tables.iter().all(|t| {
            let start = if t.start >= b {
                t.start.checked_add_signed(grew).unwrap_or(t.start)
            } else {
                t.start
            };
            let end = start + t.end - t.start;
            now.iter().any(|n| n.start == start && n.end == end)
        })
    };
    let removed = (b - a) as isize;
    if whole(after, typed.len() as isize - removed) {
        return None;
    }
    let br = line_break(before);
    let (head, rest) = (before.get(..a)?, before.get(b..)?);
    let two = format!("{br}{br}");
    for (lead, trail) in [("", br), (br, ""), (two.as_str(), "")] {
        let put = format!("{lead}{typed}{trail}");
        let text = format!("{head}{put}{rest}");
        if whole(&text, put.len() as isize - removed) {
            return Some((text, a + lead.len() + typed.len()));
        }
    }
    None
}

/// The line break `text` uses: its first, `\n` while it has none.
fn line_break(text: &str) -> &'static str {
    match text.find(['\n', '\r']) {
        Some(at) if text.get(at..at + 2) == Some("\r\n") => "\r\n",
        Some(at) if text.get(at..at + 1) == Some("\r") => "\r",
        _ => "\n",
    }
}

/// How long the line break at the byte `at` of `text` is.
fn break_len(text: &str, at: usize) -> usize {
    match text.get(at..at + 1) {
        Some("\r") if text.get(at + 1..at + 2) == Some("\n") => 2,
        Some("\r") | Some("\n") => 1,
        _ => 0,
    }
}

/// Where the line holding the byte `at` of `text` ends, before its line
/// break.
fn line_end(text: &str, at: usize) -> usize {
    text.get(at..)
        .and_then(|s| s.find(['\n', '\r']))
        .map_or(text.len(), |n| at + n)
}

/// Where the line ending at the byte `end` of `text` begins.
fn line_start(text: &str, end: usize) -> usize {
    text.get(..end)
        .and_then(|s| s.rfind(['\n', '\r']))
        .map_or(0, |n| n + 1)
}

fn blank(line: &str) -> bool {
    line.trim_matches([' ', '\t']).is_empty()
}

/// `text` and where its cursor goes after `table`: at the start of the
/// line right under it, which is no row of it, or of the line after a
/// blank one; only where the table ends the text are line breaks put
/// in, a blank line between, as a line typed right under a table is a
/// row of it (ADR 0033).
pub(crate) fn leave_after(text: &str, table: &Table) -> (String, usize) {
    let br = line_break(text);
    let end = table.end;
    let first = break_len(text, end);
    if first == 0 {
        return (format!("{text}{br}{br}"), text.len() + 2 * br.len());
    }
    let next = end + first;
    let next_end = line_end(text, next);
    if blank(text.get(next..next_end).unwrap_or_default()) {
        let second = break_len(text, next_end);
        if second == 0 {
            return (format!("{text}{br}"), text.len() + br.len());
        }
        return (text.to_string(), next_end + second);
    }
    (text.to_string(), next)
}

/// `text` and where its cursor goes before `table`: at the end of the
/// line before it, unless that line is a table's or a blank one right
/// under a table, where a line is put in.
pub(crate) fn leave_before(text: &str, table: &Table) -> (String, usize) {
    let br = line_break(text);
    let start = table.start;
    let put = |lines: usize, at: usize| {
        let breaks = br.repeat(lines);
        let put = format!(
            "{}{breaks}{}",
            text.get(..start).unwrap_or_default(),
            text.get(start..).unwrap_or_default()
        );
        (put, at)
    };
    if start == 0 {
        return put(1, 0);
    }
    // The line break before the table, and the line before that.
    let above_end = if text.get(..start).is_some_and(|s| s.ends_with("\r\n")) {
        start - 2
    } else {
        start - 1
    };
    let above = line_start(text, above_end);
    let in_table = |at: usize| table::at(text, at).is_some();
    if in_table(above) {
        return put(2, start + br.len());
    }
    if blank(text.get(above..above_end).unwrap_or_default()) && above > 0 {
        let higher_end = if text.get(..above).is_some_and(|s| s.ends_with("\r\n")) {
            above - 2
        } else {
            above - 1
        };
        if in_table(line_start(text, higher_end)) {
            return put(1, start);
        }
    }
    (text.to_string(), above_end)
}

/// Re-sections `job` so that its bytes `start..end` are drawn in a font
/// of one point, invisibly, the character `start..first` as high as
/// `height` and the rest as low as they go.
fn hide_range(job: &mut LayoutJob, (start, first, end): (usize, usize, usize), height: f32) {
    let sections = std::mem::take(&mut job.sections);
    let mut out = Vec::with_capacity(sections.len() + 3);
    for section in sections {
        let (a, b) = (section.byte_range.start.0, section.byte_range.end.0);
        let mut cuts = vec![a];
        for cut in [start, first, end] {
            if a < cut && cut < b {
                cuts.push(cut);
            }
        }
        cuts.push(b);
        for pair in cuts.windows(2) {
            let &[x, y] = pair else {
                continue;
            };
            let inside = start <= x && y <= end;
            let format = if inside {
                hidden(&section.format, if x < first { height } else { 1.0 })
            } else {
                section.format.clone()
            };
            let lead = if x == a && !inside {
                section.leading_space
            } else {
                0.0
            };
            out.push(LayoutSection {
                leading_space: lead,
                byte_range: egui::text::ByteIndex(x)..egui::text::ByteIndex(y),
                format,
            });
        }
    }
    job.sections = out;
}

/// `format` drawn invisibly in a font of one point, its line `height`
/// high.
fn hidden(format: &TextFormat, height: f32) -> TextFormat {
    let mut hidden = format.clone();
    hidden.font_id = FontId::new(1.0, format.font_id.family.clone());
    hidden.color = Color32::TRANSPARENT;
    hidden.background = Color32::TRANSPARENT;
    hidden.underline = Stroke::NONE;
    hidden.strikethrough = Stroke::NONE;
    hidden.extra_letter_spacing = 0.0;
    hidden.line_height = Some(height);
    hidden
}

/// Paints the words `laid` of a cell in `rect`, lined up as `align` says,
/// and gives where its boxes, tags and links lie.
fn paint_cell(
    ui: &mut Ui,
    laid: &LaidCell,
    rect: Rect,
    align: Align,
    look: &Look,
) -> text_marks_view::Spots {
    let words = laid.size(ui, look);
    let left = text_marks_view::aligned(align, rect.width(), words.x, PAD.x);
    let top = (rect.height() - words.y) / 2.0;
    let pos = rect.min + egui::vec2(left.max(0.0), top.max(0.0));
    let colour = ui.visuals().text_color();
    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    clipped.set_clip_rect(rect.intersect(ui.clip_rect()));
    laid.paint(&mut clipped, pos, colour, false)
}

/// Where each of the lengths `sizes` begins, one after another from
/// `first`, and where the last ends.
fn starts(first: f32, sizes: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(sizes.len() + 1);
    let mut at = first;
    out.push(at);
    for size in sizes {
        at += size;
        out.push(at);
    }
    out
}

/// Whether the menu of the heading `resp` is open, by a click or a
/// right-click.
fn menu_open(ui: &Ui, resp: &egui::Response) -> bool {
    let id = Popup::default_response_id(resp);
    Popup::is_id_open(ui.ctx(), id) || Popup::is_id_open(ui.ctx(), id.with("context"))
}

/// How much room the small button `words` takes.
fn small_size(ui: &Ui, words: &str) -> Vec2 {
    let weak = ui.visuals().weak_text_color();
    let galley =
        ui.fonts_mut(|f| f.layout_no_wrap(words.to_string(), FontId::proportional(12.0), weak));
    galley.size() + egui::vec2(12.0, 6.0)
}

/// A small button of weak words at `rect`, as the grids' switch to their
/// lines and back.
fn small_button(ui: &mut Ui, rect: Rect, id: Id, words: &str) -> egui::Response {
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, words));
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            let fill = ui.visuals().widgets.hovered.weak_bg_fill;
            ui.painter().rect_filled(rect, 6.0, fill);
        }
        let weak = ui.visuals().weak_text_color();
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            words,
            FontId::proportional(12.0),
            weak,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests;
