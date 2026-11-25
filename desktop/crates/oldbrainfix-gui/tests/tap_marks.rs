//! The green "!" of a button clicked (obf-1143), driven headless by
//! egui_kittest: a pointer click on a button of any kind shows it above
//! the pointer, over the Note dialog too, growing and fading out over
//! 0.9 s; a card, a menu's entry and a screen reader's click show none;
//! "Show button taps" in Settings ▸ This computer switches it off, kept
//! as the phone keeps it.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use egui::accesskit::Role;
use egui::epaint::{ClippedShape, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store, parse_time};
use oldbrainfix_gui::design::{self, TAP_MARK_GREEN, TAP_MARKS_OFF};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-25T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
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

/// Each "!" drawn this frame: where its middle is, how high it is and
/// its colour, and its place among all shapes drawn.
fn marks(harness: &Harness<'_, App>) -> Vec<(egui::Pos2, f32, egui::Color32, usize)> {
    let shapes: &[ClippedShape] = &harness.output().shapes;
    shapes
        .iter()
        .enumerate()
        .filter_map(|(at, clipped)| match &clipped.shape {
            Shape::Text(text) if text.galley.text() == "\u{e4ec}" => {
                let colour = text.galley.job.sections.first()?.format.color;
                let rect = text.visual_bounding_rect();
                // Larger than any "!" in a text.
                (text.galley.size().y > 30.0).then_some((
                    rect.center(),
                    text.galley.size().y,
                    colour,
                    at,
                ))
            }
            _ => None,
        })
        .collect()
}

/// The button `label`, clicked with the pointer, and the frame after.
fn pointer_click(harness: &mut Harness<'_, App>, label: &str) -> egui::Pos2 {
    let button = harness.get_by_role_and_label(Role::Button, label);
    let at = button.rect().center();
    button.click();
    harness.step();
    at
}

#[test]
fn a_button_clicked_shows_a_green_mark_above_the_pointer_that_grows_and_goes() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::All);
    assert!(marks(&harness).is_empty());
    let at = pointer_click(&mut harness, "Show Notes by…");
    let first = marks(&harness);
    assert_eq!(first.len(), 1, "{first:?}");
    let (centre, high, colour, _) = first[0];
    assert!(centre.y < at.y - 30.0, "{centre:?} above {at:?}");
    assert!((centre.x - at.x).abs() < 1.0, "{centre:?} over {at:?}");
    assert_eq!(colour, TAP_MARK_GREEN);
    // Growing and fading as it goes.
    harness.step();
    harness.step();
    let later = marks(&harness);
    assert_eq!(later.len(), 1);
    assert!(later[0].1 > high, "{} grows past {high}", later[0].1);
    assert!(later[0].2.a() < colour.a());
    // Gone once 0.9 s passed, and nothing asks to draw again for it.
    harness.step();
    harness.step();
    assert!(marks(&harness).is_empty());
    harness.run();
    assert!(marks(&harness).is_empty());
    Ok(())
}

#[test]
fn the_mark_shows_over_the_note_dialog() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store).capture("Fix the tap", None, None)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&note.id);
    harness.run();
    assert!(harness.state().note_open());
    pointer_click(&mut harness, "Add a part");
    let shown = marks(&harness);
    assert_eq!(shown.len(), 1, "{shown:?}");
    // Drawn after everything else, the dialog included.
    let last = harness.output().shapes.len();
    let dialog_text = harness
        .output()
        .shapes
        .iter()
        .rposition(
            |c| matches!(&c.shape, Shape::Text(t) if t.galley.text().contains("Fix the tap")),
        )
        .ok_or("the dialog's text")?;
    assert!(shown[0].3 > dialog_text && shown[0].3 < last);
    Ok(())
}

#[test]
fn a_screen_readers_click_shows_no_mark() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::All);
    harness
        .get_by_role_and_label(Role::Button, "Show Notes by…")
        .click_accesskit();
    harness.step();
    assert!(marks(&harness).is_empty());
    Ok(())
}

#[test]
fn show_button_clicks_switches_the_mark_off_and_on_again() -> TestResult {
    let store = store()?;
    let mut app = App::new(store.clone(), "en");
    app.open(Page::Settings);
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
    let switch = harness.get_by_role_and_label(Role::CheckBox, "Show button taps");
    assert_eq!(
        switch.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert!(
        harness
            .query_by_label_contains("when a button takes a tap")
            .is_some()
    );
    switch.click_accesskit();
    harness.run();
    assert_eq!(store.local_setting(TAP_MARKS_OFF)?.as_deref(), Some("true"));
    assert!(!design::tap_marks_on(&store));
    // Off, a click shows nothing, and a new window starts off.
    harness.state_mut().close_page();
    harness.run();
    let mut harness = window(&store, Selection::All);
    pointer_click(&mut harness, "Show Notes by…");
    assert!(marks(&harness).is_empty());
    // On again, nothing is kept.
    design::set_tap_marks_setting(&store, true)?;
    assert_eq!(store.local_setting(TAP_MARKS_OFF)?, None);
    let mut harness = window(&store, Selection::All);
    pointer_click(&mut harness, "Show Notes by…");
    assert_eq!(marks(&harness).len(), 1);
    Ok(())
}

#[test]
fn a_cards_click_shows_no_mark() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture("Fix the tap", None, None)?;
    let mut harness = window(&store, Selection::All);
    let card = harness
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("Fix the tap"))
        })
        .next()
        .ok_or("the card")?;
    card.click();
    harness.step();
    assert!(marks(&harness).is_empty());
    Ok(())
}

#[test]
fn a_menus_entry_shows_no_mark() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda);
    pointer_click(&mut harness, "New Note");
    assert_eq!(marks(&harness).len(), 1);
    for _ in 0..4 {
        harness.step();
    }
    assert!(marks(&harness).is_empty());
    // "Quick note" in the menu "+" opened.
    pointer_click(&mut harness, "Quick note");
    assert!(harness.state().quick_note_open());
    assert!(marks(&harness).is_empty());
    Ok(())
}
