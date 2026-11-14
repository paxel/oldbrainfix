//! A Note in a window of its own on the desktop (#287), driven headless
//! by egui_kittest, which draws a window of its own inside the main one:
//! "Open in its own window" in a card's menu and the Note dialog's bar
//! opens it, one window per Note, again only bringing it forward, the
//! Note dialog not opening a Note that has one; what was typed is kept
//! as it closes, also with the main window, and its size kept for the
//! next; a Note going to the trash closes its window, the one Ctrl+Z bringing it back; Keep on
//! top puts it over the other windows, greyed with the reason on
//! Wayland.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers, WindowLevel};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Notes, OwnProfile, Routines, SigningKey, Store};
use oldbrainfix_gui::window_sizes;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const OPEN: &str = "Open in its own window";
const KEEP_ON_TOP: &str = "Keep on top";

/// A store on a clock that moves a microsecond per read, with the Notes
/// Milk and Stamps: the store and their ids.
fn store() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([6; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    let stamps = Notes::new(&store).capture("Stamps", None, None)?.id;
    Ok((Arc::new(store), milk, stamps))
}

/// The main window over `store` on All Notes, where a window can be
/// kept on top when `pin_works`.
fn window(store: &Arc<Store>, pin_works: bool) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_pin_works(pin_works);
    app.select(Selection::All);
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

/// Lets egui's time for a double-click run out.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// The card whose text starts with `text`.
fn card<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> Node<'a> {
    match harness
        .query_all_by(move |n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with(text))
        })
        .next()
    {
        Some(node) => node,
        None => panic!("no card {text}"),
    }
}

/// Right-clicks the card of `text` and picks `entry` in its menu.
fn right_pick(harness: &mut Harness<'_, App>, text: &str, entry: &str) {
    pause(harness);
    card(harness, text).click_secondary();
    harness.run();
    harness
        .query_all_by_label(entry)
        .next_back()
        .unwrap_or_else(|| panic!("no {entry:?} in the card's menu"))
        .click_accesskit();
    harness.run_steps(8);
}

fn text_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(id)?.ok_or("the Note")?.text)
}

#[test]
fn the_cards_menu_and_the_dialogs_bar_open_a_note_in_one_window_of_its_own() -> TestResult {
    let (store, milk, stamps) = store()?;
    let mut harness = window(&store, true);
    right_pick(&mut harness, "Milk", OPEN);
    assert_eq!(harness.state().note_windows(), [milk.as_str()]);
    // Its window shows the Note to edit, Keep on top above it, off.
    assert_eq!(harness.query_all_by_label("Note text").count(), 1);
    let pin = harness.get_by_role_and_label(Role::CheckBox, KEEP_ON_TOP);
    assert!(!pin.accesskit_node().is_disabled());
    assert_eq!(
        harness
            .state()
            .note_window(&milk)
            .and_then(App::kept_on_top),
        Some(false)
    );
    // Again: its window comes forward, no second opens.
    right_pick(&mut harness, "Milk", OPEN);
    assert_eq!(harness.state().note_windows(), [milk.as_str()]);
    // From the Note dialog's bar, the dialog closing for it.
    harness.state_mut().select_note(&stamps);
    harness.run();
    assert!(harness.state().note_open());
    harness
        .get_by_role_and_label(Role::Button, OPEN)
        .click_accesskit();
    harness.run_steps(8);
    assert!(!harness.state().note_open());
    assert_eq!(
        harness.state().note_windows(),
        [milk.as_str(), stamps.as_str()]
    );
    // A Note with a window of its own opens there, not in the dialog.
    harness.state_mut().select_note(&milk);
    harness.run();
    assert!(!harness.state().note_open());
    assert_eq!(harness.state().note_windows().len(), 2);
    Ok(())
}

#[test]
fn what_is_typed_in_a_note_window_is_kept_as_it_closes_and_its_size_too() -> TestResult {
    let (store, milk, _) = store()?;
    let mut harness = window(&store, true);
    harness.state_mut().open_in_window(&milk);
    harness.run();
    harness.get_by_label("Note text").click();
    harness.run();
    harness.key_press(Key::End);
    harness.get_by(|n| n.is_focused()).type_text(" and bread");
    harness.run();
    harness.state_mut().close_note_window(&milk);
    harness.run();
    assert!(harness.state().note_windows().is_empty());
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    // The main window's list shows it at once.
    card(&harness, "Milk and bread");
    // The size it closed at is the next one's.
    assert!(
        store
            .local_setting(window_sizes::NOTE_WINDOW_SIZE)?
            .is_some()
    );
    Ok(())
}

