//! The alarm plan (ADR 0004) as the Dart core's `alarms.dart` has it:
//! which alarms should exist on this Device, as a pure function of the
//! Routines, now, the wall clock and this Device's snoozes; the Missed
//! Summary; and this Device's snoozes and "seen" mark in its local
//! settings, never synced (ADR 0008).

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::notes::{Note, Notes, Wordless};
use crate::nudges::{
    NudgeSnooze, PlannedChangesNudge, PlannedNudge, QuietHours, changes_nudge_plan, nudge_plan,
};
use crate::person_settings::PersonSettings;
use crate::routines::{Routine, Routines};
use crate::schedule::{ClockTime, Day, Occurrence, WallClock};
use crate::settings::DeviceSettings;
use crate::store::{Store, utf16_cmp};
use crate::{Error, Result};

/// How many days back and ahead the plan looks.
pub const ALARM_HORIZON_DAYS: i64 = 30;

/// At most this many alarms per Routine.
pub const ALARMS_PER_ROUTINE: usize = 128;

/// At most this many alarms in all, the soonest first.
pub const ALARM_LIMIT: usize = 2000;

/// After how many ignored Re-nags Escalation starts when it is turned on.
pub const DEFAULT_ESCALATION: i64 = 3;

/// The instant `day` at `time` is on `wall`. A time the clock shows twice
/// (when daylight saving ends) fires once, at the first; a time it skips
/// (when daylight saving starts) fires at the next valid instant, the
/// moment the clock jumps past it.
pub fn instant_of(day: Day, time: ClockTime, wall: &WallClock) -> DateTime<Utc> {
    let target = utc_at(day, time);
    // What the wall shows, to the millisecond, read as if it were UTC.
    let wall_of = |utc: DateTime<Utc>| -> DateTime<Utc> {
        let shown = wall(utc).and_utc();
        from_millis(shown.timestamp_millis())
    };
    let offset_at = |utc: DateTime<Utc>| wall_of(utc) - utc;
    // The offsets a day before and a day after: equal on most days, the
    // two sides of a switch on the day of one.
    let before = offset_at(target - Duration::days(1));
    let after = offset_at(target + Duration::days(1));
    let mut hits: Vec<DateTime<Utc>> = Vec::new();
    for offset in [before, after] {
        let t = target - offset;
        if !hits.contains(&t) && wall_of(t) == target {
            hits.push(t);
        }
    }
    hits.sort();
    if let Some(first) = hits.first() {
        return *first;
    }
    // Skipped: find where the clock jumps, between the instant that still
    // shows an earlier time and the one that already shows a later one.
    let mut lo = (target - after).timestamp_millis();
    let mut hi = (target - before).timestamp_millis();
    if lo >= hi {
        return from_millis(hi);
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if wall_of(from_millis(mid)) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    from_millis(hi)
}

fn from_millis(ms: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ms).single().unwrap_or_default()
}

/// `day` at `time` as if the wall clock were UTC.
fn utc_at(day: Day, time: ClockTime) -> DateTime<Utc> {
    let date = i32::try_from(day.year)
        .ok()
        .and_then(|y| NaiveDate::from_ymd_opt(y, day.month, day.day));
    date.and_then(|d| d.and_hms_opt(time.hour, time.minute, 0))
        .map(|n: NaiveDateTime| n.and_utc())
        .unwrap_or_default()
}

/// A moment on the wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WallMoment {
    pub day: Day,
    pub time: ClockTime,
}

impl fmt::Display for WallMoment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.day, self.time)
    }
}

/// When `occurrence` starts: at its time, an all-day one at
/// `all_day_from`, this Device's [`DeviceSettings::all_day_from`].
pub fn start_of(occurrence: &Occurrence, all_day_from: ClockTime) -> WallMoment {
    WallMoment {
        day: occurrence.day,
        time: occurrence.time.unwrap_or(all_day_from),
    }
}

/// When `occurrence` counts as Missed unless it is Done (CONTEXT.md):
/// when `next` starts (an all-day one at the start of its day), or at
/// midnight after its day when there is no next one.
pub fn missed_from(occurrence: &Occurrence, next: Option<&Occurrence>) -> WallMoment {
    let midnight = ClockTime { hour: 0, minute: 0 };
    match next {
        None => WallMoment {
            day: occurrence.day.add_days(1),
            time: midnight,
        },
        Some(next) => WallMoment {
            day: next.day,
            time: next.time.unwrap_or(midnight),
        },
    }
}

/// One Occurrence of a Routine placed in time.
struct Timed {
    occurrence: Occurrence,
    start: DateTime<Utc>,
    ends: WallMoment,
    ends_at: DateTime<Utc>,
}

