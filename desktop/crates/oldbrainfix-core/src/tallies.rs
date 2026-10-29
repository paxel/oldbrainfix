//! Tallies (ADR 0028) as the Dart core's `tallies.dart` reads and writes
//! them: a Note that records how much was done. Its fields lie on the
//! Note, each under `tally:<field id>:<key>`, so they are changed as the
//! Note is; its entries are entities of their own, each its author's as a
//! Section is. Every write appends the same entries, in the same order
//! and read from the clock as often, as the Dart core's.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Utc};

use crate::checklists::{keys as checklist_keys, parse_list_kind};
use crate::entry::{dart_iso, parse_time};
use crate::notes::{Notes, keys as note_keys};
use crate::people::PersonFilter;
use crate::schedule::{Day, WallClock};
use crate::store::{Store, utf16_cmp};
use crate::tally_units::{self, TallyRule, UnitKind};
use crate::{Error, Result};

/// Field keys of a Tally.
pub mod keys {
    /// What every key of a field on the Note starts with.
    pub const FIELD_PREFIX: &str = "tally:";
    /// A field's name, unit, target, rule and whether it is hidden.
    pub const NAME: &str = "name";
    pub const UNIT: &str = "unit";
    pub const TARGET: &str = "target";
    pub const RULE: &str = "rule";
    pub const HIDDEN: &str = "hidden";

    /// The key of `key` of the field `field_id` on the Note.
    pub fn field(field_id: &str, key: &str) -> String {
        format!("{FIELD_PREFIX}{field_id}:{key}")
    }

    pub const TYPE: &str = "$type";
    pub const TYPE_ENTRY: &str = "tallyEntry";
    /// The id prefix of an entry.
    pub const ENTRY_PREFIX: &str = "tallyEntry:";
    /// The Note of the Tally the entry belongs to.
    pub const NOTE: &str = "note";
    /// When the entry counts for, an instant.
    pub const AT: &str = "at";
    /// The value of a field, under `v:<field id>`, in the base unit.
    pub const VALUE_PREFIX: &str = "v:";

    pub fn value(field_id: &str) -> String {
        format!("{VALUE_PREFIX}{field_id}")
    }

    /// The entry's free text line.
    pub const LINE: &str = "line";
    /// When the entry was removed.
    pub const TRASHED: &str = "trashed";
}

/// How many bars a chart holds at most: the latest ones.
pub const MAX_BUCKETS: usize = 1000;

/// One number a Tally records per entry, with its unit (CONTEXT.md).
#[derive(Debug, Clone, PartialEq)]
pub struct TallyField {
    pub id: String,
    /// The Note the field lies on: the Tally's own, or one merged into it.
    pub note_id: String,
    pub name: String,
    /// A known unit's code ([`tally_units::KNOWN`]) or the unit as typed.
    pub unit: String,
    /// The target in the base unit.
    pub target: Option<f64>,
    pub rule: TallyRule,
    pub hidden: bool,
    /// Whether an entry holds a value for it, a removed one included: its
    /// kind of unit is fixed then.
    pub has_entries: bool,
    /// When it was added: the fields show in that order.
    pub added: DateTime<Utc>,
}

impl TallyField {
    pub fn kind(&self) -> UnitKind {
        tally_units::kind_of(&self.unit)
    }
}

/// A Note that records how much was done (CONTEXT.md).
#[derive(Debug, Clone, PartialEq)]
pub struct Tally {
    pub note_id: String,
    /// Every field, the hidden ones included, in the order they were
    /// added.
    pub fields: Vec<TallyField>,
}

impl Tally {
    /// The fields that show: those not hidden.
    pub fn shown(&self) -> Vec<&TallyField> {
        self.fields.iter().filter(|f| !f.hidden).collect()
    }

    /// The field `id`, hidden or not.
    pub fn field(&self, id: &str) -> Option<&TallyField> {
        self.fields.iter().find(|f| f.id == id)
    }

    /// What the entry form starts with: the target of every field shown
    /// that has one, by field id.
    pub fn targets(&self) -> BTreeMap<String, f64> {
        self.shown()
            .into_iter()
            .filter_map(|f| f.target.map(|t| (f.id.clone(), t)))
            .collect()
    }
}

/// One record of a Tally: values for its fields, a date, a text line and
/// its author, who is the Device that wrote its first `$type`
/// ([`Store::author_of`]).
#[derive(Debug, Clone, PartialEq)]
pub struct TallyEntry {
    /// Entity id, `tallyEntry:<id>`.
    pub id: String,
    /// The Note of the Tally, as stored.
    pub note_id: String,
    /// When it counts for.
    pub at: DateTime<Utc>,
    /// The values by field id, in the base unit; a field added later has
    /// none.
    pub values: BTreeMap<String, f64>,
    pub line: String,
    /// When it was removed.
    pub trashed: Option<DateTime<Utc>>,
}

impl TallyEntry {
    pub fn is_trashed(&self) -> bool {
        self.trashed.is_some()
    }
}

/// How long one bar of a chart is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyPeriod {
    Day,
    Week,
    Month,
    Year,
}

impl TallyPeriod {
    pub const ALL: [TallyPeriod; 4] = [
        TallyPeriod::Day,
        TallyPeriod::Week,
        TallyPeriod::Month,
        TallyPeriod::Year,
    ];

    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            TallyPeriod::Day => "day",
            TallyPeriod::Week => "week",
            TallyPeriod::Month => "month",
            TallyPeriod::Year => "year",
        }
    }
}

