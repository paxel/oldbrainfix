//! Routines and Reminders as the Dart core's `routines.dart` reads and
//! writes them: spawned from a Note (ADR 0005), on a fixed schedule (ADR
//! 0006), each Occurrence checked off on its own, with a Re-nag interval
//! and Escalation of their own, and a trash. Every write appends the same
//! entries, in the same order and read from the clock as often, as the
//! Dart core's.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::notes::{Note, Notes, keys as note_keys};
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule, WallClock};
use crate::store::{Fields, Store, utf16_cmp};
use crate::{Error, Result};

/// Field keys of a Routine.
pub mod keys {
    pub const TYPE: &str = "$type";
    pub const TYPE_ROUTINE: &str = "routine";
    /// The Note the Routine was spawned from (ADR 0005).
    pub const NOTE: &str = "note";
    /// The schedule's start day, `YYYY-MM-DD`.
    pub const START: &str = "start";
    /// The schedule's times as `HH:MM` joined by commas; empty for all day.
    pub const TIMES: &str = "times";
    /// The schedule's rule as an RRULE value; absent for a Reminder.
    pub const RULE: &str = "rule";
    /// Prefix of the field marking an Occurrence Done, keyed by its
    /// wall-clock key; the value is when it was checked off.
    pub const DONE_PREFIX: &str = "done@";
    /// The Routine's own Re-nag interval in minutes.
    pub const RENAG: &str = "renag";
    /// After how many ignored Re-nags the Routine escalates.
    pub const ESCALATION: &str = "escalation";
    /// When the Routine was moved to the trash.
    pub const TRASHED: &str = "trashed";
    /// The Note the Routine went to the trash with.
    pub const TRASHED_WITH: &str = "trashedWith";
    /// The fields that make up the schedule.
    pub const SCHEDULE_FIELDS: [&str; 3] = [START, TIMES, RULE];

    /// The field marking `occurrence` Done.
    pub fn done(occurrence: &crate::schedule::Occurrence) -> String {
        format!("{DONE_PREFIX}{}", occurrence.key())
    }
}

/// Something to be done on a schedule, spawned from a Note (CONTEXT.md).
/// A Reminder is a Routine whose schedule happens once (ADR 0005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routine {
    /// Entity id, `routine:<id>`.
    pub id: String,
    /// The Note it was spawned from, as stored.
    pub note_id: String,
    pub schedule: Schedule,
    /// When each Done Occurrence was checked off, by its key.
    pub done: BTreeMap<String, DateTime<Utc>>,
    /// Its own Re-nag interval; `None` for the Device's default.
    pub renag: Option<Duration>,
    /// After how many ignored Re-nags it escalates; `None` when off.
    pub escalation: Option<i64>,
    /// When the schedule was last written or the Routine last left the
    /// trash: Occurrences due before then were never asked for.
    pub scheduled_at: Option<DateTime<Utc>>,
    pub trashed: Option<DateTime<Utc>>,
    /// The Note it went to the trash with.
    pub trashed_with: Option<String>,
}

impl Routine {
    pub fn is_reminder(&self) -> bool {
        self.schedule.once()
    }

    pub fn is_trashed(&self) -> bool {
        self.trashed.is_some()
    }

    /// When `occurrence` was checked off; `None` while it is not Done.
    pub fn done_at(&self, occurrence: &Occurrence) -> Option<DateTime<Utc>> {
        self.done.get(&occurrence.key()).copied()
    }
}

/// One row of the Agenda: an Occurrence of a Routine, with the Note it
/// comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgendaRow {
    pub routine: Routine,
    pub occurrence: Occurrence,
    /// The Note the Routine links back to; `None` when it is gone.
    pub note: Option<Note>,
}

impl AgendaRow {
    pub fn done(&self) -> Option<DateTime<Utc>> {
        self.routine.done_at(&self.occurrence)
    }
}

/// The Agenda's content (CONTEXT.md): today's Occurrences, all-day first
/// and then by time, and tomorrow's the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agenda {
    pub today: Day,
    pub today_rows: Vec<AgendaRow>,
    pub tomorrow_rows: Vec<AgendaRow>,
}

/// Spawning, changing and checking off Routines and Reminders.
pub struct Routines<'a> {
    store: &'a Store,
}

