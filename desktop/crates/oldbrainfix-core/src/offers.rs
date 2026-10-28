//! Sharing an Area with a person from afar (#108, ADR 0023), as the Dart
//! core's `area_offers.dart`: someone in People who is not on the same
//! network, added by a Profile card or met before. The sharer grants
//! that person's Devices a role as an invite does (ADR 0013), then sends
//! an offer: a bundle of the Area's whole history sealed with the Area's
//! key, with a file `offer` beside it that carries the key, signed by
//! the sharer's Device and sealed to the person's X25519 keys as a Sent
//! Item is (ADR 0015). The receiver holds it as Pending Changes (ADR
//! 0022), never shown, until the user accepts or declines it; nothing
//! of the Area is kept before. Afterwards changes travel by the shared
//! folder or by bundles, as the sharer chose.

use std::collections::{BTreeSet, HashMap, HashSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};

use crate::box_keys::{BOX_KEY_BYTES, box_public_of, box_secret_of, hkdf_sha256, x25519};
use crate::entry::{ENTRY_VERSION, Entry, WireEntry};
use crate::folder::{SyncFolder, entries_sealed_with};
use crate::keys::{
    AREA_KEY_BYTES, AreaKey, SEAL_NONCE_BYTES, area_keys_of, blob_name, seal, sealed_as, unseal,
};
use crate::notes::keys as note_keys;
use crate::pending::PendingSet;
use crate::places::keys as place_keys;
use crate::profile::{Profile, SentProfile, SignedProfile};
use crate::routines::keys as routine_keys;
use crate::sent::SenderTrust;
use crate::shared::{AreaRole, judge_log};
use crate::signing::{ImportReport, KeyRecord, SigningKey, verify_signature};
use crate::store::{Store, entry_of};
use crate::sync::{BLOB_KINDS, cause, split_blob_ref};
use crate::{Error, Result};

/// The kind of pending set an offered Area is held in (ADR 0022).
pub const PENDING_KIND: &str = "area-offer";

/// The name of the offer inside a bundle.
pub const AREA_OFFER_FILE: &str = "offer";

/// What the signature over an offer and its sealing are bound to: never
/// a Sent Item's, so neither stands in for the other.
const SIGNED_PREFIX: &str = "oldbrainfix:area-offer:";
const SEALED_AAD: &str = "oldbrainfix:area-offer";
const KEY_AAD: &str = "oldbrainfix:area-offer-key";

/// The local setting of how changes travel with a person.
const TRANSPORT_PREFIX: &str = "transport:";

/// How the changes of a Shared Area travel with a person met from afar,
/// asked once when sharing with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// A shared folder both reach: runs on its own.
    Folder,
    /// Bundles sent through any messenger.
    Bundle,
}

impl Transport {
    pub fn name(self) -> &'static str {
        match self {
            Transport::Folder => "folder",
            Transport::Bundle => "bundle",
        }
    }

    pub fn parse(name: &str) -> Option<Transport> {
        match name {
            "folder" => Some(Transport::Folder),
            "bundle" => Some(Transport::Bundle),
            _ => None,
        }
    }
}

/// Why an offer was not taken, as the Dart core's exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferError {
    /// No offer this app can read: damaged, not signed by the key it
    /// names, with a Profile not bound to that key, or without a grant
    /// of the Area to the one who sent it.
    NotAnOffer,
    /// Sealed for other Devices, or granting this one nothing.
    NotForUs,
    /// From a person the user blocked (ADR 0021): nothing of it is shown
    /// or kept.
    FromBlocked,
    /// Of an Area this Device holds as its own, which nobody shared with
    /// it (ADR 0013).
    Refused,
}

/// What an offer carries beside the Area's history, signed by the
/// sharer's Device.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaOffer {
    pub area: String,
    pub area_keys: Vec<AreaKey>,
    pub from: KeyRecord,
    pub keys: Vec<KeyRecord>,
    pub profile: SignedProfile,
    pub transport: Transport,
}

/// Builds a JSON object in key order, the same with or without
/// serde_json's `preserve_order`.
fn object(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}

impl AreaOffer {
    /// What is signed: keys sorted, no spaces, as the Dart core writes it.
    pub fn encode(&self) -> String {
        object(vec![
            ("area", json!(self.area)),
            (
                "areaKeys",
                Value::Array(self.area_keys.iter().map(AreaKey::to_json).collect()),
            ),
            ("from", self.from.to_json()),
            (
                "keys",
                Value::Array(self.keys.iter().map(KeyRecord::to_json).collect()),
            ),
            ("profile", self.profile.to_json()),
            ("transport", json!(self.transport.name())),
        ])
        .to_string()
    }

    /// The offer `text` holds; none when it is none: an Area key of
    /// another scope, or none at all, makes none either.
    pub fn parse(text: &str) -> Option<AreaOffer> {
        let json: Value = serde_json::from_str(text).ok()?;
        let area = json.get("area")?.as_str()?;
        if !area.starts_with("area:") {
            return None;
        }
        let listed = json.get("areaKeys")?.as_array()?;
        let area_keys = area_keys_of(json.get("areaKeys"));
        if area_keys.is_empty()
            || area_keys.len() != listed.len()
            || area_keys.iter().any(|k| k.scope != area)
        {
            return None;
        }
        Some(AreaOffer {
            area: area.to_string(),
            area_keys,
            from: KeyRecord::from_json(json.get("from")?)?,
            keys: json
                .get("keys")?
                .as_array()?
                .iter()
                .filter_map(KeyRecord::from_json)
                .collect(),
            profile: SignedProfile::from_json(json.get("profile")?)?,
            transport: Transport::parse(json.get("transport")?.as_str()?)?,
        })
    }
}

/// An offered Area held here until the user accepts or declines it.
#[derive(Debug, Clone, PartialEq)]
pub struct HeldAreaOffer {
    /// The pending set it is held in.
    pub id: String,
    pub area: String,
    /// The Area's name as stored, and its Starter Set slug.
    pub area_name: String,
    pub area_starter: Option<String>,
    pub role: AreaRole,
    /// The sharer's Profile as the offer carries it, the name sanitised
    /// and the picture made again, and its short code.
    pub sender: Profile,
    pub code: String,
    pub trust: SenderTrust,
    pub transport: Transport,
    /// When it came here.
    pub held: DateTime<Utc>,
}

/// What a held set keeps for Accept beside the entries: the offer as it
/// came, the Profiles of the Area's people, and the photos and
/// recordings kept for it.
struct Kept {
    offer: AreaOffer,
    profiles: Vec<Value>,
    blobs: Vec<String>,
}

impl Kept {
    fn encode(&self) -> String {
        object(vec![
            ("blobs", json!(self.blobs)),
            ("offer", json!(self.offer.encode())),
            ("profiles", Value::Array(self.profiles.clone())),
        ])
        .to_string()
    }

