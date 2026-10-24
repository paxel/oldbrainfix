//! When a Routine happens (ADR 0006), as the Dart core's `schedule.dart`
//! has it: a start day, the times of day, and an iCalendar RRULE for the
//! days, or none for a Reminder that happens once.
//!
//! Everything here is the Device's local wall clock with no zone: a day
//! is a calendar day and a time is what the clock on the wall shows. An
//! 08:00 Routine stays at 08:00 across a daylight-saving switch because
//! nothing is ever counted in hours. Turning a wall-clock Occurrence into
//! an instant is the alarm plan's business (`alarms::instant_of`).

use std::collections::BTreeSet;
use std::fmt;

use chrono::{DateTime, Datelike, NaiveDateTime, Utc};

/// Turns an instant into the Device's local wall clock. The app hands in
/// the system's local time; tests hand in a fixed zone.
pub type WallClock = dyn Fn(DateTime<Utc>) -> NaiveDateTime;

/// A day on the wall calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

impl Day {
    /// `year`, `month` and `day` as a calendar normalises them, as Dart's
    /// `DateTime.utc` does: the 32nd of January is the 1st of February,
    /// month 13 is next January.
    pub fn new(year: i64, month: i64, day: i64) -> Day {
        let year = year + (month - 1).div_euclid(12);
        let month = (month - 1).rem_euclid(12) + 1;
        Day::from_days(days_from_civil(year, month, 1) + day - 1)
    }

    fn from_days(days: i64) -> Day {
        let (year, month, day) = civil_from_days(days);
        Day {
            year,
            month: month as u32,
            day: day as u32,
        }
    }

    /// Days since 1970-01-01.
    fn days(self) -> i64 {
        days_from_civil(self.year, i64::from(self.month), i64::from(self.day))
    }

    /// The day `wall` shows.
    pub fn of(wall: NaiveDateTime) -> Day {
        Day::new(
            i64::from(wall.year()),
            i64::from(wall.month()),
            i64::from(wall.day()),
        )
    }

    /// The day of `now` on the wall clock `wall`.
    pub fn today(now: DateTime<Utc>, wall: &WallClock) -> Day {
        Day::of(wall(now))
    }

    /// Parses `YYYY-MM-DD`; `None` when it is not a valid day.
    pub fn parse(text: &str) -> Option<Day> {
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let y = digits(&text[0..4])?;
        let m = digits(&text[5..7])?;
        let d = digits(&text[8..10])?;
        let day = Day::new(y, m, d);
        (day.year == y && i64::from(day.month) == m && i64::from(day.day) == d).then_some(day)
    }

    /// 1 for Monday to 7 for Sunday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday.
        ((self.days() + 3).rem_euclid(7) + 1) as u32
    }

    pub fn add_days(self, days: i64) -> Day {
        Day::from_days(self.days() + days)
    }

    /// How many days lie from this day to `other`; negative when before.
    pub fn days_until(self, other: Day) -> i64 {
        other.days() - self.days()
    }

    /// The `YYYYMMDD` an RRULE writes.
    fn compact(self) -> String {
        self.to_string().replace('-', "")
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The proleptic Gregorian date `days` after 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `text` as a number when it is ASCII digits only.
fn digits(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn month_length(year: i64, month: u32) -> u32 {
    Day::new(year, i64::from(month) + 1, 0).day
}

/// A time of day on the wall clock, to the minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime {
    pub hour: u32,
    pub minute: u32,
}

impl ClockTime {
    /// `None` when `hour:minute` is no time of day.
    pub fn new(hour: u32, minute: u32) -> Option<ClockTime> {
        (hour <= 23 && minute <= 59).then_some(ClockTime { hour, minute })
    }

    /// Parses `HH:MM`; `None` when it is not a time of day.
    pub fn parse(text: &str) -> Option<ClockTime> {
        let b = text.as_bytes();
        if b.len() != 5 || b[2] != b':' {
            return None;
        }
        let hour = digits(&text[0..2])?;
        let minute = digits(&text[3..5])?;
        ClockTime::new(u32::try_from(hour).ok()?, u32::try_from(minute).ok()?)
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

/// One due moment of a Routine (CONTEXT.md): a day and a time of day, or
/// the whole day. Ordered by day, within a day all-day first, then by
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Occurrence {
    pub day: Day,
    /// `None` for an all-day Occurrence.
    pub time: Option<ClockTime>,
}

impl Occurrence {
    pub fn new(day: Day, time: Option<ClockTime>) -> Occurrence {
        Occurrence { day, time }
    }

    pub fn all_day(&self) -> bool {
        self.time.is_none()
    }

    /// Names the Occurrence for good: `2026-10-25` or `2026-10-25T08:00`.
    pub fn key(&self) -> String {
        match self.time {
            None => self.day.to_string(),
            Some(time) => format!("{}T{time}", self.day),
        }
    }

    /// Parses a [`Occurrence::key`]; `None` when it names no Occurrence.
    pub fn parse(key: &str) -> Option<Occurrence> {
        let mut parts = key.split('T');
        let day = Day::parse(parts.next()?)?;
        let rest: Vec<&str> = parts.collect();
        match rest[..] {
            [] => Some(Occurrence::new(day, None)),
            [time] => Some(Occurrence::new(day, Some(ClockTime::parse(time)?))),
            _ => None,
        }
    }
}

impl fmt::Display for Occurrence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key())
    }
}

