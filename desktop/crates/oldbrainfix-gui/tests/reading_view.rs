//! The reading view of a Note's own window on the desktop (#322,
//! obf-1015), driven headless by egui_kittest: Reading view at the end of
//! the window's bar and F9 switch it both ways, Esc, Alt+Left and the
//! mouse's back button leave it after what lies over it; it shows only
//! the Note's content (its text, Lines, Sections and photos), boxes and
//! Lines ticking, in a column of at most [`note_window::measure`] in the
//! middle; what was typed is kept as it opens; the keys of a page scroll
//! it; the window is maximized as the system's are.

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
use egui::{Key, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Checklists, Notes, OwnProfile, SigningKey, Store, text_format};
use oldbrainfix_gui::note_window::{self, Scrolled};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const READING: &str = "Reading view";
const KEEP_ON_TOP: &str = "Keep on top";

/// A long paragraph, many times the column's measure.
const LONG: &str = "Mix the flour with the milk and the eggs until no lump is left, \
    then let it rest for half an hour while the pan heats up slowly.";

/// A store on a clock that moves a microsecond per read.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([6; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

/// A formatted Note `text` in `store`, with a formatted Section `section`
/// where there is one: its id.
fn recipe(
    store: &Store,
    text: &str,
    section: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let id = notes.capture("draft", None, None)?.id;
    notes.edit_as(&id, text, text_format::MARKDOWN)?;
    if let Some(section) = section {
        notes.amend_as(&id, section, text_format::MARKDOWN)?;
    }
    Ok(id)
}

/// A picture of `width` × `height` pixels, as a PNG.
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([20, 120, 200, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    let written =
        image::DynamicImage::ImageRgba8(image).write_to(&mut out, image::ImageFormat::Png);
    assert!(written.is_ok());
    out.into_inner()
}

/// The Note `note` of `store` in a window of its own, `size` large, at
/// the window's root, as the guide's frames draw it.
fn alone(store: &Arc<Store>, note: &str, size: egui::Vec2) -> Harness<'static, App> {
    let app = App::for_note_window(store.clone(), "en", note, true);
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        |ui, app: &mut App| {
            App::install(ui.ctx());
            app.show_note_window(ui);
        },
        app,
    );
    harness.run();
    harness
}

fn normal(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    alone(store, note, egui::vec2(800.0, 900.0))
}

fn reading(harness: &Harness<'_, App>) -> bool {
    harness.state().reading() == Some(true)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn shows_part(harness: &Harness<'_, App>, part: &str) -> bool {
    harness
        .query_all_by(|n| {
            n.label().is_some_and(|l| l.contains(part))
                || n.value().is_some_and(|v| v.contains(part))
        })
        .next()
        .is_some()
}

#[test]
fn the_button_and_f9_switch_the_reading_view_both_ways() -> TestResult {
    let store = store()?;
    let note = recipe(&store, "# Pancakes\n\n200 g **flour**", None)?;
    let mut harness = normal(&store, &note);
    // It opens to edit: a formatted text with Edit beside it.
    assert!(!reading(&harness));
    assert!(shows(&harness, "Edit"));
    assert!(shows(&harness, KEEP_ON_TOP));
    harness.get_by_role_and_label(Role::Button, READING).click();
    harness.run();
    assert!(reading(&harness));
    assert!(!shows(&harness, "Edit"));
    assert!(shows(&harness, "Pancakes"));
    // The same button leads back.
    harness.get_by_role_and_label(Role::Button, READING).click();
    harness.run();
    assert!(!reading(&harness));
    assert!(shows(&harness, "Edit"));
    // And F9, both ways.
    harness.key_press(Key::F9);
    harness.run();
    assert!(reading(&harness));
    harness.key_press(Key::F9);
    harness.run();
    assert!(!reading(&harness));
    Ok(())
}

#[test]
fn the_reading_view_shows_only_the_notes_content() -> TestResult {
    let store = store()?;
    let note = recipe(
        &store,
        "# Pancakes\n\n- 200 g **flour**",
        Some("Rest for *half* an hour."),
    )?;
    Notes::new(&store).add_photo(&note, &png(40, 30))?;
    let mut harness = normal(&store, &note);
    harness.key_press(Key::F9);
    harness.run();
    assert!(reading(&harness));
    // Its text and its Section's, its photo.
    assert!(shows(&harness, "Pancakes"));
    assert!(shows_part(&harness, "flour"));
    assert!(shows_part(&harness, "half"));
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Image, "Photo")
            .count(),
        1
    );
    // No field, no box, no badge and no other button: only the way back.
    assert_eq!(harness.query_all_by_role(Role::TextInput).count(), 0);
    assert_eq!(
        harness.query_all_by_role(Role::MultilineTextInput).count(),
        0
    );
    assert_eq!(harness.query_all_by_role(Role::CheckBox).count(), 0);
    let buttons: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert_eq!(buttons, [READING]);
    assert!(!shows(&harness, KEEP_ON_TOP));
    // A click on the photo shows it full screen, Esc closing that first.
    harness.get_by_role_and_label(Role::Image, "Photo").click();
    harness.run();
    assert!(shows(&harness, "Close"));
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shows(&harness, "Close"));
    assert!(reading(&harness));
    Ok(())
}

