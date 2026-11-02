//! A Note's History (#193), as the Dart core's `note_history.dart`:
//! every change of the Note, its Amends and the Notes merged into it,
//! and the Lines of its lists, read from the log alone, newest first,
//! each with who made it and when.

use chrono::{DateTime, Utc};

use crate::checklists::{keys as line_keys, parse_list_kind};
use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::notes::{Notes, keys};
use crate::profile::Profile;
use crate::shopping_lists::keys as shopping_keys;
use crate::store::Store;
use crate::text_boxes;
use crate::text_format::StoredText;

/// What a change in a Note's History did, as the Dart core's
/// `NoteChangeKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteChangeKind {
    /// The Note was captured with `after` as its text.
    Captured,
    /// The Note's text went from `before` to `after`.
    Text,
    /// The Note's text was put in another format, its words the same:
    /// `before` and `after` are the formats.
    Format,
    /// An Amend added a Section of the text `after`.
    Amended,
    /// The text of an Amend, or of a Note merged in, went from `before`
    /// to `after`.
    AmendEdited,
    /// A Note was merged in; `after` is its text now.
    MergedIn,
    /// A Line was added, of the text `after`.
    LineAdded,
    /// A Line was renamed from `before` to `after`.
    LineRenamed,
    /// The Line `after` was checked.
    LineChecked,
    /// The Line `after` was unchecked.
    LineUnchecked,
    /// The Line `after` was deleted.
    LineRemoved,
    /// The deleted Line `after` was brought back.
    LineRestored,
    /// The Importance went from `before` to `after`, by name; none is
    /// normal.
    Importance,
    /// The Note was filed from the Area or Topic `before` into `after`,
    /// by id; none is Unsorted.
    Filed,
    /// The Note became another kind of list: `checklist`, `shopping` or
    /// none for a plain Note.
    Kind,
    /// The Note went to the trash.
    Trashed,
    /// The Note came back from the trash.
    Restored,
    PhotoAdded,
    PhotoRemoved,
    RecordingAdded,
    RecordingRemoved,
}

impl NoteChangeKind {
    /// Its name, as the Dart core's enum writes it.
    pub fn name(self) -> &'static str {
        match self {
            NoteChangeKind::Captured => "captured",
            NoteChangeKind::Text => "text",
            NoteChangeKind::Format => "format",
            NoteChangeKind::Amended => "amended",
            NoteChangeKind::AmendEdited => "amendEdited",
            NoteChangeKind::MergedIn => "mergedIn",
            NoteChangeKind::LineAdded => "lineAdded",
            NoteChangeKind::LineRenamed => "lineRenamed",
            NoteChangeKind::LineChecked => "lineChecked",
            NoteChangeKind::LineUnchecked => "lineUnchecked",
            NoteChangeKind::LineRemoved => "lineRemoved",
            NoteChangeKind::LineRestored => "lineRestored",
            NoteChangeKind::Importance => "importance",
            NoteChangeKind::Filed => "filed",
            NoteChangeKind::Kind => "kind",
            NoteChangeKind::Trashed => "trashed",
            NoteChangeKind::Restored => "restored",
            NoteChangeKind::PhotoAdded => "photoAdded",
            NoteChangeKind::PhotoRemoved => "photoRemoved",
            NoteChangeKind::RecordingAdded => "recordingAdded",
            NoteChangeKind::RecordingRemoved => "recordingRemoved",
        }
    }

    /// Whether `after` is a text the change set: one "Use this version"
    /// can bring back.
    pub fn sets_text(self) -> bool {
        matches!(
            self,
            NoteChangeKind::Captured
                | NoteChangeKind::Text
                | NoteChangeKind::Amended
                | NoteChangeKind::AmendEdited
        )
    }
}

/// One change in a Note's History.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteChange {
    /// When it was made: its entry's date.
    pub at: DateTime<Utc>,
    /// What it changed: the Note, a Section of it (`section:` for an
    /// Amend, `note:` for a Note merged in) or a Line (`line:`).
    pub entity: String,
    pub kind: NoteChangeKind,
    /// The value before and after, as `kind` says: a text as it shows,
    /// each box in the glyph it was typed with and without its id.
    pub before: Option<String>,
    pub after: Option<String>,
    /// The format of a text `after`: what "Use this version" writes it
    /// in; none for a text in a format or structure not known here,
    /// which no version here may write.
    pub format: Option<String>,
    /// Who made it, as their Profile shows ([`Store::editor_of`]); none
    /// for a writer not known and a blocked person.
    pub by: Option<Profile>,
}

