//! Overrides, as the Dart core's `overrides.dart` keeps them (ADR 0017,
//! #52): a name and a picture the user gives another person's Profile
//! for themselves alone, such as "Mum". Kept in the log as the entity
//! `override:<profile id>`, outside every Area, so it is sealed under the
//! personal key and goes only to the user's own Devices. Only what one's
//! own Devices wrote counts: an entry another person's Device slipped
//! into the log changes nothing.

use std::cmp::Ordering;
use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::Result;
use crate::entry::Entry;
use crate::profile::{PICTURE_MAX_BYTES, Profile, sanitize_name};
use crate::store::Store;

/// The entity id's prefix, before the Profile id.
pub const PREFIX: &str = "override:";

/// The name the user gave the person, sanitised like a Profile name;
/// none or empty for none.
pub const NAME: &str = "overrideName";

/// The picture the user gave the person, a Profile picture (JPEG) in
/// base64; none for none.
pub const PICTURE: &str = "overridePicture";

/// The name and picture the user gave another person; none where they
/// gave none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonOverride {
    pub name: Option<String>,
    pub picture: Option<Vec<u8>>,
}

impl Store {
    /// The Override of the person `id`, through their aliases; none when
    /// the user gave them neither name nor picture, or `id` is the own
    /// Profile.
    pub fn person_override(&self, id: &str) -> Result<Option<PersonOverride>> {
        if self.is_own_person(id)? {
            return Ok(None);
        }
        let entity = format!("{PREFIX}{}", self.resolve_person(id)?);
        let mut name: Option<Entry> = None;
        let mut picture: Option<Entry> = None;
        let mut own: HashMap<String, bool> = HashMap::new();
        for e in self.history(&entity)? {
            let is_name = e.field == NAME;
            if !is_name && e.field != PICTURE {
                continue;
            }
            let by_own = match own.get(&e.device) {
                Some(known) => *known,
                None => {
                    let known = self.written_by_own(&e.device)?;
                    own.insert(e.device.clone(), known);
                    known
                }
            };
            if !by_own {
                continue;
            }
            let held = if is_name { &mut name } else { &mut picture };
            if held
                .as_ref()
                .is_some_and(|h| Store::latest_wins(&e, h) != Ordering::Greater)
            {
                continue;
            }
            *held = Some(e);
        }
        let shown_name = sanitize_name(name.and_then(|e| e.value).as_deref().unwrap_or(""));
        let shown_picture = override_picture(picture.and_then(|e| e.value).as_deref());
        if shown_name.is_empty() && shown_picture.is_none() {
            return Ok(None);
        }
        Ok(Some(PersonOverride {
            name: (!shown_name.is_empty()).then_some(shown_name),
            picture: shown_picture,
        }))
    }

    /// Gives the person `id` the `name` for the user alone, sanitised; an
    /// empty name takes it away, so their own shows again. Nothing for
    /// the own Profile.
    pub fn set_override_name(&self, id: &str, name: &str) -> Result<()> {
        if self.is_own_person(id)? {
            return Ok(());
        }
        let clean = sanitize_name(name);
        let value = (!clean.is_empty()).then_some(clean);
        if self.person_override(id)?.and_then(|o| o.name) == value {
            return Ok(());
        }
        let entity = format!("{PREFIX}{}", self.resolve_person(id)?);
        self.append(&entity, NAME, value.as_deref(), None)?;
        Ok(())
    }

    /// Gives the person `id` the `picture` (a Profile picture) for the
    /// user alone; none takes it away, so their own shows again. Nothing
    /// for the own Profile.
    pub fn set_override_picture(&self, id: &str, picture: Option<&[u8]>) -> Result<()> {
        if self.is_own_person(id)? {
            return Ok(());
        }
        if self.person_override(id)?.and_then(|o| o.picture).as_deref() == picture {
            return Ok(());
        }
        let entity = format!("{PREFIX}{}", self.resolve_person(id)?);
        let value = picture.map(|p| BASE64.encode(p));
        self.append(&entity, PICTURE, value.as_deref(), None)?;
        Ok(())
    }

    /// `profile` as the user sees it: the name and picture of its
    /// Override, where they gave one.
    pub(crate) fn with_override(&self, profile: Profile) -> Result<Profile> {
        let Some(o) = self.person_override(&profile.id)? else {
            return Ok(profile);
        };
        Ok(Profile {
            name: o.name.unwrap_or(profile.name),
            picture: o.picture.or(profile.picture),
            ..profile
        })
    }

