//! "Open with" for Markdown files on Windows (obf-1056): the app lists
//! itself among the programs offered for `.md` and `.markdown` files,
//! for this user only, on every start, so a moved program is found
//! again. It never becomes their default: only the `OpenWithList` of
//! each ending and its own entry under `Applications` are written, never
//! the ending's own program. The Windows release is a zip without an
//! installer, so the app writes this itself, through `reg.exe` as its
//! login start does. On Linux the launcher's MimeType does it, on macOS
//! the bundle's `CFBundleDocumentTypes`.
//!
//! Not called at start yet: on Windows a second start cannot hand its
//! files to the running app ([`crate::instance`] needs Unix sockets), so
//! every file opened with it would start a whole second app, its alarms
//! and syncs too.

use std::path::Path;

/// The Markdown endings offered.
pub const ENDINGS: [&str; 2] = [".md", ".markdown"];

/// The name of the program file `exe`, by Windows' separators too.
fn program_name(exe: &Path) -> String {
    let path = exe.to_string_lossy();
    match path.rsplit(['\\', '/']).next() {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "oldbrainfix.exe".to_string(),
    }
}

/// The program's own entry, which says how it opens a file.
fn application_key(exe: &Path) -> String {
    format!(
        "HKCU\\Software\\Classes\\Applications\\{}",
        program_name(exe)
    )
}

/// The `reg.exe` calls, each its arguments, that offer `exe` for
/// Markdown files: its command with the file as one argument, its name,
/// the endings it supports, and its place in each ending's Open with
/// list.
pub fn reg_calls(exe: &Path) -> Vec<Vec<String>> {
    let app = application_key(exe);
    let name = program_name(exe);
    let s = |text: &str| text.to_string();
    let mut calls = vec![
        vec![
            s("add"),
            format!("{app}\\shell\\open\\command"),
            s("/ve"),
            s("/t"),
            s("REG_SZ"),
            s("/d"),
            format!("\"{}\" \"%1\"", exe.to_string_lossy()),
            s("/f"),
        ],
        vec![
            s("add"),
            app.clone(),
            s("/v"),
            s("FriendlyAppName"),
            s("/t"),
            s("REG_SZ"),
            s("/d"),
            s(oldbrainfix_core::APP_NAME),
            s("/f"),
        ],
    ];
    for ending in ENDINGS {
        calls.push(vec![
            s("add"),
            format!("{app}\\SupportedTypes"),
            s("/v"),
            s(ending),
            s("/t"),
            s("REG_SZ"),
            s("/d"),
            String::new(),
            s("/f"),
        ]);
        calls.push(vec![
            s("add"),
            format!("HKCU\\Software\\Classes\\{ending}\\OpenWithList\\{name}"),
            s("/f"),
        ]);
    }
    calls
}

/// Offers the running program for Markdown files, on Windows only; call
/// from a thread of its own. Best effort: a failure is only logged.
pub fn register() {
    if !cfg!(windows) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    for args in reg_calls(&exe) {
        let mut command = std::process::Command::new("reg");
        command.args(&args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window flashing up.
            command.creation_flags(0x0800_0000);
        }
        match command.output() {
            Ok(out) if out.status.success() => {}
            Ok(out) => eprintln!(
                "oldbrainfix: Open with: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(e) => eprintln!("oldbrainfix: Open with: {e}"),
        }
    }
}

#[cfg(test)]
mod tests;
