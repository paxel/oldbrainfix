//! Other people's Profiles, as the Dart core's `people.dart` keeps them
//! (ADR 0017, #50): the Profiles of the people this Device shares an
//! Area with, as they arrived with the Area's sync or a bundle. Each is
//! kept as a local setting `person:<id>`, as received with its
//! signature, so it can be passed on unchanged:
//! `{"profile":…,"seen":…,"shown":…}`, the signed Profile, when it was
//! first seen here, and its picture made again for the screen. On first
//! sight an id is bound to the keys that came with it; later a record
//! counts only when one of them signed it. `"verified":true` says the
//! user compared its short code in person (#51); it stays on this
//! Device and is kept through every newer record. `"stripped":true` says
//! the person was blocked and their picture taken out of the record (ADR
//! 0021), so its signature no longer holds and it is never passed on;
//! any record of theirs at the same version or newer replaces it.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};

use crate::Result;
use crate::entry::dart_iso;
use crate::notes::Note;
use crate::profile::{
    ALIASES, FIRST, KEYS, OwnProfile, Profile, ProfileAlias, ProfileChoice, RECORD, SentProfile,
    SignedProfile, chain, check, is_key, replaces, short_code,
};
use crate::signing::device_id_from_key;
use crate::store::Store;
use crate::text_boxes::BoxTick;

const PREFIX: &str = "person:";

/// What became of another person's Profile that came in, the Dart
/// core's `PersonTaken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonTaken {
    /// Nothing new.
    Unchanged,
    /// Its signatures did not hold, it was signed by a key not bound to
    /// the Profile kept here, a copied id, or not wanted from that
    /// sender.
    Refused,
    /// Seen for the first time and kept, bound to the keys it came with.
    Added,
    /// A newer record, keys or aliases of a Profile kept here.
    Updated,
}

impl PersonTaken {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            PersonTaken::Unchanged => "unchanged",
            PersonTaken::Refused => "refused",
            PersonTaken::Added => "added",
            PersonTaken::Updated => "updated",
        }
    }

    fn changed(self) -> bool {
        matches!(self, PersonTaken::Added | PersonTaken::Updated)
    }
}

/// A Profile of another person as this Device keeps it.
#[derive(Debug, Clone)]
pub(crate) struct Held {
    sent: SignedProfile,
    /// The record read, with the picture made again.
    profile: Profile,
    bound: HashSet<String>,
    seen: String,
    /// Whether the user compared its short code in person.
    verified: bool,
    /// Whether the picture was taken out of the record for a block.
    stripped: bool,
}

/// Every kept Profile by id, and the ids they replaced.
#[derive(Debug, Default)]
pub struct PeopleIndex {
    by_id: BTreeMap<String, Held>,
    alias_to: BTreeMap<String, String>,
}

/// Who wrote a Note: the user, another person, or nobody known.
enum Writer {
    Own,
    Theirs(Profile),
    Unknown,
}

impl Writer {
    fn theirs(author: Option<Profile>) -> Writer {
        author.map_or(Writer::Unknown, Writer::Theirs)
    }
}

/// Which people's Notes the notes browser shows (#53), the Dart core's
/// `PersonFilter`: the user's own with `me`, and those of `people`, by
/// Profile id; everybody's when neither is set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonFilter {
    pub me: bool,
    pub people: BTreeSet<String>,
}

impl PersonFilter {
    /// Every Note, whoever wrote it.
    pub fn is_everyone(&self) -> bool {
        !self.me && self.people.is_empty()
    }

    /// Only the Notes of the person `id`.
    pub fn person(id: &str) -> PersonFilter {
        PersonFilter {
            me: false,
            people: BTreeSet::from([id.to_string()]),
        }
    }

    /// Puts the person `id` in, or out again.
    pub fn toggle(&mut self, id: &str) {
        if !self.people.remove(id) {
            self.people.insert(id.to_string());
        }
    }
}

/// The own Profile's ids and keys, with what they were read from.
#[derive(Debug)]
pub struct OwnKeys {
    stamp: String,
    ids: HashSet<String>,
    bound: HashSet<String>,
}

