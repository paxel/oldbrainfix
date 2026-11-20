//! App links on the desktop (#261, ADR 0046), as the phone's
//! `app_links.dart`: the "Link to…" picker of the app's pages, the Areas
//! and Topics and the Notes, and the files of the synced folder a file
//! link opens with the system's app after one question.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use egui::{Id, RichText, ScrollArea, TextEdit, Ui};
use oldbrainfix_core::app_link::AppLink;
use oldbrainfix_core::{Note, Notes, PersonSettings, Places, Result, Store};

use crate::icons;
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::text_marks_view;

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

/// One row of "Linked from" as drawn: where it lies, the Note it names
/// and its name.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkedRow {
    pub rect: egui::Rect,
    pub link: AppLink,
    pub name: String,
}

/// The Notes that link to the Note `of` (obf-1035, ADR 0046), at its end
/// after its Sections, as the phone's `LinkedFrom`: under "Linked from"
/// one row per Note, the link icon before its first line with words as
/// it reads, its capture time where it has none. Nothing while `linking`
/// is empty. A click on a row opens its Note as its app link does.
pub fn linked_from(ui: &mut Ui, t: &L10n, of: &str, linking: &[Note]) {
    let rows = linked_from_rows(ui, t, linking);
    // Where a tip points at it (obf-1043): the rows together.
    if let Some(all) = rows.iter().map(|row| row.rect).reduce(|a, b| a.union(b)) {
        crate::spotlight::anchor(ui, "note-linked-from", all);
    }
    for row in rows {
        linked_row_clicks(ui, of, &row);
    }
}

/// The rows of [`linked_from`] drawn under its heading, not yet taking
/// clicks: a card, which takes the clicks over it, has them take theirs
/// after its own with [`linked_row_clicks`].
pub fn linked_from_rows(ui: &mut Ui, t: &L10n, linking: &[Note]) -> Vec<LinkedRow> {
    if linking.is_empty() {
        return Vec::new();
    }
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.add_space(8.0);
    ui.label(RichText::new(t.linked_from()).strong().color(weak));
    let mut rows = Vec::new();
    for note in linking {
        let name = match name_of(note) {
            name if name.is_empty() => crate::app::when(t, note.captured),
            name => name,
        };
        let drawn = ui.horizontal(|ui| {
            icons::show(ui, icons::LINK, 16.0, weak);
            ui.label(RichText::new(&name).color(text));
        });
        rows.push(LinkedRow {
            rect: drawn.response.rect,
            link: AppLink::Note(note.id.clone()),
            name,
        });
    }
    rows
}

/// Takes the click on the row `row` of "Linked from" on the Note `of`,
/// registered over what lies there and named as a link by its Note: it
/// hands the link to its Note to the window, as a link in a text does.
/// Its id holds the layer it lies on, so the card behind a dialog and the
/// dialog showing the same Note never share one.
pub fn linked_row_clicks(ui: &mut Ui, of: &str, row: &LinkedRow) {
    let id = Id::new(("linked from", ui.layer_id(), of, row.link.url()));
    let click = ui.interact(row.rect, id, egui::Sense::click());
    click.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, &row.name));
    if click
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        text_marks_view::hand_app_link(ui.ctx(), row.link.clone());
    }
}

/// The first line with words of `note` as it reads, without the spaces
/// around it; empty when it has none.
pub fn name_of(note: &Note) -> String {
    note.visible_text()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
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
mod tests;
