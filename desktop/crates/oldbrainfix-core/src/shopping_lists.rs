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
}

/// How long a checked Line of a Shopping List stays struck through before
/// it comes back (CONTEXT.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reappear {
    /// Gone once checked.
    Once,
    Week1,
    Week2,
    Week3,
    Week4,
}

impl Reappear {
    pub const ALL: [Reappear; 5] = [
        Reappear::Once,
        Reappear::Week1,
        Reappear::Week2,
        Reappear::Week3,
        Reappear::Week4,
    ];

    /// What a new Line gets.
    pub const STANDARD: Reappear = Reappear::Week1;

    /// How it is stored on the Line.
    pub fn code(self) -> &'static str {
        match self {
            Reappear::Once => "once",
            Reappear::Week1 => "1w",
            Reappear::Week2 => "2w",
            Reappear::Week3 => "3w",
            Reappear::Week4 => "4w",
        }
    }

    /// The weeks it stays struck; none for [`Reappear::Once`].
    pub fn weeks(self) -> i64 {
        match self {
            Reappear::Once => 0,
            Reappear::Week1 => 1,
            Reappear::Week2 => 2,
            Reappear::Week3 => 3,
            Reappear::Week4 => 4,
        }
    }

    /// The reappear time `code` names; [`Reappear::STANDARD`] for none,
    /// or a code not known here.
    pub fn read(code: Option<&str>) -> Reappear {
        Reappear::ALL
            .into_iter()
            .find(|r| Some(r.code()) == code)
            .unwrap_or(Reappear::STANDARD)
    }
}

/// How the Agenda of a Device shows Shopping Lists (#123), a setting of
/// that Device.
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

/// A Shopping List on today's Agenda with the Lines its row stands for:
/// those that came back today, or for [`ShoppingAgenda::Open`] those open.
#[derive(Debug, Clone, PartialEq)]
pub struct ShoppingAgendaList {
    pub list: ShoppingList,
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
    /// while it is open.
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
    /// Every Line that shows: the open ones first, the one that came last
    /// at the top, then the struck ones, the one checked last at the end.
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
/// moment it sorts by: a struck or gone one by its check, an open one by
/// when it came to the top, on the wall clock.
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
                let day = Day::of(wall(check.date_at)).add_days(7 * reappear.weeks());
                struck = !gone && today < day;
                if struck {
                    back = Some(day);
                }
                if !gone && !struck {
                    came_back = Some(day);
                }
                checked = check.date_at;
                came = day_start(day);
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
    open.sort_by(|a, b| b.came.cmp(&a.came).then_with(|| by_order(&a.line, &b.line)));
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

