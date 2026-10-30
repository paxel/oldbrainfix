//! A table of a formatted text full screen (#128, ADR 0033), as the
//! phone's `table_screen.dart` shows it: over the whole window, scrolled
//! both ways with the mouse wheel, Shift and the wheel, or the scroll
//! bars, its header row always on screen and its first column too while
//! that column holds headings. Back (Alt+Left, the mouse's back button,
//! Esc or the arrow at the top) leaves it.
//!
//! Where the Note may be changed, a click on a cell edits its words on
//! one line in the cell, framed: Enter or a click elsewhere writes them,
//! Tab goes on to the next cell, Shift+Tab back, Esc leaves the cell as
//! it was. The rows' numbers and the columns' letters open their menus
//! on a click or a right-click: put one in before or after, move it,
//! delete it (asked once while it holds words; the last row or column
//! takes the table with it and leaves the view), and on the first column
//! whether it holds headings. An empty row and column after the last add
//! one when words are typed into them. Every edit writes the whole table
//! anew through the core. A box in a cell (ADR 0035) shows the state of
//! its latest tick and, where this Device may tick it, opens Check, Fail,
//! Open and its History as on a card; a tick is no edit of the table.

use egui::{Color32, Frame, Id, Key, Margin, Modifiers, Rect, RichText, Sense, TextEdit, Ui};
use std::collections::BTreeSet;

use egui::{FontId, Popup, Vec2, WidgetInfo, WidgetType};
use oldbrainfix_core::markdown::Change;
use oldbrainfix_core::markdown::table::{self, Align, Table};
use oldbrainfix_core::{BoxState, text_boxes, text_format};

use crate::format_bar;
use crate::icons;
use crate::l10n::L10n;
use crate::text_marks_view::{self, BoxAsk, BoxLook, Found, LaidCell, Look, Spots};
use crate::theme::{self, Tones};

/// A table to show full screen: the text it is in and where it begins
/// there, in bytes; the Note to write it to while this Device may change
/// it (none to read it only); the cell to edit at once, by row and
/// column; whether a click on a tag searches for it; and the Note or
/// Section whose text holds its boxes, to tick them (none for a text
/// that is not stored).
#[derive(Debug, Clone, PartialEq)]
pub struct Opening {
    pub note: Option<String>,
    pub text: String,
    pub at: usize,
    pub cell: Option<(usize, usize)>,
    pub tags: bool,
    pub boxes: Option<String>,
}

/// What the view asks of the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    /// Keep the Note's text as this, the table edited.
    Write(String),
    /// Leave the view; the cell edited is written first.
    Close,
    /// Search for this tag, as written.
    SearchFor(String),
    /// Tick the box of this id to this state.
    Tick(String, BoxState),
    /// Show the History of the box of this id.
    History(String),
}

/// The table shown full screen and what is being done to it.
#[derive(Debug, Clone, PartialEq)]
pub struct TableView {
    note: Option<String>,
    text: String,
    at: usize,
    tags: bool,
    boxes: Option<String>,
    ticking: Ticking,
    editing: Option<Editing>,
    deleting: Option<Deleting>,
}

/// How the boxes of the table show and what a click on one offers: as
/// the window read them for the Note or Section they are in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ticking {
    pub look: BoxLook,
    pub may_tick: bool,
    pub ticked: BTreeSet<String>,
}

/// The cell being edited, the row or column after the last adding one,
/// its words as typed, and whether the field is to take the keyboard.
#[derive(Debug, Clone, PartialEq)]
struct Editing {
    row: usize,
    column: usize,
    typed: String,
    focus: bool,
}

/// The row or column whose delete is asked about, and whether it is the
/// last, taking the table with it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Deleting {
    column: bool,
    index: usize,
    gone: bool,
}

/// What a heading's menu does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Before,
    After,
    Back,
    Forward,
    Delete,
    Headings,
}

/// The room around a cell's words.
const PAD: Vec2 = egui::vec2(10.0, 6.0);
/// The narrowest a column is drawn.
const NARROWEST: f32 = 72.0;
/// How wide the rows' numbers are, how high the columns' letters, and
/// how wide the empty column that adds one.
const NUMBERS: f32 = 44.0;
const LETTERS: f32 = 28.0;
const ADDING: f32 = 56.0;
/// How wide the field of the cell edited is at least.
const FIELD_WIDTH: f32 = 160.0;
/// The size the cells' words are drawn at.
const SIZE: f32 = 16.0;