/// The Occurrences of `routine` from `from` to `to` that were asked for
/// (due at or after when it was scheduled), placed on `wall`; those
/// Tomorrow added among them, each counting as the next one of the one
/// before it.
fn timed(
    routine: &Routine,
    from: Day,
    to: Day,
    wall: &WallClock,
    all_day_from: ClockTime,
) -> Vec<Timed> {
    let occurrences = routine.between(from, to);
    let after = routine.first_from(to.add_days(1));
    let mut out = Vec::new();
    for (i, occurrence) in occurrences.iter().enumerate() {
        let next = occurrences.get(i + 1).or(after.as_ref());
        let start = start_of(occurrence, all_day_from);
        let ends = missed_from(occurrence, next);
        let t = Timed {
            occurrence: *occurrence,
            start: instant_of(start.day, start.time, wall),
            ends,
            ends_at: instant_of(ends.day, ends.time, wall),
        };
        if routine.scheduled_at.is_none_or(|since| t.start >= since) {
            out.push(t);
        }
    }
    out
}

/// A stretch of time from `from` up to, not including, `until`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuietStretch {
    pub from: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

/// The person's quiet days (obf-1044, CONTEXT.md, Quiet day), as the
/// Dart core's `QuietDays`: each Day made quiet, with when End was tapped
/// on it, or `None` while it lasts to midnight. On a quiet day nothing
/// notifies: what starts on it rings, Re-nags and warns ahead nowhere,
/// and what is still undone at its midnight counts as skipped. Each
/// Device ends it at midnight on its own wall clock.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuietDays {
    /// End of each quiet Day: the moment End was tapped, or `None`.
    pub ends: BTreeMap<Day, Option<DateTime<Utc>>>,
}

impl QuietDays {
    /// The quiet stretch of `day` on `wall`: from the start of the day to
    /// midnight after it, or to End when that came first; `None` when
    /// `day` is not quiet or End came before it began.
    pub fn stretch_of(&self, day: Day, wall: &WallClock) -> Option<QuietStretch> {
        let end = self.ends.get(&day)?;
        let midnight_time = ClockTime { hour: 0, minute: 0 };
        let from = instant_of(day, midnight_time, wall);
        let midnight = instant_of(day.add_days(1), midnight_time, wall);
        let until = match end {
            Some(end) if *end < midnight => *end,
            _ => midnight,
        };
        (until > from).then_some(QuietStretch { from, until })
    }

    /// The quiet stretch `instant` lies in on `wall`; `None` when none.
    pub fn holding(&self, instant: DateTime<Utc>, wall: &WallClock) -> Option<QuietStretch> {
        self.stretch_of(Day::today(instant, wall), wall)
            .filter(|s| instant >= s.from && instant < s.until)
    }

    /// When what starts at `start` counts as skipped unless it is Done: at
    /// midnight after its quiet day, on `wall`; `None` when `start` lies
    /// in no quiet stretch, so it rings as ever.
    pub fn settles_at(&self, start: DateTime<Utc>, wall: &WallClock) -> Option<DateTime<Utc>> {
        self.holding(start, wall)?;
        Some(instant_of(
            Day::today(start, wall).add_days(1),
            ClockTime { hour: 0, minute: 0 },
            wall,
        ))
    }
}

/// Why an alarm fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlarmKind {
    /// An Occurrence is due.
    Occurrence,
    /// An Occurrence snoozed from its notification is due again.
    Snooze,
}

impl AlarmKind {
    pub fn name(self) -> &'static str {
        match self {
            AlarmKind::Occurrence => "occurrence",
            AlarmKind::Snooze => "snooze",
        }
    }
}

/// One alarm that should exist. While its Occurrence is not Done it fires
/// at `at` and then Re-nags every `renag`, up to `ends_at`, when the
/// Occurrence counts as Missed. With `escalation` on, every firing from
/// [`PlannedAlarm::escalates_at`] on is escalated. A firing in `quiet`
/// shows without sound ([`PlannedAlarm::silent_at`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAlarm {
    pub kind: AlarmKind,
    pub routine_id: String,
    pub occurrence: Occurrence,
    /// When it fires first; past for an Occurrence still Re-nagging.
    pub at: DateTime<Utc>,
    /// The wall-clock moment an Occurrence alarm fires at; `None` for a
    /// snooze, which waits a fixed time.
    pub wall: Option<WallMoment>,
    pub renag: Duration,
    pub escalation: Option<i64>,
    pub ends: WallMoment,
    pub ends_at: DateTime<Utc>,
    /// The Quiet Hours its firings show without sound or vibration in
    /// (obf-1045); `None` when they are off or its Routine escalates,
    /// which rings with sound from its first ring.
    pub quiet: Option<QuietHours>,
    /// The name the Note the Routine links back to is shown by
    /// ([`Note::shown_name`], obf-1049); `None` when gone.
    pub text: Option<String>,
    /// What that Note is titled as while `text` is empty
    /// ([`Note::wordless`]).
    pub wordless: Wordless,
    /// The name of the file it goes by as [`Wordless::File`]
    /// ([`Note::wordless_name`]); empty otherwise.
    pub wordless_name: String,
}

impl PlannedAlarm {
    /// Names the alarm for good: one per kind and Occurrence.
    pub fn id(&self) -> String {
        format!(
            "{}:{}",
            self.kind.name(),
            snooze_key(&self.routine_id, &self.occurrence)
        )
    }

