//! What a Note hands to another app (#129), the same in both cores
//! (`note_share.dart`): its text, then each Section's, every Section
//! under its date, a blank line between them. A Note or Section without
//! words of its own gives its Photo Texts and then its Transcripts
//! instead, in the order its photos and recordings were added; one
//! without either is left out. When any of these texts is formatted,
//! `text` holds them as written, the Markdown source among them with
//! each item that starts with a box written as a task list item
//! ([`markdown::task_lists`], #263), and `html` the whole as HTML
//! ([`markdown::html`]), its boxes as their glyphs, the plain texts and
//! the dates in it as they read; else `html` is `None`. `markdown` holds
//! the same as Markdown, its boxes as task list items, the plain texts
//! and the dates escaped so they read as written (the desktop's "Save as
//! Markdown…"). `photos` and
//! `recordings` are the hashes of the Note's and then its Sections'
//! photos and recordings, each once, for the app to hand along as files.
//!
//! Copying a Checklist or Shopping List ([`list_share`], #323) puts its
//! Lines as a task list of GFM between its text and its Sections.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::Result;
use crate::checklists::{Checklists, keys as checklist_keys};
use crate::markdown;
use crate::notes::{Note, Section};
use crate::schedule::WallClock;
use crate::shopping_lists::ShoppingLists;
use crate::store::Store;
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

/// One Line of a Checklist or Shopping List as it is copied (#323).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLine {
    pub text: String,
    /// Checked, or on a Shopping List struck.
    pub checked: bool,
    /// Under the Line above, in its group.
    pub in_group: bool,
}

/// The Lines of the Note `note_id` as they show now, a Checklist's
/// checked as for the Occurrence it is on ([`Checklists::shown_now`]), a
/// Shopping List's struck ones after its open ones and those checked
/// Once left out; `None` when it is no Checklist or Shopping List.
pub fn task_lines(store: &Store, note_id: &str, wall: &WallClock) -> Result<Option<Vec<TaskLine>>> {
    if let Some(list) = Checklists::new(store).shown_now(note_id, wall)? {
        return Ok(Some(
            list.lines
                .iter()
                .map(|l| TaskLine {
                    text: l.text.clone(),
                    checked: l.checked,
                    in_group: l.in_group(),
                })
                .collect(),
        ));
    }
    Ok(ShoppingLists::new(store)
        .by_note(note_id, wall)?
        .map(|list| {
            list.lines
                .iter()
                .map(|l| TaskLine {
                    text: l.text.clone(),
                    checked: l.struck,
                    in_group: false,
                })
                .collect()
        }))
}

/// `lines` as a task list of GFM (#263, #323), one item a line, `- [ ] `
/// before an open Line, `- [x] ` before a checked one, a Line of a group
/// indented by two spaces; each text as written when `escaped` is false,
/// else escaped so it reads as written ([`markdown::escape`]); `glyphs`
/// writes the boxes as ☐ and ✅ instead, as the app shows them.
fn task_list_with(lines: &[TaskLine], escaped: bool, glyphs: bool) -> String {
    lines
        .iter()
        .map(|l| {
            let indent = if l.in_group {
                checklist_keys::GROUP_INDENT
            } else {
                ""
            };
            let mark = match (glyphs, l.checked) {
                (false, false) => "[ ]",
                (false, true) => "[x]",
                (true, false) => "☐",
                (true, true) => "✅",
            };
            let text = if escaped {
                markdown::escape(&l.text)
            } else {
                l.text.clone()
            };
            format!("{indent}- {mark} {text}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `lines` as a task list of GFM (#323): `- [ ] Milk`, `- [x] Eggs`, a
/// Line of a group indented by two spaces, each text as written.
pub fn task_list(lines: &[TaskLine]) -> String {
    task_list_with(lines, false, false)
}

/// What `note` with its `sections`, oldest first, hands along; `date`
/// writes a Section's capture time as the app shows dates.
pub fn note_share(
    note: &Note,
    sections: &[Section],
    date: impl Fn(DateTime<Utc>) -> String,
) -> NoteShare {
    list_share(note, &[], sections, date)
}

/// What a Checklist or Shopping List `note` copies (#323): as
/// [`note_share`], with its `lines` after its own text as a task list
/// ([`task_list`]), formatted: the HTML beside it holds them as items
/// with their boxes as glyphs. Without `lines` it is [`note_share`].
pub fn list_share(
    note: &Note,
    lines: &[TaskLine],
    sections: &[Section],
    date: impl Fn(DateTime<Utc>) -> String,
) -> NoteShare {
    // Each box in the glyph of its current state, without its id (ADR
    // 0035).
    let note_text = note.shown_text();
    let section_texts: Vec<String> = sections.iter().map(Section::shown_text).collect();
    let mut parts = vec![(
        None,
        note_text.as_str(),
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
    for (s, text) in sections.iter().zip(&section_texts) {
        parts.push((
            Some(date(s.captured)),
            text.as_str(),
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
    let mut saved = Vec::new();
    let mut formatted = false;
    for (date, text, format, generated) in parts {
        let own = trim_end(text);
        let words: Vec<(&str, bool)> = if own.is_empty() {
            generated.iter().map(|t| (*t, false)).collect()
        } else {
            vec![(own, format == text_format::MARKDOWN)]
        };
        if !words.is_empty() {
            formatted = formatted || words.iter().any(|w| w.1);
            let mut shown: Vec<String> = date.iter().cloned().collect();
            let mut read: Vec<String> = date.iter().map(|d| markdown::escape(d)).collect();
            let mut kept = read.clone();
            for (t, md) in words {
                // Boxes go out as task list items (#263), as glyphs in HTML.
                if md {
                    let tasks = markdown::task_lists(t);
                    shown.push(tasks.clone());
                    kept.push(tasks);
                    read.push(t.to_string());
                } else {
                    shown.push(t.to_string());
                    kept.push(markdown::escape(t));
                    read.push(markdown::escape(t));
                }
            }
            written.push(shown.join("\n"));
            source.push(read.join("\n"));
            saved.push(kept.join("\n"));
        }
        // The Lines follow the Note's own text, before its Sections.
        if date.is_none() && !lines.is_empty() {
            formatted = true;
            written.push(task_list(lines));
            source.push(task_list_with(lines, true, true));
            saved.push(task_list_with(lines, true, false));
        }
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
        markdown: saved.join("\n\n"),
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
mod tests;
