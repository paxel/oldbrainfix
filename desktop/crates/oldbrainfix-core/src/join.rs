//! Joining another Device's LAN host by its pair code (ADR 0003), as the
//! Dart core's `lanSync` does: the desktop types the code a phone shows,
//! since it has no camera for the QR. The host's certificate must be the
//! one the code names before anything is sent; then `GET /vector`,
//! `POST /sync` with what the host lacks and this Device's own
//! certificate's fingerprint, and the photos and voice recordings both
//! ways. A host that invites this Device to a Shared Area (ADR 0013)
//! gets nothing of its own but its Profile; the other way round, this
//! Device inviting a host, is not offered here.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use serde_json::Value;

use crate::lan::{
    DEVICE_HEADER, FORMAT_HEADER, FormatRange, INVITE_HEADER, Joining, MAX_ENTRIES_BYTES,
    OWN_FORMATS, PIN_HEADER, PairInfo, Session, TYPED_FINGERPRINT_BYTES, apply_answer_as,
    fingerprint_of, is_private_host, sync_request_as,
};
use crate::pairing::PairedDevice;
use crate::store::Store;
use crate::sync::{UpdateNeeded, split_blob_ref};
use crate::{Error, Result};

/// Why a sync did not happen, as the Dart core's `SyncFailure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncFailure {
    /// The code names no address on a local network.
    NotPrivate,
    /// Nobody answered at the address.
    Unreachable,
    /// The certificate was not the one the code named.
    WrongHost,
    WrongPin,
    /// Too many wrong PINs: the host has to start again.
    Locked,
    /// The host said no to this Device.
    Declined,
    /// The two Devices share no wire format, and this one is the older:
    /// this app needs an update (ADR 0019).
    UpdateHere,
    /// The two Devices share no wire format, and the other one is the
    /// older: the app there needs an update.
    UpdateThere,
    /// Anything else went wrong on the way.
    Failed,
}

/// What a sync did.
#[derive(Debug, Clone)]
pub struct JoinResult {
    pub session: Session,
    pub entries_sent: usize,
    pub blobs_sent: usize,
    pub blobs_received: usize,
}

/// How long connecting may take.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// How long an answer may take: the host may ask its user first.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(300);

fn failed(failure: SyncFailure) -> Error {
    Error::Sync(failure)
}

/// Whether a certificate's fingerprint `actual` is the one the code
/// named, as the Dart core's `fingerprintMatches`: all of it from a QR,
/// its first bytes from a typed code.
pub fn fingerprint_matches(actual: &[u8], expected: &[u8]) -> bool {
    expected.len() >= TYPED_FINGERPRINT_BYTES
        && expected.len() <= actual.len()
        && actual.starts_with(expected)
}

