//! The launcher entry on Linux, written by the app itself, as cat(a)log's
//! desk writes its own.
//!
//! A Homebrew or tarball install and a downloaded AppImage are a binary
//! and nothing else: no `.desktop` entry, no icon in a place the menu
//! reads, so the app is not in the menu and on Wayland its window shows
//! a generic icon, the compositor finding the icon through the entry
//! named after the window's app id. The app closes that gap on every
//! start: it writes or refreshes the icon and the launcher in the user's
//! XDG data dirs, what `packaging/linux/install-icon.sh` does for a
//! checkout and the formula does once at install. A Flatpak exports its
//! own entry and skips this.
//!
//! Rules: only an entry carrying the `X-Oldbrainfix-Managed` marker is
//! ever overwritten, a hand-written or distro-installed one is left
//! alone; nothing is written when the content is current; every failure
//! is swallowed, a missing menu entry is never worth failing a start over.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use oldbrainfix_core::{APP_ID, APP_NAME};

use crate::{ICON, ICON_NAME};

/// The marker claiming an entry as ours to update.
const MARKER: &str = "X-Oldbrainfix-Managed=true";

/// Registers icon and launcher for the running binary; call from a
/// thread of its own. Best effort: errors, a home-less environment and a
/// Flatpak are ignored.
pub fn register() {
    if std::env::var_os("FLATPAK_ID").is_some() {
        return;
    }
    let Some(data_home) = data_home() else {
        return;
    };
    let Some(exec) = exec_path() else {
        return;
    };
    if let Ok(true) = register_at(&data_home, &exec) {
        refresh_caches(&data_home);
    }
}

/// `$XDG_DATA_HOME`, else `$HOME/.local/share`.
fn data_home() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_DATA_HOME")
        && !x.is_empty()
    {
        return Some(PathBuf::from(x));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
}

/// What the launcher executes: the AppImage file when running from one
/// (`$APPIMAGE`, the mount the binary runs from is gone after exit),
/// else the running executable, through the stable Homebrew symlink
/// when the binary lives in a Cellar.
fn exec_path() -> Option<PathBuf> {
    if let Some(ai) = std::env::var_os("APPIMAGE")
        && !ai.is_empty()
    {
        return Some(PathBuf::from(ai));
    }
    let exe = std::env::current_exe().ok()?;
    Some(stable_brew_path(&exe).unwrap_or(exe))
}

/// The `<prefix>/bin/oldbrainfix` symlink for a binary in a Homebrew
/// Cellar, verified to point back at it; none for everything else.
/// `current_exe` resolves into the versioned Cellar directory, which the
/// next `brew upgrade` deletes; a launcher pinned there dies with it. The
/// prefix's `bin` is repointed on every upgrade, so that path lives on.
fn stable_brew_path(exe: &Path) -> Option<PathBuf> {
    let prefix = exe
        .ancestors()
        .find(|a| a.file_name() == Some("Cellar".as_ref()))?
        .parent()?;
    let candidate = prefix.join("bin").join(exe.file_name()?);
    (fs::canonicalize(&candidate).ok()? == fs::canonicalize(exe).ok()?).then_some(candidate)
}

/// Writes icon and launcher under `data_home` for `exec`. Whether
/// anything was written; the caches are refreshed only then.
fn register_at(data_home: &Path, exec: &Path) -> io::Result<bool> {
    let mut wrote = false;

    let icon_path = data_home.join(format!("icons/hicolor/256x256/apps/{ICON_NAME}.png"));
    if fs::read(&icon_path).ok().as_deref() != Some(ICON) {
        write_atomic(&icon_path, ICON)?;
        wrote = true;
    }

    let desktop_path = data_home.join(format!("applications/{APP_ID}.desktop"));
    let desired = desktop_entry(exec);
    match fs::read_to_string(&desktop_path) {
        Ok(current) if !current.contains(MARKER) => {}
        Ok(current) if current == desired => {}
        _ => {
            write_atomic(&desktop_path, desired.as_bytes())?;
            wrote = true;
        }
    }
    Ok(wrote)
}

/// The launcher, as `packaging/linux/io.github.paxel.oldbrainfix.desktop`
/// has it, with Exec and TryExec pointed at the binary: TryExec hides the
/// entry once an AppImage was moved away instead of failing.
fn desktop_entry(exec: &Path) -> String {
    let exec_str = exec.to_string_lossy();
    format!(
        "[Desktop Entry]\n\
         Name={APP_NAME}\n\
         Comment=Capture a thought in one tap. Get nagged until it is sorted or done.\n\
         TryExec={exec_str}\n\
         Exec={}\n\
         Icon={ICON_NAME}\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=Utility;\n\
         StartupWMClass={APP_ID}\n\
         {MARKER}\n",
        quote_exec(&exec_str),
    )
}

