//! Search in the right-click menu (obf-1055, C9c): in the fields and the
//! texts only read of the Note open, in the Note dialog, a Note's own
//! window and its reading view, the menu has Search beside Copy and
//! Paste, greyed without a selection; it opens the Note's find bar with
//! the words selected. A text only read has no Paste, a field outside a
//! Note no Search. The find bar's Search all Notes searches All Notes
//! for its words, from a Note's own window in the main window.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::{MARKDOWN, PLAIN};
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SEARCH: &str = "Search";
const SEARCH_ALL: &str = "Search all Notes";

/// A store with the Notes `texts` in `format` captured into Unsorted, and
/// their ids.
fn store(texts: &[(&str, &str)]) -> Result<(Arc<Store>, Vec<String>), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let notes = Notes::new(&store);
    let mut ids = Vec::new();
    for (text, format) in texts {
        ids.push(notes.capture_as(text, format, None, None)?.id);
    }
    Ok((Arc::new(store), ids))
}

/// The main window on `selection`, `note` open in its dialog.
fn window(store: &Arc<Store>, selection: Selection, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(selection);
    if let Some(note) = note {
        app.select_note(note);
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
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

/// The Note `note` of `store` in a window of its own.
fn alone(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    let app = App::for_note_window(store.clone(), "en", note, true);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show_note_window(ui);
            },
            app,
        );
    harness.run();
    harness
}

/// Right-clicks the text field holding `text`.
fn right_click_field(harness: &mut Harness<'_, App>, text: &str) {
    let text = text.to_string();
    harness
        .query_all_by(move |n| {
            matches!(n.role(), Role::MultilineTextInput | Role::TextInput)
                && n.value().as_deref() == Some(text.as_str())
        })
        .next()
        .unwrap_or_else(|| panic!("no field"))
        .click_secondary();
    harness.run();
}

fn press(harness: &mut Harness<'_, App>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

/// Right-clicks the last label holding `text`, a text only read.
fn right_click_label(harness: &mut Harness<'_, App>, text: &str) {
    let at = harness
        .query_all_by_label_contains(text)
        .rfind(|n| n.accesskit_node().role() == Role::Label)
        .unwrap_or_else(|| panic!("no {text:?}"))
        .rect()
        .center();
    harness.event(Event::PointerMoved(at));
    press(harness, at, true);
    press(harness, at, false);
    harness.run();
}

/// How many entries are labelled `label`.
fn count(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .is_some_and(|n| !n.accesskit_node().is_disabled())
}

/// Picks the entry `label`, the last so named.
fn pick(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .unwrap_or_else(|| panic!("no entry {label:?}"))
        .click();
    // Select all on a text only read drags over it, a step a frame.
    harness.run_steps(4);
    harness.run();
}

/// What the widget with the keyboard is called, or its hint.
fn focused_label(harness: &Harness<'_, App>) -> Option<String> {
    let node = harness.query_by(|n| n.is_focused())?.accesskit_node();
    node.label()
        .or_else(|| node.placeholder().map(str::to_string))
}

#[test]
fn search_in_a_fields_menu_opens_the_find_bar_with_the_words_selected() -> TestResult {
    let (store, ids) = store(&[("oat milk\nand oat bread", PLAIN)])?;
    let mut harness = window(&store, Selection::Unsorted, Some(&ids[0]));
    right_click_field(&mut harness, "oat milk\nand oat bread");
    // Beside Copy and Paste, greyed with only the cursor placed.
    assert_eq!(count(&harness, SEARCH), 1);
    assert!(!enabled(&harness, SEARCH));
    assert!(enabled(&harness, "Paste"));
    pick(&mut harness, "Select all");
    right_click_field(&mut harness, "oat milk\nand oat bread");
    assert!(enabled(&harness, SEARCH));
    pick(&mut harness, SEARCH);
    // Its lines one line in the bar, which takes the keyboard.
    let found = harness
        .state()
        .find_bar()
        .map(|(q, _, n)| (q.to_string(), n));
    assert_eq!(found, Some(("oat milk and oat bread".to_string(), 0)));
    assert_eq!(
        focused_label(&harness).as_deref(),
        Some("Find in this Note")
    );
    assert_eq!(count(&harness, SEARCH), 0, "the menu closed");
    assert!(harness.state().note_open());
    Ok(())
}

#[test]
fn a_formatted_texts_words_go_into_the_bar_as_they_read() -> TestResult {
    let (store, ids) = store(&[("Buy **milk** now", MARKDOWN)])?;
    let mut harness = window(&store, Selection::Unsorted, Some(&ids[0]));
    // It reads formatted until Edit opens it for editing.
    harness
        .query_all_by_role_and_label(Role::Button, "Edit")
        .last()
        .ok_or("the text")?
        .click_accesskit();
    harness.run();
    right_click_field(&mut harness, "Buy **milk** now");
    pick(&mut harness, "Select all");
    right_click_field(&mut harness, "Buy **milk** now");
    pick(&mut harness, SEARCH);
    // Found where it shows, its marks found nowhere.
    let found = harness
        .state()
        .find_bar()
        .map(|(q, _, n)| (q.to_string(), n));
    assert_eq!(found, Some(("Buy milk now".to_string(), 1)));
    Ok(())
}

#[test]
fn a_field_outside_a_note_has_no_search() -> TestResult {
    let (store, _) = store(&[("oat milk", PLAIN)])?;
    let mut harness = window(&store, Selection::All, None);
    harness
        .get_by(|n| n.placeholder() == Some(SEARCH_ALL))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("oat");
    harness.run();
    right_click_field(&mut harness, "oat");
    assert!(count(&harness, "Paste") > 0);
    assert_eq!(count(&harness, SEARCH), 0);
    Ok(())
}

#[test]
fn a_text_only_read_offers_search_and_no_paste() -> TestResult {
    let (store, ids) = store(&[("Buy **milk**\n\nand *eggs* too", MARKDOWN)])?;
    Notes::new(&store).trash(&ids[0])?;
    let mut harness = window(&store, Selection::Trash, Some(&ids[0]));
    let pastes = count(&harness, "Paste");
    right_click_label(&mut harness, "Buy milk");
    assert!(count(&harness, "Copy") > 0, "the menu opened");
    assert_eq!(count(&harness, SEARCH), 1);
    assert!(!enabled(&harness, SEARCH));
    assert_eq!(count(&harness, "Paste"), pastes, "nothing to paste into");
    pick(&mut harness, "Select all");
    right_click_label(&mut harness, "Buy milk");
    assert!(enabled(&harness, SEARCH));
    pick(&mut harness, SEARCH);
    assert_eq!(
        harness.state().find_bar().map(|(q, _, _)| q),
        Some("Buy milk and eggs too")
    );
    assert!(harness.state().note_open());
    Ok(())
}

#[test]
fn the_history_over_the_note_offers_no_search() -> TestResult {
    let (store, ids) = store(&[("Dentist on Tuesday", PLAIN)])?;
    Notes::new(&store).edit(&ids[0], "Dentist on Wednesday")?;
    let mut harness = window(&store, Selection::Unsorted, Some(&ids[0]));
    harness
        .get_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    harness
        .get_all_by_label("History")
        .last()
        .ok_or("History")?
        .click();
    harness.run();
    right_click_label(&mut harness, "Text changed");
    assert!(count(&harness, "Select all") > 0, "the menu opened");
    assert_eq!(count(&harness, SEARCH), 0);
    Ok(())
}

#[test]
fn search_all_notes_in_the_bar_searches_all_notes_for_its_words() -> TestResult {
    let (store, ids) = store(&[("oat milk", PLAIN), ("milk tea", PLAIN)])?;
    let mut harness = window(&store, Selection::Unsorted, Some(&ids[0]));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    // Nothing typed, nothing to search.
    assert!(!enabled(&harness, SEARCH_ALL));
    harness.get_by(|n| n.is_focused()).type_text("milk ");
    harness.run();
    assert!(enabled(&harness, SEARCH_ALL));
    pick(&mut harness, SEARCH_ALL);
    harness.run();
    assert!(!harness.state().note_open());
    assert_eq!(harness.state().find_bar(), None);
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "milk");
    let mut shown = harness.state().shown_notes();
    shown.sort();
    let mut both = ids.clone();
    both.sort();
    assert_eq!(shown, both);
    Ok(())
}