/// Joins the host the pair code `info` names and syncs both ways:
/// entries, then photos and voice recordings. This Device goes by
/// `name`. After a session the host is remembered as a paired Device,
/// with the whole fingerprint of its certificate. Fails with
/// [`Error::Sync`]; an address outside the local network is refused
/// before anything is sent. The host may hold `POST /sync` while its user
/// is asked; this blocks meanwhile, so it runs off the window's thread.
pub fn lan_sync(store: &Store, info: &PairInfo, name: &str) -> Result<JoinResult> {
    if !is_private_host(&info.host) {
        return Err(failed(SyncFailure::NotPrivate));
    }
    let address: IpAddr = info
        .host
        .parse()
        .map_err(|_| failed(SyncFailure::NotPrivate))?;
    // So the host can join this Device the other way round later.
    store.tls_identity()?;
    let client = Client::new(
        SocketAddr::new(address, info.port),
        &info.fingerprint,
        &info.pin,
    )?;

    let vector = client
        .call("GET", "/vector", None)?
        .ok_or_else(|| failed(SyncFailure::Failed))?;
    // The newest format both speak; sharing none, nothing is sent.
    let host_formats = FormatRange::parse(vector.header(FORMAT_HEADER));
    if host_formats.common(OWN_FORMATS).is_none() {
        return Err(failed(update_failure(
            host_formats.update_needed(OWN_FORMATS),
        )));
    }
    let host_device = vector.header(DEVICE_HEADER).unwrap_or_default().to_string();
    let host_vector: HashMap<String, i64> =
        serde_json::from_slice(&vector.body).map_err(|_| failed(SyncFailure::Failed))?;
    let invited = vector.header(INVITE_HEADER) == Some("1");
    // A Device of this one's own group is no other person to be invited by.
    if invited && store.is_own_device(&host_device)? {
        return Err(failed(SyncFailure::Declined));
    }
    let shared = invited || store.is_shared_peer(&host_device)?;
    let joining = if invited {
        Joining::Invited
    } else {
        Joining::Plain
    };
    let request = sync_request_as(store, &host_vector, name, &host_device, &info.pin, joining)?;
    let sent = client
        .call("POST", "/sync", Some(request.to_string().into_bytes()))?
        .ok_or_else(|| failed(SyncFailure::Failed))?;
    let answer: Value =
        serde_json::from_slice(&sent.body).map_err(|_| failed(SyncFailure::Failed))?;
    if let Some(area) = answer
        .get("invite")
        .and_then(|i| i.get("area"))
        .and_then(Value::as_str)
        && store.refuses_invite(area)?
    {
        return Err(failed(SyncFailure::Declined));
    }
    let session = apply_answer_as(store, &answer, shared, None)?;
    if !session.peer.is_empty() {
        let seen = client.seen().unwrap_or_else(|| info.fingerprint.clone());
        store.remember_device(&PairedDevice {
            id: session.peer.clone(),
            name: session.name.clone(),
            fingerprint: hex::encode(seen),
            host: Some(info.host.clone()),
            port: Some(info.port),
            last_sync: store.now(),
            shared,
        })?;
        // The host took in everything beyond its vector, as it answered.
        store.note_held_by(&session.peer, &host_vector)?;
        let mine: HashMap<String, i64> = request
            .get("vector")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        store.note_held_by(&session.peer, &mine)?;
        store.update_no_longer_needed(&session.peer)?;
    }

    // The entries are in; whatever happens to the blobs now, the next
    // sync fetches what is still missing.
    let areas = if shared {
        Some(store.areas_shared_with(&session.peer)?)
    } else {
        None
    };
    let mut counts = (0, 0);
    match swap_blobs(store, &client, &answer, areas.as_ref(), &mut counts) {
        // Counted what landed; the rest waits for the next sync.
        Ok(()) | Err(Error::Sync(_)) => {}
        Err(e) => return Err(e),
    }
    Ok(JoinResult {
        session,
        entries_sent: request
            .get("entries")
            .and_then(Value::as_array)
            .map_or(0, Vec::len),
        blobs_received: counts.0,
        blobs_sent: counts.1,
    })
}

/// Fetches the blobs this Device lacks and sends those the host wants,
/// from another person's Device only those of the `areas` it shares,
/// counting both into `counts` (in, out) as they land.
fn swap_blobs(
    store: &Store,
    client: &Client,
    answer: &Value,
    areas: Option<&HashSet<String>>,
    counts: &mut (usize, usize),
) -> Result<()> {
    let allowed = |reference: &str| match areas {
        Some(areas) => store.blob_in(reference, areas),
        None => Ok(true),
    };
    for reference in store.missing_blobs()? {
        if !allowed(&reference)? {
            continue;
        }
        let Some(got) = client.call("GET", &format!("/blob/{reference}"), None)? else {
            continue;
        };
        let (kind, hash) = split_blob_ref(&reference);
        if store.put_blob(kind, hash, &got.body)? {
            counts.0 += 1;
        }
    }
    let wanted = answer.get("wantBlobs").and_then(Value::as_array);
    for reference in wanted.into_iter().flatten().filter_map(Value::as_str) {
        if !allowed(reference)? {
            continue;
        }
        let (kind, hash) = split_blob_ref(reference);
        let Some(bytes) = store.blob(kind, hash) else {
            continue;
        };
        client.call("POST", &format!("/blob/{reference}"), Some(bytes))?;
        counts.1 += 1;
    }
    Ok(())
}

