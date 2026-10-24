//! The desktop's alarm layer: what the phone's Kotlin `Scheduler` does
//! with the core's alarm plan, done while the app runs, also with the
//! window closed into the tray. Every tick shows what came due since the
//! last one as a notification (an Occurrence, its Re-nags, a snooze, the
//! Nudge), escalated ones urgent and with sound, and closes the
//! notifications of Occurrences Done meanwhile. What became Missed while
//! the app was not running is told in one Catch-up Notification, never
//! rung late. Done and the snoozes tapped on a notification come back as
//! [`Tap`]s and are applied to the log. Quiet Hours only hold back the
//! Nudge; with alerts off on this Device nothing shows (ADR 0008).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};

use chrono::{DateTime, Duration, Utc};
use oldbrainfix_core::alarms::{PlannedAlarm, alarm_plan};
use oldbrainfix_core::{
    Alarms, DeviceSettings, Occurrence, PlannedNudge, Routines, Store, WallClock,
};

use crate::l10n::L10n;
use crate::schedule_text::occurrence_time;

/// An alarm that came due this long before a tick still shows; older
/// ones wait for their next Re-nag (the phone's `GRACE_MS`).
pub fn grace() -> Duration {
    Duration::seconds(60)
}

/// How long a tick waits at most before it looks again.
pub fn longest_wait() -> Duration {
    Duration::seconds(30)
}

/// One notification to show, or to show again in place of the one with
/// the same key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// `routine:…@2026-10-25T08:00`, [`NUDGE_KEY`] or [`CATCH_UP_KEY`].
    pub key: String,
    pub title: String,
    pub body: String,
    /// Escalated: urgent and with sound.
    pub urgent: bool,
    /// The buttons it offers, by id ([`DONE`], [`SNOOZE_10`], ...) and
    /// label.
    pub actions: Vec<(String, String)>,
}

pub const NUDGE_KEY: &str = "nudge";
pub const CATCH_UP_KEY: &str = "catch-up";

/// The ids of a notification's buttons; [`OPEN`] is a click on the
/// notification itself.
pub const DONE: &str = "done";
pub const SNOOZE_10: &str = "snooze10";
pub const SNOOZE_30: &str = "snooze30";
pub const SNOOZE_60: &str = "snooze60";
pub const OPEN: &str = "default";

/// What a snooze button waits.
pub fn snooze_of(action: &str) -> Option<Duration> {
    match action {
        SNOOZE_10 => Some(Duration::minutes(10)),
        SNOOZE_30 => Some(Duration::minutes(30)),
        SNOOZE_60 => Some(Duration::hours(1)),
        _ => None,
    }
}

/// A button of a notification, tapped at `at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tap {
    pub key: String,
    pub action: String,
    pub at: DateTime<Utc>,
}

/// What reaches the alerts between ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Tap(Tap),
    /// Something changed in the log or the settings: look again now.
    Wake,
}

/// Shows and closes notifications.
pub trait Notifier: Send {
    fn show(&mut self, notice: &Notice);
    fn close(&mut self, key: &str);
}

/// When `alarm` fires escalated at `now`: its first ring and
/// `escalation` Re-nags went ignored and it has not stopped (the phone's
/// `AlarmPlan.escalates`, the core's `escalates_at`).
pub fn escalates(alarm: &PlannedAlarm, now: DateTime<Utc>) -> bool {
    alarm
        .escalates_at()
        .is_some_and(|first| now >= first && now < alarm.ends_at)
}

/// The alarms of `plan` due from `after` up to `now`, each Occurrence
/// once however many of its Re-nags came due meanwhile.
pub fn due(plan: &[PlannedAlarm], after: DateTime<Utc>, now: DateTime<Utc>) -> Vec<&PlannedAlarm> {
    let mut seen = BTreeSet::new();
    plan.iter()
        .filter(|a| a.fires_after(after).is_some_and(|at| at <= now))
        .filter(|a| seen.insert((a.routine_id.clone(), a.occurrence)))
        .collect()
}

/// When the next alarm of `plan` after `after` fires; `None` when none
/// is left.
pub fn next(plan: &[PlannedAlarm], after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    plan.iter().filter_map(|a| a.fires_after(after)).min()
}

