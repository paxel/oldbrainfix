//! The window driven headless by egui_kittest over the Dart core's
//! fixture logs: Areas and Topics on the left, Notes in the middle, the
//! selected Note on the right, and everything the user changes there.

use std::path::PathBuf;

use chrono::Duration;
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{
    Importance, Note, Notes, Places, SigningKey, Store, WireEntry, dart_iso, parse_time,
};
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
    let mut app = App::new(store(scenario)?, locale);
    // These tests browse the Notes; the window opens on the Agenda.
    app.select(Selection::All);
    // Tall enough for every Note of the `formats` scenario unscrolled,
    // in full with its Sections.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
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

/// Edit on the card of the first Note whose text contains `text`, which
/// opens the Note's editor on the right (#114).
fn edit<'a>(harness: &'a Harness<'_, App>, text: &str) -> egui_kittest::Node<'a> {
    let card = harness
        .query_by_role_and_label(Role::Button, text)
        .or_else(|| {
            harness
                .query_all_by(|n| {
                    n.role() == Role::Button && n.label().is_some_and(|l| l.contains(text))
                })
                .next()
        })
        .unwrap_or_else(|| panic!("no card of {text:?}"))
        .rect();
    // "Edit", or "Bearbeiten" in German.
    harness
        .query_all_by(|n| {
            n.role() == Role::Button && matches!(n.label().as_deref(), Some("Edit" | "Bearbeiten"))
        })
        .find(|n| card.contains_rect(n.rect()))
        .unwrap_or_else(|| panic!("no Edit on the card of {text:?}"))
}

/// Whether a Note with `text` has a card of its own.
fn has_card(harness: &Harness<'_, App>, text: &str) -> bool {
    harness
        .query_by_role_and_label(Role::Button, text)
        .is_some()
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

    // An Area lists only its Notes without a Topic (#95) ...
    harness.get_by_label("Household").click();
    harness.run();
    assert!(shows(&harness, "Sort the cellar"));
    assert!(!shows(&harness, "Descale the kettle"));
    // ... and its search also finds its Topics' Notes, each naming its
    // Topic.
    harness.state_mut().set_query("kettle");
    harness.run();
    assert!(shows(&harness, "Descale the kettle"));
    assert!(shows(&harness, "Household / Kitchen"));
    assert!(!shows(&harness, "Sort the cellar"));
    harness.state_mut().set_query("");
    harness.run();
    assert!(!shows(&harness, "Descale the kettle"));

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
    // Merged Notes leave every list: they show on their target's card.
    assert!(!has_card(&harness, "Dentist on Tuesday"));
    let target = card(&harness, "Dentist number").rect();
    edit(&harness, "Dentist number").click();
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
        let shown: Vec<egui::Rect> = harness
            .query_all_by_label(section)
            .map(|n| n.rect())
            .collect();
        assert!(
            shown.iter().any(|r| r.left() > target.right()),
            "{section} on the right"
        );
        assert!(
            shown.iter().any(|r| target.contains_rect(*r)),
            "{section} on the card"
        );
    }
    // Oldest first, on the right.
    let top = |label| {
        harness
            .query_all_by_label(label)
            .map(|n| n.rect())
            .find(|r| r.left() > target.right())
            .map_or(f32::NAN, |r| r.top())
    };
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
    assert!(!has_card(&harness, "Into the bin with the other"));
    assert!(!shows(&harness, "Not so old after all"));
    // The row on the left and the heading in the middle, then the
    // Note's state on the right.
    assert_eq!(harness.query_all_by_label("Trash").count(), 2);
    edit(&harness, "Old shopping list").click();
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
            {
                let mut app = App::new(empty, "en");
                app.select(Selection::All);
                app
            },
        );
    harness.run();
    assert!(shows(&harness, "Nothing captured yet."));
    harness.get_by_label("Trash").click();
    harness.run();
    assert!(shows(&harness, "The trash is empty."));
    Ok(())
}

// ------------------------------------------------------------ editing

/// The Note whose text is `text`, wherever it is.
fn find(app: &App, text: &str) -> Result<Note, Box<dyn std::error::Error>> {
    let notes = Notes::new(app.store());
    for id in app.store().entities_of_type("note")? {
        if let Some(note) = notes.by_id(&id)?
            && note.text == text
        {
            return Ok(note);
        }
    }
    Err(format!("no Note {text:?}").into())
}

