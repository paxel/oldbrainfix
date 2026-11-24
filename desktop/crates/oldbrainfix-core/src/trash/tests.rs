use super::*;
use crate::PairedDevice;
use crate::entry::dart_iso;
use crate::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use crate::notes::NewFile;
use crate::notes::keys::TRASHED;
use crate::places::Places;
use crate::schedule::{ClockTime, Day, Frequency, Recurrence, Schedule};
use crate::shared::{AreaRole, Invite};
use crate::signing::SigningKey;
use crate::store::Clock;
use chrono::{Duration, TimeZone, Utc};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// Devices sharing one clock, a second per read, so they write in a
/// known order; each with a Profile of its own.
struct Devices(Arc<AtomicI64>);

impl Devices {
    fn new() -> Devices {
        Devices(Arc::new(AtomicI64::new(0)))
    }

    fn device(&self, seed: u8, name: &str) -> Result<Store> {
        let ticks = Arc::clone(&self.0);
        let clock: Clock = Box::new(move || {
            let n = ticks.fetch_add(1, Ordering::SeqCst) + 1;
            let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).single();
            start.unwrap_or_default() + Duration::seconds(n)
        });
        let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
        crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }
}

/// Stand-ins: a JPEG start, so the store keeps the photo as it is.
fn photo(n: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xD9, n]
}

fn voice(n: u8) -> Vec<u8> {
    vec![0x52, 0x49, 0x46, 0x46, n]
}

const LEASE: NewFile<'static> = NewFile {
    name: "lease.txt",
    media_type: "",
    bytes: b"The lease",
    pages: None,
};

fn daily() -> Result<Schedule> {
    let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::every(Frequency::Daily, 1);
    Schedule::new(Day::new(2026, 3, 1), &eight, Some(rule))
        .ok_or_else(|| crate::Error::Invalid("a schedule".into()))
}

/// The blobs `to` lacks, carried over from `from`.
fn carry(from: &Store, to: &Store) -> Result<()> {
    for reference in to.missing_blobs()? {
        let (kind, hash) = split_blob_ref(&reference);
        if let Some(bytes) = from.blob(kind, hash) {
            to.put_blob(kind, hash, &bytes)?;
        }
    }
    Ok(())
}

/// One LAN session without the network, the blobs after it both
/// ways: `joiner` joins `host`, which invites it when `inviting`.
/// Both remember each other, as the apps do.
fn exchange(joiner: &Store, host: &Store, inviting: Option<&Invite>) -> TestResult {
    let host_id = host.device_id()?;
    let joiner_id = joiner.device_id()?;
    let shared = inviting.is_some() || joiner.is_shared_peer(&host_id)?;
    let joining = match inviting {
        Some(_) => Joining::Invited,
        None => Joining::Plain,
    };
    let vector = host.version_vector()?;
    let request = sync_request_as(joiner, &vector, "Phone", &host_id, "", joining)?;
    let (answer, _) = answer_sync_inviting(host, &request, "Desk", inviting)?.ok_or("an answer")?;
    apply_answer_as(joiner, &answer, shared, None)?;
    for (store, id) in [(host, &joiner_id), (joiner, &host_id)] {
        store.remember_device(&PairedDevice {
            id: id.clone(),
            name: "Other".into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared,
        })?;
    }
    carry(host, joiner)?;
    carry(joiner, host)?;
    Ok(())
}

fn set(ids: &[String]) -> HashSet<&str> {
    ids.iter().map(String::as_str).collect()
}

