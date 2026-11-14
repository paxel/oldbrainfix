//! Shopping Lists (ADR 0030) as the Dart core's `shopping_lists.dart`
//! reads and writes them: a Note whose Lines come back a while after they
//! are checked. The kind lies on the Note as a Checklist's does, its
//! Lines are a Checklist's Lines with no indent, and each Line holds its
//! reappear time. What is struck and what came back follows from the
//! moment each Line was checked and the wall clock, so nothing is written
//! when a Line comes back. Every write appends the same entries, in the
//! same order and read from the clock as often, as the Dart core's.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use crate::checklists::{
    Checklists, DeletedLine, NewLine, deleted_lines_of, keys as checklist_keys, line_text, order,
    parse_line_in, parse_list_kind, split_lines, write_line,
};
use crate::entry::{Entry, dart_iso, parse_time};
use crate::notes::{Note, Notes, keys as note_keys};
use crate::schedule::{Day, WallClock};
use crate::settings::DeviceSettings;
use crate::store::{Fields, Store, utf16_cmp};
use crate::tallies::Tallies;
use crate::text_format::{self, StoredText, VERSION};
use crate::transcript_prompts::{dart_trim, lower};
use crate::{Error, Result};

/// Field keys of a Shopping List; the rest are a Checklist's.
pub mod keys {
    /// The list of a Shopping List in the Note's `kind`.
    pub const SHOPPING: &str = "shopping";

    /// The value of the Note's `kind` for the making `making` of a
    /// Shopping List.
    pub fn kind_of(making: &str) -> String {
        format!("{SHOPPING}:{making}")
    }

    /// On a Line: how long it stays struck once checked, a
    /// [`super::Reappear`]'s code.
    pub const REAPPEAR: &str = "reappear";

    /// On the user's own `agenda:<note id>` entity
    /// ([`crate::checklists::keys::AGENDA_PREFIX`]), beside a Checklist's
    /// field: how the Shopping List shows on their Agenda (#186), a
    /// [`super::ShoppingOnAgenda`]'s code.
    pub const AGENDA: &str = "shopping";
}

/// The unit of a [`Reappear`] time (#195), the letter its code ends in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReappearUnit {
    Day,
    Week,
    Month,
    Year,
}

impl ReappearUnit {
    pub const ALL: [ReappearUnit; 4] = [
        ReappearUnit::Day,
        ReappearUnit::Week,
        ReappearUnit::Month,
        ReappearUnit::Year,
    ];

    /// How its code ends.
    pub fn letter(self) -> char {
        match self {
            ReappearUnit::Day => 'd',
            ReappearUnit::Week => 'w',
            ReappearUnit::Month => 'm',
            ReappearUnit::Year => 'y',
        }
    }
}

/// How long a checked Line of a Shopping List stays struck through before
/// it comes back (CONTEXT.md): none, so it stays struck until a time is
/// set; Once, gone once checked; or a number of days, weeks, months or
/// years (#195), from 1 to [`Reappear::MAX_COUNT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reappear {
    /// None: a checked Line stays struck until a time is set. What a new
    /// Line gets.
    Unset,
    /// Gone once checked.
    Once,
    /// This many of the unit.
    After(u32, ReappearUnit),
}

impl Reappear {
    /// What a Line without a reappear time, or with one not known here,
    /// reads as: one week, as every version before #195 read it.
    pub const FALLBACK: Reappear = Reappear::After(1, ReappearUnit::Week);

    /// The largest number a time holds, so its code stays short.
    pub const MAX_COUNT: u32 = 99;

    /// `count` `unit`s, or `None` for a count not from 1 to
    /// [`Reappear::MAX_COUNT`].
    pub fn after(count: u32, unit: ReappearUnit) -> Option<Reappear> {
        (1..=Reappear::MAX_COUNT)
            .contains(&count)
            .then_some(Reappear::After(count, unit))
    }

    /// How it is stored on the Line: `none`, `once`, or the number and the
    /// unit's letter, such as `4w` or `3m`.
    pub fn code(self) -> String {
        match self {
            Reappear::Unset => "none".to_string(),
            Reappear::Once => "once".to_string(),
            Reappear::After(count, unit) => format!("{count}{}", unit.letter()),
        }
    }

    /// The reappear time `code` names; [`Reappear::FALLBACK`] for none, or
    /// a code not known here: anything but `none`, `once` and a number
    /// from 1 to 99 without a leading zero followed by `d`, `w`, `m` or
    /// `y`.
    pub fn read(code: Option<&str>) -> Reappear {
        match code {
            Some("none") => return Reappear::Unset,
            Some("once") => return Reappear::Once,
            _ => {}
        }
        let Some(code) = code.filter(|c| c.is_ascii() && (2..=3).contains(&c.len())) else {
            return Reappear::FALLBACK;
        };
        let (digits, letter) = code.split_at(code.len() - 1);
        let unit = ReappearUnit::ALL
            .into_iter()
            .find(|u| letter.starts_with(u.letter()));
        let count = digits
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| digits.parse::<u32>().ok())
            .flatten()
            .filter(|_| !digits.starts_with('0'));
        match (count, unit) {
            (Some(count), Some(unit)) => Reappear::After(count, unit),
            _ => Reappear::FALLBACK,
        }
    }

    /// The day a Line checked on `checked` comes back on: as many days or
    /// weeks on, or the same day of the month as many months or years on,
    /// the month's last day where it has no such day. `None` for none and
    /// Once, which never come back by themselves.
    pub fn back_from(self, checked: Day) -> Option<Day> {
        let Reappear::After(count, unit) = self else {
            return None;
        };
        let count = i64::from(count);
        Some(match unit {
            ReappearUnit::Day => checked.add_days(count),
            ReappearUnit::Week => checked.add_days(7 * count),
            ReappearUnit::Month => months_on(checked, count),
            ReappearUnit::Year => months_on(checked, 12 * count),
        })
    }
}

