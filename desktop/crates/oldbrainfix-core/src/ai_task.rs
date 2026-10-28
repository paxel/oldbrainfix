//! AI Tasks (CONTEXT.md, ADR 0026, #112): one job an AI client does
//! through the desktop's AI port ([`crate::ai_port`]), every change of
//! it a Pending Change (ADR 0022) of one set of kind [`AI_TASK_KIND`],
//! shown while it waits and kept only when the user accepts it.
//!
//! The tools ([`crate::ai_tools`]) write through the same calls as the
//! screens. On the thread that answers a tool, inside
//! [`Store::drafting`], every [`Store::append`] becomes a draft of the
//! set instead, after two checks: the field is one an AI Task may write
//! at all (nothing that reaches other people or decides trust), and this
//! Device's role in the Shared Area, as the log has it, allows it. The
//! user's own edits on other threads, or after the call, reach the log
//! as ever.
//!
//! While a shown set waits, sync waits too ([`Store::sync_paused`]), and
//! the alarms fire from the log alone ([`Store::log_only`]): an alarm
//! never moves or comes because of a change nobody accepted yet.

use std::collections::{BTreeMap, HashMap};
use std::thread::ThreadId;

use chrono::{DateTime, Utc};

use crate::entry::Entry;
use crate::notes::keys as note_keys;
use crate::pending::{GiveWay, PendingDraft};
use crate::places::keys as place_keys;
use crate::routines::keys as routine_keys;
use crate::store::Store;
use crate::{Error, Result};

/// The kind of pending set an AI Task is.
pub const AI_TASK_KIND: &str = "ai-task";

/// The fields an AI Task may write, by the type of the entity: what the
/// user can do with Notes, Areas, Topics, Merges, the trash, Importance,
/// Routines and Reminders. Not a role, a block, an Override, a Sent
/// Item's link, a Profile, a check-off, nor a photo or recording.
const NOTE_FIELDS: [&str; 9] = [
    note_keys::TYPE,
    note_keys::CAPTURED,
    note_keys::TEXT,
    note_keys::PLACE,
    note_keys::IMPORTANCE,
    note_keys::TRASHED,
    note_keys::MERGED,
    note_keys::SPAWNED,
    note_keys::NUDGE,
];
const SECTION_FIELDS: [&str; 4] = [
    note_keys::TYPE,
    note_keys::NOTE,
    note_keys::CAPTURED,
    note_keys::TEXT,
];
const ROUTINE_FIELDS: [&str; 9] = [
    routine_keys::TYPE,
    routine_keys::NOTE,
    routine_keys::START,
    routine_keys::TIMES,
    routine_keys::RULE,
    routine_keys::RENAG,
    routine_keys::ESCALATION,
    routine_keys::TRASHED,
    routine_keys::TRASHED_WITH,
];
const AREA_FIELDS: [&str; 4] = [
    place_keys::TYPE,
    place_keys::NAME,
    place_keys::COLOUR,
    place_keys::ICON,
];
const TOPIC_FIELDS: [&str; 3] = [place_keys::TYPE, place_keys::AREA, place_keys::NAME];

/// The fields written only when an entity is made: its type, when it was
/// captured, and the Note it belongs to.
const MADE_ONCE: [&str; 3] = [note_keys::TYPE, note_keys::CAPTURED, note_keys::NOTE];

/// Which thread drafts into which set, and which read the log alone.
#[derive(Default)]
pub(crate) struct Scopes {
    drafting: HashMap<ThreadId, String>,
    log_only: HashMap<ThreadId, usize>,
}

/// Ends a scope of [`Scopes`] for the thread, also when its body panics.
struct ScopeEnd<'a> {
    store: &'a Store,
    drafting: bool,
}

impl Drop for ScopeEnd<'_> {
    fn drop(&mut self) {
        let thread = std::thread::current().id();
        let mut scopes = self.store.scopes.lock();
        if self.drafting {
            scopes.drafting.remove(&thread);
        } else if let Some(depth) = scopes.log_only.get_mut(&thread) {
            *depth -= 1;
            if *depth == 0 {
                scopes.log_only.remove(&thread);
            }
        }
    }
}

