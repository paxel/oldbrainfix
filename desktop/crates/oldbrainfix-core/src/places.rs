//! Areas and Topics as the Dart core's `Places` reads and writes them:
//! exactly two levels.

use chrono::{DateTime, Utc};

use crate::notes::{Notes, keys as note_keys};
use crate::store::{Store, utf16_cmp};
use crate::{Error, Result};

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

/// The icons an Area can carry, as keys every front end draws its own
/// way: the Dart core's `areaIcons`.
pub const AREA_ICONS: [&str; 14] = [
    "home", "work", "car", "health", "school", "kids", "kitchen", "garden", "shopping", "money",
    "pets", "sport", "travel", "star",
];

/// The id every Device plants the starter place `slug` of `ty` under.
pub fn starter_id(ty: &str, slug: &str) -> String {
    format!("{ty}:starter-{slug}")
}

/// The date of every planting, 2000-01-01: before anything a user does.
pub fn starter_date() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(946_684_800, 0).unwrap_or_default()
}

/// Where a deleted place's Notes go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesGo {
    /// Back to Unsorted.
    Unsorted,
    /// Into the deleted Topic's Area.
    Area,
    /// Into the trash.
    Trash,
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

    /// Creates an Area named `name`.
    pub fn add_area(&self, name: &str) -> Result<Area> {
        let name = Self::name(name)?;
        let id = self.store.batch(|| self.append_area(name, None))?;
        self.area_by_id(&id)?
            .ok_or_else(|| Error::Invalid("the new Area is gone".into()))
    }

    /// Creates a Topic named `name` inside the Area `area_id`. Only an
    /// Area holds Topics: there is no third level.
    pub fn add_topic(&self, area_id: &str, name: &str) -> Result<Topic> {
        self.require_area(area_id)?;
        if !self.store.may_manage(Some(area_id))? {
            return Err(Error::Invalid(format!("not a Manager of {area_id}")));
        }
        let name = Self::name(name)?;
        let id = self
            .store
            .batch(|| self.append_topic(area_id, name, None))?;
        self.topic_by_id(&id)?
            .ok_or_else(|| Error::Invalid("the new Topic is gone".into()))
    }

    /// Renames the Area or Topic `id`.
    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        let name = Self::name(name)?;
        if !self.is_place(id)? {
            return Err(Error::Invalid(format!("not an Area or Topic: {id}")));
        }
        self.store
            .append(id, keys::NAME, Some(name), None)
            .map(drop)
    }

    /// Sets or clears the colour (`RRGGBB`) of the Area `area_id`.
    pub fn set_colour(&self, area_id: &str, colour: Option<&str>) -> Result<()> {
        self.require_area(area_id)?;
        self.store
            .append(area_id, keys::COLOUR, colour, None)
            .map(drop)
    }

    /// Sets or clears the icon of the Area `area_id`, one of
    /// [`AREA_ICONS`].
    pub fn set_icon(&self, area_id: &str, icon: Option<&str>) -> Result<()> {
        self.require_area(area_id)?;
        if let Some(icon) = icon
            && !AREA_ICONS.contains(&icon)
        {
            return Err(Error::Invalid(format!("not an Area icon: {icon}")));
        }
        self.store.append(area_id, keys::ICON, icon, None).map(drop)
    }

    /// How many Notes sit in `id`; for an Area, its Topics' Notes count
    /// too.
    pub fn notes_inside(&self, id: &str) -> Result<usize> {
        Ok(Notes::new(self.store).inside(id)?.len())
    }

    /// Deletes the Topic `topic_id`; its Notes go where `notes_go` says.
    pub fn delete_topic(&self, topic_id: &str, notes_go: NotesGo) -> Result<()> {
        let topic = self
            .topic_by_id(topic_id)?
            .ok_or_else(|| Error::Invalid(format!("not a Topic: {topic_id}")))?;
        self.store.batch(|| {
            self.move_notes(topic_id, notes_go, Some(&topic.area_id))?;
            self.store.append(topic_id, keys::TYPE, None, None)?;
            Ok(())
        })
    }

    /// Deletes the Area `area_id` with its Topics; their Notes go where
    /// `notes_go` says: back to Unsorted or into the trash.
    pub fn delete_area(&self, area_id: &str, notes_go: NotesGo) -> Result<()> {
        self.require_area(area_id)?;
        if notes_go == NotesGo::Area {
            return Err(Error::Invalid("an Area has no Area".into()));
        }
        self.store.batch(|| {
            self.move_notes(area_id, notes_go, None)?;
            for topic in self.topics(area_id)? {
                self.store.append(&topic.id, keys::TYPE, None, None)?;
            }
            self.store.append(area_id, keys::TYPE, None, None)?;
            Ok(())
        })
    }

    /// Plants the Starter Set entries whose slugs are in `picked`. A
    /// Topic is planted only together with its Area.
    ///
    /// Every Device plants a starter Area or Topic under the same id
    /// ([`starter_id`]) and dated [`starter_date`], as the Dart core does:
    /// two Devices that both planted it hold one place after a sync, and
    /// whatever the user did to it is dated later and wins.
    pub fn plant_starter_set(&self, picked: &[&str]) -> Result<()> {
        self.store.batch(|| {
            for (area, topics) in STARTER_SET {
                if !picked.contains(&area.slug) {
                    continue;
                }
                let area_id = starter_id(keys::TYPE_AREA, area.slug);
                self.plant(&area_id, keys::TYPE_AREA, area.name, area.slug)?;
                for topic in topics {
                    if !picked.contains(&topic.slug) {
                        continue;
                    }
                    let topic_id = starter_id(keys::TYPE_TOPIC, topic.slug);
                    if self.plant(&topic_id, keys::TYPE_TOPIC, topic.name, topic.slug)? {
                        self.store.append(
                            &topic_id,
                            keys::AREA,
                            Some(&area_id),
                            Some(starter_date()),
                        )?;
                    }
                }
            }
            Ok(())
        })
    }

    /// Writes the starter place `id` unless it is held already; returns
    /// whether it wrote it.
    fn plant(&self, id: &str, ty: &str, name: &str, slug: &str) -> Result<bool> {
        if !self.store.history(id)?.is_empty() {
            return Ok(false);
        }
        let date = Some(starter_date());
        self.store.append(id, keys::TYPE, Some(ty), date)?;
        self.store.append(id, keys::NAME, Some(name), date)?;
        self.store.append(id, keys::STARTER, Some(slug), date)?;
        Ok(true)
    }

    /// Sends the Notes inside `id` where `notes_go` says; `area_id` is
    /// where [`NotesGo::Area`] leads.
    fn move_notes(&self, id: &str, notes_go: NotesGo, area_id: Option<&str>) -> Result<()> {
        let notes = Notes::new(self.store);
        for note in notes.inside(id)? {
            match notes_go {
                NotesGo::Unsorted => {
                    // Back to Unsorted, even when it spawned a Routine
                    // (CONTEXT.md).
                    self.store.append(&note.id, note_keys::PLACE, None, None)?;
                    if note.spawned {
                        self.store
                            .append(&note.id, note_keys::SPAWNED, None, None)?;
                    }
                }
                NotesGo::Area => {
                    self.store
                        .append(&note.id, note_keys::PLACE, area_id, None)?;
                }
                NotesGo::Trash => notes.trash(&note.id)?,
            }
        }
        Ok(())
    }

    // The writes behind add_area and add_topic, for use inside one batch.
    fn append_area(&self, name: &str, starter: Option<&str>) -> Result<String> {
        let id = format!("area:{}", self.store.new_id()?);
        self.store
            .append(&id, keys::TYPE, Some(keys::TYPE_AREA), None)?;
        self.store.append(&id, keys::NAME, Some(name), None)?;
        if let Some(starter) = starter {
            self.store.append(&id, keys::STARTER, Some(starter), None)?;
        }
        Ok(id)
    }

    fn append_topic(&self, area_id: &str, name: &str, starter: Option<&str>) -> Result<String> {
        let id = format!("topic:{}", self.store.new_id()?);
        self.store
            .append(&id, keys::TYPE, Some(keys::TYPE_TOPIC), None)?;
        self.store.append(&id, keys::AREA, Some(area_id), None)?;
        self.store.append(&id, keys::NAME, Some(name), None)?;
        if let Some(starter) = starter {
            self.store.append(&id, keys::STARTER, Some(starter), None)?;
        }
        Ok(id)
    }

    fn require_area(&self, id: &str) -> Result<()> {
        match self.area_by_id(id)? {
            Some(_) => Ok(()),
            None => Err(Error::Invalid(format!("not an Area: {id}"))),
        }
    }

    /// `name` trimmed; an error when nothing is left.
    fn name(name: &str) -> Result<&str> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(Error::Invalid("the name must not be empty".into()));
        }
        Ok(trimmed)
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
    fn place_writes_check_what_they_write_to() -> Result<()> {
        let store = crate::store::tests::store();
        let places = Places::new(&store);
        assert!(places.add_area("  ").is_err());
        let area = places.add_area(" Home ")?;
        assert_eq!(area.name, "Home");
        assert!(places.add_topic("area:none", "x").is_err());
        let topic = places.add_topic(&area.id, "Post")?;
        assert!(places.add_topic(&topic.id, "Third level").is_err());
        assert!(places.rename("area:none", "x").is_err());
        assert!(places.set_icon(&area.id, Some("rocket")).is_err());
        assert!(places.set_colour(&topic.id, Some("C62828")).is_err());
        assert!(places.delete_area(&area.id, NotesGo::Area).is_err());
        assert!(places.delete_topic(&area.id, NotesGo::Unsorted).is_err());
        places.set_icon(&area.id, Some("home"))?;
        assert_eq!(
            places.area_by_id(&area.id)?.and_then(|a| a.icon),
            Some("home".into())
        );
        places.plant_starter_set(&["work", "meetings", "doctor"])?;
        let names: Vec<_> = places.areas()?.into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Home", "Work"]);
        places.delete_area(&area.id, NotesGo::Unsorted)?;
        assert!(places.topic_by_id(&topic.id)?.is_none());
        Ok(())
    }

    #[test]
    fn starter_names_are_found_by_slug() {
        assert_eq!(starter_name("to-sort-out"), Some("To sort out"));
        assert_eq!(starter_name("health"), Some("Health"));
        assert_eq!(starter_name("nope"), None);
    }
}
