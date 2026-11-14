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
