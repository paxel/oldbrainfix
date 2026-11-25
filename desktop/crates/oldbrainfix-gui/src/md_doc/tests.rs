// A test may unwrap, expect, panic and index (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;

fn paragraph(block: &Block) -> &Inline {
    match block {
        Block::Paragraph(inline) => inline,
        other => panic!("no paragraph: {other:?}"),
    }
}

#[test]
fn six_heading_levels_and_paragraphs_with_their_looks() {
    let doc = read(
        "# One\n## Two\n### Three\n#### Four\n##### Five\n###### Six\n\n**bold** *it* ~~gone~~ `code` text",
    );
    let levels: Vec<usize> = doc
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Heading { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, [1, 2, 3, 4, 5, 6]);
    let text = paragraph(&doc.blocks[6]);
    assert_eq!(text.text, "bold it gone code text");
    let looks: Vec<Style> = text.runs.iter().map(|r| r.style).collect();
    assert!(looks[0].bold);
    assert!(looks[2].italic);
    assert!(looks[4].strike);
    assert!(looks[6].code);
    // The runs cover all of it, one after another.
    let mut at = 0;
    for run in &text.runs {
        assert_eq!(run.start, at);
        at = run.end;
    }
    assert_eq!(at, text.text.len());
}

#[test]
fn soft_line_breaks_read_as_spaces_and_hard_ones_as_breaks() {
    let doc = read("one\ntwo  \nthree");
    assert_eq!(paragraph(&doc.blocks[0]).text, "one two\nthree");
}

#[test]
fn nested_lists_and_task_lists() {
    let doc = read("- [ ] milk\n- [x] bread\n  1. rye\n  2. wheat\n- eggs");
    let Block::List { start, items } = &doc.blocks[0] else {
        panic!("no list");
    };
    assert_eq!(*start, None);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].task, Some(false));
    assert_eq!(items[1].task, Some(true));
    assert_eq!(items[2].task, None);
    assert_eq!(paragraph(&items[0].blocks[0]).text, "milk");
    let Block::List {
        start,
        items: inner,
    } = &items[1].blocks[1]
    else {
        panic!("no inner list");
    };
    assert_eq!(*start, Some(1));
    assert_eq!(paragraph(&inner[1].blocks[0]).text, "wheat");
}

#[test]
fn wiki_links_embeds_and_links_go_where_they_say() {
    let doc = read(
        "[[Garden]] [[Garden#Roses|roses]] [[#Soil]] [web](https://example.org) \
         [other](Other%20Note.md#Top) [file](report.pdf)\n\n![[rose.png]]",
    );
    let text = paragraph(&doc.blocks[0]);
    let links: Vec<&Target> = text.runs.iter().filter_map(|r| r.link.as_ref()).collect();
    assert_eq!(
        links,
        [
            &Target::Note {
                name: "Garden".into(),
                heading: None
            },
            &Target::Note {
                name: "Garden".into(),
                heading: Some("Roses".into())
            },
            &Target::Heading("Soil".into()),
            &Target::Web("https://example.org".into()),
            &Target::Note {
                name: "Other Note.md".into(),
                heading: Some("Top".into())
            },
            &Target::Nowhere,
        ]
    );
    assert!(text.text.starts_with("Garden roses Soil web other file"));
    assert_eq!(
        doc.blocks[1],
        Block::Picture(Picture {
            target: "rose.png".into(),
            wiki: true,
            alt: "rose.png".into()
        })
    );
}

#[test]
fn a_picture_inside_a_paragraph_follows_it() {
    let doc = read("Look: ![a rose](rose.png) here");
    assert_eq!(paragraph(&doc.blocks[0]).text, "Look:  here");
    let Block::Picture(picture) = &doc.blocks[1] else {
        panic!("no picture");
    };
    assert_eq!((picture.target.as_str(), picture.wiki), ("rose.png", false));
    assert_eq!(picture.alt, "a rose");
}

#[test]
fn highlights_in_pairs_outside_code() {
    let doc = read("a ==marked **bold**== and `==code==` and == alone");
    let text = paragraph(&doc.blocks[0]);
    assert_eq!(text.text, "a marked bold and ==code== and == alone");
    let lit: Vec<&str> = text
        .runs
        .iter()
        .filter(|r| r.style.highlight)
        .map(|r| &text.text[r.start..r.end])
        .collect();
    assert_eq!(lit, ["marked ", "bold"]);
}

#[test]
fn callouts_with_their_kind_title_and_body() {
    let doc = read(
        "> [!warning]- Mind the step\n> It is *high*.\n\n> [!note]\n> Plain.\n\n> Just a quote\n> on two lines",
    );
    let Block::Callout {
        kind,
        title,
        blocks,
    } = &doc.blocks[0]
    else {
        panic!("no callout: {:?}", doc.blocks[0]);
    };
    assert_eq!(kind, "warning");
    assert_eq!(title.text, "Mind the step");
    assert_eq!(paragraph(&blocks[0]).text, "It is high.");
    let Block::Callout { kind, title, .. } = &doc.blocks[1] else {
        panic!("no second callout");
    };
    assert_eq!((kind.as_str(), title.text.as_str()), ("note", "Note"));
    let Block::Quote(quoted) = &doc.blocks[2] else {
        panic!("no quote");
    };
    assert_eq!(paragraph(&quoted[0]).text, "Just a quote on two lines");
}

#[test]
fn front_matter_is_read_as_properties() {
    let doc = read(
        "---\ntitle: \"Garden plan\"\ntags:\n  - plants\n  - spring\naliases: [Plan, 'Beds']\n---\n# Hello",
    );
    assert_eq!(
        doc.properties,
        [
            ("title".to_string(), "Garden plan".to_string()),
            ("tags".to_string(), "plants, spring".to_string()),
            ("aliases".to_string(), "Plan, Beds".to_string()),
        ]
    );
    assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
}

#[test]
fn footnotes_are_numbered_by_their_first_reference() {
    let doc = read("First[^b] then[^a] and[^b].\n\n[^a]: Alpha.\n[^b]: Beta.\n[^c]: Never named.");
    let text = paragraph(&doc.blocks[0]);
    assert_eq!(text.text, "First[1] then[2] and[1].");
    let numbers: Vec<(&str, usize)> = doc
        .footnotes
        .iter()
        .map(|f| (f.label.as_str(), f.number))
        .collect();
    assert_eq!(numbers, [("b", 1), ("a", 2), ("c", 3)]);
    assert_eq!(paragraph(&doc.footnotes[0].blocks[0]).text, "Beta.");
}

#[test]
fn tables_and_code_keep_a_source_the_app_draws() {
    let doc = read(
        "| Name | [[Garden\\|Beds]] |\n|:-----|-----:|\n| rose | 3 |\n\n```rust\nfn main() {}\n```\n\n<div>raw</div>",
    );
    assert_eq!(
        doc.blocks[0],
        Block::Table("| Name | Beds |\n| :--- | ---: |\n| rose | 3 |".into())
    );
    assert_eq!(
        doc.blocks[1],
        Block::Code("```rust\nfn main() {}\n```".into())
    );
    assert_eq!(
        doc.blocks[2],
        Block::Code("```html\n<div>raw</div>\n```".into())
    );
    assert_eq!(fenced("", "a ``` b"), "````\na ``` b\n````");
}

#[test]
fn slugs_match_as_github_names_headings() {
    assert_eq!(slug("Hello, World!"), "hello-world");
    assert_eq!(slug("  Über  uns_2 "), "über-uns-2");
}
