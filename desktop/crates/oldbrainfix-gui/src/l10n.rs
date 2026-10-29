//! The strings, generated at build time from `lib/l10n/*.arb` and the
//! desktop's own `desktop/l10n/*.arb` by `oldbrainfix-l10n`. One accessor per key; the ones with placeholders
//! take typed arguments and format through the ICU subset.

include!(concat!(env!("OUT_DIR"), "/l10n.rs"));

use oldbrainfix_core::Wordless;

/// What a Note without words is titled as where only its words fit, as
/// in a row or an alarm ([`oldbrainfix_core::Note::wordless`]).
pub fn wordless(t: &L10n, wordless: Wordless) -> &'static str {
    match wordless {
        Wordless::Photo => t.photo(),
        Wordless::VoiceRecording => t.voice_recording(),
    }
}

/// `names` as a sentence lists them: "Bob", "Bob and Carol", "Bob, Carol
/// and Dave", as the phone's `namesText`.
pub fn names_text(t: &L10n, names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [head @ .., last] => t.names_and(&head.join(", "), last),
    }
}

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
    fn names_are_listed_as_a_sentence() {
        let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let en = L10n::new("en");
        assert_eq!(names_text(&en, &[]), "");
        assert_eq!(names_text(&en, &names(&["Bob"])), "Bob");
        assert_eq!(names_text(&en, &names(&["Bob", "Carol"])), "Bob and Carol");
        assert_eq!(
            names_text(&L10n::new("de"), &names(&["Bob", "Carol", "Dave"])),
            "Bob, Carol und Dave"
        );
    }

    #[test]
    fn a_note_without_words_is_titled_by_what_it_holds() {
        let en = L10n::new("en");
        let de = L10n::new("de");
        assert_eq!(wordless(&en, Wordless::Photo), "Photo");
        assert_eq!(wordless(&en, Wordless::VoiceRecording), "Voice recording");
        assert_eq!(wordless(&de, Wordless::Photo), "Foto");
        assert_eq!(wordless(&de, Wordless::VoiceRecording), "Sprachaufnahme");
    }

    #[test]
    fn the_desktop_strings_are_translated() {
        let en = L10n::new("en");
        let de = L10n::new("de");
        for (en, de) in [
            (en.close(), de.close()),
            (en.capture_keys_hint(), de.capture_keys_hint()),
            (en.drag_to_file(), de.drag_to_file()),
            (en.popup_keys_hint(), de.popup_keys_hint()),
            (en.hotkey_unavailable(), de.hotkey_unavailable()),
            (en.cli_usage(), de.cli_usage()),
            (en.bundle_save(), de.bundle_save()),
            (en.bundle_open(), de.bundle_open()),
            (en.bundle_drop(), de.bundle_drop()),
            (en.bundle_pair_first(), de.bundle_pair_first()),
            (en.from_file(), de.from_file()),
            (en.from_file_title(), de.from_file_title()),
            (en.from_file_hint(), de.from_file_hint()),
            (
                en.file_recording_unreadable(),
                de.file_recording_unreadable(),
            ),
            (
                en.file_recording_unsupported(),
                de.file_recording_unsupported(),
            ),
            (en.file_recording_fix(), de.file_recording_fix()),
            (en.calendar_file_not_saved(), de.calendar_file_not_saved()),
            (
                en.calendar_file_not_saved_title(),
                de.calendar_file_not_saved_title(),
            ),
            (
                en.calendar_file_not_saved_cause(),
                de.calendar_file_not_saved_cause(),
            ),
            (
                en.calendar_file_not_saved_fix(),
                de.calendar_file_not_saved_fix(),
            ),
            (en.changes_not_saved(), de.changes_not_saved()),
            (en.changes_not_saved_title(), de.changes_not_saved_title()),
            (en.changes_not_saved_cause(), de.changes_not_saved_cause()),
            (en.changes_not_saved_fix(), de.changes_not_saved_fix()),
            (en.changes_nothing_new(), de.changes_nothing_new()),
            (en.action_failed(), de.action_failed()),
            (en.action_failed_title(), de.action_failed_title()),
            (en.action_failed_cause(), de.action_failed_cause()),
            (en.action_failed_fix(), de.action_failed_fix()),
            (en.autostart_failed(), de.autostart_failed()),
            (en.autostart_failed_title(), de.autostart_failed_title()),
            (en.autostart_failed_cause(), de.autostart_failed_cause()),
            (en.autostart_failed_fix(), de.autostart_failed_fix()),
            (en.bundle_not_saved(), de.bundle_not_saved()),
            (en.bundle_not_saved_title(), de.bundle_not_saved_title()),
            (en.bundle_saved(), de.bundle_saved()),
            (en.sync_host_failed_cause(), de.sync_host_failed_cause()),
            (en.sync_host_failed_fix(), de.sync_host_failed_fix()),
        ] {
            assert_ne!(en, de);
        }
        assert_ne!(en.sync_conflicts_below(2), de.sync_conflicts_below(2));
        assert_ne!(en.sync_conflicts_arrived(2), de.sync_conflicts_arrived(2));
        // The count only picks the grammar: no value in an announcement.
        assert!(!en.sync_conflicts_arrived(3).contains('3'));
        assert!(!de.sync_conflicts_arrived(3).contains('3'));
        assert_eq!(
            en.file_recording_failed(1),
            "A recording from a file cannot be transcribed."
        );
        assert_ne!(en.file_recording_failed(2), de.file_recording_failed(2));
        // The count only picks the grammar: no value in an announcement.
        assert_eq!(
            en.file_recording_failed(3),
            "Recordings from files cannot be transcribed."
        );
        assert_ne!(
            en.person_filter_remove("Ben"),
            de.person_filter_remove("Ben")
        );
        assert!(en.cli_usage().contains("oldbrainfix import FILE"));
        assert_eq!(de.close(), "Schließen");
    }
}