    /// When it fires first after `after`: at `at`, or at a Re-nag; `None`
    /// when it has stopped by then.
    pub fn fires_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if self.at > after {
            return (self.at < self.ends_at).then_some(self.at);
        }
        let step = self.renag.num_milliseconds().max(1);
        let passed = (after - self.at).num_milliseconds();
        let next = self.at + Duration::milliseconds((passed / step + 1) * step);
        (next < self.ends_at).then_some(next)
    }

    /// When it first fires escalated: the firing after the first one and
    /// `escalation` Re-nags went ignored, counted from `at`. `None` when it
    /// does not escalate or stops before.
    pub fn escalates_at(&self) -> Option<DateTime<Utc>> {
        let after = self.escalation?;
        let first = self.at + self.renag * (after as i32 + 1);
        (first < self.ends_at).then_some(first)
    }

    /// Whether it fires at `firing` without sound or vibration: `firing`
    /// lies in `quiet` on `wall`.
    pub fn silent_at(&self, firing: DateTime<Utc>, wall: &WallClock) -> bool {
        self.quiet.is_some_and(|q| q.holds(firing, wall))
    }
}

/// The key a snooze of `occurrence` of the Routine `routine_id` is kept
/// under.
pub fn snooze_key(routine_id: &str, occurrence: &Occurrence) -> String {
    format!("{routine_id}@{}", occurrence.key())
}

/// The alarms that should exist at `now`: for each Routine its
/// Occurrences that are not Done, not skipped and not yet Missed, from
/// [`ALARM_HORIZON_DAYS`] back to as far ahead, at most
/// [`ALARMS_PER_ROUTINE`], the soonest first and at most [`ALARM_LIMIT`].
/// A snoozed one rings at its snooze instead; a snooze reaching past when
/// the Occurrence counts as Missed is dropped. Each Re-nags at its
/// Routine's interval, else at `renag`. An all-day Occurrence rings from
/// `all_day_from` (#201), or once `quiet` ends when they hold that time,
/// and has no alarm when they last until it counts as Missed. Every other
/// firing in `quiet` shows without sound, but for a Routine that
/// escalates (obf-1045). An Occurrence that starts on one of the
/// `quiet_days` has no alarm, and one that began before the quiet
/// stretch `now` lies in Re-nags again from its end, at the first Re-nag
/// after it (obf-1044).
#[allow(clippy::too_many_arguments)]
pub fn alarm_plan(
    routines: &[(Routine, Option<Note>)],
    snoozes: &BTreeMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    renag: Duration,
    all_day_from: ClockTime,
    quiet: Option<QuietHours>,
    quiet_days: &QuietDays,
) -> Vec<PlannedAlarm> {
    let today = Day::today(now, wall);
    let quiet_now = quiet_days.holding(now, wall);
    let mut alarms = Vec::new();
    for (routine, note) in routines {
        let mut planned = 0;
        let placed = timed(
            routine,
            today.add_days(-ALARM_HORIZON_DAYS),
            today.add_days(ALARM_HORIZON_DAYS),
            wall,
            all_day_from,
        );
        for t in placed {
            if planned >= ALARMS_PER_ROUTINE {
                break;
            }
            let occurrence = t.occurrence;
            if routine.done_at(&occurrence).is_some()
                || routine.skipped_at(&occurrence).is_some()
                || t.ends_at <= now
            {
                continue;
            }
            let snoozed = snoozes.get(&snooze_key(&routine.id, &occurrence)).copied();
            if snoozed.is_some_and(|s| s >= t.ends_at) {
                continue;
            }
            // A quiet day rings nothing of its own (obf-1044).
            if quiet_days.holding(t.start, wall).is_some() {
                continue;
            }
            let mut start = t.start;
            let mut moment = start_of(&occurrence, all_day_from);
            if let Some(quiet) = quiet.filter(|_| snoozed.is_none() && occurrence.time.is_none()) {
                // Quiet Hours win over the all-day time, not over a timed one.
                start = quiet.defer(t.start, wall);
                if start >= t.ends_at {
                    continue;
                }
                if start != t.start {
                    moment = WallMoment {
                        day: Day::of(wall(start)),
                        time: quiet.end,
                    };
                }
            }
            let every = routine.renag.unwrap_or(renag);
            let mut at = snoozed.unwrap_or(start);
            let mut moment = snoozed.is_none().then_some(moment);
            if let Some(stretch) = quiet_now.filter(|s| at < s.until) {
                // Begun before today's quiet stretch: it waits the stretch
                // out and Re-nags again from the first Re-nag after it.
                let step = every.num_milliseconds();
                if step <= 0 {
                    continue;
                }
                let waited = (stretch.until - at).num_milliseconds();
                at += Duration::milliseconds((waited + step - 1) / step * step);
                if at >= t.ends_at {
                    continue;
                }
                if moment.is_some() {
                    let shown = wall(at);
                    moment = Some(WallMoment {
                        day: Day::of(shown),
                        time: ClockTime {
                            hour: shown.hour(),
                            minute: shown.minute(),
                        },
                    });
                }
            }
            alarms.push(PlannedAlarm {
                kind: if snoozed.is_some() {
                    AlarmKind::Snooze
                } else {
                    AlarmKind::Occurrence
                },
                routine_id: routine.id.clone(),
                occurrence,
                at,
                wall: moment,
                renag: every,
                escalation: routine.escalation,
                ends: t.ends,
                ends_at: t.ends_at,
                // Escalation is the only way through Quiet Hours.
                quiet: quiet.filter(|_| routine.escalation.is_none()),
                text: note.as_ref().map(Note::shown_name),
                wordless: note.as_ref().map(Note::wordless).unwrap_or_default(),
                wordless_name: note.as_ref().map(Note::wordless_name).unwrap_or_default(),
            });
            planned += 1;
        }
    }
    alarms.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| utf16_cmp(&a.id(), &b.id())));
    alarms.truncate(ALARM_LIMIT);
    alarms
}

