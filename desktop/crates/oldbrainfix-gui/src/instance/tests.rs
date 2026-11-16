use std::sync::{Arc, Mutex};

use super::*;

#[test]
fn a_second_start_hands_its_wish_to_the_first() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    assert!(!hand_over(&socket, Some(Command::Open)));
    let (tx, rx) = std::sync::mpsc::channel();
    listen(
        &socket,
        tx,
        Box::new(|| {}),
        (
            Box::new(|_| Ok(())),
            Box::new(|_| Ok(String::new())),
            Box::new(|_| {}),
        ),
    )?;
    assert!(hand_over(&socket, Some(Command::Open)));
    assert!(hand_over(&socket, None));
    assert!(hand_over(&socket, Some(Command::Capture)));
    let wait = std::time::Duration::from_secs(5);
    assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Open));
    assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Capture));
    Ok(())
}

/// The handlers of a running app that takes nothing in.
fn idle() -> (Capture, Import, OpenFile) {
    (
        Box::new(|_| Ok(())),
        Box::new(|_| Ok(String::new())),
        Box::new(|_| {}),
    )
}

/// A running app of `version` at `socket` that, asked to quit,
/// goes, its socket removed as the app removes it at its end; the
/// commands it got.
fn running_app(
    socket: &Path,
    version: &'static str,
) -> std::io::Result<std::sync::mpsc::Receiver<Command>> {
    let (tx, rx) = std::sync::mpsc::channel();
    listen_as(socket, version, tx, Box::new(|| {}), idle())?;
    let (seen_tx, seen_rx) = std::sync::mpsc::channel();
    let path = socket.to_path_buf();
    std::thread::spawn(move || {
        for command in rx {
            if command == Command::Quit {
                let _ = std::fs::remove_file(&path);
            }
            let _ = seen_tx.send(command);
        }
    });
    Ok(seen_rx)
}

/// A running app of 0.5 or before at `socket`: it reads every word
/// and answers none; asked to quit it goes when `quits`.
fn app_of_0_5(socket: &Path, quits: bool) -> std::io::Result<()> {
    use std::io::BufRead;
    let listener = std::os::unix::net::UnixListener::bind(socket)?;
    let path = socket.to_path_buf();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            let _ = std::io::BufReader::new(stream).read_line(&mut line);
            if line.trim() == "quit" && quits {
                let _ = std::fs::remove_file(&path);
                return;
            }
        }
    });
    Ok(())
}

#[test]
fn versions_are_compared_as_semantic_versions() {
    assert!(older(Some("0.5.1"), "0.6.0"));
    assert!(older(Some("0.9.3"), "0.10.0"));
    assert!(older(Some("0.6.0-rc.1"), "0.6.0"));
    assert!(!older(Some("0.6.0"), "0.6.0"));
    assert!(!older(Some("0.10.0"), "0.9.3"));
    assert!(!older(Some("1.0.0"), "0.6.0"));
    // No answer, or none that is a version, counts as older.
    assert!(older(None, "0.6.0"));
    assert!(older(Some("ok"), "0.6.0"));
}

#[test]
fn the_running_app_says_its_version() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    assert_eq!(running_version(&socket), None);
    let (tx, rx) = std::sync::mpsc::channel();
    listen(&socket, tx, Box::new(|| {}), idle())?;
    assert_eq!(
        running_version(&socket),
        Some(Some(env!("CARGO_PKG_VERSION").to_string()))
    );
    // Asking is no wish.
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn no_app_running_lets_this_start_run_alone() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    assert_eq!(make_way(&socket, "0.6.0", QUIT_WAIT), Way::Alone);
    // One left behind by a crash answers nobody.
    drop(std::os::unix::net::UnixListener::bind(&socket)?);
    assert!(socket.exists());
    assert_eq!(make_way(&socket, "0.6.0", QUIT_WAIT), Way::Alone);
    Ok(())
}

#[test]
fn an_older_app_answering_its_version_is_asked_to_quit_and_replaced() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    let seen = running_app(&socket, "0.5.9")?;
    assert_eq!(make_way(&socket, "0.6.0", QUIT_WAIT), Way::Replaced);
    let wait = std::time::Duration::from_secs(5);
    assert_eq!(seen.recv_timeout(wait).ok(), Some(Command::Quit));
    assert!(!hand_over(&socket, Some(Command::Open)), "it has gone");
    Ok(())
}

