use super::*;
use crate::notes::Notes;
use crate::places::Places;
use crate::shared::AreaAccess;
use crate::store::Clock;
use chrono::TimeZone;
use std::sync::Arc;

const CHEAP: Kdf = Kdf {
    memory: 64,
    iterations: 1,
    parallelism: 1,
};
const PASSPHRASE: &str = "correct horse battery";

type Held = Arc<parking_lot::Mutex<DateTime<Utc>>>;

/// A clock that moves one second per read from where the test puts it.
fn clock(held: &Held) -> Clock {
    let held = held.clone();
    Box::new(move || {
        let mut now = held.lock();
        *now += Duration::seconds(1);
        *now
    })
}

fn held(day: u32) -> Held {
    let at = Utc.with_ymd_and_hms(2026, 10, day, 9, 0, 0).single();
    Arc::new(parking_lot::Mutex::new(at.unwrap_or_default()))
}

fn device(dir: &Path, name: &str, at: &Held, seed: u8) -> Result<Store> {
    let home = dir.join(name);
    std::fs::create_dir_all(&home)?;
    Store::open(&home.join("store.db"), clock(at), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })
}

/// A Device with an Area, a Note with a photo bigger than a chunk and
/// a recording, a Moment, its keys, another Device's key and settings,
/// a playback speed among them.
fn full(dir: &Path) -> Result<Store> {
    let store = device(dir, "old", &held(1), 1)?;
    let area = Places::new(&store).add_area("Doctor")?;
    let mark = store.current_seq()?;
    let photo: Vec<u8> = (0..150_000u32).map(|i| (i * 7 + 3) as u8).collect();
    Notes::new(&store).capture_with(
        "Pills at eight",
        Some(&area.id),
        None,
        &[photo],
        &[vec![42; 900]],
    )?;
    store.add_moment(mark, "sync", Some("Desk"))?;
    store.ensure_area_keys()?;
    let other = Store::in_memory(clock(&held(1)), || Ok(SigningKey::from_seed([9; 32])))?;
    let record = other.own_key_record()?;
    store.learn_keys(
        std::slice::from_ref(&record),
        Some(&record.device),
        &mut ImportReport::default(),
    )?;
    store.choose_sync_folder("/somewhere/sync")?;
    store.set_backup_folder(Some("/somewhere/backups"))?;
    store.set_local_setting("theme", "dark")?;
    DeviceSettings::new(&store).set_playback_speed(1.5)?;
    Ok(store)
}

fn backed_up(dir: &Path) -> Result<(Store, String, std::path::PathBuf)> {
    let store = full(dir)?;
    let code = set_up(&store, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.join(format!("b.{EXTENSION}"));
    write_backup(&store, &path, &mut system_random)?;
    Ok((store, code, path))
}

fn lines(store: &Store) -> Result<Vec<String>> {
    store
        .all_entries()?
        .iter()
        .map(|e| serde_json::to_string(&e.wire()).map_err(|e| Error::Invalid(e.to_string())))
        .collect()
}

fn is(result: Result<Restored>, want: Problem) -> bool {
    matches!(result, Err(Error::Backup(p)) if p == want)
}

#[test]
fn a_backup_restores_everything_but_the_settings_other_than_the_speed_onto_a_new_identity()
-> Result<()> {
    let dir = tempfile::tempdir()?;
    let (old, code, path) = backed_up(dir.path())?;
    let header = read_header(&path)?.ok_or(damaged_err())?;
    let key = unlock(&header, Some(PASSPHRASE), None)?.ok_or(damaged_err())?;
    assert_eq!(
        unlock(&header, None, Some(&code.to_lowercase()))?,
        Some(key)
    );
    let store = device(dir.path(), "new", &held(20), 2)?;
    let got = restore(&store, &path, &key)?;
    assert_eq!(got.from, old.device_id()?);
    assert_ne!(store.device_id()?, old.device_id()?);
    assert_eq!(lines(&store)?, lines(&old)?);
    assert_eq!(got.blobs, 2);
    assert_eq!(held_blobs(&store)?, held_blobs(&old)?);
    let moment = &store.moment_rows()?[0];
    let was = &old.moment_rows()?[0];
    assert_eq!((moment.seq, &moment.at), (was.seq, &was.at));
    let ids =
        |s: &Store| -> Result<Vec<String>> { Ok(s.area_keys()?.iter().map(AreaKey::id).collect()) };
    assert_eq!(ids(&store)?, ids(&old)?);
    let pinned = store.pinned_key(&old.device_id()?)?.ok_or(damaged_err())?;
    assert_eq!(pinned.trust, KeyTrust::Verified);
    let mut kept: Vec<String> = store
        .local_settings_with_prefix("")?
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| {
            !k.starts_with("areakey:") && !k.starts_with("key:") && k != "device" && k != "signseed"
        })
        .collect();
    kept.sort();
    assert_eq!(
        kept,
        vec![
            "backup:key",
            "backup:pass",
            "backup:version",
            "playbackSpeed"
        ]
    );
    assert_eq!(DeviceSettings::new(&store).playback_speed()?, 1.5);
    assert_eq!(store.recovery_code()?, Some(code));
    assert_eq!(store.backup_folder()?, None);
    assert_eq!(store.sync_folder()?, None);
    Ok(())
}

fn damaged_err() -> Error {
    problem(Problem::Damaged)
}

