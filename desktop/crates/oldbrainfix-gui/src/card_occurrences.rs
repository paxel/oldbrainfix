//! Done from the Note and from its cards (#215), as the phone's
//! `card_occurrences.dart` and the top of its Note editor have it: on a
//! Note's card, in either view, a box for today's Occurrence of each of
//! its Routines and Reminders that ticks it as the Agenda row's does; at
//! the top of the Note dialog, over its name, the Occurrence the Note
//! asks to check off, "Due 15:00" or "Missed" with its day, with Done,
//! Snooze and Not today (#269), and for a one-time Reminder, the Note's
//! only one, "Done and move to trash".

use std::collections::HashMap;

use egui::{Id, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::{
    AgendaRow, Checklists, Day, Occurrence, OpenOccurrence, Result, RoutineOccurrence, Sessions,
    Store,
};

use crate::checklist_view::paint_box;
use crate::l10n::L10n;
use crate::occurrence_answers::{self, Answer, Answers};
use crate::schedule_text::{day_text, occurrence_time};
use crate::theme::{self, Tones};

/// Today's Occurrence of a Routine or Reminder on its Note's card, and
/// whether its box ticks there.
#[derive(Debug, Clone, PartialEq)]
pub struct CardOccurrence {
    pub row: AgendaRow,
    pub checks: bool,
}

/// What the cards of the Notes `listed` tick today, by Note
/// ([`oldbrainfix_core::Routines::today_by_note`], read once for the list):
/// a box ticks as on the Agenda, not a Session's, not one its Lines made
/// Done, and only where this Device checks it off.
pub fn read(
    store: &Store,
    today: Vec<(String, Vec<AgendaRow>)>,
    listed: &[String],
) -> Result<HashMap<String, Vec<CardOccurrence>>> {
    let mut out = HashMap::new();
    for (note, rows) in today {
        if !listed.contains(&note) {
            continue;
        }
        let mut shown = Vec::new();
        for row in rows {
            let checks = checks(store, &row)?;
            shown.push(CardOccurrence { row, checks });
        }
        out.insert(note, shown);
    }
    Ok(out)
}

fn checks(store: &Store, row: &AgendaRow) -> Result<bool> {
    let id = &row.routine.id;
    if !store.may_check_off(id)? || Sessions::new(store).by_id(id)?.is_some() {
        return Ok(false);
    }
    let on = RoutineOccurrence {
        routine_id: id.clone(),
        occurrence: row.occurrence,
    };
    let lines = Checklists::new(store).for_occurrence(&on)?;
    Ok(!lines.is_some_and(|l| l.count() > 0 && l.all_checked()))
}

/// Paints the boxes of `shown` on a card, each with its time, struck
/// through once Done or skipped, a skipped one saying so, "08:00 ·
/// Skipped" (#269): the card lies over them, so [`card_clicks`] takes
/// their clicks once the card is. Where each box lies.
pub fn card_rows(ui: &mut Ui, t: &L10n, tones: Tones, shown: &[CardOccurrence]) -> Vec<egui::Rect> {
    let mut rects = Vec::new();
    for o in shown {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
            let done = o.row.done().is_some();
            let skipped = !done && o.row.skipped().is_some();
            paint_box(ui, tones, rect, done, false);
            let mut words = occurrence_time(t, &o.row.occurrence);
            if skipped {
                words = format!("{words} · {}", t.occurrence_skipped());
            }
            let mut time = RichText::new(words)
                .size(14.0)
                .color(ui.visuals().text_color());
            if done || skipped {
                time = time.strikethrough();
            }
            ui.label(time);
            rects.push(rect);
        });
    }
    rects
}

