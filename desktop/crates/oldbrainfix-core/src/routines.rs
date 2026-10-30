//! Routines and Reminders as the Dart core's `routines.dart` reads and
//! writes them: spawned from a Note (ADR 0005), on a fixed schedule (ADR
//! 0006), each Occurrence checked off on its own, with a Re-nag interval
//! and Escalation of their own, and a trash. Every write appends the same
//! entries, in the same order and read from the clock as often, as the
//! Dart core's.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::checklists::{LineChecks, keys as checklist_keys};
use crate::entry::{dart_iso, parse_time};
use crate::notes::{Note, Notes, keys as note_keys};
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule, WallClock};
use crate::sessions::{SessionMarks, keys as session_keys};
use crate::store::{Fields, Store, utf16_cmp};
use crate::transcript_prompts::{dart_trim, lower};
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
    /// The version of the schedule's structure (ADR 0034): version 1
    /// holds [`START`], [`TIMES`] and [`RULE`]; it is the default and never
    /// written, so it is absent.
    pub const SCHEDULE_VERSION: &str = "scheduleVersion";
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
    pub const SCHEDULE_FIELDS: [&str; 4] = [START, TIMES, RULE, SCHEDULE_VERSION];

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

/// A Routine or Reminder whose schedule this version cannot read, as the
/// Dart core's `NewerRoutine`: one of a later structure, or with a part
/// it does not know (ADR 0034). It is kept as it is, fires nothing here
/// and nothing here changes it; the apps show that a newer version of
/// the app is needed for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewerRoutine {
    /// Entity id, `routine:<id>`.
    pub id: String,
    /// The Note it was spawned from, as stored.
    pub note_id: String,
}

/// `routines` in alphabetical order of their names, as the list of all
/// Routines and Reminders shows them; `name_of` gives the title the app
/// shows for one. A name counts by its first line, trimmed, in lower
/// case, compared by UTF-16 code units, as the Dart core's
/// `routinesByName`; equal names go by id, so the order is the same
/// every time and on every Device.
pub fn routines_by_name(
    routines: impl IntoIterator<Item = Routine>,
    mut name_of: impl FnMut(&Routine) -> String,
) -> Vec<Routine> {
    let mut keyed: Vec<(Routine, String)> = routines
        .into_iter()
        .map(|routine| {
            let key = name_key(&name_of(&routine));
            (routine, key)
        })
        .collect();
    keyed.sort_by(|a, b| utf16_cmp(&a.1, &b.1).then_with(|| utf16_cmp(&a.0.id, &b.0.id)));
    keyed.into_iter().map(|(routine, _)| routine).collect()
}

/// The first line of `name`, trimmed and in lower case as Dart's `trim`
/// and `toLowerCase` have it.
fn name_key(name: &str) -> String {
    let first = dart_trim(name).split('\n').next().unwrap_or_default();
    lower(dart_trim(first))
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
/// and then by time, tomorrow's the same way, and those of the days
/// after when asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agenda {
    pub today: Day,
    pub today_rows: Vec<AgendaRow>,
    pub tomorrow_rows: Vec<AgendaRow>,
    /// The rows of each day after tomorrow, one list per day, in order.
    pub later_rows: Vec<Vec<AgendaRow>>,
}

