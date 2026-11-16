use super::*;
use crate::blocks::{BLOCKED, PREFIX as BLOCK_PREFIX};
use crate::keys::PERSONAL_SCOPE;
use crate::notes::keys as note_keys;
use crate::routines::keys as routine_keys;
use crate::signing::SigningKey;
use crate::sync::wire;
use crate::text_format::StoredText;
use crate::{ClockTime, Day, Notes, Places, Recurrence, Routines, Schedule};
use chrono::TimeZone;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One clock for every Device of a test, a second further at each
/// reading, so what was written later wins everywhere.
fn clock_of(ticks: &Arc<AtomicI64>) -> crate::Clock {
    let ticks = ticks.clone();
    Box::new(move || {
        let n = ticks.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).single();
        start.unwrap_or_default() + chrono::Duration::seconds(n)
    })
}

fn device(ticks: &Arc<AtomicI64>, seed: u8, author: &str) -> Made<Store> {
    let store = Store::in_memory(clock_of(ticks), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    store.set_local_setting("author", author)?;
    Ok(store)
}

struct Two {
    ann: Store,
    bob: Store,
}

fn two() -> Made<Two> {
    let ticks = Arc::new(AtomicI64::new(0));
    let ann = device(&ticks, 1, "Ann")?;
    let bob = device(&ticks, 2, "Bob")?;
    Ok(Two { ann, bob })
}

fn text(plain: &str) -> String {
    StoredText::plain(plain).encode()
}

fn text_of(store: &Store, note: &str) -> Made<Option<String>> {
    Ok(Notes::new(store).by_id(note)?.map(|n| n.text))
}

/// What the screens read of `store`: every Note's text, by id.
fn texts(store: &Store) -> Made<Vec<(String, String)>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| (n.id, n.text))
        .collect())
}

fn pills() -> Made<Schedule> {
    Ok(Schedule::new(
        Day::new(2026, 3, 1),
        &[ClockTime::new(8, 0).ok_or("a time")?],
        Some(Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e:?}"))?),
    )
    .ok_or("a schedule")?)
}

/// `to` takes in every entry `from` holds, with its keys.
fn hand_over(from: &Store, to: &Store) -> TestResult {
    to.apply_entries(
        &wire(&from.entries_since(&HashMap::new())?),
        &from.key_records()?,
        None,
        &mut ImportReport::default(),
    )?;
    Ok(())
}

fn wires(store: &Store) -> Made<Vec<WireEntry>> {
    Ok(wire(&store.all_entries()?))
}

#[test]
fn a_shown_set_reads_as_stored_and_the_log_sees_none_of_it() -> TestResult {
    let Two { ann, .. } = two()?;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    let entries = ann.all_entries()?.len();
    let vector = ann.version_vector()?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;
    let made = format!("note:{}", ann.new_id()?);
    let captured = dart_iso(ann.now());
    for (field, value) in [
        (note_keys::TYPE, note_keys::TYPE_NOTE.to_string()),
        (note_keys::CAPTURED, captured),
        (note_keys::TEXT, text("Bread")),
    ] {
        ann.draft_pending(&id, &made, field, Some(&value), None)?;
    }

    assert_eq!(text_of(&ann, &milk.id)?.as_deref(), Some("Oat milk"));
    assert_eq!(text_of(&ann, &made)?.as_deref(), Some("Bread"));
    assert_eq!(ann.all_entries()?.len(), entries);
    assert_eq!(ann.version_vector()?, vector);
    assert!(ann.entries_since(&vector)?.is_empty());
    assert_eq!(ann.logged(&milk.id, note_keys::TEXT)?, Some(text("Milk")));
    Ok(())
}

#[test]
fn a_set_not_shown_is_read_only_through_itself() -> TestResult {
    let Two { ann, .. } = two()?;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    let id = ann.open_pending("sent-item", false, Some("{}"))?;
    ann.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;

    assert_eq!(text_of(&ann, &milk.id)?.as_deref(), Some("Milk"));
    let set = ann.pending_set(&id)?.ok_or("the set")?;
    assert_eq!(set.kind, "sent-item");
    assert_eq!(set.payload.as_deref(), Some("{}"));
    assert_eq!(set.drafts()[0].value, Some(text("Oat milk")));
    Ok(())
}

