//! The Dart core writes the fixture corpus under `desktop/fixtures`
//! (`packages/oldbrainfix_core/tool/gen_fixtures.dart`); this core must
//! read every log the same, sign every entry the same, and write every
//! scenario again to the same log, byte for byte. Schedules, the alarm
//! plan, the Missed Summary and the Nudge must come to the same on the
//! same European wall clock, Europe/Berlin here.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use oldbrainfix_core::alarms::{WallMoment, instant_of};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::nudges::QuietHours;
use oldbrainfix_core::places::keys as place_keys;
use oldbrainfix_core::question_marks::{self, SHAPES};
use oldbrainfix_core::schedule::Frequency;
use oldbrainfix_core::signing::{entry_bytes, verify_signature};
use oldbrainfix_core::{
    AgendaRow, Alarms, Blob, ClockTime, Conflict, Day, Importance, Note, Notes, NotesGo,
    Occurrence, PersonFilter, Places, Recurrence, RepeatKind, Routine, Routines, STARTER_SET,
    Schedule, Search, Section, SigningKey, SimpleRepeat, Store, WireEntry, dart_iso, parse_time,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SCENARIOS: [&str; 8] = [
    "edits", "formats", "media", "notes", "places", "routines", "sections", "trash",
];

/// The queries every scenario's search is checked with, the generator's.
const QUERIES: [&str; 12] = [
    "", "sauce", "MILK", "receipt", "dentist", "x-ray", "müller", "called", "loop", " e ",
    "heading", "format",
];

/// A text in a format this version does not know, as the generator's
/// `_future` stores it.
const FUTURE: &str = r##"{"format":"x-future","text":"# Heading\n**bold** \u00fc","x":1}"##;

/// Texts by the version of their structure, as the generator's
/// `_versioned` stores them.
const VERSIONED: [&str; 5] = [
    r#"{"format":"plain","text":"Version one written out","v":1}"#,
    r##"{"format":"x-future","text":"# Version one","v":1}"##,
    r#"{"format":"plain","text":"Version two","v":2,"x":1}"#,
    r#"{"format":"plain","v":2}"#,
    r#"{"format":"plain","text":"Odd version","v":"1"}"#,
];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

#[test]
fn every_scenario_the_dart_core_wrote_is_checked() -> TestResult {
    let mut written = Vec::new();
    for dir in std::fs::read_dir(fixtures())? {
        let dir = dir?.path();
        if dir.join("log.jsonl").is_file() {
            written.push(dir.file_name().map(|n| n.to_string_lossy().into_owned()));
        }
    }
    written.sort();
    let checked: Vec<_> = SCENARIOS.iter().map(|s| Some(s.to_string())).collect();
    assert_eq!(written, checked);
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

/// The writer's key: seed bytes 1 to 32.
/// The own Profile's settings of `store`, as the generator writes them.
fn profile_settings(store: &Store) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(Value::Object(
        store
            .local_settings_with_prefix("profile")?
            .into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect(),
    ))
}

/// The backup key `store` holds, with its sealed copy and version, as
/// the Dart core's fixtures list them.
fn backup_settings(store: &Store) -> Result<Value, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for key in ["backup:key", "backup:pass", "backup:version"] {
        if let Some(value) = store.local_setting(key)? {
            out.push(json!([key, value]));
        }
    }
    Ok(Value::Array(out))
}

fn writer_key() -> SigningKey {
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8 + 1;
    }
    SigningKey::from_seed(seed)
}

fn log(scenario: &str) -> Result<Vec<WireEntry>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(fixtures().join(scenario).join("log.jsonl"))?;
    let mut entries = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        entries.push(serde_json::from_str(line)?);
    }
    Ok(entries)
}

/// A store holding the scenario's log, its clock held at `now`.
fn load(scenario: &str, now: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let now = parse_time(now)?;
    let store = Store::in_memory(Box::new(move || now), || Ok(writer_key()))?;
    for entry in log(scenario)? {
        store.insert(&entry)?;
    }
    Ok(store)
}

fn note_json(note: &Note) -> Value {
    json!({
        "id": note.id,
        "text": note.text,
        "textFormat": note.text_format,
        "textVersion": note.text_version,
        "textEditable": note.text_editable,
        "captured": dart_iso(note.captured),
        "place": note.place,
        "importance": note.importance.name(),
        "trashed": note.trashed.map(dart_iso),
        "mergedInto": note.merged_into,
        "spawned": note.spawned,
        "photos": note.photos,
        "recordings": note.recordings,
        "transcripts": note.transcripts,
        "photoTexts": note.photo_texts,
        "removedPhotos": note.removed_photos,
        "removedRecordings": note.removed_recordings,
        "wordless": note.wordless().name(),
    })
}

fn section_json(section: &Section) -> Value {
    json!({
        "id": section.id,
        "noteId": section.note_id,
        "text": section.text,
        "textFormat": section.text_format,
        "textVersion": section.text_version,
        "textEditable": section.text_editable,
        "captured": dart_iso(section.captured),
        "photos": section.photos,
        "recordings": section.recordings,
        "transcripts": section.transcripts,
        "photoTexts": section.photo_texts,
        "removedPhotos": section.removed_photos,
        "removedRecordings": section.removed_recordings,
    })
}

fn blobs_json(list: &[Blob]) -> Value {
    json!(
        list.iter()
            .map(|b| json!({"noteId": b.note_id, "hash": b.hash}))
            .collect::<Vec<_>>()
    )
}

/// What this core reads, in the shape of the Dart core's `expected.json`.
fn read(store: &Store) -> Result<Value, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let places = Places::new(store);
    let mut note_ids = store.entities_of_type("note")?;
    note_ids.sort();
    let all = notes.all()?;
    let trashed = notes.trashed()?;
    let mut areas = Vec::new();
    let mut inside = serde_json::Map::new();
    let mut within = serde_json::Map::new();
    let search = Search::new(store);
    let found = |query: &str, place: Option<&str>| -> Result<Value, Box<dyn std::error::Error>> {
        let ids: Vec<String> = search
            .notes(query, place)?
            .into_iter()
            .map(|n| n.id)
            .collect();
        Ok(json!(ids))
    };
    for area in places.areas()? {
        let topics = places.topics(&area.id)?;
        let ids = |place: &str| -> Result<Value, Box<dyn std::error::Error>> {
            Ok(json!(
                notes
                    .inside(place)?
                    .iter()
                    .map(|n| n.id.clone())
                    .collect::<Vec<_>>()
            ))
        };
        inside.insert(area.id.clone(), ids(&area.id)?);
        within.insert(area.id.clone(), found("e", Some(&area.id))?);
        for topic in &topics {
            inside.insert(topic.id.clone(), ids(&topic.id)?);
            within.insert(topic.id.clone(), found("e", Some(&topic.id))?);
        }
        areas.push(json!({
            "id": area.id,
            "name": area.name,
            "starter": area.starter,
            "colour": area.colour,
            "icon": area.icon,
            "topics": topics.iter().map(|t| json!({
                "id": t.id,
                "areaId": t.area_id,
                "name": t.name,
                "starter": t.starter,
            })).collect::<Vec<_>>(),
        }));
    }
    let mut every = Vec::new();
    let mut removable = serde_json::Map::new();
    for id in &note_ids {
        if let Some(note) = notes.by_id(id)? {
            every.push(note_json(&note));
            let mut refs = Vec::new();
            for hash in &note.photos {
                if notes.may_remove_photo(id, hash)? {
                    refs.push(format!("photo/{hash}"));
                }
            }
            for hash in &note.recordings {
                if notes.may_remove_recording(id, hash)? {
                    refs.push(format!("audio/{hash}"));
                }
            }
            if !refs.is_empty() {
                removable.insert(id.clone(), json!(refs));
            }
        }
    }
    let mut sections = serde_json::Map::new();
    for note in all.iter().chain(trashed.iter()) {
        let list: Vec<Value> = notes.sections(&note.id)?.iter().map(section_json).collect();
        sections.insert(note.id.clone(), json!(list));
    }
    let ids = |list: &[Note]| list.iter().map(|n| n.id.clone()).collect::<Vec<_>>();
    let mut searched = serde_json::Map::new();
    for query in QUERIES {
        searched.insert(query.to_string(), found(query, None)?);
    }
    Ok(json!({
        "now": dart_iso(store.now()),
        "device": store.device_id()?,
        "areas": areas,
        "notes": every,
        "all": ids(&all),
        "unsorted": ids(&notes.unsorted()?),
        "trashed": ids(&trashed),
        "inside": inside,
        "sections": sections,
        "untranscribed": blobs_json(&notes.untranscribed()?),
        "unreadPhotos": blobs_json(&notes.unread_photos()?),
        "removable": removable,
        "search": searched,
        "searchWithin": within,
        "conflicts": conflicts_json(&store.conflicts()?),
    }))
}

/// Conflicts in the shape of the generator's `_conflicts`.
fn conflicts_json(conflicts: &[Conflict]) -> Value {
    json!(
        conflicts
            .iter()
            .map(|c| json!({
                "entity": c.entity,
                "unit": c.unit,
                "type": c.entity_type,
                "resolvable": c.resolvable(),
                "versions": c.versions.iter().map(|v| json!({
                    "entry": v.entry.id(),
                    "device": v.entry.device,
                    "recorded": dart_iso(v.entry.recorded_at),
                    "values": v.values.iter().map(|(k, val)| (k.clone(), json!(val))).collect::<serde_json::Map<String, Value>>(),
                })).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>()
    )
}

#[test]
fn every_scenario_reads_as_the_dart_core_reads_it() -> TestResult {
    for scenario in SCENARIOS {
        let expected = read_json(&fixtures().join(scenario).join("expected.json"))?;
        let now = expected["now"].as_str().ok_or("expected.json names now")?;
        let store = load(scenario, now)?;
        let actual = read(&store)?;
        assert_eq!(actual, expected, "{scenario}");
    }
    Ok(())
}

#[test]
fn every_logged_entry_verifies_and_signs_the_same_here() -> TestResult {
    let key = writer_key();
    for scenario in SCENARIOS {
        for e in log(scenario)? {
            let bytes = entry_bytes(
                &e.device,
                e.dseq,
                &e.entity,
                &e.field,
                e.value.as_deref(),
                &e.date,
                &e.author,
                &e.recorded,
                e.ctx.as_deref(),
            );
            assert!(
                verify_signature(&key.public_key(), &bytes, e.sig.as_deref()),
                "{scenario} {}:{}",
                e.device,
                e.dseq
            );
            assert_eq!(Some(key.sign(&bytes)), e.sig, "{scenario} {}", e.dseq);
        }
    }
    Ok(())
}

#[test]
fn awkward_text_signs_to_the_same_bytes_and_signature() -> TestResult {
    let fixture = read_json(&fixtures().join("signing.json"))?;
    let key = SigningKey::from_seed_hex(fixture["seed"].as_str().ok_or("seed")?)?;
    assert_eq!(
        BASE64.encode(key.public_key()),
        fixture["publicKey"].as_str().ok_or("publicKey")?
    );
    assert_eq!(
        oldbrainfix_core::signing::device_id_from_key(&key.public_key()),
        fixture["device"].as_str().ok_or("device")?
    );
    assert_eq!(key.code(), fixture["code"].as_str().ok_or("code")?);
    let entries = fixture["entries"].as_array().ok_or("entries")?;
    assert_eq!(entries.len(), 4);
    for e in entries {
        let s = |k: &str| e[k].as_str().ok_or(format!("{k} is text"));
        let bytes = entry_bytes(
            s("device")?,
            e["dseq"].as_i64().ok_or("dseq")?,
            s("entity")?,
            s("field")?,
            e["value"].as_str(),
            s("date")?,
            s("author")?,
            s("recorded")?,
            e["ctx"].as_str(),
        );
        assert_eq!(BASE64.encode(&bytes), s("bytes")?);
        assert_eq!(key.sign(&bytes), s("sig")?);
    }
    Ok(())
}

#[test]
fn an_entry_appended_here_signs_as_the_dart_core_would() -> TestResult {
    // The Dart core's first entry of the `notes` scenario, written again
    // by this core under the same key, clock and author.
    let first = log("notes")?.into_iter().next().ok_or("an entry")?;
    let recorded = parse_time(&first.recorded)?;
    let date = parse_time(&first.date)?;
    let store = Store::in_memory(Box::new(move || recorded), || Ok(writer_key()))?;
    store.set_local_setting("author", &first.author)?;
    let mine = store.append(
        &first.entity,
        &first.field,
        first.value.as_deref(),
        Some(date),
    )?;
    assert_eq!(mine.wire(), first);
    Ok(())
}

#[test]
fn the_store_has_the_dart_core_tables() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    drop(Store::open(
        &path,
        Box::new(chrono::Utc::now),
        SigningKey::generate,
    )?);
    let db = rusqlite::Connection::open(&path)?;
    let mut stmt = db.prepare(
        "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut ours = String::new();
    for row in rows {
        ours.push_str(&row?);
        ours.push_str(";\n");
    }
    let theirs = std::fs::read_to_string(fixtures().join("schema.sql"))?;
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(squash(&ours), squash(&theirs));
    Ok(())
}

#[test]
fn the_question_marks_lie_where_the_dart_core_puts_them() -> TestResult {
    let fixture = read_json(&fixtures().join("question_marks.json"))?;
    let shapes = fixture["shapes"].as_array().ok_or("shapes")?;
    assert_eq!(shapes.len(), SHAPES.len());
    for (shape, theirs) in SHAPES.iter().zip(shapes) {
        let strokes: Vec<Vec<f64>> = serde_json::from_value(theirs["strokes"].clone())?;
        let ours: Vec<Vec<f64>> = shape.strokes.iter().map(|s| s.to_vec()).collect();
        assert_eq!(ours, strokes);
        let dot: Option<[f64; 3]> = serde_json::from_value(theirs["dot"].clone())?;
        assert_eq!(shape.dot, dot);
        assert_eq!(Some(shape.weight), theirs["weight"].as_f64());
    }
    let seed = fixture["seed"].as_u64().ok_or("seed")?;
    assert_eq!(seed, u64::from(question_marks::SEED));
    let region: Vec<f64> = serde_json::from_value(fixture["region"].clone())?;
    let [left, top, right, bottom] = region[..] else {
        return Err("a region of four numbers".into());
    };
    let check = |marks: Vec<question_marks::QuestionMark>, theirs: &Value| -> TestResult {
        let theirs = theirs.as_array().ok_or("marks")?;
        assert_eq!(marks.len(), theirs.len());
        for (m, t) in marks.iter().zip(theirs) {
            assert_eq!(Some(m.shape as u64), t["shape"].as_u64());
            assert_eq!(Some(m.x), t["x"].as_f64());
            assert_eq!(Some(m.y), t["y"].as_f64());
            assert_eq!(Some(m.size), t["size"].as_f64());
            assert_eq!(Some(m.rotation), t["rotation"].as_f64());
        }
        Ok(())
    };
    check(
        question_marks::question_marks_in(left, top, right, bottom, question_marks::SEED),
        &fixture["marks"],
    )?;
    check(
        question_marks::question_marks_in(0.0, 0.0, 300.0, 300.0, 1),
        &fixture["otherSeed"],
    )?;
    Ok(())
}

#[test]
fn the_workspace_version_is_the_pubspec_version() -> TestResult {
    let pubspec = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../pubspec.yaml"),
    )?;
    let version = pubspec
        .lines()
        .find_map(|l| l.strip_prefix("version: "))
        .and_then(|v| v.split('+').next())
        .ok_or("pubspec.yaml names a version")?;
    assert_eq!(version.trim(), env!("CARGO_PKG_VERSION"));
    Ok(())
}

// ------------------------------------------------- writing the scenarios

/// The generator's store: the writer key, author Ada, a clock that starts
/// at 2026-01-01 10:00 UTC and moves one second per read, and ids
/// counted from 1, as `_store()` in the generator makes it.
fn writer_store() -> Result<Store, Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};
    let start = Utc
        .with_ymd_and_hms(2026, 1, 1, 10, 0, 0)
        .single()
        .ok_or("a valid start")?;
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst) + 1);
    let mut store = Store::in_memory(Box::new(clock), || Ok(writer_key()))?;
    let ids = Arc::new(AtomicI64::new(0));
    store.set_id_maker(Box::new(move || {
        Ok(format!("{:032x}", ids.fetch_add(1, Ordering::SeqCst) + 1))
    }));
    store.set_local_setting("author", "Ada")?;
    Ok(store)
}

