//! Checklists (ADR 0030) as the Dart core's `checklists.dart` reads and
//! writes them: a Note whose Lines are checked off. Which list a Note is
//! lies on the Note, so it is changed as the Note is; its Lines are
//! entities of their own beside it, each with its text, its place,
//! whether it is under the Line above and whether it is checked. Every
//! write appends the same entries, in the same order and read from the
//! clock as often, as the Dart core's. A Routine or Reminder spawned
//! from a Checklist's Note holds the checks of its Lines for each
//! Occurrence, so every new one starts with no Line checked.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};

use crate::entry::{Entry, dart_iso, parse_time};
use crate::notes::{Note, Notes, keys as note_keys};
use crate::routines::{Routine, Routines};
use crate::schedule::{Day, Occurrence, Schedule, WallClock};
use crate::sent::keys as sent_keys;
use crate::store::{Fields, Store, utf16_cmp};
use crate::tallies::Tallies;
use crate::text_format::StoredText;
use crate::transcript_prompts::dart_trim;
use crate::{Error, Result};

/// Field keys of a Checklist.
pub mod keys {
    /// On the Note: the list it is and the id of that making, as
    /// `checklist:<making id>`; absent, or null, for a plain Note.
    pub const KIND: &str = "kind";
    /// The list of a Checklist in [`KIND`].
    pub const CHECKLIST: &str = "checklist";

    /// The value of [`KIND`] for the making `making` of a Checklist.
    pub fn kind_of(making: &str) -> String {
        format!("{CHECKLIST}:{making}")
    }

    pub const TYPE: &str = "$type";
    pub const TYPE_LINE: &str = "line";
    /// The id prefix of a Line.
    pub const LINE_PREFIX: &str = "line:";
    /// The Note the Line belongs to.
    pub const NOTE: &str = "note";
    /// The making of the list the Line belongs to.
    pub const MADE: &str = "made";
    /// The Line's text, as a stored text.
    pub const TEXT: &str = "text";
    /// Its place among the Lines.
    pub const ORDER: &str = "order";
    /// `true` while it is under the Line above.
    pub const INDENT: &str = "indent";
    /// When it was checked.
    pub const CHECKED: &str = "checked";
    /// When it was deleted.
    pub const TRASHED: &str = "trashed";
    /// On a Routine spawned from a Checklist's Note: the prefix of the
    /// check of a Line for an Occurrence, its value when it was checked,
    /// `None` when it was unchecked again.
    pub const CHECK_PREFIX: &str = "check@";

    /// The check of the Line `line_id` for `occurrence`, keyed by the
    /// wall-clock Occurrence as `done@` is.
    pub fn check_for(occurrence: &crate::schedule::Occurrence, line_id: &str) -> String {
        format!("{CHECK_PREFIX}{}@{line_id}", occurrence.key())
    }

    /// What marks a checked Line in a plain Note's text.
    pub const CHECK_MARK: &str = "✓ ";
    /// What a Line of a group starts with in a plain Note's text.
    pub const GROUP_INDENT: &str = "  ";
}

/// The list a Note's `kind` names and the id of its making; `None` for a
/// value that names none.
pub fn parse_list_kind(value: Option<&str>) -> Option<(&str, &str)> {
    let (list, making) = value?.split_once(':')?;
    if list.is_empty() || making.is_empty() || making.contains(':') {
        return None;
    }
    Some((list, making))
}

/// The keys that place Lines, as the Dart core's `LineOrder`: digits of
/// base 62 in ASCII order, compared as text, never ending in `0`.
pub mod order {
    pub const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    fn digit(c: u8) -> Option<usize> {
        DIGITS.iter().position(|d| *d == c)
    }

    fn mid() -> char {
        char::from(DIGITS[DIGITS.len() / 2])
    }

    /// Whether `key` is one this order reads: any other sorts after them.
    pub fn is_valid(key: Option<&str>) -> bool {
        let Some(key) = key else {
            return false;
        };
        let bytes = key.as_bytes();
        !bytes.is_empty()
            && bytes.iter().all(u8::is_ascii_alphanumeric)
            && bytes.last() != Some(&b'0')
    }

    /// A key after the valid key `a`, or the first key for an empty one.
    pub fn after(a: &str) -> String {
        for (i, c) in a.bytes().enumerate() {
            let Some(d) = digit(c) else {
                break;
            };
            if d < DIGITS.len() - 1 {
                return format!("{}{}", &a[..i], char::from(DIGITS[d + 1]));
            }
        }
        format!("{a}{}", mid())
    }

    /// A key between `a` and `b`: `a` valid or empty, `b` valid, `a`
    /// before `b`.
    pub fn between(a: &str, b: &str) -> String {
        if !a.is_ascii() || !b.is_ascii() {
            return after(a);
        }
        let (ab, bb) = (a.as_bytes(), b.as_bytes());
        let mut n = 0;
        while n < bb.len() && ab.get(n).copied().unwrap_or(b'0') == bb[n] {
            n += 1;
        }
        if n >= bb.len() {
            return after(a);
        }
        let prefix = &b[..n];
        let da = match ab.get(n) {
            Some(c) => digit(*c),
            None => Some(0),
        };
        let (Some(da), Some(db)) = (da, digit(bb[n])) else {
            return after(a);
        };
        if db <= da {
            return after(a);
        }
        if db - da > 1 {
            return format!("{prefix}{}", char::from(DIGITS[(da + db) / 2]));
        }
        if bb.len() > n + 1 {
            return b[..n + 1].to_string();
        }
        let rest = if n < ab.len() { &a[n + 1..] } else { "" };
        format!("{prefix}{}{}", char::from(DIGITS[da]), after(rest))
    }

    /// The key for a place after `a` and before `b`, either `None` for
    /// no Line there.
    pub fn place(a: Option<&str>, b: Option<&str>) -> String {
        match (a, b) {
            (None, None) => after(""),
            (None, Some(b)) => between("", b),
            (Some(a), None) => after(a),
            (Some(a), Some(b)) => between(a, b),
        }
    }
}

/// One line of a Checklist that is checked off (CONTEXT.md).
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// Entity id, `line:<id>`.
    pub id: String,
    pub note_id: String,
    pub text: String,
    /// Whether it shows checked: a Line with a group when every Line of
    /// the group is, any other when it was checked last.
    pub checked: bool,
    /// When it was checked, as its own entry says.
    pub checked_at: Option<DateTime<Utc>>,
    /// The Line it shows under, or `None` for one at the top.
    pub parent: Option<String>,
    /// Its place as stored.
    pub order: Option<String>,
}

impl Line {
    /// Whether it shows under a parent, in its group.
    pub fn in_group(&self) -> bool {
        self.parent.is_some()
    }
}

/// An Occurrence of the Routine `routine_id`: one a Checklist is
/// checked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineOccurrence {
    pub routine_id: String,
    pub occurrence: Occurrence,
}

/// A Note whose Lines are checked off (CONTEXT.md).
#[derive(Debug, Clone, PartialEq)]
pub struct Checklist {
    pub note_id: String,
    /// The id of its making, as the Note's `kind` names it.
    pub making: String,
    /// Every Line, in the order they show: each parent followed by its
    /// group.
    pub lines: Vec<Line>,
    /// The Occurrence its Lines show checked for, or `None` when they
    /// show their own checks.
    pub on: Option<RoutineOccurrence>,
}

impl Checklist {
    /// The Line `id`.
    pub fn line(&self, id: &str) -> Option<&Line> {
        self.lines.iter().find(|l| l.id == id)
    }

    /// The group of the Line `id`: the Lines under it, in order.
    pub fn group_of(&self, id: &str) -> Vec<&Line> {
        self.lines
            .iter()
            .filter(|l| l.parent.as_deref() == Some(id))
            .collect()
    }

    /// How many Lines show checked.
    pub fn checked_count(&self) -> usize {
        self.lines.iter().filter(|l| l.checked).count()
    }

    pub fn count(&self) -> usize {
        self.lines.len()
    }

    pub fn all_checked(&self) -> bool {
        self.lines.iter().all(|l| l.checked)
    }
}

/// One line of a text as it becomes a Line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub text: String,
    pub indent: bool,
    pub checked: bool,
}

/// The lines of `text`: split at every line break, `\r\n` and a lone
/// `\r` among them.
pub fn split_lines(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(str::to_string)
        .collect()
}

/// The Line the text line `row` becomes, or `None` for a blank one:
/// under the line above when it starts with a space or a tab, checked
/// when its text starts with "✓ ".
pub fn parse_line(row: &str) -> Option<ParsedLine> {
    let mut text = dart_trim(row);
    if text.is_empty() {
        return None;
    }
    let indent = row.starts_with(' ') || row.starts_with('\t');
    let mut checked = false;
    if let Some(rest) = text.strip_prefix(keys::CHECK_MARK) {
        let rest = dart_trim(rest);
        if !rest.is_empty() {
            text = rest;
            checked = true;
        }
    }
    Some(ParsedLine {
        text: text.to_string(),
        indent,
        checked,
    })
}

/// A Line as [`Checklists`] reads it from its fields.
struct Raw {
    id: String,
    text: String,
    order: Option<String>,
    indent: bool,
    checked: bool,
    checked_at: Option<DateTime<Utc>>,
}

impl Raw {
    fn new(id: String, fields: &Fields) -> Raw {
        Raw {
            id,
            text: StoredText::read(fields.get(keys::TEXT)).text,
            order: fields.get(keys::ORDER).map(str::to_string),
            indent: fields.get(keys::INDENT) == Some("true"),
            checked: fields.get(keys::CHECKED).is_some(),
            checked_at: fields.get(keys::CHECKED).and_then(|v| parse_time(v).ok()),
        }
    }

    /// The Line `id` of `fields` checked as `check` says.
    fn checked_by(id: String, fields: &Fields, check: Option<&Entry>) -> Raw {
        let value = check.and_then(|c| c.value.as_deref());
        Raw {
            id,
            text: StoredText::read(fields.get(keys::TEXT)).text,
            order: fields.get(keys::ORDER).map(str::to_string),
            indent: fields.get(keys::INDENT) == Some("true"),
            checked: value.is_some(),
            checked_at: value.and_then(|v| parse_time(v).ok()),
        }
    }
}

