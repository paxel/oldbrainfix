use super::*;

#[test]
fn the_site_links_follow_the_language_english_otherwise() {
    assert_eq!(
        documentation_url("en"),
        "https://paxel.github.io/oldbrainfix/en/"
    );
    assert_eq!(
        documentation_url("de"),
        "https://paxel.github.io/oldbrainfix/de/"
    );
    assert_eq!(
        documentation_url("fr"),
        "https://paxel.github.io/oldbrainfix/en/"
    );
    assert_eq!(
        guide_url("de"),
        "https://paxel.github.io/oldbrainfix/de/guide/desktop-app.html"
    );
    assert_eq!(
        guide_url("fr"),
        "https://paxel.github.io/oldbrainfix/en/guide/desktop-app.html"
    );
    assert_eq!(
        privacy_policy_url("de"),
        "https://paxel.github.io/oldbrainfix/de/reference/privacy-policy.html"
    );
    assert_eq!(
        privacy_policy_url("fr"),
        "https://paxel.github.io/oldbrainfix/en/reference/privacy-policy.html"
    );
}

#[test]
fn every_list_reads_into_named_entries() {
    let all = licences();
    assert!(all.len() > 100);
    assert!(all.iter().all(|l| !l.names.is_empty()
        && l.names.iter().all(|n| !n.trim().is_empty())
        && !l.text.trim().is_empty()));
    let of = |name: &str| {
        all.iter()
            .find(|l| l.names.contains(&name))
            .map(|l| l.text)
            .unwrap_or_default()
    };
    // A crate by its name, whatever its version.
    let of_crate = |name: &str| {
        all.iter()
            .find(|l| {
                l.names.iter().any(|n| {
                    n.strip_prefix(name)
                        .and_then(|v| v.strip_prefix(' '))
                        .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
                })
            })
            .map(|l| l.text)
            .unwrap_or_default()
    };
    assert!(of("PP-OCRv6 Tiny models").contains("PaddlePaddle Authors"));
    assert!(of("PP-OCRv6 Tiny models").contains("Apache License"));
    assert!(of("OpenAI Whisper base model").contains("Copyright (c) 2022 OpenAI"));
    assert!(of("whisper.cpp").contains("The ggml authors"));
    assert!(of("Material Icons").contains("Attribution 4.0 International"));
    assert!(of("SQLite").contains("blessing"));
    assert!(of("Ubuntu font").contains("UBUNTU FONT LICENCE"));
    assert!(of("Noto Sans Symbols 2 font").contains("SIL Open Font License"));
    assert!(of("Noto Sans Math font").contains("SIL Open Font License"));
    assert!(of_crate("rusto-rs").contains("RustO Contributors"));
    assert!(of_crate("egui").contains("Emil Ernerfeldt"));
}

#[test]
fn an_entry_takes_its_names_and_text() {
    let list = "a 1.0\nb 2.0\n\nThe text.\n\nMore.\n================================================================================\n";
    assert_eq!(
        parse(list),
        vec![Licence {
            names: vec!["a 1.0", "b 2.0"],
            text: "The text.\n\nMore.",
        }]
    );
}
