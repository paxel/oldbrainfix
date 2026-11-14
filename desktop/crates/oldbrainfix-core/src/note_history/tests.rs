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