/// The checks of Lines a Routine holds for its Occurrences (ADR 0030),
/// only what counts: in a Shared Area what the roles allow (ADR 0013),
/// anywhere else what the user's own Devices wrote.
#[derive(Debug, Clone, Default)]
pub struct LineChecks {
    /// By Occurrence key, the check entry of each Line that holds.
    pub by_occurrence: BTreeMap<String, BTreeMap<String, Entry>>,
}

impl LineChecks {
    /// The checks of the Routine `id`, read from its history.
    pub fn of(store: &Store, id: &str) -> Result<LineChecks> {
        let history = store.history(id)?;
        Self::read(store, id, &history)
    }

    /// The checks of the Routine `id`, read from `history`.
    pub fn read(store: &Store, id: &str, history: &[Entry]) -> Result<LineChecks> {
        let shared = match store.area_of(id)? {
            Some(area) => store.is_shared(&area)?,
            None => false,
        };
        let mut own = HashMap::new();
        let mut latest: BTreeMap<String, BTreeMap<String, Entry>> = BTreeMap::new();
        for e in history {
            let Some(rest) = e.field.strip_prefix(keys::CHECK_PREFIX) else {
                continue;
            };
            if !shared && !store.written_by_own_in(&e.device, &mut own)? {
                continue;
            }
            let Some((key, line)) = rest.split_once('@') else {
                continue;
            };
            let named = Occurrence::parse(key).is_some_and(|o| o.key() == key);
            if key.is_empty() || !named || !is_line_id(line) {
                continue;
            }
            let checks = latest.entry(key.to_string()).or_default();
            let newer = checks
                .get(line)
                .is_none_or(|held| Store::latest_wins(e, held) == Ordering::Greater);
            if newer {
                checks.insert(line.to_string(), e.clone());
            }
        }
        Ok(LineChecks {
            by_occurrence: latest,
        })
    }

    /// Whether the Line `line_id` itself is checked for the Occurrence
    /// `key`.
    pub fn checked(&self, key: &str, line_id: &str) -> bool {
        self.value(key, line_id).is_some()
    }

    /// When the Line `line_id` was checked for the Occurrence `key`, as
    /// its check says.
    pub fn checked_at(&self, key: &str, line_id: &str) -> Option<DateTime<Utc>> {
        self.value(key, line_id).and_then(|v| parse_time(v).ok())
    }

    fn value(&self, key: &str, line_id: &str) -> Option<&str> {
        self.by_occurrence
            .get(key)
            .and_then(|checks| checks.get(line_id))
            .and_then(|e| e.value.as_deref())
    }

    /// The Occurrences whose every Line is checked, by key, each with the
    /// moment of its last check: the Lines of the making that check's
    /// Line belongs to, of a Note of `note_ids`, not deleted, but those
    /// added after it and not checked then. Never one without a Line.
    pub fn done(
        &self,
        store: &Store,
        note_ids: &[String],
    ) -> Result<BTreeMap<String, DateTime<Utc>>> {
        let mut done = BTreeMap::new();
        if self.by_occurrence.is_empty() {
            return Ok(done);
        }
        let mut lines: BTreeMap<String, Fields> = BTreeMap::new();
        for id in store.entities_of_type(keys::TYPE_LINE)? {
            let fields = store.current_fields(&id)?;
            if fields
                .get(keys::NOTE)
                .is_some_and(|note| note_ids.iter().any(|n| n == note))
            {
                lines.insert(id, fields);
            }
        }
        let mut added: HashMap<String, Option<DateTime<Utc>>> = HashMap::new();
        for (key, checks) in &self.by_occurrence {
            let mut last: Option<(&String, &Entry)> = None;
            for (line, e) in checks {
                if e.value.is_none() {
                    continue;
                }
                if last.is_none_or(|(_, held)| Store::latest_wins(e, held) == Ordering::Greater) {
                    last = Some((line, e));
                }
            }
            let Some((last_line, last)) = last else {
                continue;
            };
            let Some(of) = lines.get(last_line) else {
                continue;
            };
            let (Some(note), Some(made)) = (of.get(keys::NOTE), of.get(keys::MADE)) else {
                continue;
            };
            let mut raw = Vec::new();
            for (id, fields) in &lines {
                if fields.get(keys::NOTE) != Some(note)
                    || fields.get(keys::MADE) != Some(made)
                    || fields.get(keys::TRASHED).is_some()
                {
                    continue;
                }
                let check = checks.get(id);
                if check.is_none_or(|c| c.value.is_none()) {
                    let at = match added.get(id) {
                        Some(at) => *at,
                        None => {
                            let at = added_at(store, id)?;
                            added.insert(id.clone(), at);
                            at
                        }
                    };
                    if at.is_some_and(|at| at > last.date_at) {
                        continue;
                    }
                }
                raw.push(Raw::checked_by(id.clone(), fields, check));
            }
            if raw.is_empty() {
                continue;
            }
            if shown(note, raw).iter().all(|l| l.checked) {
                done.insert(key.clone(), last.date_at);
            }
        }
        Ok(done)
    }
}

/// When the Line `id` was added: its earliest `$type`, the same on every
/// Device.
fn added_at(store: &Store, id: &str) -> Result<Option<DateTime<Utc>>> {
    let mut first: Option<DateTime<Utc>> = None;
    for e in store.history(id)? {
        if e.field == keys::TYPE && first.is_none_or(|f| e.date_at < f) {
            first = Some(e.date_at);
        }
    }
    Ok(first)
}

fn is_line_id(id: &str) -> bool {
    id.len() > keys::LINE_PREFIX.len()
        && id.starts_with(keys::LINE_PREFIX)
        && !id.contains('@')
        && !id.contains(',')
}

/// What a new Line holds, as [`write_line`] writes it.
pub(crate) struct NewLine<'a> {
    pub text: &'a str,
    pub order: &'a str,
    pub indent: bool,
    pub checked: bool,
    pub received: Option<&'a str>,
}

/// Writes a new Line of the Note `note_id` in its making `making`,
/// inside a batch; returns its id.
pub(crate) fn write_line(
    store: &Store,
    note_id: &str,
    making: &str,
    line: &NewLine,
) -> Result<String> {
    let id = format!("{}{}", keys::LINE_PREFIX, store.new_id()?);
    let now = store.now();
    store.append(&id, keys::TYPE, Some(keys::TYPE_LINE), Some(now))?;
    store.append(&id, keys::NOTE, Some(note_id), Some(now))?;
    store.append(&id, keys::MADE, Some(making), Some(now))?;
    let text = StoredText::plain(line.text).encode();
    store.append(&id, keys::TEXT, Some(&text), Some(now))?;
    store.append(&id, keys::ORDER, Some(line.order), Some(now))?;
    if line.indent {
        store.append(&id, keys::INDENT, Some("true"), Some(now))?;
    }
    if line.checked {
        store.append(&id, keys::CHECKED, Some(&dart_iso(now)), Some(now))?;
    }
    if let Some(received) = line.received {
        store.append(&id, sent_keys::RECEIVED, Some(received), Some(now))?;
    }
    Ok(id)
}

/// Making Notes Checklists and back, and adding, changing, moving and
/// checking their Lines (ADR 0030).
pub struct Checklists<'a> {
    store: &'a Store,
}

impl<'a> Checklists<'a> {
    pub fn new(store: &'a Store) -> Checklists<'a> {
        Checklists { store }
    }

    /// The Checklist of the Note `note_id`, or `None` when it is none: a
    /// plain Note, a Tally (the Tally wins), or no Note.
    pub fn by_note(&self, note_id: &str) -> Result<Option<Checklist>> {
        Ok(self.of_notes(&[note_id.to_string()])?.remove(note_id))
    }

    /// Whether the Note `note_id` is a Checklist.
    pub fn is_checklist(&self, note_id: &str) -> Result<bool> {
        Ok(self.by_note(note_id)?.is_some())
    }

    /// The Checklists of the Notes `note_ids` that are one, read in one
    /// pass over the Lines, by Note.
    pub fn of_notes(&self, note_ids: &[String]) -> Result<BTreeMap<String, Checklist>> {
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
        let mut raw: HashMap<String, Vec<Raw>> = HashMap::new();
        for id in self.store.entities_of_type(keys::TYPE_LINE)? {
            let fields = self.store.current_fields(&id)?;
            let Some(note) = fields.get(keys::NOTE) else {
                continue;
            };
            if fields.get(keys::TRASHED).is_some() {
                continue;
            }
            let Some(making) = makings.get(note) else {
                continue;
            };
            if fields.get(keys::MADE) != Some(making.as_str()) {
                continue;
            }
            let note = note.to_string();
            raw.entry(note).or_default().push(Raw::new(id, &fields));
        }
        for (note, making) in makings {
            let lines = shown(&note, raw.remove(&note).unwrap_or_default());
            out.insert(
                note.clone(),
                Checklist {
                    note_id: note,
                    making,
                    lines,
                    on: None,
                },
            );
        }
        Ok(out)
    }

    /// The Checklist of the Note the Routine of `on` was spawned from,
    /// checked as for that Occurrence: only the checks made for it count,
    /// so every new Occurrence starts with no Line checked. `None` when the
    /// Routine is none or its Note no Checklist.
    pub fn for_occurrence(&self, on: &RoutineOccurrence) -> Result<Option<Checklist>> {
        let routines = Routines::new(self.store);
        let Some(routine) = routines.by_id(&on.routine_id)? else {
            return Ok(None);
        };
        let Some(note) = routines.note(&routine)? else {
            return Ok(None);
        };
        let Some(list) = self.by_note(&note.id)? else {
            return Ok(None);
        };
        let checks = LineChecks::of(self.store, &on.routine_id)?;
        Ok(Some(checked_for(list, on, &checks)))
    }

    /// The Occurrence the Checklist of `note_id` is checked for now, with
    /// days as `wall` shows them; `None` while the Note has no Routine or
    /// Reminder out of the trash. Of all of theirs, those on the latest
    /// day up to today that has any: the first not Done, else the last.
    /// Before their first day, the first of them.
    pub fn occurrence_now(
        &self,
        note_id: &str,
        wall: &WallClock,
    ) -> Result<Option<RoutineOccurrence>> {
        let routines = Routines::new(self.store).of_note(note_id)?;
        Ok(now_of(&routines, Day::today(self.store.now(), wall)))
    }

    /// The Checklist of `note_id` as it shows now: checked for the
    /// Occurrence it is on ([`Checklists::occurrence_now`]), or by its own
    /// checks while the Note has no Routine or Reminder.
    pub fn shown_now(&self, note_id: &str, wall: &WallClock) -> Result<Option<Checklist>> {
        Ok(self
            .of_notes_now(&[note_id.to_string()], wall)?
            .remove(note_id))
    }

    /// [`Checklists::of_notes`] as [`Checklists::shown_now`] shows each,
    /// in one pass over the Lines and the Routines.
    pub fn of_notes_now(
        &self,
        note_ids: &[String],
        wall: &WallClock,
    ) -> Result<BTreeMap<String, Checklist>> {
        let lists = self.of_notes(note_ids)?;
        if lists.is_empty() {
            return Ok(lists);
        }
        let routines = Routines::new(self.store);
        let mut spawned: HashMap<String, Vec<Routine>> = HashMap::new();
        for routine in routines.all()? {
            if let Some(note) = routines.note(&routine)?
                && lists.contains_key(&note.id)
            {
                spawned.entry(note.id).or_default().push(routine);
            }
        }
        let today = Day::today(self.store.now(), wall);
        let mut out = BTreeMap::new();
        for (id, list) in lists {
            let on = now_of(spawned.get(&id).map(Vec::as_slice).unwrap_or(&[]), today);
            let shown = match on {
                None => list,
                Some(on) => {
                    let checks = LineChecks::of(self.store, &on.routine_id)?;
                    checked_for(list, &on, &checks)
                }
            };
            out.insert(id, shown);
        }
        Ok(out)
    }

    /// Whether this Device may make the Note `note_id` a Checklist: a Note
    /// in the lists, of a text known here, neither a Tally nor a list
    /// already, which it may change.
    pub fn may_make(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed()
            && !note.is_merged()
            && note.text_editable
            && parse_list_kind(self.store.current(note_id, keys::KIND)?.as_deref()).is_none()
            && !Tallies::new(self.store).is_tally(note_id)?
            && self.store.may_change(note_id)?)
    }