/// One bar of a field's chart: the days `start` to `end` and what the
/// field's rule makes of the values in them.
#[derive(Debug, Clone, PartialEq)]
pub struct TallyBucket {
    pub start: Day,
    pub end: Day,
    /// The sum or the largest of the values, in the base unit; `None`
    /// when no entry holds one.
    pub value: Option<f64>,
    /// How many entries hold a value.
    pub entries: usize,
}

/// Making Notes Tallies, changing their fields, recording entries and
/// reading them as tables and charts.
pub struct Tallies<'a> {
    store: &'a Store,
}

impl<'a> Tallies<'a> {
    pub fn new(store: &'a Store) -> Tallies<'a> {
        Tallies { store }
    }

    /// The Tally of the Note `note_id`, with the fields of the Notes
    /// merged into it, or `None` while it has no field.
    pub fn by_note(&self, note_id: &str) -> Result<Option<Tally>> {
        let mut holders: Vec<String> = self
            .shown_under()?
            .into_iter()
            .filter(|(_, under)| under == note_id)
            .map(|(holder, _)| holder)
            .collect();
        if holders.is_empty() {
            return Ok(None);
        }
        holders.sort_by(|a, b| utf16_cmp(a, b));
        let counted = self.counted(note_id)?;
        let mut fields = Vec::new();
        for holder in &holders {
            fields.extend(self.fields_on(holder, &counted)?);
        }
        if fields.is_empty() {
            return Ok(None);
        }
        fields.sort_by(|a, b| a.added.cmp(&b.added).then_with(|| utf16_cmp(&a.id, &b.id)));
        Ok(Some(Tally {
            note_id: note_id.to_string(),
            fields,
        }))
    }

    /// Whether the Note `note_id` is a Tally.
    pub fn is_tally(&self, note_id: &str) -> Result<bool> {
        Ok(self.by_note(note_id)?.is_some())
    }

    /// Every Tally whose Note is in the lists, newest capture first.
    pub fn all(&self) -> Result<Vec<Tally>> {
        let notes: BTreeSet<String> = self.shown_under()?.into_values().collect();
        let mut out = Vec::new();
        for note in Notes::new(self.store).all()? {
            if notes.contains(&note.id)
                && let Some(tally) = self.by_note(&note.id)?
            {
                out.push(tally);
            }
        }
        Ok(out)
    }

    /// Adds a field to the Note `note_id`, which makes it a Tally the
    /// first time and takes it out of Unsorted; returns the field's id.
    /// `target` is in the base unit; `rule` defaults by the kind of
    /// `unit`. A Note that is a list, such as a Checklist, is never a
    /// Tally too.
    pub fn add_field(
        &self,
        note_id: &str,
        name: &str,
        unit: &str,
        target: Option<f64>,
        rule: Option<TallyRule>,
    ) -> Result<String> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let Some(note) = note.filter(|n| !n.is_trashed() && !n.is_merged()) else {
            return Err(Error::Invalid(format!(
                "not a Note in the lists: {note_id}"
            )));
        };
        let kind = self.store.current(note_id, checklist_keys::KIND)?;
        if parse_list_kind(kind.as_deref()).is_some() {
            return Err(Error::Invalid(format!("{note_id} is a list")));
        }
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        let named = checked_name(name)?;
        let typed = unit.trim().to_string();
        let goal = match target {
            Some(target) => Some(encoded(target)?),
            None => None,
        };
        let id = self.store.new_id()?;
        let rule = rule.unwrap_or_else(|| tally_units::default_rule(tally_units::kind_of(&typed)));
        self.store.batch(|| {
            let s = self.store;
            s.append(note_id, &keys::field(&id, keys::NAME), Some(&named), None)?;
            s.append(note_id, &keys::field(&id, keys::UNIT), Some(&typed), None)?;
            s.append(
                note_id,
                &keys::field(&id, keys::RULE),
                Some(rule.name()),
                None,
            )?;
            if let Some(goal) = &goal {
                s.append(note_id, &keys::field(&id, keys::TARGET), Some(goal), None)?;
            }
            if !note.spawned {
                let at = dart_iso(s.now());
                s.append(note_id, note_keys::SPAWNED, Some(&at), None)?;
            }
            Ok(())
        })?;
        Ok(id)
    }

    /// Renames the field `field_id` of the Tally of `note_id`.
    pub fn rename_field(&self, note_id: &str, field_id: &str, name: &str) -> Result<()> {
        let named = checked_name(name)?;
        self.set(note_id, field_id, keys::NAME, Some(&named))
    }

    /// Sets the unit of the field `field_id`: to one of another kind only
    /// while no entry holds a value for it.
    pub fn set_unit(&self, note_id: &str, field_id: &str, unit: &str) -> Result<()> {
        let field = self.require_field(note_id, field_id)?;
        let typed = unit.trim();
        if field.has_entries && tally_units::kind_of(typed) != field.kind() {
            return Err(Error::Invalid(format!(
                "the kind of unit of {field_id} is fixed by its entries"
            )));
        }
        self.set(note_id, field_id, keys::UNIT, Some(typed))
    }

