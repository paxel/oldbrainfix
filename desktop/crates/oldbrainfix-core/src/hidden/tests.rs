use super::*;
use crate::ai_tools::ToolOutput;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
    sync_request_as,
};
use crate::notes::NewFile;
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule};
use crate::search::Search;
use crate::shared::{AreaRole, Invite};
use crate::signing::{ImportReport, SigningKey};
use crate::store::tests::{store, ticking};
use crate::tag_tree::TagTree;
use crate::{
    Checklists, Frogs, OwnProfile, PairedDevice, Routines, SessionRule, Sessions, Tallies,
};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

/// The marks of the refusal `done` ended in, if it was one.
fn refused<T: std::fmt::Debug>(done: Result<T>) -> Made<Vec<String>> {
    match done {
        Err(Error::HiddenFromAi(marks)) => Ok(marks),
        other => Err(format!("not refused as hidden: {other:?}").into()),
    }
}

fn schedule() -> Made<Schedule> {
    let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e}"))?;
    Ok(Schedule::new(Day::new(2026, 1, 1), &eight, Some(rule)).ok_or("a schedule")?)
}

/// A Note made a Tally with one field: its id and the field's.
fn tally(store: &Store, text: &str, place: Option<&str>) -> Made<(String, String)> {
    let note = Notes::new(store).capture(text, place, None)?.id;
    let field = Tallies::new(store).add_field(&note, text, "", None, None)?;
    Ok((note, field))
}

fn png() -> Made<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(8, 8, image::Rgb([200, 40, 40]))
        .write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

/// The tool `name` called with `args` for `task`, on a clock in UTC.
fn call(store: &Store, task: &str, name: &str, args: &Value) -> Result<ToolOutput> {
    store.ai_tool(task, name, args, &|t: DateTime<Utc>| t.naive_utc())
}

#[test]
fn the_mark_is_set_and_taken_off_and_the_same_again_writes_nothing() -> TestResult {
    let ann = store();
    let note = Notes::new(&ann).capture("Diary", None, None)?.id;
    assert!(!ann.is_marked_hidden_from_ai(&note)?);
    ann.set_hidden_from_ai(&note, true)?;
    assert!(ann.is_marked_hidden_from_ai(&note)?);
    let marks: Vec<String> = ann.hidden_from_ai_marks()?.into_iter().collect();
    assert_eq!(marks, std::slice::from_ref(&note));
    let written = ann.all_entries()?.len();
    ann.set_hidden_from_ai(&note, true)?;
    assert_eq!(ann.all_entries()?.len(), written);
    ann.set_hidden_from_ai(&note, false)?;
    assert!(!ann.is_marked_hidden_from_ai(&note)?);
    assert_eq!(ann.current(&keys::of(&note), keys::HIDDEN)?, None);
    ann.set_hidden_from_ai(&note, false)?;
    assert_eq!(ann.all_entries()?.len(), written + 1);
    Ok(())
}

#[test]
fn a_value_no_app_writes_is_no_mark() -> TestResult {
    let ann = store();
    let note = Notes::new(&ann).capture("Diary", None, None)?.id;
    ann.append(&keys::of(&note), keys::HIDDEN, Some("yes"), None)?;
    assert!(!ann.is_marked_hidden_from_ai(&note)?);
    assert!(!ann.is_hidden_from_ai(&note)?);
    Ok(())
}

#[test]
fn only_a_note_or_an_area_is_marked() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let area = places.add_area("Home")?.id;
    let topic = places.add_topic(&area, "Kitchen")?.id;
    assert!(!ann.may_hide_from_ai(&topic)?);
    assert!(ann.set_hidden_from_ai(&topic, true).is_err());
    assert!(ann.set_hidden_from_ai("note:gone", true).is_err());
    ann.set_hidden_from_ai(&area, true)?;
    assert!(ann.is_marked_hidden_from_ai(&area)?);
    Ok(())
}

