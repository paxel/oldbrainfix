//! The Dart core writes the fixture corpus under `desktop/fixtures`
//! (`packages/oldbrainfix_core/tool/gen_fixtures.dart`); this core must
//! read every log the same, sign every entry the same, and write every
//! scenario again to the same log, byte for byte.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Duration, TimeZone, Utc};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::places::keys as place_keys;
use oldbrainfix_core::question_marks::{self, SHAPES};
use oldbrainfix_core::signing::{entry_bytes, verify_signature};
use oldbrainfix_core::{
    Importance, Note, Notes, NotesGo, Places, STARTER_SET, Section, SigningKey, Store, WireEntry,
    dart_iso, parse_time,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SCENARIOS: [&str; 5] = ["edits", "notes", "places", "sections", "trash"];

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
    })
}

fn section_json(section: &Section) -> Value {
    json!({
        "id": section.id,
        "noteId": section.note_id,
        "text": section.text,
        "captured": dart_iso(section.captured),
    })
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
        for topic in &topics {
            inside.insert(topic.id.clone(), ids(&topic.id)?);
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
    }))
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
    assert_eq!(entries.len(), 2);
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

#[test]
fn every_scenario_written_here_is_the_dart_cores_log_byte_for_byte() -> TestResult {
    let writers: [(&str, Writer); 5] = [
        ("edits", write_edits),
        ("notes", write_notes),
        ("places", write_places),
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
