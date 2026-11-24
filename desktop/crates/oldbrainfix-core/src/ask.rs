//! Asking about the Notes (#267, ADR 0051): the desktop is the AI client
//! of the Ask panel. The user enters their own provider, its model and
//! their API key; Old Brain Fix provides no model and no account.
//!
//! Two protocols: Anthropic's Messages API, and the OpenAI-compatible
//! Chat Completions API, which DeepSeek, OpenRouter, Ollama, LM Studio and
//! most others speak. The model works through the AI port's asking tools
//! ([`crate::ai_tools::ASK_TOOLS`]), answered in-process by
//! [`Store::ask_tool`]: a read runs as the AI sees the store, in no AI
//! Task; a change is a draft of the AI Task, reviewed as Pending Changes
//! (ADR 0022, 0026).
//!
//! A [`Conversation`] is the protocol alone, with no network and no store:
//! it gives the next [`Request`], takes its [`Reply`], and says whether
//! the model calls tools or has answered. A [`Transport`] posts the
//! request ([`Web`] over HTTPS, a fake one in the tests), on a thread of
//! its own; [`run`] drives the two, the tools answered where the caller
//! says. The provider's settings are local settings of this Device: never
//! synced and in no Backup, like the AI port's token.

use std::net::IpAddr;
use std::time::Duration;

use serde_json::{Value, json};

use crate::Result;
use crate::ai_tools::{ToolContent, ToolOutput, ask_tools};
use crate::app_link::{self, AppLink};
use crate::markdown::{self, Span, SpanKind};
use crate::schedule::WallClock;
use crate::store::Store;

/// The local setting naming the protocol: `anthropic` or `openai`.
pub const PROTOCOL_KEY: &str = "ask:protocol";
/// The local settings of Anthropic's model and key.
pub const ANTHROPIC_MODEL_KEY: &str = "ask:anthropic:model";
pub const ANTHROPIC_KEY_KEY: &str = "ask:anthropic:key";
/// The local settings of an OpenAI-compatible provider: its base URL,
/// model and key.
pub const OPENAI_URL_KEY: &str = "ask:openai:url";
pub const OPENAI_MODEL_KEY: &str = "ask:openai:model";
pub const OPENAI_KEY_KEY: &str = "ask:openai:key";
/// The local setting "Let the model think" of an OpenAI-compatible
/// provider (#318): `true` while on; absent, the default, is off.
pub const OPENAI_THINKS_KEY: &str = "ask:openai:think";
/// The local setting of the main switch "AI (experimental)" (#334):
/// `true` while on; absent, the default, is off, also where a provider
/// was set up before the switch came.
pub const ON_KEY: &str = "ask:on";

/// Where Anthropic's Messages API is, its version, and the model asked
/// while the user names none.
pub const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";
pub const ANTHROPIC_MODEL: &str = "claude-opus-5-5";
/// The base URL of an OpenAI-compatible provider while the user names
/// none: Ollama on this computer.
pub const OPENAI_URL: &str = "http://localhost:11434/v1";

/// What an OpenAI-compatible request sends as `reasoning_effort` while
/// the model may not think (#318): Ollama's `/v1` reads `none` as think
/// false, and OpenAI lists it among its efforts.
pub const NO_THINKING: &str = "none";

/// Where the thinking a model wrote into its content ends (#318): a model
/// that always thinks, such as Qwen3-4B-Thinking, writes it there once
/// asked not to think.
const THINK_END: &str = "</think>";

/// How long Anthropic's answer may be, its thinking included.
const MAX_TOKENS: u32 = 16_000;
/// How often the model is asked for one question at most: each round
/// answers the tools it called.
pub const MAX_ROUNDS: usize = 24;

