//! The settings of the person rather than of this Device (#211), under
//! the keys and in the forms the Dart core's `person_settings.dart` uses:
//! kept in the log like an Override, so they are the same on all of the
//! user's own Devices and never reach another person. What another
//! person's Device wrote counts for nothing. The per-Device ones are
//! [`crate::DeviceSettings`].

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::places::{Area, Places};
use crate::search::lower_case;
use crate::store::{Store, utf16_cmp};
use crate::{Error, Result};

/// Where the person's own settings lie in the log.
pub mod keys {
    /// The entity holding them, outside every Area like an Override:
    /// sealed under the personal key, it goes to the user's own Devices
    /// only, and only what they wrote counts.
    pub const ENTITY: &str = "settings:person";
    /// "Move a Note to the Trash when its one-time Reminder is done":
    /// [`ON`] while it is on; absent, null or any other value for off.
    pub const TRASH_DONE_REMINDER: &str = "trashDoneReminder";
    pub const ON: &str = "on";
    /// Simple or Advanced (#197): [`SIMPLE`] or [`ADVANCED`] once
    /// chosen; absent, null or any other value reads as Advanced, so
    /// nothing is hidden from someone who never chose.
    pub const FEATURES: &str = "features";
    pub const SIMPLE: &str = "simple";
    pub const ADVANCED: &str = "advanced";
    /// The person's own order of the Areas (#280): a JSON list of Area
    /// ids, first to last. Ids that are no Area here, repeats and
    /// anything not a string are skipped; absent, null or anything not a
    /// list reads as no order, so the Areas show by name.
    pub const AREA_ORDER: &str = "areaOrder";
}

/// The person's own settings in the log.
pub struct PersonSettings<'a> {
    store: &'a Store,
}