fn update_failure(update: UpdateNeeded) -> SyncFailure {
    match update {
        UpdateNeeded::Here => SyncFailure::UpdateHere,
        UpdateNeeded::There => SyncFailure::UpdateThere,
    }
}

/// One answer: its headers by lower-case name, and its body.
struct Answer {
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// HTTPS to the host, one connection per request, its certificate pinned.
struct Client {
    address: SocketAddr,
    pin: String,
    config: Arc<rustls::ClientConfig>,
    seen: Arc<Mutex<Option<Vec<u8>>>>,
}

impl Client {
    fn new(address: SocketAddr, fingerprint: &[u8], pin: &str) -> Result<Client> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let seen = Arc::new(Mutex::new(None));
        let verifier = Pinned {
            expected: fingerprint.to_vec(),
            seen: seen.clone(),
            algorithms: provider.signature_verification_algorithms,
        };
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| Error::Invalid(format!("tls: {e}")))?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth();
        Ok(Client {
            address,
            pin: pin.to_string(),
            config: Arc::new(config),
            seen,
        })
    }

    /// The whole fingerprint of the certificate the host showed, once it
    /// matched.
    fn seen(&self) -> Option<Vec<u8>> {
        self.seen.lock().clone()
    }

    /// One request; none for a blob the host does not hold. A refusal
    /// fails as what it says.
    fn call(&self, method: &str, path: &str, body: Option<Vec<u8>>) -> Result<Option<Answer>> {
        let (status, answer) = match self.exchange(method, path, body.as_deref()) {
            Ok(done) => done,
            Err(Broken::Connect) => return Err(failed(SyncFailure::Unreachable)),
            // Refused at the handshake: another certificate than the
            // code names, unless it matched and broke later.
            Err(Broken::Tls) if self.seen().is_none() => {
                return Err(failed(SyncFailure::WrongHost));
            }
            Err(_) => return Err(failed(SyncFailure::Failed)),
        };
        if status == 403 {
            let text = String::from_utf8_lossy(&answer.body);
            let failure = if text.contains("locked") {
                SyncFailure::Locked
            } else if text.contains("declined") {
                SyncFailure::Declined
            } else if text.contains("refusal") {
                update_failure(
                    FormatRange::parse(answer.header(FORMAT_HEADER)).update_needed(OWN_FORMATS),
                )
            } else {
                SyncFailure::WrongPin
            };
            return Err(failed(failure));
        }
        // A blob the log mentions but nobody holds: skipped, never failed on.
        if status == 404 && path.starts_with("/blob/") {
            return Ok(None);
        }
        if status != 200 {
            return Err(failed(SyncFailure::Failed));
        }
        Ok(Some(answer))
    }

    fn exchange(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> std::result::Result<(u16, Answer), Broken> {
        let sock = TcpStream::connect_timeout(&self.address, CONNECT_TIMEOUT)
            .map_err(|_| Broken::Connect)?;
        let broken = |e: std::io::Error| {
            let tls = e
                .get_ref()
                .is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some());
            if tls { Broken::Tls } else { Broken::Io }
        };
        sock.set_read_timeout(Some(ANSWER_TIMEOUT))
            .map_err(broken)?;
        sock.set_write_timeout(Some(CONNECT_TIMEOUT))
            .map_err(broken)?;
        sock.set_nodelay(true).map_err(broken)?;
        let name = ServerName::IpAddress(self.address.ip().into());
        let conn =
            rustls::ClientConnection::new(self.config.clone(), name).map_err(|_| Broken::Tls)?;
        let mut tls = rustls::StreamOwned::new(conn, sock);
        let body = body.unwrap_or_default();
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n{PIN_HEADER}: {}\r\n{FORMAT_HEADER}: {}\r\n",
            self.address,
            body.len(),
            self.pin,
            OWN_FORMATS.encode(),
        );
        if !body.is_empty() && !path.starts_with("/blob/") {
            head.push_str("Content-Type: application/json\r\n");
        }
        head.push_str("\r\n");
        tls.write_all(head.as_bytes()).map_err(broken)?;
        tls.write_all(body).map_err(broken)?;
        tls.flush().map_err(broken)?;
        read_answer(&mut BufReader::new(tls)).map_err(broken)
    }
}

