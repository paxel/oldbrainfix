//! Blocking a person, as the Dart core's `blocks.dart` does it (ADR
//! 0021, #90): once the user blocks someone, nothing that person wrote
//! reaches the user's screen or storage, whichever Device relays it, but
//! their check-offs. The block is the entity `block:<profile id>` in the
//! log, outside every Area like an Override, so it is sealed under the
//! personal key and goes to the user's own Devices only, never to the
//! person, a Shared Area or a Sent Item; only what one's own Devices
//! wrote counts. The person's Devices are those the grants of the Shared
//! Areas name them for, never one of the user's own.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::boxes;
use crate::checklists::keys as checklist_keys;
use crate::entry::{Entry, WireEntry};
use crate::frogs::keys as frog_keys;
use crate::keys::PERSONAL_SCOPE;
use crate::notes::keys as note_keys;
use crate::profile::{Profile, sanitize_name};
use crate::routines::keys as routine_keys;
use crate::schedule::Occurrence;
use crate::shared::{AreaRole, Grant, JudgedLog, ROLE_PREFIX};
use crate::store::Store;
use crate::sync::{BLOB_KINDS, split_blob_ref};
use crate::text_boxes::{self, BoxState};
use crate::{Error, Result};

/// The entity id's prefix, before the Profile id.
pub const PREFIX: &str = "block:";

/// The name the person showed under when they were blocked, so every one
/// of the user's Devices lists them by it; none once unblocked.
pub const BLOCKED: &str = "blocked";

/// The most code points the author name of an entry kept from a blocked
/// person may have; the apps write none.
pub const AUTHOR_MAX: usize = 40;

/// An id as the apps make them: letters, digits and dashes, at most 64.
fn is_id(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase() || b == b'-')
}

/// An instant as both cores write it: `2026-03-01T08:00:00.000Z`, the
/// fraction of a second from one to six digits or none.
pub(crate) fn is_instant(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        b.get(range)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
    };
    let at = |i: usize, want: u8| b.get(i) == Some(&want);
    if b.len() < 20
        || !digits(0..4)
        || !at(4, b'-')
        || !digits(5..7)
        || !at(7, b'-')
        || !digits(8..10)
        || !at(10, b'T')
        || !digits(11..13)
        || !at(13, b':')
        || !digits(14..16)
        || !at(16, b':')
        || !digits(17..19)
        || b.last() != Some(&b'Z')
    {
        return false;
    }
    match b.len() - 20 {
        0 => true,
        n => (2..=7).contains(&n) && at(19, b'.') && digits(20..b.len() - 1),
    }
}

/// Whether `e`, written by a blocked person's Device, is kept (ADR 0021),
/// as the Dart core's `keptFromBlocked`: only what carries no text,
/// picture or sound and keeps the Shared Areas whole. That is a
/// check-off or its undoing (`done@…` on a Routine, a skip `skip@…` on
/// it, `done` on a Frog,
/// `checked` on a Checklist's Line, `check@…` on a Routine for one of its
/// Lines, a tick `box@…` of a box in the text of a Note or a Section), a
/// grant (`role:…` on an Area), and a Note's `$type` and filing, which
/// say where the Routines, Frogs and Sections others hung on it lie; the
/// Note itself shows
/// nowhere without its capture. Each in its exact shape, so nothing else
/// can hide in it.
pub fn kept_from_blocked(e: &Entry) -> bool {
    kept(
        &e.entity,
        &e.field,
        e.value.as_deref(),
        &e.author,
        e.ctx.is_none() && e.extra.is_empty(),
    )
}

/// [`kept_from_blocked`] for an entry as it came.
pub(crate) fn wire_kept_from_blocked(e: &WireEntry) -> bool {
    kept(
        &e.entity,
        &e.field,
        e.value.as_deref(),
        &e.author,
        e.ctx.is_none() && e.extra.is_empty(),
    )
}