#[test]
fn a_backup_restores_the_removal_of_a_captured_detail() -> Result<()> {
    use crate::CapturedDetail;
    use crate::notes::keys;
    let dir = tempfile::tempdir()?;
    let old = full(dir.path())?;
    let id = Notes::new(&old).capture("Order toner", None, None)?.id;
    old.append(&id, keys::LOCATION, Some("52.52,13.405"), None)?;
    old.append(&id, keys::BLUETOOTH, Some(r#"["Headset"]"#), None)?;
    Notes::new(&old).remove_captured(&id, CapturedDetail::Place)?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join(format!("b.{EXTENSION}"));
    write_backup(&old, &path, &mut system_random)?;
    let header = read_header(&path)?.ok_or(damaged_err())?;
    let key = unlock(&header, None, Some(&code))?.ok_or(damaged_err())?;
    let store = device(dir.path(), "new", &held(20), 2)?;
    restore(&store, &path, &key)?;
    let context = Notes::new(&store).by_id(&id)?.ok_or(damaged_err())?.context;
    assert!(context.location.is_none());
    assert_eq!(context.bluetooth_devices, ["Headset"]);
    Ok(())
}

#[test]
fn a_device_without_a_playback_speed_chosen_leaves_the_new_one_at_normal_speed() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let old = full(dir.path())?;
    old.set_local_setting("playbackSpeed", "7")?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join(format!("b.{EXTENSION}"));
    write_backup(&old, &path, &mut system_random)?;
    let header = read_header(&path)?.ok_or(damaged_err())?;
    let key = unlock(&header, None, Some(&code))?.ok_or(damaged_err())?;
    let store = device(dir.path(), "new", &held(20), 2)?;
    restore(&store, &path, &key)?;
    assert_eq!(store.local_setting("playbackSpeed")?, None);
    assert_eq!(DeviceSettings::new(&store).playback_speed()?, 1.0);
    Ok(())
}

#[test]
fn the_ask_panels_provider_and_key_stay_out_of_a_backup() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let old = full(dir.path())?;
    old.set_ask_key("sk-ant-never-backed-up")?;
    old.set_ask_protocol(crate::ask::Protocol::OpenAi)?;
    old.set_ask_url("https://openrouter.ai/api/v1")?;
    old.set_ask_model("deepseek-chat")?;
    old.set_ask_key("sk-or-never-backed-up")?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join(format!("b.{EXTENSION}"));
    write_backup(&old, &path, &mut system_random)?;
    let header = read_header(&path)?.ok_or(damaged_err())?;
    let key = unlock(&header, None, Some(&code))?.ok_or(damaged_err())?;
    let store = device(dir.path(), "new", &held(20), 2)?;
    restore(&store, &path, &key)?;
    assert!(store.local_settings_with_prefix("ask:")?.is_empty());
    assert_eq!(store.ask_provider()?.key, "");
    Ok(())
}

#[test]
fn a_playback_speed_not_offered_here_or_not_written_as_text_is_passed_over() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let old = full(dir.path())?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let backup_key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let pass = old.backup_pass()?.ok_or(damaged_err())?;
    // A Backup of nothing but the old Device and those speeds, sealed
    // as one last chunk.
    let content_key = [7; KEY_BYTES];
    let sealed_key = seal(&backup_key, &content_key, sealed_as::KEY, &[1; 24])?;
    let line = Header::encode(*held(2).lock(), &pass, &sealed_key, CHUNK_BYTES);
    let payload = [
        json!({"about": {"device": old.device_id()?, "seed": old.signing_key()?.seed_hex()}}),
        json!({"playbackSpeed": "1.25"}),
        json!({"playbackSpeed": 2}),
    ]
    .iter()
    .map(|r| format!("{r}\n"))
    .collect::<String>();
    let aad = sealed_as::chunk(&hex::encode(Sha256::digest(&line)), 0, true);
    let mut file = line.clone();
    file.push(b'\n');
    file.extend(seal(&content_key, payload.as_bytes(), &aad, &[2; 24])?);
    let path = dir.path().join(format!("speeds.{EXTENSION}"));
    std::fs::write(&path, file)?;
    let store = device(dir.path(), "new", &held(20), 2)?;
    restore(&store, &path, &backup_key)?;
    assert_eq!(store.local_setting("playbackSpeed")?, None);
    assert_eq!(DeviceSettings::new(&store).playback_speed()?, 1.0);
    Ok(())
}

#[test]
fn a_wrong_secret_opens_nothing() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (_old, _, path) = backed_up(dir.path())?;
    let header = read_header(&path)?.ok_or(damaged_err())?;
    assert_eq!(unlock(&header, Some("wrong horse battery"), None)?, None);
    let other = recovery::encode_code(&[0; 32]);
    assert_eq!(unlock(&header, None, Some(&other))?, None);
    assert_eq!(unlock(&header, None, Some("nonsense"))?, None);
    let store = device(dir.path(), "new", &held(20), 2)?;
    assert!(is(restore(&store, &path, &[0; 32]), Problem::WrongSecret));
    assert_eq!(store.current_seq()?, 0);
    Ok(())
}

