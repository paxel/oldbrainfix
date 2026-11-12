//! Packs on the desktop (#165, ADR 0043), as the phone's
//! `pack_screen.dart` shows them, in dialogs: "Export as Pack…" opens the
//! tree of what the Pack may hold, everything ticked but Reminders and
//! files, with its size as items are ticked, "Add my Profile" off and
//! Save…; a Pack opened shows the same tree, all ticked, under who it is
//! from with "Add … to People", "Claim as mine" ticked when it says who
//! it is from, and Add. Each box stands beside its words (UI laws,
//! Desktop, "Dialogs"); an item under a Note not ticked cannot be ticked.

use std::cell::Cell;
use std::collections::BTreeSet;

use egui::{RichText, Ui};
use oldbrainfix_core::{
    OpenedPack, Pack, PackItem, PackItemKind, PackNote, PackTicks, text_format,
};

use crate::icons;
use crate::l10n::L10n;
use crate::profile_view::{self, Pictures};
use crate::schedule_text::schedule_text;
use crate::theme;

/// `inventory_2_outlined`, as the phone marks a Pack.
pub const PACK_ICON: char = '\u{f134}';

/// `photo_outlined`, `notes`, `mic_none` and `notifications_outlined`, as
/// the phone's tree marks a photo, a text, a recording and a Reminder.
const PHOTO_ICON: char = '\u{f29e}';
const TEXT_ICON: char = '\u{e44c}';
const RECORDING_ICON: char = '\u{e3e4}';
const REMINDER_ICON: char = '\u{f237}';

/// What a click in a Pack's dialog asks beyond its ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackAsk {
    /// Save… on the export.
    Save,
    /// Add on a Pack opened.
    Add,
    /// "Add … to People" on a Pack opened.
    AddPerson,
    Cancel,
}

/// `bytes` in words, as the phone's `packSizeText`: "800 B", "12 KB",
/// "1.2 MB", the decimal point as the language writes it.
pub fn size_text(t: &L10n, bytes: usize) -> String {
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    if bytes < 1000 * 1000 {
        return format!("{} KB", (bytes as f64 / 1000.0).round());
    }
    let mb = format!("{:.1}", bytes as f64 / (1000.0 * 1000.0));
    let mb = if t.locale() == "de" {
        mb.replace('.', ",")
    } else {
        mb
    };
    format!("{mb} MB")
}

thread_local! {
    /// How many times a Pack's ticks were worked out on this thread.
    static WORKED_OUT: Cell<usize> = const { Cell::new(0) };
}

/// How many times [`Worked::of`] worked out a Pack's ticks on this
/// thread, for tests: once for each change of the ticks.
pub fn worked_out() -> usize {
    WORKED_OUT.with(Cell::get)
}

/// How many Notes a Pack's ticks pick and the Pack's size, worked out
/// once for each change of the ticks, not on each frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Worked {
    ticks: Option<PackTicks>,
    picked: usize,
    size: usize,
}

impl Worked {
    /// How many Notes `ticks` pick from `pack`, and its size then.
    pub fn of(&mut self, pack: &Pack, ticks: &PackTicks) -> (usize, usize) {
        if self.ticks.as_ref() != Some(ticks) {
            WORKED_OUT.with(|n| n.set(n.get() + 1));
            self.picked = pack.picked(ticks).notes.len();
            self.size = pack.size(ticks);
            self.ticks = Some(ticks.clone());
        }
        (self.picked, self.size)
    }
}

/// The export (Q467–Q470): the size and "Tick all" and "Untick all" over
/// the tree, "Add my Profile" under it, then Cancel and Save…, greyed
/// while no Note is ticked or the Pack is being saved (`busy`).
pub fn export_form(
    ui: &mut Ui,
    t: &L10n,
    pack: &Pack,
    (ticks, profile, worked): (&mut PackTicks, &mut bool, &mut Worked),
    busy: bool,
    asked: &mut Option<PackAsk>,
) {
    heading(ui, t.pack_export_title());
    let text = ui.visuals().text_color();
    let (_, size) = worked.of(pack, ticks);
    ui.label(
        RichText::new(t.pack_size(&size_text(t, size)))
            .strong()
            .color(text),
    );
    tick_buttons(ui, t, pack, ticks);
    tree(ui, t, pack, ticks, &BTreeSet::new());
    ui.separator();
    tick_with_hint(ui, profile, t.pack_add_profile(), t.pack_add_profile_hint());
    let picked = worked.of(pack, ticks).0 > 0;
    ui.horizontal(|ui| {
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            *asked = Some(PackAsk::Cancel);
        }
        if theme::main_action(ui, picked && !busy, t.pack_save()) {
            *asked = Some(PackAsk::Save);
        }
        if busy {
            ui.spinner();
        }
    });
    nothing_ticked(ui, t, picked);
}