    /// Makes the Note `note_id` a Checklist: its first line stays its
    /// text, every other line not blank becomes a Line, under the line
    /// above when it starts with a space or a tab, checked when it starts
    /// with "✓ ". Takes it out of Unsorted, as a Tally does.
    pub fn make(&self, note_id: &str) -> Result<()> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let Some(note) = note else {
            return Err(Error::Invalid(format!(
                "{note_id} may not be made a Checklist here"
            )));
        };
        if !self.may_make(note_id)? {
            return Err(Error::Invalid(format!(
                "{note_id} may not be made a Checklist here"
            )));
        }
        let rows = split_lines(&note.text);
        let title = dart_trim(rows.first().map(String::as_str).unwrap_or(""));
        let parsed: Vec<ParsedLine> = rows.iter().skip(1).filter_map(|r| parse_line(r)).collect();
        let making = self.store.new_id()?;
        self.store.batch(|| {
            let s = self.store;
            s.append(note_id, keys::KIND, Some(&keys::kind_of(&making)), None)?;
            if title != note.text {
                let text = StoredText::plain(title).encode();
                s.append(note_id, note_keys::TEXT, Some(&text), None)?;
            }
            if !note.spawned {
                let at = dart_iso(s.now());
                s.append(note_id, note_keys::SPAWNED, Some(&at), None)?;
            }
            let mut key = String::new();
            for (i, line) in parsed.iter().enumerate() {
                key = order::after(&key);
                write_line(
                    s,
                    note_id,
                    &making,
                    &NewLine {
                        text: &line.text,
                        order: &key,
                        indent: i > 0 && line.indent,
                        checked: line.checked,
                        received: None,
                    },
                )?;
            }
            Ok(())
        })
    }

    /// Turns the Checklist of `note_id` back into a plain Note: every Line
    /// goes into its text, one per line, under its text, a checked one
    /// marked "✓ ", one of a group indented. The Lines and their checks
    /// stay in the log.
    pub fn to_plain(&self, note_id: &str) -> Result<()> {
        let (note, list) = self.require_list(note_id)?;
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        if !note.text_editable {
            return Err(Error::Invalid(format!(
                "the text of {note_id} is in a format not known here"
            )));
        }
        let text = StoredText::plain(&Checklists::plain_text(&note.text, &list)).encode();
        self.store.batch(|| {
            self.store
                .append(note_id, note_keys::TEXT, Some(&text), None)?;
            self.store.append(note_id, keys::KIND, None, None)?;
            Ok(())
        })
    }

    /// The text the Checklist `list` of a Note titled `title` goes back
    /// to.
    pub fn plain_text(title: &str, list: &Checklist) -> String {
        let mut rows = Vec::new();
        if !title.is_empty() {
            rows.push(title.to_string());
        }
        for line in &list.lines {
            rows.push(format!(
                "{}{}{}",
                if line.in_group() {
                    keys::GROUP_INDENT
                } else {
                    ""
                },
                if line.checked { keys::CHECK_MARK } else { "" },
                line.text
            ));
        }
        rows.join("\n")
    }

    /// Whether this Device may add Lines to the Checklist of `note_id` and
    /// check them: anywhere it holds a role.
    pub fn may_check(&self, note_id: &str) -> Result<bool> {
        self.store.may_write_in(note_id)
    }

    /// Whether this Device may rename, delete, move and indent the Line
    /// `line_id`: in no Shared Area or one it manages, or as the author of
    /// the Line or of its Note.
    pub fn may_edit(&self, line_id: &str) -> Result<bool> {
        if self.store.may_change(line_id)? {
            return Ok(true);
        }
        match self.store.current(line_id, keys::NOTE)? {
            Some(note) => self.store.may_change(&note),
            None => Ok(false),
        }
    }

    /// Adds a Line holding `text` to the Checklist of `note_id` at `at`,
    /// an index in the order the Lines show, or at the end; under the
    /// Line above with `indent`. Anyone with a role in its Area may.
    /// Returns its id.
    pub fn add(
        &self,
        note_id: &str,
        text: &str,
        at: Option<usize>,
        indent: bool,
    ) -> Result<String> {
        let (_, list) = self.require_list(note_id)?;
        if !self.may_check(note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {note_id}"
            )));
        }
        let named = line_text(text)?;
        let lines = &list.lines;
        let pos = at.map_or(lines.len(), |at| at.min(lines.len()));
        let (a, b) = neighbours(lines, pos);
        let key = order::place(a.as_deref(), b.as_deref());
        self.store.batch(|| {
            write_line(
                self.store,
                note_id,
                &list.making,
                &NewLine {
                    text: &named,
                    order: &key,
                    indent: indent && pos > 0,
                    checked: false,
                    received: None,
                },
            )
        })
    }

    /// Renames the Line `line_id`.
    pub fn rename(&self, line_id: &str, text: &str) -> Result<()> {
        let (_, line) = self.require_editable(line_id)?;
        let named = line_text(text)?;
        if named == line.text {
            return Ok(());
        }
        let text = StoredText::plain(&named).encode();
        self.store
            .append(line_id, keys::TEXT, Some(&text), None)
            .map(drop)
    }

    /// Deletes the Line `line_id`; its group, as far as this Device may
    /// change it, comes up to the top. It can be restored.
    pub fn delete(&self, line_id: &str) -> Result<()> {
        let (list, _) = self.require_editable(line_id)?;
        let at = dart_iso(self.store.now());
        let group: Vec<String> = list
            .group_of(line_id)
            .into_iter()
            .map(|l| l.id.clone())
            .collect();
        self.store.batch(|| {
            self.store.append(line_id, keys::TRASHED, Some(&at), None)?;
            for id in &group {
                if self.may_edit(id)? {
                    self.store.append(id, keys::INDENT, None, None)?;
                }
            }
            Ok(())
        })
    }

    /// Brings the deleted Line `line_id` back, where it was.
    pub fn restore(&self, line_id: &str) -> Result<()> {
        let fields = self.store.current_fields(line_id)?;
        let deleted = match fields.get(keys::NOTE) {
            Some(note) => {
                fields.get(keys::TYPE) == Some(keys::TYPE_LINE)
                    && fields.get(keys::TRASHED).is_some()
                    && fields.get(keys::MADE) == self.making(note)?.as_deref()
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
            .append(line_id, keys::TRASHED, None, None)
            .map(drop)
    }

    /// Moves the Line `line_id`, with its group, to just before the Line
    /// `before`, or to the end. A single Line moved before a Line of a
    /// group joins that group; one placed before a Line at the top comes
    /// to the top. A Line with a group never goes into another: placed in
    /// one, it goes after it.
    pub fn move_line(&self, line_id: &str, before: Option<&str>) -> Result<()> {
        let (list, line) = self.require_editable(line_id)?;
        let mut block: Vec<&Line> = vec![&line];
        block.extend(list.group_of(line_id));
        if let Some(before) = before
            && block.iter().any(|l| l.id == before)
        {
            return Ok(());
        }
        let rest: Vec<Line> = list
            .lines
            .iter()
            .filter(|l| !block.iter().any(|b| b.id == l.id))
            .cloned()
            .collect();
        let mut pos = rest.len();
        if let Some(before) = before {
            pos = rest
                .iter()
                .position(|l| l.id == before)
                .ok_or_else(|| Error::Invalid(format!("not a Line of the list: {before}")))?;
        }
        let grouped = block.len() > 1;
        if grouped {
            while pos < rest.len() && rest[pos].in_group() {
                pos += 1;
            }
        }
        let indent = !grouped && pos < rest.len() && rest[pos].in_group();
        let (mut a, b) = neighbours(&rest, pos);
        let ids: Vec<String> = block.iter().map(|l| l.id.clone()).collect();
        let indented = self.indented(line_id)?;
        self.store.batch(|| {
            for id in &ids {
                let key = order::place(a.as_deref(), b.as_deref());
                self.store.append(id, keys::ORDER, Some(&key), None)?;
                a = Some(key);
            }
            if !grouped && indent != indented {
                let value = if indent { Some("true") } else { None };
                self.store.append(line_id, keys::INDENT, value, None)?;
            }
            Ok(())
        })
    }

    /// Puts the Line `line_id` under the Line above it, or with `under`
    /// false back at the top, where it is; the Lines under it then follow
    /// it. A Line at the very top has nothing to go under.
    pub fn indent(&self, line_id: &str, under: bool) -> Result<()> {
        let (list, _) = self.require_editable(line_id)?;
        if under && list.lines.first().is_some_and(|l| l.id == line_id) {
            return Err(Error::Invalid(format!("no Line above {line_id}")));
        }
        if under == self.indented(line_id)? {
            return Ok(());
        }
        let value = if under { Some("true") } else { None };
        self.store
            .append(line_id, keys::INDENT, value, None)
            .map(drop)
    }

    /// Checks the Line `line_id`: a Line with a group checks its whole
    /// group, and the last Line of a group checked checks its parent.
    /// Every Line that changes gets an entry of its own.
    pub fn check(&self, line_id: &str) -> Result<()> {
        self.set_checked(line_id, true, None)
    }

    /// Unchecks the Line `line_id`: a Line with a group unchecks its whole
    /// group, and one of a group unchecks its parent.
    pub fn uncheck(&self, line_id: &str) -> Result<()> {
        self.set_checked(line_id, false, None)
    }

    /// [`Checklists::check`] for the Occurrence `on` only: the entries lie
    /// on its Routine.
    pub fn check_for(&self, line_id: &str, on: &RoutineOccurrence) -> Result<()> {
        self.set_checked(line_id, true, Some(on))
    }

    /// [`Checklists::uncheck`] for the Occurrence `on` only.
    pub fn uncheck_for(&self, line_id: &str, on: &RoutineOccurrence) -> Result<()> {
        self.set_checked(line_id, false, Some(on))
    }

    /// Unchecks every Line of the Checklist of `note_id`, to use it again.
    pub fn uncheck_all(&self, note_id: &str) -> Result<()> {
        self.uncheck_all_on(note_id, None)
    }

    /// [`Checklists::uncheck_all`] for the Occurrence `on` only.
    pub fn uncheck_all_for(&self, note_id: &str, on: &RoutineOccurrence) -> Result<()> {
        self.uncheck_all_on(note_id, Some(on))
    }

    fn uncheck_all_on(&self, note_id: &str, on: Option<&RoutineOccurrence>) -> Result<()> {
        let (_, list) = self.require_list(note_id)?;
        let checks = match on {
            None => None,
            Some(on) => Some(self.require_occurrence(on, note_id)?),
        };
        if on.is_none() && !self.may_check(note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {note_id}"
            )));
        }
        self.store.batch(|| {
            for line in &list.lines {
                if let (Some(on), Some(checks)) = (on, &checks) {
                    if checks.checked(&on.occurrence.key(), &line.id) {
                        let field = keys::check_for(&on.occurrence, &line.id);
                        self.store.append(&on.routine_id, &field, None, None)?;
                    }
                } else if self.checked_own(&line.id)? {
                    self.store.append(&line.id, keys::CHECKED, None, None)?;
                }
            }
            Ok(())
        })
    }

    fn set_checked(
        &self,
        line_id: &str,
        checked: bool,
        on: Option<&RoutineOccurrence>,
    ) -> Result<()> {
        let (own, own_line) = self.require_line(line_id)?;
        let checks = match on {
            None => None,
            Some(on) => Some(self.require_occurrence(on, &own.note_id)?),
        };
        if on.is_none() && !self.may_check(&own.note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {}",
                own.note_id
            )));
        }
        let list = match (on, &checks) {
            (Some(on), Some(checks)) => checked_for(own, on, checks),
            _ => own,
        };
        let line = list.line(line_id).cloned().unwrap_or(own_line);
        let mut changed: Vec<String> = vec![line.id.clone()];
        changed.extend(list.group_of(line_id).into_iter().map(|l| l.id.clone()));
        if let Some(parent) = &line.parent
            && let Some(above) = list.line(parent)
        {
            let rest_checked = list
                .group_of(parent)
                .iter()
                .filter(|l| l.id != line_id)
                .all(|l| l.checked);
            if !checked || rest_checked {
                changed.push(above.id.clone());
            }
        }
        let at = dart_iso(self.store.now());
        self.store.batch(|| {
            for id in &changed {
                let value = if checked { Some(at.as_str()) } else { None };
                if let (Some(on), Some(checks)) = (on, &checks) {
                    if checks.checked(&on.occurrence.key(), id) == checked {
                        continue;
                    }
                    let field = keys::check_for(&on.occurrence, id);
                    self.store.append(&on.routine_id, &field, value, None)?;
                    continue;
                }
                if self.checked_own(id)? == checked {
                    continue;
                }
                self.store.append(id, keys::CHECKED, value, None)?;
            }
            Ok(())
        })
    }

    /// The checks of the Routine of `on`, which must be one out of the
    /// trash spawned from the Note `note_id`, due then, that this Device
    /// may check off.
    fn require_occurrence(&self, on: &RoutineOccurrence, note_id: &str) -> Result<LineChecks> {
        let routines = Routines::new(self.store);
        let routine = routines.by_id(&on.routine_id)?;
        let ours = match &routine {
            Some(r) if !r.is_trashed() => {
                r.note_id == note_id || routines.note(r)?.is_some_and(|n| n.id == note_id)
            }
            _ => false,
        };
        let Some(routine) = routine.filter(|_| ours) else {
            return Err(Error::Invalid(format!(
                "{} is no Routine of {note_id}",
                on.routine_id
            )));
        };
        if !routine.schedule.has(&on.occurrence) {
            return Err(Error::Invalid(format!(
                "{} is not due then",
                on.occurrence.key()
            )));
        }
        if !self.store.may_check_off(&on.routine_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {}",
                on.routine_id
            )));
        }
        LineChecks::of(self.store, &on.routine_id)
    }

    /// Whether the Line `id` itself was checked last.
    fn checked_own(&self, id: &str) -> Result<bool> {
        Ok(self.store.current(id, keys::CHECKED)?.is_some())
    }

    fn indented(&self, id: &str) -> Result<bool> {
        Ok(self.store.current(id, keys::INDENT)?.as_deref() == Some("true"))
    }

    /// The making of the Checklist the Note `note_id` is now, or `None`
    /// when it is no Checklist: no Note, another list, or a Tally.
    fn making(&self, note_id: &str) -> Result<Option<String>> {
        let fields = self.store.current_fields(note_id)?;
        if fields.get(note_keys::TYPE) != Some(note_keys::TYPE_NOTE) {
            return Ok(None);
        }
        let Some((list, making)) = parse_list_kind(fields.get(keys::KIND)) else {
            return Ok(None);
        };
        if list != keys::CHECKLIST || Tallies::new(self.store).is_tally(note_id)? {
            return Ok(None);
        }
        Ok(Some(making.to_string()))
    }

    /// The Note `note_id`, not in the trash, and its Checklist.
    fn require_list(&self, note_id: &str) -> Result<(Note, Checklist)> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let list = self.by_note(note_id)?;
        match (note, list) {
            (Some(note), Some(list)) if !note.is_trashed() => Ok((note, list)),
            _ => Err(Error::Invalid(format!("not a Checklist: {note_id}"))),
        }
    }

    /// The Checklist the shown Line `line_id` lies in, its Note not in the
    /// trash, and the Line.
    fn require_line(&self, line_id: &str) -> Result<(Checklist, Line)> {
        let missing = || Error::Invalid(format!("not a Line of a Checklist: {line_id}"));
        let note = self
            .store
            .current(line_id, keys::NOTE)?
            .ok_or_else(missing)?;
        let list = self.by_note(&note)?.ok_or_else(missing)?;
        let line = list.line(line_id).cloned().ok_or_else(missing)?;
        let trashed = Notes::new(self.store)
            .by_id(&note)?
            .is_none_or(|n| n.is_trashed());
        if trashed {
            return Err(missing());
        }
        Ok((list, line))
    }

    fn require_editable(&self, line_id: &str) -> Result<(Checklist, Line)> {
        let found = self.require_line(line_id)?;
        if !self.may_edit(line_id)? {
            return Err(Error::Invalid(format!("may not change {line_id}")));
        }
        Ok(found)
    }
}

