//! The window (UI laws, "Desktop"): Areas and Topics on the left, the
//! capture field and the Notes of the selection in the middle as embossed
//! cards, the selected Note's editor on the right.
//!
//! Every change goes through `oldbrainfix_core`, which writes the entries
//! the phone's core writes. A card dragged onto an Area or Topic on the
//! left is filed there. Keys replace swipes (UI laws, "Navigation"):
//! Alt+Left or the mouse's back button goes back one level, a Topic to
//! its Area and an Area, Unsorted or the trash to All Notes; Alt+Right or
//! the mouse's forward button goes forward into the last selected Area,
//! and from an Area into its last selected Topic. The keys rest while
//! text is being typed, where Alt+arrows move the cursor.

use chrono::{DateTime, Duration, Local, Utc};
use egui::{
    Color32, FontId, Frame, Id, Key, Margin, Modifiers, PointerButton, RichText, Sense, TextEdit,
    Ui, WidgetInfo, WidgetType,
};
use oldbrainfix_core::{
    AREA_ICONS, Area, Importance, Note, Notes, NotesGo, Places, Section, Store, Topic,
};

use crate::background;
use crate::icons;
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::theme::{self, AREA_PALETTE, Tones};

/// What the middle pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Every Note neither in the trash nor merged.
    All,
    Unsorted,
    /// The Area or Topic with this id; an Area holds its Topics' Notes.
    Place(String),
    Trash,
}

/// What a card carries while it is dragged: the Note's id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraggedNote(pub String);

/// A Note with the Area and Topic it is filed in.
#[derive(Debug, Clone)]
struct Placed {
    note: Note,
    area: Option<Area>,
    topic: Option<Topic>,
}

/// What the panes show, read once per change.
#[derive(Debug, Clone, Default)]
struct View {
    areas: Vec<(Area, Vec<Topic>)>,
    notes: Vec<Placed>,
    /// The selected Note with its Sections.
    detail: Option<(Placed, Vec<Section>)>,
}

impl View {
    fn area(&self, id: &str) -> Option<&Area> {
        self.areas.iter().map(|(a, _)| a).find(|a| a.id == id)
    }

    fn topic(&self, id: &str) -> Option<&Topic> {
        self.areas
            .iter()
            .flat_map(|(_, topics)| topics)
            .find(|t| t.id == id)
    }
}

/// The last selected Area and Topic, where forward leads, as the phone's
/// `BrowserTrail` keeps them.
#[derive(Debug, Clone, Default)]
struct Trail {
    area: Option<String>,
    topic: Option<String>,
}

/// A dialog over the window.
#[derive(Debug, Clone, PartialEq)]
enum Dialog {
    /// Naming a new Area, or a new Topic of the Area `area`.
    NewPlace {
        area: Option<String>,
        name: String,
    },
    /// Renaming the Area or Topic `id`, shown as `shown`.
    Rename {
        id: String,
        name: String,
        shown: String,
    },
    /// The colour and icon of an Area.
    Look {
        area: String,
    },
    Delete(Deleting),
    /// Picking the Note to merge the selected one into, by id and text.
    Merge {
        query: String,
        others: Vec<(String, String)>,
    },
}

/// The question before an Area or Topic is deleted.
#[derive(Debug, Clone, PartialEq)]
struct Deleting {
    id: String,
    question: String,
    lines: Vec<String>,
    /// The shown name of the Area a deleted Topic's Notes can go into;
    /// `None` for an Area.
    area: Option<String>,
    has_notes: bool,
}

/// What the user asked for in a frame, done after it is drawn.
#[derive(Debug, Clone, PartialEq)]
enum Action {
    Select(Selection),
    SelectNote(String),
    /// Captures the capture field's text, nudged this long from now.
    Capture(Duration),
    /// Keeps the selected Note's edited text.
    SaveText,
    Amend,
    Importance(Importance),
    File {
        note: String,
        place: String,
    },
    Trash,
    Restore,
    /// Merges the selected Note into this one.
    Merge(String),
    NewArea,
    NewTopic(String),
    Rename(String),
    Look(String),
    Delete(String),
    StartMerge,
    CloseDialog,
    AddPlace,
    SaveName,
    Colour(String, Option<String>),
    Icon(String, Option<String>),
    DeletePlace(NotesGo),
    Back,
    Forward,
}

pub struct App {
    store: Store,
    t: L10n,
    selection: Selection,
    selected_note: Option<String>,
    view: View,
    /// What went wrong reading or writing the log, shown at the top.
    error: Option<String>,
    /// How far the Notes list is scrolled, for the parallax.
    scrolled: f32,
    trail: Trail,
    /// The capture field's text.
    capture: String,
    /// The selected Note's text as it is being edited, and that Note.
    text: String,
    text_of: Option<String>,
    /// The text of the Section being added.
    amend: String,
    dialog: Option<Dialog>,
    /// Whether the dialog's text field is to take the keyboard, once, as
    /// it opens.
    focus: bool,
}

impl App {
    /// The window over `store`, speaking `locale`, showing all Notes.
    pub fn new(store: Store, locale: &str) -> App {
        let mut app = App {
            store,
            t: L10n::new(locale),
            selection: Selection::All,
            selected_note: None,
            view: View::default(),
            error: None,
            scrolled: 0.0,
            trail: Trail::default(),
            capture: String::new(),
            text: String::new(),
            text_of: None,
            amend: String::new(),
            dialog: None,
            focus: false,
        };
        app.refresh();
        app
    }

    pub fn t(&self) -> &L10n {
        &self.t
    }

    /// The log, for looking at what was written.
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn selected_note(&self) -> Option<&str> {
        self.selected_note.as_deref()
    }

