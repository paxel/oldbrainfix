use super::*;
use crate::notes::Notes;
use crate::places::Places;
use crate::store::tests::store;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn utc(t: DateTime<Utc>) -> chrono::NaiveDateTime {
    t.naive_utc()
}

/// A provider that answers with `replies` in turn and keeps every
/// request it was sent.
struct Fake {
    replies: Mutex<Vec<Reply>>,
    sent: Mutex<Vec<Request>>,
}

impl Fake {
    fn new(replies: Vec<Value>) -> Fake {
        let replies = replies
            .into_iter()
            .rev()
            .map(|body| Reply {
                status: 200,
                body: body.to_string(),
            })
            .collect();
        Fake {
            replies: Mutex::new(replies),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn failing(status: u16, body: &str) -> Fake {
        Fake {
            replies: Mutex::new(vec![Reply {
                status,
                body: body.into(),
            }]),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn bodies(&self) -> Vec<Value> {
        self.sent
            .lock()
            .iter()
            .filter_map(|r| serde_json::from_str(&r.body).ok())
            .collect()
    }
}

impl Transport for Fake {
    fn post(&self, request: &Request) -> std::result::Result<Reply, String> {
        self.sent.lock().push(request.clone());
        self.replies.lock().pop().ok_or_else(|| "no reply".into())
    }
}

fn anthropic() -> Provider {
    Provider {
        protocol: Protocol::Anthropic,
        url: ANTHROPIC_URL.into(),
        model: ANTHROPIC_MODEL.into(),
        key: "sk-test".into(),
        thinks: true,
    }
}

fn openai(key: &str) -> Provider {
    Provider {
        protocol: Protocol::OpenAi,
        url: "http://localhost:11434/v1/".into(),
        model: "qwen3".into(),
        key: key.into(),
        thinks: false,
    }
}

/// Anthropic calling one tool, with a thinking block before it.
fn tool_use(name: &str, input: Value) -> Value {
    json!({
        "content": [
            {"signature": "sig", "thinking": "Looking.", "type": "thinking"},
            {"id": format!("call-{name}"), "input": input, "name": name, "type": "tool_use"},
        ],
        "stop_reason": "tool_use",
    })
}

fn end_turn(text: &str) -> Value {
    json!({"content": [{"text": text, "type": "text"}], "stop_reason": "end_turn"})
}

fn ask(store: &Store, fake: &Fake, provider: Provider, question: &str) -> Result<Asked> {
    let mut conversation = Conversation::new(provider, &[], question, "2026-01-01 10:00");
    store
        .ask(&mut conversation, fake, &utc)
        .map_err(|e| crate::Error::Invalid(e.to_string()))
}

#[test]
fn anthropic_is_asked_with_adaptive_thinking_and_the_stable_prefix_cached() -> TestResult {
    let request = Conversation::new(anthropic(), &[], "Where is the passport?", "now").request();
    assert_eq!(request.url, "https://api.anthropic.com/v1/messages");
    let header = |name: &str| {
        request
            .headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(header("x-api-key").as_deref(), Some("sk-test"));
    assert_eq!(header("anthropic-version").as_deref(), Some("2023-06-01"));
    let body: Value = serde_json::from_str(&request.body)?;
    assert_eq!(body["model"], "claude-opus-5-5");
    assert_eq!(body["thinking"], json!({"type": "adaptive"}));
    assert_eq!(
        body["system"][0]["cache_control"],
        json!({"type": "ephemeral"})
    );
    assert_eq!(body["system"][0]["text"], SYSTEM);
    for left_out in ["temperature", "top_p", "top_k", "tool_choice"] {
        assert!(body.get(left_out).is_none(), "{left_out}");
    }
    assert!(!request.body.contains("budget_tokens"));
    let names: Vec<&str> = body["tools"]
        .as_array()
        .ok_or("tools")?
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(names.len(), crate::ai_tools::ASK_TOOLS.len());
    assert!(body["tools"][0].get("input_schema").is_some());
    assert!(!names.contains(&"list_notes") && !names.contains(&"read_photo"));
    // The time is the user's, not the system prompt's.
    assert_eq!(
        body["messages"][0]["content"],
        "Now on this computer: now\n\nWhere is the passport?"
    );
    Ok(())
}

#[test]
fn an_openai_compatible_provider_gets_function_tools_and_a_key_only_when_given() -> TestResult {
    let request = Conversation::new(openai(""), &[], "Q", "now").request();
    assert_eq!(request.url, "http://localhost:11434/v1/chat/completions");
    assert!(!request.headers.iter().any(|(n, _)| n == "authorization"));
    let body: Value = serde_json::from_str(&request.body)?;
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["tools"][0]["type"], "function");
    assert!(body["tools"][0]["function"].get("parameters").is_some());
    assert!(body.get("tool_choice").is_none() && body.get("temperature").is_none());

    let keyed = Conversation::new(openai("sk-or"), &[], "Q", "now").request();
    assert!(
        keyed
            .headers
            .contains(&("authorization".into(), "Bearer sk-or".into()))
    );
    Ok(())
}

#[test]
fn the_tools_are_answered_until_the_model_answers_with_links_to_its_notes() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Passport in the blue folder", None, None)?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        tool_use("read_note", json!({"id": note.id})),
        end_turn(&format!(
            "In the [blue folder](oldbrainfix://note/{}).",
            note.id
        )),
    ]);
    let asked = ask(&store, &fake, anthropic(), "Where is my passport?")?;
    assert_eq!(asked.answer.notes, std::slice::from_ref(&note.id));
    assert_eq!(asked.answer.read, std::slice::from_ref(&note.id));
    assert!(!asked.answer.cut);
    // Its one link names the Note read: the answer stays as it came.
    assert!(!asked.answer.unchecked);
    assert_eq!(
        asked.answer.text,
        format!("In the [blue folder](oldbrainfix://note/{}).", note.id)
    );
    assert_eq!(asked.task, None);
    let bodies = fake.bodies();
    assert_eq!(bodies.len(), 3);
    // The thinking block went back as it came, the tool's output
    // beside its call.
    let second = &bodies[1]["messages"];
    assert_eq!(second[1]["content"][0]["signature"], "sig");
    assert_eq!(second[2]["content"][0]["tool_use_id"], "call-find_notes");
    assert!(
        second[2]["content"][0]["content"]
            .as_str()
            .is_some_and(|t| t.contains("blue folder"))
    );
    Ok(())
}

#[test]
fn asking_opens_no_ai_task() -> TestResult {
    let store = store();
    let bike = Notes::new(&store).capture("Bike lock code 4711", None, None)?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["bike"]})),
        tool_use("list_new", json!({"since": "2026-01-01"})),
        tool_use("list_places", json!({})),
        tool_use("read_note", json!({"id": bike.id})),
        end_turn("4711."),
    ]);
    ask(&store, &fake, anthropic(), "Bike lock?")?;
    assert_eq!(store.ai_task()?, None);
    Ok(())
}