/// Reads a Note's History and brings back an old text of it.
pub struct NoteHistory<'a> {
    store: &'a Store,
}

impl<'a> NoteHistory<'a> {
    pub fn new(store: &'a Store) -> Self {
        NoteHistory { store }
    }

    /// Every change of the Note `note_id`, newest first by latest wins,
    /// so every Device lists them alike; empty when there is no such
    /// Note. Only what counts is read ([`Store::history`]).
    pub fn of(&self, note_id: &str) -> Result<Vec<NoteChange>> {
        let notes = Notes::new(self.store);
        if notes.by_id(note_id)?.is_none() {
            return Ok(Vec::new());
        }
        let mut found: Vec<(Entry, NoteChange)> = Vec::new();
        let add = |found: &mut Vec<(Entry, NoteChange)>,
                   e: &Entry,
                   kind: NoteChangeKind,
                   before: Option<String>,
                   after: Option<String>,
                   format: Option<String>|
         -> Result<()> {
            let by = self.store.editor_of(note_id, &e.device)?;
            found.push((
                e.clone(),
                NoteChange {
                    at: e.date_at,
                    entity: e.entity.clone(),
                    kind,
                    before,
                    after,
                    format,
                    by,
                },
            ));
            Ok(())
        };
        let texts = |found: &mut Vec<(Entry, NoteChange)>,
                     entity: &str,
                     first: NoteChangeKind,
                     later: NoteChangeKind,
                     skip_first: bool|
         -> Result<()> {
            let mut held: Option<StoredText> = None;
            for e in self.by_field(entity, keys::TEXT)? {
                let stored = StoredText::read(e.value.as_deref());
                let shown = shown(&stored);
                let previous = held.replace(stored.clone());
                let Some(previous) = previous else {
                    if !skip_first {
                        let format = stored.editable.then(|| stored.format.clone());
                        add(found, &e, first, None, Some(shown), format)?;
                    }
                    continue;
                };
                if self::shown(&previous) == shown {
                    if previous.format != stored.format {
                        let (before, after) = (previous.format, stored.format);
                        add(
                            found,
                            &e,
                            NoteChangeKind::Format,
                            Some(before),
                            Some(after),
                            None,
                        )?;
                    }
                    continue;
                }
                let before = Some(self::shown(&previous));
                let format = stored.editable.then(|| stored.format.clone());
                add(found, &e, later, before, Some(shown), format)?;
            }
            Ok(())
        };
        texts(
            &mut found,
            note_id,
            NoteChangeKind::Captured,
            NoteChangeKind::Text,
            false,
        )?;
        for (field, kind) in [
            (keys::IMPORTANCE, NoteChangeKind::Importance),
            (keys::PLACE, NoteChangeKind::Filed),
            (line_keys::KIND, NoteChangeKind::Kind),
        ] {
            let mut held: Option<String> = None;
            for e in self.by_field(note_id, field)? {
                let value = if kind == NoteChangeKind::Kind {
                    list(e.value.as_deref())
                } else {
                    e.value.clone()
                };
                if value != held {
                    add(&mut found, &e, kind, held.clone(), value.clone(), None)?;
                }
                held = value;
            }
        }
        for e in self.by_field(note_id, keys::TRASHED)? {
            let kind = if e.value.is_none() {
                NoteChangeKind::Restored
            } else {
                NoteChangeKind::Trashed
            };
            add(&mut found, &e, kind, None, None, None)?;
        }
        for e in self.store.history(note_id)? {
            let (added, removed) = if e.field.starts_with(keys::PHOTO_PREFIX) {
                (NoteChangeKind::PhotoAdded, NoteChangeKind::PhotoRemoved)
            } else if e.field.starts_with(keys::AUDIO_PREFIX) {
                (
                    NoteChangeKind::RecordingAdded,
                    NoteChangeKind::RecordingRemoved,
                )
            } else {
                continue;
            };
            let kind = if e.value.is_none() { removed } else { added };
            add(&mut found, &e, kind, None, None, None)?;
        }
        for section in notes.sections(note_id)? {
            let id = section.id.as_str();
            if id.starts_with("note:") {
                if let Some(merged) = self.by_field(id, keys::MERGED)?.pop()
                    && merged.value.is_some()
                {
                    let text = Some(self.text_of(id)?);
                    add(
                        &mut found,
                        &merged,
                        NoteChangeKind::MergedIn,
                        None,
                        text,
                        None,
                    )?;
                }
                texts(
                    &mut found,
                    id,
                    NoteChangeKind::Amended,
                    NoteChangeKind::AmendEdited,
                    true,
                )?;
            } else {
                texts(
                    &mut found,
                    id,
                    NoteChangeKind::Amended,
                    NoteChangeKind::AmendEdited,
                    false,
                )?;
            }
        }
        for line in self.lines(note_id)? {
            let name =
                StoredText::read(self.store.current(&line, line_keys::TEXT)?.as_deref()).text;
            let mut held: Option<String> = None;
            for e in self.by_field(&line, line_keys::TEXT)? {
                let text = StoredText::read(e.value.as_deref()).text;
                match &held {
                    None => add(
                        &mut found,
                        &e,
                        NoteChangeKind::LineAdded,
                        None,
                        Some(text.clone()),
                        None,
                    )?,
                    Some(before) if *before != text => add(
                        &mut found,
                        &e,
                        NoteChangeKind::LineRenamed,
                        Some(before.clone()),
                        Some(text.clone()),
                        None,
                    )?,
                    Some(_) => {}
                }
                held = Some(text);
            }
            for (field, on, off) in [
                (
                    line_keys::CHECKED,
                    NoteChangeKind::LineChecked,
                    NoteChangeKind::LineUnchecked,
                ),
                (
                    line_keys::TRASHED,
                    NoteChangeKind::LineRemoved,
                    NoteChangeKind::LineRestored,
                ),
            ] {
                for e in self.by_field(&line, field)? {
                    let kind = if e.value.is_none() { off } else { on };
                    add(&mut found, &e, kind, None, Some(name.clone()), None)?;
                }
            }
        }
        found.sort_by(|a, b| Store::latest_wins(&b.0, &a.0));
        Ok(found.into_iter().map(|(_, change)| change).collect())
    }

