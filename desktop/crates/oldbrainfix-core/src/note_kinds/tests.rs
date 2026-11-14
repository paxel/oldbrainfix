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
