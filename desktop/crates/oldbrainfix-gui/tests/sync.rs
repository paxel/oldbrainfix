//! The Sync view driven headless: it hosts while it shows, asks before a
//! Device it does not know syncs, remembers the Devices it synced with
//! and forgets them on request. A phone joins over the loopback as the
//! Dart core's `lanSync` does.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::thread::JoinHandle;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{DEVICE_HEADER, PIN_HEADER, apply_answer, sync_request};
use oldbrainfix_core::{Notes, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

/// A phone named Pixel with the key `seed` joining on `port` with
/// `pin`, as the Device `claim` when given: what came in, or the refusal
/// as its status and text.
fn join(
    port: u16,
    pin: String,
    seed: u8,
    claim: Option<String>,
) -> JoinHandle<Result<Vec<String>, String>> {
    std::thread::spawn(move || {
        let fail = |e: &dyn std::fmt::Display| e.to_string();
        let phone = store(seed).map_err(|e| fail(&e))?;
        Notes::new(&phone)
            .capture("From the phone", None, None)
            .map_err(|e| fail(&e))?;
        let config = ureq::Agent::config_builder()
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .disable_verification(true)
                    .build(),
            )
            .http_status_as_error(false)
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let base = format!("https://127.0.0.1:{port}");
        let mut res = agent
            .get(format!("{base}/vector"))
            .header(PIN_HEADER, &pin)
            .call()
            .map_err(|e| fail(&e))?;
        let host_device = res
            .headers()
            .get(DEVICE_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let vector: HashMap<String, i64> =
            serde_json::from_str(&res.body_mut().read_to_string().map_err(|e| fail(&e))?)
                .map_err(|e| fail(&e))?;
        let mut body =
            sync_request(&phone, &vector, "Pixel", &host_device, &pin).map_err(|e| fail(&e))?;
        if let Some(claim) = claim {
            body["deviceId"] = claim.into();
        }
        let mut res = agent
            .post(format!("{base}/sync"))
            .header(PIN_HEADER, &pin)
            .content_type("application/json")
            .send(body.to_string().as_bytes())
            .map_err(|e| fail(&e))?;
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if res.status().as_u16() != 200 {
            return Err(format!("{} {text}", res.status().as_u16()));
        }
        let answer = serde_json::from_str(&text).map_err(|e| fail(&e))?;
        apply_answer(&phone, &answer).map_err(|e| fail(&e))?;
        let notes = Notes::new(&phone).all().map_err(|e| fail(&e))?;
        Ok(notes.into_iter().map(|n| n.text).collect())
    })
}

/// Steps the window until `done` holds, at most a few seconds.
fn step_until(harness: &mut Harness<'_, App>, done: impl Fn(&App) -> bool) -> TestResult {
    for _ in 0..400 {
        if done(harness.state()) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

#[test]
fn the_sync_view_hosts_asks_pairs_and_forgets() -> TestResult {
    let desk = store(1)?;
    Notes::new(&desk).capture("From the desk", None, None)?;
    let mut app = App::new(desk, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.select(Selection::Profile);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    harness
        .get_by_label("Sync with another Device")
        .click_accesskit();
    step_until(&mut harness, |app| app.sync_host().hosting())?;
    harness.step();
    let host = harness.state().sync_host().host().ok_or("hosting")?;
    let (port, pin) = (host.port(), host.pin().to_string());
    assert!(shows(&harness, &format!("PIN {pin}")));
    assert!(shows(&harness, "No paired Devices yet."));

    // A Device it does not know is asked about first; refused, it hears so.
    let refused = join(port, pin.clone(), 2, None);
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    assert!(shows(&harness, "Pixel wants to sync with this computer"));
    harness.get_by_label("Refuse").click_accesskit();
    harness.step();
    let refusal = refused.join().map_err(|_| "joiner")?;
    assert_eq!(refusal, Err("403 declined".to_string()));
    assert!(harness.state().store().paired_devices()?.is_empty());

    // Trusted, it syncs, and is paired from then on.
    let trusted = join(port, pin.clone(), 2, None);
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    harness.get_by_label("Trust").click_accesskit();
    harness.step();
    let mut texts = trusted.join().map_err(|_| "joiner")??;
    texts.sort();
    assert_eq!(texts, ["From the desk", "From the phone"]);
    harness.step();
    assert!(shows(&harness, "Synced with Pixel: "));
    let paired = harness.state().store().paired_devices()?;
    assert_eq!(paired.len(), 1);
    assert_eq!(paired[0].name, "Pixel");
    assert_eq!(harness.state().store().moments()?.len(), 1);

    // Another Device claiming the paired one's id is asked about.
    let phone_id = paired[0].id.clone();
    let impostor = join(port, pin.clone(), 3, Some(phone_id));
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    harness.get_by_label("Refuse").click_accesskit();
    harness.step();
    let refusal = impostor.join().map_err(|_| "joiner")?;
    assert_eq!(refusal, Err("403 declined".to_string()));

    // Paired, proven by its key, it is not asked again.
    let again = join(port, pin, 2, None);
    let mut came = None;
    for _ in 0..400 {
        harness.step();
        if again.is_finished() {
            came = Some(again.join().map_err(|_| "joiner")?);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(came.ok_or("the second join never ended")?.is_ok());

    harness.get_by_label("Forget").click_accesskit();
    harness.step();
    assert!(harness.state().store().paired_devices()?.is_empty());

    // Leaving the view stops the session.
    harness.state_mut().select(Selection::Agenda);
    harness.step();
    assert!(!harness.state().sync_host().hosting());
    Ok(())
}

/// Everything `from` holds that `to` lacks, taken in as a session would.
fn send(from: &Store, to: &Store) -> TestResult {
    to.apply_entries(
        &oldbrainfix_core::sync::wire(&from.entries_since(&to.version_vector()?)?),
        &from.key_records()?,
        None,
        &mut oldbrainfix_core::ImportReport::default(),
    )?;
    Ok(())
}

#[test]
fn the_sync_view_lists_the_conflicts_and_keeps_the_version_clicked() -> TestResult {
    let desk = store(1)?;
    let phone = store(2)?;
    let note = Notes::new(&desk).capture("nees mor sauce", None, None)?;
    send(&desk, &phone)?;
    Notes::new(&phone).edit(&note.id, "From the phone")?;
    Notes::new(&desk).edit(&note.id, "Edited at the desk")?;
    send(&phone, &desk)?;
    assert_eq!(desk.conflicts()?.len(), 1);

    let mut app = App::new(desk, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.select(Selection::Sync);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    step_until(&mut harness, |app| app.sync_host().hosting())?;
    harness.step();
    assert!(shows(&harness, "Note text"));
    assert!(shows(&harness, "On this Device, "));
    assert!(shows(&harness, "On a Device without a name, "));
    harness
        .query_all_by_label_contains("From the phone")
        .next()
        .ok_or("the phone's version")?
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "No conflicts. Every Device agrees."));
    let desk = harness.state().store();
    assert!(desk.conflicts()?.is_empty());
    assert_eq!(
        Notes::new(desk).by_id(&note.id)?.map(|n| n.text).as_deref(),
        Some("From the phone")
    );
    // The pick reaches the phone: nothing is open there either.
    send(desk, &phone)?;
    assert!(phone.conflicts()?.is_empty());
    Ok(())
}

fn pair(desk: &Store, id: &str) -> TestResult {
    desk.remember_device(&oldbrainfix_core::PairedDevice {
        id: id.into(),
        name: "Pixel".into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: desk.now(),
        shared: false,
    })?;
    Ok(())
}

#[test]
fn the_paired_host_runs_only_while_a_paired_device_syncs_on_its_own() -> TestResult {
    use oldbrainfix_gui::auto_sync::PairedHosting;
    let desk = store(1)?;
    let mut hosting = PairedHosting::new(IpAddr::V4(Ipv4Addr::LOCALHOST), false);
    hosting.check(&desk);
    assert!(!hosting.hosting(), "nobody to sync with");

    pair(&desk, "phone")?;
    hosting.check(&desk);
    let port = hosting.port().ok_or("hosting")?;
    // It hands out nonces, and lets nobody in on a PIN.
    let asked = std::thread::spawn(move || -> Result<(u16, u16), String> {
        let config = ureq::Agent::config_builder()
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .disable_verification(true)
                    .build(),
            )
            .http_status_as_error(false)
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let base = format!("https://127.0.0.1:{port}");
        let challenge = agent
            .get(format!("{base}/challenge"))
            .call()
            .map_err(|e| e.to_string())?;
        let vector = agent
            .get(format!("{base}/vector"))
            .header(PIN_HEADER, "123456")
            .call()
            .map_err(|e| e.to_string())?;
        Ok((challenge.status().as_u16(), vector.status().as_u16()))
    });
    let sessions = hosting.step(&desk, "Desk", std::time::Duration::from_millis(50));
    assert!(sessions.is_empty());
    assert_eq!(asked.join().map_err(|_| "asker")??, (200, 403));

    desk.set_auto_sync("phone", false)?;
    hosting.check(&desk);
    assert!(!hosting.hosting(), "switched off");
    desk.set_auto_sync("phone", true)?;
    hosting.check(&desk);
    assert_eq!(hosting.port(), Some(port), "the port it had");
    Ok(())
}

#[test]
fn a_paired_device_can_be_kept_from_syncing_on_its_own() -> TestResult {
    let desk = store(1)?;
    pair(&desk, "phone")?;
    let mut app = App::new(desk, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.select(Selection::Sync);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    assert!(shows(&harness, "sync on their own"));
    assert!(harness.state().store().auto_sync_on("phone")?);
    harness.get_by_label("Sync automatically").click_accesskit();
    harness.step();
    assert!(!harness.state().store().auto_sync_on("phone")?);
    harness.get_by_label("Sync automatically").click_accesskit();
    harness.step();
    assert!(harness.state().store().auto_sync_on("phone")?);
    Ok(())
}

#[test]
fn conflicts_an_automatic_sync_brought_are_announced_until_sync_shows() -> TestResult {
    let desk = store(1)?;
    let phone = store(2)?;
    let note = Notes::new(&desk).capture("nees mor sauce", None, None)?;
    send(&desk, &phone)?;
    Notes::new(&phone).edit(&note.id, "From the phone")?;
    Notes::new(&desk).edit(&note.id, "Edited at the desk")?;
    send(&phone, &desk)?;
    desk.set_conflicts_arrived(true)?;

    let mut app = App::new(desk, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    assert_eq!(harness.state().arrived_conflicts(), 1);
    harness
        .query_all_by_label_contains("A sync found 1 thing changed on both Devices")
        .next()
        .ok_or("the announcement")?
        .click_accesskit();
    step_until(&mut harness, |app| app.sync_host().hosting())?;
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Sync);
    assert!(shows(&harness, "Note text"));
    assert!(!harness.state().store().conflicts_arrived()?);
    assert_eq!(harness.state().arrived_conflicts(), 0);
    Ok(())
}

/// The phone writes `text` into a shared folder at `dir`, after a
/// Pairing with `desk` when `paired`.
fn phone_folder(desk: &Store, dir: &std::path::Path, text: &str, paired: bool) -> TestResult {
    use oldbrainfix_core::folder::{LocalFolder, folder_sync, random_nonce};
    use oldbrainfix_core::lan::answer_sync;
    let phone = store(2)?;
    if paired {
        let request = sync_request(&phone, &desk.version_vector()?, "Pixel", "", "")?;
        let (answer, _) = answer_sync(desk, &request, "Desk")?.ok_or("an answer")?;
        apply_answer(&phone, &answer)?;
    }
    Notes::new(&phone).capture(text, None, None)?;
    folder_sync(&phone, &mut LocalFolder::new(dir), &mut random_nonce)?;
    Ok(())
}

fn sync_view(
    desk: Store,
    dir: &std::path::Path,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(desk, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    let path = dir.to_path_buf();
    app.set_folder_picker(Box::new(move |title| {
        assert_eq!(title, "Choose the shared folder");
        Some(path.clone())
    }));
    app.select(Selection::Sync);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    step_until(&mut harness, |app| app.sync_host().hosting())?;
    harness.step();
    Ok(harness)
}

#[test]
fn the_shared_folder_is_chosen_synced_and_forgotten() -> TestResult {
    let desk = store(1)?;
    let dir = tempfile::tempdir()?;
    phone_folder(&desk, dir.path(), "From the phone's folder", true)?;
    let mut harness = sync_view(desk, dir.path())?;
    assert!(shows(&harness, "Shared folder"));
    harness.get_by_label("Choose folder").click_accesskit();
    harness.step();
    harness.step();
    let desk = harness.state().store();
    assert_eq!(
        desk.sync_folder()?.as_deref(),
        Some(dir.path().to_string_lossy().as_ref())
    );
    let texts: Vec<String> = Notes::new(desk)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert_eq!(texts, vec!["From the phone's folder"]);
    assert!(shows(&harness, "Last synced"));
    assert!(shows(&harness, "came in"));
    // The desk's own files are there too, for the phone.
    let manifest = dir
        .path()
        .join("oldbrainfix-sync")
        .join(format!("{}.manifest", desk.device_id()?));
    assert!(manifest.is_file());

    harness.get_by_label("Sync now").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "nothing new"));

    harness.get_by_label("Forget").click_accesskit();
    harness.step();
    harness.step();
    assert_eq!(harness.state().store().sync_folder()?, None);
    assert!(shows(&harness, "Choose folder"));
    assert!(manifest.is_file(), "the folder itself stays");
    Ok(())
}

#[test]
fn a_shared_folder_of_devices_never_paired_reads_as_nothing() -> TestResult {
    let desk = store(1)?;
    let dir = tempfile::tempdir()?;
    phone_folder(&desk, dir.path(), "Private", false)?;
    let mut harness = sync_view(desk, dir.path())?;
    harness.get_by_label("Choose folder").click_accesskit();
    harness.step();
    harness.step();
    assert!(Notes::new(harness.state().store()).all()?.is_empty());
    assert!(shows(&harness, "not paired with"));
    Ok(())
}

#[test]
fn the_folder_thread_syncs_at_once_and_on_its_own() -> TestResult {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let desk = Arc::new(store(1)?);
    let dir = tempfile::tempdir()?;
    phone_folder(&desk, dir.path(), "Through the folder", true)?;
    desk.choose_sync_folder(&dir.path().to_string_lossy())?;
    let (stop, wanted) = (
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    );
    let rounds = Arc::new(AtomicUsize::new(0));
    let thread = {
        let (desk, stop, wanted, rounds) =
            (desk.clone(), stop.clone(), wanted.clone(), rounds.clone());
        std::thread::spawn(move || {
            oldbrainfix_gui::folder_sync::run(&desk, &stop, &wanted, |_| {
                rounds.fetch_add(1, Ordering::SeqCst);
            });
        })
    };
    let wait = |n: usize| {
        for _ in 0..400 {
            if rounds.load(Ordering::SeqCst) >= n {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        false
    };
    assert!(wait(1), "the first round comes at once");
    let texts: Vec<String> = Notes::new(&desk)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert_eq!(texts, vec!["Through the folder"]);
    wanted.store(true, Ordering::SeqCst);
    assert!(wait(2), "asked for, another comes before the half minute");
    stop.store(true, Ordering::SeqCst);
    thread.join().map_err(|_| "the thread")?;
    Ok(())
}

/// A phone that met `desk` once, as at a Pairing.
fn paired_phone(desk: &Store) -> Result<Store, Box<dyn std::error::Error>> {
    use oldbrainfix_core::lan::answer_sync;
    let phone = store(2)?;
    let request = sync_request(&phone, &desk.version_vector()?, "Pixel", "", "")?;
    let (answer, _) = answer_sync(desk, &request, "Desk")?.ok_or("an answer")?;
    apply_answer(&phone, &answer)?;
    Ok(phone)
}

fn texts_of(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(texts)
}

#[test]
fn a_bundle_is_imported_and_one_saved_in_the_sync_view() -> TestResult {
    use oldbrainfix_core::bundle::{import_bundle, write_bundle_now};
    let desk = store(1)?;
    let phone = paired_phone(&desk)?;
    Notes::new(&phone).capture("From the phone's bundle", None, None)?;
    let dir = tempfile::tempdir()?;
    let from_phone = dir.path().join("phone.obfsync");
    write_bundle_now(&phone, &from_phone, None)?;
    Notes::new(&desk).capture("From the desk", None, None)?;

    let mut harness = sync_view(desk, dir.path())?;
    let saved = dir.path().join("for-the-phone");
    {
        let app = harness.state_mut();
        let (open, save) = (from_phone.clone(), saved.clone());
        app.set_bundle_dialogs(
            Box::new(move |title, name| {
                assert_eq!(title, "Save the sync bundle");
                assert!(name.starts_with("oldbrainfix-") && name.ends_with(".obfsync"));
                Some(save.clone())
            }),
            Box::new(move |title| {
                assert_eq!(title, "Open a bundle or a sent Note");
                Some(open.clone())
            }),
        );
    }
    assert!(shows(&harness, "Sync bundle"));
    harness.get_by_label("Import bundle…").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "came in with the bundle"));
    let desk = harness.state().store();
    assert_eq!(
        texts_of(desk)?,
        vec!["From the desk", "From the phone's bundle"]
    );

    harness.get_by_label("Save bundle…").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "Bundle saved with"));
    let file = saved.with_extension("obfsync");
    assert!(file.is_file(), "the extension is added");
    import_bundle(&phone, &file)?;
    assert_eq!(texts_of(&phone)?, texts_of(harness.state().store())?);
    Ok(())
}

#[test]
fn a_bundle_dropped_onto_the_window_is_imported_and_strangers_told_to_pair() -> TestResult {
    use oldbrainfix_core::bundle::write_bundle_now;
    let desk = store(1)?;
    let phone = paired_phone(&desk)?;
    Notes::new(&phone).capture("Dropped in", None, None)?;
    let stranger = store(3)?;
    Notes::new(&stranger).capture("Private", None, None)?;
    let dir = tempfile::tempdir()?;
    let paired = dir.path().join("paired.obfsync");
    let foreign = dir.path().join("foreign.obfsync");
    let junk = dir.path().join("junk.obfsync");
    write_bundle_now(&phone, &paired, None)?;
    write_bundle_now(&stranger, &foreign, None)?;
    std::fs::write(&junk, b"no zip")?;

    let mut app = App::new(desk, "en");
    app.select(Selection::Agenda);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    let drop = |harness: &mut Harness<'_, App>, path: &std::path::Path| {
        harness.input_mut().dropped_files = vec![egui::DroppedFile {
            path: Some(path.to_path_buf()),
            ..Default::default()
        }];
        harness.run();
    };
    drop(&mut harness, &paired);
    assert!(shows(&harness, "came in with the bundle"));
    assert_eq!(texts_of(harness.state().store())?, vec!["Dropped in"]);
    drop(&mut harness, &foreign);
    assert!(shows(&harness, "Pair them first"));
    assert_eq!(texts_of(harness.state().store())?, vec!["Dropped in"]);
    drop(&mut harness, &junk);
    assert!(shows(&harness, "no sync bundle this app can read"));
    // A click puts the announcement away.
    harness
        .get_by_label_contains("no sync bundle")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().notice(), None);
    Ok(())
}
