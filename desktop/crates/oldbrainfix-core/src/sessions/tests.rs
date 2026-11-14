use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
    sync_request_as,
};
use crate::schedule::{ClockTime, Frequency, Recurrence};
use crate::shared::{AreaRole, Invite};
use crate::signing::{ImportReport, SigningKey};
use crate::store::tests::ticking;
use crate::{OwnProfile, PairedDevice, Places};
use chrono::{Duration, NaiveDateTime, TimeZone};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One clock for every Device of a test, a second further at each
/// reading, so what was written later wins everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn device(seed: u8, name: &str) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + Duration::seconds(n)
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// The wall clock `days` days after the store's now: the tests' clock
/// moves by seconds, the Agenda by days. Day 0 is a Thursday.
fn on_day(days: i64) -> impl Fn(DateTime<Utc>) -> NaiveDateTime {
    move |utc| (utc + Duration::days(days)).naive_utc()
}

/// One LAN session, `joiner` asking `host`.
fn exchange(joiner: &Store, host: &Store, shared: bool) -> TestResult {
    let request = sync_request(
        joiner,
        &host.version_vector()?,
        "Phone",
        &host.device_id()?,
        "",
    )?;
    let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
    if shared {
        apply_answer_as(joiner, &answer, true, None)?;
    } else {
        apply_answer(joiner, &answer)?;
    }
    Ok(())
}

fn remember(store: &Store, peer: &Store, shared: bool) -> TestResult {
    store.remember_device(&PairedDevice {
        id: peer.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: store.now(),
        shared,
    })?;
    Ok(())
}

/// Ann's Household shared with Ben as a Member.
fn household() -> Made<(Store, Store, String)> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
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
    remember(&ann, &ben, true)?;
    remember(&ben, &ann, true)?;
    Ok((ann, ben, area))
}

fn daily() -> Made<Schedule> {
    let rule = Some(Recurrence::every(Frequency::Daily, 1));
    Ok(Schedule::new(Day::new(2026, 1, 1), &[], rule).ok_or("a schedule")?)
}

/// Tallies with one field, Times with a target of 20, as their Notes.
fn tallies(store: &Store, names: &[&str], place: Option<&str>) -> Made<Vec<String>> {
    let mut out = Vec::new();
    for name in names {
        let note = Notes::new(store).capture(name, place, None)?;
        Tallies::new(store).add_field(&note.id, "Times", "times", Some(20.0), None)?;
        out.push(note.id);
    }
    Ok(out)
}

fn session(
    store: &Store,
    pool: &[String],
    count: i64,
    rule: SessionRule,
    place: Option<&str>,
) -> Made<Session> {
    let note = Notes::new(store).capture("Sport", place, None)?;
    Ok(Sessions::new(store).spawn(&note.id, &daily()?, pool, count, rule)?)
}

/// Today's Occurrence of a daily Session `days` days on.
fn day(store: &Store, days: i64) -> Occurrence {
    Occurrence::new(Day::today(store.now(), &on_day(days)), None)
}

/// The picks of `s` `days` days on, written as the Agenda does.
fn picked(store: &Store, s: &Session, days: i64) -> Made<Vec<String>> {
    let mut out = Vec::new();
    for today in Sessions::new(store).pick_today(&on_day(days))? {
        if today.session.id() == s.id() {
            out.extend(today.picks.into_iter().map(|p| p.note_id));
        }
    }
    Ok(out)
}

/// The picks of `s` `days` days on, written nowhere.
fn shown(store: &Store, s: &Session, days: i64) -> Made<Vec<String>> {
    let today = Sessions::new(store).on(s.id(), &day(store, days))?;
    Ok(today.picks.into_iter().map(|p| p.note_id).collect())
}

/// Records the targets for `note`, picked for `s` `days` days on.
fn record(store: &Store, s: &Session, days: i64, note: &str) -> Made<TallyEntry> {
    let targets = Tallies::new(store)
        .by_note(note)?
        .map(|t| t.targets())
        .unwrap_or_default();
    Ok(Sessions::new(store).record(s.id(), &day(store, days), note, &targets, "")?)
}

fn done_at(store: &Store, s: &Session, o: &Occurrence) -> Made<Option<DateTime<Utc>>> {
    Ok(Routines::new(store)
        .by_id(s.id())?
        .and_then(|r| r.done_at(o)))
}

