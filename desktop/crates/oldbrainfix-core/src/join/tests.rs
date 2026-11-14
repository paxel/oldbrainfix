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

#[test]
fn an_answer_may_weigh_as_much_as_its_limit_and_no_more() -> TestResult {
    // Just past a sync body's limit: too much for one, not for a blob.
    let length = MAX_ENTRIES_BYTES + 1;
    let answer = || {
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n");
        BufReader::new(
            std::io::Cursor::new(head.into_bytes()).chain(std::io::repeat(7).take(length as u64)),
        )
    };
    assert!(read_answer(&mut answer(), MAX_ENTRIES_BYTES).is_err());
    let (status, got) = read_answer(&mut answer(), MAX_BLOB_BYTES)?;
    assert_eq!((status, got.body.len()), (200, length));
    Ok(())
}

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
        Notes::new(&phone).capture_with("From the phone", None, None, &[b"phone".to_vec()], &[])?;
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
