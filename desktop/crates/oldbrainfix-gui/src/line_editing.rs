//! The Lines of a Checklist and a Shopping List typed in place in the
//! Note editor (#141, #184), as the phone's `line_editing.dart`,
//! `checklist_view.dart` and `shopping_list_view.dart` type them: the
//! same for both, one block of text, every Line a field beside its box.
//!
//! Enter splits a Line at the cursor, at its end starts a new empty Line
//! under it, in the group of a Line with one; Backspace at its start
//! joins it with the Line above, whose box and state stay, and an empty
//! Line simply goes; on a Checklist Tab and Shift+Tab put it under the
//! Line above and back at the top; Up and Down move the cursor to the
//! Line above and below. A list this Device may add to always has a Line
//! to type in, an empty one at its end while it has none, so a new list
//! starts with one. A Checklist's rows follow its order; a Shopping
//! List's stay where they are while the editor is open, a Line checked
//! there in its row, and a new one is added among the open Lines where it
//! was typed.
//!
//! What is typed is stored as the cursor leaves a Line and whenever the
//! editor saves ([`TypedLines::keep`]); as the editor is left
//! ([`TypedLines::leave`]) the Lines left empty go. A new Line exists
//! only on screen until it holds text.

use std::collections::{HashMap, HashSet};

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

    /// Whether it stays as a new Line now that its Line went: something
    /// typed in it is not stored.
    fn kept_as_new(&mut self) -> bool {
        if !self.changed() || self.typed().is_empty() {
            return false;
        }
        self.id = None;
        self.kept.clear();
        true
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
    /// The cursor left it.
    Store(u64),
}

/// The Lines of the Note shown as they are read: a Checklist's or a
/// Shopping List's.
#[derive(Debug, Clone, Copy)]
pub enum Listed<'a> {
    Checklist(&'a Checklist),
    Shopping(&'a ShoppingList),
}

/// What this Device may do with the Lines read: whether it may add
/// them, and which it may change, by id.
pub type Rights<'a> = (bool, &'a HashSet<String>);

/// The Lines of the Note shown, as they are typed, in the order they
/// show.
#[derive(Debug, Default)]
pub struct TypedLines {
    /// The Note whose Lines they are.
    pub note: Option<String>,
    pub rows: Vec<TypedLine>,
    /// The field to take the keyboard, by its row's key, with the cursor
    /// at this byte of its text.
    pub focus: Option<(u64, usize)>,
    /// Whether this Device may add Lines, as last read.
    adds: bool,
    /// The struck Lines of a Shopping List as last read, by id: a new
    /// Line is placed among the open ones.
    struck: HashSet<String>,
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

    /// The row at `at`: an error for a place no row is at, which the
    /// callers, each with a place just looked up, rule out.
    fn row_at(&self, at: usize) -> oldbrainfix_core::Result<&TypedLine> {
        self.rows.get(at).ok_or_else(|| no_row(at))
    }

    /// [`TypedLines::row_at`], to change.
    fn row_at_mut(&mut self, at: usize) -> oldbrainfix_core::Result<&mut TypedLine> {
        self.rows.get_mut(at).ok_or_else(|| no_row(at))
    }

    /// The row shown under `id`.
    pub fn shown(&self, id: &str) -> Option<&TypedLine> {
        self.rows.iter().find(|r| r.shown_id() == id)
    }

    /// The Lines with ⋮, by the id they show under: `editable`, and the
    /// new ones that hold text, stored as an entry of their ⋮ is used
    /// (#202). An empty new one has none.
    pub fn with_menus(&self, editable: &HashSet<String>) -> HashSet<String> {
        let mut menus = editable.clone();
        if self.adds {
            menus.extend(
                self.rows
                    .iter()
                    .filter(|r| r.id.is_none() && !r.typed().is_empty())
                    .map(TypedLine::shown_id),
            );
        }
        menus
    }

