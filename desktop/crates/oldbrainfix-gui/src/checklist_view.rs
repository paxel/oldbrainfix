//! Checklists on the desktop's screen (#120, ADR 0030), as the phone's
//! `checklist_view.dart` draws them: the Lines of a Checklist, each with
//! its checkbox and its text, struck through in place once checked, the
//! group of a Line drawn as one frame under it; on a full card checkable,
//! on a compact one "x of y checked"; and the Checklist part of a Note's
//! editor.
//!
//! The editor's part lists the Lines to check; for who may change a
//! Line, ⋮ and a right-click with Rename, Move up, Move down, putting it
//! under the Line above or back to the top level, and Delete; a field and
//! "Add Line" for anyone with a role in its Area, every line typed one
//! Line, one starting with a space or a tab under the Line above;
//! "Uncheck all" while one is checked; and "Back to a plain Note" for who
//! may change the Note.
//!
//! A Checklist with a Routine or Reminder (#121) shows on its card and in
//! its editor as checked for the Occurrence it is on now, and on the
//! Agenda each Occurrence's row shows its Lines as checked for that
//! Occurrence; a check made there counts for that Occurrence only.
//!
//! A Shopping List's Lines (#122) are drawn by the same rows, with the
//! day a struck one comes back under it and a menu of its own, and the
//! card's menu offers making a Note one (`shopping_view`).

use std::collections::HashSet;

use egui::{Frame, Id, Key, Modifiers, RichText, Sense, TextEdit, Ui, WidgetInfo};
use oldbrainfix_core::checklists::split_lines;
use oldbrainfix_core::sent::SentLine;
use oldbrainfix_core::{
    Checklist, Checklists, Line, Note, RoutineOccurrence, ShoppingLists, Store, WallClock,
};

use crate::app::{check, embossed, icon_button};
use crate::icons;
use crate::l10n::L10n;
use crate::shopping_view;
use crate::theme::Tones;

/// A Line as it is drawn: its id, its text, whether it is in the group
/// of the Line above it, whether it shows checked and what shows dimmed
/// under it, as the day a Shopping List's struck Line comes back (#122).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownLine {
    pub id: String,
    pub text: String,
    pub in_group: bool,
    pub checked: bool,
    pub below: Option<String>,
}

impl ShownLine {
    fn of_line(line: &Line) -> ShownLine {
        ShownLine {
            id: line.id.clone(),
            text: line.text.clone(),
            in_group: line.in_group(),
            checked: line.checked,
            below: None,
        }
    }

    /// A Line of a Sent Item, a Shopping List's with its reappear time
    /// under it, as the phone's preview shows it.
    fn of_sent(t: &L10n, line: &SentLine) -> ShownLine {
        ShownLine {
            id: line.id.clone(),
            text: line.text.clone(),
            in_group: line.indent,
            checked: line.checked,
            below: line
                .reappear
                .map(|r| t.sent_item_line_reappear(&shopping_view::reappear_label(t, r))),
        }
    }
}

/// The Lines of `list` as they are drawn.
pub fn shown_lines(list: &Checklist) -> Vec<ShownLine> {
    list.lines.iter().map(ShownLine::of_line).collect()
}

/// The Lines of a Sent Item as they are drawn.
pub fn sent_lines(t: &L10n, lines: &[SentLine]) -> Vec<ShownLine> {
    lines
        .iter()
        .map(|line| ShownLine::of_sent(t, line))
        .collect()
}

/// What the menu of a Line does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStep {
    Rename,
    Up,
    Down,
    Indent,
    Outdent,
    Delete,
}

/// Something asked of a Checklist, on a card or in the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChecklistAction {
    /// Makes the Note with this id a Checklist.
    Make(String),
    /// Turns the Checklist of the Note back into a plain Note.
    ToPlain(String),
    /// Unchecks every Line of the Note's Checklist as it shows now.
    UncheckAll(String),
    /// Checks the Line with this id, or unchecks it: for the Occurrence
    /// given, or by its own checks without one.
    Check(String, bool, Option<RoutineOccurrence>),
    /// Adds every line typed into the add field to the Note's Checklist.
    Add(String),
    /// A step of the menu of the Line with this id.
    Step(String, LineStep),
    /// Reads what the menu of the card of the Note with this id offers,
    /// as it opens.
    Menu(String),
}