#[test]
fn what_is_on_a_hidden_note_or_in_a_hidden_area_is_hidden() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let area = places.add_area("Health")?.id;
    let topic = places.add_topic(&area, "Back")?.id;
    let in_topic = notes.capture("Physio", Some(&topic), None)?.id;
    let in_area = notes.capture("Doctor", Some(&area), None)?.id;
    let alone = notes.capture("Diary", None, None)?.id;
    let open = notes.capture("Shopping", None, None)?.id;
    let section = notes.amend(&in_topic, "Exercises")?.id;
    let routine = Routines::new(&ann).spawn(&alone, &schedule()?)?.id;
    let frog = Frogs::new(&ann).spawn(&in_area)?.id;
    let list = notes.capture("Bag\nShoes", None, None)?.id;
    Checklists::new(&ann).make(&list)?;
    let line = Checklists::new(&ann).add(&list, "Towel", None, false)?;
    let merged = notes.capture("Notes on the diary", None, None)?.id;
    notes.merge(&merged, &alone)?;
    for id in [&area, &alone, &list] {
        ann.set_hidden_from_ai(id, true)?;
    }
    assert_eq!(
        ann.hidden_from_ai_by(&in_topic)?,
        std::slice::from_ref(&area)
    );
    assert_eq!(
        ann.hidden_from_ai_by(&section)?,
        std::slice::from_ref(&area)
    );
    assert_eq!(ann.hidden_from_ai_by(&frog)?, std::slice::from_ref(&area));
    assert_eq!(ann.hidden_from_ai_by(&topic)?, std::slice::from_ref(&area));
    assert_eq!(
        ann.hidden_from_ai_by(&routine)?,
        std::slice::from_ref(&alone)
    );
    assert_eq!(
        ann.hidden_from_ai_by(&merged)?,
        std::slice::from_ref(&alone)
    );
    assert_eq!(ann.hidden_from_ai_by(&line)?, std::slice::from_ref(&list));
    let agenda = format!("{}{list}", line_keys::AGENDA_PREFIX);
    assert_eq!(ann.hidden_from_ai_by(&agenda)?, std::slice::from_ref(&list));
    assert!(!ann.is_hidden_from_ai(&open)?);
    notes.file(&in_area, &places.add_area("Work")?.id)?;
    assert!(!ann.is_hidden_from_ai(&in_area)?);
    assert!(!ann.is_hidden_from_ai(&frog)?);
    ann.set_hidden_from_ai(&area, false)?;
    assert!(!ann.is_hidden_from_ai(&in_topic)?);
    Ok(())
}

#[test]
fn those_of_a_list_that_are_hidden_are_the_ones_each_walk_finds() -> TestResult {
    let ann = device(1, "Ann")?;
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let area = places.add_area("Health")?.id;
    let topic = places.add_topic(&area, "Back")?.id;
    let in_topic = notes.capture("Physio", Some(&topic), None)?.id;
    let in_area = notes.capture("Doctor", Some(&area), None)?.id;
    let alone = notes.capture("Diary", None, None)?.id;
    let open = notes.capture("Shopping", None, None)?.id;
    let merged = notes.capture("Notes on the diary", None, None)?.id;
    notes.merge(&merged, &alone)?;
    let all = [&in_topic, &in_area, &alone, &open, &merged, &topic, &area];
    assert!(ann.hidden_from_ai_among(all)?.is_empty());
    ann.set_hidden_from_ai(&area, true)?;
    ann.set_hidden_from_ai(&alone, true)?;
    let each = || -> Made<BTreeSet<String>> {
        let mut out = BTreeSet::new();
        for id in all {
            if ann.is_hidden_from_ai(id)? {
                out.insert(id.clone());
            }
        }
        Ok(out)
    };
    let want: BTreeSet<String> = [&in_topic, &in_area, &alone, &merged, &topic, &area]
        .into_iter()
        .cloned()
        .collect();
    assert_eq!(each()?, want);
    assert_eq!(ann.hidden_from_ai_among(all)?, each()?);
    assert_eq!(
        ann.hidden_from_ai_among([&open, &in_area])?,
        BTreeSet::from([in_area.clone()])
    );
    notes.file(&in_area, &places.add_area("Work")?.id)?;
    ann.set_hidden_from_ai(&alone, false)?;
    let want: BTreeSet<String> = [&in_topic, &topic, &area].into_iter().cloned().collect();
    assert_eq!(ann.hidden_from_ai_among(all)?, want);
    assert_eq!(ann.hidden_from_ai_among(all)?, each()?);
    Ok(())
}

#[test]
fn it_travels_between_own_devices() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let note = Notes::new(&ann).capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&note, true)?;
    exchange(&desk, &ann, false)?;
    assert!(desk.is_hidden_from_ai(&note)?);
    desk.set_hidden_from_ai(&note, false)?;
    exchange(&desk, &ann, false)?;
    assert!(!ann.is_hidden_from_ai(&note)?);
    Ok(())
}

