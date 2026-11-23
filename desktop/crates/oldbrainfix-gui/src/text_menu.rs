//! The right-click menu of a desktop text field (#253): Cut, Copy, Paste
//! and Select all in every field one types into, and, with a selection
//! in a formatted text, the format bar's Bold, Italic, Code, Link and
//! Code block for it. It opens at the pointer; an entry that has nothing
//! to work on is greyed.
//!
//! Cut, Copy and Paste, Edit's as well, come to the field at the start
//! of its window's next frame as the cut, copy and paste Ctrl+X, Ctrl+C
//! and Ctrl+V bring, so the field takes them as it takes the keys (#319).
//! Asked of the window instead, they came two frames late and only with
//! the next input, and never in a Note window of its own. Paste brings
//! the system clipboard's text; where it cannot be read, the window is
//! asked for it. The field keeps the keyboard through the menu, and a
//! right-click on its selection keeps it selected, which a press in a
//! field otherwise turns into the cursor at the pointer.
//!
//! In the fields of the Note open, in the Note dialog and a Note's own
//! window, the menu also has Search (obf-1055, C9c): the words selected
//! go into the Note's find bar, as they read. The App says where it is offered with
//! [`searches`] and takes the words asked with [`take_searched`]; the
//! menu of a text only shown asks the same ([`crate::read_text`]).

use egui::plugin::Plugin;
use egui::text::CCursorRange;
use egui::widgets::text_edit::TextEditState;
use egui::{
    Context, Event, Id, PointerButton, Popup, RawInput, Response, RichText, Ui, ViewportCommand,
    ViewportId,
};

use oldbrainfix_core::text_format;

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
    /// The words selected into the Note's find bar (obf-1055), asked of
    /// the App by [`ask_search`].
    Search,
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
    follow(ui, response.dragged());
    keep_selection(ui, response);
    let (start, end) = format_bar::selection(ui, id, text);
    let selected = start < end;
    // As the find bar finds: a formatted text as it reads, without the
    // ids of its boxes.
    let format = if formats.is_empty() {
        text_format::PLAIN
    } else {
        text_format::MARKDOWN
    };
    let selection = text.get(start..end).unwrap_or_default();
    let words = search_words(&text_format::visible(format, selection));
    let offers_search = offers_search(ui.ctx());
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
        if offers_search {
            entry(ui, t.menu_search(), !words.is_empty(), Pick::Search);
        }
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
    if picked == Some(Pick::Search) {
        // The find bar takes the keyboard.
        ask_search(ui.ctx(), &words);
    } else if picked.is_some() || clicked_menu {
        ui.memory_mut(|m| m.request_focus(id));
    }
    picked
}

/// Where the fields and texts drawn offer Search, in the window drawn
/// now.
fn searches_id(ctx: &Context) -> Id {
    Id::new(("text menu searches", ctx.viewport_id()))
}

/// The words Search asked in the window drawn now.
fn searched_id(ctx: &Context) -> Id {
    Id::new(("text menu searched", ctx.viewport_id()))
}

/// Whether the menus of the fields and texts drawn from now on offer
/// Search (obf-1055): the App's while it draws the Note open. Gives what
/// it was, to be put back.
pub fn searches(ctx: &Context, on: bool) -> bool {
    let id = searches_id(ctx);
    ctx.data_mut(|d| {
        let before = d.get_temp::<bool>(id).unwrap_or(false);
        d.insert_temp(id, on);
        before
    })
}

/// Whether what is drawn now offers Search.
pub(crate) fn offers_search(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(searches_id(ctx)))
        .unwrap_or(false)
}

/// The words of `selected` as the find bar's one line takes them: its
/// lines and spaces each one space, none at the ends.
pub(crate) fn search_words(selected: &str) -> String {
    selected.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Asks the App to find `words` in the Note open: Search picked.
pub(crate) fn ask_search(ctx: &Context, words: &str) {
    let id = searched_id(ctx);
    ctx.data_mut(|d| d.insert_temp(id, words.to_string()));
    ctx.request_repaint();
}

/// The words Search asked in the window drawn now, once.
pub fn take_searched(ctx: &Context) -> Option<String> {
    let id = searched_id(ctx);
    ctx.data_mut(|d| d.remove_temp::<String>(id))
}

/// Does `pick` in the field `id` holding `text`; a format is left to its
/// bar.
pub fn apply(ui: &Ui, id: Id, text: &str, pick: Pick) {
    let clip = match pick {
        Pick::Cut => Clip::Cut,
        Pick::Copy => Clip::Copy,
        Pick::Paste => Clip::Paste,
        Pick::SelectAll => {
            format_bar::select(ui, id, text, (0, text.len()));
            return;
        }
        Pick::Search | Pick::Format(_) => return,
    };
    clip_into(ui.ctx(), id, clip);
}

/// Cut, Copy or Paste, of the menu or of Edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clip {
    Cut,
    Copy,
    Paste,
}