/// How often an RRULE's periods come round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Frequency {
    fn name(self) -> &'static str {
        match self {
            Frequency::Daily => "DAILY",
            Frequency::Weekly => "WEEKLY",
            Frequency::Monthly => "MONTHLY",
            Frequency::Yearly => "YEARLY",
        }
    }
}

const WEEKDAY_CODES: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];

/// One BYDAY item: a weekday, with an ordinal for "the first Monday"
/// (1MO) or "the last Friday" (-1FR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeekdayRule {
    /// 1 for Monday to 7 for Sunday.
    pub weekday: u32,
    pub ordinal: Option<i64>,
}

impl fmt::Display for WeekdayRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ordinal) = self.ordinal {
            write!(f, "{ordinal}")?;
        }
        f.write_str(WEEKDAY_CODES[(self.weekday - 1) as usize])
    }
}

/// The end an RRULE's UNTIL sets, inclusive, read as wall clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Until {
    pub day: Day,
    pub time: Option<ClockTime>,
}

/// Why an RRULE was refused: what is wrong, and the part it is wrong in,
/// as the Dart core's `FormatException` says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    pub message: String,
    pub source: String,
}

impl RuleError {
    fn new(message: &str, source: impl Into<String>) -> RuleError {
        RuleError {
            message: message.to_string(),
            source: source.into(),
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.source)
    }
}

impl std::error::Error for RuleError {}

const SUPPORTED_KEYS: [&str; 11] = [
    "FREQ",
    "INTERVAL",
    "BYDAY",
    "BYMONTHDAY",
    "BYMONTH",
    "BYHOUR",
    "BYMINUTE",
    "BYSECOND",
    "COUNT",
    "UNTIL",
    "WKST",
];

/// An iCalendar recurrence rule (RFC 5545, 3.3.10), the subset a Routine
/// needs: FREQ (DAILY, WEEKLY, MONTHLY, YEARLY), INTERVAL, BYDAY with
/// ordinals, BYMONTHDAY (negative from the month's end), BYMONTH,
/// BYHOUR, BYMINUTE, COUNT, UNTIL and WKST.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Recurrence {
    pub frequency: Frequency,
    pub interval: i64,
    pub by_day: Vec<WeekdayRule>,
    pub by_month_day: Vec<i64>,
    pub by_month: Vec<i64>,
    pub by_hour: Vec<i64>,
    pub by_minute: Vec<i64>,
    pub count: Option<i64>,
    pub until: Option<Until>,
    /// 1 for Monday to 7 for Sunday.
    pub week_start: u32,
}