fn utc(y: i32, m: u32, d: u32, h: u32) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    Ok(Utc
        .with_ymd_and_hms(y, m, d, h, 0, 0)
        .single()
        .ok_or("a valid date")?)
}

type Built = Result<(), Box<dyn std::error::Error>>;

/// Writes one scenario into a store.
type Writer = fn(&Store) -> Built;

/// The generator's `notes` scenario.
fn write_notes(store: &Store) -> Built {
    let notes = Notes::new(store);
    let a = notes.capture("Buy more sauce", None, None)?;
    notes.set_importance(&a.id, Importance::Important)?;
    let b = notes.capture("nees mor sauce", None, None)?;
    notes.edit(&b.id, "Needs more sauce")?;
    let c = notes.capture("Maybe paint the shed", None, None)?;
    notes.set_importance(&c.id, Importance::Unimportant)?;
    notes.set_importance(&c.id, Importance::Normal)?;
    notes.set_importance(&c.id, Importance::Unimportant)?;
    let d = notes.capture("First text", None, None)?;
    store.append(
        &d.id,
        note_keys::TEXT,
        Some("Dated back"),
        Some(utc(2025, 12, 31, 0)?),
    )?;
    let e = notes.capture("Sub-millisecond", None, None)?;
    store.append(
        &e.id,
        note_keys::TEXT,
        Some("Half a millisecond later"),
        Some(utc(2026, 1, 2, 9)? + Duration::microseconds(500)),
    )?;
    store.append(
        &e.id,
        note_keys::TEXT,
        Some("Earlier, written later"),
        Some(utc(2026, 1, 2, 9)?),
    )?;
    notes.capture(
        "Quote \" backslash \\ slash / newline \n tab \t bell \u{1} umlaut ü emoji 🧠",
        None,
        None,
    )?;
    Ok(())
}

/// The generator's `sections` scenario.
fn write_sections(store: &Store) -> Built {
    let notes = Notes::new(store);
    let a = notes.capture("Dentist on Tuesday", None, None)?;
    notes.amend(&a.id, "Moved to Wednesday")?;
    let b = notes.capture("Ask about the crown", None, None)?;
    notes.amend(&b.id, "Bring the old X-ray")?;
    let c = notes.capture("Dentist number", None, None)?;
    notes.merge(&b.id, &a.id)?;
    notes.merge(&a.id, &c.id)?;
    notes.amend(&c.id, "Called, all set")?;
    let x = notes.capture("Loop one", None, None)?;
    let y = notes.capture("Loop two", None, None)?;
    store.append(&y.id, note_keys::MERGED, Some(&x.id), None)?;
    store.append(&x.id, note_keys::MERGED, Some(&y.id), None)?;
    let z = notes.capture("Merged into a stranger", None, None)?;
    store.append(&z.id, note_keys::MERGED, Some("note:elsewhere"), None)?;
    Ok(())
}

/// The generator's `formats` scenario.
fn write_formats(store: &Store) -> Built {
    let notes = Notes::new(store);
    let a = notes.capture("Plain milk", None, None)?;
    notes.edit(&a.id, "Plain oat milk")?;
    let b = notes.capture("Before the format", None, None)?;
    store.append(
        &b.id,
        note_keys::TEXT,
        Some("Written before the format"),
        None,
    )?;
    store.append(&b.id, note_keys::TEXT, Some(r#"{"format":"plain"}"#), None)?;
    let f = notes.capture("Formatted later", None, None)?;
    store.append(&f.id, note_keys::TEXT, Some(FUTURE), None)?;
    let s = notes.amend(&f.id, "Amended here")?;
    store.append(&s.id, note_keys::TEXT, Some(FUTURE), None)?;
    let host = notes.capture("Plain host", None, None)?;
    notes.merge(&f.id, &host.id)?;
    let g = notes.capture("Formatted, standing alone", None, None)?;
    store.append(&g.id, note_keys::TEXT, Some(FUTURE), None)?;
    for value in VERSIONED {
        let note = notes.capture("Versioned", None, None)?;
        store.append(&note.id, note_keys::TEXT, Some(value), None)?;
    }
    Ok(())
}

/// The generator's `places` scenario.
fn write_places(store: &Store) -> Built {
    let places = Places::new(store);
    let notes = Notes::new(store);
    let every: Vec<&str> = STARTER_SET
        .iter()
        .flat_map(|(area, topics)| std::iter::once(area.slug).chain(topics.iter().map(|t| t.slug)))
        .collect();
    places.plant_starter_set(&every)?;
    let starter = |slug: &str| -> Result<String, Box<dyn std::error::Error>> {
        let area = places
            .areas()?
            .into_iter()
            .find(|a| a.starter.as_deref() == Some(slug));
        Ok(area.ok_or("a starter Area")?.id)
    };
    let car = starter("car")?;
    places.rename(&car, "Bike")?;
    let kids = places.add_area("Kids")?;
    places.set_colour(&kids.id, Some("6A1B9A"))?;
    places.set_icon(&kids.id, Some("kids"))?;
    let school = places.add_topic(&kids.id, "School")?;
    let household = starter("household")?;
    let kitchen = places
        .topics(&household)?
        .into_iter()
        .find(|t| t.starter.as_deref() == Some("kitchen"))
        .ok_or("the kitchen")?;
    notes.capture("Descale the kettle", Some(&kitchen.id), None)?;
    notes.capture("Sort the cellar", Some(&household), None)?;
    notes.capture("Parents evening", Some(&school.id), None)?;
    notes.capture("New tyres", Some(&car), None)?;
    let lost = notes.capture("Where did this go?", Some(&school.id), None)?;
    store.append(&school.id, place_keys::TYPE, None, None)?;
    notes.capture("Still Unsorted", None, None)?;
    let gone = places.add_area("Gone")?;
    places.add_topic(&gone.id, "Gone too")?;
    places.delete_area(&gone.id, NotesGo::Unsorted)?;
    notes.set_importance(&lost.id, Importance::Important)?;
    Ok(())
}

/// The generator's `trash` scenario.
fn write_trash(store: &Store) -> Built {
    let notes = Notes::new(store);
    let a = notes.capture("Old shopping list", None, None)?;
    notes.trash(&a.id)?;
    let b = notes.capture("Not so old after all", None, None)?;
    notes.trash(&b.id)?;
    notes.restore(&b.id)?;
    let c = notes.capture("Long gone", None, None)?;
    store.append(
        &c.id,
        note_keys::TRASHED,
        Some(&dart_iso(utc(2025, 11, 1, 0)?)),
        None,
    )?;
    let d = notes.capture("Into the bin with the other", None, None)?;
    let e = notes.capture("The other", None, None)?;
    notes.merge(&d.id, &e.id)?;
    notes.trash(&e.id)?;
    Ok(())
}

/// The generator's `edits` scenario: what the desktop writes beyond the
/// others.
fn write_edits(store: &Store) -> Built {
    let notes = Notes::new(store);
    let places = Places::new(store);
    let home = places.add_area("Hom")?;
    places.rename(&home.id, "Home")?;
    places.set_colour(&home.id, Some("00838F"))?;
    places.set_icon(&home.id, Some("home"))?;
    let post = places.add_topic(&home.id, "Post")?;
    let bills = places.add_topic(&home.id, "Bils")?;
    places.rename(&bills.id, "Bills")?;
    let junk = places.add_topic(&home.id, "Junk")?;
    let empty = places.add_topic(&home.id, "Empty")?;
    let errands = places.add_area("Errands")?;
    places.set_colour(&errands.id, Some("C62828"))?;
    places.set_icon(&errands.id, Some("shopping"))?;
    places.set_colour(&errands.id, None)?;
    places.set_icon(&errands.id, None)?;
    let shop = places.add_topic(&errands.id, "Shop")?;
    let garage = places.add_area("Garage")?;
    let tools = places.add_topic(&garage.id, "Tools")?;
    let plumber = notes.capture("Call the plumber", None, Some(Duration::minutes(10)))?;
    let plants = notes.capture("Water the plants", None, Some(Duration::minutes(30)))?;
    let hour = Some(Notes::default_nudge());
    let passport = notes.capture("Renew the passport", None, hour)?;
    notes.capture("Pay the rent", Some(&bills.id), hour)?;
    notes.capture("Old flyer", Some(&junk.id), hour)?;
    notes.capture("Buy stamps", Some(&shop.id), hour)?;
    notes.capture("Return the drill", Some(&errands.id), hour)?;
    notes.capture("Oil the saw", Some(&tools.id), hour)?;
    notes.capture("Sweep the floor", Some(&garage.id), hour)?;
    notes.file(&plumber.id, &post.id)?;
    notes.file(&plumber.id, &bills.id)?;
    notes.file(&plants.id, &post.id)?;
    notes.file(&passport.id, &post.id)?;
    store.append(
        &passport.id,
        note_keys::SPAWNED,
        Some(&dart_iso(utc(2026, 1, 1, 12)?)),
        None,
    )?;
    places.delete_topic(&post.id, NotesGo::Unsorted)?;
    places.delete_topic(&bills.id, NotesGo::Area)?;
    places.delete_topic(&junk.id, NotesGo::Trash)?;
    places.delete_topic(&empty.id, NotesGo::Unsorted)?;
    places.delete_area(&errands.id, NotesGo::Trash)?;
    places.delete_area(&garage.id, NotesGo::Unsorted)?;
    Ok(())
}

/// The generator's `_photo`: a small JPEG, different for every `n`, with
/// an APP0, an Exif APP1, an ICC APP2 and a comment before its tables and
/// scan.
fn photo(n: u8) -> Vec<u8> {
    let segment = |marker: u8, body: &[u8]| {
        let length = (body.len() + 2) as u16;
        let mut out = vec![0xFF, marker];
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(body);
        out
    };
    let mut out = vec![0xFF, 0xD8];
    out.extend(segment(0xE0, b"JFIF\0"));
    out.extend(segment(0xE1, b"Exif\0\0GPS 52.5N 13.4E"));
    out.extend(segment(0xE2, b"ICC_PROFILE"));
    out.extend(segment(0xFE, b"taken by the fixture"));
    out.extend(segment(0xDB, &[0, n, n, n]));
    out.extend([0xFF, 0xDA, 0x00, 0x03, n, 1, 2, 3, n, 0xFF, 0xD9]);
    out
}

/// The generator's `_recording`.
fn recording(n: u8) -> Vec<u8> {
    let mut out = format!("RIFF voice {n} WAVE").into_bytes();
    out.extend([0, n, 255]);
    out
}

/// The generator's `media` scenario.
fn write_media(store: &Store) -> Built {
    let notes = Notes::new(store);
    let receipt = notes.capture_with("", None, None, &[photo(1)], &[])?;
    notes.set_photo_text(&receipt.id, &receipt.photos[0], "  RECEIPT\nTotal 12,50  ")?;
    let voice = notes.capture_with(
        "Groceries",
        None,
        Some(Duration::minutes(10)),
        &[],
        &[recording(1)],
    )?;
    notes.set_transcript(&voice.id, &voice.recordings[0], " Milk, eggs and bread ")?;
    notes.add_photo(&voice.id, &photo(1))?;
    let shed = notes.capture_with("Shed", None, None, &[photo(2), photo(3)], &[recording(2)])?;
    let key = notes.add_photo(&shed.id, &photo(4))?;
    notes.add_recording(&shed.id, &recording(3))?;
    notes.set_transcript(&shed.id, &shed.recordings[0], "")?;
    notes.set_photo_text(&shed.id, &shed.photos[0], "")?;
    let dentist = notes.capture("Dentist", None, None)?;
    let card = notes.capture_with("", None, None, &[photo(5)], &[])?;
    notes.set_photo_text(&card.id, &card.photos[0], "Dr. Müller, Tuesday 9:00")?;
    notes.merge(&card.id, &dentist.id)?;
    let memo = notes.capture_with("", None, None, &[], &[recording(4)])?;
    notes.merge(&memo.id, &dentist.id)?;
    let binned = notes.capture_with("Old flyer", None, None, &[photo(6)], &[recording(5)])?;
    notes.trash(&binned.id)?;
    let elsewhere = format!("photo:{}", "ab".repeat(32));
    store.append(&dentist.id, &elsewhere, Some(note_keys::ADDED), None)?;
    notes.set_photo_text(&shed.id, &key, "Spare key under the pot")?;
    notes.remove_photo(&receipt.id, &receipt.photos[0])?;
    notes.remove_photo(&card.id, &card.photos[0])?;
    notes.remove_recording(&voice.id, &voice.recordings[0])?;
    notes.capture_with("", None, None, &[photo(7)], &[])?;
    let hum = notes.capture_with("", None, None, &[], &[recording(6)])?;
    notes.set_transcript(&hum.id, &hum.recordings[0], "")?;
    Ok(())
}

/// The files under `dir` of a scenario's folder, by name.
/// A file's name and bytes.
type Named = (String, Vec<u8>);

fn files(dir: &Path) -> Result<Vec<Named>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            out.push((name.ok_or("a file name")?, std::fs::read(&path)?));
        }
    }
    out.sort();
    Ok(out)
}

