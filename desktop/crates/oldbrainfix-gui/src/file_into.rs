//! File into… (#272, UI laws "File into…"), the same in the window's
//! dialog, over the quick note and as a page of the capture popup: a
//! filter field that takes the keyboard, then "Filed into recently" with
//! the last places this Device filed into while nothing is typed, then
//! the Areas by name, each with its Topics and, in Advanced for who may
//! set up a Topic there, "New Topic…", and "New Area…" at the end. A
//! new place is named here and made, and the Note goes into it.
//!
//! Enter in the filter takes the first row shown: the latest place filed
//! into, else the first match, else New Area… with what is typed.

use egui::{Id, Key, RichText, TextEdit, Ui};
use oldbrainfix_core::{Area, DeviceSettings, PersonSettings, Places, Store, Topic};

use crate::app::name_field;
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::text_menu;
use crate::theme;

/// How tall the list grows before it scrolls.
const LIST_HEIGHT: f32 = 420.0;

/// An Area as File into… lists it: with the Topics it shows and whether
/// "New Topic…" ends it.
#[derive(Debug, Clone, PartialEq)]
struct Branch {
    area: Area,
    topics: Vec<Topic>,
    adds_topic: bool,
}

/// A row of File into… that was clicked or that Enter takes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    Place(String),
    NewTopic(String),
    NewArea,
}

/// Naming a new Area, or a new Topic of the Area `area`.
#[derive(Debug, Clone, PartialEq)]
struct Naming {
    area: Option<String>,
    name: String,
}

/// What File into… came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picked {
    /// The Area or Topic of this id.
    Place(String),
    /// The new Area `name`, or the new Topic `name` of `area`, to make
    /// and file into.
    Make { area: Option<String>, name: String },
    /// Cancel: nothing.
    Close,
}

/// File into… while it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct FileIntoPicker {
    tree: Vec<Branch>,
    /// The places filed into last that the tree lists, the latest first,
    /// each as its Area and, for a Topic, the Topic.
    recent: Vec<(Area, Option<Topic>)>,
    query: String,
    naming: Option<Naming>,
    /// Whether the field shown is still to take the keyboard.
    focus: bool,
    /// The height the list took so far.
    tall: f32,
    /// Whether the list is to show from its top: as it opens and for
    /// what is typed, not where it was scrolled to last.
    from_top: bool,
}

impl FileIntoPicker {
    /// File into… over `areas` with their Topics: only the Areas this
    /// Device may write in (ADR 0013), by name as `t` shows them, and
    /// the places it filed into last among them.
    pub fn read(
        store: &Store,
        t: &L10n,
        areas: &[(Area, Vec<Topic>)],
    ) -> oldbrainfix_core::Result<FileIntoPicker> {
        let advanced = PersonSettings::new(store).advanced()?;
        let by_name = |a: &String, b: &String| a.to_lowercase().cmp(&b.to_lowercase());
        let mut tree = Vec::new();
        for (area, topics) in areas {
            if !store.may_write_in(&area.id)? {
                continue;
            }
            let mut topics = topics.clone();
            topics.sort_by(|a, b| by_name(&topic_name(t, a), &topic_name(t, b)));
            tree.push(Branch {
                area: area.clone(),
                topics,
                adds_topic: advanced && store.may_manage(Some(&area.id))?,
            });
        }
        tree.sort_by(|a, b| by_name(&area_name(t, &a.area), &area_name(t, &b.area)));
        let recent = DeviceSettings::new(store)
            .recently_filed_into()?
            .iter()
            .filter_map(|id| listed(&tree, id))
            .collect();
        Ok(FileIntoPicker {
            tree,
            recent,
            query: String::new(),
            naming: None,
            focus: true,
            tall: 0.0,
            from_top: true,
        })
    }

    /// Whether a new place is being named.
    pub fn naming(&self) -> bool {
        self.naming.is_some()
    }

    /// From naming a new place back to the list, nothing made.
    pub fn back(&mut self) {
        self.naming = None;
        self.focus = true;
    }