/// One Heads-up that should exist (obf-1046): a single notification
/// `minutes` before `occurrence` of the Routine `routine_id` starts, at
/// `at`, with no Re-nag and nothing to check off. A Heads-up in `quiet`
/// shows without sound ([`PlannedHeadsUp::silent_at`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedHeadsUp {
    pub routine_id: String,
    pub occurrence: Occurrence,
    /// How long before the Occurrence's start it warns, in minutes.
    pub minutes: i64,
    /// When it fires.
    pub at: DateTime<Utc>,
    /// The wall-clock moment it fires at, so the platform re-arms it at
    /// the same local time after a time-zone change.
    pub wall: WallMoment,
    /// When the Occurrence starts on the wall clock ([`start_of`]).
    pub start: WallMoment,
    /// This Device's Quiet Hours when they are on; `None` when they are
    /// off. A Heads-up never escalates, so its Routine's Escalation
    /// changes nothing.
    pub quiet: Option<QuietHours>,
    /// The name the Note the Routine links back to is shown by
    /// ([`Note::shown_name`], obf-1049); `None` when gone.
    pub text: Option<String>,
    /// What that Note is titled as while `text` is empty
    /// ([`Note::wordless`]).
    pub wordless: Wordless,
    /// The name of the file it goes by as [`Wordless::File`]
    /// ([`Note::wordless_name`]); empty otherwise.
    pub wordless_name: String,
}

impl PlannedHeadsUp {
    /// Names the Heads-up for good: one per offset and Occurrence.
    pub fn id(&self) -> String {
        format!(
            "headsUp:{}:{}",
            self.minutes,
            snooze_key(&self.routine_id, &self.occurrence)
        )
    }

    /// Whether it fires without sound or vibration: `at` lies in `quiet`
    /// on `wall`.
    pub fn silent_at(&self, wall: &WallClock) -> bool {
        self.quiet.is_some_and(|q| q.holds(self.at, wall))
    }
}

impl fmt::Display for PlannedHeadsUp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.id(), dart_iso(self.at))?;
        if let Some(quiet) = self.quiet {
            write!(f, " silent in {quiet}")?;
        }
        Ok(())
    }
}

/// When something placed in time fires, as an instant and on the wall.
type Firing = (DateTime<Utc>, WallMoment);

/// `minutes` before `day` at `time` on the calendar, daylight saving left
/// out, so a day before 10:00 is 10:00 the day before, placed on `wall`.
fn timed_before(day: Day, time: ClockTime, minutes: i64, wall: &WallClock) -> Firing {
    let shifted = (utc_at(day, time) - Duration::minutes(minutes)).naive_utc();
    let moment = WallMoment {
        day: Day::of(shifted),
        time: ClockTime {
            hour: shifted.hour(),
            minute: shifted.minute(),
        },
    };
    (instant_of(moment.day, moment.time, wall), moment)
}

/// When something all-day rings on `day`: at `all_day_from`, or once
/// `quiet` ends when they hold that time, as the first ring of an all-day
/// Occurrence in [`alarm_plan`] (#201). The Heads-ups of an all-day
/// Occurrence ring so on their day (obf-1046).
fn all_day_ring(
    day: Day,
    all_day_from: ClockTime,
    quiet: Option<QuietHours>,
    wall: &WallClock,
) -> Firing {
    let at = instant_of(day, all_day_from, wall);
    if let Some(quiet) = quiet {
        let deferred = quiet.defer(at, wall);
        if deferred != at {
            let moment = WallMoment {
                day: Day::of(wall(deferred)),
                time: quiet.end,
            };
            return (deferred, moment);
        }
    }
    (
        at,
        WallMoment {
            day,
            time: all_day_from,
        },
    )
}

