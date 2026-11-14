//! Starting at login, into the tray, on by default (spec, "Desktop"):
//! an XDG autostart entry on Linux, a LaunchAgent on macOS, a value under
//! the user's Run key on Windows. Inside the Flatpak, which cannot write
//! the user's autostart folder, the Background portal sets it up instead
//! (#45). The entries are made by plain functions, so every platform's is
//! tested here.

use std::path::{Path, PathBuf};

use oldbrainfix_core::{APP_ID, APP_NAME};

use crate::ICON_NAME;

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
         Icon={ICON_NAME}\n\
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
    /// Windows, where the entry is a registry value, and in the Flatpak.
    pub file: PathBuf,
    /// Inside the Flatpak: the Background portal writes the entry.
    pub through_portal: bool,
}

impl Autostart {
    /// This user's login start of the running program; `None` when its
    /// place cannot be told. An AppImage starts its image, not the
    /// mounted copy.
    pub fn for_this_user() -> Option<Autostart> {
        let exe = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())?;
        if std::env::var_os("FLATPAK_ID").is_some() {
            return Some(Autostart {
                exe,
                file: PathBuf::new(),
                through_portal: true,
            });
        }
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
        Some(Autostart {
            exe,
            file,
            through_portal: false,
        })
    }

    /// Sets the login start up, or takes it away. Writing the same entry
    /// again changes nothing. Through the portal the answer comes later
    /// (the desktop may ask the user first); a refusal is only logged,
    /// the switch keeps what the user chose.
    pub fn set(&self, on: bool) -> Result<(), String> {
        if self.through_portal {
            portal::request(self.portal_command(), on);
            return Ok(());
        }
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

    /// What the portal's entry runs inside the sandbox: the program by
    /// its name in `/app/bin`, into the tray.
    pub fn portal_command(&self) -> Vec<String> {
        let name = self.exe.file_name().map_or_else(
            || "oldbrainfix".into(),
            |n| n.to_string_lossy().into_owned(),
        );
        vec![name, TRAY_ARG.to_string()]
    }

    /// Whether the login start is set up. The portal's entry lies outside
    /// the sandbox, so through the portal this is never known: the stored
    /// setting counts.
    pub fn is_set(&self) -> bool {
        if self.through_portal {
            return false;
        }
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

#[cfg(target_os = "linux")]
mod portal {
    use ashpd::desktop::background::Background;

    /// Asks the Background portal for the login start (`on`) or to drop
    /// it, on a thread of its own: the desktop may show a dialog first.
    pub fn request(command: Vec<String>, on: bool) {
        std::thread::spawn(move || {
            let asked = async_io::block_on(async {
                Background::request()
                    .auto_start(on)
                    .command(&command)
                    .dbus_activatable(false)
                    .send()
                    .await?
                    .response()
            });
            match asked {
                Ok(answer) if answer.auto_start() != on => {
                    eprintln!("oldbrainfix: the desktop kept the login start as it was");
                }
                Ok(_) => {}
                Err(e) => eprintln!("oldbrainfix: no login start from the desktop: {e}"),
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
mod portal {
    /// There is no Flatpak, and so no portal, off Linux.
    pub fn request(_command: Vec<String>, _on: bool) {}
}

#[cfg(test)]
mod tests;
