//! Later files of the same sender (obf-1147), as the Dart core's
//! `axt_updates_test.dart`.

use super::*;
use crate::store::tests::ticking;
use crate::text_format::{self, StoredText};
use crate::{OwnProfile, SigningKey};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn utc(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// One clock for every Device here, a second further at each reading, so
/// what was written later is later everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// A Device of its own person named `name`, on the shared clock.
fn device(seed: u8, name: &str) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + chrono::Duration::seconds(n)
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn read(store: &Store, bytes: &[u8]) -> Made<AxtFile> {
    Ok(store.axt_read(bytes, &utc)?.map_err(|_| "no AXT file")?)
}

/// A file signed with the key of `sender` holding `records`, written as
/// the library writes any file, at `sender`'s now: what Old Brain Fix
/// does not write itself, such as a deletion.
fn file_of(sender: &Store, records: &[OutgoingRecord]) -> Made<Vec<u8>> {
    let seed = *sender.signing_key()?.seed();
    let mut config = NodeConfig::new(axt_manifest());
    config.readers = type_descriptions()?;
    let at = sender.now();
    config.clock = Box::new(move || system_time(at));
    let mut node = AxtNode::open(
        MemoryStore::default(),
        SigningSeed::new(seed),
        EncryptionSeed::new(box_secret_of(&seed)),
        config,
    );
    Ok(node
        .write_file(records, None, false)
        .map_err(|e| e.to_string())?)
}

fn description(type_name: &str) -> Made<TypeDescription> {
    type_descriptions()?
        .into_iter()
        .find(|d| d.name() == type_name)
        .ok_or_else(|| "a type".into())
}

/// An AXT `Note` of `text`, plain.
fn note_value(text: &str) -> Value {
    record(vec![
        ("title", s(text.split('\n').next().unwrap_or(""))),
        ("text", s(text)),
        (
            "format",
            Value::Enum {
                index: 0,
                symbol: "PLAIN".into(),
            },
        ),
        ("created", Value::Null),
        ("updated", Value::Null),
        ("tags", Value::Array(Vec::new())),
        ("relations", Value::Array(Vec::new())),
        ("extensions", Value::Array(Vec::new())),
    ])
}

/// A Note of Ann's and the one Ben took of it.
struct Pair {
    ann: String,
    ben: String,
}

fn taken(ann: &Store, ben: &Store, id: String) -> Made<Pair> {
    let file = read(ben, &ann.axt_write(std::slice::from_ref(&id))?)?;
    let record = file.taken().first().ok_or("a record")?.id.clone();
    let made = ben.axt_take(&file, &[record], None)?;
    Ok(Pair {
        ann: id,
        ben: made.first().ok_or("a Note")?.clone(),
    })
}

/// Ann's Note Dentist with a Reminder on 3 December at 10:00, and the
/// Note Ben took of it.
fn taken_dentist(ann: &Store, ben: &Store) -> Made<Pair> {
    let id = Notes::new(ann)
        .capture("Dentist\nAt the new place", None, None)?
        .id;
    let ten = ClockTime::new(10, 0).ok_or("10:00")?;
    Routines::new(ann).spawn(
        &id,
        &Schedule::new(Day::new(2026, 12, 3), &[ten], None).ok_or("once")?,
    )?;
    taken(ann, ben, id)
}

fn taken_shopping(ann: &Store, ben: &Store) -> Made<Pair> {
    let id = Notes::new(ann).capture("Shopping\nMilk", None, None)?.id;
    taken(ann, ben, id)
}

fn text_of_note(store: &Store, id: &str) -> Made<String> {
    Ok(Notes::new(store).by_id(id)?.ok_or("a Note")?.text)
}

#[test]
fn a_later_file_of_the_same_sender_updates_the_note_taken_and_keeps_the_version() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let dentist = taken_dentist(&ann, &ben)?;
    let first = ben.axt_from(&dentist.ben)?.ok_or("from")?;
    Notes::new(&ann).edit(&dentist.ann, "Dentist\nAt the old place")?;
    let reminder = Routines::new(&ann)
        .of_note(&dentist.ann)?
        .pop()
        .ok_or("reminder")?;
    let half = ClockTime::new(11, 30).ok_or("11:30")?;
    Routines::new(&ann).reschedule(
        &reminder.id,
        &Schedule::new(Day::new(2026, 12, 4), &[half], None).ok_or("once")?,
    )?;
    let later = ann.axt_write(std::slice::from_ref(&dentist.ann))?;
    let file = read(&ben, &later)?;
    let record = file.records.first().ok_or("a record")?.clone();
    assert_eq!(record.kind, AxtRecordKind::Update);
    assert_eq!(record.kind.code(), "update");
    assert_eq!(record.note_id.as_deref(), Some(dentist.ben.as_str()));
    assert!(!record.kept);
    assert_eq!(file.changes().len(), 1);
    assert!(file.taken().is_empty());
    assert!(record.version > first.version);
    assert!(
        ben.axt_take(&file, std::slice::from_ref(&record.id), None)?
            .is_empty()
    );
    assert_eq!(
        text_of_note(&ben, &dentist.ben)?,
        "Dentist\nAt the old place"
    );
    let routine = Routines::new(&ben)
        .of_note(&dentist.ben)?
        .pop()
        .ok_or("routine")?;
    assert_eq!(routine.schedule.start, Day::new(2026, 12, 4));
    assert_eq!(routine.schedule.times, vec![half]);
    assert_eq!(
        ben.axt_from(&dentist.ben)?.map(|f| f.version),
        Some(record.version)
    );
    // The same file again changes nothing more.
    let before = ben.all_entries()?.len();
    let again = read(&ben, &later)?;
    assert_eq!(again.records[0].kind, AxtRecordKind::Stale);
    ben.axt_take(&again, &[record.id], None)?;
    assert_eq!(ben.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn taking_the_same_file_twice_makes_one_note() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let id = Notes::new(&ann).capture("Milk", None, None)?.id;
    let bytes = ann.axt_write(&[id])?;
    let file = read(&ben, &bytes)?;
    let ids: Vec<String> = file.records.iter().map(|r| r.id.clone()).collect();
    assert_eq!(ben.axt_take(&file, &ids, None)?.len(), 1);
    assert!(ben.axt_take(&file, &ids, None)?.is_empty());
    assert_eq!(read(&ben, &bytes)?.records[0].kind, AxtRecordKind::Stale);
    assert_eq!(Notes::new(&ben).unsorted()?.len(), 1);
    Ok(())
}

#[test]
fn a_note_the_user_changed_since_keeps_the_change_for_every_later_update() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let shopping = taken_shopping(&ann, &ben)?;
    Notes::new(&ben).edit(&shopping.ben, "Shopping\nMilk and bread")?;
    let version = ben.axt_from(&shopping.ben)?.ok_or("from")?.version;
    Notes::new(&ann).edit(&shopping.ann, "Shopping\nMilk and eggs")?;
    let file = read(&ben, &ann.axt_write(std::slice::from_ref(&shopping.ann))?)?;
    let record = file.records[0].clone();
    assert_eq!(record.kind, AxtRecordKind::Update);
    assert!(record.kept);
    let before = ben.all_entries()?.len();
    ben.axt_take(&file, &[record.id], None)?;
    assert_eq!(ben.all_entries()?.len(), before);
    assert_eq!(
        text_of_note(&ben, &shopping.ben)?,
        "Shopping\nMilk and bread"
    );
    assert_eq!(
        ben.axt_from(&shopping.ben)?.map(|f| f.version),
        Some(version)
    );
    Notes::new(&ann).edit(&shopping.ann, "Shopping\nMilk, eggs and tea")?;
    let later = read(&ben, &ann.axt_write(std::slice::from_ref(&shopping.ann))?)?;
    assert!(later.records[0].kept);
    Ok(())
}

