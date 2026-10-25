//! LAN sync as the desktop hosts it (ADR 0003), Catlog's in-person host
//! copied and adapted: a small HTTPS server with a self-signed
//! certificate made once per install, the pair code carrying its
//! fingerprint and a PIN, and the phone joining as the Dart core's
//! `lanSync` does: `GET /vector`, `POST /sync`, then the photos and
//! voice recordings under `/blob/<kind>/<hash>`. Every request carries
//! the PIN; an address is locked out after
//! [`PIN_FAILURES_PER_ADDRESS`] wrong ones, everyone after
//! [`PIN_FAILURES_TOTAL`].
//!
//! The network runs on its own threads and never touches the store:
//! every request that passed the PIN crosses a channel to whoever polls
//! [`Host::next_request`], who answers it with [`Store::serve`] (or
//! holds it while the user is asked whether to trust the Device) and
//! hands the response back.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::entry::WireEntry;
use crate::signing::{ImportReport, KeyRecord};
use crate::store::Store;
use crate::sync::{Moment, cause, split_blob_ref, wire};
use crate::{Error, Result};

/// The wire format this build speaks, the Dart core's `syncFormat`.
pub const SYNC_FORMAT: i64 = 1;

pub const PIN_FAILURES_PER_ADDRESS: u32 = 5;
pub const PIN_FAILURES_TOTAL: u32 = 20;

/// The most a sync body or a blob may weigh.
pub const MAX_ENTRIES_BYTES: usize = 64 << 20;
pub const MAX_BLOB_BYTES: usize = 64 << 20;

/// How much of the fingerprint a typed code carries.
pub const TYPED_FINGERPRINT_BYTES: usize = 8;
const FULL_FINGERPRINT_BYTES: usize = 32;

/// Local settings holding this Device's certificate and key, DER in
/// base64.
pub const TLS_CERT_KEY: &str = "tls:cert";
pub const TLS_KEY_KEY: &str = "tls:key";

pub const PIN_HEADER: &str = "x-oldbrainfix-pin";
pub const DEVICE_HEADER: &str = "x-oldbrainfix-device";
pub const FORMAT_HEADER: &str = "x-oldbrainfix-format";

// ---------------------------------------------------------------- pair code

/// Crockford base32, lowercase, no i/l/o/u, as the Dart core writes it.
const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// What a pair code names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairInfo {
    pub host: String,
    pub port: u16,
    pub pin: String,
    /// The certificate's fingerprint, or its first bytes from a typed
    /// code.
    pub fingerprint: Vec<u8>,
}

/// Host, port, PIN and fingerprint packed into base32, grouped in fives:
/// the QR carries the whole fingerprint, the typed code its first bytes.
pub fn encode_pair_code(
    host: Ipv4Addr,
    port: u16,
    pin: &str,
    fingerprint: &[u8],
    typed: bool,
) -> String {
    let pin_number: u32 = pin.parse().unwrap_or(0);
    let mut bytes = host.octets().to_vec();
    bytes.extend_from_slice(&port.to_be_bytes());
    bytes.extend_from_slice(&pin_number.to_be_bytes()[1..]);
    let take = if typed {
        TYPED_FINGERPRINT_BYTES.min(fingerprint.len())
    } else {
        fingerprint.len()
    };
    bytes.extend_from_slice(&fingerprint[..take]);
    to_base32(&bytes)
        .as_bytes()
        .chunks(5)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join("_")
}

/// The code's groups in lines of three.
pub fn pair_code_lines(code: &str) -> Vec<String> {
    code.split('_')
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|c| c.join("_"))
        .collect()
}

/// Reads a code as typed: spaces, underscores, dashes, case and the
/// classic confusions are forgiven; none when it is not a code.
pub fn decode_pair_code(input: &str) -> Option<PairInfo> {
    let cleaned: String = input
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .map(|c| match c {
            'i' | 'l' => '1',
            'o' => '0',
            'u' => 'v',
            other => other,
        })
        .collect();
    let bytes = from_base32(&cleaned)?;
    let (address_length, fp_length) = [4usize, 16]
        .iter()
        .flat_map(|a| {
            [TYPED_FINGERPRINT_BYTES, FULL_FINGERPRINT_BYTES]
                .into_iter()
                .map(move |fp| (*a, fp))
        })
        .find(|(a, fp)| ((a + 5 + fp) * 8).div_ceil(5) == cleaned.len())?;
    if bytes.len() < address_length + 5 + fp_length {
        return None;
    }
    let host = if address_length == 4 {
        Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]).to_string()
    } else {
        let mut o = [0u8; 16];
        o.copy_from_slice(&bytes[..16]);
        std::net::Ipv6Addr::from(o).to_string()
    };
    let port = u16::from_be_bytes([bytes[address_length], bytes[address_length + 1]]);
    let pin = (u32::from(bytes[address_length + 2]) << 16)
        | (u32::from(bytes[address_length + 3]) << 8)
        | u32::from(bytes[address_length + 4]);
    if port == 0 || pin > 999_999 {
        return None;
    }
    Some(PairInfo {
        host,
        port,
        pin: format!("{pin:06}"),
        fingerprint: bytes[address_length + 5..address_length + 5 + fp_length].to_vec(),
    })
}