    /// The Shopping Lists on today's Agenda by `wall` when it shows them
    /// the way `way`: each list not in the trash nor merged with the Lines
    /// that came back today, or for [`ShoppingAgenda::Open`] those open,
    /// where there are any; oldest capture first. For
    /// [`ShoppingAgenda::Lines`] each of those Lines is a row of its own.
    pub fn agenda(&self, way: ShoppingAgenda, wall: &WallClock) -> Result<Vec<ShoppingAgendaList>> {
        if way == ShoppingAgenda::Off {
            return Ok(Vec::new());
        }
        let notes = Notes::new(self.store);
        let mut shown = Vec::new();
        for id in self.store.entities_of_type(note_keys::TYPE_NOTE)? {
            if self.making(&id)?.is_none() {
                continue;
            }
            if let Some(note) = notes.by_id(&id)?
                && !note.is_trashed()
                && !note.is_merged()
            {
                shown.push(note);
            }
        }
        shown.sort_by(|a, b| {
            a.captured
                .cmp(&b.captured)
                .then_with(|| utf16_cmp(&a.id, &b.id))
        });
        let ids: Vec<String> = shown.iter().map(|n| n.id.clone()).collect();
        let mut lists = self.of_notes(&ids, wall)?;
        let today = Day::today(self.store.now(), wall);
        let mut out = Vec::new();
        for id in &ids {
            let Some(list) = lists.remove(id) else {
                continue;
            };
            let lines: Vec<ShoppingLine> = if way == ShoppingAgenda::Open {
                list.open().into_iter().cloned().collect()
            } else {
                list.came_back(today).into_iter().cloned().collect()
            };
            if !lines.is_empty() {
                out.push(ShoppingAgendaList { list, lines });
            }
        }
        Ok(out)
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
    /// Note in the lists, of a text known here, neither a Tally nor a list
    /// already, which it may change.
    pub fn may_make(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed()
            && !note.is_merged()
            && note.text_editable
            && parse_list_kind(
                self.store
                    .current(note_id, checklist_keys::KIND)?
                    .as_deref(),
            )
            .is_none()
            && !Tallies::new(self.store).is_tally(note_id)?
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

    /// Adds `text` to the Shopping List of `note_id`, at the top. A name it
    /// has already, in another case or with spaces around it, adds
    /// nothing: a struck or gone Line of that name comes back now, at the
    /// top, an open one stays. Anyone with a role in its Area may. Returns
    /// the id of the Line added, brought back or found.
    pub fn add(&self, note_id: &str, text: &str, wall: &WallClock) -> Result<String> {
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
        let mut last: Option<&str> = None;
        for line in list.lines.iter().chain(&list.gone) {
            if let Some(key) = line.order.as_deref()
                && order::is_valid(Some(key))
                && last.is_none_or(|last| utf16_cmp(key, last) == Ordering::Greater)
            {
                last = Some(key);
            }
        }
        let key = order::place(last, None);
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
    /// `note_id` as [`ShoppingLists::add`] does, all in one batch, the first
    /// line ending at the top: what a capture sorted into the list becomes
    /// (#124). Returns the ids of the Lines added, brought back or found,
    /// in the order of the lines; none for a blank `text`.
    pub fn add_lines(&self, note_id: &str, text: &str, wall: &WallClock) -> Result<Vec<String>> {
        let rows: Vec<String> = split_lines(text)
            .into_iter()
            .filter(|row| !dart_trim(row).is_empty())
            .collect();
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        self.store.batch(|| {
            let mut ids = rows
                .iter()
                .rev()
                .map(|row| self.add(note_id, row, wall))
                .collect::<Result<Vec<String>>>()?;
            ids.reverse();
            Ok(ids)
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
            .append(line_id, keys::REAPPEAR, Some(reappear.code()), None)
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
    /// until the start of the same weekday as many weeks on as its
    /// reappear time says, by `wall`; a Line Once is gone. A struck one
    /// stays as it is.
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

    /// Brings the struck Line `line_id` back now, at the top. An open one
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
            Some(Reappear::STANDARD.code()),
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
    use crate::checklists::tests::{TestResult, exchange, invite, send};
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
            assert_eq!(l.reappear, Reappear::Week1);
            assert_eq!(ann.current(&l.id, keys::REAPPEAR)?.as_deref(), Some("1w"));
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
    fn a_line_came_back_on_the_day_its_check_runs_out_and_each_way_shows_its_lists() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let lists = ShoppingLists::new(&ann);
        let week = shopping(&ann, "Groceries\nMilk\nEggs\nTea\nRice", None)?;
        let hardware = shopping(&ann, "Hardware\nNails\nGlue", None)?;
        let gone = shopping(&ann, "Gone\nSalt", None)?;
        for text in ["Milk", "Eggs", "Tea"] {
            lists.check(&id_of(&ann, &week, text)?, &wall)?;
        }
        lists.uncheck(&id_of(&ann, &week, "Eggs")?, &wall)?;
        lists.add(&week, "tea", &wall)?;
        lists.set_reappear(&id_of(&ann, &hardware, "Nails")?, Reappear::Week2)?;
        lists.check(&id_of(&ann, &hardware, "Nails")?, &wall)?;
        lists.check(&id_of(&ann, &gone, "Salt")?, &wall)?;
        Notes::new(&ann).trash(&gone)?;
        assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, None);
        let agenda = |way| -> Made<Vec<String>> {
            Ok(lists
                .agenda(way, &wall)?
                .iter()
                .map(|a| {
                    let texts: Vec<&str> = a.lines.iter().map(|l| l.text.as_str()).collect();
                    format!("{}: {}", a.list.note_id, texts.join(", "))
                })
                .collect())
        };
        assert!(agenda(ShoppingAgenda::Lines)?.is_empty());
        assert_eq!(
            agenda(ShoppingAgenda::Open)?,
            [
                format!("{week}: Tea, Eggs, Rice"),
                format!("{hardware}: Glue")
            ]
        );
        // Midnight on the wall: Thursday the 12th begins.
        clock.set(utc(11, 23, 0, 0)?);
        let back = Day::new(2026, 3, 12);
        assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, Some(back));
        for text in ["Eggs", "Tea", "Rice"] {
            assert_eq!(line_of(&ann, &week, text)?.came_back_on, None, "{text}");
        }
        assert_eq!(agenda(ShoppingAgenda::Lines)?, [format!("{week}: Milk")]);
        assert_eq!(
            agenda(ShoppingAgenda::Back)?,
            agenda(ShoppingAgenda::Lines)?
        );
        assert!(agenda(ShoppingAgenda::Off)?.is_empty());
        // The day after it is still open, but no longer back today.
        clock.set(utc(12, 23, 0, 0)?);
        assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, Some(back));
        assert!(agenda(ShoppingAgenda::Back)?.is_empty());
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
                Reappear::Week1,
                "{code:?}"
            );
        }
        assert_eq!(Reappear::read(Some("once")), Reappear::Once);
        assert_eq!(Reappear::read(Some("4w")), Reappear::Week4);
        Ok(())
    }

