//! The All Routines and Reminders page (#115): every Routine and
//! Reminder in alphabetical order of its name, also those beyond the
//! Agenda's week; a click opens its editor, and Export saves one
//! calendar file of all of them, as the phone's `AllRoutinesScreen`.

use std::collections::HashMap;

use egui::{Id, RichText, Ui};
use oldbrainfix_core::{Note, Routine, Routines};

use crate::app::{clickable_card, icon_button};
use crate::icons;
use crate::l10n::{self, L10n};
use crate::schedule_text::schedule_text;
use crate::theme::Tones;

/// What the page asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllRoutinesAction {
    /// The editor of the Routine with this id, on the right.
    Open(String),
    /// One calendar file of every Routine and Reminder listed.
    Export,
}

/// The name a Routine goes by, on the page, in its order and in the
/// calendar file: its Note's text, a Note without words by what it holds,
/// or that the Note is gone; the phone's `noteText` and `noteGone`.
pub fn routine_name(t: &L10n, note: Option<&Note>) -> String {
    match note {
        Some(note) if note.text.trim().is_empty() => l10n::wordless(t, note.wordless()).to_string(),
        Some(note) => note.text.clone(),
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

/// The page: its heading with Export, and each of `rows`, a Routine with
/// its name, as a card with its schedule; a plain line while there is
/// none.
pub fn page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    rows: &[(Routine, String)],
    actions: &mut Vec<AllRoutinesAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.horizontal(|ui| {
        ui.heading(RichText::new(t.all_routines_title()).color(text));
        ui.add_space((ui.available_width() - 28.0).max(0.0));
        let export = ui.add_enabled_ui(!rows.is_empty(), |ui| {
            icon_button(ui, icons::EXPORT, t.export_calendar_file(), text)
        });
        if export.inner.clicked() {
            actions.push(AllRoutinesAction::Export);
        }
    });
    ui.add_space(8.0);
    if rows.is_empty() {
        ui.label(RichText::new(t.all_routines_empty()).color(weak));
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("all routines")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add_space(4.0);
            for (routine, name) in rows {
                let first = name.trim().lines().next().unwrap_or_default();
                let id = Id::new(("all routines", &routine.id));
                let clicked = clickable_card(ui, tones, (id, first), |ui| {
                    ui.horizontal(|ui| {
                        let icon = if routine.is_reminder() {
                            icons::REMINDER
                        } else {
                            icons::REPEAT
                        };
                        icons::show(ui, icon, 20.0, weak);
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Label::new(RichText::new(first).size(15.0).color(text))
                                    .truncate(),
                            );
                            let when = schedule_text(t, &routine.schedule);
                            ui.label(RichText::new(when).small().color(weak));
                        });
                    });
                });
                if clicked {
                    actions.push(AllRoutinesAction::Open(routine.id.clone()));
                }
            }
        });
}
