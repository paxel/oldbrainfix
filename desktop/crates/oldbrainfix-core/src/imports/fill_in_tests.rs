//! Importing the same source again adds only what is missing (obf-1078,
//! ADR 0036), as the Dart core's `import_fill_in_test.dart`.

use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
};
use crate::notes::{Note, Notes};
use crate::places::NotesGo;
use crate::shared::{AreaRole, Invite};
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use crate::{OwnProfile, PairedDevice};
use chrono::{Duration, TimeZone};
use std::sync::atomic::Ordering;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn march() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 3, 1, 8, 0, 0)
        .single()
        .unwrap_or_default()
}

fn store() -> Made<Store> {
    Ok(Store::in_memory(ticking(), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?)
}

fn file(name: &str, text: &str) -> ImportFile {
    ImportFile {
        name: name.into(),
        modified: Some(march()),
        bytes: text.as_bytes().to_vec(),
    }
}

fn folder() -> Vec<ImportFile> {
    vec![
        file("Shopping.md", "# Shopping\nmilk"),
        file("sub\\Dentist.txt", "Dentist\nTuesday"),
        file("Ideas.txt", "Ideas\nmore"),
    ]
}

fn area_of(summary: &ImportSummary) -> Made<String> {
    Ok(summary.area_id.clone().ok_or("an Area")?)
}

fn source(store: &Store, id: &str) -> Result<Option<String>> {
    store.current(id, keys::SOURCE)
}

/// The paths the Notes inside `area` came from.
fn paths_inside(store: &Store, area: &str) -> Made<HashSet<String>> {
    let mut paths = HashSet::new();
    for note in Notes::new(store).inside(area)? {
        paths.insert(source(store, &note.id)?.ok_or("a path")?);
    }
    Ok(paths)
}

fn with_path(store: &Store, area: &str, path: &str) -> Made<Note> {
    for note in Notes::new(store).inside(area)? {
        if source(store, &note.id)?.as_deref() == Some(path) {
            return Ok(note);
        }
    }
    Err(format!("no Note of {path}").into())
}

fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

#[test]
fn each_note_keeps_its_path_and_the_import_the_area_what_was_picked() -> TestResult {
    let store = store()?;
    let s = Imports::new(&store)
        .from_source("/home/a/Notes")
        .folder("From Notes", &folder())?;
    let area = area_of(&s)?;
    assert_eq!(
        paths_inside(&store, &area)?,
        set(&["Shopping.md", "sub/Dentist.txt", "Ideas.txt"])
    );
    for note in Notes::new(&store).inside(&area)? {
        assert_eq!(
            store.current(&note.id, keys::IMPORT_AREA)?,
            Some(area.clone())
        );
    }
    let id = source_id(ImportSourceKind::Folder, "/home/a/Notes");
    assert_eq!(store.current(&area, keys::IMPORT_SOURCE)?, Some(id.clone()));
    // No path lies in the log as written.
    assert_eq!(id.len(), 64);
    assert!(!id.contains("Notes"));
    assert_ne!(id, source_id(ImportSourceKind::Keep, "/home/a/Notes"));
    Ok(())
}

#[test]
fn without_a_source_picked_the_area_keeps_none() -> TestResult {
    let store = store()?;
    let s = Imports::new(&store).folder("From Notes", &folder())?;
    assert_eq!(store.current(&area_of(&s)?, keys::IMPORT_SOURCE)?, None);
    let found = Imports::new(&store).earlier(ImportSourceKind::Folder, "", ["Shopping.md"])?;
    assert_eq!(found, None);
    Ok(())
}

#[test]
fn the_earlier_import_is_found_by_what_was_picked_and_its_paths_never_by_a_name() -> TestResult {
    let store = store()?;
    let imports = Imports::new(&store);
    let s = imports.from_source("/n").folder("From Notes", &folder())?;
    let area = area_of(&s)?;
    let at = store
        .logged_history(&area)?
        .last()
        .ok_or("an entry")?
        .date_at;
    let imports = Imports::new(&store);
    let found = imports
        .earlier(
            ImportSourceKind::Folder,
            "/n",
            ["Shopping.md", "sub\\Dentist.txt", "New.md"],
        )?
        .ok_or("an earlier Import")?;
    assert_eq!(found.area_id, area);
    assert_eq!(found.area_name, "From Notes");
    assert_eq!(found.at, at);
    assert_eq!(found.held, 2);
    let all: Vec<String> = folder().into_iter().map(|f| f.name).collect();
    // Another folder of the same names, the same folder read as a Keep
    // export, and paths none of its Notes came from: none of them.
    assert_eq!(imports.earlier(ImportSourceKind::Folder, "/m", &all)?, None);
    assert_eq!(imports.earlier(ImportSourceKind::Keep, "/n", &all)?, None);
    assert_eq!(
        imports.earlier(ImportSourceKind::Folder, "/n", ["New.md"])?,
        None
    );
    Ok(())
}

#[test]
fn the_earlier_import_is_the_newer_one_once_the_first_was_not_filled_in() -> TestResult {
    let store = store()?;
    let first = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &folder())?;
    let second = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &folder())?;
    assert_ne!(second.area_id, first.area_id);
    assert_eq!(second.notes, 3);
    let name = Places::new(&store).area_by_id(&area_of(&second)?)?;
    assert_eq!(name.map(|a| a.name).as_deref(), Some("From Notes (2)"));
    let found = Imports::new(&store).earlier(ImportSourceKind::Folder, "/n", ["Ideas.txt"])?;
    assert_eq!(found.map(|f| f.area_id), second.area_id);
    Ok(())
}