#[test]
fn photos_and_recordings_are_kept_as_the_dart_core_keeps_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(
        &dir.path().join("oldbrainfix.db"),
        Box::new(Utc::now),
        || Ok(writer_key()),
    )?;
    write_media(&store)?;
    for sub in ["photos", "audio"] {
        let ours = files(&dir.path().join(sub))?;
        let theirs = files(&fixtures().join("media").join(sub))?;
        assert!(!theirs.is_empty(), "{sub}");
        assert_eq!(ours, theirs, "{sub}");
    }
    Ok(())
}

#[test]
fn every_scenario_written_here_is_the_dart_cores_log_byte_for_byte() -> TestResult {
    let writers: [(&str, Writer); 8] = [
        ("edits", write_edits),
        ("formats", write_formats),
        ("media", write_media),
        ("notes", write_notes),
        ("places", write_places),
        ("routines", write_routines),
        ("sections", write_sections),
        ("trash", write_trash),
    ];
    assert_eq!(writers.map(|(s, _)| s), SCENARIOS);
    for (scenario, write) in writers {
        let store = writer_store()?;
        write(&store)?;
        let ours: Vec<String> = store
            .all_entries()?
            .iter()
            .map(|e| serde_json::to_string(&e.wire()))
            .collect::<Result<_, _>>()?;
        let text = std::fs::read_to_string(fixtures().join(scenario).join("log.jsonl"))?;
        let theirs: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(ours.len(), theirs.len(), "{scenario}: entry count");
        for (i, (ours, theirs)) in ours.iter().zip(&theirs).enumerate() {
            assert_eq!(ours, theirs, "{scenario}: line {}", i + 1);
        }
    }
    Ok(())
}

// ------------------------------------------------------------- Routines

/// The European wall clock the generator's `_europe` keeps.
fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

fn at(hour: u32, minute: u32) -> Result<ClockTime, Box<dyn std::error::Error>> {
    Ok(ClockTime::new(hour, minute).ok_or("a time of day")?)
}

fn schedule(
    start: Day,
    times: &[ClockTime],
    rule: Option<&str>,
) -> Result<Schedule, Box<dyn std::error::Error>> {
    let rule = rule.map(Recurrence::parse).transpose()?;
    Ok(Schedule::new(start, times, rule).ok_or("a schedule")?)
}

fn daily() -> Option<Recurrence> {
    Some(Recurrence::every(Frequency::Daily, 1))
}

/// The generator's `routines` scenario.
fn write_routines(store: &Store) -> Built {
    let notes = Notes::new(store);
    let routines = Routines::new(store);
    let day = Day::new;
    let pills = notes.capture("Take the pills", None, None)?;
    let three = [at(20, 0)?, at(8, 0)?, at(14, 0)?];
    let medication_schedule = Schedule::new(day(2026, 1, 1), &three, daily()).ok_or("daily")?;
    let medication = routines.spawn(&pills.id, &medication_schedule)?;
    routines.set_renag(&medication.id, Some(Duration::minutes(5)))?;
    routines.set_escalation(&medication.id, Some(3))?;
    routines.spawn(
        &pills.id,
        &schedule(day(2026, 3, 1), &[], Some("FREQ=MONTHLY;BYDAY=1SU"))?,
    )?;
    let plants = notes.capture("Water the plants", None, Some(Duration::minutes(30)))?;
    let weekends = schedule(day(2026, 1, 3), &[], Some("FREQ=WEEKLY;BYDAY=SA,SU"))?;
    let weekend = routines.spawn(&plants.id, &weekends)?;
    routines.reschedule(&weekend.id, &weekends)?;
    let dentist = notes.capture("Dentist", None, None)?;
    routines.spawn(
        &dentist.id,
        &schedule(day(2026, 3, 29), &[at(2, 30)?], None)?,
    )?;
    routines.spawn(
        &dentist.id,
        &schedule(day(2026, 10, 25), &[at(2, 30)?], None)?,
    )?;
    let night = routines.create(
        "  Night meds  ",
        &schedule(
            day(2026, 1, 1),
            &[at(23, 0)?],
            Some("FREQ=DAILY;INTERVAL=2"),
        )?,
    )?;
    routines.set_renag(&night.id, Some(Duration::minutes(15)))?;
    routines.set_renag(&night.id, None)?;
    let month_end = Some("FREQ=MONTHLY;BYMONTHDAY=-1");
    let rent = routines.create(
        "Pay the rent",
        &schedule(day(2026, 1, 31), &[at(9, 0)?], month_end)?,
    )?;
    routines.reschedule(
        &rent.id,
        &schedule(day(2026, 1, 31), &[at(9, 0)?, at(18, 0)?], month_end)?,
    )?;
    let o = |d: Day, t: Option<ClockTime>| Occurrence::new(d, t);
    routines.mark_done(&medication.id, &o(day(2026, 3, 28), Some(at(8, 0)?)), None)?;
    routines.mark_done(
        &medication.id,
        &o(day(2026, 3, 28), Some(at(14, 0)?)),
        Some(utc(2026, 3, 28, 15)?),
    )?;
    routines.mark_done(&medication.id, &o(day(2026, 3, 29), Some(at(8, 0)?)), None)?;
    routines.unmark_done(&medication.id, &o(day(2026, 3, 29), Some(at(8, 0)?)))?;
    routines.mark_done(&weekend.id, &o(day(2026, 10, 24), None), None)?;
    let habit = routines.create(
        "Old habit",
        &Schedule::new(day(2026, 1, 1), &[], daily()).ok_or("daily")?,
    )?;
    routines.trash(&habit.id)?;
    let stretch = routines.create(
        "Stretch",
        &schedule(
            day(2026, 1, 2),
            &[at(7, 30)?],
            Some("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH;WKST=SU"),
        )?,
    )?;
    routines.trash(&stretch.id)?;
    routines.restore(&stretch.id)?;
    let walk = notes.capture("Walk the dog", None, None)?;
    routines.spawn(
        &walk.id,
        &Schedule::new(day(2026, 1, 1), &[at(7, 0)?], daily()).ok_or("daily")?,
    )?;
    routines.trash_note(&walk.id, true)?;
    routines.restore_note(&walk.id)?;
    let flyer = notes.capture("Flyer for the fair", None, None)?;
    routines.spawn(&flyer.id, &schedule(day(2026, 4, 1), &[at(10, 0)?], None)?)?;
    routines.trash_note(&flyer.id, false)?;
    let car = notes.capture("Car inspection", None, None)?;
    routines.spawn(&car.id, &schedule(day(2026, 5, 1), &[], None)?)?;
    routines.trash_note(&car.id, true)?;
    notes.capture("Unsorted thought", None, Some(Duration::minutes(10)))?;
    notes.capture("Another thought", None, None)?;
    Ok(())
}

fn iso(at: Option<DateTime<Utc>>) -> Value {
    json!(at.map(dart_iso))
}

fn wall(moment: &WallMoment) -> String {
    format!("{}T{}", moment.day, moment.time)
}

fn routine_json(r: &Routine) -> Value {
    let done: serde_json::Map<String, Value> = r
        .done
        .iter()
        .map(|(k, v)| (k.clone(), json!(dart_iso(*v))))
        .collect();
    json!({
        "id": r.id,
        "noteId": r.note_id,
        "start": r.schedule.start.to_string(),
        "times": r.schedule.times.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "rule": r.schedule.rule.as_ref().map(Recurrence::format),
        "done": done,
        "renag": r.renag.map(|d| d.num_minutes()),
        "escalation": r.escalation,
        "scheduledAt": iso(r.scheduled_at),
        "trashed": iso(r.trashed),
        "trashedWith": r.trashed_with,
        "reminder": r.is_reminder(),
    })
}

fn rows_json(rows: &[AgendaRow]) -> Value {
    json!(
        rows.iter()
            .map(|r| json!([
                r.routine.id,
                r.occurrence.key(),
                r.note.as_ref().map(|n| n.id.clone()),
                iso(r.done()),
            ]))
            .collect::<Vec<_>>()
    )
}

