//! The Nudge (CONTEXT.md) as the Dart core's `nudges.dart` plans it: one
//! reminder covering every Unsorted Note, as a pure function of the
//! Unsorted Notes, now, the wall clock and this Device's snooze, Quiet
//! Hours and repeat interval.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Duration, Timelike, Utc};

use crate::alarms::instant_of;
use crate::notes::Note;
use crate::schedule::{ClockTime, Day, WallClock};
use crate::{Error, Result};

/// A daily window in which Nudges wait (CONTEXT.md). It runs over
/// midnight when `start` is later than `end`; with both the same it is
/// empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuietHours {
    pub start: ClockTime,
    pub end: ClockTime,
}

impl QuietHours {
    /// Whether `time` lies in the window: from `start`, up to `end`.
    pub fn contains(&self, time: ClockTime) -> bool {
        let (s, e, t) = (minutes(self.start), minutes(self.end), minutes(time));
        if s <= e {
            s <= t && t < e
        } else {
            t >= s || t < e
        }
    }

    /// When a Nudge due at `instant` fires: at `instant`, or at the end of
    /// the window `instant` lies in, on `wall`.
    pub fn defer(&self, instant: DateTime<Utc>, wall: &WallClock) -> DateTime<Utc> {
        let shown = wall(instant);
        let time = ClockTime {
            hour: shown.hour(),
            minute: shown.minute(),
        };
        if !self.contains(time) {
            return instant;
        }
        // In a window over midnight, before midnight it ends tomorrow.
        let tomorrow = minutes(self.start) > minutes(self.end) && time >= self.start;
        let day = Day::of(shown).add_days(i64::from(tomorrow));
        let ends = instant_of(day, self.end, wall);
        if ends > instant { ends } else { instant }
    }
}

impl fmt::Display for QuietHours {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

fn minutes(t: ClockTime) -> u32 {
    t.hour * 60 + t.minute
}

/// A snooze of the Nudge, tapped at `at` for the Notes Unsorted then: it
/// fires again at `until`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NudgeSnooze {
    pub at: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

/// One Unsorted Note's own Nudge time, and when it was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NudgeAnchor {
    pub due: DateTime<Utc>,
    pub captured: DateTime<Utc>,
}

/// The one Nudge that should exist while Notes are Unsorted: it fires
/// first at the earliest own time of the Unsorted Notes and covers them
/// all; while ignored it comes again every `repeat` after it last fired.
/// A snooze holds back the Notes Unsorted when it was tapped until it
/// ends; a firing in Quiet Hours waits for their end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedNudge {
    /// The Unsorted Notes, oldest first.
    pub notes: Vec<Note>,
    pub anchors: Vec<NudgeAnchor>,
    pub snooze: Option<NudgeSnooze>,
    pub repeat: Duration,
    /// This Device's Quiet Hours; `None` when they are off.
    pub quiet: Option<QuietHours>,
    /// When it last fired up to the plan's now; `None` when it has not.
    pub since: Option<DateTime<Utc>>,
    /// When it fires next after the plan's now.
    pub at: DateTime<Utc>,
}

impl PlannedNudge {
    /// How many Notes are Unsorted.
    pub fn count(&self) -> usize {
        self.notes.len()
    }

    /// When it fires first after `after`, on `wall`.
    pub fn fires_after(&self, after: DateTime<Utc>, wall: &WallClock) -> DateTime<Utc> {
        let from = self.since.filter(|from| after >= *from);
        self.rule().firings(from, after, wall).1
    }

    fn rule(&self) -> Rule<'_> {
        Rule {
            anchors: &self.anchors,
            snooze: self.snooze,
            repeat: self.repeat,
            quiet: self.quiet,
        }
    }
}

/// How a Nudge fires over its anchors, as the Dart core's `_firings`:
/// shared by the Nudge and the Changes Nudge.
struct Rule<'a> {
    anchors: &'a [NudgeAnchor],
    snooze: Option<NudgeSnooze>,
    repeat: Duration,
    quiet: Option<QuietHours>,
}

impl Rule<'_> {
    /// Follows the Nudge from `from`, when it fired (`None`: from the
    /// start), to its last firing up to `after` and the next one after.
    fn firings(
        &self,
        from: Option<DateTime<Utc>>,
        after: DateTime<Utc>,
        wall: &WallClock,
    ) -> (Option<DateTime<Utc>>, DateTime<Utc>) {
        let mut last = from;
        loop {
            // Never `None`: there is an anchor, and each firing is followed
            // by a repeat.
            let next = self.next_firing(last, wall).unwrap_or(after + self.repeat);
            if next > after {
                return (last, next);
            }
            last = Some(next);
        }
    }

    /// When the Nudge fires next after it last fired at `fired`.
    fn next_firing(&self, fired: Option<DateTime<Utc>>, wall: &WallClock) -> Option<DateTime<Utc>> {
        fn earliest(times: impl IntoIterator<Item = DateTime<Utc>>) -> Option<DateTime<Utc>> {
            times.into_iter().min()
        }
        // The own times of the Notes the last firing did not cover.
        let own_after = |from: Option<DateTime<Utc>>| {
            earliest(
                self.anchors
                    .iter()
                    .filter(|a| from.is_none_or(|f| a.captured > f))
                    .filter(|a| fired.is_none_or(|f| a.due > f))
                    .map(|a| a.due),
            )
        };
        let mut raw = earliest(
            fired
                .map(|f| f + self.repeat)
                .into_iter()
                .chain(own_after(fired)),
        );
        if let Some(NudgeSnooze { at: tapped, until }) = self.snooze {
            if fired.is_none_or(|f| f < tapped) {
                // From the tap to its end only Notes captured after it fire.
                if raw.is_some_and(|r| r >= tapped) {
                    raw = earliest(std::iter::once(until).chain(own_after(Some(tapped))));
                }
            } else if fired.is_some_and(|f| f < until) {
                raw = earliest(raw.into_iter().chain(std::iter::once(until)));
            }
        }
        let raw = raw?;
        Some(match &self.quiet {
            Some(quiet) => quiet.defer(raw, wall),
            None => raw,
        })
    }
}