/// The Checklist of the selected Note as its editor shows it.
#[derive(Debug, Clone)]
pub struct ChecklistShown {
    pub list: Checklist,
    /// Whether this Device may check its Lines as they show.
    pub checks: bool,
    /// Whether it may add Lines.
    pub adds: bool,
    /// The Lines it may rename, delete, move and indent, by id.
    pub editable: HashSet<String>,
    /// Whether it may turn it back into a plain Note.
    pub plain: bool,
}

/// Whether this Device may check the Lines of `list` as it shows: for
/// its Occurrence who may check that off, else who may check the Note's
/// Lines.
pub fn may_check_shown(store: &Store, list: &Checklist) -> oldbrainfix_core::Result<bool> {
    match &list.on {
        Some(on) => store.may_check_off(&on.routine_id),
        None => Checklists::new(store).may_check(&list.note_id),
    }
}

/// Reads the Checklist of the Note `note` as it shows now, days as
/// `wall` shows them, or `None` while it is none; `changes` says whether
/// this Device may change the Note.
pub fn read(
    store: &Store,
    note: &Note,
    changes: bool,
    wall: &WallClock,
) -> oldbrainfix_core::Result<Option<ChecklistShown>> {
    let lists = Checklists::new(store);
    let Some(list) = lists.shown_now(&note.id, wall)? else {
        return Ok(None);
    };
    let mut editable = HashSet::new();
    for line in &list.lines {
        if lists.may_edit(&line.id)? {
            editable.insert(line.id.clone());
        }
    }
    Ok(Some(ChecklistShown {
        checks: may_check_shown(store, &list)?,
        adds: lists.may_check(&note.id)?,
        editable,
        plain: changes && note.text_editable,
        list,
    }))
}

/// What the menu of a Note's card offers of a Checklist (#120) and a
/// Shopping List (#122), read as it opens: making it one, unchecking all
/// of a Checklist's Lines and turning either back into a plain Note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardMenu {
    pub note: String,
    pub make: bool,
    pub make_shopping: bool,
    pub uncheck_all: bool,
    pub to_plain: bool,
}

/// Reads what the card menu of `note` offers, its Checklist as it shows
/// now with days as `wall` shows them.
pub fn card_menu(
    store: &Store,
    note: &Note,
    wall: &WallClock,
) -> oldbrainfix_core::Result<CardMenu> {
    let lists = Checklists::new(store);
    let list = lists.shown_now(&note.id, wall)?;
    let uncheck_all = match &list {
        Some(list) => list.checked_count() > 0 && may_check_shown(store, list)?,
        None => false,
    };
    let shopping = ShoppingLists::new(store);
    let is_list = list.is_some() || shopping.is_shopping_list(&note.id)?;
    let to_plain = is_list && note.text_editable && store.may_change(&note.id)?;
    Ok(CardMenu {
        note: note.id.clone(),
        make: list.is_none() && lists.may_make(&note.id)?,
        make_shopping: shopping.may_make(&note.id)?,
        uncheck_all,
        to_plain,
    })
}

/// The Lines on the level of `line`: its group, or the top.
fn level<'a>(list: &'a Checklist, line: &Line) -> Vec<&'a Line> {
    match &line.parent {
        Some(parent) => list.group_of(parent),
        None => list.lines.iter().filter(|l| !l.in_group()).collect(),
    }
}

/// The Line `line` goes before when it moves up, or `None` at the top of
/// its level; and whether it can move down: Lines of a group move within
/// it, Lines at the top past the next one with its group.
fn neighbours(list: &Checklist, line: &Line) -> (Option<String>, bool) {
    let level = level(list, line);
    let Some(at) = level.iter().position(|l| l.id == line.id) else {
        return (None, false);
    };
    let before = at.checked_sub(1).map(|i| level[i].id.clone());
    (before, at + 1 < level.len())
}

