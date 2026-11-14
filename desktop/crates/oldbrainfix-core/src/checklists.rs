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
use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};

use crate::entry::{Entry, dart_iso, parse_time};
use crate::markdown::BlockKind;
use crate::notes::{Note, Notes, keys as note_keys};
use crate::routines::{Routine, Routines};
use crate::schedule::{Day, Occurrence, Schedule, WallClock};
use crate::sent::keys as sent_keys;
use crate::shopping_lists::ListOnAgenda;
use crate::store::{Fields, Store, utf16_cmp};
use crate::tallies::Tallies;
use crate::text_format::{self, StoredText, VERSION};
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

    /// The id prefix, before the Note's id, of the user's own "Show on
    /// the Agenda" choice for a Checklist (#143): `agenda:<note id>`,
    /// outside every Area like an Override, so it is sealed under the
    /// personal key and goes only to the user's own Devices.
    pub const AGENDA_PREFIX: &str = "agenda:";
    /// On that entity: how the Checklist shows on the Agenda (#219), a
    /// [`crate::ListOnAgenda`]'s code: [`AGENDA_OFF`] while it stays off,
    /// `count` for one row with the number of its open Lines; absent,
    /// none, `lines` or any other value for every open Line under its
    /// name.
    pub const AGENDA: &str = "agenda";
    pub const AGENDA_OFF: &str = "off";
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
    pub const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    /// The middle digit of [`DIGITS`] (a test checks it).
    const MID: u8 = b'V';

    fn digit(c: u8) -> Option<usize> {
        DIGITS.iter().position(|d| *d == c)
    }

    /// The digit of value `d`, none past the last.
    fn symbol(d: usize) -> Option<char> {
        DIGITS.get(d).map(|c| char::from(*c))
    }

    fn mid() -> char {
        char::from(MID)
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
            if let Some(next) = symbol(d + 1) {
                return format!("{}{next}", &a[..i]);
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
        while bb
            .get(n)
            .is_some_and(|c| ab.get(n).copied().unwrap_or(b'0') == *c)
        {
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
        let (Some(da), Some(db)) = (da, bb.get(n).and_then(|c| digit(*c))) else {
            return after(a);
        };
        if db <= da {
            return after(a);
        }
        // `da` and `db` are digits, so their middle and `da` are too;
        // were they not, the key after `a` still sorts right.
        if db - da > 1 {
            let Some(middle) = symbol((da + db) / 2) else {
                return after(a);
            };
            return format!("{prefix}{middle}");
        }
        if bb.len() > n + 1 {
            return b[..n + 1].to_string();
        }
        let rest = if n < ab.len() { &a[n + 1..] } else { "" };
        let Some(da) = symbol(da) else {
            return after(a);
        };
        format!("{prefix}{da}{}", after(rest))
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

    #[cfg(test)]
    mod tests;
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

    /// Whether it shows checked by a check made on `day` by `wall`.
    pub fn checked_on(&self, day: Day, wall: &WallClock) -> bool {
        self.checked && self.checked_at.is_some_and(|at| Day::of(wall(at)) == day)
    }
}

/// An Occurrence of the Routine `routine_id`: one a Checklist is
/// checked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineOccurrence {
    pub routine_id: String,
    pub occurrence: Occurrence,
}

/// A Checklist on today's Agenda (#219) and how it shows there: every
/// open Line under its name for [`ListOnAgenda::Lines`], its name with the
/// count for [`ListOnAgenda::Count`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChecklistAgendaRow {
    pub list: Checklist,
    pub shows: ListOnAgenda,
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

    /// How many Lines do not show checked: "3 of 7 open" on the Agenda.
    pub fn open_count(&self) -> usize {
        self.count() - self.checked_count()
    }

    pub fn all_checked(&self) -> bool {
        self.lines.iter().all(|l| l.checked)
    }
}

/// A deleted Line of a Checklist or Shopping List (#268), for its folded
/// "Removed Lines": it can be restored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedLine {
    /// Entity id, `line:<id>`.
    pub id: String,
    pub text: String,
    /// When it was deleted.
    pub deleted: DateTime<Utc>,
}

