use super::*;

fn day(y: i64, m: i64, d: i64) -> Day {
    Day::new(y, m, d)
}

fn at(h: u32, m: u32) -> ClockTime {
    ClockTime { hour: h, minute: m }
}

#[test]
fn a_weekday_is_written_by_its_code_and_one_out_of_range_as_the_nearest() {
    for (i, code) in WEEKDAY_CODES.iter().enumerate() {
        assert_eq!(weekday_code(i as u32 + 1), *code);
    }
    let rule = |weekday| WeekdayRule {
        weekday,
        ordinal: Some(1),
    };
    assert_eq!(rule(0).to_string(), "1MO");
    assert_eq!(rule(8).to_string(), "1SU");
}

#[test]
fn days_normalise_like_a_calendar() {
    assert_eq!(day(2026, 1, 32), day(2026, 2, 1));
    assert_eq!(day(2026, 13, 1), day(2027, 1, 1));
    assert_eq!(day(2026, 3, 0), day(2026, 2, 28));
    assert_eq!(day(2024, 0, 1), day(2023, 12, 1));
    assert_eq!(day(2026, 10, 24).weekday(), 6);
    assert_eq!(day(1970, 1, 1).weekday(), 4);
    assert_eq!(Day::parse("2026-02-29"), None);
    assert_eq!(Day::parse("2028-02-29"), Some(day(2028, 2, 29)));
    assert_eq!(day(2026, 1, 1).days_until(day(2027, 1, 1)), 365);
    assert_eq!(day(5, 1, 1).to_string(), "0005-01-01");
}

#[test]
fn occurrences_name_themselves_and_order_all_day_first() {
    let o = Occurrence::new(day(2026, 10, 25), Some(at(8, 0)));
    assert_eq!(o.key(), "2026-10-25T08:00");
    assert_eq!(Occurrence::parse(&o.key()), Some(o));
    assert_eq!(Occurrence::parse("2026-10-25T08:00T1"), None);
    let all = Occurrence::new(day(2026, 10, 25), None);
    assert!(all < o);
    assert_eq!(Occurrence::parse("2026-10-25"), Some(all));
}

#[test]
fn a_rule_reads_and_writes_back() {
    let rule = Recurrence::parse("rrule:freq=monthly;byday=-1fr;interval=2");
    assert_eq!(
        rule.map(|r| r.format()),
        Ok("FREQ=MONTHLY;INTERVAL=2;BYDAY=-1FR".to_string())
    );
    assert_eq!(
        Recurrence::parse("FREQ=WEEKLY;BYDAY=1MO"),
        Err(RuleError::new("ordinal only with MONTHLY or YEARLY", "1MO"))
    );
    assert_eq!(
        Recurrence::parse("FREQ=HOURLY"),
        Err(RuleError::new("FREQ not supported", "HOURLY"))
    );
}

#[test]
fn several_times_a_day_count_each_toward_count() -> Result<(), RuleError> {
    let rule = Recurrence::parse("FREQ=DAILY;COUNT=3")?;
    let schedule = Schedule::new(day(2026, 1, 1), &[at(20, 0), at(8, 0)], Some(rule));
    let keys: Vec<String> = schedule
        .map(|s| s.next(day(2026, 1, 1), 10))
        .unwrap_or_default()
        .iter()
        .map(Occurrence::key)
        .collect();
    assert_eq!(
        keys,
        ["2026-01-01T08:00", "2026-01-01T20:00", "2026-01-02T08:00"]
    );
    Ok(())
}

#[test]
fn a_reminder_happens_once_and_only_once() {
    assert!(Schedule::new(day(2026, 1, 1), &[at(8, 0), at(9, 0)], None).is_none());
    let reminder = Schedule::new(day(2026, 1, 5), &[at(8, 0)], None);
    let Some(reminder) = reminder else {
        panic!("a Reminder")
    };
    assert_eq!(reminder.next(day(2026, 1, 1), 3).len(), 1);
    assert!(reminder.next(day(2026, 1, 6), 3).is_empty());
}

#[test]
fn the_simple_editor_shows_what_it_can() -> Result<(), RuleError> {
    let weekdays = Recurrence::parse("FREQ=WEEKLY;BYDAY=SA,MO")?;
    let simple = SimpleRepeat::of(Some(&weekdays));
    assert_eq!(simple.as_ref().map(|s| s.kind), Some(RepeatKind::Weekdays));
    assert_eq!(
        simple.and_then(|s| s.to_rule()).map(|r| r.format()),
        Some("FREQ=WEEKLY;BYDAY=MO,SA".to_string())
    );
    let first = Recurrence::parse("FREQ=MONTHLY;BYDAY=1MO;COUNT=3")?;
    let simple = SimpleRepeat::of(Some(&first));
    assert_eq!(
        simple.as_ref().map(|s| (s.kind, s.end())),
        Some((RepeatKind::MonthWeekday, RepeatEnd::Count))
    );
    assert_eq!(
        simple.and_then(|s| s.to_rule()).map(|r| r.format()),
        Some("FREQ=MONTHLY;COUNT=3;BYDAY=1MO".to_string())
    );
    let both = Recurrence::parse("FREQ=MONTHLY;BYDAY=1MO,-1FR")?;
    assert_eq!(SimpleRepeat::of(Some(&both)), None);
    let at_noon = Recurrence::parse("FREQ=DAILY;UNTIL=20261231T120000")?;
    assert_eq!(SimpleRepeat::of(Some(&at_noon)), None);
    assert_eq!(SimpleRepeat::of(None), Some(SimpleRepeat::once()));
    Ok(())
}