/// What this core makes of the `routines` log at one held moment, in the
/// shape of a check in the generator's `alarms.json`.
fn check_json(store: &Store, local: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    for (key, value) in local.as_object().ok_or("local settings")? {
        store.set_local_setting(key, value.as_str().ok_or("a setting")?)?;
    }
    let now = store.now();
    let alarms = Alarms::new(store);
    let agenda = Routines::new(store).agenda(&berlin)?;
    let plan = alarms.plan(&berlin)?;
    let missed = alarms.missed(&berlin)?;
    let nudge = alarms.nudge(&berlin)?;
    let later = now + Duration::hours(3);
    Ok(json!({
        "now": dart_iso(now),
        "local": local,
        "today": agenda.today.to_string(),
        "agenda": {
            "today": rows_json(&agenda.today_rows),
            "tomorrow": rows_json(&agenda.tomorrow_rows),
        },
        "alarmCount": plan.len(),
        "alarms": plan.iter().take(40).map(|a| json!({
            "id": a.id(),
            "kind": a.kind.name(),
            "routine": a.routine_id,
            "occurrence": a.occurrence.key(),
            "at": dart_iso(a.at),
            "wall": a.wall.as_ref().map(wall),
            "renag": a.renag.num_minutes(),
            "escalation": a.escalation,
            "escalatesAt": iso(a.escalates_at()),
            "ends": wall(&a.ends),
            "endsAt": dart_iso(a.ends_at),
            "text": a.text,
            "wordless": a.wordless.name(),
            "firesAfterNow": iso(a.fires_after(now)),
        })).collect::<Vec<_>>(),
        "missed": {
            "count": missed.count(),
            "nextChange": iso(missed.next_change),
            "routines": missed.routines.iter().map(|m| json!({
                "routine": m.routine.id,
                "note": m.note.as_ref().map(|n| n.id.clone()),
                "missed": m.missed.iter()
                    .map(|o| json!([o.occurrence.key(), dart_iso(o.missed_at)]))
                    .collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        },
        "nudge": nudge.map(|n| json!({
            "notes": n.notes.iter().map(|n| n.id.clone()).collect::<Vec<_>>(),
            "at": dart_iso(n.at),
            "since": iso(n.since),
            "firesAfterLater": dart_iso(n.fires_after(later, &berlin)),
        })),
        "recentlyDone": alarms.recently_done(&berlin)?.iter()
            .map(|(id, o)| json!([id, o.key()]))
            .collect::<Vec<_>>(),
    }))
}

#[test]
fn routines_and_their_alarms_come_to_what_the_dart_core_says() -> TestResult {
    let expected = read_json(&fixtures().join("routines").join("alarms.json"))?;
    let read_at = read_json(&fixtures().join("routines").join("expected.json"))?;
    let store = load("routines", read_at["now"].as_str().ok_or("now")?)?;
    let routines = Routines::new(&store);
    let mut ids = store.entities_of_type("routine")?;
    ids.sort();
    let mut every = Vec::new();
    for id in &ids {
        if let Some(routine) = routines.by_id(id)? {
            every.push(routine_json(&routine));
        }
    }
    assert_eq!(json!(every), expected["routines"]);
    let all: Vec<String> = routines.all()?.into_iter().map(|r| r.id).collect();
    assert_eq!(json!(all), expected["all"]);
    let trashed: Vec<String> = routines.trashed()?.into_iter().map(|r| r.id).collect();
    assert_eq!(json!(trashed), expected["trashed"]);
    let mut of_note = serde_json::Map::new();
    for note in Notes::new(&store).all()? {
        let ids: Vec<String> = routines
            .of_note(&note.id)?
            .into_iter()
            .map(|r| r.id)
            .collect();
        of_note.insert(note.id, json!(ids));
    }
    assert_eq!(Value::Object(of_note), expected["ofNote"]);
    let checks = expected["checks"].as_array().ok_or("checks")?;
    assert_eq!(checks.len(), 5);
    for check in checks {
        let store = load("routines", check["now"].as_str().ok_or("now")?)?;
        let ours = check_json(&store, &check["local"])?;
        for key in [
            "today",
            "agenda",
            "alarmCount",
            "alarms",
            "missed",
            "nudge",
            "recentlyDone",
        ] {
            assert_eq!(ours[key], check[key], "{} {key}", check["now"]);
        }
    }
    Ok(())
}

#[test]
fn rules_schedules_and_instants_come_out_as_the_dart_core_has_them() -> TestResult {
    let fixture = read_json(&fixtures().join("schedules.json"))?;
    let simple_json = |s: Option<SimpleRepeat>| {
        json!(s.map(|s| json!({
            "kind": match s.kind {
                RepeatKind::Once => "once",
                RepeatKind::Every => "every",
                RepeatKind::Weekdays => "weekdays",
                RepeatKind::MonthDay => "monthDay",
                RepeatKind::MonthWeekday => "monthWeekday",
            },
            "every": s.every,
            "unit": format!("{:?}", s.unit).to_lowercase(),
            "weekdays": s.weekdays.iter().collect::<Vec<_>>(),
            "monthDay": s.month_day,
            "nth": s.nth.map(|n| n.to_string()),
            "end": format!("{:?}", s.end()).to_lowercase(),
            "count": s.count,
            "until": s.until.map(|d| d.to_string()),
            "rule": s.to_rule().map(|r| r.format()),
        })))
    };
    for case in fixture["rules"].as_array().ok_or("rules")? {
        let text = case["text"].as_str().ok_or("text")?;
        let ours = match Recurrence::parse(text) {
            Ok(rule) => json!({
                "text": text,
                "format": rule.format(),
                "times": rule.times().map(|t| t.iter().map(ToString::to_string).collect::<Vec<_>>()),
                "simple": simple_json(SimpleRepeat::of(Some(&rule))),
            }),
            Err(e) => json!({"text": text, "error": e.message, "source": e.source}),
        };
        assert_eq!(&ours, case, "{text:?}");
    }
    assert_eq!(simple_json(SimpleRepeat::of(None)), fixture["simpleOnce"]);
    for case in fixture["schedules"].as_array().ok_or("schedules")? {
        let s = |k: &str| case[k].as_str().ok_or(format!("{k} is text"));
        let times: Vec<ClockTime> = case["times"]
            .as_array()
            .ok_or("times")?
            .iter()
            .filter_map(|t| t.as_str().and_then(ClockTime::parse))
            .collect();
        let start = Day::parse(s("start")?).ok_or("start")?;
        let from = Day::parse(s("from")?).ok_or("from")?;
        let to = Day::parse(s("to")?).ok_or("to")?;
        let schedule = schedule(start, &times, case["rule"].as_str())?;
        let keys =
            |list: Vec<Occurrence>| json!(list.iter().map(Occurrence::key).collect::<Vec<_>>());
        let day_times: Vec<String> = schedule
            .day_times()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(json!(day_times), case["dayTimes"], "{case}");
        assert_eq!(keys(schedule.between(from, to)), case["between"], "{case}");
        assert_eq!(keys(schedule.next(from, 8)), case["next"], "{case}");
    }
    for case in fixture["days"].as_array().ok_or("days")? {
        let n: Vec<i64> = serde_json::from_value(case["in"].clone())?;
        let [y, m, d] = n[..] else {
            return Err("three numbers".into());
        };
        let day = Day::new(y, m, d);
        assert_eq!(json!(day.to_string()), case["day"]);
        assert_eq!(json!(day.weekday()), case["weekday"]);
    }
    for case in fixture["instants"].as_array().ok_or("instants")? {
        let day = Day::parse(case[0].as_str().ok_or("day")?).ok_or("a day")?;
        let time = ClockTime::parse(case[1].as_str().ok_or("time")?).ok_or("a time")?;
        assert_eq!(
            json!(dart_iso(instant_of(day, time, &berlin))),
            case[2],
            "{case}"
        );
    }
    for case in fixture["quiet"].as_array().ok_or("quiet")? {
        let hours = case[0].as_str().ok_or("hours")?;
        let (start, end) = hours.split_once('-').ok_or("from-to")?;
        let quiet = QuietHours {
            start: ClockTime::parse(start).ok_or("start")?,
            end: ClockTime::parse(end).ok_or("end")?,
        };
        let instant = parse_time(case[1].as_str().ok_or("instant")?)?;
        assert_eq!(
            json!(dart_iso(quiet.defer(instant, &berlin))),
            case[2],
            "{case}"
        );
    }
    Ok(())
}

// ------------------------------------------------------------ LAN sync

#[test]
fn pair_codes_are_written_and_read_as_the_dart_core_does() -> TestResult {
    use oldbrainfix_core::lan::{decode_pair_code, encode_pair_code};
    let fixture = read_json(&fixtures().join("pair_codes.json"))?;
    for code in fixture["codes"].as_array().ok_or("codes")? {
        let s = |k: &str| code[k].as_str().ok_or(format!("{k} is text"));
        let host: std::net::Ipv4Addr = s("host")?.parse()?;
        let port = u16::try_from(code["port"].as_u64().ok_or("port")?)?;
        let fingerprint = hex::decode(s("fingerprint")?)?;
        assert_eq!(
            encode_pair_code(host, port, s("pin")?, &fingerprint, false),
            s("full")?
        );
        assert_eq!(
            encode_pair_code(host, port, s("pin")?, &fingerprint, true),
            s("typed")?
        );
        let full = decode_pair_code(s("full")?).ok_or("a code")?;
        assert_eq!(
            (full.host.as_str(), full.port, full.pin.as_str()),
            (s("host")?, port, s("pin")?)
        );
        assert_eq!(full.fingerprint, fingerprint);
        let typed = decode_pair_code(s("typed")?).ok_or("a code")?;
        assert_eq!(typed.fingerprint, fingerprint[..8]);
    }
    for (typed, code) in fixture["typedAs"].as_object().ok_or("typedAs")? {
        assert_eq!(
            decode_pair_code(typed),
            decode_pair_code(code.as_str().ok_or("a code")?)
        );
    }
    for invalid in fixture["invalid"].as_array().ok_or("invalid")? {
        assert_eq!(decode_pair_code(invalid.as_str().ok_or("text")?), None);
    }
    Ok(())
}

#[test]
fn key_records_travel_as_the_dart_core_writes_them() -> TestResult {
    use oldbrainfix_core::KeyRecord;
    let fixture = read_json(&fixtures().join("keys.json"))?;
    let record = KeyRecord::make(&writer_key());
    assert_eq!(record.to_json(), fixture["record"]);
    assert!(KeyRecord::from_json(&fixture["record"]).is_some_and(|r| r.valid()));
    assert!(KeyRecord::from_json(&fixture["claimed"]).is_some_and(|r| !r.valid()));
    Ok(())
}

/// The joiner's key: seed bytes 33 to 64.
fn joiner_key() -> SigningKey {
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8 + 33;
    }
    SigningKey::from_seed(seed)
}

/// One Device of the sync fixture: its log, the keys it holds and its
/// blobs, the clock held at `now`.
fn sync_device(
    name: &str,
    key: fn() -> SigningKey,
    now: &str,
) -> Result<Store, Box<dyn std::error::Error>> {
    fixture_device("sync", name, key, now)
}

/// One Device of a two-Device fixture under `scenario`.
fn fixture_device(
    scenario: &str,
    name: &str,
    key: fn() -> SigningKey,
    now: &str,
) -> Result<Store, Box<dyn std::error::Error>> {
    let dir = fixtures().join(scenario);
    let now = parse_time(now)?;
    let store = Store::in_memory(Box::new(move || now), move || Ok(key()))?;
    store.set_local_setting("author", "Ada")?;
    let text = std::fs::read_to_string(dir.join(format!("{name}.jsonl")))?;
    for line in text.lines().filter(|l| !l.is_empty()) {
        store.insert(&serde_json::from_str(line)?)?;
    }
    let settings = read_json(&dir.join("settings.json"))?;
    for pair in settings[name].as_array().ok_or("settings")? {
        store.set_local_setting(
            pair[0].as_str().ok_or("a key")?,
            pair[1].as_str().ok_or("a value")?,
        )?;
    }
    for kind in ["photo", "audio"] {
        let sub = if kind == "photo" { "photos" } else { "audio" };
        for (hash, bytes) in files(&dir.join(name).join(sub))? {
            assert!(store.put_blob(kind, &hash, &bytes)?, "{name} {hash}");
        }
    }
    Ok(store)
}

#[test]
fn a_sync_session_goes_as_the_dart_core_runs_it() -> TestResult {
    use oldbrainfix_core::lan::{answer_sync, apply_answer, sync_request};
    let dir = fixtures().join("sync");
    let mut expected = read_json(&dir.join("expected.json"))?;
    let now = expected["now"].as_str().ok_or("now")?.to_string();
    let host = sync_device("host", writer_key, &now)?;
    let joiner = sync_device("joiner", joiner_key, &now)?;
    let vector: std::collections::HashMap<String, i64> =
        serde_json::from_value(read_json(&dir.join("vector.json"))?)?;
    assert_eq!(serde_json::to_value(host.version_vector()?)?, json!(vector));

    // The joiner asks as the phone asks.
    let request = read_json(&dir.join("request.json"))?;
    assert_eq!(
        sync_request(&joiner, &vector, "Phone", &host.device_id()?, "246810")?,
        request
    );

    // The host answers as the Dart core answers.
    let (answer, session) = answer_sync(&host, &request, "Desk")?.ok_or("an answer")?;
    assert_eq!(answer, read_json(&dir.join("answer.json"))?);
    assert_eq!(session.name, "Phone");
    assert!(session.moment.is_some());
    assert_eq!(session.report.refused_count(), 0);

    // Both end up reading the same, with the kept Profile's backup key.
    let back = apply_answer(&joiner, &answer)?;
    // The joiner, still with the defaults, took the host's Profile over.
    let profiles = read_json(&dir.join("profile.json"))?;
    assert_eq!(profile_settings(&host)?, profiles["host"]);
    assert_eq!(profile_settings(&joiner)?, profiles["joiner"]);
    assert_eq!(back.profile.name(), "adopted");
    assert_eq!(back.name, "Desk");
    let held = read_json(&dir.join("backup_keys.json"))?;
    for (side, store) in [("host", &host), ("joiner", &joiner)] {
        assert_eq!(backup_settings(store)?, held[side], "{side}");
    }
    assert_eq!(host.recovery_code()?, joiner.recovery_code()?);
    assert!(!back.applied.is_empty());
    if let Some(map) = expected.as_object_mut() {
        map.remove("device");
    }
    for (side, store) in [("host", &host), ("joiner", &joiner)] {
        let mut actual = read(store)?;
        if let Some(map) = actual.as_object_mut() {
            map.remove("device");
        }
        assert_eq!(actual, expected, "{side}");
    }
    Ok(())
}

#[test]
fn conflicts_are_found_and_picked_as_the_dart_core_does() -> TestResult {
    use oldbrainfix_core::lan::{answer_sync, apply_answer, sync_request};
    let dir = fixtures().join("conflicts");
    let expected = read_json(&dir.join("expected.json"))?;
    let now = expected["now"].as_str().ok_or("now")?.to_string();
    let host = fixture_device("conflicts", "host", writer_key, &now)?;
    let joiner = fixture_device("conflicts", "joiner", joiner_key, &now)?;
    let vector: std::collections::HashMap<String, i64> =
        serde_json::from_value(read_json(&dir.join("vector.json"))?)?;
    let request = read_json(&dir.join("request.json"))?;
    assert_eq!(sync_request(&joiner, &vector, "Phone", "", "")?, request);

    // Both sides find the same Conflicts at import, and nothing is
    // written for them.
    let (answer, session) = answer_sync(&host, &request, "Desk")?.ok_or("an answer")?;
    assert_eq!(answer, read_json(&dir.join("answer.json"))?);
    let back = apply_answer(&joiner, &answer)?;
    let found = &expected["conflicts"];
    assert_eq!(&conflicts_json(&session.conflicts), found);
    assert_eq!(&conflicts_json(&back.conflicts), found);
    assert_eq!(&conflicts_json(&host.conflicts()?), found);
    assert_eq!(&conflicts_json(&joiner.conflicts()?), found);

    // The host picks as the Dart core picked, to the same entries.
    let mark = host.current_seq()?;
    for pick in expected["picks"].as_array().ok_or("picks")? {
        let entity = pick[0].as_str().ok_or("entity")?;
        let unit = pick[1].as_str().ok_or("unit")?;
        let index = pick[2].as_u64().ok_or("index")? as usize;
        let conflict = host.conflict_on(entity, unit)?.ok_or("a conflict")?;
        host.resolve_conflict(&conflict, &conflict.versions[index])?;
    }
    let ours: Vec<String> = host
        .entries_after(mark)?
        .iter()
        .map(|e| serde_json::to_string(&e.wire()))
        .collect::<Result<_, _>>()?;
    let text = std::fs::read_to_string(dir.join("picks.jsonl"))?;
    let theirs: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(ours, theirs);
    assert!(host.conflicts()?.is_empty());

    // A second session carries the picks over: nothing open on either.
    let request = sync_request(&joiner, &host.version_vector()?, "Phone", "", "")?;
    let (answer, _) = answer_sync(&host, &request, "Desk")?.ok_or("an answer")?;
    let back = apply_answer(&joiner, &answer)?;
    assert!(back.conflicts.is_empty());
    let mut after = expected["after"].clone();
    if let Some(map) = after.as_object_mut() {
        map.remove("device");
    }
    for (side, store) in [("host", &host), ("joiner", &joiner)] {
        let mut actual = read(store)?;
        if let Some(map) = actual.as_object_mut() {
            map.remove("device");
        }
        assert_eq!(actual, after, "{side}");
    }
    Ok(())
}

// --------------------------------------------------------- Folder sync

#[test]
fn messages_seal_key_ids_and_blob_names_come_out_as_the_dart_core_has_them() -> TestResult {
    use oldbrainfix_core::keys::{area_key_id, blob_name, seal, unseal};
    let fixture = read_json(&fixtures().join("crypto.json"))?;
    for case in fixture["sealed"].as_array().ok_or("sealed")? {
        let key: [u8; 32] = hex::decode(case["key"].as_str().ok_or("key")?)?
            .try_into()
            .map_err(|_| "a 32-byte key")?;
        let nonce: [u8; 24] = hex::decode(case["nonce"].as_str().ok_or("nonce")?)?
            .try_into()
            .map_err(|_| "a 24-byte nonce")?;
        let aad = case["aad"].as_str().ok_or("aad")?;
        let plain = case["plain"].as_str().ok_or("plain")?;
        let sealed = hex::decode(case["sealed"].as_str().ok_or("sealed")?)?;
        assert_eq!(seal(&key, plain.as_bytes(), aad, &nonce)?, sealed, "{aad}");
        assert_eq!(
            unseal(&key, &sealed, aad).as_deref(),
            Some(plain.as_bytes())
        );
    }
    let id = &fixture["keyId"];
    assert_eq!(
        area_key_id(&hex::decode(id["key"].as_str().ok_or("key")?)?),
        id["id"].as_str().ok_or("id")?
    );
    let name = &fixture["blobName"];
    assert_eq!(
        blob_name(
            &hex::decode(name["key"].as_str().ok_or("key")?)?,
            name["ref"].as_str().ok_or("ref")?
        ),
        name["name"].as_str().ok_or("name")?
    );
    Ok(())
}

/// Copies the folder the Dart core wrote into `to`, file for file.
fn copy_folder(from: &Path, to: &Path) -> TestResult {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            copy_folder(&path, &to.join(entry.file_name()))?;
        } else {
            std::fs::copy(&path, to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Every file under `dir` with its bytes, by path.
fn tree(dir: &Path) -> Result<Vec<Named>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            for (sub, bytes) in tree(&path)? {
                out.push((format!("{}/{sub}", name.clone().unwrap_or_default()), bytes));
            }
        } else {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            out.push((name.unwrap_or_default(), std::fs::read(&path)?));
        }
    }
    out.sort();
    Ok(out)
}

/// The nonces the generator counted from 1001 on.
fn counted_nonces() -> impl FnMut() -> oldbrainfix_core::Result<[u8; 24]> {
    counted_nonces_after(1000)
}

/// The nonces the generator counted from `start` + 1 on.
fn counted_nonces_after(start: u64) -> impl FnMut() -> oldbrainfix_core::Result<[u8; 24]> {
    let mut n: u64 = start;
    move || {
        n += 1;
        Ok(std::array::from_fn(|i| ((n * 37 + i as u64) & 0xff) as u8))
    }
}

#[test]
fn the_folder_the_dart_core_wrote_is_read_here_and_written_the_same() -> TestResult {
    use oldbrainfix_core::folder::{LocalFolder, ROOT, folder_sync, random_nonce};
    let dir = fixtures().join("folder");
    let expected = read_json(&dir.join("expected.json"))?;
    let mut read_back = expected["read"].clone();
    let now = read_back["now"].as_str().ok_or("now")?.to_string();

    // The joiner holds the keys handed over at Pairing and reads what the
    // host put in the folder.
    let joiner = fixture_device("folder", "joiner", joiner_key, &now)?;
    let work = tempfile::tempdir()?;
    copy_folder(&dir.join(ROOT), &work.path().join(ROOT))?;
    let result = folder_sync(
        &joiner,
        &mut LocalFolder::new(work.path()),
        &mut random_nonce,
    )?;
    assert_eq!(
        result.applied.len() as u64,
        expected["applied"].as_u64().ok_or("applied")?
    );
    assert_eq!(
        result.blobs_in as u64,
        expected["blobsIn"].as_u64().ok_or("blobsIn")?
    );
    assert_eq!(result.unreadable, 0);
    assert_eq!(result.report.refused_count(), 0);
    // The backup key came in the host's keys file.
    assert_eq!(backup_settings(&joiner)?, expected["backupKey"]);
    // So did the host's Profile, renamed since they met.
    assert!(result.profile_changed);
    assert_eq!(
        profile_settings(&joiner)?,
        read_json(&dir.join("profile.json"))?["joiner"]
    );
    let mut actual = read(&joiner)?;
    if let Some(map) = actual.as_object_mut() {
        map.remove("device");
    }
    if let Some(map) = read_back.as_object_mut() {
        map.remove("device");
    }
    assert_eq!(actual, read_back);

    // The host writes the very same folder here, nonce for nonce.
    let host = fixture_device("folder", "host", writer_key, &now)?;
    let ours = tempfile::tempdir()?;
    let mut nonces = counted_nonces();
    let wrote = folder_sync(&host, &mut LocalFolder::new(ours.path()), &mut nonces)?;
    assert_eq!(
        wrote.entries_out as u64,
        expected["entriesOut"].as_u64().ok_or("entriesOut")?
    );
    assert_eq!(
        wrote.blobs_out as u64,
        expected["blobsOut"].as_u64().ok_or("blobsOut")?
    );
    let theirs = tree(&dir.join(ROOT))?;
    let mine = tree(&ours.path().join(ROOT))?;
    assert_eq!(
        mine.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
        theirs.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()
    );
    for ((name, a), (_, b)) in mine.iter().zip(&theirs) {
        assert!(a == b, "{name} differs");
    }
    Ok(())
}

// ------------------------------------------------------------- Bundles

/// The files inside the zip at `path` with their bytes, by name.
fn zip_files(path: &Path) -> Result<Vec<Named>, Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        out.push((file.name()?.into_owned(), bytes));
    }
    out.sort();
    Ok(out)
}