/// The same day of the month `months` on, the month's last day where it
/// has no such day.
fn months_on(day: Day, months: i64) -> Day {
    let first = Day::new(day.year, i64::from(day.month) + months, 1);
    let last = Day::new(first.year, i64::from(first.month) + 1, 0).day;
    Day::new(
        first.year,
        i64::from(first.month),
        i64::from(day.day.min(last)),
    )
}

/// How the Agenda of a Device showed Shopping Lists (#123), a setting of
/// that Device until each Shopping List had its own choice (#186). It is
/// read only for the default of [`ShoppingOnAgenda`] on this Device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoppingAgenda {
    /// Each Line that came back today on its own row, checkable there.
    Lines,
    /// One row per Shopping List on a day when Lines came back.
    Back,
    /// One row per Shopping List on every day it has open Lines.
    Open,
    /// Not at all.
    Off,
}

impl ShoppingAgenda {
    pub const ALL: [ShoppingAgenda; 4] = [
        ShoppingAgenda::Lines,
        ShoppingAgenda::Back,
        ShoppingAgenda::Open,
        ShoppingAgenda::Off,
    ];

    /// What a Device shows when nothing was chosen.
    pub const STANDARD: ShoppingAgenda = ShoppingAgenda::Back;

    /// How it is stored in the Device's settings, as the Dart core's
    /// enum names it.
    pub fn code(self) -> &'static str {
        match self {
            ShoppingAgenda::Lines => "lines",
            ShoppingAgenda::Back => "back",
            ShoppingAgenda::Open => "open",
            ShoppingAgenda::Off => "off",
        }
    }

    /// The way `code` names; [`ShoppingAgenda::STANDARD`] for none, or a
    /// code not known here.
    pub fn read(code: Option<&str>) -> ShoppingAgenda {
        ShoppingAgenda::ALL
            .into_iter()
            .find(|w| Some(w.code()) == code)
            .unwrap_or(ShoppingAgenda::STANDARD)
    }
}

/// How a Shopping List shows on today's Agenda (#186), the name its code
/// knows the shared [`ListOnAgenda`] by.
pub type ShoppingOnAgenda = ListOnAgenda;

/// How a Shopping List (#186) or a Checklist (#219) shows on today's
/// Agenda, each person's own choice for each list, kept as an Override is
/// (ADR 0030).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListOnAgenda {
    /// Never.
    Off,
    /// One row with the number of its open Lines, such as "Groceries: 4
    /// open" or "Before the trip: 3 of 7 open".
    Count,
    /// Each open Line on a row of its own, checkable there.
    Lines,
}

impl ListOnAgenda {
    pub const ALL: [ListOnAgenda; 3] =
        [ListOnAgenda::Off, ListOnAgenda::Count, ListOnAgenda::Lines];

    /// How it is stored, as the Dart core's enum names it.
    pub fn code(self) -> &'static str {
        match self {
            ListOnAgenda::Off => "off",
            ListOnAgenda::Count => "count",
            ListOnAgenda::Lines => "lines",
        }
    }

    /// The choice `code` names; `None` for none known here.
    pub fn parse(code: &str) -> Option<ListOnAgenda> {
        ListOnAgenda::ALL.into_iter().find(|s| s.code() == code)
    }

    /// What a list nobody chose for shows as on a Device whose Agenda
    /// showed Shopping Lists the way `old` before: off for "Not at all",
    /// each open Line for "Each Line that came back today", else one row
    /// with the count.
    pub fn from_device(old: ShoppingAgenda) -> ListOnAgenda {
        match old {
            ShoppingAgenda::Off => ListOnAgenda::Off,
            ShoppingAgenda::Lines => ListOnAgenda::Lines,
            ShoppingAgenda::Back | ShoppingAgenda::Open => ListOnAgenda::Count,
        }
    }
}

/// A Shopping List on today's Agenda (#186), how it shows there and its
/// open Lines: one row with their number for [`ShoppingOnAgenda::Count`],
/// one row each for [`ShoppingOnAgenda::Lines`].
#[derive(Debug, Clone, PartialEq)]
pub struct ShoppingAgendaRow {
    pub list: ShoppingList,
    pub shows: ShoppingOnAgenda,
    pub lines: Vec<ShoppingLine>,
}

/// One line of a Shopping List (CONTEXT.md): a Line with its reappear
/// time, struck through for a while once checked.
#[derive(Debug, Clone, PartialEq)]
pub struct ShoppingLine {
    /// Entity id, `line:<id>`.
    pub id: String,
    pub note_id: String,
    pub text: String,
    pub reappear: Reappear,
    /// Whether it shows struck through: checked, and its day to come back
    /// not here yet.
    pub struck: bool,
    /// When it was checked last, as its own entry says; `None` when it was
    /// never checked or was brought back since.
    pub checked_at: Option<DateTime<Utc>>,
    /// The day it comes back, at its start, while it is struck; `None`
    /// while it is open, and while it is struck without a reappear time.
    pub back_on: Option<Day>,
    /// The day it came back by itself, at its start, while it is open
    /// since then: on that day it "came back today". `None` while it is
    /// struck, and for a Line never checked or brought back by hand or by
    /// adding its name.
    pub came_back_on: Option<Day>,
    /// Its place as stored.
    pub order: Option<String>,
}

impl ShoppingLine {
    /// Whether it is checked, struck or gone, since a moment on `day` by
    /// `wall`.
    pub fn checked_on(&self, day: Day, wall: &WallClock) -> bool {
        self.checked_at.is_some_and(|at| Day::of(wall(at)) == day)
    }
}

/// A Note whose Lines come back a while after they are checked
/// (CONTEXT.md).
#[derive(Debug, Clone, PartialEq)]
pub struct ShoppingList {
    pub note_id: String,
    /// The id of its making, as the Note's `kind` names it.
    pub making: String,
    /// Every Line that shows: the open ones first, in the order they were
    /// placed in, then the struck ones, the one
    /// checked last at the end.
    pub lines: Vec<ShoppingLine>,
    /// The Lines checked Once: gone, but a name added again brings them
    /// back.
    gone: Vec<ShoppingLine>,
}

