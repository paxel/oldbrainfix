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
mod tests {
    use super::*;
    use crate::ai_tools::ToolOutput;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
        sync_request_as,
    };
    use crate::notes::NewFile;
    use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule};
    use crate::search::Search;
    use crate::shared::{AreaRole, Invite};
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::{store, ticking};
    use crate::tag_tree::TagTree;
    use crate::{
        Checklists, Frogs, OwnProfile, PairedDevice, Routines, SessionRule, Sessions, Tallies,
    };
    use chrono::{DateTime, Duration, Utc};
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

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

    /// The marks of the refusal `done` ended in, if it was one.
    fn refused<T: std::fmt::Debug>(done: Result<T>) -> Made<Vec<String>> {
        match done {
            Err(Error::HiddenFromAi(marks)) => Ok(marks),
            other => Err(format!("not refused as hidden: {other:?}").into()),
        }
    }

    fn schedule() -> Made<Schedule> {
        let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
        let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e}"))?;
        Ok(Schedule::new(Day::new(2026, 1, 1), &eight, Some(rule)).ok_or("a schedule")?)
    }

    /// A Note made a Tally with one field: its id and the field's.
    fn tally(store: &Store, text: &str, place: Option<&str>) -> Made<(String, String)> {
        let note = Notes::new(store).capture(text, place, None)?.id;
        let field = Tallies::new(store).add_field(&note, text, "", None, None)?;
        Ok((note, field))
    }

    fn png() -> Made<Vec<u8>> {
        let mut out = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(8, 8, image::Rgb([200, 40, 40]))
            .write_to(&mut out, image::ImageFormat::Png)?;
        Ok(out.into_inner())
    }

    /// The tool `name` called with `args` for `task`, on a clock in UTC.
    fn call(store: &Store, task: &str, name: &str, args: &Value) -> Result<ToolOutput> {
        store.ai_tool(task, name, args, &|t: DateTime<Utc>| t.naive_utc())
    }

    #[test]
    fn the_mark_is_set_and_taken_off_and_the_same_again_writes_nothing() -> TestResult {
        let ann = store();
        let note = Notes::new(&ann).capture("Diary", None, None)?.id;
        assert!(!ann.is_marked_hidden_from_ai(&note)?);
        ann.set_hidden_from_ai(&note, true)?;
        assert!(ann.is_marked_hidden_from_ai(&note)?);
        let marks: Vec<String> = ann.hidden_from_ai_marks()?.into_iter().collect();
        assert_eq!(marks, std::slice::from_ref(&note));
        let written = ann.all_entries()?.len();
        ann.set_hidden_from_ai(&note, true)?;
        assert_eq!(ann.all_entries()?.len(), written);
        ann.set_hidden_from_ai(&note, false)?;
        assert!(!ann.is_marked_hidden_from_ai(&note)?);
        assert_eq!(ann.current(&keys::of(&note), keys::HIDDEN)?, None);
        ann.set_hidden_from_ai(&note, false)?;
        assert_eq!(ann.all_entries()?.len(), written + 1);
        Ok(())
    }

    #[test]
    fn a_value_no_app_writes_is_no_mark() -> TestResult {
        let ann = store();
        let note = Notes::new(&ann).capture("Diary", None, None)?.id;
        ann.append(&keys::of(&note), keys::HIDDEN, Some("yes"), None)?;
        assert!(!ann.is_marked_hidden_from_ai(&note)?);
        assert!(!ann.is_hidden_from_ai(&note)?);
        Ok(())
    }

    #[test]
    fn only_a_note_or_an_area_is_marked() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let area = places.add_area("Home")?.id;
        let topic = places.add_topic(&area, "Kitchen")?.id;
        assert!(!ann.may_hide_from_ai(&topic)?);
        assert!(ann.set_hidden_from_ai(&topic, true).is_err());
        assert!(ann.set_hidden_from_ai("note:gone", true).is_err());
        ann.set_hidden_from_ai(&area, true)?;
        assert!(ann.is_marked_hidden_from_ai(&area)?);
        Ok(())
    }

    #[test]
    fn what_is_on_a_hidden_note_or_in_a_hidden_area_is_hidden() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let area = places.add_area("Health")?.id;
        let topic = places.add_topic(&area, "Back")?.id;
        let in_topic = notes.capture("Physio", Some(&topic), None)?.id;
        let in_area = notes.capture("Doctor", Some(&area), None)?.id;
        let alone = notes.capture("Diary", None, None)?.id;
        let open = notes.capture("Shopping", None, None)?.id;
        let section = notes.amend(&in_topic, "Exercises")?.id;
        let routine = Routines::new(&ann).spawn(&alone, &schedule()?)?.id;
        let frog = Frogs::new(&ann).spawn(&in_area)?.id;
        let list = notes.capture("Bag\nShoes", None, None)?.id;
        Checklists::new(&ann).make(&list)?;
        let line = Checklists::new(&ann).add(&list, "Towel", None, false)?;
        let merged = notes.capture("Notes on the diary", None, None)?.id;
        notes.merge(&merged, &alone)?;
        for id in [&area, &alone, &list] {
            ann.set_hidden_from_ai(id, true)?;
        }
        assert_eq!(
            ann.hidden_from_ai_by(&in_topic)?,
            std::slice::from_ref(&area)
        );
        assert_eq!(
            ann.hidden_from_ai_by(&section)?,
            std::slice::from_ref(&area)
        );
        assert_eq!(ann.hidden_from_ai_by(&frog)?, std::slice::from_ref(&area));
        assert_eq!(ann.hidden_from_ai_by(&topic)?, std::slice::from_ref(&area));
        assert_eq!(
            ann.hidden_from_ai_by(&routine)?,
            std::slice::from_ref(&alone)
        );
        assert_eq!(
            ann.hidden_from_ai_by(&merged)?,
            std::slice::from_ref(&alone)
        );
        assert_eq!(ann.hidden_from_ai_by(&line)?, std::slice::from_ref(&list));
        let agenda = format!("{}{list}", line_keys::AGENDA_PREFIX);
        assert_eq!(ann.hidden_from_ai_by(&agenda)?, std::slice::from_ref(&list));
        assert!(!ann.is_hidden_from_ai(&open)?);
        notes.file(&in_area, &places.add_area("Work")?.id)?;
        assert!(!ann.is_hidden_from_ai(&in_area)?);
        assert!(!ann.is_hidden_from_ai(&frog)?);
        ann.set_hidden_from_ai(&area, false)?;
        assert!(!ann.is_hidden_from_ai(&in_topic)?);
        Ok(())
    }

    #[test]
    fn those_of_a_list_that_are_hidden_are_the_ones_each_walk_finds() -> TestResult {
        let ann = device(1, "Ann")?;
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let area = places.add_area("Health")?.id;
        let topic = places.add_topic(&area, "Back")?.id;
        let in_topic = notes.capture("Physio", Some(&topic), None)?.id;
        let in_area = notes.capture("Doctor", Some(&area), None)?.id;
        let alone = notes.capture("Diary", None, None)?.id;
        let open = notes.capture("Shopping", None, None)?.id;
        let merged = notes.capture("Notes on the diary", None, None)?.id;
        notes.merge(&merged, &alone)?;
        let all = [&in_topic, &in_area, &alone, &open, &merged, &topic, &area];
        assert!(ann.hidden_from_ai_among(all)?.is_empty());
        ann.set_hidden_from_ai(&area, true)?;
        ann.set_hidden_from_ai(&alone, true)?;
        let each = || -> Made<BTreeSet<String>> {
            let mut out = BTreeSet::new();
            for id in all {
                if ann.is_hidden_from_ai(id)? {
                    out.insert(id.clone());
                }
            }
            Ok(out)
        };
        let want: BTreeSet<String> = [&in_topic, &in_area, &alone, &merged, &topic, &area]
            .into_iter()
            .cloned()
            .collect();
        assert_eq!(each()?, want);
        assert_eq!(ann.hidden_from_ai_among(all)?, each()?);
        assert_eq!(
            ann.hidden_from_ai_among([&open, &in_area])?,
            BTreeSet::from([in_area.clone()])
        );
        notes.file(&in_area, &places.add_area("Work")?.id)?;
        ann.set_hidden_from_ai(&alone, false)?;
        let want: BTreeSet<String> = [&in_topic, &topic, &area].into_iter().cloned().collect();
        assert_eq!(ann.hidden_from_ai_among(all)?, want);
        assert_eq!(ann.hidden_from_ai_among(all)?, each()?);
        Ok(())
    }

    #[test]
    fn it_travels_between_own_devices() -> TestResult {
        let ann = device(1, "Ann")?;
        let desk = device(3, "Ann at the desk")?;
        exchange(&desk, &ann, false)?;
        remember(&ann, &desk, false)?;
        remember(&desk, &ann, false)?;
        let note = Notes::new(&ann).capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&note, true)?;
        exchange(&desk, &ann, false)?;
        assert!(desk.is_hidden_from_ai(&note)?);
        desk.set_hidden_from_ai(&note, false)?;
        exchange(&desk, &ann, false)?;
        assert!(!ann.is_hidden_from_ai(&note)?);
        Ok(())
    }

    #[test]
    fn the_mark_is_not_set_in_a_shared_area_but_always_taken_off() -> TestResult {
        let (ann, _, area) = household()?;
        let note = Notes::new(&ann).capture("Milk", Some(&area), None)?.id;
        assert!(!ann.may_hide_from_ai(&area)?);
        assert!(!ann.may_hide_from_ai(&note)?);
        assert!(ann.set_hidden_from_ai(&area, true).is_err());
        assert!(ann.set_hidden_from_ai(&note, true).is_err());
        assert!(ann.hidden_from_ai_marks()?.is_empty());
        // Marked before it was shared, as two own Devices apart could.
        ann.append(&keys::of(&note), keys::HIDDEN, Some(keys::ON), None)?;
        assert!(ann.is_hidden_from_ai(&note)?);
        ann.set_hidden_from_ai(&note, false)?;
        assert!(!ann.is_hidden_from_ai(&note)?);
        Ok(())
    }

    #[test]
    fn it_never_reaches_another_person() -> TestResult {
        let (ann, ben, area) = household()?;
        let diary = Notes::new(&ann).capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&diary, true)?;
        Notes::new(&ann).capture("Milk", Some(&area), None)?;
        exchange(&ben, &ann, true)?;
        assert!(ben.entities_with_prefix(keys::PREFIX)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_mark_another_persons_device_wrote_counts_for_nothing() -> TestResult {
        let (ann, ben, _) = household()?;
        let diary = Notes::new(&ann).capture("Diary", None, None)?.id;
        exchange(&ben, &ann, true)?;
        // As a modified app would: Ben slips a mark into Ann's log.
        let forged = ben.append(&keys::of(&diary), keys::HIDDEN, Some(keys::ON), None)?;
        let mut report = ImportReport::default();
        ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
        assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
        assert!(!ann.is_marked_hidden_from_ai(&diary)?);
        assert!(!ann.is_hidden_from_ai(&diary)?);
        Ok(())
    }

    #[test]
    fn a_hidden_note_is_not_filed_into_a_shared_area() -> TestResult {
        let (ann, _, area) = household()?;
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let diary = notes.capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&diary, true)?;
        let topic = places.add_topic(&area, "Kitchen")?.id;
        let written = ann.all_entries()?.len();
        assert_eq!(
            refused(notes.file(&diary, &area))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(
            refused(notes.file(&diary, &topic))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(ann.all_entries()?.len(), written);
        let health = places.add_area("Health")?.id;
        ann.set_hidden_from_ai(&health, true)?;
        let doctor = notes.capture("Doctor", Some(&health), None)?.id;
        assert_eq!(refused(notes.file(&doctor, &area))?, [health]);
        notes.file(&diary, &places.add_area("Private")?.id)?;
        notes.file(&doctor, &places.add_area("Work")?.id)?;
        notes.file(&doctor, &area)?;
        assert!(!ann.is_hidden_from_ai(&doctor)?);
        Ok(())
    }

    #[test]
    fn hidden_and_shared_notes_do_not_merge_either_way() -> TestResult {
        let (ann, _, area) = household()?;
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let diary = notes.capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&diary, true)?;
        let health = places.add_area("Health")?.id;
        ann.set_hidden_from_ai(&health, true)?;
        let doctor = notes.capture("Doctor", Some(&health), None)?.id;
        let topic = places.add_topic(&area, "Kitchen")?.id;
        let milk = notes.capture("Milk", Some(&area), None)?.id;
        let eggs = notes.capture("Eggs", Some(&topic), None)?.id;
        let plain = notes.capture("Plain", None, None)?.id;
        let written = ann.all_entries()?.len();
        // Hidden into shared, by the marks hiding what merges.
        assert_eq!(
            refused(notes.merge(&diary, &milk))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(
            refused(notes.merge(&doctor, &eggs))?,
            std::slice::from_ref(&health)
        );
        // Shared into hidden, by the marks hiding the target.
        assert_eq!(
            refused(notes.merge(&milk, &diary))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(
            refused(notes.merge(&eggs, &doctor))?,
            std::slice::from_ref(&health)
        );
        let mut both = vec![diary.clone(), health.clone()];
        both.sort();
        assert_eq!(
            refused(notes.merge_all([&doctor, &plain, &diary], &milk))?,
            both
        );
        assert_eq!(
            refused(notes.merge_all([&plain, &milk], &diary))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(ann.all_entries()?.len(), written);
        // Hidden with hidden, shared with shared, and anything with a
        // Note of neither merge as before.
        notes.merge(&doctor, &diary)?;
        notes.merge(&eggs, &milk)?;
        notes.merge_all([&plain], &diary)?;
        assert!(notes.by_id(&doctor)?.is_some_and(|n| n.is_merged()));
        assert!(notes.by_id(&eggs)?.is_some_and(|n| n.is_merged()));
        Ok(())
    }

    #[test]
    fn a_link_another_persons_device_wrote_shows_nothing_hidden() -> TestResult {
        let (ann, ben, _) = household()?;
        let notes = Notes::new(&ann);
        let health = Places::new(&ann).add_area("Health")?.id;
        let doctor = notes.capture("Doctor", Some(&health), None)?.id;
        let diary = notes.capture("Diary", None, None)?.id;
        let later = notes.capture("Diary, later", None, None)?.id;
        notes.merge(&later, &diary)?;
        ann.set_hidden_from_ai(&health, true)?;
        ann.set_hidden_from_ai(&diary, true)?;
        exchange(&ben, &ann, true)?;
        // As a modified app would: Ben slips entries into Ann's log that
        // take her Notes out of the hidden Area and the hidden Note.
        let forged = [
            ben.append(&doctor, note_keys::PLACE, None, None)?.wire(),
            ben.append(&later, note_keys::MERGED, None, None)?.wire(),
        ];
        let mut report = ImportReport::default();
        ann.apply_entries(&forged, &[], Some(&ben.device_id()?), &mut report)?;
        assert_eq!(ann.logged(&doctor, note_keys::PLACE)?, None);
        assert_eq!(ann.hidden_from_ai_by(&doctor)?, [health]);
        assert_eq!(ann.hidden_from_ai_by(&later)?, [diary]);
        Ok(())
    }

    #[test]
    fn sharing_a_hidden_area_or_one_holding_hidden_notes_is_refused() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let health = places.add_area("Health")?.id;
        notes.capture("Doctor", Some(&health), None)?;
        ann.set_hidden_from_ai(&health, true)?;
        let written = ann.all_entries()?.len();
        assert_eq!(refused(ann.share(&health))?, std::slice::from_ref(&health));
        assert_eq!(ann.all_entries()?.len(), written);
        assert!(!ann.is_shared(&health)?);
        let home = places.add_area("Home")?.id;
        let kitchen = places.add_topic(&home, "Kitchen")?.id;
        let recipe = notes.capture("Recipe", Some(&kitchen), None)?.id;
        notes.capture("Diary", Some(&home), None)?;
        let old = notes.capture("Old", Some(&home), None)?.id;
        ann.set_hidden_from_ai(&recipe, true)?;
        ann.set_hidden_from_ai(&old, true)?;
        Routines::new(&ann).trash_note(&old)?;
        let before = ann.all_entries()?.len();
        let mut both = vec![recipe.clone(), old.clone()];
        both.sort_by(|a, b| utf16_cmp(a, b));
        assert_eq!(refused(ann.share(&home))?, both);
        assert_eq!(ann.all_entries()?.len(), before);
        ann.set_hidden_from_ai(&recipe, false)?;
        ann.set_hidden_from_ai(&old, false)?;
        assert_eq!(ann.share(&home)?, home);
        assert!(ann.is_shared(&home)?);
        // A starter Area, before it moves to fresh ids.
        places.plant_starter_set(&["household"])?;
        let household = crate::places::starter_id(place_keys::TYPE_AREA, "household");
        ann.set_hidden_from_ai(&household, true)?;
        let written = ann.all_entries()?.len();
        assert_eq!(
            refused(ann.share(&household))?,
            std::slice::from_ref(&household)
        );
        assert_eq!(ann.all_entries()?.len(), written);
        Ok(())
    }

    #[test]
    fn a_hidden_note_is_not_sent_nor_its_cancellation() -> TestResult {
        let ann = device(1, "Ann")?;
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let health = places.add_area("Health")?.id;
        let doctor = notes.capture("Doctor", Some(&health), None)?.id;
        let diary = notes.capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&health, true)?;
        ann.set_hidden_from_ai(&diary, true)?;
        let written = ann.all_entries()?.len();
        assert_eq!(refused(ann.sent_item_of(&doctor, false))?, [health]);
        assert_eq!(
            refused(ann.sent_item_of(&diary, false))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(
            refused(ann.sent_item_of(&diary, true))?,
            std::slice::from_ref(&diary)
        );
        assert_eq!(ann.all_entries()?.len(), written);
        ann.set_hidden_from_ai(&diary, false)?;
        assert_eq!(ann.sent_item_of(&diary, false)?.text, "Diary");
        Ok(())
    }

    #[test]
    fn a_pack_holding_something_hidden_is_refused() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let none = HashSet::new();
        let health = places.add_area("Health")?.id;
        let back = places.add_topic(&health, "Back")?.id;
        let physio = notes.capture("Physio", Some(&back), None)?.id;
        notes.capture("Doctor", Some(&health), None)?;
        let diary = notes.capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&physio, true)?;
        ann.set_hidden_from_ai(&diary, true)?;
        assert_eq!(
            refused(ann.pack_of(&health, &none))?,
            std::slice::from_ref(&physio)
        );
        assert_eq!(
            refused(ann.pack_of(&back, &none))?,
            std::slice::from_ref(&physio)
        );
        assert_eq!(
            refused(ann.pack_of(&diary, &none))?,
            std::slice::from_ref(&diary)
        );
        ann.set_hidden_from_ai(&physio, false)?;
        assert_eq!(ann.pack_of(&health, &none)?.notes.len(), 2);
        ann.set_hidden_from_ai(&health, true)?;
        assert_eq!(refused(ann.pack_of(&back, &none))?, [health]);
        // A Session picking from a hidden Tally.
        let (laps, _) = tally(&ann, "Laps", None)?;
        let training = notes.capture("Training", None, None)?.id;
        Sessions::new(&ann).make(
            &training,
            std::slice::from_ref(&laps),
            1,
            SessionRule::Rolling,
        )?;
        ann.set_hidden_from_ai(&laps, true)?;
        assert_eq!(refused(ann.pack_of(&training, &none))?, [laps]);
        Ok(())
    }

    #[test]
    fn a_pack_of_everything_leaves_out_what_is_hidden_and_a_backup_keeps_it() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let none = HashSet::new();
        let health = places.add_area("Health")?.id;
        notes.capture("Doctor", Some(&health), None)?;
        let diary = notes.capture("Diary", None, None)?.id;
        notes.capture("Shopping", None, None)?;
        let (laps, _) = tally(&ann, "Laps", None)?;
        let training = notes.capture("Training", None, None)?.id;
        Sessions::new(&ann).make(
            &training,
            std::slice::from_ref(&laps),
            1,
            SessionRule::Rolling,
        )?;
        for id in [&health, &diary, &laps] {
            ann.set_hidden_from_ai(id, true)?;
        }
        let texts: Vec<String> = ann
            .pack_of_all(&none)?
            .notes
            .into_iter()
            .map(|n| n.text)
            .collect();
        assert_eq!(texts, ["Training", "Shopping"]);
        let size_of = |_: &str, _: &str| None;
        let everything = ann.pack_of_everything(&none, |a| a.name.clone(), &size_of)?;
        assert_eq!(everything.pack.notes.len(), 5);
        Ok(())
    }

    /// What every reading tool without an id hands out, one line each.
    fn read_all(store: &Store, task: &str, ids: &[(&str, &String)]) -> Made<Vec<String>> {
        let mut out = Vec::new();
        let mut reads = vec![
            ("list_notes", json!({})),
            ("list_notes", json!({"search": "secret"})),
            ("list_notes", json!({"which": "trash"})),
            ("list_notes", json!({"which": "unsorted"})),
            ("list_places", json!({})),
            ("list_tags", json!({})),
            ("list_routines", json!({})),
            ("list_routines", json!({"trash": true})),
            ("read_settings", json!({})),
            ("read_check_offs", json!({})),
            ("read_skips", json!({})),
            ("read_eaten_frogs", json!({})),
        ];
        for (name, id) in ids {
            let key = match *name {
                "read_note" => "id",
                _ => "note",
            };
            reads.push((name, json!({ key: id })));
        }
        for (name, args) in reads {
            out.push(format!(
                "{name} {args}: {:?}",
                call(store, task, name, &args)?
            ));
        }
        Ok(out)
    }

    #[test]
    fn for_the_ai_what_is_hidden_does_not_exist() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let work = places.add_area("Work")?.id;
        let plan = notes.capture("Plan #work", Some(&work), None)?.id;
        Routines::new(&ann).spawn(&plan, &schedule()?)?;
        let (laps, field) = tally(&ann, "Laps", None)?;
        Tallies::new(&ann).add(&laps, &BTreeMap::from([(field, 3.0)]), "", None)?;
        let training = notes.capture("Training", None, None)?.id;
        Sessions::new(&ann).make(
            &training,
            std::slice::from_ref(&laps),
            1,
            SessionRule::Rolling,
        )?;
        let task = ann.open_ai_task()?;
        let visible = [
            ("read_note", &plan),
            ("read_note", &training),
            ("read_tally_entries", &laps),
            ("read_note_history", &plan),
        ];
        let before = read_all(&ann, &task, &visible)?;

        // Everything a Note can carry, in a hidden Area and on a hidden
        // Note, and a hidden Tally in the pool of the visible Session.
        let health = places.add_area("Health secret")?.id;
        let back = places.add_topic(&health, "Back secret")?.id;
        let file = NewFile {
            name: "secret.txt",
            media_type: "text/plain",
            bytes: b"secret",
            pages: None,
        };
        let physio = notes.capture_with_files(
            "Physio secret #health/back",
            Some(&back),
            None,
            &[png()?],
            &[],
            &[file],
        )?;
        notes.amend(&physio.id, "secret exercises")?;
        let hash = physio.attachments.first().ok_or("a file")?.hash.clone();
        notes.set_attachment_text(&physio.id, &hash, "secret text")?;
        let routine = Routines::new(&ann).spawn(&physio.id, &schedule()?)?.id;
        let first = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
        Routines::new(&ann).mark_done(&routine, &first, None)?;
        let frog = Frogs::new(&ann).spawn(&physio.id)?;
        Frogs::new(&ann).mark_done(&frog.id, None)?;
        let bag = notes.capture("Bag secret\nShoes", Some(&health), None)?.id;
        Checklists::new(&ann).make(&bag)?;
        let (push, field) = tally(&ann, "Push-ups secret", Some(&health))?;
        Tallies::new(&ann).add(&push, &BTreeMap::from([(field, 9.0)]), "", None)?;
        let old = notes.capture("Old secret", Some(&health), None)?.id;
        Routines::new(&ann).trash_note(&old)?;
        let diary = notes.capture("Diary secret #private", None, None)?.id;
        Routines::new(&ann).spawn(&diary, &schedule()?)?;
        let extra = notes.capture("Extra secret", None, None)?.id;
        notes.merge(&extra, &diary)?;
        Sessions::new(&ann).set_pool(&training, &[laps.clone(), push.clone()])?;
        ann.set_hidden_from_ai(&health, true)?;
        ann.set_hidden_from_ai(&diary, true)?;
        // Worked out for the user first, so nothing kept leaks.
        assert!(!TagTree::new(&ann).tree(&HashSet::new())?.is_empty());
        assert_eq!(Search::new(&ann).notes("secret", None)?.len(), 4);

        assert_eq!(read_all(&ann, &task, &visible)?, before);
        // Read by its id, a hidden one reads as one there never was.
        let never = "note:00000000000000000000000000000000";
        let photo = physio.photos.first().ok_or("a photo")?.clone();
        let by_id = [
            ("read_note", json!({"id": physio.id})),
            ("read_note", json!({"id": extra})),
            ("read_photo", json!({"note": physio.id, "photo": photo})),
            (
                "read_attachment_text",
                json!({"file": hash, "note": physio.id}),
            ),
            ("list_notes", json!({"place": health})),
            ("list_routines", json!({"note": physio.id})),
            ("read_tally_entries", json!({"note": push})),
            ("read_ticks", json!({"note": bag})),
            ("read_check_offs", json!({"note": physio.id})),
            ("read_note_history", json!({"note": diary})),
            ("read_session_picks", json!({"id": routine})),
        ];
        for (name, args) in by_id {
            let hidden = format!("{:?}", call(&ann, &task, name, &args)?);
            let mut text = args.to_string();
            let mut shown = hidden.clone();
            for id in [&physio.id, &extra, &health, &push, &bag, &diary, &routine] {
                text = text.replace(id.as_str(), never);
                shown = shown.replace(id.as_str(), never);
            }
            let unknown = format!(
                "{:?}",
                call(&ann, &task, name, &serde_json::from_str(&text)?)?
            );
            assert_eq!(shown, unknown, "{name}");
        }
        // The user still sees all of it.
        assert_eq!(Search::new(&ann).notes("secret", None)?.len(), 4);
        assert!(
            ann.pending_set(&task)?
                .ok_or("the task")?
                .drafts()
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn writes_cannot_reach_what_is_hidden() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let work = places.add_area("Work")?.id;
        let plan = notes.capture("Plan", Some(&work), None)?.id;
        let health = places.add_area("Health")?.id;
        let back = places.add_topic(&health, "Back")?.id;
        let physio = notes.capture("Physio", Some(&back), None)?.id;
        let routine = Routines::new(&ann).spawn(&physio, &schedule()?)?.id;
        let bag = notes.capture("Bag\nShoes", Some(&health), None)?.id;
        Checklists::new(&ann).make(&bag)?;
        let line = Checklists::new(&ann).add(&bag, "Towel", None, false)?;
        let diary = notes.capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&health, true)?;
        ann.set_hidden_from_ai(&diary, true)?;
        let task = ann.open_ai_task()?;
        let never = "note:00000000000000000000000000000000";
        let writes = [
            ("edit_note", json!({"id": diary, "text": "x"})),
            ("amend_note", json!({"id": physio, "text": "x"})),
            ("file_note", json!({"id": plan, "place": health})),
            ("file_note", json!({"id": plan, "place": back})),
            ("file_note", json!({"id": diary, "place": work})),
            ("merge_notes", json!({"id": plan, "into": diary})),
            ("merge_notes", json!({"id": diary, "into": plan})),
            (
                "set_importance",
                json!({"id": diary, "importance": "important"}),
            ),
            ("trash_note", json!({"id": diary})),
            ("restore_note", json!({"id": diary})),
            ("capture_note", json!({"place": back, "text": "x"})),
            ("add_topic", json!({"area": health, "name": "x"})),
            ("rename_place", json!({"id": health, "name": "x"})),
            (
                "set_area_colour",
                json!({"colour": "#ff0000", "id": health}),
            ),
            ("delete_place", json!({"id": back, "notesGo": "unsorted"})),
            ("delete_place", json!({"id": health, "notesGo": "trash"})),
            (
                "add_routine",
                json!({"note": diary, "start": "2026-01-01", "times": ["08:00"]}),
            ),
            (
                "reschedule_routine",
                json!({"id": routine, "start": "2026-01-02", "times": ["09:00"]}),
            ),
            ("trash_routine", json!({"id": routine})),
            (
                "make_tally",
                json!({"fields": [{"name": "Laps"}], "id": diary}),
            ),
            ("make_list", json!({"id": diary, "list": "checklist"})),
            ("add_line", json!({"id": bag, "text": "x"})),
            ("rename_line", json!({"id": line, "text": "x"})),
            ("remove_line", json!({"id": line})),
            (
                "set_checklist_agenda",
                json!({"agenda": "count", "id": bag}),
            ),
            ("make_frog", json!({"id": diary})),
            ("set_text_format", json!({"format": "code", "id": diary})),
        ];
        for (name, args) in writes {
            let hidden = format!("{:?}", call(&ann, &task, name, &args)?);
            let mut text = args.to_string();
            let mut shown = hidden.clone();
            for id in [&diary, &physio, &health, &back, &routine, &bag, &line] {
                let kind = id.split_once(':').map_or("", |(kind, _)| kind);
                let unknown = format!("{kind}:00000000000000000000000000000000");
                text = text.replace(id.as_str(), &unknown);
                shown = shown.replace(id.as_str(), &unknown);
            }
            let unknown = format!(
                "{:?}",
                call(&ann, &task, name, &serde_json::from_str(&text)?)?
            );
            assert_eq!(shown, unknown, "{name} {args}");
            assert!(hidden.contains("is_error: true"), "{name} {args}: {hidden}");
        }
        assert!(
            ann.pending_set(&task)?
                .ok_or("the task")?
                .drafts()
                .is_empty()
        );
        assert!(!never.is_empty());
        Ok(())
    }

    #[test]
    fn a_draft_reaching_something_hidden_is_refused_whatever_writes_it() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let plan = notes.capture("Plan", None, None)?.id;
        let health = places.add_area("Health")?.id;
        let diary = notes.capture("Diary", None, None)?.id;
        ann.set_hidden_from_ai(&health, true)?;
        ann.set_hidden_from_ai(&diary, true)?;
        let task = ann.open_ai_task()?;
        // Outside the AI's view the reads see everything: the draft's
        // check alone stands in the way.
        assert!(ann.drafting(&task, || notes.file(&plan, &health)).is_err());
        assert!(ann.drafting(&task, || notes.merge(&plan, &diary)).is_err());
        assert!(ann.drafting(&task, || notes.edit(&diary, "x")).is_err());
        assert!(
            ann.pending_set(&task)?
                .ok_or("the task")?
                .drafts()
                .is_empty()
        );
        ann.drafting(&task, || notes.edit(&plan, "Plan B"))?;
        Ok(())
    }

    #[test]
    fn picks_history_and_a_merged_note_name_nothing_hidden() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let (laps, _) = tally(&ann, "Laps", None)?;
        let (push, _) = tally(&ann, "Push-ups", None)?;
        let training = notes.capture("Training", None, None)?.id;
        let pool = [laps.clone(), push.clone()];
        let session =
            Sessions::new(&ann).spawn(&training, &schedule()?, &pool, 2, SessionRule::Rolling)?;
        Sessions::new(&ann).pick_today(&|t: DateTime<Utc>| t.naive_utc())?;
        let health = places.add_area("Health")?.id;
        let plan = notes.capture("Plan", Some(&health), None)?.id;
        notes.file(&plan, &places.add_area("Work")?.id)?;
        let diary = notes.capture("Diary secret", None, None)?.id;
        notes.amend(&diary, "Amend secret")?;
        let reminder = Routines::new(&ann).spawn(&diary, &schedule()?)?.id;
        notes.merge(&diary, &plan)?;
        for id in [&push, &health, &diary] {
            ann.set_hidden_from_ai(id, true)?;
        }
        let task = ann.open_ai_task()?;
        let picks = format!(
            "{:?}",
            call(
                &ann,
                &task,
                "read_session_picks",
                &json!({"id": session.id()})
            )?
        );
        assert!(picks.contains(&laps) && !picks.contains(&push), "{picks}");
        for (name, args) in [
            ("read_note_history", json!({"note": plan})),
            ("read_note", json!({"id": plan})),
            ("list_routines", json!({})),
            ("list_notes", json!({"search": "secret"})),
        ] {
            let out = format!("{:?}", call(&ann, &task, name, &args)?);
            assert!(!out.contains("is_error: true"), "{name}: {out}");
            for hidden in [&health, &diary, &reminder] {
                assert!(!out.contains(hidden.as_str()), "{name}: {out}");
            }
            assert!(!out.contains("secret"), "{name}: {out}");
        }
        Ok(())
    }

    #[test]
    fn a_sessions_pool_keeps_its_hidden_tallies_and_a_place_holding_one_stays() -> TestResult {
        let ann = store();
        let places = Places::new(&ann);
        let notes = Notes::new(&ann);
        let (laps, _) = tally(&ann, "Laps", None)?;
        let (push, _) = tally(&ann, "Push-ups", None)?;
        let (sit, _) = tally(&ann, "Sit-ups", None)?;
        let training = notes.capture("Training", None, None)?.id;
        Sessions::new(&ann).make(
            &training,
            &[laps.clone(), push.clone()],
            1,
            SessionRule::Rolling,
        )?;
        ann.set_hidden_from_ai(&push, true)?;
        let home = places.add_area("Home")?.id;
        let diary = notes.capture("Diary", Some(&home), None)?.id;
        ann.set_hidden_from_ai(&diary, true)?;
        let task = ann.open_ai_task()?;
        let read = call(&ann, &task, "read_note", &json!({"id": training}))?;
        assert!(!format!("{read:?}").contains(&push), "{read:?}");
        let set = call(
            &ann,
            &task,
            "set_session",
            &json!({"id": training, "pool": [sit]}),
        )?;
        assert!(!set.is_error, "{set:?}");
        let deleted = call(
            &ann,
            &task,
            "delete_place",
            &json!({"id": home, "notesGo": "unsorted"}),
        )?;
        assert!(deleted.is_error, "{deleted:?}");
        ann.accept_pending(&task)?;
        let setup = Sessions::new(&ann)
            .setup_of(&training)?
            .ok_or("a Session")?;
        assert_eq!(setup.pool, [sit, push]);
        assert!(places.area_by_id(&home)?.is_some());
        assert_eq!(notes.by_id(&diary)?.and_then(|n| n.place), Some(home));
        Ok(())
    }
}