#[test]
fn the_mark_is_not_set_in_a_shared_area_but_always_taken_off() -> TestResult {
    let (ann, _, area) = household()?;
    let note = Notes::new(&ann).capture("Milk", Some(&area), None)?.id;
    assert!(!ann.may_hide_from_ai(&area)?);
    assert!(!ann.may_hide_from_ai(&note)?);
    assert!(ann.set_hidden_from_ai(&area, true).is_err());
    assert!(ann.set_hidden_from_ai(&note, true).is_err());
    assert!(ann.hidden_from_ai_marks()?.is_empty());
    // Marked before it was shared, as two own Devices apart could.
    ann.append(&keys::of(&note), keys::HIDDEN, Some(keys::ON), None)?;
    assert!(ann.is_hidden_from_ai(&note)?);
    ann.set_hidden_from_ai(&note, false)?;
    assert!(!ann.is_hidden_from_ai(&note)?);
    Ok(())
}

#[test]
fn it_never_reaches_another_person() -> TestResult {
    let (ann, ben, area) = household()?;
    let diary = Notes::new(&ann).capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&diary, true)?;
    Notes::new(&ann).capture("Milk", Some(&area), None)?;
    exchange(&ben, &ann, true)?;
    assert!(ben.entities_with_prefix(keys::PREFIX)?.is_empty());
    Ok(())
}

#[test]
fn a_mark_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let (ann, ben, _) = household()?;
    let diary = Notes::new(&ann).capture("Diary", None, None)?.id;
    exchange(&ben, &ann, true)?;
    // As a modified app would: Ben slips a mark into Ann's log.
    let forged = ben.append(&keys::of(&diary), keys::HIDDEN, Some(keys::ON), None)?;
    let mut report = ImportReport::default();
    ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
    assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
    assert!(!ann.is_marked_hidden_from_ai(&diary)?);
    assert!(!ann.is_hidden_from_ai(&diary)?);
    Ok(())
}

