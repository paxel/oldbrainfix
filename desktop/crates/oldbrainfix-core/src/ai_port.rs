//! The AI port (#112, ADR 0026): an MCP server for one AI Task at a time,
//! opened by the user, off otherwise. Plain HTTP on the loopback only,
//! `POST /mcp` with one JSON-RPC message per request and the answer as
//! JSON (MCP's Streamable HTTP without a stream), every request with the
//! port's token as `Authorization: Bearer`. A request without it, or from
//! a web page's origin, is refused before anything is read from the
//! store.
//!
//! As the LAN host does, the network runs on its own threads and never
//! touches the store: `initialize`, `ping` and `tools/list` are answered
//! there; every `tools/call` crosses a channel to whoever polls
//! [`AiPort::next_call`], who answers it with [`AiCall::answer`] (or asks
//! the user first) and hands the result back.

use std::io::{BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ai_tools::tools;
use crate::lan::{Response, read_request};
use crate::store::Store;
use crate::{Error, Result};

/// The local setting holding the port's token: kept, so a client set up
/// once stays set up.
pub const TOKEN_KEY: &str = "ai:token";
/// The local setting holding the port number last bound, tried first.
pub const PORT_KEY: &str = "ai:port";
/// The one path the port answers on.
pub const MCP_PATH: &str = "/mcp";
/// The MCP versions this port speaks, the newest first.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// The name the port gives itself, and the client config uses.
pub const SERVER_NAME: &str = "oldbrainfix";

/// What the port tells the client about itself at `initialize`.
const INSTRUCTIONS: &str = "Old Brain Fix is a memory aid. Through this port you work on the user's Notes, Areas, Topics, Routines and Reminders for one task. Every change you make waits as a Pending Change: the user reviews all of them together when the task is done and accepts or rejects them. Nothing reaches other people.";

/// A tool call waiting for its answer.
#[derive(Debug)]
pub struct AiCall {
    /// The tool's name.
    pub name: String,
    pub arguments: Value,
    reply: Sender<std::result::Result<Value, String>>,
}

impl AiCall {
    /// Answers the call for the AI Task `task` ([`Store::ai_tool`]).
    pub fn answer(self, store: &Store, task: &str) {
        let out = store
            .ai_tool(task, &self.name, &self.arguments)
            .map(|output| output.to_json())
            .map_err(|e| e.to_string());
        drop(self.reply.send(out));
    }

    /// Answers the call as a tool that did nothing, saying `why`.
    pub fn refuse(self, why: &str) {
        let out = json!({"content": [{"text": why, "type": "text"}], "isError": true});
        drop(self.reply.send(Ok(out)));
    }
}

/// The open port.
pub struct AiPort {
    task: String,
    port: u16,
    token: String,
    calls: Receiver<AiCall>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Store {
    /// The port's token, made the first time it is asked for: 32 random
    /// bytes, hex.
    pub fn ai_port_token(&self) -> Result<String> {
        if let Some(token) = self.local_setting(TOKEN_KEY)?
            && !token.is_empty()
        {
            return Ok(token);
        }
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| Error::Invalid(format!("random: {e}")))?;
        let token = hex::encode(bytes);
        self.set_local_setting(TOKEN_KEY, &token)?;
        Ok(token)
    }
}

impl AiPort {
    /// Opens the port on the loopback for the AI Task that waits, or a
    /// new one ([`Store::open_ai_task`]): on the port it had last time
    /// while that is free, else on a free one, which is kept.
    pub fn open(store: &Store) -> Result<AiPort> {
        let task = store.open_ai_task()?;
        let token = store.ai_port_token()?;
        let kept: u16 = store
            .local_setting(PORT_KEY)?
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        let bind_error = |e: std::io::Error| Error::Invalid(format!("bind: {e}"));
        let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let listener = TcpListener::bind(SocketAddr::new(loopback, kept))
            .or_else(|_| TcpListener::bind(SocketAddr::new(loopback, 0)))
            .map_err(bind_error)?;
        listener.set_nonblocking(true).map_err(bind_error)?;
        let port = listener.local_addr().map_err(bind_error)?.port();
        if port != kept {
            store.set_local_setting(PORT_KEY, &port.to_string())?;
        }
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = stop.clone();
            let digest = Sha256::digest(token.as_bytes()).to_vec();
            std::thread::spawn(move || accept_loop(listener, tx, digest, stop))
        };
        Ok(AiPort {
            task,
            port,
            token,
            calls: rx,
            stop,
            thread: Some(thread),
        })
    }

    /// The id of the AI Task the port writes into.
    pub fn task(&self) -> &str {
        &self.task
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Where a client reaches the port.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}{MCP_PATH}", self.port)
    }

    /// The next tool call, if one waits.
    pub fn next_call(&self) -> Option<AiCall> {
        self.calls.try_recv().ok()
    }

    /// The next tool call, waiting up to `wait` for one.
    pub fn wait_call(&self, wait: Duration) -> Option<AiCall> {
        self.calls.recv_timeout(wait).ok()
    }

    /// Answers every tool call that waits; returns how many.
    pub fn answer_waiting(&self, store: &Store) -> usize {
        let mut answered = 0;
        while let Some(call) = self.next_call() {
            call.answer(store, &self.task);
            answered += 1;
        }
        answered
    }

    /// Stops accepting; the AI Task waits for its review.
    pub fn close(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

impl Drop for AiPort {
    fn drop(&mut self) {
        self.close();
    }
}

/// The client config most MCP clients read (`mcpServers`, as in Claude
/// Code's `.mcp.json`), for the port at `url` with `token`.
pub fn client_config(url: &str, token: &str) -> String {
    let config = json!({
        "mcpServers": {
            SERVER_NAME: {
                "headers": {"Authorization": format!("Bearer {token}")},
                "type": "http",
                "url": url,
            },
        },
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

/// The command that adds the port to Claude Code.
pub fn claude_code_command(url: &str, token: &str) -> String {
    format!(
        "claude mcp add --transport http {SERVER_NAME} {url} --header \"Authorization: Bearer {token}\""
    )
}

/// The config for a client that starts its servers itself, such as
/// Claude Desktop: the port reached through `mcp-remote`.
pub fn bridged_client_config(url: &str, token: &str) -> String {
    let config = json!({
        "mcpServers": {
            SERVER_NAME: {
                "args": ["mcp-remote", url, "--header", format!("Authorization: Bearer {token}")],
                "command": "npx",
            },
        },
    });
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

fn accept_loop(listener: TcpListener, tx: Sender<AiCall>, digest: Vec<u8>, stop: Arc<AtomicBool>) {
    let digest = Arc::new(digest);
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let tx = tx.clone();
                let digest = digest.clone();
                std::thread::spawn(move || drop(serve_connection(stream, peer, &tx, &digest)));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// Whether `header` carries the token whose SHA-256 is `digest`: the
/// digests are compared, so the time taken tells nothing of the token.
fn holds_token(header: Option<&str>, digest: &[u8]) -> bool {
    let Some(token) = header.and_then(|h| h.strip_prefix("Bearer ")) else {
        return false;
    };
    let given = Sha256::digest(token.trim().as_bytes());
    given
        .iter()
        .zip(digest)
        .fold(given.len() ^ digest.len(), |diff, (a, b)| {
            diff | usize::from(a ^ b)
        })
        == 0
}

/// Whether a request's `Origin`, if it has one, is the loopback's: a web
/// page elsewhere must not reach the port through the user's browser.
fn local_origin(origin: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    let host = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or("");
    let host = match host.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => host,
        _ => host,
    };
    matches!(host, "127.0.0.1" | "localhost" | "[::1]")
}

/// One client's request: checked, answered, and the connection closed.
fn serve_connection(
    stream: TcpStream,
    peer: SocketAddr,
    tx: &Sender<AiCall>,
    digest: &[u8],
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;
    let mut reader = BufReader::new(stream);
    let response = match read_request(&mut reader, peer.ip()) {
        Ok(Some(request)) => {
            if !local_origin(request.header("origin")) {
                Response::text(403, "origin")
            } else if !holds_token(request.header("authorization"), digest) {
                let mut refused = Response::text(401, "token");
                refused
                    .headers
                    .push(("WWW-Authenticate".into(), "Bearer".into()));
                refused
            } else if request.path != MCP_PATH {
                Response::status(404)
            } else if request.method != "POST" {
                let mut refused = Response::status(405);
                refused.headers.push(("Allow".into(), "POST".into()));
                refused
            } else {
                answer(&request.body, tx)
            }
        }
        Ok(None) => Response::status(413),
        Err(_) => Response::status(400),
    };
    let stream = reader.get_mut();
    response.write(stream)?;
    stream.flush()
}

fn rpc_result(id: &Value, result: Value) -> Response {
    Response::json(200, &json!({"id": id, "jsonrpc": "2.0", "result": result}))
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Response {
    Response::json(
        200,
        &json!({"error": {"code": code, "message": message}, "id": id, "jsonrpc": "2.0"}),
    )
}

/// The answer to one JSON-RPC message.
fn answer(body: &[u8], tx: &Sender<AiCall>) -> Response {
    let Ok(message) = serde_json::from_slice::<Value>(body) else {
        return Response::json(
            400,
            &json!({"error": {"code": -32700, "message": "parse error"}, "id": null, "jsonrpc": "2.0"}),
        );
    };
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A response or something else no answer is owed for.
        return Response::status(202);
    };
    let Some(id) = message.get("id") else {
        // A notification.
        return Response::status(202);
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str);
            let version = asked
                .filter(|v| PROTOCOL_VERSIONS.contains(v))
                .unwrap_or(PROTOCOL_VERSIONS[0]);
            rpc_result(
                id,
                json!({
                    "capabilities": {"tools": {"listChanged": false}},
                    "instructions": INSTRUCTIONS,
                    "protocolVersion": version,
                    "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")},
                }),
            )
        }
        "ping" => rpc_result(id, json!({})),
        "tools/list" => rpc_result(id, json!({"tools": tools()})),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return rpc_error(id, -32602, "no tool named");
            };
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let (reply, replied) = channel();
            let call = AiCall {
                name: name.to_string(),
                arguments,
                reply,
            };
            if tx.send(call).is_err() {
                return rpc_error(id, -32603, "the port is closing");
            }
            match replied.recv() {
                Ok(Ok(result)) => rpc_result(id, result),
                Ok(Err(why)) => rpc_error(id, -32602, &why),
                Err(_) => rpc_error(id, -32603, "the call was not answered"),
            }
        }
        _ => rpc_error(id, -32601, "no such method"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::Notes;
    use crate::store::tests::store;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn agent() -> ureq::Agent {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build();
        ureq::Agent::new_with_config(config)
    }

    /// Posts `message` to the port with `token`; the status and the body.
    fn post(url: &str, token: Option<&str>, message: &Value) -> Made<(u16, String)> {
        let mut request = agent().post(url).header("Content-Type", "application/json");
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let mut res = request.send(message.to_string())?;
        Ok((res.status().as_u16(), res.body_mut().read_to_string()?))
    }

    fn rpc(method: &str, params: Value) -> Value {
        json!({"id": 1, "jsonrpc": "2.0", "method": method, "params": params})
    }

    #[test]
    fn the_port_is_on_the_loopback_and_answers_nothing_without_its_token() -> TestResult {
        let store = store();
        Notes::new(&store).capture("Milk", None, None)?;
        let port = AiPort::open(&store)?;
        assert!(port.url().starts_with("http://127.0.0.1:"));
        assert_eq!(store.ai_task()?.as_deref(), Some(port.task()));
        let call = rpc("tools/call", json!({"name": "list_notes", "arguments": {}}));
        let (url, token) = (port.url(), port.token().to_string());
        let refused = || -> Made<Vec<u16>> {
            let mut res = agent()
                .post(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Origin", "https://example.com")
                .send(call.to_string())?;
            drop(res.body_mut().read_to_string());
            Ok(vec![
                post(&url, None, &call)?.0,
                post(&url, Some("guess"), &call)?.0,
                post(&url, Some(&format!("{token}0")), &call)?.0,
                res.status().as_u16(),
            ])
        };

        // A call that crosses to the store anyway is counted, and turned
        // away so the client does not wait.
        let (crossed, statuses) = std::thread::scope(|s| {
            let client = s.spawn(|| refused().map_err(|e| e.to_string()));
            let mut crossed = 0;
            while !client.is_finished() {
                if let Some(call) = port.wait_call(Duration::from_millis(20)) {
                    crossed += 1;
                    call.refuse("crossed");
                }
            }
            (crossed, client.join())
        });
        assert_eq!(statuses.map_err(|_| "the client")??, [401, 401, 401, 403]);
        assert_eq!(crossed, 0, "nothing crossed to the store");
        let (status, _) = post(&url, Some(&token), &rpc("ping", json!({})))?;
        assert_eq!(status, 200);
        Ok(())
    }

    #[test]
    fn the_port_keeps_its_token_and_port_for_the_next_task() -> TestResult {
        let store = store();
        let mut first = AiPort::open(&store)?;
        let (token, number) = (first.token().to_string(), first.port());
        first.close();
        let again = AiPort::open(&store)?;
        assert_eq!(again.token(), token);
        assert_eq!(again.port(), number);
        assert!(client_config(&again.url(), &token).contains(&token));
        Ok(())
    }

    /// The client's half of [`a_client_drives_the_tools_end_to_end`].
    fn client(url: &str, token: &str) -> Made<Vec<Value>> {
        let send = |message: Value| -> Made<Value> {
            let (status, body) = post(url, Some(token), &message)?;
            if status != 200 {
                return Err(format!("{status}: {body}").into());
            }
            Ok(serde_json::from_str(&body)?)
        };
        let call = |name: &str, arguments: Value| -> Made<Value> {
            let answer = send(rpc(
                "tools/call",
                json!({"arguments": arguments, "name": name}),
            ))?;
            let text = answer
                .pointer("/result/content/0/text")
                .and_then(Value::as_str)
                .ok_or("a text")?;
            Ok(serde_json::from_str(text).unwrap_or(Value::String(text.to_string())))
        };
        let mut seen = Vec::new();
        seen.push(send(rpc(
            "initialize",
            json!({"capabilities": {}, "clientInfo": {"name": "test", "version": "1"}, "protocolVersion": "2025-03-26"}),
        ))?);
        let (status, _) = post(
            url,
            Some(token),
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        )?;
        seen.push(json!(status));
        seen.push(send(rpc("tools/list", json!({})))?);
        let notes = call("list_notes", json!({"which": "unsorted"}))?;
        let milk = notes
            .pointer("/0/id")
            .and_then(Value::as_str)
            .ok_or("a Note")?
            .to_string();
        seen.push(call("edit_note", json!({"id": milk, "text": "Oat milk"}))?);
        let area = call("add_area", json!({"name": "Household"}))?;
        let area = area.get("id").and_then(Value::as_str).ok_or("an id")?;
        seen.push(call("file_note", json!({"id": milk, "place": area}))?);
        seen.push(call("read_note", json!({"id": milk}))?);
        seen.push(call("edit_note", json!({"id": "note:none", "text": "x"}))?);
        seen.push(send(rpc(
            "tools/call",
            json!({"name": "block_person", "arguments": {}}),
        ))?);
        Ok(seen)
    }

    #[test]
    fn a_client_drives_the_tools_end_to_end_and_only_accept_keeps_it() -> TestResult {
        let store = store();
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        let logged = store.all_entries()?;
        let port = AiPort::open(&store)?;
        let (url, token) = (port.url(), port.token().to_string());

        let seen = std::thread::scope(|s| {
            let client = s.spawn(|| client(&url, &token).map_err(|e| e.to_string()));
            while !client.is_finished() {
                if let Some(call) = port.wait_call(Duration::from_millis(20)) {
                    call.answer(&store, port.task());
                }
            }
            client.join()
        })
        .map_err(|_| "the client")??;

        assert_eq!(
            seen[0].pointer("/result/protocolVersion"),
            Some(&json!("2025-03-26"))
        );
        assert_eq!(seen[1], json!(202));
        let listed = seen[2]
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .ok_or("tools")?;
        assert_eq!(listed.len(), crate::ai_tools::tools().len());
        assert!(
            listed
                .iter()
                .all(|t| t.pointer("/inputSchema/type") == Some(&json!("object")))
        );
        let names: Vec<&str> = listed
            .iter()
            .filter_map(|t| t.get("name").and_then(Value::as_str))
            .collect();
        for absent in ["send_note", "share_area", "block_person", "set_setting"] {
            assert!(!names.contains(&absent));
        }
        assert_eq!(seen[3].get("id"), Some(&json!(milk.id)));
        assert_eq!(seen[5].get("text"), Some(&json!("Oat milk")));
        assert!(seen[5].get("place").is_some_and(Value::is_string));
        assert_eq!(seen[6], json!("not a Note: note:none"));
        assert_eq!(seen[7].pointer("/error/code"), Some(&json!(-32602)));

        assert_eq!(store.all_entries()?, logged, "the log saw nothing yet");
        assert!(store.sync_paused()?);
        let review = store.ai_review(port.task())?;
        assert_eq!(
            review.groups.len(),
            3,
            "filed, text changed, Areas: {review:?}"
        );
        let task = port.task().to_string();
        drop(port);
        assert_eq!(
            store.ai_task()?,
            Some(task.clone()),
            "closed, it waits for review"
        );
        store.accept_pending(&task)?;
        assert_eq!(
            Notes::new(&store).by_id(&milk.id)?.map(|n| n.text),
            Some("Oat milk".into())
        );
        assert_eq!(store.all_entries()?.len(), logged.len() + 4);
        assert!(!store.sync_paused()?);
        Ok(())
    }
}