/// The id of the Area or Topic named `name`.
fn place(app: &App, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let places = Places::new(app.store());
    for area in places.areas()? {
        if area.name == name {
            return Ok(area.id);
        }
        for topic in places.topics(&area.id)? {
            if topic.name == name {
                return Ok(topic.id);
            }
        }
    }
    Err(format!("no place {name:?}").into())
}

/// The empty text field whose hint is `hint`.
fn field<'a>(harness: &'a Harness<'_, App>, hint: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(move |n| n.placeholder() == Some(hint))
}

/// The one node labelled `label` right of `x`: in the middle, the right
/// pane or a dialog rather than the left pane.
fn right_of<'a>(
    harness: &'a Harness<'_, App>,
    label: &'a str,
    x: f32,
) -> Result<egui_kittest::Node<'a>, Box<dyn std::error::Error>> {
    let mut found = harness
        .get_all_by_label(label)
        .filter(|n| n.rect().left() > x);
    let node = found.next().ok_or(format!("{label:?} right of {x}"))?;
    assert!(found.next().is_none(), "{label:?} once right of {x}");
    Ok(node)
}

/// Opens the context menu of the left pane's row `label` and picks
/// `item`.
fn menu(harness: &mut Harness<'_, App>, label: &str, item: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_secondary();
    harness.run();
    harness.get_by_label(item).click();
    harness.run();
}

/// The text field that has the keyboard.
fn typing<'a>(harness: &'a Harness<'_, App>) -> egui_kittest::Node<'a> {
    harness.get_by(|n| n.is_focused())
}

/// The text field holding `text`.
fn holding<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> egui_kittest::Node<'a> {
    harness
        .get_by(move |n| n.role() == Role::MultilineTextInput && n.value().as_deref() == Some(text))
}

/// Replaces the focused field's text with `text`.
fn retype(harness: &mut Harness<'_, App>, text: &str) {
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    typing(harness).type_text(text);
}

#[test]
fn a_typed_thought_is_captured_with_the_nudge_picked() -> TestResult {
    let mut harness = harness("places", "en")?;
    let now = harness.state().store().now();
    field(&harness, "What's on your mind?").click();
    harness.run();
    typing(&harness).type_text("Buy milk");
    harness.run();
    // The Nudge times show once something is typed.
    harness.get_by_label("30 min").click();
    harness.run();
    let milk = find(harness.state(), "Buy milk")?;
    assert!(milk.is_unsorted());
    assert_eq!(harness.state().selected_note(), Some(milk.id.as_str()));
    let nudge = |h: &Harness<'_, App>, id: &str| h.state().store().current(id, "nudge");
    assert_eq!(
        nudge(&harness, &milk.id)?,
        Some(dart_iso(now + Duration::minutes(30)))
    );
    assert!(field(&harness, "What's on your mind?").rect().width() > 0.0);

    // Inside a Topic the Note is filed there, sorted: one Save instead
    // of the Nudge times, and Ctrl+Enter saves it without a Nudge (#113).
    harness.get_by_label("Kitchen").click();
    harness.run();
    field(&harness, "What's on your mind?").click();
    harness.run();
    typing(&harness).type_text("  Buy descaler ");
    harness.run();
    assert!(harness.query_by_label("30 min").is_none());
    assert!(harness.query_by_label("Ctrl+Enter saves it.").is_some());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    let descaler = find(harness.state(), "Buy descaler")?;
    assert_eq!(descaler.place, Some(place(harness.state(), "Kitchen")?));
    assert_eq!(nudge(&harness, &descaler.id)?, None);
    assert!(card(&harness, "Buy descaler").rect().width() > 0.0);
    Ok(())
}

