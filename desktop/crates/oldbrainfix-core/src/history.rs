//! A Routine's history (CONTEXT.md) as the Dart core's `history.dart`
//! has it: each Occurrence Done, Missed, skipped or still open, its
//! Streak and its Adherence, as a pure function of the schedules it had,
//! what was Done and skipped, this Device's snoozes, now and the wall
//! clock. Missed follows the same
//! rules as the Missed Summary.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, Utc};

use crate::Result;
use crate::alarms::{Alarms, instant_of, missed_from, start_of};
use crate::entry::Entry;
use crate::routines::{
    Routine, Routines, first_occurrence_from, keys, moved_occurrences, occurrences_between,
};
use crate::schedule::{ClockTime, Day, Occurrence, Schedule, WallClock};
use crate::settings::DeviceSettings;
use crate::store::{Fields, Store};

/// Monday, the day weeks start on unless the app says otherwise.
pub const MONDAY: u32 = 1;

/// A stretch of time in which a Routine had `schedule` and was out of
/// the trash: what fell due in it was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleSpan {
    pub schedule: Schedule,
    /// When the schedule was written, or the Routine left the trash.
    pub from: DateTime<Utc>,
    /// When the schedule was changed or the Routine went to the trash;
    /// `None` while it still holds.
    pub until: Option<DateTime<Utc>>,
}

/// The [`ScheduleSpan`]s a Routine's log `entries` tell, oldest first.
/// Saving a schedule writes its fields one after the other, a moment
/// apart: a state that stood for less than a minute never stood, as no
/// Occurrence can fall in it.
pub fn schedule_spans(entries: &[Entry]) -> Vec<ScheduleSpan> {
    let mut changes: Vec<&Entry> = entries
        .iter()
        .filter(|e| keys::SCHEDULE_FIELDS.contains(&e.field.as_str()) || e.field == keys::TRASHED)
        .collect();
    changes.sort_by(|a, b| Store::latest_wins(a, b));
    let mut fields: Vec<(String, Option<String>)> = Vec::new();
    let mut states: Vec<(DateTime<Utc>, Option<Schedule>)> = Vec::new();
    for entry in changes {
        match fields.iter_mut().find(|(field, _)| *field == entry.field) {
            Some((_, value)) => value.clone_from(&entry.value),
            None => fields.push((entry.field.clone(), entry.value.clone())),
        }
        let current = Fields::new(fields.clone());
        let trashed = current.get(keys::TRASHED).is_some();
        let schedule = if trashed {
            None
        } else {
            Routines::schedule_of(&current)
        };
        states.push((entry.date_at, schedule));
    }
    let settled = Duration::minutes(1);
    let mut spans = Vec::new();
    for (i, (from, schedule)) in states.iter().enumerate() {
        let Some(schedule) = schedule else {
            continue;
        };
        match states.get(i + 1) {
            None => spans.push(ScheduleSpan {
                schedule: schedule.clone(),
                from: *from,
                until: None,
            }),
            Some((next, _)) if *next - *from >= settled => spans.push(ScheduleSpan {
                schedule: schedule.clone(),
                from: *from,
                until: Some(*next),
            }),
            Some(_) => {}
        }
    }
    spans
}

/// Where an Occurrence of a Routine stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OccurrenceStatus {
    /// Checked off.
    Done,
    /// Never Done, and its time is over (CONTEXT.md).
    Missed,
    /// Skipped, by Skip or Tomorrow, and not Done (CONTEXT.md, Skip).
    Skipped,
    /// Due, not Done, not Missed yet.
    Open,
    /// Not due yet.
    Coming,
}

impl OccurrenceStatus {
    /// The Dart core's name for it.
    pub fn name(self) -> &'static str {
        match self {
            OccurrenceStatus::Done => "done",
            OccurrenceStatus::Missed => "missed",
            OccurrenceStatus::Skipped => "skipped",
            OccurrenceStatus::Open => "open",
            OccurrenceStatus::Coming => "coming",
        }
    }
}

/// When a Done came, against the Occurrence's time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoneTiming {
    /// Before the Occurrence started.
    Early,
    /// While it was due.
    OnTime,
    /// After it was Missed: checked off late.
    Late,
}

impl DoneTiming {
    /// The Dart core's name for it.
    pub fn name(self) -> &'static str {
        match self {
            DoneTiming::Early => "early",
            DoneTiming::OnTime => "onTime",
            DoneTiming::Late => "late",
        }
    }
}