    /// Sets the target of the field `field_id`, in the base unit; `None`
    /// takes it away.
    pub fn set_target(&self, note_id: &str, field_id: &str, target: Option<f64>) -> Result<()> {
        let value = match target {
            Some(target) => Some(encoded(target)?),
            None => None,
        };
        self.set(note_id, field_id, keys::TARGET, value.as_deref())
    }

    /// Sets how the field `field_id` counts a day with several entries.
    pub fn set_rule(&self, note_id: &str, field_id: &str, rule: TallyRule) -> Result<()> {
        self.set(note_id, field_id, keys::RULE, Some(rule.name()))
    }

    /// Hides the field `field_id`: its values stay stored and return when
    /// it is restored.
    pub fn hide_field(&self, note_id: &str, field_id: &str) -> Result<()> {
        self.set(note_id, field_id, keys::HIDDEN, Some("true"))
    }

    /// Shows the hidden field `field_id` again, with its values.
    pub fn restore_field(&self, note_id: &str, field_id: &str) -> Result<()> {
        self.set(note_id, field_id, keys::HIDDEN, None)
    }

    /// Records an entry in the Tally of `note_id`: `values` by field id,
    /// in the base unit, and a text `line`, counting for `at` or the
    /// store's now. Anyone with a role in its Area may.
    pub fn add(
        &self,
        note_id: &str,
        values: &BTreeMap<String, f64>,
        line: &str,
        at: Option<DateTime<Utc>>,
    ) -> Result<TallyEntry> {
        self.add_with(note_id, values, line, at, |_| Ok(()))
    }

