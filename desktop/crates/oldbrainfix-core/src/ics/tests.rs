use super::*;
use crate::schedule::Schedule;
use crate::settings::DEFAULT_ALL_DAY_FROM;
use chrono::TimeZone;
use std::collections::BTreeMap;

fn at(hour: u32, minute: u32) -> ClockTime {
    ClockTime { hour, minute }
}

fn routine(id: &str, start: Day, times: &[ClockTime], rule: Option<&str>) -> Routine {
    let rule = rule.map(|r| match Recurrence::parse(r) {
        Ok(rule) => rule,
        Err(e) => panic!("{e}"),
    });
    let Some(schedule) = Schedule::new(start, times, rule) else {
        panic!("a schedule");
    };
    Routine {
        id: id.to_string(),
        note_id: "note:n1".to_string(),
        schedule,
        done: BTreeMap::new(),
        renag: None,
        escalation: None,
        scheduled_at: None,
        trashed: None,
        trashed_with: None,
        skipped: BTreeMap::new(),
        tomorrow: Default::default(),
        moved: Vec::new(),
    }
}

fn stamp() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 24, 7, 5, 9)
        .single()
        .unwrap_or_default()
}

fn saturday() -> Day {
    Day::new(2026, 10, 24)
}

/// The file's logical lines: folds undone, CRLFs gone.
fn unfolded(ics: &str) -> Vec<String> {
    let joined = ics.replace("\r\n ", "");
    let mut lines: Vec<String> = joined.split("\r\n").map(String::from).collect();
    lines.pop();
    lines
}

#[test]
fn a_calendar_around_the_events_every_line_ending_in_crlf() {
    let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
    let ics = write_ics(&routine_events(&r, "Pills"), stamp(), DEFAULT_ALL_DAY_FROM);
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:"));
    assert!(ics.ends_with("END:VEVENT\r\nEND:VCALENDAR\r\n"));
    let bare = ics.replace("\r\n", "");
    assert!(!bare.contains('\n') && !bare.contains('\r'));
    assert!(unfolded(&ics).contains(&"DTSTAMP:20261024T070509Z".to_string()));
}

#[test]
fn an_all_day_alarm_is_at_the_exporting_devices_time() {
    let r = routine("routine:r1", saturday(), &[], None);
    let events = routine_events(&r, "Party");
    let ics = unfolded(&write_ics(&events, stamp(), DEFAULT_ALL_DAY_FROM));
    assert!(
        ics.contains(&"TRIGGER:PT900M".to_string()),
        "15:00 by default"
    );
    let own = ClockTime {
        hour: 11,
        minute: 30,
    };
    let ics = unfolded(&write_ics(&events, stamp(), own));
    assert!(ics.contains(&"TRIGGER:PT690M".to_string()));
}

#[test]
fn an_event_without_a_start_is_left_out() {
    let event = CalendarEvent {
        uid: "note:n1".into(),
        summary: "Call Mum".into(),
        description: String::new(),
        start: None,
        rule: None,
    };
    assert!(!write_ics(&[event], stamp(), DEFAULT_ALL_DAY_FROM).contains("VEVENT"));
}

#[test]
fn backslash_semicolon_comma_and_line_breaks_are_escaped() {
    let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
    let ics = write_ics(
        &routine_events(&r, "Buy a\\b; milk, eggs\r\nand bread\nnow\rthen"),
        stamp(),
        DEFAULT_ALL_DAY_FROM,
    );
    let lines = unfolded(&ics);
    assert!(lines.contains(&r"SUMMARY:Buy a\\b\; milk\, eggs".to_string()));
    assert!(
        lines.contains(&r"DESCRIPTION:Buy a\\b\; milk\, eggs\nand bread\nnow\nthen".to_string())
    );
}

#[test]
fn long_lines_fold_at_75_octets_without_cutting_a_character() {
    let text = format!("{}Ende", "Medikament für Oma ü 💊 ".repeat(8));
    let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
    let ics = write_ics(&routine_events(&r, &text), stamp(), DEFAULT_ALL_DAY_FROM);
    let mut physical: Vec<&str> = ics.split("\r\n").collect();
    physical.pop();
    assert!(physical.iter().all(|l| l.len() <= 75));
    assert!(physical.iter().any(|l| l.starts_with(' ')));
    assert!(unfolded(&ics).contains(&format!("SUMMARY:{text}")));
}

#[test]
fn a_line_of_exactly_75_octets_is_not_folded() {
    let text = "x".repeat(67);
    let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
    let ics = write_ics(&routine_events(&r, &text), stamp(), DEFAULT_ALL_DAY_FROM);
    assert!(ics.contains(&format!("\r\nSUMMARY:{text}\r\nBEGIN:VALARM")));
}