/// One Occurrence in a Routine's history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub occurrence: Occurrence,
    pub status: OccurrenceStatus,
    /// When it starts, as an instant.
    pub start: DateTime<Utc>,
    /// When it counts as Missed unless Done, as an instant.
    pub missed_at: DateTime<Utc>,
    /// When it was checked off; `None` unless Done.
    pub done_at: Option<DateTime<Utc>>,
    /// Until when it was last snoozed on this Device, if this Device still
    /// knows; snoozes are never synced (ADR 0008).
    pub snoozed_until: Option<DateTime<Utc>>,
    /// When it was skipped; `None` unless it was.
    pub skipped_at: Option<DateTime<Utc>>,
    /// Whether Tomorrow added it, a day after the one it skipped.
    pub moved: bool,
}

impl HistoryEntry {
    /// Whether it was Done early, on time or late; `None` unless Done.
    /// An all-day one is due all its day, before it rings too (#201), so
    /// it is never early.
    pub fn timing(&self) -> Option<DoneTiming> {
        let done = self.done_at?;
        Some(if self.occurrence.time.is_some() && done < self.start {
            DoneTiming::Early
        } else if done < self.missed_at {
            DoneTiming::OnTime
        } else {
            DoneTiming::Late
        })
    }
}

/// The Done and Missed Occurrences over a period; those still open do
/// not count, nor do those skipped, which are counted beside them
/// ("92 % this month · 3 skipped").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Adherence {
    pub done: usize,
    pub missed: usize,
    /// The Occurrences skipped and not Done: never part of the share.
    pub skipped: usize,
}

impl Adherence {
    /// The share Done, from 0 to 1; `None` when nothing counts yet.
    pub fn share(&self) -> Option<f64> {
        let counted = self.done + self.missed;
        (counted > 0).then(|| self.done as f64 / counted as f64)
    }
}

/// What one day of a Routine came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayTally {
    pub day: Day,
    pub done: usize,
    pub missed: usize,
    /// Not Done and not Missed yet.
    pub open: usize,
    /// Skipped and not Done.
    pub skipped: usize,
}

impl DayTally {
    /// Whether nothing was due that day.
    pub fn is_empty(&self) -> bool {
        self.done + self.missed + self.open + self.skipped == 0
    }
}

/// A Routine's history up to the end of today.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutineHistory {
    /// Every Occurrence that was asked for or Done, oldest first. One that
    /// fell due while the Routine was in the trash, or under a schedule
    /// changed before it was Missed, was never asked for and is left out,
    /// unless it was Done.
    pub entries: Vec<HistoryEntry>,
    pub today: Day,
    /// The day weeks start on, 1 for Monday to 7 for Sunday.
    pub week_start: u32,
    /// The run of Done Occurrences up to the latest that is no longer
    /// open (CONTEXT.md, Streak): one still due does not break it, nor
    /// does one skipped, which does not count in it either.
    pub current_streak: usize,
    /// The longest run of Done Occurrences ever.
    pub longest_streak: usize,
    /// The last seven days, today last.
    pub last_seven_days: Vec<DayTally>,
    /// Since the week began, on `week_start`.
    pub this_week: Adherence,
    /// Since the 1st of the month.
    pub this_month: Adherence,
    /// The last 30 days, today included.
    pub last_30_days: Adherence,
}

impl RoutineHistory {
    /// Works out the Streaks, the last seven days and the Adherence
    /// figures once, up front.
    pub fn new(entries: Vec<HistoryEntry>, today: Day, week_start: u32) -> RoutineHistory {
        let mut history = RoutineHistory {
            entries,
            today,
            week_start,
            current_streak: 0,
            longest_streak: 0,
            last_seven_days: Vec::new(),
            this_week: Adherence::default(),
            this_month: Adherence::default(),
            last_30_days: Adherence::default(),
        };
        history.current_streak = history.find_current_streak();
        history.longest_streak = history.find_longest_streak();
        history.last_seven_days = (0..=6)
            .rev()
            .map(|i| history.tally(today.add_days(-i)))
            .collect();
        let into_week = (i64::from(today.weekday()) - i64::from(week_start)).rem_euclid(7);
        history.this_week = history.adherence(today.add_days(-into_week), today);
        let first = Day::new(today.year, i64::from(today.month), 1);
        history.this_month = history.adherence(first, today);
        history.last_30_days = history.adherence(today.add_days(-29), today);
        history
    }

