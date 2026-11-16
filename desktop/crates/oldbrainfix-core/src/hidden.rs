//! Hidden from AI (#267, ADR 0049), under the keys and in the forms the
//! Dart core's `hidden_from_ai.dart` uses: a mark on a Note or an Area,
//! kept like an Override, so it goes to the user's own Devices only and
//! only what they wrote counts. For the AI such a Note, a Note in such an
//! Area, and everything on them do not exist: inside [`Store::for_ai`],
//! where every tool of the AI port runs ([`Store::ai_tool`]), the reads
//! leave them out and an AI Task's drafts cannot reach them. Hidden and
//! shared exclude each other: sharing something hidden is refused with
//! [`Error::HiddenFromAi`]. Decided from the log alone, so a pending
//! change hides or shows nothing.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use crate::checklists::keys as line_keys;
use crate::entry::Entry;
use crate::frogs::keys as frog_keys;
use crate::notes::{Notes, keys as note_keys};
use crate::places::{Places, keys as place_keys};
use crate::routines::keys as routine_keys;
use crate::sessions::keys as session_keys;
use crate::store::{Store, utf16_cmp};
use crate::tallies::keys as tally_keys;
use crate::{Error, Result};

/// Where the marks lie in the log.
pub mod keys {
    /// The id prefix, before the id of the marked Note or Area:
    /// `aiHidden:<id>`, outside every Area like an Override, so it is
    /// sealed under the personal key, goes only to the user's own
    /// Devices, and only what they wrote counts.
    pub const PREFIX: &str = "aiHidden:";
    /// On that entity: [`ON`] while the mark is set; absent, null or any
    /// other value for none.
    pub const HIDDEN: &str = "hidden";
    pub const ON: &str = "on";

    /// The entity holding the mark of the Note or Area `id`.
    pub fn of(id: &str) -> String {
        format!("{PREFIX}{id}")
    }
}

/// What one thread reads as the AI ([`Store::for_ai`]): how deep it is
/// in, whether the hiding itself is worked out just now (reading all),
/// the marks, and what was found hidden so far.
#[derive(Default)]
pub(crate) struct AiView {
    depth: usize,
    seeing_all: usize,
    marks: Option<Arc<BTreeSet<String>>>,
    hides: HashMap<String, bool>,
}

/// Ends a scope of [`AiView`] for the thread, also when its body panics.
struct ViewEnd<'a> {
    store: &'a Store,
    seeing_all: bool,
}

impl Drop for ViewEnd<'_> {
    fn drop(&mut self) {
        let thread = std::thread::current().id();
        let mut scopes = self.store.scopes.lock();
        let gone = match scopes.for_ai.get_mut(&thread) {
            Some(view) if self.seeing_all => {
                view.seeing_all = view.seeing_all.saturating_sub(1);
                false
            }
            Some(view) => {
                view.depth = view.depth.saturating_sub(1);
                view.depth == 0
            }
            None => false,
        };
        if gone {
            scopes.for_ai.remove(&thread);
        }
    }
}

const NOTE: [&str; 1] = ["note:"];

impl Store {
    /// Whether the Note or Area `id` carries the mark itself.
    pub fn is_marked_hidden_from_ai(&self, id: &str) -> Result<bool> {
        let value = self.own_current(&keys::of(id), keys::HIDDEN, &mut HashMap::new())?;
        Ok(value.as_deref() == Some(keys::ON))
    }

    /// The ids of every Note and Area carrying the mark.
    pub fn hidden_from_ai_marks(&self) -> Result<BTreeSet<String>> {
        let mut own = HashMap::new();
        let mut out = BTreeSet::new();
        for entity in self.entities_with_prefix(keys::PREFIX)? {
            if self
                .own_current(&entity, keys::HIDDEN, &mut own)?
                .as_deref()
                == Some(keys::ON)
                && let Some(id) = entity.strip_prefix(keys::PREFIX)
            {
                out.insert(id.to_string());
            }
        }
        Ok(out)
    }

    /// Whether the mark may be set on `id`, as the apps offer it: an Area
    /// that is not shared, or a Note in no Shared Area.
    pub fn may_hide_from_ai(&self, id: &str) -> Result<bool> {
        if Places::new(self).area_by_id(id)?.is_some() {
            return Ok(!self.is_shared(id)?);
        }
        if !id.starts_with("note:") || Notes::new(self).by_id(id)?.is_none() {
            return Ok(false);
        }
        match self.area_of(id)? {
            Some(area) => Ok(!self.is_shared(&area)?),
            None => Ok(true),
        }
    }

