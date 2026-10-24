//! The strings, generated at build time from `lib/l10n/*.arb` and the
//! desktop's own `desktop/l10n/*.arb` by `oldbrainfix-l10n`. One accessor per key; the ones with placeholders
//! take typed arguments and format through the ICU subset.

include!(concat!(env!("OUT_DIR"), "/l10n.rs"));

/// The app's locale for a system locale such as `de_DE.UTF-8` or
/// `en-US`: the language part when the app speaks it, else English.
pub fn locale_for_system(system: Option<&str>) -> &'static str {
    let lang = system
        .unwrap_or("en")
        .split(['-', '_', '.'])
        .next()
        .unwrap_or("en")
        .to_lowercase();
    LOCALES.iter().copied().find(|l| *l == lang).unwrap_or("en")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phone_strings_are_here_in_both_languages() {
        assert_eq!(LOCALES, ["en", "de"]);
        let en = L10n::new("en");
        let de = L10n::new("de");
        assert_eq!(en.app_title(), "Old Brain Fix");
        assert_eq!(en.unsorted(), "Unsorted");
        assert_eq!(de.unsorted(), "Unsortiert");
        assert_eq!(en.notes_inside(1), "1 Note inside.");
        assert_eq!(en.notes_inside(3), "3 Notes inside.");
        assert_eq!(L10n::new("xx"), en);
        assert_eq!(locale_for_system(Some("de_DE.UTF-8")), "de");
        assert_eq!(locale_for_system(Some("en-US")), "en");
        assert_eq!(locale_for_system(Some("fr")), "en");
        assert_eq!(locale_for_system(None), "en");
    }

    #[test]
    fn the_desktop_strings_are_translated() {
        let en = L10n::new("en");
        let de = L10n::new("de");
        for (en, de) in [
            (en.close(), de.close()),
            (en.capture_keys_hint(), de.capture_keys_hint()),
            (en.drag_to_file(), de.drag_to_file()),
        ] {
            assert_ne!(en, de);
        }
        assert_eq!(de.close(), "Schließen");
    }
}