/// What the model is told, the same for every question so that a
/// provider can keep it cached; the time and the question come in the
/// user's turn. Numbered short steps with an example, which a small
/// local model follows (#314, ADR 0051).
pub const SYSTEM: &str = "You answer questions about the user's own Notes in Old Brain Fix, a memory aid. Do these steps for every question:
1. Call find_notes first, always, before you answer. Give it the main words of the question one by one, such as {\"words\": [\"dentist\", \"appointment\"]}, then search again with synonyms and other spellings in the language of the question. Use list_new for what came in since a time.
2. Call read_note with the id of every Note you want to use. What find_notes and list_new show is not enough; only read_note counts. A Note an earlier answer named counts only once you read it again in this question.
3. Answer briefly and only from the Notes you read with read_note in this question, never from general knowledge or a guess. The Areas, Topics, tags, Routines and check-offs the other tools list count too.
4. Link the Note of every statement as a Markdown link: its title as the label, oldbrainfix://note/ and its id as the address. A link to a Note you did not read with read_note is removed.
5. Only when you searched and found nothing about it, answer only \"That's not in your Notes\", translated into the language of the question.
Example: the question is \"When is the dentist?\". You call find_notes with words [\"dentist\", \"doctor\"]. It finds the Note note:3f2a titled \"Dentist\". You call read_note with id \"note:3f2a\". It says \"Dentist on Friday at 9:00\". You answer: \"On Friday at 9:00, see [Dentist](oldbrainfix://note/note:3f2a).\"
Answer in the language of the question. Change something only when the user asks for it, such as filing what is new into Areas and Topics or adding tags to a text. Every change waits for the user's review in the app, so say what you changed and that it waits for review, linking only the Notes you read. Notes the user hid from the AI do not exist for you.";

/// What the model hears, once per question, when it answers having
/// called no tool at all (#314): a small model otherwise says "That's not
/// in your Notes" without having searched. It quotes `question` (#315):
/// after earlier exchanges a small model otherwise searched the words of
/// an earlier question.
pub fn search_first(question: &str) -> String {
    format!(
        "Search the Notes first for my question \"{question}\": call find_notes with its main words, such as {{\"words\": [\"...\"]}}."
    )
}

/// What the model hears, once per question, when it answered without
/// words having read Notes (#315).
fn answer_now(question: &str) -> String {
    format!("Answer my question \"{question}\" from the Notes you read.")
}

/// What the model hears, once, when it answers having read none of the
/// Notes find_notes or list_new showed it (#314), with the calls to make
/// for the first [`READ_NAMED`] of them: a small model otherwise answers
/// from an excerpt.
fn read_first(shown: &[String]) -> String {
    let calls: Vec<String> = shown
        .iter()
        .take(READ_NAMED)
        .map(|id| json!({"id": id}).to_string())
        .collect();
    format!(
        "Read before you answer: call read_note with {} for each Note you use. Then answer from what read_note returns.",
        calls.join(" or ")
    )
}

/// How many of the Notes shown [`read_first`] names.
const READ_NAMED: usize = 3;

/// The line before the question in the user's turn, the time after it.
const NOW: &str = "Now on this computer: ";

/// The line before the earlier questions and answers of the session,
/// each as `Q: …` and `A: …`, in the user's turn (#315).
const EARLIER: &str = "Earlier in this session, for context only:";

/// The words before the question in a user's turn with earlier exchanges
/// (#315).
const QUESTION_NOW: &str = "My question now: ";

/// How the desktop talks to the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Anthropic's Messages API.
    Anthropic,
    /// The OpenAI-compatible Chat Completions API at a base URL.
    OpenAi,
}

impl Protocol {
    /// Its name in the local setting.
    pub fn name(self) -> &'static str {
        match self {
            Protocol::Anthropic => "anthropic",
            Protocol::OpenAi => "openai",
        }
    }

    /// The protocol named `name`; Anthropic for none or any other.
    pub fn from_name(name: Option<&str>) -> Protocol {
        match name {
            Some("openai") => Protocol::OpenAi,
            _ => Protocol::Anthropic,
        }
    }
}

/// The provider the Ask panel asks, as the user entered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub protocol: Protocol,
    /// The base URL of an OpenAI-compatible provider; Anthropic's address
    /// for Anthropic.
    pub url: String,
    pub model: String,
    /// The API key; empty for none, as a local Ollama needs.
    pub key: String,
    /// Whether the model may think before it answers: "Let the model
    /// think" of an OpenAI-compatible provider, off by default (#318);
    /// Anthropic's thinks adaptively, always.
    pub thinks: bool,
}

/// Why a provider cannot be asked yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unready {
    /// No model is named.
    NoModel,
    /// Anthropic needs an API key.
    NoKey,
    /// The base URL is no `https://` address, nor an `http://` one on
    /// this computer or the home network.
    BadUrl,
    /// The main switch "AI (experimental)" is off on the desktop (#334).
    AiOff,
}

impl Provider {
    /// What keeps the provider from being asked, if anything.
    pub fn unready(&self) -> Option<Unready> {
        if self.model.trim().is_empty() {
            return Some(Unready::NoModel);
        }
        match self.protocol {
            Protocol::Anthropic if self.key.trim().is_empty() => Some(Unready::NoKey),
            Protocol::Anthropic => None,
            Protocol::OpenAi if !url_allowed(&self.url) => Some(Unready::BadUrl),
            Protocol::OpenAi => None,
        }
    }

    /// Whom the questions go to, as the panel names it: Anthropic, or the
    /// host of the base URL with its port.
    pub fn name(&self) -> String {
        match self.protocol {
            Protocol::Anthropic => "Anthropic".into(),
            Protocol::OpenAi => host_of(&self.url).to_string(),
        }
    }
}

/// The host of `url` with its port, without scheme, user and path.
pub(crate) fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or_default();
    rest.rsplit_once('@').map_or(rest, |(_, host)| host)
}

/// The host of `url` without its port, an IPv6 address without its
/// brackets.
fn host_name(url: &str) -> &str {
    let host = host_of(url);
    if let Some(inner) = host.strip_prefix('[') {
        return inner.split(']').next().unwrap_or_default();
    }
    host.rsplit_once(':').map_or(host, |(name, _)| name)
}

