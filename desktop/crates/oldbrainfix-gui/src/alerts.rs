//! The desktop's alarm layer: what the phone's Kotlin `Scheduler` does
//! with the core's alarm plan, done while the app runs, also with the
//! window closed into the tray. Every tick shows what came due since the
//! last one as a notification (an Occurrence, its Re-nags, a snooze, the
//! Nudge, the Changes Nudge), escalated ones urgent and with the
//! system's alarm sound, the others of an Occurrence silent while the app
//! rings its alarm sound itself (#102), and closes the notifications of Occurrences Done or skipped meanwhile, and the
//! Changes Nudge's once nobody waits. What became Missed while the app
//! was not running is told in one Catch-up Notification, never rung
//! late. An Occurrence's notification offers Done, Snooze and Not today
//! (#269); Snooze shows it again in place with 10 min, 30 min and 1 h,
//! and Not today with Skip and Tomorrow, as the phone's Android
//! notification does (Q650), as Windows toasts and some Linux desktops
//! show only a few buttons. What is tapped comes back as [`Tap`]s and is
//! applied to the log; a click on an Occurrence's notification opens
//! the window with its Note (#270), a click on the Nudge the
//! window on Unsorted, as the phone's Nudge screen, and a click on the
//! Changes Nudge asks the window to send each person waiting their
//! bundle (#109). Quiet
//! Hours hold back the Nudge and the Changes Nudge, and an Occurrence
//! firing in them shows silent with no alarm sound rung, but for one
//! that escalates (obf-1045); with alerts off on this Device nothing
//! shows (ADR 0008). A Heads-up shows once in its Occurrence's
//! notification, which the ring then takes over: the Note's title with
//! how long until it starts, a click opening the Note, nothing to check
//! off, without sound in the Quiet Hours (obf-1046).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use oldbrainfix_core::alarms::{PlannedAlarm, alarm_plan, heads_up_plan};
use oldbrainfix_core::person_settings::PersonSettings;
use oldbrainfix_core::{
    Alarms, Day, DeviceSettings, Occurrence, PlannedChangesNudge, PlannedHeadsUp, PlannedNudge,
    Routines, Store, WallClock, Wordless,
};

use crate::l10n::{L10n, names_text, wordless};
use crate::profile_view::display_name;
use crate::ring::{Ringer, alarm_sound};
use crate::schedule_text::{heads_up_in, occurrence_time};

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
    /// `routine:…@2026-10-25T08:00`, [`NUDGE_KEY`], [`CHANGES_KEY`] or
    /// [`CATCH_UP_KEY`].
    pub key: String,
    pub title: String,
    pub body: String,
    /// Escalated: urgent and with the system's alarm sound.
    pub urgent: bool,
    /// Without the notification's own sound: the app rings an
    /// Occurrence's alarm sound itself (#102).
    pub silent: bool,
    /// The buttons it offers, by id ([`DONE`], [`SNOOZE_10`], ...) and
    /// label.
    pub actions: Vec<(String, String)>,
}

pub const NUDGE_KEY: &str = "nudge";
/// The Changes Nudge's (#109).
pub const CHANGES_KEY: &str = "changes";
pub const CATCH_UP_KEY: &str = "catch-up";

/// The ids of a notification's buttons; [`OPEN`] is a click on the
/// notification itself.
pub const DONE: &str = "done";
pub const SNOOZE_10: &str = "snooze10";
pub const SNOOZE_30: &str = "snooze30";
pub const SNOOZE_60: &str = "snooze60";
/// Snooze and Not today, which show their choices in place (#269).
pub const SNOOZE: &str = "snooze";
pub const NOT_TODAY: &str = "notToday";
pub const SKIP: &str = "skip";
pub const TOMORROW: &str = "tomorrow";
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

/// The Routine whose Occurrence's notification was clicked, held until
/// the window opens its Note (#270).
pub type OccurrenceWanted = Arc<Mutex<Option<String>>>;

/// The Routine and Occurrence a notification's key names.
fn split_key(key: &str) -> Option<(&str, &str)> {
    key.rsplit_once('@')
}