/// Whether an entry of `entity`, `field` and `value` by `author` is
/// kept; `plain` when it carries no causal context and no key of a later
/// version.
fn kept(entity: &str, field: &str, value: Option<&str>, author: &str, plain: bool) -> bool {
    if !plain || author.chars().count() > AUTHOR_MAX {
        return false;
    }
    let Some((kind, id)) = entity.split_once(':') else {
        return false;
    };
    if !is_id(id) {
        return false;
    }
    if kind == "routine"
        && let Some(key) = field.strip_prefix(routine_keys::DONE_PREFIX)
    {
        return Occurrence::parse(key).is_some_and(|o| o.key() == key)
            && value.is_none_or(is_instant);
    }
    if kind == "routine"
        && let Some(key) = field.strip_prefix(routine_keys::SKIP_PREFIX)
    {
        let at = value.map(|v| {
            v.strip_suffix(routine_keys::TOMORROW_MARK)
                .and_then(|v| v.strip_suffix(' '))
                .unwrap_or(v)
        });
        return Occurrence::parse(key).is_some_and(|o| o.key() == key) && at.is_none_or(is_instant);
    }
    if kind == "routine"
        && let Some(rest) = field.strip_prefix(checklist_keys::CHECK_PREFIX)
    {
        let Some((key, line)) = rest.split_once('@') else {
            return false;
        };
        return !key.is_empty()
            && Occurrence::parse(key).is_some_and(|o| o.key() == key)
            && line
                .strip_prefix(checklist_keys::LINE_PREFIX)
                .is_some_and(is_id)
            && value.is_none_or(is_instant);
    }
    if kind == frog_keys::TYPE_FROG && field == frog_keys::DONE
        || kind == checklist_keys::TYPE_LINE && field == checklist_keys::CHECKED
    {
        return value.is_none_or(is_instant);
    }
    if (kind == "note" || kind == "section")
        && let Some(id) = field.strip_prefix(boxes::PREFIX)
    {
        return text_boxes::is_id(id) && BoxState::named(value).is_some();
    }
    if kind == "area"
        && let Some(device) = field.strip_prefix(ROLE_PREFIX)
    {
        if !is_id(device) {
            return false;
        }
        let Some(value) = value else {
            return true;
        };
        return Grant::parse(value).is_some_and(|grant| {
            grant.encode().as_deref() == Some(value) && is_id(&grant.profile)
        });
    }
    if kind == "note" && field == note_keys::TYPE {
        return value.is_none_or(|v| v == note_keys::TYPE_NOTE);
    }
    if kind == "note" && field == note_keys::PLACE {
        return value.is_none_or(|v| {
            v.split_once(':')
                .is_some_and(|(k, id)| (k == "area" || k == "topic") && is_id(id))
        });
    }
    false
}

impl Store {
    /// Every block that holds now, as (Profile id as written, name then):
    /// those one of the user's own Devices wrote last.
    fn blocks(&self) -> Result<Vec<(String, String)>> {
        let mut own: HashMap<String, bool> = HashMap::new();
        let mut out = Vec::new();
        for entity in self.entities_with_prefix(PREFIX)? {
            if let Some(name) = self.own_current(&entity, BLOCKED, &mut own)? {
                out.push((entity[PREFIX.len()..].to_string(), name));
            }
        }
        Ok(out)
    }

    /// The Profile ids blocked now, through their aliases.
    pub fn blocked_ids(&self) -> Result<HashSet<String>> {
        let mut ids = HashSet::new();
        for (id, _) in self.blocks()? {
            ids.insert(self.resolve_person(&id)?);
        }
        Ok(ids)
    }

    /// Whether the person `id` is blocked; the own Profile never is.
    pub fn is_blocked(&self, id: &str) -> Result<bool> {
        Ok(!self.is_own_person(id)? && self.blocked_ids()?.contains(&self.resolve_person(id)?))
    }

    /// The name the person `id` showed under when they were blocked, for
    /// a Device that does not keep their Profile; none when not blocked.
    pub(crate) fn name_when_blocked(&self, id: &str) -> Result<Option<String>> {
        for (blocked, name) in self.blocks()? {
            if self.resolve_person(&blocked)? == id {
                return Ok(Some(sanitize_name(&name)));
            }
        }
        Ok(None)
    }

    /// The people blocked now, for the folded "Blocked" list in People:
    /// as the user sees them, without their own picture (ADR 0021), by
    /// name.
    pub fn blocked_people(&self) -> Result<Vec<Profile>> {
        let mut shown = Vec::new();
        for id in self.blocked_ids()? {
            shown.push(self.person_shown(&id)?);
        }
        shown.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        Ok(shown)
    }