/// Takes the clicks on the boxes of `shown` on the card of the Note
/// `note`, painted by [`card_rows`] at `rects`: the Routine and the
/// Occurrence to check off, or to uncheck when it is Done.
pub fn card_clicks(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    note: &str,
    shown: &[CardOccurrence],
    rects: &[egui::Rect],
) -> Option<(String, Occurrence)> {
    let mut clicked = None;
    for (o, rect) in shown.iter().zip(rects) {
        let key = o.row.occurrence.key();
        let id = Id::new(("card occurrence", note, &o.row.routine.id, &key));
        let sense = if o.checks {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resp = ui.interact(*rect, id, sense);
        let done = o.row.done().is_some();
        let label = format!(
            "{}: {}",
            t.mark_done(),
            occurrence_time(t, &o.row.occurrence)
        );
        resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, o.checks, done, &label));
        if o.checks && resp.hovered() && ui.is_rect_visible(*rect) {
            paint_box(ui, tones, *rect, done, true);
        }
        if resp.clicked() {
            clicked = Some((o.row.routine.id.clone(), o.row.occurrence));
        }
        if o.checks {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
    clicked
}

/// What the Note dialog's line of `open` asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAsked {
    /// Check it off.
    Done(String, Occurrence),
    /// Check it off and move the Note with its one Reminder to the trash.
    DoneAndTrash(String, Occurrence),
    /// Snooze it, skip it or move it to tomorrow (#269).
    Answer(String, Occurrence, Answer),
}

/// What the line of `open` says: "Due 15:00", with its day when it is
/// not `today`'s, or "Missed" with its day and time.
pub fn open_text(t: &L10n, open: &OpenOccurrence, today: Day) -> String {
    let time = occurrence_time(t, &open.occurrence);
    let day = open.occurrence.day;
    match (open.missed, day == today) {
        (true, _) => t.occurrence_missed_at(&format!("{} {time}", day_text(t, day))),
        (false, true) => t.occurrence_due(&time),
        (false, false) => t.occurrence_due(&format!("{} {time}", day_text(t, day))),
    }
}

/// The lines at the top of the Note dialog, over its name (#215): of
/// each Occurrence in `open` its text and Done; Snooze while it is not
/// Missed and Not today (#269), each opening its choices under it,
/// Tomorrow only for an Occurrence of today or later; and "Done and move
/// to trash" for a one-time Reminder where `trashes` allows it: the
/// Note's only Routine or Reminder, the Note this Device's to move.
pub fn open_lines(
    ui: &mut Ui,
    t: &L10n,
    open: &[OpenOccurrence],
    (today, trashes): (Day, bool),
) -> Option<OpenAsked> {
    let mut asked = None;
    for o in open {
        ui.horizontal_wrapped(|ui| {
            let text = ui.visuals().text_color();
            ui.label(RichText::new(open_text(t, o, today)).size(16.0).color(text));
            let id = o.routine.id.clone();
            if theme::primary(ui, true, t.mark_done()).clicked() {
                asked = Some(OpenAsked::Done(id.clone(), o.occurrence));
            }
            // Snooze rings from now on: not for one Missed.
            let snooze = Answers {
                snooze: true,
                ..Answers::default()
            };
            let not_today = Answers {
                not_today: true,
                tomorrow: o.occurrence.day >= today,
                ..Answers::default()
            };
            let mut picked = None;
            for (shown, label, answers) in [
                (!o.missed, t.snooze(), snooze),
                (true, t.not_today(), not_today),
            ] {
                if !shown {
                    continue;
                }
                let button = ui.button(RichText::new(label).color(text));
                egui::Popup::menu(&button).show(|ui| {
                    picked = picked.or(occurrence_answers::entries(ui, t, answers));
                });
            }
            if let Some(answer) = picked {
                asked = Some(OpenAsked::Answer(id.clone(), o.occurrence, answer));
            }
            if trashes
                && o.routine.is_reminder()
                && theme::destructive(ui, true, t.done_and_trash()).clicked()
            {
                asked = Some(OpenAsked::DoneAndTrash(id, o.occurrence));
            }
        });
    }
    asked
}