/// The buttons an Occurrence's notification offers first: Done, Snooze,
/// Not today and Open, a click on it.
pub fn occurrence_buttons(t: &L10n) -> Vec<(String, String)> {
    buttons(&[
        (DONE, t.mark_done()),
        (SNOOZE, t.snooze()),
        (NOT_TODAY, t.not_today()),
        (OPEN, t.tray_open()),
    ])
}

/// The buttons it shows in place once `action` was tapped, Snooze or
/// Not today, with the text that asks for them; `None` for any other.
/// Tomorrow only while `occurrence` is of `today` or later.
pub fn choices(
    t: &L10n,
    action: &str,
    occurrence: &Occurrence,
    today: Day,
) -> Option<(String, Vec<(String, String)>)> {
    match action {
        SNOOZE => Some((
            t.snooze_question().to_string(),
            buttons(&[
                (SNOOZE_10, t.snooze10()),
                (SNOOZE_30, t.snooze30()),
                (SNOOZE_60, t.snooze60()),
                (OPEN, t.tray_open()),
            ]),
        )),
        NOT_TODAY => {
            let mut offered = vec![(SKIP, t.skip())];
            if occurrence.day >= today {
                offered.push((TOMORROW, t.not_today_tomorrow()));
            }
            offered.push((OPEN, t.tray_open()));
            Some((t.not_today_question().to_string(), buttons(&offered)))
        }
        _ => None,
    }
}

fn buttons(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(id, label)| (id.to_string(), label.to_string()))
        .collect()
}

/// The first line of a Note's text, or `None` when there is none.
fn first_line(text: Option<&str>) -> Option<String> {
    let line = text?.trim().lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_string())
}

/// What an Occurrence's notification is titled: the first line of its
/// Note's `text`, what the Note holds while it has no words, or that the
/// Note is gone.
fn ring_title(t: &L10n, text: Option<&str>, kind: Wordless, name: &str) -> String {
    first_line(text).unwrap_or_else(|| {
        if text.is_none() {
            t.note_gone().to_string()
        } else {
            wordless(t, kind, name)
        }
    })
}

/// The Heads-ups that should exist at `at` (obf-1046), as
/// [`Alarms::heads_ups`] reads them at the store's now: from the log
/// alone, each one's offsets as the user's own Devices set them, with
/// this Device's snoozes, all-day time and Quiet Hours. Read at the last
/// tick, it holds those that came due since, with Done, Skip and Snooze
/// read anew.
pub fn heads_ups_at(
    store: &Store,
    wall: &WallClock,
    at: DateTime<Utc>,
) -> oldbrainfix_core::Result<Vec<PlannedHeadsUp>> {
    let routines = store.log_only(|| -> oldbrainfix_core::Result<Vec<_>> {
        let read = Routines::new(store);
        let mut out = Vec::new();
        for (routine, note) in read.all_with_notes()? {
            let minutes = read.heads_ups(&routine.id)?;
            if !minutes.is_empty() {
                out.push((routine, note, minutes));
            }
        }
        Ok(out)
    })?;
    if routines.is_empty() {
        return Ok(Vec::new());
    }
    let settings = DeviceSettings::new(store);
    let quiet = if settings.quiet_hours_on()? {
        Some(settings.quiet_hours()?)
    } else {
        None
    };
    Ok(heads_up_plan(
        &routines,
        &Alarms::new(store).snoozes()?,
        at,
        wall,
        settings.all_day_from()?,
        quiet,
        &PersonSettings::new(store).quiet_days()?,
    ))
}