    #[test]
    fn a_checked_line_is_struck_until_the_start_of_its_day_back_by_the_wall_clock() -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
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
        let lists = ShoppingLists::new(&ann);
        let once = id_of(&ann, &note, "Once")?;
        lists.set_reappear(&once, Reappear::Once)?;
        lists.set_reappear(&id_of(&ann, &note, "Two")?, Reappear::Week2)?;
        lists.set_reappear(&id_of(&ann, &note, "Three")?, Reappear::Week3)?;
        lists.set_reappear(&id_of(&ann, &note, "Four")?, Reappear::Week4)?;
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
        assert_eq!(shown(&ann, &note)?, ["Three", "Two", "One", "~Four"]);
        Ok(())
    }

    #[test]
    fn the_open_line_that_came_last_is_at_the_top_and_the_struck_one_checked_last_at_the_end()
    -> TestResult {
        let clock = Clock::new()?;
        let ann = clock.device(1, "Ann")?;
        let note = shopping(&ann, "Shop\nMilk\nEggs\nTea\nRice", None)?;
        let lists = ShoppingLists::new(&ann);
        lists.check(&id_of(&ann, &note, "Eggs")?, &wall)?;
        lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
        lists.check(&id_of(&ann, &note, "Rice")?, &wall)?;
        assert_eq!(shown(&ann, &note)?, ["Tea", "~Eggs", "~Milk", "~Rice"]);
        lists.uncheck(&id_of(&ann, &note, "Milk")?, &wall)?;
        assert_eq!(shown(&ann, &note)?, ["Milk", "Tea", "~Eggs", "~Rice"]);
        let before = ann.all_entries()?.len();
        lists.check(&id_of(&ann, &note, "Eggs")?, &wall)?;
        lists.uncheck(&id_of(&ann, &note, "Tea")?, &wall)?;
        assert_eq!(ann.all_entries()?.len(), before);
        clock.set(utc(11, 23, 0, 0)?);
        assert_eq!(shown(&ann, &note)?, ["Eggs", "Rice", "Milk", "Tea"]);
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
        assert_eq!(shown(&ann, &note)?, ["Tea", "Milk", "Eggs", "Äpfel"]);
        assert_eq!(line_of(&ann, &note, "Tea")?.reappear, Reappear::Once);
        // A new name comes in at the top; a deleted one is not brought back.
        let rice = lists.add(&note, "  Rice\n", &wall)?;
        assert_eq!(
            shown(&ann, &note)?.first().map(String::as_str),
            Some("Rice")
        );
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
    fn the_lines_of_a_text_come_in_at_once_the_first_at_the_top() -> TestResult {
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
            ["Eggs", "Milk", "bread", "Tea"],
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
        lists.set_reappear(&milk, Reappear::Week1)?;
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
        assert_eq!(shown(&ann, &note)?, ["Bread", "Milk", "~Eggs"]);
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
        assert_eq!(line_of(&ann, &note, "Milk")?.reappear, Reappear::Week1);
        assert_eq!(shown(&ann, &note)?, ["Bread", "Milk", "~Eggs"]);
        let grants = ann.grants_of(&area)?.ok_or("the grants")?;
        let person = grants
            .person_of(&ben.device_id()?)
            .ok_or("Ben")?
            .to_string();
        ann.set_role(&area, &person, AreaRole::Manager)?;
        exchange(&ben, &ann)?;
        lists.set_reappear(&milk, Reappear::Week2)?;
        exchange(&ben, &ann)?;
        assert_eq!(line_of(&ann, &note, "Milk")?.reappear, Reappear::Week2);
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
        assert_eq!(
            shown(&desk, &note)?.first().map(String::as_str),
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
}
