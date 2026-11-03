//! Send feedback on the desktop (#214, Q536 (a)), as the phone's
//! `feedback_screen.dart` has it: Settings ▸ "Send feedback" opens a
//! page over the window with a text field, ticks for what goes along
//! (the version, the system and the install method ticked at first,
//! Notes as a Pack not), the preview of exactly what is sent, always
//! shown above the buttons, and Mail, a mail link to the developer, and
//! "Save as file…". A mail link cannot carry the Pack: it is saved with
//! "Save the Pack…" to be attached. The desktop keeps no crash report,
//! so it offers none.

use std::path::Path;

use egui::{RichText, TextEdit, Ui};
use oldbrainfix_core::{Pack, PackTicks};

use crate::app::{self, REPORT_MAIL};
use crate::l10n::L10n;
use crate::pack_view;
use crate::theme;

/// How much text a mail link carries, as on the phone (`mailBodyLimit`):
/// mail apps cap the link around 2000 characters.
pub const MAIL_BODY_LIMIT: usize = 1800;

/// The subject of every feedback mail, as on the phone.
pub const SUBJECT: &str = "Old Brain Fix feedback";

/// What the feedback can say of this build and this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackFacts {
    /// The app's version, "0.4.0".
    pub version: String,
    /// The system and the processor, "Fedora Linux 44 (KDE Plasma), x86_64".
    pub system: String,
    /// Where the app was installed from ([`install_method`]).
    pub install: String,
}

impl FeedbackFacts {
    /// The facts of the computer this runs on.
    pub fn of_this_computer() -> FeedbackFacts {
        let env = |name: &str| std::env::var(name).ok();
        let exe = std::env::current_exe().ok();
        let deb = Path::new("/var/lib/dpkg/info/oldbrainfix.list").exists();
        let release = ["/run/host/os-release", "/etc/os-release"]
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok());
        FeedbackFacts {
            version: env!("CARGO_PKG_VERSION").to_string(),
            system: system_line(
                release.as_deref(),
                std::env::consts::OS,
                std::env::consts::ARCH,
            ),
            install: install_method(&env, exe.as_deref(), deb, std::env::consts::OS),
        }
    }
}

/// Where the app came from, by its packaging: Flatpak by `FLATPAK_ID`,
/// an AppImage by `APPIMAGE`, Homebrew and Scoop by the path of the
/// executable `exe`, the .deb by `/usr/bin` and dpkg's list of it
/// (`deb_listed`), else a binary unpacked by hand. On macOS an app
/// outside Homebrew's paths may be the .dmg's or Homebrew's cask, so it
/// is "unknown", as is any executable whose path cannot be read.
pub fn install_method(
    env: &dyn Fn(&str) -> Option<String>,
    exe: Option<&Path>,
    deb_listed: bool,
    os: &str,
) -> String {
    if env("FLATPAK_ID").is_some_and(|id| !id.is_empty()) {
        return "Flatpak".into();
    }
    if env("APPIMAGE").is_some_and(|p| !p.is_empty()) {
        return "AppImage".into();
    }
    let Some(exe) = exe else {
        return "unknown".into();
    };
    let path = exe.to_string_lossy().replace('\\', "/").to_lowercase();
    if ["/cellar/", "/caskroom/", "/homebrew/", "/linuxbrew/"]
        .iter()
        .any(|p| path.contains(p))
    {
        return "Homebrew".into();
    }
    if path.contains("/scoop/apps/") {
        return "Scoop".into();
    }
    if os == "linux" && path == "/usr/bin/oldbrainfix" && deb_listed {
        return ".deb".into();
    }
    if os == "macos" {
        return "unknown".into();
    }
    "binary".into()
}

/// The system as `os-release` names it (`PRETTY_NAME`, else `NAME`),
/// else `os`, with the processor `arch`.
pub fn system_line(os_release: Option<&str>, os: &str, arch: &str) -> String {
    let field = |key: &str| {
        os_release?.lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix('=')?;
            let value = value.trim().trim_matches('"').trim();
            (!value.is_empty()).then(|| value.to_string())
        })
    };
    let name = field("PRETTY_NAME")
        .or_else(|| field("NAME"))
        .unwrap_or_else(|| os.to_string());
    format!("{name}, {arch}")
}

