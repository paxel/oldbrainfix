//! The view in place of the window for a database a newer version saved
//! (ADR 0034), driven headless: it says so in the app's language and
//! offers nothing else, no capture field and no button.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_gui::App;
use oldbrainfix_gui::store_needs_update::StoreNeedsUpdateView;

fn harness(locale: &str) -> Harness<'static, StoreNeedsUpdateView> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 360.0))
        .build_ui_state(
            |ui, view: &mut StoreNeedsUpdateView| {
                App::install(ui.ctx());
                view.show(ui);
            },
            StoreNeedsUpdateView::new(locale),
        );
    harness.run();
    harness
}

#[test]
fn it_says_the_data_is_of_a_newer_version_and_offers_nothing_else() {
    let harness = harness("en");
    harness.get_by_label(
        "This Device's data was saved by a newer Old Brain Fix; install that version again.",
    );
    harness.get_by_label("Nothing on this Device was changed.");
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
        "Die Daten dieses Geräts wurden von einer neueren Version von Old Brain Fix gespeichert; installiere diese Version wieder.",
    );
}