#[test]
fn a_hidden_note_is_not_filed_into_a_shared_area() -> TestResult {
    let (ann, _, area) = household()?;
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let diary = notes.capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&diary, true)?;
    let topic = places.add_topic(&area, "Kitchen")?.id;
    let written = ann.all_entries()?.len();
    assert_eq!(
        refused(notes.file(&diary, &area))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(
        refused(notes.file(&diary, &topic))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(ann.all_entries()?.len(), written);
    let health = places.add_area("Health")?.id;
    ann.set_hidden_from_ai(&health, true)?;
    let doctor = notes.capture("Doctor", Some(&health), None)?.id;
    assert_eq!(refused(notes.file(&doctor, &area))?, [health]);
    notes.file(&diary, &places.add_area("Private")?.id)?;
    notes.file(&doctor, &places.add_area("Work")?.id)?;
    notes.file(&doctor, &area)?;
    assert!(!ann.is_hidden_from_ai(&doctor)?);
    Ok(())
}

#[test]
fn hidden_and_shared_notes_do_not_merge_either_way() -> TestResult {
    let (ann, _, area) = household()?;
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let diary = notes.capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&diary, true)?;
    let health = places.add_area("Health")?.id;
    ann.set_hidden_from_ai(&health, true)?;
    let doctor = notes.capture("Doctor", Some(&health), None)?.id;
    let topic = places.add_topic(&area, "Kitchen")?.id;
    let milk = notes.capture("Milk", Some(&area), None)?.id;
    let eggs = notes.capture("Eggs", Some(&topic), None)?.id;
    let plain = notes.capture("Plain", None, None)?.id;
    let written = ann.all_entries()?.len();
    // Hidden into shared, by the marks hiding what merges.
    assert_eq!(
        refused(notes.merge(&diary, &milk))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(
        refused(notes.merge(&doctor, &eggs))?,
        std::slice::from_ref(&health)
    );
    // Shared into hidden, by the marks hiding the target.
    assert_eq!(
        refused(notes.merge(&milk, &diary))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(
        refused(notes.merge(&eggs, &doctor))?,
        std::slice::from_ref(&health)
    );
    let mut both = vec![diary.clone(), health.clone()];
    both.sort();
    assert_eq!(
        refused(notes.merge_all([&doctor, &plain, &diary], &milk))?,
        both
    );
    assert_eq!(
        refused(notes.merge_all([&plain, &milk], &diary))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(ann.all_entries()?.len(), written);
    // Hidden with hidden, shared with shared, and anything with a
    // Note of neither merge as before.
    notes.merge(&doctor, &diary)?;
    notes.merge(&eggs, &milk)?;
    notes.merge_all([&plain], &diary)?;
    assert!(notes.by_id(&doctor)?.is_some_and(|n| n.is_merged()));
    assert!(notes.by_id(&eggs)?.is_some_and(|n| n.is_merged()));
    Ok(())
}

#[test]
fn a_link_another_persons_device_wrote_shows_nothing_hidden() -> TestResult {
    let (ann, ben, _) = household()?;
    let notes = Notes::new(&ann);
    let health = Places::new(&ann).add_area("Health")?.id;
    let doctor = notes.capture("Doctor", Some(&health), None)?.id;
    let diary = notes.capture("Diary", None, None)?.id;
    let later = notes.capture("Diary, later", None, None)?.id;
    notes.merge(&later, &diary)?;
    ann.set_hidden_from_ai(&health, true)?;
    ann.set_hidden_from_ai(&diary, true)?;
    exchange(&ben, &ann, true)?;
    // As a modified app would: Ben slips entries into Ann's log that
    // take her Notes out of the hidden Area and the hidden Note.
    let forged = [
        ben.append(&doctor, note_keys::PLACE, None, None)?.wire(),
        ben.append(&later, note_keys::MERGED, None, None)?.wire(),
    ];
    let mut report = ImportReport::default();
    ann.apply_entries(&forged, &[], Some(&ben.device_id()?), &mut report)?;
    assert_eq!(ann.logged(&doctor, note_keys::PLACE)?, None);
    assert_eq!(ann.hidden_from_ai_by(&doctor)?, [health]);
    assert_eq!(ann.hidden_from_ai_by(&later)?, [diary]);
    Ok(())
}

#[test]
fn sharing_a_hidden_area_or_one_holding_hidden_notes_is_refused() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let health = places.add_area("Health")?.id;
    notes.capture("Doctor", Some(&health), None)?;
    ann.set_hidden_from_ai(&health, true)?;
    let written = ann.all_entries()?.len();
    assert_eq!(refused(ann.share(&health))?, std::slice::from_ref(&health));
    assert_eq!(ann.all_entries()?.len(), written);
    assert!(!ann.is_shared(&health)?);
    let home = places.add_area("Home")?.id;
    let kitchen = places.add_topic(&home, "Kitchen")?.id;
    let recipe = notes.capture("Recipe", Some(&kitchen), None)?.id;
    notes.capture("Diary", Some(&home), None)?;
    let old = notes.capture("Old", Some(&home), None)?.id;
    ann.set_hidden_from_ai(&recipe, true)?;
    ann.set_hidden_from_ai(&old, true)?;
    Routines::new(&ann).trash_note(&old)?;
    let before = ann.all_entries()?.len();
    let mut both = vec![recipe.clone(), old.clone()];
    both.sort_by(|a, b| utf16_cmp(a, b));
    assert_eq!(refused(ann.share(&home))?, both);
    assert_eq!(ann.all_entries()?.len(), before);
    ann.set_hidden_from_ai(&recipe, false)?;
    ann.set_hidden_from_ai(&old, false)?;
    assert_eq!(ann.share(&home)?, home);
    assert!(ann.is_shared(&home)?);
    // A starter Area, before it moves to fresh ids.
    places.plant_starter_set(&["household"])?;
    let household = crate::places::starter_id(place_keys::TYPE_AREA, "household");
    ann.set_hidden_from_ai(&household, true)?;
    let written = ann.all_entries()?.len();
    assert_eq!(
        refused(ann.share(&household))?,
        std::slice::from_ref(&household)
    );
    assert_eq!(ann.all_entries()?.len(), written);
    Ok(())
}

#[test]
fn a_hidden_note_is_not_sent_nor_its_cancellation() -> TestResult {
    let ann = device(1, "Ann")?;
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let health = places.add_area("Health")?.id;
    let doctor = notes.capture("Doctor", Some(&health), None)?.id;
    let diary = notes.capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&health, true)?;
    ann.set_hidden_from_ai(&diary, true)?;
    let written = ann.all_entries()?.len();
    assert_eq!(refused(ann.sent_item_of(&doctor, false))?, [health]);
    assert_eq!(
        refused(ann.sent_item_of(&diary, false))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(
        refused(ann.sent_item_of(&diary, true))?,
        std::slice::from_ref(&diary)
    );
    assert_eq!(ann.all_entries()?.len(), written);
    ann.set_hidden_from_ai(&diary, false)?;
    assert_eq!(ann.sent_item_of(&diary, false)?.text, "Diary");
    Ok(())
}

#[test]
fn a_pack_holding_something_hidden_is_refused() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let none = HashSet::new();
    let health = places.add_area("Health")?.id;
    let back = places.add_topic(&health, "Back")?.id;
    let physio = notes.capture("Physio", Some(&back), None)?.id;
    notes.capture("Doctor", Some(&health), None)?;
    let diary = notes.capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&physio, true)?;
    ann.set_hidden_from_ai(&diary, true)?;
    assert_eq!(
        refused(ann.pack_of(&health, &none))?,
        std::slice::from_ref(&physio)
    );
    assert_eq!(
        refused(ann.pack_of(&back, &none))?,
        std::slice::from_ref(&physio)
    );
    assert_eq!(
        refused(ann.pack_of(&diary, &none))?,
        std::slice::from_ref(&diary)
    );
    ann.set_hidden_from_ai(&physio, false)?;
    assert_eq!(ann.pack_of(&health, &none)?.notes.len(), 2);
    ann.set_hidden_from_ai(&health, true)?;
    assert_eq!(refused(ann.pack_of(&back, &none))?, [health]);
    // A Session picking from a hidden Tally.
    let (laps, _) = tally(&ann, "Laps", None)?;
    let training = notes.capture("Training", None, None)?.id;
    Sessions::new(&ann).make(
        &training,
        std::slice::from_ref(&laps),
        1,
        SessionRule::Rolling,
    )?;
    ann.set_hidden_from_ai(&laps, true)?;
    assert_eq!(refused(ann.pack_of(&training, &none))?, [laps]);
    Ok(())
}