    /// Whether this Device may bring back the text of `change` with "Use
    /// this version": it set a text that is not the one showing now, on
    /// the Note or a Section this Device may edit.
    pub fn may_use(&self, note_id: &str, change: &NoteChange) -> Result<bool> {
        let Some(after) = change.after.as_deref() else {
            return Ok(false);
        };
        if !change.kind.sets_text() || change.format.is_none() {
            return Ok(false);
        }
        let id = change.entity.as_str();
        let notes = Notes::new(self.store);
        let may = if id == note_id {
            notes.by_id(note_id)?.is_some_and(|n| n.text_editable)
                && self.store.may_change(note_id)?
        } else {
            notes.may_edit_section(id)?
        };
        Ok(may && self.text_of(id)? != after)
    }

    /// Brings back the text of `change` as a new version of the Note
    /// `note_id` or its Section (#193): an edit like any other, nothing
    /// is deleted. Refused unless [`NoteHistory::may_use`].
    pub fn use_version(&self, note_id: &str, change: &NoteChange) -> Result<()> {
        if !self.may_use(note_id, change)? {
            return Err(Error::Invalid(format!(
                "the text of {} may not be used here",
                change.entity
            )));
        }
        let notes = Notes::new(self.store);
        let text = change.after.as_deref().unwrap_or_default();
        match (change.entity == note_id, change.format.as_deref()) {
            (true, Some(format)) => notes.edit_as(note_id, text, format),
            (true, None) => notes.edit(note_id, text),
            (false, Some(format)) => notes.edit_section_as(&change.entity, text, format),
            (false, None) => notes.edit_section(&change.entity, text),
        }
    }

    /// The entries of `entity` under `field` that count, oldest first by
    /// latest wins: the last is the value that shows.
    fn by_field(&self, entity: &str, field: &str) -> Result<Vec<Entry>> {
        let mut held: Vec<Entry> = self
            .store
            .history(entity)?
            .into_iter()
            .filter(|e| e.field == field)
            .collect();
        held.sort_by(Store::latest_wins);
        Ok(held)
    }

