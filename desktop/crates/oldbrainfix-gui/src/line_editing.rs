//! The Lines of a Checklist and a Shopping List typed in place in the
//! Note editor (#141), as the phone's `line_editing.dart`,
//! `checklist_view.dart` and `shopping_list_view.dart` type them.
//!
//! On a Checklist each Line is a field beside its box: Enter splits it at
//! the cursor, at its end starts a new empty Line under it, in the group
//! of a Line with one; Backspace at its start joins it with the Line
//! above, whose box and state stay, and an empty Line simply goes; Tab
//! and Shift+Tab put it under the Line above and back at the top; Up and
//! Down move the cursor to the Line above and below. A Shopping List's
//! Lines are renamed in place, Enter ending the rename, and its field at
//! the top adds on Enter and stays ready for the next.
//!
//! What is typed is stored as the cursor leaves a Line and whenever the
//! editor saves ([`TypedLines::keep`]); as the editor is left
//! ([`TypedLines::leave`]) the Lines left empty go, and what is left in
//! the Shopping List's field is added. A new Line exists only on screen
//! until it holds text.

use std::collections::HashMap;

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::text_edit::TextEditState;
use egui::{FontId, Id, Key, Modifiers, Stroke, TextEdit, Ui};
use oldbrainfix_core::{
    Checklist, Checklists, Notes, ShoppingList, ShoppingLists, Store, WallClock,
};

use crate::format_bar;

/// What the id a new Line shows under starts with, its row's key after
/// it: no Line's id looks like it.
pub const NEW: &str = "new-";

/// A Line as it is typed in place: its text, and its id once it is
/// stored, `None` while it is new and only typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedLine {
    /// Which row it is, the same while it is typed in, stored or moved,
    /// so its field keeps the keyboard.
    pub key: u64,
    pub id: Option<String>,
    /// What its field holds.
    pub text: String,
    /// The text as it was last stored or read, to tell what was typed.
    pub kept: String,
    /// Whether it is in the group of the Line above; for a stored Line
    /// as the list says.
    pub in_group: bool,
}

impl TypedLine {
    /// The text typed, on one line and trimmed, as a Line holds it; empty
    /// for none.
    pub fn typed(&self) -> String {
        one_line(&self.text)
    }

    /// Whether something was typed that is not stored yet.
    pub fn changed(&self) -> bool {
        self.typed() != self.kept
    }

    /// The id it shows under: its Line's, or one of its own while new.
    pub fn shown_id(&self) -> String {
        match &self.id {
            Some(id) => id.clone(),
            None => format!("{NEW}{}", self.key),
        }
    }

    /// Puts `stored` into the field, unless something typed is not
    /// stored yet.
    fn take(&mut self, stored: &str) {
        if stored == self.kept {
            return;
        }
        if !self.changed() {
            self.text = stored.to_string();
        }
        self.kept = stored.to_string();
    }
}

/// `text` on one line, every run of line breaks a space, and trimmed.
pub fn one_line(text: &str) -> String {
    let mut line = String::with_capacity(text.len());
    let mut broken = false;
    for c in text.chars() {
        if c == '\r' || c == '\n' {
            broken = true;
            continue;
        }
        if broken {
            line.push(' ');
            broken = false;
        }
        line.push(c);
    }
    line.trim().to_string()
}

/// What a Line's field asked as it was typed in, by the row's key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typing {
    /// Enter at this byte of its text.
    Split(u64, usize),
    /// Backspace at its start.
    Join(u64),
    /// Tab, or with `false` Shift+Tab.
    Indent(u64, bool),
    /// The cursor left it, or Enter ended a Shopping List's rename.
    Store(u64),
}

/// A Shopping List as it is read, and which of its Lines, by id, this
/// Device may rename.
pub type ShoppingRows<'a> = (&'a ShoppingList, &'a dyn Fn(&str) -> bool);

/// The Lines of the Note shown, as they are typed, in the order they
/// show; a Shopping List's only those this Device may rename.
#[derive(Debug, Default)]
pub struct TypedLines {
    /// The Note whose Lines they are.
    pub note: Option<String>,
    pub rows: Vec<TypedLine>,
    /// What the Shopping List's field at the top holds, and whether it
    /// is to take the keyboard, until it has it: a list just made (#161).
    pub add: String,
    pub add_focus: bool,
    /// The field to take the keyboard, by its row's key, with the cursor
    /// at this byte of its text.
    pub focus: Option<(u64, usize)>,
    next: u64,
}

