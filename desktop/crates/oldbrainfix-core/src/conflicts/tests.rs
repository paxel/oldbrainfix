use super::*;
use crate::Notes;
use crate::signing::{ImportReport, SigningKey};
use crate::store::tests::ticking;
use crate::sync::wire;
use std::collections::HashMap;

fn device(seed: u8) -> Result<Store> {
    Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
}

/// Everything `from` holds that `to` lacks, taken in.
fn send(from: &Store, to: &Store) -> Result<Vec<WireEntry>> {
    to.apply_entries(
        &wire(&from.entries_since(&to.version_vector()?)?),
        &from.key_records()?,
        None,
        &mut ImportReport::default(),
    )
}

#[test]
fn contexts_are_written_and_read_by_device() {
    let seen = vec![("b".to_string(), 2), ("a".to_string(), 10)];
    assert_eq!(encode_context(&seen), "a:10 b:2");
    assert_eq!(encode_context(&[]), "");
    let read = decode_context("a:10 b:2");
    assert_eq!(read.get("a"), Some(&10));
    assert_eq!(read.get("b"), Some(&2));
    assert!(decode_context("").is_empty());
    assert!(decode_context("junk :3 c:x").is_empty());
}

#[test]
fn concurrent_edits_conflict_and_a_pick_ends_it_on_both() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("nees mor sauce", None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "Needs more sauce")?;
    Notes::new(&b).edit(&note.id, "Needs more hot sauce")?;
    let came = send(&b, &a)?;
    assert_eq!(a.conflicts_among(&came)?.len(), 1);
    send(&a, &b)?;
    for store in [&a, &b] {
        let conflicts = store.conflicts()?;
        assert_eq!(conflicts.len(), 1);
        let values: Vec<_> = conflicts[0]
            .versions
            .iter()
            .map(|v| v.text().text)
            .collect();
        // Each store has its own clock: the first Device's edit is
        // the later one.
        assert_eq!(values, ["Needs more sauce", "Needs more hot sauce"]);
    }
    let conflict = a.conflicts()?.remove(0);
    a.resolve_conflict(&conflict, &conflict.versions[1])?;
    assert!(a.conflicts()?.is_empty());
    let came = send(&a, &b)?;
    assert!(b.conflicts_among(&came)?.is_empty());
    assert!(b.conflicts()?.is_empty());
    assert_eq!(
        Notes::new(&b).by_id(&note.id)?.map(|n| n.text).as_deref(),
        Some("Needs more hot sauce")
    );
    Ok(())
}

#[test]
fn a_text_and_its_format_conflict_as_one_left_for_a_device_that_knows_it() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("Milk", None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "Oat milk")?;
    b.append(
        &note.id,
        note_keys::TEXT,
        Some(r#"{"format":"x-future","text":"Oat milk"}"#),
        None,
    )?;
    send(&b, &a)?;
    let conflict = a.conflicts()?.remove(0);
    let formats: Vec<String> = conflict.versions.iter().map(|v| v.text().format).collect();
    assert_eq!(formats, ["plain", "x-future"]);
    assert!(!conflict.resolvable());
    for kept in &conflict.versions {
        assert!(a.resolve_conflict(&conflict, kept).is_err());
    }
    assert_eq!(a.conflicts()?.len(), 1);
    Ok(())
}

#[test]
fn a_conflict_over_a_formatted_text_is_resolved_as_over_a_plain_one() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture_as("Milk", crate::text_format::MARKDOWN, None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "**Milk**")?;
    Notes::new(&b).edit(&note.id, "*Milk*")?;
    send(&b, &a)?;
    let conflict = a.conflicts()?.remove(0);
    assert!(conflict.resolvable());
    a.resolve_conflict(&conflict, &conflict.versions[0])?;
    assert!(a.conflicts()?.is_empty());
    let format = Notes::new(&a).by_id(&note.id)?.map(|n| n.text_format);
    assert_eq!(format.as_deref(), Some(crate::text_format::MARKDOWN));
    Ok(())
}

#[test]
fn a_text_written_before_its_format_is_the_same_as_it_marked_plain() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("Milk", None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "Oat milk")?;
    b.append(&note.id, note_keys::TEXT, Some("Oat milk"), None)?;
    send(&b, &a)?;
    assert!(a.conflicts()?.is_empty());
    Ok(())
}

