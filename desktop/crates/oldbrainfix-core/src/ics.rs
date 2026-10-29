//! Routines and Reminders as calendar events, as the Dart core's
//! `ics.dart` writes them: the iCalendar file (RFC 5545) for an export
//! of all of them or to share one, byte for byte the same.
//!
//! Times are floating: the Device's wall clock without a zone, as the
//! schedule itself is (ADR 0006). An 08:00 Routine is 08:00 wherever the
//! calendar is read, as the app's own alarm is. A TZID would need a
//! VTIMEZONE with daylight-saving rules the core does not have.

use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::alarms::ALL_DAY_ALARM_TIME;
use crate::routines::Routine;
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Until};
use crate::transcript_prompts::dart_trim;

/// One calendar event: a Reminder, or one time of day of a Routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    /// Stable per Routine and time of day, so importing an updated export
    /// replaces events instead of doubling them.
    pub uid: String,
    /// One line: what shows in the calendar's grid.
    pub summary: String,
    /// The full text; empty when `summary` already says it all.
    pub description: String,
    /// The first Occurrence; `None` for an event without a time, which
    /// the file leaves out.
    pub start: Option<Occurrence>,
    /// The RRULE value for the days after `start`, in step with it;
    /// `None` for one time only.
    pub rule: Option<String>,
}

impl CalendarEvent {
    /// How long a timed event lasts in the calendar, in minutes.
    pub const LENGTH_MINUTES: u32 = 15;
}

/// The events of `routine`, whose Note reads `text`: one per time of day,
/// each repeating on the Routine's days, or one all-day event; none when
/// the schedule has no Occurrence at all.
pub fn routine_events(routine: &Routine, text: &str) -> Vec<CalendarEvent> {
    let schedule = &routine.schedule;
    let Some(first) = schedule.next(schedule.start, 1).first().copied() else {
        return Vec::new();
    };
    let (summary, description) = split(text);
    let times = schedule.day_times();
    let rule_for = |time: Option<ClockTime>| -> Option<String> {
        let rule = schedule.rule.as_ref()?;
        let mut count = rule.count;
        let mut until = rule.until;
        if let Some(n) = count
            && times.len() > 1
        {
            // COUNT counts every time of day; per event the same last
            // Occurrence as UNTIL says it once for all of them.
            let n = usize::try_from(n).unwrap_or(0);
            if let Some(last) = schedule.next(first.day, n).last() {
                count = None;
                until = Some(Until {
                    day: last.day,
                    time: last.time,
                });
            }
        }
        // UNTIL has DTSTART's type: a date for all day, a time otherwise.
        let until = until.map(|u| Until {
            day: u.day,
            time: time.map(|_| {
                u.time.unwrap_or(ClockTime {
                    hour: 23,
                    minute: 59,
                })
            }),
        });
        Some(
            Recurrence {
                frequency: rule.frequency,
                interval: rule.interval,
                by_day: rule.by_day.clone(),
                by_month_day: rule.by_month_day.clone(),
                by_month: rule.by_month.clone(),
                by_hour: Vec::new(),
                by_minute: Vec::new(),
                count,
                until,
                week_start: rule.week_start,
            }
            .format(),
        )
    };
    // A time cut off on the first day by COUNT or UNTIL never comes; the
    // others start there, as DTSTART always counts as an Occurrence.
    schedule
        .on(first.day)
        .into_iter()
        .map(|start| CalendarEvent {
            uid: match start.time {
                None => format!("{}@oldbrainfix", routine.id),
                Some(t) => format!("{}-{:02}{:02}@oldbrainfix", routine.id, t.hour, t.minute),
            },
            summary: summary.clone(),
            description: description.clone(),
            start: Some(start),
            rule: rule_for(start.time),
        })
        .collect()
}

/// `routines` as one iCalendar file, every event of each in turn, each
/// titled by `text_of`: the export of all of them, or one Routine with
/// its whole series and its end, or one Reminder, to share.
pub fn routines_ics<'a>(
    routines: impl IntoIterator<Item = &'a Routine>,
    mut text_of: impl FnMut(&Routine) -> String,
    stamp: DateTime<Utc>,
) -> String {
    let events: Vec<CalendarEvent> = routines
        .into_iter()
        .flat_map(|routine| routine_events(routine, &text_of(routine)))
        .collect();
    write_ics(&events, stamp)
}

