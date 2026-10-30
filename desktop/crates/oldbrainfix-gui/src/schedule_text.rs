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

/// The full name of `weekday`, 1 for Monday to 7 for Sunday.
pub fn weekday_long(t: &L10n, weekday: u32) -> &'static str {
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
pub(crate) fn month_short(t: &L10n, month: u32) -> &'static str {
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

/// `day` as a short weekday and date, as the phone's `MMMEd` writes it:
/// `Sat, Oct 31` or `Sa., 31. Okt.`
pub fn short_day_text(t: &L10n, day: Day) -> String {
    let weekday = weekday_text(t, day.weekday());
    let month = month_short(t, day.month);
    match t.locale() {
        "de" => format!("{weekday}, {}. {month}", day.day),
        _ => format!("{weekday}, {month} {}", day.day),
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

/// How `schedule` repeats, in words, without its times, with its end.
pub fn repeat_text(t: &L10n, schedule: &Schedule) -> String {
    let rule = schedule.rule.as_ref();
    let Some(simple) = SimpleRepeat::of(rule) else {
        return t.rule_summary(&rule.map(|r| r.format()).unwrap_or_default());
    };
    let repeat = match simple.kind {
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
        RepeatKind::MonthDay => t.every_month_on_day(
            simple.every,
            simple.month_day.unwrap_or(i64::from(schedule.start.day)),
        ),
        RepeatKind::MonthWeekday => t.every_month_on_nth(
            simple.every,
            // `1` to `4`; anything else is the last.
            &t.nth_word(
                &simple
                    .nth
                    .and_then(|nth| nth.ordinal)
                    .unwrap_or(-1)
                    .to_string(),
            ),
            weekday_long(
                t,
                simple
                    .nth
                    .map_or(schedule.start.weekday(), |nth| nth.weekday),
            ),
        ),
    };
    if let Some(count) = simple.count {
        return t.repeat_count(count, &repeat);
    }
    if let Some(until) = simple.until {
        return t.repeat_until(&repeat, &day_text(t, until));
    }
    repeat
}

#[cfg(test)]
mod tests {
    use oldbrainfix_core::Recurrence;

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
        let saturday = Day::new(2026, 10, 31);
        assert_eq!(short_day_text(&en, saturday), "Sat, Oct 31");
        assert_eq!(short_day_text(&de, saturday), "Sa., 31. Okt.");
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
        // The phone's monthly kinds and ends read as on the phone.
        let repeat = |rule: &str| {
            let schedule = Schedule::new(day, &[], Recurrence::parse(rule).ok());
            let Some(schedule) = schedule else {
                panic!("a schedule")
            };
            (repeat_text(&en, &schedule), repeat_text(&de, &schedule))
        };
        let (first_monday, erster_montag) = repeat("FREQ=MONTHLY;BYDAY=1MO");
        assert_eq!(first_monday, en.every_month_on_nth(1, "first", "Monday"));
        assert_eq!(
            erster_montag,
            de.every_month_on_nth(1, &de.nth_word("1"), "Montag")
        );
        let (last_friday, _) = repeat("FREQ=MONTHLY;INTERVAL=2;BYDAY=-1FR");
        assert_eq!(last_friday, en.every_month_on_nth(2, "last", "Friday"));
        let (fifteenth, _) = repeat("FREQ=MONTHLY;BYMONTHDAY=15");
        assert_eq!(fifteenth, en.every_month_on_day(1, 15));
        let (three_times, _) = repeat("FREQ=DAILY;COUNT=3");
        assert_eq!(three_times, en.repeat_count(3, &en.every_n_days(1)));
        let (until, _) = repeat("FREQ=WEEKLY;BYDAY=MO;UNTIL=20261130");
        assert_eq!(
            until,
            en.repeat_until(&en.on_weekdays("Mon"), "Mon, 11/30/2026")
        );
        // What the simple editor cannot show reads as its rule.
        let (odd, _) = repeat("FREQ=MONTHLY;BYDAY=1MO,2TU");
        assert!(odd.contains("FREQ=MONTHLY;BYDAY=1MO,2TU"));
    }
}
