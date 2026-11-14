//! The search at the top of the Settings dialog (#282): it shows the
//! settings that hold every word typed, in any case and in the language
//! on screen, under their card's heading, a whole card when its heading
//! matches, and says so when none does; Ctrl+F puts the cursor into it,
//! nothing on opening; ✕ and opening the Settings anew show everything.

// A test may panic, its helpers too (#229).
#![allow(clippy::panic)]

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{DeviceSettings, Notes, SigningKey, Store};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store with the Unsorted Note "Call the bank".
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    Notes::new(&store).capture("Call the bank", None, None)?;
    Ok(store)
}

/// The window in `language` over the Unsorted list with the Settings
/// open.
fn settings(language: &str) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store()?, language);
    app.select(Selection::Unsorted);
    app.open(Page::Settings);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    Ok(harness)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The search field of the Settings, by the name a screen reader says.
fn field<'a>(harness: &'a Harness<'_, App>, name: &'a str) -> egui_kittest::Node<'a> {
    match harness
        .query_all_by(|n| n.role() == Role::TextInput && n.label().as_deref() == Some(name))
        .next()
    {
        Some(node) => node,
        None => panic!("no field {name}"),
    }
}

/// Types `text` into the empty search field.
fn search(harness: &mut Harness<'_, App>, text: &str) {
    let name = harness.state().t().settings_search_hint();
    field(harness, name).click();
    harness.run();
    field(harness, name).type_text(text);
    harness.run();
}

/// The label of the widget with the keyboard.
fn focused(harness: &Harness<'_, App>) -> Option<String> {
    let node = harness.query_by(|n| n.is_focused())?.accesskit_node();
    node.label()
        .or_else(|| node.placeholder().map(str::to_string))
}

#[test]
fn every_word_typed_must_be_in_a_settings_label_or_detail_line_in_any_case() -> TestResult {
    let mut harness = settings("en")?;
    assert!(shows(&harness, "Bigger text and icons"));
    assert!(shows(&harness, "Quiet Hours"));
    assert!(shows(&harness, "belong to this computer"));
    // "login" in the label, "brain" in the detail line of one setting.
    search(&mut harness, "LOGIN Brain");
    assert!(shows(&harness, "Start at login"));
    assert!(shows(&harness, "This computer"));
    assert!(!shows(&harness, "Bigger text and icons"));
    assert!(!shows(&harness, "Notifications on this computer"));
    assert!(!shows(&harness, "Quiet Hours"));
    assert!(!shows(&harness, "Alarms"));
    // The intro line goes while a search runs.
    assert!(!shows(&harness, "belong to this computer"));
    assert!(!shows(&harness, "No setting matches"));
    Ok(())
}

#[test]
fn words_from_two_settings_find_neither_and_the_dialog_says_so() -> TestResult {
    let mut harness = settings("en")?;
    search(&mut harness, " login quiet ");
    assert!(!shows(&harness, "Start at login"));
    assert!(!shows(&harness, "Quiet Hours"));
    assert!(shows(&harness, "No setting matches “login quiet”."));
    // ✕ shows everything again.
    harness.get_by_label("Clear search").click();
    harness.run();
    assert!(shows(&harness, "Start at login"));
    assert!(shows(&harness, "Quiet Hours"));
    assert!(shows(&harness, "belong to this computer"));
    assert!(!shows(&harness, "No setting matches"));
    Ok(())
}

#[test]
fn a_heading_that_matches_shows_its_card_whole() -> TestResult {
    let mut harness = settings("en")?;
    search(&mut harness, "nudges");
    assert!(shows(&harness, "Nudge again after"));
    assert!(shows(&harness, "Quiet Hours"));
    assert!(shows(&harness, "Until"));
    // "Nudge buttons" of the quick note holds no "nudges".
    assert!(!shows(&harness, "Nudge buttons"));
    assert!(!shows(&harness, "Start at login"));
    harness.get_by_label("Clear search").click();
    harness.run();
    // A word of one switch of the quick note: its card, that switch.
    search(&mut harness, "nudge buttons");
    assert!(shows(&harness, "Quick note"));
    assert!(shows(&harness, "Nudge buttons"));
    assert!(!shows(&harness, "File into"));
    assert!(!shows(&harness, "Nudge again after"));
    Ok(())
}

#[test]
fn only_the_language_on_screen_is_searched() -> TestResult {
    let mut harness = settings("de")?;
    search(&mut harness, "login");
    assert!(shows(&harness, "Keine Einstellung passt zu „login“."));
    harness.get_by_label("Suche leeren").click();
    harness.run();
    search(&mut harness, "ANMELDEN");
    assert!(shows(&harness, "Beim Anmelden starten"));
    assert!(!shows(&harness, "Ruhezeit"));
    Ok(())
}

#[test]
fn a_setting_found_still_switches() -> TestResult {
    let mut harness = settings("en")?;
    search(&mut harness, "bigger");
    assert!(!DeviceSettings::new(harness.state().store()).fat_fingers()?);
    harness
        .get_by_role_and_label(Role::CheckBox, "Bigger text and icons")
        .click();
    harness.run();
    assert!(DeviceSettings::new(harness.state().store()).fat_fingers()?);
    Ok(())
}

#[test]
fn ctrl_f_puts_the_cursor_into_the_search_and_opening_does_not() -> TestResult {
    let mut harness = settings("en")?;
    assert_eq!(focused(&harness), None);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some("Search Settings"));
    // The list beneath stays as it was, its search untouched.
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    assert_eq!(harness.state().page(), Some(Page::Settings));
    Ok(())
}

#[test]
fn the_search_stays_into_about_and_back_and_goes_when_the_settings_open_anew() -> TestResult {
    let mut harness = settings("en")?;
    search(&mut harness, "about");
    harness
        .get_by_role_and_label(Role::Button, "About Old Brain Fix")
        .click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::About));
    harness.get_by_label("Back to Settings").click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::Settings));
    assert!(!shows(&harness, "Start at login"));
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().page(), None);
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert!(shows(&harness, "Start at login"));
    assert!(shows(&harness, "About Old Brain Fix"));
    Ok(())
}
