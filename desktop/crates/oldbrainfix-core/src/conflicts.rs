//! Concurrent edits, as the Dart core's `Conflicts` finds and resolves
//! them: two Devices changed the same thing without either having seen
//! the other's change. Latest wins keeps showing one of them, the same
//! on every Device, until the user picks the version that stays.
//!
//! Every edit to a field it applies to carries its causal context
//! ([`Entry::ctx`]): how far the writer had seen each other Device that
//! wrote the field. An entry has seen another when it comes later from
//! the same Device or its context reaches that entry's dseq; an entry
//! without a context has been seen by everything newer by latest wins.
//! A Conflict is a field whose entries nobody has seen since (its
//! heads) hold more than one value. It is read from the log, so a pick
//! that has seen every head ends it on every Device it reaches.
//!
//! A Note's or Section's text, the name of an Area, a Topic or a Named
//! Place and a Named Place's radius each conflict on their own; a
//! Routine's start, times and rule conflict together as its schedule.
//! A text carries its format in the same value (ADR 0018), so text and
//! format conflict as one; a Conflict one of whose versions is in a
//! format this version does not know is left for a Device that knows it.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chrono::Duration;

use crate::entry::{Entry, WireEntry};
use crate::notes::keys as note_keys;
use crate::places::keys as place_keys;
use crate::routines::keys as routine_keys;
use crate::store::{Store, utf16_cmp};
use crate::text_format::StoredText;
use crate::{Error, Result};

/// The unit a Routine's schedule fields conflict as.
pub const SCHEDULE_UNIT: &str = "schedule";

/// A Named Place's radius, the Dart core's `NamedPlaceKeys.radius`.
pub const RADIUS: &str = "radius";

/// Every field concurrent edits are a Conflict on.
pub const CONFLICT_FIELDS: [&str; 6] = [
    note_keys::TEXT,
    place_keys::NAME,
    RADIUS,
    routine_keys::START,
    routine_keys::TIMES,
    routine_keys::RULE,
];

/// The unit `field` conflicts as, or `None` when concurrent edits to it
/// simply go by latest wins.
pub fn conflict_unit(field: &str) -> Option<&'static str> {
    match field {
        note_keys::TEXT => Some(note_keys::TEXT),
        place_keys::NAME => Some(place_keys::NAME),
        RADIUS => Some(RADIUS),
        routine_keys::START | routine_keys::TIMES | routine_keys::RULE => Some(SCHEDULE_UNIT),
        _ => None,
    }
}

/// The fields of `unit`.
pub fn conflict_unit_fields(unit: &str) -> Vec<&str> {
    if unit == SCHEDULE_UNIT {
        routine_keys::SCHEDULE_FIELDS.to_vec()
    } else {
        vec![unit]
    }
}

/// [`Entry::ctx`] for `seen`: `<device>:<dseq>` pairs by device.
pub fn encode_context(seen: &[(String, i64)]) -> String {
    let mut pairs: Vec<&(String, i64)> = seen.iter().collect();
    pairs.sort_by(|a, b| utf16_cmp(&a.0, &b.0));
    pairs
        .iter()
        .map(|(d, n)| format!("{d}:{n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// What [`encode_context`] wrote; pairs that are not pairs are skipped.
pub fn decode_context(ctx: &str) -> BTreeMap<String, i64> {
    let mut seen = BTreeMap::new();
    for pair in ctx.split(' ') {
        let Some(colon) = pair.rfind(':') else {
            continue;
        };
        if colon == 0 {
            continue;
        }
        if let Ok(dseq) = pair[colon + 1..].parse::<i64>() {
            seen.insert(pair[..colon].to_string(), dseq);
        }
    }
    seen
}

/// Whether `later` was written having seen `earlier`: what its context
/// reaches. An entry without a context (a first write, or written before
/// contexts were kept) has been seen by everything newer by latest wins,
/// and sees nothing that has one, so it never starts a Conflict.
fn saw(later: &Entry, earlier: &Entry) -> bool {
    if later.device == earlier.device {
        return earlier.dseq < later.dseq;
    }
    if let Some(ctx) = &later.ctx
        && decode_context(ctx)
            .get(&earlier.device)
            .copied()
            .unwrap_or(0)
            >= earlier.dseq
    {
        return true;
    }
    earlier.ctx.is_none() && Store::latest_wins(earlier, later) == Ordering::Less
}

/// One side of a Conflict: what its unit held for the Device that wrote
/// `entry`, the newest edit holding it nobody has seen since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictVersion {
    pub entry: Entry,
    /// The value of every field of the unit, in the unit's order; `None`
    /// when cleared or never written.
    pub values: Vec<(String, Option<String>)>,
}

impl ConflictVersion {
    /// The value of a unit of one field.
    pub fn value(&self) -> Option<&str> {
        self.values.first().and_then(|(_, v)| v.as_deref())
    }

    /// The value of `field`.
    pub fn get(&self, field: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| k == field)
            .and_then(|(_, v)| v.as_deref())
    }

    /// The text and format of a text unit's value.
    pub fn text(&self) -> StoredText {
        StoredText::read(self.value())
    }
}

/// Concurrent edits to one unit of one entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub entity: String,
    /// The field, or [`SCHEDULE_UNIT`].
    pub unit: String,
    /// The entity's `$type`.
    pub entity_type: String,
    /// At least two, the newest by latest wins first.
    pub versions: Vec<ConflictVersion>,
}

