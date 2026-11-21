//! The app's own Notes that explain it (#180, ADR 0042), under the ids
//! and in the order the Dart core's `help_notes.dart` writes them: one
//! "What's new in X.Y.Z" per release, filed into their own Area "Old
//! Brain Fix". Each has an id fixed by the person and the version, so a
//! second Device of the same person finds it made and makes none, two
//! Devices that make it apart write the same entities, and a Note the
//! person deleted stays deleted. "Quick start" and "What the icons mean"
//! are made no more (obf-1043, ADR 0055), as Spotlights explain the
//! screens; the ones made up to 0.6.1 stay as the person has them, under
//! the ids below.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::Result;
use crate::checklists;
use crate::entry::dart_iso;
use crate::notes::keys as note_keys;
use crate::places::{Places, keys as place_keys};
use crate::profile::OwnProfile;
use crate::store::Store;
use crate::text_format::{self, StoredText};

/// The version before the first one that makes help Notes: an update
/// from a Device that recorded none passed every version after it.
pub const SINCE: &str = "0.3.1";

/// The local setting holding the version this Device last made the help
/// Notes for.
pub const VERSION_SETTING: &str = "helpVersion";

/// The Area's look: the app's teal and a brain, as its icon.
pub const AREA_COLOUR: &str = "00796B";
pub const AREA_ICON: &str = "brain";

/// Each release's Play notes by version, then by language.
pub type WhatsNew = BTreeMap<String, BTreeMap<String, String>>;

/// What the help Notes say, in the language of the Device that makes
/// them; the app builds it from its strings and the What's new it
/// bundles.
pub struct HelpTexts<'a> {
    /// The Area's name, "Old Brain Fix".
    pub area_name: String,
    /// The title of a What's new Note for a version.
    pub whats_new_title: Box<dyn Fn(&str) -> String + 'a>,
}

/// The Area every help Note of the person `person` is filed into.
pub fn area_id(person: &str) -> String {
    format!("area:help-{person}")
}

pub fn quick_start_id(person: &str) -> String {
    format!("note:help-{person}-quick-start")
}

pub fn icons_id(person: &str) -> String {
    format!("note:help-{person}-icons")
}

pub fn whats_new_id(person: &str, version: &str) -> String {
    format!("note:help-{person}-whats-new-{version}")
}

/// The `index`th Line of the Quick start, from 0.
pub fn quick_start_line_id(person: &str, index: usize) -> String {
    format!(
        "{}help-{person}-quick-start-{index}",
        checklists::keys::LINE_PREFIX
    )
}

/// The making of the Quick start's Checklist.
pub fn quick_start_making(person: &str) -> String {
    format!("help-{person}-quick-start")
}

/// Orders the versions `a` and `b`, `X.Y.Z` each, by their numbers; a
/// part that is no number counts as 0.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| -> Vec<i64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let by = x.get(i).unwrap_or(&0).cmp(y.get(i).unwrap_or(&0));
        if by != Ordering::Equal {
            return by;
        }
    }
    Ordering::Equal
}

/// The What's new both apps bundle (`assets/help/whats_new.json`,
/// written by `tool/whats_new.py --bundle`): each version's notes by
/// language; what is no text is left out, and a file that is no JSON
/// object reads as none.
pub fn parse_whats_new(json: &str) -> WhatsNew {
    let Ok(serde_json::Value::Object(all)) = serde_json::from_str(json) else {
        return WhatsNew::new();
    };
    all.into_iter()
        .filter_map(|(version, notes)| {
            let serde_json::Value::Object(notes) = notes else {
                return None;
            };
            let by_lang = notes
                .into_iter()
                .filter_map(|(lang, text)| Some((lang, text.as_str()?.to_string())))
                .collect();
            Some((version, by_lang))
        })
        .collect()
}

/// Makes the help Notes, as the Dart core's `HelpNotes`.
pub struct HelpNotes<'a> {
    store: &'a Store,
}

