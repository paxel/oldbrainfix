//! The window driven headless by egui_kittest over the Dart core's
//! fixture logs: Areas and Topics on the left, Notes in the middle, the
//! selected Note on the right.

use std::path::PathBuf;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{SigningKey, Store, WireEntry, parse_time};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store holding the fixture scenario's log, its clock held at the
/// moment the Dart core read it.
fn store(scenario: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(scenario);
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json"))?)?;
    let now = parse_time(expected["now"].as_str().ok_or("expected.json names now")?)?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    for line in std::fs::read_to_string(dir.join("log.jsonl"))?.lines() {
        let entry: WireEntry = serde_json::from_str(line)?;
        store.insert(&entry)?;
    }
    Ok(store)
}

fn harness(
    scenario: &str,
    locale: &str,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let app = App::new(store(scenario)?, locale);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    Ok(harness)
}

/// Whether anything in the window is labelled `label`.
fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The card of the Note with `text`.
fn card<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, text)
}

#[test]
fn places_sit_left_of_the_notes_and_filter_them() -> TestResult {
    let mut harness = harness("places", "en")?;
    for label in [
        "All Notes",
        "Unsorted",
        "Areas",
        "Household",
        "Kitchen",
        "Kids",
        "Trash",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    // A renamed starter keeps the name it was given.
    assert!(shows(&harness, "Bike"));
    let kettle = card(&harness, "Descale the kettle").rect();
    assert!(harness.get_by_label("Kitchen").rect().right() < kettle.left());
    assert!(shows(&harness, "Sort the cellar"));

    harness.get_by_label("Kitchen").click();
    harness.run();
    assert!(matches!(harness.state().selection(), Selection::Place(_)));
    assert!(shows(&harness, "Descale the kettle"));
    assert!(!shows(&harness, "Sort the cellar"));

    // An Area holds its Topics' Notes.
    harness.get_by_label("Household").click();
    harness.run();
    assert!(shows(&harness, "Descale the kettle"));
    assert!(shows(&harness, "Sort the cellar"));

    // The Note whose Topic vanished is Unsorted again.
    harness.get_by_label("Unsorted").click();
    harness.run();
    assert!(shows(&harness, "Where did this go?"));
    assert!(!shows(&harness, "Descale the kettle"));
    Ok(())
}

#[test]
fn starter_names_show_in_the_apps_language() -> TestResult {
    let harness = harness("places", "de")?;
    for label in [
        "Alle Notizen",
        "Unsortiert",
        "Haushalt",
        "Küche",
        "Papierkorb",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(!shows(&harness, "Household"));
    assert!(shows(&harness, "Bike"));
    Ok(())
}

#[test]
fn a_selected_note_shows_right_with_its_sections_and_merged_notes() -> TestResult {
    let mut harness = harness("sections", "en")?;
    // Merged Notes leave every list.
    assert!(!shows(&harness, "Dentist on Tuesday"));
    let target = card(&harness, "Dentist number").rect();
    card(&harness, "Dentist number").click();
    harness.run();
    assert_eq!(
        harness.state().selected_note(),
        Some("note:00000000000000000000000000000005")
    );
    for section in [
        "Dentist on Tuesday",
        "Ask about the crown",
        "Moved to Wednesday",
        "Bring the old X-ray",
        "Called, all set",
    ] {
        let node = harness.get_by_label(section);
        assert!(
            node.rect().left() > target.right(),
            "{section} on the right"
        );
    }
    // Oldest first.
    let top = |label| harness.get_by_label(label).rect().top();
    assert!(top("Dentist on Tuesday") < top("Called, all set"));
    assert!(shows(&harness, "Importance"));
    assert!(shows(&harness, "Normal"));
    Ok(())
}

#[test]
fn trashed_notes_show_only_in_the_trash() -> TestResult {
    let mut harness = harness("trash", "en")?;
    assert!(!shows(&harness, "Old shopping list"));
    assert!(shows(&harness, "Not so old after all"));
    harness.get_by_label("Trash").click();
    harness.run();
    assert!(shows(&harness, "Old shopping list"));
    assert!(shows(&harness, "The other"));
    // Past restoring, and the merged one goes with its target.
    assert!(!shows(&harness, "Long gone"));
    assert!(!shows(&harness, "Into the bin with the other"));
    assert!(!shows(&harness, "Not so old after all"));
    // The row on the left and the heading in the middle, then the
    // Note's state on the right.
    assert_eq!(harness.query_all_by_label("Trash").count(), 2);
    card(&harness, "Old shopping list").click();
    harness.run();
    assert_eq!(harness.query_all_by_label("Trash").count(), 3);
    Ok(())
}

#[test]
fn every_importance_draws_and_an_empty_store_says_so() -> TestResult {
    let harness = harness("notes", "en")?;
    for text in ["Buy more sauce", "Needs more sauce", "Maybe paint the shed"] {
        assert!(
            harness
                .query_by_role_and_label(Role::Button, text)
                .is_some(),
            "{text}"
        );
    }
    let empty = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            App::new(empty, "en"),
        );
    harness.run();
    assert!(shows(&harness, "Nothing captured yet."));
    harness.get_by_label("Trash").click();
    harness.run();
    assert!(shows(&harness, "The trash is empty."));
    Ok(())
}
