//! Sessions (ADR 0029, 0039) as the Dart core's `sessions.dart` reads
//! and writes them: a kind of Note (ADR 0038) that picks Tallies from a
//! pool for each Occurrence of every Routine (ADR 0005, 0006) spawned
//! from it, by Random, Rolling or Least selected. Its pool, counts and
//! rule lie on the Note, or as an older version left them on a Routine;
//! the picks and the entries recorded for them lie on each Routine,
//! keyed by Occurrence, which everyone with a role in its Area writes.
//! Every write appends the same entries, in the same order and read from
//! the clock as often, as the Dart core's.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::entry::{Entry, dart_iso};
use crate::note_kinds::NoteKind;
use crate::notes::{Note, Notes, keys as note_keys};
use crate::people::PersonFilter;
use crate::routines::{Routine, Routines, keys as routine_keys};
use crate::schedule::{Day, Occurrence, Schedule, WallClock};
use crate::store::{Fields, Store, utf16_cmp};
use crate::tallies::{Tallies, Tally, TallyEntry, keys as tally_keys};
use crate::{Error, Result};

/// Field keys of a Session: its setup on the entity of its Note, written
/// there since #191 and read from the Routine as an older version wrote
/// it (ADR 0039); its picks on the entity of each Routine that runs it.
pub mod keys {
    /// The pool: the Notes of its Tallies, in the order Rolling goes
    /// through them, joined by commas. A Note with a pool is a Session.
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

    /// Whether `field` is a field of the setup: the pool, the count, a
    /// weekday's count or the rule.
    pub fn is_setup(field: &str) -> bool {
        field == POOL || field == COUNT || field == RULE || (1..=7).any(|d| field == count_on(d))
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

/// A Session as its Note holds it (#191): the pool, the counts and the
/// rule each Routine of the Note runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSetup {
    /// The Note that is the Session.
    pub note_id: String,
    /// The Notes of its Tallies as stored, in the order Rolling goes
    /// through them; see [`Sessions::pool`] for those it picks from.
    pub pool: Vec<String>,
    /// How many Tallies an Occurrence picks.
    pub count: i64,
    /// The count of a weekday (1 for Monday to 7 for Sunday) that has its
    /// own.
    pub counts: BTreeMap<u32, i64>,
    pub rule: SessionRule,
    /// The Routine an older version wrote the setup on, while the Note
    /// holds none of its own (ADR 0039); `None` once it does.
    pub held_by: Option<String>,
}

impl SessionSetup {
    /// How many Tallies an Occurrence on `day` picks.
    pub fn count_on(&self, day: Day) -> i64 {
        self.counts
            .get(&day.weekday())
            .copied()
            .unwrap_or(self.count)
    }
}

/// A Routine that runs the Session of its Note: it picks Tallies from
/// the Note's pool for each Occurrence (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub routine: Routine,
    pub setup: SessionSetup,
}

impl Session {
    /// The Routine's id: its picks lie on it.
    pub fn id(&self) -> &str {
        &self.routine.id
    }

    /// The Note whose Session it runs.
    pub fn note_id(&self) -> &str {
        &self.setup.note_id
    }

    /// How many Tallies an Occurrence on `day` picks.
    pub fn count_on(&self, day: Day) -> i64 {
        self.setup.count_on(day)
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

/// One past run of a Session (#264): an Occurrence of one of its
/// Routines with its pick.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRun {
    pub routine_id: String,
    pub occurrence: Occurrence,
    /// The Tallies picked, as their Notes in the pick's order, each with
    /// whether an entry not removed was recorded for it in the run.
    pub picks: Vec<(String, bool)>,
}

/// What a Session's Routine holds of its picks and of the entries
/// recorded for them, by Occurrence key, only what counts: in a Shared
/// Area what the roles allow (ADR 0013), anywhere else what the user's
/// own Devices wrote.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionMarks {
    /// The current pick of each Occurrence: the one its field holds,
    /// with every Tally that has an entry recorded for it in it.
    pub picks: BTreeMap<String, Vec<String>>,
    /// The pick each Occurrence's field holds, as written.
    pub stored: BTreeMap<String, Vec<String>>,
    /// Every Tally a pick of each Occurrence ever held.
    pub offered: BTreeMap<String, BTreeSet<String>>,
    /// The entries recorded for each Occurrence: the Tally's Note by
    /// entry id.
    pub entries: BTreeMap<String, BTreeMap<String, String>>,
    /// The entries of `entries` not removed, by Occurrence key and the
    /// Tally's Note, oldest first: read with the marks, in one query.
    recorded: BTreeMap<String, BTreeMap<String, Vec<TallyEntry>>>,
}