#[test]
fn a_change_the_model_makes_waits_as_an_ai_task() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Dentist on Friday", None, None)?;
    let area = Places::new(&store).add_area("Health")?;
    let fake = Fake::new(vec![
        tool_use("file_note", json!({"id": note.id, "place": area.id})),
        end_turn("Filed into Health; it waits for your review."),
    ]);
    let asked = ask(&store, &fake, anthropic(), "Sort what's new")?;
    let task = asked.task.ok_or("an AI Task")?;
    assert_eq!(store.ai_task()?, Some(task.clone()));
    assert!(!store.ai_review(&task)?.is_empty());
    // In the log the Note is still Unsorted until Accept.
    let logged = store.log_only(|| Notes::new(&store).by_id(&note.id))?;
    assert_eq!(logged.ok_or("the Note")?.place, None);
    Ok(())
}

#[test]
fn an_ai_task_left_without_a_change_is_declined() -> TestResult {
    let store = store();
    let fake = Fake::new(vec![
        tool_use(
            "file_note",
            json!({"id": "note:none", "place": "area:none"}),
        ),
        end_turn("Could not file it."),
    ]);
    let asked = ask(&store, &fake, anthropic(), "File it")?;
    assert_eq!(asked.task, None);
    assert_eq!(store.ai_task()?, None);
    Ok(())
}

#[test]
fn a_note_hidden_from_ai_is_in_no_request() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let hidden = notes.capture("Diary: the kidney results", None, None)?;
    let area = Places::new(&store).add_area("Private")?;
    let in_area = notes.capture("Kidney appointment letter", Some(&area.id), None)?;
    notes.capture("Kidney beans on the shopping list", None, None)?;
    store.set_hidden_from_ai(&hidden.id, true)?;
    store.set_hidden_from_ai(&area.id, true)?;
    let fake = Fake::new(vec![
        tool_use(
            "find_notes",
            json!({"words": ["kidney", "diary", "letter"]}),
        ),
        tool_use("list_new", json!({"since": "2000-01-01"})),
        tool_use("read_note", json!({"id": hidden.id})),
        tool_use("read_note", json!({"id": in_area.id})),
        tool_use("list_places", json!({})),
        end_turn("Only beans."),
        // Told to read the Notes shown, it answers again.
        end_turn("Only beans."),
    ]);
    ask(&store, &fake, anthropic(), "What about my kidney?")?;
    let sent: String = fake.sent.lock().iter().map(|r| r.body.clone()).collect();
    assert!(sent.contains("Kidney beans"));
    for secret in ["kidney results", "appointment letter", "Private"] {
        assert!(!sent.contains(secret), "{secret} was sent");
    }
    Ok(())
}

#[test]
fn the_stop_reasons_end_the_question_as_they_say() -> TestResult {
    let store = store();
    let refused = Fake::new(vec![json!({"content": [], "stop_reason": "refusal"})]);
    let mut c = Conversation::new(anthropic(), &[], "Q", "now");
    assert_eq!(store.ask(&mut c, &refused, &utc), Err(AskError::Refused));

    let cut = Fake::new(vec![json!({
        "content": [{"text": "The first half", "type": "text"}],
        "stop_reason": "max_tokens",
    })]);
    let asked = ask(&store, &cut, anthropic(), "Q")?;
    assert!(asked.answer.cut);
    assert_eq!(asked.answer.text, "The first half");

    let paused = Fake::new(vec![
        json!({"content": [{"text": "Still", "type": "text"}], "stop_reason": "pause_turn"}),
        tool_use("list_tags", json!({})),
        end_turn("Done."),
    ]);
    assert_eq!(ask(&store, &paused, anthropic(), "Q")?.answer.text, "Done.");
    // The paused turn went back as it was.
    assert_eq!(
        paused.bodies()[1]["messages"][1]["content"][0]["text"],
        "Still"
    );

    let looping = Fake::new(
        (0..MAX_ROUNDS + 1)
            .map(|_| tool_use("list_tags", json!({})))
            .collect(),
    );
    let mut c = Conversation::new(anthropic(), &[], "Q", "now");
    assert_eq!(
        store.ask(&mut c, &looping, &utc),
        Err(AskError::TooManyRounds)
    );
    Ok(())
}

