//! Shopping Lists on the desktop's screen (#122, ADR 0030), as the
//! phone's `shopping_list_view.dart` draws them: the Lines of a Shopping
//! List as a Checklist's are drawn, without groups, the open ones first
//! and the struck ones at the end with the day each comes back under it;
//! on a full card checkable, on a compact one "N open"; and the Shopping
//! List part of a Note's editor.
//!
//! The editor's part has at the top a field and "Add" for anyone with a
//! role in its Area, every line typed one Line, the first typed at the
//! top; then the Lines to check; for who may change a Line, ⋮ and a
//! right-click with Rename, its reappear time as radio buttons and
//! "Delete for good"; and "Back to a plain Note" for who may change the
//! Note. Nothing is written when a struck Line comes back: the window
//! reads the list again every half minute, so it shows open soon after
//! the start of its day.

use std::collections::HashSet;

use egui::{Id, Key, Modifiers, RichText, TextEdit, Ui};
use oldbrainfix_core::checklists::split_lines;
use oldbrainfix_core::{Note, Reappear, ShoppingList, ShoppingLists, Store, WallClock};

use crate::app::embossed;
use crate::checklist_view::{self, CardMenu, LineAsk, ShownLine};
use crate::l10n::L10n;
use crate::schedule_text::short_day_text;
use crate::theme::Tones;

/// What the menu of a Shopping List's Line does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopStep {
    Rename,
    Reappear(Reappear),
    Delete,
}

/// Something asked of a Shopping List, on a card or in the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShoppingAction {
    /// Makes the Note with this id a Shopping List.
    Make(String),
    /// Turns the Shopping List of the Note back into a plain Note.
    ToPlain(String),
    /// Checks the Line with this id, striking it, or unchecks it.
    Check(String, bool),
    /// Adds every line typed into the add field to the Note's list.
    Add(String),
    /// A step of the menu of the Line with this id.
    Step(String, ShopStep),
}

/// The Shopping List of the selected Note as its editor shows it.
#[derive(Debug, Clone)]
pub struct ShoppingShown {
    pub list: ShoppingList,
    /// Whether this Device may check and add Lines.
    pub checks: bool,
    /// The Lines it may rename, delete and give a reappear time, by id.
    pub editable: HashSet<String>,
    /// Whether it may turn it back into a plain Note.
    pub plain: bool,
}

/// Reads the Shopping List of the Note `note` as it shows now, days as
/// `wall` shows them, or `None` while it is none; `changes` says whether
/// this Device may change the Note.
pub fn read(
    store: &Store,
    note: &Note,
    changes: bool,
    wall: &WallClock,
) -> oldbrainfix_core::Result<Option<ShoppingShown>> {
    let lists = ShoppingLists::new(store);
    let Some(list) = lists.by_note(&note.id, wall)? else {
        return Ok(None);
    };
    let mut editable = HashSet::new();
    for line in &list.lines {
        if lists.may_edit(&line.id)? {
            editable.insert(line.id.clone());
        }
    }
    Ok(Some(ShoppingShown {
        checks: lists.may_check(&note.id)?,
        editable,
        plain: changes && note.text_editable,
        list,
    }))
}

/// The Lines of `list` as they are drawn: open ones first, struck ones at
/// the end with the day they come back under them, as a short weekday
/// and date ("back Sat, Oct 31").
pub fn shown_lines(t: &L10n, list: &ShoppingList) -> Vec<ShownLine> {
    list.lines
        .iter()
        .map(|line| ShownLine {
            id: line.id.clone(),
            text: line.text.clone(),
            in_group: false,
            checked: line.struck,
            below: line
                .back_on
                .filter(|_| line.struck)
                .map(|day| t.shopping_back(&short_day_text(t, day))),
        })
        .collect()
}

/// Adds a Line to the Shopping List of `note_id` for every line of
/// `typed` that is not blank, the first typed ending at the top; a name
/// the list has already brings its Line back instead. Whether any line
/// was taken.
pub fn add_typed(
    store: &Store,
    note_id: &str,
    typed: &str,
    wall: &WallClock,
) -> oldbrainfix_core::Result<bool> {
    let lists = ShoppingLists::new(store);
    let rows: Vec<String> = split_lines(typed)
        .into_iter()
        .filter(|row| !row.trim().is_empty())
        .collect();
    for row in rows.iter().rev() {
        lists.add(note_id, row, wall)?;
    }
    Ok(!rows.is_empty())
}