/// How many Occurrences of the plan as it stood at `since` became Missed
/// by `now` with a ring or Re-nag due after `since` that never came: what
/// was Missed while the app was not running (the phone's
/// `AlarmPlan.catchUp`).
pub fn catch_up(plan_then: &[PlannedAlarm], since: DateTime<Utc>, now: DateTime<Utc>) -> usize {
    if since > now {
        return 0;
    }
    plan_then
        .iter()
        .filter(|a| a.ends_at <= now && a.fires_after(since).is_some())
        .map(|a| (a.routine_id.clone(), a.occurrence))
        .collect::<BTreeSet<_>>()
        .len()
}

/// The key a notification of `occurrence` of the Routine `routine_id`
/// goes by.
pub fn occurrence_key(routine_id: &str, occurrence: &Occurrence) -> String {
    oldbrainfix_core::alarms::snooze_key(routine_id, occurrence)
}

/// The Routine and Occurrence a notification's key names.
fn split_key(key: &str) -> Option<(&str, &str)> {
    key.rsplit_once('@')
}

/// The first line of a Note's text, or `None` when there is none.
fn first_line(text: Option<&str>) -> Option<String> {
    let line = text?.trim().lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_string())
}

/// The alarm layer's state between ticks.
pub struct Alerts {
    notifier: Box<dyn Notifier>,
    t: L10n,
    /// When the last tick looked; what came due before is shown.
    last: Option<DateTime<Utc>>,
    /// The notifications showing, by key.
    shown: BTreeSet<String>,
    /// Bumped whenever a tap changed the log, for the window to read
    /// again.
    changed: Arc<AtomicU64>,
    /// Asked for by a click on a notification: the window is to open.
    pub open_requested: bool,
}

impl Alerts {
    pub fn new(notifier: Box<dyn Notifier>, t: L10n, changed: Arc<AtomicU64>) -> Alerts {
        Alerts {
            notifier,
            t,
            last: None,
            shown: BTreeSet::new(),
            changed,
            open_requested: false,
        }
    }