#[test]
fn an_error_reply_names_the_providers_message() {
    let store = store();
    let bad_key = Fake::failing(
        401,
        r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
    );
    let mut c = Conversation::new(anthropic(), &[], "Q", "now");
    assert_eq!(
        store.ask(&mut c, &bad_key, &utc),
        Err(AskError::Key("invalid x-api-key".into()))
    );
    let no_model = Fake::failing(404, r#"{"error":{"message":"model not found"}}"#);
    let mut c = Conversation::new(openai(""), &[], "Q", "now");
    assert_eq!(
        store.ask(&mut c, &no_model, &utc),
        Err(AskError::Provider(404, "model not found".into()))
    );
}

#[test]
fn an_openai_compatible_model_calls_tools_and_hears_of_arguments_it_got_wrong() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Wifi password on the router", None, None)?;
    let calls = |id: &str, name: &str, args: &str| {
        json!({"choices": [{"finish_reason": "tool_calls", "message": {
            "content": null,
            "role": "assistant",
            "tool_calls": [{"function": {"arguments": args, "name": name}, "id": id, "type": "function"}],
        }}]})
    };
    let fake = Fake::new(vec![
        calls("c1", "find_notes", "{words: wifi"),
        calls("c2", "find_notes", r#"{"words": ["wifi"]}"#),
        calls("c3", "read_note", &json!({"id": note.id}).to_string()),
        json!({"choices": [{"finish_reason": "stop", "message": {
            "content": format!("[On the router](oldbrainfix://note/{})", note.id),
            "role": "assistant",
        }}]}),
    ]);
    let asked = ask(&store, &fake, openai(""), "Wifi?")?;
    assert_eq!(asked.answer.notes, [note.id]);
    let bodies = fake.bodies();
    let second = &bodies[1]["messages"];
    assert_eq!(second[3]["role"], "tool");
    assert_eq!(second[3]["tool_call_id"], "c1");
    assert!(
        second[3]["content"]
            .as_str()
            .is_some_and(|t| t.starts_with("the arguments are no JSON object"))
    );
    let third = &bodies[2]["messages"];
    assert!(
        third[5]["content"]
            .as_str()
            .is_some_and(|t| t.contains("router"))
    );
    Ok(())
}

#[test]
fn a_follow_up_is_asked_after_the_earlier_exchanges() -> TestResult {
    let earlier = [Exchange {
        question: "Where is the passport?".into(),
        answer: "In the blue folder.".into(),
    }];
    let request = Conversation::new(anthropic(), &earlier, "And the ID card?", "now").request();
    let body: Value = serde_json::from_str(&request.body)?;
    let messages = body["messages"].as_array().ok_or("messages")?;
    // One user turn: the earlier exchange as a block for context, then
    // the question (#315).
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(
        messages[0]["content"],
        "Now on this computer: now\n\nEarlier in this session, for context only:\nQ: Where is the passport?\nA: In the blue folder.\n\nMy question now: And the ID card?"
    );
    // Two earlier exchanges, one line after the other, the oldest first.
    let two = [
        Exchange {
            question: "Where is the passport?".into(),
            answer: "In the blue folder.".into(),
        },
        Exchange {
            question: "And the ID card?".into(),
            answer: "In the drawer.".into(),
        },
    ];
    let request = Conversation::new(openai(""), &two, "And the keys?", "now").request();
    let body: Value = serde_json::from_str(&request.body)?;
    let messages = body["messages"].as_array().ok_or("messages")?;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(
        messages[1],
        json!({"role": "user", "content": "Now on this computer: now\n\nEarlier in this session, for context only:\nQ: Where is the passport?\nA: In the blue folder.\nQ: And the ID card?\nA: In the drawer.\n\nMy question now: And the keys?"})
    );
    // Without earlier exchanges the turn is the question alone.
    let request = Conversation::new(anthropic(), &[], "And the keys?", "now").request();
    let body: Value = serde_json::from_str(&request.body)?;
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": "Now on this computer: now\n\nAnd the keys?"}])
    );
    Ok(())
}

#[test]
fn the_provider_is_a_local_setting_that_goes_nowhere() -> TestResult {
    let store = store();
    assert_eq!(store.ask_provider()?.model, ANTHROPIC_MODEL);
    assert_eq!(store.ask_provider()?.unready(), Some(Unready::NoKey));
    store.set_ask_key("sk-ant-secret-1")?;
    assert_eq!(store.ask_provider()?.unready(), None);
    store.set_ask_protocol(Protocol::OpenAi)?;
    assert_eq!(store.ask_provider()?.url, OPENAI_URL);
    assert_eq!(store.ask_provider()?.unready(), Some(Unready::NoModel));
    store.set_ask_model("llama3.2")?;
    store.set_ask_key("sk-or-secret-2")?;
    assert_eq!(store.ask_provider()?.unready(), None);
    // Each protocol keeps its own.
    store.set_ask_protocol(Protocol::Anthropic)?;
    assert_eq!(store.ask_provider()?.key, "sk-ant-secret-1");
    Notes::new(&store).capture("A Note", None, None)?;
    for entry in store.all_entries()? {
        let row = format!("{} {} {:?}", entry.entity, entry.field, entry.value);
        assert!(!row.contains("secret") && !row.contains("ask:"), "{row}");
    }
    let settings = store.ai_tool("t", "read_settings", &json!({}), &utc)?;
    assert!(!format!("{settings:?}").contains("secret"));
    Ok(())
}