impl TypedLines {
    /// A row for the Line `id` holding `text`, or a new one.
    fn row(&mut self, id: Option<String>, text: &str, in_group: bool) -> TypedLine {
        self.next += 1;
        TypedLine {
            key: self.next,
            id,
            text: text.to_string(),
            kept: text.to_string(),
            in_group,
        }
    }

    fn at(&self, key: u64) -> Option<usize> {
        self.rows.iter().position(|r| r.key == key)
    }

    /// The row shown under `id`.
    pub fn shown(&self, id: &str) -> Option<&TypedLine> {
        self.rows.iter().find(|r| r.shown_id() == id)
    }

    /// Takes the Lines of the Note `note` as they are read now, keeping
    /// what is typed into them: a Checklist's `checklist`, else a Shopping
    /// List's `shopping` with the Lines this Device may rename. Another
    /// Note starts afresh.
    pub fn arrange(
        &mut self,
        note: Option<&str>,
        checklist: Option<&Checklist>,
        shopping: Option<ShoppingRows>,
    ) {
        if self.note.as_deref() != note {
            self.note = note.map(str::to_string);
            self.rows.clear();
            self.add.clear();
            self.add_focus = false;
            self.focus = None;
        }
        match (checklist, shopping) {
            (Some(list), _) => self.arrange_checklist(list),
            (None, Some((list, editable))) => self.arrange_shopping(list, editable),
            (None, None) => self.rows.clear(),
        }
        if let Some((key, _)) = self.focus
            && self.at(key).is_none()
        {
            self.focus = None;
        }
    }

    /// The rows for the Lines of `list`: those there already kept with
    /// what is typed in them, a new one after the Line it was started
    /// under, a row whose Line went while typed in kept as a new one.
    fn arrange_checklist(&mut self, list: &Checklist) {
        let mut stored = HashMap::new();
        let mut after: Vec<(Option<String>, TypedLine)> = Vec::new();
        let mut above: Option<String> = None;
        for mut row in std::mem::take(&mut self.rows) {
            match row.id.clone() {
                Some(id) if list.line(&id).is_some() => {
                    above = Some(id.clone());
                    stored.insert(id, row);
                }
                Some(_) => {
                    if !row.changed() || row.typed().is_empty() {
                        continue;
                    }
                    row.id = None;
                    row.kept = String::new();
                    after.push((above.clone(), row));
                }
                None => after.push((above.clone(), row)),
            }
        }
        let mut rows = take_after(&mut after, None);
        for line in &list.lines {
            let mut row = match stored.remove(&line.id) {
                Some(row) => row,
                None => self.row(Some(line.id.clone()), &line.text, false),
            };
            row.in_group = line.in_group();
            row.take(&line.text);
            rows.push(row);
            rows.extend(take_after(&mut after, Some(&line.id)));
        }
        // Started under a Line that went: at the end.
        rows.extend(after.into_iter().map(|(_, row)| row));
        self.rows = rows;
    }

    /// The rows for the Lines of `list` this Device may rename.
    fn arrange_shopping(&mut self, list: &ShoppingList, editable: &dyn Fn(&str) -> bool) {
        let mut old: HashMap<String, TypedLine> = std::mem::take(&mut self.rows)
            .into_iter()
            .filter_map(|row| row.id.clone().map(|id| (id, row)))
            .collect();
        let mut rows = Vec::new();
        for line in list.lines.iter().filter(|l| editable(&l.id)) {
            let mut row = match old.remove(&line.id) {
                Some(row) => row,
                None => self.row(Some(line.id.clone()), &line.text, false),
            };
            row.take(&line.text);
            rows.push(row);
        }
        self.rows = rows;
    }

    /// How many stored Lines show before the row at `at`: where it is
    /// added.
    fn stored_before(&self, at: usize) -> usize {
        self.rows[..at].iter().filter(|r| r.id.is_some()).count()
    }

