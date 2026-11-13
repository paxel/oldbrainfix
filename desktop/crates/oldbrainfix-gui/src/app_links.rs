//! App links on the desktop (#261, ADR 0046), as the phone's
//! `app_links.dart`: the "Link to…" picker of the app's pages, the Areas
//! and Topics and the Notes, and the files of the synced folder a file
//! link opens with the system's app after one question.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use egui::{Id, RichText, ScrollArea, TextEdit, Ui};
use oldbrainfix_core::app_link::AppLink;
use oldbrainfix_core::{Notes, PersonSettings, Places, Result, Store};

use crate::l10n::L10n;
use crate::names::{area_name, topic_name};

/// What the picker groups its targets under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Pages,
    Places,
    Notes,
}

/// One target of the picker: what the link opens, the name it is put in
/// with, and its group.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub link: AppLink,
    pub label: String,
    pub group: Group,
}

/// The "Link to…" picker while it shows: its search and its targets.
#[derive(Debug, Clone, PartialEq)]
pub struct Picker {
    pub search: String,
    pub targets: Vec<Target>,
}

/// The Settings' sections the desktop has, by their link's name, with
/// their headings.
pub fn settings_sections(t: &L10n) -> [(&'static str, &'static str); 7] {
    [
        ("computer", t.settings_computer()),
        ("alarms", t.settings_alarms()),
        ("nudges", t.settings_nudges()),
        ("units", t.settings_units()),
        ("agenda", t.settings_agenda()),
        ("history", t.settings_history()),
        ("features", t.features()),
    ]
}

/// Every target of the picker: the app's pages, New Session only in
/// Advanced, then the Areas with their Topics, then the Notes by their
/// first line as it reads, those without words left out.
pub fn targets(store: &Store, t: &L10n, advanced: bool) -> Result<Vec<Target>> {
    let page = |link: AppLink, label: &str| Target {
        link,
        label: label.to_string(),
        group: Group::Pages,
    };
    let mut out = vec![
        page(AppLink::Agenda, t.agenda()),
        page(AppLink::Unsorted, t.unsorted()),
        page(AppLink::Routines, t.all_routines_title()),
        page(AppLink::Frogs, t.frogs_title()),
    ];
    if advanced {
        out.push(page(AppLink::NewSession, t.new_session()));
    }
    out.extend([
        page(AppLink::Profile, t.profile_title()),
        page(AppLink::Backups, t.backups_title()),
        page(AppLink::Sync, t.sync_title()),
        page(AppLink::Settings(None), t.settings_title()),
    ]);
    for (slug, heading) in settings_sections(t) {
        out.push(page(
            AppLink::Settings(Some(slug.to_string())),
            &format!("{} ▸ {heading}", t.settings_title()),
        ));
    }
    out.push(page(AppLink::Guide(None, None), t.link_to_guide()));
    let places = Places::new(store);
    // In the person's own order, as the sidebar has them (#280).
    for area in PersonSettings::new(store).areas_in_order_by(|a| area_name(t, a))? {
        let name = area_name(t, &area);
        let topics = places.topics(&area.id)?;
        out.push(Target {
            link: AppLink::Place(area.id),
            label: name.clone(),
            group: Group::Places,
        });
        for topic in topics {
            out.push(Target {
                label: format!("{name} ▸ {}", topic_name(t, &topic)),
                link: AppLink::Place(topic.id),
                group: Group::Places,
            });
        }
    }
    for note in Notes::new(store).all()? {
        let visible = note.visible_text();
        let name = visible.lines().next().unwrap_or_default().trim();
        if !name.is_empty() {
            out.push(Target {
                link: AppLink::Note(note.id.clone()),
                label: name.to_string(),
                group: Group::Notes,
            });
        }
    }
    Ok(out)
}

/// What the picker's user did this frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Picked {
    Target(Target),
    Cancel,
}

/// The picker over its search field, the targets the search finds under
/// their groups' headings; a click on one picks it.
pub fn picker(ui: &mut Ui, t: &L10n, picker: &mut Picker, focus: bool) -> Option<Picked> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.format_link_to()).heading().color(text));
    let field = ui.add(
        TextEdit::singleline(&mut picker.search)
            .id(Id::new("link to search"))
            .hint_text(t.link_to_search())
            .text_color(text)
            .desired_width(f32::INFINITY),
    );
    if focus {
        field.request_focus();
    }
    let query = picker.search.trim().to_lowercase();
    let found: Vec<&Target> = picker
        .targets
        .iter()
        .filter(|target| query.is_empty() || target.label.to_lowercase().contains(&query))
        .collect();
    let mut picked = None;
    ScrollArea::vertical()
        .id_salt("link to")
        .max_height(360.0)
        .show(ui, |ui| {
            if found.is_empty() {
                ui.label(RichText::new(t.link_to_nothing()).color(weak));
            }
            for (group, heading) in [
                (Group::Pages, t.link_to_pages()),
                (Group::Places, t.link_to_places()),
                (Group::Notes, t.link_to_notes()),
            ] {
                let mut shown = found.iter().filter(|target| target.group == group);
                let Some(first) = shown.next() else {
                    continue;
                };
                ui.label(RichText::new(heading).strong().color(text));
                for target in std::iter::once(first).chain(shown) {
                    let row = ui
                        .selectable_label(false, RichText::new(&target.label).color(text))
                        .on_hover_text(target.link.url());
                    if row.clicked() {
                        picked = Some(Picked::Target((*target).clone()));
                    }
                }
            }
        });
    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
        picked = Some(Picked::Cancel);
    }
    picked
}