#[test]
fn accept_writes_them_numbered_and_signed_then_dated_when_drafted() -> TestResult {
    let Two { ann, bob, .. } = two()?;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    let routine = Routines::new(&ann).spawn(&milk.id, &pills()?)?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;
    ann.draft_pending(&id, &routine.id, routine_keys::RENAG, Some("15"), None)?;
    let drafted: Vec<PendingDraft> = ann
        .pending_set(&id)?
        .ok_or("the set")?
        .drafts()
        .into_iter()
        .cloned()
        .collect();
    let before = texts(&ann)?;
    let renag = ann.current_fields(&routine.id)?;
    let own = ann.device_id()?;
    let max = ann.version_vector()?.get(&own).copied().unwrap_or(0);

    let accepted = ann.accept_pending(&id)?;

    assert!(accepted.kept.is_empty());
    assert!(accepted.taken.is_empty());
    let dseqs: Vec<i64> = accepted.written.iter().map(|e| e.dseq).collect();
    assert_eq!(dseqs, [max + 1, max + 2]);
    for (e, d) in accepted.written.iter().zip(&drafted) {
        assert_eq!(e.device, own);
        assert_eq!(e.date_at, d.made);
        assert!(e.sig.is_some());
    }
    assert_eq!(texts(&ann)?, before);
    assert_eq!(ann.current_fields(&routine.id)?, renag);
    assert!(ann.pending_set(&id)?.is_none());
    assert!(ann.pending_sets()?.is_empty());
    // What the log now holds verifies on another Device.
    hand_over(&ann, &bob)?;
    assert_eq!(text_of(&bob, &milk.id)?.as_deref(), Some("Oat milk"));
    Ok(())
}

#[test]
fn decline_leaves_the_log_as_it_was() -> TestResult {
    let Two { ann, .. } = two()?;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    let entries = wires(&ann)?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;

    assert!(ann.decline_pending(&id)?);

    assert_eq!(wires(&ann)?, entries);
    assert_eq!(text_of(&ann, &milk.id)?.as_deref(), Some("Milk"));
    assert!(ann.pending_sets()?.is_empty());
    assert!(!ann.decline_pending(&id)?);
    Ok(())
}

#[test]
fn own_edits_made_meanwhile_win_a_whole_schedule_too_and_the_review_lists_them() -> TestResult {
    let Two { ann, .. } = two()?;
    let notes = Notes::new(&ann);
    let milk = notes.capture("Milk", None, None)?;
    let bread = notes.capture("Bread", None, None)?;
    let routine = Routines::new(&ann).spawn(&milk.id, &pills()?)?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;
    ann.draft_pending(
        &id,
        &routine.id,
        routine_keys::RULE,
        Some("FREQ=WEEKLY"),
        None,
    )?;
    // The user edits the Note and the Routine's start, then the AI
    // edits the other Note, after the user did.
    notes.edit(&milk.id, "Milk, two litres")?;
    notes.edit(&bread.id, "Brown bread")?;
    ann.append(&routine.id, routine_keys::START, Some("2026-03-02"), None)?;
    ann.draft_pending(
        &id,
        &bread.id,
        note_keys::TEXT,
        Some(&text("Rye bread")),
        None,
    )?;

    let accepted = ann.accept_pending(&id)?;

    assert_eq!(
        accepted.kept,
        [
            KeptEdit {
                entity: milk.id.clone(),
                field: note_keys::TEXT.into(),
                pending: Some(text("Oat milk")),
                kept: Some(text("Milk, two litres")),
            },
            KeptEdit {
                entity: routine.id.clone(),
                field: routine_keys::RULE.into(),
                pending: Some("FREQ=WEEKLY".into()),
                kept: Some("FREQ=DAILY".into()),
            },
        ]
    );
    let entities: Vec<&str> = accepted.written.iter().map(|e| e.entity.as_str()).collect();
    assert_eq!(entities, [bread.id.as_str()]);
    assert_eq!(
        text_of(&ann, &milk.id)?.as_deref(),
        Some("Milk, two litres")
    );
    assert_eq!(text_of(&ann, &bread.id)?.as_deref(), Some("Rye bread"));
    assert_eq!(
        ann.current(&routine.id, routine_keys::RULE)?.as_deref(),
        Some("FREQ=DAILY")
    );
    Ok(())
}

#[test]
fn the_reads_show_what_accept_keeps_not_a_rule_the_user_changed_the_schedule_after() -> TestResult {
    let Two { ann, .. } = two()?;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    let routine = Routines::new(&ann).spawn(&milk.id, &pills()?)?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &routine.id,
        routine_keys::RULE,
        Some("FREQ=WEEKLY"),
        None,
    )?;
    assert_eq!(
        ann.current(&routine.id, routine_keys::RULE)?.as_deref(),
        Some("FREQ=WEEKLY")
    );
    ann.append(&routine.id, routine_keys::START, Some("2026-03-02"), None)?;
    let shown = ann.current_fields(&routine.id)?;

    assert_eq!(shown.get(routine_keys::RULE), Some("FREQ=DAILY"));
    ann.accept_pending(&id)?;
    assert_eq!(ann.current_fields(&routine.id)?, shown);
    Ok(())
}

