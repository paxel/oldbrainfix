use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
    sync_request_as,
};
use crate::shared::{AreaRole, Invite};
use crate::signing::{ImportReport, SigningKey};
use crate::store::tests::ticking;
use crate::{Alarms, OwnProfile, PairedDevice, Places, Routines};
use chrono::{Duration, NaiveDateTime};

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
/// moves by seconds, the Agenda by days.
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

/// Frogs spawned from new Notes with `texts`, in that order.
fn spawned(store: &Store, texts: &[&str], place: Option<&str>) -> Made<Vec<Frog>> {
    let mut out = Vec::new();
    for text in texts {
        let note = Notes::new(store).capture(text, place, None)?;
        out.push(Frogs::new(store).spawn(&note.id)?);
    }
    Ok(out)
}

fn ids(frogs: &[Frog]) -> Vec<String> {
    frogs.iter().map(|f| f.id.clone()).collect()
}

fn shown(today: &FrogsToday) -> Vec<String> {
    ids(&today.shown)
}

#[test]
fn spawning_takes_the_note_out_of_unsorted_once_per_note() -> TestResult {
    let ann = device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let frogs = Frogs::new(&ann);
    let note = notes.capture("Wash the windows", None, None)?;
    assert_eq!(notes.unsorted()?.len(), 1);
    let frog = frogs.spawn(&note.id)?;
    assert!(notes.unsorted()?.is_empty());
    assert_eq!(frog.note_id, note.id);
    // The Note has its open Frog: spawning again writes nothing.
    let before = ann.all_entries()?.len();
    assert_eq!(frogs.spawn(&note.id)?.id, frog.id);
    assert_eq!(ann.all_entries()?.len(), before);
    // Never rings: no Routine, no alarm.
    assert!(Routines::new(&ann).all()?.is_empty());
    assert!(Alarms::new(&ann).plan(&on_day(0))?.is_empty());
    frogs.mark_done(&frog.id, None)?;
    assert_ne!(frogs.spawn(&note.id)?.id, frog.id);
    // Only a Note in the lists spawns one.
    let gone = notes.capture("Gone", None, None)?;
    notes.trash(&gone.id)?;
    assert!(frogs.spawn(&gone.id).is_err());
    assert!(frogs.spawn("note:none").is_err());
    Ok(())
}

#[test]
fn done_undone_the_lists_and_the_trash() -> TestResult {
    let ann = device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let frogs = Frogs::new(&ann);
    let made = spawned(&ann, &["Cupboard", "Windows", "DVDs"], None)?;
    let [cupboard, windows, dvds] = [&made[0].id, &made[1].id, &made[2].id];
    frogs.mark_done(windows, None)?;
    let at = parse_time("2025-12-01T12:00:00.000Z")?;
    frogs.mark_done(cupboard, Some(at))?;
    assert_eq!(ids(&frogs.open()?), vec![dvds.clone()]);
    assert_eq!(ids(&frogs.done()?), [windows.clone(), cupboard.clone()]);
    let before = ann.all_entries()?.len();
    frogs.mark_done(cupboard, None)?;
    assert_eq!(ann.all_entries()?.len(), before);
    assert_eq!(frogs.by_id(cupboard)?.and_then(|f| f.done), Some(at));
    frogs.unmark_done(windows)?;
    assert_eq!(ids(&frogs.open()?), [windows.clone(), dvds.clone()]);
    frogs.trash(windows)?;
    assert_eq!(ids(&frogs.open()?), vec![dvds.clone()]);
    frogs.restore(windows)?;
    notes.trash(&made[2].note_id)?;
    assert_eq!(ids(&frogs.open()?), vec![windows.clone()]);
    notes.restore(&made[2].note_id)?;
    assert!(notes.unsorted()?.is_empty());
    let into = notes.capture("Into", None, None)?;
    notes.merge(&made[2].note_id, &into.id)?;
    assert_eq!(ids(&frogs.of_note(&into.id)?), vec![dvds.clone()]);
    Ok(())
}

