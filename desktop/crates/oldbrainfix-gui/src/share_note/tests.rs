use super::*;

#[test]
fn a_note_is_proposed_under_its_first_line() {
    assert_eq!(file_name("Milk, eggs & bread\nmore"), "Milk eggs bread.md");
    assert_eq!(file_name("../../etc/passwd"), "etc passwd.md");
    assert_eq!(file_name("**?**"), "note.md");
    assert_eq!(
        file_name("one two three four five six seven eight nine ten"),
        "one two three four five six seven eight.md"
    );
    assert_eq!(file_name(&"x".repeat(50)), format!("{}.md", "x".repeat(40)));
}

#[test]
fn the_markdown_links_its_files_by_name() {
    let files = [
        Beside {
            name: "photo-abc.jpg".into(),
            photo: true,
        },
        Beside {
            name: "recording-def.wav".into(),
            photo: false,
        },
    ];
    assert_eq!(
        markdown("**Milk**", "Milk", &files),
        "**Milk**\n\n![photo-abc.jpg](Milk/photo-abc.jpg)\n\n[recording-def.wav](Milk/recording-def.wav)\n"
    );
    assert_eq!(
        markdown("", "Milk (2) ä", &files[1..]),
        "[recording-def.wav](Milk%20%282%29%20ä/recording-def.wav)\n"
    );
    assert_eq!(short("0123456789abcdef"), "0123456789ab");
    assert_eq!(short("abc"), "abc");
}
