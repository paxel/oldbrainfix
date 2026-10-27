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
//!
//! On silent or very short recordings Whisper may write out its prompt
//! instead of what it heard; [`TranscriptPrompts::guard`] takes that out
//! of a Transcript.

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
    /// The Area and Topic names both prompts list, as they stand there.
    pub names: Vec<String>,
    /// The app's words the German prompt lists, as they stand there.
    pub terms: Vec<String>,
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
                names: Vec::new(),
                terms,
            };
        }
        let list = format!("{}.", listed.join(", "));
        TranscriptPrompts {
            german: format!("{german} {list}"),
            english: list,
            names: listed,
            terms,
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

    /// `transcript` without what Whisper wrote out of these prompts
    /// rather than heard: every repeat of [`GERMAN_LEAD_IN`], whole or
    /// from its start at least through "Wörtern:", and every run of three
    /// or more of the listed names and terms one after another. A
    /// Transcript that is nothing but the listed names, two or more, in
    /// their order, goes whole. Words are compared in any case and
    /// without the punctuation around them, a name of several words as a
    /// whole. A lone name, or two amid speech, stays: real speech names
    /// places too.
    ///
    /// With nothing taken out, `transcript` comes back as it is;
    /// otherwise what is left of it, its words joined by single spaces,
    /// or empty when no word is left.
    pub fn guard(&self, transcript: &str) -> String {
        // Each word with the punctuation alone after it; what comes
        // before the first word goes with that.
        let mut words: Vec<String> = Vec::new();
        let mut groups: Vec<Vec<&str>> = Vec::new();
        let mut before: Vec<&str> = Vec::new();
        for token in tokens(transcript) {
            let word = word(token);
            if !word.is_empty() {
                words.push(word);
                before.push(token);
                groups.push(std::mem::take(&mut before));
            } else if let Some(group) = groups.last_mut() {
                group.push(token);
            } else {
                before.push(token);
            }
        }
        let mut dropped = vec![false; words.len()];
        let lead_in = words_of(GERMAN_LEAD_IN);
        let opening = words_of(
            GERMAN_LEAD_IN
                .split_once(':')
                .map_or(GERMAN_LEAD_IN, |(opening, _)| opening),
        );
        let mut i = 0;
        while i < words.len() {
            let mut n = 0;
            while n < lead_in.len() && i + n < words.len() && words[i + n] == lead_in[n] {
                n += 1;
            }
            if n >= opening.len() {
                dropped[i..i + n].fill(true);
                i += n;
            } else {
                i += 1;
            }
        }
        let left: Vec<usize> = (0..words.len()).filter(|&i| !dropped[i]).collect();
        let rest: Vec<&str> = left.iter().map(|&i| words[i].as_str()).collect();
        let listed: Vec<String> = self.names.iter().flat_map(|n| words_of(n)).collect();
        if self.names.len() >= 2 && rest == listed {
            for &i in &left {
                dropped[i] = true;
            }
        } else {
            let vocabulary: Vec<Vec<String>> = self
                .names
                .iter()
                .chain(&self.terms)
                .map(|item| words_of(item))
                .filter(|item| !item.is_empty())
                .collect();
            // The length in words of the longest name or term at rest[at].
            let matching = |at: usize| {
                let mut longest = 0;
                for item in &vocabulary {
                    if item.len() <= longest || at + item.len() > rest.len() {
                        continue;
                    }
                    if rest[at..at + item.len()] == item[..] {
                        longest = item.len();
                    }
                }
                longest
            };
            let mut i = 0;
            while i < rest.len() {
                let mut end = i;
                let mut count = 0;
                loop {
                    let n = matching(end);
                    if n == 0 {
                        break;
                    }
                    end += n;
                    count += 1;
                }
                if count >= 3 {
                    for &k in &left[i..end] {
                        dropped[k] = true;
                    }
                }
                i = if end > i { end } else { i + 1 };
            }
        }
        if !dropped.contains(&true) {
            return transcript.to_string();
        }
        groups
            .iter()
            .zip(&dropped)
            .filter(|(_, dropped)| !**dropped)
            .flat_map(|(group, _)| group.iter().copied())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// What a word may start or end with that is no part of it.
const PUNCTUATION: &[char] = &[
    '.', ',', ':', ';', '!', '?', '…', '"', '\'', '„', '“', '”', '‚', '‘', '’', '«', '»', '(', ')',
    '[', ']', '–', '—', '-',
];

/// The parts of `text` between whitespace, as Dart's `trim` knows it.
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(is_space).filter(|token| !token.is_empty())
}

/// Unicode's White_Space and the byte order mark.
fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// `token` in lower case without the punctuation around it; empty when
/// it is punctuation alone.
fn word(token: &str) -> String {
    lower(token.trim_matches(PUNCTUATION))
}

/// The words of `text`, as [`TranscriptPrompts::guard`] compares them.
fn words_of(text: &str) -> Vec<String> {
    tokens(text).map(word).filter(|w| !w.is_empty()).collect()
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
    text.trim_matches(is_space)
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
    fn a_transcript_that_only_repeats_a_prompt_is_left_empty() {
        let prompts = TranscriptPrompts::of(["Arzt", "Work", "Meetings"], TERMS);
        assert_eq!(prompts.names, ["Arzt", "Work", "Meetings"]);
        assert_eq!(prompts.terms, TERMS);
        assert_eq!(prompts.guard(GERMAN_LEAD_IN), "");
        assert_eq!(prompts.guard(&prompts.german), "");
        assert_eq!(prompts.guard(&prompts.english), "");
        let twice = format!(" {} {GERMAN_LEAD_IN}", GERMAN_LEAD_IN.to_uppercase());
        assert_eq!(prompts.guard(&twice), "");
        assert_eq!(
            prompts.guard("Notiz auf Deutsch mit englischen Wörtern:"),
            ""
        );
    }

    #[test]
    fn speech_beside_the_lead_in_or_naming_places_stays() {
        let prompts = TranscriptPrompts::of(["Arzt", "Work", "Meetings"], TERMS);
        assert_eq!(
            prompts.guard(&format!("{GERMAN_LEAD_IN} Morgen den Zahnarzt anrufen.")),
            "Morgen den Zahnarzt anrufen."
        );
        assert_eq!(
            prompts.guard(&format!("Brot holen. {GERMAN_LEAD_IN}")),
            "Brot holen."
        );
        for speech in [
            "  Morgen um zehn zum Arzt,  Rezept nicht vergessen. ",
            "Arzt und Work.",
            "Work, Arzt.",
            "Nach dem Meeting das Update ans Team.",
            "Notiz auf Deutsch mit englischen Sätzen.",
        ] {
            assert_eq!(prompts.guard(speech), speech);
        }
    }

    #[test]
    fn three_names_in_a_row_go_two_only_as_the_whole_list() {
        let prompts = TranscriptPrompts::of(["Arzt", "Work", "Meetings"], TERMS);
        assert_eq!(
            prompts.guard("Erst Arzt, Work, Meetings – dann Pause."),
            "Erst dann Pause."
        );
        assert_eq!(prompts.guard("Unsortiert, Routine, Agenda."), "");
        let two = TranscriptPrompts::of(["Arzt", "Work"], Vec::<String>::new());
        assert_eq!(two.guard("Arzt, Work."), "");
        assert_eq!(two.guard("Work, Arzt."), "Work, Arzt.");
        let one = TranscriptPrompts::of(["Garten"], Vec::<String>::new());
        assert_eq!(one.guard("Garten."), "Garten.");
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