#[test]
fn a_pack_of_everything_leaves_out_what_is_hidden_and_a_backup_keeps_it() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let none = HashSet::new();
    let health = places.add_area("Health")?.id;
    notes.capture("Doctor", Some(&health), None)?;
    let diary = notes.capture("Diary", None, None)?.id;
    notes.capture("Shopping", None, None)?;
    let (laps, _) = tally(&ann, "Laps", None)?;
    let training = notes.capture("Training", None, None)?.id;
    Sessions::new(&ann).make(
        &training,
        std::slice::from_ref(&laps),
        1,
        SessionRule::Rolling,
    )?;
    for id in [&health, &diary, &laps] {
        ann.set_hidden_from_ai(id, true)?;
    }
    let texts: Vec<String> = ann
        .pack_of_all(&none)?
        .notes
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert_eq!(texts, ["Training", "Shopping"]);
    let size_of = |_: &str, _: &str| None;
    let everything = ann.pack_of_everything(&none, |a| a.name.clone(), &size_of)?;
    assert_eq!(everything.pack.notes.len(), 5);
    Ok(())
}

/// What every reading tool without an id hands out, one line each.
fn read_all(store: &Store, task: &str, ids: &[(&str, &String)]) -> Made<Vec<String>> {
    let mut out = Vec::new();
    let mut reads = vec![
        ("list_notes", json!({})),
        ("list_notes", json!({"search": "secret"})),
        ("list_notes", json!({"which": "trash"})),
        ("list_notes", json!({"which": "unsorted"})),
        ("list_places", json!({})),
        ("list_tags", json!({})),
        ("list_routines", json!({})),
        ("list_routines", json!({"trash": true})),
        ("read_settings", json!({})),
        ("read_check_offs", json!({})),
        ("read_skips", json!({})),
        ("read_eaten_frogs", json!({})),
    ];
    for (name, id) in ids {
        let key = match *name {
            "read_note" => "id",
            _ => "note",
        };
        reads.push((name, json!({ key: id })));
    }
    for (name, args) in reads {
        out.push(format!(
            "{name} {args}: {:?}",
            call(store, task, name, &args)?
        ));
    }
    Ok(out)
}

