//! A schedule of the simple editor as a sentence (ADR 0044), as the Dart
//! core's `schedule_sentence.dart` has it: written in the app's language,
//! read back as typed, with the next words to offer while typing. The
//! sentence is never stored; the schedule's fields are.
//!
//! Offsets here are byte offsets into the text (fixture
//! `schedule_sentences.json` counts code points).

use std::collections::BTreeSet;
use std::ops::Range;

use chrono::{NaiveDateTime, Timelike};

use crate::schedule::{
    ClockTime, Day, RepeatKind, RepeatUnit, Schedule, SimpleRepeat, WeekdayRule,
};

/// A schedule the simple editor shows: how it repeats, with its end, its
/// first day and its times of day, ascending, none for all day. Unlike a
/// [`Schedule`] it may be one the editor cannot save yet: weekdays with
/// none picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleSchedule {
    pub repeat: SimpleRepeat,
    pub start: Day,
    pub times: Vec<ClockTime>,
}

impl SimpleSchedule {
    /// `times` ascending, each once.
    pub fn new(repeat: SimpleRepeat, start: Day, times: &[ClockTime]) -> SimpleSchedule {
        let times = times.iter().copied().collect::<BTreeSet<_>>();
        SimpleSchedule {
            repeat,
            start,
            times: times.into_iter().collect(),
        }
    }

    /// `schedule` as the simple editor shows it; `None` when only the
    /// advanced mode can.
    pub fn of(schedule: &Schedule) -> Option<SimpleSchedule> {
        let repeat = SimpleRepeat::of(schedule.rule.as_ref())?;
        Some(SimpleSchedule::new(repeat, schedule.start, &schedule.times))
    }

    /// The [`Schedule`] to store; `None` while weekdays have none picked
    /// or once has more than one time.
    pub fn to_schedule(&self) -> Option<Schedule> {
        if self.repeat.kind == RepeatKind::Weekdays && self.repeat.weekdays.is_empty() {
            return None;
        }
        Schedule::new(self.start, &self.times, self.repeat.to_rule())
    }
}

/// What a sentence read as: the schedule, or `None` when it names no
/// repeat; and the parts not understood, which changed nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceRead {
    pub schedule: Option<SimpleSchedule>,
    pub unknown: Vec<Range<usize>>,
}

/// The moment a sentence is read at, for its relative words (#271): the
/// day and the time on this Device's wall clock, and when its day starts
/// (`DeviceSettings::day_start`), at which "tomorrow" rings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentenceNow {
    pub today: Day,
    pub time: ClockTime,
    pub day_start: ClockTime,
}

impl SentenceNow {
    /// At `wall`, a time on the wall clock, to the minute.
    pub fn at(wall: NaiveDateTime, day_start: ClockTime) -> SentenceNow {
        SentenceNow {
            today: Day::of(wall),
            time: ClockTime {
                hour: wall.hour(),
                minute: wall.minute(),
            },
            day_start,
        }
    }
}

/// The chips over a new Reminder's schedule (#271), in their order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuickChip {
    InAnHour,
    Tonight,
    Tomorrow,
    InAWeek,
}

/// A chip with the day and time it sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickReminder {
    pub chip: QuickChip,
    pub day: Day,
    pub time: ClockTime,
}

impl QuickReminder {
    /// The Reminder the chip sets.
    pub fn schedule(&self) -> SimpleSchedule {
        SimpleSchedule::new(SimpleRepeat::once(), self.day, &[self.time])
    }
}

/// "Tonight", and "this evening" after a day.
const TONIGHT: ClockTime = ClockTime {
    hour: 19,
    minute: 0,
};

/// `minutes` after `now`, counted on the wall clock, the day carried.
fn later(now: &SentenceNow, minutes: i64) -> (Day, ClockTime) {
    let total = i64::from(now.time.hour) * 60 + i64::from(now.time.minute) + minutes;
    let time = ClockTime {
        hour: (total / 60 % 24) as u32,
        minute: (total % 60) as u32,
    };
    (now.today.add_days(total / (24 * 60)), time)
}

/// The chips' Reminders at `now`, as the words read them: "in 1 hour",
/// "tonight" while 19:00 is still to come, "tomorrow" and "in 1 week",
/// these two at the day start.
pub fn quick_reminders(now: &SentenceNow) -> Vec<QuickReminder> {
    let (day, time) = later(now, 60);
    let mut chips = vec![QuickReminder {
        chip: QuickChip::InAnHour,
        day,
        time,
    }];
    if now.time < TONIGHT {
        chips.push(QuickReminder {
            chip: QuickChip::Tonight,
            day: now.today,
            time: TONIGHT,
        });
    }
    chips.push(QuickReminder {
        chip: QuickChip::Tomorrow,
        day: now.today.add_days(1),
        time: now.day_start,
    });
    chips.push(QuickReminder {
        chip: QuickChip::InAWeek,
        day: now.today.add_days(7),
        time: now.day_start,
    });
    chips
}

/// A word to put in for the text in `range`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub range: Range<usize>,
    pub text: String,
}

/// `schedule` as a sentence in `locale` (`en` or `de`, `de_DE` counts as
/// `de`), written on `today`: its days without the year when they lie in
/// `today`'s, its start left out when it is `today`.
pub fn sentence(schedule: &SimpleSchedule, locale: &str, today: Day) -> String {
    let w = Words::of(locale);
    let repeat = &schedule.repeat;
    let date = |day: Day| w.date(day, today);
    let mut out = match repeat.kind {
        RepeatKind::Once => format!("{} {} {}", w.once_head(), w.on(), date(schedule.start)),
        RepeatKind::Every => w.every(repeat.every, repeat.unit),
        RepeatKind::Weekdays => {
            let mut head = w.weekdays_head().to_string();
            if !repeat.weekdays.is_empty() {
                let names: Vec<&str> = repeat
                    .weekdays
                    .iter()
                    .map(|&d| weekday_word(w.weekdays(), d))
                    .collect();
                head.push(' ');
                head.push_str(&names.join(", "));
            }
            head
        }
        RepeatKind::MonthDay => format!(
            "{} {}",
            w.every(repeat.every, RepeatUnit::Months),
            w.month_day(
                repeat
                    .month_day
                    .unwrap_or_else(|| i64::from(schedule.start.day))
            )
        ),
        RepeatKind::MonthWeekday => format!(
            "{} {}",
            w.every(repeat.every, RepeatUnit::Months),
            w.nth(repeat.nth.unwrap_or(WeekdayRule {
                weekday: schedule.start.weekday(),
                ordinal: Some(1),
            }))
        ),
    };
    if !schedule.times.is_empty() {
        let times: Vec<String> = schedule.times.iter().map(ToString::to_string).collect();
        out.push_str(&format!(" {} {}", w.at(), times.join(", ")));
    }
    if repeat.kind != RepeatKind::Once {
        if schedule.start != today {
            out.push_str(&format!(", {} {}", w.from(), date(schedule.start)));
        }
        if let Some(count) = repeat.count {
            if count == 1 {
                out.push_str(&format!(", {}", w.count_once()));
            } else {
                out.push_str(&format!(", {}", w.count(count)));
            }
        } else if let Some(until) = repeat.until {
            out.push_str(&format!(", {} {}", w.until(), date(until)));
        }
    }
    out
}

