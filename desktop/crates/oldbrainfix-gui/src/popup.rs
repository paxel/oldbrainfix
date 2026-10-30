//! The capture popup (UI laws, "Capture"): a small window over all
//! others that the hotkey, the tray's Capture and `oldbrainfix capture`
//! open. It never asks anything: Enter keeps the text with a Nudge in
//! 1 h, the three buttons with theirs, and leaving it with Esc or its
//! close button keeps what is typed the same way as Enter, as leaving the
//! phone's capture screen does; empty, it just closes.
//!
//! Above the Nudge buttons sit the Note card's buttons (#113): "File
//! into…", Routine, Reminder, Frog and Session where this Device may
//! spawn, and "More" with the Importance, "Make it a Tally" and "Add to a
//! Shopping List", whose lines then go into the list picked instead of a
//! Note, the rest waiting unshown (#124). Routine,
//! Reminder, Session and the Tally open their page in the popup, grown
//! for it, which hands its choice back; every choice shows at the top,
//! not saved yet, with a button that drops it. All of it is saved with
//! the Note in one go when the popup is left or saved, nothing of it for
//! an empty Note. A Note filed or with something set up gets one Save
//! instead of the Nudge buttons and no Nudge. Esc on a page goes back to
//! the capture without its choice, as do Alt+Left and the mouse's back
//! button.
//!
//! Its microphone button records voice into the same Note: recording
//! starts at once, and one big stop button ends it and leaves it on the
//! popup, with the buttons and room to type; it is saved with the rest.
//! The Nudge times, Save, Enter, Esc and closing end a recording still
//! running and keep it as well, so a recording is never lost.
//!
//! Closing the popup gives the keyboard back to the window that had it
//! where the system does so for a closed window: on X11, Windows and
//! macOS mostly. On Wayland the compositor decides that, and whether the
//! popup stays on top at all (winit has no window levels there), and
//! whether it grows for a page.

use std::sync::Arc;

use chrono::Duration;
use egui::{
    FontId, Frame, Id, Key, Margin, Modifiers, PointerButton, RichText, TextEdit, Ui,
    ViewportCommand,
};
use oldbrainfix_core::{APP_ID, Area, DeviceSettings, Note, Notes, Places, Store, Topic};

use crate::app::{Wall, failure_form, local_wall, region_of};
use crate::capture_choices::{self, Ask, CaptureChoices, Captured, ListTitle, Opens};
use crate::cli::note_text;
use crate::enrich::Enricher;
use crate::icons;
use crate::l10n::L10n;
use crate::media::Recorder;
use crate::media_view::{self, Voice};
use crate::routine_editor::{Intent, RoutineEditor};
use crate::session_editor::{SessionDraft, SessionIntent, WEEKDAYS_OPEN};
use crate::speech::SpeechModel;
use crate::tally_view::{FieldForm, FormIntent, TallyForm};
use crate::theme;

/// The popup's size, and its size while a page shows.
const SIZE: [f32; 2] = [480.0, 340.0];
const PAGE_SIZE: [f32; 2] = [560.0, 680.0];

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

/// A page a button opened over the capture, handing its choice back.
enum Page {
    Schedule(Box<RoutineEditor>),
    Tally(Box<TallyForm>),
}

/// What was asked of the page shown.
enum PageAsk {
    Schedule(Intent),
    Tally(FormIntent),
}

/// A failure announced at the top of the popup: what it says, why, and
/// the full text.
struct Failure {
    said: &'static str,
    cause: &'static str,
    details: String,
}

impl Failure {
    /// What went wrong as `error` says, said without its text.
    fn of(t: &L10n, error: &oldbrainfix_core::Error) -> Failure {
        Failure {
            said: t.action_failed(),
            cause: t.action_failed_cause(),
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
    recorder: Option<Box<dyn Recorder>>,
    /// The recordings that ended, held until the Note is saved.
    recorded: Vec<Vec<u8>>,
    /// What was set up for the Note, not written yet.
    choices: CaptureChoices,
    /// The Areas with their Topics "File into…" lists, read as the popup
    /// opens.
    areas: Vec<(Area, Vec<Topic>)>,
    /// Whether this Device may spawn from the Note where it goes.
    spawns: bool,
    /// The Shopping Lists the lines can go into, by their Note's id with
    /// their title, read with the Areas.
    lists: Vec<ListTitle>,
    /// The thread making Transcripts, and the speech model it needs; a
    /// recording goes into a Shopping List only with both.
    transcriber: Option<(Enricher, Option<SpeechModel>)>,
    wall: Wall,
    page: Option<Page>,
    /// The region the system reports, for the units of a Tally.
    region: Option<String>,
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
            recorder: None,
            recorded: Vec::new(),
            choices: CaptureChoices::default(),
            areas: Vec::new(),
            spawns: true,
            lists: Vec::new(),
            transcriber: None,
            wall: Arc::new(local_wall),
            page: None,
            region: region_of(sys_locale::get_locale().as_deref()),
        };
        popup.read_places();
        popup
    }

