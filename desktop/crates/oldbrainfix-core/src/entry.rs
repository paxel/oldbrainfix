use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

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
        }
    }
}

/// An entry on the wire: `seq` is a local handle and stays out.
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
