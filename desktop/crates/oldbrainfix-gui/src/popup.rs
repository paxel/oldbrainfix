//! The capture popup (UI laws, "Capture"): a small window over all
//! others that the hotkey, the tray's Capture and `oldbrainfix capture`
//! open. It never asks anything: Enter keeps the text with a Nudge in
//! 1 h, the three buttons with theirs, and leaving it with Esc or its
//! close button keeps what is typed the same way as Enter, as leaving the
//! phone's capture screen does; empty, it just closes. The Note stays
//! Unsorted.
//!
//! Its microphone button records voice into the same Note: recording
//! starts at once, and one big stop button ends it and keeps the Note
//! with a Nudge in 1 h; the Nudge times, Enter, Esc and closing keep it
//! as well, so a recording is never lost.
//!
//! Closing the popup gives the keyboard back to the window that had it
//! where the system does so for a closed window: on X11, Windows and
//! macOS mostly. On Wayland the compositor decides that, and whether the
//! popup stays on top at all (winit has no window levels there).

use std::sync::Arc;

use chrono::Duration;
use egui::{FontId, Frame, Id, Key, Margin, Modifiers, RichText, TextEdit, Ui, ViewportCommand};
use oldbrainfix_core::{APP_ID, Note, Notes, Store};

use crate::cli::note_text;
use crate::l10n::L10n;
use crate::media::Recorder;
use crate::media_view::{self, Voice};
use crate::theme;

/// What the popup is after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Open,
    /// Done with: it saved what was typed, or nothing was.
    Closed,
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
    Notes::new(store)
        .capture(text, None, Some(nudge_in))
        .map(Some)
}

pub struct CapturePopup {
    store: Arc<Store>,
    t: L10n,
    text: String,
    /// Whether the field is still to take the keyboard.
    focus: bool,
    /// What went wrong saving, shown at the top; the text stays.
    error: Option<String>,
    saved: Option<Box<dyn Fn()>>,
    recorder: Option<Box<dyn Recorder>>,
    /// A recording that ended but could not be kept yet.
    recorded: Option<Vec<u8>>,
}

impl CapturePopup {
    /// An empty popup over `store`, speaking `locale`.
    pub fn new(store: Arc<Store>, locale: &str) -> CapturePopup {
        CapturePopup {
            store,
            t: L10n::new(locale),
            text: String::new(),
            focus: true,
            error: None,
            saved: None,
            recorder: None,
            recorded: None,
        }
    }

    /// The microphone voice Notes are recorded with.
    pub fn set_recorder(&mut self, recorder: Box<dyn Recorder>) {
        self.recorder = Some(recorder);
    }

    /// Whether a voice recording runs.
    pub fn recording(&self) -> bool {
        self.recorder.as_ref().is_some_and(|r| r.recording())
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
            .with_inner_size([460.0, 280.0])
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

    /// What went wrong saving, as announced at the top.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Takes the keyboard again, as when the hotkey is pressed once more.
    pub fn refocus(&mut self) {
        self.focus = true;
    }

    /// Leaving: keeps what is typed with a Nudge in 1 h. Stays open when
    /// that failed, so nothing is lost.
    pub fn leave(&mut self) -> Outcome {
        self.save(Notes::default_nudge())
    }

    fn save(&mut self, nudge_in: Duration) -> Outcome {
        // A running recording is ended and kept first.
        if let Some(recorder) = &mut self.recorder
            && recorder.recording()
            && let Some(bytes) = recorder.stop()
        {
            self.recorded = Some(bytes);
        }
        let saved = match self.recorded.clone() {
            None => capture(&self.store, &self.text, nudge_in),
            Some(recording) => {
                let text = note_text(&self.text).unwrap_or_default();
                Notes::new(&self.store)
                    .capture_with(text, None, Some(nudge_in), &[], &[recording])
                    .map(Some)
            }
        };
        match saved {
            Ok(saved) => {
                self.text.clear();
                self.recorded = None;
                self.error = None;
                if saved.is_some()
                    && let Some(saved) = &self.saved
                {
                    saved();
                }
                Outcome::Closed
            }
            Err(e) => {
                self.error = Some(e.to_string());
                Outcome::Open
            }
        }
    }

    /// Draws the popup into `ui`, the whole of its window, and does what
    /// was asked in it.
    pub fn show(&mut self, ui: &mut Ui) -> Outcome {
        let t = self.t;
        let tones = theme::tones(ui.visuals());
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let id = Id::new("capture popup");
        let recording = self
            .recorder
            .as_ref()
            .filter(|r| r.recording())
            .map(|r| r.elapsed());
        let typed =
            note_text(&self.text).is_some() || recording.is_some() || self.recorded.is_some();
        // Enter on a focused button is that button's.
        let in_field = ui.memory(|m| m.has_focus(id));
        let (enter, escape, closing) = ui.input_mut(|i| {
            (
                in_field && i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
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
        if escape {
            return self.leave();
        }
        if enter && typed {
            return self.save(Notes::default_nudge());
        }
        let mut picked = None;
        let mut record = false;
        let has_recorder = self.recorder.is_some();
        Frame::NONE
            .fill(tones.background)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                if let Some(error) = &self.error {
                    // Announcements go to the top (UI laws).
                    ui.label(RichText::new(error).color(ui.visuals().error_fg_color));
                }
                let field = ui.add(
                    TextEdit::singleline(&mut self.text)
                        .id(id)
                        .hint_text(t.capture_hint())
                        .text_color(text)
                        .font(FontId::proportional(18.0))
                        .desired_width(f32::INFINITY),
                );
                // The keyboard types into the field unless Tab took it on.
                if ui.memory(|m| m.focused().is_none()) {
                    field.request_focus();
                }
                if std::mem::take(&mut self.focus) {
                    field.request_focus();
                    ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                }
                if has_recorder {
                    match media_view::voice_button(ui, &t, recording) {
                        Some(Voice::Record) => record = true,
                        Some(Voice::StopAndSave) => picked = Some(Notes::default_nudge()),
                        None => {}
                    }
                }
                ui.add_space(8.0);
                ui.label(RichText::new(t.nudge_heading()).strong().color(text));
                ui.horizontal(|ui| {
                    let choices = [
                        (t.snooze10(), Duration::minutes(10)),
                        (t.snooze30(), Duration::minutes(30)),
                        (t.snooze60(), Notes::default_nudge()),
                    ];
                    for (label, nudge_in) in choices {
                        let button = egui::Button::new(RichText::new(label).color(text));
                        if ui.add_enabled(typed, button).clicked() {
                            picked = Some(nudge_in);
                        }
                    }
                });
                ui.add_space(4.0);
                ui.label(RichText::new(t.popup_keys_hint()).small().color(weak));
            });
        if record && let Some(recorder) = &mut self.recorder {
            if recorder.start() {
                self.error = None;
            } else {
                self.error = Some(t.microphone_missing().to_string());
            }
        }
        match picked {
            Some(nudge_in) => self.save(nudge_in),
            None => Outcome::Open,
        }
    }
}