/// The zip containers differ (each core's zip library writes its own
/// headers); the files inside are compared byte for byte.
#[test]
fn the_bundle_the_dart_core_wrote_is_read_here_and_written_the_same() -> TestResult {
    use oldbrainfix_core::bundle::{import_bundle, write_bundle};
    let dir = fixtures().join("bundle");
    let expected = read_json(&dir.join("expected.json"))?;
    let mut read_back = expected["read"].clone();
    let now = read_back["now"].as_str().ok_or("now")?.to_string();
    let theirs = dir.join("host.obfsync");

    let joiner = fixture_device("bundle", "joiner", joiner_key, &now)?;
    let host = fixture_device("bundle", "host", writer_key, &now)?;
    let got = import_bundle(&joiner, &theirs)?;
    assert_eq!(
        got.applied.len() as u64,
        expected["applied"].as_u64().ok_or("applied")?
    );
    assert_eq!(
        got.blobs_in as u64,
        expected["blobsIn"].as_u64().ok_or("blobsIn")?
    );
    assert_eq!(got.unreadable, 0);
    assert_eq!(got.report.refused_count(), 0);
    assert_eq!(got.from, host.device_id()?);
    // Both Profiles named by the user: "Which one is you?" waits.
    assert!(got.profile_changed);
    assert_eq!(
        profile_settings(&joiner)?,
        read_json(&dir.join("profile.json"))?["joiner"]
    );
    assert_eq!(
        joiner
            .local_setting(&format!("held:{}", host.device_id()?))?
            .as_deref(),
        expected["held"].as_str()
    );
    let mut actual = read(&joiner)?;
    if let Some(map) = actual.as_object_mut() {
        map.remove("device");
    }
    if let Some(map) = read_back.as_object_mut() {
        map.remove("device");
    }
    assert_eq!(actual, read_back);

    // The host writes the very same files for the joiner, nonce for nonce.
    let ours = tempfile::tempdir()?;
    let path = ours.path().join("host.obfsync");
    let mut nonces = counted_nonces_after(2000);
    let (entries, blobs) =
        write_bundle(&host, &path, Some(&joiner.device_id()?), &mut nonces)?.ok_or("a bundle")?;
    assert_eq!(
        entries as u64,
        expected["entries"].as_u64().ok_or("entries")?
    );
    assert_eq!(blobs as u64, expected["blobs"].as_u64().ok_or("blobs")?);
    let theirs = zip_files(&theirs)?;
    let mine = zip_files(&path)?;
    assert_eq!(
        mine.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
        theirs.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()
    );
    for ((name, a), (_, b)) in mine.iter().zip(&theirs) {
        assert!(a == b, "{name} differs");
    }
    Ok(())
}

// -------------------------------------------------------- Shared Areas

/// A clock held where a test puts it.
fn held_clock(
    at: &str,
) -> Result<std::sync::Arc<parking_lot::Mutex<DateTime<Utc>>>, Box<dyn std::error::Error>> {
    Ok(std::sync::Arc::new(parking_lot::Mutex::new(parse_time(
        at,
    )?)))
}

/// One Device of the `shared` fixture: the log `file`, the keys its side
/// of `settings.json` holds, on the held `clock`.
fn shared_device(
    file: &str,
    side: &str,
    key: fn() -> SigningKey,
    clock: &std::sync::Arc<parking_lot::Mutex<DateTime<Utc>>>,
) -> Result<Store, Box<dyn std::error::Error>> {
    let dir = fixtures().join("shared");
    let held = clock.clone();
    let store = Store::in_memory(Box::new(move || *held.lock()), move || Ok(key()))?;
    store.set_local_setting("author", "Ada")?;
    insert_lines(&store, &dir.join(file))?;
    let settings = read_json(&dir.join("settings.json"))?;
    for pair in settings[side].as_array().ok_or("settings")? {
        store.set_local_setting(
            pair[0].as_str().ok_or("a key")?,
            pair[1].as_str().ok_or("a value")?,
        )?;
    }
    Ok(store)
}

fn insert_lines(store: &Store, path: &Path) -> TestResult {
    let text = std::fs::read_to_string(path)?;
    for line in text.lines().filter(|l| !l.is_empty()) {
        store.insert(&serde_json::from_str(line)?)?;
    }
    Ok(())
}

/// The entries `store` wrote after `mark`, as lines of the log files.
fn lines_after(store: &Store, mark: i64) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(store
        .entries_after(mark)?
        .iter()
        .map(|e| serde_json::to_string(&e.wire()))
        .collect::<Result<_, _>>()?)
}

