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

use crate::entry::Entry;
use crate::profile::{PICTURE_MAX_BYTES, Profile, sanitize_name};
use crate::store::Store;
use crate::{Error, Result};

/// The entity id's prefix, before the Profile id.
pub const PREFIX: &str = "override:";

/// The name the user gave the person, sanitised like a Profile name;
/// none or empty for none.
pub const NAME: &str = "overrideName";

/// The picture the user gave the person, a Profile picture (JPEG) in
/// base64; none for none.
pub const PICTURE: &str = "overridePicture";

/// The version of the Override's structure (ADR 0034): version 1 holds
/// [`NAME`] and [`PICTURE`]; it is the default and never written, so it
/// is absent.
pub const VERSION: &str = "overrideVersion";

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
    /// the own Profile; another name is refused while the Override
    /// [`Store::override_is_newer`].
    pub fn set_override_name(&self, id: &str, name: &str) -> Result<()> {
        if self.is_own_person(id)? {
            return Ok(());
        }
        let clean = sanitize_name(name);
        let value = (!clean.is_empty()).then_some(clean);
        if self.person_override(id)?.and_then(|o| o.name) == value {
            return Ok(());
        }
        self.refuse_newer_override(id)?;
        let entity = format!("{PREFIX}{}", self.resolve_person(id)?);
        self.append(&entity, NAME, value.as_deref(), None)?;
        Ok(())
    }

    /// Gives the person `id` the `picture` (a Profile picture) for the
    /// user alone; none takes it away, so their own shows again. Nothing
    /// for the own Profile; another picture is refused while the Override
    /// [`Store::override_is_newer`].
    pub fn set_override_picture(&self, id: &str, picture: Option<&[u8]>) -> Result<()> {
        if self.is_own_person(id)? {
            return Ok(());
        }
        if self.person_override(id)?.and_then(|o| o.picture).as_deref() == picture {
            return Ok(());
        }
        self.refuse_newer_override(id)?;
        let entity = format!("{PREFIX}{}", self.resolve_person(id)?);
        let value = picture.map(|p| BASE64.encode(p));
        self.append(&entity, PICTURE, value.as_deref(), None)?;
        Ok(())
    }

    /// Whether the Override of the person `id` is of a structure this
    /// version does not know: its [`VERSION`], as one of the user's own
    /// Devices wrote it last, anything but absent or `1`. It shows as far
    /// as it can and is changed only by a newer version of the app (ADR
    /// 0034); what another person's Device wrote counts for nothing here
    /// either.
    pub fn override_is_newer(&self, id: &str) -> Result<bool> {
        if self.is_own_person(id)? {
            return Ok(false);
        }
        let entity = format!("{PREFIX}{}", self.resolve_person(id)?);
        let version = self.own_current(&entity, VERSION, &mut HashMap::new())?;
        Ok(!matches!(version.as_deref(), None | Some("1")))
    }

    fn refuse_newer_override(&self, id: &str) -> Result<()> {
        if self.override_is_newer(id)? {
            return Err(Error::Invalid(format!(
                "the Override of {id} is of a structure not known here"
            )));
        }
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
mod tests;
