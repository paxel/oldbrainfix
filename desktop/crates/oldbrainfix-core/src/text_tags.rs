//! Tags in user text (ADR 0032), as the Dart core's `text_tags.dart`
//! finds them: a `#word` in any text is drawn highlighted, and a click on
//! it searches for it. Nothing is stored for a tag and there is no list
//! of them; it is part of the text.
//!
//! A tag is a `#` followed by letters, digits, `_` and `-`, at least one
//! letter among them, the first no `-` and any `-` at its end left out.
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
        for (i, c) in text.get(from..).unwrap_or_default().char_indices() {
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
    fn offsets_are_bytes() {
        let found = find("🧠 #𝔸");
        assert_eq!(
            found.iter().map(|t| (t.start, t.end)).collect::<Vec<_>>(),
            vec![(5, 10)]
        );
        assert!(find("").is_empty());
    }
}