fn read_held(raw: &str) -> Option<Held> {
    let json: Value = serde_json::from_str(raw).ok()?;
    let seen = json.get("seen")?.as_str()?.to_string();
    let sent = SignedProfile::from_json(json.get("profile")?)?;
    let mut profile = Profile::parse(&sent.record)?;
    profile.picture = json
        .get("shown")
        .and_then(Value::as_str)
        .and_then(|s| BASE64.decode(s).ok());
    let bound = chain(&profile.id, &sent.first, &[], &sent.keys).0;
    Some(Held {
        sent,
        profile,
        bound,
        seen,
        verified: json.get("verified") == Some(&Value::Bool(true)),
        stripped: json.get("stripped") == Some(&Value::Bool(true)),
    })
}

/// The own Profile as it goes to another person: without what only
/// one's own Devices are told.
fn for_others(own: &SignedProfile) -> SignedProfile {
    SignedProfile {
        fresh: false,
        ..own.clone()
    }
}

impl Store {
    fn people_index(&self) -> Result<Arc<PeopleIndex>> {
        if let Some(held) = self.people.lock().as_ref() {
            return Ok(held.clone());
        }
        let mut index = PeopleIndex::default();
        for (key, raw) in self.local_settings_with_prefix(PREFIX)? {
            if let Some(held) = read_held(&raw)
                && key == format!("{PREFIX}{}", held.profile.id)
            {
                index.by_id.insert(held.profile.id.clone(), held);
            }
        }
        // An id counts as replaced only when its own Profile is kept here
        // and a key bound to it made the alias: one Profile cannot claim
        // to replace somebody else's.
        let mut alias_to = BTreeMap::new();
        for (id, held) in &index.by_id {
            for a in &held.sent.aliases {
                if a.alias != *id && replaced(&index.by_id, a) {
                    alias_to
                        .entry(a.alias.clone())
                        .or_insert_with(|| id.clone());
                }
            }
        }
        index.alias_to = alias_to;
        let index = Arc::new(index);
        *self.people.lock() = Some(index.clone());
        Ok(index)
    }

    fn own_keys(&self) -> Result<Arc<OwnKeys>> {
        let record = self.local_setting(RECORD)?.unwrap_or_default();
        let first = self.local_setting(FIRST)?.unwrap_or_default();
        let keys = self.local_setting(KEYS)?.unwrap_or_default();
        let aliases = self.local_setting(ALIASES)?.unwrap_or_default();
        let stamp = format!("{first}\n{keys}\n{aliases}\n{record}");
        if let Some(held) = self.own_keys.lock().as_ref()
            && held.stamp == stamp
        {
            return Ok(held.clone());
        }
        let own = OwnProfile::new(self);
        let profile = Profile::parse(&record);
        let mut ids: HashSet<String> = own.aliases()?.into_iter().collect();
        if let Some(p) = &profile {
            ids.insert(p.id.clone());
        }
        let bound = match &profile {
            Some(p) if is_key(&first) => chain(&p.id, &first, &own.held_keys()?, &[]).0,
            _ => HashSet::new(),
        };
        let made = Arc::new(OwnKeys { stamp, ids, bound });
        *self.own_keys.lock() = Some(made.clone());
        Ok(made)
    }

    /// Whether the Profile id `id` is the own Profile's, or one it
    /// replaced.
    pub fn is_own_person(&self, id: &str) -> Result<bool> {
        Ok(self.own_keys()?.ids.contains(id))
    }

    /// The id `id` stands for now: the Profile kept here that replaced
    /// it, through every alias, else `id` itself.
    pub fn resolve_person(&self, id: &str) -> Result<String> {
        let index = self.people_index()?;
        let mut at = id.to_string();
        for _ in 0..=index.alias_to.len() {
            match index.alias_to.get(&at) {
                Some(next) => at = next.clone(),
                None => break,
            }
        }
        Ok(at)
    }

    /// Whether the key `key` (base64) is bound to the own Profile.
    pub(crate) fn own_bound(&self, key: &str) -> Result<bool> {
        Ok(self.own_keys()?.bound.contains(key))
    }

    /// The person `id` as every screen shows them: the own Profile for
    /// one's own ids, a Profile kept here through its aliases with its
    /// picture made again, else the placeholder with no name, until their
    /// Profile arrives; with the name and picture of the user's Override,
    /// where they gave one (#52).
    pub fn person_shown(&self, id: &str) -> Result<Profile> {
        let theirs = self.person_theirs(id)?;
        if self.is_own_person(id)? {
            return Ok(theirs);
        }
        self.with_override(theirs)
    }