    /// [`Tallies::add`], with `within` writing more in the same batch,
    /// given the entry's id.
    pub fn add_with(
        &self,
        note_id: &str,
        values: &BTreeMap<String, f64>,
        line: &str,
        at: Option<DateTime<Utc>>,
        within: impl FnOnce(&str) -> Result<()>,
    ) -> Result<TallyEntry> {
        let tally = self.by_note(note_id)?;
        let note = Notes::new(self.store).by_id(note_id)?;
        let (Some(tally), Some(note)) = (tally, note) else {
            return Err(Error::Invalid(format!(
                "not a Tally in the lists: {note_id}"
            )));
        };
        if note.is_trashed() || note.is_merged() {
            return Err(Error::Invalid(format!(
                "not a Tally in the lists: {note_id}"
            )));
        }
        if !self.store.may_write_in(note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {note_id}"
            )));
        }
        let given: BTreeMap<String, Option<f64>> =
            values.iter().map(|(k, v)| (k.clone(), Some(*v))).collect();
        let encoded = encoded_values(&tally, &given)?;
        let text = line.trim();
        if encoded.iter().all(|(_, v)| v.is_none()) && text.is_empty() {
            return Err(Error::Invalid("an entry holds something".into()));
        }
        let id = format!("{}{}", keys::ENTRY_PREFIX, self.store.new_id()?);
        let now = self.store.now();
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_ENTRY), Some(now))?;
            s.append(&id, keys::NOTE, Some(note_id), Some(now))?;
            let at = dart_iso(at.unwrap_or(now));
            s.append(&id, keys::AT, Some(&at), Some(now))?;
            for (field, value) in &encoded {
                if let Some(value) = value {
                    s.append(&id, &keys::value(field), Some(value), Some(now))?;
                }
            }
            if !text.is_empty() {
                s.append(&id, keys::LINE, Some(text), Some(now))?;
            }
            within(&id)
        })?;
        self.entry_by_id(&id)?
            .ok_or_else(|| Error::Invalid("the recorded entry is gone".into()))
    }

    /// Changes the entry `id`: the values given (`None` takes one away),
    /// the text line and when it counts for. Only what differs is
    /// written; only its author, or a Manager of its Area, may.
    pub fn edit_entry(
        &self,
        id: &str,
        values: &BTreeMap<String, Option<f64>>,
        line: Option<&str>,
        at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let entry = self.require_entry(id)?;
        let under = self
            .shown_under()?
            .get(&entry.note_id)
            .cloned()
            .unwrap_or_else(|| entry.note_id.clone());
        let Some(tally) = self.by_note(&under)? else {
            return Err(Error::Invalid(format!("not a Tally: {id}")));
        };
        let wanted = encoded_values(&tally, values)?;
        self.store.batch(|| {
            let s = self.store;
            if let Some(at) = at
                && at != entry.at
            {
                s.append(id, keys::AT, Some(&dart_iso(at)), None)?;
            }
            for (field, value) in &wanted {
                let held = match entry.values.get(field) {
                    Some(held) => Some(encoded(*held)?),
                    None => None,
                };
                if *value == held {
                    continue;
                }
                s.append(id, &keys::value(field), value.as_deref(), None)?;
            }
            if let Some(line) = line
                && line.trim() != entry.line
            {
                s.append(id, keys::LINE, Some(line.trim()), None)?;
            }
            Ok(())
        })
    }

    /// Removes the entry `id` from the table and the charts; it can be
    /// restored.
    pub fn trash_entry(&self, id: &str) -> Result<()> {
        self.require_entry(id)?;
        let at = dart_iso(self.store.now());
        self.store
            .append(id, keys::TRASHED, Some(&at), None)
            .map(drop)
    }

    /// Brings the removed entry `id` back.
    pub fn restore_entry(&self, id: &str) -> Result<()> {
        if !self.entry_by_id(id)?.is_some_and(|e| e.is_trashed()) {
            return Err(Error::Invalid(format!("not a removed entry: {id}")));
        }
        if !self.store.may_change(id)? {
            return Err(Error::Invalid(format!("may not change {id}")));
        }
        self.store.append(id, keys::TRASHED, None, None).map(drop)
    }

    /// The entry `id`, removed or not.
    pub fn entry_by_id(&self, id: &str) -> Result<Option<TallyEntry>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_ENTRY) {
            return Ok(None);
        }
        let Some(note_id) = fields.get(keys::NOTE) else {
            return Ok(None);
        };
        let Some(at) = fields.get(keys::AT).and_then(|v| parse_time(v).ok()) else {
            return Ok(None);
        };
        let values = fields
            .iter()
            .filter_map(|(key, value)| {
                let field = key.strip_prefix(keys::VALUE_PREFIX)?;
                Some((field.to_string(), tally_units::decode_number(value)?))
            })
            .collect();
        Ok(Some(TallyEntry {
            id: id.to_string(),
            note_id: note_id.to_string(),
            at,
            values,
            line: fields.get(keys::LINE).unwrap_or("").to_string(),
            trashed: fields.get(keys::TRASHED).and_then(|v| parse_time(v).ok()),
        }))
    }

    /// The entries of the Tally of `note_id`, those of the Notes merged
    /// into it included, newest first, without removed ones; `filter`
    /// keeps those of the user, of everyone or of one person.
    pub fn entries(&self, note_id: &str, filter: &PersonFilter) -> Result<Vec<TallyEntry>> {
        let shown_under = self.shown_under()?;
        let mut out = Vec::new();
        for entry in self.all_entries()? {
            let under = shown_under.get(&entry.note_id).unwrap_or(&entry.note_id);
            if !entry.is_trashed() && under == note_id && self.store.wrote_by(&entry.id, filter)? {
                out.push(entry);
            }
        }
        out.sort_by(newest_first);
        Ok(out)
    }

    /// The removed entries of the Tally of `note_id`, those of the Notes
    /// merged into it included, newest first, everyone's; they can be
    /// restored.
    pub fn removed_entries(&self, note_id: &str) -> Result<Vec<TallyEntry>> {
        let shown_under = self.shown_under()?;
        let mut out: Vec<TallyEntry> = self
            .all_entries()?
            .into_iter()
            .filter(|entry| {
                entry.is_trashed()
                    && shown_under.get(&entry.note_id).unwrap_or(&entry.note_id) == note_id
            })
            .collect();
        out.sort_by(newest_first);
        Ok(out)
    }

    /// The chart of the field `field_id` of the Tally of `note_id`: one
    /// bar per `period` on `wall`, weeks from Monday, from the first with
    /// a value to today's, at most [`MAX_BUCKETS`] of the latest. Each bar
    /// holds the sum or the largest of the values in it, by the field's
    /// rule; entries after today stay out.
    pub fn chart(
        &self,
        note_id: &str,
        field_id: &str,
        period: TallyPeriod,
        wall: &WallClock,
        filter: &PersonFilter,
    ) -> Result<Vec<TallyBucket>> {
        let Some(tally) = self.by_note(note_id)? else {
            return Ok(Vec::new());
        };
        let Some(field) = tally.field(field_id) else {
            return Ok(Vec::new());
        };
        let today = Day::today(self.store.now(), wall);
        let last = bucket_start(today, period);
        let mut micros: HashMap<Day, Vec<i64>> = HashMap::new();
        for entry in self.entries(note_id, filter)? {
            let Some(micro) = entry
                .values
                .get(field_id)
                .and_then(|v| tally_units::micro(*v))
            else {
                continue;
            };
            let day = Day::today(entry.at, wall);
            if day > today {
                continue;
            }
            micros
                .entry(bucket_start(day, period))
                .or_default()
                .push(micro);
        }
        let Some(first) = micros.keys().min().copied() else {
            return Ok(Vec::new());
        };
        let mut starts = Vec::new();
        let mut start = last;
        while first <= start && starts.len() < MAX_BUCKETS {
            starts.push(start);
            start = previous_bucket(start, period);
        }
        starts.reverse();
        Ok(starts
            .into_iter()
            .map(|start| {
                let held = micros.get(&start);
                TallyBucket {
                    start,
                    end: next_bucket(start, period).add_days(-1),
                    value: held.and_then(|m| bar(m, field.rule)),
                    entries: held.map_or(0, Vec::len),
                }
            })
            .collect())
    }

    /// Writes `value` under `key` of the field `field_id`, on the Note it
    /// lies on, unless it holds that already.
    fn set(&self, note_id: &str, field_id: &str, key: &str, value: Option<&str>) -> Result<()> {
        let field = self.require_field(note_id, field_id)?;
        if !self.store.may_change(&field.note_id)? {
            return Err(Error::Invalid(format!("may not change {}", field.note_id)));
        }
        let name = keys::field(field_id, key);
        if self.store.current(&field.note_id, &name)?.as_deref() == value {
            return Ok(());
        }
        self.store
            .append(&field.note_id, &name, value, None)
            .map(drop)
    }

    /// The field `field_id` of the Tally of `note_id`.
    fn require_field(&self, note_id: &str, field_id: &str) -> Result<TallyField> {
        self.by_note(note_id)?
            .and_then(|t| t.field(field_id).cloned())
            .ok_or_else(|| Error::Invalid(format!("not a field of {note_id}: {field_id}")))
    }

    /// The entry `id`, not removed, which this Device may change.
    fn require_entry(&self, id: &str) -> Result<TallyEntry> {
        let Some(entry) = self.entry_by_id(id)?.filter(|e| !e.is_trashed()) else {
            return Err(Error::Invalid(format!("not an entry: {id}")));
        };
        if !self.store.may_change(id)? {
            return Err(Error::Invalid(format!("may not change {id}")));
        }
        Ok(entry)
    }

    /// The fields lying on the Note `holder`, with whether `counted` holds
    /// a value for each.
    fn fields_on(&self, holder: &str, counted: &BTreeSet<String>) -> Result<Vec<TallyField>> {
        let fields = self.store.current_fields(holder)?;
        let mut added: Vec<(String, DateTime<Utc>)> = Vec::new();
        for e in self.store.history(holder)? {
            let Some(id) = field_id_in(&e.field, keys::NAME) else {
                continue;
            };
            match added.iter_mut().find(|(held, _)| *held == id) {
                Some((_, date)) => {
                    if e.date_at < *date {
                        *date = e.date_at;
                    }
                }
                None => added.push((id, e.date_at)),
            }
        }
        let get = |id: &str, key: &str| fields.get(&keys::field(id, key));
        Ok(added
            .into_iter()
            .filter_map(|(id, date)| {
                let name = get(&id, keys::NAME)?.to_string();
                Some(TallyField {
                    note_id: holder.to_string(),
                    name,
                    unit: get(&id, keys::UNIT).unwrap_or("").to_string(),
                    target: tally_units::decode_number(get(&id, keys::TARGET)),
                    rule: get(&id, keys::RULE)
                        .and_then(TallyRule::parse)
                        .unwrap_or(TallyRule::Sum),
                    hidden: get(&id, keys::HIDDEN) == Some("true"),
                    has_entries: counted.contains(&id),
                    added: date,
                    id,
                })
            })
            .collect())
    }

    /// The ids of the fields some entry of the Tally of `note_id` holds a
    /// value for, removed entries included.
    fn counted(&self, note_id: &str) -> Result<BTreeSet<String>> {
        let shown_under = self.shown_under()?;
        let mut out = BTreeSet::new();
        for entry in self.all_entries()? {
            if shown_under.get(&entry.note_id).unwrap_or(&entry.note_id) == note_id {
                out.extend(entry.values.into_keys());
            }
        }
        Ok(out)
    }

    /// Every Note with a field of a Tally on it, by the Note it shows
    /// under: itself, or the one it was merged into.
    fn shown_under(&self) -> Result<BTreeMap<String, String>> {
        let notes = Notes::new(self.store);
        let mut under = BTreeMap::new();
        for (entity, _) in self.store.fields_with_prefix(keys::FIELD_PREFIX)? {
            if under.contains_key(&entity) {
                continue;
            }
            if let Some(note) = notes.by_id(&entity)? {
                let end = note.merged_into.unwrap_or_else(|| entity.clone());
                under.insert(entity, end);
            }
        }
        Ok(under)
    }

    /// Every entry, removed ones included.
    fn all_entries(&self) -> Result<Vec<TallyEntry>> {
        let mut out = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_ENTRY)? {
            if let Some(entry) = self.entry_by_id(&id)? {
                out.push(entry);
            }
        }
        Ok(out)
    }
}

