//! The right-click menu of a desktop text field (#253): Cut, Copy, Paste
//! and Select all in every field one types into, and, with a selection
//! in a formatted text, the format bar's Bold, Italic, Code, Link and
//! Code block for it. It opens at the pointer; an entry that has nothing
//! to work on is greyed.
//!
//! Cut, Copy and Paste ask the window's clipboard as their keys do, as
//! Edit's do, so the field takes them as it takes Ctrl+X, Ctrl+C and
//! Ctrl+V. The field keeps the keyboard through the menu, and a
//! right-click on its selection keeps it selected, which a press in a
//! field otherwise turns into the cursor at the pointer.

use egui::text::CCursorRange;
use egui::widgets::text_edit::TextEditState;
use egui::{Id, PointerButton, Popup, Response, RichText, Ui, ViewportCommand};

use crate::format_bar::{self, Format};
use crate::l10n::L10n;

/// The format bar's entries the menu offers, in its order.
pub const FORMATS: [Format; 5] = [
    Format::Bold,
    Format::Italic,
    Format::Code,
    Format::Link,
    Format::CodeBlock,
];

/// An entry of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Cut,
    Copy,
    Paste,
    SelectAll,
    /// One of the format bar's, for the field to put on as its bar does.
    Format(Format),
}

/// The menu of the field `response` holding `text`, whose selection the
/// menu's entries take: Cut, Copy, Paste and Select all done at once.
pub fn show(ui: &Ui, t: &L10n, response: &Response, text: &str) {
    if let Some(pick) = entries(ui, t, (response, text), &[], true) {
        apply(ui, response.id, text, pick);
    }
}

/// The menu of the password field `response` holding `text`: Cut and
/// Copy greyed, as the window copies no password.
pub fn secret(ui: &Ui, t: &L10n, response: &Response, text: &str) {
    if let Some(pick) = entries(ui, t, (response, text), &[], false) {
        apply(ui, response.id, text, pick);
    }
}

/// The menu of the field `response` holding `text`, with those of
/// `formats` that are in [`FORMATS`] while some of it is selected; the
/// entry picked, done by [`apply`] or by the field's own format bar.
pub fn menu(
    ui: &Ui,
    t: &L10n,
    response: &Response,
    text: &str,
    formats: &[Format],
) -> Option<Pick> {
    entries(ui, t, (response, text), formats, true)
}

/// The menu, Cut and Copy greyed unless the field `copies`.
fn entries(
    ui: &Ui,
    t: &L10n,
    (response, text): (&Response, &str),
    formats: &[Format],
    copies: bool,
) -> Option<Pick> {
    let id = response.id;
    keep_selection(ui, response);
    let (start, end) = format_bar::selection(ui, id, text);
    let selected = start < end;
    let mut picked = None;
    let shown = Popup::context_menu(response).show(|ui| {
        let colour = ui.visuals().text_color();
        let mut entry = |ui: &mut Ui, label: &str, enabled: bool, pick: Pick| {
            let button = egui::Button::new(RichText::new(label).color(colour));
            if ui.add_enabled(enabled, button).clicked() {
                picked = Some(pick);
                ui.close();
            }
        };
        entry(ui, t.menu_cut(), copies && selected, Pick::Cut);
        entry(ui, t.menu_copy(), copies && selected, Pick::Copy);
        entry(ui, t.paste(), true, Pick::Paste);
        entry(ui, t.menu_select_all(), !text.is_empty(), Pick::SelectAll);
        let offered: Vec<Format> = FORMATS
            .into_iter()
            .filter(|f| formats.contains(f))
            .collect();
        if selected && !offered.is_empty() {
            ui.separator();
            for format in offered {
                entry(ui, format.label(t), true, Pick::Format(format));
            }
        }
    });
    // A click on the menu takes the keyboard from the field, which keeps
    // it: what it does on losing it, a save or a close, does not happen.
    let clicked_menu = shown.is_some_and(|shown| {
        ui.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|at| shown.response.rect.contains(at))
        })
    });
    if picked.is_some() || clicked_menu {
        ui.memory_mut(|m| m.request_focus(id));
    }
    picked
}

/// Does `pick` in the field `id` holding `text`; a format is left to its
/// bar.
pub fn apply(ui: &Ui, id: Id, text: &str, pick: Pick) {
    let asked = match pick {
        Pick::Cut => ViewportCommand::RequestCut,
        Pick::Copy => ViewportCommand::RequestCopy,
        Pick::Paste => ViewportCommand::RequestPaste,
        Pick::SelectAll => {
            format_bar::select(ui, id, text, (0, text.len()));
            return;
        }
        Pick::Format(_) => return,
    };
    ui.memory_mut(|m| m.request_focus(id));
    ui.ctx().send_viewport_cmd(asked);
}

/// A press of the right button in the field `response` puts its cursor
/// at the pointer; on its selection, the selection is put back. Else
/// the field's selection is remembered for the next press.
fn keep_selection(ui: &Ui, response: &Response) {
    let id = response.id;
    let kept = id.with("menu selection");
    let range = TextEditState::load(ui.ctx(), id).and_then(|s| s.cursor.char_range());
    let pressed = response.contains_pointer()
        && ui.input(|i| i.pointer.button_pressed(PointerButton::Secondary));
    let before: Option<CCursorRange> = ui.data(|d| d.get_temp(kept));
    if pressed
        && let (Some(before), Some(now)) = (before, range)
        && let Some(mut state) = TextEditState::load(ui.ctx(), id)
    {
        let (a, b) = (before.primary.index, before.secondary.index);
        let at = now.primary.index;
        if a != b && a.min(b) <= at && at <= a.max(b) {
            state.cursor.set_char_range(Some(before));
            state.store(ui.ctx(), id);
            return;
        }
    }
    if let Some(range) = range {
        ui.data_mut(|d| d.insert_temp(kept, range));
    }
}
