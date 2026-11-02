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
mod tests {
    use super::*;
    use crate::checklists::Checklists;
    use crate::notes::Notes;
    use crate::shopping_lists::ShoppingLists;
    use crate::signing::SigningKey;
    use crate::{OwnProfile, Store};
    use chrono::{DateTime, TimeZone, Utc};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn ann() -> std::result::Result<Store, Box<dyn std::error::Error>> {
        let start = Utc
            .with_ymd_and_hms(2026, 3, 5, 10, 0, 0)
            .single()
            .ok_or("a start")?;
        let ticks = Arc::new(AtomicI64::new(start.timestamp_micros()));
        let clock: crate::Clock = Box::new(move || {
            let n = ticks.fetch_add(1_000_000, Ordering::SeqCst) + 1_000_000;
            DateTime::from_timestamp_micros(n).unwrap_or_default()
        });
        let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([1; 32])))?;
        OwnProfile::new(&store).apply_defaults("Ann", None)?;
        Ok(store)
    }

    #[test]
    fn a_note_is_of_one_kind_and_a_tally_wins() -> TestResult {
        let ann = ann()?;
        let notes = Notes::new(&ann);
        let plain = notes.capture("Call Ben", None, None)?.id;
        let check = notes.capture("Trip\nSocks", None, None)?.id;
        Checklists::new(&ann).make(&check)?;
        let shop = notes.capture("Shop\nMilk", None, None)?.id;
        ShoppingLists::new(&ann).make(&shop)?;
        let gym = notes.capture("Gym", None, None)?.id;
        Tallies::new(&ann).add_field(&gym, "Reps", "reps", None, None)?;
        assert_eq!(NoteKind::of(&ann, &plain)?, NoteKind::Plain);
        assert_eq!(NoteKind::of(&ann, &check)?, NoteKind::Checklist);
        assert_eq!(NoteKind::of(&ann, &shop)?, NoteKind::ShoppingList);
        assert_eq!(NoteKind::of(&ann, &gym)?, NoteKind::Tally);
        let kind = checklist_keys::kind_of("x");
        ann.append(&gym, checklist_keys::KIND, Some(&kind), None)?;
        assert_eq!(NoteKind::of(&ann, &gym)?, NoteKind::Tally);
        Checklists::new(&ann).to_plain(&check)?;
        assert_eq!(NoteKind::of(&ann, &check)?, NoteKind::Plain);
        Ok(())
    }

    #[test]
    fn it_reads_the_fields_of_a_note_once_to_tell_its_kind() -> TestResult {
        let ann = ann()?;
        let notes = Notes::new(&ann);
        let plain = notes.capture("Call Ben", None, None)?.id;
        let check = notes.capture("Trip\nSocks\nShirts", None, None)?.id;
        Checklists::new(&ann).make(&check)?;
        let shop = notes.capture("Shop\nMilk\nBread", None, None)?.id;
        ShoppingLists::new(&ann).make(&shop)?;
        let gym = notes.capture("Gym", None, None)?.id;
        Tallies::new(&ann).add_field(&gym, "Reps", "reps", None, None)?;
        // What every Note shares is worked out once.
        NoteKind::of(&ann, &plain)?;
        for (id, kind) in [
            (&plain, NoteKind::Plain),
            (&check, NoteKind::Checklist),
            (&shop, NoteKind::ShoppingList),
        ] {
            let before = ann.history_reads.load(Ordering::SeqCst);
            assert_eq!(NoteKind::of(&ann, id)?, kind);
            assert_eq!(
                ann.history_reads.load(Ordering::SeqCst) - before,
                1,
                "{kind:?}"
            );
        }
        let before = ann.history_reads.load(Ordering::SeqCst);
        assert_eq!(NoteKind::of(&ann, &gym)?, NoteKind::Tally);
        assert_eq!(ann.history_reads.load(Ordering::SeqCst), before);
        Ok(())
    }

    #[test]
    fn the_first_line_is_the_name_and_the_rest_the_description() {
        assert_eq!(name("Groceries"), "Groceries");
        assert_eq!(description("Groceries"), "");
        let text = "Groceries\nfor Sunday\n\nand Monday";
        assert_eq!(name(text), "Groceries");
        assert_eq!(description(text), "for Sunday\n\nand Monday");
        assert_eq!((name("A\r\nB"), description("A\r\nB")), ("A", "B"));
        assert_eq!((name(""), description("A\n")), ("", ""));
        assert_eq!(name("# Shop **now**\n- milk"), "# Shop **now**");
    }

    #[test]
    fn joined_back_they_are_the_same_text() {
        for text in [
            "Groceries",
            "Groceries\nfor Sunday",
            "Groceries\n\nfor Sunday\n",
            " Trip \n  socks",
        ] {
            assert_eq!(join(name(text), description(text)), text);
        }
        assert_eq!(join("Groceries", ""), "Groceries");
    }
}