    /// Sets the mark on the Note or Area `id`, or takes it off, for all
    /// of the user's own Devices; the same again writes nothing. Taking
    /// it off is never refused. Setting it is refused for an id that is
    /// no Note or Area, and in a Shared Area.
    pub fn set_hidden_from_ai(&self, id: &str, hidden: bool) -> Result<()> {
        if self.is_marked_hidden_from_ai(id)? == hidden {
            return Ok(());
        }
        if hidden && !self.may_hide_from_ai(id)? {
            let known = Places::new(self).area_by_id(id)?.is_some()
                || (id.starts_with("note:") && Notes::new(self).by_id(id)?.is_some());
            return Err(Error::Invalid(if known {
                format!("{id} is in a Shared Area")
            } else {
                format!("not a Note or Area: {id}")
            }));
        }
        self.append(
            &keys::of(id),
            keys::HIDDEN,
            hidden.then_some(keys::ON),
            None,
        )
        .map(drop)
    }

    /// The marked Notes and Areas that hide `entity` from the AI, sorted:
    /// its own mark, its Area's through its Topic, those of the Note it
    /// is merged into, and for a Section, Routine, Frog, Tally entry,
    /// Line or an own choice on a Note, those of the Note. Empty when it
    /// is not hidden. As the Dart core's `hiddenFromAiBy`.
    pub fn hidden_from_ai_by(&self, entity: &str) -> Result<Vec<String>> {
        let marks = self.hidden_from_ai_marks()?;
        self.hidden_by(entity, &marks)
    }

    /// Whether `entity` does not exist for the AI
    /// ([`Store::hidden_from_ai_by`]).
    pub fn is_hidden_from_ai(&self, entity: &str) -> Result<bool> {
        Ok(!self.hidden_from_ai_by(entity)?.is_empty())
    }

    /// The marks that keep the Area `area` from being shared, sorted: its
    /// own, and those hiding a Note filed in it or one of its Topics,
    /// trashed and merged ones too, as their history goes along.
    pub fn hidden_from_ai_in(&self, area: &str) -> Result<Vec<String>> {
        let marks = self.hidden_from_ai_marks()?;
        if marks.is_empty() {
            return Ok(Vec::new());
        }
        let mut found: BTreeSet<String> = BTreeSet::new();
        if marks.contains(area) {
            found.insert(area.to_string());
        }
        for note in self.entities_of_type(note_keys::TYPE_NOTE)? {
            if self.scope_of(&note)? == area {
                found.extend(self.hidden_by(&note, &marks)?);
            }
        }
        Ok(sorted(found.into_iter().collect()))
    }

    /// [`Error::HiddenFromAi`] for `marks`, when there are any.
    pub(crate) fn refuse_hidden_from_ai(marks: Vec<String>) -> Result<()> {
        if marks.is_empty() {
            return Ok(());
        }
        Err(Error::HiddenFromAi(marks))
    }

