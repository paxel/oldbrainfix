//! The All Routines and Reminders page (#115): every Routine and
//! Reminder in alphabetical order of its name, also those beyond the
//! Agenda's week; a click opens its editor, New Session opens the editor
//! of a Session whose Note is made from the text typed there, and Export
//! saves one calendar file of all of them, as the phone's
//! `AllRoutinesScreen`.
//! Each card carries its Area's colour and icon, as on the Agenda. Those
//! whose schedule a newer version of the app saved follow, saying so, a
//! click opening their Note (ADR 0034); they are in no file.

use std::collections::HashMap;

use egui::{Id, RichText, Ui};
use oldbrainfix_core::{Area, Note, Routine, Routines};

use crate::app::{area_colour, banded_clickable_card, icon_button};
use crate::icons;
use crate::l10n::{self, L10n};
use crate::schedule_text::schedule_text;
use crate::theme::Tones;

/// What the page asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllRoutinesAction {
    /// The editor of the Routine with this id, on the right.
    Open(String),
    /// The Note with this id, that of a Routine a newer version saved.
    OpenNote(String),
    /// The editor of a new Session, its Note made behind the scenes.
    NewSession,
    /// One calendar file of every Routine and Reminder listed.
    Export,
}

/// A Routine or Reminder as the page lists it: with its name and its
/// Note's Area.
pub type Row = (Routine, String, Option<Area>);

/// A Routine or Reminder whose schedule a newer version of the app saved
/// (ADR 0034), as the page lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct NewerRow {
    pub id: String,
    /// The Note it shows under, which a click opens.
    pub note: String,
    pub name: String,
    pub area: Option<Area>,
}

/// The name a Routine goes by, on the page, in its order and in the
/// calendar file: its Note's text, a Note without words by what it holds,
/// or that the Note is gone; the phone's `noteText` and `noteGone`.
pub fn routine_name(t: &L10n, note: Option<&Note>) -> String {
    match note {
        Some(note) if note.text.trim().is_empty() => l10n::wordless(t, note.wordless()).to_string(),
        Some(note) => note.visible_text(),
        None => t.note_gone().to_string(),
    }
}

/// The name of each of `list` by its id, as [`routine_name`] gives it.
pub fn names(
    t: &L10n,
    routines: &Routines,
    list: &[Routine],
) -> oldbrainfix_core::Result<HashMap<String, String>> {
    let mut names = HashMap::new();
    for routine in list {
        let name = routine_name(t, routines.note(routine)?.as_ref());
        names.insert(routine.id.clone(), name);
    }
    Ok(names)
}

/// The page: its heading with New Session and Export, and each of `rows`, a Routine with
/// its name and its Note's Area, as a card with its schedule, the Area's
/// colour down its side and its icon at the end, then each of `newer`
/// saying a newer version is needed; a plain line while there is none.
/// An Unsorted one has neither.
pub fn page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (rows, newer): (&[Row], &[NewerRow]),
    actions: &mut Vec<AllRoutinesAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.horizontal(|ui| {
        ui.heading(RichText::new(t.all_routines_title()).color(text));
        // Export in the corner, New Session before it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let export = ui.add_enabled_ui(!rows.is_empty(), |ui| {
                icon_button(ui, icons::EXPORT, t.export_calendar_file(), text)
            });
            if export.inner.clicked() {
                actions.push(AllRoutinesAction::Export);
            }
            if ui
                .button(RichText::new(t.new_session()).color(text))
                .clicked()
            {
                actions.push(AllRoutinesAction::NewSession);
            }
        });
    });
    ui.add_space(8.0);
    if rows.is_empty() && newer.is_empty() {
        ui.label(RichText::new(t.all_routines_empty()).color(weak));
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("all routines")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add_space(4.0);
            for (routine, name, area) in rows {
                let icon = if routine.is_reminder() {
                    icons::REMINDER
                } else {
                    icons::REPEAT
                };
                let when = schedule_text(t, &routine.schedule);
                if card(ui, tones, (&routine.id, name, area.as_ref()), icon, &when) {
                    actions.push(AllRoutinesAction::Open(routine.id.clone()));
                }
            }
            for row in newer {
                let when = t.newer_version_needed();
                if card(
                    ui,
                    tones,
                    (&row.id, &row.name, row.area.as_ref()),
                    icons::HISTORY,
                    when,
                ) {
                    actions.push(AllRoutinesAction::OpenNote(row.note.clone()));
                }
            }
        });
}

/// One Routine or Reminder as a card: `icon` for its kind, its name and
/// `when` below it, its Area's colour down its side and its icon at the
/// end. Whether it was clicked.
fn card(
    ui: &mut Ui,
    tones: Tones,
    (routine, name, area): (&str, &str, Option<&Area>),
    icon: char,
    when: &str,
) -> bool {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let first = name.trim().lines().next().unwrap_or_default();
    let id = Id::new(("all routines", routine));
    let colour = area.and_then(|a| area_colour(a.colour.as_deref()));
    banded_clickable_card(ui, tones, (id, first), colour, |ui| {
        ui.horizontal(|ui| {
            icons::show(ui, icon, 20.0, weak);
            let width = (ui.available_width() - 32.0).max(40.0);
            ui.vertical(|ui| {
                ui.set_max_width(width);
                ui.add(egui::Label::new(RichText::new(first).size(15.0).color(text)).truncate());
                ui.label(RichText::new(when).small().color(weak));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(icon) = area.and_then(|a| icons::area_icon(a.icon.as_deref())) {
                    icons::show(ui, icon, 20.0, colour.unwrap_or(weak));
                }
            });
        });
    })
}
