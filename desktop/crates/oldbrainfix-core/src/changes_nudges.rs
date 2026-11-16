//! Who waits for changes by bundle (#109), as the Dart core's
//! `changes_nudges.dart`: the people changes travel with by bundles
//! lacking a change that no bundle for them carried yet, as the Changes
//! Nudge covers them (CONTEXT.md, [`crate::nudges::changes_nudge_plan`]).
//!
//! What a bundle for a person carries stays what lies beyond what their
//! Devices said they hold ([`Store::entries_for_person`]), so a bundle
//! that never arrived loses nothing. Beside it this Device keeps, per
//! person and Shared Area, how far the log had come when the last bundle
//! for them was sent: a local setting (ADR 0008), the mark of the log
//! ([`Store::current_seq`]) by Area, each only ever growing.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::entry::Entry;
use crate::offers::{TRANSPORT_PREFIX, Transport};
use crate::store::{MomentRow, Store};
use crate::{Error, Result, parse_time};

const PREFIX: &str = "changesSent:";

impl Store {
    /// How far the log had come, by Area, when changes were last sent to
    /// `person`; an Area missing was never sent to.
    pub fn changes_sent_to(&self, person: &str) -> Result<BTreeMap<String, i64>> {
        let key = format!("{PREFIX}{}", self.resolve_person(person)?);
        let Some(raw) = self.local_setting(&key)? else {
            return Ok(BTreeMap::new());
        };
        let Ok(Value::Object(json)) = serde_json::from_str::<Value>(&raw) else {
            return Ok(BTreeMap::new());
        };
        Ok(json
            .into_iter()
            .filter_map(|(area, seq)| seq.as_i64().map(|seq| (area, seq)))
            .collect())
    }

    /// A bundle for `person` went out, written when the log stood at
    /// `seq` ([`Store::current_seq`], read before writing it): the changes
    /// up to there in every Area shared with them count as sent. The
    /// user's tap on the share sheet or the save dialog, never the writing
    /// alone, says it went.
    pub fn note_changes_sent(&self, person: &str, seq: i64) -> Result<()> {
        let mut areas = HashSet::new();
        for device in self.devices_of(person)? {
            areas.extend(self.areas_shared_with(&device)?);
        }
        self.note_sent(person, &areas, seq)
    }

    /// The changes up to `seq` in `area` count as sent to `person`: the
    /// Area was offered to them, or taken from their offer (#108), so
    /// they hold its history.
    pub fn note_area_sent(&self, person: &str, area: &str, seq: i64) -> Result<()> {
        self.note_sent(person, &HashSet::from([area.to_string()]), seq)
    }

    fn note_sent(&self, person: &str, areas: &HashSet<String>, seq: i64) -> Result<()> {
        if areas.is_empty() {
            return Ok(());
        }
        let mut sent = self.changes_sent_to(person)?;
        for area in areas {
            let mark = sent.entry(area.clone()).or_insert(0);
            if seq > *mark {
                *mark = seq;
            }
        }
        let key = format!("{PREFIX}{}", self.resolve_person(person)?);
        let json = serde_json::to_string(&sent).map_err(|e| Error::Invalid(e.to_string()))?;
        self.set_local_setting(&key, &json)
    }

    /// Whether changes travel with `person` by bundles: so chosen, not
    /// blocked, and holding a role in an Area shared with them.
    pub fn sends_by_bundle(&self, person: &str) -> Result<bool> {
        let id = self.resolve_person(person)?;
        Ok(!self.is_own_person(&id)?
            && !self.is_blocked(&id)?
            && self.transport_with(&id)? == Some(Transport::Bundle)
            && !self.devices_of(&id)?.is_empty())
    }

    /// The changes a bundle for `person` carries that no bundle for them
    /// carried yet: of [`Store::entries_for_person`] those beyond what was
    /// sent in their Area ([`Store::changes_sent_to`]), never one their
    /// own Devices wrote. In (device, dseq) order.
    pub fn unsent_to(&self, person: &str) -> Result<Vec<Entry>> {
        let id = self.resolve_person(person)?;
        let sent = self.changes_sent_to(&id)?;
        let theirs: HashSet<String> = self.devices_of(&id)?.into_iter().collect();
        let mut scopes: HashMap<String, String> = HashMap::new();
        let mut out = Vec::new();
        for e in self.entries_for_person(&id)? {
            if theirs.contains(&e.device) {
                continue;
            }
            let scope = match scopes.get(&e.entity) {
                Some(s) => s.clone(),
                None => {
                    let s = self.scope_of(&e.entity)?;
                    scopes.insert(e.entity.clone(), s.clone());
                    s
                }
            };
            if e.seq > sent.get(&scope).copied().unwrap_or(0) {
                out.push(e);
            }
        }
        Ok(out)
    }

    /// The people waiting for changes by bundle
    /// ([`Store::sends_by_bundle`]), by Profile id, with the time of the
    /// first change each lacks ([`Store::unsent_to`]). A change this
    /// Device wrote counts from when it was written; any other from when
    /// it came here, the Moment it came with: another Device's clock never
    /// moves the Changes Nudge.
    pub fn waiting_for_changes(&self) -> Result<BTreeMap<String, DateTime<Utc>>> {
        let mut people = Vec::new();
        for (key, value) in self.local_settings_with_prefix(TRANSPORT_PREFIX)? {
            if value == Transport::Bundle.name() {
                let id = self.resolve_person(&key[TRANSPORT_PREFIX.len()..])?;
                if !people.contains(&id) {
                    people.push(id);
                }
            }
        }
        people.sort();
        let own = self.device_id()?;
        // Newest first, read once and only when another Device's change
        // waits.
        let mut moments: Option<Vec<MomentRow>> = None;
        let mut out = BTreeMap::new();
        for person in people {
            if !self.sends_by_bundle(&person)? {
                continue;
            }
            let mut first: Option<DateTime<Utc>> = None;
            for e in self.unsent_to(&person)? {
                let mut at = parse_time(&e.recorded)?;
                if e.device != own {
                    if moments.is_none() {
                        moments = Some(self.moment_rows()?);
                    }
                    // The last Moment recorded before it came.
                    if let Some(m) = moments.iter().flatten().find(|m| m.seq < e.seq) {
                        at = parse_time(&m.at)?;
                    }
                }
                if first.is_none_or(|f| at < f) {
                    first = Some(at);
                }
            }
            if let Some(first) = first {
                out.insert(person, first);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