    /// The Shared Areas the user has to leave to block the person `id`:
    /// those this Device holds a role in where a Device of theirs is a
    /// Manager now, sorted. Their Routines, Reminders and names there
    /// would otherwise go half missing.
    pub fn areas_to_leave_for_blocking(&self, id: &str) -> Result<Vec<String>> {
        let person = self.resolve_person(id)?;
        let own_device = self.device_id()?;
        let judged = self.judged()?;
        let mut out = Vec::new();
        for (area, grants) in &judged.areas {
            if grants.grant_of(&own_device).is_none() {
                continue;
            }
            let mut manages = false;
            for device in grants.devices() {
                let Some(grant) = grants.grant_of(&device) else {
                    continue;
                };
                if grant.role == Some(AreaRole::Manager)
                    && device != own_device
                    && !self.is_own_person(&grant.profile)?
                    && self.resolve_person(&grant.profile)? == person
                    && !self.written_by_own(&device)?
                {
                    manages = true;
                    break;
                }
            }
            if manages {
                out.push(area.clone());
            }
        }
        out.sort();
        Ok(out)
    }

    /// Blocks the person `id` on all of the user's Devices (#90), as the
    /// Dart core's `blockPerson`: every entry of theirs held here goes
    /// but their check-offs ([`Store::enforce_blocks`]), and nothing more
    /// of theirs is taken in. When they manage a Shared Area this Device
    /// holds a role in ([`Store::areas_to_leave_for_blocking`]), blocking
    /// leaves it, and only with `leave_areas`: otherwise
    /// [`Error::MustLeaveAreas`], and nothing changes. The own Profile is
    /// [`Error::Invalid`]. Returns the Areas left.
    pub fn block_person(&self, id: &str, leave_areas: bool) -> Result<Vec<String>> {
        if self.is_own_person(id)? {
            return Err(Error::Invalid(format!("{id} is the own Profile")));
        }
        let areas = self.areas_to_leave_for_blocking(id)?;
        if !areas.is_empty() && !leave_areas {
            return Err(Error::MustLeaveAreas(areas));
        }
        // Left first: an Area left without the block is the safer half.
        for area in &areas {
            self.leave_area(area)?;
        }
        let person = self.resolve_person(id)?;
        if !self.is_blocked(&person)? {
            let name = self.person_shown(&person)?.name;
            self.append(&format!("{PREFIX}{person}"), BLOCKED, Some(&name), None)?;
        }
        self.enforce_blocks()?;
        Ok(areas)
    }

    /// Unblocks the person `id`: what was refused of theirs is fetched
    /// again from the Devices that still hold it, and their Profile is
    /// taken in again with the next sync that carries it.
    pub fn unblock_person(&self, id: &str) -> Result<()> {
        let person = self.resolve_person(id)?;
        let mut own: HashMap<String, bool> = HashMap::new();
        let mut entities = Vec::new();
        for entity in self.entities_with_prefix(PREFIX)? {
            if self.resolve_person(&entity[PREFIX.len()..])? == person
                && self.own_current(&entity, BLOCKED, &mut own)?.is_some()
            {
                entities.push(entity);
            }
        }
        if entities.is_empty() {
            return Ok(());
        }
        self.batch(|| {
            for entity in &entities {
                self.append(entity, BLOCKED, None, None)?;
            }
            Ok(())
        })?;
        self.enforce_blocks()?;
        Ok(())
    }

    /// The Devices of the people blocked now: those a grant in a Shared
    /// Area names one of them for (its last grant that named anyone),
    /// never this Device nor one of the user's own. A Profile's binding
    /// does not count, since any key bound to it can bind another.
    pub fn blocked_devices(&self) -> Result<HashSet<String>> {
        self.blocked_devices_by(&*self.judged()?)
    }