impl Recurrence {
    /// Every `interval` periods of `frequency`, nothing more.
    pub fn every(frequency: Frequency, interval: i64) -> Recurrence {
        Recurrence {
            frequency,
            interval: interval.max(1),
            by_day: Vec::new(),
            by_month_day: Vec::new(),
            by_month: Vec::new(),
            by_hour: Vec::new(),
            by_minute: Vec::new(),
            count: None,
            until: None,
            week_start: 1,
        }
    }

    /// Parses an RRULE value, with or without the `RRULE:` prefix, in any
    /// case, refusing what the Dart core refuses with the same message.
    pub fn parse(text: &str) -> Result<Recurrence, RuleError> {
        let upper = text.trim().to_uppercase();
        let body = upper.strip_prefix("RRULE:").unwrap_or(&upper);
        if body.is_empty() {
            return Err(RuleError::new("empty rule", ""));
        }
        let mut parts: Vec<(String, String)> = Vec::new();
        for part in body.split(';') {
            if part.is_empty() {
                continue;
            }
            let eq = part.find('=').unwrap_or(0);
            if eq == 0 {
                return Err(RuleError::new("not KEY=VALUE", part));
            }
            let key = &part[..eq];
            if parts.iter().any(|(k, _)| k == key) {
                return Err(RuleError::new("twice", key));
            }
            parts.push((key.to_string(), part[eq + 1..].to_string()));
        }
        for (key, _) in &parts {
            if !SUPPORTED_KEYS.contains(&key.as_str()) {
                return Err(RuleError::new("not supported", key.clone()));
            }
        }
        let get = |key: &str| {
            parts
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        let frequency = match get("FREQ") {
            Some("DAILY") => Frequency::Daily,
            Some("WEEKLY") => Frequency::Weekly,
            Some("MONTHLY") => Frequency::Monthly,
            Some("YEARLY") => Frequency::Yearly,
            None => return Err(RuleError::new("FREQ missing", "FREQ")),
            Some(other) => return Err(RuleError::new("FREQ not supported", other)),
        };
        let mut by_day = Vec::new();
        for item in list(get("BYDAY")) {
            by_day.push(weekday_rule(item, frequency)?);
        }
        let by_month_day = numbers(get("BYMONTHDAY"), "BYMONTHDAY", -31, 31)?;
        if by_month_day.contains(&0) {
            return Err(RuleError::new("BYMONTHDAY 0", "BYMONTHDAY"));
        }
        if !by_month_day.is_empty() && frequency == Frequency::Weekly {
            return Err(RuleError::new("BYMONTHDAY with WEEKLY", "BYMONTHDAY"));
        }
        if let Some(seconds) = get("BYSECOND")
            && seconds != "0"
        {
            return Err(RuleError::new("BYSECOND not supported", seconds));
        }
        if get("COUNT").is_some() && get("UNTIL").is_some() {
            return Err(RuleError::new("COUNT and UNTIL together", "COUNT, UNTIL"));
        }
        let week_start = match get("WKST") {
            None => 1,
            Some(code) => weekday(code)?,
        };
        let interval = number(get("INTERVAL"), "INTERVAL", 1, 9999)?.unwrap_or(1);
        let by_month = numbers(get("BYMONTH"), "BYMONTH", 1, 12)?;
        let by_hour = numbers(get("BYHOUR"), "BYHOUR", 0, 23)?;
        let by_minute = numbers(get("BYMINUTE"), "BYMINUTE", 0, 59)?;
        let count = number(get("COUNT"), "COUNT", 1, 99_999)?;
        let until = match get("UNTIL") {
            None => None,
            Some(text) => Some(until(text)?),
        };
        Ok(Recurrence {
            frequency,
            interval,
            by_day,
            by_month_day,
            by_month,
            by_hour,
            by_minute,
            count,
            until,
            week_start,
        })
    }

    /// The RRULE value, without the `RRULE:` prefix, in RFC order.
    pub fn format(&self) -> String {
        let join = |values: &[i64]| {
            values
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let mut parts = vec![format!("FREQ={}", self.frequency.name())];
        if self.interval != 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={count}"));
        }
        if let Some(until) = self.until {
            let time = until
                .time
                .map(|t| format!("T{:02}{:02}00", t.hour, t.minute))
                .unwrap_or_default();
            parts.push(format!("UNTIL={}{time}", until.day.compact()));
        }
        if !self.by_month.is_empty() {
            parts.push(format!("BYMONTH={}", join(&self.by_month)));
        }
        if !self.by_month_day.is_empty() {
            parts.push(format!("BYMONTHDAY={}", join(&self.by_month_day)));
        }
        if !self.by_day.is_empty() {
            let days: Vec<String> = self.by_day.iter().map(ToString::to_string).collect();
            parts.push(format!("BYDAY={}", days.join(",")));
        }
        if !self.by_hour.is_empty() {
            parts.push(format!("BYHOUR={}", join(&self.by_hour)));
        }
        if !self.by_minute.is_empty() {
            parts.push(format!("BYMINUTE={}", join(&self.by_minute)));
        }
        if self.week_start != 1 {
            parts.push(format!(
                "WKST={}",
                WEEKDAY_CODES[(self.week_start - 1) as usize]
            ));
        }
        parts.join(";")
    }

    /// The times of day the rule itself names, or `None` when it names
    /// none and the Schedule's own times count. A BYMINUTE without BYHOUR
    /// is taken at every hour, as RFC 5545 expands it.
    pub fn times(&self) -> Option<Vec<ClockTime>> {
        if self.by_hour.is_empty() && self.by_minute.is_empty() {
            return None;
        }
        let hours: Vec<i64> = if self.by_hour.is_empty() {
            (0..24).collect()
        } else {
            self.by_hour.clone()
        };
        let minutes: Vec<i64> = if self.by_minute.is_empty() {
            vec![0]
        } else {
            self.by_minute.clone()
        };
        let mut times = BTreeSet::new();
        for h in &hours {
            for m in &minutes {
                if let Some(t) = ClockTime::new(*h as u32, *m as u32) {
                    times.insert(t);
                }
            }
        }
        Some(times.into_iter().collect())
    }

    /// Hands `visit` the days the rule falls on, from `start` (which
    /// counts only when the rule falls on it) up to `to`, in order, until
    /// it answers false. With `from`, periods wholly before it are
    /// skipped; COUNT needs every day, so it never skips.
    pub fn each_day(
        &self,
        start: Day,
        from: Option<Day>,
        to: Day,
        mut visit: impl FnMut(Day) -> bool,
    ) {
        let interval = self.interval;
        let week_zero = start
            .add_days(-(i64::from(start.weekday()) - i64::from(self.week_start)).rem_euclid(7));
        let mut k = match from {
            Some(from) if self.count.is_none() => match self.frequency {
                Frequency::Daily => start.days_until(from) / interval,
                Frequency::Weekly => week_zero.days_until(from) / 7 / interval,
                Frequency::Monthly => {
                    ((from.year - start.year) * 12 + i64::from(from.month) - i64::from(start.month))
                        / interval
                }
                Frequency::Yearly => (from.year - start.year) / interval,
            },
            _ => 0,
        };
        if k < 0 {
            k = 0;
        }
        loop {
            let step = k * interval;
            let (period_start, candidates) = match self.frequency {
                Frequency::Daily => {
                    let day = start.add_days(step);
                    let keep = if self.daily_keeps(day) {
                        vec![day]
                    } else {
                        Vec::new()
                    };
                    (day, keep)
                }
                Frequency::Weekly => {
                    let begins = week_zero.add_days(7 * step);
                    (begins, self.week_days(begins, start))
                }
                Frequency::Monthly => {
                    let begins = Day::new(start.year, i64::from(start.month) + step, 1);
                    let days = if self.by_month.is_empty()
                        || self.by_month.contains(&i64::from(begins.month))
                    {
                        self.month_days(begins.year, begins.month, start.day)
                    } else {
                        Vec::new()
                    };
                    (begins, days)
                }
                Frequency::Yearly => {
                    let begins = Day::new(start.year + step, 1, 1);
                    (begins, self.year_days(begins.year, start))
                }
            };
            if period_start > to {
                return;
            }
            for day in candidates {
                if day < start || day > to {
                    continue;
                }
                if !visit(day) {
                    return;
                }
            }
            k += 1;
        }
    }

    fn daily_keeps(&self, day: Day) -> bool {
        if !self.by_month.is_empty() && !self.by_month.contains(&i64::from(day.month)) {
            return false;
        }
        if !self.by_month_day.is_empty()
            && !self
                .resolve_month_days(day.year, day.month)
                .contains(&day.day)
        {
            return false;
        }
        if !self.by_day.is_empty() && !self.by_day.iter().any(|r| r.weekday == day.weekday()) {
            return false;
        }
        true
    }

    fn week_days(&self, begins: Day, start: Day) -> Vec<Day> {
        let mut weekdays: Vec<u32> = Vec::new();
        if self.by_day.is_empty() {
            weekdays.push(start.weekday());
        } else {
            for rule in &self.by_day {
                if !weekdays.contains(&rule.weekday) {
                    weekdays.push(rule.weekday);
                }
            }
        }
        let mut days: Vec<Day> = weekdays
            .into_iter()
            .map(|w| begins.add_days((i64::from(w) - i64::from(self.week_start)).rem_euclid(7)))
            .filter(|d| self.by_month.is_empty() || self.by_month.contains(&i64::from(d.month)))
            .collect();
        days.sort();
        days
    }

    fn year_days(&self, year: i64, start: Day) -> Vec<Day> {
        if self.by_month.is_empty() && self.by_month_day.is_empty() && !self.by_day.is_empty() {
            // BYDAY over the whole year: 1MO is the year's first Monday.
            let first = Day::new(year, 1, 1);
            let length = first.days_until(Day::new(year + 1, 1, 1));
            let span: Vec<Day> = (0..length).map(|i| first.add_days(i)).collect();
            return self.weekday_picks(&span);
        }
        let mut months: Vec<i64> = if !self.by_month.is_empty() {
            self.by_month.clone()
        } else if !self.by_month_day.is_empty() {
            (1..=12).collect()
        } else {
            vec![i64::from(start.month)]
        };
        months.sort();
        months
            .into_iter()
            .flat_map(|m| self.month_days(year, m as u32, start.day))
            .collect()
    }

    /// The days of `month` the rule picks; `default_day` when it names no
    /// day, skipped where the month is too short (RFC 5545).
    fn month_days(&self, year: i64, month: u32, default_day: u32) -> Vec<Day> {
        let length = month_length(year, month);
        let day = |d: u32| Day::new(year, i64::from(month), i64::from(d));
        if self.by_month_day.is_empty() && self.by_day.is_empty() {
            return if default_day <= length {
                vec![day(default_day)]
            } else {
                Vec::new()
            };
        }
        let by_date = (!self.by_month_day.is_empty()).then(|| self.resolve_month_days(year, month));
        let by_weekday: Option<BTreeSet<u32>> = (!self.by_day.is_empty()).then(|| {
            let span: Vec<Day> = (1..=length).map(day).collect();
            self.weekday_picks(&span)
                .into_iter()
                .map(|d| d.day)
                .collect()
        });
        (1..=length)
            .filter(|d| by_date.as_ref().is_none_or(|s| s.contains(d)))
            .filter(|d| by_weekday.as_ref().is_none_or(|s| s.contains(d)))
            .map(day)
            .collect()
    }

    /// The days of `span` each BYDAY item picks: every such weekday, or
    /// the nth one from the start or the end.
    fn weekday_picks(&self, span: &[Day]) -> Vec<Day> {
        let mut picked = BTreeSet::new();
        for rule in &self.by_day {
            let matching: Vec<Day> = span
                .iter()
                .copied()
                .filter(|d| d.weekday() == rule.weekday)
                .collect();
            match rule.ordinal {
                None => picked.extend(matching),
                Some(ordinal) => {
                    let index = if ordinal > 0 {
                        ordinal - 1
                    } else {
                        matching.len() as i64 + ordinal
                    };
                    if index >= 0
                        && let Some(day) = matching.get(index as usize)
                    {
                        picked.insert(*day);
                    }
                }
            }
        }
        picked.into_iter().collect()
    }

    fn resolve_month_days(&self, year: i64, month: u32) -> BTreeSet<u32> {
        let length = i64::from(month_length(year, month));
        self.by_month_day
            .iter()
            .filter_map(|&d| {
                if d > 0 && d <= length {
                    Some(d as u32)
                } else if d < 0 && -d <= length {
                    Some((length + 1 + d) as u32)
                } else {
                    None
                }
            })
            .collect()
    }
}

fn list(value: Option<&str>) -> Vec<&str> {
    value.map(|v| v.split(',').collect()).unwrap_or_default()
}

/// A number as Dart's `int.tryParse` reads it: an optional sign, then
/// digits.
fn integer(text: &str) -> Option<i64> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn number(value: Option<&str>, key: &str, min: i64, max: i64) -> Result<Option<i64>, RuleError> {
    let Some(value) = value else {
        return Ok(None);
    };
    match integer(value) {
        Some(n) if n >= min && n <= max => Ok(Some(n)),
        _ => Err(RuleError::new("out of range", format!("{key}={value}"))),
    }
}

fn numbers(value: Option<&str>, key: &str, min: i64, max: i64) -> Result<Vec<i64>, RuleError> {
    let mut out = Vec::new();
    for item in list(value) {
        if let Some(n) = number(Some(item), key, min, max)? {
            out.push(n);
        }
    }
    Ok(out)
}

fn weekday(code: &str) -> Result<u32, RuleError> {
    WEEKDAY_CODES
        .iter()
        .position(|c| *c == code)
        .map(|i| i as u32 + 1)
        .ok_or_else(|| RuleError::new("not a weekday", code))
}

fn weekday_rule(item: &str, frequency: Frequency) -> Result<WeekdayRule, RuleError> {
    // ^([+-]?\d{1,2})?([A-Z]{2})$
    let not = || RuleError::new("not a weekday", item);
    if item.len() < 2 || !item.is_char_boundary(item.len() - 2) {
        return Err(not());
    }
    let (ordinal_text, code) = item.split_at(item.len() - 2);
    if !code.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(not());
    }
    let digits = ordinal_text
        .strip_prefix(['+', '-'])
        .unwrap_or(ordinal_text);
    let valid_ordinal = ordinal_text.is_empty()
        || ((1..=2).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()));
    if !valid_ordinal {
        return Err(not());
    }
    let weekday = weekday(code)?;
    if ordinal_text.is_empty() {
        return Ok(WeekdayRule {
            weekday,
            ordinal: None,
        });
    }
    let ordinal = integer(ordinal_text).ok_or_else(not)?;
    if ordinal == 0 || ordinal.abs() > 53 {
        return Err(RuleError::new("ordinal out of range", item));
    }
    if frequency != Frequency::Monthly && frequency != Frequency::Yearly {
        return Err(RuleError::new("ordinal only with MONTHLY or YEARLY", item));
    }
    Ok(WeekdayRule {
        weekday,
        ordinal: Some(ordinal),
    })
}