/// Reads `text` in `locale` on `today`: what is left out is all day,
/// from `today`, never ending, and a day without its year lies in
/// `today`'s. Parts not understood are skipped. The relative words are
/// not understood here; [`read_at`] reads them.
pub fn read(text: &str, locale: &str, today: Day) -> SentenceRead {
    read_with(text, locale, today, None)
}

/// Reads `text` in `locale` at `now` as [`read`] does on its day, and
/// also a one-time Reminder in relative words (#271): "today",
/// "tonight", "tomorrow", "in 2 hours" and in German "heute",
/// "heute Abend", "morgen", "in 2 Stunden".
pub fn read_at(text: &str, locale: &str, now: &SentenceNow) -> SentenceRead {
    read_with(text, locale, now.today, Some(*now))
}

fn read_with(text: &str, locale: &str, today: Day, now: Option<SentenceNow>) -> SentenceRead {
    let w = Words::of(locale);
    let mut parser = Parser::new(w, today, now);
    for token in tokens(text, w.german) {
        parser.take(&token);
    }
    parser.finish();
    let mut unknown: Vec<Range<usize>> = parser.unknown.iter().map(|t| t.range.clone()).collect();
    unknown.sort_by_key(|r| r.start);
    SentenceRead {
        schedule: parser.schedule(),
        unknown,
    }
}

/// The words that may come at the byte offset `cursor` in `text` in
/// `locale`: those that start with the word being typed there, each
/// replacing it.
pub fn suggest(text: &str, cursor: usize, locale: &str) -> Vec<Suggestion> {
    let w = Words::of(locale);
    let before = text.get(..cursor).unwrap_or(text);
    let start = before
        .char_indices()
        .rev()
        .take_while(|&(_, c)| in_word(c))
        .last()
        .map_or(before.len(), |(i, _)| i);
    let typed = &before[start..];
    let noon = ClockTime {
        hour: 12,
        minute: 0,
    };
    let day = Day::new(2000, 1, 1);
    let mut parser = Parser::new(
        w,
        day,
        Some(SentenceNow {
            today: day,
            time: noon,
            day_start: noon,
        }),
    );
    for token in tokens(&before[..start], w.german) {
        parser.take(&token);
    }
    // "8:3" is typed for "08:30".
    let bytes = typed.as_bytes();
    let padded = if matches!(bytes, [first] | [first, b':', ..] if first.is_ascii_digit()) {
        format!("0{typed}")
    } else {
        typed.to_string()
    };
    let prefix = padded.to_lowercase();
    let mut seen = BTreeSet::new();
    parser
        .expected(typed)
        .into_iter()
        .filter(|word| {
            let lower = word.to_lowercase();
            lower.starts_with(&prefix) && lower != prefix && seen.insert(word.clone())
        })
        .map(|text| Suggestion {
            range: start..before.len(),
            text,
        })
        .collect()
}

/// Whether `c` continues a word being typed: a letter, a digit, or a
/// dot, colon or slash inside a day, a time or an abbreviation.
fn in_word(c: char) -> bool {
    is_letter(c) || c.is_ascii_digit() || matches!(c, '.' | ':' | '/')
}

/// A Latin letter, with the accented ones of Latin-1 and Latin
/// Extended-A and -B; other scripts are no words of the sentence.
fn is_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || (('\u{c0}'..='\u{24f}').contains(&c) && c != '×' && c != '÷')
}

/// Whitespace and the hyphen of "10-mal" part words.
fn is_space(c: char) -> bool {
    is_blank(c) || c == '-'
}

fn is_blank(c: char) -> bool {
    c <= ' ' || matches!(c, '\u{a0}' | '\u{2009}' | '\u{202f}')
}

/// The end, in chars, of the "am" or "pm" (English) or "Uhr" (German)
/// that follows a time or an hour ending at `i`, blanks before it, with
/// its dot; `None` when none does.
fn suffix_end(chars: &[(usize, char)], i: usize, german: bool) -> Option<usize> {
    let at = |j: usize| chars.get(j).map(|&(_, c)| c);
    let mut j = i;
    while at(j).is_some_and(is_blank) {
        j += 1;
    }
    let from = j;
    while at(j).is_some_and(is_letter) {
        j += 1;
    }
    let word: String = (from..j).filter_map(at).collect();
    let word = word.to_lowercase();
    let fits = if german {
        word == "uhr"
    } else {
        word == "am" || word == "pm"
    };
    if !fits {
        return None;
    }
    if at(j) == Some('.') {
        j += 1;
    }
    Some(j)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Word,
    Number,
    Time,
    Date,
    Comma,
    Other,
}

#[derive(Debug, Clone)]
struct Token {
    kind: Kind,
    text: String,
    range: Range<usize>,
}

/// The tokens of `text`; an hour or a time with its "pm" or "Uhr" is one
/// time.
fn tokens(text: &str, german: bool) -> Vec<Token> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let at = |j: usize| chars.get(j).map(|&(_, c)| c);
    let offset = |j: usize| chars.get(j).map_or(text.len(), |&(i, _)| i);
    let digit = |j: usize| at(j).is_some_and(|c| c.is_ascii_digit());
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(c) = at(i) {
        let from = i;
        let mut kind;
        if is_space(c) {
            i += 1;
            continue;
        } else if c == ',' || c == ';' {
            i += 1;
            kind = Kind::Comma;
        } else if is_letter(c) {
            while at(i).is_some_and(is_letter) {
                i += 1;
            }
            if at(i) == Some('.') {
                i += 1;
            }
            kind = Kind::Word;
        } else if c.is_ascii_digit() {
            while digit(i) {
                i += 1;
            }
            let sep = at(i);
            if sep == Some(':') && digit(i + 1) {
                i += 1;
                while digit(i) {
                    i += 1;
                }
                kind = Kind::Time;
            } else if matches!(sep, Some('/' | '.')) && digit(i + 1) {
                i += 1;
                while digit(i) {
                    i += 1;
                }
                if at(i) == sep && digit(i + 1) {
                    i += 1;
                    while digit(i) {
                        i += 1;
                    }
                }
                if sep == Some('.') && at(i) == Some('.') {
                    i += 1;
                }
                kind = Kind::Date;
            } else {
                if sep == Some('.') {
                    // "am 15." is the 15th.
                    i += 1;
                }
                kind = Kind::Number;
            }
            if (kind == Kind::Time || (kind == Kind::Number && sep != Some('.')))
                && let Some(end) = suffix_end(&chars, i, german)
            {
                i = end;
                kind = Kind::Time;
            }
        } else {
            // One code point that is no part of a sentence.
            i += 1;
            kind = Kind::Other;
        }
        let range = offset(from)..offset(i);
        out.push(Token {
            kind,
            text: text[range.clone()].to_string(),
            range,
        });
    }
    out
}

/// The words of one language. Weekday names are the app's own: the
/// phone's short ones (`Mo`, as on its weekday chips) and its long ones;
/// a word is matched without case and without a trailing dot, so the
/// desktop's `Mo.` reads too.
#[derive(Debug, Clone, Copy)]
struct Words {
    german: bool,
}