    /// The person `id` with their own name and picture, as
    /// [`Store::person_shown`] but without the user's Override: what
    /// their Profile view shows them as.
    pub fn person_theirs(&self, id: &str) -> Result<Profile> {
        if self.is_own_person(id)?
            && let Some(own) = OwnProfile::new(self).get()?
        {
            return Ok(own);
        }
        let resolved = self.resolve_person(id)?;
        Ok(match self.people_index()?.by_id.get(&resolved) {
            Some(held) => held.profile.clone(),
            None => {
                let name = self.name_when_blocked(&resolved)?;
                let mut shown = placeholder(resolved);
                shown.name = name.unwrap_or_default();
                shown
            }
        })
    }

    /// Whether the Profile [`Store::person_theirs`] shows for `id` is of
    /// a structure this version does not know
    /// ([`Profile::record_is_newer`]): shown as far as it can be, a newer
    /// version of the app is needed for the rest (ADR 0034).
    pub fn person_is_newer(&self, id: &str) -> Result<bool> {
        if self.is_own_person(id)? && OwnProfile::new(self).get()?.is_some() {
            return OwnProfile::new(self).is_newer();
        }
        let resolved = self.resolve_person(id)?;
        Ok(self
            .people_index()?
            .by_id
            .get(&resolved)
            .is_some_and(|held| Profile::record_is_newer(&held.sent.record)))
    }

    /// The short code of the person `id`'s Profile, when it is kept here
    /// or is the own one.
    pub fn person_code(&self, id: &str) -> Result<Option<String>> {
        if self.is_own_person(id)? {
            return OwnProfile::new(self).code();
        }
        let resolved = self.resolve_person(id)?;
        Ok(self.people_index()?.by_id.get(&resolved).and_then(|held| {
            BASE64
                .decode(&held.sent.first)
                .ok()
                .map(|first| short_code(&held.profile.id, &first))
        }))
    }

    /// The Devices pinned here whose keys the Profile of the person `id`
    /// kept here binds, sorted: those a Sent Item to them is sealed for
    /// when no Shared Area names their Devices, as for a person added
    /// from a Profile card (#107).
    pub fn pinned_devices_of(&self, id: &str) -> Result<Vec<String>> {
        let resolved = self.resolve_person(id)?;
        let index = self.people_index()?;
        let Some(held) = index.by_id.get(&resolved) else {
            return Ok(Vec::new());
        };
        let mut out: Vec<String> = self
            .pinned_keys()?
            .into_iter()
            .filter(|k| held.bound.contains(&BASE64.encode(&k.record.public_key)))
            .map(|k| k.record.device)
            .collect();
        out.sort();
        Ok(out)
    }

    /// Every Profile of another person kept here, by id.
    pub fn kept_people(&self) -> Result<Vec<Profile>> {
        Ok(self
            .people_index()?
            .by_id
            .values()
            .map(|h| h.profile.clone())
            .collect())
    }

    /// The Profile `json` that the Device `device` sent of its person, to
    /// show before an invite: its signatures hold, its binding holds the
    /// key `device` is named for, and its picture is made again. None
    /// when any of it fails.
    pub fn sent_profile_of(&self, json: Option<&Value>, device: &str) -> Option<ProfileChoice> {
        let sent = SignedProfile::from_json(json?)?;
        let checked = check(&sent)?;
        let p = &checked.profile;
        let bound = chain(&p.id, &sent.first, &[], &sent.keys).0;
        let holds = bound.iter().any(|k| {
            BASE64
                .decode(k)
                .is_ok_and(|bytes| device_id_from_key(&bytes) == device)
        });
        if !holds {
            return None;
        }
        let first = BASE64.decode(&sent.first).ok()?;
        Some(ProfileChoice {
            profile: Profile {
                id: p.id.clone(),
                name: p.name.clone(),
                picture: checked.shown.clone(),
                version: p.version,
            },
            code: short_code(&p.id, &first),
        })
    }

    /// Takes in every Profile of `list` (signed Profiles as JSON) that
    /// `allowed` lets in by id, or by an id it replaced; all of them
    /// without `allowed`. Returns whether any was added or changed.
    pub fn take_people(
        &self,
        list: Option<&Value>,
        allowed: Option<&HashSet<String>>,
    ) -> Result<bool> {
        let mut changed = false;
        for json in list.and_then(Value::as_array).into_iter().flatten() {
            changed |= self.take_person(json, allowed)?.changed();
        }
        Ok(changed)
    }