fn until(text: &str) -> Result<Until, RuleError> {
    // ^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2})Z?)?$
    let not_date = || RuleError::new("not a date", format!("UNTIL={text}"));
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if text.len() < 8 || !text.is_char_boundary(8) || !all_digits(&text[..8]) {
        return Err(not_date());
    }
    let rest = &text[8..];
    let time_text = match rest {
        "" => None,
        _ => {
            let body = rest.strip_prefix('T').ok_or_else(not_date)?;
            let body = body.strip_suffix('Z').unwrap_or(body);
            if body.len() != 6 || !all_digits(body) {
                return Err(not_date());
            }
            Some(body)
        }
    };
    let day = Day::parse(&format!("{}-{}-{}", &text[0..4], &text[4..6], &text[6..8]))
        .ok_or_else(not_date)?;
    let Some(time_text) = time_text else {
        return Ok(Until { day, time: None });
    };
    let time = ClockTime::parse(&format!("{}:{}", &time_text[0..2], &time_text[2..4]))
        .ok_or_else(|| RuleError::new("not a time", format!("UNTIL={text}")))?;
    Ok(Until {
        day,
        time: Some(time),
    })
}

/// When a Routine happens: from `start`, at `times` (all day when there
/// are none), on the days `rule` picks, or only on `start` when there is
/// no rule. A Reminder is a Schedule without a rule (ADR 0005). The
/// Schedule is fixed (ADR 0006): it never reads what was Done.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Schedule {
    /// The first day; for a Reminder its only one.
    pub start: Day,
    /// The times of day, ascending, without repeats; empty for all day.
    pub times: Vec<ClockTime>,
    /// The repeat rule; `None` for one time only.
    pub rule: Option<Recurrence>,
}

