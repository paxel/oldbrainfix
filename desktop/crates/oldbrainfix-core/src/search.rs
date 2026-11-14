//! Finding Notes by what they say, as the Dart core's `Search` finds
//! them: their text, the Transcripts of their voice recordings, the Photo
//! Texts of their photos, the names and Attachment Texts of their
//! attached files (ADR 0047) and the same of their Sections (Amends and
//! merged Notes), the text lines of the entries of a Tally and the Lines
//! of a Checklist or a Shopping List, but those gone. A formatted text
//! counts as it reads without its marks (ADR 0033), a text in a format
//! not known here as its source text.
//!
//! The full-text index (ADR 0048, [`crate::search_index`]) names the
//! entities that may hold a query, and only those are read; without it,
//! or for a query holding a box's glyph, which a tick changes, every Note
//! is read as before. Both find the same Notes.

use std::collections::{HashMap, HashSet};

use crate::Result;
use crate::checklists::{Checklists, keys as checklist_keys};
use crate::notes::{Note, Notes, keys};
use crate::places::Places;
use crate::search_index;
use crate::shopping_lists::ShoppingLists;
use crate::store::Store;
use crate::tallies::keys as tally_keys;
use crate::text_format::StoredText;

/// The glyphs of a box, which shows the state it is ticked to rather
/// than the one it was written with (ADR 0035).
const BOX_GLYPHS: [char; 3] = ['\u{2610}', '\u{2705}', '\u{274C}'];

/// The Notes read while searching, by id, so none is read twice.
type Read = HashMap<String, Option<Note>>;

pub struct Search<'a> {
    store: &'a Store,
    /// Whether the index is asked; else every Note is read.
    with_index: bool,
}

