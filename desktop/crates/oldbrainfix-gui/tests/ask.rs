//! The Ask panel on the desktop (#267, ADR 0051): AI ▸ Ask… and Ctrl+K
//! show it; it names the provider and what is sent, asks a fake provider
//! whose tools are answered as the AI sees the store, shows the answer
//! with links that open the Notes it came from, keeps it as a Note,
//! copies it, adds it to a Note and saves it as Markdown (#316), and
//! leaves a change the model made waiting for its review; the provider's
//! card in the Settings keeps it all on this computer. The questions
//! stand as a thread, the oldest first, each with Edit, and Up in the
//! empty field fetches the last (#315). All of it only
//! while "AI (experimental)" is switched on (#334), off by default.
//! Driven headless by egui_kittest, never over the network.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::ask::{Protocol, Reply, Request, Transport};
use oldbrainfix_core::{DeviceSettings, Notes, Places, SigningKey, Store, parse_time};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::media::Clipboard;
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

/// A store with "Passport in the blue folder", the AI switched on and
/// Anthropic's key entered; the Note's id.
fn store() -> Result<(Store, String), Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-13T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([7; 32])))?;
    let note = Notes::new(&store).capture(PASSPORT, None, None)?;
    store.set_ai_on(true)?;
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

/// Whether a widget is labelled `label` exactly, as a card's heading.
fn labelled(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Opens the AI menu.
fn ai_menu(harness: &mut Harness<'_, App>) {
    harness.get_all_by_label("AI").next().unwrap().click();
    harness.run();
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
fn the_question_field_is_as_tall_as_the_ask_button_beside_it() -> TestResult {
    let (store, _) = store()?;
    let mut harness = harness(store, &Fake::default());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    let t = L10n::new("en");
    let at = field(&harness, t.ask_hint()).rect();
    let button = harness
        .get_by_role_and_label(Role::Button, t.ask_button())
        .rect();
    // One height for fields and buttons (obf-1128), one row.
    assert!(
        (at.height() - button.height()).abs() <= 1.0,
        "the field {at:?}, the button {button:?}"
    );
    assert!((at.center().y - button.center().y).abs() <= 1.0);
    Ok(())
}

#[test]
fn the_ai_is_off_by_default_also_with_a_provider_set_up_before() -> TestResult {
    // Anthropic's key entered before the switch came, never switched on.
    let (store, _) = store()?;
    store.set_ai_on(false)?;
    let t = L10n::new("en");
    let mut harness = harness(store, &Fake::default());
    ai_menu(&mut harness);
    let switch = harness.get_by_label(t.ai_switch());
    assert_eq!(switch.accesskit_node().toggled(), Some(Toggled::False));
    assert!(!shows(&harness, t.menu_ask()));
    assert!(labelled(&harness, t.menu_ai_settings()));
    harness.key_press(Key::Escape);
    harness.run();
    // Ctrl+K does nothing.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    // In a text field it is the field's again: it deletes to the end.
    harness.state_mut().select(Selection::All);
    harness.run();
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    harness.get_by_role(Role::TextInput).type_text("passport");
    harness.run();
    harness.key_press(Key::Home);
    harness.run();
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(harness.state().query(), "");
    assert_eq!(harness.state().selection(), &Selection::All);
    // The AI settings (#335): the switch and the AI port's card, no
    // provider's.
    harness.state_mut().open(Page::AiSettings);
    harness.run();
    assert!(labelled(&harness, t.ai_switch()));
    assert!(labelled(&harness, t.ai_port_title()));
    assert!(labelled(&harness, t.ai_port_open()));
    assert!(!labelled(&harness, t.ask_settings_title()));
    assert!(!labelled(&harness, t.ask_key()));
    // Found by a search of the Settings, the page still has none.
    harness.state_mut().open(Page::Settings);
    harness.run();
    let search = t.settings_search_hint();
    field(&harness, search).click();
    harness.run();
    field(&harness, search).type_text("API key");
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, t.ai_settings_open())
        .click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
    assert!(labelled(&harness, t.ai_switch()));
    assert!(!labelled(&harness, t.ask_settings_title()));
    // The key stays kept.
    assert_eq!(
        harness.state().store().ask_provider()?.key,
        "sk-ant-test-key"
    );
    assert!(!harness.state().store().ai_on()?);
    Ok(())
}

#[test]
fn switched_on_in_the_ai_menu_all_comes_back_and_off_the_panel_goes() -> TestResult {
    let (store, _) = store()?;
    store.set_ai_on(false)?;
    let t = L10n::new("en");
    let mut harness = harness(store, &Fake::default());
    ai_menu(&mut harness);
    harness.get_by_label(t.ai_switch()).click();
    harness.run();
    assert!(harness.state().store().ai_on()?);
    ai_menu(&mut harness);
    let switch = harness.get_by_label(t.ai_switch());
    assert_eq!(switch.accesskit_node().toggled(), Some(Toggled::True));
    assert!(labelled(&harness, "Ask… Ctrl+K"));
    harness.key_press(Key::Escape);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Ask);
    harness.state_mut().open(Page::AiSettings);
    harness.run();
    assert!(labelled(&harness, t.ask_settings_title()));
    // Switched off in the AI settings: the provider's card goes, and the
    // panel with it, back where it was opened from.
    harness
        .get_all_by_label(t.ai_switch())
        .last()
        .unwrap()
        .click_accesskit();
    harness.run();
    assert!(!harness.state().store().ai_on()?);
    assert!(!labelled(&harness, t.ask_settings_title()));
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    // Forward leads not back into it.
    harness.state_mut().close_page();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
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
    // Typed, the question still cannot be asked (#317).
    field(&harness, t.ask_hint()).type_text("Where is my passport?");
    harness.run();
    assert!(
        harness
            .get_by_role_and_label(Role::Button, t.ask_button())
            .accesskit_node()
            .is_disabled()
    );
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().ask_panel().exchanges.is_empty());
    assert!(harness.state().ask_panel().failed.is_none());

    // No model named, and an address it may not ask: the same.
    let reopen_typed = |harness: &mut Harness<'_, App>| {
        harness.state_mut().select(Selection::Agenda);
        harness.run();
        harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
        harness.run();
        field(harness, t.ask_hint()).type_text("Where is my passport?");
        harness.run();
    };
    harness.state().store().set_ask_protocol(Protocol::OpenAi)?;
    reopen_typed(&mut harness);
    assert!(shows(&harness, t.ask_unready_model()));
    assert!(
        harness
            .get_by_role_and_label(Role::Button, t.ask_button())
            .accesskit_node()
            .is_disabled()
    );
    let store = harness.state().store();
    store.set_ask_model("qwen2.5:3b")?;
    store.set_ask_url("http://example.org:11434/v1")?;
    reopen_typed(&mut harness);
    assert!(shows(&harness, t.ask_unready_url()));
    assert!(
        harness
            .get_by_role_and_label(Role::Button, t.ask_button())
            .accesskit_node()
            .is_disabled()
    );
    harness
        .get_by_role_and_label(Role::Button, t.ask_open_settings())
        .click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
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
    // Its link names the Note read: nothing is marked.
    assert!(!shows(&harness, t.ask_unchecked()));
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
fn the_other_notes_read_show_as_also_read_under_the_linked_ones_and_open() -> TestResult {
    let (store, passport) = store()?;
    let booth = Notes::new(&store).capture("Passport photo booth at the station", None, None)?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        tool_use("read_note", json!({"id": booth.id})),
        tool_use("read_note", json!({"id": passport})),
        end_turn(&format!(
            "It is in the [blue folder](oldbrainfix://note/{passport})."
        )),
        // A small model's answer that links nothing (#314).
        tool_use("read_note", json!({"id": passport})),
        end_turn("It is in the blue folder."),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    let asked = &harness.state().ask_panel().exchanges[0];
    assert_eq!(
        asked.sources,
        vec![(passport.clone(), PASSPORT.to_string())]
    );
    assert_eq!(
        asked.also_read,
        vec![(booth.id.clone(), booth.text.clone())]
    );
    assert!(shows(&harness, t.ask_from_notes()));
    assert!(shows(&harness, t.ask_also_read()));
    // Linked first, then "Also read" under them.
    let top = |label: &str| {
        harness
            .get_by_role_and_label(Role::Link, label)
            .rect()
            .min
            .y
    };
    assert!(top(PASSPORT) < top(&booth.text));
    harness
        .get_by_role_and_label(Role::Link, &booth.text)
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(booth.id.as_str()));
    assert_eq!(harness.state().selection(), &Selection::Ask);

    harness.key_press(Key::Escape);
    harness.run();
    ask(&mut harness, "And now?");
    let asked = &harness.state().ask_panel().exchanges[1];
    assert!(asked.sources.is_empty());
    assert_eq!(
        asked.also_read,
        vec![(passport.clone(), PASSPORT.to_string())]
    );
    Ok(())
}