/// A Pack opened (Q459, Q462, Q463): who it is from, with its picture,
/// and "Add … to People" while they are not kept here; the tree with
/// "Tick all" and "Untick all"; "Claim as mine" when it says who it is
/// from; then Cancel and Add, greyed while no Note is ticked.
pub fn import_form(
    ui: &mut Ui,
    t: &L10n,
    (opened, pictures): (&OpenedPack, &mut Pictures),
    (ticks, claim, worked): (&mut PackTicks, &mut bool, &mut Worked),
    asked: &mut Option<PackAsk>,
) {
    heading(ui, t.pack_title());
    let text = ui.visuals().text_color();
    if let Some(from) = &opened.from {
        let name = profile_view::display_name(t, &from.name);
        let said = t.pack_from(name);
        ui.horizontal(|ui| {
            profile_view::person_picture(ui, pictures, from, 40.0, &t.author_picture(name));
            ui.label(RichText::new(&said).strong().color(text));
        });
        if !opened.from_kept
            && ui
                .button(RichText::new(t.pack_add_person(name)).color(text))
                .clicked()
        {
            *asked = Some(PackAsk::AddPerson);
        }
        ui.separator();
    }
    let pack = &opened.pack;
    tick_buttons(ui, t, pack, ticks);
    tree(ui, t, pack, ticks, &BTreeSet::new());
    ui.separator();
    if opened.from.is_some() {
        tick_with_hint(ui, claim, t.pack_claim(), t.pack_claim_hint());
    }
    let picked = worked.of(pack, ticks).0 > 0;
    ui.horizontal(|ui| {
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            *asked = Some(PackAsk::Cancel);
        }
        if theme::main_action(ui, picked, t.pack_add()) {
            *asked = Some(PackAsk::Add);
        }
    });
    nothing_ticked(ui, t, picked);
}

fn heading(ui: &mut Ui, title: &str) {
    let text = ui.visuals().text_color();
    ui.label(RichText::new(title).size(18.0).strong().color(text));
}

/// "Tick all" and "Untick all", in place of the phone's bar.
pub(crate) fn tick_buttons(ui: &mut Ui, t: &L10n, pack: &Pack, ticks: &mut PackTicks) {
    let text = ui.visuals().text_color();
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.pack_tick_all()).color(text))
            .clicked()
        {
            *ticks = pack.all_keys();
        }
        if ui
            .button(RichText::new(t.pack_untick_all()).color(text))
            .clicked()
        {
            ticks.clear();
        }
    });
}

/// A box with its words beside it and its detail in a small weak line
/// under them.
fn tick_with_hint(ui: &mut Ui, on: &mut bool, label: &str, hint: &str) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        theme::checkbox(ui, on, RichText::new(label).color(text));
        ui.label(RichText::new(hint).small().color(weak));
    });
}

pub(crate) fn nothing_ticked(ui: &mut Ui, t: &L10n, picked: bool) {
    if !picked {
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.pack_nothing_ticked()).color(weak));
    }
}

/// The tree of `pack`, as high as the window allows: an Area and a Topic
/// as headings, each other item a box with its words and its icon at the
/// end, as the phone's; a box changes the ticks by the Session rule
/// (Q458). A Note whose index is in `trashed`, from a Backup's Trash
/// (#285), says "In the Trash" under its words, not by colour alone.
pub(crate) fn tree(
    ui: &mut Ui,
    t: &L10n,
    pack: &Pack,
    ticks: &mut PackTicks,
    trashed: &BTreeSet<usize>,
) {
    let high = (ui.ctx().content_rect().height() - 360.0).max(200.0);
    // Up to `high`, not held to the room left under it: the dialog finds
    // its place over the window from its size, and a tree held to that
    // room would grow on every frame.
    let room = egui::vec2(ui.available_width(), high);
    let mut changed = None;
    ui.allocate_ui(room, |ui| {
        egui::ScrollArea::vertical()
            .max_height(high)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for item in pack.tree() {
                    let shown = (pack, &*ticks, trashed);
                    row(ui, t, shown, (&item, 0, true), &mut changed);
                }
            });
    });
    if let Some((key, on)) = changed {
        *ticks = pack.tick(ticks, &key, on);
    }
}