    /// The row Enter takes, the first shown: the place filed into last
    /// while nothing is typed, else the first match, where only Topics
    /// of an Area match the first of them, as the Area is no match
    /// itself; New Area… when there is none.
    fn first(&self, t: &L10n) -> Row {
        if self.filtering() {
            return match self.shown(t).first() {
                Some((area, _, _)) if matches(&self.query, &area_name(t, area)) => {
                    Row::Place(area.id.clone())
                }
                Some((_, topics, _)) => topics
                    .first()
                    .map_or(Row::NewArea, |p| Row::Place(p.id.clone())),
                None => Row::NewArea,
            };
        }
        match (self.recent.first(), self.tree.first()) {
            (Some((area, topic)), _) => Row::Place(id_of(area, topic.as_ref())),
            (None, Some(branch)) => Row::Place(branch.area.id.clone()),
            (None, None) => Row::NewArea,
        }
    }

    fn filtering(&self) -> bool {
        !self.query.trim().is_empty()
    }

    /// The tree with what the filter matches: an Area whose name matches
    /// with all its Topics, else with the Topics whose names match.
    fn shown(&self, t: &L10n) -> Vec<(&Area, Vec<&Topic>, bool)> {
        let mut shown = Vec::new();
        for branch in &self.tree {
            if matches(&self.query, &area_name(t, &branch.area)) {
                shown.push((
                    &branch.area,
                    branch.topics.iter().collect(),
                    branch.adds_topic,
                ));
                continue;
            }
            let topics: Vec<&Topic> = branch
                .topics
                .iter()
                .filter(|p| matches(&self.query, &topic_name(t, p)))
                .collect();
            if !topics.is_empty() {
                shown.push((&branch.area, topics, branch.adds_topic));
            }
        }
        shown
    }