impl Schedule {
    /// How far ahead [`Schedule::next`] looks.
    pub const LOOK_AHEAD_YEARS: i64 = 50;

    /// `None` for a Reminder with more than one time: it happens once.
    pub fn new(start: Day, times: &[ClockTime], rule: Option<Recurrence>) -> Option<Schedule> {
        let times: Vec<ClockTime> = times
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if rule.is_none() && times.len() > 1 {
            return None;
        }
        Some(Schedule { start, times, rule })
    }

    /// Happens only once: a Reminder.
    pub fn once(&self) -> bool {
        self.rule.is_none()
    }

    /// The times of day each day gets; empty for all day.
    pub fn day_times(&self) -> Vec<ClockTime> {
        self.rule
            .as_ref()
            .and_then(Recurrence::times)
            .unwrap_or_else(|| self.times.clone())
    }

    /// The Occurrences from `from` to `to`, both days included, in order.
    pub fn between(&self, from: Day, to: Day) -> Vec<Occurrence> {
        let mut out = Vec::new();
        self.each(from, to, |o| {
            out.push(o);
            true
        });
        out
    }

    /// The Occurrences on `day`, all-day first, then by time.
    pub fn on(&self, day: Day) -> Vec<Occurrence> {
        self.between(day, day)
    }

    /// The next `n` Occurrences from `from` on, within
    /// [`Schedule::LOOK_AHEAD_YEARS`].
    pub fn next(&self, from: Day, n: usize) -> Vec<Occurrence> {
        let mut out = Vec::new();
        if n == 0 {
            return out;
        }
        let to = Day::new(
            from.year + Self::LOOK_AHEAD_YEARS,
            i64::from(from.month),
            i64::from(from.day),
        );
        self.each(from, to, |o| {
            out.push(o);
            out.len() < n
        });
        out
    }