impl<'a> Search<'a> {
    pub fn new(store: &'a Store) -> Search<'a> {
        Search {
            store,
            with_index: true,
        }
    }

    /// A search that reads every Note, as before the index: what tests
    /// and measurements compare the index with.
    #[doc(hidden)]
    pub fn scanning(store: &'a Store) -> Search<'a> {
        Search {
            store,
            with_index: false,
        }
    }

    /// The Notes whose text, Transcripts, Photo Texts, file names or
    /// Attachment Texts, or those of one
    /// of their Sections, or the text line of an entry of their Tally, or
    /// a Line of their Checklist or Shopping List, hold `query`, ignoring
    /// case, newest capture first. The query counts as typed, inner spaces included; only its
    /// ends are trimmed, and a blank one finds everything. With `within`,
    /// only the Notes inside that Area or Topic count (an Area's Topics
    /// included).
    pub fn notes(&self, query: &str, within: Option<&str>) -> Result<Vec<Note>> {
        let notes = Notes::new(self.store);
        let wanted = lower_case(query.trim());
        let mut read = Read::new();
        let found = if wanted.is_empty() {
            None
        } else {
            self.indexed(&wanted, &mut read)?
        };
        let Some(found) = found else {
            let scope = match within {
                Some(place) => notes.inside(place)?,
                None => notes.all()?,
            };
            return if wanted.is_empty() {
                Ok(scope)
            } else {
                self.scanned(&wanted, scope)
            };
        };
        let places = match within {
            Some(place) => {
                let mut ids = HashSet::from([place.to_string()]);
                ids.extend(
                    Places::new(self.store)
                        .topics(place)?
                        .into_iter()
                        .map(|t| t.id),
                );
                Some(ids)
            }
            None => None,
        };
        // As [`Notes::all`] and [`Notes::inside`] list them.
        let mut hits = Vec::new();
        for id in found {
            let note = match read.remove(&id) {
                Some(note) => note,
                None => notes.by_id(&id)?,
            };
            if let Some(note) = note
                && !note.is_trashed()
                && !note.is_merged()
                && places
                    .as_ref()
                    .is_none_or(|ids| note.place.as_ref().is_some_and(|p| ids.contains(p)))
            {
                hits.push(note);
            }
        }
        if hits.len() < 2 {
            return Ok(hits);
        }
        let order: HashMap<String, usize> = self
            .store
            .ever_of_type(keys::TYPE_NOTE)?
            .into_iter()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        // Newest capture first; at the same time in the order the store
        // lists the Notes in, as the stable sort of [`Notes::all`] keeps
        // it.
        hits.sort_by(|a, b| {
            b.captured
                .cmp(&a.captured)
                .then_with(|| order.get(&a.id).cmp(&order.get(&b.id)))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(hits)
    }

    /// Those of `scope` that `query` finds, as [`Search::notes`] finds
    /// them, in the order given: for a list the search does not read
    /// itself, such as the trash (#274). A blank query keeps them all.
    pub fn among(&self, query: &str, scope: Vec<Note>) -> Result<Vec<Note>> {
        let wanted = lower_case(query.trim());
        if wanted.is_empty() {
            return Ok(scope);
        }
        match self.indexed(&wanted, &mut Read::new())? {
            Some(found) => Ok(scope
                .into_iter()
                .filter(|note| found.contains(&note.id))
                .collect()),
            None => self.scanned(&wanted, scope),
        }
    }

    /// The ids of the Notes `wanted`, in lower case and not blank, is
    /// found in, as [`Search::scanned`] finds them in a scope holding
    /// every Note: those of the candidates the index names, read as they
    /// are now. Each Note read is kept in `read`. `None` when the index
    /// cannot answer.
    fn indexed(&self, wanted: &str, read: &mut Read) -> Result<Option<HashSet<String>>> {
        if !self.with_index || wanted.contains(BOX_GLYPHS) {
            return Ok(None);
        }
        let Some(candidates) = search_index::candidates(self.store, wanted) else {
            return Ok(None);
        };
        let notes = Notes::new(self.store);
        let holds = |text: &str| lower_case(text).contains(wanted);
        // Read in one query per few hundred rather than one each (#204).
        let fields_of = self
            .store
            .current_fields_of(candidates.iter().map(String::as_str))?;
        let mut found = HashSet::new();
        let mut makings: HashMap<String, (Option<String>, bool)> = HashMap::new();
        for id in &candidates {
            let Some(fields) = fields_of.get(id) else {
                continue;
            };
            match fields.get(keys::TYPE) {
                Some(keys::TYPE_NOTE) => {
                    let Some(note) = note_of(&notes, read, id)? else {
                        continue;
                    };
                    if holds_own(&note, &holds) {
                        found.insert(id.clone());
                        found.extend(note.merged_into.clone());
                    }
                }
                Some(keys::TYPE_SECTION) => {
                    let Some(owner) = fields.get(keys::NOTE) else {
                        continue;
                    };
                    if holds(&StoredText::read(fields.get(keys::TEXT)).visible()) {
                        found.insert(shown_under(&notes, read, owner)?);
                    }
                }
                Some(tally_keys::TYPE_ENTRY) => {
                    let (Some(owner), Some(line)) =
                        (fields.get(tally_keys::NOTE), fields.get(tally_keys::LINE))
                    else {
                        continue;
                    };
                    if fields.get(tally_keys::TRASHED).is_none() && holds(line) {
                        found.insert(shown_under(&notes, read, owner)?);
                    }
                }
                Some(checklist_keys::TYPE_LINE) => {
                    let Some(owner) = fields.get(checklist_keys::NOTE) else {
                        continue;
                    };
                    if fields.get(checklist_keys::TRASHED).is_some() {
                        continue;
                    }
                    let (making, shopping) = match makings.get(owner) {
                        Some(held) => held.clone(),
                        None => {
                            let held = self.making(owner)?;
                            makings.insert(owner.to_string(), held.clone());
                            held
                        }
                    };
                    if making.is_none() || fields.get(checklist_keys::MADE) != making.as_deref() {
                        continue;
                    }
                    if shopping && ShoppingLists::is_gone(fields) {
                        continue;
                    }
                    if holds(&StoredText::read(fields.get(checklist_keys::TEXT)).text) {
                        found.insert(shown_under(&notes, read, owner)?);
                    }
                }
                _ => {}
            }
        }
        Ok(Some(found))
    }

    /// The making of the Checklist or Shopping List of the Note `owner`
    /// whose Lines count now, and whether it is a Shopping List.
    fn making(&self, owner: &str) -> Result<(Option<String>, bool)> {
        Ok(match Checklists::new(self.store).by_note(owner)? {
            Some(list) => (Some(list.making), false),
            None => (ShoppingLists::new(self.store).making(owner)?, true),
        })
    }

    /// Those of `scope` `wanted`, in lower case and not blank, is found
    /// in, each Note read in full: the search without the index.
    fn scanned(&self, wanted: &str, scope: Vec<Note>) -> Result<Vec<Note>> {
        let notes = Notes::new(self.store);
        let sections = self.section_texts(&notes)?;
        let holds = |text: &str| lower_case(text).contains(wanted);
        Ok(scope
            .into_iter()
            .filter(|note| {
                holds_own(note, &holds)
                    || sections
                        .get(&note.id)
                        .is_some_and(|s| s.iter().any(|t| holds(t)))
            })
            .collect())
    }

    /// The texts of every Section and Tally entry, by the Note they show
    /// under, read in one pass: a merged Note's text, Transcripts and
    /// Photo Texts under the end of its Merge chain, an Amend or the text
    /// line of an entry not removed under its Note, or where that Note
    /// was merged into.
    fn section_texts(&self, notes: &Notes) -> Result<HashMap<String, Vec<String>>> {
        let mut texts: HashMap<String, Vec<String>> = HashMap::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = notes.by_id(&id)?
                && let Some(into) = note.merged_into.clone()
            {
                let list = texts.entry(into).or_default();
                list.push(note.visible_text());
                list.extend(note.transcripts.into_values());
                list.extend(note.photo_texts.into_values());
                list.extend(note.attachment_texts.into_values());
                list.extend(note.attachments.into_iter().map(|a| a.name));
            }
        }
        for id in self.store.entities_of_type(keys::TYPE_SECTION)? {
            let fields = self.store.current_fields(&id)?;
            let Some(owner) = fields.get(keys::NOTE) else {
                continue;
            };
            let shown_under = notes
                .by_id(owner)?
                .and_then(|n| n.merged_into)
                .unwrap_or_else(|| owner.to_string());
            texts
                .entry(shown_under)
                .or_default()
                .push(StoredText::read(fields.get(keys::TEXT)).visible());
        }
        for id in self.store.entities_of_type(tally_keys::TYPE_ENTRY)? {
            let fields = self.store.current_fields(&id)?;
            let (Some(owner), Some(line)) =
                (fields.get(tally_keys::NOTE), fields.get(tally_keys::LINE))
            else {
                continue;
            };
            if fields.get(tally_keys::TRASHED).is_some() {
                continue;
            }
            let shown_under = notes
                .by_id(owner)?
                .and_then(|n| n.merged_into)
                .unwrap_or_else(|| owner.to_string());
            texts.entry(shown_under).or_default().push(line.to_string());
        }
        let mut makings: HashMap<String, (Option<String>, bool)> = HashMap::new();
        for id in self.store.entities_of_type(checklist_keys::TYPE_LINE)? {
            let fields = self.store.current_fields(&id)?;
            let Some(owner) = fields.get(checklist_keys::NOTE) else {
                continue;
            };
            if fields.get(checklist_keys::TRASHED).is_some() {
                continue;
            }
            let (making, shopping) = match makings.get(owner) {
                Some(found) => found.clone(),
                None => {
                    let found = match Checklists::new(self.store).by_note(owner)? {
                        Some(list) => (Some(list.making), false),
                        None => (ShoppingLists::new(self.store).making(owner)?, true),
                    };
                    makings.insert(owner.to_string(), found.clone());
                    found
                }
            };
            if making.is_none() || fields.get(checklist_keys::MADE) != making.as_deref() {
                continue;
            }
            if shopping && ShoppingLists::is_gone(&fields) {
                continue;
            }
            let shown_under = notes
                .by_id(owner)?
                .and_then(|n| n.merged_into)
                .unwrap_or_else(|| owner.to_string());
            texts
                .entry(shown_under)
                .or_default()
                .push(StoredText::read(fields.get(checklist_keys::TEXT)).text);
        }
        Ok(texts)
    }
}

/// Whether the Note's own text, Transcripts, Photo Texts, Attachment
/// Texts or file names `holds` the query.
fn holds_own(note: &Note, holds: &impl Fn(&str) -> bool) -> bool {
    holds(&note.visible_text())
        || note.transcripts.values().any(|t| holds(t))
        || note.photo_texts.values().any(|t| holds(t))
        || note.attachment_texts.values().any(|t| holds(t))
        || note.attachments.iter().any(|a| holds(&a.name))
}

/// The Note `id`, read once.
fn note_of(notes: &Notes, read: &mut Read, id: &str) -> Result<Option<Note>> {
    if let Some(held) = read.get(id) {
        return Ok(held.clone());
    }
    let note = notes.by_id(id)?;
    read.insert(id.to_string(), note.clone());
    Ok(note)
}

/// Where what names the Note `owner` shows: at the end of its Merge
/// chain, or under it.
fn shown_under(notes: &Notes, read: &mut Read, owner: &str) -> Result<String> {
    Ok(note_of(notes, read, owner)?
        .and_then(|n| n.merged_into)
        .unwrap_or_else(|| owner.to_string()))
}

/// `text` in lower case letter by letter, as the Dart core's
/// `toLowerCase` lowers it: each character to the first of its lower
/// case, so "İ" becomes "i" and a closing "Σ" "σ", where
/// `str::to_lowercase` writes "i̇" and "ς".
pub(crate) fn lower_case(text: &str) -> String {
    text.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect()
}

#[cfg(test)]
mod tests;