/// The Heads-ups that should exist at `now` (obf-1046): for each Routine
/// its offsets ([`Routines::heads_ups`]) before each Occurrence the alarm
/// plan looks at that is neither Done nor skipped nor snoozed, so Done,
/// Skip or a snooze before a Heads-up cancels it. A timed Occurrence's
/// Heads-up is its wall-clock start that many minutes earlier on the
/// calendar; an all-day one takes whole days only, rung on that day as
/// its all-day ring would ring (`all_day_from`, or once `quiet` ends).
/// Each lies after `now` and before its Occurrence rings; at most
/// [`ALARMS_PER_ROUTINE`] per Routine, the soonest first and at most
/// [`ALARM_LIMIT`]. In `quiet` they show without sound, whether the
/// Routine escalates or not. None warns of an Occurrence that starts on
/// one of the `quiet_days`, nor fires on one (obf-1044).
pub fn heads_up_plan(
    routines: &[(Routine, Option<Note>, Vec<i64>)],
    snoozes: &BTreeMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    all_day_from: ClockTime,
    quiet: Option<QuietHours>,
    quiet_days: &QuietDays,
) -> Vec<PlannedHeadsUp> {
    let today = Day::today(now, wall);
    let mut heads_ups = Vec::new();
    for (routine, note, minutes) in routines {
        if minutes.is_empty() {
            continue;
        }
        let mut planned = 0;
        let placed = timed(
            routine,
            today.add_days(-ALARM_HORIZON_DAYS),
            today.add_days(ALARM_HORIZON_DAYS),
            wall,
            all_day_from,
        );
        'occurrences: for t in placed {
            if planned >= ALARMS_PER_ROUTINE {
                break;
            }
            let occurrence = t.occurrence;
            if routine.done_at(&occurrence).is_some()
                || routine.skipped_at(&occurrence).is_some()
                || snoozes.contains_key(&snooze_key(&routine.id, &occurrence))
                || quiet_days.holding(t.start, wall).is_some()
            {
                continue;
            }
            let rings = match occurrence.time {
                Some(_) => t.start,
                None => all_day_ring(occurrence.day, all_day_from, quiet, wall).0,
            };
            for &offset in minutes {
                if planned >= ALARMS_PER_ROUTINE {
                    break 'occurrences;
                }
                // An all-day one takes whole days only.
                if occurrence.time.is_none() && offset % 1440 != 0 {
                    continue;
                }
                let (at, moment) = match occurrence.time {
                    Some(time) => timed_before(occurrence.day, time, offset, wall),
                    None => all_day_ring(
                        occurrence.day.add_days(-(offset / 1440)),
                        all_day_from,
                        quiet,
                        wall,
                    ),
                };
                if at <= now || at >= rings || quiet_days.holding(at, wall).is_some() {
                    continue;
                }
                heads_ups.push(PlannedHeadsUp {
                    routine_id: routine.id.clone(),
                    occurrence,
                    minutes: offset,
                    at,
                    wall: moment,
                    start: start_of(&occurrence, all_day_from),
                    quiet,
                    text: note.as_ref().map(Note::shown_name),
                    wordless: note.as_ref().map(Note::wordless).unwrap_or_default(),
                    wordless_name: note.as_ref().map(Note::wordless_name).unwrap_or_default(),
                });
                planned += 1;
            }
        }
    }
    heads_ups.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| utf16_cmp(&a.id(), &b.id())));
    heads_ups.truncate(ALARM_LIMIT);
    heads_ups
}

/// One Missed Occurrence and when it became Missed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissedOccurrence {
    pub occurrence: Occurrence,
    pub missed_at: DateTime<Utc>,
}

/// What one Routine Missed, the latest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissedRoutine {
    pub routine: Routine,
    pub note: Option<Note>,
    pub missed: Vec<MissedOccurrence>,
}

impl MissedRoutine {
    pub fn count(&self) -> usize {
        self.missed.len()
    }
}

/// The Missed Summary (CONTEXT.md): what was Missed since the user last
/// tapped "seen", by Routine, the one Missed latest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MissedSummary {
    pub routines: Vec<MissedRoutine>,
    /// When the next Occurrence not Done becomes Missed; `None` when none
    /// will.
    pub next_change: Option<DateTime<Utc>>,
}

impl MissedSummary {
    pub fn is_empty(&self) -> bool {
        self.routines.is_empty()
    }

    /// How many Occurrences were Missed in all.
    pub fn count(&self) -> usize {
        self.routines.iter().map(MissedRoutine::count).sum()
    }
}

