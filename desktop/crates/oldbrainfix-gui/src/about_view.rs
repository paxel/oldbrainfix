//! The About view and its licences page: the version and the source, and
//! every third-party component in the desktop app with its licence, read
//! from the lists in `licenses/` at the root of the repository (the
//! crates' list written by `tool/licenses.dart`).

use std::sync::LazyLock;

use egui::{CollapsingHeader, RichText, Ui};

use crate::app::embossed;
use crate::l10n::L10n;
use crate::theme::Tones;

const REPO: &str = "https://github.com/paxel/oldbrainfix";
const SITE: &str = "https://paxel.github.io/oldbrainfix";

/// The site's language for the app's `locale`: German or English.
fn site_language(locale: &str) -> &'static str {
    if locale == "de" { "de" } else { "en" }
}

/// The documentation site, in the app's language where the site has one
/// (English and German), English otherwise.
pub fn documentation_url(locale: &str) -> String {
    format!("{SITE}/{}/", site_language(locale))
}

/// The user guide's page on the desktop app, in the app's language where
/// the site has one, English otherwise: Help ▸ User guide (#145).
pub fn guide_url(locale: &str) -> String {
    format!("{SITE}/{}/guide/desktop-app.html", site_language(locale))
}

/// The privacy policy on the documentation site, in the app's language
/// where the site has one, English otherwise.
pub fn privacy_policy_url(locale: &str) -> String {
    format!(
        "{SITE}/{}/reference/privacy-policy.html",
        site_language(locale)
    )
}

/// The lists: what the phone ships too (the PP-OCRv6 models, whisper.cpp,
/// the Whisper model, Material Icons, Rust's standard library), what only
/// the desktop ships (SQLite, egui's fonts, KDE's protocols), and the
/// Rust crates.
const LISTS: [&str; 3] = [
    include_str!("../../../../licenses/common.txt"),
    include_str!("../../../../licenses/desktop.txt"),
    include_str!("../../../../licenses/desktop-crates.txt"),
];

/// One component, or several under the same text, and its licence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Licence {
    pub names: Vec<&'static str>,
    pub text: &'static str,
}

/// Every licence of the desktop app, in the order of the lists.
pub fn licences() -> &'static [Licence] {
    static ALL: LazyLock<Vec<Licence>> =
        LazyLock::new(|| LISTS.iter().flat_map(|list| parse(list)).collect());
    &ALL
}

/// The entries of a licence `list`: each is one or more names, one per
/// line, a blank line and the licence text, closed by a line of 80 `=`.
pub fn parse(list: &'static str) -> Vec<Licence> {
    let separator = format!("\n{}\n", "=".repeat(80));
    list.split(separator.as_str())
        .filter_map(|entry| {
            let (names, text) = entry.split_once("\n\n")?;
            Some(Licence {
                names: names.lines().collect(),
                text,
            })
        })
        .collect()
}

/// The About view; true when the licences are asked for.
pub fn about(ui: &mut Ui, t: &L10n, tones: Tones) -> bool {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut asked = false;
    ui.heading(RichText::new(t.about_title()).color(text));
    ui.label(RichText::new(t.about_version(env!("CARGO_PKG_VERSION"))).color(weak));
    ui.add_space(8.0);
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.about_tagline()).color(text));
        ui.add_space(8.0);
        ui.hyperlink_to(t.documentation(), documentation_url(t.locale()));
        ui.hyperlink_to(t.privacy_policy(), privacy_policy_url(t.locale()));
        ui.add_space(8.0);
        ui.label(RichText::new(t.source_code()).strong().color(text));
        ui.horizontal_wrapped(|ui| {
            ui.hyperlink(REPO);
            ui.label(RichText::new("Apache-2.0 / MIT").color(weak));
        });
        ui.add_space(8.0);
        if ui
            .button(RichText::new(t.open_source_licenses()).color(text))
            .clicked()
        {
            asked = true;
        }
    });
    asked
}

/// The licences page: one entry per component, its text opened on a
/// click.
pub fn licences_page(ui: &mut Ui, t: &L10n) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.heading(RichText::new(t.open_source_licenses()).color(text));
    ui.label(RichText::new(t.licenses_intro()).color(weak));
    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .id_salt("licences")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // A header lists up to a hundred crates: wrapped to the pane,
            // as a header on its own runs on in one line.
            let font = egui::TextStyle::Button.resolve(ui.style());
            let width = ui.available_width() - ui.spacing().indent - 8.0;
            for (i, licence) in licences().iter().enumerate() {
                let names =
                    ui.painter()
                        .layout(licence.names.join(", "), font.clone(), text, width);
                CollapsingHeader::new(names)
                    .id_salt(("licence", i))
                    .show(ui, |ui| {
                        ui.label(RichText::new(licence.text).monospace().small().color(text));
                    });
            }
        });
}

#[cfg(test)]
mod tests;