/// The steps the menu of `line` offers, as `list` stands.
pub fn steps(list: &Checklist, line: &Line) -> Vec<LineStep> {
    let at = list.lines.iter().position(|l| l.id == line.id);
    let (before, after) = neighbours(list, line);
    let mut steps = vec![LineStep::Rename];
    if before.is_some() {
        steps.push(LineStep::Up);
    }
    if after {
        steps.push(LineStep::Down);
    }
    if !line.in_group() && at.is_some_and(|at| at > 0) {
        steps.push(LineStep::Indent);
    }
    if line.in_group() {
        steps.push(LineStep::Outdent);
    }
    steps.push(LineStep::Delete);
    steps
}

/// Moves `line` one place up on its level.
pub fn move_up(store: &Store, list: &Checklist, line: &Line) -> oldbrainfix_core::Result<()> {
    match neighbours(list, line).0 {
        Some(before) => Checklists::new(store).move_line(&line.id, Some(&before)),
        None => Ok(()),
    }
}

/// Moves `line` one place down on its level: past the next Line of its
/// group, or past the next Line at the top with that one's group.
pub fn move_down(store: &Store, list: &Checklist, line: &Line) -> oldbrainfix_core::Result<()> {
    let lists = Checklists::new(store);
    let level = level(list, line);
    let Some(at) = level.iter().position(|l| l.id == line.id) else {
        return Ok(());
    };
    if at + 2 < level.len() {
        return lists.move_line(&line.id, Some(&level[at + 2].id));
    }
    if !line.in_group() {
        return lists.move_line(&line.id, None);
    }
    // Last but one of its group: after the group's last Line, under it
    // again.
    let Some(last) = level
        .last()
        .and_then(|last| list.lines.iter().position(|l| l.id == last.id))
    else {
        return Ok(());
    };
    let before = list.lines.get(last + 1).map(|l| l.id.clone());
    lists.move_line(&line.id, before.as_deref())?;
    lists.indent(&line.id, true)
}

/// Adds a Line to the Checklist of `note_id` for every line of `typed`
/// that is not blank, one that starts with a space or a tab under the
/// Line above; whether any was added.
pub fn add_typed(store: &Store, note_id: &str, typed: &str) -> oldbrainfix_core::Result<bool> {
    let lists = Checklists::new(store);
    let rows: Vec<String> = split_lines(typed)
        .into_iter()
        .filter(|row| !row.trim().is_empty())
        .collect();
    for row in &rows {
        let indent = row.starts_with(' ') || row.starts_with('\t');
        lists.add(note_id, row, None, indent)?;
    }
    Ok(!rows.is_empty())
}

/// A Line's new text as the rename dialog took it: on one line, trimmed;
/// `None` when nothing is left.
pub fn renamed(text: &str) -> Option<String> {
    let named = text
        .split(['\r', '\n'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let named = named.trim();
    (!named.is_empty()).then(|| named.to_string())
}

/// The label of a Line's ⋮.
pub fn menu_label(t: &L10n, line: &str) -> String {
    format!("{}: {line}", t.more())
}

/// The words of a step of a Line's menu.
fn step_label(t: &L10n, step: LineStep) -> &'static str {
    match step {
        LineStep::Rename => t.rename(),
        LineStep::Up => t.checklist_move_up(),
        LineStep::Down => t.checklist_move_down(),
        LineStep::Indent => t.checklist_indent(),
        LineStep::Outdent => t.checklist_outdent(),
        LineStep::Delete => t.delete(),
    }
}

/// The menu of a Line, from its ⋮ and a right-click on it: the step
/// clicked.
fn line_menu(ui: &mut Ui, t: &L10n, steps: &[LineStep]) -> Option<LineStep> {
    let text = ui.visuals().text_color();
    let mut picked = None;
    for step in steps {
        if ui
            .button(RichText::new(step_label(t, *step)).color(text))
            .clicked()
        {
            picked = Some(*step);
        }
    }
    picked
}

/// A Line's text, struck through and dimmed once checked.
fn line_text(ui: &Ui, line: &ShownLine) -> RichText {
    let words = RichText::new(&line.text).size(16.0);
    if line.checked {
        words.strikethrough().color(ui.visuals().weak_text_color())
    } else {
        words.color(ui.visuals().text_color())
    }
}

/// The frame a group is drawn in, under its parent.
fn group_frame(ui: &Ui) -> Frame {
    let stroke = ui.visuals().weak_text_color().gamma_multiply(0.5);
    Frame::NONE
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(4))
}

