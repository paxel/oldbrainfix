//! Tallies (ADR 0028) as the Dart core's `tallies.dart` reads and writes
//! them: a Note that records how much was done. Its fields lie on the
//! Note, each under `tally:<field id>:<key>`, so they are changed as the
//! Note is; its entries are entities of their own, each its author's as a
//! Section is. Every write appends the same entries, in the same order
//! and read from the clock as often, as the Dart core's.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::checklists::{keys as checklist_keys, parse_list_kind};
use crate::entry::{dart_iso, parse_time};
use crate::notes::{Notes, keys as note_keys};
use crate::people::PersonFilter;
use crate::schedule::{Day, WallClock};
use crate::store::{Fields, Store, utf16_cmp};
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
    /// The id of the other field a field multiplied (#200), as stored: no
    /// app writes it any more, but a field it names still counts the sets
    /// of the older entries (#264).
    pub const MULTIPLIES: &str = "multiplies";
    /// A field's kind: [`KIND_CHECK`] for a Check field, absent for a
    /// number.
    pub const KIND: &str = "kind";
    pub const KIND_CHECK: &str = "check";
    /// The length of a bar of the Tally's charts, a period's name, on the
    /// Tally's Note; absent or unknown reads as a day (#264).
    pub const BAR: &str = "tally:bar";
    /// Whether the Tally counts in sets, `true` or `false`, on the
    /// Tally's Note; absent, it does while a field counts sets as a
    /// multiplier did (#264).
    pub const SETS: &str = "tally:sets";
    /// How many sets an entry holds at most.
    pub const MAX_SETS: usize = 99;

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

    /// The value of a field in the set `set` of an entry, counting from
    /// 1: the first under `v:<field id>`, each further one under
    /// `v<set>:<field id>` (#264).
    pub fn set_value(set: usize, field_id: &str) -> String {
        if set == 1 {
            value(field_id)
        } else {
            format!("v{set}:{field_id}")
        }
    }

    /// The set and the field id `key` holds a value of: `v:` for the
    /// first, `v2:` to `v99:`, without a leading zero, for the others.
    pub fn value_of(key: &str) -> Option<(usize, &str)> {
        if let Some(id) = key.strip_prefix(VALUE_PREFIX) {
            return Some((1, id));
        }
        let (set, id) = key.strip_prefix('v')?.split_once(':')?;
        let digits =
            set.len() <= 2 && set.bytes().all(|b| b.is_ascii_digit()) && !set.starts_with('0');
        let set: usize = set.parse().ok().filter(|_| digits)?;
        (set >= 2 && !id.is_empty()).then_some((set, id))
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
    /// The target in the base unit, per bar of the Tally's charts; a field
    /// that counts sets has none.
    pub target: Option<f64>,
    pub rule: TallyRule,
    pub hidden: bool,
    /// Whether it is a Check field (#264): a box per entry, a tick
    /// counting as 1, summed.
    pub check: bool,
    /// The id of the field this one multiplied (#200), or `None`: such a
    /// field counts the sets of the entries that hold no sets of their
    /// own (#264) and counts no value of its own.
    pub multiplies: Option<String>,
    /// The id of the field its setting names, read or not.
    stored: Option<String>,
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
    /// How long a bar of its charts is; its targets count per bar (#264).
    pub bar: TallyPeriod,
    /// Whether its entry form has "+ Set" (#264).
    pub counts_in_sets: bool,
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

    /// The fields shown that count values of their own: all but those
    /// that count sets as a multiplier did (#264). Totals, targets and
    /// charts are theirs.
    pub fn counting(&self) -> Vec<&TallyField> {
        self.fields
            .iter()
            .filter(|f| !f.hidden && f.multiplies.is_none())
            .collect()
    }

    /// The number fields of [`Tally::counting`]: a set is a row across
    /// them.
    pub fn numbers(&self) -> Vec<&TallyField> {
        self.counting().into_iter().filter(|f| !f.check).collect()
    }

    /// The Check fields of [`Tally::counting`]: ticked once per entry,
    /// above its sets.
    pub fn checks(&self) -> Vec<&TallyField> {
        self.counting().into_iter().filter(|f| f.check).collect()
    }

    /// What the entry form starts with: the target of every number field
    /// shown that has one, by field id; a Check field starts unticked.
    pub fn targets(&self) -> BTreeMap<String, f64> {
        self.shown()
            .into_iter()
            .filter(|f| !f.check)
            .filter_map(|f| f.target.map(|t| (f.id.clone(), t)))
            .collect()
    }

    /// The sets of `entry` (#264), each a row of values by field id across
    /// its number fields, hidden ones included, in the base unit; none
    /// when it holds no number. An entry without sets of its own written
    /// that a field counting sets holds a value for reads as that many
    /// sets of its values: the first such field in their order, its value
    /// rounded half away from zero, at least 1 and at most
    /// [`keys::MAX_SETS`].
    pub fn sets_of(&self, entry: &TallyEntry) -> Vec<BTreeMap<String, f64>> {
        let row = |values: &BTreeMap<String, f64>| -> BTreeMap<String, f64> {
            self.fields
                .iter()
                .filter(|f| !f.check && f.multiplies.is_none())
                .filter_map(|f| values.get(&f.id).map(|v| (f.id.clone(), *v)))
                .collect()
        };
        let first = row(&entry.values);
        let mut rows: Vec<BTreeMap<String, f64>> = Vec::new();
        if !first.is_empty() {
            rows.push(first.clone());
        }
        rows.extend(entry.more.iter().map(row).filter(|r| !r.is_empty()));
        if !entry.more.is_empty() || first.is_empty() {
            return rows;
        }
        for field in self.fields.iter().filter(|f| f.multiplies.is_some()) {
            let Some(micro) = entry
                .values
                .get(&field.id)
                .and_then(|v| tally_units::micro(*v))
            else {
                continue;
            };
            let count = usize::try_from(round(micro).clamp(1, keys::MAX_SETS as i64)).unwrap_or(1);
            return vec![first; count];
        }
        rows
    }

    /// What `entry` counts for the field `id`: a Check field its tick, a
    /// number field its rule over the entry's sets, the sum or the
    /// largest (#264); `None` for a field counting sets or a value it
    /// does not hold.
    pub fn counted(&self, entry: &TallyEntry, id: &str) -> Option<f64> {
        self.counted_micro(entry, id)
            .map(|micro| micro as f64 / 1_000_000.0)
    }

    /// [`Tally::counted`] in millionths, sums held at the 64-bit limits,
    /// as in both cores.
    pub fn counted_micro(&self, entry: &TallyEntry, id: &str) -> Option<i64> {
        let field = self.field(id)?;
        if field.multiplies.is_some() {
            return None;
        }
        if field.check {
            return entry.values.get(id).and_then(|v| tally_units::micro(*v));
        }
        let micros: Vec<i64> = self
            .sets_of(entry)
            .iter()
            .filter_map(|set| set.get(id).and_then(|v| tally_units::micro(*v)))
            .collect();
        fold(&micros, field.rule)
    }

    /// Whether the setting of `field` names a shown field other than it:
    /// such a field is never multiplied, so no chain forms.
    fn would_multiply(&self, field: &TallyField) -> bool {
        let other = field.stored.as_deref().and_then(|id| self.field(id));
        !field.hidden && other.is_some_and(|o| o.id != field.id && !o.hidden)
    }

    /// `self` with the multipliers its settings make (#200): a field
    /// whose setting names a shown other field multiplied it, unless that
    /// field's own setting names a shown other field too. A hidden field
    /// multiplies nothing and is multiplied by nothing. Such a field now
    /// counts sets (#264).
    fn multiplied(mut self) -> Tally {
        let read: Vec<Option<String>> = self
            .fields
            .iter()
            .map(|f| {
                let other = f.stored.as_deref().and_then(|id| self.field(id))?;
                (self.would_multiply(f) && !self.would_multiply(other)).then(|| other.id.clone())
            })
            .collect();
        for (field, multiplies) in self.fields.iter_mut().zip(read) {
            if multiplies.is_some() {
                field.target = None;
            }
            field.multiplies = multiplies;
        }
        self
    }
}