impl<'a> Routines<'a> {
    pub fn new(store: &'a Store) -> Routines<'a> {
        Routines { store }
    }

    /// Spawns a Routine (or a Reminder, when `schedule` happens once) from
    /// the Note `note_id`. The Note leaves Unsorted; it can spawn several.
    pub fn spawn(&self, note_id: &str, schedule: &Schedule) -> Result<Routine> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let Some(note) = note.filter(|n| !n.is_trashed() && !n.is_merged()) else {
            return Err(Error::Invalid(format!(
                "not a Note in the lists: {note_id}"
            )));
        };
        if !self
            .store
            .may_manage(self.store.area_of(note_id)?.as_deref())?
        {
            return Err(Error::Invalid(format!(
                "not a Manager of the Area of {note_id}"
            )));
        }
        let id = format!("routine:{}", self.store.new_id()?);
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_ROUTINE), None)?;
            s.append(&id, keys::NOTE, Some(note_id), None)?;
            self.write_schedule(&id, schedule)?;
            if !note.spawned {
                let at = dart_iso(s.now());
                s.append(note_id, note_keys::SPAWNED, Some(&at), None)?;
            }
            Ok(())
        })?;
        self.by_id(&id)?
            .ok_or_else(|| Error::Invalid("the spawned Routine is gone".into()))
    }

    /// Creates a Routine without a Note at hand: its Note, holding
    /// `text`, is captured behind the scenes and is not Unsorted.
    pub fn create(&self, text: &str, schedule: &Schedule) -> Result<Routine> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::Invalid("the text must not be empty".into()));
        }
        let note = Notes::new(self.store).capture(text, None, None)?;
        self.spawn(&note.id, schedule)
    }

    /// Gives the Routine `id` a new schedule. What was Done stays Done;
    /// the same schedule again writes nothing.
    pub fn reschedule(&self, id: &str, schedule: &Schedule) -> Result<()> {
        self.require(id)?;
        self.store.batch(|| self.write_schedule(id, schedule))
    }

    /// Gives the Routine `id` its own Re-nag interval, or back the
    /// Device's default with `None`.
    pub fn set_renag(&self, id: &str, renag: Option<Duration>) -> Result<()> {
        let routine = self.require(id)?;
        if routine.renag == renag {
            return Ok(());
        }
        if let Some(renag) = renag
            && renag.num_minutes() < 1
        {
            return Err(Error::Invalid("a Re-nag needs at least a minute".into()));
        }
        let value = renag.map(|r| r.num_minutes().to_string());
        self.store
            .append(id, keys::RENAG, value.as_deref(), None)
            .map(drop)
    }

    /// Turns Escalation on for the Routine `id`, after `after` ignored
    /// Re-nags, or off with `None`.
    pub fn set_escalation(&self, id: &str, after: Option<i64>) -> Result<()> {
        let routine = self.require(id)?;
        if routine.escalation == after {
            return Ok(());
        }
        if after.is_some_and(|a| a < 1) {
            return Err(Error::Invalid(
                "Escalation needs at least one Re-nag".into(),
            ));
        }
        let value = after.map(|a| a.to_string());
        self.store
            .append(id, keys::ESCALATION, value.as_deref(), None)
            .map(drop)
    }

    /// Moves the Routine `id` to the trash: it leaves the Agenda, fires no
    /// more and is never Missed. Its Note stays.
    pub fn trash(&self, id: &str) -> Result<()> {
        self.require(id)?;
        let at = dart_iso(self.store.now());
        self.store
            .append(id, keys::TRASHED, Some(&at), None)
            .map(drop)
    }

    /// Brings the Routine `id` back from the trash.
    pub fn restore(&self, id: &str) -> Result<()> {
        let routine = self.by_id(id)?.filter(Routine::is_trashed);
        let Some(routine) = routine else {
            return Err(Error::Invalid(format!("not a Routine in the trash: {id}")));
        };
        self.store.batch(|| self.untrash(&routine))
    }

    /// Moves the Note `note_id` to the trash, and with `with_routines` the
    /// Routines and Reminders it spawned too: they come back with it.
    /// Without, they stay and link back to a Note in the trash.
    pub fn trash_note(&self, note_id: &str, with_routines: bool) -> Result<()> {
        let routines = if with_routines {
            self.of_note(note_id)?
        } else {
            Vec::new()
        };
        self.store.batch(|| {
            Notes::new(self.store).trash(note_id)?;
            // The same moment as the Note's, so both are purged together.
            let at = match self.store.current(note_id, note_keys::TRASHED)? {
                Some(at) => at,
                None => dart_iso(self.store.now()),
            };
            for routine in &routines {
                self.store
                    .append(&routine.id, keys::TRASHED, Some(&at), None)?;
                self.store
                    .append(&routine.id, keys::TRASHED_WITH, Some(note_id), None)?;
            }
            Ok(())
        })
    }

    /// Brings the Note `note_id` back from the trash, with the Routines
    /// and Reminders that went there with it.
    pub fn restore_note(&self, note_id: &str) -> Result<()> {
        self.store.batch(|| {
            Notes::new(self.store).restore(note_id)?;
            for id in self.store.entities_of_type(keys::TYPE_ROUTINE)? {
                if let Some(routine) = self.by_id(&id)?
                    && routine.trashed_with.as_deref() == Some(note_id)
                {
                    self.untrash(&routine)?;
                }
            }
            Ok(())
        })
    }

    /// When the trashed `routine` is purged; `None` when it is not in the
    /// trash.
    pub fn purge_at(routine: &Routine) -> Option<DateTime<Utc>> {
        routine.trashed.map(|t| t + Notes::trash_keep())
    }

    /// The Routines and Reminders trashed on their own that can still be
    /// restored, most recently deleted first.
    pub fn trashed(&self) -> Result<Vec<Routine>> {
        let now = self.store.now();
        let mut routines = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_ROUTINE)? {
            if let Some(routine) = self.by_id(&id)?
                && routine.trashed_with.is_none()
                && Self::purge_at(&routine).is_some_and(|at| now < at)
            {
                routines.push(routine);
            }
        }
        routines.sort_by_key(|r| std::cmp::Reverse(r.trashed));
        Ok(routines)
    }

    /// Checks off `occurrence` of the Routine `id` as Done, at `at` or the
    /// store's now. Early or late, the schedule stays as it is.
    pub fn mark_done(
        &self,
        id: &str,
        occurrence: &Occurrence,
        at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let routine = self.require(id)?;
        if !routine.schedule.has(occurrence) {
            return Err(Error::Invalid(format!("not due then: {occurrence}")));
        }
        let value = dart_iso(at.unwrap_or_else(|| self.store.now()));
        self.store
            .append(id, &keys::done(occurrence), Some(&value), None)
            .map(drop)
    }

    /// Unchecks `occurrence` of the Routine `id` again.
    pub fn unmark_done(&self, id: &str, occurrence: &Occurrence) -> Result<()> {
        self.require(id)?;
        self.store
            .append(id, &keys::done(occurrence), None, None)
            .map(drop)
    }

    /// The Routine `id`, trashed or not, or `None` when there is none.
    pub fn by_id(&self, id: &str) -> Result<Option<Routine>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_ROUTINE) {
            return Ok(None);
        }
        let (Some(note_id), Some(schedule)) = (fields.get(keys::NOTE), Self::schedule_of(&fields))
        else {
            return Ok(None);
        };
        let renag = fields
            .get(keys::RENAG)
            .and_then(|r| r.parse::<i64>().ok())
            .filter(|r| *r >= 1)
            .map(Duration::minutes);
        let escalation = fields
            .get(keys::ESCALATION)
            .and_then(|e| e.parse::<i64>().ok())
            .filter(|e| *e >= 1);
        let trashed = fields.get(keys::TRASHED).map(parse_time).transpose()?;
        let mut scheduled_at: Option<DateTime<Utc>> = None;
        for entry in self.store.history(id)? {
            // Leaving the trash bounds what was asked for like a new
            // schedule.
            if !keys::SCHEDULE_FIELDS.contains(&entry.field.as_str())
                && entry.field != keys::TRASHED
            {
                continue;
            }
            if scheduled_at.is_none_or(|s| entry.date_at > s) {
                scheduled_at = Some(entry.date_at);
            }
        }
        let mut done = BTreeMap::new();
        for (key, value) in fields.iter() {
            if let (Some(occurrence), Some(value)) = (key.strip_prefix(keys::DONE_PREFIX), value) {
                done.insert(occurrence.to_string(), parse_time(value)?);
            }
        }
        Ok(Some(Routine {
            id: id.to_string(),
            note_id: note_id.to_string(),
            schedule,
            done,
            renag,
            escalation,
            scheduled_at,
            trashed,
            trashed_with: trashed
                .and(fields.get(keys::TRASHED_WITH))
                .map(str::to_string),
        }))
    }

    /// The schedule the Routine `fields` hold, or `None` when they hold
    /// none.
    pub fn schedule_of(fields: &Fields) -> Option<Schedule> {
        let start = Day::parse(fields.get(keys::START).unwrap_or_default())?;
        let rule = match fields.get(keys::RULE) {
            None => None,
            Some(text) => Some(Recurrence::parse(text).ok()?),
        };
        let times: Vec<ClockTime> = fields
            .get(keys::TIMES)
            .unwrap_or_default()
            .split(',')
            .filter_map(ClockTime::parse)
            .collect();
        Schedule::new(start, &times, rule)
    }

    /// Every Routine and Reminder not in the trash.
    pub fn all(&self) -> Result<Vec<Routine>> {
        let mut out = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_ROUTINE)? {
            if let Some(routine) = self.by_id(&id)?
                && !routine.is_trashed()
            {
                out.push(routine);
            }
        }
        Ok(out)
    }

    /// The Note `routine` links back to: the Note it was spawned from, or
    /// the one that Note was merged into; it may be in the trash. `None`
    /// when it is gone for good.
    pub fn note(&self, routine: &Routine) -> Result<Option<Note>> {
        let notes = Notes::new(self.store);
        let note = notes.by_id(&routine.note_id)?;
        match note.as_ref().and_then(|n| n.merged_into.clone()) {
            None => Ok(note),
            Some(into) => notes.by_id(&into),
        }
    }

    /// Every Routine and Reminder with the Note it links back to.
    pub fn all_with_notes(&self) -> Result<Vec<(Routine, Option<Note>)>> {
        let mut out = Vec::new();
        for routine in self.all()? {
            let note = self.note(&routine)?;
            out.push((routine, note));
        }
        Ok(out)
    }

    /// The Routines and Reminders the Note `note_id` spawned, those of the
    /// Notes merged into it included, without those in the trash.
    pub fn of_note(&self, note_id: &str) -> Result<Vec<Routine>> {
        let mut out = Vec::new();
        for routine in self.all()? {
            if self.note(&routine)?.is_some_and(|n| n.id == note_id) {
                out.push(routine);
            }
        }
        Ok(out)
    }

    /// The Agenda at the store's now, with days as `wall` shows them.
    pub fn agenda(&self, wall: &WallClock) -> Result<Agenda> {
        let today = Day::today(self.store.now(), wall);
        let routines = self.all_with_notes()?;
        let rows_on = |day: Day| {
            let mut rows: Vec<AgendaRow> = routines
                .iter()
                .flat_map(|(routine, note)| {
                    routine
                        .schedule
                        .on(day)
                        .into_iter()
                        .map(|occurrence| AgendaRow {
                            routine: routine.clone(),
                            occurrence,
                            note: note.clone(),
                        })
                })
                .collect();
            rows.sort_by(agenda_order);
            rows
        };
        Ok(Agenda {
            today,
            today_rows: rows_on(today),
            tomorrow_rows: rows_on(today.add_days(1)),
        })
    }

    /// Writes the fields of `schedule` that differ from what is there.
    pub(crate) fn write_schedule(&self, id: &str, schedule: &Schedule) -> Result<()> {
        let current = self.store.current_fields(id)?;
        let write = |field: &str, value: Option<&str>| -> Result<()> {
            let held = current.iter().find(|(k, _)| *k == field);
            if held.is_some_and(|(_, v)| v == value) {
                return Ok(());
            }
            self.store.append(id, field, value, None).map(drop)
        };
        let times: Vec<String> = schedule.times.iter().map(ToString::to_string).collect();
        write(keys::START, Some(&schedule.start.to_string()))?;
        write(keys::TIMES, Some(&times.join(",")))?;
        let rule = schedule.rule.as_ref().map(Recurrence::format);
        write(keys::RULE, rule.as_deref())
    }

    fn untrash(&self, routine: &Routine) -> Result<()> {
        self.store.append(&routine.id, keys::TRASHED, None, None)?;
        if routine.trashed_with.is_some() {
            self.store
                .append(&routine.id, keys::TRASHED_WITH, None, None)?;
        }
        Ok(())
    }

    /// The Routine `id`, which must not be in the trash.
    fn require(&self, id: &str) -> Result<Routine> {
        self.by_id(id)?
            .filter(|r| !r.is_trashed())
            .ok_or_else(|| Error::Invalid(format!("not a Routine: {id}")))
    }
}