/// What a group of the review holds (#112): the Notes added, filed,
/// merged, moved to the trash or back, whose text or Importance changed,
/// the Routines and Reminders changed, the Areas and Topics changed, and
/// anything else. In the order the review shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AiChangeKind {
    NoteAdded,
    NoteFiled,
    NoteMerged,
    NoteTrashed,
    NoteRestored,
    NoteText,
    NoteImportance,
    Routine,
    Place,
    Other,
}

/// One field an AI Task changes: what the log holds now (`before`) and
/// what the task wrote (`after`). `mine_wins` when one of the user's own
/// Devices wrote the field after the task did: Accept keeps the user's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiFieldChange {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub mine_wins: bool,
}

/// One entity an AI Task changes, within one group: a Note, Section,
/// Routine, Area or Topic, with the Note to open for it, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiReviewItem {
    pub kind: AiChangeKind,
    pub entity: String,
    pub note: Option<String>,
    pub changes: Vec<AiFieldChange>,
}

/// The review of an AI Task: its changes grouped by kind, each group in
/// the order the task first changed its entities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiReview {
    pub groups: Vec<(AiChangeKind, Vec<AiReviewItem>)>,
}

impl AiReview {
    /// Whether the task changed nothing.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

/// The group the change of `field` to `after` on an entity of type `ty`
/// falls in; `made` for an entity the task made.
fn kind_of(ty: &str, made: bool, field: &str, after: Option<&str>) -> AiChangeKind {
    match ty {
        "note" if made => AiChangeKind::NoteAdded,
        "note" => match field {
            note_keys::PLACE | note_keys::SPAWNED | note_keys::NUDGE => AiChangeKind::NoteFiled,
            note_keys::MERGED if after.is_some() => AiChangeKind::NoteMerged,
            note_keys::TRASHED if after.is_some() => AiChangeKind::NoteTrashed,
            note_keys::TRASHED => AiChangeKind::NoteRestored,
            note_keys::TEXT => AiChangeKind::NoteText,
            note_keys::IMPORTANCE => AiChangeKind::NoteImportance,
            _ => AiChangeKind::Other,
        },
        "section" => AiChangeKind::NoteText,
        "routine" => AiChangeKind::Routine,
        "area" | "topic" => AiChangeKind::Place,
        _ => AiChangeKind::Other,
    }
}

impl Store {
    /// The AI Task waiting for its review, or open: the id of its set;
    /// `None` when there is none.
    pub fn ai_task(&self) -> Result<Option<String>> {
        Ok(self
            .pending_sets()?
            .into_iter()
            .find(|s| s.kind == AI_TASK_KIND)
            .map(|s| s.id))
    }

    /// Opens an AI Task, shown at once, and returns its id: the one that
    /// waits, when one does, as there is one at a time.
    pub fn open_ai_task(&self) -> Result<String> {
        match self.ai_task()? {
            Some(id) => Ok(id),
            None => self.open_pending(AI_TASK_KIND, true, None),
        }
    }

    /// Runs `body` with every write this thread makes drafted into the AI
    /// Task `id` ([`Store::append`]): the reads see it as stored, the log
    /// sees nothing. Photos, recordings, local settings and entries from
    /// elsewhere cannot be written meanwhile. When `body` fails, the set
    /// is left as it was before.
    pub fn drafting<T>(&self, id: &str, body: impl FnOnce() -> Result<T>) -> Result<T> {
        let before = self
            .pending_set(id)?
            .filter(|s| s.kind == AI_TASK_KIND)
            .ok_or_else(|| Error::Invalid(format!("no AI Task {id}")))?;
        let thread = std::thread::current().id();
        {
            let mut scopes = self.scopes.lock();
            if scopes.drafting.contains_key(&thread) {
                return Err(Error::Invalid("already drafting".into()));
            }
            scopes.drafting.insert(thread, id.to_string());
        }
        let out = {
            let _end = ScopeEnd {
                store: self,
                drafting: true,
            };
            body()
        };
        if out.is_err() {
            self.keep_pending_set(before)?;
        }
        out
    }

    /// The set this thread drafts into, inside [`Store::drafting`].
    pub(crate) fn drafting_into(&self) -> Option<String> {
        let thread = std::thread::current().id();
        self.scopes.lock().drafting.get(&thread).cloned()
    }

