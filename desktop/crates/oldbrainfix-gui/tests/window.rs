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

    // Inside a Topic the Note is filed there; Ctrl+Enter takes 1 h.
    harness.get_by_label("Kitchen").click();
    harness.run();
    field(&harness, "What's on your mind?").click();
    harness.run();
    typing(&harness).type_text("  Buy descaler ");
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    let descaler = find(harness.state(), "Buy descaler")?;
    assert_eq!(descaler.place, Some(place(harness.state(), "Kitchen")?));
    assert_eq!(
        nudge(&harness, &descaler.id)?,
        Some(dart_iso(now + Duration::hours(1)))
    );
    assert!(card(&harness, "Buy descaler").rect().width() > 0.0);
    Ok(())
}

#[test]
fn a_notes_text_is_fixed_in_place() -> TestResult {
    let mut harness = harness("notes", "en")?;
    card(&harness, "Needs more sauce").click();
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
    card(&harness, "Buy more sauce").click();
    harness.run();
    assert_eq!(text(&harness)?, "Sauce, lots");
    let before = harness.state().store().all_entries()?.len();
    card(&harness, "Sauce, lots").click();
    harness.run();
    holding(&harness, "Sauce, lots").click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    card(&harness, "Buy more sauce").click();
    harness.run();
    assert_eq!(text(&harness)?, "Sauce, lots");
    assert_eq!(harness.state().store().all_entries()?.len(), before);
    Ok(())
}

#[test]
fn a_note_is_amended_weighed_trashed_restored_and_merged() -> TestResult {
    let mut harness = harness("places", "en")?;
    harness.get_by_label("Kitchen").click();
    harness.run();
    card(&harness, "Descale the kettle").click();
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

    harness.get_by_label("Move to trash").click();
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
    card(&harness, "Still Unsorted").click();
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
    card(&harness, "Still Unsorted").click();
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
    card(&harness, "Still Unsorted").click();
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