    /// The entries from `from` to `to`, both days included.
    pub fn between(&self, from: Day, to: Day) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.occurrence.day >= from && e.occurrence.day <= to)
            .collect()
    }

    fn find_current_streak(&self) -> usize {
        let mut run = 0;
        for entry in self.entries.iter().rev() {
            match entry.status {
                OccurrenceStatus::Done => run += 1,
                OccurrenceStatus::Missed => return run,
                OccurrenceStatus::Skipped | OccurrenceStatus::Open | OccurrenceStatus::Coming => {}
            }
        }
        run
    }

    fn find_longest_streak(&self) -> usize {
        let mut run = 0;
        let mut longest = 0;
        for entry in &self.entries {
            match entry.status {
                OccurrenceStatus::Done => {
                    run += 1;
                    longest = longest.max(run);
                }
                OccurrenceStatus::Missed => run = 0,
                OccurrenceStatus::Skipped | OccurrenceStatus::Open | OccurrenceStatus::Coming => {}
            }
        }
        longest
    }

    fn tally(&self, day: Day) -> DayTally {
        let mut tally = DayTally {
            day,
            done: 0,
            missed: 0,
            open: 0,
            skipped: 0,
        };
        for entry in self.between(day, day) {
            match entry.status {
                OccurrenceStatus::Done => tally.done += 1,
                OccurrenceStatus::Missed => tally.missed += 1,
                OccurrenceStatus::Skipped => tally.skipped += 1,
                OccurrenceStatus::Open | OccurrenceStatus::Coming => tally.open += 1,
            }
        }
        tally
    }

    /// The Adherence from `from` to `to`, both days included.
    pub fn adherence(&self, from: Day, to: Day) -> Adherence {
        let mut adherence = Adherence::default();
        for entry in self.between(from, to) {
            match entry.status {
                OccurrenceStatus::Done => adherence.done += 1,
                OccurrenceStatus::Missed => adherence.missed += 1,
                OccurrenceStatus::Skipped => adherence.skipped += 1,
                OccurrenceStatus::Open | OccurrenceStatus::Coming => {}
            }
        }
        adherence
    }
}

/// What the Occurrences of a Routine were answered with, by
/// [`Occurrence::key`]: when each was Done, when each was skipped, and
/// which of those Tomorrow skipped.
#[derive(Debug, Clone, Copy)]
pub struct Answers<'a> {
    pub done: &'a BTreeMap<String, DateTime<Utc>>,
    pub skipped: &'a BTreeMap<String, DateTime<Utc>>,
    pub tomorrow: &'a BTreeSet<String>,
}

