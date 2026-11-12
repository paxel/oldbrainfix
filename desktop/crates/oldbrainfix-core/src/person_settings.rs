//! The settings of the person rather than of this Device (#211), under
//! the keys and in the forms the Dart core's `person_settings.dart` uses:
//! kept in the log like an Override, so they are the same on all of the
//! user's own Devices and never reach another person. What another
//! person's Device wrote counts for nothing. The per-Device ones are
//! [`crate::DeviceSettings`].

use std::collections::HashMap;

use crate::Result;
use crate::store::Store;

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
