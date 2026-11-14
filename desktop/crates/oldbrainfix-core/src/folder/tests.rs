use super::*;
use crate::lan::{answer_sync, apply_answer, sync_request};
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use crate::sync::{FOLDER_SOURCE, UpdateNeeded};
use crate::{Notes, Places};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn device(seed: u8) -> Result<Store> {
    Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
}

/// One LAN session without the network, as Pairing hands over keys.
fn meet(joiner: &Store, host: &Store) -> TestResult {
    let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
    let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
    apply_answer(joiner, &answer)?;
    Ok(())
}

fn texts(store: &Store) -> Result<Vec<String>> {
    let mut texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(texts)
}

fn sync(store: &Store, folder: &mut MemoryFolder) -> Result<FolderSyncResult> {
    folder_sync(store, folder, &mut random_nonce)
}

#[test]
fn paired_devices_converge_and_strangers_read_nothing() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    let doctor = Places::new(&desk).add_area("Doctor")?;
    meet(&phone, &desk)?;
    Notes::new(&phone).capture("Feeling dizzy", None, None)?;
    Notes::new(&desk).capture_with(
        "Blood test",
        Some(&doctor.id),
        None,
        &[b"xray".to_vec()],
        &[],
    )?;
    let kids = Places::new(&desk).add_area("Kids")?;
    let mut folder = MemoryFolder::default();
    sync(&phone, &mut folder)?;
    let second = sync(&desk, &mut folder)?;
    assert!(!second.applied.is_empty());
    let third = sync(&phone, &mut folder)?;
    assert_eq!(third.blobs_in, 1);
    assert_eq!(texts(&phone)?, texts(&desk)?);
    assert_eq!(
        phone.writing_key(&kids.id)?.map(|k| k.id()),
        desk.writing_key(&kids.id)?.map(|k| k.id())
    );
    let quiet = sync(&desk, &mut folder)?;
    assert!(quiet.applied.is_empty());
    assert_eq!(quiet.entries_out, 0);

    let stranger = device(3)?;
    let seen = sync(&stranger, &mut folder)?;
    assert!(seen.applied.is_empty());
    assert!(seen.unreadable > 0);
    for files in folder.dirs.values() {
        for bytes in files.values() {
            let text = String::from_utf8_lossy(bytes);
            for secret in ["dizzy", "Blood", "Doctor", "note:", "area:"] {
                assert!(!text.contains(secret), "{secret}");
            }
        }
    }
    Ok(())
}

#[test]
fn a_newer_manifest_is_announced_and_its_data_not_read() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    Notes::new(&phone).capture("Written later", None, None)?;
    let mut folder = MemoryFolder::default();
    sync(&phone, &mut folder)?;
    let name = manifest_name(&phone.device_id()?);
    let bytes = folder.read("", &name)?.ok_or("a manifest")?;
    // Format 1 is never written; a missing one reads as 1.
    let mut manifest: Value = serde_json::from_slice(&bytes)?;
    assert!(manifest.get("format").is_none());
    assert!(Manifest::parse(Some(&bytes)).is_some());
    // The same manifest from a newer version.
    if let Some(map) = manifest.as_object_mut() {
        map.insert("format".into(), json!(MANIFEST_FORMAT + 1));
    }
    folder.write("", &name, manifest.to_string().as_bytes())?;
    let mut nonce = random_nonce;
    let read = folder_round(&desk, &mut folder, &mut nonce)?.ok_or("a round")?;
    assert_eq!(read.newer, 1);
    assert!(read.applied.is_empty());
    assert!(texts(&desk)?.is_empty());
    assert_eq!(desk.folder_status()?.map(|s| s.newer), Some(1));
    assert_eq!(
        desk.take_update_notices()?,
        vec![(FOLDER_SOURCE.to_string(), UpdateNeeded::Here)]
    );
    // Announced once, then forgotten once the folder reads again.
    folder_round(&desk, &mut folder, &mut nonce)?;
    assert!(desk.take_update_notices()?.is_empty());
    folder.write("", &name, &bytes)?;
    let again = folder_round(&desk, &mut folder, &mut nonce)?.ok_or("a round")?;
    assert_eq!(again.newer, 0);
    assert_eq!(texts(&desk)?, vec!["Written later"]);
    desk.note_folder_newer(true)?;
    assert_eq!(
        desk.take_update_notices()?.len(),
        1,
        "news again after it was over"
    );
    Ok(())
}

