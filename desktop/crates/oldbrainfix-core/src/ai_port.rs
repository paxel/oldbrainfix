//! The AI port (#112, ADR 0026): an MCP server for one AI Task at a time,
//! opened by the user, off otherwise. Plain HTTP on the loopback, or for
//! AI clients in Docker containers on the Docker bridge (#167), `POST
//! /mcp` with one JSON-RPC message per request and the answer as JSON
//! (MCP's Streamable HTTP without a stream), every request with the
//! port's token as `Authorization: Bearer`. A request without it, from a
//! web page's origin or naming a `Host` the choice does not accept, is
//! refused before anything is read from the store.
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
use crate::schedule::WallClock;
use crate::store::Store;
use crate::{Error, Result};

/// The local setting holding the port's token: kept, so a client set up
/// once stays set up.
pub const TOKEN_KEY: &str = "ai:token";
/// The local setting holding the port number last bound, tried first.
pub const PORT_KEY: &str = "ai:port";
/// The local setting holding who may connect ([`Reach`]).
pub const REACH_KEY: &str = "ai:reach";
/// What containers call the computer they run on.
pub const DOCKER_HOST: &str = "host.docker.internal";
/// The `Host` names the loopback answers to.
const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];
/// The one path the port answers on.
pub const MCP_PATH: &str = "/mcp";
/// The MCP versions this port speaks, the newest first.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// The name the port gives itself, and the client config uses.
pub const SERVER_NAME: &str = "oldbrainfix";

/// What the port tells the client about itself at `initialize`.
const INSTRUCTIONS: &str = "Old Brain Fix is a memory aid. Through this port you work on the user's Notes, Areas, Topics, Routines and Reminders for one task, and set up Tallies, Sessions, Checklists, Shopping Lists and Frogs. Every change you make waits as a Pending Change: the user reviews all of them together when the task is done and accepts or rejects them. What the user recorded, a Tally's entries, the ticks of Lines and Boxes, the check-offs, a Session's picks and the eaten Frogs, you only read. Nothing reaches other people.";

/// A tool call waiting for its answer.
#[derive(Debug)]
pub struct AiCall {
    /// The tool's name.
    pub name: String,
    pub arguments: Value,
    reply: Sender<std::result::Result<Value, String>>,
}

impl AiCall {
    /// Answers the call for the AI Task `task` ([`Store::ai_tool`]), on
    /// this computer's clock `wall`.
    pub fn answer(self, store: &Store, task: &str, wall: &WallClock) {
        let out = store
            .ai_tool(task, &self.name, &self.arguments, wall)
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

/// Who may connect to the port, a local setting of this Device (#167).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Reach {
    /// Programs on this computer, through 127.0.0.1.
    #[default]
    Local,
    /// AI clients in Docker containers on this computer, as
    /// `host.docker.internal`.
    Docker,
}

impl Reach {
    /// The name the setting stores.
    pub fn name(self) -> &'static str {
        match self {
            Reach::Local => "local",
            Reach::Docker => "docker",
        }
    }

    /// The choice a stored name stands for; anything else is
    /// [`Reach::Local`].
    pub fn from_name(name: Option<&str>) -> Reach {
        match name {
            Some("docker") => Reach::Docker,
            _ => Reach::Local,
        }
    }

    /// What the port binds and accepts for this choice, `bridge` being
    /// this computer's address on the Docker bridge, if it has one.
    pub fn endpoint(self, bridge: Option<Ipv4Addr>) -> Result<Endpoint> {
        match self {
            Reach::Local => Ok(Endpoint::local()),
            Reach::Docker => Endpoint::docker(bridge),
        }
    }
}

/// How a [`Reach`] turns into a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// The address the port binds.
    pub bind: IpAddr,
    /// The `Host` names the port accepts (any port number); every other
    /// one is refused, against DNS rebinding.
    pub hosts: Vec<String>,
    /// The host a client puts into the URL.
    pub client_host: String,
}

impl Endpoint {
    /// The loopback: `localhost`, `127.0.0.1` and `::1`, and no
    /// container's `host.docker.internal`.
    pub fn local() -> Endpoint {
        Endpoint {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            hosts: LOOPBACK_HOSTS.iter().map(|h| (*h).to_string()).collect(),
            client_host: "127.0.0.1".into(),
        }
    }

    /// On Linux containers reach the computer through the bridge
    /// (`docker0`), which alone is bound; Docker Desktop on macOS and
    /// Windows forwards `host.docker.internal` to 127.0.0.1.
    pub fn docker(bridge: Option<Ipv4Addr>) -> Result<Endpoint> {
        let ip = if cfg!(target_os = "linux") {
            bridge.ok_or(Error::NoDockerBridge)?
        } else {
            Ipv4Addr::LOCALHOST
        };
        Ok(Endpoint {
            bind: IpAddr::V4(ip),
            hosts: vec![ip.to_string(), DOCKER_HOST.to_string()],
            client_host: DOCKER_HOST.into(),
        })
    }