    fn parse(text: Option<&str>) -> Option<Kept> {
        let json: Value = serde_json::from_str(text?).ok()?;
        let blobs = json.get("blobs")?.as_array()?;
        let offer = AreaOffer::parse(json.get("offer")?.as_str()?)?;
        let profiles = json.get("profiles")?.as_array()?.clone();
        Some(Kept {
            offer,
            profiles,
            blobs: blobs
                .iter()
                .filter_map(|b| b.as_str().map(str::to_string))
                .collect(),
        })
    }
}

/// The file `offer` goes out as, signed by `key` and sealed to each X25519
/// key of `to`, as the Dart core's `encodeAreaOffer`. The bytes come from
/// `random` as the Dart core draws them: the content key, the content's
/// nonce, then per recipient its ephemeral secret and nonce.
pub fn encode_area_offer(
    offer: &AreaOffer,
    key: &SigningKey,
    to: &[[u8; BOX_KEY_BYTES]],
    random: &mut dyn FnMut(usize) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    if to.is_empty() {
        return Err(Error::Invalid("nobody to seal an offer for".into()));
    }
    let payload = offer.encode();
    let signed = object(vec![
        ("offer", json!(payload)),
        ("sig", json!(key.sign(&signed_bytes(&payload)))),
    ])
    .to_string()
    .into_bytes();
    let content: [u8; AREA_KEY_BYTES] = array(random(AREA_KEY_BYTES)?)?;
    let nonce: [u8; SEAL_NONCE_BYTES] = array(random(SEAL_NONCE_BYTES)?)?;
    let sealed = seal(&content, &signed, SEALED_AAD, &nonce)?;
    let mut stanzas = Vec::new();
    for recipient in to {
        let ephemeral: [u8; BOX_KEY_BYTES] = array(random(BOX_KEY_BYTES)?)?;
        let public = box_public_of(&ephemeral);
        let shared = x25519(&ephemeral, recipient)
            .ok_or_else(|| Error::Invalid("not a key to encrypt to".into()))?;
        let nonce: [u8; SEAL_NONCE_BYTES] = array(random(SEAL_NONCE_BYTES)?)?;
        let wrapped = seal(
            &wrap_key(&shared, &public, recipient),
            &content,
            KEY_AAD,
            &nonce,
        )?;
        stanzas.push(object(vec![
            ("eph", json!(BASE64.encode(public))),
            ("key", json!(BASE64.encode(wrapped))),
        ]));
    }
    Ok(object(vec![
        ("sealed", json!(BASE64.encode(sealed))),
        ("to", Value::Array(stanzas)),
    ])
    .to_string()
    .into_bytes())
}

/// Fresh random bytes, for the offer's content key, nonces and ephemeral
/// keys.
pub(crate) fn random_vec(n: usize) -> Result<Vec<u8>> {
    let mut out = vec![0u8; n];
    getrandom::fill(&mut out).map_err(|e| Error::Invalid(format!("random: {e}")))?;
    Ok(out)
}

fn array<const N: usize>(bytes: Vec<u8>) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::Invalid("random: wrong length".into()))
}

fn signed_bytes(payload: &str) -> Vec<u8> {
    format!("{SIGNED_PREFIX}{payload}").into_bytes()
}

/// The key the content key is sealed with for one recipient: HKDF over
/// the X25519 secret, bound to both public keys and to an offer.
fn wrap_key(shared: &[u8; 32], ephemeral: &[u8; 32], to: &[u8; 32]) -> [u8; AREA_KEY_BYTES] {
    let mut salt = ephemeral.to_vec();
    salt.extend_from_slice(to);
    let mut out = [0u8; AREA_KEY_BYTES];
    out.copy_from_slice(&hkdf_sha256(
        shared,
        &salt,
        KEY_AAD.as_bytes(),
        AREA_KEY_BYTES,
    ));
    out
}

/// Whether `json` is in the format this version reads: `format`, 1 when
/// missing, and 1 is never written (ADR 0019).
fn in_format(json: &Value) -> bool {
    json.get("format").is_none_or(|f| f.as_i64() == Some(1))
}

impl Store {
    /// How changes travel with `person`, or none when the user was never
    /// asked (#108): a local setting of this Device (ADR 0008).
    pub fn transport_with(&self, person: &str) -> Result<Option<Transport>> {
        let key = format!("{TRANSPORT_PREFIX}{}", self.resolve_person(person)?);
        Ok(self
            .local_setting(&key)?
            .and_then(|name| Transport::parse(&name)))
    }

    pub fn set_transport_with(&self, person: &str, transport: Transport) -> Result<()> {
        let key = format!("{TRANSPORT_PREFIX}{}", self.resolve_person(person)?);
        self.set_local_setting(&key, transport.name())
    }

    /// The offer the Shared Area `area` goes out as now, from this
    /// Device, as the Dart core's `areaOfferFor`.
    pub fn area_offer_for(&self, area: &str, person: &str) -> Result<AreaOffer> {
        let profile = self
            .own_profile_for_others()?
            .and_then(|json| SignedProfile::from_json(&json))
            .ok_or_else(|| Error::Invalid("no own Profile to offer with".into()))?;
        self.ensure_area_keys()?;
        let own = self.device_id()?;
        let mut devices: BTreeSet<String> = self
            .area_history(area)?
            .into_iter()
            .map(|e| e.device)
            .collect();
        if let Some(grants) = self.grants_of(area)? {
            devices.extend(grants.devices());
        }
        let mut keys = vec![self.own_key_record()?];
        for device in devices {
            if device == own {
                continue;
            }
            if let Some(pinned) = self.pinned_key(&device)? {
                keys.push(pinned.record);
            }
        }
        keys.sort_by(|a, b| a.device.cmp(&b.device));
        let mut area_keys: Vec<AreaKey> = self
            .area_keys()?
            .into_iter()
            .filter(|k| k.scope == area)
            .collect();
        area_keys.sort_by_key(AreaKey::id);
        Ok(AreaOffer {
            area: area.to_string(),
            area_keys,
            from: self.own_key_record()?,
            keys,
            profile,
            transport: self.transport_with(person)?.unwrap_or(Transport::Bundle),
        })
    }

    /// The Devices of `person` an offer can be sealed for: pinned here,
    /// bound to their Profile, with an X25519 key, never one of the
    /// user's own; sorted.
    pub fn offer_devices(&self, person: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for device in self.pinned_devices_of(&self.resolve_person(person)?)? {
            if self.written_by_own(&device)? {
                continue;
            }
            if self
                .pinned_key(&device)?
                .is_some_and(|p| p.record.verified_box_key().is_some())
            {
                out.push(device);
            }
        }
        Ok(out)
    }