fn to_base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            out.push(ALPHABET[((buffer >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn from_base32(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        let value = ALPHABET.iter().position(|a| *a == c)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            out.push(((buffer >> (bits - 8)) & 0xff) as u8);
            bits -= 8;
        }
        buffer &= (1 << bits) - 1;
    }
    Some(out)
}

// ------------------------------------------------------------------ identity

/// The certificate the host serves with and the key behind it; the
/// fingerprint is SHA-256 over the certificate's DER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
    pub fingerprint: Vec<u8>,
}

pub fn fingerprint_of(cert_der: &[u8]) -> Vec<u8> {
    Sha256::digest(cert_der).to_vec()
}

/// A fresh self-signed certificate, ECDSA P-256.
pub fn generate_identity() -> Result<Identity> {
    let made = rcgen::generate_simple_self_signed(vec!["oldbrainfix".to_string()])
        .map_err(|e| Error::Invalid(format!("certificate: {e}")))?;
    let cert_der = made.cert.der().to_vec();
    Ok(Identity {
        fingerprint: fingerprint_of(&cert_der),
        cert_der,
        key_der: made.key_pair.serialize_der(),
    })
}

impl Store {
    /// This Device's identity, made on first hosting and kept, so a
    /// paired phone knows it again.
    pub fn tls_identity(&self) -> Result<Identity> {
        if let (Some(cert), Some(key)) = (
            self.local_setting(TLS_CERT_KEY)?,
            self.local_setting(TLS_KEY_KEY)?,
        ) && let (Ok(cert_der), Ok(key_der)) = (BASE64.decode(cert), BASE64.decode(key))
        {
            return Ok(Identity {
                fingerprint: fingerprint_of(&cert_der),
                cert_der,
                key_der,
            });
        }
        let made = generate_identity()?;
        self.set_local_setting(TLS_CERT_KEY, &BASE64.encode(&made.cert_der))?;
        self.set_local_setting(TLS_KEY_KEY, &BASE64.encode(&made.key_der))?;
        Ok(made)
    }
}

// ---------------------------------------------------------------- requests

/// One request that passed the PIN, waiting for its answer.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub from: IpAddr,
    reply: Sender<Response>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Answers the joiner; a joiner gone meanwhile is nobody's problem.
    pub fn reply(self, response: Response) {
        drop(self.reply.send(response));
    }

    /// The Device asking to sync, for a `POST /sync`: its id and name.
    pub fn join_ask(&self) -> Option<JoinAsk> {
        if self.method != "POST" || self.path != "/sync" {
            return None;
        }
        let body: Value = serde_json::from_slice(&self.body).ok()?;
        Some(JoinAsk {
            device: body.get("deviceId")?.as_str()?.to_string(),
            name: body
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string(),
            proof: body
                .get("proof")
                .and_then(|p| p.as_str())
                .map(str::to_string),
        })
    }
}

/// A Device asking to sync: the id and name it gives, and its signature
/// over this host's id and the PIN ([`pairing_proof_bytes`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinAsk {
    pub device: String,
    pub name: String,
    pub proof: Option<String>,
}

/// The bytes a joiner signs to prove it is the Device it names, as the
/// Dart core's `pairingProofBytes`.
pub fn pairing_proof_bytes(host_device: &str, pin: &str) -> Vec<u8> {
    json!(["pair", host_device, pin]).to_string().into_bytes()
}

impl Store {
    /// Whether `ask` comes from a paired Device: one paired before whose
    /// pinned key signed the proof for this host and `pin`. A device id
    /// alone proves nothing; anyone with the PIN can read it.
    pub fn trusts_joiner(&self, ask: &JoinAsk, pin: &str) -> Result<bool> {
        if self.paired_device(&ask.device)?.is_none() {
            return Ok(false);
        }
        let Some(key) = self.pinned_key(&ask.device)? else {
            return Ok(false);
        };
        let bytes = pairing_proof_bytes(&self.device_id()?, pin);
        Ok(crate::signing::verify_signature(
            &key.record.public_key,
            &bytes,
            ask.proof.as_deref(),
        ))
    }
}