#[test]
fn a_changed_moved_or_cut_off_byte_restores_nothing() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (_old, code, path) = backed_up(dir.path())?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let bytes = std::fs::read(&path)?;
    let head = read_header(&path)?.ok_or(damaged_err())?.line.len() + 1;
    let whole = CHUNK_BYTES + 40;
    let mut flipped = bytes.clone();
    flipped[head + 5000] ^= 1;
    let mut swapped = bytes[..head].to_vec();
    swapped.extend_from_slice(&bytes[head + whole..head + 2 * whole]);
    swapped.extend_from_slice(&bytes[head..head + whole]);
    swapped.extend_from_slice(&bytes[head + 2 * whole..]);
    let text =
        String::from_utf8_lossy(&bytes[..head]).replace("\"created\":\"2026", "\"created\":\"2027");
    let mut header = text.into_bytes();
    header.extend_from_slice(&bytes[head..]);
    let variants = [
        flipped,
        bytes[..head + 2 * whole].to_vec(),
        bytes[..bytes.len() - 3].to_vec(),
        swapped,
        header,
    ];
    for (n, changed) in variants.iter().enumerate() {
        let file = dir.path().join(format!("changed{n}.{EXTENSION}"));
        std::fs::write(&file, changed)?;
        let store = device(dir.path(), &format!("t{n}"), &held(20), 2)?;
        assert!(
            is(restore(&store, &file, &key), Problem::Damaged),
            "variant {n}"
        );
        assert_eq!(store.current_seq()?, 0, "variant {n}");
        assert!(!store.backups_set_up()?, "variant {n}");
    }
    Ok(())
}

#[test]
fn one_of_a_later_format_needs_an_update_in_its_header_or_first_record() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let old = full(dir.path())?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let backup_key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let pass = old.backup_pass()?.ok_or(damaged_err())?;
    let content_key = [7; KEY_BYTES];
    let sealed_key = seal(&backup_key, &content_key, sealed_as::KEY, &[1; 24])?;
    let line = Header::encode(*held(2).lock(), &pass, &sealed_key, CHUNK_BYTES);
    let text = String::from_utf8_lossy(&line).into_owned();
    let mut later: Vec<Vec<u8>> = ["2", r#""1""#, "1.0", "null", "0"]
        .iter()
        .map(|f| format!("{{\"format\":{f},{}\n", &text[1..]).into_bytes())
        .collect();
    // Whatever else a later header holds.
    later.push(format!("{{\"format\":2,\"type\":\"{TYPE}\"}}\n").into_bytes());
    // The first record says it too.
    let aad = sealed_as::chunk(&hex::encode(Sha256::digest(&line)), 0, true);
    let about = b"{\"about\":{\"device\":\"x\",\"format\":2,\"seed\":\"y\"}}\n";
    let mut file = line.clone();
    file.push(b'\n');
    file.extend(seal(&content_key, about, &aad, &[2; 24])?);
    let first = later.len();
    later.push(file);
    for (n, bytes) in later.iter().enumerate() {
        let path = dir.path().join(format!("later{n}.{EXTENSION}"));
        std::fs::write(&path, bytes)?;
        if n < first {
            assert!(
                matches!(read_header(&path), Err(Error::Backup(Problem::NeedsUpdate))),
                "variant {n}"
            );
        }
        let store = device(dir.path(), &format!("l{n}"), &held(20), 2)?;
        assert!(
            is(restore(&store, &path, &backup_key), Problem::NeedsUpdate),
            "variant {n}"
        );
        assert_eq!(store.current_seq()?, 0, "variant {n}");
    }
    // A header that says no Backup, or is not whole, is none.
    for none in [
        r#"{"format":2,"type":"other"}"#,
        r#"{"format":1,"type":"oldbrainfix-backup"}"#,
    ] {
        let path = dir.path().join(format!("none.{EXTENSION}"));
        std::fs::write(&path, format!("{none}\n"))?;
        assert!(read_header(&path)?.is_none(), "{none}");
    }
    Ok(())
}

#[test]
fn a_backup_goes_only_into_an_empty_log_and_a_zip_is_none() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (_old, code, path) = backed_up(dir.path())?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let store = device(dir.path(), "new", &held(20), 2)?;
    let zip = dir.path().join("x.obfbackup");
    std::fs::write(&zip, b"PK\x03\x04 a zip")?;
    assert!(is(restore(&store, &zip, &key), Problem::NotABackup));
    Notes::new(&store).capture("Already here", None, None)?;
    assert!(is(restore(&store, &path, &key), Problem::NotEmpty));
    Ok(())
}

#[test]
fn the_new_device_takes_over_the_old_ones_role_in_a_shared_area() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let old = device(dir.path(), "ann", &held(1), 1)?;
    let area = Places::new(&old).add_area("Family")?;
    old.ensure_area_keys()?;
    let shared = old.share(&area.id)?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join("ann.obfbackup");
    write_backup(&old, &path, &mut system_random)?;
    // The new phone's clock is behind: the role still comes after.
    let store = device(dir.path(), "new", &held(1), 2)?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    assert_eq!(restore(&store, &path, &key)?.roles, 1);
    assert_eq!(store.access_in(&shared, None)?, AreaAccess::Manager);
    let note = Notes::new(&store).capture("Groceries", Some(&shared), None)?;
    let inside: Vec<String> = Notes::new(&store)
        .inside(&shared)?
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(inside, vec![note.id]);
    Ok(())
}

#[test]
fn the_folder_keeps_the_last_four_of_this_device_and_backups_come_weekly() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let at = held(1);
    let store = device(dir.path(), "me", &at, 1)?;
    assert!(!store.backup_due(store.now())?);
    set_up(&store, PASSPHRASE, CHEAP, &mut system_random)?;
    let folder = dir.path().join("folder");
    std::fs::create_dir_all(&folder)?;
    std::fs::write(
        folder.join("oldbrainfix-20200101-000000-ffffffff.obfbackup"),
        "other",
    )?;
    std::fs::write(folder.join("notes.txt"), "mine")?;
    store.set_backup_folder(folder.to_str())?;
    let mut names = Vec::new();
    for _ in 0..6 {
        let Some(name) = back_up_if_due(&store, &mut system_random)? else {
            panic!("a Backup was due");
        };
        names.push(name);
        assert_eq!(back_up_if_due(&store, &mut system_random)?, None);
        *at.lock() += Duration::days(INTERVAL_DAYS);
    }
    let mut left: Vec<String> = std::fs::read_dir(&folder)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    let mut want = vec![
        "notes.txt".to_string(),
        "oldbrainfix-20200101-000000-ffffffff.obfbackup".to_string(),
    ];
    want.extend(names[2..].iter().cloned());
    assert_eq!(left, want);
    store.set_backups_weekly(false)?;
    assert_eq!(back_up_if_due(&store, &mut system_random)?, None);
    Ok(())
}

