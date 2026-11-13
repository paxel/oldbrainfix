//! The tag tree on the desktop (#267, Q623, Q639): Tags below the Areas
//! in the sidebar lists every tag by its levels with its count of Notes,
//! the levels below unfolding by the arrow, a click on a tag searching
//! All Notes for it; and "#" typed into a list's search offers the tags
//! starting with it as buttons under the field. Driven headless by
//! egui_kittest.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, SigningKey, Store, parse_time};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(texts: &[&str]) -> Result<Store, Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-13T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([5; 32])))?;
    for text in texts {
        Notes::new(&store).capture(text, None, None)?;
    }
    Ok(store)
}

fn harness(store: Store) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(Selection::Agenda);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// How many buttons named `label` show.
fn buttons<S>(harness: &Harness<'_, S>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

const NOTES: [&str; 4] = [
    "Back exercises #life/health/back",
    "Dentist #Life/health",
    "Seeds #garden",
    "Tax #life",
];

#[test]
fn tags_below_the_areas_list_every_tag_by_its_levels() -> TestResult {
    let mut harness = harness(store(&NOTES)?);
    harness.get_by_label("Tags").click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Tags);
    // The top level alone at first, each with its count of Notes.
    assert!(shows(&harness, "#garden") && shows(&harness, "#life"));
    assert!(shows(&harness, "1 Note") && shows(&harness, "3 Notes"));
    assert!(!shows(&harness, "health"));
    // Only a tag with levels below has the arrow.
    assert_eq!(harness.query_all_by_label("Show the tags below").count(), 1);
    harness.get_by_label("Show the tags below").click();
    harness.run();
    assert!(shows(&harness, "health") && shows(&harness, "2 Notes"));
    assert!(!shows(&harness, "back"));
    harness.get_by_label("Show the tags below").click();
    harness.run();
    assert!(shows(&harness, "back"));
    // A click on a level searches All Notes for the tag down to it.
    harness.get_by_label("health").click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "#life/health");
    assert_eq!(harness.state().shown_notes().len(), 2);
    // Back to the tree, still unfolded.
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Tags);
    assert!(shows(&harness, "back"));
    // Folding #life hides all below it.
    assert_eq!(harness.query_all_by_label("Hide the tags below").count(), 2);
    harness
        .query_all_by_label("Hide the tags below")
        .next()
        .ok_or("the arrow of #life")?
        .click();
    harness.run();
    assert!(!shows(&harness, "health") && !shows(&harness, "back"));
    Ok(())
}

#[test]
fn without_tags_the_tree_says_how_one_comes() -> TestResult {
    let mut harness = harness(store(&["Milk"])?);
    harness.get_by_label("Tags").click();
    harness.run();
    assert!(shows(
        &harness,
        "No tags yet. A word with # in front in a Note, such as #garden or #life/health, shows here."
    ));
    Ok(())
}

#[test]
fn a_hash_typed_into_the_search_offers_the_tags_starting_with_it() -> TestResult {
    let mut harness = harness(store(&NOTES)?);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("#li");
    harness.run();
    // Read with the search, shown the frame after.
    harness.run();
    for tag in ["#life", "#life/health", "#life/health/back"] {
        assert_eq!(buttons(&harness, tag), 1, "{tag}");
    }
    assert_eq!(buttons(&harness, "#garden"), 0);
    harness
        .get_by_role_and_label(Role::Button, "#life/health")
        .click();
    harness.run();
    assert_eq!(harness.state().query(), "#life/health");
    harness.run();
    // The cursor stands after it, to type on.
    harness.get_by(|n| n.is_focused()).type_text("/b");
    harness.run();
    assert_eq!(harness.state().query(), "#life/health/b");
    harness.state_mut().set_query("#life/health");
    harness.run();
    // The one typed exactly is left out; those below it stay.
    assert_eq!(buttons(&harness, "#life/health/back"), 1);
    assert_eq!(buttons(&harness, "#life/health"), 0);
    // Without a "#", none.
    harness.state_mut().set_query("life");
    harness.run();
    assert_eq!(buttons(&harness, "#life/health/back"), 0);
    Ok(())
}