/// The words of a reappear time: "Once" or "2 weeks".
pub fn reappear_label(t: &L10n, reappear: Reappear) -> String {
    match reappear {
        Reappear::Once => t.shopping_once().to_string(),
        weeks => t.shopping_weeks(weeks.weeks()),
    }
}

/// The menu of a Line whose reappear time is `current`, from its ⋮ and a
/// right-click on it: Rename, the reappear times as radio buttons under
/// their heading and "Delete for good"; the step clicked. A click on the
/// time it has already changes nothing.
fn line_menu(ui: &mut Ui, t: &L10n, current: Reappear) -> Option<ShopStep> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut picked = None;
    if ui.button(RichText::new(t.rename()).color(text)).clicked() {
        picked = Some(ShopStep::Rename);
    }
    ui.separator();
    ui.label(RichText::new(t.shopping_reappear()).small().color(weak));
    for reappear in Reappear::ALL {
        let on = reappear == current;
        let label = reappear_label(t, reappear);
        if ui.radio(on, RichText::new(label).color(text)).clicked() && !on {
            picked = Some(ShopStep::Reappear(reappear));
        }
    }
    ui.separator();
    if ui
        .button(RichText::new(t.shopping_delete_for_good()).color(text))
        .clicked()
    {
        picked = Some(ShopStep::Delete);
    }
    picked
}

/// How many Lines of `list` are open, on a compact card.
pub fn card_count(ui: &mut Ui, t: &L10n, list: &ShoppingList) {
    let text = ui.visuals().text_color();
    let said = t.shopping_open_count(list.open_count() as i64);
    ui.label(RichText::new(said).size(14.0).color(text));
}

/// The Shopping List part of a Note's editor (#122), under its text: the
/// field `typed` and "Add" at the top, where a new Line shows, its Lines,
/// and "Back to a plain Note" as `shown` allows.
pub fn section(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    shown: &ShoppingShown,
    typed: &mut String,
    actions: &mut Vec<ShoppingAction>,
) {
    let text = ui.visuals().text_color();
    let list = &shown.list;
    let note = &list.note_id;
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.shopping_heading()).strong().color(text));
        if shown.checks {
            let id = Id::new("shopping add");
            if ui.memory(|m| m.has_focus(id))
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
                && !typed.trim().is_empty()
            {
                actions.push(ShoppingAction::Add(note.clone()));
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
                egui::Button::new(RichText::new(t.add()).color(text)),
            );
            if add.clicked() {
                actions.push(ShoppingAction::Add(note.clone()));
            }
        }
        let menu = |ui: &mut Ui, id: &str| {
            let current = list.line(id).map_or(Reappear::STANDARD, |l| l.reappear);
            line_menu(ui, t, current)
        };
        let menus = (
            &shown.editable,
            &menu as &dyn Fn(&mut Ui, &str) -> Option<ShopStep>,
        );
        let rows = shown_lines(t, list);
        for asked in checklist_view::line_rows(ui, (t, tones), &rows, shown.checks, Some(menus)) {
            actions.push(match asked {
                LineAsk::Check(id, checked) => ShoppingAction::Check(id, checked),
                LineAsk::Menu(id, step) => ShoppingAction::Step(id, step),
            });
        }
        if shown.plain
            && ui
                .button(RichText::new(t.checklist_to_plain()).color(text))
                .clicked()
        {
            actions.push(ShoppingAction::ToPlain(note.clone()));
        }
    });
}

/// The Shopping List entry of a card's menu as `menu` offers it, making
/// the Note one: whether it was clicked.
pub fn card_menu_entry(ui: &mut Ui, t: &L10n, menu: &CardMenu) -> Option<ShoppingAction> {
    let text = ui.visuals().text_color();
    (menu.make_shopping
        && ui
            .button(RichText::new(t.make_shopping_list()).color(text))
            .clicked())
    .then(|| ShoppingAction::Make(menu.note.clone()))
}