/// What goes back over the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub content_type: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn status(status: u16) -> Response {
        Response {
            status,
            content_type: None,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn text(status: u16, body: &str) -> Response {
        Response {
            body: body.as_bytes().to_vec(),
            ..Response::status(status)
        }
    }

    pub fn json(status: u16, value: &Value) -> Response {
        Response {
            content_type: Some("application/json".to_string()),
            body: serde_json::to_vec(value).unwrap_or_default(),
            ..Response::status(status)
        }
    }

    pub fn bytes(body: Vec<u8>) -> Response {
        Response {
            content_type: Some("application/octet-stream".to_string()),
            body,
            ..Response::status(200)
        }
    }

    /// The refusal a Device the user said no to hears.
    pub fn declined() -> Response {
        Response::text(403, "declined")
    }

    fn reason(&self) -> &'static str {
        match self.status {
            200 => "OK",
            403 => "Forbidden",
            404 => "Not Found",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }

    fn write(&self, out: &mut impl Write) -> std::io::Result<()> {
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status, self.reason());
        if let Some(ct) = &self.content_type {
            head.push_str(&format!("Content-Type: {ct}\r\n"));
        }
        for (k, v) in &self.headers {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        head.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            self.body.len()
        ));
        out.write_all(head.as_bytes())?;
        out.write_all(&self.body)?;
        out.flush()
    }
}

// -------------------------------------------------------------------- host

/// The server: a thread accepting joiners, one thread per connection,
/// every PIN-checked request handed over the channel.
pub struct Host {
    address: IpAddr,
    port: u16,
    pin: String,
    fingerprint: Vec<u8>,
    requests: Receiver<Request>,
    stop: Arc<AtomicBool>,
    locked: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// Wrong PINs so far, per address and in total.
#[derive(Default)]
struct Failures {
    per_address: HashMap<IpAddr, u32>,
    total: u32,
}

/// A six-digit PIN, never starting with zero.
pub fn new_pin() -> String {
    let mut random = [0u8; 4];
    getrandom::fill(&mut random).ok();
    let n = u32::from_be_bytes(random) % 900_000 + 100_000;
    n.to_string()
}

/// The address this machine has on its local network, found without
/// sending anything; none without a network.
pub fn lan_address() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("10.255.255.255:1").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

impl Host {
    /// Binds and starts serving on `bind`, or on the LAN address (the
    /// loopback without a network), on a free port.
    pub fn start(identity: &Identity, pin: &str, bind: Option<IpAddr>) -> Result<Host> {
        let address = bind
            .or_else(|| lan_address().map(IpAddr::V4))
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let config = server_config(identity)?;
        let bind_error = |e: std::io::Error| Error::Invalid(format!("bind: {e}"));
        let listener = TcpListener::bind(SocketAddr::new(address, 0)).map_err(bind_error)?;
        listener.set_nonblocking(true).map_err(bind_error)?;
        let port = listener.local_addr().map_err(bind_error)?.port();
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let locked = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(Gate {
            pin: pin.to_string(),
            failures: Mutex::new(Failures::default()),
            locked: locked.clone(),
        });
        let thread = {
            let stop = stop.clone();
            std::thread::spawn(move || accept_loop(listener, config, tx, gate, stop))
        };
        Ok(Host {
            address,
            port,
            pin: pin.to_string(),
            fingerprint: identity.fingerprint.clone(),
            requests: rx,
            stop,
            locked,
            thread: Some(thread),
        })
    }

    pub fn address(&self) -> IpAddr {
        self.address
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }

    pub fn fingerprint(&self) -> &[u8] {
        &self.fingerprint
    }

