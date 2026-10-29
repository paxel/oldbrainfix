//! A Session's part of its Agenda row (#104, ADR 0029), as the phone's
//! `session_view.dart` draws it: today its picks under the title, in the
//! order picked, each a line with its targets, struck through once it
//! has an entry, a click opening its entry form, and under each without
//! an entry "Another one"; on later days how many it picks.

use egui::{Id, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::{Occurrence, SessionOccurrence, SessionPick, Store, UnitSystem};

use crate::app::row_title;
use crate::icons;
use crate::l10n::L10n;
use crate::tally_view::{decimal_sign, shown_value};

/// Today's Occurrence of a Session as its row shows it: the text of each
/// pick, whether each opens its entry form here, and whether they swap.
#[derive(Debug, Clone)]
pub struct PicksShown {
    pub today: SessionOccurrence,
    pub texts: Vec<String>,
    pub opens: Vec<bool>,
    pub swaps: bool,
}

impl PicksShown {
    /// `today` as this Device shows it, in the unit system `system`.
    pub fn read(
        store: &Store,
        t: &L10n,
        today: SessionOccurrence,
        system: UnitSystem,
    ) -> oldbrainfix_core::Result<PicksShown> {
        let id = today.session.id().to_string();
        let mut opens = Vec::new();
        for pick in &today.picks {
            opens.push(opens_pick(store, &id, pick)?);
        }
        Ok(PicksShown {
            texts: today
                .picks
                .iter()
                .map(|p| pick_text(t, p, system))
                .collect(),
            swaps: today.can_swap && store.may_check_off(&id)?,
            opens,
            today,
        })
    }
}

/// What shows under a Session's title on the Agenda.
#[derive(Debug, Clone)]
pub enum SessionPart {
    /// Today: its picks.
    Today(Box<PicksShown>),
    /// A later day: how many it picks.
    Later(i64),
}

/// What the user asked of a Session's row.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionAction {
    /// Opens the entry form of a pick: a new entry recorded for the
    /// Occurrence, or the pick's latest entry to change.
    Open {
        session: String,
        occurrence: Occurrence,
        note_id: String,
        entry: Option<String>,
    },
    /// "Another one": swaps the Tally of this Note for the next.
    Another {
        session: String,
        occurrence: Occurrence,
        note_id: String,
    },
}

/// A picked Tally as its line shows: its Note's text and the targets of
/// its fields, each with the field's name when it has more than one
/// ("Push-ups 20 times").
pub fn pick_text(t: &L10n, pick: &SessionPick, system: UnitSystem) -> String {
    let name = row_title(t, pick.note.as_ref());
    let decimal = decimal_sign(t);
    let aimed: Vec<_> = pick
        .tally
        .as_ref()
        .map(|tally| tally.shown())
        .unwrap_or_default()
        .into_iter()
        .filter(|field| field.target.is_some())
        .collect();
    let targets: Vec<String> = aimed
        .iter()
        .filter_map(|field| {
            let shown = shown_value(field.target?, &field.unit, system, decimal);
            Some(if aimed.len() > 1 {
                format!("{} {shown}", field.name)
            } else {
                shown
            })
        })
        .collect();
    if targets.is_empty() {
        name
    } else {
        format!("{name} {}", targets.join(" · "))
    }
}

/// Whether this Device may open the entry form of `pick` of the Session
/// `session`: record for it, or change the entry it has.
pub fn opens_pick(
    store: &Store,
    session: &str,
    pick: &SessionPick,
) -> oldbrainfix_core::Result<bool> {
    let gone = pick.note.as_ref().is_none_or(|n| n.is_trashed()) || pick.tally.is_none();
    if gone {
        return Ok(false);
    }
    match pick.entries.last() {
        Some(entry) => store.may_change(&entry.id),
        None => Ok(store.may_check_off(session)? && store.may_write_in(&pick.note_id)?),
    }
}

/// Draws `part` under a Session's title.
pub fn part(ui: &mut Ui, t: &L10n, part: &SessionPart, actions: &mut Vec<SessionAction>) {
    let text = ui.visuals().text_color();
    match part {
        SessionPart::Later(count) => {
            ui.label(RichText::new(t.session_picks(*count)).color(text));
        }
        SessionPart::Today(shown) => picks(ui, t, shown, actions),
    }
}

/// Today's picks, each a line opening its entry form, and "Another one"
/// under each without an entry (the control goes below its text).
fn picks(ui: &mut Ui, t: &L10n, shown: &PicksShown, actions: &mut Vec<SessionAction>) {
    let text = ui.visuals().text_color();
    let today = &shown.today;
    if today.picks.is_empty() {
        ui.label(RichText::new(t.session_nothing_to_pick()).color(text));
        return;
    }
    let session = today.session.id();
    let occurrence = today.occurrence;
    ui.spacing_mut().item_spacing.y = 4.0;
    for (i, pick) in today.picks.iter().enumerate() {
        let words = shown.texts.get(i).map(String::as_str).unwrap_or_default();
        let opens = shown.opens.get(i).copied().unwrap_or(false);
        let line = ui.horizontal(|ui| {
            ui.style_mut().interaction.selectable_labels = false;
            let icon = if pick.done() {
                icons::PICK_DONE
            } else {
                icons::PICK_OPEN
            };
            icons::show(ui, icon, 18.0, text);
            let mut shown = RichText::new(words).color(text);
            if pick.done() {
                shown = shown.strikethrough();
            }
            ui.add(egui::Label::new(shown).wrap());
        });
        let id = Id::new(("session pick", session, occurrence.key(), &pick.note_id));
        let sense = if opens {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut open = ui.interact(line.response.rect, id, sense);
        open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, opens, words));
        if opens {
            open = open.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        if open.clicked() {
            actions.push(SessionAction::Open {
                session: session.to_string(),
                occurrence,
                note_id: pick.note_id.clone(),
                entry: pick.entries.last().map(|e| e.id.clone()),
            });
        }
        if shown.swaps && !pick.done() {
            let another = ui.horizontal(|ui| {
                ui.add_space(26.0);
                icons::show(ui, icons::SWAP, 18.0, text);
                ui.button(RichText::new(t.session_another()).color(text))
            });
            if another.inner.clicked() {
                actions.push(SessionAction::Another {
                    session: session.to_string(),
                    occurrence,
                    note_id: pick.note_id.clone(),
                });
            }
        }
    }
}