impl TableView {
    pub fn new(opening: Opening) -> TableView {
        let mut view = TableView {
            note: opening.note,
            text: opening.text,
            at: opening.at,
            tags: opening.tags,
            boxes: opening.boxes,
            ticking: Ticking::default(),
            editing: None,
            deleting: None,
        };
        if let Some((row, column)) = opening.cell {
            view.start_editing(row, column);
        }
        view
    }

    /// The Note it writes to; none while it is only read.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// The text the table is in, as edited here.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Where the table begins in its text, in bytes.
    pub fn at(&self) -> usize {
        self.at
    }

    /// The Note or Section whose text holds its boxes, while it is stored.
    pub fn boxes(&self) -> Option<&str> {
        self.boxes.as_deref()
    }

    /// Shows its boxes as `ticking` says.
    pub fn set_ticking(&mut self, ticking: Ticking) {
        self.ticking = ticking;
    }

    /// Takes `text` as the text the table is in, the table at `at`: the
    /// text as it was stored, its new boxes with their ids.
    pub fn take_stored(&mut self, text: String, at: usize) {
        self.text = text;
        self.at = at;
    }

    /// The table shown, while it is there.
    fn table(&self) -> Option<std::sync::Arc<Table>> {
        table::at(&self.text, self.at)
    }

    fn edits(&self) -> bool {
        self.note.is_some()
    }

    fn start_editing(&mut self, row: usize, column: usize) {
        if !self.edits() {
            return;
        }
        let typed = table::cell(&self.text, self.at, row, column).unwrap_or_default();
        self.editing = Some(Editing {
            row,
            column,
            typed,
            focus: true,
        });
    }

    /// Takes `change` as the text now; gives it to write when it changed.
    fn take(&mut self, change: Change) -> Option<String> {
        if change.text == self.text {
            return None;
        }
        self.text = change.text;
        self.at = change.start;
        Some(self.text.clone())
    }

    /// Ends editing the cell, its words written when they changed; one
    /// in the empty row or column adds that row or column, but only with
    /// words. Gives the text to write.
    pub fn commit(&mut self) -> Option<String> {
        let editing = self.editing.take()?;
        let table = self.table()?;
        let (rows, columns) = (table.rows.len(), table.columns());
        let (row, column) = (editing.row, editing.column);
        let adds = row >= rows || column >= columns;
        if adds && editing.typed.trim().is_empty() {
            return None;
        }
        if !adds
            && table::cell(&self.text, self.at, row, column).as_deref()
                == Some(editing.typed.as_str())
        {
            return None;
        }
        let (mut text, mut at) = (self.text.clone(), self.at);
        if row >= rows {
            let change = table::insert_row(&text, at, rows);
            (text, at) = (change.text, change.start);
        }
        if column >= columns {
            let change = table::insert_column(&text, at, columns);
            (text, at) = (change.text, change.start);
        }
        // What is typed goes into the row or column now there.
        self.take(table::set_cell(&text, at, row, column, &editing.typed))
    }

    /// Ends editing the cell, written, and edits the one at `row` and
    /// `column`.
    fn edit(&mut self, (row, column): (usize, usize), asked: &mut Vec<Asked>) {
        if self
            .editing
            .as_ref()
            .is_some_and(|e| (e.row, e.column) == (row, column))
        {
            return;
        }
        if let Some(text) = self.commit() {
            asked.push(Asked::Write(text));
        }
        self.start_editing(row, column);
    }