#[test]
fn a_session_is_a_routine_from_a_note_with_a_pool_a_count_and_a_rule() -> TestResult {
    let ann = device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["Push-ups", "Sit-ups", "Wall sit"], None)?;
    let milk = notes.capture("Milk", None, None)?.id;
    let pool = [t[0].clone(), t[1].clone(), t[0].clone(), t[2].clone()];
    let s = session(&ann, &pool, 2, SessionRule::Least, None)?;
    assert_eq!(s.setup.pool, t);
    assert_eq!((s.setup.count, s.setup.rule), (2, SessionRule::Least));
    assert!(Routines::new(&ann).by_id(s.id())?.is_some());
    let unsorted: Vec<String> = notes.unsorted()?.into_iter().map(|n| n.id).collect();
    assert_eq!(unsorted, std::slice::from_ref(&milk));
    let newest_first: Vec<String> = t.iter().rev().cloned().collect();
    assert_eq!(sessions.choices(&s.routine.note_id)?, newest_first);
    assert_eq!(sessions.pool(&s.setup)?, t);
    // A pool of Tallies, at least one; a count of 1 to 99.
    let note = notes.capture("Learn", None, None)?.id;
    let every_day = daily()?;
    let spawn = |pool: &[String], count| {
        sessions.spawn(&note, &every_day, pool, count, SessionRule::Random)
    };
    assert!(spawn(&[milk], 1).is_err());
    assert!(spawn(&[], 1).is_err());
    assert!(spawn(&t[..1], 0).is_err());
    assert!(spawn(&t[..1], 100).is_err());
    // Changed: what is the same writes nothing.
    sessions.set_count(s.note_id(), 3)?;
    sessions.set_count_on(s.note_id(), 6, Some(1))?;
    sessions.set_rule(s.note_id(), SessionRule::Random)?;
    let two = [t[2].clone(), t[0].clone()];
    sessions.set_pool(s.note_id(), &two)?;
    let changed = sessions.by_id(s.id())?.ok_or("a Session")?.setup;
    assert_eq!((changed.count, changed.rule), (3, SessionRule::Random));
    assert_eq!(changed.counts, BTreeMap::from([(6, 1)]));
    assert_eq!(changed.pool, two);
    let before = ann.all_entries()?.len();
    sessions.set_count(s.note_id(), 3)?;
    sessions.set_pool(s.note_id(), &two)?;
    assert_eq!(ann.all_entries()?.len(), before);
    sessions.set_count_on(s.note_id(), 6, None)?;
    assert!(
        sessions
            .by_id(s.id())?
            .ok_or("a Session")?
            .setup
            .counts
            .is_empty()
    );
    // A plain Routine is none.
    let walk = Routines::new(&ann).create("Walk", &daily()?)?;
    assert!(sessions.by_id(&walk.id)?.is_none());
    let all: Vec<String> = sessions.all()?.iter().map(|s| s.id().to_string()).collect();
    assert_eq!(all, [s.id().to_string()]);
    Ok(())
}

#[test]
fn what_no_app_writes_is_passed_over() -> TestResult {
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B", "C"], None)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    let key = today.key();
    let c = t[2].as_str();
    // As a modified app would.
    let setup = [
        (keys::COUNT.to_string(), "+3".to_string()),
        (keys::count_on(8), "5".to_string()),
        (format!("{}:0", keys::COUNT), "5".to_string()),
        (keys::count_on(7), "07".to_string()),
        (keys::RULE.to_string(), "sometimes".to_string()),
    ];
    for (field, value) in &setup {
        ann.append(s.note_id(), field, Some(value), None)?;
    }
    let crafted = [
        (keys::PICK_PREFIX.to_string(), t[0].clone()),
        (format!("{}2026-3-1", keys::PICK_PREFIX), t[0].clone()),
        (keys::pick(&today), format!("x,{c},{c},note:,note:a@b,")),
        (format!("{}{key}", keys::ENTRY_PREFIX), c.to_string()),
        (format!("{}{key}@", keys::ENTRY_PREFIX), c.to_string()),
        (
            format!("{}@tallyEntry:x", keys::ENTRY_PREFIX),
            c.to_string(),
        ),
        (keys::entry(&today, "note:x"), c.to_string()),
    ];
    for (field, value) in &crafted {
        ann.append(s.id(), field, Some(value), None)?;
    }
    let read = Sessions::new(&ann).by_id(s.id())?.ok_or("a Session")?.setup;
    assert_eq!((read.count, read.rule), (1, SessionRule::Rolling));
    assert!(read.counts.is_empty());
    let marks = SessionMarks::of(&ann, s.id())?;
    assert_eq!(marks.picks, BTreeMap::from([(key, vec![c.to_string()])]));
    assert!(marks.entries.is_empty());
    assert_eq!(shown(&ann, &s, 0)?, [c.to_string()]);
    let routine = Routines::new(&ann).by_id(s.id())?.ok_or("a Routine")?;
    assert!(routine.done.is_empty());
    Ok(())
}

#[test]
fn rolling_goes_through_the_pool_continuing_where_it_stopped() -> TestResult {
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B", "C", "D", "E"], None)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let of = |i: &[usize]| -> Vec<String> { i.iter().map(|i| t[*i].clone()).collect() };
    assert_eq!(picked(&ann, &s, 0)?, of(&[0, 1]));
    assert_eq!(picked(&ann, &s, 1)?, of(&[2, 3]));
    assert_eq!(picked(&ann, &s, 2)?, of(&[4, 0]));
    // A day nobody looked picks nothing.
    assert_eq!(picked(&ann, &s, 4)?, of(&[1, 2]));
    assert_eq!(picked(&ann, &s, 5)?, of(&[3, 4]));
    assert_eq!(picked(&ann, &s, 6)?, of(&[0, 1]));
    // Called again the same day, it writes nothing.
    let before = ann.all_entries()?.len();
    assert_eq!(picked(&ann, &s, 6)?, of(&[0, 1]));
    assert_eq!(ann.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn least_selected_picks_those_with_the_fewest_entries_first() -> TestResult {
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B"], None)?;
    let s = session(&ann, &t, 1, SessionRule::Least, None)?;
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone()]);
    // A skipped, B recorded: A comes back.
    assert_eq!(picked(&ann, &s, 1)?, [t[1].clone()]);
    record(&ann, &s, 1, &t[1])?;
    assert_eq!(picked(&ann, &s, 2)?, [t[0].clone()]);
    // Entries from anywhere count.
    let tallies = Tallies::new(&ann);
    tallies.add(&t[0], &BTreeMap::new(), "Morning", None)?;
    tallies.add(&t[0], &BTreeMap::new(), "Evening", None)?;
    assert_eq!(picked(&ann, &s, 3)?, [t[1].clone()]);
    Ok(())
}