impl<'a> Answers<'a> {
    /// What `routine` holds.
    pub fn of(routine: &'a Routine) -> Answers<'a> {
        Answers {
            done: &routine.done,
            skipped: &routine.skipped,
            tomorrow: &routine.tomorrow,
        }
    }
}

/// The history at `now` of a Routine that had the `spans` (see
/// [`schedule_spans`]), with the `answers` it was given and snoozed on
/// this Device until `snoozes`, all by [`Occurrence::key`], with days as
/// `wall` shows them. An Occurrence is Missed as the Missed Summary says:
/// when the next one starts, unless the schedule changed or the Routine
/// went to the trash before, or it was skipped. Whatever was Done counts,
/// asked for or not. Each span holds the Occurrences Tomorrow added to
/// its schedule ([`moved_occurrences`]). An all-day Occurrence starts at
/// `all_day_from`, as the alarm plan has it.
pub fn routine_history(
    spans: &[ScheduleSpan],
    answers: Answers<'_>,
    snoozes: &BTreeMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    week_start: u32,
    all_day_from: ClockTime,
) -> RoutineHistory {
    let Answers {
        done,
        skipped,
        tomorrow,
    } = answers;
    let today = Day::today(now, wall);
    let mut asked: BTreeMap<String, HistoryEntry> = BTreeMap::new();
    // Done, but outside every span: kept unless asked for elsewhere.
    let mut done_aside: BTreeMap<String, HistoryEntry> = BTreeMap::new();
    let entry = |occurrence: Occurrence,
                 status: OccurrenceStatus,
                 start: DateTime<Utc>,
                 missed_at: DateTime<Utc>,
                 moved: bool| {
        let key = occurrence.key();
        HistoryEntry {
            occurrence,
            status,
            start,
            missed_at,
            done_at: done.get(&key).copied(),
            snoozed_until: snoozes.get(&key).copied(),
            skipped_at: skipped.get(&key).copied(),
            moved,
        }
    };
    for span in spans {
        let schedule = &span.schedule;
        let from = Day::today(span.from, wall);
        let to = span.until.map_or(today, |until| Day::today(until, wall));
        if to < from {
            continue;
        }
        let moved = moved_occurrences(schedule, tomorrow);
        let occurrences = occurrences_between(schedule, &moved, from, to);
        let after = first_occurrence_from(schedule, &moved, to.add_days(1));
        for (i, occurrence) in occurrences.iter().enumerate() {
            if occurrence.day > today {
                break;
            }
            let next = occurrences.get(i + 1).or(after.as_ref());
            let is_moved = moved.contains(occurrence);
            let wall_start = start_of(occurrence, all_day_from);
            let wall_ends = missed_from(occurrence, next);
            let start = instant_of(wall_start.day, wall_start.time, wall);
            let missed_at = instant_of(wall_ends.day, wall_ends.time, wall);
            let in_span = start >= span.from && span.until.is_none_or(|until| start < until);
            let key = occurrence.key();
            if done.contains_key(&key) {
                let e = entry(
                    *occurrence,
                    OccurrenceStatus::Done,
                    start,
                    missed_at,
                    is_moved,
                );
                if in_span {
                    asked.insert(key, e);
                } else {
                    done_aside.entry(key).or_insert(e);
                }
                continue;
            }
            if !in_span {
                continue;
            }
            if skipped.contains_key(&key) {
                let e = entry(
                    *occurrence,
                    OccurrenceStatus::Skipped,
                    start,
                    missed_at,
                    is_moved,
                );
                asked.insert(key, e);
            } else if missed_at <= now && span.until.is_none_or(|until| missed_at <= until) {
                let e = entry(
                    *occurrence,
                    OccurrenceStatus::Missed,
                    start,
                    missed_at,
                    is_moved,
                );
                asked.insert(key, e);
            } else if span.until.is_none() {
                let status = if start > now {
                    OccurrenceStatus::Coming
                } else {
                    OccurrenceStatus::Open
                };
                asked.insert(key, entry(*occurrence, status, start, missed_at, is_moved));
            }
        }
    }
    for (key, value) in done_aside {
        asked.entry(key).or_insert(value);
    }
    // Done under a schedule no span shows, as from another Device.
    for key in done.keys() {
        if asked.contains_key(key) {
            continue;
        }
        let Some(occurrence) = Occurrence::parse(key) else {
            continue;
        };
        if occurrence.day > today {
            continue;
        }
        let wall_start = start_of(&occurrence, all_day_from);
        let wall_ends = missed_from(&occurrence, None);
        let e = entry(
            occurrence,
            OccurrenceStatus::Done,
            instant_of(wall_start.day, wall_start.time, wall),
            instant_of(wall_ends.day, wall_ends.time, wall),
            false,
        );
        asked.insert(key.clone(), e);
    }
    let mut entries: Vec<HistoryEntry> = asked.into_values().collect();
    entries.sort_by_key(|e| e.occurrence);
    RoutineHistory::new(entries, today, week_start)
}

impl Routines<'_> {
    /// The history of `routine` at the store's now, on `wall`, with the
    /// snoozes this Device knows of, weeks starting on `week_start` (1
    /// for Monday to 7 for Sunday).
    pub fn history(
        &self,
        routine: &Routine,
        wall: &WallClock,
        week_start: u32,
    ) -> Result<RoutineHistory> {
        let prefix = format!("{}@", routine.id);
        // Keyed by snooze_key: the Routine's id, then the Occurrence's.
        let snoozes: BTreeMap<String, DateTime<Utc>> = Alarms::new(self.store)
            .snoozes()?
            .into_iter()
            .filter_map(|(key, until)| Some((key.strip_prefix(&prefix)?.to_string(), until)))
            .collect();
        let spans = schedule_spans(&self.store.history(&routine.id)?);
        Ok(routine_history(
            &spans,
            Answers::of(routine),
            &snoozes,
            self.store.now(),
            wall,
            week_start,
            DeviceSettings::new(self.store).all_day_from()?,
        ))
    }
}

#[cfg(test)]
mod tests;
