use super::*;
use std::collections::BTreeMap;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// A group as ARB keys: its slug, its heading's key and each icon's
/// name and meaning keys.
type PhoneGroup = (String, String, Vec<(String, String)>);

fn repo(path: &str) -> String {
    format!("{}/../../../{path}", env!("CARGO_MANIFEST_DIR"))
}

/// The phone's `helpIconGroups`, as ARB keys: each group's slug and
/// heading key with its icons' name and meaning keys.
fn phone_groups() -> std::result::Result<Vec<PhoneGroup>, Box<dyn std::error::Error>> {
    let dart = std::fs::read_to_string(repo("lib/src/help_notes.dart"))?;
    // `String _kind(AppLocalizations t) => t.helpIconKind;`
    let mut keys = BTreeMap::new();
    for line in dart.lines() {
        if let Some(rest) = line.strip_prefix("String _")
            && let Some((name, key)) = rest.split_once("(AppLocalizations t) => t.")
        {
            keys.insert(name.to_string(), key.trim_end_matches(';').to_string());
        }
    }
    let key = |name: &str| -> std::result::Result<String, Box<dyn std::error::Error>> {
        Ok(keys
            .get(name.trim().trim_start_matches('_'))
            .ok_or(format!("no key for {name}"))?
            .clone())
    };
    let list = dart
        .split("const helpIconGroups = [")
        .nth(1)
        .ok_or("the groups")?;
    let list = list.split("\n];").next().ok_or("their end")?;
    let mut groups = Vec::new();
    for line in list.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("HelpIconGroup('") {
            let (slug, rest) = rest.split_once("', ").ok_or("a slug")?;
            let heading = rest.split(',').next().ok_or("a heading")?;
            groups.push((slug.to_string(), key(heading)?, Vec::new()));
        } else if let Some(rest) = line
            .strip_prefix("HelpIcon(")
            .or_else(|| line.strip_prefix("HelpIcon.drawn("))
        {
            let parts: Vec<&str> = rest.trim_end_matches("),").split(", ").collect();
            let [_, name, means] = parts[..] else {
                return Err(format!("an icon: {line}").into());
            };
            let group = groups.last_mut().ok_or("an icon before a group")?;
            group.2.push((key(name)?, key(means)?));
        }
    }
    Ok(groups)
}

#[test]
fn the_groups_and_their_words_are_the_phones() -> TestResult {
    let phone = phone_groups()?;
    assert_eq!(phone.len(), GROUPS.len());
    for lang in ["en", "de"] {
        let arb: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(repo(
            &format!("lib/l10n/app_{lang}.arb"),
        ))?)?;
        let word = |key: &str| arb[key].as_str().unwrap_or_default().to_string();
        let t = L10n::new(lang);
        for (group, (slug, heading, icons)) in GROUPS.iter().zip(&phone) {
            assert_eq!(group.slug, slug);
            assert_eq!((group.heading)(&t), word(heading));
            let ours: Vec<_> = group
                .icons
                .iter()
                .map(|(n, m)| (n(&t).to_string(), m(&t).to_string()))
                .collect();
            let theirs: Vec<_> = icons.iter().map(|(n, m)| (word(n), word(m))).collect();
            assert_eq!(ours, theirs, "{slug} in {lang}");
        }
    }
    Ok(())
}

#[test]
fn every_icon_named_in_the_text_has_its_picture() -> TestResult {
    for lang in ["en", "de"] {
        let t = L10n::new(lang);
        let text = icons_text(&t);
        for (i, group) in GROUPS.iter().enumerate() {
            let at = text
                .find(&format!("\n\n{}\n", (group.heading)(&t)))
                .ok_or("the heading")?;
            for (name, means) in group.icons {
                assert!(text[at..].contains(&format!("\n{}: {}", name(&t), means(&t))));
            }
            let picture = std::fs::read(repo(&format!(
                "assets/help/icons-{}-{lang}.png",
                group.slug
            )))?;
            assert_eq!(pictures(&t)[i], &picture[..], "{} {lang}", group.slug);
        }
    }
    Ok(())
}