/// The Missed Summary at `now`: every Occurrence neither Done nor
/// skipped that became Missed after `seen` (or within [`ALARM_HORIZON_DAYS`] when never seen,
/// and never further back than that), by Routine. One that starts on one
/// of the `quiet_days` is never Missed: left undone, it counts as skipped
/// (obf-1044).
pub fn missed_summary(
    routines: &[(Routine, Option<Note>)],
    seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    all_day_from: ClockTime,
    quiet_days: &QuietDays,
) -> MissedSummary {
    let today = Day::today(now, wall);
    let from = today.add_days(-ALARM_HORIZON_DAYS);
    let mut result = Vec::new();
    let mut next_change: Option<DateTime<Utc>> = None;
    for (routine, note) in routines {
        let mut missed = Vec::new();
        for t in timed(routine, from, today, wall, all_day_from) {
            // A skip is a decision, not a lapse (CONTEXT.md, Skip).
            if routine.done_at(&t.occurrence).is_some()
                || routine.skipped_at(&t.occurrence).is_some()
                || quiet_days.holding(t.start, wall).is_some()
            {
                continue;
            }
            if t.ends_at > now {
                if next_change.is_none_or(|n| t.ends_at < n) {
                    next_change = Some(t.ends_at);
                }
                continue;
            }
            if seen.is_some_and(|s| t.ends_at <= s) {
                continue;
            }
            missed.push(MissedOccurrence {
                occurrence: t.occurrence,
                missed_at: t.ends_at,
            });
        }
        if missed.is_empty() {
            continue;
        }
        missed.reverse();
        result.push(MissedRoutine {
            routine: routine.clone(),
            note: note.clone(),
            missed,
        });
    }
    result.sort_by(|a, b| {
        let latest = |r: &MissedRoutine| r.missed.first().map(|m| m.missed_at);
        latest(b)
            .cmp(&latest(a))
            .then_with(|| utf16_cmp(&a.routine.id, &b.routine.id))
    });
    MissedSummary {
        routines: result,
        next_change,
    }
}

/// An Occurrence that asks to be checked off: due now, or `missed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenOccurrence {
    pub routine: Routine,
    pub occurrence: Occurrence,
    pub missed: bool,
}

/// For each of `routines`, in their order, the Occurrence that asks to be
/// checked off at `now` (#215): the one due now, started and not yet
/// Missed, else the latest Missed one, whether the Missed Summary was
/// "seen" or not, within [`ALARM_HORIZON_DAYS`]; none when neither is
/// open. What is Done, from its check-off, its entries or its Lines, asks
/// for nothing, nor does what is skipped. One that starts on one of the
/// `quiet_days` asks while it is due on its day, and is never Missed
/// (obf-1044).
pub fn open_occurrences(
    routines: &[Routine],
    now: DateTime<Utc>,
    wall: &WallClock,
    all_day_from: ClockTime,
    quiet_days: &QuietDays,
) -> Vec<OpenOccurrence> {
    let today = Day::today(now, wall);
    let from = today.add_days(-ALARM_HORIZON_DAYS);
    let mut out = Vec::new();
    for routine in routines {
        let mut due = None;
        let mut missed = None;
        for t in timed(routine, from, today, wall, all_day_from) {
            if routine.done_at(&t.occurrence).is_some()
                || routine.skipped_at(&t.occurrence).is_some()
                || t.start > now
            {
                continue;
            }
            // Left undone on a quiet day, it is skipped, not Missed.
            if let Some(settles) = quiet_days.settles_at(t.start, wall)
                && (t.ends_at <= now || settles <= now)
            {
                continue;
            }
            if t.ends_at > now {
                due = Some(t.occurrence);
            } else {
                missed = Some(t.occurrence);
            }
        }
        if let Some(occurrence) = due.or(missed) {
            out.push(OpenOccurrence {
                routine: routine.clone(),
                occurrence,
                missed: due.is_none(),
            });
        }
    }
    out
}

/// This Device's alarms: its snoozes, the plan over the store, and what a
/// notification's Done and Snooze leave to be applied.
pub struct Alarms<'a> {
    store: &'a Store,
}

const SNOOZES: &str = "snoozes";
const NUDGE_SNOOZE: &str = "nudgeSnooze";

