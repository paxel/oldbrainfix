use super::*;
use crate::Notes;
use crate::Places;
use crate::lan::{DEVICE_HEADER, NONCE_HEADER, PROOF_HEADER, paired_proof_bytes, sync_request};
use crate::pairing::PairedDevice;
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

const KEY: &str = "sk-ant-secret-4711";

fn utc(t: DateTime<Utc>) -> NaiveDateTime {
    t.naive_utc()
}

/// A provider answering with `replies` in turn, keeping what it was
/// sent; with a gate, each reply waits until the test lets it go.
struct Fake {
    replies: Mutex<Vec<ask::Reply>>,
    sent: Mutex<Vec<String>>,
    gate: Option<Mutex<Receiver<()>>>,
}

impl Fake {
    fn new(replies: Vec<(u16, Value)>) -> Fake {
        Fake {
            replies: Mutex::new(
                replies
                    .into_iter()
                    .rev()
                    .map(|(status, body)| ask::Reply {
                        status,
                        body: body.to_string(),
                    })
                    .collect(),
            ),
            sent: Mutex::new(Vec::new()),
            gate: None,
        }
    }

    fn gated(replies: Vec<(u16, Value)>) -> (Fake, Sender<()>) {
        let (open, gate) = channel();
        let fake = Fake {
            gate: Some(Mutex::new(gate)),
            ..Fake::new(replies)
        };
        (fake, open)
    }

    fn sent(&self) -> String {
        self.sent.lock().concat()
    }
}

impl Transport for Fake {
    fn post(&self, request: &ask::Request) -> std::result::Result<ask::Reply, String> {
        self.sent.lock().push(request.body.clone());
        if let Some(gate) = &self.gate {
            gate.lock().recv().map_err(|e| e.to_string())?;
        }
        self.replies.lock().pop().ok_or_else(|| "no reply".into())
    }
}

fn tool_use(name: &str, input: Value) -> (u16, Value) {
    (
        200,
        json!({
            "content": [{"id": format!("call-{name}"), "input": input, "name": name, "type": "tool_use"}],
            "stop_reason": "tool_use",
        }),
    )
}

fn end_turn(text: &str) -> (u16, Value) {
    (
        200,
        json!({"content": [{"text": text, "type": "text"}], "stop_reason": "end_turn"}),
    )
}

/// The desk with Anthropic and its key set up, and a phone paired
/// with it as one of the user's own Devices, or by invite.
fn paired(shared: bool) -> Result<(Store, Store)> {
    let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
    let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
    let first = sync_request(&phone, &HashMap::new(), "Phone", "", "")?;
    desk.serve_sync(first.to_string().as_bytes(), "Desk")?;
    desk.remember_device(&PairedDevice {
        id: phone.device_id()?,
        name: "Phone".into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: desk.now(),
        shared,
    })?;
    desk.set_ask_key(KEY)?;
    Ok((desk, phone))
}

/// `signer`'s request to `path` on `desk`, with its proof over a
/// nonce the host handed out when `challenged`.
fn asked(
    desk: &Store,
    signer: &Store,
    method: &str,
    path: &str,
    body: &Value,
    challenged: bool,
) -> Result<(Request, Receiver<Response>)> {
    let proof = signer
        .signing_key()?
        .sign(&paired_proof_bytes(&desk.device_id()?, "n1"));
    let headers = vec![
        (DEVICE_HEADER.into(), signer.device_id()?),
        (NONCE_HEADER.into(), "n1".into()),
        (PROOF_HEADER.into(), proof),
    ];
    let body = if method == "POST" {
        body.to_string().into_bytes()
    } else {
        Vec::new()
    };
    Ok(Request::handed_over(
        method, path, headers, body, challenged,
    ))
}

fn question(text: &str) -> Value {
    Question {
        question: text.into(),
        earlier: Vec::new(),
    }
    .to_json()
}

fn reply_of(response: &Response) -> std::result::Result<AskReply, Box<dyn std::error::Error>> {
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body)?;
    Ok(AskReply::from_json(&body).ok_or("a reply")?)
}

