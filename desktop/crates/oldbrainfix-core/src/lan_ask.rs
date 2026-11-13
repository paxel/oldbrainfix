//! The phone's question through the desktop (#267, ADR 0003, ADR 0051):
//! a paired phone of the user asks over the paired LAN connection, and
//! the desktop runs the question as its Ask panel does, with its own
//! provider and key ([`Store::ask`]), and sends back the answer with the
//! Notes it links, or why there is none. The phone holds no provider and
//! no key; neither ever crosses the wire.
//!
//! Two requests on the paired host, each with the paired Device's proof
//! over a nonce as a sync's (ADR 0003): `GET /ask/provider` names the
//! provider the desktop uses (its protocol, model and host, never its
//! key nor the whole address), which the phone shows before its first
//! question (App Store guideline 5.1.2(i)); `POST /ask` carries the
//! question and the earlier exchanges of the same conversation as text.
//! Only the user's own paired Devices ask; another person's Device,
//! paired by invite, is declined, as is any Device without its proof. A
//! failure of the question is answered with 200 and its kind in the
//! body, so that the phone tells it apart from a refused request.
//!
//! The network never touches the store, as for a sync: the request
//! crosses the host's channel, and [`AskDesk`] answers a question on a
//! thread of its own, so that sync goes on while the provider thinks.
//! The Dart core's `lan_ask.dart` writes and reads the same bodies.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Map, Value, json};

use crate::Result;
use crate::ask::{self, AskError, Conversation, Exchange, Protocol, Provider, Transport, Unready};
use crate::lan::{Request, Response};
use crate::schedule::WallClock;
use crate::store::Store;

/// Where the phone asks its question.
pub const ASK_PATH: &str = "/ask";
/// Where the phone learns which provider the desktop asks.
pub const PROVIDER_PATH: &str = "/ask/provider";

/// Whether `request` is one of the phone's questions rather than a sync.
pub fn is_ask(request: &Request) -> bool {
    request.path == ASK_PATH || request.path == PROVIDER_PATH
}

/// The provider as the phone may learn it: how the desktop asks, the
/// model, and the host of its address with its port, without scheme,
/// user, path or query; whether it can be asked yet, and why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderShown {
    pub protocol: Protocol,
    pub model: String,
    pub host: String,
    pub unready: Option<Unready>,
}

impl ProviderShown {
    /// What of `provider` the phone is told: never its key.
    pub fn of(provider: &Provider) -> ProviderShown {
        ProviderShown {
            protocol: provider.protocol,
            model: provider.model.trim().to_string(),
            host: ask::host_of(provider.url.trim()).to_string(),
            unready: provider.unready(),
        }
    }

    /// The body of `GET /ask/provider`, keys in order as the Dart core
    /// writes them.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("host".into(), json!(self.host));
        map.insert("model".into(), json!(self.model));
        map.insert("protocol".into(), json!(self.protocol.name()));
        map.insert("ready".into(), json!(self.unready.is_none()));
        if let Some(why) = self.unready {
            map.insert("unready".into(), json!(unready_name(why)));
        }
        Value::Object(map)
    }

    /// The provider `json` names; none for a protocol this build does
    /// not know.
    pub fn from_json(json: &Value) -> Option<ProviderShown> {
        let protocol = match json.get("protocol")?.as_str()? {
            "anthropic" => Protocol::Anthropic,
            "openai" => Protocol::OpenAi,
            _ => return None,
        };
        let text = |key: &str| {
            json.get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let ready = json.get("ready").and_then(Value::as_bool) == Some(true);
        Some(ProviderShown {
            protocol,
            model: text("model"),
            host: text("host"),
            // Not ready for a reason this build does not know: no model.
            unready: (!ready).then(|| unready_of(&text("unready")).unwrap_or(Unready::NoModel)),
        })
    }
}

/// The name of `why` on the wire.
pub fn unready_name(why: Unready) -> &'static str {
    match why {
        Unready::NoModel => "noModel",
        Unready::NoKey => "noKey",
        Unready::BadUrl => "badUrl",
    }
}

fn unready_of(name: &str) -> Option<Unready> {
    match name {
        "noModel" => Some(Unready::NoModel),
        "noKey" => Some(Unready::NoKey),
        "badUrl" => Some(Unready::BadUrl),
        _ => None,
    }
}

/// The phone's question: its text and the earlier exchanges of the same
/// conversation, which it is asked after, as the panel's follow-ups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub question: String,
    pub earlier: Vec<Exchange>,
}