#[test]
fn the_ai_is_off_until_switched_on_and_off_keeps_the_provider() -> TestResult {
    let store = store();
    // A provider set up before the switch came: off all the same.
    store.set_ask_key("sk-ant-kept")?;
    assert!(!store.ai_on()?);
    let provider = store.ask_provider()?;
    assert_eq!(provider.unready(), None);
    assert_eq!(store.ask_unready(&provider)?, Some(Unready::AiOff));
    store.set_ai_on(true)?;
    assert!(store.ai_on()?);
    assert_eq!(store.ask_unready(&provider)?, None);
    store.set_ai_on(false)?;
    assert_eq!(store.ask_unready(&provider)?, Some(Unready::AiOff));
    assert_eq!(store.ask_provider()?.key, "sk-ant-kept");
    // A local setting: in no entry, so never synced.
    store.set_ai_on(true)?;
    for entry in store.all_entries()? {
        assert!(!format!("{entry:?}").contains("ask:on"));
    }
    // The AI port only reads it: no tool but read_settings names it.
    for tool in crate::ai_tools::tools() {
        let named = tool.to_string().contains("aiOn");
        assert_eq!(named, tool["name"] == "read_settings", "{}", tool["name"]);
    }
    Ok(())
}

#[test]
fn plain_http_goes_only_to_this_computer_and_the_home_network() {
    for allowed in [
        "https://api.deepseek.com",
        "https://openrouter.ai/api/v1",
        "http://localhost:11434/v1",
        "http://127.0.0.1:1234/v1",
        "http://[::1]:8080/v1",
        "http://192.168.1.20:11434/v1",
        "http://10.0.0.5/v1",
        "http://nas.local:11434/v1",
    ] {
        assert!(url_allowed(allowed), "{allowed}");
    }
    for refused in [
        "http://api.example.com/v1",
        "http://8.8.8.8/v1",
        "ftp://localhost/v1",
        "localhost:11434",
        "https://",
        "",
    ] {
        assert!(!url_allowed(refused), "{refused}");
    }
}

#[test]
fn the_web_posts_the_request_and_reads_an_error_reply_with_its_body() -> TestResult {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = std::thread::spawn(move || -> std::io::Result<String> {
        let (stream, _) = listener.accept()?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = n.trim().parse().unwrap_or(0);
            }
            if line == "\r\n" || line.is_empty() {
                break;
            }
            head.push_str(&line);
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body)?;
        let reply = r#"{"error":{"message":"overloaded"}}"#;
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 529 Overloaded\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
            reply.len()
        )?;
        Ok(format!("{head}\n{}", String::from_utf8_lossy(&body)))
    });
    let request = Request {
        url: format!("http://127.0.0.1:{port}/v1/messages"),
        headers: vec![
            ("content-type".into(), "application/json".into()),
            ("x-api-key".into(), "sk-test".into()),
        ],
        body: r#"{"model":"m"}"#.into(),
    };
    let reply = Web::new().post(&request)?;
    assert_eq!(reply.status, 529);
    assert_eq!(error_message(&reply.body), "overloaded");
    let seen = server.join().map_err(|_| "server")??;
    assert!(seen.starts_with("POST /v1/messages"));
    assert!(seen.to_ascii_lowercase().contains("x-api-key: sk-test"));
    assert!(seen.ends_with(r#"{"model":"m"}"#));
    Ok(())
}

#[test]
fn the_system_prompt_answers_only_from_the_notes_read_with_a_link_for_each() {
    for rule in [
        "Call find_notes first, always, before you answer",
        "Call read_note with the id of every Note you want to use",
        "only read_note counts",
        "A Note an earlier answer named counts only once you read it again in this question",
        "only from the Notes you read with read_note in this question",
        "never from general knowledge",
        "Link the Note of every statement",
        "The Areas, Topics, tags, Routines and check-offs the other tools list count too",
        "oldbrainfix://note/",
        "A link to a Note you did not read with read_note is removed",
        "Only when you searched and found nothing about it",
        "\"That's not in your Notes\"",
        "translated into the language of the question",
        "linking only the Notes you read",
        "Example:",
    ] {
        assert!(SYSTEM.contains(rule), "{rule}");
    }
}

#[test]
fn a_link_to_a_note_not_read_in_this_question_is_removed_and_the_answer_marked() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let passport = notes.capture("Passport in the blue folder", None, None)?;
    let old = notes.capture("Passport photo booth at the station", None, None)?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        tool_use("read_note", json!({"id": passport.id})),
        end_turn(&format!(
            "In the [blue folder](oldbrainfix://note/{}), photos from the [station](oldbrainfix://note/{}).",
            passport.id, old.id
        )),
    ]);
    let asked = ask(&store, &fake, anthropic(), "Where is my passport?")?;
    // The Note only found keeps its words, without its link; the one
    // read keeps its link.
    assert_eq!(
        asked.answer.text,
        format!(
            "In the [blue folder](oldbrainfix://note/{}), photos from the station.",
            passport.id
        )
    );
    assert_eq!(asked.answer.notes, std::slice::from_ref(&passport.id));
    assert!(asked.answer.unchecked);
    Ok(())
}

