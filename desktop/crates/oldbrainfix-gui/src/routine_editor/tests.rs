use super::*;

fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.naive_utc()
}

fn now() -> DateTime<Utc> {
    oldbrainfix_core::parse_time("2026-10-24T07:30:00.000Z").unwrap_or_default()
}

#[test]
fn a_new_reminder_starts_at_the_next_full_hour() {
    let editor = RoutineEditor::new(None, true, now(), &utc_wall, Duration::minutes(10));
    assert!(editor.once());
    assert_eq!(
        editor.times,
        ClockTime::new(8, 0).into_iter().collect::<Vec<_>>()
    );
    assert_eq!(editor.start, Day::new(2026, 10, 24));
    // No Note and no text: nothing to save yet.
    assert!(!editor.can_save());
}

#[test]
fn a_new_routine_starts_every_day_all_day() {
    let editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
    assert!(!editor.once());
    assert!(editor.times.is_empty());
    assert_eq!(editor.start, Day::new(2026, 10, 24));
}

#[test]
fn modes_carry_the_rule_across() {
    let mut editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
    editor.kind = RepeatKind::Weekdays;
    editor.weekdays = [1, 6].into_iter().collect();
    editor.switch_mode(true);
    editor.update();
    assert_eq!(editor.rule, "FREQ=WEEKLY;BYDAY=MO,SA");
    editor.rule = "FREQ=MONTHLY;BYDAY=1MO".into();
    editor.update();
    assert!(editor.schedule().is_some());
    editor.rule = "FREQ=HOURLY".into();
    editor.update();
    assert_eq!(editor.rule_error.as_deref(), Some("HOURLY"));
    assert!(editor.schedule().is_none());
    editor.rule = "FREQ=DAILY;INTERVAL=3".into();
    editor.switch_mode(false);
    editor.update();
    assert_eq!(
        (editor.kind, editor.every, editor.unit),
        (RepeatKind::Every, 3, RepeatUnit::Days)
    );
}

#[test]
fn the_monthly_kinds_and_ends_stay_in_the_simple_mode() {
    let mut editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
    for rule in [
        "FREQ=MONTHLY;INTERVAL=2;COUNT=5;BYDAY=-1FR",
        "FREQ=MONTHLY;UNTIL=20270131;BYMONTHDAY=31",
        "FREQ=WEEKLY;COUNT=8;BYDAY=MO,TH",
    ] {
        editor.advanced = true;
        editor.rule = rule.into();
        editor.update();
        editor.switch_mode(false);
        editor.update();
        assert!(!editor.advanced, "{rule}");
        let Some(rule_back) = editor.simple_rule() else {
            panic!("a rule for {rule}")
        };
        assert_eq!(rule_back.format(), rule);
    }
    assert_eq!(editor.end, RepeatEnd::Count);
    assert_eq!(editor.count, 8);
    // The last-Friday kind kept its choices while another was shown.
    editor.kind = RepeatKind::MonthWeekday;
    assert_eq!((editor.ordinal, editor.nth_weekday), (Some(-1), Some(5)));
}

#[test]
fn a_new_monthly_repeat_starts_from_the_start_day() {
    // Saturday, 24 October 2026: the fourth Saturday of its month.
    let mut editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
    editor.kind = RepeatKind::MonthWeekday;
    editor.update();
    assert_eq!(
        editor.simple_rule().map(|r| r.format()).as_deref(),
        Some("FREQ=MONTHLY;BYDAY=4SA")
    );
    editor.kind = RepeatKind::MonthDay;
    editor.end = RepeatEnd::Until;
    editor.update();
    // A month after the start while no end day is chosen.
    assert_eq!(
        editor.simple_rule().map(|r| r.format()).as_deref(),
        Some("FREQ=MONTHLY;UNTIL=20261124;BYMONTHDAY=24")
    );
    // The 31st of October is a fifth weekday: the last.
    assert_eq!(RoutineEditor::ordinal_of(Day::new(2026, 10, 31)), -1);
    assert_eq!(RoutineEditor::ordinal_of(Day::new(2026, 10, 28)), 4);
}
