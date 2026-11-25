//! The find bar (obf-1047, C7c and C7d): above an opened Note, in the
//! Note dialog, a Note's own window and its reading view, its search word
//! in a field of its own, how many places it finds and which is current
//! ("3 of 12"), Previous and Next (Shift+Enter and Enter in its field),
//! Search all Notes, which searches All Notes for its words (obf-1055),
//! and ✕, which closes it as Esc and leaving the Note do. Every place
//! found shows on a tint while it is open, the current one on a stronger
//! one, and comes into view as it becomes current ([`crate::find_marks`]).
//!
//! It finds in any text the one drawing it hands it: the App hands it a
//! Note's places ([`oldbrainfix_core::findings::of_note`]); a viewer of a
//! single text finds with [`in_text`]. Opening it with words already in
//! its field, at a place found, is [`FindBar::new`] with
//! [`FindBar::starting_at`].

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use egui::{Id, Key, RichText, Ui};
use oldbrainfix_core::findings::{self, Finding};

use crate::app::{icon_button, name_field};
use crate::find_marks::{self, Current, FindMarks};
use crate::icons;
use crate::l10n::L10n;

/// What a click or key on the bar asked of whoever shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// ✕: the bar closes.
    Close,
    /// Search all Notes: All Notes' search is to find its words
    /// (obf-1055).
    SearchAll,
}

/// A find bar's state: its words, the places they were found last and
/// which is current, and what the next frames still owe.
#[derive(Debug, Clone)]
pub struct FindBar {
    /// Tells this bar's marks from another's, in another window.
    id: u64,
    query: String,
    found: Arc<Vec<Finding>>,
    /// What `found` was found in, with the words, by [`FindBar::find`].
    found_in: Option<u64>,
    at: usize,
    /// The place to start at once found, as a card's finding asks.
    wanted: Option<Finding>,
    /// Its field takes the keyboard in the next frame, its words selected.
    focus: bool,
    /// The current place is to come into view.
    reveal: bool,
}

impl FindBar {
    /// A bar finding `query`, its field taking the keyboard while `focus`
    /// (Ctrl+F), at the first place found.
    pub fn new(query: &str, focus: bool) -> FindBar {
        static BARS: AtomicU64 = AtomicU64::new(1);
        FindBar {
            id: BARS.fetch_add(1, Ordering::Relaxed),
            query: query.to_string(),
            found: Arc::new(Vec::new()),
            found_in: None,
            at: 0,
            wanted: None,
            focus,
            reveal: true,
        }
    }

    /// As it is, starting at `finding` once its places are found, or at
    /// the first where it is none of them: a card's finding clicked.
    pub fn starting_at(mut self, finding: Finding) -> FindBar {
        self.wanted = Some(finding);
        self
    }

    /// Its words.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Its field takes the keyboard again, its words selected: Ctrl+F
    /// while it is open.
    pub fn focus(&mut self) {
        self.focus = true;
    }

    /// The id of its field.
    pub fn field_id(&self) -> Id {
        Id::new(("find bar", self.id))
    }

    /// Finds its words anew with `find` where they or what it finds in,
    /// whose contents hash to `texts`, changed; the current place stays
    /// the one it was where it is still found, else the first.
    pub fn find(&mut self, texts: u64, find: impl FnOnce(&str) -> Vec<Finding>) {
        let key = find_marks::text_key(&self.query) ^ texts.rotate_left(1);
        if self.found_in == Some(key) {
            return;
        }
        let before = self.current().cloned();
        let found = find(&self.query);
        let same = |a: &Finding, b: &Finding| {
            a.part == b.part && a.key == b.key && a.section == b.section && a.start == b.start
        };
        self.at = match self.wanted.take().or(before) {
            Some(wanted) => found.iter().position(|f| same(f, &wanted)).unwrap_or(0),
            None => 0,
        };
        self.found = Arc::new(found);
        self.found_in = Some(key);
    }

    /// The places found last, top first.
    pub fn found(&self) -> &[Finding] {
        &self.found
    }

    /// Which of them is current, counted from 0.
    pub fn at(&self) -> usize {
        self.at
    }

    /// The current place found, while there is one.
    pub fn current(&self) -> Option<&Finding> {
        self.found.get(self.at)
    }

    /// Goes to the next place found, or back to the one before, round
    /// from the last to the first and back; it comes into view.
    pub fn step(&mut self, forward: bool) {
        let count = self.found.len();
        if count == 0 {
            return;
        }
        self.at = if forward {
            (self.at + 1) % count
        } else {
            (self.at + count - 1) % count
        };
        self.reveal = true;
    }