#[test]
fn a_notes_text_is_fixed_in_place() -> TestResult {
    let mut harness = harness("notes", "en")?;
    edit(&harness, "Needs more sauce").click();
    harness.run();
    let id = find(harness.state(), "Needs more sauce")?.id;
    holding(&harness, "Needs more sauce").click();
    harness.run();
    retype(&mut harness, "Needs much more sauce ");
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    let text = |h: &Harness<'_, App>| -> Result<_, Box<dyn std::error::Error>> {
        Ok(Notes::new(h.state().store())
            .by_id(&id)?
            .map(|n| n.text)
            .unwrap_or_default())
    };
    assert_eq!(text(&harness)?, "Needs much more sauce");
    // Leaving the field keeps the text too; an emptied one changes
    // nothing.
    holding(&harness, "Needs much more sauce").click();
    harness.run();
    retype(&mut harness, "Sauce, lots");
    harness.run();
    edit(&harness, "Buy more sauce").click();
    harness.run();
    assert_eq!(text(&harness)?, "Sauce, lots");
    let before = harness.state().store().all_entries()?.len();
    edit(&harness, "Sauce, lots").click();
    harness.run();
    holding(&harness, "Sauce, lots").click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    edit(&harness, "Buy more sauce").click();
    harness.run();
    assert_eq!(text(&harness)?, "Sauce, lots");
    assert_eq!(harness.state().store().all_entries()?.len(), before);
    Ok(())
}

