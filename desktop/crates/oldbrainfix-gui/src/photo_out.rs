//! A photo of a Note handed to another program from the desktop (#149):
//! a copy of it written for "Open in another program", and the system's
//! own opener started on it, `xdg-open` on Linux (the OpenURI portal
//! inside the Flatpak), `open` on macOS and `start` on Windows. Only the
//! standard library, so it is type-checked for every system.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long an opener is waited for to say it found no program; one
/// that runs on, as some show the photo themselves, is left running.
const OPENER_WAIT: Duration = Duration::from_secs(2);

/// The copy of a photo `bytes` named `name` in `dir`, written there
/// unless it is already: a stored photo has no file ending and may lie
/// where no other program reads, and the same name is the same photo.
pub fn copy_to_open(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64) {
        return Ok(path);
    }
    // Written beside, then renamed: a program never opens half a photo.
    let part = dir.join(format!(".{name}.part"));
    std::fs::write(&part, bytes)?;
    std::fs::rename(&part, &path)?;
    Ok(path)
}

/// Opens the file `path` in the program the system has for it; an
/// error when the opener cannot be started or says it found none.
pub fn open_with_system(path: &Path) -> io::Result<()> {
    let mut child = opener(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let start = Instant::now();
    while start.elapsed() < OPENER_WAIT {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!("the opener ended with {status}")))
            };
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Still running: it opened the photo itself. Reaped when it ends.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// The system's opener for `path`.
#[cfg(target_os = "macos")]
fn opener(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg(path);
    command
}

/// The system's opener for `path`: `start` with an empty window title,
/// without a console window.
#[cfg(windows)]
fn opener(path: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new("cmd");
    command
        .args(["/C", "start", ""])
        .raw_arg(format!("\"{}\"", path.display()))
        .creation_flags(CREATE_NO_WINDOW);
    command
}

/// The system's opener for `path`.
#[cfg(not(any(windows, target_os = "macos")))]
fn opener(path: &Path) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_holds_the_bytes_and_is_written_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = copy_to_open(dir.path(), "photo-abc.jpg", b"one").unwrap();
        assert_eq!(path, dir.path().join("photo-abc.jpg"));
        assert_eq!(std::fs::read(&path).unwrap(), b"one");
        // The same name again leaves the file as it is.
        std::fs::write(&path, b"two").unwrap();
        copy_to_open(dir.path(), "photo-abc.jpg", b"one").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        // One cut off is written again.
        std::fs::write(&path, b"o").unwrap();
        copy_to_open(dir.path(), "photo-abc.jpg", b"one").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"one");
    }

    #[test]
    fn a_missing_folder_is_made() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("a/b");
        let path = copy_to_open(&inner, "photo-x.png", b"png").unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"png");
    }
}
