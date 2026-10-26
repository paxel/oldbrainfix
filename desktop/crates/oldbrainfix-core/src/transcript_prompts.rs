//! The lead-in Whisper reads before it transcribes a voice recording
//! (whisper.cpp's initial prompt), one per language it can hear, as the
//! Dart core's `TranscriptPrompts` writes them: words it should expect,
//! so "Unsorted" or "Meeting" in German speech are not written in German
//! spelling. Whisper picks English or German by the recording and takes
//! the matching one; an empty one leads in nothing.
//!
//! Whisper keeps only the last 224 tokens of a prompt and drops the
//! start, so a prompt stays under [`MAX_LENGTH`] characters and lists at
//! most [`MAX_NAMES`] names.

use std::collections::HashSet;

use crate::error::Result;
use crate::places::{Area, Places, Topic};

/// German with the English words German speech is full of, as a
/// sentence, since Whisper follows the style of what it reads.
pub const GERMAN_LEAD_IN: &str = "Notiz auf Deutsch mit englischen Wörtern: Nach dem Meeting das \
     Update ans Team, Feedback vor der Deadline, Call und Termin am \
     Montag, Shopping, Download, Ticket, Event, Homeoffice.";

/// The most characters (Unicode code points) a prompt has: well under
/// Whisper's 224 tokens even for German, at about three characters a
/// token.
pub const MAX_LENGTH: usize = 400;

/// The most Area and Topic names a prompt lists.
pub const MAX_NAMES: usize = 40;

/// What Whisper is led in with, by the language it hears.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranscriptPrompts {
    /// For a recording in English: the Area and Topic names.
    pub english: String,
    /// For a recording in German: [`GERMAN_LEAD_IN`], the app's words and
    /// the Area and Topic names.
    pub german: String,
}

impl TranscriptPrompts {
    /// The prompts for `names` (Area and Topic names, the most wanted
    /// first) and `terms` (the app's own words, such as Unsorted, as the
    /// user reads them). Names are trimmed, empty ones and repeats (in
    /// any case) left out; a name that no longer fits is left out and the
    /// next one tried.
    pub fn of<N, T>(names: N, terms: T) -> TranscriptPrompts
    where
        N: IntoIterator,
        N::Item: AsRef<str>,
        T: IntoIterator,
        T::Item: AsRef<str>,
    {
        let terms: Vec<String> = terms
            .into_iter()
            .map(|term| dart_trim(term.as_ref()).to_string())
            .filter(|term| !term.is_empty())
            .collect();
        let mut german = GERMAN_LEAD_IN.to_string();
        if !terms.is_empty() {
            german = format!("{german} {}.", terms.join(", "));
        }
        // The German prompt is the longer one: what fits there fits in both.
        let budget = MAX_LENGTH as i64 - length(&german) as i64 - 2;
        let listed = fit(names, budget);
        if listed.is_empty() {
            return TranscriptPrompts {
                english: String::new(),
                german,
            };
        }
        let list = format!("{}.", listed.join(", "));
        TranscriptPrompts {
            german: format!("{german} {list}"),
            english: list,
        }
    }

    /// The prompts for the Areas and Topics in `places`: the Areas first,
    /// by stored name, then their Topics, each as `area_name` and
    /// `topic_name` show it.
    pub fn for_places<T>(
        places: &Places,
        terms: T,
        area_name: &dyn Fn(&Area) -> String,
        topic_name: &dyn Fn(&Topic) -> String,
    ) -> Result<TranscriptPrompts>
    where
        T: IntoIterator,
        T::Item: AsRef<str>,
    {
        let areas = places.areas()?;
        let mut names: Vec<String> = areas.iter().map(area_name).collect();
        for area in &areas {
            names.extend(places.topics(&area.id)?.iter().map(topic_name));
        }
        Ok(TranscriptPrompts::of(names, terms))
    }
}

/// The names of `names` that fit into `budget` characters, joined by
/// ", ", at most [`MAX_NAMES`] of them.
fn fit<N>(names: N, budget: i64) -> Vec<String>
where
    N: IntoIterator,
    N::Item: AsRef<str>,
{
    let mut seen = HashSet::new();
    let mut fitting: Vec<String> = Vec::new();
    let mut used = 0i64;
    for raw in names {
        if fitting.len() == MAX_NAMES {
            break;
        }
        let name = dart_trim(raw.as_ref());
        if name.is_empty() || !seen.insert(lower(name)) {
            continue;
        }
        let cost = length(name) as i64 + if fitting.is_empty() { 0 } else { 2 };
        if used + cost > budget {
            continue;
        }
        fitting.push(name.to_string());
        used += cost;
    }
    fitting
}

