//! One running app per user: a second start hands its wish (open the
//! window, open the capture popup, or nothing for a login start) to the
//! running one and ends, so reminders never come twice. `oldbrainfix
//! capture <text>` hands its text over the same way, `oldbrainfix import
//! <file>` the bundle's path, so only the running app writes to the log,
//! and `oldbrainfix open <file>` the path of a Sent Item for the window
//! to show. On Unix the way is a local socket, on Windows a named pipe
//! of this user's (Q12); elsewhere every start runs on its own.
//!
//! Before it hands anything over, a start asks the running app for its
//! version (#290): one older than this start, or one that does not
//! answer, as 0.5 and before, is asked to quit, and this start runs in
//! its place once it has gone, so an update takes effect on the next
//! start.
//!
//! The messages: a word (`open`, `capture`, `quit`, `none`), and nothing
//! back; or `version`, answered by the running app's version; or `text`
//! and the text, answered by `ok` or by `error` and what went wrong; or
//! `import` and the path, answered by `ok` and what came in, or by
//! `error` and what went wrong; or `file` and the path, and nothing
//! back: the window opens with it. Markdown files given at the command
//! line (obf-1056) go one per message, each into a viewer of its own.
//! Both ways hand them to [`Running::answer`].
//!
//! On the socket a message is one line with its word, then what follows
//! it to the end of the stream, the start closing its side; the answer
//! is one line. A named pipe cannot be closed on one side only, so there
//! each message and each answer is a frame: its length in bytes on a
//! line, then its bytes, the word on the message's first line and what
//! follows it after; a message without an answer is answered by an
//! empty frame.

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
#[cfg(any(unix, windows))]
const VERSION_WAIT: Duration = Duration::from_secs(5);

/// How long a start waits for any other answer: a store busy for longer
/// than this is a failure, not a hang.
#[cfg(any(unix, windows))]
const ANSWER_WAIT: Duration = Duration::from_secs(20);

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
/// waits up to `wait` for it to be gone.
#[cfg(any(unix, windows))]
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
        if !reachable(socket) {
            return Way::Replaced;
        }
        if std::time::Instant::now() >= until {
            return Way::Stays;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(any(unix, windows)))]
pub fn make_way(_socket: &Path, _own: &str, _wait: Duration) -> Way {
    Way::Alone
}

/// Whether an app listens at `socket`.
#[cfg(unix)]
fn reachable(socket: &Path) -> bool {
    std::os::unix::net::UnixStream::connect(socket).is_ok()
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

/// Whether `word` is followed by a text or a path.
fn takes_payload(word: &str) -> bool {
    word == TEXT || word == IMPORT || word == FILE
}

/// What an answer of the running app says: what follows `ok`, or what
/// follows `error`.
fn said(answer: &str) -> Result<String, String> {
    match answer.trim().strip_prefix(OK) {
        Some(said) => Ok(said.trim().to_string()),
        None => Err(answer
            .trim()
            .strip_prefix(ERROR)
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .unwrap_or("no answer from the running app")
            .to_string()),
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

#[cfg(not(any(unix, windows)))]
pub fn hand_over(_socket: &Path, _wish: Option<Command>) -> bool {
    false
}

/// Hands `text` to the app running at `socket` to capture; `None` when
/// none runs, else whether it kept the text.
#[cfg(any(unix, windows))]
pub fn hand_over_text(socket: &Path, text: &str) -> Option<Result<(), String>> {
    ask(socket, TEXT, text).map(|answer| answer.map(drop))
}

/// Hands the bundle at `path` to the app running at `socket` to import;
/// `None` when none runs, else what came in or what went wrong.
#[cfg(any(unix, windows))]
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

#[cfg(not(any(unix, windows)))]
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
        stream.set_read_timeout(Some(ANSWER_WAIT))?;
        write!(stream, "{word}\n{payload}")?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer)?;
        Ok(answer)
    })();
    Some(match answer {
        Ok(answer) => said(&answer),
        Err(e) => Err(e.to_string()),
    })
}

#[cfg(not(any(unix, windows)))]
pub fn hand_over_text(_socket: &Path, _text: &str) -> Option<Result<(), String>> {
    None
}

#[cfg(not(any(unix, windows)))]
pub fn hand_over_import(_socket: &Path, _path: &Path) -> Option<Result<String, String>> {
    None
}

/// The running app's side: what it does with what later starts hand
/// over.
pub struct Running {
    /// The version it says.
    version: &'static str,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
    capture: Capture,
    import: Import,
    open: OpenFile,
}