/// Newest first, by id on the same moment.
fn newest_first(a: &TallyEntry, b: &TallyEntry) -> std::cmp::Ordering {
    b.at.cmp(&a.at).then_with(|| utf16_cmp(&a.id, &b.id))
}

/// `name` trimmed; an empty one is refused.
fn checked_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::Invalid("a field's name must not be empty".into()));
    }
    Ok(trimmed.to_string())
}

/// `value` as the log keeps it; refused when no field holds it.
fn encoded(value: f64) -> Result<String> {
    tally_units::encode_number(value)
        .ok_or_else(|| Error::Invalid(format!("not a number a field holds: {value}")))
}

/// `values` as the log keeps them, in the order of the fields of `tally`;
/// a value for no field of it is refused.
fn encoded_values(
    tally: &Tally,
    values: &BTreeMap<String, Option<f64>>,
) -> Result<Vec<(String, Option<String>)>> {
    for id in values.keys() {
        if tally.field(id).is_none() {
            return Err(Error::Invalid(format!("not a field of the Tally: {id}")));
        }
    }
    let mut out = Vec::new();
    for field in &tally.fields {
        if let Some(value) = values.get(&field.id) {
            let value = match value {
                Some(value) => Some(encoded(*value)?),
                None => None,
            };
            out.push((field.id.clone(), value));
        }
    }
    Ok(out)
}

/// The field id in `key` when it is the `part` of a field on a Note.
fn field_id_in(key: &str, part: &str) -> Option<String> {
    let rest = key.strip_prefix(keys::FIELD_PREFIX)?;
    let id = rest.strip_suffix(part)?.strip_suffix(':')?;
    (!id.is_empty() && !id.contains(':')).then(|| id.to_string())
}