/// `raw` in the order they show, each with its parent and what shows
/// checked.
fn shown(note_id: &str, mut raw: Vec<Raw>) -> Vec<Line> {
    raw.sort_by(|a, b| {
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
    });
    let mut parents: Vec<Option<String>> = Vec::new();
    let mut top: Option<String> = None;
    for r in &raw {
        match &top {
            Some(t) if r.indent => parents.push(Some(t.clone())),
            _ => {
                top = Some(r.id.clone());
                parents.push(None);
            }
        }
    }
    let mut group_checked: HashMap<String, bool> = HashMap::new();
    for (r, parent) in raw.iter().zip(&parents) {
        if let Some(parent) = parent {
            let held = group_checked.entry(parent.clone()).or_insert(true);
            *held = *held && r.checked;
        }
    }
    raw.into_iter()
        .zip(parents)
        .map(|(r, parent)| Line {
            checked: group_checked.get(&r.id).copied().unwrap_or(r.checked),
            note_id: note_id.to_string(),
            text: r.text,
            checked_at: r.checked_at,
            parent,
            order: r.order,
            id: r.id,
        })
        .collect()
}

/// `list` with its Lines checked as `checks` hold them for `on`.
fn checked_for(list: Checklist, on: &RoutineOccurrence, checks: &LineChecks) -> Checklist {
    let key = on.occurrence.key();
    let mut group_checked: HashMap<String, bool> = HashMap::new();
    for line in &list.lines {
        if let Some(parent) = &line.parent {
            let held = group_checked.entry(parent.clone()).or_insert(true);
            *held = *held && checks.checked(&key, &line.id);
        }
    }
    let lines = list
        .lines
        .into_iter()
        .map(|line| Line {
            checked: group_checked
                .get(&line.id)
                .copied()
                .unwrap_or_else(|| checks.checked(&key, &line.id)),
            checked_at: checks.checked_at(&key, &line.id),
            ..line
        })
        .collect();
    Checklist {
        note_id: list.note_id,
        making: list.making,
        lines,
        on: Some(on.clone()),
    }
}

