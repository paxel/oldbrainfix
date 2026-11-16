use super::*;

#[test]
fn the_desktop_entry_starts_the_program_into_the_tray() {
    let entry = desktop_entry(Path::new("/opt/old brain/oldbrainfix"));
    assert!(entry.starts_with("[Desktop Entry]\n"));
    assert!(entry.contains("\nExec=\"/opt/old brain/oldbrainfix\" --tray\n"));
    assert!(entry.contains("\nName=Old Brain Fix\n"));
    assert!(entry.contains("\nIcon=oldbrainfix\n"));
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

#[test]
fn the_portal_starts_the_sandboxed_program_into_the_tray_and_writes_no_file() -> Result<(), String>
{
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let start = Autostart {
        exe: PathBuf::from("/app/bin/oldbrainfix"),
        file: dir.path().join("x.desktop"),
        through_portal: true,
    };
    assert_eq!(start.portal_command(), ["oldbrainfix", "--tray"]);
    assert!(!start.is_set());
    assert!(!start.file.exists());
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
#[test]
fn it_is_set_up_and_taken_away_again() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let start = Autostart {
        exe: PathBuf::from("/usr/bin/oldbrainfix"),
        file: dir.path().join("autostart").join("x.desktop"),
        through_portal: false,
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