#[test]
fn a_text_with_version_1_written_out_is_the_same_as_without() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("Milk", None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "Oat milk")?;
    b.append(
        &note.id,
        note_keys::TEXT,
        Some(r#"{"format":"plain","text":"Oat milk","v":1}"#),
        None,
    )?;
    send(&b, &a)?;
    assert!(a.conflicts()?.is_empty());
    Ok(())
}

#[test]
fn a_conflict_with_a_later_structure_is_not_resolved_here() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("Milk", None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "Oat milk")?;
    b.append(
        &note.id,
        note_keys::TEXT,
        Some(r#"{"format":"plain","text":"Oat milk","v":2}"#),
        None,
    )?;
    send(&b, &a)?;
    let conflict = a.conflicts()?.remove(0);
    assert!(!conflict.resolvable());
    assert!(
        a.resolve_conflict(&conflict, &conflict.versions[0])
            .is_err()
    );
    Ok(())
}

fn at(hour: u32) -> Result<crate::Schedule> {
    let time = crate::ClockTime::new(hour, 0).ok_or(Error::Invalid("a time".into()))?;
    crate::Schedule::new(crate::Day::new(2026, 3, 1), &[time], None)
        .ok_or(Error::Invalid("a schedule".into()))
}

#[test]
fn a_schedule_kept_writes_no_version() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let routine = Routines::new(&a).create("Pills", &at(8)?)?;
    send(&a, &b)?;
    Routines::new(&a).reschedule(&routine.id, &at(9)?)?;
    Routines::new(&b).reschedule(&routine.id, &at(10)?)?;
    send(&b, &a)?;
    let conflict = a
        .conflict_on(&routine.id, SCHEDULE_UNIT)?
        .ok_or(Error::Invalid("a conflict".into()))?;
    assert!(conflict.resolvable());
    assert!(conflict.versions.iter().all(|v| {
        v.values
            .iter()
            .all(|(k, _)| k != routine_keys::SCHEDULE_VERSION)
    }));
    a.resolve_conflict(&conflict, &conflict.versions[0])?;
    // Version 1 of a schedule is never written (ADR 0034).
    assert!(
        a.history(&routine.id)?
            .iter()
            .all(|e| e.field != routine_keys::SCHEDULE_VERSION)
    );
    Ok(())
}

#[test]
fn a_schedule_of_a_later_version_is_no_pick_here() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let routine = Routines::new(&a).create("Pills", &at(8)?)?;
    send(&a, &b)?;
    Routines::new(&a).reschedule(&routine.id, &at(9)?)?;
    // As the later version writes it.
    b.batch(|| {
        b.append(&routine.id, routine_keys::TIMES, Some("08:00~2"), None)?;
        b.append(&routine.id, routine_keys::SCHEDULE_VERSION, Some("2"), None)?;
        Ok(())
    })?;
    send(&b, &a)?;
    let conflict = a
        .conflict_on(&routine.id, SCHEDULE_UNIT)?
        .ok_or(Error::Invalid("a conflict".into()))?;
    assert!(!conflict.resolvable());
    let before = a.history(&routine.id)?.len();
    for kept in &conflict.versions {
        assert!(a.resolve_conflict(&conflict, kept).is_err());
    }
    assert_eq!(a.history(&routine.id)?.len(), before);
    Ok(())
}

#[test]
fn a_sequential_edit_or_the_same_value_is_no_conflict() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("First", None, None)?;
    send(&a, &b)?;
    Notes::new(&b).edit(&note.id, "Second")?;
    send(&b, &a)?;
    Notes::new(&a).edit(&note.id, "Third")?;
    Notes::new(&b).edit(&note.id, "Third")?;
    send(&a, &b)?;
    send(&b, &a)?;
    assert!(a.conflicts()?.is_empty());
    assert!(b.conflicts()?.is_empty());
    Ok(())
}

#[test]
fn a_log_without_contexts_has_no_conflicts() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("First", None, None)?;
    send(&a, &b)?;
    Notes::new(&a).edit(&note.id, "A")?;
    Notes::new(&b).edit(&note.id, "B")?;
    let old = device(3)?;
    for e in a.all_entries()?.into_iter().chain(b.all_entries()?) {
        old.insert(&WireEntry {
            ctx: None,
            ..e.wire()
        })?;
    }
    assert!(old.conflicts()?.is_empty());
    assert!(old.version_vector()? != HashMap::new());
    Ok(())
}
