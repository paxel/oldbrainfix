//! AXT files in the window (obf-1145, obf-1146, ADR 0056): "Export as
//! AXT file…" beside every "Export as Pack…" writes the Notes of a card,
//! the cards selected with it, the Note dialog's Note, or an Area's or a
//! Topic's Notes as one AXT file for another app, where the user says.
//! What goes into the file is the core's ([`Store::axt_write`]).
//!
//! An AXT file opened (File ▸ Open ▸ AXT file…, a drop, the command line
//! and "Open with") shows its records in a dialog, as an opened Pack
//! shows its tree: who sent it, each record it can take ticked, those it
//! cannot named and greyed, the place they go to, Unsorted first, and
//! "Take it" ([`Store::axt_take`]).

use std::collections::{BTreeSet, HashMap};

use egui::{RichText, Ui};
use oldbrainfix_core::axt::{AxtFile, AxtRecord, AxtRecordKind};
use oldbrainfix_core::{Area, Day, Note, Notes, Places, Result, Store, Topic};

use crate::design;
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::pack_view;
use crate::schedule_text::schedule_text;
use crate::theme;

/// What an AXT file ends in (AXT `spec/envelope.md`, section 9).
pub const EXTENSION: &str = "axt";

/// The longest stem of an AXT file's name, in characters.
const STEM_MAX: usize = 60;

/// The names Windows keeps for its devices, with any ending.
const RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

/// What "Export as AXT file…" writes: Notes picked, or the Notes of an
/// Area, its Topics' with them, or of a Topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxtOf {
    Notes(Vec<String>),
    Place(String),
}

/// The name an AXT file of `title` goes by, as the phone's `axtFileName`
/// names it: `Buy milk.axt`, each character a file name cannot hold on
/// any system made `_`, without blanks and full stops at either end, at
/// most 60 characters, `_` before a name Windows keeps for a device;
/// `oldbrainfix-notes-2026-11-25.axt` of `day` when nothing is left.
pub fn file_name(title: Option<&str>, day: Day) -> String {
    let first = title
        .unwrap_or_default()
        .split('\n')
        .next()
        .unwrap_or_default();
    let kept: String = first
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_ascii_control() {
                '_'
            } else {
                c
            }
        })
        .take(STEM_MAX)
        .collect();
    let mut stem = kept
        .trim_matches(|c: char| c.is_whitespace() || c == '.')
        .to_string();
    if stem.is_empty() {
        stem = format!("oldbrainfix-notes-{day}");
    }
    let base = stem.split('.').next().unwrap_or_default().to_uppercase();
    let numbered = |prefix: &str| {
        base.strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && n.chars().all(|c| ('1'..='9').contains(&c)))
    };
    if RESERVED.contains(&base.as_str()) || numbered("COM") || numbered("LPT") {
        stem.insert(0, '_');
    }
    format!("{stem}.{EXTENSION}")
}

/// The title the Note `note` goes by in an AXT file, its record's: the
/// first line of its text as it reads, trimmed.
pub fn note_title(note: &Note) -> String {
    let text = note.visible_text();
    text.trim()
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// The Notes of the Area or Topic `place`, as an export of it as a Pack
/// lists them: the place's own first, then each Topic's of an Area, in
/// the Topics' order; nothing in the trash or merged.
pub fn notes_of(store: &Store, place: &str) -> Result<Vec<String>> {
    let places = Places::new(store);
    let mut ids = vec![place.to_string()];
    if places.area_by_id(place)?.is_some() {
        ids.extend(places.topics(place)?.into_iter().map(|t| t.id));
    }
    let all = Notes::new(store).all()?;
    Ok(ids
        .iter()
        .flat_map(|id| {
            all.iter()
                .filter(move |n| n.place.as_deref() == Some(id.as_str()))
        })
        .filter(|n| !n.is_trashed() && !n.is_merged())
        .map(|n| n.id.clone())
        .collect())
}

/// Writes the AXT file of the Notes `ids` to `path`.
pub fn write_file(
    store: &Store,
    ids: &[String],
    path: &std::path::Path,
) -> std::result::Result<(), String> {
    let bytes = store.axt_write(ids).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// Avro's object container magic, which every AXT file starts with (AXT
/// `spec/envelope.md`): a reader knows the file by its content.
const MAGIC: &[u8; 4] = b"Obj\x01";

/// Whether the file at `path` is to be read as an AXT file: it starts as
/// one, or it is named as one, so that a broken one says it is no AXT
/// file rather than no bundle.
pub fn looks_like_axt(path: &std::path::Path) -> bool {
    use std::io::Read;
    let named = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION));
    let mut head = [0u8; 4];
    let starts = std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut head))
        .is_ok_and(|()| &head == MAGIC);
    named || starts
}