#[test]
fn a_checklists_lines_show_and_tick_in_the_reading_view() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store)
        .capture("Packing\nPassport\nTicket", None, None)?
        .id;
    let lists = Checklists::new(&store);
    lists.make(&note)?;
    let mut harness = normal(&store, &note);
    harness.key_press(Key::F9);
    harness.run();
    assert!(reading(&harness));
    assert!(shows_part(&harness, "Packing"));
    let line = harness.get_by_role_and_label(Role::CheckBox, "Passport");
    line.click();
    harness.run_steps(4);
    let checked = lists
        .by_note(&note)?
        .ok_or("a Checklist")?
        .lines
        .iter()
        .any(|l| l.text == "Passport" && l.checked);
    assert!(checked);
    // Its Lines are no fields here.
    assert_eq!(harness.query_all_by_role(Role::TextInput).count(), 0);
    Ok(())
}

#[test]
fn what_was_typed_shows_and_is_kept_as_the_reading_view_opens() -> TestResult {
    let store = store()?;
    // A plain one, its text a field at once.
    let note = Notes::new(&store).capture("Pancakes", None, None)?.id;
    let mut harness = normal(&store, &note);
    harness.get_by_label("Note text").click();
    harness.run();
    harness.key_press(Key::End);
    harness.get_by(|n| n.is_focused()).type_text(" with syrup");
    harness.run();
    // F9 works while typing.
    harness.key_press(Key::F9);
    harness.run();
    assert!(reading(&harness));
    assert_eq!(
        Notes::new(&store).by_id(&note)?.ok_or("the Note")?.text,
        "Pancakes with syrup"
    );
    assert!(shows(&harness, "Pancakes with syrup"));
    Ok(())
}

#[test]
fn a_wide_window_keeps_the_column_at_its_measure_in_the_middle() -> TestResult {
    let store = store()?;
    let note = recipe(&store, LONG, None)?;
    let width = 2400.0;
    let mut harness = alone(&store, &note, egui::vec2(width, 900.0));
    harness.key_press(Key::F9);
    harness.run();
    let text = harness.get_by_label(LONG).rect();
    // At Normal, the text's 18 points.
    let measure = note_window::measure(18.0);
    assert!(text.width() <= measure + 1.0, "{} wide", text.width());
    // Wrapped, it fills the column, which lies in the middle.
    assert!(text.width() > measure * 0.8, "{} wide", text.width());
    let middle = (text.left() + measure / 2.0 - width / 2.0).abs();
    assert!(middle < 24.0, "{middle} off the middle");
    // A narrow window reads it as wide as it is.
    let mut narrow = alone(&store, &note, egui::vec2(500.0, 900.0));
    narrow.key_press(Key::F9);
    narrow.run();
    assert!(narrow.get_by_label(LONG).rect().width() < 500.0);
    Ok(())
}