impl<'a> PersonSettings<'a> {
    pub fn new(store: &'a Store) -> PersonSettings<'a> {
        PersonSettings { store }
    }

    /// Whether Done on a one-time Reminder that is the only Routine or
    /// Reminder of a plain Note moves the Note to the trash with it
    /// ([`crate::Routines::check_off`]). Off until switched on; a value
    /// no app writes reads as off, so nothing is trashed by it.
    pub fn trashes_done_reminder(&self) -> Result<bool> {
        let value =
            self.store
                .own_current(keys::ENTITY, keys::TRASH_DONE_REMINDER, &mut HashMap::new())?;
        Ok(value.as_deref() == Some(keys::ON))
    }

    /// Switches it on or off for all of the user's own Devices; the same
    /// again writes nothing.
    pub fn set_trashes_done_reminder(&self, on: bool) -> Result<()> {
        if self.trashes_done_reminder()? == on {
            return Ok(());
        }
        let value = on.then_some(keys::ON);
        self.store
            .append(keys::ENTITY, keys::TRASH_DONE_REMINDER, value, None)
            .map(drop)
    }

    /// Whether making Tallies, Sessions, Frogs and Topics, the format bar
    /// and Shared Area roles are offered (#197). Only [`keys::SIMPLE`]
    /// hides them; never chosen, or a value no app writes, reads as
    /// Advanced. What exists stays visible and usable either way.
    pub fn advanced(&self) -> Result<bool> {
        Ok(self.features()?.as_deref() != Some(keys::SIMPLE))
    }

    /// Chooses Simple or Advanced for all of the user's own Devices; the
    /// same value stored again writes nothing, while Advanced chosen
    /// where none was stored is written, so the choice is recorded.
    pub fn set_advanced(&self, on: bool) -> Result<()> {
        let value = if on { keys::ADVANCED } else { keys::SIMPLE };
        if self.features()?.as_deref() == Some(value) {
            return Ok(());
        }
        self.store
            .append(keys::ENTITY, keys::FEATURES, Some(value), None)
            .map(drop)
    }

    fn features(&self) -> Result<Option<String>> {
        self.store
            .own_current(keys::ENTITY, keys::FEATURES, &mut HashMap::new())
    }

    /// Every Area in the person's own order (#280), as
    /// [`Self::areas_in_order_by`] lists them by stored name.
    pub fn areas_in_order(&self) -> Result<Vec<Area>> {
        self.areas_in_order_by(stored_name)
    }

    /// Every Area in the person's own order (#280), as the Areas list and
    /// the sidebar show them: those in the stored order first, as it has
    /// them, then the Areas missing from it (a new one, a Shared Area
    /// joined later) by the name `shown` gives each, the one the app
    /// shows, in lower case; equal names go by id. With no order stored,
    /// all of them by that name. As the Dart core's `areasInOrder`.
    pub fn areas_in_order_by(&self, shown: impl Fn(&Area) -> String) -> Result<Vec<Area>> {
        self.ordered(&shown)
    }

    /// Puts the Area `area_id` at `index` of [`Self::areas_in_order`], as
    /// [`Self::move_area_by`] by stored name.
    pub fn move_area(&self, area_id: &str, index: usize) -> Result<()> {
        self.move_area_by(area_id, index, stored_name)
    }

    /// Puts the Area `area_id` at `index` of [`Self::areas_in_order_by`]
    /// by `shown`, counted after it was taken out, for all of the user's
    /// own Devices. An index past the end puts it last. The whole order
    /// is stored, so a new Area goes to the end. Refused when `area_id` is
    /// no Area.
    pub fn move_area_by(
        &self,
        area_id: &str,
        index: usize,
        shown: impl Fn(&Area) -> String,
    ) -> Result<()> {
        let mut ids = self.shown_area_ids(&shown)?;
        let Some(at) = ids.iter().position(|id| id == area_id) else {
            return Err(Error::Invalid(format!("not an Area: {area_id}")));
        };
        let id = ids.remove(at);
        ids.insert(index.min(ids.len()), id);
        self.store_area_order(&ids, &shown)
    }

    /// Puts the Areas in the order of `area_ids`, as
    /// [`Self::set_area_order_by`] by stored name.
    pub fn set_area_order<S: AsRef<str>>(&self, area_ids: &[S]) -> Result<()> {
        self.set_area_order_by(area_ids, stored_name)
    }

    /// Puts the Areas in the order of `area_ids` for all of the user's
    /// own Devices; the Areas it leaves out follow in the order
    /// [`Self::areas_in_order_by`] by `shown` had them. Ids that are no
    /// Area are skipped.
    pub fn set_area_order_by<S: AsRef<str>>(
        &self,
        area_ids: &[S],
        shown: impl Fn(&Area) -> String,
    ) -> Result<()> {
        let had = self.shown_area_ids(&shown)?;
        let mut ids: Vec<String> = Vec::new();
        for id in area_ids.iter().map(AsRef::as_ref) {
            if had.iter().any(|s| s == id) && !ids.iter().any(|i| i == id) {
                ids.push(id.to_string());
            }
        }
        for id in had {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        self.store_area_order(&ids, &shown)
    }

    /// Puts the Area `to` where `from` stands in the stored order, as a
    /// starter Area shared under a new id (#280). Nothing when `from` is
    /// not in it: `to` is then missing from it too, in the same place by
    /// name. As the Dart core's `putAreaInPlaceOf`.
    pub fn put_area_in_place_of(&self, from: &str, to: &str) -> Result<()> {
        let held = self.held_area_order()?;
        if !held.iter().any(|id| id == from) {
            return Ok(());
        }
        let areas: Vec<String> = Places::new(self.store)
            .areas()?
            .into_iter()
            .map(|a| a.id)
            .collect();
        let mut ids: Vec<String> = Vec::new();
        for id in held {
            let id = if id == from { to.to_string() } else { id };
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids.retain(|id| areas.contains(id));
        let value = json!(ids).to_string();
        self.store
            .append(keys::ENTITY, keys::AREA_ORDER, Some(&value), None)
            .map(drop)
    }

    fn ordered(&self, shown: &dyn Fn(&Area) -> String) -> Result<Vec<Area>> {
        let mut by_name: Vec<Option<Area>> = Places::new(self.store)
            .areas()?
            .into_iter()
            .map(Some)
            .collect();
        let mut ordered = Vec::new();
        for id in self.held_area_order()? {
            let slot = by_name
                .iter_mut()
                .find(|a| a.as_ref().is_some_and(|a| a.id == id));
            ordered.extend(slot.and_then(Option::take));
        }
        ordered.extend(by_shown_name(by_name.into_iter().flatten(), shown));
        Ok(ordered)
    }

    fn shown_area_ids(&self, shown: &dyn Fn(&Area) -> String) -> Result<Vec<String>> {
        Ok(self.ordered(shown)?.into_iter().map(|a| a.id).collect())
    }

    /// Stores `ids` as the order; the order already shown by `shown`
    /// writes nothing.
    fn store_area_order(&self, ids: &[String], shown: &dyn Fn(&Area) -> String) -> Result<()> {
        if self.shown_area_ids(shown)? == ids {
            return Ok(());
        }
        let value = json!(ids).to_string();
        self.store
            .append(keys::ENTITY, keys::AREA_ORDER, Some(&value), None)
            .map(drop)
    }

    /// The ids of the stored order, as written by the user's own Devices,
    /// the newest such entry winning; empty when there is none or it is
    /// not a list.
    fn held_area_order(&self) -> Result<Vec<String>> {
        let held = self
            .store
            .own_current(keys::ENTITY, keys::AREA_ORDER, &mut HashMap::new())?;
        let Some(Ok(Value::Array(list))) = held.map(|v| serde_json::from_str::<Value>(&v)) else {
            return Ok(Vec::new());
        };
        Ok(list
            .into_iter()
            .filter_map(|v| match v {
                Value::String(id) => Some(id),
                _ => None,
            })
            .collect())
    }
}

/// The stored name of `area`, for the calls that take no name shown.
fn stored_name(area: &Area) -> String {
    area.name.clone()
}

/// `areas` by the name `shown` gives each, in lower case as Dart's
/// `toLowerCase` has it and compared by UTF-16 code units, as the apps
/// listed the Areas before #280; equal names go by id, so the order is
/// the same every time and in both cores.
fn by_shown_name(
    areas: impl IntoIterator<Item = Area>,
    shown: &dyn Fn(&Area) -> String,
) -> Vec<Area> {
    let mut keyed: Vec<(Area, String)> = areas
        .into_iter()
        .map(|area| {
            let key = lower_case(&shown(&area));
            (area, key)
        })
        .collect();
    keyed.sort_by(|a, b| utf16_cmp(&a.1, &b.1).then_with(|| utf16_cmp(&a.0.id, &b.0.id)));
    keyed.into_iter().map(|(area, _)| area).collect()
}

#[cfg(test)]
mod tests;
