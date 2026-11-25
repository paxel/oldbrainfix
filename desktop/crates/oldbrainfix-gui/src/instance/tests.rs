use std::path::PathBuf;
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

#[test]
fn several_markdown_files_go_to_the_running_app_one_after_another() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("instance.sock");
    let files = [dir.path().join("Garden.md"), dir.path().join("Roses.md")];
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
    for file in &files {
        assert!(hand_over_file(&socket, file));
    }
    let wait = std::time::Duration::from_secs(5);
    assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Open));
    assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Open));
    let opened = opened.lock().map(|o| o.clone()).unwrap_or_default();
    assert_eq!(opened, files);
    Ok(())
}

/// A running app of the version of this build, over a named pipe's
/// frames as on Windows (Q12), served on a socket pair: the start's end,
/// with the captured texts, the files opened and the commands it got.
type Framed = (
    std::os::unix::net::UnixStream,
    Arc<Mutex<Vec<String>>>,
    Arc<Mutex<Vec<PathBuf>>>,
    std::sync::mpsc::Receiver<Command>,
);

fn framed() -> std::io::Result<Framed> {
    let (start, mut app) = std::os::unix::net::UnixStream::pair()?;
    let (texts, files) = (
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );
    let (into_texts, into_files) = (texts.clone(), files.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    let running = Running::new(
        "0.8.0",
        tx,
        Box::new(|| {}),
        (
            Box::new(move |text| {
                if text == "fail" {
                    return Err("the log is locked".into());
                }
                into_texts
                    .lock()
                    .map_err(|e| e.to_string())?
                    .push(text.to_string());
                Ok(())
            }),
            Box::new(|path| Ok(format!("{} came in.", path.display()))),
            Box::new(move |path| {
                if let Ok(mut held) = into_files.lock() {
                    held.push(path.to_path_buf());
                }
            }),
        ),
    );
    std::thread::spawn(move || serve_framed(&running, &mut app));
    Ok((start, texts, files, rx))
}

const WAIT: Duration = Duration::from_secs(5);

#[test]
fn over_frames_a_text_is_captured_and_answered_as_over_the_socket() -> std::io::Result<()> {
    let (start, texts, _, rx) = framed()?;
    let answer = exchange_framed(start, TEXT, "buy milk\nand bread\n", WAIT);
    assert_eq!(answer.as_deref().map(said), Some(Ok(String::new())));
    assert_eq!(
        texts.lock().map(|t| t.clone()).unwrap_or_default(),
        ["buy milk\nand bread\n"]
    );
    let (start, _, _, _) = framed()?;
    let answer = exchange_framed(start, TEXT, "fail", WAIT);
    assert_eq!(
        answer.as_deref().map(said),
        Some(Err("the log is locked".into()))
    );
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn over_frames_the_version_an_import_a_file_and_a_wish_are_answered() -> std::io::Result<()> {
    let (start, _, _, _) = framed()?;
    assert_eq!(
        exchange_framed(start, VERSION, "", WAIT).as_deref(),
        Some("0.8.0")
    );
    let (start, _, _, _) = framed()?;
    let answer = exchange_framed(start, IMPORT, "/home/a b.obfsync", WAIT);
    assert_eq!(
        answer.as_deref().map(said),
        Some(Ok("/home/a b.obfsync came in.".into()))
    );
    let (start, _, files, rx) = framed()?;
    // A file is not answered: the frame comes back empty.
    let answer = exchange_framed(start, FILE, "/home/Garden notes.md", WAIT);
    assert_eq!(answer.as_deref(), Some(""));
    assert_eq!(rx.recv_timeout(WAIT).ok(), Some(Command::Open));
    assert_eq!(
        files.lock().map(|f| f.clone()).unwrap_or_default(),
        [PathBuf::from("/home/Garden notes.md")]
    );
    let (start, _, _, rx) = framed()?;
    assert_eq!(
        exchange_framed(start, word(Some(Command::Capture)), "", WAIT).as_deref(),
        Some("")
    );
    assert_eq!(rx.recv_timeout(WAIT).ok(), Some(Command::Capture));
    Ok(())
}

#[test]
fn a_frame_holds_any_bytes_and_a_broken_one_is_refused() -> std::io::Result<()> {
    let mut out = Vec::new();
    write_frame(&mut out, b"text\nline one\n\n0\n")?;
    assert_eq!(out, b"17\ntext\nline one\n\n0\n");
    assert_eq!(read_frame(&mut out.as_slice())?, b"text\nline one\n\n0\n");
    // Cut short, no length, too long.
    assert!(read_frame(&mut b"16\ntext".as_slice()).is_err());
    assert!(read_frame(&mut b"text\n".as_slice()).is_err());
    assert!(read_frame(&mut b"99999999999999999999999\n".as_slice()).is_err());
    assert!(read_frame(&mut format!("{}\n", MAX_FRAME + 1).as_bytes()).is_err());
    Ok(())
}

#[test]
fn an_app_that_never_answers_is_waited_for_no_longer_than_asked() -> std::io::Result<()> {
    let (start, _quiet) = std::os::unix::net::UnixStream::pair()?;
    let asked = std::time::Instant::now();
    assert_eq!(
        exchange_framed(start, VERSION, "", Duration::from_millis(200)),
        None
    );
    assert!(asked.elapsed() < Duration::from_secs(3));
    Ok(())
}

#[test]
fn each_users_pipe_has_a_name_of_its_own_every_version_works_out_alike() {
    let own = Path::new(r"C:\Users\Ada\AppData\Roaming\oldbrainfix\data\instance.sock");
    let other = Path::new(r"C:\Users\Bob\AppData\Roaming\oldbrainfix\data\instance.sock");
    let name = pipe_name(own);
    assert!(name.starts_with(r"\\.\pipe\io.github.paxel.oldbrainfix-"));
    assert_eq!(name, pipe_name(own));
    assert_ne!(name, pipe_name(other));
    // FNV-1a, fixed, so an update finds the pipe of the app before it.
    assert_eq!(
        pipe_name(Path::new("")),
        r"\\.\pipe\io.github.paxel.oldbrainfix-cbf29ce484222325"
    );
    // No backslash after the prefix, which a pipe's name may not hold.
    assert!(!name.trim_start_matches(r"\\.\pipe\").contains('\\'));
}