#[test]
fn a_note_read_in_an_earlier_question_is_not_read_in_this_one() -> TestResult {
    let store = store();
    let passport = Notes::new(&store).capture("Passport in the blue folder", None, None)?;
    let link = format!("[blue folder](oldbrainfix://note/{})", passport.id);
    let earlier = [Exchange {
        question: "Where is my passport?".into(),
        answer: format!("In the {link}."),
    }];
    // Told to search first, it answers the same again.
    let fake = Fake::new(vec![
        end_turn(&format!("Still in the {link}.")),
        end_turn(&format!("Still in the {link}.")),
    ]);
    let mut conversation =
        Conversation::new(anthropic(), &earlier, "Still there?", "2026-01-01 10:00");
    let answer = store
        .ask(&mut conversation, &fake, &utc)
        .map_err(|e| e.to_string())?
        .answer;
    assert_eq!(answer.text, "Still in the blue folder.");
    assert!(answer.notes.is_empty() && answer.unchecked);
    Ok(())
}

#[test]
fn a_link_to_a_note_that_does_not_exist_is_removed_and_the_answer_marked() -> TestResult {
    let store = store();
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": "note:0a1b2c3d"})),
        end_turn("On [Friday](oldbrainfix://note/note:0a1b2c3d) at 9."),
    ]);
    let asked = ask(&store, &fake, anthropic(), "When is the dentist?")?;
    assert!(
        asked.answer.read.is_empty(),
        "a read that failed reads none"
    );
    assert_eq!(asked.answer.text, "On Friday at 9.");
    assert!(asked.answer.notes.is_empty());
    assert!(asked.answer.unchecked);
    Ok(())
}

#[test]
fn an_answer_without_links_stays_as_it_came_and_unmarked() -> TestResult {
    let store = store();
    Notes::new(&store).capture("Bike lock code 4711", None, None)?;
    let text = "That's not in your Notes. See [the guide](oldbrainfix://guide/ask) or [the web](https://example.com).";
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["tax"]})),
        end_turn(text),
    ]);
    let asked = ask(&store, &fake, anthropic(), "When is my tax return due?")?;
    assert_eq!(asked.answer.text, text);
    assert!(asked.answer.notes.is_empty());
    assert!(!asked.answer.unchecked);
    Ok(())
}

#[test]
fn a_link_to_a_note_hidden_from_ai_is_removed_even_when_the_model_names_it() -> TestResult {
    let store = store();
    let hidden = Notes::new(&store).capture("Diary: the kidney results", None, None)?;
    store.set_hidden_from_ai(&hidden.id, true)?;
    // The model knows the id from elsewhere and asks every tool that
    // takes a Note for it: none hands it out.
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"id": hidden.id})),
        tool_use("read_check_offs", json!({"note": hidden.id})),
        tool_use(
            "read_attachment_text",
            json!({"note": hidden.id, "file": "abc"}),
        ),
        end_turn(&format!(
            "See your [diary](oldbrainfix://note/{}).",
            hidden.id
        )),
    ]);
    let asked = ask(&store, &fake, anthropic(), "What about my kidney?")?;
    assert!(asked.answer.read.is_empty(), "{:?}", asked.answer.read);
    assert_eq!(asked.answer.text, "See your diary.");
    assert!(asked.answer.notes.is_empty());
    assert!(asked.answer.unchecked);
    Ok(())
}

#[test]
fn every_note_link_is_checked_wherever_it_stands() {
    let read = vec!["note:a".to_string()];
    let (text, unchecked) = checked(
        "**[A](OLDBRAINFIX://note/note:b)**, [a](oldbrainfix://note/note:a)",
        &read,
    );
    assert_eq!(text, "**A**, [a](oldbrainfix://note/note:a)");
    assert!(unchecked);
    // A link that shows once the one around it is gone goes too.
    let (text, unchecked) = checked(
        "[[x](oldbrainfix://note/note:c)](oldbrainfix://note/note:d)",
        &read,
    );
    assert_eq!(text, "x");
    assert!(unchecked);
    // The id as the link says it once its escapes are read.
    let (text, unchecked) = checked(
        "[a](oldbrainfix://note/note%3Aa) [c](oldbrainfix://note/note%3Ac)",
        &read,
    );
    assert_eq!(text, "[a](oldbrainfix://note/note%3Aa) c");
    assert!(unchecked);
    // In a table's cell, and beside an Area's link, which stays.
    let table = "| What | Where |\n| --- | --- |\n| Key | [hook](oldbrainfix://note/note:e) |\n\n[Home](oldbrainfix://area/area:h)";
    let (text, unchecked) = checked(table, &read);
    assert_eq!(
        text,
        "| What | Where |\n| --- | --- |\n| Key | hook |\n\n[Home](oldbrainfix://area/area:h)"
    );
    assert!(unchecked);
    for text in [
        "Plain, `[a](oldbrainfix://note/note:b)` in code, and oldbrainfix://note/note:b bare.",
        "[a](oldbrainfix://note/note:a)",
        "",
    ] {
        assert_eq!(checked(text, &read), (text.to_string(), false), "{text}");
    }
    // What is left links only Notes read.
    let (text, _) = checked(
        "[a](oldbrainfix://note/note:a) [b](oldbrainfix://note/note:b) [[c](oldbrainfix://note/note:a)](oldbrainfix://note/note:z)",
        &read,
    );
    assert!(linked_notes(&text).iter().all(|id| read.contains(id)));
    assert_eq!(linked_notes(&text), ["note:a"]);
}