#[test]
fn random_draws_by_a_hash_the_same_on_every_device() -> TestResult {
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B", "C", "D", "E"], None)?;
    let s = session(&ann, &t, 3, SessionRule::Random, None)?;
    for days in 0..4 {
        let key = day(&ann, days).key();
        let hash = |id: &String| {
            hex::encode(Sha256::digest(
                format!("{}\n{key}\n{id}", s.id()).as_bytes(),
            ))
        };
        let mut drawn = t.clone();
        drawn.sort_by_key(hash);
        drawn.truncate(3);
        assert_eq!(shown(&ann, &s, days)?, drawn);
        assert_eq!(picked(&ann, &s, days)?, drawn);
    }
    Ok(())
}

#[test]
fn a_weekday_has_a_count_of_its_own() -> TestResult {
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B", "C", "D"], None)?;
    let s = session(&ann, &t, 1, SessionRule::Rolling, None)?;
    Sessions::new(&ann).set_count_on(s.note_id(), 6, Some(3))?;
    assert_eq!(day(&ann, 2).day.weekday(), 6);
    assert_eq!(shown(&ann, &s, 1)?.len(), 1);
    assert_eq!(shown(&ann, &s, 2)?.len(), 3);
    // More than the pool holds picks the pool.
    Sessions::new(&ann).set_count_on(s.note_id(), 6, Some(9))?;
    assert_eq!(shown(&ann, &s, 2)?.len(), 4);
    Ok(())
}

#[test]
fn another_one_swaps_in_place_and_the_one_swapped_keeps_its_place() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A", "B", "C", "D"], None)?;
    let of = |i: &[usize]| -> Vec<String> { i.iter().map(|i| t[*i].clone()).collect() };
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, of(&[0, 1]));
    sessions.another(s.id(), &today, &t[0])?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[2, 1]));
    sessions.another(s.id(), &today, &t[2])?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[3, 1]));
    // Every one offered: round again.
    sessions.another(s.id(), &today, &t[3])?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[0, 1]));
    assert!(sessions.another(s.id(), &today, &t[2]).is_err());
    // Swapped out, C and D come first the next day.
    assert_eq!(picked(&ann, &s, 1)?, of(&[2, 3]));
    // Nothing to swap in, nothing written.
    let xy = tallies(&ann, &["X", "Y"], None)?;
    let two = session(&ann, &xy, 2, SessionRule::Rolling, None)?;
    let pick = sessions.pick_today(&on_day(0))?;
    let held = pick
        .iter()
        .find(|o| o.session.id() == two.id())
        .ok_or("a pick")?;
    assert!(!held.can_swap);
    let before = ann.all_entries()?.len();
    sessions.another(two.id(), &today, &xy[0])?;
    assert_eq!(ann.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn a_tally_gone_from_the_pool_is_replaced_unless_it_has_an_entry() -> TestResult {
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B", "C"], None)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    record(&ann, &s, 0, &t[1])?;
    Notes::new(&ann).trash(&t[1])?;
    Notes::new(&ann).trash(&t[0])?;
    assert_eq!(picked(&ann, &s, 0)?, [t[2].clone(), t[1].clone()]);
    let read = Sessions::new(&ann).by_id(s.id())?.ok_or("a Session")?;
    assert_eq!(Sessions::new(&ann).pool(&read.setup)?, [t[2].clone()]);
    Ok(())
}

#[test]
fn a_tally_with_an_entry_for_it_stays_in_its_pick_in_its_place() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A", "B", "C", "D", "E"], None)?;
    let tallies = Tallies::new(&ann);
    let of = |i: &[usize]| -> Vec<String> { i.iter().map(|i| t[*i].clone()).collect() };
    let s = session(&ann, &t, 3, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, of(&[0, 1, 2]));
    sessions.another(s.id(), &today, &t[2])?;
    let entry = record(&ann, &s, 0, &t[3])?;
    // As a pick another Device wrote later.
    let stale = of(&[0, 1, 2]).join(",");
    ann.append(s.id(), &keys::pick(&today), Some(&stale), None)?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[0, 1, 3]));
    assert_eq!(done_at(&ann, &s, &today)?, None);
    // A pick with an entry is swapped for none.
    let before = ann.all_entries()?.len();
    sessions.another(s.id(), &today, &t[3])?;
    assert_eq!(ann.all_entries()?.len(), before);
    // Removed, the entry takes it out again; an entry made on the
    // Tally itself is no pick's.
    tallies.trash_entry(&entry.id)?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[0, 1, 2]));
    tallies.add(&t[3], &BTreeMap::new(), "Elsewhere", None)?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[0, 1, 2]));
    tallies.restore_entry(&entry.id)?;
    assert_eq!(shown(&ann, &s, 0)?, of(&[0, 1, 3]));
    // Its place taken by one with an entry, it goes at the end, and
    // the pick is trimmed by one without.
    let desk = of(&[0, 1, 4]).join(",");
    ann.append(s.id(), &keys::pick(&today), Some(&desk), None)?;
    tallies.add_with(&t[4], &BTreeMap::new(), &[], "Desk", None, |id| {
        ann.append(s.id(), &keys::entry(&today, id), Some(&t[4]), None)
            .map(drop)
    })?;
    let marks = SessionMarks::of(&ann, s.id())?;
    assert_eq!(marks.picks.get(&today.key()), Some(&of(&[0, 1, 4, 3])));
    assert_eq!(shown(&ann, &s, 0)?, of(&[0, 4, 3]));
    Ok(())
}

