//! One running app per user: a second start hands its wish (open the
//! window, open the capture popup, or nothing for a login start) to the
//! running one over a local socket and ends, so reminders never come
//! twice. `oldbrainfix capture <text>` hands its text over the same way,
//! `oldbrainfix import <file>` the bundle's path, so only the running
//! app writes to the log, and `oldbrainfix open <file>` the path of a
//! Sent Item for the window to show. On systems without Unix sockets
//! every start runs on its own.
//!
//! The protocol: one line with a word (`open`, `capture`, `quit`,
//! `none`), and nothing back; or the line `text`, then the text to the
//! end of the stream, answered by `ok` or by `error` and what went wrong;
//! or the line `import`, then the path to the end of the stream,
//! answered by `ok` and what came in, or by `error` and what went wrong;
//! or the line `file`, then the path to the end of the stream, and
//! nothing back: the window opens with it.

use std::path::Path;
use std::sync::mpsc::Sender;

use crate::tray::Command;

/// The word before a text to capture.
const TEXT: &str = "text";
/// The word before the path of a bundle to import.
const IMPORT: &str = "import";
/// The word before the path of a file for the window to open.
const FILE: &str = "file";
const OK: &str = "ok";
const ERROR: &str = "error";

/// Captures a text handed over; what went wrong, if anything.
pub type Capture = Box<dyn Fn(&str) -> Result<(), String> + Send>;

/// Imports the bundle at a path handed over; what came in, or what went
/// wrong.
pub type Import = Box<dyn Fn(&Path) -> Result<String, String> + Send>;

/// Takes the path of a file for the window to open.
pub type OpenFile = Box<dyn Fn(&Path) + Send>;

/// What a second start says.
fn word(command: Option<Command>) -> &'static str {
    match command {
        Some(Command::Open) => "open",
        Some(Command::Capture) => "capture",
        Some(Command::Quit) => "quit",
        None => "none",
    }
}

fn command(word: &str) -> Option<Command> {
    match word.trim() {
        "open" => Some(Command::Open),
        "capture" => Some(Command::Capture),
        "quit" => Some(Command::Quit),
        _ => None,
    }
}

/// Hands `wish` to the app already running at `socket`; true when one
/// took it, false when this start is the only one.
#[cfg(unix)]
pub fn hand_over(socket: &Path, wish: Option<Command>) -> bool {
    use std::io::Write;
    match std::os::unix::net::UnixStream::connect(socket) {
        Ok(mut stream) => {
            drop(writeln!(stream, "{}", word(wish)));
            true
        }
        Err(_) => false,
    }
}

#[cfg(not(unix))]
pub fn hand_over(_socket: &Path, _wish: Option<Command>) -> bool {
    false
}

/// Hands `text` to the app running at `socket` to capture; `None` when
/// none runs, else whether it kept the text.
#[cfg(unix)]
pub fn hand_over_text(socket: &Path, text: &str) -> Option<Result<(), String>> {
    ask(socket, TEXT, text).map(|answer| answer.map(drop))
}

/// Hands the bundle at `path` to the app running at `socket` to import;
/// `None` when none runs, else what came in or what went wrong.
#[cfg(unix)]
pub fn hand_over_import(socket: &Path, path: &Path) -> Option<Result<String, String>> {
    ask(socket, IMPORT, &path.to_string_lossy())
}

/// Hands the file at `path` to the app running at `socket`, for its
/// window to open; false when none runs.
#[cfg(unix)]
pub fn hand_over_file(socket: &Path, path: &Path) -> bool {
    use std::io::Write;
    match std::os::unix::net::UnixStream::connect(socket) {
        Ok(mut stream) => {
            drop(write!(stream, "{FILE}\n{}", path.to_string_lossy()));
            true
        }
        Err(_) => false,
    }
}

#[cfg(not(unix))]
pub fn hand_over_file(_socket: &Path, _path: &Path) -> bool {
    false
}

/// Sends `word` and `payload` to the app running at `socket` and reads
/// its answer: what follows `ok`, or what follows `error`.
#[cfg(unix)]
fn ask(socket: &Path, word: &str, payload: &str) -> Option<Result<String, String>> {
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
    let answer = (|| -> std::io::Result<String> {
        // A store busy for longer than this is a failure, not a hang.
        stream.set_read_timeout(Some(std::time::Duration::from_secs(20)))?;
        write!(stream, "{word}\n{payload}")?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer)?;
        Ok(answer)
    })();
    Some(match answer {
        Ok(answer) => match answer.trim().strip_prefix(OK) {
            Some(said) => Ok(said.trim().to_string()),
            None => Err(answer
                .trim()
                .strip_prefix(ERROR)
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .unwrap_or("no answer from the running app")
                .to_string()),
        },
        Err(e) => Err(e.to_string()),
    })
}

#[cfg(not(unix))]
pub fn hand_over_text(_socket: &Path, _text: &str) -> Option<Result<(), String>> {
    None
}

#[cfg(not(unix))]
pub fn hand_over_import(_socket: &Path, _path: &Path) -> Option<Result<String, String>> {
    None
}

/// Listens at `socket` for later starts and passes their wishes to
/// `commands`, calling `wake` after each; texts go to `capture`, the
/// paths of bundles to `import`, the paths of files to open to `open`,
/// with the window opened.
#[cfg(unix)]
pub fn listen(
    socket: &Path,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
    (capture, import, open): (Capture, Import, OpenFile),
) -> std::io::Result<()> {
    use std::io::{BufRead, Read, Write};
    // One left behind by a crash answers nobody.
    if socket.exists() {
        std::fs::remove_file(socket)?;
    }
    let listener = std::os::unix::net::UnixListener::bind(socket)?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = std::io::BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let word = line.trim();
            if word == TEXT || word == IMPORT {
                let mut text = String::new();
                let answer = match reader.read_to_string(&mut text) {
                    Ok(_) if word == TEXT => capture(&text).map(|()| String::new()),
                    Ok(_) => import(Path::new(&text)),
                    Err(e) => Err(e.to_string()),
                };
                let answer = match answer {
                    Ok(said) if said.is_empty() => OK.to_string(),
                    Ok(said) => format!("{OK} {said}"),
                    Err(e) => format!("{ERROR} {e}"),
                };
                drop(writeln!(reader.get_mut(), "{answer}"));
            } else if word == FILE {
                let mut path = String::new();
                if reader.read_to_string(&mut path).is_ok() && !path.is_empty() {
                    open(Path::new(&path));
                    let _ = commands.send(Command::Open);
                    wake();
                }
            } else if let Some(c) = command(&line) {
                let _ = commands.send(c);
                wake();
            }
        }
    });
    Ok(())
}

#[cfg(not(unix))]
pub fn listen(
    _socket: &Path,
    _commands: Sender<Command>,
    _wake: Box<dyn Fn() + Send>,
    _: (Capture, Import, OpenFile),
) -> std::io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
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
}