/// The marks [`SessionMarks::of`] read since the log changed, by Routine.
#[derive(Default)]
struct HeldMarks(parking_lot::Mutex<HashMap<String, Arc<SessionMarks>>>);

impl SessionMarks {
    /// The marks of the Routine `id`, read from its history once until
    /// the log changes (#204) and shared by every caller, as the Dart
    /// core's `SessionMarks.of`.
    pub fn of(store: &Store, id: &str) -> Result<Arc<SessionMarks>> {
        let held = store.worked_out("sessions:marks", || Ok(HeldMarks::default()))?;
        if let Some(marks) = held.0.lock().get(id) {
            return Ok(marks.clone());
        }
        // Read without the lock held.
        let history = store.history(id)?;
        let marks = Arc::new(Self::read(store, id, &history)?);
        held.0
            .lock()
            .entry(id.to_string())
            .or_insert_with(|| marks.clone());
        Ok(marks)
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
        // The place each Tally had in the latest pick that held it.
        let mut places: Places = BTreeMap::new();
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
                let ids = note_ids(e.value.as_deref());
                let placed = places.entry(key.clone()).or_default();
                for (i, note_id) in ids.iter().enumerate() {
                    let newer = placed
                        .get(note_id)
                        .is_none_or(|(was, _)| Store::latest_wins(e, was) == Ordering::Greater);
                    if newer {
                        placed.insert(note_id.clone(), (e, i));
                    }
                }
                offered.entry(key).or_default().extend(ids);
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
                    marks.stored.insert(key, ids);
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
        let found = Tallies::new(store).entries_by_id(
            marks
                .entries
                .values()
                .flat_map(|recorded| recorded.keys().map(String::as_str)),
        )?;
        for (key, recorded) in &marks.entries {
            for (entry_id, note) in recorded {
                if let Some(entry) = found.get(entry_id).filter(|e| !e.is_trashed()) {
                    marks
                        .recorded
                        .entry(key.clone())
                        .or_default()
                        .entry(note.clone())
                        .or_default()
                        .push(entry.clone());
                }
            }
        }
        for list in marks.recorded.values_mut().flat_map(BTreeMap::values_mut) {
            list.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| utf16_cmp(&a.id, &b.id)));
        }
        marks.picks = marks.stored.clone();
        marks.heal(store, &places)?;
        Ok(marks)
    }

    /// Puts each Tally with an entry recorded for an Occurrence back into
    /// its current pick when the field no longer holds it, as when a pick
    /// another Device wrote later wins over the one it was recorded for:
    /// in the place it had in the latest pick that held it, while the
    /// Tally there has no entry, else at the end.
    fn heal(&mut self, store: &Store, places: &Places) -> Result<()> {
        for (key, recorded) in &self.entries {
            let mut pick = self.picks.get(key).cloned().unwrap_or_default();
            let mut missing: Vec<&String> = Vec::new();
            for note_id in recorded.values() {
                if !pick.contains(note_id) && !missing.contains(&note_id) {
                    missing.push(note_id);
                }
            }
            missing.sort_by(|a, b| utf16_cmp(a, b));
            let mut healed = false;
            for note_id in missing {
                if self.entries_for(store, key, note_id)?.is_empty() {
                    continue;
                }
                let place = places
                    .get(key)
                    .and_then(|placed| placed.get(note_id))
                    .map(|(_, i)| *i);
                let free = match place.and_then(|i| pick.get(i)) {
                    Some(there) => self.entries_for(store, key, there)?.is_empty(),
                    None => false,
                };
                match place.and_then(|i| pick.get_mut(i)) {
                    Some(there) if free => *there = note_id.clone(),
                    _ => pick.push(note_id.clone()),
                }
                healed = true;
            }
            if healed {
                self.picks.insert(key.clone(), pick);
            }
        }
        Ok(())
    }

    /// The entries recorded for `note_id` in the Occurrence `key`, not
    /// removed, oldest first, as they were read with the marks.
    pub fn entries_for(&self, _store: &Store, key: &str, note_id: &str) -> Result<Vec<TallyEntry>> {
        Ok(self
            .recorded
            .get(key)
            .and_then(|by_note| by_note.get(note_id))
            .cloned()
            .unwrap_or_default())
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

/// The place each Tally had in the latest pick of an Occurrence that held
/// it, with that pick's entry, by Occurrence key and Note.
type Places<'a> = BTreeMap<String, BTreeMap<String, (&'a Entry, usize)>>;

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
        && b.first().is_some_and(|c| (b'1'..=b'9').contains(c))
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