    /// The newest value of `field` on `entity` that one of the user's own
    /// Devices wrote (`written_by_own`); none when none did. In a Shared
    /// Area another person's Device can write any field of a Note, so a
    /// link one's own Devices keep is read this way. `own` remembers,
    /// across calls, which Devices are one's own. Read from the log alone:
    /// a pending change (ADR 0022) is trusted for nothing.
    pub(crate) fn own_current(
        &self,
        entity: &str,
        field: &str,
        own: &mut HashMap<String, bool>,
    ) -> Result<Option<String>> {
        let mut held: Option<Entry> = None;
        for e in self.logged_history(entity)? {
            if e.field != field || !self.written_by_own_in(&e.device, own)? {
                continue;
            }
            if held
                .as_ref()
                .is_none_or(|h| Store::latest_wins(&e, h) == Ordering::Greater)
            {
                held = Some(e);
            }
        }
        Ok(held.and_then(|e| e.value))
    }

    /// `written_by_own`, remembered in `own`.
    pub(crate) fn written_by_own_in(
        &self,
        device: &str,
        own: &mut HashMap<String, bool>,
    ) -> Result<bool> {
        if let Some(known) = own.get(device) {
            return Ok(*known);
        }
        let known = self.written_by_own(device)?;
        own.insert(device.to_string(), known);
        Ok(known)
    }

    /// Whether the Device `device` is one of the user's own: this one,
    /// one paired as its own, or one whose key is bound to the own
    /// Profile.
    pub(crate) fn written_by_own(&self, device: &str) -> Result<bool> {
        if device == self.device_id()? || self.is_own_device(device)? {
            return Ok(true);
        }
        Ok(match self.pinned_key(device)? {
            Some(key) => self.own_bound(&BASE64.encode(key.record.public_key))?,
            None => false,
        })
    }

    /// People (#52): every Profile of another person kept here, as the
    /// user sees them, Overrides applied, by name; a Profile another one
    /// kept here replaced shows only as that one, and a blocked person
    /// only among [`Store::blocked_people`].
    pub fn people_shown(&self) -> Result<Vec<Profile>> {
        let blocked = self.blocked_ids()?;
        let mut shown = Vec::new();
        for kept in self.kept_people()? {
            if self.resolve_person(&kept.id)? == kept.id && !blocked.contains(&kept.id) {
                shown.push(self.person_shown(&kept.id)?);
            }
        }
        shown.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        Ok(shown)
    }
}