fn lines_of(path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// What the roles of `area` come to on `store`, in the shape of the
/// generator's `_roles`.
fn roles_json(
    store: &Store,
    area: &str,
    ids: &[String],
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut may = serde_json::Map::new();
    let mut authors = serde_json::Map::new();
    for id in ids {
        may.insert(id.clone(), json!(store.may_change(id)?));
        authors.insert(id.clone(), json!(store.author_of(id)?.map(|p| p.id)));
    }
    let mut people = Vec::new();
    for p in store.people(area)? {
        people.push(json!({
            "id": p.id,
            "name": store.person_shown(&p.id)?.name,
            "role": p.role.name(),
            "devices": p.devices,
        }));
    }
    let mut ignored: Vec<String> = store.judged()?.ignored.iter().cloned().collect();
    ignored.sort();
    let mut routines = Vec::new();
    let mut ids_of = store.entities_of_type("routine")?;
    ids_of.sort();
    for id in ids_of {
        if let Some(r) = Routines::new(store).by_id(&id)? {
            routines.push(routine_json(&r));
        }
    }
    Ok(json!({
        "access": store.access_in(area, None)?.name(),
        "shared": store.is_shared(area)?,
        "people": people,
        "authors": authors,
        "mayChange": may,
        "mayManage": store.may_manage(Some(area))?,
        "ignored": ignored,
        "routines": routines,
    }))
}

/// The Profiles of other people `store` keeps, signed as received, as
/// the generator's `_people`.
fn people_json(store: &Store) -> Result<Value, Box<dyn std::error::Error>> {
    let mut out = serde_json::Map::new();
    for (key, value) in store.local_settings_with_prefix("person:")? {
        let held: Value = serde_json::from_str(&value)?;
        out.insert(key, held["profile"].clone());
    }
    Ok(Value::Object(out))
}

fn without_device(mut value: Value) -> Value {
    if let Some(map) = value.as_object_mut() {
        map.remove("device");
    }
    value
}

#[test]
fn a_shared_area_goes_as_the_dart_core_runs_it() -> TestResult {
    use oldbrainfix_core::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
    };
    use oldbrainfix_core::{AreaRole, Invite, OwnProfile, PairedDevice};
    let dir = fixtures().join("shared");
    let expected = read_json(&dir.join("expected.json"))?;
    let at = |key: &str| -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
        Ok(parse_time(expected[key].as_str().ok_or("a time")?)?)
    };
    let clock = held_clock(expected["shareAt"].as_str().ok_or("shareAt")?)?;
    let mut ann = shared_device("before.jsonl", "host", writer_key, &clock)?;
    let ids = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(
        expected["ids"].as_u64().ok_or("ids")?,
    ));
    ann.set_id_maker(Box::new(move || {
        let n = ids.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(format!("{n:032x}"))
    }));

    // Sharing the starter Household moves it and makes Ann its Manager,
    // to the same entries.
    let mark = ann.current_seq()?;
    let area = ann.share(&oldbrainfix_core::places::starter_id("area", "household"))?;
    assert_eq!(area, expected["area"].as_str().ok_or("area")?);
    assert_eq!(
        lines_after(&ann, mark)?,
        lines_of(&dir.join("share.jsonl"))?
    );
    for pair in expected["keysAfterShare"].as_array().ok_or("keys")? {
        ann.set_local_setting(
            pair[0].as_str().ok_or("a key")?,
            pair[1].as_str().ok_or("a value")?,
        )?;
    }

    // The invite session: Ben sends nothing of his own, Ann only the Area.
    *clock.lock() = at("sessionAt")?;
    let ben = shared_device("joiner.jsonl", "joiner", joiner_key, &clock)?;
    let ann_id = ann.device_id()?;
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann_id,
        "246810",
        Joining::Invited,
    )?;
    assert_eq!(request, read_json(&dir.join("request.json"))?);
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    let (answer, session) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    assert_eq!(answer, read_json(&dir.join("answer.json"))?);
    assert_eq!(session.shared_area.as_deref(), Some(area.as_str()));
    let back = apply_answer_as(&ben, &answer, true, None)?;
    assert_eq!(back.shared_area.as_deref(), Some(area.as_str()));
    // A plain joiner is not answered by a host that invites.
    let plain = sync_request(&ben, &ann.version_vector()?, "Phone", &ann_id, "")?;
    assert!(answer_sync_inviting(&ann, &plain, "Desk", Some(&invite))?.is_none());
    let now = at("sessionAt")?;
    for (store, peer, name) in [
        (&ann, ben.device_id()?, "Phone"),
        (&ben, ann_id.clone(), "Desk"),
    ] {
        store.remember_device(&PairedDevice {
            id: peer,
            name: name.into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: now,
            shared: true,
        })?;
    }
    let after_invite = &expected["afterInvite"];
    assert_eq!(without_device(read(&ann)?), after_invite["host"]);
    assert_eq!(without_device(read(&ben)?), after_invite["joiner"]);
    assert_eq!(people_json(&ann)?, after_invite["hostPeople"]);
    assert_eq!(people_json(&ben)?, after_invite["joinerPeople"]);

    // Ann gives Ben a name and picture for herself alone: the same
    // entries, every view shows them, and they never reach Ben.
    *clock.lock() = at("overrideAt")?;
    let overridden = &expected["override"];
    let ben_id = overridden["person"].as_str().ok_or("person")?;
    let mark = ann.current_seq()?;
    ann.set_override_name(ben_id, "Benny")?;
    ann.set_override_picture(ben_id, Some(&std::fs::read(dir.join("override.jpg"))?))?;
    assert_eq!(
        lines_after(&ann, mark)?,
        lines_of(&dir.join("override.jsonl"))?
    );
    assert_eq!(json!(ann.person_shown(ben_id)?.name), overridden["shown"]);
    assert_eq!(json!(ann.person_theirs(ben_id)?.name), overridden["theirs"]);

    // Ann renames herself and syncs through a shared folder: the
    // Household's Profiles go sealed under its key, as the Dart core
    // sealed them, and Ben reads hers.
    *clock.lock() = at("folderAt")?;
    OwnProfile::new(&ann).rename("Ann Lovelace")?;
    let mut folder = oldbrainfix_core::folder::MemoryFolder::default();
    let mut nonces = counted_nonces_after(2000);
    oldbrainfix_core::folder::folder_sync(&ann, &mut folder, &mut nonces)?;
    let after_folder = &expected["afterFolder"];
    let file = after_folder["profilesFile"].as_str().ok_or("a file")?;
    assert_eq!(
        oldbrainfix_core::folder::SyncFolder::read(&folder, "keys", file)?,
        Some(std::fs::read(dir.join(file))?)
    );
    let taken = oldbrainfix_core::folder::folder_sync(
        &ben,
        &mut folder,
        &mut oldbrainfix_core::folder::random_nonce,
    )?;
    assert!(taken.profile_changed);
    assert_eq!(people_json(&ben)?, after_folder["joinerPeople"]);

    // What Ben wrote, allowed and not, and Ann filing a Note in; then a
    // paired session: only the Area goes, the filed Note whole.
    insert_lines(&ben, &dir.join("member.jsonl"))?;
    insert_lines(&ann, &dir.join("owner.jsonl"))?;
    *clock.lock() = at("syncAt")?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann_id, "")?;
    assert_eq!(request, read_json(&dir.join("request2.json"))?);
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    assert_eq!(answer, read_json(&dir.join("answer2.json"))?);
    apply_answer_as(&ben, &answer, true, None)?;
    assert!(
        ben.all_entries()?
            .iter()
            .all(|e| !e.entity.starts_with(oldbrainfix_core::overrides::PREFIX)),
        "Ann's Override reached Ben"
    );
    let after = &expected["after"];
    let ids: Vec<String> = after["hostRoles"]["mayChange"]
        .as_object()
        .ok_or("mayChange")?
        .keys()
        .cloned()
        .collect();
    assert_eq!(without_device(read(&ann)?), after["host"]);
    assert_eq!(without_device(read(&ben)?), after["joiner"]);
    assert_eq!(roles_json(&ann, &area, &ids)?, after["hostRoles"]);
    assert_eq!(roles_json(&ben, &area, &ids)?, after["joinerRoles"]);

    // Ann makes Ben a Manager, then removes him: the same entries, and
    // nothing more goes to him.
    let person = expected["person"].as_str().ok_or("person")?;
    *clock.lock() = at("roleAt")?;
    let mark = ann.current_seq()?;
    ann.set_role(&area, person, AreaRole::Manager)?;
    assert_eq!(lines_after(&ann, mark)?, lines_of(&dir.join("role.jsonl"))?);
    // Ben learns of it and writes as a Manager: that counts now, what he
    // wrote as a Member still not.
    let session = |at: DateTime<Utc>| -> TestResult {
        *clock.lock() = at;
        let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann_id, "")?;
        let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
        apply_answer_as(&ben, &answer, true, None)?;
        Ok(())
    };
    session(at("promoteSyncAt")?)?;
    insert_lines(&ben, &dir.join("manager.jsonl"))?;
    session(at("managerSyncAt")?)?;
    assert_eq!(roles_json(&ann, &area, &ids)?, expected["promoted"]["host"]);
    assert_eq!(
        roles_json(&ben, &area, &ids)?,
        expected["promoted"]["joiner"]
    );
    *clock.lock() = at("removeAt")?;
    let mark = ann.current_seq()?;
    ann.remove_person(&area, person)?;
    assert_eq!(
        lines_after(&ann, mark)?,
        lines_of(&dir.join("remove.jsonl"))?
    );
    *clock.lock() = at("lastAt")?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann_id, "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    assert_eq!(answer, read_json(&dir.join("answer3.json"))?);
    assert_eq!(roles_json(&ann, &area, &ids)?, expected["removed"]);
    Ok(())
}

// --------------------------------------------------------------- Blocks

/// What blocking left on `store`, in the shape of the generator's
/// `_blockState`: the entries it holds, the vector it tells, its refusal
/// marks, its Profiles of other people as kept, the people blocked and
/// shown, the photos of `photos` it holds and what it reads.
fn block_state(store: &Store, photos: &[&str]) -> Result<Value, Box<dyn std::error::Error>> {
    let pairs = |prefix: &str| -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        Ok(store
            .local_settings_with_prefix(prefix)?
            .into_iter()
            .map(|(key, value)| json!([key, value]))
            .collect())
    };
    let vector: std::collections::BTreeMap<String, i64> =
        store.version_vector()?.into_iter().collect();
    let blocked: Vec<Value> = store
        .blocked_people()?
        .iter()
        .map(|p| json!({"id": p.id, "name": p.name, "picture": p.picture.is_some()}))
        .collect();
    let shown: Vec<String> = store.people_shown()?.into_iter().map(|p| p.id).collect();
    let held: Vec<&str> = photos
        .iter()
        .filter(|hash| store.photo(hash).is_some())
        .copied()
        .collect();
    Ok(json!({
        "entries": store.all_entries()?.iter().map(|e| e.id()).collect::<Vec<_>>(),
        "vector": vector,
        "refused": pairs("refused:")?,
        "people": pairs("person:")?,
        "blocked": blocked,
        "shown": shown,
        "photos": held,
        "read": without_device(read(store)?),
    }))
}

/// The entries of the log file `path`, as they came.
fn entries_in(path: &Path) -> Result<Vec<WireEntry>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    for line in lines_of(path)? {
        out.push(serde_json::from_str(&line)?);
    }
    Ok(out)
}

#[test]
fn blocking_a_person_goes_as_the_dart_core_runs_it() -> TestResult {
    use oldbrainfix_core::{Error, ImportReport};
    let dir = fixtures().join("block");
    let expected = read_json(&dir.join("expected.json"))?;
    let at = |key: &str| -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
        Ok(parse_time(expected[key].as_str().ok_or("a time")?)?)
    };
    let text = |key: &str| -> Result<&str, Box<dyn std::error::Error>> {
        Ok(expected[key].as_str().ok_or("a text")?)
    };
    let clock = held_clock(text("blockAt")?)?;
    let held = clock.clone();
    let ann = Store::in_memory(Box::new(move || *held.lock()), || Ok(writer_key()))?;
    insert_lines(&ann, &dir.join("before.jsonl"))?;
    for pair in read_json(&dir.join("settings.json"))?
        .as_array()
        .ok_or("settings")?
    {
        ann.set_local_setting(
            pair[0].as_str().ok_or("a key")?,
            pair[1].as_str().ok_or("a value")?,
        )?;
    }
    let photo = text("photo")?;
    ann.put_blob(
        "photo",
        photo,
        &std::fs::read(dir.join("before/photos").join(photo))?,
    )?;
    let carol = text("carol")?;

    // Carol manages the School: blocking her leaves it, and only so.
    assert_eq!(
        json!(ann.areas_to_leave_for_blocking(carol)?),
        expected["leave"]
    );
    assert!(matches!(
        ann.block_person(carol, false),
        Err(Error::MustLeaveAreas(_))
    ));
    let mark = ann.current_seq()?;
    ann.block_person(carol, true)?;
    assert_eq!(
        lines_after(&ann, mark)?,
        lines_of(&dir.join("block.jsonl"))?
    );
    assert_eq!(block_state(&ann, &[photo])?, expected["blocked"]);

    // What Carol writes later: her check-off is kept, nothing else.
    *clock.lock() = at("incomingAt")?;
    ann.apply_entries(
        &entries_in(&dir.join("incoming.jsonl"))?,
        &[],
        None,
        &mut ImportReport::default(),
    )?;
    assert_eq!(block_state(&ann, &[photo])?, expected["incoming"]);

    // Unblocked, what was refused is wanted again, and taken in again.
    *clock.lock() = at("unblockAt")?;
    let mark = ann.current_seq()?;
    ann.unblock_person(carol)?;
    assert_eq!(
        lines_after(&ann, mark)?,
        lines_of(&dir.join("unblock.jsonl"))?
    );
    assert_eq!(block_state(&ann, &[photo])?, expected["unblocked"]);
    // Carol's Device syncs: what they still share comes back, and
    // nothing of hers is waited for any more.
    let refetch = read_json(&dir.join("refetch.json"))?;
    oldbrainfix_core::lan::answer_sync(&ann, &refetch, "Desk")?.ok_or("an answer")?;
    assert_eq!(block_state(&ann, &[photo])?, expected["refetched"]);
    Ok(())
}

// ----------------------------------------------------------- Sent Items

/// Random bytes as the generator's `_counted` hands them out.
fn counted_after(start: u64) -> impl FnMut(usize) -> oldbrainfix_core::Result<Vec<u8>> {
    let mut n: u64 = start;
    move |length| {
        n += 1;
        Ok((0..length as u64)
            .map(|i| ((n * 37 + i) & 0xff) as u8)
            .collect())
    }
}