/// A path for an `Exec=` line: double-quoted with the reserved characters
/// escaped, unless it is plain enough to stand bare.
fn quote_exec(path: &str) -> String {
    let plain = path
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+' | ':' | '@'));
    if plain {
        return path.to_owned();
    }
    let mut out = String::with_capacity(path.len() + 2);
    out.push('"');
    for c in path.chars() {
        if matches!(c, '"' | '\\' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    // A literal percent is doubled in Exec.
    out.replace('%', "%%")
}

/// Parent directories made, then a same-directory temp file renamed into
/// place, so a crash leaves no half-written launcher.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent"))?;
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

/// Pokes the desktop's caches so the entry shows without a new login.
/// Every tool is optional; absence and failure are both fine.
fn refresh_caches(data_home: &Path) {
    let run = |cmd: &str, args: &[&std::ffi::OsStr]| {
        let _ = std::process::Command::new(cmd)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    };
    let icons = data_home.join("icons/hicolor");
    let apps = data_home.join("applications");
    run(
        "gtk-update-icon-cache",
        &["-f".as_ref(), "-t".as_ref(), icons.as_os_str()],
    );
    run("update-desktop-database", &[apps.as_os_str()]);
    run("kbuildsycoca6", &[]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRY: &str = "applications/io.github.paxel.oldbrainfix.desktop";

    #[test]
    fn a_fresh_register_writes_the_icon_and_a_marked_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let exec = Path::new("/opt/brew/bin/oldbrainfix");
        assert!(register_at(dir.path(), exec).unwrap());
        assert_eq!(
            fs::read(
                dir.path()
                    .join("icons/hicolor/256x256/apps/oldbrainfix.png")
            )
            .unwrap(),
            ICON
        );
        let desktop = fs::read_to_string(dir.path().join(ENTRY)).unwrap();
        assert!(desktop.contains("\nExec=/opt/brew/bin/oldbrainfix\n"));
        assert!(desktop.contains("\nTryExec=/opt/brew/bin/oldbrainfix\n"));
        assert!(desktop.contains("\nIcon=oldbrainfix\n"));
        assert!(desktop.contains("\nStartupWMClass=io.github.paxel.oldbrainfix\n"));
        assert!(desktop.contains(MARKER));
        // Current content is not written again; a moved binary is.
        assert!(!register_at(dir.path(), exec).unwrap());
        assert!(register_at(dir.path(), Path::new("/new/oldbrainfix")).unwrap());
        let desktop = fs::read_to_string(dir.path().join(ENTRY)).unwrap();
        assert!(desktop.contains("\nExec=/new/oldbrainfix\n"));
    }

    #[test]
    fn a_launcher_without_the_marker_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let desktop_path = dir.path().join(ENTRY);
        fs::create_dir_all(desktop_path.parent().unwrap()).unwrap();
        let foreign = "[Desktop Entry]\nName=my brain\nExec=/home/me/custom\n";
        fs::write(&desktop_path, foreign).unwrap();
        register_at(dir.path(), Path::new("/opt/oldbrainfix")).unwrap();
        assert_eq!(fs::read_to_string(&desktop_path).unwrap(), foreign);
    }

    #[test]
    fn the_shipped_launcher_is_marked_and_matches_the_written_one() {
        // The packaged entry (tarball, Homebrew, .deb, AppImage) carries
        // the marker, so the app may heal it, and differs from the one
        // the app writes only in Exec and TryExec.
        let shipped = include_str!("../../../packaging/linux/io.github.paxel.oldbrainfix.desktop");
        let written = desktop_entry(Path::new("oldbrainfix"));
        let keys = |entry: &str| -> Vec<String> {
            entry
                .lines()
                .filter(|l| !l.starts_with('#') && !l.starts_with("TryExec="))
                .map(str::to_owned)
                .collect()
        };
        assert_eq!(keys(shipped), keys(&written));
    }

    #[test]
    fn a_cellar_binary_registers_the_stable_bin_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let cellar_bin = dir.path().join("Cellar/oldbrainfix/0.1.0/libexec");
        fs::create_dir_all(&cellar_bin).unwrap();
        let exe = cellar_bin.join("oldbrainfix");
        fs::write(&exe, b"binary").unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(
            "../Cellar/oldbrainfix/0.1.0/libexec/oldbrainfix",
            bin.join("oldbrainfix"),
        )
        .unwrap();
        assert_eq!(stable_brew_path(&exe), Some(bin.join("oldbrainfix")));
        // A bin symlink naming another binary is not ours; no Cellar, itself.
        let other = dir.path().join("Cellar/other/1.0/bin");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("oldbrainfix"), b"other").unwrap();
        assert_eq!(stable_brew_path(&other.join("oldbrainfix")), None);
        assert_eq!(
            stable_brew_path(Path::new("/usr/local/bin/oldbrainfix")),
            None
        );
    }

    #[test]
    fn exec_paths_with_specials_are_quoted() {
        assert_eq!(
            quote_exec("/opt/brew/bin/oldbrainfix"),
            "/opt/brew/bin/oldbrainfix"
        );
        assert_eq!(
            quote_exec("/home/me/My Apps/oldbrainfix.AppImage"),
            "\"/home/me/My Apps/oldbrainfix.AppImage\""
        );
        assert_eq!(quote_exec("/tmp/a\"b"), "\"/tmp/a\\\"b\"");
        assert_eq!(quote_exec("/tmp/$HOME%"), "\"/tmp/\\$HOME%%\"");
    }
}