/// How a request broke on the way.
enum Broken {
    /// Nobody took the connection.
    Connect,
    /// The handshake or the encryption refused.
    Tls,
    Io,
}

/// Reads one answer: by its length, in chunks, or up to the end, never
/// more than [`MAX_ENTRIES_BYTES`].
fn read_answer<R: BufRead>(reader: &mut R) -> std::io::Result<(u16, Answer)> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let status: u16 = line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| std::io::Error::other("no status"))?;
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
    let answer = Answer {
        headers,
        body: Vec::new(),
    };
    let chunked = answer
        .header("transfer-encoding")
        .is_some_and(|v| v.to_lowercase().contains("chunked"));
    let length: Option<usize> = answer.header("content-length").and_then(|v| v.parse().ok());
    let body = if chunked {
        crate::lan::read_chunked(reader, MAX_ENTRIES_BYTES)?
            .ok_or_else(|| std::io::Error::other("too long"))?
    } else if let Some(length) = length {
        if length > MAX_ENTRIES_BYTES {
            return Err(std::io::Error::other("too long"));
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body)?;
        body
    } else {
        let mut body = Vec::new();
        let read = reader
            .take(MAX_ENTRIES_BYTES as u64 + 1)
            .read_to_end(&mut body);
        // A host that closes without saying goodbye still said it all.
        if let Err(e) = read
            && body.is_empty()
        {
            return Err(e);
        }
        if body.len() > MAX_ENTRIES_BYTES {
            return Err(std::io::Error::other("too long"));
        }
        body
    };
    Ok((status, Answer { body, ..answer }))
}

/// Takes the host's certificate only when its fingerprint is the one the
/// code named; the handshake's signatures are checked as usual.
#[derive(Debug)]
struct Pinned {
    expected: Vec<u8>,
    seen: Arc<Mutex<Option<Vec<u8>>>>,
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        let actual = fingerprint_of(end_entity.as_ref());
        if !fingerprint_matches(&actual, &self.expected) {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }
        *self.seen.lock() = Some(actual);
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Notes;
    use crate::lan::{Host, decode_pair_code};
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use std::net::Ipv4Addr;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::JoinHandle;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn open(path: &Path, seed: u8) -> Result<Store> {
        Store::open(path, ticking(), move || {
            Ok(SigningKey::from_seed([seed; 32]))
        })
    }

    /// A phone's host as the Dart core runs it: the store answers every
    /// request and remembers every joiner.
    fn serve_until(
        path: PathBuf,
        host: Host,
        stop: Arc<AtomicBool>,
    ) -> JoinHandle<Result<Vec<Session>>> {
        std::thread::spawn(move || {
            let store = open(&path, 1)?;
            let mut sessions = Vec::new();
            while !stop.load(Ordering::Relaxed) {
                let Some(request) = host.wait_request(Duration::from_millis(20)) else {
                    continue;
                };
                let (response, session) = store.serve(&request, "Phone")?;
                request.reply(response);
                if let Some(session) = session {
                    store.remember_joiner(&session)?;
                    sessions.push(session);
                }
            }
            Ok(sessions)
        })
    }

    fn failure(result: Result<JoinResult>) -> Option<SyncFailure> {
        match result {
            Err(Error::Sync(failure)) => Some(failure),
            _ => None,
        }
    }