#[test]
fn only_the_users_own_paired_device_with_its_proof_may_ask() -> TestResult {
    let fake = Fake::new(vec![]);
    for shared in [false, true] {
        let (desk, phone) = paired(shared)?;
        let stranger = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
        let status = |signer: &Store, path: &str, challenged: bool| -> Result<u16> {
            let (request, _) = asked(&desk, signer, "GET", path, &json!({}), challenged)?;
            Ok(desk.serve_ask(&request, &fake, &utc)?.status)
        };
        let own = if shared { 403 } else { 200 };
        assert_eq!(
            status(&phone, PROVIDER_PATH, true)?,
            own,
            "shared: {shared}"
        );
        // Without a nonce of this host, or from an unpaired Device.
        assert_eq!(status(&phone, PROVIDER_PATH, false)?, 403);
        assert_eq!(status(&stranger, PROVIDER_PATH, true)?, 403);
        // A question the same way.
        let (request, _) = asked(&desk, &stranger, "POST", ASK_PATH, &question("Q"), true)?;
        assert_eq!(desk.serve_ask(&request, &fake, &utc)?.body, b"declined");
        // Neither the PIN host nor a sync answers a question.
        let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &question("Q"), true)?;
        assert_eq!(desk.serve(&request, "Desk")?.0.status, 404);
        assert_eq!(desk.serve_paired(&request, "Desk")?.0.status, 404);
        // Forgotten, it asks no more.
        desk.forget_device(&phone.device_id()?)?;
        assert_eq!(status(&phone, PROVIDER_PATH, true)?, 403);
    }
    assert_eq!(fake.sent(), "", "nothing was asked");
    Ok(())
}

#[test]
fn a_change_drafted_on_the_asking_connection_waits_for_the_windows_review() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("desk.db");
    let open = || Store::open(&path, ticking(), || Ok(SigningKey::from_seed([1; 32])));
    let (window, asking) = (open()?, open()?);
    let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
    let first = sync_request(&phone, &HashMap::new(), "Phone", "", "")?;
    window.serve_sync(first.to_string().as_bytes(), "Desk")?;
    window.remember_device(&PairedDevice {
        id: phone.device_id()?,
        name: "Phone".into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: window.now(),
        shared: false,
    })?;
    window.set_ask_key(KEY)?;
    let note = Notes::new(&window).capture("Dentist on Friday", None, None)?;
    let area = Places::new(&window).add_area("Health")?;
    let fake = Fake::new(vec![
        tool_use("file_note", json!({"id": note.id, "place": area.id})),
        end_turn("Filed into Health; it waits for your review."),
    ]);
    let (request, _) = asked(&asking, &phone, "POST", ASK_PATH, &question("Sort"), true)?;
    assert!(reply_of(&asking.serve_ask(&request, &fake, &utc)?)?.review);
    // The window's own connection sees the review waiting.
    let task = window.ai_task()?.ok_or("the AI Task")?;
    assert!(!window.ai_review(&task)?.is_empty());
    assert!(window.sync_paused()?);
    Ok(())
}

#[test]
fn the_provider_is_named_without_its_key_or_address() -> TestResult {
    let (desk, phone) = paired(false)?;
    let fake = Fake::new(vec![]);
    let shown = |desk: &Store| -> std::result::Result<Value, Box<dyn std::error::Error>> {
        let (request, _) = asked(desk, &phone, "GET", PROVIDER_PATH, &json!({}), true)?;
        let response = desk.serve_ask(&request, &fake, &utc)?;
        let text = String::from_utf8(response.body.clone())?;
        assert!(!text.contains("secret"), "{text}");
        Ok(serde_json::from_str(&text)?)
    };
    assert_eq!(
        shown(&desk)?,
        json!({"host": "api.anthropic.com", "model": "claude-opus-5-5", "protocol": "anthropic", "ready": true})
    );
    desk.set_ask_protocol(Protocol::OpenAi)?;
    desk.set_ask_url("https://me:secret-pass@llm.example.org:8443/v1?key=secret-q")?;
    desk.set_ask_key("secret-or")?;
    assert_eq!(
        shown(&desk)?,
        json!({"host": "llm.example.org:8443", "model": "", "protocol": "openai", "ready": false, "unready": "noModel"})
    );
    Ok(())
}

