//! The AI port in the window (#112, ADR 0026): off by default, opened
//! from the AI settings (#335) with the client config to copy, its token
//! kept from one opening to the next; while it is open,
//! a tool call is answered as the window idles, and the top says "AI
//! port open". "Done, review" closes it and shows the
//! review: a section per kind of change, before and after, the user's
//! own later edit marked as staying; Accept all keeps the changes, Reject
//! all asks once and keeps nothing; a task that changed nothing is
//! declined without a review. A task left waiting shows its review on
//! start; a bundle comes in meanwhile, as sync goes on, and the review
//! marks the drafts it touched, which edit Accept keeps.
//! Who may connect (#167) is chosen on the card while the port is
//! closed, this computer only until then, and kept for the next start;
//! the Docker choice without a Docker bridge says so and does not open.
//! The set-ups of Tallies, Checklists and Shopping Lists, Frogs and
//! Sessions (#168) show in groups of their own, before and after; how a
//! Shopping List (#186) or a Checklist (#219) shows on the Agenda in the
//! words of its choice.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::ai_port::{Reach, docker_bridge};
use oldbrainfix_core::schedule::{ClockTime, Day, Schedule};
use oldbrainfix_core::tallies::Tallies;
use oldbrainfix_core::{
    Checklists, Frogs, ListOnAgenda, Notes, OwnProfile, Places, SessionRule, Sessions,
    ShoppingLists, ShoppingOnAgenda, SigningKey, Store,
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

/// The window with the AI settings open (#335), where the AI port's
/// card is.
fn settings(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.open(Page::AiSettings);
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
    assert!(store.ai_task()?.is_none());
    // It works without "AI (experimental)", which is off (#334).
    assert!(!store.ai_on()?);
    assert!(!shows(&harness, t.ai_port_open_banner()));

    click(&mut harness, t.ai_port_open());
    let url = harness.state().ai_port().ok_or("the port is open")?.url();
    assert!(url.starts_with("http://127.0.0.1:"));
    assert!(shows(&harness, &url));
    assert!(store.ai_task()?.is_some());
    assert!(shows(&harness, t.ai_port_open_banner()));
    assert!(!shows(&harness, t.ai_port_open()));

    let token = harness
        .state()
        .ai_port()
        .ok_or("the port is open")?
        .token()
        .to_string();
    let copied = copying(&mut harness, t.ai_port_copy_claude_code());
    assert!(copied.contains(&token), "{copied}");
    assert!(copied.contains(&url), "{copied}");
    assert!(shows(&harness, t.ai_port_copied()));

    // Opened again, the token is the same, so a client set up once
    // stays set up.
    click(&mut harness, t.ai_port_done());
    harness.state_mut().open(Page::AiSettings);
    harness.run_steps(4);
    click(&mut harness, t.ai_port_open());
    let again = harness.state().ai_port().ok_or("the port is open")?;
    assert_eq!(again.token(), token);
    let copied = copying(&mut harness, t.ai_port_copy_config());
    assert!(copied.contains(&token), "{copied}");
    Ok(())
}

/// Clicks `label` and gives what the frames after it copy.
fn copying(harness: &mut Harness<'_, App>, label: &str) -> String {
    match harness.get_all_by_label(label).last() {
        Some(node) => node.click_accesskit(),
        None => panic!("no {label}"),
    }
    let mut copied = String::new();
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                copied.clone_from(text);
            }
        }
    }
    copied
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
    assert_eq!(harness.state().notice(), Some(t.ai_accepted()));
    Ok(())
}

