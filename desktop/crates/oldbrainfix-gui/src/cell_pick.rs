//! Cells of a table picked as a rectangle to be copied (obf-1054), as
//! the phone's `cell_selection.dart` does: a drag that starts on a cell
//! picks every cell from it to the one under the pointer, in the rows as
//! they show, and Shift+click picks from the cell picked first to the
//! one clicked, a single cell where none is. A drag that starts outside
//! a table selects its words as text, through the table too. Ctrl+C
//! (Cmd on macOS) and Copy in the right-click menu put the cells on the
//! clipboard as a table of their own, as the table's Copy does (#324):
//! an HTML table beside its Markdown, the first row picked its header
//! row, the line marking the first column as headings only while that
//! column is picked, each box in the state it shows. Search there takes
//! the words of a single cell. Esc, a click on the table and a press
//! beside it end the pick; the click does nothing else.
//!
//! A table being read keeps its pick here, one per window
//! ([`read_table`]), its copy waiting for the App ([`take_copy`]); the
//! table full screen and the editor's grids keep their own.

use std::collections::BTreeMap;

use egui::{Context, Event, Id, Key, Popup, Pos2, Rect, Response, RichText, Ui};
use oldbrainfix_core::BoxState;
use oldbrainfix_core::markdown::table::{self, Align, TableCopy};

use crate::l10n::L10n;

/// A rectangle of the cells of a table: its rows from `top` to `bottom`
/// in the order shown, the header row 0, and its columns from `left` to
/// `right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRect {
    pub top: usize,
    pub left: usize,
    pub bottom: usize,
    pub right: usize,
}

impl CellRect {
    /// Whether it holds the cell `(row, column)`, its row as shown.
    pub fn holds(self, (row, column): (usize, usize)) -> bool {
        (self.top..=self.bottom).contains(&row) && (self.left..=self.right).contains(&column)
    }

    /// Whether it is a single cell.
    pub fn one(self) -> bool {
        self.top == self.bottom && self.left == self.right
    }
}

/// The cells picked: the cell the pick started on and the one it
/// reached, each by its row as shown and its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

impl Span {
    /// The cells from one to the other.
    pub fn cells(self) -> CellRect {
        CellRect {
            top: self.from.0.min(self.to.0),
            left: self.from.1.min(self.to.1),
            bottom: self.from.0.max(self.to.0),
            right: self.from.1.max(self.to.1),
        }
    }

    /// Whether it lies within a table of `rows` rows and `columns`
    /// columns.
    pub fn within(self, (rows, columns): (usize, usize)) -> bool {
        let cells = self.cells();
        cells.bottom < rows && cells.right < columns
    }
}

/// The cells of the rows `rows`, by their index in the table and in that
/// order, and of the columns `left` to `right` of the table at `at` of
/// `text`, as stored, as a table of their own: each cell as written, its
/// column lining up as it does, the first row its header row, its first
/// column headings while the table's first column is one of them and
/// holds headings. `None` where there is no such table or no cell.
pub fn table_cells(
    text: &str,
    at: usize,
    rows: &[usize],
    (left, right): (usize, usize),
) -> Option<String> {
    let table = table::at(text, at)?;
    let picked: Vec<usize> = rows
        .iter()
        .copied()
        .filter(|&r| r < table.rows.len())
        .collect();
    let to = right.min(table.columns().checked_sub(1)?);
    let (first, rest) = picked.split_first()?;
    if left > to {
        return None;
    }
    let line = |cells: Vec<&str>| format!("| {} |", cells.join(" | "));
    let row = |r: usize| {
        let cells = table.rows.get(r).map(|row| row.cells.as_slice());
        line(
            (left..=to)
                .map(|c| {
                    cells
                        .and_then(|cells| cells.get(c))
                        .and_then(|cell| text.get(cell.start..cell.end))
                        .unwrap_or_default()
                })
                .collect(),
        )
    };
    let aligns = (left..=to)
        .map(|c| match table.aligns.get(c) {
            Some(Align::Left) => ":--",
            Some(Align::Center) => ":-:",
            Some(Align::Right) => "--:",
            _ => "---",
        })
        .collect();
    let mut lines = Vec::new();
    if table.heading_column && left == 0 {
        lines.push(table::HEADING_COLUMN_LINE.to_string());
    }
    lines.push(row(*first));
    lines.push(line(aligns));
    lines.extend(rest.iter().map(|&r| row(r)));
    Some(lines.join("\n"))
}

/// The cells `rows` and `columns` of the table at `at` of `text` as Copy
/// puts them on the clipboard, each box in its state in `states`.
pub fn copy(
    text: &str,
    at: usize,
    (rows, columns): (&[usize], (usize, usize)),
    states: &BTreeMap<String, BoxState>,
) -> Option<TableCopy> {
    let cells = table_cells(text, at, rows, columns)?;
    table::copy(&cells, 0, states)
}

