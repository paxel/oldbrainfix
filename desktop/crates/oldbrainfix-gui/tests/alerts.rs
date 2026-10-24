//! The desktop's alarm layer over the Dart core's `routines` fixture
//! log, on a European wall clock and a clock the test moves: what shows
//! when, Re-nag, Escalation, the Nudge in and out of Quiet Hours, Done and
//! Snooze from a notification, alerts off, and the one Catch-up
//! Notification after the app was not running.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use oldbrainfix_core::{
    ClockTime, Day, DeviceSettings, Occurrence, Routines, SigningKey, Store, WireEntry,
};
use oldbrainfix_gui::alerts::{
    Alerts, CATCH_UP_KEY, DONE, NUDGE_KEY, Notice, SNOOZE_10, Tap, occurrence_key,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::notify::{Recorded, RecordingNotifier};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// A clock the test sets.
#[derive(Clone)]
struct Clock(Arc<AtomicI64>);

impl Clock {
    fn set(&self, at: &str) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
        let at = oldbrainfix_core::parse_time(at)?;
        self.0.store(at.timestamp_millis(), Ordering::SeqCst);
        Ok(at)
    }
}

fn store() -> Result<(Store, Clock), Box<dyn std::error::Error>> {
    let clock = Clock(Arc::new(AtomicI64::new(0)));
    let read = clock.0.clone();
    let store = Store::in_memory(
        Box::new(move || {
            Utc.timestamp_millis_opt(read.load(Ordering::SeqCst))
                .single()
                .unwrap_or_default()
        }),
        || Ok(SigningKey::from_seed([9; 32])),
    )?;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/routines");
    for line in std::fs::read_to_string(dir.join("log.jsonl"))?.lines() {
        let entry: WireEntry = serde_json::from_str(line)?;
        store.insert(&entry)?;
    }
    Ok((store, clock))
}

fn shown(record: &[Recorded]) -> Vec<&Notice> {
    record
        .iter()
        .filter_map(|r| match r {
            Recorded::Shown(n) => Some(n),
            Recorded::Closed(_) => None,
        })
        .collect()
}

fn pills(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    let routines = Routines::new(store);
    for routine in routines.all()? {
        if routines
            .note(&routine)?
            .is_some_and(|n| n.text == "Take the pills")
            && routine.schedule.times.len() == 3
        {
            return Ok(routine.id);
        }
    }
    Err("the pills".into())
}

fn alerts(record: &RecordingNotifier) -> Alerts {
    Alerts::new(
        Box::new(record.clone()),
        L10n::new("en"),
        Arc::new(AtomicU64::new(0)),
    )
}

