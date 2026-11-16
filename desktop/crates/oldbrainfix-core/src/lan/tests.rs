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
    let full =
        decode_pair_code(&encode_pair_code(host, 45231, "042817", &fp, false)).ok_or("no code")?;
    assert_eq!(full.fingerprint, fp);
    assert_eq!(
        pair_code_lines("r2m02_5xgnw_0aeg8_3188h_g7s65_mt0"),
        vec!["r2m02_5xgnw_0aeg8", "3188h_g7s65_mt0"]
    );
    assert!(decode_pair_code("nope").is_none());
    Ok(())
}

#[test]
fn pair_codes_of_a_later_version_need_an_update() {
    let fp = [7u8; 32];
    let typed = encode_pair_code(Ipv4Addr::new(10, 0, 0, 5), 8080, "999999", &fp, true);
    assert!(!pair_code_needs_update(&typed));
    // Its version first, at a length version 1 does not have.
    let later = format!("2{}", typed.replace('_', ""));
    assert!(decode_pair_code(&later).is_none());
    assert!(pair_code_needs_update(&later));
    assert!(pair_code_needs_update(&format!(
        "9 {} 00",
        typed.to_uppercase()
    )));
    // A 10.x code cut short starts with 1: no code, not a later one.
    assert!(!pair_code_needs_update(&typed[..typed.len() - 1]));
    assert!(!pair_code_needs_update(&format!(
        "1{}",
        typed.replace('_', "")
    )));
    // Too short for any code, or not base32.
    assert!(!pair_code_needs_update("2abcd"));
    assert!(!pair_code_needs_update(&format!("2{}", "!".repeat(30))));
    // At a length of version 1 it is read as version 1.
    assert!(!pair_code_needs_update(&format!("2{}", "0".repeat(27))));
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
    let note =
        Notes::new(&phone).capture_with("From the phone", None, None, &[], &[b"voice".to_vec()])?;
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
    let host_vector: HashMap<String, i64> = serde_json::from_str(&text).map_err(|e| fail(&e))?;
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
    let inside = Notes::new(&desk).capture_with("In", Some(&house.id), None, &[vec![1; 8]], &[])?;
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
