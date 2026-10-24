//! One running app per user: a second start hands its wish (open the
//! window, or nothing for a login start) to the running one over a local
//! socket and ends, so reminders never come twice. On systems without
//! Unix sockets every start runs on its own.

use std::path::Path;
use std::sync::mpsc::Sender;

use crate::tray::Command;

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

/// Listens at `socket` for later starts and passes their wishes to
/// `commands`, calling `wake` after each.
#[cfg(unix)]
pub fn listen(
    socket: &Path,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
) -> std::io::Result<()> {
    use std::io::BufRead;
    // One left behind by a crash answers nobody.
    if socket.exists() {
        std::fs::remove_file(socket)?;
    }
    let listener = std::os::unix::net::UnixListener::bind(socket)?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if std::io::BufReader::new(stream).read_line(&mut line).is_ok()
                && let Some(c) = command(&line)
            {
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
) -> std::io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_second_start_hands_its_wish_to_the_first() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let socket = dir.path().join("instance.sock");
        assert!(!hand_over(&socket, Some(Command::Open)));
        let (tx, rx) = std::sync::mpsc::channel();
        listen(&socket, tx, Box::new(|| {}))?;
        assert!(hand_over(&socket, Some(Command::Open)));
        assert!(hand_over(&socket, None));
        assert!(hand_over(&socket, Some(Command::Capture)));
        let wait = std::time::Duration::from_secs(5);
        assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Open));
        assert_eq!(rx.recv_timeout(wait).ok(), Some(Command::Capture));
        Ok(())
    }
}
