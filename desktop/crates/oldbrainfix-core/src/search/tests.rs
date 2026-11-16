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

#[test]
fn attachment_texts_and_file_names_are_searched_and_a_removed_text_is_not() -> Result<()> {
    use crate::notes::NewFile;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let file = |name: &'static str, bytes: &'static [u8]| NewFile {
        name,
        media_type: "",
        bytes,
        pages: None,
    };
    let letter =
        notes.capture_with_files("", None, None, &[], &[], &[file("Lease.txt", b"lease")])?;
    let hash = &letter.attachments[0].hash;
    notes.set_attachment_text(&letter.id, hash, "Rent due on the first")?;
    let target = notes.capture("Papers", None, None)?;
    let merged = notes.capture_with_files("", None, None, &[], &[], &[file("plan.dwg", b"dwg")])?;
    notes.merge(&merged.id, &target.id)?;
    let search = Search::new(&store);
    let ids = |q| -> Result<Vec<String>> {
        Ok(search.notes(q, None)?.into_iter().map(|n| n.id).collect())
    };
    assert_eq!(ids("RENT DUE")?, vec![letter.id.clone()]);
    assert_eq!(ids("lease.t")?, vec![letter.id.clone()]);
    assert_eq!(ids("plan.dwg")?, vec![target.id.clone()]);
    notes.remove_attachment_text(&letter.id, hash)?;
    assert!(ids("rent")?.is_empty());
    assert_eq!(ids("lease")?, vec![letter.id.clone()]);
    Ok(())
}

#[test]
fn among_finds_trashed_notes_as_the_search_finds_notes_in_the_order_given() -> Result<()> {
    use crate::checklists::Checklists;
    use crate::tallies::Tallies;
    use std::collections::BTreeMap;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let text = notes.capture("Old sauce recipe", None, None)?;
    let voice = notes.capture_with("", None, None, &[], &[b"voice".to_vec()])?;
    notes.set_transcript(&voice.id, &voice.recordings[0], "Pick up the parcel")?;
    let photo = notes.capture_with("", None, None, &[b"photo".to_vec()], &[])?;
    notes.set_photo_text(&photo.id, &photo.photos[0], "RECEIPT 12,50")?;
    let amended = notes.capture("Saturday", None, None)?;
    notes.amend(&amended.id, "Bring the bags")?;
    let host = notes.capture("Errands", None, None)?;
    let merged = notes.capture("Opening hours", None, None)?;
    notes.merge(&merged.id, &host.id)?;
    let tally = notes.capture("Push-ups", None, None)?;
    let tallies = Tallies::new(&store);
    let reps = tallies.add_field(&tally.id, "times", "times", None, None)?;
    tallies.add(
        &tally.id,
        &BTreeMap::from([(reps, 20.0)]),
        "after work",
        None,
    )?;
    let checklist = notes.capture("Trip\nTent\nSleeping bag", None, None)?;
    Checklists::new(&store).make(&checklist.id)?;
    let shopping = notes.capture("Shop\nOat milk", None, None)?;
    ShoppingLists::new(&store).make(&shopping.id)?;
    // Live, so never among the trash's.
    let live = notes.capture("Fresh sauce", None, None)?;
    for note in [
        &text, &voice, &photo, &amended, &host, &tally, &checklist, &shopping,
    ] {
        notes.trash(&note.id)?;
    }
    let trashed = notes.trashed()?;
    let search = Search::new(&store);
    let ids = |q: &str, scope: Vec<Note>| -> Result<Vec<String>> {
        Ok(search.among(q, scope)?.into_iter().map(|n| n.id).collect())
    };
    assert_eq!(ids("SAUCE", trashed.clone())?, vec![text.id.clone()]);
    assert_eq!(ids("parcel", trashed.clone())?, vec![voice.id]);
    assert_eq!(ids("receipt", trashed.clone())?, vec![photo.id]);
    assert_eq!(ids("bags", trashed.clone())?, vec![amended.id]);
    assert_eq!(ids("opening", trashed.clone())?, vec![host.id]);
    assert_eq!(ids("after work", trashed.clone())?, vec![tally.id]);
    assert_eq!(ids("sleeping", trashed.clone())?, vec![checklist.id]);
    assert_eq!(ids("oat", trashed.clone())?, vec![shopping.id]);
    assert_eq!(search.notes("sauce", None)?.len(), 1);
    // In the order given, whatever it is; a blank query keeps all.
    let mut given = trashed.clone();
    given.reverse();
    let order: Vec<String> = given.iter().map(|n| n.id.clone()).collect();
    assert_eq!(ids("  ", given.clone())?, order);
    let both = vec![text.clone(), live.clone()];
    assert_eq!(ids("sauce", both)?, vec![text.id, live.id]);
    assert!(ids("sauce", Vec::new())?.is_empty());
    Ok(())
}

#[test]
fn a_formatted_text_is_found_by_its_words_never_its_marks() -> Result<()> {
    use crate::text_format::MARKDOWN;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let bold = notes.capture_as("**bold** `code`", MARKDOWN, None, None)?;
    let plain = notes.capture("stars ** here", None, None)?;
    let host = notes.capture("Host", None, None)?;
    notes.amend_as(&host.id, "~~struck~~", MARKDOWN)?;
    let merged = notes.capture_as("[label](https://x.org)", MARKDOWN, None, None)?;
    notes.merge(&merged.id, &host.id)?;
    let search = Search::new(&store);
    let ids = |q: &str| -> Result<Vec<String>> {
        Ok(search.notes(q, None)?.into_iter().map(|n| n.id).collect())
    };
    assert_eq!(ids("bold")?, vec![bold.id.clone()]);
    assert_eq!(ids("bold code")?, vec![bold.id]);
    assert_eq!(ids("**")?, vec![plain.id]);
    assert!(ids("`")?.is_empty());
    assert_eq!(ids("struck")?, vec![host.id.clone()]);
    assert!(ids("~~")?.is_empty());
    assert_eq!(ids("label")?, vec![host.id]);
    assert!(ids("x.org")?.is_empty());
    Ok(())
}

#[test]
fn a_tag_finds_every_note_holding_it_in_any_case_not_the_word_alone() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let in_text = notes.capture("#garten water the beans", None, None)?;
    let in_section = notes.capture("Saturday", None, None)?;
    let longer = notes.capture("#Gartenarbeit: dig", None, None)?;
    notes.capture("Garten without a tag", None, None)?;
    notes.amend(&in_section.id, "then the #GARTEN")?;
    let search = Search::new(&store);
    let ids = |q: &str| -> Result<Vec<String>> {
        Ok(search.notes(q, None)?.into_iter().map(|n| n.id).collect())
    };
    let expected = vec![longer.id, in_section.id, in_text.id];
    assert_eq!(ids("#Garten")?, expected);
    let tapped = crate::text_tags::find("see #Garten");
    assert_eq!(tapped.len(), 1);
    for tag in tapped {
        assert_eq!(ids(&tag.tag)?, expected);
    }
    // Lowered letter by letter, as the tag is and the Dart core does.
    let turkish = notes.capture("#İzmir trip", None, None)?;
    let greek = notes.capture("#ΟΔΟΣ", None, None)?;
    assert_eq!(ids("#izmir")?, vec![turkish.id.clone()]);
    assert_eq!(ids("#İZMİR")?, vec![turkish.id]);
    assert_eq!(ids("#οδοσ")?, vec![greek.id]);
    Ok(())
}
