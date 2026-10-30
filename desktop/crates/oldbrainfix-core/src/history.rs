//! A Routine's history (CONTEXT.md) as the Dart core's `history.dart`
//! has it: each Occurrence Done, Missed or still open, its Streak and its
//! Adherence, as a pure function of the schedules it had, what was Done,
//! this Device's snoozes, now and the wall clock. Missed follows the same
//! rules as the Missed Summary.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::Result;
use crate::alarms::{Alarms, instant_of, missed_from, start_of};
use crate::entry::Entry;
use crate::routines::{Routine, Routines, keys};
use crate::schedule::{Day, Occurrence, Schedule, WallClock};
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
}

impl HistoryEntry {
    /// Whether it was Done early, on time or late; `None` unless Done.
    pub fn timing(&self) -> Option<DoneTiming> {
        let done = self.done_at?;
        Some(if done < self.start {
            DoneTiming::Early
        } else if done < self.missed_at {
            DoneTiming::OnTime
        } else {
            DoneTiming::Late
        })
    }
}

/// The Done and Missed Occurrences over a period; those still open do
/// not count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Adherence {
    pub done: usize,
    pub missed: usize,
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
}

impl DayTally {
    /// Whether nothing was due that day.
    pub fn is_empty(&self) -> bool {
        self.done + self.missed + self.open == 0
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
    /// open (CONTEXT.md, Streak): one still due does not break it.
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
                OccurrenceStatus::Open | OccurrenceStatus::Coming => {}
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
                OccurrenceStatus::Open | OccurrenceStatus::Coming => {}
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
        };
        for entry in self.between(day, day) {
            match entry.status {
                OccurrenceStatus::Done => tally.done += 1,
                OccurrenceStatus::Missed => tally.missed += 1,
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
                OccurrenceStatus::Open | OccurrenceStatus::Coming => {}
            }
        }
        adherence
    }
}