/// Where the setups of the Sessions lie (ADR 0039), worked out in one
/// pass over what wrote a pool and kept until the log changes, as the
/// Dart core's `_Setups`: a Note is read once for many, and the Routines
/// that never held a setup not at all.
struct Setups {
    /// The fields of every entity that holds a pool of its own, of
    /// `None` too, by entity.
    own: HashMap<String, Fields>,
    /// The setup an older version left on a Routine not in the trash, by
    /// its Note: the Routine and its fields; of several the one whose
    /// setup was written last.
    left: HashMap<String, (String, Fields)>,
}

impl Setups {
    fn read(store: &Store) -> Result<Setups> {
        let mut own = HashMap::new();
        let mut left: HashMap<String, (String, Fields)> = HashMap::new();
        let mut newest: HashMap<String, Entry> = HashMap::new();
        let mut ids: Vec<String> = Vec::new();
        for (id, field) in store.fields_with_prefix(keys::POOL)? {
            if field == keys::POOL && !ids.contains(&id) {
                ids.push(id);
            }
        }
        for id in ids {
            let fields = store.current_fields(&id)?;
            if !fields.iter().any(|(field, _)| field == keys::POOL) {
                continue;
            }
            let note_id = fields.get(routine_keys::NOTE).map(str::to_string);
            let held = fields.get(routine_keys::TYPE) == Some(routine_keys::TYPE_ROUTINE)
                && fields.get(routine_keys::TRASHED).is_none()
                && fields.get(keys::POOL).is_some();
            own.insert(id.clone(), fields.clone());
            let Some(note_id) = note_id.filter(|_| held) else {
                continue;
            };
            for e in store.history(&id)? {
                if !keys::is_setup(&e.field) {
                    continue;
                }
                if newest
                    .get(&note_id)
                    .is_none_or(|n| Store::latest_wins(&e, n) == Ordering::Greater)
                {
                    newest.insert(note_id.clone(), e);
                    left.insert(note_id.clone(), (id.clone(), fields.clone()));
                }
            }
        }
        Ok(Setups { own, left })
    }
}

/// The setup `fields` of the Note `note_id` hold, or `None` without a
/// pool; `held_by` is the Routine they lie on, if not on the Note.
fn setup_from(note_id: &str, fields: &Fields, held_by: Option<String>) -> Option<SessionSetup> {
    let pool = fields.get(keys::POOL)?;
    let mut counts = BTreeMap::new();
    for weekday in 1..=7 {
        if let Some(count) = count_of(fields.get(&keys::count_on(weekday))) {
            counts.insert(weekday, count);
        }
    }
    Some(SessionSetup {
        note_id: note_id.to_string(),
        pool: note_ids(Some(pool)),
        count: count_of(fields.get(keys::COUNT)).unwrap_or(1),
        counts,
        rule: fields
            .get(keys::RULE)
            .and_then(SessionRule::parse)
            .unwrap_or(SessionRule::Rolling),
        held_by,
    })
}

/// Setting up Sessions on their Notes, and picking their Tallies for the
/// Routines that run them.
pub struct Sessions<'a> {
    store: &'a Store,
}