    /// Where a client reaches the port at `port`.
    pub fn url(&self, port: u16) -> String {
        format!("http://{}:{port}{MCP_PATH}", self.client_host)
    }
}

/// This computer's address on the Docker bridge, read from `/proc`
/// (Linux only; elsewhere none).
pub fn docker_bridge() -> Option<Ipv4Addr> {
    let route = std::fs::read_to_string("/proc/net/route").ok()?;
    let trie = std::fs::read_to_string("/proc/net/fib_trie").ok()?;
    bridge_address(&route, &trie, "docker0")
}

/// The local address that lies in the network routed through `iface`:
/// `route` is `/proc/net/route` (little-endian hex), `trie` is
/// `/proc/net/fib_trie`.
pub fn bridge_address(route: &str, trie: &str, iface: &str) -> Option<Ipv4Addr> {
    let hex = |s: &str| u32::from_str_radix(s, 16).ok().map(u32::from_be);
    // The interface's own network, not a default route through it (mask 0).
    let (net, mask) = route.lines().skip(1).find_map(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.first() != Some(&iface) {
            return None;
        }
        let (net, mask) = (hex(fields.get(1)?)?, hex(fields.get(7)?)?);
        (mask != 0).then_some((net, mask))
    })?;
    let mut last: Option<Ipv4Addr> = None;
    for line in trie.lines() {
        let line = line.trim_start();
        if let Some(addr) = line.strip_prefix("|-- ") {
            last = addr.trim().parse().ok();
        } else if line.contains("host LOCAL")
            && let Some(ip) = last
            && u32::from(ip) & mask == net
        {
            return Some(ip);
        }
    }
    None
}

/// The open port.
pub struct AiPort {
    task: String,
    port: u16,
    client_host: String,
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

    /// Who may connect to the port; [`Reach::Local`] until chosen.
    pub fn ai_reach(&self) -> Result<Reach> {
        Ok(Reach::from_name(self.local_setting(REACH_KEY)?.as_deref()))
    }

    pub fn set_ai_reach(&self, reach: Reach) -> Result<()> {
        self.set_local_setting(REACH_KEY, reach.name())
    }
}

impl AiPort {
    /// Opens the port for whoever may connect ([`Store::ai_reach`]), for
    /// the AI Task that waits, or a new one ([`Store::open_ai_task`]): on
    /// the port it had last time while that is free, else on a free one,
    /// which is kept. The Docker choice without a Docker bridge opens
    /// nothing ([`Error::NoDockerBridge`]).
    pub fn open(store: &Store) -> Result<AiPort> {
        let reach = store.ai_reach()?;
        let bridge = match reach {
            Reach::Docker if cfg!(target_os = "linux") => docker_bridge(),
            _ => None,
        };
        AiPort::open_at(store, &reach.endpoint(bridge)?)
    }