#[test]
fn a_text_in_an_unknown_format_shows_as_it_is_and_is_never_overwritten() -> TestResult {
    const NEWER: &str =
        "This text was written with a newer version of Old Brain Fix. Update the app to change it.";
    let mut harness = harness("formats", "en")?;
    assert!(!shows(&harness, NEWER));
    edit(&harness, "# Heading").click();
    harness.run();
    let id = harness
        .state()
        .selected_note()
        .ok_or("selected")?
        .to_string();
    let note = Notes::new(harness.state().store())
        .by_id(&id)?
        .ok_or("note")?;
    assert_eq!(note.text, "# Heading\n**bold** ü");
    assert!(!note.text_editable);
    let stored = |h: &Harness<'_, App>| h.state().store().current(&id, "text");
    let before = stored(&harness)?;
    assert!(shows(&harness, NEWER));
    assert!(
        harness
            .query_by(|n| n.role() == Role::MultilineTextInput
                && n.value().is_some_and(|v| v.contains("Heading")))
            .is_none(),
        "no field to type into"
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    // Amend adds a Section and leaves the text as it is.
    field(&harness, "What's new?").click();
    harness.run();
    typing(&harness).type_text("Oat milk too");
    harness.run();
    harness.get_by_label("Add to Note").click();
    harness.run();
    let sections = Notes::new(harness.state().store()).sections(&id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(stored(&harness)?, before);
    // Written before the format was recorded, it reads as plain.
    edit(&harness, r#"{"format":"plain"}"#).click();
    harness.run();
    assert_eq!(stored(&harness)?, before);
    // A plain Note says nothing of it.
    assert_ne!(harness.state().selected_note(), Some(id.as_str()));
    assert!(!shows(&harness, NEWER));
    Ok(())
}

#[test]
fn a_text_in_a_later_structure_shows_as_it_is_and_version_1_is_plain() -> TestResult {
    const NEWER: &str =
        "This text was written with a newer version of Old Brain Fix. Update the app to change it.";
    let mut harness = harness("formats", "en")?;
    // Version 2, a known format and a key more: its text, read-only.
    edit(&harness, "Version two").click();
    harness.run();
    let id = harness
        .state()
        .selected_note()
        .ok_or("selected")?
        .to_string();
    let before = harness.state().store().current(&id, "text")?;
    assert!(shows(&harness, NEWER));
    assert!(
        harness
            .query_by(|n| n.role() == Role::MultilineTextInput
                && n.value().is_some_and(|v| v.contains("Version two")))
            .is_none(),
        "no field to type into"
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    assert_eq!(harness.state().store().current(&id, "text")?, before);
    // Version 1 in a format not known here: read-only too.
    edit(&harness, "# Version one").click();
    harness.run();
    assert!(shows(&harness, NEWER));
    // Version 1 written out, in a known format, is editable.
    edit(&harness, "Version one written out").click();
    harness.run();
    assert!(!shows(&harness, NEWER));
    holding(&harness, "Version one written out");
    Ok(())
}

#[test]
fn a_note_in_an_unknown_format_is_merged_with_its_text_as_it_is() -> TestResult {
    let mut harness = harness("formats", "en")?;
    edit(&harness, "# Heading").click();
    harness.run();
    let id = harness
        .state()
        .selected_note()
        .ok_or("selected")?
        .to_string();
    let before = harness.state().store().current(&id, "text")?;
    harness.get_by_label("Merge into…").click();
    harness.run();
    // The picker's search has the keyboard; the list behind has one too.
    typing(&harness).type_text("OAT");
    harness.run();
    // The list behind shows the card too; the picker is drawn last.
    harness
        .get_all_by_label("Plain oat milk")
        .last()
        .ok_or("the pick")?
        .click();
    harness.run();
    let merged = Notes::new(harness.state().store())
        .by_id(&id)?
        .ok_or("note")?;
    assert!(merged.merged_into.is_some());
    assert_eq!(harness.state().store().current(&id, "text")?, before);
    Ok(())
}

#[test]
fn a_section_in_an_unknown_format_says_so() -> TestResult {
    let mut harness = harness("formats", "en")?;
    edit(&harness, "Plain host").click();
    harness.run();
    let newer = harness
        .query_all_by_label(
            "This text was written with a newer version of Old Brain Fix. Update the app to change it.",
        )
        .count();
    // The merged Note and its Amend, both formatted later.
    assert_eq!(newer, 2);
    // The host's own text is plain and stays a field to type into.
    holding(&harness, "Plain host");
    Ok(())
}

#[test]
fn a_note_is_amended_weighed_trashed_restored_and_merged() -> TestResult {
    let mut harness = harness("places", "en")?;
    harness.get_by_label("Kitchen").click();
    harness.run();
    edit(&harness, "Descale the kettle").click();
    harness.run();
    let kettle = find(harness.state(), "Descale the kettle")?.id;
    let note = |h: &Harness<'_, App>, id: &str| -> Result<Note, Box<dyn std::error::Error>> {
        Notes::new(h.state().store())
            .by_id(id)?
            .ok_or_else(|| "the Note".into())
    };

    harness.get_by_label("Important").click();
    harness.run();
    assert_eq!(note(&harness, &kettle)?.importance, Importance::Important);

    field(&harness, "What's new?").click();
    harness.run();
    typing(&harness).type_text("Vinegar works");
    harness.run();
    harness.get_by_label("Add to Note").click();
    harness.run();
    let sections = Notes::new(harness.state().store()).sections(&kettle)?;
    assert_eq!(sections.len(), 1);
    assert!(shows(&harness, "Vinegar works"));

    // Deleting asks once; Cancel keeps the Note.
    harness.get_by_label("Move to trash").click();
    harness.run();
    assert!(shows(&harness, "Delete this Note?"));
    assert!(shows(&harness, "It stays in the trash for 30 days."));
    assert!(!shows(&harness, "Delete only the Note"));
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(!shows(&harness, "Delete this Note?"));
    assert!(!note(&harness, &kettle)?.is_trashed());
    harness.get_by_label("Move to trash").click();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    assert!(note(&harness, &kettle)?.is_trashed());
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Descale the kettle")
            .is_none()
    );
    // The Note stays on the right, to be restored at once.
    harness.get_by_label("Restore").click();
    harness.run();
    assert!(!note(&harness, &kettle)?.is_trashed());
    assert!(card(&harness, "Descale the kettle").rect().width() > 0.0);

    // Merging picks from every other Note; the merged one becomes a
    // Section of the one picked.
    harness.get_by_label("Merge into…").click();
    harness.run();
    assert!(shows(&harness, "Sort the cellar"));
    field(&harness, "Search all Notes").type_text("CELLAR");
    harness.run();
    assert!(!shows(&harness, "New tyres"));
    harness.get_by_label("Sort the cellar").click();
    harness.run();
    let cellar = find(harness.state(), "Sort the cellar")?.id;
    assert_eq!(note(&harness, &kettle)?.merged_into, Some(cellar.clone()));
    assert_eq!(harness.state().selected_note(), Some(cellar.as_str()));
    assert!(shows(&harness, "Descale the kettle"));
    assert!(shows(&harness, "Vinegar works"));
    Ok(())
}

#[test]
fn a_note_is_filed_from_its_editor_or_by_dragging_its_card() -> TestResult {
    let mut harness = harness("places", "en")?;
    edit(&harness, "Still Unsorted").click();
    harness.run();
    let id = find(harness.state(), "Still Unsorted")?.id;
    harness.get_by_label("File into…").click();
    harness.run();
    right_of(&harness, "Kitchen", 300.0)?.click();
    harness.run();
    let filed = |h: &Harness<'_, App>| -> Result<_, Box<dyn std::error::Error>> {
        Ok(Notes::new(h.state().store())
            .by_id(&id)?
            .and_then(|n| n.place))
    };
    assert_eq!(filed(&harness)?, Some(place(harness.state(), "Kitchen")?));

    // Dragged onto a Topic on the left, the card is filed there.
    let from = card(&harness, "Still Unsorted").rect().center();
    let onto = harness.get_by_label("Meetings").rect().center();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(20.0, 0.0));
    harness.run();
    harness.hover_at(onto);
    harness.run();
    harness.event(Event::PointerButton {
        pos: onto,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(filed(&harness)?, Some(place(harness.state(), "Meetings")?));
    // Dropped elsewhere, nothing changes.
    let entries = harness.state().store().all_entries()?.len();
    let from = card(&harness, "Still Unsorted").rect().center();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(0.0, 40.0));
    harness.run();
    harness.drop_at(from + egui::vec2(0.0, 40.0));
    harness.run();
    assert_eq!(harness.state().store().all_entries()?.len(), entries);
    Ok(())
}

#[test]
fn areas_and_topics_are_made_renamed_and_given_a_look() -> TestResult {
    let mut harness = harness("places", "en")?;
    harness.get_by_label("New Area").click();
    harness.run();
    field(&harness, "Name").type_text("Garden shed");
    harness.run();
    harness.get_by_label("Create").click();
    harness.run();
    assert!(shows(&harness, "Garden shed"));

    menu(&mut harness, "Garden shed", "Rename");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    typing(&harness).type_text("Shed");
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    assert!(shows(&harness, "Shed"));
    assert!(!shows(&harness, "Garden shed"));
    let shed = place(harness.state(), "Shed")?;

    menu(&mut harness, "Shed", "Colour and icon");
    harness.get_by_label("6A1B9A").click();
    harness.run();
    harness.get_by_label("kids").click();
    harness.run();
    let look = |h: &Harness<'_, App>| -> Result<_, Box<dyn std::error::Error>> {
        let area = Places::new(h.state().store()).area_by_id(&shed)?;
        Ok(area.map(|a| (a.colour, a.icon)))
    };
    assert_eq!(
        look(&harness)?,
        Some((Some("6A1B9A".into()), Some("kids".into())))
    );
    harness.get_by_label("No icon").click();
    harness.run();
    harness.get_by_label("Close").click();
    harness.run();
    assert_eq!(look(&harness)?, Some((Some("6A1B9A".into()), None)));

    // Enter takes the name.
    menu(&mut harness, "Shed", "New Topic");
    field(&harness, "Name").type_text("Tools");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        Places::new(harness.state().store())
            .topics(&shed)?
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["Tools"]
    );

    // A starter renamed to what it shows stays as it was, translatable.
    let entries = harness.state().store().all_entries()?.len();
    menu(&mut harness, "Household", "Rename");
    harness.get_by_label("Save").click();
    harness.run();
    assert_eq!(harness.state().store().all_entries()?.len(), entries);
    // Cancel changes nothing either.
    menu(&mut harness, "Shed", "Rename");
    harness.get_by_label("Cancel").click();
    harness.run();
    assert!(shows(&harness, "Shed"));
    assert_eq!(harness.state().store().all_entries()?.len(), entries);
    Ok(())
}