#[test]
fn the_links_of_an_answer_name_its_notes_once_in_order() {
    let text = "See [a](oldbrainfix://note/note:a), **[b](oldbrainfix://note/note:b)** and [a again](oldbrainfix://note/note:a), not [web](https://example.com).";
    assert_eq!(linked_notes(text), ["note:a", "note:b"]);
}

#[test]
fn a_model_that_answers_before_any_tool_is_told_once_to_search_first() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Wifi password: kiwi-42", None, None)?;
    let fake = Fake::new(vec![
        end_turn("That's not in your Notes"),
        tool_use("find_notes", json!({"words": ["wifi"]})),
        tool_use("read_note", json!({"id": note.id})),
        end_turn("kiwi-42."),
    ]);
    let asked = ask(&store, &fake, anthropic(), "Wifi?")?;
    assert_eq!(asked.answer.text, "kiwi-42.");
    assert_eq!(asked.answer.read, std::slice::from_ref(&note.id));
    let bodies = fake.bodies();
    assert_eq!(bodies.len(), 4);
    // The first answer stays in the conversation, the word after it.
    let second = bodies[1]["messages"].as_array().ok_or("messages")?;
    assert_eq!(second.len(), 3);
    // It names the question asked (#315).
    assert_eq!(
        second[2],
        json!({"role": "user", "content": search_first("Wifi?")})
    );
    assert!(search_first("Wifi?").contains("\"Wifi?\""));

    // Told once only: the next answer stands, however it reads.
    let stubborn = Fake::new(vec![
        end_turn("That's not in your Notes"),
        end_turn("Still not."),
    ]);
    let asked = ask(&store, &stubborn, anthropic(), "Wifi?")?;
    assert_eq!(asked.answer.text, "Still not.");
    assert_eq!(stubborn.bodies().len(), 2);
    Ok(())
}

#[test]
fn a_follow_up_told_to_search_hears_its_own_question_not_an_earlier_one() -> TestResult {
    // With an earlier exchange before it, a small model answered the
    // next question from it, and searched for its words (#315).
    let store = store();
    let note = Notes::new(&store).capture("Wifi password: kiwi-42", None, None)?;
    let fake = Fake::new(vec![
        end_turn("That's not in your Notes"),
        tool_use("find_notes", json!({"words": ["wifi"]})),
        tool_use("read_note", json!({"id": note.id})),
        end_turn("kiwi-42."),
    ]);
    let earlier = [Exchange {
        question: "Where is my passport?".into(),
        answer: "In the blue folder.".into(),
    }];
    let mut conversation = Conversation::new(
        anthropic(),
        &earlier,
        "What is the wifi password?",
        "2026-01-01 10:00",
    );
    let asked = store
        .ask(&mut conversation, &fake, &utc)
        .map_err(|e| crate::Error::Invalid(e.to_string()))?;
    assert_eq!(asked.answer.read, std::slice::from_ref(&note.id));
    let bodies = fake.bodies();
    let second = bodies[1]["messages"].as_array().ok_or("messages")?;
    let told = second.last().ok_or("a message")?;
    assert_eq!(told["role"], "user");
    let words = told["content"].as_str().ok_or("words")?;
    assert!(words.contains("\"What is the wifi password?\""), "{words}");
    assert!(!words.contains("passport"), "{words}");
    // The earlier exchange goes before the question, in its turn.
    assert_eq!(second[0]["role"], "user");
    assert_eq!(
        second[0]["content"],
        "Now on this computer: 2026-01-01 10:00\n\nEarlier in this session, for context only:\nQ: Where is my passport?\nA: In the blue folder.\n\nMy question now: What is the wifi password?"
    );
    Ok(())
}

#[test]
fn a_read_note_that_names_its_note_as_note_reads_it_by_its_id() -> TestResult {
    // Told to read, a small model passed the id as `note`, the key of
    // read_attachment_text, and the read failed (#315).
    let store = store();
    let note = Notes::new(&store).capture("Wifi password: kiwi-42", None, None)?;
    let fake = Fake::new(vec![
        tool_use("read_note", json!({"note": note.id})),
        end_turn("kiwi-42."),
    ]);
    let asked = ask(&store, &fake, anthropic(), "What is the wifi password?")?;
    assert_eq!(asked.answer.read, std::slice::from_ref(&note.id));
    let call = |args: Value| {
        json!({"choices": [{"finish_reason": "tool_calls", "message": {
            "content": null,
            "role": "assistant",
            "tool_calls": [{"function": {"arguments": args.to_string(), "name": "read_note"}, "id": "c1", "type": "function"}],
        }}]})
    };
    let say = json!({"choices": [{"finish_reason": "stop", "message": {
        "content": "kiwi-42.", "role": "assistant",
    }}]});
    let fake = Fake::new(vec![call(json!({"note": note.id})), say.clone()]);
    let asked = ask(&store, &fake, openai(""), "What is the wifi password?")?;
    assert_eq!(asked.answer.read, std::slice::from_ref(&note.id));
    // Not a Note's id: nothing read. Never over an `id` given.
    let fake = Fake::new(vec![call(json!({"note": "kiwi"})), say.clone()]);
    let asked = ask(&store, &fake, openai(""), "What is the wifi password?")?;
    assert!(asked.answer.read.is_empty());
    let fake = Fake::new(vec![
        call(json!({"id": "note:0a1b2c3d", "note": note.id})),
        say,
    ]);
    let asked = ask(&store, &fake, openai(""), "What is the wifi password?")?;
    assert!(asked.answer.read.is_empty());
    Ok(())
}

