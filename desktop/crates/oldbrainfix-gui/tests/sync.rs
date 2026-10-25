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
    app.select(Selection::Settings);
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