/// What a press, a drag, Shift+click and Esc do to `span`, the cells
/// picked of a table whose cells lie in `bounds` on the layer of `ui`,
/// `at` giving the cell at a point there: the drag of `id` that starts on
/// a cell picks from it to the cell under the pointer, held within
/// `bounds`; Shift+click picks from the cell picked first to the one
/// clicked; Esc and a press beside the cells end it. Gives whether it
/// changed.
pub fn follow(
    ui: &Ui,
    id: Id,
    span: &mut Option<Span>,
    bounds: Rect,
    at: impl Fn(Pos2) -> Option<(usize, usize)>,
) -> bool {
    let before = *span;
    let ctx = ui.ctx();
    let on_layer = |pos: Pos2| ctx.layer_id_at(pos) == Some(ui.layer_id());
    let inside = |pos: Pos2| bounds.contains(pos) && on_layer(pos);
    let (escape, pressed, clicked, shift, origin, latest) = ui.input(|i| {
        (
            i.key_pressed(Key::Escape),
            i.pointer.primary_pressed(),
            i.pointer.primary_clicked(),
            i.modifiers.shift,
            i.pointer.press_origin(),
            i.pointer.latest_pos(),
        )
    });
    if escape {
        *span = None;
    }
    if pressed
        && !shift
        && let Some(pos) = origin
        && on_layer(pos)
        && !inside(pos)
    {
        *span = None;
    }
    // A drag-only widget is dragged from the press on; it picks once the
    // pointer moved as no click does, or was held so long.
    let dragging = ctx.is_being_dragged(id) && ui.input(|i| i.pointer.is_decidedly_dragging());
    let begun_id = id.with("begun");
    let begun = ctx.data(|d| d.get_temp::<bool>(begun_id)).unwrap_or(false);
    if dragging != begun {
        ctx.data_mut(|d| d.insert_temp(begun_id, dragging));
    }
    // A table scrolled out of view mid-drag has no cells to reach.
    if dragging && bounds.width() >= 1.0 && bounds.height() >= 1.0 {
        if !begun {
            let from = origin.filter(|&p| bounds.contains(p)).and_then(&at);
            *span = from.map(|from| Span { from, to: from });
        }
        let edge = egui::vec2(0.5, 0.5);
        let held = latest.map(|p| p.clamp(bounds.min + edge, bounds.max - edge));
        if let (Some(picked), Some(cell)) = (span.as_mut(), held.and_then(&at)) {
            picked.to = cell;
        }
    }
    if clicked
        && shift
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        && inside(pos)
        && let Some(cell) = at(pos)
    {
        let from = span.map_or(cell, |s| s.from);
        *span = Some(Span { from, to: cell });
    }
    let changed = *span != before;
    if changed {
        ctx.request_repaint();
    }
    changed
}

/// Whether a click on a table whose cells `span` are picked is the
/// pick's: with Shift it picked ([`follow`]); without it, it ends the
/// pick and does nothing else.
pub fn takes_click(ui: &Ui, span: &mut Option<Span>) -> bool {
    if ui.input(|i| i.modifiers.shift) {
        return true;
    }
    span.take().is_some()
}

/// Whether Ctrl+C came this frame, not the copy a right-click on a text
/// only read asks of its selection ([`crate::read_text`]).
pub fn copy_asked(ui: &Ui) -> bool {
    !crate::read_text::copying(ui.ctx())
        && ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Copy)))
}

/// Takes Ctrl+C out of this frame's input, before a field sees it;
/// whether it came.
pub fn take_copy_key(ui: &Ui) -> bool {
    ui.input_mut(|i| {
        let before = i.events.len();
        i.events.retain(|e| !matches!(e, Event::Copy));
        i.events.len() != before
    })
}

/// Takes Esc out of this frame's input; whether it came.
pub fn take_escape(ui: &Ui) -> bool {
    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
}

/// What the right-click menu of the cells picked offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Copy,
    Search,
}

/// The right-click menu of the cell `response` while it is picked: Copy,
/// and where the Note open is searched, Search, with the words `words`
/// of a single cell, greyed for none. Search asks the App at once; gives
/// what was picked.
pub fn menu(response: &Response, t: &L10n, words: Option<&str>) -> Option<Pick> {
    let ctx = response.ctx.clone();
    let words = words
        .map(crate::text_menu::search_words)
        .unwrap_or_default();
    let searches = crate::text_menu::offers_search(&ctx);
    let mut picked = None;
    Popup::context_menu(response).show(|ui| {
        let colour = ui.visuals().text_color();
        let mut entry = |ui: &mut Ui, label: &str, enabled: bool, pick: Pick| {
            let button = egui::Button::new(RichText::new(label).color(colour));
            if ui.add_enabled(enabled, button).clicked() {
                picked = Some(pick);
                ui.close();
            }
        };
        entry(ui, t.menu_copy(), true, Pick::Copy);
        if searches {
            entry(
                ui,
                crate::text_menu::search_label(&ctx, t),
                !words.is_empty(),
                Pick::Search,
            );
        }
    });
    if picked == Some(Pick::Search) {
        crate::text_menu::ask_search(&ctx, &words);
    }
    picked
}