/// Draws `lines` one after another, each group as one frame under its
/// parent, every Line by `row`.
fn grouped(ui: &mut Ui, lines: &[ShownLine], row: &mut dyn FnMut(&mut Ui, &ShownLine)) {
    let mut at = 0;
    while at < lines.len() {
        if !lines[at].in_group {
            row(ui, &lines[at]);
            at += 1;
        }
        let start = at;
        while at < lines.len() && lines[at].in_group {
            at += 1;
        }
        if start == at {
            continue;
        }
        let group = &lines[start..at];
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            group_frame(ui).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for line in group {
                        row(ui, line);
                    }
                });
            });
        });
    }
}

/// The Lines that have a menu, by id, and the steps the menu of each
/// gives.
pub type LineMenus<'a> = (&'a HashSet<String>, &'a dyn Fn(&str) -> Vec<LineStep>);

/// The Lines that have a menu, by id, and what draws the menu of each,
/// by its id: what was picked in it (#122).
pub type MenuOf<'a, P> = (&'a HashSet<String>, &'a dyn Fn(&mut Ui, &str) -> Option<P>);

/// What was asked of a Line drawn by [`line_rows`]: its checkbox clicked,
/// with whether it is checked now, or something picked in its menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineAsk<P> {
    Check(String, bool),
    Menu(String, P),
}

/// The Lines of a Checklist, each with its checkbox, checkable with
/// `checks` for the Occurrence `on` (by their own checks without one),
/// and its text; with `menus`, the Lines whose ids it holds have ⋮ and a
/// right-click with the steps `steps` gives each. What is asked goes to
/// `actions`.
pub fn lines(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    shown: &[ShownLine],
    (checks, on): (bool, Option<&RoutineOccurrence>),
    menus: Option<LineMenus>,
    actions: &mut Vec<ChecklistAction>,
) {
    let menu = |ui: &mut Ui, id: &str| menus.and_then(|(_, steps)| line_menu(ui, t, &steps(id)));
    let menus = menus.map(|(editable, _)| {
        (
            editable,
            &menu as &dyn Fn(&mut Ui, &str) -> Option<LineStep>,
        )
    });
    for asked in line_rows(ui, (t, tones), shown, checks, menus) {
        actions.push(match asked {
            LineAsk::Check(id, checked) => ChecklistAction::Check(id, checked, on.cloned()),
            LineAsk::Menu(id, step) => ChecklistAction::Step(id, step),
        });
    }
}