#[test]
fn keep_as_note_saves_the_question_and_its_answer_and_leaving_drops_the_rest() -> TestResult {
    let (store, passport) = store()?;
    let answer = format!("In the [blue folder](oldbrainfix://note/{passport}).");
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn(&answer),
        end_turn("Also in the drawer."),
        // Told to search first (#314), it answers the same.
        end_turn("Also in the drawer."),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    ask(&mut harness, "And the ID card?");
    // A follow-up is asked after the first exchange.
    assert!(fake.sent().contains("In the [blue folder]"));
    // The thread holds the oldest first (#315).
    harness
        .get_all_by_role_and_label(Role::Button, t.ask_keep())
        .next()
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
        // Told to read the passport list_new showed (#314).
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
fn a_change_the_model_makes_waits_for_its_review() -> TestResult {
    let (store, passport) = store()?;
    let area = Places::new(&store).add_area("Papers")?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        end_turn("Nothing to change."),
        // Told to read the Note found (#314), it answers the same.
        end_turn("Nothing to change."),
        tool_use("file_note", json!({"id": passport, "place": area.id})),
        end_turn("Filed into Papers; it waits for your review."),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    // Asking reads only: no AI Task.
    assert_eq!(harness.state().store().ai_task()?, None);
    assert!(!shows(&harness, t.ask_task_waits()));

    ask(&mut harness, "Sort what's new");
    assert!(harness.state().store().ai_task()?.is_some());
    assert!(shows(&harness, t.ask_task_waits()));
    harness
        .get_by_role_and_label(Role::Button, t.ai_review_open())
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::AiReview);
    Ok(())
}