/// Whether `url` may be asked: any `https://` address, and an `http://`
/// one only on this computer or the home network (a private or
/// link-local address, `localhost`, a `.local` name), where a local model
/// runs; what crosses the internet is encrypted.
pub fn url_allowed(url: &str) -> bool {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let name = host_name(url).to_ascii_lowercase();
    if name.is_empty() || name.contains(char::is_whitespace) {
        return false;
    }
    if lower.starts_with("https://") {
        return true;
    }
    if !lower.starts_with("http://") {
        return false;
    }
    if name == "localhost" || name.ends_with(".localhost") || name.ends_with(".local") {
        return true;
    }
    match name.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Ok(IpAddr::V6(ip)) => {
            let first = ip.segments()[0];
            ip.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
        Err(_) => false,
    }
}

impl Store {
    /// Whether the main switch "AI (experimental)" is on on this computer
    /// (#334): off until the user switches it on. It gates the Ask panel
    /// and the phone's questions, never the AI port.
    pub fn ai_on(&self) -> Result<bool> {
        Ok(self.local_setting(ON_KEY)?.as_deref() == Some("true"))
    }

    /// Switches the AI on or off; off forgets nothing of the provider.
    pub fn set_ai_on(&self, on: bool) -> Result<()> {
        if on {
            self.set_local_setting(ON_KEY, "true")
        } else {
            self.remove_local_setting(ON_KEY)
        }
    }

    /// What keeps the Ask panel and the phone from asking now: the AI
    /// switched off, else what the provider lacks ([`Provider::unready`]).
    pub fn ask_unready(&self, provider: &Provider) -> Result<Option<Unready>> {
        if !self.ai_on()? {
            return Ok(Some(Unready::AiOff));
        }
        Ok(provider.unready())
    }

    /// The provider the Ask panel asks, as this Device keeps it: the
    /// protocol chosen, with its base URL, model and key, Anthropic's
    /// model and Ollama's address while none is entered.
    pub fn ask_provider(&self) -> Result<Provider> {
        let protocol = Protocol::from_name(self.local_setting(PROTOCOL_KEY)?.as_deref());
        let kept =
            |key: &str| -> Result<String> { Ok(self.local_setting(key)?.unwrap_or_default()) };
        Ok(match protocol {
            Protocol::Anthropic => Provider {
                protocol,
                url: ANTHROPIC_URL.into(),
                model: match kept(ANTHROPIC_MODEL_KEY)? {
                    m if m.trim().is_empty() => ANTHROPIC_MODEL.into(),
                    m => m,
                },
                key: kept(ANTHROPIC_KEY_KEY)?,
                thinks: true,
            },
            Protocol::OpenAi => Provider {
                protocol,
                url: match kept(OPENAI_URL_KEY)? {
                    u if u.trim().is_empty() => OPENAI_URL.into(),
                    u => u,
                },
                model: kept(OPENAI_MODEL_KEY)?,
                key: kept(OPENAI_KEY_KEY)?,
                thinks: self.ask_thinks()?,
            },
        })
    }

    /// Keeps the protocol the Ask panel asks with; each keeps its own
    /// model and key.
    pub fn set_ask_protocol(&self, protocol: Protocol) -> Result<()> {
        self.set_local_setting(PROTOCOL_KEY, protocol.name())
    }

    /// Keeps the model of the protocol chosen; empty for its default.
    pub fn set_ask_model(&self, model: &str) -> Result<()> {
        let key = match self.ask_provider()?.protocol {
            Protocol::Anthropic => ANTHROPIC_MODEL_KEY,
            Protocol::OpenAi => OPENAI_MODEL_KEY,
        };
        self.keep_or_remove(key, model)
    }

    /// Keeps the API key of the protocol chosen; empty for none.
    pub fn set_ask_key(&self, api_key: &str) -> Result<()> {
        let key = match self.ask_provider()?.protocol {
            Protocol::Anthropic => ANTHROPIC_KEY_KEY,
            Protocol::OpenAi => OPENAI_KEY_KEY,
        };
        self.keep_or_remove(key, api_key)
    }

    /// Keeps the base URL of an OpenAI-compatible provider; empty for
    /// Ollama on this computer.
    pub fn set_ask_url(&self, url: &str) -> Result<()> {
        self.keep_or_remove(OPENAI_URL_KEY, url)
    }

    /// Whether "Let the model think" is on for an OpenAI-compatible
    /// provider (#318): off until switched on.
    pub fn ask_thinks(&self) -> Result<bool> {
        Ok(self.local_setting(OPENAI_THINKS_KEY)?.as_deref() == Some("true"))
    }

    /// Switches "Let the model think" on or off.
    pub fn set_ask_thinks(&self, thinks: bool) -> Result<()> {
        if thinks {
            self.set_local_setting(OPENAI_THINKS_KEY, "true")
        } else {
            self.remove_local_setting(OPENAI_THINKS_KEY)
        }
    }

    fn keep_or_remove(&self, key: &str, value: &str) -> Result<()> {
        if value.trim().is_empty() {
            self.remove_local_setting(key)
        } else {
            self.set_local_setting(key, value.trim())
        }
    }

