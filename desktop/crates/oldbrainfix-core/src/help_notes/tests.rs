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

fn english() -> HelpTexts<'static> {
    HelpTexts {
        area_name: "Old Brain Fix".into(),
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
fn a_fresh_install_makes_nothing_and_records_the_version() -> TestResult {
    let ann = device(1, "Ann")?;
    let p = person(&ann)?;
    let written = ann.all_entries()?.len();
    assert!(make(&ann, "0.4.0", true)?.is_empty());
    assert_eq!(ann.all_entries()?.len(), written);
    assert_eq!(
        ann.local_setting(VERSION_SETTING)?.as_deref(),
        Some("0.4.0")
    );
    // No Area without a Note to go into it.
    assert!(Places::new(&ann).area_by_id(&area_id(&p))?.is_none());
    assert!(make(&ann, "0.4.0", true)?.is_empty());
    assert_eq!(ann.all_entries()?.len(), written);
    Ok(())
}

/// Spotlights explain the screens instead (obf-1043, ADR 0055): neither
/// a fresh start, an update nor a Device that recorded no version makes
/// a Quick start or What the icons mean.
#[test]
fn no_start_makes_a_quick_start_or_what_the_icons_mean() -> TestResult {
    let ann = device(1, "Ann")?;
    let p = person(&ann)?;
    let mut made = make(&ann, "0.4.0", true)?;
    made.extend(make(&ann, "0.4.1", false)?);
    ann.remove_local_setting(VERSION_SETTING)?;
    made.extend(make(&ann, "0.5.0", false)?);
    assert_eq!(
        made,
        [
            whats_new_id(&p, "0.4.1"),
            whats_new_id(&p, "0.4.0"),
            whats_new_id(&p, "0.5.0"),
        ]
    );
    for id in [quick_start_id(&p), icons_id(&p)] {
        assert!(ann.logged_history(&id)?.is_empty(), "{id}");
    }
    assert!(Checklists::new(&ann).agenda_rows()?.is_empty());
    Ok(())
}

#[test]
fn a_quick_start_made_before_stays_as_it_is() -> TestResult {
    let ann = device(1, "Ann")?;
    let p = person(&ann)?;
    // As 0.6.1 made it, in short.
    let id = quick_start_id(&p);
    ann.append(&id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), None)?;
    ann.append(&id, note_keys::TEXT, Some("Quick start"), None)?;
    ann.append(&id, note_keys::PLACE, Some(&area_id(&p)), None)?;
    let before = ann.logged_history(&id)?.len();
    assert_eq!(make(&ann, "0.4.0", false)?, [whats_new_id(&p, "0.4.0")]);
    assert_eq!(ann.logged_history(&id)?.len(), before);
    Ok(())
}

#[test]
fn nothing_is_made_before_the_own_profile_exists() -> TestResult {
    let store = bare(1)?;
    assert!(make(&store, "0.4.0", false)?.is_empty());
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
        [whats_new_id(&p, "0.4.0"), whats_new_id(&p, "0.4.1")]
    );
    let note = Notes::new(&ann)
        .by_id(&whats_new_id(&p, "0.4.0"))?
        .ok_or("a Note")?;
    assert_eq!(note.text, "What's new in 0.4.0\n\nAdded\n• Help Notes.");
    assert_eq!(note.text_format, text_format::PLAIN);
    assert_eq!(note.nudge, None);
    let area = Places::new(&ann)
        .area_by_id(&area_id(&p))?
        .ok_or("the Area")?;
    assert_eq!(area.name, "Old Brain Fix");
    assert_eq!(area.colour.as_deref(), Some(AREA_COLOUR));
    assert_eq!(area.icon.as_deref(), Some(AREA_ICON));
    assert!(Notes::new(&ann).unsorted()?.is_empty());
    assert_eq!(make(&ann, "0.5.0", false)?, [whats_new_id(&p, "0.5.0")]);
    Ok(())
}

#[test]
fn a_second_device_makes_nothing_and_a_deleted_note_never_returns() -> TestResult {
    let ann = device(1, "Ann")?;
    let p = person(&ann)?;
    make(&ann, "0.4.1", false)?;
    Notes::new(&ann).trash(&whats_new_id(&p, "0.4.0"))?;
    // Purged, as 30 days in the trash do.
    ann.append(&whats_new_id(&p, "0.4.1"), note_keys::TYPE, None, None)?;
    let desk = own_device(2, &ann)?;
    assert_eq!(person(&desk)?, p);
    assert!(make(&desk, "0.4.1", true)?.is_empty());
    assert_eq!(make(&desk, "0.5.0", false)?, [whats_new_id(&p, "0.5.0")]);
    desk.remove_local_setting(VERSION_SETTING)?;
    assert!(make(&desk, "0.5.0", false)?.is_empty());
    Ok(())
}

#[test]
fn a_note_another_person_wrote_under_the_id_counts_for_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(3, "Ben")?;
    let p = person(&ann)?;
    ben.append(
        &whats_new_id(&p, "0.4.0"),
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
    assert!(make(&ann, "0.4.0", false)?.contains(&whats_new_id(&p, "0.4.0")));
    Ok(())
}

#[test]
fn a_deleted_area_is_made_again_with_its_look() -> TestResult {
    let ann = device(1, "Ann")?;
    let p = person(&ann)?;
    make(&ann, "0.4.0", false)?;
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