    /// "Share with…" (#108), as the Dart core's `shareAreaWith`: shares
    /// the Area `area`, grants `person` `role` on every Device of theirs
    /// an offer can be sealed for, and keeps `transport` as how changes
    /// travel with them. Returns the id the Area goes on under. An error,
    /// changing nothing, for the own Profile, a blocked person, or one
    /// none of whose Devices' keys is known here.
    pub fn share_area_with(
        &self,
        area: &str,
        person: &str,
        role: AreaRole,
        transport: Transport,
    ) -> Result<String> {
        let id = self.resolve_person(person)?;
        if self.is_own_person(&id)? || self.is_blocked(&id)? {
            return Err(Error::Invalid(format!("nobody to share with: {person}")));
        }
        let devices = self.offer_devices(&id)?;
        if devices.is_empty() {
            return Err(Error::Invalid(format!("no key of {person} to offer to")));
        }
        let shared = self.share(area)?;
        self.batch(|| {
            for device in &devices {
                self.invite(&shared, device, role, &id)?;
            }
            Ok(())
        })?;
        self.set_transport_with(&id, transport)?;
        Ok(shared)
    }

    /// The X25519 keys of the Devices of `person` that the grants of
    /// `area` give a role now, by device: those an offer is sealed for.
    pub fn offer_keys(&self, area: &str, person: &str) -> Result<Vec<[u8; BOX_KEY_BYTES]>> {
        let Some(grants) = self.grants_of(area)? else {
            return Ok(Vec::new());
        };
        let target = self.resolve_person(person)?;
        let mut out = Vec::new();
        for device in grants.devices() {
            let Some(grant) = grants.grant_of(&device) else {
                continue;
            };
            if self.resolve_person(&grant.profile)? != target || self.written_by_own(&device)? {
                continue;
            }
            if let Some(key) = self
                .pinned_key(&device)?
                .and_then(|p| p.record.verified_box_key())
            {
                out.push(key);
            }
        }
        Ok(out)
    }

    // -------------------------------------------------------- receiving

    /// Takes in the offer `bytes` that came inside the bundle `folder`, as
    /// the Dart core's `receiveAreaOffer`: opens and checks it, reads the
    /// Area's history and Profiles from the bundle with the offered key,
    /// and holds them as a pending set never shown, the photos and
    /// recordings among the blobs this Device lacked. The offer as held;
    /// the one held already when the same offer comes again; none when
    /// this Device holds a role in the Area already.
    pub fn receive_area_offer(
        &self,
        folder: &dyn SyncFolder,
        bytes: &[u8],
    ) -> Result<std::result::Result<Option<HeldAreaOffer>, OfferError>> {
        let (offer, sender) = match self.read_offer(bytes)? {
            Ok(read) => read,
            Err(e) => return Ok(Err(e)),
        };
        let area = offer.area.clone();
        let own = self.device_id()?;
        if self
            .grants_of(&area)?
            .is_some_and(|g| g.grant_of(&own).is_some())
        {
            for key in &offer.area_keys {
                if self.area_key(&key.id())?.is_some() {
                    return Ok(Ok(None));
                }
            }
        }
        if self.refuses_invite(&area)? {
            return Ok(Err(OfferError::Refused));
        }
        for set in self.held_offer_sets()? {
            if let Some(kept) = Kept::parse(set.payload.as_deref())
                && kept.offer.area == area
                && kept.offer.from.device == offer.from.device
            {
                return Ok(self.held_of(&set)?.map(Some).ok_or(OfferError::NotAnOffer));
            }
        }
        let Some(read) = entries_sealed_with(self, folder, &offer.area_keys)? else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        let entries = self.in_area(&area, &read)?;
        if self.judge_offer(&offer, &entries)?.is_none() {
            return Ok(Err(OfferError::NotForUs));
        }
        let profiles = self.offer_profiles(folder, &offer.area_keys)?;
        let blobs = self.keep_offer_blobs(folder, &offer.area_keys, &entries)?;
        let keys = offer.keys.clone();
        let kept = Kept {
            offer,
            profiles,
            blobs,
        };
        let id = self.open_pending(PENDING_KIND, false, Some(&kept.encode()))?;
        self.receive_pending(&id, &entries, &keys, &mut ImportReport::default())?;
        let set = self
            .pending_set(&id)?
            .ok_or_else(|| Error::Invalid(format!("no pending set {id}")))?;
        // What the grants said holds only for entries whose signatures
        // hold.
        match self.offer_of(&set, &kept.offer, &sender)? {
            Some(held) => Ok(Ok(Some(held))),
            None => {
                self.decline_area_offer(&id)?;
                Ok(Err(OfferError::NotAnOffer))
            }
        }
    }

    /// The offered Areas held here, the oldest first; one that no longer
    /// reads, as from a person blocked since, is left out.
    pub fn held_area_offers(&self) -> Result<Vec<HeldAreaOffer>> {
        let mut out = Vec::new();
        for set in self.held_offer_sets()? {
            if let Some(held) = self.held_of(&set)? {
                out.push(held);
            }
        }
        Ok(out)
    }

    /// The offered Area held as the pending set `id`; none when it is
    /// gone, accepted or declined, or no longer reads.
    pub fn held_area_offer(&self, id: &str) -> Result<Option<HeldAreaOffer>> {
        match self.held_offer_set(id)? {
            Some(set) => self.held_of(&set),
            None => Ok(None),
        }
    }

    /// Accepts the offered Area held as `id`, as the Dart core's
    /// `acceptAreaOffer`: the Area's key is kept, then its history taken
    /// in as any import is, with a Moment before it; then the Profiles its
    /// grants name and how changes travel with the sharer are kept; with
    /// `verified` the sharer's Profile is marked verified. Returns the
    /// Area's id. [`OfferError::FromBlocked`] for one from a person
    /// blocked since and [`OfferError::Refused`] for an Area this Device
    /// made its own meanwhile, both declined.
    pub fn accept_area_offer(&self, id: &str, verified: bool) -> Result<String> {
        let missing = || Error::Invalid(format!("no offer {id}"));
        let set = self.held_offer_set(id)?.ok_or_else(missing)?;
        let kept = Kept::parse(set.payload.as_deref()).ok_or_else(missing)?;
        let offer = kept.offer;
        let sender = self
            .sent_item_profile(&offer.profile.to_json(), &offer.from.public_key)?
            .ok_or_else(missing)?;
        if self.offer_from_blocked(&offer, Some(&sender.profile.id))? {
            self.decline_area_offer(id)?;
            return Err(Error::Offer(OfferError::FromBlocked));
        }
        if self.refuses_invite(&offer.area)? {
            self.decline_area_offer(id)?;
            return Err(Error::Offer(OfferError::Refused));
        }
        let before = self.current_seq()?;
        // The key first: a history held without it could never be read on.
        let keys: Vec<AreaKey> = offer
            .area_keys
            .iter()
            .filter(|k| k.scope == offer.area)
            .cloned()
            .collect();
        self.learn_area_keys(&keys)?;
        let accepted = self.accept_pending(id)?;
        let mut people = kept.profiles;
        people.push(offer.profile.to_json());
        let allowed = self.people_of(&offer.area)?;
        self.take_people(Some(&Value::Array(people)), Some(&allowed))?;
        if verified {
            self.verify_person(&sender.profile.id, &sender.code)?;
        }
        self.set_transport_with(&sender.profile.id, offer.transport)?;
        let label = (!sender.profile.name.is_empty()).then_some(sender.profile.name.as_str());
        self.moment_for(before, !accepted.taken.is_empty(), cause::BUNDLE, label)?;
        Ok(offer.area)
    }

