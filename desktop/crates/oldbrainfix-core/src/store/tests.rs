use super::*;
use crate::signing::verify_signature;
use chrono::TimeZone;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

/// A clock that starts at 2026-01-01 10:00 UTC and moves one second
/// per read.
pub fn ticking() -> Clock {
    let ticks = Arc::new(AtomicI64::new(0));
    Box::new(move || {
        let n = ticks.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).single();
        start.unwrap_or_default() + chrono::Duration::seconds(n)
    })
}

pub fn store() -> Store {
    match Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32]))) {
        Ok(s) => s,
        Err(e) => panic!("store: {e}"),
    }
}

#[test]
fn it_reads_the_fields_of_many_entities_in_one_query_as_one_by_one() -> Result<()> {
    let store = store();
    let on = |d: u32| Utc.with_ymd_and_hms(2026, 5, d, 0, 0, 0).single();
    store.append("note:a", "text", Some("new"), on(2))?;
    store.append("note:a", "text", Some("old"), on(1))?;
    store.append("note:a", "place", Some("home"), None)?;
    store.append("note:b", "text", Some("b"), None)?;
    store.append("note:b", "text", None, None)?;
    let ids: Vec<String> = (0..1200).map(|i| format!("note:x{i}")).collect();
    for id in &ids {
        store.append(id, "text", Some(id), None)?;
    }
    // Shown pending changes count as they do one by one.
    let set = store.open_pending("ai-task", true, None)?;
    store.draft_pending(&set, "note:a", "place", Some("work"), None)?;
    store.draft_pending(&set, "note:c", "text", Some("drafted"), None)?;
    let mut all = vec!["note:a", "note:b", "note:c", "note:none"];
    all.extend(ids.iter().map(String::as_str));
    let before = store.history_reads.load(AtomicOrdering::SeqCst);
    let read = store.current_fields_of(all.iter().copied())?;
    assert_eq!(store.history_reads.load(AtomicOrdering::SeqCst) - before, 1);
    let mut one_by_one = HashMap::new();
    for id in &all {
        let fields = store.current_fields(id)?;
        if fields.iter().next().is_some() {
            one_by_one.insert(id.to_string(), fields);
        }
    }
    assert!(!read.contains_key("note:none"));
    assert_eq!(read, one_by_one);
    assert_eq!(
        read.get("note:a"),
        Some(&Fields::new(vec![
            ("text".into(), Some("new".into())),
            ("place".into(), Some("work".into())),
        ]))
    );
    assert!(store.current_fields_of([])?.is_empty());
    Ok(())
}

#[test]
fn it_reads_the_fields_of_every_entity_of_a_type_in_one_query_as_one_by_one() -> Result<()> {
    let store = store();
    for i in 0..30 {
        let id = format!("thing:{i}");
        store.append(&id, "$type", Some("thing"), None)?;
        store.append(&id, "text", Some(&format!("t{i}")), None)?;
    }
    store.append("thing:0", "$type", Some("other"), None)?;
    store.append("other:a", "$type", Some("other"), None)?;
    // Shown pending changes count as they do one by one.
    let set = store.open_pending("ai-task", true, None)?;
    store.draft_pending(&set, "thing:new", "$type", Some("thing"), None)?;
    store.draft_pending(&set, "thing:1", "text", Some("drafted"), None)?;
    let before = store.history_reads.load(AtomicOrdering::SeqCst);
    let read = store.current_fields_of_type("thing")?;
    assert_eq!(store.history_reads.load(AtomicOrdering::SeqCst) - before, 1);
    let mut one_by_one = HashMap::new();
    for id in store.entities_of_type("thing")? {
        let fields = store.current_fields(&id)?;
        one_by_one.insert(id, fields);
    }
    assert_eq!(read, one_by_one);
    assert!(!read.contains_key("thing:0"));
    assert!(read.contains_key("thing:new"));
    assert_eq!(
        read.get("thing:1"),
        Some(&Fields::new(vec![
            ("$type".into(), Some("thing".into())),
            ("text".into(), Some("drafted".into())),
        ]))
    );
    Ok(())
}