#[test]
fn a_keys_file_of_a_later_version_is_not_read_and_the_own_one_not_written_over() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    // Its key reaches the phone only through the desk's keys file.
    let kids = Places::new(&desk).add_area("Kids")?;
    let mut folder = MemoryFolder::default();
    sync(&desk, &mut folder)?;
    let personal = desk
        .area_keys()?
        .into_iter()
        .find(|k| k.scope == PERSONAL_SCOPE)
        .ok_or("a personal key")?;
    let own = desk.device_id()?;
    let name = keys_file_name(&own, &personal.id());
    let aad = sealed_as::keys(&own);
    let held = folder.read("keys", &name)?.ok_or("a keys file")?;
    let plain = unseal(&personal.key, &held, &aad).ok_or("opens")?;
    let json: Value = serde_json::from_slice(&plain)?;
    assert!(json.get("v").is_none(), "version 1 says none");
    let with = |v: Value| -> Result<Vec<u8>> {
        let mut json = json.clone();
        if let Some(map) = json.as_object_mut() {
            map.insert("v".into(), v);
        }
        seal(
            &personal.key,
            json.to_string().as_bytes(),
            &aad,
            &random_nonce()?,
        )
    };
    let later = with(json!(2))?;
    folder.write("keys", &name, &later)?;

    let mut nonce = random_nonce;
    let read = folder_round(&phone, &mut folder, &mut nonce)?.ok_or("a round")?;
    assert!(phone.writing_key(&kids.id)?.is_none());
    assert_eq!(read.newer, 1);
    assert_eq!(
        phone.take_update_notices()?,
        vec![(FOLDER_SOURCE.to_string(), UpdateNeeded::Here)]
    );
    // The desk, back at this version, keeps what its newer app wrote.
    Places::new(&desk).add_area("School")?;
    assert_eq!(sync(&desk, &mut folder)?.newer, 1);
    assert_eq!(folder.read("keys", &name)?, Some(later));
    // Version 1 written out reads as before.
    folder.write("keys", &name, &with(json!(1))?)?;
    let again = folder_round(&phone, &mut folder, &mut nonce)?.ok_or("a round")?;
    assert_eq!(again.newer, 0);
    assert_eq!(
        phone.writing_key(&kids.id)?.map(|k| k.id()),
        desk.writing_key(&kids.id)?.map(|k| k.id())
    );
    Ok(())
}

#[test]
fn a_key_that_comes_late_fills_the_gap() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    let work = Places::new(&phone).add_area("Work")?;
    Notes::new(&phone).capture("Report", Some(&work.id), None)?;
    Notes::new(&phone).capture("Loose", None, None)?;
    let mut folder = MemoryFolder::default();
    sync(&phone, &mut folder)?;
    let keys = folder.dirs.get("keys").cloned().unwrap_or_default();
    folder.dirs.insert("keys".into(), BTreeMap::new());
    let early = sync(&desk, &mut folder)?;
    assert_eq!(early.unreadable, 1);
    assert_eq!(texts(&desk)?, vec!["Loose"]);
    folder.dirs.insert("keys".into(), keys);
    let late = sync(&desk, &mut folder)?;
    assert_eq!(late.unreadable, 0);
    assert_eq!(texts(&desk)?, texts(&phone)?);
    Ok(())
}

#[test]
fn full_segments_stay_and_plain_files_carry_the_folder() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    let long = "x".repeat(2000);
    for i in 0..40 {
        Notes::new(&phone).capture(&format!("{i} {long}"), None, None)?;
    }
    let dir = tempfile::tempdir()?;
    folder_sync(&phone, &mut LocalFolder::new(dir.path()), &mut random_nonce)?;
    let own = phone.device_id()?;
    let read = |name: &str| std::fs::read(dir.path().join(ROOT).join(name));
    let manifest = Manifest::parse(Some(&read(&manifest_name(&own))?)).ok_or("manifest")?;
    assert!(manifest.segments.len() > 1);
    let first = read(&manifest.segments[0].name)?;
    assert!(first.len() >= SEGMENT_CAP);
    Notes::new(&phone).capture("After", None, None)?;
    folder_sync(&phone, &mut LocalFolder::new(dir.path()), &mut random_nonce)?;
    assert_eq!(read(&manifest.segments[0].name)?, first);
    folder_sync(&desk, &mut LocalFolder::new(dir.path()), &mut random_nonce)?;
    assert_eq!(texts(&desk)?, texts(&phone)?);
    Ok(())
}

#[test]
fn a_round_keeps_how_it_went_and_a_new_folder_is_read_whole() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    Notes::new(&phone).capture("Hello", None, None)?;
    let mut folder = MemoryFolder::default();
    folder_round(&phone, &mut folder, &mut random_nonce)?;
    desk.choose_sync_folder("/mnt/a")?;
    let got = folder_round(&desk, &mut folder, &mut random_nonce)?.ok_or("a round")?;
    let status = desk.folder_status()?.ok_or("a status")?;
    assert_eq!(status.entries_in, got.applied.len());
    assert!(!status.failed);
    assert_eq!(FolderStatus::from_json(&status.to_json()), Some(status));
    let gone = tempfile::tempdir()?;
    let file = gone.path().join("file");
    std::fs::write(&file, b"")?;
    // A folder that is a file: nothing can be written into it.
    assert!(folder_round(&desk, &mut LocalFolder::new(&file), &mut random_nonce)?.is_none());
    assert_eq!(desk.folder_status()?.map(|s| s.failed), Some(true));
    // A folder moved away is gone, not made again.
    let moved = gone.path().join("moved");
    assert!(folder_round(&desk, &mut LocalFolder::new(&moved), &mut random_nonce)?.is_none());
    assert!(!moved.exists());
    assert!(!desk.local_settings_with_prefix("folderRead:")?.is_empty());
    desk.choose_sync_folder("/mnt/b")?;
    assert!(desk.local_settings_with_prefix("folderRead:")?.is_empty());
    assert_eq!(desk.sync_folder()?.as_deref(), Some("/mnt/b"));
    desk.forget_sync_folder()?;
    assert_eq!(desk.sync_folder()?, None);
    Ok(())
}

#[test]
fn names_from_the_folder_stay_inside_it() {
    let folder = LocalFolder::new("/nowhere");
    assert!(folder.read("", "../etc").is_err());
    assert!(folder.read("blobs", "a/b").is_err());
}