/// `text` without the whitespace Dart's `trim` takes off: Unicode's
/// White_Space and the byte order mark.
fn dart_trim(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// `text` in lower case character by character, one character for one,
/// as Dart's `toLowerCase` does it: with no regard for a word's end, and
/// a dotted capital I becomes a plain i.
fn lower(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            '\u{130}' => 'i'.to_lowercase(),
            c => c.to_lowercase(),
        })
        .collect()
}

/// The length of `text` in Unicode code points, as Dart's `runes`.
fn length(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    const TERMS: [&str; 3] = ["Unsortiert", "Routine", "Agenda"];

    #[test]
    fn both_prompts_list_the_area_and_topic_names() {
        let prompts = TranscriptPrompts::of(["Work", "Garten", "Fahrradladen"], TERMS);
        assert_eq!(prompts.english, "Work, Garten, Fahrradladen.");
        for name in ["Work", "Garten", "Fahrradladen"] {
            assert!(prompts.german.contains(name), "{}", prompts.german);
        }
    }

    #[test]
    fn german_leads_in_with_english_words_then_the_apps_words() {
        let prompts = TranscriptPrompts::of(["Garten"], TERMS);
        for word in ["Meeting", "Deadline", "Update", "Shopping", "Termin"] {
            assert!(prompts.german.contains(word));
        }
        assert_eq!(
            prompts.german,
            format!("{GERMAN_LEAD_IN} Unsortiert, Routine, Agenda. Garten.")
        );
        assert_eq!(prompts.english, "Garten.");
    }

    #[test]
    fn without_names_english_leads_in_nothing_german_still_does() {
        let prompts = TranscriptPrompts::of(Vec::<String>::new(), Vec::<String>::new());
        assert_eq!(prompts.english, "");
        assert_eq!(prompts.german, GERMAN_LEAD_IN);
    }

    #[test]
    fn empty_names_and_repeats_in_any_case_are_left_out() {
        let names = [" Work ", "", "work", "WORK", "  ", "\u{feff}Home\u{a0}"];
        let prompts = TranscriptPrompts::of(names, Vec::<String>::new());
        assert_eq!(prompts.english, "Work, Home.");
    }

    #[test]
    fn many_long_names_are_cut_so_both_prompts_fit_whisper() {
        let names: Vec<String> = (0..200)
            .map(|i| format!("Grundstücksverwaltungsangelegenheit {i}"))
            .collect();
        let prompts = TranscriptPrompts::of(&names, TERMS);
        assert!(length(&prompts.german) <= MAX_LENGTH);
        assert!(length(&prompts.english) <= MAX_LENGTH);
        assert!(prompts.german.starts_with(GERMAN_LEAD_IN));
        assert!(prompts.german.contains("angelegenheit 0"));
        assert!(!prompts.english.contains("angelegenheit 199"));
    }

    #[test]
    fn at_most_max_names_and_a_name_too_long_is_skipped() {
        let names: Vec<String> = (0..100).map(|i| format!("N{i}")).collect();
        let prompts = TranscriptPrompts::of(&names, Vec::<String>::new());
        assert_eq!(prompts.english.split(", ").count(), MAX_NAMES);
        let prompts = TranscriptPrompts::of(["x".repeat(500), "Work".into()], TERMS);
        assert_eq!(prompts.english, "Work.");
    }

    #[test]
    fn long_terms_leave_room_for_no_name() {
        let prompts = TranscriptPrompts::of(["Work"], ["y".repeat(500)]);
        assert_eq!(prompts.english, "");
        assert!(prompts.german.ends_with(&format!("{}.", "y".repeat(500))));
    }

    #[test]
    fn places_list_the_areas_first_then_their_topics_as_shown() -> Result<()> {
        let store = Store::in_memory(Box::new(chrono::Utc::now), || {
            Ok(crate::signing::SigningKey::from_seed([1; 32]))
        })?;
        let places = Places::new(&store);
        let work = places.add_area("Work")?;
        places.add_area("Arzt")?;
        places.add_topic(&work.id, "Meetings")?;
        let plain = TranscriptPrompts::for_places(
            &places,
            Vec::<String>::new(),
            &|a| a.name.clone(),
            &|t| t.name.clone(),
        )?;
        assert_eq!(plain.english, "Arzt, Work, Meetings.");
        let shown = TranscriptPrompts::for_places(
            &places,
            ["Unsortiert"],
            &|a| a.name.to_uppercase(),
            &|t| format!("{}!", t.name),
        )?;
        assert_eq!(shown.english, "ARZT, WORK, Meetings!.");
        assert!(shown.german.ends_with("Unsortiert. ARZT, WORK, Meetings!."));
        Ok(())
    }
}
