//! A schedule of the simple editor as a sentence (ADR 0044), as the Dart
//! core's `schedule_sentence.dart` has it: written in the app's language,
//! read back as typed, with the next words to offer while typing. The
//! sentence is never stored; the schedule's fields are.
//!
//! Offsets here are byte offsets into the text (fixture
//! `schedule_sentences.json` counts code points).

use std::collections::BTreeSet;
use std::ops::Range;

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
/// `today`'s. Parts not understood are skipped.
pub fn read(text: &str, locale: &str, today: Day) -> SentenceRead {
    let mut parser = Parser::new(Words::of(locale), today);
    for token in tokens(text) {
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
    let mut parser = Parser::new(w, Day::new(2000, 1, 1));
    for token in tokens(&before[..start]) {
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
    c <= ' ' || matches!(c, '\u{a0}' | '\u{2009}' | '\u{202f}' | '-')
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

/// The tokens of `text`.
fn tokens(text: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let at = |j: usize| chars.get(j).map(|&(_, c)| c);
    let offset = |j: usize| chars.get(j).map_or(text.len(), |&(i, _)| i);
    let digit = |j: usize| at(j).is_some_and(|c| c.is_ascii_digit());
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(c) = at(i) {
        let from = i;
        let kind;
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
            State::Every | State::EveryN => State::Start,
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
    /// The number before "times", and the ordinal before its weekday.
    pending_number: i64,
    pending_ordinal: i64,
    /// The tokens of the part being read that is not finished yet.
    pending: Vec<Token>,
    unknown: Vec<Token>,
}

impl Parser {
    fn new(w: Words, today: Day) -> Parser {
        Parser {
            w,
            today,
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

    fn time(t: &Token) -> Option<ClockTime> {
        if t.kind != Kind::Time {
            return None;
        }
        let (h, m) = t.text.split_once(':')?;
        if h.len() > 2 || m.len() != 2 {
            return None;
        }
        ClockTime::new(h.parse().ok()?, m.parse().ok()?)
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
                false
            }
            State::Once => {
                if self.is_word(t, w.on()) {
                    return self.wait(State::OnceOn, t);
                }
                if let Some(day) = self.date(t) {
                    self.start = Some(day);
                    return self.done(State::Body);
                }
                self.body(t)
            }
            State::OnceOn | State::FromDate => {
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
            State::Start => return strings(&w.heads()),
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
        Some(SimpleSchedule::new(
            repeat,
            self.start.unwrap_or(self.today),
            &self.times,
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
}
