//! The text's own size driven headless (#326, obf-1019): normal, larger
//! and largest, in View ▸ Text size, in Settings ▸ This computer and by
//! Ctrl+Plus and Ctrl+Minus, kept at the ends and across a restart; the
//! text of Notes and the Agenda's rows grow, buttons and boxes keep their
//! size, egui's zoom stays that of "Bigger text and icons", and the
//! smallest window holds it all at largest with Bigger text and icons on.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::schedule::Recurrence;
use oldbrainfix_core::{
    Checklists, ClockTime, Day, DeviceSettings, Notes, Routines, Schedule, SigningKey, Store,
    TextSize,
};
use oldbrainfix_gui::app::local_wall;
use oldbrainfix_gui::window_sizes::MIN_WINDOW;
use oldbrainfix_gui::{App, Selection, theme};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(Utc::now), || Ok(SigningKey::from_seed([7; 32])))?;
    Ok(Arc::new(store))
}

fn open(path: &Path) -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    Ok(Arc::new(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?))
}

/// The window over `store` at `size`, showing `selection`.
fn window(store: &Arc<Store>, selection: Selection, size: egui::Vec2) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(selection);
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        |ui, app: &mut App| {
            App::install(ui.ctx());
            app.show(ui);
        },
        app,
    );
    harness.run();
    harness
}

fn big(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    window(store, selection, egui::vec2(1280.0, 1400.0))
}

/// Every day at 09:00 since three days ago.
fn daily() -> Result<Schedule, Box<dyn std::error::Error>> {
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let at = [ClockTime::new(9, 0).ok_or("a time")?];
    let today = Day::today(Utc::now(), &local_wall);
    Ok(Schedule::new(today.add_days(-3), &at, Some(rule)).ok_or("a schedule")?)
}

/// The narrowest node labelled `label`, but a box: the words themselves,
/// not the card or row around them, whose width grows with their size.
fn words(harness: &Harness<'_, App>, label: &str) -> egui::Rect {
    harness
        .query_all_by_label(label)
        .filter(|n| n.accesskit_node().role() != Role::CheckBox)
        .map(|n| n.rect())
        .min_by(|a, b| a.width().total_cmp(&b.width()))
        .unwrap_or_else(|| panic!("no {label}"))
}

/// Opens View ▸ Text size.
fn open_text_size(harness: &mut Harness<'_, App>) {
    match harness.get_all_by_label("View").next() {
        Some(view) => view.click(),
        None => panic!("no menu View"),
    }
    harness.run();
    match harness.get_all_by_label("Text size ⏵").last() {
        Some(entry) => entry.hover(),
        None => panic!("no Text size"),
    }
    harness.run();
}

fn press(harness: &mut Harness<'_, App>, key: Key) {
    harness.key_press_modifiers(Modifiers::COMMAND, key);
    harness.run();
}

fn kept(store: &Store) -> TextSize {
    DeviceSettings::new(store).text_size().unwrap()
}

#[test]
fn ctrl_plus_and_ctrl_minus_step_the_text_kept_at_the_ends_and_across_a_restart() -> TestResult {
    let db = tempfile::tempdir()?;
    let path = db.path().join("oldbrainfix.db");
    let store = open(&path)?;
    let mut harness = big(&store, Selection::Agenda);
    assert_eq!(theme::text_factor(&harness.ctx), 1.0);
    assert_eq!(kept(&store), TextSize::Normal);

    press(&mut harness, Key::Plus);
    assert_eq!(kept(&store), TextSize::Larger);
    assert_eq!(theme::text_factor(&harness.ctx), 1.15);
    // Ctrl+= as Ctrl+Plus without Shift.
    press(&mut harness, Key::Equals);
    assert_eq!(kept(&store), TextSize::Largest);
    press(&mut harness, Key::Plus);
    assert_eq!(kept(&store), TextSize::Largest, "kept at the end");
    assert_eq!(theme::text_factor(&harness.ctx), 1.3);
    // egui's own zoom keys are off: the zoom stays Bigger text and icons'.
    assert_eq!(harness.ctx.zoom_factor(), 1.0);

    for _ in 0..3 {
        press(&mut harness, Key::Minus);
    }
    assert_eq!(kept(&store), TextSize::Normal, "kept at the end");
    assert_eq!(theme::text_factor(&harness.ctx), 1.0);
    assert_eq!(harness.ctx.zoom_factor(), 1.0);

    press(&mut harness, Key::Plus);
    press(&mut harness, Key::Plus);
    drop(harness);
    drop(store);
    let store = open(&path)?;
    let harness = big(&store, Selection::Agenda);
    assert_eq!(kept(&store), TextSize::Largest);
    assert_eq!(theme::text_factor(&harness.ctx), 1.3);
    Ok(())
}