/// The Occurrence of `routines` a Checklist is on `today`
/// ([`Checklists::occurrence_now`]).
fn now_of(routines: &[Routine], today: Day) -> Option<RoutineOccurrence> {
    let latest = routines
        .iter()
        .filter_map(|r| last_day(&r.schedule, today))
        .max();
    let mut found: Vec<(&Routine, Occurrence)> = Vec::new();
    for routine in routines {
        let occurrences = match latest {
            Some(day) => routine.schedule.on(day),
            None => routine.schedule.next(today, 1),
        };
        found.extend(occurrences.into_iter().map(|o| (routine, o)));
    }
    found.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| utf16_cmp(&a.0.id, &b.0.id)));
    let picked = match latest {
        None => found.first(),
        Some(_) => found
            .iter()
            .find(|(r, o)| r.done_at(o).is_none())
            .or(found.last()),
    }?;
    Some(RoutineOccurrence {
        routine_id: picked.0.id.clone(),
        occurrence: picked.1,
    })
}

/// The latest day up to `today` on which `schedule` has an Occurrence,
/// looked for in ever wider spans before it.
fn last_day(schedule: &Schedule, today: Day) -> Option<Day> {
    let start = schedule.start;
    if start > today {
        return None;
    }
    for span in [0, 7, 62, 400] {
        let from = today.add_days(-span).max(start);
        if let Some(last) = schedule.between(from, today).last() {
            return Some(last.day);
        }
        if from == start {
            return None;
        }
    }
    schedule.between(start, today).last().map(|o| o.day)
}

/// The valid keys around the place `pos` of `lines`: the last one before
/// it, and the first one after it past that one.
fn neighbours(lines: &[Line], pos: usize) -> (Option<String>, Option<String>) {
    let a = lines[..pos]
        .iter()
        .rev()
        .find(|l| order::is_valid(l.order.as_deref()))
        .and_then(|l| l.order.clone());
    let b = lines[pos..]
        .iter()
        .filter_map(|l| l.order.as_deref())
        .find(|key| {
            order::is_valid(Some(key))
                && a.as_deref()
                    .is_none_or(|a| utf16_cmp(key, a) == std::cmp::Ordering::Greater)
        })
        .map(str::to_string);
    (a, b)
}