    /// The Lines of the Note `note_id`, of every making, by id.
    fn lines(&self, note_id: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for id in self.store.entities_of_type(line_keys::TYPE_LINE)? {
            if self.store.current(&id, line_keys::NOTE)?.as_deref() == Some(note_id) {
                out.push(id);
            }
        }
        out.sort();
        Ok(out)
    }

    /// The text of the Note or Section `id` as it is now, as [`shown`].
    fn text_of(&self, id: &str) -> Result<String> {
        Ok(shown(&StoredText::read(
            self.store.current(id, keys::TEXT)?.as_deref(),
        )))
    }
}

/// `text` as it shows: each box in its glyph without its id, in a format
/// known here.
fn shown(text: &StoredText) -> String {
    if text.editable {
        text_boxes::strip(&text.text)
    } else {
        text.text.clone()
    }
}

/// The list a value of the Note's `kind` names; none for none.
fn list(value: Option<&str>) -> Option<String> {
    match parse_list_kind(value) {
        Some((list, _)) if list == line_keys::CHECKLIST || list == shopping_keys::SHOPPING => {
            Some(list.to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OwnProfile;
    use crate::checklists::Checklists;
    use crate::notes::Importance as Level;
    use crate::places::Places;
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use crate::text_format::{MARKDOWN, PLAIN, VERSION};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn store() -> std::result::Result<Store, Box<dyn std::error::Error>> {
        let store = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([7; 32])))?;
        OwnProfile::new(&store).apply_defaults("Ann", None)?;
        Ok(store)
    }

    /// A change as its kind and its values before and after.
    type Said = (NoteChangeKind, Option<String>, Option<String>);

    fn said(store: &Store, note: &str) -> Result<Vec<Said>> {
        Ok(NoteHistory::new(store)
            .of(note)?
            .into_iter()
            .map(|c| (c.kind, c.before, c.after))
            .collect())
    }

    fn some(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    #[test]
    fn every_change_of_the_text_newest_first_with_who_and_when() -> TestResult {
        let store = store()?;
        let notes = Notes::new(&store);
        let note = notes.capture("Dentist on Tuesday", None, None)?;
        notes.edit(&note.id, "Dentist on Wednesday")?;
        let dated_back = StoredText::of(PLAIN, "Dated back", VERSION).encode();
        let back = note.captured + chrono::Duration::milliseconds(1);
        store.append(&note.id, keys::TEXT, Some(&dated_back), Some(back))?;
        use NoteChangeKind::*;
        assert_eq!(
            said(&store, &note.id)?,
            vec![
                (Text, some("Dated back"), some("Dentist on Wednesday")),
                (Text, some("Dentist on Tuesday"), some("Dated back")),
                (Captured, None, some("Dentist on Tuesday")),
            ]
        );
        let changes = NoteHistory::new(&store).of(&note.id)?;
        assert!(
            changes
                .iter()
                .all(|c| c.by.as_ref().map(|p| p.name.as_str()) == Some("Ann"))
        );
        assert_eq!(changes[2].at, note.captured);
        assert!(NoteHistory::new(&store).of("note:none")?.is_empty());
        Ok(())
    }

    #[test]
    fn amends_merges_lines_and_settings_are_listed() -> TestResult {
        let store = store()?;
        let notes = Notes::new(&store);
        let home = Places::new(&store).add_area("Home")?;
        let note = notes.capture("Trip", None, None)?;
        let amend = notes.amend(&note.id, "Moved to Wednesday")?;
        notes.edit_section(&amend.id, "Moved to Thursday")?;
        let other = notes.capture("Ask about the crown", None, None)?;
        notes.merge(&other.id, &note.id)?;
        notes.set_importance(&note.id, Level::Important)?;
        notes.file(&note.id, &home.id)?;
        notes.set_format(&note.id, MARKDOWN)?;
        notes.trash(&note.id)?;
        notes.restore(&note.id)?;
        use NoteChangeKind::*;
        assert_eq!(
            said(&store, &note.id)?,
            vec![
                (Restored, None, None),
                (Trashed, None, None),
                (Format, some(PLAIN), some(MARKDOWN)),
                (Filed, None, Some(home.id.clone())),
                (Importance, None, some("important")),
                (MergedIn, None, some("Ask about the crown")),
                (
                    AmendEdited,
                    some("Moved to Wednesday"),
                    some("Moved to Thursday")
                ),
                (Amended, None, some("Moved to Wednesday")),
                (Captured, None, some("Trip")),
            ]
        );
        let list = notes.capture("Packing", None, None)?;
        let lists = Checklists::new(&store);
        lists.make(&list.id)?;
        let tent = lists.add(&list.id, "Tent", None, false)?;
        lists.rename(&tent, "Big tent")?;
        lists.check(&tent)?;
        lists.uncheck(&tent)?;
        lists.delete(&tent)?;
        lists.restore(&tent)?;
        let all = said(&store, &list.id)?;
        assert_eq!(
            all[..7].to_vec(),
            vec![
                (LineRestored, None, some("Big tent")),
                (LineRemoved, None, some("Big tent")),
                (LineUnchecked, None, some("Big tent")),
                (LineChecked, None, some("Big tent")),
                (LineRenamed, some("Tent"), some("Big tent")),
                (LineAdded, None, some("Tent")),
                (Kind, None, some("checklist")),
            ]
        );
        Ok(())
    }

    #[test]
    fn use_version_writes_the_old_text_anew_and_never_the_current_one() -> TestResult {
        let store = store()?;
        let notes = Notes::new(&store);
        let history = NoteHistory::new(&store);
        let note = notes.capture("Dentist on Tuesday", None, None)?;
        notes.edit(&note.id, "Dentist on Wednesday")?;
        let amend = notes.amend(&note.id, "Bring the X-ray")?;
        notes.edit_section(&amend.id, "Bring nothing")?;
        let changes = history.of(&note.id)?;
        let captured = changes.last().ok_or("captured")?.clone();
        let amended = changes
            .iter()
            .find(|c| c.kind == NoteChangeKind::Amended)
            .ok_or("amended")?
            .clone();
        let now = changes
            .iter()
            .find(|c| c.kind == NoteChangeKind::Text)
            .ok_or("text")?;
        assert!(!history.may_use(&note.id, now)?);
        assert!(!history.may_use(&note.id, &changes[0])?);
        assert!(history.may_use(&note.id, &captured)?);
        history.use_version(&note.id, &captured)?;
        history.use_version(&note.id, &amended)?;
        assert_eq!(
            notes.by_id(&note.id)?.ok_or("note")?.text,
            "Dentist on Tuesday"
        );
        assert_eq!(
            notes.section_by_id(&amend.id)?.ok_or("amend")?.text,
            "Bring the X-ray"
        );
        let changes = history.of(&note.id)?;
        assert_eq!(changes.len(), 6);
        assert_eq!(
            (
                changes[1].kind,
                changes[1].before.as_deref(),
                changes[1].after.as_deref()
            ),
            (
                NoteChangeKind::Text,
                Some("Dentist on Wednesday"),
                Some("Dentist on Tuesday")
            )
        );
        let last = changes.last().ok_or("captured")?;
        assert!(!history.may_use(&note.id, last)?);
        assert!(history.use_version(&note.id, last).is_err());
        Ok(())
    }

    #[test]
    fn a_text_in_a_format_not_known_here_is_never_used_again() -> TestResult {
        let store = store()?;
        let notes = Notes::new(&store);
        let history = NoteHistory::new(&store);
        let note = notes.capture("Plain", None, None)?;
        let later = r##"{"format":"x-future","text":"# Later"}"##;
        store.append(&note.id, keys::TEXT, Some(later), None)?;
        let plain = StoredText::of(PLAIN, "Plain again", VERSION).encode();
        store.append(&note.id, keys::TEXT, Some(&plain), None)?;
        let changes = history.of(&note.id)?;
        let future = &changes[1];
        assert_eq!(future.after.as_deref(), Some("# Later"));
        assert_eq!(future.format, None);
        assert!(!history.may_use(&note.id, future)?);
        assert!(history.may_use(&note.id, &changes[2])?);
        Ok(())
    }
}
