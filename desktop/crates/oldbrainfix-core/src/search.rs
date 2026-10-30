//! Finding Notes by what they say, as the Dart core's `Search` finds
//! them: their text, the Transcripts of their voice recordings, the Photo
//! Texts of their photos and the same of their Sections (Amends and
//! merged Notes), the text lines of the entries of a Tally and the Lines
//! of a Checklist or a Shopping List, but those gone. A text in a format
//! not known here counts as its source text.

use std::collections::HashMap;

use crate::Result;
use crate::checklists::{Checklists, keys as checklist_keys};
use crate::notes::{Note, Notes, keys};
use crate::shopping_lists::ShoppingLists;
use crate::store::Store;
use crate::tallies::keys as tally_keys;
use crate::text_format::StoredText;

pub struct Search<'a> {
    store: &'a Store,
}

impl<'a> Search<'a> {
    pub fn new(store: &'a Store) -> Search<'a> {
        Search { store }
    }

    /// The Notes whose text, Transcripts or Photo Texts, or those of one
    /// of their Sections, or the text line of an entry of their Tally, or
    /// a Line of their Checklist or Shopping List, hold `query`, ignoring
    /// case, newest capture first. The query counts as typed, inner spaces included; only its
    /// ends are trimmed, and a blank one finds everything. With `within`,
    /// only the Notes inside that Area or Topic count (an Area's Topics
    /// included).
    pub fn notes(&self, query: &str, within: Option<&str>) -> Result<Vec<Note>> {
        let notes = Notes::new(self.store);
        let scope = match within {
            Some(place) => notes.inside(place)?,
            None => notes.all()?,
        };
        let wanted = query.trim().to_lowercase();
        if wanted.is_empty() {
            return Ok(scope);
        }
        let sections = self.section_texts(&notes)?;
        let holds = |text: &String| text.to_lowercase().contains(&wanted);
        Ok(scope
            .into_iter()
            .filter(|note| {
                holds(&note.text)
                    || note.transcripts.values().any(holds)
                    || note.photo_texts.values().any(holds)
                    || sections.get(&note.id).is_some_and(|s| s.iter().any(holds))
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
                && let Some(into) = note.merged_into
            {
                let list = texts.entry(into).or_default();
                list.push(note.text);
                list.extend(note.transcripts.into_values());
                list.extend(note.photo_texts.into_values());
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
                .push(StoredText::read(fields.get(keys::TEXT)).text);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_query_finds_everything_and_case_does_not_count() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let a = notes.capture("Buy MILK", None, None)?;
        let b = notes.capture("Call mum", None, None)?;
        let search = Search::new(&store);
        let ids = |found: Vec<Note>| found.into_iter().map(|n| n.id).collect::<Vec<_>>();
        assert_eq!(
            ids(search.notes("  ", None)?),
            vec![b.id.clone(), a.id.clone()]
        );
        assert_eq!(ids(search.notes(" milk ", None)?), vec![a.id.clone()]);
        assert_eq!(ids(search.notes("buy  milk", None)?), Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn transcripts_photo_texts_and_sections_are_searched() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let voice = notes.capture_with("", None, None, &[], &[b"voice".to_vec()])?;
        notes.set_transcript(&voice.id, &voice.recordings[0], "Pick up the parcel")?;
        let photo = notes.capture_with("", None, None, &[b"photo".to_vec()], &[])?;
        notes.set_photo_text(&photo.id, &photo.photos[0], "RECEIPT 12,50")?;
        let target = notes.capture("Errands", None, None)?;
        let merged = notes.capture_with("", None, None, &[b"other".to_vec()], &[])?;
        notes.set_photo_text(&merged.id, &merged.photos[0], "Opening hours")?;
        notes.merge(&merged.id, &target.id)?;
        notes.amend(&target.id, "Bring the bags")?;
        let search = Search::new(&store);
        let ids = |q| -> Result<Vec<String>> {
            Ok(search.notes(q, None)?.into_iter().map(|n| n.id).collect())
        };
        assert_eq!(ids("parcel")?, vec![voice.id.clone()]);
        assert_eq!(ids("receipt")?, vec![photo.id.clone()]);
        assert_eq!(ids("opening")?, vec![target.id.clone()]);
        assert_eq!(ids("bags")?, vec![target.id.clone()]);
        assert!(
            notes
                .set_transcript(&photo.id, &photo.photos[0], "x")
                .is_err()
        );
        assert!(
            notes
                .set_photo_text(&voice.id, &voice.recordings[0], "x")
                .is_err()
        );
        Ok(())
    }
}