/// What `rule` makes of `micros`, or `None` for none.
fn bar(micros: &[i64], rule: TallyRule) -> Option<f64> {
    let (first, rest) = micros.split_first()?;
    let mut held = *first;
    for micro in rest {
        held = match rule {
            TallyRule::Sum => held.saturating_add(*micro),
            TallyRule::Max => held.max(*micro),
        };
    }
    Some(held as f64 / 1_000_000.0)
}

/// The first day of the `period` `day` lies in.
fn bucket_start(day: Day, period: TallyPeriod) -> Day {
    match period {
        TallyPeriod::Day => day,
        TallyPeriod::Week => day.add_days(1 - i64::from(day.weekday())),
        TallyPeriod::Month => Day::new(day.year, i64::from(day.month), 1),
        TallyPeriod::Year => Day::new(day.year, 1, 1),
    }
}

/// The first day of the `period` after the one starting on `start`.
fn next_bucket(start: Day, period: TallyPeriod) -> Day {
    match period {
        TallyPeriod::Day => start.add_days(1),
        TallyPeriod::Week => start.add_days(7),
        TallyPeriod::Month => Day::new(start.year, i64::from(start.month) + 1, 1),
        TallyPeriod::Year => Day::new(start.year + 1, 1, 1),
    }
}

/// The first day of the `period` before the one starting on `start`.
fn previous_bucket(start: Day, period: TallyPeriod) -> Day {
    match period {
        TallyPeriod::Day => start.add_days(-1),
        TallyPeriod::Week => start.add_days(-7),
        TallyPeriod::Month => Day::new(start.year, i64::from(start.month) - 1, 1),
        TallyPeriod::Year => Day::new(start.year - 1, 1, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
    };
    use crate::shared::{AreaRole, Invite};
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use crate::{Entry, OwnProfile, Places, Search};
    use chrono::{Duration, NaiveDateTime, TimeZone};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    fn device(seed: u8, name: &str) -> Made<Store> {
        let start = ticking()();
        let clock: crate::Clock = Box::new(move || {
            let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            start + Duration::seconds(n)
        });
        let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    /// The wall clock on UTC.
    fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
        utc.naive_utc()
    }

    fn at(y: i32, m: u32, d: u32, h: u32) -> Made<DateTime<Utc>> {
        Ok(Utc
            .with_ymd_and_hms(y, m, d, h, 0, 0)
            .single()
            .ok_or("a time")?)
    }

    fn everyone() -> PersonFilter {
        PersonFilter::default()
    }

    fn values<S: AsRef<str>>(pairs: &[(S, f64)]) -> BTreeMap<String, f64> {
        pairs
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), *v))
            .collect()
    }

    fn bars(chart: &[TallyBucket]) -> Vec<Option<f64>> {
        chart.iter().map(|b| b.value).collect()
    }

    /// A Tally made from a new Note with `text` and a field of each of
    /// `units`, named after its unit.
    fn tally(
        store: &Store,
        text: &str,
        units: &[&str],
        place: Option<&str>,
    ) -> Made<(String, Vec<String>)> {
        let note = Notes::new(store).capture(text, place, None)?;
        let mut fields = Vec::new();
        for unit in units {
            fields.push(Tallies::new(store).add_field(&note.id, unit, unit, None, None)?);
        }
        Ok((note.id, fields))
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

    /// Ann's Household shared with Ben as a Member.
    fn household() -> Made<(Store, Store, String)> {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        let invite = Invite {
            area: area.clone(),
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            &ben,
            &ann.version_vector()?,
            "Phone",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        let (answer, _) =
            answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
        apply_answer_as(&ben, &answer, true, None)?;
        for (store, peer) in [(&ann, &ben), (&ben, &ann)] {
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
        Ok((ann, ben, area))
    }

    #[test]
    fn a_first_field_makes_a_note_a_tally_and_takes_it_out_of_unsorted() -> TestResult {
        let ann = device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let tallies = Tallies::new(&ann);
        let note = notes.capture("Push-ups", None, None)?;
        assert_eq!(tallies.by_note(&note.id)?, None);
        assert_eq!(notes.unsorted()?.len(), 1);
        let reps = tallies.add_field(&note.id, " Push-ups ", "times", Some(20.0), None)?;
        let weight = tallies.add_field(&note.id, "Weight", "kg", None, None)?;
        assert!(notes.unsorted()?.is_empty());
        let made = tallies.by_note(&note.id)?.ok_or("a Tally")?;
        let ids: Vec<&str> = made.fields.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, [reps.as_str(), weight.as_str()]);
        assert_eq!(made.fields[0].name, "Push-ups");
        assert_eq!(made.fields[0].rule, TallyRule::Sum);
        assert_eq!(made.fields[1].rule, TallyRule::Max);
        assert_eq!(made.targets(), values(&[(&reps, 20.0)]));
        assert!(tallies.add_field(&note.id, " ", "", None, None).is_err());
        Ok(())
    }

    #[test]
    fn a_fields_kind_of_unit_is_fixed_once_it_has_entries() -> TestResult {
        let ann = device(1, "Ann")?;
        let tallies = Tallies::new(&ann);
        let (note, fields) = tally(&ann, "Lifting", &["kg", "reps"], None)?;
        let (weight, other) = (&fields[0], &fields[1]);
        tallies.set_unit(&note, other, "min")?;
        tallies.set_unit(&note, other, "reps")?;
        let entry = tallies.add(&note, &values(&[(weight, 80.0)]), "", None)?;
        tallies.set_unit(&note, weight, "g")?;
        assert!(tallies.set_unit(&note, weight, "km").is_err());
        assert!(tallies.set_unit(&note, weight, "bags").is_err());
        tallies.trash_entry(&entry.id)?;
        assert!(tallies.set_unit(&note, weight, "km").is_err());
        tallies.restore_entry(&entry.id)?;
        assert_eq!(
            tallies.entries(&note, &everyone())?[0].values.get(weight),
            Some(&80.0)
        );
        Ok(())
    }

    #[test]
    fn keys_that_are_no_field_of_a_tally_are_passed_over() -> TestResult {
        let ann = device(1, "Ann")?;
        let (note, fields) = tally(&ann, "Push-ups", &["times"], None)?;
        // As a modified app would.
        for (key, value) in [
            ("tally:name", "Short"),
            ("tally::name", "Empty"),
            ("tally:a:b:name", "Colon"),
        ] {
            ann.append(&note, key, Some(value), None)?;
        }
        let tally = Tallies::new(&ann).by_note(&note)?.ok_or("a Tally")?;
        let ids: Vec<String> = tally.fields.into_iter().map(|f| f.id).collect();
        assert_eq!(ids, fields);
        assert_eq!(Tallies::new(&ann).all()?.len(), 1);
        Ok(())
    }

    #[test]
    fn hidden_values_return_on_restore_and_lines_are_found() -> TestResult {
        let ann = device(1, "Ann")?;
        let tallies = Tallies::new(&ann);
        let (note, fields) = tally(&ann, "Reading", &["pages", "min"], None)?;
        let (pages, minutes) = (&fields[0], &fields[1]);
        let entry = tallies.add(
            &note,
            &values(&[(pages, 12.0), (minutes, 1800.0)]),
            " On the train ",
            None,
        )?;
        assert_eq!(entry.line, "On the train");
        tallies.hide_field(&note, minutes)?;
        let hidden = tallies.by_note(&note)?.ok_or("a Tally")?;
        assert_eq!(hidden.shown().len(), 1);
        tallies.restore_field(&note, minutes)?;
        let chart = tallies.chart(&note, minutes, TallyPeriod::Day, &utc_wall, &everyone())?;
        assert_eq!(bars(&chart), [Some(1800.0)]);
        let found: Vec<String> = Search::new(&ann)
            .notes("TRAIN", None)?
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(found, std::slice::from_ref(&note));
        let before = ann.all_entries()?.len();
        tallies.edit_entry(
            &entry.id,
            &BTreeMap::from([(pages.clone(), Some(12.0))]),
            Some("On the train"),
            None,
        )?;
        assert_eq!(ann.all_entries()?.len(), before);
        tallies.trash_entry(&entry.id)?;
        assert!(Search::new(&ann).notes("train", None)?.is_empty());
        Ok(())
    }

    #[test]
    fn removed_ones_are_listed_apart_a_merged_notes_included() -> TestResult {
        let ann = device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let tallies = Tallies::new(&ann);
        let (note, reps) = tally(&ann, "Push-ups", &["times"], None)?;
        let (other, pages) = tally(&ann, "Reading", &["pages"], None)?;
        let older = tallies.add(
            &note,
            &values(&[(&reps[0], 20.0)]),
            "",
            Some(at(2026, 2, 27, 18)?),
        )?;
        let kept = tallies.add(
            &note,
            &values(&[(&reps[0], 25.0)]),
            "",
            Some(at(2026, 2, 28, 18)?),
        )?;
        let elsewhere = tallies.add(&other, &values(&[(&pages[0], 12.0)]), "", None)?;
        let into = notes.capture("Sport", None, None)?;
        notes.merge(&note, &into.id)?;
        let newer = tallies.add(
            &into.id,
            &values(&[(&reps[0], 5.0)]),
            "",
            Some(at(2026, 3, 1, 18)?),
        )?;
        tallies.trash_entry(&newer.id)?;
        tallies.trash_entry(&older.id)?;
        tallies.trash_entry(&elsewhere.id)?;
        let ids =
            |list: Vec<TallyEntry>| -> Vec<String> { list.into_iter().map(|e| e.id).collect() };
        assert_eq!(
            ids(tallies.removed_entries(&into.id)?),
            [newer.id.clone(), older.id.clone()]
        );
        assert_eq!(ids(tallies.entries(&into.id, &everyone())?), [kept.id]);
        assert!(tallies.removed_entries(&note)?.is_empty());
        tallies.restore_entry(&older.id)?;
        assert_eq!(ids(tallies.removed_entries(&into.id)?), [newer.id]);
        Ok(())
    }

    #[test]
    fn charts_sum_or_take_the_largest_per_day_week_month_and_year() -> TestResult {
        let ann = device(1, "Ann")?;
        let tallies = Tallies::new(&ann);
        let (note, fields) = tally(&ann, "Gym", &["times", "kg"], None)?;
        let (reps, weight) = (&fields[0], &fields[1]);
        // The clock is at Thursday 2026-01-01; its week began on the 29th.
        for (when, r, w) in [
            (at(2025, 12, 29, 8)?, 10.0, 80.0),
            (at(2025, 12, 29, 18)?, 15.0, 82.5),
            (at(2025, 12, 31, 8)?, 20.0, 81.0),
            (at(2026, 1, 1, 8)?, 5.0, 79.0),
            (at(2026, 1, 5, 8)?, 99.0, 99.0),
        ] {
            tallies.add(&note, &values(&[(reps, r), (weight, w)]), "", Some(when))?;
        }
        let chart =
            |field: &str, period| tallies.chart(&note, field, period, &utc_wall, &everyone());
        let days = chart(reps.as_str(), TallyPeriod::Day)?;
        assert_eq!(days[0].start, Day::new(2025, 12, 29));
        assert_eq!(bars(&days), [Some(25.0), None, Some(20.0), Some(5.0)]);
        assert_eq!(days[0].entries, 2);
        let weeks = chart(reps.as_str(), TallyPeriod::Week)?;
        assert_eq!(bars(&weeks), [Some(50.0)]);
        assert_eq!(weeks[0].end, Day::new(2026, 1, 4));
        assert_eq!(
            bars(&chart(reps.as_str(), TallyPeriod::Month)?),
            [Some(45.0), Some(5.0)]
        );
        assert_eq!(
            bars(&chart(weight.as_str(), TallyPeriod::Month)?),
            [Some(82.5), Some(79.0)]
        );
        assert_eq!(
            bars(&chart(reps.as_str(), TallyPeriod::Year)?),
            [Some(45.0), Some(5.0)]
        );
        assert!(chart("none", TallyPeriod::Day)?.is_empty());
        assert_eq!(tallies.entries(&note, &everyone())?.len(), 5);
        Ok(())
    }

    #[test]
    fn everyone_records_entries_and_each_carries_its_author() -> TestResult {
        let (ann, ben, area) = household()?;
        let (note, fields) = tally(&ann, "Push-ups", &["times"], Some(&area))?;
        let reps = &fields[0];
        let own = Tallies::new(&ann).add(&note, &values(&[(reps, 20.0)]), "", None)?;
        exchange(&ben, &ann)?;
        assert_eq!(ben.area_of(&own.id)?, Some(area.clone()));
        let his = Tallies::new(&ben).add(&note, &values(&[(reps, 30.0)]), "Ben", None)?;
        exchange(&ben, &ann)?;
        let tallies = Tallies::new(&ann);
        let ids: Vec<String> = tallies
            .entries(&note, &everyone())?
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, [his.id.clone(), own.id.clone()]);
        assert_eq!(ann.author_of(&his.id)?.map(|p| p.name), Some("Ben".into()));
        assert_eq!(ann.author_of(&own.id)?, None);
        let mine = PersonFilter {
            me: true,
            ..PersonFilter::default()
        };
        let chart = tallies.chart(&note, reps, TallyPeriod::Day, &utc_wall, &mine)?;
        assert_eq!(bars(&chart), [Some(20.0)]);
        let chart = tallies.chart(&note, reps, TallyPeriod::Day, &utc_wall, &everyone())?;
        assert_eq!(bars(&chart), [Some(50.0)]);
        Ok(())
    }

    #[test]
    fn a_member_changes_only_their_own_entries_never_the_fields() -> TestResult {
        let (ann, ben, area) = household()?;
        let (note, fields) = tally(&ann, "Push-ups", &["times"], Some(&area))?;
        let reps = &fields[0];
        let own = Tallies::new(&ann).add(&note, &values(&[(reps, 20.0)]), "", None)?;
        exchange(&ben, &ann)?;
        let by_ben = Tallies::new(&ben);
        let changed = BTreeMap::from([(reps.clone(), Some(1.0))]);
        assert!(by_ben.edit_entry(&own.id, &changed, None, None).is_err());
        assert!(by_ben.rename_field(&note, reps, "Sit-ups").is_err());
        // As a modified app would.
        let forged = [
            ben.append(&own.id, &keys::value(reps), Some("1"), None)?,
            ben.append(&note, &keys::field(reps, keys::NAME), Some("Sit-ups"), None)?,
        ];
        exchange(&ben, &ann)?;
        let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
        assert!(forged.iter().all(|e| held.contains(&e.id())));
        let entry = Tallies::new(&ann).entry_by_id(&own.id)?.ok_or("an entry")?;
        assert_eq!(entry.values.get(reps), Some(&20.0));
        let tally = Tallies::new(&ann).by_note(&note)?.ok_or("a Tally")?;
        assert_eq!(tally.fields[0].name, "times");
        Ok(())
    }

    #[test]
    fn a_tally_recorded_in_before_its_note_was_filed_in_goes_along() -> TestResult {
        let (ann, ben, area) = household()?;
        let (note, fields) = tally(&ann, "Push-ups", &["times"], None)?;
        let entry = Tallies::new(&ann).add(&note, &values(&[(&fields[0], 20.0)]), "", None)?;
        // Written in the Area since, so Ben's vector passes the entry.
        Notes::new(&ann).capture("Milk", Some(&area), None)?;
        exchange(&ben, &ann)?;
        assert_eq!(Tallies::new(&ben).entry_by_id(&entry.id)?, None);
        Notes::new(&ann).file(&note, &area)?;
        exchange(&ben, &ann)?;
        let ids: Vec<String> = Tallies::new(&ben)
            .entries(&note, &everyone())?
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, [entry.id]);
        Ok(())
    }
}
