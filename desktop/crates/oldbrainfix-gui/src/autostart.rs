//! Starting at login, into the tray, on by default (spec, "Desktop"):
//! an XDG autostart entry on Linux, a LaunchAgent on macOS, a value under
//! the user's Run key on Windows. The entries are made by plain
//! functions, so every platform's is tested here.

use std::path::{Path, PathBuf};

use oldbrainfix_core::{APP_ID, APP_NAME};

/// The argument the login start runs with: into the tray, no window.
pub const TRAY_ARG: &str = "--tray";

/// The Run key value's name on Windows.
pub const RUN_VALUE: &str = APP_NAME;

/// The user's Run key on Windows.
pub const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

/// `path` quoted for a desktop entry's `Exec` (Desktop Entry spec,
/// "The Exec key").
fn exec_quoted(path: &Path) -> String {
    let text = path.to_string_lossy();
    let mut out = String::from("\"");
    for c in text.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    // A literal percent is doubled in Exec.
    out.replace('%', "%%")
}

/// The XDG autostart entry that starts `exe` into the tray.
pub fn desktop_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_NAME}\n\
         Exec={} {TRAY_ARG}\n\
         Icon={APP_ID}\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        exec_quoted(exe)
    )
}

fn xml_escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The macOS LaunchAgent that starts `exe` into the tray at login.
pub fn launch_agent(exe: &Path) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{APP_ID}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         \t\t<string>{}</string>\n\
         \t\t<string>{TRAY_ARG}</string>\n\
         \t</array>\n\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         </dict>\n\
         </plist>\n",
        xml_escaped(&exe.to_string_lossy())
    )
}

/// The `reg.exe` arguments that set (`on`) or remove the Run key value
/// starting `exe` into the tray.
pub fn run_key_args(exe: &Path, on: bool) -> Vec<String> {
    let mut args = vec![
        if on { "add" } else { "delete" }.to_string(),
        RUN_KEY.to_string(),
        "/v".to_string(),
        RUN_VALUE.to_string(),
    ];
    if on {
        args.extend([
            "/t".to_string(),
            "REG_SZ".to_string(),
            "/d".to_string(),
            format!("\"{}\" {TRAY_ARG}", exe.to_string_lossy()),
        ]);
    }
    args.push("/f".to_string());
    args
}

/// Where this user's login start lives, and what it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Autostart {
    /// The program started at login.
    pub exe: PathBuf,
    /// The entry's file: the `.desktop` file or the LaunchAgent; unused on
    /// Windows, where the entry is a registry value.
    pub file: PathBuf,
}

impl Autostart {
    /// This user's login start of the running program; `None` when its
    /// place cannot be told. An AppImage starts its image, not the
    /// mounted copy.
    pub fn for_this_user() -> Option<Autostart> {
        let exe = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())?;
        let dirs = directories::BaseDirs::new()?;
        let file = if cfg!(target_os = "macos") {
            dirs.home_dir()
                .join("Library/LaunchAgents")
                .join(format!("{APP_ID}.plist"))
        } else {
            dirs.config_dir()
                .join("autostart")
                .join(format!("{APP_ID}.desktop"))
        };
        Some(Autostart { exe, file })
    }

    /// Sets the login start up, or takes it away. Writing the same entry
    /// again changes nothing.
    pub fn set(&self, on: bool) -> Result<(), String> {
        if cfg!(windows) {
            return self.set_run_key(on);
        }
        if !on {
            return match std::fs::remove_file(&self.file) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("{}: {e}", self.file.display()))
                }
                _ => Ok(()),
            };
        }
        let content = if cfg!(target_os = "macos") {
            launch_agent(&self.exe)
        } else {
            desktop_entry(&self.exe)
        };
        if std::fs::read_to_string(&self.file).is_ok_and(|held| held == content) {
            return Ok(());
        }
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&self.file, content).map_err(|e| format!("{}: {e}", self.file.display()))
    }

    /// Whether the login start is set up.
    pub fn is_set(&self) -> bool {
        if cfg!(windows) {
            return std::process::Command::new("reg")
                .args(["query", RUN_KEY, "/v", RUN_VALUE])
                .output()
                .is_ok_and(|o| o.status.success());
        }
        self.file.is_file()
    }

    fn set_run_key(&self, on: bool) -> Result<(), String> {
        let mut command = std::process::Command::new("reg");
        command.args(run_key_args(&self.exe, on));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window flashing up.
            command.creation_flags(0x0800_0000);
        }
        let out = command.output().map_err(|e| format!("reg: {e}"))?;
        // Removing a value that is not there is fine.
        if out.status.success() || !on {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_entry_starts_the_program_into_the_tray() {
        let entry = desktop_entry(Path::new("/opt/old brain/oldbrainfix"));
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("\nExec=\"/opt/old brain/oldbrainfix\" --tray\n"));
        assert!(entry.contains("\nName=Old Brain Fix\n"));
        assert!(entry.contains("\nIcon=io.github.paxel.oldbrainfix\n"));
        assert_eq!(exec_quoted(Path::new("/a\"b$c%")), "\"/a\\\"b\\$c%%\"");
    }

    #[test]
    fn the_launch_agent_and_the_run_key_start_it_too() {
        let plist = launch_agent(Path::new("/Applications/Old & Brain.app/x"));
        assert!(plist.contains("<string>/Applications/Old &amp; Brain.app/x</string>"));
        assert!(plist.contains("<string>--tray</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        let add = run_key_args(Path::new(r"C:\Old Brain Fix\oldbrainfix.exe"), true);
        assert_eq!(
            add,
            [
                "add",
                RUN_KEY,
                "/v",
                "Old Brain Fix",
                "/t",
                "REG_SZ",
                "/d",
                "\"C:\\Old Brain Fix\\oldbrainfix.exe\" --tray",
                "/f"
            ]
        );
        let delete = run_key_args(Path::new("x"), false);
        assert_eq!(delete, ["delete", RUN_KEY, "/v", "Old Brain Fix", "/f"]);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn it_is_set_up_and_taken_away_again() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let start = Autostart {
            exe: PathBuf::from("/usr/bin/oldbrainfix"),
            file: dir.path().join("autostart").join("x.desktop"),
        };
        assert!(!start.is_set());
        start.set(true)?;
        start.set(true)?;
        assert!(start.is_set());
        let held = std::fs::read_to_string(&start.file).map_err(|e| e.to_string())?;
        assert_eq!(held, desktop_entry(&start.exe));
        start.set(false)?;
        start.set(false)?;
        assert!(!start.is_set());
        Ok(())
    }
}