    /// What went wrong last, as announced at the top.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Shows `selection` in the middle pane.
    pub fn select(&mut self, selection: Selection) {
        self.apply(Action::Select(selection));
    }

    /// Shows the Note `id` on the right.
    pub fn select_note(&mut self, id: &str) {
        self.apply(Action::SelectNote(id.to_string()));
    }

    /// Keeps what is typed but not saved yet, as the window closes: the
    /// capture field's text as a new Note nudged in 1 h, as leaving the
    /// phone's capture screen does, and the selected Note's edited text.
    pub fn flush(&mut self) {
        if !self.capture.trim().is_empty() {
            self.apply(Action::Capture(Notes::default_nudge()));
        }
        self.apply(Action::SaveText);
    }

    /// Installs the look and the fonts. Cheap enough to call every frame.
    pub fn install(ctx: &egui::Context) {
        theme::install(ctx);
        let bound = ctx.fonts(|f| {
            f.families()
                .contains(&egui::FontFamily::Name(icons::FAMILY.into()))
        });
        if !bound {
            ctx.set_fonts(icons::fonts());
        }
    }

    /// Reads what the panes show from the log. A selected place that is
    /// gone falls back to All Notes.
    fn refresh(&mut self) {
        let mut read = self.read();
        if let (Ok(view), Selection::Place(id)) = (&read, &self.selection)
            && view.area(id).is_none()
            && view.topic(id).is_none()
        {
            self.selection = Selection::All;
            read = self.read();
        }
        match read {
            Ok(view) => {
                self.view = view;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        let shown = self.view.detail.as_ref().map(|(p, _)| &p.note);
        if shown.map(|n| &n.id) != self.text_of.as_ref() {
            self.text = shown.map(|n| n.text.clone()).unwrap_or_default();
            self.text_of = shown.map(|n| n.id.clone());
            self.amend.clear();
        }
    }

    fn read(&self) -> oldbrainfix_core::Result<View> {
        let notes = Notes::new(&self.store);
        let places = Places::new(&self.store);
        let mut areas = Vec::new();
        for area in places.areas()? {
            let topics = places.topics(&area.id)?;
            areas.push((area, topics));
        }
        let shown = match &self.selection {
            Selection::All => notes.all()?,
            Selection::Unsorted => notes.unsorted()?,
            Selection::Place(id) => notes.inside(id)?,
            Selection::Trash => notes.trashed()?,
        };
        let place = |note: Note| -> oldbrainfix_core::Result<Placed> {
            let Some(id) = note.place.clone() else {
                return Ok(Placed {
                    note,
                    area: None,
                    topic: None,
                });
            };
            let topic = places.topic_by_id(&id)?;
            let area_id = topic.as_ref().map_or(id, |t| t.area_id.clone());
            Ok(Placed {
                note,
                area: places.area_by_id(&area_id)?,
                topic,
            })
        };
        let detail = match &self.selected_note {
            Some(id) => match notes.by_id(id)? {
                Some(note) if !note.is_merged() => Some((place(note)?, notes.sections(id)?)),
                _ => None,
            },
            None => None,
        };
        let mut placed = Vec::new();
        for note in shown {
            placed.push(place(note)?);
        }
        Ok(View {
            areas,
            notes: placed,
            detail,
        })
    }

    /// Does `action`, reads the log again and announces what went wrong.
    fn apply(&mut self, action: Action) {
        let done = self.act(action);
        self.refresh();
        if let Err(e) = done {
            self.error = Some(e.to_string());
        }
    }

    fn act(&mut self, action: Action) -> oldbrainfix_core::Result<()> {
        if matches!(
            action,
            Action::NewArea | Action::NewTopic(_) | Action::Rename(_) | Action::StartMerge
        ) {
            self.focus = true;
        }
        let notes = Notes::new(&self.store);
        let places = Places::new(&self.store);
        let selected = self.selected_note.clone().unwrap_or_default();
        match action {
            Action::Select(selection) => {
                self.save_text()?;
                if let Selection::Place(id) = &selection {
                    if let Some(topic) = self.view.topic(id) {
                        self.trail.area = Some(topic.area_id.clone());
                        self.trail.topic = Some(topic.id.clone());
                    } else {
                        self.trail.area = Some(id.clone());
                    }
                }
                self.selection = selection;
            }
            Action::SelectNote(id) => {
                self.save_text()?;
                self.selected_note = Some(id);
            }
            Action::Capture(nudge_in) => {
                let text = self.capture.trim();
                if text.is_empty() {
                    return Ok(());
                }
                // Captured inside an Area or Topic, the Note is filed
                // there, as the phone's "Note here".
                let place = match &self.selection {
                    Selection::Place(id) => Some(id.as_str()),
                    _ => None,
                };
                let note = notes.capture(text, place, Some(nudge_in))?;
                self.capture.clear();
                self.save_text()?;
                self.selected_note = Some(note.id);
            }
            Action::SaveText => self.save_text()?,
            Action::Amend => {
                notes.amend(&selected, &self.amend)?;
                self.amend.clear();
            }
            Action::Importance(importance) => notes.set_importance(&selected, importance)?,
            Action::File { note, place } => notes.file(&note, &place)?,
            Action::Trash => {
                self.save_text()?;
                Notes::new(&self.store).trash(&selected)?;
            }
            Action::Restore => notes.restore(&selected)?,
            Action::Merge(into) => {
                self.save_text()?;
                Notes::new(&self.store).merge(&selected, &into)?;
                self.dialog = None;
                self.selected_note = Some(into);
            }
            Action::NewArea => {
                self.dialog = Some(Dialog::NewPlace {
                    area: None,
                    name: String::new(),
                });
            }
            Action::NewTopic(area) => {
                self.dialog = Some(Dialog::NewPlace {
                    area: Some(area),
                    name: String::new(),
                });
            }
            Action::Rename(id) => {
                let shown = self.shown_name(&id);
                self.dialog = Some(Dialog::Rename {
                    id,
                    name: shown.clone(),
                    shown,
                });
            }
            Action::Look(area) => self.dialog = Some(Dialog::Look { area }),
            Action::Delete(id) => self.dialog = Some(Dialog::Delete(self.deleting(&id)?)),
            Action::StartMerge => {
                let others = notes
                    .all()?
                    .into_iter()
                    .filter(|n| n.id != selected)
                    .map(|n| (n.id, n.text))
                    .collect();
                self.dialog = Some(Dialog::Merge {
                    query: String::new(),
                    others,
                });
            }
            Action::CloseDialog => self.dialog = None,
            Action::AddPlace => {
                if let Some(Dialog::NewPlace { area, name }) = &self.dialog {
                    match area {
                        Some(area) => drop(places.add_topic(area, name)?),
                        None => drop(places.add_area(name)?),
                    }
                    self.dialog = None;
                }
            }
            Action::SaveName => {
                if let Some(Dialog::Rename { id, name, shown }) = &self.dialog {
                    // An unchanged starter name stays translatable.
                    if name.trim() != shown {
                        places.rename(id, name)?;
                    }
                    self.dialog = None;
                }
            }
            Action::Colour(area, colour) => places.set_colour(&area, colour.as_deref())?,
            Action::Icon(area, icon) => places.set_icon(&area, icon.as_deref())?,
            Action::DeletePlace(go) => {
                if let Some(Dialog::Delete(deleting)) = &self.dialog {
                    if deleting.area.is_none() {
                        places.delete_area(&deleting.id, go)?;
                    } else {
                        places.delete_topic(&deleting.id, go)?;
                    }
                    self.dialog = None;
                }
            }
            Action::Back => {
                if let Some(back) = self.back() {
                    return self.act(Action::Select(back));
                }
            }
            Action::Forward => {
                if let Some(forward) = self.forward() {
                    return self.act(Action::Select(forward));
                }
            }
        }
        Ok(())
    }

    /// Keeps the edited text of the shown Note when it changed; an
    /// emptied text goes back to what it was.
    fn save_text(&mut self) -> oldbrainfix_core::Result<()> {
        let Some((placed, _)) = &self.view.detail else {
            return Ok(());
        };
        let note = &placed.note;
        if self.text_of.as_deref() != Some(note.id.as_str()) || note.is_trashed() {
            return Ok(());
        }
        let text = self.text.trim();
        if text.is_empty() {
            self.text = note.text.clone();
        } else if text != note.text {
            Notes::new(&self.store).edit(&note.id, text)?;
            self.text = text.to_string();
        }
        Ok(())
    }

    /// One level up: a Topic to its Area; an Area, Unsorted or the trash
    /// to All Notes.
    fn back(&self) -> Option<Selection> {
        match &self.selection {
            Selection::All => None,
            Selection::Place(id) => Some(match self.view.topic(id) {
                Some(topic) => Selection::Place(topic.area_id.clone()),
                None => Selection::All,
            }),
            Selection::Unsorted | Selection::Trash => Some(Selection::All),
        }
    }

    /// Into the last selected Area, or from an Area into its last
    /// selected Topic.
    fn forward(&self) -> Option<Selection> {
        match &self.selection {
            Selection::Place(id) if self.view.area(id).is_some() => self
                .trail
                .topic
                .as_ref()
                .filter(|t| self.view.topic(t).is_some_and(|t| t.area_id == *id))
                .map(|t| Selection::Place(t.clone())),
            Selection::Place(_) => None,
            _ => self
                .trail
                .area
                .as_ref()
                .filter(|a| self.view.area(a).is_some())
                .map(|a| Selection::Place(a.clone())),
        }
    }

    /// The name the Area or Topic `id` shows under.
    fn shown_name(&self, id: &str) -> String {
        if let Some(area) = self.view.area(id) {
            return area_name(&self.t, area);
        }
        self.view
            .topic(id)
            .map(|t| topic_name(&self.t, t))
            .unwrap_or_default()
    }

    /// The question before the Area or Topic `id` goes, as the phone asks
    /// it.
    fn deleting(&self, id: &str) -> oldbrainfix_core::Result<Deleting> {
        let t = &self.t;
        let places = Places::new(&self.store);
        let count = places.notes_inside(id)?;
        let mut lines = Vec::new();
        let (question, area) = match self.view.topic(id) {
            Some(topic) => (
                t.delete_topic_question(&topic_name(t, topic)),
                Some(self.shown_name(&topic.area_id)),
            ),
            None => {
                if !places.topics(id)?.is_empty() {
                    lines.push(t.topics_go_too().to_string());
                }
                (t.delete_area_question(&self.shown_name(id)), None)
            }
        };
        if count > 0 {
            lines.push(t.notes_inside(count as i64));
            lines.push(t.where_notes_go().to_string());
        }
        Ok(Deleting {
            id: id.to_string(),
            question,
            lines,
            area,
            has_notes: count > 0,
        })
    }

    /// Draws one frame into `ui` and does what was asked in it.
    pub fn show(&mut self, ui: &mut Ui) {
        let tones = theme::tones(ui.visuals());
        let mut actions = Vec::new();
        if self.dialog.is_none() {
            let typing = ui.ctx().text_edit_focused();
            ui.input_mut(|i| {
                let back = i.pointer.button_pressed(PointerButton::Extra1)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowLeft));
                let forward = i.pointer.button_pressed(PointerButton::Extra2)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowRight));
                if back {
                    actions.push(Action::Back);
                } else if forward {
                    actions.push(Action::Forward);
                }
            });
        }
        background::paint(
            ui.painter(),
            ui.max_rect(),
            tones.background,
            tones.mark,
            self.scrolled * background::PARALLAX,
        );
        if let Some(error) = self.error.clone() {
            // Announcements go to the top (UI laws).
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    ui.label(RichText::new(error).color(ui.visuals().error_fg_color));
                });
        }
        egui::Panel::left("places")
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .resizable(true)
            .default_size(260.0)
            .min_size(180.0)
            .show(ui, |ui| self.places_pane(ui, tones, &mut actions));
        egui::Panel::right("note")
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .resizable(true)
            .default_size(380.0)
            .min_size(240.0)
            .show(ui, |ui| self.note_pane(ui, tones, &mut actions));
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .show(ui, |ui| self.notes_pane(ui, tones, &mut actions));
        self.dialog(ui.ctx(), tones, &mut actions);
        self.dragged(ui.ctx(), tones);
        for action in actions {
            self.apply(action);
        }
    }

    fn places_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        egui::ScrollArea::vertical()
            .id_salt("places")
            .show(ui, |ui| {
                let text = ui.visuals().text_color();
                let rows = [
                    (Selection::All, icons::ALL_NOTES, t.all_notes_title()),
                    (Selection::Unsorted, icons::UNSORTED, t.unsorted()),
                ];
                for (selection, icon, label) in rows {
                    let on = self.selection == selection;
                    if row(ui, tones, on, 0.0, Some((icon, text)), label, false).clicked() {
                        actions.push(Action::Select(selection));
                    }
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.areas_title()).strong().color(text));
                    // "+" at the right end of the heading.
                    ui.add_space((ui.available_width() - 28.0).max(0.0));
                    if icon_button(ui, icons::ADD, t.new_area(), text).clicked() {
                        actions.push(Action::NewArea);
                    }
                });
                ui.add_space(4.0);
                for (area, topics) in &self.view.areas {
                    let colour = area_colour(area.colour.as_deref()).unwrap_or(text);
                    let icon = icons::area_icon(area.icon.as_deref()).map(|i| (i, colour));
                    let on = self.selection == Selection::Place(area.id.clone());
                    let resp = row(ui, tones, on, 0.0, icon, &area_name(&t, area), true);
                    place_row(&resp, &area.id, actions);
                    resp.context_menu(|ui| {
                        let items = [
                            (t.new_topic(), Action::NewTopic(area.id.clone())),
                            (t.rename(), Action::Rename(area.id.clone())),
                            (t.area_look(), Action::Look(area.id.clone())),
                            (t.delete(), Action::Delete(area.id.clone())),
                        ];
                        for (label, action) in items {
                            if ui.button(RichText::new(label).color(text)).clicked() {
                                actions.push(action);
                            }
                        }
                    });
                    for topic in topics {
                        let on = self.selection == Selection::Place(topic.id.clone());
                        let resp = row(ui, tones, on, 24.0, None, &topic_name(&t, topic), true);
                        place_row(&resp, &topic.id, actions);
                        resp.context_menu(|ui| {
                            let items = [
                                (t.rename(), Action::Rename(topic.id.clone())),
                                (t.delete(), Action::Delete(topic.id.clone())),
                            ];
                            for (label, action) in items {
                                if ui.button(RichText::new(label).color(text)).clicked() {
                                    actions.push(action);
                                }
                            }
                        });
                    }
                }
                ui.add_space(12.0);
                let on = self.selection == Selection::Trash;
                let trash = Some((icons::TRASH, text));
                if row(ui, tones, on, 0.0, trash, t.trash_title(), false).clicked() {
                    actions.push(Action::Select(Selection::Trash));
                }
            });
    }

    /// The title of the middle pane.
    fn title(&self) -> String {
        match &self.selection {
            Selection::All => self.t.all_notes_title().to_string(),
            Selection::Unsorted => self.t.unsorted().to_string(),
            Selection::Trash => self.t.trash_title().to_string(),
            Selection::Place(id) => self
                .view
                .areas
                .iter()
                .find_map(|(area, topics)| {
                    if area.id == *id {
                        return Some(area_name(&self.t, area));
                    }
                    topics.iter().find(|t| t.id == *id).map(|t| {
                        format!("{} / {}", area_name(&self.t, area), topic_name(&self.t, t))
                    })
                })
                .unwrap_or_default(),
        }
    }

    /// The capture field (UI laws, "Capture"): it never asks anything.
    /// Once something is typed, the Nudge times show; Ctrl+Enter takes
    /// the 1 h one.
    fn capture_card(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let id = Id::new("capture");
        embossed(ui, tones, false, None, |ui| {
            let focused = ui.memory(|m| m.has_focus(id));
            if focused
                && !self.capture.trim().is_empty()
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
            {
                actions.push(Action::Capture(Notes::default_nudge()));
            }
            ui.add(
                TextEdit::multiline(&mut self.capture)
                    .id(id)
                    .hint_text(t.capture_hint())
                    .text_color(text)
                    .font(FontId::proportional(16.0))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .frame(Frame::NONE),
            );
            if self.capture.trim().is_empty() {
                return;
            }
            ui.add_space(4.0);
            ui.label(RichText::new(t.nudge_heading()).strong().color(text));
            ui.horizontal(|ui| {
                let choices = [
                    (t.snooze10(), Duration::minutes(10)),
                    (t.snooze30(), Duration::minutes(30)),
                    (t.snooze60(), Notes::default_nudge()),
                ];
                for (label, nudge_in) in choices {
                    if ui.button(RichText::new(label).color(text)).clicked() {
                        actions.push(Action::Capture(nudge_in));
                    }
                }
            });
            ui.label(RichText::new(t.capture_keys_hint()).small().color(weak));
        });
    }

    fn notes_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let text = ui.visuals().text_color();
        ui.horizontal(|ui| {
            ui.heading(RichText::new(self.title()).color(text));
            // An Area's level adds Topics (UI laws, "Navigation").
            if let Selection::Place(id) = &self.selection
                && self.view.area(id).is_some()
                && icon_button(ui, icons::ADD, self.t.new_topic(), text).clicked()
            {
                actions.push(Action::NewTopic(id.clone()));
            }
        });
        ui.add_space(8.0);
        let trash = self.selection == Selection::Trash;
        if !trash {
            ui.spacing_mut().item_spacing.y = 16.0;
            self.capture_card(ui, tones, actions);
        }
        if self.view.notes.is_empty() {
            let empty = if trash {
                self.t.trash_empty()
            } else {
                self.t.no_notes_yet()
            };
            ui.label(RichText::new(empty).color(ui.visuals().weak_text_color()));
            return;
        }
        let output = egui::ScrollArea::vertical()
            .id_salt("notes")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Room for the emboss around every card.
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                for placed in &self.view.notes {
                    let selected = self.selected_note.as_deref() == Some(placed.note.id.as_str());
                    // A Note in the trash is restored, not filed.
                    let draggable = !placed.note.is_trashed();
                    let card = note_card(ui, &self.t, tones, placed, selected, draggable);
                    if card.clicked() {
                        actions.push(Action::SelectNote(placed.note.id.clone()));
                    }
                }
                ui.add_space(8.0);
            });
        if output.state.offset.y != self.scrolled {
            // The marks follow on the next pass.
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
    }

    /// The selected Note's editor (UI laws, "Desktop").
    fn note_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let Some((placed, sections)) = &self.view.detail else {
            return;
        };
        let t = self.t;
        let note = &placed.note;
        let trashed = note.is_trashed();
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let areas = &self.view.areas;
        let edited = &mut self.text;
        let amend = &mut self.amend;
        egui::ScrollArea::vertical()
            .id_salt("note")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                embossed(ui, tones, false, None, |ui| {
                    if trashed {
                        ui.label(RichText::new(&note.text).size(18.0).color(text));
                        return;
                    }
                    let id = Id::new("note text");
                    if ui.memory(|m| m.has_focus(id))
                        && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
                    {
                        actions.push(Action::SaveText);
                    }
                    let field = ui.add(
                        TextEdit::multiline(edited)
                            .id(id)
                            .text_color(text)
                            .font(FontId::proportional(18.0))
                            .desired_rows(1)
                            .desired_width(f32::INFINITY)
                            .frame(Frame::NONE),
                    );
                    if field.lost_focus() {
                        actions.push(Action::SaveText);
                    }
                    let changed = edited.trim();
                    if !changed.is_empty()
                        && changed != note.text
                        && ui.button(RichText::new(t.save()).color(text)).clicked()
                    {
                        actions.push(Action::SaveText);
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.importance()).strong().color(text));
                    let choices = [
                        (Importance::Unimportant, t.importance_unimportant()),
                        (Importance::Normal, t.importance_normal()),
                        (Importance::Important, t.importance_important()),
                    ];
                    for (importance, label) in choices {
                        let on = note.importance == importance;
                        if trashed {
                            if on {
                                ui.label(RichText::new(label).color(text));
                            }
                        } else if row(ui, tones, on, 0.0, None, label, false).clicked() && !on {
                            actions.push(Action::Importance(importance));
                        }
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.captured_heading()).strong().color(text));
                    ui.label(RichText::new(when(&t, note.captured)).color(text));
                    ui.horizontal(|ui| {
                        let colour = placed
                            .area
                            .as_ref()
                            .and_then(|a| area_colour(a.colour.as_deref()))
                            .unwrap_or(weak);
                        let icon = placed
                            .area
                            .as_ref()
                            .and_then(|a| icons::area_icon(a.icon.as_deref()))
                            .unwrap_or(icons::NO_AREA_ICON);
                        icons::show(ui, icon, 18.0, colour);
                        let place =
                            place_line(&t, placed).unwrap_or_else(|| t.unsorted().to_string());
                        ui.label(RichText::new(place).color(text));
                    });
                    if trashed {
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::TRASH, 18.0, weak);
                            ui.label(RichText::new(t.trash_title()).color(text));
                        });
                        return;
                    }
                    ui.menu_button(RichText::new(t.file_into()).color(text), |ui| {
                        for (area, topics) in areas {
                            let mut file = |ui: &mut Ui, id: &str, label: String, indent| {
                                ui.horizontal(|ui| {
                                    ui.add_space(indent);
                                    if ui.button(RichText::new(label).color(text)).clicked() {
                                        actions.push(Action::File {
                                            note: note.id.clone(),
                                            place: id.to_string(),
                                        });
                                    }
                                });
                            };
                            file(ui, &area.id, area_name(&t, area), 0.0);
                            for topic in topics {
                                file(ui, &topic.id, topic_name(&t, topic), 16.0);
                            }
                        }
                        if areas.is_empty() {
                            ui.label(RichText::new(t.no_areas_yet()).color(weak));
                        }
                    });
                    ui.label(RichText::new(t.drag_to_file()).small().color(weak));
                });
                for section in sections {
                    embossed(ui, tones, false, None, |ui| {
                        ui.label(RichText::new(&section.text).color(text));
                        ui.label(
                            RichText::new(when(&t, section.captured))
                                .small()
                                .color(weak),
                        );
                    });
                }
                if trashed {
                    if ui.button(RichText::new(t.restore()).color(text)).clicked() {
                        actions.push(Action::Restore);
                    }
                    return;
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.add(
                        TextEdit::multiline(amend)
                            .id(Id::new("amend"))
                            .hint_text(t.amend_hint())
                            .text_color(text)
                            .desired_rows(1)
                            .desired_width(f32::INFINITY)
                            .frame(Frame::NONE),
                    );
                    let ready = !amend.trim().is_empty();
                    let add = ui.add_enabled(
                        ready,
                        egui::Button::new(RichText::new(t.amend()).color(text)),
                    );
                    if add.clicked() {
                        actions.push(Action::Amend);
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t.merge_into()).color(text))
                        .clicked()
                    {
                        actions.push(Action::StartMerge);
                    }
                    if ui
                        .button(RichText::new(t.move_to_trash()).color(text))
                        .clicked()
                    {
                        actions.push(Action::Trash);
                    }
                });
            });
    }

    /// The open dialog, over everything else.
    fn dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        let focus = std::mem::take(&mut self.focus);
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let t = self.t;
        let view = &self.view;
        let frame = Frame::NONE
            .fill(tones.card)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(20));
        let modal = egui::Modal::new(Id::new("dialog"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(400.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                match dialog {
                    Dialog::NewPlace { area, name } => {
                        let title = if area.is_some() {
                            t.new_topic()
                        } else {
                            t.new_area()
                        };
                        let form = (title, t.create(), Action::AddPlace);
                        name_form(ui, &t, form, (name, focus), actions);
                    }
                    Dialog::Rename { name, .. } => {
                        let form = (t.rename(), t.save(), Action::SaveName);
                        name_form(ui, &t, form, (name, focus), actions);
                    }
                    Dialog::Look { area } => look_form(ui, &t, view.area(area), actions),
                    Dialog::Delete(deleting) => delete_form(ui, &t, deleting, actions),
                    Dialog::Merge { query, others } => {
                        merge_form(ui, &t, tones, (query, focus), others, actions);
                    }
                }
            });
        if modal.should_close() {
            actions.push(Action::CloseDialog);
        }
    }

    /// The card's text next to the pointer while a card is dragged.
    fn dragged(&self, ctx: &egui::Context, tones: Tones) {
        let Some(dragged) = egui::DragAndDrop::payload::<DraggedNote>(ctx) else {
            return;
        };
        let Some(pos) = ctx.pointer_interact_pos() else {
            return;
        };
        let Some(placed) = self.view.notes.iter().find(|p| p.note.id == dragged.0) else {
            return;
        };
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        egui::Area::new(Id::new("dragged"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos + egui::vec2(14.0, 14.0))
            .interactable(false)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(tones.card)
                    .corner_radius(8)
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_max_width(240.0);
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(&placed.note.text).color(text));
                    });
            });
    }
}

