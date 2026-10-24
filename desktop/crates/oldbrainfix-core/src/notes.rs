//! Notes and their Sections as the Dart core's `Notes` reads and writes
//! them: the text, the capture time, the Importance, where a Note is
//! filed, whether it is in the trash, and what was merged into it. Every
//! write appends the same entries, in the same order and read from the
//! clock as often, as the Dart core's, so both logs come out alike.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::places::Places;
use crate::store::{Fields, Store};
use crate::{Error, Result};

/// Field keys of a Note.
pub mod keys {
    pub const TYPE: &str = "$type";
    pub const TYPE_NOTE: &str = "note";
    pub const TEXT: &str = "text";
    pub const CAPTURED: &str = "captured";
    /// The Area or Topic the Note is filed in; absent while Unsorted.
    pub const PLACE: &str = "place";
    /// The Note's Importance by name; absent means normal.
    pub const IMPORTANCE: &str = "importance";
    /// When the Note was moved to the trash; absent while it is not there.
    pub const TRASHED: &str = "trashed";
    /// The Note this one was merged into; absent while it stands alone.
    pub const MERGED: &str = "merged";
    /// When the Note first spawned a Routine or Reminder.
    pub const SPAWNED: &str = "spawned";
    /// When the Note's Nudge is due, chosen at Capture.
    pub const NUDGE: &str = "nudge";

    /// Field keys of a Section.
    pub const TYPE_SECTION: &str = "section";
    /// The Note a Section belongs to.
    pub const NOTE: &str = "note";
}

/// How much a Note matters (CONTEXT.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Importance {
    Unimportant,
    #[default]
    Normal,
    Important,
}

impl Importance {
    /// The name the log stores.
    pub fn name(self) -> &'static str {
        match self {
            Importance::Unimportant => "unimportant",
            Importance::Normal => "normal",
            Importance::Important => "important",
        }
    }

    /// The Importance stored as `name`; normal for anything else.
    pub fn from_name(name: Option<&str>) -> Importance {
        match name {
            Some("unimportant") => Importance::Unimportant,
            Some("important") => Importance::Important,
            _ => Importance::Normal,
        }
    }
}

/// A Note as the screens want it (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Entity id, `note:<id>`.
    pub id: String,
    pub text: String,
    pub captured: DateTime<Utc>,
    /// The Area or Topic the Note is filed in, or `None` when it has
    /// none or that place is gone.
    pub place: Option<String>,
    pub importance: Importance,
    /// When the Note was moved to the trash.
    pub trashed: Option<DateTime<Utc>>,
    /// The Note this one was merged into and now is a Section of.
    pub merged_into: Option<String>,
    /// Whether the Note ever spawned a Routine or Reminder.
    pub spawned: bool,
    /// When its Nudge is due, as chosen at Capture; `None` when none was.
    pub nudge: Option<DateTime<Utc>>,
}

impl Note {
    pub fn is_trashed(&self) -> bool {
        self.trashed.is_some()
    }

    pub fn is_merged(&self) -> bool {
        self.merged_into.is_some()
    }

    /// Not filed, not merged, not deleted and without a Routine or
    /// Reminder spawned from it (CONTEXT.md).
    pub fn is_unsorted(&self) -> bool {
        self.place.is_none() && !self.is_trashed() && !self.is_merged() && !self.spawned
    }

    /// When the Nudge for this Note is due: as chosen at Capture, else
    /// [`Notes::default_nudge`] after it.
    pub fn nudge_at(&self) -> DateTime<Utc> {
        self.nudge.unwrap_or(self.captured + Notes::default_nudge())
    }
}

/// A dated part of a Note, added by Amend or Merge (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// `section:<id>` for an Amend, the merged Note's `note:<id>` for a
    /// Merge.
    pub id: String,
    pub note_id: String,
    pub text: String,
    pub captured: DateTime<Utc>,
}

/// Reading Notes.
pub struct Notes<'a> {
    store: &'a Store,
}

impl<'a> Notes<'a> {
    /// How long a deleted Note stays in the trash before it is purged.
    pub fn trash_keep() -> Duration {
        Duration::days(30)
    }

    /// How long after Capture the Nudge comes when none was picked (UI
    /// laws, "Capture").
    pub fn default_nudge() -> Duration {
        Duration::hours(1)
    }

