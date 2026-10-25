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
    Occurrence, Places, Recurrence, Routine, Routines, STARTER_SET, Schedule, Search, Section,
    SigningKey, SimpleRepeat, Store, WireEntry, dart_iso, parse_time,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SCENARIOS: [&str; 7] = [
    "edits", "media", "notes", "places", "routines", "sections", "trash",
];

/// The queries every scenario's search is checked with, the generator's.
const QUERIES: [&str; 10] = [
    "", "sauce", "MILK", "receipt", "dentist", "x-ray", "müller", "called", "loop", " e ",
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
    })
}

fn section_json(section: &Section) -> Value {
    json!({
        "id": section.id,
        "noteId": section.note_id,
        "text": section.text,
        "captured": dart_iso(section.captured),
        "photos": section.photos,
        "recordings": section.recordings,
        "transcripts": section.transcripts,
        "photoTexts": section.photo_texts,
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
    for id in &note_ids {
        if let Some(note) = notes.by_id(id)? {
            every.push(note_json(&note));
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
    notes.add_photo(&shed.id, &photo(4))?;
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
    let writers: [(&str, Writer); 7] = [
        ("edits", write_edits),
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
            "kind": format!("{:?}", s.kind).to_lowercase(),
            "every": s.every,
            "unit": format!("{:?}", s.unit).to_lowercase(),
            "weekdays": s.weekdays.iter().collect::<Vec<_>>(),
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

    // Both end up reading the same.
    let back = apply_answer(&joiner, &answer)?;
    assert_eq!(back.name, "Desk");
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
