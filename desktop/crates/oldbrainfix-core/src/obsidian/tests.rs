use super::*;

fn paths(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn wiki_links_are_found_with_their_heading_and_label() {
    let text = "See [[Garden]], [[Garden#Roses|the roses]] and ![[rose.png]].";
    let found = wiki_links(text);
    assert_eq!(found.len(), 3);
    assert_eq!(found[0].target, "Garden");
    assert_eq!(found[0].shown(), "Garden");
    assert_eq!(text.get(found[0].start..found[0].end), Some("[[Garden]]"));
    assert_eq!(found[1].heading.as_deref(), Some("Roses"));
    assert_eq!(found[1].label.as_deref(), Some("the roses"));
    assert_eq!(found[1].shown(), "the roses");
    assert!(found[2].embed);
    assert_eq!(
        text.get(found[2].start..found[2].end),
        Some("![[rose.png]]")
    );
}

#[test]
fn a_heading_alone_and_a_block_reference_read_as_obsidian_shows_them() {
    let found = wiki_links("[[Garden#Roses]] [[#Soil]] [[Garden#^b1]]");
    assert_eq!(found[0].shown(), "Garden > Roses");
    assert_eq!(found[1].target, "");
    assert_eq!(found[1].shown(), "Soil");
    assert_eq!(found[2].heading, None);
}

#[test]
fn no_wiki_link_in_code_across_lines_or_empty() {
    let text = "`[[Code]]`\n```\n[[Fenced]]\n```\n[[Broken\nline]] [[]] [[a[b]]";
    assert!(wiki_links(text).is_empty());
}

#[test]
fn pictures_in_markdowns_own_way_are_found_without_their_titles() {
    let text = "![a rose](img/rose%20red.png \"Rose\") ![](<my pic.png>) [![x]] ![no](";
    let found = pictures(text);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].path, "img/rose%20red.png");
    assert_eq!(found[1].path, "my pic.png");
}

#[test]
fn a_note_is_found_by_its_path_or_its_name_in_any_case() {
    let all = paths(&["Garden.md", "sub/Roses.md", "sub/deep/Roses.md", "rose.png"]);
    assert_eq!(note_target("garden", &all), Some(0));
    assert_eq!(note_target("Garden.md", &all), Some(0));
    assert_eq!(note_target("Roses", &all), Some(1));
    assert_eq!(note_target("deep/Roses", &all), Some(2));
    assert_eq!(note_target("sub/deep/roses", &all), Some(2));
    assert_eq!(note_target("rose", &all), None);
    assert_eq!(note_target("Lost", &all), None);
    assert_eq!(note_target("", &all), None);
}

#[test]
fn a_file_is_found_beside_the_note_from_the_top_or_by_its_name() {
    let all = paths(&["a/rose.png", "b/rose.png", "pics/tulip.jpg"]);
    assert_eq!(file_target("rose.png", "b/Note.md", &all), Some(1));
    assert_eq!(file_target("rose.png", "Note.md", &all), Some(0));
    assert_eq!(file_target("pics/tulip.jpg", "a/Note.md", &all), Some(2));
    assert_eq!(file_target("TULIP.JPG", "Note.md", &all), Some(2));
    assert_eq!(file_target("other/tulip.jpg", "Note.md", &all), None);
}

#[test]
fn nothing_outside_the_folder_is_named() {
    let all = paths(&["rose.png", "sub/Note.md"]);
    assert_eq!(normal("a/../b/./c"), Some("b/c".into()));
    assert_eq!(normal("../x"), None);
    assert_eq!(normal("/etc/passwd"), None);
    assert_eq!(normal("C:/x"), None);
    assert_eq!(picture_target("../rose.png", "sub/Note.md", &all), Some(0));
    assert_eq!(picture_target("../../rose.png", "sub/Note.md", &all), None);
    assert_eq!(
        picture_target("https://example.org/rose.png", "Note.md", &all),
        None
    );
    assert_eq!(file_target("../../rose.png", "sub/Note.md", &all), None);
    assert_eq!(note_target("../Note", &all), None);
}

#[test]
fn a_space_written_as_percent_twenty_is_one() {
    assert_eq!(percent_decoded("rose%20red.png"), "rose red.png");
    assert_eq!(percent_decoded("100%"), "100%");
    assert_eq!(percent_decoded("%C3%A4"), "ä");
}