#[test]
fn a_failure_is_kept_until_a_backup_is_written() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = device(dir.path(), "me", &held(1), 1)?;
    set_up(&store, PASSPHRASE, CHEAP, &mut system_random)?;
    assert!(back_up_now(&store, &dir.path().join("gone"), &mut system_random).is_err());
    assert!(store.backup_failed()?.is_some());
    assert!(store.last_backup()?.is_none());
    back_up_now(&store, dir.path(), &mut system_random)?;
    assert!(store.backup_failed()?.is_none());
    assert!(store.last_backup()?.is_some());
    Ok(())
}

#[test]
fn the_passphrase_derives_as_the_dart_core_derives_it() -> Result<()> {
    let key = pass_key(" password  ", Kdf::STANDARD, b"somesaltsomesalt")?;
    assert_eq!(
        hex::encode(key),
        "7664ad4ba1a3c999fcdd0991ffc2270f78302d2383233db5e7befc85d1bb1819"
    );
    let store = Store::in_memory(clock(&held(1)), || Ok(SigningKey::from_seed([1; 32])))?;
    assert!(set_up(&store, " short ", CHEAP, &mut system_random).is_err());
    assert!(!store.backups_set_up()?);
    Ok(())
}

/// A Device of the user's own with a Profile, named by the user
/// unless `named` is none.
fn own(dir: &Path, name: &str, at: &Held, seed: u8, named: Option<&str>) -> Result<Store> {
    let store = device(dir, name, at, seed)?;
    let profile = OwnProfile::new(&store);
    profile.apply_defaults(name, None)?;
    if let Some(named) = named {
        profile.rename(named)?;
    }
    Ok(store)
}

/// One LAN session, `joiner` asking `host`, both of the user's own.
fn exchange(joiner: &Store, host: &Store) -> Result<Value> {
    use crate::lan::{answer_sync, apply_answer, sync_request};
    let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
    let (answer, _) =
        answer_sync(host, &request, "Desk")?.ok_or(Error::Invalid("no answer".into()))?;
    apply_answer(joiner, &answer)?;
    Ok(request)
}

fn profile_id(store: &Store) -> Result<Option<String>> {
    Ok(OwnProfile::new(store).get()?.map(|p| p.id))
}

#[test]
fn one_recovery_sheet_opens_the_backups_of_every_own_device() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let at = held(1);
    let phone = own(dir.path(), "phone", &at, 1, None)?;
    let desk = own(dir.path(), "desk", &at, 2, Some("Ada"))?;
    exchange(&phone, &desk)?;
    assert_eq!(profile_id(&phone)?, profile_id(&desk)?);
    let code = set_up(&phone, PASSPHRASE, CHEAP, &mut system_random)?;
    let request = exchange(&phone, &desk)?;
    assert_eq!(
        request.get("backupKey"),
        phone.backup_key_to_send()?.as_ref()
    );
    assert_eq!(desk.recovery_code()?, Some(code.clone()));
    assert_eq!(desk.backup_pass()?, phone.backup_pass()?);
    // Each writes its own Backups; the one sheet and the one
    // passphrase open both, and the Profile comes back with them.
    for (i, store) in [&phone, &desk].into_iter().enumerate() {
        let path = dir.path().join(format!("{i}.{EXTENSION}"));
        write_backup(store, &path, &mut system_random)?;
        let header = read_header(&path)?.ok_or(Error::Invalid("no header".into()))?;
        let by_code = unlock(&header, None, Some(&code))?;
        let by_pass = unlock(&header, Some(PASSPHRASE), None)?;
        assert!(by_code.is_some() && by_code == by_pass);
        let restored = device(dir.path(), &format!("new{i}"), &at, 10 + i as u8)?;
        OwnProfile::new(&restored).apply_defaults("New phone", None)?;
        restore(&restored, &path, &by_code.unwrap_or_default())?;
        assert_eq!(profile_id(&restored)?, profile_id(&desk)?);
        assert_eq!(
            OwnProfile::new(&restored).code()?,
            OwnProfile::new(&desk).code()?
        );
        assert_eq!(restored.recovery_code()?, Some(code.clone()));
        // Its key is bound by the old Device's: a rename counts.
        let me = BASE64.encode(restored.signing_key()?.public_key());
        let by = BASE64.encode(store.signing_key()?.public_key());
        let signed = OwnProfile::new(&restored)
            .signed()?
            .ok_or(Error::Invalid("no Profile".into()))?;
        assert!(signed.keys.iter().any(|k| k.key == me && k.by == by));
    }

    // Set up again on the desk: the newer key reaches the phone.
    let again = set_up(&desk, PASSPHRASE, CHEAP, &mut system_random)?;
    assert_eq!(desk.backup_key_version()?, 2);
    exchange(&phone, &desk)?;
    assert_eq!(phone.recovery_code()?, Some(again));
    Ok(())
}

