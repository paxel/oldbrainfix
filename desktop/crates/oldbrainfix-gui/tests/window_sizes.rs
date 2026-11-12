//! What the window keeps on this computer and where it was (#278): the
//! sidebar's width and the Note dialog's size come back from this
//! Device's settings, a drag on their borders keeps them, and back and
//! forward bring each place again with its search and how far its list
//! was scrolled.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use egui::{Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, TouchPhase};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, SigningKey, Store};
use oldbrainfix_gui::window_sizes::{NOTE_DIALOG_SIZE, SIDEBAR_WIDTH};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SIZE: egui::Vec2 = egui::vec2(1280.0, 640.0);

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([27; 32]))
    })?;
    Ok(Arc::new(store))
}

/// Forty Notes, "Errand 01" to "Errand 40", far more than show at once.
fn errands(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let mut ids = Vec::new();
    for n in 1..=40 {
        ids.push(
            notes
                .capture_as(&format!("Errand {n:02}"), "plain", None, None)?
                .id,
        );
    }
    Ok(ids)
}

fn harness(store: &Arc<Store>, place: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(place);
    let mut harness = Harness::builder().with_size(SIZE).build_ui_state(
        |ui, app: &mut App| {
            App::install(ui.ctx());
            app.show(ui);
        },
        app,
    );
    harness.run();
    harness
}

/// Presses the button at `from`, moves it by `by` in a few steps and
/// lets it go.
fn drag(harness: &mut Harness<'_, App>, from: Pos2, by: egui::Vec2) {
    let button = |harness: &mut Harness<'_, App>, pos, pressed| {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    };
    harness.event(Event::PointerMoved(from));
    harness.step();
    button(harness, from, true);
    for step in 1..=5 {
        harness.event(Event::PointerMoved(from + by * (step as f32 / 5.0)));
        harness.step();
    }
    button(harness, from + by, false);
    harness.run();
}

/// Scrolls the list under the pointer at `at` down by `by` points.
fn scroll(harness: &mut Harness<'_, App>, at: Pos2, by: f32) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -by),
        phase: TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    for _ in 0..30 {
        harness.step();
    }
    harness.run();
}

/// Where the Note dialog's right edge lies: its close button sits its
/// inner margin inside it.
fn dialog_right(harness: &Harness<'_, App>) -> f32 {
    harness
        .query_all_by_label("Close")
        .last()
        .expect("the Note dialog's close button")
        .rect()
        .right()
        + 16.0
}

#[test]
fn the_sidebar_opens_as_wide_as_it_was_left() -> TestResult {
    let store = store()?;
    // The first start's width, as drawn.
    let harness = harness(&store, Selection::All);
    assert_eq!(harness.state().sidebar_width(), 260.0);
    drop(harness);
    store.set_local_setting(SIDEBAR_WIDTH, "340")?;
    let harness = self::harness(&store, Selection::All);
    assert_eq!(harness.state().sidebar_width(), 340.0);
    Ok(())
}

#[test]
fn dragging_the_sidebar_border_keeps_its_width() -> TestResult {
    let store = store()?;
    let mut harness = harness(&store, Selection::All);
    // A click elsewhere keeps nothing.
    harness.get_by_label("Unsorted").click();
    harness.run();
    assert_eq!(store.local_setting(SIDEBAR_WIDTH)?, None);
    let border = harness.state().sidebar_width();
    drag(
        &mut harness,
        egui::pos2(border, 400.0),
        egui::vec2(60.0, 0.0),
    );
    // As far as egui's border followed the pointer.
    let width = harness.state().sidebar_width();
    assert!(width > border + 40.0, "{width}");
    let kept = format!("{}", width.round());
    assert_eq!(store.local_setting(SIDEBAR_WIDTH)?, Some(kept));
    drop(harness);
    let harness = self::harness(&store, Selection::All);
    assert!((harness.state().sidebar_width() - width).abs() <= 1.0);
    Ok(())
}