#[test]
fn a_setting_a_later_version_wrote_is_kept_as_it_is_and_noted() -> Result<()> {
    use crate::sync::{SETTINGS_SOURCE, UpdateNeeded};
    let store = store();
    let later = r#"{"generation":3,"v":2}"#;
    let listed = r#"[{"v":1},{"id":"a","v":"2"}]"#;
    store.set_local_setting("folderRead:x", later)?;
    store.set_local_setting("changes:x", listed)?;
    assert!(store.take_update_notices()?.is_empty());
    store.set_local_setting("folderRead:x", r#"{"generation":1,"lines":{}}"#)?;
    store.remove_local_setting("changes:x")?;
    store.forget_folder_reads()?;
    assert_eq!(store.local_setting("folderRead:x")?.as_deref(), Some(later));
    assert_eq!(store.local_setting("changes:x")?.as_deref(), Some(listed));
    assert_eq!(
        store.take_update_notices()?,
        vec![(SETTINGS_SOURCE.to_string(), UpdateNeeded::Here)]
    );
    // Version 1, said or not, and what is no object of a later one
    // are this version's.
    for mine in [
        r#"{"v":1}"#,
        r#"[{"v":1},2]"#,
        r#""v""#,
        "2",
        "not json",
        r#"{"a":{"v":2}}"#,
    ] {
        store.set_local_setting("k", mine)?;
        store.set_local_setting("k", "next")?;
        assert_eq!(store.local_setting("k")?.as_deref(), Some("next"), "{mine}");
        store.remove_local_setting("k")?;
    }
    // The own Profile's record has a rule of its own.
    store.set_local_setting("profile", r#"{"id":"p","v":2,"version":1}"#)?;
    store.set_local_setting("profile", r#"{"id":"p","version":2}"#)?;
    assert_eq!(
        store.local_setting("profile")?.as_deref(),
        Some(r#"{"id":"p","version":2}"#)
    );
    Ok(())
}

#[test]
fn own_entries_are_signed_and_verify_under_the_device_key() -> Result<()> {
    let store = store();
    store.set_local_setting("author", "Ada")?;
    let e = store.append("note:a", "text", Some("hello"), None)?;
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
    let key = store.signing_key()?;
    assert!(verify_signature(
        &key.public_key(),
        &bytes,
        e.sig.as_deref()
    ));
    assert_eq!(store.device_id()?, device_id_from_key(&key.public_key()));
    assert_eq!(e.author, "Ada");
    assert_eq!(e.date, "2026-01-01T10:00:01.000Z");
    Ok(())
}

#[test]
fn the_latest_date_wins_not_the_latest_append() -> Result<()> {
    let store = store();
    let day = |d| Utc.with_ymd_and_hms(2026, 5, d, 0, 0, 0).single();
    store.append("note:a", "text", Some("new"), day(2))?;
    store.append("note:a", "text", Some("old"), day(1))?;
    assert_eq!(store.current("note:a", "text")?.as_deref(), Some("new"));
    store.append("note:a", "text", None, day(3))?;
    assert_eq!(store.current("note:a", "text")?, None);
    Ok(())
}

#[test]
fn dseq_counts_up_and_a_held_entry_is_not_taken_twice() -> Result<()> {
    let store = store();
    store.append("note:a", "text", Some("1"), None)?;
    let e = store.append("note:a", "text", Some("2"), None)?;
    assert_eq!(e.dseq, 2);
    assert_eq!(store.version_vector()?.get(&e.device), Some(&2));
    assert!(!store.insert(&e.wire())?);
    let mut other = e.wire();
    other.device = "other".into();
    assert!(store.insert(&other)?);
    assert_eq!(store.all_entries()?.len(), 3);
    assert_eq!(e.id(), format!("{}:2", e.device));
    Ok(())
}

#[test]
fn the_device_identity_survives_a_reopen() -> Result<()> {
    let dir = tempfile::tempdir().map_err(|e| Error::Invalid(e.to_string()))?;
    let path = dir.path().join("oldbrainfix.db");
    let first = Store::open(&path, ticking(), SigningKey::generate)?;
    let device = first.device_id()?;
    first.append("note:a", "text", Some("kept"), None)?;
    drop(first);
    let again = Store::open(&path, ticking(), || {
        Err(Error::Invalid("no new key on reopen".into()))
    })?;
    assert_eq!(again.device_id()?, device);
    assert_eq!(again.current("note:a", "text")?.as_deref(), Some("kept"));
    Ok(())
}

/// A store with one Note in `dir`, closed again.
fn saved(dir: &Path) -> Result<std::path::PathBuf> {
    let path = dir.join("oldbrainfix.db");
    let store = Store::open(&path, ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
    store.append("note:a", "text", Some("kept"), None)?;
    drop(store);
    Ok(path)
}

fn version_of(path: &Path) -> Result<i64> {
    Ok(Connection::open(path)?.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

fn set_version(path: &Path, version: i64) -> Result<()> {
    Ok(Connection::open(path)?.execute_batch(&format!("PRAGMA user_version = {version}"))?)
}

/// Every file under `dir`, with its bytes.
fn files(dir: &Path) -> Result<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>> {
    let mut found = std::collections::BTreeMap::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(next) = dirs.pop() {
        for entry in std::fs::read_dir(next)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                found.insert(path.clone(), std::fs::read(&path)?);
            }
        }
    }
    Ok(found)
}

#[test]
fn the_database_version_is_recorded() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = saved(dir.path())?;
    assert_eq!(version_of(&path)?, STORE_VERSION);
    assert_eq!(STORE_VERSION, 1);
    Ok(())
}

#[test]
fn a_database_from_before_the_version_keeps_its_log_and_gets_it() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = saved(dir.path())?;
    set_version(&path, 0)?;
    let store = Store::open(&path, ticking(), || {
        Err(Error::Invalid("no new key on reopen".into()))
    })?;
    assert_eq!(store.current("note:a", "text")?.as_deref(), Some("kept"));
    drop(store);
    assert_eq!(version_of(&path)?, 1);
    Ok(())
}

#[test]
fn a_database_a_newer_version_saved_is_not_opened_and_nothing_changes() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = saved(dir.path())?;
    // The directories beside it are made again only by an open that
    // goes ahead.
    std::fs::remove_dir_all(dir.path().join("photos"))?;
    std::fs::remove_dir_all(dir.path().join("audio"))?;
    std::fs::remove_dir_all(dir.path().join("files"))?;
    for newer in [2, 1000, -1] {
        set_version(&path, newer)?;
        let before = files(dir.path())?;
        let opened = Store::open(&path, ticking(), || {
            Err(Error::Invalid("no new key on reopen".into()))
        });
        assert!(
            matches!(opened, Err(Error::StoreNeedsUpdate(v)) if v == newer),
            "{newer}"
        );
        assert_eq!(files(dir.path())?, before, "{newer}");
        assert!(!dir.path().join("photos").exists());
        assert!(!dir.path().join("files").exists());
        assert_eq!(version_of(&path)?, newer);
    }
    set_version(&path, 1)?;
    let store = Store::open(&path, ticking(), || {
        Err(Error::Invalid("no new key on reopen".into()))
    })?;
    assert_eq!(store.current("note:a", "text")?.as_deref(), Some("kept"));
    Ok(())
}

#[test]
fn ids_are_random_unless_counted_and_a_failed_batch_leaves_nothing() -> Result<()> {
    let mut store = store();
    let a = store.new_id()?;
    assert_eq!(a.len(), 32);
    assert_ne!(a, store.new_id()?);
    store.set_id_maker(Box::new(|| Ok("1".into())));
    assert_eq!(store.new_id()?, "1");
    let failed: Result<()> = store.batch(|| {
        store.append("note:a", "text", Some("lost"), None)?;
        Err(Error::Invalid("on purpose".into()))
    });
    assert!(failed.is_err());
    assert!(store.all_entries()?.is_empty());
    store.batch(|| store.append("note:a", "text", Some("kept"), None))?;
    assert_eq!(store.all_entries()?.len(), 1);
    Ok(())
}

#[test]
fn a_batch_inside_a_batch_joins_it() -> Result<()> {
    let store = store();
    let before = store.generation.load(std::sync::atomic::Ordering::SeqCst);
    store.batch(|| {
        store.append("note:a", "text", Some("one"), None)?;
        store.batch(|| store.append("note:b", "text", Some("two"), None))?;
        assert_eq!(store.current("note:b", "text")?.as_deref(), Some("two"));
        store.append("note:c", "text", Some("three"), None)
    })?;
    assert_eq!(store.all_entries()?.len(), 3);
    let after = store.generation.load(std::sync::atomic::Ordering::SeqCst);
    // Each append counts once, the batch once at its end, not twice.
    assert_eq!(after - before, 4);
    let failed: Result<()> = store.batch(|| {
        store.append("note:d", "text", Some("four"), None)?;
        store.batch(|| {
            store.append("note:e", "text", Some("five"), None)?;
            Err(Error::Invalid("on purpose".into()))
        })
    });
    assert!(failed.is_err());
    assert_eq!(store.all_entries()?.len(), 3, "the outer write is gone too");
    assert_eq!(store.current("note:d", "text")?, None);
    store.batch(|| store.append("note:f", "text", Some("six"), None))?;
    assert_eq!(store.all_entries()?.len(), 4, "the next batch commits");
    Ok(())
}

#[test]
fn blobs_lie_next_to_the_database_by_their_hash() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(&dir.path().join("oldbrainfix.db"), ticking(), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?;
    let photo = store.put_photo(b"not a jpeg")?;
    let audio = store.put_audio(b"RIFF....WAVE")?;
    assert_eq!(
        photo,
        hex::encode(Sha256::digest(b"not a jpeg")),
        "the photo's hash"
    );
    assert_eq!(
        std::fs::read(dir.path().join("photos").join(&photo))?,
        b"not a jpeg"
    );
    assert_eq!(
        std::fs::read(dir.path().join("audio").join(&audio))?,
        b"RIFF....WAVE"
    );
    assert_eq!(store.photo(&photo).as_deref(), Some(&b"not a jpeg"[..]));
    assert_eq!(store.audio(&photo), None);
    assert!(store.has_blob("photo", &photo) && store.has_blob("audio", &audio));
    assert!(!store.has_blob("audio", &photo) && !store.has_blob("photo", "../x"));
    // An attached file is kept as it is, JPEG metadata and all.
    let jpeg = b"\xFF\xD8\xFF\xE1\x00\x04GPS\xFF\xDA\x00\x02";
    let file = store.put_file(jpeg)?;
    assert_eq!(file, hex::encode(Sha256::digest(jpeg)));
    assert_eq!(std::fs::read(dir.path().join("files").join(&file))?, jpeg);
    assert_eq!(store.file(&file).as_deref(), Some(&jpeg[..]));
    assert!(store.has_blob("file", &file) && !store.has_blob("photo", &file));
    assert!(!store.put_blob("file", &file, b"other")?);
    assert!(store.drop_blob("file", &file)?);
    assert!(!store.has_blob("file", &file));
    assert!(store.all_entries()?.is_empty());
    Ok(())
}

#[test]
fn text_orders_by_utf16_code_units_like_dart() {
    // U+FF5E sorts after U+1F600 in UTF-16, before it by code point.
    assert_eq!(utf16_cmp("\u{FF5E}", "\u{1F600}"), Ordering::Greater);
    assert_eq!(utf16_cmp("a", "b"), Ordering::Less);
}