/// A provider that answers each request with the next of its replies,
/// a status and a body, or no reply at all.
#[derive(Clone)]
struct Replies(Arc<Mutex<Vec<Result<Reply, String>>>>);

impl Replies {
    fn new(replies: Vec<Result<Reply, String>>) -> Replies {
        Replies(Arc::new(Mutex::new(replies.into_iter().rev().collect())))
    }
}

impl Transport for Replies {
    fn post(&self, _: &Request) -> Result<Reply, String> {
        self.0
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err("no reply".into()))
    }
}

/// What a provider replies, and the cause, the fix and the provider's
/// words the panel then shows.
type Failing<'a> = (Vec<Result<Reply, String>>, String, &'a str, &'a str);

fn reply(status: u16, body: &str) -> Result<Reply, String> {
    Ok(Reply {
        status,
        body: body.into(),
    })
}

#[test]
fn a_question_not_answered_stays_under_it_with_its_cause_and_fix_until_asked_again() -> TestResult {
    let t = L10n::new("en");
    let refusal = json!({"content": [], "stop_reason": "refusal"}).to_string();
    let searching = tool_use("find_notes", json!({"words": ["x"]})).to_string();
    let kinds: Vec<Failing<'_>> = vec![
        (
            vec![reply(200, &refusal)],
            t.ask_refused().into(),
            t.ask_refused_fix(),
            "",
        ),
        (
            vec![reply(401, r#"{"error":{"message":"invalid x-api-key"}}"#)],
            t.ask_key_refused("Anthropic"),
            t.ask_key_fix(),
            "invalid x-api-key",
        ),
        (
            vec![reply(
                404,
                r#"{"error":{"message":"model: qwen9 not found"}}"#,
            )],
            t.ask_provider_failed("Anthropic"),
            t.ask_provider_fix(),
            "HTTP 404",
        ),
        (
            vec![Err("Connection refused".into())],
            t.ask_no_reply("Anthropic"),
            t.ask_no_reply_fix(),
            "Connection refused",
        ),
        (
            vec![reply(200, "<html>")],
            t.ask_unreadable("Anthropic"),
            t.ask_unreadable_fix(),
            "",
        ),
        (
            (0..30).map(|_| reply(200, &searching)).collect(),
            t.ask_too_many_rounds().into(),
            t.ask_too_many_rounds_fix(),
            "",
        ),
    ];
    for (replies, cause, fix, said) in kinds {
        let (store, _) = store()?;
        let mut app = App::new(store, "en");
        app.set_ask_transport(Arc::new(Replies::new(replies)));
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
        ask(&mut harness, "Tell me something");
        // Under its question, not at the top.
        assert_eq!(harness.state().notice(), None, "{cause}");
        assert!(shows(&harness, &cause), "{cause}");
        assert!(shows(&harness, fix), "{cause}");
        assert!(labelled(&harness, "Tell me something"), "{cause}");
        assert_eq!(harness.state().ask_panel().question, "Tell me something");
        assert!(harness.state().ask_panel().exchanges.is_empty());
        // Nothing to copy, keep, add or save under a failure (#316).
        for action in [
            t.copy_text(),
            t.ask_keep(),
            t.add_to_note(),
            t.save_as_markdown(),
        ] {
            let offered = harness
                .query_all_by_role_and_label(Role::Button, action)
                .next()
                .is_some();
            assert!(!offered, "{cause}: {action}");
        }
        assert!(!shows(&harness, "sk-ant-test-key"));
        // It stays while the user does something else.
        harness.run_steps(5);
        field(&harness, t.ask_hint()).type_text(" now");
        harness.run();
        assert!(shows(&harness, &cause), "{cause} stays");
        // Its details show the provider's words, with Report.
        harness
            .get_by_role_and_label(Role::Button, t.ask_failure_details())
            .click();
        harness.run();
        assert!(shows(&harness, said), "{cause}: {said}");
        assert!(shows(&harness, t.report()), "{cause}");
    }

    // Asked again, the failure goes and the answer shows.
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.set_ask_transport(Arc::new(Replies::new(vec![
        Err("Connection refused".into()),
        reply(200, &end_turn("In the blue folder.").to_string()),
        // Told to search first (#314), it answers the same.
        reply(200, &end_turn("In the blue folder.").to_string()),
    ])));
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
    ask(&mut harness, "Where is my passport?");
    assert!(shows(&harness, &t.ask_no_reply("Anthropic")));
    field(&harness, t.ask_hint()).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!shows(&harness, &t.ask_no_reply("Anthropic")));
    assert!(harness.state().ask_panel().failed.is_none());
    assert!(shows(&harness, "In the blue folder."));
    Ok(())
}