/// The pick of a table being read in a window: the table's
/// [`crate::text_marks_view`] key, the cells, the frame it was drawn in
/// last, the words of a single cell for Search, and whether the menu's
/// Copy asked for them.
#[derive(Debug, Clone, Default)]
struct Read {
    key: Option<Id>,
    span: Option<Span>,
    drawn: u64,
    words: String,
    asked: bool,
}

fn read_id(ctx: &Context) -> Id {
    Id::new(("read cell pick", ctx.viewport_id()))
}

fn copy_id() -> Id {
    Id::new("cell pick copy")
}

/// The pick of a table being read in the window drawn now, while the
/// table was drawn in the frame before.
fn live(ctx: &Context) -> Option<Read> {
    let slot = read_id(ctx);
    let read: Read = ctx.data(|d| d.get_temp(slot))?;
    let frame = ctx.cumulative_frame_nr();
    (read.span.is_some() && read.drawn + 1 >= frame).then_some(read)
}

/// The cells of the table being read under `key`, `size` rows and
/// columns big, its cells in `bounds`, `at` giving the cell at a point:
/// [`follow`] for the drag `id`, the pick kept for the window. Gives the
/// cells picked and whether they are to be copied now, by Ctrl+C or the
/// menu's Copy; `words` gives the words of a single cell for Search.
pub fn read_table(
    ui: &Ui,
    (key, id): (Id, Id),
    (bounds, size): (Rect, (usize, usize)),
    at: impl Fn(Pos2) -> Option<(usize, usize)>,
    words: impl Fn((usize, usize)) -> String,
) -> (Option<CellRect>, bool) {
    let ctx = ui.ctx();
    let held = live(ctx).filter(|r| r.key == Some(key));
    let mut span = held.as_ref().and_then(|r| r.span);
    follow(ui, id, &mut span, bounds, at);
    span = span.filter(|s| s.within(size));
    if held.is_none() && span.is_none() {
        // Another table's, or none.
        return (None, false);
    }
    let mut read = held.unwrap_or_default();
    read.key = Some(key);
    read.span = span;
    read.drawn = ctx.cumulative_frame_nr();
    let cells = span.map(Span::cells);
    read.words = cells
        .filter(|c| c.one())
        .map(|c| words((c.top, c.left)))
        .unwrap_or_default();
    let copies = cells.is_some() && (std::mem::take(&mut read.asked) || copy_asked(ui));
    let slot = read_id(ctx);
    ctx.data_mut(|d| d.insert_temp(slot, read));
    (cells, copies)
}

/// Ends the pick of the table `key` being read, as a click on it does;
/// whether there was one.
pub fn end_read(ctx: &Context, key: Id) -> bool {
    let Some(read) = live(ctx).filter(|r| r.key == Some(key)) else {
        return false;
    };
    let slot = read_id(ctx);
    ctx.data_mut(|d| d.insert_temp(slot, Read { span: None, ..read }));
    ctx.request_repaint();
    true
}

/// Whether the table `key` being read has cells picked.
pub fn read_picks(ctx: &Context, key: Id) -> bool {
    live(ctx).is_some_and(|r| r.key == Some(key))
}

/// Whether cells of a table being read are picked in the window drawn
/// now, and the words of the cell when a single one is.
pub fn read_pick(ctx: &Context) -> Option<String> {
    live(ctx).map(|r| r.words)
}

/// Asks the table being read whose cells are picked for its copy, in the
/// next frame: the menu's Copy.
pub fn ask_copy(ctx: &Context) {
    if let Some(mut read) = live(ctx) {
        read.asked = true;
        let slot = read_id(ctx);
        ctx.data_mut(|d| d.insert_temp(slot, read));
        ctx.request_repaint();
    }
}

/// Ends the pick of the table being read in the window drawn now.
pub fn end_any_read(ctx: &Context) {
    let slot = read_id(ctx);
    ctx.data_mut(|d| d.remove_temp::<Read>(slot));
}

/// Leaves `copy`, the cells of a table being read, for the App to put on
/// the clipboard.
pub fn put_copy(ctx: &Context, copy: TableCopy) {
    ctx.data_mut(|d| d.insert_temp(copy_id(), Some(copy)));
    ctx.request_repaint();
}

/// The cells a table being read left to be copied, once.
pub fn take_copy(ctx: &Context) -> Option<TableCopy> {
    ctx.data_mut(|d| d.remove_temp::<Option<TableCopy>>(copy_id()))
        .flatten()
}

#[cfg(test)]
mod tests;
