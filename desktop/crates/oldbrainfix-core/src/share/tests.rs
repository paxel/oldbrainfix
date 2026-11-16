use super::*;
use crate::notes::Notes;
use crate::{Error, Result};

fn day(d: DateTime<Utc>) -> String {
    d.format("%H:%M:%S").to_string()
}

#[test]
fn the_text_and_each_section_under_its_date_plain_without_html() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Milk\n", None, None)?;
    let eggs = notes.amend_as(&note.id, "Eggs", text_format::PLAIN)?;
    let note = notes
        .by_id(&note.id)?
        .ok_or_else(|| crate::Error::Invalid("gone".into()))?;
    let share = note_share(&note, &notes.sections(&note.id)?, day);
    assert_eq!(share.text, format!("Milk\n\n{}\nEggs", day(eggs.captured)));
    assert_eq!(share.html, None);
    Ok(())
}

#[test]
fn a_formatted_text_goes_as_written_with_html_beside_it() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture_as("**Milk** *now*", text_format::MARKDOWN, None, None)?;
    let more = notes.amend_as(&note.id, "2*3*4 # x", text_format::PLAIN)?;
    let share = note_share(&note, &notes.sections(&note.id)?, day);
    let at = day(more.captured);
    assert_eq!(share.text, format!("**Milk** *now*\n\n{at}\n2*3*4 # x"));
    assert_eq!(
        share.html.as_deref(),
        Some(
            format!("<p><strong>Milk</strong> <em>now</em></p>\n<p>{at}<br>2*3*4 # x</p>\n")
                .as_str()
        )
    );
    // Saved as Markdown, the plain Section reads as written.
    let plain = markdown::escape("2*3*4 # x");
    assert!(plain.contains("2\\*3\\*4"), "{plain}");
    assert_eq!(
        share.markdown,
        format!("**Milk** *now*\n\n{}\n{plain}", markdown::escape(&at))
    );
    Ok(())
}

#[test]
fn boxes_go_as_task_list_items_and_as_glyphs_in_html() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture_as("- [ ] milk\n- [x] bread", text_format::MARKDOWN, None, None)?;
    let id = notes
        .by_id(&note.id)?
        .ok_or_else(|| Error::Invalid("a Note".into()))?
        .boxes[0]
        .id
        .clone();
    crate::boxes::Boxes::new(&store).tick(&note.id, &id, crate::text_boxes::BoxState::Failed)?;
    let more = notes.amend_as(&note.id, "- ☐ plain", text_format::PLAIN)?;
    let note = notes
        .by_id(&note.id)?
        .ok_or_else(|| Error::Invalid("a Note".into()))?;
    let share = note_share(&note, &notes.sections(&note.id)?, day);
    let at = day(more.captured);
    assert_eq!(
        share.text,
        format!("- [x] ~~milk~~\n- [x] bread\n\n{at}\n- ☐ plain")
    );
    assert_eq!(
        share.html.as_deref(),
        Some(
            format!("<ul><li>❌ milk</li><li>✅ bread</li></ul>\n<p>{at}<br>- ☐ plain</p>\n")
                .as_str()
        )
    );
    assert!(
        share
            .markdown
            .starts_with("- [x] ~~milk~~\n- [x] bread\n\n"),
        "{}",
        share.markdown
    );
    Ok(())
}

#[test]
fn transcripts_go_along_only_without_words() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let voice = vec![0x52, 0x49, 0x46, 0x46, 1];
    let silent = notes.capture_with("", None, None, &[], std::slice::from_ref(&voice))?;
    let hash = silent.recordings[0].clone();
    notes.set_transcript(&silent.id, &hash, "*buy* milk ")?;
    let silent = notes
        .by_id(&silent.id)?
        .ok_or_else(|| crate::Error::Invalid("gone".into()))?;
    let share = note_share(&silent, &[], day);
    assert_eq!(share.text, "*buy* milk");
    assert_eq!(share.html, None);
    assert_eq!(share.recordings, vec![hash.clone()]);
    let worded = notes.capture_with("Shop", None, None, &[], &[voice])?;
    notes.set_transcript(&worded.id, &hash, "buy milk")?;
    let worded = notes
        .by_id(&worded.id)?
        .ok_or_else(|| crate::Error::Invalid("gone".into()))?;
    assert_eq!(note_share(&worded, &[], day).text, "Shop");
    Ok(())
}

fn wall(utc: DateTime<Utc>) -> chrono::NaiveDateTime {
    utc.naive_utc()
}

/// The Note `id` as the store holds it now.
fn note_of(notes: &Notes, id: &str) -> Result<Note> {
    notes
        .by_id(id)?
        .ok_or_else(|| Error::Invalid("a Note".into()))
}