#[test]
fn the_providers_card_keeps_it_on_this_computer_only() -> TestResult {
    let (store, _) = store()?;
    let t = L10n::new("en");
    let mut harness = harness(store, &Fake::default());
    // Found by a search of the Settings, its row opens the AI settings
    // (#335).
    harness.state_mut().open(Page::Settings);
    harness.run();
    let search = t.settings_search_hint();
    field(&harness, search).click();
    harness.run();
    field(&harness, search).type_text("API key");
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, t.ai_settings_open())
        .click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::AiSettings));
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

#[test]
fn let_the_model_think_is_off_and_shown_for_an_openai_compatible_provider_only() -> TestResult {
    let (store, _) = store()?;
    let t = L10n::new("en");
    let mut harness = harness(store, &Fake::default());
    harness.state_mut().open(Page::AiSettings);
    harness.run();
    // The models tried with Ask, for either protocol (#318).
    for model in [
        t.ask_models_title(),
        t.ask_model_qwen25(),
        t.ask_model_qwen3(),
    ] {
        assert!(shows(&harness, model), "{model}");
    }
    assert!(!labelled(&harness, t.ask_thinks()));
    harness
        .get_by_role_and_label(Role::RadioButton, t.ask_openai())
        .click();
    harness.run();
    let thinks = harness.get_by_role_and_label(Role::CheckBox, t.ask_thinks());
    assert_eq!(thinks.accesskit_node().toggled(), Some(Toggled::False));
    assert!(shows(&harness, t.ask_thinks_detail()));
    assert!(!harness.state().store().ask_provider()?.thinks);
    harness
        .get_by_role_and_label(Role::CheckBox, t.ask_thinks())
        .click();
    harness.run();
    assert!(harness.state().store().ask_provider()?.thinks);
    let thinks = harness.get_by_role_and_label(Role::CheckBox, t.ask_thinks());
    assert_eq!(thinks.accesskit_node().toggled(), Some(Toggled::True));
    // Found by a search of the Settings by its name.
    harness.state_mut().open(Page::Settings);
    harness.run();
    let search = t.settings_search_hint();
    field(&harness, search).click();
    harness.run();
    field(&harness, search).type_text("model think");
    harness.run();
    assert!(labelled(&harness, t.ai_settings_open()));
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

#[test]
fn a_link_to_a_note_not_read_goes_and_the_answer_says_a_source_could_not_be_checked() -> TestResult
{
    let (store, passport) = store()?;
    // The model only found the Note, and links it, also once told to
    // read it (#314).
    let linked = end_turn(&format!(
        "It is in the [blue folder](oldbrainfix://note/{passport})."
    ));
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        linked.clone(),
        linked,
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    harness.state_mut().set_redraw(Arc::new(|| {}));
    ask_off_thread(&mut harness, "Where is my passport?");
    until(&mut harness, |app| !app.ask_panel().exchanges.is_empty());
    assert!(shows(&harness, t.ask_unchecked()));
    assert!(shows(&harness, "It is in the blue folder."));
    assert!(
        harness
            .query_by_role_and_label(Role::Link, "blue folder")
            .is_none()
    );
    assert!(!shows(&harness, t.ask_from_notes()));
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

/// A provider whose first reply comes at once and each later one only
/// once the test lets it go.
struct Held {
    fake: Fake,
    gate: Mutex<std::sync::mpsc::Receiver<()>>,
    first: std::sync::atomic::AtomicBool,
}

impl Transport for Held {
    fn post(&self, request: &Request) -> Result<Reply, String> {
        if self.first.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return self.fake.post(request);
        }
        self.gate
            .lock()
            .unwrap()
            .recv()
            .map_err(|e| e.to_string())?;
        self.fake.post(request)
    }
}

#[test]
fn while_it_asks_the_status_line_says_whom_and_how_many_notes_were_read_with_stop() -> TestResult {
    let (store, passport) = store()?;
    let t = L10n::new("en");
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn(&format!("[Here](oldbrainfix://note/{passport})")),
    ]);
    let (open, gate) = std::sync::mpsc::channel();
    let mut app = App::new(store, "en");
    app.set_ask_transport(Arc::new(Held {
        fake,
        gate: Mutex::new(gate),
        first: std::sync::atomic::AtomicBool::new(true),
    }));
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
    assert!(shows(&harness, &t.ask_asking("Anthropic")));
    // Still asking, the spinner keeps the window drawing: stepped.
    for _ in 0..300 {
        harness.step();
        if harness.state().ask_panel().read == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    harness.run_steps(2);
    assert!(shows(&harness, &t.ask_asking_reading("Anthropic", 1)));
    assert!(labelled(&harness, "Where is my passport?"));
    assert!(
        harness
            .query_by_role_and_label(Role::Button, t.ask_stop())
            .is_some()
    );
    open.send(())?;
    until(&mut harness, |app| !app.ask_panel().exchanges.is_empty());
    assert!(!shows(&harness, &t.ask_asking("Anthropic")));
    assert!(!shows(&harness, &t.ask_asking_reading("Anthropic", 1)));
    Ok(())
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

// ------------------------------------------------------------ the thread

/// The "Edit" beside `question` in the thread (#315).
fn edit_of<'a>(harness: &'a Harness<'_, App>, question: &str) -> egui_kittest::Node<'a> {
    let row = harness.get_all_by_label(question).next().unwrap().rect();
    harness
        .get_all_by_role_and_label(Role::Button, L10n::new("en").edit())
        .find(|edit| {
            let y = edit.rect().center().y;
            row.min.y <= y && y <= row.max.y
        })
        .expect("no Edit beside the question")
}

/// A reply that reads the passport's Note, then one that answers `text`.
fn read_then(passport: &str, text: &str) -> [Value; 2] {
    [
        tool_use("read_note", json!({"id": passport})),
        end_turn(text),
    ]
}

#[test]
fn the_questions_stand_as_a_thread_the_oldest_first_above_the_field() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(
        [
            read_then(&passport, "In the blue folder."),
            read_then(&passport, "In the drawer of the desk."),
        ]
        .concat(),
    );
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    ask(&mut harness, "And which drawer?");
    let panel = harness.state().ask_panel();
    let questions: Vec<&str> = panel
        .exchanges
        .iter()
        .map(|a| a.question.as_str())
        .collect();
    assert_eq!(questions, ["Where is my passport?", "And which drawer?"]);
    // The follow-up is asked after the first exchange, as ADR 0051 has it.
    assert!(fake.sent().contains("In the blue folder."));
    let top = |label: &str| harness.get_all_by_label(label).next().unwrap().rect().min.y;
    let field_top = field(&harness, L10n::new("en").ask_hint()).rect().min.y;
    assert!(top("Where is my passport?") < top("In the blue folder."));
    assert!(top("In the blue folder.") < top("And which drawer?"));
    assert!(top("In the drawer of the desk.") < field_top);
    Ok(())
}