const WEEKDAYS_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_DE: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
const WEEKDAYS_LONG_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const WEEKDAYS_LONG_DE: [&str; 7] = [
    "Montag",
    "Dienstag",
    "Mittwoch",
    "Donnerstag",
    "Freitag",
    "Samstag",
    "Sonntag",
];

impl Words {
    fn of(locale: &str) -> Words {
        Words {
            german: locale == "de" || locale.starts_with("de_") || locale.starts_with("de-"),
        }
    }

    fn pick(self, en: &'static str, de: &'static str) -> &'static str {
        if self.german { de } else { en }
    }

    fn once_head(self) -> &'static str {
        self.pick("Once", "Einmal")
    }
    fn weekdays_head(self) -> &'static str {
        self.pick("Weekdays", "Wochentags")
    }
    fn on(self) -> &'static str {
        self.pick("on", "am")
    }
    fn at(self) -> &'static str {
        self.pick("at", "um")
    }
    fn from(self) -> &'static str {
        self.pick("from", "ab")
    }
    fn until(self) -> &'static str {
        self.pick("until", "bis")
    }
    fn count_once(self) -> &'static str {
        self.pick("once", "einmal")
    }
    fn times_word(self) -> &'static str {
        self.pick("times", "mal")
    }
    fn and(self) -> &'static str {
        self.pick("and", "und")
    }

    fn weekdays(self) -> &'static [&'static str; 7] {
        if self.german {
            &WEEKDAYS_DE
        } else {
            &WEEKDAYS_EN
        }
    }

    fn weekdays_long(self) -> &'static [&'static str; 7] {
        if self.german {
            &WEEKDAYS_LONG_DE
        } else {
            &WEEKDAYS_LONG_EN
        }
    }

    /// The ordinals 1 to 4 and the last, in that order.
    fn ordinals(self) -> &'static [&'static str; 5] {
        if self.german {
            &["ersten", "zweiten", "dritten", "vierten", "letzten"]
        } else {
            &["first", "second", "third", "fourth", "last"]
        }
    }

    /// Each unit's word for one.
    fn units(self) -> &'static [&'static str; 4] {
        if self.german {
            &["Tag", "Woche", "Monat", "Jahr"]
        } else {
            &["day", "week", "month", "year"]
        }
    }

    /// Each unit's word for more.
    fn units_many(self) -> &'static [&'static str; 4] {
        if self.german {
            &["Tage", "Wochen", "Monate", "Jahre"]
        } else {
            &["days", "weeks", "months", "years"]
        }
    }

    /// The words that start every N units: "Every", or German's "Jeden",
    /// "Jede", "Jedes" and "Alle".
    fn every_heads(self) -> &'static [&'static str] {
        if self.german {
            &["Jeden", "Jede", "Jedes", "Alle"]
        } else {
            &["Every"]
        }
    }

    fn weekdays_heads(self) -> &'static [&'static str] {
        if self.german {
            &["Wochentags", "Immer"]
        } else {
            &["Weekdays"]
        }
    }

    fn heads(self) -> Vec<&'static str> {
        let mut heads = vec![self.once_head()];
        heads.extend_from_slice(self.every_heads());
        heads.push(self.weekdays_head());
        heads
    }

    /// The relative words a one-time Reminder starts with (#271).
    fn today(self) -> &'static str {
        self.pick("today", "heute")
    }
    fn tomorrow(self) -> &'static str {
        self.pick("tomorrow", "morgen")
    }
    fn relative_heads(self) -> &'static [&'static str] {
        if self.german {
            &["Heute", "Morgen", "In"]
        } else {
            &["Today", "Tonight", "Tomorrow", "In"]
        }
    }

    /// The words after a day for its evening, the first one suggested.
    fn evenings(self) -> &'static [&'static str] {
        if self.german {
            &["Abend"]
        } else {
            &["evening", "night"]
        }
    }

    /// The words for one after "in".
    fn articles(self) -> &'static [&'static str] {
        if self.german {
            &["eine", "einer", "einem", "einen"]
        } else {
            &["a", "an"]
        }
    }

    /// The units after "in N", for one, as suggested.
    fn relative_units(self) -> &'static [&'static str; 4] {
        if self.german {
            &["Minute", "Stunde", "Tag", "Woche"]
        } else {
            &["minute", "hour", "day", "week"]
        }
    }

    /// The units after "in N", for more, as suggested.
    fn relative_units_many(self) -> &'static [&'static str; 4] {
        if self.german {
            &["Minuten", "Stunden", "Tagen", "Wochen"]
        } else {
            &["minutes", "hours", "days", "weeks"]
        }
    }

    /// The unit after "in N" `word` names, 0 to 3 for minutes, hours,
    /// days and weeks, in the singular, the plural or short.
    fn relative_unit(self, word: &str) -> Option<u8> {
        match (self.german, plain(word).as_str()) {
            (false, "minute" | "minutes" | "min" | "mins") => Some(0),
            (false, "hour" | "hours" | "h" | "hr" | "hrs") => Some(1),
            (false, "day" | "days") => Some(2),
            (false, "week" | "weeks") => Some(3),
            (true, "minute" | "minuten" | "min") => Some(0),
            (true, "stunde" | "stunden" | "std" | "h") => Some(1),
            (true, "tag" | "tage" | "tagen") => Some(2),
            (true, "woche" | "wochen") => Some(3),
            _ => None,
        }
    }

    fn every(self, n: i64, unit: RepeatUnit) -> String {
        if !self.german {
            return if n == 1 {
                format!("Every {}", unit_word(self.units(), unit))
            } else {
                format!("Every {n} {}", unit_word(self.units_many(), unit))
            };
        }
        if n != 1 {
            return format!("Alle {n} {}", unit_word(self.units_many(), unit));
        }
        let article = unit_word(&["Jeden", "Jede", "Jeden", "Jedes"], unit);
        format!("{article} {}", unit_word(self.units(), unit))
    }

    fn month_day(self, day: i64) -> String {
        if self.german {
            format!("am {day}.")
        } else {
            format!("on day {day}")
        }
    }

    fn nth(self, rule: WeekdayRule) -> String {
        let [first, second, third, fourth, last] = *self.ordinals();
        let ordinal = match rule.ordinal {
            Some(1) => first,
            Some(2) => second,
            Some(3) => third,
            Some(4) => fourth,
            _ => last,
        };
        let weekday = weekday_word(self.weekdays_long(), rule.weekday);
        if self.german {
            format!("am {ordinal} {weekday}")
        } else {
            format!("on the {ordinal} {weekday}")
        }
    }

    fn count(self, n: i64) -> String {
        if self.german {
            format!("{n}-mal")
        } else {
            format!("{n} times")
        }
    }

    /// `day` in the locale's numbers, its year only when not `today`'s.
    fn date(self, day: Day, today: Day) -> String {
        let year = (day.year != today.year).then_some(day.year);
        if self.german {
            let year = year.map(|y| y.to_string()).unwrap_or_default();
            format!("{}.{}.{year}", day.day, day.month)
        } else {
            let year = year.map(|y| format!("/{y}")).unwrap_or_default();
            format!("{}/{}{year}", day.month, day.day)
        }
    }

    /// The index of `word` among `words`.
    fn find(word: &str, words: &[&str]) -> Option<usize> {
        let wanted = plain(word);
        words.iter().position(|w| plain(w) == wanted)
    }

    /// The unit `word` names, for one or more.
    fn unit(self, word: &str) -> Option<RepeatUnit> {
        let i = Words::find(word, self.units()).or_else(|| Words::find(word, self.units_many()))?;
        [
            RepeatUnit::Days,
            RepeatUnit::Weeks,
            RepeatUnit::Months,
            RepeatUnit::Years,
        ]
        .get(i)
        .copied()
    }

    /// 1 for Monday to 7 for Sunday, for a short or long name.
    fn weekday(self, word: &str) -> Option<u32> {
        let i = Words::find(word, self.weekdays())
            .or_else(|| Words::find(word, self.weekdays_long()))?;
        Some(i as u32 + 1)
    }

    /// 1 to 4, or -1 for the last.
    fn ordinal(self, word: &str) -> Option<i64> {
        let i = Words::find(word, self.ordinals())?;
        Some(if i == 4 { -1 } else { i as i64 + 1 })
    }
}

