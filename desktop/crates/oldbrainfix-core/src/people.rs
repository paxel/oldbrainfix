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
//! Device and is kept through every newer record.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};

use crate::Result;
use crate::entry::dart_iso;
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
}

/// Every kept Profile by id, and the ids they replaced.
#[derive(Debug, Default)]
pub struct PeopleIndex {
    by_id: BTreeMap<String, Held>,
    alias_to: BTreeMap<String, String>,
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
            None => placeholder(resolved),
        })
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
    /// Profile is left to [`OwnProfile::take`].
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
        let mut aliases: BTreeMap<String, ProfileAlias> = held
            .sent
            .aliases
            .iter()
            .map(|a| (a.alias.clone(), a.clone()))
            .collect();
        for a in &theirs.aliases {
            if a.alias == id || aliases.contains_key(&a.alias) {
                continue;
            }
            if bound.contains(&a.by) && a.signed() {
                aliases.insert(a.alias.clone(), a.clone());
            }
        }
        let newer = match Profile::parse(&held.sent.record) {
            None => true,
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
        if next.to_json() == held.sent.to_json() {
            return Ok(PersonTaken::Unchanged);
        }
        let shown = if newer {
            checked.shown.clone()
        } else {
            held.profile.picture.clone()
        };
        let seen = held.seen.clone();
        self.keep_person(&next, shown.as_deref(), &seen, held.verified)?;
        Ok(PersonTaken::Updated)
    }

    fn keep_person(
        &self,
        sent: &SignedProfile,
        shown: Option<&[u8]>,
        seen: &str,
        verified: bool,
    ) -> Result<()> {
        let Some(id) = Profile::parse(&sent.record).map(|p| p.id) else {
            return Ok(());
        };
        let mut value = json!({"profile": sent.to_json(), "seen": seen});
        if let Some(shown) = shown {
            value["shown"] = json!(BASE64.encode(shown));
        }
        if verified {
            value["verified"] = json!(true);
        }
        self.set_local_setting(&format!("{PREFIX}{id}"), &value.to_string())?;
        *self.people.lock() = None;
        Ok(())
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
    /// people the grants of `areas` name now.
    pub fn profiles_to_send(&self, areas: Option<&HashSet<String>>) -> Result<Vec<Value>> {
        let index = self.people_index()?;
        let mut out: BTreeMap<String, Value> = BTreeMap::new();
        match areas {
            None => {
                for (id, held) in &index.by_id {
                    out.insert(id.clone(), held.sent.to_json());
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
                        if let Some(held) = index.by_id.get(&self.resolve_person(&named)?) {
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
        match self.author_theirs(entity)? {
            Some(author) => Ok(Some(self.with_override(author)?)),
            None => Ok(None),
        }
    }

    fn author_theirs(&self, entity: &str) -> Result<Option<Profile>> {
        if let Some((key, person)) = self.received_from(entity)? {
            if self.own_keys()?.bound.contains(&key) || self.is_own_person(&person)? {
                return Ok(None);
            }
            return self.shown_for(Some(&key), Some(person));
        }
        let Some(device) = self
            .history(entity)?
            .into_iter()
            .find(|e| e.field == "$type")
            .map(|e| e.device)
        else {
            return Ok(None);
        };
        if device == self.device_id()? || self.is_own_device(&device)? {
            return Ok(None);
        }
        let key = self
            .pinned_key(&device)?
            .map(|k| BASE64.encode(&k.record.public_key));
        if let Some(key) = &key
            && self.own_keys()?.bound.contains(key)
        {
            return Ok(None);
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
            return Ok(None);
        }
        self.shown_for(key.as_deref(), named)
    }

    /// The sender's key and Profile id the Note `entity` keeps from the
    /// Sent Item it was accepted from; none for any other.
    fn received_from(&self, entity: &str) -> Result<Option<(String, String)>> {
        let Some(raw) = self.current(entity, "received")? else {
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
    aliases.sort_by(|a, b| a.alias.cmp(&b.alias));
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