#[test]
fn an_occurrence_rings_renags_escalates_and_stops_when_done() -> TestResult {
    let (store, clock) = store()?;
    let record = RecordingNotifier::default();
    let mut alerts = alerts(&record);
    let pills = pills(&store)?;
    let monday = Day::new(2026, 10, 26);
    let eight = Occurrence::new(monday, ClockTime::new(8, 0));
    let key = occurrence_key(&pills, &eight);
    // Started at 07:59 in Europe: nothing yet.
    clock.set("2026-10-26T06:59:00.000Z")?;
    let next = alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != key));
    assert!(next <= oldbrainfix_core::parse_time("2026-10-26T07:00:00.000Z")?);
    // 08:00: it rings with Done and the snoozes.
    clock.set("2026-10-26T07:00:05.000Z")?;
    alerts.tick(&store, &berlin)?;
    let record_now = record.take();
    let rang = shown(&record_now);
    let notice = rang.iter().find(|n| n.key == key).ok_or("the pills rang")?;
    assert_eq!(notice.title, "Take the pills");
    assert_eq!(notice.body, "08:00");
    assert!(!notice.urgent);
    let actions: Vec<&str> = notice.actions.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        actions,
        ["done", "snooze10", "snooze30", "snooze60", "default"]
    );
    // Its own Re-nag is 5 minutes: again at 08:05, not before.
    clock.set("2026-10-26T07:03:00.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != key));
    clock.set("2026-10-26T07:05:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    let again = record.take();
    assert!(shown(&again).iter().any(|n| n.key == key && !n.urgent));
    // After three ignored Re-nags it escalates: 08:20.
    clock.set("2026-10-26T07:20:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    let escalated = record.take();
    assert!(shown(&escalated).iter().any(|n| n.key == key && n.urgent));
    // Done from the notification: checked off as of the tap, closed.
    let at = clock.set("2026-10-26T07:21:00.000Z")?;
    alerts.apply(
        &store,
        &Tap {
            key: key.clone(),
            action: DONE.into(),
            at,
        },
    )?;
    let routine = Routines::new(&store).by_id(&pills)?.ok_or("the pills")?;
    assert_eq!(routine.done_at(&eight), Some(at));
    assert!(record.take().contains(&Recorded::Closed(key.clone())));
    clock.set("2026-10-26T07:25:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != key));
    Ok(())
}

#[test]
fn a_snooze_rings_again_when_it_ends() -> TestResult {
    let (store, clock) = store()?;
    let record = RecordingNotifier::default();
    let mut alerts = alerts(&record);
    let pills = pills(&store)?;
    let two = Occurrence::new(Day::new(2026, 10, 26), ClockTime::new(14, 0));
    let key = occurrence_key(&pills, &two);
    clock.set("2026-10-26T13:00:05.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().any(|n| n.key == key));
    let at = clock.set("2026-10-26T13:01:00.000Z")?;
    alerts.apply(
        &store,
        &Tap {
            key: key.clone(),
            action: SNOOZE_10.into(),
            at,
        },
    )?;
    // Not at the Re-nag, but ten minutes after the tap.
    clock.set("2026-10-26T13:05:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != key));
    clock.set("2026-10-26T13:11:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().any(|n| n.key == key));
    Ok(())
}

#[test]
fn the_nudge_waits_out_quiet_hours_and_alerts_off_shows_nothing() -> TestResult {
    let (store, clock) = store()?;
    let record = RecordingNotifier::default();
    let mut alerts = alerts(&record);
    // 23:30 in Europe: within the Quiet Hours, no Nudge.
    clock.set("2026-10-26T22:30:00.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != NUDGE_KEY));
    // 08:00 they end, and the Nudge covers both Unsorted Notes.
    clock.set("2026-10-27T07:00:10.000Z")?;
    alerts.tick(&store, &berlin)?;
    let record_now = record.take();
    let nudge = shown(&record_now)
        .into_iter()
        .find(|n| n.key == NUDGE_KEY)
        .cloned()
        .ok_or("the Nudge")?;
    assert_eq!(nudge.title, "2 Notes are Unsorted");
    assert_eq!(nudge.body, "Click to sort them.");
    // With alerts off on this Device nothing shows, and nothing piles up
    // for when they are on again.
    DeviceSettings::new(&store).set_alerts_on(false)?;
    clock.set("2026-10-27T13:00:10.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).is_empty());
    DeviceSettings::new(&store).set_alerts_on(true)?;
    clock.set("2026-10-27T13:00:40.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != CATCH_UP_KEY));
    Ok(())
}

#[test]
fn what_was_missed_while_not_running_is_told_once() -> TestResult {
    let (store, clock) = store()?;
    let record = RecordingNotifier::default();
    // It last ran on Monday morning, then not until Wednesday noon.
    clock.set("2026-10-26T05:00:00.000Z")?;
    alerts(&record).tick(&store, &berlin)?;
    record.take();
    clock.set("2026-10-28T11:00:00.000Z")?;
    let mut later = alerts(&record);
    later.tick(&store, &berlin)?;
    let record_now = record.take();
    let shown = shown(&record_now);
    let catch_up: Vec<_> = shown.iter().filter(|n| n.key == CATCH_UP_KEY).collect();
    assert_eq!(catch_up.len(), 1);
    assert_eq!(
        catch_up[0].title,
        "Missed while Old Brain Fix was not running"
    );
    assert!(
        catch_up[0]
            .body
            .ends_with("alarms were Missed. Open the app to see them.")
    );
    // Nothing Missed rings late: Monday's pills are told, not rung.
    let pills = pills(&store)?;
    let monday = Occurrence::new(Day::new(2026, 10, 26), ClockTime::new(8, 0));
    assert!(
        shown
            .iter()
            .all(|n| n.key != occurrence_key(&pills, &monday))
    );
    // The next start has nothing to catch up.
    clock.set("2026-10-28T11:00:30.000Z")?;
    alerts(&record).tick(&store, &berlin)?;
    assert!(
        self::shown(&record.take())
            .iter()
            .all(|n| n.key != CATCH_UP_KEY)
    );
    Ok(())
}
