//! The system's own program for a file, started without a shell (#267):
//! `ShellExecuteW` on Windows, `open` on macOS and `xdg-open` on Linux
//! (the OpenURI portal inside the Flatpak), the path handed over as one
//! argument, never as words a shell reads. A photo, an Attachment and a
//! file link's file open through it.

// `ShellExecuteW` is FFI (#229): unsafe code is allowed in this module
// only, for that one call on Windows.
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;

/// The program that opens files on a system other than Windows, `macos`
/// or not, and its arguments for `path`: the path one argument of its
/// own, so no name is split into words. The paths handed in are
/// absolute, so none is taken for an option.
pub fn argv(path: &Path, macos: bool) -> (&'static str, Vec<OsString>) {
    let program = if macos { "open" } else { "xdg-open" };
    (program, vec![path.as_os_str().to_owned()])
}

/// The command that opens `path` with the system's program for it, on
/// systems that start one through a program; none on Windows, where
/// [`shell_execute`] does it.
pub fn command(path: &Path) -> Option<Command> {
    if cfg!(windows) {
        return None;
    }
    let (program, args) = argv(path, cfg!(target_os = "macos"));
    let mut command = Command::new(program);
    command.args(args);
    Some(command)
}

/// Opens `path` with the program Windows has for it, through
/// `ShellExecuteW` with the verb "open": no `cmd`, so no name is read as
/// a command. An error when Windows has no program for it.
#[cfg(windows)]
pub fn shell_execute(path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let file = wide(path.as_os_str());
    let verb = wide(std::ffi::OsStr::new("open"));
    // SAFETY: both strings end with a zero and live past the call; the
    // window, the parameters and the folder may be null.
    let done = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // A value above 32 is success; the others are error codes.
    let code = done as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(io::Error::other(format!("ShellExecuteW ended with {code}")))
    }
}

/// None on other systems: they start a program ([`command`]).
#[cfg(not(windows))]
pub fn shell_execute(_path: &Path) -> io::Result<()> {
    Err(io::Error::other("ShellExecuteW is Windows only"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hostile_name_stays_one_argument_and_no_shell_reads_it() {
        let hostile = Path::new("/home/a/x\" & calc.exe & \"; rm -rf ~ $(id) `id` |sh.pdf");
        for macos in [false, true] {
            let (program, args) = argv(hostile, macos);
            assert!(!["cmd", "cmd.exe", "sh", "bash", "start"].contains(&program));
            assert_eq!(args, [hostile.as_os_str().to_owned()]);
        }
        assert_eq!(argv(hostile, true).0, "open");
        assert_eq!(argv(hostile, false).0, "xdg-open");
    }

    #[cfg(not(windows))]
    #[test]
    fn the_command_hands_the_path_over_as_it_is() {
        let path = Path::new("/tmp/a b;c.txt");
        let command = command(path).unwrap();
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [path.as_os_str()]);
    }
}