#[test]
fn a_change_of_its_reminder_is_a_change_of_the_note() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let dentist = taken_dentist(&ann, &ben)?;
    let routine = Routines::new(&ben)
        .of_note(&dentist.ben)?
        .pop()
        .ok_or("routine")?;
    Routines::new(&ben).set_heads_ups(&routine.id, &[30])?;
    Notes::new(&ann).edit(&dentist.ann, "Dentist\nAt the old place")?;
    let file = read(&ben, &ann.axt_write(std::slice::from_ref(&dentist.ann))?)?;
    assert!(file.records[0].kept);
    ben.axt_take(&file, &[file.records[0].id.clone()], None)?;
    assert_eq!(
        text_of_note(&ben, &dentist.ben)?,
        "Dentist\nAt the new place"
    );
    Ok(())
}

#[test]
fn a_deletion_moves_the_note_to_the_trash_and_a_later_version_brings_it_back() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let dentist = taken_dentist(&ann, &ben)?;
    let record_id = record_id(&dentist.ann, types::TASK);
    let file = read(
        &ben,
        &file_of(&ann, &[OutgoingRecord::deleted(record_id.clone())])?,
    )?;
    let record = file.records[0].clone();
    assert_eq!(record.kind, AxtRecordKind::Deletion);
    assert_eq!(record.note_id.as_deref(), Some(dentist.ben.as_str()));
    assert!(!record.kept);
    assert!(file.not_taken().is_empty());
    ben.axt_take(&file, std::slice::from_ref(&record_id), None)?;
    let note = Notes::new(&ben).by_id(&dentist.ben)?.ok_or("note")?;
    assert!(note.is_trashed());
    assert!(Routines::new(&ben).of_note(&dentist.ben)?.is_empty());
    assert_eq!(
        ben.axt_from(&dentist.ben)?.map(|f| f.version),
        Some(record.version)
    );
    Notes::new(&ann).edit(&dentist.ann, "Dentist\nOn Friday")?;
    let back = read(&ben, &ann.axt_write(std::slice::from_ref(&dentist.ann))?)?;
    assert_eq!(back.records[0].kind, AxtRecordKind::Update);
    assert!(!back.records[0].kept);
    ben.axt_take(&back, &[record_id], None)?;
    let note = Notes::new(&ben).by_id(&dentist.ben)?.ok_or("note")?;
    assert!(!note.is_trashed());
    assert_eq!(note.text, "Dentist\nOn Friday");
    assert_eq!(Routines::new(&ben).of_note(&dentist.ben)?.len(), 1);
    Ok(())
}