#[test]
fn an_answer_without_words_is_sent_back_once_more_with_what_fits() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Wifi password: kiwi-42", None, None)?;
    let say = |text: &str| {
        json!({"choices": [{"finish_reason": "stop", "message": {
            "content": text, "role": "assistant",
        }}]})
    };
    let call = |id: &str, name: &str, args: Value| {
        json!({"choices": [{"finish_reason": "tool_calls", "message": {
            "content": null,
            "role": "assistant",
            "tool_calls": [{"function": {"arguments": args.to_string(), "name": name}, "id": id, "type": "function"}],
        }}]})
    };
    // Shown the Note, told to read it, it says nothing: told again.
    let fake = Fake::new(vec![
        call("c1", "find_notes", json!({"words": ["wifi"]})),
        say("kiwi"),
        say(""),
        call("c2", "read_note", json!({"id": note.id})),
        say("kiwi-42."),
    ]);
    let asked = ask(&store, &fake, openai(""), "What is the wifi password?")?;
    assert_eq!(asked.answer.text, "kiwi-42.");
    let bodies = fake.bodies();
    assert_eq!(bodies.len(), 5);
    let told = |at: usize| -> std::result::Result<String, Box<dyn std::error::Error>> {
        let messages = bodies[at]["messages"].as_array().ok_or("messages")?;
        let last = messages.last().ok_or("a message")?;
        Ok(last["content"].as_str().ok_or("words")?.to_string())
    };
    assert_eq!(told(2)?, read_first(std::slice::from_ref(&note.id)));
    assert_eq!(told(3)?, read_first(std::slice::from_ref(&note.id)));

    // Read, then nothing said: told to answer, once only.
    let quiet = Fake::new(vec![
        call("c1", "read_note", json!({"id": note.id})),
        say(" "),
        say(""),
    ]);
    let asked = ask(&store, &quiet, openai(""), "What is the wifi password?")?;
    assert!(asked.answer.text.trim().is_empty());
    let bodies = quiet.bodies();
    assert_eq!(bodies.len(), 3);
    let messages = bodies[2]["messages"].as_array().ok_or("messages")?;
    let last = messages.last().ok_or("a message")?;
    assert_eq!(last["content"], answer_now("What is the wifi password?"));
    assert!(answer_now("Wifi?").contains("\"Wifi?\""));
    Ok(())
}

#[test]
fn a_model_that_answers_from_an_excerpt_is_told_once_to_read_the_notes_shown() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Passport in the blue folder", None, None)?;
    let say = |text: &str| {
        json!({"choices": [{"finish_reason": "stop", "message": {
            "content": text, "role": "assistant",
        }}]})
    };
    let call = |id: &str, name: &str, args: Value| {
        json!({"choices": [{"finish_reason": "tool_calls", "message": {
            "content": null,
            "role": "assistant",
            "tool_calls": [{"function": {"arguments": args.to_string(), "name": name}, "id": id, "type": "function"}],
        }}]})
    };
    let fake = Fake::new(vec![
        call("c1", "find_notes", json!({"words": ["passport"]})),
        say("In the blue folder."),
        call("c2", "read_note", json!({"id": note.id})),
        say("In the blue folder."),
    ]);
    let asked = ask(&store, &fake, openai(""), "Where is my passport?")?;
    // Read, not linked: it shows as "Also read".
    assert_eq!(asked.answer.read, std::slice::from_ref(&note.id));
    assert!(asked.answer.notes.is_empty());
    assert_eq!(asked.answer.also_read(), std::slice::from_ref(&note.id));
    let bodies = fake.bodies();
    assert_eq!(bodies.len(), 4);
    let told = &bodies[2]["messages"].as_array().ok_or("messages")?;
    let last = told.last().ok_or("a message")?;
    assert_eq!(last["role"], "user");
    let words = last["content"].as_str().ok_or("words")?;
    // The call to make, as the tool takes it.
    assert!(
        words.contains(&json!({"id": note.id}).to_string()),
        "{words}"
    );
    assert_eq!(words, read_first(std::slice::from_ref(&note.id)));

    // An answer after a search that showed nothing stands at once.
    let none = Fake::new(vec![
        call("c1", "find_notes", json!({"words": ["tax"]})),
        say("That's not in your Notes."),
    ]);
    let asked = ask(&store, &none, openai(""), "Tax?")?;
    assert_eq!(asked.answer.text, "That's not in your Notes.");
    assert_eq!(none.bodies().len(), 2);
    Ok(())
}

#[test]
fn also_read_are_the_notes_read_the_answer_does_not_link_in_the_order_read() {
    let ids = |all: &[&str]| all.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let answer = Answer {
        text: "See [B](oldbrainfix://note/note:b).".into(),
        notes: ids(&["note:b"]),
        read: ids(&["note:c", "note:b", "note:a"]),
        cut: false,
        unchecked: false,
    };
    assert_eq!(answer.also_read(), ids(&["note:c", "note:a"]));
    assert_eq!(also_read(&ids(&["note:a"]), &ids(&["note:a"])), ids(&[]));
    assert_eq!(
        also_read(&[], &ids(&["note:a", "note:a"])),
        ids(&["note:a"])
    );
}