impl Running {
    /// The running app of `version`, passing wishes to `commands` and
    /// calling `wake` after each; texts go to `capture`, the paths of
    /// bundles to `import`, the paths of files to open to `open`.
    pub fn new(
        version: &'static str,
        commands: Sender<Command>,
        wake: Box<dyn Fn() + Send>,
        (capture, import, open): (Capture, Import, OpenFile),
    ) -> Running {
        Running {
            version,
            commands,
            wake,
            capture,
            import,
            open,
        }
    }

    /// Does what `word`, with `payload` after it as read, asks; the line
    /// to answer, if any.
    pub fn answer(&self, word: &str, payload: Result<String, String>) -> Option<String> {
        let word = word.trim();
        if word == TEXT || word == IMPORT {
            let answer = match payload {
                Ok(text) if word == TEXT => (self.capture)(&text).map(|()| String::new()),
                Ok(path) => (self.import)(Path::new(&path)),
                Err(e) => Err(e),
            };
            return Some(match answer {
                Ok(said) if said.is_empty() => OK.to_string(),
                Ok(said) => format!("{OK} {said}"),
                Err(e) => format!("{ERROR} {e}"),
            });
        }
        if word == VERSION {
            return Some(self.version.to_string());
        }
        if word == FILE {
            if let Ok(path) = payload
                && !path.is_empty()
            {
                file_handed(Path::new(&path), &self.open, &self.commands, &*self.wake);
            }
            return None;
        }
        if let Some(c) = command(word) {
            let _ = self.commands.send(c);
            (self.wake)();
        }
        None
    }
}

/// A file handed to the running app, by a later start or by the system
/// ("Open with" on macOS): it goes to `open`, and the window opens.
pub fn file_handed(path: &Path, open: &OpenFile, commands: &Sender<Command>, wake: &dyn Fn()) {
    open(path);
    let _ = commands.send(Command::Open);
    wake();
}

/// Listens at `socket` for later starts and passes their wishes to
/// `commands`, calling `wake` after each; texts go to `capture`, the
/// paths of bundles to `import`, the paths of files to open to `open`,
/// with the window opened.
#[cfg(any(unix, windows))]
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
    handlers: (Capture, Import, OpenFile),
) -> std::io::Result<()> {
    use std::io::{BufRead, Read, Write};
    let running = Running::new(version, commands, wake, handlers);
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
            let word = line.trim().to_string();
            let mut payload = String::new();
            let payload = if takes_payload(&word) {
                reader
                    .read_to_string(&mut payload)
                    .map(|_| payload)
                    .map_err(|e| e.to_string())
            } else {
                Ok(payload)
            };
            if let Some(answer) = running.answer(&word, payload) {
                drop(writeln!(reader.get_mut(), "{answer}"));
            }
        }
    });
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub fn listen(
    _socket: &Path,
    _commands: Sender<Command>,
    _wake: Box<dyn Fn() + Send>,
    _: (Capture, Import, OpenFile),
) -> std::io::Result<()> {
    Ok(())
}

/// The largest frame read from a named pipe.
#[cfg(any(windows, test))]
const MAX_FRAME: usize = 64 << 20;

/// Writes `bytes` as one frame: their length on a line, then them.
#[cfg(any(windows, test))]
fn write_frame(to: &mut impl std::io::Write, bytes: &[u8]) -> std::io::Result<()> {
    writeln!(to, "{}", bytes.len())?;
    to.write_all(bytes)?;
    to.flush()
}

/// Reads one frame written by [`write_frame`].
#[cfg(any(windows, test))]
fn read_frame(from: &mut impl std::io::Read) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, ErrorKind, Read};
    let mut length = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        from.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            break;
        }
        if length.len() >= 20 {
            return Err(Error::new(ErrorKind::InvalidData, "no frame"));
        }
        length.push(byte[0]);
    }
    let length: usize = std::str::from_utf8(&length)
        .ok()
        .and_then(|l| l.parse().ok())
        .filter(|&l| l <= MAX_FRAME)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "no frame"))?;
    let mut bytes = Vec::with_capacity(length);
    from.take(length as u64).read_to_end(&mut bytes)?;
    if bytes.len() < length {
        return Err(Error::new(ErrorKind::UnexpectedEof, "frame cut short"));
    }
    Ok(bytes)
}