#[test]
fn the_kept_profiles_key_wins_and_the_old_backups_keep_their_sheet() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let at = held(1);
    let phone = own(dir.path(), "phone", &at, 1, Some("Ada"))?;
    let desk = own(dir.path(), "desk", &at, 2, Some("Ada L."))?;
    let phone_code = set_up(&phone, PASSPHRASE, CHEAP, &mut system_random)?;
    let old = dir.path().join(format!("old.{EXTENSION}"));
    write_backup(&phone, &old, &mut system_random)?;
    let desk_code = set_up(&desk, "another passphrase", CHEAP, &mut system_random)?;
    exchange(&phone, &desk)?;
    assert!(OwnProfile::new(&phone).question()?.is_some());
    assert_eq!(phone.recovery_code()?, Some(phone_code.clone()));
    OwnProfile::new(&phone).answer(false)?;
    assert_eq!(phone.backup_key_version()?, 0);
    exchange(&phone, &desk)?;
    assert_eq!(phone.recovery_code()?, Some(desk_code.clone()));
    assert_eq!(desk.recovery_code()?, Some(desk_code));
    let header = read_header(&old)?.ok_or(Error::Invalid("no header".into()))?;
    assert!(unlock(&header, None, Some(&phone_code))?.is_some());
    Ok(())
}

#[test]
fn a_kept_profile_without_backups_takes_the_others_key() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let at = held(1);
    let phone = own(dir.path(), "phone", &at, 1, None)?;
    let desk = own(dir.path(), "desk", &at, 2, Some("Ada"))?;
    let code = set_up(&phone, PASSPHRASE, CHEAP, &mut system_random)?;
    exchange(&phone, &desk)?;
    assert_eq!(profile_id(&phone)?, profile_id(&desk)?);
    exchange(&phone, &desk)?;
    assert_eq!(desk.recovery_code()?, Some(code));
    assert_eq!(desk.backup_key_version()?, 0);
    Ok(())
}

#[test]
fn a_new_device_without_a_profile_takes_profile_places_and_key_joining_or_hosting() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let at = held(1);
    for joins in [true, false] {
        // Still with the defaults, as most are: the other's id may be
        // the lower one, and still its Profile stays.
        let desk = own(dir.path(), &format!("desk{joins}"), &at, 2, None)?;
        let code = set_up(&desk, PASSPHRASE, CHEAP, &mut system_random)?;
        let places = Places::new(&desk);
        let home = places.add_area("Home")?;
        places.add_topic(&home.id, "Kitchen")?;
        let held_by = |store: &Store| -> Result<(Option<String>, Option<String>)> {
            let own = OwnProfile::new(store);
            Ok((own.get()?.map(|p| p.encode()), own.code()?))
        };
        let kept = held_by(&desk)?;
        let phone = device(dir.path(), &format!("phone{joins}"), &at, 1)?;
        if joins {
            exchange(&phone, &desk)?;
        } else {
            exchange(&desk, &phone)?;
        }
        assert_eq!(held_by(&desk)?, kept);
        assert_eq!(held_by(&phone)?, kept);
        assert_eq!(phone.recovery_code()?, Some(code));
        assert!(phone.backups_set_up()?);
        let places = Places::new(&phone);
        let areas: Vec<String> = places.areas()?.into_iter().map(|a| a.name).collect();
        assert_eq!(areas, ["Home"]);
        let topics: Vec<String> = places
            .topics(&home.id)?
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(topics, ["Kitchen"]);
        // The phone's key is bound to the Profile on the desk at once.
        let signed = OwnProfile::new(&desk)
            .signed()?
            .ok_or(Error::Invalid("no Profile".into()))?;
        let keys: Vec<String> = signed.keys.into_iter().map(|k| k.key).collect();
        assert_eq!(keys, [BASE64.encode(phone.signing_key()?.public_key())]);
    }
    Ok(())
}

#[test]
fn a_key_is_taken_only_for_the_own_profile_whole_and_newer() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let at = held(1);
    let store = own(dir.path(), "a", &at, 1, Some("Ada"))?;
    let other = own(dir.path(), "b", &at, 2, Some("Ada"))?;
    set_up(&other, PASSPHRASE, CHEAP, &mut system_random)?;
    let sent = other
        .backup_key_to_send()?
        .ok_or(Error::Invalid("none".into()))?;
    let id = profile_id(&store)?.unwrap_or_default();
    let with = |changes: Value| {
        let mut v = sent.clone();
        v["profile"] = json!(id);
        if let (Some(map), Some(more)) = (v.as_object_mut(), changes.as_object()) {
            for (k, x) in more {
                map.insert(k.clone(), x.clone());
            }
        }
        v
    };
    assert!(!store.take_backup_key(Some(&sent))?, "another Profile");
    assert!(!store.take_backup_key(None)?);
    assert!(!store.take_backup_key(Some(&with(json!({"key": "no base64!"}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"key": "AQID"}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"pass": "none"}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"version": -1}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"version": "2"}))))?);
    assert!(!store.backups_set_up()?);
    let low = BASE64.encode([1u8; 32]);
    let high = BASE64.encode([2u8; 32]);
    assert!(store.take_backup_key(Some(&with(json!({"key": high, "version": 2}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"key": high, "version": 2}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"key": low, "version": 2}))))?);
    assert!(!store.take_backup_key(Some(&with(json!({"key": low, "version": 1}))))?);
    assert!(store.take_backup_key(Some(&with(json!({"key": low, "version": 3}))))?);
    assert_eq!(store.backup_key()?, Some([1u8; 32]));
    assert_eq!(store.backup_key_version()?, 3);
    Ok(())
}

// ------------------------------------------------ taking Notes back