/// The history at `now` of a Routine that had the `spans` (see
/// [`schedule_spans`]), was Done at `done` and was snoozed on this Device
/// until `snoozes`, both by [`Occurrence::key`], with days as `wall`
/// shows them. An Occurrence is Missed as the Missed Summary says: when
/// the next one starts, unless the schedule changed or the Routine went
/// to the trash before. Whatever was Done counts, asked for or not.
pub fn routine_history(
    spans: &[ScheduleSpan],
    done: &BTreeMap<String, DateTime<Utc>>,
    snoozes: &BTreeMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    week_start: u32,
) -> RoutineHistory {
    let today = Day::today(now, wall);
    let mut asked: BTreeMap<String, HistoryEntry> = BTreeMap::new();
    // Done, but outside every span: kept unless asked for elsewhere.
    let mut done_aside: BTreeMap<String, HistoryEntry> = BTreeMap::new();
    let entry = |occurrence: Occurrence,
                 status: OccurrenceStatus,
                 start: DateTime<Utc>,
                 missed_at: DateTime<Utc>| {
        let key = occurrence.key();
        HistoryEntry {
            occurrence,
            status,
            start,
            missed_at,
            done_at: done.get(&key).copied(),
            snoozed_until: snoozes.get(&key).copied(),
        }
    };
    for span in spans {
        let schedule = &span.schedule;
        let from = Day::today(span.from, wall);
        let to = span.until.map_or(today, |until| Day::today(until, wall));
        if to < from {
            continue;
        }
        let occurrences = schedule.between(from, to);
        let after = if schedule.once() {
            None
        } else {
            schedule.next(to.add_days(1), 1).into_iter().next()
        };
        for (i, occurrence) in occurrences.iter().enumerate() {
            if occurrence.day > today {
                break;
            }
            let next = occurrences.get(i + 1).or(after.as_ref());
            let wall_start = start_of(occurrence);
            let wall_ends = missed_from(occurrence, next);
            let start = instant_of(wall_start.day, wall_start.time, wall);
            let missed_at = instant_of(wall_ends.day, wall_ends.time, wall);
            let in_span = start >= span.from && span.until.is_none_or(|until| start < until);
            let key = occurrence.key();
            if done.contains_key(&key) {
                let e = entry(*occurrence, OccurrenceStatus::Done, start, missed_at);
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
            if missed_at <= now && span.until.is_none_or(|until| missed_at <= until) {
                let e = entry(*occurrence, OccurrenceStatus::Missed, start, missed_at);
                asked.insert(key, e);
            } else if span.until.is_none() {
                let status = if start > now {
                    OccurrenceStatus::Coming
                } else {
                    OccurrenceStatus::Open
                };
                asked.insert(key, entry(*occurrence, status, start, missed_at));
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
        let wall_start = start_of(&occurrence);
        let wall_ends = missed_from(&occurrence, None);
        let e = entry(
            occurrence,
            OccurrenceStatus::Done,
            instant_of(wall_start.day, wall_start.time, wall),
            instant_of(wall_ends.day, wall_ends.time, wall),
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
            &routine.done,
            &snoozes,
            self.store.now(),
            wall,
            week_start,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    use chrono::{NaiveDateTime, TimeZone};
    use parking_lot::Mutex;

    use super::*;
    use crate::alarms::missed_summary;
    use crate::schedule::{ClockTime, Recurrence};
    use crate::signing::SigningKey;

    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Central European time, as Europe/Berlin keeps it.
    fn europe(utc: DateTime<Utc>) -> NaiveDateTime {
        utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
    }

    fn at(hour: u32, minute: u32) -> ClockTime {
        ClockTime { hour, minute }
    }

    /// The instant the clock in Europe shows `hour`:`minute` on
    /// `month`/`day` 2026.
    fn wall(month: i64, day: i64, hour: u32, minute: u32) -> DateTime<Utc> {
        instant_of(Day::new(2026, month, day), at(hour, minute), &europe)
    }

    fn utc(month: u32, day: u32, hour: u32, minute: u32) -> Made<DateTime<Utc>> {
        Ok(Utc
            .with_ymd_and_hms(2026, month, day, hour, minute, 0)
            .single()
            .ok_or("an instant")?)
    }

    fn on(month: i64, day: i64, time: ClockTime) -> Occurrence {
        Occurrence::new(Day::new(2026, month, day), Some(time))
    }

    fn daily(times: &[ClockTime]) -> Made<Schedule> {
        let rule = Recurrence::parse("FREQ=DAILY")?;
        Ok(Schedule::new(Day::new(2026, 9, 1), times, Some(rule)).ok_or("a schedule")?)
    }

    fn log(history: &RoutineHistory) -> Vec<String> {
        history
            .entries
            .iter()
            .map(|e| format!("{} {}", e.occurrence.key(), e.status.name()))
            .collect()
    }

    /// A store on a held clock, at 09:00 on Saturday, 24 October 2026,
    /// in Europe.
    struct World {
        now: Arc<Mutex<DateTime<Utc>>>,
        store: Store,
    }

    impl World {
        fn new() -> Made<World> {
            let now = Arc::new(Mutex::new(wall(10, 24, 9, 0)));
            let held = now.clone();
            let store = Store::in_memory(Box::new(move || *held.lock()), || {
                Ok(SigningKey::from_seed([1; 32]))
            })?;
            Ok(World { now, store })
        }

        fn set_now(&self, at: DateTime<Utc>) {
            *self.now.lock() = at;
        }

        fn routines(&self) -> Routines<'_> {
            Routines::new(&self.store)
        }

        /// Runs `body` as of `then`.
        fn as_of<T>(&self, then: DateTime<Utc>, body: impl FnOnce() -> T) -> T {
            let saved = *self.now.lock();
            self.set_now(then);
            let out = body();
            self.set_now(saved);
            out
        }

        /// Makes a Routine as of `made`, before its Occurrences were due.
        fn made_at(&self, made: DateTime<Utc>, schedule: &Schedule) -> Made<Routine> {
            Ok(self.as_of(made, || self.routines().create("pills", schedule))?)
        }

        fn done(&self, routine: &Routine, occurrence: Occurrence, at: DateTime<Utc>) -> Made<()> {
            Ok(self
                .routines()
                .mark_done(&routine.id, &occurrence, Some(at))?)
        }

        fn history_of(&self, routine: &Routine, week_start: u32) -> Made<RoutineHistory> {
            let routines = self.routines();
            let current = routines
                .by_id(&routine.id)?
                .unwrap_or_else(|| routine.clone());
            Ok(routines.history(&current, &europe, week_start)?)
        }
    }

    #[test]
    fn several_times_a_day_each_occurrence_on_its_own_and_one_still_due_neither_counts_nor_breaks_the_streak()
    -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 20, 6, 0), &daily(&[at(8, 0), at(20, 0)])?)?;
        w.done(&pills, on(10, 20, at(8, 0)), wall(10, 20, 8, 5))?;
        for day in [21, 22, 23] {
            w.done(&pills, on(10, day, at(8, 0)), wall(10, day, 8, 5))?;
            w.done(&pills, on(10, day, at(20, 0)), wall(10, day, 20, 15))?;
        }
        let history = w.history_of(&pills, MONDAY)?;
        assert_eq!(
            log(&history),
            [
                "2026-10-20T08:00 done",
                "2026-10-20T20:00 missed",
                "2026-10-21T08:00 done",
                "2026-10-21T20:00 done",
                "2026-10-22T08:00 done",
                "2026-10-22T20:00 done",
                "2026-10-23T08:00 done",
                "2026-10-23T20:00 done",
                "2026-10-24T08:00 open",
                "2026-10-24T20:00 coming",
            ]
        );
        let first = &history.entries[0];
        assert_eq!(first.start, wall(10, 20, 8, 0));
        assert_eq!(first.done_at, Some(wall(10, 20, 8, 5)));
        assert_eq!(first.timing(), Some(DoneTiming::OnTime));
        // Missed when the next one started.
        assert_eq!(history.entries[1].missed_at, wall(10, 21, 8, 0));
        assert_eq!(history.entries[1].done_at, None);
        assert_eq!(history.current_streak, 6);
        assert_eq!(history.longest_streak, 6);
        Ok(())
    }

    #[test]
    fn done_early_on_time_or_late_and_late_done_keeps_the_streak() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 22, 6, 0), &daily(&[at(8, 0), at(20, 0)])?)?;
        w.done(&pills, on(10, 22, at(8, 0)), wall(10, 22, 7, 50))?;
        w.done(&pills, on(10, 22, at(20, 0)), wall(10, 22, 21, 0))?;
        // From the Missed Summary, the next morning.
        w.done(&pills, on(10, 23, at(8, 0)), wall(10, 23, 8, 10))?;
        w.done(&pills, on(10, 23, at(20, 0)), wall(10, 24, 8, 30))?;
        w.done(&pills, on(10, 24, at(8, 0)), wall(10, 24, 8, 1))?;
        w.done(&pills, on(10, 24, at(20, 0)), wall(10, 24, 9, 0))?;
        let history = w.history_of(&pills, MONDAY)?;
        let timings: Vec<_> = history.entries.iter().map(HistoryEntry::timing).collect();
        assert_eq!(
            timings,
            [
                Some(DoneTiming::Early),
                Some(DoneTiming::OnTime),
                Some(DoneTiming::OnTime),
                Some(DoneTiming::Late),
                Some(DoneTiming::OnTime),
                Some(DoneTiming::Early),
            ]
        );
        assert_eq!(history.current_streak, 6);
        Ok(())
    }

    #[test]
    fn the_longest_streak_outlasts_a_missed_and_the_current_one_restarts() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 18, 6, 0), &daily(&[at(8, 0)])?)?;
        for day in [18, 19, 20, 22, 23] {
            w.done(&pills, on(10, day, at(8, 0)), wall(10, day, 8, 0))?;
        }
        let history = w.history_of(&pills, MONDAY)?;
        assert_eq!(history.longest_streak, 3);
        assert_eq!(history.current_streak, 2);

        w.routines().unmark_done(&pills.id, &on(10, 23, at(8, 0)))?;
        assert_eq!(w.history_of(&pills, MONDAY)?.current_streak, 0);
        Ok(())
    }

    #[test]
    fn nothing_from_before_the_routine_was_made_counts() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 24, 8, 30), &daily(&[at(8, 0)])?)?;
        let history = w.history_of(&pills, MONDAY)?;
        assert!(history.entries.is_empty());
        assert_eq!(history.current_streak, 0);
        assert_eq!(history.longest_streak, 0);
        assert_eq!(history.this_week.share(), None);
        Ok(())
    }

    #[test]
    fn the_last_seven_days_day_by_day() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 20, 6, 0), &daily(&[at(8, 0), at(20, 0)])?)?;
        w.done(&pills, on(10, 20, at(8, 0)), wall(10, 20, 8, 0))?;
        w.done(&pills, on(10, 21, at(8, 0)), wall(10, 21, 8, 0))?;
        w.done(&pills, on(10, 21, at(20, 0)), wall(10, 21, 20, 0))?;
        let days = w.history_of(&pills, MONDAY)?.last_seven_days;
        let shown: Vec<Day> = days.iter().map(|d| d.day).collect();
        let want: Vec<Day> = (18..=24).map(|d| Day::new(2026, 10, d)).collect();
        assert_eq!(shown, want);
        let counts: Vec<_> = days.iter().map(|d| (d.done, d.missed, d.open)).collect();
        assert_eq!(
            counts,
            [
                (0, 0, 0),
                (0, 0, 0),
                (1, 1, 0),
                (2, 0, 0),
                (0, 2, 0),
                (0, 2, 0),
                (0, 0, 2),
            ]
        );
        assert!(days[0].is_empty());
        assert!(!days[2].is_empty());
        Ok(())
    }

    #[test]
    fn snoozes_this_device_knows_of_show_and_those_of_others_do_not() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 22, 6, 0), &daily(&[at(8, 0)])?)?;
        let other = w.made_at(wall(10, 22, 6, 0), &daily(&[at(8, 0)])?)?;
        let alarms = Alarms::new(&w.store);
        alarms.apply_snooze(&pills.id, "2026-10-23T08:00", wall(10, 23, 8, 30))?;
        alarms.apply_snooze(&other.id, "2026-10-22T08:00", wall(10, 22, 9, 0))?;
        let history = w.history_of(&pills, MONDAY)?;
        let snoozed: Vec<_> = history.entries.iter().map(|e| e.snoozed_until).collect();
        assert_eq!(snoozed, [None, Some(wall(10, 23, 8, 30)), None]);
        let synced = w.store.all_entries()?;
        assert!(!synced.iter().any(|e| e.field.contains("snooze")));
        Ok(())
    }

    #[test]
    fn missed_agrees_with_the_missed_summary() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(
            wall(10, 10, 6, 0),
            &daily(&[at(8, 0), at(13, 0), at(20, 0)])?,
        )?;
        for day in 10..=24 {
            if day % 3 == 0 {
                continue;
            }
            w.done(&pills, on(10, day, at(13, 0)), wall(10, day, 13, 0))?;
        }
        w.done(&pills, on(10, 15, at(8, 0)), wall(10, 15, 8, 0))?;
        let routine = w.routines().by_id(&pills.id)?.ok_or("the Routine")?;
        let summary = missed_summary(&[(routine, None)], None, w.store.now(), &europe);
        let history = w.history_of(&pills, MONDAY)?;
        let missed: Vec<&HistoryEntry> = history
            .entries
            .iter()
            .filter(|e| e.status == OccurrenceStatus::Missed)
            .collect();
        let summed = &summary.routines[0].missed;
        assert!(!summed.is_empty());
        assert_eq!(
            missed.iter().map(|e| e.occurrence).collect::<BTreeSet<_>>(),
            summed.iter().map(|m| m.occurrence).collect::<BTreeSet<_>>()
        );
        assert_eq!(
            missed.iter().map(|e| e.missed_at).collect::<BTreeSet<_>>(),
            summed.iter().map(|m| m.missed_at).collect::<BTreeSet<_>>()
        );
        Ok(())
    }

    #[test]
    fn missed_across_the_end_of_summer_time_by_the_wall_clock() -> Made<()> {
        let w = World::new()?;
        // 09:00 on Tuesday, 27 October, in winter time.
        w.set_now(wall(10, 27, 9, 0));
        // 02:30 comes twice on the 25th; it is due at the first.
        let pills = w.made_at(wall(10, 22, 6, 0), &daily(&[at(2, 30)])?)?;
        w.done(&pills, on(10, 23, at(2, 30)), wall(10, 23, 2, 40))?;
        w.done(&pills, on(10, 25, at(2, 30)), utc(10, 25, 1, 15)?)?;
        w.done(&pills, on(10, 27, at(2, 30)), wall(10, 27, 3, 0))?;
        let history = w.history_of(&pills, MONDAY)?;
        assert_eq!(
            log(&history),
            [
                "2026-10-23T02:30 done",
                "2026-10-24T02:30 missed",
                "2026-10-25T02:30 done",
                "2026-10-26T02:30 missed",
                "2026-10-27T02:30 done",
            ]
        );
        let sunday = &history.entries[2];
        assert_eq!(sunday.start, utc(10, 25, 0, 30)?);
        // 02:15 the second time round, still before the next one.
        assert_eq!(sunday.timing(), Some(DoneTiming::OnTime));
        assert_eq!(history.entries[3].missed_at, utc(10, 27, 1, 30)?);
        let counted: Vec<_> = history
            .last_seven_days
            .iter()
            .map(|d| d.done + d.missed)
            .collect();
        assert_eq!(counted, [0, 0, 1, 1, 1, 1, 1]);
        assert_eq!(history.last_30_days.done, 3);
        assert_eq!(history.last_30_days.missed, 2);
        Ok(())
    }

    #[test]
    fn a_schedule_change_keeps_what_was_missed_before_it_and_the_occurrence_across_it_counts_neither_way()
    -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 20, 6, 0), &daily(&[at(8, 0)])?)?;
        w.done(&pills, on(10, 22, at(8, 0)), wall(10, 22, 8, 0))?;
        // At noon on the 23rd, 08:00 that day still not Done, to 10:00.
        let later = daily(&[at(10, 0)])?;
        w.as_of(wall(10, 23, 12, 0), || {
            w.routines().reschedule(&pills.id, &later)
        })?;
        let history = w.history_of(&pills, MONDAY)?;
        assert_eq!(
            log(&history),
            [
                "2026-10-20T08:00 missed",
                "2026-10-21T08:00 missed",
                "2026-10-22T08:00 done",
                "2026-10-24T10:00 coming",
            ]
        );
        assert_eq!(history.last_30_days.done, 1);
        assert_eq!(history.last_30_days.missed, 2);
        // The Missed Summary only looks at the schedule as it is now.
        assert!(Alarms::new(&w.store).missed(&europe)?.is_empty());
        Ok(())
    }

    #[test]
    fn nothing_from_the_trash_period_counts_and_the_streak_goes_on_across_it() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 18, 20, 0), &daily(&[at(8, 0)])?)?;
        w.done(&pills, on(10, 20, at(8, 0)), wall(10, 20, 8, 0))?;
        w.done(&pills, on(10, 21, at(8, 0)), wall(10, 21, 8, 0))?;
        w.as_of(wall(10, 21, 12, 0), || w.routines().trash(&pills.id))?;
        w.as_of(wall(10, 23, 9, 30), || w.routines().restore(&pills.id))?;
        w.done(&pills, on(10, 24, at(8, 0)), wall(10, 24, 8, 0))?;
        let history = w.history_of(&pills, MONDAY)?;
        assert_eq!(
            log(&history),
            [
                "2026-10-19T08:00 missed",
                "2026-10-20T08:00 done",
                "2026-10-21T08:00 done",
                "2026-10-24T08:00 done",
            ]
        );
        assert_eq!(history.current_streak, 3);
        assert_eq!(history.longest_streak, 3);
        assert_eq!(history.last_30_days.missed, 1);
        Ok(())
    }

    #[test]
    fn a_routine_trashed_before_its_occurrence_was_missed() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 22, 6, 0), &daily(&[at(8, 0)])?)?;
        w.as_of(wall(10, 22, 12, 0), || w.routines().trash(&pills.id))?;
        assert!(log(&w.history_of(&pills, MONDAY)?).is_empty());
        Ok(())
    }

    /// Done every day at 08:05 from 27 September but the 30th, 5 and 20
    /// October; today's 08:00 is still due.
    fn adherent() -> Made<(World, Routine)> {
        let w = World::new()?;
        let pills = w.made_at(wall(9, 27, 6, 0), &daily(&[at(8, 0)])?)?;
        let missed = [(9, 30), (10, 5), (10, 20)];
        let mut d = Day::new(2026, 9, 27);
        while d < Day::new(2026, 10, 24) {
            let (month, day) = (i64::from(d.month), i64::from(d.day));
            if !missed.contains(&(month, day)) {
                w.done(&pills, on(month, day, at(8, 0)), wall(month, day, 8, 5))?;
            }
            d = d.add_days(1);
        }
        Ok((w, pills))
    }

    #[test]
    fn adherence_this_week_this_month_and_the_last_30_days_without_one_still_due() -> Made<()> {
        let (w, pills) = adherent()?;
        let history = w.history_of(&pills, MONDAY)?;
        // Monday the 19th to today.
        assert_eq!((history.this_week.done, history.this_week.missed), (4, 1));
        assert_eq!(history.this_week.share(), Some(0.8));
        assert_eq!(
            (history.this_month.done, history.this_month.missed),
            (21, 2)
        );
        assert_eq!(
            (history.last_30_days.done, history.last_30_days.missed),
            (24, 3)
        );
        Ok(())
    }

    #[test]
    fn a_week_can_start_on_sunday() -> Made<()> {
        let (w, pills) = adherent()?;
        let history = w.history_of(&pills, 7)?;
        assert_eq!(history.week_start, 7);
        assert_eq!((history.this_week.done, history.this_week.missed), (5, 1));
        Ok(())
    }

    #[test]
    fn adherence_over_any_period() -> Made<()> {
        let (w, pills) = adherent()?;
        let history = w.history_of(&pills, MONDAY)?;
        let september = history.adherence(Day::new(2026, 9, 1), Day::new(2026, 9, 30));
        assert_eq!((september.done, september.missed), (3, 1));
        assert_eq!(september.share(), Some(0.75));
        Ok(())
    }

    #[test]
    fn a_schedule_written_a_moment_apart_is_one_change_as_of_its_last_write() -> Made<()> {
        // Every read of the clock moves it on a little.
        let start = wall(10, 20, 8, 0) - Duration::milliseconds(5);
        let ticks = Arc::new(AtomicI64::new(0));
        let clock =
            move || start + Duration::milliseconds(ticks.fetch_add(1, Ordering::SeqCst) + 1);
        let store = Store::in_memory(Box::new(clock), || Ok(SigningKey::from_seed([1; 32])))?;
        let routines = Routines::new(&store);
        let pills = routines.create("pills", &daily(&[at(8, 0)])?)?;
        let spans = schedule_spans(&store.history(&pills.id)?);
        assert_eq!(spans.len(), 1);
        let current = routines.by_id(&pills.id)?.ok_or("the Routine")?;
        assert_eq!(Some(spans[0].from), current.scheduled_at);
        assert_eq!(spans[0].until, None);
        assert_eq!(spans[0].schedule.day_times(), [at(8, 0)]);
        Ok(())
    }

    #[test]
    fn the_trash_and_a_new_schedule_end_a_span() -> Made<()> {
        let w = World::new()?;
        let pills = w.made_at(wall(10, 18, 6, 0), &daily(&[at(8, 0)])?)?;
        w.as_of(wall(10, 19, 12, 0), || w.routines().trash(&pills.id))?;
        w.as_of(wall(10, 20, 12, 0), || w.routines().restore(&pills.id))?;
        let later = daily(&[at(9, 0)])?;
        w.as_of(wall(10, 21, 12, 0), || {
            w.routines().reschedule(&pills.id, &later)
        })?;
        let spans = schedule_spans(&w.store.history(&pills.id)?);
        let bounds: Vec<_> = spans.iter().map(|s| (s.from, s.until)).collect();
        assert_eq!(
            bounds,
            [
                (wall(10, 18, 6, 0), Some(wall(10, 19, 12, 0))),
                (wall(10, 20, 12, 0), Some(wall(10, 21, 12, 0))),
                (wall(10, 21, 12, 0), None),
            ]
        );
        let times: Vec<_> = spans.iter().map(|s| s.schedule.day_times()).collect();
        assert_eq!(times, [vec![at(8, 0)], vec![at(8, 0)], vec![at(9, 0)]]);
        Ok(())
    }
}