    /// Declines the AI Task `id` when it holds no change, so that a
    /// question whose model changed nothing leaves no review waiting;
    /// whether it did.
    pub fn drop_empty_ai_task(&self, id: &str) -> Result<bool> {
        if self.ai_task()?.as_deref() != Some(id) || !self.ai_review(id)?.is_empty() {
            return Ok(false);
        }
        self.decline_pending(id)?;
        Ok(true)
    }
}

/// One HTTP request to the provider: a POST of a JSON body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// The provider's reply: its HTTP status and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

/// What posts a [`Request`]; an error when no reply came, saying why.
pub trait Transport: Send + Sync {
    fn post(&self, request: &Request) -> std::result::Result<Reply, String>;
}

/// The network: HTTPS, or HTTP on this computer and the home network
/// ([`url_allowed`]), following no redirect, so that the key goes to
/// the address entered only.
pub struct Web {
    agent: ureq::Agent,
}

impl Web {
    pub fn new() -> Web {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_connect(Some(Duration::from_secs(30)))
            // A long answer with its thinking comes in one piece.
            .timeout_global(Some(Duration::from_secs(600)))
            .build();
        Web {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Default for Web {
    fn default() -> Web {
        Web::new()
    }
}

impl Transport for Web {
    fn post(&self, request: &Request) -> std::result::Result<Reply, String> {
        let mut post = self.agent.post(&request.url);
        for (name, value) in &request.headers {
            post = post.header(name.as_str(), value.as_str());
        }
        let response = post
            .send(request.body.as_bytes())
            .map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok(Reply { status, body })
    }
}

/// A tool the model calls: the provider's id of the call, the tool's
/// name and its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

/// An earlier question of the panel and its answer, which a follow-up
/// question is asked after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    pub question: String,
    pub answer: String,
}

/// The model's answer: its text, Markdown with the Notes it came from as
/// app links, those Notes' ids in the order first linked, the Notes the
/// tools handed out in full, whether the answer was cut off at its
/// length, and whether a link was removed from it: every Note it links
/// was read in this question ([`checked`], #294).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub text: String,
    pub notes: Vec<String>,
    pub read: Vec<String>,
    pub cut: bool,
    pub unchecked: bool,
}

impl Answer {
    /// The Notes shown under the answer as "Also read" (#314, Q719): those
    /// read in this question that it does not link, in the order read.
    /// Its links come first, as [`Answer::notes`].
    pub fn also_read(&self) -> Vec<String> {
        also_read(&self.notes, &self.read)
    }
}

/// The ids of `read` not in `linked`, in the order read, each once: the
/// Notes an answer shows as "Also read" (#314), on the panel and the
/// phone alike.
pub fn also_read(linked: &[String], read: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in read {
        if !linked.contains(id) && !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// What the model did after a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// It paused a long turn, or answered before it searched or read and
    /// was told to ([`search_first`], #314): ask again as it stands.
    Again,
    /// It calls these tools; their outputs go to [`Conversation::answer`].
    Tools(Vec<ToolCall>),
    Answered(Answer),
}

/// Why a question got no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskError {
    /// The model declined to answer.
    Refused,
    /// The provider refused the API key (401, 403).
    Key(String),
    /// The provider answered with another error, its status and message.
    Provider(u16, String),
    /// No reply came: the network, the address, a timeout.
    Network(String),
    /// The reply was not one the protocol gives.
    Unreadable(String),
    /// The model called tools more than [`MAX_ROUNDS`] times.
    TooManyRounds,
    /// The panel stopped waiting.
    Stopped,
}

impl std::fmt::Display for AskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AskError::Refused => write!(f, "the model declined to answer"),
            AskError::Key(message) => write!(f, "the API key was refused: {message}"),
            AskError::Provider(status, message) => write!(f, "HTTP {status}: {message}"),
            AskError::Network(why) => write!(f, "no reply: {why}"),
            AskError::Unreadable(why) => write!(f, "the reply could not be read: {why}"),
            AskError::TooManyRounds => {
                write!(f, "the model called tools more than {MAX_ROUNDS} times")
            }
            AskError::Stopped => write!(f, "stopped"),
        }
    }
}

/// One question to the provider, with the earlier ones of the panel, as
/// the protocol carries it.
#[derive(Debug, Clone)]
pub struct Conversation {
    provider: Provider,
    messages: Vec<Value>,
    rounds: usize,
    read: Vec<String>,
    /// Calls whose arguments could not be read, answered as errors with
    /// the next tool outputs.
    unreadable: Vec<(String, String)>,
    /// Whether the model called any tool in this question.
    called: bool,
    /// The Notes `find_notes` and `list_new` showed in this question, in
    /// the order shown.
    shown: Vec<String>,
    /// Whether the model was told [`search_first`], or to read the Notes
    /// shown ([`read_first`]): each once per question (#314).
    told_search: bool,
    told_read: bool,
    /// Whether an answer without words was sent back, once per question
    /// (#315).
    told_empty: bool,
    /// The question asked, which [`search_first`] quotes (#315).
    question: String,
    /// Whether the provider refused the request asking its model not to
    /// think, so that it is asked without (#318).
    no_thinking_refused: bool,
}