/// The text the feedback sends, exactly as the preview shows it, as the
/// phone writes it: the user's `text`, then each fact given.
pub fn feedback_body(
    text: &str,
    version: Option<&str>,
    system: Option<&str>,
    install: Option<&str>,
) -> String {
    let facts: Vec<String> = [
        version.map(|v| format!("Version: {v}")),
        system.map(|s| format!("System: {s}")),
        install.map(|i| format!("Installed from: {i}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut body = text.trim().to_string();
    if !facts.is_empty() {
        body.push_str("\n\n");
        body.push_str(&facts.join("\n"));
    }
    body
}

/// The `mailto:` link of the feedback mail carrying `body` as it is.
pub fn mail_url(body: &str) -> String {
    format!(
        "mailto:{REPORT_MAIL}?subject={}&body={}",
        app::percent_encoded(SUBJECT),
        app::percent_encoded(body),
    )
}

/// The name a saved feedback is offered under, on `day` (YYYY-MM-DD).
pub fn file_name(day: &str) -> String {
    format!("oldbrainfix-feedback-{day}.txt")
}

/// What a click on the page asks beyond its ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackAsk {
    /// Mail: the mail link with the preview's text.
    Mail,
    /// "Save as file…": the preview's text into a file.
    SaveFile,
    /// "Save the Pack…": the Notes ticked as a Pack.
    SavePack,
    /// "Notes as a Pack" ticked or not; the Pack is made at the first.
    Notes(bool),
}

/// The page's state while it shows, made anew each time it opens.
#[derive(Debug, Clone)]
pub struct FeedbackForm {
    pub text: String,
    pub version: bool,
    pub system: bool,
    pub install: bool,
    pub notes: bool,
    /// The Pack of everything the tree offers, nothing ticked at first.
    pub pack: Option<Pack>,
    pub ticks: PackTicks,
}

impl Default for FeedbackForm {
    fn default() -> FeedbackForm {
        FeedbackForm {
            text: String::new(),
            version: true,
            system: true,
            install: true,
            notes: false,
            pack: None,
            ticks: PackTicks::new(),
        }
    }
}

impl FeedbackForm {
    /// The text sent with `facts` as ticked.
    pub fn body(&self, facts: &FeedbackFacts) -> String {
        feedback_body(
            &self.text,
            self.version.then_some(facts.version.as_str()),
            self.system.then_some(facts.system.as_str()),
            self.install.then_some(facts.install.as_str()),
        )
    }

    /// The Pack going along: Notes ticked, at least one picked.
    pub fn picked(&self) -> Option<&Pack> {
        let pack = self.pack.as_ref().filter(|_| self.notes)?;
        (!pack.picked(&self.ticks).notes.is_empty()).then_some(pack)
    }
}

/// The page: the field, the ticks with their values beside them, the
/// Pack's tree under "Notes as a Pack" while it is ticked, the preview,
/// Mail and "Save as file…", greyed until there are words, Mail also
/// while the text is longer than a mail link carries, and "Save the
/// Pack…" while Notes go along.
pub fn form(
    ui: &mut Ui,
    t: &L10n,
    facts: &FeedbackFacts,
    form: &mut FeedbackForm,
) -> Option<FeedbackAsk> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut asked = None;
    egui::ScrollArea::vertical()
        .id_salt("feedback")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.label(
                RichText::new(t.feedback_title())
                    .size(18.0)
                    .strong()
                    .color(text),
            );
            let field = ui.add(
                TextEdit::multiline(&mut form.text)
                    .id(egui::Id::new("feedback-text"))
                    .hint_text(t.feedback_text_label())
                    .text_color(text)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            app::name_field(ui, &field, t.feedback_text_label());
            ui.label(RichText::new(t.feedback_goes_along()).strong().color(text));
            for (on, label, value) in [
                (&mut form.version, t.feedback_version(), &facts.version),
                (&mut form.system, t.feedback_system(), &facts.system),
                (&mut form.install, t.feedback_install(), &facts.install),
            ] {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    theme::checkbox(ui, on, RichText::new(label).color(text));
                    ui.label(RichText::new(value).small().color(weak));
                });
            }
            let mut notes = form.notes;
            if theme::checkbox(
                ui,
                &mut notes,
                RichText::new(t.feedback_notes()).color(text),
            )
            .changed()
            {
                asked = Some(FeedbackAsk::Notes(notes));
            }
            let picked = form.picked().map(|pack| {
                let size = pack_view::size_text(t, pack.size(&form.ticks));
                t.feedback_notes_picked(pack.picked(&form.ticks).notes.len() as i64, &size)
            });
            let hint = picked
                .clone()
                .unwrap_or_else(|| t.feedback_notes_hint().to_string());
            ui.label(RichText::new(hint).small().color(weak));
            if form.notes
                && let Some(pack) = &form.pack
            {
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 20,
                        ..Default::default()
                    })
                    .show(ui, |ui| {
                        let size = pack_view::size_text(t, pack.size(&form.ticks));
                        ui.label(RichText::new(t.pack_size(&size)).strong().color(text));
                        pack_view::tick_buttons(ui, t, pack, &mut form.ticks);
                        pack_view::tree(ui, t, pack, &mut form.ticks);
                    });
            }
            ui.separator();
            ui.label(RichText::new(t.feedback_preview()).strong().color(text));
            let body = form.body(facts);
            egui::Frame::NONE
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(8.0)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(
                        egui::Label::new(RichText::new(&body).monospace().color(text))
                            .selectable(true),
                    );
                });
            if let Some(notes) = &picked {
                ui.label(RichText::new(t.feedback_pack_along(notes)).color(text));
            }
            let written = !form.text.trim().is_empty();
            let fits = body.chars().count() <= MAIL_BODY_LIMIT;
            ui.horizontal(|ui| {
                if theme::primary(ui, written && fits, t.feedback_mail()).clicked() {
                    asked = Some(FeedbackAsk::Mail);
                }
                let save = RichText::new(t.feedback_save_file()).color(text);
                if ui.add_enabled(written, egui::Button::new(save)).clicked() {
                    asked = Some(FeedbackAsk::SaveFile);
                }
                if picked.is_some()
                    && ui
                        .button(RichText::new(t.feedback_save_pack()).color(text))
                        .clicked()
                {
                    asked = Some(FeedbackAsk::SavePack);
                }
            });
            if !written {
                ui.label(RichText::new(t.feedback_text_first()).color(weak));
            } else if !fits {
                ui.label(RichText::new(t.feedback_too_long_desktop()).color(weak));
            }
            if picked.is_some() {
                ui.label(RichText::new(t.feedback_pack_attach()).color(weak));
            }
        });
    asked
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| vars.get(name).cloned()
    }

    fn method(vars: &[(&str, &str)], exe: Option<&str>, deb: bool, os: &str) -> String {
        let exe = exe.map(PathBuf::from);
        install_method(&env(vars), exe.as_deref(), deb, os)
    }

    #[test]
    fn the_install_method_comes_from_the_packaging_and_says_unknown_when_unsure() {
        let id = [("FLATPAK_ID", "io.github.paxel.oldbrainfix")];
        assert_eq!(
            method(&id, Some("/app/bin/oldbrainfix"), false, "linux"),
            "Flatpak"
        );
        let image = [("APPIMAGE", "/home/ann/Old.AppImage")];
        assert_eq!(
            method(
                &image,
                Some("/tmp/.mount/usr/bin/oldbrainfix"),
                false,
                "linux"
            ),
            "AppImage"
        );
        for exe in [
            "/home/linuxbrew/.linuxbrew/Cellar/oldbrainfix/0.4.0/bin/oldbrainfix",
            "/opt/homebrew/Caskroom/oldbrainfix/0.4.0/Old Brain Fix.app/Contents/MacOS/oldbrainfix",
        ] {
            assert_eq!(method(&[], Some(exe), false, "linux"), "Homebrew", "{exe}");
        }
        let scoop = r"C:\Users\Ann\scoop\apps\oldbrainfix\current\oldbrainfix.exe";
        assert_eq!(method(&[], Some(scoop), false, "windows"), "Scoop");
        assert_eq!(
            method(&[], Some("/usr/bin/oldbrainfix"), true, "linux"),
            ".deb"
        );
        assert_eq!(
            method(&[], Some("/usr/bin/oldbrainfix"), false, "linux"),
            "binary"
        );
        assert_eq!(
            method(&[], Some("/home/ann/bin/oldbrainfix"), false, "linux"),
            "binary"
        );
        let app = "/Applications/Old Brain Fix.app/Contents/MacOS/oldbrainfix";
        assert_eq!(method(&[], Some(app), false, "macos"), "unknown");
        assert_eq!(method(&[], None, false, "linux"), "unknown");
        assert_eq!(
            method(&[("FLATPAK_ID", "")], None, false, "linux"),
            "unknown"
        );
    }

    #[test]
    fn the_system_line_names_the_distribution_and_the_processor() {
        let release = "NAME=\"Fedora Linux\"\nPRETTY_NAME=\"Fedora Linux 44 (KDE Plasma)\"\n";
        assert_eq!(
            system_line(Some(release), "linux", "x86_64"),
            "Fedora Linux 44 (KDE Plasma), x86_64"
        );
        assert_eq!(
            system_line(Some("NAME=Debian\n"), "linux", "aarch64"),
            "Debian, aarch64"
        );
        assert_eq!(system_line(None, "windows", "x86_64"), "windows, x86_64");
    }

    #[test]
    fn the_body_is_the_words_then_each_fact_ticked_as_the_phone_writes_it() {
        assert_eq!(
            feedback_body(
                " It counts wrong. ",
                Some("0.4.0"),
                Some("Debian, x86_64"),
                Some("Flatpak")
            ),
            "It counts wrong.\n\nVersion: 0.4.0\nSystem: Debian, x86_64\nInstalled from: Flatpak"
        );
        assert_eq!(
            feedback_body("Hi", None, Some("Debian, x86_64"), None),
            "Hi\n\nSystem: Debian, x86_64"
        );
        assert_eq!(feedback_body("Hi", None, None, None), "Hi");
    }

    #[test]
    fn the_mail_link_goes_to_the_developer_with_the_body_as_it_is() {
        assert_eq!(
            mail_url("A & B\n#1"),
            "mailto:taum@tuta.io?subject=Old%20Brain%20Fix%20feedback&body=A%20%26%20B%0A%231"
        );
    }
}