    /// True once too many wrong PINs arrived: nothing is answered any
    /// more; hosting must start again with a new PIN.
    pub fn locked_out(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// The pair code for this host: the whole fingerprint for a QR, its
    /// first bytes for typing.
    pub fn pair_code(&self, typed: bool) -> String {
        let host = match self.address {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => Ipv4Addr::LOCALHOST,
        };
        encode_pair_code(host, self.port, &self.pin, &self.fingerprint, typed)
    }

    /// The next request that passed the PIN, if one waits.
    pub fn next_request(&self) -> Option<Request> {
        self.requests.try_recv().ok()
    }

    /// Stops accepting; connections in flight finish on their own.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The PIN and the wrong guesses counted against it.
struct Gate {
    pin: String,
    failures: Mutex<Failures>,
    locked: Arc<AtomicBool>,
}

impl Gate {
    /// Whether this request may pass: `Ok` when it did, else the answer
    /// to send instead.
    fn check(&self, from: IpAddr, pin: Option<&str>) -> std::result::Result<(), Response> {
        let mut failures = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        let locked = failures.total >= PIN_FAILURES_TOTAL
            || failures.per_address.get(&from).copied().unwrap_or(0) >= PIN_FAILURES_PER_ADDRESS;
        if locked {
            return Err(Response::text(403, "locked"));
        }
        if pin != Some(self.pin.as_str()) {
            *failures.per_address.entry(from).or_insert(0) += 1;
            failures.total += 1;
            if failures.total >= PIN_FAILURES_TOTAL {
                self.locked.store(true, Ordering::Relaxed);
            }
            return Err(Response::status(403));
        }
        Ok(())
    }
}

fn server_config(identity: &Identity) -> Result<Arc<rustls::ServerConfig>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| Error::Invalid(format!("tls: {e}")))?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(identity.cert_der.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key_der.clone())),
        )
        .map_err(|e| Error::Invalid(format!("tls: {e}")))?;
    Ok(Arc::new(config))
}

fn accept_loop(
    listener: TcpListener,
    config: Arc<rustls::ServerConfig>,
    tx: Sender<Request>,
    gate: Arc<Gate>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let config = config.clone();
                let tx = tx.clone();
                let gate = gate.clone();
                std::thread::spawn(move || {
                    drop(serve_connection(stream, peer, config, tx, gate));
                });
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// One joiner's connection: the request read, PIN-checked, answered by
/// whoever polls the host, and the connection closed after it. The
/// answer may wait while the user is asked whether to trust the joiner.
fn serve_connection(
    stream: TcpStream,
    peer: SocketAddr,
    config: Arc<rustls::ServerConfig>,
    tx: Sender<Request>,
    gate: Arc<Gate>,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;
    let conn =
        rustls::ServerConnection::new(config).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut reader = BufReader::new(rustls::StreamOwned::new(conn, stream));
    let response = match read_request(&mut reader, peer.ip()) {
        Ok(Some(partial)) => match gate.check(partial.from, partial.header(PIN_HEADER)) {
            Err(refusal) => refusal,
            Ok(()) => {
                let (reply_tx, reply_rx) = channel();
                let request = Request {
                    reply: reply_tx,
                    ..partial
                };
                if tx.send(request).is_err() {
                    Response::status(503)
                } else {
                    reply_rx.recv().unwrap_or_else(|_| Response::status(503))
                }
            }
        },
        Ok(None) => Response::status(413),
        Err(_) => Response::status(500),
    };
    let stream = reader.get_mut();
    response.write(stream)?;
    stream.conn.send_close_notify();
    drop(stream.conn.complete_io(&mut stream.sock));
    Ok(())
}

/// Reads one request; `None` once its body exceeds the limit for its
/// path (the rest is not read).
fn read_request<R: BufRead>(reader: &mut R, from: IpAddr) -> std::io::Result<Option<Request>> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .to_string();
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_lowercase(), v.trim().to_string()));
        }
    }
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    let limit = if path.starts_with("/blob/") {
        MAX_BLOB_BYTES
    } else {
        MAX_ENTRIES_BYTES
    };
    let body = if header("transfer-encoding").is_some_and(|v| v.to_lowercase().contains("chunked"))
    {
        read_chunked(reader, limit)?
    } else {
        let length: usize = header("content-length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if length > limit {
            return Ok(None);
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body)?;
        Some(body)
    };
    let Some(body) = body else {
        return Ok(None);
    };
    let (reply, _) = channel();
    Ok(Some(Request {
        method,
        path,
        headers,
        body,
        from,
        reply,
    }))
}

/// A chunked body; `None` past the limit.
fn read_chunked<R: BufRead>(reader: &mut R, limit: usize) -> std::io::Result<Option<Vec<u8>>> {
    let mut body = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(std::io::Error::other("chunk cut short"));
        }
        let size_text = line.trim().split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| std::io::Error::other("bad chunk size"))?;
        if size == 0 {
            loop {
                let mut trailer = String::new();
                if reader.read_line(&mut trailer)? == 0 || trailer.trim().is_empty() {
                    break;
                }
            }
            return Ok(Some(body));
        }
        if body.len() + size > limit {
            return Ok(None);
        }
        let start = body.len();
        body.resize(start + size, 0);
        reader.read_exact(&mut body[start..])?;
        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf)?;
    }
}

// ------------------------------------------------------------- store side

/// What one side of a session brought in, as the Dart core's
/// `SyncSession`.
#[derive(Debug, Clone)]
pub struct Session {
    /// The other Device's id and the name it goes by.
    pub peer: String,
    pub name: String,
    pub applied: Vec<WireEntry>,
    pub moment: Option<Moment>,
    pub report: ImportReport,
}