/// Reads the system clipboard's text; `None` where it cannot, or it
/// holds none.
pub type ReadClipboard = fn() -> Option<String>;

/// The cuts, copies and pastes asked, each for the next frame of its
/// window, and where Paste reads the clipboard.
#[derive(Debug, Default)]
pub struct Clips {
    asked: Vec<(ViewportId, Event)>,
    read: Option<ReadClipboard>,
}

impl Plugin for Clips {
    fn debug_name(&self) -> &'static str {
        "text menu clips"
    }

    /// Hands the window drawn now what was asked for it, as the keys
    /// would bring it; what was asked for a window closed since is
    /// dropped.
    fn input_hook(&mut self, _ctx: &Context, input: &mut RawInput) {
        let viewport = input.viewport_id;
        let open = |id: &ViewportId| input.viewports.is_empty() || input.viewports.contains_key(id);
        let mut kept = Vec::new();
        for (id, event) in std::mem::take(&mut self.asked) {
            if id == viewport {
                input.events.push(event);
            } else if open(&id) {
                kept.push((id, event));
            }
        }
        self.asked = kept;
    }
}

/// Where Paste reads the system clipboard's text, for every window of
/// `ctx`.
pub fn read_clipboard_by(ctx: &Context, read: ReadClipboard) {
    ctx.plugin_or_default::<Clips>().lock().read = Some(read);
}

/// Gives the field `id` `clip` at the start of its window's next frame,
/// and the keyboard now. A Paste where the clipboard's text cannot be
/// read asks the window for it.
pub fn clip_into(ctx: &Context, id: Id, clip: Clip) {
    ctx.memory_mut(|m| m.request_focus(id));
    let plugin = ctx.plugin_or_default::<Clips>();
    let mut clips = plugin.lock();
    let event = match clip {
        Clip::Cut => Event::Cut,
        Clip::Copy => Event::Copy,
        Clip::Paste => match clips.read.and_then(|read| read()) {
            Some(text) => Event::Paste(text),
            None => {
                drop(clips);
                ctx.send_viewport_cmd(ViewportCommand::RequestPaste);
                return;
            }
        },
    };
    clips.asked.push((ctx.viewport_id(), event));
    drop(clips);
    ctx.request_repaint();
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

/// How fast a drag past the edge scrolls, in points a second for each
/// point the pointer is out.
const EDGE_SPEED: f32 = 10.0;

/// While a selection is being dragged, `dragging`, in the text whose
/// `ui` lies in a scrolled view (#256): the wheel scrolls the view, and
/// the pointer past its top or bottom edge scrolls it by itself, faster
/// the further out it is. The text's selection follows the pointer as
/// the view moves under it.
pub fn follow(ui: &Ui, dragging: bool) {
    if !dragging {
        return;
    }
    let view = ui.clip_rect();
    let (wheel, pointer, dt) = ui.input(|i| {
        (
            i.smooth_scroll_delta.y,
            i.pointer.interact_pos(),
            i.stable_dt.min(0.1),
        )
    });
    let mut down = -wheel;
    if wheel != 0.0 {
        ui.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
    }
    if let Some(at) = pointer {
        let out = if at.y < view.top() {
            at.y - view.top()
        } else if at.y > view.bottom() {
            at.y - view.bottom()
        } else {
            0.0
        };
        if out != 0.0 {
            down += out * EDGE_SPEED * dt;
            ui.ctx().request_repaint();
        }
    }
    if down == 0.0 {
        return;
    }
    // The view's top `down` further on: its scroll area puts the item
    // spacing above what it brings into view.
    let spacing = ui.spacing().item_spacing;
    let top = egui::pos2(view.left() + spacing.x, view.top() + down + spacing.y);
    ui.scroll_to_rect_animation(
        egui::Rect::from_min_size(top, egui::vec2(1.0, 1.0)),
        Some(egui::Align::Min),
        egui::style::ScrollAnimation::none(),
    );
}
