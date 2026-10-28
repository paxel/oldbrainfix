//! LAN sync as the desktop hosts it (ADR 0003), Catlog's in-person host
//! copied and adapted: a small HTTPS server with a self-signed
//! certificate made once per install, the pair code carrying its
//! fingerprint and a PIN, and the phone joining as the Dart core's
//! `lanSync` does: `GET /vector`, `POST /sync`, then the photos and
//! voice recordings under `/blob/<kind>/<hash>`. Phones host the same
//! way (the Dart core's `LanHost`); the joiner names its own
//! certificate's fingerprint in `POST /sync`, so the host can join it
//! the other way round later. Every request carries
//! the PIN; an address is locked out after
//! [`PIN_FAILURES_PER_ADDRESS`] wrong ones, everyone after
//! [`PIN_FAILURES_TOTAL`].
//!
//! A Device paired before syncs on its own without the PIN, with the
//! host [`Host::start_paired`] runs while the app does: the joiner checks
//! the certificate against the fingerprint it pinned, takes a fresh
//! nonce from `GET /challenge` and signs it with its own key in every
//! request ([`paired_proof_bytes`]); [`Store::serve_paired`] answers only
//! a paired Device whose verified key made that signature.
//!
//! The network runs on its own threads and never touches the store:
//! every request that passed the PIN, or carries a nonce this host handed
//! out, crosses a channel to whoever polls [`Host::next_request`], who
//! answers it with [`Store::serve`] (or holds it while the user is asked
//! whether to trust the Device) and hands the response back.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::conflicts::Conflict;
use crate::entry::WireEntry;
use crate::keys::{AreaKey, area_keys_of};
use crate::pairing::PairedDevice;
use crate::profile::{OwnProfile, ProfileChoice, ProfileTaken};
use crate::shared::Invite;
use crate::signing::{ImportReport, KeyRecord};
use crate::store::Store;
use crate::sync::{Moment, UpdateNeeded, cause, split_blob_ref, wire};
use crate::{Error, Result};

/// The wire formats this build speaks, the oldest and the newest, as the
/// Dart core's `oldestSyncFormat` and `newestSyncFormat` (ADR 0019).
/// Both sides state theirs in [`FORMAT_HEADER`], `<oldest>-<newest>`,
/// and sync in the newest both speak, which the `/sync` body names as
/// its `format`; a Device stating none speaks format 1. Format 2 carries
/// the entries' causal contexts, which format 1 would refuse as forged;
/// format 3 the invites to Shared Areas, which format 2 would answer
/// with everything it holds; format 4 the own Profile between one's own
/// Devices (ADR 0017), which format 3 would leave behind.
pub const OLDEST_SYNC_FORMAT: i64 = 4;
pub const NEWEST_SYNC_FORMAT: i64 = 4;

/// The wire formats a Device speaks, the oldest and the newest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatRange {
    pub oldest: i64,
    pub newest: i64,
}

/// The wire formats this build speaks.
pub const OWN_FORMATS: FormatRange = FormatRange {
    oldest: OLDEST_SYNC_FORMAT,
    newest: NEWEST_SYNC_FORMAT,
};

impl FormatRange {
    /// The range as [`FORMAT_HEADER`] states it.
    pub fn encode(self) -> String {
        format!("{}-{}", self.oldest, self.newest)
    }

    /// The range a header states: `<oldest>-<newest>`, or one format
    /// alone. A Device stating none, or nothing readable, speaks 1.
    pub fn parse(text: Option<&str>) -> FormatRange {
        let numbers: Vec<Option<i64>> = text
            .unwrap_or_default()
            .trim()
            .split('-')
            .map(|p| p.trim().parse().ok())
            .collect();
        match numbers.as_slice() {
            [Some(one)] if *one >= 1 => FormatRange {
                oldest: *one,
                newest: *one,
            },
            [Some(oldest), Some(newest)] if *oldest >= 1 && oldest <= newest => FormatRange {
                oldest: *oldest,
                newest: *newest,
            },
            _ => FormatRange {
                oldest: 1,
                newest: 1,
            },
        }
    }

    /// The newest format both this range and `ours` speak; none when
    /// they share none.
    pub fn common(self, ours: FormatRange) -> Option<i64> {
        let newest = self.newest.min(ours.newest);
        (newest >= self.oldest.max(ours.oldest)).then_some(newest)
    }

    /// Which Device needs an update when this range, the other one's,
    /// and `ours` share no format: the one whose newest is older than the
    /// other's oldest.
    pub fn update_needed(self, ours: FormatRange) -> UpdateNeeded {
        if self.newest < ours.oldest {
            UpdateNeeded::There
        } else {
            UpdateNeeded::Here
        }
    }
}

/// The wire format a body is written in: its `format`, 1 when missing, 0
/// when it is no whole number.
pub fn format_of(body: &Value) -> i64 {
    match body.get("format") {
        None => 1,
        Some(format) => format.as_i64().unwrap_or(0),
    }
}

/// Whether this build reads a body written in `format`.
pub fn speaks_format(format: i64) -> bool {
    (OLDEST_SYNC_FORMAT..=NEWEST_SYNC_FORMAT).contains(&format)
}

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
/// On `/vector` while this host invites the joiner to a Shared Area
/// (ADR 0013): the joiner then sends nothing of its own.
pub const INVITE_HEADER: &str = "x-oldbrainfix-invite";
/// A paired Device's nonce from `/challenge` and its signature over it;
/// its id goes in [`DEVICE_HEADER`].
pub const NONCE_HEADER: &str = "x-oldbrainfix-nonce";
pub const PROOF_HEADER: &str = "x-oldbrainfix-proof";

/// How long a nonce from `/challenge` holds, and how many may be out at
/// once; the oldest goes first.
pub const CHALLENGE_TTL: Duration = Duration::from_secs(120);
pub const MAX_CHALLENGES: usize = 256;

/// The source of the update notice of a joiner that did not name itself.
pub const LAN_SOURCE: &str = "lan";

/// The local setting holding the port the paired host had, taken again
/// when it is free, so a phone finds it where it was.
pub const PAIRED_PORT_KEY: &str = "lan:port";

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