#[test]
fn the_earlier_import_is_none_once_its_area_is_deleted_and_an_import_makes_a_new_one() -> TestResult
{
    let store = store()?;
    let first = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &folder())?;
    let area = area_of(&first)?;
    Places::new(&store).delete_area(&area, NotesGo::Unsorted)?;
    let found = Imports::new(&store).earlier(ImportSourceKind::Folder, "/n", ["Ideas.txt"])?;
    assert_eq!(found, None);
    let again = Imports::new(&store)
        .from_source("/n")
        .filling_in(&area)
        .folder("From Notes", &folder())?;
    assert_ne!(again.area_id.as_deref(), Some(area.as_str()));
    assert_eq!(again.notes, 3);
    assert_eq!(again.already, 0);
    Ok(())
}

#[test]
fn filling_in_makes_only_what_was_never_made_into_the_first_area() -> TestResult {
    let store = store()?;
    let first = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &folder())?;
    let area = area_of(&first)?;
    let notes = Notes::new(&store);
    let health = Places::new(&store).add_area("Health")?.id;
    notes.file(&with_path(&store, &area, "sub/Dentist.txt")?.id, &health)?;
    notes.trash(&with_path(&store, &area, "Ideas.txt")?.id)?;
    let filled = Imports::new(&store)
        .from_source("/n")
        .filling_in(&area)
        .folder(
            "From Notes",
            &[
                file("Shopping.md", "# Shopping\nmilk and bread"),
                file("sub/Dentist.txt", "Dentist\nTuesday"),
                file("Ideas.txt", "Ideas\nmore"),
                file("New.md", "New"),
                file("sub\\Later.txt", "Later"),
            ],
        )?;
    assert_eq!(filled.area_id.as_deref(), Some(area.as_str()));
    assert_eq!(filled.notes, 2);
    assert_eq!(filled.already, 3);
    assert_eq!(filled.skipped(), 0);
    let names: Vec<String> = Places::new(&store)
        .areas()?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(names, ["From Notes", "Health"]);
    assert_eq!(
        paths_inside(&store, &area)?,
        set(&["Shopping.md", "New.md", "sub/Later.txt"])
    );
    // Changed since, it stays as it was made.
    assert_eq!(
        with_path(&store, &area, "Shopping.md")?.text,
        "# Shopping\nmilk"
    );
    let new = with_path(&store, &area, "New.md")?;
    assert_eq!(
        store.current(&new.id, keys::IMPORT_AREA)?,
        Some(area.clone())
    );
    Ok(())
}