/// The Lines `shown`, each with its checkbox, checkable with `checks`,
/// its text and what shows under it; with `menus`, the Lines whose ids
/// it holds have ⋮ and a right-click with the menu it draws. What is
/// asked, in the order clicked.
pub fn line_rows<P>(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    shown: &[ShownLine],
    checks: bool,
    menus: Option<MenuOf<P>>,
) -> Vec<LineAsk<P>> {
    let weak = ui.visuals().weak_text_color();
    let mut asked = Vec::new();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        grouped(ui, shown, &mut |ui, line| {
            ui.horizontal(|ui| {
                let toggled = ui
                    .add_enabled_ui(checks, |ui| check(ui, tones, line.checked, &line.text))
                    .inner;
                if toggled.clicked() {
                    asked.push(LineAsk::Check(line.id.clone(), !line.checked));
                }
                let editable = menus.filter(|(editable, _)| editable.contains(&line.id));
                let room = if editable.is_some() {
                    (ui.available_width() - 32.0).max(40.0)
                } else {
                    ui.available_width()
                };
                // Its words take the right-click where it has a menu.
                let sense = if editable.is_some() {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let words = ui
                    .allocate_ui(egui::vec2(room, 0.0), |ui| words(ui, line, sense))
                    .inner;
                let Some((_, menu)) = editable else {
                    return;
                };
                let mut picked = None;
                // A pick closes the menu, also one made with the keyboard
                // or a screen reader, which no pointer click closes.
                let mut entries = |ui: &mut Ui| {
                    let step = menu(ui, &line.id);
                    if step.is_some() {
                        ui.close();
                    }
                    picked = picked.take().or(step);
                };
                words.context_menu(&mut entries);
                // ⋮ at the right edge, as on an Agenda row.
                let more = ui
                    .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        icon_button(ui, icons::MORE_VERT, &menu_label(t, &line.text), weak)
                    })
                    .inner;
                egui::Popup::menu(&more).show(&mut entries);
                if let Some(step) = picked {
                    asked.push(LineAsk::Menu(line.id.clone(), step));
                }
            });
        });
    });
    asked
}

/// A Line's words, sensing as `sense` says, and under them, dimmed, what
/// shows below it: the response of the words.
fn words(ui: &mut Ui, line: &ShownLine, sense: Sense) -> egui::Response {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let words = ui.add(egui::Label::new(line_text(ui, line)).sense(sense));
        if let Some(below) = &line.below {
            let weak = ui.visuals().weak_text_color();
            ui.label(RichText::new(below).size(13.0).color(weak));
        }
        words
    })
    .inner
}

/// A Line's checkbox drawn on a card, where the card takes the clicks:
/// its place, the Line's id, text and whether it shows checked.
#[derive(Debug, Clone)]
pub struct CardBox {
    pub rect: egui::Rect,
    pub line: ShownLine,
}

/// The Lines of a Checklist on a full card (#120), under its text, each
/// checkbox only painted: the card lies over them, so [`card_checks`]
/// takes their clicks once the card is.
pub fn card_lines(ui: &mut Ui, tones: Tones, shown: &[ShownLine]) -> Vec<CardBox> {
    let mut boxes = Vec::new();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        grouped(ui, shown, &mut |ui, line| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
                paint_box(ui, tones, rect, line.checked, false);
                words(ui, line, Sense::hover());
                boxes.push(CardBox {
                    rect,
                    line: line.clone(),
                });
            });
        });
    });
    boxes
}