/// `micro` millionths as a whole number, rounded half away from zero.
fn round(micro: i64) -> i64 {
    let whole = micro / 1_000_000;
    if (micro % 1_000_000).abs() * 2 < 1_000_000 {
        whole
    } else {
        whole + micro.signum()
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
    /// The values by field id, in the base unit, of its first set and its
    /// Check fields; a field added later has none.
    pub values: BTreeMap<String, f64>,
    /// The values of its further sets (#264), in their order, each by
    /// field id in the base unit; as stored, read as [`Tally::sets_of`]
    /// says.
    pub more: Vec<BTreeMap<String, f64>>,
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

    /// The period named `name`, as the Dart core names it.
    pub fn parse(name: &str) -> Option<TallyPeriod> {
        TallyPeriod::ALL.into_iter().find(|p| p.name() == name)
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
        let shapes = self.shapes()?;
        let Some(shape) = shapes.get(note_id) else {
            return Ok(None);
        };
        let counted = self.counted()?;
        let counted = counted.get(note_id);
        let fields = shape
            .fields
            .iter()
            .map(|field| TallyField {
                has_entries: counted.is_some_and(|c| c.contains(&field.id)),
                ..field.clone()
            })
            .collect();
        let mut tally = Tally {
            note_id: note_id.to_string(),
            fields,
            bar: TallyPeriod::Day,
            counts_in_sets: false,
        }
        .multiplied();
        tally.bar = shape
            .bar
            .as_deref()
            .and_then(TallyPeriod::parse)
            .unwrap_or(TallyPeriod::Day);
        tally.counts_in_sets = match shape.sets.as_deref() {
            Some("true") => true,
            Some("false") => false,
            _ => tally.fields.iter().any(|f| f.multiplies.is_some()),
        };
        Ok(Some(tally))
    }

    /// Whether the Note `note_id` is a Tally: told by its fields alone,
    /// without reading its entries (#216).
    pub fn is_tally(&self, note_id: &str) -> Result<bool> {
        Ok(self.shapes()?.contains_key(note_id))
    }

    /// Every Tally whose Note is in the lists, newest capture first.
    pub fn all(&self) -> Result<Vec<Tally>> {
        let notes: BTreeSet<String> = self.shown_under()?.values().cloned().collect();
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
    /// `target` is in the base unit, per bar; `rule` defaults by the kind
    /// of `unit`. A Note that is a list, such as a Checklist, is never a
    /// Tally too.
    pub fn add_field(
        &self,
        note_id: &str,
        name: &str,
        unit: &str,
        target: Option<f64>,
        rule: Option<TallyRule>,
    ) -> Result<String> {
        self.add_any_field(note_id, name, unit, target, rule, false)
    }

    /// Adds a Check field (#264) as [`Tallies::add_field`] adds a number:
    /// it has no unit, sums its ticks and stays a Check field.
    pub fn add_check(&self, note_id: &str, name: &str, target: Option<f64>) -> Result<String> {
        self.add_any_field(note_id, name, "", target, None, true)
    }

    fn add_any_field(
        &self,
        note_id: &str,
        name: &str,
        unit: &str,
        target: Option<f64>,
        rule: Option<TallyRule>,
        check: bool,
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
        let typed = if check {
            String::new()
        } else {
            unit.trim().to_string()
        };
        let goal = match target {
            Some(target) => Some(encoded(target)?),
            None => None,
        };
        let id = self.store.new_id()?;
        let rule = if check {
            TallyRule::Sum
        } else {
            rule.unwrap_or_else(|| tally_units::default_rule(tally_units::kind_of(&typed)))
        };
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
            if check {
                s.append(
                    note_id,
                    &keys::field(&id, keys::KIND),
                    Some(keys::KIND_CHECK),
                    None,
                )?;
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
        if field.check {
            return Err(Error::Invalid(format!("{field_id} is a Check field")));
        }
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

    /// Sets how the field `field_id` counts the sets of an entry and a
    /// bar with several entries; a Check field always sums.
    pub fn set_rule(&self, note_id: &str, field_id: &str, rule: TallyRule) -> Result<()> {
        if self.require_field(note_id, field_id)?.check {
            return Err(Error::Invalid(format!("{field_id} is a Check field")));
        }
        self.set(note_id, field_id, keys::RULE, Some(rule.name()))
    }

    /// Sets how long a bar of the charts of the Tally of `note_id` is
    /// (#264), on its Note; its targets count per bar.
    pub fn set_bar(&self, note_id: &str, bar: TallyPeriod) -> Result<()> {
        self.set_on_tally(note_id, keys::BAR, bar.name())
    }

    /// Switches whether the Tally of `note_id` counts in sets (#264): its
    /// entry form has "+ Set" then. The entries keep their sets either
    /// way.
    pub fn set_counts_in_sets(&self, note_id: &str, on: bool) -> Result<()> {
        self.set_on_tally(note_id, keys::SETS, if on { "true" } else { "false" })
    }

    /// Writes `value` under `key` on the Note of the Tally `note_id`,
    /// unless it holds that already.
    fn set_on_tally(&self, note_id: &str, key: &str, value: &str) -> Result<()> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let in_lists = note.is_some_and(|n| !n.is_trashed() && !n.is_merged());
        if self.by_note(note_id)?.is_none() || !in_lists {
            return Err(Error::Invalid(format!(
                "not a Tally in the lists: {note_id}"
            )));
        }
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        if self.store.current(note_id, key)?.as_deref() == Some(value) {
            return Ok(());
        }
        self.store.append(note_id, key, Some(value), None).map(drop)
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
        self.add_with(note_id, values, &[], line, at, |_| Ok(()))
    }

    /// [`Tallies::add`] with `values` its first set and its Check
    /// fields' ticks (1), the further sets `more` (#264), each by number
    /// field id, a set that holds nothing left out, and `within` writing
    /// more in the same batch, given the entry's id.
    pub fn add_with(
        &self,
        note_id: &str,
        values: &BTreeMap<String, f64>,
        more: &[BTreeMap<String, f64>],
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
        let sets = encoded_sets(&tally, more)?;
        let text = line.trim();
        if encoded.iter().all(|(_, v)| v.is_none()) && sets.is_empty() && text.is_empty() {
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
            for (i, set) in sets.iter().enumerate() {
                for (field, value) in set {
                    s.append(&id, &keys::set_value(i + 2, field), Some(value), Some(now))?;
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
        self.edit_entry_sets(id, values, None, line, at)
    }

    /// [`Tallies::edit_entry`] with the further sets when `more` is given
    /// (#264), which then replace those it held and take away the values
    /// of the fields that counted its sets as multipliers.
    pub fn edit_entry_sets(
        &self,
        id: &str,
        values: &BTreeMap<String, Option<f64>>,
        more: Option<&[BTreeMap<String, f64>]>,
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
        let sets = match more {
            Some(more) => Some(encoded_sets(&tally, more)?),
            None => None,
        };
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
            if let Some(sets) = &sets {
                for field in &tally.fields {
                    if field.multiplies.is_some()
                        && !values.contains_key(&field.id)
                        && entry.values.contains_key(&field.id)
                    {
                        s.append(id, &keys::value(&field.id), None, None)?;
                    }
                }
                let held = s.current_fields(id)?;
                for set in 2..=keys::MAX_SETS {
                    for field in &tally.fields {
                        let key = keys::set_value(set, &field.id);
                        let want = sets
                            .get(set - 2)
                            .and_then(|row| row.iter().find(|(f, _)| *f == field.id))
                            .map(|(_, v)| v.as_str());
                        if held.get(&key) == want {
                            continue;
                        }
                        s.append(id, &key, want, None)?;
                    }
                }
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
        Ok(Self::entry_from(id, &self.store.current_fields(id)?))
    }

    /// The entries of `ids`, removed or not, by id, read in one query
    /// (#204); what is no entry is left out.
    pub fn entries_by_id<'i>(
        &self,
        ids: impl IntoIterator<Item = &'i str>,
    ) -> Result<HashMap<String, TallyEntry>> {
        Ok(self
            .store
            .current_fields_of(ids)?
            .into_iter()
            .filter_map(|(id, fields)| Some((id.clone(), Self::entry_from(&id, &fields)?)))
            .collect())
    }

    /// The entry `id` its current `fields` hold, or `None`.
    fn entry_from(id: &str, fields: &Fields) -> Option<TallyEntry> {
        if fields.get(keys::TYPE) != Some(keys::TYPE_ENTRY) {
            return None;
        }
        let note_id = fields.get(keys::NOTE)?;
        let at = fields.get(keys::AT).and_then(|v| parse_time(v).ok())?;
        let mut values = BTreeMap::new();
        let mut more: BTreeMap<usize, BTreeMap<String, f64>> = BTreeMap::new();
        for (key, value) in fields.iter() {
            let Some((set, field)) = keys::value_of(key) else {
                continue;
            };
            let Some(number) = tally_units::decode_number(value) else {
                continue;
            };
            let row = if set == 1 {
                &mut values
            } else {
                more.entry(set).or_default()
            };
            row.insert(field.to_string(), number);
        }
        Some(TallyEntry {
            id: id.to_string(),
            note_id: note_id.to_string(),
            at,
            values,
            more: more.into_values().collect(),
            line: fields.get(keys::LINE).unwrap_or("").to_string(),
            trashed: fields.get(keys::TRASHED).and_then(|v| parse_time(v).ok()),
        })
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
    /// bar per the Tally's bar length, or `period` when given, on `wall`,
    /// weeks from Monday, from the first with a value to today's, at most
    /// [`MAX_BUCKETS`] of the latest. Each bar holds the sum or the
    /// largest of what its entries count ([`Tally::counted`]), by the
    /// field's rule; a Check field sums its ticks, a field counting sets
    /// has none. Entries after today stay out.
    pub fn chart(
        &self,
        note_id: &str,
        field_id: &str,
        period: Option<TallyPeriod>,
        wall: &WallClock,
        filter: &PersonFilter,
    ) -> Result<Vec<TallyBucket>> {
        let Some(tally) = self.by_note(note_id)? else {
            return Ok(Vec::new());
        };
        let Some(field) = tally.field(field_id) else {
            return Ok(Vec::new());
        };
        let period = period.unwrap_or(tally.bar);
        let today = Day::today(self.store.now(), wall);
        let last = bucket_start(today, period);
        let mut micros: HashMap<Day, Vec<i64>> = HashMap::new();
        for entry in self.entries(note_id, filter)? {
            let Some(micro) = tally.counted_micro(&entry, field_id) else {
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
                    value: held
                        .and_then(|m| fold(m, field.rule))
                        .map(|micro| micro as f64 / 1_000_000.0),
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

    /// The fields of every Tally, by its Note, in the order they were
    /// added, each as if it had no entries: read from the Notes alone,
    /// not from the entries, and kept until the log changes (#216), as
    /// the Dart core's `_shapes`.
    fn shapes(&self) -> Result<Arc<HashMap<String, Shape>>> {
        self.store.worked_out("tallies:shapes", || {
            let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (holder, note) in self.shown_under()?.iter() {
                holders
                    .entry(note.clone())
                    .or_default()
                    .push(holder.clone());
            }
            let mut shapes = HashMap::new();
            for (note, mut on) in holders {
                on.sort_by(|a, b| utf16_cmp(a, b));
                let mut fields = Vec::new();
                let (mut bar, mut sets) = (None, None);
                for holder in &on {
                    let current = self.store.current_fields(holder)?;
                    fields.extend(self.fields_on(holder, &current)?);
                    if *holder == note {
                        bar = current.get(keys::BAR).map(str::to_string);
                        sets = current.get(keys::SETS).map(str::to_string);
                    }
                }
                if fields.is_empty() {
                    continue;
                }
                fields.sort_by(|a, b| a.added.cmp(&b.added).then_with(|| utf16_cmp(&a.id, &b.id)));
                shapes.insert(note, Shape { fields, bar, sets });
            }
            Ok(shapes)
        })
    }

    /// The fields lying on the Note `holder`, as if none had entries,
    /// its current `fields` given.
    fn fields_on(&self, holder: &str, fields: &Fields) -> Result<Vec<TallyField>> {
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
                let check = get(&id, keys::KIND) == Some(keys::KIND_CHECK);
                Some(TallyField {
                    note_id: holder.to_string(),
                    name,
                    unit: get(&id, keys::UNIT).unwrap_or("").to_string(),
                    target: tally_units::decode_number(get(&id, keys::TARGET)),
                    rule: get(&id, keys::RULE)
                        .and_then(TallyRule::parse)
                        .filter(|_| !check)
                        .unwrap_or(TallyRule::Sum),
                    hidden: get(&id, keys::HIDDEN) == Some("true"),
                    check,
                    multiplies: None,
                    stored: get(&id, keys::MULTIPLIES).map(str::to_string),
                    has_entries: false,
                    added: date,
                    id,
                })
            })
            .collect())
    }

    /// The ids of the fields some entry holds a value for, removed
    /// entries included, by the Note of the Tally the entry shows under:
    /// every entry read in one query, kept until the log changes (#216),
    /// as the Dart core's `_counted`.
    fn counted(&self) -> Result<Arc<HashMap<String, BTreeSet<String>>>> {
        self.store.worked_out("tallies:counted", || {
            let shown_under = self.shown_under()?;
            let mut counted: HashMap<String, BTreeSet<String>> = HashMap::new();
            for (id, fields) in self.store.current_fields_of_type(keys::TYPE_ENTRY)? {
                if let Some(entry) = Self::entry_from(&id, &fields) {
                    let under = shown_under
                        .get(&entry.note_id)
                        .cloned()
                        .unwrap_or(entry.note_id);
                    let ids = counted.entry(under).or_default();
                    ids.extend(entry.values.into_keys());
                    ids.extend(entry.more.into_iter().flat_map(BTreeMap::into_keys));
                }
            }
            Ok(counted)
        })
    }

    /// Every Note with a field of a Tally on it, by the Note it shows
    /// under: itself, or the one it was merged into. Kept until the log
    /// changes (#216).
    fn shown_under(&self) -> Result<Arc<BTreeMap<String, String>>> {
        self.store.worked_out("tallies:shown_under", || {
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
        })
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

/// A Tally as its Notes hold it, read without its entries: its fields,
/// and its bar length and sets switch as stored on its own Note.
struct Shape {
    fields: Vec<TallyField>,
    bar: Option<String>,
    sets: Option<String>,
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
/// a value for no field of it is refused, as a tick other than 1.
fn encoded_values(
    tally: &Tally,
    values: &BTreeMap<String, Option<f64>>,
) -> Result<Vec<(String, Option<String>)>> {
    for (id, value) in values {
        let Some(field) = tally.field(id) else {
            return Err(Error::Invalid(format!("not a field of the Tally: {id}")));
        };
        if field.check && value.is_some_and(|v| v != 1.0) {
            return Err(Error::Invalid("a tick counts as 1".into()));
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

/// The further sets `more` as the log keeps them, each by number field in
/// the order of the fields of `tally`, those that hold nothing left out;
/// a value for a Check field, a field counting sets or no field of it is
/// refused, as are more than [`keys::MAX_SETS`] sets in all.
fn encoded_sets(
    tally: &Tally,
    more: &[BTreeMap<String, f64>],
) -> Result<Vec<Vec<(String, String)>>> {
    let mut sets = Vec::new();
    for set in more {
        for id in set.keys() {
            if !tally
                .field(id)
                .is_some_and(|f| !f.check && f.multiplies.is_none())
            {
                return Err(Error::Invalid(format!("not a number field of it: {id}")));
            }
        }
        let mut row = Vec::new();
        for field in &tally.fields {
            if let Some(value) = set.get(&field.id) {
                row.push((field.id.clone(), encoded(*value)?));
            }
        }
        if !row.is_empty() {
            sets.push(row);
        }
    }
    if sets.len() >= keys::MAX_SETS {
        return Err(Error::Invalid("too many sets".into()));
    }
    Ok(sets)
}

/// The field id in `key` when it is the `part` of a field on a Note.
fn field_id_in(key: &str, part: &str) -> Option<String> {
    let rest = key.strip_prefix(keys::FIELD_PREFIX)?;
    let id = rest.strip_suffix(part)?.strip_suffix(':')?;
    (!id.is_empty() && !id.contains(':')).then(|| id.to_string())
}

/// What `rule` makes of `micros`, or `None` for none.
fn fold(micros: &[i64], rule: TallyRule) -> Option<i64> {
    let (first, rest) = micros.split_first()?;
    let mut held = *first;
    for micro in rest {
        held = match rule {
            TallyRule::Sum => held.saturating_add(*micro),
            TallyRule::Max => held.max(*micro),
        };
    }
    Some(held)
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
mod tests;
