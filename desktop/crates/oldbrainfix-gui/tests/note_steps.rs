//! Undo, Redo and "Revert all changes" of the Note open (obf-1051),
//! driven headless by egui_kittest: in the bar of the Note dialog, step
//! by step, for its text, typing taken as a step once it rests, a
//! table's menu, its Lines and its badges; Ctrl+Z, Ctrl+Shift+Z and
//! Ctrl+Y outside a text field; "Revert all changes" in its ⋮ after one
//! question, whose changes Redo makes again one by one; the same bar in a
//! Note's own window.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{
    Checklists, ClockTime, Day, Importance, Notes, Occurrence, OwnProfile, Places, Routines,
    Schedule, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a microsecond per read.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([4; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

/// The window over `store` in English listing All Notes, the Note
/// `note` open in its dialog.
fn window(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    app.select_note(note);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
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

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Clicks the last widget labelled `label`: a menu's or a dialog's, over
/// the window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Whether the bar's button `label` can be clicked.
fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    !harness
        .get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .is_disabled()
}

fn typed(harness: &mut Harness<'_, App>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn text_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(id)?.ok_or("the Note")?.text)
}

/// What the Note's field shows.
fn field(harness: &Harness<'_, App>) -> String {
    harness
        .get_by_label("Note text")
        .value()
        .unwrap_or_default()
}

/// The texts of the Lines of `note`, a Line of a group after "  ".
fn lines(store: &Store, note: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Checklists::new(store)
        .by_note(note)?
        .ok_or("a Checklist")?
        .lines
        .iter()
        .map(|l| format!("{}{}", if l.in_group() { "  " } else { "" }, l.text))
        .collect())
}

/// Puts the cursor at the end of the Note's text.
fn into_text(harness: &mut Harness<'_, App>) {
    harness.get_by_label("Note text").click();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
}

/// Lets `seconds` of kittest's time pass, a quarter of a second a step.
fn rest(harness: &mut Harness<'_, App>, seconds: f64) {
    let steps = (seconds / 0.25).ceil() as usize + 1;
    for _ in 0..steps {
        harness.step();
    }
    harness.run();
}

#[test]
fn a_saved_text_is_one_step_that_undo_takes_back_and_redo_makes_again() -> TestResult {
    let store = store()?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    let mut harness = window(&store, &milk);
    // Nothing changed yet: Undo and Redo greyed.
    assert!(!enabled(&harness, "Undo"));
    assert!(!enabled(&harness, "Redo"));
    into_text(&mut harness);
    typed(&mut harness, " and bread");
    click(&mut harness, "Save")?;
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    assert!(enabled(&harness, "Undo"));
    click(&mut harness, "Undo")?;
    // Written back as an edit, the field showing it.
    assert_eq!(text_of(&store, &milk)?, "Milk");
    assert_eq!(field(&harness), "Milk");
    assert_eq!(harness.state().note_steps(), (false, true, false));
    assert!(!enabled(&harness, "Undo"));
    click(&mut harness, "Redo")?;
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    assert_eq!(harness.state().note_steps(), (true, false, true));
    // Undo while typing keeps what is typed first, then takes it back.
    into_text(&mut harness);
    typed(&mut harness, " now");
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    assert_eq!(field(&harness), "Milk and bread");
    assert_eq!(harness.state().error(), None);
    Ok(())
}

#[test]
fn typing_that_rests_a_second_is_a_step_of_its_own() -> TestResult {
    let store = store()?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    let mut harness = window(&store, &milk);
    into_text(&mut harness);
    typed(&mut harness, " and");
    rest(&mut harness, 1.5);
    typed(&mut harness, " bread");
    rest(&mut harness, 1.5);
    // Nothing saved yet, two steps waiting.
    assert_eq!(text_of(&store, &milk)?, "Milk");
    assert_eq!(harness.state().note_steps(), (true, false, true));
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &milk)?, "Milk and");
    assert_eq!(field(&harness), "Milk and");
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &milk)?, "Milk");
    assert!(!enabled(&harness, "Undo"));
    // Typed on without a rest, it is one step.
    into_text(&mut harness);
    typed(&mut harness, " a");
    typed(&mut harness, "nd eggs");
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &milk)?, "Milk");
    Ok(())
}

