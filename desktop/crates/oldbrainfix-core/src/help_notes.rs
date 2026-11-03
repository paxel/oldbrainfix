//! The app's own Notes that explain it (#180, ADR 0042), under the ids
//! and in the order the Dart core's `help_notes.dart` writes them:
//! "Quick start", "What the icons mean" and one "What's new in X.Y.Z" per
//! release, filed into their own Area "Old Brain Fix". Each has an id
//! fixed by the person and the version, so a second Device of the same
//! person finds it made and makes none, two Devices that make it apart
//! write the same entities, and a Note the person deleted stays deleted.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::Result;
use crate::checklists::{self, NewLine, order, write_line_as};
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
    /// "Quick start" and its Lines, the steps to try.
    pub quick_start_title: String,
    pub quick_start_lines: Vec<String>,
    /// "What the icons mean" in full, its title the first line, and the
    /// pictures of the icons, one per group.
    pub icons_text: String,
    pub icon_pictures: Vec<&'a [u8]>,
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
    /// Quick start and What the icons mean are made once per person.
    /// Each What's new of a version after the one this Device last ran
    /// (or after [`SINCE`], for a Device that recorded none, unless
    /// `fresh`) up to `version` is made once per person, oldest first.
    /// Nothing is made before the own Profile exists; then the version is
    /// recorded and the same start again makes nothing. Returns the ids
    /// of the Notes made.
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
            if !self.made(&persons, quick_start_id)? {
                let into = self.area_once(&mut area, &person, &persons, &texts.area_name)?;
                self.quick_start(&person, &into, texts)?;
                made.push(quick_start_id(&person));
            }
            if !self.made(&persons, icons_id)? {
                let into = self.area_once(&mut area, &person, &persons, &texts.area_name)?;
                self.note(
                    &icons_id(&person),
                    &into,
                    &texts.icons_text,
                    &texts.icon_pictures,
                )?;
                made.push(icons_id(&person));
            }
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

    /// Writes the plain Note `id` with `text` and `photos`, filed into
    /// `area`, with no Nudge.
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

    /// Writes the Quick start: a Checklist whose Lines are the steps to
    /// try, under ids fixed by the person and the Line's place.
    fn quick_start(&self, person: &str, area: &str, texts: &HelpTexts) -> Result<()> {
        let s = self.store;
        let id = quick_start_id(person);
        let making = quick_start_making(person);
        let now = s.now();
        self.note(&id, area, &texts.quick_start_title, &[])?;
        let kind = checklists::keys::kind_of(&making);
        s.append(&id, checklists::keys::KIND, Some(&kind), Some(now))?;
        s.append(&id, note_keys::SPAWNED, Some(&dart_iso(now)), Some(now))?;
        let mut key = String::new();
        for (i, line) in texts.quick_start_lines.iter().enumerate() {
            key = order::after(&key);
            let new = NewLine {
                text: line,
                order: &key,
                indent: false,
                checked: false,
                received: None,
                at: Some(now),
            };
            write_line_as(s, quick_start_line_id(person, i), &id, &making, &new)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{answer_sync, apply_answer, sync_request};
    use crate::signing::SigningKey;
    use crate::{Checklists, Notes, NotesGo, PairedDevice};
    use chrono::Duration;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    fn bare(seed: u8) -> Made<Store> {
        let start = crate::store::tests::ticking()();
        let clock: crate::Clock = Box::new(move || {
            let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            start + Duration::seconds(n)
        });
        Ok(Store::in_memory(clock, move || {
            Ok(SigningKey::from_seed([seed; 32]))
        })?)
    }

    fn device(seed: u8, name: &str) -> Made<Store> {
        let store = bare(seed)?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    /// A Device of the person on `of` that joined it: paired as its own
    /// both ways round and synced, so it holds her Profile.
    fn own_device(seed: u8, of: &Store) -> Made<Store> {
        let desk = bare(seed)?;
        for (store, peer) in [(&desk, of), (of, &desk)] {
            store.remember_device(&PairedDevice {
                id: peer.device_id()?,
                name: String::new(),
                fingerprint: String::new(),
                host: None,
                port: None,
                last_sync: store.now(),
                shared: false,
            })?;
        }
        exchange(&desk, of)?;
        Ok(desk)
    }

    /// One LAN session, `joiner` asking `host`.
    fn exchange(joiner: &Store, host: &Store) -> TestResult {
        let request = sync_request(
            joiner,
            &host.version_vector()?,
            "Phone",
            &host.device_id()?,
            "",
        )?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        apply_answer(joiner, &answer)?;
        Ok(())
    }

    const PICTURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 1];

    fn english() -> HelpTexts<'static> {
        HelpTexts {
            area_name: "Old Brain Fix".into(),
            quick_start_title: "Quick start".into(),
            quick_start_lines: vec![
                "Capture a thought with +".into(),
                "File it into an Area".into(),
            ],
            icons_text: "What the icons mean\n\nKind: what the Note is".into(),
            icon_pictures: vec![PICTURE],
            whats_new_title: Box::new(|v| format!("What's new in {v}")),
        }
    }

    fn bundle() -> WhatsNew {
        let mut all = WhatsNew::new();
        for (v, en) in [
            ("0.5.0", "Added\n• More."),
            ("0.4.0", "Added\n• Help Notes."),
            ("0.4.1", "Fixed\n• A thing."),
        ] {
            all.insert(v.into(), [("en".to_string(), en.to_string())].into());
        }
        all
    }

    fn make(store: &Store, version: &str, fresh: bool) -> Result<Vec<String>> {
        HelpNotes::new(store).make_due(version, fresh, "en", &english(), &bundle())
    }

    fn person(store: &Store) -> Made<String> {
        Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
    }

    #[test]
    fn a_fresh_install_files_quick_start_and_the_icons_and_no_whats_new() -> TestResult {
        let ann = device(1, "Ann")?;
        let p = person(&ann)?;
        assert_eq!(
            make(&ann, "0.4.0", true)?,
            [quick_start_id(&p), icons_id(&p)]
        );
        let area = Places::new(&ann)
            .area_by_id(&area_id(&p))?
            .ok_or("the Area")?;
        assert_eq!(area.name, "Old Brain Fix");
        assert_eq!(area.colour.as_deref(), Some(AREA_COLOUR));
        assert_eq!(area.icon.as_deref(), Some(AREA_ICON));
        let notes = Notes::new(&ann);
        assert!(notes.unsorted()?.is_empty());
        let list = Checklists::new(&ann)
            .by_note(&quick_start_id(&p))?
            .ok_or("a list")?;
        let lines: Vec<_> = list
            .lines
            .iter()
            .map(|l| (l.id.clone(), l.text.clone()))
            .collect();
        assert_eq!(
            lines,
            [
                (
                    quick_start_line_id(&p, 0),
                    "Capture a thought with +".to_string()
                ),
                (
                    quick_start_line_id(&p, 1),
                    "File it into an Area".to_string()
                ),
            ]
        );
        let icons = notes.by_id(&icons_id(&p))?.ok_or("the icons")?;
        assert_eq!(icons.photos.len(), 1);
        assert_eq!(icons.nudge, None);
        let written = ann.all_entries()?.len();
        assert!(make(&ann, "0.4.0", true)?.is_empty());
        assert_eq!(ann.all_entries()?.len(), written);
        Ok(())
    }

    #[test]
    fn nothing_is_made_before_the_own_profile_exists() -> TestResult {
        let store = bare(1)?;
        assert!(make(&store, "0.4.0", true)?.is_empty());
        assert!(store.all_entries()?.is_empty());
        assert_eq!(store.local_setting(VERSION_SETTING)?, None);
        Ok(())
    }

    #[test]
    fn an_update_makes_each_whats_new_passed_oldest_first() -> TestResult {
        let ann = device(1, "Ann")?;
        let p = person(&ann)?;
        assert_eq!(
            make(&ann, "0.4.1", false)?,
            [
                quick_start_id(&p),
                icons_id(&p),
                whats_new_id(&p, "0.4.0"),
                whats_new_id(&p, "0.4.1"),
            ]
        );
        let note = Notes::new(&ann)
            .by_id(&whats_new_id(&p, "0.4.0"))?
            .ok_or("a Note")?;
        assert_eq!(note.text, "What's new in 0.4.0\n\nAdded\n• Help Notes.");
        assert_eq!(make(&ann, "0.5.0", false)?, [whats_new_id(&p, "0.5.0")]);
        Ok(())
    }

    #[test]
    fn a_second_device_makes_nothing_and_a_deleted_note_never_returns() -> TestResult {
        let ann = device(1, "Ann")?;
        let p = person(&ann)?;
        make(&ann, "0.4.1", false)?;
        Notes::new(&ann).trash(&quick_start_id(&p))?;
        ann.append(&icons_id(&p), note_keys::TYPE, None, None)?;
        let desk = own_device(2, &ann)?;
        assert_eq!(person(&desk)?, p);
        assert!(make(&desk, "0.4.1", true)?.is_empty());
        assert_eq!(make(&desk, "0.5.0", false)?, [whats_new_id(&p, "0.5.0")]);
        Ok(())
    }

    #[test]
    fn a_note_another_person_wrote_under_the_id_counts_for_nothing() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(3, "Ben")?;
        let p = person(&ann)?;
        ben.append(
            &quick_start_id(&p),
            note_keys::TYPE,
            Some(note_keys::TYPE_NOTE),
            None,
        )?;
        let wire: Vec<_> = ben
            .entries_since(&HashMap::new())?
            .iter()
            .map(|e| e.wire())
            .collect();
        let mut report = crate::ImportReport::default();
        ann.apply_entries(&wire, &[ben.own_key_record()?], None, &mut report)?;
        assert!(make(&ann, "0.4.0", true)?.contains(&quick_start_id(&p)));
        Ok(())
    }

    #[test]
    fn a_deleted_area_is_made_again_with_its_look() -> TestResult {
        let ann = device(1, "Ann")?;
        let p = person(&ann)?;
        make(&ann, "0.4.0", true)?;
        let places = Places::new(&ann);
        places.set_colour(&area_id(&p), Some("FF0000"))?;
        places.delete_area(&area_id(&p), NotesGo::Trash)?;
        assert_eq!(make(&ann, "0.4.1", false)?, [whats_new_id(&p, "0.4.1")]);
        let area = places.area_by_id(&area_id(&p))?.ok_or("the Area")?;
        assert_eq!(area.colour.as_deref(), Some("FF0000"));
        assert!(Notes::new(&ann).unsorted()?.is_empty());
        Ok(())
    }

    #[test]
    fn the_bundled_whats_new_reads_by_version_and_language() {
        let read = parse_whats_new(r#"{"0.4.0": {"de": "Neu", "en": "Added", "x": 1}, "y": 2}"#);
        assert_eq!(read.len(), 1);
        assert_eq!(read["0.4.0"].len(), 2);
        assert_eq!(read["0.4.0"]["de"], "Neu");
        assert!(parse_whats_new("[]").is_empty());
        assert!(parse_whats_new("no JSON").is_empty());
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert_eq!(compare_versions("0.4.0", "0.3.1"), Ordering::Greater);
        assert_eq!(compare_versions("0.10.0", "0.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("0.4", "0.4.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.0.0", "1.0.1"), Ordering::Less);
    }
}
