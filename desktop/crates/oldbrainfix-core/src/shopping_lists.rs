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
    Checklists, NewLine, keys as checklist_keys, line_text, order, parse_line_in, parse_list_kind,
    split_lines, write_line,
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
        let mut out = Vec::new();
        for (note, shows) in shown {
            let Some(list) = lists.remove(&note.id) else {
                continue;
            };
            let lines: Vec<ShoppingLine> = list.open().into_iter().cloned().collect();
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

    /// Whether this Device may rename and delete the Line `line_id` and
    /// set its reappear time: as for a Checklist's Line.
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

    /// Checks the Line `line_id`: it is struck through and goes to the end
    /// until the start of the day its reappear time brings it back on
    /// ([`Reappear::back_from`]), by `wall`, or until a time is set for one
    /// without; a Line Once is gone. A struck one stays as it is.
    pub fn check(&self, line_id: &str, wall: &WallClock) -> Result<()> {
        let (list, line) = self.require_shown(line_id, wall)?;
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

    /// Brings the struck Line `line_id` back now, in its place. An open one
    /// stays as it is.
    pub fn uncheck(&self, line_id: &str, wall: &WallClock) -> Result<()> {
        let (list, line) = self.require_shown(line_id, wall)?;
        if !self.may_check(&list.note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {}",
                list.note_id
            )));
        }
        if !line.struck {
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
    /// Note is not in the trash.
    fn require_line(&self, line_id: &str) -> Result<Fields> {
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
            && !ShoppingLists::is_gone(&fields);
        if !shown {
            return Err(Error::Invalid(format!(
                "not a Line of a Shopping List: {line_id}"
            )));
        }
        Ok(fields)
    }

    fn require_editable(&self, line_id: &str) -> Result<Fields> {
        let fields = self.require_line(line_id)?;
        if !self.may_edit(line_id)? {
            return Err(Error::Invalid(format!("may not change {line_id}")));
        }
        Ok(fields)
    }

    /// The Shopping List the shown Line `line_id` lies in, and the Line.
    fn require_shown(
        &self,
        line_id: &str,
        wall: &WallClock,
    ) -> Result<(ShoppingList, ShoppingLine)> {
        let missing = || Error::Invalid(format!("not a Line of a Shopping List: {line_id}"));
        let fields = self.require_line(line_id)?;
        let note = fields.get(checklist_keys::NOTE).ok_or_else(missing)?;
        let list = self.by_note(note, wall)?.ok_or_else(missing)?;
        let line = list.line(line_id).cloned().ok_or_else(missing)?;
        Ok((list, line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checklists::tests::{TestResult, exchange, invite, own_exchange, send};
    use crate::routines::Routines;
    use crate::schedule::{Occurrence, Schedule};
    use crate::shared::AreaRole;
    use crate::signing::SigningKey;
    use crate::{OwnProfile, Places, Search};
    use chrono::{Duration, TimeZone};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// A clock that moves one second per read from where it is set, in
    /// microseconds since the epoch.
    #[derive(Clone)]
    struct Clock(Arc<AtomicI64>);

    impl Clock {
        /// A Thursday.
        fn new() -> Made<Clock> {
            let start = Utc
                .with_ymd_and_hms(2026, 3, 5, 10, 0, 0)
                .single()
                .ok_or("a start")?;
            Ok(Clock(Arc::new(AtomicI64::new(start.timestamp_micros()))))
        }

        /// The next read is `at`.
        fn set(&self, at: DateTime<Utc>) {
            self.0
                .store(at.timestamp_micros() - 1_000_000, AtomicOrdering::SeqCst);
        }

        fn store(&self, seed: u8) -> Made<Store> {
            let ticks = self.0.clone();
            let clock: crate::Clock = Box::new(move || {
                let n = ticks.fetch_add(1_000_000, AtomicOrdering::SeqCst) + 1_000_000;
                DateTime::from_timestamp_micros(n).unwrap_or_default()
            });
            Ok(Store::in_memory(clock, move || {
                Ok(SigningKey::from_seed([seed; 32]))
            })?)
        }

        fn device(&self, seed: u8, name: &str) -> Made<Store> {
            let store = self.store(seed)?;
            OwnProfile::new(&store).apply_defaults(name, None)?;
            Ok(store)
        }
    }

    fn utc(day: u32, hour: u32, minute: u32, second: u32) -> Made<DateTime<Utc>> {
        Ok(Utc
            .with_ymd_and_hms(2026, 3, day, hour, minute, second)
            .single()
            .ok_or("a moment")?)
    }

    /// The wall clock an hour ahead of UTC, as in Central Europe's winter.
    fn wall(utc: DateTime<Utc>) -> NaiveDateTime {
        (utc + Duration::hours(1)).naive_utc()
    }

    fn shopping(store: &Store, text: &str, place: Option<&str>) -> Made<String> {
        let id = Notes::new(store).capture(text, place, None)?.id;
        ShoppingLists::new(store).make(&id)?;
        Ok(id)
    }

    /// The Lines of the Shopping List of `note` as they show: `~text` for
    /// a struck one.
    /// Gives each Line of the Shopping List of `note` a week.
    fn weekly(store: &Store, note: &str) -> Made<()> {
        let lists = ShoppingLists::new(store);
        for line in lists.by_note(note, &wall)?.ok_or("a Shopping List")?.lines {
            lists.set_reappear(&line.id, Reappear::FALLBACK)?;
        }
        Ok(())
    }

    fn shown(store: &Store, note: &str) -> Made<Vec<String>> {
        let list = ShoppingLists::new(store).by_note(note, &wall)?;
        Ok(list
            .map(|l| l.lines)
            .unwrap_or_default()
            .iter()
            .map(|l| format!("{}{}", if l.struck { "~" } else { "" }, l.text))
            .collect())
    }

    fn line_of(store: &Store, note: &str, text: &str) -> Made<ShoppingLine> {
        let list = ShoppingLists::new(store)
            .by_note(note, &wall)?
            .ok_or("a Shopping List")?;
        Ok(list
            .lines
            .into_iter()
            .find(|l| l.text == text)
            .ok_or("a Line")?)
    }

    fn id_of(store: &Store, note: &str, text: &str) -> Made<String> {
        Ok(line_of(store, note, text)?.id)
    }

    /// How many entries the log holds for `field` of `entity`.
    fn writes(store: &Store, entity: &str, field: &str) -> Made<usize> {
        Ok(store
            .history(entity)?
            .iter()
            .filter(|e| e.field == field)
            .count())
    }

    #[test]
    fn a_checklist_switches_to_a_shopping_list_of_its_lines_the_text_untouched() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = Notes::new(&ann)
            .capture("Trip\nPassport\n Socks\nTickets", None, None)?
            .id;
        let lists = Checklists::new(&ann);
        lists.make(&note)?;
        let third = lists.by_note(&note)?.ok_or("a Checklist")?.lines[2]
            .id
            .clone();
        lists.check(&third)?;
        let texts = writes(&ann, &note, note_keys::TEXT)?;
        let shops = ShoppingLists::new(&ann);
        assert!(shops.may_switch(&note)?);
        shops.switch_list(&note, &wall)?;
        assert!(lists.by_note(&note)?.is_none());
        assert_eq!(shown(&ann, &note)?, ["Passport", "Socks", "~Tickets"]);
        assert_eq!(line_of(&ann, &note, "Socks")?.reappear, Reappear::Unset);
        assert_eq!(writes(&ann, &note, note_keys::TEXT)?, texts, "no text");
        let made = Notes::new(&ann).by_id(&note)?.ok_or("the Note")?;
        assert_eq!(made.text, "Trip");
        Ok(())
    }

    #[test]
    fn a_shopping_list_switches_to_a_checklist_of_the_lines_that_show() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nBread\nEggs", None)?;
        let shops = ShoppingLists::new(&ann);
        shops.set_reappear(&id_of(&ann, &note, "Bread")?, Reappear::Once)?;
        shops.check(&id_of(&ann, &note, "Bread")?, &wall)?;
        shops.check(&id_of(&ann, &note, "Milk")?, &wall)?;
        shops.switch_list(&note, &wall)?;
        assert!(!shops.is_shopping_list(&note)?);
        let list = Checklists::new(&ann).by_note(&note)?.ok_or("a Checklist")?;
        let lines: Vec<String> = list
            .lines
            .iter()
            .map(|l| format!("{}{}", if l.checked { "~" } else { "" }, l.text))
            .collect();
        assert_eq!(lines, ["Eggs", "~Milk"]);
        assert!(list.lines.iter().all(|l| !l.in_group()));
        Ok(())
    }

    #[test]
    fn switching_back_and_forth_starts_a_new_making_each_time() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk", None)?;
        let shops = ShoppingLists::new(&ann);
        let first = shops.making(&note)?.ok_or("a making")?;
        shops.switch_list(&note, &wall)?;
        shops.switch_list(&note, &wall)?;
        assert_ne!(shops.making(&note)?.ok_or("a making")?, first);
        assert_eq!(shown(&ann, &note)?, ["Milk"]);
        assert_eq!(
            ann.entities_with_prefix(checklist_keys::LINE_PREFIX)?.len(),
            3
        );
        Ok(())
    }

    #[test]
    fn neither_a_plain_note_nor_a_tally_switches() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let shops = ShoppingLists::new(&ann);
        let plain = Notes::new(&ann).capture("Shop\nMilk", None, None)?.id;
        assert!(!shops.may_switch(&plain)?);
        assert!(shops.switch_list(&plain, &wall).is_err());
        let tally = Notes::new(&ann).capture("Gym", None, None)?.id;
        Tallies::new(&ann).add_field(&tally, "Reps", "reps", None, None)?;
        assert!(!shops.may_switch(&tally)?);
        Ok(())
    }

    #[test]
    fn the_first_line_stays_and_the_others_become_lines_in_their_order() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = Notes::new(&ann).capture(
            "Groceries\r\nMilk\n  Eggs\n\tTea\n\n✓ Bread\r   \nApples",
            None,
            None,
        )?;
        let lists = ShoppingLists::new(&ann);
        assert!(lists.may_make(&note.id)?);
        lists.make(&note.id)?;
        let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
        assert_eq!(made.text, "Groceries");
        assert!(!made.is_unsorted());
        assert_eq!(
            shown(&ann, &note.id)?,
            ["Milk", "Eggs", "Tea", "Apples", "~Bread"]
        );
        let list = lists.by_note(&note.id, &wall)?.ok_or("a Shopping List")?;
        assert_eq!(
            (list.count(), list.open_count(), list.struck_count()),
            (5, 4, 1)
        );
        for l in &list.lines {
            assert_eq!(l.reappear, Reappear::Unset);
            assert_eq!(ann.current(&l.id, keys::REAPPEAR)?.as_deref(), Some("none"));
            assert_eq!(ann.current(&l.id, checklist_keys::INDENT)?, None);
        }
        assert!(lists.is_shopping_list(&note.id)?);
        assert!(Checklists::new(&ann).by_note(&note.id)?.is_none());
        assert!(!lists.may_make(&note.id)?);
        assert!(lists.make(&note.id).is_err());
        Ok(())
    }

    #[test]
    fn from_a_formatted_text_the_marks_of_items_and_quotes_go() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = Notes::new(&ann).capture_as(
            "**Groceries**\n- Milk\n  * Eggs\n+ ✓ Bread\n12. Tea\n> Apples",
            text_format::MARKDOWN,
            None,
            None,
        )?;
        let lists = ShoppingLists::new(&ann);
        lists.make(&note.id)?;
        let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
        assert_eq!(made.text, "**Groceries**");
        assert_eq!(
            shown(&ann, &note.id)?,
            ["Milk", "Eggs", "Tea", "Apples", "~Bread"]
        );
        // A plain text keeps them as written.
        let plain = Notes::new(&ann).capture("Shop\n- Milk", None, None)?.id;
        lists.make(&plain)?;
        assert_eq!(shown(&ann, &plain)?, ["- Milk"]);
        Ok(())
    }

    #[test]
    fn a_line_came_back_on_the_day_its_check_runs_out() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let lists = ShoppingLists::new(&ann);
        let week = shopping(&ann, "Groceries\nMilk\nEggs\nTea\nRice", None)?;
        weekly(&ann, &week)?;
        let hardware = shopping(&ann, "Hardware\nNails\nGlue", None)?;
        let gone = shopping(&ann, "Gone\nSalt", None)?;
        for text in ["Milk", "Eggs", "Tea"] {
            lists.check(&id_of(&ann, &week, text)?, &wall)?;
        }
        lists.uncheck(&id_of(&ann, &week, "Eggs")?, &wall)?;
        lists.add(&week, "tea", &wall)?;
        lists.set_reappear(
            &id_of(&ann, &hardware, "Nails")?,
            Reappear::After(2, ReappearUnit::Week),
        )?;
        lists.check(&id_of(&ann, &hardware, "Nails")?, &wall)?;
        lists.check(&id_of(&ann, &gone, "Salt")?, &wall)?;
        Notes::new(&ann).trash(&gone)?;
        assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, None);
        // What came back on `day`, by list, as "list: Line, Line".
        let came_back = |day: Day| -> Made<Vec<String>> {
            let mut out = Vec::new();
            for note in [&week, &hardware] {
                let list = lists.by_note(note, &wall)?.ok_or("a Shopping List")?;
                let texts: Vec<&str> = list
                    .came_back(day)
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect();
                if !texts.is_empty() {
                    out.push(format!("{note}: {}", texts.join(", ")));
                }
            }
            Ok(out)
        };
        assert!(came_back(Day::new(2026, 3, 11))?.is_empty());
        // Midnight on the wall: Thursday the 12th begins.
        clock.set(utc(11, 23, 0, 0)?);
        let back = Day::new(2026, 3, 12);
        assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, Some(back));
        for text in ["Eggs", "Tea", "Rice"] {
            assert_eq!(line_of(&ann, &week, text)?.came_back_on, None, "{text}");
        }
        assert_eq!(came_back(back)?, [format!("{week}: Milk")]);
        // The day after it is still open, but no longer back today.
        clock.set(utc(12, 23, 0, 0)?);
        assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, Some(back));
        assert!(came_back(Day::new(2026, 3, 13))?.is_empty());
        Ok(())
    }

    #[test]
    fn a_note_is_at_most_one_of_a_tally_a_checklist_and_a_shopping_list() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let lists = ShoppingLists::new(&ann);
        let gym = Notes::new(&ann).capture("Gym\nPush-ups", None, None)?.id;
        Tallies::new(&ann).add_field(&gym, "Push-ups", "", None, None)?;
        assert!(!lists.may_make(&gym)?);
        assert!(lists.make(&gym).is_err());
        let packing = Notes::new(&ann).capture("Packing\nSocks", None, None)?.id;
        Checklists::new(&ann).make(&packing)?;
        assert!(!lists.may_make(&packing)?);
        assert!(lists.by_note(&packing, &wall)?.is_none());
        let shop = shopping(&ann, "Shop\nMilk", None)?;
        assert!(!Checklists::new(&ann).may_make(&shop)?);
        assert!(Checklists::new(&ann).make(&shop).is_err());
        assert!(Checklists::new(&ann).by_note(&shop)?.is_none());
        assert!(
            Checklists::new(&ann)
                .add(&shop, "Tea", None, false)
                .is_err()
        );
        assert!(
            Tallies::new(&ann)
                .add_field(&shop, "Milk", "", None, None)
                .is_err()
        );
        // Made one on two Devices apart: the Tally wins.
        let both = shopping(&ann, "Both\nMilk", None)?;
        let field = |key| crate::tallies::keys::field("f1", key);
        ann.append(
            &both,
            &field(crate::tallies::keys::NAME),
            Some("Milk"),
            None,
        )?;
        ann.append(&both, &field(crate::tallies::keys::UNIT), Some("l"), None)?;
        assert!(Tallies::new(&ann).is_tally(&both)?);
        assert!(lists.by_note(&both, &wall)?.is_none());
        assert!(!lists.is_shopping_list(&both)?);
        Ok(())
    }

    #[test]
    fn kinds_and_reappear_times_no_app_writes_are_read_without_fail() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = Notes::new(&ann).capture("Odd\nOne", None, None)?.id;
        for kind in ["shopping", "shopping:", "shopping:a:b"] {
            ann.append(&note, checklist_keys::KIND, Some(kind), None)?;
            assert!(
                ShoppingLists::new(&ann).by_note(&note, &wall)?.is_none(),
                "{kind}"
            );
        }
        ann.append(&note, checklist_keys::KIND, None, None)?;
        ShoppingLists::new(&ann).make(&note)?;
        let one = id_of(&ann, &note, "One")?;
        for code in [Some("0"), Some("5"), Some("x"), Some(""), Some("1W"), None] {
            ann.append(&one, keys::REAPPEAR, code, None)?;
            assert_eq!(
                line_of(&ann, &note, "One")?.reappear,
                Reappear::After(1, ReappearUnit::Week),
                "{code:?}"
            );
        }
        assert_eq!(Reappear::read(Some("once")), Reappear::Once);
        assert_eq!(
            Reappear::read(Some("4w")),
            Reappear::After(4, ReappearUnit::Week)
        );
        Ok(())
    }

    fn at(year: i32, month: u32, day: u32, hour: u32) -> Made<DateTime<Utc>> {
        Ok(Utc
            .with_ymd_and_hms(year, month, day, hour, 0, 0)
            .single()
            .ok_or("a moment")?)
    }

    fn after(count: u32, unit: ReappearUnit) -> Made<Reappear> {
        Ok(Reappear::after(count, unit).ok_or("a count")?)
    }

    #[test]
    fn a_new_line_stays_struck_until_a_time_is_set_which_counts_from_its_check() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs", None)?;
        let lists = ShoppingLists::new(&ann);
        let milk = id_of(&ann, &note, "Milk")?;
        assert_eq!(line_of(&ann, &note, "Milk")?.reappear, Reappear::Unset);
        lists.check(&milk, &wall)?;
        assert!(line_of(&ann, &note, "Milk")?.struck);
        assert_eq!(line_of(&ann, &note, "Milk")?.back_on, None);
        // A year on, still struck.
        clock.set(at(2027, 3, 5, 10)?);
        assert_eq!(shown(&ann, &note)?, ["Eggs", "~Milk"]);
        // Ten days after the check, a week makes it back since the 12th.
        clock.set(at(2026, 3, 15, 10)?);
        lists.set_reappear(&milk, after(1, ReappearUnit::Week)?)?;
        assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs"]);
        assert_eq!(
            line_of(&ann, &note, "Milk")?.came_back_on,
            Some(Day::new(2026, 3, 12))
        );
        // A month instead: struck again until the 5th of April.
        lists.set_reappear(&milk, after(1, ReappearUnit::Month)?)?;
        assert_eq!(shown(&ann, &note)?, ["Eggs", "~Milk"]);
        assert_eq!(
            line_of(&ann, &note, "Milk")?.back_on,
            Some(Day::new(2026, 4, 5))
        );
        lists.set_reappear(&milk, Reappear::Unset)?;
        assert_eq!(ann.current(&milk, keys::REAPPEAR)?.as_deref(), Some("none"));
        assert_eq!(line_of(&ann, &note, "Milk")?.back_on, None);
        Ok(())
    }

    #[test]
    fn days_and_weeks_count_on_and_months_and_years_go_by_the_calendar() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        // Saturday the 31st of January on the wall.
        clock.set(at(2026, 1, 31, 10)?);
        let note = shopping(&ann, "Shop\nA\nB\nC\nD\nE\nF", None)?;
        let lists = ShoppingLists::new(&ann);
        let times = [
            ("A", after(1, ReappearUnit::Day)?, Day::new(2026, 2, 1)),
            ("B", after(6, ReappearUnit::Week)?, Day::new(2026, 3, 14)),
            ("C", after(1, ReappearUnit::Month)?, Day::new(2026, 2, 28)),
            ("D", after(13, ReappearUnit::Month)?, Day::new(2027, 2, 28)),
            ("E", after(1, ReappearUnit::Year)?, Day::new(2027, 1, 31)),
            ("F", after(99, ReappearUnit::Day)?, Day::new(2026, 5, 10)),
        ];
        for (text, time, _) in times {
            lists.set_reappear(&id_of(&ann, &note, text)?, time)?;
            lists.check(&id_of(&ann, &note, text)?, &wall)?;
        }
        for (text, _, back) in times {
            assert_eq!(line_of(&ann, &note, text)?.back_on, Some(back), "{text}");
        }
        // At the start of the 28th of February on the wall, C is back.
        clock.set(at(2026, 2, 27, 23)?);
        assert!(!line_of(&ann, &note, "C")?.struck);
        assert_eq!(
            line_of(&ann, &note, "C")?.came_back_on,
            Some(Day::new(2026, 2, 28))
        );
        assert!(line_of(&ann, &note, "B")?.struck);
        // A leap day a year on is the 28th, four years on the 29th.
        let leap = Day::new(2028, 2, 29);
        let back =
            |count, unit, day| -> Made<Option<Day>> { Ok(after(count, unit)?.back_from(day)) };
        assert_eq!(
            back(1, ReappearUnit::Year, leap)?,
            Some(Day::new(2029, 2, 28))
        );
        assert_eq!(
            back(4, ReappearUnit::Year, leap)?,
            Some(Day::new(2032, 2, 29))
        );
        assert_eq!(
            back(1, ReappearUnit::Month, leap)?,
            Some(Day::new(2028, 3, 29))
        );
        assert_eq!(
            back(11, ReappearUnit::Month, Day::new(2026, 3, 31))?,
            Some(Day::new(2027, 2, 28))
        );
        assert_eq!(Reappear::Unset.back_from(leap), None);
        assert_eq!(Reappear::Once.back_from(leap), None);
        Ok(())
    }

    #[test]
    fn the_week_values_of_before_are_kept_and_reading_writes_nothing() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
        let lists = ShoppingLists::new(&ann);
        // As a version before #195 wrote them.
        for (text, code) in [("Milk", Some("1w")), ("Eggs", Some("4w")), ("Tea", None)] {
            ann.append(&id_of(&ann, &note, text)?, keys::REAPPEAR, code, None)?;
        }
        for text in ["Milk", "Eggs", "Tea"] {
            lists.check(&id_of(&ann, &note, text)?, &wall)?;
        }
        let before = ann.all_entries()?.len();
        assert_eq!(
            line_of(&ann, &note, "Milk")?.reappear,
            after(1, ReappearUnit::Week)?
        );
        assert_eq!(
            line_of(&ann, &note, "Eggs")?.reappear,
            after(4, ReappearUnit::Week)?
        );
        assert_eq!(line_of(&ann, &note, "Tea")?.reappear, Reappear::FALLBACK);
        assert_eq!(
            line_of(&ann, &note, "Milk")?.back_on,
            Some(Day::new(2026, 3, 12))
        );
        assert_eq!(
            line_of(&ann, &note, "Eggs")?.back_on,
            Some(Day::new(2026, 4, 2))
        );
        assert_eq!(
            line_of(&ann, &note, "Tea")?.back_on,
            Some(Day::new(2026, 3, 12))
        );
        assert_eq!(ann.all_entries()?.len(), before);
        let eggs = id_of(&ann, &note, "Eggs")?;
        assert_eq!(ann.current(&eggs, keys::REAPPEAR)?.as_deref(), Some("4w"));
        Ok(())
    }

    #[test]
    fn reappear_codes_as_stored_and_those_no_app_writes_read_as_a_week() -> TestResult {
        let codes = [
            ("none", Reappear::Unset),
            ("once", Reappear::Once),
            ("1d", after(1, ReappearUnit::Day)?),
            ("20w", after(20, ReappearUnit::Week)?),
            ("3m", after(3, ReappearUnit::Month)?),
            ("99y", after(99, ReappearUnit::Year)?),
        ];
        for (code, time) in codes {
            assert_eq!(Reappear::read(Some(code)), time, "{code}");
            assert_eq!(time.code(), code);
        }
        let odd = [
            None,
            Some(""),
            Some("0w"),
            Some("01w"),
            Some("100d"),
            Some("1W"),
            Some(" 1w"),
            Some("1w "),
            Some("w1"),
            Some("1 w"),
            Some("5"),
            Some("1x"),
            Some("٣w"),
            Some("None"),
        ];
        for code in odd {
            assert_eq!(Reappear::read(code), Reappear::FALLBACK, "{code:?}");
        }
        assert_eq!(Reappear::FALLBACK, after(1, ReappearUnit::Week)?);
        assert_eq!(Reappear::after(0, ReappearUnit::Day), None);
        assert_eq!(Reappear::after(100, ReappearUnit::Day), None);
        Ok(())
    }

    #[test]
    fn a_checked_line_is_struck_until_the_start_of_its_day_back_by_the_wall_clock() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
        weekly(&ann, &note)?;
        let lists = ShoppingLists::new(&ann);
        lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
        assert_eq!(shown(&ann, &note)?, ["Eggs", "Tea", "~Milk"]);
        let milk = line_of(&ann, &note, "Milk")?;
        assert_eq!(milk.back_on, Some(Day::new(2026, 3, 12)));
        assert!(milk.checked_at.is_some());
        let before = ann.all_entries()?.len();
        clock.set(utc(11, 22, 59, 59)?);
        assert_eq!(shown(&ann, &note)?, ["Eggs", "Tea", "~Milk"]);
        clock.set(utc(11, 23, 0, 0)?);
        assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea"]);
        assert_eq!(line_of(&ann, &note, "Milk")?.back_on, None);
        assert_eq!(ann.all_entries()?.len(), before, "reading writes none");
        // 22:30 UTC on Wednesday is 00:30 on Thursday two hours ahead.
        let two_ahead = |utc: DateTime<Utc>| (utc + Duration::hours(2)).naive_utc();
        let late = shopping(&ann, "Late\nSalt", None)?;
        weekly(&ann, &late)?;
        clock.set(utc(18, 22, 30, 0)?);
        lists.check(&id_of(&ann, &late, "Salt")?, &two_ahead)?;
        let back = |wall: &WallClock| -> Made<Option<Day>> {
            let list = lists.by_note(&late, wall)?.ok_or("a Shopping List")?;
            Ok(list.lines.first().and_then(|l| l.back_on))
        };
        assert_eq!(back(&two_ahead)?, Some(Day::new(2026, 3, 26)));
        assert_eq!(back(&wall)?, Some(Day::new(2026, 3, 25)));
        clock.set(utc(25, 21, 59, 59)?);
        assert_eq!(back(&two_ahead)?, Some(Day::new(2026, 3, 26)));
        clock.set(utc(25, 22, 0, 0)?);
        assert_eq!(back(&two_ahead)?, None);
        Ok(())
    }

    #[test]
    fn each_reappear_time_and_once_is_gone() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nOnce\nOne\nTwo\nThree\nFour", None)?;
        weekly(&ann, &note)?;
        let lists = ShoppingLists::new(&ann);
        let once = id_of(&ann, &note, "Once")?;
        lists.set_reappear(&once, Reappear::Once)?;
        lists.set_reappear(
            &id_of(&ann, &note, "Two")?,
            Reappear::After(2, ReappearUnit::Week),
        )?;
        lists.set_reappear(
            &id_of(&ann, &note, "Three")?,
            Reappear::After(3, ReappearUnit::Week),
        )?;
        lists.set_reappear(
            &id_of(&ann, &note, "Four")?,
            Reappear::After(4, ReappearUnit::Week),
        )?;
        for text in ["Once", "One", "Two", "Three", "Four"] {
            lists.check(&id_of(&ann, &note, text)?, &wall)?;
        }
        assert_eq!(shown(&ann, &note)?, ["~One", "~Two", "~Three", "~Four"]);
        let list = lists.by_note(&note, &wall)?.ok_or("a Shopping List")?;
        let days: Vec<Option<Day>> = list.lines.iter().map(|l| l.back_on).collect();
        assert_eq!(days, [12, 19, 26, 33].map(|d| Some(Day::new(2026, 3, d))));
        assert_eq!(ann.current(&once, checklist_keys::TRASHED)?, None);
        assert!(lists.check(&once, &wall).is_err());
        clock.set(utc(25, 23, 0, 0)?);
        assert_eq!(shown(&ann, &note)?, ["One", "Two", "Three", "~Four"]);
        Ok(())
    }

    #[test]
    fn the_open_lines_keep_their_order_and_the_struck_one_checked_last_is_at_the_end() -> TestResult
    {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nTea\nRice", None)?;
        weekly(&ann, &note)?;
        let lists = ShoppingLists::new(&ann);
        lists.check(&id_of(&ann, &note, "Eggs")?, &wall)?;
        lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
        lists.check(&id_of(&ann, &note, "Rice")?, &wall)?;
        assert_eq!(shown(&ann, &note)?, ["Tea", "~Eggs", "~Milk", "~Rice"]);
        // Unchecked, it comes back now, in its place.
        lists.uncheck(&id_of(&ann, &note, "Rice")?, &wall)?;
        assert_eq!(shown(&ann, &note)?, ["Tea", "Rice", "~Eggs", "~Milk"]);
        let before = ann.all_entries()?.len();
        lists.check(&id_of(&ann, &note, "Eggs")?, &wall)?;
        lists.uncheck(&id_of(&ann, &note, "Tea")?, &wall)?;
        assert_eq!(ann.all_entries()?.len(), before);
        // Eggs and Milk come back on the same day, each in its place.
        clock.set(utc(11, 23, 0, 0)?);
        assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea", "Rice"]);
        Ok(())
    }

    #[test]
    fn a_line_without_a_valid_key_shows_after_the_others_the_one_added_last_first() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs", None)?;
        let lists = ShoppingLists::new(&ann);
        let making = lists
            .by_note(&note, &wall)?
            .ok_or("a Shopping List")?
            .making;
        // As a modified app would.
        for (text, order) in [("Old", None), ("Odd", Some("é")), ("New", Some(""))] {
            let id = format!("line:{}", ann.new_id()?);
            ann.append(
                &id,
                checklist_keys::TYPE,
                Some(checklist_keys::TYPE_LINE),
                None,
            )?;
            ann.append(&id, checklist_keys::NOTE, Some(&note), None)?;
            ann.append(&id, checklist_keys::MADE, Some(&making), None)?;
            let text = StoredText::plain(text).encode();
            ann.append(&id, checklist_keys::TEXT, Some(&text), None)?;
            if let Some(order) = order {
                ann.append(&id, checklist_keys::ORDER, Some(order), None)?;
            }
        }
        lists.add(&note, "Tea", &wall)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Milk", "Eggs", "Tea", "New", "Odd", "Old"]
        );
        Ok(())
    }

    #[test]
    fn a_new_name_comes_in_where_it_is_told_before_the_struck_ones_after_the_line_above()
    -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nHam\nJam\nTea", None)?;
        weekly(&ann, &note)?;
        let lists = ShoppingLists::new(&ann);
        for text in ["Eggs", "Ham", "Jam"] {
            lists.check(&id_of(&ann, &note, text)?, &wall)?;
        }
        lists.add_at(&note, "Bread", &wall, Some(1))?;
        lists.add_at(&note, "Rice", &wall, Some(0))?;
        lists.add_at(&note, "Salt", &wall, Some(9))?;
        assert_eq!(
            shown(&ann, &note)?,
            [
                "Rice", "Milk", "Bread", "Tea", "Salt", "~Eggs", "~Ham", "~Jam"
            ]
        );
        // A name it has stays where it is.
        lists.add_at(&note, "tea", &wall, Some(0))?;
        clock.set(utc(11, 23, 0, 0)?);
        assert_eq!(
            shown(&ann, &note)?,
            ["Rice", "Milk", "Bread", "Eggs", "Ham", "Jam", "Tea", "Salt"]
        );
        Ok(())
    }

    #[test]
    fn a_name_it_has_adds_nothing_and_brings_a_struck_or_gone_line_back() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nÄpfel\nTea", None)?;
        let lists = ShoppingLists::new(&ann);
        let milk = id_of(&ann, &note, "Milk")?;
        let tea = id_of(&ann, &note, "Tea")?;
        lists.set_reappear(&tea, Reappear::Once)?;
        lists.check(&milk, &wall)?;
        lists.check(&tea, &wall)?;
        assert_eq!(shown(&ann, &note)?, ["Eggs", "Äpfel", "~Milk"]);
        let before = ann.all_entries()?.len();
        assert_eq!(
            lists.add(&note, " eggs ", &wall)?,
            id_of(&ann, &note, "Eggs")?
        );
        assert_eq!(
            lists.add(&note, "äPFEL", &wall)?,
            id_of(&ann, &note, "Äpfel")?
        );
        assert_eq!(ann.all_entries()?.len(), before, "open ones stay");
        assert_eq!(lists.add(&note, "MILK", &wall)?, milk);
        assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Äpfel"]);
        assert_eq!(lists.add(&note, "tea", &wall)?, tea);
        assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Äpfel", "Tea"]);
        assert_eq!(line_of(&ann, &note, "Tea")?.reappear, Reappear::Once);
        // A new name comes in at the end; a deleted one is not brought back.
        let rice = lists.add(&note, "  Rice\n", &wall)?;
        assert_eq!(shown(&ann, &note)?.last().map(String::as_str), Some("Rice"));
        lists.delete(&rice)?;
        assert_ne!(lists.add(&note, "rice", &wall)?, rice);
        assert!(lists.add(&note, "  ", &wall).is_err());
        Ok(())
    }

    #[test]
    fn the_lists_to_add_to_are_those_not_in_the_trash_oldest_first() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let groceries = shopping(&ann, "Groceries", None)?;
        notes.capture("plain", None, None)?;
        let gone = shopping(&ann, "Gone", None)?;
        let hardware = shopping(&ann, "Hardware", None)?;
        let packing = notes.capture("Packing\nSocks", None, None)?.id;
        Checklists::new(&ann).make(&packing)?;
        notes.trash(&gone)?;
        let ids: Vec<String> = ShoppingLists::new(&ann)
            .addable()?
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, [groceries, hardware]);
        Ok(())
    }

    #[test]
    fn the_lines_of_a_text_come_in_at_once_in_their_order_at_the_end() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nTea", None)?;
        let lists = ShoppingLists::new(&ann);
        let milk = id_of(&ann, &note, "Milk")?;
        let tea = id_of(&ann, &note, "Tea")?;
        lists.check(&milk, &wall)?;
        let ids = lists.add_lines(&note, "Eggs\r\n  milk \n\n Tea\rbread", &wall)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Milk", "Tea", "Eggs", "bread"],
            "an open one stays"
        );
        assert_eq!(
            ids,
            [
                id_of(&ann, &note, "Eggs")?,
                milk,
                tea,
                id_of(&ann, &note, "bread")?
            ]
        );
        assert_eq!(ann.entities_of_type(checklist_keys::TYPE_LINE)?.len(), 4);
        assert!(lists.add_lines(&note, " \n ", &wall)?.is_empty());
        let plain = Notes::new(&ann).capture("plain", None, None)?.id;
        assert!(lists.add_lines(&plain, "Milk", &wall).is_err());
        Ok(())
    }

    #[test]
    fn lines_are_renamed_deleted_restored_and_go_back_into_a_plain_note() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
        let lists = ShoppingLists::new(&ann);
        let milk = id_of(&ann, &note, "Milk")?;
        lists.rename(&milk, "Oat milk")?;
        let before = ann.all_entries()?.len();
        lists.rename(&milk, " Oat milk ")?;
        lists.set_reappear(&milk, Reappear::Unset)?;
        assert_eq!(ann.all_entries()?.len(), before);
        lists.delete(&milk)?;
        assert_eq!(shown(&ann, &note)?, ["Eggs", "Tea"]);
        assert!(lists.rename(&milk, "x").is_err());
        lists.restore(&milk)?;
        assert!(lists.restore(&milk).is_err());
        assert_eq!(shown(&ann, &note)?, ["Oat milk", "Eggs", "Tea"]);
        lists.set_reappear(&id_of(&ann, &note, "Tea")?, Reappear::Once)?;
        lists.check(&id_of(&ann, &note, "Tea")?, &wall)?;
        lists.check(&milk, &wall)?;
        // Search finds it by its Lines, not by those gone.
        assert_eq!(Search::new(&ann).notes("MILK", None)?.len(), 1);
        assert!(Search::new(&ann).notes("tea", None)?.is_empty());
        lists.to_plain(&note, &wall)?;
        assert!(!lists.is_shopping_list(&note)?);
        let plain = Notes::new(&ann).by_id(&note)?.ok_or("the Note")?;
        assert_eq!(plain.text, "Shop\nEggs\n✓ Oat milk");
        lists.make(&note)?;
        assert_eq!(shown(&ann, &note)?, ["Eggs", "~Oat milk"]);
        Ok(())
    }

    #[test]
    fn every_member_checks_and_adds_and_the_rest_stays_with_the_author_and_the_managers()
    -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        invite(&ann, &ben, &area)?;
        let note = shopping(&ann, "Shop\nMilk\nEggs", Some(&area))?;
        exchange(&ben, &ann)?;
        let lists = ShoppingLists::new(&ben);
        let milk = id_of(&ben, &note, "Milk")?;
        let eggs = id_of(&ben, &note, "Eggs")?;
        assert!(lists.may_check(&note)?);
        lists.check(&milk, &wall)?;
        lists.check(&eggs, &wall)?;
        lists.add(&note, "milk", &wall)?;
        let bread = lists.add(&note, "Bread", &wall)?;
        exchange(&ben, &ann)?;
        assert_eq!(shown(&ann, &note)?, ["Milk", "Bread", "~Eggs"]);
        assert!(lists.may_edit(&bread)?);
        assert!(!lists.may_edit(&milk)?);
        assert!(lists.rename(&milk, "Beer").is_err());
        assert!(lists.delete(&milk).is_err());
        assert!(lists.set_reappear(&milk, Reappear::Once).is_err());
        assert!(lists.to_plain(&note, &wall).is_err());
        // As a modified app would.
        let forged = [
            ben.append(&milk, keys::REAPPEAR, Some("once"), None)?,
            ben.append(
                &eggs,
                checklist_keys::TRASHED,
                Some("2026-03-05T11:00:00.000Z"),
                None,
            )?,
        ];
        exchange(&ben, &ann)?;
        let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
        assert!(forged.iter().all(|e| held.contains(&e.id())));
        assert_eq!(line_of(&ann, &note, "Milk")?.reappear, Reappear::Unset);
        assert_eq!(shown(&ann, &note)?, ["Milk", "Bread", "~Eggs"]);
        let grants = ann.grants_of(&area)?.ok_or("the grants")?;
        let person = grants
            .person_of(&ben.device_id()?)
            .ok_or("Ben")?
            .to_string();
        ann.set_role(&area, &person, AreaRole::Manager)?;
        exchange(&ben, &ann)?;
        lists.set_reappear(&milk, Reappear::After(2, ReappearUnit::Week))?;
        exchange(&ben, &ann)?;
        assert_eq!(
            line_of(&ann, &note, "Milk")?.reappear,
            Reappear::After(2, ReappearUnit::Week)
        );
        Ok(())
    }

    #[test]
    fn two_renames_of_one_line_are_a_conflict_and_a_later_check_wins() -> TestResult {
        let clock = Clock::new()?;
        let phone = clock.store(1)?;
        let desk = clock.store(2)?;
        let note = shopping(&phone, "Shop\nMilk\nEggs", None)?;
        send(&phone, &desk)?;
        let milk = id_of(&phone, &note, "Milk")?;
        let eggs = id_of(&phone, &note, "Eggs")?;
        ShoppingLists::new(&phone).rename(&milk, "Oat milk")?;
        ShoppingLists::new(&desk).rename(&milk, "Soy milk")?;
        ShoppingLists::new(&desk).check(&eggs, &wall)?;
        ShoppingLists::new(&phone).check(&eggs, &wall)?;
        ShoppingLists::new(&phone).uncheck(&eggs, &wall)?;
        send(&phone, &desk)?;
        send(&desk, &phone)?;
        let conflicts = desk.conflicts()?;
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].entity, milk);
        assert_eq!(conflicts[0].entity_type, checklist_keys::TYPE_LINE);
        assert!(desk.may_resolve(&milk, &conflicts[0].entity_type)?);
        // Back in its place.
        assert_eq!(
            shown(&desk, &note)?.last().map(String::as_str),
            Some("Eggs")
        );
        assert_eq!(shown(&phone, &note)?, shown(&desk, &note)?);
        Ok(())
    }

    #[test]
    fn a_routine_spawned_from_a_shopping_list_is_done_only_when_checked_off() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk", None)?;
        let routines = Routines::new(&ann);
        let schedule = Schedule::new(Day::new(2026, 3, 5), &[], None).ok_or("a schedule")?;
        let routine = routines.spawn(&note, &schedule)?.id;
        let day = Occurrence::new(Day::new(2026, 3, 5), None);
        let milk = id_of(&ann, &note, "Milk")?;
        ShoppingLists::new(&ann).check(&milk, &wall)?;
        // As a modified app would.
        ann.append(
            &routine,
            &checklist_keys::check_for(&day, &milk),
            Some("2026-03-05T10:30:00.000Z"),
            None,
        )?;
        let done = |o: &Occurrence| -> Made<Option<DateTime<Utc>>> {
            Ok(routines.by_id(&routine)?.ok_or("the Routine")?.done_at(o))
        };
        assert_eq!(done(&day)?, None);
        let on = crate::RoutineOccurrence {
            routine_id: routine.clone(),
            occurrence: day,
        };
        assert!(Checklists::new(&ann).for_occurrence(&on)?.is_none());
        routines.mark_done(&routine, &day, None)?;
        assert!(done(&day)?.is_some());
        Ok(())
    }

    #[test]
    fn a_shopping_list_made_of_a_received_checklist_stays_one() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let note = Notes::new(&ann).capture("Shop\nMilk", None, None)?.id;
        Checklists::new(&ann).make(&note)?;
        let item = ann.sent_item_of(&note, false)?;
        let got = ben.accept_sent_item(&item, false)?.ok_or("accepted")?;
        Checklists::new(&ben).to_plain(&got)?;
        ShoppingLists::new(&ben).make(&got)?;
        Checklists::new(&ann).add(&note, "Tea", None, false)?;
        let key = ann.signing_key()?;
        let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
        let item = ann.sent_item_of(&note, false)?;
        let file = crate::sent::encode(&item, &key, &[], &mut random)?;
        let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
        let changes = opened.changes.ok_or("an update")?;
        assert!(!changes.makes_checklist);
        assert!(changes.lines_added.is_empty());
        ben.accept_sent_item(&opened.item, false)?;
        assert_eq!(shown(&ben, &got)?, ["Milk"]);
        assert!(Checklists::new(&ben).by_note(&got)?.is_none());
        Ok(())
    }

    /// What `to` opens of what `from` sends of `note`.
    fn sent(from: &Store, to: &Store, note: &str) -> Made<crate::sent::Opened> {
        let item = from.sent_item_of(note, false)?;
        let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
        let file = crate::sent::encode(&item, &from.signing_key()?, &[], &mut random)?;
        let json: serde_json::Value = serde_json::from_slice(&file)?;
        assert_eq!(json["format"], crate::sent::LIST_FORMAT);
        Ok(to.open_sent_item(&file)?.map_err(|_| "a Sent Item")?)
    }

    #[test]
    fn a_sent_shopping_list_carries_its_lines_and_they_arrive_open() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let note = shopping(&ann, "Shop\nMilk\nBread\nEggs", None)?;
        let lists = ShoppingLists::new(&ann);
        lists.set_reappear(
            &id_of(&ann, &note, "Eggs")?,
            Reappear::After(2, ReappearUnit::Week),
        )?;
        lists.set_reappear(&id_of(&ann, &note, "Bread")?, Reappear::Once)?;
        lists.check(&id_of(&ann, &note, "Bread")?, &wall)?;
        lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
        let opened = sent(&ann, &ben, &note)?;
        assert!(opened.item.shopping);
        let carried: Vec<String> = opened
            .item
            .lines
            .iter()
            .flatten()
            .map(|l| {
                format!(
                    "{} {}",
                    l.text,
                    l.reappear.map(Reappear::code).unwrap_or_default()
                )
            })
            .collect();
        assert_eq!(carried, ["Eggs 2w", "Milk none"]);
        let got = ben
            .accept_sent_item(&opened.item, false)?
            .ok_or("accepted")?;
        assert_eq!(shown(&ben, &got)?, ["Eggs", "Milk"]);
        assert_eq!(
            line_of(&ben, &got, "Eggs")?.reappear,
            Reappear::After(2, ReappearUnit::Week)
        );
        assert!(Checklists::new(&ben).by_note(&got)?.is_none());
        assert!(Notes::new(&ben).by_id(&got)?.ok_or("note")?.is_unsorted());
        Ok(())
    }

    #[test]
    fn an_update_changes_the_senders_lines_and_keeps_the_receivers() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs", None)?;
        let got = ben
            .accept_sent_item(&sent(&ann, &ben, &note)?.item, false)?
            .ok_or("accepted")?;
        let his = ShoppingLists::new(&ben);
        his.check(&id_of(&ben, &got, "Eggs")?, &wall)?;
        his.add(&got, "Coffee", &wall)?;
        let hers = ShoppingLists::new(&ann);
        hers.rename(&id_of(&ann, &note, "Eggs")?, "Free-range eggs")?;
        hers.delete(&id_of(&ann, &note, "Milk")?)?;
        hers.add(&note, "Tea", &wall)?;
        let opened = sent(&ann, &ben, &note)?;
        let changes = opened.changes.as_ref().ok_or("an update")?;
        assert!(!changes.makes_shopping_list);
        let added: Vec<&str> = changes
            .lines_added
            .iter()
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(added, ["Tea"]);
        assert_eq!(changes.lines_renamed.len(), 1);
        assert_eq!(changes.lines_renamed[0].after, "Free-range eggs");
        let removed: Vec<&str> = changes
            .lines_removed
            .iter()
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(removed, ["Milk"]);
        ben.accept_sent_item(&opened.item, false)?;
        assert_eq!(shown(&ben, &got)?, ["Coffee", "Tea", "~Free-range eggs"]);
        Ok(())
    }

    #[test]
    fn a_line_gone_here_is_not_added_again() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let note = shopping(&ann, "Shop\nMilk", None)?;
        let got = ben
            .accept_sent_item(&sent(&ann, &ben, &note)?.item, false)?
            .ok_or("accepted")?;
        let milk = id_of(&ben, &got, "Milk")?;
        ShoppingLists::new(&ben).set_reappear(&milk, Reappear::Once)?;
        ShoppingLists::new(&ben).check(&milk, &wall)?;
        let opened = sent(&ann, &ben, &note)?;
        assert!(opened.changes.as_ref().ok_or("an update")?.is_empty());
        Ok(())
    }

    #[test]
    fn a_note_follows_the_sender_into_a_shopping_list_and_out() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let note = Notes::new(&ann).capture("Shop\nMilk", None, None)?.id;
        let got = ben
            .accept_sent_item(&sent_plain(&ann, &ben, &note)?, false)?
            .ok_or("accepted")?;
        ShoppingLists::new(&ann).make(&note)?;
        let made = sent(&ann, &ben, &note)?;
        let changes = made.changes.as_ref().ok_or("an update")?;
        assert!(changes.makes_shopping_list && !changes.makes_checklist);
        ben.accept_sent_item(&made.item, false)?;
        assert_eq!(shown(&ben, &got)?, ["Milk"]);
        ShoppingLists::new(&ann).to_plain(&note, &wall)?;
        let plain = ann.sent_item_of(&note, false)?;
        let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
        let file = crate::sent::encode(&plain, &ann.signing_key()?, &[], &mut random)?;
        let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
        assert!(opened.changes.as_ref().ok_or("an update")?.makes_plain);
        ben.accept_sent_item(&opened.item, false)?;
        assert!(!ShoppingLists::new(&ben).is_shopping_list(&got)?);
        Ok(())
    }

    /// What `to` opens of the plain Note `note` that `from` sends.
    fn sent_plain(from: &Store, to: &Store, note: &str) -> Made<crate::sent::SentItem> {
        let item = from.sent_item_of(note, false)?;
        let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
        let file = crate::sent::encode(&item, &from.signing_key()?, &[], &mut random)?;
        Ok(to.open_sent_item(&file)?.map_err(|_| "a Sent Item")?.item)
    }

    #[test]
    fn a_file_naming_another_list_is_no_sent_item() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk", None)?;
        let item = ann.sent_item_of(&note, false)?;
        let payload = item.encode();
        assert!(crate::sent::SentItem::parse(&payload).is_some_and(|i| i.shopping));
        let other = payload.replacen("\"list\":\"shopping\"", "\"list\":\"pantry\"", 1);
        assert_ne!(other, payload);
        assert!(crate::sent::SentItem::parse(&other).is_none());
        Ok(())
    }

    /// Writes how this Device's Agenda showed Shopping Lists as a version
    /// before #186 did, which no app offers any more.
    fn old_setting(store: &Store, way: ShoppingAgenda) -> Made<()> {
        Ok(store.set_local_setting("shoppingAgenda", way.code())?)
    }

    /// Today's Agenda rows of the Shopping Lists of `store`: "title: n
    /// open" for one row with the count, "title: Line" for each open Line.
    fn agenda_rows(store: &Store) -> Made<Vec<String>> {
        let notes = Notes::new(store);
        let mut out = Vec::new();
        for row in ShoppingLists::new(store).agenda_rows(&wall)? {
            let title = notes.by_id(&row.list.note_id)?.ok_or("the Note")?.text;
            match row.shows {
                ShoppingOnAgenda::Count => out.push(format!("{title}: {} open", row.lines.len())),
                ShoppingOnAgenda::Lines => {
                    for line in &row.lines {
                        out.push(format!("{title}: {}", line.text));
                    }
                }
                ShoppingOnAgenda::Off => return Err("an Off row".into()),
            }
        }
        Ok(out)
    }

    /// Every value of a Shopping List's Agenda choice `store` holds, with
    /// the Device that wrote it.
    fn choices(store: &Store) -> Made<Vec<(String, Option<String>)>> {
        Ok(store
            .all_entries()?
            .into_iter()
            .filter(|e| {
                e.entity.starts_with(checklist_keys::AGENDA_PREFIX) && e.field == keys::AGENDA
            })
            .map(|e| (e.device, e.value))
            .collect())
    }

    #[test]
    fn each_list_shows_on_the_agenda_as_chosen_for_it() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let lists = ShoppingLists::new(&ann);
        let week = shopping(&ann, "Groceries\nMilk\nEggs\nTea", None)?;
        let hardware = shopping(&ann, "Hardware\nNails\nGlue", None)?;
        let pharmacy = shopping(&ann, "Pharmacy\nPlasters", None)?;
        lists.check(&id_of(&ann, &week, "Tea")?, &wall)?;
        assert_eq!(lists.on_agenda(&week)?, ShoppingOnAgenda::Count);
        assert!(lists.may_show_on_agenda(&week)?);
        assert_eq!(
            agenda_rows(&ann)?,
            ["Groceries: 2 open", "Hardware: 2 open", "Pharmacy: 1 open"]
        );
        lists.set_on_agenda(&hardware, ShoppingOnAgenda::Lines)?;
        lists.set_on_agenda(&pharmacy, ShoppingOnAgenda::Off)?;
        assert_eq!(lists.on_agenda(&pharmacy)?, ShoppingOnAgenda::Off);
        assert_eq!(
            agenda_rows(&ann)?,
            ["Groceries: 2 open", "Hardware: Nails", "Hardware: Glue"]
        );
        let count = ann.all_entries()?.len();
        lists.set_on_agenda(&pharmacy, ShoppingOnAgenda::Off)?;
        assert_eq!(ann.all_entries()?.len(), count, "the same again");
        // Chosen as the default shows it, it is written all the same.
        lists.set_on_agenda(&week, ShoppingOnAgenda::Count)?;
        let values: Vec<Option<String>> = choices(&ann)?.into_iter().map(|(_, v)| v).collect();
        assert_eq!(
            values,
            [
                Some("lines".into()),
                Some("off".into()),
                Some("count".into())
            ]
        );
        let count = ann.all_entries()?.len();
        lists.agenda_rows(&wall)?;
        lists.on_agenda(&week)?;
        assert_eq!(ann.all_entries()?.len(), count, "reading writes nothing");
        Ok(())
    }

    #[test]
    fn a_list_with_no_open_line_never_shows() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let lists = ShoppingLists::new(&ann);
        let week = shopping(&ann, "Groceries\nMilk\nEggs", None)?;
        let hardware = shopping(&ann, "Hardware\nNails", None)?;
        weekly(&ann, &week)?;
        weekly(&ann, &hardware)?;
        shopping(&ann, "Title only", None)?;
        lists.set_on_agenda(&week, ShoppingOnAgenda::Lines)?;
        lists.set_reappear(&id_of(&ann, &week, "Eggs")?, Reappear::Once)?;
        for text in ["Milk", "Eggs"] {
            lists.check(&id_of(&ann, &week, text)?, &wall)?;
        }
        lists.check(&id_of(&ann, &hardware, "Nails")?, &wall)?;
        assert!(agenda_rows(&ann)?.is_empty());
        // A week on, Milk and Nails are back, Eggs gone Once.
        clock.set(utc(12, 9, 0, 0)?);
        assert_eq!(agenda_rows(&ann)?, ["Groceries: Milk", "Hardware: 1 open"]);
        Ok(())
    }

    #[test]
    fn a_list_nobody_chose_for_shows_as_the_device_showed_shopping_lists() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let lists = ShoppingLists::new(&ann);
        let week = shopping(&ann, "Groceries\nMilk", None)?;
        let chosen = shopping(&ann, "Hardware\nNails", None)?;
        lists.set_on_agenda(&chosen, ShoppingOnAgenda::Count)?;
        for (old, shows) in [
            (ShoppingAgenda::Off, ShoppingOnAgenda::Off),
            (ShoppingAgenda::Lines, ShoppingOnAgenda::Lines),
            (ShoppingAgenda::Back, ShoppingOnAgenda::Count),
            (ShoppingAgenda::Open, ShoppingOnAgenda::Count),
        ] {
            old_setting(&ann, old)?;
            assert_eq!(lists.on_agenda(&week)?, shows, "{old:?}");
            assert_eq!(
                lists.on_agenda(&chosen)?,
                ShoppingOnAgenda::Count,
                "{old:?}"
            );
        }
        old_setting(&ann, ShoppingAgenda::Off)?;
        assert_eq!(agenda_rows(&ann)?, ["Hardware: 1 open"]);
        ann.set_local_setting("shoppingAgenda", "sideways")?;
        assert_eq!(lists.on_agenda(&week)?, ShoppingOnAgenda::Count);
        Ok(())
    }

    #[test]
    fn an_agenda_choice_no_app_writes_reads_as_one_row_with_the_count() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        old_setting(&ann, ShoppingAgenda::Off)?;
        let week = shopping(&ann, "Groceries\nMilk", None)?;
        let entity = format!("{}{week}", checklist_keys::AGENDA_PREFIX);
        for value in ["OFF", "off ", "later", ""] {
            ann.append(&entity, keys::AGENDA, Some(value), None)?;
            assert_eq!(
                ShoppingLists::new(&ann).on_agenda(&week)?,
                ShoppingOnAgenda::Count,
                "{value}"
            );
        }
        // Null is no choice: the default again.
        ann.append(&entity, keys::AGENDA, None, None)?;
        assert_eq!(
            ShoppingLists::new(&ann).on_agenda(&week)?,
            ShoppingOnAgenda::Off
        );
        Ok(())
    }

    #[test]
    fn a_checklists_switch_is_not_a_shopping_lists_choice() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = Notes::new(&ann).capture("Groceries\nMilk", None, None)?.id;
        let checklists = Checklists::new(&ann);
        checklists.make(&note)?;
        checklists.set_on_agenda(&note, ListOnAgenda::Off)?;
        checklists.to_plain(&note)?;
        let lists = ShoppingLists::new(&ann);
        lists.make(&note)?;
        assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Count);
        assert_eq!(agenda_rows(&ann)?, ["Groceries: 1 open"]);
        lists.set_on_agenda(&note, ShoppingOnAgenda::Off)?;
        lists.to_plain(&note, &wall)?;
        checklists.make(&note)?;
        checklists.set_on_agenda(&note, ListOnAgenda::Lines)?;
        assert_eq!(checklists.agenda_rows()?.len(), 1);
        Ok(())
    }

    #[test]
    fn only_shopping_lists_take_an_agenda_choice() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let lists = ShoppingLists::new(&ann);
        let plain = notes.capture("Plain\nline", None, None)?.id;
        let trip = notes.capture("Trip\nTent", None, None)?.id;
        Checklists::new(&ann).make(&trip)?;
        let trashed = shopping(&ann, "Trashed\nOne", None)?;
        notes.trash(&trashed)?;
        let merged = shopping(&ann, "Merged\nOne", None)?;
        let into = notes.capture("Into", None, None)?.id;
        notes.merge(&merged, &into)?;
        let errands = shopping(&ann, "Errands\nStamps", None)?;
        Routines::new(&ann).spawn(
            &errands,
            &Schedule::new(Day::new(2026, 3, 5), &[], None).ok_or("a schedule")?,
        )?;
        for id in [&plain, &trip, &trashed, &merged, &into] {
            assert!(!lists.may_show_on_agenda(id)?, "{id}");
        }
        assert!(lists.may_show_on_agenda(&errands)?);
        assert_eq!(agenda_rows(&ann)?, ["Errands: 1 open"]);
        assert!(lists.set_on_agenda(&plain, ShoppingOnAgenda::Off).is_err());
        assert!(lists.set_on_agenda(&trip, ShoppingOnAgenda::Off).is_err());
        assert!(choices(&ann)?.is_empty());
        Ok(())
    }

    #[test]
    fn in_a_shared_area_each_person_chooses_for_their_own_agenda() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        invite(&ann, &ben, &area)?;
        let note = shopping(&ann, "Shop\nMilk\nEggs", Some(&area))?;
        exchange(&ben, &ann)?;
        // Ben, a Member, may choose for a list he did not write.
        ShoppingLists::new(&ben).set_on_agenda(&note, ShoppingOnAgenda::Lines)?;
        ShoppingLists::new(&ann).set_on_agenda(&note, ShoppingOnAgenda::Off)?;
        exchange(&ben, &ann)?;
        exchange(&ben, &ann)?;
        assert!(agenda_rows(&ann)?.is_empty());
        assert_eq!(agenda_rows(&ben)?, ["Shop: Milk", "Shop: Eggs"]);
        assert_eq!(
            choices(&ann)?,
            [(ann.device_id()?, Some("off".to_string()))]
        );
        assert_eq!(
            choices(&ben)?,
            [(ben.device_id()?, Some("lines".to_string()))]
        );
        Ok(())
    }

    #[test]
    fn ones_own_devices_carry_the_agenda_choice() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let desk = clock.device(3, "Ann at the desk")?;
        own_exchange(&desk, &ann)?;
        let note = shopping(&ann, "Shop\nMilk", None)?;
        own_exchange(&desk, &ann)?;
        // The desk's old setting showed nothing; her choice shows it there.
        old_setting(&desk, ShoppingAgenda::Off)?;
        assert!(agenda_rows(&desk)?.is_empty());
        ShoppingLists::new(&ann).set_on_agenda(&note, ShoppingOnAgenda::Lines)?;
        own_exchange(&desk, &ann)?;
        assert_eq!(agenda_rows(&desk)?, ["Shop: Milk"]);
        ShoppingLists::new(&desk).set_on_agenda(&note, ShoppingOnAgenda::Count)?;
        send(&desk, &ann)?;
        assert_eq!(agenda_rows(&ann)?, ["Shop: 1 open"]);
        Ok(())
    }

    #[test]
    fn an_agenda_choice_another_persons_device_wrote_counts_for_nothing() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let ben = clock.device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        invite(&ann, &ben, &area)?;
        let note = shopping(&ann, "Shop\nMilk", Some(&area))?;
        exchange(&ben, &ann)?;
        // As a modified app would: Ben takes the list off Ann's Agenda.
        let forged = ben.append(
            &format!("{}{note}", checklist_keys::AGENDA_PREFIX),
            keys::AGENDA,
            Some("off"),
            None,
        )?;
        ann.apply_entries(
            &[forged.wire()],
            &[],
            Some(&ben.device_id()?),
            &mut crate::signing::ImportReport::default(),
        )?;
        assert_eq!(
            choices(&ann)?,
            [(ben.device_id()?, Some("off".to_string()))]
        );
        let lists = ShoppingLists::new(&ann);
        assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Count);
        assert_eq!(agenda_rows(&ann)?, ["Shop: 1 open"]);
        // Nor does it count as her own choice made already.
        lists.set_on_agenda(&note, ShoppingOnAgenda::Off)?;
        assert!(agenda_rows(&ann)?.is_empty());
        Ok(())
    }
}