fn paint_box(ui: &Ui, tones: Tones, rect: egui::Rect, checked: bool, hovered: bool) {
    if hovered {
        ui.painter()
            .rect_filled(rect, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let (icon, colour) = if checked {
        (icons::CHECKED, tones.accent)
    } else {
        (icons::UNCHECKED, ui.visuals().text_color())
    };
    icons::paint(ui.painter(), rect.shrink(2.0), icon, colour);
}

/// Takes the clicks on the checkboxes `boxes` of the card of the Note
/// `note`, painted by [`card_lines`], checkable with `checks`: the Line
/// clicked and whether it is checked now.
pub fn card_checks(
    ui: &mut Ui,
    tones: Tones,
    note: &str,
    boxes: &[CardBox],
    checks: bool,
) -> Option<(String, bool)> {
    let mut clicked = None;
    let sense = if checks {
        Sense::click()
    } else {
        Sense::hover()
    };
    for b in boxes {
        let id = Id::new(("card line", note, &b.line.id));
        let resp = ui.interact(b.rect, id, sense);
        resp.widget_info(|| {
            WidgetInfo::selected(
                egui::WidgetType::Checkbox,
                checks,
                b.line.checked,
                &b.line.text,
            )
        });
        if checks && resp.hovered() && ui.is_rect_visible(b.rect) {
            paint_box(ui, tones, b.rect, b.line.checked, true);
        }
        if resp.clicked() {
            clicked = Some((b.line.id.clone(), !b.line.checked));
        }
        if checks {
            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
    clicked
}

/// How many Lines of `list` are checked, on a compact card.
pub fn card_count(ui: &mut Ui, t: &L10n, list: &Checklist) {
    let text = ui.visuals().text_color();
    let said = t.checklist_count(list.checked_count() as i64, list.count() as i64);
    ui.label(RichText::new(said).size(14.0).color(text));
}

/// The Checklist part of a Note's editor (#120), under its text: its
/// Lines, the field `typed` to add Lines, and "Uncheck all" and "Back to
/// a plain Note" as `shown` allows.
pub fn section(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    shown: &ChecklistShown,
    typed: &mut String,
    actions: &mut Vec<ChecklistAction>,
) {
    let text = ui.visuals().text_color();
    let list = &shown.list;
    let note = &list.note_id;
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.checklist_heading()).strong().color(text));
        let lines_shown = shown_lines(list);
        let steps = |id: &str| list.line(id).map(|l| steps(list, l)).unwrap_or_default();
        lines(
            ui,
            (t, tones),
            &lines_shown,
            (shown.checks, list.on.as_ref()),
            Some((&shown.editable, &steps)),
            actions,
        );
        if shown.adds {
            let id = Id::new("checklist add");
            if ui.memory(|m| m.has_focus(id))
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
                && !typed.trim().is_empty()
            {
                actions.push(ChecklistAction::Add(note.clone()));
            }
            ui.add(
                TextEdit::multiline(typed)
                    .id(id)
                    .hint_text(t.checklist_new_line())
                    .text_color(text)
                    .desired_rows(1)
                    .desired_width(f32::INFINITY),
            );
            let add = ui.add_enabled(
                !typed.trim().is_empty(),
                egui::Button::new(RichText::new(t.checklist_add_line()).color(text)),
            );
            if add.clicked() {
                actions.push(ChecklistAction::Add(note.clone()));
            }
        }
        let uncheck = shown.checks && list.checked_count() > 0;
        if uncheck || shown.plain {
            ui.horizontal_wrapped(|ui| {
                if uncheck
                    && ui
                        .button(RichText::new(t.checklist_uncheck_all()).color(text))
                        .clicked()
                {
                    actions.push(ChecklistAction::UncheckAll(note.clone()));
                }
                if shown.plain
                    && ui
                        .button(RichText::new(t.checklist_to_plain()).color(text))
                        .clicked()
                {
                    actions.push(ChecklistAction::ToPlain(note.clone()));
                }
            });
        }
    });
}

/// The Checklist entries of a card's menu as `menu` offers them: the
/// action clicked.
pub fn card_menu_entries(ui: &mut Ui, t: &L10n, menu: &CardMenu) -> Option<ChecklistAction> {
    let text = ui.visuals().text_color();
    let note = &menu.note;
    let entries = [
        (
            menu.make,
            t.make_checklist(),
            ChecklistAction::Make(note.clone()),
        ),
        (
            menu.uncheck_all,
            t.checklist_uncheck_all(),
            ChecklistAction::UncheckAll(note.clone()),
        ),
        (
            menu.to_plain,
            t.checklist_to_plain(),
            ChecklistAction::ToPlain(note.clone()),
        ),
    ];
    let mut picked = None;
    for (shown, label, action) in entries {
        if shown && ui.button(RichText::new(label).color(text)).clicked() {
            picked = Some(action);
        }
    }
    picked
}

/// The Lines of a Sent Item's preview, to look at, not to check.
pub fn preview(ui: &mut Ui, (t, tones): (&L10n, Tones), lines_sent: &[SentLine]) {
    let mut ignored = Vec::new();
    lines(
        ui,
        (t, tones),
        &sent_lines(t, lines_sent),
        (false, None),
        None,
        &mut ignored,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_renamed_line_is_one_trimmed_line_or_nothing() {
        assert_eq!(renamed("  Milk \r\n oat ").as_deref(), Some("Milk   oat"));
        assert_eq!(renamed(" \n "), None);
        assert_eq!(renamed("Bread").as_deref(), Some("Bread"));
    }
}
