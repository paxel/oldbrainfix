//! Paste in the capture (#130), in the window and the capture popup
//! alike: its button, and Ctrl+V in the empty field, take the text and
//! the picture on the clipboard, read only then. The text goes into the
//! field, the picture into the Note; a clipboard with neither leaves a
//! grey line above the field, no announcement (UI laws, "Capture").
//!
//! The window's Ctrl+V brings the clipboard's text as a paste, and
//! nothing at all when it holds only a picture; only the key's release
//! still comes then. So the empty field takes the paste itself, before
//! the field would, and a release without one pastes the picture.

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Event, Id, Key, RichText, Ui};

use crate::icons;
use crate::l10n::L10n;
use crate::media::Clipboard;

/// What a paste found on the clipboard.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Pasted {
    /// Its text, not blank.
    pub text: Option<String>,
    /// Its picture, as a photo to keep.
    pub photo: Option<Vec<u8>>,
}

impl Pasted {
    /// Whether there was nothing to use.
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.photo.is_none()
    }
}

/// What is on `clipboard`: `text` when the paste brought it, else the
/// clipboard's own, and its picture.
pub fn take(clipboard: &mut dyn Clipboard, text: Option<String>) -> Pasted {
    let text = match text {
        Some(text) => Some(text.replace("\r\n", "\n")),
        None => clipboard.text(),
    };
    Pasted {
        text: text.filter(|t| !t.trim().is_empty()),
        photo: clipboard.photo(),
    }
}

/// Whether Ctrl+V asks for a paste in the field `id`, blank when `blank`:
/// `Some` with the text the window's paste brought, if it brought any,
/// which the field then does not get as well. `held` keeps, until the
/// keys are let go, that the paste was taken, so its release does not
/// paste again. A field with text pastes as it always does.
pub fn ctrl_v(ui: &Ui, id: Id, blank: bool, held: &mut bool) -> Option<Option<String>> {
    let focused = ui.memory(|m| m.has_focus(id));
    ui.input_mut(|i| {
        let released = i.events.iter().any(|e| {
            matches!(e, Event::Key { key: Key::V, pressed: false, modifiers, .. } if modifiers.command)
        });
        let mut asked = None;
        if focused && blank {
            let pasted = i.events.iter().find_map(|e| match e {
                Event::Paste(text) => Some(text.clone()),
                _ => None,
            });
            if let Some(text) = pasted {
                // Held down, it repeats: one paste is enough.
                i.events.retain(|e| !matches!(e, Event::Paste(_)));
                *held = true;
                asked = Some(Some(text));
            } else if released && !*held {
                asked = Some(None);
            }
        }
        if released {
            *held = false;
        }
        asked
    })
}

/// Puts `text` into the field `id` holding `field`: in place of what is
/// selected, else at the cursor, else on a new line at the end, the
/// cursor after it and the keyboard on the field.
pub fn insert(ui: &Ui, id: Id, field: &mut String, text: &str) {
    let chars = field.chars().count();
    let byte =
        |field: &str, at: usize| field.char_indices().nth(at).map_or(field.len(), |(b, _)| b);
    let range = TextEditState::load(ui.ctx(), id)
        .and_then(|s| s.cursor.char_range())
        .filter(|r| r.primary.index.0 <= chars && r.secondary.index.0 <= chars);
    let end = match range {
        Some(range) => {
            let (a, b) = (range.primary.index.0, range.secondary.index.0);
            let (from, to) = (byte(field, a.min(b)), byte(field, a.max(b)));
            field.replace_range(from..to, text);
            a.min(b) + text.chars().count()
        }
        None => {
            if !field.is_empty() && !field.ends_with('\n') {
                field.push('\n');
            }
            field.push_str(text);
            field.chars().count()
        }
    };
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(end))));
    state.store(ui.ctx(), id);
    ui.memory_mut(|m| m.request_focus(id));
    ui.ctx().request_repaint();
}

/// The Paste button with its icon; whether it was clicked.
pub fn button(ui: &mut Ui, t: &L10n) -> bool {
    let text = ui.visuals().text_color();
    ui.horizontal(|ui| {
        icons::show(ui, icons::PASTE, 20.0, text);
        ui.button(RichText::new(t.paste()).color(text)).clicked()
    })
    .inner
}

/// The grey line above the field after a paste found nothing.
pub fn empty_line(ui: &mut Ui, t: &L10n) {
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.paste_empty()).color(weak));
}