    #[test]
    fn the_desktop_joins_a_phone_by_the_code_it_types() -> TestResult {
        let dir = tempfile::tempdir()?;
        let phone_path = dir.path().join("phone.db");
        let identity = {
            let phone = open(&phone_path, 1)?;
            Notes::new(&phone).capture_with(
                "From the phone",
                None,
                None,
                &[b"phone".to_vec()],
                &[],
            )?;
            phone.tls_identity()?
        };
        let host = Host::start(&identity, "246810", Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
        let code = host.pair_code(true);
        let stop = Arc::new(AtomicBool::new(false));
        let served = serve_until(phone_path.clone(), host, stop.clone());

        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;
        Notes::new(&desk).capture_with("From the desk", None, None, &[], &[b"desk".to_vec()])?;
        let info = decode_pair_code(&code).ok_or("a code")?;
        let joined = lan_sync(&desk, &info, "Desk");
        stop.store(true, Ordering::Relaxed);
        let sessions = served.join().map_err(|_| "host thread")??;
        let joined = joined?;
        assert_eq!(joined.session.name, "Phone");
        assert_eq!((joined.blobs_received, joined.blobs_sent), (1, 1));
        assert!(joined.entries_sent > 0);

        let phone = open(&phone_path, 1)?;
        for store in [&desk, &phone] {
            let mut texts: Vec<String> = Notes::new(store)
                .all()?
                .into_iter()
                .map(|n| n.text)
                .collect();
            texts.sort();
            assert_eq!(texts, ["From the desk", "From the phone"]);
            assert!(store.missing_blobs()?.is_empty());
        }
        // The desk keeps the phone's whole certificate from a typed code,
        // and the phone the desk's, so either joins the other later.
        let on_desk = desk.paired_device(&phone.device_id()?)?.ok_or("paired")?;
        assert_eq!(on_desk.fingerprint, hex::encode(&identity.fingerprint));
        assert_eq!(on_desk.port, Some(info.port));
        let desk_print = hex::encode(desk.tls_identity()?.fingerprint);
        assert_eq!(
            sessions.first().and_then(|s| s.fingerprint.clone()),
            Some(desk_print.clone())
        );
        let on_phone = phone.paired_device(&desk.device_id()?)?.ok_or("paired")?;
        assert_eq!(on_phone.fingerprint, desk_print);
        assert_eq!(desk.held_by(&phone.device_id()?)?, desk.version_vector()?);
        Ok(())
    }

    #[test]
    fn another_certificate_a_wrong_pin_and_other_networks_are_refused() -> TestResult {
        let dir = tempfile::tempdir()?;
        let phone_path = dir.path().join("phone.db");
        let identity = open(&phone_path, 1)?.tls_identity()?;
        let host = Host::start(&identity, "246810", Some(IpAddr::V4(Ipv4Addr::LOCALHOST)))?;
        let info = decode_pair_code(&host.pair_code(true)).ok_or("a code")?;
        let port = host.port();
        let stop = Arc::new(AtomicBool::new(false));
        let served = serve_until(phone_path.clone(), host, stop.clone());
        let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([2; 32])))?;

        let other = PairInfo {
            fingerprint: vec![0; TYPED_FINGERPRINT_BYTES],
            ..info.clone()
        };
        let wrong_host = failure(lan_sync(&desk, &other, "Desk"));
        let wrong_pin = failure(lan_sync(
            &desk,
            &PairInfo {
                pin: "000000".into(),
                ..info.clone()
            },
            "Desk",
        ));
        stop.store(true, Ordering::Relaxed);
        served.join().map_err(|_| "host thread")??;
        assert_eq!(wrong_host, Some(SyncFailure::WrongHost));
        assert_eq!(wrong_pin, Some(SyncFailure::WrongPin));
        assert!(desk.paired_devices()?.is_empty());

        let public = PairInfo {
            host: "8.8.8.8".into(),
            ..info.clone()
        };
        assert_eq!(
            failure(lan_sync(&desk, &public, "Desk")),
            Some(SyncFailure::NotPrivate)
        );
        // Nobody hosts there any more.
        let gone = PairInfo { port, ..info };
        assert_eq!(
            failure(lan_sync(&desk, &gone, "Desk")),
            Some(SyncFailure::Unreachable)
        );
        Ok(())
    }
}