    /// Declines the offered Area held as `id`: the set is deleted, and the
    /// photos and recordings kept for it that the log does not refer to
    /// go with it. Returns whether there was one.
    pub fn decline_area_offer(&self, id: &str) -> Result<bool> {
        let Some(set) = self.held_offer_set(id)? else {
            return Ok(false);
        };
        let kept = Kept::parse(set.payload.as_deref());
        self.decline_pending(id)?;
        for reference in kept.map(|k| k.blobs).unwrap_or_default() {
            if self.knows_blob(&reference)? {
                continue;
            }
            let (kind, hash) = split_blob_ref(&reference);
            self.drop_blob(kind, hash)?;
        }
        Ok(true)
    }

    /// "Decline and block" on the offered Area held as `id` (#90), as the
    /// Dart core's `blockAreaOfferSender`: declines it and blocks its
    /// sharer as a Sent Item's sender is blocked
    /// ([`Store::block_sent_item_sender`]), with the same
    /// [`Error::MustLeaveAreas`] before anything changes. Returns whether
    /// anybody was blocked; the offer is declined either way.
    pub fn block_area_offer_sender(&self, id: &str, leave_areas: bool) -> Result<bool> {
        let Some(set) = self.held_offer_set(id)? else {
            return Ok(false);
        };
        let mut blocked = false;
        if let Some(kept) = Kept::parse(set.payload.as_deref())
            && let Some(sender) =
                self.sent_item_profile(&kept.offer.profile.to_json(), &kept.offer.from.public_key)?
        {
            blocked = self.block_sender(
                &sender.profile.id,
                &sender.code,
                &kept.offer.profile,
                leave_areas,
            )?;
        }
        self.decline_area_offer(id)?;
        Ok(blocked)
    }

    /// Declines every offered Area held here from a person the user
    /// blocked (ADR 0021): their Profile's id is blocked, or one of their
    /// Devices signed it. Returns whether any went.
    pub(crate) fn drop_offers_from_blocked(&self) -> Result<bool> {
        if self.blocked_ids()?.is_empty() {
            return Ok(false);
        }
        let mut dropped = false;
        for set in self.held_offer_sets()? {
            let Some(kept) = Kept::parse(set.payload.as_deref()) else {
                continue;
            };
            let id = Profile::parse(&kept.offer.profile.record).map(|p| p.id);
            if self.offer_from_blocked(&kept.offer, id.as_deref())?
                && self.decline_area_offer(&set.id)?
            {
                dropped = true;
            }
        }
        Ok(dropped)
    }

    /// The offer in `bytes`, opened with this Device's X25519 key and
    /// checked: its signature under the key record it names, and the
    /// Profile it carries bound to that key, as a Sent Item's.
    fn read_offer(
        &self,
        bytes: &[u8],
    ) -> Result<std::result::Result<(AreaOffer, SentProfile), OfferError>> {
        let Ok(outer) = serde_json::from_slice::<Value>(bytes) else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        let (true, Some(sealed), Some(to)) = (
            in_format(&outer),
            outer.get("sealed").and_then(Value::as_str),
            outer.get("to").and_then(Value::as_array),
        ) else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        let Ok(body) = BASE64.decode(sealed) else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        let Some(inner) = self.open_offer(&body, to)? else {
            return Ok(Err(OfferError::NotForUs));
        };
        let Ok(signed) = serde_json::from_slice::<Value>(&inner) else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        let (true, Some(payload), Some(sig)) = (
            in_format(&signed),
            signed.get("offer").and_then(Value::as_str),
            signed.get("sig").and_then(Value::as_str),
        ) else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        let Some(offer) = AreaOffer::parse(payload) else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        if !offer.from.valid()
            || !verify_signature(&offer.from.public_key, &signed_bytes(payload), Some(sig))
        {
            return Ok(Err(OfferError::NotAnOffer));
        }
        let Some(sender) =
            self.sent_item_profile(&offer.profile.to_json(), &offer.from.public_key)?
        else {
            return Ok(Err(OfferError::NotAnOffer));
        };
        if self.offer_from_blocked(&offer, Some(&sender.profile.id))? {
            return Ok(Err(OfferError::FromBlocked));
        }
        Ok(Ok((offer, sender)))
    }

    /// The signed offer inside `body`, opened with the first stanza of
    /// `to` sealed for this Device's X25519 key.
    fn open_offer(&self, body: &[u8], to: &[Value]) -> Result<Option<Vec<u8>>> {
        let secret = box_secret_of(self.signing_key()?.seed());
        let mine = box_public_of(&secret);
        for stanza in to {
            let (Some(eph), Some(wrapped)) = (
                stanza.get("eph").and_then(Value::as_str),
                stanza.get("key").and_then(Value::as_str),
            ) else {
                continue;
            };
            let Some(public) = BASE64
                .decode(eph)
                .ok()
                .and_then(|p| <[u8; BOX_KEY_BYTES]>::try_from(p).ok())
            else {
                continue;
            };
            let Ok(wrapped) = BASE64.decode(wrapped) else {
                continue;
            };
            let Some(shared) = x25519(&secret, &public) else {
                continue;
            };
            let Some(content) = unseal(&wrap_key(&shared, &public, &mine), &wrapped, KEY_AAD)
                .and_then(|c| <[u8; AREA_KEY_BYTES]>::try_from(c).ok())
            else {
                continue;
            };
            return Ok(unseal(&content, body, SEALED_AAD));
        }
        Ok(None)
    }

    /// Whether `offer` comes from a person the user blocked: the Profile
    /// id `profile` it carries is, or a blocked person's Device signed it.
    fn offer_from_blocked(&self, offer: &AreaOffer, profile: Option<&str>) -> Result<bool> {
        if let Some(id) = profile
            && self.is_blocked(id)?
        {
            return Ok(true);
        }
        Ok(self.blocked_devices()?.contains(&offer.from.device))
    }

