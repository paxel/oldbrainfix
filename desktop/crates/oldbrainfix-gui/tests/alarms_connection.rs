//! The alarm layer on a connection of its own (obf-1091), as the app
//! runs it beside the window's: what the window writes on its own
//! connection reaches the alarms, the rights among it worked out anew
//! once the log moved, and a Profile kept elsewhere once a wake says so.
//! Over a store on disk, as two connections need.

use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use oldbrainfix_core::offers::Transport;
use oldbrainfix_core::{AreaRole, DeviceSettings, Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::alerts::{self, Alerts, CHANGES_KEY, Event, Notice};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::notify::{Recorded, RecordingNotifier};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// A clock the test sets, shared by every store.
#[derive(Clone, Default)]
struct Clock(Arc<AtomicI64>);

impl Clock {
    fn set(&self, at: &str) -> Result<(), Box<dyn std::error::Error>> {
        let at = oldbrainfix_core::parse_time(at)?;
        self.0.store(at.timestamp_millis(), Ordering::SeqCst);
        Ok(())
    }

    fn boxed(&self) -> oldbrainfix_core::store::Clock {
        let read = self.0.clone();
        Box::new(move || {
            Utc.timestamp_millis_opt(read.load(Ordering::SeqCst))
                .single()
                .unwrap_or_default()
        })
    }
}

/// Ann's store on the database at `path`: a connection of its own each
/// time.
fn open(path: &Path, clock: &Clock) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, clock.boxed(), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?)
}

/// The window's connection and the alarms', Ann's with Ben's Profile
/// kept, and Ben's store with his Profile id.
struct Two {
    window: Store,
    alarms: Store,
    ben: Store,
    ben_id: String,
    clock: Clock,
    _dir: tempfile::TempDir,
}

fn two() -> Result<Two, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    let clock = Clock::default();
    clock.set("2026-10-26T09:00:00.000Z")?;
    let window = open(&path, &clock)?;
    let alarms = open(&path, &clock)?;
    OwnProfile::new(&window).apply_defaults("Device", None)?;
    OwnProfile::new(&window).rename("Ann")?;
    let ben = Store::in_memory(clock.boxed(), || Ok(SigningKey::from_seed([2; 32])))?;
    OwnProfile::new(&ben).apply_defaults("Device", None)?;
    OwnProfile::new(&ben).rename("Ben")?;
    window.open_profile_card(&ben.profile_card(false)?.ok_or("his card")?, false)?;
    let ben_id = OwnProfile::new(&ben).get()?.ok_or("his Profile")?.id;
    Ok(Two {
        window,
        alarms,
        ben,
        ben_id,
        clock,
        _dir: dir,
    })
}

/// The window shares Ann's Household with Ben by bundles and files a
/// Note there: due in the Changes Nudge half an hour on.
fn share(two: &Two) -> TestResult {
    let house = Places::new(&two.window).add_area("Household")?;
    let shared =
        two.window
            .share_area_with(&house.id, &two.ben_id, AreaRole::Member, Transport::Bundle)?;
    Notes::new(&two.window).capture("Buy milk", Some(&shared), None)?;
    Ok(())
}

fn alerts(record: &RecordingNotifier) -> Alerts {
    Alerts::new(
        Box::new(record.clone()),
        L10n::new("en"),
        Arc::new(AtomicU64::new(0)),
    )
}

fn changes_nudge(record: &[Recorded]) -> Option<Notice> {
    record.iter().find_map(|r| match r {
        Recorded::Shown(n) if n.key == CHANGES_KEY => Some(n.clone()),
        _ => None,
    })
}

#[test]
fn the_alarms_see_what_the_window_wrote_on_its_own_connection() -> TestResult {
    let two = two()?;
    let record = RecordingNotifier::default();
    let mut alerts = alerts(&record);
    // The alarms worked out the log once: nothing shared yet.
    two.clock.set("2026-10-26T10:00:00.000Z")?;
    alerts.tick(&two.alarms, &berlin)?;
    assert!(changes_nudge(&record.take()).is_none());

    // The Area shared and a Note filed there in the window, not on the
    // alarms' connection.
    share(&two)?;
    two.clock.set("2026-10-26T10:30:05.000Z")?;
    alerts.tick(&two.alarms, &berlin)?;
    let notice = changes_nudge(&record.take()).ok_or("the Changes Nudge")?;
    assert_eq!(notice.title, "Changes for Ben");
    Ok(())
}

#[test]
fn a_wake_tells_the_alarms_thread_of_a_profile_kept_in_the_window() -> TestResult {
    let two = two()?;
    share(&two)?;
    two.clock.set("2026-10-26T10:10:00.000Z")?;
    let record = RecordingNotifier::default();
    let mut alerts = alerts(&record);
    let (events, heard) = mpsc::channel::<Event>();
    let alarms = Arc::new(two.alarms);
    let running = {
        let alarms = alarms.clone();
        std::thread::spawn(move || alerts::run(&alarms, &berlin, &mut alerts, &heard, |_| {}))
    };
    // Its first tick read Ben's Profile, before the Changes Nudge is due.
    let settings = DeviceSettings::new(&two.window);
    let start = Instant::now();
    while settings.last_fired()?.is_none() {
        assert!(start.elapsed() < Duration::from_secs(10), "no tick");
        std::thread::sleep(Duration::from_millis(10));
    }

    // Ben renamed, his Profile kept in the window: a local setting, no
    // entry in the log.
    OwnProfile::new(&two.ben).rename("Benjamin")?;
    let before = two.window.current_seq()?;
    two.window
        .open_profile_card(&two.ben.profile_card(false)?.ok_or("his card")?, false)?;
    assert_eq!(two.window.current_seq()?, before);
    two.clock.set("2026-10-26T10:30:05.000Z")?;
    events.send(Event::Wake)?;
    let start = Instant::now();
    let notice = loop {
        if let Some(notice) = changes_nudge(&record.take()) {
            break notice;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "no Changes Nudge"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(notice.title, "Changes for Benjamin");
    drop(events);
    running.join().map_err(|_| "the alarms thread")?;
    Ok(())
}
