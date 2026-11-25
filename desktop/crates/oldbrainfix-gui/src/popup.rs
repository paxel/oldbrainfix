//! The capture popup (UI laws, "Capture"): a small window over all
//! others that the hotkey, the tray's Capture and `oldbrainfix capture`
//! open. It is the quick note (#352): the text, the three Nudge buttons
//! and Open in the editor, nothing else, and it never asks anything.
//! Enter keeps the text with a Nudge in 1 h, the three buttons with
//! theirs, and leaving it with Esc or its close button keeps what is
//! typed the same way as Enter, as leaving the phone's capture screen
//! does; empty, it just closes.
//!
//! Open in the editor, a full-width button under the Nudge buttons
//! (obf-1080), saves the Note as Enter does and opens it in the window's
//! Note dialog, where the rest is done to it; it can be clicked once
//! something is typed or held. A Note's place, Kind, Reminder,
//! recording and the rest are the Note dialog's. With the Nudge buttons
//! switched off in Settings ▸ Quick note, one Save takes
//! their place and nudges in 1 h.
//!
//! Ctrl+V in the empty field, and Paste in its right-click menu, take
//! the clipboard's text into the field and its picture into the Note,
//! held above the text and saved with the rest; a clipboard with neither
//! leaves a grey line above the field (#130). Enter saves, Shift+Enter
//! starts a new line (#277); a pasted text keeps its line breaks in the
//! Note all the same. Ctrl+1, Ctrl+2 and Ctrl+3 (Cmd on macOS) save with
//! the Nudge buttons' times, Ctrl+Enter as Enter does.
//!
//! Closing the popup gives the keyboard back to the window that had it
//! where the system does so for a closed window: on X11, Windows and
//! macOS mostly. On Wayland the compositor decides that, and whether the
//! popup stays on top at all (winit has no window levels there).

use std::sync::Arc;

use chrono::Duration;
use egui::{
    Event, FontId, Frame, Id, InputState, Key, KeyboardShortcut, Margin, Modifiers, RichText,
    ScrollArea, TextEdit, Ui, ViewportCommand,
};
use oldbrainfix_core::{APP_ID, Note, Notes, Store, markdown, text_format};

use crate::app::{Wall, done_form, failure_form, local_wall, name_field};
use crate::capture_choices::{self, CaptureChoices, Captured, Held, QuickNote};
use crate::cli::note_text;
use crate::icons;
use crate::l10n::L10n;
use crate::media::Clipboard;
use crate::paste;
use crate::text_menu::{self, Pick};
use crate::theme;

/// The popup's size.
const SIZE: [f32; 2] = [480.0, 340.0];
/// How high the field grows before its lines scroll: four of them.
const FIELD_HEIGHT: f32 = 100.0;

/// What the popup is after a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Open,
    /// Done with: it saved what was typed, or nothing was.
    Closed,
    /// Done with, the Note of this id saved, to be opened in the
    /// window's Note dialog (Open in the editor, #142, #146, #352).
    Edit(String),
}

/// Captures `text` as a new Unsorted Note nudged `nudge_in` from now,
/// without the blanks around it; `None` when it is blank.
pub fn capture(
    store: &Store,
    text: &str,
    nudge_in: Duration,
) -> oldbrainfix_core::Result<Option<Note>> {
    let Some(text) = note_text(text) else {
        return Ok(None);
    };
    // Typed or pasted with a format, it is a formatted text (#127).
    let format = if markdown::uses(text) {
        text_format::MARKDOWN
    } else {
        text_format::PLAIN
    };
    Notes::new(store)
        .capture_as(text, format, None, Some(nudge_in))
        .map(Some)
}

/// What is announced at the top of the popup: what it says, why for a
/// failure (`None` for a job that finished another way than asked, as
/// the window's announcements), and the full text.
struct Failure {
    said: &'static str,
    cause: Option<&'static str>,
    details: String,
}

impl Failure {
    /// What went wrong as `error` says, said without its text.
    fn of(t: &L10n, error: &oldbrainfix_core::Error) -> Failure {
        Failure {
            said: t.action_failed(),
            cause: Some(t.action_failed_cause()),
            details: error.to_string(),
        }
    }
}