/// The picture an Override keeps in `value`; none for none, and for
/// anything that is no standard base64 of a JPEG within
/// [`PICTURE_MAX_BYTES`].
fn override_picture(value: Option<&str>) -> Option<Vec<u8>> {
    let value = value.filter(|v| !v.is_empty())?;
    let bytes = BASE64.decode(value).ok()?;
    if BASE64.encode(&bytes) != value
        || bytes.len() > PICTURE_MAX_BYTES
        || !bytes.starts_with(&[0xFF, 0xD8, 0xFF])
    {
        return None;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::folder::{MemoryFolder, folder_sync, random_nonce};
    use crate::keys::{PERSONAL_SCOPE, sealed_as, unseal};
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer, apply_answer_as, sync_request,
        sync_request_as,
    };
    use crate::shared::{AreaRole, Invite};
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::ticking;
    use crate::{Notes, OwnProfile, PairedDevice, Places};
    use serde_json::Value;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// The name the tests give Ben for Ann alone: whatever carries it is
    /// easy to find.
    const MARKER: &str = "Mum-Marker";

    /// A Profile picture the Dart core made.
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

    /// Ann's Household shared with Ben as a Member, Ann's desk one of her
    /// own Devices.
    struct Household {
        ann: Store,
        desk: Store,
        ben: Store,
        area: String,
    }

    fn household() -> Made<Household> {
        let ann = device(1, "Ann")?;
        let desk = device(3, "Ann at the desk")?;
        let ben = device(2, "Ben")?;
        exchange(&desk, &ann, false)?;
        remember(&ann, &desk, false)?;
        remember(&desk, &ann, false)?;
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
        exchange(&desk, &ann, false)?;
        Ok(Household {
            ann,
            desk,
            ben,
            area,
        })
    }

    fn overrides(store: &Store) -> Made<usize> {
        Ok(store
            .all_entries()?
            .iter()
            .filter(|e| e.entity.starts_with(PREFIX))
            .count())
    }

    #[test]
    fn every_view_shows_the_override_and_the_profile_view_their_own() -> TestResult {
        let h = household()?;
        let ben_id = own_id(&h.ben)?;
        h.ann.set_override_name(&ben_id, MARKER)?;
        h.ann.set_override_picture(&ben_id, Some(JPEG))?;
        assert_eq!(h.ann.person_shown(&ben_id)?.name, MARKER);
        assert_eq!(h.ann.person_shown(&ben_id)?.picture.as_deref(), Some(JPEG));
        assert_eq!(h.ann.person_theirs(&ben_id)?.name, "Ben");
        assert_eq!(h.ann.person_theirs(&ben_id)?.picture, None);
        let mut names = Vec::new();
        for p in h.ann.people(&h.area)? {
            names.push(h.ann.person_shown(&p.id)?.name);
        }
        assert_eq!(names, ["Ann", MARKER]);
        let shown: Vec<String> = h.ann.people_shown()?.into_iter().map(|p| p.name).collect();
        assert_eq!(shown, [MARKER]);
        let note = Notes::new(&h.ben).capture("Milk", Some(&h.area), None)?;
        exchange(&h.ben, &h.ann, true)?;
        let author = h.ann.author_of(&note.id)?.ok_or("an author")?;
        assert_eq!(author.name, MARKER);
        assert_eq!(author.picture.as_deref(), Some(JPEG));
        h.ann.set_override_picture(&ben_id, None)?;
        assert_eq!(h.ann.person_shown(&ben_id)?.picture, None);
        h.ann.set_override_name(&ben_id, "  ")?;
        assert_eq!(h.ann.person_override(&ben_id)?, None);
        assert_eq!(h.ann.person_shown(&ben_id)?.name, "Ben");
        Ok(())
    }

    #[test]
    fn a_name_is_sanitised_and_the_same_again_writes_nothing() -> TestResult {
        let h = household()?;
        let ben_id = own_id(&h.ben)?;
        h.ann
            .set_override_name(&ben_id, " \u{202e}M\u{200b}um\u{7} ")?;
        assert_eq!(h.ann.person_shown(&ben_id)?.name, "Mum");
        let count = h.ann.all_entries()?.len();
        h.ann.set_override_name(&ben_id, "Mum ")?;
        assert_eq!(h.ann.all_entries()?.len(), count);
        // The own Profile takes none.
        h.ann.set_override_name(&own_id(&h.ann)?, "Me")?;
        assert_eq!(h.ann.all_entries()?.len(), count);
        Ok(())
    }

    #[test]
    fn a_picture_that_is_no_jpeg_or_too_large_shows_none() -> TestResult {
        let h = household()?;
        let ben_id = own_id(&h.ben)?;
        let entity = format!("{PREFIX}{ben_id}");
        let png = BASE64.encode([0x89, 0x50, 0x4E, 0x47, 0, 0]);
        h.ann.append(&entity, PICTURE, Some(&png), None)?;
        assert_eq!(h.ann.person_shown(&ben_id)?.picture, None);
        let mut large = vec![0u8; PICTURE_MAX_BYTES + 1];
        large[..3].copy_from_slice(&[0xFF, 0xD8, 0xFF]);
        h.ann
            .append(&entity, PICTURE, Some(&BASE64.encode(&large)), None)?;
        assert_eq!(h.ann.person_shown(&ben_id)?.picture, None);
        h.ann.append(&entity, PICTURE, Some("not base64!"), None)?;
        assert_eq!(h.ann.person_shown(&ben_id)?.picture, None);
        Ok(())
    }

    #[test]
    fn an_override_another_persons_device_wrote_counts_for_nothing() -> TestResult {
        let h = household()?;
        let ben_id = own_id(&h.ben)?;
        let forged = h
            .ben
            .append(&format!("{PREFIX}{ben_id}"), NAME, Some("Mallory"), None)?;
        let mut report = ImportReport::default();
        h.ann.apply_entries(
            &[forged.wire()],
            &[],
            Some(&h.ben.device_id()?),
            &mut report,
        )?;
        assert_eq!(overrides(&h.ann)?, 1);
        assert_eq!(h.ann.person_shown(&ben_id)?.name, "Ben");
        assert_eq!(h.ann.person_override(&ben_id)?, None);
        Ok(())
    }

    #[test]
    fn a_sent_item_shows_the_override_only_for_the_profile_kept() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let ben_id = own_id(&ben)?;
        let note = Notes::new(&ben).capture("Dentist", None, None)?;
        let file = ben.send_note(&note.id, None, false)?;
        ann.set_override_name(&ben_id, MARKER)?;
        let opened = ann.open_sent_item(&file)?.map_err(|e| format!("{e:?}"))?;
        assert_eq!(opened.sender.name, "Ben");
        ann.accept_sent_item(&opened.item, false)?;
        let again = ann.open_sent_item(&file)?.map_err(|e| format!("{e:?}"))?;
        assert_eq!(again.sender.name, MARKER);
        Ok(())
    }

    #[test]
    fn ones_own_devices_get_it_by_lan_folder_and_bundle() -> TestResult {
        let h = household()?;
        let ben_id = own_id(&h.ben)?;
        h.ann.set_override_name(&ben_id, MARKER)?;
        h.ann.set_override_picture(&ben_id, Some(JPEG))?;
        // A bundle.
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("ann.obfsync");
        crate::bundle::write_bundle_now(&h.ann, &path, Some(&h.desk.device_id()?))?;
        crate::bundle::import_bundle(&h.desk, &path)?;
        assert_eq!(h.desk.person_shown(&ben_id)?.name, MARKER);
        // A LAN session, both ways.
        h.desk.set_override_name(&ben_id, "Mum")?;
        exchange(&h.desk, &h.ann, false)?;
        assert_eq!(h.ann.person_shown(&ben_id)?.name, "Mum");
        assert_eq!(h.desk.person_shown(&ben_id)?.picture.as_deref(), Some(JPEG));
        // The shared folder.
        h.ann.set_override_name(&ben_id, "Mother")?;
        let mut folder = MemoryFolder::default();
        folder_sync(&h.ann, &mut folder, &mut random_nonce)?;
        folder_sync(&h.desk, &mut folder, &mut random_nonce)?;
        assert_eq!(h.desk.person_shown(&ben_id)?.name, "Mother");
        Ok(())
    }

    #[test]
    fn a_lan_session_with_another_persons_device_carries_none() -> TestResult {
        let h = household()?;
        let ben_id = own_id(&h.ben)?;
        h.ann.set_override_name(&ben_id, MARKER)?;
        h.ann.set_override_picture(&ben_id, Some(JPEG))?;
        Notes::new(&h.ann).capture("Written since", None, None)?;
        let (request, answer) = exchange(&h.ben, &h.ann, true)?;
        let (back_request, back_answer) = exchange(&h.ann, &h.ben, true)?;
        for wire in [request, answer, back_request, back_answer] {
            assert!(!wire.to_string().contains(MARKER));
        }
        assert_eq!(overrides(&h.ben)?, 0);
        Ok(())
    }

    #[test]
    fn the_shared_folder_holds_none_another_person_can_open() -> TestResult {
        let h = household()?;
        h.ann.set_override_name(&own_id(&h.ben)?, MARKER)?;
        Notes::new(&h.ann).capture("Milk", Some(&h.area), None)?;
        let mut folder = MemoryFolder::default();
        folder_sync(&h.ann, &mut folder, &mut random_nonce)?;
        folder_sync(&h.ben, &mut folder, &mut random_nonce)?;
        assert_eq!(overrides(&h.ben)?, 0);
        let personal: Vec<String> = h
            .ann
            .area_keys()?
            .into_iter()
            .filter(|k| k.scope == PERSONAL_SCOPE)
            .map(|k| k.id())
            .collect();
        let keys = h.ben.area_keys()?;
        assert!(keys.iter().all(|k| !personal.contains(&k.id())));
        let mut opened = 0;
        for files in folder.dirs.values() {
            for bytes in files.values() {
                for line in String::from_utf8_lossy(bytes).lines() {
                    let Ok(sealed) = BASE64.decode(line.trim()) else {
                        continue;
                    };
                    for key in &keys {
                        if let Some(plain) = unseal(&key.key, &sealed, sealed_as::ENTRY) {
                            opened += 1;
                            assert!(!String::from_utf8_lossy(&plain).contains(MARKER));
                        }
                    }
                }
            }
        }
        assert!(opened > 0);
        Ok(())
    }

    #[test]
    fn a_bundle_for_another_person_carries_none() -> TestResult {
        let h = household()?;
        h.ann.set_override_name(&own_id(&h.ben)?, MARKER)?;
        Notes::new(&h.ann).capture("Milk", Some(&h.area), None)?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("ann.obfsync");
        let wrote = crate::bundle::write_bundle_now(&h.ann, &path, Some(&h.ben.device_id()?))?;
        assert!(wrote.is_some());
        crate::bundle::import_bundle(&h.ben, &path)?;
        let texts: Vec<String> = Notes::new(&h.ben)
            .all()?
            .into_iter()
            .map(|n| n.text)
            .collect();
        assert!(texts.contains(&"Milk".to_string()));
        assert_eq!(overrides(&h.ben)?, 0);
        Ok(())
    }

    #[test]
    fn a_sent_item_carries_none() -> TestResult {
        let h = household()?;
        h.ann.set_override_name(&own_id(&h.ben)?, MARKER)?;
        let note = Notes::new(&h.ann).capture("Dentist", None, None)?;
        let file = h.ann.send_note(&note.id, None, false)?;
        assert!(!String::from_utf8_lossy(&file).contains(MARKER));
        let cid = device(4, "Cid")?;
        let opened = cid.open_sent_item(&file)?.map_err(|e| format!("{e:?}"))?;
        cid.accept_sent_item(&opened.item, false)?;
        assert_eq!(overrides(&cid)?, 0);
        Ok(())
    }
}
