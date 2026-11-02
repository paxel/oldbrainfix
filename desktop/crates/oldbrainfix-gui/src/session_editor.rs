//! The Session's part of the Routine editor (#104, ADR 0029), as the
//! phone's `session_editor.dart` draws it, on top of the schedule: the
//! pool as a checkbox per Tally, filtered by the pills of its Tallies'
//! tags (#220), ordered with up and down buttons for Rolling while it is
//! not filtered; the count's − / + stepper; the weekdays in a fold, each with
//! a checkbox for its own count and then its stepper, open as last left
//! on this Device; and the rule as radio buttons. Nothing is written
//! before Save. For a Note being captured (#113) Save hands the draft
//! back as a [`SessionChoice`]. On a Session Note's page in the Note
//! dialog (#191) the pool and the count show the same way, each change
//! written at once, only what changed, and the rule and the weekdays'
//! counts are badges whose dialogs draw [`SessionDraft::rules`] and
//! [`SessionDraft::weekdays`].

use std::collections::{BTreeMap, BTreeSet};

use egui::{RichText, Ui};
use oldbrainfix_core::sessions::keys::MAX_COUNT;
use oldbrainfix_core::{Notes, Schedule, SessionRule, SessionSetup, Sessions, Store, text_tags};

use crate::app::{embossed, icon_button, pill, row_title};
use crate::capture_choices::SessionChoice;
use crate::icons;
use crate::l10n::L10n;
use crate::routine_editor::stepper;
use crate::schedule_text::weekday_long;
use crate::theme::{self, Tones};

/// The local setting that keeps the weekdays' counts open, the phone's.
pub const WEEKDAYS_OPEN: &str = "sessionWeekdaysOpen";

/// What the user asked of the Session's part beyond typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIntent {
    /// Keeps whether the weekdays' counts are open.
    WeekdaysOpen(bool),
    /// "New Tally" at the end of the pool (#161): the New Tally page,
    /// whose Tally comes back ticked.
    NewTally,
}

/// What the Routine editor holds of a Session while it is edited: the
/// Tallies of its pool picker, in the order Rolling goes through them,
/// those chosen, the count, the weekdays' own counts and the rule.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionDraft {
    /// The Tallies the pool picker shows, the chosen ones first in pool
    /// order.
    pub order: Vec<String>,
    pub chosen: BTreeSet<String>,
    /// What each Tally of [`SessionDraft::order`] shows as.
    names: BTreeMap<String, String>,
    /// The tags in the text of each Tally of [`SessionDraft::order`]
    /// (ADR 0032).
    tags: BTreeMap<String, BTreeSet<String>>,
    /// The tags picked to filter the pool by (#220), a view only: it
    /// shows the Tallies carrying any of them. Kept while the page is
    /// open, never stored.
    pub picked: BTreeSet<String>,
    pub rule: SessionRule,
    /// How many it picks each time, 1 to 99.
    pub count: i64,
    /// The own count of each weekday that has one, Monday first, 1 to 99.
    pub counts: [Option<i64>; 7],
    /// Whether the weekdays' counts are open.
    pub weekdays_open: bool,
    /// Set by a Save that found something to fix: the pool's error shows
    /// from then on.
    pub checked: bool,
    /// Whether the pool ends in "New Tally", as on the New Session page
    /// (#161).
    pub new_tally: bool,
}

