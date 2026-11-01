//! The AI port in the window (#112, ADR 0026): off by default, opened
//! from the Settings with the client config to copy; while it is open,
//! a tool call is answered as the window idles, and the top says "AI
//! port open, sync paused". "Done, review" closes it and shows the
//! review: a section per kind of change, before and after, the user's
//! own later edit marked as staying; Accept all keeps the changes, Reject
//! all asks once and keeps nothing; a task that changed nothing is
//! declined without a review. A task left waiting shows its review on
//! start, and a bundle is refused meanwhile with words that say why.
//! Who may connect (#167) is chosen on the card while the port is
//! closed, this computer only until then, and kept for the next start;
//! the Docker choice without a Docker bridge says so and does not open.
//! The set-ups of Tallies, Checklists and Shopping Lists, Frogs and
//! Sessions (#168) show in groups of their own, before and after.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::ai_port::{Reach, docker_bridge};
use oldbrainfix_core::schedule::{ClockTime, Day, Schedule};
use oldbrainfix_core::tallies::Tallies;
use oldbrainfix_core::{
    Checklists, Frogs, Notes, OwnProfile, Places, SessionRule, Sessions, SigningKey, Store,
};
use oldbrainfix_gui::ai_view::{AiAction, PortShown, port_card};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Page, Selection};
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
    app.open(Page::Settings);
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
    // In the list, the Settings closed.
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert_eq!(harness.state().page(), None);
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
fn the_set_ups_of_tallies_lists_frogs_and_sessions_show_in_groups_of_their_own() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let notes = Notes::new(&store);
    let run = notes.capture("Run", None, None)?.id;
    let trip = notes.capture("Trip\nSocks", None, None)?.id;
    let bank = notes.capture("Call the bank", None, None)?.id;
    let ten: Vec<ClockTime> = ClockTime::new(10, 0).into_iter().collect();
    let daily = Schedule::new(Day::new(2026, 3, 2), &ten, None).ok_or("a schedule")?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || {
        Tallies::new(&store).add_field(&run, "Distance", "km", Some(5000.0), None)?;
        Checklists::new(&store).make(&trip)?;
        Frogs::new(&store).spawn(&bank)?;
        Sessions::new(&store).spawn(
            &bank,
            &daily,
            std::slice::from_ref(&run),
            2,
            SessionRule::Random,
        )?;
        Ok(())
    })?;

    let harness = window(App::new(store.clone(), "en"));
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    for group in [
        t.ai_kind_tally(),
        t.ai_kind_list(),
        t.ai_kind_frog(),
        t.ai_kind_session(),
    ] {
        assert!(shows(&harness, group), "{group}");
    }
    assert!(!shows(&harness, t.ai_kind_routine()), "the Session as one");
    assert!(shows(
        &harness,
        &t.ai_tally_field("Distance", t.tally_target_badge())
    ));
    assert!(shows(&harness, &t.ai_before(t.tally_no_target())));
    assert!(shows(&harness, &t.ai_after("5 km")));
    assert!(shows(&harness, t.ai_field_kind()));
    assert!(shows(&harness, &t.ai_before(t.ai_plain_note())));
    assert!(shows(&harness, &t.ai_after(t.checklist_heading())));
    assert!(shows(&harness, &t.ai_after("Socks")));
    assert!(shows(&harness, &t.ai_after(t.session_rule_random())));
    assert!(shows(&harness, &t.ai_after("Run")), "the pool by its Notes");
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
    assert_eq!(harness.state().page(), Some(Page::Settings));
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

#[test]
fn who_may_connect_is_chosen_on_the_card_and_kept() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = settings(&store);
    assert!(shows(&harness, t.ai_reach_heading()));
    assert!(shows(&harness, t.ai_reach_local()));
    assert_eq!(store.ai_reach()?, Reach::Local);
    assert!(!shows(&harness, t.ai_reach_docker_detail()));

    click(&mut harness, t.ai_reach_docker());
    assert_eq!(store.ai_reach()?, Reach::Docker);
    assert!(shows(&harness, t.ai_reach_docker_detail()));
    let no_bridge = cfg!(target_os = "linux") && docker_bridge().is_none();
    assert_eq!(shows(&harness, t.ai_reach_no_bridge()), no_bridge);

    // Kept, and the port closed at the next start.
    let mut harness = settings(&store);
    assert!(harness.state().ai_port().is_none());
    assert!(shows(&harness, t.ai_reach_docker_detail()));
    click(&mut harness, t.ai_reach_local());
    assert_eq!(store.ai_reach()?, Reach::Local);
    assert!(!shows(&harness, t.ai_reach_docker_detail()));
    assert!(!shows(&harness, t.ai_reach_no_bridge()));

    // Open, the choice is not offered: it holds until the port closes.
    click(&mut harness, t.ai_port_open());
    let url = harness.state().ai_port().ok_or("the port is open")?.url();
    assert!(url.starts_with("http://127.0.0.1:"));
    assert!(!shows(&harness, t.ai_reach_heading()));
    Ok(())
}

#[test]
fn the_docker_choice_without_a_bridge_says_so_and_does_not_open() {
    let t = L10n::new("en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui_state(
            |ui, asked: &mut Option<AiAction>| {
                oldbrainfix_gui::theme::install(ui.ctx());
                let tones = oldbrainfix_gui::theme::tones(ui.visuals());
                let shown = PortShown {
                    url: None,
                    waiting: false,
                    copied: None,
                    reach: Reach::Docker,
                    no_bridge: true,
                };
                if let Some(action) = port_card(ui, &L10n::new("en"), tones, shown) {
                    *asked = Some(action);
                }
            },
            None,
        );
    harness.run();
    assert!(shows_in(&harness, t.ai_reach_no_bridge()));
    assert!(shows_in(&harness, t.ai_reach_docker_detail()));
    match harness.get_all_by_label(t.ai_port_open()).last() {
        Some(node) => node.click_accesskit(),
        None => panic!("no {}", t.ai_port_open()),
    }
    harness.run();
    assert_eq!(*harness.state(), None, "the port was not asked to open");
}

fn shows_in<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}
