//! One running app per user: a second start hands its wish (open the
//! window, open the capture popup, or nothing for a login start) to the
//! running one over a local socket and ends, so reminders never come
//! twice. `oldbrainfix capture <text>` hands its text over the same way,
//! so only the running app writes to the log. On systems without Unix
//! sockets every start runs on its own.
//!
//! The protocol: one line with a word (`open`, `capture`, `quit`,
//! `none`), and nothing back; or the line `text`, then the text to the
//! end of the stream, answered by `ok` or by `error` and what went wrong.

use std::path::Path;
use std::sync::mpsc::Sender;

use crate::tray::Command;

/// The word before a text to capture.
const TEXT: &str = "text";
const OK: &str = "ok";
const ERROR: &str = "error";

/// Captures a text handed over; what went wrong, if anything.
pub type Capture = Box<dyn Fn(&str) -> Result<(), String> + Send>;

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
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
    let answer = (|| -> std::io::Result<String> {
        // A store busy for longer than this is a failure, not a hang.
        stream.set_read_timeout(Some(std::time::Duration::from_secs(20)))?;
        write!(stream, "{TEXT}\n{text}")?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer)?;
        Ok(answer)
    })();
    Some(match answer {
        Ok(answer) if answer.trim() == OK => Ok(()),
        Ok(answer) => Err(answer
            .trim()
            .strip_prefix(ERROR)
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .unwrap_or("no answer from the running app")
            .to_string()),
        Err(e) => Err(e.to_string()),
    })
}

#[cfg(not(unix))]
pub fn hand_over_text(_socket: &Path, _text: &str) -> Option<Result<(), String>> {
    None
}

/// Listens at `socket` for later starts and passes their wishes to
/// `commands`, calling `wake` after each; texts go to `capture`.
#[cfg(unix)]
pub fn listen(
    socket: &Path,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
    capture: Capture,
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
            if line.trim() == TEXT {
                let mut text = String::new();
                let answer = match reader.read_to_string(&mut text) {
                    Ok(_) => capture(&text),
                    Err(e) => Err(e.to_string()),
                };
                let answer = match answer {
                    Ok(()) => OK.to_string(),
                    Err(e) => format!("{ERROR} {e}"),
                };
                drop(writeln!(reader.get_mut(), "{answer}"));
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
    _capture: Capture,
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
        listen(&socket, tx, Box::new(|| {}), Box::new(|_| Ok(())))?;
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
        listen(&socket, tx, Box::new(|| {}), capture)?;
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
}