#[test]
fn what_is_in_the_trash_goes_at_once_and_the_count_names_it() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let routines = Routines::new(&ann);
    let kept = notes.capture("Kept", None, None)?;
    let recent = notes.capture("Recent", None, None)?;
    let amended = notes.capture("Amended", None, None)?;
    notes.amend(&amended.id, "later")?;
    let merged = notes.capture("Merged", None, None)?;
    notes.merge(&merged.id, &amended.id)?;
    let dentist = notes.capture("Dentist", None, None)?;
    let reminder = routines.spawn(&dentist.id, &daily()?)?;
    let pills = notes.capture("Pills", None, None)?;
    let alone = routines.spawn(&pills.id, &daily()?)?;
    let old = notes.capture("Old", None, None)?;
    notes.trash(&recent.id)?;
    notes.trash(&amended.id)?;
    routines.trash_note(&dentist.id)?;
    routines.trash(&alone.id)?;
    // Past the 30 days, as a Device that has not purged yet holds it.
    let long_ago = Utc.with_ymd_and_hms(2025, 11, 1, 0, 0, 0).single();
    let long_ago = dart_iso(long_ago.ok_or("a date")?);
    ann.append(&old.id, TRASHED, Some(&long_ago), None)?;

    let trash = Trash::new(&ann);
    let asked = trash.to_empty()?;
    assert_eq!(
        set(&asked.notes),
        set(&[
            recent.id.clone(),
            amended.id.clone(),
            dentist.id.clone(),
            old.id.clone()
        ])
    );
    assert_eq!(
        set(&asked.routines),
        set(&[reminder.id.clone(), alone.id.clone()])
    );
    assert!(asked.blobs.is_empty());

    let removed = trash.empty()?;
    assert_eq!(removed.notes, asked.notes);
    assert_eq!(removed.routines, asked.routines);
    for id in [&recent.id, &amended.id, &merged.id, &dentist.id, &old.id] {
        assert!(notes.by_id(id)?.is_none(), "{id} is gone");
        assert!(!ann.history(id)?.is_empty(), "the log keeps {id}");
    }
    assert!(notes.sections(&amended.id)?.is_empty());
    assert!(ann.entities_of_type(note_keys::TYPE_SECTION)?.is_empty());
    assert!(routines.by_id(&reminder.id)?.is_none());
    assert!(routines.by_id(&alone.id)?.is_none());
    assert!(notes.trashed()?.is_empty());
    assert!(routines.trashed()?.is_empty());
    let left: Vec<String> = notes.all()?.into_iter().map(|n| n.id).collect();
    assert_eq!(left, [pills.id, kept.id]);

    assert!(trash.to_empty()?.is_empty());
    let mark = ann.current_seq()?;
    assert!(trash.empty()?.is_empty());
    assert!(ann.entries_after(mark)?.is_empty(), "nothing written");
    Ok(())
}

#[test]
fn what_an_ai_task_only_drafted_into_the_trash_stays_and_rejected_is_back() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let routines = Routines::new(&ann);
    let drafted = notes.capture("Drafted", None, None)?;
    let routine = routines.spawn(&drafted.id, &daily()?)?;
    let alone = routines.spawn(&notes.capture("Alone", None, None)?.id, &daily()?)?;
    let set = ann.open_pending("ai-task", true, None)?;
    let at = dart_iso(ann.now());
    ann.draft_pending(&set, &drafted.id, TRASHED, Some(&at), None)?;
    ann.draft_pending(&set, &routine.id, routine_keys::TRASHED, Some(&at), None)?;
    ann.draft_pending(&set, &alone.id, routine_keys::TRASHED, Some(&at), None)?;
    // Shown as the screens show it, waiting to be accepted.
    let shown: Vec<String> = notes.trashed()?.into_iter().map(|n| n.id).collect();
    assert_eq!(shown, std::slice::from_ref(&drafted.id));
    let shown: Vec<String> = routines.trashed()?.into_iter().map(|r| r.id).collect();
    assert!(shown.contains(&alone.id));

    let trash = Trash::new(&ann);
    assert!(trash.to_empty()?.is_empty());
    let mark = ann.current_seq()?;
    assert!(trash.empty()?.is_empty());
    assert!(ann.entries_after(mark)?.is_empty(), "nothing written");
    ann.decline_pending(&set)?;
    assert!(
        notes
            .by_id(&drafted.id)?
            .is_some_and(|n| n.trashed.is_none())
    );
    for id in [&routine.id, &alone.id] {
        assert!(routines.by_id(id)?.is_some_and(|r| r.trashed.is_none()));
    }
    Ok(())
}

#[test]
fn the_files_leave_this_device_and_are_not_wanted_back() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let card = notes.capture_with_files(
        "Dentist",
        None,
        None,
        &[photo(1), photo(2)],
        &[voice(1)],
        &[LEASE],
    )?;
    let (shared, own) = match card.photos.as_slice() {
        [shared, own] => (shared.clone(), own.clone()),
        _ => return Err("two photos".into()),
    };
    let recording = card.recordings.first().ok_or("a recording")?.clone();
    let file = card.attachments.first().ok_or("a file")?.hash.clone();
    let target = notes.capture_with("Health", None, None, &[], &[voice(2)])?;
    let into = notes.capture_with("Into it", None, None, &[photo(3)], &[])?;
    notes.merge(&into.id, &target.id)?;
    let other = notes.capture_with("Same card", None, None, &[photo(1)], &[])?;
    notes.trash(&card.id)?;
    notes.trash(&target.id)?;

    let removed = Trash::new(&ann).empty()?;
    assert!(notes.by_id(&into.id)?.is_none());
    let from_target = target.recordings.first().ok_or("a recording")?.clone();
    let from_merged = into.photos.first().ok_or("a photo")?.clone();
    let mut expected = vec![
        format!("audio/{recording}"),
        format!("audio/{from_target}"),
        format!("file/{file}"),
        format!("photo/{own}"),
        format!("photo/{from_merged}"),
    ];
    expected.sort();
    assert_eq!(removed.blobs, expected);
    assert_eq!(ann.photo(&shared), Some(photo(1)), "another Note shows it");
    let shown = notes.by_id(&other.id)?.map(|n| n.photos);
    assert_eq!(shown, Some(vec![shared.clone()]));
    assert_eq!(ann.photo(&own), None);
    assert_eq!(ann.audio(&recording), None);
    assert_eq!(ann.audio(&from_target), None);
    assert_eq!(ann.file(&file), None);
    assert_eq!(ann.photo(&from_merged), None);
    assert!(ann.missing_blobs()?.is_empty());
    assert!(!ann.knows_blob(&format!("photo/{own}"))?);
    assert!(ann.knows_blob(&format!("photo/{shared}"))?);
    // Pushed back by another Device, they are not kept.
    assert!(!ann.knows_blob(&format!("file/{file}"))?);
    Ok(())
}

