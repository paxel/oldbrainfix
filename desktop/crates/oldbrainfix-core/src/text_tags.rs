//! Tags in user text (ADR 0032), as the Dart core's `text_tags.dart`
//! finds them: a `#word` in any text is drawn highlighted, and a click on
//! it searches for it. Nothing is stored for a tag; it is part of the
//! text, and the tag tree is read from the texts ([`crate::tag_tree`]).
//!
//! A tag is a `#` followed by letters, digits, `_` and `-`, at least one
//! letter among them, the first no `-` and any `-` at its end left out.
//! A `/` inside parts a nested tag, such as "#life/health": each part
//! starts as the tag does, so a `/` at the end, `//` and `/-` end it.
//! Before the `#` comes the start of the text or a character that is no
//! letter, digit, `_` or `-` and no `#`, `/`, `\` or `&`, so neither "C#"
//! nor a web address's "/#top" is one. A letter is a character of
//! Unicode's Alphabetic property, a digit one of its numbers. Offsets are
//! in UTF-8 bytes here, where the Dart core counts UTF-16 code units.

use crate::search::lower_case;

/// A tag found in a text: where it lies, `start..end` in bytes with its
/// `#`, and the tag it is, `#` and its name in lower case, which is also
/// what a search for it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundTag {
    pub start: usize,
    pub end: usize,
    pub tag: String,
}

/// Every tag in `text`, first one first.
pub fn find(text: &str) -> Vec<FoundTag> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(hash) = text.get(at..).and_then(|rest| rest.find('#')) {
        let hash = at + hash;
        let from = hash + 1;
        at = from;
        if !text
            .get(..hash)
            .and_then(|before| before.chars().next_back())
            .is_none_or(may_precede)
        {
            continue;
        }
        let mut stop = from;
        let mut letters = false;
        let rest = text.get(from..).unwrap_or_default();
        for (i, c) in rest.char_indices() {
            if c == '/' {
                let next = rest.get(i + 1..).and_then(|r| r.chars().next());
                if i == 0 || !next.is_some_and(|n| n != '-' && in_tag(n)) {
                    break;
                }
                at = from + i + 1;
                continue;
            }
            if !in_tag(c) || (i == 0 && c == '-') {
                break;
            }
            letters |= c.is_alphabetic();
            at = from + i + c.len_utf8();
            if c != '-' {
                stop = at;
            }
        }
        if letters && let Some(name) = text.get(from..stop) {
            found.push(FoundTag {
                start: hash,
                end: stop,
                tag: format!("#{}", lower_case(name)),
            });
        }
    }
    found
}

/// The label `name` of another app as a tag, such as "Home & Garden" as
/// "#Home-Garden": each run of characters that are no letter, digit or
/// `_` becomes one `-`, and a `-` at either end goes; the case stays as
/// written. `None` when nothing is left. A name without a letter, such as
/// "2024", comes to "#2024", which reads as a number, not a tag.
pub fn from_label(name: &str) -> Option<String> {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if in_tag(c) && c != '-' {
            if gap && !out.is_empty() {
                out.push('-');
            }
            out.push(c);
            gap = false;
        } else {
            gap = true;
        }
    }
    (!out.is_empty()).then(|| format!("#{out}"))
}

