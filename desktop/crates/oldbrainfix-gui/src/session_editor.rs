//! The Session's part of the Routine editor (#104, ADR 0029), as the
//! phone's `session_editor.dart` draws it, on top of the schedule: the
//! pool as a checkbox per Tally, ordered with up and down buttons for
//! Rolling; the count; the weekdays' own counts in a fold, open as last
//! left on this Device; and the rule as radio buttons. Nothing is written
//! before Save, and then only what changed. For a Note being captured
//! (#113) Save hands the draft back as a [`SessionChoice`].

use std::collections::{BTreeMap, BTreeSet};

use egui::{Id, RichText, TextEdit, Ui};
use oldbrainfix_core::sessions::keys::MAX_COUNT;
use oldbrainfix_core::{Notes, Schedule, Session, SessionRule, Sessions, Store};

use crate::app::{embossed, icon_button, row_title};
use crate::capture_choices::SessionChoice;
use crate::icons;
use crate::l10n::L10n;
use crate::schedule_text::weekday_long;
use crate::theme::Tones;

/// The local setting that keeps the weekdays' counts open, the phone's.
pub const WEEKDAYS_OPEN: &str = "sessionWeekdaysOpen";

/// What the user asked of the Session's part beyond typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIntent {
    /// Keeps whether the weekdays' counts are open.
    WeekdaysOpen(bool),
}

/// What the Routine editor holds of a Session while it is edited: the
/// Tallies of its pool picker, in the order Rolling goes through them,
/// those chosen, the count, the weekdays' own counts and the rule.
#[derive(Debug, Clone)]
pub struct SessionDraft {
    /// The Tallies the pool picker shows, the chosen ones first in pool
    /// order.
    pub order: Vec<String>,
    pub chosen: BTreeSet<String>,
    /// What each Tally of [`SessionDraft::order`] shows as.
    names: BTreeMap<String, String>,
    pub rule: SessionRule,
    /// The count as typed.
    pub count: String,
    /// The count typed for each weekday, Monday first; empty for none.
    pub counts: [String; 7],
    /// Whether the weekdays' counts are open.
    pub weekdays_open: bool,
    /// Set by a Save that found something to fix: the pool's error shows
    /// from then on.
    pub checked: bool,
}

impl SessionDraft {
    /// The draft of `session`, or of a new Session spawned from the Note
    /// `entity`: its pool as held, then the other Tallies it may pick.
    pub fn of(
        store: &Store,
        t: &L10n,
        entity: &str,
        session: Option<&Session>,
    ) -> oldbrainfix_core::Result<SessionDraft> {
        let held = session.map(|s| s.pool.clone()).unwrap_or_default();
        let mut order = held.clone();
        for id in Sessions::new(store).choices(entity)? {
            if !order.contains(&id) {
                order.push(id);
            }
        }
        let notes = Notes::new(store);
        let mut names = BTreeMap::new();
        for id in &order {
            let note = notes.by_id(id)?;
            names.insert(id.clone(), row_title(t, note.as_ref()));
        }
        let counts = session.map(|s| s.counts.clone()).unwrap_or_default();
        Ok(SessionDraft {
            chosen: held.into_iter().collect(),
            order,
            names,
            rule: session.map_or(SessionRule::Rolling, |s| s.rule),
            count: session.map_or(1, |s| s.count).to_string(),
            counts: std::array::from_fn(|i| {
                let day = i as u32 + 1;
                counts.get(&day).map(i64::to_string).unwrap_or_default()
            }),
            weekdays_open: store.local_setting(WEEKDAYS_OPEN)?.as_deref() == Some("true"),
            checked: false,
        })
    }

    /// The chosen Tallies in their order.
    pub fn pool(&self) -> Vec<String> {
        self.order
            .iter()
            .filter(|id| self.chosen.contains(*id))
            .cloned()
            .collect()
    }

    /// `text` as a count, 1 to 99, as the core reads one: one or two
    /// digits; `None` when it is none.
    pub fn count_of(text: &str) -> Option<i64> {
        let typed = text.trim();
        if typed.is_empty() || typed.len() > 2 || !typed.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        typed
            .parse::<i64>()
            .ok()
            .filter(|c| (1..=MAX_COUNT).contains(c))
    }