pub struct CapturePopup {
    store: Arc<Store>,
    t: L10n,
    text: String,
    /// Whether the field is still to take the keyboard.
    focus: bool,
    /// What went wrong, announced at the top without values (UI laws,
    /// "Announcements"); the text stays.
    error: Option<Failure>,
    /// Whether the dialog with the failure's cause, fix and full text
    /// shows.
    failure_open: bool,
    saved: Option<Box<dyn Fn()>>,
    clipboard: Option<Box<dyn Clipboard>>,
    /// The pictures pasted, held until the Note is saved.
    photos: Vec<Vec<u8>>,
    /// Whether the last Paste found nothing, said in grey above the
    /// field until something is pasted or typed.
    paste_empty: bool,
    /// Whether Ctrl+V's paste was taken, until its keys are let go.
    paste_held: bool,
    wall: Wall,
    /// What this Device's quick note shows (#142), read as the popup
    /// opens.
    quick: QuickNote,
}

impl CapturePopup {
    /// An empty popup over `store`, speaking `locale`.
    pub fn new(store: Arc<Store>, locale: &str) -> CapturePopup {
        let mut popup = CapturePopup {
            store,
            t: L10n::new(locale),
            text: String::new(),
            focus: true,
            error: None,
            failure_open: false,
            saved: None,
            clipboard: None,
            photos: Vec::new(),
            paste_empty: false,
            paste_held: false,
            wall: Arc::new(local_wall),
            quick: QuickNote::default(),
        };
        popup.read_quick();
        popup
    }

    /// The clipboard Paste reads.
    pub fn set_clipboard(&mut self, clipboard: Box<dyn Clipboard>) {
        self.clipboard = Some(clipboard);
    }

    /// The wall clock the popup's Notes are saved by.
    pub fn set_wall_clock(&mut self, wall: Wall) {
        self.wall = wall;
    }

    /// Calls `saved` after every Note kept.
    pub fn on_saved(&mut self, saved: Box<dyn Fn()>) {
        self.saved = Some(saved);
    }

    /// The window the popup opens in: small, over all others.
    pub fn viewport(&self) -> egui::ViewportBuilder {
        egui::ViewportBuilder::default()
            .with_title(self.t.capture_tooltip())
            .with_app_id(APP_ID)
            .with_inner_size(SIZE)
            .with_resizable(false)
            .with_minimize_button(false)
            .with_maximize_button(false)
            .with_always_on_top()
            .with_active(true)
    }

    /// What is typed and not saved yet.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What went wrong, in full, behind the announcement at the top.
    pub fn error(&self) -> Option<&str> {
        self.error.as_ref().map(|f| f.details.as_str())
    }

    /// The announcement of what went wrong, at the top; its click opens
    /// the failure's dialog.
    fn failure_line(&mut self, ui: &mut Ui) {
        let Some(failure) = &self.error else {
            return;
        };
        let colour = if failure.cause.is_some() {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().text_color()
        };
        let label =
            egui::Label::new(RichText::new(failure.said).color(colour)).sense(egui::Sense::click());
        if ui.add(label).clicked() {
            self.failure_open = true;
        }
    }

    /// The failure's dialog, while it shows: the cause, the fix and the
    /// full text, Close and Report. Closed, the failure is done with.
    fn failure_dialog(&mut self, ctx: &egui::Context) {
        let t = self.t;
        let Some(failure) = &self.error else {
            self.failure_open = false;
            return;
        };
        if !self.failure_open {
            return;
        }
        let mut closed = false;
        let modal = egui::Modal::new(Id::new("popup failure")).show(ctx, |ui| {
            ui.set_width(360.0);
            closed = match failure.cause {
                Some(cause) => failure_form(
                    ui,
                    &t,
                    t.action_failed_title(),
                    (cause, t.action_failed_fix()),
                    &failure.details,
                ),
                None => done_form(ui, &t, failure.said, &failure.details),
            };
        });
        if closed || modal.should_close() {
            self.failure_open = false;
            self.error = None;
        }
    }

    /// Takes the keyboard again, as when the hotkey is pressed once more,
    /// and reads again what the quick note shows.
    pub fn refocus(&mut self) {
        self.focus = true;
        self.read_quick();
    }

