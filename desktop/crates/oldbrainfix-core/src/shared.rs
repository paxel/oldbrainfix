//! Shared Areas, as the Dart core's `shared_areas.dart` keeps them
//! (CONTEXT.md, ADR 0013): who may do what in an Area other people sync
//! with is signed into the Area itself, one grant per Device as
//! `role:<device>` on the Area entity. Every Device works out from the
//! raw log which entries a role does not allow ([`judge_log`]) and leaves
//! them out of everything it shows; nothing is refused at import.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Value, json};

use crate::Result;
use crate::checklists::{Checklists, keys as checklist_keys};
use crate::entry::Entry;
use crate::frogs::keys as frog_keys;
use crate::keys::{AreaKey, PERSONAL_SCOPE};
use crate::notes::keys as note_keys;
use crate::places::keys as place_keys;
use crate::places::{Places, starter_id};
use crate::routines::keys as routine_keys;
use crate::sessions::keys as session_keys;
use crate::signing::KeyRecord;
use crate::store::Store;
use crate::tallies::keys as tally_keys;

/// The field prefix of the grants on an Area.
pub const ROLE_PREFIX: &str = "role:";

/// On an Area moved to fresh ids before it was shared: the Area it went
/// on as.
pub const MOVED_TO: &str = "movedTo";

/// The grant field of `device`.
pub fn role_field(device: &str) -> String {
    format!("{ROLE_PREFIX}{device}")
}

/// What a Device may do in a Shared Area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaRole {
    /// Everything.
    Manager,
    /// Add Notes and change their own, Amend any Note and check off
    /// Occurrences.
    Member,
}

impl AreaRole {
    pub fn name(self) -> &'static str {
        match self {
            AreaRole::Manager => "manager",
            AreaRole::Member => "member",
        }
    }

    fn parse(name: &str) -> Option<AreaRole> {
        match name {
            "manager" => Some(AreaRole::Manager),
            "member" => Some(AreaRole::Member),
            _ => None,
        }
    }
}

/// What a Device may do in an Area right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaAccess {
    /// The Area is not shared: its owner's Devices do everything.
    Owner,
    Manager,
    Member,
    /// Shared, and no role here, or removed.
    None,
}

impl AreaAccess {
    pub fn name(self) -> &'static str {
        match self {
            AreaAccess::Owner => "owner",
            AreaAccess::Manager => "manager",
            AreaAccess::Member => "member",
            AreaAccess::None => "none",
        }
    }

    fn of(grant: Option<&Grant>) -> AreaAccess {
        match grant.map(|g| g.role) {
            Some(AreaRole::Manager) => AreaAccess::Manager,
            Some(AreaRole::Member) => AreaAccess::Member,
            None => AreaAccess::None,
        }
    }

    fn full(self) -> bool {
        matches!(self, AreaAccess::Owner | AreaAccess::Manager)
    }
}

/// One grant as it is signed, as the Dart core's `Grant`: the role, and
/// the Profile of the person the Device belongs to (ADR 0017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub role: AreaRole,
    /// The id of the person's Profile, as the grant named it.
    pub profile: String,
}

impl Grant {
    /// The value of the grant field: keys sorted, no spaces.
    pub fn encode(&self) -> String {
        json!({"profile": self.profile, "role": self.role.name()}).to_string()
    }

    /// The grant in `value`; `None` when it is none.
    pub fn parse(value: &str) -> Option<Grant> {
        let json: Value = serde_json::from_str(value).ok()?;
        let profile = json.get("profile")?.as_str()?;
        let role = AreaRole::parse(json.get("role")?.as_str()?)?;
        if profile.is_empty() {
            return None;
        }
        Some(Grant {
            role,
            profile: profile.to_string(),
        })
    }
}

/// What the inviting side of a session hands out, as the Dart core's
/// `Invite`: the Area and the role the other Device gets there. The
/// person it is given to is the one whose Profile that Device sent,
/// which the inviter saw first (ADR 0017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    pub area: String,
    pub role: AreaRole,
}

/// Someone a Shared Area is shared with, as its grants say now: the id
/// of their Profile as the grants name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub role: AreaRole,
    /// The ids of their Devices that hold the role, sorted.
    pub devices: Vec<String>,
}

/// A grant that counts, with the entry that signed it; `None` for a
/// removal.
#[derive(Debug, Clone)]
struct Granted {
    entry: Entry,
    grant: Option<Grant>,
}

/// The grants of one Shared Area that count, in latest-wins order.
#[derive(Debug, Clone)]
pub struct AreaGrants {
    /// The first grant that counts: what is dated before it was the
    /// owner's.
    first: Entry,
    by_device: BTreeMap<String, Vec<Granted>>,
}

impl AreaGrants {
    /// Every Device that ever held a grant here, sorted.
    pub fn devices(&self) -> Vec<String> {
        self.by_device.keys().cloned().collect()
    }

    /// The grant `device` holds now; `None` without one, or once removed.
    pub fn grant_of(&self, device: &str) -> Option<&Grant> {
        self.by_device.get(device)?.last()?.grant.as_ref()
    }

    /// Whether `device` was ever granted anything here.
    pub fn ever_granted(&self, device: &str) -> bool {
        self.by_device.contains_key(device)
    }

