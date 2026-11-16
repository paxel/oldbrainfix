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

use crate::checklists::{Checklists, keys as line_keys};
use crate::entry::Entry;
use crate::frogs::keys as frog_keys;
use crate::notes::keys as note_keys;
use crate::pending::{GiveWay, PendingDraft};
use crate::places::keys as place_keys;
use crate::routines::keys as routine_keys;
use crate::sessions::{Sessions, keys as session_keys};
use crate::shopping_lists::{ShoppingLists, keys as shopping_keys};
use crate::store::Store;
use crate::tallies::keys as tally_keys;
use crate::{Error, Result};

/// The kind of pending set an AI Task is.
pub const AI_TASK_KIND: &str = "ai-task";

/// The fields an AI Task may write, by the type of the entity: what the
/// user can do with Notes, Areas, Topics, Merges, the trash, Importance,
/// Routines and Reminders, and the set-up of Tallies, Sessions,
/// Checklists, Shopping Lists and Frogs (#168). Not a role, a block, an
/// Override, a Sent Item's link, a Profile, nor a photo or recording; and
/// none of the user's own data: no check-off, Tally entry, tick of a Line
/// or Box, Session pick nor eaten Frog.
const NOTE_FIELDS: [&str; 15] = [
    note_keys::TYPE,
    note_keys::CAPTURED,
    note_keys::TEXT,
    note_keys::PLACE,
    note_keys::IMPORTANCE,
    note_keys::TRASHED,
    note_keys::MERGED,
    note_keys::SPAWNED,
    note_keys::NUDGE,
    line_keys::KIND,
    session_keys::POOL,
    session_keys::COUNT,
    session_keys::RULE,
    tally_keys::BAR,
    tally_keys::SETS,
];
/// The keys of a Tally's field on its Note, `tally:<field id>:<key>`.
const TALLY_FIELD_KEYS: [&str; 6] = [
    tally_keys::NAME,
    tally_keys::UNIT,
    tally_keys::TARGET,
    tally_keys::RULE,
    tally_keys::HIDDEN,
    tally_keys::KIND,
];
const SECTION_FIELDS: [&str; 4] = [
    note_keys::TYPE,
    note_keys::NOTE,
    note_keys::CAPTURED,
    note_keys::TEXT,
];
/// A Routine: never a Session's setup, which lies on its Note (ADR 0039).
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
/// A Line of a Checklist or Shopping List: never whether it is checked.
const LINE_FIELDS: [&str; 8] = [
    line_keys::TYPE,
    line_keys::NOTE,
    line_keys::MADE,
    line_keys::TEXT,
    line_keys::ORDER,
    line_keys::INDENT,
    line_keys::TRASHED,
    shopping_keys::REAPPEAR,
];
/// A Frog: never when it was eaten.
const FROG_FIELDS: [&str; 3] = [frog_keys::TYPE, frog_keys::NOTE, frog_keys::TRASHED];
const AREA_FIELDS: [&str; 4] = [
    place_keys::TYPE,
    place_keys::NAME,
    place_keys::COLOUR,
    place_keys::ICON,
];
const TOPIC_FIELDS: [&str; 3] = [place_keys::TYPE, place_keys::AREA, place_keys::NAME];
/// The type of the user's own Agenda choices for a Note, the entity
/// `agenda:<note id>`
/// ([`crate::checklists::keys::AGENDA_PREFIX`]).
const AGENDA_TYPE: &str = "agenda";
/// The user's own choices for the Agenda, `agenda:<note id>`: how a
/// Shopping List (#186) or a Checklist without a Routine (#219) shows
/// there.
const AGENDA_FIELDS: [&str; 2] = [shopping_keys::AGENDA, line_keys::AGENDA];

/// The fields of a Note's or Section's Capture Context, which an AI Task
/// only ever removes (ADR 0037): it never says where or when something
/// was captured.
const CONTEXT_FIELDS: [&str; 5] = [
    note_keys::LOCATION,
    note_keys::SOURCE_PACKAGE,
    note_keys::SOURCE_LABEL,
    note_keys::CALENDAR,
    note_keys::BLUETOOTH,
];
/// The fields written only when an entity is made: its type, when it was
/// captured, the Note it belongs to and the making of a Line's list.
const MADE_ONCE: [&str; 4] = [
    note_keys::TYPE,
    note_keys::CAPTURED,
    note_keys::NOTE,
    line_keys::MADE,
];

/// Whether `field` is the key of a Tally's field on a Note.
fn is_tally_field(field: &str) -> bool {
    field
        .strip_prefix(tally_keys::FIELD_PREFIX)
        .and_then(|rest| rest.rsplit_once(':'))
        .is_some_and(|(id, key)| !id.is_empty() && TALLY_FIELD_KEYS.contains(&key))
}