/// The file at `path` inside the synced `folder`, its names separated by
/// `/`, when it is a file there: found below the folder after symbolic
/// links are followed, so a link never opens anything outside it. The
/// core already refuses `..`, `.`, empty names, `\` and `:`.
pub fn file_in(folder: &Path, path: &str) -> Option<PathBuf> {
    let root = folder.canonicalize().ok()?;
    let mut at = root.clone();
    for name in path.split('/') {
        if name.is_empty() || name == "." || name == ".." || name.contains(['\\', ':']) {
            return None;
        }
        at.push(name);
    }
    let found = at.canonicalize().ok()?;
    (found.starts_with(&root) && found.is_file()).then_some(found)
}

/// The name of the app the system opens `file` with, "" when it does not
/// tell: on Linux the desktop file `xdg-mime` names for the file's type,
/// read for its `Name`; elsewhere, and inside the Flatpak, none.
pub fn file_app(file: &Path) -> String {
    if !cfg!(target_os = "linux") {
        return String::new();
    }
    let query = |args: &[&str]| -> Option<String> {
        let out = Command::new("xdg-mime")
            .args(args)
            .stderr(Stdio::null())
            .output()
            .ok()?;
        let said = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (out.status.success() && !said.is_empty()).then_some(said)
    };
    let Some(file) = file.to_str() else {
        return String::new();
    };
    let Some(mime) = query(&["query", "filetype", file]) else {
        return String::new();
    };
    let Some(desktop) = query(&["query", "default", &mime]) else {
        return String::new();
    };
    desktop_name(&desktop).unwrap_or_default()
}

/// The `Name` of the desktop file `desktop`, such as `org.gnome.Evince.desktop`,
/// from the XDG data directories.
fn desktop_name(desktop: &str) -> Option<String> {
    let home = std::env::var("XDG_DATA_HOME").ok().or_else(|| {
        std::env::var("HOME")
            .ok()
            .map(|h| format!("{h}/.local/share"))
    });
    let dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in home.into_iter().chain(dirs.split(':').map(str::to_string)) {
        let Ok(text) = std::fs::read_to_string(Path::new(&dir).join("applications").join(desktop))
        else {
            continue;
        };
        if let Some(name) = desktop_entry_name(&text) {
            return Some(name);
        }
    }
    None
}

/// The plain `Name=` of a desktop file's `[Desktop Entry]` group.
pub fn desktop_entry_name(text: &str) -> Option<String> {
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        } else if in_entry && let Some(name) = line.strip_prefix("Name=") {
            return Some(name.trim().to_string());
        }
    }
    None
}

/// Opens `file` with the app the system has for its type, without a
/// shell ([`crate::system_open`]): `xdg-open` on Linux, `open` on macOS
/// and `ShellExecuteW` on Windows. Whether it could be started.
pub fn open_file(file: &Path) -> bool {
    let Some(mut command) = crate::system_open::command(file) else {
        return crate::system_open::shell_execute(file).is_ok();
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_link_finds_only_files_inside_the_folder() -> std::result::Result<(), String> {
        let base = std::env::temp_dir().join(format!("obf-links-{}", std::process::id()));
        let folder = base.join("Sync");
        std::fs::create_dir_all(folder.join("Scans")).map_err(|e| e.to_string())?;
        std::fs::write(folder.join("Scans/tax 2026.pdf"), b"x").map_err(|e| e.to_string())?;
        std::fs::write(base.join("secret.txt"), b"x").map_err(|e| e.to_string())?;
        let found = file_in(&folder, "Scans/tax 2026.pdf");
        let outside = file_in(&folder, "../secret.txt");
        let dir = file_in(&folder, "Scans");
        let missing = file_in(&folder, "nope.pdf");
        #[cfg(unix)]
        let linked = {
            std::os::unix::fs::symlink(base.join("secret.txt"), folder.join("ln.txt"))
                .map_err(|e| e.to_string())?;
            file_in(&folder, "ln.txt")
        };
        drop(std::fs::remove_dir_all(&base));
        assert!(found.is_some_and(|f| f.ends_with("Scans/tax 2026.pdf")));
        assert_eq!(outside, None);
        assert_eq!(dir, None);
        assert_eq!(missing, None);
        #[cfg(unix)]
        assert_eq!(linked, None, "a symbolic link out of the folder");
        Ok(())
    }

    #[test]
    fn a_desktop_file_names_its_app() {
        let text = "[Desktop Entry]\nType=Application\nName[de]=Dokumente\nName=Document Viewer\n[Desktop Action new]\nName=New";
        assert_eq!(desktop_entry_name(text).as_deref(), Some("Document Viewer"));
        assert_eq!(desktop_entry_name("[Other]\nName=x"), None);
    }
}
