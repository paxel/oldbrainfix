//! Frogs (ADR 0027) as the Dart core's `frogs.dart` reads and writes
//! them: a Note's job with no time, done once, spawned from its Note
//! (ADR 0005) and offered on the Agenda two a day, one to eat. Today's
//! pick lies outside every Area and counts only when one of the user's
//! own Devices wrote it. Every write appends the same entries, in the
//! same order and read from the clock as often, as the Dart core's.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::notes::{Note, Notes, keys as note_keys};
use crate::schedule::{Day, WallClock};
use crate::store::{Store, utf16_cmp};
use crate::{Error, Result};

/// Field keys of a Frog.
pub mod keys {
    pub const TYPE: &str = "$type";
    pub const TYPE_FROG: &str = "frog";
    /// The entity id's prefix.
    pub const PREFIX: &str = "frog:";
    /// The Note the Frog was spawned from (ADR 0005).
    pub const NOTE: &str = "note";
    /// When the Frog was checked off; `None` when it was unchecked again.
    pub const DONE: &str = "done";
    /// When the Frog was moved to the trash.
    pub const TRASHED: &str = "trashed";
    /// The entity holding today's pick, outside every Area: sealed under
    /// the personal key, it goes to the user's own Devices only, and only
    /// what they wrote counts.
    pub const PICK_ENTITY: &str = "frogs:pick";
    /// Today's pick: the day, then the ids of the Frogs it offers, joined
    /// by commas.
    pub const PICK: &str = "pick";
    /// How many Frogs the Agenda offers a day.
    pub const PER_DAY: usize = 2;
}

/// A Note's job with no time, done once, offered on the Agenda to eat one
/// a day (CONTEXT.md). It never rings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frog {
    /// Entity id, `frog:<id>`.
    pub id: String,
    /// The Note it was spawned from, as stored.
    pub note_id: String,
    /// When it was spawned.
    pub spawned: DateTime<Utc>,
    /// When it was checked off; `None` while it is open.
    pub done: Option<DateTime<Utc>>,
    /// When it was moved to the trash.
    pub trashed: Option<DateTime<Utc>>,
}

impl Frog {
    pub fn is_done(&self) -> bool {
        self.done.is_some()
    }

    pub fn is_trashed(&self) -> bool {
        self.trashed.is_some()
    }
}

/// The Agenda's "Eat a frog" section today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrogsToday {
    pub today: Day,
    /// The Frogs the section shows, in order: those of today's pick
    /// checked off today when there are any, else the open ones of the
    /// pick, at most [`keys::PER_DAY`].
    pub shown: Vec<Frog>,
    /// Whether a Frog of today's pick was eaten today: `shown` holds only
    /// those then, and the others stay hidden.
    pub eaten: bool,
    /// Whether "Another one" has an open Frog to swap in.
    pub can_swap: bool,
    /// Whether the section shows at all: while any Frog is open, or one
    /// eaten today is shown.
    pub visible: bool,
}

/// A pick as written: its day and the Frogs it offers.
struct Pick {
    day: Day,
    ids: Vec<String>,
}

impl Pick {
    /// The pick `value` holds; `None` for anything else.
    fn parse(value: Option<&str>) -> Option<Pick> {
        let mut parts = value?.split(',');
        let day = Day::parse(parts.next()?)?;
        let ids: Vec<String> = parts.map(str::to_string).collect();
        if ids.iter().any(|id| !id.starts_with(keys::PREFIX)) {
            return None;
        }
        Some(Pick { day, ids })
    }
}

/// Spawning, checking off and offering Frogs.
pub struct Frogs<'a> {
    store: &'a Store,
}

