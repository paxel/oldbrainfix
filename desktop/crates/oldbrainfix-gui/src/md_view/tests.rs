// A test may unwrap, expect, panic and index (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;

#[test]
fn the_find_bar_finds_in_every_text_in_the_order_they_show() {
    let doc = crate::md_doc::read(
        "---\ntitle: Plan\n---\n# Garden\n> [!tip] Water\n> daily\n\n- roses\n\n| a |\n|---|\n| b |\n\nNote[^1]\n\n[^1]: Late.",
    );
    let found = pieces(&doc);
    let texts: Vec<(&str, &str)> = found.iter().map(|p| (p.text.as_str(), p.format)).collect();
    assert_eq!(
        texts,
        [
            ("title", text_format::PLAIN),
            ("Plan", text_format::PLAIN),
            ("Garden", text_format::PLAIN),
            ("Water", text_format::PLAIN),
            ("daily", text_format::PLAIN),
            ("roses", text_format::PLAIN),
            ("| a |\n| --- |\n| b |", text_format::MARKDOWN),
            ("Note[1]", text_format::PLAIN),
            ("Late.", text_format::PLAIN),
        ]
    );
}

#[test]
fn callouts_are_coloured_by_their_kind() {
    assert_eq!(
        callout_colour("warning"),
        Color32::from_rgb(0xEF, 0x6C, 0x00)
    );
    assert_eq!(callout_colour("note"), callout_colour("anything else"));
    assert_ne!(highlight_tint(false), theme::found_tint(false, false));
}