    /// Takes the Lines of the Note `note` as they are read now, keeping
    /// what is typed into them, and as `rights` allow an empty row to
    /// type in where none of them can be. Another Note starts afresh.
    pub fn arrange(
        &mut self,
        note: Option<&str>,
        listed: Option<Listed>,
        (adds, editable): Rights,
    ) {
        if self.note.as_deref() != note {
            self.note = note.map(str::to_string);
            self.rows.clear();
            self.focus = None;
        }
        self.adds = adds && listed.is_some();
        self.struck.clear();
        match listed {
            Some(Listed::Checklist(list)) => self.arrange_checklist(list),
            Some(Listed::Shopping(list)) => self.arrange_shopping(list),
            None => self.rows.clear(),
        }
        // A list without Lines, or with only others', has one to type in.
        if self.adds
            && !self
                .rows
                .iter()
                .any(|r| r.id.as_ref().is_none_or(|id| editable.contains(id)))
        {
            let row = self.row(None, "", false);
            self.rows.push(row);
        }
        if let Some((key, _)) = self.focus
            && self.at(key).is_none()
        {
            self.focus = None;
        }
    }

    /// The cursor into the empty Line of a list without Lines, as a new
    /// list starts (#161).
    pub fn focus_empty_row(&mut self) {
        if let [row] = self.rows.as_slice()
            && row.id.is_none()
            && row.typed().is_empty()
        {
            self.focus = Some((row.key, 0));
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
                    if row.kept_as_new() {
                        after.push((above.clone(), row));
                    }
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

    /// The rows for the Lines of `list` with the rows there already where
    /// they are, so a Line stays where it was typed or checked while the
    /// editor is open; a Line new to the list from elsewhere comes in at
    /// the end. A row whose Line went while typed in is kept as a new one.
    fn arrange_shopping(&mut self, list: &ShoppingList) {
        self.struck = list
            .lines
            .iter()
            .filter(|l| l.struck)
            .map(|l| l.id.clone())
            .collect();
        let known: HashSet<String> = self.rows.iter().filter_map(|r| r.id.clone()).collect();
        let mut rows = Vec::new();
        for mut row in std::mem::take(&mut self.rows) {
            let Some(id) = row.id.clone() else {
                rows.push(row);
                continue;
            };
            match list.line(&id) {
                Some(line) => {
                    row.in_group = false;
                    row.take(&line.text);
                    rows.push(row);
                }
                None => {
                    if row.kept_as_new() {
                        rows.push(row);
                    }
                }
            }
        }
        for line in list.lines.iter().filter(|l| !known.contains(&l.id)) {
            let row = self.row(Some(line.id.clone()), &line.text, false);
            rows.push(row);
        }
        self.rows = rows;
    }

    /// Whether the Line of the row at `at` counts for the place a new
    /// Line is added at: every stored one; on a Shopping List the open
    /// ones alone.
    fn places_among(&self, at: usize) -> bool {
        self.rows
            .get(at)
            .and_then(|row| row.id.as_ref())
            .is_some_and(|id| !self.struck.contains(id))
    }

    /// How many stored Lines show before the row at `at`: where it is
    /// added.
    fn stored_before(&self, at: usize) -> usize {
        (0..at).filter(|i| self.places_among(*i)).count()
    }

    /// Stores what was typed into the row at `at`: a new Line once it
    /// holds text, a Line renamed once its text changed. An empty one
    /// waits. The key of the row that holds the Line now: its own, or for
    /// a name a Shopping List has already the row of that Line, the row
    /// at `at` gone.
    fn store_row(
        &mut self,
        store: &Store,
        wall: &WallClock,
        at: usize,
    ) -> oldbrainfix_core::Result<u64> {
        let row = self.row_at(at)?;
        let (key, in_group) = (row.key, row.in_group);
        let Some(note) = self.note.clone() else {
            return Ok(key);
        };
        let typed = row.typed();
        if typed.is_empty() || !row.changed() {
            return Ok(key);
        }
        let Some(kind) = open_kind(store, &note)? else {
            return Ok(key);
        };
        match (kind, row.id.clone()) {
            (FieldOf::Checklist, None) => {
                let checklists = Checklists::new(store);
                if !checklists.may_check(&note)? {
                    return Ok(key);
                }
                let place = self.stored_before(at);
                let id = checklists.add(&note, &typed, Some(place), in_group)?;
                self.row_at_mut(at)?.id = Some(id);
            }
            (FieldOf::Shopping, None) => {
                let shopping = ShoppingLists::new(store);
                if !shopping.may_check(&note)? {
                    return Ok(key);
                }
                let place = self.stored_before(at);
                let id = shopping.add_at(&note, &typed, wall, Some(place))?;
                // A name the list has already: its row holds the Line.
                if let Some(other) = self
                    .rows
                    .iter()
                    .find(|r| r.key != key && r.id.as_deref() == Some(id.as_str()))
                {
                    let other = other.key;
                    self.rows.remove(at);
                    return Ok(other);
                }
                self.row_at_mut(at)?.id = Some(id);
            }
            (kind, Some(id)) => {
                if !shows(store, wall, (&note, kind), &id)?
                    || !Checklists::new(store).may_edit(&id)?
                {
                    return Ok(key);
                }
                match kind {
                    FieldOf::Checklist => Checklists::new(store).rename(&id, &typed)?,
                    FieldOf::Shopping => ShoppingLists::new(store).rename(&id, &typed)?,
                }
            }
        }
        self.row_at_mut(at)?.kept = typed;
        Ok(key)
    }

    /// Stores what was typed into every Line, as the editor saves.
    pub fn keep(&mut self, store: &Store, wall: &WallClock) -> oldbrainfix_core::Result<()> {
        let typed: Vec<u64> = self
            .rows
            .iter()
            .filter(|r| r.changed() && !r.typed().is_empty())
            .map(|r| r.key)
            .collect();
        if typed.is_empty() {
            return Ok(());
        }
        store.batch(|| {
            for key in &typed {
                // A row may have gone into another of the same name.
                if let Some(at) = self.at(*key) {
                    self.store_row(store, wall, at)?;
                }
            }
            Ok(())
        })
    }

    /// Stores what was typed, as the editor is left: the Lines left empty
    /// go.
    pub fn leave(&mut self, store: &Store, wall: &WallClock) -> oldbrainfix_core::Result<()> {
        self.keep(store, wall)?;
        let Some(note) = self.note.clone() else {
            return Ok(());
        };
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
                    self.delete_row(store, wall, &note, at)?;
                }
            }
            Ok(())
        })?;
        self.rows.retain(|r| !empty.contains(&r.key));
        Ok(())
    }

    /// Deletes the Line of the row at `at`, where this Device may: whether
    /// it may, or the row is new.
    fn delete_row(
        &self,
        store: &Store,
        wall: &WallClock,
        note: &str,
        at: usize,
    ) -> oldbrainfix_core::Result<bool> {
        let Some(id) = &self.row_at(at)?.id else {
            return Ok(true);
        };
        let Some(kind) = open_kind(store, note)? else {
            return Ok(true);
        };
        if !shows(store, wall, (note, kind), id)? {
            return Ok(true);
        }
        if !Checklists::new(store).may_edit(id)? {
            return Ok(false);
        }
        match kind {
            FieldOf::Checklist => Checklists::new(store).delete(id)?,
            FieldOf::Shopping => ShoppingLists::new(store).delete(id)?,
        }
        Ok(true)
    }

    /// The id of the Line shown under `shown`: a new one stored first,
    /// `None` while it holds nothing to store.
    pub fn stored_id(
        &mut self,
        store: &Store,
        wall: &WallClock,
        shown: &str,
    ) -> oldbrainfix_core::Result<Option<String>> {
        let Some(at) = self.rows.iter().position(|r| r.shown_id() == shown) else {
            return Ok(Some(shown.to_string()));
        };
        if let Some(id) = &self.row_at(at)?.id {
            return Ok(Some(id.clone()));
        }
        let holder = self.store_row(store, wall, at)?;
        Ok(self
            .at(holder)
            .and_then(|at| self.rows.get(at))
            .and_then(|row| row.id.clone()))
    }

    /// Does what a field asked.
    pub fn typing(
        &mut self,
        store: &Store,
        wall: &WallClock,
        asked: Typing,
    ) -> oldbrainfix_core::Result<()> {
        match asked {
            Typing::Split(key, at) => self.split(store, wall, key, at),
            Typing::Join(key) => self.join(store, wall, key),
            Typing::Indent(key, under) => self.indent(store, key, under),
            Typing::Store(key) => match self.at(key) {
                Some(at) => self.store_row(store, wall, at).map(|_| ()),
                None => Ok(()),
            },
        }
    }

    /// Splits the row `key` at the byte `byte` of its text: the text
    /// before stays, the text after becomes a new Line under it, an empty
    /// one at its end; a Line with a group gets it as the first of its
    /// group. At its start, an empty Line goes in above and the cursor
    /// stays.
    fn split(
        &mut self,
        store: &Store,
        wall: &WallClock,
        key: u64,
        byte: usize,
    ) -> oldbrainfix_core::Result<()> {
        let (Some(at), Some(note)) = (self.at(key), self.note.clone()) else {
            return Ok(());
        };
        let text = self.row_at(at)?.text.clone();
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
            let in_group = self.row_at(at)?.in_group;
            let row = self.row(None, "", in_group);
            self.rows.insert(at, row);
            self.focus = Some((key, 0));
            return Ok(());
        }
        let Some(kind) = open_kind(store, &note)? else {
            return Ok(());
        };
        let row = self.row_at(at)?;
        let group = kind == FieldOf::Checklist
            && (row.in_group || has_group(store, &note, row.id.as_deref())?);
        self.row_at_mut(at)?.text = before;
        store.batch(|| {
            let holder = self.store_row(store, wall, at)?;
            let Some(held) = self.at(holder) else {
                return Ok(());
            };
            if self.row_at(held)?.id.is_none() {
                // Not stored: the field keeps all that was typed.
                self.row_at_mut(held)?.text = text.clone();
                return Ok(());
            }
            // Gone into a Line of the same name: the new one goes where
            // it was.
            let into = if holder == key { held + 1 } else { at };
            if holder != key {
                self.focus = Some((holder, self.row_at(held)?.text.len()));
            }
            let row = if after.is_empty() {
                self.row(None, "", group)
            } else {
                let place = self.stored_before(held) + usize::from(self.places_among(held));
                let id = match kind {
                    FieldOf::Checklist => {
                        Checklists::new(store).add(&note, &after, Some(place), group)?
                    }
                    FieldOf::Shopping => {
                        ShoppingLists::new(store).add_at(&note, &after, wall, Some(place))?
                    }
                };
                // A name a Shopping List has already keeps its row.
                if self
                    .rows
                    .iter()
                    .any(|r| r.id.as_deref() == Some(id.as_str()))
                {
                    return Ok(());
                }
                self.row(Some(id), &after, group)
            };
            self.focus = Some((row.key, 0));
            self.rows.insert(into.min(self.rows.len()), row);
            Ok(())
        })
    }

    /// Joins the row `key` with the Line above at a Backspace at its
    /// start: its text goes to the end of the Line above, which keeps its
    /// box and state, and its Line is deleted, its checks staying in the
    /// log; a Checklist's group stays a group, as far as this Device may
    /// change it. An empty Line simply goes; under a Line of someone
    /// else's nothing is joined.
    fn join(&mut self, store: &Store, wall: &WallClock, key: u64) -> oldbrainfix_core::Result<()> {
        let (Some(at), Some(note)) = (self.at(key), self.note.clone()) else {
            return Ok(());
        };
        let checklists = Checklists::new(store);
        let text = self.row_at(at)?.typed();
        if text.is_empty() {
            return self.drop_row(store, wall, &note, at);
        }
        let Some(kind) = open_kind(store, &note)? else {
            return Ok(());
        };
        let Some(above) = at.checked_sub(1) else {
            return Ok(());
        };
        if let Some(upper) = &self.row_at(above)?.id
            && !checklists.may_edit(upper)?
        {
            return Ok(());
        }
        let head = self.row_at(above)?.typed();
        let mut group = Vec::new();
        if kind == FieldOf::Checklist
            && let Some(id) = &self.row_at(at)?.id
            && let Some(list) = checklists.by_note(&note)?
        {
            for line in list.group_of(id) {
                if checklists.may_edit(&line.id)? {
                    group.push(line.id.clone());
                }
            }
        }
        self.row_at_mut(above)?.text = format!("{head}{text}");
        let holder = store.batch(|| {
            let holder = self.store_row(store, wall, above)?;
            let joined = self
                .at(key)
                .and_then(|at| self.rows.get(at))
                .and_then(|row| row.id.clone());
            if let Some(id) = joined
                && shows(store, wall, (&note, kind), &id)?
                && checklists.may_edit(&id)?
            {
                match kind {
                    FieldOf::Checklist => {
                        checklists.delete(&id)?;
                        for line in &group {
                            checklists.indent(line, true)?;
                        }
                    }
                    FieldOf::Shopping => ShoppingLists::new(store).delete(&id)?,
                }
            }
            Ok(holder)
        })?;
        self.focus = Some((holder, head.len()));
        if let Some(at) = self.at(key) {
            self.rows.remove(at);
        }
        Ok(())
    }

    /// Takes the empty row at `at` away, deleting its Line; the cursor
    /// goes to the end of the Line above, or to the start of the next.
    /// The only row to type in stays, empty and new.
    fn drop_row(
        &mut self,
        store: &Store,
        wall: &WallClock,
        note: &str,
        at: usize,
    ) -> oldbrainfix_core::Result<()> {
        if !self.delete_row(store, wall, note, at)? {
            return Ok(());
        }
        let key = self.row_at(at)?.key;
        let checklists = Checklists::new(store);
        let mut others = false;
        for row in self.rows.iter().filter(|r| r.key != key) {
            others = match &row.id {
                None => true,
                Some(id) => checklists.may_edit(id)?,
            };
            if others {
                break;
            }
        }
        if self.adds && !others {
            let row = self.row_at_mut(at)?;
            row.id = None;
            row.text.clear();
            row.kept.clear();
            self.focus = Some((key, 0));
            return Ok(());
        }
        self.rows.remove(at);
        self.focus = match at.checked_sub(1).and_then(|above| self.rows.get(above)) {
            Some(above) => Some((above.key, above.text.len())),
            None => self.rows.first().map(|first| (first.key, 0)),
        };
        Ok(())
    }

    /// Puts the row `key` of a Checklist under the Line above, or with
    /// `under` false back at the top.
    fn indent(&mut self, store: &Store, key: u64, under: bool) -> oldbrainfix_core::Result<()> {
        let (Some(at), Some(note)) = (self.at(key), self.note.clone()) else {
            return Ok(());
        };
        if open_kind(store, &note)? != Some(FieldOf::Checklist) {
            return Ok(());
        }
        let Some(id) = self.row_at(at)?.id.clone() else {
            self.row_at_mut(at)?.in_group = under && at > 0;
            return Ok(());
        };
        let checklists = Checklists::new(store);
        if !checklists.may_edit(&id)? {
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

/// The error for a row looked for at `at` and not found.
fn no_row(at: usize) -> oldbrainfix_core::Error {
    oldbrainfix_core::Error::Invalid(format!("no typed Line at {at}"))
}

/// What the Note `note` is to write Lines to: a Checklist or a Shopping
/// List out of the trash, or `None`.
fn open_kind(store: &Store, note: &str) -> oldbrainfix_core::Result<Option<FieldOf>> {
    if trashed(store, note)? {
        return Ok(None);
    }
    if Checklists::new(store).is_checklist(note)? {
        return Ok(Some(FieldOf::Checklist));
    }
    if ShoppingLists::new(store).is_shopping_list(note)? {
        return Ok(Some(FieldOf::Shopping));
    }
    Ok(None)
}

fn trashed(store: &Store, note: &str) -> oldbrainfix_core::Result<bool> {
    Ok(Notes::new(store)
        .by_id(note)?
        .is_none_or(|n| n.is_trashed()))
}

/// Whether the Line `id` still shows on the list of `note`, of `kind`.
fn shows(
    store: &Store,
    wall: &WallClock,
    (note, kind): (&str, FieldOf),
    id: &str,
) -> oldbrainfix_core::Result<bool> {
    Ok(match kind {
        FieldOf::Checklist => Checklists::new(store)
            .by_note(note)?
            .is_some_and(|list| list.line(id).is_some()),
        FieldOf::Shopping => ShoppingLists::new(store)
            .by_note(note, wall)?
            .is_some_and(|list| list.line(id).is_some()),
    })
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
    /// A Checklist's Line, which Tab and Shift+Tab move too.
    Checklist,
    /// A Shopping List's Line, which has no group.
    Shopping,
}

/// The id of the field of the row `key`.
pub fn field_id(key: u64) -> Id {
    Id::new(("typed line", key))
}

/// The field of the row `key` of `lines`, `width` wide, struck through
/// once `checked`; `(above, below)` are the rows Up and Down go to. What
/// it asks goes to `asked`.
pub fn field(
    ui: &mut Ui,
    lines: &mut TypedLines,
    (key, kind): (u64, FieldOf),
    (checked, width): (bool, f32),
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
    // The keys before do not take rows away; were it gone, it shows no
    // more.
    let Some(row) = lines.rows.get_mut(at) else {
        return;
    };
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
            if let Some(row) = lines.rows.get(at) {
                format_bar::select(
                    ui,
                    id,
                    &row.text,
                    (byte.min(row.text.len()), byte.min(row.text.len())),
                );
            }
            ui.ctx().request_repaint();
        }
    }
    if response.lost_focus() {
        asked.push(Typing::Store(key));
    }
}

