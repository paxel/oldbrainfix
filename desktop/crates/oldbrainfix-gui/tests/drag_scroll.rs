//! Scrolling while a selection is dragged (#256): with the button held
//! the wheel scrolls the Note dialog and the selection follows, and a
//! drag past the dialog's bottom edge scrolls it by itself, faster the
//! further out the pointer is; in the Note's text field and in a text
//! only read alike.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use egui::accesskit::Role;
use egui::{
    Event, Modifiers, MouseWheelUnit, OutputCommand, PointerButton, Pos2, Rect, TouchPhase,
};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const HEIGHT: f32 = 700.0;

/// Sixty lines, "Line 1" to "Line 60".
fn long() -> String {
    (1..=60)
        .map(|n| format!("Line {n}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([8; 32]))
    })?;
    Ok(Arc::new(store))
}

fn opened(store: &Arc<Store>, place: Selection, note: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(place);
    app.select_note(note);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, HEIGHT))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    assert!(harness.state().note_open());
    harness
}

fn button(harness: &mut Harness<'_, App>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

/// Presses the button at `at` and drags a little down, to start a
/// selection there.
fn start(harness: &mut Harness<'_, App>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    button(harness, at, true);
    for dy in [6.0, 12.0] {
        harness.event(Event::PointerMoved(at + egui::vec2(0.0, dy)));
        harness.step();
    }
}

/// Releases the button and copies what is selected.
fn release_and_copy(harness: &mut Harness<'_, App>, at: Pos2) -> String {
    button(harness, at, false);
    harness.event(Event::Copy);
    let mut copied = String::new();
    for _ in 0..3 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let OutputCommand::CopyText(text) = command {
                copied.clone_from(text);
            }
        }
    }
    copied
}

fn field(harness: &Harness<'_, App>) -> Rect {
    harness
        .query_all_by(|n| {
            n.role() == Role::MultilineTextInput && n.label().as_deref() == Some("Note text")
        })
        .next()
        .expect("the Note's field")
        .rect()
}

fn label(harness: &Harness<'_, App>, text: &str) -> Rect {
    harness
        .query_all_by_label(text)
        .last()
        .unwrap_or_else(|| panic!("no {text:?}"))
        .rect()
}

/// The pointer `out` below the window, held there `frames` frames.
fn past_the_bottom(harness: &mut Harness<'_, App>, x: f32, out: f32, frames: usize) -> Pos2 {
    let at = egui::pos2(x, HEIGHT + out);
    harness.event(Event::PointerMoved(at));
    for _ in 0..frames {
        harness.step();
    }
    at
}

fn note(store: &Store, format: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .capture_as(&long(), format, None, None)?
        .id)
}

#[test]
fn a_drag_past_the_bottom_scrolls_the_field_faster_the_further_out() -> TestResult {
    let store = store()?;
    let id = note(&store, "plain")?;
    let mut harness = opened(&store, Selection::Unsorted, &id);
    let top = field(&harness).top();
    let first = label_free_start(&harness);
    start(&mut harness, first);
    past_the_bottom(&mut harness, first.x, 10.0, 10);
    let near = top - field(&harness).top();
    assert!(near > 0.0, "it scrolled: {near}");
    past_the_bottom(&mut harness, first.x, 120.0, 10);
    let far = top - field(&harness).top() - near;
    assert!(far > near * 3.0, "faster further out: {near} then {far}");
    let at = past_the_bottom(&mut harness, first.x, 120.0, 0);
    let copied = release_and_copy(&mut harness, at);
    assert!(copied.starts_with("Line 1"), "{copied:?}");
    assert!(
        copied.contains("Line 20"),
        "the selection followed: {copied:?}"
    );
    Ok(())
}

/// Where the Note's field starts: its first line.
fn label_free_start(harness: &Harness<'_, App>) -> Pos2 {
    field(harness).left_top() + egui::vec2(2.0, 4.0)
}

#[test]
fn the_wheel_scrolls_while_the_selection_is_dragged() -> TestResult {
    let store = store()?;
    let id = note(&store, "plain")?;
    let mut harness = opened(&store, Selection::Unsorted, &id);
    let top = field(&harness).top();
    let first = label_free_start(&harness);
    start(&mut harness, first);
    let held = first + egui::vec2(0.0, 12.0);
    for _ in 0..4 {
        harness.event(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -150.0),
            phase: TouchPhase::Move,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    for _ in 0..20 {
        harness.step();
    }
    let scrolled = top - field(&harness).top();
    assert!(scrolled > 300.0, "{scrolled}");
    let copied = release_and_copy(&mut harness, held);
    assert!(copied.starts_with("Line 1"), "{copied:?}");
    assert!(
        copied.contains("Line 6"),
        "the selection followed: {copied:?}"
    );
    Ok(())
}

#[test]
fn a_text_only_read_scrolls_while_its_selection_is_dragged() -> TestResult {
    let store = store()?;
    let id = note(&store, MARKDOWN)?;
    Notes::new(&store).trash(&id)?;
    let mut harness = opened(&store, Selection::Trash, &id);
    let first = label(&harness, "Line 1");
    let at = first.left_top() + egui::vec2(1.0, first.height() / 2.0);
    start(&mut harness, at);
    past_the_bottom(&mut harness, at.x, 60.0, 15);
    assert!(label(&harness, "Line 1").top() < first.top() - 50.0);
    let at = past_the_bottom(&mut harness, at.x, 60.0, 0);
    let copied = release_and_copy(&mut harness, at);
    assert!(copied.starts_with("Line 1"), "{copied:?}");
    assert!(
        copied.contains("Line 20"),
        "the selection followed: {copied:?}"
    );
    Ok(())
}
