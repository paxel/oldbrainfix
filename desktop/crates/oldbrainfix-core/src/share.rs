//! What a Note hands to another app (#129), the same in both cores
//! (`note_share.dart`): its text, then each Section's, every Section
//! under its date, a blank line between them. A Note or Section without
//! words of its own gives its Photo Texts and then its Transcripts
//! instead, in the order its photos and recordings were added; one
//! without either is left out. When any of these texts is formatted,
//! `text` holds them as written, the Markdown source among them, and
//! `html` the whole as HTML ([`markdown::html`]), the plain texts and the
//! dates in it as they read; else `html` is `None`. `markdown` holds
//! the same as Markdown, the plain texts and the dates escaped so they
//! read as written (the desktop's "Save as Markdown…"). `photos` and
//! `recordings` are the hashes of the Note's and then its Sections'
//! photos and recordings, each once, for the app to hand along as files.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::markdown;
use crate::notes::{Note, Section};
use crate::text_format;

/// What a Note hands to another app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteShare {
    pub text: String,
    pub html: Option<String>,
    pub markdown: String,
    pub photos: Vec<String>,
    pub recordings: Vec<String>,
}

/// What `note` with its `sections`, oldest first, hands along; `date`
/// writes a Section's capture time as the app shows dates.
pub fn note_share(
    note: &Note,
    sections: &[Section],
    date: impl Fn(DateTime<Utc>) -> String,
) -> NoteShare {
    let mut parts = vec![(
        None,
        note.text.as_str(),
        note.text_format.as_str(),
        generated(
            &note.photos,
            &note.removed_photos,
            &note.photo_texts,
            &note.recordings,
            &note.removed_recordings,
            &note.transcripts,
        ),
    )];
    for s in sections {
        parts.push((
            Some(date(s.captured)),
            s.text.as_str(),
            s.text_format.as_str(),
            generated(
                &s.photos,
                &s.removed_photos,
                &s.photo_texts,
                &s.recordings,
                &s.removed_recordings,
                &s.transcripts,
            ),
        ));
    }
    let mut written = Vec::new();
    let mut source = Vec::new();
    let mut formatted = false;
    for (date, text, format, generated) in parts {
        let own = trim_end(text);
        let words: Vec<(&str, bool)> = if own.is_empty() {
            generated.iter().map(|t| (*t, false)).collect()
        } else {
            vec![(own, format == text_format::MARKDOWN)]
        };
        if words.is_empty() {
            continue;
        }
        formatted = formatted || words.iter().any(|w| w.1);
        let mut shown: Vec<String> = date.iter().cloned().collect();
        let mut read: Vec<String> = date.iter().map(|d| markdown::escape(d)).collect();
        for (t, md) in words {
            shown.push(t.to_string());
            read.push(if md {
                t.to_string()
            } else {
                markdown::escape(t)
            });
        }
        written.push(shown.join("\n"));
        source.push(read.join("\n"));
    }
    let once = |lists: Vec<&Vec<String>>| {
        let mut out: Vec<String> = Vec::new();
        for hash in lists.into_iter().flatten() {
            if !out.contains(hash) {
                out.push(hash.clone());
            }
        }
        out
    };
    let source = source.join("\n\n");
    NoteShare {
        text: written.join("\n\n"),
        html: formatted.then(|| markdown::html(&source)),
        markdown: source,
        photos: once(
            std::iter::once(&note.photos)
                .chain(sections.iter().map(|s| &s.photos))
                .collect(),
        ),
        recordings: once(
            std::iter::once(&note.recordings)
                .chain(sections.iter().map(|s| &s.recordings))
                .collect(),
        ),
    }
}

/// The Photo Texts and then the Transcripts that say something, in the
/// order their photos and recordings were added, removed ones last.
fn generated<'a>(
    photos: &[String],
    removed_photos: &[String],
    photo_texts: &'a BTreeMap<String, String>,
    recordings: &[String],
    removed_recordings: &[String],
    transcripts: &'a BTreeMap<String, String>,
) -> Vec<&'a str> {
    let said = |texts: &'a BTreeMap<String, String>, hash: &String| {
        texts
            .get(hash)
            .map(|t| trim_end(t))
            .filter(|t| !t.is_empty())
    };
    photos
        .iter()
        .chain(removed_photos)
        .filter_map(|h| said(photo_texts, h))
        .chain(
            recordings
                .iter()
                .chain(removed_recordings)
                .filter_map(|h| said(transcripts, h)),
        )
        .collect()
}

/// `text` without spaces, tabs and line breaks at its end; empty when it
/// holds nothing else.
fn trim_end(text: &str) -> &str {
    let end = text.trim_end_matches([' ', '\t', '\r', '\n']);
    if end.trim_start_matches([' ', '\t', '\r', '\n']).is_empty() {
        ""
    } else {
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;
    use crate::notes::Notes;

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
}
