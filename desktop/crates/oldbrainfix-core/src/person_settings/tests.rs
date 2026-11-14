use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
    sync_request_as,
};
use crate::schedule::{Day, Schedule};
use crate::shared::{AreaRole, Invite};
use crate::signing::{ImportReport, SigningKey};
use crate::store::tests::ticking;
use crate::{Notes, OwnProfile, PairedDevice, Places, Routines};
use chrono::Duration;

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

#[test]
fn off_until_switched_on_and_the_same_again_writes_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let settings = PersonSettings::new(&ann);
    assert!(!settings.trashes_done_reminder()?);
    settings.set_trashes_done_reminder(true)?;
    assert!(settings.trashes_done_reminder()?);
    let written = ann.all_entries()?.len();
    settings.set_trashes_done_reminder(true)?;
    assert_eq!(ann.all_entries()?.len(), written);
    settings.set_trashes_done_reminder(false)?;
    assert!(!settings.trashes_done_reminder()?);
    assert_eq!(ann.current(keys::ENTITY, keys::TRASH_DONE_REMINDER)?, None);
    Ok(())
}

#[test]
fn a_value_no_app_writes_reads_as_off() -> TestResult {
    let ann = device(1, "Ann")?;
    ann.append(keys::ENTITY, keys::TRASH_DONE_REMINDER, Some("yes"), None)?;
    assert!(!PersonSettings::new(&ann).trashes_done_reminder()?);
    Ok(())
}

#[test]
fn it_travels_between_own_devices() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    PersonSettings::new(&ann).set_trashes_done_reminder(true)?;
    exchange(&desk, &ann, false)?;
    assert!(PersonSettings::new(&desk).trashes_done_reminder()?);
    PersonSettings::new(&desk).set_trashes_done_reminder(false)?;
    exchange(&desk, &ann, false)?;
    assert!(!PersonSettings::new(&ann).trashes_done_reminder()?);
    Ok(())
}

#[test]
fn it_never_reaches_another_person() -> TestResult {
    let (ann, ben, area) = household()?;
    PersonSettings::new(&ann).set_trashes_done_reminder(true)?;
    Notes::new(&ann).capture("Milk", Some(&area), None)?;
    exchange(&ben, &ann, true)?;
    assert!(ben.logged_history(keys::ENTITY)?.is_empty());
    assert!(!PersonSettings::new(&ben).trashes_done_reminder()?);
    Ok(())
}

#[test]
fn what_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let (ann, ben, _) = household()?;
    exchange(&ben, &ann, true)?;
    // As a modified app would: Ben slips the setting into Ann's log.
    let forged = ben.append(
        keys::ENTITY,
        keys::TRASH_DONE_REMINDER,
        Some(keys::ON),
        None,
    )?;
    let mut report = ImportReport::default();
    ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
    assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
    assert!(!PersonSettings::new(&ann).trashes_done_reminder()?);
    Ok(())
}

#[test]
fn advanced_until_simple_is_chosen_and_the_same_again_writes_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let settings = PersonSettings::new(&ann);
    assert!(settings.advanced()?);
    settings.set_advanced(false)?;
    assert!(!settings.advanced()?);
    assert_eq!(
        ann.current(keys::ENTITY, keys::FEATURES)?.as_deref(),
        Some(keys::SIMPLE)
    );
    let written = ann.all_entries()?.len();
    settings.set_advanced(false)?;
    assert_eq!(ann.all_entries()?.len(), written);
    settings.set_advanced(true)?;
    assert!(settings.advanced()?);
    assert_eq!(
        ann.current(keys::ENTITY, keys::FEATURES)?.as_deref(),
        Some(keys::ADVANCED)
    );
    Ok(())
}

#[test]
fn advanced_chosen_while_none_was_stored_is_written() -> TestResult {
    let ann = device(1, "Ann")?;
    let settings = PersonSettings::new(&ann);
    let written = ann.all_entries()?.len();
    settings.set_advanced(true)?;
    assert_eq!(ann.all_entries()?.len(), written + 1);
    settings.set_advanced(true)?;
    assert_eq!(ann.all_entries()?.len(), written + 1);
    Ok(())
}