/// One item of the tree at `depth` and the items under it; its box is
/// greyed unless `enabled`.
fn row(
    ui: &mut Ui,
    t: &L10n,
    (pack, ticks, trashed): (&Pack, &PackTicks, &BTreeSet<usize>),
    (item, depth, enabled): (&PackItem, usize, bool),
    changed: &mut Option<(String, bool)>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let indent = 20.0 * depth as f32;
    match item.kind {
        PackItemKind::Area | PackItemKind::Topic => {
            let (a, topic) = place_of(&item.key);
            let name = pack
                .areas
                .get(a)
                .map(|area| match topic {
                    Some(j) => area.topics.get(j).cloned().unwrap_or_default(),
                    None => area.name.clone(),
                })
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let size = if item.kind == PackItemKind::Area {
                    16.0
                } else {
                    14.0
                };
                ui.label(RichText::new(name).size(size).strong().color(text));
            });
            for child in &item.children {
                row(
                    ui,
                    t,
                    (pack, ticks, trashed),
                    (child, depth + 1, true),
                    changed,
                );
            }
        }
        _ => {
            let ticked = ticks.contains(&item.key);
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let mut on = ticked;
                let label = RichText::new(label(t, pack, item)).color(text);
                // Its words cut short before the icon, as the dialog is
                // only so wide.
                let words = egui::vec2(
                    (ui.available_width() - 28.0).max(40.0),
                    ui.spacing().interact_size.y,
                );
                let tick = ui.allocate_ui(words, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    ui.add_enabled_ui(enabled, |ui| theme::checkbox(ui, &mut on, label))
                        .inner
                });
                if tick.inner.changed() {
                    *changed = Some((item.key.clone(), on));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    icons::show(ui, icon(pack, item), 18.0, weak);
                });
            });
            let in_trash =
                item.kind == PackItemKind::Note && item.note.is_some_and(|i| trashed.contains(&i));
            if in_trash {
                ui.horizontal(|ui| {
                    // Under the words, past the box.
                    ui.add_space(indent + 18.0);
                    ui.label(RichText::new(t.backup_note_trashed()).small().color(weak));
                });
            }
            let under = if item.kind == PackItemKind::Note {
                enabled && ticked
            } else {
                enabled
            };
            for child in &item.children {
                row(
                    ui,
                    t,
                    (pack, ticks, trashed),
                    (child, depth + 1, under),
                    changed,
                );
            }
        }
    }
}

/// The Area and Topic `a<i>` and `a<i>.t<j>` name.
fn place_of(key: &str) -> (usize, Option<usize>) {
    let mut parts = key.split('.');
    let index = |part: Option<&str>| part.and_then(|p| p.get(1..)).and_then(|n| n.parse().ok());
    (index(parts.next()).unwrap_or(0), index(parts.next()))
}

fn icon(pack: &Pack, item: &PackItem) -> char {
    match item.kind {
        PackItemKind::Note => {
            let kind = item
                .note
                .and_then(|i| pack.notes.get(i))
                .map(PackNote::kind);
            match kind {
                Some(oldbrainfix_core::note_kinds::NoteKind::Checklist) => icons::CHECKLIST,
                Some(oldbrainfix_core::note_kinds::NoteKind::ShoppingList) => icons::SHOPPING_CART,
                Some(oldbrainfix_core::note_kinds::NoteKind::Tally) => icons::TALLY,
                Some(oldbrainfix_core::note_kinds::NoteKind::Session) => icons::SESSION,
                _ => icons::ALL_NOTES,
            }
        }
        PackItemKind::Photo => PHOTO_ICON,
        PackItemKind::PhotoText | PackItemKind::Transcript => TEXT_ICON,
        PackItemKind::Recording => RECORDING_ICON,
        PackItemKind::Routine => icons::REPEAT,
        PackItemKind::Reminder => REMINDER_ICON,
        PackItemKind::Frog => icons::FROG,
        PackItemKind::Area | PackItemKind::Topic => icons::NO_AREA_ICON,
    }
}

/// The words of an item that is ticked: a Note's first line, a file with
/// its size, a text's start, a Routine's or Reminder's schedule.
fn label(t: &L10n, pack: &Pack, item: &PackItem) -> String {
    let Some(note) = item.note.and_then(|i| pack.notes.get(i)) else {
        return String::new();
    };
    let start = |text: Option<&String>| {
        text.map(|s| s.split('\n').next().unwrap_or_default().trim().to_string())
            .unwrap_or_default()
    };
    let at = item.index.unwrap_or(0);
    match item.kind {
        PackItemKind::Note => first_line(note),
        PackItemKind::Photo => {
            t.pack_photo(&size_text(t, note.photos.get(at).map_or(0, |p| p.size)))
        }
        PackItemKind::PhotoText => {
            t.pack_photo_text(&start(note.photos.get(at).and_then(|p| p.text.as_ref())))
        }
        PackItemKind::Recording => {
            t.pack_recording(&size_text(t, note.recordings.get(at).map_or(0, |a| a.size)))
        }
        PackItemKind::Transcript => t.pack_transcript(&start(
            note.recordings.get(at).and_then(|a| a.text.as_ref()),
        )),
        PackItemKind::Routine => t.pack_routine(
            &note
                .routines
                .get(at)
                .map(|r| schedule_text(t, r))
                .unwrap_or_default(),
        ),
        PackItemKind::Reminder => t.pack_reminder(
            &note
                .routines
                .get(at)
                .map(|r| schedule_text(t, r))
                .unwrap_or_default(),
        ),
        PackItemKind::Frog => t.pack_frog().to_string(),
        PackItemKind::Area | PackItemKind::Topic => String::new(),
    }
}

/// The first line with words of `note`'s text as it reads.
fn first_line(note: &PackNote) -> String {
    let shown = text_format::visible(&note.format, &note.text);
    shown
        .split('\n')
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(shown.trim())
        .to_string()
}