#[test]
fn with_a_wake_the_open_port_is_not_polled_and_a_call_wakes_the_window() -> TestResult {
    use std::sync::atomic::AtomicUsize;
    let store = device()?;
    let t = L10n::new("en");
    let woken = Arc::new(AtomicUsize::new(0));
    let mut app = App::new(store.clone(), "en");
    {
        let woken = woken.clone();
        app.set_redraw(Arc::new(move || {
            woken.fetch_add(1, Ordering::SeqCst);
        }));
    }
    app.open(Page::AiSettings);
    let mut harness = window(app);
    click(&mut harness, t.ai_port_open());
    // Idle with the port open: no frame asks for the next within a second.
    harness.run_steps(3);
    let delay = harness.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert!(
        delay > StdDuration::from_secs(1),
        "the open port is polled: next frame in {delay:?}"
    );
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
    let client = std::thread::spawn(move || -> Result<u16, String> {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build();
        let res = ureq::Agent::new_with_config(config)
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {token}"))
            .send(call.to_string())
            .map_err(|e| e.to_string())?;
        Ok(res.status().as_u16())
    });
    // The window draws only once woken, as the event loop would.
    let deadline = Instant::now() + StdDuration::from_secs(20);
    let mut seen = 0;
    while !client.is_finished() {
        assert!(Instant::now() < deadline, "the call was not answered");
        let now = woken.load(Ordering::SeqCst);
        if now != seen {
            seen = now;
            harness.step();
        }
        std::thread::sleep(StdDuration::from_millis(5));
    }
    assert_eq!(client.join().map_err(|_| "client")??, 200);
    assert_eq!(seen, 1, "one wake for one call");
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
fn how_a_shopping_list_shows_on_the_agenda_reads_in_the_words_of_the_choice() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let groceries = Notes::new(&store)
        .capture("Groceries\nMilk", None, None)?
        .id;
    let lists = ShoppingLists::new(&store);
    lists.make(&groceries)?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || {
        lists.set_on_agenda(&groceries, ShoppingOnAgenda::Lines)
    })?;

    let harness = window(App::new(store.clone(), "en"));
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert!(shows(&harness, t.ai_kind_list()));
    assert!(shows(&harness, t.checklist_show_on_agenda()));
    // None chosen before: as this Device showed Shopping Lists.
    assert!(shows(&harness, &t.ai_before(t.shopping_on_agenda_count())));
    assert!(shows(&harness, &t.ai_after(t.shopping_on_agenda_lines())));
    Ok(())
}

#[test]
fn how_a_checklist_shows_on_the_agenda_reads_in_the_words_of_the_choice() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let trip = Notes::new(&store).capture("Trip\nTent", None, None)?.id;
    let lists = Checklists::new(&store);
    lists.make(&trip)?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || lists.set_on_agenda(&trip, ListOnAgenda::Count))?;

    let harness = window(App::new(store.clone(), "en"));
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert!(shows(&harness, t.ai_kind_list()));
    assert!(shows(&harness, t.checklist_show_on_agenda()));
    // None chosen before: every open Line (#219).
    assert!(shows(&harness, &t.ai_before(t.shopping_on_agenda_lines())));
    assert!(shows(&harness, &t.ai_after(t.shopping_on_agenda_count())));
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
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
    assert!(!shows(&harness, t.ai_review_title()));
    Ok(())
}

/// A desk with the user's phone, met once as at a Pairing, on one clock,
/// so what was written later is later on both.
fn desk_and_phone() -> Result<(Arc<Store>, Store), Box<dyn std::error::Error>> {
    use oldbrainfix_core::lan::{answer_sync, apply_answer, sync_request};
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = |ticks: &Arc<AtomicI64>| -> oldbrainfix_core::Clock {
        let ticks = ticks.clone();
        Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)))
    };
    let desk = Store::in_memory(clock(&ticks), || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&desk).apply_defaults("Device", None)?;
    let phone = Store::in_memory(clock(&ticks), || Ok(SigningKey::from_seed([8; 32])))?;
    let request = sync_request(&phone, &desk.version_vector()?, "Pixel", "", "")?;
    let (answer, _) = answer_sync(&desk, &request, "Desk")?.ok_or("an answer")?;
    apply_answer(&phone, &answer)?;
    Ok((Arc::new(desk), phone))
}

