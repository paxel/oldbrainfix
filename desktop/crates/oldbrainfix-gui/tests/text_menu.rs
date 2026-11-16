//! The right-click menu of the desktop's text fields (#253): Cut, Copy,
//! Paste and Select all, greyed where they have nothing to work on, and
//! with a selection in a formatted text the format bar's Bold, Italic,
//! Code, Link and Code block. A right-click on the selection keeps it,
//! the field keeps the keyboard through the menu, and Cut, Copy and
//! Paste come to it in the next frame as their keys bring them (#319).

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
    Event, Id, Modifiers, OutputCommand, PointerButton, RawInput, ViewportCommand, ViewportId,
};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::text_menu::{self, Clip};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CLIPBOARD: [&str; 4] = ["Cut", "Copy", "Paste", "Select all"];
const FORMATS: [&str; 5] = ["Bold", "Italic", "Code", "Link", "Code block"];

/// A store with the Note `text` in `format` filed in "Work", and its id.
fn with_note(text: &str, format: &str) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([5; 32]))
    })?;
    let work = Places::new(&store).add_area("Work")?;
    let note = Notes::new(&store).capture_as(text, format, Some(&work.id), None)?;
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English on `selection`, `note` open.
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

/// The text field holding `text`, one or more lines.
fn field<'a>(harness: &'a Harness<'_, App>, text: &str) -> Option<egui_kittest::Node<'a>> {
    let text = text.to_string();
    harness
        .query_all_by(move |n| {
            matches!(n.role(), Role::MultilineTextInput | Role::TextInput)
                && n.value().as_deref() == Some(text.as_str())
        })
        .next()
}

/// Right-clicks the field holding `text`.
fn right_click(harness: &mut Harness<'_, App>, text: &str) {
    field(harness, text)
        .unwrap_or_else(|| panic!("no field {text:?}"))
        .click_secondary();
    harness.run();
}

/// The menu's entry `label`: the last so named, as the format bar's
/// buttons are drawn under the menu.
fn entry<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Option<egui_kittest::Node<'a>> {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
}

/// How many buttons are labelled `label`.
fn count(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    entry(harness, label).is_some_and(|n| !n.accesskit_node().is_disabled())
}

/// Picks the menu's entry `label`; the frame of the click is the last.
fn pick(harness: &mut Harness<'_, App>, label: &str) {
    entry(harness, label)
        .unwrap_or_else(|| panic!("no entry {label:?}"))
        .click();
    harness.step();
}

/// Whether the frame just drawn asked the window's clipboard `asked`.
fn asked(harness: &Harness<'_, App>, asked: &ViewportCommand) -> bool {
    harness.output().viewport_output[&ViewportId::ROOT]
        .commands
        .contains(asked)
}

/// The system clipboard's text, for Paste.
fn clipboard_milk() -> Option<String> {
    Some(" milk".into())
}

