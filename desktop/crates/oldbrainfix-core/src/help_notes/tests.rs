use super::*;
use crate::lan::{answer_sync, apply_answer, sync_request};
use crate::signing::SigningKey;
use crate::{Checklists, ListOnAgenda, Notes, NotesGo, PairedDevice};
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
        quick_start_description: "On the desktop:\nCapture: Ctrl+N\nFile: drag it".into(),
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
    // Named "Quick start", the desktop's way folded under it (#260).
    let text = notes.by_id(&quick_start_id(&p))?.ok_or("the Note")?.text;
    assert_eq!(
        text,
        "Quick start\nOn the desktop:\nCapture: Ctrl+N\nFile: drag it"
    );
    // Formatted, so the links of its steps open (#261); so is What
    // the icons mean.
    for id in [quick_start_id(&p), icons_id(&p)] {
        let note = notes.by_id(&id)?.ok_or("the Note")?;
        assert_eq!(note.text_format, text_format::MARKDOWN, "{id}");
    }
    assert_eq!(crate::note_kinds::name(&text), "Quick start");
    assert_eq!(
        crate::note_kinds::description(&text),
        english().quick_start_description
    );
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
    // On the Agenda as one row with its count, not a row per Line.
    let checklists = Checklists::new(&ann);
    assert_eq!(
        checklists.on_agenda(&quick_start_id(&p))?,
        ListOnAgenda::Count
    );
    let rows = checklists.agenda_rows()?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].shows, ListOnAgenda::Count);
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
