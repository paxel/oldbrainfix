//! The kind of a Note (#188), as the Dart core's `note_kinds.dart` reads
//! it: the page it is. A Note has exactly one; a Frog is no kind but a
//! mark any Note can carry. For every kind but a plain Note its text is
//! its name, the first line, and its description, the rest; that split
//! is shown only, nothing is written.

use crate::Result;
use crate::checklists::{keys as checklist_keys, parse_list_kind};
use crate::notes::keys as note_keys;
use crate::sessions::Sessions;
use crate::shopping_lists::keys as shopping_keys;
use crate::store::Store;
use crate::tallies::Tallies;

/// The kind of a Note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// A Note of text alone.
    Plain,
    Checklist,
    ShoppingList,
    Tally,
    /// A Note that picks Tallies for each Occurrence of its Routines
    /// (#191, ADR 0039).
    Session,
}

impl NoteKind {
    /// The kind of the Note `note_id`: a Tally while it has a field, which
    /// wins over the rest (ADR 0030), else a Session while it has a pool,
    /// on the Note or as an older version left it on a Routine (ADR 0039),
    /// else the list its `kind` names, else a plain Note.
    pub fn of(store: &Store, note_id: &str) -> Result<NoteKind> {
        if Tallies::new(store).is_tally(note_id)? {
            return Ok(NoteKind::Tally);
        }
        if Sessions::new(store).is_session_note(note_id)? {
            return Ok(NoteKind::Session);
        }
        // Its fields are read once (#216).
        let fields = store.current_fields(note_id)?;
        if fields.get(note_keys::TYPE) != Some(note_keys::TYPE_NOTE) {
            return Ok(NoteKind::Plain);
        }
        Ok(match parse_list_kind(fields.get(checklist_keys::KIND)) {
            Some((checklist_keys::CHECKLIST, _)) => NoteKind::Checklist,
            Some((shopping_keys::SHOPPING, _)) => NoteKind::ShoppingList,
            _ => NoteKind::Plain,
        })
    }
}

/// The name of a Note of a kind with the text `text`: its first line, as
/// stored, without a line break.
pub fn name(text: &str) -> &str {
    match text.find(['\n', '\r']) {
        Some(at) => &text[..at],
        None => text,
    }
}

/// The description of a Note of a kind with the text `text`: all after
/// its first line, as stored; empty when it has one line.
pub fn description(text: &str) -> &str {
    let Some(at) = text.find(['\n', '\r']) else {
        return "";
    };
    let skip = if text[at..].starts_with("\r\n") { 2 } else { 1 };
    &text[at + skip..]
}

/// The text of a Note of the name `name` and the description
/// `description`: the name alone while the description is empty, else
/// both, a line break between them. The text of a name and a description
/// read from a text with "\n" between them is that text.
pub fn join(name: &str, description: &str) -> String {
    if description.is_empty() {
        name.to_string()
    } else {
        format!("{name}\n{description}")
    }
}

#[cfg(test)]
mod tests;
