//! What the desktop shows in place of the window when this Device's
//! database was saved by a newer version (ADR 0034): one view that says
//! so, and nothing else. The launcher shows it for every start, the tray's
//! and the capture popup's too, starts nothing more, and leaves the
//! database as it was; the command line prints its words.

use egui::{Frame, Margin, RichText, Ui};

use crate::l10n::L10n;

/// The view, in the app's language.
pub struct StoreNeedsUpdateView {
    t: L10n,
}

impl StoreNeedsUpdateView {
    pub fn new(locale: &str) -> StoreNeedsUpdateView {
        StoreNeedsUpdateView {
            t: L10n::new(locale),
        }
    }

    /// The strings it speaks with, for the window's title.
    pub fn t(&self) -> &L10n {
        &self.t
    }

    pub fn show(&self, ui: &mut Ui) {
        let weak = ui.visuals().weak_text_color();
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::same(24)))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.label(RichText::new(self.t.store_needs_update()).heading());
                    ui.add_space(8.0);
                    ui.label(RichText::new(self.t.store_needs_update_unchanged()).color(weak));
                });
            });
    }
}