    /// Does `op` to the row (or, `column`, the column) `index`.
    fn operate(&mut self, op: Op, column: bool, index: usize, asked: &mut Vec<Asked>) {
        if let Some(text) = self.commit() {
            asked.push(Asked::Write(text));
        }
        let Some(table) = self.table() else {
            return;
        };
        let (text, at) = (self.text.as_str(), self.at);
        let change = match (op, column) {
            (Op::Before, false) => table::insert_row(text, at, index),
            (Op::After, false) => table::insert_row(text, at, index + 1),
            (Op::Back, false) if index > 0 => table::move_row(text, at, index, index - 1),
            (Op::Forward, false) => table::move_row(text, at, index, index + 1),
            (Op::Before, true) => table::insert_column(text, at, index),
            (Op::After, true) => table::insert_column(text, at, index + 1),
            (Op::Back, true) if index > 0 => table::move_column(text, at, index, index - 1),
            (Op::Forward, true) => table::move_column(text, at, index, index + 1),
            (Op::Headings, _) => table::toggle_heading_column(text, at),
            (Op::Delete, _) => {
                let gone = if column {
                    table.columns() == 1
                } else {
                    table.rows.len() == 1
                };
                let deleting = Deleting {
                    column,
                    index,
                    gone,
                };
                if holds_words(&table, deleting) {
                    self.deleting = Some(deleting);
                } else {
                    self.delete(deleting, asked);
                }
                return;
            }
            (Op::Back, _) => return,
        };
        if let Some(text) = self.take(change) {
            asked.push(Asked::Write(text));
        }
    }

    /// Deletes the row or column `deleting` says; the last one leaves the
    /// view with the table.
    fn delete(&mut self, deleting: Deleting, asked: &mut Vec<Asked>) {
        let (text, at) = (self.text.as_str(), self.at);
        let change = if deleting.column {
            table::delete_column(text, at, deleting.index)
        } else {
            table::delete_row(text, at, deleting.index)
        };
        if let Some(text) = self.take(change) {
            asked.push(Asked::Write(text));
        }
        if deleting.gone {
            asked.push(Asked::Close);
        }
    }

