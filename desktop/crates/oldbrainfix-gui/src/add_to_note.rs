//! Add to Note… (#308), the same over the window's quick note, as a page
//! of the capture popup and from an Unsorted Note's card: a filter field
//! that takes the keyboard, then "Added to recently" with the Notes this
//! Device added to last while nothing is typed, as File into… has its
//! places (#272), then every Note the core's search finds, newest first.
//! Only Notes this Device may add to are offered (ADR 0013), none in the
//! trash and not a single Note being added; of several cards selected
//! each is offered, the one picked taking the others in, as Merge into…
//! does (#273). The pick is kept as added to last, added or not, as File
//! into… keeps its pick.
//!
//! Enter in the filter takes the first row shown: the Note added to
//! last while nothing is typed, else the first found.

use egui::{Id, Key, RichText, TextEdit, Ui};
use oldbrainfix_core::{DeviceSettings, Search, Store};

use crate::app::{merge_row, name_field, row_title};
use crate::l10n::L10n;
use crate::text_menu;
use crate::theme::{self, Tones};

/// How tall the list grows before it scrolls.
const LIST_HEIGHT: f32 = 320.0;

/// A Note as Add to Note… lists it: its id and the title it shows under.
pub type Listed = (String, String);

/// What was asked of Add to Note….
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteAsk {
    /// What is typed changed: find the Notes again ([`NotePicker::find`]).
    Find,
    /// The Note of this id, to keep as added to last ([`picked`]).
    Note(String),
    /// Cancel: nothing.
    Close,
}

/// Add to Note… while it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct NotePicker {
    /// The Note being added alone, not offered; several are (#273).
    alone: Option<String>,
    /// The Notes added to last that are offered, the latest first.
    recent: Vec<Listed>,
    /// The Notes found for what is typed.
    found: Vec<Listed>,
    query: String,
    /// Whether the field is still to take the keyboard.
    focus: bool,
}

impl NotePicker {
    /// Add to Note… for the Notes `adding`, none for a capture: every
    /// Note this Device may add to but a single one being added, and those
    /// added to last among them.
    pub fn read(
        store: &Store,
        t: &L10n,
        adding: &[String],
    ) -> oldbrainfix_core::Result<NotePicker> {
        let alone = match adding {
            [one] => Some(one.clone()),
            _ => None,
        };
        let found = offered(store, t, "", alone.as_deref())?;
        let recent = DeviceSettings::new(store)
            .recently_added_to()?
            .iter()
            .filter_map(|id| found.iter().find(|(note, _)| note == id).cloned())
            .collect();
        Ok(NotePicker {
            alone,
            recent,
            found,
            query: String::new(),
            focus: true,
        })
    }

    /// The Notes it shows by id, the recent ones first while nothing is
    /// typed.
    pub fn shown(&self) -> Vec<&str> {
        let recent = if self.filtering() {
            &[][..]
        } else {
            &self.recent
        };
        recent
            .iter()
            .chain(&self.found)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    fn filtering(&self) -> bool {
        !self.query.trim().is_empty()
    }

    /// Finds the Notes again for what is typed.
    pub fn find(&mut self, store: &Store, t: &L10n) -> oldbrainfix_core::Result<()> {
        self.found = offered(store, t, &self.query, self.alone.as_deref())?;
        Ok(())
    }

    /// What is typed into its field.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The Note being added alone, if one is.
    pub fn alone(&self) -> Option<&str> {
        self.alone.as_deref()
    }

    /// Shows the Notes `found` for what is typed, found elsewhere
    /// ([`offered`]), such as on a thread of its own (obf-1068).
    pub fn set_found(&mut self, found: Vec<Listed>) {
        self.found = found;
    }

    /// Draws Add to Note… under its title and says what was asked.
    pub fn show(&mut self, ui: &mut Ui, (t, tones): (&L10n, Tones)) -> Option<NoteAsk> {
        let weak = ui.visuals().weak_text_color();
        let text = ui.visuals().text_color();
        // On its band, so the Notes scroll under its line (obf-1088).
        let field = theme::search_band(ui, |ui| {
            let field = ui.add(
                TextEdit::singleline(&mut self.query)
                    .id(Id::new("add to note filter"))
                    .hint_text(t.search_all_hint())
                    .text_color(text)
                    .desired_width(f32::INFINITY),
            );
            text_menu::show(ui, t, &field, &self.query);
            name_field(ui, &field, t.search_all_hint());
            field
        });
        if self.focus {
            field.request_focus();
            if !ui.is_sizing_pass() {
                self.focus = false;
            }
        }
        let mut picked = None;
        // Ctrl+Enter as Enter, as in every dialog (#277).
        if (field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) || theme::ctrl_enter(ui)
        {
            picked = self.shown().first().map(|id| id.to_string());
        }
        if self.shown().is_empty() {
            let empty = if self.filtering() {
                t.nothing_found()
            } else {
                t.no_note_to_add_to()
            };
            ui.add_space(4.0);
            ui.label(RichText::new(empty).color(weak));
        }
        egui::ScrollArea::vertical()
            .id_salt("add to note")
            .max_height(LIST_HEIGHT)
            .show(ui, |ui| {
                // Room under the band's line, inside, so the list is cut
                // there.
                ui.add_space(4.0);
                if !self.filtering() && !self.recent.is_empty() {
                    ui.label(RichText::new(t.add_to_note_recent()).small().color(weak));
                    for (id, title) in &self.recent {
                        if merge_row(ui, tones, title).clicked() {
                            picked = Some(id.clone());
                        }
                    }
                    ui.separator();
                }
                for (id, title) in &self.found {
                    if merge_row(ui, tones, title).clicked() {
                        picked = Some(id.clone());
                    }
                }
            });
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            return Some(NoteAsk::Close);
        }
        match picked {
            Some(id) => Some(NoteAsk::Note(id)),
            None => field.changed().then_some(NoteAsk::Find),
        }
    }
}

/// Keeps the Note `id` picked as added to last (#308), added or not, as
/// File into… keeps its pick.
pub fn picked(store: &Store, id: &str) -> oldbrainfix_core::Result<()> {
    DeviceSettings::new(store).added_to(id)
}

/// The Notes the core's search finds for `query` that can be added to,
/// by id and the title they show under: those this Device may write in
/// (ADR 0013), but not the Note `alone` being added. The search leaves
/// out the trash and the Notes merged into others.
pub fn offered(
    store: &Store,
    t: &L10n,
    query: &str,
    alone: Option<&str>,
) -> oldbrainfix_core::Result<Vec<Listed>> {
    let mut offered = Vec::new();
    for note in Search::new(store).notes(query, None)? {
        if alone == Some(note.id.as_str()) || !store.may_write_in(&note.id)? {
            continue;
        }
        offered.push((note.id.clone(), row_title(t, Some(&note))));
    }
    Ok(offered)
}