/// A provider that answers with `replies`, each a status and a body, in
/// turn.
fn replying(replies: Vec<(u16, Value)>) -> Fake {
    Fake {
        replies: Mutex::new(
            replies
                .into_iter()
                .rev()
                .map(|(status, body)| Reply {
                    status,
                    body: body.to_string(),
                })
                .collect(),
        ),
        sent: Mutex::new(Vec::new()),
    }
}

fn openai_said(content: &str) -> Value {
    json!({"choices": [{"finish_reason": "stop", "message": {
        "content": content,
        "role": "assistant",
    }}]})
}

#[test]
fn an_openai_compatible_model_is_asked_not_to_think_until_let_think() -> TestResult {
    let store = store();
    store.set_ask_protocol(Protocol::OpenAi)?;
    store.set_ask_model("qwen3:4b")?;
    // Off by default (#318): the request asks the model not to think, in
    // the field Ollama's /v1 reads as think false.
    assert!(!store.ask_thinks()?);
    let body = |store: &Store| -> std::result::Result<Value, Box<dyn std::error::Error>> {
        let request = Conversation::new(store.ask_provider()?, &[], "Q", "now").request();
        Ok(serde_json::from_str(&request.body)?)
    };
    assert_eq!(body(&store)?["reasoning_effort"], "none");
    assert!(body(&store)?.get("reasoning").is_none());
    assert!(body(&store)?.get("think").is_none());
    // "Let the model think" on: nothing is sent, the model thinks as it
    // would; an effort such as "low" fails on a model that cannot think.
    store.set_ask_thinks(true)?;
    assert!(store.ask_thinks()?);
    assert!(body(&store)?.get("reasoning_effort").is_none());
    store.set_ask_thinks(false)?;
    assert_eq!(body(&store)?["reasoning_effort"], "none");
    // Anthropic is asked as before, thinking adaptively either way.
    store.set_ask_protocol(Protocol::Anthropic)?;
    for thinks in [false, true] {
        store.set_ask_thinks(thinks)?;
        let anthropic = body(&store)?;
        assert!(anthropic.get("reasoning_effort").is_none());
        assert_eq!(anthropic["thinking"], json!({"type": "adaptive"}));
    }
    // A local setting like the provider's: in no entry, so never synced.
    for entry in store.all_entries()? {
        let row = format!("{} {} {:?}", entry.entity, entry.field, entry.value);
        assert!(!row.contains("ask:"), "{row}");
    }
    Ok(())
}

#[test]
fn a_provider_that_refuses_the_no_think_field_is_asked_again_without_it() -> TestResult {
    let store = store();
    let refused = json!({"error": {
        "message": "Unsupported parameter: 'reasoning_effort' is not supported with this model.",
        "param": "reasoning_effort",
    }});
    // The answer without a search is sent back once (#314), and stays
    // asked without the field.
    let fake = replying(vec![
        (400, refused.clone()),
        (200, openai_said("Hello.")),
        (200, openai_said("Hello.")),
    ]);
    let asked = ask(&store, &fake, openai(""), "Hi?")?;
    assert_eq!(asked.answer.text, "Hello.");
    let bodies = fake.bodies();
    assert_eq!(bodies.len(), 3);
    assert_eq!(bodies[0]["reasoning_effort"], "none");
    assert!(bodies[1].get("reasoning_effort").is_none());
    assert!(bodies[2].get("reasoning_effort").is_none());
    assert_eq!(bodies[0]["messages"], bodies[1]["messages"]);

    // Only once: a second refusal is the provider's error.
    let fake = replying(vec![(400, refused.clone()), (400, refused.clone())]);
    let failed = ask(&store, &fake, openai(""), "Hi?");
    assert!(failed.is_err());
    assert_eq!(fake.bodies().len(), 2);

    // Nothing to leave out while the model may think: not asked again.
    let fake = replying(vec![(400, refused), (200, openai_said("Hello."))]);
    let thinking = Provider {
        thinks: true,
        ..openai("")
    };
    assert!(ask(&store, &fake, thinking, "Hi?").is_err());
    assert_eq!(fake.bodies().len(), 1);
    Ok(())
}

#[test]
fn thinking_a_model_wrote_into_its_reply_is_left_out() -> TestResult {
    // A model that thinks whatever it is asked, such as Qwen3-4B-Thinking
    // (Ollama's qwen3:4b), writes its thinking into the content once
    // asked not to think, ending it with </think> (#318).
    let store = store();
    let note = Notes::new(&store).capture("Passport in the blue folder", None, None)?;
    let fake = Fake::new(vec![
        json!({"choices": [{"finish_reason": "tool_calls", "message": {
            "content": "Okay, the user asks where the passport is.\n</think>\n\n",
            "role": "assistant",
            "tool_calls": [{"function": {"arguments": json!({"id": note.id}).to_string(), "name": "read_note"}, "id": "c1", "type": "function"}],
        }}]}),
        openai_said(&format!(
            "I should link it.\n</think>\n\nIn the blue folder, see [Passport](oldbrainfix://note/{}).",
            note.id
        )),
    ]);
    let asked = ask(&store, &fake, openai(""), "Where is the passport?")?;
    assert_eq!(
        asked.answer.text,
        format!(
            "In the blue folder, see [Passport](oldbrainfix://note/{}).",
            note.id
        )
    );
    assert_eq!(asked.answer.notes, [note.id]);
    // The thinking does not go back to the model either.
    let second = fake.bodies()[1]["messages"].to_string();
    assert!(!second.contains("Okay, the user") && !second.contains("</think>"));
    Ok(())
}
