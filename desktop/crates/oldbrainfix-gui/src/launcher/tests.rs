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
    let shipped = include_str!("../../../../packaging/linux/io.github.paxel.oldbrainfix.desktop");
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