/// As the phone's test: every icon that leads to an Advanced feature
/// says so (#197); File into… does not, as Simple files into Topics too.
#[test]
fn every_icon_of_an_advanced_feature_says_so() {
    for (lang, marker) in [("en", "in Advanced"), ("de", "in Erweitert")] {
        let t = L10n::new(lang);
        let advanced = [
            t.help_icon_kind_means(),
            t.help_icon_place_means(),
            t.help_icon_add_means(),
            t.help_icon_new_session_means(),
            t.help_icon_alarm_means(),
            t.help_icon_more_means(),
        ];
        let named = [
            "Tally",
            "Session",
            "Frog",
            "Strichliste",
            "Einheit",
            "Frosch",
        ];
        for group in &GROUPS {
            for (name, means) in group.icons {
                let line = format!("{}: {}", name(&t), means(&t));
                let expected =
                    advanced.contains(&means(&t)) || named.iter().any(|word| line.contains(word));
                assert_eq!(line.contains(marker), expected, "{line}");
            }
        }
    }
}

#[test]
fn the_quick_start_has_as_many_lines_in_each_language() {
    assert_eq!(
        quick_start_lines(&L10n::new("en")).len(),
        quick_start_lines(&L10n::new("de")).len()
    );
}

/// As the phone's test: the Description is "On the desktop" and the
/// desktop's way for each Line, and neither teaches Advanced (#260).
#[test]
fn the_quick_start_description_has_a_desktop_way_per_line_and_nothing_advanced() {
    // As the phone's test: each place but the Checklist's own is a
    // link that opens (#261).
    let t = L10n::new("en");
    let links: Vec<String> = oldbrainfix_core::markdown::parse(&quick_start_description(&t))
        .into_iter()
        .flat_map(|block| block.spans)
        .filter(|span| span.kind == oldbrainfix_core::markdown::SpanKind::Link)
        .map(|span| span.url)
        .collect();
    assert_eq!(
        links,
        [
            "oldbrainfix://agenda",
            "oldbrainfix://unsorted",
            "oldbrainfix://routines",
            "oldbrainfix://routines",
            "oldbrainfix://agenda",
            "oldbrainfix://settings/backups",
            "oldbrainfix://settings/sync",
        ]
    );
    assert_eq!(quick_start_places(&t)[4], "Here, in this Checklist.");
    assert_eq!(
        icons_text(&t).lines().last(),
        Some("[Everything else is in the user guide.](oldbrainfix://guide)")
    );
    let advanced = [
        "tally",
        "session",
        "frog",
        "topic",
        "format bar",
        "role",
        "strichliste",
        "einheit",
        "frosch",
        "thema",
        "formatleiste",
        "rolle",
    ];
    for lang in ["en", "de"] {
        let t = L10n::new(lang);
        let description = quick_start_description(&t);
        let lines: Vec<&str> = description.split('\n').collect();
        let count = quick_start_lines(&t).len();
        // A link to each Line's place first (#261), then the
        // desktop's way for each Line.
        assert_eq!(lines[0], t.help_quick_start_places_title());
        let places: Vec<String> = quick_start_places(&t)
            .iter()
            .enumerate()
            .map(|(i, place)| format!("{}. {place}", i + 1))
            .collect();
        assert_eq!(lines[1..=count], places, "{lang}");
        assert_eq!(lines[count + 1], "");
        assert_eq!(lines[count + 2], t.help_quick_start_desktop_title());
        assert_eq!(lines.len() - count - 3, count, "{lang}");
        for line in lines
            .iter()
            .copied()
            .chain(quick_start_lines(&t).iter().map(String::as_str))
        {
            let lower = line.to_lowercase();
            assert!(!advanced.iter().any(|w| lower.contains(w)), "{line}");
        }
    }
}