    /// Takes in another person's Profile `json`, as the Dart core's
    /// `takePerson`: kept on first sight, bound to the keys it came
    /// with; later a newer record replaces the one kept only when a key
    /// bound to it signed it, keys join only when a bound key signed
    /// them, and aliases only when a bound key made them. Its name is
    /// sanitised and its picture made again, or it is refused. The own
    /// Profile is left to [`OwnProfile::take`]; a blocked person's is
    /// refused (ADR 0021).
    pub fn take_person(
        &self,
        json: &Value,
        allowed: Option<&HashSet<String>>,
    ) -> Result<PersonTaken> {
        let Some(theirs) = SignedProfile::from_json(json) else {
            return Ok(PersonTaken::Refused);
        };
        let Some(checked) = check(&theirs) else {
            return Ok(PersonTaken::Refused);
        };
        let id = checked.profile.id.clone();
        if self.is_own_person(&id)? {
            return Ok(PersonTaken::Unchanged);
        }
        if self.is_blocked(&id)? {
            return Ok(PersonTaken::Refused);
        }
        let index = self.people_index()?;
        if let Some(allowed) = allowed
            && !allowed.contains(&id)
            && !theirs
                .aliases
                .iter()
                .any(|a| allowed.contains(&a.alias) && replaced(&index.by_id, a) && a.signed())
        {
            return Ok(PersonTaken::Refused);
        }
        let Some(held) = index.by_id.get(&id) else {
            let bound = chain(&id, &theirs.first, &[], &theirs.keys).0;
            let aliases = sorted_aliases(
                theirs
                    .aliases
                    .iter()
                    .filter(|a| a.alias != id && bound.contains(&a.by) && a.signed())
                    .cloned()
                    .collect(),
            );
            self.keep_person(
                &SignedProfile {
                    keys: checked.keys.clone(),
                    aliases,
                    fresh: false,
                    ..theirs.clone()
                },
                checked.shown.as_deref(),
                &dart_iso(self.now()),
                false,
                false,
            )?;
            return Ok(PersonTaken::Added);
        };
        // A copied id with another first key is somebody else.
        if theirs.first != held.sent.first {
            return Ok(PersonTaken::Refused);
        }
        let (bound, keys) = chain(&id, &held.sent.first, &held.sent.keys, &theirs.keys);
        if !bound.contains(&theirs.signer) {
            return Ok(PersonTaken::Refused);
        }
        let mut aliases: BTreeMap<(String, String), ProfileAlias> = held
            .sent
            .aliases
            .iter()
            .map(|a| ((a.alias.clone(), a.by.clone()), a.clone()))
            .collect();
        for a in &theirs.aliases {
            let at = (a.alias.clone(), a.by.clone());
            if a.alias == id || aliases.contains_key(&at) {
                continue;
            }
            if bound.contains(&a.by) && a.signed() {
                aliases.insert(at, a.clone());
            }
        }
        // A record whose picture a block took out gives way to theirs.
        let newer = match Profile::parse(&held.sent.record) {
            None => true,
            Some(kept) if held.stripped => checked.profile.version >= kept.version,
            Some(kept) => replaces(&theirs.record, &checked.profile, &held.sent.record, &kept),
        };
        let from = if newer { &theirs } else { &held.sent };
        let next = SignedProfile {
            record: from.record.clone(),
            signer: from.signer.clone(),
            sig: from.sig.clone(),
            first: held.sent.first.clone(),
            keys,
            aliases: sorted_aliases(aliases.into_values().collect()),
            fresh: false,
        };
        if !(held.stripped && newer) && next.to_json() == held.sent.to_json() {
            return Ok(PersonTaken::Unchanged);
        }
        let shown = if newer {
            checked.shown.clone()
        } else {
            held.profile.picture.clone()
        };
        let seen = held.seen.clone();
        let stripped = held.stripped && !newer;
        self.keep_person(&next, shown.as_deref(), &seen, held.verified, stripped)?;
        Ok(PersonTaken::Updated)
    }

