//! The Notes that link to a Note (obf-1035, ADR 0046), as the Dart
//! core's `Backlinks` reads them: those neither in the trash nor merged
//! whose text, or the text of one of their Sections, holds an app link to
//! it, `oldbrainfix://note/<id>`, in a formatted text as it is drawn.
//! Nothing is stored for it; it is read from the texts and kept until the
//! log changes, the full-text index (ADR 0048) naming the texts that may
//! hold one, so a change reads only those. The Notes filed into an Area
//! of `closed`, the locked Areas not open now, are left out, as from
//! every list (Area Locks); a Note of a Shared Area the reader is no
//! member of is none this store reads (ADR 0013).

use std::collections::{HashMap, HashSet};

use crate::Result;
use crate::app_link::{self, AppLink, SCHEME};
use crate::markdown::{self, Span, SpanKind};
use crate::notes::{Note, Notes, keys};
use crate::places::Places;
use crate::search_index;
use crate::store::{Store, utf16_cmp};
use crate::text_format;

pub struct Backlinks<'a> {
    store: &'a Store,
}

impl<'a> Backlinks<'a> {
    pub fn new(store: &'a Store) -> Backlinks<'a> {
        Backlinks { store }
    }

    /// The Notes that link to the Note `note_id`, newest capture first
    /// and by id within one capture time, each once; never `note_id`
    /// itself. Those filed into an Area of `closed` are left out.
    pub fn to(&self, note_id: &str, closed: &HashSet<String>) -> Result<Vec<Note>> {
        let read = self.store.worked_out("backlinks", || self.read())?;
        Ok(read
            .get(note_id)
            .into_iter()
            .flatten()
            .filter(|l| l.area.as_ref().is_none_or(|a| !closed.contains(a)))
            .map(|l| l.note.clone())
            .collect())
    }

    /// The Notes linking to each Note, by the id linked to, with the Area
    /// each is filed into, whatever is closed: kept until the log changes.
    fn read(&self) -> Result<HashMap<String, Vec<Linking>>> {
        let notes = Notes::new(self.store);
        let places = Places::new(self.store);
        let mut listed: Vec<Note> = self
            .maybe_linking(&notes)?
            .into_iter()
            .filter(|n| !n.is_trashed() && !n.is_merged())
            .collect();
        listed.sort_by(|a, b| {
            b.captured
                .cmp(&a.captured)
                .then_with(|| utf16_cmp(&a.id, &b.id))
        });
        let mut out: HashMap<String, Vec<Linking>> = HashMap::new();
        for note in listed {
            let mut targets = links_in(&note.text_format, &note.shown_text());
            for section in notes.sections(&note.id)? {
                targets.extend(links_in(&section.text_format, &section.shown_text()));
            }
            targets.remove(&note.id);
            if targets.is_empty() {
                continue;
            }
            let area = match &note.place {
                Some(place) => Some(match places.topic_by_id(place)? {
                    Some(topic) => topic.area_id,
                    None => place.clone(),
                }),
                None => None,
            };
            for target in targets {
                out.entry(target).or_default().push(Linking {
                    note: note.clone(),
                    area: area.clone(),
                });
            }
        }
        Ok(out)
    }

    /// The Notes whose text or Sections may hold an app link: those the
    /// full-text index names as holding the scheme once, by their own
    /// text or a Section's, and those they were merged into; every Note
    /// when the index cannot answer. Some may hold none, and none is
    /// missed.
    fn maybe_linking(&self, notes: &Notes<'_>) -> Result<Vec<Note>> {
        let Some(found) = search_index::candidates(self.store, SCHEME) else {
            return notes.all();
        };
        let mut ids: HashSet<String> = HashSet::new();
        let mut add = |id: &str| -> Result<()> {
            if !ids.insert(id.to_string()) {
                return Ok(());
            }
            if let Some(into) = notes.by_id(id)?.and_then(|n| n.merged_into) {
                ids.insert(into);
            }
            Ok(())
        };
        let fields_of = self
            .store
            .current_fields_of(found.iter().map(String::as_str))?;
        for id in &found {
            let Some(fields) = fields_of.get(id) else {
                continue;
            };
            match fields.get(keys::TYPE) {
                Some(keys::TYPE_NOTE) => add(id)?,
                Some(keys::TYPE_SECTION) => {
                    if let Some(owner) = fields.get(keys::NOTE) {
                        add(owner)?;
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for id in &ids {
            if let Some(note) = notes.by_id(id)? {
                out.push(note);
            }
        }
        Ok(out)
    }
}

/// The ids of the Notes `text` in `format` links to, as it is drawn: only
/// a formatted text has links; in it every link whose destination
/// [`app_link::parse`] reads as a Note's, in a line, a table's cell,
/// bold, italic or struck through.
pub fn links_in(format: &str, text: &str) -> HashSet<String> {
    fn walk(spans: &[Span], ids: &mut HashSet<String>) {
        for span in spans {
            if span.kind == SpanKind::Link
                && let Some(AppLink::Note(id)) = app_link::parse(&span.url)
            {
                ids.insert(id);
            }
            walk(&span.children, ids);
        }
    }
    let mut ids = HashSet::new();
    if format != text_format::MARKDOWN {
        return ids;
    }
    for block in markdown::parse(text) {
        walk(&block.spans, &mut ids);
    }
    ids
}

/// A Note linking to another, and the Area it is filed into, directly or
/// through a Topic.
struct Linking {
    note: Note,
    area: Option<String>,
}

#[cfg(test)]
mod tests;