impl ShoppingList {
    /// The Line `id` that shows.
    pub fn line(&self, id: &str) -> Option<&ShoppingLine> {
        self.lines.iter().find(|l| l.id == id)
    }

    pub fn open(&self) -> Vec<&ShoppingLine> {
        self.lines.iter().filter(|l| !l.struck).collect()
    }

    /// The Lines checked Once: gone, shown nowhere.
    pub fn gone(&self) -> &[ShoppingLine] {
        &self.gone
    }

    pub fn struck(&self) -> Vec<&ShoppingLine> {
        self.lines.iter().filter(|l| l.struck).collect()
    }

    /// The Lines checked on `day` by `wall` that are still checked, struck
    /// or gone, the one checked first first: what stays struck on the
    /// Agenda for the day (#276).
    pub fn checked_on(&self, day: Day, wall: &WallClock) -> Vec<&ShoppingLine> {
        let mut checked: Vec<&ShoppingLine> = self
            .lines
            .iter()
            .chain(&self.gone)
            .filter(|l| l.checked_on(day, wall))
            .collect();
        checked.sort_by(|a, b| {
            a.checked_at
                .cmp(&b.checked_at)
                .then_with(|| utf16_cmp(&a.id, &b.id))
        });
        checked
    }

    /// The Lines that came back by themselves at the start of `day`.
    pub fn came_back(&self, day: Day) -> Vec<&ShoppingLine> {
        self.lines
            .iter()
            .filter(|l| l.came_back_on == Some(day))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.lines.len()
    }

    pub fn open_count(&self) -> usize {
        self.lines.iter().filter(|l| !l.struck).count()
    }

    pub fn struck_count(&self) -> usize {
        self.lines.iter().filter(|l| l.struck).count()
    }
}

/// A Line of a Shopping List as read from its latest entries, with the
/// moment it sorts by: a struck or gone one by its check, an open one
/// without a valid key by when it came back or was added, on the wall
/// clock.
struct Read {
    line: ShoppingLine,
    gone: bool,
    checked: DateTime<Utc>,
    came: NaiveDateTime,
}

impl Read {
    fn new(
        id: String,
        note_id: &str,
        latest: &HashMap<String, Entry>,
        today: Day,
        wall: &WallClock,
    ) -> Read {
        let value_of = |field: &str| latest.get(field).and_then(|e| e.value.as_deref());
        let reappear = Reappear::read(value_of(keys::REAPPEAR));
        let check = latest.get(checklist_keys::CHECKED);
        let value = check.and_then(|e| e.value.as_deref());
        let mut back = None;
        let mut came_back = None;
        let mut struck = false;
        let mut gone = false;
        let mut checked = DateTime::<Utc>::MIN_UTC;
        let came;
        match (check, value) {
            (Some(check), Some(_)) => {
                gone = reappear == Reappear::Once;
                let day = reappear.back_from(Day::of(wall(check.date_at)));
                // Without a time it stays struck until one is set.
                struck = !gone && day.is_none_or(|day| today < day);
                if struck {
                    back = day;
                }
                if !gone && !struck {
                    came_back = day;
                }
                checked = check.date_at;
                came = day.map_or_else(|| wall(check.date_at), day_start);
            }
            _ => {
                let at = check
                    .or_else(|| latest.get(checklist_keys::TYPE))
                    .map(|e| e.date_at);
                came = at.map_or(NaiveDateTime::MIN, wall);
            }
        }
        Read {
            line: ShoppingLine {
                id,
                note_id: note_id.to_string(),
                text: StoredText::read(value_of(checklist_keys::TEXT)).text,
                reappear,
                struck,
                checked_at: value.and_then(|v| parse_time(v).ok()),
                back_on: back,
                came_back_on: came_back,
                order: value_of(checklist_keys::ORDER).map(str::to_string),
            },
            gone,
            checked,
            came,
        }
    }
}

/// The start of `day` on the wall clock.
fn day_start(day: Day) -> NaiveDateTime {
    i32::try_from(day.year)
        .ok()
        .and_then(|year| NaiveDate::from_ymd_opt(year, day.month, day.day))
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .unwrap_or(NaiveDateTime::MIN)
}

/// Valid keys first, then by key, then by id.
fn by_order(a: &ShoppingLine, b: &ShoppingLine) -> Ordering {
    let av = order::is_valid(a.order.as_deref());
    let bv = order::is_valid(b.order.as_deref());
    bv.cmp(&av)
        .then_with(|| {
            utf16_cmp(
                a.order.as_deref().unwrap_or(""),
                b.order.as_deref().unwrap_or(""),
            )
        })
        .then_with(|| utf16_cmp(&a.id, &b.id))
}

/// The key for a new Line at the place `at` of the open Lines of `list`,
/// or after every key with none or one past them: between the last valid
/// key above it and the first valid key of any Line after that one, a
/// struck or gone Line too.
fn key_at(list: &ShoppingList, at: Option<usize>) -> String {
    let open = list.open();
    let keys = || {
        list.lines
            .iter()
            .chain(&list.gone)
            .filter_map(|l| l.order.as_deref())
            .filter(|key| order::is_valid(Some(key)))
    };
    let at = match at {
        Some(at) if at < open.len() => at,
        _ => {
            let last = keys().max_by(|a, b| utf16_cmp(a, b));
            return order::place(last, None);
        }
    };
    let a = open
        .iter()
        .take(at)
        .rev()
        .find(|l| order::is_valid(l.order.as_deref()))
        .and_then(|l| l.order.as_deref());
    let b = keys()
        .filter(|key| a.is_none_or(|a| utf16_cmp(key, a) == Ordering::Greater))
        .min_by(|a, b| utf16_cmp(a, b));
    order::place(a, b)
}