impl Conversation {
    /// The question `question` asked at `now` (this computer's time, as
    /// it reads), after the panel's `earlier` exchanges. Those go in the
    /// same one user turn, as a block for context before the question
    /// ([`EARLIER`], [`QUESTION_NOW`], #315): as turns of their own, a
    /// small model answered an earlier question again.
    pub fn new(provider: Provider, earlier: &[Exchange], question: &str, now: &str) -> Self {
        let content = if earlier.is_empty() {
            format!("{NOW}{now}\n\n{question}")
        } else {
            let before: Vec<String> = earlier
                .iter()
                .map(|e| format!("Q: {}\nA: {}", e.question, e.answer))
                .collect();
            format!(
                "{NOW}{now}\n\n{EARLIER}\n{}\n\n{QUESTION_NOW}{question}",
                before.join("\n")
            )
        };
        let messages = vec![json!({"role": "user", "content": content})];
        Conversation {
            provider,
            messages,
            rounds: 0,
            read: Vec::new(),
            unreadable: Vec::new(),
            called: false,
            shown: Vec::new(),
            told_search: false,
            told_read: false,
            told_empty: false,
            question: question.to_string(),
            no_thinking_refused: false,
        }
    }

    pub fn provider(&self) -> &Provider {
        &self.provider
    }

    /// The next request to the provider.
    pub fn request(&self) -> Request {
        let json_type = ("content-type".to_string(), "application/json".to_string());
        match self.provider.protocol {
            Protocol::Anthropic => {
                let tools: Vec<Value> = ask_tools()
                    .iter()
                    .map(|t| {
                        json!({
                            "description": t["description"],
                            "input_schema": t["inputSchema"],
                            "name": t["name"],
                        })
                    })
                    .collect();
                // The system prompt and the tools never change, so the
                // provider keeps them cached: the mark on the system
                // block caches the tools before it too.
                let body = json!({
                    "max_tokens": MAX_TOKENS,
                    "messages": self.messages,
                    "model": self.provider.model.trim(),
                    "system": [{
                        "cache_control": {"type": "ephemeral"},
                        "text": SYSTEM,
                        "type": "text",
                    }],
                    "thinking": {"type": "adaptive"},
                    "tools": tools,
                });
                Request {
                    url: ANTHROPIC_URL.into(),
                    headers: vec![
                        json_type,
                        ("x-api-key".into(), self.provider.key.trim().into()),
                        ("anthropic-version".into(), ANTHROPIC_VERSION.into()),
                    ],
                    body: body.to_string(),
                }
            }
            Protocol::OpenAi => {
                let tools: Vec<Value> = ask_tools()
                    .iter()
                    .map(|t| {
                        json!({
                            "function": {
                                "description": t["description"],
                                "name": t["name"],
                                "parameters": t["inputSchema"],
                            },
                            "type": "function",
                        })
                    })
                    .collect();
                let mut messages = vec![json!({"role": "system", "content": SYSTEM})];
                messages.extend(self.messages.iter().cloned());
                let mut body = json!({
                    "messages": messages,
                    "model": self.provider.model.trim(),
                    "tools": tools,
                });
                if self.asks_no_thinking()
                    && let Some(fields) = body.as_object_mut()
                {
                    fields.insert("reasoning_effort".into(), json!(NO_THINKING));
                }
                let mut headers = vec![json_type];
                let key = self.provider.key.trim();
                if !key.is_empty() {
                    headers.push(("authorization".into(), format!("Bearer {key}")));
                }
                Request {
                    url: format!(
                        "{}/chat/completions",
                        self.provider.url.trim().trim_end_matches('/')
                    ),
                    headers,
                    body: body.to_string(),
                }
            }
        }
    }

    /// Whether the next request asks the model not to think: an
    /// OpenAI-compatible one while "Let the model think" is off, unless
    /// the provider refused that once.
    fn asks_no_thinking(&self) -> bool {
        self.provider.protocol == Protocol::OpenAi
            && !self.provider.thinks
            && !self.no_thinking_refused
    }

    /// Takes the provider's `reply` to the last request.
    pub fn take(&mut self, reply: &Reply) -> std::result::Result<Step, AskError> {
        // A server that knows no `reasoning_effort`, or not `none` for
        // this model, refuses the request: it is asked once more without
        // (#318), and a second refusal is its error.
        if matches!(reply.status, 400 | 422) && self.asks_no_thinking() {
            self.no_thinking_refused = true;
            return Ok(Step::Again);
        }
        if !(200..300).contains(&reply.status) {
            let message = error_message(&reply.body);
            return Err(match reply.status {
                401 | 403 => AskError::Key(message),
                status => AskError::Provider(status, message),
            });
        }
        let body: Value =
            serde_json::from_str(&reply.body).map_err(|e| AskError::Unreadable(e.to_string()))?;
        self.rounds += 1;
        let step = match self.provider.protocol {
            Protocol::Anthropic => self.take_anthropic(body)?,
            Protocol::OpenAi => self.take_openai(body)?,
        };
        if matches!(step, Step::Tools(_)) && self.rounds >= MAX_ROUNDS {
            return Err(AskError::TooManyRounds);
        }
        Ok(step)
    }

