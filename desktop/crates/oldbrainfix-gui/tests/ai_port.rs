//! The AI port in the window (#112, ADR 0026): off by default, opened
//! from the Settings with the client config to copy; while it is open,
//! a tool call is answered as the window idles, and the top says "AI
//! port open, sync paused". "Done, review" closes it and shows the
//! review: a section per kind of change, before and after, the user's
//! own later edit marked as staying; Accept all keeps the changes, Reject
//! all asks once and keeps nothing; a task that changed nothing is
//! declined without a review. A task left waiting shows its review on
//! start, and a bundle is refused meanwhile with words that say why.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Selection};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn device() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&store).apply_defaults("Device", None)?;
    Ok(Arc::new(store))
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    // An open port repaints every 100 ms, so steps, not run().
    harness.run_steps(4);
    harness
}

fn settings(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Settings);
    window(app)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// Clicks the last widget labelled `label`: a dialog's, over the
/// window's.
fn click(harness: &mut Harness<'_, App>, label: &str) {
    match harness.get_all_by_label(label).last() {
        Some(node) => node.click_accesskit(),
        None => panic!("no {label}"),
    }
    // An open port repaints every 100 ms, so steps, not run().
    harness.run_steps(4);
}

/// The AI Task the open port writes into.
fn task(harness: &Harness<'_, App>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(harness
        .state()
        .ai_port()
        .ok_or("the port is open")?
        .task()
        .to_string())
}

#[test]
fn the_port_is_off_by_default_and_opens_from_the_settings() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = settings(&store);
    assert!(harness.state().ai_port().is_none());
    assert!(!store.sync_paused()?);
    assert!(!shows(&harness, t.ai_port_open_banner()));

    click(&mut harness, t.ai_port_open());
    let url = harness.state().ai_port().ok_or("the port is open")?.url();
    assert!(url.starts_with("http://127.0.0.1:"));
    assert!(shows(&harness, &url));
    assert!(store.sync_paused()?);
    assert!(shows(&harness, t.ai_port_open_banner()));
    assert!(!shows(&harness, t.ai_port_open()));

    click(&mut harness, t.ai_port_copy_claude_code());
    assert!(shows(&harness, t.ai_port_copied()));
    Ok(())
}

#[test]
fn a_tool_call_is_answered_while_the_window_idles_and_kept_on_accept() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = settings(&store);
    click(&mut harness, t.ai_port_open());
    let (url, token) = {
        let port = harness.state().ai_port().ok_or("the port is open")?;
        (port.url(), port.token().to_string())
    };
    let call = json!({
        "id": 1,
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {"name": "add_area", "arguments": {"name": "Garden"}},
    });
    let client = std::thread::spawn(move || -> Result<String, String> {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build();
        let mut res = ureq::Agent::new_with_config(config)
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {token}"))
            .send(call.to_string())
            .map_err(|e| e.to_string())?;
        res.body_mut().read_to_string().map_err(|e| e.to_string())
    });
    let deadline = Instant::now() + StdDuration::from_secs(20);
    while !client.is_finished() {
        assert!(Instant::now() < deadline, "the call was not answered");
        harness.step();
        std::thread::sleep(StdDuration::from_millis(10));
    }
    let answer: Value = serde_json::from_str(&client.join().map_err(|_| "client")??)?;
    assert_eq!(answer["result"]["isError"], json!(false));
    // Drafted: the log has no Garden, the reads have.
    let garden = |store: &Store| -> oldbrainfix_core::Result<bool> {
        Ok(Places::new(store)
            .areas()?
            .iter()
            .any(|a| a.name == "Garden"))
    };
    assert!(garden(&store)?);
    assert!(!store.log_only(|| garden(&store))?);

    click(&mut harness, t.ai_port_done());
    assert!(harness.state().ai_port().is_none());
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert!(shows(&harness, t.ai_kind_place()));
    assert!(shows(&harness, "Garden"));
    assert!(shows(&harness, t.ai_review_banner()));

    click(&mut harness, t.ai_accept_all());
    assert!(store.log_only(|| garden(&store))?);
    assert!(store.ai_task()?.is_none());
    assert!(!store.sync_paused()?);
    assert_eq!(harness.state().notice(), Some(t.ai_accepted()));
    Ok(())
}

#[test]
fn reject_all_asks_once_and_keeps_nothing() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let mut harness = settings(&store);
    click(&mut harness, t.ai_port_open());
    let task = task(&harness)?;
    store.drafting(&task, || Notes::new(&store).edit(&milk.id, "Milk and eggs"))?;

    click(&mut harness, t.ai_port_done());
    assert!(shows(&harness, t.ai_kind_note_text()));
    assert!(shows(&harness, &t.ai_before("Milk")));
    assert!(shows(&harness, &t.ai_after("Milk and eggs")));
    assert!(!shows(&harness, t.ai_your_edit_stays()));

    click(&mut harness, t.ai_reject_all());
    assert!(shows(&harness, t.ai_reject_question()));
    assert!(store.ai_task()?.is_some());
    click(&mut harness, t.ai_reject_all());
    assert!(store.ai_task()?.is_none());
    let kept = Notes::new(&store).by_id(&milk.id)?.ok_or("the Note")?;
    assert_eq!(kept.text, "Milk");
    assert_eq!(harness.state().notice(), Some(t.ai_rejected()));
    Ok(())
}

#[test]
fn the_users_own_later_edit_is_marked_as_staying() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || Notes::new(&store).edit(&milk.id, "Milk and eggs"))?;
    Notes::new(&store).edit(&milk.id, "Oat milk")?;

    // Waiting since before the window opened: its review shows at once.
    let harness = window(App::new(store.clone(), "en"));
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert!(shows(&harness, t.ai_your_edit_stays()));
    Ok(())
}

#[test]
fn a_task_that_changed_nothing_is_declined_without_a_review() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = settings(&store);
    click(&mut harness, t.ai_port_open());
    click(&mut harness, t.ai_port_done());
    assert!(store.ai_task()?.is_none());
    assert!(!store.sync_paused()?);
    assert_eq!(harness.state().selection(), &Selection::Settings);
    assert!(!shows(&harness, t.ai_review_title()));
    Ok(())
}

#[test]
fn a_bundle_is_refused_while_a_task_waits_and_says_why() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || Notes::new(&store).edit(&milk.id, "Milk and eggs"))?;
    let mut harness = window(App::new(store.clone(), "en"));
    assert!(shows(&harness, t.ai_review_banner()));

    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(Path::new("changes.obfsync").to_path_buf()),
        ..Default::default()
    }];
    // An open port repaints every 100 ms, so steps, not run().
    harness.run_steps(4);
    assert_eq!(harness.state().notice(), Some(t.sync_paused_error()));
    // Said at the top, the standing banner back once it is clicked away.
    assert!(shows(&harness, t.sync_paused_error()));
    assert!(!shows(&harness, t.ai_review_banner()));
    click(&mut harness, t.sync_paused_error());
    assert!(shows(&harness, t.ai_review_banner()));
    Ok(())
}
