use super::*;

#[test]
fn an_address_without_a_scheme_gets_the_one_it_reads_as() {
    assert_eq!(address(" ann@example.org "), "mailto:ann@example.org");
    assert_eq!(address("example.org/a@b"), "https://example.org/a@b");
    assert_eq!(address("example.org"), "https://example.org");
    assert_eq!(address("https://example.org"), "https://example.org");
    assert_eq!(address("  "), "");
}

#[test]
fn the_buttons_work_on_the_word_or_the_line_at_the_cursor() {
    let bold = change(Format::Bold, "buy milk", (5, 5));
    assert_eq!(bold.map(|c| c.text).as_deref(), Some("buy **milk**"));
    let list = change(Format::Bullets, "milk", (0, 0));
    assert_eq!(list.map(|c| c.text).as_deref(), Some("- milk"));
    assert_eq!(change(Format::Link, "milk", (0, 0)), None);
    assert_eq!(change(Format::Table, "milk", (0, 0)), None);
    let marked = change(Format::Mark, "Pack\nPassport", (7, 7));
    assert_eq!(
        marked,
        Some(Change {
            text: "Pack\nPa☐ssport".into(),
            start: 10,
            end: 10,
        })
    );
}

#[test]
fn a_button_shows_the_format_at_the_cursor() {
    let text = "## Shop **milk**";
    let at = At::of(text, (11, 11), true);
    assert!(at.holds(Format::Heading2));
    assert!(at.holds(Format::Bold));
    assert!(!at.holds(Format::Italic));
    assert_eq!(At::of(text, (11, 11), false), At::default());
    let table = At::of("| a | b |\n| --- | --- |", (2, 2), true);
    assert!(table.holds(Format::Table));
    assert!(!at.holds(Format::Table));
    let link = At::of("[a](https://x.org)", (1, 1), true);
    assert_eq!(link.url.as_deref(), Some("https://x.org"));
}