    /// The person `device` belongs to, the id of their Profile: of its
    /// last grant that named one.
    pub fn person_of(&self, device: &str) -> Option<&str> {
        self.by_device
            .get(device)?
            .iter()
            .rev()
            .find_map(|g| g.grant.as_ref().map(|grant| grant.profile.as_str()))
    }

    /// The Profile ids the grants that hold now name, sorted.
    pub fn profiles(&self) -> Vec<String> {
        let named: std::collections::BTreeSet<String> = self
            .by_device
            .values()
            .filter_map(|g| g.last()?.grant.as_ref().map(|grant| grant.profile.clone()))
            .collect();
        named.into_iter().collect()
    }

    /// What `device` may do at the entry `at`: everything before the
    /// first grant; else as its last grant before it says, or its first
    /// grant when it had none yet.
    pub fn access_at(&self, device: &str, at: &Entry) -> AreaAccess {
        if Store::latest_wins(at, &self.first) == Ordering::Less {
            return AreaAccess::Owner;
        }
        let Some(granted) = self.by_device.get(device) else {
            return AreaAccess::None;
        };
        let mut pick = granted.first();
        for g in granted {
            if Store::latest_wins(&g.entry, at) == Ordering::Less {
                pick = Some(g);
            }
        }
        AreaAccess::of(pick.and_then(|g| g.grant.as_ref()))
    }
}

/// What [`judge_log`] found: the entries no role allows, and the grants
/// of every Shared Area.
#[derive(Debug, Clone, Default)]
pub struct JudgedLog {
    /// The ids (`<device>:<dseq>`) of the entries left out.
    pub ignored: HashSet<String>,
    /// The Shared Areas by id.
    pub areas: BTreeMap<String, AreaGrants>,
}

impl JudgedLog {
    fn access(&self, area: Option<&str>, e: &Entry) -> AreaAccess {
        match area.and_then(|a| self.areas.get(a)) {
            Some(grants) => grants.access_at(&e.device, e),
            None => AreaAccess::Owner,
        }
    }

    fn person_of(&self, area: Option<&str>, device: &str) -> Option<&str> {
        self.areas.get(area?)?.person_of(device)
    }

    /// The person who wrote the first `$type` of `list`, in `area`.
    fn owner_in(&self, area: Option<&str>, list: &[Entry]) -> Option<&str> {
        let first = list.iter().find(|e| e.field == "$type")?;
        self.person_of(area, &first.device)
    }
}

/// A filing that counts: its entry and the Area it put the entity in.
type Filing = (Entry, Option<String>);

/// Whether a Device of `AreaAccess` may write the entry in the Area.
type May<'a> = &'a dyn Fn(&JudgedLog, AreaAccess, &Entry, Option<&str>) -> bool;

/// The Area of the filing that counts at `e`: the last one before it,
/// else the first one.
fn area_at<'f>(filings: &'f [Filing], e: &Entry) -> Option<&'f str> {
    let mut area = filings.first()?.1.as_deref();
    for (f, a) in filings {
        if Store::latest_wins(f, e) == Ordering::Less {
            area = a.as_deref();
        }
    }
    area
}