#[test]
fn deleting_a_place_asks_where_its_notes_go() -> TestResult {
    let mut harness = harness("places", "en")?;
    // An empty Topic just goes.
    menu(&mut harness, "Garden", "Delete");
    assert!(shows(&harness, "Delete the Topic “Garden”?"));
    assert!(!shows(&harness, "Move to Unsorted"));
    right_of(&harness, "Delete", 0.0)?.click();
    harness.run();
    assert!(!shows(&harness, "Garden"));

    // A Topic's Notes go to Unsorted, its Area or the trash.
    harness.get_by_label("Kitchen").click();
    harness.run();
    menu(&mut harness, "Kitchen", "Delete");
    for line in [
        "Delete the Topic “Kitchen”?",
        "1 Note inside.",
        "Where should they go?",
        "Move to Unsorted",
        "Move to trash",
    ] {
        assert!(shows(&harness, line), "{line}");
    }
    harness.get_by_label("Move to “Household”").click();
    harness.run();
    let household = place(harness.state(), "Household")?;
    let kettle = find(harness.state(), "Descale the kettle")?;
    assert_eq!(kettle.place, Some(household));
    assert!(!shows(&harness, "Kitchen"));
    // The deleted Topic was shown; All Notes is now.
    assert_eq!(harness.state().selection(), &Selection::All);

    // An Area takes its Topics along; its Notes go to Unsorted or the
    // trash.
    menu(&mut harness, "Household", "Delete");
    assert!(shows(&harness, "Its Topics are deleted with it."));
    assert!(shows(&harness, "2 Notes inside."));
    assert!(harness.query_by_label_contains("Move to “").is_none());
    harness.get_by_label("Move to trash").click();
    harness.run();
    assert!(find(harness.state(), "Descale the kettle")?.is_trashed());
    assert!(find(harness.state(), "Sort the cellar")?.is_trashed());
    assert!(!shows(&harness, "Household"));
    assert!(!shows(&harness, "Cleaning"));
    Ok(())
}

