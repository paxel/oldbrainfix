//! Sessions (ADR 0029) as the Dart core's `sessions.dart` reads and
//! writes them: a Routine (ADR 0005, 0006) that picks Tallies from a
//! pool for each Occurrence, by Random, Rolling or Least selected. Its
//! pool, counts and rule lie on the Routine; so do the picks and the
//! entries recorded for them, keyed by Occurrence, which everyone with a
//! role in its Area writes. Every write appends the same entries, in the
//! same order and read from the clock as often, as the Dart core's.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::entry::Entry;
use crate::notes::{Note, Notes};
use crate::people::PersonFilter;
use crate::routines::{Routine, Routines};
use crate::schedule::{Day, Occurrence, Schedule, WallClock};
use crate::store::{Store, utf16_cmp};
use crate::tallies::{Tallies, Tally, TallyEntry, keys as tally_keys};
use crate::{Error, Result};

/// Field keys of a Session, on the entity of its Routine.
pub mod keys {
    /// The pool: the Notes of its Tallies, in the order Rolling goes
    /// through them, joined by commas. A Routine with a pool is a
    /// Session.
    pub const POOL: &str = "session:pool";
    /// How many Tallies an Occurrence picks.
    pub const COUNT: &str = "session:count";
    /// The picking rule, a [`super::SessionRule`]'s name.
    pub const RULE: &str = "session:rule";
    /// Prefix of the pick of an Occurrence, keyed by its wall-clock key:
    /// the Notes of its Tallies joined by commas.
    pub const PICK_PREFIX: &str = "pick@";
    /// Prefix of an entry recorded for a pick of an Occurrence,
    /// `entry@<occurrence>@<entry id>`; the value is the Tally's Note.
    pub const ENTRY_PREFIX: &str = "entry@";
    /// The highest count a Session takes.
    pub const MAX_COUNT: i64 = 99;

    /// How many an Occurrence on `weekday` (1 for Monday to 7 for Sunday)
    /// picks, instead of [`COUNT`].
    pub fn count_on(weekday: u32) -> String {
        format!("{COUNT}:{weekday}")
    }

    /// The pick of `occurrence`.
    pub fn pick(occurrence: &crate::schedule::Occurrence) -> String {
        format!("{PICK_PREFIX}{}", occurrence.key())
    }

    /// An entry recorded for a pick of `occurrence`.
    pub fn entry(occurrence: &crate::schedule::Occurrence, entry_id: &str) -> String {
        format!("{ENTRY_PREFIX}{}@{entry_id}", occurrence.key())
    }
}

/// How a Session picks its Tallies (CONTEXT.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRule {
    /// A new random pick each time, the same on every Device.
    Random,
    /// Through the pool in its order, continuing where it stopped.
    Rolling,
    /// The Tallies with the fewest entries first.
    Least,
}

impl SessionRule {
    pub const ALL: [SessionRule; 3] = [
        SessionRule::Random,
        SessionRule::Rolling,
        SessionRule::Least,
    ];

    /// The name the log keeps, the Dart core's.
    pub fn name(self) -> &'static str {
        match self {
            SessionRule::Random => "random",
            SessionRule::Rolling => "rolling",
            SessionRule::Least => "least",
        }
    }

    pub fn parse(name: &str) -> Option<SessionRule> {
        SessionRule::ALL.into_iter().find(|r| r.name() == name)
    }
}

/// A Routine that picks Tallies from a pool for each Occurrence
/// (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub routine: Routine,
    /// The Notes of its Tallies as stored, in the order Rolling goes
    /// through them; see [`Sessions::pool`] for those it picks from.
    pub pool: Vec<String>,
    /// How many Tallies an Occurrence picks.
    pub count: i64,
    /// The count of a weekday (1 for Monday to 7 for Sunday) that has its
    /// own.
    pub counts: BTreeMap<u32, i64>,
    pub rule: SessionRule,
}

impl Session {
    pub fn id(&self) -> &str {
        &self.routine.id
    }

    /// How many Tallies an Occurrence on `day` picks.
    pub fn count_on(&self, day: Day) -> i64 {
        self.counts
            .get(&day.weekday())
            .copied()
            .unwrap_or(self.count)
    }
}

/// One Tally a Session's Occurrence picked.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionPick {
    /// The Tally's Note.
    pub note_id: String,
    /// The Note; `None` when this Device holds none.
    pub note: Option<Note>,
    /// The Tally with its fields and targets; `None` when the Note is
    /// none any more.
    pub tally: Option<Tally>,
    /// The entries recorded for this pick of the Occurrence, not
    /// removed, oldest first, anyone's.
    pub entries: Vec<TallyEntry>,
}

impl SessionPick {
    pub fn done(&self) -> bool {
        !self.entries.is_empty()
    }
}

/// A Session's Occurrence with its picks.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionOccurrence {
    pub session: Session,
    pub occurrence: Occurrence,
    pub picks: Vec<SessionPick>,
    /// Whether "Another one" has a Tally to swap in.
    pub can_swap: bool,
}

impl SessionOccurrence {
    /// Whether every pick has an entry: the Occurrence is Done then.
    pub fn complete(&self) -> bool {
        !self.picks.is_empty() && self.picks.iter().all(SessionPick::done)
    }
}

/// What a Session's Routine holds of its picks and of the entries
/// recorded for them, by Occurrence key, only what counts: in a Shared
/// Area what the roles allow (ADR 0013), anywhere else what the user's
/// own Devices wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionMarks {
    /// The current pick of each Occurrence.
    pub picks: BTreeMap<String, Vec<String>>,
    /// Every Tally a pick of each Occurrence ever held.
    pub offered: BTreeMap<String, BTreeSet<String>>,
    /// The entries recorded for each Occurrence: the Tally's Note by
    /// entry id.
    pub entries: BTreeMap<String, BTreeMap<String, String>>,
}

impl SessionMarks {
    /// The marks of the Routine `id`, read from its history.
    pub fn of(store: &Store, id: &str) -> Result<SessionMarks> {
        let history = store.history(id)?;
        Self::read(store, id, &history)
    }