#[test]
fn a_tables_menu_is_a_step_of_its_own_after_the_cell_typed_before_it() -> TestResult {
    let store = store()?;
    let table = "| Item | Qty |\n| --- | --- |\n| milk | 2 |\n| eggs | 6 |";
    let note = Notes::new(&store)
        .capture_as(&format!("{table}\n\nend"), MARKDOWN, None, None)?
        .id;
    let mut harness = window(&store, &note);
    // The dialog's, over the card's.
    let last = |harness: &mut Harness<'_, App>, button: &str| -> TestResult {
        harness
            .query_all_by_role_and_label(Role::Button, button)
            .last()
            .ok_or(button.to_string())?
            .click_accesskit();
        harness.run();
        Ok(())
    };
    last(&mut harness, "Open the table")?;
    harness
        .query_all_by_role(Role::Button)
        .rfind(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.starts_with("Row 2, column A"))
        })
        .ok_or("milk's cell")?
        .click_accesskit();
    harness.run();
    // Moved at once, within the second typing rests for.
    harness.event(egui::Event::Text(" and cream".into()));
    harness.step();
    harness
        .query_all_by_role_and_label(Role::Button, "Row 3")
        .last()
        .ok_or("Row 3")?
        .click_accesskit();
    harness.step();
    harness
        .query_all_by_label("Move up")
        .last()
        .ok_or("Move up")?
        .click_accesskit();
    harness.run();
    let typed = "| Item | Qty |\n| --- | --- |\n| milk and cream | 2 |\n| eggs | 6 |";
    let moved = "| Item | Qty |\n| --- | --- |\n| eggs | 6 |\n| milk and cream | 2 |";
    assert!(field(&harness).starts_with(moved), "{:?}", field(&harness));
    // Not saved yet; Undo takes the move back alone, the cell typed
    // before it stays.
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &note)?, format!("{typed}\n\nend"));
    assert!(field(&harness).starts_with(typed), "{:?}", field(&harness));
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &note)?, format!("{table}\n\nend"));
    click(&mut harness, "Redo")?;
    click(&mut harness, "Redo")?;
    assert_eq!(text_of(&store, &note)?, format!("{moved}\n\nend"));
    Ok(())
}

#[test]
fn ctrl_z_and_ctrl_y_outside_a_field_take_back_a_badge_and_make_it_again() -> TestResult {
    let store = store()?;
    let tap = Notes::new(&store).capture("Fix the tap", None, None)?.id;
    let mut harness = window(&store, &tap);
    let importance = |h: &Harness<'_, App>| {
        Notes::new(h.state().store())
            .by_id(&tap)
            .ok()
            .flatten()
            .map(|n| n.importance)
    };
    click(&mut harness, "Importance: Normal")?;
    harness
        .get_by_role_and_label(Role::RadioButton, "Important")
        .click_accesskit();
    harness.run();
    assert_eq!(importance(&harness), Some(Importance::Important));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(importance(&harness), Some(Importance::Normal));
    assert!(shows(&harness, "Importance: Normal"));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Y);
    harness.run();
    assert_eq!(importance(&harness), Some(Importance::Important));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    harness.run();
    assert_eq!(importance(&harness), Some(Importance::Important));
    // In a text field Ctrl+Z is the field's: the badge stays.
    into_text(&mut harness);
    typed(&mut harness, " today");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(importance(&harness), Some(Importance::Important));
    assert_eq!(field(&harness), "Fix the tap");
    Ok(())
}

#[test]
fn revert_all_changes_asks_once_and_redo_makes_the_changes_again() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store)
        .capture("Packing\nPassport\nTicket\nCharger", None, None)?
        .id;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, &note);
    // Nothing to go back from yet.
    click(&mut harness, "More for this Note")?;
    assert!(
        harness
            .get_by_label("Revert all changes")
            .accesskit_node()
            .is_disabled()
    );
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().note_open());
    // A Line deleted, one moved, the Importance changed.
    click(&mut harness, "More: Ticket")?;
    click(&mut harness, "Delete")?;
    click(&mut harness, "More: Charger")?;
    click(&mut harness, "Move up Alt+Up")?;
    assert_eq!(lines(&store, &note)?, ["Charger", "Passport"]);
    click(&mut harness, "Importance: Normal")?;
    harness
        .get_by_role_and_label(Role::RadioButton, "Unimportant")
        .click_accesskit();
    harness.run();
    // Undo takes back the last of them alone.
    click(&mut harness, "Undo")?;
    let importance = Notes::new(&store)
        .by_id(&note)?
        .ok_or("Packing")?
        .importance;
    assert_eq!(importance, Importance::Normal);
    assert_eq!(lines(&store, &note)?, ["Charger", "Passport"]);
    click(&mut harness, "Undo")?;
    assert_eq!(lines(&store, &note)?, ["Passport", "Charger"]);
    click(&mut harness, "Redo")?;
    assert_eq!(lines(&store, &note)?, ["Charger", "Passport"]);
    // Revert asks once; Cancel changes nothing.
    click(&mut harness, "More for this Note")?;
    click(&mut harness, "Revert all changes")?;
    assert!(shows(&harness, "Revert all changes?"));
    click(&mut harness, "Cancel")?;
    assert_eq!(lines(&store, &note)?, ["Charger", "Passport"]);
    click(&mut harness, "More for this Note")?;
    click(&mut harness, "Revert all changes")?;
    click(&mut harness, "Revert")?;
    assert!(!shows(&harness, "Revert all changes?"));
    assert_eq!(lines(&store, &note)?, ["Passport", "Ticket", "Charger"]);
    assert_eq!(harness.state().note_steps(), (false, true, false));
    assert_eq!(harness.state().error(), None);
    // Redo brings the changes back one by one, the one taken back before
    // the revert last.
    click(&mut harness, "Redo")?;
    assert_eq!(lines(&store, &note)?, ["Passport", "Charger"]);
    click(&mut harness, "Redo")?;
    assert_eq!(lines(&store, &note)?, ["Charger", "Passport"]);
    click(&mut harness, "Redo")?;
    let importance = Notes::new(&store)
        .by_id(&note)?
        .ok_or("Packing")?
        .importance;
    assert_eq!(importance, Importance::Unimportant);
    assert!(!enabled(&harness, "Redo"));
    Ok(())
}