    /// An error inside [`Store::drafting`]: what is no entry cannot wait
    /// for Accept, nor be left behind by a Reject.
    pub(crate) fn refuse_while_drafting(&self) -> Result<()> {
        match self.drafting_into() {
            Some(_) => Err(Error::Invalid(
                "an AI Task writes only Pending Changes".into(),
            )),
            None => Ok(()),
        }
    }

    /// Runs `body` with the reads seeing the log alone, without the shown
    /// pending sets: what is kept, not what waits.
    pub fn log_only<T>(&self, body: impl FnOnce() -> T) -> T {
        let thread = std::thread::current().id();
        *self.scopes.lock().log_only.entry(thread).or_insert(0) += 1;
        let _end = ScopeEnd {
            store: self,
            drafting: false,
        };
        body()
    }

    /// Whether this thread reads the log alone ([`Store::log_only`]).
    pub(crate) fn reads_log_only(&self) -> bool {
        let thread = std::thread::current().id();
        self.scopes.lock().log_only.contains_key(&thread)
    }

    /// Drafts the change of `field` on `entity` to `value` into the set
    /// `id`, once an AI Task may write it ([`Store::check_ai_draft`]), and
    /// returns it as the reads show it.
    pub(crate) fn draft_entry(
        &self,
        id: &str,
        entity: &str,
        field: &str,
        value: Option<&str>,
        date: Option<DateTime<Utc>>,
    ) -> Result<Entry> {
        self.check_ai_draft(entity, field, value)?;
        self.draft_pending(id, entity, field, value, date)?;
        let set = self
            .pending_set(id)?
            .ok_or_else(|| Error::Invalid(format!("no pending set {id}")))?;
        let drafts: Vec<&PendingDraft> = set.drafts();
        let draft = drafts
            .last()
            .ok_or_else(|| Error::Invalid("the draft is gone".into()))?;
        Ok(draft.as_entry(&self.device_id()?, &self.author()?, -(drafts.len() as i64)))
    }

    /// Whether an AI Task may write `field` on `entity` as `value`: a
    /// field of [`NOTE_FIELDS`] and the others, an entity's type and
    /// links only when it is made, and only what this Device's role in
    /// the Shared Area allows, as the log has it, as the screens ask
    /// before they write.
    fn check_ai_draft(&self, entity: &str, field: &str, value: Option<&str>) -> Result<()> {
        let refuse = |why: &str| {
            Err(Error::Invalid(format!(
                "not for an AI Task: {field} of {entity}, {why}"
            )))
        };
        let ty = entity.split_once(':').map_or("", |(ty, _)| ty);
        let fields: &[&str] = match ty {
            "note" => &NOTE_FIELDS,
            "section" => &SECTION_FIELDS,
            "routine" => &ROUTINE_FIELDS,
            "area" => &AREA_FIELDS,
            "topic" => &TOPIC_FIELDS,
            _ => &[],
        };
        if !fields.contains(&field) {
            return refuse("a field it never writes");
        }
        let made = self.logged_history(entity)?.is_empty();
        // Deleting an Area or Topic writes its type to null.
        let deletes = field == note_keys::TYPE && value.is_none() && matches!(ty, "area" | "topic");
        if MADE_ONCE.contains(&field) && !made && !deletes {
            return refuse("written when it was made");
        }
        if field == note_keys::TYPE && value != Some(ty) && !deletes {
            return refuse("another type");
        }
        let allowed = match ty {
            "note" => {
                (made || self.may_change(entity)?)
                    && match (field, value) {
                        (note_keys::PLACE, Some(place)) => self.may_write_in(place)?,
                        (note_keys::MERGED, Some(into)) => self.may_write_in(into)?,
                        _ => true,
                    }
            }
            "section" => {
                (made || self.may_change(entity)?)
                    && match (field, value) {
                        (note_keys::NOTE, Some(note)) => self.may_write_in(note)?,
                        _ => true,
                    }
            }
            "routine" => {
                (made || self.may_change_routine(entity)?)
                    && match (field, value) {
                        (routine_keys::NOTE, Some(note)) => {
                            self.may_manage(self.area_of(note)?.as_deref())?
                        }
                        _ => true,
                    }
            }
            "area" => made || self.may_manage(Some(entity))?,
            "topic" => {
                (made || self.may_manage(self.area_of(entity)?.as_deref())?)
                    && match (field, value) {
                        (place_keys::AREA, Some(area)) => self.may_manage(Some(area))?,
                        _ => true,
                    }
            }
            _ => false,
        };
        if !allowed {
            return refuse("not allowed by this Device's role in its Shared Area");
        }
        Ok(())
    }