    fn keep_person(
        &self,
        sent: &SignedProfile,
        shown: Option<&[u8]>,
        seen: &str,
        verified: bool,
        stripped: bool,
    ) -> Result<()> {
        let Some(id) = Profile::parse(&sent.record).map(|p| p.id) else {
            return Ok(());
        };
        let mut value = serde_json::Map::new();
        value.insert("profile".into(), sent.to_json());
        value.insert("seen".into(), json!(seen));
        if let Some(shown) = shown {
            value.insert("shown".into(), json!(BASE64.encode(shown)));
        }
        if stripped {
            value.insert("stripped".into(), json!(true));
        }
        if verified {
            value.insert("verified".into(), json!(true));
        }
        self.set_local_setting(&format!("{PREFIX}{id}"), &Value::Object(value).to_string())?;
        *self.people.lock() = None;
        Ok(())
    }

    /// Takes the picture out of the Profile of the blocked person `id`
    /// kept here (ADR 0021); returns whether there was one.
    pub(crate) fn strip_person_picture(&self, id: &str) -> Result<bool> {
        let index = self.people_index()?;
        let Some(held) = index.by_id.get(id) else {
            return Ok(false);
        };
        let Some(record) = Profile::parse(&held.sent.record) else {
            return Ok(false);
        };
        if record.picture.is_none() && held.profile.picture.is_none() {
            return Ok(false);
        }
        let bare = Profile {
            picture: None,
            ..record
        };
        let sent = SignedProfile {
            record: bare.encode(),
            fresh: false,
            ..held.sent.clone()
        };
        self.keep_person(&sent, None, &held.seen, held.verified, true)?;
        Ok(true)
    }

    /// Whether the user compared the short code of the person `id`'s
    /// Profile in person; the own Profile counts as verified.
    pub fn person_verified(&self, id: &str) -> Result<bool> {
        if self.is_own_person(id)? {
            return Ok(true);
        }
        let resolved = self.resolve_person(id)?;
        Ok(self
            .people_index()?
            .by_id
            .get(&resolved)
            .is_some_and(|h| h.verified))
    }

    /// Marks the Profile of the person `id` kept here as verified, when
    /// its short code is `code`, the one the user compared in person: a
    /// copied id with another first key has another code and stays as it
    /// is. Returns whether it is verified now.
    pub fn verify_person(&self, id: &str, code: &str) -> Result<bool> {
        if self.is_own_person(id)? {
            return Ok(true);
        }
        let resolved = self.resolve_person(id)?;
        let index = self.people_index()?;
        let Some(held) = index.by_id.get(&resolved) else {
            return Ok(false);
        };
        if self.person_code(&held.profile.id)?.as_deref() != Some(code) {
            return Ok(false);
        }
        if !held.verified {
            self.keep_person(
                &held.sent,
                held.profile.picture.as_deref(),
                &held.seen,
                true,
                held.stripped,
            )?;
        }
        Ok(true)
    }

    /// The Profile `json` a Sent Item carries, signed with `key`, as its
    /// preview shows it (#51): its signatures hold and its binding holds
    /// `key`, its name sanitised and its picture made again, with its
    /// short code; whether it is kept here already, bound to `key`, and
    /// verified. None when any of it fails: the file is then no Sent
    /// Item.
    pub fn sent_item_profile(&self, json: &Value, key: &[u8]) -> Result<Option<SentProfile>> {
        let Some(sent) = SignedProfile::from_json(json) else {
            return Ok(None);
        };
        let Some(checked) = check(&sent) else {
            return Ok(None);
        };
        let p = &checked.profile;
        let signer = BASE64.encode(key);
        if !chain(&p.id, &sent.first, &[], &sent.keys)
            .0
            .contains(&signer)
        {
            return Ok(None);
        }
        let Ok(first) = BASE64.decode(&sent.first) else {
            return Ok(None);
        };
        let code = short_code(&p.id, &first);
        let (mut kept, mut verified) = (false, false);
        if self.is_own_person(&p.id)? {
            kept = OwnProfile::new(self).code()?.as_deref() == Some(code.as_str());
            verified = kept;
        } else if let Some(held) = self.people_index()?.by_id.get(&p.id)
            && held.sent.first == sent.first
        {
            kept = chain(&p.id, &held.sent.first, &held.sent.keys, &sent.keys)
                .0
                .contains(&signer);
            verified = kept && held.verified;
        }
        let shown = Profile {
            id: p.id.clone(),
            name: p.name.clone(),
            picture: checked.shown.clone(),
            version: p.version,
        };
        // The user's Override shows only for the Profile kept here, bound
        // to the key that signed: a copied id gets none.
        let profile = if kept && !self.is_own_person(&p.id)? {
            self.with_override(shown)?
        } else {
            shown
        };
        Ok(Some(SentProfile {
            profile,
            code,
            kept,
            verified,
        }))
    }

