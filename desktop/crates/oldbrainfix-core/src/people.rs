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
        let mut value = json!({"profile": sent.to_json(), "seen": seen});
        if let Some(shown) = shown {
            value["shown"] = json!(BASE64.encode(shown));
        }
        if stripped {
            value["stripped"] = json!(true);
        }
        if verified {
            value["verified"] = json!(true);
        }
        self.set_local_setting(&format!("{PREFIX}{id}"), &value.to_string())?;
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
        if device == self.device_id()? || self.is_own_device(&device)? {
            return Ok(Writer::Own);
        }
        let key = self
            .pinned_key(&device)?
            .map(|k| BASE64.encode(&k.record.public_key));
        if let Some(key) = &key
            && self.own_keys()?.bound.contains(key)
        {
            return Ok(Writer::Own);
        }
        let named = match self.area_of(entity)? {
            Some(area) => self
                .grants_of(&area)?
                .and_then(|g| g.person_of(&device).map(str::to_string)),
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
mod tests {
    use super::*;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer_as, profile_answer, sync_request,
        sync_request_as,
    };
    use crate::profile::{ProfileKey, key_bytes, record_bytes};
    use crate::sent::SentItemState;
    use crate::shared::{AreaRole, Invite};
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use crate::{Notes, PairedDevice, Places};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    /// A Device with its own Profile, named by the user.
    fn device(seed: u8, name: &str) -> Result<Store> {
        let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
        let own = OwnProfile::new(&store);
        own.apply_defaults("Device", None)?;
        own.rename(name)?;
        Ok(store)
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

    /// Ann's Household shared with Ben as a Member: Ann's desk invites
    /// Ben's phone, then they sync once more as paired.
    fn household(
        ann: &Store,
        ben: &Store,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let house = Places::new(ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        let invite = Invite {
            area: area.clone(),
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            ben,
            &ann.version_vector()?,
            "Phone",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        let (answer, session) =
            answer_sync_inviting(ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
        assert!(session.people_changed);
        apply_answer_as(ben, &answer, true, None)?;
        remember(ann, ben, true)?;
        remember(ben, ann, true)?;
        Ok(area)
    }

    fn own_id(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
        Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
    }

    #[test]
    fn an_invite_carries_both_profiles() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let area = household(&ann, &ben)?;
        assert_eq!(ann.person_shown(&own_id(&ben)?)?.name, "Ben");
        assert_eq!(ben.person_shown(&own_id(&ann)?)?.name, "Ann");
        assert_eq!(
            ben.person_code(&own_id(&ann)?)?,
            OwnProfile::new(&ann).code()?
        );
        let mut names = Vec::new();
        for p in ann.people(&area)? {
            names.push(ann.person_shown(&p.id)?.name);
        }
        assert_eq!(names, ["Ann", "Ben"]);
        // A renamed Profile reaches the other with the next sync.
        OwnProfile::new(&ben).rename("Benjamin")?;
        let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
        let (_, session) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        assert!(session.people_changed);
        assert_eq!(ann.person_shown(&own_id(&ben)?)?.name, "Benjamin");
        Ok(())
    }

    #[test]
    fn an_update_signed_by_a_key_not_bound_is_refused() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        household(&ann, &ben)?;
        let sent = OwnProfile::new(&ben).signed()?.ok_or("a Profile")?;
        let id = own_id(&ben)?;
        let mallory = SigningKey::from_seed([9; 32]);
        let mallory_key = BASE64.encode(mallory.public_key());
        let record = Profile {
            id: id.clone(),
            name: "Mallory".into(),
            picture: None,
            version: 9,
        }
        .encode();
        let mut keys = sent.keys.clone();
        keys.push(ProfileKey {
            key: mallory_key.clone(),
            by: mallory_key.clone(),
            sig: mallory.sign(&key_bytes(&id, &mallory_key)),
        });
        let forged = SignedProfile {
            sig: mallory.sign(&record_bytes(&record)),
            record,
            signer: mallory_key,
            first: sent.first.clone(),
            keys,
            aliases: Vec::new(),
            fresh: false,
        };
        assert_eq!(
            ann.take_person(&forged.to_json(), None)?,
            PersonTaken::Refused
        );
        assert_eq!(ann.person_shown(&id)?.name, "Ben");
        OwnProfile::new(&ben).rename("Ben B.")?;
        let update = ben.own_profile_for_others()?.ok_or("a Profile")?;
        assert_eq!(ann.take_person(&update, None)?, PersonTaken::Updated);
        assert_eq!(ann.person_shown(&id)?.name, "Ben B.");
        Ok(())
    }

    #[test]
    fn a_received_name_is_sanitised_again() -> TestResult {
        let ann = device(1, "Ann")?;
        let key = SigningKey::from_seed([5; 32]);
        let record = r#"{"id":"cat","name":" \u202eCat\u200b\u0007 Lee ","version":1}"#;
        let sent = SignedProfile {
            record: record.into(),
            signer: BASE64.encode(key.public_key()),
            sig: key.sign(&record_bytes(record)),
            first: BASE64.encode(key.public_key()),
            keys: Vec::new(),
            aliases: Vec::new(),
            fresh: false,
        };
        assert_eq!(ann.take_person(&sent.to_json(), None)?, PersonTaken::Added);
        assert_eq!(ann.person_shown("cat")?.name, "Cat Lee");
        Ok(())
    }

    #[test]
    fn notes_are_marked_by_the_profile_the_grant_names() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let area = household(&ann, &ben)?;
        let mine = Notes::new(&ann).capture("Milk", Some(&area), None)?;
        let theirs = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
        let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
        let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        apply_answer_as(&ben, &answer, true, None)?;
        assert_eq!(ann.author_of(&mine.id)?, None);
        assert_eq!(
            ann.author_of(&theirs.id)?.map(|p| p.name),
            Some("Ben".into())
        );
        assert_eq!(ben.author_of(&theirs.id)?, None);
        assert_eq!(ben.author_of(&mine.id)?.map(|p| p.name), Some("Ann".into()));
        // A Profile that binds Ben's key into its own marks none of his
        // Notes.
        let mallory = SigningKey::from_seed([9; 32]);
        let mallory_key = BASE64.encode(mallory.public_key());
        let ben_key = BASE64.encode(ben.signing_key()?.public_key());
        let id = "0".repeat(32);
        let record = Profile {
            id: id.clone(),
            name: "Mallory".into(),
            picture: None,
            version: 1,
        }
        .encode();
        let claim = SignedProfile {
            sig: mallory.sign(&record_bytes(&record)),
            record,
            signer: mallory_key.clone(),
            first: mallory_key.clone(),
            keys: vec![ProfileKey {
                key: ben_key.clone(),
                by: mallory_key,
                sig: mallory.sign(&key_bytes(&id, &ben_key)),
            }],
            aliases: Vec::new(),
            fresh: false,
        };
        assert_eq!(ann.take_person(&claim.to_json(), None)?, PersonTaken::Added);
        assert_eq!(
            ann.author_of(&theirs.id)?.map(|p| p.name),
            Some("Ben".into())
        );
        Ok(())
    }

    fn ids(notes: &[Note]) -> Vec<String> {
        notes.iter().map(|n| n.id.clone()).collect()
    }

    fn me() -> PersonFilter {
        PersonFilter {
            me: true,
            people: BTreeSet::new(),
        }
    }

    #[test]
    fn the_person_filter_keeps_me_them_or_both_with_the_search() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let area = household(&ann, &ben)?;
        let notes = Notes::new(&ann);
        let mine = notes.capture("Milk and eggs", Some(&area), None)?;
        let unsorted = notes.capture("Eggs for the cake", None, None)?;
        let theirs = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
        let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
        let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        apply_answer_as(&ben, &answer, true, None)?;
        let all = notes.all()?;
        let ben_only = PersonFilter::person(&own_id(&ben)?);
        assert_eq!(
            ids(&ann.notes_by(all.clone(), &PersonFilter::default())?),
            ids(&all)
        );
        assert_eq!(
            ids(&ann.notes_by(all.clone(), &me())?),
            vec![unsorted.id.clone(), mine.id.clone()]
        );
        assert_eq!(
            ids(&ann.notes_by(all.clone(), &ben_only)?),
            vec![theirs.id.clone()]
        );
        let mut both = me();
        both.toggle(&own_id(&ben)?);
        assert_eq!(ids(&ann.notes_by(all.clone(), &both)?), ids(&all));
        // The own Profile's id among the people counts as "Me".
        assert_eq!(
            ids(&ann.notes_by(all.clone(), &PersonFilter::person(&own_id(&ann)?))?),
            vec![unsorted.id.clone(), mine.id.clone()]
        );
        // Ben sees it the other way round.
        let on_ben = Notes::new(&ben).all()?;
        assert_eq!(
            ids(&ben.notes_by(on_ben.clone(), &me())?),
            vec![theirs.id.clone()]
        );
        assert_eq!(
            ids(&ben.notes_by(on_ben, &PersonFilter::person(&own_id(&ann)?))?),
            vec![mine.id.clone()]
        );
        // With the search over the Area.
        let found = crate::Search::new(&ann).notes("eggs", Some(&area))?;
        assert_eq!(found.len(), 2);
        assert_eq!(ids(&ann.notes_by(found.clone(), &me())?), vec![mine.id]);
        assert_eq!(ids(&ann.notes_by(found, &ben_only)?), vec![theirs.id]);
        Ok(())
    }

    #[test]
    fn a_note_whose_writer_is_not_known_is_nobodys() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let area = household(&ann, &ben)?;
        let theirs = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
        let moved = Notes::new(&ben).capture("Flour", Some(&area), None)?;
        let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
        answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        // Ann takes one of Ben's Notes into an Area of her own.
        let baking = Places::new(&ann).add_area("Baking")?;
        Notes::new(&ann).file(&moved.id, &baking.id)?;
        let ben_id = own_id(&ben)?;
        assert_eq!(
            ann.author_of(&moved.id)?.map(|p| p.id),
            Some(ben_id.clone())
        );
        // Another Device of Ann's, met without the Profiles.
        let pad = device(3, "Ann on the pad")?;
        let request = sync_request(&pad, &ann.version_vector()?, "Pad", &ann.device_id()?, "")?;
        let (mut answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        answer
            .as_object_mut()
            .ok_or("an object")?
            .remove("profiles");
        crate::lan::apply_answer(&pad, &answer)?;
        assert!(pad.kept_people()?.is_empty());
        let ben_only = PersonFilter::person(&ben_id);
        // In the Area its grant names Ben, before his Profile arrives.
        assert!(!pad.wrote_by(&theirs.id, &me())?);
        assert!(pad.wrote_by(&theirs.id, &ben_only)?);
        // Outside it no grant names him and no Profile binds his key here.
        assert_eq!(pad.author_of(&moved.id)?, None);
        assert!(!pad.wrote_by(&moved.id, &me())?);
        assert!(!pad.wrote_by(&moved.id, &ben_only)?);
        // Nothing in the log says who wrote it.
        let nothing = "0".repeat(32);
        assert!(!pad.wrote_by(&nothing, &me())?);
        assert!(!pad.wrote_by(&nothing, &ben_only)?);
        Ok(())
    }

    /// Ann's Household shared with Ben's desk, invited on its own under
    /// its own Profile, which wrote a Note there.
    fn desk_with_a_note(
        ann: &Store,
        desk: &Store,
    ) -> std::result::Result<(String, String), Box<dyn std::error::Error>> {
        let area = household(ann, desk)?;
        let note = Notes::new(desk).capture("Fix the tap", Some(&area), None)?;
        let request = sync_request(desk, &ann.version_vector()?, "Desk", &ann.device_id()?, "")?;
        answer_sync(ann, &request, "Desk")?.ok_or("an answer")?;
        Ok((area, note.id))
    }

    #[test]
    fn a_note_is_marked_through_the_aliases_of_its_writers_profile() -> TestResult {
        let ann = device(1, "Ann")?;
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([4; 32])))?;
        OwnProfile::new(&desk).apply_defaults("Ben at the desk", None)?;
        let desk_id = own_id(&desk)?;
        let (_, note) = desk_with_a_note(&ann, &desk)?;
        assert_eq!(
            ann.author_of(&note)?.map(|p| p.name),
            Some("Ben at the desk".into())
        );
        // Ben pairs it with his phone, whose Profile it takes over.
        let ben = device(2, "Ben")?;
        for _ in 0..2 {
            let request = sync_request(&ben, &desk.version_vector()?, "Phone", "", "")?;
            let (answer, _) = answer_sync(&desk, &request, "Desk")?.ok_or("an answer")?;
            crate::lan::apply_answer(&ben, &answer)?;
        }
        assert_eq!(own_id(&desk)?, own_id(&ben)?);
        let request = sync_request(&desk, &ann.version_vector()?, "Desk", &ann.device_id()?, "")?;
        answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        assert_eq!(ann.resolve_person(&desk_id)?, own_id(&ben)?);
        assert_eq!(ann.author_of(&note)?.map(|p| p.name), Some("Ben".into()));
        // The person filter finds it through the old id and the new.
        assert!(ann.wrote_by(&note, &PersonFilter::person(&desk_id))?);
        assert!(ann.wrote_by(&note, &PersonFilter::person(&own_id(&ben)?))?);
        // On Ben's phone it is his own, under his old id too.
        let request = sync_request(&ben, &desk.version_vector()?, "Phone", "", "")?;
        let (answer, _) = answer_sync(&desk, &request, "Desk")?.ok_or("an answer")?;
        crate::lan::apply_answer(&ben, &answer)?;
        assert!(ben.wrote_by(&note, &me())?);
        assert!(ben.wrote_by(&note, &PersonFilter::person(&desk_id))?);
        Ok(())
    }

    #[test]
    fn a_profile_naming_somebody_elses_id_as_its_alias_takes_nobodys_place() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let (_, note) = desk_with_a_note(&ann, &ben)?;
        let ben_id = own_id(&ben)?;
        let mallory = SigningKey::from_seed([9; 32]);
        let key = BASE64.encode(mallory.public_key());
        let id = "0".repeat(32);
        let record = Profile {
            id: id.clone(),
            name: "Mallory".into(),
            picture: None,
            version: 1,
        }
        .encode();
        let claim = SignedProfile {
            sig: mallory.sign(&record_bytes(&record)),
            record,
            signer: key.clone(),
            first: key.clone(),
            keys: Vec::new(),
            aliases: vec![ProfileAlias {
                alias: ben_id.clone(),
                to: id.clone(),
                by: key,
                sig: mallory.sign(&crate::profile::alias_bytes(&ben_id, &id)),
            }],
            fresh: false,
        }
        .to_json();
        let from_ben = ann.people_from(&ben.device_id()?)?;
        assert_eq!(
            ann.take_person(&claim, Some(&from_ben))?,
            PersonTaken::Refused
        );
        assert_eq!(ann.take_person(&claim, None)?, PersonTaken::Added);
        assert_eq!(ann.resolve_person(&ben_id)?, ben_id);
        assert_eq!(ann.person_shown(&ben_id)?.name, "Ben");
        assert_eq!(ann.author_of(&note)?.map(|p| p.name), Some("Ben".into()));
        Ok(())
    }

    /// Ben's phone syncs with Ann's desk, as people sharing an Area.
    fn sync(ben: &Store, ann: &Store) -> TestResult {
        let request = sync_request(ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
        let (answer, _) = answer_sync(ann, &request, "Desk")?.ok_or("an answer")?;
        apply_answer_as(ben, &answer, true, None)?;
        Ok(())
    }

    /// The Note `from` sends, accepted on `to`.
    fn accepted(
        to: &Store,
        file: &[u8],
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let opened = to.open_sent_item(file)?.map_err(|_| "a Sent Item")?;
        Ok(to.accept_sent_item(&opened.item, false)?.ok_or("a Note")?)
    }

    fn sent_from(from: &Store) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
        let note = Notes::new(from).capture("Hi", None, None)?;
        Ok(from.send_note(&note.id, None, false)?)
    }

    /// A `received` link as accepting a Sent Item writes it.
    fn link(key: &str, person: &str) -> String {
        json!({"from": key, "id": "x", "profile": person, "version": 1}).to_string()
    }

    fn key_of(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
        Ok(BASE64.encode(store.signing_key()?.public_key()))
    }

    #[test]
    fn a_received_link_another_persons_device_wrote_marks_nothing() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        let cids = accepted(&ann, &sent_from(&cid)?)?;
        let cid_id = own_id(&cid)?;
        assert_eq!(ann.author_of(&cids)?.map(|p| p.id), Some(cid_id.clone()));
        // Ben's modified app marks his Notes as Ann's and as Cid's.
        let as_ann = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
        let as_cid = Notes::new(&ben).capture("Flour", Some(&area), None)?;
        let ann_link = link(&key_of(&ann)?, &own_id(&ann)?);
        ben.append(&as_ann.id, "received", Some(&ann_link), None)?;
        ben.append(
            &as_cid.id,
            "received",
            Some(&link(&key_of(&cid)?, &cid_id)),
            None,
        )?;
        sync(&ben, &ann)?;
        let ben_id = own_id(&ben)?;
        assert_eq!(
            ann.author_of(&as_ann.id)?.map(|p| p.id),
            Some(ben_id.clone())
        );
        assert!(!ann.wrote_by(&as_ann.id, &me())?);
        assert_eq!(ann.author_of(&as_cid.id)?.map(|p| p.id), Some(ben_id));
        Ok(())
    }

    #[test]
    fn a_newer_received_link_another_persons_device_wrote_hides_nothing() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        let cids = accepted(&ann, &sent_from(&cid)?)?;
        // A Manager can write on Ann's Note too.
        ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
        sync(&ben, &ann)?;
        Notes::new(&ann).file(&cids, &area)?;
        sync(&ben, &ann)?;
        let forged = link(&key_of(&ann)?, &own_id(&ann)?);
        // Later than anything Ann wrote: each Device has a clock of its own.
        let later = ann.now() + chrono::Duration::hours(1);
        ben.append(&cids, "received", Some(&forged), Some(later))?;
        sync(&ben, &ann)?;
        assert_eq!(ann.current(&cids, "received")?, Some(forged), "it landed");
        assert_eq!(ann.author_of(&cids)?.map(|p| p.id), Some(own_id(&cid)?));
        Ok(())
    }

    type Boxed<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// Cid's Note "Hi" with a Reminder, accepted on `ann`: Cid's Note and
    /// the Note `ann` holds it as.
    fn from_cid(ann: &Store, cid: &Store) -> Boxed<(String, String)> {
        let sent = Notes::new(cid).capture("Hi", None, None)?.id;
        let schedule = crate::Schedule::new(
            crate::Day::parse("2026-03-03").ok_or("day")?,
            &[crate::ClockTime::parse("10:00").ok_or("time")?],
            None,
        )
        .ok_or("schedule")?;
        crate::Routines::new(cid).spawn(&sent, &schedule)?;
        let held = accepted(ann, &cid.send_note(&sent, None, false)?)?;
        Ok((sent, held))
    }

    /// The `received` link `ann` keeps on `held`, with `version`.
    fn relinked(ann: &Store, held: &str, version: i64) -> Boxed<String> {
        let mut link: Value =
            serde_json::from_str(&ann.current(held, "received")?.ok_or("a link")?)?;
        link["version"] = json!(version);
        Ok(link.to_string())
    }

    fn opened(store: &Store, file: &[u8]) -> Boxed<crate::sent::Opened> {
        Ok(store.open_sent_item(file)?.map_err(|_| "a Sent Item")?)
    }

    fn text_of(store: &Store, note: &str) -> Boxed<String> {
        Ok(Notes::new(store).by_id(note)?.ok_or("a Note")?.text)
    }

    fn trashed(store: &Store, note: &str) -> Boxed<bool> {
        Ok(Notes::new(store).by_id(note)?.ok_or("a Note")?.is_trashed())
    }

    #[test]
    fn a_sent_items_link_another_persons_device_wrote_is_no_note_it_reaches() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        let (sent, held) = from_cid(&ann, &cid)?;
        // A Manager can write on Ann's Note too.
        ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
        sync(&ben, &ann)?;
        let rent = Notes::new(&ann)
            .capture("Pay the rent", Some(&area), None)?
            .id;
        sync(&ben, &ann)?;
        // Later than the grant: each Device has a clock of its own.
        let later = ann.now() + chrono::Duration::hours(1);
        ben.append(
            &rent,
            "received",
            Some(&relinked(&ann, &held, 1)?),
            Some(later),
        )?;
        sync(&ben, &ann)?;
        assert!(ann.current(&rent, "received")?.is_some(), "it landed");

        Notes::new(&cid).edit(&sent, "Hi again")?;
        let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
        assert_eq!(update.state, SentItemState::Update);
        assert_eq!(update.accepted.as_deref(), Some(held.as_str()));
        assert_eq!(
            ann.accept_sent_item(&update.item, false)?,
            Some(held.clone())
        );
        assert_eq!(text_of(&ann, &held)?, "Hi again");
        assert_eq!(text_of(&ann, &rent)?, "Pay the rent");

        let cancel = opened(&ann, &cid.send_note(&sent, None, true)?)?;
        assert_eq!(cancel.state, SentItemState::Cancel);
        assert_eq!(
            ann.accept_sent_item(&cancel.item, false)?,
            Some(held.clone())
        );
        assert!(trashed(&ann, &held)?);
        assert!(!trashed(&ann, &rent)?);
        Ok(())
    }

    #[test]
    fn a_members_own_note_linked_to_a_sent_item_never_accepted_is_left_alone() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        let sent = Notes::new(&cid).capture("Hi", None, None)?.id;
        let item = opened(&ann, &cid.send_note(&sent, None, false)?)?.item;
        let eggs = Notes::new(&ben).capture("Eggs", Some(&area), None)?.id;
        let forged = json!({
            "from": key_of(&cid)?,
            "id": item.id,
            "profile": own_id(&cid)?,
            "version": 1,
        });
        ben.append(&eggs, "received", Some(&forged.to_string()), None)?;
        sync(&ben, &ann)?;
        assert!(ann.current(&eggs, "received")?.is_some(), "it landed");

        let cancel = opened(&ann, &cid.send_note(&sent, None, true)?)?;
        assert_eq!(cancel.state, SentItemState::NothingToCancel);
        assert_eq!(ann.accept_sent_item(&cancel.item, false)?, None);
        assert!(!trashed(&ann, &eggs)?);

        let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
        assert_eq!(update.state, SentItemState::Fresh);
        assert_ne!(
            ann.accept_sent_item(&update.item, false)?,
            Some(eggs.clone())
        );
        assert_eq!(text_of(&ann, &eggs)?, "Eggs");
        Ok(())
    }

    #[test]
    fn the_link_ones_own_phone_wrote_carries_updates_applied_on_the_desk() -> TestResult {
        let ann = device(1, "Ann")?;
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([4; 32])))?;
        let cid = device(3, "Cid")?;
        remember(&ann, &desk, false)?;
        remember(&desk, &ann, false)?;
        let (sent, held) = from_cid(&ann, &cid)?;
        own_sync(&desk, &ann)?;
        Notes::new(&cid).edit(&sent, "Hi again")?;
        let update = opened(&desk, &cid.send_note(&sent, None, false)?)?;
        assert_eq!(update.state, SentItemState::Update);
        assert!(update.changes.ok_or("changes")?.routines_added.is_empty());
        assert_eq!(
            desk.accept_sent_item(&update.item, false)?,
            Some(held.clone())
        );
        assert_eq!(text_of(&desk, &held)?, "Hi again");
        own_sync(&desk, &ann)?;

        let cancel = opened(&ann, &cid.send_note(&sent, None, true)?)?;
        assert_eq!(cancel.state, SentItemState::Cancel);
        assert_eq!(
            ann.accept_sent_item(&cancel.item, false)?,
            Some(held.clone())
        );
        assert!(trashed(&ann, &held)?);
        assert!(crate::Routines::new(&ann).all()?.is_empty(), "alarms off");
        Ok(())
    }

    #[test]
    fn a_newer_link_another_persons_device_wrote_hides_no_update() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        let (sent, held) = from_cid(&ann, &cid)?;
        ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
        sync(&ben, &ann)?;
        Notes::new(&ann).file(&held, &area)?;
        sync(&ben, &ann)?;
        let routines = crate::Routines::new(&ann).of_note(&held)?;
        let [routine] = routines.as_slice() else {
            return Err("one Reminder".into());
        };
        // Claims every version is held already, and the Reminder is none
        // of Cid's; later than anything Ann wrote.
        let later = ann.now() + chrono::Duration::hours(1);
        ben.append(
            &held,
            "received",
            Some(&relinked(&ann, &held, 99)?),
            Some(later),
        )?;
        ben.append(&routine.id, "received", Some("routine:forged"), Some(later))?;
        sync(&ben, &ann)?;
        let landed: Value =
            serde_json::from_str(&ann.current(&held, "received")?.ok_or("a link")?)?;
        assert_eq!(landed["version"], json!(99), "it landed");
        assert_eq!(
            ann.current(&routine.id, "received")?.as_deref(),
            Some("routine:forged")
        );

        Notes::new(&cid).edit(&sent, "Hi again")?;
        let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
        assert_eq!(update.state, SentItemState::Update);
        let changes = update.changes.clone().ok_or("changes")?;
        assert!(changes.routines_added.is_empty());
        assert!(changes.routines_removed.is_empty());
        ann.accept_sent_item(&update.item, false)?;
        assert_eq!(text_of(&ann, &held)?, "Hi again");
        assert_eq!(crate::Routines::new(&ann).of_note(&held)?.len(), 1);
        Ok(())
    }

    #[test]
    fn a_text_another_persons_device_wrote_with_a_link_is_not_the_one_sent_last() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        let (sent, held) = from_cid(&ann, &cid)?;
        ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
        sync(&ben, &ann)?;
        Notes::new(&ann).file(&held, &area)?;
        sync(&ben, &ann)?;
        // Both at once, as accepting writes them.
        let later = Some(ann.now() + chrono::Duration::hours(1));
        let text = crate::StoredText::plain("Hi from Ben").encode();
        ben.append(&held, crate::notes::keys::TEXT, Some(&text), later)?;
        ben.append(&held, "received", Some(&relinked(&ann, &held, 1)?), later)?;
        sync(&ben, &ann)?;
        assert_eq!(text_of(&ann, &held)?, "Hi from Ben", "it landed");

        Notes::new(&cid).edit(&sent, "Hi again")?;
        let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
        let changes = update.changes.ok_or("changes")?;
        assert_eq!(changes.text_before.as_deref(), Some("Hi from Ben"));
        assert!(changes.text_edited);
        Ok(())
    }

    /// The link `ann` keeps on a Note it sent.
    fn sent_field(ann: &Store) -> Boxed<String> {
        Ok(crate::sent::keys::sent(&ann.device_id()?))
    }

    /// Ann's Note "Dentist" sent to Cid and accepted there: the Sent Item
    /// and the Note Cid holds it as.
    fn dentist_at(ann: &Store, cid: &Store) -> Boxed<(crate::sent::SentItem, String)> {
        let dentist = Notes::new(ann).capture("Dentist", None, None)?.id;
        let item = opened(cid, &ann.send_note(&dentist, None, false)?)?.item;
        let held = cid.accept_sent_item(&item, false)?.ok_or("a Note")?;
        Ok((item, held))
    }

    #[test]
    fn a_sent_link_another_persons_device_wrote_sends_a_new_sent_item() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
        sync(&ben, &ann)?;
        let (dentist, held) = dentist_at(&ann, &cid)?;
        let rent = Notes::new(&ann)
            .capture("Pay the rent", Some(&area), None)?
            .id;
        sync(&ben, &ann)?;
        let forged = json!({"id": dentist.id, "version": 1}).to_string();
        let later = ann.now() + chrono::Duration::hours(1);
        ben.append(&rent, &sent_field(&ann)?, Some(&forged), Some(later))?;
        sync(&ben, &ann)?;
        assert!(
            ann.current(&rent, &sent_field(&ann)?)?.is_some(),
            "it landed"
        );
        assert!(!ann.was_sent(&rent)?);

        let cancel = opened(&cid, &ann.send_note(&rent, None, true)?)?;
        assert_ne!(cancel.item.id, dentist.id);
        assert_eq!(cancel.state, SentItemState::NothingToCancel);
        assert_eq!(cid.accept_sent_item(&cancel.item, false)?, None);
        assert!(!trashed(&cid, &held)?);

        let again = opened(&cid, &ann.send_note(&rent, None, false)?)?;
        assert_ne!(again.item.id, dentist.id);
        assert_eq!(again.state, SentItemState::Fresh);
        assert_ne!(
            cid.accept_sent_item(&again.item, false)?,
            Some(held.clone())
        );
        assert_eq!(text_of(&cid, &held)?, "Dentist");
        Ok(())
    }

    #[test]
    fn ones_own_sent_link_makes_the_second_send_an_update_through_the_desk() -> TestResult {
        let ann = device(1, "Ann")?;
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([4; 32])))?;
        let cid = device(3, "Cid")?;
        remember(&ann, &desk, false)?;
        remember(&desk, &ann, false)?;
        let hi = Notes::new(&ann).capture("Hi", None, None)?.id;
        let first = opened(&cid, &ann.send_note(&hi, None, false)?)?.item;
        let held = cid.accept_sent_item(&first, false)?.ok_or("a Note")?;
        own_sync(&desk, &ann)?;
        Notes::new(&desk).edit(&hi, "Hi again")?;
        own_sync(&desk, &ann)?;
        assert!(ann.was_sent(&hi)?);

        let update = opened(&cid, &ann.send_note(&hi, None, false)?)?;
        assert_eq!(update.item.id, first.id);
        assert_eq!(update.item.version, 2);
        assert_eq!(update.state, SentItemState::Update);
        assert_eq!(
            cid.accept_sent_item(&update.item, false)?,
            Some(held.clone())
        );
        assert_eq!(text_of(&cid, &held)?, "Hi again");
        // Another Device of Ann's sends under an id of its own (ADR 0015).
        assert!(!desk.was_sent(&hi)?);
        assert_ne!(desk.sent_item_of(&hi, false)?.id, first.id);
        Ok(())
    }

    #[test]
    fn a_newer_sent_link_another_persons_device_wrote_hides_not_ones_own() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let cid = device(3, "Cid")?;
        let area = household(&ann, &ben)?;
        ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
        sync(&ben, &ann)?;
        let (dentist, dentist_held) = dentist_at(&ann, &cid)?;
        let rent = Notes::new(&ann)
            .capture("Pay the rent", Some(&area), None)?
            .id;
        let first = opened(&cid, &ann.send_note(&rent, None, false)?)?.item;
        let held = cid.accept_sent_item(&first, false)?.ok_or("a Note")?;
        sync(&ben, &ann)?;
        let forged = json!({"id": dentist.id, "version": 99}).to_string();
        let later = ann.now() + chrono::Duration::hours(1);
        ben.append(&rent, &sent_field(&ann)?, Some(&forged), Some(later))?;
        sync(&ben, &ann)?;
        let landed: Value =
            serde_json::from_str(&ann.current(&rent, &sent_field(&ann)?)?.ok_or("a link")?)?;
        assert_eq!(landed["version"], json!(99), "it landed");

        let update = opened(&cid, &ann.send_note(&rent, None, false)?)?;
        assert_eq!(update.item.id, first.id);
        assert_eq!(update.item.version, 2);
        assert_eq!(update.state, SentItemState::Update);
        assert_eq!(
            cid.accept_sent_item(&update.item, false)?,
            Some(held.clone())
        );
        assert_eq!(text_of(&cid, &dentist_held)?, "Dentist");
        Ok(())
    }

    #[test]
    fn a_sent_item_carrying_the_receivers_id_or_a_third_persons_shows_its_key() -> TestResult {
        let ben = device(2, "Ben")?;
        let eve = device(5, "Eve")?;
        let cid = device(3, "Cid")?;
        accepted(&ben, &sent_from(&eve)?)?;
        accepted(&ben, &sent_from(&cid)?)?;
        let forged_as =
            |id: &str, name: &str| -> std::result::Result<String, Box<dyn std::error::Error>> {
                let key = eve.signing_key()?;
                let record = Profile {
                    id: id.into(),
                    name: name.into(),
                    picture: None,
                    version: 9,
                }
                .encode();
                let copy = SignedProfile {
                    sig: key.sign(&record_bytes(&record)),
                    record,
                    signer: key_of(&eve)?,
                    first: key_of(&eve)?,
                    keys: Vec::new(),
                    aliases: Vec::new(),
                    fresh: false,
                };
                let item = crate::sent::SentItem {
                    id: format!("as-{name}"),
                    version: 1,
                    cancel: false,
                    from: eve.own_key_record()?,
                    profile: copy,
                    text: format!("Hello from {name}"),
                    text_format: crate::text_format::PLAIN.into(),
                    text_version: crate::text_format::VERSION,
                    sections: Vec::new(),
                    routines: Vec::new(),
                    lines: None,
                };
                accepted(
                    &ben,
                    &crate::sent::encode(&item, &key, &[], &mut |n| Ok(vec![0; n]))?,
                )
            };
        let eve_id = own_id(&eve)?;
        let as_ben = forged_as(&own_id(&ben)?, "Ben")?;
        assert_eq!(ben.author_of(&as_ben)?.map(|p| p.id), Some(eve_id.clone()));
        assert!(!ben.wrote_by(&as_ben, &me())?);
        let cid_id = own_id(&cid)?;
        let as_cid = forged_as(&cid_id, "Cid")?;
        assert_eq!(ben.author_of(&as_cid)?.map(|p| p.id), Some(eve_id));
        assert!(!ben.wrote_by(&as_cid, &PersonFilter::person(&cid_id))?);
        Ok(())
    }

    /// One LAN session between two of one's own Devices: `joiner` joins
    /// `host`.
    fn own_sync(joiner: &Store, host: &Store) -> TestResult {
        let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        crate::lan::apply_answer(joiner, &answer)?;
        Ok(())
    }

    #[test]
    fn keep_mine_reaches_the_people_who_knew_the_profile_given_up() -> TestResult {
        for on_host in [true, false] {
            let phone = device(1, "Ada L.")?;
            let desk = device(2, "Ada")?;
            let cid = device(3, "Cid")?;
            // Cid shares an Area with the phone and keeps its Profile.
            household(&phone, &cid)?;
            let (phone_id, desk_id) = (own_id(&phone)?, own_id(&desk)?);
            assert_eq!(cid.resolve_person(&phone_id)?, phone_id);
            // The phone gives its Profile up for the computer's.
            if on_host {
                own_sync(&phone, &desk)?;
            } else {
                own_sync(&desk, &phone)?;
            }
            OwnProfile::new(&desk).answer(true)?;
            for _ in 0..3 {
                own_sync(&phone, &desk)?;
            }
            assert_eq!(own_id(&phone)?, desk_id);
            assert_eq!(phone.local_setting(ALIASES)?, desk.local_setting(ALIASES)?);
            sync(&cid, &phone)?;
            assert_eq!(
                cid.resolve_person(&phone_id)?,
                desk_id,
                "on host: {on_host}"
            );
            assert_eq!(cid.person_shown(&phone_id)?.name, "Ada");
        }
        Ok(())
    }

    #[test]
    fn a_host_inviting_refuses_a_joiner_without_a_profile() -> TestResult {
        let ann = device(1, "Ann")?;
        let bare = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
        let house = Places::new(&ann).add_area("Household")?;
        let area = ann.share(&house.id)?;
        let invite = Invite {
            area,
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            &bare,
            &ann.version_vector()?,
            "Phone",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        assert!(answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.is_none());
        // The host shows its own Profile to an inviting joiner.
        let answer = profile_answer(&ann)?;
        let shown = bare
            .sent_profile_of(answer.get("profile"), &ann.device_id()?)
            .ok_or("a Profile")?;
        assert_eq!(shown.profile.name, "Ann");
        assert_eq!(Some(shown.code), OwnProfile::new(&ann).code()?);
        assert!(
            bare.sent_profile_of(answer.get("profile"), &bare.device_id()?)
                .is_none()
        );
        Ok(())
    }
}