    /// The entries of `entries` whose entity lies in `area` by what the
    /// entries themselves say, as the Dart core's `_inArea`; whatever this
    /// Device holds already in another scope stays out.
    fn in_area(&self, area: &str, entries: &[WireEntry]) -> Result<Vec<WireEntry>> {
        let is = |e: &WireEntry, field: &str| e.field == field;
        let topics: HashSet<&str> = entries
            .iter()
            .filter(|e| {
                e.entity.starts_with("topic:")
                    && is(e, place_keys::AREA)
                    && e.value.as_deref() == Some(area)
            })
            .map(|e| e.entity.as_str())
            .collect();
        let notes: HashSet<&str> = entries
            .iter()
            .filter(|e| {
                e.entity.starts_with("note:")
                    && is(e, note_keys::PLACE)
                    && e.value
                        .as_deref()
                        .is_some_and(|v| v == area || topics.contains(v))
            })
            .map(|e| e.entity.as_str())
            .collect();
        let mut inside: HashSet<&str> = HashSet::from([area]);
        inside.extend(&topics);
        inside.extend(&notes);
        for e in entries {
            let child = (e.entity.starts_with("section:") && is(e, note_keys::NOTE))
                || (e.entity.starts_with("routine:") && is(e, routine_keys::NOTE));
            if child && e.value.as_deref().is_some_and(|v| notes.contains(v)) {
                inside.insert(e.entity.as_str());
            }
        }
        let mut foreign: HashMap<String, bool> = HashMap::new();
        let mut out = Vec::new();
        for e in entries {
            if !inside.contains(e.entity.as_str()) {
                continue;
            }
            let elsewhere = match foreign.get(&e.entity) {
                Some(f) => *f,
                None => {
                    let f = !self.logged_history(&e.entity)?.is_empty()
                        && self.scope_of(&e.entity)? != area;
                    foreign.insert(e.entity.clone(), f);
                    f
                }
            };
            if !elsewhere {
                out.push(e.clone());
            }
        }
        Ok(out)
    }

    /// The role `entries`, judged with the log, give this Device in the
    /// offered Area, under the user's own Profile, when the Device that
    /// signed the offer is a Manager there under the Profile it carries,
    /// with what the roles leave out.
    fn judge_offer(
        &self,
        offer: &AreaOffer,
        entries: &[WireEntry],
    ) -> Result<Option<(AreaRole, HashSet<String>)>> {
        let mut all: Vec<Entry> = self
            .all_entries()?
            .into_iter()
            .filter(|e| e.v() <= ENTRY_VERSION)
            .collect();
        for e in entries {
            if e.applies() {
                all.push(entry_of(0, e.clone())?);
            }
        }
        let judged = judge_log(&all);
        let Some(grants) = judged.areas.get(&offer.area) else {
            return Ok(None);
        };
        let Some(mine) = grants.grant_of(&self.device_id()?) else {
            return Ok(None);
        };
        if !self.is_own_person(&mine.profile)? {
            return Ok(None);
        }
        let Some(theirs) = grants.grant_of(&offer.from.device) else {
            return Ok(None);
        };
        let Some(sharer) = Profile::parse(&offer.profile.record).map(|p| p.id) else {
            return Ok(None);
        };
        if theirs.role != AreaRole::Manager
            || self.resolve_person(&theirs.profile)? != self.resolve_person(&sharer)?
        {
            return Ok(None);
        }
        Ok(Some((mine.role, judged.ignored.clone())))
    }

    /// The Profiles the bundle `folder` holds sealed with one of `keys`,
    /// in the order they lie there.
    fn offer_profiles(&self, folder: &dyn SyncFolder, keys: &[AreaKey]) -> Result<Vec<Value>> {
        let own = self.device_id()?;
        let by_id: HashMap<String, &AreaKey> = keys.iter().map(|k| (k.id(), k)).collect();
        let mut out = Vec::new();
        for name in folder.list("keys")? {
            let Some(stem) = name.strip_suffix(".profiles") else {
                continue;
            };
            let parts: Vec<&str> = stem.split('.').collect();
            let [device, id] = parts.as_slice() else {
                continue;
            };
            if device.is_empty() || *device == own {
                continue;
            }
            let Some(key) = by_id.get(*id) else {
                continue;
            };
            let Some(bytes) = folder.read("keys", &name)? else {
                continue;
            };
            let Some(plain) = unseal(&key.key, &bytes, &sealed_as::profiles(device)) else {
                continue;
            };
            let Ok(json) = serde_json::from_slice::<Value>(&plain) else {
                continue;
            };
            if let Some(list) = json.get("profiles").and_then(Value::as_array) {
                out.extend(list.iter().cloned());
            }
        }
        Ok(out)
    }

    /// Keeps the photos and recordings `entries` name that this Device
    /// lacks and the bundle `folder` holds sealed with one of `keys`;
    /// returns those kept, sorted, so a Decline lets them go again.
    fn keep_offer_blobs(
        &self,
        folder: &dyn SyncFolder,
        keys: &[AreaKey],
        entries: &[WireEntry],
    ) -> Result<Vec<String>> {
        let mut refs = BTreeSet::new();
        for e in entries {
            for (kind, prefix) in BLOB_KINDS {
                if let Some(hash) = e.field.strip_prefix(prefix) {
                    refs.insert(format!("{kind}/{hash}"));
                }
            }
        }
        let names: HashSet<String> = folder.list("blobs")?.into_iter().collect();
        let mut kept = Vec::new();
        for reference in refs {
            let (kind, hash) = split_blob_ref(&reference);
            if self.blob(kind, hash).is_some() {
                continue;
            }
            for key in keys {
                let name = blob_name(&key.key, &reference);
                if !names.contains(&name) {
                    continue;
                }
                let Some(bytes) = folder.read("blobs", &name)? else {
                    continue;
                };
                // Bytes that are not the blob they are named for stay out.
                if let Some(plain) = unseal(&key.key, &bytes, &sealed_as::blob(&reference))
                    && self.put_blob(kind, hash, &plain)?
                {
                    kept.push(reference.clone());
                    break;
                }
            }
        }
        Ok(kept)
    }

    fn held_offer_sets(&self) -> Result<Vec<PendingSet>> {
        Ok(self
            .pending_sets()?
            .into_iter()
            .filter(|s| s.kind == PENDING_KIND)
            .collect())
    }

    fn held_offer_set(&self, id: &str) -> Result<Option<PendingSet>> {
        Ok(self.pending_set(id)?.filter(|s| s.kind == PENDING_KIND))
    }