#[test]
fn a_features_value_no_app_writes_reads_as_advanced() -> TestResult {
    let ann = device(1, "Ann")?;
    ann.append(keys::ENTITY, keys::FEATURES, Some("easy"), None)?;
    assert!(PersonSettings::new(&ann).advanced()?);
    Ok(())
}

#[test]
fn simple_travels_between_own_devices_and_never_to_another_person() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    PersonSettings::new(&ann).set_advanced(false)?;
    exchange(&desk, &ann, false)?;
    assert!(!PersonSettings::new(&desk).advanced()?);
    PersonSettings::new(&desk).set_advanced(true)?;
    exchange(&desk, &ann, false)?;
    assert!(PersonSettings::new(&ann).advanced()?);
    let (ann, ben, area) = household()?;
    PersonSettings::new(&ann).set_advanced(false)?;
    Notes::new(&ann).capture("Milk", Some(&area), None)?;
    exchange(&ben, &ann, true)?;
    assert!(ben.logged_history(keys::ENTITY)?.is_empty());
    assert!(PersonSettings::new(&ben).advanced()?);
    Ok(())
}

#[test]
fn simple_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let (ann, ben, _) = household()?;
    exchange(&ben, &ann, true)?;
    let forged = ben.append(keys::ENTITY, keys::FEATURES, Some(keys::SIMPLE), None)?;
    let mut report = ImportReport::default();
    ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
    assert!(PersonSettings::new(&ann).advanced()?);
    Ok(())
}

fn names(store: &Store) -> Made<Vec<String>> {
    let areas = PersonSettings::new(store).areas_in_order()?;
    Ok(areas.into_iter().map(|a| a.name).collect())
}

fn ids(store: &Store) -> Made<Vec<String>> {
    let areas = PersonSettings::new(store).areas_in_order()?;
    Ok(areas.into_iter().map(|a| a.id).collect())
}

/// Ann with the Areas Work, Home and Car, by their names.
fn three_areas() -> Made<(Store, String, String, String)> {
    let ann = device(1, "Ann")?;
    let places = Places::new(&ann);
    let work = places.add_area("Work")?.id;
    let home = places.add_area("Home")?.id;
    let car = places.add_area("Car")?.id;
    Ok((ann, work, home, car))
}

#[test]
fn areas_by_name_until_moved_and_the_same_place_again_writes_nothing() -> TestResult {
    let (ann, work, home, car) = three_areas()?;
    let settings = PersonSettings::new(&ann);
    assert_eq!(names(&ann)?, ["Car", "Home", "Work"]);
    settings.move_area(&work, 0)?;
    assert_eq!(names(&ann)?, ["Work", "Car", "Home"]);
    assert_eq!(
        ann.current(keys::ENTITY, keys::AREA_ORDER)?,
        Some(json!([work, car, home]).to_string())
    );
    let written = ann.all_entries()?.len();
    settings.move_area(&work, 0)?;
    settings.set_area_order(&[&work, &car])?;
    assert_eq!(ann.all_entries()?.len(), written);
    settings.move_area(&car, 99)?;
    assert_eq!(names(&ann)?, ["Work", "Home", "Car"]);
    let by_name: Vec<_> = Places::new(&ann)
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(by_name, ["Car", "Home", "Work"]);
    Ok(())
}

#[test]
fn nothing_is_stored_while_the_order_stays_by_name() -> TestResult {
    let (ann, _, home, car) = three_areas()?;
    let settings = PersonSettings::new(&ann);
    settings.move_area(&car, 0)?;
    settings.set_area_order(&[&car, &home])?;
    assert!(ann.logged_history(keys::ENTITY)?.is_empty());
    Ok(())
}