    /// Opens the port on `endpoint`, as [`AiPort::open`] does.
    pub fn open_at(store: &Store, endpoint: &Endpoint) -> Result<AiPort> {
        let task = store.open_ai_task()?;
        let token = store.ai_port_token()?;
        let kept: u16 = store
            .local_setting(PORT_KEY)?
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        let bind_error = |e: std::io::Error| Error::Invalid(format!("bind: {e}"));
        let bind = endpoint.bind;
        let listener = TcpListener::bind(SocketAddr::new(bind, kept))
            .or_else(|_| TcpListener::bind(SocketAddr::new(bind, 0)))
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
            let hosts = endpoint.hosts.clone();
            std::thread::spawn(move || accept_loop(listener, tx, digest, hosts, stop))
        };
        Ok(AiPort {
            task,
            port,
            client_host: endpoint.client_host.clone(),
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
        format!("http://{}:{}{MCP_PATH}", self.client_host, self.port)
    }

    /// The next tool call, if one waits.
    pub fn next_call(&self) -> Option<AiCall> {
        self.calls.try_recv().ok()
    }

    /// The next tool call, waiting up to `wait` for one.
    pub fn wait_call(&self, wait: Duration) -> Option<AiCall> {
        self.calls.recv_timeout(wait).ok()
    }

    /// Answers every tool call that waits, on this computer's clock
    /// `wall`; returns how many.
    pub fn answer_waiting(&self, store: &Store, wall: &WallClock) -> usize {
        let mut answered = 0;
        while let Some(call) = self.next_call() {
            call.answer(store, &self.task, wall);
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

fn accept_loop(
    listener: TcpListener,
    tx: Sender<AiCall>,
    digest: Vec<u8>,
    hosts: Vec<String>,
    stop: Arc<AtomicBool>,
) {
    let digest = Arc::new(digest);
    let hosts = Arc::new(hosts);
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let tx = tx.clone();
                let digest = digest.clone();
                let hosts = hosts.clone();
                std::thread::spawn(move || {
                    drop(serve_connection(stream, peer, &tx, &digest, &hosts));
                });
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

/// Whether a request's `Host`, without its port number, is one of
/// `hosts`: a page whose name was pointed at this computer (DNS
/// rebinding) names its own host and is refused. A request without one
/// is refused too.
fn allowed_host(host: Option<&str>, hosts: &[String]) -> bool {
    let Some(host) = host.map(str::trim) else {
        return false;
    };
    let name = match host.strip_prefix('[') {
        // An IPv6 address, as `[::1]:port`.
        Some(rest) => rest.split_once(']').map_or(rest, |(name, _)| name),
        None => match host.rsplit_once(':') {
            Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
            _ => host,
        },
    };
    hosts.iter().any(|h| h.eq_ignore_ascii_case(name))
}

/// One client's request: checked, answered, and the connection closed.
fn serve_connection(
    stream: TcpStream,
    peer: SocketAddr,
    tx: &Sender<AiCall>,
    digest: &[u8],
    hosts: &[String],
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;
    let mut reader = BufReader::new(stream);
    let response = match read_request(&mut reader, peer.ip()) {
        Ok(Some(request)) => {
            if !allowed_host(request.header("host"), hosts) {
                Response::text(403, "host")
            } else if !local_origin(request.header("origin")) {
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

    const ROUTE: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
wlan0\t0002A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\t0\t0\t0
";

    const TRIE: &str = "\
Main:
  +-- 0.0.0.0/0 3 0 5
     |-- 0.0.0.0
        /0 universe UNICAST
     +-- 172.17.0.0/16 2 0 2
        +-- 172.17.0.0/30 2 0 2
           |-- 172.17.0.0
              /16 link UNICAST
           |-- 172.17.0.1
              /32 host LOCAL
        |-- 172.17.255.255
           /32 link BROADCAST
     +-- 192.168.2.0/24 2 0 2
        |-- 192.168.2.40
           /32 host LOCAL
";

    /// A computer without Docker: only the wireless network.
    const ROUTE_NO_DOCKER: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0
wlan0\t0002A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\t0\t0\t0
";

    const TRIE_NO_DOCKER: &str = "\
Main:
  +-- 0.0.0.0/0 3 0 5
     |-- 0.0.0.0
        /0 universe UNICAST
     +-- 192.168.2.0/24 2 0 2
        |-- 192.168.2.40
           /32 host LOCAL
";

    #[test]
    fn the_bridge_address_comes_from_route_and_trie() {
        assert_eq!(
            bridge_address(ROUTE, TRIE, "docker0"),
            Some(Ipv4Addr::new(172, 17, 0, 1))
        );
        assert_eq!(
            bridge_address(ROUTE, TRIE, "wlan0"),
            Some(Ipv4Addr::new(192, 168, 2, 40))
        );
        assert_eq!(
            bridge_address(ROUTE_NO_DOCKER, TRIE_NO_DOCKER, "docker0"),
            None
        );
        assert_eq!(bridge_address(ROUTE, TRIE_NO_DOCKER, "docker0"), None);
        assert_eq!(bridge_address("", "", "docker0"), None);
    }

    #[test]
    fn who_may_connect_is_kept_and_this_computer_only_until_chosen() -> TestResult {
        let store = store();
        assert_eq!(store.ai_reach()?, Reach::Local);
        store.set_ai_reach(Reach::Docker)?;
        assert_eq!(store.ai_reach()?, Reach::Docker);
        assert_eq!(store.local_setting(REACH_KEY)?.as_deref(), Some("docker"));
        store.set_local_setting(REACH_KEY, "elsewhere")?;
        assert_eq!(store.ai_reach()?, Reach::Local);
        Ok(())
    }

    #[test]
    fn each_choice_binds_and_accepts_its_own_hosts() -> TestResult {
        let hosts = |e: &Endpoint, host: &str| allowed_host(Some(host), &e.hosts);
        let local = Reach::Local.endpoint(Some(Ipv4Addr::new(172, 17, 0, 1)))?;
        assert_eq!(local, Endpoint::local());
        assert_eq!(local.bind, IpAddr::V4(Ipv4Addr::LOCALHOST));
        for passes in ["localhost", "127.0.0.1:7342", "[::1]:7342", "LOCALHOST:1"] {
            assert!(hosts(&local, passes), "{passes}");
        }
        for refused in [
            "host.docker.internal:7342",
            "172.17.0.1:7342",
            "attacker.example",
            "127.0.0.1.attacker.example",
            "",
        ] {
            assert!(!hosts(&local, refused), "{refused}");
        }
        assert!(!allowed_host(None, &local.hosts));

        let bridge = Ipv4Addr::new(172, 17, 0, 1);
        let docker = Reach::Docker.endpoint(Some(bridge))?;
        assert_eq!(docker.client_host, DOCKER_HOST);
        assert!(hosts(&docker, "host.docker.internal:7342"));
        for refused in ["localhost", "::1", "attacker.example:7342"] {
            assert!(!hosts(&docker, refused), "{refused}");
        }
        if cfg!(target_os = "linux") {
            // Only the bridge, not every address of this computer.
            assert_eq!(docker.bind, IpAddr::V4(bridge));
            assert!(hosts(&docker, "172.17.0.1:7342"));
            assert!(!hosts(&docker, "127.0.0.1"));
            assert!(matches!(
                Reach::Docker.endpoint(None),
                Err(Error::NoDockerBridge)
            ));
        } else {
            // Docker Desktop forwards the name to the loopback.
            assert_eq!(docker.bind, IpAddr::V4(Ipv4Addr::LOCALHOST));
            assert!(Reach::Docker.endpoint(None).is_ok());
        }
        Ok(())
    }

    #[test]
    fn the_copied_commands_use_the_address_of_the_choice() -> TestResult {
        let token = "abc";
        let local = Endpoint::local().url(7342);
        assert_eq!(local, "http://127.0.0.1:7342/mcp");
        let docker = Reach::Docker
            .endpoint(Some(Ipv4Addr::new(172, 17, 0, 1)))?
            .url(7342);
        assert_eq!(docker, "http://host.docker.internal:7342/mcp");
        for (url, other) in [(&local, &docker), (&docker, &local)] {
            for text in [
                client_config(url, token),
                claude_code_command(url, token),
                bridged_client_config(url, token),
            ] {
                assert!(text.contains(url.as_str()), "{text}");
                assert!(!text.contains(other.as_str()), "{text}");
            }
        }
        Ok(())
    }

    /// The status line of a ping sent with this `Host` header and the
    /// token: ureq always sends the URL's own.
    fn status_for_host(port: &AiPort, host: &str) -> Made<String> {
        use std::io::Read;
        let body = rpc("ping", json!({})).to_string();
        let mut stream = TcpStream::connect(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            port.port(),
        ))?;
        write!(
            stream,
            "POST /mcp HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
            port.token(),
            body.len()
        )?;
        let mut raw = String::new();
        stream.read_to_string(&mut raw)?;
        Ok(raw.chars().take(12).collect())
    }

    #[test]
    fn a_container_gets_in_only_when_docker_containers_may_connect() -> TestResult {
        let store = store();
        let mut local = AiPort::open(&store)?;
        let number = local.port();
        assert_eq!(
            status_for_host(&local, &format!("{DOCKER_HOST}:{number}"))?,
            "HTTP/1.1 403"
        );
        assert_eq!(status_for_host(&local, "attacker.example")?, "HTTP/1.1 403");
        assert_eq!(
            status_for_host(&local, &format!("localhost:{number}"))?,
            "HTTP/1.1 200"
        );
        local.close();

        // The Docker choice as Docker Desktop has it, on the loopback:
        // this sandbox need not have a bridge.
        let docker = Endpoint {
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            hosts: vec!["127.0.0.1".into(), DOCKER_HOST.into()],
            client_host: DOCKER_HOST.into(),
        };
        let port = AiPort::open_at(&store, &docker)?;
        assert_eq!(port.url(), format!("http://{DOCKER_HOST}:{number}/mcp"));
        assert_eq!(
            status_for_host(&port, &format!("{DOCKER_HOST}:{number}"))?,
            "HTTP/1.1 200"
        );
        assert_eq!(status_for_host(&port, "attacker.example")?, "HTTP/1.1 403");
        assert_eq!(status_for_host(&port, "localhost")?, "HTTP/1.1 403");
        Ok(())
    }

    #[test]
    fn without_a_bridge_the_docker_choice_opens_nothing() -> TestResult {
        let store = store();
        store.set_ai_reach(Reach::Docker)?;
        if !cfg!(target_os = "linux") || docker_bridge().is_some() {
            // A bridge here (or Docker Desktop): nothing to show.
            return Ok(());
        }
        assert!(matches!(AiPort::open(&store), Err(Error::NoDockerBridge)));
        assert_eq!(store.ai_task()?, None, "no AI Task was opened");
        assert!(!store.sync_paused()?);
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
                    call.answer(&store, port.task(), &|t: chrono::DateTime<chrono::Utc>| {
                        t.naive_utc()
                    });
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