    pub fn count_valid(&self) -> bool {
        Self::count_of(&self.count).is_some()
    }

    /// Whether the count of the weekday at `index` (0 for Monday) is
    /// empty or a count.
    pub fn weekday_valid(&self, index: usize) -> bool {
        let text = self.counts[index].trim();
        text.is_empty() || Self::count_of(text).is_some()
    }

    pub fn weekdays_valid(&self) -> bool {
        (0..7).all(|i| self.weekday_valid(i))
    }

    /// How many weekdays have their own count typed.
    pub fn own_counts(&self) -> i64 {
        self.counts
            .iter()
            .filter(|c| Self::count_of(c).is_some())
            .count() as i64
    }

    pub fn valid(&self) -> bool {
        !self.pool().is_empty() && self.count_valid() && self.weekdays_valid()
    }

    /// Marks the draft as checked by a Save: the pool's error shows, and
    /// a weekday to fix comes into view.
    pub fn check(&mut self) {
        self.checked = true;
        if !self.weekdays_valid() {
            self.weekdays_open = true;
        }
    }

    /// What the draft holds as plain values, kept after the editor is
    /// gone; read while it is [`SessionDraft::valid`].
    pub fn choice(&self) -> SessionChoice {
        let counts = self
            .counts
            .iter()
            .enumerate()
            .filter_map(|(i, typed)| Some((i as u32 + 1, Self::count_of(typed)?)))
            .collect();
        SessionChoice {
            pool: self.pool(),
            count: Self::count_of(&self.count).unwrap_or(1),
            counts,
            rule: self.rule,
        }
    }

    /// Spawns the Session from the Note `note_id` on `schedule`; its id.
    pub fn spawn(
        &self,
        store: &Store,
        note_id: &str,
        schedule: &Schedule,
    ) -> oldbrainfix_core::Result<String> {
        self.choice().spawn(store, note_id, schedule)
    }

    /// Writes what changed of `session`.
    pub fn save(&self, store: &Store, session: &Session) -> oldbrainfix_core::Result<()> {
        let sessions = Sessions::new(store);
        let id = session.id();
        let pool = self.pool();
        if pool != session.pool {
            sessions.set_pool(id, &pool)?;
        }
        let count = Self::count_of(&self.count).unwrap_or(session.count);
        if count != session.count {
            sessions.set_count(id, count)?;
        }
        for (i, typed) in self.counts.iter().enumerate() {
            let day = i as u32 + 1;
            let own = Self::count_of(typed);
            if own != session.counts.get(&day).copied() {
                sessions.set_count_on(id, day, own)?;
            }
        }
        if self.rule != session.rule {
            sessions.set_rule(id, self.rule)?;
        }
        Ok(())
    }