#[test]
fn for_the_ai_what_is_hidden_does_not_exist() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let work = places.add_area("Work")?.id;
    let plan = notes.capture("Plan #work", Some(&work), None)?.id;
    Routines::new(&ann).spawn(&plan, &schedule()?)?;
    let (laps, field) = tally(&ann, "Laps", None)?;
    Tallies::new(&ann).add(&laps, &BTreeMap::from([(field, 3.0)]), "", None)?;
    let training = notes.capture("Training", None, None)?.id;
    Sessions::new(&ann).make(
        &training,
        std::slice::from_ref(&laps),
        1,
        SessionRule::Rolling,
    )?;
    let task = ann.open_ai_task()?;
    let visible = [
        ("read_note", &plan),
        ("read_note", &training),
        ("read_tally_entries", &laps),
        ("read_note_history", &plan),
    ];
    let before = read_all(&ann, &task, &visible)?;

    // Everything a Note can carry, in a hidden Area and on a hidden
    // Note, and a hidden Tally in the pool of the visible Session.
    let health = places.add_area("Health secret")?.id;
    let back = places.add_topic(&health, "Back secret")?.id;
    let file = NewFile {
        name: "secret.txt",
        media_type: "text/plain",
        bytes: b"secret",
        pages: None,
    };
    let physio = notes.capture_with_files(
        "Physio secret #health/back",
        Some(&back),
        None,
        &[png()?],
        &[],
        &[file],
    )?;
    notes.amend(&physio.id, "secret exercises")?;
    let hash = physio.attachments.first().ok_or("a file")?.hash.clone();
    notes.set_attachment_text(&physio.id, &hash, "secret text")?;
    let routine = Routines::new(&ann).spawn(&physio.id, &schedule()?)?.id;
    let first = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
    Routines::new(&ann).mark_done(&routine, &first, None)?;
    let frog = Frogs::new(&ann).spawn(&physio.id)?;
    Frogs::new(&ann).mark_done(&frog.id, None)?;
    let bag = notes.capture("Bag secret\nShoes", Some(&health), None)?.id;
    Checklists::new(&ann).make(&bag)?;
    let (push, field) = tally(&ann, "Push-ups secret", Some(&health))?;
    Tallies::new(&ann).add(&push, &BTreeMap::from([(field, 9.0)]), "", None)?;
    let old = notes.capture("Old secret", Some(&health), None)?.id;
    Routines::new(&ann).trash_note(&old)?;
    let diary = notes.capture("Diary secret #private", None, None)?.id;
    Routines::new(&ann).spawn(&diary, &schedule()?)?;
    let extra = notes.capture("Extra secret", None, None)?.id;
    notes.merge(&extra, &diary)?;
    Sessions::new(&ann).set_pool(&training, &[laps.clone(), push.clone()])?;
    ann.set_hidden_from_ai(&health, true)?;
    ann.set_hidden_from_ai(&diary, true)?;
    // Worked out for the user first, so nothing kept leaks.
    assert!(!TagTree::new(&ann).tree(&HashSet::new())?.is_empty());
    assert_eq!(Search::new(&ann).notes("secret", None)?.len(), 4);

    assert_eq!(read_all(&ann, &task, &visible)?, before);
    // Read by its id, a hidden one reads as one there never was.
    let never = "note:00000000000000000000000000000000";
    let photo = physio.photos.first().ok_or("a photo")?.clone();
    let by_id = [
        ("read_note", json!({"id": physio.id})),
        ("read_note", json!({"id": extra})),
        ("read_photo", json!({"note": physio.id, "photo": photo})),
        (
            "read_attachment_text",
            json!({"file": hash, "note": physio.id}),
        ),
        ("list_notes", json!({"place": health})),
        ("list_routines", json!({"note": physio.id})),
        ("read_tally_entries", json!({"note": push})),
        ("read_ticks", json!({"note": bag})),
        ("read_check_offs", json!({"note": physio.id})),
        ("read_note_history", json!({"note": diary})),
        ("read_session_picks", json!({"id": routine})),
    ];
    for (name, args) in by_id {
        let hidden = format!("{:?}", call(&ann, &task, name, &args)?);
        let mut text = args.to_string();
        let mut shown = hidden.clone();
        for id in [&physio.id, &extra, &health, &push, &bag, &diary, &routine] {
            text = text.replace(id.as_str(), never);
            shown = shown.replace(id.as_str(), never);
        }
        let unknown = format!(
            "{:?}",
            call(&ann, &task, name, &serde_json::from_str(&text)?)?
        );
        assert_eq!(shown, unknown, "{name}");
    }
    // The user still sees all of it.
    assert_eq!(Search::new(&ann).notes("secret", None)?.len(), 4);
    assert!(
        ann.pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .is_empty()
    );
    Ok(())
}