impl Question {
    /// The body of `POST /ask`, keys in order as the Dart core writes
    /// them.
    pub fn to_json(&self) -> Value {
        let earlier: Vec<Value> = self
            .earlier
            .iter()
            .map(|e| json!({"answer": e.answer, "question": e.question}))
            .collect();
        json!({"earlier": earlier, "question": self.question})
    }

    /// The question `json` carries; none without one. An earlier exchange
    /// that is not two texts is left out.
    pub fn from_json(json: &Value) -> Option<Question> {
        let question = json.get("question")?.as_str()?;
        if question.trim().is_empty() {
            return None;
        }
        let earlier = json
            .get("earlier")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                Some(Exchange {
                    question: e.get("question")?.as_str()?.to_string(),
                    answer: e.get("answer")?.as_str()?.to_string(),
                })
            })
            .collect();
        Some(Question {
            question: question.to_string(),
            earlier,
        })
    }
}

/// The answer as the phone gets it: its Markdown text, the Notes it
/// links in the order first linked, and whether it was cut off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    pub text: String,
    pub notes: Vec<String>,
    pub cut: bool,
}

/// Why the phone's question got no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskFailure {
    /// No provider is set up on the desktop yet, and why, when this build
    /// knows the reason.
    Unready(Option<Unready>),
    /// The provider refused the API key, in its own words.
    Key(String),
    /// The provider answered with another error: its status and message.
    Provider(u16, String),
    /// The desktop got no reply from the provider.
    Network(String),
    /// The provider's reply could not be read.
    Unreadable(String),
    /// The model called tools more than [`ask::MAX_ROUNDS`] times.
    Rounds,
    /// The model declined to answer.
    Refused,
    /// The desktop stopped waiting.
    Stopped,
    /// The desktop answers another question now.
    Busy,
    /// A kind a later desktop gives, which this build does not know.
    Unknown,
}

impl AskFailure {
    fn name(&self) -> &'static str {
        match self {
            AskFailure::Unready(_) => "unready",
            AskFailure::Key(_) => "key",
            AskFailure::Provider(..) => "provider",
            AskFailure::Network(_) => "network",
            AskFailure::Unreadable(_) => "unreadable",
            AskFailure::Rounds => "rounds",
            AskFailure::Refused => "refused",
            AskFailure::Stopped => "stopped",
            AskFailure::Busy => "busy",
            AskFailure::Unknown => "unknown",
        }
    }

    /// The failure of a question `provider` was asked, its messages
    /// without the key or the address's user and query in them: a
    /// provider may repeat the key it refused, and an error the address.
    pub fn of(error: AskError, provider: &Provider) -> AskFailure {
        let clean = |text: String| scrub(&text, provider);
        match error {
            AskError::Refused => AskFailure::Refused,
            AskError::Key(message) => AskFailure::Key(clean(message)),
            AskError::Provider(status, message) => AskFailure::Provider(status, clean(message)),
            AskError::Network(why) => AskFailure::Network(clean(why)),
            AskError::Unreadable(why) => AskFailure::Unreadable(clean(why)),
            AskError::TooManyRounds => AskFailure::Rounds,
            AskError::Stopped => AskFailure::Stopped,
        }
    }
}

/// `text` with every secret of `provider` in it replaced: the key, and
/// the user and the query of its address.
fn scrub(text: &str, provider: &Provider) -> String {
    let mut secrets = vec![provider.key.trim()];
    let url = provider.url.trim();
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if let Some((user, _)) = authority.rsplit_once('@') {
        secrets.push(user);
    }
    if let Some((_, query)) = rest.split_once('?') {
        secrets.push(query.split('#').next().unwrap_or_default());
    }
    let mut text = text.to_string();
    for secret in secrets.into_iter().filter(|s| !s.is_empty()) {
        text = text.replace(secret, "…");
    }
    text
}

/// The desktop's reply to the phone's question: the answer or why there
/// is none, and whether changes of an AI Task now wait for the review on
/// the desktop (ADR 0026), which the model's changes went into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskReply {
    pub outcome: std::result::Result<Answered, AskFailure>,
    pub review: bool,
}