#[test]
fn a_checklist_copies_its_lines_as_a_task_list_between_its_text_and_its_sections() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture(
        "Groceries\nMilk\n ✓ Eggs\n Butter\nBread *fresh*",
        None,
        None,
    )?;
    Checklists::new(&store).make(&note.id)?;
    let later = notes.amend_as(&note.id, "From the shop", text_format::PLAIN)?;
    let lines =
        task_lines(&store, &note.id, &wall)?.ok_or_else(|| Error::Invalid("Lines".into()))?;
    assert_eq!(
        task_list(&lines),
        "- [ ] Milk\n  - [x] Eggs\n  - [ ] Butter\n- [ ] Bread *fresh*"
    );
    let share = list_share(
        &note_of(&notes, &note.id)?,
        &lines,
        &notes.sections(&note.id)?,
        day,
    );
    let at = day(later.captured);
    assert_eq!(
        share.text,
        format!(
            "Groceries\n\n- [ ] Milk\n  - [x] Eggs\n  - [ ] Butter\n- [ ] Bread *fresh*\n\n{at}\nFrom the shop"
        )
    );
    // The HTML reads the Line's text as written, its boxes as glyphs.
    assert_eq!(
        share.html.as_deref(),
        Some(
            format!(
                "<p>Groceries</p>\n<ul><li>☐ Milk<ul><li>✅ Eggs</li><li>☐ Butter</li></ul></li><li>☐ Bread *fresh*</li></ul>\n<p>{at}<br>From the shop</p>\n"
            )
            .as_str()
        )
    );
    Ok(())
}

#[test]
fn a_shopping_list_copies_its_struck_lines_checked_after_the_open_ones_and_leaves_out_gone_ones()
-> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Shop\nMilk\n✓ Eggs\nTea\nCake", None, None)?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note.id)?;
    let cake = lists
        .by_note(&note.id, &wall)?
        .ok_or_else(|| Error::Invalid("a list".into()))?
        .lines
        .iter()
        .find(|l| l.text == "Cake")
        .ok_or_else(|| Error::Invalid("Cake".into()))?
        .id
        .clone();
    lists.set_reappear(&cake, crate::shopping_lists::Reappear::Once)?;
    lists.check(&cake, &wall)?;
    let lines =
        task_lines(&store, &note.id, &wall)?.ok_or_else(|| Error::Invalid("Lines".into()))?;
    let share = list_share(&note_of(&notes, &note.id)?, &lines, &[], day);
    assert_eq!(share.text, "Shop\n\n- [ ] Milk\n- [ ] Tea\n- [x] Eggs");
    Ok(())
}

#[test]
fn a_checklist_with_a_routine_copies_its_lines_as_checked_for_the_occurrence_now() -> Result<()> {
    use crate::schedule::{Day, Frequency, Recurrence, Schedule};
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Pills\nBlue\n✓ Red", None, None)?;
    Checklists::new(&store).make(&note.id)?;
    let lines =
        task_lines(&store, &note.id, &wall)?.ok_or_else(|| Error::Invalid("Lines".into()))?;
    assert_eq!(task_list(&lines), "- [ ] Blue\n- [x] Red");
    let daily = Schedule::new(
        Day::new(2026, 1, 1),
        &[],
        Some(Recurrence::every(Frequency::Daily, 1)),
    )
    .ok_or_else(|| Error::Invalid("a schedule".into()))?;
    crate::Routines::new(&store).spawn(&note.id, &daily)?;
    // Every Occurrence starts with no Line checked, as its card shows.
    let lines =
        task_lines(&store, &note.id, &wall)?.ok_or_else(|| Error::Invalid("Lines".into()))?;
    assert_eq!(task_list(&lines), "- [ ] Blue\n- [ ] Red");
    Ok(())
}

#[test]
fn a_list_without_a_name_copies_only_its_lines_and_a_plain_note_has_none() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Milk\nEggs", None, None)?;
    assert_eq!(task_lines(&store, &note.id, &wall)?, None);
    Checklists::new(&store).make_from_text(&note.id)?;
    let lines =
        task_lines(&store, &note.id, &wall)?.ok_or_else(|| Error::Invalid("Lines".into()))?;
    let share = list_share(&note_of(&notes, &note.id)?, &lines, &[], day);
    assert_eq!(share.text, "- [ ] Milk\n- [ ] Eggs");
    Ok(())
}

#[test]
fn a_copied_list_pasted_into_a_formatted_text_gives_the_same_boxes() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Trip\nTent\n ✓ Pegs\n Poles\n✓ Map", None, None)?;
    Checklists::new(&store).make(&note.id)?;
    let lines =
        task_lines(&store, &note.id, &wall)?.ok_or_else(|| Error::Invalid("Lines".into()))?;
    let copied = list_share(&note_of(&notes, &note.id)?, &lines, &[], day).text;
    let pasted = notes.capture_as(&copied, text_format::MARKDOWN, None, None)?;
    assert_eq!(
        note_of(&notes, &pasted.id)?.shown_text(),
        "Trip\n\n- ☐ Tent\n  - ✅ Pegs\n  - ☐ Poles\n- ✅ Map"
    );
    Ok(())
}