#[test]
fn a_google_keep_export_stopped_is_filled_in_to_all_its_notes() -> TestResult {
    let store = store()?;
    let note = |name: &str, title: &str| ImportFile {
        name: format!("Takeout/Keep/{name}.json"),
        modified: None,
        bytes: serde_json::json!({
            "title": title,
            "textContent": "text",
            "createdTimestampUsec": 1_700_000_000_000_000_i64,
        })
        .to_string()
        .into_bytes(),
    };
    let export = vec![note("a", "A"), note("b", "B"), note("c", "C")];
    let asked = std::sync::atomic::AtomicUsize::new(0);
    // Before the Area and the first Note, then stopped.
    let stop = || asked.fetch_add(1, Ordering::SeqCst) >= 2;
    let first = Imports::new(&store)
        .from_source("content://zip")
        .stopping(&stop)
        .keep("From Google Keep", &export)?;
    assert_eq!(first.notes, 1);
    assert_eq!(first.stopped, Some(3));
    let paths: Vec<&str> = export.iter().map(|e| e.name.as_str()).collect();
    let found = Imports::new(&store)
        .earlier(ImportSourceKind::Keep, "content://zip", &paths)?
        .ok_or("an earlier Import")?;
    assert_eq!(Some(&found.area_id), first.area_id.as_ref());
    assert_eq!(found.held, 1);
    let filled = Imports::new(&store)
        .from_source("content://zip")
        .filling_in(&found.area_id)
        .keep("From Google Keep", &export)?;
    assert_eq!(filled.area_id, first.area_id);
    assert_eq!(filled.notes, 2);
    assert_eq!(filled.already, 1);
    assert_eq!(Notes::new(&store).inside(&found.area_id)?.len(), 3);
    Ok(())
}

#[test]
fn an_area_of_another_source_gets_nothing_a_new_area_takes_all() -> TestResult {
    let store = store()?;
    let first = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &folder())?;
    let area = area_of(&first)?;
    for from in [Some("/m"), None] {
        let imports = Imports::new(&store).filling_in(&area);
        let imports = match from {
            Some(from) => imports.from_source(from),
            None => imports,
        };
        let other = imports.folder("From Notes", &folder())?;
        assert_ne!(other.area_id.as_deref(), Some(area.as_str()), "{from:?}");
        assert_eq!(other.notes, 3, "{from:?}");
        assert_eq!(other.already, 0, "{from:?}");
    }
    let keep = Imports::new(&store)
        .from_source("/n")
        .filling_in(&area)
        .keep("From Google Keep", &[])?;
    assert_eq!(keep.area_id, None);
    Ok(())
}

#[test]
fn filling_in_reads_as_often_for_a_few_notes_as_for_many() -> TestResult {
    let reads = |count: usize| -> Made<(usize, usize)> {
        let s = store()?;
        let made: Vec<ImportFile> = (0..count)
            .map(|i| file(&format!("n{i}.txt"), &format!("Note {i}")))
            .collect();
        let first = Imports::new(&s)
            .from_source("/n")
            .folder("From Notes", &made)?;
        let area = area_of(&first)?;
        s.logged_reads.store(0, Ordering::SeqCst);
        s.history_reads.store(0, Ordering::SeqCst);
        let mut paths: Vec<String> = made.iter().map(|f| f.name.clone()).collect();
        paths.push("new.txt".into());
        let found = Imports::new(&s).earlier(ImportSourceKind::Folder, "/n", &paths)?;
        assert_eq!(found.map(|f| f.held), Some(count));
        let mut all = made.clone();
        all.push(file("new.txt", "New"));
        let filled = Imports::new(&s)
            .from_source("/n")
            .filling_in(&area)
            .folder("From Notes", &all)?;
        assert_eq!(filled.notes, 1);
        assert_eq!(filled.already, count);
        Ok((
            s.logged_reads.load(Ordering::SeqCst),
            s.history_reads.load(Ordering::SeqCst),
        ))
    };
    assert_eq!(reads(60)?, reads(3)?);
    Ok(())
}