/// Counted ids from `after` on.
fn counted_ids(store: &mut Store, after: u64) {
    let ids = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(after));
    store.set_id_maker(Box::new(move || {
        let n = ids.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(format!("{n:032x}"))
    }));
}

#[test]
fn a_sent_item_goes_as_the_dart_core_sends_and_takes_it() -> TestResult {
    use oldbrainfix_core::sent::{self, OpenError};
    let dir = fixtures().join("sent");
    let expected = read_json(&dir.join("expected.json"))?;
    let clock = held_clock("2026-01-03T12:00:00.000Z")?;
    let at = |time: &str| -> TestResult {
        *clock.lock() = parse_time(time)?;
        Ok(())
    };
    let held = clock.clone();
    let mut ada = Store::in_memory(Box::new(move || *held.lock()), || Ok(writer_key()))?;
    ada.set_local_setting("author", "Ada")?;
    for line in lines_of(&dir.join("sender.jsonl"))? {
        ada.insert(&serde_json::from_str(&line)?)?;
    }
    // Her Profile, which every file she sends carries.
    let settings = read_json(&dir.join("settings.json"))?;
    for pair in settings.as_array().ok_or("settings")? {
        ada.set_local_setting(
            pair[0].as_str().ok_or("a key")?,
            pair[1].as_str().ok_or("a value")?,
        )?;
    }
    counted_ids(&mut ada, 0x300);
    let note = ada
        .entities_of_type("note")?
        .into_iter()
        .next()
        .ok_or("note")?;
    let sent_note = note.clone();

    // Sending writes the same entry and the same files, byte for byte.
    let mark = ada.current_seq()?;
    let item = ada.sent_item_of(&note, false)?;
    assert_eq!(lines_after(&ada, mark)?, lines_of(&dir.join("sent.jsonl"))?);
    let signed = std::fs::read(dir.join("signed.oldbrain"))?;
    let sealed = std::fs::read(dir.join("sealed.oldbrain"))?;
    let key = writer_key();
    assert!(sent::encode(&item, &key, &[], &mut counted_after(0))? == signed);
    let mut to = [0u8; 32];
    to.copy_from_slice(&BASE64.decode(expected["box"].as_str().ok_or("box")?)?);
    assert_eq!(
        joiner_key_record_box()?,
        to,
        "Ben's X25519 key, as the Dart core derives it"
    );
    assert!(sent::encode(&item, &key, &[to], &mut counted_after(4000))? == sealed);

    // Ben reads both the same, and accepting writes the same entries.
    at("2026-01-04T18:00:00.000Z")?;
    let held = clock.clone();
    let mut ben = Store::in_memory(Box::new(move || *held.lock()), || Ok(joiner_key()))?;
    ben.set_local_setting("author", "Ben")?;
    let plain = ben.open_sent_item(&signed)?.map_err(|_| "signed")?;
    let opened = ben.open_sent_item(&sealed)?.map_err(|_| "sealed")?;
    assert!(!plain.encrypted);
    assert!(opened.encrypted);
    assert_eq!(opened.item, plain.item);
    assert_eq!(
        plain.trust.name(),
        expected["trust"].as_str().ok_or("trust")?
    );
    let got = &plain.item;
    let want = &expected["item"];
    assert_eq!(got.id, want["id"].as_str().ok_or("id")?);
    assert_eq!(got.version, want["version"].as_i64().ok_or("version")?);
    assert_eq!(got.from.device, want["device"].as_str().ok_or("device")?);
    let sender = &expected["sender"];
    assert_eq!(plain.sender.id, sender["id"].as_str().ok_or("id")?);
    assert_eq!(plain.sender.name, sender["name"].as_str().ok_or("name")?);
    assert_eq!(plain.code, sender["code"].as_str().ok_or("code")?);
    assert_eq!(got.text, want["text"].as_str().ok_or("text")?);
    let sections: Vec<Value> = got
        .sections
        .iter()
        .map(|s| json!({"text": s.text, "captured": dart_iso(s.captured)}))
        .collect();
    assert_eq!(json!(sections), want["sections"]);
    let routines: Vec<Value> = got
        .routines
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "start": r.schedule.start.to_string(),
                "times": r.schedule.times.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "rule": r.schedule.rule.as_ref().map(Recurrence::format),
            })
        })
        .collect();
    assert_eq!(json!(routines), want["routines"]);
    assert_eq!(got.encode(), item.encode());

    counted_ids(&mut ben, 0x400);
    let mark = ben.current_seq()?;
    assert_eq!(plain.state.name(), expected["state"]);
    let accepted = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    assert_eq!(accepted, expected["accepted"].as_str().ok_or("accepted")?);
    assert_eq!(
        lines_after(&ben, mark)?,
        lines_of(&dir.join("accepted.jsonl"))?
    );
    // Her Profile is kept as the Dart core keeps it, and the Note shows her.
    assert_eq!(person_settings(&ben)?, expected["people"]);
    assert_eq!(
        ben.author_of(&accepted)?.map(|p| p.id).as_deref(),
        expected["author"].as_str()
    );
    // The person filter counts it as hers, not Ben's own (#53).
    let author = expected["author"].as_str().ok_or("author")?;
    assert!(ben.wrote_by(&accepted, &PersonFilter::person(author))?);
    let me = PersonFilter {
        me: true,
        ..PersonFilter::default()
    };
    assert!(!ben.wrote_by(&accepted, &me)?);
    let note = Notes::new(&ben).by_id(&accepted)?.ok_or("the Note")?;
    assert_eq!(note_json(&note), expected["note"]);
    let mut routines = Routines::new(&ben).of_note(&accepted)?;
    routines.sort_by(|a, b| a.id.cmp(&b.id));
    let routines: Vec<Value> = routines.iter().map(routine_json).collect();
    assert_eq!(json!(routines), expected["routines"]);
    let again = ben.open_sent_item(&signed)?.map_err(|_| "again")?;
    assert_eq!(
        again.accepted.as_deref(),
        expected["acceptedAgain"].as_str()
    );
    assert_eq!(again.state.name(), expected["stateAgain"]);

    // Ben makes it his own; Ada changes it and sends the update, the
    // same entry and file byte for byte.
    insert_lines(&ben, &dir.join("receiver.jsonl"))?;
    insert_lines(&ada, &dir.join("changed.jsonl"))?;
    oldbrainfix_core::OwnProfile::new(&ada).rename("Ada Lovelace")?;
    at("2026-01-06T11:00:00.000Z")?;
    let mark = ada.current_seq()?;
    let item = ada.sent_item_of(&sent_note, false)?;
    assert_eq!(
        lines_after(&ada, mark)?,
        lines_of(&dir.join("update.jsonl"))?
    );
    let update = std::fs::read(dir.join("update.oldbrain"))?;
    assert!(sent::encode(&item, &key, &[], &mut counted_after(0))? == update);

    // Ben sees what changes and applies it as the Dart core does.
    let want = &expected["update"];
    counted_ids(&mut ben, 0x700);
    at("2026-01-07T08:00:00.000Z")?;
    let opened = ben.open_sent_item(&update)?.map_err(|_| "update")?;
    assert_eq!(
        opened.item.version,
        want["version"].as_i64().ok_or("version")?
    );
    assert_eq!(opened.state.name(), want["state"]);
    assert_eq!(opened.trust.name(), want["trust"]);
    assert_eq!(opened.sender.name, want["sender"].as_str().ok_or("sender")?);
    let changes = opened.changes.as_ref().ok_or("changes")?;
    assert_eq!(changes_json(changes), want["changes"]);
    let mark = ben.current_seq()?;
    ben.accept_sent_item(&opened.item, true)?;
    assert_eq!(
        lines_after(&ben, mark)?,
        lines_of(&dir.join("updated.jsonl"))?
    );
    assert_eq!(person_settings(&ben)?, want["people"]);
    assert_held(&ben, &accepted, want)?;
    let stale = ben.open_sent_item(&signed)?.map_err(|_| "stale")?;
    assert_eq!(stale.state.name(), want["stale"]);

    // Ada cancels it; Ben accepts that.
    let want = &expected["cancel"];
    at("2026-01-08T07:00:00.000Z")?;
    let mark = ada.current_seq()?;
    let item = ada.sent_item_of(&sent_note, true)?;
    assert_eq!(
        lines_after(&ada, mark)?,
        lines_of(&dir.join("cancel.jsonl"))?
    );
    let cancel = std::fs::read(dir.join("cancel.oldbrain"))?;
    assert!(sent::encode(&item, &key, &[], &mut counted_after(0))? == cancel);
    counted_ids(&mut ben, 0x800);
    at("2026-01-08T12:00:00.000Z")?;
    let opened = ben.open_sent_item(&cancel)?.map_err(|_| "cancel")?;
    assert!(opened.item.cancel);
    assert_eq!(
        opened.item.version,
        want["version"].as_i64().ok_or("version")?
    );
    assert_eq!(opened.state.name(), want["state"]);
    assert_eq!(opened.trust.name(), want["trust"]);
    let mark = ben.current_seq()?;
    ben.accept_sent_item(&opened.item, false)?;
    assert_eq!(
        lines_after(&ben, mark)?,
        lines_of(&dir.join("cancelled.jsonl"))?
    );
    assert_held(&ben, &accepted, want)?;

    // Somebody else holds no key to it, nor anything to cancel; and
    // a cancellation signed by another key is refused.
    let held = clock.clone();
    let eve = Store::in_memory(Box::new(move || *held.lock()), || {
        Ok(SigningKey::from_seed([9; 32]))
    })?;
    assert_eq!(
        eve.open_sent_item(&sealed)?.err(),
        Some(OpenError::NotForUs)
    );
    let elsewhere = eve.open_sent_item(&cancel)?.map_err(|_| "elsewhere")?;
    assert_eq!(elsewhere.state.name(), want["elsewhere"]);
    let forged = std::fs::read(dir.join("forged.oldbrain"))?;
    assert_eq!(
        ben.open_sent_item(&forged)?.err(),
        Some(OpenError::NotFromSender)
    );
    let impostor = std::fs::read(dir.join("impostor.oldbrain"))?;
    assert_eq!(
        ben.open_sent_item(&impostor)?.err(),
        Some(OpenError::NotASentItem)
    );
    Ok(())
}

/// The `person:` settings of `store`, as the generator lists them.
fn person_settings(store: &Store) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(json!(
        store
            .local_settings_with_prefix("person:")?
            .into_iter()
            .map(|(key, value)| json!([key, value]))
            .collect::<Vec<_>>()
    ))
}

/// What an update changes, as the Dart core writes it.
fn changes_json(c: &oldbrainfix_core::sent::SentItemChanges) -> Value {
    let schedule = |s: &Schedule| {
        json!({
            "start": s.start.to_string(),
            "times": s.times.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "rule": s.rule.as_ref().map(Recurrence::format),
        })
    };
    json!({
        "textBefore": c.text_before,
        "textEdited": c.text_edited,
        "sectionsAdded": c.sections_added.iter().map(|s| s.text.clone()).collect::<Vec<_>>(),
        "rescheduled": c.rescheduled.iter().map(|r| json!({
            "routine": r.routine,
            "before": schedule(&r.before),
            "after": schedule(&r.after),
        })).collect::<Vec<_>>(),
        "routinesAdded": c.routines_added.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        "routinesRemoved": c.routines_removed.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
        "restores": c.restores,
    })
}

/// That `store` holds the accepted Note `id` as `want` says: the Note,
/// its Sections and every Routine and Reminder of it, trashed too.
fn assert_held(store: &Store, id: &str, want: &Value) -> TestResult {
    let note = Notes::new(store).by_id(id)?.ok_or("the Note")?;
    assert_eq!(note_json(&note), want["note"]);
    let sections: Vec<Value> = Notes::new(store)
        .sections(id)?
        .iter()
        .map(section_json)
        .collect();
    assert_eq!(json!(sections), want["sections"]);
    let mut ids = store.entities_of_type("routine")?;
    ids.sort();
    let mut routines = Vec::new();
    for r in ids {
        if let Some(routine) = Routines::new(store).by_id(&r)?
            && routine.note_id == id
        {
            routines.push(routine_json(&routine));
        }
    }
    assert_eq!(json!(routines), want["routines"]);
    Ok(())
}

/// Ben's X25519 key, from his key record.
fn joiner_key_record_box() -> Result<[u8; 32], Box<dyn std::error::Error>> {
    Ok(oldbrainfix_core::KeyRecord::make(&joiner_key())
        .verified_box_key()
        .ok_or("a box key")?)
}

// ------------------------------------------------------------- Backups

/// The restored Device's key in the backup fixture: seed bytes 65 to 96.
fn restored_key() -> SigningKey {
    SigningKey::from_seed(std::array::from_fn(|i| i as u8 + 65))
}

fn wire_lines(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    for e in store.all_entries()? {
        lines.push(serde_json::to_string(&e.wire())?);
    }
    Ok(lines)
}