impl Conflict {
    /// Whether this version may pick one: not when a version is a text in
    /// a format it does not know, which picking another would overwrite
    /// and keeping could not show as it is (ADR 0018).
    pub fn resolvable(&self) -> bool {
        self.unit != note_keys::TEXT || self.versions.iter().all(|v| v.text().editable)
    }
}

impl Store {
    /// Every open Conflict, by entity and unit.
    pub fn conflicts(&self) -> Result<Vec<Conflict>> {
        let mut devices: BTreeMap<(String, &str), BTreeSet<String>> = BTreeMap::new();
        for (entity, field, device) in self.writers_of(&CONFLICT_FIELDS)? {
            if let Some(unit) = conflict_unit(&field) {
                devices.entry((entity, unit)).or_default().insert(device);
            }
        }
        let units: Vec<(String, &str)> = devices
            .into_iter()
            .filter(|(_, d)| d.len() > 1)
            .map(|(k, _)| k)
            .collect();
        self.conflicts_on(units)
    }

    /// The open Conflicts `entries` touch, by entity and unit: what an
    /// import brought.
    pub fn conflicts_among(&self, entries: &[WireEntry]) -> Result<Vec<Conflict>> {
        let units: BTreeSet<(String, &str)> = entries
            .iter()
            .filter_map(|e| conflict_unit(&e.field).map(|u| (e.entity.clone(), u)))
            .collect();
        self.conflicts_on(units.into_iter().collect())
    }

    fn conflicts_on(&self, mut units: Vec<(String, &str)>) -> Result<Vec<Conflict>> {
        units.sort_by(|a, b| utf16_cmp(&a.0, &b.0).then_with(|| utf16_cmp(a.1, b.1)));
        let mut out = Vec::new();
        for (entity, unit) in units {
            if let Some(conflict) = self.conflict_on(&entity, unit)? {
                out.push(conflict);
            }
        }
        Ok(out)
    }