    fn take_anthropic(&mut self, body: Value) -> std::result::Result<Step, AskError> {
        let content = body
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| AskError::Unreadable("no content".into()))?;
        let stop = body.get("stop_reason").and_then(Value::as_str);
        let text = content
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("");
        if stop == Some("refusal") {
            return Err(AskError::Refused);
        }
        // Thinking blocks included, as they came: the next request
        // carries them back unchanged.
        self.messages
            .push(json!({"role": "assistant", "content": content}));
        Ok(match stop {
            Some("pause_turn") => Step::Again,
            Some("tool_use") => {
                let calls: Vec<ToolCall> = content
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
                    .map(|b| ToolCall {
                        id: b["id"].as_str().unwrap_or_default().to_string(),
                        name: b["name"].as_str().unwrap_or_default().to_string(),
                        args: mended(
                            b["name"].as_str().unwrap_or_default(),
                            b.get("input").cloned().unwrap_or_else(|| json!({})),
                        ),
                    })
                    .collect();
                if calls.is_empty() {
                    self.answered(text, false)
                } else {
                    Step::Tools(calls)
                }
            }
            Some("max_tokens") | Some("model_context_window_exceeded") => self.answered(text, true),
            _ => self.answered(text, false),
        })
    }

    fn take_openai(&mut self, body: Value) -> std::result::Result<Step, AskError> {
        let choice = body
            .pointer("/choices/0")
            .cloned()
            .ok_or_else(|| AskError::Unreadable("no choice".into()))?;
        let message = choice
            .get("message")
            .cloned()
            .ok_or_else(|| AskError::Unreadable("no message".into()))?;
        let finish = choice.get("finish_reason").and_then(Value::as_str);
        if finish == Some("content_filter") {
            return Err(AskError::Refused);
        }
        let mut message = message;
        let text = without_thinking(
            message
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )
        .to_string();
        if let Some(content) = message.get_mut("content")
            && content.is_string()
        {
            // Nor does the thinking go back to the model.
            *content = json!(text);
        }
        let mut calls = Vec::new();
        for call in message
            .get("tool_calls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = call["id"].as_str().unwrap_or_default().to_string();
            let name = call
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            // The arguments come as a JSON text, which a small model may
            // get wrong: the model hears so and may try again.
            let args = match call.pointer("/function/arguments") {
                Some(Value::String(text)) if text.trim().is_empty() => Ok(json!({})),
                Some(Value::String(text)) => serde_json::from_str::<Value>(text),
                Some(Value::Object(args)) => Ok(Value::Object(args.clone())),
                _ => Ok(json!({})),
            };
            match args {
                Ok(args) => {
                    let args = mended(&name, args);
                    calls.push(ToolCall { id, name, args })
                }
                Err(e) => self
                    .unreadable
                    .push((id, format!("the arguments are no JSON object: {e}"))),
            }
        }
        let mut kept = message;
        if let Some(all) = kept.as_object_mut() {
            all.insert("role".into(), json!("assistant"));
        }
        self.messages.push(kept);
        if !calls.is_empty() || !self.unreadable.is_empty() {
            return Ok(Step::Tools(calls));
        }
        Ok(self.answered(text, finish == Some("length")))
    }

    /// The model's answer `text`, or [`Step::Again`] after telling it to
    /// search or to read first (#314): once each per question, only
    /// while it has read no Note, and never for an answer cut off at its
    /// length. It is told to search when it called no tool at all, since
    /// an answer from the Areas, tags or Routines the other tools list
    /// counts, and to read when `find_notes` or `list_new` showed Notes.
    /// An answer without words is none (#315): once per question it hears
    /// again what fits, to read, to search, or to answer.
    fn answered(&mut self, text: String, cut: bool) -> Step {
        let mut told = None;
        if !cut && self.read.is_empty() {
            if !self.called && !self.told_search {
                self.told_search = true;
                told = Some(search_first(&self.question));
            } else if !self.shown.is_empty() && !self.told_read {
                self.told_read = true;
                told = Some(read_first(&self.shown));
            }
        }
        if told.is_none() && !cut && text.trim().is_empty() && !self.told_empty {
            self.told_empty = true;
            told = Some(if !self.shown.is_empty() && self.read.is_empty() {
                read_first(&self.shown)
            } else if !self.called {
                search_first(&self.question)
            } else {
                answer_now(&self.question)
            });
        }
        if let Some(told) = told {
            self.messages.push(json!({"role": "user", "content": told}));
            return Step::Again;
        }
        let (text, unchecked) = checked(&text, &self.read);
        Step::Answered(Answer {
            notes: linked_notes(&text),
            text,
            read: self.read.clone(),
            cut,
            unchecked,
        })
    }

    /// Gives the model what the tools it called answered, each output
    /// beside its call. A Note counts as read when `read_note` or
    /// `read_attachment_text` handed it out: a tool that only names a
    /// Note, such as `read_check_offs`, answers for one hidden from the AI
    /// or none at all without an error.
    pub fn answer(&mut self, done: &[(ToolCall, ToolOutput)]) {
        if !done.is_empty() || !self.unreadable.is_empty() {
            self.called = true;
        }
        for (call, output) in done {
            if let Some(id) = note_read(call, output)
                && !self.read.iter().any(|r| r == id)
            {
                self.read.push(id.to_string());
            }
            for id in notes_shown(call, output) {
                if !self.shown.contains(&id) {
                    self.shown.push(id);
                }
            }
        }
        let unreadable = std::mem::take(&mut self.unreadable);
        match self.provider.protocol {
            Protocol::Anthropic => {
                let results: Vec<Value> = done
                    .iter()
                    .map(|(call, output)| {
                        json!({
                            "content": output_text(output),
                            "is_error": output.is_error,
                            "tool_use_id": call.id,
                            "type": "tool_result",
                        })
                    })
                    .collect();
                self.messages
                    .push(json!({"role": "user", "content": results}));
            }
            Protocol::OpenAi => {
                for (id, why) in unreadable {
                    self.messages
                        .push(json!({"role": "tool", "tool_call_id": id, "content": why}));
                }
                for (call, output) in done {
                    self.messages.push(json!({
                        "content": output_text(output),
                        "role": "tool",
                        "tool_call_id": call.id,
                    }));
                }
            }
        }
    }
}