#[test]
fn done_once_every_pick_has_an_entry_or_when_checked_off() -> TestResult {
    let ann = device(1, "Ann")?;
    let routines = Routines::new(&ann);
    let tallies = Tallies::new(&ann);
    let t = tallies_of(&ann)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    let first = Sessions::new(&ann).record(s.id(), &today, &t[0], &BTreeMap::new(), "Morning")?;
    assert!(first.values.is_empty());
    assert_eq!(done_at(&ann, &s, &today)?, None);
    // An entry made on the Tally itself is no pick's.
    tallies.add(&t[1], &BTreeMap::new(), "Elsewhere", None)?;
    assert_eq!(done_at(&ann, &s, &today)?, None);
    let last = record(&ann, &s, 0, &t[1])?;
    assert_eq!(done_at(&ann, &s, &today)?, Some(last.at));
    let row = routines.agenda(&on_day(0))?.today_rows;
    assert_eq!(row.len(), 1);
    assert_eq!(row[0].done(), Some(last.at));
    let occurrence = Sessions::new(&ann).on(s.id(), &today)?;
    assert!(occurrence.complete());
    let recorded: Vec<Vec<String>> = occurrence
        .picks
        .iter()
        .map(|p| p.entries.iter().map(|e| e.id.clone()).collect())
        .collect();
    assert_eq!(recorded, [vec![first.id.clone()], vec![last.id.clone()]]);
    // Removed, the entry takes Done with it.
    tallies.trash_entry(&last.id)?;
    assert_eq!(done_at(&ann, &s, &today)?, None);
    // A check-off makes it Done, and keeps its moment.
    let at = Utc
        .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
        .single()
        .ok_or("a time")?;
    routines.mark_done(s.id(), &today, Some(at))?;
    tallies.restore_entry(&last.id)?;
    assert_eq!(done_at(&ann, &s, &today)?, Some(at));
    // Only a pick takes an entry.
    assert!(record(&ann, &s, 0, &t[2]).is_err());
    Ok(())
}

fn tallies_of(store: &Store) -> Made<Vec<String>> {
    tallies(store, &["A", "B", "C"], None)
}

#[test]
fn picks_swaps_and_entries_travel_between_own_devices() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let t = tallies_of(&ann)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    exchange(&desk, &ann, false)?;
    assert_eq!(shown(&desk, &s, 0)?, [t[0].clone(), t[1].clone()]);
    let before = desk.all_entries()?.len();
    assert_eq!(picked(&desk, &s, 0)?, [t[0].clone(), t[1].clone()]);
    assert_eq!(desk.all_entries()?.len(), before);
    Sessions::new(&desk).another(s.id(), &today, &t[0])?;
    record(&desk, &s, 0, &t[2])?;
    exchange(&desk, &ann, false)?;
    assert_eq!(shown(&ann, &s, 0)?, [t[2].clone(), t[1].clone()]);
    record(&ann, &s, 0, &t[1])?;
    exchange(&desk, &ann, false)?;
    let done = done_at(&ann, &s, &today)?;
    assert!(done.is_some());
    assert_eq!(done_at(&desk, &s, &today)?, done);
    assert_eq!(shown(&desk, &s, 1)?, shown(&ann, &s, 1)?);
    Ok(())
}

#[test]
fn a_pick_recorded_stays_when_a_first_pick_written_later_wins() -> TestResult {
    // Push-ups and Half bridge picked, Half bridge recorded, Push-ups
    // swapped for a new Tally and that recorded; meanwhile the desk,
    // not synced yet, showed the day and wrote its own first pick,
    // which wins on the field after the sync.
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let t = tallies(&ann, &["Push-ups", "Half bridge"], None)?;
    let (push_ups, bridge) = (t[0].clone(), t[1].clone());
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    exchange(&desk, &ann, false)?;
    assert_eq!(picked(&ann, &s, 0)?, [push_ups.clone(), bridge.clone()]);
    record(&ann, &s, 0, &bridge)?;
    let march = tallies(&ann, &["Long march"], None)?.remove(0);
    let pool = [push_ups.clone(), bridge.clone(), march.clone()];
    Sessions::new(&ann).set_pool(s.note_id(), &pool)?;
    Sessions::new(&ann).another(s.id(), &today, &push_ups)?;
    assert_eq!(shown(&ann, &s, 0)?, [march.clone(), bridge.clone()]);
    record(&ann, &s, 0, &march)?;
    assert_eq!(picked(&desk, &s, 0)?, [push_ups.clone(), bridge.clone()]);
    exchange(&desk, &ann, false)?;
    let stored = SessionMarks::of(&ann, s.id())?.stored.clone();
    assert_eq!(
        stored.get(&today.key()),
        Some(&vec![push_ups.clone(), bridge.clone()])
    );
    for store in [&ann, &desk] {
        let occurrence = Sessions::new(store).on(s.id(), &today)?;
        let ids: Vec<&str> = occurrence
            .picks
            .iter()
            .map(|p| p.note_id.as_str())
            .collect();
        assert_eq!(ids, [march.as_str(), bridge.as_str()]);
        let done: Vec<bool> = occurrence.picks.iter().map(SessionPick::done).collect();
        assert_eq!(done, [true, true]);
        assert!(occurrence.complete());
        assert!(done_at(store, &s, &today)?.is_some());
    }
    // The Agenda writes the pick back once; Push-ups stays out.
    let before = ann.all_entries()?.len();
    assert_eq!(picked(&ann, &s, 0)?, [march.clone(), bridge.clone()]);
    assert_eq!(ann.all_entries()?.len(), before + 1);
    assert_eq!(picked(&ann, &s, 0)?, [march.clone(), bridge.clone()]);
    assert_eq!(ann.all_entries()?.len(), before + 1);
    let stored = SessionMarks::of(&ann, s.id())?.stored.clone();
    assert_eq!(stored.get(&today.key()), Some(&vec![march, bridge]));
    Ok(())
}