#[test]
fn back_and_forward_keys_walk_the_areas_and_topics() -> TestResult {
    let mut harness = harness("places", "en")?;
    let household = Selection::Place(place(harness.state(), "Household")?);
    let kitchen = Selection::Place(place(harness.state(), "Kitchen")?);
    harness.get_by_label("Kitchen").click();
    harness.run();
    let mut press = |modifiers, key| {
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        harness.state().selection().clone()
    };
    assert_eq!(press(Modifiers::ALT, Key::ArrowLeft), household);
    assert_eq!(press(Modifiers::ALT, Key::ArrowLeft), Selection::All);
    assert_eq!(press(Modifiers::ALT, Key::ArrowLeft), Selection::All);
    assert_eq!(press(Modifiers::ALT, Key::ArrowRight), household);
    assert_eq!(press(Modifiers::ALT, Key::ArrowRight), kitchen);
    assert_eq!(press(Modifiers::ALT, Key::ArrowRight), kitchen);

    // The mouse's back and forward buttons do the same.
    let mut button = |button| {
        harness.get_by_label("Areas").click_button(button);
        harness.run();
        harness.state().selection().clone()
    };
    assert_eq!(button(PointerButton::Extra1), household);
    assert_eq!(button(PointerButton::Extra2), kitchen);

    // Unsorted and the trash go back to All Notes, and forward into the
    // last Area.
    harness.get_by_label("Trash").click();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selection(), &household);

    // While typing, Alt+arrows stay with the text.
    harness.get_by_label("Unsorted").click();
    harness.run();
    field(&harness, "What's on your mind?").click();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    Ok(())
}

#[test]
fn the_editor_speaks_german() -> TestResult {
    let mut harness = harness("places", "de")?;
    edit(&harness, "Still Unsorted").click();
    harness.run();
    for label in [
        "Ablegen in …",
        "Zusammenführen mit …",
        "In den Papierkorb",
        "Zur Notiz hinzufügen",
        "Oder ihre Karte auf einen Bereich oder ein Thema ziehen.",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(harness.state().error().is_none());
    Ok(())
}

#[test]
fn what_is_typed_is_kept_as_the_window_closes() -> TestResult {
    let mut harness = harness("places", "en")?;
    let now = harness.state().store().now();
    edit(&harness, "Still Unsorted").click();
    harness.run();
    holding(&harness, "Still Unsorted").click();
    harness.run();
    retype(&mut harness, "Still Unsorted, sadly");
    harness.run();
    field(&harness, "What's on your mind?").click();
    harness.run();
    typing(&harness).type_text("Half a thought");
    harness.run();
    harness.state_mut().flush();
    let thought = find(harness.state(), "Half a thought")?;
    let nudge = harness.state().store().current(&thought.id, "nudge")?;
    assert_eq!(nudge, Some(dart_iso(now + Duration::hours(1))));
    assert!(find(harness.state(), "Still Unsorted, sadly").is_ok());
    Ok(())
}

#[test]
fn an_areas_level_adds_topics() -> TestResult {
    let mut harness = harness("places", "en")?;
    harness.get_by_label("Kids").click();
    harness.run();
    harness.get_by_label("New Topic").click();
    harness.run();
    typing(&harness).type_text("Homework");
    harness.run();
    harness.get_by_label("Create").click();
    harness.run();
    assert!(place(harness.state(), "Homework").is_ok());
    // Not on a Topic's level: there is no third.
    harness.get_by_label("Homework").click();
    harness.run();
    assert!(harness.query_by_label("New Topic").is_none());
    Ok(())
}
