//! Areas and Topics as the Dart core's `Places` reads them: exactly two
//! levels.

use crate::Result;
use crate::store::{Store, utf16_cmp};

/// Field keys of Areas and Topics.
pub mod keys {
    pub const TYPE: &str = "$type";
    pub const TYPE_AREA: &str = "area";
    pub const TYPE_TOPIC: &str = "topic";
    pub const NAME: &str = "name";
    /// The Starter Set slug a place was planted from.
    pub const STARTER: &str = "starter";
    /// An Area's colour as `RRGGBB` hex.
    pub const COLOUR: &str = "colour";
    /// An Area's icon, one of the core's `areaIcons`.
    pub const ICON: &str = "icon";
    /// The Area a Topic lives in.
    pub const AREA: &str = "area";
}

/// The top level Notes are filed under (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    /// Entity id, `area:<id>`.
    pub id: String,
    /// The name as stored: typed by the user, or a starter's English name.
    pub name: String,
    /// The Starter Set slug, or `None` when the user made it.
    pub starter: Option<String>,
    pub colour: Option<String>,
    pub icon: Option<String>,
}

/// The second and last level, inside an Area (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    /// Entity id, `topic:<id>`.
    pub id: String,
    pub area_id: String,
    /// The name as stored: typed by the user, or a starter's English name.
    pub name: String,
    /// The Starter Set slug, or `None` when the user made it.
    pub starter: Option<String>,
}

/// One Area or Topic of the Starter Set, by slug and canonical English
/// name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarterPlace {
    pub slug: &'static str,
    pub name: &'static str,
}

/// The Starter Set: each Area with its Topics, as in the Dart core.
pub const STARTER_SET: [(StarterPlace, &[StarterPlace]); 4] = [
    (
        StarterPlace {
            slug: "household",
            name: "Household",
        },
        &[
            StarterPlace {
                slug: "kitchen",
                name: "Kitchen",
            },
            StarterPlace {
                slug: "garden",
                name: "Garden",
            },
            StarterPlace {
                slug: "cleaning",
                name: "Cleaning",
            },
        ],
    ),
    (
        StarterPlace {
            slug: "work",
            name: "Work",
        },
        &[
            StarterPlace {
                slug: "meetings",
                name: "Meetings",
            },
            StarterPlace {
                slug: "to-sort-out",
                name: "To sort out",
            },
        ],
    ),
    (
        StarterPlace {
            slug: "car",
            name: "Car",
        },
        &[
            StarterPlace {
                slug: "service",
                name: "Service",
            },
            StarterPlace {
                slug: "repairs",
                name: "Repairs",
            },
        ],
    ),
    (
        StarterPlace {
            slug: "health",
            name: "Health",
        },
        &[
            StarterPlace {
                slug: "doctor",
                name: "Doctor",
            },
            StarterPlace {
                slug: "medication",
                name: "Medication",
            },
        ],
    ),
];

/// The canonical English name of the Starter Set entry `slug`.
pub fn starter_name(slug: &str) -> Option<&'static str> {
    STARTER_SET
        .iter()
        .flat_map(|(area, topics)| std::iter::once(area).chain(topics.iter()))
        .find(|p| p.slug == slug)
        .map(|p| p.name)
}

/// Reading Areas and Topics.
pub struct Places<'a> {
    store: &'a Store,
}

impl<'a> Places<'a> {
    pub fn new(store: &'a Store) -> Places<'a> {
        Places { store }
    }

    /// The Area `id`, or `None` when there is none.
    pub fn area_by_id(&self, id: &str) -> Result<Option<Area>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_AREA) {
            return Ok(None);
        }
        let owned = |k| fields.get(k).map(str::to_string);
        Ok(Some(Area {
            id: id.to_string(),
            name: owned(keys::NAME).unwrap_or_default(),
            starter: owned(keys::STARTER),
            colour: owned(keys::COLOUR),
            icon: owned(keys::ICON),
        }))
    }

    /// The Topic `id`, or `None` when there is none or its Area is gone.
    pub fn topic_by_id(&self, id: &str) -> Result<Option<Topic>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_TOPIC) {
            return Ok(None);
        }
        let Some(area_id) = fields.get(keys::AREA) else {
            return Ok(None);
        };
        if self.area_by_id(area_id)?.is_none() {
            return Ok(None);
        }
        Ok(Some(Topic {
            id: id.to_string(),
            area_id: area_id.to_string(),
            name: fields.get(keys::NAME).unwrap_or_default().to_string(),
            starter: fields.get(keys::STARTER).map(str::to_string),
        }))
    }

    /// Every Area, by stored name.
    pub fn areas(&self) -> Result<Vec<Area>> {
        let mut areas = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_AREA)? {
            areas.extend(self.area_by_id(&id)?);
        }
        areas.sort_by(|a, b| utf16_cmp(&a.name, &b.name));
        Ok(areas)
    }

    /// The Topics of the Area `area_id`, by stored name.
    pub fn topics(&self, area_id: &str) -> Result<Vec<Topic>> {
        let mut topics = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_TOPIC)? {
            if self.store.current(&id, keys::AREA)?.as_deref() == Some(area_id) {
                topics.extend(self.topic_by_id(&id)?);
            }
        }
        topics.sort_by(|a, b| utf16_cmp(&a.name, &b.name));
        Ok(topics)
    }

    /// Whether `id` is an Area or a Topic that exists.
    pub fn is_place(&self, id: &str) -> Result<bool> {
        Ok(self.area_by_id(id)?.is_some() || self.topic_by_id(id)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_names_are_found_by_slug() {
        assert_eq!(starter_name("to-sort-out"), Some("To sort out"));
        assert_eq!(starter_name("health"), Some("Health"));
        assert_eq!(starter_name("nope"), None);
    }
}
