//! The keys over the desktop's card lists (#274): Up and Down move the
//! selection, Shift with them runs it on, Ctrl+A selects all, Enter
//! opens the Note, Delete moves the selection to the trash for Ctrl+Z to
//! bring back, 1 to 3 set the Importance, Ctrl+M files and Ctrl+Shift+M
//! merges; Down from the search goes into the cards found, and the card
//! reached comes into view. None of them acts while a field has the
//! keyboard. Unsorted and the trash have a search of their own, which
//! Ctrl+F reaches.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    ClockTime, Day, Importance, Note, Notes, OwnProfile, Places, Routines, Schedule, SigningKey,
    Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MILK: &str = "Buy milk";
const BANK: &str = "Call the bank";
const ROSES: &str = "Water the roses";

/// A store on a clock that moves a second per read, so the Notes are
/// captured one after another: the Unsorted Notes "Buy milk", "Call the
/// bank" and "Water the roses", Unsorted oldest first, and the Area
/// Household.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    Places::new(&store).add_area("Household")?;
    let notes = Notes::new(&store);
    for text in [MILK, BANK, ROSES] {
        notes.capture(text, None, None)?;
    }
    Ok(store)
}

fn harness(store: Store, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(selection);
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

fn press(harness: &mut Harness<'_, App>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn press_with(harness: &mut Harness<'_, App>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// Every Note, also those in the trash or merged, as the store has it
/// now.
fn notes(harness: &Harness<'_, App>) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    let store = harness.state().store();
    let notes = Notes::new(store);
    let mut all = Vec::new();
    for id in store.entities_of_type("note")? {
        all.extend(notes.by_id(&id)?);
    }
    Ok(all)
}

fn note(harness: &Harness<'_, App>, text: &str) -> Result<Note, Box<dyn std::error::Error>> {
    notes(harness)?
        .into_iter()
        .find(|n| n.text == text)
        .ok_or_else(|| format!("no Note {text:?}").into())
}

/// The texts of the cards selected, in the list's order.
fn selected(harness: &Harness<'_, App>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let all = notes(harness)?;
    Ok(harness
        .state()
        .selected_cards()
        .iter()
        .filter_map(|id| all.iter().find(|n| &n.id == id))
        .map(|n| n.text.clone())
        .collect())
}

/// The texts of the cards shown, in the list's order.
fn shown(harness: &Harness<'_, App>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let all = notes(harness)?;
    Ok(harness
        .state()
        .shown_notes()
        .iter()
        .filter_map(|id| all.iter().find(|n| &n.id == id))
        .map(|n| n.text.clone())
        .collect())
}

/// The field the keyboard is in, by its label or its hint.
fn focused(harness: &Harness<'_, App>) -> Option<String> {
    let node = harness.query_by(|n| n.is_focused())?.accesskit_node();
    node.label()
        .or_else(|| node.placeholder().map(str::to_string))
}

/// Types `text` where the keyboard is.
fn type_here(harness: &mut Harness<'_, App>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

#[test]
fn up_and_down_move_the_selection_and_shift_runs_it_on() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    assert_eq!(shown(&harness)?, [MILK, BANK, ROSES]);
    // Nothing selected: Down takes the first card, Up the last.
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [MILK]);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [BANK]);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    // It stays at the end.
    assert_eq!(selected(&harness)?, [ROSES]);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(selected(&harness)?, [BANK]);
    press(&mut harness, Key::Escape);
    assert!(selected(&harness)?.is_empty());
    press(&mut harness, Key::ArrowUp);
    assert_eq!(selected(&harness)?, [ROSES]);

    // Shift runs the selection on from the card it started at.
    press(&mut harness, Key::ArrowUp);
    press_with(&mut harness, Modifiers::SHIFT, Key::ArrowUp);
    assert_eq!(selected(&harness)?, [MILK, BANK]);
    press_with(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    press_with(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [BANK, ROSES]);
    // The selected cards say so to a screen reader.
    let roses = harness.get_by_role_and_label(Role::Button, ROSES);
    assert_eq!(roses.accesskit_node().toggled(), Some(Toggled::True));
    Ok(())
}

#[test]
fn a_click_is_where_the_keys_go_on_from() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    harness.run_steps(50);
    harness.get_by_role_and_label(Role::Button, BANK).click();
    harness.run();
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [ROSES]);
    // Enter opens the Note, not a click on a card.
    press(&mut harness, Key::Enter);
    assert_eq!(
        harness.state().selected_note(),
        Some(note(&harness, ROSES)?.id.as_str())
    );
    Ok(())
}

#[test]
fn ctrl_a_selects_every_card() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press_with(&mut harness, Modifiers::COMMAND, Key::A);
    assert_eq!(selected(&harness)?, [MILK, BANK, ROSES]);
    Ok(())
}

#[test]
fn enter_opens_the_note_reached_and_nothing_with_none() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press(&mut harness, Key::Enter);
    assert!(!harness.state().note_open());
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::ArrowDown);
    press(&mut harness, Key::Enter);
    assert!(harness.state().note_open());
    assert_eq!(
        harness.state().selected_note(),
        Some(note(&harness, BANK)?.id.as_str())
    );
    // Over the Note dialog the list's keys rest.
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [BANK]);
    Ok(())
}

