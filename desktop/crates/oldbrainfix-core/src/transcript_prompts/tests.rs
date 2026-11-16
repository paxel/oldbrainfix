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
    let plain =
        TranscriptPrompts::for_places(&places, Vec::<String>::new(), &|a| a.name.clone(), &|t| {
            t.name.clone()
        })?;
    assert_eq!(plain.english, "Arzt, Work, Meetings.");
    let shown =
        TranscriptPrompts::for_places(&places, ["Unsortiert"], &|a| a.name.to_uppercase(), &|t| {
            format!("{}!", t.name)
        })?;
    assert_eq!(shown.english, "ARZT, WORK, Meetings!.");
    assert!(shown.german.ends_with("Unsortiert. ARZT, WORK, Meetings!."));
    Ok(())
}