impl<'a> Frogs<'a> {
    pub fn new(store: &'a Store) -> Frogs<'a> {
        Frogs { store }
    }

    /// Spawns a Frog from the Note `note_id`, which leaves Unsorted. A
    /// Note has one open Frog at a time: while it has one, that one is
    /// returned and nothing is written.
    pub fn spawn(&self, note_id: &str) -> Result<Frog> {
        let note = Notes::new(self.store).by_id(note_id)?;
        let Some(note) = note.filter(|n| !n.is_trashed() && !n.is_merged()) else {
            return Err(Error::Invalid(format!(
                "not a Note in the lists: {note_id}"
            )));
        };
        if !self
            .store
            .may_manage(self.store.area_of(note_id)?.as_deref())?
        {
            return Err(Error::Invalid(format!(
                "not a Manager of the Area of {note_id}"
            )));
        }
        if let Some(frog) = self.of_note(note_id)?.into_iter().find(|f| !f.is_done()) {
            return Ok(frog);
        }
        let id = format!("{}{}", keys::PREFIX, self.store.new_id()?);
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_FROG), None)?;
            s.append(&id, keys::NOTE, Some(note_id), None)?;
            if !note.spawned {
                let at = dart_iso(s.now());
                s.append(note_id, note_keys::SPAWNED, Some(&at), None)?;
            }
            Ok(())
        })?;
        self.by_id(&id)?
            .ok_or_else(|| Error::Invalid("the spawned Frog is gone".into()))
    }

    /// Checks off the Frog `id`, at `at` or the store's now. Checking off
    /// one already Done writes nothing.
    pub fn mark_done(&self, id: &str, at: Option<DateTime<Utc>>) -> Result<()> {
        if self.require(id)?.is_done() {
            return Ok(());
        }
        let value = dart_iso(at.unwrap_or_else(|| self.store.now()));
        self.store
            .append(id, keys::DONE, Some(&value), None)
            .map(drop)
    }

    /// Unchecks the Frog `id` again; one still open writes nothing.
    pub fn unmark_done(&self, id: &str) -> Result<()> {
        if !self.require(id)?.is_done() {
            return Ok(());
        }
        self.store.append(id, keys::DONE, None, None).map(drop)
    }

    /// Moves the Frog `id` to the trash: it leaves every list. Its Note
    /// stays, and stays out of Unsorted.
    pub fn trash(&self, id: &str) -> Result<()> {
        self.require(id)?;
        let at = dart_iso(self.store.now());
        self.store
            .append(id, keys::TRASHED, Some(&at), None)
            .map(drop)
    }

    /// Brings the Frog `id` back from the trash.
    pub fn restore(&self, id: &str) -> Result<()> {
        if !self.by_id(id)?.is_some_and(|f| f.is_trashed()) {
            return Err(Error::Invalid(format!("not a Frog in the trash: {id}")));
        }
        self.store.append(id, keys::TRASHED, None, None).map(drop)
    }

    /// The Frog `id`, trashed or not, or `None` when there is none.
    pub fn by_id(&self, id: &str) -> Result<Option<Frog>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_FROG) {
            return Ok(None);
        }
        let Some(note_id) = fields.get(keys::NOTE) else {
            return Ok(None);
        };
        let mut spawned: Option<DateTime<Utc>> = None;
        for e in self.store.history(id)? {
            if e.field != keys::TYPE || e.value.as_deref() != Some(keys::TYPE_FROG) {
                continue;
            }
            if spawned.is_none_or(|s| e.date_at < s) {
                spawned = Some(e.date_at);
            }
        }
        let time = |field: &str| fields.get(field).and_then(|v| parse_time(v).ok());
        Ok(Some(Frog {
            id: id.to_string(),
            note_id: note_id.to_string(),
            spawned: spawned.unwrap_or(DateTime::<Utc>::MIN_UTC),
            done: time(keys::DONE),
            trashed: time(keys::TRASHED),
        }))
    }

    /// The Note `frog` links back to: the Note it was spawned from, or the
    /// one that Note was merged into; it may be in the trash. `None` when
    /// it is gone for good.
    pub fn note(&self, frog: &Frog) -> Result<Option<Note>> {
        let notes = Notes::new(self.store);
        let note = notes.by_id(&frog.note_id)?;
        match note.as_ref().and_then(|n| n.merged_into.clone()) {
            None => Ok(note),
            Some(into) => notes.by_id(&into),
        }
    }

    /// The open Frogs, oldest first: not checked off, not in the trash,
    /// their Note neither.
    pub fn open(&self) -> Result<Vec<Frog>> {
        Ok(self
            .listed()?
            .into_iter()
            .filter(|f| !f.is_done())
            .collect())
    }

    /// The Frogs checked off, most recently first, but those in the trash
    /// or whose Note is.
    pub fn done(&self) -> Result<Vec<Frog>> {
        let mut done: Vec<Frog> = self.listed()?.into_iter().filter(Frog::is_done).collect();
        done.sort_by(|a, b| b.done.cmp(&a.done).then_with(|| utf16_cmp(&a.id, &b.id)));
        Ok(done)
    }

    /// The Frogs the Note `note_id` spawned, those of the Notes merged
    /// into it included, oldest first, without those in the trash.
    pub fn of_note(&self, note_id: &str) -> Result<Vec<Frog>> {
        let mut out = Vec::new();
        for frog in self.all()? {
            if !frog.is_trashed() && self.note(&frog)?.is_some_and(|n| n.id == note_id) {
                out.push(frog);
            }
        }
        Ok(out)
    }

    /// The open Frogs in the order the Agenda offers them: those offered
    /// the longest ago first, those never offered before them, oldest
    /// first among equals.
    pub fn rotation(&self) -> Result<Vec<Frog>> {
        let mut last: HashMap<String, i64> = HashMap::new();
        for (i, pick) in self.picks()?.into_iter().enumerate() {
            for id in pick.ids {
                last.insert(id, i as i64);
            }
        }
        let mut open: Vec<(usize, Frog)> = self.open()?.into_iter().enumerate().collect();
        open.sort_by(|(ai, a), (bi, b)| {
            let shown = |f: &Frog| last.get(&f.id).copied().unwrap_or(-1);
            shown(a).cmp(&shown(b)).then_with(|| ai.cmp(bi))
        });
        Ok(open.into_iter().map(|(_, f)| f).collect())
    }

    /// Today's "Eat a frog" section, on `wall`, as [`Frogs::pick_today`]
    /// would write it: written nowhere.
    pub fn today(&self, wall: &WallClock) -> Result<FrogsToday> {
        let day = Day::today(self.store.now(), wall);
        let wanted = self.wanted(day, wall)?;
        self.section(day, wall, &wanted)
    }

    /// Today's "Eat a frog" section, on `wall`, its pick written for the
    /// user's own Devices when it is new: the first of the day, or one
    /// that replaces Frogs gone since. The Agenda calls it as it shows;
    /// called again, it writes nothing.
    pub fn pick_today(&self, wall: &WallClock) -> Result<FrogsToday> {
        let day = Day::today(self.store.now(), wall);
        let mut wanted = self.wanted(day, wall)?;
        if !self.held(day, &wanted)? {
            // Worked out again and written in one transaction (#207): a
            // pick another connection wrote meanwhile is the pick, not
            // written twice.
            wanted = self.store.batch(|| {
                let again = self.wanted(day, wall)?;
                self.write_pick(day, &again)?;
                Ok(again)
            })?;
        }
        self.section(day, wall, &wanted)
    }

    /// "Another one": swaps the Frog `id` offered today for the next one
    /// in the rotation, in its place. Nothing happens when there is none
    /// left to offer.
    pub fn another(&self, wall: &WallClock, id: &str) -> Result<()> {
        let day = Day::today(self.store.now(), wall);
        let wanted = self.wanted(day, wall)?;
        let shown = self.section(day, wall, &wanted)?;
        if shown.eaten || !shown.shown.iter().any(|f| f.id == id) {
            return Err(Error::Invalid(format!("not a Frog offered today: {id}")));
        }
        self.store.batch(|| {
            // The pick it swaps from counts as offered.
            self.write_pick(day, &wanted)?;
            for frog in self.rotation()? {
                if wanted.contains(&frog.id) {
                    continue;
                }
                let swapped: Vec<String> = wanted
                    .iter()
                    .map(|p| if p == id { frog.id.clone() } else { p.clone() })
                    .collect();
                return self.write_pick(day, &swapped);
            }
            Ok(())
        })
    }

    /// The section for `day` with the Frogs `picked`.
    fn section(&self, day: Day, wall: &WallClock, picked: &[String]) -> Result<FrogsToday> {
        let mut frogs = Vec::new();
        for id in picked {
            if let Some(frog) = self.shown_frog(id)? {
                frogs.push(frog);
            }
        }
        let eaten: Vec<Frog> = frogs
            .iter()
            .filter(|f| done_on(f, day, wall))
            .cloned()
            .collect();
        let open = self.open()?;
        let is_eaten = !eaten.is_empty();
        Ok(FrogsToday {
            today: day,
            shown: if is_eaten {
                eaten
            } else {
                frogs.into_iter().filter(|f| !f.is_done()).collect()
            },
            eaten: is_eaten,
            can_swap: !is_eaten && open.iter().any(|f| !picked.contains(&f.id)),
            visible: !open.is_empty() || is_eaten,
        })
    }

    /// The pick `day` should have: today's as it is while one of it was
    /// eaten today or it is whole; else with what is gone replaced in
    /// place from the rotation and topped up to [`keys::PER_DAY`].
    fn wanted(&self, day: Day, wall: &WallClock) -> Result<Vec<String>> {
        let picked: Vec<String> = match self.current_pick()? {
            Some(held) if held.day == day => held.ids.into_iter().take(keys::PER_DAY).collect(),
            _ => Vec::new(),
        };
        let mut frogs = Vec::new();
        for id in &picked {
            frogs.push(self.shown_frog(id)?);
        }
        if frogs.iter().flatten().any(|f| done_on(f, day, wall)) {
            return Ok(picked);
        }
        let mut next = self
            .rotation()?
            .into_iter()
            .map(|f| f.id)
            .filter(|id| !picked.contains(id));
        let mut wanted = Vec::new();
        for (id, frog) in picked.iter().zip(&frogs) {
            if frog.as_ref().is_some_and(|f| !f.is_done()) {
                wanted.push(id.clone());
            } else if let Some(id) = next.next() {
                wanted.push(id);
            }
        }
        while wanted.len() < keys::PER_DAY {
            let Some(id) = next.next() else {
                break;
            };
            wanted.push(id);
        }
        Ok(wanted)
    }

    /// Writes `ids` as the pick of `day`, unless that is the pick already
    /// or there is nothing to pick.
    fn write_pick(&self, day: Day, ids: &[String]) -> Result<()> {
        if self.held(day, ids)? {
            return Ok(());
        }
        let value = format!("{day},{}", ids.join(","));
        self.store
            .append(keys::PICK_ENTITY, keys::PICK, Some(&value), None)
            .map(drop)
    }

    /// Whether `ids` need no writing as the pick of `day`: there is
    /// nothing to pick, or that is the pick already.
    fn held(&self, day: Day, ids: &[String]) -> Result<bool> {
        if ids.is_empty() {
            return Ok(true);
        }
        Ok(self
            .current_pick()?
            .is_some_and(|held| held.day == day && held.ids == ids))
    }

    /// Every pick one of the user's own Devices wrote, in latest-wins
    /// order; another person's Device has no say in them.
    fn picks(&self) -> Result<Vec<Pick>> {
        let mut own = HashMap::new();
        let mut entries = Vec::new();
        for e in self.store.logged_history(keys::PICK_ENTITY)? {
            if e.field == keys::PICK && self.store.written_by_own_in(&e.device, &mut own)? {
                entries.push(e);
            }
        }
        entries.sort_by(Store::latest_wins);
        Ok(entries
            .iter()
            .filter_map(|e| Pick::parse(e.value.as_deref()))
            .collect())
    }

    /// The newest pick of the user's own Devices, or `None` when there is
    /// none it can read.
    fn current_pick(&self) -> Result<Option<Pick>> {
        Ok(self.picks()?.pop())
    }

    /// The Frog `id` when it shows in the lists, Done or not.
    fn shown_frog(&self, id: &str) -> Result<Option<Frog>> {
        let Some(frog) = self.by_id(id)?.filter(|f| !f.is_trashed()) else {
            return Ok(None);
        };
        Ok(match self.note(&frog)? {
            Some(note) if !note.is_trashed() => Some(frog),
            _ => None,
        })
    }

    /// Every Frog that shows in the lists, Done or not, oldest first.
    fn listed(&self) -> Result<Vec<Frog>> {
        let mut out = Vec::new();
        for frog in self.all()? {
            if self.shown_frog(&frog.id)?.is_some() {
                out.push(frog);
            }
        }
        Ok(out)
    }

    /// Every Frog, trashed or not, oldest first.
    fn all(&self) -> Result<Vec<Frog>> {
        let mut frogs = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_FROG)? {
            if let Some(frog) = self.by_id(&id)? {
                frogs.push(frog);
            }
        }
        frogs.sort_by(|a, b| {
            a.spawned
                .cmp(&b.spawned)
                .then_with(|| utf16_cmp(&a.id, &b.id))
        });
        Ok(frogs)
    }

    /// The Frog `id`, which must not be in the trash.
    fn require(&self, id: &str) -> Result<Frog> {
        self.by_id(id)?
            .filter(|f| !f.is_trashed())
            .ok_or_else(|| Error::Invalid(format!("not a Frog: {id}")))
    }
}

/// Whether `frog` was checked off on `day` as `wall` shows it.
fn done_on(frog: &Frog, day: Day, wall: &WallClock) -> bool {
    frog.done.is_some_and(|done| Day::today(done, wall) == day)
}

#[cfg(test)]
mod tests;