/// Selects the Area or Topic `place` when its row is clicked, and files a
/// card dropped on it there.
fn place_row(resp: &egui::Response, place: &str, actions: &mut Vec<Action>) {
    if resp.clicked() {
        actions.push(Action::Select(Selection::Place(place.to_string())));
    }
    if let Some(dragged) = resp.dnd_release_payload::<DraggedNote>() {
        actions.push(Action::File {
            note: dragged.0.clone(),
            place: place.to_string(),
        });
    }
}

/// A square button that shows `icon`, called `label`.
fn icon_button(ui: &mut Ui, icon: char, label: &str, colour: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    icons::paint(ui.painter(), rect.shrink(2.0), icon, colour);
    resp.on_hover_text(label)
}

/// The heading of a dialog.
fn title(ui: &mut Ui, title: &str) {
    let text = ui.visuals().text_color();
    ui.label(RichText::new(title).size(18.0).strong().color(text));
}

/// A dialog's text field, taking the keyboard when `focus` says so.
fn focused_field(
    ui: &mut Ui,
    (value, focus): (&mut String, bool),
    id: &str,
    hint: &str,
) -> egui::Response {
    let text = ui.visuals().text_color();
    let field = ui.add(
        TextEdit::singleline(value)
            .id(Id::new(id))
            .hint_text(hint)
            .text_color(text)
            .desired_width(f32::INFINITY),
    );
    if focus {
        field.request_focus();
    }
    field
}