#[test]
fn bundles_come_in_and_go_out_while_a_task_waits_and_the_review_marks_what_came_in() -> TestResult {
    use oldbrainfix_core::lan::{answer_sync, apply_answer, sync_request};
    let (store, phone) = desk_and_phone()?;
    let t = L10n::new("en");
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let bread = Notes::new(&store).capture("Bread", None, None)?;
    let request = sync_request(&phone, &store.version_vector()?, "Pixel", "", "")?;
    let (answer, _) = answer_sync(&store, &request, "Desk")?.ok_or("an answer")?;
    apply_answer(&phone, &answer)?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || {
        Notes::new(&store).edit(&milk.id, "Milk and eggs")?;
        Notes::new(&store).edit(&bread.id, "Rye bread")
    })?;
    // The user fixes the Note on the phone meanwhile; its bundle comes.
    Notes::new(&phone).edit(&milk.id, "Milk, 2 l")?;
    let dir = tempfile::tempdir()?;
    let bundle = dir.path().join("changes.obfsync");
    oldbrainfix_core::bundle::write_bundle_now(&phone, &bundle, None)?.ok_or("a bundle")?;
    let mut harness = window(App::new(store.clone(), "en"));
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert!(shows(&harness, t.ai_review_banner()));
    assert!(!shows(&harness, t.ai_synced_mine_stays()));

    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(bundle),
        ..Default::default()
    }];
    harness.run_steps(4);
    // Taken in, not refused; the task waits on.
    let in_log = store.log_only(|| Notes::new(&store).by_id(&milk.id))?;
    assert_eq!(in_log.ok_or("the Note")?.text, "Milk, 2 l");
    assert!(store.ai_task()?.is_some());
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    // The phone's newer value is marked as staying, once, on Milk only.
    assert_eq!(
        harness.query_all_by_label(t.ai_synced_mine_stays()).count(),
        1
    );
    assert!(!shows(&harness, t.ai_your_edit_stays()));
    assert!(!shows(&harness, t.ai_synced_overwritten()));

    // Once its announcement is gone, the standing line is back, with no
    // word of a pause.
    if let Some(said) = harness.state().notice().map(str::to_string) {
        click(&mut harness, &said);
    }
    assert!(shows(&harness, t.ai_review_banner()));
    assert!(!shows(&harness, "paused"));

    // A bundle is saved meanwhile too, as the log has it: no draft in it.
    let saved = dir.path().join("for-the-phone.obfsync");
    {
        let save = saved.clone();
        harness
            .state_mut()
            .set_bundle_dialogs(Box::new(move |_, _| Some(save.clone())), Box::new(|_| None));
    }
    harness.state_mut().open(Page::Sync);
    harness.run_steps(4);
    click(&mut harness, t.bundle_save());
    assert_eq!(harness.state().notice(), Some(t.bundle_saved()));
    oldbrainfix_core::bundle::import_bundle(&phone, &saved)?;
    let on_phone = Notes::new(&phone).by_id(&bread.id)?.ok_or("the Note")?;
    assert_eq!(on_phone.text, "Bread");
    Ok(())
}

/// The review page alone, showing one change of a Note marked as
/// `mine_wins` and `synced` say.
fn review_with(mine_wins: bool, synced: bool) -> Harness<'static, ()> {
    use oldbrainfix_core::ai_task::AiChangeKind;
    use oldbrainfix_gui::ai_view::{ReviewChange, ReviewItem, ReviewShown, review_page};
    let shown = ReviewShown {
        task: "task".into(),
        groups: vec![(
            AiChangeKind::NoteText,
            vec![ReviewItem {
                entity: "note".into(),
                title: "Milk".into(),
                note: None,
                changes: vec![ReviewChange {
                    label: "Text".into(),
                    before: Some("Milk".into()),
                    after: Some("Milk and eggs".into()),
                    mine_wins,
                    synced,
                }],
            }],
        )],
    };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            oldbrainfix_gui::theme::install(ui.ctx());
            let tones = oldbrainfix_gui::theme::tones(ui.visuals());
            review_page(ui, &L10n::new("en"), tones, &shown, &mut None);
        });
    harness.run();
    harness
}

#[test]
fn the_review_says_which_later_edit_accept_keeps_and_which_it_writes_over() {
    let t = L10n::new("en");
    let marks = [
        t.ai_your_edit_stays(),
        t.ai_synced_mine_stays(),
        t.ai_synced_overwritten(),
    ];
    let cases = [
        ((false, false), None),
        ((true, false), Some(marks[0])),
        ((true, true), Some(marks[1])),
        ((false, true), Some(marks[2])),
    ];
    for ((mine_wins, synced), said) in cases {
        let harness = review_with(mine_wins, synced);
        for mark in marks {
            assert_eq!(
                harness.query_all_by_label(mark).next().is_some(),
                said == Some(mark),
                "{mark} with mine_wins {mine_wins}, synced {synced}"
            );
        }
    }
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
                    at: false,
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

/// The menu bar's `menu`, ahead of the window's widgets of the same name
/// (#279).
fn bar<'a>(harness: &'a Harness<'_, App>, menu: &'a str) -> egui_kittest::Node<'a> {
    match harness.get_all_by_label(menu).next() {
        Some(node) => node,
        None => panic!("no menu {menu}"),
    }
}

/// Whether `entry` of the menu bar's AI menu can be picked; the menu
/// stays open.
fn ai_entry_enabled(harness: &mut Harness<'_, App>, entry: &str) -> bool {
    bar(harness, "AI").click();
    harness.run_steps(4);
    match harness.get_all_by_label(entry).last() {
        Some(node) => !node.accesskit_node().is_disabled(),
        None => panic!("no {entry}"),
    }
}