#[test]
fn the_key_is_in_no_failure_the_phone_hears() -> TestResult {
    let (desk, phone) = paired(false)?;
    let failures = [
        (401, format!("invalid x-api-key: {KEY}")),
        (500, format!("upstream said {KEY}")),
    ];
    for (status, message) in failures {
        let fake = Fake::new(vec![(status, json!({"error": {"message": message}}))]);
        let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &question("Q"), true)?;
        let response = desk.serve_ask(&request, &fake, &utc)?;
        let text = String::from_utf8(response.body.clone())?;
        assert!(!text.contains(KEY), "{text}");
        let reply = reply_of(&response)?;
        let expected = if status == 401 {
            AskFailure::Key("invalid x-api-key: …".into())
        } else {
            AskFailure::Provider(500, "upstream said …".into())
        };
        assert_eq!(reply.outcome, Err(expected));
    }
    // The user and the query of the address go the same way.
    let provider = Provider {
        protocol: Protocol::OpenAi,
        url: "https://me:pw@llm.example.org/v1?token=t0k".into(),
        model: "m".into(),
        key: String::new(),
    };
    assert_eq!(
        AskFailure::of(
            AskError::Network("https://me:pw@llm.example.org/v1?token=t0k: refused".into()),
            &provider
        ),
        AskFailure::Network("https://…@llm.example.org/v1?…: refused".into())
    );
    Ok(())
}

#[test]
fn the_phones_question_is_answered_with_its_notes_and_hidden_ones_stay_out() -> TestResult {
    let (desk, phone) = paired(false)?;
    let notes = Notes::new(&desk);
    let passport = notes.capture("Passport in the blue folder", None, None)?;
    let hidden = notes.capture("Passport scan in the diary", None, None)?;
    desk.set_hidden_from_ai(&hidden.id, true)?;
    let fake = Fake::new(vec![
        tool_use("find_notes", json!({"words": ["passport"]})),
        tool_use("read_note", json!({"id": hidden.id})),
        tool_use("read_note", json!({"id": passport.id})),
        end_turn(&format!(
            "In the [blue folder](oldbrainfix://note/{}), a [scan](oldbrainfix://note/{}).",
            passport.id, hidden.id
        )),
    ]);
    let body = Question {
        question: "And where is it now?".into(),
        earlier: vec![Exchange {
            question: "Do I have a passport?".into(),
            answer: "Yes.".into(),
        }],
    }
    .to_json();
    let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &body, true)?;
    let reply = reply_of(&desk.serve_ask(&request, &fake, &utc)?)?;
    assert_eq!(
        reply,
        AskReply {
            // The hidden Note was never read, so its link is gone, and
            // the phone hears that a source could not be checked.
            outcome: Ok(Answered {
                text: format!(
                    "In the [blue folder](oldbrainfix://note/{}), a scan.",
                    passport.id
                ),
                notes: vec![passport.id.clone()],
                cut: false,
                unchecked: true,
            }),
            review: false,
        }
    );
    let sent = fake.sent();
    assert!(sent.contains("blue folder"));
    assert!(sent.contains("Do I have a passport?"), "asked after it");
    assert!(!sent.contains("diary"), "the hidden Note was sent");
    assert!(!sent.contains(KEY));
    // Nothing that is no question.
    for body in [json!({}), json!({"question": "  "}), json!({"question": 4})] {
        let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &body, true)?;
        assert_eq!(desk.serve_ask(&request, &fake, &utc)?.status, 400);
    }
    Ok(())
}

#[test]
fn a_change_waits_for_the_review_and_the_phone_hears_so_while_sync_waits() -> TestResult {
    let (desk, phone) = paired(false)?;
    let note = Notes::new(&desk).capture("Dentist on Friday", None, None)?;
    let area = Places::new(&desk).add_area("Health")?;
    let fake = Fake::new(vec![
        tool_use("file_note", json!({"id": note.id, "place": area.id})),
        end_turn("Filed into Health; it waits for your review."),
        end_turn("Still waiting."),
    ]);
    let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &question("Sort"), true)?;
    assert!(reply_of(&desk.serve_ask(&request, &fake, &utc)?)?.review);
    assert!(desk.sync_paused()?);
    // Asked again while sync waits: answered, and the review still
    // waits.
    let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &question("And?"), true)?;
    let reply = reply_of(&desk.serve_ask(&request, &fake, &utc)?)?;
    assert!(reply.outcome.is_ok() && reply.review);
    Ok(())
}

