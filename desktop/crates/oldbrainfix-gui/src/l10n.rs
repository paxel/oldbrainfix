//! The strings, generated at build time from `lib/l10n/*.arb` and the
//! desktop's own `desktop/l10n/*.arb` by `oldbrainfix-l10n`. One accessor per key; the ones with placeholders
//! take typed arguments and format through the ICU subset.

include!(concat!(env!("OUT_DIR"), "/l10n.rs"));

use oldbrainfix_core::{Note, Wordless};

/// What a Note without words is titled as where only its words fit, as
/// in a row or an alarm ([`oldbrainfix_core::Note::wordless`]): a file by
/// its `name` ([`oldbrainfix_core::Note::wordless_name`]), the word for
/// an Attachment when it has none; a list without a Name by its kind
/// (#309).
pub fn wordless(t: &L10n, wordless: Wordless, name: &str) -> String {
    match wordless {
        Wordless::Photo => t.photo().to_string(),
        Wordless::VoiceRecording => t.voice_recording().to_string(),
        Wordless::File if name.is_empty() => t.attachment().to_string(),
        Wordless::File => name.to_string(),
        Wordless::Checklist => t.checklist_heading().to_string(),
        Wordless::ShoppingList => t.shopping_heading().to_string(),
    }
}

/// What a list without a name shows where its name would be (#309):
/// "Checklist" or "Shopping List" by the list `note` is, as the phone's
/// `untitledList`; `None` for any other Note, which shows nothing there.
pub fn untitled_list(t: &L10n, note: &Note) -> Option<String> {
    match note.list_kind.as_deref() {
        Some(oldbrainfix_core::checklists::keys::CHECKLIST) => {
            Some(t.checklist_heading().to_string())
        }
        Some(oldbrainfix_core::shopping_lists::keys::SHOPPING) => {
            Some(t.shopping_heading().to_string())
        }
        _ => None,
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
