use super::*;
use crate::text_format::{CODE, MARKDOWN};

/// The first lines of the Notes linking to `id`, nothing closed.
fn linking(store: &Store, id: &str) -> Result<Vec<String>> {
    Ok(Backlinks::new(store)
        .to(id, &HashSet::new())?
        .iter()
        .map(|n| n.text.clone())
        .collect())
}

fn link(id: &str) -> String {
    AppLink::Note(id.to_string()).url()
}

#[test]
fn a_note_lists_the_notes_whose_text_or_sections_link_to_it_newest_first() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let ann = notes.capture("Ann", None, None)?;
    let first = format!("Call [Ann]({})", link(&ann.id));
    notes.capture_as(&first, MARKDOWN, None, None)?;
    let other = notes.capture("Groceries", None, None)?;
    notes.amend_as(
        &other.id,
        &format!("**[Ann]({})**", link(&ann.id)),
        MARKDOWN,
    )?;
    let table = format!("| Who |\n| --- |\n| [Ann]({}) |", link(&ann.id));
    notes.capture_as(&table, MARKDOWN, None, None)?;
    assert_eq!(
        linking(&store, &ann.id)?,
        [table.as_str(), "Groceries", first.as_str()]
    );
    assert!(linking(&store, &other.id)?.is_empty());
    Ok(())
}

#[test]
fn only_a_formatted_text_links_and_only_a_note_link_counts() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let ann = notes.capture("Ann", None, None)?;
    let written = format!("[Ann]({})", link(&ann.id));
    notes.capture(&written, None, None)?;
    notes.capture_as(&written, CODE, None, None)?;
    notes.capture_as(&format!("`{written}`"), MARKDOWN, None, None)?;
    let place = AppLink::Place(ann.id.clone()).url();
    notes.capture_as(&format!("[Ann]({place})"), MARKDOWN, None, None)?;
    assert!(linking(&store, &ann.id)?.is_empty());
    Ok(())
}

#[test]
fn the_scheme_in_any_case_and_an_encoded_id_link_too() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let ann = notes.capture("Ann", None, None)?;
    let upper = link(&ann.id).replacen("oldbrainfix://", "OldBrainFix://", 1);
    notes.capture_as(&format!("[Ann]({upper})"), MARKDOWN, None, None)?;
    let encoded = link(&ann.id)
        .replace(':', "%3A")
        .replacen("%3A//", "://", 1);
    notes.capture_as(&format!("[Ann]({encoded})"), MARKDOWN, None, None)?;
    assert_eq!(linking(&store, &ann.id)?.len(), 2);
    Ok(())
}

#[test]
fn a_note_is_listed_once_and_never_under_itself() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let ann = notes.capture("Ann", None, None)?;
    let twice = format!("[Ann]({0}) and [again]({0})", link(&ann.id));
    let note = notes.capture_as(&twice, MARKDOWN, None, None)?;
    notes.amend_as(&note.id, &format!("[Ann]({})", link(&ann.id)), MARKDOWN)?;
    notes.edit_as(&ann.id, &format!("[me]({})", link(&ann.id)), MARKDOWN)?;
    assert_eq!(linking(&store, &ann.id)?, [twice.as_str()]);
    Ok(())
}

#[test]
fn trashed_notes_leave_and_a_merged_one_links_from_where_it_went() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let ann = notes.capture("Ann", None, None)?;
    let gone = notes.capture_as(&format!("[Ann]({})", link(&ann.id)), MARKDOWN, None, None)?;
    notes.trash(&gone.id)?;
    let host = notes.capture("Household", None, None)?;
    let merged = notes.capture_as(&format!("[Ann]({})", link(&ann.id)), MARKDOWN, None, None)?;
    notes.merge(&merged.id, &host.id)?;
    assert_eq!(linking(&store, &ann.id)?, ["Household"]);
    Ok(())
}

#[test]
fn the_notes_of_a_closed_area_are_left_out() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let places = Places::new(&store);
    let locked = places.add_area("Locked")?;
    let topic = places.add_topic(&locked.id, "Inside")?;
    let ann = notes.capture("Ann", None, None)?;
    let text = format!("[Ann]({})", link(&ann.id));
    notes.capture_as(&text, MARKDOWN, Some(&topic.id), None)?;
    notes.capture_as(&text, MARKDOWN, Some(&locked.id), None)?;
    assert_eq!(linking(&store, &ann.id)?.len(), 2);
    let closed = HashSet::from([locked.id.clone()]);
    assert!(Backlinks::new(&store).to(&ann.id, &closed)?.is_empty());
    Ok(())
}

#[test]
fn a_link_written_later_shows_at_once() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let ann = notes.capture("Ann", None, None)?;
    let note = notes.capture("Call Ann", None, None)?;
    assert!(linking(&store, &ann.id)?.is_empty());
    notes.edit_as(
        &note.id,
        &format!("Call [Ann]({})", link(&ann.id)),
        MARKDOWN,
    )?;
    assert_eq!(linking(&store, &ann.id)?.len(), 1);
    notes.edit(&note.id, "Call Ann")?;
    assert!(linking(&store, &ann.id)?.is_empty());
    Ok(())
}

#[test]
fn links_are_read_from_spans_at_any_depth() {
    let text = "- *see [a](oldbrainfix://note/a)*\n> ~~[b](oldbrainfix://note/b)~~ \
                [c](oldbrainfix://area/c) [d](https://x.org)";
    let found = links_in(MARKDOWN, text);
    assert_eq!(found, HashSet::from(["a".to_string(), "b".to_string()]));
    assert!(links_in("plain", text).is_empty());
}