#[test]
fn edit_puts_a_question_into_the_field_to_ask_again_and_the_thread_keeps_it() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(
        [
            read_then(&passport, "In the blue folder."),
            read_then(&passport, "In the drawer of the desk."),
            read_then(&passport, "Your passport is in the blue folder."),
        ]
        .concat(),
    );
    let hint = L10n::new("en").ask_hint();
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    ask(&mut harness, "And which drawer?");
    edit_of(&harness, "Where is my passport?").click();
    harness.run();
    assert_eq!(
        harness.state().ask_panel().question,
        "Where is my passport?"
    );
    assert!(field(&harness, hint).accesskit_node().is_focused());
    // The cursor at its end: what is typed goes after it.
    field(&harness, hint).type_text(" At home");
    harness.run();
    assert_eq!(
        harness.state().ask_panel().question,
        "Where is my passport? At home"
    );
    harness.key_press(Key::Enter);
    harness.run();
    let questions: Vec<String> = harness
        .state()
        .ask_panel()
        .exchanges
        .iter()
        .map(|a| a.question.clone())
        .collect();
    assert_eq!(
        questions,
        [
            "Where is my passport?",
            "And which drawer?",
            "Where is my passport? At home"
        ]
    );
    assert!(harness.state().ask_panel().question.is_empty());
    Ok(())
}