/// The Nudge at `now` over the `unsorted` Notes, `None` when there are
/// none. It repeats every `repeat` while ignored and waits out `quiet`
/// (`None` when Quiet Hours are off); `snooze` is this Device's latest.
pub fn nudge_plan(
    unsorted: &[Note],
    snooze: Option<NudgeSnooze>,
    now: DateTime<Utc>,
    wall: &WallClock,
    repeat: Duration,
    quiet: Option<QuietHours>,
) -> Result<Option<PlannedNudge>> {
    if unsorted.is_empty() {
        return Ok(None);
    }
    if repeat.num_minutes() < 1 {
        return Err(Error::Invalid(
            "a Nudge repeats after a minute at least".into(),
        ));
    }
    let mut notes = unsorted.to_vec();
    notes.sort_by_key(|n| n.captured);
    let anchors = notes
        .iter()
        .map(|n| NudgeAnchor {
            due: n.nudge_at(),
            captured: n.captured,
        })
        .collect();
    let mut plan = PlannedNudge {
        notes,
        anchors,
        snooze,
        repeat,
        quiet,
        since: None,
        at: now,
    };
    let (since, at) = plan.rule().firings(None, now, wall);
    plan.since = since;
    plan.at = at;
    Ok(Some(plan))
}

/// How long after the first change a person lacks the Changes Nudge
/// waits, so several changes travel in one bundle (#109).
pub const CHANGES_NUDGE_DELAY_MINUTES: i64 = 30;

/// The one Changes Nudge (CONTEXT.md, #109) that should exist while
/// people synced by bundles lack changes, as the Dart core's
/// `PlannedChangesNudge`: it fires first 30 minutes after the earliest
/// first unsent change of the people waiting and covers them all; while
/// they wait it comes again every `repeat` after it last fired, as the
/// Nudge does, and a firing in Quiet Hours waits for their end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedChangesNudge {
    /// The people waiting, by Profile id: by their first unsent change,
    /// then by id.
    pub people: Vec<String>,
    /// Their own times, in the same order: due 30 minutes after the first
    /// unsent change, captured at it.
    pub anchors: Vec<NudgeAnchor>,
    pub repeat: Duration,
    /// This Device's Quiet Hours; `None` when they are off.
    pub quiet: Option<QuietHours>,
    /// When it last fired up to the plan's now; `None` when it has not.
    pub since: Option<DateTime<Utc>>,
    /// When it fires next after the plan's now.
    pub at: DateTime<Utc>,
}

impl PlannedChangesNudge {
    /// When it fires first after `after`, on `wall`.
    pub fn fires_after(&self, after: DateTime<Utc>, wall: &WallClock) -> DateTime<Utc> {
        let from = self.since.filter(|from| after >= *from);
        self.rule().firings(from, after, wall).1
    }

    fn rule(&self) -> Rule<'_> {
        Rule {
            anchors: &self.anchors,
            snooze: None,
            repeat: self.repeat,
            quiet: self.quiet,
        }
    }
}

/// The Changes Nudge at `now` for the people `waiting`, by Profile id
/// with the time of the first change each lacks, as the Dart core's
/// `changesNudgePlan`; `None` when nobody waits.
pub fn changes_nudge_plan(
    waiting: &BTreeMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    repeat: Duration,
    quiet: Option<QuietHours>,
) -> Result<Option<PlannedChangesNudge>> {
    if waiting.is_empty() {
        return Ok(None);
    }
    if repeat.num_minutes() < 1 {
        return Err(Error::Invalid(
            "a Nudge repeats after a minute at least".into(),
        ));
    }
    let mut firsts: Vec<(&String, DateTime<Utc>)> = waiting.iter().map(|(p, t)| (p, *t)).collect();
    firsts.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)));
    let delay = Duration::minutes(CHANGES_NUDGE_DELAY_MINUTES);
    let mut plan = PlannedChangesNudge {
        people: firsts.iter().map(|(p, _)| (*p).clone()).collect(),
        anchors: firsts
            .iter()
            .map(|(_, first)| NudgeAnchor {
                due: *first + delay,
                captured: *first,
            })
            .collect(),
        repeat,
        quiet,
        since: None,
        at: now,
    };
    let (since, at) = plan.rule().firings(None, now, wall);
    plan.since = since;
    plan.at = at;
    Ok(Some(plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
        utc.naive_utc()
    }

    fn at(h: u32, m: u32) -> ClockTime {
        ClockTime { hour: h, minute: m }
    }

    #[test]
    fn quiet_hours_over_midnight_hold_a_nudge_until_they_end() -> Result<()> {
        let quiet = QuietHours {
            start: at(22, 0),
            end: at(8, 0),
        };
        assert!(quiet.contains(at(23, 0)));
        assert!(quiet.contains(at(7, 59)));
        assert!(!quiet.contains(at(8, 0)));
        let late = crate::parse_time("2026-01-01T23:00:00.000Z")?;
        let morning = crate::parse_time("2026-01-02T08:00:00.000Z")?;
        assert_eq!(quiet.defer(late, &utc_wall), morning);
        assert_eq!(quiet.defer(morning, &utc_wall), morning);
        Ok(())
    }
}