    /// The marks of the Routine `id`, read from `history`.
    pub fn read(store: &Store, id: &str, history: &[Entry]) -> Result<SessionMarks> {
        let shared = match store.area_of(id)? {
            Some(area) => store.is_shared(&area)?,
            None => false,
        };
        let mut own = HashMap::new();
        let mut latest: BTreeMap<&str, &Entry> = BTreeMap::new();
        let mut offered: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for e in history {
            let pick = e.field.strip_prefix(keys::PICK_PREFIX);
            if pick.is_none() && !e.field.starts_with(keys::ENTRY_PREFIX) {
                continue;
            }
            if !shared && !store.written_by_own_in(&e.device, &mut own)? {
                continue;
            }
            let newer = latest
                .get(e.field.as_str())
                .is_none_or(|held| Store::latest_wins(e, held) == Ordering::Greater);
            if newer {
                latest.insert(&e.field, e);
            }
            if let Some(key) = pick.and_then(occurrence_key) {
                offered
                    .entry(key)
                    .or_default()
                    .extend(note_ids(e.value.as_deref()));
            }
        }
        let mut marks = SessionMarks {
            offered,
            ..SessionMarks::default()
        };
        for (field, e) in latest {
            if let Some(key) = field.strip_prefix(keys::PICK_PREFIX) {
                let ids = note_ids(e.value.as_deref());
                if let Some(key) = occurrence_key(key)
                    && !ids.is_empty()
                {
                    marks.picks.insert(key, ids);
                }
                continue;
            }
            let Some((key, entry_id)) = field
                .strip_prefix(keys::ENTRY_PREFIX)
                .and_then(|rest| rest.split_once('@'))
            else {
                continue;
            };
            let (Some(key), Some(note)) = (occurrence_key(key), e.value.as_deref()) else {
                continue;
            };
            if !is_id(entry_id, tally_keys::ENTRY_PREFIX) || !is_id(note, NOTE_PREFIX) {
                continue;
            }
            marks
                .entries
                .entry(key)
                .or_default()
                .insert(entry_id.to_string(), note.to_string());
        }
        Ok(marks)
    }

    /// The entries recorded for `note_id` in the Occurrence `key`, not
    /// removed, oldest first.
    pub fn entries_for(&self, store: &Store, key: &str, note_id: &str) -> Result<Vec<TallyEntry>> {
        let tallies = Tallies::new(store);
        let mut out = Vec::new();
        for (id, note) in self.entries.get(key).into_iter().flatten() {
            if note != note_id {
                continue;
            }
            if let Some(entry) = tallies.entry_by_id(id)?.filter(|e| !e.is_trashed()) {
                out.push(entry);
            }
        }
        out.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| utf16_cmp(&a.id, &b.id)));
        Ok(out)
    }

    /// The Occurrences whose every pick has an entry, by key, each with
    /// the moment the last of them got its first.
    pub fn done_from_entries(&self, store: &Store) -> Result<BTreeMap<String, DateTime<Utc>>> {
        let mut done = BTreeMap::new();
        for (key, ids) in &self.picks {
            let mut last: Option<DateTime<Utc>> = None;
            let mut whole = true;
            for id in ids {
                let Some(first) = self.entries_for(store, key, id)?.first().map(|e| e.at) else {
                    whole = false;
                    break;
                };
                if last.is_none_or(|l| first > l) {
                    last = Some(first);
                }
            }
            if whole && let Some(last) = last {
                done.insert(key.clone(), last);
            }
        }
        Ok(done)
    }
}

const NOTE_PREFIX: &str = "note:";

/// `text` when it is an Occurrence's key as the apps write it.
fn occurrence_key(text: &str) -> Option<String> {
    Occurrence::parse(text)
        .map(|o| o.key())
        .filter(|key| key == text)
}