/// `events` as an iCalendar file (RFC 5545), each with an alarm when it
/// starts (an all-day one at [`ALL_DAY_ALARM_TIME`], as the app rings
/// it). Events without a start are left out. `stamp` is when it was
/// made.
pub fn write_ics(events: &[CalendarEvent], stamp: DateTime<Utc>) -> String {
    let mut lines = Vec::from(
        [
            "BEGIN:VCALENDAR",
            "VERSION:2.0",
            "PRODID:-//Old Brain Fix//Routines//EN",
            "CALSCALE:GREGORIAN",
            "METHOD:PUBLISH",
        ]
        .map(String::from),
    );
    let dtstamp = utc_stamp(stamp);
    for event in events {
        let Some(start) = event.start else {
            continue;
        };
        lines.push("BEGIN:VEVENT".into());
        lines.push(format!("UID:{}", escape(&event.uid)));
        lines.push(format!("DTSTAMP:{dtstamp}"));
        match start.time {
            None => {
                lines.push(format!("DTSTART;VALUE=DATE:{}", date(start.day)));
                lines.push(format!("DTEND;VALUE=DATE:{}", date(start.day.add_days(1))));
            }
            Some(time) => {
                lines.push(format!("DTSTART:{}T{}", date(start.day), clock(time)));
                lines.push(format!("DURATION:PT{}M", CalendarEvent::LENGTH_MINUTES));
            }
        }
        if let Some(rule) = &event.rule {
            lines.push(format!("RRULE:{rule}"));
        }
        lines.push(format!("SUMMARY:{}", escape(&event.summary)));
        if !event.description.is_empty() {
            lines.push(format!("DESCRIPTION:{}", escape(&event.description)));
        }
        let trigger = match start.time {
            None => format!(
                "PT{}M",
                ALL_DAY_ALARM_TIME.hour * 60 + ALL_DAY_ALARM_TIME.minute
            ),
            Some(_) => "PT0M".to_string(),
        };
        lines.push("BEGIN:VALARM".into());
        lines.push("ACTION:DISPLAY".into());
        lines.push(format!("DESCRIPTION:{}", escape(&event.summary)));
        lines.push(format!("TRIGGER:{trigger}"));
        lines.push("END:VALARM".into());
        lines.push("END:VEVENT".into());
    }
    lines.push("END:VCALENDAR".into());
    let mut out = String::new();
    for line in &lines {
        fold(line, &mut out);
    }
    out
}

/// The first line of `text` as the summary, and the whole text as the
/// description when there is more than that line; trimmed as Dart's
/// `trim` does, lines broken at `\n` or `\r\n`, as the Dart core's.
fn split(text: &str) -> (String, String) {
    let trimmed = dart_trim(text);
    let mut lines = trimmed.split('\n');
    let first = lines.next().unwrap_or_default();
    let first = first.strip_suffix('\r').unwrap_or(first);
    let more = lines.next().is_some();
    let description = if more {
        trimmed.to_string()
    } else {
        String::new()
    };
    (dart_trim(first).to_string(), description)
}

fn date(day: Day) -> String {
    format!("{:04}{:02}{:02}", day.year, day.month, day.day)
}

fn clock(time: ClockTime) -> String {
    format!("{:02}{:02}00", time.hour, time.minute)
}

fn utc_stamp(instant: DateTime<Utc>) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        instant.year(),
        instant.month(),
        instant.day(),
        instant.hour(),
        instant.minute(),
        instant.second()
    )
}

/// TEXT escaping (RFC 5545, 3.3.11): backslash, semicolon and comma,
/// and each `\r\n`, `\r` or `\n` as one `\n`.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\r' => {
                chars.next_if_eq(&'\n');
                out.push_str("\\n");
            }
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

/// `line` with its CRLF onto `out`, folded so no physical line is longer
/// than 75 octets (RFC 5545, 3.1); a continuation starts with a space,
/// and no UTF-8 sequence is cut.
fn fold(line: &str, out: &mut String) {
    const MAX: usize = 75;
    let mut octets = 0;
    for c in line.chars() {
        let size = c.len_utf8();
        if octets + size > MAX {
            out.push_str("\r\n ");
            octets = 1;
        }
        out.push(c);
        octets += size;
    }
    out.push_str("\r\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
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
        let ics = write_ics(&routine_events(&r, "Pills"), stamp());
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:"));
        assert!(ics.ends_with("END:VEVENT\r\nEND:VCALENDAR\r\n"));
        let bare = ics.replace("\r\n", "");
        assert!(!bare.contains('\n') && !bare.contains('\r'));
        assert!(unfolded(&ics).contains(&"DTSTAMP:20261024T070509Z".to_string()));
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
        assert!(!write_ics(&[event], stamp()).contains("VEVENT"));
    }

    #[test]
    fn backslash_semicolon_comma_and_line_breaks_are_escaped() {
        let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
        let ics = write_ics(
            &routine_events(&r, "Buy a\\b; milk, eggs\r\nand bread\nnow\rthen"),
            stamp(),
        );
        let lines = unfolded(&ics);
        assert!(lines.contains(&r"SUMMARY:Buy a\\b\; milk\, eggs".to_string()));
        assert!(
            lines
                .contains(&r"DESCRIPTION:Buy a\\b\; milk\, eggs\nand bread\nnow\nthen".to_string())
        );
    }

    #[test]
    fn long_lines_fold_at_75_octets_without_cutting_a_character() {
        let text = format!("{}Ende", "Medikament für Oma ü 💊 ".repeat(8));
        let r = routine("routine:r1", saturday(), &[at(8, 0)], None);
        let ics = write_ics(&routine_events(&r, &text), stamp());
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
        let ics = write_ics(&routine_events(&r, &text), stamp());
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
        let one = routines_ics([&bins], text_of, stamp());
        assert_eq!(one, write_ics(&routine_events(&bins, "bins"), stamp()));
        assert!(unfolded(&one).contains(&"DTSTART:20261027T070000".to_string()));
        assert!(
            unfolded(&one)
                .contains(&"RRULE:FREQ=WEEKLY;UNTIL=20261124T235900;BYDAY=TU".to_string())
        );
        let all = routines_ics([&party, &bins], text_of, stamp());
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
}