    /// Whether `occurrence` is one of this Schedule's.
    pub fn has(&self, occurrence: &Occurrence) -> bool {
        self.on(occurrence.day).contains(occurrence)
    }

    fn each(&self, from: Day, to: Day, mut visit: impl FnMut(Occurrence) -> bool) {
        let times = self.day_times();
        let slots: Vec<Option<ClockTime>> = if times.is_empty() {
            vec![None]
        } else {
            times.into_iter().map(Some).collect()
        };
        let Some(rule) = &self.rule else {
            if self.start <= to && self.start >= from {
                for time in slots {
                    if !visit(Occurrence::new(self.start, time)) {
                        return;
                    }
                }
            }
            return;
        };
        let mut counted = 0i64;
        rule.each_day(self.start, Some(from), to, |day| {
            for time in &slots {
                let occurrence = Occurrence::new(day, *time);
                if beyond(&occurrence, rule, counted) {
                    return false;
                }
                counted += 1;
                if day >= from && !visit(occurrence) {
                    return false;
                }
            }
            true
        });
    }
}

/// Whether `occurrence`, with `counted` Occurrences before it, lies past
/// the rule's COUNT or UNTIL. COUNT counts every Occurrence, each time of
/// day on its own.
fn beyond(occurrence: &Occurrence, rule: &Recurrence, counted: i64) -> bool {
    if let Some(count) = rule.count
        && counted >= count
    {
        return true;
    }
    let Some(until) = rule.until else {
        return false;
    };
    if occurrence.day != until.day {
        return occurrence.day > until.day;
    }
    matches!((occurrence.time, until.time), (Some(t), Some(last)) if t > last)
}