impl AskReply {
    /// The body of the answer to `POST /ask`, keys in order as the Dart
    /// core writes them.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        match &self.outcome {
            Ok(answer) => {
                map.insert(
                    "answer".into(),
                    json!({"cut": answer.cut, "notes": answer.notes, "text": answer.text}),
                );
                map.insert("review".into(), json!(self.review));
            }
            Err(failure) => {
                map.insert("failure".into(), json!(failure.name()));
                let message = match failure {
                    AskFailure::Key(m)
                    | AskFailure::Provider(_, m)
                    | AskFailure::Network(m)
                    | AskFailure::Unreadable(m) => Some(m),
                    _ => None,
                };
                if let Some(message) = message {
                    map.insert("message".into(), json!(message));
                }
                map.insert("review".into(), json!(self.review));
                if let AskFailure::Provider(status, _) = failure {
                    map.insert("status".into(), json!(status));
                }
                if let AskFailure::Unready(Some(why)) = failure {
                    map.insert("unready".into(), json!(unready_name(*why)));
                }
            }
        }
        Value::Object(map)
    }

    /// The reply `json` holds; none when it is neither an answer nor a
    /// failure. A failure of a kind this build does not know is
    /// [`AskFailure::Unknown`].
    pub fn from_json(json: &Value) -> Option<AskReply> {
        let review = json.get("review").and_then(Value::as_bool) == Some(true);
        if let Some(answer) = json.get("answer").filter(|a| !a.is_null()) {
            let notes = answer
                .get("notes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            return Some(AskReply {
                outcome: Ok(Answered {
                    text: answer.get("text")?.as_str()?.to_string(),
                    notes,
                    cut: answer.get("cut").and_then(Value::as_bool) == Some(true),
                }),
                review,
            });
        }
        let message = || {
            json.get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let failure = match json.get("failure")?.as_str()? {
            "unready" => AskFailure::Unready(
                json.get("unready")
                    .and_then(Value::as_str)
                    .and_then(unready_of),
            ),
            "key" => AskFailure::Key(message()),
            "provider" => AskFailure::Provider(
                json.get("status")
                    .and_then(Value::as_u64)
                    .and_then(|s| u16::try_from(s).ok())
                    .unwrap_or(0),
                message(),
            ),
            "network" => AskFailure::Network(message()),
            "unreadable" => AskFailure::Unreadable(message()),
            "rounds" => AskFailure::Rounds,
            "refused" => AskFailure::Refused,
            "stopped" => AskFailure::Stopped,
            "busy" => AskFailure::Busy,
            _ => AskFailure::Unknown,
        };
        Some(AskReply {
            outcome: Err(failure),
            review,
        })
    }
}

impl Store {
    /// The refusal of an asking `request` that does not come from one of
    /// the user's own paired Devices with its proof over a nonce of this
    /// host, as a sync's ([`Store::paired_joiner`]); none for one that
    /// may ask. Another person's Device, paired by invite, never asks:
    /// the question reads every Note. A Device paired as one's own is
    /// one's own, whatever role it holds ([`Store::is_others_device`]
    /// says so too).
    pub fn ask_refusal(&self, request: &Request) -> Result<Option<Response>> {
        let Some(device) = self.paired_joiner(request)? else {
            return Ok(Some(Response::declined()));
        };
        if !self.is_own_device(&device)? {
            return Ok(Some(Response::declined()));
        }
        Ok(None)
    }

    /// Answers one of the phone's asking requests on this thread: the
    /// provider, or the question asked through `transport` on this
    /// computer's clock `wall`. Answered while sync waits too: asking
    /// syncs nothing.
    pub fn serve_ask(
        &self,
        request: &Request,
        transport: &dyn Transport,
        wall: &WallClock,
    ) -> Result<Response> {
        if let Some(refusal) = self.ask_refusal(request)? {
            return Ok(refusal);
        }
        Ok(match (request.method.as_str(), request.path.as_str()) {
            ("GET", PROVIDER_PATH) => {
                Response::json(200, &ProviderShown::of(&self.ask_provider()?).to_json())
            }
            ("POST", ASK_PATH) => {
                let question = serde_json::from_slice::<Value>(&request.body)
                    .ok()
                    .and_then(|body| Question::from_json(&body));
                match question {
                    Some(question) => Response::json(
                        200,
                        &self.answer_question(&question, transport, wall)?.to_json(),
                    ),
                    None => Response::status(400),
                }
            }
            _ => Response::status(404),
        })
    }

