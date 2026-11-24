//! The search fields that stand over what they list (obf-1088): the
//! Settings', File into…, Merge into…, Add to Note…, Link to… and the
//! main list's each sit on a band of their own fill with a line along its
//! bottom, and what scrolls under them is cut at that line, not at an edge
//! in the page's own colour.

// A test may panic, its helpers too (#229).
#![allow(clippy::panic)]

use std::sync::Arc;

use egui::accesskit::Role;
use egui::epaint::ClippedShape;
use egui::{Key, Modifiers, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::theme::{self, DARK, LIGHT};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SAUCE: &str = "Tomato sauce";
const PASTA: &str = "Pasta for Friday";
const MILK: &str = "Buy milk";

/// "Tomato sauce" and "Pasta for Friday" in Household, "Buy milk"
/// Unsorted; the Area's id.
fn store() -> Result<(Store, String), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([9; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let house = Places::new(&store).add_area("Household")?;
    let notes = Notes::new(&store);
    notes.capture(SAUCE, Some(&house.id), None)?;
    notes.capture(PASTA, Some(&house.id), None)?;
    notes.capture(MILK, None, None)?;
    Ok((store, house.id))
}

/// The window, 900 tall, over `app`.
fn window(app: App) -> Harness<'static, App> {
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

/// Every shape drawn, out of its lists, with the rect it is clipped to.
fn drawn(harness: &Harness<'_, App>) -> Vec<(Rect, Shape)> {
    fn flat(clip: Rect, shape: &Shape, into: &mut Vec<(Rect, Shape)>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flat(clip, s, into)),
            other => into.push((clip, other.clone())),
        }
    }
    let mut shapes = Vec::new();
    for ClippedShape { clip_rect, shape } in &harness.output().shapes {
        flat(*clip_rect, shape, &mut shapes);
    }
    shapes
}

/// Where the search field named or hinted `hint` lies.
fn field(harness: &Harness<'_, App>, hint: &str) -> Rect {
    let node = harness
        .query_all_by(|n| {
            n.role() == Role::TextInput
                && (n.label().as_deref() == Some(hint) || n.placeholder() == Some(hint))
        })
        .next()
        .unwrap_or_else(|| panic!("no field {hint:?}"));
    node.rect()
}

/// Asserts that the field hinted `hint` stands on the band, its line
/// along the band's bottom, and that `row`, listed under it, is cut at
/// that line.
fn on_the_band(harness: &Harness<'_, App>, hint: &str, row: &str) {
    let field = field(harness, hint);
    let shapes = drawn(harness);
    let fills = [theme::band_fill(LIGHT), theme::band_fill(DARK)];
    // The last drawn, over the window's behind a dialog.
    let band = shapes
        .iter()
        .rev()
        .find_map(|(_, shape)| match shape {
            Shape::Rect(r) if fills.contains(&r.fill) && r.rect.contains_rect(field) => {
                Some(r.rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no band under {hint:?}"));
    let lined = shapes.iter().any(|(_, shape)| match shape {
        Shape::LineSegment { points, stroke } => {
            stroke.width > 0.0
                && (points[0].y - band.bottom()).abs() <= 1.0
                && (points[1].y - band.bottom()).abs() <= 1.0
                && (points[0].x - band.left()).abs() <= 1.0
                && (points[1].x - band.right()).abs() <= 1.0
        }
        _ => false,
    });
    assert!(lined, "no line along the bottom of {hint:?}'s band");
    // Under the band and as wide, not the window's behind a dialog.
    let cut = shapes.iter().find_map(|(clip, shape)| match shape {
        Shape::Text(text)
            if text.galley.text() == row
                && text.pos.y >= band.bottom() - 0.5
                && band.expand(1.0).x_range().contains(text.pos.x) =>
        {
            Some(clip.top())
        }
        _ => None,
    });
    let Some(cut) = cut else {
        panic!("{row:?} is not listed under {hint:?}");
    };
    assert!(
        (cut - band.bottom()).abs() <= 1.0,
        "{row:?} is cut at {cut}, not at the band's line at {}",
        band.bottom()
    );
}

/// The last button named `label`, the one over the window's.
fn press(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .next_back()
        .ok_or_else(|| format!("no {label:?}"))?
        .click_accesskit();
    harness.run_steps(8);
    Ok(())
}

#[test]
fn the_settings_scroll_under_their_searchs_line() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Unsorted);
    app.open(Page::Settings);
    let harness = window(app);
    let t = *harness.state().t();
    on_the_band(&harness, t.settings_search_hint(), t.settings_computer());
    Ok(())
}

#[test]
fn the_cards_of_a_list_scroll_under_its_searchs_line() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Unsorted);
    let mut harness = window(app);
    harness.run_steps(50);
    let t = *harness.state().t();
    on_the_band(&harness, t.search_unsorted_hint(), MILK);
    Ok(())
}

#[test]
fn file_intos_places_scroll_under_its_filters_line() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Unsorted);
    let mut harness = window(app);
    harness.run_steps(50);
    harness.get_by_role_and_label(Role::Button, MILK).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
    let t = *harness.state().t();
    on_the_band(&harness, t.file_into_filter_hint(), "Household");
    Ok(())
}

#[test]
fn merge_intos_notes_scroll_under_its_searchs_line() -> TestResult {
    let (store, house) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(house));
    let mut harness = window(app);
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, SAUCE)
        .click_secondary();
    harness.run();
    press(&mut harness, "Merge into…")?;
    let t = *harness.state().t();
    on_the_band(&harness, t.search_all_hint(), PASTA);
    Ok(())
}

#[test]
fn add_to_notes_notes_scroll_under_its_searchs_line() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Unsorted);
    let mut harness = window(app);
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, MILK)
        .click_secondary();
    harness.run();
    press(&mut harness, "Add to Note…")?;
    harness.run_steps(20);
    let t = *harness.state().t();
    on_the_band(&harness, t.search_all_hint(), SAUCE);
    Ok(())
}

#[test]
fn link_tos_targets_scroll_under_its_searchs_line() -> TestResult {
    let (store, _) = store()?;
    let store = Arc::new(store);
    let note = Notes::new(&store).capture_as("see ", "plain", None, None)?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    app.select_note(&note.id);
    let mut harness = window(app);
    harness
        .query_all_by_role_and_label(Role::Button, "Link to…")
        .next()
        .ok_or("Link to…")?
        .click_accesskit();
    harness.run();
    let t = *harness.state().t();
    on_the_band(&harness, t.link_to_search(), t.link_to_pages());
    Ok(())
}