    /// Draws the Session's panels; what was asked beyond typing.
    pub fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<SessionIntent> {
        let mut intent = None;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let error = ui.visuals().error_fg_color;
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.session_pool()).strong().color(text));
            ui.label(RichText::new(t.session_pool_explain()).color(text));
            if self.order.is_empty() {
                ui.label(RichText::new(t.session_pool_none_desk()).color(weak));
            } else {
                self.pool_rows(ui, t);
            }
            if self.rule == SessionRule::Rolling && self.order.len() > 1 {
                ui.label(
                    RichText::new(t.session_pool_order_desk())
                        .small()
                        .color(weak),
                );
            }
            if self.checked && self.pool().is_empty() {
                ui.label(RichText::new(t.session_pool_missing()).color(error));
            }
        });
        embossed(ui, tones, false, None, |ui| {
            let label = ui.label(RichText::new(t.session_count()).strong().color(text));
            ui.add(
                TextEdit::singleline(&mut self.count)
                    .id(Id::new("session count"))
                    .text_color(text)
                    .desired_width(60.0),
            )
            .labelled_by(label.id);
            if !self.count_valid() {
                ui.label(RichText::new(t.session_count_invalid()).color(error));
            }
        });
        embossed(ui, tones, false, None, |ui| {
            let open = self.weekdays_open;
            let fold =
                egui::CollapsingHeader::new(RichText::new(t.session_weekday_counts()).color(text))
                    .id_salt("session weekdays")
                    .open(Some(open))
                    .show(ui, |ui| {
                        ui.label(RichText::new(t.session_weekday_counts_explain()).color(text));
                        for i in 0..7 {
                            self.weekday_row(ui, t, i);
                        }
                    });
            if fold.header_response.clicked() {
                self.weekdays_open = !open;
                intent = Some(SessionIntent::WeekdaysOpen(!open));
            }
            if !open {
                if self.weekdays_valid() {
                    let summary = t.session_weekday_counts_summary(self.own_counts());
                    ui.label(RichText::new(summary).small().color(weak));
                } else {
                    ui.label(RichText::new(t.session_count_invalid()).color(error));
                }
            }
        });
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.session_rule()).strong().color(text));
            let rules = [
                (
                    SessionRule::Random,
                    t.session_rule_random(),
                    t.session_rule_random_detail(),
                ),
                (
                    SessionRule::Rolling,
                    t.session_rule_rolling(),
                    t.session_rule_rolling_detail(),
                ),
                (
                    SessionRule::Least,
                    t.session_rule_least(),
                    t.session_rule_least_detail(),
                ),
            ];
            for (rule, title, detail) in rules {
                ui.radio_value(&mut self.rule, rule, RichText::new(title).color(text));
                ui.indent(("session rule", rule.name()), |ui| {
                    ui.label(RichText::new(detail).small().color(weak));
                });
            }
        });
        intent
    }

    /// A checkbox per Tally; for Rolling each with buttons to move it up
    /// or down, a drag's place on a desk without a long press.
    fn pool_rows(&mut self, ui: &mut Ui, t: &L10n) {
        let text = ui.visuals().text_color();
        let rolling = self.rule == SessionRule::Rolling;
        let last = self.order.len() - 1;
        let mut moved = None;
        for (i, id) in self.order.iter().enumerate() {
            let name = self.names.get(id).cloned().unwrap_or_default();
            ui.horizontal(|ui| {
                let mut on = self.chosen.contains(id);
                if ui
                    .checkbox(&mut on, RichText::new(&name).color(text))
                    .changed()
                {
                    if on {
                        self.chosen.insert(id.clone());
                    } else {
                        self.chosen.remove(id);
                    }
                }
                if !rolling {
                    return;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if i < last
                        && icon_button(ui, icons::ARROW_DOWN, &t.session_move_down(&name), text)
                            .clicked()
                    {
                        moved = Some((i, i + 1));
                    }
                    if i > 0
                        && icon_button(ui, icons::ARROW_UP, &t.session_move_up(&name), text)
                            .clicked()
                    {
                        moved = Some((i, i - 1));
                    }
                });
            });
        }
        if let Some((from, to)) = moved {
            self.order.swap(from, to);
        }
    }

    /// The count of the weekday at `index`, 0 for Monday: empty for as
    /// many as every day.
    fn weekday_row(&mut self, ui: &mut Ui, t: &L10n, index: usize) {
        let text = ui.visuals().text_color();
        let valid = self.weekday_valid(index);
        ui.horizontal(|ui| {
            let day = weekday_long(t, index as u32 + 1);
            let label = ui.add_sized(
                [110.0, 20.0],
                egui::Label::new(RichText::new(day).color(text)),
            );
            ui.add(
                TextEdit::singleline(&mut self.counts[index])
                    .id(Id::new(("session weekday count", index)))
                    .hint_text(t.session_weekday_count_hint())
                    .text_color(text)
                    .desired_width(110.0),
            )
            .labelled_by(label.id);
        });
        if !valid {
            ui.label(RichText::new(t.session_count_invalid()).color(ui.visuals().error_fg_color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_is_one_or_two_digits_from_1_to_99() {
        assert_eq!(SessionDraft::count_of("3"), Some(3));
        assert_eq!(SessionDraft::count_of(" 12 "), Some(12));
        assert_eq!(SessionDraft::count_of("99"), Some(99));
        for wrong in ["", "0", "00", "100", "-1", "+1", "1.5", "x"] {
            assert_eq!(SessionDraft::count_of(wrong), None, "{wrong:?}");
        }
    }
}