#[test]
fn the_backup_the_dart_core_wrote_is_restored_here_and_written_the_same() -> TestResult {
    use oldbrainfix_core::backup::{read_header, restore, unlock, write_backup};
    let dir = fixtures().join("backup");
    let expected = read_json(&dir.join("expected.json"))?;
    let file = dir.join("backup.obfbackup");
    let header = read_header(&file)?.ok_or("a header")?;
    assert_eq!(
        Some(dart_iso(header.created).as_str()),
        expected["created"].as_str()
    );

    // Opened with the passphrase and with the recovery key alike.
    let passphrase = expected["passphrase"].as_str().ok_or("passphrase")?;
    let code = expected["recoveryCode"].as_str().ok_or("code")?;
    let key = unlock(&header, Some(passphrase), None)?.ok_or("the passphrase opens it")?;
    assert_eq!(unlock(&header, None, Some(code))?, Some(key));
    assert_eq!(
        Some(hex::encode(key).as_str()),
        expected["backupKey"].as_str()
    );

    // A new Device restores it as the Dart core did, entry for entry.
    let at = parse_time(expected["restoredAt"].as_str().ok_or("restoredAt")?)?;
    let restored = Store::in_memory(Box::new(move || at), || Ok(restored_key()))?;
    let got = restore(&restored, &file, &key)?;
    assert_eq!(got.roles as u64, expected["roles"].as_u64().ok_or("roles")?);
    assert_eq!(
        wire_lines(&restored)?,
        lines_of(&dir.join("restored.jsonl"))?
    );
    let moments: Vec<Value> = restored
        .moment_rows()?
        .iter()
        .map(|m| json!({"seq": m.seq, "cause": m.cause, "label": m.label, "at": m.at}))
        .collect();
    assert_eq!(Value::Array(moments), expected["moments"]);
    let settings: Vec<Value> = restored
        .local_settings_with_prefix("")?
        .into_iter()
        .filter(|(k, _)| k != "device" && k != "signseed")
        .map(|(k, v)| json!([k, v]))
        .collect();
    assert_eq!(Value::Array(settings), expected["settings"]);
    let mut actual = read(&restored)?;
    let mut read_back = expected["read"].clone();
    for value in [&mut actual, &mut read_back] {
        if let Some(map) = value.as_object_mut() {
            map.remove("device");
        }
    }
    assert_eq!(actual, read_back);

    // The same Device writes the very same file here, byte for byte.
    let created = dart_iso(header.created);
    let source = fixture_device("backup", "device", writer_key, &created)?;
    for m in read_json(&dir.join("moments.json"))?
        .as_array()
        .ok_or("moments")?
    {
        source.add_moment_at(
            m["seq"].as_i64().ok_or("seq")?,
            m["cause"].as_str().ok_or("cause")?,
            m["label"].as_str(),
            m["at"].as_str().ok_or("at")?,
        )?;
    }
    let ours = tempfile::tempdir()?;
    let path = ours.path().join("backup.obfbackup");
    let counts = write_backup(&source, &path, &mut counted_after(5000))?;
    assert_eq!(
        counts.entries as u64,
        expected["counts"]["entries"].as_u64().ok_or("entries")?
    );
    assert_eq!(
        counts.blobs as u64,
        expected["counts"]["blobs"].as_u64().ok_or("blobs")?
    );
    assert!(
        std::fs::read(&path)? == std::fs::read(&file)?,
        "the Backup differs"
    );
    Ok(())
}

#[test]
fn profiles_come_out_as_the_dart_core_has_them() -> TestResult {
    use oldbrainfix_core::profile::{self, OwnProfile, Profile};
    let dir = fixtures().join("profiles");
    let fixture = read_json(&dir.join("profiles.json"))?;
    for case in fixture["names"].as_array().ok_or("names")? {
        let raw = case["raw"].as_str().ok_or("raw")?;
        assert_eq!(profile::sanitize_name(raw), case["kept"], "{raw:?}");
    }
    for case in fixture["initials"].as_array().ok_or("initials")? {
        let name = case["name"].as_str().ok_or("name")?;
        assert_eq!(profile::initials(name), case["initials"], "{name}");
    }
    for case in fixture["colours"].as_array().ok_or("colours")? {
        let id = case["id"].as_str().ok_or("id")?;
        assert_eq!(json!(profile::color(id)), case["colour"], "{id}");
    }
    for case in fixture["codes"].as_array().ok_or("codes")? {
        let key = BASE64.decode(case["key"].as_str().ok_or("key")?)?;
        let id = case["id"].as_str().ok_or("id")?;
        assert_eq!(profile::short_code(id, &key), case["code"], "{id}");
    }
    for case in fixture["records"].as_array().ok_or("records")? {
        let payload = case["payload"].as_str().ok_or("payload")?;
        let parsed = Profile::parse(payload).ok_or("a record")?;
        assert_eq!(parsed.encode(), payload);
    }
    for bad in fixture["refused"].as_array().ok_or("refused")? {
        assert_eq!(
            Profile::parse(bad.as_str().ok_or("refused")?),
            None,
            "{bad}"
        );
    }

    // The picture the Dart core made is taken in here as one received.
    let picture = std::fs::read(dir.join("picture.jpg"))?;
    assert_eq!(profile::picture_size(&picture), Some((256, 256)));
    assert!(profile::received_picture(&picture).is_some());

    // The own Profile goes through the same steps to the same settings.
    let store = writer_store()?;
    let own = OwnProfile::new(&store);
    let steps = fixture["own"].as_array().ok_or("own")?;
    for (i, step) in steps.iter().enumerate() {
        match i {
            0 => own.apply_defaults(" Ada\u{2019}s desk ", None)?,
            1 => own.apply_defaults("desk", Some(picture.clone()))?,
            2 => own.rename("\u{202E}Ada\u{200B}")?,
            3 => own.apply_defaults("other", None)?,
            4 => own.set_picture(None)?,
            _ => return Err(format!("a step more than here: {step}").into()),
        };
        let settings: serde_json::Map<String, Value> = store
            .local_settings_with_prefix("profile")?
            .into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect();
        assert_eq!(
            Value::Object(settings),
            step["settings"],
            "{}",
            step["step"]
        );
        assert_eq!(json!(own.code()?), step["code"], "{}", step["step"]);
    }
    assert_eq!(steps.len(), 5);

    // Profiles another of the own Devices sends, each taken as there.
    let takes = &fixture["takes"];
    let from = BASE64.decode(takes["from"].as_str().ok_or("from")?)?;
    for case in takes["cases"].as_array().ok_or("cases")? {
        let store = writer_store()?;
        for (key, value) in takes["before"].as_object().ok_or("before")? {
            store.set_local_setting(key, value.as_str().ok_or("a value")?)?;
        }
        let taken = OwnProfile::new(&store).take(Some(&case["profile"]), &from)?;
        assert_eq!(taken.name(), case["taken"], "{}", case["case"]);
        assert_eq!(
            profile_settings(&store)?,
            case["settings"],
            "{}",
            case["case"]
        );
    }

    // Profiles of another person, each taken by a Device that keeps
    // Ben's: an update signed by a key not bound to it is refused.
    let people = &fixture["people"];
    let at = Utc
        .with_ymd_and_hms(2026, 3, 1, 0, 0, 0)
        .single()
        .ok_or("a time")?;
    for case in people["cases"].as_array().ok_or("cases")? {
        let store = Store::in_memory(Box::new(move || at), || Ok(writer_key()))?;
        store.set_local_setting(
            "profile",
            &Profile {
                id: "own".into(),
                name: "Ada".into(),
                picture: None,
                version: 1,
            }
            .encode(),
        )?;
        store.set_local_setting("profile:first", &BASE64.encode(writer_key().public_key()))?;
        for held in people["held"].as_array().ok_or("held")? {
            store.take_person(held, None)?;
        }
        let taken = store.take_person(&case["profile"], None)?;
        assert_eq!(taken.name(), case["taken"], "{}", case["case"]);
        assert_eq!(people_json(&store)?, case["people"], "{}", case["case"]);
        for (id, resolved) in case["resolved"].as_object().ok_or("resolved")? {
            assert_eq!(
                json!(store.resolve_person(id)?),
                *resolved,
                "{}",
                case["case"]
            );
        }
    }
    Ok(())
}

#[test]
fn entries_of_a_later_version_are_kept_and_passed_on_byte_for_byte() -> TestResult {
    use oldbrainfix_core::signing::{ImportReport, KeyRecord};
    let dir = fixtures().join("versions");
    let expected = read_json(&dir.join("expected.json"))?;
    let record = KeyRecord::from_json(&read_json(&dir.join("key.json"))?).ok_or("a key record")?;
    let parse = |name: &str| -> Result<Vec<WireEntry>, Box<dyn std::error::Error>> {
        lines_of(&dir.join(name))?
            .iter()
            .map(|l| WireEntry::parse(l).ok_or_else(|| format!("an entry: {l}").into()))
            .collect()
    };
    let now = parse_time("2026-01-02T10:00:00.000Z")?;
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8 + 33;
    }
    let reader = Store::in_memory(Box::new(move || now), move || {
        Ok(SigningKey::from_seed(seed))
    })?;

    // A key changed, the version raised or a key added on the way: refused.
    let mut report = ImportReport::default();
    let refused = reader.apply_entries(
        &parse("tampered.jsonl")?,
        std::slice::from_ref(&record),
        None,
        &mut report,
    )?;
    assert!(refused.is_empty());
    assert_eq!(json!(report.refused_count()), expected["refused"]);

    let incoming = parse("incoming.jsonl")?;
    let applied = reader.apply_entries(&incoming, &[record], None, &mut ImportReport::default())?;
    let ids: Vec<String> = applied
        .iter()
        .map(|e| format!("{}:{}", e.device, e.dseq))
        .collect();
    assert_eq!(json!(ids), expected["applied"]);
    assert_eq!(json!(reader.version_vector()?), expected["vector"]);
    let note = incoming
        .last()
        .map(|e| e.entity.clone())
        .ok_or("an entry")?;
    assert_eq!(
        Notes::new(&reader).by_id(&note)?.map(|n| json!(n.text)),
        Some(expected["text"].clone())
    );

    // Passed on as they came, the one of version 2 among them.
    let forwarded: Vec<String> = reader
        .entries_since(&std::collections::HashMap::new())?
        .iter()
        .map(|e| e.wire().line())
        .collect::<Result<_, _>>()?;
    assert_eq!(forwarded, lines_of(&dir.join("forwarded.jsonl"))?);
    assert_eq!(forwarded, lines_of(&dir.join("incoming.jsonl"))?);
    // Over the LAN an entry travels as a JSON object and arrives in the
    // same canonical form.
    for line in &forwarded {
        let value: Value = serde_json::from_str(line)?;
        let entry = WireEntry::from_value(&value).ok_or("an entry")?;
        assert_eq!(&entry.line()?, line);
    }
    Ok(())
}

/// The name a place with `slug` and `name` shows under, as the
/// generator's `_shownName`: the translation in `starters` while it keeps
/// its Starter Set name, else the name as stored.
fn shown_name(starters: &Value, slug: Option<&str>, name: &str) -> String {
    match slug {
        Some(slug) if oldbrainfix_core::places::starter_name(slug) == Some(name) => {
            starters[slug].as_str().unwrap_or(name).to_string()
        }
        _ => name.to_string(),
    }
}

#[test]
fn transcript_prompts_come_out_as_the_dart_core_writes_them() -> TestResult {
    use oldbrainfix_core::transcript_prompts::{
        GERMAN_LEAD_IN, MAX_LENGTH, MAX_NAMES, TranscriptPrompts,
    };
    let fixture = read_json(&fixtures().join("transcript_prompts.json"))?;
    assert_eq!(fixture["germanLeadIn"], GERMAN_LEAD_IN);
    assert_eq!(fixture["maxLength"], MAX_LENGTH);
    assert_eq!(fixture["maxNames"], MAX_NAMES);
    let texts = |list: &Value| -> Vec<String> {
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    for case in fixture["lists"].as_array().ok_or("lists")? {
        let prompts = TranscriptPrompts::of(texts(&case["names"]), texts(&case["terms"]));
        assert_eq!(prompts.english, case["english"], "{case}");
        assert_eq!(prompts.german, case["german"], "{case}");
    }
    let mut seen = 0;
    for view in fixture["views"].as_array().ok_or("views")? {
        let name = view["store"].as_str().ok_or("store")?;
        let ui = &fixture["ui"][view["locale"].as_str().ok_or("locale")?];
        let store = Store::in_memory(Box::new(Utc::now), || Ok(writer_key()))?;
        for entry in fixture["stores"][name].as_array().ok_or("entries")? {
            store.insert(&WireEntry::from_value(entry).ok_or("an entry")?)?;
        }
        let starters = &ui["starters"];
        let prompts = TranscriptPrompts::for_places(
            &Places::new(&store),
            texts(&ui["terms"]),
            &|a| shown_name(starters, a.starter.as_deref(), &a.name),
            &|t| shown_name(starters, t.starter.as_deref(), &t.name),
        )?;
        assert_eq!(prompts.english, view["english"], "{name}");
        assert_eq!(prompts.german, view["german"], "{name}");
        seen += 1;
    }
    assert_eq!(seen, 6);
    Ok(())
}

#[test]
fn transcripts_are_guarded_as_the_dart_core_guards_them() -> TestResult {
    use oldbrainfix_core::transcript_prompts::TranscriptPrompts;
    let fixture = read_json(&fixtures().join("transcript_prompts.json"))?;
    let texts = |list: &Value| -> Vec<String> {
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    let guards = fixture["guards"].as_array().ok_or("guards")?;
    assert!(guards.len() >= 4);
    for case in guards {
        let prompts = TranscriptPrompts::of(texts(&case["names"]), texts(&case["terms"]));
        let transcript = case["transcript"].as_str().ok_or("transcript")?;
        assert_eq!(prompts.guard(transcript), case["guarded"], "{case}");
    }
    Ok(())
}