    /// The held `set` as an offer, checked again against what this Device
    /// knows now; none when it no longer reads.
    fn held_of(&self, set: &PendingSet) -> Result<Option<HeldAreaOffer>> {
        let Some(kept) = Kept::parse(set.payload.as_deref()) else {
            return Ok(None);
        };
        let offer = kept.offer;
        let Some(sender) =
            self.sent_item_profile(&offer.profile.to_json(), &offer.from.public_key)?
        else {
            return Ok(None);
        };
        if self.offer_from_blocked(&offer, Some(&sender.profile.id))? {
            return Ok(None);
        }
        self.offer_of(set, &offer, &sender)
    }

    fn offer_of(
        &self,
        set: &PendingSet,
        offer: &AreaOffer,
        sender: &SentProfile,
    ) -> Result<Option<HeldAreaOffer>> {
        let entries: Vec<WireEntry> = set.received().iter().map(|r| r.entry.clone()).collect();
        let Some((role, ignored)) = self.judge_offer(offer, &entries)? else {
            return Ok(None);
        };
        // The name and slug as the roles have them: a Member's rename of
        // the Area is relayed, but counts nowhere.
        let latest = |field: &str| -> Result<Option<String>> {
            let mut held: Option<Entry> = None;
            for e in &entries {
                if e.entity != offer.area || e.field != field || !e.applies() {
                    continue;
                }
                if ignored.contains(&format!("{}:{}", e.device, e.dseq)) {
                    continue;
                }
                let e = entry_of(0, e.clone())?;
                if held
                    .as_ref()
                    .is_none_or(|h| Store::latest_wins(&e, h) == std::cmp::Ordering::Greater)
                {
                    held = Some(e);
                }
            }
            Ok(held.and_then(|e| e.value))
        };
        Ok(Some(HeldAreaOffer {
            id: set.id.clone(),
            area: offer.area.clone(),
            area_name: latest(place_keys::NAME)?.unwrap_or_default(),
            area_starter: latest(place_keys::STARTER)?,
            role,
            sender: sender.profile.clone(),
            code: sender.code.clone(),
            trust: if sender.verified {
                SenderTrust::Verified
            } else if sender.kept {
                SenderTrust::Known
            } else {
                SenderTrust::Unknown
            },
            transport: offer.transport,
            held: set.made,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{
        import_bundle, write_area_offer_now, write_bundle_now, write_person_bundle_now,
    };
    use crate::folder::{MemoryFolder, folder_sync, random_nonce, write_sealed};
    use crate::lan::{answer_sync, apply_answer, sync_request};
    use crate::profile::OwnProfile;
    use crate::routines::Routines;
    use crate::schedule::{ClockTime, Day, Schedule};
    use crate::shared::AreaAccess;
    use crate::store::tests::ticking;
    use crate::{Notes, Places};
    use std::path::{Path, PathBuf};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    fn bare(seed: u8, name: &str) -> Made<Store> {
        let start = ticking()();
        let clock: crate::Clock = Box::new(move || {
            let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            start + chrono::Duration::seconds(n)
        });
        let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    fn device(seed: u8, name: &str) -> Made<Store> {
        let store = bare(seed, name)?;
        OwnProfile::new(&store).rename(name)?;
        Ok(store)
    }

    fn id_of(store: &Store) -> Made<String> {
        Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
    }

    /// Each sent the other their Profile card through a messenger.
    fn met(a: &Store, b: &Store) -> TestResult {
        b.open_profile_card(&a.profile_card(false)?.ok_or("a card")?, false)?;
        a.open_profile_card(&b.profile_card(true)?.ok_or("a card")?, false)?;
        Ok(())
    }

    /// One LAN session between two of one's own Devices.
    fn meet(joiner: &Store, host: &Store) -> TestResult {
        let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        apply_answer(joiner, &answer)?;
        Ok(())
    }

    fn photo() -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend((0..64u32).map(|i| ((i * 3) & 0xff) as u8));
        bytes.extend([0xff, 0xd9]);
        bytes
    }

    fn texts(store: &Store) -> Made<Vec<String>> {
        let mut texts: Vec<String> = Notes::new(store)
            .all()?
            .into_iter()
            .map(|n| n.text)
            .collect();
        texts.sort();
        Ok(texts)
    }

    struct Offered {
        ann: Store,
        ben: Store,
        area: String,
        note: String,
        file: PathBuf,
        dir: tempfile::TempDir,
    }

    /// Ann's Household with a Note and its photo, a Routine and a private
    /// Note beside it, shared with Ben, whom she knows from his card, as
    /// `role`; the offer written.
    fn offered(role: AreaRole, transport: Transport) -> Made<Offered> {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        met(&ann, &ben)?;
        let house = Places::new(&ann).add_area("Household")?;
        let milk =
            Notes::new(&ann).capture_with("Buy milk", Some(&house.id), None, &[photo()], &[])?;
        let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
        let schedule = Schedule::new(Day::new(2026, 3, 2), &eight, None).ok_or("a schedule")?;
        Routines::new(&ann).spawn(&milk.id, &schedule)?;
        Notes::new(&ann).capture("Private", None, None)?;
        let area = ann.share_area_with(&house.id, &id_of(&ben)?, role, transport)?;
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("offer.obfsync");
        write_area_offer_now(&ann, &file, &area, &id_of(&ben)?)?;
        Ok(Offered {
            ann,
            ben,
            area,
            note: milk.id,
            file,
            dir,
        })
    }

    fn held(store: &Store, file: &Path) -> Made<HeldAreaOffer> {
        Ok(import_bundle(store, file)?.offer.ok_or("an offer")?)
    }

    fn is(result: Result<impl std::fmt::Debug>, want: OfferError) -> bool {
        matches!(result, Err(Error::Offer(e)) if e == want)
    }

    #[test]
    fn the_offer_is_held_until_accepted_and_nothing_of_the_area_is_kept_before() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let before = o.ben.all_entries()?.len();
        let got = import_bundle(&o.ben, &o.file)?;
        assert!(got.applied.is_empty());
        let offer = got.offer.ok_or("an offer")?;
        assert_eq!(offer.sender.name, "Ann");
        assert_eq!(offer.area_name, "Household");
        assert_eq!(offer.role, AreaRole::Member);
        assert_eq!(offer.trust, SenderTrust::Known);
        assert_eq!(offer.transport, Transport::Bundle);
        assert_eq!(o.ben.all_entries()?.len(), before);
        assert!(Notes::new(&o.ben).by_id(&o.note)?.is_none());
        assert!(o.ben.writing_key(&o.area)?.is_none());
        assert_eq!(held(&o.ben, &o.file)?.id, offer.id);

        assert_eq!(o.ben.accept_area_offer(&offer.id, false)?, o.area);
        assert!(o.ben.held_area_offers()?.is_empty());
        assert_eq!(texts(&o.ben)?, vec!["Buy milk"]);
        let hash = Notes::new(&o.ben)
            .by_id(&o.note)?
            .ok_or("the Note")?
            .photos
            .first()
            .cloned()
            .ok_or("a photo")?;
        assert_eq!(o.ben.photo(&hash), o.ann.photo(&hash));
        assert_eq!(Routines::new(&o.ben).of_note(&o.note)?.len(), 1);
        assert_eq!(o.ben.access_in(&o.area, None)?, AreaAccess::Member);
        assert_eq!(
            o.ben.writing_key(&o.area)?.map(|k| k.id()),
            o.ann.writing_key(&o.area)?.map(|k| k.id())
        );
        assert_eq!(
            o.ben.transport_with(&id_of(&o.ann)?)?,
            Some(Transport::Bundle)
        );
        Ok(())
    }

    #[test]
    fn changes_travel_by_bundles_both_ways_and_the_roles_hold() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let offer = held(&o.ben, &o.file)?;
        o.ben.accept_area_offer(&offer.id, false)?;
        Notes::new(&o.ben).capture("From Ben", Some(&o.area), None)?;
        // A Member may not change Ann's Note: her Devices ignore it.
        o.ben
            .append(&o.note, note_keys::TEXT, Some("Changed by Ben"), None)?;
        let back = o.dir.path().join("back.obfsync");
        assert!(write_person_bundle_now(&o.ben, &back, &id_of(&o.ann)?)?.is_some());
        assert_eq!(import_bundle(&o.ann, &back)?.unreadable, 0);
        assert_eq!(texts(&o.ann)?, vec!["Buy milk", "From Ben", "Private"]);

        Notes::new(&o.ann).capture("From Ann", Some(&o.area), None)?;
        let again = o.dir.path().join("again.obfsync");
        assert!(write_person_bundle_now(&o.ann, &again, &id_of(&o.ben)?)?.is_some());
        import_bundle(&o.ben, &again)?;
        assert_eq!(texts(&o.ben)?, vec!["Buy milk", "From Ann", "From Ben"]);
        Ok(())
    }

    #[test]
    fn through_a_shared_folder_every_device_of_the_person_is_checked() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Folder)?;
        let offer = held(&o.ben, &o.file)?;
        assert_eq!(offer.transport, Transport::Folder);
        o.ben.accept_area_offer(&offer.id, false)?;
        // Ben's computer, paired as his own, takes his role over.
        let desk = bare(3, "Ben")?;
        meet(&desk, &o.ben)?;
        meet(&desk, &o.ben)?;
        assert_eq!(desk.access_in(&o.area, None)?, AreaAccess::Member);

        let mut folder = MemoryFolder::default();
        Notes::new(&desk).capture("From the desk", Some(&o.area), None)?;
        folder_sync(&o.ann, &mut folder, &mut random_nonce)?;
        folder_sync(&desk, &mut folder, &mut random_nonce)?;
        folder_sync(&o.ben, &mut folder, &mut random_nonce)?;
        Notes::new(&o.ann).capture("From Ann", Some(&o.area), None)?;
        folder_sync(&o.ann, &mut folder, &mut random_nonce)?;
        folder_sync(&o.ben, &mut folder, &mut random_nonce)?;
        // Ann met Ben's computer nowhere: its key came with the Area.
        assert_eq!(
            texts(&o.ann)?,
            vec!["Buy milk", "From Ann", "From the desk", "Private"]
        );
        assert_eq!(
            texts(&o.ben)?,
            vec!["Buy milk", "From Ann", "From the desk"]
        );
        Ok(())
    }