fn vector_of(value: Option<&Value>) -> HashMap<String, i64> {
    value
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n)))
                .collect()
        })
        .unwrap_or_default()
}

fn entries_of(value: Option<&Value>) -> Vec<WireEntry> {
    value
        .and_then(|e| e.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|e| serde_json::from_value(e.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn keys_of(value: Option<&Value>) -> Vec<KeyRecord> {
    value
        .and_then(|k| k.as_array())
        .map(|list| list.iter().filter_map(KeyRecord::from_json).collect())
        .unwrap_or_default()
}

fn text_of(body: &Value, key: &str) -> String {
    body.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn keys_json(store: &Store) -> Result<Value> {
    Ok(json!(
        store
            .key_records()?
            .iter()
            .map(KeyRecord::to_json)
            .collect::<Vec<_>>()
    ))
}

/// The joiner's `POST /sync` body, as the Dart core's `syncRequest`,
/// with its proof over `host_device` and `pin`.
pub fn sync_request(
    store: &Store,
    host_vector: &HashMap<String, i64>,
    name: &str,
    host_device: &str,
    pin: &str,
) -> Result<Value> {
    Ok(json!({
        "format": SYNC_FORMAT,
        "deviceId": store.device_id()?,
        "name": name,
        "proof": store.signing_key()?.sign(&pairing_proof_bytes(host_device, pin)),
        "vector": store.version_vector()?,
        "entries": wire(&store.entries_since(host_vector)?),
        "keys": keys_json(store)?,
    }))
}

/// Takes in a joiner's `POST /sync` body and answers it, as the Dart
/// core's `answerSync`; none when the joiner speaks another format.
pub fn answer_sync(store: &Store, body: &Value, name: &str) -> Result<Option<(Value, Session)>> {
    if body.get("format").and_then(|f| f.as_i64()) != Some(SYNC_FORMAT) {
        return Ok(None);
    }
    let peer = text_of(body, "deviceId");
    let peer_name = text_of(body, "name");
    let mut report = ImportReport::default();
    let before = store.current_seq()?;
    let applied = store.apply_entries(
        &entries_of(body.get("entries")),
        &keys_of(body.get("keys")),
        (!peer.is_empty()).then_some(peer.as_str()),
        &mut report,
    )?;
    let moment = store.moment_for(before, !applied.is_empty(), cause::SYNC, Some(&peer_name))?;
    let answer = json!({
        "format": SYNC_FORMAT,
        "deviceId": store.device_id()?,
        "name": name,
        "entries": wire(&store.entries_since(&vector_of(body.get("vector")))?),
        "wantBlobs": store.missing_blobs()?,
        "keys": keys_json(store)?,
    });
    Ok(Some((
        answer,
        Session {
            peer,
            name: peer_name,
            applied,
            moment,
            report,
        },
    )))
}

/// The joiner takes in the host's `/sync` answer, as the Dart core's
/// `applyAnswer`.
pub fn apply_answer(store: &Store, answer: &Value) -> Result<Session> {
    let host = text_of(answer, "deviceId");
    let name = text_of(answer, "name");
    let mut report = ImportReport::default();
    let before = store.current_seq()?;
    let applied = store.apply_entries(
        &entries_of(answer.get("entries")),
        &keys_of(answer.get("keys")),
        (!host.is_empty()).then_some(host.as_str()),
        &mut report,
    )?;
    let moment = store.moment_for(before, !applied.is_empty(), cause::SYNC, Some(&name))?;
    Ok(Session {
        peer: host,
        name,
        applied,
        moment,
        report,
    })
}

impl Store {
    /// Answers one request as the host, going by `name`. A completed
    /// `/sync` comes back as its session.
    pub fn serve(&self, request: &Request, name: &str) -> Result<(Response, Option<Session>)> {
        let path = request.path.as_str();
        Ok(match (request.method.as_str(), path) {
            ("GET", "/vector") => (self.serve_vector()?, None),
            ("POST", "/sync") => self.serve_sync(&request.body, name)?,
            ("GET", p) if p.starts_with("/blob/") => {
                let (kind, hash) = split_blob_ref(&p["/blob/".len()..]);
                match self.blob(kind, hash) {
                    Some(bytes) => (Response::bytes(bytes), None),
                    None => (Response::status(404), None),
                }
            }
            ("POST", p) if p.starts_with("/blob/") => {
                let reference = &p["/blob/".len()..];
                let (kind, hash) = split_blob_ref(reference);
                // Only a blob the log mentions: anything else is storage
                // filled by whoever holds the PIN.
                let kept =
                    self.knows_blob(reference)? && self.put_blob(kind, hash, &request.body)?;
                (Response::status(if kept { 200 } else { 404 }), None)
            }
            _ => (Response::status(404), None),
        })
    }

    /// `/vector`: the version vector, with the wire format and this
    /// Device's id in the headers.
    pub fn serve_vector(&self) -> Result<Response> {
        let mut response = Response::json(200, &json!(self.version_vector()?));
        response
            .headers
            .push((FORMAT_HEADER.to_string(), SYNC_FORMAT.to_string()));
        response
            .headers
            .push((DEVICE_HEADER.to_string(), self.device_id()?));
        Ok(response)
    }

    /// `/sync`: refuses another format with its reason, answers the rest.
    pub fn serve_sync(&self, body: &[u8], name: &str) -> Result<(Response, Option<Session>)> {
        let body: Value =
            serde_json::from_slice(body).map_err(|e| Error::Invalid(format!("sync body: {e}")))?;
        Ok(match answer_sync(self, &body, name)? {
            Some((answer, session)) => (Response::json(200, &answer), Some(session)),
            None => (
                Response::json(403, &json!({"refusal": "format", "format": SYNC_FORMAT})),
                None,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Notes;
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use std::io::Cursor;
    use std::path::PathBuf;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn pair_codes_match_the_dart_core() -> TestResult {
        let fp: Vec<u8> = (0..32u32).map(|i| ((i * 7 + 3) & 0xff) as u8).collect();
        let host = Ipv4Addr::new(192, 168, 1, 23);
        assert_eq!(
            encode_pair_code(host, 45231, "042817", &fp, true),
            "r2m02_5xgnw_0aeg8_3188h_g7s65_mt0"
        );
        let typed = decode_pair_code("R2M02 5XGNW 0AEG8 3188H G7S65 MTO").ok_or("no code")?;
        assert_eq!(typed.host, "192.168.1.23");
        assert_eq!(typed.port, 45231);
        assert_eq!(typed.pin, "042817");
        assert_eq!(typed.fingerprint, fp[..8]);
        let full = decode_pair_code(&encode_pair_code(host, 45231, "042817", &fp, false))
            .ok_or("no code")?;
        assert_eq!(full.fingerprint, fp);
        assert_eq!(
            pair_code_lines("r2m02_5xgnw_0aeg8_3188h_g7s65_mt0"),
            vec!["r2m02_5xgnw_0aeg8", "3188h_g7s65_mt0"]
        );
        assert!(decode_pair_code("nope").is_none());
        Ok(())
    }

    #[test]
    fn a_chunked_body_is_read() -> TestResult {
        let raw = b"POST /sync HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\nX-Oldbrainfix-Pin: 123456\r\n\r\n5\r\nhello\r\n6;ext=1\r\n world\r\n0\r\n\r\n";
        let request = read_request(&mut Cursor::new(&raw[..]), IpAddr::V4(Ipv4Addr::LOCALHOST))?
            .ok_or("no request")?;
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/sync");
        assert_eq!(request.header(PIN_HEADER), Some("123456"));
        assert_eq!(request.body, b"hello world");
        let plain = b"GET /vector?x=1 HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc";
        let request = read_request(
            &mut Cursor::new(&plain[..]),
            IpAddr::V4(Ipv4Addr::LOCALHOST),
        )?
        .ok_or("no request")?;
        assert_eq!(request.path, "/vector");
        assert_eq!(request.body, b"abc");
        Ok(())
    }

    fn agent() -> ureq::Agent {
        // The phone pins the certificate by its fingerprint; here the
        // loopback is trusted outright.
        let config = ureq::Agent::config_builder()
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .disable_verification(true)
                    .build(),
            )
            .http_status_as_error(false)
            .build();
        ureq::Agent::new_with_config(config)
    }

    /// What a joiner brought home.
    struct Joined {
        session: Session,
        blobs_in: usize,
        blobs_out: usize,
    }

    /// Joins as the Dart core's `lanSync` does: vector, sync, then the
    /// blobs both ways; a refusal comes back as its status and text.
    fn join(store: &Store, port: u16, pin: &str) -> std::result::Result<Joined, String> {
        let fail = |e: &dyn std::fmt::Display| e.to_string();
        let agent = agent();
        let base = format!("https://127.0.0.1:{port}");
        let mut res = agent
            .get(format!("{base}/vector"))
            .header(PIN_HEADER, pin)
            .call()
            .map_err(|e| fail(&e))?;
        if res.status().as_u16() != 200 {
            let text = res.body_mut().read_to_string().unwrap_or_default();
            return Err(format!("{} {text}", res.status().as_u16()));
        }
        let host_device = res
            .headers()
            .get(DEVICE_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let host_vector: HashMap<String, i64> =
            serde_json::from_str(&res.body_mut().read_to_string().map_err(|e| fail(&e))?)
                .map_err(|e| fail(&e))?;
        let body =
            sync_request(store, &host_vector, "Phone", &host_device, pin).map_err(|e| fail(&e))?;
        let mut res = agent
            .post(format!("{base}/sync"))
            .header(PIN_HEADER, pin)
            .content_type("application/json")
            .send(body.to_string().as_bytes())
            .map_err(|e| fail(&e))?;
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if res.status().as_u16() != 200 {
            return Err(format!("{} {text}", res.status().as_u16()));
        }
        let answer: Value = serde_json::from_str(&text).map_err(|e| fail(&e))?;
        let session = apply_answer(store, &answer).map_err(|e| fail(&e))?;
        let mut blobs_in = 0;
        for reference in store.missing_blobs().map_err(|e| fail(&e))? {
            let mut res = agent
                .get(format!("{base}/blob/{reference}"))
                .header(PIN_HEADER, pin)
                .call()
                .map_err(|e| fail(&e))?;
            let (kind, hash) = split_blob_ref(&reference);
            let bytes = res.body_mut().read_to_vec().map_err(|e| fail(&e))?;
            if res.status().as_u16() == 200 && store.put_blob(kind, hash, &bytes).unwrap_or(false) {
                blobs_in += 1;
            }
        }
        let mut blobs_out = 0;
        for reference in answer["wantBlobs"].as_array().into_iter().flatten() {
            let reference = reference.as_str().unwrap_or_default();
            let (kind, hash) = split_blob_ref(reference);
            if let Some(bytes) = store.blob(kind, hash) {
                agent
                    .post(format!("{base}/blob/{reference}"))
                    .header(PIN_HEADER, pin)
                    .send(bytes.as_slice())
                    .map_err(|e| fail(&e))?;
                blobs_out += 1;
            }
        }
        Ok(Joined {
            session,
            blobs_in,
            blobs_out,
        })
    }

    fn open(path: &std::path::Path, seed: u8) -> Result<Store> {
        Store::open(path, ticking(), move || {
            Ok(SigningKey::from_seed([seed; 32]))
        })
    }

    /// The host's side as the desktop runs it: the store answers every
    /// request, the first `/sync` of a Device declined when `decline`.
    fn serve_until(
        path: PathBuf,
        host: Host,
        stop: Arc<AtomicBool>,
        decline: bool,
    ) -> JoinHandle<Result<Vec<Session>>> {
        std::thread::spawn(move || {
            let store = open(&path, 1)?;
            let mut sessions = Vec::new();
            let mut declined = !decline;
            while !stop.load(Ordering::Relaxed) {
                let Some(request) = host.next_request() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                if request.join_ask().is_some() && !declined {
                    declined = true;
                    request.reply(Response::declined());
                    continue;
                }
                let (response, session) = store.serve(&request, "Desk")?;
                sessions.extend(session);
                request.reply(response);
            }
            drop(host);
            Ok(sessions)
        })
    }

    #[test]
    fn a_phone_joins_the_desktop_over_the_loopback() -> TestResult {
        let dir = tempfile::tempdir()?;
        let host_path = dir.path().join("desk").join("oldbrainfix.db");
        std::fs::create_dir_all(dir.path().join("desk"))?;
        std::fs::create_dir_all(dir.path().join("phone"))?;
        let (identity, desk_photo) = {
            let desk = open(&host_path, 1)?;
            let note = Notes::new(&desk).capture_with(
                "From the desk",
                None,
                None,
                &[b"desk".to_vec()],
                &[],
            )?;
            (desk.tls_identity()?, note.photos[0].clone())
        };
        let host = Host::start(&identity, "246810", Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
        let port = host.port();
        assert_eq!(
            decode_pair_code(&host.pair_code(false)).map(|i| i.fingerprint),
            Some(identity.fingerprint.clone())
        );
        let stop = Arc::new(AtomicBool::new(false));
        let served = serve_until(host_path.clone(), host, stop.clone(), true);

        let phone = open(&dir.path().join("phone").join("oldbrainfix.db"), 2)?;
        let note = Notes::new(&phone).capture_with(
            "From the phone",
            None,
            None,
            &[],
            &[b"voice".to_vec()],
        )?;
        let phone_voice = note.recordings[0].clone();

        assert_eq!(join(&phone, port, "000000").err().as_deref(), Some("403 "));
        assert_eq!(
            join(&phone, port, "246810").err().as_deref(),
            Some("403 declined")
        );
        let first = join(&phone, port, "246810")?;
        assert!(!first.session.applied.is_empty());
        assert_eq!(first.session.name, "Desk");
        assert!(first.session.moment.is_some());
        assert_eq!((first.blobs_in, first.blobs_out), (1, 1));
        assert_eq!(phone.photo(&desk_photo).as_deref(), Some(&b"desk"[..]));
        let again = join(&phone, port, "246810")?;
        assert!(again.session.applied.is_empty());
        assert_eq!((again.blobs_in, again.blobs_out), (0, 0));

        stop.store(true, Ordering::Relaxed);
        let sessions = served.join().map_err(|_| "host thread")??;
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "Phone");
        assert!(sessions[0].moment.is_some());
        assert!(sessions[1].moment.is_none());
        let desk = open(&host_path, 1)?;
        assert_eq!(desk.audio(&phone_voice).as_deref(), Some(&b"voice"[..]));
        assert_eq!(desk.moments()?.len(), 1);
        Ok(())
    }

    #[test]
    fn wrong_pins_lock_an_address_out() -> TestResult {
        let identity = generate_identity()?;
        let host = Host::start(&identity, "111111", Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
        let agent = agent();
        let url = format!("https://127.0.0.1:{}/vector", host.port());
        for _ in 0..PIN_FAILURES_PER_ADDRESS {
            let res = agent.get(&url).header(PIN_HEADER, "999999").call()?;
            assert_eq!(res.status().as_u16(), 403);
        }
        let mut res = agent.get(&url).header(PIN_HEADER, "111111").call()?;
        assert_eq!(res.status().as_u16(), 403);
        assert_eq!(res.body_mut().read_to_string()?, "locked");
        assert!(!host.locked_out(), "one address, not everyone");
        Ok(())
    }

    #[test]
    fn another_format_is_refused_with_its_reason() -> TestResult {
        let store = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let body = json!({"format": 2, "deviceId": "p", "entries": []}).to_string();
        let (response, session) = store.serve_sync(body.as_bytes(), "Desk")?;
        assert!(session.is_none());
        assert_eq!(response.status, 403);
        let refusal: Value = serde_json::from_slice(&response.body)?;
        assert_eq!(refusal["refusal"], "format");
        Ok(())
    }

    #[test]
    fn identities_are_made_once_and_fingerprinted() -> TestResult {
        let store = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let first = store.tls_identity()?;
        assert_eq!(first, store.tls_identity()?);
        assert_eq!(first.fingerprint.len(), 32);
        assert_eq!(first.fingerprint, fingerprint_of(&first.cert_der));
        assert_ne!(generate_identity()?.fingerprint, first.fingerprint);
        Ok(())
    }

    #[test]
    fn only_a_paired_device_with_its_proof_is_trusted() -> TestResult {
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let request = |store: &Store, pin: &str| -> Result<JoinAsk> {
            let body = sync_request(store, &HashMap::new(), "Pixel", &desk.device_id()?, pin)?;
            Ok(JoinAsk {
                device: store.device_id()?,
                name: "Pixel".into(),
                proof: body["proof"].as_str().map(str::to_string),
            })
        };
        let ask = request(&phone, "123456")?;
        assert!(!desk.trusts_joiner(&ask, "123456")?, "not paired yet");
        let first = sync_request(&phone, &HashMap::new(), "Pixel", "", "")?;
        let (_, session) = desk.serve_sync(first.to_string().as_bytes(), "Desk")?;
        assert!(session.is_some());
        desk.remember_device(&crate::PairedDevice {
            id: phone.device_id()?,
            name: "Pixel".into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: desk.now(),
        })?;
        assert!(desk.trusts_joiner(&ask, "123456")?);
        assert!(!desk.trusts_joiner(&ask, "654321")?, "another session");
        let impostor = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
        let claimed = JoinAsk {
            device: phone.device_id()?,
            ..request(&impostor, "123456")?
        };
        assert!(!desk.trusts_joiner(&claimed, "123456")?, "a claimed id");
        Ok(())
    }

    #[test]
    fn a_join_ask_names_the_device() {
        let (reply, _) = channel();
        let request = Request {
            method: "POST".into(),
            path: "/sync".into(),
            headers: Vec::new(),
            body: br#"{"deviceId":"abc","name":"Pixel","proof":"x"}"#.to_vec(),
            from: IpAddr::V4(Ipv4Addr::LOCALHOST),
            reply,
        };
        assert_eq!(
            request.join_ask(),
            Some(JoinAsk {
                device: "abc".into(),
                name: "Pixel".into(),
                proof: Some("x".into()),
            })
        );
    }
}