/// Cancel, which closes the dialog without a change.
fn cancel(ui: &mut Ui, t: &L10n, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
        actions.push(Action::CloseDialog);
    }
}

/// A confirm button for a destructive step: its verb in the error colour
/// (UI laws).
fn destructive(ui: &mut Ui, label: &str) -> bool {
    let error = ui.visuals().error_fg_color;
    ui.button(RichText::new(label).color(error)).clicked()
}

/// Asks for a name: `(title, verb, done)` say what for. Enter takes it.
fn name_form(
    ui: &mut Ui,
    t: &L10n,
    (heading, verb, done): (&str, &str, Action),
    (name, focus): (&mut String, bool),
    actions: &mut Vec<Action>,
) {
    title(ui, heading);
    let field = focused_field(ui, (&mut *name, focus), "name", t.name_hint());
    let ready = !name.trim().is_empty();
    if ready && field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
        actions.push(done.clone());
    }
    ui.horizontal(|ui| {
        cancel(ui, t, actions);
        let text = ui.visuals().text_color();
        let button = egui::Button::new(RichText::new(verb).color(text));
        if ui.add_enabled(ready, button).clicked() {
            actions.push(done);
        }
    });
}

/// An Area's colour and icon, each kept as it is picked.
fn look_form(ui: &mut Ui, t: &L10n, area: Option<&Area>, actions: &mut Vec<Action>) {
    let Some(area) = area else {
        return;
    };
    let text = ui.visuals().text_color();
    title(ui, t.area_look());
    ui.label(RichText::new(t.colour_heading()).strong().color(text));
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(RichText::new(t.no_colour()).color(text))
            .clicked()
        {
            actions.push(Action::Colour(area.id.clone(), None));
        }
        for hex in AREA_PALETTE {
            let on = area.colour.as_deref() == Some(hex);
            let colour = area_colour(Some(hex)).unwrap_or(text);
            if swatch(ui, hex, on, |painter, rect| {
                painter.circle_filled(rect.center(), rect.width() / 2.0 - 3.0, colour);
            })
            .clicked()
            {
                actions.push(Action::Colour(area.id.clone(), Some(hex.to_string())));
            }
        }
    });
    ui.label(RichText::new(t.icon_heading()).strong().color(text));
    ui.horizontal_wrapped(|ui| {
        if ui.button(RichText::new(t.no_icon()).color(text)).clicked() {
            actions.push(Action::Icon(area.id.clone(), None));
        }
        for key in AREA_ICONS {
            let Some(glyph) = icons::area_icon(Some(key)) else {
                continue;
            };
            let on = area.icon.as_deref() == Some(key);
            if swatch(ui, key, on, |painter, rect| {
                icons::paint(painter, rect.shrink(5.0), glyph, text);
            })
            .clicked()
            {
                actions.push(Action::Icon(area.id.clone(), Some(key.to_string())));
            }
        }
    });
    if ui.button(RichText::new(t.close()).color(text)).clicked() {
        actions.push(Action::CloseDialog);
    }
}

