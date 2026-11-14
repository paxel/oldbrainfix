//! Hidden from AI on the desktop (#267, ADR 0049): a card's menu, an
//! Area's right-click and the Area menu mark a Note or Area, offered in
//! no Shared Area; a shield on the card and the sidebar's row shows what
//! is hidden, the Note dialog's badge "Off, hidden by its Area"; and
//! filing, sending, sharing or packing something hidden asks to show it
//! to the AI first, a Note hidden by its Area only naming the Area.
//! Driven headless by egui_kittest.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Notes, Places, SigningKey, Store, parse_time};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MILK: &str = "Buy milk";

/// Household with "Buy milk" in it, and the Shared Area Club; the ids
/// of the Note, Household and Club.
fn store() -> Result<(Store, [String; 3]), Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-13T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([3; 32])))?;
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    let club = places.add_area("Club")?;
    let club = store.share(&club.id)?;
    let milk = Notes::new(&store).capture(MILK, Some(&house.id), None)?;
    Ok((store, [milk.id, house.id, club]))
}

fn harness(store: Store) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(Selection::All);
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

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Lets egui's time for a double-click run out.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// Opens the menu of the card of "Buy milk".
fn card_menu(harness: &mut Harness<'_, App>) {
    pause(harness);
    harness
        .get_by_role_and_label(Role::Button, MILK)
        .click_secondary();
    harness.run();
}

/// Opens the right-click menu of the sidebar's row `name`.
fn row_menu(harness: &mut Harness<'_, App>, name: &str) {
    harness
        .query_all_by_label(name)
        .next()
        .ok_or_else(|| format!("no row {name}"))
        .unwrap()
        .click_secondary();
    harness.run();
}

/// The switch "Hidden from AI" of a menu open.
fn switch<'a>(harness: &'a Harness<'_, App>) -> Option<egui_kittest::Node<'a>> {
    harness
        .query_all_by_role_and_label(Role::CheckBox, "Hidden from AI")
        .next()
}

/// How many shields show.
fn shields(harness: &Harness<'_, App>) -> usize {
    harness
        .query_all_by_role_and_label(Role::Label, "Hidden from AI")
        .count()
}