    #[test]
    fn a_bundle_for_a_device_met_from_afar_holds_only_what_it_shares() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        o.ben.accept_area_offer(&held(&o.ben, &o.file)?.id, false)?;
        Notes::new(&o.ann).capture("Still private", None, None)?;
        let file = o.dir.path().join("for-ben.obfsync");
        assert!(write_bundle_now(&o.ann, &file, Some(&o.ben.device_id()?))?.is_some());
        // Nothing in it is sealed with a key Ben lacks.
        assert_eq!(import_bundle(&o.ben, &file)?.unreadable, 0);
        assert_eq!(texts(&o.ben)?, vec!["Buy milk"]);
        Ok(())
    }

    #[test]
    fn declining_keeps_nothing_no_entry_key_profile_or_photo() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let settings = o.ben.local_settings_with_prefix("")?;
        let entries = o.ben.all_entries()?.len();
        let offer = held(&o.ben, &o.file)?;
        let hash = Notes::new(&o.ann)
            .by_id(&o.note)?
            .ok_or("the Note")?
            .photos
            .first()
            .cloned()
            .ok_or("a photo")?;
        assert!(o.ben.photo(&hash).is_some());
        assert!(o.ben.decline_area_offer(&offer.id)?);
        assert!(o.ben.held_area_offers()?.is_empty());
        assert_eq!(o.ben.all_entries()?.len(), entries);
        assert_eq!(o.ben.local_settings_with_prefix("")?, settings);
        assert!(o.ben.photo(&hash).is_none());
        assert!(o.ben.pending_sets()?.is_empty());
        Ok(())
    }

    #[test]
    fn decline_and_block_blocks_the_sharer_keeps_their_name_and_nothing_of_the_area() -> TestResult
    {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let offer = held(&o.ben, &o.file)?;
        let hash = Notes::new(&o.ann)
            .by_id(&o.note)?
            .ok_or("the Note")?
            .photos
            .first()
            .cloned()
            .ok_or("a photo")?;
        assert!(o.ben.block_area_offer_sender(&offer.id, false)?);
        assert!(o.ben.is_blocked(&id_of(&o.ann)?)?);
        let names: Vec<String> = o
            .ben
            .blocked_people()?
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, vec!["Ann"]);
        assert!(o.ben.pending_sets()?.is_empty());
        assert!(o.ben.photo(&hash).is_none());
        assert!(Notes::new(&o.ben).by_id(&o.note)?.is_none());
        Ok(())
    }

    #[test]
    fn a_blocked_persons_offer_is_refused_before_its_preview_and_one_held_goes() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let offer = held(&o.ben, &o.file)?;
        o.ben.block_person(&id_of(&o.ann)?, false)?;
        assert!(o.ben.held_area_offer(&offer.id)?.is_none());
        assert!(o.ben.pending_sets()?.is_empty());
        assert!(is(import_bundle(&o.ben, &o.file), OfferError::FromBlocked));
        assert!(o.ben.pending_sets()?.is_empty());
        Ok(())
    }

    #[test]
    fn an_offer_of_an_area_held_here_as_its_own_is_refused() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        o.ben
            .append(&o.area, place_keys::TYPE, Some(place_keys::TYPE_AREA), None)?;
        assert!(is(import_bundle(&o.ben, &o.file), OfferError::Refused));
        assert!(o.ben.pending_sets()?.is_empty());
        Ok(())
    }

    #[test]
    fn an_offer_sealed_for_somebody_else_is_not_for_us() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let cat = device(4, "Cat")?;
        assert!(is(import_bundle(&cat, &o.file), OfferError::NotForUs));
        assert!(cat.pending_sets()?.is_empty());
        Ok(())
    }

    /// An offer of `area` from `from` to `to`, written by hand around
    /// `from`'s history of it, and the bundle's files.
    fn by_hand(from: &Store, area: &str, to: &Store) -> Made<(MemoryFolder, Vec<u8>)> {
        let mut folder = MemoryFolder::default();
        write_sealed(
            from,
            &mut folder,
            &from.area_history(area)?,
            &BTreeSet::new(),
            &mut random_nonce,
        )?;
        let key = to.own_key_record()?.verified_box_key().ok_or("a box key")?;
        let bytes = encode_area_offer(
            &from.area_offer_for(area, &id_of(to)?)?,
            &from.signing_key()?,
            &[key],
            &mut random_vec,
        )?;
        Ok((folder, bytes))
    }

    #[test]
    fn an_offer_that_grants_this_device_nothing_is_not_for_us() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let cat = device(4, "Cat")?;
        met(&o.ann, &cat)?;
        // Sealed for Cat, but the grants name only Ben.
        let (folder, bytes) = by_hand(&o.ann, &o.area, &cat)?;
        assert_eq!(
            cat.receive_area_offer(&folder, &bytes)?,
            Err(OfferError::NotForUs)
        );
        assert!(cat.pending_sets()?.is_empty());
        Ok(())
    }

    #[test]
    fn the_preview_names_the_area_as_its_managers_do() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        o.ben.accept_area_offer(&held(&o.ben, &o.file)?.id, false)?;
        // Ben, a Member, renames the Area: relayed, but it counts nowhere.
        o.ben
            .append(&o.area, place_keys::NAME, Some("Ben's"), None)?;
        let back = o.dir.path().join("back.obfsync");
        write_person_bundle_now(&o.ben, &back, &id_of(&o.ann)?)?;
        import_bundle(&o.ann, &back)?;
        let cat = device(4, "Cat")?;
        met(&o.ann, &cat)?;
        o.ann
            .share_area_with(&o.area, &id_of(&cat)?, AreaRole::Member, Transport::Bundle)?;
        let file = o.dir.path().join("cat.obfsync");
        write_area_offer_now(&o.ann, &file, &o.area, &id_of(&cat)?)?;
        assert_eq!(held(&cat, &file)?.area_name, "Household");
        Ok(())
    }

    #[test]
    fn an_offer_only_a_manager_signed_counts() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let cat = device(4, "Cat")?;
        met(&o.ann, &cat)?;
        o.ann
            .share_area_with(&o.area, &id_of(&cat)?, AreaRole::Member, Transport::Bundle)?;
        let file = o.dir.path().join("again.obfsync");
        write_area_offer_now(&o.ann, &file, &o.area, &id_of(&o.ben)?)?;
        o.ben.accept_area_offer(&held(&o.ben, &file)?.id, false)?;
        // Ben, a Member, passes the Area on to Cat in his own name: her
        // grant is real, but he may not offer.
        let (folder, bytes) = by_hand(&o.ben, &o.area, &cat)?;
        assert_eq!(
            cat.receive_area_offer(&folder, &bytes)?,
            Err(OfferError::NotForUs)
        );
        assert!(cat.pending_sets()?.is_empty());
        Ok(())
    }

    #[test]
    fn an_offer_signed_by_a_key_its_profile_does_not_bind_is_none() -> TestResult {
        let o = offered(AreaRole::Member, Transport::Bundle)?;
        let eve = SigningKey::from_seed([9; 32]);
        let offer = o.ann.area_offer_for(&o.area, &id_of(&o.ben)?)?;
        let forged = AreaOffer {
            from: KeyRecord::make(&eve),
            ..offer
        };
        let key = o
            .ben
            .own_key_record()?
            .verified_box_key()
            .ok_or("a box key")?;
        let bytes = encode_area_offer(&forged, &eve, &[key], &mut random_vec)?;
        assert_eq!(
            o.ben.receive_area_offer(&MemoryFolder::default(), &bytes)?,
            Err(OfferError::NotAnOffer)
        );
        Ok(())
    }

    #[test]
    fn an_offer_never_moves_the_users_own_notes_into_the_area() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        met(&ann, &ben)?;
        let mine = Notes::new(&ben).capture("Ben private", None, None)?;
        let house = Places::new(&ann).add_area("Household")?;
        Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
        // Ann's modified app files Ben's Note under her Area.
        ann.append(&mine.id, note_keys::PLACE, Some(&house.id), None)?;
        ann.append(&mine.id, note_keys::TEXT, Some("Taken"), None)?;
        ann.share_area_with(
            &house.id,
            &id_of(&ben)?,
            AreaRole::Manager,
            Transport::Bundle,
        )?;
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("offer.obfsync");
        write_area_offer_now(&ann, &file, &house.id, &id_of(&ben)?)?;
        ben.accept_area_offer(&held(&ben, &file)?.id, false)?;
        assert_eq!(ben.scope_of(&mine.id)?, crate::keys::PERSONAL_SCOPE);
        assert_eq!(
            Notes::new(&ben).by_id(&mine.id)?.ok_or("the Note")?.text,
            "Ben private"
        );
        Ok(())
    }

    #[test]
    fn an_offer_carries_keys_of_its_area_only() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        met(&ann, &ben)?;
        let house = Places::new(&ann).add_area("Household")?;
        let offer = ann.area_offer_for(&house.id, &id_of(&ben)?)?;
        assert_eq!(
            offer
                .area_keys
                .iter()
                .map(|k| k.scope.clone())
                .collect::<Vec<_>>(),
            vec![house.id.clone()]
        );
        let personal = ann
            .writing_key(crate::keys::PERSONAL_SCOPE)?
            .ok_or("a key")?;
        let mut widened = offer.clone();
        widened.area_keys.push(personal);
        assert!(AreaOffer::parse(&widened.encode()).is_none());
        assert_eq!(
            AreaOffer::parse(&offer.encode()).map(|o| o.encode()),
            Some(offer.encode())
        );
        Ok(())
    }

    #[test]
    fn sharing_needs_a_key_of_the_person_to_seal_for() -> TestResult {
        let ann = device(1, "Ann")?;
        let dan = device(5, "Dan")?;
        let house = Places::new(&ann).add_area("Household")?;
        ann.take_person(&dan.own_profile_for_others()?.ok_or("a Profile")?, None)?;
        assert!(
            ann.share_area_with(
                &house.id,
                &id_of(&dan)?,
                AreaRole::Member,
                Transport::Bundle
            )
            .is_err()
        );
        assert!(!ann.is_shared(&house.id)?);
        Ok(())
    }
}