    pub fn new(store: &'a Store) -> Notes<'a> {
        Notes { store }
    }

    /// Captures a new Note at the store's now with `text`, filed into the
    /// Area or Topic `place` when one is given, nudged `nudge_in` from now
    /// while it is Unsorted. The text is kept as given.
    pub fn capture(
        &self,
        text: &str,
        place: Option<&str>,
        nudge_in: Option<Duration>,
    ) -> Result<Note> {
        if let Some(place) = place
            && !Places::new(self.store).is_place(place)?
        {
            return Err(Error::Invalid(format!("not an Area or Topic: {place}")));
        }
        let id = format!("note:{}", self.store.new_id()?);
        let now = self.store.now();
        let at = Some(now);
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_NOTE), at)?;
            s.append(&id, keys::CAPTURED, Some(&dart_iso(now)), at)?;
            s.append(&id, keys::TEXT, Some(text), at)?;
            if let Some(place) = place {
                s.append(&id, keys::PLACE, Some(place), at)?;
            }
            if let Some(nudge_in) = nudge_in {
                s.append(&id, keys::NUDGE, Some(&dart_iso(now + nudge_in)), at)?;
            }
            Ok(())
        })?;
        self.by_id(&id)?
            .ok_or_else(|| Error::Invalid("the captured Note is gone".into()))
    }

    /// Files the Note `note_id` into the Area or Topic `place_id`, out of
    /// wherever it was before.
    pub fn file(&self, note_id: &str, place_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        if !Places::new(self.store).is_place(place_id)? {
            return Err(Error::Invalid(format!("not an Area or Topic: {place_id}")));
        }
        self.store
            .append(note_id, keys::PLACE, Some(place_id), None)
            .map(drop)
    }

    /// Fixes the text of the Note `note_id` in place. The old text stays
    /// in the log only.
    pub fn edit(&self, note_id: &str, text: &str) -> Result<()> {
        self.require_note(note_id)?;
        let text = Self::text(text)?;
        self.store
            .append(note_id, keys::TEXT, Some(text), None)
            .map(drop)
    }

    /// Amends the Note `note_id` with a new Section holding `text`,
    /// captured at the store's now.
    pub fn amend(&self, note_id: &str, text: &str) -> Result<Section> {
        self.require_note(note_id)?;
        let text = Self::text(text)?;
        let id = format!("section:{}", self.store.new_id()?);
        let now = self.store.now();
        let at = Some(now);
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_SECTION), at)?;
            s.append(&id, keys::NOTE, Some(note_id), at)?;
            s.append(&id, keys::CAPTURED, Some(&dart_iso(now)), at)?;
            s.append(&id, keys::TEXT, Some(text), at)?;
            Ok(())
        })?;
        let fields = self.store.current_fields(&id)?;
        Self::section(&id, note_id, &fields)?
            .ok_or_else(|| Error::Invalid("the new Section is gone".into()))
    }

    /// Merges the Note `note_id` into the Note `into_id`: it becomes a
    /// dated Section of it and leaves every list and Unsorted. Nothing is
    /// copied; the merged Note only points at its target.
    pub fn merge(&self, note_id: &str, into_id: &str) -> Result<()> {
        for id in [note_id, into_id] {
            let listed = self
                .by_id(id)?
                .is_some_and(|n| !n.is_trashed() && !n.is_merged());
            if !listed {
                return Err(Error::Invalid(format!("not a Note in the lists: {id}")));
            }
        }
        if note_id == into_id {
            return Err(Error::Invalid("a Note cannot merge into itself".into()));
        }
        self.store
            .append(note_id, keys::MERGED, Some(into_id), None)
            .map(drop)
    }

    /// Sets the Importance of the Note `note_id`.
    pub fn set_importance(&self, note_id: &str, importance: Importance) -> Result<()> {
        self.require_note(note_id)?;
        self.store
            .append(note_id, keys::IMPORTANCE, Some(importance.name()), None)
            .map(drop)
    }

    /// Moves the Note `note_id` to the trash: it leaves every list and
    /// Unsorted, and can be restored until [`Notes::trash_keep`] has
    /// passed.
    pub fn trash(&self, note_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        // The time is read on its own before the entry reads it again, as
        // in the Dart core.
        let at = dart_iso(self.store.now());
        self.store
            .append(note_id, keys::TRASHED, Some(&at), None)
            .map(drop)
    }

    /// Brings the Note `note_id` back from the trash, to where it was
    /// filed.
    pub fn restore(&self, note_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        self.store
            .append(note_id, keys::TRASHED, None, None)
            .map(drop)
    }

    fn require_note(&self, id: &str) -> Result<()> {
        match self.by_id(id)? {
            Some(_) => Ok(()),
            None => Err(Error::Invalid(format!("not a Note: {id}"))),
        }
    }

    /// `text` trimmed; an error when nothing is left.
    fn text(text: &str) -> Result<&str> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(Error::Invalid("the text must not be empty".into()));
        }
        Ok(trimmed)
    }

    /// The Note `id`, trashed or not, or `None` when there is none.
    pub fn by_id(&self, id: &str) -> Result<Option<Note>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_NOTE) {
            return Ok(None);
        }
        let Some(captured) = fields.get(keys::CAPTURED) else {
            return Ok(None);
        };
        // A place that is gone leaves the Note Unsorted.
        let place = match fields.get(keys::PLACE) {
            Some(p) if Places::new(self.store).is_place(p)? => Some(p.to_string()),
            _ => None,
        };
        Ok(Some(Note {
            id: id.to_string(),
            text: fields.get(keys::TEXT).unwrap_or_default().to_string(),
            captured: parse_time(captured)?,
            place,
            importance: Importance::from_name(fields.get(keys::IMPORTANCE)),
            trashed: fields.get(keys::TRASHED).map(parse_time).transpose()?,
            merged_into: self.merged_into(id)?,
            spawned: fields.get(keys::SPAWNED).is_some(),
            nudge: fields.get(keys::NUDGE).and_then(|n| parse_time(n).ok()),
        }))
    }

    /// Every Note neither in the trash nor merged, newest capture first.
    pub fn all(&self) -> Result<Vec<Note>> {
        let mut notes = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = self.by_id(&id)?
                && !note.is_trashed()
                && !note.is_merged()
            {
                notes.push(note);
            }
        }
        notes.sort_by_key(|n| std::cmp::Reverse(n.captured));
        Ok(notes)
    }

    /// Every Unsorted Note, oldest capture first.
    pub fn unsorted(&self) -> Result<Vec<Note>> {
        Ok(self
            .all()?
            .into_iter()
            .rev()
            .filter(Note::is_unsorted)
            .collect())
    }

    /// The Notes filed in the Area or Topic `place_id`, newest capture
    /// first; an Area holds its Topics' Notes too.
    pub fn inside(&self, place_id: &str) -> Result<Vec<Note>> {
        let mut ids: HashSet<String> = HashSet::from([place_id.to_string()]);
        ids.extend(
            Places::new(self.store)
                .topics(place_id)?
                .into_iter()
                .map(|t| t.id),
        );
        Ok(self
            .all()?
            .into_iter()
            .filter(|n| n.place.as_ref().is_some_and(|p| ids.contains(p)))
            .collect())
    }

    /// The Notes in the trash that can still be restored, most recently
    /// deleted first.
    pub fn trashed(&self) -> Result<Vec<Note>> {
        let now = self.store.now();
        let mut notes = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = self.by_id(&id)?
                && !note.is_merged()
                && let Some(at) = note.trashed
                && now < at + Self::trash_keep()
            {
                notes.push(note);
            }
        }
        notes.sort_by(|a, b| {
            let a = a.trashed.unwrap_or(a.captured);
            let b = b.trashed.unwrap_or(b.captured);
            b.cmp(&a)
        });
        Ok(notes)
    }

    /// The Sections of the Note `note_id`, oldest first: its Amends, the
    /// Notes merged into it, and their Amends.
    pub fn sections(&self, note_id: &str) -> Result<Vec<Section>> {
        let mut merged: Vec<String> = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if self.merged_into(&id)?.as_deref() == Some(note_id) && !merged.contains(&id) {
                merged.push(id);
            }
        }
        let mut sections = Vec::new();
        for id in &merged {
            if let Some(note) = self.by_id(id)? {
                sections.push(Section {
                    id: id.clone(),
                    note_id: note_id.to_string(),
                    text: note.text,
                    captured: note.captured,
                });
            }
        }
        for id in self.store.entities_of_type(keys::TYPE_SECTION)? {
            let owner = self.store.current(&id, keys::NOTE)?;
            let owned = owner
                .as_deref()
                .is_some_and(|o| o == note_id || merged.iter().any(|m| m == o));
            if owned {
                let fields = self.store.current_fields(&id)?;
                sections.extend(Self::section(&id, note_id, &fields)?);
            }
        }
        sections.sort_by_key(|s| s.captured);
        Ok(sections)
    }

    /// The Section `id` from its `fields`, shown under `note_id`.
    fn section(id: &str, note_id: &str, fields: &Fields) -> Result<Option<Section>> {
        if fields.get(keys::TYPE) != Some(keys::TYPE_SECTION) {
            return Ok(None);
        }
        let (Some(_), Some(captured)) = (fields.get(keys::NOTE), fields.get(keys::CAPTURED)) else {
            return Ok(None);
        };
        Ok(Some(Section {
            id: id.to_string(),
            note_id: note_id.to_string(),
            text: fields.get(keys::TEXT).unwrap_or_default().to_string(),
            captured: parse_time(captured)?,
        }))
    }

    /// The Note `id` ends up in at the end of its Merge chain, or `None`
    /// when it stands alone. A chain stops before a Note this Device does
    /// not hold. A loop stops at its lowest id, so every Device keeps the
    /// same Note.
    fn merged_into(&self, id: &str) -> Result<Option<String>> {
        let mut chain = vec![id.to_string()];
        loop {
            let Some(last) = chain.last() else {
                return Ok(None);
            };
            let Some(next) = self.store.current(last, keys::MERGED)? else {
                break;
            };
            if self.store.current(&next, keys::TYPE)?.as_deref() != Some(keys::TYPE_NOTE) {
                break;
            }
            if let Some(start) = chain.iter().position(|c| *c == next) {
                let kept = chain[start..]
                    .iter()
                    .min_by(|a, b| crate::store::utf16_cmp(a, b))
                    .cloned();
                return Ok(kept.filter(|k| k != id));
            }
            chain.push(next);
        }
        Ok(chain.last().filter(|last| *last != id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importance_reads_its_stored_name_and_defaults_to_normal() {
        for i in [
            Importance::Unimportant,
            Importance::Normal,
            Importance::Important,
        ] {
            assert_eq!(Importance::from_name(Some(i.name())), i);
        }
        assert_eq!(Importance::from_name(None), Importance::Normal);
        assert_eq!(Importance::from_name(Some("huge")), Importance::Normal);
    }

    #[test]
    fn writes_check_what_they_write_to() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        assert!(notes.capture("x", Some("area:none"), None).is_err());
        let a = notes.capture("  kept as typed ", None, None)?;
        assert_eq!(a.text, "  kept as typed ");
        assert!(notes.edit(&a.id, "   ").is_err());
        assert!(notes.edit("note:none", "x").is_err());
        assert!(notes.amend(&a.id, "").is_err());
        assert!(notes.file(&a.id, "topic:none").is_err());
        assert!(notes.merge(&a.id, &a.id).is_err());
        assert!(notes.merge(&a.id, "note:none").is_err());
        notes.edit(&a.id, " fixed ")?;
        assert_eq!(notes.by_id(&a.id)?.map(|n| n.text), Some("fixed".into()));
        let b = notes.capture("other", None, Some(Duration::minutes(10)))?;
        notes.trash(&b.id)?;
        assert!(notes.merge(&a.id, &b.id).is_err());
        notes.restore(&b.id)?;
        notes.merge(&a.id, &b.id)?;
        assert_eq!(notes.sections(&b.id)?.len(), 1);
        assert_eq!(
            store.current(&b.id, keys::NUDGE)?.as_deref(),
            Some("2026-01-01T10:10:06.000Z")
        );
        Ok(())
    }

    #[test]
    fn a_note_without_a_capture_time_is_no_note() -> Result<()> {
        let store = crate::store::tests::store();
        store.append("note:a", keys::TYPE, Some(keys::TYPE_NOTE), None)?;
        assert_eq!(Notes::new(&store).by_id("note:a")?, None);
        store.append(
            "note:a",
            keys::CAPTURED,
            Some("2026-01-01T10:00:00.000Z"),
            None,
        )?;
        let note = Notes::new(&store).by_id("note:a")?;
        assert!(note.is_some_and(|n| n.is_unsorted() && n.text.is_empty()));
        Ok(())
    }
}