/// Files "Buy milk" into Club with File into….
fn file_into_club(harness: &mut Harness<'_, App>) {
    pause(harness);
    harness.get_by_role_and_label(Role::Button, MILK).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
    harness.get_by(|n| n.is_focused()).type_text("Club");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

const BODY: &str = "Buy milk: hidden from AI. What is hidden from AI is not given to other people. Show it to the AI first.";

#[test]
fn a_cards_menu_marks_the_note_and_a_shield_shows_it() -> TestResult {
    let (store, [milk, ..]) = store()?;
    let mut harness = harness(store);
    assert_eq!(shields(&harness), 0);
    card_menu(&mut harness);
    switch(&harness).ok_or("the switch")?.click_accesskit();
    harness.run();
    assert!(harness.state().store().is_marked_hidden_from_ai(&milk)?);
    assert_eq!(shields(&harness), 1);
    // Ticked as the menu opens again; its click takes the mark off.
    card_menu(&mut harness);
    let on = switch(&harness).ok_or("the switch")?;
    assert_eq!(
        on.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    on.click_accesskit();
    harness.run();
    assert!(!harness.state().store().is_marked_hidden_from_ai(&milk)?);
    assert_eq!(shields(&harness), 0);
    Ok(())
}

#[test]
fn an_areas_mark_hides_its_notes_and_is_not_offered_in_a_shared_area() -> TestResult {
    let (store, [milk, house, _]) = store()?;
    let mut harness = harness(store);
    row_menu(&mut harness, "Club");
    assert!(switch(&harness).is_none());
    harness.key_press(Key::Escape);
    harness.run();
    row_menu(&mut harness, "Household");
    switch(&harness).ok_or("the switch")?.click_accesskit();
    harness.run();
    assert!(harness.state().store().is_marked_hidden_from_ai(&house)?);
    // The row's shield and the card's, its Area's mark hiding it.
    assert_eq!(shields(&harness), 2);
    // The Note dialog's badge says why it is hidden.
    harness.state_mut().select_note(&milk);
    harness.run();
    assert!(shows(&harness, "Hidden from AI: Off, hidden by its Area"));
    // The Area menu offers the same switch for the Area shown.
    harness.state_mut().close_note();
    harness.state_mut().select(Selection::Place(house.clone()));
    harness.run();
    harness.get_by_label("Area").click();
    harness.run();
    let on = switch(&harness).ok_or("the Area menu's switch")?;
    assert_eq!(
        on.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    on.click_accesskit();
    harness.run();
    assert!(!harness.state().store().is_marked_hidden_from_ai(&house)?);
    Ok(())
}

#[test]
fn filing_a_hidden_note_into_a_shared_area_asks_to_show_it_to_the_ai() -> TestResult {
    let (store, [milk, house, club]) = store()?;
    store.set_hidden_from_ai(&milk, true)?;
    let mut harness = harness(store);
    file_into_club(&mut harness);
    assert!(shows(&harness, BODY));
    let place = |h: &Harness<'_, App>| -> Option<String> {
        Notes::new(h.state().store())
            .by_id(&milk)
            .ok()
            .flatten()
            .and_then(|n| n.place)
    };
    // Cancel gives nothing and keeps the mark.
    harness.get_by_label("Cancel").click_accesskit();
    harness.run();
    assert_eq!(place(&harness), Some(house.clone()));
    assert!(harness.state().store().is_marked_hidden_from_ai(&milk)?);
    // "Show to the AI" takes the mark off and files.
    file_into_club(&mut harness);
    harness.get_by_label("Show to the AI").click_accesskit();
    harness.run();
    assert_eq!(place(&harness), Some(club));
    assert!(!harness.state().store().is_marked_hidden_from_ai(&milk)?);
    Ok(())
}

#[test]
fn a_note_hidden_by_its_area_only_names_the_area() -> TestResult {
    let (store, [milk, house, _]) = store()?;
    store.set_hidden_from_ai(&house, true)?;
    let mut harness = harness(store);
    file_into_club(&mut harness);
    assert!(shows(
        &harness,
        "Household: hidden from AI. A Note in an Area hidden from AI is not given to other people. Take the mark off in the Area's menu first."
    ));
    // One click never shows the Area's other Notes to the AI.
    assert!(!shows(&harness, "Show to the AI"));
    harness.get_by_label("Close").click_accesskit();
    harness.run();
    assert!(harness.state().store().is_marked_hidden_from_ai(&house)?);
    let note = Notes::new(harness.state().store())
        .by_id(&milk)?
        .ok_or("milk")?;
    assert_eq!(note.place, Some(house));
    Ok(())
}

#[test]
fn sending_sharing_and_packing_ask_before_anything_else() -> TestResult {
    let (store, [milk, house, _]) = store()?;
    store.set_hidden_from_ai(&milk, true)?;
    let mut harness = harness(store);
    // Send asks before whom.
    card_menu(&mut harness);
    harness.get_by_label("Send to a person").click_accesskit();
    harness.run();
    assert!(shows(&harness, BODY));
    harness.get_by_label("Cancel").click_accesskit();
    harness.run();
    // Export as Pack… asks before the tree.
    card_menu(&mut harness);
    harness.get_by_label("Export as Pack…").click_accesskit();
    harness.run();
    assert!(shows(&harness, BODY));
    harness.get_by_label("Show to the AI").click_accesskit();
    harness.run();
    assert!(!harness.state().store().is_marked_hidden_from_ai(&milk)?);
    assert!(shows(&harness, "Tick all"));
    harness.key_press(Key::Escape);
    harness.run();
    // Sharing an Area holding a hidden Note asks before whom, naming it.
    harness.state().store().set_hidden_from_ai(&milk, true)?;
    harness.state_mut().select(Selection::Place(house.clone()));
    harness.run();
    row_menu(&mut harness, "Household");
    harness.get_by_label("Share…").click_accesskit();
    harness.run();
    assert!(shows(&harness, BODY));
    Ok(())
}

/// "Club rota" in Club, and "Bread" beside "Buy milk" in Household,
/// "Buy milk" hidden; the ids of Buy milk, Bread and Club rota.
fn to_merge() -> Result<(Store, [String; 3]), Box<dyn std::error::Error>> {
    let (store, [milk, house, club]) = store()?;
    let notes = Notes::new(&store);
    let bread = notes.capture("Bread", Some(&house), None)?;
    let rota = notes.capture("Club rota", Some(&club), None)?;
    store.set_hidden_from_ai(&milk, true)?;
    Ok((store, [milk, bread.id, rota.id]))
}

fn merged(harness: &Harness<'_, App>, id: &str) -> bool {
    Notes::new(harness.state().store())
        .by_id(id)
        .ok()
        .flatten()
        .is_some_and(|n| n.is_merged())
}

/// Picks "Club rota" in the Merge dialog open.
fn pick_rota(harness: &mut Harness<'_, App>) -> TestResult {
    harness
        .query_all_by_label("Club rota")
        .next_back()
        .ok_or("Club rota to pick")?
        .click_accesskit();
    harness.run();
    Ok(())
}

#[test]
fn merging_a_hidden_note_with_one_of_a_shared_area_asks_and_merges_nothing() -> TestResult {
    let (store, [milk, bread, rota]) = to_merge()?;
    let mut harness = harness(store);
    // The cards selected: none merges, the selection stays.
    pause(&mut harness);
    harness.get_by_role_and_label(Role::Button, MILK).click();
    harness.run();
    pause(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Bread")
        .click_modifiers(Modifiers::COMMAND);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::M);
    harness.run_steps(8);
    pick_rota(&mut harness)?;
    assert!(shows(&harness, BODY));
    assert!(!merged(&harness, &milk) && !merged(&harness, &bread));
    assert_eq!(harness.state().selected_cards().len(), 2);
    harness.get_by_label("Cancel").click_accesskit();
    harness.run();
    // Nothing waits to be undone.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert!(!merged(&harness, &milk) && !merged(&harness, &bread));
    // From the Note dialog, the other way round: Club rota into the
    // hidden Note.
    harness.key_press(Key::Escape);
    harness.run();
    harness.state_mut().select_note(&rota);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::M);
    harness.run_steps(8);
    harness
        .query_all_by_label(MILK)
        .next_back()
        .ok_or("Buy milk to pick")?
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, BODY));
    assert!(!merged(&harness, &rota));
    harness.get_by_label("Show to the AI").click_accesskit();
    harness.run();
    assert!(merged(&harness, &rota));
    assert!(!harness.state().store().is_marked_hidden_from_ai(&milk)?);
    Ok(())
}

#[test]
fn a_note_in_a_shared_area_offers_no_mark() -> TestResult {
    let (store, [_, _, rota]) = to_merge()?;
    let mut harness = harness(store);
    pause(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Club rota")
        .click_secondary();
    harness.run();
    assert!(switch(&harness).is_none());
    harness.key_press(Key::Escape);
    harness.run();
    harness.state_mut().select_note(&rota);
    harness.run();
    assert!(!shows(&harness, "Hidden from AI: Off"));
    assert!(!shows(&harness, "Hidden from AI: On"));
    Ok(())
}