    /// The open Conflict on `unit` of `entity`, or `None`.
    pub fn conflict_on(&self, entity: &str, unit: &str) -> Result<Option<Conflict>> {
        let Some(entity_type) = self.current(entity, "$type")? else {
            return Ok(None);
        };
        let fields = conflict_unit_fields(unit);
        let written: Vec<Entry> = self
            .history(entity)?
            .into_iter()
            .filter(|e| fields.contains(&e.field.as_str()))
            .collect();
        let devices: BTreeSet<&str> = written.iter().map(|e| e.device.as_str()).collect();
        if devices.len() < 2 {
            return Ok(None);
        }
        let mut heads: Vec<usize> = (0..written.len())
            .filter(|&h| {
                !written
                    .iter()
                    .enumerate()
                    .any(|(x, e)| x != h && saw(e, &written[h]))
            })
            .collect();
        heads.sort_by(|&a, &b| Store::latest_wins(&written[b], &written[a]));
        let mut versions: Vec<ConflictVersion> = Vec::new();
        for h in heads {
            let head = &written[h];
            let values: Vec<(String, Option<String>)> = fields
                .iter()
                .map(|field| {
                    // What the head's writer had of the field: the last it
                    // had of it, the newest by latest wins among several.
                    let had: Vec<&Entry> = written
                        .iter()
                        .enumerate()
                        .filter(|(x, e)| e.field == *field && (*x == h || saw(head, e)))
                        .map(|(_, e)| e)
                        .collect();
                    let mut newest: Option<&Entry> = None;
                    for (i, e) in had.iter().enumerate() {
                        if had.iter().enumerate().any(|(j, y)| j != i && saw(y, e)) {
                            continue;
                        }
                        if newest.is_none_or(|n| Store::latest_wins(e, n) == Ordering::Greater) {
                            newest = Some(e);
                        }
                    }
                    (field.to_string(), newest.and_then(|e| e.value.clone()))
                })
                .collect();
            // A text written before its format was recorded is the same
            // as that text marked plain.
            let same = |v: &ConflictVersion| {
                if unit == note_keys::TEXT {
                    v.text() == StoredText::read(values.first().and_then(|(_, v)| v.as_deref()))
                } else {
                    v.values == values
                }
            };
            if !versions.iter().any(same) {
                versions.push(ConflictVersion {
                    entry: head.clone(),
                    values,
                });
            }
        }
        if versions.len() < 2 {
            return Ok(None);
        }
        Ok(Some(Conflict {
            entity: entity.to_string(),
            unit: unit.to_string(),
            entity_type,
            versions,
        }))
    }