/// A Backup of [`full`] with a Transcript on its recording, a Note in
/// its trash, one filed nowhere and a Shared Area, and its key.
fn with_notes(dir: &Path) -> Result<(Store, std::path::PathBuf, [u8; KEY_BYTES])> {
    let store = full(dir)?;
    let notes = Notes::new(&store);
    let pills = notes.all()?.into_iter().next().ok_or(damaged_err())?;
    let recording = pills.recordings.first().ok_or(damaged_err())?;
    notes.set_transcript(&pills.id, recording, "Eight sharp")?;
    let area = Places::new(&store).add_area("Old")?;
    let doctor = pills.place.clone().ok_or(damaged_err())?;
    let old = notes.capture("Old list", Some(&doctor), None)?;
    notes.trash(&old.id)?;
    notes.capture("Call the vet", None, None)?;
    store.share(&area.id)?;
    let code = set_up(&store, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.join(format!("n.{EXTENSION}"));
    write_backup(&store, &path, &mut system_random)?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    Ok((store, path, key))
}

fn settings(store: &Store) -> Result<Vec<(String, String)>> {
    store.local_settings_with_prefix("")
}

fn by_name(area: &Area) -> String {
    area.name.clone()
}

fn utc_wall(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

fn refused<T>(result: Result<T>, want: Problem) -> bool {
    matches!(result, Err(Error::Backup(p)) if p == want)
}

#[test]
fn a_backup_opens_as_a_packs_tree_and_leaves_the_log_as_it_is() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (old, path, key) = with_notes(dir.path())?;
    let store = device(dir.path(), "o", &held(20), 2)?;
    Notes::new(&store).capture("Already here", None, None)?;
    let seq = store.current_seq()?;
    let before = settings(&store)?;
    let opened = open_notes(&store, &path, &key, &HashSet::new(), by_name)?;
    assert_eq!(store.current_seq()?, seq);
    assert_eq!(settings(&store)?, before);
    assert!(held_blobs(&store)?.is_empty());
    let header = read_header(&path)?.ok_or(damaged_err())?;
    assert_eq!(opened.created, header.created);
    let names: Vec<&str> = opened.pack.areas.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Doctor"]);
    let texts: Vec<&str> = opened.pack.notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(texts, ["Old list", "Pills at eight", "Call the vet"]);
    assert_eq!(opened.trashed, BTreeSet::from([0]));
    // As the Device that made it reads it, files and sizes alike.
    let held = |kind: &str, hash: &str| old.blob(kind, hash).map(|b| b.len());
    let there = old.pack_of_everything(&HashSet::new(), by_name, &held)?;
    assert_eq!(opened.pack.tree(), there.pack.tree());
    assert_eq!(opened.trashed, there.trashed);
    let pills = opened.pack.notes.get(1).ok_or(damaged_err())?;
    assert_eq!(pills.photos.first().map(|p| p.size), Some(150_000));
    assert_eq!(pills.recordings.first().map(|a| a.size), Some(900));
    assert_eq!(
        pills.recordings.first().and_then(|a| a.text.as_deref()),
        Some("Eight sharp")
    );
    // Nothing of a Note is ticked, everything under each is.
    let ticks = opened.pack.backup_ticks();
    let mut under = opened.pack.all_keys();
    for i in 0..3 {
        under.remove(&crate::packs::item_keys::note(i));
    }
    assert_eq!(ticks, under);
    assert_eq!(opened.pack.size(&ticks), 0);
    Ok(())
}

#[test]
fn add_takes_the_ticked_notes_in_as_copies_and_nothing_of_the_backups_identity() -> Result<()> {
    use crate::packs::item_keys;
    let dir = tempfile::tempdir()?;
    let (old, path, key) = with_notes(dir.path())?;
    let store = device(dir.path(), "p", &held(20), 2)?;
    Places::new(&store).add_area("Doctor")?;
    let before = settings(&store)?;
    let opened = open_notes(&store, &path, &key, &HashSet::new(), by_name)?;
    let mut ticks = opened.pack.backup_ticks();
    for i in [0, 1] {
        ticks = opened.pack.tick(&ticks, &item_keys::note(i), true);
    }
    // The recording stays behind, its Transcript comes.
    ticks.remove(&item_keys::recording(1, 0));
    let mark = store.current_seq()?;
    let added = add_notes(
        &store,
        &opened,
        &ticks,
        &utc_wall,
        "Backup",
        "From a Backup",
    )?;
    assert_eq!(added.notes, 2);
    let area = added.areas.first().ok_or(damaged_err())?;
    assert_eq!(
        Places::new(&store).area_by_id(area)?.map(|a| a.name),
        Some("Doctor (Backup)".to_string())
    );
    let copies = Notes::new(&store).inside(area)?;
    let pills = copies
        .iter()
        .find(|n| n.text == "Pills at eight")
        .ok_or(damaged_err())?;
    assert!(
        copies
            .iter()
            .any(|n| n.text == "Old list" && !n.is_trashed())
    );
    let photo = pills.photos.first().ok_or(damaged_err())?;
    assert_eq!(store.photo(photo), old.photo(photo));
    assert!(pills.recordings.is_empty());
    let recording = pills.removed_recordings.first().ok_or(damaged_err())?;
    assert_eq!(
        pills.transcripts.get(recording).map(String::as_str),
        Some("Eight sharp")
    );
    assert_eq!(store.audio(recording), None);
    // Nothing of the Backup's identity, keys, Profile or roles.
    assert_eq!(settings(&store)?, before);
    let own = store.device_id()?;
    let old_ids: HashSet<String> = old.all_entries()?.into_iter().map(|e| e.entity).collect();
    for e in store.entries_after(mark)? {
        assert_eq!(e.device, own);
        assert!(!old_ids.contains(&e.entity), "{}", e.entity);
    }
    Ok(())
}

#[test]
fn a_backup_carries_attached_files_back_whole_and_into_copies() -> Result<()> {
    use crate::notes::NewFile;
    use crate::packs::item_keys;
    let dir = tempfile::tempdir()?;
    let old = full(dir.path())?;
    let notes = Notes::new(&old);
    let pills = notes.all()?.into_iter().next().ok_or(damaged_err())?;
    let lease = notes.attach_file(
        &pills.id,
        &NewFile {
            name: "Lease.pdf",
            media_type: "",
            bytes: b"%PDF-1.4 lease",
            pages: Some(1),
        },
    )?;
    notes.set_attachment_text(&pills.id, &lease, "Rent due")?;
    let code = set_up(&old, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join(format!("f.{EXTENSION}"));
    write_backup(&old, &path, &mut system_random)?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let restored = device(dir.path(), "new", &held(20), 2)?;
    restore(&restored, &path, &key)?;
    assert_eq!(restored.file(&lease), old.file(&lease));
    let other = device(dir.path(), "other", &held(20), 3)?;
    let opened = open_notes(&other, &path, &key, &HashSet::new(), by_name)?;
    let i = opened
        .pack
        .notes
        .iter()
        .position(|n| !n.attachments.is_empty())
        .ok_or(damaged_err())?;
    let ticks = opened
        .pack
        .tick(&opened.pack.backup_ticks(), &item_keys::note(i), true);
    add_notes(
        &other,
        &opened,
        &ticks,
        &utc_wall,
        "Backup",
        "From a Backup",
    )?;
    let copy = Notes::new(&other)
        .all()?
        .into_iter()
        .find(|n| !n.attachments.is_empty())
        .ok_or(damaged_err())?;
    assert_eq!(copy.attachments[0].name, "Lease.pdf");
    assert_eq!(other.file(&lease), old.file(&lease));
    assert_eq!(
        copy.attachment_texts.get(&lease).map(String::as_str),
        Some("Rent due")
    );
    Ok(())
}

#[test]
fn opening_refuses_as_restoring_does() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (_old, path, key) = with_notes(dir.path())?;
    let store = device(dir.path(), "r", &held(20), 2)?;
    assert!(refused(
        open_notes(&store, &path, &[0; 32], &HashSet::new(), by_name),
        Problem::WrongSecret
    ));
    let bytes = std::fs::read(&path)?;
    let header = read_header(&path)?.ok_or(damaged_err())?;
    let line = String::from_utf8_lossy(&header.line).into_owned();
    let mut later = format!("{{\"format\":2,{}", &line[1..]).into_bytes();
    later.extend_from_slice(&bytes[header.line.len()..]);
    let mut flipped = bytes.clone();
    let at = bytes.len() - 5;
    flipped[at] ^= 1;
    let variants = [
        (later, Problem::NeedsUpdate),
        (b"no Backup\n".to_vec(), Problem::NotABackup),
        (flipped, Problem::Damaged),
        (bytes[..bytes.len() - 3].to_vec(), Problem::Damaged),
    ];
    for (n, (changed, want)) in variants.into_iter().enumerate() {
        let file = dir.path().join(format!("r{n}.{EXTENSION}"));
        std::fs::write(&file, changed)?;
        assert!(
            refused(
                open_notes(&store, &file, &key, &HashSet::new(), by_name),
                want
            ),
            "variant {n}"
        );
    }
    assert_eq!(store.current_seq()?, 0);
    Ok(())
}

#[test]
fn a_file_changed_after_it_was_opened_adds_nothing() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let (old, path, key) = with_notes(dir.path())?;
    let store = device(dir.path(), "c", &held(20), 2)?;
    let opened = open_notes(&store, &path, &key, &HashSet::new(), by_name)?;
    // Another Backup of the same Device in its place.
    Notes::new(&old).capture("Written since", None, None)?;
    write_backup(&old, &path, &mut system_random)?;
    let ticks = opened.pack.tick(&opened.pack.backup_ticks(), "n1", true);
    assert!(refused(
        add_notes(
            &store,
            &opened,
            &ticks,
            &utc_wall,
            "Backup",
            "From a Backup"
        ),
        Problem::Damaged
    ));
    assert_eq!(store.current_seq()?, 0);
    Ok(())
}