#[test]
fn up_in_the_empty_field_fetches_the_last_question() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(
        [
            read_then(&passport, "In the blue folder."),
            read_then(&passport, "In the drawer of the desk."),
        ]
        .concat(),
    );
    let hint = L10n::new("en").ask_hint();
    let mut harness = harness(store, &fake);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::K);
    harness.run();
    // Nothing asked yet: Up fetches nothing.
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert!(harness.state().ask_panel().question.is_empty());
    ask(&mut harness, "Where is my passport?");
    ask(&mut harness, "And which drawer?");
    assert!(field(&harness, hint).accesskit_node().is_focused());
    // With words in the field, Up leaves them.
    field(&harness, hint).type_text("Wh");
    harness.run();
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().ask_panel().question, "Wh");
    // egui's own Up took the cursor to the start.
    harness.key_press(Key::End);
    harness.key_press(Key::Backspace);
    harness.key_press(Key::Backspace);
    harness.run();
    assert!(harness.state().ask_panel().question.is_empty());
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().ask_panel().question, "And which drawer?");
    // The cursor at its end.
    field(&harness, hint).type_text(" Left?");
    harness.run();
    assert_eq!(
        harness.state().ask_panel().question,
        "And which drawer? Left?"
    );
    Ok(())
}

#[test]
fn while_a_question_is_asked_edit_waits() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(
        [
            read_then(&passport, "In the blue folder."),
            read_then(&passport, "In the drawer of the desk."),
        ]
        .concat(),
    );
    let mut app = App::new(store, "en");
    app.set_ask_transport(Arc::new(Slow(fake)));
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
    until(&mut harness, |app| !app.ask_panel().exchanges.is_empty());
    assert!(
        !edit_of(&harness, "Where is my passport?")
            .accesskit_node()
            .is_disabled()
    );
    ask_off_thread(&mut harness, "And which drawer?");
    assert!(harness.state().ask_panel().asking.is_some());
    let edit = edit_of(&harness, "Where is my passport?");
    assert!(edit.accesskit_node().is_disabled());
    edit.click();
    harness.step();
    assert!(harness.state().ask_panel().question.is_empty());
    until(&mut harness, |app| app.ask_panel().exchanges.len() == 2);
    assert!(
        !edit_of(&harness, "Where is my passport?")
            .accesskit_node()
            .is_disabled()
    );
    Ok(())
}