/// A round choice called `label`, ringed while it is `on`.
fn swatch(
    ui: &mut Ui,
    label: &str,
    on: bool,
    paint: impl FnOnce(&egui::Painter, egui::Rect),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, label));
    paint(ui.painter(), rect);
    if on || resp.hovered() {
        let ring = if on {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        ui.painter().circle_stroke(
            rect.center(),
            rect.width() / 2.0 - 1.0,
            egui::Stroke::new(2.0, ring),
        );
    }
    resp
}

/// Asks once whether to delete an Area or Topic and, with Notes inside,
/// where they go: Unsorted, the Topic's Area or the trash.
fn delete_form(ui: &mut Ui, t: &L10n, deleting: &Deleting, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    title(ui, &deleting.question);
    for line in &deleting.lines {
        ui.label(RichText::new(line).color(text));
    }
    ui.horizontal_wrapped(|ui| {
        cancel(ui, t, actions);
        if !deleting.has_notes {
            if destructive(ui, t.delete()) {
                actions.push(Action::DeletePlace(NotesGo::Unsorted));
            }
            return;
        }
        if destructive(ui, t.move_to_unsorted()) {
            actions.push(Action::DeletePlace(NotesGo::Unsorted));
        }
        if let Some(area) = &deleting.area
            && destructive(ui, &t.move_to_area(area))
        {
            actions.push(Action::DeletePlace(NotesGo::Area));
        }
        if destructive(ui, t.trash_them()) {
            actions.push(Action::DeletePlace(NotesGo::Trash));
        }
    });
}