/// One clock for every Device of a trust test, a second further at each
/// reading, so what was written later wins everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn device(seed: u8, name: &str) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, Ordering::SeqCst);
        start + Duration::seconds(n)
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn remember(store: &Store, peer: &Store) -> TestResult {
    store.remember_device(&PairedDevice {
        id: peer.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: store.now(),
        shared: true,
    })?;
    Ok(())
}

/// Ann's Import from `/n` of `a.md`, its Area shared with Ben in `role`:
/// both Devices and the Area.
fn shared_import(role: AreaRole) -> Made<(Store, Store, String)> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let made = Imports::new(&ann)
        .from_source("/n")
        .folder("From Notes", &[file("a.md", "a")])?;
    let area = ann.share(&area_of(&made)?)?;
    let invite = Invite {
        area: area.clone(),
        role,
    };
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    remember(&ann, &ben)?;
    remember(&ben, &ann)?;
    Ok((ann, ben, area))
}

/// One LAN session, Ben asking Ann: what Ben wrote reaches Ann.
fn exchange(ben: &Store, ann: &Store) -> TestResult {
    let request = sync_request(ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(ben, &answer, true, None)?;
    Ok(())
}

#[test]
fn a_path_another_persons_device_wrote_on_a_note_counts_for_nothing() -> TestResult {
    let (ann, ben, area) = shared_import(AreaRole::Member)?;
    // As a modified app would: Ben slips a Note of that Import, of a path
    // still to come, into the Area.
    let forged = Notes::new(&ben).capture("b", Some(&area), None)?.id;
    ben.append(&forged, keys::IMPORT_AREA, Some(&area), None)?;
    ben.append(&forged, keys::SOURCE, Some("b.md"), None)?;
    exchange(&ben, &ann)?;
    assert_eq!(ann.current(&forged, keys::SOURCE)?.as_deref(), Some("b.md"));
    let found = Imports::new(&ann).earlier(ImportSourceKind::Folder, "/n", ["a.md", "b.md"])?;
    assert_eq!(found.map(|f| f.held), Some(1));
    let filled = Imports::new(&ann)
        .from_source("/n")
        .filling_in(&area)
        .folder("From Notes", &[file("a.md", "a"), file("b.md", "b")])?;
    assert_eq!(filled.notes, 1);
    assert_eq!(filled.already, 1);
    Ok(())
}

#[test]
fn what_was_picked_as_another_persons_device_wrote_it_on_an_area_counts_for_nothing() -> TestResult
{
    let (ann, ben, area) = shared_import(AreaRole::Manager)?;
    // Ben, a Manager there, says the Area holds another source of Ann's.
    let other = source_id(ImportSourceKind::Folder, "/other");
    ben.append(&area, keys::IMPORT_SOURCE, Some(&other), None)?;
    exchange(&ben, &ann)?;
    assert_eq!(ann.current(&area, keys::IMPORT_SOURCE)?, Some(other));
    let imports = Imports::new(&ann);
    assert_eq!(
        imports.earlier(ImportSourceKind::Folder, "/other", ["a.md"])?,
        None
    );
    let found = imports.earlier(ImportSourceKind::Folder, "/n", ["a.md"])?;
    assert_eq!(found.map(|f| f.area_id), Some(area.clone()));
    let filled = Imports::new(&ann)
        .from_source("/other")
        .filling_in(&area)
        .folder("From Notes", &[file("a.md", "a")])?;
    assert_ne!(filled.area_id.as_deref(), Some(area.as_str()));
    assert_eq!(filled.notes, 1);
    Ok(())
}

#[test]
fn the_ai_port_reads_where_an_imported_note_came_from() -> TestResult {
    use crate::ai_tools::ToolContent;
    let store = store()?;
    let s = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &[file("sub\\a.md", "a")])?;
    let area = area_of(&s)?;
    let made = with_path(&store, &area, "sub/a.md")?.id;
    let own = Notes::new(&store).capture("Own", None, None)?.id;
    assert_eq!(
        Imports::new(&store).source_of(&made)?,
        Some(ImportedFrom {
            path: "sub/a.md".into(),
            area_id: area.clone(),
        })
    );
    let task = store.open_ai_task()?;
    let read = |id: &str| -> Made<Value> {
        let out = store.ai_tool(
            task.as_str(),
            "read_note",
            &serde_json::json!({ "id": id }),
            &|t| t.naive_utc(),
        )?;
        let [ToolContent::Text(text)] = &out.content[..] else {
            return Err(format!("no text: {out:?}").into());
        };
        Ok(serde_json::from_str(text)?)
    };
    assert_eq!(
        read(&made)?.get("importedFrom"),
        Some(&serde_json::json!({ "importArea": area, "path": "sub/a.md" }))
    );
    assert_eq!(read(&own)?.get("importedFrom"), None);
    Ok(())
}