/// Which of `entries`, the whole raw log, no role allows, as the Dart
/// core's `judgeLog`: the grants of every Area by latest wins; the Topics
/// by their Area; the Notes by their Area or Topic; the Sections and
/// Routines by their Note, each entry judged in the Area its entity was
/// in at that point.
pub fn judge_log(entries: &[Entry]) -> JudgedLog {
    let mut by_entity: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    for e in entries {
        by_entity
            .entry(e.entity.clone())
            .or_default()
            .push(e.clone());
    }
    for list in by_entity.values_mut() {
        list.sort_by(Store::latest_wins);
    }
    let mut judged = JudgedLog::default();

    // The grants, Area by Area.
    for (entity, list) in &by_entity {
        if !entity.starts_with("area:") {
            continue;
        }
        let mut first: Option<Entry> = None;
        let mut by_device: BTreeMap<String, Vec<Granted>> = BTreeMap::new();
        for e in list {
            let Some(target) = e.field.strip_prefix(ROLE_PREFIX) else {
                continue;
            };
            let grant = e.value.as_deref().and_then(Grant::parse);
            let counts = if target.is_empty() || (e.value.is_some() && grant.is_none()) {
                false
            } else if first.is_none() {
                target == e.device && grant.as_ref().map(|g| g.role) == Some(AreaRole::Manager)
            } else {
                match by_device
                    .get(&e.device)
                    .and_then(|g| g.last())
                    .and_then(|g| g.grant.as_ref())
                {
                    None => false,
                    Some(signer) if signer.role == AreaRole::Manager => true,
                    // A person ends the grants of their own Devices: they
                    // leave.
                    Some(signer) if grant.is_none() => by_device
                        .get(target)
                        .and_then(|g| g.last())
                        .and_then(|g| g.grant.as_ref())
                        .is_some_and(|g| g.profile == signer.profile),
                    Some(signer) => grant.as_ref().is_some_and(|g| {
                        g.role == AreaRole::Member
                            && g.profile == signer.profile
                            && !by_device.contains_key(target)
                    }),
                }
            };
            if !counts {
                judged.ignored.insert(e.id());
                continue;
            }
            if first.is_none() {
                first = Some(e.clone());
            }
            by_device
                .entry(target.to_string())
                .or_default()
                .push(Granted {
                    entry: e.clone(),
                    grant,
                });
        }
        if let Some(first) = first {
            judged
                .areas
                .insert(entity.clone(), AreaGrants { first, by_device });
        }
    }

    let empty: Vec<Entry> = Vec::new();

    // Judges every entry of `list`: the filings first, each in the Area
    // it leaves and the one it enters, then the rest in the Area of the
    // filing before them. Returns the filings that count.
    fn judge(
        judged: &mut JudgedLog,
        list: &[Entry],
        placing: &str,
        area_of: &dyn Fn(Option<&str>) -> Option<String>,
        may: May,
    ) -> Vec<Filing> {
        let mut filings: Vec<Filing> = Vec::new();
        let mut ignored = Vec::new();
        for e in list {
            if e.field != placing {
                continue;
            }
            let target = area_of(e.value.as_deref());
            let from = filings.last().and_then(|f| f.1.clone());
            let ok = (filings.is_empty()
                || may(
                    judged,
                    judged.access(from.as_deref(), e),
                    e,
                    from.as_deref(),
                ))
                && may(
                    judged,
                    judged.access(target.as_deref(), e),
                    e,
                    target.as_deref(),
                );
            if ok {
                filings.push((e.clone(), target));
            } else {
                ignored.push(e.id());
            }
        }
        for e in list {
            if e.field == placing {
                continue;
            }
            let area = area_at(&filings, e);
            if !may(judged, judged.access(area, e), e, area) {
                ignored.push(e.id());
            }
        }
        judged.ignored.extend(ignored);
        filings
    }

    let mut topic_area: HashMap<String, Option<String>> = HashMap::new();
    for (id, list) in &by_entity {
        if !id.starts_with("topic:") {
            continue;
        }
        let filings = judge(
            &mut judged,
            list,
            place_keys::AREA,
            &|value| value.filter(|v| v.starts_with("area:")).map(str::to_string),
            &|_, a, _, _| a.full(),
        );
        topic_area.insert(id.clone(), filings.last().and_then(|f| f.1.clone()));
    }

    let mut note_filings: HashMap<String, Vec<Filing>> = HashMap::new();
    for (id, list) in &by_entity {
        if !id.starts_with("note:") {
            continue;
        }
        let filings = judge(
            &mut judged,
            list,
            note_keys::PLACE,
            &|value| match value {
                Some(v) if v.starts_with("topic:") => topic_area.get(v).cloned().flatten(),
                Some(v) if v.starts_with("area:") => Some(v.to_string()),
                _ => None,
            },
            &|judged, a, e, area| {
                if a.full() {
                    return true;
                }
                if a != AreaAccess::Member {
                    return false;
                }
                // Any Note gets its texts from a Member's machine;
                // removing one is the owner's (ADR 0025).
                if e.value.is_some()
                    && (e.field.starts_with(note_keys::TRANSCRIPT_PREFIX)
                        || e.field.starts_with(note_keys::OCR_PREFIX))
                {
                    return true;
                }
                let owner = judged.owner_in(area, list);
                owner.is_some() && owner == judged.person_of(area, &e.device)
            },
        );
        note_filings.insert(id.clone(), filings);
    }

    for (id, list) in &by_entity {
        // A Tally's entry is its author's, as a Section is.
        let section = id.starts_with("section:") || id.starts_with(tally_keys::ENTRY_PREFIX);
        let frog = id.starts_with(frog_keys::PREFIX);
        let line = id.starts_with(checklist_keys::LINE_PREFIX);
        if !section && !frog && !line && !id.starts_with("routine:") {
            continue;
        }
        let note = list
            .iter()
            .find(|e| e.field == note_keys::NOTE)
            .and_then(|e| e.value.clone());
        let filings = note
            .as_ref()
            .and_then(|n| note_filings.get(n))
            .unwrap_or(&Vec::new())
            .clone();
        let mut ignored = Vec::new();
        for e in list {
            let area = area_at(&filings, e);
            let ok = match judged.access(area, e) {
                AreaAccess::Owner | AreaAccess::Manager => true,
                AreaAccess::None => false,
                AreaAccess::Member => {
                    if section {
                        let owner = judged.owner_in(area, list);
                        owner == judged.person_of(area, &e.device) && owner.is_some()
                    } else if line {
                        // A Member checks any Line and adds their own; the
                        // rest is for the author of the Line or of its Note
                        // (ADR 0030).
                        let note_list = note
                            .as_ref()
                            .and_then(|n| by_entity.get(n))
                            .unwrap_or(&empty);
                        e.field == checklist_keys::CHECKED
                            || judged.person_of(area, &e.device).is_some_and(|writer| {
                                judged.owner_in(area, list) == Some(writer)
                                    || judged.owner_in(area, note_list) == Some(writer)
                            })
                    } else if frog {
                        // A Member checks off; the Frog is the Manager's.
                        e.field == frog_keys::DONE
                    } else {
                        // So are a Session's picks and the entries
                        // recorded for them (ADR 0029); the Routine is
                        // the Manager's.
                        e.field.starts_with(routine_keys::DONE_PREFIX)
                            || e.field.starts_with(session_keys::PICK_PREFIX)
                            || e.field.starts_with(session_keys::ENTRY_PREFIX)
                    }
                }
            };
            if !ok {
                ignored.push(e.id());
            }
        }
        judged.ignored.extend(ignored);
    }

    // An Area's own fields, but for the grants judged above.
    let mut ignored = Vec::new();
    for (entity, grants) in &judged.areas {
        for e in by_entity.get(entity).unwrap_or(&empty) {
            if e.field.starts_with(ROLE_PREFIX) {
                continue;
            }
            if !grants.access_at(&e.device, e).full() {
                ignored.push(e.id());
            }
        }
    }
    judged.ignored.extend(ignored);
    judged
}