    /// Draws the view over the whole window, and the question before a
    /// delete over it; gives what it asks of the window.
    pub fn show(&mut self, ctx: &egui::Context, t: &L10n, tones: Tones) -> Vec<Asked> {
        let mut asked = Vec::new();
        // Esc in a cell leaves its words as they were, and the view open.
        if self.editing.is_some()
            && self.deleting.is_none()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.editing = None;
        }
        let modal = egui::Modal::new(Id::new("table view"))
            .frame(Frame::NONE.fill(tones.background))
            .show(ctx, |ui| {
                let screen = ui.ctx().content_rect();
                ui.set_min_size(screen.size());
                ui.set_max_size(screen.size());
                let text = ui.visuals().text_color();
                Frame::NONE.inner_margin(Margin::same(12)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let back = crate::app::icon_button(ui, icons::ARROW_BACK, t.close(), text);
                        if back.clicked() {
                            asked.push(Asked::Close);
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(t.table_title()).size(18.0).color(text));
                    });
                });
                egui::ScrollArea::both()
                    .id_salt("table view")
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        self.grid(ui, viewport, (t, tones), &mut asked);
                    });
            });
        if let Some(deleting) = self.deleting {
            self.question(ctx, t, tones, deleting, &mut asked);
        } else if modal.should_close() {
            asked.push(Asked::Close);
        }
        if self.table().is_none() && !asked.contains(&Asked::Close) {
            // The table is gone from under it.
            asked.push(Asked::Close);
        }
        asked
    }

    /// The question before a row or column holding words is deleted.
    fn question(
        &mut self,
        ctx: &egui::Context,
        t: &L10n,
        tones: Tones,
        deleting: Deleting,
        asked: &mut Vec<Asked>,
    ) {
        let frame = Frame::NONE
            .fill(tones.card)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(20));
        let mut answer = None;
        let modal = egui::Modal::new(Id::new("table delete"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                let text = ui.visuals().text_color();
                let question = if deleting.gone {
                    t.table_delete_table_question()
                } else if deleting.column {
                    t.table_delete_column_question()
                } else {
                    t.table_delete_row_question()
                };
                ui.label(RichText::new(question).size(18.0).color(text));
                ui.label(RichText::new(t.table_delete_body()).color(text));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
                        answer = Some(false);
                    }
                    let error = ui.visuals().error_fg_color;
                    if ui.button(RichText::new(t.delete()).color(error)).clicked() {
                        answer = Some(true);
                    }
                });
            });
        if modal.should_close() && answer.is_none() {
            answer = Some(false);
        }
        if let Some(sure) = answer {
            self.deleting = None;
            if sure {
                self.delete(deleting, asked);
            }
        }
    }

    /// The table in the scrolled part, `viewport` of it on screen: the
    /// cells, the headings and the field of the cell edited.
    fn grid(
        &mut self,
        ui: &mut Ui,
        viewport: Rect,
        (t, tones): (&L10n, Tones),
        asked: &mut Vec<Asked>,
    ) {
        // Tab and Shift+Tab in a cell: on to the next one, or back.
        let field = Id::new("table cell");
        if self.editing.is_some() && ui.memory(|m| m.has_focus(field)) {
            let (on, back) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::Tab),
                    i.consume_key(Modifiers::SHIFT, Key::Tab),
                )
            });
            if (on || back)
                && let Some(editing) = &self.editing
            {
                let from = (editing.row, editing.column);
                if let Some(text) = self.commit() {
                    asked.push(Asked::Write(text));
                }
                if let Some(table) = self.table() {
                    let next = if on {
                        next_cell(&table, from)
                    } else {
                        previous_cell(&table, from)
                    };
                    self.start_editing(next.0, next.1);
                }
            }
        }
        let Some(table) = self.table() else {
            return;
        };
        let edits = self.edits();
        let colour = ui.visuals().text_color();
        let look = Look::of(ui, (Some(SIZE), colour));
        let found = Found::of(&self.text, text_format::MARKDOWN, &self.ticking.look);
        let laid = text_marks_view::lay_table(ui, (&self.text, &table), (&found, &look));
        let (mut widths, mut heights) = text_marks_view::grid(ui, &laid, &look, PAD);
        let lowest = ui.fonts_mut(|f| f.row_height(&FontId::proportional(SIZE))) + PAD.y * 2.0;
        for width in &mut widths {
            *width = width.max(NARROWEST);
        }
        for height in &mut heights {
            *height = height.max(lowest);
        }
        let (rows, columns) = (table.rows.len(), table.columns());
        if edits {
            widths.push(ADDING);
            heights.push(lowest);
        }
        let (lead, top) = if edits {
            (NUMBERS, LETTERS)
        } else {
            (0.0, 0.0)
        };
        let xs = starts(lead, &widths);
        let ys = starts(top, &heights);
        let size = egui::vec2(
            xs.last().copied().unwrap_or(lead),
            ys.last().copied().unwrap_or(top),
        );
        let origin = ui.max_rect().min;
        // The space beside the cells ends editing, the cell written.
        let room = Rect::from_min_size(origin, size.max(viewport.size()));
        if ui.allocate_rect(room, Sense::click()).clicked()
            && let Some(text) = self.commit()
        {
            asked.push(Asked::Write(text));
        }
        let sticky_column = table.heading_column;
        let x_of = |c: usize| {
            let x = xs.get(c).copied().unwrap_or_default();
            if c == 0 && sticky_column {
                x.max(viewport.min.x + lead)
            } else {
                x
            }
        };
        let y_of = |r: usize| {
            let y = ys.get(r).copied().unwrap_or_default();
            if r == 0 {
                y.max(viewport.min.y + top)
            } else {
                y
            }
        };
        let rect_of = |r: usize, c: usize| {
            Rect::from_min_size(
                origin + egui::vec2(x_of(c), y_of(r)),
                egui::vec2(
                    widths.get(c).copied().unwrap_or_default(),
                    heights.get(r).copied().unwrap_or_default(),
                ),
            )
        };
        // The cells, those that stay on screen last, over the others.
        let (all_rows, all_columns) = if edits {
            (rows + 1, columns + 1)
        } else {
            (rows, columns)
        };
        let mut cells: Vec<(usize, usize)> = (0..all_rows)
            .flat_map(|r| (0..all_columns).map(move |c| (r, c)))
            .filter(|&(r, c)| r < rows || c < columns)
            .collect();
        cells.sort_by_key(|&(r, c)| (r == 0, c == 0 && sticky_column));
        let of = Id::new("table view cells");
        let (stroke, header) = text_marks_view::table_tones(colour);
        for (r, c) in cells {
            let rect = rect_of(r, c);
            let laid = laid.get(r).and_then(|row| row.get(c));
            if ui.is_rect_visible(rect) {
                ui.painter().rect_filled(rect, 0.0, tones.card);
                if r == 0 {
                    ui.painter().rect_filled(rect, 0.0, header);
                }
            }
            let mut spots = Spots::default();
            if let Some(laid) = laid {
                let align = table.aligns.get(c).copied().unwrap_or(Align::None);
                spots = paint_cell(ui, laid, rect, align, &look);
            }
            if ui.is_rect_visible(rect) {
                ui.painter()
                    .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            }
            let sense = if edits {
                Sense::click()
            } else {
                Sense::hover()
            };
            let resp = ui.interact(rect, of.with((r, c)), sense);
            let words = table
                .rows
                .get(r)
                .and_then(|row| row.cells.get(c))
                .map(|cell| text_boxes::strip(&cell.visible()))
                .unwrap_or_default();
            let label = if r >= rows {
                t.table_add_row().to_string()
            } else if c >= columns {
                t.table_add_column().to_string()
            } else {
                words
            };
            let typ = if edits {
                WidgetType::Button
            } else {
                WidgetType::Label
            };
            resp.widget_info(|| WidgetInfo::labeled(typ, true, &label));
            if edits && resp.on_hover_cursor(egui::CursorIcon::Text).clicked() {
                self.edit((r, c), asked);
            }
            // The cell's boxes, tags and links over it, under the cells
            // drawn after it.
            let ticked = |id: &str| self.ticking.ticked.contains(id);
            let ticks = self.ticking.may_tick && self.boxes.is_some();
            match text_marks_view::box_clicks(ui, t, of, &spots.boxes, (ticks, &ticked)) {
                Some(BoxAsk::Tick(id, to)) => asked.push(Asked::Tick(id, to)),
                Some(BoxAsk::History(id)) => asked.push(Asked::History(id)),
                None => {}
            }
            if self.tags
                && let Some(tag) = text_marks_view::tag_clicks(ui, t, of, &spots.tags)
            {
                asked.push(Asked::SearchFor(tag));
            }
            text_marks_view::link_clicks(ui, of, &spots.links);
        }
        if edits {
            self.headings(
                ui,
                (&table, viewport, origin),
                (&xs, &ys),
                (&widths, &heights),
                (t, tones),
                asked,
            );
            self.field(ui, field, rect_of, asked);
        }
    }

    /// The rows' numbers on the left and the columns' letters at the top,
    /// both always on screen, each opening its menu; the "+" of the empty
    /// row or column edits its first cell.
    fn headings(
        &mut self,
        ui: &mut Ui,
        (table, viewport, origin): (&Table, Rect, egui::Pos2),
        (xs, ys): (&[f32], &[f32]),
        (widths, heights): (&[f32], &[f32]),
        (t, tones): (&L10n, Tones),
        asked: &mut Vec<Asked>,
    ) {
        let (rows, columns) = (table.rows.len(), table.columns());
        let colour = ui.visuals().weak_text_color();
        let left = viewport.min.x;
        let top = viewport.min.y;
        let sticky_column = table.heading_column;
        let mut picked: Option<(Op, bool, usize)> = None;
        let mut adding: Option<(usize, usize)> = None;
        // The rows' numbers, the header row's last, over the others.
        let mut order: Vec<usize> = (1..=rows).collect();
        order.push(0);
        for r in order {
            let y = ys.get(r).copied().unwrap_or_default();
            let y = if r == 0 { y.max(top + LETTERS) } else { y };
            let rect = Rect::from_min_size(
                origin + egui::vec2(left, y),
                egui::vec2(NUMBERS, heights.get(r).copied().unwrap_or_default()),
            );
            let (shown, label) = if r < rows {
                ((r + 1).to_string(), t.table_row(r as i64 + 1))
            } else {
                ("+".to_string(), t.table_add_row().to_string())
            };
            let resp = heading(
                ui,
                rect,
                (&shown, &label),
                Id::new(("table row", r)),
                (tones, colour),
            );
            if r >= rows {
                if resp.clicked() {
                    adding = Some((rows, 0));
                }
            } else if let Some(op) = menu(t, &resp, (false, r, rows), false) {
                picked = Some((op, false, r));
            }
        }
        // The columns' letters, the first's last while it stays.
        let mut order: Vec<usize> = (0..=columns).collect();
        if sticky_column {
            order.retain(|&c| c != 0);
            order.push(0);
        }
        for c in order {
            let x = xs.get(c).copied().unwrap_or_default();
            let x = if c == 0 && sticky_column {
                x.max(left + NUMBERS)
            } else {
                x
            };
            let rect = Rect::from_min_size(
                origin + egui::vec2(x, top),
                egui::vec2(widths.get(c).copied().unwrap_or_default(), LETTERS),
            );
            let (shown, label) = if c < columns {
                let letter = letter(c);
                let label = t.table_column(&letter);
                (letter, label)
            } else {
                ("+".to_string(), t.table_add_column().to_string())
            };
            let resp = heading(
                ui,
                rect,
                (&shown, &label),
                Id::new(("table column", c)),
                (tones, colour),
            );
            if c >= columns {
                if resp.clicked() {
                    adding = Some((0, columns));
                }
            } else if let Some(op) = menu(t, &resp, (true, c, columns), table.heading_column) {
                picked = Some((op, true, c));
            }
        }
        // The corner over both.
        let corner =
            Rect::from_min_size(origin + egui::vec2(left, top), egui::vec2(NUMBERS, LETTERS));
        ui.painter().rect_filled(corner, 0.0, tones.background);
        if let Some((op, column, index)) = picked {
            self.operate(op, column, index, asked);
        }
        if let Some(cell) = adding {
            self.edit(cell, asked);
        }
    }

    /// The field of the cell edited, framed, over it; Enter or a click
    /// elsewhere writes it.
    fn field(
        &mut self,
        ui: &mut Ui,
        id: Id,
        rect_of: impl Fn(usize, usize) -> Rect,
        asked: &mut Vec<Asked>,
    ) {
        let Some(editing) = &mut self.editing else {
            return;
        };
        let cell = rect_of(editing.row, editing.column);
        let rect = Rect::from_min_size(
            cell.min,
            egui::vec2(cell.width().max(FIELD_WIDTH), cell.height()),
        );
        let accent = ui.visuals().hyperlink_color;
        let fill = ui.visuals().text_edit_bg_color();
        ui.painter().rect_filled(rect, 0.0, fill);
        let end = editing.typed.len();
        if editing.focus {
            format_bar::select(ui, id, &editing.typed, (end, end));
        }
        let text = ui.visuals().text_color();
        // Its boxes as they show, their ids not drawn and the cursor kept
        // out of them (ADR 0035).
        let look = &self.ticking.look;
        let mut layouter = |ui: &Ui, typed: &dyn egui::TextBuffer, width: f32| {
            let typed = typed.as_str();
            let font = FontId::proportional(SIZE);
            let job = text_marks_view::tag_job(typed, (0, typed.len()), font, (text, accent));
            let mut job = text_marks_view::field_job(job, look, &|_| 0.0);
            job.wrap.max_width = width;
            ui.fonts_mut(|f| f.layout_job(job))
        };
        text_marks_view::guard_field(ui, id, &mut editing.typed, !editing.focus);
        let response = ui.put(
            rect.shrink2(egui::vec2(PAD.x, 0.0)),
            TextEdit::singleline(&mut editing.typed)
                .id(id)
                .font(FontId::proportional(SIZE))
                .text_color(text)
                .layouter(&mut layouter)
                .lock_focus(true)
                .frame(Frame::NONE)
                .desired_width(f32::INFINITY)
                .vertical_align(egui::Align::Center),
        );
        text_marks_view::guard_cursor(ui, id, &mut editing.typed);
        text_marks_view::remember_text(ui, id, &editing.typed);
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, accent),
            egui::StrokeKind::Inside,
        );
        // The field takes the keyboard the moment editing starts. egui
        // drops that focus again while the field is new under the click
        // that started it, or in the view's first, unseen frame; it is
        // asked again after the field until it holds it.
        if editing.focus {
            if response.has_focus() {
                editing.focus = false;
            } else {
                format_bar::select(ui, id, &editing.typed, (end, end));
                ui.ctx().request_repaint();
            }
            return;
        }
        if response.lost_focus()
            && let Some(text) = self.commit()
        {
            asked.push(Asked::Write(text));
        } else if response.lost_focus() {
            self.editing = None;
        }
    }
}

