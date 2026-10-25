//! The About view driven headless: reached from the Settings, it links
//! the documentation site and its privacy policy, and opens the licences
//! page, which lists every third-party component and opens its licence on
//! a click.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn harness(locale: &str) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?;
    let mut app = App::new(store, locale);
    app.select(Selection::Settings);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
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

#[test]
fn the_licences_open_from_the_settings_and_list_every_component() -> TestResult {
    let mut harness = harness("en")?;
    harness
        .get_by_label("About Old Brain Fix")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::About);
    assert!(shows(&harness, "Version 0.1.0"));
    harness.get_by_label("Documentation");
    harness.get_by_label("Privacy policy");
    harness
        .get_by_label("Open-source licenses")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Licences);
    for name in [
        "ocrs models",
        "OpenAI Whisper base model",
        "whisper.cpp",
        "Material Icons",
        "SQLite",
        "Noto Emoji font",
        "ocrs 0.",
        "egui 0.",
    ] {
        assert!(shows(&harness, name), "{name}");
    }
    // The header naming the most crates wraps inside the window.
    let widest = harness
        .query_all_by_label_contains("windows-sys 0.")
        .map(|n| n.rect().max.x)
        .fold(0.0, f32::max);
    assert!(widest > 0.0 && widest <= 1280.0, "{widest}");
    assert!(!shows(&harness, "Copyright (c) 2022 OpenAI"));
    harness
        .get_by_label_contains("OpenAI Whisper base model")
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "Copyright (c) 2022 OpenAI"));
    Ok(())
}

#[test]
fn the_about_view_speaks_german() -> TestResult {
    let mut harness = harness("de")?;
    harness.get_by_label("Über Old Brain Fix").click_accesskit();
    harness.run();
    harness.get_by_label("Dokumentation");
    harness.get_by_label("Datenschutzerklärung");
    harness
        .get_by_label("Open-Source-Lizenzen")
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "Jede Fremdkomponente in dieser App"));
    assert!(shows(&harness, "ocrs models"));
    Ok(())
}