/// `content` without the thinking a model wrote into it: what follows the
/// last [`THINK_END`], its leading blank lines left out; all of it when
/// there is none (#318).
fn without_thinking(content: &str) -> &str {
    content
        .rsplit_once(THINK_END)
        .map_or(content, |(_, after)| after.trim_start())
}

/// The text of a tool's output; a photo, which the panel's tools never
/// give, as a word.
fn output_text(output: &ToolOutput) -> String {
    output
        .content
        .iter()
        .map(|c| match c {
            ToolContent::Text(text) => text.clone(),
            ToolContent::Jpeg(_) => "(a photo, not sent)".into(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The message of an error reply, as both protocols give it under
/// `error.message`, else the body as it came, cut short.
fn error_message(body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let message = parsed.as_ref().and_then(|v| {
        v.pointer("/error/message")
            .and_then(Value::as_str)
            .or_else(|| v.get("error").and_then(Value::as_str))
            .or_else(|| v.get("message").and_then(Value::as_str))
    });
    match message {
        Some(m) => m.to_string(),
        None => body.chars().take(300).collect(),
    }
}

/// The ids of the Notes `text` links as app links (ADR 0046), in the
/// order first linked, each once.
pub fn linked_notes(text: &str) -> Vec<String> {
    fn walk(spans: &[Span], out: &mut Vec<String>) {
        for span in spans {
            if span.kind == SpanKind::Link
                && let Some(AppLink::Note(id)) = app_link::parse(&span.url)
                && !out.contains(&id)
            {
                out.push(id);
            }
            walk(&span.children, out);
        }
    }
    let mut out = Vec::new();
    for block in markdown::parse(text) {
        walk(&block.spans, &mut out);
    }
    out
}

/// `text` with every app link to a Note not in `read` made plain text,
/// its label's words kept (#294, Q691), and whether one was: the answer
/// may name only the Notes the model read in this question, which also
/// leaves out any Hidden from AI or none that exists. A link's label is
/// read without links, so one that shows once its link is gone is
/// checked too. An answer without such a link stays as it is.
pub fn checked(text: &str, read: &[String]) -> (String, bool) {
    fn walk(spans: &[Span], read: &[String], out: &mut Vec<(usize, usize, usize, usize)>) {
        for span in spans {
            if span.kind == SpanKind::Link
                && let Some(AppLink::Note(id)) = app_link::parse(&span.url)
            {
                if !read.contains(&id) {
                    out.push((span.start, span.end, span.content_start, span.content_end));
                }
                continue;
            }
            walk(&span.children, read, out);
        }
    }
    let mut text = text.to_string();
    let mut unchecked = false;
    loop {
        let mut cut = Vec::new();
        for block in markdown::parse(&text) {
            walk(&block.spans, read, &mut cut);
        }
        cut.sort_unstable();
        let mut changed = false;
        for (start, end, label_start, label_end) in cut.into_iter().rev() {
            if let (Some(before), Some(label), Some(after)) = (
                text.get(..start),
                text.get(label_start..label_end),
                text.get(end..),
            ) {
                text = format!("{before}{label}{after}");
                changed = true;
            }
        }
        if !changed {
            return (text, unchecked);
        }
        unchecked = true;
    }
}

/// The arguments of a call as the tool takes them: a `read_note` that
/// names its Note as `note`, the key of `read_attachment_text`, and gives
/// no `id`, reads it by `id`; an `id` given wins (#315). A small model
/// passed it so even after [`read_first`] spelt the call out. The one
/// argument rule, which the end to end check's record of the Notes asked
/// for follows too.
pub fn mended(name: &str, mut args: Value) -> Value {
    if name != "read_note" || args.get("id").is_some() {
        return args;
    }
    if let Some(all) = args.as_object_mut()
        && let Some(id) = all.remove("note")
    {
        all.insert("id".into(), id);
    }
    args
}

/// The Note `call` read in full, as `output` handed it out: the id that
/// `read_note` or `read_attachment_text` was given, when it answered
/// without an error. A tool that only names a Note, such as
/// `read_check_offs`, reads none. The one rule for what counts as read,
/// for the check of the answer's links (#294) and for how many Notes the
/// question has read so far (#317).
pub fn note_read<'a>(call: &'a ToolCall, output: &ToolOutput) -> Option<&'a str> {
    let key = match call.name.as_str() {
        "read_note" => "id",
        "read_attachment_text" => "note",
        _ => return None,
    };
    if output.is_error {
        return None;
    }
    call.args
        .get(key)
        .and_then(Value::as_str)
        .filter(|id| id.starts_with("note:"))
}

/// The ids of the Notes `find_notes` or `list_new` showed in `output`,
/// in order; none for another tool or an error.
fn notes_shown(call: &ToolCall, output: &ToolOutput) -> Vec<String> {
    if output.is_error || !matches!(call.name.as_str(), "find_notes" | "list_new") {
        return Vec::new();
    }
    let parsed: Option<Value> = serde_json::from_str(&output_text(output)).ok();
    parsed
        .as_ref()
        .and_then(|v| v.get("notes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|n| n.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

/// Asks `conversation` through `transport` until the model answers, the
/// tools it calls answered by `tools`, an output for each call in
/// order; `None` from `tools` stops it ([`AskError::Stopped`]).
pub fn run(
    conversation: &mut Conversation,
    transport: &dyn Transport,
    mut tools: impl FnMut(&[ToolCall]) -> Option<Vec<ToolOutput>>,
) -> std::result::Result<Answer, AskError> {
    loop {
        let reply = transport
            .post(&conversation.request())
            .map_err(AskError::Network)?;
        match conversation.take(&reply)? {
            Step::Again => {}
            Step::Tools(calls) => {
                let outputs = tools(&calls).ok_or(AskError::Stopped)?;
                let done: Vec<(ToolCall, ToolOutput)> = calls.into_iter().zip(outputs).collect();
                conversation.answer(&done);
            }
            Step::Answered(answer) => return Ok(answer),
        }
    }
}

/// An answer and the AI Task its model's changes went into, if it made
/// any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub answer: Answer,
    pub task: Option<String>,
}

impl Store {
    /// Asks `conversation` through `transport` on this thread, every tool
    /// answered here by [`Store::ask_tool`] on this computer's clock
    /// `wall`. An AI Task the model opened and left without a change is
    /// declined ([`Store::drop_empty_ai_task`]) unless one waited before,
    /// so that sync goes on.
    pub fn ask(
        &self,
        conversation: &mut Conversation,
        transport: &dyn Transport,
        wall: &WallClock,
    ) -> std::result::Result<Asked, AskError> {
        self.ask_with(conversation, transport, wall, &|_| true)
    }

    /// [`Store::ask`], telling `progress` how many Notes the model read
    /// so far ([`note_read`]) before and after each round of tools (#317);
    /// `false` from it stops the question there ([`AskError::Stopped`]),
    /// as the phone's Stop does.
    pub fn ask_with(
        &self,
        conversation: &mut Conversation,
        transport: &dyn Transport,
        wall: &WallClock,
        progress: &dyn Fn(usize) -> bool,
    ) -> std::result::Result<Asked, AskError> {
        let before = self.ai_task().ok().flatten();
        let mut task = before.clone();
        let mut read: Vec<String> = Vec::new();
        let answered = run(conversation, transport, |calls| {
            if !progress(read.len()) {
                return None;
            }
            let outputs = self.ask_calls(calls, wall, &mut task);
            for (call, output) in calls.iter().zip(&outputs) {
                if let Some(id) = note_read(call, output)
                    && !read.iter().any(|r| r == id)
                {
                    read.push(id.to_string());
                }
            }
            progress(read.len()).then_some(outputs)
        });
        if before.is_none()
            && let Some(id) = &task
            && self.drop_empty_ai_task(id).unwrap_or(false)
        {
            task = None;
        }
        Ok(Asked {
            answer: answered?,
            task,
        })
    }

    /// The outputs of the tool `calls` of the Ask panel, in order
    /// ([`Store::ask_tool`]), a failure of the store as an error output.
    pub fn ask_calls(
        &self,
        calls: &[ToolCall],
        wall: &WallClock,
        task: &mut Option<String>,
    ) -> Vec<ToolOutput> {
        calls
            .iter()
            .map(|call| {
                self.ask_tool(&call.name, &call.args, wall, task)
                    .unwrap_or_else(|e| ToolOutput {
                        content: vec![ToolContent::Text(e.to_string())],
                        is_error: true,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