/// `text` on one line and trimmed; an empty one is refused.
fn line_text(text: &str) -> Result<String> {
    let mut one = String::new();
    let mut gap = false;
    for c in text.chars() {
        if c == '\r' || c == '\n' {
            if !gap {
                one.push(' ');
            }
            gap = true;
        } else {
            one.push(c);
            gap = false;
        }
    }
    let trimmed = dart_trim(&one);
    if trimmed.is_empty() {
        return Err(Error::Invalid("a Line's text must not be empty".into()));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
    };
    use crate::shared::{AreaRole, Invite};
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::ticking;
    use crate::sync::wire;
    use crate::{Entry, OwnProfile, Places, Search};
    use chrono::Duration;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    fn clocked(seed: u8) -> Made<Store> {
        let start = ticking()();
        let clock: crate::Clock = Box::new(move || {
            let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            start + Duration::seconds(n)
        });
        Ok(Store::in_memory(clock, move || {
            Ok(SigningKey::from_seed([seed; 32]))
        })?)
    }

    fn device(seed: u8, name: &str) -> Made<Store> {
        let store = clocked(seed)?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    /// One LAN session, `joiner` asking `host`, between two people.
    fn exchange(joiner: &Store, host: &Store) -> TestResult {
        let request = sync_request(
            joiner,
            &host.version_vector()?,
            "Phone",
            &host.device_id()?,
            "",
        )?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        apply_answer_as(joiner, &answer, true, None)?;
        Ok(())
    }

    /// Everything `from` holds that `to` lacks, taken in: two of the
    /// user's own Devices.
    fn send(from: &Store, to: &Store) -> TestResult {
        to.apply_entries(
            &wire(&from.entries_since(&to.version_vector()?)?),
            &from.key_records()?,
            None,
            &mut ImportReport::default(),
        )?;
        Ok(())
    }

    /// `member` invited by `ann` into `area` as a Member.
    fn invite(ann: &Store, member: &Store, area: &str) -> TestResult {
        let invite = Invite {
            area: area.to_string(),
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            member,
            &ann.version_vector()?,
            "Phone",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        let (answer, _) =
            answer_sync_inviting(ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
        apply_answer_as(member, &answer, true, None)?;
        for (store, peer) in [(ann, member), (member, ann)] {
            store.remember_device(&crate::PairedDevice {
                id: peer.device_id()?,
                name: String::new(),
                fingerprint: String::new(),
                host: None,
                port: None,
                last_sync: store.now(),
                shared: true,
            })?;
        }
        Ok(())
    }

    /// Ann's Household shared with Ben as a Member.
    fn household() -> Made<(Store, Store, String)> {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        invite(&ann, &ben, &area)?;
        Ok((ann, ben, area))
    }

    /// A Checklist made from a new Note with `text`.
    fn checklist(store: &Store, text: &str, place: Option<&str>) -> Made<String> {
        let id = Notes::new(store).capture(text, place, None)?.id;
        Checklists::new(store).make(&id)?;
        Ok(id)
    }

    /// The Lines of the Checklist of `note` as `text`, `  text` for one of
    /// a group, `[x]` for one shown checked.
    fn shown(store: &Store, note: &str) -> Made<Vec<String>> {
        let list = Checklists::new(store).by_note(note)?;
        Ok(list
            .map(|l| l.lines)
            .unwrap_or_default()
            .iter()
            .map(|l| {
                format!(
                    "{}{}{}",
                    if l.in_group() { "  " } else { "" },
                    if l.checked { "[x] " } else { "" },
                    l.text
                )
            })
            .collect())
    }

    fn id_of(store: &Store, note: &str, text: &str) -> Made<String> {
        let list = Checklists::new(store).by_note(note)?.ok_or("a Checklist")?;
        Ok(list
            .lines
            .iter()
            .find(|l| l.text == text)
            .ok_or("a Line")?
            .id
            .clone())
    }

    #[test]
    fn the_first_line_stays_and_an_indented_one_goes_under_the_line_above() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = Notes::new(&ann).capture(
            "Packing\r\nSocks\n  Blue\n\tRed\n\n✓ Passport\r   \nTent\n  ✓ Pegs",
            None,
            None,
        )?;
        let lists = Checklists::new(&ann);
        assert!(lists.may_make(&note.id)?);
        lists.make(&note.id)?;
        let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
        assert_eq!(made.text, "Packing");
        assert!(!made.is_unsorted());
        assert_eq!(
            shown(&ann, &note.id)?,
            [
                "Socks",
                "  Blue",
                "  Red",
                "[x] Passport",
                "[x] Tent",
                "  [x] Pegs"
            ]
        );
        let list = lists.by_note(&note.id)?.ok_or("a Checklist")?;
        assert_eq!((list.checked_count(), list.count()), (3, 6));
        assert!(!lists.may_make(&note.id)?);
        assert!(lists.make(&note.id).is_err());
        // Two levels only; a first Line indented stays at the top.
        let deep = checklist(&ann, "List\n  a\n    b\n      c\nd", None)?;
        assert_eq!(shown(&ann, &deep)?, ["a", "  b", "  c", "d"]);
        Ok(())
    }

    #[test]
    fn a_note_is_at_most_one_of_a_tally_and_a_checklist() -> TestResult {
        let ann = device(1, "Ann")?;
        let gym = Notes::new(&ann).capture("Gym\nPush-ups", None, None)?.id;
        Tallies::new(&ann).add_field(&gym, "Push-ups", "", None, None)?;
        assert!(!Checklists::new(&ann).may_make(&gym)?);
        assert!(Checklists::new(&ann).make(&gym).is_err());
        let list = checklist(&ann, "Packing\nSocks", None)?;
        assert!(
            Tallies::new(&ann)
                .add_field(&list, "Socks", "", None, None)
                .is_err()
        );
        assert!(!Tallies::new(&ann).is_tally(&list)?);
        Ok(())
    }

    #[test]
    fn kinds_and_places_no_app_writes_are_read_without_fail() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = Notes::new(&ann).capture("Odd\nOne", None, None)?.id;
        for kind in [
            "checklist",
            "checklist:",
            ":x",
            "checklist:a:b",
            "shoppingList:x",
        ] {
            ann.append(&note, keys::KIND, Some(kind), None)?;
            assert!(Checklists::new(&ann).by_note(&note)?.is_none(), "{kind}");
        }
        ann.append(&note, keys::KIND, None, None)?;
        let lists = Checklists::new(&ann);
        lists.make(&note)?;
        let making = lists.by_note(&note)?.ok_or("a Checklist")?.making;
        for (i, order) in [Some("V0"), Some(""), Some("é"), None, Some("V")]
            .iter()
            .enumerate()
        {
            let id = format!("line:odd{i}");
            ann.append(&id, keys::TYPE, Some(keys::TYPE_LINE), None)?;
            ann.append(&id, keys::NOTE, Some(&note), None)?;
            ann.append(&id, keys::MADE, Some(&making), None)?;
            ann.append(&id, keys::TEXT, Some(&format!("Odd {i}")), None)?;
            if let Some(order) = order {
                ann.append(&id, keys::ORDER, Some(order), None)?;
            }
        }
        assert_eq!(
            shown(&ann, &note)?,
            ["One", "Odd 4", "Odd 1", "Odd 3", "Odd 0", "Odd 2"]
        );
        let added = lists.add(&note, "Added", Some(3), false)?;
        lists.move_line("line:odd2", Some("line:odd4"))?;
        lists.move_line(&added, None)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["One", "Odd 4", "Odd 2", "Added", "Odd 1", "Odd 3", "Odd 0"]
        );
        Ok(())
    }

    #[test]
    fn a_parent_and_its_group_check_each_other() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = checklist(&ann, "Trip\nBath\n  Soap\n  Towel\nTent", None)?;
        let lists = Checklists::new(&ann);
        let text = ann.current(&note, note_keys::TEXT)?;
        lists.check(&id_of(&ann, &note, "Soap")?)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Bath", "  [x] Soap", "  Towel", "Tent"]
        );
        let mark = ann.current_seq()?;
        lists.check(&id_of(&ann, &note, "Towel")?)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["[x] Bath", "  [x] Soap", "  [x] Towel", "Tent"]
        );
        let written: Vec<String> = ann
            .entries_after(mark)?
            .into_iter()
            .map(|e| e.entity)
            .collect();
        assert_eq!(
            written,
            [id_of(&ann, &note, "Towel")?, id_of(&ann, &note, "Bath")?]
        );
        lists.uncheck(&id_of(&ann, &note, "Soap")?)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Bath", "  Soap", "  [x] Towel", "Tent"]
        );
        lists.check(&id_of(&ann, &note, "Bath")?)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["[x] Bath", "  [x] Soap", "  [x] Towel", "Tent"]
        );
        lists.uncheck(&id_of(&ann, &note, "Bath")?)?;
        assert_eq!(shown(&ann, &note)?, ["Bath", "  Soap", "  Towel", "Tent"]);
        assert_eq!(ann.current(&note, note_keys::TEXT)?, text);
        let shop = checklist(&ann, "Shop\n✓ Milk\n✓ Eggs\nBread\n  ✓ Rye", None)?;
        lists.uncheck_all(&shop)?;
        assert_eq!(shown(&ann, &shop)?, ["Milk", "Eggs", "Bread", "  Rye"]);
        Ok(())
    }

    #[test]
    fn lines_are_added_renamed_deleted_restored_moved_and_indented() -> TestResult {
        let ann = device(1, "Ann")?;
        let lists = Checklists::new(&ann);
        let note = checklist(&ann, "Shop\nMilk\nEggs", None)?;
        lists.add(&note, "  Bread  ", None, false)?;
        lists.add(&note, "Butter", Some(0), false)?;
        lists.add(&note, "Oat", Some(2), true)?;
        lists.add(&note, "Top", Some(0), true)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Top", "Butter", "Milk", "  Oat", "Eggs", "Bread"]
        );
        assert!(lists.add(&note, " \n ", None, false).is_err());
        let milk = id_of(&ann, &note, "Milk")?;
        lists.rename(&milk, "Whole\nmilk")?;
        lists.delete(&milk)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Top", "Butter", "Oat", "Eggs", "Bread"]
        );
        lists.restore(&milk)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["Top", "Butter", "Whole milk", "Oat", "Eggs", "Bread"]
        );
        assert!(lists.restore(&milk).is_err());

        let trip = checklist(&ann, "Trip\nBath\n  Soap\n  Towel\nTent\nMap", None)?;
        let line = |text: &str| id_of(&ann, &trip, text);
        lists.move_line(&line("Map")?, Some(&line("Towel")?))?;
        assert_eq!(
            shown(&ann, &trip)?,
            ["Bath", "  Soap", "  Map", "  Towel", "Tent"]
        );
        lists.move_line(&line("Map")?, Some(&line("Bath")?))?;
        assert_eq!(
            shown(&ann, &trip)?,
            ["Map", "Bath", "  Soap", "  Towel", "Tent"]
        );
        lists.move_line(&line("Bath")?, None)?;
        assert_eq!(
            shown(&ann, &trip)?,
            ["Map", "Tent", "Bath", "  Soap", "  Towel"]
        );
        lists.indent(&line("Tent")?, true)?;
        lists.move_line(&line("Bath")?, Some(&line("Tent")?))?;
        assert_eq!(
            shown(&ann, &trip)?,
            ["Map", "  Tent", "Bath", "  Soap", "  Towel"]
        );
        lists.indent(&line("Soap")?, false)?;
        assert_eq!(
            shown(&ann, &trip)?,
            ["Map", "  Tent", "Bath", "Soap", "  Towel"]
        );
        lists.indent(&line("Soap")?, true)?;
        assert!(lists.indent(&line("Map")?, true).is_err());
        lists.delete(&line("Bath")?)?;
        assert_eq!(shown(&ann, &trip)?, ["Map", "  Tent", "Soap", "Towel"]);
        Ok(())
    }

    #[test]
    fn keys_fit_between_any_two_and_stay_short_at_the_end() {
        assert_eq!(order::place(None, None), "V");
        assert_eq!(order::after("V"), "W");
        assert_eq!(order::after("Vz"), "W");
        assert_eq!(order::after("z"), "zV");
        assert_eq!(order::after("zzz"), "zzzV");
        assert_eq!(order::between("", "V"), "F");
        assert_eq!(order::between("", "1"), "0V");
        assert_eq!(order::between("", "0V"), "0F");
        assert_eq!(order::between("V", "W"), "VV");
        assert_eq!(order::between("V", "V05"), "V02");
        assert_eq!(order::between("V", "X"), "W");
        assert_eq!(order::between("VV", "W"), "VW");
        assert_eq!(order::between("Vz", "W"), "VzV");
        // No key slices a character apart.
        assert_eq!(order::between("é", "éa"), "éV");
        assert_eq!(order::after("é"), "éV");
        let (mut low, mut high) = ("V".to_string(), "W".to_string());
        for i in 0..200 {
            let mid = order::between(&low, &high);
            assert!(order::is_valid(Some(&mid)), "{mid}");
            assert!(low < mid && mid < high);
            if i % 2 == 0 {
                low = mid;
            } else {
                high = mid;
            }
        }
        let mut key = String::new();
        for _ in 0..100 {
            key = order::after(&key);
        }
        assert!(key.len() <= 4);
    }

    #[test]
    fn back_to_a_plain_note_and_made_again_afresh() -> TestResult {
        let ann = device(1, "Ann")?;
        let lists = Checklists::new(&ann);
        let note = checklist(&ann, "Trip\nBath\n  Soap\n  ✓ Towel\n✓ Tent", None)?;
        let soap = id_of(&ann, &note, "Soap")?;
        lists.check(&soap)?;
        lists.add(&note, "Map", None, false)?;
        lists.to_plain(&note)?;
        assert!(lists.by_note(&note)?.is_none());
        let text = Notes::new(&ann).by_id(&note)?.ok_or("the Note")?.text;
        assert_eq!(text, "Trip\n✓ Bath\n  ✓ Soap\n  ✓ Towel\n✓ Tent\nMap");
        assert!(ann.current(&soap, keys::CHECKED)?.is_some());
        lists.make(&note)?;
        assert_eq!(
            shown(&ann, &note)?,
            ["[x] Bath", "  [x] Soap", "  [x] Towel", "[x] Tent", "Map"]
        );
        assert!(lists.by_note(&note)?.ok_or("a list")?.line(&soap).is_none());
        let found: Vec<String> = Search::new(&ann)
            .notes("SOAP", None)?
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(found, std::slice::from_ref(&note));
        lists.delete(&id_of(&ann, &note, "Soap")?)?;
        assert!(Search::new(&ann).notes("soap", None)?.is_empty());
        Ok(())
    }

    #[test]
    fn every_member_checks_unchecks_and_adds_lines() -> TestResult {
        let (ann, ben, area) = household()?;
        let note = checklist(&ann, "Shop\nMilk\nEggs", Some(&area))?;
        exchange(&ben, &ann)?;
        let lists = Checklists::new(&ben);
        assert!(lists.may_check(&note)?);
        lists.check(&id_of(&ben, &note, "Milk")?)?;
        lists.check(&id_of(&ben, &note, "Eggs")?)?;
        lists.uncheck(&id_of(&ben, &note, "Eggs")?)?;
        let bread = lists.add(&note, "Bread", None, false)?;
        exchange(&ben, &ann)?;
        assert_eq!(shown(&ann, &note)?, ["[x] Milk", "Eggs", "Bread"]);
        assert!(lists.may_edit(&bread)?);
        assert!(!lists.may_edit(&id_of(&ben, &note, "Milk")?)?);
        lists.rename(&bread, "Rye bread")?;
        exchange(&ben, &ann)?;
        Checklists::new(&ann).rename(&bread, "Brown bread")?;
        exchange(&ben, &ann)?;
        assert_eq!(shown(&ben, &note)?, ["[x] Milk", "Eggs", "Brown bread"]);
        Ok(())
    }

    #[test]
    fn renaming_deleting_moving_and_the_kind_stay_with_the_author_and_the_managers() -> TestResult {
        let (ann, ben, area) = household()?;
        let note = checklist(&ann, "Shop\nMilk\nEggs\nTea", Some(&area))?;
        exchange(&ben, &ann)?;
        let lists = Checklists::new(&ben);
        let milk = id_of(&ben, &note, "Milk")?;
        let eggs = id_of(&ben, &note, "Eggs")?;
        assert!(lists.rename(&milk, "Beer").is_err());
        assert!(lists.delete(&milk).is_err());
        assert!(lists.move_line(&milk, None).is_err());
        assert!(lists.indent(&eggs, true).is_err());
        assert!(lists.to_plain(&note).is_err());
        // As a modified app would.
        let beer = StoredText::plain("Beer").encode();
        let tea = id_of(&ben, &note, "Tea")?;
        let forged = [
            ben.append(&milk, keys::TEXT, Some(&beer), None)?,
            ben.append(&eggs, keys::TRASHED, Some("2026-03-01T09:00:00.000Z"), None)?,
            ben.append(&milk, keys::ORDER, Some("z"), None)?,
            ben.append(&tea, keys::INDENT, Some("true"), None)?,
            ben.append(&note, keys::KIND, None, None)?,
        ];
        exchange(&ben, &ann)?;
        let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
        assert!(forged.iter().all(|e| held.contains(&e.id())));
        assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea"]);
        Ok(())
    }

    #[test]
    fn a_member_who_made_the_checklist_changes_every_line_of_it() -> TestResult {
        let (ann, ben, area) = household()?;
        let cleo = device(3, "Cleo")?;
        invite(&ann, &cleo, &area)?;
        let note = checklist(&ben, "Shop\nMilk", Some(&area))?;
        exchange(&ben, &ann)?;
        exchange(&cleo, &ann)?;
        let tea = Checklists::new(&cleo).add(&note, "Tea", None, false)?;
        exchange(&cleo, &ann)?;
        exchange(&ben, &ann)?;
        assert!(Checklists::new(&ben).may_edit(&tea)?);
        Checklists::new(&ben).rename(&tea, "Green tea")?;
        Checklists::new(&ben).move_line(&tea, Some(&id_of(&ben, &note, "Milk")?))?;
        exchange(&ben, &ann)?;
        exchange(&cleo, &ann)?;
        assert_eq!(shown(&ann, &note)?, ["Green tea", "Milk"]);
        assert_eq!(shown(&cleo, &note)?, ["Green tea", "Milk"]);
        assert!(!Checklists::new(&cleo).may_edit(&id_of(&cleo, &note, "Milk")?)?);
        Ok(())
    }

    #[test]
    fn a_checklist_made_before_its_note_was_filed_in_goes_along() -> TestResult {
        let (ann, ben, area) = household()?;
        let note = checklist(&ann, "Shop\nMilk", None)?;
        // Written in the Area since, so Ben's vector passes the Lines.
        Notes::new(&ann).capture("Bread", Some(&area), None)?;
        exchange(&ben, &ann)?;
        Notes::new(&ann).file(&note, &area)?;
        exchange(&ben, &ann)?;
        assert_eq!(shown(&ben, &note)?, ["Milk"]);
        Ok(())
    }

    #[test]
    fn two_renames_of_one_line_are_a_conflict_and_a_later_check_wins() -> TestResult {
        let phone = clocked(1)?;
        let desk = clocked(2)?;
        let note = checklist(&phone, "Shop\nMilk\n✓ Eggs", None)?;
        send(&phone, &desk)?;
        let milk = id_of(&phone, &note, "Milk")?;
        let eggs = id_of(&phone, &note, "Eggs")?;
        Checklists::new(&phone).rename(&milk, "Oat milk")?;
        Checklists::new(&desk).rename(&milk, "Soy milk")?;
        Checklists::new(&desk).uncheck(&eggs)?;
        Checklists::new(&phone).uncheck(&eggs)?;
        Checklists::new(&phone).check(&eggs)?;
        send(&phone, &desk)?;
        send(&desk, &phone)?;
        let conflicts = desk.conflicts()?;
        assert_eq!(conflicts.len(), 1);
        let conflict = &conflicts[0];
        assert_eq!(conflict.entity, milk);
        assert_eq!(conflict.unit, note_keys::TEXT);
        assert_eq!(conflict.entity_type, keys::TYPE_LINE);
        assert!(conflict.resolvable());
        let texts: Vec<String> = conflict.versions.iter().map(|v| v.text().text).collect();
        assert_eq!(texts, ["Soy milk", "Oat milk"]);
        assert!(desk.may_resolve(&milk, &conflict.entity_type)?);
        desk.resolve_conflict(conflict, &conflict.versions[1])?;
        send(&desk, &phone)?;
        assert!(phone.conflicts()?.is_empty());
        assert_eq!(shown(&phone, &note)?, ["Oat milk", "[x] Eggs"]);
        assert_eq!(shown(&desk, &note)?, shown(&phone, &note)?);
        Ok(())
    }

    #[test]
    fn a_checklist_goes_out_in_format_2_signed_or_encrypted_and_arrives_as_one() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let note = checklist(&ann, "Trip\nBath\n  Soap\n✓ Tent", None)?;
        let item = ann.sent_item_of(&note, false)?;
        let to = ben
            .own_key_record()?
            .verified_box_key()
            .ok_or("Ben's X25519 key")?;
        let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
        let key = ann.signing_key()?;
        for file in [
            crate::sent::encode(&item, &key, &[], &mut random)?,
            crate::sent::encode(&item, &key, &[to], &mut random)?,
        ] {
            let outer: serde_json::Value = serde_json::from_slice(&file)?;
            assert_eq!(outer["format"], 2);
            let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
            let lines: Vec<(String, bool, bool)> = opened
                .item
                .lines
                .iter()
                .flatten()
                .map(|l| (l.text.clone(), l.indent, l.checked))
                .collect();
            assert_eq!(
                lines,
                [
                    ("Bath".to_string(), false, false),
                    ("Soap".to_string(), true, false),
                    ("Tent".to_string(), false, true),
                ]
            );
        }
        let got = ben.accept_sent_item(&item, false)?.ok_or("accepted")?;
        assert!(
            Notes::new(&ben)
                .by_id(&got)?
                .ok_or("the Note")?
                .is_unsorted()
        );
        assert_eq!(shown(&ben, &got)?, ["Bath", "  Soap", "[x] Tent"]);
        let plain = Notes::new(&ann).capture("Plain", None, None)?.id;
        let file = crate::sent::encode(&ann.sent_item_of(&plain, false)?, &key, &[], &mut random)?;
        let outer: serde_json::Value = serde_json::from_slice(&file)?;
        assert!(outer.get("format").is_none());
        Ok(())
    }

    #[test]
    fn lines_of_text_and_line_breaks_read_as_the_dart_core_reads_them() {
        assert_eq!(
            split_lines("a\r\nb\rc\n\nd\r\r\n"),
            ["a", "b", "c", "", "d", "", ""]
        );
        let row = |text: &str, indent: bool, checked: bool| {
            Some(ParsedLine {
                text: text.into(),
                indent,
                checked,
            })
        };
        assert_eq!(parse_line("   "), None);
        assert_eq!(parse_line(" \u{a0}Nbsp"), row("Nbsp", true, false));
        assert_eq!(parse_line("\u{a0}Lead"), row("Lead", false, false));
        assert_eq!(parse_line("\u{feff}Bom"), row("Bom", false, false));
        assert_eq!(parse_line("  ✓ Pegs"), row("Pegs", true, true));
        assert_eq!(parse_line("✓ "), row("✓", false, false));
        assert_eq!(parse_line("✓x"), row("✓x", false, false));
        assert_eq!(line_text("a\r\n\nb").ok().as_deref(), Some("a b"));
    }

    // ------------------------------------------ With a Routine (#121)

    /// The wall clock `days` days after the store's now: the tests' clock
    /// moves by seconds, the Agenda by days. Day 0 is 2026-01-01.
    fn on_day(days: i64) -> impl Fn(DateTime<Utc>) -> chrono::NaiveDateTime {
        move |utc| (utc + Duration::days(days)).naive_utc()
    }

    fn time(hour: u32) -> Made<crate::schedule::ClockTime> {
        Ok(crate::schedule::ClockTime::new(hour, 0).ok_or("a time")?)
    }

    fn daily(times: &[crate::schedule::ClockTime]) -> Made<Schedule> {
        let rule = Some(crate::schedule::Recurrence::every(
            crate::schedule::Frequency::Daily,
            1,
        ));
        Ok(Schedule::new(Day::new(2026, 1, 1), times, rule).ok_or("a schedule")?)
    }

    fn spawn(store: &Store, note: &str, schedule: &Schedule) -> Made<String> {
        Ok(Routines::new(store).spawn(note, schedule)?.id)
    }

    /// The Occurrence of `routine` on `days` days on, at `time`.
    fn on(routine: &str, days: i64, time: Option<crate::schedule::ClockTime>) -> RoutineOccurrence {
        RoutineOccurrence {
            routine_id: routine.to_string(),
            occurrence: Occurrence::new(Day::new(2026, 1, 1).add_days(days), time),
        }
    }

    /// The Lines as shown for `at`, `[x]` for one checked.
    fn shown_for(store: &Store, at: &RoutineOccurrence) -> Made<Vec<String>> {
        let list = Checklists::new(store).for_occurrence(at)?;
        Ok(list
            .map(|l| l.lines)
            .unwrap_or_default()
            .iter()
            .map(|l| {
                format!(
                    "{}{}{}",
                    if l.in_group() { "  " } else { "" },
                    if l.checked { "[x] " } else { "" },
                    l.text
                )
            })
            .collect())
    }

    fn done_at(store: &Store, at: &RoutineOccurrence) -> Made<Option<DateTime<Utc>>> {
        Ok(Routines::new(store)
            .by_id(&at.routine_id)?
            .and_then(|r| r.done_at(&at.occurrence)))
    }

    #[test]
    fn each_occurrence_starts_unchecked_and_is_done_once_every_line_is_checked() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = checklist(&ann, "Gym bag\nTowel\nShoes\n  Laces\nBottle", None)?;
        let lists = Checklists::new(&ann);
        lists.check(&id_of(&ann, &note, "Towel")?)?;
        let routine = spawn(&ann, &note, &daily(&[])?)?;
        let today = on(&routine, 0, None);
        // Its own checks show for no Occurrence.
        assert_eq!(
            shown_for(&ann, &today)?,
            ["Towel", "Shoes", "  Laces", "Bottle"]
        );
        lists.check_for(&id_of(&ann, &note, "Towel")?, &today)?;
        lists.check_for(&id_of(&ann, &note, "Laces")?, &today)?;
        assert_eq!(
            shown_for(&ann, &today)?,
            ["[x] Towel", "[x] Shoes", "  [x] Laces", "Bottle"]
        );
        assert_eq!(done_at(&ann, &today)?, None);
        lists.check_for(&id_of(&ann, &note, "Bottle")?, &today)?;
        let done = done_at(&ann, &today)?;
        assert!(done.is_some());
        let rows = Routines::new(&ann).agenda(&on_day(0))?.today_rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].done(), done);
        let list = lists.for_occurrence(&today)?.ok_or("a Checklist")?;
        assert_eq!(list.on.as_ref(), Some(&today));
        // The next one starts afresh, with nothing written for it.
        let before = ann.all_entries()?.len();
        let tomorrow = on(&routine, 1, None);
        assert_eq!(
            shown_for(&ann, &tomorrow)?,
            ["Towel", "Shoes", "  Laces", "Bottle"]
        );
        assert_eq!(done_at(&ann, &tomorrow)?, None);
        assert_eq!(ann.all_entries()?.len(), before);
        // Every check lies on the Routine; no Line's own check moved.
        assert_eq!(
            lists.by_note(&note)?.ok_or("a Checklist")?.checked_count(),
            1
        );
        // Unchecking a Line undoes the Done; checking it off by hand
        // still works.
        lists.uncheck_for(&id_of(&ann, &note, "Laces")?, &today)?;
        assert_eq!(
            shown_for(&ann, &today)?,
            ["[x] Towel", "Shoes", "  Laces", "[x] Bottle"]
        );
        assert_eq!(done_at(&ann, &today)?, None);
        Routines::new(&ann).mark_done(&routine, &tomorrow.occurrence, None)?;
        assert!(done_at(&ann, &tomorrow)?.is_some());
        // "Uncheck all" for one Occurrence.
        lists.check_for(&id_of(&ann, &note, "Shoes")?, &today)?;
        assert!(done_at(&ann, &today)?.is_some());
        lists.uncheck_all_for(&note, &today)?;
        assert_eq!(
            shown_for(&ann, &today)?,
            ["Towel", "Shoes", "  Laces", "Bottle"]
        );
        assert_eq!(done_at(&ann, &today)?, None);
        Ok(())
    }

    #[test]
    fn a_line_added_after_done_leaves_it_done_and_no_line_is_never_done() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = checklist(&ann, "Shop\nMilk\nEggs", None)?;
        let routine = spawn(&ann, &note, &daily(&[])?)?;
        let lists = Checklists::new(&ann);
        let today = on(&routine, 0, None);
        lists.check_for(&id_of(&ann, &note, "Milk")?, &today)?;
        lists.check_for(&id_of(&ann, &note, "Eggs")?, &today)?;
        let done = done_at(&ann, &today)?;
        assert!(done.is_some());
        let bread = lists.add(&note, "Bread", None, false)?;
        assert_eq!(done_at(&ann, &today)?, done);
        assert_eq!(shown_for(&ann, &today)?, ["[x] Milk", "[x] Eggs", "Bread"]);
        // Checked again since, the Line added counts.
        lists.uncheck_for(&id_of(&ann, &note, "Eggs")?, &today)?;
        lists.check_for(&id_of(&ann, &note, "Eggs")?, &today)?;
        assert_eq!(done_at(&ann, &today)?, None);
        lists.delete(&bread)?;
        assert!(done_at(&ann, &today)?.is_some());
        lists.restore(&bread)?;
        assert_eq!(done_at(&ann, &today)?, None);
        // No Line, no Done, whatever a modified app marks.
        let empty = checklist(&ann, "Empty", None)?;
        let idle = spawn(&ann, &empty, &daily(&[])?)?;
        let field = keys::check_for(&today.occurrence, "line:x");
        ann.append(&idle, &field, Some("1"), None)?;
        assert_eq!(done_at(&ann, &on(&idle, 0, None))?, None);
        let milk = id_of(&ann, &note, "Milk")?;
        assert!(lists.check_for(&milk, &on(&idle, 0, None)).is_err());
        // Back to a plain Note and made again, what was Done stays.
        let tidy = checklist(&ann, "Tidy\nDesk\nBin", None)?;
        let chores = spawn(&ann, &tidy, &daily(&[])?)?;
        let at = on(&chores, 0, None);
        lists.check_for(&id_of(&ann, &tidy, "Desk")?, &at)?;
        lists.check_for(&id_of(&ann, &tidy, "Bin")?, &at)?;
        let done = done_at(&ann, &at)?;
        lists.to_plain(&tidy)?;
        assert_eq!(done_at(&ann, &at)?, done);
        lists.make(&tidy)?;
        assert_eq!(done_at(&ann, &at)?, done);
        assert_eq!(shown_for(&ann, &at)?, ["Desk", "Bin"]);
        Ok(())
    }

    #[test]
    fn checks_no_app_writes_are_passed_over() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = checklist(&ann, "Shop\nMilk", None)?;
        let other = checklist(&ann, "Other\nTea", None)?;
        let routine = spawn(&ann, &note, &daily(&[])?)?;
        let today = on(&routine, 0, None);
        let key = today.occurrence.key();
        let milk = id_of(&ann, &note, "Milk")?;
        let prefix = keys::CHECK_PREFIX;
        // As a modified app would.
        for field in [
            format!("{prefix}{key}"),
            format!("{prefix}@{milk}"),
            format!("{prefix}2026-1-1@{milk}"),
            format!("{prefix}{key}@line:"),
            format!("{prefix}{key}@{milk}@x"),
            format!("{prefix}{key}@note:x"),
            keys::check_for(&today.occurrence, &id_of(&ann, &other, "Tea")?),
        ] {
            ann.append(&routine, &field, Some("2026-01-01T09:00:00.000Z"), None)?;
        }
        assert_eq!(shown_for(&ann, &today)?, ["Milk"]);
        assert_eq!(done_at(&ann, &today)?, None);
        let read = LineChecks::of(&ann, &routine)?;
        assert_eq!(read.by_occurrence.keys().collect::<Vec<_>>(), [&key]);
        // An Occurrence not due, or a Routine of another Note, takes no
        // check.
        let lists = Checklists::new(&ann);
        assert!(
            lists
                .check_for(&milk, &on(&routine, 0, Some(time(8)?)))
                .is_err()
        );
        let elsewhere = spawn(&ann, &other, &daily(&[])?)?;
        assert!(lists.check_for(&milk, &on(&elsewhere, 0, None)).is_err());
        assert_eq!(lists.for_occurrence(&on("routine:none", 0, None))?, None);
        Ok(())
    }

    #[test]
    fn shown_now_is_the_first_not_done_of_the_latest_day_up_to_today() -> TestResult {
        let ann = device(1, "Ann")?;
        let note = checklist(&ann, "Pills\nBlue\nRed", None)?;
        let lists = Checklists::new(&ann);
        let blue = id_of(&ann, &note, "Blue")?;
        let red = id_of(&ann, &note, "Red")?;
        lists.check(&blue)?;
        // Without a Routine, its own checks.
        assert_eq!(lists.occurrence_now(&note, &on_day(0))?, None);
        let now = lists.shown_now(&note, &on_day(0))?.ok_or("a Checklist")?;
        assert_eq!((now.checked_count(), now.on), (1, None));
        let routine = spawn(&ann, &note, &daily(&[time(8)?, time(20)?])?)?;
        let morning = on(&routine, 0, Some(time(8)?));
        assert_eq!(
            lists.occurrence_now(&note, &on_day(0))?,
            Some(morning.clone())
        );
        let now = lists.shown_now(&note, &on_day(0))?.ok_or("a Checklist")?;
        assert_eq!(now.checked_count(), 0);
        lists.check_for(&blue, &morning)?;
        lists.check_for(&red, &morning)?;
        let evening = on(&routine, 0, Some(time(20)?));
        assert_eq!(
            lists.occurrence_now(&note, &on_day(0))?,
            Some(evening.clone())
        );
        Routines::new(&ann).mark_done(&routine, &evening.occurrence, None)?;
        assert_eq!(lists.occurrence_now(&note, &on_day(0))?, Some(evening));
        assert_eq!(
            lists.occurrence_now(&note, &on_day(1))?,
            Some(on(&routine, 1, Some(time(8)?)))
        );
        // A weekly one stays on its day until the next comes; before its
        // first day, the first.
        let weekly = checklist(&ann, "Bins\nPaper", None)?;
        let rule = crate::schedule::Recurrence::parse("FREQ=WEEKLY;BYDAY=MO")?;
        let mondays = spawn(
            &ann,
            &weekly,
            &Schedule::new(Day::new(2026, 1, 5), &[], Some(rule)).ok_or("weekly")?,
        )?;
        for (days, monday) in [(0, 4), (4, 4), (10, 4), (11, 11)] {
            assert_eq!(
                lists.occurrence_now(&weekly, &on_day(days))?,
                Some(on(&mondays, monday, None)),
                "{days}"
            );
        }
        // Of two Routines, the earlier.
        let later = checklist(&ann, "Trip\nTent", None)?;
        let once = |day: i64| Schedule::new(Day::new(2026, 1, 1).add_days(day), &[], None);
        let trip = spawn(&ann, &later, &once(9).ok_or("once")?)?;
        let soon = spawn(&ann, &later, &once(4).ok_or("once")?)?;
        for (days, routine, day) in [(0, &soon, 4), (6, &soon, 4), (9, &trip, 9)] {
            assert_eq!(
                lists.occurrence_now(&later, &on_day(days))?,
                Some(on(routine, day, None)),
                "{days}"
            );
        }
        // In the trash, the Routine leaves it to its own checks.
        Routines::new(&ann).trash(&routine)?;
        assert_eq!(lists.occurrence_now(&note, &on_day(0))?, None);
        let now = lists.shown_now(&note, &on_day(0))?.ok_or("a Checklist")?;
        assert_eq!(now.checked_count(), 1);
        // Many at once, as one by one.
        let plain = Notes::new(&ann).capture("Plain", None, None)?.id;
        let ids = [note, weekly, later, plain];
        let at_once = lists.of_notes_now(&ids, &on_day(1))?;
        assert_eq!(at_once.len(), 3);
        for (id, list) in &at_once {
            assert_eq!(
                Some(list),
                lists.shown_now(id, &on_day(1))?.as_ref(),
                "{id}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_check_another_persons_device_slipped_in_counts_for_nothing() -> TestResult {
        let (ann, ben, _) = household()?;
        let note = checklist(&ann, "Shop\nMilk", None)?;
        let routine = spawn(&ann, &note, &daily(&[])?)?;
        let today = on(&routine, 0, None);
        // As a modified app would: Ben slips a check into Ann's own list.
        let field = keys::check_for(&today.occurrence, &id_of(&ann, &note, "Milk")?);
        let forged = ben.append(&routine, &field, Some("2026-01-01T09:00:00.000Z"), None)?;
        let mut report = ImportReport::default();
        ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
        let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
        assert!(held.contains(&forged.id()));
        assert_eq!(shown_for(&ann, &today)?, ["Milk"]);
        assert_eq!(done_at(&ann, &today)?, None);
        Ok(())
    }

    #[test]
    fn in_a_shared_area_a_member_checks_for_everyone() -> TestResult {
        let (ann, ben, area) = household()?;
        let note = checklist(&ann, "Shop\nMilk\nEggs", Some(&area))?;
        let routine = spawn(&ann, &note, &daily(&[])?)?;
        exchange(&ben, &ann)?;
        assert_eq!(ben.area_of(&routine)?, Some(area));
        let today = on(&routine, 0, None);
        Checklists::new(&ben).check_for(&id_of(&ben, &note, "Milk")?, &today)?;
        Checklists::new(&ann).check_for(&id_of(&ann, &note, "Eggs")?, &today)?;
        assert_eq!(done_at(&ann, &today)?, None);
        exchange(&ben, &ann)?;
        exchange(&ben, &ann)?;
        let done = done_at(&ann, &today)?;
        assert!(done.is_some());
        assert_eq!(done_at(&ben, &today)?, done);
        assert_eq!(shown_for(&ann, &today)?, ["[x] Milk", "[x] Eggs"]);
        // The next Occurrence starts afresh for everyone.
        assert_eq!(shown_for(&ben, &on(&routine, 1, None))?, ["Milk", "Eggs"]);
        assert_eq!(
            Checklists::new(&ben).occurrence_now(&note, &on_day(1))?,
            Some(on(&routine, 1, None))
        );
        Ok(())
    }
}