impl Store {
    /// The grants of the Shared Area `area`, or `None` when it is not
    /// shared.
    pub fn grants_of(&self, area: &str) -> Result<Option<AreaGrants>> {
        Ok(self.judged()?.areas.get(area).cloned())
    }

    pub fn is_shared(&self, area: &str) -> Result<bool> {
        Ok(self.judged()?.areas.contains_key(area))
    }

    /// The Area `entity` lies in now, or `None` when it lies in none.
    pub fn area_of(&self, entity: &str) -> Result<Option<String>> {
        let scope = self.scope_of(entity)?;
        Ok((scope != PERSONAL_SCOPE).then_some(scope))
    }

    /// What `device` (this one when `None`) may do in `area` now.
    pub fn access_in(&self, area: &str, device: Option<&str>) -> Result<AreaAccess> {
        let judged = self.judged()?;
        let Some(grants) = judged.areas.get(area) else {
            return Ok(AreaAccess::Owner);
        };
        let device = match device {
            Some(d) => d.to_string(),
            None => self.device_id()?,
        };
        Ok(AreaAccess::of(grants.grant_of(&device)))
    }

    /// Whether this Device may change the Area `area` itself, its Topics
    /// and its Routines and Reminders.
    pub fn may_manage(&self, area: Option<&str>) -> Result<bool> {
        match area {
            None => Ok(true),
            Some(area) => Ok(self.access_in(area, None)?.full()),
        }
    }

    /// Whether this Device may change the Note or Section `id`.
    pub fn may_change(&self, id: &str) -> Result<bool> {
        let area = self.area_of(id)?;
        if self.may_manage(area.as_deref())? {
            return Ok(true);
        }
        let Some(area) = area else { return Ok(false) };
        if self.access_in(&area, None)? != AreaAccess::Member {
            return Ok(false);
        }
        let Some(grants) = self.grants_of(&area)? else {
            return Ok(false);
        };
        let Some(mine) = grants.person_of(&self.device_id()?).map(str::to_string) else {
            return Ok(false);
        };
        let creator = self
            .history(id)?
            .into_iter()
            .find(|e| e.field == "$type")
            .and_then(|e| grants.person_of(&e.device).map(str::to_string));
        Ok(creator.as_deref() == Some(mine.as_str()))
    }

    /// Whether this Device may write anything at all in the Area `entity`
    /// lies in: in none, or holding a role there.
    pub fn may_write_in(&self, entity: &str) -> Result<bool> {
        let area = if entity.starts_with("area:") {
            Some(entity.to_string())
        } else {
            self.area_of(entity)?
        };
        match area {
            None => Ok(true),
            Some(area) => Ok(self.access_in(&area, None)? != AreaAccess::None),
        }
    }

    /// Whether this Device may pick a version of a Conflict on `entity`
    /// of `ty` (`$type`), as the Dart core's `mayResolve`.
    pub fn may_resolve(&self, entity: &str, ty: &str) -> Result<bool> {
        match ty {
            "note" | "section" | tally_keys::TYPE_ENTRY => self.may_change(entity),
            checklist_keys::TYPE_LINE => Checklists::new(self).may_edit(entity),
            "routine" | "frog" => self.may_change_routine(entity),
            "area" => self.may_manage(Some(entity)),
            "topic" => self.may_manage(self.area_of(entity)?.as_deref()),
            _ => Ok(true),
        }
    }

    /// Whether this Device may change the Routine or Reminder `id`.
    pub fn may_change_routine(&self, id: &str) -> Result<bool> {
        self.may_manage(self.area_of(id)?.as_deref())
    }

    /// Whether this Device may check off Occurrences of the Routine `id`.
    pub fn may_check_off(&self, id: &str) -> Result<bool> {
        match self.area_of(id)? {
            None => Ok(true),
            Some(area) => Ok(self.access_in(&area, None)? != AreaAccess::None),
        }
    }