fn copied(harness: &Harness<'_, App>) -> Vec<String> {
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|c| match c {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn focused_value(harness: &Harness<'_, App>) -> Option<String> {
    harness.query_by(|n| n.is_focused())?.value()
}

#[test]
fn a_right_click_offers_the_clipboard_greyed_without_a_selection() -> TestResult {
    let (store, note) = with_note("buy milk now", "plain")?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    right_click(&mut harness, "buy milk now");
    for label in CLIPBOARD {
        assert!(entry(&harness, label).is_some(), "{label}");
    }
    // The cursor is only placed: nothing to cut or copy.
    assert!(!enabled(&harness, "Cut"));
    assert!(!enabled(&harness, "Copy"));
    assert!(enabled(&harness, "Paste"));
    assert!(enabled(&harness, "Select all"));
    // A plain text has no format entries.
    let bars = count(&harness, "Bold");
    pick(&mut harness, "Select all");
    harness.run();
    right_click(&mut harness, "buy milk now");
    assert!(enabled(&harness, "Cut"));
    assert!(enabled(&harness, "Copy"));
    assert_eq!(count(&harness, "Bold"), bars, "the format bars' Bold only");
    Ok(())
}

#[test]
fn copy_from_the_menu_puts_the_selection_on_the_clipboard() -> TestResult {
    let (store, note) = with_note("buy milk now", "plain")?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    right_click(&mut harness, "buy milk now");
    pick(&mut harness, "Select all");
    harness.run();
    // The right-click on the selection keeps it.
    right_click(&mut harness, "buy milk now");
    pick(&mut harness, "Copy");
    // The window is not asked: the field copies in the next frame, as
    // Ctrl+C would have it (#319).
    assert!(!asked(&harness, &ViewportCommand::RequestCopy));
    harness.step();
    assert_eq!(copied(&harness), ["buy milk now"]);
    harness.run();
    // The field kept the keyboard, its selection with it.
    assert_eq!(focused_value(&harness).as_deref(), Some("buy milk now"));
    Ok(())
}

#[test]
fn cut_and_paste_from_the_menu_go_into_the_field() -> TestResult {
    let (store, _) = with_note("buy milk", "plain")?;
    let mut harness = window(&store, Selection::All, None);
    text_menu::read_clipboard_by(&harness.ctx, clipboard_milk);
    let search = harness.get_by(|n| n.placeholder() == Some("Search all Notes"));
    search.click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("oat milk");
    harness.run();
    right_click(&mut harness, "oat milk");
    pick(&mut harness, "Select all");
    harness.run();
    right_click(&mut harness, "oat milk");
    pick(&mut harness, "Cut");
    assert!(!asked(&harness, &ViewportCommand::RequestCut));
    harness.step();
    assert_eq!(copied(&harness), ["oat milk"]);
    harness.run();
    assert_eq!(focused_value(&harness).as_deref(), Some(""));

    harness.get_by(|n| n.is_focused()).type_text("rice");
    harness.run();
    right_click(&mut harness, "rice");
    pick(&mut harness, "Paste");
    // The system clipboard's text, read at once.
    assert!(!asked(&harness, &ViewportCommand::RequestPaste));
    harness.run();
    assert_eq!(focused_value(&harness).as_deref(), Some("rice milk"));
    Ok(())
}

#[test]
fn paste_asks_the_window_where_the_clipboard_cannot_be_read() -> TestResult {
    let (store, _) = with_note("buy milk", "plain")?;
    let mut harness = window(&store, Selection::All, None);
    let search = harness.get_by(|n| n.placeholder() == Some("Search all Notes"));
    search.click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("rice");
    harness.run();
    right_click(&mut harness, "rice");
    pick(&mut harness, "Paste");
    assert!(asked(&harness, &ViewportCommand::RequestPaste));
    Ok(())
}

const NOTE_WINDOW: &str = "note window";

/// A frame of the window `viewport`, beside the main window and a Note
/// window.
fn frame_of(viewport: ViewportId) -> RawInput {
    let mut input = RawInput {
        viewport_id: viewport,
        ..RawInput::default()
    };
    for id in [ViewportId::ROOT, ViewportId::from_hash_of(NOTE_WINDOW)] {
        input.viewports.insert(id, egui::ViewportInfo::default());
    }
    input
}

/// The events the frame of the window `viewport` brings.
fn events_in(ctx: &egui::Context, viewport: ViewportId) -> Vec<Event> {
    let mut events = Vec::new();
    let _ = ctx.run_ui(frame_of(viewport), |ui| {
        events = ui.input(|i| i.events.clone());
    });
    events
}

#[test]
fn a_clip_comes_to_the_window_it_was_asked_in_once() {
    let ctx = egui::Context::default();
    let note_window = ViewportId::from_hash_of(NOTE_WINDOW);
    let _ = ctx.run_ui(frame_of(note_window), |ui| {
        text_menu::clip_into(ui.ctx(), Id::new("field"), Clip::Copy);
        text_menu::clip_into(ui.ctx(), Id::new("field"), Clip::Cut);
    });
    // Not in the main window's frame; in the Note window's next, once.
    assert!(!events_in(&ctx, ViewportId::ROOT).contains(&Event::Copy));
    assert_eq!(events_in(&ctx, note_window), [Event::Copy, Event::Cut]);
    assert!(events_in(&ctx, note_window).is_empty());
}

#[test]
fn a_selection_in_a_formatted_text_gets_the_format_entries() -> TestResult {
    let (store, note) = with_note("buy *milk* now", MARKDOWN)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    harness
        .query_all_by_role_and_label(Role::Button, "Edit")
        .last()
        .ok_or("the text")?
        .click_accesskit();
    harness.run();
    let bars = FORMATS.map(|label| count(&harness, label));
    right_click(&mut harness, "buy *milk* now");
    // Nothing selected: no format entries.
    assert!(entry(&harness, "Select all").is_some());
    assert_eq!(FORMATS.map(|label| count(&harness, label)), bars);
    // A click on a greyed entry leaves the text open for editing.
    pick(&mut harness, "Copy");
    harness.run();
    assert_eq!(focused_value(&harness).as_deref(), Some("buy *milk* now"));
    right_click(&mut harness, "buy *milk* now");
    pick(&mut harness, "Select all");
    harness.run();
    right_click(&mut harness, "buy *milk* now");
    let offered = FORMATS.map(|label| count(&harness, label));
    assert_eq!(offered, bars.map(|n| n + 1), "the bars' and the menu's");
    // Copy keeps the formatted text open for editing.
    pick(&mut harness, "Copy");
    harness.run();
    assert_eq!(focused_value(&harness).as_deref(), Some("buy *milk* now"));
    right_click(&mut harness, "buy *milk* now");
    pick(&mut harness, "Bold");
    harness.run();
    assert!(field(&harness, "**buy *milk* now**").is_some());
    assert_eq!(
        focused_value(&harness).as_deref(),
        Some("**buy *milk* now**")
    );
    Ok(())
}

#[test]
fn a_right_click_pressed_and_released_in_two_frames_keeps_the_selection() -> TestResult {
    let (store, note) = with_note("buy milk now", "plain")?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    right_click(&mut harness, "buy milk now");
    pick(&mut harness, "Select all");
    harness.run();
    let pos = field(&harness, "buy milk now")
        .ok_or("the field")?
        .rect()
        .center();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    assert!(enabled(&harness, "Copy"));
    assert!(enabled(&harness, "Cut"));
    assert!(field(&harness, "buy milk now").is_some());
    Ok(())
}