    /// Asks `question` as the Ask panel does, with this Device's provider,
    /// through `transport` on the clock `wall`: what the model changes is
    /// a draft of the AI Task, and Hidden from AI stays out of every
    /// request ([`Store::ask`]).
    pub fn answer_question(
        &self,
        question: &Question,
        transport: &dyn Transport,
        wall: &WallClock,
    ) -> Result<AskReply> {
        let provider = self.ask_provider()?;
        let outcome = match provider.unready() {
            Some(why) => Err(AskFailure::Unready(Some(why))),
            None => {
                // The time as the panel gives it.
                let now = wall(self.now()).format("%Y-%m-%d %H:%M, %A").to_string();
                let mut conversation = Conversation::new(
                    provider.clone(),
                    &question.earlier,
                    &question.question,
                    &now,
                );
                self.ask(&mut conversation, transport, wall)
                    .map(|asked| Answered {
                        text: asked.answer.text,
                        notes: asked.answer.notes,
                        cut: asked.answer.cut,
                    })
                    .map_err(|e| AskFailure::of(e, &provider))
            }
        };
        // The answer is in: a failure to read the review does not lose it.
        Ok(AskReply {
            outcome,
            review: self.review_waits().unwrap_or(false),
        })
    }

    /// Whether changes of an AI Task wait for the review now: read after
    /// the question, so that one that failed after a draft says so too.
    fn review_waits(&self) -> Result<bool> {
        Ok(match self.ai_task()? {
            Some(id) => !self.ai_review(&id)?.is_empty(),
            None => false,
        })
    }
}

/// This computer's wall clock, shared with the thread that asks.
pub type SharedWall = Arc<dyn Fn(DateTime<Utc>) -> NaiveDateTime + Send + Sync>;

/// Answers the phone's asking requests as the paired host hands them
/// over: a refusal and the provider at once, a question on a thread of
/// its own, so that the host's other requests, the syncs, go on
/// meanwhile. One question at a time: another one meanwhile hears
/// [`AskFailure::Busy`].
pub struct AskDesk {
    store: Arc<Store>,
    transport: Arc<dyn Transport>,
    wall: SharedWall,
    asking: Arc<AtomicBool>,
    answered: Arc<dyn Fn() + Send + Sync>,
}

/// A question being answered, until dropped, also when it panics.
struct Asking(Arc<AtomicBool>);

impl Drop for Asking {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl AskDesk {
    /// Asks on `store` through `transport` on the clock `wall`, telling
    /// `answered` after each question, as the window then shows a review
    /// that may wait.
    pub fn new(
        store: Arc<Store>,
        transport: Arc<dyn Transport>,
        wall: SharedWall,
        answered: impl Fn() + Send + Sync + 'static,
    ) -> AskDesk {
        AskDesk {
            store,
            transport,
            wall,
            asking: Arc::new(AtomicBool::new(false)),
            answered: Arc::new(answered),
        }
    }

    /// Whether a question is being answered now.
    pub fn asking(&self) -> bool {
        self.asking.load(Ordering::SeqCst)
    }

    /// Answers `request`, one of [`is_ask`]'s, and hands its answer back.
    pub fn take(&self, request: Request) {
        let refusal = match self.store.ask_refusal(&request) {
            Ok(refusal) => refusal,
            Err(_) => Some(Response::status(500)),
        };
        if let Some(refusal) = refusal {
            request.reply(refusal);
            return;
        }
        if request.method != "POST" || request.path != ASK_PATH {
            let response = self
                .store
                .serve_ask(&request, &*self.transport, &*self.wall)
                .unwrap_or_else(|_| Response::status(500));
            request.reply(response);
            return;
        }
        if self.asking.swap(true, Ordering::SeqCst) {
            let busy = AskReply {
                outcome: Err(AskFailure::Busy),
                review: self.store.review_waits().unwrap_or(false),
            };
            request.reply(Response::json(200, &busy.to_json()));
            return;
        }
        let asking = Asking(self.asking.clone());
        let (store, transport, wall, answered) = (
            self.store.clone(),
            self.transport.clone(),
            self.wall.clone(),
            self.answered.clone(),
        );
        std::thread::spawn(move || {
            let response = store
                .serve_ask(&request, &*transport, &*wall)
                .unwrap_or_else(|_| Response::status(500));
            drop(asking);
            request.reply(response);
            answered();
        });
    }
}

#[cfg(test)]
mod tests {
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
            end_turn(&format!(
                "In the [blue folder](oldbrainfix://note/{}).",
                passport.id
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
                outcome: Ok(Answered {
                    text: format!("In the [blue folder](oldbrainfix://note/{}).", passport.id),
                    notes: vec![passport.id.clone()],
                    cut: false,
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
        let (second, second_answer) =
            asked(&desk, &phone, "POST", ASK_PATH, &question("Q2"), true)?;
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
}