#[test]
fn delete_moves_the_selection_to_the_trash_and_ctrl_z_brings_it_back() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press(&mut harness, Key::ArrowDown);
    press_with(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    press(&mut harness, Key::Delete);
    assert!(note(&harness, MILK)?.is_trashed());
    assert!(note(&harness, BANK)?.is_trashed());
    assert!(!note(&harness, ROSES)?.is_trashed());
    // Asked nothing.
    assert!(harness.query_by_label_contains("Delete this").is_none());
    assert_eq!(shown(&harness)?, [ROSES]);
    press_with(&mut harness, Modifiers::COMMAND, Key::Z);
    assert!(!note(&harness, MILK)?.is_trashed());
    assert!(!note(&harness, BANK)?.is_trashed());
    Ok(())
}

#[test]
fn in_the_trash_delete_and_the_digits_do_nothing() -> TestResult {
    let store = store()?;
    let routines = Routines::new(&store);
    for note in Notes::new(&store).all()? {
        routines.trash_note(&note.id)?;
    }
    let mut harness = harness(store, Selection::Trash);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?.len(), 1);
    let picked = selected(&harness)?.join("");
    press(&mut harness, Key::Delete);
    press(&mut harness, Key::Num1);
    let kept = note(&harness, &picked)?;
    assert!(kept.is_trashed());
    assert_eq!(kept.importance, Importance::Normal);
    Ok(())
}

#[test]
fn one_two_and_three_set_the_importance_of_the_selection() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press_with(&mut harness, Modifiers::COMMAND, Key::A);
    press(&mut harness, Key::Num1);
    for text in [MILK, BANK, ROSES] {
        assert_eq!(note(&harness, text)?.importance, Importance::Important);
    }
    // Set, it ends the selection, as a pick in a menu does; the keys go
    // on from the card reached last, the last of all.
    assert!(selected(&harness)?.is_empty());
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [ROSES]);
    press(&mut harness, Key::Num3);
    assert_eq!(note(&harness, ROSES)?.importance, Importance::Unimportant);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(selected(&harness)?, [BANK]);
    press(&mut harness, Key::Num2);
    assert_eq!(note(&harness, BANK)?.importance, Importance::Normal);
    assert_eq!(note(&harness, MILK)?.importance, Importance::Important);
    Ok(())
}

#[test]
fn in_a_field_the_list_keys_are_the_fields() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press(&mut harness, Key::ArrowDown);
    press_with(&mut harness, Modifiers::COMMAND, Key::F);
    assert_eq!(focused(&harness).as_deref(), Some("Search Unsorted"));
    press(&mut harness, Key::Delete);
    press(&mut harness, Key::Num1);
    press_with(&mut harness, Modifiers::COMMAND, Key::A);
    press(&mut harness, Key::Enter);
    assert!(!note(&harness, MILK)?.is_trashed());
    assert_eq!(note(&harness, MILK)?.importance, Importance::Normal);
    assert_eq!(selected(&harness)?, [MILK]);
    assert!(!harness.state().note_open());
    Ok(())
}

#[test]
fn a_widget_with_the_keyboard_keeps_the_keys() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press(&mut harness, Key::ArrowDown);
    // Tab puts the keyboard on a button.
    press(&mut harness, Key::Tab);
    assert!(harness.query_by(|n| n.is_focused()).is_some());
    press(&mut harness, Key::Delete);
    press(&mut harness, Key::Num1);
    press(&mut harness, Key::ArrowDown);
    assert!(!note(&harness, MILK)?.is_trashed());
    assert_eq!(note(&harness, MILK)?.importance, Importance::Normal);
    assert_eq!(selected(&harness)?, [MILK]);
    Ok(())
}

#[test]
fn down_from_the_search_goes_into_the_cards_found() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    // A card selected before: Down still starts at the first found.
    press(&mut harness, Key::ArrowUp);
    press_with(&mut harness, Modifiers::COMMAND, Key::F);
    type_here(&mut harness, "the");
    assert_eq!(shown(&harness)?, [BANK, ROSES]);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [BANK]);
    assert_eq!(focused(&harness), None);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(selected(&harness)?, [ROSES]);
    Ok(())
}

#[test]
fn unsorted_searches_itself_and_ctrl_f_stays_there() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press_with(&mut harness, Modifiers::COMMAND, Key::F);
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    assert_eq!(focused(&harness).as_deref(), Some("Search Unsorted"));
    type_here(&mut harness, "milk");
    assert_eq!(shown(&harness)?, [MILK]);
    // A Note filed elsewhere is not found in Unsorted.
    let household = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .next()
        .ok_or("Household")?;
    Notes::new(harness.state().store()).file(&note(&harness, MILK)?.id, &household.id)?;
    harness.state_mut().set_query("milk");
    harness.run();
    assert!(shown(&harness)?.is_empty());
    assert!(harness.query_by_label("No Note matches.").is_some());
    Ok(())
}