    /// Keeps `kept`: writes every field of the unit again as it holds
    /// them, having seen every version, so the Conflict ends wherever
    /// this reaches. Dated after every edit of the unit, so a Device
    /// whose clock ran ahead cannot keep showing its value.
    /// A Conflict that is not [`Conflict::resolvable`] here is refused.
    pub fn resolve_conflict(&self, conflict: &Conflict, kept: &ConflictVersion) -> Result<()> {
        if !conflict.resolvable() {
            return Err(Error::Invalid(format!(
                "a version of {} is in a format not known here",
                conflict.entity
            )));
        }
        let fields = conflict_unit_fields(&conflict.unit);
        let mut date = self.now();
        for e in self.history(&conflict.entity)? {
            if !fields.contains(&e.field.as_str()) {
                continue;
            }
            let after = e.date_at + Duration::milliseconds(1);
            if after > date {
                date = after;
            }
        }
        self.batch(|| {
            for field in &fields {
                self.append(&conflict.entity, field, kept.get(field), Some(date))?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Notes;
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::ticking;
    use crate::sync::wire;
    use std::collections::HashMap;

    fn device(seed: u8) -> Result<Store> {
        Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
    }

    /// Everything `from` holds that `to` lacks, taken in.
    fn send(from: &Store, to: &Store) -> Result<Vec<WireEntry>> {
        to.apply_entries(
            &wire(&from.entries_since(&to.version_vector()?)?),
            &from.key_records()?,
            None,
            &mut ImportReport::default(),
        )
    }

    #[test]
    fn contexts_are_written_and_read_by_device() {
        let seen = vec![("b".to_string(), 2), ("a".to_string(), 10)];
        assert_eq!(encode_context(&seen), "a:10 b:2");
        assert_eq!(encode_context(&[]), "");
        let read = decode_context("a:10 b:2");
        assert_eq!(read.get("a"), Some(&10));
        assert_eq!(read.get("b"), Some(&2));
        assert!(decode_context("").is_empty());
        assert!(decode_context("junk :3 c:x").is_empty());
    }

    #[test]
    fn concurrent_edits_conflict_and_a_pick_ends_it_on_both() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("nees mor sauce", None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "Needs more sauce")?;
        Notes::new(&b).edit(&note.id, "Needs more hot sauce")?;
        let came = send(&b, &a)?;
        assert_eq!(a.conflicts_among(&came)?.len(), 1);
        send(&a, &b)?;
        for store in [&a, &b] {
            let conflicts = store.conflicts()?;
            assert_eq!(conflicts.len(), 1);
            let values: Vec<_> = conflicts[0]
                .versions
                .iter()
                .map(|v| v.text().text)
                .collect();
            // Each store has its own clock: the first Device's edit is
            // the later one.
            assert_eq!(values, ["Needs more sauce", "Needs more hot sauce"]);
        }
        let conflict = a.conflicts()?.remove(0);
        a.resolve_conflict(&conflict, &conflict.versions[1])?;
        assert!(a.conflicts()?.is_empty());
        let came = send(&a, &b)?;
        assert!(b.conflicts_among(&came)?.is_empty());
        assert!(b.conflicts()?.is_empty());
        assert_eq!(
            Notes::new(&b).by_id(&note.id)?.map(|n| n.text).as_deref(),
            Some("Needs more hot sauce")
        );
        Ok(())
    }

    #[test]
    fn a_text_and_its_format_conflict_as_one_left_for_a_device_that_knows_it() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("Milk", None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "Oat milk")?;
        b.append(
            &note.id,
            note_keys::TEXT,
            Some(r#"{"format":"x-future","text":"Oat milk"}"#),
            None,
        )?;
        send(&b, &a)?;
        let conflict = a.conflicts()?.remove(0);
        let formats: Vec<String> = conflict.versions.iter().map(|v| v.text().format).collect();
        assert_eq!(formats, ["plain", "x-future"]);
        assert!(!conflict.resolvable());
        for kept in &conflict.versions {
            assert!(a.resolve_conflict(&conflict, kept).is_err());
        }
        assert_eq!(a.conflicts()?.len(), 1);
        Ok(())
    }

    #[test]
    fn a_conflict_over_a_formatted_text_is_resolved_as_over_a_plain_one() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture_as("Milk", crate::text_format::MARKDOWN, None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "**Milk**")?;
        Notes::new(&b).edit(&note.id, "*Milk*")?;
        send(&b, &a)?;
        let conflict = a.conflicts()?.remove(0);
        assert!(conflict.resolvable());
        a.resolve_conflict(&conflict, &conflict.versions[0])?;
        assert!(a.conflicts()?.is_empty());
        let format = Notes::new(&a).by_id(&note.id)?.map(|n| n.text_format);
        assert_eq!(format.as_deref(), Some(crate::text_format::MARKDOWN));
        Ok(())
    }

    #[test]
    fn a_text_written_before_its_format_is_the_same_as_it_marked_plain() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("Milk", None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "Oat milk")?;
        b.append(&note.id, note_keys::TEXT, Some("Oat milk"), None)?;
        send(&b, &a)?;
        assert!(a.conflicts()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_text_with_version_1_written_out_is_the_same_as_without() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("Milk", None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "Oat milk")?;
        b.append(
            &note.id,
            note_keys::TEXT,
            Some(r#"{"format":"plain","text":"Oat milk","v":1}"#),
            None,
        )?;
        send(&b, &a)?;
        assert!(a.conflicts()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_conflict_with_a_later_structure_is_not_resolved_here() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("Milk", None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "Oat milk")?;
        b.append(
            &note.id,
            note_keys::TEXT,
            Some(r#"{"format":"plain","text":"Oat milk","v":2}"#),
            None,
        )?;
        send(&b, &a)?;
        let conflict = a.conflicts()?.remove(0);
        assert!(!conflict.resolvable());
        assert!(
            a.resolve_conflict(&conflict, &conflict.versions[0])
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn a_sequential_edit_or_the_same_value_is_no_conflict() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("First", None, None)?;
        send(&a, &b)?;
        Notes::new(&b).edit(&note.id, "Second")?;
        send(&b, &a)?;
        Notes::new(&a).edit(&note.id, "Third")?;
        Notes::new(&b).edit(&note.id, "Third")?;
        send(&a, &b)?;
        send(&b, &a)?;
        assert!(a.conflicts()?.is_empty());
        assert!(b.conflicts()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_log_without_contexts_has_no_conflicts() -> Result<()> {
        let a = device(1)?;
        let b = device(2)?;
        let note = Notes::new(&a).capture("First", None, None)?;
        send(&a, &b)?;
        Notes::new(&a).edit(&note.id, "A")?;
        Notes::new(&b).edit(&note.id, "B")?;
        let old = device(3)?;
        for e in a.all_entries()?.into_iter().chain(b.all_entries()?) {
            old.insert(&WireEntry {
                ctx: None,
                ..e.wire()
            })?;
        }
        assert!(old.conflicts()?.is_empty());
        assert!(old.version_vector()? != HashMap::new());
        Ok(())
    }
}