#[test]
fn a_notes_own_window_and_its_reading_view_search_from_the_menu() -> TestResult {
    let (store, ids) = store(&[("Pancakes\n200 g flour, more flour", PLAIN)])?;
    let mut harness = alone(&store, &ids[0]);
    right_click_field(&mut harness, "Pancakes\n200 g flour, more flour");
    pick(&mut harness, "Select all");
    right_click_field(&mut harness, "Pancakes\n200 g flour, more flour");
    pick(&mut harness, SEARCH);
    assert_eq!(
        harness.state().find_bar().map(|(q, _, _)| q),
        Some("Pancakes 200 g flour, more flour")
    );
    // The reading view: its texts only read.
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    harness.key_press(Key::F9);
    harness.run();
    assert_eq!(harness.state().reading(), Some(true));
    right_click_label(&mut harness, "200 g flour");
    assert_eq!(count(&harness, SEARCH), 1);
    pick(&mut harness, "Select all");
    right_click_label(&mut harness, "200 g flour");
    pick(&mut harness, SEARCH);
    let words = harness.state().find_bar().map(|(q, _, _)| q.to_string());
    assert!(
        words.as_deref().is_some_and(|w| w.contains("200 g flour")),
        "{words:?}"
    );
    Ok(())
}

#[test]
fn search_all_notes_in_a_notes_own_window_searches_in_the_main_window() -> TestResult {
    let (store, ids) = store(&[("oat milk", PLAIN), ("milk tea", PLAIN)])?;
    let mut harness = window(&store, Selection::Unsorted, None);
    harness.run_steps(50);
    harness
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("oat milk"))
        })
        .next()
        .unwrap_or_else(|| panic!("no card"))
        .click_secondary();
    harness.run();
    harness
        .query_all_by_label("Open in its own window")
        .next_back()
        .unwrap_or_else(|| panic!("no Open in its own window"))
        .click_accesskit();
    harness.run_steps(8);
    assert_eq!(harness.state().note_windows(), [ids[0].as_str()]);
    right_click_field(&mut harness, "oat milk");
    pick(&mut harness, "Select all");
    right_click_field(&mut harness, "oat milk");
    pick(&mut harness, SEARCH);
    let bar = harness
        .state()
        .note_window(&ids[0])
        .and_then(|w| w.find_bar().map(|(q, _, n)| (q.to_string(), n)));
    assert_eq!(bar, Some(("oat milk".to_string(), 1)));
    pick(&mut harness, SEARCH_ALL);
    harness.run_steps(4);
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "oat milk");
    assert_eq!(harness.state().note_windows(), [ids[0].as_str()]);
    Ok(())
}
