//! Asking about the Notes (#267, ADR 0051): the desktop is the AI client
//! of the Ask panel. The user enters their own provider, its model and
//! their API key; Old Brain Fix provides no model and no account.
//!
//! Two protocols: Anthropic's Messages API, and the OpenAI-compatible
//! Chat Completions API, which DeepSeek, OpenRouter, Ollama, LM Studio and
//! most others speak. The model works through the AI port's asking tools
//! ([`crate::ai_tools::ASK_TOOLS`]), answered in-process by
//! [`Store::ask_tool`]: a read runs as the AI sees the store, in no AI
//! Task, so asking does not pause sync; a change is a draft of the AI
//! Task, reviewed as Pending Changes (ADR 0022, 0026).
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

/// Where Anthropic's Messages API is, its version, and the model asked
/// while the user names none.
pub const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
pub const ANTHROPIC_VERSION: &str = "2023-06-01";
pub const ANTHROPIC_MODEL: &str = "claude-opus-5-5";
/// The base URL of an OpenAI-compatible provider while the user names
/// none: Ollama on this computer.
pub const OPENAI_URL: &str = "http://localhost:11434/v1";

/// How long Anthropic's answer may be, its thinking included.
const MAX_TOKENS: u32 = 16_000;
/// How often the model is asked for one question at most: each round
/// answers the tools it called.
pub const MAX_ROUNDS: usize = 24;

/// What the model is told, the same for every question so that a
/// provider can keep it cached; the time and the question come in the
/// user's turn.
pub const SYSTEM: &str = "You answer questions about the user's own Notes in Old Brain Fix, a memory aid. Find what you need with the tools: search with find_notes several times, with different words, synonyms and spellings in the user's language, use list_new for what came in since a time, and read the Notes you use with read_note. Answer briefly and only from what the Notes say; when they hold nothing about it, say so. Link every Note you use as a Markdown link with its title as the label and oldbrainfix://note/ followed by its id as the address, such as [Dentist](oldbrainfix://note/note:3f2a). Answer in the language of the question. Change something only when the user asks for it, such as filing what is new into Areas and Topics or adding tags to a text: every change waits for the user's review in the app, so say what you changed and that it waits for review. Notes the user hid from the AI do not exist for you.";

/// The line before the question in the user's turn, the time after it.
const NOW: &str = "Now on this computer: ";

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
            },
            Protocol::OpenAi => Provider {
                protocol,
                url: match kept(OPENAI_URL_KEY)? {
                    u if u.trim().is_empty() => OPENAI_URL.into(),
                    u => u,
                },
                model: kept(OPENAI_MODEL_KEY)?,
                key: kept(OPENAI_KEY_KEY)?,
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

    fn keep_or_remove(&self, key: &str, value: &str) -> Result<()> {
        if value.trim().is_empty() {
            self.remove_local_setting(key)
        } else {
            self.set_local_setting(key, value.trim())
        }
    }

    /// Declines the AI Task `id` when it holds no change, so that a
    /// question whose model changed nothing leaves sync running; whether
    /// it did.
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
/// app links, those Notes' ids in the order first linked (explicit, so
/// that what they link can be checked, #294), the Notes the tools handed
/// out in full, and whether the answer was cut off at its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub text: String,
    pub notes: Vec<String>,
    pub read: Vec<String>,
    pub cut: bool,
}

/// What the model did after a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// It paused a long turn: ask again as it stands.
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
}

impl Conversation {
    /// The question `question` asked at `now` (this computer's time, as
    /// it reads), after the panel's `earlier` exchanges.
    pub fn new(provider: Provider, earlier: &[Exchange], question: &str, now: &str) -> Self {
        let mut messages = Vec::new();
        for exchange in earlier {
            messages.push(json!({"role": "user", "content": exchange.question}));
            messages.push(json!({"role": "assistant", "content": exchange.answer}));
        }
        messages.push(json!({"role": "user", "content": format!("{NOW}{now}\n\n{question}")}));
        Conversation {
            provider,
            messages,
            rounds: 0,
            read: Vec::new(),
            unreadable: Vec::new(),
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
                let body = json!({
                    "messages": messages,
                    "model": self.provider.model.trim(),
                    "tools": tools,
                });
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

    /// Takes the provider's `reply` to the last request.
    pub fn take(&mut self, reply: &Reply) -> std::result::Result<Step, AskError> {
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
                        args: b.get("input").cloned().unwrap_or_else(|| json!({})),
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
        let text = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
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
                Ok(args) => calls.push(ToolCall { id, name, args }),
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

    fn answered(&self, text: String, cut: bool) -> Step {
        Step::Answered(Answer {
            notes: linked_notes(&text),
            text,
            read: self.read.clone(),
            cut,
        })
    }

    /// Gives the model what the tools it called answered, each output
    /// beside its call.
    pub fn answer(&mut self, done: &[(ToolCall, ToolOutput)]) {
        for (call, output) in done {
            if !output.is_error {
                for key in ["id", "note"] {
                    if call.name.starts_with("read_")
                        && let Some(id) = call.args.get(key).and_then(Value::as_str)
                        && id.starts_with("note:")
                        && !self.read.iter().any(|r| r == id)
                    {
                        self.read.push(id.to_string());
                    }
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
        let before = self.ai_task().ok().flatten();
        let mut task = before.clone();
        let answered = run(conversation, transport, |calls| {
            Some(self.ask_calls(calls, wall, &mut task))
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
mod tests {
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
        }
    }

    fn openai(key: &str) -> Provider {
        Provider {
            protocol: Protocol::OpenAi,
            url: "http://localhost:11434/v1/".into(),
            model: "qwen3".into(),
            key: key.into(),
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
        let request =
            Conversation::new(anthropic(), &[], "Where is the passport?", "now").request();
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
    fn asking_opens_no_ai_task_and_leaves_sync_running() -> TestResult {
        let store = store();
        Notes::new(&store).capture("Bike lock code 4711", None, None)?;
        let fake = Fake::new(vec![
            tool_use("find_notes", json!({"words": ["bike"]})),
            tool_use("list_new", json!({"since": "2026-01-01"})),
            tool_use("list_places", json!({})),
            end_turn("4711."),
        ]);
        ask(&store, &fake, anthropic(), "Bike lock?")?;
        assert_eq!(store.ai_task()?, None);
        assert!(!store.sync_paused()?);
        Ok(())
    }

    #[test]
    fn a_change_the_model_makes_waits_as_an_ai_task_and_sync_waits_with_it() -> TestResult {
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
        assert!(store.sync_paused()?);
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
        assert!(!store.sync_paused()?);
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
        let calls = |id: &str, args: &str| {
            json!({"choices": [{"finish_reason": "tool_calls", "message": {
                "content": null,
                "role": "assistant",
                "tool_calls": [{"function": {"arguments": args, "name": "find_notes"}, "id": id, "type": "function"}],
            }}]})
        };
        let fake = Fake::new(vec![
            calls("c1", "{words: wifi"),
            calls("c2", r#"{"words": ["wifi"]}"#),
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
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], "In the blue folder.");
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
    fn the_links_of_an_answer_name_its_notes_once_in_order() {
        let text = "See [a](oldbrainfix://note/note:a), **[b](oldbrainfix://note/note:b)** and [a again](oldbrainfix://note/note:a), not [web](https://example.com).";
        assert_eq!(linked_notes(text), ["note:a", "note:b"]);
    }
}