#[test]
fn a_deletion_of_a_note_the_user_changed_since_keeps_it() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let shopping = taken_shopping(&ann, &ben)?;
    Notes::new(&ben).edit(&shopping.ben, "Shopping\nMilk and bread")?;
    let record_id = record_id(&shopping.ann, types::NOTE);
    let file = read(
        &ben,
        &file_of(&ann, &[OutgoingRecord::deleted(record_id.clone())])?,
    )?;
    assert_eq!(file.records[0].kind, AxtRecordKind::Deletion);
    assert!(file.records[0].kept);
    ben.axt_take(&file, &[record_id], None)?;
    let note = Notes::new(&ben).by_id(&shopping.ben)?.ok_or("note")?;
    assert!(!note.is_trashed());
    Ok(())
}

#[test]
fn another_sender_can_neither_change_nor_delete_a_note_taken_nor_another_type() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let shopping = taken_shopping(&ann, &ben)?;
    let id = record_id(&shopping.ann, types::NOTE);
    let cy = device(3, "Cy")?;
    let note_type = description(types::NOTE)?;
    let forged = read(
        &ben,
        &file_of(
            &cy,
            &[OutgoingRecord::new(
                id.clone(),
                &note_type,
                note_value("Not yours"),
            )],
        )?,
    )?;
    assert_eq!(forged.records[0].kind, AxtRecordKind::Refused);
    assert_eq!(
        forged.records[0].note_id.as_deref(),
        Some(shopping.ben.as_str())
    );
    assert!(forged.taken().is_empty());
    let deleting = read(&ben, &file_of(&cy, &[OutgoingRecord::deleted(id.clone())])?)?;
    assert_eq!(deleting.records[0].kind, AxtRecordKind::Refused);
    let task_type = description(types::TASK)?;
    let task = record(vec![
        ("title", s("Shopping")),
        ("notes", s("")),
        ("due", Value::Null),
        ("due_all_day", Value::Boolean(false)),
        ("repeat", Value::Null),
        ("done", Value::Boolean(false)),
        ("done_at", Value::Null),
        (
            "priority",
            Value::Enum {
                index: 0,
                symbol: "NONE".into(),
            },
        ),
        ("relations", Value::Array(Vec::new())),
        ("extensions", Value::Array(Vec::new())),
    ]);
    let retyped = read(
        &ben,
        &file_of(&ann, &[OutgoingRecord::new(id.clone(), &task_type, task)])?,
    )?;
    assert_eq!(retyped.records[0].kind, AxtRecordKind::Refused);
    let before = ben.all_entries()?.len();
    for file in [&forged, &deleting, &retyped] {
        assert!(
            ben.axt_take(file, std::slice::from_ref(&id), None)?
                .is_empty()
        );
    }
    assert_eq!(ben.all_entries()?.len(), before);
    assert_eq!(text_of_note(&ben, &shopping.ben)?, "Shopping\nMilk");
    Ok(())
}

