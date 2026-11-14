use super::*;
use crate::lan::{answer_sync, apply_answer, sync_request};
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use crate::{Notes, Places};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn device(seed: u8) -> Result<Store> {
    Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
}

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

#[test]
fn a_bundle_carries_everything_sealed_and_strangers_read_nothing() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    let doctor = Places::new(&desk).add_area("Doctor")?;
    Notes::new(&desk).capture_with(
        "Blood test",
        Some(&doctor.id),
        None,
        &[b"xray".to_vec()],
        &[],
    )?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("all.obfsync");
    let (entries, blobs) = write_bundle_now(&desk, &path, None)?.ok_or("a bundle")?;
    assert_eq!(entries, desk.all_entries()?.len());
    assert_eq!(blobs, 1);
    let raw = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
    for secret in ["Blood", "Doctor", "note:", "area:"] {
        assert!(!raw.contains(secret), "{secret}");
    }
    let got = import_bundle(&phone, &path)?;
    assert!(!got.applied.is_empty());
    assert_eq!(got.blobs_in, 1);
    assert_eq!(got.from, desk.device_id()?);
    assert_eq!(got.moment.map(|m| m.cause).as_deref(), Some(cause::BUNDLE));
    assert_eq!(texts(&phone)?, texts(&desk)?);
    assert!(import_bundle(&phone, &path)?.applied.is_empty());

    let stranger = device(3)?;
    let seen = import_bundle(&stranger, &path)?;
    assert!(seen.needs_pairing());
    assert!(texts(&stranger)?.is_empty());
    Ok(())
}

#[test]
fn a_bundle_for_a_paired_device_carries_what_it_lacks() -> TestResult {
    let phone = device(1)?;
    let desk = device(2)?;
    Notes::new(&desk).capture("Old", None, None)?;
    meet(&phone, &desk)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("delta.obfsync");
    let phone_id = phone.device_id()?;
    assert!(write_bundle_now(&desk, &path, Some(&phone_id))?.is_none());
    assert!(!path.exists());
    let before = desk.all_entries()?.len();
    Notes::new(&desk).capture("New", None, None)?;
    let (entries, _) = write_bundle_now(&desk, &path, Some(&phone_id))?.ok_or("a bundle")?;
    assert_eq!(entries, desk.all_entries()?.len() - before);
    import_bundle(&phone, &path)?;
    assert_eq!(texts(&phone)?, vec!["New", "Old"]);
    assert_eq!(phone.held_by(&desk.device_id()?)?, desk.version_vector()?);
    phone.forget_device(&desk.device_id()?)?;
    assert!(phone.held_by(&desk.device_id()?)?.is_empty());
    Ok(())
}

#[test]
fn a_bundle_carries_the_removal_of_a_captured_detail() -> TestResult {
    use crate::CapturedDetail;
    use crate::notes::keys;
    let phone = device(1)?;
    let desk = device(2)?;
    let id = Notes::new(&desk).capture("Order toner", None, None)?.id;
    desk.append(&id, keys::CALENDAR, Some(r#"["Katzenklo"]"#), None)?;
    desk.append(&id, keys::BLUETOOTH, Some(r#"["Headset"]"#), None)?;
    meet(&phone, &desk)?;
    Notes::new(&desk).remove_captured(&id, CapturedDetail::Device)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("captured.obfsync");
    write_bundle_now(&desk, &path, Some(&phone.device_id()?))?.ok_or("a bundle")?;
    import_bundle(&phone, &path)?;
    let context = Notes::new(&phone).by_id(&id)?.ok_or("the Note")?.context;
    assert!(context.bluetooth_devices.is_empty());
    assert_eq!(context.calendar_events, ["Katzenklo"]);
    Ok(())
}

#[test]
fn a_bundle_carries_a_checklists_lines_and_checks() -> TestResult {
    use crate::Checklists;
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    let note = Notes::new(&desk)
        .capture("Trip\nBath\n  Soap\nTent", None, None)?
        .id;
    let lists = Checklists::new(&desk);
    lists.make(&note)?;
    let made = lists.by_note(&note)?.ok_or("a Checklist")?;
    let soap = made.lines.iter().find(|l| l.text == "Soap").ok_or("Soap")?;
    lists.check(&soap.id)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("list.obfsync");
    write_bundle_now(&desk, &path, Some(&phone.device_id()?))?.ok_or("a bundle")?;
    import_bundle(&phone, &path)?;
    let shown = |store: &Store| -> Result<Vec<(String, bool, bool)>> {
        let list = Checklists::new(store).by_note(&note)?;
        Ok(list
            .map(|l| l.lines)
            .unwrap_or_default()
            .iter()
            .map(|l| (l.text.clone(), l.in_group(), l.checked))
            .collect())
    };
    assert_eq!(
        shown(&phone)?,
        [
            ("Bath".to_string(), false, true),
            ("Soap".to_string(), true, true),
            ("Tent".to_string(), false, false),
        ]
    );
    assert_eq!(shown(&phone)?, shown(&desk)?);
    Ok(())
}

#[test]
fn a_bundle_leaves_the_shared_folder_to_read_in_full() -> TestResult {
    use crate::folder::{MemoryFolder, folder_sync};
    let phone = device(1)?;
    let desk = device(2)?;
    meet(&phone, &desk)?;
    Notes::new(&desk).capture("First", None, None)?;
    let mut folder = MemoryFolder::default();
    folder_sync(&desk, &mut folder, &mut random_nonce)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("b.obfsync");
    write_bundle_now(&desk, &path, None)?;
    import_bundle(&phone, &path)?;
    assert!(phone.local_settings_with_prefix("folderRead:")?.is_empty());
    Notes::new(&desk).capture("Second", None, None)?;
    folder_sync(&desk, &mut folder, &mut random_nonce)?;
    folder_sync(&phone, &mut folder, &mut random_nonce)?;
    assert_eq!(texts(&phone)?, texts(&desk)?);
    Ok(())
}

#[test]
fn a_file_that_is_no_bundle_says_so() -> TestResult {
    let phone = device(1)?;
    let dir = tempfile::tempdir()?;
    let junk = dir.path().join("junk.obfsync");
    std::fs::write(&junk, b"not a zip at all")?;
    assert!(matches!(
        import_bundle(&phone, &junk),
        Err(Error::NotABundle)
    ));
    assert!(matches!(
        import_bundle(&phone, &dir.path().join("missing")),
        Err(Error::NotABundle)
    ));
    let zip = dir.path().join("other.zip");
    let mut writer = ZipWriter::new(File::create(&zip)?);
    writer.start_file("photo.jpg", SimpleFileOptions::default())?;
    writer.write_all(b"x")?;
    writer.finish()?;
    assert!(matches!(
        import_bundle(&phone, &zip),
        Err(Error::NotABundle)
    ));
    Ok(())
}

#[test]
fn a_bundle_of_only_newer_formats_needs_an_update() -> TestResult {
    let phone = device(1)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("later.obfsync");
    let mut writer = ZipWriter::new(File::create(&path)?);
    writer.start_file("abc.manifest", SimpleFileOptions::default())?;
    writer.write_all(br#"{"format":2,"later":true}"#)?;
    writer.finish()?;
    assert!(matches!(
        import_bundle(&phone, &path),
        Err(Error::BundleNeedsUpdate)
    ));
    Ok(())
}

#[test]
fn the_file_name_tells_the_day() {
    let day = chrono::NaiveDate::from_ymd_opt(2026, 3, 1);
    assert_eq!(
        day.map(file_name).as_deref(),
        Some("oldbrainfix-2026-03-01.obfsync")
    );
}