#[test]
fn summary_and_description_trim_as_dart_does() {
    let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
    let events = routine_events(&r, "\u{feff}  Call the bank \r\nabout the card  ");
    assert_eq!(events[0].summary, "Call the bank");
    assert_eq!(events[0].description, "Call the bank \r\nabout the card");
    // A lone CR breaks no line of the summary.
    let events = routine_events(&r, "One\rline");
    assert_eq!(events[0].summary, "One\rline");
    assert_eq!(events[0].description, "");
}

#[test]
fn count_over_several_times_becomes_the_last_occurrence_as_until() {
    let r = routine(
        "routine:r1",
        saturday(),
        &[at(8, 0), at(20, 0)],
        Some("FREQ=DAILY;COUNT=5"),
    );
    let rules: Vec<_> = routine_events(&r, "Antibiotics")
        .into_iter()
        .map(|e| e.rule)
        .collect();
    let until = Some("FREQ=DAILY;UNTIL=20261026T080000".to_string());
    assert_eq!(rules, vec![until.clone(), until]);
}

#[test]
fn a_time_count_cuts_off_on_the_first_day_gets_no_event() {
    let r = routine(
        "routine:r1",
        saturday(),
        &[at(8, 0), at(20, 0)],
        Some("FREQ=DAILY;COUNT=1"),
    );
    let keys: Vec<_> = routine_events(&r, "Once")
        .iter()
        .filter_map(|e| e.start.map(|s| s.key()))
        .collect();
    assert_eq!(keys, ["2026-10-24T08:00"]);
}

#[test]
fn until_takes_the_type_of_dtstart() {
    let timed = routine(
        "routine:r1",
        saturday(),
        &[at(8, 0)],
        Some("FREQ=DAILY;UNTIL=20261031"),
    );
    assert_eq!(
        routine_events(&timed, "Pills")[0].rule.as_deref(),
        Some("FREQ=DAILY;UNTIL=20261031T235900")
    );
    let all_day = routine(
        "routine:r1",
        saturday(),
        &[],
        Some("FREQ=DAILY;UNTIL=20261031T120000Z"),
    );
    assert_eq!(
        routine_events(&all_day, "Water")[0].rule.as_deref(),
        Some("FREQ=DAILY;UNTIL=20261031")
    );
}

#[test]
fn times_from_byhour_become_events_the_rule_loses_them() {
    let r = routine(
        "routine:r1",
        saturday(),
        &[],
        Some("FREQ=DAILY;BYHOUR=8,20;BYMINUTE=30"),
    );
    let events = routine_events(&r, "Pills");
    let uids: Vec<_> = events.iter().map(|e| e.uid.as_str()).collect();
    assert_eq!(
        uids,
        ["routine:r1-0830@oldbrainfix", "routine:r1-2030@oldbrainfix"]
    );
    assert!(
        events
            .iter()
            .all(|e| e.rule.as_deref() == Some("FREQ=DAILY"))
    );
}

#[test]
fn a_schedule_that_never_happens_has_no_events() {
    let r = routine(
        "routine:r1",
        saturday(),
        &[at(8, 0)],
        Some("FREQ=DAILY;UNTIL=20261001"),
    );
    assert!(routine_events(&r, "Gone").is_empty());
}

#[test]
fn one_routine_or_all_of_them_in_turn() {
    let bins = routine(
        "routine:bins",
        saturday(),
        &[at(7, 0)],
        Some("FREQ=WEEKLY;BYDAY=TU;UNTIL=20261124"),
    );
    let party = routine("routine:party", Day::new(2026, 12, 31), &[], None);
    let text_of = |r: &Routine| r.id.trim_start_matches("routine:").to_string();
    let one = routines_ics([&bins], text_of, stamp(), DEFAULT_ALL_DAY_FROM);
    assert_eq!(
        one,
        write_ics(
            &routine_events(&bins, "bins"),
            stamp(),
            DEFAULT_ALL_DAY_FROM
        )
    );
    assert!(unfolded(&one).contains(&"DTSTART:20261027T070000".to_string()));
    assert!(
        unfolded(&one).contains(&"RRULE:FREQ=WEEKLY;UNTIL=20261124T235900;BYDAY=TU".to_string())
    );
    let all = routines_ics([&party, &bins], text_of, stamp(), DEFAULT_ALL_DAY_FROM);
    let uids: Vec<_> = unfolded(&all)
        .into_iter()
        .filter(|l| l.starts_with("UID:"))
        .collect();
    assert_eq!(
        uids,
        [
            "UID:routine:party@oldbrainfix",
            "UID:routine:bins-0700@oldbrainfix"
        ]
    );
    assert!(all.contains("SUMMARY:party\r\n"));
}