impl<'a> HelpNotes<'a> {
    pub fn new(store: &'a Store) -> HelpNotes<'a> {
        HelpNotes { store }
    }

    /// Makes the help Notes due on this Device running `version`, in
    /// `language` with `texts`; `fresh` when this start began with the
    /// first start (a new person, a Device added or a Backup restored).
    ///
    /// Each What's new of a version after the one this Device last ran
    /// (or after [`SINCE`], for a Device that recorded none, unless
    /// `fresh`) up to `version` is made once per person, oldest first; a
    /// fresh start makes none. Nothing is made before the own Profile
    /// exists; then the version is recorded and the same start again
    /// makes nothing. Returns the ids of the Notes made.
    pub fn make_due(
        &self,
        version: &str,
        fresh: bool,
        language: &str,
        texts: &HelpTexts,
        whats_new: &WhatsNew,
    ) -> Result<Vec<String>> {
        let s = self.store;
        let last = s.local_setting(VERSION_SETTING)?;
        if last.as_deref() == Some(version) {
            return Ok(Vec::new());
        }
        let own = OwnProfile::new(s);
        let Some(profile) = own.get()? else {
            return Ok(Vec::new());
        };
        let person = profile.id;
        let mut aliases = own.aliases()?;
        aliases.sort();
        let mut persons = vec![person.clone()];
        persons.extend(aliases);
        let from = match &last {
            Some(last) => last.clone(),
            None if fresh => version.to_string(),
            None => SINCE.to_string(),
        };
        let mut due: Vec<&String> = whats_new
            .keys()
            .filter(|v| {
                compare_versions(v, &from) == Ordering::Greater
                    && compare_versions(v, version) != Ordering::Greater
            })
            .collect();
        due.sort_by(|a, b| compare_versions(a, b));
        s.batch(|| {
            let mut made = Vec::new();
            let mut area: Option<String> = None;
            for v in due {
                if self.made(&persons, |p| whats_new_id(p, v))? {
                    continue;
                }
                let Some(notes) = whats_new.get(v) else {
                    continue;
                };
                let Some(text) = notes.get(language).or_else(|| notes.get("en")) else {
                    continue;
                };
                let into = self.area_once(&mut area, &person, &persons, &texts.area_name)?;
                let text = format!("{}\n\n{text}", (texts.whats_new_title)(v));
                self.note(&whats_new_id(&person, v), &into, &text, &[])?;
                made.push(whats_new_id(&person, v));
            }
            s.set_local_setting(VERSION_SETTING, version)?;
            Ok(made)
        })
    }

    /// Whether one of the user's own Devices ever wrote the Note `id`
    /// makes for one of `persons`: in the trash or purged, it counts as
    /// made.
    fn made(&self, persons: &[String], id: impl Fn(&str) -> String) -> Result<bool> {
        let mut own = HashMap::new();
        for person in persons {
            for e in self.store.logged_history(&id(person))? {
                if self.store.written_by_own_in(&e.device, &mut own)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// The Area, worked out once per run.
    fn area_once(
        &self,
        area: &mut Option<String>,
        person: &str,
        persons: &[String],
        name: &str,
    ) -> Result<String> {
        if let Some(id) = area {
            return Ok(id.clone());
        }
        let id = self.area(person, persons, name)?;
        *area = Some(id.clone());
        Ok(id)
    }

    /// The Area the help Notes go into: the person's own, made now when
    /// it never was, and made again with the look it had when the user
    /// deleted it, so no help Note is ever Unsorted.
    fn area(&self, person: &str, persons: &[String], name: &str) -> Result<String> {
        let places = Places::new(self.store);
        for p in persons {
            if places.area_by_id(&area_id(p))?.is_some() {
                return Ok(area_id(p));
            }
        }
        let s = self.store;
        let id = area_id(person);
        let never = s.logged_history(&id)?.is_empty();
        let now = Some(s.now());
        s.append(&id, place_keys::TYPE, Some(place_keys::TYPE_AREA), now)?;
        if never {
            s.append(&id, place_keys::NAME, Some(name), now)?;
            s.append(&id, place_keys::COLOUR, Some(AREA_COLOUR), now)?;
            s.append(&id, place_keys::ICON, Some(AREA_ICON), now)?;
        }
        Ok(id)
    }

    /// Writes the Note `id` with `text` and `photos`, filed into `area`,
    /// with no Nudge, plain.
    fn note(&self, id: &str, area: &str, text: &str, photos: &[&[u8]]) -> Result<()> {
        let s = self.store;
        let now = s.now();
        let at = Some(now);
        let hashes = photos
            .iter()
            .map(|p| s.put_photo(p))
            .collect::<Result<Vec<_>>>()?;
        s.append(id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), at)?;
        s.append(id, note_keys::CAPTURED, Some(&dart_iso(now)), at)?;
        let text = StoredText::of(text_format::PLAIN, text, text_format::VERSION).encode();
        s.append(id, note_keys::TEXT, Some(&text), at)?;
        s.append(id, note_keys::PLACE, Some(area), at)?;
        for hash in &hashes {
            s.append(id, &note_keys::photo(hash), Some(note_keys::ADDED), at)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
