//! A card starts with its text (#330), driven headless by egui_kittest:
//! one top row holds the check mark, the place and the buttons on the
//! right, no divider under it, and the text starts in the second row. In
//! the list of an Area or Topic the place is left out and the text starts
//! beside the buttons; All Notes, a search, Unsorted and the trash name
//! it.

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
use egui::{Modifiers, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MILK: &str = "Buy milk";
const TAP: &str = "Fix the tap";
const ROSES: &str = "Water the roses";

/// Household with its Topic Kitchen: Buy milk in the Kitchen, Fix the
/// tap in Household itself and Water the roses Unsorted. Returns the ids
/// of Household and Kitchen.
fn store() -> Result<(Store, String, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([1; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    let kitchen = places.add_topic(&house.id, "Kitchen")?;
    let notes = Notes::new(&store);
    notes.capture(MILK, Some(&kitchen.id), None)?;
    notes.capture(TAP, Some(&house.id), None)?;
    notes.capture(ROSES, None, None)?;
    Ok((store, house.id, kitchen.id))
}

fn harness(store: Store, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
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

/// Where the card of the Note `text` lies.
fn card(harness: &Harness<'_, App>, text: &str) -> Rect {
    harness.get_by_role_and_label(Role::Button, text).rect()
}

/// Where the label `label` lies on the card of `text`, if it shows there.
fn on_card(harness: &Harness<'_, App>, text: &str, label: &str) -> Option<Rect> {
    let card = card(harness, text);
    harness
        .query_all_by_role_and_label(Role::Label, label)
        .map(|n| n.rect())
        .find(|r| card.contains_rect(*r))
}

/// Where the card of `text` has its Edit button.
fn edit(harness: &Harness<'_, App>, text: &str) -> Rect {
    let card = card(harness, text);
    harness
        .query_all_by_role_and_label(Role::Button, "Edit")
        .map(|n| n.rect())
        .find(|r| card.contains_rect(*r))
        .unwrap_or_else(|| panic!("no Edit on {text:?}"))
}

/// Where the text of the card of `text` lies.
fn text_of(harness: &Harness<'_, App>, text: &str) -> Rect {
    on_card(harness, text, text).unwrap_or_else(|| panic!("no text on {text:?}"))
}

#[test]
fn all_notes_names_the_place_beside_the_buttons_and_the_text_follows_without_a_divider()
-> TestResult {
    let (store, _, _) = store()?;
    let harness = harness(store, Selection::All);
    let place = on_card(&harness, MILK, "Household / Kitchen").ok_or("no place")?;
    let edit = edit(&harness, MILK);
    // One row: the place in line with the buttons on the right.
    assert!(
        place.center().y > edit.top() && place.center().y < edit.bottom(),
        "place {place:?} beside Edit {edit:?}"
    );
    assert!(place.right() < edit.left());
    // The text in the second row, right under it: no divider between.
    let text = text_of(&harness, MILK);
    assert!(
        text.top() >= place.bottom(),
        "text {text:?} under {place:?}"
    );
    assert!(
        text.top() - edit.bottom() <= 20.0,
        "text {text:?} right under the top row {edit:?}"
    );
    // An Unsorted Note says so in All Notes.
    assert!(on_card(&harness, ROSES, "Unsorted").is_some());
    Ok(())
}

#[test]
fn an_areas_list_leaves_out_the_place_and_its_text_starts_beside_the_buttons() -> TestResult {
    let (store, house, _) = store()?;
    let harness = harness(store, Selection::Place(house));
    assert!(on_card(&harness, TAP, "Household").is_none());
    let text = text_of(&harness, TAP);
    let edit = edit(&harness, TAP);
    assert!(text.top() < edit.bottom(), "text {text:?} beside {edit:?}");
    assert!(text.right() < edit.left());
    Ok(())
}

#[test]
fn a_topics_list_leaves_out_the_place() -> TestResult {
    let (store, _, kitchen) = store()?;
    let harness = harness(store, Selection::Place(kitchen));
    assert!(on_card(&harness, MILK, "Household / Kitchen").is_none());
    assert!(text_of(&harness, MILK).top() < edit(&harness, MILK).bottom());
    Ok(())
}

#[test]
fn an_areas_search_names_the_topic_of_a_hit_in_one() -> TestResult {
    let (store, house, _) = store()?;
    let mut harness = harness(store, Selection::Place(house));
    // Without a search the Kitchen's Notes show inside the Kitchen.
    assert!(
        harness
            .query_by_role_and_label(Role::Button, MILK)
            .is_none()
    );
    harness.state_mut().set_query("Buy");
    harness.run();
    assert!(on_card(&harness, MILK, "Household / Kitchen").is_some());
    harness.state_mut().set_query("Fix");
    harness.run();
    assert!(on_card(&harness, TAP, "Household").is_none());
    Ok(())
}

#[test]
fn unsorted_and_the_trash_name_the_place() -> TestResult {
    let (store, _, _) = store()?;
    let notes = Notes::new(&store);
    let tap = notes
        .all()?
        .into_iter()
        .find(|n| n.text == TAP)
        .ok_or("no tap")?;
    notes.trash(&tap.id)?;
    let mut harness = harness(store, Selection::Unsorted);
    assert!(on_card(&harness, ROSES, "Unsorted").is_some());
    harness.state_mut().select(Selection::Trash);
    harness.run();
    assert!(on_card(&harness, TAP, "Household").is_some());
    Ok(())
}

#[test]
fn selecting_a_card_along_moves_none_of_its_text() -> TestResult {
    let (store, house, _) = store()?;
    let mut harness = harness(store, Selection::Place(house));
    let before = text_of(&harness, TAP);
    harness
        .get_by_role_and_label(Role::Button, TAP)
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    assert_eq!(harness.state().selected_cards().len(), 1, "selected along");
    assert_eq!(text_of(&harness, TAP), before);
    Ok(())
}

#[test]
fn an_important_cards_marker_stands_clear_of_its_buttons() -> TestResult {
    let (store, house, _) = store()?;
    let notes = Notes::new(&store);
    let tap = notes.all()?.into_iter().find(|n| n.text == TAP).unwrap().id;
    notes.set_importance(&tap, oldbrainfix_core::Importance::Important)?;
    let harness = harness(store, Selection::Place(house));
    let card = card(&harness, TAP);
    let pencil = edit(&harness, TAP);
    // The marker's glyph, on the card (obf-1139).
    let glyph = oldbrainfix_gui::icons::IMPORTANT.to_string();
    let mut shapes = Vec::new();
    for clipped in &harness.output().shapes {
        shapes.push(clipped.shape.clone());
    }
    let marker = shapes
        .iter()
        .flat_map(|s| match s {
            egui::Shape::Vec(inner) => inner.clone(),
            other => vec![other.clone()],
        })
        .find_map(|s| match s {
            egui::Shape::Text(text) if text.galley.text() == glyph => {
                Some(text.visual_bounding_rect())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no Important marker"));
    assert!(card.contains_rect(marker), "the marker {marker:?}");
    assert!(
        pencil.right() <= marker.left(),
        "the pencil {pencil:?} under the marker {marker:?}"
    );
    Ok(())
}
