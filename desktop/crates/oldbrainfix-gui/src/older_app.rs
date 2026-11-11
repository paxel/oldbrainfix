//! What a start shows when an older app it asked to make way for an
//! update is still running (#290): one view that says so and what to do,
//! and nothing else. The launcher shows it in place of the window, the
//! tray and the capture popup, starts nothing more and opens no
//! database; the command line prints its words.

use egui::{Frame, Margin, RichText, Ui};

use crate::l10n::L10n;

/// The view, in the app's language.
pub struct OlderAppView {
    t: L10n,
}

impl OlderAppView {
    pub fn new(locale: &str) -> OlderAppView {
        OlderAppView {
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
                    ui.label(RichText::new(self.t.older_app_stays()).heading());
                    ui.add_space(8.0);
                    ui.label(RichText::new(self.t.older_app_stays_fix()).color(weak));
                });
            });
    }
}