    /// Applies one tap to the log and this Device's snoozes.
    pub fn apply(&mut self, store: &Store, tap: &Tap) -> oldbrainfix_core::Result<()> {
        if tap.action == OPEN {
            self.open_requested = true;
            return Ok(());
        }
        let alarms = Alarms::new(store);
        if tap.key == NUDGE_KEY {
            if let Some(wait) = snooze_of(&tap.action) {
                alarms.apply_nudge_snooze(tap.at, tap.at + wait)?;
                self.notifier.close(NUDGE_KEY);
                self.shown.remove(NUDGE_KEY);
            }
            return Ok(());
        }
        let Some((routine, occurrence)) = split_key(&tap.key) else {
            return Ok(());
        };
        if tap.action == DONE {
            alarms.apply_done(routine, occurrence, tap.at)?;
        } else if let Some(wait) = snooze_of(&tap.action) {
            alarms.apply_snooze(routine, occurrence, tap.at + wait)?;
        } else {
            return Ok(());
        }
        self.notifier.close(&tap.key);
        self.shown.remove(&tap.key);
        self.changed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Shows what came due since the last tick, at the store's now, and
    /// returns when to look next.
    pub fn tick(
        &mut self,
        store: &Store,
        wall: &WallClock,
    ) -> oldbrainfix_core::Result<DateTime<Utc>> {
        let now = store.now();
        let settings = DeviceSettings::new(store);
        let first = self.last.is_none();
        let last = self.last.replace(now);
        if !settings.alerts_on()? {
            // Nothing piles up to ring once they are on again.
            for key in std::mem::take(&mut self.shown) {
                self.notifier.close(&key);
            }
            settings.set_last_fired(now)?;
            return Ok(now + longest_wait());
        }
        let alarms = Alarms::new(store);
        let plan = alarms.plan(wall)?;
        let nudge = alarms.nudge(wall)?;
        let fired = settings.last_fired()?;
        // After a start, or a sleep of the computer while running.
        let slept = last.filter(|l| now - *l > longest_wait() * 4);
        if let Some(since) = slept.or(fired.filter(|_| first)) {
            self.catch_up(store, wall, since, now)?;
        }
        // From the last tick, or a short grace before now after a start.
        let after = match last {
            Some(last) if last <= now => last.max(now - grace()),
            _ => now - grace(),
        };
        let after = fired.filter(|f| *f <= now).map_or(after, |f| after.max(f));
        let mut showed = false;
        for alarm in due(&plan, after, now) {
            self.show_alarm(alarm, now);
            showed = true;
        }
        if let Some(nudge) = &nudge
            && nudge.fires_after(after, wall) <= now
        {
            self.show_nudge(nudge);
            showed = true;
        }
        self.close_settled(store, wall, nudge.is_none())?;
        if showed || first || fired.is_none_or(|f| now - f >= Duration::minutes(5)) {
            settings.set_last_fired(now)?;
        }
        let mut next = next(&plan, now).unwrap_or(now + longest_wait());
        if let Some(nudge) = &nudge {
            next = next.min(nudge.fires_after(now, wall));
        }
        Ok(next.min(now + longest_wait()))
    }

    /// One Catch-up Notification for what became Missed between `since`
    /// and `now`.
    fn catch_up(
        &mut self,
        store: &Store,
        wall: &WallClock,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> oldbrainfix_core::Result<()> {
        let routines = Routines::new(store).all_with_notes()?;
        let alarms = Alarms::new(store);
        let renag = DeviceSettings::new(store).renag_interval()?;
        let then = alarm_plan(&routines, &alarms.snoozes()?, since, wall, renag);
        let missed = catch_up(&then, since, now);
        if missed == 0 {
            return Ok(());
        }
        let t = self.t;
        let body = if missed == 1 {
            t.catch_up_one().to_string()
        } else {
            t.catch_up_many(&missed.to_string())
        };
        self.notifier.show(&Notice {
            key: CATCH_UP_KEY.into(),
            title: t.catch_up_title().into(),
            body,
            urgent: false,
            actions: vec![(OPEN.into(), t.tray_open().into())],
        });
        self.shown.insert(CATCH_UP_KEY.into());
        Ok(())
    }

    fn show_alarm(&mut self, alarm: &PlannedAlarm, now: DateTime<Utc>) {
        let t = self.t;
        let title = first_line(alarm.text.as_deref()).unwrap_or_else(|| {
            if alarm.text.is_none() {
                t.note_gone().to_string()
            } else {
                t.photo().to_string()
            }
        });
        let key = occurrence_key(&alarm.routine_id, &alarm.occurrence);
        let actions = [
            (DONE, t.mark_done()),
            (SNOOZE_10, t.snooze10()),
            (SNOOZE_30, t.snooze30()),
            (SNOOZE_60, t.snooze60()),
            (OPEN, t.tray_open()),
        ];
        self.notifier.show(&Notice {
            key: key.clone(),
            title,
            body: occurrence_time(&t, &alarm.occurrence),
            urgent: escalates(alarm, now),
            actions: actions
                .iter()
                .map(|(id, label)| (id.to_string(), label.to_string()))
                .collect(),
        });
        self.shown.insert(key);
    }

    fn show_nudge(&mut self, nudge: &PlannedNudge) {
        let t = self.t;
        let actions = [
            (SNOOZE_10, t.snooze10()),
            (SNOOZE_30, t.snooze30()),
            (SNOOZE_60, t.snooze60()),
            (OPEN, t.tray_open()),
        ];
        self.notifier.show(&Notice {
            key: NUDGE_KEY.into(),
            title: t.nudge_title(nudge.count() as i64),
            body: t.nudge_body().into(),
            urgent: false,
            actions: actions
                .iter()
                .map(|(id, label)| (id.to_string(), label.to_string()))
                .collect(),
        });
        self.shown.insert(NUDGE_KEY.into());
    }

    /// Closes the notifications of Occurrences Done meanwhile, and the
    /// Nudge's once nothing is Unsorted.
    fn close_settled(
        &mut self,
        store: &Store,
        wall: &WallClock,
        sorted: bool,
    ) -> oldbrainfix_core::Result<()> {
        let done: BTreeSet<String> = Alarms::new(store)
            .recently_done(wall)?
            .iter()
            .map(|(id, o)| occurrence_key(id, o))
            .collect();
        let settled: Vec<String> = self
            .shown
            .iter()
            .filter(|k| done.contains(*k) || (sorted && *k == NUDGE_KEY))
            .cloned()
            .collect();
        for key in settled {
            self.notifier.close(&key);
            self.shown.remove(&key);
        }
        Ok(())
    }
}

/// Runs the alerts over `store` until `events` is closed: a tick, then
/// waiting for the next one or for an event. `on_tap` is called after
/// each tap applied, with true when the window is to open.
pub fn run(
    store: &Store,
    wall: &WallClock,
    alerts: &mut Alerts,
    events: &Receiver<Event>,
    mut on_tap: impl FnMut(bool),
) {
    loop {
        let next = match alerts.tick(store, wall) {
            Ok(next) => next,
            Err(e) => {
                eprintln!("oldbrainfix: alerts: {e}");
                store.now() + longest_wait()
            }
        };
        let wait = (next - store.now())
            .to_std()
            .unwrap_or_default()
            .max(std::time::Duration::from_millis(200));
        match events.recv_timeout(wait) {
            Ok(Event::Tap(tap)) => {
                if let Err(e) = alerts.apply(store, &tap) {
                    eprintln!("oldbrainfix: alerts: {e}");
                }
                on_tap(std::mem::take(&mut alerts.open_requested));
            }
            Ok(Event::Wake) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oldbrainfix_core::alarms::{AlarmKind, WallMoment};
    use oldbrainfix_core::{ClockTime, Day, parse_time};

    fn alarm(
        at: &str,
        ends: &str,
        escalation: Option<i64>,
    ) -> oldbrainfix_core::Result<PlannedAlarm> {
        let day = Day::new(2026, 1, 1);
        let moment = WallMoment {
            day,
            time: ClockTime { hour: 9, minute: 0 },
        };
        Ok(PlannedAlarm {
            kind: AlarmKind::Occurrence,
            routine_id: "routine:a".into(),
            occurrence: Occurrence::new(day, ClockTime::new(9, 0)),
            at: parse_time(at)?,
            wall: Some(moment),
            renag: Duration::minutes(10),
            escalation,
            ends: moment,
            ends_at: parse_time(ends)?,
            text: Some("Pills".into()),
        })
    }

    #[test]
    fn an_occurrence_shows_once_per_tick_however_many_renags_passed() -> oldbrainfix_core::Result<()>
    {
        let a = alarm(
            "2026-01-01T09:00:00.000Z",
            "2026-01-01T12:00:00.000Z",
            Some(2),
        )?;
        let plan = vec![a.clone()];
        let t = |s: &str| parse_time(s);
        assert_eq!(
            due(
                &plan,
                t("2026-01-01T08:00:00.000Z")?,
                t("2026-01-01T08:59:00.000Z")?
            )
            .len(),
            0
        );
        assert_eq!(
            due(
                &plan,
                t("2026-01-01T08:00:00.000Z")?,
                t("2026-01-01T09:45:00.000Z")?
            )
            .len(),
            1
        );
        assert_eq!(
            next(&plan, t("2026-01-01T09:00:00.000Z")?),
            Some(t("2026-01-01T09:10:00.000Z")?)
        );
        assert!(!escalates(&a, t("2026-01-01T09:20:00.000Z")?));
        assert!(escalates(&a, t("2026-01-01T09:30:00.000Z")?));
        assert!(!escalates(&a, t("2026-01-01T12:00:00.000Z")?));
        // Missed while not running: told once, not rung.
        assert_eq!(
            catch_up(
                &plan,
                t("2026-01-01T08:00:00.000Z")?,
                t("2026-01-01T13:00:00.000Z")?
            ),
            1
        );
        assert_eq!(
            catch_up(
                &plan,
                t("2026-01-01T08:00:00.000Z")?,
                t("2026-01-01T11:00:00.000Z")?
            ),
            0
        );
        assert_eq!(
            catch_up(
                &plan,
                t("2026-01-01T14:00:00.000Z")?,
                t("2026-01-01T13:00:00.000Z")?
            ),
            0
        );
        Ok(())
    }

    #[test]
    fn snooze_buttons_wait_what_they_say() {
        assert_eq!(snooze_of(SNOOZE_10), Some(Duration::minutes(10)));
        assert_eq!(snooze_of(SNOOZE_60), Some(Duration::hours(1)));
        assert_eq!(snooze_of(DONE), None);
        assert_eq!(
            split_key("routine:a@2026-01-01T09:00"),
            Some(("routine:a", "2026-01-01T09:00"))
        );
        assert_eq!(first_line(Some("  \n")), None);
        assert_eq!(first_line(Some(" Pills \n and more")), Some("Pills".into()));
    }
}