    /// The signed Profiles that go out with a sync, as JSON, sorted by id.
    /// To one of the user's own Devices (`areas` none) every Profile of
    /// another person kept here; to another person's Device, or sealed
    /// under an Area's key, the own Profile and the Profiles of the
    /// people the grants of `areas` name now. A blocked person's Profile
    /// goes nowhere (ADR 0021).
    pub fn profiles_to_send(&self, areas: Option<&HashSet<String>>) -> Result<Vec<Value>> {
        let index = self.people_index()?;
        let blocked = self.blocked_ids()?;
        let passes = |held: &Held| !held.stripped && !blocked.contains(&held.profile.id);
        let mut out: BTreeMap<String, Value> = BTreeMap::new();
        match areas {
            None => {
                for (id, held) in &index.by_id {
                    if passes(held) {
                        out.insert(id.clone(), held.sent.to_json());
                    }
                }
            }
            Some(areas) => {
                if let Some(own) = OwnProfile::new(self).signed()?
                    && let Some(p) = Profile::parse(&own.record)
                {
                    out.insert(p.id, for_others(&own).to_json());
                }
                let mut sorted: Vec<&String> = areas.iter().collect();
                sorted.sort();
                for area in sorted {
                    let named = match self.grants_of(area)? {
                        Some(grants) => grants.profiles(),
                        None => Vec::new(),
                    };
                    for named in named {
                        if self.is_own_person(&named)? {
                            continue;
                        }
                        if let Some(held) = index.by_id.get(&self.resolve_person(&named)?)
                            && passes(held)
                        {
                            out.insert(held.profile.id.clone(), held.sent.to_json());
                        }
                    }
                }
            }
        }
        Ok(out.into_values().collect())
    }

    /// The own Profile as it goes to another person: signed, with its
    /// binding and aliases, without what only one's own Devices are
    /// told.
    pub fn own_profile_for_others(&self) -> Result<Option<Value>> {
        Ok(OwnProfile::new(self)
            .signed()?
            .map(|own| for_others(&own).to_json()))
    }

    /// Which Profile ids may come from the Device `peer` of another
    /// person: those the grants of an Area shared with it name.
    pub fn people_from(&self, peer: &str) -> Result<HashSet<String>> {
        let mut named = HashSet::new();
        for area in self.areas_shared_with(peer)? {
            if let Some(grants) = self.grants_of(&area)? {
                named.extend(grants.profiles());
            }
        }
        Ok(named)
    }

    /// Which Profile ids may come sealed under the key of the Area
    /// `area`: those its grants name.
    pub fn people_of(&self, area: &str) -> Result<HashSet<String>> {
        Ok(match self.grants_of(area)? {
            Some(grants) => grants.profiles().into_iter().collect(),
            None => HashSet::new(),
        })
    }

    /// The person who wrote the Note or Section `entity`, as their
    /// Profile shows, as the Dart core's `authorOf`: through the key of
    /// the Device that wrote its first `$type`, the Profile bound to it,
    /// through its aliases. In a Shared Area the grant of that Device
    /// names whose Profile it must be; until that Profile arrives, its
    /// placeholder. None for the user's own and for a Note whose writer
    /// is not known. A Note accepted from a Sent Item shows its sender,
    /// through the key and the Profile id it keeps (#51).
    pub fn author_of(&self, entity: &str) -> Result<Option<Profile>> {
        match self.writer_of(entity)? {
            Writer::Theirs(author) => Ok(Some(self.with_override(author)?)),
            Writer::Own | Writer::Unknown => Ok(None),
        }
    }

