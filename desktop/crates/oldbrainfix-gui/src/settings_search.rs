//! The search at the top of the Settings (#282), as the phone's
//! `settings_search.dart` has it: a setting shows when every word typed
//! is in its label, its detail line or, without a label of its own, the
//! names of its options, in any case, in the language on screen; a
//! section's heading that matches shows the section whole.

/// The query the field gives: trimmed and lowercased, empty while
/// nothing is typed.
pub fn query_of(typed: &str) -> String {
    typed.trim().to_lowercase()
}

/// Whether `words` match `query`: each of its words is in one of them,
/// in any case; everything matches an empty query.
pub fn matches(words: &[&str], query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let text = words.join("\n").to_lowercase();
    query.split_whitespace().all(|word| text.contains(word))
}

/// Which of a section's `rows`, each by its words, show for `query`
/// under the section's `heading`: all of them while nothing is searched
/// or the heading matches, else those that match.
pub fn rows<const N: usize>(query: &str, heading: Option<&str>, rows: [&[&str]; N]) -> [bool; N] {
    let whole = heading_matches(query, heading);
    rows.map(|words| whole || matches(words, query))
}

/// Whether a section's `heading` shows it whole for `query`: while
/// nothing is searched, or when it matches.
pub fn heading_matches(query: &str, heading: Option<&str>) -> bool {
    query.is_empty() || heading.is_some_and(|h| matches(&[h], query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_typed_is_in_the_label_or_the_detail_line_in_any_case() {
        let words = ["Start at login", "Old Brain Fix starts when you log in"];
        assert!(matches(&words, &query_of("  LOGIN ")));
        assert!(matches(&words, &query_of("start brain")));
        assert!(!matches(&words, &query_of("login quiet")));
        assert!(matches(&words, ""));
    }

    #[test]
    fn a_matching_heading_shows_its_section_whole() {
        let shown = rows("alarms", Some("Alarms"), [&["Alarm sound"], &["Default"]]);
        assert_eq!(shown, [true, true]);
        let shown = rows("sound", Some("Alarms"), [&["Alarm sound"], &["Default"]]);
        assert_eq!(shown, [true, false]);
        let shown = rows("", None, [&["Send feedback"]]);
        assert_eq!(shown, [true]);
        let shown = rows("units", None, [&["Send feedback"]]);
        assert_eq!(shown, [false]);
    }
}
