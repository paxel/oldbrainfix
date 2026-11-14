//! The strings, generated at build time from `lib/l10n/*.arb` and the
//! desktop's own `desktop/l10n/*.arb` by `oldbrainfix-l10n`. One accessor per key; the ones with placeholders
//! take typed arguments and format through the ICU subset.

include!(concat!(env!("OUT_DIR"), "/l10n.rs"));

use oldbrainfix_core::Wordless;

/// What a Note without words is titled as where only its words fit, as
/// in a row or an alarm ([`oldbrainfix_core::Note::wordless`]): a file by
/// its `name` ([`oldbrainfix_core::Note::wordless_name`]), the word for
/// an Attachment when it has none.
pub fn wordless(t: &L10n, wordless: Wordless, name: &str) -> String {
    match wordless {
        Wordless::Photo => t.photo().to_string(),
        Wordless::VoiceRecording => t.voice_recording().to_string(),
        Wordless::File if name.is_empty() => t.attachment().to_string(),
        Wordless::File => name.to_string(),
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
mod tests;