/// An AXT file opened, as its dialog shows it: the records ticked, by
/// id, and the place picked, 0 for Unsorted, else one of `places` after
/// it, each an Area or a Topic with its name; `names` names each Note a
/// record changes, or may not change (obf-1147), by its id.
#[derive(Debug, Clone, PartialEq)]
pub struct AxtOpened {
    pub file: AxtFile,
    pub ticks: BTreeSet<String>,
    pub place: usize,
    pub places: Vec<(String, String)>,
    pub names: HashMap<String, String>,
}

/// Whether `record` can be ticked: a new one, or an update or deletion
/// of a Note the user did not change since (obf-1147).
fn tickable(record: &AxtRecord) -> bool {
    record.taken() || (record.changes() && !record.kept)
}

impl AxtOpened {
    /// `file` with every record it can take ticked, its updates and
    /// deletions too but those the user's change keeps, into Unsorted.
    pub fn new(
        file: AxtFile,
        places: Vec<(String, String)>,
        names: HashMap<String, String>,
    ) -> AxtOpened {
        let ticks = file
            .records
            .iter()
            .filter(|r| tickable(r))
            .map(|r| r.id.clone())
            .collect();
        AxtOpened {
            file,
            ticks,
            place: 0,
            places,
            names,
        }
    }

    /// Whether the records ticked change a Note taken before (obf-1147).
    pub fn picks_changes(&self) -> bool {
        self.file
            .changes()
            .iter()
            .any(|r| !r.kept && self.ticks.contains(&r.id))
    }

    /// How many updates and deletions are ticked.
    pub fn changes_picked(&self) -> usize {
        self.file
            .changes()
            .iter()
            .filter(|r| !r.kept && self.ticks.contains(&r.id))
            .count()
    }

    /// The Area or Topic picked; `None` for Unsorted.
    pub fn place_id(&self) -> Option<&str> {
        let at = self.place.checked_sub(1)?;
        self.places.get(at).map(|(id, _)| id.as_str())
    }

    /// The ids of the records ticked that it can take, updates and
    /// deletions among them, in the file's order.
    pub fn picked(&self) -> Vec<String> {
        self.file
            .records
            .iter()
            .filter(|r| tickable(r) && self.ticks.contains(&r.id))
            .map(|r| r.id.clone())
            .collect()
    }
}

/// The places the records of `file` may go to besides Unsorted, in the
/// sidebar's order (`areas`): each Area, then its Topics, where this
/// Device may write, and, when a record brings a Reminder or Routine,
/// where it may make them too, as [`Store::axt_take`] asks.
pub fn places_for(
    t: &L10n,
    store: &Store,
    areas: &[(Area, Vec<Topic>)],
    file: &AxtFile,
) -> Result<Vec<(String, String)>> {
    let scheduled = file
        .taken()
        .iter()
        .any(|r| r.draft.as_ref().is_some_and(|d| d.schedule.is_some()));
    let mut places = Vec::new();
    for (area, topics) in areas {
        if scheduled && !store.may_manage(Some(&area.id))? {
            continue;
        }
        let name = area_name(t, area);
        if store.may_write_in(&area.id)? {
            places.push((area.id.clone(), name.clone()));
        }
        for topic in topics {
            if store.may_write_in(&topic.id)? {
                let label = format!("{name} / {}", topic_name(t, topic));
                places.push((topic.id.clone(), label));
            }
        }
    }
    Ok(places)
}

