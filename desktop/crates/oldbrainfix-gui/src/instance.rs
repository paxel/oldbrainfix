//! One running app per user: a second start hands its wish (open the
//! window, open the capture popup, or nothing for a login start) to the
//! running one over a local socket and ends, so reminders never come
//! twice. `oldbrainfix capture <text>` hands its text over the same way,
//! `oldbrainfix import <file>` the bundle's path, so only the running
//! app writes to the log, and `oldbrainfix open <file>` the path of a
//! Sent Item for the window to show. On systems without Unix sockets
//! every start runs on its own.
//!
//! Before it hands anything over, a start asks the running app for its
//! version (#290): one older than this start, or one that does not
//! answer, as 0.5 and before, is asked to quit, and this start runs in
//! its place once it has gone, so an update takes effect on the next
//! start.
//!
//! The protocol: one line with a word (`open`, `capture`, `quit`,
//! `none`), and nothing back; or the line `version`, answered by one
//! line with the running app's version; or the line `text`, then the text to the
//! end of the stream, answered by `ok` or by `error` and what went wrong;
//! or the line `import`, then the path to the end of the stream,
//! answered by `ok` and what came in, or by `error` and what went wrong;
//! or the line `file`, then the path to the end of the stream, and
//! nothing back: the window opens with it.

use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::tray::Command;

/// The word before a text to capture.
const TEXT: &str = "text";
/// The word before the path of a bundle to import.
const IMPORT: &str = "import";
/// The word before the path of a file for the window to open.
const FILE: &str = "file";
/// The word that asks the running app for its version.
#[cfg(unix)]
const VERSION: &str = "version";
const OK: &str = "ok";
const ERROR: &str = "error";

/// Captures a text handed over; what went wrong, if anything.
pub type Capture = Box<dyn Fn(&str) -> Result<(), String> + Send>;

/// Imports the bundle at a path handed over; what came in, or what went
/// wrong.
pub type Import = Box<dyn Fn(&Path) -> Result<String, String> + Send>;

/// Takes the path of a file for the window to open.
pub type OpenFile = Box<dyn Fn(&Path) + Send>;

/// How long a start waits for an older app it asked to quit to go: it
/// ends its syncs and its Backup first.
pub const QUIT_WAIT: Duration = Duration::from_secs(5);

/// How long a start waits for the running app to say its version; one
/// of 0.5 or before answers at once, by closing the connection.
#[cfg(unix)]
const VERSION_WAIT: Duration = Duration::from_secs(5);

/// What a start found at the socket, and did about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// No app runs: this start runs, or does its wish on its own.
    Alone,
    /// An older app ran and has gone: this start runs in its place.
    Replaced,
    /// An app of this version or newer runs: the wish goes to it.
    HandOver,
    /// An older app was asked to quit and is still there: this start
    /// says so and ends, so reminders never come twice.
    Stays,
}

/// Whether the running app's `theirs` is older than `own`: an answer
/// that is no version, or none at all, counts as older.
pub fn older(theirs: Option<&str>, own: &str) -> bool {
    let Ok(own) = semver::Version::parse(own) else {
        return false;
    };
    match theirs.map(semver::Version::parse) {
        Some(Ok(theirs)) => theirs < own,
        _ => true,
    }
}

/// Makes way for this start, of version `own`: asks the app running at
/// `socket` for its version and, when it is older, asks it to quit and
/// waits up to `wait` for its socket to be free.
#[cfg(unix)]
pub fn make_way(socket: &Path, own: &str, wait: Duration) -> Way {
    let Some(theirs) = running_version(socket) else {
        return Way::Alone;
    };
    if !older(theirs.as_deref(), own) {
        return Way::HandOver;
    }
    hand_over(socket, Some(Command::Quit));
    let until = std::time::Instant::now() + wait;
    loop {
        if std::os::unix::net::UnixStream::connect(socket).is_err() {
            return Way::Replaced;
        }
        if std::time::Instant::now() >= until {
            return Way::Stays;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(unix))]
pub fn make_way(_socket: &Path, _own: &str, _wait: Duration) -> Way {
    Way::Alone
}

/// The version the app running at `socket` says: `None` when none runs,
/// `Some(None)` when it says nothing.
#[cfg(unix)]
fn running_version(socket: &Path) -> Option<Option<String>> {
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket).ok()?;
    let answer = (|| -> std::io::Result<String> {
        stream.set_read_timeout(Some(VERSION_WAIT))?;
        writeln!(stream, "{VERSION}")?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer)?;
        Ok(answer.trim().to_string())
    })();
    Some(answer.ok().filter(|a| !a.is_empty()))
}

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
    handlers: (Capture, Import, OpenFile),
) -> std::io::Result<()> {
    listen_as(socket, env!("CARGO_PKG_VERSION"), commands, wake, handlers)
}

/// [`listen`], answering `version` with `version`.
#[cfg(unix)]
fn listen_as(
    socket: &Path,
    version: &'static str,
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
            } else if word == VERSION {
                drop(writeln!(reader.get_mut(), "{version}"));
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
mod tests;