/// The Agenda's order: by Occurrence, then by the Note's text, then by
/// id, as the Dart core sorts.
fn agenda_order(a: &AgendaRow, b: &AgendaRow) -> Ordering {
    let text = |r: &AgendaRow| r.note.as_ref().map(|n| n.text.clone()).unwrap_or_default();
    a.occurrence
        .cmp(&b.occurrence)
        .then_with(|| utf16_cmp(&text(a), &text(b)))
        .then_with(|| utf16_cmp(&a.routine.id, &b.routine.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Frequency;

    fn daily_at_eight() -> Schedule {
        let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
        let rule = Recurrence::every(Frequency::Daily, 1);
        match Schedule::new(Day::new(2026, 1, 1), &eight, Some(rule)) {
            Some(s) => s,
            None => panic!("a schedule"),
        }
    }

    #[test]
    fn a_spawned_routine_takes_its_note_out_of_unsorted() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let note = notes.capture("Take the pills", None, None)?;
        assert!(note.is_unsorted());
        let routines = Routines::new(&store);
        let routine = routines.spawn(&note.id, &daily_at_eight())?;
        assert!(!routine.is_reminder());
        assert_eq!(notes.unsorted()?, vec![]);
        assert_eq!(routines.of_note(&note.id)?, vec![routine.clone()]);
        // The same schedule again writes nothing.
        let before = store.all_entries()?.len();
        routines.reschedule(&routine.id, &daily_at_eight())?;
        assert_eq!(store.all_entries()?.len(), before);
        Ok(())
    }

    #[test]
    fn done_and_its_undoing_and_the_trash() -> Result<()> {
        let store = crate::store::tests::store();
        let routines = Routines::new(&store);
        let routine = routines.create("  Water  ", &daily_at_eight())?;
        let note = routines.note(&routine)?;
        assert_eq!(note.map(|n| n.text), Some("Water".to_string()));
        let o = Occurrence::new(Day::new(2026, 1, 2), ClockTime::new(8, 0));
        routines.mark_done(&routine.id, &o, None)?;
        assert!(
            routines
                .by_id(&routine.id)?
                .is_some_and(|r| r.done_at(&o).is_some())
        );
        let not_due = Occurrence::new(Day::new(2026, 1, 2), ClockTime::new(9, 0));
        assert!(routines.mark_done(&routine.id, &not_due, None).is_err());
        routines.unmark_done(&routine.id, &o)?;
        assert!(
            routines
                .by_id(&routine.id)?
                .is_some_and(|r| r.done_at(&o).is_none())
        );
        routines.trash(&routine.id)?;
        assert!(routines.all()?.is_empty());
        assert_eq!(routines.trashed()?.len(), 1);
        routines.restore(&routine.id)?;
        assert_eq!(routines.all()?.len(), 1);
        assert!(
            routines
                .set_renag(&routine.id, Some(Duration::seconds(5)))
                .is_err()
        );
        routines.set_renag(&routine.id, Some(Duration::minutes(5)))?;
        routines.set_escalation(&routine.id, Some(3))?;
        let read = routines.by_id(&routine.id)?;
        assert_eq!(
            read.as_ref().and_then(|r| r.renag),
            Some(Duration::minutes(5))
        );
        assert_eq!(read.and_then(|r| r.escalation), Some(3));
        Ok(())
    }

    #[test]
    fn a_note_trashed_with_its_routines_brings_them_back() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let routines = Routines::new(&store);
        let note = notes.capture("Walk the dog", None, None)?;
        routines.spawn(&note.id, &daily_at_eight())?;
        routines.trash_note(&note.id, true)?;
        assert!(routines.all()?.is_empty());
        // Trashed with the Note, they come back with it, not on their own.
        assert!(routines.trashed()?.is_empty());
        routines.restore_note(&note.id)?;
        assert_eq!(routines.all()?.len(), 1);
        routines.trash_note(&note.id, false)?;
        assert_eq!(routines.all()?.len(), 1);
        Ok(())
    }
}