#[test]
fn the_trash_searches_itself_and_ctrl_f_stays_there() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let routines = Routines::new(&store);
    let live = notes.all()?;
    let of = |text: &str| live.iter().find(|n| n.text == text).ok_or(text.to_string());
    notes.amend(&of(MILK)?.id, "and eggs")?;
    routines.trash_note(&of(MILK)?.id)?;
    routines.trash_note(&of(BANK)?.id)?;
    // The roses' Note stays; only its Reminder is in the trash.
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let once = Schedule::new(Day::new(2026, 3, 2), &times, None).ok_or("a schedule")?;
    let reminder = routines.spawn(&of(ROSES)?.id, &once)?;
    routines.trash(&reminder.id)?;
    let mut harness = harness(store, Selection::Trash);
    assert_eq!(shown(&harness)?, [BANK, MILK]);
    assert!(harness.query_by_label(ROSES).is_some());
    press_with(&mut harness, Modifiers::COMMAND, Key::F);
    assert_eq!(harness.state().selection(), &Selection::Trash);
    assert_eq!(focused(&harness).as_deref(), Some("Search the Trash"));
    // Found by its Amend; the Reminder's Note does not hold it.
    type_here(&mut harness, "eggs");
    assert_eq!(shown(&harness)?, [MILK]);
    assert!(harness.query_by_label(ROSES).is_none());
    // The roses' Note is no card here, its Reminder is found by it.
    harness.state_mut().set_query("the");
    harness.run();
    assert_eq!(shown(&harness)?, [BANK]);
    assert!(harness.query_by_label(ROSES).is_some());
    // Only the Reminder found: its row, and no word of nothing found.
    harness.state_mut().set_query("roses");
    harness.run();
    assert!(shown(&harness)?.is_empty());
    assert!(harness.query_by_label(ROSES).is_some());
    assert!(harness.query_by_label("No Note matches.").is_none());
    harness.state_mut().set_query("tent");
    harness.run();
    assert!(shown(&harness)?.is_empty());
    assert!(harness.query_by_label(ROSES).is_none());
    assert!(harness.query_by_label("No Note matches.").is_some());
    assert!(harness.query_by_label("The trash is empty.").is_none());
    Ok(())
}

#[test]
fn sorting_unsorted_is_down_ctrl_m_type_enter_for_each_note() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    let household = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .next()
        .ok_or("Household")?
        .id;
    for (text, left) in [(MILK, 2), (BANK, 1)] {
        press(&mut harness, Key::ArrowDown);
        assert_eq!(selected(&harness)?, [text]);
        press_with(&mut harness, Modifiers::COMMAND, Key::M);
        harness.run_steps(8);
        type_here(&mut harness, "hou");
        press(&mut harness, Key::Enter);
        assert_eq!(
            note(&harness, text)?.place.as_deref(),
            Some(household.as_str())
        );
        assert_eq!(shown(&harness)?.len(), left);
    }
    assert_eq!(shown(&harness)?, [ROSES]);
    Ok(())
}

#[test]
fn ctrl_shift_m_merges_the_selection_and_files_nothing() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press(&mut harness, Key::ArrowDown);
    press_with(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::M);
    harness.run_steps(8);
    // Merge into…'s picker, not File into…'s.
    assert!(harness.query_by_label("Filter Areas and Topics").is_none());
    harness
        .query_all_by_label(ROSES)
        .next_back()
        .ok_or("the roses' Note to pick")?
        .click();
    harness.run();
    let roses = note(&harness, ROSES)?.id;
    assert_eq!(
        note(&harness, MILK)?.merged_into.as_deref(),
        Some(roses.as_str())
    );
    Ok(())
}

#[test]
fn the_card_reached_scrolls_into_view() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    for n in 0..30 {
        notes.capture(&format!("Errand {n}"), None, None)?;
    }
    let mut harness = harness(store, Selection::Unsorted);
    // Up from nothing: the last card, far below the window's end.
    press(&mut harness, Key::ArrowUp);
    harness.run_steps(30);
    let last = harness.get_by_role_and_label(Role::Button, "Errand 29");
    let bottom = last.rect().max.y;
    assert!(bottom <= 900.0, "the last card ends at {bottom}");
    Ok(())
}

#[test]
fn the_note_menu_shows_the_keys_beside_its_entries() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    press(&mut harness, Key::ArrowDown);
    harness
        .query_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    for entry in [
        "File into… Ctrl+M",
        "Merge into… Ctrl+Shift+M",
        "Delete Delete",
    ] {
        assert!(harness.query_by_label(entry).is_some(), "{entry}");
    }
    harness
        .query_all_by_label_contains("Importance")
        .next_back()
        .ok_or("Importance")?
        .click();
    harness.run();
    for entry in ["Important 1", "Normal 2", "Unimportant 3"] {
        assert!(harness.query_by_label(entry).is_some(), "{entry}");
    }
    Ok(())
}