    /// [`Store::blocked_devices`] by the grants of `judged`: the log with
    /// a pending set on top (ADR 0022).
    pub fn blocked_devices_by(&self, judged: &JudgedLog) -> Result<HashSet<String>> {
        let blocked = self.blocked_ids()?;
        let mut out = HashSet::new();
        if blocked.is_empty() {
            return Ok(out);
        }
        let own_device = self.device_id()?;
        for grants in judged.areas.values() {
            for device in grants.devices() {
                if device == own_device || out.contains(&device) {
                    continue;
                }
                let Some(person) = grants.person_of(&device) else {
                    continue;
                };
                if self.is_own_person(person)? {
                    continue;
                }
                if blocked.contains(&self.resolve_person(person)?)
                    && !self.written_by_own(&device)?
                {
                    out.insert(device);
                }
            }
        }
        Ok(out)
    }

    /// Makes the log what the blocks say (ADR 0021), as the Dart core's
    /// `enforceBlocks`, after a block, an unblock and every sync that
    /// brings anything: every entry a blocked person's Device wrote that
    /// is not [`kept_from_blocked`] is deleted and its dseq noted as
    /// refused with the scope its entity lay in, and the bytes of their photos and recordings nothing else
    /// refers to go; their Profile's picture is taken out. A Device no
    /// longer blocked has its refusals released, so what was refused is
    /// asked for again, the shared folder read whole again too. The
    /// shared folder itself is not touched, and nothing of theirs waits
    /// in a pending set (ADR 0022), a Sent Item held until "Show" (#91)
    /// or an Area they offer (#108) neither. Returns whether anything was
    /// deleted or let go.
    pub fn enforce_blocks(&self) -> Result<bool> {
        let blocked = self.blocked_ids()?;
        let marks = self.refusal_marks()?;
        if blocked.is_empty() && marks.is_empty() {
            return Ok(false);
        }
        let devices = self.blocked_devices()?;
        let mut refs: BTreeSet<String> = BTreeSet::new();
        let changed = self.batch(|| {
            let mut changed = false;
            let mut sorted: Vec<&String> = devices.iter().collect();
            sorted.sort();
            for device in sorted {
                let refused: Vec<Entry> = self
                    .entries_of(device)?
                    .into_iter()
                    .filter(|e| !kept_from_blocked(e))
                    .collect();
                // Where each lay is read before anything goes.
                let mut scopes: HashMap<String, String> = HashMap::new();
                for e in &refused {
                    if !scopes.contains_key(&e.entity) {
                        scopes.insert(e.entity.clone(), self.scope_of(&e.entity)?);
                    }
                }
                for e in &refused {
                    for (kind, prefix) in BLOB_KINDS {
                        if let Some(hash) = e.field.strip_prefix(prefix) {
                            refs.insert(format!("{kind}/{hash}"));
                        }
                    }
                    let scope = scopes.get(&e.entity).map_or(PERSONAL_SCOPE, String::as_str);
                    self.note_refused(device, e.dseq, scope)?;
                }
                if !refused.is_empty() {
                    let dseqs: Vec<i64> = refused.iter().map(|e| e.dseq).collect();
                    self.delete_entries(device, &dseqs)?;
                    changed = true;
                }
            }
            let mut released = false;
            for (device, mark) in &marks {
                if devices.contains(device) {
                    // Blocked again: the mark refuses again.
                    if mark.released {
                        self.hold_refusals(device)?;
                    }
                } else if self.release_refusals(device)? {
                    released = true;
                }
            }
            if released {
                self.forget_folder_reads()?;
            }
            self.drop_filled_refusals()?;
            let mut ids: Vec<&String> = blocked.iter().collect();
            ids.sort();
            for id in ids {
                changed |= self.strip_person_picture(id)?;
            }
            // Nor does anything of theirs wait in a pending set, a Sent
            // Item held until "Show" or an Area they offer neither.
            changed |= self.drop_blocked_from_pending()?;
            changed |= self.drop_held_from_blocked()?;
            changed |= self.drop_offers_from_blocked()?;
            Ok(changed)
        })?;
        let mut dropped = false;
        for reference in refs {
            if self.knows_blob(&reference)? {
                continue;
            }
            let (kind, hash) = split_blob_ref(&reference);
            dropped |= self.drop_blob(kind, hash)?;
        }
        Ok(changed || dropped)
    }
}

#[cfg(test)]
mod tests;