#[test]
fn a_pick_another_persons_device_slipped_in_counts_for_nothing() -> TestResult {
    let (ann, ben, _) = household()?;
    let t = tallies_of(&ann)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    // As a modified app would: Ben slips marks into Ann's own Session.
    let forged = [
        ben.append(s.id(), &keys::pick(&today), Some(&t[2]), None)?,
        ben.append(
            s.id(),
            &keys::entry(&today, "tallyEntry:x"),
            Some(&t[2]),
            None,
        )?,
    ];
    let wire: Vec<_> = forged.iter().map(Entry::wire).collect();
    let mut report = ImportReport::default();
    ann.apply_entries(&wire, &[], Some(&ben.device_id()?), &mut report)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(forged.iter().all(|e| held.contains(&e.id())));
    assert_eq!(shown(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    assert!(SessionMarks::of(&ann, s.id())?.entries.is_empty());
    Ok(())
}

/// A daily schedule at `hour`:00.
fn daily_at(hour: u32) -> Made<Schedule> {
    let rule = Some(Recurrence::every(Frequency::Daily, 1));
    let at = ClockTime::new(hour, 0).ok_or("a time")?;
    Ok(Schedule::new(Day::new(2026, 1, 1), &[at], rule).ok_or("a schedule")?)
}

/// The Occurrence at `hour`:00 `days` days on.
fn at(store: &Store, days: i64, hour: u32) -> Occurrence {
    Occurrence::new(
        Day::today(store.now(), &on_day(days)),
        ClockTime::new(hour, 0),
    )
}

/// The picks of the Routine `id` once the Agenda showed `days` days
/// on, by Occurrence key.
fn picks_of(store: &Store, id: &str, days: i64) -> Made<BTreeMap<String, Vec<String>>> {
    Sessions::new(store).pick_today(&on_day(days))?;
    Ok(SessionMarks::of(store, id)?.picks.clone())
}

/// A Session as an older version spawned it from `note`: the setup on
/// the Routine.
fn old(store: &Store, note: &str, setup: &[(String, &str)]) -> Made<String> {
    let routine = Routines::new(store).spawn_with(note, &daily()?, |id| {
        for (field, value) in setup {
            store.append(id, field, Some(value), None)?;
        }
        Ok(())
    })?;
    Ok(routine.id)
}

#[test]
fn a_plain_note_becomes_one_and_plain_again() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let notes = Notes::new(&ann);
    let t = tallies(&ann, &["A", "B"], None)?;
    let (a, b) = (t[0].clone(), t[1].clone());
    let note = notes.capture("Sport", None, None)?.id;
    sessions.make(&note, &t, 1, SessionRule::Least)?;
    assert_eq!(NoteKind::of(&ann, &note)?, NoteKind::Session);
    assert!(!notes.unsorted()?.iter().any(|n| n.id == note));
    let setup = sessions.setup_of(&note)?.ok_or("a setup")?;
    assert_eq!(setup.pool, t);
    assert_eq!((setup.count, setup.rule), (1, SessionRule::Least));
    assert_eq!(setup.held_by, None);
    assert!(sessions.of_note(&note)?.is_empty());
    // A Session becomes no list.
    assert!(!crate::checklists::Checklists::new(&ann).may_make(&note)?);
    assert!(!crate::shopping_lists::ShoppingLists::new(&ann).may_make(&note)?);
    // Only a plain Note becomes one.
    assert!(
        sessions
            .make(&note, &t[..1], 1, SessionRule::Least)
            .is_err()
    );
    assert!(sessions.make(&a, &t[1..], 1, SessionRule::Least).is_err());
    let list = notes.capture("Milk", None, None)?.id;
    crate::checklists::Checklists::new(&ann).make(&list)?;
    assert!(
        sessions
            .make(&list, &t[..1], 1, SessionRule::Least)
            .is_err()
    );
    // Any Routine of it runs it.
    let routine = Routines::new(&ann).spawn(&note, &daily()?)?;
    assert!(sessions.is_session(&routine.id)?);
    let s = sessions.by_id(&routine.id)?.ok_or("a Session")?;
    assert_eq!(picked(&ann, &s, 0)?, std::slice::from_ref(&a));
    record(&ann, &s, 0, &a)?;
    let today = day(&ann, 0);
    let done = done_at(&ann, &s, &today)?;
    assert!(done.is_some());
    sessions.set_count_on(&note, 6, Some(2))?;
    // Plain again: its Routine runs as a plain one; what was Done stays.
    sessions.unmake(&note)?;
    assert_eq!(NoteKind::of(&ann, &note)?, NoteKind::Plain);
    assert!(!sessions.is_session(&routine.id)?);
    assert!(sessions.all()?.is_empty());
    assert_eq!(done_at(&ann, &s, &today)?, done);
    // Made again, it starts from the setup given.
    sessions.make(&note, &t[1..], 1, SessionRule::Random)?;
    let again = sessions.setup_of(&note)?.ok_or("a setup")?;
    assert_eq!(again.pool, [b]);
    assert!(again.counts.is_empty());
    Ok(())
}

#[test]
fn several_routines_run_it_each_picking_for_its_occurrences() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A", "B", "C"], None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    let first = sessions
        .spawn(&note, &daily_at(7)?, &t, 1, SessionRule::Rolling)?
        .routine
        .id;
    let second = Routines::new(&ann).spawn(&note, &daily_at(18)?)?.id;
    let mut both = vec![first.clone(), second.clone()];
    both.sort();
    let runs: Vec<String> = sessions
        .of_note(&note)?
        .iter()
        .map(|s| s.id().to_string())
        .collect();
    assert_eq!(runs, both);
    let read = sessions.by_id(&second)?.ok_or("a Session")?;
    assert_eq!(read.setup.pool, t);
    let (morning, evening) = (at(&ann, 0, 7), at(&ann, 0, 18));
    // Rolling goes on from one Routine's Occurrence to the next.
    assert_eq!(
        picks_of(&ann, &first, 0)?,
        BTreeMap::from([(morning.key(), vec![t[0].clone()])])
    );
    assert_eq!(
        SessionMarks::of(&ann, &second)?.picks,
        BTreeMap::from([(evening.key(), vec![t[1].clone()])])
    );
    let next = picks_of(&ann, &first, 1)?;
    assert_eq!(next.get(&at(&ann, 1, 7).key()), Some(&vec![t[2].clone()]));
    let later = SessionMarks::of(&ann, &second)?.picks.clone();
    assert_eq!(later.get(&at(&ann, 1, 18).key()), Some(&vec![t[0].clone()]));
    // Done per Occurrence, each on its Routine.
    sessions.record(&first, &morning, &t[0], &BTreeMap::new(), "Done")?;
    let routines = Routines::new(&ann);
    let done = |id: &str, o: &Occurrence| -> Made<bool> {
        Ok(routines.by_id(id)?.and_then(|r| r.done_at(o)).is_some())
    };
    assert!(done(&first, &morning)?);
    assert!(!done(&second, &evening)?);
    // One change of the Note's setup for both.
    sessions.set_count(&note, 2)?;
    for id in [&first, &second] {
        let s = sessions.by_id(id)?.ok_or("a Session")?;
        assert_eq!(s.setup.count, 2);
    }
    Ok(())
}