impl SessionDraft {
    /// The draft of the Session `setup` of the Note `entity` (#191), or
    /// of a new Session spawned from the Note `entity`: its pool as held,
    /// then the other Tallies it may pick.
    pub fn of(
        store: &Store,
        t: &L10n,
        entity: &str,
        setup: Option<&SessionSetup>,
    ) -> oldbrainfix_core::Result<SessionDraft> {
        let held = setup.map(|s| s.pool.clone()).unwrap_or_default();
        let mut order = held.clone();
        for id in Sessions::new(store).choices(entity)? {
            if !order.contains(&id) {
                order.push(id);
            }
        }
        let notes = Notes::new(store);
        let mut names = BTreeMap::new();
        let mut tags = BTreeMap::new();
        for id in &order {
            let note = notes.by_id(id)?;
            names.insert(id.clone(), row_title(t, note.as_ref()));
            tags.insert(id.clone(), tags_of(note.map(|n| n.text).as_deref()));
        }
        let counts = setup.map(|s| s.counts.clone()).unwrap_or_default();
        Ok(SessionDraft {
            chosen: held.into_iter().collect(),
            order,
            names,
            tags,
            picked: BTreeSet::new(),
            rule: setup.map_or(SessionRule::Rolling, |s| s.rule),
            count: setup.map_or(1, |s| s.count),
            counts: std::array::from_fn(|i| counts.get(&(i as u32 + 1)).copied()),
            weekdays_open: store.local_setting(WEEKDAYS_OPEN)?.as_deref() == Some("true"),
            checked: false,
            new_tally: false,
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

    /// Gives the weekday at `index` (0 for Monday) its own count,
    /// starting from every day's, or takes it away when `own` is false.
    pub fn set_own(&mut self, index: usize, own: bool) {
        self.counts[index] = own.then_some(self.count);
    }

    /// How many weekdays have their own count.
    pub fn own_counts(&self) -> i64 {
        self.counts.iter().flatten().count() as i64
    }

    /// Puts the Tally `id`, shown as `name`, its text, just made, first
    /// in the picker, ticked.
    pub fn add_made(&mut self, id: &str, name: &str) {
        self.order.retain(|held| held != id);
        self.order.insert(0, id.to_string());
        self.chosen.insert(id.to_string());
        self.names.insert(id.to_string(), name.to_string());
        self.tags.insert(id.to_string(), tags_of(Some(name)));
    }

    /// The tags of the Tallies of [`SessionDraft::order`], those most of
    /// them carry first, then by name.
    pub fn tag_list(&self) -> Vec<String> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for id in &self.order {
            for tag in self.tags.get(id).into_iter().flatten() {
                *counts.entry(tag).or_default() += 1;
            }
        }
        let mut tags: Vec<(&str, usize)> = counts.into_iter().collect();
        // Stable, so ties stay by name.
        tags.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        tags.into_iter().map(|(tag, _)| tag.to_string()).collect()
    }

    /// The tags [`SessionDraft::picked`] that a Tally still carries, in
    /// the order of [`SessionDraft::tag_list`].
    pub fn filter(&self) -> Vec<String> {
        self.tag_list()
            .into_iter()
            .filter(|tag| self.picked.contains(tag))
            .collect()
    }

    /// Whether the Tally `id` shows under `filter`: with no tag in it
    /// always, else when it carries any of them.
    fn shows(&self, id: &str, filter: &[String]) -> bool {
        filter.is_empty()
            || self
                .tags
                .get(id)
                .is_some_and(|tags| filter.iter().any(|tag| tags.contains(tag)))
    }

    /// How many Tallies ticked the filter hides.
    pub fn hidden_ticked(&self) -> usize {
        let filter = self.filter();
        self.order
            .iter()
            .filter(|id| self.chosen.contains(*id) && !self.shows(id, &filter))
            .count()
    }

    /// What a Tally made from the filtered pool starts its name with: the
    /// filter's tags, each followed by a space; empty without one.
    pub fn new_tally_text(&self) -> String {
        self.filter().iter().map(|tag| format!("{tag} ")).collect()
    }

    pub fn valid(&self) -> bool {
        !self.pool().is_empty()
    }

    /// Marks the draft as checked by a Save: the pool's error shows.
    pub fn check(&mut self) {
        self.checked = true;
    }

    /// What the draft holds as plain values, kept after the editor is
    /// gone; read while it is [`SessionDraft::valid`].
    pub fn choice(&self) -> SessionChoice {
        let counts = self
            .counts
            .iter()
            .enumerate()
            .filter_map(|(i, own)| Some((i as u32 + 1, (*own)?)))
            .collect();
        SessionChoice {
            pool: self.pool(),
            count: self.count,
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

    /// Writes what changed of `setup`, on its Note (ADR 0039).
    pub fn write(&self, store: &Store, setup: &SessionSetup) -> oldbrainfix_core::Result<()> {
        let sessions = Sessions::new(store);
        let id = setup.note_id.as_str();
        let pool = self.pool();
        if pool != setup.pool {
            sessions.set_pool(id, &pool)?;
        }
        if self.count != setup.count {
            sessions.set_count(id, self.count)?;
        }
        for (i, own) in self.counts.iter().copied().enumerate() {
            let day = i as u32 + 1;
            if own != setup.counts.get(&day).copied() {
                sessions.set_count_on(id, day, own)?;
            }
        }
        if self.rule != setup.rule {
            sessions.set_rule(id, self.rule)?;
        }
        Ok(())
    }

    /// Draws the Session's panels; what was asked beyond typing.
    pub fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<SessionIntent> {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let mut intent = self.pool_panel(ui, t, tones, None);
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.session_count()).strong().color(text));
            stepper(ui, t, &mut self.count, (1, Some(MAX_COUNT)), None);
        });
        embossed(ui, tones, false, None, |ui| {
            let open = self.weekdays_open;
            let fold =
                egui::CollapsingHeader::new(RichText::new(t.session_weekday_counts()).color(text))
                    .id_salt("session weekdays")
                    .open(Some(open))
                    .show(ui, |ui| self.weekdays(ui, t));
            if fold.header_response.clicked() {
                self.weekdays_open = !open;
                intent = Some(SessionIntent::WeekdaysOpen(!open));
            }
            if !open {
                let summary = t.session_weekday_counts_summary(self.own_counts());
                ui.label(RichText::new(summary).small().color(weak));
            }
        });
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.session_rule()).strong().color(text));
            self.rules(ui, t);
        });
        intent
    }

    /// The setup of a Session Note on its page in the Note dialog (#191),
    /// after its name and description: its pool, the last Tally ticked
    /// kept, with the arrows of Rolling on those ticked and "New Tally"
    /// at its end, then how many it picks with − and +; only shown where
    /// this Device may not `change` them. The caller writes each change
    /// at once ([`SessionDraft::write`]): whether there was one, and
    /// what was asked beyond it.
    pub fn note_page(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        tones: Tones,
        changes: bool,
    ) -> (bool, Option<SessionIntent>) {
        let text = ui.visuals().text_color();
        let before = (self.pool(), self.count);
        let intent = self.pool_panel(ui, t, tones, Some(changes));
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.session_count()).strong().color(text));
            if changes {
                stepper(ui, t, &mut self.count, (1, Some(MAX_COUNT)), None);
            } else {
                ui.label(RichText::new(self.count.to_string()).size(16.0).color(text));
            }
        });
        (before != (self.pool(), self.count), intent)
    }

    /// The pool's panel: for a Session Note's page `page` holds whether
    /// this Device may change it, `None` on the Session page.
    fn pool_panel(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        tones: Tones,
        page: Option<bool>,
    ) -> Option<SessionIntent> {
        let mut intent = None;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let error = ui.visuals().error_fg_color;
        let changes = page.unwrap_or(true);
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.session_pool()).strong().color(text));
            ui.label(RichText::new(t.session_pool_explain()).color(text));
            if self.order.is_empty() && !self.new_tally {
                ui.label(RichText::new(t.session_pool_none_desk()).color(weak));
            } else if !self.order.is_empty() {
                self.tag_chips(ui, tones);
                self.pool_rows(ui, t, page);
            }
            // Making one stays inside the list, on this page (#161).
            if self.new_tally && changes {
                let made = ui.horizontal(|ui| {
                    icons::show(ui, icons::ADD, 20.0, text);
                    ui.button(RichText::new(t.new_tally()).color(text))
                });
                if made.inner.clicked() {
                    intent = Some(SessionIntent::NewTally);
                }
            }
            let ordered = match page {
                None => self.order.len(),
                Some(_) => self.chosen.len(),
            };
            let hidden = self.hidden_ticked();
            if hidden > 0 {
                ui.label(RichText::new(t.session_pool_hidden(hidden as i64)).color(weak));
            }
            if self.rule == SessionRule::Rolling
                && changes
                && ordered > 1
                && self.filter().is_empty()
            {
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
        intent
    }

    /// The rules as radio buttons, each with what it does under it;
    /// whether one was clicked.
    pub fn rules(&mut self, ui: &mut Ui, t: &L10n) -> bool {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
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
        let mut clicked = false;
        for (rule, title, detail) in rules {
            clicked |= ui
                .radio_value(&mut self.rule, rule, RichText::new(title).color(text))
                .clicked();
            ui.indent(("session rule", rule.name()), |ui| {
                ui.label(RichText::new(detail).small().color(weak));
            });
        }
        clicked
    }

    /// What the weekdays' counts are about, then a row per weekday,
    /// Monday first.
    pub fn weekdays(&mut self, ui: &mut Ui, t: &L10n) {
        let text = ui.visuals().text_color();
        ui.label(RichText::new(t.session_weekday_counts_explain()).color(text));
        for i in 0..7 {
            self.weekday_row(ui, t, i);
        }
    }

    /// The tags of the pool's Tallies as pills (#220), those most of them
    /// carry first; a click picks one to show only the Tallies carrying
    /// any of those picked, and again takes it back. Nothing without a
    /// tag.
    fn tag_chips(&mut self, ui: &mut Ui, tones: Tones) {
        let tags = self.tag_list();
        if tags.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for tag in tags {
                let on = self.picked.contains(&tag);
                if pill(ui, tones, on, &tag).clicked() {
                    if on {
                        self.picked.remove(&tag);
                    } else {
                        self.picked.insert(tag);
                    }
                }
            }
        });
    }

    /// A checkbox per Tally the filter shows; for Rolling, while nothing
    /// is filtered, each with buttons to move it up or down, a drag's
    /// place on a desk without a long press. On a Session Note's page
    /// (`page`, with whether this Device may change it) only those ticked
    /// move, being the pool's order, and the last one ticked stays.
    fn pool_rows(&mut self, ui: &mut Ui, t: &L10n, page: Option<bool>) {
        let text = ui.visuals().text_color();
        let changes = page.unwrap_or(true);
        let filter = self.filter();
        // Rolling's order is set in the whole list, never a filtered one.
        let rolling = self.rule == SessionRule::Rolling && changes && filter.is_empty();
        // The rows that move: every one on the Session page, those
        // ticked, which come first, on a Note's.
        let moving = match page {
            None => self.order.len(),
            Some(_) => self.chosen.len().min(self.order.len()),
        };
        let mut moved = None;
        for (i, id) in self.order.iter().enumerate() {
            if !self.shows(id, &filter) {
                continue;
            }
            let name = self.names.get(id).cloned().unwrap_or_default();
            ui.horizontal(|ui| {
                let mut on = self.chosen.contains(id);
                let last = page.is_some() && on && self.chosen.len() == 1;
                let tick = egui::Checkbox::new(&mut on, RichText::new(&name).color(text));
                if theme::square_boxes(ui, |ui| ui.add_enabled(changes && !last, tick)).changed() {
                    if on {
                        self.chosen.insert(id.clone());
                    } else {
                        self.chosen.remove(id);
                    }
                }
                if !rolling || i >= moving {
                    return;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if i + 1 < moving
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

    /// The weekday at `index`, 0 for Monday: a checkbox for an own
    /// count, "As every day" while it is off, and the count's stepper
    /// once it is on.
    fn weekday_row(&mut self, ui: &mut Ui, t: &L10n, index: usize) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let day = weekday_long(t, index as u32 + 1);
        let mut own = self.counts[index].is_some();
        if theme::checkbox(ui, &mut own, RichText::new(day).color(text)).changed() {
            self.set_own(index, own);
        }
        match &mut self.counts[index] {
            Some(count) => {
                ui.indent(("session weekday", index), |ui| {
                    stepper(ui, t, count, (1, Some(MAX_COUNT)), None);
                });
            }
            None => {
                ui.indent(("session weekday", index), |ui| {
                    ui.label(
                        RichText::new(t.session_weekday_count_hint())
                            .small()
                            .color(weak),
                    );
                });
            }
        }
    }
}

/// The tags in `text`, each once; none for a Note that is gone.
fn tags_of(text: Option<&str>) -> BTreeSet<String> {
    text_tags::find(text.unwrap_or_default())
        .into_iter()
        .map(|found| found.tag)
        .collect()
}
