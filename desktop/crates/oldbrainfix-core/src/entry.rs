use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, Result};

/// The version of the entries' structure this build applies, as the
/// Dart core's `entryVersion` (ADR 0019): an entry with a higher `v` is
/// kept and passed on, never applied.
pub const ENTRY_VERSION: i64 = 1;

/// The version of an entry with these `extra` keys: `v`, 1 when missing.
fn version_of(extra: &Map<String, Value>) -> i64 {
    extra.get("v").and_then(Value::as_i64).unwrap_or(1)
}

/// One immutable fact in the append-only log (ADR 0002), as the store
/// holds it. The current value of a field is the entry with the highest
/// (date, recorded, device, dseq) for its (entity, field) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Local insertion order: a local handle only.
    pub seq: i64,
    /// Origin device id; with `dseq` globally unique.
    pub device: String,
    /// Per-origin-device counter.
    pub dseq: i64,
    /// Entity id, e.g. `note:<id>`.
    pub entity: String,
    /// Field key, e.g. `text`, `$type`.
    pub field: String,
    /// The value; `None` clears the field.
    pub value: Option<String>,
    /// Effective date of the fact, as stored.
    pub date: String,
    /// Author name the recording Device was configured with.
    pub author: String,
    /// Wall clock at recording time, as stored.
    pub recorded: String,
    /// The writing Device's signature over the row (base64).
    pub sig: Option<String>,
    /// The causal context of an edit to a field concurrent edits are a
    /// Conflict on, as the Dart core's `Entry.ctx`: `<device>:<dseq>`
    /// pairs by device, empty when only the writer had written the field;
    /// `None` for a first write, other fields and older entries.
    pub ctx: Option<String>,
    /// The keys a later version added, `v` among them, as the Dart core's
    /// `Entry.extra`: signed with the entry and passed on as they are.
    pub extra: Map<String, Value>,
    /// The entry's JSON as it came, kept when it is not what this version
    /// writes for it, as the Dart core's `Entry.raw`.
    pub raw: Option<String>,
    /// [`Entry::date`] read as an instant.
    pub date_at: DateTime<Utc>,
    /// [`Entry::recorded`] read as an instant.
    pub recorded_at: DateTime<Utc>,
}

impl Entry {
    /// Identity on the wire.
    pub fn id(&self) -> String {
        format!("{}:{}", self.device, self.dseq)
    }

    /// The version of the entry's structure: `v`, 1 when missing.
    pub fn v(&self) -> i64 {
        version_of(&self.extra)
    }

    /// The entry as it travels: what the Dart core's `Entry.toJson`
    /// writes.
    pub fn wire(&self) -> WireEntry {
        WireEntry {
            device: self.device.clone(),
            dseq: self.dseq,
            entity: self.entity.clone(),
            field: self.field.clone(),
            value: self.value.clone(),
            date: self.date.clone(),
            author: self.author.clone(),
            recorded: self.recorded.clone(),
            ctx: self.ctx.clone(),
            sig: self.sig.clone(),
            extra: self.extra.clone(),
            raw: self.raw.clone(),
        }
    }
}

/// An entry on the wire: `seq` is a local handle and stays out. It is
/// written in the canonical form of ADR 0019: the keys of version 1 in
/// their order, then the keys a later version added, sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireEntry {
    pub device: String,
    pub dseq: i64,
    pub entity: String,
    pub field: String,
    pub value: Option<String>,
    pub date: String,
    pub author: String,
    pub recorded: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
    /// The keys a later version added, `v` among them; sorted, as a
    /// JSON object keeps them here.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
    /// The entry's JSON as it came, kept when it is not what this
    /// version writes for it: passed on byte for byte.
    #[serde(skip)]
    pub raw: Option<String>,
}

impl WireEntry {
    /// The entry `text` holds, as it came (a segment, a bundle, a
    /// Backup); none when it is no entry, or its `v` is no whole number
    /// from 1.
    pub fn parse(text: &str) -> Option<WireEntry> {
        let entry: WireEntry = serde_json::from_str(text).ok()?;
        entry.came_as(text)
    }

    /// The entry `value` holds, in its canonical form: a LAN body keeps
    /// no bytes of its own.
    pub fn from_value(value: &Value) -> Option<WireEntry> {
        let entry: WireEntry = serde_json::from_value(value.clone()).ok()?;
        let text = serde_json::to_string(&entry).ok()?;
        entry.came_as(&text)
    }

    /// This entry, kept as `text` when that is not what this version
    /// writes for it.
    fn came_as(mut self, text: &str) -> Option<WireEntry> {
        if self.extra.contains_key("v") && !self.extra.get("v")?.as_i64().is_some_and(|v| v >= 1) {
            return None;
        }
        self.raw = None;
        if !self.extra.is_empty() || serde_json::to_string(&self).ok()? != text {
            self.raw = Some(text.to_string());
        }
        Some(self)
    }

    /// The version of the entry's structure: `v`, 1 when missing.
    pub fn v(&self) -> i64 {
        version_of(&self.extra)
    }

    /// Whether this version applies the entry: one of a later structure
    /// is only kept and passed on.
    pub fn applies(&self) -> bool {
        self.v() <= ENTRY_VERSION
    }

    /// The entry's JSON line in a segment, a bundle or a Backup: as it
    /// came when kept.
    pub fn line(&self) -> Result<String> {
        match &self.raw {
            Some(raw) => Ok(raw.clone()),
            None => serde_json::to_string(self).map_err(|e| Error::Invalid(e.to_string())),
        }
    }
}

/// Reads a timestamp as the Dart core writes it (ISO 8601 with an
/// offset, `Z` for UTC).
pub fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| Error::Invalid(format!("not a timestamp: {value:?}: {e}")))
}

/// A UTC instant in the form Dart's `DateTime.toIso8601String` gives:
/// milliseconds always, microseconds only when there are any.
pub fn dart_iso(at: DateTime<Utc>) -> String {
    let micros = at.timestamp_subsec_micros();
    let base = at.format("%Y-%m-%dT%H:%M:%S");
    if micros.is_multiple_of(1000) {
        format!("{base}.{:03}Z", micros / 1000)
    } else {
        format!("{base}.{micros:06}Z")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn instants_print_as_dart_prints_them() {
        let at = Utc.with_ymd_and_hms(2026, 1, 2, 9, 0, 0).single();
        let Some(at) = at else { panic!("a valid date") };
        assert_eq!(dart_iso(at), "2026-01-02T09:00:00.000Z");
        let later = at + chrono::Duration::microseconds(500);
        assert_eq!(dart_iso(later), "2026-01-02T09:00:00.000500Z");
        let millis = at + chrono::Duration::milliseconds(123);
        assert_eq!(dart_iso(millis), "2026-01-02T09:00:00.123Z");
        assert_eq!(parse_time(&dart_iso(later)).ok(), Some(later));
        assert!(parse_time("yesterday").is_err());
    }
}