#[test]
fn a_session_an_older_version_left_on_its_routine_reads_as_one() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A", "B", "C"], None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    let pool = t.join(",");
    let old = old(
        &ann,
        &note,
        &[
            (keys::POOL.to_string(), &pool),
            (keys::COUNT.to_string(), "2"),
            (keys::count_on(6), "1"),
            (keys::RULE.to_string(), "least"),
        ],
    )?;
    assert_eq!(NoteKind::of(&ann, &note)?, NoteKind::Session);
    let setup = sessions.setup_of(&note)?.ok_or("a setup")?;
    assert_eq!(setup.held_by.as_deref(), Some(old.as_str()));
    assert_eq!(setup.pool, t);
    assert_eq!(setup.counts, BTreeMap::from([(6, 1)]));
    assert_eq!(setup.rule, SessionRule::Least);
    let s = sessions.by_id(&old)?.ok_or("a Session")?;
    assert_eq!(picked(&ann, &s, 0)?, t[..2]);
    // A later write goes to the Note, with the rest of the setup; the
    // Routine is not rewritten.
    let routine_before = ann.history(&old)?.len();
    sessions.set_count(&note, 3)?;
    assert_eq!(ann.history(&old)?.len(), routine_before);
    let fields = ann.current_fields(&note)?;
    assert_eq!(fields.get(keys::POOL), Some(pool.as_str()));
    assert_eq!(fields.get(keys::COUNT), Some("3"));
    assert_eq!(fields.get(&keys::count_on(6)), Some("1"));
    assert_eq!(fields.get(keys::RULE), Some("least"));
    let now = sessions.setup_of(&note)?.ok_or("a setup")?;
    assert_eq!(now.held_by, None);
    assert_eq!(now.count, 3);
    assert_eq!(now.counts, BTreeMap::from([(6, 1)]));
    Ok(())
}

#[test]
fn spawned_again_from_an_older_one_the_note_takes_its_setup() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A", "B"], None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    let old = old(
        &ann,
        &note,
        &[
            (keys::POOL.to_string(), &t[0]),
            (keys::COUNT.to_string(), "1"),
            (keys::count_on(7), "2"),
            (keys::RULE.to_string(), "least"),
        ],
    )?;
    let routine_before = ann.history(&old)?.len();
    let added = sessions.spawn(&note, &daily_at(7)?, &t, 2, SessionRule::Random)?;
    assert_eq!(ann.history(&old)?.len(), routine_before);
    let setup = sessions.setup_of(&note)?.ok_or("a setup")?;
    assert_eq!(setup.held_by, None);
    assert_eq!(setup.pool, t);
    assert_eq!(setup.count, 2);
    assert_eq!(setup.counts, BTreeMap::from([(7, 2)]));
    assert_eq!(setup.rule, SessionRule::Random);
    let mut both = vec![old, added.id().to_string()];
    both.sort();
    let runs: Vec<String> = sessions
        .of_note(&note)?
        .iter()
        .map(|s| s.id().to_string())
        .collect();
    assert_eq!(runs, both);
    Ok(())
}

#[test]
fn of_several_older_ones_on_a_note_the_one_written_last_counts() -> TestResult {
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A", "B"], None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    let older = |pool: &str, rule: &str| {
        old(
            &ann,
            &note,
            &[
                (keys::POOL.to_string(), pool),
                (keys::COUNT.to_string(), "1"),
                (keys::RULE.to_string(), rule),
            ],
        )
    };
    let first = older(&t[0], "rolling")?;
    let second = older(&t[1], "random")?;
    let held_by =
        || -> Made<Option<String>> { Ok(sessions.setup_of(&note)?.and_then(|s| s.held_by)) };
    let pool_of =
        |id: &str| -> Made<Vec<String>> { Ok(sessions.by_id(id)?.ok_or("a Session")?.setup.pool) };
    assert_eq!(held_by()?, Some(second.clone()));
    assert_eq!(pool_of(&first)?, [t[1].clone()]);
    // A change to the first, as the older version wrote it, wins then.
    ann.append(&first, keys::COUNT, Some("1"), None)?;
    assert_eq!(held_by()?, Some(first.clone()));
    assert_eq!(pool_of(&second)?, [t[0].clone()]);
    // One in the trash counts for nothing.
    Routines::new(&ann).trash(&first)?;
    assert_eq!(held_by()?, Some(second));
    // Plain again, no older one counts either.
    sessions.unmake(&note)?;
    assert_eq!(sessions.setup_of(&note)?, None);
    assert_eq!(NoteKind::of(&ann, &note)?, NoteKind::Plain);
    Ok(())
}