/// Picks the Note to merge the selected one into, from every other Note,
/// newest first, narrowed by what their text holds.
fn merge_form(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (query, focus): (&mut String, bool),
    others: &[(String, String)],
    actions: &mut Vec<Action>,
) {
    let weak = ui.visuals().weak_text_color();
    title(ui, t.merge_into());
    focused_field(ui, (&mut *query, focus), "merge query", t.search_all_hint());
    let wanted = query.trim().to_lowercase();
    let mut shown = others
        .iter()
        .filter(|(_, text)| text.to_lowercase().contains(&wanted))
        .peekable();
    if shown.peek().is_none() {
        let empty = if others.is_empty() {
            t.no_other_notes()
        } else {
            t.nothing_found()
        };
        ui.label(RichText::new(empty).color(weak));
    }
    egui::ScrollArea::vertical()
        .id_salt("merge")
        .max_height(320.0)
        .show(ui, |ui| {
            for (id, text) in shown {
                if row(ui, tones, false, 0.0, None, text, false).clicked() {
                    actions.push(Action::Merge(id.clone()));
                }
            }
        });
    cancel(ui, t, actions);
}
/// An Area's stored colour (`RRGGBB`), or `None` when it has none or it
/// cannot be read.
pub fn area_colour(hex: Option<&str>) -> Option<Color32> {
    let value = u32::from_str_radix(hex?, 16).ok()?;
    let [_, r, g, b] = value.to_be_bytes();
    Some(Color32::from_rgb(r, g, b))
}

