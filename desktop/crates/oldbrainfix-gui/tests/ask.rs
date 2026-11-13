//! The Ask panel on the desktop (#267, ADR 0051): AI ▸ Ask… and Ctrl+K
//! show it; it names the provider and what is sent, asks a fake provider
//! whose tools are answered as the AI sees the store, shows the answer
//! with links that open the Notes it came from, keeps it as a Note, and
//! leaves a change the model made waiting for its review; the provider's
//! card in the Settings keeps it all on this computer. Driven headless by
//! egui_kittest, never over the network.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::{Arc, Mutex};

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::ask::{Protocol, Reply, Request, Transport};
use oldbrainfix_core::{Notes, Places, SigningKey, Store, parse_time};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Page, Selection};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A provider that answers with its replies in turn and keeps what it
/// was sent.
#[derive(Clone, Default)]
struct Fake {
    replies: Arc<Mutex<Vec<Value>>>,
    sent: Arc<Mutex<Vec<String>>>,
}

impl Fake {
    fn new(replies: Vec<Value>) -> Fake {
        Fake {
            replies: Arc::new(Mutex::new(replies.into_iter().rev().collect())),
            sent: Arc::default(),
        }
    }

    fn sent(&self) -> String {
        self.sent.lock().unwrap().concat()
    }
}

impl Transport for Fake {
    fn post(&self, request: &Request) -> Result<Reply, String> {
        self.sent.lock().unwrap().push(request.body.clone());
        let body = self.replies.lock().unwrap().pop().ok_or("no reply")?;
        Ok(Reply {
            status: 200,
            body: body.to_string(),
        })
    }
}

fn tool_use(name: &str, input: Value) -> Value {
    json!({
        "content": [{"id": format!("call-{name}"), "input": input, "name": name, "type": "tool_use"}],
        "stop_reason": "tool_use",
    })
}

fn end_turn(text: &str) -> Value {
    json!({"content": [{"text": text, "type": "text"}], "stop_reason": "end_turn"})
}

const PASSPORT: &str = "Passport in the blue folder";

/// A store with "Passport in the blue folder", Anthropic's key entered;
/// the Note's id.
fn store() -> Result<(Store, String), Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-13T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([7; 32])))?;
    let note = Notes::new(&store).capture(PASSPORT, None, None)?;
    store.set_ask_key("sk-ant-test-key")?;
    Ok((store, note.id))
}

fn harness(store: Store, fake: &Fake) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.set_ask_transport(Arc::new(fake.clone()));
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
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The text field a screen reader calls `name`.
fn field<'a>(harness: &'a Harness<'_, App>, name: &'a str) -> egui_kittest::Node<'a> {
    match harness
        .query_all_by(|n| {
            matches!(n.role(), Role::TextInput | Role::PasswordInput)
                && n.label().as_deref() == Some(name)
        })
        .next()
    {
        Some(node) => node,
        None => panic!("no field {name}"),
    }
}

/// Opens the panel with Ctrl+K and asks `question` with Enter.
fn ask(harness: &mut Harness<'_, App>, question: &str) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    let hint = L10n::new("en").ask_hint();
    field(harness, hint).type_text(question);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

#[test]
fn ai_ask_and_ctrl_k_show_the_panel_with_the_cursor_in_its_field() -> TestResult {
    let (store, _) = store()?;
    let mut harness = harness(store, &Fake::default());
    harness.get_all_by_label("AI").next().unwrap().click();
    harness.run();
    harness
        .get_all_by_label("Ask… Ctrl+K")
        .last()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Ask);
    let hint = L10n::new("en").ask_hint();
    assert!(field(&harness, hint).accesskit_node().is_focused());

    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Ask);
    Ok(())
}

#[test]
fn the_panel_names_the_provider_and_what_is_sent_or_what_is_missing() -> TestResult {
    let (store, _) = store()?;
    let t = L10n::new("en");
    let mut harness = harness(store, &Fake::default());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert!(shows(
        &harness,
        &t.ask_sends("Anthropic", "claude-opus-5-5")
    ));

    harness.state().store().set_ask_key("")?;
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert!(shows(&harness, t.ask_unready_key()));
    harness
        .get_by_role_and_label(Role::Button, t.ask_open_settings())
        .click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::Settings));
    assert!(shows(&harness, t.ask_settings_title()));
    Ok(())
}