/// The deleted Lines of the Note `note_id` of the list `making`, those a
/// restore brings back, the last deleted first; none without a list.
pub(crate) fn deleted_lines_of(
    store: &Store,
    note_id: &str,
    making: Option<&str>,
) -> Result<Vec<DeletedLine>> {
    let Some(making) = making else {
        return Ok(Vec::new());
    };
    let mut found = Vec::new();
    for id in store.entities_of_type(keys::TYPE_LINE)? {
        let fields = store.current_fields(&id)?;
        if fields.get(keys::NOTE) != Some(note_id) || fields.get(keys::MADE) != Some(making) {
            continue;
        }
        let Some(Ok(deleted)) = fields.get(keys::TRASHED).map(parse_time) else {
            continue;
        };
        found.push(DeletedLine {
            text: StoredText::read(fields.get(keys::TEXT)).text,
            id,
            deleted,
        });
    }
    found.sort_by(|a, b| b.deleted.cmp(&a.deleted).then_with(|| a.id.cmp(&b.id)));
    Ok(found)
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
    parse_line_in(row, false)
}

/// [`parse_line`] of a text in the format `markdown` says, as the Dart
/// core's `parseLine`: in a formatted text, the mark of a bulleted or
/// numbered item or a quote goes first, so "- milk", "1. milk" and
/// "> milk" become "milk", and "  - milk" goes under the line above.
pub fn parse_line_in(row: &str, markdown: bool) -> Option<ParsedLine> {
    let mut text = dart_trim(if markdown {
        without_item_mark(row)
    } else {
        row
    });
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

/// `row` without the mark it starts with as a bulleted or numbered item
/// or a quote of formatted text; as it is when it is none of them.
fn without_item_mark(row: &str) -> &str {
    match crate::markdown::parse(row).first() {
        Some(block)
            if matches!(
                block.kind,
                BlockKind::Bullet | BlockKind::Numbered | BlockKind::Quote
            ) =>
        {
            row.get(block.content_start..block.end).unwrap_or(row)
        }
        _ => row,
    }
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
    /// Line belongs to, a making of a Checklist of a Note of `note_ids`,
    /// not deleted, but those added after it and not checked then. Never
    /// one without a Line.
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
            if !of_checklist(store, note, made)? {
                continue;
            }
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

/// Whether the Note `note` was ever a Checklist of the making `made`:
/// the Lines of a Shopping List count for no Occurrence.
fn of_checklist(store: &Store, note: &str, made: &str) -> Result<bool> {
    let kind = keys::kind_of(made);
    Ok(store
        .history(note)?
        .iter()
        .any(|e| e.field == keys::KIND && e.value.as_deref() == Some(kind.as_str())))
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

/// What a new Line holds, as [`write_line`] writes it, and when it is
/// written: `None` for now.
pub(crate) struct NewLine<'a> {
    pub text: &'a str,
    pub order: &'a str,
    pub indent: bool,
    pub checked: bool,
    pub received: Option<&'a str>,
    pub at: Option<DateTime<Utc>>,
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
    write_line_as(store, id, note_id, making, line)
}

/// Writes a new Line as [`write_line`] does, under the id `id`.
pub(crate) fn write_line_as(
    store: &Store,
    id: String,
    note_id: &str,
    making: &str,
    line: &NewLine,
) -> Result<String> {
    let now = line.at.unwrap_or_else(|| store.now());
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
        Ok(self.making(note_id)?.is_some())
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

    /// The Checklists on today's Agenda (#143, #219): each not in the
    /// trash nor merged, with no Routine or Reminder out of the trash,
    /// that this person does not keep [`ListOnAgenda::Off`] and that has a
    /// Line open, with how it shows ([`Checklists::on_agenda`]); oldest
    /// capture first. One with a Routine shows as its Occurrence instead.
    /// Each list says how many Lines are open of how many
    /// ([`Checklist::open_count`], [`Checklist::count`]); the apps show one
    /// of a locked Area set to Masked as a row that tells nothing of it,
    /// as a Shopping List's. Reading writes nothing.
    pub fn agenda_rows(&self) -> Result<Vec<ChecklistAgendaRow>> {
        self.agenda_rows_with(None)
    }

    /// [`Checklists::agenda_rows`]; with `checked_today`, the wall clock
    /// today is read on, a list whose open Lines were all checked today
    /// shows too, its Lines struck for the day (#276). Reading writes
    /// nothing.
    pub fn agenda_rows_with(
        &self,
        checked_today: Option<&WallClock>,
    ) -> Result<Vec<ChecklistAgendaRow>> {
        let notes = Notes::new(self.store);
        let routines = Routines::new(self.store);
        let mut spawned = HashSet::new();
        for routine in routines.all()? {
            if let Some(note) = routines.note(&routine)? {
                spawned.insert(note.id);
            }
        }
        let mut own = HashMap::new();
        let mut shown = Vec::new();
        for id in self.store.entities_of_type(note_keys::TYPE_NOTE)? {
            if self.making(&id)?.is_none() || spawned.contains(&id) {
                continue;
            }
            let shows = self.shows(&id, &mut own)?;
            if shows == ListOnAgenda::Off {
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
        let mut lists = self.of_notes(&ids)?;
        // The clock is read only when asked for.
        let today = checked_today.map(|wall| (Day::today(self.store.now(), wall), wall));
        let listed = |list: &Checklist| {
            list.open_count() > 0
                || today
                    .is_some_and(|(day, wall)| list.lines.iter().any(|l| l.checked_on(day, wall)))
        };
        let mut out = Vec::new();
        for (note, shows) in shown {
            if let Some(list) = lists.remove(&note.id)
                && listed(&list)
            {
                out.push(ChecklistAgendaRow { list, shows });
            }
        }
        Ok(out)
    }

    /// How the Checklist of `note_id` shows on today's Agenda for this
    /// person (#219), as one of the user's own Devices chose last: off,
    /// one row with its count, or, for a list nobody chose for or a value
    /// no app writes, every open Line, as #143's "on" showed it. What
    /// another person's Device wrote counts for nothing, so in a Shared
    /// Area each person decides for their own Agenda.
    pub fn on_agenda(&self, note_id: &str) -> Result<ListOnAgenda> {
        self.shows(note_id, &mut HashMap::new())
    }

    fn shows(&self, note_id: &str, own: &mut HashMap<String, bool>) -> Result<ListOnAgenda> {
        let entity = format!("{}{note_id}", keys::AGENDA_PREFIX);
        let value = self.store.own_current(&entity, keys::AGENDA, own)?;
        Ok(match value.as_deref() {
            Some(keys::AGENDA_OFF) => ListOnAgenda::Off,
            Some(code) if code == ListOnAgenda::Count.code() => ListOnAgenda::Count,
            _ => ListOnAgenda::Lines,
        })
    }

    /// Whether "Show on the Agenda" applies to the Note `note_id`: a
    /// Checklist not in the trash nor merged, with no Routine or Reminder
    /// out of the trash; one with a Routine shows as its Occurrence.
    pub fn may_show_on_agenda(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed()
            && !note.is_merged()
            && self.making(note_id)?.is_some()
            && Routines::new(self.store).of_note(note_id)?.is_empty())
    }

    /// Sets how the Checklist of `note_id` shows on the Agenda for this
    /// person alone (#219), kept like an Override: on the user's own
    /// Devices only, whatever their role in its Area. A choice it already
    /// shows by writes nothing, as every Device reads no choice as every
    /// open Line.
    pub fn set_on_agenda(&self, note_id: &str, shows: ListOnAgenda) -> Result<()> {
        if self.making(note_id)?.is_none() {
            return Err(Error::Invalid(format!("not a Checklist: {note_id}")));
        }
        if self.on_agenda(note_id)? == shows {
            return Ok(());
        }
        let entity = format!("{}{note_id}", keys::AGENDA_PREFIX);
        self.store
            .append(&entity, keys::AGENDA, Some(shows.code()), None)?;
        Ok(())
    }

    /// Whether this Device may make the Note `note_id` a Checklist: a Note
    /// in the lists, of a text known here and not code, neither a Tally
    /// nor a list already, which it may change.
    pub fn may_make(&self, note_id: &str) -> Result<bool> {
        let Some(note) = Notes::new(self.store).by_id(note_id)? else {
            return Ok(false);
        };
        Ok(!note.is_trashed()
            && !note.is_merged()
            && note.text_editable
            && note.text_format != crate::text_format::CODE
            && parse_list_kind(self.store.current(note_id, keys::KIND)?.as_deref()).is_none()
            && !Tallies::new(self.store).is_tally(note_id)?
            && !crate::sessions::Sessions::new(self.store).is_session_note(note_id)?
            && self.store.may_change(note_id)?)
    }

    /// Makes the Note `note_id` a Checklist: its first line stays its
    /// text, every other line not blank becomes a Line, under the line
    /// above when it starts with a space or a tab, checked when it starts
    /// with "✓ ", in a formatted text without the mark of an item or a
    /// quote ([`parse_line_in`]). Takes it out of Unsorted, as a Tally
    /// does. The text keeps its format.
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
        let markdown = note.text_format == text_format::MARKDOWN;
        let parsed: Vec<ParsedLine> = rows
            .iter()
            .skip(1)
            .filter_map(|r| parse_line_in(r, markdown))
            .collect();
        let making = self.store.new_id()?;
        self.store.batch(|| {
            let s = self.store;
            s.append(note_id, keys::KIND, Some(&keys::kind_of(&making)), None)?;
            if title != note.text {
                let text = StoredText::of(&note.text_format, title, VERSION).encode();
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
                        at: None,
                    },
                )?;
            }
            Ok(())
        })
    }

    /// Turns the Checklist of `note_id` back into a plain Note: every Line
    /// goes into its text, one per line, under its text, a checked one
    /// marked "✓ ", one of a group indented, in the format the text is
    /// in. The Lines and their checks stay in the log.
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
        let text = Checklists::plain_text(&note.text, &list);
        let text = StoredText::of(&note.text_format, &text, VERSION).encode();
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
                    at: None,
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

    /// The deleted Lines of the Checklist of `note_id` that
    /// [`Checklists::restore`] brings back, the last deleted first; none
    /// when it is no Checklist.
    pub fn deleted_lines(&self, note_id: &str) -> Result<Vec<DeletedLine>> {
        deleted_lines_of(self.store, note_id, self.making(note_id)?.as_deref())
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
            while rest.get(pos).is_some_and(Line::in_group) {
                pos += 1;
            }
        }
        let indent = !grouped && rest.get(pos).is_some_and(Line::in_group);
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
        if !routine.has(&on.occurrence) {
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
    // A one-time Occurrence Tomorrow added counts as any other.
    let latest = routines
        .iter()
        .filter_map(|r| {
            let moved = r.moved.iter().map(|o| o.day).filter(|d| *d <= today).max();
            last_day(&r.schedule, today).max(moved)
        })
        .max();
    let mut found: Vec<(&Routine, Occurrence)> = Vec::new();
    for routine in routines {
        let occurrences = match latest {
            Some(day) => routine.on(day),
            None => routine.first_from(today).into_iter().collect(),
        };
        found.extend(occurrences.into_iter().map(|o| (routine, o)));
    }
    found.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| utf16_cmp(&a.0.id, &b.0.id)));
    let picked = match latest {
        None => found.first(),
        Some(_) => found
            .iter()
            .find(|(r, o)| r.done_at(o).is_none() && r.skipped_at(o).is_none())
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
    // A place past the end is the end.
    let (before, after) = lines.split_at_checked(pos).unwrap_or((lines, &[]));
    let a = before
        .iter()
        .rev()
        .find(|l| order::is_valid(l.order.as_deref()))
        .and_then(|l| l.order.clone());
    let b = after
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

/// `text` on one line and trimmed, as a Line holds it; an empty one is
/// refused.
pub(crate) fn line_text(text: &str) -> Result<String> {
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
pub(crate) mod tests;