    /// Stores what was typed into the row at `at`: a new Line once it
    /// holds text, a Line renamed once its text changed. An empty one
    /// waits.
    fn store_row(&mut self, store: &Store, at: usize) -> oldbrainfix_core::Result<()> {
        let Some(note) = self.note.clone() else {
            return Ok(());
        };
        let row = &self.rows[at];
        let typed = row.typed();
        if typed.is_empty() || !row.changed() {
            return Ok(());
        }
        let checklists = Checklists::new(store);
        if open_checklist(store, &note)? {
            match row.id.clone() {
                None => {
                    if !checklists.may_check(&note)? {
                        return Ok(());
                    }
                    let place = self.stored_before(at);
                    let id = checklists.add(&note, &typed, Some(place), row.in_group)?;
                    self.rows[at].id = Some(id);
                }
                Some(id) => {
                    if !shows(store, &note, &id)? || !checklists.may_edit(&id)? {
                        return Ok(());
                    }
                    checklists.rename(&id, &typed)?;
                }
            }
        } else {
            let shopping = ShoppingLists::new(store);
            let Some(id) = row.id.clone() else {
                return Ok(());
            };
            if !open_shopping(store, &note)? || !shopping.may_edit(&id)? {
                return Ok(());
            }
            shopping.rename(&id, &typed)?;
        }
        self.rows[at].kept = typed;
        Ok(())
    }

    /// Stores what was typed into every Line, as the editor saves.
    pub fn keep(&mut self, store: &Store) -> oldbrainfix_core::Result<()> {
        if !self
            .rows
            .iter()
            .any(|r| r.changed() && !r.typed().is_empty())
        {
            return Ok(());
        }
        store.batch(|| {
            for at in 0..self.rows.len() {
                self.store_row(store, at)?;
            }
            Ok(())
        })
    }

    /// Stores what was typed, as the editor is left: what is left in the
    /// Shopping List's field is added, and the Lines left empty go.
    pub fn leave(&mut self, store: &Store, wall: &WallClock) -> oldbrainfix_core::Result<()> {
        self.keep(store)?;
        let Some(note) = self.note.clone() else {
            return Ok(());
        };
        if !self.add.trim().is_empty() {
            let shopping = ShoppingLists::new(store);
            if open_shopping(store, &note)? && shopping.may_check(&note)? {
                shopping.add_lines(&note, &self.add, wall)?;
            }
            self.add.clear();
        }
        let empty: Vec<u64> = self
            .rows
            .iter()
            .filter(|r| r.typed().is_empty())
            .map(|r| r.key)
            .collect();
        if empty.is_empty() {
            return Ok(());
        }
        store.batch(|| {
            for key in &empty {
                if let Some(at) = self.at(*key) {
                    self.delete_row(store, &note, at)?;
                }
            }
            Ok(())
        })?;
        self.rows.retain(|r| !empty.contains(&r.key));
        Ok(())
    }

    /// Deletes the Line of the row at `at`, where this Device may: whether
    /// it may, or the row is new.
    fn delete_row(&self, store: &Store, note: &str, at: usize) -> oldbrainfix_core::Result<bool> {
        let Some(id) = &self.rows[at].id else {
            return Ok(true);
        };
        if open_checklist(store, note)? {
            let checklists = Checklists::new(store);
            if !shows(store, note, id)? {
                return Ok(true);
            }
            if !checklists.may_edit(id)? {
                return Ok(false);
            }
            checklists.delete(id)?;
        } else if open_shopping(store, note)? {
            let shopping = ShoppingLists::new(store);
            if !shopping.may_edit(id)? {
                return Ok(false);
            }
            shopping.delete(id)?;
        }
        Ok(true)
    }

    /// The id of the Line shown under `shown`: a new one stored first,
    /// `None` while it holds nothing to store.
    pub fn stored_id(
        &mut self,
        store: &Store,
        shown: &str,
    ) -> oldbrainfix_core::Result<Option<String>> {
        let Some(at) = self.rows.iter().position(|r| r.shown_id() == shown) else {
            return Ok(Some(shown.to_string()));
        };
        if self.rows[at].id.is_none() {
            self.store_row(store, at)?;
        }
        Ok(self.rows[at].id.clone())
    }

    /// Does what a field asked.
    pub fn typing(&mut self, store: &Store, asked: Typing) -> oldbrainfix_core::Result<()> {
        match asked {
            Typing::Split(key, at) => self.split(store, key, at),
            Typing::Join(key) => self.join(store, key),
            Typing::Indent(key, under) => self.indent(store, key, under),
            Typing::Store(key) => match self.at(key) {
                Some(at) => self.store_row(store, at),
                None => Ok(()),
            },
        }
    }