/// The ways the simple editor repeats (ADR 0006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepeatKind {
    /// Once: a Reminder.
    Once,
    /// Every N days, weeks, months or years.
    Every,
    /// On chosen weekdays, every week.
    Weekdays,
}

/// The unit of [`RepeatKind::Every`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepeatUnit {
    Days,
    Weeks,
    Months,
    Years,
}

/// A schedule the simple editor can show: once, every N days, weeks,
/// months or years, or chosen weekdays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleRepeat {
    pub kind: RepeatKind,
    pub every: i64,
    pub unit: RepeatUnit,
    /// 1 for Monday to 7 for Sunday, for [`RepeatKind::Weekdays`].
    pub weekdays: BTreeSet<u32>,
}

impl SimpleRepeat {
    pub fn once() -> SimpleRepeat {
        SimpleRepeat {
            kind: RepeatKind::Once,
            every: 1,
            unit: RepeatUnit::Days,
            weekdays: BTreeSet::new(),
        }
    }

    pub fn every(every: i64, unit: RepeatUnit) -> SimpleRepeat {
        SimpleRepeat {
            kind: RepeatKind::Every,
            every,
            unit,
            weekdays: BTreeSet::new(),
        }
    }

    pub fn weekdays(weekdays: BTreeSet<u32>) -> SimpleRepeat {
        SimpleRepeat {
            kind: RepeatKind::Weekdays,
            every: 1,
            unit: RepeatUnit::Weeks,
            weekdays,
        }
    }