    /// Reads what the quick note shows.
    fn read_quick(&mut self) {
        match QuickNote::read(&self.store) {
            Ok(quick) => self.quick = quick,
            Err(e) => self.error = Some(Failure::of(&self.t, &e)),
        }
    }

    /// Leaving: keeps what is typed with a Nudge in 1 h. Stays open when
    /// that failed, so nothing is lost.
    pub fn leave(&mut self) -> Outcome {
        self.save(Notes::default_nudge())
    }

    fn save(&mut self, nudge_in: Duration) -> Outcome {
        self.keep(nudge_in).0
    }

    /// Open in the editor: saves as leaving does, and hands the Note on to
    /// the window's Note dialog; stays open when nothing was saved.
    fn open_in_editor(&mut self) -> Outcome {
        match self.keep(Notes::default_nudge()) {
            (Outcome::Closed, Some(id)) => Outcome::Edit(id),
            (Outcome::Closed, None) => Outcome::Open,
            (outcome, _) => outcome,
        }
    }

    /// Saves as [`CapturePopup::save`] does; with it, the Note kept, if
    /// one was.
    fn keep(&mut self, nudge_in: Duration) -> (Outcome, Option<String>) {
        let held = Held {
            photos: &self.photos,
            recordings: &[],
            files: &[],
        };
        let saved = capture_choices::save(
            (&self.store, &self.wall),
            &self.text,
            None,
            nudge_in,
            held,
            &CaptureChoices::default(),
            self.t,
        );
        match saved {
            // Only once all of it is written: a failed save keeps it here.
            Ok((saved, _)) => {
                let kept = match &saved {
                    Captured::Note(note) | Captured::Kept(note) => Some(note.id.clone()),
                    _ => None,
                };
                self.text.clear();
                self.photos.clear();
                self.paste_empty = false;
                self.error = None;
                if !matches!(saved, Captured::Nothing)
                    && let Some(saved) = &self.saved
                {
                    saved();
                }
                (Outcome::Closed, kept)
            }
            Err(e) => {
                self.error = Some(Failure::of(&self.t, &e));
                (Outcome::Open, None)
            }
        }
    }

    /// Draws the popup into `ui`, the whole of its window, and does what
    /// was asked in it; each button clicked shows its green "!"
    /// (obf-1143).
    pub fn show(&mut self, ui: &mut Ui) -> Outcome {
        let outcome = self.show_content(ui);
        crate::design::tap_marks(ui);
        outcome
    }