impl<'a> Sessions<'a> {
    pub fn new(store: &'a Store) -> Sessions<'a> {
        Sessions { store }
    }

    /// Makes the Note `note_id` a Session (#191): it picks `count`
    /// Tallies of `pool` (their Notes, in the order Rolling goes through
    /// them) by `rule` for each Occurrence of every Routine of the Note.
    /// Only a plain Note in the lists becomes one; the pool takes
    /// [`Sessions::choices`] only. It leaves Unsorted, as a Tally does.
    pub fn make(
        &self,
        note_id: &str,
        pool: &[String],
        count: i64,
        rule: SessionRule,
    ) -> Result<()> {
        self.require_changeable(note_id)?;
        if NoteKind::of(self.store, note_id)? != NoteKind::Plain {
            return Err(Error::Invalid(format!("not a plain Note: {note_id}")));
        }
        let chosen = self.chosen(self.store.area_of(note_id)?.as_deref(), pool, &[])?;
        check_count(count)?;
        self.store.batch(|| {
            self.write_setup(note_id, &chosen, count, rule)?;
            let spawned = Notes::new(self.store)
                .by_id(note_id)?
                .is_some_and(|n| n.spawned);
            if !spawned {
                let at = dart_iso(self.store.now());
                self.store
                    .append(note_id, note_keys::SPAWNED, Some(&at), None)?;
            }
            Ok(())
        })
    }

    /// Makes the Session `note_id` a plain Note again: its pool is taken
    /// away, so its Routines run as plain ones; their picks and entries
    /// stay in the log.
    pub fn unmake(&self, note_id: &str) -> Result<()> {
        self.require_setup(note_id)?;
        self.require_changeable(note_id)?;
        self.store.append(note_id, keys::POOL, None, None).map(drop)
    }

    /// Spawns a Routine on `schedule` from the Note `note_id` and sets
    /// the Note up as a Session of `pool`, `count` and `rule`, as
    /// [`Sessions::make`] does, in one batch; a Session already gets that
    /// setup. The Session the new Routine runs.
    pub fn spawn(
        &self,
        note_id: &str,
        schedule: &Schedule,
        pool: &[String],
        count: i64,
        rule: SessionRule,
    ) -> Result<Session> {
        let held = self.setup_of(note_id)?;
        if held.is_none() && NoteKind::of(self.store, note_id)? != NoteKind::Plain {
            return Err(Error::Invalid(format!("not a plain Note: {note_id}")));
        }
        let kept: &[String] = held.as_ref().map_or(&[], |h| &h.pool);
        let chosen = self.chosen(self.store.area_of(note_id)?.as_deref(), pool, kept)?;
        check_count(count)?;
        if held.is_some() {
            self.require_changeable(note_id)?;
        }
        let routine = Routines::new(self.store).spawn_with(note_id, schedule, |_| match &held {
            None => self.write_setup(note_id, &chosen, count, rule),
            Some(held) => {
                self.adopt(held)?;
                self.write(note_id, keys::POOL, Some(&chosen.join(",")))?;
                self.write(note_id, keys::COUNT, Some(&count.to_string()))?;
                self.write(note_id, keys::RULE, Some(rule.name()))
            }
        })?;
        self.by_id(&routine.id)?
            .ok_or_else(|| Error::Invalid("the spawned Session is gone".into()))
    }

    /// Creates a Session without a Note at hand: its Note, holding
    /// `text`, is captured behind the scenes (ADR 0005) and set up with a
    /// Routine on `schedule`, as [`Sessions::spawn`] does.
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

    /// Gives the Session `note_id` the pool `pool`:
    /// [`Sessions::choices`], and those it holds already.
    pub fn set_pool(&self, note_id: &str, pool: &[String]) -> Result<()> {
        let setup = self.require_managed(note_id)?;
        let area = self.store.area_of(note_id)?;
        let chosen = self.chosen(area.as_deref(), pool, &setup.pool)?;
        self.change(&setup, keys::POOL, Some(&chosen.join(",")))
    }

    /// Sets how many Tallies an Occurrence of the Session `note_id`
    /// picks.
    pub fn set_count(&self, note_id: &str, count: i64) -> Result<()> {
        let setup = self.require_managed(note_id)?;
        check_count(count)?;
        self.change(&setup, keys::COUNT, Some(&count.to_string()))
    }

    /// Sets how many an Occurrence on `weekday` (1 for Monday to 7 for
    /// Sunday) picks; `None` takes the weekday's own count away.
    pub fn set_count_on(&self, note_id: &str, weekday: u32, count: Option<i64>) -> Result<()> {
        let setup = self.require_managed(note_id)?;
        if !(1..=7).contains(&weekday) {
            return Err(Error::Invalid(format!("a weekday of 1 to 7: {weekday}")));
        }
        if let Some(count) = count {
            check_count(count)?;
        }
        let value = count.map(|c| c.to_string());
        self.change(&setup, &keys::count_on(weekday), value.as_deref())
    }

    /// Sets how the Session `note_id` picks.
    pub fn set_rule(&self, note_id: &str, rule: SessionRule) -> Result<()> {
        let setup = self.require_managed(note_id)?;
        self.change(&setup, keys::RULE, Some(rule.name()))
    }

    /// The Session the Note `note_id` is, or `None` when it is none: the
    /// setup on the Note, or, while the Note holds none, on the Routine
    /// of it an older version wrote it on (ADR 0039), of several the one
    /// whose setup was written last. A Note made plain again holds a pool
    /// of null and is none.
    pub fn setup_of(&self, note_id: &str) -> Result<Option<SessionSetup>> {
        let setups = self
            .store
            .worked_out("sessions:setups", || Setups::read(self.store))?;
        if let Some(fields) = setups.own.get(note_id) {
            return Ok(setup_from(note_id, fields, None));
        }
        Ok(setups
            .left
            .get(note_id)
            .and_then(|(id, fields)| setup_from(note_id, fields, Some(id.clone()))))
    }

    /// Whether the Note `note_id` is a Session.
    pub fn is_session_note(&self, note_id: &str) -> Result<bool> {
        Ok(self.setup_of(note_id)?.is_some())
    }

    /// The Routine `id` running the Session of its Note, trashed or not,
    /// or `None` when it is no Routine or its Note is no Session.
    pub fn by_id(&self, id: &str) -> Result<Option<Session>> {
        let Some(routine) = Routines::new(self.store).by_id(id)? else {
            return Ok(None);
        };
        let Some(setup) = self.setup_of(&routine.note_id)? else {
            return Ok(None);
        };
        Ok(Some(Session { routine, setup }))
    }

    /// Whether the Routine `id` runs a Session.
    pub fn is_session(&self, id: &str) -> Result<bool> {
        Ok(self.by_id(id)?.is_some())
    }

    /// Every Routine not in the trash that runs a Session, by id.
    pub fn all(&self) -> Result<Vec<Session>> {
        let mut setups: HashMap<String, Option<SessionSetup>> = HashMap::new();
        let mut out = Vec::new();
        for routine in Routines::new(self.store).all()? {
            let setup = match setups.get(&routine.note_id) {
                Some(setup) => setup.clone(),
                None => {
                    let setup = self.setup_of(&routine.note_id)?;
                    setups.insert(routine.note_id.clone(), setup.clone());
                    setup
                }
            };
            if let Some(setup) = setup {
                out.push(Session { routine, setup });
            }
        }
        out.sort_by(|a, b| utf16_cmp(a.id(), b.id()));
        Ok(out)
    }

    /// The past runs of the Session `note_id` (#264): every Occurrence of
    /// its Routines before today on `wall` that holds a pick, newest
    /// first, by Routine on the same moment, each Tally of the pick done
    /// or not by the entries recorded for it.
    pub fn runs(&self, note_id: &str, wall: &WallClock) -> Result<Vec<SessionRun>> {
        let today = Day::today(self.store.now(), wall);
        let mut runs = Vec::new();
        for session in self.of_note(note_id)? {
            let marks = SessionMarks::of(self.store, &session.routine.id)?;
            for (key, picks) in &marks.picks {
                let Some(occurrence) = Occurrence::parse(key).filter(|o| o.day < today) else {
                    continue;
                };
                let mut done = Vec::new();
                for pick in picks {
                    let entries = marks.entries_for(self.store, key, pick)?;
                    done.push((pick.clone(), !entries.is_empty()));
                }
                runs.push(SessionRun {
                    routine_id: session.routine.id.clone(),
                    occurrence,
                    picks: done,
                });
            }
        }
        runs.sort_by(|a, b| {
            b.occurrence
                .cmp(&a.occurrence)
                .then_with(|| utf16_cmp(&a.routine_id, &b.routine_id))
        });
        Ok(runs)
    }

    /// The Routines not in the trash spawned from the Session `note_id`,
    /// each running it, by id; none while the Note is no Session.
    pub fn of_note(&self, note_id: &str) -> Result<Vec<Session>> {
        let Some(setup) = self.setup_of(note_id)? else {
            return Ok(Vec::new());
        };
        let mut out: Vec<Session> = Routines::new(self.store)
            .all()?
            .into_iter()
            .filter(|routine| routine.note_id == note_id)
            .map(|routine| Session {
                routine,
                setup: setup.clone(),
            })
            .collect();
        out.sort_by(|a, b| utf16_cmp(a.id(), b.id()));
        Ok(out)
    }

    /// The Tallies a Session of `entity` may pick, as their Notes, newest
    /// capture first: `entity` is the Note of the Session, or one of its
    /// Routines. Every Tally in the lists, but those outside its Area
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

    /// The Tallies `setup` picks from, as their Notes, in its order:
    /// those of its pool that are Tallies in the lists, a merged one as
    /// the Note it went into, each once, and in a Shared Area only those
    /// of that Area.
    pub fn pool(&self, setup: &SessionSetup) -> Result<Vec<String>> {
        let area = self.store.area_of(&setup.note_id)?;
        let notes = Notes::new(self.store);
        let tallies = Tallies::new(self.store);
        let mut out: Vec<String> = Vec::new();
        for id in &setup.pool {
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
        let pool = self.pool(&session.setup)?;
        let runners = self.runners(&session, None)?;
        let picks = self.wanted(&session, occurrence, &pool, &marks, &runners)?;
        self.shown(session, *occurrence, picks, &pool, &marks)
    }

    /// Today's Occurrences of every Session on `wall` with their picks,
    /// by time, then by Session; written nowhere.
    pub fn today(&self, wall: &WallClock) -> Result<Vec<SessionOccurrence>> {
        Ok(self.day(Day::today(self.store.now(), wall), false)?.0)
    }

    /// Today's Occurrences of every Session on `wall` with their picks,
    /// each pick written for everyone the Session reaches when it is new:
    /// the first, or one that replaces Tallies gone since. The Agenda
    /// calls it as it shows; called again, it writes nothing.
    pub fn pick_today(&self, wall: &WallClock) -> Result<Vec<SessionOccurrence>> {
        let day = Day::today(self.store.now(), wall);
        let (shown, fresh) = self.day(day, false)?;
        if !fresh {
            return Ok(shown);
        }
        // Worked out again and written in one transaction (#207): a pick
        // another connection wrote meanwhile is the pick, not written
        // twice.
        self.store.batch(|| Ok(self.day(day, true)?.0))
    }

    /// "Another one": swaps the Tally of `note_id` picked for
    /// `occurrence` of the Session `id` for the next by the Session's
    /// rule, in its place: one this Occurrence has not offered yet, else
    /// the first not picked. The one swapped out keeps its place in the
    /// rule. Nothing happens when the pool holds no other, or when the
    /// pick has an entry.
    pub fn another(&self, id: &str, occurrence: &Occurrence, note_id: &str) -> Result<()> {
        let session = self.require_due(id, occurrence)?;
        self.require_check_off(id)?;
        let marks = SessionMarks::of(self.store, id)?;
        let pool = self.pool(&session.setup)?;
        let runners = self.runners(&session, None)?;
        let wanted = self.wanted(&session, occurrence, &pool, &marks, &runners)?;
        if !wanted.iter().any(|w| w == note_id) {
            return Err(Error::Invalid(format!("not picked then: {note_id}")));
        }
        // A pick with an entry stays in the pick.
        if !marks
            .entries_for(self.store, &occurrence.key(), note_id)?
            .is_empty()
        {
            return Ok(());
        }
        let mut offered = marks
            .offered
            .get(&occurrence.key())
            .cloned()
            .unwrap_or_default();
        offered.extend(wanted.iter().cloned());
        let order = self.order(&session, occurrence, &pool, &marks, &runners)?;
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
        self.record_sets(id, occurrence, note_id, values, &[], line)
    }

    /// [`Sessions::record`] with the entry's further sets `more` (#264).
    pub fn record_sets(
        &self,
        id: &str,
        occurrence: &Occurrence,
        note_id: &str,
        values: &BTreeMap<String, f64>,
        more: &[BTreeMap<String, f64>],
        line: &str,
    ) -> Result<TallyEntry> {
        let session = self.require_due(id, occurrence)?;
        self.require_check_off(id)?;
        let marks = SessionMarks::of(self.store, id)?;
        let pool = self.pool(&session.setup)?;
        let runners = self.runners(&session, None)?;
        let wanted = self.wanted(&session, occurrence, &pool, &marks, &runners)?;
        if !wanted.iter().any(|w| w == note_id) {
            return Err(Error::Invalid(format!("not picked then: {note_id}")));
        }
        Tallies::new(self.store).add_with(note_id, values, more, line, None, |entry_id| {
            self.write_pick(id, occurrence, &wanted, &marks)?;
            self.store
                .append(id, &keys::entry(occurrence, entry_id), Some(note_id), None)
                .map(drop)
        })
    }

    /// The Occurrences of `day`, today, with their picks, each new one
    /// written with `write`; and whether one was new.
    fn day(&self, day: Day, write: bool) -> Result<(Vec<SessionOccurrence>, bool)> {
        // By time, then by Routine, so that Rolling over several Routines
        // of a Note goes on from the pick written before (ADR 0039).
        let sessions = self.all()?;
        let routines: Vec<Routine> = sessions.iter().map(|s| s.routine.clone()).collect();
        let mut due: Vec<(Session, Occurrence)> = Vec::new();
        for session in sessions {
            for occurrence in session.routine.on(day) {
                due.push((session.clone(), occurrence));
            }
        }
        due.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| utf16_cmp(a.0.id(), b.0.id())));
        let mut shown = Vec::new();
        let mut fresh = false;
        for (session, occurrence) in due {
            let pool = self.pool(&session.setup)?;
            let mut marks = SessionMarks::of(self.store, session.id())?;
            let runners = self.runners(&session, Some(&routines))?;
            let picks = self.wanted(&session, &occurrence, &pool, &marks, &runners)?;
            if self.store.may_check_off(session.id())? && !held(&occurrence, &picks, &marks) {
                fresh = true;
                if write {
                    self.write_pick(session.id(), &occurrence, &picks, &marks)?;
                    marks = SessionMarks::of(self.store, session.id())?;
                }
            }
            shown.push(self.shown(session, occurrence, picks, &pool, &marks)?);
        }
        Ok((shown, fresh))
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
        runners: &[String],
    ) -> Result<Vec<String>> {
        let count = usize::try_from(session.count_on(occurrence.day)).unwrap_or(0);
        let order = self.order(session, occurrence, pool, marks, runners)?;
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
            let Some(id) = wanted.get(i) else {
                break;
            };
            if !recorded(id)? {
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
        runners: &[String],
    ) -> Result<Vec<String>> {
        let mut out = pool.to_vec();
        if session.setup.rule == SessionRule::Random {
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
        // The picks of the Occurrences before this one of every Routine
        // that runs the Session, in schedule order, those at the same
        // moment by their Routine (ADR 0039).
        let mut held: Vec<(String, Arc<SessionMarks>)> = Vec::new();
        for runner in runners {
            if runner != session.id() {
                let marks = SessionMarks::of(self.store, runner)?;
                held.push((runner.clone(), marks));
            }
        }
        let mut before: Vec<(Occurrence, &str, &[String])> = Vec::new();
        let runners = std::iter::once((session.id(), marks)).chain(
            held.iter()
                .map(|(runner, marks)| (runner.as_str(), &**marks)),
        );
        for (runner, marks) in runners {
            for (key, ids) in &marks.picks {
                let Some(o) = Occurrence::parse(key) else {
                    continue;
                };
                let by_time = o.cmp(occurrence);
                if by_time == Ordering::Less
                    || (by_time == Ordering::Equal
                        && utf16_cmp(runner, session.id()) == Ordering::Less)
                {
                    before.push((o, runner, ids));
                }
            }
        }
        before.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| utf16_cmp(a.1, b.1)));
        // When each was picked last: later Occurrences count higher, and
        // within one, those further on in its pick.
        let mut last: HashMap<&str, i64> = HashMap::new();
        let mut picked = 0;
        for (_, _, ids) in &before {
            for id in ids.iter() {
                last.insert(id, picked);
                picked += 1;
            }
        }
        let mut entries: HashMap<&str, usize> = HashMap::new();
        if session.setup.rule == SessionRule::Least {
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

    /// Writes `ids` as the pick of `occurrence`, unless its field holds
    /// that pick already or there is nothing to pick.
    fn write_pick(
        &self,
        id: &str,
        occurrence: &Occurrence,
        ids: &[String],
        marks: &SessionMarks,
    ) -> Result<()> {
        if held(occurrence, ids, marks) {
            return Ok(());
        }
        self.store
            .append(id, &keys::pick(occurrence), Some(&ids.join(",")), None)
            .map(drop)
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

    /// The Routine `id` running a Session, not in the trash.
    fn require(&self, id: &str) -> Result<Session> {
        self.by_id(id)?
            .filter(|s| !s.routine.is_trashed())
            .ok_or_else(|| Error::Invalid(format!("not a Session: {id}")))
    }

    /// The Routines that run `session`: those not in the trash spawned
    /// from its Note, and its own, by id. `routines` are those not in the
    /// trash, read once for many; `None` reads them.
    fn runners(&self, session: &Session, routines: Option<&[Routine]>) -> Result<Vec<String>> {
        let read;
        let routines = match routines {
            Some(routines) => routines,
            None => {
                read = Routines::new(self.store).all()?;
                &read
            }
        };
        let mut out = vec![session.id().to_string()];
        for routine in routines {
            if routine.note_id == session.note_id() && !out.contains(&routine.id) {
                out.push(routine.id.clone());
            }
        }
        out.sort_by(|a, b| utf16_cmp(a, b));
        Ok(out)
    }

    /// The setup of the Session `note_id`.
    fn require_setup(&self, note_id: &str) -> Result<SessionSetup> {
        self.setup_of(note_id)?
            .ok_or_else(|| Error::Invalid(format!("not a Session: {note_id}")))
    }

    /// The setup of the Session `note_id`, in the lists, which this
    /// Device may change as it may change the Note (ADR 0013).
    fn require_managed(&self, note_id: &str) -> Result<SessionSetup> {
        let setup = self.require_setup(note_id)?;
        self.require_changeable(note_id)?;
        Ok(setup)
    }

    /// Requires the Note `note_id` in the lists, which this Device may
    /// change.
    fn require_changeable(&self, note_id: &str) -> Result<()> {
        let listed = Notes::new(self.store)
            .by_id(note_id)?
            .is_some_and(|n| !n.is_trashed() && !n.is_merged());
        if !listed {
            return Err(Error::Invalid(format!(
                "not a Note in the lists: {note_id}"
            )));
        }
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!("may not change {note_id}")));
        }
        Ok(())
    }

