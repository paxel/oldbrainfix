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
fn is_instant(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        b.get(range)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
    };
    if b.len() < 20
        || !digits(0..4)
        || b[4] != b'-'
        || !digits(5..7)
        || b[7] != b'-'
        || !digits(8..10)
        || b[10] != b'T'
        || !digits(11..13)
        || b[13] != b':'
        || !digits(14..16)
        || b[16] != b':'
        || !digits(17..19)
        || b[b.len() - 1] != b'Z'
    {
        return false;
    }
    match b.len() - 20 {
        0 => true,
        n => (2..=7).contains(&n) && b[19] == b'.' && digits(20..b.len() - 1),
    }
}

/// Whether `e`, written by a blocked person's Device, is kept (ADR 0021),
/// as the Dart core's `keptFromBlocked`: only what carries no text,
/// picture or sound and keeps the Shared Areas whole. That is a
/// check-off or its undoing (`done@…` on a Routine, `done` on a Frog,
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
mod tests {
    use super::*;
    use crate::folder::{MemoryFolder, SyncFolder, folder_sync, random_nonce};
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
        sync_request_as,
    };
    use crate::sent::{OpenError, Received};
    use crate::shared::Invite;
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::ticking;
    use crate::sync::wire;
    use crate::{
        AreaAccess, ClockTime, Day, Notes, OwnProfile, PairedDevice, Places, Recurrence, Routines,
        Schedule, WireEntry,
    };
    use serde_json::Value;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// A Profile picture, and a photo, the Dart core made.
    const JPEG: &[u8] = include_bytes!("../../../fixtures/shared/override.jpg");

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    fn device(seed: u8, name: &str) -> Made<Store> {
        let start = ticking()();
        let clock: crate::Clock = Box::new(move || {
            let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            start + chrono::Duration::seconds(n)
        });
        let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
        let own = OwnProfile::new(&store);
        own.apply_defaults("Device", None)?;
        own.rename(name)?;
        Ok(store)
    }

    fn own_id(store: &Store) -> Made<String> {
        Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
    }

    /// One LAN session, `joiner` asking `host`: what went both ways.
    fn exchange(joiner: &Store, host: &Store, shared: bool) -> Made<(Value, Value)> {
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
        Ok((request, answer))
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

    /// `host` invites `joiner` to `area` as a Member.
    fn invite(host: &Store, joiner: &Store, area: &str) -> TestResult {
        let invite = Invite {
            area: area.to_string(),
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            joiner,
            &host.version_vector()?,
            "Phone",
            &host.device_id()?,
            "",
            Joining::Invited,
        )?;
        let (answer, _) =
            answer_sync_inviting(host, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
        apply_answer_as(joiner, &answer, true, None)?;
        remember(host, joiner, true)?;
        remember(joiner, host, true)?;
        Ok(())
    }

    /// Ann's Household shared with Ben and Carol as Members, Ann's desk
    /// one of her own Devices.
    struct Household {
        ann: Store,
        desk: Store,
        ben: Store,
        carol: Store,
        area: String,
    }

    fn household() -> Made<Household> {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let desk = device(3, "Ann at the desk")?;
        let carol = device(4, "Carol")?;
        exchange(&desk, &ann, false)?;
        remember(&ann, &desk, false)?;
        remember(&desk, &ann, false)?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        invite(&ann, &ben, &area)?;
        invite(&ann, &carol, &area)?;
        exchange(&desk, &ann, false)?;
        Ok(Household {
            ann,
            desk,
            ben,
            carol,
            area,
        })
    }

    fn pills() -> Made<Schedule> {
        Ok(Schedule::new(
            Day::new(2026, 3, 1),
            &[ClockTime::new(8, 0).ok_or("a time")?],
            Some(Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e:?}"))?),
        )
        .ok_or("a schedule")?)
    }

    fn at(day: i64) -> Made<Occurrence> {
        Ok(Occurrence::new(
            Day::new(2026, 3, day),
            Some(ClockTime::new(8, 0).ok_or("a time")?),
        ))
    }

    /// The entries `store` holds that `writer` wrote.
    fn by(store: &Store, writer: &Store) -> Made<Vec<Entry>> {
        Ok(store.entries_of(&writer.device_id()?)?)
    }

    /// Ben takes in what Carol holds, as her Device would hand it over.
    fn relay_to_ben(h: &Household) -> TestResult {
        let entries: Vec<WireEntry> = wire(&h.carol.entries_since(&h.ben.version_vector()?)?);
        h.ben.apply_entries(
            &entries,
            &h.carol.key_records()?,
            None,
            &mut ImportReport::default(),
        )?;
        Ok(())
    }

    fn text_of(store: &Store, note: &str) -> Made<Option<String>> {
        Ok(Notes::new(store).by_id(note)?.map(|n| n.text))
    }

    fn done_keys(store: &Store, routine: &str) -> Made<Vec<String>> {
        Ok(Routines::new(store)
            .by_id(routine)?
            .ok_or("a Routine")?
            .done
            .keys()
            .cloned()
            .collect())
    }

    #[test]
    fn blocking_keeps_her_ticks_of_boxes_on_a_note_and_on_a_section() -> TestResult {
        use crate::boxes::Boxes;
        use crate::text_boxes::BoxState;
        let h = household()?;
        let (ann, carol) = (&h.ann, &h.carol);
        let milk = Notes::new(ann).capture("☐ Milk", Some(&h.area), None)?;
        let eggs = Notes::new(ann).amend(&milk.id, "☐ Eggs")?;
        exchange(carol, ann, true)?;
        Boxes::new(carol).tick(&milk.id, &milk.boxes[0].id, BoxState::Done)?;
        Boxes::new(carol).tick(&eggs.id, &eggs.boxes[0].id, BoxState::Failed)?;
        exchange(carol, ann, true)?;

        assert!(ann.block_person(&own_id(carol)?, false)?.is_empty());

        let note = Notes::new(ann).by_id(&milk.id)?.ok_or("a Note")?;
        assert_eq!(note.shown_text(), "✅ Milk");
        let tick = note.boxes[0].last.clone().ok_or("a tick")?;
        assert_eq!(tick.device, carol.device_id()?);
        // Without her name or picture.
        assert_eq!(ann.ticked_by(&milk.id, &tick)?, None);
        let section = Notes::new(ann)
            .section_by_id(&eggs.id)?
            .ok_or("a Section")?;
        assert_eq!(section.shown_text(), "❌ Eggs");
        assert!(by(ann, carol)?.iter().all(kept_from_blocked));
        Ok(())
    }

    #[test]
    fn blocking_deletes_what_she_wrote_her_photo_too_and_keeps_her_check_offs() -> TestResult {
        let h = household()?;
        let (ann, carol) = (&h.ann, &h.carol);
        let milk = Notes::new(ann).capture("Milk", Some(&h.area), None)?;
        let bins = Routines::new(ann).spawn(&milk.id, &pills()?)?;
        exchange(carol, ann, true)?;
        let rude =
            Notes::new(carol).capture_with("Rude", Some(&h.area), None, &[JPEG.to_vec()], &[])?;
        Notes::new(carol).amend(&milk.id, "Rude too")?;
        Routines::new(carol).mark_done(&bins.id, &at(1)?, None)?;
        exchange(carol, ann, true)?;
        exchange(&h.ben, ann, true)?;
        let hash = rude.photos[0].clone();
        assert!(ann.put_blob("photo", &hash, &carol.photo(&hash).ok_or("bytes")?)?);
        // Ann made a Reminder of Carol's Note: it stays in the Area.
        let cake = Routines::new(ann).spawn(&rude.id, &pills()?)?;
        let carol_max = carol.version_vector()?.get(&carol.device_id()?).copied();

        assert!(ann.block_person(&own_id(carol)?, false)?.is_empty());

        assert_eq!(text_of(ann, &rude.id)?, None);
        assert!(Notes::new(ann).sections(&milk.id)?.is_empty());
        let texts: Vec<String> = Notes::new(ann).all()?.into_iter().map(|n| n.text).collect();
        assert_eq!(texts, ["Milk"]);
        assert!(ann.photo(&hash).is_none());
        assert_eq!(done_keys(ann, &bins.id)?, [at(1)?.key()]);
        assert_eq!(ann.area_of(&cake.id)?.as_deref(), Some(h.area.as_str()));
        let hers = by(ann, carol)?;
        assert!(!hers.is_empty());
        assert!(hers.iter().all(kept_from_blocked));
        assert_eq!(
            ann.version_vector()?.get(&carol.device_id()?).copied(),
            carol_max
        );
        // Nothing of hers goes into the shared folder.
        let mut folder = MemoryFolder::default();
        folder_sync(ann, &mut folder, &mut random_nonce)?;
        assert!(folder.list("blobs")?.is_empty());
        // Ben keeps what he has.
        exchange(&h.ben, ann, true)?;
        assert_eq!(text_of(&h.ben, &rude.id)?.as_deref(), Some("Rude"));
        Ok(())
    }

    #[test]
    fn a_sync_relaying_her_note_through_a_third_device_stores_none_of_it() -> TestResult {
        let h = household()?;
        let (ann, carol, ben) = (&h.ann, &h.carol, &h.ben);
        let milk = Notes::new(ann).capture("Milk", Some(&h.area), None)?;
        let bins = Routines::new(ann).spawn(&milk.id, &pills()?)?;
        exchange(carol, ann, true)?;
        exchange(ben, ann, true)?;
        ann.block_person(&own_id(carol)?, false)?;
        let rude =
            Notes::new(carol).capture_with("Rude", Some(&h.area), None, &[JPEG.to_vec()], &[])?;
        Routines::new(carol).mark_done(&bins.id, &at(1)?, None)?;
        relay_to_ben(&h)?;
        let hash = rude.photos[0].clone();
        exchange(ben, ann, true)?;

        assert_eq!(text_of(ann, &rude.id)?, None);
        let hers = by(ann, carol)?;
        assert!(hers.iter().all(kept_from_blocked));
        let done = crate::routines::keys::done(&at(1)?);
        assert_eq!(hers.iter().filter(|e| e.field == done).count(), 1);
        assert_eq!(done_keys(ann, &bins.id)?, [at(1)?.key()]);
        let reference = format!("photo/{hash}");
        assert!(!ann.missing_blobs()?.contains(&reference));
        assert!(!ann.knows_blob(&reference)?);
        // Nobody is asked for it again.
        let count = ann.all_entries()?.len();
        let (request, _) = exchange(ben, ann, true)?;
        assert_eq!(ann.all_entries()?.len(), count);
        let carol_device = carol.device_id()?;
        let again = request["entries"].as_array().ok_or("entries")?;
        assert!(again.iter().all(|e| e["device"] != carol_device.as_str()));
        Ok(())
    }

    #[test]
    fn her_device_met_only_through_a_grant_that_comes_later_is_cleared_when_it_comes() -> TestResult
    {
        let h = household()?;
        h.ann.block_person(&own_id(&h.carol)?, false)?;
        let rude = Notes::new(&h.carol).capture("Rude", Some(&h.area), None)?;
        relay_to_ben(&h)?;
        exchange(&h.ben, &h.ann, true)?;
        assert_eq!(text_of(&h.ann, &rude.id)?, None);
        Ok(())
    }

    #[test]
    fn what_is_refused_counts_as_held_and_a_forged_entry_of_hers_moves_nothing() -> TestResult {
        let h = household()?;
        let (ann, carol) = (&h.ann, &h.carol);
        ann.block_person(&own_id(carol)?, false)?;
        Notes::new(carol).capture("Rude", Some(&h.area), None)?;
        let mut real = wire(&by(carol, carol)?);
        let last = real
            .iter()
            .rev()
            .find(|e| !wire_kept_from_blocked(e))
            .ok_or("an entry")?
            .clone();
        let mut forged = last.clone();
        forged.dseq = 999;
        real.push(forged);
        ann.apply_entries(
            &real,
            &carol.key_records()?,
            None,
            &mut ImportReport::default(),
        )?;
        let carol_device = carol.device_id()?;
        assert_eq!(
            ann.version_vector()?.get(&carol_device),
            carol.version_vector()?.get(&carol_device)
        );
        assert!(!ann.holds(&carol_device, last.dseq)?);
        Ok(())
    }

    #[test]
    fn unblocking_fetches_her_content_again_from_a_device_holding_it() -> TestResult {
        let h = household()?;
        let (ann, carol, ben) = (&h.ann, &h.carol, &h.ben);
        let milk = Notes::new(ann).capture("Milk", Some(&h.area), None)?;
        let bins = Routines::new(ann).spawn(&milk.id, &pills()?)?;
        exchange(carol, ann, true)?;
        let rude =
            Notes::new(carol).capture_with("Rude", Some(&h.area), None, &[JPEG.to_vec()], &[])?;
        // A check-off of hers above her Note, a Note of her own nobody
        // else gets and an Amend above both.
        Routines::new(carol).mark_done(&bins.id, &at(2)?, None)?;
        Notes::new(carol).capture("Private", None, None)?;
        Notes::new(carol).amend(&rude.id, "Ruder")?;
        relay_to_ben(&h)?;
        exchange(ben, ann, true)?;
        ann.block_person(&own_id(carol)?, false)?;
        assert_eq!(text_of(ann, &rude.id)?, None);
        let first = by(ben, carol)?
            .iter()
            .filter(|e| e.entity == rude.id && !kept_from_blocked(e))
            .map(|e| e.dseq)
            .min()
            .ok_or("an entry")?;

        ann.unblock_person(&own_id(carol)?)?;
        assert!(!ann.is_blocked(&own_id(carol)?)?);
        let carol_device = carol.device_id()?;
        assert!(
            ann.version_vector()?
                .get(&carol_device)
                .copied()
                .unwrap_or(0)
                < first
        );
        exchange(ben, ann, true)?;

        assert_eq!(text_of(ann, &rude.id)?.as_deref(), Some("Rude"));
        assert!(
            ann.missing_blobs()?
                .contains(&format!("photo/{}", rude.photos[0]))
        );
        let sections: Vec<String> = Notes::new(ann)
            .sections(&rude.id)?
            .into_iter()
            .map(|s| s.text)
            .collect();
        assert_eq!(sections, ["Ruder"]);
        assert_eq!(done_keys(ann, &bins.id)?, [at(2)?.key()]);
        assert!(ann.refusal_marks()?.is_empty());
        // Her own Note is not waited for: nobody asks for her entries
        // again.
        let (again, _) = exchange(ben, ann, true)?;
        let entries = again["entries"].as_array().ok_or("entries")?;
        assert!(entries.iter().all(|e| e["device"] != carol_device.as_str()));
        Ok(())
    }

    #[test]
    fn her_profile_loses_its_picture_and_goes_nowhere_and_comes_back_on_unblocking() -> TestResult {
        let ann = device(1, "Ann")?;
        let carol = device(4, "Carol")?;
        OwnProfile::new(&carol).set_picture(Some(JPEG.to_vec()))?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        invite(&ann, &carol, &area)?;
        let id = own_id(&carol)?;
        assert!(ann.person_shown(&id)?.picture.is_some());
        ann.block_person(&id, false)?;

        assert!(ann.person_shown(&id)?.picture.is_none());
        assert_eq!(ann.person_shown(&id)?.name, "Carol");
        assert!(ann.people_shown()?.iter().all(|p| p.name != "Carol"));
        let blocked: Vec<String> = ann.blocked_people()?.into_iter().map(|p| p.name).collect();
        assert_eq!(blocked, ["Carol"]);
        assert!(!serde_json::to_string(&ann.profiles_to_send(None)?)?.contains(&id));
        let areas: HashSet<String> = ann.judged()?.areas.keys().cloned().collect();
        assert!(!serde_json::to_string(&ann.profiles_to_send(Some(&areas))?)?.contains(&id));
        let theirs = carol.own_profile_for_others()?.ok_or("a Profile")?;
        assert_eq!(ann.take_person(&theirs, None)?, crate::PersonTaken::Refused);

        ann.unblock_person(&id)?;
        assert!(ann.blocked_people()?.is_empty());
        exchange(&carol, &ann, true)?;
        assert!(ann.person_shown(&id)?.picture.is_some());
        assert!(ann.people_shown()?.iter().any(|p| p.name == "Carol"));
        Ok(())
    }

    #[test]
    fn the_own_profile_cannot_be_blocked() -> TestResult {
        let ann = device(1, "Ann")?;
        assert!(matches!(
            ann.block_person(&own_id(&ann)?, false),
            Err(Error::Invalid(_))
        ));
        Ok(())
    }

    #[test]
    fn the_block_reaches_the_users_other_device_and_no_other_person() -> TestResult {
        let h = household()?;
        let (ann, desk, carol, ben) = (&h.ann, &h.desk, &h.carol, &h.ben);
        let rude = Notes::new(carol).capture("Rude", Some(&h.area), None)?;
        exchange(carol, ann, true)?;
        exchange(desk, ann, false)?;
        assert_eq!(text_of(desk, &rude.id)?.as_deref(), Some("Rude"));

        ann.block_person(&own_id(carol)?, false)?;
        exchange(desk, ann, false)?;
        assert!(desk.is_blocked(&own_id(carol)?)?);
        assert_eq!(text_of(desk, &rude.id)?, None);
        let names: Vec<String> = desk.blocked_people()?.into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Carol"]);

        let blocks = |store: &Store| -> Made<bool> {
            Ok(store
                .all_entries()?
                .iter()
                .any(|e| e.entity.starts_with(PREFIX)))
        };
        let (_, to_ben) = exchange(ben, ann, true)?;
        let (_, to_carol) = exchange(carol, ann, true)?;
        assert!(!blocks(ben)?);
        assert!(!blocks(carol)?);
        for answer in [to_ben, to_carol] {
            assert!(!answer.to_string().contains(PREFIX));
        }
        let dentist = Notes::new(ann).capture("Dentist", None, None)?;
        let file = ann.send_note(&dentist.id, None, false)?;
        assert!(!String::from_utf8_lossy(&file).contains(PREFIX));
        Ok(())
    }

    #[test]
    fn the_block_reaches_the_other_device_through_the_shared_folder_and_no_other_person()
    -> TestResult {
        let h = household()?;
        h.ann.block_person(&own_id(&h.carol)?, false)?;
        let mut folder = MemoryFolder::default();
        folder_sync(&h.ann, &mut folder, &mut random_nonce)?;
        folder_sync(&h.desk, &mut folder, &mut random_nonce)?;
        folder_sync(&h.ben, &mut folder, &mut random_nonce)?;
        assert!(h.desk.is_blocked(&own_id(&h.carol)?)?);
        assert!(
            h.ben
                .all_entries()?
                .iter()
                .all(|e| !e.entity.starts_with(PREFIX))
        );
        Ok(())
    }

    #[test]
    fn a_block_another_persons_device_wrote_counts_for_nothing() -> TestResult {
        let h = household()?;
        let (ann, carol, ben) = (&h.ann, &h.carol, &h.ben);
        let rude = Notes::new(carol).capture("Rude", Some(&h.area), None)?;
        exchange(carol, ann, true)?;
        // As a modified app would: Ben blocks Carol in Ann's log.
        let forged = ben.append(
            &format!("{PREFIX}{}", own_id(carol)?),
            BLOCKED,
            Some("Carol"),
            None,
        )?;
        ann.apply_entries(
            &wire(std::slice::from_ref(&forged)),
            &[],
            Some(&ben.device_id()?),
            &mut ImportReport::default(),
        )?;
        assert!(ann.holds(&forged.device, forged.dseq)?);
        assert!(!ann.is_blocked(&own_id(carol)?)?);
        assert_eq!(text_of(ann, &rude.id)?.as_deref(), Some("Rude"));
        Ok(())
    }

    #[test]
    fn a_grant_naming_her_for_one_of_the_users_own_devices_blocks_none_of_it() -> TestResult {
        let h = household()?;
        let (ann, desk, carol) = (&h.ann, &h.desk, &h.carol);
        let mine = Notes::new(desk).capture("Mine", Some(&h.area), None)?;
        exchange(desk, ann, false)?;
        // As a modified app of a Manager would: the desk named as Carol's.
        let grant = crate::shared::encode_grant(AreaRole::Member, &own_id(carol)?);
        ann.append(
            &h.area,
            &crate::shared::role_field(&desk.device_id()?),
            Some(&grant),
            None,
        )?;
        let grants = ann.grants_of(&h.area)?.ok_or("grants")?;
        assert_eq!(
            grants.person_of(&desk.device_id()?),
            Some(own_id(carol)?.as_str())
        );
        ann.block_person(&own_id(carol)?, false)?;
        assert_eq!(ann.blocked_devices()?, HashSet::from([carol.device_id()?]));
        assert_eq!(text_of(ann, &mine.id)?.as_deref(), Some("Mine"));
        Ok(())
    }

    #[test]
    fn blocking_a_manager_leaves_the_area_she_manages_only_when_asked() -> TestResult {
        let ann = device(1, "Ann")?;
        let carol = device(4, "Carol")?;
        // Carol shares her own Area with Ann as a Member.
        let school = carol.share(&Places::new(&carol).add_area("School")?.id)?;
        invite(&carol, &ann, &school)?;
        assert_eq!(ann.access_in(&school, None)?, AreaAccess::Member);
        let id = own_id(&carol)?;
        assert_eq!(ann.areas_to_leave_for_blocking(&id)?, vec![school.clone()]);
        let count = ann.all_entries()?.len();
        assert!(matches!(
            ann.block_person(&id, false),
            Err(Error::MustLeaveAreas(areas)) if areas == [school.clone()]
        ));
        assert_eq!(ann.all_entries()?.len(), count);
        assert!(!ann.is_blocked(&id)?);

        assert_eq!(ann.block_person(&id, true)?, vec![school.clone()]);
        assert!(ann.is_blocked(&id)?);
        assert_eq!(ann.access_in(&school, None)?, AreaAccess::None);
        assert!(ann.areas_to_leave_for_blocking(&id)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_member_leaves_an_area_and_its_manager_sees_her_gone() -> TestResult {
        let h = household()?;
        h.ben.leave_area(&h.area)?;
        assert_eq!(h.ben.access_in(&h.area, None)?, AreaAccess::None);
        exchange(&h.ben, &h.ann, true)?;
        let grants = h.ann.grants_of(&h.area)?.ok_or("grants")?;
        assert!(grants.grant_of(&h.ben.device_id()?).is_none());
        let ben_id = own_id(&h.ben)?;
        assert!(h.ann.people(&h.area)?.iter().all(|p| p.id != ben_id));
        Ok(())
    }

    #[test]
    fn a_member_ends_no_other_persons_grant() -> TestResult {
        let h = household()?;
        // As a modified app would: Ben takes Carol out.
        h.ben.append(
            &h.area,
            &crate::shared::role_field(&h.carol.device_id()?),
            None,
            None,
        )?;
        exchange(&h.ben, &h.ann, true)?;
        let grants = h.ann.grants_of(&h.area)?.ok_or("grants")?;
        assert!(grants.grant_of(&h.carol.device_id()?).is_some());
        Ok(())
    }

    #[test]
    fn a_sent_item_from_a_blocked_person_is_neither_shown_nor_accepted() -> TestResult {
        let ann = device(1, "Ann")?;
        let carol = device(4, "Carol")?;
        let rude = Notes::new(&carol).capture("Rude", None, None)?;
        let file = carol.send_note(&rude.id, None, false)?;
        let opened = ann.open_sent_item(&file)?.map_err(|e| format!("{e:?}"))?;
        ann.block_person(&own_id(&carol)?, false)?;
        assert_eq!(
            ann.open_sent_item(&file)?.err(),
            Some(OpenError::FromBlocked)
        );
        assert!(matches!(
            ann.accept_sent_item(&opened.item, false),
            Err(Error::FromBlocked)
        ));
        assert!(Notes::new(&ann).all()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_card_a_blocked_persons_device_made_under_a_new_id_is_refused() -> TestResult {
        use crate::cards::{CardTaken, LINK, card_bytes};
        use base64::Engine;
        use base64::engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD};
        let h = household()?;
        exchange(&h.carol, &h.ann, true)?;
        h.carol.block_person(&own_id(&h.ben)?, false)?;
        let key = SigningKey::from_seed([2; 32]);
        let record = r#"{"id":"0123456789abcdef0123456789abcdef","name":"Not Ben","version":1}"#;
        let me = BASE64.encode(key.public_key());
        let payload = serde_json::json!({
            "format": 1,
            "key": crate::KeyRecord::make(&key).to_json(),
            "profile": crate::profile::SignedProfile {
                record: record.into(),
                signer: me.clone(),
                sig: key.sign(&crate::profile::record_bytes(record)),
                first: me,
                keys: Vec::new(),
                aliases: Vec::new(),
                fresh: false,
            }
            .to_json(),
        })
        .to_string();
        let sig = BASE64.decode(key.sign(&card_bytes(&payload)))?;
        let card = format!(
            "{LINK}{}.{}",
            URL_SAFE_NO_PAD.encode(payload.as_bytes()),
            URL_SAFE_NO_PAD.encode(sig)
        );
        assert_eq!(
            h.carol.open_profile_card(&card, false)?.taken,
            CardTaken::Blocked
        );
        assert_eq!(
            h.carol.person_code("0123456789abcdef0123456789abcdef")?,
            None
        );
        Ok(())
    }

    #[test]
    fn decline_and_block_keeps_nothing_of_it_and_blocks_its_sender() -> TestResult {
        let ann = device(1, "Ann")?;
        let carol = device(4, "Carol")?;
        let rude = Notes::new(&carol).capture("Rude", None, None)?;
        let file = carol.send_note(&rude.id, None, false)?;
        let opened = ann.open_sent_item(&file)?.map_err(|e| format!("{e:?}"))?;
        assert!(ann.block_sent_item_sender(&opened, false)?);
        assert!(Notes::new(&ann).all()?.is_empty());
        assert!(ann.is_blocked(&own_id(&carol)?)?);
        let names: Vec<String> = ann.blocked_people()?.into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Carol"]);
        assert!(ann.people_shown()?.is_empty());
        assert_eq!(
            ann.open_sent_item(&file)?.err(),
            Some(OpenError::FromBlocked)
        );
        Ok(())
    }

    #[test]
    fn a_strangers_one_held_until_show_goes_with_the_block() -> TestResult {
        let ann = device(1, "Ann")?;
        let mallory = device(9, "Mallory")?;
        let rude = Notes::new(&mallory).capture("Rude", None, None)?;
        let file = mallory.send_note(&rude.id, None, false)?;
        let Ok(Received::Held(held)) = ann.receive_sent_item(&file)? else {
            return Err("held".into());
        };
        assert_eq!(ann.pending_sets()?.len(), 1);
        ann.block_person(&held.sender.id, false)?;
        assert!(ann.pending_sets()?.is_empty(), "nothing of hers stored");
        assert!(ann.held_sent_items()?.is_empty());
        assert_eq!(
            ann.receive_sent_item(&file)?.err(),
            Some(OpenError::FromBlocked)
        );
        Ok(())
    }

    #[test]
    fn decline_and_block_on_a_held_one_before_show() -> TestResult {
        let ann = device(1, "Ann")?;
        let mallory = device(9, "Mallory")?;
        let rude = Notes::new(&mallory).capture("Rude", None, None)?;
        let file = mallory.send_note(&rude.id, None, false)?;
        let Ok(Received::Held(held)) = ann.receive_sent_item(&file)? else {
            return Err("held".into());
        };
        assert!(ann.block_held_sent_item_sender(&held.id, false)?);
        assert!(ann.is_blocked(&held.sender.id)?);
        let names: Vec<String> = ann.blocked_people()?.into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Mallory"]);
        assert!(ann.pending_sets()?.is_empty());
        assert!(Notes::new(&ann).all()?.is_empty());
        Ok(())
    }

    #[test]
    fn decline_and_block_blocks_nobody_for_a_copied_id() -> TestResult {
        let h = household()?;
        let mallory = device(9, "Mallory")?;
        let x = Notes::new(&h.carol).capture("x", None, None)?;
        let file = h.carol.send_note(&x.id, None, false)?;
        let mut copied = h.ann.open_sent_item(&file)?.map_err(|e| format!("{e:?}"))?;
        copied.code = OwnProfile::new(&mallory).code()?.ok_or("a code")?;
        assert!(!h.ann.block_sent_item_sender(&copied, false)?);
        assert!(!h.ann.is_blocked(&own_id(&h.carol)?)?);
        Ok(())
    }

    fn entry(entity: &str, field: &str, value: Option<&str>) -> Entry {
        let at = chrono::DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z")
            .map(|t| t.with_timezone(&chrono::Utc))
            .unwrap_or_default();
        Entry {
            seq: 0,
            device: "d".into(),
            dseq: 1,
            entity: entity.into(),
            field: field.into(),
            value: value.map(str::to_string),
            date: "2026-03-01T00:00:00.000Z".into(),
            date_at: at,
            author: String::new(),
            recorded: "2026-03-01T00:00:00.000Z".into(),
            recorded_at: at,
            sig: None,
            ctx: None,
            extra: Default::default(),
            raw: None,
        }
    }

    #[test]
    fn a_check_off_a_grant_and_a_notes_place_are_kept_as_the_apps_write_them() {
        let member = crate::shared::encode_grant(AreaRole::Member, "cd34");
        for e in [
            entry(
                "routine:ab12",
                "done@2026-03-01T08:00",
                Some("2026-03-01T08:00:00.000Z"),
            ),
            entry("routine:ab12", "done@2026-03-01", None),
            entry("frog:ab12", "done", Some("2026-03-01T08:00:00.000Z")),
            entry("frog:ab12", "done", None),
            entry("line:ab12", "checked", Some("2026-03-01T08:00:00.000Z")),
            entry("line:ab12", "checked", None),
            entry(
                "routine:ab12",
                "check@2026-03-01T08:00@line:cd34",
                Some("2026-03-01T08:00:00.000Z"),
            ),
            entry("routine:ab12", "check@2026-03-01@line:cd34", None),
            entry("area:starter-household", "role:ab12", None),
            entry("area:ab12", "role:ab12", Some(&member)),
            entry("note:ab12", "$type", Some("note")),
            entry("note:ab12", "place", Some("topic:ab12")),
            entry("note:ab12", "place", None),
            entry("note:ab12", "box@a7k2", Some("done")),
            entry("note:ab12", "box@0000", Some("open")),
            entry("section:ab12", "box@zz99", Some("failed")),
        ] {
            assert!(kept_from_blocked(&e), "{e:?}");
        }
    }

    #[test]
    fn nothing_else_is_kept_and_nothing_hidden_in_them() {
        let big = format!("routine:{}", "x".repeat(65));
        let mut with_ctx = entry("note:ab12", "place", None);
        with_ctx.ctx = Some(String::new());
        let mut long_author = entry("note:ab12", "place", None);
        long_author.author = "Rude ".repeat(10);
        let mut extra = entry("note:ab12", "place", None);
        extra
            .extra
            .insert("x".into(), serde_json::Value::String("Rude".into()));
        for e in [
            entry("note:ab12", "text", Some("Rude")),
            entry("note:ab12", "captured", Some("2026-03-01T08:00:00.000Z")),
            entry("note:ab12", "photo:ab", Some("added")),
            entry("note:ab12", "transcript:ab", Some("Rude")),
            entry("section:ab12", "$type", Some("section")),
            entry("routine:ab12", "done@2026-03-01T08:00", Some("Rude words")),
            entry("routine:ab12", "done@Rude", None),
            entry("routine:ab12", "done@2026-3-1", None),
            entry("routine:Rude words", "done@2026-03-01", None),
            entry(&big, "done@2026-03-01", None),
            entry("frog:ab12", "done", Some("Rude words")),
            entry("frog:ab12", "trashed", Some("2026-03-01T08:00:00.000Z")),
            entry("frog:ab12", "$type", Some("frog")),
            entry("frog:Rude words", "done", None),
            entry("line:ab12", "checked", Some("Rude words")),
            entry(
                "line:ab12",
                "text",
                Some(r#"{"format":"plain","text":"Rude"}"#),
            ),
            entry("line:ab12", "$type", Some("line")),
            entry("line:Rude words", "checked", None),
            entry(
                "routine:ab12",
                "check@2026-03-01@line:cd34",
                Some("Rude words"),
            ),
            entry("routine:ab12", "check@Rude@line:cd34", None),
            entry("routine:ab12", "check@2026-03-01@line:Rude words", None),
            entry("routine:ab12", "check@2026-03-01@note:cd34", None),
            entry("routine:ab12", "check@2026-03-01@line:", None),
            entry("routine:ab12", "check@2026-03-01", None),
            entry("routine:ab12", "check@@line:cd34", None),
            entry("routine:Rude words", "check@2026-03-01@line:cd34", None),
            entry(
                "area:ab12",
                "role:ab12",
                Some(r#"{"profile":"cd34","role":"member","x":1}"#),
            ),
            entry(
                "area:ab12",
                "role:ab12",
                Some(r#"{"profile":"cd34","role":"member","v":2}"#),
            ),
            entry(
                "area:ab12",
                "role:ab12",
                Some(r#"{"profile":"Rude words","role":"member"}"#),
            ),
            entry("area:ab12", "name", Some("Rude")),
            entry("note:ab12", "$type", Some("Rude")),
            entry("note:ab12", "place", Some("area:Rude words")),
            entry("note:ab12", "box@a7k2", Some("Rude words")),
            entry("note:ab12", "box@a7k2", Some("Done")),
            entry("note:ab12", "box@a7k2", None),
            entry("note:ab12", "box@A7K2", Some("done")),
            entry("note:ab12", "box@a7k", Some("done")),
            entry("note:ab12", "box@a7k22", Some("done")),
            entry("note:ab12", "box@", Some("done")),
            entry("note:ab12", "box@a7-2", Some("done")),
            entry("routine:ab12", "box@a7k2", Some("done")),
            entry("line:ab12", "box@a7k2", Some("done")),
            entry("note:Rude words", "box@a7k2", Some("done")),
            with_ctx,
            long_author,
            extra,
        ] {
            assert!(!kept_from_blocked(&e), "{e:?}");
        }
    }

    /// The entries the pending set `id` of `store` holds from others.
    fn pending_entries(store: &Store, id: &str) -> Made<Vec<WireEntry>> {
        Ok(store
            .pending_set(id)?
            .ok_or("the set")?
            .received()
            .into_iter()
            .map(|r| r.entry.clone())
            .collect())
    }

    #[test]
    fn her_entries_never_enter_a_pending_set_and_leave_one_at_the_block() -> TestResult {
        let h = household()?;
        let (ann, carol) = (&h.ann, &h.carol);
        let milk = Notes::new(ann).capture("Milk", Some(&h.area), None)?;
        let bins = Routines::new(ann).spawn(&milk.id, &pills()?)?;
        exchange(carol, ann, true)?;
        let early = ann.open_pending("bundle", true, None)?;
        let rude = Notes::new(carol).capture("Rude", Some(&h.area), None)?;
        let theirs = wire(&carol.entries_since(&ann.version_vector()?)?);
        ann.receive_pending(&early, &theirs, &[], &mut ImportReport::default())?;
        assert!(text_of(ann, &rude.id)?.is_some());

        ann.block_person(&own_id(carol)?, false)?;

        assert_eq!(text_of(ann, &rude.id)?, None);
        let left = pending_entries(ann, &early)?;
        assert!(!left.is_empty());
        assert!(left.iter().all(wire_kept_from_blocked));
        Notes::new(carol).capture("Ruder", Some(&h.area), None)?;
        Routines::new(carol).mark_done(&bins.id, &at(1)?, None)?;
        let late = ann.open_pending("bundle", true, None)?;
        let theirs = wire(&carol.entries_since(&ann.version_vector()?)?);
        let taken = ann.receive_pending(&late, &theirs, &[], &mut ImportReport::default())?;
        assert!(taken.iter().all(wire_kept_from_blocked));
        let done = routine_keys::done(&at(1)?);
        assert!(taken.iter().any(|e| e.field == done));
        assert!(!taken.iter().any(|e| e.field == note_keys::TEXT));
        Ok(())
    }

    #[test]
    fn her_device_named_only_by_a_grant_in_a_pending_set_is_refused_too() -> TestResult {
        let ann = device(1, "Ann")?;
        let bob = device(2, "Bob")?;
        let carol = device(4, "Carol")?;
        // Ann blocks Carol, whom she knows from no Area.
        ann.block_person(&own_id(&carol)?, false)?;
        let school = bob.share(&Places::new(&bob).add_area("School")?.id)?;
        invite(&bob, &carol, &school)?;
        let rude = Notes::new(&carol).capture("Rude", Some(&school), None)?;
        exchange(&carol, &bob, true)?;
        let offer = ann.open_pending("area-offer", false, None)?;

        ann.receive_pending(
            &offer,
            &wire(&bob.entries_since(&HashMap::new())?),
            &bob.key_records()?,
            &mut ImportReport::default(),
        )?;

        let held = pending_entries(&ann, &offer)?;
        let hers = carol.device_id()?;
        assert!(held.iter().any(|e| e.entity == school));
        assert!(
            held.iter()
                .filter(|e| e.device == hers)
                .all(wire_kept_from_blocked)
        );
        assert!(held.iter().any(|e| e.entity == rude.id));
        assert!(
            !held
                .iter()
                .any(|e| e.entity == rude.id && e.field == note_keys::TEXT)
        );
        // The log's own judgement knows nothing of the offer.
        assert!(ann.blocked_devices()?.is_empty());
        Ok(())
    }

    #[test]
    fn instants_are_read_as_both_cores_write_them() {
        for good in [
            "2026-03-01T08:00:00Z",
            "2026-03-01T08:00:00.000Z",
            "2026-03-01T08:00:00.000123Z",
        ] {
            assert!(is_instant(good), "{good}");
        }
        for bad in [
            "2026-03-01T08:00:00.Z",
            "2026-03-01T08:00:00.0001234Z",
            "2026-03-01 08:00:00Z",
            "2026-03-01T08:00:00+01:00",
            "Rude",
        ] {
            assert!(!is_instant(bad), "{bad}");
        }
    }
}