/// When something was captured, in the app's language and local time:
/// the phone's `yMd` and `Hm`.
pub fn when(t: &L10n, at: DateTime<Utc>) -> String {
    let local = at.with_timezone(&Local);
    match t.locale() {
        "de" => local.format("%-d.%-m.%Y %H:%M").to_string(),
        _ => local.format("%-m/%-d/%Y %H:%M").to_string(),
    }
}

/// Where a Note is filed, `Area / Topic`; `None` while it is in no
/// place.
fn place_line(t: &L10n, placed: &Placed) -> Option<String> {
    let area = placed.area.as_ref()?;
    Some(match &placed.topic {
        Some(topic) => format!("{} / {}", area_name(t, area), topic_name(t, topic)),
        None => area_name(t, area),
    })
}

/// A row of the left pane: self-painted so its text reads in every
/// state, with the accent while selected. A `droppable` row is outlined
/// while a card is held over it.
fn row(
    ui: &mut Ui,
    tones: Tones,
    on: bool,
    indent: f32,
    icon: Option<(char, Color32)>,
    label: &str,
    droppable: bool,
) -> egui::Response {
    let font = egui::FontId::proportional(15.0);
    let height = 30.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, on, label));
    if ui.is_rect_visible(rect) {
        let fg = ui.visuals().text_color();
        if droppable && resp.dnd_hover_payload::<DraggedNote>().is_some() {
            ui.painter().rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(2.0, tones.accent),
                egui::StrokeKind::Inside,
            );
        }
        let fill = if on {
            tones.accent.gamma_multiply(0.3)
        } else if resp.hovered() {
            tones.card.gamma_multiply(0.7)
        } else {
            Color32::TRANSPARENT
        };
        if fill.a() > 0 {
            ui.painter().rect_filled(rect, 8.0, fill);
        }
        let mut x = rect.left() + 8.0 + indent;
        if let Some((glyph, colour)) = icon {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(x + 9.0, rect.center().y),
                egui::vec2(18.0, 18.0),
            );
            icons::paint(ui.painter(), icon_rect, glyph, colour);
            x += 26.0;
        }
        let text = ui.painter().layout(
            label.to_string(),
            font,
            fg,
            (rect.right() - x - 8.0).max(0.0),
        );
        let y = rect.center().y - text.size().y / 2.0;
        ui.painter().galley(egui::pos2(x, y), text, fg);
    }
    resp
}