#[test]
fn an_edit_another_person_wrote_meanwhile_does_not_stop_a_draft() -> TestResult {
    let Two { ann, bob, .. } = two()?;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    hand_over(&ann, &bob)?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;
    Notes::new(&bob).edit(&milk.id, "Milk from Bob")?;
    hand_over(&bob, &ann)?;

    let accepted = ann.accept_pending(&id)?;

    assert!(accepted.kept.is_empty());
    assert_eq!(accepted.written.len(), 1);
    Ok(())
}

fn reopen(dir: &std::path::Path, ticks: &Arc<AtomicI64>) -> Made<Store> {
    let store = Store::open(&dir.join("log.db"), clock_of(ticks), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?;
    store.set_local_setting("author", "Ann")?;
    Ok(store)
}

#[test]
fn a_restart_finds_the_set_waiting_as_it_was() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ticks = Arc::new(AtomicI64::new(0));
    let store = reopen(dir.path(), &ticks)?;
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let id = store.open_pending("ai-task", true, None)?;
    store.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;
    let file = dir.path().join("pending").join(format!("{id}.json"));
    let written = std::fs::read_to_string(&file)?;
    drop(store);

    let again = reopen(dir.path(), &ticks)?;
    assert_eq!(again.pending_set(&id)?.ok_or("the set")?.encode()?, written);
    assert_eq!(text_of(&again, &milk.id)?.as_deref(), Some("Oat milk"));
    again.accept_pending(&id)?;
    assert!(!file.exists());
    assert_eq!(
        again.logged(&milk.id, note_keys::TEXT)?,
        Some(text("Oat milk"))
    );
    Ok(())
}

#[test]
fn a_restart_after_an_accept_cut_short_before_its_file_went_finds_none() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ticks = Arc::new(AtomicI64::new(0));
    let store = reopen(dir.path(), &ticks)?;
    let id = store.open_pending("ai-task", true, None)?;
    store.draft_pending(&id, "note:x", note_keys::TEXT, Some(&text("Lost?")), None)?;
    // What an Accept writes with its entries, before the file goes.
    store.set_local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"), "accepted")?;
    drop(store);

    let again = reopen(dir.path(), &ticks)?;
    assert!(again.pending_sets()?.is_empty());
    assert!(
        !dir.path()
            .join("pending")
            .join(format!("{id}.json"))
            .exists()
    );
    assert!(
        again
            .local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"))?
            .is_none()
    );
    Ok(())
}

#[test]
fn a_second_store_on_the_same_file_sees_what_the_other_did_to_a_set() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ticks = Arc::new(AtomicI64::new(0));
    let app = reopen(dir.path(), &ticks)?;
    let milk = Notes::new(&app).capture("Milk", None, None)?;
    let id = app.open_pending("ai-task", true, None)?;
    app.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Oat milk")),
        None,
    )?;
    let background = reopen(dir.path(), &ticks)?;
    // The app adds a draft the background store never read.
    app.draft_pending(
        &id,
        &milk.id,
        note_keys::IMPORTANCE,
        Some("important"),
        None,
    )?;
    background.draft_pending(
        &id,
        &milk.id,
        note_keys::TEXT,
        Some(&text("Rye milk")),
        None,
    )?;
    assert_eq!(app.pending_set(&id)?.ok_or("the set")?.drafts().len(), 3);

    app.accept_pending(&id)?;

    assert!(background.pending_set(&id)?.is_none());
    assert_eq!(text_of(&background, &milk.id)?.as_deref(), Some("Rye milk"));
    assert!(
        background
            .draft_pending(&id, &milk.id, note_keys::TEXT, Some(&text("No")), None)
            .is_err()
    );
    assert!(
        !dir.path()
            .join("pending")
            .join(format!("{id}.json"))
            .exists()
    );
    Ok(())
}

#[test]
fn a_restart_leaves_a_file_it_cannot_read_where_it_is_unread() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ticks = Arc::new(AtomicI64::new(0));
    drop(reopen(dir.path(), &ticks)?);
    let pending = dir.path().join("pending");
    std::fs::create_dir_all(&pending)?;
    let file = pending.join("abc.json");
    std::fs::write(&file, r#"{"changes":[],"v":2}"#)?;

    let again = reopen(dir.path(), &ticks)?;
    assert!(again.pending_sets()?.is_empty());
    assert!(file.exists());
    Ok(())
}

