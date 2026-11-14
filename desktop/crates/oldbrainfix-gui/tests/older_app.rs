//! The view in place of the window when an older app did not make way
//! for an update (#290), driven headless: it says so and what to do, in
//! the app's language, and offers nothing else.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_gui::App;
use oldbrainfix_gui::older_app::OlderAppView;

fn harness(locale: &str) -> Harness<'static, OlderAppView> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 360.0))
        .build_ui_state(
            |ui, view: &mut OlderAppView| {
                App::install(ui.ctx());
                view.show(ui);
            },
            OlderAppView::new(locale),
        );
    harness.run();
    harness
}

#[test]
fn it_says_the_older_app_still_runs_and_what_to_do() {
    let harness = harness("en");
    harness
        .get_by_label("An older Old Brain Fix is still running and did not end for this update.");
    harness.get_by_label("Choose Quit in its tray menu, then start Old Brain Fix again.");
    assert!(
        harness
            .query_all_by(|n| matches!(n.role(), Role::Button | Role::TextInput))
            .next()
            .is_none()
    );
}

#[test]
fn it_speaks_german_too() {
    let harness = harness("de");
    harness.get_by_label(
        "Wähle Beenden in seinem Menü im Infobereich und starte Old Brain Fix dann erneut.",
    );
}
