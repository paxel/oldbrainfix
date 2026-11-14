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
fn a_tool_call_wakes_the_app_so_it_needs_no_polling() -> TestResult {
    use std::sync::atomic::AtomicUsize;
    let store = store();
    Notes::new(&store).capture("Milk", None, None)?;
    let woken = Arc::new(AtomicUsize::new(0));
    let port = {
        let woken = woken.clone();
        AiPort::open_waking(
            &store,
            Arc::new(move || {
                woken.fetch_add(1, Ordering::SeqCst);
            }),
        )?
    };
    let (url, token) = (port.url(), port.token().to_string());
    let call = rpc("tools/call", json!({"name": "list_notes", "arguments": {}}));
    let (answered, status) = std::thread::scope(|s| {
        let client = s.spawn(|| post(&url, Some(&token), &call).map_err(|e| e.to_string()));
        // Nothing is polled: the call is taken only once the wake came.
        while woken.load(Ordering::SeqCst) == 0 && !client.is_finished() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let answered = port.next_call().map(|call| call.refuse("seen"));
        (answered.is_some(), client.join())
    });
    assert!(answered, "the call waited when the wake came");
    assert_eq!(status.map_err(|_| "the client")??.0, 200);
    assert_eq!(woken.load(Ordering::SeqCst), 1, "one wake for one call");
    // Calls that need no answer from the app wake nothing.
    post(&url, Some(&token), &rpc("ping", json!({})))?;
    assert_eq!(woken.load(Ordering::SeqCst), 1);
    Ok(())
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
    // An outside client answers by the Ask panel's rule (#294).
    let told = seen[0]
        .pointer("/result/instructions")
        .and_then(Value::as_str)
        .ok_or("instructions")?;
    for rule in [
        "answer only from the Notes you read through this port",
        "never from general knowledge",
        "link the Note of every statement",
        "oldbrainfix://note/",
        "say that it is not in their Notes",
    ] {
        assert!(told.contains(rule), "{rule}");
    }
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