#[test]
fn received_entries_are_checked_as_they_come_pinned_nothing_and_taken_in_at_accept() -> TestResult {
    let Two { ann, bob, .. } = two()?;
    let note = Notes::new(&bob).capture("From Bob", None, None)?;
    let entries = wire(&bob.entries_since(&HashMap::new())?);
    let (last, rest) = entries.split_last().ok_or("entries")?;
    let mut forged = last.clone();
    forged.value = Some(text("Forged"));
    let mut mine = entries[0].clone();
    mine.device = ann.device_id()?;
    mine.dseq = 99;
    let mut report = ImportReport::default();
    let id = ann.open_pending("bundle", true, None)?;
    let mut first = rest.to_vec();
    first.extend([forged, mine]);

    let taken = ann.receive_pending(&id, &first, &[bob.own_key_record()?], &mut report)?;

    assert_eq!(taken.len(), rest.len());
    assert_eq!(report.refused_count(), 2);
    assert!(ann.pinned_key(&bob.device_id()?)?.is_none());
    assert!(!ann.holds(&bob.device_id()?, 1)?);
    assert!(text_of(&ann, &note.id)?.is_some());
    // The rest comes later; one already in the set is not taken twice.
    let again = ann.receive_pending(
        &id,
        &entries,
        &[bob.own_key_record()?],
        &mut ImportReport::default(),
    )?;
    assert_eq!(again.len(), 1);

    let accepted = ann.accept_pending(&id)?;

    assert_eq!(accepted.taken.len(), entries.len());
    assert!(accepted.written.is_empty());
    assert!(ann.pinned_key(&bob.device_id()?)?.is_some());
    assert_eq!(text_of(&ann, &note.id)?.as_deref(), Some("From Bob"));
    assert_eq!(
        ann.version_vector()?.get(&bob.device_id()?).copied(),
        Some(last.dseq)
    );
    Ok(())
}

/// Bob edits Ann's Note; Ann takes his edit into a set, shown or not,
/// and edits the Note herself.
fn edited_over(shown: bool) -> Made<(Two, String, AcceptedChanges)> {
    let two = two()?;
    let (ann, bob) = (&two.ann, &two.bob);
    let milk = Notes::new(ann).capture("Milk", None, None)?;
    hand_over(ann, bob)?;
    Notes::new(bob).edit(&milk.id, "Milk from Bob")?;
    let id = ann.open_pending(if shown { "bundle" } else { "area-offer" }, shown, None)?;
    ann.receive_pending(
        &id,
        &wire(&bob.entries_since(&ann.version_vector()?)?),
        &bob.key_records()?,
        &mut ImportReport::default(),
    )?;
    if shown {
        assert_eq!(text_of(ann, &milk.id)?.as_deref(), Some("Milk from Bob"));
    }
    Notes::new(ann).edit(&milk.id, "Milk, two litres")?;
    assert!(ann.conflicts()?.is_empty());
    let accepted = ann.accept_pending(&id)?;
    assert!(ann.conflicts()?.is_empty());
    assert_eq!(
        accepted.kept,
        [KeptEdit {
            entity: milk.id.clone(),
            field: note_keys::TEXT.into(),
            pending: Some(text("Milk from Bob")),
            kept: Some(text("Milk, two litres")),
        }]
    );
    assert_eq!(text_of(ann, &milk.id)?.as_deref(), Some("Milk, two litres"));
    Ok((two, milk.id, accepted))
}

#[test]
fn received_entries_give_way_to_an_own_edit_made_over_them_which_saw_them() -> TestResult {
    let (_, _, accepted) = edited_over(true)?;
    assert!(accepted.written.is_empty());
    Ok(())
}

#[test]
fn received_entries_of_a_set_not_shown_give_way_written_again_so_no_conflict_stands() -> TestResult
{
    let (_, _, accepted) = edited_over(false)?;
    assert_eq!(accepted.written.len(), 1);
    assert_eq!(accepted.written[0].value, Some(text("Milk, two litres")));
    Ok(())
}

#[test]
fn a_pending_change_is_trusted_for_nothing_no_block_no_area() -> TestResult {
    let Two { ann, .. } = two()?;
    let carol = ann.new_id()?;
    let area = Places::new(&ann).add_area("Household")?.id;
    let milk = Notes::new(&ann).capture("Milk", None, None)?;
    let id = ann.open_pending("ai-task", true, None)?;
    ann.draft_pending(
        &id,
        &format!("{BLOCK_PREFIX}{carol}"),
        BLOCKED,
        Some("Carol"),
        None,
    )?;
    ann.draft_pending(&id, &milk.id, note_keys::PLACE, Some(&area), None)?;

    assert!(ann.blocked_ids()?.is_empty());
    assert_eq!(ann.scope_of(&milk.id)?, PERSONAL_SCOPE);
    assert_eq!(
        Notes::new(&ann).by_id(&milk.id)?.and_then(|n| n.place),
        Some(area)
    );
    Ok(())
}