    /// The person who wrote `tick` on a box in the text of `entity`, as
    /// their Profile shows, as the Dart core's `tickedBy` and as for
    /// [`Store::author_of`]: through the key of its Device, in a Shared
    /// Area the person its grant names. None for the user's own
    /// (`tick.own`), for a writer not known, and for a blocked person,
    /// whose ticks stay without their name or picture (ADR 0021).
    pub fn ticked_by(&self, entity: &str, tick: &BoxTick) -> Result<Option<Profile>> {
        if tick.own {
            return Ok(None);
        }
        match self.device_writer(&tick.device, entity)? {
            Writer::Theirs(author) => {
                if self
                    .blocked_ids()?
                    .contains(&self.resolve_person(&author.id)?)
                {
                    return Ok(None);
                }
                Ok(Some(self.with_override(author)?))
            }
            Writer::Own | Writer::Unknown => Ok(None),
        }
    }

    /// The person the Device `device` wrote an edit of the text of the
    /// Note or Section `entity` for, as their Profile shows, as the Dart
    /// core's `editorOf` (#183): the own Profile for the user's own
    /// Devices; another person's as for [`Store::ticked_by`], through the
    /// key of the Device, in a Shared Area the person its grant names.
    /// None for a writer not known, for a blocked person, and for the
    /// user's own while there is no own Profile yet.
    pub fn editor_of(&self, entity: &str, device: &str) -> Result<Option<Profile>> {
        match self.device_writer(device, entity)? {
            Writer::Own => OwnProfile::new(self).get(),
            Writer::Theirs(author) => {
                if self
                    .blocked_ids()?
                    .contains(&self.resolve_person(&author.id)?)
                {
                    return Ok(None);
                }
                Ok(Some(self.with_override(author)?))
            }
            Writer::Unknown => Ok(None),
        }
    }

    /// The person who added the file `hash` to the Note or Section
    /// `entity`, as their Profile shows, as the Dart core's `fileAddedBy`:
    /// the first of another person's Devices that wrote its `file:` field,
    /// through the key of the Device, in a Shared Area the person its
    /// grant names, as for [`Store::ticked_by`]. None for a file the
    /// user's own Devices added, for a writer not known, and for a blocked
    /// person (ADR 0021).
    pub fn file_added_by(&self, entity: &str, hash: &str) -> Result<Option<Profile>> {
        let field = crate::notes::keys::file(hash);
        for e in self.history(entity)? {
            if e.field != field {
                continue;
            }
            if let Writer::Theirs(author) = self.device_writer(&e.device, entity)? {
                if self
                    .blocked_ids()?
                    .contains(&self.resolve_person(&author.id)?)
                {
                    return Ok(None);
                }
                return Ok(Some(self.with_override(author)?));
            }
        }
        Ok(None)
    }