/// Serves one later start over a named pipe, `stream`: reads its
/// message, does what it asks and answers, with an empty frame where
/// nothing is answered.
#[cfg(any(windows, test))]
fn serve_framed(running: &Running, stream: &mut (impl std::io::Read + std::io::Write)) {
    let Ok(message) = read_frame(stream) else {
        return;
    };
    let message = String::from_utf8_lossy(&message);
    let (word, payload) = message.split_once('\n').unwrap_or((&message, ""));
    let answer = running.answer(word, Ok(payload.to_string()));
    drop(write_frame(stream, answer.unwrap_or_default().as_bytes()));
}

/// Sends `word` and `payload` as one frame over `stream` and reads the
/// answer frame, waiting at most `wait`; `None` when none came.
#[cfg(any(windows, test))]
fn exchange_framed<S>(mut stream: S, word: &str, payload: &str, wait: Duration) -> Option<String>
where
    S: std::io::Read + std::io::Write + Send + 'static,
{
    let message = if takes_payload(word) {
        format!("{word}\n{payload}")
    } else {
        word.to_string()
    };
    let (sent, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let answered = write_frame(&mut stream, message.as_bytes())
            .and_then(|()| read_frame(&mut stream))
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
        drop(sent.send(answered));
    });
    answer.recv_timeout(wait).ok()?.ok()
}

/// The name of the named pipe of the app whose socket would be at
/// `socket`, in this user's data folder: the path hashed (FNV-1a, which
/// every version works out alike), so each user's app has its own.
#[cfg(any(windows, test))]
fn pipe_name(socket: &Path) -> String {
    let hash = socket
        .to_string_lossy()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
    format!(r"\\.\pipe\io.github.paxel.oldbrainfix-{hash:016x}")
}

/// The Windows end of the named pipe; only creating and connecting it
/// needs the system's calls ([`pipe`]).
#[cfg(windows)]
mod pipe;

/// Connects to the named pipe of the app running at `socket`, waiting a
/// little while it serves another start; `None` when none runs.
#[cfg(windows)]
fn connect(socket: &Path) -> Option<std::fs::File> {
    /// `ERROR_PIPE_BUSY`: every instance of the pipe is serving.
    const BUSY: i32 = 231;
    let name = pipe_name(socket);
    let until = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&name)
        {
            Ok(pipe) => return Some(pipe),
            Err(e) if e.raw_os_error() == Some(BUSY) && std::time::Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}

#[cfg(windows)]
fn reachable(socket: &Path) -> bool {
    connect(socket).is_some()
}

#[cfg(windows)]
fn running_version(socket: &Path) -> Option<Option<String>> {
    let pipe = connect(socket)?;
    let answer = exchange_framed(pipe, VERSION, "", VERSION_WAIT);
    Some(
        answer
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty()),
    )
}

#[cfg(windows)]
pub fn hand_over(socket: &Path, wish: Option<Command>) -> bool {
    match connect(socket) {
        Some(pipe) => {
            exchange_framed(pipe, word(wish), "", ANSWER_WAIT);
            true
        }
        None => false,
    }
}

#[cfg(windows)]
pub fn hand_over_file(socket: &Path, path: &Path) -> bool {
    match connect(socket) {
        Some(pipe) => {
            exchange_framed(pipe, FILE, &path.to_string_lossy(), ANSWER_WAIT);
            true
        }
        None => false,
    }
}

#[cfg(windows)]
fn ask(socket: &Path, word: &str, payload: &str) -> Option<Result<String, String>> {
    let pipe = connect(socket)?;
    Some(match exchange_framed(pipe, word, payload, ANSWER_WAIT) {
        Some(answer) => said(&answer),
        None => Err("no answer from the running app".to_string()),
    })
}

#[cfg(windows)]
fn listen_as(
    socket: &Path,
    version: &'static str,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
    handlers: (Capture, Import, OpenFile),
) -> std::io::Result<()> {
    let running = Running::new(version, commands, wake, handlers);
    let name = pipe_name(socket);
    // The first, so no other program holds the name already.
    let mut waiting = pipe::create(&name, true)?;
    std::thread::spawn(move || {
        loop {
            let connected = pipe::accept(waiting);
            // The next start finds the next instance while this one is
            // served.
            let next = pipe::create(&name, false);
            if let Ok(mut stream) = connected {
                serve_framed(&running, &mut stream);
                // The answer is read before the pipe closes.
                drop(stream.sync_all());
            }
            match next {
                Ok(next) => waiting = next,
                Err(e) => {
                    eprintln!("oldbrainfix: {e}");
                    return;
                }
            }
        }
    });
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