/// What a click in the dialog asks beyond its ticks and its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxtAsk {
    Take,
    Cancel,
}

/// The dialog of an AXT file opened, as the phone's page: who sent it,
/// "From your own Device" or the app by its name, the key's short
/// fingerprint under it to compare; "Tick all" and "Untick all" over the
/// records, each it can take a box with its first line, its type and its
/// Reminder or Routine under it, those it cannot greyed under "Not
/// taken", named by their type; where they go, Unsorted first; then Cancel and
/// "Take it", greyed while nothing is ticked.
pub fn opened_form(ui: &mut Ui, t: &L10n, opened: &mut AxtOpened, asked: &mut Option<AxtAsk>) {
    design::dialog_title(ui, t.axt_title());
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let file = &opened.file;
    let from = match &file.app_name {
        _ if file.from_own_device => t.axt_from_own_device().to_string(),
        Some(app) if !app.trim().is_empty() => t.axt_from_app(app),
        _ => t.axt_from_no_name().to_string(),
    };
    ui.label(RichText::new(from).strong());
    let key = t.axt_key(&file.short_fingerprint);
    ui.label(RichText::new(key).small().color(weak));
    ui.separator();
    let all: Vec<String> = file
        .records
        .iter()
        .filter(|r| tickable(r))
        .map(|r| r.id.clone())
        .collect();
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.pack_tick_all()).color(text))
            .clicked()
        {
            opened.ticks = all.iter().cloned().collect();
        }
        if ui
            .button(RichText::new(t.pack_untick_all()).color(text))
            .clicked()
        {
            opened.ticks.clear();
        }
    });
    let high = (ui.ctx().content_rect().height() - 420.0).max(160.0);
    let room = egui::vec2(ui.available_width(), high);
    ui.allocate_ui(room, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("axt records")
            .max_height(high)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                // Each record once, in the file's order, so none drops
                // out of the list.
                // The new ones and the changes to Notes taken before
                // (obf-1147), a change the user's own keeps greyed.
                let (taken, not_taken): (Vec<_>, Vec<_>) = opened
                    .file
                    .records
                    .iter()
                    .partition(|r| r.taken() || r.changes());
                if taken.is_empty() {
                    ui.label(RichText::new(t.axt_nothing_to_take()).color(text));
                }
                let names = &opened.names;
                for record in taken {
                    let may = tickable(record);
                    let mut on = may && opened.ticks.contains(&record.id);
                    let label = RichText::new(record_label(t, record, names)).color(text);
                    let tick = ui.add_enabled_ui(may, |ui| theme::checkbox(ui, &mut on, label));
                    if tick.inner.changed() {
                        if on {
                            opened.ticks.insert(record.id.clone());
                        } else {
                            opened.ticks.remove(&record.id);
                        }
                    }
                    // Its type and its Reminder or Routine, or the Note it
                    // changes, under its words, past the box.
                    under(ui, &record_detail(t, record, names), weak);
                }
                if !not_taken.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.axt_not_taken_heading()).strong());
                    for record in not_taken {
                        let mut off = false;
                        let label = RichText::new(record_label(t, record, names)).color(text);
                        ui.add_enabled_ui(false, |ui| theme::checkbox(ui, &mut off, label));
                        // Why a record of a Note taken before changes
                        // nothing (obf-1147).
                        if let Some(why) = not_taken_detail(t, record, names) {
                            under(ui, &why, weak);
                        }
                    }
                }
            });
    });
    ui.separator();
    ui.label(RichText::new(t.axt_into()).strong());
    let options = std::iter::once((0, t.unsorted().to_string())).chain(
        opened
            .places
            .iter()
            .enumerate()
            .map(|(i, (_, name))| (i + 1, name.clone())),
    );
    let high = 200.0;
    egui::ScrollArea::vertical()
        .id_salt("axt places")
        .max_height(high)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            crate::app::radio_column(ui, &mut opened.place, options);
        });
    let picked = !opened.picked().is_empty();
    ui.horizontal(|ui| {
        if design::row_button(ui, t.cancel()).clicked() {
            *asked = Some(AxtAsk::Cancel);
        }
        if theme::main_action(ui, picked, t.axt_take()) {
            *asked = Some(AxtAsk::Take);
        }
    });
    pack_view::nothing_ticked(ui, t, picked);
}