/// Whether `field` is the count of a Session for one weekday, on its
/// Note.
fn is_weekday_count(field: &str) -> bool {
    (1..=7).any(|weekday| field == session_keys::count_on(weekday))
}

/// Which thread drafts into which set, which read the log alone, and
/// which read as the AI ([`Store::for_ai`]).
#[derive(Default)]
pub(crate) struct Scopes {
    drafting: HashMap<ThreadId, String>,
    log_only: HashMap<ThreadId, usize>,
    pub(crate) for_ai: HashMap<ThreadId, crate::hidden::AiView>,
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

/// What a group of the review holds (#112, #168): the Notes added, filed,
/// merged, moved to the trash or back, whose text or Importance changed,
/// the Tallies set up, the Checklists and Shopping Lists with their
/// Lines, the Frogs, the Routines and Reminders changed, the Sessions,
/// the Areas and Topics changed, and anything else. In the order the
/// review shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AiChangeKind {
    NoteAdded,
    NoteFiled,
    NoteMerged,
    NoteTrashed,
    NoteRestored,
    NoteText,
    NoteImportance,
    Tally,
    List,
    Frog,
    Routine,
    Session,
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
/// falls in; `made` for an entity the task made, `session` for a Routine
/// that runs a Session.
fn kind_of(ty: &str, made: bool, session: bool, field: &str, after: Option<&str>) -> AiChangeKind {
    match ty {
        "note" if is_tally_field(field) => AiChangeKind::Tally,
        "note" if session_keys::is_setup(field) => AiChangeKind::Session,
        "note" if field == line_keys::KIND => AiChangeKind::List,
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
        "routine" if session => AiChangeKind::Session,
        "routine" => AiChangeKind::Routine,
        line_keys::TYPE_LINE | AGENDA_TYPE => AiChangeKind::List,
        frog_keys::TYPE_FROG => AiChangeKind::Frog,
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
    /// field of [`NOTE_FIELDS`] and the others, a field of
    /// [`CONTEXT_FIELDS`] only as null, an entity's type and links only
    /// when it is made, and only what this Device's role in
    /// the Shared Area allows, as the log has it, as the screens ask
    /// before they write.
    fn check_ai_draft(&self, entity: &str, field: &str, value: Option<&str>) -> Result<()> {
        // What is Hidden from AI does not exist for it (#267): refused as
        // an unknown id, before anything below reads it.
        if self.draft_reaches_hidden(entity, field, value)? {
            return Err(Error::Invalid(format!("not known: {entity}")));
        }
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
            line_keys::TYPE_LINE => &LINE_FIELDS,
            frog_keys::TYPE_FROG => &FROG_FIELDS,
            AGENDA_TYPE => &AGENDA_FIELDS,
            _ => &[],
        };
        let context = matches!(ty, "note" | "section") && CONTEXT_FIELDS.contains(&field);
        if context && value.is_some() {
            return refuse("a captured detail it only removes");
        }
        let set_up = match ty {
            "note" => is_tally_field(field) || is_weekday_count(field),
            _ => false,
        };
        if !context && !set_up && !fields.contains(&field) {
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
            line_keys::TYPE_LINE => {
                if made {
                    match (field, value) {
                        (line_keys::NOTE, Some(note)) => self.may_write_in(note)?,
                        _ => true,
                    }
                } else {
                    Checklists::new(self).may_edit(entity)?
                }
            }
            frog_keys::TYPE_FROG => {
                (made || self.may_change_routine(entity)?)
                    && match (field, value) {
                        (frog_keys::NOTE, Some(note)) => {
                            self.may_manage(self.area_of(note)?.as_deref())?
                        }
                        _ => true,
                    }
            }
            // The user's own, whatever their role in the list's Area.
            AGENDA_TYPE => {
                let note = entity.split_once(':').map_or("", |(_, note)| note);
                match field {
                    line_keys::AGENDA => Checklists::new(self).may_show_on_agenda(note)?,
                    _ => ShoppingLists::new(self).making(note)?.is_some(),
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
                "section" | "routine" | line_keys::TYPE_LINE | frog_keys::TYPE_FROG => {
                    self.current(entity, note_keys::NOTE)?
                }
                AGENDA_TYPE => entity.split_once(':').map(|(_, note)| note.to_string()),
                _ => None,
            };
            let session = ty == "routine" && Sessions::new(self).is_session(entity)?;
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
                let kind = kind_of(ty, made, session, field, d.value.as_deref());
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
mod tests;