/// The alarm layer's state between ticks.
pub struct Alerts {
    notifier: Box<dyn Notifier>,
    t: L10n,
    /// When the last tick looked; what came due before is shown.
    last: Option<DateTime<Utc>>,
    /// The notifications showing, by key, each as last shown.
    shown: BTreeMap<String, Notice>,
    /// Bumped whenever a tap changed the log, for the window to read
    /// again.
    changed: Arc<AtomicU64>,
    /// Asked for by a click on a notification: the window is to open.
    pub open_requested: bool,
    /// Given the Routine by a click on an Occurrence's notification: the
    /// window is to open its Note.
    occurrence_wanted: OccurrenceWanted,
    /// Raised by a click on the Nudge: the window is to show Unsorted.
    unsorted_wanted: Arc<AtomicBool>,
    /// Raised by a click on the Changes Nudge: the window is to send each
    /// person waiting their bundle.
    changes_wanted: Arc<AtomicBool>,
    /// Plays the alarm sound of an Occurrence that rings; without one
    /// nothing plays.
    ringer: Option<Box<dyn Ringer>>,
    /// The Heads-ups shown, by id, with when each fired: each shows once,
    /// also when the clock is set back.
    heads_ups_fired: BTreeMap<String, DateTime<Utc>>,
    /// The log's mark when the last tick looked ([`Store::current_seq`]).
    mark: Option<i64>,
}

impl Alerts {
    pub fn new(notifier: Box<dyn Notifier>, t: L10n, changed: Arc<AtomicU64>) -> Alerts {
        Alerts {
            notifier,
            t,
            last: None,
            shown: BTreeMap::new(),
            changed,
            open_requested: false,
            occurrence_wanted: Arc::new(Mutex::new(None)),
            unsorted_wanted: Arc::new(AtomicBool::new(false)),
            changes_wanted: Arc::new(AtomicBool::new(false)),
            ringer: None,
            heads_ups_fired: BTreeMap::new(),
            mark: None,
        }
    }

    /// Plays the alarm sound of each Occurrence that rings through
    /// `ringer`.
    pub fn set_ringer(&mut self, ringer: Box<dyn Ringer>) {
        self.ringer = Some(ringer);
    }

    /// Raises `wanted` when the Nudge is clicked, for the window to show
    /// Unsorted ([`crate::App::set_unsorted_wanted`]).
    pub fn set_unsorted_wanted(&mut self, wanted: Arc<AtomicBool>) {
        self.unsorted_wanted = wanted;
    }

    /// Puts the Routine into `wanted` when an Occurrence's notification
    /// is clicked, for the window to open its Note
    /// ([`crate::App::set_occurrence_wanted`]).
    pub fn set_occurrence_wanted(&mut self, wanted: OccurrenceWanted) {
        self.occurrence_wanted = wanted;
    }

    /// Raises `wanted` when the Changes Nudge is clicked, for the window
    /// to send the bundles ([`crate::App::set_changes_wanted`]).
    pub fn set_changes_wanted(&mut self, wanted: Arc<AtomicBool>) {
        self.changes_wanted = wanted;
    }