#[test]
fn the_ai_menu_opens_the_ai_settings_with_the_switch_the_provider_and_the_port() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = window(App::new(store.clone(), "en"));
    assert!(ai_entry_enabled(&mut harness, t.menu_ai_settings()));
    click(&mut harness, t.menu_ai_settings());
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
    assert!(shows(&harness, t.ai_settings_title()));
    assert!(shows(&harness, t.ai_switch()));
    assert!(shows(&harness, t.ai_port_title()));
    assert!(shows(&harness, t.ai_port_open()));
    // The provider's card only while the AI is on (#334).
    let provider = |harness: &Harness<'_, App>| {
        harness
            .query_all_by_label(t.ask_settings_title())
            .next()
            .is_some()
    };
    assert!(!provider(&harness));
    click(&mut harness, t.ai_switch());
    assert!(store.ai_on()?);
    assert!(provider(&harness));
    assert!(shows(&harness, t.ask_key()));
    assert!(shows(&harness, t.ai_port_open()));
    Ok(())
}

#[test]
fn the_settings_keep_one_row_for_the_ai_that_opens_its_page() -> TestResult {
    let store = device()?;
    store.set_ai_on(true)?;
    let t = L10n::new("en");
    let mut app = App::new(store.clone(), "en");
    app.open(Page::Settings);
    let mut harness = window(app);
    // None of the AI's cards, the row in their place.
    for gone in [
        t.ai_switch(),
        t.ai_port_title(),
        t.ai_port_open(),
        t.ask_settings_title(),
        t.ask_key(),
    ] {
        let shown = harness.query_all_by_label(gone).next().is_some();
        assert!(!shown, "{gone} in the Settings");
    }
    assert!(shows(&harness, t.ai_settings_detail()));
    click(&mut harness, t.ai_settings_open());
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
    assert!(shows(&harness, t.ask_settings_title()));
    assert!(shows(&harness, t.ai_port_title()));
    // The arrow leads back to the Settings.
    click(&mut harness, &t.page_back(t.settings_title()));
    assert_eq!(harness.state().page(), Some(Page::Settings));
    Ok(())
}

#[test]
fn the_open_ports_announcement_opens_the_ai_settings_at_its_card() -> TestResult {
    let store = device()?;
    // The provider's card above the port's, OpenAI-compatible with its
    // base URL, pushes it low.
    store.set_ai_on(true)?;
    store.set_ask_protocol(oldbrainfix_core::ask::Protocol::OpenAi)?;
    let t = L10n::new("en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 520.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            App::new(store.clone(), "en"),
        );
    harness.run_steps(4);
    let card_top =
        |harness: &Harness<'_, App>| match harness.get_all_by_label(t.ai_port_title()).last() {
            Some(node) => node.rect().top(),
            None => panic!("no {}", t.ai_port_title()),
        };
    harness.state_mut().open(Page::AiSettings);
    harness.run_steps(4);
    let low = card_top(&harness);
    click(&mut harness, t.ai_port_open());
    harness.state_mut().close_page();
    harness.run_steps(4);

    click(&mut harness, t.ai_port_open_banner());
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
    let top = card_top(&harness);
    assert!(
        top + 100.0 < low,
        "scrolled up to the top: {top}, from {low}"
    );
    assert!(shows(&harness, t.ai_port_done()));
    Ok(())
}

#[test]
fn the_ai_menu_reviews_the_task_that_waits_and_is_greyed_without_one() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let mut harness = window(App::new(store.clone(), "en"));
    assert!(!ai_entry_enabled(&mut harness, t.ai_review_title()));
    bar(&harness, "AI").click();
    harness.run_steps(4);

    // The menu bar is under the AI settings: they close for it.
    harness.state_mut().open(Page::AiSettings);
    harness.run_steps(4);
    click(&mut harness, t.ai_port_open());
    harness.state_mut().close_page();
    harness.run_steps(4);
    // Open, the task does not wait yet.
    assert!(!ai_entry_enabled(&mut harness, t.ai_review_title()));
    bar(&harness, "AI").click();
    harness.run_steps(4);
    let task = task(&harness)?;
    store.drafting(&task, || Notes::new(&store).edit(&milk.id, "Milk and eggs"))?;
    harness.state_mut().open(Page::AiSettings);
    harness.run_steps(4);
    click(&mut harness, t.ai_port_done());
    assert_eq!(harness.state().selection(), &Selection::AiReview);

    harness.state_mut().select(Selection::Agenda);
    harness.run_steps(4);
    assert!(ai_entry_enabled(&mut harness, t.ai_review_title()));
    click(&mut harness, t.ai_review_title());
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    assert!(shows(&harness, &t.ai_after("Milk and eggs")));
    // The menu closed with the pick.
    assert!(!shows(&harness, t.menu_ai_settings()));
    Ok(())
}