#[test]
fn the_steps_start_anew_each_time_the_note_opens() -> TestResult {
    let store = store()?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    let mut harness = window(&store, &milk);
    into_text(&mut harness);
    typed(&mut harness, " and bread");
    click(&mut harness, "Save")?;
    click(&mut harness, "Close")?;
    harness.state_mut().select_note(&milk);
    harness.run();
    assert_eq!(harness.state().note_steps(), (false, false, false));
    // Over the list, without a Note open, Ctrl+Y does nothing.
    click(&mut harness, "Close")?;
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Y);
    harness.run();
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    Ok(())
}

#[test]
fn a_notes_own_window_has_the_same_undo() -> TestResult {
    let store = store()?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness.state_mut().open_in_window(&milk);
    harness.run();
    into_text(&mut harness);
    typed(&mut harness, " and bread");
    click(&mut harness, "Save")?;
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    click(&mut harness, "Undo")?;
    assert_eq!(text_of(&store, &milk)?, "Milk");
    Ok(())
}

#[test]
fn a_sections_text_and_the_area_badge_are_steps() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let household = Places::new(&store).add_area("Household")?.id;
    let tap = notes.capture("Fix the tap", None, None)?.id;
    let washer = notes.amend(&tap, "Washer bought")?.id;
    let mut harness = window(&store, &tap);
    let section = |store: &Store| -> Result<String, Box<dyn std::error::Error>> {
        Ok(Notes::new(store)
            .section_by_id(&washer)?
            .ok_or("the Section")?
            .text)
    };
    harness.get_by_label("Section text").click();
    harness.run();
    harness.key_press(Key::End);
    typed(&mut harness, ", size 3/4");
    click(&mut harness, "Save")?;
    assert_eq!(section(&store)?, "Washer bought, size 3/4");
    click(&mut harness, "Area and Topic: Unsorted")?;
    click(&mut harness, "Household")?;
    let place = |store: &Store| {
        Notes::new(store)
            .by_id(&tap)
            .ok()
            .flatten()
            .and_then(|n| n.place)
    };
    assert_eq!(place(&store), Some(household.clone()));
    // Filed back into Unsorted, then the Section's text back.
    click(&mut harness, "Undo")?;
    assert_eq!(place(&store), None);
    assert!(shows(&harness, "Area and Topic: Unsorted"));
    click(&mut harness, "Undo")?;
    assert_eq!(section(&store)?, "Washer bought");
    assert!(!enabled(&harness, "Undo"));
    Ok(())
}

#[test]
fn a_kind_made_and_taken_back_brings_its_lines_back_on_redo() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store)
        .capture("Packing\nPassport\nTicket", None, None)?
        .id;
    let mut harness = window(&store, &note);
    click(&mut harness, "Kind: Plain Note")?;
    harness
        .get_by_role_and_label(Role::RadioButton, "Checklist")
        .click_accesskit();
    harness.run();
    let made = lines(&store, &note)?;
    assert!(made.contains(&"Ticket".to_string()), "{made:?}");
    click(&mut harness, "More: Ticket")?;
    click(&mut harness, "Delete")?;
    let deleted = lines(&store, &note)?;
    assert!(!deleted.contains(&"Ticket".to_string()));
    click(&mut harness, "Undo")?;
    assert_eq!(lines(&store, &note)?, made);
    // Back to a plain Note with the text it had.
    click(&mut harness, "Undo")?;
    assert!(Checklists::new(&store).by_note(&note)?.is_none());
    assert_eq!(text_of(&store, &note)?, "Packing\nPassport\nTicket");
    click(&mut harness, "Redo")?;
    click(&mut harness, "Redo")?;
    assert_eq!(lines(&store, &note)?, deleted);
    assert_eq!(harness.state().error(), None);
    Ok(())
}