#[test]
fn the_note_dialog_opens_as_large_as_it_was_left() -> TestResult {
    let store = store()?;
    let id = errands(&store)?.remove(0);
    let mut harness = harness(&store, Selection::All);
    harness.state_mut().select_note(&id);
    harness.run();
    // The first start's: 760 wide at most, as before.
    let first = harness.state().note_dialog_size().ok_or("open")?;
    assert_eq!(first.x, 760.0);
    drop(harness);
    store.set_local_setting(NOTE_DIALOG_SIZE, "1000,420")?;
    let mut harness = self::harness(&store, Selection::All);
    harness.state_mut().select_note(&id);
    harness.run();
    assert_eq!(
        harness.state().note_dialog_size(),
        Some(egui::vec2(1000.0, 420.0))
    );
    // Wider than the window has room for: as wide as it has.
    drop(harness);
    store.set_local_setting(NOTE_DIALOG_SIZE, "4000,420")?;
    let mut harness = self::harness(&store, Selection::All);
    harness.state_mut().select_note(&id);
    harness.run();
    let size = harness.state().note_dialog_size().ok_or("open")?;
    assert!(size.x > 1100.0 && size.x <= SIZE.x - 48.0, "{size:?}");
    Ok(())
}

#[test]
fn dragging_the_note_dialog_edge_resizes_and_keeps_it() -> TestResult {
    let store = store()?;
    let id = errands(&store)?.remove(0);
    let mut harness = harness(&store, Selection::All);
    harness.state_mut().select_note(&id);
    harness.run();
    let before = harness.state().note_dialog_size().ok_or("open")?;
    let right = dialog_right(&harness);
    // Centred, it grows on both sides as far as the edge moved.
    drag(
        &mut harness,
        egui::pos2(right, 300.0),
        egui::vec2(50.0, 0.0),
    );
    assert!(harness.state().note_open(), "the drag closed nothing");
    let after = harness.state().note_dialog_size().ok_or("open")?;
    assert!((after.x - (before.x + 100.0)).abs() <= 1.0, "{after:?}");
    assert_eq!(after.y, before.y);
    assert!((dialog_right(&harness) - (right + 50.0)).abs() <= 1.0);
    let kept = store.local_setting(NOTE_DIALOG_SIZE)?.ok_or("kept")?;
    assert_eq!(kept, format!("{},{}", after.x.round(), after.y.round()));
    // And it opens so again.
    drop(harness);
    let mut harness = self::harness(&store, Selection::All);
    harness.state_mut().select_note(&id);
    harness.run();
    let again = harness.state().note_dialog_size().ok_or("open")?;
    assert!((again.x - after.x).abs() <= 1.0, "{again:?}");
    Ok(())
}

#[test]
fn back_and_forward_bring_the_search_and_the_scroll_again() -> TestResult {
    let store = store()?;
    errands(&store)?;
    let mut harness = harness(&store, Selection::All);
    harness.state_mut().set_query("Errand");
    harness.run();
    let list = egui::pos2(800.0, 400.0);
    scroll(&mut harness, list, 600.0);
    let scrolled = harness.state().list_scrolled();
    assert!(scrolled > 300.0, "{scrolled}");

    // Unsorted, a place new to the history: its own search, at the top.
    // The sidebar's row, ahead of the cards naming it.
    harness
        .get_all_by_label("Unsorted")
        .next()
        .ok_or("Unsorted")?
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    assert_eq!(harness.state().query(), "");
    assert_eq!(harness.state().list_scrolled(), 0.0);
    harness.state_mut().set_query("Errand 3");
    harness.run();
    scroll(&mut harness, list, 120.0);
    let unsorted = harness.state().list_scrolled();

    // Back: All Notes as it was left.
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "Errand");
    let back = harness.state().list_scrolled();
    assert!((back - scrolled).abs() <= 1.0, "{back} != {scrolled}");

    // Forward: Unsorted as it was left.
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    assert_eq!(harness.state().query(), "Errand 3");
    let forward = harness.state().list_scrolled();
    assert!((forward - unsorted).abs() <= 1.0, "{forward} != {unsorted}");

    // All Notes shown anew, from the sidebar: as a new place.
    harness
        .get_all_by_label("All Notes")
        .next()
        .ok_or("All Notes")?
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "");
    assert_eq!(harness.state().list_scrolled(), 0.0);

    // Each visit as it was left, the same place's earlier one too.
    scroll(&mut harness, list, 150.0);
    for _ in 0..2 {
        harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
        harness.run();
    }
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "Errand");
    let first = harness.state().list_scrolled();
    assert!((first - scrolled).abs() <= 1.0, "{first} != {scrolled}");
    Ok(())
}