#[test]
fn another_persons_device_makes_no_note_an_update_target_and_its_edit_is_no_change() -> TestResult {
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let shopping = taken_shopping(&ann, &ben)?;
    let milk = Notes::new(&ben).capture("Milk", None, None)?.id;
    let cy = device(3, "Cy")?;
    let from = ben.axt_from(&shopping.ben)?.ok_or("from")?;
    let id = record_id(&shopping.ann, types::NOTE);
    let other = Notes::new(&ann).capture("Tea", None, None)?.id;
    let other_id = record_id(&other, types::NOTE);
    // Cy's Device names another record of Ann's on Ben's own Note and on
    // the Note Ben took, later, and edits the Note Ben took.
    let forged = AxtFrom {
        sender: from.sender,
        id: other_id.clone(),
        type_name: types::NOTE.into(),
        version: 0,
    }
    .encode();
    for note in [&milk, &shopping.ben] {
        ben.insert(&cy.append(note, keys::FROM, Some(&forged), None)?.wire())?;
    }
    let text = StoredText::of(
        text_format::PLAIN,
        "Shopping\nCy was here",
        text_format::VERSION,
    )
    .encode();
    ben.insert(
        &cy.append(&shopping.ben, crate::notes::keys::TEXT, Some(&text), None)?
            .wire(),
    )?;
    Notes::new(&ann).edit(&shopping.ann, "Shopping\nMilk and eggs")?;
    let file = read(&ben, &ann.axt_write(&[shopping.ann.clone(), other])?)?;
    let by_id = |id: &str| file.records.iter().find(|r| r.id == id).cloned();
    let fresh = by_id(&other_id).ok_or("other")?;
    assert_eq!(fresh.kind, AxtRecordKind::New);
    assert_eq!(fresh.note_id, None);
    let update = by_id(&id).ok_or("shopping")?;
    assert_eq!(update.kind, AxtRecordKind::Update);
    assert!(!update.kept);
    ben.axt_take(&file, &[id, other_id], None)?;
    assert_eq!(text_of_note(&ben, &milk)?, "Milk");
    assert_eq!(
        text_of_note(&ben, &shopping.ben)?,
        "Shopping\nMilk and eggs"
    );
    Ok(())
}

fn remember(store: &Store, peer: &Store) -> TestResult {
    store.remember_device(&crate::PairedDevice {
        id: peer.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: store.now(),
        shared: true,
    })?;
    Ok(())
}

#[test]
fn a_note_this_device_may_no_longer_change_is_refused_before_anything_is_written() -> TestResult {
    use crate::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
    use crate::shared::{AreaRole, Invite};
    let (ann, ben) = (device(1, "Ann")?, device(2, "Ben")?);
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    remember(&ann, &ben)?;
    remember(&ben, &ann)?;
    let id = Notes::new(&ann).capture("Milk", None, None)?.id;
    let file = read(&ben, &ann.axt_write(std::slice::from_ref(&id))?)?;
    ben.axt_take(&file, &[file.records[0].id.clone()], Some(&area))?;
    ben.leave_area(&area)?;
    Notes::new(&ann).edit(&id, "Milk and eggs")?;
    let later = read(&ben, &ann.axt_write(&[id])?)?;
    assert_eq!(later.records[0].kind, AxtRecordKind::Update);
    let before = ben.all_entries()?.len();
    assert!(
        ben.axt_take(&later, &[later.records[0].id.clone()], None)
            .is_err()
    );
    assert_eq!(ben.all_entries()?.len(), before);
    Ok(())
}
