//! A Note shows the Notes that link to it (obf-1035), driven headless by
//! egui_kittest: "Linked from" at the end of its detailed card, of the
//! Note dialog and of its own window's reading view, one row per Note by
//! its first line, a click opening that Note; nothing on a Note none
//! links to, nor for a link from the trash.

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
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::app_link::AppLink;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LINKED_FROM: &str = "Linked from";

/// A store on a clock that moves a second per read, with its own
/// Profile.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-11-05T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([5; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

/// Ann's Note and "Call Ann" linking to it: their ids.
fn linked(store: &Store) -> Result<(String, String), Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let ann = notes.capture("Ann's number", None, None)?;
    let text = format!(
        "Call Ann\nabout [her number]({})",
        AppLink::Note(ann.id.clone()).url()
    );
    let call = notes.capture_as(&text, MARKDOWN, None, None)?;
    Ok((ann.id, call.id))
}

/// The window on All Notes, the Note `note` open where there is one.
fn window(store: &Arc<Store>, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    if let Some(note) = note {
        app.select_note(note);
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
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

/// How often "Linked from" shows: a label's text is its value.
fn headings(harness: &Harness<'_, App>) -> usize {
    harness.query_all_by_value(LINKED_FROM).count()
}

fn click_row(harness: &mut Harness<'_, App>, name: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Link, name)
        .last()
        .ok_or(format!("the row {name:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

#[test]
fn the_note_dialog_lists_the_notes_linking_to_it_and_a_click_opens_one() -> TestResult {
    let store = store()?;
    let (ann, call) = linked(&store)?;
    let mut harness = window(&store, Some(&ann));
    assert!(headings(&harness) >= 1);
    click_row(&mut harness, "Call Ann")?;
    assert_eq!(harness.state().selected_note(), Some(call.as_str()));
    Ok(())
}

#[test]
fn a_detailed_card_lists_them_and_a_note_none_links_to_shows_none() -> TestResult {
    let store = store()?;
    let (_, call) = linked(&store)?;
    let mut harness = window(&store, None);
    // Only Ann's card: nothing links to "Call Ann".
    assert_eq!(headings(&harness), 1);
    click_row(&mut harness, "Call Ann")?;
    assert_eq!(harness.state().selected_note(), Some(call.as_str()));
    Ok(())
}

#[test]
fn a_link_from_the_trash_shows_none() -> TestResult {
    let store = store()?;
    let (ann, call) = linked(&store)?;
    Notes::new(&store).trash(&call)?;
    let harness = window(&store, Some(&ann));
    assert_eq!(headings(&harness), 0);
    Ok(())
}

#[test]
fn a_notes_own_window_lists_them_in_its_reading_view() -> TestResult {
    let store = store()?;
    let (ann, _) = linked(&store)?;
    let app = App::for_note_window(store.clone(), "en", &ann, true);
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
    harness.key_press(Key::F9);
    harness.run();
    assert_eq!(harness.state().reading(), Some(true));
    assert_eq!(headings(&harness), 1);
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Link, "Call Ann")
            .count(),
        1
    );
    Ok(())
}
