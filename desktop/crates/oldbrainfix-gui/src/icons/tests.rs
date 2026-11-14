use super::*;

#[test]
fn every_area_icon_of_the_core_has_a_glyph() {
    for key in oldbrainfix_core::AREA_ICONS {
        assert!(area_icon(Some(key)).is_some(), "{key}");
    }
    assert_eq!(area_icon(Some("rocket")), None);
    assert_eq!(area_icon(None), None);
}

/// The faces of `family`'s chain, in order, by name.
fn chain<'a>(
    defs: &'a egui::FontDefinitions,
    family: &FontFamily,
) -> Vec<(&'a str, ttf_parser::Face<'a>)> {
    defs.families[family]
        .iter()
        .map(|name| {
            let data = &defs.font_data[name];
            let face = ttf_parser::Face::parse(&data.font, data.index).expect(name);
            (name.as_str(), face)
        })
        .collect()
}

/// Every file under `dir` whose name ends in `suffix`, recursively.
fn files(dir: &std::path::Path, suffix: &str) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files(&path, suffix));
        } else if path.to_string_lossy().ends_with(suffix) {
            found.push(path);
        }
    }
    found
}

/// Code points that draw nothing: spaces, joiners, variation
/// selectors and the tag characters of a box's id.
fn invisible(c: char) -> bool {
    c.is_whitespace()
        || c.is_control()
        || matches!(c, '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{206f}')
        || matches!(c, '\u{fe00}'..='\u{fe0f}' | '\u{feff}' | '\u{e0000}'..='\u{e007f}')
}

/// Every symbol the desktop draws as text: the non-ASCII characters
/// other than letters of the strings (the help Notes among them), the
/// gui's own code, the guide and the docs a Note may hold, the boxes,
/// the frog, and the arrows, quotes and dashes of everyday text.
fn symbols() -> std::collections::BTreeSet<char> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut sources = files(&root.join("lib/l10n"), ".arb");
    sources.extend(files(&root.join("desktop/l10n"), ".arb"));
    sources.extend(files(&root.join("docs/agents"), ".md"));
    sources.extend(files(
        &root.join("desktop/crates/oldbrainfix-gui/src"),
        ".rs",
    ));
    for site in std::fs::read_dir(root.join("site"))
        .expect("the site")
        .flatten()
    {
        let guide = site.path().join("guide");
        if guide.is_dir() {
            sources.extend(files(&guide, ".html"));
        }
    }
    assert!(sources.len() > 20, "{sources:?}");
    let mut all: std::collections::BTreeSet<char> = sources
        .iter()
        .flat_map(|path| {
            std::fs::read_to_string(path)
                .expect("a text")
                .chars()
                .collect::<Vec<_>>()
        })
        .filter(|&c| !c.is_ascii() && !c.is_alphabetic() && !invisible(c))
        .collect();
    all.extend(oldbrainfix_core::text_boxes::BoxState::ALL.map(|b| b.glyph()));
    all.extend(crate::frogs_view::FROG.chars());
    all.extend("→←↑↓⇄▸◂⋮×·…–—„“”‚‘’".chars());
    all
}

/// The code points of `range` that are characters.
fn range(range: std::ops::RangeInclusive<u32>) -> impl Iterator<Item = char> {
    range.filter_map(char::from_u32)
}

/// The symbols of `wanted` no font of `family`'s chain has a glyph for.
fn lacking(
    defs: &egui::FontDefinitions,
    family: &FontFamily,
    wanted: impl IntoIterator<Item = char>,
) -> String {
    let chain = chain(defs, family);
    wanted
        .into_iter()
        .filter(|&c| {
            !chain
                .iter()
                .any(|(_, face)| face.glyph_index(c).is_some_and(|g| g.0 != 0))
        })
        .collect()
}

#[test]
fn every_symbol_of_the_app_and_its_texts_has_a_glyph() {
    let defs = fonts();
    let symbols = symbols();
    // Arrows, math operators, box drawing and geometric shapes, whole.
    let blocks = || {
        range(0x2190..=0x21ff)
            .chain(range(0x2200..=0x22ff))
            .chain(range(0x2500..=0x257f))
            .chain(range(0x25a0..=0x25ff))
    };
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let wanted = symbols.iter().copied().chain(blocks());
        let missing = lacking(&defs, &family, wanted);
        assert!(missing.is_empty(), "{family:?} lacks {missing:?}");
    }
    // The check itself: egui's own fonts lack the arrow (#254).
    let before = lacking(
        &egui::FontDefinitions::default(),
        &FontFamily::Proportional,
        symbols,
    );
    assert!(before.contains('→'), "{before:?}");
}