/// Paints the words `laid` of a cell in `rect`, lined up as `align` says,
/// and gives where its boxes, tags and links lie.
fn paint_cell(ui: &mut Ui, laid: &LaidCell, rect: Rect, align: Align, look: &Look) -> Spots {
    let words = laid.size(ui, look);
    let left = text_marks_view::aligned(align, rect.width(), words.x, PAD.x);
    let top = (rect.height() - words.y) / 2.0;
    let pos = rect.min + egui::vec2(left, top.max(0.0));
    let colour = ui.visuals().text_color();
    // Its words stay inside it, a long one cut at its edge.
    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    clipped.set_clip_rect(rect.intersect(ui.clip_rect()));
    laid.paint(&mut clipped, pos, colour, false)
}

/// A row's number or a column's letter in `rect`, `(shown, label)` its
/// text and what a screen reader says; a button.
fn heading(
    ui: &mut Ui,
    rect: Rect,
    (shown, label): (&str, &str),
    id: Id,
    (tones, colour): (Tones, Color32),
) -> egui::Response {
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, tones.background);
        if resp.hovered() {
            let fill = ui.visuals().widgets.hovered.weak_bg_fill;
            ui.painter().rect_filled(rect.shrink(2.0), 6.0, fill);
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            shown,
            FontId::proportional(14.0),
            colour,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

/// The menu of a row's number or a column's letter, `(column, index,
/// count)` saying which of how many, opened by a click or a right-click
/// where the pointer is; the first column's says whether it holds
/// headings, `headings`. Gives what was picked.
fn menu(
    t: &L10n,
    resp: &egui::Response,
    (column, index, count): (bool, usize, usize),
    headings: bool,
) -> Option<Op> {
    let mut picked = None;
    let mut entries = |ui: &mut Ui| {
        let text = ui.visuals().text_color();
        let items = if column {
            [
                (Op::Before, t.table_column_before()),
                (Op::After, t.table_column_after()),
                (Op::Back, t.table_column_left()),
                (Op::Forward, t.table_column_right()),
                (Op::Delete, t.table_column_delete()),
            ]
        } else {
            [
                (Op::Before, t.table_row_above()),
                (Op::After, t.table_row_below()),
                (Op::Back, t.table_row_up()),
                (Op::Forward, t.table_row_down()),
                (Op::Delete, t.table_row_delete()),
            ]
        };
        for (op, words) in items {
            let enabled = match op {
                Op::Back => index > 0,
                Op::Forward => index + 1 < count,
                _ => true,
            };
            let colour = if op == Op::Delete {
                ui.visuals().error_fg_color
            } else {
                text
            };
            if ui
                .add_enabled(
                    enabled,
                    egui::Button::new(RichText::new(words).color(colour)),
                )
                .clicked()
            {
                picked = Some(op);
                ui.close();
            }
        }
        if column && index == 0 {
            ui.separator();
            let mut on = headings;
            let words = RichText::new(t.table_heading_column()).color(text);
            if ui.checkbox(&mut on, words).changed() {
                picked = Some(Op::Headings);
                ui.close();
            }
        }
    };
    // The right-click's menu has its own id: on a click the context menu
    // closes its popup, which would otherwise be this one just opened.
    Popup::menu(resp).show(&mut entries);
    Popup::context_menu(resp)
        .id(Popup::default_response_id(resp).with("context"))
        .show(&mut entries);
    picked
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

/// Whether the row or column `deleting` says holds words.
fn holds_words(table: &Table, deleting: Deleting) -> bool {
    if deleting.column {
        table
            .rows
            .iter()
            .filter_map(|row| row.cells.get(deleting.index))
            .any(|cell| cell.end > cell.start)
    } else {
        table
            .rows
            .get(deleting.index)
            .is_some_and(|row| row.cells.iter().any(|cell| cell.end > cell.start))
    }
}

/// The cell after `(row, column)` in `table`, row by row, the empty row
/// after the last one included.
fn next_cell(table: &Table, (row, column): (usize, usize)) -> (usize, usize) {
    if column + 1 < table.columns() {
        (row, column + 1)
    } else if row < table.rows.len() {
        (row + 1, 0)
    } else {
        (row, column)
    }
}

/// The cell before `(row, column)` in `table`, row by row.
fn previous_cell(table: &Table, (row, column): (usize, usize)) -> (usize, usize) {
    if column > 0 {
        (row, column.min(table.columns()) - 1)
    } else if row > 0 {
        (row - 1, table.columns().saturating_sub(1))
    } else {
        (row, column)
    }
}

/// The letter of the column `index`: A to Z, then AA and on.
pub fn letter(index: usize) -> String {
    let mut name = Vec::new();
    let mut i = index;
    loop {
        name.push(char::from(b'A' + (i % 26) as u8));
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    name.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "| Item | Qty |\n| --- | --- |\n| milk | 2 |";

    fn view(text: &str, cell: Option<(usize, usize)>) -> TableView {
        TableView::new(Opening {
            note: Some("n".into()),
            text: text.into(),
            at: 0,
            cell,
            tags: true,
            boxes: Some("n".into()),
        })
    }

    #[test]
    fn columns_are_lettered_as_a_sheet_letters_them() {
        assert_eq!(letter(0), "A");
        assert_eq!(letter(25), "Z");
        assert_eq!(letter(26), "AA");
        assert_eq!(letter(27), "AB");
        assert_eq!(letter(701), "ZZ");
        assert_eq!(letter(702), "AAA");
    }

    #[test]
    fn a_cell_typed_into_is_written_and_one_left_alone_is_not() {
        let mut edited = view(TABLE, Some((1, 1)));
        assert_eq!(edited.commit(), None);
        let mut edited = view(TABLE, Some((1, 1)));
        if let Some(editing) = &mut edited.editing {
            editing.typed = "3 | 4".into();
        }
        let written = edited.commit();
        assert_eq!(
            written.as_deref(),
            Some("| Item | Qty |\n| --- | --- |\n| milk | 3 \\| 4 |")
        );
        assert_eq!(edited.editing, None);
    }

    #[test]
    fn the_empty_row_and_column_add_one_only_with_words() {
        let mut blank = view(TABLE, Some((2, 0)));
        if let Some(editing) = &mut blank.editing {
            editing.typed = "  ".into();
        }
        assert_eq!(blank.commit(), None);
        let mut row = view(TABLE, Some((2, 0)));
        if let Some(editing) = &mut row.editing {
            editing.typed = "eggs".into();
        }
        assert_eq!(
            row.commit().as_deref(),
            Some("| Item | Qty |\n| --- | --- |\n| milk | 2 |\n| eggs |  |")
        );
        let mut column = view(TABLE, Some((0, 2)));
        if let Some(editing) = &mut column.editing {
            editing.typed = "Shop".into();
        }
        assert_eq!(
            column.commit().as_deref(),
            Some("| Item | Qty | Shop |\n| --- | --- | --- |\n| milk | 2 |  |")
        );
    }

    #[test]
    fn deleting_the_last_row_takes_the_table_and_leaves() {
        let mut shown = view("| a |\n| --- |", None);
        let mut asked = Vec::new();
        shown.operate(Op::Delete, false, 0, &mut asked);
        // "a" is a word: asked first.
        assert_eq!(asked, []);
        let deleting = shown.deleting.take();
        assert_eq!(
            deleting,
            Some(Deleting {
                column: false,
                index: 0,
                gone: true
            })
        );
        if let Some(deleting) = deleting {
            shown.delete(deleting, &mut asked);
        }
        assert_eq!(asked, [Asked::Write(String::new()), Asked::Close]);
        // An empty row goes without asking.
        let mut empty = view("|  |  |\n| --- | --- |\n| x |  |", None);
        let mut asked = Vec::new();
        empty.operate(Op::Delete, true, 1, &mut asked);
        assert_eq!(asked, [Asked::Write("|  |\n| --- |\n| x |".into())]);
    }

    #[test]
    fn tab_goes_row_by_row_into_the_empty_row() {
        let Some(table) = table::at(TABLE, 0) else {
            panic!("a table");
        };
        assert_eq!(next_cell(&table, (0, 0)), (0, 1));
        assert_eq!(next_cell(&table, (0, 1)), (1, 0));
        assert_eq!(next_cell(&table, (1, 1)), (2, 0));
        assert_eq!(next_cell(&table, (2, 1)), (2, 1));
        assert_eq!(previous_cell(&table, (1, 0)), (0, 1));
        assert_eq!(previous_cell(&table, (0, 0)), (0, 0));
    }

    #[test]
    fn a_view_to_read_edits_nothing() {
        let mut read = TableView::new(Opening {
            note: None,
            text: TABLE.into(),
            at: 0,
            cell: Some((0, 0)),
            tags: false,
            boxes: None,
        });
        assert_eq!(read.editing, None);
        assert_eq!(read.commit(), None);
    }
}
