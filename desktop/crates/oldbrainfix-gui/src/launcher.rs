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
         Comment=Capture a thought in seconds. Get nagged until it is sorted or done.\n\
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
mod tests;
