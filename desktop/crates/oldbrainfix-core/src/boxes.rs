//! Ticking the boxes in the texts of Notes and Sections (ADR 0035), as
//! the Dart core's `boxes.dart` does: each tick is an entry of its own,
//! `box@<id>` = the state, on the entity whose text holds the box, the
//! Note, the Section, or the Note merged into another; the later one by
//! its date wins, and every one stays in the log. In a Shared Area
//! anyone with a role there ticks any box; anywhere else only ticks the
//! user's own Devices wrote count. A tick for an id the text no longer
//! holds counts for nothing.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::entry::{Entry, parse_time};
use crate::notes::Notes;
use crate::store::Store;
use crate::text_boxes::{self, BoxState, BoxTick, ShownBox};
use crate::{Error, Result};

/// The prefix of a tick's field, before the box id.
pub const PREFIX: &str = "box@";

/// The field a tick of the box `id` is written under.
pub fn tick_field(id: &str) -> String {
    format!("{PREFIX}{id}")
}

/// Reading and writing the ticks of boxes.
pub struct Boxes<'a> {
    store: &'a Store,
}

impl<'a> Boxes<'a> {
    pub fn new(store: &'a Store) -> Boxes<'a> {
        Boxes { store }
    }

    /// The latest tick of each box id of `entity` that counts; ids the
    /// text does not hold among them.
    pub fn ticks(&self, entity: &str) -> Result<BTreeMap<String, BoxTick>> {
        let mut latest: BTreeMap<String, (Entry, BoxTick)> = BTreeMap::new();
        for (e, id, tick) in self.counted(entity)? {
            let newer = latest
                .get(&id)
                .is_none_or(|(held, _)| Store::latest_wins(&e, held) == Ordering::Greater);
            if newer {
                latest.insert(id, (e, tick));
            }
        }
        Ok(latest
            .into_iter()
            .map(|(id, (_, tick))| (id, tick))
            .collect())
    }

    /// Every tick of the box `id` in the text of `entity` that counts,
    /// the oldest first.
    pub fn history(&self, entity: &str, id: &str) -> Result<Vec<BoxTick>> {
        let mut held: Vec<(Entry, BoxTick)> = self
            .counted(entity)?
            .into_iter()
            .filter(|(_, box_id, _)| box_id == id)
            .map(|(e, _, tick)| (e, tick))
            .collect();
        held.sort_by(|a, b| Store::latest_wins(&a.0, &b.0));
        Ok(held.into_iter().map(|(_, tick)| tick).collect())
    }

    /// The boxes of the text of `entity`, a Note's or a Section's id, as
    /// they show now; empty when there is no such text.
    pub fn of(&self, entity: &str) -> Result<Vec<ShownBox>> {
        Ok(self.boxes(entity)?.unwrap_or_default())
    }

    /// Whether this Device may tick the boxes of the text of `entity`:
    /// wherever it holds a role, as for checking a Line.
    pub fn may_tick(&self, entity: &str) -> Result<bool> {
        Ok(self.boxes(entity)?.is_some() && self.store.may_write_in(entity)?)
    }

    /// Ticks the box `id` in the text of `entity`, a Note's or a
    /// Section's id, to `state`. Nothing is written when it shows `state`
    /// already. An error when the text holds no such box, or this Device
    /// may not tick it.
    pub fn tick(&self, entity: &str, id: &str, state: BoxState) -> Result<()> {
        let Some(shown) = self.of(entity)?.into_iter().find(|b| b.id == id) else {
            return Err(Error::Invalid(format!("not a box of {entity}: {id}")));
        };
        if !self.store.may_write_in(entity)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {entity}"
            )));
        }
        if shown.state == state {
            return Ok(());
        }
        self.store
            .append(entity, &tick_field(id), Some(state.name()), None)
            .map(drop)
    }

    /// The boxes of the text of `entity`, or `None` when there is no such
    /// text.
    fn boxes(&self, entity: &str) -> Result<Option<Vec<ShownBox>>> {
        let notes = Notes::new(self.store);
        Ok(if entity.starts_with("note:") {
            notes.by_id(entity)?.map(|n| n.boxes)
        } else {
            notes.section_by_id(entity)?.map(|s| s.boxes)
        })
    }

    /// Every tick of `entity` in the shape the apps write that counts: in
    /// a Shared Area what the roles allow, anywhere else what the user's
    /// own Devices wrote, so a tick another person's modified app slips
    /// into the log changes nothing.
    fn counted(&self, entity: &str) -> Result<Vec<(Entry, String, BoxTick)>> {
        let shared = match self.store.area_of(entity)? {
            Some(area) => self.store.is_shared(&area)?,
            None => false,
        };
        let mut own = HashMap::new();
        let mut out = Vec::new();
        for e in self.store.history(entity)? {
            let Some(id) = e.field.strip_prefix(PREFIX) else {
                continue;
            };
            let Some(state) = BoxState::named(e.value.as_deref()) else {
                continue;
            };
            if !text_boxes::is_id(id) {
                continue;
            }
            let id = id.to_string();
            let mine = self.store.written_by_own_in(&e.device, &mut own)?;
            if !shared && !mine {
                continue;
            }
            let tick = BoxTick {
                state,
                at: parse_time(&e.date)?,
                device: e.device.clone(),
                own: mine,
            };
            out.push((e, id, tick));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