    /// What its marks highlight in the frame drawn now: every place its
    /// words find, the current one where `drawn_at` says it is drawn (the
    /// [`find_marks::text_key`] of the source drawn and where it begins
    /// there as it shows; for a single text, that text's key and the
    /// place's start), still to come into view until it was drawn so.
    pub fn marks(
        &mut self,
        ctx: &egui::Context,
        drawn_at: impl Fn(&Finding) -> Option<Current>,
    ) -> Arc<FindMarks> {
        if find_marks::take_revealed(ctx, self.id) {
            self.reveal = false;
        }
        let current = self.current().and_then(drawn_at);
        Arc::new(FindMarks {
            bar: self.id,
            query: self.query.clone(),
            current,
            reveal: self.reveal && current.is_some(),
        })
    }

    /// Draws the bar across `ui`: the search icon, its field, how many
    /// places its words find and which is current, Previous, Next,
    /// Search all Notes and ✕. Enter in its field goes to the next place, Shift+Enter back,
    /// the field keeping the keyboard. What was asked comes back.
    pub fn show(&mut self, ui: &mut Ui, t: &L10n) -> Option<Asked> {
        self.show_with(ui, t, (t.desk_find_hint(), t.desk_find_none()))
    }

    /// As [`FindBar::show`], its field's hint and what it says while its
    /// words are nowhere `said`: a viewer of a file names the file there
    /// (obf-1056).
    pub fn show_with(&mut self, ui: &mut Ui, t: &L10n, said: (&str, &str)) -> Option<Asked> {
        let (hint, nowhere) = said;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let mut asked = None;
        let before = (self.at, self.found_in);
        ui.horizontal(|ui| {
            // From the right: ✕, Next, Previous and the count keep their
            // room, the field takes what is left.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, icons::CLOSE, t.desk_find_close(), text).clicked() {
                    asked = Some(Asked::Close);
                }
                let typed = !self.query.trim().is_empty();
                let shade = if typed { text } else { weak };
                let all = ui
                    .add_enabled_ui(typed, |ui| {
                        icon_button(ui, icons::ALL_NOTES, t.search_all_hint(), shade)
                    })
                    .inner;
                if all.clicked() {
                    asked = Some(Asked::SearchAll);
                }
                let count = self.found.len();
                let some = count > 0;
                let shade = if some { text } else { weak };
                let next = ui
                    .add_enabled_ui(some, |ui| {
                        icon_button(ui, icons::EXPAND_MORE, t.desk_find_next(), shade)
                    })
                    .inner;
                if next.clicked() {
                    self.step(true);
                }
                let previous = ui
                    .add_enabled_ui(some, |ui| {
                        icon_button(ui, icons::EXPAND_LESS, t.desk_find_previous(), shade)
                    })
                    .inner;
                if previous.clicked() {
                    self.step(false);
                }
                let said = if findings::fold(self.query.trim()).is_empty() {
                    String::new()
                } else if count == 0 {
                    nowhere.to_string()
                } else {
                    t.desk_find_count(self.at as i64 + 1, count as i64)
                };
                ui.label(RichText::new(said).color(weak));
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    icons::show(ui, icons::SEARCH, 20.0, weak);
                    let id = self.field_id();
                    let field = ui.add(
                        crate::theme::field(ui, &mut self.query)
                            .id(id)
                            .hint_text(hint)
                            .text_color(text)
                            .desired_width(ui.available_width()),
                    );
                    name_field(ui, &field, hint);
                    crate::text_menu::show(ui, t, &field, &self.query);
                    // Not in a dialog's first, sizing pass, which drops a
                    // focus.
                    if !ui.is_sizing_pass() && std::mem::take(&mut self.focus) {
                        field.request_focus();
                        select_all(ui, id, &self.query);
                    }
                    if field.changed() {
                        self.found_in = None;
                        self.at = 0;
                        self.reveal = true;
                    }
                    // Enter takes the keyboard out of a one-line field;
                    // here it stays, the next place current.
                    let entered = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    if entered {
                        let back = ui.input(|i| i.modifiers.shift);
                        self.step(!back);
                        field.request_focus();
                    }
                });
            });
        });
        // The count follows in the next frame, found anew.
        if (self.at, self.found_in) != before {
            ui.ctx().request_repaint();
        }
        asked
    }
}

/// The places `query` is found in `shown`, a single text as it shows in
/// `format`: what a bar over a viewer of one text finds with
/// [`FindBar::find`], its current place's text named by
/// [`find_marks::text_key`] of the source drawn.
pub fn in_text(shown: &str, format: &str, query: &str) -> Vec<Finding> {
    findings::in_text(shown, format, query)
}

/// Selects all of `text` in the field `id`, as Ctrl+F finds it again.
fn select_all(ui: &Ui, id: Id, text: &str) {
    let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
    let all = egui::text::CCursorRange::two(
        egui::text::CCursor::new(0),
        egui::text::CCursor::new(text.chars().count()),
    );
    state.cursor.set_char_range(Some(all));
    state.store(ui.ctx(), id);
}