    /// Those of `entities` that are Hidden from AI
    /// ([`Store::hidden_from_ai_by`]), as a list of cards shows the mark:
    /// the marks read once, and each link followed once for all of them.
    /// Empty while nothing is marked. As the Dart core's
    /// `hiddenFromAiAmong`.
    pub fn hidden_from_ai_among<I, S>(&self, entities: I) -> Result<BTreeSet<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let marks = self.hidden_from_ai_marks()?;
        let mut out = BTreeSet::new();
        if marks.is_empty() {
            return Ok(out);
        }
        let mut own = HashMap::new();
        let mut above = HashMap::new();
        for id in entities {
            let id = id.as_ref();
            if !self
                .walk_hidden(id, &marks, &mut own, &mut above)?
                .is_empty()
            {
                out.insert(id.to_string());
            }
        }
        Ok(out)
    }

    /// [`Store::hidden_from_ai_by`] with the marks read already.
    pub(crate) fn hidden_by(&self, entity: &str, marks: &BTreeSet<String>) -> Result<Vec<String>> {
        if marks.is_empty() {
            return Ok(Vec::new());
        }
        let found = self.walk_hidden(entity, marks, &mut HashMap::new(), &mut HashMap::new())?;
        Ok(sorted(found.into_iter().collect()))
    }

    /// The marks of `marks` reached walking up from `entity`; `own` and
    /// `above` remember, for this walk or several, which Devices are the
    /// user's own and what each id lies in or under
    /// ([`Store::above_for_ai`]).
    fn walk_hidden(
        &self,
        entity: &str,
        marks: &BTreeSet<String>,
        own: &mut HashMap<String, bool>,
        above: &mut HashMap<String, Vec<String>>,
    ) -> Result<BTreeSet<String>> {
        let mut found: BTreeSet<String> = BTreeSet::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut todo = vec![entity.to_string()];
        while let Some(id) = todo.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            if marks.contains(&id) && (id.starts_with("note:") || id.starts_with("area:")) {
                found.insert(id.clone());
            }
            if let Some(held) = above.get(&id) {
                todo.extend(held.iter().cloned());
                continue;
            }
            let held = self.above_for_ai(&id, own)?;
            todo.extend(held.iter().cloned());
            above.insert(id, held);
        }
        Ok(found)
    }

    /// What `id` lies in or under, as the log alone has it and as the
    /// user's own Devices last wrote it (`own` remembers which Devices
    /// are theirs): an Area for a Topic; the Area or Topic and the Note it
    /// is merged into for a Note; the Note for what is on one; the Note or
    /// Area a mark or an own choice is about. Only ids of the kind
    /// expected count.
    fn above_for_ai(&self, id: &str, own: &mut HashMap<String, bool>) -> Result<Vec<String>> {
        let mut held = |field: &str, kinds: &[&str]| -> Result<Vec<String>> {
            let mut latest: Option<Entry> = None;
            let mut mine: Option<Entry> = None;
            for e in self.logged_history(id)? {
                if e.field != field {
                    continue;
                }
                let newer = |held: &Option<Entry>| {
                    held.as_ref()
                        .is_none_or(|h| Store::latest_wins(&e, h) == Ordering::Greater)
                };
                if newer(&mine) && self.written_by_own_in(&e.device, own)? {
                    mine = Some(e.clone());
                }
                if newer(&latest) {
                    latest = Some(e);
                }
            }
            let mut out: Vec<String> = Vec::new();
            for value in [latest, mine].into_iter().flatten().filter_map(|e| e.value) {
                if kinds.iter().any(|k| value.starts_with(k)) && !out.contains(&value) {
                    out.push(value);
                }
            }
            Ok(out)
        };
        let after = |prefix: &str, kinds: &[&str]| {
            id.strip_prefix(prefix)
                .filter(|rest| kinds.iter().any(|k| rest.starts_with(k)))
                .map(str::to_string)
        };
        if id.starts_with("topic:") {
            return held(place_keys::AREA, &["area:"]);
        }
        if id.starts_with("note:") {
            let mut above = held(note_keys::PLACE, &["area:", "topic:"])?;
            above.extend(held(note_keys::MERGED, &NOTE)?);
            return Ok(above);
        }
        for (prefix, field) in [
            ("section:", note_keys::NOTE),
            ("routine:", routine_keys::NOTE),
            (frog_keys::PREFIX, frog_keys::NOTE),
            (tally_keys::ENTRY_PREFIX, tally_keys::NOTE),
            (line_keys::LINE_PREFIX, line_keys::NOTE),
        ] {
            if id.starts_with(prefix) {
                return held(field, &NOTE);
            }
        }
        let one = if id.starts_with(line_keys::AGENDA_PREFIX) {
            after(line_keys::AGENDA_PREFIX, &NOTE)
        } else if id.starts_with(keys::PREFIX) {
            after(keys::PREFIX, &["note:", "area:"])
        } else {
            None
        };
        Ok(one.into_iter().collect())
    }

    /// Runs `body` as the AI sees the store (#267, ADR 0049): on this
    /// thread every read leaves out what is Hidden from AI, as if it had
    /// never been, and nothing worked out from the reads is kept or
    /// taken from what was kept. [`Store::ai_tool`] runs every tool so,
    /// for the AI port and the Ask panel alike. The marks are read once
    /// per scope.
    pub fn for_ai<T>(&self, body: impl FnOnce() -> T) -> T {
        let thread = std::thread::current().id();
        {
            let mut scopes = self.scopes.lock();
            let view = scopes.for_ai.entry(thread).or_default();
            view.depth = view.depth.saturating_add(1);
        }
        let _end = ViewEnd {
            store: self,
            seeing_all: false,
        };
        body()
    }

    /// Runs `body` with this thread reading everything, inside
    /// [`Store::for_ai`]: what works out what is hidden.
    fn seeing_all<T>(&self, body: impl FnOnce() -> T) -> T {
        let thread = std::thread::current().id();
        {
            let mut scopes = self.scopes.lock();
            let Some(view) = scopes.for_ai.get_mut(&thread) else {
                drop(scopes);
                return body();
            };
            view.seeing_all = view.seeing_all.saturating_add(1);
        }
        let _end = ViewEnd {
            store: self,
            seeing_all: true,
        };
        body()
    }

    /// Whether this thread reads as the AI ([`Store::for_ai`]).
    pub(crate) fn reads_for_ai(&self) -> bool {
        let thread = std::thread::current().id();
        self.scopes
            .lock()
            .for_ai
            .get(&thread)
            .is_some_and(|v| v.seeing_all == 0)
    }

    /// Whether the reads on this thread leave `entity` out: inside
    /// [`Store::for_ai`], when it is Hidden from AI.
    pub(crate) fn ai_hides(&self, entity: &str) -> Result<bool> {
        let thread = std::thread::current().id();
        let marks = {
            let scopes = self.scopes.lock();
            let Some(view) = scopes.for_ai.get(&thread) else {
                return Ok(false);
            };
            if view.seeing_all > 0 {
                return Ok(false);
            }
            if let Some(hidden) = view.hides.get(entity) {
                return Ok(*hidden);
            }
            view.marks.clone()
        };
        let marks = match marks {
            Some(marks) => marks,
            None => {
                let marks = Arc::new(self.seeing_all(|| self.hidden_from_ai_marks())?);
                if let Some(view) = self.scopes.lock().for_ai.get_mut(&thread) {
                    view.marks = Some(marks.clone());
                }
                marks
            }
        };
        if marks.is_empty() {
            return Ok(false);
        }
        let hidden = !self
            .seeing_all(|| self.hidden_by(entity, &marks))?
            .is_empty();
        if let Some(view) = self.scopes.lock().for_ai.get_mut(&thread) {
            view.hides.insert(entity.to_string(), hidden);
        }
        Ok(hidden)
    }

    /// Whether an AI Task's draft of `field` on `entity` as `value`
    /// reaches what is Hidden from AI: the entity itself, or the Note,
    /// Area or Topic it is filed into, merged into or put on. A Session's
    /// pool may keep the hidden Tallies it held, which the AI never saw.
    pub(crate) fn draft_reaches_hidden(
        &self,
        entity: &str,
        field: &str,
        value: Option<&str>,
    ) -> Result<bool> {
        self.seeing_all(|| {
            let marks = self.hidden_from_ai_marks()?;
            if marks.is_empty() {
                return Ok(false);
            }
            if !self.hidden_by(entity, &marks)?.is_empty() {
                return Ok(true);
            }
            let Some(value) = value else {
                return Ok(false);
            };
            let held = match field {
                session_keys::POOL => self.logged(entity, field)?.unwrap_or_default(),
                _ => String::new(),
            };
            let linked: Vec<&str> = match field {
                note_keys::PLACE | note_keys::MERGED | note_keys::NOTE | place_keys::AREA => {
                    vec![value]
                }
                session_keys::POOL => value
                    .split(',')
                    .filter(|id| !held.split(',').any(|h| h == *id))
                    .collect(),
                _ => Vec::new(),
            };
            for id in linked {
                if !self.hidden_by(id, &marks)?.is_empty() {
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }

    /// The Tallies of the pool `pool` that are Hidden from AI, read all
    /// the same: what a Session set up by the AI keeps (#267).
    pub(crate) fn hidden_of(&self, pool: &[String]) -> Result<Vec<String>> {
        self.seeing_all(|| {
            let marks = self.hidden_from_ai_marks()?;
            let mut out = Vec::new();
            for id in pool {
                if !self.hidden_by(id, &marks)?.is_empty() {
                    out.push(id.clone());
                }
            }
            Ok(out)
        })
    }

    /// Whether the Area or Topic `place` holds a Note Hidden from AI,
    /// read all the same: one the AI may not delete, as the Note would be
    /// left behind or moved unseen.
    pub(crate) fn holds_hidden_from_ai(&self, place: &str) -> Result<bool> {
        self.seeing_all(|| {
            let marks = self.hidden_from_ai_marks()?;
            if marks.is_empty() {
                return Ok(false);
            }
            for note in self.entities_of_type(note_keys::TYPE_NOTE)? {
                let filed = self.logged(&note, note_keys::PLACE)?;
                let inside = filed.as_deref() == Some(place) || self.scope_of(&note)? == place;
                if inside && !self.hidden_by(&note, &marks)?.is_empty() {
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort_by(|a, b| utf16_cmp(a, b));
    ids
}

#[cfg(test)]
mod tests;