#[test]
fn it_reads_its_pool_without_the_entries_of_its_tallies() -> TestResult {
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let pool = tallies(&ann, &["A", "B", "C"], None)?;
    let setup = session(&ann, &pool, 2, SessionRule::Rolling, None)?.setup;
    let pool_reads = || -> Made<usize> {
        // After a change, so nothing worked out before is kept.
        Notes::new(&ann).capture("Change", None, None)?;
        let before = ann.history_reads.load(SeqCst);
        assert_eq!(sessions.pool(&setup)?, pool);
        Ok(ann.history_reads.load(SeqCst) - before)
    };
    let none = pool_reads()?;
    for id in &pool {
        for _ in 0..10 {
            Tallies::new(&ann).add(id, &BTreeMap::new(), "ok", None)?;
        }
    }
    assert_eq!(pool_reads()?, none);
    Ok(())
}

#[test]
fn it_finds_the_setups_without_reading_every_routine() -> TestResult {
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1, "Ann")?;
    let sessions = Sessions::new(&ann);
    let t = tallies(&ann, &["A"], None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    let older = old(&ann, &note, &[(keys::POOL.to_string(), &t[0])])?;
    let mut plain = Vec::new();
    for i in 0..10 {
        let id = Notes::new(&ann)
            .capture(&format!("Plain {i}"), None, None)?
            .id;
        Routines::new(&ann).spawn(&id, &daily()?)?;
        plain.push(id);
    }
    // Worked out once: the Routine that holds a setup is read, its
    // fields and its history, and none of the plain ones.
    let before = ann.history_reads.load(SeqCst);
    assert_eq!(sessions.setup_of(&plain[0])?, None);
    assert_eq!(ann.history_reads.load(SeqCst) - before, 2);
    // Then kept until the log changes: no read at all.
    let kept = ann.history_reads.load(SeqCst);
    for id in &plain {
        assert_eq!(sessions.setup_of(id)?, None);
    }
    assert_eq!(
        sessions.setup_of(&note)?.and_then(|s| s.held_by),
        Some(older)
    );
    assert_eq!(ann.history_reads.load(SeqCst), kept);
    sessions.unmake(&note)?;
    assert_eq!(sessions.setup_of(&note)?, None);
    Ok(())
}

#[test]
fn it_reads_its_marks_once_per_change_and_its_entries_in_one_query() -> TestResult {
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1, "Ann")?;
    let t = tallies(&ann, &["A", "B", "C"], None)?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
    // The reads of the Session's marks and of its Routine, twice as
    // an Agenda load reads them (its Done, its picks), after a change.
    let reads_after = || -> Made<usize> {
        Notes::new(&ann).capture("A change", None, None)?;
        let before = ann.history_reads.load(SeqCst);
        // Its history, then its entries in one query.
        SessionMarks::of(&ann, s.id())?;
        assert_eq!(ann.history_reads.load(SeqCst) - before, 2);
        for _ in 0..2 {
            Routines::new(&ann).by_id(s.id())?;
            SessionMarks::of(&ann, s.id())?;
        }
        Ok(ann.history_reads.load(SeqCst) - before)
    };
    let record_days = |days: std::ops::Range<i64>| -> TestResult {
        for k in days {
            for p in picked(&ann, &s, k)? {
                record(&ann, &s, k, &p)?;
            }
        }
        Ok(())
    };
    record_days(0..3)?;
    let short = reads_after()?;
    record_days(3..10)?;
    // Seven more days of entries, not one read more.
    assert_eq!(reads_after()?, short);
    // Then kept until the log changes.
    let kept = ann.history_reads.load(SeqCst);
    SessionMarks::of(&ann, s.id())?;
    assert_eq!(ann.history_reads.load(SeqCst), kept);
    // What it shares is what a read of its own holds.
    let marks = SessionMarks::of(&ann, s.id())?;
    let read = SessionMarks::read(&ann, s.id(), &ann.history(s.id())?)?;
    assert_eq!(*marks, read);
    assert_eq!(marks.done_from_entries(&ann)?.len(), 10);
    Ok(())
}