impl<'a> Alarms<'a> {
    pub fn new(store: &'a Store) -> Alarms<'a> {
        Alarms { store }
    }

    /// This Device's snoozes, by [`snooze_key`].
    pub fn snoozes(&self) -> Result<BTreeMap<String, DateTime<Utc>>> {
        let Some(text) = self.store.local_setting(SNOOZES)? else {
            return Ok(BTreeMap::new());
        };
        let Ok(serde_json::Value::Object(raw)) = serde_json::from_str(&text) else {
            return Ok(BTreeMap::new());
        };
        Ok(raw
            .into_iter()
            .filter_map(|(k, v)| Some((k, parse_time(v.as_str()?).ok()?)))
            .collect())
    }

    /// The alarm plan at the store's now, on `wall`, from the log alone:
    /// a Routine an AI Task changes fires as it is kept until the change
    /// is accepted (ADR 0026).
    pub fn plan(&self, wall: &WallClock) -> Result<Vec<PlannedAlarm>> {
        let routines = self
            .store
            .log_only(|| Routines::new(self.store).all_with_notes())?;
        let settings = DeviceSettings::new(self.store);
        let quiet = if settings.quiet_hours_on()? {
            Some(settings.quiet_hours()?)
        } else {
            None
        };
        Ok(alarm_plan(
            &routines,
            &self.snoozes()?,
            self.store.now(),
            wall,
            settings.renag_interval()?,
            settings.all_day_from()?,
            quiet,
            &PersonSettings::new(self.store).quiet_days()?,
        ))
    }

    /// The Heads-ups at the store's now, on `wall` (obf-1046): of each
    /// Routine the offsets the user's own Devices set
    /// ([`Routines::heads_ups`]), with this Device's snoozes, all-day time
    /// and Quiet Hours; from the log alone, as [`Alarms::plan`].
    pub fn heads_ups(&self, wall: &WallClock) -> Result<Vec<PlannedHeadsUp>> {
        let routines = self.store.log_only(|| -> Result<Vec<_>> {
            let read = Routines::new(self.store);
            let mut own = HashMap::new();
            let mut out = Vec::new();
            for (routine, note) in read.all_with_notes()? {
                let minutes = read.heads_ups_by_own(&routine.id, &mut own)?;
                out.push((routine, note, minutes));
            }
            Ok(out)
        })?;
        let settings = DeviceSettings::new(self.store);
        let quiet = if settings.quiet_hours_on()? {
            Some(settings.quiet_hours()?)
        } else {
            None
        };
        Ok(heads_up_plan(
            &routines,
            &self.snoozes()?,
            self.store.now(),
            wall,
            settings.all_day_from()?,
            quiet,
            &PersonSettings::new(self.store).quiet_days()?,
        ))
    }

    /// The Nudge at the store's now over the Unsorted Notes, with this
    /// Device's snooze, repeat interval and Quiet Hours; `None` when
    /// nothing is Unsorted. On a quiet day it waits for the day's end
    /// (obf-1044).
    pub fn nudge(&self, wall: &WallClock) -> Result<Option<PlannedNudge>> {
        let settings = DeviceSettings::new(self.store);
        let quiet = if settings.quiet_hours_on()? {
            Some(settings.quiet_hours()?)
        } else {
            None
        };
        nudge_plan(
            &Notes::new(self.store).nudge_anchors()?,
            self.nudge_snooze()?,
            self.store.now(),
            wall,
            settings.nudge_repeat()?,
            quiet,
            PersonSettings::new(self.store).quiet_until(wall)?,
        )
    }

    /// The Changes Nudge at the store's now (#109): the people changes
    /// travel with by bundles who lack some
    /// ([`Store::waiting_for_changes`]), with the Nudge's repeat interval
    /// and Quiet Hours; `None` when nobody waits, or while sync is paused
    /// ([`Store::sync_paused`]). On a quiet day it waits for the day's end
    /// (obf-1044).
    pub fn changes_nudge(&self, wall: &WallClock) -> Result<Option<PlannedChangesNudge>> {
        if self.store.sync_paused()? {
            return Ok(None);
        }
        let settings = DeviceSettings::new(self.store);
        let quiet = if settings.quiet_hours_on()? {
            Some(settings.quiet_hours()?)
        } else {
            None
        };
        changes_nudge_plan(
            &self.store.waiting_for_changes()?,
            self.store.now(),
            wall,
            settings.nudge_repeat()?,
            quiet,
            PersonSettings::new(self.store).quiet_until(wall)?,
        )
    }

    /// This Device's latest snooze of the Nudge.
    pub fn nudge_snooze(&self) -> Result<Option<NudgeSnooze>> {
        let text = self.store.local_setting(NUDGE_SNOOZE)?.unwrap_or_default();
        let parts: Vec<&str> = text.split(' ').collect();
        let [from, to] = parts[..] else {
            return Ok(None);
        };
        Ok(match (parse_time(from), parse_time(to)) {
            (Ok(at), Ok(until)) => Some(NudgeSnooze { at, until }),
            _ => None,
        })
    }

    /// Applies a snooze of the Nudge tapped at `at`: the Notes Unsorted
    /// then wait until `until`.
    pub fn apply_nudge_snooze(&self, at: DateTime<Utc>, until: DateTime<Utc>) -> Result<()> {
        self.store.set_local_setting(
            NUDGE_SNOOZE,
            &format!("{} {}", dart_iso(at), dart_iso(until)),
        )
    }

    /// The Missed Summary at the store's now, since this Device last
    /// marked it "seen".
    pub fn missed(&self, wall: &WallClock) -> Result<MissedSummary> {
        let routines = Routines::new(self.store).all_with_notes()?;
        let settings = DeviceSettings::new(self.store);
        Ok(missed_summary(
            &routines,
            settings.missed_seen()?,
            self.store.now(),
            wall,
            settings.all_day_from()?,
            &PersonSettings::new(self.store).quiet_days()?,
        ))
    }

    /// What the Note `note_id` asks to check off at the store's now
    /// ([`open_occurrences`]): of each of its Routines and Reminders the
    /// Occurrence due, else the latest Missed one.
    pub fn open_of(&self, note_id: &str, wall: &WallClock) -> Result<Vec<OpenOccurrence>> {
        let routines = Routines::new(self.store).of_note(note_id)?;
        let settings = DeviceSettings::new(self.store);
        Ok(open_occurrences(
            &routines,
            self.store.now(),
            wall,
            settings.all_day_from()?,
            &PersonSettings::new(self.store).quiet_days()?,
        ))
    }

    /// The user tapped "seen": the Missed Summary starts over from now.
    pub fn mark_seen(&self) -> Result<()> {
        DeviceSettings::new(self.store).set_missed_seen(Some(self.store.now()))
    }

    /// The Occurrences Done or skipped from yesterday to tomorrow on
    /// `wall`, by Routine id, and those starting on a quiet day
    /// (obf-1044): their notifications, if still showing, can go.
    pub fn recently_done(&self, wall: &WallClock) -> Result<Vec<(String, Occurrence)>> {
        let today = Day::today(self.store.now(), wall);
        let quiet_days = PersonSettings::new(self.store).quiet_days()?;
        let all_day = DeviceSettings::new(self.store).all_day_from()?;
        let quieted = |occurrence: &Occurrence| {
            let start = start_of(occurrence, all_day);
            quiet_days
                .holding(instant_of(start.day, start.time, wall), wall)
                .is_some()
        };
        let mut out = Vec::new();
        for routine in Routines::new(self.store).all()? {
            for occurrence in routine.between(today.add_days(-1), today.add_days(1)) {
                if routine.done_at(&occurrence).is_some()
                    || routine.skipped_at(&occurrence).is_some()
                    || quieted(&occurrence)
                {
                    out.push((routine.id.clone(), occurrence));
                }
            }
        }
        Ok(out)
    }

    /// Applies a Done tapped on a notification at `at`. Returns whether
    /// it changed anything; a Routine trashed or rescheduled meanwhile,
    /// or an Occurrence already Done, is left as it is.
    pub fn apply_done(
        &self,
        routine_id: &str,
        occurrence_key: &str,
        at: DateTime<Utc>,
    ) -> Result<bool> {
        let routines = Routines::new(self.store);
        let (Some(occurrence), Some(routine)) = (
            Occurrence::parse(occurrence_key),
            routines.by_id(routine_id)?,
        ) else {
            return Ok(false);
        };
        if routine.is_trashed() || routine.done_at(&occurrence).is_some() {
            return Ok(false);
        }
        match routines.check_off(routine_id, &occurrence, Some(at)) {
            Ok(_) => Ok(true),
            Err(Error::Invalid(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Applies a Skip tapped on a notification at `at`
    /// ([`Routines::skip`]). Returns whether it changed anything; a
    /// Routine trashed or rescheduled meanwhile, or an Occurrence already
    /// Done or skipped, is left as it is.
    pub fn apply_skip(
        &self,
        routine_id: &str,
        occurrence_key: &str,
        at: DateTime<Utc>,
    ) -> Result<bool> {
        self.apply_not_today(routine_id, occurrence_key, at, false)
    }

    /// Applies a Tomorrow tapped on a notification at `at`
    /// ([`Routines::tomorrow`]), as [`Alarms::apply_skip`] does.
    pub fn apply_tomorrow(
        &self,
        routine_id: &str,
        occurrence_key: &str,
        at: DateTime<Utc>,
    ) -> Result<bool> {
        self.apply_not_today(routine_id, occurrence_key, at, true)
    }

    fn apply_not_today(
        &self,
        routine_id: &str,
        occurrence_key: &str,
        at: DateTime<Utc>,
        tomorrow: bool,
    ) -> Result<bool> {
        let routines = Routines::new(self.store);
        let (Some(occurrence), Some(routine)) = (
            Occurrence::parse(occurrence_key),
            routines.by_id(routine_id)?,
        ) else {
            return Ok(false);
        };
        if routine.is_trashed()
            || routine.done_at(&occurrence).is_some()
            || routine.skipped_at(&occurrence).is_some()
            || !routine.has(&occurrence)
            || !self.store.may_check_off(routine_id)?
        {
            return Ok(false);
        }
        if tomorrow {
            routines.tomorrow(routine_id, &occurrence, Some(at))?;
        } else {
            routines.skip(routine_id, &occurrence, Some(at))?;
        }
        Ok(true)
    }

    /// Applies a Snooze tapped on a notification: the Occurrence fires
    /// again at `until` and Re-nags from there. Snoozes over for longer
    /// than the plan looks back are dropped on the way.
    pub fn apply_snooze(
        &self,
        routine_id: &str,
        occurrence_key: &str,
        until: DateTime<Utc>,
    ) -> Result<()> {
        let Some(occurrence) = Occurrence::parse(occurrence_key) else {
            return Ok(());
        };
        let oldest = self.store.now() - Duration::days(ALARM_HORIZON_DAYS + 1);
        let mut kept: BTreeMap<String, DateTime<Utc>> = self
            .snoozes()?
            .into_iter()
            .filter(|(_, v)| *v > oldest)
            .collect();
        kept.insert(snooze_key(routine_id, &occurrence), until);
        let json: serde_json::Map<String, serde_json::Value> = kept
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(dart_iso(v))))
            .collect();
        self.store
            .set_local_setting(SNOOZES, &serde_json::Value::Object(json).to_string())
    }
}

#[cfg(test)]
mod tests;