/// `word` lower case, without a trailing dot.
fn plain(word: &str) -> String {
    let lower = word.to_lowercase();
    match lower.strip_suffix('.') {
        Some(stripped) => stripped.to_string(),
        None => lower,
    }
}

/// The word of `words`, one per unit in the order days, weeks, months,
/// years, for `unit`.
fn unit_word(words: &[&'static str; 4], unit: RepeatUnit) -> &'static str {
    let [days, weeks, months, years] = *words;
    match unit {
        RepeatUnit::Days => days,
        RepeatUnit::Weeks => weeks,
        RepeatUnit::Months => months,
        RepeatUnit::Years => years,
    }
}

/// The word of `words`, Monday to Sunday, for `weekday`, 1 for Monday to
/// 7 for Sunday. A number outside that breaks the contract of the fields
/// that hold it; it reads as the nearest weekday, as the rule writes it.
fn weekday_word(words: &[&'static str; 7], weekday: u32) -> &'static str {
    let [mon, tue, wed, thu, fri, sat, sun] = *words;
    match weekday {
        0 | 1 => mon,
        2 => tue,
        3 => wed,
        4 => thu,
        5 => fri,
        6 => sat,
        _ => sun,
    }
}

/// Where the reader is in the sentence (ADR 0044).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Before the repeat.
    Start,
    /// After "Once"; "on" and a day may follow.
    Once,
    OnceOn,
    /// After "Every": a unit, a number or a weekday.
    Every,
    EveryN,
    /// After every N months: "on" a day or a weekday may follow.
    Months,
    MonthOn,
    MonthOnDay,
    MonthOnThe,
    MonthOrd,
    /// After "today" or "tomorrow", "evening" or a time may follow;
    /// after the other relative words a time (#271).
    RelativeDay,
    Relative,
    /// After "in" and after its number: a number, then a unit.
    InRelative,
    InRelativeN,
    /// Weekdays, more of them may follow.
    Weekdays,
    WeekdayAnd,
    WeekdayComma,
    /// After the repeat: times, the start and the end.
    Body,
    Times0,
    Times,
    TimesAnd,
    TimesComma,
    FromDate,
    UntilDate,
    CountN,
}

impl State {
    /// Whether it waits for more, and falls back when it gets none.
    fn waiting(self) -> bool {
        matches!(
            self,
            State::OnceOn
                | State::Every
                | State::EveryN
                | State::MonthOn
                | State::MonthOnDay
                | State::MonthOnThe
                | State::MonthOrd
                | State::InRelative
                | State::InRelativeN
                | State::WeekdayAnd
                | State::Times0
                | State::TimesAnd
                | State::FromDate
                | State::UntilDate
                | State::CountN
        )
    }

    /// Where a waiting state falls back to.
    fn fallback(self) -> State {
        match self {
            State::Every | State::EveryN | State::InRelative | State::InRelativeN => State::Start,
            State::MonthOn | State::MonthOnDay | State::MonthOnThe | State::MonthOrd => {
                State::Months
            }
            State::WeekdayAnd => State::Weekdays,
            State::OnceOn => State::Once,
            _ => State::Body,
        }
    }
}

/// Reads a sentence token by token. A state that waits for more falls
/// back when the next token does not fit, its tokens marked unknown, and
/// that token is read again from the fallback.
struct Parser {
    w: Words,
    today: Day,
    /// Now, for the relative words; without it they are not understood.
    now: Option<SentenceNow>,
    state: State,
    /// Whether a repeat was read.
    head_done: bool,
    kind: RepeatKind,
    every: i64,
    unit: RepeatUnit,
    weekdays: BTreeSet<u32>,
    month_day: Option<i64>,
    nth: Option<WeekdayRule>,
    count: Option<i64>,
    until: Option<Day>,
    start: Option<Day>,
    times: Vec<ClockTime>,
    /// The time of a relative day when none is typed: 19:00 "tonight",
    /// the day start "tomorrow"; none for all day.
    day_time: Option<ClockTime>,
    /// The number before "times", and the ordinal before its weekday.
    pending_number: i64,
    pending_ordinal: i64,
    /// The tokens of the part being read that is not finished yet.
    pending: Vec<Token>,
    unknown: Vec<Token>,
}

impl Parser {
    fn new(w: Words, today: Day, now: Option<SentenceNow>) -> Parser {
        Parser {
            w,
            today,
            now,
            state: State::Start,
            head_done: false,
            kind: RepeatKind::Once,
            every: 1,
            unit: RepeatUnit::Days,
            weekdays: BTreeSet::new(),
            month_day: None,
            nth: None,
            count: None,
            until: None,
            start: None,
            times: Vec::new(),
            day_time: None,
            pending_number: 0,
            pending_ordinal: 0,
            pending: Vec::new(),
            unknown: Vec::new(),
        }
    }

    fn take(&mut self, token: &Token) {
        if self.step(token) {
            return;
        }
        if self.state.waiting() {
            self.unknown.append(&mut self.pending);
            self.state = self.state.fallback();
            self.take(token);
            return;
        }
        self.unknown.push(token.clone());
    }

    /// Marks what is left unfinished.
    fn finish(&mut self) {
        if self.state.waiting() {
            self.unknown.append(&mut self.pending);
        }
        self.pending.clear();
    }

    /// Waits for more in `next`, `token` being part of it.
    fn wait(&mut self, next: State, token: &Token) -> bool {
        self.pending.push(token.clone());
        self.state = next;
        true
    }

    /// Done with the part being read; `to` next.
    fn done(&mut self, to: State) -> bool {
        self.pending.clear();
        self.state = to;
        true
    }

    fn number(t: &Token) -> Option<i64> {
        if t.kind != Kind::Number {
            return None;
        }
        let digits = t.text.strip_suffix('.').unwrap_or(&t.text);
        if digits.len() > 5 {
            return None;
        }
        digits.parse().ok()
    }

    /// `08:00`, `8:00`, and `7 pm`, `7:30pm` or `19 Uhr`.
    fn time(t: &Token) -> Option<ClockTime> {
        if t.kind != Kind::Time {
            return None;
        }
        let text = t.text.as_str();
        let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
        let i = digits(text);
        let h = &text[..i];
        let (m, rest) = match text[i..].strip_prefix(':') {
            Some(after) => after.split_at(digits(after)),
            None => ("00", &text[i..]),
        };
        let suffix: String = rest
            .chars()
            .filter(char::is_ascii_alphabetic)
            .collect::<String>()
            .to_ascii_lowercase();
        if h.len() > 2 || m.len() != 2 {
            return None;
        }
        let mut hour: u32 = h.parse().ok()?;
        let minute: u32 = m.parse().ok()?;
        if minute > 59 {
            return None;
        }
        if suffix == "am" || suffix == "pm" {
            if !(1..=12).contains(&hour) {
                return None;
            }
            hour = hour % 12 + if suffix == "pm" { 12 } else { 0 };
        }
        ClockTime::new(hour, minute)
    }

    fn date(&self, t: &Token) -> Option<Day> {
        if t.kind != Kind::Date {
            return None;
        }
        let dots = t.text.contains('.');
        let body = if dots {
            t.text.strip_suffix('.').unwrap_or(&t.text)
        } else {
            &t.text
        };
        let parts: Vec<&str> = body.split(if dots { '.' } else { '/' }).collect();
        let [first, second, rest @ ..] = parts.as_slice() else {
            return None;
        };
        if first.len() > 2 || second.len() > 2 {
            return None;
        }
        let a: i64 = first.parse().ok()?;
        let b: i64 = second.parse().ok()?;
        let year = match rest.first() {
            None => self.today.year,
            Some(y) if y.len() == 2 => 2000 + y.parse::<i64>().ok()?,
            Some(y) if y.len() == 4 => y.parse().ok()?,
            Some(_) => return None,
        };
        let (month, day) = if dots { (b, a) } else { (a, b) };
        let date = Day::new(year, month, day);
        (date.year == year && i64::from(date.month) == month && i64::from(date.day) == day)
            .then_some(date)
    }

    fn is_word(&self, t: &Token, expected: &str) -> bool {
        t.kind == Kind::Word && Words::find(&t.text, &[expected]).is_some()
    }

    fn any_word(&self, t: &Token, words: &[&str]) -> bool {
        t.kind == Kind::Word && Words::find(&t.text, words).is_some()
    }

    fn weekday(&self, t: &Token) -> Option<u32> {
        (t.kind == Kind::Word)
            .then(|| self.w.weekday(&t.text))
            .flatten()
    }

    fn ordinal(&self, t: &Token) -> Option<i64> {
        (t.kind == Kind::Word)
            .then(|| self.w.ordinal(&t.text))
            .flatten()
    }

    fn unit_of(&self, t: &Token) -> Option<RepeatUnit> {
        (t.kind == Kind::Word)
            .then(|| self.w.unit(&t.text))
            .flatten()
    }

    /// Takes `t` in the current state; false when it does not fit there.
    fn step(&mut self, t: &Token) -> bool {
        let w = self.w;
        match self.state {
            State::Start => {
                if self.is_word(t, w.once_head()) {
                    self.head_done = true;
                    self.kind = RepeatKind::Once;
                    return self.done(State::Once);
                }
                if self.any_word(t, w.every_heads()) {
                    return self.wait(State::Every, t);
                }
                if self.any_word(t, w.weekdays_heads()) {
                    self.head_done = true;
                    self.kind = RepeatKind::Weekdays;
                    return self.done(State::Weekdays);
                }
                self.relative_head(t)
            }
            State::RelativeDay => {
                if self.any_word(t, w.evenings()) {
                    self.day_time = Some(TONIGHT);
                    return self.done(State::Relative);
                }
                self.relative(t)
            }
            State::Relative => self.relative(t),
            State::InRelative => {
                let n = Parser::number(t).or_else(|| self.any_word(t, w.articles()).then_some(1));
                match n.filter(|&n| n >= 1) {
                    Some(n) => {
                        self.pending_number = n;
                        self.wait(State::InRelativeN, t)
                    }
                    None => false,
                }
            }
            State::InRelativeN => {
                let unit = (t.kind == Kind::Word)
                    .then(|| w.relative_unit(&t.text))
                    .flatten();
                match unit {
                    Some(unit) => self.in_units(self.pending_number, unit),
                    None => false,
                }
            }
            State::Once => {
                if self.is_word(t, w.on()) {
                    return self.wait(State::OnceOn, t);
                }
                if self.relative_day_word(t) {
                    return true;
                }
                if let Some(day) = self.date(t) {
                    self.start = Some(day);
                    return self.done(State::Body);
                }
                self.body(t)
            }
            State::OnceOn | State::FromDate => {
                if self.state == State::OnceOn && self.relative_day_word(t) {
                    return true;
                }
                if let Some(day) = self.date(t) {
                    self.start = Some(day);
                    return self.done(State::Body);
                }
                false
            }
            State::Every => {
                if let Some(unit) = self.unit_of(t) {
                    return self.unit(1, unit);
                }
                if let Some(d) = self.weekday(t) {
                    self.head_done = true;
                    self.kind = RepeatKind::Weekdays;
                    self.weekdays.insert(d);
                    return self.done(State::Weekdays);
                }
                if let Some(n) = Parser::number(t).filter(|&n| n >= 1) {
                    self.every = n;
                    return self.wait(State::EveryN, t);
                }
                false
            }
            State::EveryN => match self.unit_of(t) {
                Some(unit) => self.unit(self.every, unit),
                None => false,
            },
            State::Months => {
                if self.is_word(t, w.on()) {
                    return self.wait(State::MonthOn, t);
                }
                self.body(t)
            }
            State::MonthOn => {
                if !w.german && self.is_word(t, "day") {
                    return self.wait(State::MonthOnDay, t);
                }
                if !w.german && self.is_word(t, "the") {
                    return self.wait(State::MonthOnThe, t);
                }
                if let Some(o) = self.ordinal(t) {
                    self.pending_ordinal = o;
                    return self.wait(State::MonthOrd, t);
                }
                w.german && self.month_day(t)
            }
            State::MonthOnDay => self.month_day(t),
            State::MonthOnThe => {
                if let Some(o) = self.ordinal(t) {
                    self.pending_ordinal = o;
                    return self.wait(State::MonthOrd, t);
                }
                false
            }
            State::MonthOrd => {
                if let Some(d) = self.weekday(t) {
                    self.kind = RepeatKind::MonthWeekday;
                    self.nth = Some(WeekdayRule {
                        weekday: d,
                        ordinal: Some(self.pending_ordinal),
                    });
                    return self.done(State::Body);
                }
                false
            }
            State::Weekdays => {
                if let Some(d) = self.weekday(t) {
                    self.weekdays.insert(d);
                    return self.done(State::Weekdays);
                }
                if self.is_word(t, w.and()) {
                    return self.wait(State::WeekdayAnd, t);
                }
                if t.kind == Kind::Comma {
                    return self.done(State::WeekdayComma);
                }
                self.body(t)
            }
            State::WeekdayAnd | State::WeekdayComma => {
                if let Some(d) = self.weekday(t) {
                    self.weekdays.insert(d);
                    return self.done(State::Weekdays);
                }
                if self.state == State::WeekdayAnd {
                    return false;
                }
                self.body(t)
            }
            State::Body => self.body(t),
            State::Times0 | State::TimesAnd => match Parser::time(t) {
                Some(time) => {
                    self.add_time(t, time);
                    self.done(State::Times)
                }
                None => false,
            },
            State::Times => {
                if let Some(time) = Parser::time(t) {
                    self.add_time(t, time);
                    return self.done(State::Times);
                }
                if t.kind == Kind::Comma {
                    return self.done(State::TimesComma);
                }
                if self.is_word(t, w.and()) {
                    return self.wait(State::TimesAnd, t);
                }
                self.body(t)
            }
            State::TimesComma => {
                if let Some(time) = Parser::time(t) {
                    self.add_time(t, time);
                    return self.done(State::Times);
                }
                self.body(t)
            }
            State::UntilDate => {
                if let Some(day) = self.date(t) {
                    if self.kind == RepeatKind::Once {
                        return self.refuse(t);
                    }
                    self.until = Some(day);
                    self.count = None;
                    return self.done(State::Body);
                }
                false
            }
            State::CountN => {
                if self.is_word(t, w.times_word()) || (!w.german && self.is_word(t, "time")) {
                    if self.kind == RepeatKind::Once {
                        return self.refuse(t);
                    }
                    self.count = Some(self.pending_number);
                    self.until = None;
                    return self.done(State::Body);
                }
                false
            }
        }
    }

    /// A relative word a Reminder starts with; false for none, and while
    /// there is no now to read it at.
    fn relative_head(&mut self, t: &Token) -> bool {
        if self.relative_day_word(t) {
            return true;
        }
        if self.now.is_some() && self.is_word(t, "in") {
            return self.wait(State::InRelative, t);
        }
        false
    }

    /// "today", "tonight" or "tomorrow", at the start or as the day of
    /// Once; false for none, and while there is no now.
    fn relative_day_word(&mut self, t: &Token) -> bool {
        let Some(now) = self.now else {
            return false;
        };
        let w = self.w;
        if self.is_word(t, w.today()) {
            return self.relative_day(now.today, None, true);
        }
        if !w.german && self.is_word(t, "tonight") {
            return self.relative_day(now.today, Some(TONIGHT), false);
        }
        if self.is_word(t, w.tomorrow()) {
            return self.relative_day(now.today.add_days(1), Some(now.day_start), true);
        }
        false
    }

    /// Once on `day`, at `time` unless one is typed; "evening" may follow
    /// when `evening`.
    fn relative_day(&mut self, day: Day, time: Option<ClockTime>, evening: bool) -> bool {
        self.head_done = true;
        self.kind = RepeatKind::Once;
        self.start = Some(day);
        self.day_time = time;
        self.done(if evening {
            State::RelativeDay
        } else {
            State::Relative
        })
    }

    /// Once in `n` of the `unit`th of minutes, hours, days and weeks: at
    /// the moment for minutes and hours, at the day start for days and
    /// weeks unless a time is typed.
    fn in_units(&mut self, n: i64, unit: u8) -> bool {
        let Some(now) = self.now else {
            return false;
        };
        if unit < 2 {
            let (day, time) = later(&now, if unit == 0 { n } else { n * 60 });
            self.times.push(time);
            return self.relative_day(day, None, false);
        }
        let days = if unit == 2 { n } else { n * 7 };
        self.relative_day(now.today.add_days(days), Some(now.day_start), false)
    }

    /// After a relative word: a time without "at", or what follows a
    /// repeat.
    fn relative(&mut self, t: &Token) -> bool {
        if let Some(time) = Parser::time(t) {
            self.add_time(t, time);
            return self.done(State::Times);
        }
        self.body(t)
    }

    /// The repeat is every `n` `unit`s.
    fn unit(&mut self, n: i64, unit: RepeatUnit) -> bool {
        self.head_done = true;
        self.kind = RepeatKind::Every;
        self.every = n;
        self.unit = unit;
        self.done(if unit == RepeatUnit::Months {
            State::Months
        } else {
            State::Body
        })
    }

    fn month_day(&mut self, t: &Token) -> bool {
        match Parser::number(t).filter(|d| (1..=31).contains(d)) {
            Some(d) => {
                self.kind = RepeatKind::MonthDay;
                self.month_day = Some(d);
                self.done(State::Body)
            }
            None => false,
        }
    }

    /// A part once cannot have: marked with its tokens, read past.
    fn refuse(&mut self, t: &Token) -> bool {
        self.unknown.append(&mut self.pending);
        self.unknown.push(t.clone());
        self.done(State::Body)
    }

    fn add_time(&mut self, t: &Token, time: ClockTime) {
        if self.kind == RepeatKind::Once && !self.times.is_empty() && !self.times.contains(&time) {
            self.unknown.push(t.clone());
            return;
        }
        self.times.push(time);
    }

    /// What may follow the repeat.
    fn body(&mut self, t: &Token) -> bool {
        if !self.head_done {
            return false;
        }
        let w = self.w;
        if t.kind == Kind::Comma {
            return self.done(State::Body);
        }
        if self.is_word(t, w.at()) {
            return self.wait(State::Times0, t);
        }
        if self.is_word(t, w.from()) {
            return self.wait(State::FromDate, t);
        }
        if self.is_word(t, w.until()) {
            return self.wait(State::UntilDate, t);
        }
        if self.is_word(t, w.count_once()) {
            if self.kind == RepeatKind::Once {
                return self.refuse(t);
            }
            self.count = Some(1);
            self.until = None;
            return self.done(State::Body);
        }
        if let Some(n) = Parser::number(t).filter(|&n| n >= 1) {
            self.pending_number = n;
            return self.wait(State::CountN, t);
        }
        false
    }

    /// The words that may come next, for `typed` being typed.
    fn expected(&self, typed: &str) -> Vec<String> {
        let w = self.w;
        // Once has no end; a second "at" adds to the times already read.
        let mut body: Vec<&str> = Vec::new();
        if self.times.is_empty() {
            body.push(w.at());
        }
        if self.kind != RepeatKind::Once {
            body.extend([w.from(), w.until()]);
        }
        let picked: Vec<&str> = (1..=7u32)
            .filter(|d| !self.weekdays.contains(d))
            .map(|d| weekday_word(w.weekdays(), d))
            .collect();
        let words: Vec<String> = match self.state {
            State::Start => return [strings(&w.heads()), strings(w.relative_heads())].concat(),
            State::RelativeDay => [
                strings(w.evenings().get(..1).unwrap_or(&[])),
                strings(&body),
            ]
            .concat(),
            State::Relative => strings(&body),
            State::InRelative => Vec::new(),
            State::InRelativeN if self.pending_number == 1 => strings(w.relative_units()),
            State::InRelativeN => strings(w.relative_units_many()),
            State::Once => return strings(&[w.on(), w.at()]),
            State::OnceOn | State::FromDate | State::UntilDate | State::MonthOnDay => Vec::new(),
            State::Every => [w.units().as_slice(), w.weekdays().as_slice()]
                .concat()
                .iter()
                .map(ToString::to_string)
                .collect(),
            State::EveryN => strings(w.units_many()),
            State::Months => strings(&[&[w.on()], &body[..]].concat()),
            State::MonthOn if w.german => strings(w.ordinals()),
            State::MonthOn => strings(&["day", "the"]),
            State::MonthOnThe => strings(w.ordinals()),
            State::MonthOrd => strings(w.weekdays_long()),
            State::Weekdays | State::WeekdayComma => strings(&[&picked[..], &body[..]].concat()),
            State::WeekdayAnd => strings(&picked),
            State::Body | State::Times => strings(&body),
            State::Times0 | State::TimesAnd => times_for(typed),
            State::TimesComma => [times_for(typed), strings(&body)].concat(),
            // Once has no end.
            State::CountN if self.kind == RepeatKind::Once => Vec::new(),
            State::CountN => strings(&[w.times_word()]),
        };
        words
    }

    /// The schedule read; `None` when no repeat was.
    fn schedule(&self) -> Option<SimpleSchedule> {
        if !self.head_done {
            return None;
        }
        let repeat = match self.kind {
            RepeatKind::Once => SimpleRepeat::once(),
            RepeatKind::Every => SimpleRepeat::every(self.every, self.unit),
            RepeatKind::Weekdays => SimpleRepeat::weekdays(self.weekdays.clone()),
            RepeatKind::MonthDay => {
                SimpleRepeat::month_day(self.month_day.unwrap_or(1), self.every)
            }
            RepeatKind::MonthWeekday => SimpleRepeat::month_weekday(
                self.nth.unwrap_or(WeekdayRule {
                    weekday: 1,
                    ordinal: Some(1),
                }),
                self.every,
            ),
        };
        let repeat = if self.kind == RepeatKind::Once {
            repeat
        } else {
            repeat.ending(self.count, self.until)
        };
        let times = match self.day_time {
            Some(time) if self.times.is_empty() => vec![time],
            _ => self.times.clone(),
        };
        Some(SimpleSchedule::new(
            repeat,
            self.start.unwrap_or(self.today),
            &times,
        ))
    }
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(ToString::to_string).collect()
}

/// Times for `typed`: its hour on the hour and the half, or three times
/// of the day.
fn times_for(typed: &str) -> Vec<String> {
    let (hour, minute) = match typed.split_once(':') {
        Some((h, m)) => (h, Some(m)),
        None => (typed, None),
    };
    let shaped = !hour.is_empty()
        && hour.len() <= 2
        && hour.bytes().all(|b| b.is_ascii_digit())
        && minute.is_none_or(|m| m.len() <= 2 && m.bytes().all(|b| b.is_ascii_digit()));
    if !shaped {
        return strings(&["08:00", "12:00", "18:00"]);
    }
    let Ok(hour) = hour.parse::<u32>() else {
        return Vec::new();
    };
    if hour > 23 {
        return Vec::new();
    }
    vec![format!("{hour:02}:00"), format!("{hour:02}:30")]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u32, minute: u32) -> ClockTime {
        ClockTime::new(hour, minute).unwrap_or(ClockTime { hour: 0, minute: 0 })
    }

    const TODAY: Day = Day {
        year: 2026,
        month: 11,
        day: 3,
    };

    fn unknown(text: &str, locale: &str) -> Vec<String> {
        read(text, locale, TODAY)
            .unknown
            .into_iter()
            .map(|r| text[r].to_string())
            .collect()
    }

    fn suggested(text: &str, locale: &str) -> Vec<String> {
        suggest(text, text.len(), locale)
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    #[test]
    fn a_weekday_out_of_range_reads_as_the_nearest() {
        let schedule =
            SimpleSchedule::new(SimpleRepeat::weekdays([0, 9].into()), TODAY, &[at(8, 0)]);
        assert_eq!(
            sentence(&schedule, "en", TODAY),
            "Weekdays Mon, Sun at 08:00"
        );
    }

    #[test]
    fn the_issues_example_reads_both_ways() {
        let schedule = SimpleSchedule::new(
            SimpleRepeat::weekdays([1, 4].into()).ending(None, Some(Day::new(2026, 12, 31))),
            TODAY,
            &[at(15, 0), at(8, 0)],
        );
        let en = sentence(&schedule, "en", TODAY);
        let de = sentence(&schedule, "de_DE", TODAY);
        assert_eq!(en, "Weekdays Mon, Thu at 08:00, 15:00, until 12/31");
        assert_eq!(de, "Wochentags Mo, Do um 08:00, 15:00, bis 31.12.");
        for (text, locale) in [(en, "en"), (de, "de")] {
            let back = read(&text, locale, TODAY);
            assert_eq!(back.schedule.as_ref(), Some(&schedule), "{text}");
            assert!(back.unknown.is_empty(), "{text}");
        }
    }

    #[test]
    fn the_start_and_the_year_show_only_when_not_today() {
        let later = SimpleSchedule::new(
            SimpleRepeat::every(2, RepeatUnit::Weeks),
            Day::new(2027, 1, 5),
            &[],
        );
        assert_eq!(
            sentence(&later, "en", TODAY),
            "Every 2 weeks, from 1/5/2027"
        );
        assert_eq!(sentence(&later, "de", TODAY), "Alle 2 Wochen, ab 5.1.2027");
        let now = SimpleSchedule::new(SimpleRepeat::every(1, RepeatUnit::Days), TODAY, &[]);
        assert_eq!(sentence(&now, "en", TODAY), "Every day");
        assert_eq!(read("Every day", "en", TODAY).schedule, Some(now));
    }

    #[test]
    fn unknown_parts_are_marked_and_change_nothing() {
        assert_eq!(unknown("Every day at 08:00 please", "en"), ["please"]);
        assert_eq!(unknown("Every 2 wekks", "en"), ["Every", "2", "wekks"]);
        assert_eq!(read("Every 2 wekks", "en", TODAY).schedule, None);
        assert_eq!(
            unknown("Once at 08:00, 09:00, until 12/1", "en"),
            ["09:00", "until", "12/1"]
        );
        assert_eq!(unknown("Jeden Tag ab 31.2.", "de"), ["ab", "31.2."]);
        assert_eq!(unknown("Jeden Tag 🧠", "de"), ["🧠"]);
    }

    #[test]
    fn suggestions_are_the_next_words() {
        assert_eq!(suggested("Every 2 w", "en"), ["weeks"]);
        assert_eq!(suggested("Weekdays Tue, T", "en"), ["Thu"]);
        assert_eq!(suggested("Every day at 8", "en"), ["08:00", "08:30"]);
        assert_eq!(suggested("Einmal ", "de"), ["am", "um"]);
        let mid = suggest("Every 2 we at 08:00", 10, "en");
        assert_eq!(
            mid,
            [Suggestion {
                range: 8..10,
                text: "weeks".into()
            }]
        );
    }

    /// A Tuesday afternoon; the day starts at 07:30.
    const NOW: SentenceNow = SentenceNow {
        today: TODAY,
        time: ClockTime {
            hour: 14,
            minute: 20,
        },
        day_start: ClockTime {
            hour: 7,
            minute: 30,
        },
    };

    fn read_now(text: &str, locale: &str, now: &SentenceNow) -> Option<SimpleSchedule> {
        read_at(text, locale, now).schedule
    }

    fn once(day: Day, time: Option<ClockTime>) -> Option<SimpleSchedule> {
        let times: Vec<ClockTime> = time.into_iter().collect();
        Some(SimpleSchedule::new(SimpleRepeat::once(), day, &times))
    }

    fn unknown_now(text: &str, locale: &str) -> Vec<String> {
        read_at(text, locale, &NOW)
            .unknown
            .into_iter()
            .map(|r| text[r].to_string())
            .collect()
    }

    #[test]
    fn a_relative_day_is_at_the_day_start_at_19_or_at_the_time_typed() {
        let tomorrow = TODAY.add_days(1);
        let cases = [
            ("tomorrow", "en", once(tomorrow, Some(at(7, 30)))),
            ("Tomorrow 7 pm", "en", once(tomorrow, Some(at(19, 0)))),
            ("tomorrow at 7:30pm", "en", once(tomorrow, Some(at(19, 30)))),
            ("tomorrow evening", "en", once(tomorrow, Some(at(19, 0)))),
            ("tonight", "en", once(TODAY, Some(at(19, 0)))),
            ("Tonight 8 PM", "en", once(TODAY, Some(at(20, 0)))),
            ("today", "en", once(TODAY, None)),
            ("today 12 am", "en", once(TODAY, Some(at(0, 0)))),
            ("today 12 pm", "en", once(TODAY, Some(at(12, 0)))),
            ("morgen 19 Uhr", "de", once(tomorrow, Some(at(19, 0)))),
            ("heute Abend", "de", once(TODAY, Some(at(19, 0)))),
            ("heute", "de", once(TODAY, None)),
            (
                "Once tomorrow at 15:00",
                "en",
                once(tomorrow, Some(at(15, 0))),
            ),
            ("Once on tomorrow", "en", once(tomorrow, Some(at(7, 30)))),
            (
                "Einmal am morgen um 15:00",
                "de",
                once(tomorrow, Some(at(15, 0))),
            ),
            ("in 2 hours", "en", once(TODAY, Some(at(16, 20)))),
            ("in an hour", "en", once(TODAY, Some(at(15, 20)))),
            ("in einer Minute", "de", once(TODAY, Some(at(14, 21)))),
            ("in 3 days", "en", once(TODAY.add_days(3), Some(at(7, 30)))),
            (
                "in 2 weeks 9 am",
                "en",
                once(TODAY.add_days(14), Some(at(9, 0))),
            ),
            ("in 3 Tagen", "de", once(TODAY.add_days(3), Some(at(7, 30)))),
        ];
        for (text, locale, schedule) in cases {
            assert_eq!(read_now(text, locale, &NOW), schedule, "{text}");
        }
        let late = SentenceNow {
            today: Day::new(2026, 12, 31),
            time: at(23, 30),
            ..NOW
        };
        assert_eq!(
            read_now("in 90 min", "en", &late),
            once(Day::new(2027, 1, 1), Some(at(1, 0)))
        );
    }

    #[test]
    fn relative_words_are_read_only_at_a_now() {
        assert_eq!(read("tomorrow", "en", TODAY).schedule, None);
        assert_eq!(unknown("in 2 hours", "en"), ["in", "2", "hours"]);
        assert_eq!(
            read("Every day at 7 pm", "en", TODAY).schedule,
            Some(SimpleSchedule::new(
                SimpleRepeat::every(1, RepeatUnit::Days),
                TODAY,
                &[at(19, 0)]
            ))
        );
    }

    #[test]
    fn relative_parts_that_do_not_fit_are_marked() {
        assert_eq!(unknown_now("today 13 pm", "en"), ["13 pm"]);
        assert_eq!(unknown_now("heute 25 Uhr", "de"), ["25 Uhr"]);
        assert_eq!(unknown_now("in 0 minutes", "en"), ["in", "0", "minutes"]);
        assert_eq!(unknown_now("in 2 hours at 08:00", "en"), ["08:00"]);
        assert!(unknown_now("Einmal am 5.11.", "de").is_empty());
    }

    #[test]
    fn relative_words_are_suggested() {
        assert_eq!(suggested("T", "en"), ["Today", "Tonight", "Tomorrow"]);
        assert_eq!(suggested("Tomorrow ", "en"), ["evening", "at"]);
        assert_eq!(
            suggested("In 2 ", "en"),
            ["minutes", "hours", "days", "weeks"]
        );
        assert_eq!(suggested("in a ", "en"), ["minute", "hour", "day", "week"]);
        assert_eq!(suggested("Heute ", "de"), ["Abend", "um"]);
        assert_eq!(suggested("In 3 T", "de"), ["Tagen"]);
        assert!(suggested("Tomorrow 7 ", "en").is_empty());
    }

    #[test]
    fn the_chips_are_their_words_read_at_now() {
        let chip = |chip, day, time| QuickReminder { chip, day, time };
        assert_eq!(
            quick_reminders(&NOW),
            [
                chip(QuickChip::InAnHour, TODAY, at(15, 20)),
                chip(QuickChip::Tonight, TODAY, at(19, 0)),
                chip(QuickChip::Tomorrow, TODAY.add_days(1), at(7, 30)),
                chip(QuickChip::InAWeek, TODAY.add_days(7), at(7, 30)),
            ]
        );
        let word = |chip: QuickChip, german: bool| match (chip, german) {
            (QuickChip::InAnHour, false) => "in 1 hour",
            (QuickChip::Tonight, false) => "tonight",
            (QuickChip::Tomorrow, false) => "tomorrow",
            (QuickChip::InAWeek, false) => "in 1 week",
            (QuickChip::InAnHour, true) => "in 1 Stunde",
            (QuickChip::Tonight, true) => "heute Abend",
            (QuickChip::Tomorrow, true) => "morgen",
            (QuickChip::InAWeek, true) => "in 1 Woche",
        };
        let moments = [
            NOW,
            SentenceNow {
                today: Day::new(2026, 12, 31),
                time: at(23, 30),
                day_start: at(9, 0),
            },
            SentenceNow {
                time: at(0, 0),
                day_start: at(6, 15),
                ..NOW
            },
        ];
        for now in moments {
            for quick in quick_reminders(&now) {
                for (locale, german) in [("en", false), ("de", true)] {
                    assert_eq!(
                        read_now(word(quick.chip, german), locale, &now),
                        Some(quick.schedule()),
                        "{now:?} {locale}"
                    );
                }
            }
        }
        // "Tonight" while 19:00 is still to come.
        let evening = SentenceNow {
            time: at(19, 0),
            ..NOW
        };
        let chips: Vec<QuickChip> = quick_reminders(&evening).iter().map(|q| q.chip).collect();
        assert_eq!(
            chips,
            [QuickChip::InAnHour, QuickChip::Tomorrow, QuickChip::InAWeek]
        );
    }
}