#[test]
fn a_blob_whose_bytes_are_not_its_hash_is_a_damaged_backup() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = device(dir.path(), "b", &held(1), 1)?;
    set_up(&store, PASSPHRASE, CHEAP, &mut system_random)?;
    let backup_key = store.backup_key()?.ok_or(damaged_err())?;
    let pass = store.backup_pass()?.ok_or(damaged_err())?;
    let content_key = [9; KEY_BYTES];
    let sealed_key = seal(&backup_key, &content_key, sealed_as::KEY, &[1; 24])?;
    let line = Header::encode(*held(2).lock(), &pass, &sealed_key, CHUNK_BYTES);
    let hash = hex::encode(Sha256::digest(b"the photo"));
    let about = json!({"about": {
        "device": store.device_id()?,
        "seed": store.signing_key()?.seed_hex(),
    }});
    let payload = format!("{about}\n{{\"blob\":\"photo/{hash}\",\"size\":9}}\nnot it!!!");
    let aad = sealed_as::chunk(&hex::encode(Sha256::digest(&line)), 0, true);
    let mut file = line.clone();
    file.push(b'\n');
    file.extend(seal(&content_key, payload.as_bytes(), &aad, &[2; 24])?);
    let path = dir.path().join(format!("crafted.{EXTENSION}"));
    std::fs::write(&path, file)?;
    assert!(refused(
        open_notes(&store, &path, &backup_key, &HashSet::new(), by_name),
        Problem::Damaged
    ));
    Ok(())
}

