//! Days, times and schedules in words, as the phone's `schedule_text.dart`
//! writes them: a time as the clock shows it (`Hm`), a day as a short
//! date with its weekday (`yMEd`), and how a schedule repeats.

use oldbrainfix_core::{
    ClockTime, Day, Occurrence, Recurrence, RepeatEnd, RepeatKind, RepeatUnit, Schedule,
    SimpleRepeat,
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

/// The full name of `weekday`, 1 for Monday to 7 for Sunday.
fn weekday_long(t: &L10n, weekday: u32) -> &'static str {
    match weekday {
        1 => t.weekday_long_mon(),
        2 => t.weekday_long_tue(),
        3 => t.weekday_long_wed(),
        4 => t.weekday_long_thu(),
        5 => t.weekday_long_fri(),
        6 => t.weekday_long_sat(),
        _ => t.weekday_long_sun(),
    }
}

/// The short name of `month`, 1 for January to 12 for December.
fn month_short(t: &L10n, month: u32) -> &'static str {
    match month {
        1 => t.month_short_jan(),
        2 => t.month_short_feb(),
        3 => t.month_short_mar(),
        4 => t.month_short_apr(),
        5 => t.month_short_may(),
        6 => t.month_short_jun(),
        7 => t.month_short_jul(),
        8 => t.month_short_aug(),
        9 => t.month_short_sep(),
        10 => t.month_short_oct(),
        11 => t.month_short_nov(),
        _ => t.month_short_dec(),
    }
}

/// `day` as the heading of a day after tomorrow on the Agenda, as the
/// phone's `agendaDay` writes it: `Thursday, 29 Oct` or
/// `Donnerstag, 29. Okt.`
pub fn agenda_day_text(t: &L10n, day: Day) -> String {
    let weekday = weekday_long(t, day.weekday());
    let month = month_short(t, day.month);
    match t.locale() {
        "de" => format!("{weekday}, {}. {month}", day.day),
        _ => format!("{weekday}, {} {month}", day.day),
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

/// What the desktop's simple editor shows for `rule`: once, every N
/// days, weeks, months or years, or chosen weekdays, without an end. The
/// phone's monthly kinds and ends stay in the advanced mode here, so
/// nothing of them is dropped on save.
pub fn desktop_simple(rule: Option<&Recurrence>) -> Option<SimpleRepeat> {
    SimpleRepeat::of(rule).filter(|simple| {
        matches!(
            simple.kind,
            RepeatKind::Once | RepeatKind::Every | RepeatKind::Weekdays
        ) && simple.end() == RepeatEnd::Never
    })
}

/// How `schedule` repeats, in words, without its times.
pub fn repeat_text(t: &L10n, schedule: &Schedule) -> String {
    let rule = schedule.rule.as_ref();
    let Some(simple) = desktop_simple(rule) else {
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
        // Not shown simply here; see `desktop_simple`.
        RepeatKind::MonthDay | RepeatKind::MonthWeekday => {
            t.rule_summary(&rule.map(|r| r.format()).unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_schedules_read_in_the_apps_language() {
        let en = L10n::new("en");
        let de = L10n::new("de");
        let day = Day::new(2026, 10, 26);
        assert_eq!(day_text(&en, day), "Mon, 10/26/2026");
        assert_eq!(day_text(&de, day), "Mo., 26.10.2026");
        let thursday = Day::new(2026, 10, 29);
        assert_eq!(agenda_day_text(&en, thursday), "Thursday, 29 Oct");
        assert_eq!(agenda_day_text(&de, thursday), "Donnerstag, 29. Okt.");
        let sunday = Day::new(2026, 11, 1);
        assert_eq!(agenda_day_text(&en, sunday), "Sunday, 1 Nov");
        assert_eq!(agenda_day_text(&de, sunday), "Sonntag, 1. Nov.");
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
        let ends = Schedule::new(day, &[], Recurrence::parse("FREQ=DAILY;COUNT=3").ok());
        let Some(ends) = ends else {
            panic!("a schedule")
        };
        assert!(repeat_text(&en, &ends).contains("FREQ=DAILY;COUNT=3"));
    }
}