    /// The microphone voice Notes are recorded with.
    pub fn set_recorder(&mut self, recorder: Box<dyn Recorder>) {
        self.recorder = Some(recorder);
    }

    /// The thread making Transcripts and the speech model it needs, for
    /// a voice capture going into a Shopping List; without a model it
    /// waits for none.
    pub fn set_transcriber(&mut self, transcriber: Enricher, speech: Option<SpeechModel>) {
        self.transcriber = Some((transcriber, speech));
    }

    /// The wall clock the days of a Shopping List's Lines are on.
    pub fn set_wall_clock(&mut self, wall: Wall) {
        self.wall = wall;
    }

    /// Whether a recording captured now gets its Transcript here.
    fn transcribes(&self) -> bool {
        self.transcriber
            .as_ref()
            .is_some_and(|(_, speech)| speech.as_ref().is_none_or(|s| s.current().is_some()))
    }

    /// The Shopping Lists the lines can go into: none while a recording
    /// gets no Transcript here.
    fn offered_lists(&self) -> &[ListTitle] {
        let voice = self.recording() || !self.recorded.is_empty();
        if voice && !self.transcribes() {
            return &[];
        }
        &self.lists
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

    /// What was set up for the Note and not saved yet.
    pub fn choices(&self) -> &CaptureChoices {
        &self.choices
    }

    /// Whether a page opened from the capture shows.
    pub fn page_open(&self) -> bool {
        self.page.is_some()
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
        let colour = ui.visuals().error_fg_color;
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
            closed = failure_form(
                ui,
                &t,
                t.action_failed_title(),
                (failure.cause, t.action_failed_fix()),
                &failure.details,
            );
        });
        if closed || modal.should_close() {
            self.failure_open = false;
            self.error = None;
        }
    }

    /// Takes the keyboard again, as when the hotkey is pressed once more,
    /// and reads the Areas and Topics again.
    pub fn refocus(&mut self) {
        self.focus = true;
        self.read_places();
    }

    /// Reads the Areas with their Topics, and whether the Note may spawn
    /// where it goes.
    fn read_places(&mut self) {
        let places = Places::new(&self.store);
        let read = places.areas().and_then(|areas| {
            areas
                .into_iter()
                .map(|area| {
                    let topics = places.topics(&area.id)?;
                    Ok((area, topics))
                })
                .collect::<oldbrainfix_core::Result<Vec<_>>>()
        });
        match read {
            Ok(areas) => self.areas = areas,
            Err(e) => self.error = Some(Failure::of(&self.t, &e)),
        }
        match capture_choices::shopping_lists(&self.store, &self.t) {
            Ok(lists) => self.lists = lists,
            Err(e) => self.error = Some(Failure::of(&self.t, &e)),
        }
        // An Area or Topic deleted meanwhile is no place to go.
        if let Some(place) = &self.choices.place
            && capture_choices::place_name(&self.t, &self.areas, place).is_none()
        {
            self.choices.place = None;
        }
        self.read_spawns();
    }

    fn read_spawns(&mut self) {
        match capture_choices::may_spawn(&self.store, self.choices.place.as_deref()) {
            Ok(spawns) => self.spawns = spawns,
            Err(e) => self.error = Some(Failure::of(&self.t, &e)),
        }
    }

    /// Leaving: keeps what is typed with a Nudge in 1 h, and all that was
    /// set up for it; a page open goes without its choice. Stays open
    /// when that failed, so nothing is lost.
    pub fn leave(&mut self) -> Outcome {
        self.save(Notes::default_nudge())
    }

    fn save(&mut self, nudge_in: Duration) -> Outcome {
        // A running recording is ended and kept first.
        if let Some(recorder) = &mut self.recorder
            && recorder.recording()
            && let Some(bytes) = recorder.stop()
        {
            self.recorded.push(bytes);
        }
        // Without a Transcript to come, a recording keeps it a Note, as
        // said once it is saved.
        let picked = self.choices.shopping.clone();
        let untranscribed = picked.is_some() && !self.recorded.is_empty() && !self.transcribes();
        let mut choices = self.choices.clone();
        if untranscribed {
            choices.shopping = None;
        }
        let list = choices.shopping.clone();
        let saved = capture_choices::capture(
            &self.store,
            &self.text,
            self.choices.place.as_deref(),
            nudge_in,
            (&[], &self.recorded),
            (&choices, &*self.wall),
        );
        match saved {
            // Only once all of it is written: a failed save keeps it here.
            Ok(saved) => {
                self.text.clear();
                self.recorded.clear();
                self.error = None;
                self.choices = CaptureChoices::default();
                self.page = None;
                if let (Captured::Waiting(note), Some((transcriber, _)), Some(list)) =
                    (&saved, &self.transcriber, &list)
                {
                    let held = (self.store.clone(), self.wall.clone());
                    capture_choices::pour_later(transcriber, held, self.t, &note.id, list);
                }
                if !matches!(saved, Captured::Nothing)
                    && let Some(saved) = &self.saved
                {
                    saved();
                }
                // Kept as a Note instead: said here, as the popup is all
                // there is to say it.
                let t = self.t;
                let list = picked.as_deref().unwrap_or("");
                let said = match saved {
                    Captured::Kept(_) => Some((
                        t.capture_shopping_gone(),
                        format!("shopping list {list}: it can no longer be added to"),
                    )),
                    Captured::Note(_) if untranscribed => Some((
                        t.capture_shopping_untranscribed(),
                        format!("shopping list {list}: no speech model to transcribe with"),
                    )),
                    _ => None,
                };
                if let Some((said, details)) = said {
                    self.error = Some(Failure {
                        said,
                        cause: said,
                        details,
                    });
                    self.read_places();
                    return Outcome::Open;
                }
                Outcome::Closed
            }
            Err(e) => {
                self.error = Some(Failure::of(&self.t, &e));
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
            note_text(&self.text).is_some() || recording.is_some() || !self.recorded.is_empty();
        // Enter on a focused button is that button's.
        let in_field = ui.memory(|m| m.has_focus(id));
        let typing = ui.ctx().text_edit_focused();
        let paged = self.page.is_some();
        // The failure's dialog takes Esc for itself.
        let failure_open = self.failure_open && self.error.is_some();
        let (enter, escape, closing, back) = ui.input_mut(|i| {
            (
                in_field && i.consume_key(Modifiers::NONE, Key::Enter),
                !failure_open && i.consume_key(Modifiers::NONE, Key::Escape),
                i.viewport().close_requested(),
                // A page goes back as a level does (UI laws, "Desktop").
                paged
                    && (i.pointer.button_pressed(PointerButton::Extra1)
                        || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowLeft))),
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
        if paged {
            if escape || back {
                self.close_page(ui.ctx());
            } else {
                self.show_page(ui);
            }
            return Outcome::Open;
        }
        if escape {
            return self.leave();
        }
        if enter && typed {
            return self.save(Notes::default_nudge());
        }
        let mut picked = None;
        let mut record = false;
        let mut stop = false;
        let mut asked = None;
        let held = self.recorded.len();
        let has_recorder = self.recorder.is_some();
        let sorted = self.choices.sorted(self.choices.place.as_deref());
        let place_chip = self.choices.place.as_ref().and_then(|place| {
            capture_choices::place_name(&t, &self.areas, place).map(|name| (name, true))
        });
        let lists = self.offered_lists().to_vec();
        Frame::NONE
            .fill(tones.background)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                // Announcements go to the top (UI laws).
                self.failure_line(ui);
                let shown = (place_chip, self.lists.as_slice());
                if let Some(chip) = capture_choices::chips(ui, &t, &self.choices, shown) {
                    asked = Some(chip);
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
                for _ in 0..held {
                    ui.horizontal(|ui| {
                        icons::show(ui, icons::MIC, 18.0, text);
                        ui.label(RichText::new(t.voice_recording()).color(text));
                    });
                }
                if has_recorder {
                    match media_view::voice_button(ui, &t, recording) {
                        Some(Voice::Record) => record = true,
                        Some(Voice::Stop) => stop = true,
                        None => {}
                    }
                }
                ui.add_space(4.0);
                let offer = (self.areas.as_slice(), self.spawns, lists.as_slice());
                if let Some(button) = capture_choices::action_row(ui, &t, &self.choices, offer) {
                    asked = Some(button);
                }
                ui.add_space(4.0);
                if sorted {
                    // A sorted Note needs no Nudge (#113).
                    let save = egui::Button::new(RichText::new(t.save()).color(text));
                    if ui.add_enabled(typed, save).clicked() {
                        picked = Some(Notes::default_nudge());
                    }
                } else {
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
                }
                ui.add_space(4.0);
                let hint = if sorted {
                    t.popup_keys_hint_sorted()
                } else {
                    t.popup_keys_hint()
                };
                ui.label(RichText::new(hint).small().color(weak));
            });
        // Stopped, the recording stays here with the rest (UI laws).
        if stop
            && let Some(recorder) = &mut self.recorder
            && let Some(bytes) = recorder.stop()
        {
            self.recorded.push(bytes);
            self.focus = true;
        }
        if record && let Some(recorder) = &mut self.recorder {
            if recorder.start() {
                self.error = None;
            } else {
                self.error = Some(Failure {
                    said: t.microphone_missing(),
                    cause: t.microphone_missing(),
                    details: "record: no microphone".into(),
                });
            }
        }
        if let Some(asked) = asked {
            self.ask(ui.ctx(), asked);
        }
        match picked {
            Some(nudge_in) => self.save(nudge_in),
            None => Outcome::Open,
        }
    }

    /// Takes what was asked of the buttons or the choices; a page it
    /// opens shows in the popup, grown for it.
    fn ask(&mut self, ctx: &egui::Context, asked: Ask) {
        let opened = self
            .choices
            .take(asked, &self.store)
            .and_then(|opens| opens.map(|opens| self.page_for(opens)).transpose());
        match opened {
            Ok(Some(page)) => {
                self.page = Some(page);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(PAGE_SIZE.into()));
            }
            Ok(None) => {}
            Err(e) => self.error = Some(Failure::of(&self.t, &e)),
        }
        self.read_spawns();
    }

    /// The page for a new Routine, Reminder or Session, or the Tally's
    /// first field, of the Note being captured.
    fn page_for(&self, opens: Opens) -> oldbrainfix_core::Result<Page> {
        let settings = DeviceSettings::new(&self.store);
        match opens {
            Opens::Schedule { reminder, session } => {
                let renag = settings.renag_interval()?;
                let now = self.store.now();
                let mut editor =
                    RoutineEditor::choose(&self.text, reminder, now, &local_wall, renag);
                if session {
                    // The Tallies of where it goes; while Unsorted, any.
                    let entity = self.choices.place.as_deref().unwrap_or("");
                    let draft = SessionDraft::of(&self.store, &self.t, entity, None)?;
                    editor = editor.with_session(draft, None);
                }
                Ok(Page::Schedule(Box::new(editor)))
            }
            Opens::Tally => {
                let system = settings.unit_system(self.region.as_deref())?;
                let form = FieldForm::choose(system, &self.t);
                Ok(Page::Tally(Box::new(TallyForm::Field(form))))
            }
        }
    }

    /// Back to the capture, its size and the field.
    fn close_page(&mut self, ctx: &egui::Context) {
        self.page = None;
        self.focus = true;
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(SIZE.into()));
    }

    /// Draws the page and takes the choice it hands back.
    fn show_page(&mut self, ui: &mut Ui) {
        let t = self.t;
        let tones = theme::tones(ui.visuals());
        let mut asked = None;
        Frame::NONE
            .fill(tones.background)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.failure_line(ui);
                asked = match &mut self.page {
                    Some(Page::Schedule(editor)) => {
                        editor.show(ui, &t, tones).map(PageAsk::Schedule)
                    }
                    Some(Page::Tally(form)) => form.show(ui, &t, tones).map(PageAsk::Tally),
                    None => None,
                };
            });
        let Some(asked) = asked else {
            return;
        };
        match self.page_asked(asked) {
            Ok(true) => self.close_page(ui.ctx()),
            Ok(false) => {}
            Err(e) => self.error = Some(Failure::of(&self.t, &e)),
        }
    }

    /// Does what was asked of the page; whether it is done with.
    fn page_asked(&mut self, asked: PageAsk) -> oldbrainfix_core::Result<bool> {
        match (asked, &mut self.page) {
            (PageAsk::Schedule(Intent::Save), Some(Page::Schedule(editor))) => {
                let Some(choice) = editor.choice(&self.store)? else {
                    return Ok(false);
                };
                self.choices.schedules.push(choice);
                Ok(true)
            }
            (PageAsk::Schedule(Intent::Session(SessionIntent::WeekdaysOpen(open))), _) => {
                let value = if open { "true" } else { "false" };
                self.store.set_local_setting(WEEKDAYS_OPEN, value)?;
                Ok(false)
            }
            (PageAsk::Tally(FormIntent::Save), Some(Page::Tally(form))) => {
                let TallyForm::Field(form) = form.as_mut() else {
                    return Ok(false);
                };
                let Some(field) = form.choice() else {
                    return Ok(false);
                };
                self.choices.tally = Some(field);
                Ok(true)
            }
            (PageAsk::Schedule(Intent::Close) | PageAsk::Tally(FormIntent::Close), _) => Ok(true),
            // A page of a Note not written yet deletes, hides and links
            // to nothing.
            _ => Ok(false),
        }
    }
}