/// A small weak line under a record's words, past its box.
fn under(ui: &mut Ui, words: &str, weak: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_space(26.0);
        ui.label(RichText::new(words).small().color(weak));
    });
}

/// The name of the Note `record` changes or may not change, empty when it
/// names none.
fn note_name<'a>(record: &AxtRecord, names: &'a HashMap<String, String>) -> &'a str {
    record
        .note_id
        .as_ref()
        .and_then(|id| names.get(id))
        .map_or("", String::as_str)
}

/// What a record is called in the dialog: the first line of the Note it
/// makes, else, for a deletion, the Note it changes, else its type's
/// name, else that it has none.
fn record_label(t: &L10n, record: &AxtRecord, names: &HashMap<String, String>) -> String {
    let first = record
        .draft
        .as_ref()
        .map(|d| {
            d.text
                .trim()
                .lines()
                .next()
                .unwrap_or_default()
                .trim_start_matches('#')
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    if !first.is_empty() {
        return first;
    }
    let name = note_name(record, names);
    if !name.is_empty() {
        return name.to_string();
    }
    match &record.type_name {
        Some(name) => name.clone(),
        None => t.axt_no_type().to_string(),
    }
}

/// What stands under a record to take: its type, and the Reminder or
/// Routine it brings; for an update or a deletion the Note it changes,
/// and that the user's change was kept where it was (obf-1147).
fn record_detail(t: &L10n, record: &AxtRecord, names: &HashMap<String, String>) -> String {
    use oldbrainfix_core::axt::types;
    let name = note_name(record, names);
    let change = match record.kind {
        AxtRecordKind::Update => Some(t.axt_updates(name)),
        AxtRecordKind::Deletion => Some(t.axt_deletes(name)),
        _ => None,
    };
    if let Some(change) = change {
        return if record.kept {
            format!("{change} · {}", t.axt_kept_change())
        } else {
            change
        };
    }
    let kind = match record.type_name.as_deref() {
        Some(types::TASK) => t.axt_type_task(),
        Some(types::APPOINTMENT) => t.axt_type_appointment(),
        _ => t.axt_type_note(),
    };
    match record.draft.as_ref().and_then(|d| d.schedule.as_ref()) {
        Some(schedule) => format!("{kind} · {}", schedule_text(t, schedule)),
        None => kind.to_string(),
    }
}

/// Why a record under "Not taken" changes nothing of the Note taken
/// under its id (obf-1147): it may not, or that Note has it already.
fn not_taken_detail(
    t: &L10n,
    record: &AxtRecord,
    names: &HashMap<String, String>,
) -> Option<String> {
    let name = note_name(record, names);
    match record.kind {
        AxtRecordKind::Refused => Some(t.axt_refused_record(name)),
        AxtRecordKind::Stale => Some(t.axt_stale_record(name)),
        _ => None,
    }
}

/// The names of the Notes the records of `file` change or may not change
/// (obf-1147), by id, as `name` calls each.
pub fn note_names(
    store: &Store,
    file: &AxtFile,
    name: impl Fn(Option<&Note>) -> String,
) -> Result<HashMap<String, String>> {
    let notes = Notes::new(store);
    let mut names = HashMap::new();
    for id in file.records.iter().filter_map(|r| r.note_id.as_ref()) {
        if !names.contains_key(id) {
            names.insert(id.clone(), name(notes.by_id(id)?.as_ref()));
        }
    }
    Ok(names)
}