#[test]
fn a_question_asked_scrolls_the_thread_to_it() -> TestResult {
    let (store, passport) = store()?;
    let long = "The passport is in the blue folder. ".repeat(40);
    let fake = Fake::new(
        (0..4)
            .flat_map(|_| read_then(&passport, &long))
            .collect::<Vec<_>>(),
    );
    let mut app = App::new(store, "en");
    app.set_ask_transport(Arc::new(fake.clone()));
    let height = 600.0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, height))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    for question in ["First?", "Second?", "Third?", "Fourth?"] {
        ask(&mut harness, question);
    }
    harness.run_steps(4);
    let top = |label: &str| harness.get_all_by_label(label).next().unwrap().rect().min.y;
    let field_top = field(&harness, L10n::new("en").ask_hint()).rect().min.y;
    // The newest in view above the field, the oldest scrolled away.
    let newest = top("Fourth?");
    assert!(0.0 < newest && newest < field_top, "{newest} {field_top}");
    assert!(top("First?") < 0.0, "{}", top("First?"));
    Ok(())
}

/// What went to the fake clipboard: each text with its HTML.
type Copied = Rc<RefCell<Vec<(String, Option<String>)>>>;

struct FakeClipboard(Copied);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn put_text(&mut self, text: &str, html: Option<&str>) -> bool {
        self.0
            .borrow_mut()
            .push((text.to_string(), html.map(str::to_string)));
        true
    }
}

/// Clicks the button `label` of the answer, the first in the thread.
fn press(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .next()
        .unwrap()
        .click();
    harness.run();
}

#[test]
fn copy_as_text_puts_the_answer_with_its_links_on_the_clipboard_and_html_beside() -> TestResult {
    let (store, passport) = store()?;
    let answer = format!("In the **blue folder**, see [Passport](oldbrainfix://note/{passport}).");
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn(&answer),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    let copied = Copied::default();
    harness
        .state_mut()
        .set_clipboard(Box::new(FakeClipboard(copied.clone())));
    ask(&mut harness, "Where is my passport?");
    press(&mut harness, t.copy_text());
    let copied = copied.borrow();
    let [(text, html)] = copied.as_slice() else {
        panic!("copied once: {copied:?}");
    };
    // The answer alone, its Markdown as written, its app link kept.
    assert_eq!(text, &answer);
    // Beside it the HTML, the app link only its label (ADR 0046).
    let html = html.as_deref().ok_or("no HTML")?;
    assert!(html.contains("<strong>blue folder</strong>"), "{html}");
    assert!(html.contains("Passport"), "{html}");
    assert!(!html.contains("oldbrainfix://"), "{html}");
    // Copied says nothing, and keeps nothing.
    assert_eq!(harness.state().notice(), None);
    assert!(!shows(&harness, t.ask_kept()));
    Ok(())
}

