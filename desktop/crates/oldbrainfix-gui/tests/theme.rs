//! The window's theme driven headless (#147): it follows the light or
//! dark the system reports, View ▸ Theme shows System, Light or Dark at
//! once and keeps the choice on this Device, and a new window starts in
//! the one kept.

use std::sync::Arc;

use egui::Theme;
use egui::accesskit::Toggled;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{DeviceSettings, SigningKey, Store, ThemeChoice};
use oldbrainfix_gui::App;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    Ok(Arc::new(store))
}

/// The window over `store`, on a system that reports `system`.
fn window(store: &Arc<Store>, system: Theme) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            App::new(store.clone(), "en"),
        );
    harness.input_mut().system_theme = Some(system);
    harness.run();
    harness
}

/// Whether the window draws dark, by its visuals.
fn dark(harness: &Harness<'_, App>) -> bool {
    harness.ctx.global_style().visuals.dark_mode
}

/// The last widget labelled `label`: a menu's entry, drawn over the
/// window's buttons of the same name.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

/// Opens View ▸ Theme.
fn open_theme(harness: &mut Harness<'_, App>) {
    match harness.get_all_by_label("View").next() {
        Some(view) => view.click(),
        None => panic!("no menu View"),
    }
    harness.run();
    last(harness, "Theme ⏵").hover();
    harness.run();
}

/// Picks `entry` in View ▸ Theme.
fn pick(harness: &mut Harness<'_, App>, entry: &str) {
    open_theme(harness);
    last(harness, entry).click();
    harness.run();
}

fn kept(store: &Store) -> Result<ThemeChoice, Box<dyn std::error::Error>> {
    Ok(DeviceSettings::new(store).theme()?)
}

#[test]
fn the_window_follows_the_light_or_dark_the_system_reports() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Theme::Light);
    assert_eq!(harness.state().theme(), ThemeChoice::System);
    assert!(!dark(&harness));
    harness.input_mut().system_theme = Some(Theme::Dark);
    harness.run();
    assert!(dark(&harness));
    harness.input_mut().system_theme = Some(Theme::Light);
    harness.run();
    assert!(!dark(&harness));
    // System is the one marked.
    open_theme(&mut harness);
    assert_eq!(
        last(&harness, "System").accesskit_node().toggled(),
        Some(Toggled::True)
    );
    assert_eq!(
        last(&harness, "Dark").accesskit_node().toggled(),
        Some(Toggled::False)
    );
    Ok(())
}

#[test]
fn view_theme_shows_light_or_dark_at_once_and_keeps_it_on_this_device() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Theme::Light);
    pick(&mut harness, "Dark");
    assert!(dark(&harness));
    assert_eq!(harness.state().theme(), ThemeChoice::Dark);
    assert_eq!(kept(&store)?, ThemeChoice::Dark);
    // The system's setting no longer counts.
    harness.input_mut().system_theme = Some(Theme::Light);
    harness.run();
    assert!(dark(&harness));
    open_theme(&mut harness);
    assert_eq!(
        last(&harness, "Dark").accesskit_node().toggled(),
        Some(Toggled::True)
    );
    assert_eq!(
        last(&harness, "System").accesskit_node().toggled(),
        Some(Toggled::False)
    );
    last(&harness, "Light").click();
    harness.run();
    assert!(!dark(&harness));
    assert_eq!(kept(&store)?, ThemeChoice::Light);
    harness.input_mut().system_theme = Some(Theme::Dark);
    harness.run();
    assert!(!dark(&harness));
    // Back to the system's.
    pick(&mut harness, "System");
    assert!(dark(&harness));
    assert_eq!(kept(&store)?, ThemeChoice::System);
    Ok(())
}

#[test]
fn a_new_window_starts_in_the_theme_kept() -> TestResult {
    let store = store()?;
    DeviceSettings::new(&store).set_theme(ThemeChoice::Dark)?;
    let harness = window(&store, Theme::Light);
    assert_eq!(harness.state().theme(), ThemeChoice::Dark);
    assert!(dark(&harness));
    drop(harness);
    DeviceSettings::new(&store).set_theme(ThemeChoice::Light)?;
    let harness = window(&store, Theme::Dark);
    assert!(!dark(&harness));
    Ok(())
}
