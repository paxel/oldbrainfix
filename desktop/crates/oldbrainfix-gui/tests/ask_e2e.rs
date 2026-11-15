//! Ask end to end against a real local model ([obf-1024], #331): the
//! questions go through the real code path, [`Store::ask`] with the real
//! [`Web`] transport, to an Ollama the script `tool/ask-e2e.sh` starts in
//! a container, on seeded Notes. The desktop's path asks the store as the
//! Ask panel does; the phone's path asks over the paired host's TLS
//! connection, answered by the [`AskDesk`] beside the syncs as the app
//! runs it, a phone of the user's own proving itself with its key as the
//! Dart core's `askDesktop` does. The panel itself is drawn with a real
//! answer into PNG files for the report.
//!
//! Every test is ignored: neither `cargo test` nor CI runs them. The
//! script does, one at a time:
//!
//! ```sh
//! tool/ask-e2e.sh run
//! ```
//!
//! What is asserted does not depend on the model's wording: an answer
//! arrives within the time limit, it has words, the Notes read include the
//! seeded Note that holds the answer, a question the Notes hold no answer
//! to ends with "That's not in your Notes" or links no Note, and a failure
//! comes back as its kind with a message. Each test prints the question,
//! the answer, the Notes read and the timings of every round first, so a
//! red test still says what happened.
//!
//! [obf-1024]: https://github.com/paxel/oldbrainfix/issues/331

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::ask::{
    AskError, Asked, Conversation, Protocol, Reply, Request, Transport, Web,
};
use oldbrainfix_core::lan::{
    DEVICE_HEADER, NONCE_HEADER, PROOF_HEADER, paired_proof_bytes, sync_request,
};
use oldbrainfix_core::lan_ask::{
    ASK_PATH, AskDesk, AskFailure, AskReply, PROVIDER_PATH, ProviderShown, Question,
};
use oldbrainfix_core::{Notes, PairedDevice, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::auto_sync::{PairedHosting, Syncing};
use oldbrainfix_gui::l10n::L10n;
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ------------------------------------------------------------ settings

fn setting(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn seconds(key: &str, default: u64) -> Duration {
    Duration::from_secs(
        setting(key, &default.to_string())
            .parse()
            .unwrap_or(default),
    )
}

/// The OpenAI-compatible base URL of the Ollama the script started.
fn url() -> String {
    setting("ASK_E2E_URL", "http://127.0.0.1:11435/v1")
}

/// The model asked.
fn model() -> String {
    setting("ASK_E2E_MODEL", "qwen2.5:3b")
}

/// How long one question may take to count as answered in time.
fn limit() -> Duration {
    seconds("ASK_E2E_LIMIT_SECS", 120)
}

/// How long a test waits for an answer at all before it gives up.
fn hard_cap() -> Duration {
    seconds("ASK_E2E_HARD_SECS", 900)
}

/// Where the panel's PNG files go: never into the repo.
fn shots_dir() -> PathBuf {
    let dir = std::env::var_os("ASK_E2E_SHOTS").map_or_else(
        || std::env::temp_dir().join("oldbrainfix-ask-e2e"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An address on this computer where nothing listens, as an Ollama that
/// is stopped.
fn stopped_url() -> String {
    let port = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap();
    format!("http://127.0.0.1:{port}/v1")
}

/// A model name Ollama does not have.
const NO_MODEL: &str = "no-such-model-obf:1b";

// ---------------------------------------------------- the seeded Notes

/// The Notes seeded on the desktop: the answers among things that are
/// not.
const PASSPORT: &str = "Passport is in the blue folder in the desk drawer";
const WIFI: &str = "Wifi password at home: kiwi-sunset-42";
const TYRES: &str = "Winterreifen stehen im Keller, linkes Regal";
const OTHERS: [&str; 7] = [
    "Dentist Dr. Weber on Tuesday 3 December at 10:30",
    "Buy oat milk and coffee beans",
    "Call grandma about the birthday cake",
    "Bike lock code 4711",
    "Spare house key is with the neighbour Mrs. Lange",
    "Insurance papers in the green box on the wardrobe",
    "The car goes to the garage for its inspection in March",
];

/// A question the Notes hold no answer to.
const UNKNOWN: &str = "What is my mother's maiden name?";

/// This computer's wall clock, as the app reads it.
fn wall(t: DateTime<Utc>) -> NaiveDateTime {
    t.with_timezone(&chrono::Local).naive_local()
}

/// A store with the seeded Notes and "AI (experimental)" switched on
/// (#334), asking `url` for `model` with the OpenAI-compatible protocol;
/// the Notes' ids by their texts.
fn seeded(
    seed: u8,
    url: &str,
    model: &str,
) -> Result<(Store, HashMap<String, String>), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    let notes = Notes::new(&store);
    let mut ids = HashMap::new();
    for text in [PASSPORT, WIFI, TYRES].into_iter().chain(OTHERS) {
        let note = notes.capture(text, None, None)?;
        ids.insert(text.to_string(), note.id);
    }
    store.set_ai_on(true)?;
    store.set_ask_protocol(Protocol::OpenAi)?;
    store.set_ask_url(url)?;
    store.set_ask_model(model)?;
    Ok((store, ids))
}

// ------------------------------------------------- the recorded rounds

/// One request to the provider and what came back.
#[derive(Debug, Clone)]
struct Round {
    took: Duration,
    /// The HTTP status, or why no reply came.
    status: Result<u16, String>,
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    /// The tools the model called, with their arguments as it sent them.
    calls: Vec<(String, String)>,
    /// The model's words, or the error body, cut short.
    text: String,
    finish: String,
    /// The length of a thinking text the reply carried, if any.
    reasoning: Option<usize>,
}

/// The real network, keeping every round it carried.
#[derive(Clone)]
struct Recorded {
    web: Arc<Web>,
    rounds: Arc<Mutex<Vec<Round>>>,
}

impl Recorded {
    fn new() -> Recorded {
        Recorded {
            web: Arc::new(Web::new()),
            rounds: Arc::default(),
        }
    }

    fn rounds(&self) -> Vec<Round> {
        self.rounds.lock().unwrap().clone()
    }

    /// The ids the model asked `read_note` for, in order, each once.
    fn read_asked(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for round in self.rounds() {
            for (name, args) in round.calls {
                let id = serde_json::from_str::<Value>(&args)
                    .ok()
                    .and_then(|a| a.get("id").and_then(Value::as_str).map(str::to_string));
                if name == "read_note"
                    && let Some(id) = id
                    && !out.contains(&id)
                {
                    out.push(id);
                }
            }
        }
        out
    }
}

impl Transport for Recorded {
    fn post(&self, request: &Request) -> Result<Reply, String> {
        let started = Instant::now();
        let reply = self.web.post(request);
        let took = started.elapsed();
        let round = match &reply {
            Ok(reply) => round_of(took, reply),
            Err(why) => Round {
                took,
                status: Err(why.clone()),
                prompt_tokens: None,
                completion_tokens: None,
                calls: Vec::new(),
                text: String::new(),
                finish: String::new(),
                reasoning: None,
            },
        };
        self.rounds.lock().unwrap().push(round);
        reply
    }
}

fn round_of(took: Duration, reply: &Reply) -> Round {
    let body: Value = serde_json::from_str(&reply.body).unwrap_or(Value::Null);
    let message = body.pointer("/choices/0/message");
    let text = message
        .and_then(|m| m.get("content"))
        .and_then(Value::as_str)
        .map_or_else(|| reply.body.chars().take(300).collect(), |t| t.to_string());
    let calls = message
        .and_then(|m| m.get("tool_calls"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|call| {
            let name = call
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let args = match call.pointer("/function/arguments") {
                Some(Value::String(text)) => text.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            (name, args)
        })
        .collect();
    let reasoning = message
        .and_then(|m| m.get("reasoning").or_else(|| m.get("reasoning_content")))
        .and_then(Value::as_str)
        .map(str::len);
    Round {
        took,
        status: Ok(reply.status),
        prompt_tokens: body.pointer("/usage/prompt_tokens").and_then(Value::as_u64),
        completion_tokens: body
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64),
        calls,
        text,
        finish: body
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        reasoning,
    }
}

/// The Note `id` by its text, for the printout.
fn titled(ids: &HashMap<String, String>, id: &str) -> String {
    ids.iter().find(|(_, v)| v.as_str() == id).map_or_else(
        || format!("{id} (no seeded Note)"),
        |(text, _)| format!("\"{text}\""),
    )
}

fn print_rounds(rounds: &[Round]) {
    for (n, round) in rounds.iter().enumerate() {
        let status = match &round.status {
            Ok(status) => format!("HTTP {status}"),
            Err(why) => format!("no reply: {why}"),
        };
        let tokens = match (round.prompt_tokens, round.completion_tokens) {
            (Some(p), Some(c)) => format!("prompt {p} / out {c} tokens"),
            _ => "no usage".into(),
        };
        let thinking = round
            .reasoning
            .map_or_else(String::new, |n| format!(", thinking {n} chars"));
        println!(
            "  round {}: {:.1}s, {status}, {tokens}, finish {:?}{thinking}",
            n + 1,
            round.took.as_secs_f64(),
            round.finish
        );
        for (name, args) in &round.calls {
            println!("    calls {name} {args}");
        }
        if round.calls.is_empty() && !round.text.is_empty() {
            println!("    says {:?}", round.text);
        }
    }
}

/// Whether `text` is the "That's not in your Notes" kind of reply.
fn says_not_in_notes(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("not in your notes") || lower.contains("nicht in deinen notizen")
}

// ------------------------------------------------------- the desktop

/// `question` asked on `store` as the Ask panel asks it, with no earlier
/// questions, through `transport`: what came back and how long it took.
fn ask_desktop(
    store: Arc<Store>,
    transport: &Recorded,
    question: &str,
) -> (Result<Asked, AskError>, Duration) {
    let (done, arrived) = std::sync::mpsc::channel();
    let transport = transport.clone();
    let question = question.to_string();
    let started = Instant::now();
    std::thread::spawn(move || {
        let now = wall(store.now()).format("%Y-%m-%d %H:%M, %A").to_string();
        let mut conversation =
            Conversation::new(store.ask_provider().unwrap(), &[], &question, &now);
        let asked = store.ask(&mut conversation, &transport, &wall);
        drop(done.send(asked));
    });
    match arrived.recv_timeout(hard_cap()) {
        Ok(asked) => (asked, started.elapsed()),
        Err(_) => panic!("no answer within {:?}", hard_cap()),
    }
}

/// Asks `question` on the desktop and checks that the answer comes from
/// the seeded Note `expected` in time.
fn desktop_answers(question: &str, expected: &str) -> TestResult {
    let (store, ids) = seeded(1, &url(), &model())?;
    let store = Arc::new(store);
    let transport = Recorded::new();
    let (asked, took) = ask_desktop(store, &transport, question);
    println!("=== desktop, {} at {}: {question}", model(), url());
    print_rounds(&transport.rounds());
    let asked = match asked {
        Ok(asked) => asked,
        Err(e) => panic!("no answer after {took:?}: {e:?}"),
    };
    let answer = &asked.answer;
    println!(
        "  answer after {:.1}s: {:?}",
        took.as_secs_f64(),
        answer.text
    );
    println!(
        "  links: {:?}",
        answer
            .notes
            .iter()
            .map(|id| titled(&ids, id))
            .collect::<Vec<_>>()
    );
    println!(
        "  read: {:?}",
        answer
            .read
            .iter()
            .map(|id| titled(&ids, id))
            .collect::<Vec<_>>()
    );
    println!(
        "  cut {}, link removed {}, AI Task {:?}",
        answer.cut, answer.unchecked, asked.task
    );
    assert!(!answer.text.trim().is_empty(), "an empty answer");
    let id = &ids[expected];
    assert!(
        answer.read.contains(id),
        "the Note {expected:?} was not read"
    );
    assert!(took <= limit(), "took {took:?}, over {:?}", limit());
    Ok(())
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_desktop_answers_where_the_passport_is_from_its_note() -> TestResult {
    desktop_answers("Where is my passport?", PASSPORT)
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_desktop_answers_the_wifi_password_from_its_note() -> TestResult {
    desktop_answers("What is the wifi password at home?", WIFI)
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_desktop_answers_a_german_question_from_its_note() -> TestResult {
    desktop_answers("Wo sind die Winterreifen?", TYRES)
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_desktop_says_what_is_not_in_the_notes() -> TestResult {
    let (store, ids) = seeded(1, &url(), &model())?;
    let transport = Recorded::new();
    let (asked, took) = ask_desktop(Arc::new(store), &transport, UNKNOWN);
    println!("=== desktop, {} at {}: {UNKNOWN}", model(), url());
    print_rounds(&transport.rounds());
    let asked = match asked {
        Ok(asked) => asked,
        Err(e) => panic!("no answer after {took:?}: {e:?}"),
    };
    let answer = &asked.answer;
    println!(
        "  answer after {:.1}s: {:?}",
        took.as_secs_f64(),
        answer.text
    );
    println!(
        "  links: {:?}",
        answer
            .notes
            .iter()
            .map(|id| titled(&ids, id))
            .collect::<Vec<_>>()
    );
    println!(
        "  read: {:?}",
        answer
            .read
            .iter()
            .map(|id| titled(&ids, id))
            .collect::<Vec<_>>()
    );
    assert!(!answer.text.trim().is_empty(), "an empty answer");
    assert!(
        answer.notes.is_empty() || says_not_in_notes(&answer.text),
        "an answer from the Notes to a question they hold nothing about"
    );
    assert!(took <= limit(), "took {took:?}, over {:?}", limit());
    Ok(())
}

/// The desktop asking `url` for `model`, expected to fail: the failure.
fn desktop_fails(url: &str, model: &str) -> AskError {
    let (store, _) = seeded(1, url, model).unwrap();
    let transport = Recorded::new();
    let (asked, took) = ask_desktop(Arc::new(store), &transport, "Where is my passport?");
    println!("=== desktop, {model} at {url}: Where is my passport?");
    print_rounds(&transport.rounds());
    match asked {
        Ok(asked) => panic!("answered after {took:?}: {:?}", asked.answer.text),
        Err(e) => {
            println!("  failure after {:.1}s: {e:?}", took.as_secs_f64());
            println!("  as said: {e}");
            e
        }
    }
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_desktop_says_ollama_gave_no_reply_when_it_is_stopped() {
    match desktop_fails(&stopped_url(), &model()) {
        AskError::Network(message) => assert!(!message.trim().is_empty()),
        other => panic!("not a network failure: {other:?}"),
    }
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_desktop_says_the_provider_failed_for_a_model_it_lacks() {
    match desktop_fails(&url(), NO_MODEL) {
        AskError::Provider(status, message) => {
            assert!((400..600).contains(&status), "status {status}");
            assert!(!message.trim().is_empty());
        }
        other => panic!("not a provider failure: {other:?}"),
    }
}

// --------------------------------------------------------- the phone

/// The phone's request to `path` on the paired host at `port` of the
/// desktop `desk_id`, proven with a fresh nonce: its status and body.
/// Timed out as the Dart core's `askDesktop` waits: 20 seconds to
/// connect, ten minutes for the answer.
fn phone_request(
    port: u16,
    desk_id: &str,
    phone: &Store,
    path: &str,
    body: Option<&Value>,
) -> Result<(u16, String), String> {
    let fail = |e: &dyn std::fmt::Display| e.to_string();
    let config = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .disable_verification(true)
                .build(),
        )
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_global(Some(Duration::from_secs(600)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let base = format!("https://127.0.0.1:{port}");
    let mut challenge = agent
        .get(format!("{base}/challenge"))
        .call()
        .map_err(|e| fail(&e))?;
    let nonce: Value = serde_json::from_str(
        &challenge
            .body_mut()
            .read_to_string()
            .map_err(|e| fail(&e))?,
    )
    .map_err(|e| fail(&e))?;
    let nonce = nonce["nonce"].as_str().unwrap_or_default().to_string();
    let proof = phone
        .signing_key()
        .map_err(|e| fail(&e))?
        .sign(&paired_proof_bytes(desk_id, &nonce));
    let device = phone.device_id().map_err(|e| fail(&e))?;
    let mut res = match body {
        Some(body) => agent
            .post(format!("{base}{path}"))
            .header(DEVICE_HEADER, &device)
            .header(NONCE_HEADER, &nonce)
            .header(PROOF_HEADER, &proof)
            .content_type("application/json")
            .send(body.to_string().as_bytes()),
        None => agent
            .get(format!("{base}{path}"))
            .header(DEVICE_HEADER, &device)
            .header(NONCE_HEADER, &nonce)
            .header(PROOF_HEADER, &proof)
            .call(),
    }
    .map_err(|e| fail(&e))?;
    let text = res.body_mut().read_to_string().map_err(|e| fail(&e))?;
    Ok((res.status().as_u16(), text))
}

/// What the phone heard: the provider shown, and the reply to its
/// question with its raw body and how long it took.
struct PhoneAsked {
    provider: (u16, String),
    status: u16,
    body: String,
    took: Duration,
}

/// The desktop seeded with the Notes asking `url` for `model`, hosting
/// for a phone of the user's own paired with it, which asks `question`
/// after it learnt the provider; the desktop's rounds are recorded by
/// `transport`.
fn ask_phone(
    url: &str,
    model: &str,
    question: &str,
    transport: &Recorded,
) -> Result<(PhoneAsked, HashMap<String, String>), Box<dyn std::error::Error>> {
    let (desk, ids) = seeded(1, url, model)?;
    let desk = Arc::new(desk);
    let phone = Store::in_memory(Box::new(Utc::now), || Ok(SigningKey::from_seed([2; 32])))?;
    // Paired once by code: the phone's key verified, the phone remembered.
    let first = sync_request(&phone, &HashMap::new(), "Pixel", "", "")?;
    desk.serve_sync(first.to_string().as_bytes(), "Desk")?;
    desk.remember_device(&PairedDevice {
        id: phone.device_id()?,
        name: "Pixel".into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: desk.now(),
        shared: false,
    })?;
    let mut hosting = PairedHosting::new(IpAddr::V4(Ipv4Addr::LOCALHOST), false);
    hosting.set_asking(AskDesk::new(
        desk.clone(),
        Arc::new(transport.clone()),
        Arc::new(wall),
        || {},
    ));
    hosting.check(&desk);
    let port = hosting.port().ok_or("not hosting")?;
    let desk_id = desk.device_id()?;
    let body = Question {
        question: question.into(),
        earlier: Vec::new(),
    }
    .to_json();
    let started = Instant::now();
    let asked = std::thread::spawn(move || {
        let provider = phone_request(port, &desk_id, &phone, PROVIDER_PATH, None)?;
        let started = Instant::now();
        let (status, body) = phone_request(port, &desk_id, &phone, ASK_PATH, Some(&body))?;
        Ok::<_, String>(PhoneAsked {
            provider,
            status,
            body,
            took: started.elapsed(),
        })
    });
    let syncing = Syncing::default();
    while !asked.is_finished() {
        assert!(
            started.elapsed() < hard_cap(),
            "no answer within {:?}",
            hard_cap()
        );
        let sessions = hosting.step(&desk, "Desk", Duration::from_millis(100), &syncing);
        assert!(sessions.is_empty());
    }
    let asked = asked.join().map_err(|_| "the phone panicked")??;
    Ok((asked, ids))
}

/// What the phone heard, the desktop's rounds and the seeded Notes' ids.
type PhoneHeard = (PhoneAsked, AskReply, Recorded, HashMap<String, String>);

/// The phone's answer, printed with what the desktop sent and heard.
fn phone_answer(
    url: &str,
    model: &str,
    question: &str,
) -> Result<PhoneHeard, Box<dyn std::error::Error>> {
    let transport = Recorded::new();
    let (asked, ids) = ask_phone(url, model, question, &transport)?;
    println!("=== phone via the desktop, {model} at {url}: {question}");
    println!(
        "  GET {PROVIDER_PATH}: HTTP {} {}",
        asked.provider.0, asked.provider.1
    );
    print_rounds(&transport.rounds());
    println!(
        "  POST {ASK_PATH} after {:.1}s: HTTP {} {}",
        asked.took.as_secs_f64(),
        asked.status,
        asked.body
    );
    println!(
        "  read_note asked for: {:?}",
        transport
            .read_asked()
            .iter()
            .map(|id| titled(&ids, id))
            .collect::<Vec<_>>()
    );
    let provider = ProviderShown::from_json(&serde_json::from_str(&asked.provider.1)?)
        .ok_or("no provider shown")?;
    assert_eq!(asked.provider.0, 200);
    assert_eq!(provider.protocol, Protocol::OpenAi);
    assert_eq!(provider.model, model);
    assert_eq!(asked.status, 200, "{}", asked.body);
    let reply = AskReply::from_json(&serde_json::from_str(&asked.body)?).ok_or("no reply")?;
    Ok((asked, reply, transport, ids))
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_phone_hears_where_the_passport_is_from_its_note() -> TestResult {
    let (asked, reply, transport, ids) = phone_answer(&url(), &model(), "Where is my passport?")?;
    let answer = match reply.outcome {
        Ok(answer) => answer,
        Err(failure) => panic!("no answer: {failure:?}"),
    };
    println!(
        "  links: {:?}",
        answer
            .notes
            .iter()
            .map(|id| titled(&ids, id))
            .collect::<Vec<_>>()
    );
    assert!(!answer.text.trim().is_empty(), "an empty answer");
    assert!(
        transport.read_asked().contains(&ids[PASSPORT]),
        "the passport's Note was not read"
    );
    assert!(
        asked.took <= limit(),
        "took {:?}, over {:?}",
        asked.took,
        limit()
    );
    Ok(())
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_phone_hears_what_is_not_in_the_notes() -> TestResult {
    let (asked, reply, _, _) = phone_answer(&url(), &model(), UNKNOWN)?;
    let answer = match reply.outcome {
        Ok(answer) => answer,
        Err(failure) => panic!("no answer: {failure:?}"),
    };
    assert!(!answer.text.trim().is_empty(), "an empty answer");
    assert!(
        answer.notes.is_empty() || says_not_in_notes(&answer.text),
        "an answer from the Notes to a question they hold nothing about"
    );
    assert!(
        asked.took <= limit(),
        "took {:?}, over {:?}",
        asked.took,
        limit()
    );
    Ok(())
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_phone_hears_ollama_gave_no_reply_when_it_is_stopped() -> TestResult {
    let (_, reply, _, _) = phone_answer(&stopped_url(), &model(), "Where is my passport?")?;
    match reply.outcome {
        Err(AskFailure::Network(message)) => assert!(!message.trim().is_empty()),
        other => panic!("not a network failure: {other:?}"),
    }
    Ok(())
}

#[test]
#[ignore = "needs Ollama with a model: tool/ask-e2e.sh run"]
fn the_phone_hears_the_provider_failed_for_a_model_it_lacks() -> TestResult {
    let (_, reply, _, _) = phone_answer(&url(), NO_MODEL, "Where is my passport?")?;
    match reply.outcome {
        Err(AskFailure::Provider(status, message)) => {
            assert!((400..600).contains(&status), "status {status}");
            assert!(!message.trim().is_empty());
        }
        other => panic!("not a provider failure: {other:?}"),
    }
    Ok(())
}

// ------------------------------------------------- the panel's frames

/// The Ask panel over the seeded Notes asking `url` for `model`, with
/// `question` asked and answered (or failed), drawn into `name` in the
/// shots' folder; a failure also with its announcement opened, into
/// `name` ending in `-details.png`.
fn shoot_panel(url: &str, model: &str, question: &str, name: &str) -> TestResult {
    let (store, ids) = seeded(1, url, model)?;
    let transport = Recorded::new();
    let mut app = App::new(store, "en");
    app.set_wall_clock(Arc::new(wall));
    app.set_ask_transport(Arc::new(transport.clone()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_theme(egui::Theme::Light)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.input_mut().system_theme = Some(egui::Theme::Light);
    harness.run_steps(4);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(question);
    harness.run();
    let started = Instant::now();
    // Without a way to wake the window the panel asks at once, here.
    harness.key_press(egui::Key::Enter);
    harness.run();
    let took = started.elapsed();
    harness.run_steps(4);
    println!("=== panel, {model} at {url}: {question}");
    print_rounds(&transport.rounds());
    match harness.state().ask_panel().exchanges.last() {
        Some(asked) => {
            println!(
                "  answer after {:.1}s: {:?}",
                took.as_secs_f64(),
                asked.answer.text
            );
            println!(
                "  sources shown: {:?}",
                asked.sources.iter().map(|(_, t)| t).collect::<Vec<_>>()
            );
            println!(
                "  read: {:?}",
                asked
                    .answer
                    .read
                    .iter()
                    .map(|id| titled(&ids, id))
                    .collect::<Vec<_>>()
            );
        }
        None => println!(
            "  no answer after {:.1}s; announced at the top",
            took.as_secs_f64()
        ),
    }
    let file = shots_dir().join(name);
    harness.render()?.save(&file)?;
    println!("  drawn: {}", file.display());
    // A failure is announced at the top; its click shows why.
    let failed = L10n::new("en").ask_failed();
    if harness.query_by_label(failed).is_some() {
        harness.get_by_label(failed).click();
        harness.run_steps(4);
        let file = shots_dir().join(name.replace(".png", "-details.png"));
        harness.render()?.save(&file)?;
        println!("  drawn: {}", file.display());
    }
    Ok(())
}

#[test]
#[ignore = "needs Ollama with a model and a GPU or lavapipe: tool/ask-e2e.sh run"]
fn the_panel_is_drawn_with_real_answers_and_a_failure() -> TestResult {
    shoot_panel(
        &url(),
        &model(),
        "Where is my passport?",
        "ask-answered.png",
    )?;
    shoot_panel(&url(), &model(), UNKNOWN, "ask-not-in-notes.png")?;
    shoot_panel(
        &stopped_url(),
        &model(),
        "Where is my passport?",
        "ask-ollama-stopped.png",
    )?;
    Ok(())
}