#[test]
fn two_a_day_those_offered_the_longest_ago_first() -> TestResult {
    let ann = device(1, "Ann")?;
    let frogs = Frogs::new(&ann);
    let all = ids(&spawned(&ann, &["A", "B", "C", "D", "E"], None)?);
    let [a, b, c, d, e] = [&all[0], &all[1], &all[2], &all[3], &all[4]];
    assert_eq!(ids(&frogs.rotation()?), all);
    let before = ann.all_entries()?.len();
    assert_eq!(shown(&frogs.today(&on_day(0))?), [a.clone(), b.clone()]);
    assert_eq!(ann.all_entries()?.len(), before);
    let first = frogs.pick_today(&on_day(0))?;
    assert_eq!(shown(&first), [a.clone(), b.clone()]);
    assert!(first.visible && first.can_swap && !first.eaten);
    let before = ann.all_entries()?.len();
    frogs.pick_today(&on_day(0))?;
    assert_eq!(ann.all_entries()?.len(), before);
    assert_eq!(
        ids(&frogs.rotation()?),
        [c.clone(), d.clone(), e.clone(), a.clone(), b.clone()]
    );
    assert_eq!(
        shown(&frogs.pick_today(&on_day(1))?),
        [c.clone(), d.clone()]
    );
    assert_eq!(
        shown(&frogs.pick_today(&on_day(2))?),
        [e.clone(), a.clone()]
    );
    assert_eq!(
        shown(&frogs.pick_today(&on_day(3))?),
        [b.clone(), c.clone()]
    );
    Ok(())
}

#[test]
fn another_one_swaps_and_eating_one_hides_the_other() -> TestResult {
    let ann = device(1, "Ann")?;
    let frogs = Frogs::new(&ann);
    let all = ids(&spawned(&ann, &["A", "B", "C"], None)?);
    let [a, b, c] = [&all[0], &all[1], &all[2]];
    let day = on_day(0);
    frogs.pick_today(&day)?;
    frogs.another(&day, a)?;
    assert_eq!(shown(&frogs.today(&day)?), [c.clone(), b.clone()]);
    frogs.another(&day, c)?;
    assert_eq!(shown(&frogs.today(&day)?), [a.clone(), b.clone()]);
    assert!(frogs.another(&day, c).is_err());
    frogs.mark_done(b, None)?;
    let eaten = frogs.pick_today(&day)?;
    assert_eq!(shown(&eaten), vec![b.clone()]);
    assert!(eaten.eaten && !eaten.can_swap);
    assert!(frogs.another(&day, a).is_err());
    frogs.unmark_done(b)?;
    assert_eq!(shown(&frogs.pick_today(&day)?), [a.clone(), b.clone()]);
    // Gone from the pick, replaced in its place.
    frogs.trash(a)?;
    assert_eq!(shown(&frogs.pick_today(&day)?), [c.clone(), b.clone()]);
    frogs.trash(c)?;
    let one = frogs.pick_today(&day)?;
    assert_eq!(shown(&one), vec![b.clone()]);
    assert!(!one.can_swap);
    let before = ann.all_entries()?.len();
    frogs.another(&day, b)?;
    assert_eq!(ann.all_entries()?.len(), before);
    // The last one eaten stays for the day; the section goes after.
    frogs.mark_done(b, None)?;
    assert!(frogs.pick_today(&day)?.visible);
    let tomorrow = frogs.pick_today(&on_day(1))?;
    assert!(!tomorrow.visible && tomorrow.shown.is_empty());
    Ok(())
}

#[test]
fn todays_pick_and_another_one_travel_between_own_devices() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let all = ids(&spawned(&ann, &["A", "B", "C"], None)?);
    let day = on_day(0);
    Frogs::new(&ann).pick_today(&day)?;
    exchange(&desk, &ann, false)?;
    assert_eq!(
        shown(&Frogs::new(&desk).today(&day)?),
        [all[0].clone(), all[1].clone()]
    );
    let before = desk.all_entries()?.len();
    Frogs::new(&desk).pick_today(&day)?;
    assert_eq!(desk.all_entries()?.len(), before);
    Frogs::new(&desk).another(&day, &all[1])?;
    exchange(&desk, &ann, false)?;
    assert_eq!(
        shown(&Frogs::new(&ann).today(&day)?),
        [all[0].clone(), all[2].clone()]
    );
    assert_eq!(
        ids(&Frogs::new(&ann).rotation()?),
        ids(&Frogs::new(&desk).rotation()?)
    );
    Ok(())
}

