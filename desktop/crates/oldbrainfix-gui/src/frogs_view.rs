//! Frogs on the desktop's screen (#101, ADR 0027): the Agenda's "Eat a
//! frog" section under today's Occurrences, and the Frog view its heading
//! opens, as the phone's `frogs_screen.dart` draws them. A Frog's row has
//! the box to check it off, its Note's text, struck through when Done,
//! and its Area's colour and icon; a click opens its Note, a right-click
//! offers Delete to who may manage it.

use egui::{Id, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::{Area, Day, Frog, Note};

use crate::app::{area_colour, check, embossed, row_title};
use crate::icons;
use crate::l10n::L10n;
use crate::schedule_text::day_text;
use crate::theme::Tones;

/// The frog that marks Frogs: there is none among the Material icons,
/// and egui's emoji font has it.
pub const FROG: &str = "\u{1F438}";

/// A Frog as it shows: with its Note, the Area that Note is filed in,
/// whether this Device may check it off and manage it (ADR 0013), and
/// the day it was Done on the wall clock.
#[derive(Debug, Clone)]
pub struct FrogShown {
    pub frog: Frog,
    pub note: Note,
    pub area: Option<Area>,
    pub may_check: bool,
    pub manages: bool,
    pub done_on: Option<Day>,
}

/// What the user asked of a Frog or the Frog view.
#[derive(Debug, Clone, PartialEq)]
pub enum FrogAction {
    /// Checks the Frog off, or unchecks it.
    Toggle(String),
    /// "Another one": swaps the Frog offered today for the next.
    Another(String),
    /// Opens the Note with this id.
    OpenNote(String),
    /// Asks whether the Frog goes to the trash.
    AskDelete(String),
    /// Opens the Frog view.
    OpenView,
    /// Keeps whether the Frog view's "Done" section is open.
    DoneOpen(bool),
}

/// The frog, `size` high, in `colour`.
pub fn frog_mark(ui: &mut Ui, size: f32, colour: egui::Color32) {
    ui.label(RichText::new(FROG).size(size).color(colour));
}

/// The heading of the "Eat a frog" section, with the frog; a click opens
/// the Frog view.
pub fn heading(ui: &mut Ui, t: &L10n, actions: &mut Vec<FrogAction>) {
    let text = ui.visuals().text_color();
    let label = t.eat_a_frog();
    let line = ui.horizontal(|ui| {
        ui.style_mut().interaction.selectable_labels = false;
        frog_mark(ui, 18.0, text);
        ui.label(RichText::new(label).strong().size(16.0).color(text));
        icons::show(ui, icons::NEXT, 20.0, text);
    });
    let open = ui
        .interact(line.response.rect, Id::new("frogs heading"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if open.clicked() {
        actions.push(FrogAction::OpenView);
    }
}

/// One Frog as an embossed row: the box to check it off, disabled when
/// this Device may not, its Note's text, struck through when Done, and
/// its Area's colour and icon. Under the text `subtitle` and, with
/// `another`, "Another one" (the control goes below its text). A click
/// on the text opens its Note; a right-click offers Delete to a Manager.
pub fn frog_row(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    shown: &FrogShown,
    (another, subtitle): (bool, Option<&str>),
    actions: &mut Vec<FrogAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let frog = &shown.frog;
    let done = frog.is_done();
    let colour = shown
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    let title = row_title(t, Some(&shown.note));
    embossed(ui, tones, false, colour, |ui| {
        ui.horizontal(|ui| {
            let label = format!("{}: {title}", t.mark_done());
            let toggled = ui
                .add_enabled_ui(shown.may_check, |ui| check(ui, tones, done, &label))
                .inner;
            if toggled.clicked() {
                actions.push(FrogAction::Toggle(frog.id.clone()));
            }
            ui.add_space(4.0);
            let width = (ui.available_width() - 32.0).max(40.0);
            let body = ui.vertical(|ui| {
                ui.set_max_width(width);
                let mut line = RichText::new(&title).size(15.0).color(text);
                if done {
                    line = line.strikethrough();
                }
                ui.add(egui::Label::new(line).truncate());
                if let Some(subtitle) = subtitle {
                    ui.label(RichText::new(subtitle).small().color(weak));
                }
            });
            let open = ui
                .interact(
                    body.response.rect,
                    Id::new(("frog", &frog.id)),
                    Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &title));
            if open.clicked() {
                actions.push(FrogAction::OpenNote(shown.note.id.clone()));
            }
            if shown.manages {
                open.context_menu(|ui| {
                    if ui.button(RichText::new(t.delete()).color(text)).clicked() {
                        actions.push(FrogAction::AskDelete(frog.id.clone()));
                    }
                });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(icon) = shown
                    .area
                    .as_ref()
                    .and_then(|a| icons::area_icon(a.icon.as_deref()))
                {
                    icons::show(ui, icon, 20.0, colour.unwrap_or(weak));
                }
            });
        });
        if another
            && ui
                .button(RichText::new(t.another_frog()).color(text))
                .clicked()
        {
            actions.push(FrogAction::Another(frog.id.clone()));
        }
    });
}

/// The Frog view: the open Frogs, oldest first, each with a box to check
/// it off; below, a folded "Done" section, closed until opened, with
/// those checked off, most recently first, each with the day it was
/// Done, whose box unchecks it. Whether it is open is a setting of this
/// Device.
pub fn frogs_page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (open, done, done_open): (&[FrogShown], &[FrogShown], bool),
    actions: &mut Vec<FrogAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.horizontal(|ui| {
        frog_mark(ui, 22.0, text);
        ui.heading(RichText::new(t.frogs_title()).color(text));
    });
    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .id_salt("frogs")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add_space(4.0);
            if open.is_empty() {
                ui.label(RichText::new(t.frogs_empty_desk()).color(weak));
            }
            for shown in open {
                frog_row(ui, t, tones, shown, (false, None), actions);
            }
            if done.is_empty() {
                return;
            }
            let fold = egui::CollapsingHeader::new(RichText::new(t.frogs_done()).color(text))
                .id_salt("frogs done")
                .open(Some(done_open))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    for shown in done {
                        let on = shown.done_on.map(|day| t.frog_done_on(&day_text(t, day)));
                        frog_row(ui, t, tones, shown, (false, on.as_deref()), actions);
                    }
                });
            if fold.header_response.clicked() {
                actions.push(FrogAction::DoneOpen(!done_open));
            }
        });
}