#[test]
fn an_app_answering_nothing_counts_as_older_and_is_replaced() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    app_of_0_5(&socket, true)?;
    assert_eq!(make_way(&socket, "0.6.0", QUIT_WAIT), Way::Replaced);
    assert!(!socket.exists());
    Ok(())
}

#[test]
fn an_app_of_the_same_version_or_newer_takes_the_wish() -> std::io::Result<()> {
    for theirs in ["0.6.0", "0.7.0"] {
        let dir = tempfile::tempdir()?;
        let socket = dir.path().join("instance.sock");
        let seen = running_app(&socket, theirs)?;
        assert_eq!(make_way(&socket, "0.6.0", QUIT_WAIT), Way::HandOver);
        assert!(hand_over(&socket, Some(Command::Open)));
        // The first command it got is the wish: it was not asked to quit.
        let wait = std::time::Duration::from_secs(5);
        assert_eq!(seen.recv_timeout(wait).ok(), Some(Command::Open));
        assert!(socket.exists());
    }
    Ok(())
}

#[test]
fn an_older_app_that_does_not_quit_in_time_stays_and_this_start_ends() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    app_of_0_5(&socket, false)?;
    let wait = std::time::Duration::from_millis(300);
    let asked = std::time::Instant::now();
    assert_eq!(make_way(&socket, "0.6.0", wait), Way::Stays);
    assert!(asked.elapsed() < std::time::Duration::from_secs(3));
    assert!(socket.exists());
    Ok(())
}

#[test]
fn a_text_is_captured_by_the_running_app_and_answered() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    assert_eq!(hand_over_text(&socket, "buy milk"), None);
    let kept = Arc::new(Mutex::new(Vec::<String>::new()));
    let into = kept.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let capture: Capture = Box::new(move |text| {
        if text.contains("fail") {
            return Err("the log is locked".into());
        }
        into.lock().map_err(|e| e.to_string())?.push(text.into());
        Ok(())
    });
    listen(
        &socket,
        tx,
        Box::new(|| {}),
        (capture, Box::new(|_| Ok(String::new())), Box::new(|_| {})),
    )?;
    assert_eq!(hand_over_text(&socket, "buy milk"), Some(Ok(())));
    // Lines and blanks come through as given; a word is no command.
    assert_eq!(hand_over_text(&socket, "open\nthe window\n"), Some(Ok(())));
    assert_eq!(
        hand_over_text(&socket, "fail"),
        Some(Err("the log is locked".into()))
    );
    let kept = kept.lock().map(|k| k.clone()).unwrap_or_default();
    assert_eq!(kept, ["buy milk", "open\nthe window\n"]);
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn a_bundle_is_imported_by_the_running_app_and_answered() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    let file = dir.path().join("a b.obfsync");
    assert_eq!(hand_over_import(&socket, &file), None);
    let (tx, _rx) = std::sync::mpsc::channel();
    let import: Import = Box::new(|path| {
        if path.ends_with("a b.obfsync") {
            Ok("3 changes came in.\nAnd more.".into())
        } else {
            Err("no sync bundle".into())
        }
    });
    listen(
        &socket,
        tx,
        Box::new(|| {}),
        (Box::new(|_| Ok(())), import, Box::new(|_| {})),
    )?;
    assert_eq!(
        hand_over_import(&socket, &file),
        Some(Ok("3 changes came in.\nAnd more.".into()))
    );
    assert_eq!(
        hand_over_import(&socket, &dir.path().join("other")),
        Some(Err("no sync bundle".into()))
    );
    Ok(())
}

#[test]
fn a_file_to_open_goes_to_the_running_window() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    let file = dir.path().join("a note.oldbrain");
    assert!(!hand_over_file(&socket, &file));
    let opened = Arc::new(Mutex::new(Vec::new()));
    let into = opened.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    listen(
        &socket,
        tx,
        Box::new(|| {}),
        (
            Box::new(|_| Ok(())),
            Box::new(|_| Ok(String::new())),
            Box::new(move |path| {
                if let Ok(mut held) = into.lock() {
                    held.push(path.to_path_buf());
                }
            }),
        ),
    )?;
    assert!(hand_over_file(&socket, &file));
    let wait = std::time::Duration::from_secs(5);
    assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Open));
    let opened = opened.lock().map(|o| o.clone()).unwrap_or_default();
    assert_eq!(opened, [file]);
    Ok(())
}