#[test]
fn in_a_shared_area_done_is_shared_and_the_rotation_is_ones_own() -> TestResult {
    let (ann, ben, area) = household()?;
    let all = ids(&spawned(&ann, &["A", "B", "C"], Some(&area))?);
    let day = on_day(0);
    Frogs::new(&ann).pick_today(&day)?;
    Frogs::new(&ann).another(&day, &all[0])?;
    exchange(&ben, &ann, true)?;
    assert_eq!(ids(&Frogs::new(&ben).open()?), all);
    // Ann's picks never reach Ben: his are his own.
    assert!(ben.logged_history(keys::PICK_ENTITY)?.is_empty());
    assert_eq!(
        shown(&Frogs::new(&ben).pick_today(&day)?),
        [all[0].clone(), all[1].clone()]
    );
    // A Member checks off, but spawns nothing.
    Frogs::new(&ben).mark_done(&all[1], None)?;
    let own = Notes::new(&ben).capture("Bread", Some(&area), None)?;
    assert!(Frogs::new(&ben).spawn(&own.id).is_err());
    exchange(&ben, &ann, true)?;
    assert!(
        Frogs::new(&ann)
            .by_id(&all[1])?
            .is_some_and(|f| f.is_done())
    );
    assert_eq!(shown(&Frogs::new(&ann).today(&day)?), vec![all[1].clone()]);
    Ok(())
}

#[test]
fn a_member_writes_nothing_of_the_frog_but_its_check_off() -> TestResult {
    let (ann, ben, area) = household()?;
    let all = ids(&spawned(&ann, &["A"], Some(&area))?);
    exchange(&ben, &ann, true)?;
    // As a modified app would.
    let forged = ben.append(
        &all[0],
        keys::TRASHED,
        Some("2026-03-01T10:00:00.000Z"),
        None,
    )?;
    exchange(&ben, &ann, true)?;
    assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
    assert!(
        Frogs::new(&ann)
            .by_id(&all[0])?
            .is_some_and(|f| !f.is_trashed())
    );
    assert_eq!(ids(&Frogs::new(&ann).open()?), all);
    Ok(())
}

#[test]
fn a_pick_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let (ann, ben, area) = household()?;
    let all = ids(&spawned(&ann, &["A", "B", "C"], Some(&area))?);
    exchange(&ben, &ann, true)?;
    let day = on_day(0);
    Frogs::new(&ann).pick_today(&day)?;
    let today = Day::today(ann.now(), &day);
    let value = format!("{today},{}", all[2]);
    let forged = ben.append(keys::PICK_ENTITY, keys::PICK, Some(&value), None)?;
    let mut report = ImportReport::default();
    ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
    assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
    assert_eq!(
        shown(&Frogs::new(&ann).today(&day)?),
        [all[0].clone(), all[1].clone()]
    );
    assert_eq!(
        ids(&Frogs::new(&ann).rotation()?),
        [all[2].clone(), all[0].clone(), all[1].clone()]
    );
    Ok(())
}

#[test]
fn a_frog_spawned_before_its_note_was_filed_in_goes_along() -> TestResult {
    let (ann, ben, area) = household()?;
    let all = ids(&spawned(&ann, &["A"], None)?);
    // Written in the Area since, so Ben's vector passes the Frog.
    Notes::new(&ann).capture("Milk", Some(&area), None)?;
    exchange(&ben, &ann, true)?;
    assert_eq!(Frogs::new(&ben).by_id(&all[0])?, None);
    let note = Frogs::new(&ann).by_id(&all[0])?.ok_or("a Frog")?.note_id;
    Notes::new(&ann).file(&note, &area)?;
    exchange(&ben, &ann, true)?;
    assert_eq!(ids(&Frogs::new(&ben).open()?), all);
    Ok(())
}