#[test]
fn ctrl_z_takes_back_a_check_off_made_after_the_notes_last_step_first() -> TestResult {
    // Made at 07:00, due at 08:00, opened at 09:00.
    let start = parse_time("2026-10-26T07:00:00.000Z")?.timestamp_millis();
    let now = Arc::new(AtomicI64::new(start));
    let held = now.clone();
    let clock = move || DateTime::from_timestamp_millis(held.fetch_add(1, Ordering::Relaxed));
    let store = Store::in_memory(Box::new(move || clock().unwrap_or_default()), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    let store = Arc::new(store);
    let note = Notes::new(&store).capture("Call mum", None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let monday = Day::new(2026, 10, 26);
    let schedule = Schedule::new(monday, &times, None).ok_or("a schedule")?;
    let id = Routines::new(&store).spawn(&note, &schedule)?.id;
    let opened = parse_time("2026-10-26T09:00:00.000Z")?.timestamp_millis();
    now.store(opened, Ordering::Relaxed);
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(|at: DateTime<Utc>| -> NaiveDateTime {
        at.naive_utc()
    }));
    app.select(Selection::All);
    app.select_note(&note);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    click(&mut harness, "Importance: Normal")?;
    harness
        .get_by_role_and_label(Role::RadioButton, "Important")
        .click_accesskit();
    harness.run();
    harness
        .get_all_by_role_and_label(Role::Button, "Done")
        .last()
        .ok_or("Done")?
        .click_accesskit();
    harness.run();
    let occurrence = Occurrence::new(monday, ClockTime::new(8, 0));
    let done = |store: &Store| {
        Routines::new(store)
            .by_id(&id)
            .ok()
            .flatten()
            .is_some_and(|r| r.done_at(&occurrence).is_some())
    };
    assert!(done(&store));
    let importance = |store: &Store| {
        Notes::new(store)
            .by_id(&note)
            .ok()
            .flatten()
            .map(|n| n.importance)
    };
    // The check-off came last: it goes first, then the Importance.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(!done(&store));
    assert_eq!(importance(&store), Some(Importance::Important));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(importance(&store), Some(Importance::Normal));
    Ok(())
}

#[test]
fn a_line_checked_and_a_shopping_line_deleted_are_taken_back() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store)
        .capture("Packing\nPassport\nTicket", None, None)?
        .id;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, &note);
    let checked = |store: &Store, text: &str| {
        Checklists::new(store)
            .by_note(&note)
            .ok()
            .flatten()
            .and_then(|l| l.lines.into_iter().find(|l| l.text == text))
            .is_some_and(|l| l.checked)
    };
    harness
        .query_all_by_role_and_label(Role::CheckBox, "Ticket")
        .last()
        .ok_or("Ticket's box")?
        .click_accesskit();
    harness.run();
    harness.run();
    assert!(checked(&store, "Ticket"));
    click(&mut harness, "Undo")?;
    assert!(!checked(&store, "Ticket"));
    click(&mut harness, "Redo")?;
    assert!(checked(&store, "Ticket"));

    // A Shopping List's Line deleted.
    let list = Notes::new(&store)
        .capture("Shop\nMilk\nBread", None, None)?
        .id;
    oldbrainfix_core::ShoppingLists::new(&store).make(&list)?;
    let mut harness = window(&store, &list);
    let texts = |store: &Store| -> Vec<String> {
        oldbrainfix_core::ShoppingLists::new(store)
            .by_note(&list, &|at| at.naive_utc())
            .ok()
            .flatten()
            .map(|l| l.lines.into_iter().map(|l| l.text).collect())
            .unwrap_or_default()
    };
    let before = texts(&store);
    assert!(before.contains(&"Bread".to_string()), "{before:?}");
    click(&mut harness, "More: Bread")?;
    click(&mut harness, "Delete")?;
    assert!(!texts(&store).contains(&"Bread".to_string()));
    click(&mut harness, "Undo")?;
    assert_eq!(texts(&store), before);
    assert_eq!(harness.state().error(), None);
    Ok(())
}
