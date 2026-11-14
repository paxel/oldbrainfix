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