/// Lays out `content` on a softly embossed card the width of `ui`;
/// `band` runs an Area's colour down its left edge.
fn embossed(
    ui: &mut Ui,
    tones: Tones,
    strong: bool,
    band: Option<Color32>,
    content: impl FnOnce(&mut Ui),
) -> egui::Response {
    let under = ui.painter().add(egui::Shape::Noop);
    let left = if band.is_some() { 22 } else { 16 };
    let inner = Frame::NONE
        .inner_margin(Margin {
            left,
            right: 16,
            top: 14,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.style_mut().interaction.selectable_labels = false;
            content(ui);
        });
    let rect = inner.response.rect;
    let mut shapes = theme::embossed(rect, tones, strong);
    if let Some(colour) = band {
        shapes.push(theme::area_band(rect, colour));
    }
    ui.painter().set(under, egui::Shape::Vec(shapes));
    inner.response
}

/// A Note as a card (UI laws, "Cards"): its text, when it was captured
/// and where it is filed, its Area's colour down the left edge and icon
/// before the place. Important cards carry the accent marker and a
/// stronger emboss; unimportant ones are dimmed. A `draggable` card can
/// be dropped onto an Area or Topic.
fn note_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    placed: &Placed,
    selected: bool,
    draggable: bool,
) -> egui::Response {
    let note = &placed.note;
    let important = note.importance == Importance::Important;
    let colour = placed
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    let inner = ui.scope(|ui| {
        if note.importance == Importance::Unimportant {
            ui.multiply_opacity(theme::UNIMPORTANT_OPACITY);
        }
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let card = embossed(ui, tones, important, colour, |ui| {
            if important {
                // Room for the marker on the right.
                ui.set_max_width(ui.available_width() - 36.0);
            }
            // A photo Note may have no words.
            if !note.text.is_empty() {
                ui.label(RichText::new(&note.text).size(16.0).color(text));
            }
            ui.label(RichText::new(when(t, note.captured)).small().color(weak));
            // A Note that spawned a Routine is not Unsorted, though not
            // filed.
            let place = match place_line(t, placed) {
                Some(line) => Some(line),
                None if note.spawned => None,
                None => Some(t.unsorted().to_string()),
            };
            if let Some(place) = place {
                ui.horizontal(|ui| {
                    if let Some(icon) = placed
                        .area
                        .as_ref()
                        .and_then(|a| icons::area_icon(a.icon.as_deref()))
                    {
                        icons::show(ui, icon, 16.0, colour.unwrap_or(weak));
                    }
                    ui.label(RichText::new(place).size(13.0).color(text));
                });
            }
        });
        let rect = card.rect;
        if important {
            let marker = egui::Rect::from_center_size(
                egui::pos2(rect.right() - 28.0, rect.top() + 28.0),
                egui::vec2(24.0, 24.0),
            );
            icons::paint(ui.painter(), marker, icons::IMPORTANT, tones.accent);
        }
        if selected {
            ui.painter().rect_stroke(
                rect,
                theme::CARD_RADIUS,
                egui::Stroke::new(2.0, tones.accent),
                egui::StrokeKind::Outside,
            );
        }
        rect
    });
    let rect = inner.inner;
    let sense = if draggable {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let resp = ui.interact(rect, Id::new(("card", &note.id)), sense);
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, &note.text));
    if draggable {
        resp.dnd_set_drag_payload(DraggedNote(note.id.clone()));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn area_colours_read_as_the_phone_reads_them() {
        assert_eq!(
            area_colour(Some("6A1B9A")),
            Some(Color32::from_rgb(0x6A, 0x1B, 0x9A))
        );
        assert_eq!(area_colour(Some("nope")), None);
        assert_eq!(area_colour(None), None);
    }

    #[test]
    fn capture_times_read_in_the_apps_language() {
        let at = Utc.with_ymd_and_hms(2026, 3, 4, 12, 0, 0).single();
        let Some(at) = at else { panic!("a valid date") };
        let local = at.with_timezone(&Local);
        let en = when(&L10n::new("en"), at);
        let de = when(&L10n::new("de"), at);
        assert!(
            en.starts_with(&local.format("%-m/%-d/%Y").to_string()),
            "{en}"
        );
        assert!(
            de.starts_with(&local.format("%-d.%-m.%Y").to_string()),
            "{de}"
        );
    }
}