/// `lines` as they show, the gone ones kept apart.
fn list_of(note_id: &str, making: &str, lines: Vec<Read>) -> ShoppingList {
    let mut open = Vec::new();
    let mut struck = Vec::new();
    let mut gone = Vec::new();
    for r in lines {
        if r.gone {
            gone.push(r);
        } else if r.line.struck {
            struck.push(r);
        } else {
            open.push(r);
        }
    }
    // In the order of their keys; one without a valid key, which no app
    // writes, after them, the one that came last first.
    open.sort_by(|a, b| {
        let valid = order::is_valid(a.line.order.as_deref());
        if valid != order::is_valid(b.line.order.as_deref()) {
            return if valid {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let by_came = if valid {
            Ordering::Equal
        } else {
            b.came.cmp(&a.came)
        };
        by_came.then_with(|| by_order(&a.line, &b.line))
    });
    let by_check = |a: &Read, b: &Read| {
        a.checked
            .cmp(&b.checked)
            .then_with(|| by_order(&a.line, &b.line))
    };
    struck.sort_by(by_check);
    gone.sort_by(by_check);
    ShoppingList {
        note_id: note_id.to_string(),
        making: making.to_string(),
        lines: open.into_iter().chain(struck).map(|r| r.line).collect(),
        gone: gone.into_iter().map(|r| r.line).collect(),
    }
}

/// Making Notes Shopping Lists and back, and adding, checking and
/// changing their Lines (ADR 0030).
pub struct ShoppingLists<'a> {
    store: &'a Store,
}

impl<'a> ShoppingLists<'a> {
    pub fn new(store: &'a Store) -> ShoppingLists<'a> {
        ShoppingLists { store }
    }

    /// The Shopping List of the Note `note_id` as it shows now, with days
    /// as `wall` shows them, or `None` when it is none: a plain Note, a
    /// Checklist, a Tally (the Tally wins), or no Note.
    pub fn by_note(&self, note_id: &str, wall: &WallClock) -> Result<Option<ShoppingList>> {
        Ok(self.of_notes(&[note_id.to_string()], wall)?.remove(note_id))
    }

    /// Whether the Note `note_id` is a Shopping List.
    pub fn is_shopping_list(&self, note_id: &str) -> Result<bool> {
        Ok(self.making(note_id)?.is_some())
    }

    /// The Shopping Lists of the Notes `note_ids` that are one, read in
    /// one pass over the Lines, by Note.
    pub fn of_notes(
        &self,
        note_ids: &[String],
        wall: &WallClock,
    ) -> Result<BTreeMap<String, ShoppingList>> {
        let mut makings: HashMap<String, String> = HashMap::new();
        for id in note_ids {
            if let Some(making) = self.making(id)? {
                makings.insert(id.clone(), making);
            }
        }
        let mut out = BTreeMap::new();
        if makings.is_empty() {
            return Ok(out);
        }
        let today = Day::today(self.store.now(), wall);
        let mut read: HashMap<String, Vec<Read>> = HashMap::new();
        for id in self.store.entities_of_type(checklist_keys::TYPE_LINE)? {
            let mut latest: HashMap<String, Entry> = HashMap::new();
            for e in self.store.history(&id)? {
                let newer = latest
                    .get(&e.field)
                    .is_none_or(|held| Store::latest_wins(&e, held) == Ordering::Greater);
                if newer {
                    latest.insert(e.field.clone(), e);
                }
            }
            let value_of = |field: &str| latest.get(field).and_then(|e| e.value.clone());
            let Some(note) = value_of(checklist_keys::NOTE) else {
                continue;
            };
            if value_of(checklist_keys::TRASHED).is_some() {
                continue;
            }
            let Some(making) = makings.get(&note) else {
                continue;
            };
            if value_of(checklist_keys::MADE).as_deref() != Some(making.as_str()) {
                continue;
            }
            let line = Read::new(id, &note, &latest, today, wall);
            read.entry(note).or_default().push(line);
        }
        for (note, making) in makings {
            let lines = read.remove(&note).unwrap_or_default();
            out.insert(note.clone(), list_of(&note, &making, lines));
        }
        Ok(out)
    }

    /// The Shopping Lists on today's Agenda by `wall` (#186): each list
    /// not in the trash nor merged that has an open Line and that this
    /// person does not keep [`ShoppingOnAgenda::Off`], with how it shows
    /// ([`ShoppingLists::on_agenda`]) and its open Lines; oldest capture
    /// first. A list whose Lines are all struck or gone shows whatever the
    /// choice. Reading writes nothing.
    pub fn agenda_rows(&self, wall: &WallClock) -> Result<Vec<ShoppingAgendaRow>> {
        self.agenda_rows_with(wall, false)
    }

    /// [`ShoppingLists::agenda_rows`]; with `checked_today` the Lines
    /// checked today and still checked follow the open ones, struck for
    /// the day ([`ShoppingList::checked_on`]), and a list with only those
    /// shows too (#276). Reading writes nothing.
    pub fn agenda_rows_with(
        &self,
        wall: &WallClock,
        checked_today: bool,
    ) -> Result<Vec<ShoppingAgendaRow>> {
        let notes = Notes::new(self.store);
        let standard =
            ShoppingOnAgenda::from_device(DeviceSettings::new(self.store).shopping_agenda()?);
        let mut own = HashMap::new();
        let mut shown = Vec::new();
        for id in self.store.entities_of_type(note_keys::TYPE_NOTE)? {
            if self.making(&id)?.is_none() {
                continue;
            }
            let shows = self.shows_on_agenda(&id, standard, &mut own)?;
            if shows == ShoppingOnAgenda::Off {
                continue;
            }
            if let Some(note) = notes.by_id(&id)?
                && !note.is_trashed()
                && !note.is_merged()
            {
                shown.push((note, shows));
            }
        }
        shown.sort_by(|(a, _), (b, _)| {
            a.captured
                .cmp(&b.captured)
                .then_with(|| utf16_cmp(&a.id, &b.id))
        });
        let ids: Vec<String> = shown.iter().map(|(n, _)| n.id.clone()).collect();
        let mut lists = self.of_notes(&ids, wall)?;
        // The clock is read only when asked for.
        let today = checked_today.then(|| Day::today(self.store.now(), wall));
        let mut out = Vec::new();
        for (note, shows) in shown {
            let Some(list) = lists.remove(&note.id) else {
                continue;
            };
            let mut lines: Vec<ShoppingLine> = list.open().into_iter().cloned().collect();
            if let Some(day) = today {
                lines.extend(list.checked_on(day, wall).into_iter().cloned());
            }
            if !lines.is_empty() {
                out.push(ShoppingAgendaRow { list, shows, lines });
            }
        }
        Ok(out)
    }

    /// How the Shopping List of `note_id` shows on today's Agenda for this
    /// person: as one of the user's own Devices chose last; for a list
    /// nobody chose for, as this Device showed Shopping Lists before
    /// (#186, [`ShoppingOnAgenda::from_device`]). What another person's
    /// Device wrote counts for nothing, so in a Shared Area each person
    /// decides for their own Agenda; a value no app writes reads as
    /// [`ShoppingOnAgenda::Count`], so it hides no list.
    pub fn on_agenda(&self, note_id: &str) -> Result<ShoppingOnAgenda> {
        let standard =
            ShoppingOnAgenda::from_device(DeviceSettings::new(self.store).shopping_agenda()?);
        self.shows_on_agenda(note_id, standard, &mut HashMap::new())
    }

    fn shows_on_agenda(
        &self,
        note_id: &str,
        standard: ShoppingOnAgenda,
        own: &mut HashMap<String, bool>,
    ) -> Result<ShoppingOnAgenda> {
        Ok(match self.chosen(note_id, own)? {
            None => standard,
            Some(code) => ShoppingOnAgenda::parse(&code).unwrap_or(ShoppingOnAgenda::Count),
        })
    }

    fn chosen(&self, note_id: &str, own: &mut HashMap<String, bool>) -> Result<Option<String>> {
        let entity = format!("{}{note_id}", checklist_keys::AGENDA_PREFIX);
        self.store.own_current(&entity, keys::AGENDA, own)
    }

    /// Whether "Show on the Agenda" applies to the Note `note_id`: a
    /// Shopping List not in the trash nor merged. One with a Routine or
    /// Reminder shows too, as its Lines keep their own checks.
    pub fn may_show_on_agenda(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed() && !note.is_merged() && self.making(note_id)?.is_some())
    }

    /// Sets how the Shopping List of `note_id` shows on the Agenda for
    /// this person alone, kept like an Override: on the user's own Devices
    /// only, whatever their role in its Area. The choice is written even
    /// when the list already shows that way by this Device's default, so
    /// every own Device shows it alike; the same choice again writes
    /// nothing.
    pub fn set_on_agenda(&self, note_id: &str, shows: ShoppingOnAgenda) -> Result<()> {
        if self.making(note_id)?.is_none() {
            return Err(Error::Invalid(format!("not a Shopping List: {note_id}")));
        }
        if self.chosen(note_id, &mut HashMap::new())?.as_deref() == Some(shows.code()) {
            return Ok(());
        }
        let entity = format!("{}{note_id}", checklist_keys::AGENDA_PREFIX);
        self.store
            .append(&entity, keys::AGENDA, Some(shows.code()), None)?;
        Ok(())
    }

    /// The making of the Shopping List the Note `note_id` is now, or
    /// `None` when it is none: no Note, another list, or a Tally.
    pub fn making(&self, note_id: &str) -> Result<Option<String>> {
        let fields = self.store.current_fields(note_id)?;
        if fields.get(note_keys::TYPE) != Some(note_keys::TYPE_NOTE) {
            return Ok(None);
        }
        let Some((list, making)) = parse_list_kind(fields.get(checklist_keys::KIND)) else {
            return Ok(None);
        };
        if list != keys::SHOPPING || Tallies::new(self.store).is_tally(note_id)? {
            return Ok(None);
        }
        Ok(Some(making.to_string()))
    }

    /// Whether the Line of `fields` is gone: checked, and Once.
    pub fn is_gone(fields: &Fields) -> bool {
        fields.get(checklist_keys::CHECKED).is_some()
            && Reappear::read(fields.get(keys::REAPPEAR)) == Reappear::Once
    }

    /// What a name is compared as when it is added: trimmed, in lower
    /// case, as the Dart core's `trim` and `toLowerCase` make it.
    pub fn name_key(text: &str) -> String {
        lower(dart_trim(text))
    }

    /// Whether this Device may make the Note `note_id` a Shopping List: a
    /// Note in the lists, of a text known here and not code, neither a Tally
    /// nor a list already, which it may change.
    pub fn may_make(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed()
            && !note.is_merged()
            && note.text_editable
            && note.text_format != crate::text_format::CODE
            && parse_list_kind(
                self.store
                    .current(note_id, checklist_keys::KIND)?
                    .as_deref(),
            )
            .is_none()
            && !Tallies::new(self.store).is_tally(note_id)?
            && !crate::sessions::Sessions::new(self.store).is_session_note(note_id)?
            && self.store.may_change(note_id)?)
    }

    /// Makes the Note `note_id` a Shopping List: its first line stays its
    /// text, every other line not blank becomes a Line, indented or not,
    /// struck when it starts with "✓ ", in a formatted text without the
    /// mark of an item or a quote ([`parse_line_in`]). Every Line comes in
    /// at one moment, so they show in the order of the text. Takes it out of Unsorted, as
    /// a Checklist does. The text keeps its format.
    pub fn make(&self, note_id: &str) -> Result<()> {
        let refused = || Error::Invalid(format!("{note_id} may not be made a Shopping List here"));
        let note = Notes::new(self.store).by_id(note_id)?.ok_or_else(refused)?;
        if !self.may_make(note_id)? {
            return Err(refused());
        }
        let rows = split_lines(&note.text);
        let title = dart_trim(rows.first().map(String::as_str).unwrap_or(""));
        let markdown = note.text_format == text_format::MARKDOWN;
        let parsed: Vec<_> = rows
            .iter()
            .skip(1)
            .filter_map(|r| parse_line_in(r, markdown))
            .collect();
        let making = self.store.new_id()?;
        self.store.batch(|| {
            let s = self.store;
            let kind = keys::kind_of(&making);
            s.append(note_id, checklist_keys::KIND, Some(&kind), None)?;
            if title != note.text {
                let text = StoredText::of(&note.text_format, title, VERSION).encode();
                s.append(note_id, note_keys::TEXT, Some(&text), None)?;
            }
            if !note.spawned {
                let at = dart_iso(s.now());
                s.append(note_id, note_keys::SPAWNED, Some(&at), None)?;
            }
            let now = s.now();
            let mut key = String::new();
            for line in &parsed {
                key = order::after(&key);
                self.write_line(note_id, &making, &line.text, &key, line.checked, Some(now))?;
            }
            Ok(())
        })
    }

    /// Whether this Device may switch the list the Note `note_id` is, a
    /// Checklist to a Shopping List or the other way ([`Self::switch_list`]):
    /// a Note in the lists that is one of the two, which it may change.
    pub fn may_switch(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed()
            && !note.is_merged()
            && (self.making(note_id)?.is_some()
                || Checklists::new(self.store).is_checklist(note_id)?)
            && self.store.may_change(note_id)?)
    }

    /// Switches the list the Note `note_id` is (#188): a Checklist becomes
    /// a Shopping List and a Shopping List a Checklist, in a new making.
    /// The Lines that show carry over, as Lines of the new making, in the
    /// order they show, at one moment: a checked Line checked, a struck one
    /// checked, each at the top with the reappear time a new Line gets on
    /// a Shopping List. A Checklist's Lines show their own checks here, not
    /// those of an Occurrence. The text is not touched; the Lines of the
    /// old making stay in the log.
    pub fn switch_list(&self, note_id: &str, wall: &WallClock) -> Result<()> {
        if !self.may_switch(note_id)? {
            return Err(Error::Invalid(format!(
                "the list of {note_id} may not be switched here"
            )));
        }
        let shopping = self.by_note(note_id, wall)?;
        let carried: Vec<(String, bool)> = match &shopping {
            Some(list) => list
                .lines
                .iter()
                .map(|l| (l.text.clone(), l.struck))
                .collect(),
            None => Checklists::new(self.store)
                .by_note(note_id)?
                .map(|list| list.lines)
                .unwrap_or_default()
                .into_iter()
                .map(|l| (l.text, l.checked))
                .collect(),
        };
        let making = self.store.new_id()?;
        self.store.batch(|| {
            let s = self.store;
            let kind = match shopping {
                Some(_) => checklist_keys::kind_of(&making),
                None => keys::kind_of(&making),
            };
            s.append(note_id, checklist_keys::KIND, Some(&kind), None)?;
            let now = s.now();
            let mut key = String::new();
            for (text, checked) in &carried {
                key = order::after(&key);
                match shopping {
                    Some(_) => {
                        write_line(
                            s,
                            note_id,
                            &making,
                            &NewLine {
                                text,
                                order: &key,
                                indent: false,
                                checked: *checked,
                                received: None,
                                at: Some(now),
                            },
                        )?;
                    }
                    None => {
                        self.write_line(note_id, &making, text, &key, *checked, Some(now))?;
                    }
                }
            }
            Ok(())
        })
    }

    /// Turns the Shopping List of `note_id` back into a plain Note: every
    /// Line that shows goes into its text, one per line, under its text, a
    /// struck one marked "✓ ", in the format the text is in. The Lines and
    /// their checks stay in the log.
    pub fn to_plain(&self, note_id: &str, wall: &WallClock) -> Result<()> {
        let (note, list) = self.require_list(note_id, wall)?;
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        if !note.text_editable {
            return Err(Error::Invalid(format!(
                "the text of {note_id} is in a format not known here"
            )));
        }
        let text = ShoppingLists::plain_text(&note.text, &list);
        let text = StoredText::of(&note.text_format, &text, VERSION).encode();
        self.store.batch(|| {
            self.store
                .append(note_id, note_keys::TEXT, Some(&text), None)?;
            self.store
                .append(note_id, checklist_keys::KIND, None, None)?;
            Ok(())
        })
    }

    /// The text the Shopping List `list` of a Note titled `title` goes
    /// back to.
    pub fn plain_text(title: &str, list: &ShoppingList) -> String {
        let mut rows = Vec::new();
        if !title.is_empty() {
            rows.push(title.to_string());
        }
        for line in &list.lines {
            let mark = if line.struck {
                checklist_keys::CHECK_MARK
            } else {
                ""
            };
            rows.push(format!("{mark}{}", line.text));
        }
        rows.join("\n")
    }

    /// Whether this Device may add Lines to the Shopping List of `note_id`
    /// and check them: anywhere it holds a role.
    pub fn may_check(&self, note_id: &str) -> Result<bool> {
        self.store.may_write_in(note_id)
    }

    /// Whether this Device may rename, delete and move the Line `line_id`
    /// and set its reappear time: as for a Checklist's Line.
    pub fn may_edit(&self, line_id: &str) -> Result<bool> {
        Checklists::new(self.store).may_edit(line_id)
    }

    /// Adds `text` to the Shopping List of `note_id` after every other Line,
    /// as [`ShoppingLists::add_at`] does.
    pub fn add(&self, note_id: &str, text: &str, wall: &WallClock) -> Result<String> {
        self.add_at(note_id, text, wall, None)
    }

    /// Adds `text` to the Shopping List of `note_id` at `at`, an index in
    /// the order its open Lines show, or after every other. A name it has
    /// already, in another case or with spaces around it, adds nothing: a
    /// struck or gone Line of that name comes back now, in its place, an
    /// open one stays. Anyone with a role in its Area may. Returns the id
    /// of the Line added, brought back or found.
    pub fn add_at(
        &self,
        note_id: &str,
        text: &str,
        wall: &WallClock,
        at: Option<usize>,
    ) -> Result<String> {
        let (_, list) = self.require_list(note_id, wall)?;
        if !self.may_check(note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {note_id}"
            )));
        }
        let named = line_text(text)?;
        let wanted = ShoppingLists::name_key(&named);
        if let Some(line) = list
            .lines
            .iter()
            .find(|l| ShoppingLists::name_key(&l.text) == wanted)
        {
            if line.struck {
                self.store
                    .append(&line.id, checklist_keys::CHECKED, None, None)?;
            }
            return Ok(line.id.clone());
        }
        if let Some(line) = list
            .gone
            .iter()
            .find(|l| ShoppingLists::name_key(&l.text) == wanted)
        {
            self.store
                .append(&line.id, checklist_keys::CHECKED, None, None)?;
            return Ok(line.id.clone());
        }
        let key = key_at(&list, at);
        self.store
            .batch(|| self.write_line(note_id, &list.making, &named, &key, false, None))
    }

    /// The Notes that are a Shopping List this Device may add Lines to:
    /// each one not in the trash nor merged where it holds a role, oldest
    /// capture first. What a capture can be sorted into (#124).
    pub fn addable(&self) -> Result<Vec<Note>> {
        let notes = Notes::new(self.store);
        let mut found = Vec::new();
        for id in self.store.entities_of_type(note_keys::TYPE_NOTE)? {
            if self.making(&id)?.is_none() || !self.may_check(&id)? {
                continue;
            }
            if let Some(note) = notes.by_id(&id)?
                && !note.is_trashed()
                && !note.is_merged()
            {
                found.push(note);
            }
        }
        found.sort_by(|a, b| {
            a.captured
                .cmp(&b.captured)
                .then_with(|| utf16_cmp(&a.id, &b.id))
        });
        Ok(found)
    }

    /// Adds every line of `text` that is not blank to the Shopping List of
    /// `note_id` as [`ShoppingLists::add`] does, all in one batch, in the
    /// order of the lines after every other: what a capture sorted into the
    /// list becomes (#124). Returns the ids of the Lines added, brought back
    /// or found, in the order of the lines; none for a blank `text`.
    pub fn add_lines(&self, note_id: &str, text: &str, wall: &WallClock) -> Result<Vec<String>> {
        let rows: Vec<String> = split_lines(text)
            .into_iter()
            .filter(|row| !dart_trim(row).is_empty())
            .collect();
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        self.store.batch(|| {
            rows.iter()
                .map(|row| self.add(note_id, row, wall))
                .collect::<Result<Vec<String>>>()
        })
    }

    /// Renames the Line `line_id`.
    pub fn rename(&self, line_id: &str, text: &str) -> Result<()> {
        let fields = self.require_editable(line_id)?;
        let named = line_text(text)?;
        if named == StoredText::read(fields.get(checklist_keys::TEXT)).text {
            return Ok(());
        }
        let text = StoredText::plain(&named).encode();
        self.store
            .append(line_id, checklist_keys::TEXT, Some(&text), None)
            .map(drop)
    }

    /// Sets how long the Line `line_id` stays struck once checked.
    pub fn set_reappear(&self, line_id: &str, reappear: Reappear) -> Result<()> {
        let fields = self.require_editable(line_id)?;
        if Reappear::read(fields.get(keys::REAPPEAR)) == reappear {
            return Ok(());
        }
        self.store
            .append(line_id, keys::REAPPEAR, Some(&reappear.code()), None)
            .map(drop)
    }

    /// Moves the open Line `line_id` to just before the open Line `before`,
    /// or after the open Lines (#283): its key goes where
    /// [`ShoppingLists::add_at`] puts a new Line there, so a struck Line
    /// keeps its place among the open ones for when it comes back. A
    /// struck Line does not move. Where it is already, nothing is written.
    pub fn move_line(&self, line_id: &str, wall: &WallClock, before: Option<&str>) -> Result<()> {
        self.require_editable(line_id)?;
        let (list, line) = self.require_shown(line_id, wall, false)?;
        if line.struck {
            return Err(Error::Invalid(format!(
                "a struck Line does not move: {line_id}"
            )));
        }
        let from = list.open().iter().position(|l| l.id == line_id);
        let rest = ShoppingList {
            lines: list
                .lines
                .iter()
                .filter(|l| l.id != line_id)
                .cloned()
                .collect(),
            ..list
        };
        let others = rest.open();
        let mut at = None;
        if let Some(before) = before {
            if before == line_id {
                return Ok(());
            }
            at = Some(
                others
                    .iter()
                    .position(|l| l.id == before)
                    .ok_or_else(|| Error::Invalid(format!("not an open Line: {before}")))?,
            );
        }
        if Some(at.unwrap_or(others.len())) == from {
            return Ok(());
        }
        let key = key_at(&rest, at);
        self.store
            .append(line_id, checklist_keys::ORDER, Some(&key), None)
            .map(drop)
    }

    /// Deletes the Line `line_id`. It can be restored.
    pub fn delete(&self, line_id: &str) -> Result<()> {
        self.require_editable(line_id)?;
        let at = dart_iso(self.store.now());
        self.store
            .append(line_id, checklist_keys::TRASHED, Some(&at), None)
            .map(drop)
    }

    /// Brings the deleted Line `line_id` back.
    pub fn restore(&self, line_id: &str) -> Result<()> {
        let fields = self.store.current_fields(line_id)?;
        let deleted = match fields.get(checklist_keys::NOTE) {
            Some(note) => {
                fields.get(checklist_keys::TYPE) == Some(checklist_keys::TYPE_LINE)
                    && fields.get(checklist_keys::TRASHED).is_some()
                    && fields.get(checklist_keys::MADE) == self.making(note)?.as_deref()
            }
            None => false,
        };
        if !deleted {
            return Err(Error::Invalid(format!("not a deleted Line: {line_id}")));
        }
        if !self.may_edit(line_id)? {
            return Err(Error::Invalid(format!("may not change {line_id}")));
        }
        self.store
            .append(line_id, checklist_keys::TRASHED, None, None)
            .map(drop)
    }

    /// The deleted Lines of the Shopping List of `note_id` that
    /// [`ShoppingLists::restore`] brings back, the last deleted first;
    /// none when it is no Shopping List.
    pub fn deleted_lines(&self, note_id: &str) -> Result<Vec<DeletedLine>> {
        deleted_lines_of(self.store, note_id, self.making(note_id)?.as_deref())
    }

    /// Checks the Line `line_id`: it is struck through and goes to the end
    /// until the start of the day its reappear time brings it back on
    /// ([`Reappear::back_from`]), by `wall`, or until a time is set for one
    /// without; a Line Once is gone. A struck one stays as it is.
    pub fn check(&self, line_id: &str, wall: &WallClock) -> Result<()> {
        let (list, line) = self.require_shown(line_id, wall, false)?;
        if !self.may_check(&list.note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {}",
                list.note_id
            )));
        }
        if line.struck {
            return Ok(());
        }
        let at = dart_iso(self.store.now());
        self.store
            .append(line_id, checklist_keys::CHECKED, Some(&at), None)
            .map(drop)
    }

    /// Brings the struck or gone Line `line_id` back now, in its place, as
    /// adding its name does. An open one stays as it is.
    pub fn uncheck(&self, line_id: &str, wall: &WallClock) -> Result<()> {
        let (list, line) = self.require_shown(line_id, wall, true)?;
        if !self.may_check(&list.note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {}",
                list.note_id
            )));
        }
        if !line.struck && !list.gone.iter().any(|l| l.id == line.id) {
            return Ok(());
        }
        self.store
            .append(line_id, checklist_keys::CHECKED, None, None)
            .map(drop)
    }

    /// Writes a new Line, inside a batch, with the reappear time new Lines
    /// get, at `at` or now; returns its id.
    fn write_line(
        &self,
        note_id: &str,
        making: &str,
        text: &str,
        key: &str,
        checked: bool,
        at: Option<DateTime<Utc>>,
    ) -> Result<String> {
        let now = at.unwrap_or_else(|| self.store.now());
        let id = write_line(
            self.store,
            note_id,
            making,
            &NewLine {
                text,
                order: key,
                indent: false,
                checked,
                received: None,
                at: Some(now),
            },
        )?;
        self.store.append(
            &id,
            keys::REAPPEAR,
            Some(&Reappear::Unset.code()),
            Some(now),
        )?;
        Ok(id)
    }

    /// The Note `note_id`, not in the trash, and its Shopping List.
    fn require_list(&self, note_id: &str, wall: &WallClock) -> Result<(Note, ShoppingList)> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let list = self.by_note(note_id, wall)?;
        match (note, list) {
            (Some(note), Some(list)) if !note.is_trashed() => Ok((note, list)),
            _ => Err(Error::Invalid(format!("not a Shopping List: {note_id}"))),
        }
    }

    /// The fields of the Line `line_id` that shows in a Shopping List whose
    /// Note is not in the trash, or with `gone` one gone there.
    fn require_line(&self, line_id: &str, gone: bool) -> Result<Fields> {
        let fields = self.store.current_fields(line_id)?;
        let note = fields.get(checklist_keys::NOTE).map(str::to_string);
        let making = match &note {
            Some(note) => self.making(note)?,
            None => None,
        };
        let trashed = match &note {
            Some(note) => Notes::new(self.store)
                .by_id(note)?
                .is_none_or(|n| n.is_trashed()),
            None => true,
        };
        let shown = fields.get(checklist_keys::TYPE) == Some(checklist_keys::TYPE_LINE)
            && making.is_some()
            && fields.get(checklist_keys::MADE) == making.as_deref()
            && fields.get(checklist_keys::TRASHED).is_none()
            && !trashed
            && (gone || !ShoppingLists::is_gone(&fields));
        if !shown {
            return Err(Error::Invalid(format!(
                "not a Line of a Shopping List: {line_id}"
            )));
        }
        Ok(fields)
    }

    fn require_editable(&self, line_id: &str) -> Result<Fields> {
        let fields = self.require_line(line_id, false)?;
        if !self.may_edit(line_id)? {
            return Err(Error::Invalid(format!("may not change {line_id}")));
        }
        Ok(fields)
    }

    /// The Shopping List the shown Line `line_id` lies in, and the Line;
    /// with `gone` a gone one too.
    fn require_shown(
        &self,
        line_id: &str,
        wall: &WallClock,
        gone: bool,
    ) -> Result<(ShoppingList, ShoppingLine)> {
        let missing = || Error::Invalid(format!("not a Line of a Shopping List: {line_id}"));
        let fields = self.require_line(line_id, gone)?;
        let note = fields.get(checklist_keys::NOTE).ok_or_else(missing)?;
        let list = self.by_note(note, wall)?.ok_or_else(missing)?;
        let line = list
            .line(line_id)
            .or_else(|| {
                gone.then(|| list.gone.iter().find(|l| l.id == line_id))
                    .flatten()
            })
            .cloned()
            .ok_or_else(missing)?;
        Ok((list, line))
    }
}

#[cfg(test)]
mod tests;