/// The keys a focused field takes before it: Enter, Backspace at its
/// start, on a Checklist Tab and Shift+Tab, Up and Down.
fn keys(
    ui: &mut Ui,
    lines: &mut TypedLines,
    (at, kind): (usize, FieldOf),
    (above, below): (Option<u64>, Option<u64>),
    asked: &mut Vec<Typing>,
) {
    let Some(row) = lines.rows.get(at) else {
        return;
    };
    let key = row.key;
    let id = field_id(key);
    let text = &row.text;
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
        // Over a selection, Enter takes its place.
        if let Some(row) = lines.rows.get_mut(at) {
            row.text.replace_range(from_byte..to_byte, "");
        }
        cursor_to(ui, from);
        asked.push(Typing::Split(key, from_byte));
        return;
    }
    if to == 0 && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace)) {
        asked.push(Typing::Join(key));
        return;
    }
    // Shift+Tab first: a plain Tab matches it too.
    if kind == FieldOf::Checklist && ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab)) {
        asked.push(Typing::Indent(key, false));
        return;
    }
    if kind == FieldOf::Checklist && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
        asked.push(Typing::Indent(key, true));
        return;
    }
    for (arrow, next) in [(Key::ArrowUp, above), (Key::ArrowDown, below)] {
        if let Some(next) = next
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, arrow))
            && let Some(row) = lines.at(next).and_then(|n| lines.rows.get(n))
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