#[test]
fn the_ai_port_does_not_name_an_area_hidden_from_it() -> TestResult {
    use crate::ai_tools::ToolContent;
    let store = store()?;
    let s = Imports::new(&store)
        .from_source("/n")
        .folder("From Notes", &[file("a.md", "a")])?;
    let area = area_of(&s)?;
    let made = with_path(&store, &area, "a.md")?.id;
    let elsewhere = Places::new(&store).add_area("Elsewhere")?.id;
    Notes::new(&store).file(&made, &elsewhere)?;
    store.set_hidden_from_ai(&area, true)?;
    let task = store.open_ai_task()?;
    let out = store.ai_tool(
        task.as_str(),
        "read_note",
        &serde_json::json!({ "id": made }),
        &|t| t.naive_utc(),
    )?;
    let [ToolContent::Text(text)] = &out.content[..] else {
        return Err(format!("no text: {out:?}").into());
    };
    let read: Value = serde_json::from_str(text)?;
    assert_eq!(
        read.get("importedFrom"),
        Some(&serde_json::json!({ "importArea": null, "path": "a.md" }))
    );
    Ok(())
}

#[test]
fn a_markdown_file_and_its_links_are_filled_in_too() -> TestResult {
    let store = store()?;
    let garden = file("Garden.md", "# Garden\n[[Roses]] and [[Tulips]]");
    let roses = file("Roses.md", "# Roses\nred");
    let first = Imports::new(&store)
        .from_source("/vault/Garden.md")
        .linked("From Garden", &[garden.clone(), roses.clone()], &[])?;
    let area = area_of(&first)?;
    assert_eq!(
        store.current(&area, keys::IMPORT_SOURCE)?,
        Some(source_id(ImportSourceKind::Markdown, "/vault/Garden.md"))
    );
    assert_eq!(
        paths_inside(&store, &area)?,
        set(&["Garden.md", "Roses.md"])
    );
    let paths = ["Garden.md", "Roses.md", "sub/Tulips.md"];
    let found = Imports::new(&store)
        .earlier(ImportSourceKind::Markdown, "/vault/Garden.md", paths)?
        .ok_or("an earlier Import")?;
    assert_eq!((found.area_id.as_str(), found.held), (area.as_str(), 2));
    assert_eq!(
        Imports::new(&store).earlier(ImportSourceKind::Folder, "/vault/Garden.md", paths)?,
        None
    );
    let filled = Imports::new(&store)
        .from_source("/vault/Garden.md")
        .filling_in(&area)
        .linked(
            "From Garden",
            &[
                garden,
                roses,
                file("sub/Tulips.md", "# Tulips\nyellow, see [[Roses]]"),
            ],
            &[],
        )?;
    assert_eq!(filled.area_id.as_deref(), Some(area.as_str()));
    assert_eq!((filled.notes, filled.already), (1, 2));
    // A link to a file the first Import made stays as written.
    let new = with_path(&store, &area, "sub/Tulips.md")?;
    assert!(new.text.contains("see [[Roses]]"), "{}", new.text);
    Ok(())
}