    /// The RRULE for this repeat; `None` for once.
    pub fn to_rule(&self) -> Option<Recurrence> {
        match self.kind {
            RepeatKind::Once => None,
            RepeatKind::Every => Some(Recurrence::every(
                match self.unit {
                    RepeatUnit::Days => Frequency::Daily,
                    RepeatUnit::Weeks => Frequency::Weekly,
                    RepeatUnit::Months => Frequency::Monthly,
                    RepeatUnit::Years => Frequency::Yearly,
                },
                self.every,
            )),
            RepeatKind::Weekdays => {
                let mut rule = Recurrence::every(Frequency::Weekly, 1);
                rule.by_day = self
                    .weekdays
                    .iter()
                    .map(|&weekday| WeekdayRule {
                        weekday,
                        ordinal: None,
                    })
                    .collect();
                Some(rule)
            }
        }
    }

    /// What the simple editor shows for `rule`, or `None` when only the
    /// advanced mode can show it.
    pub fn of(rule: Option<&Recurrence>) -> Option<SimpleRepeat> {
        let Some(rule) = rule else {
            return Some(SimpleRepeat::once());
        };
        if !rule.by_month_day.is_empty()
            || !rule.by_month.is_empty()
            || !rule.by_hour.is_empty()
            || !rule.by_minute.is_empty()
            || rule.count.is_some()
            || rule.until.is_some()
            || rule.week_start != 1
            || rule.by_day.iter().any(|r| r.ordinal.is_some())
        {
            return None;
        }
        let unit = match rule.frequency {
            Frequency::Daily => RepeatUnit::Days,
            Frequency::Weekly => RepeatUnit::Weeks,
            Frequency::Monthly => RepeatUnit::Months,
            Frequency::Yearly => RepeatUnit::Years,
        };
        if rule.by_day.is_empty() {
            return Some(SimpleRepeat::every(rule.interval, unit));
        }
        if rule.frequency != Frequency::Weekly || rule.interval != 1 {
            return None;
        }
        Some(SimpleRepeat::weekdays(
            rule.by_day.iter().map(|r| r.weekday).collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i64, m: i64, d: i64) -> Day {
        Day::new(y, m, d)
    }

    fn at(h: u32, m: u32) -> ClockTime {
        ClockTime { hour: h, minute: m }
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
        let first = Recurrence::parse("FREQ=MONTHLY;BYDAY=1MO")?;
        assert_eq!(SimpleRepeat::of(Some(&first)), None);
        assert_eq!(SimpleRepeat::of(None), Some(SimpleRepeat::once()));
        Ok(())
    }
}