#[test]
fn new_areas_go_to_the_end_and_those_missing_from_the_order_by_name() -> TestResult {
    let (ann, work, _, car) = three_areas()?;
    let settings = PersonSettings::new(&ann);
    let places = Places::new(&ann);
    settings.move_area(&work, 0)?;
    places.add_area("Garden")?;
    assert_eq!(names(&ann)?, ["Work", "Car", "Home", "Garden"]);
    let attic = places.add_area("Attic")?.id;
    assert_eq!(names(&ann)?, ["Work", "Car", "Home", "Attic", "Garden"]);
    settings.move_area(&attic, 1)?;
    places.delete_area(&car, crate::NotesGo::Unsorted)?;
    assert_eq!(names(&ann)?, ["Work", "Attic", "Home", "Garden"]);
    Ok(())
}

#[test]
fn the_areas_left_out_of_a_new_order_keep_the_order_they_had() -> TestResult {
    let (ann, work, home, _) = three_areas()?;
    let settings = PersonSettings::new(&ann);
    settings.move_area(&work, 0)?;
    settings.set_area_order(&[home.as_str(), "area:gone", home.as_str()])?;
    assert_eq!(names(&ann)?, ["Home", "Work", "Car"]);
    Ok(())
}

#[test]
fn moving_what_is_no_area_is_refused() -> TestResult {
    let (ann, _, home, _) = three_areas()?;
    let settings = PersonSettings::new(&ann);
    assert!(settings.move_area("area:gone", 0).is_err());
    let topic = Places::new(&ann).add_topic(&home, "Garden")?;
    assert!(settings.move_area(&topic.id, 0).is_err());
    assert!(ann.logged_history(keys::ENTITY)?.is_empty());
    Ok(())
}

#[test]
fn ids_that_are_no_area_repeats_and_other_values_are_skipped() -> TestResult {
    let (ann, work, home, _) = three_areas()?;
    let topic = Places::new(&ann).add_topic(&home, "Garden")?.id;
    let order = json!([work, "area:gone", topic, 7, null, work]).to_string();
    ann.append(keys::ENTITY, keys::AREA_ORDER, Some(&order), None)?;
    assert_eq!(names(&ann)?, ["Work", "Car", "Home"]);
    for value in ["not json", r#"{"a":1}"#, r#""area:x""#, "", "null"] {
        ann.append(keys::ENTITY, keys::AREA_ORDER, Some(value), None)?;
        assert_eq!(names(&ann)?, ["Car", "Home", "Work"], "{value}");
    }
    Ok(())
}

#[test]
fn an_area_order_dated_back_loses_to_the_newer_one() -> TestResult {
    let (ann, work, home, _) = three_areas()?;
    PersonSettings::new(&ann).move_area(&work, 0)?;
    let old = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")?.to_utc();
    let order = json!([home]).to_string();
    ann.append(keys::ENTITY, keys::AREA_ORDER, Some(&order), Some(old))?;
    assert_eq!(names(&ann)?, ["Work", "Car", "Home"]);
    Ok(())
}

#[test]
fn the_area_order_travels_between_own_devices() -> TestResult {
    let (ann, work, home, _) = three_areas()?;
    let desk = device(3, "Ann at the desk")?;
    exchange(&desk, &ann, false)?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    PersonSettings::new(&ann).move_area(&work, 0)?;
    exchange(&desk, &ann, false)?;
    assert_eq!(names(&desk)?, ["Work", "Car", "Home"]);
    PersonSettings::new(&desk).move_area(&home, 0)?;
    exchange(&desk, &ann, false)?;
    assert_eq!(names(&ann)?, ["Home", "Work", "Car"]);
    Ok(())
}

#[test]
fn a_starter_area_keeps_its_place_when_it_is_shared() -> TestResult {
    let (ann, ..) = three_areas()?;
    let places = Places::new(&ann);
    places.plant_starter_set(&["household"])?;
    let house = places
        .areas()?
        .into_iter()
        .find(|a| a.starter.as_deref() == Some("household"))
        .ok_or("the starter Area")?;
    PersonSettings::new(&ann).move_area(&house.id, 0)?;
    let before = names(&ann)?;
    let shared = ann.share(&house.id)?;
    assert_ne!(shared, house.id);
    assert_eq!(names(&ann)?, before);
    assert_eq!(ids(&ann)?.first(), Some(&shared));
    Ok(())
}

