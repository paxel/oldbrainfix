use super::*;
use std::collections::HashMap;
use std::path::PathBuf;

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let vars: HashMap<String, String> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| vars.get(name).cloned()
}

fn method(vars: &[(&str, &str)], exe: Option<&str>, deb: bool, os: &str) -> String {
    let exe = exe.map(PathBuf::from);
    install_method(&env(vars), exe.as_deref(), deb, os)
}

#[test]
fn the_install_method_comes_from_the_packaging_and_says_unknown_when_unsure() {
    let id = [("FLATPAK_ID", "io.github.paxel.oldbrainfix")];
    assert_eq!(
        method(&id, Some("/app/bin/oldbrainfix"), false, "linux"),
        "Flatpak"
    );
    let image = [("APPIMAGE", "/home/ann/Old.AppImage")];
    assert_eq!(
        method(
            &image,
            Some("/tmp/.mount/usr/bin/oldbrainfix"),
            false,
            "linux"
        ),
        "AppImage"
    );
    for exe in [
        "/home/linuxbrew/.linuxbrew/Cellar/oldbrainfix/0.4.0/bin/oldbrainfix",
        "/opt/homebrew/Caskroom/oldbrainfix/0.4.0/Old Brain Fix.app/Contents/MacOS/oldbrainfix",
    ] {
        assert_eq!(method(&[], Some(exe), false, "linux"), "Homebrew", "{exe}");
    }
    let scoop = r"C:\Users\Ann\scoop\apps\oldbrainfix\current\oldbrainfix.exe";
    assert_eq!(method(&[], Some(scoop), false, "windows"), "Scoop");
    assert_eq!(
        method(&[], Some("/usr/bin/oldbrainfix"), true, "linux"),
        ".deb"
    );
    assert_eq!(
        method(&[], Some("/usr/bin/oldbrainfix"), false, "linux"),
        "binary"
    );
    assert_eq!(
        method(&[], Some("/home/ann/bin/oldbrainfix"), false, "linux"),
        "binary"
    );
    let app = "/Applications/Old Brain Fix.app/Contents/MacOS/oldbrainfix";
    assert_eq!(method(&[], Some(app), false, "macos"), "unknown");
    assert_eq!(method(&[], None, false, "linux"), "unknown");
    assert_eq!(
        method(&[("FLATPAK_ID", "")], None, false, "linux"),
        "unknown"
    );
}

#[test]
fn the_system_line_names_the_distribution_and_the_processor() {
    let release = "NAME=\"Fedora Linux\"\nPRETTY_NAME=\"Fedora Linux 44 (KDE Plasma)\"\n";
    assert_eq!(
        system_line(Some(release), "linux", "x86_64"),
        "Fedora Linux 44 (KDE Plasma), x86_64"
    );
    assert_eq!(
        system_line(Some("NAME=Debian\n"), "linux", "aarch64"),
        "Debian, aarch64"
    );
    assert_eq!(system_line(None, "windows", "x86_64"), "windows, x86_64");
}

#[test]
fn the_body_is_the_words_then_each_fact_ticked_as_the_phone_writes_it() {
    assert_eq!(
        feedback_body(
            " It counts wrong. ",
            Some("0.4.0"),
            Some("Debian, x86_64"),
            Some("Flatpak")
        ),
        "It counts wrong.\n\nVersion: 0.4.0\nSystem: Debian, x86_64\nInstalled from: Flatpak"
    );
    assert_eq!(
        feedback_body("Hi", None, Some("Debian, x86_64"), None),
        "Hi\n\nSystem: Debian, x86_64"
    );
    assert_eq!(feedback_body("Hi", None, None, None), "Hi");
}

#[test]
fn the_mail_link_goes_to_the_developer_with_the_body_as_it_is() {
    assert_eq!(
        mail_url("A & B\n#1"),
        "mailto:taum@tuta.io?subject=Old%20Brain%20Fix%20feedback&body=A%20%26%20B%0A%231"
    );
}
