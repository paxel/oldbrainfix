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

/// The lists: what the phone ships too (the ocrs models, whisper.cpp,
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
mod tests {
    use super::*;

    #[test]
    fn every_list_reads_into_named_entries() {
        let all = licences();
        assert!(all.len() > 100);
        assert!(all.iter().all(|l| !l.names.is_empty()
            && l.names.iter().all(|n| !n.trim().is_empty())
            && !l.text.trim().is_empty()));
        let of = |name: &str| {
            all.iter()
                .find(|l| l.names.contains(&name))
                .map(|l| l.text)
                .unwrap_or_default()
        };
        // A crate by its name, whatever its version.
        let of_crate = |name: &str| {
            all.iter()
                .find(|l| {
                    l.names.iter().any(|n| {
                        n.strip_prefix(name)
                            .and_then(|v| v.strip_prefix(' '))
                            .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
                    })
                })
                .map(|l| l.text)
                .unwrap_or_default()
        };
        assert!(of("ocrs models").contains("Robert Knight"));
        assert!(of("ocrs models").contains("bundled unmodified"));
        assert!(of("OpenAI Whisper base model").contains("Copyright (c) 2022 OpenAI"));
        assert!(of("whisper.cpp").contains("The ggml authors"));
        assert!(of("Material Icons").contains("Attribution 4.0 International"));
        assert!(of("SQLite").contains("blessing"));
        assert!(of("Ubuntu font").contains("UBUNTU FONT LICENCE"));
        assert!(of_crate("ocrs").contains("Ocrs project contributors"));
        assert!(of_crate("egui").contains("Emil Ernerfeldt"));
    }

    #[test]
    fn an_entry_takes_its_names_and_text() {
        let list = "a 1.0\nb 2.0\n\nThe text.\n\nMore.\n================================================================================\n";
        assert_eq!(
            parse(list),
            vec![Licence {
                names: vec!["a 1.0", "b 2.0"],
                text: "The text.\n\nMore.",
            }]
        );
    }
}