    /// The people of the Shared Area `area` as its grants say now, this
    /// Device's own first, then by the name of their Profile.
    pub fn people(&self, area: &str) -> Result<Vec<Person>> {
        let Some(grants) = self.grants_of(area)? else {
            return Ok(Vec::new());
        };
        let mut people: Vec<Person> = Vec::new();
        for device in grants.devices() {
            let Some(grant) = grants.grant_of(&device) else {
                continue;
            };
            match people.iter_mut().find(|p| p.id == grant.profile) {
                Some(person) => person.devices.push(device),
                None => people.push(Person {
                    id: grant.profile.clone(),
                    role: grant.role,
                    devices: vec![device],
                }),
            }
        }
        let mine = grants.person_of(&self.device_id()?).map(str::to_string);
        let mut names = HashMap::new();
        for p in &people {
            names.insert(p.id.clone(), self.person_shown(&p.id)?.name);
        }
        people.sort_by(|a, b| {
            let a_mine = Some(&a.id) == mine.as_ref();
            let b_mine = Some(&b.id) == mine.as_ref();
            b_mine
                .cmp(&a_mine)
                .then_with(|| crate::store::utf16_cmp(&names[&a.id], &names[&b.id]))
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(people)
    }

    /// The Shared Areas `device` holds a role in now.
    pub fn areas_shared_with(&self, device: &str) -> Result<HashSet<String>> {
        Ok(self
            .judged()?
            .areas
            .iter()
            .filter(|(_, g)| g.grant_of(device).is_some())
            .map(|(id, _)| id.clone())
            .collect())
    }

    /// Shares the Area `area`, as the Dart core's `share`: moves it to
    /// fresh ids first when it has a Starter Set id, then makes this
    /// Device its first Manager, under the own Profile, and every own
    /// paired Device a Manager with it. Returns the id it goes on under.
    pub fn share(&self, area: &str) -> Result<String> {
        if self.is_shared(area)? {
            return Ok(area.to_string());
        }
        self.batch(|| {
            let starter = self
                .current(area, place_keys::STARTER)?
                .map(|slug| starter_id(place_keys::TYPE_AREA, &slug));
            let id = if starter.as_deref() == Some(area) {
                self.rehome(area)?
            } else {
                area.to_string()
            };
            let me = self.device_id()?;
            // Without a Profile yet (only in tests) the Device stands for
            // it.
            let profile = match crate::profile::OwnProfile::new(self).get()? {
                Some(own) => own.id,
                None => me.clone(),
            };
            let grant = Grant {
                role: AreaRole::Manager,
                profile,
            }
            .encode();
            self.append(&id, &role_field(&me), Some(&grant), None)?;
            for device in self.paired_devices()? {
                if !device.shared {
                    self.append(&id, &role_field(&device.id), Some(&grant), None)?;
                }
            }
            Ok(id)
        })
    }

    /// Grants `device` `role` in `area`, as the person whose Profile is
    /// `profile`: under the person it already belongs to there, else
    /// that one. Nothing for a Device of this one's own group.
    pub fn invite(&self, area: &str, device: &str, role: AreaRole, profile: &str) -> Result<()> {
        // A Device of this one's own group is no other person: it keeps
        // the role it has.
        if self.is_own_device(device)? {
            return Ok(());
        }
        let person = self
            .grants_of(area)?
            .and_then(|g| g.person_of(device).map(str::to_string))
            .unwrap_or_else(|| profile.to_string());
        let grant = Grant {
            role,
            profile: person,
        };
        self.append(area, &role_field(device), Some(&grant.encode()), None)
            .map(drop)
    }

    /// Gives every Device of `person` in `area` the role `role`.
    pub fn set_role(&self, area: &str, person: &str, role: AreaRole) -> Result<()> {
        let Some(grants) = self.grants_of(area)? else {
            return Ok(());
        };
        self.batch(|| {
            for device in grants.devices() {
                let Some(grant) = grants.grant_of(&device) else {
                    continue;
                };
                if grant.profile != person || grant.role == role {
                    continue;
                }
                let next = Grant {
                    role,
                    profile: person.to_string(),
                };
                self.append(area, &role_field(&device), Some(&next.encode()), None)?;
            }
            Ok(())
        })
    }

    /// Takes `person` out of `area`: their Devices' grants end and no
    /// Device that learns of it sends them anything more of the Area.
    pub fn remove_person(&self, area: &str, person: &str) -> Result<()> {
        let Some(grants) = self.grants_of(area)? else {
            return Ok(());
        };
        self.batch(|| {
            for device in grants.devices() {
                if grants.grant_of(&device).map(|g| g.profile.as_str()) == Some(person) {
                    self.append(area, &role_field(&device), None, None)?;
                }
            }
            Ok(())
        })
    }

    /// Leaves the Shared Area `area` (#90), as the Dart core's
    /// `leaveArea`: ends the grant of every Device of the user's there,
    /// this Device's last, so nothing more of it comes or goes. What is
    /// here stays as it was. A Member may leave too: a person may end the
    /// grants of their own Devices.
    pub fn leave_area(&self, area: &str) -> Result<()> {
        let Some(grants) = self.grants_of(area)? else {
            return Ok(());
        };
        let own_device = self.device_id()?;
        let mine = grants.grant_of(&own_device).map(|g| g.profile.clone());
        let mut devices = Vec::new();
        for device in grants.devices() {
            let Some(grant) = grants.grant_of(&device) else {
                continue;
            };
            if Some(&grant.profile) == mine.as_ref() || self.is_own_person(&grant.profile)? {
                devices.push(device);
            }
        }
        devices.sort_by_key(|d| *d == own_device);
        if devices.is_empty() {
            return Ok(());
        }
        self.batch(|| {
            for device in &devices {
                self.append(area, &role_field(device), None, None)?;
            }
            Ok(())
        })
    }

    /// Gives the Device `peer` of this Device's own group this Device's
    /// role and person in every Shared Area where it holds one and
    /// `peer` never held any. Returns how many it gave.
    pub fn extend_roles_to(&self, peer: &str) -> Result<usize> {
        let me = self.device_id()?;
        if peer.is_empty() || peer == me {
            return Ok(0);
        }
        let given: Vec<(String, String)> = self
            .judged()?
            .areas
            .iter()
            .filter(|(_, g)| !g.ever_granted(peer))
            .filter_map(|(id, g)| g.grant_of(&me).map(|grant| (id.clone(), grant.encode())))
            .collect();
        if given.is_empty() {
            return Ok(0);
        }
        self.batch(|| {
            for (area, grant) in &given {
                self.append(area, &role_field(peer), Some(grant), None)?;
            }
            Ok(())
        })?;
        Ok(given.len())
    }

    /// Whether this Device must refuse an invite to `area`: it holds an
    /// Area of that id nobody shared.
    pub fn refuses_invite(&self, area: &str) -> Result<bool> {
        Ok(!self.is_shared(area)? && !self.history(area)?.is_empty())
    }

    /// Moves the Area `area` with its Topics to fresh ids, as the Dart
    /// core's `_rehome`. Runs inside a batch.
    fn rehome(&self, area: &str) -> Result<String> {
        let places = Places::new(self);
        let Some(old) = places.area_by_id(area)? else {
            return Ok(area.to_string());
        };
        let id = format!("area:{}", self.new_id()?);
        self.append(&id, place_keys::TYPE, Some(place_keys::TYPE_AREA), None)?;
        self.append(&id, place_keys::NAME, Some(&old.name), None)?;
        if let Some(starter) = &old.starter {
            self.append(&id, place_keys::STARTER, Some(starter), None)?;
        }
        if let Some(colour) = &old.colour {
            self.append(&id, place_keys::COLOUR, Some(colour), None)?;
        }
        if let Some(icon) = &old.icon {
            self.append(&id, place_keys::ICON, Some(icon), None)?;
        }
        let mut moved: BTreeMap<String, String> = BTreeMap::new();
        moved.insert(area.to_string(), id.clone());
        for topic in places.topics(area)? {
            let topic_id = format!("topic:{}", self.new_id()?);
            self.append(
                &topic_id,
                place_keys::TYPE,
                Some(place_keys::TYPE_TOPIC),
                None,
            )?;
            self.append(&topic_id, place_keys::AREA, Some(&id), None)?;
            self.append(&topic_id, place_keys::NAME, Some(&topic.name), None)?;
            if let Some(starter) = &topic.starter {
                self.append(&topic_id, place_keys::STARTER, Some(starter), None)?;
            }
            moved.insert(topic.id, topic_id);
        }
        let mut notes = self.entities_of_type(note_keys::TYPE_NOTE)?;
        notes.sort();
        for note in notes {
            let to = self
                .current(&note, note_keys::PLACE)?
                .and_then(|p| moved.get(&p).cloned());
            if let Some(to) = to {
                self.append(&note, note_keys::PLACE, Some(&to), None)?;
            }
        }
        for old_id in moved.keys() {
            if old_id != area {
                self.append(old_id, place_keys::TYPE, None, None)?;
            }
        }
        self.append(area, MOVED_TO, Some(&id), None)?;
        self.append(area, place_keys::TYPE, None, None)?;
        Ok(id)
    }

    // -------------------------------------------------------------- sending

    /// What goes to the Device `peer` paired by invite, of what a Device
    /// holding `vector` lacks, as the Dart core's `entriesFor`: the
    /// entries of the Shared Areas `peer` holds a role in now (and
    /// `also`), with the whole history of every Note filed into one of
    /// them meanwhile; `whole` sends the Areas' whole history.
    pub fn entries_for(
        &self,
        peer: &str,
        vector: &HashMap<String, i64>,
        whole: bool,
        also: &[String],
    ) -> Result<Vec<Entry>> {
        let mut areas = self.areas_shared_with(peer)?;
        areas.extend(also.iter().cloned());
        if areas.is_empty() {
            return Ok(Vec::new());
        }
        let delta = self.entries_since(&if whole {
            HashMap::new()
        } else {
            vector.clone()
        })?;
        let list = if whole {
            delta
        } else {
            self.with_filed(delta, &|a| areas.contains(a))?
        };
        let mut scopes: HashMap<String, String> = HashMap::new();
        let mut out = Vec::new();
        for e in list {
            let scope = match scopes.get(&e.entity) {
                Some(s) => s.clone(),
                None => {
                    let s = self.scope_of(&e.entity)?;
                    scopes.insert(e.entity.clone(), s.clone());
                    s
                }
            };
            if areas.contains(&scope) {
                out.push(e);
            }
        }
        Ok(out)
    }

    /// `delta` with the whole history of every Note a filing in it put
    /// into an Area `wanted` says yes to, with its Sections, Routines,
    /// Frogs, Tally entries and Checklist Lines, as the Dart core's
    /// `withFiled`; by (device, dseq), each entry once.
    pub fn with_filed(
        &self,
        delta: Vec<Entry>,
        wanted: &dyn Fn(&str) -> bool,
    ) -> Result<Vec<Entry>> {
        let mut notes: HashSet<String> = HashSet::new();
        for e in &delta {
            if e.field == note_keys::PLACE
                && e.entity.starts_with("note:")
                && let Some(area) = self.area_of(&e.entity)?
                && wanted(&area)
            {
                notes.insert(e.entity.clone());
            }
        }
        if notes.is_empty() {
            return Ok(delta);
        }
        let mut entities = notes.clone();
        for ty in [
            note_keys::TYPE_SECTION,
            routine_keys::TYPE_ROUTINE,
            frog_keys::TYPE_FROG,
            tally_keys::TYPE_ENTRY,
            checklist_keys::TYPE_LINE,
        ] {
            for id in self.entities_of_type(ty)? {
                if let Some(note) = self.current(&id, note_keys::NOTE)?
                    && notes.contains(&note)
                {
                    entities.insert(id);
                }
            }
        }
        let mut seen: HashSet<String> = delta.iter().map(Entry::id).collect();
        let mut out = delta;
        for e in self.raw_history_of(&entities)? {
            if seen.insert(e.id()) {
                out.push(e);
            }
        }
        out.sort_by(|a, b| a.device.cmp(&b.device).then(a.dseq.cmp(&b.dseq)));
        Ok(out)
    }

    /// Every entry that lies in the Area `area` now, by (device, dseq):
    /// its whole history, as an offer from afar carries it (#108), as the
    /// Dart core's `areaHistory`.
    pub fn area_history(&self, area: &str) -> Result<Vec<Entry>> {
        let mut scopes: HashMap<String, String> = HashMap::new();
        let mut out = Vec::new();
        for e in self.entries_since(&HashMap::new())? {
            let scope = match scopes.get(&e.entity) {
                Some(s) => s.clone(),
                None => {
                    let s = self.scope_of(&e.entity)?;
                    scopes.insert(e.entity.clone(), s.clone());
                    s
                }
            };
            if scope == area {
                out.push(e);
            }
        }
        Ok(out)
    }

    /// The Devices the grants of the Shared Areas give `person` a role on
    /// now, never one of the user's own; sorted.
    pub fn devices_of(&self, person: &str) -> Result<Vec<String>> {
        let target = self.resolve_person(person)?;
        let judged = self.judged()?;
        let mut out = std::collections::BTreeSet::new();
        for grants in judged.areas.values() {
            for device in grants.devices() {
                let Some(grant) = grants.grant_of(&device) else {
                    continue;
                };
                if self.resolve_person(&grant.profile)? == target
                    && !self.written_by_own(&device)?
                {
                    out.insert(device);
                }
            }
        }
        Ok(out.into_iter().collect())
    }

    /// What a bundle for the person `person` carries (#108), as the Dart
    /// core's `entriesForPerson`: what lies in the Areas shared with any
    /// Device of theirs ([`Store::devices_of`]), beyond what any of those
    /// Devices said it holds, with the whole history of Notes filed into
    /// them meanwhile.
    pub fn entries_for_person(&self, person: &str) -> Result<Vec<Entry>> {
        let devices = self.devices_of(person)?;
        let Some(first) = devices.first() else {
            return Ok(Vec::new());
        };
        let mut vector: HashMap<String, i64> = HashMap::new();
        let mut areas: HashSet<String> = HashSet::new();
        for device in &devices {
            for (key, value) in self.held_by(device)? {
                let held = vector.entry(key).or_insert(0);
                if value > *held {
                    *held = value;
                }
            }
            areas.extend(self.areas_shared_with(device)?);
        }
        let also: Vec<String> = areas.into_iter().collect();
        self.entries_for(first, &vector, false, &also)
    }

    /// The signing keys the people of the Shared Area `area` check its
    /// entries with (#108): this Device's own and those held here of
    /// every Device its grants ever named, by device.
    pub fn signing_keys_of(&self, area: &str) -> Result<Vec<KeyRecord>> {
        let own = self.device_id()?;
        let mut out = vec![self.own_key_record()?];
        if let Some(grants) = self.grants_of(area)? {
            for device in grants.devices() {
                if device == own {
                    continue;
                }
                if let Some(pinned) = self.pinned_key(&device)? {
                    out.push(pinned.record);
                }
            }
        }
        out.sort_by(|a, b| a.device.cmp(&b.device));
        Ok(out)
    }

    /// The Area keys `peer`, paired by invite, gets: those of the Areas
    /// it holds a role in, and of `also`.
    pub fn area_keys_for(&self, peer: &str, also: &[String]) -> Result<Vec<AreaKey>> {
        let mut areas = self.areas_shared_with(peer)?;
        areas.extend(also.iter().cloned());
        Ok(self
            .area_keys()?
            .into_iter()
            .filter(|k| areas.contains(&k.scope))
            .collect())
    }

    /// Whether the blob `reference` (`photo/<hash>`) lies on a Note or
    /// Section of one of `areas`.
    pub fn blob_in(&self, reference: &str, areas: &HashSet<String>) -> Result<bool> {
        let Some((kind, hash)) = reference.split_once('/') else {
            return Ok(false);
        };
        let prefix = match kind {
            "photo" => note_keys::PHOTO_PREFIX,
            "audio" => note_keys::AUDIO_PREFIX,
            _ => return Ok(false),
        };
        let field = format!("{prefix}{hash}");
        for (entity, f) in self.fields_with_prefix(&field)? {
            if f != field || self.current(&entity, &f)?.is_none() {
                continue;
            }
            if areas.contains(&self.scope_of(&entity)?) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether the Device `id` is another person's, paired by invite.
    pub fn is_shared_peer(&self, id: &str) -> Result<bool> {
        Ok(self.paired_device(id)?.is_some_and(|d| d.shared))
    }

    /// Whether the Device `id` is one of this Device's own group: paired
    /// without an invite.
    pub fn is_own_device(&self, id: &str) -> Result<bool> {
        Ok(self.paired_device(id)?.is_some_and(|d| !d.shared))
    }

    /// Whether the Device `id` is another person's as far as this Device
    /// knows (#108), as the Dart core's `isOthersDevice`: paired by
    /// invite, given a role in a Shared Area for another person, or bound
    /// to the Profile of another person kept here, as one added from a
    /// Profile card; never one of the user's own.
    pub fn is_others_device(&self, id: &str) -> Result<bool> {
        if self.is_shared_peer(id)? {
            return Ok(true);
        }
        if self.written_by_own(id)? {
            return Ok(false);
        }
        let judged = self.judged()?;
        for grants in judged.areas.values() {
            if let Some(person) = grants.person_of(id)
                && !self.is_own_person(person)?
            {
                return Ok(true);
            }
        }
        for person in self.kept_people()? {
            if self.pinned_devices_of(&person.id)?.iter().any(|d| d == id) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
    };
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use crate::{Notes, Places, Routines};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    /// A Device with its own Profile: Ann's, Ben's or the laptop's.
    fn device(seed: u8) -> Result<Store> {
        let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
        let name = ["", "Ann", "Ben", "Laptop"][usize::from(seed).min(3)];
        crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    #[test]
    fn a_member_writes_only_what_a_member_may() -> TestResult {
        let ann = device(1)?;
        let ben = device(2)?;
        let house = Places::new(&ann).add_area("Household")?;
        let milk = Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
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
        assert!(!ben.may_change(&milk.id)?);
        // Written all the same, as a modified app would: ignored.
        Notes::new(&ben).edit(&milk.id, "Buy beer")?;
        let own = Notes::new(&ben).capture("Bread", Some(&area), None)?;
        assert!(ben.may_change(&own.id)?);
        assert!(Places::new(&ben).add_topic(&area, "Garage").is_err());
        assert!(
            Routines::new(&ben)
                .spawn(
                    &own.id,
                    &crate::Schedule {
                        start: crate::Day::new(2026, 3, 1),
                        times: Vec::new(),
                        rule: None,
                    }
                )
                .is_err()
        );
        assert_eq!(
            Notes::new(&ben).by_id(&milk.id)?.map(|n| n.text),
            Some("Buy milk".into())
        );
        assert!(!ben.refuses_invite("area:nothing")?);
        Ok(())
    }

    #[test]
    fn an_own_device_is_not_invited() -> TestResult {
        let ann = device(1)?;
        let laptop = device(3)?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        ann.remember_device(&crate::PairedDevice {
            id: laptop.device_id()?,
            name: "Laptop".into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: ann.now(),
            shared: false,
        })?;
        let invite = Invite {
            area,
            role: AreaRole::Member,
        };
        let invited = sync_request_as(
            &laptop,
            &ann.version_vector()?,
            "Laptop",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        assert!(answer_sync_inviting(&ann, &invited, "Desk", Some(&invite))?.is_none());
        let host_profile = crate::lan::profile_answer(&ann)?;
        let inviting = sync_request_as(
            &laptop,
            &ann.version_vector()?,
            "Laptop",
            &ann.device_id()?,
            "",
            Joining::Inviting(&invite, host_profile.get("profile")),
        )?;
        assert!(answer_sync(&ann, &inviting, "Desk")?.is_none());
        assert!(!ann.is_shared_peer(&laptop.device_id()?)?);
        // Nor is it granted anything as another person.
        ann.invite(&invite.area, &laptop.device_id()?, AreaRole::Member, "ben")?;
        let mut names = Vec::new();
        for p in ann.people(&invite.area)? {
            names.push(ann.person_shown(&p.id)?.name);
        }
        assert_eq!(names, vec!["Ann".to_string()]);
        Ok(())
    }

    #[test]
    fn an_own_device_met_in_a_sync_gets_the_same_role() -> TestResult {
        let ann = device(1)?;
        let laptop = device(3)?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        let request = sync_request(&laptop, &ann.version_vector()?, "Laptop", "", "")?;
        let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        crate::lan::apply_answer(&laptop, &answer)?;
        let id = laptop.device_id()?;
        assert_eq!(laptop.access_in(&area, None)?, AreaAccess::Manager);
        assert_eq!(ann.access_in(&area, Some(&id))?, AreaAccess::Manager);
        assert_eq!(ann.people(&area)?.len(), 1);
        assert_eq!(ann.extend_roles_to(&id)?, 0, "given once");
        Ok(())
    }
}
