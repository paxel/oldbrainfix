use super::*;

fn state(text: &str, importance: Importance) -> State {
    State {
        typed: Typed {
            text: text.to_string(),
            sections: BTreeMap::new(),
        },
        kept: Kept {
            kind: NoteKind::Plain,
            format: "plain".to_string(),
            language: None,
            importance,
            place: None,
            lines: None,
            on_agenda: None,
            hidden: false,
            ticks: false,
            tally: None,
            session: None,
        },
    }
}

#[test]
fn typing_is_a_step_once_it_rests_or_something_else_is_done() {
    let mut steps = Steps::new("n", state("Mi", Importance::Normal));
    assert!(!steps.see(state("Mil", Importance::Normal), See::Drawn(0.0)));
    assert!(steps.typing());
    // Typed on, the pause starts again.
    assert!(!steps.see(state("Milk", Importance::Normal), See::Drawn(0.5)));
    assert!(!steps.see(state("Milk", Importance::Normal), See::Drawn(1.2)));
    assert!(steps.see(state("Milk", Importance::Normal), See::Drawn(1.6)));
    assert_eq!(steps.undo_to().map(|s| s.typed.text.as_str()), Some("Mi"));
    // Typing before a badge is picked is a step of its own.
    assert!(steps.see(state("Milk, eggs", Importance::Normal), See::Before));
    assert!(steps.see(state("Milk, eggs", Importance::Important), See::After));
    assert_eq!(
        steps.undo_to().map(|s| s.kept.importance),
        Some(Importance::Normal)
    );
    assert!(steps.changed());
}

#[test]
fn a_save_that_trims_and_gives_boxes_ids_is_no_step() {
    let mut steps = Steps::new("n", state("Milk", Importance::Normal));
    assert!(!steps.see(state("Milk \n", Importance::Normal), See::Before));
    assert!(!steps.see(state("Milk", Importance::Normal), See::After));
    assert_eq!(steps.at().typed.text, "Milk");
    assert!(!steps.changed());
}

#[test]
fn something_written_elsewhere_starts_the_steps_again() {
    let mut steps = Steps::new("n", state("Milk", Importance::Normal));
    assert!(steps.see(state("Milk", Importance::Important), See::After));
    assert!(!steps.see(state("Milk", Importance::Unimportant), See::Before));
    assert!(steps.undo_to().is_none());
    assert!(!steps.changed());
    assert_eq!(steps.opened().kept.importance, Importance::Unimportant);
}

#[test]
fn undo_redo_and_revert_walk_the_steps() {
    let mut steps = Steps::new("n", state("a", Importance::Normal));
    steps.see(state("ab", Importance::Normal), See::Before);
    steps.see(state("abc", Importance::Normal), See::Before);
    steps.undone(state("ab", Importance::Normal));
    assert_eq!(steps.redo_to().map(|s| s.typed.text.as_str()), Some("abc"));
    assert_eq!(steps.undo_to().map(|s| s.typed.text.as_str()), Some("a"));
    steps.redone(state("abc", Importance::Normal));
    assert!(steps.redo_to().is_none());
    steps.reverted(state("a", Importance::Normal));
    assert!(!steps.changed());
    assert!(steps.undo_to().is_none());
    // Redo makes the changes again one by one.
    assert_eq!(steps.redo_to().map(|s| s.typed.text.as_str()), Some("ab"));
    steps.redone(state("ab", Importance::Normal));
    assert_eq!(steps.redo_to().map(|s| s.typed.text.as_str()), Some("abc"));
    // A new step forgets what lay ahead.
    steps.see(state("abd", Importance::Normal), See::Before);
    assert!(steps.redo_to().is_none());
}

#[test]
fn a_new_word_begun_makes_the_words_before_it_a_step() {
    let mut steps = Steps::new("n", state("Milk", Importance::Normal));
    assert!(!steps.see(state("Milk and", Importance::Normal), See::Drawn(0.0)));
    assert!(steps.see(
        state("Milk and bread", Importance::Normal),
        See::Drawn(0.25)
    ));
    assert_eq!(steps.undo_to().map(|s| s.typed.text.as_str()), Some("Milk"));
    assert_eq!(steps.at().typed.text, "Milk and");
    assert!(steps.typing());
    // Inside a word, at its end or with letters only, no step.
    assert!(!begins_word("Milk", "Milky"));
    assert!(!begins_word("Milk  ", "Milk   "));
    assert!(begins_word("Milk", "Milk "));
    assert!(begins_word("Brot für", "Brot für\nmich"));
    assert!(!begins_word("Brot für", "Brot fü"));
}

#[test]
fn a_section_added_and_an_emptied_text_make_no_step() {
    let mut steps = Steps::new("n", state("Milk", Importance::Normal));
    let mut amended = state("Milk", Importance::Normal);
    amended
        .typed
        .sections
        .insert("s".to_string(), "Eggs".to_string());
    assert!(!steps.see(amended.clone(), See::After));
    assert!(!steps.changed());
    assert!(!steps.see(state("  ", Importance::Normal), See::Before));
    assert!(!steps.see(state("", Importance::Normal), See::Drawn(5.0)));
    assert!(steps.undo_to().is_none());
    // Its own text typed is a step again.
    amended
        .typed
        .sections
        .insert("s".to_string(), "Eggs, ham".to_string());
    assert!(steps.see(amended, See::Before));
}