fn in_tag(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn may_precede(c: char) -> bool {
    !(in_tag(c) || matches!(c, '#' | '/' | '\\' | '&'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tags of `text` as (as written, as searched).
    fn tags(text: &str) -> Vec<(&str, String)> {
        find(text)
            .into_iter()
            .map(|t| (text.get(t.start..t.end).unwrap_or("?"), t.tag))
            .collect()
    }

    fn written(text: &str) -> Vec<&str> {
        tags(text).into_iter().map(|(w, _)| w).collect()
    }

    #[test]
    fn a_tag_is_a_hash_and_a_word_in_any_script() {
        assert_eq!(
            tags("#Garten water\nsee #urlaub2026 #to_do #Müll #Straße #日記 #𝔸x"),
            vec![
                ("#Garten", "#garten".to_string()),
                ("#urlaub2026", "#urlaub2026".to_string()),
                ("#to_do", "#to_do".to_string()),
                ("#Müll", "#müll".to_string()),
                ("#Straße", "#straße".to_string()),
                ("#日記", "#日記".to_string()),
                ("#𝔸x", "#𝔸x".to_string()),
            ]
        );
    }

    #[test]
    fn each_letter_is_lowered_on_its_own_as_the_dart_core_does() {
        assert_eq!(
            tags("#İzmir #ΟΔΟΣ"),
            vec![
                ("#İzmir", "#izmir".to_string()),
                ("#ΟΔΟΣ", "#οδοσ".to_string()),
            ]
        );
    }

    #[test]
    fn a_label_becomes_a_tag_of_its_words() {
        let cases = [
            ("Errands", Some("#Errands")),
            ("Home Garden", Some("#Home-Garden")),
            ("  Home & Garden!  ", Some("#Home-Garden")),
            ("to-do", Some("#to-do")),
            ("#Work", Some("#Work")),
            ("Über_Ideen 日記", Some("#Über_Ideen-日記")),
            ("C#", Some("#C")),
            ("2024", Some("#2024")),
            ("🛒 - !", None),
            ("", None),
        ];
        for (name, tag) in cases {
            assert_eq!(from_label(name).as_deref(), tag, "{name}");
        }
        for name in ["Errands", "Home Garden", "to-do", "Über_Ideen 日記"] {
            let tag = from_label(name).unwrap_or_default();
            assert_eq!(written(&tag), vec![tag.as_str()], "{name}");
        }
    }

    #[test]
    fn a_dash_inside_counts_at_its_ends_it_does_not() {
        assert_eq!(
            written("#to-do #Garten- und #a-- #-a #- #garten. (#x)"),
            vec!["#to-do", "#Garten", "#a", "#garten", "#x"]
        );
    }

    #[test]
    fn numbers_headings_and_hashes_after_words_are_no_tags() {
        assert!(written("#1 issue #126 #_ #² # Heading ## Part #\n##x").is_empty());
        assert!(written("C# F#m x#a 1#a _#a -#a https://a.org/#top \\#e &#x27;").is_empty());
        assert_eq!(written("#v2 #3d #Ⅻ"), vec!["#v2", "#3d", "#Ⅻ"]);
        assert_eq!(written("#a#b"), vec!["#a"]);
    }

    #[test]
    fn a_slash_parts_a_nested_tag_each_part_starting_as_a_tag_does() {
        assert_eq!(
            tags("#life/health/back #Life/Work"),
            vec![
                ("#life/health/back", "#life/health/back".to_string()),
                ("#Life/Work", "#life/work".to_string()),
            ]
        );
        assert_eq!(
            written("#a/ #a-/ #a/- #a/-b #a//b #a/b#c #a/b/ (#a/b). #a-/b"),
            vec![
                "#a", "#a", "#a", "#a", "#a", "#a/b", "#a/b", "#a/b", "#a-/b"
            ]
        );
        assert_eq!(
            written("#q1/2026 #2026/q1 #to-do/x_y"),
            vec!["#q1/2026", "#2026/q1", "#to-do/x_y"]
        );
        assert!(written("#/a #1/2 #2024/05 # /a #-/a").is_empty());
        assert_eq!(
            written("x/#a https://example.org/#top/a page.html#a/b #a/#b"),
            vec!["#a"]
        );
        assert_eq!(written("#c/d"), vec!["#c/d"]);
    }

    #[test]
    fn offsets_are_bytes() {
        let found = find("🧠 #𝔸");
        assert_eq!(
            found.iter().map(|t| (t.start, t.end)).collect::<Vec<_>>(),
            vec![(5, 10)]
        );
        assert!(find("").is_empty());
    }
}
