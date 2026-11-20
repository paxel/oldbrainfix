//! Add to Note… on the desktop (#308), driven headless by egui_kittest:
//! an Unsorted Note's card offers it in place of Merge into…, and the
//! Note picked gets it merged in as a Section. The quick note offers it
//! no longer (#352): its More opens the Note dialog, whose Merge into…
//! adds the capture to another Note.

use chrono::{DateTime, Utc};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, Note, Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SAUCE: &str = "Tomato sauce";
const PASTA: &str = "Pasta for Friday";
const MILK: &str = "Buy milk";

/// "Tomato sauce" filed in Household, "Pasta for Friday" Unsorted, and
/// "Buy milk" in the trash.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let now = DateTime::parse_from_rfc3339("2026-11-15T09:00:00Z")?.with_timezone(&Utc);
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([8; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let house = Places::new(&store).add_area("Household")?;
    let notes = Notes::new(&store);
    notes.capture(SAUCE, Some(&house.id), None)?;
    notes.capture(PASTA, None, None)?;
    let milk = notes.capture(MILK, None, None)?;
    notes.trash(&milk.id)?;
    Ok(store)
}

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

/// The window's quick note, open, with `text` typed.
fn quick_note(app: App, text: &str) -> Harness<'static, App> {
    let mut harness = window(app);
    harness.state_mut().new_note();
    harness.run();
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    type_here(&mut harness, text);
    harness
}

/// Types `text` where the keyboard is.
fn type_here<S>(harness: &mut Harness<'_, S>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Clicks the last button named `label`, the one over the rest.
fn press<S>(harness: &mut Harness<'_, S>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next_back()
        .ok_or_else(|| format!("no {label:?}"))?
        .click_accesskit();
    harness.run_steps(8);
    Ok(())
}

/// Picks the Note `text` in Add to Note…, drawn after its card.
fn pick<S>(harness: &mut Harness<'_, S>, text: &str) -> TestResult {
    harness
        .query_all_by_label(text)
        .next_back()
        .ok_or_else(|| format!("no {text:?} to pick"))?
        .click();
    harness.run();
    Ok(())
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The Note `text` in the lists.
fn note(store: &Store, text: &str) -> Result<Note, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .find(|n| n.text == text)
        .ok_or_else(|| format!("no {text:?}"))?)
}

/// The texts of the Notes left Unsorted.
fn unsorted(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .all()?
        .into_iter()
        .filter(Note::is_unsorted)
        .map(|n| n.text)
        .collect())
}

#[test]
fn the_quick_notes_more_leads_to_merge_into_another_note() -> TestResult {
    // The quick note no longer adds to another Note itself (#352): More
    // saves it and opens the Note dialog, whose Merge into… does it.
    let store = store()?;
    let mut harness = quick_note(App::new(store, "en"), "more garlic");
    assert!(!shows(&harness, "Add to Note…"));
    press(&mut harness, "More")?;
    assert!(!harness.state().quick_note_open());
    press(&mut harness, "Merge into…")?;
    type_here(&mut harness, "sauce");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::Enter);
    harness.run();

    let store = harness.state().store();
    let sauce = note(store, SAUCE)?;
    let sections = Notes::new(store).sections(&sauce.id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].text, "more garlic");
    assert_eq!(unsorted(store)?, [PASTA]);
    Ok(())
}

#[test]
fn an_unsorted_cards_right_click_offers_add_to_note_in_place_of_merge_into() -> TestResult {
    let store = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let mut harness = window(app);
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, PASTA)
        .click_secondary();
    harness.run();
    assert!(shows(&harness, "Add to Note…"));
    assert!(!shows(&harness, "Merge into…"));
    harness
        .query_all_by_label("Add to Note…")
        .next_back()
        .ok_or("Add to Note… in the menu")?
        .click();
    harness.run_steps(8);
    pick(&mut harness, SAUCE)?;

    let store = harness.state().store();
    let sauce = note(store, SAUCE)?;
    // The core's Merge, as Merge into… does.
    let pasta = Notes::new(store)
        .sections(&sauce.id)?
        .into_iter()
        .next()
        .ok_or("Pasta merged")?;
    assert_eq!(pasta.text, PASTA);
    assert!(unsorted(store)?.is_empty());
    assert_eq!(DeviceSettings::new(store).recently_added_to()?, [sauce.id]);
    // A filed Note's card keeps Merge into….
    harness.run_steps(50);
    harness
        .get_by_role_and_label(Role::Button, SAUCE)
        .click_secondary();
    harness.run();
    assert!(shows(&harness, "Merge into…"));
    assert!(!shows(&harness, "Add to Note…"));
    Ok(())
}
