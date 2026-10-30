//! The alarm plan (ADR 0004) as the Dart core's `alarms.dart` has it:
//! which alarms should exist on this Device, as a pure function of the
//! Routines, now, the wall clock and this Device's snoozes; the Missed
//! Summary; and this Device's snoozes and "seen" mark in its local
//! settings, never synced (ADR 0008).

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::notes::{Note, Notes, Wordless};
use crate::nudges::{
    NudgeSnooze, PlannedChangesNudge, PlannedNudge, changes_nudge_plan, nudge_plan,
};
use crate::routines::{Routine, Routines};
use crate::schedule::{ClockTime, Day, Occurrence, WallClock};
use crate::settings::DeviceSettings;
use crate::store::{Store, utf16_cmp};
use crate::{Error, Result};

/// When an all-day Occurrence's alarm fires, on the wall clock.
pub const ALL_DAY_ALARM_TIME: ClockTime = ClockTime { hour: 9, minute: 0 };

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
/// [`ALL_DAY_ALARM_TIME`].
pub fn start_of(occurrence: &Occurrence) -> WallMoment {
    WallMoment {
        day: occurrence.day,
        time: occurrence.time.unwrap_or(ALL_DAY_ALARM_TIME),
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
/// (due at or after when it was scheduled), placed on `wall`.
fn timed(routine: &Routine, from: Day, to: Day, wall: &WallClock) -> Vec<Timed> {
    let schedule = &routine.schedule;
    let occurrences = schedule.between(from, to);
    let after = if schedule.once() {
        None
    } else {
        schedule.next(to.add_days(1), 1).into_iter().next()
    };
    let mut out = Vec::new();
    for (i, occurrence) in occurrences.iter().enumerate() {
        let next = occurrences.get(i + 1).or(after.as_ref());
        let start = start_of(occurrence);
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
/// [`PlannedAlarm::escalates_at`] on is escalated.
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
    /// The text of the Note the Routine links back to as it reads without
    /// its marks ([`Note::visible_text`]); `None` when gone.
    pub text: Option<String>,
    /// What that Note is titled as while `text` is blank
    /// ([`Note::wordless`]).
    pub wordless: Wordless,
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
}

/// The key a snooze of `occurrence` of the Routine `routine_id` is kept
/// under.
pub fn snooze_key(routine_id: &str, occurrence: &Occurrence) -> String {
    format!("{routine_id}@{}", occurrence.key())
}

/// The alarms that should exist at `now`: for each Routine its
/// Occurrences that are not Done and not yet Missed, from
/// [`ALARM_HORIZON_DAYS`] back to as far ahead, at most
/// [`ALARMS_PER_ROUTINE`], the soonest first and at most [`ALARM_LIMIT`].
/// A snoozed one rings at its snooze instead; a snooze reaching past when
/// the Occurrence counts as Missed is dropped. Each Re-nags at its
/// Routine's interval, else at `renag`.
pub fn alarm_plan(
    routines: &[(Routine, Option<Note>)],
    snoozes: &BTreeMap<String, DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
    renag: Duration,
) -> Vec<PlannedAlarm> {
    let today = Day::today(now, wall);
    let mut alarms = Vec::new();
    for (routine, note) in routines {
        let mut planned = 0;
        let placed = timed(
            routine,
            today.add_days(-ALARM_HORIZON_DAYS),
            today.add_days(ALARM_HORIZON_DAYS),
            wall,
        );
        for t in placed {
            if planned >= ALARMS_PER_ROUTINE {
                break;
            }
            let occurrence = t.occurrence;
            if routine.done_at(&occurrence).is_some() || t.ends_at <= now {
                continue;
            }
            let snoozed = snoozes.get(&snooze_key(&routine.id, &occurrence)).copied();
            if snoozed.is_some_and(|s| s >= t.ends_at) {
                continue;
            }
            alarms.push(PlannedAlarm {
                kind: if snoozed.is_some() {
                    AlarmKind::Snooze
                } else {
                    AlarmKind::Occurrence
                },
                routine_id: routine.id.clone(),
                occurrence,
                at: snoozed.unwrap_or(t.start),
                wall: snoozed.is_none().then(|| start_of(&occurrence)),
                renag: routine.renag.unwrap_or(renag),
                escalation: routine.escalation,
                ends: t.ends,
                ends_at: t.ends_at,
                text: note.as_ref().map(Note::visible_text),
                wordless: note.as_ref().map(Note::wordless).unwrap_or_default(),
            });
            planned += 1;
        }
    }
    alarms.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| utf16_cmp(&a.id(), &b.id())));
    alarms.truncate(ALARM_LIMIT);
    alarms
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

/// The Missed Summary at `now`: every Occurrence not Done that became
/// Missed after `seen` (or within [`ALARM_HORIZON_DAYS`] when never seen,
/// and never further back than that), by Routine.
pub fn missed_summary(
    routines: &[(Routine, Option<Note>)],
    seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    wall: &WallClock,
) -> MissedSummary {
    let today = Day::today(now, wall);
    let from = today.add_days(-ALARM_HORIZON_DAYS);
    let mut result = Vec::new();
    let mut next_change: Option<DateTime<Utc>> = None;
    for (routine, note) in routines {
        let mut missed = Vec::new();
        for t in timed(routine, from, today, wall) {
            if routine.done_at(&t.occurrence).is_some() {
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
        let renag = DeviceSettings::new(self.store).renag_interval()?;
        Ok(alarm_plan(
            &routines,
            &self.snoozes()?,
            self.store.now(),
            wall,
            renag,
        ))
    }

    /// The Nudge at the store's now over the Unsorted Notes, with this
    /// Device's snooze, repeat interval and Quiet Hours; `None` when
    /// nothing is Unsorted.
    pub fn nudge(&self, wall: &WallClock) -> Result<Option<PlannedNudge>> {
        let settings = DeviceSettings::new(self.store);
        let quiet = if settings.quiet_hours_on()? {
            Some(settings.quiet_hours()?)
        } else {
            None
        };
        nudge_plan(
            &Notes::new(self.store).unsorted()?,
            self.nudge_snooze()?,
            self.store.now(),
            wall,
            settings.nudge_repeat()?,
            quiet,
        )
    }

    /// The Changes Nudge at the store's now (#109): the people changes
    /// travel with by bundles who lack some
    /// ([`Store::waiting_for_changes`]), with the Nudge's repeat interval
    /// and Quiet Hours; `None` when nobody waits, or while sync is paused
    /// ([`Store::sync_paused`]).
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
        let seen = DeviceSettings::new(self.store).missed_seen()?;
        Ok(missed_summary(&routines, seen, self.store.now(), wall))
    }

    /// The user tapped "seen": the Missed Summary starts over from now.
    pub fn mark_seen(&self) -> Result<()> {
        DeviceSettings::new(self.store).set_missed_seen(Some(self.store.now()))
    }

    /// The Occurrences Done from yesterday to tomorrow on `wall`, by
    /// Routine id: their notifications, if still showing, can go.
    pub fn recently_done(&self, wall: &WallClock) -> Result<Vec<(String, Occurrence)>> {
        let today = Day::today(self.store.now(), wall);
        let mut out = Vec::new();
        for routine in Routines::new(self.store).all()? {
            for occurrence in routine
                .schedule
                .between(today.add_days(-1), today.add_days(1))
            {
                if routine.done_at(&occurrence).is_some() {
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
        match routines.mark_done(routine_id, &occurrence, Some(at)) {
            Ok(()) => Ok(true),
            Err(Error::Invalid(_)) => Ok(false),
            Err(e) => Err(e),
        }
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
mod tests {
    use super::*;

    fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
        utc.naive_utc()
    }

    fn alarm(at: &str, renag: i64, ends: &str) -> Result<PlannedAlarm> {
        let moment = WallMoment {
            day: Day::new(2026, 1, 1),
            time: ALL_DAY_ALARM_TIME,
        };
        Ok(PlannedAlarm {
            kind: AlarmKind::Occurrence,
            routine_id: "routine:a".into(),
            occurrence: Occurrence::new(Day::new(2026, 1, 1), None),
            at: parse_time(at)?,
            wall: Some(moment),
            renag: Duration::minutes(renag),
            escalation: Some(2),
            ends: moment,
            ends_at: parse_time(ends)?,
            text: None,
            wordless: Wordless::Photo,
        })
    }

    #[test]
    fn an_alarm_renags_until_it_counts_as_missed() -> Result<()> {
        let a = alarm("2026-01-01T09:00:00.000Z", 10, "2026-01-01T09:35:00.000Z")?;
        let t = |s: &str| parse_time(s);
        assert_eq!(a.fires_after(t("2026-01-01T08:00:00.000Z")?), Some(a.at));
        assert_eq!(
            a.fires_after(t("2026-01-01T09:00:00.000Z")?),
            Some(t("2026-01-01T09:10:00.000Z")?)
        );
        assert_eq!(a.fires_after(t("2026-01-01T09:30:00.000Z")?), None);
        assert_eq!(a.escalates_at(), Some(t("2026-01-01T09:30:00.000Z")?));
        Ok(())
    }

    #[test]
    fn an_alarm_names_what_its_note_without_words_holds() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let routines = Routines::new(&store);
        let memo = notes.capture_with("", None, None, &[], &[vec![0x52, 0x49, 0x46, 0x46]])?;
        let receipt = notes.capture_with("", None, None, &[vec![0xFF, 0xD8, 0xFF, 0xD9]], &[])?;
        let ten = ClockTime::new(10, 0).into_iter().collect::<Vec<_>>();
        let schedule = crate::schedule::Schedule::new(Day::new(2026, 1, 2), &ten, None)
            .ok_or_else(|| Error::Invalid("a schedule".into()))?;
        let voice = routines.spawn(&memo.id, &schedule)?.id;
        let photo = routines.spawn(&receipt.id, &schedule)?.id;
        let titled: BTreeMap<String, Wordless> = Alarms::new(&store)
            .plan(&utc_wall)?
            .into_iter()
            .map(|a| (a.routine_id, a.wordless))
            .collect();
        assert_eq!(
            titled,
            BTreeMap::from([(voice, Wordless::VoiceRecording), (photo, Wordless::Photo)])
        );
        Ok(())
    }

    #[test]
    fn an_alarm_names_a_formatted_note_as_it_reads() -> Result<()> {
        let store = crate::store::tests::store();
        let note = Notes::new(&store).capture_as(
            "**pills** ~~now~~",
            crate::text_format::MARKDOWN,
            None,
            None,
        )?;
        let ten = ClockTime::new(10, 0).into_iter().collect::<Vec<_>>();
        let schedule = crate::schedule::Schedule::new(Day::new(2026, 1, 2), &ten, None)
            .ok_or_else(|| Error::Invalid("a schedule".into()))?;
        Routines::new(&store).spawn(&note.id, &schedule)?;
        let texts: Vec<Option<String>> = Alarms::new(&store)
            .plan(&utc_wall)?
            .into_iter()
            .map(|a| a.text)
            .collect();
        assert_eq!(texts.first(), Some(&Some("pills now".to_string())));
        Ok(())
    }

    #[test]
    fn a_skipped_time_fires_when_the_clock_jumps() {
        let day = Day::new(2026, 3, 29);
        let half_past_two = ClockTime {
            hour: 2,
            minute: 30,
        };
        assert_eq!(
            instant_of(day, half_past_two, &utc_wall),
            utc_at(day, half_past_two)
        );
    }

    #[test]
    fn snoozes_are_kept_on_this_device() -> Result<()> {
        let store = crate::store::tests::store();
        let alarms = Alarms::new(&store);
        let until = parse_time("2026-01-01T11:00:00.000Z")?;
        alarms.apply_snooze("routine:a", "2026-01-01T10:00", until)?;
        alarms.apply_snooze("routine:a", "not a key", until)?;
        assert_eq!(
            alarms.snoozes()?.into_iter().collect::<Vec<_>>(),
            vec![("routine:a@2026-01-01T10:00".to_string(), until)]
        );
        alarms.apply_nudge_snooze(until, until)?;
        assert_eq!(
            alarms.nudge_snooze()?,
            Some(NudgeSnooze { at: until, until })
        );
        Ok(())
    }
}