    /// Splits the row `key` at the byte `byte` of its text: the text
    /// before stays, the text after becomes a new Line under it, an empty
    /// one at its end; a Line with a group gets it as the first of its
    /// group. At its start, an empty Line goes in above and the cursor
    /// stays.
    fn split(&mut self, store: &Store, key: u64, byte: usize) -> oldbrainfix_core::Result<()> {
        let (Some(at), Some(note)) = (self.at(key), self.note.clone()) else {
            return Ok(());
        };
        let text = self.rows[at].text.clone();
        let byte = (0..=byte.min(text.len()))
            .rev()
            .find(|b| text.is_char_boundary(*b))
            .unwrap_or(0);
        let (before, after) = (one_line(&text[..byte]), one_line(&text[byte..]));
        if before.is_empty() {
            // Enter in an empty Line: it is the new one already.
            if after.is_empty() {
                return Ok(());
            }
            let in_group = self.rows[at].in_group;
            let row = self.row(None, "", in_group);
            self.rows.insert(at, row);
            self.focus = Some((key, 0));
            return Ok(());
        }
        let group = self.rows[at].in_group || has_group(store, &note, self.rows[at].id.as_deref())?;
        self.rows[at].text = before;
        store.batch(|| {
            self.store_row(store, at)?;
            if self.rows[at].id.is_none() {
                // Not stored: the field keeps all that was typed.
                self.rows[at].text = text.clone();
                return Ok(());
            }
            let row = if after.is_empty() {
                self.row(None, "", group)
            } else {
                let place = self.stored_before(at) + 1;
                let id = Checklists::new(store).add(&note, &after, Some(place), group)?;
                self.row(Some(id), &after, group)
            };
            self.focus = Some((row.key, 0));
            self.rows.insert(at + 1, row);
            Ok(())
        })
    }

    /// Joins the row `key` with the Line above at a Backspace at its
    /// start: its text goes to the end of the Line above, which keeps its
    /// box and state, and its Line is deleted, its checks staying in the
    /// log; its group stays a group, as far as this Device may change it.
    /// An empty Line simply goes; under a Line of someone else's nothing
    /// is joined.
    fn join(&mut self, store: &Store, key: u64) -> oldbrainfix_core::Result<()> {
        let (Some(at), Some(note)) = (self.at(key), self.note.clone()) else {
            return Ok(());
        };
        let checklists = Checklists::new(store);
        let text = self.rows[at].typed();
        if text.is_empty() {
            return self.drop_row(store, &note, at);
        }
        if at == 0 || !open_checklist(store, &note)? {
            return Ok(());
        }
        if let Some(upper) = &self.rows[at - 1].id
            && !checklists.may_edit(upper)?
        {
            return Ok(());
        }
        let head = self.rows[at - 1].typed();
        let mut group = Vec::new();
        if let Some(id) = &self.rows[at].id
            && let Some(list) = checklists.by_note(&note)?
        {
            for line in list.group_of(id) {
                if checklists.may_edit(&line.id)? {
                    group.push(line.id.clone());
                }
            }
        }
        self.rows[at - 1].text = format!("{head}{text}");
        store.batch(|| {
            self.store_row(store, at - 1)?;
            if let Some(id) = &self.rows[at].id
                && shows(store, &note, id)?
                && checklists.may_edit(id)?
            {
                checklists.delete(id)?;
                for line in &group {
                    checklists.indent(line, true)?;
                }
            }
            Ok(())
        })?;
        self.focus = Some((self.rows[at - 1].key, head.len()));
        self.rows.remove(at);
        Ok(())
    }

    /// Takes the empty row at `at` away, deleting its Line; the cursor
    /// goes to the end of the Line above, or to the start of the next.
    fn drop_row(&mut self, store: &Store, note: &str, at: usize) -> oldbrainfix_core::Result<()> {
        if !self.delete_row(store, note, at)? {
            return Ok(());
        }
        self.rows.remove(at);
        self.focus = match at.checked_sub(1).and_then(|above| self.rows.get(above)) {
            Some(above) => Some((above.key, above.text.len())),
            None => self.rows.first().map(|first| (first.key, 0)),
        };
        Ok(())
    }