#[test]
fn an_answer_shows_with_links_that_open_the_notes_it_came_from() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        tool_use("read_note", json!({"id": passport})),
        end_turn(&format!(
            "It is in the [blue folder](oldbrainfix://note/{passport})."
        )),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    assert!(shows(&harness, "Where is my passport?"));
    assert!(shows(&harness, t.ask_from_notes()));
    // The tools read the Note itself.
    assert!(fake.sent().contains(PASSPORT));
    harness.get_by_role_and_label(Role::Link, PASSPORT).click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(passport.as_str()));
    // The panel stays under the Note.
    assert_eq!(harness.state().selection(), &Selection::Ask);
    harness.key_press(Key::Escape);
    harness.run();
    // The link in the answer's text opens it too.
    harness
        .get_by_role_and_label(Role::Link, "blue folder")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(passport.as_str()));
    Ok(())
}

#[test]
fn keep_as_note_saves_the_question_and_its_answer_and_leaving_drops_the_rest() -> TestResult {
    let (store, passport) = store()?;
    let answer = format!("In the [blue folder](oldbrainfix://note/{passport}).");
    let fake = Fake::new(vec![end_turn(&answer), end_turn("Also in the drawer.")]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    ask(&mut harness, "And the ID card?");
    // A follow-up is asked after the first exchange.
    assert!(fake.sent().contains("In the [blue folder]"));
    harness
        .get_all_by_role_and_label(Role::Button, t.ask_keep())
        .last()
        .unwrap()
        .click();
    harness.run();
    let kept = Notes::new(harness.state().store())
        .all()?
        .into_iter()
        .find(|n| n.text.starts_with("Where is my passport?"))
        .ok_or("the kept Note")?;
    assert_eq!(kept.text, format!("Where is my passport?\n\n{answer}"));
    assert_eq!(kept.text_format, "markdown");
    assert!(shows(&harness, t.ask_kept()));
    assert_eq!(harness.state().ask_panel().exchanges.len(), 2);

    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert!(harness.state().ask_panel().exchanges.is_empty());
    assert!(!shows(&harness, "And the ID card?"));
    Ok(())
}

#[test]
fn a_note_hidden_from_ai_is_in_no_request() -> TestResult {
    let (store, _) = store()?;
    let notes = Notes::new(&store);
    let diary = notes.capture("Diary: the kidney results came back", None, None)?;
    let area = Places::new(&store).add_area("Health papers")?;
    let letter = notes.capture("Kidney clinic letter", Some(&area.id), None)?;
    store.set_hidden_from_ai(&diary.id, true)?;
    store.set_hidden_from_ai(&area.id, true)?;
    let fake = Fake::new(vec![
        tool_use(
            "find_notes",
            json!({"words": ["kidney", "clinic", "diary"]}),
        ),
        tool_use("list_new", json!({"since": "2000-01-01"})),
        tool_use("read_note", json!({"id": diary.id})),
        tool_use("read_note", json!({"id": letter.id})),
        tool_use("list_places", json!({})),
        tool_use("list_tags", json!({})),
        end_turn("Nothing about that."),
    ]);
    let mut harness = harness(store, &fake);
    ask(&mut harness, "What about my kidney?");
    let sent = fake.sent();
    assert!(sent.contains(PASSPORT), "list_new found the passport");
    for secret in ["kidney results", "clinic letter", "Health papers"] {
        assert!(!sent.contains(secret), "{secret} was sent");
    }
    Ok(())
}

#[test]
fn a_change_the_model_makes_waits_for_its_review_with_sync() -> TestResult {
    let (store, passport) = store()?;
    let area = Places::new(&store).add_area("Papers")?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        end_turn("Nothing to change."),
        tool_use("file_note", json!({"id": passport, "place": area.id})),
        end_turn("Filed into Papers; it waits for your review."),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    // Asking reads only: no AI Task, sync goes on.
    assert_eq!(harness.state().store().ai_task()?, None);
    assert!(!harness.state().store().sync_paused()?);
    assert!(!shows(&harness, t.ask_task_waits()));

    ask(&mut harness, "Sort what's new");
    assert!(harness.state().store().ai_task()?.is_some());
    assert!(harness.state().store().sync_paused()?);
    assert!(shows(&harness, t.ask_task_waits()));
    harness
        .get_by_role_and_label(Role::Button, t.ai_review_open())
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    Ok(())
}

#[test]
fn a_question_not_answered_is_announced_and_goes_back_into_the_field() -> TestResult {
    let (store, _) = store()?;
    let fake = Fake::new(vec![json!({"content": [], "stop_reason": "refusal"})]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Tell me something");
    assert_eq!(harness.state().notice(), Some(t.ask_failed()));
    assert_eq!(harness.state().ask_panel().question, "Tell me something");
    assert!(harness.state().ask_panel().exchanges.is_empty());
    assert!(!shows(&harness, "sk-ant-test-key"));
    Ok(())
}

#[test]
fn the_providers_card_keeps_it_on_this_computer_only() -> TestResult {
    let (store, _) = store()?;
    let t = L10n::new("en");
    let mut harness = harness(store, &Fake::default());
    harness.state_mut().open(Page::Settings);
    harness.run();
    let search = t.settings_search_hint();
    field(&harness, search).click();
    harness.run();
    field(&harness, search).type_text("API key");
    harness.run();
    assert!(shows(&harness, t.ask_settings_title()));
    assert!(shows(&harness, t.ask_key_stays()));
    harness
        .get_by_role_and_label(Role::RadioButton, t.ask_openai())
        .click();
    harness.run();
    let provider = harness.state().store().ask_provider()?;
    assert_eq!(provider.protocol, Protocol::OpenAi);
    assert_eq!(provider.key, "");
    field(&harness, t.ask_url()).click();
    harness.run();
    field(&harness, t.ask_url()).type_text("http://api.example.com/v1");
    harness.run();
    assert!(shows(&harness, t.ask_unready_url()));
    field(&harness, t.ask_model()).click();
    harness.run();
    field(&harness, t.ask_model()).type_text("deepseek-chat");
    harness.run();
    field(&harness, t.ask_key()).click();
    harness.run();
    field(&harness, t.ask_key()).type_text("sk-or-typed-secret");
    harness.run();
    let provider = harness.state().store().ask_provider()?;
    assert_eq!(provider.url, "http://api.example.com/v1");
    assert_eq!(provider.model, "deepseek-chat");
    assert_eq!(provider.key, "sk-or-typed-secret");
    // Shown as dots, and in no entry of the log.
    assert!(!shows(&harness, "sk-or-typed-secret"));
    for entry in harness.state().store().all_entries()? {
        let row = format!("{} {:?}", entry.field, entry.value);
        assert!(!row.contains("secret") && !row.contains("sk-ant"), "{row}");
    }
    // Anthropic keeps its own key.
    harness
        .get_by_role_and_label(Role::RadioButton, t.ask_anthropic())
        .click();
    harness.run();
    assert_eq!(
        harness.state().store().ask_provider()?.key,
        "sk-ant-test-key"
    );
    Ok(())
}

/// Asks `question` as [`ask`] does, off the window's thread: frames
/// stepped, as the spinner keeps asking for them.
fn ask_off_thread(harness: &mut Harness<'_, App>, question: &str) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    let hint = L10n::new("en").ask_hint();
    field(harness, hint).type_text(question);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run_steps(3);
}

/// Steps the window until `done` says so, as the thread asking answers.
fn until(harness: &mut Harness<'_, App>, done: impl Fn(&App) -> bool) {
    for _ in 0..300 {
        harness.step();
        if done(harness.state()) {
            harness.run();
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("never done");
}

#[test]
fn off_the_windows_thread_the_tools_are_answered_on_it() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn(&format!("[Here](oldbrainfix://note/{passport})")),
    ]);
    let mut harness = harness(store, &fake);
    harness.state_mut().set_redraw(Arc::new(|| {}));
    ask_off_thread(&mut harness, "Where is my passport?");
    until(&mut harness, |app| !app.ask_panel().exchanges.is_empty());
    assert!(harness.state().ask_panel().asking.is_none());
    assert!(fake.sent().contains(PASSPORT));
    assert!(shows(&harness, PASSPORT));
    Ok(())
}

/// A provider that answers only after a while.
struct Slow(Fake);

impl Transport for Slow {
    fn post(&self, request: &Request) -> Result<Reply, String> {
        std::thread::sleep(std::time::Duration::from_millis(300));
        self.0.post(request)
    }
}

#[test]
fn stop_puts_the_question_back_and_drops_what_comes_after() -> TestResult {
    let (store, _) = store()?;
    let t = L10n::new("en");
    let fake = Fake::new(vec![end_turn("Too late.")]);
    let mut app = App::new(store, "en");
    app.set_ask_transport(Arc::new(Slow(fake.clone())));
    app.set_redraw(Arc::new(|| {}));
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
    ask_off_thread(&mut harness, "Where is my passport?");
    assert!(harness.state().ask_panel().asking.is_some());
    harness
        .get_by_role_and_label(Role::Button, t.ask_stop())
        .click();
    harness.step();
    harness.step();
    assert!(harness.state().ask_panel().asking.is_none());
    assert_eq!(
        harness.state().ask_panel().question,
        "Where is my passport?"
    );
    std::thread::sleep(std::time::Duration::from_millis(500));
    harness.step();
    harness.step();
    assert!(harness.state().ask_panel().exchanges.is_empty());
    Ok(())
}