    /// Writes a new setup on the Note `note_id`, taking away the
    /// weekdays' counts an earlier one left.
    fn write_setup(
        &self,
        note_id: &str,
        pool: &[String],
        count: i64,
        rule: SessionRule,
    ) -> Result<()> {
        let s = self.store;
        s.append(note_id, keys::POOL, Some(&pool.join(",")), None)?;
        s.append(note_id, keys::COUNT, Some(&count.to_string()), None)?;
        s.append(note_id, keys::RULE, Some(rule.name()), None)?;
        let fields = s.current_fields(note_id)?;
        for weekday in 1..=7 {
            let field = keys::count_on(weekday);
            if fields.get(&field).is_some() {
                s.append(note_id, &field, None, None)?;
            }
        }
        Ok(())
    }

    /// Writes `field` of the Session `setup` as `value` on its Note, with
    /// the rest of a setup an older version left on a Routine in the
    /// same batch.
    fn change(&self, setup: &SessionSetup, field: &str, value: Option<&str>) -> Result<()> {
        self.store.batch(|| {
            self.adopt(setup)?;
            self.write(&setup.note_id, field, value)
        })
    }

    /// Writes the setup an older version left on a Routine onto its
    /// Note, as it reads, so the Note holds it from now on (ADR 0039);
    /// nothing when the Note holds its own.
    fn adopt(&self, setup: &SessionSetup) -> Result<()> {
        if setup.held_by.is_none() {
            return Ok(());
        }
        let (s, id) = (self.store, setup.note_id.as_str());
        s.append(id, keys::POOL, Some(&setup.pool.join(",")), None)?;
        s.append(id, keys::COUNT, Some(&setup.count.to_string()), None)?;
        for (weekday, count) in &setup.counts {
            s.append(
                id,
                &keys::count_on(*weekday),
                Some(&count.to_string()),
                None,
            )?;
        }
        s.append(id, keys::RULE, Some(setup.rule.name()), None)?;
        Ok(())
    }

    /// The Session `id`, due at `occurrence`.
    fn require_due(&self, id: &str, occurrence: &Occurrence) -> Result<Session> {
        let session = self.require(id)?;
        if !session.routine.has(occurrence) {
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

/// Whether `ids` need no writing as the pick of `occurrence` in
/// `marks`: there is nothing to pick, or that is the pick already.
fn held(occurrence: &Occurrence, ids: &[String], marks: &SessionMarks) -> bool {
    ids.is_empty()
        || marks
            .stored
            .get(&occurrence.key())
            .is_some_and(|held| held == ids)
}

#[cfg(test)]
mod tests;