#[test]
fn the_note_windows_close_with_the_main_window_keeping_what_was_typed() -> TestResult {
    let (store, milk, stamps) = store()?;
    let mut harness = window(&store, true);
    harness.state_mut().open_in_window(&milk);
    harness.run();
    harness.get_by_label("Note text").click();
    harness.run();
    harness.key_press(Key::End);
    harness.get_by(|n| n.is_focused()).type_text(" and bread");
    harness.run();
    harness.state_mut().open_in_window(&stamps);
    harness.run();
    assert_eq!(harness.state().note_windows().len(), 2);
    // As the main window closes or hides into the tray.
    harness.state_mut().flush();
    harness.run();
    assert!(harness.state().note_windows().is_empty());
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    Ok(())
}

#[test]
fn a_note_going_to_the_trash_closes_its_window_and_ctrl_z_brings_it_back() -> TestResult {
    let (store, milk, stamps) = store()?;
    let mut harness = window(&store, true);
    // Elsewhere, as another Device's sync brings it: the main window
    // reads the log again, and so does the Note window.
    harness.state_mut().open_in_window(&milk);
    harness.run();
    Routines::new(&store).trash_note(&milk)?;
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(harness.state().note_windows().is_empty());
    // From its own window: the one Undo slot is the main window's too.
    harness.state_mut().open_in_window(&stamps);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Move to trash")
        .click_accesskit();
    harness.run_steps(8);
    assert!(
        Notes::new(&store)
            .by_id(&stamps)?
            .ok_or("Stamps")?
            .is_trashed()
    );
    assert!(harness.state().note_windows().is_empty());
    pause(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(
        !Notes::new(&store)
            .by_id(&stamps)?
            .ok_or("Stamps")?
            .is_trashed()
    );
    Ok(())
}

#[test]
fn keep_on_top_puts_the_window_over_the_others_until_unticked() -> TestResult {
    let (store, milk, _) = store()?;
    let mut harness = window(&store, true);
    harness.state_mut().open_in_window(&milk);
    harness.run();
    let level = |harness: &Harness<'_, App>| {
        harness
            .state()
            .note_window_viewport(&milk)
            .and_then(|v| v.window_level)
    };
    assert_eq!(level(&harness), Some(WindowLevel::Normal));
    harness
        .get_by_role_and_label(Role::CheckBox, KEEP_ON_TOP)
        .click_accesskit();
    harness.run();
    assert_eq!(
        harness
            .state()
            .note_window(&milk)
            .and_then(App::kept_on_top),
        Some(true)
    );
    assert_eq!(level(&harness), Some(WindowLevel::AlwaysOnTop));
    harness
        .get_by_role_and_label(Role::CheckBox, KEEP_ON_TOP)
        .click_accesskit();
    harness.run();
    assert_eq!(level(&harness), Some(WindowLevel::Normal));
    Ok(())
}

#[test]
fn on_wayland_keep_on_top_is_greyed_with_the_reason() -> TestResult {
    let (store, milk, _) = store()?;
    let mut harness = window(&store, false);
    harness.state_mut().open_in_window(&milk);
    harness.run();
    let pin = harness.get_by_role_and_label(Role::CheckBox, KEEP_ON_TOP);
    assert!(pin.accesskit_node().is_disabled());
    assert!(
        harness
            .query_by_label_contains("Wayland lets no app keep its window on top")
            .is_some()
    );
    harness
        .get_by_role_and_label(Role::CheckBox, KEEP_ON_TOP)
        .click_accesskit();
    harness.run();
    assert_eq!(
        harness
            .state()
            .note_window(&milk)
            .and_then(App::kept_on_top),
        Some(false)
    );
    assert_eq!(
        harness
            .state()
            .note_window_viewport(&milk)
            .and_then(|v| v.window_level),
        Some(WindowLevel::Normal)
    );
    Ok(())
}