/// The Notes `value` lists, each once, skipping what is no Note id.
fn note_ids(value: Option<&str>) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for id in value.unwrap_or_default().split(',') {
        if is_id(id, NOTE_PREFIX) && !ids.iter().any(|held| held == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

fn is_id(id: &str, prefix: &str) -> bool {
    id.len() > prefix.len() && id.starts_with(prefix) && !id.contains(',') && !id.contains('@')
}

/// The count `value` holds when written as the apps write one.
fn count_of(value: Option<&str>) -> Option<i64> {
    let value = value?;
    let b = value.as_bytes();
    let digits = matches!(b.len(), 1 | 2)
        && (b'1'..=b'9').contains(&b[0])
        && b.iter().all(u8::is_ascii_digit);
    if !digits {
        return None;
    }
    value.parse().ok()
}

fn check_count(count: i64) -> Result<()> {
    if !(1..=keys::MAX_COUNT).contains(&count) {
        return Err(Error::Invalid(format!(
            "a count of 1 to {}: {count}",
            keys::MAX_COUNT
        )));
    }
    Ok(())
}

/// Spawning and changing Sessions, and picking their Tallies.
pub struct Sessions<'a> {
    store: &'a Store,
}

impl<'a> Sessions<'a> {
    pub fn new(store: &'a Store) -> Sessions<'a> {
        Sessions { store }
    }

    /// Spawns a Session from the Note `note_id`, a Routine on `schedule`
    /// that picks `count` Tallies of `pool` (their Notes, in the order
    /// Rolling goes through them) by `rule` for each Occurrence. The pool
    /// takes [`Sessions::choices`] only.
    pub fn spawn(
        &self,
        note_id: &str,
        schedule: &Schedule,
        pool: &[String],
        count: i64,
        rule: SessionRule,
    ) -> Result<Session> {
        let chosen = self.chosen(self.store.area_of(note_id)?.as_deref(), pool, &[])?;
        check_count(count)?;
        let routine = Routines::new(self.store).spawn_with(note_id, schedule, |id| {
            let s = self.store;
            s.append(id, keys::POOL, Some(&chosen.join(",")), None)?;
            s.append(id, keys::COUNT, Some(&count.to_string()), None)?;
            s.append(id, keys::RULE, Some(rule.name()), None)?;
            Ok(())
        })?;
        self.by_id(&routine.id)?
            .ok_or_else(|| Error::Invalid("the spawned Session is gone".into()))
    }

    /// Creates a Session without a Note at hand: its Note, holding
    /// `text`, is captured behind the scenes (ADR 0005).
    pub fn create(
        &self,
        text: &str,
        schedule: &Schedule,
        pool: &[String],
        count: i64,
        rule: SessionRule,
    ) -> Result<Session> {
        self.chosen(None, pool, &[])?;
        check_count(count)?;
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::Invalid("the text must not be empty".into()));
        }
        let note = Notes::new(self.store).capture(text, None, None)?;
        self.spawn(&note.id, schedule, pool, count, rule)
    }

    /// Gives the Session `id` the pool `pool`: [`Sessions::choices`],
    /// and those it holds already.
    pub fn set_pool(&self, id: &str, pool: &[String]) -> Result<()> {
        let session = self.require_managed(id)?;
        let chosen = self.chosen(self.store.area_of(id)?.as_deref(), pool, &session.pool)?;
        self.write(id, keys::POOL, Some(&chosen.join(",")))
    }

    /// Sets how many Tallies an Occurrence of the Session `id` picks.
    pub fn set_count(&self, id: &str, count: i64) -> Result<()> {
        self.require_managed(id)?;
        check_count(count)?;
        self.write(id, keys::COUNT, Some(&count.to_string()))
    }

    /// Sets how many an Occurrence on `weekday` (1 for Monday to 7 for
    /// Sunday) picks; `None` takes the weekday's own count away.
    pub fn set_count_on(&self, id: &str, weekday: u32, count: Option<i64>) -> Result<()> {
        self.require_managed(id)?;
        if !(1..=7).contains(&weekday) {
            return Err(Error::Invalid(format!("a weekday of 1 to 7: {weekday}")));
        }
        if let Some(count) = count {
            check_count(count)?;
        }
        let value = count.map(|c| c.to_string());
        self.write(id, &keys::count_on(weekday), value.as_deref())
    }

    /// Sets how the Session `id` picks.
    pub fn set_rule(&self, id: &str, rule: SessionRule) -> Result<()> {
        self.require_managed(id)?;
        self.write(id, keys::RULE, Some(rule.name()))
    }

    /// The Session `id`, trashed or not, or `None` when it is none.
    pub fn by_id(&self, id: &str) -> Result<Option<Session>> {
        let Some(routine) = Routines::new(self.store).by_id(id)? else {
            return Ok(None);
        };
        let fields = self.store.current_fields(id)?;
        let Some(pool) = fields.get(keys::POOL) else {
            return Ok(None);
        };
        let mut counts = BTreeMap::new();
        for weekday in 1..=7 {
            if let Some(count) = count_of(fields.get(&keys::count_on(weekday))) {
                counts.insert(weekday, count);
            }
        }
        Ok(Some(Session {
            pool: note_ids(Some(pool)),
            count: count_of(fields.get(keys::COUNT)).unwrap_or(1),
            counts,
            rule: fields
                .get(keys::RULE)
                .and_then(SessionRule::parse)
                .unwrap_or(SessionRule::Rolling),
            routine,
        }))
    }

    /// Whether the Routine `id` is a Session.
    pub fn is_session(&self, id: &str) -> Result<bool> {
        Ok(self.by_id(id)?.is_some())
    }

    /// Every Session not in the trash, by id.
    pub fn all(&self) -> Result<Vec<Session>> {
        let mut out = Vec::new();
        for routine in Routines::new(self.store).all()? {
            if let Some(session) = self.by_id(&routine.id)? {
                out.push(session);
            }
        }
        out.sort_by(|a, b| utf16_cmp(a.id(), b.id()));
        Ok(out)
    }

    /// The Tallies a Session of `entity` may pick, as their Notes, newest
    /// capture first: `entity` is the Note to spawn it from, or the
    /// Session. Every Tally in the lists, but those outside its Area
    /// while that is a Shared Area, which not everyone in it holds.
    pub fn choices(&self, entity: &str) -> Result<Vec<String>> {
        let area = self.store.area_of(entity)?;
        let mut out = Vec::new();
        for tally in Tallies::new(self.store).all()? {
            if self.fits(area.as_deref(), &tally.note_id)? {
                out.push(tally.note_id);
            }
        }
        Ok(out)
    }

    /// The Tallies `session` picks from, as their Notes, in its order:
    /// those of its pool that are Tallies in the lists, a merged one as
    /// the Note it went into, each once, and in a Shared Area only those
    /// of that Area.
    pub fn pool(&self, session: &Session) -> Result<Vec<String>> {
        let area = self.store.area_of(session.id())?;
        let notes = Notes::new(self.store);
        let tallies = Tallies::new(self.store);
        let mut out: Vec<String> = Vec::new();
        for id in &session.pool {
            let shown = notes
                .by_id(id)?
                .and_then(|n| n.merged_into)
                .unwrap_or_else(|| id.clone());
            let Some(note) = notes.by_id(&shown)? else {
                continue;
            };
            if note.is_trashed() || note.is_merged() {
                continue;
            }
            if out.contains(&shown) || !self.fits(area.as_deref(), &shown)? {
                continue;
            }
            if tallies.is_tally(&shown)? {
                out.push(shown);
            }
        }
        Ok(out)
    }

    /// The Occurrence `occurrence` of the Session `id` with its picks, as
    /// [`Sessions::pick_today`] would write them when there are none yet:
    /// written nowhere.
    pub fn on(&self, id: &str, occurrence: &Occurrence) -> Result<SessionOccurrence> {
        let session = self.require_due(id, occurrence)?;
        let marks = SessionMarks::of(self.store, id)?;
        let pool = self.pool(&session)?;
        let picks = self.wanted(&session, occurrence, &pool, &marks)?;
        self.shown(session, *occurrence, picks, &pool, &marks)
    }

    /// Today's Occurrences of every Session on `wall` with their picks,
    /// by time, then by Session; written nowhere.
    pub fn today(&self, wall: &WallClock) -> Result<Vec<SessionOccurrence>> {
        self.day(wall, false)
    }

    /// Today's Occurrences of every Session on `wall` with their picks,
    /// each pick written for everyone the Session reaches when it is new:
    /// the first, or one that replaces Tallies gone since. The Agenda
    /// calls it as it shows; called again, it writes nothing.
    pub fn pick_today(&self, wall: &WallClock) -> Result<Vec<SessionOccurrence>> {
        self.day(wall, true)
    }

    /// "Another one": swaps the Tally of `note_id` picked for
    /// `occurrence` of the Session `id` for the next by the Session's
    /// rule, in its place: one this Occurrence has not offered yet, else
    /// the first not picked. The one swapped out keeps its place in the
    /// rule. Nothing happens when the pool holds no other.
    pub fn another(&self, id: &str, occurrence: &Occurrence, note_id: &str) -> Result<()> {
        let session = self.require_due(id, occurrence)?;
        self.require_check_off(id)?;
        let marks = SessionMarks::of(self.store, id)?;
        let pool = self.pool(&session)?;
        let wanted = self.wanted(&session, occurrence, &pool, &marks)?;
        if !wanted.iter().any(|w| w == note_id) {
            return Err(Error::Invalid(format!("not picked then: {note_id}")));
        }
        let mut offered = marks
            .offered
            .get(&occurrence.key())
            .cloned()
            .unwrap_or_default();
        offered.extend(wanted.iter().cloned());
        let order = self.order(&session, occurrence, &pool, &marks)?;
        let next = order
            .iter()
            .find(|t| !offered.contains(*t))
            .or_else(|| order.iter().find(|t| !wanted.contains(t)));
        let Some(next) = next else {
            return Ok(());
        };
        let swapped: Vec<String> = wanted
            .iter()
            .map(|p| {
                if p == note_id {
                    next.clone()
                } else {
                    p.clone()
                }
            })
            .collect();
        self.store.batch(|| {
            // The pick it swaps from counts as offered.
            self.write_pick(id, occurrence, &wanted, &marks)?;
            self.store
                .append(id, &keys::pick(occurrence), Some(&swapped.join(",")), None)
                .map(drop)
        })
    }

    /// Records an entry in the Tally of `note_id` picked for `occurrence`
    /// of the Session `id`, with `values` by field id in the base unit
    /// and a text `line`. Once every pick has one, the Occurrence is Done
    /// for everyone the Session reaches.
    pub fn record(
        &self,
        id: &str,
        occurrence: &Occurrence,
        note_id: &str,
        values: &BTreeMap<String, f64>,
        line: &str,
    ) -> Result<TallyEntry> {
        let session = self.require_due(id, occurrence)?;
        self.require_check_off(id)?;
        let marks = SessionMarks::of(self.store, id)?;
        let pool = self.pool(&session)?;
        let wanted = self.wanted(&session, occurrence, &pool, &marks)?;
        if !wanted.iter().any(|w| w == note_id) {
            return Err(Error::Invalid(format!("not picked then: {note_id}")));
        }
        Tallies::new(self.store).add_with(note_id, values, line, None, |entry_id| {
            self.write_pick(id, occurrence, &wanted, &marks)?;
            self.store
                .append(id, &keys::entry(occurrence, entry_id), Some(note_id), None)
                .map(drop)
        })
    }

    fn day(&self, wall: &WallClock, write: bool) -> Result<Vec<SessionOccurrence>> {
        let day = Day::today(self.store.now(), wall);
        let mut shown = Vec::new();
        for session in self.all()? {
            let pool = self.pool(&session)?;
            let may = write && self.store.may_check_off(session.id())?;
            for occurrence in session.routine.schedule.on(day) {
                let mut marks = SessionMarks::of(self.store, session.id())?;
                let picks = self.wanted(&session, &occurrence, &pool, &marks)?;
                if may && self.write_pick(session.id(), &occurrence, &picks, &marks)? {
                    marks = SessionMarks::of(self.store, session.id())?;
                }
                shown.push(self.shown(session.clone(), occurrence, picks, &pool, &marks)?);
            }
        }
        shown.sort_by(|a, b| {
            a.occurrence
                .cmp(&b.occurrence)
                .then_with(|| utf16_cmp(a.session.id(), b.session.id()))
        });
        Ok(shown)
    }

    fn shown(
        &self,
        session: Session,
        occurrence: Occurrence,
        picks: Vec<String>,
        pool: &[String],
        marks: &SessionMarks,
    ) -> Result<SessionOccurrence> {
        let notes = Notes::new(self.store);
        let tallies = Tallies::new(self.store);
        let key = occurrence.key();
        let mut shown = Vec::new();
        for id in &picks {
            shown.push(SessionPick {
                note_id: id.clone(),
                note: notes.by_id(id)?,
                tally: tallies.by_note(id)?,
                entries: marks.entries_for(self.store, &key, id)?,
            });
        }
        Ok(SessionOccurrence {
            can_swap: pool.iter().any(|id| !picks.contains(id)),
            session,
            occurrence,
            picks: shown,
        })
    }

    /// The pick `occurrence` should have: the one held, with what left
    /// the pool replaced in its place, unless it has an entry, and
    /// brought to the day's count; with none held, the first of the
    /// rule's order.
    fn wanted(
        &self,
        session: &Session,
        occurrence: &Occurrence,
        pool: &[String],
        marks: &SessionMarks,
    ) -> Result<Vec<String>> {
        let count = usize::try_from(session.count_on(occurrence.day)).unwrap_or(0);
        let order = self.order(session, occurrence, pool, marks)?;
        let key = occurrence.key();
        let Some(held) = marks.picks.get(&key) else {
            return Ok(order.into_iter().take(count).collect());
        };
        let empty = BTreeSet::new();
        let offered = marks.offered.get(&key).unwrap_or(&empty);
        let mut next = order
            .iter()
            .filter(|id| !offered.contains(*id))
            .chain(
                order
                    .iter()
                    .filter(|id| offered.contains(*id) && !held.contains(id)),
            )
            .cloned()
            .collect::<Vec<_>>()
            .into_iter();
        let recorded =
            |id: &str| -> Result<bool> { Ok(!marks.entries_for(self.store, &key, id)?.is_empty()) };
        let mut wanted = Vec::new();
        for id in held {
            if pool.contains(id) || recorded(id)? {
                wanted.push(id.clone());
            } else if let Some(next) = next.next() {
                wanted.push(next);
            }
        }
        while wanted.len() < count {
            let Some(id) = next.next() else {
                break;
            };
            wanted.push(id);
        }
        let mut i = wanted.len();
        while i > 0 && wanted.len() > count {
            i -= 1;
            if !recorded(&wanted[i])? {
                wanted.remove(i);
            }
        }
        Ok(wanted)
    }

    /// `pool` in the order `session`'s rule picks for `occurrence`.
    /// Rolling: those picked longest ago first, by the Occurrences before
    /// this one and their place in its pick, those never picked before
    /// them in pool order. Least selected: those with the fewest entries
    /// first, anyone's, then as Rolling. Random: by a hash of the
    /// Session, the Occurrence and the Tally, so every Device draws the
    /// same.
    fn order(
        &self,
        session: &Session,
        occurrence: &Occurrence,
        pool: &[String],
        marks: &SessionMarks,
    ) -> Result<Vec<String>> {
        let mut out = pool.to_vec();
        if session.rule == SessionRule::Random {
            let draw: HashMap<&str, String> = pool
                .iter()
                .map(|id| {
                    let text = format!("{}\n{}\n{id}", session.id(), occurrence.key());
                    (id.as_str(), hex::encode(Sha256::digest(text.as_bytes())))
                })
                .collect();
            out.sort_by(|a, b| {
                let drawn = |id: &String| draw.get(id.as_str()).cloned().unwrap_or_default();
                drawn(a).cmp(&drawn(b)).then_with(|| utf16_cmp(a, b))
            });
            return Ok(out);
        }
        let mut before: Vec<Occurrence> = marks
            .picks
            .keys()
            .filter_map(|key| Occurrence::parse(key))
            .filter(|o| o < occurrence)
            .collect();
        before.sort();
        // When each was picked last: later Occurrences count higher, and
        // within one, those further on in its pick.
        let mut last: HashMap<&str, i64> = HashMap::new();
        let mut picked = 0;
        for o in &before {
            for id in marks.picks.get(&o.key()).into_iter().flatten() {
                last.insert(id, picked);
                picked += 1;
            }
        }
        let mut entries: HashMap<&str, usize> = HashMap::new();
        if session.rule == SessionRule::Least {
            let tallies = Tallies::new(self.store);
            for id in pool {
                let n = tallies.entries(id, &PersonFilter::default())?.len();
                entries.insert(id, n);
            }
        }
        let index: HashMap<&str, usize> = pool
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        out.sort_by(|a, b| {
            let n = |id: &String| entries.get(id.as_str()).copied().unwrap_or(0);
            let at = |id: &String| last.get(id.as_str()).copied().unwrap_or(-1);
            let place = |id: &String| index.get(id.as_str()).copied().unwrap_or(0);
            n(a).cmp(&n(b))
                .then_with(|| at(a).cmp(&at(b)))
                .then_with(|| place(a).cmp(&place(b)))
        });
        Ok(out)
    }

    /// Writes `ids` as the pick of `occurrence`, unless that is the pick
    /// already or there is nothing to pick; whether it wrote.
    fn write_pick(
        &self,
        id: &str,
        occurrence: &Occurrence,
        ids: &[String],
        marks: &SessionMarks,
    ) -> Result<bool> {
        if ids.is_empty()
            || marks
                .picks
                .get(&occurrence.key())
                .is_some_and(|held| held == ids)
        {
            return Ok(false);
        }
        self.store
            .append(id, &keys::pick(occurrence), Some(&ids.join(",")), None)?;
        Ok(true)
    }

    /// `pool` without repeats, each a Tally a Session in `area` may pick
    /// or one of `held`; at least one.
    fn chosen(&self, area: Option<&str>, pool: &[String], held: &[String]) -> Result<Vec<String>> {
        let mut allowed = BTreeSet::new();
        for tally in Tallies::new(self.store).all()? {
            if self.fits(area, &tally.note_id)? {
                allowed.insert(tally.note_id);
            }
        }
        let mut chosen: Vec<String> = Vec::new();
        for id in pool {
            if chosen.contains(id) {
                continue;
            }
            if !allowed.contains(id) && !held.contains(id) {
                return Err(Error::Invalid(format!("not a Tally to pick: {id}")));
            }
            chosen.push(id.clone());
        }
        if chosen.is_empty() {
            return Err(Error::Invalid("a pool of at least one Tally".into()));
        }
        Ok(chosen)
    }

    /// Whether a Session in `area` picks the Tally of `note_id`: in no
    /// Shared Area any Tally, in one only those of that Area.
    fn fits(&self, area: Option<&str>, note_id: &str) -> Result<bool> {
        let Some(area) = area else {
            return Ok(true);
        };
        Ok(!self.store.is_shared(area)? || self.store.area_of(note_id)?.as_deref() == Some(area))
    }

    fn write(&self, id: &str, field: &str, value: Option<&str>) -> Result<()> {
        let current = self.store.current_fields(id)?;
        if current.iter().any(|(k, v)| k == field && v == value) {
            return Ok(());
        }
        self.store.append(id, field, value, None).map(drop)
    }

    /// The Session `id`, not in the trash.
    fn require(&self, id: &str) -> Result<Session> {
        self.by_id(id)?
            .filter(|s| !s.routine.is_trashed())
            .ok_or_else(|| Error::Invalid(format!("not a Session: {id}")))
    }

    /// The Session `id`, which this Device may change.
    fn require_managed(&self, id: &str) -> Result<Session> {
        let session = self.require(id)?;
        if !self.store.may_change_routine(id)? {
            return Err(Error::Invalid(format!("not a Manager of the Area of {id}")));
        }
        Ok(session)
    }

    /// The Session `id`, due at `occurrence`.
    fn require_due(&self, id: &str, occurrence: &Occurrence) -> Result<Session> {
        let session = self.require(id)?;
        if !session.routine.schedule.has(occurrence) {
            return Err(Error::Invalid(format!("not due then: {occurrence}")));
        }
        Ok(session)
    }

    fn require_check_off(&self, id: &str) -> Result<()> {
        if !self.store.may_check_off(id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {id}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
        sync_request_as,
    };
    use crate::schedule::{Frequency, Recurrence};
    use crate::shared::{AreaRole, Invite};
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::ticking;
    use crate::{OwnProfile, PairedDevice, Places};
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

    /// The wall clock `days` days after the store's now: the tests' clock
    /// moves by seconds, the Agenda by days. Day 0 is a Thursday.
    fn on_day(days: i64) -> impl Fn(DateTime<Utc>) -> NaiveDateTime {
        move |utc| (utc + Duration::days(days)).naive_utc()
    }

    /// One LAN session, `joiner` asking `host`.
    fn exchange(joiner: &Store, host: &Store, shared: bool) -> TestResult {
        let request = sync_request(
            joiner,
            &host.version_vector()?,
            "Phone",
            &host.device_id()?,
            "",
        )?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        if shared {
            apply_answer_as(joiner, &answer, true, None)?;
        } else {
            apply_answer(joiner, &answer)?;
        }
        Ok(())
    }

    fn remember(store: &Store, peer: &Store, shared: bool) -> TestResult {
        store.remember_device(&PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared,
        })?;
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
        remember(&ann, &ben, true)?;
        remember(&ben, &ann, true)?;
        Ok((ann, ben, area))
    }

    fn daily() -> Made<Schedule> {
        let rule = Some(Recurrence::every(Frequency::Daily, 1));
        Ok(Schedule::new(Day::new(2026, 1, 1), &[], rule).ok_or("a schedule")?)
    }

    /// Tallies with one field, Times with a target of 20, as their Notes.
    fn tallies(store: &Store, names: &[&str], place: Option<&str>) -> Made<Vec<String>> {
        let mut out = Vec::new();
        for name in names {
            let note = Notes::new(store).capture(name, place, None)?;
            Tallies::new(store).add_field(&note.id, "Times", "times", Some(20.0), None)?;
            out.push(note.id);
        }
        Ok(out)
    }

    fn session(
        store: &Store,
        pool: &[String],
        count: i64,
        rule: SessionRule,
        place: Option<&str>,
    ) -> Made<Session> {
        let note = Notes::new(store).capture("Sport", place, None)?;
        Ok(Sessions::new(store).spawn(&note.id, &daily()?, pool, count, rule)?)
    }

    /// Today's Occurrence of a daily Session `days` days on.
    fn day(store: &Store, days: i64) -> Occurrence {
        Occurrence::new(Day::today(store.now(), &on_day(days)), None)
    }

    /// The picks of `s` `days` days on, written as the Agenda does.
    fn picked(store: &Store, s: &Session, days: i64) -> Made<Vec<String>> {
        let mut out = Vec::new();
        for today in Sessions::new(store).pick_today(&on_day(days))? {
            if today.session.id() == s.id() {
                out.extend(today.picks.into_iter().map(|p| p.note_id));
            }
        }
        Ok(out)
    }

    /// The picks of `s` `days` days on, written nowhere.
    fn shown(store: &Store, s: &Session, days: i64) -> Made<Vec<String>> {
        let today = Sessions::new(store).on(s.id(), &day(store, days))?;
        Ok(today.picks.into_iter().map(|p| p.note_id).collect())
    }

    /// Records the targets for `note`, picked for `s` `days` days on.
    fn record(store: &Store, s: &Session, days: i64, note: &str) -> Made<TallyEntry> {
        let targets = Tallies::new(store)
            .by_note(note)?
            .map(|t| t.targets())
            .unwrap_or_default();
        Ok(Sessions::new(store).record(s.id(), &day(store, days), note, &targets, "")?)
    }

    fn done_at(store: &Store, s: &Session, o: &Occurrence) -> Made<Option<DateTime<Utc>>> {
        Ok(Routines::new(store)
            .by_id(s.id())?
            .and_then(|r| r.done_at(o)))
    }

    #[test]
    fn a_session_is_a_routine_from_a_note_with_a_pool_a_count_and_a_rule() -> TestResult {
        let ann = device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let sessions = Sessions::new(&ann);
        let t = tallies(&ann, &["Push-ups", "Sit-ups", "Wall sit"], None)?;
        let milk = notes.capture("Milk", None, None)?.id;
        let pool = [t[0].clone(), t[1].clone(), t[0].clone(), t[2].clone()];
        let s = session(&ann, &pool, 2, SessionRule::Least, None)?;
        assert_eq!(s.pool, t);
        assert_eq!((s.count, s.rule), (2, SessionRule::Least));
        assert!(Routines::new(&ann).by_id(s.id())?.is_some());
        let unsorted: Vec<String> = notes.unsorted()?.into_iter().map(|n| n.id).collect();
        assert_eq!(unsorted, std::slice::from_ref(&milk));
        let newest_first: Vec<String> = t.iter().rev().cloned().collect();
        assert_eq!(sessions.choices(&s.routine.note_id)?, newest_first);
        assert_eq!(sessions.pool(&s)?, t);
        // A pool of Tallies, at least one; a count of 1 to 99.
        let note = notes.capture("Learn", None, None)?.id;
        let every_day = daily()?;
        let spawn = |pool: &[String], count| {
            sessions.spawn(&note, &every_day, pool, count, SessionRule::Random)
        };
        assert!(spawn(&[milk], 1).is_err());
        assert!(spawn(&[], 1).is_err());
        assert!(spawn(&t[..1], 0).is_err());
        assert!(spawn(&t[..1], 100).is_err());
        // Changed: what is the same writes nothing.
        sessions.set_count(s.id(), 3)?;
        sessions.set_count_on(s.id(), 6, Some(1))?;
        sessions.set_rule(s.id(), SessionRule::Random)?;
        let two = [t[2].clone(), t[0].clone()];
        sessions.set_pool(s.id(), &two)?;
        let changed = sessions.by_id(s.id())?.ok_or("a Session")?;
        assert_eq!((changed.count, changed.rule), (3, SessionRule::Random));
        assert_eq!(changed.counts, BTreeMap::from([(6, 1)]));
        assert_eq!(changed.pool, two);
        let before = ann.all_entries()?.len();
        sessions.set_count(s.id(), 3)?;
        sessions.set_pool(s.id(), &two)?;
        assert_eq!(ann.all_entries()?.len(), before);
        sessions.set_count_on(s.id(), 6, None)?;
        assert!(
            sessions
                .by_id(s.id())?
                .ok_or("a Session")?
                .counts
                .is_empty()
        );
        // A plain Routine is none.
        let walk = Routines::new(&ann).create("Walk", &daily()?)?;
        assert!(sessions.by_id(&walk.id)?.is_none());
        let all: Vec<String> = sessions.all()?.iter().map(|s| s.id().to_string()).collect();
        assert_eq!(all, [s.id().to_string()]);
        Ok(())
    }

    #[test]
    fn what_no_app_writes_is_passed_over() -> TestResult {
        let ann = device(1, "Ann")?;
        let t = tallies(&ann, &["A", "B", "C"], None)?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        let today = day(&ann, 0);
        let key = today.key();
        let c = t[2].as_str();
        // As a modified app would.
        let crafted = [
            (keys::COUNT.to_string(), "+3".to_string()),
            (keys::count_on(8), "5".to_string()),
            (format!("{}:0", keys::COUNT), "5".to_string()),
            (keys::count_on(7), "07".to_string()),
            (keys::RULE.to_string(), "sometimes".to_string()),
            (keys::PICK_PREFIX.to_string(), t[0].clone()),
            (format!("{}2026-3-1", keys::PICK_PREFIX), t[0].clone()),
            (keys::pick(&today), format!("x,{c},{c},note:,note:a@b,")),
            (format!("{}{key}", keys::ENTRY_PREFIX), c.to_string()),
            (format!("{}{key}@", keys::ENTRY_PREFIX), c.to_string()),
            (
                format!("{}@tallyEntry:x", keys::ENTRY_PREFIX),
                c.to_string(),
            ),
            (keys::entry(&today, "note:x"), c.to_string()),
        ];
        for (field, value) in &crafted {
            ann.append(s.id(), field, Some(value), None)?;
        }
        let read = Sessions::new(&ann).by_id(s.id())?.ok_or("a Session")?;
        assert_eq!((read.count, read.rule), (1, SessionRule::Rolling));
        assert!(read.counts.is_empty());
        let marks = SessionMarks::of(&ann, s.id())?;
        assert_eq!(marks.picks, BTreeMap::from([(key, vec![c.to_string()])]));
        assert!(marks.entries.is_empty());
        assert_eq!(shown(&ann, &s, 0)?, [c.to_string()]);
        let routine = Routines::new(&ann).by_id(s.id())?.ok_or("a Routine")?;
        assert!(routine.done.is_empty());
        Ok(())
    }

    #[test]
    fn rolling_goes_through_the_pool_continuing_where_it_stopped() -> TestResult {
        let ann = device(1, "Ann")?;
        let t = tallies(&ann, &["A", "B", "C", "D", "E"], None)?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        let of = |i: &[usize]| -> Vec<String> { i.iter().map(|i| t[*i].clone()).collect() };
        assert_eq!(picked(&ann, &s, 0)?, of(&[0, 1]));
        assert_eq!(picked(&ann, &s, 1)?, of(&[2, 3]));
        assert_eq!(picked(&ann, &s, 2)?, of(&[4, 0]));
        // A day nobody looked picks nothing.
        assert_eq!(picked(&ann, &s, 4)?, of(&[1, 2]));
        assert_eq!(picked(&ann, &s, 5)?, of(&[3, 4]));
        assert_eq!(picked(&ann, &s, 6)?, of(&[0, 1]));
        // Called again the same day, it writes nothing.
        let before = ann.all_entries()?.len();
        assert_eq!(picked(&ann, &s, 6)?, of(&[0, 1]));
        assert_eq!(ann.all_entries()?.len(), before);
        Ok(())
    }

    #[test]
    fn least_selected_picks_those_with_the_fewest_entries_first() -> TestResult {
        let ann = device(1, "Ann")?;
        let t = tallies(&ann, &["A", "B"], None)?;
        let s = session(&ann, &t, 1, SessionRule::Least, None)?;
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone()]);
        // A skipped, B recorded: A comes back.
        assert_eq!(picked(&ann, &s, 1)?, [t[1].clone()]);
        record(&ann, &s, 1, &t[1])?;
        assert_eq!(picked(&ann, &s, 2)?, [t[0].clone()]);
        // Entries from anywhere count.
        let tallies = Tallies::new(&ann);
        tallies.add(&t[0], &BTreeMap::new(), "Morning", None)?;
        tallies.add(&t[0], &BTreeMap::new(), "Evening", None)?;
        assert_eq!(picked(&ann, &s, 3)?, [t[1].clone()]);
        Ok(())
    }

    #[test]
    fn random_draws_by_a_hash_the_same_on_every_device() -> TestResult {
        let ann = device(1, "Ann")?;
        let t = tallies(&ann, &["A", "B", "C", "D", "E"], None)?;
        let s = session(&ann, &t, 3, SessionRule::Random, None)?;
        for days in 0..4 {
            let key = day(&ann, days).key();
            let hash = |id: &String| {
                hex::encode(Sha256::digest(
                    format!("{}\n{key}\n{id}", s.id()).as_bytes(),
                ))
            };
            let mut drawn = t.clone();
            drawn.sort_by_key(hash);
            drawn.truncate(3);
            assert_eq!(shown(&ann, &s, days)?, drawn);
            assert_eq!(picked(&ann, &s, days)?, drawn);
        }
        Ok(())
    }

    #[test]
    fn a_weekday_has_a_count_of_its_own() -> TestResult {
        let ann = device(1, "Ann")?;
        let t = tallies(&ann, &["A", "B", "C", "D"], None)?;
        let s = session(&ann, &t, 1, SessionRule::Rolling, None)?;
        Sessions::new(&ann).set_count_on(s.id(), 6, Some(3))?;
        assert_eq!(day(&ann, 2).day.weekday(), 6);
        assert_eq!(shown(&ann, &s, 1)?.len(), 1);
        assert_eq!(shown(&ann, &s, 2)?.len(), 3);
        // More than the pool holds picks the pool.
        Sessions::new(&ann).set_count_on(s.id(), 6, Some(9))?;
        assert_eq!(shown(&ann, &s, 2)?.len(), 4);
        Ok(())
    }

    #[test]
    fn another_one_swaps_in_place_and_the_one_swapped_keeps_its_place() -> TestResult {
        let ann = device(1, "Ann")?;
        let sessions = Sessions::new(&ann);
        let t = tallies(&ann, &["A", "B", "C", "D"], None)?;
        let of = |i: &[usize]| -> Vec<String> { i.iter().map(|i| t[*i].clone()).collect() };
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        let today = day(&ann, 0);
        assert_eq!(picked(&ann, &s, 0)?, of(&[0, 1]));
        sessions.another(s.id(), &today, &t[0])?;
        assert_eq!(shown(&ann, &s, 0)?, of(&[2, 1]));
        sessions.another(s.id(), &today, &t[2])?;
        assert_eq!(shown(&ann, &s, 0)?, of(&[3, 1]));
        // Every one offered: round again.
        sessions.another(s.id(), &today, &t[3])?;
        assert_eq!(shown(&ann, &s, 0)?, of(&[0, 1]));
        assert!(sessions.another(s.id(), &today, &t[2]).is_err());
        // Swapped out, C and D come first the next day.
        assert_eq!(picked(&ann, &s, 1)?, of(&[2, 3]));
        // Nothing to swap in, nothing written.
        let xy = tallies(&ann, &["X", "Y"], None)?;
        let two = session(&ann, &xy, 2, SessionRule::Rolling, None)?;
        let pick = sessions.pick_today(&on_day(0))?;
        let held = pick
            .iter()
            .find(|o| o.session.id() == two.id())
            .ok_or("a pick")?;
        assert!(!held.can_swap);
        let before = ann.all_entries()?.len();
        sessions.another(two.id(), &today, &xy[0])?;
        assert_eq!(ann.all_entries()?.len(), before);
        Ok(())
    }

    #[test]
    fn a_tally_gone_from_the_pool_is_replaced_unless_it_has_an_entry() -> TestResult {
        let ann = device(1, "Ann")?;
        let t = tallies(&ann, &["A", "B", "C"], None)?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        record(&ann, &s, 0, &t[1])?;
        Notes::new(&ann).trash(&t[1])?;
        Notes::new(&ann).trash(&t[0])?;
        assert_eq!(picked(&ann, &s, 0)?, [t[2].clone(), t[1].clone()]);
        let read = Sessions::new(&ann).by_id(s.id())?.ok_or("a Session")?;
        assert_eq!(Sessions::new(&ann).pool(&read)?, [t[2].clone()]);
        Ok(())
    }

    #[test]
    fn done_once_every_pick_has_an_entry_or_when_checked_off() -> TestResult {
        let ann = device(1, "Ann")?;
        let routines = Routines::new(&ann);
        let tallies = Tallies::new(&ann);
        let t = tallies_of(&ann)?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        let today = day(&ann, 0);
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        let first =
            Sessions::new(&ann).record(s.id(), &today, &t[0], &BTreeMap::new(), "Morning")?;
        assert!(first.values.is_empty());
        assert_eq!(done_at(&ann, &s, &today)?, None);
        // An entry made on the Tally itself is no pick's.
        tallies.add(&t[1], &BTreeMap::new(), "Elsewhere", None)?;
        assert_eq!(done_at(&ann, &s, &today)?, None);
        let last = record(&ann, &s, 0, &t[1])?;
        assert_eq!(done_at(&ann, &s, &today)?, Some(last.at));
        let row = routines.agenda(&on_day(0))?.today_rows;
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].done(), Some(last.at));
        let occurrence = Sessions::new(&ann).on(s.id(), &today)?;
        assert!(occurrence.complete());
        let recorded: Vec<Vec<String>> = occurrence
            .picks
            .iter()
            .map(|p| p.entries.iter().map(|e| e.id.clone()).collect())
            .collect();
        assert_eq!(recorded, [vec![first.id.clone()], vec![last.id.clone()]]);
        // Removed, the entry takes Done with it.
        tallies.trash_entry(&last.id)?;
        assert_eq!(done_at(&ann, &s, &today)?, None);
        // A check-off makes it Done, and keeps its moment.
        let at = Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .ok_or("a time")?;
        routines.mark_done(s.id(), &today, Some(at))?;
        tallies.restore_entry(&last.id)?;
        assert_eq!(done_at(&ann, &s, &today)?, Some(at));
        // Only a pick takes an entry.
        assert!(record(&ann, &s, 0, &t[2]).is_err());
        Ok(())
    }

    fn tallies_of(store: &Store) -> Made<Vec<String>> {
        tallies(store, &["A", "B", "C"], None)
    }

    #[test]
    fn picks_swaps_and_entries_travel_between_own_devices() -> TestResult {
        let ann = device(1, "Ann")?;
        let desk = device(3, "Ann at the desk")?;
        exchange(&desk, &ann, false)?;
        remember(&ann, &desk, false)?;
        remember(&desk, &ann, false)?;
        let t = tallies_of(&ann)?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        let today = day(&ann, 0);
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        exchange(&desk, &ann, false)?;
        assert_eq!(shown(&desk, &s, 0)?, [t[0].clone(), t[1].clone()]);
        let before = desk.all_entries()?.len();
        assert_eq!(picked(&desk, &s, 0)?, [t[0].clone(), t[1].clone()]);
        assert_eq!(desk.all_entries()?.len(), before);
        Sessions::new(&desk).another(s.id(), &today, &t[0])?;
        record(&desk, &s, 0, &t[2])?;
        exchange(&desk, &ann, false)?;
        assert_eq!(shown(&ann, &s, 0)?, [t[2].clone(), t[1].clone()]);
        record(&ann, &s, 0, &t[1])?;
        exchange(&desk, &ann, false)?;
        let done = done_at(&ann, &s, &today)?;
        assert!(done.is_some());
        assert_eq!(done_at(&desk, &s, &today)?, done);
        assert_eq!(shown(&desk, &s, 1)?, shown(&ann, &s, 1)?);
        Ok(())
    }

    #[test]
    fn a_pick_another_persons_device_slipped_in_counts_for_nothing() -> TestResult {
        let (ann, ben, _) = household()?;
        let t = tallies_of(&ann)?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, None)?;
        let today = day(&ann, 0);
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        // As a modified app would: Ben slips marks into Ann's own Session.
        let forged = [
            ben.append(s.id(), &keys::pick(&today), Some(&t[2]), None)?,
            ben.append(
                s.id(),
                &keys::entry(&today, "tallyEntry:x"),
                Some(&t[2]),
                None,
            )?,
        ];
        let wire: Vec<_> = forged.iter().map(Entry::wire).collect();
        let mut report = ImportReport::default();
        ann.apply_entries(&wire, &[], Some(&ben.device_id()?), &mut report)?;
        let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
        assert!(forged.iter().all(|e| held.contains(&e.id())));
        assert_eq!(shown(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        assert!(SessionMarks::of(&ann, s.id())?.entries.is_empty());
        Ok(())
    }

    #[test]
    fn in_a_shared_area_everyone_gets_the_same_picks_and_one_done() -> TestResult {
        let (ann, ben, area) = household()?;
        let t = tallies(&ann, &["A", "B", "C"], Some(&area))?;
        let s = session(&ann, &t, 2, SessionRule::Least, Some(&area))?;
        let today = day(&ann, 0);
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        exchange(&ben, &ann, true)?;
        assert_eq!(ben.area_of(s.id())?, Some(area.clone()));
        let before = ben.all_entries()?.len();
        assert_eq!(picked(&ben, &s, 0)?, [t[0].clone(), t[1].clone()]);
        assert_eq!(ben.all_entries()?.len(), before);
        // One entry each: one Done for both.
        record(&ben, &s, 0, &t[0])?;
        record(&ann, &s, 0, &t[1])?;
        assert_eq!(done_at(&ann, &s, &today)?, None);
        exchange(&ben, &ann, true)?;
        exchange(&ben, &ann, true)?;
        let done = done_at(&ann, &s, &today)?;
        assert!(done.is_some());
        assert_eq!(done_at(&ben, &s, &today)?, done);
        // A Member records and swaps, but changes no Session.
        assert!(Sessions::new(&ben).set_count(s.id(), 1).is_err());
        let own = Notes::new(&ben).capture("Gym", Some(&area), None)?.id;
        let spawned = Sessions::new(&ben).spawn(&own, &daily()?, &t[..1], 1, SessionRule::Rolling);
        assert!(spawned.is_err());
        Ok(())
    }

    #[test]
    fn a_member_swaps_for_everyone_but_writes_nothing_else_of_it() -> TestResult {
        let (ann, ben, area) = household()?;
        let t = tallies(&ann, &["A", "B", "C"], Some(&area))?;
        let s = session(&ann, &t, 2, SessionRule::Rolling, Some(&area))?;
        let today = day(&ann, 0);
        assert_eq!(picked(&ann, &s, 0)?, [t[0].clone(), t[1].clone()]);
        exchange(&ben, &ann, true)?;
        Sessions::new(&ben).another(s.id(), &today, &t[0])?;
        // As a modified app would.
        let forged = [
            ben.append(s.id(), keys::COUNT, Some("3"), None)?,
            ben.append(s.id(), keys::POOL, Some(&t[0]), None)?,
        ];
        exchange(&ben, &ann, true)?;
        let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
        assert!(forged.iter().all(|e| held.contains(&e.id())));
        assert_eq!(shown(&ann, &s, 0)?, [t[2].clone(), t[1].clone()]);
        let read = Sessions::new(&ann).by_id(s.id())?.ok_or("a Session")?;
        assert_eq!(read.count, 2);
        assert_eq!(read.pool, t);
        Ok(())
    }

    #[test]
    fn only_tallies_of_its_area_count_while_it_is_shared() -> TestResult {
        let (ann, _ben, area) = household()?;
        let mine = tallies(&ann, &["Mine"], None)?.remove(0);
        let a = tallies(&ann, &["A"], Some(&area))?.remove(0);
        let note = Notes::new(&ann).capture("Sport", Some(&area), None)?.id;
        let sessions = Sessions::new(&ann);
        assert_eq!(sessions.choices(&note)?, std::slice::from_ref(&a));
        let both = [a.clone(), mine.clone()];
        assert!(
            sessions
                .spawn(&note, &daily()?, &both, 2, SessionRule::Rolling)
                .is_err()
        );
        // A Session of one's own picks from every Area.
        let private = session(
            &ann,
            &[mine.clone(), a.clone()],
            2,
            SessionRule::Rolling,
            None,
        )?;
        assert_eq!(sessions.pool(&private)?, [mine.clone(), a.clone()]);
        // Filed into the Area, it leaves the others out.
        Notes::new(&ann).file(&private.routine.note_id, &area)?;
        assert_eq!(sessions.pool(&private)?, std::slice::from_ref(&a));
        assert_eq!(picked(&ann, &private, 0)?, [a]);
        Ok(())
    }
}