#[test]
fn the_answers_lesser_actions_are_text_buttons_in_the_accent() -> TestResult {
    let (store, passport) = store()?;
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn("In the blue folder."),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    let accent = harness.ctx.global_style().visuals.hyperlink_color;
    let mut words = Vec::new();
    for egui::epaint::ClippedShape { shape, .. } in &harness.output().shapes {
        flat(shape, &mut words);
    }
    // Inside the card, each in the accent's text colour with no frame
    // (obf-1103), as the design system's text button.
    for label in [
        t.copy_text(),
        t.ask_keep(),
        t.add_to_note(),
        t.save_as_markdown(),
    ] {
        assert!(
            words
                .iter()
                .any(|(text, colour)| text == label && *colour == accent),
            "{label} in the accent"
        );
    }
    Ok(())
}

/// Every text drawn, out of its lists, with its colour.
fn flat(shape: &egui::Shape, into: &mut Vec<(String, egui::Color32)>) {
    match shape {
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| flat(s, into)),
        egui::Shape::Text(text) => into.push((text.galley.text().to_string(), text.fallback_color)),
        _ => {}
    }
}

#[test]
fn save_as_markdown_writes_the_answer_into_the_file_named_after_the_question() -> TestResult {
    let (store, passport) = store()?;
    let answer = format!("In the [blue folder](oldbrainfix://note/{passport}).");
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn(&answer),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    let dir = tempfile::tempdir()?;
    let at = dir.path().to_path_buf();
    let proposed = Rc::new(RefCell::new(Vec::new()));
    let named = proposed.clone();
    harness
        .state_mut()
        .set_markdown_saver(Box::new(move |_, name| {
            named.borrow_mut().push(name.to_string());
            Some(at.join(name))
        }));
    ask(&mut harness, "Where is my passport?");
    press(&mut harness, t.save_as_markdown());
    assert_eq!(*proposed.borrow(), ["Where is my passport.md"]);
    let saved = std::fs::read_to_string(dir.path().join("Where is my passport.md"))?;
    assert_eq!(saved.trim_end(), answer);
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn add_to_note_adds_the_question_and_answer_to_the_note_picked_as_a_section() -> TestResult {
    let (store, passport) = store()?;
    let travel = Places::new(&store).add_area("Travel")?;
    let papers = Notes::new(&store).capture("Travel papers", Some(&travel.id), None)?;
    let answer = format!("In the [blue folder](oldbrainfix://note/{passport}).");
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": passport})),
        end_turn(&answer),
    ]);
    let t = L10n::new("en");
    let mut harness = harness(store, &fake);
    ask(&mut harness, "Where is my passport?");
    press(&mut harness, t.add_to_note());
    // The picker of Add to Note… (#308) offers the Notes; one is picked.
    harness
        .query_all_by_label("Travel papers")
        .next_back()
        .ok_or("Travel papers not offered")?
        .click();
    harness.run();
    let store = harness.state().store();
    let notes = Notes::new(store);
    let sections = notes.sections(&papers.id)?;
    let [section] = sections.as_slice() else {
        panic!("one Section: {sections:?}");
    };
    // What Keep as Note keeps, as Markdown, in the Note picked.
    assert_eq!(section.text, format!("Where is my passport?\n\n{answer}"));
    assert_eq!(section.text_format, "markdown");
    let note = notes.by_id(&papers.id)?.ok_or("the Note")?;
    assert_eq!(note.place.as_deref(), Some(travel.id.as_str()));
    assert_eq!(note.text, "Travel papers");
    // Nothing waits in Unsorted, and nothing was kept as a Note.
    let left: Vec<_> = notes
        .all()?
        .into_iter()
        .filter(|n| n.text.starts_with("Where is my passport?"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    // The pick is kept as added to last, as from Capture.
    assert_eq!(
        DeviceSettings::new(store).recently_added_to()?.first(),
        Some(&papers.id)
    );
    // The panel stays, names the Note and opens it.
    assert_eq!(harness.state().ask_panel().exchanges.len(), 1);
    let added = t.ask_added_to("Travel papers");
    assert!(shows(&harness, &added));
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, t.ask_keep())
            .next()
            .is_some()
    );
    harness.get_all_by_label(&added).next().unwrap().click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(papers.id.as_str()));
    Ok(())
}
