//! The desktop's alarm layer over the Dart core's `routines` fixture
//! log, on a European wall clock and a clock the test moves: what shows
//! when, the alarm sound the app rings itself (#102), Re-nag, Escalation,
//! the Nudge in and out of Quiet Hours, Done and Snooze from a
//! notification, alerts off, the one Catch-up Notification after the app
//! was not running, and the Changes Nudge for a person synced by bundles
//! (#109).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use oldbrainfix_core::offers::Transport;
use oldbrainfix_core::{
    AreaRole, ClockTime, Day, DeviceSettings, Notes, Occurrence, OwnProfile, Places, Routines,
    SigningKey, Store, WireEntry,
};
use oldbrainfix_gui::alerts::{
    Alerts, CATCH_UP_KEY, CHANGES_KEY, DONE, NUDGE_KEY, Notice, OPEN, SNOOZE_10, Tap,
    occurrence_key,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::notify::{Recorded, RecordingNotifier};
use oldbrainfix_gui::ring::{OWN_SOUND, RecordingRinger};

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

/// A mono 16-bit WAV of a tenth of a second of 440 Hz.
fn tone_wav() -> Vec<u8> {
    let samples: Vec<i16> = (0..800)
        .map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / 8000.0).sin() * 8000.0) as i16)
        .collect();
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&8000u32.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

#[test]
fn an_occurrence_rings_the_alarm_sound_itself_with_a_silent_notification() -> TestResult {
    let (store, clock) = store()?;
    let record = RecordingNotifier::default();
    let ringer = RecordingRinger::default();
    let mut alerts = alerts(&record);
    alerts.set_ringer(Box::new(ringer.clone()));
    let pills = pills(&store)?;
    let eight = Occurrence::new(Day::new(2026, 10, 26), ClockTime::new(8, 0));
    let key = occurrence_key(&pills, &eight);
    // 08:00 on a fresh install: a silent notification, and the app rings
    // its own chime once for the tick.
    clock.set("2026-10-26T07:00:05.000Z")?;
    alerts.tick(&store, &berlin)?;
    let rang = record.take();
    let notice = shown(&rang)
        .into_iter()
        .find(|n| n.key == key)
        .ok_or("the pills rang")?;
    assert!(notice.silent && !notice.urgent);
    // The Nudge and the other notifications keep their own sound.
    assert!(
        shown(&rang)
            .iter()
            .filter(|n| !n.key.contains('@'))
            .all(|n| !n.silent)
    );
    assert_eq!(ringer.take(), [OWN_SOUND.to_vec()]);
    // A sound file chosen rings at the Re-nag.
    let dir = tempfile::tempdir()?;
    let bell = dir.path().join("bell.wav");
    std::fs::write(&bell, tone_wav())?;
    DeviceSettings::new(&store).set_alarm_sound(bell.to_str())?;
    clock.set("2026-10-26T07:05:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(
        shown(&record.take())
            .iter()
            .any(|n| n.key == key && n.silent)
    );
    assert_eq!(ringer.take(), [tone_wav()]);
    // The file gone: the own chime rings in its place.
    std::fs::remove_file(&bell)?;
    clock.set("2026-10-26T07:10:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    assert!(shown(&record.take()).iter().any(|n| n.key == key));
    assert_eq!(ringer.take(), [OWN_SOUND.to_vec()]);
    // Escalated at 08:20: urgent with the system's alarm sound; the app
    // rings only for another Occurrence still ringing silently.
    clock.set("2026-10-26T07:20:01.000Z")?;
    alerts.tick(&store, &berlin)?;
    let escalated = record.take();
    let notice = shown(&escalated)
        .into_iter()
        .find(|n| n.key == key)
        .ok_or("the pills escalated")?;
    assert!(notice.urgent && !notice.silent);
    assert_eq!(
        !ringer.take().is_empty(),
        shown(&escalated).iter().any(|n| n.silent)
    );
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

/// Ann on the clock the test sets, who shared her Household with Ben by
/// bundles: her store, the clock, Ben's Profile id and the Area's id.
fn shared_by_bundles() -> Result<(Store, Clock, String, String), Box<dyn std::error::Error>> {
    let clock = Clock(Arc::new(AtomicI64::new(0)));
    clock.set("2026-10-26T09:00:00.000Z")?;
    let read = clock.0.clone();
    let ann = Store::in_memory(
        Box::new(move || {
            Utc.timestamp_millis_opt(read.load(Ordering::SeqCst))
                .single()
                .unwrap_or_default()
        }),
        || Ok(SigningKey::from_seed([1; 32])),
    )?;
    OwnProfile::new(&ann).apply_defaults("Device", None)?;
    OwnProfile::new(&ann).rename("Ann")?;
    let then = oldbrainfix_core::parse_time("2026-10-26T08:00:00.000Z")?;
    let ben = Store::in_memory(Box::new(move || then), || {
        Ok(SigningKey::from_seed([2; 32]))
    })?;
    OwnProfile::new(&ben).apply_defaults("Device", None)?;
    OwnProfile::new(&ben).rename("Ben")?;
    ann.open_profile_card(&ben.profile_card(false)?.ok_or("his card")?, false)?;
    let ben_id = OwnProfile::new(&ben).get()?.ok_or("his Profile")?.id;
    let house = Places::new(&ann).add_area("Household")?;
    let shared = ann.share_area_with(&house.id, &ben_id, AreaRole::Member, Transport::Bundle)?;
    Ok((ann, clock, ben_id, shared))
}

#[test]
fn the_changes_nudge_names_who_waits_and_closes_once_sent() -> TestResult {
    let (ann, clock, ben, house) = shared_by_bundles()?;
    let record = RecordingNotifier::default();
    let mut alerts = alerts(&record);
    let wanted = Arc::new(AtomicBool::new(false));
    alerts.set_changes_wanted(wanted.clone());
    // The offer carried the history: nobody waits yet.
    clock.set("2026-10-26T10:00:00.000Z")?;
    alerts.tick(&ann, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != CHANGES_KEY));
    // A Note in the shared Area at 11:00 in Europe: due 30 minutes on.
    Notes::new(&ann).capture("Buy milk", Some(&house), None)?;
    clock.set("2026-10-26T10:10:00.000Z")?;
    let next = alerts.tick(&ann, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != CHANGES_KEY));
    assert!(next <= oldbrainfix_core::parse_time("2026-10-26T10:30:00.000Z")?);
    clock.set("2026-10-26T10:30:05.000Z")?;
    alerts.tick(&ann, &berlin)?;
    let record_now = record.take();
    let notice = shown(&record_now)
        .into_iter()
        .find(|n| n.key == CHANGES_KEY)
        .cloned()
        .ok_or("the Changes Nudge")?;
    assert_eq!(notice.title, "Changes for Ben");
    assert_eq!(notice.body, "Click to send them.");
    assert_eq!(notice.actions, [(OPEN.to_string(), "Send".to_string())]);
    assert!(!notice.urgent);

    // A click asks the window to send the bundles and keeps it.
    let at = clock.set("2026-10-26T10:31:00.000Z")?;
    alerts.apply(
        &ann,
        &Tap {
            key: CHANGES_KEY.into(),
            action: OPEN.into(),
            at,
        },
    )?;
    assert!(wanted.load(Ordering::SeqCst));
    assert!(alerts.open_requested);
    alerts.tick(&ann, &berlin)?;
    assert!(
        !record
            .take()
            .iter()
            .any(|r| matches!(r, Recorded::Closed(k) if k == CHANGES_KEY))
    );

    // Sent: nobody waits, the notification closes.
    ann.note_changes_sent(&ben, ann.current_seq()?)?;
    clock.set("2026-10-26T10:31:30.000Z")?;
    alerts.tick(&ann, &berlin)?;
    let closed = record.take();
    assert!(
        closed
            .iter()
            .any(|r| matches!(r, Recorded::Closed(k) if k == CHANGES_KEY))
    );
    assert!(shown(&closed).iter().all(|n| n.key != CHANGES_KEY));
    Ok(())
}