impl Agenda {
    /// The day of `later_rows[index]`.
    pub fn later(&self, index: usize) -> Day {
        self.today.add_days(2 + index as i64)
    }
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
        self.spawn_with(note_id, schedule, |_| Ok(()))
    }

    /// [`Routines::spawn`], with `within` writing more of it in the same
    /// batch, given its id.
    pub fn spawn_with(
        &self,
        note_id: &str,
        schedule: &Schedule,
        within: impl FnOnce(&str) -> Result<()>,
    ) -> Result<Routine> {
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
            within(&id)
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
        let history = self.store.history(id)?;
        for entry in &history {
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
        // A Session's Occurrence is Done too once every pick has an
        // entry (ADR 0029); a check-off, written below, keeps its moment.
        let mut done = if fields.get(session_keys::POOL).is_some() {
            SessionMarks::read(self.store, id, &history)?.done_from_entries(self.store)?
        } else {
            BTreeMap::new()
        };
        // Spawned from a Checklist, once every Line is checked for it
        // (ADR 0030).
        if fields
            .iter()
            .any(|(key, _)| key.starts_with(checklist_keys::CHECK_PREFIX))
        {
            let mut notes = vec![note_id.to_string()];
            if let Some(into) = Notes::new(self.store)
                .by_id(note_id)?
                .and_then(|n| n.merged_into)
            {
                notes.push(into);
            }
            done.extend(LineChecks::read(self.store, id, &history)?.done(self.store, &notes)?);
        }
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
    /// none or one this version cannot read ([`Routines::schedule_is_newer`]).
    pub fn schedule_of(fields: &Fields) -> Option<Schedule> {
        if !known_schedule_version(fields) {
            return None;
        }
        let start = Day::parse(fields.get(keys::START).unwrap_or_default())?;
        let rule = match fields.get(keys::RULE) {
            None => None,
            Some(text) => Some(Recurrence::parse(text).ok()?),
        };
        let text = fields.get(keys::TIMES).unwrap_or_default();
        // Empty for all day; a time this version cannot read is a part it
        // does not know, not one to leave out.
        let times: Vec<ClockTime> = if text.is_empty() {
            Vec::new()
        } else {
            text.split(',')
                .map(ClockTime::parse)
                .collect::<Option<_>>()?
        };
        Schedule::new(start, &times, rule)
    }

    /// Whether the Routine `fields` hold a schedule this version cannot
    /// read (ADR 0034): of a later structure, its
    /// [`keys::SCHEDULE_VERSION`] anything but absent or `1`, or with a
    /// start, times or rule it does not know. Fields without a start or a
    /// version hold no schedule.
    pub fn schedule_is_newer(fields: &Fields) -> bool {
        Self::schedule_of(fields).is_none()
            && (!known_schedule_version(fields) || fields.get(keys::START).is_some())
    }

    /// The Routine `id` when its schedule needs a newer version of the
    /// app ([`Routines::schedule_is_newer`]), trashed or not.
    pub fn newer_by_id(&self, id: &str) -> Result<Option<NewerRoutine>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_ROUTINE) {
            return Ok(None);
        }
        let Some(note_id) = fields.get(keys::NOTE) else {
            return Ok(None);
        };
        if !Self::schedule_is_newer(&fields) {
            return Ok(None);
        }
        Ok(Some(NewerRoutine {
            id: id.to_string(),
            note_id: note_id.to_string(),
        }))
    }

    /// Every Routine and Reminder not in the trash whose schedule needs a
    /// newer version of the app, by id: [`Routines::all`] leaves them out.
    pub fn newer(&self) -> Result<Vec<NewerRoutine>> {
        let mut ids = self.store.entities_of_type(keys::TYPE_ROUTINE)?;
        ids.sort();
        let mut out = Vec::new();
        for id in ids {
            if let Some(routine) = self.newer_by_id(&id)?
                && self.store.current(&id, keys::TRASHED)?.is_none()
            {
                out.push(routine);
            }
        }
        Ok(out)
    }

    /// The Routines and Reminders of [`Routines::newer`] the Note
    /// `note_id` spawned, those of the Notes merged into it included.
    pub fn newer_of_note(&self, note_id: &str) -> Result<Vec<NewerRoutine>> {
        let mut out = Vec::new();
        for routine in self.newer()? {
            if self
                .note_of_newer(&routine)?
                .is_some_and(|n| n.id == note_id)
            {
                out.push(routine);
            }
        }
        Ok(out)
    }

    /// The Note `routine` shows under, as [`Routines::note`] finds it for
    /// a Routine.
    pub fn note_of_newer(&self, routine: &NewerRoutine) -> Result<Option<Note>> {
        let notes = Notes::new(self.store);
        let note = notes.by_id(&routine.note_id)?;
        match note.as_ref().and_then(|n| n.merged_into.clone()) {
            None => Ok(note),
            Some(into) => notes.by_id(&into),
        }
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

    /// The Agenda at the store's now, with days as `wall` shows them:
    /// today and tomorrow.
    pub fn agenda(&self, wall: &WallClock) -> Result<Agenda> {
        self.agenda_over(wall, 2)
    }

    /// The Agenda at the store's now, with days as `wall` shows them,
    /// over `days` days from today; never fewer than today and tomorrow.
    pub fn agenda_over(&self, wall: &WallClock, days: usize) -> Result<Agenda> {
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
            later_rows: (2..days)
                .map(|i| rows_on(today.add_days(i as i64)))
                .collect(),
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
/// Whether the schedule version the Routine `fields` hold is 1: absent
/// or `1` exactly.
fn known_schedule_version(fields: &Fields) -> bool {
    matches!(fields.get(keys::SCHEDULE_VERSION), None | Some("1"))
}

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

    fn fields(pairs: &[(&str, &str)]) -> Fields {
        Fields::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), Some(v.to_string())))
                .collect(),
        )
    }

    #[test]
    fn a_schedule_reads_as_newer_by_its_version_or_a_part_not_known() {
        let daily = [
            (keys::START, "2026-10-24"),
            (keys::TIMES, "08:00"),
            (keys::RULE, "FREQ=DAILY"),
        ];
        let with = |key: &'static str, value: &'static str| {
            let mut out: Vec<(&str, &str)> =
                daily.iter().filter(|(k, _)| *k != key).copied().collect();
            out.push((key, value));
            out
        };
        for known in [
            daily.to_vec(),
            with(keys::SCHEDULE_VERSION, "1"),
            with(keys::TIMES, ""),
        ] {
            let f = fields(&known);
            assert!(Routines::schedule_of(&f).is_some(), "{known:?}");
            assert!(!Routines::schedule_is_newer(&f), "{known:?}");
        }
        for newer in [
            with(keys::SCHEDULE_VERSION, "2"),
            with(keys::SCHEDULE_VERSION, "01"),
            with(keys::SCHEDULE_VERSION, " 1"),
            with(keys::SCHEDULE_VERSION, "1.0"),
            with(keys::SCHEDULE_VERSION, ""),
            vec![(keys::SCHEDULE_VERSION, "2")],
            with(keys::TIMES, "08:00,8pm"),
            with(keys::TIMES, "08:00,"),
            with(keys::RULE, "FREQ=DAILY;BYEASTER=1"),
            with(keys::START, "2026-13-01"),
        ] {
            let f = fields(&newer);
            assert!(Routines::schedule_of(&f).is_none(), "{newer:?}");
            assert!(Routines::schedule_is_newer(&f), "{newer:?}");
        }
        for none in [
            vec![],
            vec![(keys::TIMES, "08:00"), (keys::RULE, "FREQ=DAILY")],
            vec![(keys::SCHEDULE_VERSION, "1")],
        ] {
            assert!(!Routines::schedule_is_newer(&fields(&none)), "{none:?}");
        }
    }

    #[test]
    fn a_routine_of_a_later_version_fires_nothing_and_nothing_here_changes_it() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let routines = Routines::new(&store);
        let pills = notes.capture("pills", None, None)?;
        let id = "routine:later";
        store.batch(|| {
            store.append(id, keys::TYPE, Some(keys::TYPE_ROUTINE), None)?;
            store.append(id, keys::NOTE, Some(&pills.id), None)?;
            store.append(id, keys::START, Some("2026-01-01"), None)?;
            store.append(id, keys::TIMES, Some("08:00"), None)?;
            store.append(id, keys::RULE, Some("FREQ=DAILY"), None)?;
            store.append(id, keys::SCHEDULE_VERSION, Some("2"), None)?;
            Ok(())
        })?;
        assert_eq!(routines.by_id(id)?, None);
        assert!(routines.all()?.is_empty());
        let newer = NewerRoutine {
            id: id.to_string(),
            note_id: pills.id.clone(),
        };
        assert_eq!(routines.newer()?, vec![newer.clone()]);
        assert_eq!(routines.newer_of_note(&pills.id)?, vec![newer]);
        let before = store.history(id)?.len();
        let occurrence = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
        assert!(routines.reschedule(id, &daily_at_eight()).is_err());
        assert!(routines.set_renag(id, Some(Duration::minutes(5))).is_err());
        assert!(routines.set_escalation(id, Some(2)).is_err());
        assert!(routines.trash(id).is_err());
        assert!(routines.mark_done(id, &occurrence, None).is_err());
        assert!(routines.unmark_done(id, &occurrence).is_err());
        assert_eq!(store.history(id)?.len(), before);
        // Merged into another Note, it shows under that one.
        let bx = notes.capture("pill box", None, None)?;
        notes.merge(&pills.id, &bx.id)?;
        assert_eq!(routines.newer_of_note(&bx.id)?.len(), 1);
        assert!(routines.newer_of_note(&pills.id)?.is_empty());
        // In the trash, by a newer version, it is listed no more.
        store.append(id, keys::TRASHED, Some("2026-01-01T10:00:00.000Z"), None)?;
        assert!(routines.newer()?.is_empty());
        Ok(())
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

    #[test]
    fn over_a_week_the_days_after_tomorrow_cross_a_month_end() -> Result<()> {
        let now = parse_time("2026-10-28T11:00:00.000Z")?;
        let store = Store::in_memory(Box::new(move || now), || {
            Ok(crate::signing::SigningKey::from_seed([1; 32]))
        })?;
        let routines = Routines::new(&store);
        routines.create("pills", &daily_at_eight())?;
        let Some(once) = Schedule::new(Day::new(2026, 11, 2), &[], None) else {
            panic!("a schedule");
        };
        routines.create("dentist", &once)?;
        let wall = |at: DateTime<Utc>| at.naive_utc();
        assert!(routines.agenda(&wall)?.later_rows.is_empty());
        let agenda = routines.agenda_over(&wall, 7)?;
        assert_eq!(agenda.today, Day::new(2026, 10, 28));
        assert_eq!(agenda.later_rows.len(), 5);
        let days: Vec<String> = (0..5).map(|i| agenda.later(i).to_string()).collect();
        assert_eq!(
            days,
            [
                "2026-10-30",
                "2026-10-31",
                "2026-11-01",
                "2026-11-02",
                "2026-11-03"
            ]
        );
        let texts = |rows: &[AgendaRow]| -> Vec<String> {
            rows.iter()
                .map(|r| r.note.as_ref().map_or(String::new(), |n| n.text.clone()))
                .collect()
        };
        assert_eq!(texts(&agenda.later_rows[1]), ["pills"]);
        // All-day first, then by time.
        assert_eq!(texts(&agenda.later_rows[3]), ["dentist", "pills"]);
        Ok(())
    }

    /// The ids of `names`, given in this order, listed by name.
    fn named_order(names: &[(&str, &str)]) -> Vec<String> {
        let given = names.iter().map(|(id, _)| Routine {
            id: id.to_string(),
            note_id: format!("note:{id}"),
            schedule: daily_at_eight(),
            done: BTreeMap::new(),
            renag: None,
            escalation: None,
            scheduled_at: None,
            trashed: None,
            trashed_with: None,
        });
        let name_of = |r: &Routine| {
            names
                .iter()
                .find(|(id, _)| *id == r.id)
                .map(|(_, name)| name.to_string())
                .unwrap_or_default()
        };
        routines_by_name(given, name_of)
            .into_iter()
            .map(|r| r.id)
            .collect()
    }

    #[test]
    fn by_name_alphabetical_whatever_the_case() {
        let order = named_order(&[
            ("routine:1", "Cherry"),
            ("routine:2", "apple"),
            ("routine:3", "Banana"),
        ]);
        assert_eq!(order, ["routine:2", "routine:3", "routine:1"]);
    }

    #[test]
    fn by_name_equal_names_go_by_id() {
        let order = named_order(&[
            ("routine:b", "Pills"),
            ("routine:a", "pills"),
            ("routine:c", "PILLS"),
        ]);
        assert_eq!(order, ["routine:a", "routine:b", "routine:c"]);
    }

    #[test]
    fn by_name_a_name_counts_by_its_first_line_trimmed() {
        let order = named_order(&[
            ("routine:1", "  Zebra"),
            ("routine:2", "Mango\nAardvark"),
            ("routine:3", "\u{feff}\n Kiwi "),
        ]);
        assert_eq!(order, ["routine:3", "routine:2", "routine:1"]);
    }

    #[test]
    fn by_name_lowers_as_dart_does_and_compares_utf16() {
        // Dart lowers ΣΑΣ to σασ, with no final sigma, so σας comes first;
        // U+1F34E comes before U+FF41 in UTF-16, not in code points.
        let order = named_order(&[
            ("routine:1", "ΣΑΣ"),
            ("routine:2", "σας"),
            ("routine:3", "\u{ff21}"),
            ("routine:4", "\u{1f34e}"),
            ("routine:5", "Ärzte"),
            ("routine:6", "zoo"),
        ]);
        assert_eq!(
            order,
            [
                "routine:6",
                "routine:5",
                "routine:2",
                "routine:1",
                "routine:4",
                "routine:3"
            ]
        );
    }
}
