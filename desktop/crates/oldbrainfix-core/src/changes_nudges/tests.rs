use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Duration, NaiveDateTime, TimeZone};
use parking_lot::Mutex;

use super::*;
use crate::alarms::Alarms;
use crate::bundle::{import_bundle, write_area_offer_now, write_person_bundle_now};
use crate::nudges::{PlannedChangesNudge, changes_nudge_plan};
use crate::profile::OwnProfile;
use crate::shared::AreaRole;
use crate::signing::SigningKey;
use crate::{Notes, Places};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The wall clock of these tests: UTC.
fn utc(instant: DateTime<Utc>) -> NaiveDateTime {
    instant.naive_utc()
}

fn at_on(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, day, hour, minute, 0)
        .single()
        .unwrap_or_default()
}

fn at(hour: u32, minute: u32) -> DateTime<Utc> {
    at_on(2, hour, minute)
}

/// Ann on a held clock, at 09:00 on Monday, 2 March 2026; Quiet Hours
/// 22:00 to 08:00, a repeat every hour: the defaults.
struct Ann {
    now: Arc<Mutex<DateTime<Utc>>>,
    ann: Store,
    dir: tempfile::TempDir,
    files: usize,
}

fn device(now: &Arc<Mutex<DateTime<Utc>>>, seed: u8, name: &str) -> Made<Store> {
    let held = now.clone();
    let store = Store::in_memory(Box::new(move || *held.lock()), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    OwnProfile::new(&store).rename(name)?;
    Ok(store)
}

/// Ann, and Ben and Cat on her clock, each of whom swapped Profile
/// cards with her.
fn world() -> Made<(Ann, Store, Store)> {
    let now = Arc::new(Mutex::new(at(9, 0)));
    let ann = device(&now, 1, "Ann")?;
    let ben = device(&now, 2, "Ben")?;
    let cat = device(&now, 3, "Cat")?;
    for other in [&ben, &cat] {
        other.open_profile_card(&ann.profile_card(false)?.ok_or("a card")?, false)?;
        ann.open_profile_card(&other.profile_card(true)?.ok_or("a card")?, false)?;
    }
    let ann = Ann {
        now,
        ann,
        dir: tempfile::tempdir()?,
        files: 0,
    };
    Ok((ann, ben, cat))
}

impl Ann {
    fn set(&self, instant: DateTime<Utc>) {
        *self.now.lock() = instant;
    }

    fn path(&mut self) -> PathBuf {
        self.files += 1;
        self.dir.path().join(format!("c{}.obfsync", self.files))
    }

    /// Ann offers `area` to `to`, changes travelling as `transport`;
    /// `to` accepts.
    fn offer(&mut self, area: &str, to: &Store, transport: Transport) -> Made<String> {
        let shared = self
            .ann
            .share_area_with(area, &id_of(to)?, AreaRole::Member, transport)?;
        let file = self.path();
        write_area_offer_now(&self.ann, &file, &shared, &id_of(to)?)?;
        let held = import_bundle(to, &file)?.offer.ok_or("an offer")?;
        to.accept_area_offer(&held.id, false)?;
        Ok(shared)
    }

    /// Ann shares a new Area `name` with `to`.
    fn shared(&mut self, name: &str, to: &Store, transport: Transport) -> Made<String> {
        let area = Places::new(&self.ann).add_area(name)?.id;
        self.offer(&area, to, transport)
    }

    /// Ann sends `to` a bundle, as the share sheet does once the user
    /// picked an app.
    fn send(&mut self, to: &Store) -> TestResult {
        let seq = self.ann.current_seq()?;
        let file = self.path();
        write_person_bundle_now(&self.ann, &file, &id_of(to)?)?.ok_or("a bundle")?;
        self.ann.note_changes_sent(&id_of(to)?, seq)?;
        Ok(())
    }

    fn nudge(&self) -> Result<Option<PlannedChangesNudge>> {
        Alarms::new(&self.ann).changes_nudge(&utc)
    }

    /// When Ann's Changes Nudge fires after now, `count` times.
    fn firings(&self, count: usize) -> Made<Vec<DateTime<Utc>>> {
        let Some(nudge) = self.nudge()? else {
            return Ok(Vec::new());
        };
        let mut after = *self.now.lock();
        let mut out = Vec::new();
        for _ in 0..count {
            after = nudge.fires_after(after, &utc);
            out.push(after);
        }
        Ok(out)
    }
}

/// `from` sends `to` a bundle of what is new for them at `file`, and
/// `to` takes it in.
fn carry(file: &Path, from: &Store, to: &Store) -> TestResult {
    write_person_bundle_now(from, file, &id_of(to)?)?.ok_or("a bundle")?;
    import_bundle(to, file)?;
    Ok(())
}

fn id_of(store: &Store) -> Made<String> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

fn capture(store: &Store, text: &str, area: &str) -> TestResult {
    Notes::new(store).capture(text, Some(area), None)?;
    Ok(())
}

fn waiting(pairs: &[(String, DateTime<Utc>)]) -> BTreeMap<String, DateTime<Utc>> {
    pairs.iter().cloned().collect()
}

#[test]
fn sharing_and_taking_an_offer_leave_nothing_waiting() -> TestResult {
    let (mut a, ben, _) = world()?;
    a.shared("Household", &ben, Transport::Bundle)?;
    assert!(a.ann.waiting_for_changes()?.is_empty());
    assert!(ben.waiting_for_changes()?.is_empty());
    assert!(a.nudge()?.is_none());
    Ok(())
}

#[test]
fn it_comes_30_minutes_after_the_first_unsent_change_every_hour_and_waits_out_quiet_hours()
-> TestResult {
    let (mut a, ben, _) = world()?;
    let area = a.shared("Household", &ben, Transport::Bundle)?;
    capture(&a.ann, "Buy milk", &area)?;
    a.set(at(9, 10));
    capture(&a.ann, "Buy bread", &area)?;
    assert_eq!(
        a.ann.waiting_for_changes()?,
        waiting(&[(id_of(&ben)?, at(9, 0))])
    );

    let nudge = a.nudge()?.ok_or("a Changes Nudge")?;
    assert_eq!(nudge.people, vec![id_of(&ben)?]);
    assert_eq!(nudge.since, None);
    assert_eq!(nudge.at, at(9, 30));
    assert_eq!(a.firings(3)?, vec![at(9, 30), at(10, 30), at(11, 30)]);

    a.set(at(9, 30));
    let nudge = a.nudge()?.ok_or("a Changes Nudge")?;
    assert_eq!(nudge.since, Some(at(9, 30)));
    assert_eq!(nudge.at, at(10, 30));

    // Ignored all day: 22:30 waits for 08:00, and the hours count on.
    a.set(at(21, 0));
    assert_eq!(
        a.firings(3)?,
        vec![at(21, 30), at_on(3, 8, 0), at_on(3, 9, 0)]
    );
    a.set(at(23, 0));
    assert_eq!(a.nudge()?.ok_or("a Changes Nudge")?.at, at_on(3, 8, 0));
    Ok(())
}

#[test]
fn sending_the_bundle_ends_it_and_it_keeps_everything_until_the_person_sends_back() -> TestResult {
    let (mut a, ben, _) = world()?;
    let area = a.shared("Household", &ben, Transport::Bundle)?;
    capture(&a.ann, "Buy milk", &area)?;
    a.set(at(10, 0));
    a.send(&ben)?;
    assert!(a.ann.waiting_for_changes()?.is_empty());
    assert!(a.nudge()?.is_none());
    // The bundle may never arrive: the next one carries it again.
    assert!(!a.ann.entries_for_person(&id_of(&ben)?)?.is_empty());

    a.set(at(10, 5));
    capture(&a.ann, "Buy bread", &area)?;
    assert_eq!(a.nudge()?.ok_or("a Changes Nudge")?.at, at(10, 35));
    Ok(())
}

#[test]
fn a_share_sheet_left_without_an_app_leaves_it_waiting() -> TestResult {
    let (mut a, ben, _) = world()?;
    let area = a.shared("Household", &ben, Transport::Bundle)?;
    capture(&a.ann, "Buy milk", &area)?;
    let file = a.path();
    assert!(write_person_bundle_now(&a.ann, &file, &id_of(&ben)?)?.is_some());
    assert_eq!(
        a.ann.waiting_for_changes()?,
        waiting(&[(id_of(&ben)?, at(9, 0))])
    );
    Ok(())
}

#[test]
fn one_changes_nudge_covers_everyone_waiting_and_sending_to_one_leaves_the_other() -> TestResult {
    let (mut a, ben, cat) = world()?;
    let house = a.shared("Household", &ben, Transport::Bundle)?;
    let garden = a.shared("Garden", &cat, Transport::Bundle)?;
    capture(&a.ann, "Water the roses", &garden)?;
    assert_eq!(a.nudge()?.ok_or("a Changes Nudge")?.at, at(9, 30));

    // Ben's first change came after it fired: it comes again at his
    // own time, sooner than the repeat, for both.
    a.set(at(9, 40));
    capture(&a.ann, "Buy milk", &house)?;
    let nudge = a.nudge()?.ok_or("a Changes Nudge")?;
    assert_eq!(nudge.people, vec![id_of(&cat)?, id_of(&ben)?]);
    assert_eq!(nudge.since, Some(at(9, 30)));
    assert_eq!(nudge.at, at(10, 10));
    assert_eq!(a.firings(2)?, vec![at(10, 10), at(11, 10)]);

    a.send(&cat)?;
    let nudge = a.nudge()?.ok_or("a Changes Nudge")?;
    assert_eq!(nudge.people, vec![id_of(&ben)?]);
    assert_eq!(nudge.at, at(10, 10));
    Ok(())
}

#[test]
fn a_change_the_person_sent_back_never_waits_for_them() -> TestResult {
    let (mut a, ben, _) = world()?;
    let area = a.shared("Household", &ben, Transport::Bundle)?;
    capture(&ben, "From Ben", &area)?;
    let file = a.path();
    carry(&file, &ben, &a.ann)?;
    assert!(a.ann.waiting_for_changes()?.is_empty());
    Ok(())
}

#[test]
fn changes_by_a_shared_folder_or_for_a_blocked_person_wait_for_no_bundle() -> TestResult {
    let (mut a, ben, cat) = world()?;
    let house = a.shared("Household", &ben, Transport::Folder)?;
    capture(&a.ann, "Buy milk", &house)?;
    assert!(a.ann.waiting_for_changes()?.is_empty());

    let garden = a.shared("Garden", &cat, Transport::Bundle)?;
    capture(&a.ann, "Water the roses", &garden)?;
    let people: Vec<String> = a.ann.waiting_for_changes()?.into_keys().collect();
    assert_eq!(people, vec![id_of(&cat)?]);
    a.ann.block_person(&id_of(&cat)?, false)?;
    assert!(a.ann.waiting_for_changes()?.is_empty());
    assert!(a.nudge()?.is_none());
    Ok(())
}

#[test]
fn another_devices_change_counts_from_when_it_came_here_never_from_its_own_clock() -> TestResult {
    let (mut a, ben, cat) = world()?;
    let area = a.shared("Household", &ben, Transport::Bundle)?;
    a.offer(&area, &cat, Transport::Bundle)?;

    // Ben's clock runs a year ahead; his change reaches Ann at 11:00.
    a.set(
        Utc.with_ymd_and_hms(2027, 3, 2, 9, 0, 0)
            .single()
            .unwrap_or_default(),
    );
    capture(&ben, "From Ben", &area)?;
    let file = a.path();
    write_person_bundle_now(&ben, &file, &id_of(&a.ann)?)?.ok_or("a bundle")?;
    a.set(at(11, 0));
    import_bundle(&a.ann, &file)?;
    // Ben lacks Cat's grant since 09:00; Cat lacks Ben's Note since
    // 11:00, not since a year ahead.
    assert_eq!(
        a.ann.waiting_for_changes()?,
        waiting(&[(id_of(&ben)?, at(9, 0)), (id_of(&cat)?, at(11, 0))])
    );
    Ok(())
}

#[test]
fn taking_an_offer_counts_what_came_with_it_as_sent_to_the_sharer() -> TestResult {
    let (mut a, ben, cat) = world()?;
    let area = a.shared("Household", &cat, Transport::Bundle)?;
    capture(&cat, "From Cat", &area)?;
    let file = a.path();
    carry(&file, &cat, &a.ann)?;

    // Ann offers the Area, Cat's Note in it, to Ben.
    a.offer(&area, &ben, Transport::Bundle)?;
    let texts: Vec<String> = Notes::new(&ben)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert!(texts.contains(&"From Cat".to_string()));
    assert!(ben.waiting_for_changes()?.is_empty());
    Ok(())
}

#[test]
fn the_sent_marks_are_kept_by_area_each_only_growing() -> TestResult {
    let (a, ben, _) = world()?;
    let id = id_of(&ben)?;
    a.ann.note_area_sent(&id, "area:b", 7)?;
    a.ann.note_area_sent(&id, "area:a", 3)?;
    a.ann.note_area_sent(&id, "area:b", 5)?;
    assert_eq!(
        a.ann.local_setting(&format!("changesSent:{id}"))?,
        Some(r#"{"area:a":3,"area:b":7}"#.to_string())
    );
    let marks: Vec<(String, i64)> = a.ann.changes_sent_to(&id)?.into_iter().collect();
    assert_eq!(
        marks,
        vec![("area:a".to_string(), 3), ("area:b".to_string(), 7)]
    );
    Ok(())
}

#[test]
fn the_plan_sorts_the_people_by_their_first_change_then_by_id() -> TestResult {
    let people = waiting(&[
        ("b".to_string(), at(9, 0)),
        ("a".to_string(), at(9, 0)),
        ("c".to_string(), at(8, 0)),
    ]);
    let plan =
        changes_nudge_plan(&people, at(7, 0), &utc, Duration::hours(1), None)?.ok_or("a plan")?;
    assert_eq!(plan.people, vec!["c", "a", "b"]);
    assert_eq!(plan.at, at(8, 30));
    let nobody = BTreeMap::new();
    assert!(changes_nudge_plan(&nobody, at(7, 0), &utc, Duration::hours(1), None)?.is_none());
    Ok(())
}
