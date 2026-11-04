//! The help Notes on the desktop (#180, ADR 0042): what the phone's
//! `help_notes.dart` hands the core, in the same words and with the same
//! pictures, so either app makes the same Notes. The pictures and the
//! What's new are the phone's assets, bundled at build time.

use oldbrainfix_core::{HelpNotes, HelpTexts, Result, Store, parse_whats_new};

use crate::l10n::L10n;

/// The What's new of every release, as `tool/whats_new.py --bundle`
/// writes it.
pub static WHATS_NEW: &str = include_str!("../../../../assets/help/whats_new.json");

/// One icon of "What the icons mean": its name and its meaning.
type Named = (fn(&L10n) -> &'static str, fn(&L10n) -> &'static str);

/// A group of icons: its slug, as the picture is named, its heading and
/// its icons, as the phone's `helpIconGroups`.
pub struct Group {
    pub slug: &'static str,
    pub heading: fn(&L10n) -> &'static str,
    pub icons: &'static [Named],
    /// The picture, English and German.
    pub pictures: [&'static [u8]; 2],
}

/// The groups, in the phone's order, with its pictures.
pub static GROUPS: [Group; 3] = [
    Group {
        slug: "badges",
        heading: L10n::help_icons_badges,
        icons: &[
            (L10n::help_icon_kind, L10n::help_icon_kind_means),
            (L10n::help_icon_importance, L10n::help_icon_importance_means),
            (L10n::help_icon_place, L10n::help_icon_place_means),
            (L10n::help_icon_captured, L10n::help_icon_captured_means),
            (L10n::help_icon_reminder, L10n::help_icon_reminder_means),
            (L10n::help_icon_routine, L10n::help_icon_routine_means),
            (L10n::help_icon_add, L10n::help_icon_add_means),
        ],
        pictures: [
            include_bytes!("../../../../assets/help/icons-badges-en.png"),
            include_bytes!("../../../../assets/help/icons-badges-de.png"),
        ],
    },
    Group {
        slug: "agenda",
        heading: L10n::help_icons_agenda,
        icons: &[
            (L10n::help_icon_browse, L10n::help_icon_browse_means),
            (
                L10n::help_icon_all_routines,
                L10n::help_icon_all_routines_means,
            ),
            (
                L10n::help_icon_new_routine,
                L10n::help_icon_new_routine_means,
            ),
            (
                L10n::help_icon_new_session,
                L10n::help_icon_new_session_means,
            ),
            (L10n::help_icon_unsorted, L10n::help_icon_unsorted_means),
        ],
        pictures: [
            include_bytes!("../../../../assets/help/icons-agenda-en.png"),
            include_bytes!("../../../../assets/help/icons-agenda-de.png"),
        ],
    },
    Group {
        slug: "card",
        heading: L10n::help_icons_card,
        icons: &[
            (L10n::help_icon_file_into, L10n::help_icon_file_into_means),
            (L10n::help_icon_alarm, L10n::help_icon_alarm_means),
            (L10n::help_icon_merge, L10n::help_icon_merge_means),
            (L10n::help_icon_more, L10n::help_icon_more_means),
        ],
        pictures: [
            include_bytes!("../../../../assets/help/icons-card-en.png"),
            include_bytes!("../../../../assets/help/icons-card-de.png"),
        ],
    },
];

/// The Lines of the Quick start, in `t`'s language.
pub fn quick_start_lines(t: &L10n) -> Vec<String> {
    [
        t.help_quick_start1(),
        t.help_quick_start2(),
        t.help_quick_start3(),
        t.help_quick_start4(),
        t.help_quick_start5(),
        t.help_quick_start6(),
        t.help_quick_start7(),
        t.help_quick_start8(),
    ]
    .map(String::from)
    .to_vec()
}

/// The text of "What the icons mean", as the phone writes it: its name,
/// then each group's heading and its icons, "name: meaning" one per line.
pub fn icons_text(t: &L10n) -> String {
    let mut out = t.help_icons_title().to_string();
    for group in &GROUPS {
        out.push_str("\n\n");
        out.push_str((group.heading)(t));
        for (name, means) in group.icons {
            out.push_str(&format!("\n{}: {}", name(t), means(t)));
        }
    }
    out
}

/// The pictures of the groups in `t`'s language.
pub fn pictures(t: &L10n) -> Vec<&'static [u8]> {
    let german = t.locale() == "de";
    GROUPS
        .iter()
        .map(|g| {
            let [english, in_german] = g.pictures;
            if german { in_german } else { english }
        })
        .collect()
}

/// Makes the help Notes due on this Device in `t`'s language, running
/// `version`; `fresh` right after the first start. Returns the ids made.
pub fn make_due(store: &Store, t: &L10n, version: &str, fresh: bool) -> Result<Vec<String>> {
    let texts = HelpTexts {
        area_name: t.help_area_name().to_string(),
        quick_start_title: t.help_quick_start_title().to_string(),
        quick_start_lines: quick_start_lines(t),
        icons_text: icons_text(t),
        icon_pictures: pictures(t),
        whats_new_title: Box::new(|v| t.help_whats_new_title(v)),
    };
    HelpNotes::new(store).make_due(
        version,
        fresh,
        t.locale(),
        &texts,
        &parse_whats_new(WHATS_NEW),
    )
}

#[cfg(test)]
mod tests {
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
            } else if let Some(rest) = line.strip_prefix("HelpIcon(") {
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
                    let expected = advanced.contains(&means(&t))
                        || named.iter().any(|word| line.contains(word));
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
}