#[test]
fn the_keys_of_a_page_scroll_the_reading_view() -> TestResult {
    let store = store()?;
    let text = vec![LONG; 40].join("\n\n");
    let note = recipe(&store, &text, None)?;
    let mut harness = alone(&store, &note, egui::vec2(800.0, 600.0));
    harness.key_press(Key::F9);
    harness.run();
    let at = |h: &Harness<'_, App>| h.state().reading_scrolled().unwrap_or_default();
    let start = at(&harness);
    assert_eq!(start.offset, 0.0);
    assert!(start.height > 2.0 * start.page);
    let mut press = |key: Key, modifiers: Modifiers| -> Scrolled {
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        at(&harness)
    };
    let down = press(Key::PageDown, Modifiers::NONE);
    assert!(down.offset > start.page / 2.0, "{down:?}");
    let space = press(Key::Space, Modifiers::NONE);
    assert!(space.offset > down.offset, "{space:?}");
    let back = press(Key::Space, Modifiers::SHIFT);
    assert!((back.offset - down.offset).abs() < 1.0, "{back:?}");
    let end = press(Key::End, Modifiers::NONE);
    assert!(
        (end.offset - (end.height - end.page)).abs() < 1.0,
        "{end:?}"
    );
    let up = press(Key::PageUp, Modifiers::NONE);
    assert!(up.offset < end.offset, "{up:?}");
    let line = press(Key::ArrowDown, Modifiers::NONE);
    assert!(line.offset > up.offset, "{line:?}");
    let top = press(Key::Home, Modifiers::NONE);
    assert_eq!(top.offset, 0.0);
    Ok(())
}

#[test]
fn esc_alt_left_and_the_back_button_leave_the_reading_view() -> TestResult {
    let store = store()?;
    let note = recipe(&store, "Pancakes", None)?;
    let mut harness = normal(&store, &note);
    let open = |h: &mut Harness<'_, App>| {
        h.key_press(Key::F9);
        h.run();
        assert!(reading(h));
    };
    open(&mut harness);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!reading(&harness));
    open(&mut harness);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert!(!reading(&harness));
    open(&mut harness);
    let at = harness.get_by_label("Pancakes").rect().center();
    harness.event(egui::Event::PointerButton {
        pos: at,
        button: PointerButton::Extra1,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(egui::Event::PointerButton {
        pos: at,
        button: PointerButton::Extra1,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(!reading(&harness));
    // Esc over the Note to edit closes nothing.
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().reading(), Some(false));
    Ok(())
}

#[test]
fn a_note_window_opened_from_the_main_window_reads_and_can_be_maximized() -> TestResult {
    let store = store()?;
    let note = recipe(&store, "# Pancakes", None)?;
    let mut app = App::new(store.clone(), "en");
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
    harness.state_mut().open_in_window(&note);
    harness.run_steps(4);
    let builder = harness
        .state()
        .note_window_viewport(&note)
        .ok_or("its window")?;
    assert_eq!(builder.maximize_button, Some(true));
    assert_eq!(builder.resizable, Some(true));
    harness.get_by_role_and_label(Role::Button, READING).click();
    harness.run_steps(4);
    let window = harness.state().note_window(&note).ok_or("its App")?;
    assert_eq!(window.reading(), Some(true));
    Ok(())
}

#[test]
fn a_maximized_window_keeps_no_size_for_the_next() -> TestResult {
    let store = store()?;
    let note = recipe(&store, "Pancakes", None)?;
    let mut harness = normal(&store, &note);
    let before = harness.state().note_window_size();
    assert!(before.is_some());
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .maximized = Some(true);
    harness.set_size(egui::vec2(1600.0, 1000.0));
    harness.run();
    assert_eq!(harness.state().note_window_size(), before);
    Ok(())
}

#[test]
fn the_page_keys_scroll_also_while_a_widget_has_the_keyboard() -> TestResult {
    let store = store()?;
    let lines: Vec<String> = (1..=60).map(|n| format!("Item {n}")).collect();
    let text = format!("Packing\n{}", lines.join("\n"));
    let note = Notes::new(&store).capture(&text, None, None)?.id;
    Checklists::new(&store).make(&note)?;
    let mut harness = alone(&store, &note, egui::vec2(800.0, 600.0));
    harness.key_press(Key::F9);
    harness.run();
    // A click ticks the Line; Tab gives a widget the keyboard.
    harness
        .get_by_role_and_label(Role::CheckBox, "Item 1")
        .click();
    harness.run_steps(4);
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.query_all_by(|n| n.is_focused()).next().is_some());
    let at = |h: &Harness<'_, App>| h.state().reading_scrolled().unwrap_or_default();
    assert_eq!(at(&harness).offset, 0.0);
    harness.key_press(Key::PageDown);
    harness.run();
    assert!(at(&harness).offset > 0.0, "{:?}", at(&harness));
    harness.key_press(Key::End);
    harness.run();
    let end = at(&harness);
    assert!(
        (end.offset - (end.height - end.page)).abs() < 1.0,
        "{end:?}"
    );
    Ok(())
}