    /// Draws File into… under its title and says what was picked.
    pub fn show(&mut self, ui: &mut Ui, t: &L10n) -> Option<Picked> {
        if self.naming.is_some() {
            return self.show_naming(ui, t);
        }
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let accent = theme::tones(ui.visuals()).accent;
        let field = ui.add(
            TextEdit::singleline(&mut self.query)
                .id(Id::new("file into filter"))
                .hint_text(t.file_into_filter_hint())
                .text_color(text)
                .desired_width(f32::INFINITY),
        );
        text_menu::show(ui, t, &field, &self.query);
        name_field(ui, &field, t.file_into_filter_hint());
        self.take_focus(ui, &field);
        let mut row = None;
        // Ctrl+Enter as Enter, as in every dialog (#277).
        if (field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) || theme::ctrl_enter(ui)
        {
            row = Some(self.first(t));
        }
        // As tall as the whole list was, so the dialog stays put while
        // typing; it grows with what it shows, not with the room it has.
        let tall = self.tall;
        let mut list = egui::ScrollArea::vertical()
            .id_salt("file into")
            .max_height(LIST_HEIGHT)
            // Not the room a dialog had the frame before.
            .min_scrolled_height(LIST_HEIGHT);
        if std::mem::take(&mut self.from_top) || field.changed() {
            list = list.vertical_scroll_offset(0.0);
        }
        let list = list.show(ui, |ui| {
            ui.set_min_height(tall);
            let place = |ui: &mut Ui, id: &str, name: String, indent: f32| {
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    ui.button(RichText::new(name).color(text)).clicked()
                })
                .inner
                .then(|| Row::Place(id.to_string()))
            };
            let new_topic = |ui: &mut Ui, area: &str| {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    let label = RichText::new(t.file_into_new_topic()).color(accent);
                    ui.button(label).clicked()
                })
                .inner
                .then(|| Row::NewTopic(area.to_string()))
            };
            if self.filtering() {
                let shown = self.shown(t);
                if shown.is_empty() {
                    ui.label(RichText::new(t.file_into_no_match()).color(weak));
                }
                for (area, topics, adds_topic) in shown {
                    row = row.take().or(place(ui, &area.id, area_name(t, area), 0.0));
                    for topic in topics {
                        let name = topic_name(t, topic);
                        row = row.take().or(place(ui, &topic.id, name, 24.0));
                    }
                    if adds_topic {
                        row = row.take().or(new_topic(ui, &area.id));
                    }
                }
            } else {
                if !self.recent.is_empty() {
                    ui.label(RichText::new(t.file_into_recent()).small().color(weak));
                }
                for (area, topic) in &self.recent {
                    let shown = match topic {
                        Some(topic) => topic_name(t, topic),
                        None => area_name(t, area),
                    };
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new(shown).color(text)).clicked() {
                            row = Some(Row::Place(id_of(area, topic.as_ref())));
                        }
                        // A Topic shows the Area it is in.
                        if topic.is_some() {
                            let in_area = area_name(t, area);
                            ui.label(RichText::new(in_area).small().color(weak));
                        }
                    });
                }
                if !self.recent.is_empty() {
                    ui.separator();
                }
                for branch in &self.tree {
                    let name = area_name(t, &branch.area);
                    row = row.take().or(place(ui, &branch.area.id, name, 0.0));
                    for topic in &branch.topics {
                        let name = topic_name(t, topic);
                        row = row.take().or(place(ui, &topic.id, name, 24.0));
                    }
                    if branch.adds_topic {
                        row = row.take().or(new_topic(ui, &branch.area.id));
                    }
                }
            }
            let label = RichText::new(t.file_into_new_area()).color(accent);
            if ui.button(label).clicked() {
                row = Some(Row::NewArea);
            }
        });
        self.tall = self.tall.max(list.content_size.y.min(LIST_HEIGHT));
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            return Some(Picked::Close);
        }
        match row? {
            Row::Place(id) => Some(Picked::Place(id)),
            Row::NewTopic(area) => {
                // What is typed matched a name there already: left out.
                self.start_naming(Some(area), String::new());
                None
            }
            Row::NewArea => {
                let name = self.query.trim().to_string();
                self.start_naming(None, name);
                None
            }
        }
    }

    fn start_naming(&mut self, area: Option<String>, name: String) {
        self.naming = Some(Naming { area, name });
        self.focus = true;
    }

    /// The name of the new place: Create makes it, Cancel goes back to
    /// the list.
    fn show_naming(&mut self, ui: &mut Ui, t: &L10n) -> Option<Picked> {
        let text = ui.visuals().text_color();
        let naming = self.naming.as_mut()?;
        let heading = if naming.area.is_some() {
            t.new_topic()
        } else {
            t.new_area()
        };
        ui.label(RichText::new(heading).strong().color(text));
        let field = ui.add(
            TextEdit::singleline(&mut naming.name)
                .id(Id::new("file into name"))
                .hint_text(t.name_hint())
                .text_color(text)
                .desired_width(f32::INFINITY),
        );
        text_menu::show(ui, t, &field, &naming.name);
        name_field(ui, &field, t.name_hint());
        let ready = !naming.name.trim().is_empty();
        let mut made = ready && field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let mut back = false;
        ui.horizontal(|ui| {
            back = ui.button(RichText::new(t.cancel()).color(text)).clicked();
            made |= theme::main_action(ui, ready, t.create());
        });
        let picked = made.then(|| Picked::Make {
            area: naming.area.clone(),
            name: naming.name.trim().to_string(),
        });
        self.take_focus(ui, &field);
        if back {
            self.back();
        }
        picked
    }

    /// Gives `field` the keyboard while asked; a dialog's sizing pass
    /// drops it, so it is asked again after one.
    fn take_focus(&mut self, ui: &Ui, field: &egui::Response) {
        if self.focus {
            field.request_focus();
            if !ui.is_sizing_pass() {
                self.focus = false;
            }
        }
    }
}

/// Whether `name` holds what is typed, `query`, case aside.
fn matches(query: &str, name: &str) -> bool {
    name.to_lowercase().contains(&query.trim().to_lowercase())
}

/// The place `id` as `tree` lists it: its Area and, for a Topic, the
/// Topic.
fn listed(tree: &[Branch], id: &str) -> Option<(Area, Option<Topic>)> {
    tree.iter().find_map(|branch| {
        if branch.area.id == id {
            return Some((branch.area.clone(), None));
        }
        let topic = branch.topics.iter().find(|p| p.id == id)?;
        Some((branch.area.clone(), Some(topic.clone())))
    })
}

/// The id of the Topic, else of the Area.
fn id_of(area: &Area, topic: Option<&Topic>) -> String {
    topic.map_or(&area.id, |p| &p.id).clone()
}

/// Makes the new Area `name`, or the new Topic `name` of `area`; its id.
pub fn make(store: &Store, area: Option<&str>, name: &str) -> oldbrainfix_core::Result<String> {
    let places = Places::new(store);
    Ok(match area {
        Some(area) => places.add_topic(area, name)?.id,
        None => places.add_area(name)?.id,
    })
}