#[test]
fn view_text_size_picks_a_step_and_greys_the_way_past_the_end() -> TestResult {
    let store = store()?;
    let mut harness = big(&store, Selection::Agenda);
    open_text_size(&mut harness);
    let smaller = harness.get_by_role_and_label(Role::Button, "Smaller text Ctrl+Minus");
    assert!(
        smaller.accesskit_node().is_disabled(),
        "normal is the smallest"
    );
    harness.get_by_label("Largest").click();
    harness.run();
    assert_eq!(kept(&store), TextSize::Largest);
    assert_eq!(theme::text_factor(&harness.ctx), 1.3);

    open_text_size(&mut harness);
    let larger = harness.get_by_role_and_label(Role::Button, "Larger text Ctrl+Plus");
    assert!(
        larger.accesskit_node().is_disabled(),
        "largest is the largest"
    );
    harness
        .get_by_role_and_label(Role::Button, "Smaller text Ctrl+Minus")
        .click();
    harness.run();
    assert_eq!(kept(&store), TextSize::Larger);
    Ok(())
}

#[test]
fn the_settings_pick_a_step_beside_bigger_text_and_icons() -> TestResult {
    let store = store()?;
    let mut harness = big(&store, Selection::Agenda);
    harness.get_by_label("Edit").click();
    harness.run();
    harness.get_by_label("Settings… Ctrl+,").click();
    harness.run();
    assert!(harness.query_by_label("Bigger text and icons").is_some());
    harness
        .get_by_role_and_label(Role::RadioButton, "Larger")
        .click();
    harness.run();
    assert_eq!(kept(&store), TextSize::Larger);
    assert!(
        !DeviceSettings::new(&store).fat_fingers()?,
        "a setting of its own"
    );
    assert_eq!(harness.ctx.zoom_factor(), 1.0);
    Ok(())
}

#[test]
fn a_notes_text_and_the_agendas_rows_grow_while_buttons_and_boxes_keep_their_size() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture("Buy milk", None, None)?;
    Routines::new(&store).create("Take pills", &daily()?)?;
    let measure = |size: TextSize| -> Result<_, Box<dyn std::error::Error>> {
        DeviceSettings::new(&store).set_text_size(size)?;
        let cards = big(&store, Selection::All);
        let note = words(&cards, "Buy milk");
        let menu = cards.get_by_label("View").rect();
        let agenda = big(&store, Selection::Agenda);
        let row = words(&agenda, "Take pills");
        let tick = agenda
            .query_all_by_role(Role::CheckBox)
            .next()
            .map(|n| n.rect())
            .ok_or("a box")?;
        Ok((note.width(), menu.height(), row.width(), tick.size()))
    };
    let (note, menu, row, tick) = measure(TextSize::Normal)?;
    for size in [TextSize::Larger, TextSize::Largest] {
        let (grown_note, grown_menu, grown_row, grown_tick) = measure(size)?;
        let factor = size.factor();
        assert!(
            (grown_note / note - factor).abs() < 0.08,
            "{size:?}: a Note's text {note} → {grown_note}"
        );
        assert!(
            (grown_row / row - factor).abs() < 0.08,
            "{size:?}: a row's words {row} → {grown_row}"
        );
        assert_eq!(grown_menu, menu, "{size:?}: the menu bar keeps its size");
        assert_eq!(grown_tick, tick, "{size:?}: a box keeps its size");
    }
    Ok(())
}