    /// Whether sync waits: while a shown pending set does, such as an AI
    /// Task, open or waiting for its review (ADR 0022, 0026), nothing is
    /// synced, sent or taken in from a bundle.
    pub fn sync_paused(&self) -> Result<bool> {
        Ok(self.pending_sets()?.iter().any(|s| s.shown))
    }

    /// [`Error::SyncPaused`] while [`Store::sync_paused`].
    pub(crate) fn refuse_while_paused(&self) -> Result<()> {
        if self.sync_paused()? {
            return Err(Error::SyncPaused);
        }
        Ok(())
    }

    /// The review of the AI Task `id`: every field it changes, by entity
    /// and grouped by kind ([`AiChangeKind`]), with what the log holds now
    /// and what the task wrote last. A field written back to what the log
    /// holds is no change.
    pub fn ai_review(&self, id: &str) -> Result<AiReview> {
        let set = self
            .pending_set(id)?
            .ok_or_else(|| Error::Invalid(format!("no pending set {id}")))?;
        let mut entities: Vec<String> = Vec::new();
        let mut fields: HashMap<String, Vec<String>> = HashMap::new();
        let mut last: HashMap<(String, String), &PendingDraft> = HashMap::new();
        for d in set.drafts() {
            let held = fields.entry(d.entity.clone()).or_insert_with(|| {
                entities.push(d.entity.clone());
                Vec::new()
            });
            if !held.contains(&d.field) {
                held.push(d.field.clone());
            }
            last.insert((d.entity.clone(), d.field.clone()), d);
        }
        let mut groups: BTreeMap<AiChangeKind, Vec<AiReviewItem>> = BTreeMap::new();
        let mut by = GiveWay::default();
        for entity in &entities {
            let ty = entity.split_once(':').map_or("", |(ty, _)| ty);
            let made = self.logged_history(entity)?.is_empty();
            let note = match ty {
                "note" => Some(entity.clone()),
                "section" | "routine" => self.current(entity, note_keys::NOTE)?,
                _ => None,
            };
            let mut items: Vec<AiReviewItem> = Vec::new();
            for field in fields.get(entity).into_iter().flatten() {
                let Some(d) = last.get(&(entity.clone(), field.clone())) else {
                    continue;
                };
                let before = self.logged(entity, field)?;
                if before == d.value {
                    continue;
                }
                let change = AiFieldChange {
                    field: field.clone(),
                    before,
                    mine_wins: self.gives_way(entity, field, d.mark, &mut by)?,
                    after: d.value.clone(),
                };
                let kind = kind_of(ty, made, field, d.value.as_deref());
                match items.iter_mut().find(|i| i.kind == kind) {
                    Some(item) => item.changes.push(change),
                    None => items.push(AiReviewItem {
                        kind,
                        entity: entity.clone(),
                        note: note.clone(),
                        changes: vec![change],
                    }),
                }
            }
            for item in items {
                groups.entry(item.kind).or_default().push(item);
            }
        }
        Ok(AiReview {
            groups: groups.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alarms::Alarms;
    use crate::lan::{
        Joining, answer_sync_inviting, apply_answer_as, read_request, sync_request_as,
    };
    use crate::notes::Notes;
    use crate::places::Places;
    use crate::routines::Routines;
    use crate::schedule::{ClockTime, Day, Schedule};
    use crate::shared::{AreaAccess, AreaRole, Invite};
    use crate::signing::SigningKey;
    use crate::store::tests::{store, ticking};
    use chrono::NaiveDateTime;
    use std::net::{IpAddr, Ipv4Addr};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn utc_wall(t: DateTime<Utc>) -> NaiveDateTime {
        t.naive_utc()
    }

    fn at_ten(day: i64) -> Made<Schedule> {
        let ten: Vec<ClockTime> = ClockTime::new(10, 0).into_iter().collect();
        Ok(Schedule::new(Day::new(2026, 1, day), &ten, None).ok_or("a schedule")?)
    }

    #[test]
    fn writes_inside_drafting_wait_in_the_task_while_the_users_own_reach_the_log() -> TestResult {
        let store = store();
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        let logged = store.all_entries()?.len();
        let task = store.open_ai_task()?;

        let made = store.drafting(&task, || {
            Notes::new(&store).edit(&milk.id, "Oat milk")?;
            // The user edits on another thread meanwhile.
            std::thread::scope(|s| s.spawn(|| Notes::new(&store).amend(&milk.id, "2 l")).join())
                .map_err(|_| Error::Invalid("the user's thread".into()))??;
            Ok(Notes::new(&store).capture("Bread", None, None)?.id)
        })?;

        assert_eq!(store.all_entries()?.len(), logged + 4, "only the Amend");
        let notes = Notes::new(&store);
        assert_eq!(
            notes.by_id(&milk.id)?.map(|n| n.text),
            Some("Oat milk".into())
        );
        assert_eq!(notes.by_id(&made)?.map(|n| n.text), Some("Bread".into()));
        assert!(store.logged_history(&made)?.is_empty());
        let set = store.pending_set(&task)?.ok_or("the task")?;
        assert_eq!(set.drafts().len(), 1 + 3, "the edit and the capture");
        // After the call, the same thread writes to the log again.
        notes.set_importance(&milk.id, crate::Importance::Important)?;
        assert_eq!(store.all_entries()?.len(), logged + 5);
        Ok(())
    }

    #[test]
    fn a_call_that_fails_leaves_the_task_as_it_was() -> TestResult {
        let store = store();
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        let task = store.open_ai_task()?;
        let failed: Result<()> = store.drafting(&task, || {
            Notes::new(&store).edit(&milk.id, "Oat milk")?;
            Notes::new(&store).file(&milk.id, "area:nowhere")
        });
        assert!(failed.is_err());
        assert!(
            store
                .pending_set(&task)?
                .ok_or("the task")?
                .drafts()
                .is_empty()
        );
        assert_eq!(
            Notes::new(&store).by_id(&milk.id)?.map(|n| n.text),
            Some("Milk".into())
        );
        Ok(())
    }

    #[test]
    fn an_ai_task_shares_blocks_sends_and_sets_nothing() -> TestResult {
        let store = store();
        let house = Places::new(&store).add_area("Household")?;
        let milk = Notes::new(&store).capture("Milk", Some(&house.id), None)?;
        let task = store.open_ai_task()?;
        let tries: Vec<Result<()>> = vec![
            store.drafting(&task, || store.share(&house.id).map(drop)),
            store.drafting(&task, || {
                store.block_person("profile:carol", true).map(drop)
            }),
            store.drafting(&task, || {
                store.append(&milk.id, "sent:x", Some("{}"), None).map(drop)
            }),
            store.drafting(&task, || {
                store
                    .append(
                        &milk.id,
                        &crate::notes::keys::photo("ab"),
                        Some("added"),
                        None,
                    )
                    .map(drop)
            }),
            store.drafting(&task, || {
                store
                    .append(&milk.id, "$type", Some("routine"), None)
                    .map(drop)
            }),
            store.drafting(&task, || store.set_local_setting("renagMinutes", "1")),
            store.drafting(&task, || {
                store.put_photo(&[0xFF, 0xD8, 0xFF, 0xD9]).map(drop)
            }),
        ];

        for (n, tried) in tries.iter().enumerate() {
            assert!(tried.is_err(), "try {n} went through");
        }
        assert!(
            store
                .pending_set(&task)?
                .ok_or("the task")?
                .drafts()
                .is_empty()
        );
        assert!(!store.is_shared(&house.id)?);
        assert!(store.local_setting("renagMinutes")?.is_none());
        Ok(())
    }

    /// Ann shares her Household with Ben as a Member, with her Note and
    /// Reminder in it.
    fn member() -> Made<(Store, Store, String, String, String)> {
        let device = |seed: u8, name: &str| -> Made<Store> {
            let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
            crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
            Ok(store)
        };
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let milk = Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
        let reminder = Routines::new(&ann).spawn(&milk.id, &at_ten(2)?)?;
        let area = ann.share(&house.id)?;
        let invite = Invite {
            area: area.clone(),
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            &ben,
            &ann.version_vector()?,
            "P",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        let (answer, _) =
            answer_sync_inviting(&ann, &request, "D", Some(&invite))?.ok_or("an answer")?;
        apply_answer_as(&ben, &answer, true, None)?;
        assert_eq!(ben.access_in(&area, None)?, AreaAccess::Member);
        Ok((ann, ben, area, milk.id, reminder.id))
    }

    #[test]
    fn an_ai_task_does_only_what_the_role_in_a_shared_area_allows() -> TestResult {
        let (_ann, ben, area, milk, reminder) = member()?;
        let task = ben.open_ai_task()?;
        let notes = Notes::new(&ben);
        let later = at_ten(3)?;

        // A Member's own Note in the Area, and an Amend of Ann's: allowed.
        let own = ben.drafting(&task, || notes.capture("Bread", Some(&area), None))?;
        ben.drafting(&task, || notes.amend(&milk, "Two litres").map(drop))?;
        ben.drafting(&task, || notes.edit(&own.id, "Rye bread"))?;
        // Ann's Note and Reminder: not a Member's to change.
        assert!(
            ben.drafting(&task, || notes.edit(&milk, "Buy beer"))
                .is_err()
        );
        assert!(
            ben.drafting(&task, || Routines::new(&ben).reschedule(&reminder, &later))
                .is_err()
        );
        assert!(
            ben.drafting(&task, || Places::new(&ben).rename(&area, "Ours"))
                .is_err()
        );

        let drafted: Vec<String> = ben
            .pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .iter()
            .map(|d| d.entity.clone())
            .collect();
        assert!(!drafted.contains(&milk) && !drafted.contains(&reminder));
        assert!(!drafted.contains(&area));
        Ok(())
    }

    #[test]
    fn the_review_groups_each_change_with_before_and_after() -> TestResult {
        let store = store();
        let notes = Notes::new(&store);
        let places = Places::new(&store);
        let house = places.add_area("Household")?;
        let milk = notes.capture("Milk", None, None)?;
        let old = notes.capture("Old list", None, None)?;
        let pills = notes.capture("Pills", None, None)?;
        let routine = Routines::new(&store).spawn(&pills.id, &at_ten(2)?)?;
        let task = store.open_ai_task()?;
        let bread = store.drafting(&task, || {
            notes.edit(&milk.id, "Oat milk")?;
            notes.file(&milk.id, &house.id)?;
            notes.set_importance(&pills.id, crate::Importance::Important)?;
            Routines::new(&store).set_renag(&routine.id, Some(chrono::Duration::minutes(5)))?;
            Routines::new(&store).trash_note(&old.id, true)?;
            places.add_topic(&house.id, "Kitchen")?;
            // Written back as it was: no change.
            notes.edit(&pills.id, "Pills")?;
            Ok(notes.capture("Bread", None, None)?.id)
        })?;
        // The user fixes the text meanwhile: hers stays.
        notes.edit(&milk.id, "Milk, 2 l")?;

        let review = store.ai_review(&task)?;

        let kinds: Vec<AiChangeKind> = review.groups.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            [
                AiChangeKind::NoteAdded,
                AiChangeKind::NoteFiled,
                AiChangeKind::NoteTrashed,
                AiChangeKind::NoteText,
                AiChangeKind::NoteImportance,
                AiChangeKind::Routine,
                AiChangeKind::Place,
            ]
        );
        let item = |kind: AiChangeKind| {
            review
                .groups
                .iter()
                .find(|(k, _)| *k == kind)
                .and_then(|(_, items)| items.first())
                .cloned()
                .ok_or("an item")
        };
        let text = item(AiChangeKind::NoteText)?;
        assert_eq!(text.entity, milk.id);
        assert_eq!(text.note.as_deref(), Some(milk.id.as_str()));
        let change = &text.changes[0];
        assert_eq!(change.field, "text");
        assert!(change.mine_wins);
        let stored = |t: &str| crate::StoredText::plain(t).encode();
        assert_eq!(change.before, Some(stored("Milk, 2 l")));
        assert_eq!(change.after, Some(stored("Oat milk")));
        let filed = item(AiChangeKind::NoteFiled)?;
        assert_eq!(filed.changes[0].before, None);
        assert_eq!(filed.changes[0].after.as_deref(), Some(house.id.as_str()));
        assert!(!filed.changes[0].mine_wins);
        assert_eq!(item(AiChangeKind::NoteAdded)?.entity, bread);
        assert_eq!(item(AiChangeKind::NoteTrashed)?.entity, old.id);
        let renag = item(AiChangeKind::Routine)?;
        assert_eq!(renag.note.as_deref(), Some(pills.id.as_str()));
        assert_eq!(renag.changes[0].after.as_deref(), Some("5"));
        assert!(
            store
                .ai_review(&store.open_pending("bundle", true, None)?)?
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn one_ai_task_at_a_time_and_none_drafts_into_another_set() -> TestResult {
        let store = store();
        let task = store.open_ai_task()?;
        assert_eq!(store.open_ai_task()?, task);
        assert_eq!(store.ai_task()?, Some(task.clone()));
        let other = store.open_pending("bundle", true, None)?;
        assert!(store.drafting(&other, || Ok(())).is_err());
        store.accept_pending(&task)?;
        assert_ne!(store.open_ai_task()?, task);
        Ok(())
    }

    #[test]
    fn sync_waits_while_an_ai_task_does() -> TestResult {
        let store = store();
        let dir = tempfile::tempdir()?;
        let bundle = dir.path().join("out.zip");
        crate::profile::OwnProfile::new(&store).apply_defaults("Ann", None)?;
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        assert!(!store.sync_paused()?);
        let task = store.open_ai_task()?;
        assert!(store.sync_paused()?);

        let vector = read_request(
            &mut std::io::Cursor::new(&b"GET /vector HTTP/1.1\r\n\r\n"[..]),
            IpAddr::V4(Ipv4Addr::LOCALHOST),
        )?
        .ok_or("a request")?;
        let (answered, _) = store.serve(&vector, "Desk")?;
        assert_eq!(answered.status, 503);
        let paused = |r: Result<()>| matches!(r, Err(Error::SyncPaused));
        assert!(paused(
            crate::bundle::write_bundle_now(&store, &bundle, None).map(drop)
        ));
        assert!(paused(
            crate::bundle::import_bundle(&store, &bundle).map(drop)
        ));
        let mut folder = crate::folder::LocalFolder::new(dir.path().join("folder"));
        assert!(paused(
            crate::folder::folder_sync(&store, &mut folder, &mut crate::folder::random_nonce)
                .map(drop)
        ));
        assert!(paused(store.send_note(&milk.id, None, false).map(drop)));
        assert!(!dir.path().join("folder").exists());

        store.decline_pending(&task)?;
        assert!(!store.sync_paused()?);
        assert!(crate::bundle::write_bundle_now(&store, &bundle, None)?.is_some());
        assert_ne!(store.serve(&vector, "Desk")?.0.status, 503);
        Ok(())
    }

    #[test]
    fn alarms_fire_as_the_log_has_them_until_the_task_is_accepted() -> TestResult {
        let store = store();
        let notes = Notes::new(&store);
        let pills = notes.capture("Pills", None, None)?;
        let walk = notes.capture("Walk", None, None)?;
        let routine = Routines::new(&store).spawn(&pills.id, &at_ten(2)?)?;
        let (later, same) = (at_ten(3)?, at_ten(2)?);
        let task = store.open_ai_task()?;
        store.drafting(&task, || {
            Routines::new(&store).reschedule(&routine.id, &later)?;
            Routines::new(&store).spawn(&walk.id, &same).map(drop)
        })?;
        let planned = |store: &Store| -> Made<Vec<(String, String)>> {
            Ok(Alarms::new(store)
                .plan(&utc_wall)?
                .into_iter()
                .map(|a| (a.routine_id, a.occurrence.key()))
                .collect())
        };

        assert_eq!(
            Routines::new(&store).all()?.len(),
            2,
            "the Agenda shows both"
        );
        let before = planned(&store)?;
        assert_eq!(before, [(routine.id.clone(), occurrence_key(2))]);

        store.accept_pending(&task)?;
        let after = planned(&store)?;
        assert_eq!(after.len(), 2);
        assert!(after.contains(&(routine.id.clone(), occurrence_key(3))));
        Ok(())
    }

    fn occurrence_key(day: i64) -> String {
        crate::schedule::Occurrence::new(Day::new(2026, 1, day), ClockTime::new(10, 0)).key()
    }
}