#[test]
fn writes_cannot_reach_what_is_hidden() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let work = places.add_area("Work")?.id;
    let plan = notes.capture("Plan", Some(&work), None)?.id;
    let health = places.add_area("Health")?.id;
    let back = places.add_topic(&health, "Back")?.id;
    let physio = notes.capture("Physio", Some(&back), None)?.id;
    let routine = Routines::new(&ann).spawn(&physio, &schedule()?)?.id;
    let bag = notes.capture("Bag\nShoes", Some(&health), None)?.id;
    Checklists::new(&ann).make(&bag)?;
    let line = Checklists::new(&ann).add(&bag, "Towel", None, false)?;
    let diary = notes.capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&health, true)?;
    ann.set_hidden_from_ai(&diary, true)?;
    let task = ann.open_ai_task()?;
    let never = "note:00000000000000000000000000000000";
    let writes = [
        ("edit_note", json!({"id": diary, "text": "x"})),
        ("amend_note", json!({"id": physio, "text": "x"})),
        ("file_note", json!({"id": plan, "place": health})),
        ("file_note", json!({"id": plan, "place": back})),
        ("file_note", json!({"id": diary, "place": work})),
        ("merge_notes", json!({"id": plan, "into": diary})),
        ("merge_notes", json!({"id": diary, "into": plan})),
        (
            "set_importance",
            json!({"id": diary, "importance": "important"}),
        ),
        ("trash_note", json!({"id": diary})),
        ("restore_note", json!({"id": diary})),
        ("capture_note", json!({"place": back, "text": "x"})),
        ("add_topic", json!({"area": health, "name": "x"})),
        ("rename_place", json!({"id": health, "name": "x"})),
        (
            "set_area_colour",
            json!({"colour": "#ff0000", "id": health}),
        ),
        ("delete_place", json!({"id": back, "notesGo": "unsorted"})),
        ("delete_place", json!({"id": health, "notesGo": "trash"})),
        (
            "add_routine",
            json!({"note": diary, "start": "2026-01-01", "times": ["08:00"]}),
        ),
        (
            "reschedule_routine",
            json!({"id": routine, "start": "2026-01-02", "times": ["09:00"]}),
        ),
        ("trash_routine", json!({"id": routine})),
        (
            "make_tally",
            json!({"fields": [{"name": "Laps"}], "id": diary}),
        ),
        ("make_list", json!({"id": diary, "list": "checklist"})),
        ("add_line", json!({"id": bag, "text": "x"})),
        ("rename_line", json!({"id": line, "text": "x"})),
        ("remove_line", json!({"id": line})),
        (
            "set_checklist_agenda",
            json!({"agenda": "count", "id": bag}),
        ),
        ("make_frog", json!({"id": diary})),
        ("set_text_format", json!({"format": "code", "id": diary})),
    ];
    for (name, args) in writes {
        let hidden = format!("{:?}", call(&ann, &task, name, &args)?);
        let mut text = args.to_string();
        let mut shown = hidden.clone();
        for id in [&diary, &physio, &health, &back, &routine, &bag, &line] {
            let kind = id.split_once(':').map_or("", |(kind, _)| kind);
            let unknown = format!("{kind}:00000000000000000000000000000000");
            text = text.replace(id.as_str(), &unknown);
            shown = shown.replace(id.as_str(), &unknown);
        }
        let unknown = format!(
            "{:?}",
            call(&ann, &task, name, &serde_json::from_str(&text)?)?
        );
        assert_eq!(shown, unknown, "{name} {args}");
        assert!(hidden.contains("is_error: true"), "{name} {args}: {hidden}");
    }
    assert!(
        ann.pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .is_empty()
    );
    assert!(!never.is_empty());
    Ok(())
}