    fn show_content(&mut self, ui: &mut Ui) -> Outcome {
        let t = self.t;
        let tones = theme::tones(ui.visuals());
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let id = Id::new("capture popup");
        let typed = note_text(&self.text).is_some() || !self.photos.is_empty();
        // Enter on a focused button is that button's.
        let in_field = ui.memory(|m| m.has_focus(id));
        // The failure's dialog takes Esc for itself.
        let failure_open = self.failure_open && self.error.is_some();
        let (enter, escape, closing) = ui.input_mut(|i| {
            (
                in_field && take_plain_enter(i),
                !failure_open && i.consume_key(Modifiers::NONE, Key::Escape),
                i.viewport().close_requested(),
            )
        });
        if closing {
            let left = self.leave();
            if left == Outcome::Open {
                ui.ctx().send_viewport_cmd(ViewportCommand::CancelClose);
            }
            return left;
        }
        self.failure_dialog(ui.ctx());
        if escape {
            return self.leave();
        }
        if enter && typed {
            return self.save(Notes::default_nudge());
        }
        let quick = self.quick;
        // Ctrl+1 to 3 with the Nudge buttons' times while they show,
        // Ctrl+Enter as Enter (#277).
        if typed
            && !failure_open
            && let Some(nudge_in) = capture_choices::nudge_key(ui, quick.nudge_buttons)
        {
            return self.save(nudge_in);
        }
        let mut picked = None;
        let mut to_editor = false;
        let has_clipboard = self.clipboard.is_some();
        Frame::NONE
            .fill(tones.background)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                // Announcements go to the top (UI laws).
                self.failure_line(ui);
                // Ctrl+V in the empty field takes text and picture alike,
                // before the field takes the text alone.
                let blank = self.text.trim().is_empty();
                if has_clipboard
                    && let Some(text) = paste::ctrl_v(ui, id, blank, &mut self.paste_held)
                {
                    self.paste(ui, id, text);
                }
                if self.paste_empty {
                    paste::empty_line(ui, &t);
                }
                // A picture pasted stays above the text until the Note is
                // saved (#352).
                for _ in &self.photos {
                    ui.horizontal(|ui| {
                        icons::show(ui, icons::PHOTO, 18.0, text);
                        ui.label(RichText::new(t.photo()).color(text));
                    });
                }
                // Enter saves, taken above; Shift+Enter starts a new
                // line (#277), the lines past a few scrolled.
                let field = ScrollArea::vertical()
                    .id_salt("capture popup lines")
                    .max_height(FIELD_HEIGHT)
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::multiline(&mut self.text)
                                .id(id)
                                .hint_text(t.capture_hint())
                                .text_color(text)
                                // At the Device's text size (#326, obf-1019).
                                .font(FontId::proportional(theme::content(ui, 18.0)))
                                .desired_rows(1)
                                .desired_width(f32::INFINITY)
                                .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)),
                        )
                    })
                    .inner;
                // Its right-click menu; Paste as Ctrl+V's (#130).
                match text_menu::menu(ui, &t, &field, &self.text, &[]) {
                    Some(Pick::Paste) if has_clipboard => self.paste(ui, id, None),
                    Some(pick) => text_menu::apply(ui, id, &self.text, pick),
                    None => {}
                }
                name_field(ui, &field, t.capture_field_name());
                if field.changed() {
                    self.paste_empty = false;
                }
                // The keyboard types into the field unless Tab took it on.
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
                if std::mem::take(&mut self.focus) {
                    field.request_focus();
                    ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                }
                ui.add_space(4.0);
                if !quick.nudge_buttons {
                    // Without the Nudge buttons, Save nudges in 1 h (#142).
                    if theme::primary(ui, typed, t.save()).clicked() {
                        picked = Some(Notes::default_nudge());
                    }
                } else {
                    ui.label(RichText::new(t.nudge_heading()).strong());
                    ui.horizontal(|ui| {
                        let labels = [t.snooze10(), t.snooze30(), t.snooze60()];
                        for (label, nudge_in) in
                            labels.into_iter().zip(capture_choices::nudge_times())
                        {
                            let button = egui::Button::new(RichText::new(label).color(text));
                            if ui.add_enabled(typed, button).clicked() {
                                picked = Some(nudge_in);
                            }
                        }
                    });
                }
                ui.add_space(4.0);
                // Saves as Enter does and opens the Note dialog (#352,
                // obf-1080).
                if capture_choices::open_in_editor(ui, &t, typed).clicked() {
                    to_editor = true;
                }
                ui.add_space(4.0);
                let hint = if quick.nudge_buttons {
                    t.popup_nudge_keys_hint()
                } else {
                    t.popup_keys_hint()
                };
                ui.label(RichText::new(hint).small().color(weak));
            });
        if to_editor {
            return self.open_in_editor();
        }
        match picked {
            Some(nudge_in) => self.save(nudge_in),
            None => Outcome::Open,
        }
    }

    /// Pastes into the field `id`: the clipboard's text, or `text` when
    /// Ctrl+V brought it, at its cursor, and its picture held for the
    /// Note; with neither, the grey line says so.
    fn paste(&mut self, ui: &Ui, id: Id, text: Option<String>) {
        let Some(clipboard) = self.clipboard.as_deref_mut() else {
            return;
        };
        let pasted = paste::take(clipboard, text);
        self.paste_empty = pasted.is_empty();
        if let Some(text) = &pasted.text {
            paste::insert(ui, id, &mut self.text, text);
        }
        if let Some(photo) = pasted.photo {
            self.photos.push(photo);
        }
    }
}

/// Enter alone pressed, taken: not Shift+Enter, which starts a new line,
/// nor Ctrl+Enter (#277).
fn take_plain_enter(i: &mut InputState) -> bool {
    let before = i.events.len();
    i.events.retain(|e| {
        !matches!(
            e,
            Event::Key {
                key: Key::Enter,
                pressed: true,
                modifiers,
                ..
            } if modifiers.matches_exact(Modifiers::NONE)
        )
    });
    i.events.len() != before
}