#[test]
fn a_backup_reads_as_the_device_that_made_it_not_as_another_persons() -> Result<()> {
    use crate::person_settings::{PersonSettings, keys as person_keys};
    let dir = tempfile::tempdir()?;
    let ada = device(dir.path(), "ada", &held(1), 1)?;
    let places = Places::new(&ada);
    let health = places.add_area("Health")?;
    let work = places.add_area("Work")?;
    Notes::new(&ada).capture("Stretch", Some(&health.id), None)?;
    Notes::new(&ada).capture("Report", Some(&work.id), None)?;
    PersonSettings::new(&ada).set_area_order(&[&work.id, &health.id])?;
    let cy = device(dir.path(), "cy", &held(25), 7)?;
    let mark = cy.current_seq()?;
    let order = json!([health.id, work.id]).to_string();
    cy.append(
        person_keys::ENTITY,
        person_keys::AREA_ORDER,
        Some(&order),
        None,
    )?;
    for e in cy.entries_after(mark)? {
        ada.insert(&e.wire())?;
    }
    let shown: Vec<String> = PersonSettings::new(&ada)
        .areas_in_order_by(by_name)?
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(shown, ["Work", "Health"]);
    let code = set_up(&ada, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join(format!("order.{EXTENSION}"));
    write_backup(&ada, &path, &mut system_random)?;
    let store = device(dir.path(), "w", &held(26), 2)?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let opened = open_notes(&store, &path, &key, &HashSet::new(), by_name)?;
    let names: Vec<&str> = opened.pack.areas.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Work", "Health"]);
    Ok(())
}

#[test]
fn the_notes_of_a_locked_area_closed_here_stay_out_but_not_of_one_that_is_none_here() -> Result<()>
{
    use crate::sessions::{SessionRule, Sessions};
    use crate::tallies::Tallies;
    let dir = tempfile::tempdir()?;
    let ada = device(dir.path(), "ada", &held(1), 1)?;
    let places = Places::new(&ada);
    let notes = Notes::new(&ada);
    let diary = places.add_area("Diary")?;
    let dreams = places.add_topic(&diary.id, "Dreams")?;
    let health = places.add_area("Health")?;
    notes.capture("Dear diary", Some(&diary.id), None)?;
    notes.capture("Flying", Some(&dreams.id), None)?;
    let torn = notes.capture("Torn page", Some(&diary.id), None)?;
    notes.trash(&torn.id)?;
    let laps = notes.capture("Laps", Some(&diary.id), None)?;
    Tallies::new(&ada).add_field(&laps.id, "Laps", "", Some(10.0), None)?;
    let training = notes.capture("Training", Some(&health.id), None)?;
    let pool = std::slice::from_ref(&laps.id);
    Sessions::new(&ada).make(&training.id, pool, 1, SessionRule::Least)?;
    let old = notes.capture("Old list", Some(&health.id), None)?;
    notes.trash(&old.id)?;
    notes.capture("Call the vet", None, None)?;
    let code = set_up(&ada, PASSPHRASE, CHEAP, &mut system_random)?;
    let path = dir.path().join(format!("closed.{EXTENSION}"));
    write_backup(&ada, &path, &mut system_random)?;
    let key = recovery::decode_code(&code).ok_or(damaged_err())?;
    let texts =
        |o: &OpenedBackup| -> Vec<String> { o.pack.notes.iter().map(|n| n.text.clone()).collect() };

    // Its own Backup, the Diary closed here.
    let closed = HashSet::from([diary.id.clone()]);
    let shut = open_notes(&ada, &path, &key, &closed, by_name)?;
    let names: Vec<&str> = shut.pack.areas.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Health"]);
    assert_eq!(texts(&shut), ["Old list", "Training", "Call the vet"]);
    assert_eq!(shut.trashed, BTreeSet::from([0]));
    assert_eq!(shut.left_out, BTreeSet::from([diary.id.clone()]));
    let open = open_notes(&ada, &path, &key, &HashSet::new(), by_name)?;
    for t in ["Dear diary", "Flying", "Torn page", "Laps"] {
        assert!(texts(&open).iter().any(|n| n == t), "{t}");
    }
    assert!(open.left_out.is_empty());
    // An Area of this name here is another Area: nothing stays out.
    let other = device(dir.path(), "other", &held(2), 2)?;
    let mine = Places::new(&other).add_area("Diary")?;
    let elsewhere = open_notes(&other, &path, &key, &HashSet::from([mine.id]), by_name)?;
    assert_eq!(texts(&elsewhere), texts(&open));
    assert!(elsewhere.left_out.is_empty());

    // "Tick all" adds nothing of the Diary, nor the Session left with
    // none of its pool, as a Pack's Session goes only with a Tally.
    let all = shut.pack.all_keys();
    let added = add_notes(&ada, &shut, &all, &utc_wall, "Backup", "From a Backup")?;
    assert_eq!(added.notes, 2);
    let mut copies = Vec::new();
    for area in &added.areas {
        copies.extend(Notes::new(&ada).inside(area)?.into_iter().map(|n| n.text));
    }
    copies.sort();
    assert_eq!(copies, ["Call the vet", "Old list"]);
    Ok(())
}