    /// Puts the row `key` under the Line above, or with `under` false
    /// back at the top.
    fn indent(&mut self, store: &Store, key: u64, under: bool) -> oldbrainfix_core::Result<()> {
        let (Some(at), Some(note)) = (self.at(key), self.note.clone()) else {
            return Ok(());
        };
        let Some(id) = self.rows[at].id.clone() else {
            self.rows[at].in_group = under && at > 0;
            return Ok(());
        };
        let checklists = Checklists::new(store);
        if !open_checklist(store, &note)? || !checklists.may_edit(&id)? {
            return Ok(());
        }
        let Some(list) = checklists.by_note(&note)? else {
            return Ok(());
        };
        if list.line(&id).is_none() || (under && list.lines.first().is_some_and(|l| l.id == id)) {
            return Ok(());
        }
        checklists.indent(&id, under)
    }

    /// "Add Line": an empty Line at the end, the cursor in it.
    pub fn add_empty(&mut self) {
        if let Some(last) = self.rows.last()
            && last.id.is_none()
            && last.typed().is_empty()
        {
            self.focus = Some((last.key, 0));
            return;
        }
        let row = self.row(None, "", false);
        self.focus = Some((row.key, 0));
        self.rows.push(row);
    }

    /// Adds what the Shopping List's field holds, every line not blank a
    /// Line, the first ending at the top; the field is empty again.
    pub fn add_typed(&mut self, store: &Store, wall: &WallClock) -> oldbrainfix_core::Result<()> {
        let Some(note) = self.note.clone() else {
            return Ok(());
        };
        let shopping = ShoppingLists::new(store);
        if self.add.trim().is_empty()
            || !open_shopping(store, &note)?
            || !shopping.may_check(&note)?
        {
            return Ok(());
        }
        shopping.add_lines(&note, &self.add, wall)?;
        self.add.clear();
        Ok(())
    }
}

/// The rows of `after` started under the Line `above`, or at the top,
/// in the order they were.
fn take_after(after: &mut Vec<(Option<String>, TypedLine)>, above: Option<&str>) -> Vec<TypedLine> {
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(after)
        .into_iter()
        .partition(|(a, _)| a.as_deref() == above);
    *after = rest;
    mine.into_iter().map(|(_, row)| row).collect()
}

/// Whether the Note `note` is a Checklist out of the trash, to write to.
fn open_checklist(store: &Store, note: &str) -> oldbrainfix_core::Result<bool> {
    Ok(!trashed(store, note)? && Checklists::new(store).is_checklist(note)?)
}

/// Whether the Note `note` is a Shopping List out of the trash.
fn open_shopping(store: &Store, note: &str) -> oldbrainfix_core::Result<bool> {
    Ok(!trashed(store, note)? && ShoppingLists::new(store).is_shopping_list(note)?)
}

fn trashed(store: &Store, note: &str) -> oldbrainfix_core::Result<bool> {
    Ok(Notes::new(store)
        .by_id(note)?
        .is_none_or(|n| n.is_trashed()))
}

/// Whether the Line `id` still shows on the Checklist of `note`.
fn shows(store: &Store, note: &str, id: &str) -> oldbrainfix_core::Result<bool> {
    Ok(Checklists::new(store)
        .by_note(note)?
        .is_some_and(|list| list.line(id).is_some()))
}

/// Whether the Line `id` has a group under it.
fn has_group(store: &Store, note: &str, id: Option<&str>) -> oldbrainfix_core::Result<bool> {
    let Some(id) = id else {
        return Ok(false);
    };
    Ok(Checklists::new(store)
        .by_note(note)?
        .is_some_and(|list| !list.group_of(id).is_empty()))
}

/// What kind of list a field types into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldOf {
    /// A Checklist's Line, which Enter splits, Backspace at its start
    /// joins and Tab moves.
    Checklist,
    /// A Shopping List's Line, which Enter is done renaming.
    Shopping,
}

/// The id of the field of the row `key`.
pub fn field_id(key: u64) -> Id {
    Id::new(("typed line", key))
}