#[test]
fn the_order_of_the_areas_never_reaches_another_person() -> TestResult {
    let (ann, ben, area) = household()?;
    let attic = Places::new(&ann).add_area("Attic")?.id;
    PersonSettings::new(&ann).move_area(&attic, 99)?;
    exchange(&ben, &ann, true)?;
    assert!(ben.logged_history(keys::ENTITY)?.is_empty());
    let bens = Places::new(&ben).add_area("Bike")?.id;
    assert_eq!(ids(&ben)?, [bens.clone(), area.clone()]);
    PersonSettings::new(&ben).move_area(&bens, 99)?;
    exchange(&ben, &ann, true)?;
    assert_eq!(ids(&ben)?, [area.clone(), bens]);
    assert_eq!(ids(&ann)?, [area, attic]);
    Ok(())
}

#[test]
fn an_area_order_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let (ann, ben, area) = household()?;
    let attic = Places::new(&ann).add_area("Attic")?.id;
    exchange(&ben, &ann, true)?;
    let order = json!([area, attic]).to_string();
    let forged = ben.append(keys::ENTITY, keys::AREA_ORDER, Some(&order), None)?;
    let mut report = ImportReport::default();
    ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
    assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
    assert_eq!(ids(&ann)?, [attic, area]);
    Ok(())
}

/// The starter Areas as a German phone shows them.
fn de(area: &Area) -> String {
    match area.starter.as_deref() {
        Some("household") => "Haushalt",
        Some("work") => "Arbeit",
        Some("car") => "Auto",
        Some("health") => "Gesundheit",
        _ => area.name.as_str(),
    }
    .to_string()
}

fn shown(store: &Store) -> Made<Vec<String>> {
    let areas = PersonSettings::new(store).areas_in_order_by(de)?;
    Ok(areas.iter().map(de).collect())
}

fn starter(slug: &str) -> String {
    crate::places::starter_id(crate::places::keys::TYPE_AREA, slug)
}

/// Ann with the whole Starter Set.
fn starter_set() -> Made<Store> {
    let ann = device(1, "Ann")?;
    Places::new(&ann).plant_starter_set(&["household", "work", "car", "health"])?;
    Ok(ann)
}

#[test]
fn until_moved_the_areas_go_by_the_name_shown_in_lower_case() -> TestResult {
    let ann = starter_set()?;
    assert_eq!(shown(&ann)?, ["Arbeit", "Auto", "Gesundheit", "Haushalt"]);
    assert_eq!(names(&ann)?, ["Car", "Health", "Household", "Work"]);
    let places = Places::new(&ann);
    places.add_area("bike")?;
    places.add_area("Zoo")?;
    assert_eq!(
        shown(&ann)?,
        ["Arbeit", "Auto", "bike", "Gesundheit", "Haushalt", "Zoo"]
    );
    assert_eq!(
        names(&ann)?,
        ["bike", "Car", "Health", "Household", "Work", "Zoo"]
    );
    Ok(())
}

#[test]
fn a_move_counts_its_place_in_the_order_shown() -> TestResult {
    let ann = starter_set()?;
    let settings = PersonSettings::new(&ann);
    let written = ann.all_entries()?.len();
    settings.move_area_by(&starter("work"), 0, de)?;
    settings.set_area_order_by(&[starter("work"), starter("car")], de)?;
    assert_eq!(ann.all_entries()?.len(), written);
    settings.move_area_by(&starter("household"), 1, de)?;
    assert_eq!(shown(&ann)?, ["Arbeit", "Haushalt", "Auto", "Gesundheit"]);
    assert_eq!(
        ann.current(keys::ENTITY, keys::AREA_ORDER)?,
        Some(
            json!([
                starter("work"),
                starter("household"),
                starter("car"),
                starter("health")
            ])
            .to_string()
        )
    );
    Ok(())
}

#[test]
fn the_areas_left_out_of_a_new_order_keep_the_order_shown() -> TestResult {
    let ann = starter_set()?;
    PersonSettings::new(&ann).set_area_order_by(&[starter("health")], de)?;
    assert_eq!(shown(&ann)?, ["Gesundheit", "Arbeit", "Auto", "Haushalt"]);
    Ok(())
}