#[test]
fn a_sync_with_the_users_other_device_brings_nothing_back() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let desk = devices.device(2, "Ann")?;
    let notes = Notes::new(&ann);
    let card = notes.capture_with_files("Dentist", None, None, &[photo(1)], &[], &[LEASE])?;
    let p = card.photos.first().ok_or("a photo")?.clone();
    let file = card.attachments.first().ok_or("a file")?.hash.clone();
    exchange(&desk, &ann, None)?;
    assert!(desk.photo(&p).is_some());
    notes.trash(&card.id)?;
    exchange(&desk, &ann, None)?;
    Trash::new(&ann).empty()?;
    exchange(&desk, &ann, None)?;
    exchange(&ann, &desk, None)?;
    assert_eq!(ann.photo(&p), None);
    assert_eq!(ann.file(&file), None);
    assert!(ann.missing_blobs()?.is_empty());
    assert!(
        Notes::new(&desk).by_id(&card.id)?.is_none(),
        "the purge synced"
    );
    assert_eq!(desk.photo(&p), Some(photo(1)), "only this Device lets go");
    assert!(desk.missing_blobs()?.is_empty());
    Ok(())
}

#[test]
fn what_this_device_may_not_change_in_a_shared_area_stays_uncounted() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let area = ann.share(&Places::new(&ann).add_area("Household")?.id)?;
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    exchange(&ben, &ann, Some(&invite))?;
    let hers = Notes::new(&ann).capture_with("Hers", Some(&area), None, &[photo(1)], &[])?;
    let routine = Routines::new(&ann).spawn(&hers.id, &daily()?)?;
    let his = Notes::new(&ben).capture_with("His", Some(&area), None, &[photo(2)], &[])?;
    exchange(&ben, &ann, None)?;
    Routines::new(&ann).trash_note(&hers.id)?;
    Notes::new(&ben).trash(&his.id)?;
    exchange(&ben, &ann, None)?;
    let trashed: Vec<String> = Notes::new(&ben)
        .trashed()?
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(set(&trashed), set(&[hers.id.clone(), his.id.clone()]));

    let trash = Trash::new(&ben);
    let asked = trash.to_empty()?;
    assert_eq!(asked.notes, std::slice::from_ref(&his.id));
    assert!(asked.routines.is_empty(), "a Member may not manage it");
    let removed = trash.empty()?;
    assert_eq!(removed.notes, std::slice::from_ref(&his.id));
    assert!(removed.routines.is_empty());
    let his_photo = his.photos.first().ok_or("a photo")?.clone();
    let her_photo = hers.photos.first().ok_or("a photo")?.clone();
    assert_eq!(removed.blobs, [format!("photo/{his_photo}")]);
    let left: Vec<String> = Notes::new(&ben)
        .trashed()?
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(left, std::slice::from_ref(&hers.id));
    let kept = Routines::new(&ben).by_id(&routine.id)?;
    assert!(kept.is_some_and(|r| r.trashed.is_some()));
    assert_eq!(ben.photo(&her_photo), Some(photo(1)));
    assert_eq!(ben.photo(&his_photo), None);
    assert!(ben.missing_blobs()?.is_empty());
    Ok(())
}