#[test]
fn the_smallest_window_holds_the_largest_text_with_bigger_text_and_icons() -> TestResult {
    let store = store()?;
    DeviceSettings::new(&store).set_fat_fingers(true)?;
    let long = "Call the plumber about the dripping kitchen tap before the weekend";
    Notes::new(&store).capture(long, None, None)?;
    let pills = "Take the evening pills with water";
    Routines::new(&store).create(pills, &daily()?)?;
    // Where the words end at normal, Bigger text and icons on; largest
    // keeps them in the window, wrapped or cut off.
    let ends = |size: TextSize| -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        DeviceSettings::new(&store).set_text_size(size)?;
        let mut ends = Vec::new();
        for selection in [Selection::All, Selection::Agenda] {
            let harness = window(&store, selection, MIN_WINDOW);
            assert_eq!(harness.ctx.zoom_factor(), 1.3);
            for label in [long, pills] {
                if let Some(end) = harness
                    .query_all_by_label(label)
                    .map(|n| n.rect().right())
                    .reduce(f32::max)
                {
                    ends.push(end);
                }
            }
        }
        Ok(ends)
    };
    let normal = ends(TextSize::Normal)?;
    let largest = ends(TextSize::Largest)?;
    // Both in All Notes, the Routine's on the Agenda.
    assert_eq!(normal.len(), 3, "the texts on their screens");
    assert_eq!(normal.len(), largest.len());
    for (at_normal, at_largest) in normal.iter().zip(&largest) {
        assert!(
            *at_largest <= at_normal + 0.5,
            "largest reaches {at_largest}, normal {at_normal}"
        );
    }
    Ok(())
}

#[test]
fn ctrl_plus_grows_the_text_shown_at_once_and_rests_over_the_settings() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture("Buy milk", None, None)?;
    let trip = Notes::new(&store).capture("Trip\nPassport\nTickets", None, None)?;
    Checklists::new(&store).make(&trip.id)?;
    let mut harness = big(&store, Selection::All);
    let note = words(&harness, "Buy milk").width();
    let line = words(&harness, "Passport").width();
    press(&mut harness, Key::Plus);
    let grown = words(&harness, "Buy milk").width();
    assert!(
        (grown / note - 1.15).abs() < 0.08,
        "a Note's text {note} → {grown}"
    );
    let grown = words(&harness, "Passport").width();
    assert!(
        (grown / line - 1.15).abs() < 0.08,
        "a Line {line} → {grown}"
    );

    // Over the Settings dialog they rest, as every key of the menu bar
    // does while another dialog is open. The menu bar's Edit comes before
    // the cards'.
    match harness.get_all_by_label("Edit").next() {
        Some(edit) => edit.click(),
        None => panic!("no menu Edit"),
    }
    harness.run();
    harness.get_by_label("Settings… Ctrl+,").click();
    harness.run();
    press(&mut harness, Key::Plus);
    assert_eq!(kept(&store), TextSize::Larger);
    assert_eq!(harness.ctx.zoom_factor(), 1.0);
    Ok(())
}

#[test]
fn quiet_today_goes_under_the_agendas_bar_where_the_bar_has_no_room_for_it() -> TestResult {
    let store = store()?;
    DeviceSettings::new(&store).set_fat_fingers(true)?;
    let pills = "Take the evening pills with water";
    Routines::new(&store).create(pills, &daily()?)?;
    DeviceSettings::new(&store).set_text_size(TextSize::Largest)?;
    let harness = window(&store, Selection::Agenda, MIN_WINDOW);
    let scale = harness.ctx.pixels_per_point();
    let edge = harness.ctx.content_rect().right() * scale;
    let quiet = harness.get_by_label("Quiet today").rect();
    let all = harness.get_by_label("All Routines and Reminders").rect();
    let more = harness
        .query_all_by_label(&format!("More for {pills}"))
        .map(|n| n.rect().right())
        .reduce(f32::max)
        .ok_or("a row's menu")?;
    assert!(
        quiet.right() <= edge,
        "Quiet today ends at {}",
        quiet.right()
    );
    assert!(more <= edge, "a row's menu ends at {more}");
    // On the line under the bar, at its right.
    assert!(quiet.top() >= all.bottom());
    Ok(())
}