#[test]
fn it_sees_what_another_store_on_the_same_file_wrote() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    let app = Store::open(&path, ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
    OwnProfile::new(&app).apply_defaults("Ann", None)?;
    // As the phone's background sync does: a second store, open on
    // the same file at the same time.
    let background = Store::open(&path, ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
    let t = tallies(&app, &["A"], None)?;
    let note = Notes::new(&app).capture("Sport", None, None)?.id;
    assert_eq!(NoteKind::of(&app, &note)?, NoteKind::Plain);
    Sessions::new(&background).make(&note, &t, 1, SessionRule::Rolling)?;
    // No write here since: the store still sees it.
    assert_eq!(NoteKind::of(&app, &note)?, NoteKind::Session);
    assert_eq!(
        Sessions::new(&app).setup_of(&note)?.ok_or("a setup")?.pool,
        t
    );
    Ok(())
}

#[test]
fn it_travels_between_own_devices_as_part_of_its_note() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let t = tallies(&ann, &["A"], None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    Sessions::new(&ann).make(&note, &t, 1, SessionRule::Rolling)?;
    Routines::new(&ann).spawn(&note, &daily()?)?;
    Routines::new(&ann).spawn(&note, &daily_at(18)?)?;
    exchange(&desk, &ann, false)?;
    assert_eq!(NoteKind::of(&desk, &note)?, NoteKind::Session);
    assert_eq!(Sessions::new(&desk).of_note(&note)?.len(), 2);
    Ok(())
}

#[test]
fn in_a_shared_area_everyone_gets_the_same_picks_and_one_done() -> TestResult {
    let (ann, ben, area) = household()?;
    let t = tallies(&ann, &["A", "B", "C"], Some(&area))?;
    let s = session(&ann, &t, 2, SessionRule::Least, Some(&area))?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    exchange(&ben, &ann, true)?;
    assert_eq!(ben.area_of(s.id())?, Some(area.clone()));
    let before = ben.all_entries()?.len();
    assert_eq!(picked(&ben, &s, 0)?, [t[0].clone(), t[1].clone()]);
    assert_eq!(ben.all_entries()?.len(), before);
    // One entry each: one Done for both.
    record(&ben, &s, 0, &t[0])?;
    record(&ann, &s, 0, &t[1])?;
    assert_eq!(done_at(&ann, &s, &today)?, None);
    exchange(&ben, &ann, true)?;
    exchange(&ben, &ann, true)?;
    let done = done_at(&ann, &s, &today)?;
    assert!(done.is_some());
    assert_eq!(done_at(&ben, &s, &today)?, done);
    // A Member records and swaps, but changes no Session.
    assert!(Sessions::new(&ben).set_count(s.note_id(), 1).is_err());
    let own = Notes::new(&ben).capture("Gym", Some(&area), None)?.id;
    let spawned = Sessions::new(&ben).spawn(&own, &daily()?, &t[..1], 1, SessionRule::Rolling);
    assert!(spawned.is_err());
    Ok(())
}

#[test]
fn a_member_swaps_for_everyone_but_writes_nothing_else_of_it() -> TestResult {
    let (ann, ben, area) = household()?;
    let t = tallies(&ann, &["A", "B", "C"], Some(&area))?;
    let s = session(&ann, &t, 2, SessionRule::Rolling, Some(&area))?;
    let today = day(&ann, 0);
    assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
    exchange(&ben, &ann, true)?;
    Sessions::new(&ben).another(s.id(), &today, &t[0])?;
    // As a modified app would.
    let forged = [
        ben.append(s.note_id(), keys::COUNT, Some("3"), None)?,
        ben.append(s.note_id(), keys::POOL, Some(&t[0]), None)?,
        ben.append(s.id(), keys::COUNT, Some("3"), None)?,
        ben.append(s.id(), keys::POOL, Some(&t[0]), None)?,
    ];
    exchange(&ben, &ann, true)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(forged.iter().all(|e| held.contains(&e.id())));
    assert_eq!(shown(&ann, &s, 0)?, [t[2].clone(), t[1].clone()]);
    let read = Sessions::new(&ann).by_id(s.id())?.ok_or("a Session")?.setup;
    assert_eq!(read.count, 2);
    assert_eq!(read.pool, t);
    Ok(())
}

#[test]
fn a_member_sets_up_their_own_note_but_spawns_no_routine() -> TestResult {
    let (ann, ben, area) = household()?;
    let t = tallies(&ann, &["A"], Some(&area))?;
    exchange(&ben, &ann, true)?;
    let own = Notes::new(&ben).capture("Gym", Some(&area), None)?.id;
    Sessions::new(&ben).make(&own, &t, 1, SessionRule::Rolling)?;
    assert!(Routines::new(&ben).spawn(&own, &daily()?).is_err());
    exchange(&ben, &ann, true)?;
    assert_eq!(NoteKind::of(&ann, &own)?, NoteKind::Session);
    // Another's Note stays theirs.
    let hers = Notes::new(&ann).capture("Run", Some(&area), None)?.id;
    exchange(&ben, &ann, true)?;
    assert!(Notes::new(&ben).by_id(&hers)?.is_some());
    let made = Sessions::new(&ben).make(&hers, &t, 1, SessionRule::Rolling);
    assert!(made.is_err());
    Ok(())
}

#[test]
fn only_tallies_of_its_area_count_while_it_is_shared() -> TestResult {
    let (ann, _ben, area) = household()?;
    let mine = tallies(&ann, &["Mine"], None)?.remove(0);
    let a = tallies(&ann, &["A"], Some(&area))?.remove(0);
    let note = Notes::new(&ann).capture("Sport", Some(&area), None)?.id;
    let sessions = Sessions::new(&ann);
    assert_eq!(sessions.choices(&note)?, std::slice::from_ref(&a));
    let both = [a.clone(), mine.clone()];
    assert!(
        sessions
            .spawn(&note, &daily()?, &both, 2, SessionRule::Rolling)
            .is_err()
    );
    // A Session of one's own picks from every Area.
    let private = session(
        &ann,
        &[mine.clone(), a.clone()],
        2,
        SessionRule::Rolling,
        None,
    )?;
    assert_eq!(sessions.pool(&private.setup)?, [mine.clone(), a.clone()]);
    // Filed into the Area, it leaves the others out.
    Notes::new(&ann).file(&private.routine.note_id, &area)?;
    assert_eq!(sessions.pool(&private.setup)?, std::slice::from_ref(&a));
    assert_eq!(picked(&ann, &private, 0)?, [a]);
    Ok(())
}