#[test]
fn a_desktop_without_a_provider_says_so() -> TestResult {
    let (desk, phone) = paired(false)?;
    desk.set_ask_key("")?;
    let fake = Fake::new(vec![]);
    let (request, _) = asked(&desk, &phone, "POST", ASK_PATH, &question("Q"), true)?;
    let reply = reply_of(&desk.serve_ask(&request, &fake, &utc)?)?;
    assert_eq!(
        reply.outcome,
        Err(AskFailure::Unready(Some(Unready::NoKey)))
    );
    assert_eq!(fake.sent(), "");
    Ok(())
}

#[test]
fn the_failure_kinds_go_both_ways() {
    let failures = [
        AskFailure::Unready(Some(Unready::BadUrl)),
        AskFailure::Unready(None),
        AskFailure::Key("refused".into()),
        AskFailure::Provider(529, "overloaded".into()),
        AskFailure::Network("timed out".into()),
        AskFailure::Unreadable("no content".into()),
        AskFailure::Rounds,
        AskFailure::Refused,
        AskFailure::Stopped,
        AskFailure::Busy,
        AskFailure::Unknown,
    ];
    for failure in failures {
        let reply = AskReply {
            outcome: Err(failure),
            review: true,
        };
        assert_eq!(AskReply::from_json(&reply.to_json()), Some(reply));
    }
    let later = json!({"failure": "quota", "message": "x", "review": false});
    assert_eq!(
        AskReply::from_json(&later).map(|r| r.outcome),
        Some(Err(AskFailure::Unknown))
    );
    assert_eq!(AskReply::from_json(&json!({"what": 1})), None);
}

#[test]
fn a_question_runs_off_the_hosts_thread_one_at_a_time() -> TestResult {
    let (desk, phone) = paired(false)?;
    let desk = Arc::new(desk);
    let (fake, open) = Fake::gated(vec![end_turn("Here.")]);
    let (told, heard) = channel();
    let wall: SharedWall = Arc::new(|t: DateTime<Utc>| t.naive_utc());
    let ask_desk = AskDesk::new(desk.clone(), Arc::new(fake), wall, move || {
        told.send(()).ok();
    });
    let (first, first_answer) = asked(&desk, &phone, "POST", ASK_PATH, &question("Q"), true)?;
    // Taken at once, while the provider still thinks.
    ask_desk.take(first);
    assert!(ask_desk.asking());
    let (second, second_answer) = asked(&desk, &phone, "POST", ASK_PATH, &question("Q2"), true)?;
    ask_desk.take(second);
    let busy = second_answer.recv_timeout(Duration::from_secs(10))?;
    assert_eq!(reply_of(&busy)?.outcome, Err(AskFailure::Busy));
    // The provider query is answered meanwhile.
    let (shown, shown_answer) = asked(&desk, &phone, "GET", PROVIDER_PATH, &json!({}), true)?;
    ask_desk.take(shown);
    assert_eq!(
        shown_answer.recv_timeout(Duration::from_secs(10))?.status,
        200
    );
    open.send(())?;
    let answer = first_answer.recv_timeout(Duration::from_secs(10))?;
    assert_eq!(
        reply_of(&answer)?.outcome.map(|a| a.text),
        Ok("Here.".into())
    );
    heard.recv_timeout(Duration::from_secs(10))?;
    assert!(!ask_desk.asking());
    // An unpaired Device hears no more than a refusal.
    let stranger = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
    let (request, refused) = asked(&desk, &stranger, "POST", ASK_PATH, &question("Q"), true)?;
    ask_desk.take(request);
    assert_eq!(refused.recv_timeout(Duration::from_secs(10))?.status, 403);
    Ok(())
}