#[test]
fn those_missing_from_the_order_go_by_the_name_shown() -> TestResult {
    let ann = starter_set()?;
    PersonSettings::new(&ann).move_area_by(&starter("health"), 0, de)?;
    let places = Places::new(&ann);
    for name in ["Zoo", "Bike", "apple"] {
        places.add_area(name)?;
    }
    assert_eq!(shown(&ann)?[4..], ["apple", "Bike", "Zoo"]);
    Ok(())
}

#[test]
fn sharing_a_starter_area_leaves_those_missing_from_the_order_by_name() -> TestResult {
    let ann = starter_set()?;
    PersonSettings::new(&ann).move_area_by(&starter("health"), 0, de)?;
    let places = Places::new(&ann);
    places.add_area("Banana")?;
    places.add_area("apple")?;
    let shared = ann.share(&starter("car"))?;
    assert_ne!(shared, starter("car"));
    let third = PersonSettings::new(&ann).areas_in_order_by(de)?;
    assert_eq!(third.get(2).map(|a| a.id.as_str()), Some(shared.as_str()));
    places.add_area("avocado")?;
    assert_eq!(
        shown(&ann)?,
        [
            "Gesundheit",
            "Arbeit",
            "Auto",
            "Haushalt",
            "apple",
            "avocado",
            "Banana"
        ]
    );
    Ok(())
}

#[test]
fn a_members_skip_and_tomorrow_count_on_every_device() -> TestResult {
    let (ann, ben, area) = household()?;
    let bins = Notes::new(&ann).capture("Bins", Some(&area), None)?;
    let eight = crate::schedule::ClockTime::new(8, 0);
    let daily = crate::schedule::Recurrence::parse("FREQ=DAILY")?;
    let schedule = Schedule::new(
        Day::new(2026, 3, 1),
        &eight.into_iter().collect::<Vec<_>>(),
        Some(daily),
    )
    .ok_or("a schedule")?;
    let routine = Routines::new(&ann).spawn(&bins.id, &schedule)?;
    exchange(&ben, &ann, true)?;
    let skipped = crate::schedule::Occurrence::new(Day::new(2026, 3, 2), eight);
    let moved = crate::schedule::Occurrence::new(Day::new(2026, 3, 3), eight);
    Routines::new(&ben).skip(&routine.id, &skipped, None)?;
    Routines::new(&ben).tomorrow(&routine.id, &moved, None)?;
    exchange(&ben, &ann, true)?;
    for store in [&ann, &ben] {
        let read = Routines::new(store)
            .by_id(&routine.id)?
            .ok_or("the Routine")?;
        assert!(read.skipped_at(&skipped).is_some());
        assert!(read.skipped_at(&moved).is_some());
        assert_eq!(Vec::from_iter(&read.tomorrow), [&moved.key()]);
    }
    Ok(())
}

#[test]
fn a_members_done_moves_nothing_of_another_persons_note() -> TestResult {
    let (ann, ben, area) = household()?;
    let dentist = Notes::new(&ann).capture("Dentist", Some(&area), None)?;
    let start = Day::today(ann.now(), &|utc: chrono::DateTime<chrono::Utc>| {
        utc.naive_utc()
    });
    let schedule = Schedule::new(start, &[], None).ok_or("a schedule")?;
    let reminder = Routines::new(&ann).spawn(&dentist.id, &schedule)?;
    exchange(&ben, &ann, true)?;
    PersonSettings::new(&ben).set_trashes_done_reminder(true)?;
    let [occurrence] = reminder.schedule.between(start, start)[..] else {
        return Err("one Occurrence".into());
    };
    let routines = Routines::new(&ben);
    assert!(!routines.check_off(&reminder.id, &occurrence, None)?);
    let note = Notes::new(&ben).by_id(&dentist.id)?.ok_or("the Note")?;
    assert!(!note.is_trashed());
    let done = routines.by_id(&reminder.id)?.ok_or("the Reminder")?;
    assert!(done.done_at(&occurrence).is_some());
    Ok(())
}