#[test]
fn a_draft_reaching_something_hidden_is_refused_whatever_writes_it() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let plan = notes.capture("Plan", None, None)?.id;
    let health = places.add_area("Health")?.id;
    let diary = notes.capture("Diary", None, None)?.id;
    ann.set_hidden_from_ai(&health, true)?;
    ann.set_hidden_from_ai(&diary, true)?;
    let task = ann.open_ai_task()?;
    // Outside the AI's view the reads see everything: the draft's
    // check alone stands in the way.
    assert!(ann.drafting(&task, || notes.file(&plan, &health)).is_err());
    assert!(ann.drafting(&task, || notes.merge(&plan, &diary)).is_err());
    assert!(ann.drafting(&task, || notes.edit(&diary, "x")).is_err());
    assert!(
        ann.pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .is_empty()
    );
    ann.drafting(&task, || notes.edit(&plan, "Plan B"))?;
    Ok(())
}

#[test]
fn picks_history_and_a_merged_note_name_nothing_hidden() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let (laps, _) = tally(&ann, "Laps", None)?;
    let (push, _) = tally(&ann, "Push-ups", None)?;
    let training = notes.capture("Training", None, None)?.id;
    let pool = [laps.clone(), push.clone()];
    let session =
        Sessions::new(&ann).spawn(&training, &schedule()?, &pool, 2, SessionRule::Rolling)?;
    Sessions::new(&ann).pick_today(&|t: DateTime<Utc>| t.naive_utc())?;
    let health = places.add_area("Health")?.id;
    let plan = notes.capture("Plan", Some(&health), None)?.id;
    notes.file(&plan, &places.add_area("Work")?.id)?;
    let diary = notes.capture("Diary secret", None, None)?.id;
    notes.amend(&diary, "Amend secret")?;
    let reminder = Routines::new(&ann).spawn(&diary, &schedule()?)?.id;
    notes.merge(&diary, &plan)?;
    for id in [&push, &health, &diary] {
        ann.set_hidden_from_ai(id, true)?;
    }
    let task = ann.open_ai_task()?;
    let picks = format!(
        "{:?}",
        call(
            &ann,
            &task,
            "read_session_picks",
            &json!({"id": session.id()})
        )?
    );
    assert!(picks.contains(&laps) && !picks.contains(&push), "{picks}");
    for (name, args) in [
        ("read_note_history", json!({"note": plan})),
        ("read_note", json!({"id": plan})),
        ("list_routines", json!({})),
        ("list_notes", json!({"search": "secret"})),
    ] {
        let out = format!("{:?}", call(&ann, &task, name, &args)?);
        assert!(!out.contains("is_error: true"), "{name}: {out}");
        for hidden in [&health, &diary, &reminder] {
            assert!(!out.contains(hidden.as_str()), "{name}: {out}");
        }
        assert!(!out.contains("secret"), "{name}: {out}");
    }
    Ok(())
}

#[test]
fn a_sessions_pool_keeps_its_hidden_tallies_and_a_place_holding_one_stays() -> TestResult {
    let ann = store();
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    let (laps, _) = tally(&ann, "Laps", None)?;
    let (push, _) = tally(&ann, "Push-ups", None)?;
    let (sit, _) = tally(&ann, "Sit-ups", None)?;
    let training = notes.capture("Training", None, None)?.id;
    Sessions::new(&ann).make(
        &training,
        &[laps.clone(), push.clone()],
        1,
        SessionRule::Rolling,
    )?;
    ann.set_hidden_from_ai(&push, true)?;
    let home = places.add_area("Home")?.id;
    let diary = notes.capture("Diary", Some(&home), None)?.id;
    ann.set_hidden_from_ai(&diary, true)?;
    let task = ann.open_ai_task()?;
    let read = call(&ann, &task, "read_note", &json!({"id": training}))?;
    assert!(!format!("{read:?}").contains(&push), "{read:?}");
    let set = call(
        &ann,
        &task,
        "set_session",
        &json!({"id": training, "pool": [sit]}),
    )?;
    assert!(!set.is_error, "{set:?}");
    let deleted = call(
        &ann,
        &task,
        "delete_place",
        &json!({"id": home, "notesGo": "unsorted"}),
    )?;
    assert!(deleted.is_error, "{deleted:?}");
    ann.accept_pending(&task)?;
    let setup = Sessions::new(&ann)
        .setup_of(&training)?
        .ok_or("a Session")?;
    assert_eq!(setup.pool, [sit, push]);
    assert!(places.area_by_id(&home)?.is_some());
    assert_eq!(notes.by_id(&diary)?.and_then(|n| n.place), Some(home));
    Ok(())
}
