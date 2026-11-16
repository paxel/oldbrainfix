//! Routines and Reminders as calendar events, as the Dart core's
//! `ics.dart` writes them: the iCalendar file (RFC 5545) for an export
//! of all of them or to share one, byte for byte the same.
//!
//! Times are floating: the Device's wall clock without a zone, as the
//! schedule itself is (ADR 0006). An 08:00 Routine is 08:00 wherever the
//! calendar is read, as the app's own alarm is. A TZID would need a
//! VTIMEZONE with daylight-saving rules the core does not have.

use chrono::{DateTime, Datelike, Timelike, Utc};

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
/// its whole series and its end, or one Reminder, to share. An all-day
/// event's alarm is at `all_day_from`, the exporting Device's time.
pub fn routines_ics<'a>(
    routines: impl IntoIterator<Item = &'a Routine>,
    mut text_of: impl FnMut(&Routine) -> String,
    stamp: DateTime<Utc>,
    all_day_from: ClockTime,
) -> String {
    let events: Vec<CalendarEvent> = routines
        .into_iter()
        .flat_map(|routine| routine_events(routine, &text_of(routine)))
        .collect();
    write_ics(&events, stamp, all_day_from)
}

/// `events` as an iCalendar file (RFC 5545), each with an alarm when it
/// starts (an all-day one at `all_day_from`, as this Device rings it).
/// Events without a start are left out. `stamp` is when it was made.
pub fn write_ics(
    events: &[CalendarEvent],
    stamp: DateTime<Utc>,
    all_day_from: ClockTime,
) -> String {
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
            None => format!("PT{}M", all_day_from.hour * 60 + all_day_from.minute),
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
mod tests;