#[test]
fn it_purges_per_batch_another_connection_writes_between_and_one_brought_back_stays() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("log.db");
    // One clock for both, as one Device: what the window writes is later.
    let devices = Devices::new();
    let open = || {
        let ticks = Arc::clone(&devices.0);
        let clock: Clock = Box::new(move || {
            let n = ticks.fetch_add(1, Ordering::SeqCst) + 1;
            let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).single();
            start.unwrap_or_default() + Duration::seconds(n)
        });
        Store::open(&path, clock, || Ok(SigningKey::from_seed([1; 32])))
    };
    let (emptying, window) = (open()?, open()?);
    let writing = Notes::new(&emptying);
    let (card, reminder) = emptying.batch(|| {
        let card = writing.capture_with("Card", None, None, &[photo(1)], &[])?;
        writing.trash(&card.id)?;
        for i in 0..2 * PER_BATCH {
            writing.trash(&writing.capture(&format!("Note {i}"), None, None)?.id)?;
        }
        let dentist = writing.capture("Dentist", None, None)?;
        let reminder = Routines::new(&emptying).spawn(&dentist.id, &daily()?)?;
        Routines::new(&emptying).trash_note(&dentist.id)?;
        Ok((card, reminder))
    })?;
    let trash = Trash::new(&emptying);
    let asked = trash.to_empty()?;
    let of = 2 * PER_BATCH + 2;
    assert_eq!(asked.notes.len(), of);
    assert_eq!(asked.routines, std::slice::from_ref(&reminder.id));
    // In the second batch.
    let back = asked.notes[PER_BATCH + 1].clone();
    let mark = emptying.current_seq()?;
    let told = std::cell::RefCell::new(Vec::new());
    let captured = std::cell::RefCell::new(Vec::new());
    let restored = std::cell::RefCell::new(Vec::new());
    // A write here waits for no lock the purge holds.
    let tell = |done: usize, of: usize| {
        told.borrow_mut().push((done, of));
        if done == PER_BATCH {
            restored
                .borrow_mut()
                .push(Routines::new(&window).restore_note(&back));
        }
        let made = Notes::new(&window).capture(&format!("Meanwhile {done}"), None, None);
        captured.borrow_mut().push(made.map(|n| n.id));
    };
    let started = std::time::Instant::now();
    let removed = trash.empty_stopping(&tell, &|| false)?;
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
    assert_eq!(
        told.take(),
        [(0, of), (PER_BATCH, of), (2 * PER_BATCH, of), (of, of)]
    );
    let captured = captured.take();
    assert_eq!(captured.len(), 4);
    for made in captured {
        assert!(writing.by_id(&made?)?.is_some());
    }
    let restored = restored.take();
    assert_eq!(restored.len(), 1);
    for done in restored {
        done?;
    }
    let expected: Vec<String> = asked
        .notes
        .iter()
        .filter(|id| **id != back)
        .cloned()
        .collect();
    assert_eq!(removed.notes, expected);
    assert_eq!(removed.routines, std::slice::from_ref(&reminder.id));
    let card_photo = card.photos.first().ok_or("a photo")?.clone();
    assert_eq!(removed.blobs, [format!("photo/{card_photo}")]);
    assert_eq!(emptying.photo(&card_photo), None);
    assert!(writing.by_id(&back)?.is_some_and(|n| !n.is_trashed()));
    // The purge's entries in the order one batch wrote them.
    let written: Vec<String> = emptying
        .entries_after(mark)?
        .into_iter()
        .filter(|e| e.field == note_keys::TYPE && e.value.is_none())
        .map(|e| e.entity)
        .collect();
    let purged: Vec<String> = removed
        .notes
        .iter()
        .chain(&removed.routines)
        .cloned()
        .collect();
    assert_eq!(written, purged);
    let again = emptying.current_seq()?;
    assert!(trash.empty()?.is_empty());
    assert!(emptying.entries_after(again)?.is_empty(), "nothing written");
    Ok(())
}

#[test]
fn stopped_what_it_purged_stays_purged_and_its_files_go() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let routines = Routines::new(&ann);
    let card = notes.capture_with("Card", None, None, &[photo(1)], &[])?;
    notes.trash(&card.id)?;
    for i in 0..PER_BATCH {
        notes.trash(&notes.capture(&format!("Note {i}"), None, None)?.id)?;
    }
    let alone = routines.spawn(&notes.capture("Pills", None, None)?.id, &daily()?)?;
    routines.trash(&alone.id)?;
    let trash = Trash::new(&ann);
    let asked = trash.to_empty()?;
    let batches = std::cell::Cell::new(0);
    let stop = || {
        batches.set(batches.get() + 1);
        batches.get() > 1
    };
    let removed = trash.empty_stopping(&|_, _| (), &stop)?;
    assert_eq!(removed.notes, asked.notes[..PER_BATCH]);
    assert!(removed.routines.is_empty());
    let card_photo = card.photos.first().ok_or("a photo")?.clone();
    assert_eq!(removed.blobs, [format!("photo/{card_photo}")]);
    assert_eq!(ann.photo(&card_photo), None);
    let last = asked.notes.last().ok_or("a Note")?.clone();
    let left: Vec<String> = notes.trashed()?.into_iter().map(|n| n.id).collect();
    assert_eq!(left, std::slice::from_ref(&last));
    let kept = routines.by_id(&alone.id)?;
    assert!(kept.is_some_and(|r| r.trashed.is_some()));

    let rest = trash.empty()?;
    assert_eq!(rest.notes, std::slice::from_ref(&last));
    assert_eq!(rest.routines, std::slice::from_ref(&alone.id));
    Ok(())
}
