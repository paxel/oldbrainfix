//! Days, times and schedules in words, as the phone's `schedule_text.dart`
//! writes them: a time as the clock shows it (`Hm`), a day as a short
//! date with its weekday (`yMEd`), and how a schedule repeats.

use oldbrainfix_core::{
    ClockTime, Day, Occurrence, RepeatKind, RepeatUnit, Schedule, SimpleRepeat,
};

use crate::l10n::L10n;

/// `time` as the clock shows it, `08:00`, in both languages.
pub fn clock_text(time: ClockTime) -> String {
    time.to_string()
}

/// The short name of `weekday`, 1 for Monday to 7 for Sunday.
pub fn weekday_text(t: &L10n, weekday: u32) -> &'static str {
    match weekday {
        1 => t.weekday_mon(),
        2 => t.weekday_tue(),
        3 => t.weekday_wed(),
        4 => t.weekday_thu(),
        5 => t.weekday_fri(),
        6 => t.weekday_sat(),
        _ => t.weekday_sun(),
    }
}

/// `day` as a short date with its weekday: `Mon, 10/26/2026` or
/// `Mo., 26.10.2026`.
pub fn day_text(t: &L10n, day: Day) -> String {
    let weekday = weekday_text(t, day.weekday());
    match t.locale() {
        "de" => format!("{weekday}, {}.{}.{}", day.day, day.month, day.year),
        _ => format!("{weekday}, {}/{}/{}", day.month, day.day, day.year),
    }
}

/// When an Occurrence is due: its time, or "All day".
pub fn occurrence_time(t: &L10n, occurrence: &Occurrence) -> String {
    match occurrence.time {
        None => t.all_day().to_string(),
        Some(time) => clock_text(time),
    }
}

/// `schedule` in words: how it repeats, and at which times.
pub fn schedule_text(t: &L10n, schedule: &Schedule) -> String {
    let times = schedule.day_times();
    let at = if times.is_empty() {
        t.all_day().to_string()
    } else {
        times
            .iter()
            .map(|time| clock_text(*time))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!("{} · {at}", repeat_text(t, schedule))
}

/// How `schedule` repeats, in words, without its times.
pub fn repeat_text(t: &L10n, schedule: &Schedule) -> String {
    let rule = schedule.rule.as_ref();
    let Some(simple) = SimpleRepeat::of(rule) else {
        return t.rule_summary(&rule.map(|r| r.format()).unwrap_or_default());
    };
    match simple.kind {
        RepeatKind::Once => t.once_on(&day_text(t, schedule.start)),
        RepeatKind::Every => match simple.unit {
            RepeatUnit::Days => t.every_n_days(simple.every),
            RepeatUnit::Weeks => t.every_n_weeks(simple.every),
            RepeatUnit::Months => t.every_n_months(simple.every),
            RepeatUnit::Years => t.every_n_years(simple.every),
        },
        RepeatKind::Weekdays => t.on_weekdays(
            &simple
                .weekdays
                .iter()
                .map(|d| weekday_text(t, *d))
                .collect::<Vec<_>>()
                .join(", "),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oldbrainfix_core::Recurrence;

    #[test]
    fn days_and_schedules_read_in_the_apps_language() {
        let en = L10n::new("en");
        let de = L10n::new("de");
        let day = Day::new(2026, 10, 26);
        assert_eq!(day_text(&en, day), "Mon, 10/26/2026");
        assert_eq!(day_text(&de, day), "Mo., 26.10.2026");
        let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
        let weekly = Recurrence::parse("FREQ=WEEKLY;BYDAY=SA,MO").ok();
        let schedule = Schedule::new(day, &eight, weekly);
        let Some(schedule) = schedule else {
            panic!("a schedule")
        };
        assert_eq!(schedule_text(&en, &schedule), "Every Mon, Sat · 08:00");
        let once = Schedule::new(day, &[], None);
        let Some(once) = once else {
            panic!("a Reminder")
        };
        assert_eq!(schedule_text(&en, &once), "Once, Mon, 10/26/2026 · All day");
        let odd = Schedule::new(day, &[], Recurrence::parse("FREQ=MONTHLY;BYDAY=1MO").ok());
        let Some(odd) = odd else { panic!("a schedule") };
        assert!(repeat_text(&en, &odd).contains("FREQ=MONTHLY;BYDAY=1MO"));
    }
}