/// The field of the row `key` of `lines`, `width` wide, struck through
/// once `checked`, with `hint` while empty; `(above, below)` are the
/// rows Up and Down go to. What it asks goes to `asked`.
pub fn field(
    ui: &mut Ui,
    lines: &mut TypedLines,
    (key, kind): (u64, FieldOf),
    (checked, hint, width): (bool, &str, f32),
    (above, below): (Option<u64>, Option<u64>),
    asked: &mut Vec<Typing>,
) {
    let Some(at) = lines.at(key) else {
        return;
    };
    let id = field_id(key);
    if ui.memory(|m| m.has_focus(id)) {
        keys(ui, lines, (at, kind), (above, below), asked);
    }
    let pending = lines.focus.filter(|(k, _)| *k == key).map(|(_, at)| at);
    let row = &mut lines.rows[at];
    if let Some(byte) = pending {
        let byte = byte.min(row.text.len());
        format_bar::select(ui, id, &row.text, (byte, byte));
    }
    let colour = if checked {
        ui.visuals().weak_text_color()
    } else {
        ui.visuals().text_color()
    };
    let mut layouter = |ui: &Ui, typed: &dyn egui::TextBuffer, wrap: f32| {
        let format = TextFormat {
            font_id: FontId::proportional(16.0),
            color: colour,
            strikethrough: if checked {
                Stroke::new(1.0, colour)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        };
        let mut job = LayoutJob::single_section(typed.as_str().to_string(), format);
        job.wrap.max_width = wrap;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let response = ui.add(
        TextEdit::singleline(&mut row.text)
            .id(id)
            .hint_text(hint)
            .layouter(&mut layouter)
            .lock_focus(true)
            .return_key(None)
            .frame(egui::Frame::NONE)
            .desired_width(width),
    );
    // egui drops a focus asked for a field new this frame; it is asked
    // again until the field holds it.
    if let Some(byte) = pending {
        if response.has_focus() {
            lines.focus = None;
        } else {
            let row = &lines.rows[at];
            format_bar::select(
                ui,
                id,
                &row.text,
                (byte.min(row.text.len()), byte.min(row.text.len())),
            );
            ui.ctx().request_repaint();
        }
    }
    if response.lost_focus() {
        asked.push(Typing::Store(key));
    }
}

/// The keys a focused field takes before it: Enter, Backspace at its
/// start, Tab and Shift+Tab, Up and Down.
fn keys(
    ui: &mut Ui,
    lines: &mut TypedLines,
    (at, kind): (usize, FieldOf),
    (above, below): (Option<u64>, Option<u64>),
    asked: &mut Vec<Typing>,
) {
    let key = lines.rows[at].key;
    let id = field_id(key);
    let text = &lines.rows[at].text;
    let chars = text.chars().count();
    let [from, to] = TextEditState::load(ui.ctx(), id)
        .and_then(|s| s.cursor.char_range())
        .map_or([chars, chars], |range| {
            range.sorted_cursors().map(|c| c.index.0.min(chars))
        });
    let byte = |char_at: usize| {
        text.char_indices()
            .nth(char_at)
            .map_or(text.len(), |(b, _)| b)
    };
    let (from_byte, to_byte) = (byte(from), byte(to));
    let cursor_to = |ui: &Ui, char_at: usize| {
        let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(char_at))));
        state.store(ui.ctx(), id);
    };
    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
        match kind {
            FieldOf::Checklist => {
                // Over a selection, Enter takes its place.
                lines.rows[at].text.replace_range(from_byte..to_byte, "");
                cursor_to(ui, from);
                asked.push(Typing::Split(key, from_byte));
            }
            FieldOf::Shopping => {
                asked.push(Typing::Store(key));
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        }
        return;
    }
    if kind == FieldOf::Shopping {
        return;
    }
    if to == 0 && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace)) {
        asked.push(Typing::Join(key));
        return;
    }
    // Shift+Tab first: a plain Tab matches it too.
    if ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab)) {
        asked.push(Typing::Indent(key, false));
        return;
    }
    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
        asked.push(Typing::Indent(key, true));
        return;
    }
    for (arrow, next) in [(Key::ArrowUp, above), (Key::ArrowDown, below)] {
        if let Some(next) = next
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, arrow))
            && let Some(row) = lines.at(next).map(|n| &lines.rows[n])
        {
            // As far along as here, as far as its text goes.
            let byte = row
                .text
                .char_indices()
                .nth(from)
                .map_or(row.text.len(), |(b, _)| b);
            lines.focus = Some((next, byte));
            return;
        }
    }
}
