use super::*;

#[test]
fn every_word_typed_is_in_the_label_or_the_detail_line_in_any_case() {
    let words = ["Start at login", "Old Brain Fix starts when you log in"];
    assert!(matches(&words, &query_of("  LOGIN ")));
    assert!(matches(&words, &query_of("start brain")));
    assert!(!matches(&words, &query_of("login quiet")));
    assert!(matches(&words, ""));
}

#[test]
fn a_matching_heading_shows_its_section_whole() {
    let shown = rows("alarms", Some("Alarms"), [&["Alarm sound"], &["Default"]]);
    assert_eq!(shown, [true, true]);
    let shown = rows("sound", Some("Alarms"), [&["Alarm sound"], &["Default"]]);
    assert_eq!(shown, [true, false]);
    let shown = rows("", None, [&["Send feedback"]]);
    assert_eq!(shown, [true]);
    let shown = rows("units", None, [&["Send feedback"]]);
    assert_eq!(shown, [false]);
}