    /// Whether the person filter of the browser keeps the Note or
    /// Section `entity` (#53), as the Dart core's `wroteBy`: with
    /// `filter.me`, the user's own, written on any of their Devices or
    /// under any of their Profile's ids; with its people, those of each
    /// of them, matched through the aliases from either side. A Note
    /// whose writer is not known is nobody's; with an empty filter every
    /// Note is kept.
    pub fn wrote_by(&self, entity: &str, filter: &PersonFilter) -> Result<bool> {
        if filter.is_everyone() {
            return Ok(true);
        }
        match self.writer_of(entity)? {
            Writer::Own => {
                if filter.me {
                    return Ok(true);
                }
                for id in &filter.people {
                    if self.is_own_person(id)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Writer::Theirs(author) => {
                let writer = self.resolve_person(&author.id)?;
                for id in &filter.people {
                    if !self.is_own_person(id)? && self.resolve_person(id)? == writer {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Writer::Unknown => Ok(false),
        }
    }

    /// The Notes of `notes` that `filter` keeps, in their order
    /// ([`Store::wrote_by`]).
    pub fn notes_by(&self, notes: Vec<Note>, filter: &PersonFilter) -> Result<Vec<Note>> {
        let mut kept = Vec::with_capacity(notes.len());
        for note in notes {
            if self.wrote_by(&note.id, filter)? {
                kept.push(note);
            }
        }
        Ok(kept)
    }

    /// Who wrote the Note or Section `entity`: the user, another person
    /// as their Profile shows without the Override, or nobody known.
    fn writer_of(&self, entity: &str) -> Result<Writer> {
        if let Some((key, person)) = self.received_from(entity)? {
            if self.own_keys()?.bound.contains(&key) {
                return Ok(Writer::Own);
            }
            // The id counts only when a Profile kept under it binds the
            // key: a Sent Item carrying one's own id, or a third
            // person's, is shown by its key alone.
            let target = self.resolve_person(&person)?;
            let mut kept = false;
            for held in self.people_index()?.by_id.values() {
                if held.bound.contains(&key) && self.resolve_person(&held.profile.id)? == target {
                    kept = true;
                    break;
                }
            }
            let named = kept.then_some(person);
            return Ok(Writer::theirs(self.shown_for(Some(&key), named)?));
        }
        let Some(device) = self
            .history(entity)?
            .into_iter()
            .find(|e| e.field == "$type")
            .map(|e| e.device)
        else {
            return Ok(Writer::Unknown);
        };
        self.device_writer(&device, entity)
    }

    /// Who the Device `device` writes for in the place of `entity`: the
    /// user, another person as their Profile shows without the Override,
    /// or nobody known.
    fn device_writer(&self, device: &str, entity: &str) -> Result<Writer> {
        if device == self.device_id()? || self.is_own_device(device)? {
            return Ok(Writer::Own);
        }
        let key = self
            .pinned_key(device)?
            .map(|k| BASE64.encode(&k.record.public_key));
        if let Some(key) = &key
            && self.own_keys()?.bound.contains(key)
        {
            return Ok(Writer::Own);
        }
        let named = match self.area_of(entity)? {
            Some(area) => self
                .grants_of(&area)?
                .and_then(|g| g.person_of(device).map(str::to_string)),
            None => None,
        };
        if let Some(named) = &named
            && self.is_own_person(named)?
        {
            return Ok(Writer::Own);
        }
        Ok(Writer::theirs(self.shown_for(key.as_deref(), named)?))
    }

    /// The sender's key and Profile id the Note `entity` keeps from the
    /// Sent Item it was accepted from; none for any other. Only a link
    /// one of the user's own Devices wrote counts: another person's
    /// Device in a Shared Area could write any.
    fn received_from(&self, entity: &str) -> Result<Option<(String, String)>> {
        let Some(raw) = self.own_current(entity, "received", &mut Default::default())? else {
            return Ok(None);
        };
        let Ok(link) = serde_json::from_str::<Value>(&raw) else {
            return Ok(None);
        };
        Ok(
            match (
                link.get("from").and_then(Value::as_str),
                link.get("profile").and_then(Value::as_str),
            ) {
                (Some(key), Some(person)) => Some((key.to_string(), person.to_string())),
                _ => None,
            },
        )
    }

    /// The Profile shown for a writer with the key `key`: the one `named`
    /// stands for among those bound to it, or its placeholder; without
    /// `named`, the Profile first seen with the key.
    fn shown_for(&self, key: Option<&str>, named: Option<String>) -> Result<Option<Profile>> {
        let index = self.people_index()?;
        let mut candidates: Vec<&Held> = match key {
            Some(key) => index
                .by_id
                .values()
                .filter(|h| h.bound.contains(key))
                .collect(),
            None => Vec::new(),
        };
        candidates.sort_by(|a, b| {
            a.seen
                .cmp(&b.seen)
                .then_with(|| a.profile.id.cmp(&b.profile.id))
        });
        let Some(named) = named else {
            return Ok(match candidates.first() {
                None => None,
                Some(first) => {
                    let resolved = self.resolve_person(&first.profile.id)?;
                    Some(index.by_id.get(&resolved).unwrap_or(first).profile.clone())
                }
            });
        };
        let target = self.resolve_person(&named)?;
        for held in candidates {
            if self.resolve_person(&held.profile.id)? == target {
                return Ok(Some(
                    index.by_id.get(&target).unwrap_or(held).profile.clone(),
                ));
            }
        }
        Ok(Some(placeholder(target)))
    }
}

/// Whether `a` gives up a Profile kept in `by_id`, made by a key bound to
/// it.
fn replaced(by_id: &BTreeMap<String, Held>, a: &ProfileAlias) -> bool {
    by_id
        .get(&a.alias)
        .is_some_and(|held| held.bound.contains(&a.by))
}

fn placeholder(id: String) -> Profile {
    Profile {
        id,
        name: String::new(),
        picture: None,
        version: 1,
    }
}

fn sorted_aliases(mut aliases: Vec<ProfileAlias>) -> Vec<ProfileAlias> {
    aliases.sort_by(|a, b| a.alias.cmp(&b.alias).then_with(|| a.by.cmp(&b.by)));
    aliases
}

#[cfg(test)]
mod tests;