    /// Applies one tap to the log and this Device's snoozes, `wall`
    /// telling which day it is; Snooze and Not today show their choices
    /// in place, quietly.
    pub fn apply(
        &mut self,
        store: &Store,
        wall: &WallClock,
        tap: &Tap,
    ) -> oldbrainfix_core::Result<()> {
        if tap.action == OPEN {
            // The Changes Nudge stays until the bundles went: a save
            // dialog cancelled keeps it for that person.
            if tap.key == CHANGES_KEY {
                self.changes_wanted.store(true, Ordering::SeqCst);
            }
            if tap.key == NUDGE_KEY {
                self.unsorted_wanted.store(true, Ordering::SeqCst);
            }
            if let Some((routine, occurrence)) = split_key(&tap.key)
                && Occurrence::parse(occurrence).is_some()
                && let Ok(mut wanted) = self.occurrence_wanted.lock()
            {
                *wanted = Some(routine.to_string());
            }
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
        let today = Day::today(store.now(), wall);
        if let (Some(parsed), Some(held)) =
            (Occurrence::parse(occurrence), self.shown.get(&tap.key))
            && let Some((body, actions)) = choices(&self.t, &tap.action, &parsed, today)
        {
            let notice = Notice {
                body,
                actions,
                urgent: false,
                silent: true,
                ..held.clone()
            };
            self.notifier.show(&notice);
            self.shown.insert(tap.key.clone(), notice);
            return Ok(());
        }
        match tap.action.as_str() {
            DONE => drop(alarms.apply_done(routine, occurrence, tap.at)?),
            SKIP => drop(alarms.apply_skip(routine, occurrence, tap.at)?),
            TOMORROW => drop(alarms.apply_tomorrow(routine, occurrence, tap.at)?),
            action => match snooze_of(action) {
                Some(wait) => alarms.apply_snooze(routine, occurrence, tap.at + wait)?,
                None => return Ok(()),
            },
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
        // On a connection of its own (obf-1091), what it worked out from
        // the log, the rights among it, is forgotten once the window or a
        // sync wrote since the last tick.
        let mark = store.current_seq()?;
        if self.mark.replace(mark).is_some_and(|last| last != mark) {
            store.changed_elsewhere();
        }
        let now = store.now();
        let settings = DeviceSettings::new(store);
        let first = self.last.is_none();
        let last = self.last.replace(now);
        if !settings.alerts_on()? {
            // Nothing piles up to ring once they are on again.
            for key in std::mem::take(&mut self.shown).into_keys() {
                self.notifier.close(&key);
            }
            settings.set_last_fired(now)?;
            return Ok(now + longest_wait());
        }
        let alarms = Alarms::new(store);
        let plan = alarms.plan(wall)?;
        let nudge = alarms.nudge(wall)?;
        let changes = alarms.changes_nudge(wall)?;
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
        // Before the rings, so one of the same Occurrence takes over its
        // notification.
        let heads_ups = heads_ups_at(store, wall, after)?;
        for heads_up in heads_ups.iter().filter(|h| h.at <= now) {
            if self
                .heads_ups_fired
                .insert(heads_up.id(), heads_up.at)
                .is_none()
            {
                self.show_heads_up(heads_up, wall);
                showed = true;
            }
        }
        self.heads_ups_fired
            .retain(|_, at| *at > now - Duration::days(1));
        let mut ring = false;
        for alarm in due(&plan, after, now) {
            ring |= self.show_alarm(alarm, now, wall);
            showed = true;
        }
        if ring && let Some(ringer) = &mut self.ringer {
            // Unreadable, it rings the own sound: an alarm never stays
            // silent over a setting.
            let chosen = settings.alarm_sound().ok().flatten();
            ringer.ring(alarm_sound(chosen.as_deref()).0);
        }
        if let Some(nudge) = &nudge
            && nudge.fires_after(after, wall) <= now
        {
            self.show_nudge(nudge);
            showed = true;
        }
        if let Some(changes) = &changes
            && changes.fires_after(after, wall) <= now
        {
            self.show_changes(store, changes)?;
            showed = true;
        }
        self.close_settled(store, wall, nudge.is_none(), changes.is_none())?;
        if showed || first || fired.is_none_or(|f| now - f >= Duration::minutes(5)) {
            settings.set_last_fired(now)?;
        }
        let mut next = next(&plan, now).unwrap_or(now + longest_wait());
        if let Some(at) = heads_ups.iter().map(|h| h.at).filter(|at| *at > now).min() {
            next = next.min(at);
        }
        if let Some(nudge) = &nudge {
            next = next.min(nudge.fires_after(now, wall));
        }
        if let Some(changes) = &changes {
            next = next.min(changes.fires_after(now, wall));
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
        let settings = DeviceSettings::new(store);
        let quiet = if settings.quiet_hours_on()? {
            Some(settings.quiet_hours()?)
        } else {
            None
        };
        let then = alarm_plan(
            &routines,
            &alarms.snoozes()?,
            since,
            wall,
            settings.renag_interval()?,
            settings.all_day_from()?,
            quiet,
            &PersonSettings::new(store).quiet_days()?,
        );
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
        self.show(Notice {
            key: CATCH_UP_KEY.into(),
            title: t.catch_up_title().into(),
            body,
            urgent: false,
            silent: false,
            actions: vec![(OPEN.into(), t.tray_open().into())],
        });
        Ok(())
    }

    /// Shows `alarm`'s notification; true when the app is to ring its
    /// alarm sound for it, false when it escalated and the system's
    /// alarm sound rings, or when it fires in the Quiet Hours and stays
    /// silent (obf-1045).
    fn show_alarm(&mut self, alarm: &PlannedAlarm, now: DateTime<Utc>, wall: &WallClock) -> bool {
        let t = self.t;
        let title = ring_title(
            &t,
            alarm.text.as_deref(),
            alarm.wordless,
            &alarm.wordless_name,
        );
        let key = occurrence_key(&alarm.routine_id, &alarm.occurrence);
        let urgent = escalates(alarm, now);
        self.show(Notice {
            key,
            title,
            body: occurrence_time(&t, &alarm.occurrence),
            urgent,
            silent: !urgent,
            actions: occurrence_buttons(&t),
        });
        !urgent && !alarm.silent_at(now, wall)
    }

    /// Shows `heads_up` in its Occurrence's notification (obf-1046): the
    /// Note's title with how long until it starts, nothing but a click to
    /// open the Note, with the notification's own sound but in the Quiet
    /// Hours; the app rings no alarm sound for it.
    fn show_heads_up(&mut self, heads_up: &PlannedHeadsUp, wall: &WallClock) {
        let t = self.t;
        let title = ring_title(
            &t,
            heads_up.text.as_deref(),
            heads_up.wordless,
            &heads_up.wordless_name,
        );
        self.show(Notice {
            key: occurrence_key(&heads_up.routine_id, &heads_up.occurrence),
            title,
            body: heads_up_in(&t, heads_up.minutes),
            urgent: false,
            silent: heads_up.silent_at(wall),
            actions: vec![(OPEN.into(), t.tray_open().into())],
        });
    }

    fn show_nudge(&mut self, nudge: &PlannedNudge) {
        let t = self.t;
        let actions = [
            (SNOOZE_10, t.snooze10()),
            (SNOOZE_30, t.snooze30()),
            (SNOOZE_60, t.snooze60()),
            (OPEN, t.tray_open()),
        ];
        self.show(Notice {
            key: NUDGE_KEY.into(),
            title: t.nudge_title(nudge.count() as i64),
            body: t.nudge_body().into(),
            urgent: false,
            silent: false,
            actions: buttons(&actions),
        });
    }

    /// The Changes Nudge, naming the people waiting: "Changes for Bob and
    /// Carol".
    fn show_changes(
        &mut self,
        store: &Store,
        changes: &PlannedChangesNudge,
    ) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        let mut names = Vec::new();
        for person in &changes.people {
            let shown = store.person_shown(person)?;
            names.push(display_name(&t, &shown.name).to_string());
        }
        self.show(Notice {
            key: CHANGES_KEY.into(),
            title: t.changes_nudge_title(&names_text(&t, &names)),
            body: t.changes_nudge_body().into(),
            urgent: false,
            silent: false,
            actions: vec![(OPEN.into(), t.send_action().into())],
        });
        Ok(())
    }

    /// Shows `notice`, in place of the one with its key.
    fn show(&mut self, notice: Notice) {
        self.notifier.show(&notice);
        self.shown.insert(notice.key.clone(), notice);
    }

    /// Closes the notifications of Occurrences Done or skipped meanwhile, the
    /// Nudge's once nothing is Unsorted and the Changes Nudge's once
    /// nobody waits.
    fn close_settled(
        &mut self,
        store: &Store,
        wall: &WallClock,
        sorted: bool,
        sent: bool,
    ) -> oldbrainfix_core::Result<()> {
        let done: BTreeSet<String> = Alarms::new(store)
            .recently_done(wall)?
            .iter()
            .map(|(id, o)| occurrence_key(id, o))
            .collect();
        let settled: Vec<String> = self
            .shown
            .keys()
            .filter(|k| {
                done.contains(*k) || (sorted && *k == NUDGE_KEY) || (sent && *k == CHANGES_KEY)
            })
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
/// each tap applied, with true when the window is to open. On a
/// connection of its own, a wake forgets what it read of the local
/// settings too (obf-1091): a Profile kept elsewhere moves no mark.
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
                if let Err(e) = alerts.apply(store, wall, &tap) {
                    eprintln!("oldbrainfix: alerts: {e}");
                }
                on_tap(std::mem::take(&mut alerts.open_requested));
            }
            Ok(Event::Wake) => store.changed_elsewhere(),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests;