/// Whether `host` is an address on a local network, as the Dart core's
/// `isPrivateHost`: the only place a Device to pair with in person can
/// be. A code naming a public address would send every entry to a
/// stranger before any answer came back.
pub fn is_private_host(host: &str) -> bool {
    let Ok(ip) = host.parse::<Ipv4Addr>() else {
        return false;
    };
    let [a, b, _, _] = ip.octets();
    a == 10
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 169 && b == 254)
        || a == 127
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
        if let Some(held) = self.held_tls_identity()? {
            return Ok(held);
        }
        let made = generate_identity()?;
        self.set_local_setting(TLS_CERT_KEY, &BASE64.encode(&made.cert_der))?;
        self.set_local_setting(TLS_KEY_KEY, &BASE64.encode(&made.key_der))?;
        Ok(made)
    }

    /// The identity kept here, as the Dart core's `heldHostIdentity`;
    /// none before this Device first hosted or joined.
    pub fn held_tls_identity(&self) -> Result<Option<Identity>> {
        if let (Some(cert), Some(key)) = (
            self.local_setting(TLS_CERT_KEY)?,
            self.local_setting(TLS_KEY_KEY)?,
        ) && let (Ok(cert_der), Ok(key_der)) = (BASE64.decode(cert), BASE64.decode(key))
        {
            return Ok(Some(Identity {
                fingerprint: fingerprint_of(&cert_der),
                cert_der,
                key_der,
            }));
        }
        Ok(None)
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
    /// Whether it carries a nonce this host handed out and that still
    /// holds; whose proof it is, [`Store::paired_joiner`] checks.
    challenged: bool,
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

    /// The Profile the Device asking to sync sent of its person, for a
    /// `POST /sync` of a joiner invited (ADR 0017), checked and with its
    /// picture made again; none without one that holds its key.
    pub fn joiner_profile(&self, store: &Store) -> Option<ProfileChoice> {
        let ask = self.join_ask()?;
        let body: Value = serde_json::from_slice(&self.body).ok()?;
        store.sent_profile_of(body.get("profile"), &ask.device)
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

/// The bytes a paired Device signs in every request of an automatic
/// sync, as the Dart core's `pairedProofBytes`: this host's id and the
/// nonce it handed out.
pub fn paired_proof_bytes(host_device: &str, nonce: &str) -> Vec<u8> {
    json!(["auto", host_device, nonce]).to_string().into_bytes()
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

    /// Whether `device`, signing `proof` over `nonce`, is a paired Device
    /// this one syncs with on its own: its verified key signed
    /// [`paired_proof_bytes`] for this host, as the Dart core's
    /// `trustsPairedJoiner`.
    pub fn trusts_paired_joiner(
        &self,
        device: &str,
        nonce: &str,
        proof: Option<&str>,
    ) -> Result<bool> {
        if self.paired_device(device)?.is_none() || !self.auto_sync_on(device)? {
            return Ok(false);
        }
        let Some(key) = self.pinned_key(device)? else {
            return Ok(false);
        };
        if key.trust != crate::KeyTrust::Verified {
            return Ok(false);
        }
        let bytes = paired_proof_bytes(&self.device_id()?, nonce);
        Ok(crate::signing::verify_signature(
            &key.record.public_key,
            &bytes,
            proof,
        ))
    }

    /// The paired Device `request` comes from, when it carries a nonce
    /// this host handed out and that Device's proof over it.
    pub fn paired_joiner(&self, request: &Request) -> Result<Option<String>> {
        let (Some(device), Some(nonce)) =
            (request.header(DEVICE_HEADER), request.header(NONCE_HEADER))
        else {
            return Ok(None);
        };
        if !request.challenged {
            return Ok(None);
        }
        Ok(self
            .trusts_paired_joiner(device, nonce, request.header(PROOF_HEADER))?
            .then(|| device.to_string()))
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
        let bind_error = |e: std::io::Error| Error::Invalid(format!("bind: {e}"));
        let listener = TcpListener::bind(SocketAddr::new(address, 0)).map_err(bind_error)?;
        Host::serve(identity, Some(pin), address, listener)
    }

    /// The host paired Devices sync with on their own, without a PIN:
    /// on `bind` (every address of this machine, so it outlives a change
    /// of network), on `port` while that is free, else on a free one.
    pub fn start_paired(identity: &Identity, bind: IpAddr, port: u16) -> Result<Host> {
        let bind_error = |e: std::io::Error| Error::Invalid(format!("bind: {e}"));
        let listener = TcpListener::bind(SocketAddr::new(bind, port))
            .or_else(|_| TcpListener::bind(SocketAddr::new(bind, 0)))
            .map_err(bind_error)?;
        Host::serve(identity, None, bind, listener)
    }

    fn serve(
        identity: &Identity,
        pin: Option<&str>,
        address: IpAddr,
        listener: TcpListener,
    ) -> Result<Host> {
        let config = server_config(identity)?;
        let bind_error = |e: std::io::Error| Error::Invalid(format!("bind: {e}"));
        listener.set_nonblocking(true).map_err(bind_error)?;
        let port = listener.local_addr().map_err(bind_error)?.port();
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let locked = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(Gate {
            pin: pin.map(str::to_string),
            failures: Mutex::new(Failures::default()),
            challenges: Mutex::new(Vec::new()),
            locked: locked.clone(),
        });
        let thread = {
            let stop = stop.clone();
            std::thread::spawn(move || accept_loop(listener, config, tx, gate, stop))
        };
        Ok(Host {
            address,
            port,
            pin: pin.unwrap_or_default().to_string(),
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

    /// The PIN; empty for the paired host, which takes none.
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

    /// The next request, waiting up to `wait` for one.
    pub fn wait_request(&self, wait: Duration) -> Option<Request> {
        self.requests.recv_timeout(wait).ok()
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

/// The PIN and the wrong guesses counted against it, and the nonces
/// handed out with when.
struct Gate {
    /// None for the paired host: only nonces let a request in.
    pin: Option<String>,
    failures: Mutex<Failures>,
    challenges: Mutex<Vec<(String, Instant)>>,
    locked: Arc<AtomicBool>,
}

impl Gate {
    /// A fresh nonce, kept for [`CHALLENGE_TTL`].
    fn challenge(&self) -> String {
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).ok();
        let nonce = hex::encode(random);
        let mut held = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        held.retain(|(_, at)| at.elapsed() < CHALLENGE_TTL);
        if held.len() >= MAX_CHALLENGES {
            held.remove(0);
        }
        held.push((nonce.clone(), Instant::now()));
        nonce
    }

    /// Whether `nonce` was handed out here and still holds.
    fn knows(&self, nonce: Option<&str>) -> bool {
        let Some(nonce) = nonce else {
            return false;
        };
        let held = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        held.iter()
            .any(|(n, at)| n == nonce && at.elapsed() < CHALLENGE_TTL)
    }

    /// Whether this request may pass: `Ok` when it did, else the answer
    /// to send instead.
    fn check(&self, from: IpAddr, pin: Option<&str>) -> std::result::Result<(), Response> {
        let Some(own) = self.pin.as_deref() else {
            // No PIN to guess here.
            return Err(Response::status(403));
        };
        let mut failures = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        let locked = failures.total >= PIN_FAILURES_TOTAL
            || failures.per_address.get(&from).copied().unwrap_or(0) >= PIN_FAILURES_PER_ADDRESS;
        if locked {
            return Err(Response::text(403, "locked"));
        }
        if pin != Some(own) {
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
        // Nonces only on the paired host: on a PIN host they would let
        // anyone past the PIN. Asking there costs no PIN try.
        Ok(Some(partial)) if partial.method == "GET" && partial.path == "/challenge" => {
            if gate.pin.is_none() {
                Response::json(200, &json!({"nonce": gate.challenge()}))
            } else {
                Response::status(404)
            }
        }
        Ok(Some(partial)) => {
            let challenged = gate.pin.is_none() && gate.knows(partial.header(NONCE_HEADER));
            let passed = if challenged {
                Ok(())
            } else {
                gate.check(partial.from, partial.header(PIN_HEADER))
            };
            match passed {
                Err(refusal) => refusal,
                Ok(()) => {
                    let (reply_tx, reply_rx) = channel();
                    let request = Request {
                        reply: reply_tx,
                        challenged,
                        ..partial
                    };
                    if tx.send(request).is_err() {
                        Response::status(503)
                    } else {
                        reply_rx.recv().unwrap_or_else(|_| Response::status(503))
                    }
                }
            }
        }
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
        challenged: false,
        reply,
    }))
}

/// A chunked body; `None` past the limit.
pub(crate) fn read_chunked<R: BufRead>(
    reader: &mut R,
    limit: usize,
) -> std::io::Result<Option<Vec<u8>>> {
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
/// `SyncSession`, with the Conflicts what came in is part of.
#[derive(Debug, Clone)]
pub struct Session {
    /// The other Device's id and the name it goes by.
    pub peer: String,
    pub name: String,
    pub applied: Vec<WireEntry>,
    pub moment: Option<Moment>,
    pub report: ImportReport,
    pub conflicts: Vec<Conflict>,
    /// The Shared Area this session invited one side to, if it did.
    pub shared_area: Option<String>,
    /// What became of the own Profile the other Device sent (ADR 0017).
    pub profile: ProfileTaken,
    /// Whether a Profile of another person came in or changed.
    pub people_changed: bool,
    /// The whole fingerprint of the certificate the joiner hosts with,
    /// hex, as it named it in `POST /sync` and proved with its key; none
    /// on the joiner's side and when it named none.
    pub fingerprint: Option<String>,
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

/// The entries of a body, each in its canonical form (ADR 0019).
fn entries_of(value: Option<&Value>) -> Vec<WireEntry> {
    value
        .and_then(|e| e.as_array())
        .map(|list| list.iter().filter_map(WireEntry::from_value).collect())
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

/// Area keys, as they travel over an authenticated session only.
fn area_keys_json(keys: &[AreaKey]) -> Value {
    json!(keys.iter().map(AreaKey::to_json).collect::<Vec<_>>())
}

/// The Area a `POST /sync` body or an answer invites to, if any.
fn invited_to(value: Option<&Value>) -> Option<String> {
    value?
        .get("area")?
        .as_str()
        .filter(|a| a.starts_with("area:"))
        .map(str::to_string)
}

/// How a joiner's request goes to the host, as the Dart core's
/// `syncRequest` options (ADR 0013).
#[derive(Debug, Clone, Copy)]
pub enum Joining<'a> {
    /// To a Device of this one's own group, or one paired by invite
    /// before, which [`Store::is_shared_peer`] tells.
    Plain,
    /// The host invites this Device: nothing of its own goes but its
    /// Profile, for the host to show.
    Invited,
    /// This Device invites the host, whose Profile, sent before, is the
    /// person invited: the grant, then the Area's history.
    Inviting(&'a Invite, Option<&'a Value>),
}

/// The host's answer to `GET /profile`, as the Dart core's
/// `profileAnswer`: its own Profile, signed, as it goes to another
/// person, for an inviting joiner to show before it invites (ADR 0017);
/// empty before the Profile exists.
pub fn profile_answer(store: &Store) -> Result<Value> {
    Ok(match store.own_profile_for_others()? {
        Some(profile) => json!({"profile": profile}),
        None => json!({}),
    })
}

/// Keeps the Profile `json` the Device `device` sent of its person for
/// an invite, and returns its id; none when it is none, or its binding
/// does not hold that Device's key.
fn invited_person(store: &Store, json: Option<&Value>, device: &str) -> Result<Option<String>> {
    let Some(choice) = store.sent_profile_of(json, device) else {
        return Ok(None);
    };
    if let Some(json) = json {
        store.take_person(json, None)?;
    }
    Ok(Some(choice.profile.id))
}

/// The joiner's `POST /sync` body, as the Dart core's `syncRequest`,
/// with its proof over `host_device` and `pin` and the Area keys, and
/// the whole fingerprint of the certificate this Device hosts with once
/// it has one, so the host can join it later.
pub fn sync_request(
    store: &Store,
    host_vector: &HashMap<String, i64>,
    name: &str,
    host_device: &str,
    pin: &str,
) -> Result<Value> {
    sync_request_as(store, host_vector, name, host_device, pin, Joining::Plain)
}

/// [`sync_request`] as `joining` says: to another person's Device only
/// what it shares.
pub fn sync_request_as(
    store: &Store,
    host_vector: &HashMap<String, i64>,
    name: &str,
    host_device: &str,
    pin: &str,
    joining: Joining,
) -> Result<Value> {
    store.ensure_area_keys()?;
    // The backup key goes to a Device of its own group only.
    let own = matches!(joining, Joining::Plain) && !store.is_shared_peer(host_device)?;
    let (entries, area_keys, profiles) = match joining {
        Joining::Invited => (Vec::new(), Vec::new(), None),
        Joining::Inviting(invite, host_profile) => {
            let Some(person) = invited_person(store, host_profile, host_device)? else {
                return Err(Error::Invalid(
                    "the host sent no Profile of its person".into(),
                ));
            };
            store.invite(&invite.area, host_device, invite.role, &person)?;
            let also = [invite.area.clone()];
            let mut areas = store.areas_shared_with(host_device)?;
            areas.insert(invite.area.clone());
            (
                store.entries_for(host_device, &HashMap::new(), true, &also)?,
                store.area_keys_for(host_device, &also)?,
                Some(store.profiles_to_send(Some(&areas))?),
            )
        }
        Joining::Plain if store.is_shared_peer(host_device)? => (
            store.entries_for(host_device, host_vector, false, &[])?,
            store.area_keys_for(host_device, &[])?,
            Some(store.profiles_to_send(Some(&store.areas_shared_with(host_device)?))?),
        ),
        Joining::Plain => (
            store.entries_since(host_vector)?,
            store.area_keys()?,
            Some(store.profiles_to_send(None)?),
        ),
    };
    let mut body = json!({
        "format": NEWEST_SYNC_FORMAT,
        "deviceId": store.device_id()?,
        "name": name,
        "proof": store.signing_key()?.sign(&pairing_proof_bytes(host_device, pin)),
        "vector": store.version_vector()?,
        "entries": wire(&entries),
        "keys": keys_json(store)?,
        "areaKeys": area_keys_json(&area_keys),
    });
    if let Some(map) = body.as_object_mut() {
        match joining {
            Joining::Invited => {
                map.insert("invited".to_string(), Value::Bool(true));
                if let Some(profile) = store.own_profile_for_others()? {
                    map.insert("profile".to_string(), profile);
                }
            }
            Joining::Inviting(invite, _) => {
                map.insert("invite".to_string(), json!({"area": invite.area}));
            }
            Joining::Plain => {}
        }
        if let Some(profiles) = profiles {
            map.insert("profiles".to_string(), Value::Array(profiles));
        }
        if let Some(identity) = store.held_tls_identity()? {
            map.insert(
                "fingerprint".to_string(),
                Value::String(hex::encode(&identity.fingerprint)),
            );
        }
        if own && let Some(key) = store.backup_key_to_send()? {
            map.insert("backupKey".to_string(), key);
        }
        if own && let Some(profile) = OwnProfile::new(store).to_send()? {
            map.insert("profile".to_string(), profile);
        }
    }
    Ok(body)
}

/// The certificate fingerprint the joiner `peer` named in `body`, as the
/// Dart core's `_provenFingerprint`: only a whole one, and only when the
/// joiner proved, with the key pinned for `peer`, that it is that
/// Device: its proof over this host and `pin` (empty for a paired
/// Device's session). Anyone holding the PIN can name any device id, and
/// a fingerprint kept for another Device's id would have this one join
/// them as that Device later.
fn proven_fingerprint(
    store: &Store,
    body: &Value,
    peer: &str,
    pin: &str,
) -> Result<Option<String>> {
    let Some(fingerprint) = body.get("fingerprint").and_then(Value::as_str) else {
        return Ok(None);
    };
    let whole = fingerprint.len() == 64
        && fingerprint
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !whole || peer.is_empty() {
        return Ok(None);
    }
    let Some(key) = store.pinned_key(peer)? else {
        return Ok(None);
    };
    let proven = crate::signing::verify_signature(
        &key.record.public_key,
        &pairing_proof_bytes(&store.device_id()?, pin),
        body.get("proof").and_then(Value::as_str),
    );
    Ok(proven.then(|| fingerprint.to_string()))
}

/// The blobs this Device lacks that `peer`, another person's Device, may
/// be asked for: those of the Areas it holds a role in.
fn missing_for(store: &Store, peer: &str) -> Result<Vec<String>> {
    let areas = store.areas_shared_with(peer)?;
    let mut out = Vec::new();
    for reference in store.missing_blobs()? {
        if store.blob_in(&reference, &areas)? {
            out.push(reference);
        }
    }
    Ok(out)
}

/// Takes in a joiner's `POST /sync` body and answers it, as the Dart
/// core's `answerSync`; none when the body is in a format this build
/// does not speak. The answer is in the body's format.
pub fn answer_sync(store: &Store, body: &Value, name: &str) -> Result<Option<(Value, Session)>> {
    answer_sync_inviting(store, body, name, None)
}

/// [`answer_sync`] while this host hands out `inviting`. None as well
/// when the joiner ignores the invite, or invites this Device to an Area
/// it holds as its own. Another person's Device gets only what it
/// shares and only those Areas' keys are taken from it; a Device of this
/// one's own group gets this one's roles it lacks before the answer.
pub fn answer_sync_inviting(
    store: &Store,
    body: &Value,
    name: &str,
    inviting: Option<&Invite>,
) -> Result<Option<(Value, Session)>> {
    answer_sync_with(store, body, name, inviting, "")
}

/// [`answer_sync_inviting`] for a joiner whose proof is over `pin`: the
/// certificate fingerprint it names comes back in the session only when
/// that proof holds ([`proven_fingerprint`]).
pub fn answer_sync_with(
    store: &Store,
    body: &Value,
    name: &str,
    inviting: Option<&Invite>,
    pin: &str,
) -> Result<Option<(Value, Session)>> {
    let format = format_of(body);
    if !speaks_format(format) {
        return Ok(None);
    }
    let peer = text_of(body, "deviceId");
    let peer_name = text_of(body, "name");
    let peer_vector = vector_of(body.get("vector"));
    let invited = invited_to(body.get("invite"));
    if inviting.is_some()
        && (body.get("invited").and_then(Value::as_bool) != Some(true) || peer.is_empty())
    {
        return Ok(None);
    }
    // The person invited is the one whose Profile the joiner sent, and
    // the user saw before the session went on.
    if inviting.is_some() && store.sent_profile_of(body.get("profile"), &peer).is_none() {
        return Ok(None);
    }
    if let Some(area) = &invited
        && (inviting.is_some() || store.refuses_invite(area)?)
    {
        return Ok(None);
    }
    // A Device of this one's own group is no other person: inviting it
    // would cut its sync down to one Area.
    if (inviting.is_some() || invited.is_some()) && store.is_own_device(&peer)? {
        return Ok(None);
    }
    let shared = inviting.is_some() || invited.is_some() || store.is_shared_peer(&peer)?;
    let shared_area = inviting.map(|i| i.area.clone()).or_else(|| invited.clone());
    let mut report = ImportReport::default();
    let before = store.current_seq()?;
    let offered = area_keys_of(body.get("areaKeys"));
    if !shared {
        store.learn_area_keys(&offered)?;
    }
    let entries = if inviting.is_some() {
        Vec::new()
    } else {
        entries_of(body.get("entries"))
    };
    let applied = store.apply_entries(
        &entries,
        &keys_of(body.get("keys")),
        (!peer.is_empty()).then_some(peer.as_str()),
        &mut report,
    )?;
    if shared {
        let mut areas = store.areas_shared_with(&peer)?;
        areas.extend(invited.iter().cloned());
        let taken: Vec<AreaKey> = offered
            .into_iter()
            .filter(|k| areas.contains(&k.scope))
            .collect();
        store.learn_area_keys(&taken)?;
    }
    store.ensure_area_keys()?;
    let mut profile = ProfileTaken::Unchanged;
    let people;
    if let Some(invite) = inviting {
        let person = invited_person(store, body.get("profile"), &peer)?;
        if let Some(person) = &person {
            store.invite(&invite.area, &peer, invite.role, person)?;
        }
        people = person.is_some();
    } else if !shared {
        store.extend_roles_to(&peer)?;
        profile = take_profile(store, &peer, body)?;
        people = store.take_people(body.get("profiles"), None)?;
    } else {
        people = store.take_people(body.get("profiles"), Some(&store.people_from(&peer)?))?;
    }
    let moment = store.moment_for(before, !applied.is_empty(), cause::SYNC, Some(&peer_name))?;
    // The answer carries everything beyond what the joiner holds: once it
    // took it in, it holds what this Device holds now.
    store.note_held_by(&peer, &peer_vector)?;
    store.note_held_by(&peer, &store.version_vector()?)?;
    let (entries, want, area_keys, profiles) = if shared {
        (
            store.entries_for(&peer, &peer_vector, inviting.is_some(), &[])?,
            missing_for(store, &peer)?,
            store.area_keys_for(&peer, &[])?,
            store.profiles_to_send(Some(&store.areas_shared_with(&peer)?))?,
        )
    } else {
        (
            store.entries_since(&peer_vector)?,
            store.missing_blobs()?,
            store.area_keys()?,
            store.profiles_to_send(None)?,
        )
    };
    let mut answer = json!({
        "format": format,
        "deviceId": store.device_id()?,
        "name": name,
        "entries": wire(&entries),
        "wantBlobs": want,
        "keys": keys_json(store)?,
        "areaKeys": area_keys_json(&area_keys),
        "profiles": profiles,
    });
    if let Some(invite) = inviting
        && let Some(map) = answer.as_object_mut()
    {
        map.insert("invite".to_string(), json!({"area": invite.area}));
    }
    if !shared
        && let Some(key) = store.backup_key_to_send()?
        && let Some(map) = answer.as_object_mut()
    {
        map.insert("backupKey".to_string(), key);
    }
    if !shared
        && let Some(own) = OwnProfile::new(store).to_send()?
        && let Some(map) = answer.as_object_mut()
    {
        map.insert("profile".to_string(), own);
    }
    let conflicts = store.conflicts_among(&applied)?;
    let fingerprint = proven_fingerprint(store, body, &peer, pin)?;
    Ok(Some((
        answer,
        Session {
            peer,
            name: peer_name,
            applied,
            moment,
            report,
            conflicts,
            shared_area,
            profile,
            people_changed: people,
            fingerprint,
        },
    )))
}

/// Takes in the own Profile `sent` by `peer`, a Device of this one's own
/// group whose key the session verified; nothing without that key.
fn take_profile(store: &Store, peer: &str, body: &Value) -> Result<ProfileTaken> {
    match store.pinned_key(peer)? {
        Some(pinned) if !peer.is_empty() => {
            let taken =
                OwnProfile::new(store).take(body.get("profile"), &pinned.record.public_key)?;
            store.take_backup_key(body.get("backupKey"))?;
            Ok(taken)
        }
        _ => Ok(ProfileTaken::Unchanged),
    }
}

/// The joiner takes in the host's `/sync` answer, as the Dart core's
/// `applyAnswer`.
pub fn apply_answer(store: &Store, answer: &Value) -> Result<Session> {
    apply_answer_as(store, answer, false, None)
}

/// [`apply_answer`] from another person's Device when `shared`: only the
/// keys of the Areas it shares are taken, and an invite to an Area this
/// Device holds as its own is refused before anything is taken. A Device
/// of this one's own group gets this one's roles it lacks.
pub fn apply_answer_as(
    store: &Store,
    answer: &Value,
    shared: bool,
    invited_by: Option<&str>,
) -> Result<Session> {
    let host = text_of(answer, "deviceId");
    let name = text_of(answer, "name");
    let invited = invited_to(answer.get("invite"));
    if let Some(area) = &invited
        && store.refuses_invite(area)?
    {
        return Err(Error::Invalid(format!(
            "declined: {area} is this Device's own"
        )));
    }
    let mut report = ImportReport::default();
    let before = store.current_seq()?;
    let offered = area_keys_of(answer.get("areaKeys"));
    if !shared {
        store.learn_area_keys(&offered)?;
    }
    let applied = store.apply_entries(
        &entries_of(answer.get("entries")),
        &keys_of(answer.get("keys")),
        (!host.is_empty()).then_some(host.as_str()),
        &mut report,
    )?;
    let (profile, people) = if shared {
        let mut areas = store.areas_shared_with(&host)?;
        areas.extend(invited.iter().cloned());
        let taken: Vec<AreaKey> = offered
            .into_iter()
            .filter(|k| areas.contains(&k.scope))
            .collect();
        store.learn_area_keys(&taken)?;
        let people = store.take_people(answer.get("profiles"), Some(&store.people_from(&host)?))?;
        (ProfileTaken::Unchanged, people)
    } else {
        store.extend_roles_to(&host)?;
        let taken = take_profile(store, &host, answer)?;
        (taken, store.take_people(answer.get("profiles"), None)?)
    };
    let moment = store.moment_for(before, !applied.is_empty(), cause::SYNC, Some(&name))?;
    let conflicts = store.conflicts_among(&applied)?;
    Ok(Session {
        peer: host,
        name,
        applied,
        moment,
        report,
        conflicts,
        shared_area: invited.or_else(|| invited_by.map(str::to_string)),
        profile,
        people_changed: people,
        fingerprint: None,
    })
}

impl Store {
    /// Answers one request as the host, going by `name`. A completed
    /// `/sync` comes back as its session.
    pub fn serve(&self, request: &Request, name: &str) -> Result<(Response, Option<Session>)> {
        self.serve_as(request, name, None, None)
    }

    /// [`Store::serve`] while this host hands out `inviting` (ADR 0013):
    /// `/vector` says so, and only the invited Area's blobs are handed out.
    pub fn serve_inviting(
        &self,
        request: &Request,
        name: &str,
        inviting: Option<&Invite>,
    ) -> Result<(Response, Option<Session>)> {
        self.serve_as(request, name, inviting, None)
    }

    /// Answers `request`, from the paired Device `joiner` when known: one
    /// paired by invite gets only the blobs of the Areas it shares.
    fn serve_as(
        &self,
        request: &Request,
        name: &str,
        inviting: Option<&Invite>,
        joiner: Option<&str>,
    ) -> Result<(Response, Option<Session>)> {
        let path = request.path.as_str();
        Ok(match (request.method.as_str(), path) {
            ("GET", "/vector") => {
                // A joiner sharing no format with this build syncs nothing,
                // and is told so (ADR 0019).
                let theirs = FormatRange::parse(request.header(FORMAT_HEADER));
                if theirs.common(OWN_FORMATS).is_none() {
                    let joiner = request.header(DEVICE_HEADER);
                    (
                        self.refuse_format(joiner, theirs.update_needed(OWN_FORMATS))?,
                        None,
                    )
                } else {
                    (self.serve_vector_inviting(inviting.is_some())?, None)
                }
            }
            ("GET", "/profile") => (Response::json(200, &profile_answer(self)?), None),
            ("POST", "/sync") => {
                // A paired Device's proof is over no PIN.
                let pin = if request.challenged {
                    ""
                } else {
                    request.header(PIN_HEADER).unwrap_or_default()
                };
                self.serve_sync_with(&request.body, name, inviting, pin)?
            }
            ("GET", p) if p.starts_with("/blob/") => {
                let reference = &p["/blob/".len()..];
                let (kind, hash) = split_blob_ref(reference);
                let mut areas: Option<std::collections::HashSet<String>> = None;
                if let Some(invite) = inviting {
                    areas
                        .get_or_insert_with(Default::default)
                        .insert(invite.area.clone());
                }
                if let Some(device) = joiner
                    && self.is_shared_peer(device)?
                {
                    areas
                        .get_or_insert_with(Default::default)
                        .extend(self.areas_shared_with(device)?);
                }
                let allowed = match &areas {
                    Some(areas) => self.blob_in(reference, areas)?,
                    None => true,
                };
                match self.blob(kind, hash).filter(|_| allowed) {
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

    /// The refusal of a joiner, `joiner` when it named itself, that shares
    /// no wire format with this build: this build's formats in the header
    /// and the body. Which Device needs an update is kept as an update
    /// notice, by the joiner's id, to be announced once; a joiner that did
    /// not name itself, joining by pair code, is news every time.
    fn refuse_format(&self, joiner: Option<&str>, update: UpdateNeeded) -> Result<Response> {
        let source = joiner.filter(|j| !j.is_empty()).unwrap_or(LAN_SOURCE);
        if source == LAN_SOURCE {
            self.update_no_longer_needed(LAN_SOURCE)?;
        }
        self.note_update_needed(source, update)?;
        let mut response = Response::json(
            403,
            &json!({"refusal": "format", "formats": OWN_FORMATS.encode()}),
        );
        response
            .headers
            .push((FORMAT_HEADER.to_string(), OWN_FORMATS.encode()));
        Ok(response)
    }

    /// `/vector`: the version vector, with the wire formats and this
    /// Device's id in the headers.
    pub fn serve_vector(&self) -> Result<Response> {
        self.serve_vector_inviting(false)
    }

    /// [`Store::serve_vector`], saying so when this host invites.
    pub fn serve_vector_inviting(&self, inviting: bool) -> Result<Response> {
        let mut response = Response::json(200, &json!(self.version_vector()?));
        response
            .headers
            .push((FORMAT_HEADER.to_string(), OWN_FORMATS.encode()));
        response
            .headers
            .push((DEVICE_HEADER.to_string(), self.device_id()?));
        if inviting {
            response
                .headers
                .push((INVITE_HEADER.to_string(), "1".to_string()));
        }
        Ok(response)
    }

    /// `/sync`: refuses a format this build does not speak with its
    /// reason, answers the rest.
    pub fn serve_sync(&self, body: &[u8], name: &str) -> Result<(Response, Option<Session>)> {
        self.serve_sync_inviting(body, name, None)
    }

    /// [`Store::serve_sync`] while this host hands out `inviting`. An
    /// invite this Device must refuse, or a joiner ignoring this host's,
    /// is declined.
    pub fn serve_sync_inviting(
        &self,
        body: &[u8],
        name: &str,
        inviting: Option<&Invite>,
    ) -> Result<(Response, Option<Session>)> {
        self.serve_sync_with(body, name, inviting, "")
    }

    /// [`Store::serve_sync_inviting`] for a joiner whose proof is over
    /// `pin`.
    fn serve_sync_with(
        &self,
        body: &[u8],
        name: &str,
        inviting: Option<&Invite>,
        pin: &str,
    ) -> Result<(Response, Option<Session>)> {
        let body: Value =
            serde_json::from_slice(body).map_err(|e| Error::Invalid(format!("sync body: {e}")))?;
        let format = format_of(&body);
        Ok(match answer_sync_with(self, &body, name, inviting, pin)? {
            Some((answer, session)) => {
                self.update_no_longer_needed(&session.peer)?;
                let mut response = Response::json(200, &answer);
                response
                    .headers
                    .push((FORMAT_HEADER.to_string(), OWN_FORMATS.encode()));
                (response, Some(session))
            }
            None if speaks_format(format) => (Response::declined(), None),
            None => {
                let update = if format > NEWEST_SYNC_FORMAT {
                    UpdateNeeded::Here
                } else {
                    UpdateNeeded::There
                };
                let joiner = body.get("deviceId").and_then(Value::as_str);
                (self.refuse_format(joiner, update)?, None)
            }
        })
    }

    /// Answers one request on the paired host, going by `name`: only a
    /// paired Device that syncs on its own, proven by its key, and its
    /// `/sync` only in its own name. The Device is remembered as synced
    /// after a completed `/sync` ([`Store::remember_joiner`]).
    pub fn serve_paired(
        &self,
        request: &Request,
        name: &str,
    ) -> Result<(Response, Option<Session>)> {
        let Some(device) = self.paired_joiner(request)? else {
            return Ok((Response::declined(), None));
        };
        if let Some(ask) = request.join_ask()
            && ask.device != device
        {
            return Ok((Response::declined(), None));
        }
        let (response, session) = self.serve_as(request, name, None, Some(&device))?;
        if let Some(session) = &session {
            self.remember_joiner(session)?;
        }
        Ok((response, session))
    }

    /// Remembers the joiner of `session` as a paired Device, as the Dart
    /// core's `rememberJoiner`: with the certificate it proved it hosts
    /// with ([`Session::fingerprint`]), by invite when the session invited
    /// one side.
    pub fn remember_joiner(&self, session: &Session) -> Result<()> {
        if session.peer.is_empty() {
            return Ok(());
        }
        self.remember_device(&PairedDevice {
            id: session.peer.clone(),
            name: session.name.clone(),
            fingerprint: session.fingerprint.clone().unwrap_or_default(),
            host: None,
            port: None,
            last_sync: self.now(),
            shared: session.shared_area.is_some(),
        })
    }

    /// Starts the paired host with this Device's certificate on `bind`,
    /// on the port it had before while that is free, and keeps the port.
    pub fn start_paired_host(&self, bind: IpAddr) -> Result<Host> {
        let port = self
            .local_setting(PAIRED_PORT_KEY)?
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        let host = Host::start_paired(&self.tls_identity()?, bind, port)?;
        if host.port() != port {
            self.set_local_setting(PAIRED_PORT_KEY, &host.port().to_string())?;
        }
        Ok(host)
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
            .header(FORMAT_HEADER, OWN_FORMATS.encode())
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
    fn a_pin_host_hands_out_no_nonces_and_counts_no_tries_for_asking() -> TestResult {
        let identity = generate_identity()?;
        let host = Host::start(&identity, "111111", Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
        let agent = agent();
        let base = format!("https://127.0.0.1:{}", host.port());
        for _ in 0..=PIN_FAILURES_PER_ADDRESS {
            let res = agent.get(format!("{base}/challenge")).call()?;
            assert_eq!(res.status().as_u16(), 404);
        }
        let res = agent
            .get(format!("{base}/vector"))
            .header(DEVICE_HEADER, "phone")
            .header(NONCE_HEADER, "anything")
            .header(PROOF_HEADER, "x")
            .call()?;
        assert_eq!(res.status().as_u16(), 403, "no PIN, no entry");
        // The right PIN still gets in: asking was no wrong try.
        let stop = Arc::new(AtomicBool::new(false));
        let served = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    if let Some(request) = host.wait_request(Duration::from_millis(5)) {
                        request.reply(Response::status(200));
                    }
                }
            })
        };
        let res = agent
            .get(format!("{base}/vector"))
            .header(PIN_HEADER, "111111")
            .call()?;
        assert_eq!(res.status().as_u16(), 200);
        stop.store(true, Ordering::Relaxed);
        served.join().map_err(|_| "host thread")?;
        Ok(())
    }

    #[test]
    fn another_format_is_refused_with_its_reason() -> TestResult {
        let store = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        // No format written means format 1, which this build no longer
        // speaks: the joiner needs an update.
        let body = json!({"deviceId": "p", "entries": []}).to_string();
        let (response, session) = store.serve_sync(body.as_bytes(), "Desk")?;
        assert!(session.is_none());
        assert_eq!(response.status, 403);
        let refusal: Value = serde_json::from_slice(&response.body)?;
        assert_eq!(refusal["refusal"], "format");
        assert_eq!(refusal["formats"], "4-4");
        assert_eq!(
            store.take_update_notices()?,
            vec![("p".to_string(), UpdateNeeded::There)]
        );
        // A newer format: this Device needs the update.
        let body = json!({"format": 9, "deviceId": "p", "entries": []}).to_string();
        store.serve_sync(body.as_bytes(), "Desk")?;
        assert_eq!(
            store.take_update_notices()?,
            vec![("p".to_string(), UpdateNeeded::Here)]
        );
        Ok(())
    }

    #[test]
    fn format_ranges_meet_in_the_newest_both_speak() {
        let range = |oldest, newest| FormatRange { oldest, newest };
        assert_eq!(FormatRange::parse(Some("4-4")), range(4, 4));
        assert_eq!(FormatRange::parse(Some("3-6")), range(3, 6));
        assert_eq!(FormatRange::parse(Some("5")), range(5, 5));
        // None stated, or nothing readable: format 1.
        for text in [None, Some(""), Some("x"), Some("6-3"), Some("0-2")] {
            assert_eq!(FormatRange::parse(text), range(1, 1), "{text:?}");
        }
        assert_eq!(OWN_FORMATS.encode(), "4-4");
        assert_eq!(range(3, 6).common(range(2, 5)), Some(5));
        assert_eq!(range(4, 4).common(range(4, 7)), Some(4));
        assert_eq!(range(1, 3).common(range(4, 4)), None);
        assert_eq!(range(1, 3).update_needed(range(4, 4)), UpdateNeeded::There);
        assert_eq!(range(5, 6).update_needed(range(4, 4)), UpdateNeeded::Here);
    }

    #[test]
    fn a_joiner_sharing_no_format_is_refused_at_the_vector_and_announced_once() -> TestResult {
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let vector = |formats: Option<&str>| -> Result<Response> {
            let (reply, _) = channel();
            let mut headers = vec![(DEVICE_HEADER.to_string(), "phone".to_string())];
            if let Some(formats) = formats {
                headers.push((FORMAT_HEADER.to_string(), formats.to_string()));
            }
            let request = Request {
                method: "GET".into(),
                path: "/vector".into(),
                headers,
                body: Vec::new(),
                from: IpAddr::V4(Ipv4Addr::LOCALHOST),
                challenged: false,
                reply,
            };
            Ok(desk.serve(&request, "Desk")?.0)
        };
        let header = |r: &Response| {
            r.headers
                .iter()
                .find(|(k, _)| k == FORMAT_HEADER)
                .map(|(_, v)| v.clone())
        };
        let newer = vector(Some("5-7"))?;
        assert_eq!(newer.status, 403);
        assert_eq!(header(&newer).as_deref(), Some("4-4"));
        assert_eq!(
            desk.take_update_notices()?,
            vec![("phone".to_string(), UpdateNeeded::Here)]
        );
        // Every automatic try after it is refused, but announced once.
        assert_eq!(vector(Some("5-7"))?.status, 403);
        assert!(desk.take_update_notices()?.is_empty());
        // A joiner stating no formats speaks format 1.
        assert_eq!(vector(None)?.status, 403);
        assert_eq!(
            desk.take_update_notices()?,
            vec![("phone".to_string(), UpdateNeeded::There)]
        );
        // A joiner by pair code names no Device: every refusal is news.
        let unnamed = |formats: &str| -> Result<Response> {
            let (reply, _) = channel();
            let request = Request {
                method: "GET".into(),
                path: "/vector".into(),
                headers: vec![(FORMAT_HEADER.to_string(), formats.to_string())],
                body: Vec::new(),
                from: IpAddr::V4(Ipv4Addr::LOCALHOST),
                challenged: false,
                reply,
            };
            Ok(desk.serve(&request, "Desk")?.0)
        };
        for _ in 0..2 {
            assert_eq!(unnamed("1-3")?.status, 403);
            assert_eq!(
                desk.take_update_notices()?,
                vec![(LAN_SOURCE.to_string(), UpdateNeeded::There)]
            );
        }
        let shared = vector(Some("2-4"))?;
        assert_eq!(shared.status, 200);
        assert_eq!(header(&shared).as_deref(), Some("4-4"));
        // A sync that goes through forgets the notice.
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let body = sync_request(&phone, &desk.version_vector()?, "Phone", "", "")?;
        let phone_id = phone.device_id()?;
        desk.note_update_needed(&phone_id, UpdateNeeded::There)?;
        let (response, _) = desk.serve_sync(body.to_string().as_bytes(), "Desk")?;
        assert_eq!(response.status, 200);
        assert_eq!(header(&response).as_deref(), Some("4-4"));
        assert!(desk.take_update_notices()?.is_empty());
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
            shared: false,
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
    fn the_paired_proof_matches_the_dart_core() {
        assert_eq!(
            String::from_utf8(paired_proof_bytes("desk", "n0nce"))
                .ok()
                .as_deref(),
            Some(r#"["auto","desk","n0nce"]"#)
        );
    }

    /// The paired host's side as the desktop runs it in the background.
    fn serve_paired_until(
        path: PathBuf,
        host: Host,
        stop: Arc<AtomicBool>,
    ) -> JoinHandle<Result<Vec<Session>>> {
        std::thread::spawn(move || {
            let store = open(&path, 1)?;
            let mut sessions = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                let Some(request) = host.wait_request(Duration::from_millis(5)) else {
                    continue;
                };
                let (response, session) = store.serve_paired(&request, "Desk")?;
                sessions.extend(session);
                request.reply(response);
            }
            drop(host);
            Ok(sessions)
        })
    }

    /// A nonce from the paired host.
    fn challenge(agent: &ureq::Agent, base: &str) -> std::result::Result<String, String> {
        let mut res = agent
            .get(format!("{base}/challenge"))
            .call()
            .map_err(|e| e.to_string())?;
        let body: Value =
            serde_json::from_str(&res.body_mut().read_to_string().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        Ok(body["nonce"].as_str().unwrap_or_default().to_string())
    }

    /// Syncs as the Dart core's `pairedSync` does, as `device` signing
    /// with `signer`'s key and sending `body_store`'s entries; a refusal
    /// comes back as its status and text.
    fn join_paired(
        signer: &Store,
        device: &str,
        port: u16,
        host_device: &str,
    ) -> std::result::Result<Session, String> {
        let fail = |e: &dyn std::fmt::Display| e.to_string();
        let agent = agent();
        let base = format!("https://127.0.0.1:{port}");
        let nonce = challenge(&agent, &base)?;
        let proof = signer
            .signing_key()
            .map_err(|e| fail(&e))?
            .sign(&paired_proof_bytes(host_device, &nonce));
        let headers = [
            (DEVICE_HEADER, device.to_string()),
            (NONCE_HEADER, nonce),
            (PROOF_HEADER, proof),
            (FORMAT_HEADER, OWN_FORMATS.encode()),
        ];
        let mut get = agent.get(format!("{base}/vector"));
        for (k, v) in &headers {
            get = get.header(*k, v);
        }
        let mut res = get.call().map_err(|e| fail(&e))?;
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if res.status().as_u16() != 200 {
            return Err(format!("{} {text}", res.status().as_u16()));
        }
        let host_vector: HashMap<String, i64> =
            serde_json::from_str(&text).map_err(|e| fail(&e))?;
        let body =
            sync_request(signer, &host_vector, "Phone", host_device, "").map_err(|e| fail(&e))?;
        let mut post = agent
            .post(format!("{base}/sync"))
            .content_type("application/json");
        for (k, v) in &headers {
            post = post.header(*k, v);
        }
        let mut res = post
            .send(body.to_string().as_bytes())
            .map_err(|e| fail(&e))?;
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if res.status().as_u16() != 200 {
            return Err(format!("{} {text}", res.status().as_u16()));
        }
        let answer: Value = serde_json::from_str(&text).map_err(|e| fail(&e))?;
        apply_answer(signer, &answer).map_err(|e| fail(&e))
    }

    #[test]
    fn a_paired_device_syncs_without_the_pin() -> TestResult {
        let dir = tempfile::tempdir()?;
        let host_path = dir.path().join("desk.db");
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let phone_id = phone.device_id()?;
        let (desk_id, port) = {
            let desk = open(&host_path, 1)?;
            // Paired once by code: the phone's key verified, the phone
            // remembered.
            let first = sync_request(&phone, &HashMap::new(), "Phone", "", "")?;
            desk.serve_sync(first.to_string().as_bytes(), "Desk")?;
            desk.remember_device(&PairedDevice {
                id: phone_id.clone(),
                name: "Phone".into(),
                fingerprint: String::new(),
                host: None,
                port: None,
                last_sync: desk.now(),
                shared: false,
            })?;
            Notes::new(&desk).capture("Done on the desk", None, None)?;
            let host = desk.start_paired_host(IpAddr::V4(Ipv4Addr::LOCALHOST))?;
            assert_eq!(
                desk.local_setting(PAIRED_PORT_KEY)?,
                Some(host.port().to_string())
            );
            let port = host.port();
            drop(host);
            (desk.device_id()?, port)
        };
        let host = open(&host_path, 1)?.start_paired_host(IpAddr::V4(Ipv4Addr::LOCALHOST))?;
        assert_eq!(host.port(), port, "the port it had");
        assert_eq!(host.pin(), "");
        let stop = Arc::new(AtomicBool::new(false));
        let served = serve_paired_until(host_path.clone(), host, stop.clone());

        let agent = agent();
        let base = format!("https://127.0.0.1:{port}");
        // Neither a PIN nor a nonce from here lets anyone in.
        let res = agent
            .get(format!("{base}/vector"))
            .header(PIN_HEADER, "000000")
            .call()?;
        assert_eq!(res.status().as_u16(), 403);
        let res = agent
            .get(format!("{base}/vector"))
            .header(DEVICE_HEADER, &phone_id)
            .header(NONCE_HEADER, "made-up")
            .header(PROOF_HEADER, "x")
            .call()?;
        assert_eq!(res.status().as_u16(), 403);
        // Another key claiming the phone's id is declined.
        let impostor = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
        assert_eq!(
            join_paired(&impostor, &phone_id, port, &desk_id)
                .err()
                .as_deref(),
            Some("403 declined")
        );

        // The phone hosts too, and names its certificate.
        let phone_print = hex::encode(phone.tls_identity()?.fingerprint);
        let session = join_paired(&phone, &phone_id, port, &desk_id)?;
        assert!(!session.applied.is_empty());
        assert_eq!(session.name, "Desk");
        assert!(
            join_paired(&phone, &phone_id, port, &desk_id)?
                .applied
                .is_empty()
        );

        stop.store(true, Ordering::Relaxed);
        let sessions = served.join().map_err(|_| "host thread")??;
        assert_eq!(sessions.len(), 2);
        let desk = open(&host_path, 1)?;
        let paired = desk.paired_device(&phone_id)?.ok_or("paired")?;
        assert_eq!(paired.name, "Phone");
        assert_eq!(paired.fingerprint, phone_print, "so it can join the phone");
        Ok(())
    }

    #[test]
    fn a_joiners_certificate_is_kept_only_with_its_proof() -> TestResult {
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let pad = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
        let desk_id = desk.device_id()?;
        let request = |joiner: &Store, pin: &str| {
            sync_request(joiner, &desk.version_vector()?, "Phone", &desk_id, pin)
        };
        // Without a certificate of its own the joiner names none.
        assert!(request(&phone, "246810")?.get("fingerprint").is_none());
        let print = hex::encode(phone.tls_identity()?.fingerprint);
        assert_eq!(request(&phone, "246810")?["fingerprint"], json!(print));
        let (_, session) =
            answer_sync_with(&desk, &request(&phone, "246810")?, "Desk", None, "246810")?
                .ok_or("an answer")?;
        assert_eq!(session.fingerprint.as_deref(), Some(print.as_str()));
        // A proof for another PIN proves nothing here.
        let (_, session) =
            answer_sync_with(&desk, &request(&phone, "000000")?, "Desk", None, "246810")?
                .ok_or("an answer")?;
        assert_eq!(session.fingerprint, None);
        // Nor does naming another Device whose key is pinned here.
        answer_sync_with(&desk, &request(&pad, "246810")?, "Desk", None, "246810")?;
        let mut posing = request(&phone, "246810")?;
        posing["deviceId"] = json!(pad.device_id()?);
        let (_, session) =
            answer_sync_with(&desk, &posing, "Desk", None, "246810")?.ok_or("an answer")?;
        assert_eq!(session.peer, pad.device_id()?);
        assert_eq!(session.fingerprint, None);
        Ok(())
    }

    #[test]
    fn a_switched_off_or_forgotten_device_and_a_borrowed_name_are_declined() -> TestResult {
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let other = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
        for joiner in [&phone, &other] {
            let first = sync_request(joiner, &HashMap::new(), "Phone", "", "")?;
            desk.serve_sync(first.to_string().as_bytes(), "Desk")?;
            desk.remember_device(&PairedDevice {
                id: joiner.device_id()?,
                name: "Phone".into(),
                fingerprint: String::new(),
                host: None,
                port: None,
                last_sync: desk.now(),
                shared: false,
            })?;
        }
        let desk_id = desk.device_id()?;
        let request =
            |signer: &Store, path: &str, body: Vec<u8>, challenged: bool| -> Result<Request> {
                let proof = signer
                    .signing_key()?
                    .sign(&paired_proof_bytes(&desk_id, "n1"));
                let (reply, _) = channel();
                Ok(Request {
                    method: if body.is_empty() { "GET" } else { "POST" }.into(),
                    path: path.into(),
                    headers: vec![
                        (DEVICE_HEADER.into(), signer.device_id()?),
                        (NONCE_HEADER.into(), "n1".into()),
                        (PROOF_HEADER.into(), proof),
                        (FORMAT_HEADER.into(), OWN_FORMATS.encode()),
                    ],
                    body,
                    from: IpAddr::V4(Ipv4Addr::LOCALHOST),
                    challenged,
                    reply,
                })
            };
        let status = |r: &Request| -> Result<u16> { Ok(desk.serve_paired(r, "Desk")?.0.status) };
        assert_eq!(status(&request(&phone, "/vector", Vec::new(), true)?)?, 200);
        assert_eq!(
            status(&request(&phone, "/vector", Vec::new(), false)?)?,
            403,
            "a nonce this host never handed out"
        );
        // The phone's proof with the other Device's entries.
        let borrowed = sync_request(&other, &HashMap::new(), "Other", &desk_id, "")?;
        assert_eq!(
            status(&request(
                &phone,
                "/sync",
                borrowed.to_string().into_bytes(),
                true
            )?)?,
            403
        );
        desk.set_auto_sync(&phone.device_id()?, false)?;
        assert_eq!(status(&request(&phone, "/vector", Vec::new(), true)?)?, 403);
        desk.set_auto_sync(&phone.device_id()?, true)?;
        desk.forget_device(&phone.device_id()?)?;
        assert_eq!(status(&request(&phone, "/vector", Vec::new(), true)?)?, 403);
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
            challenged: false,
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

    fn get(path: &str) -> Request {
        let (reply, _) = channel();
        Request {
            method: "GET".into(),
            path: path.into(),
            headers: Vec::new(),
            body: Vec::new(),
            from: IpAddr::V4(Ipv4Addr::LOCALHOST),
            challenged: false,
            reply,
        }
    }

    #[test]
    fn a_host_shows_its_profile_to_whoever_holds_the_pin() -> TestResult {
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let (response, _) = desk.serve(&get("/profile"), "Desk")?;
        assert_eq!(response.status, 200);
        assert_eq!(serde_json::from_slice::<Value>(&response.body)?, json!({}));
        crate::profile::OwnProfile::new(&desk).apply_defaults("Ann", None)?;
        let (response, _) = desk.serve(&get("/profile"), "Desk")?;
        let body: Value = serde_json::from_slice(&response.body)?;
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let shown = phone
            .sent_profile_of(body.get("profile"), &desk.device_id()?)
            .ok_or("a Profile")?;
        assert_eq!(shown.profile.name, "Ann");
        assert!(body["profile"].get("fresh").is_none());
        Ok(())
    }

    #[test]
    fn a_host_inviting_says_so_and_hands_out_only_the_area() -> TestResult {
        use crate::shared::{AreaRole, Invite};
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let places = crate::Places::new(&desk);
        let house = places.add_area("Household")?;
        let inside =
            Notes::new(&desk).capture_with("In", Some(&house.id), None, &[vec![1; 8]], &[])?;
        let outside = Notes::new(&desk).capture_with("Out", None, None, &[vec![2; 8]], &[])?;
        Notes::new(&phone).capture("Phone private", None, None)?;
        for (store, name) in [(&desk, "Ann"), (&phone, "Ben")] {
            crate::profile::OwnProfile::new(store).apply_defaults(name, None)?;
        }
        let area = desk.share(&house.id)?;
        let invite = Invite {
            area: area.clone(),
            role: AreaRole::Member,
        };
        let vector = desk.serve_vector_inviting(true)?;
        assert!(
            vector
                .headers
                .contains(&(INVITE_HEADER.to_string(), "1".to_string()))
        );

        // A joiner that does not know it is invited is declined.
        let plain = sync_request(&phone, &desk.version_vector()?, "Pixel", "", "")?;
        let (response, _) =
            desk.serve_sync_inviting(plain.to_string().as_bytes(), "Desk", Some(&invite))?;
        assert_eq!(response.status, 403);
        assert_eq!(response.body, b"declined");
        assert!(
            Notes::new(&desk)
                .all()?
                .iter()
                .all(|n| n.text != "Phone private")
        );

        // Backups set up on both: no backup key goes to another person.
        let cheap = crate::backup::Kdf {
            memory: 64,
            iterations: 1,
            parallelism: 1,
        };
        let mut codes = Vec::new();
        for (store, name) in [(&desk, "Desk"), (&phone, "Pixel")] {
            crate::profile::OwnProfile::new(store).apply_defaults(name, None)?;
            codes.push(crate::backup::set_up(
                store,
                "a long passphrase",
                cheap,
                &mut crate::backup::system_random,
            )?);
            assert!(store.backup_key_to_send()?.is_some());
        }
        let body = sync_request_as(
            &phone,
            &desk.version_vector()?,
            "Pixel",
            &desk.device_id()?,
            "",
            Joining::Invited,
        )?;
        assert_eq!(body["entries"], json!([]));
        assert_eq!(body["areaKeys"], json!([]));
        assert!(body.get("backupKey").is_none());
        let (response, session) =
            desk.serve_sync_inviting(body.to_string().as_bytes(), "Desk", Some(&invite))?;
        assert_eq!(response.status, 200);
        assert_eq!(session.and_then(|s| s.shared_area), Some(area.clone()));
        let answer: Value = serde_json::from_slice(&response.body)?;
        let scopes: Vec<&str> = answer["areaKeys"]
            .as_array()
            .ok_or("keys")?
            .iter()
            .filter_map(|k| k["scope"].as_str())
            .collect();
        assert_eq!(scopes, vec![area.as_str()]);
        assert!(answer.get("backupKey").is_none());
        apply_answer_as(&phone, &answer, true, None)?;
        assert_eq!(desk.recovery_code()?.as_ref(), Some(&codes[0]));
        assert_eq!(phone.recovery_code()?.as_ref(), Some(&codes[1]));
        let texts: Vec<String> = Notes::new(&phone)
            .all()?
            .into_iter()
            .map(|n| n.text)
            .collect();
        assert!(texts.contains(&"In".to_string()));
        assert!(!texts.contains(&"Out".to_string()));
        assert_eq!(phone.access_in(&area, None)?, crate::AreaAccess::Member);

        // While inviting, only the Area's blobs go out.
        let photo = |n: &crate::Note| format!("/blob/photo/{}", n.photos[0]);
        let (ok, _) = desk.serve_inviting(&get(&photo(&inside)), "Desk", Some(&invite))?;
        assert_eq!(ok.status, 200);
        let (no, _) = desk.serve_inviting(&get(&photo(&outside)), "Desk", Some(&invite))?;
        assert_eq!(no.status, 404);
        Ok(())
    }

    #[test]
    fn an_invite_to_an_area_held_as_ones_own_is_declined() -> TestResult {
        use crate::shared::{AreaRole, Invite};
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
        let phone = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        let house = crate::Places::new(&phone).add_area("Household")?;
        // The desk holds the same Area as its own, unshared.
        for e in phone.all_entries()? {
            desk.insert(&e.wire())?;
        }
        let area = phone.share(&house.id)?;
        let invite = Invite {
            area,
            role: AreaRole::Member,
        };
        crate::profile::OwnProfile::new(&desk).apply_defaults("Ben", None)?;
        let host_profile = profile_answer(&desk)?;
        let body = sync_request_as(
            &phone,
            &desk.version_vector()?,
            "Pixel",
            &desk.device_id()?,
            "",
            Joining::Inviting(&invite, host_profile.get("profile")),
        )?;
        let (response, session) = desk.serve_sync(body.to_string().as_bytes(), "Desk")?;
        assert!(session.is_none());
        assert_eq!(response.body, b"declined");
        Ok(())
    }
}
