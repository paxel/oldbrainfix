//! Two connections to one database, as the desktop's Agenda reads on a
//! thread of its own beside the window's store (#207): a batch holds the
//! write lock from its start, an append takes its number in one
//! transaction, and today's picks are checked and written in one, so
//! neither connection writes a pick the other wrote meanwhile.

use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use oldbrainfix_core::frogs::Frogs;
use oldbrainfix_core::notes::Notes;
use oldbrainfix_core::schedule::{Day, Frequency, Recurrence, Schedule};
use oldbrainfix_core::sessions::{SessionRule, Sessions};
use oldbrainfix_core::signing::SigningKey;
use oldbrainfix_core::store::Store;
use oldbrainfix_core::tallies::Tallies;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// How long the first connection holds still where the second one may
/// get in between: long enough for the second to start, far below the
/// busy timeout it waits for a lock with.
const HOLD: Duration = Duration::from_millis(500);

/// How long any wait for the other thread lasts before the test fails
/// instead of hanging.
const PATIENCE: Duration = Duration::from_secs(30);

/// The wall clock as the store's: the day as UTC shows it.
fn wall(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.naive_utc()
}

/// What the second connection does once told to go.
#[derive(Clone, Copy)]
enum Job {
    Append,
    Frogs,
    Sessions,
}

/// A second connection to `path` on a thread of its own: sending on the
/// first lets its job run, the second brings back what it failed with.
fn second(path: &Path, job: Job) -> (mpsc::Sender<()>, mpsc::Receiver<Option<String>>) {
    let (go, orders) = mpsc::channel::<()>();
    let (answer, answers) = mpsc::channel();
    let path = path.to_path_buf();
    thread::spawn(move || {
        let run = || -> oldbrainfix_core::Result<()> {
            let store = Store::open(&path, Box::new(Utc::now), SigningKey::generate)?;
            if orders.recv_timeout(PATIENCE).is_err() {
                return Err(oldbrainfix_core::Error::Invalid("never told to go".into()));
            }
            match job {
                Job::Append => store
                    .append("note:second", "text", Some("second"), None)
                    .map(drop),
                Job::Frogs => Frogs::new(&store).pick_today(&wall).map(drop),
                Job::Sessions => Sessions::new(&store).pick_today(&wall).map(drop),
            }
        };
        drop(answer.send(run().err().map(|e| e.to_string())));
    });
    (go, answers)
}

/// What a clock lets in once.
type Between = Arc<Mutex<Option<Box<dyn FnOnce() + Send>>>>;

/// A store on `path` whose clock lets what is put into the returned slot
/// in the first time an append asks it for now: after what the append
/// was worked out from was read, before it is written.
fn first(path: &Path) -> oldbrainfix_core::Result<(Store, Between)> {
    let slot: Between = Arc::new(Mutex::new(None));
    let waiting = slot.clone();
    let clock = Box::new(move || {
        let mut held = waiting.lock().unwrap_or_else(|e| e.into_inner());
        if held.is_some()
            && std::backtrace::Backtrace::force_capture()
                .to_string()
                .contains("Store>::append")
            && let Some(between) = held.take()
        {
            drop(held);
            between();
        }
        Utc::now()
    });
    Ok((Store::open(path, clock, SigningKey::generate)?, slot))
}

/// Tells the second connection to go, then holds still for [`HOLD`].
fn let_in(go: mpsc::Sender<()>) -> Box<dyn FnOnce() + Send> {
    Box::new(move || {
        let _ = go.send(());
        thread::sleep(HOLD);
    })
}

#[test]
fn a_batch_holds_the_write_lock_from_its_start_and_an_append_beside_it_takes_the_next_number()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    let a = Store::open(&path, Box::new(Utc::now), SigningKey::generate)?;
    a.append("note:first", "text", Some("before"), None)?;
    let (go, answers) = second(&path, Job::Append);
    a.batch(|| {
        // Read in the batch, as a check before a write is.
        a.current_seq()?;
        let_in(go)();
        a.append("note:first", "text", Some("after"), None)
            .map(drop)
    })?;
    assert_eq!(answers.recv_timeout(PATIENCE)?, None);
    let mut numbers: Vec<i64> = a.all_entries()?.iter().map(|e| e.dseq).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, [1, 2, 3]);
    assert_eq!(a.current("note:first", "text")?.as_deref(), Some("after"));
    assert_eq!(a.current("note:second", "text")?.as_deref(), Some("second"));
    Ok(())
}

#[test]
fn two_connections_picking_todays_frogs_at_once_write_one_pick() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    {
        let setup = Store::open(&path, Box::new(Utc::now), SigningKey::generate)?;
        for text in ["Call mum", "Tax return"] {
            let note = Notes::new(&setup).capture(text, None, None)?;
            Frogs::new(&setup).spawn(&note.id)?;
        }
    }
    let (a, slot) = first(&path)?;
    let (go, answers) = second(&path, Job::Frogs);
    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(let_in(go));
    Frogs::new(&a).pick_today(&wall)?;
    let let_in_once = slot.lock().unwrap_or_else(|e| e.into_inner()).is_none();
    assert!(let_in_once, "the second connection was let in");
    assert_eq!(answers.recv_timeout(PATIENCE)?, None);
    let picks = a
        .all_entries()?
        .into_iter()
        .filter(|e| e.entity == "frogs:pick")
        .count();
    assert_eq!(picks, 1);
    Ok(())
}

#[test]
fn two_connections_picking_todays_session_at_once_write_one_pick() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    {
        let setup = Store::open(&path, Box::new(Utc::now), SigningKey::generate)?;
        let mut pool = Vec::new();
        for name in ["Squats", "Push-ups", "Plank"] {
            let note = Notes::new(&setup).capture(name, None, None)?;
            Tallies::new(&setup).add_field(&note.id, "Times", "times", None, None)?;
            pool.push(note.id);
        }
        let rule = Some(Recurrence::every(Frequency::Daily, 1));
        let daily = Schedule::new(Day::new(2026, 1, 1), &[], rule).ok_or("a schedule")?;
        let training = Notes::new(&setup).capture("Training", None, None)?;
        Sessions::new(&setup).spawn(&training.id, &daily, &pool, 2, SessionRule::Rolling)?;
    }
    let (a, slot) = first(&path)?;
    let (go, answers) = second(&path, Job::Sessions);
    *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(let_in(go));
    let shown = Sessions::new(&a).pick_today(&wall)?;
    let let_in_once = slot.lock().unwrap_or_else(|e| e.into_inner()).is_none();
    assert!(let_in_once, "the second connection was let in");
    assert_eq!(answers.recv_timeout(PATIENCE)?, None);
    assert_eq!(shown.len(), 1);
    let picks = a
        .all_entries()?
        .into_iter()
        .filter(|e| e.field.starts_with("pick@"))
        .count();
    assert_eq!(picks, 1);
    Ok(())
}
