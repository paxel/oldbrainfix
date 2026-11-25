// A test may unwrap, expect, panic and index (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;

#[test]
fn markdown_files_are_offered_never_made_the_programs_default() {
    let calls = reg_calls(Path::new("C:\\Tools\\Old Brain Fix\\oldbrainfix.exe"));
    let app = "HKCU\\Software\\Classes\\Applications\\oldbrainfix.exe";
    assert_eq!(
        calls[0],
        [
            "add",
            &format!("{app}\\shell\\open\\command"),
            "/ve",
            "/t",
            "REG_SZ",
            "/d",
            "\"C:\\Tools\\Old Brain Fix\\oldbrainfix.exe\" \"%1\"",
            "/f",
        ]
    );
    let keys: Vec<&str> = calls.iter().map(|c| c[1].as_str()).collect();
    assert!(keys.contains(&"HKCU\\Software\\Classes\\.md\\OpenWithList\\oldbrainfix.exe"));
    assert!(keys.contains(&"HKCU\\Software\\Classes\\.markdown\\OpenWithList\\oldbrainfix.exe"));
    // And AXT files (obf-1146).
    assert!(keys.contains(&"HKCU\\Software\\Classes\\.axt\\OpenWithList\\oldbrainfix.exe"));
    // Nothing names the ending's own program, which picks the default.
    for call in &calls {
        let key = call[1].as_str();
        assert!(
            key.starts_with(app) || key.contains("\\OpenWithList\\"),
            "{key}"
        );
        assert!(!call.iter().any(|a| a == "/ve") || key.ends_with("\\shell\\open\\command"));
    }
}
