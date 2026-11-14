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
pub const SYSTEM: &str = "You answer questions about the user's own Notes in Old Brain Fix, a memory aid. Find what you need with the tools: search with find_notes several times, with different words, synonyms and spellings in the user's language, use list_new for what came in since a time, and read the Notes you use with read_note. Answer briefly and only from the Notes you read with read_note during this question, never from general knowledge or a guess: an excerpt find_notes or list_new gives, or a Note an earlier answer named, counts only once you read that Note with read_note in this question; the Areas, Topics, tags, Routines and check-offs the other tools list count too. Link the Note of every statement you take from a Note as a Markdown link with its title as the label and oldbrainfix://note/ followed by its id as the address, such as [Dentist](oldbrainfix://note/note:3f2a); a link to a Note you did not read in this question is removed. When the Notes you read hold nothing about it, answer only \"That's not in your Notes\", translated into the language of the question. Answer in the language of the question. Change something only when the user asks for it, such as filing what is new into Areas and Topics or adding tags to a text: every change waits for the user's review in the app, so say what you changed and that it waits for review, linking only the Notes you read. Notes the user hid from the AI do not exist for you.";

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
        for (call, output) in done {
            let key = match call.name.as_str() {
                "read_note" => "id",
                "read_attachment_text" => "note",
                _ => continue,
            };
            if !output.is_error
                && let Some(id) = call.args.get(key).and_then(Value::as_str)
                && id.starts_with("note:")
                && !self.read.iter().any(|r| r == id)
            {
                self.read.push(id.to_string());
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
mod tests;