#[test]
fn the_changes_nudge_waits_out_quiet_hours_and_speaks_german() -> TestResult {
    let (ann, clock, _ben, house) = shared_by_bundles()?;
    let record = RecordingNotifier::default();
    let mut alerts = Alerts::new(
        Box::new(record.clone()),
        L10n::new("de"),
        Arc::new(AtomicU64::new(0)),
    );
    // A Note at 23:00 in Europe, within the Quiet Hours.
    clock.set("2026-10-26T22:00:00.000Z")?;
    Notes::new(&ann).capture("Buy milk", Some(&house), None)?;
    clock.set("2026-10-26T22:40:00.000Z")?;
    alerts.tick(&ann, &berlin)?;
    assert!(shown(&record.take()).iter().all(|n| n.key != CHANGES_KEY));
    // They end at 08:00: it comes then.
    clock.set("2026-10-27T07:00:10.000Z")?;
    alerts.tick(&ann, &berlin)?;
    let record_now = record.take();
    let notice = shown(&record_now)
        .into_iter()
        .find(|n| n.key == CHANGES_KEY)
        .cloned()
        .ok_or("the Changes Nudge")?;
    assert_eq!(notice.title, "Änderungen für Ben");
    assert_eq!(notice.body, "Klicke, um sie zu schicken.");
    // With alerts off it closes and shows nothing.
    DeviceSettings::new(&ann).set_alerts_on(false)?;
    clock.set("2026-10-27T09:00:10.000Z")?;
    alerts.tick(&ann, &berlin)?;
    let off = record.take();
    assert!(shown(&off).is_empty());
    assert!(
        off.iter()
            .any(|r| matches!(r, Recorded::Closed(k) if k == CHANGES_KEY))
    );
    Ok(())
}
