//! The settings of the person rather than of this Device (#211), under
//! the keys and in the forms the Dart core's `person_settings.dart` uses:
//! kept in the log like an Override, so they are the same on all of the
//! user's own Devices and never reach another person. What another
//! person's Device wrote counts for nothing. The per-Device ones are
//! [`crate::DeviceSettings`].

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::places::{Area, Places};
use crate::store::Store;
use crate::{Error, Result};

/// Where the person's own settings lie in the log.
pub mod keys {
    /// The entity holding them, outside every Area like an Override:
    /// sealed under the personal key, it goes to the user's own Devices
    /// only, and only what they wrote counts.
    pub const ENTITY: &str = "settings:person";
    /// "Move a Note to the Trash when its one-time Reminder is done":
    /// [`ON`] while it is on; absent, null or any other value for off.
    pub const TRASH_DONE_REMINDER: &str = "trashDoneReminder";
    pub const ON: &str = "on";
    /// Simple or Advanced (#197): [`SIMPLE`] or [`ADVANCED`] once
    /// chosen; absent, null or any other value reads as Advanced, so
    /// nothing is hidden from someone who never chose.
    pub const FEATURES: &str = "features";
    pub const SIMPLE: &str = "simple";
    pub const ADVANCED: &str = "advanced";
    /// The person's own order of the Areas (#280): a JSON list of Area
    /// ids, first to last. Ids that are no Area here, repeats and
    /// anything not a string are skipped; absent, null or anything not a
    /// list reads as no order, so the Areas show by name.
    pub const AREA_ORDER: &str = "areaOrder";
}

/// The person's own settings in the log.
pub struct PersonSettings<'a> {
    store: &'a Store,
}

impl<'a> PersonSettings<'a> {
    pub fn new(store: &'a Store) -> PersonSettings<'a> {
        PersonSettings { store }
    }

    /// Whether Done on a one-time Reminder that is the only Routine or
    /// Reminder of a plain Note moves the Note to the trash with it
    /// ([`crate::Routines::check_off`]). Off until switched on; a value
    /// no app writes reads as off, so nothing is trashed by it.
    pub fn trashes_done_reminder(&self) -> Result<bool> {
        let value =
            self.store
                .own_current(keys::ENTITY, keys::TRASH_DONE_REMINDER, &mut HashMap::new())?;
        Ok(value.as_deref() == Some(keys::ON))
    }

    /// Switches it on or off for all of the user's own Devices; the same
    /// again writes nothing.
    pub fn set_trashes_done_reminder(&self, on: bool) -> Result<()> {
        if self.trashes_done_reminder()? == on {
            return Ok(());
        }
        let value = on.then_some(keys::ON);
        self.store
            .append(keys::ENTITY, keys::TRASH_DONE_REMINDER, value, None)
            .map(drop)
    }

    /// Whether making Tallies, Sessions, Frogs and Topics, the format bar
    /// and Shared Area roles are offered (#197). Only [`keys::SIMPLE`]
    /// hides them; never chosen, or a value no app writes, reads as
    /// Advanced. What exists stays visible and usable either way.
    pub fn advanced(&self) -> Result<bool> {
        Ok(self.features()?.as_deref() != Some(keys::SIMPLE))
    }

    /// Chooses Simple or Advanced for all of the user's own Devices; the
    /// same value stored again writes nothing, while Advanced chosen
    /// where none was stored is written, so the choice is recorded.
    pub fn set_advanced(&self, on: bool) -> Result<()> {
        let value = if on { keys::ADVANCED } else { keys::SIMPLE };
        if self.features()?.as_deref() == Some(value) {
            return Ok(());
        }
        self.store
            .append(keys::ENTITY, keys::FEATURES, Some(value), None)
            .map(drop)
    }

    fn features(&self) -> Result<Option<String>> {
        self.store
            .own_current(keys::ENTITY, keys::FEATURES, &mut HashMap::new())
    }

    /// Every Area in the person's own order (#280), as the Areas list and
    /// the sidebar show them: those in the stored order first, as it has
    /// them, then the Areas missing from it (a new one, a Shared Area
    /// joined later) by name, as [`Places::areas`] lists them. With no
    /// order stored, all of them by name.
    pub fn areas_in_order(&self) -> Result<Vec<Area>> {
        let mut by_name: Vec<Option<Area>> = Places::new(self.store)
            .areas()?
            .into_iter()
            .map(Some)
            .collect();
        let mut ordered = Vec::new();
        for id in self.held_area_order()? {
            let slot = by_name
                .iter_mut()
                .find(|a| a.as_ref().is_some_and(|a| a.id == id));
            ordered.extend(slot.and_then(Option::take));
        }
        ordered.extend(by_name.into_iter().flatten());
        Ok(ordered)
    }

    /// Puts the Area `area_id` at `index` of [`Self::areas_in_order`],
    /// counted after it was taken out, for all of the user's own Devices.
    /// An index past the end puts it last. The whole order is stored, so
    /// a new Area goes to the end. Refused when `area_id` is no Area.
    pub fn move_area(&self, area_id: &str, index: usize) -> Result<()> {
        let mut ids = self.shown_area_ids()?;
        let Some(at) = ids.iter().position(|id| id == area_id) else {
            return Err(Error::Invalid(format!("not an Area: {area_id}")));
        };
        let id = ids.remove(at);
        ids.insert(index.min(ids.len()), id);
        self.store_area_order(&ids)
    }

    /// Puts the Areas in the order of `area_ids` for all of the user's
    /// own Devices; the Areas it leaves out follow in the order they had.
    /// Ids that are no Area are skipped.
    pub fn set_area_order<S: AsRef<str>>(&self, area_ids: &[S]) -> Result<()> {
        let shown = self.shown_area_ids()?;
        let mut ids: Vec<String> = Vec::new();
        for id in area_ids.iter().map(AsRef::as_ref) {
            if shown.iter().any(|s| s == id) && !ids.iter().any(|i| i == id) {
                ids.push(id.to_string());
            }
        }
        for id in shown {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        self.store_area_order(&ids)
    }

    fn shown_area_ids(&self) -> Result<Vec<String>> {
        Ok(self.areas_in_order()?.into_iter().map(|a| a.id).collect())
    }

    /// Stores `ids` as the order; the order already shown writes nothing.
    fn store_area_order(&self, ids: &[String]) -> Result<()> {
        if self.shown_area_ids()? == ids {
            return Ok(());
        }
        let value = json!(ids).to_string();
        self.store
            .append(keys::ENTITY, keys::AREA_ORDER, Some(&value), None)
            .map(drop)
    }

    /// The ids of the stored order, as written by the user's own Devices,
    /// the newest such entry winning; empty when there is none or it is
    /// not a list.
    fn held_area_order(&self) -> Result<Vec<String>> {
        let held = self
            .store
            .own_current(keys::ENTITY, keys::AREA_ORDER, &mut HashMap::new())?;
        let Some(Ok(Value::Array(list))) = held.map(|v| serde_json::from_str::<Value>(&v)) else {
            return Ok(Vec::new());
        };
        Ok(list
            .into_iter()
            .filter_map(|v| match v {
                Value::String(id) => Some(id),
                _ => None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
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
}
