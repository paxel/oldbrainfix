//! Shopping Lists on the desktop's screen (#122, ADR 0030), as the
//! phone's `shopping_list_view.dart` draws them: the Lines of a Shopping
//! List as a Checklist's are drawn, without groups, the open ones first
//! and the struck ones at the end with the day each comes back under it;
//! on a full card checkable, on a compact one "N open"; and the Shopping
//! List part of a Note's editor.
//!
//! The editor's part lists the Lines with their boxes, typed in place as
//! one block of text as a Checklist's are (#141, #184, `line_editing`):
//! each where this Device may change it, new ones by anyone with a role
//! in its Area, every one staying in its row while the editor is open; a
//! new list starts with one empty Line. Each Line ends in the badge of
//! its reappear time (#195), "—", "Once", "4w", whose click opens its
//! dialog where this Device may change the Line: None, Once or After,
//! After with its number and Day, Week, Month or Year, each written at
//! once. For such a Line, ⋮ and a right-click on its box's row hold
//! "Delete for good". "Back to a plain Note" is a badge
//! of the Note dialog (#160), as is "Show on the Agenda" (#186), whose
//! three ways the card's ⋮ offers as radio buttons too.
//! Nothing is written when a struck Line comes back: the window
//! reads the list again every half minute, so it shows open soon after
//! the start of its day.

use std::collections::HashSet;

use egui::{RichText, Ui};
use oldbrainfix_core::{
    Note, Reappear, ReappearUnit, ShoppingList, ShoppingLists, ShoppingOnAgenda, Store, WallClock,
};

use crate::app::embossed;
use crate::checklist_view::{self, CardMenu, LineAsk, ShownLine};
use crate::l10n::L10n;
use crate::line_editing::{self, FieldOf, TypedLine, TypedLines, Typing};
use crate::schedule_text::short_day_text;
use crate::setting_badges::{BadgeValue, badge};
use crate::theme::{self, Tones};
use crate::{icons, routine_editor};

/// What a Shopping List's Line is asked from its badge, its reappear time
/// dialog and its menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopStep {
    /// Opens its reappear time dialog (#195).
    PickReappear,
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
    /// What a Line's field asked as it was typed in (#141, #184).
    Typed(Typing),
    /// A step of the menu of the Line with this id.
    Step(String, ShopStep),
    /// Chooses how the Note's Shopping List shows on this person's
    /// Agenda (#186).
    OnAgenda(String, ShoppingOnAgenda),
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
    /// How it shows on this person's Agenda, while "Show on the Agenda"
    /// applies to it (#186).
    pub on_agenda: Option<ShoppingOnAgenda>,
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
        on_agenda: on_agenda(&lists, &note.id)?,
        list,
    }))
}

/// How the Shopping List of `note` shows on this person's Agenda, or
/// `None` while "Show on the Agenda" does not apply to it (#186): offered
/// while no Line is open too, for when Lines come back.
pub fn on_agenda(
    lists: &ShoppingLists,
    note: &str,
) -> oldbrainfix_core::Result<Option<ShoppingOnAgenda>> {
    if !lists.may_show_on_agenda(note)? {
        return Ok(None);
    }
    Ok(Some(lists.on_agenda(note)?))
}

/// The words of a way a Shopping List or a Checklist shows on the Agenda.
pub fn on_agenda_label(t: &L10n, shows: ShoppingOnAgenda) -> &'static str {
    match shows {
        ShoppingOnAgenda::Off => t.setting_off(),
        ShoppingOnAgenda::Count => t.shopping_on_agenda_count(),
        ShoppingOnAgenda::Lines => t.shopping_on_agenda_lines(),
    }
}

/// The way `shows` as the value icon of its badge (#192), the phone's
/// `shoppingOnAgendaIcon`.
pub fn on_agenda_icon(shows: ShoppingOnAgenda) -> char {
    match shows {
        ShoppingOnAgenda::Off => crate::setting_badges::switch_icon(false),
        ShoppingOnAgenda::Count => crate::icons::COUNT,
        ShoppingOnAgenda::Lines => crate::icons::FORMAT_BULLETS,
    }
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

/// The words of a reappear time: "None", "Once", "2 weeks", "3 months".
pub fn reappear_label(t: &L10n, reappear: Reappear) -> String {
    match reappear {
        Reappear::Unset => t.shopping_reappear_none().to_string(),
        Reappear::Once => t.shopping_once().to_string(),
        Reappear::After(count, unit) => {
            let count = i64::from(count);
            match unit {
                ReappearUnit::Day => t.shopping_days(count),
                ReappearUnit::Week => t.shopping_weeks(count),
                ReappearUnit::Month => t.shopping_months(count),
                ReappearUnit::Year => t.shopping_years(count),
            }
        }
    }
}

/// A reappear time as its badge shows it, short text (#192, #195): "—"
/// for none, "Once", "4w", "3m".
pub fn reappear_short(t: &L10n, reappear: Reappear) -> String {
    match reappear {
        Reappear::Unset => "—".to_string(),
        Reappear::Once => t.shopping_once().to_string(),
        Reappear::After(count, unit) => {
            let count = i64::from(count);
            match unit {
                ReappearUnit::Day => t.shopping_short_days(count),
                ReappearUnit::Week => t.shopping_short_weeks(count),
                ReappearUnit::Month => t.shopping_short_months(count),
                ReappearUnit::Year => t.shopping_short_years(count),
            }
        }
    }
}

/// The badge of a Line's reappear time at its end (#195), read as
/// "Reappear time: 4 weeks"; clicked while `active`, it asks for its
/// dialog.
fn reappear_badge(
    ui: &mut Ui,
    shown: (&L10n, Tones),
    reappear: Reappear,
    active: bool,
) -> Option<ShopStep> {
    let t = shown.0;
    let short = reappear_short(t, reappear);
    let words = reappear_label(t, reappear);
    let value = BadgeValue::Short(&short, &words);
    let clicked = badge(
        ui,
        shown,
        (icons::REAPPEAR, None),
        t.shopping_reappear(),
        Some(value),
        active,
    )
    .clicked();
    (active && clicked).then_some(ShopStep::PickReappear)
}

/// What a Line's reappear time dialog shows of `current` (#195): None,
/// Once or After as choice chips, After with its number from 1 to 99 and
/// Day, Week, Month or Year under it; the time picked, written at once by
/// the caller. After starts at a week.
pub fn reappear_editor(ui: &mut Ui, t: &L10n, current: Reappear) -> Option<Reappear> {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        None,
        Once,
        After,
    }
    let mut kind = match current {
        Reappear::Unset => Kind::None,
        Reappear::Once => Kind::Once,
        Reappear::After(..) => Kind::After,
    };
    let options = [
        (Kind::None, t.shopping_reappear_none().to_string()),
        (Kind::Once, t.shopping_once().to_string()),
        (Kind::After, t.shopping_reappear_after().to_string()),
    ];
    if crate::app::chip_row(ui, &mut kind, options) {
        return Some(match kind {
            Kind::None => Reappear::Unset,
            Kind::Once => Reappear::Once,
            Kind::After => Reappear::FALLBACK,
        });
    }
    let Reappear::After(count, unit) = current else {
        return None;
    };
    let mut number = i64::from(count);
    let most = Some(i64::from(Reappear::MAX_COUNT));
    if routine_editor::stepper(ui, t, &mut number, (1, most), None) {
        return u32::try_from(number)
            .ok()
            .and_then(|n| Reappear::after(n, unit));
    }
    let mut picked = unit;
    let units = ReappearUnit::ALL.map(|u| {
        let words = match u {
            ReappearUnit::Day => t.unit_days(number),
            ReappearUnit::Week => t.unit_weeks(number),
            ReappearUnit::Month => t.unit_months(number),
            ReappearUnit::Year => t.unit_years(number),
        };
        (u, words)
    });
    (crate::app::chip_row(ui, &mut picked, units) && picked != unit)
        .then_some(Reappear::After(count, picked))
}

/// The menu of a Line, from its ⋮ and a right-click on it: "Delete for
/// good"; the step clicked.
fn line_menu(ui: &mut Ui, t: &L10n) -> Option<ShopStep> {
    theme::destructive(ui, true, t.shopping_delete_for_good())
        .clicked()
        .then_some(ShopStep::Delete)
}

/// How many Lines of `list` are open, on a compact card.
pub fn card_count(ui: &mut Ui, t: &L10n, list: &ShoppingList) {
    let text = ui.visuals().text_color();
    let said = t.shopping_open_count(list.open_count() as i64);
    ui.label(RichText::new(said).size(14.0).color(text));
}

/// The Shopping List part of a Note's editor (#122), under its text: its
/// Lines as they are typed in `typed`, in their rows, each typed in place
/// where this Device may (#141, #184).
pub fn section(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    shown: &ShoppingShown,
    typed: &mut TypedLines,
    actions: &mut Vec<ShoppingAction>,
) {
    let text = ui.visuals().text_color();
    let list = &shown.list;
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.shopping_heading()).strong().color(text));
        let menu = |ui: &mut Ui, _: &str| line_menu(ui, t);
        let menued = typed.with_menus(&shown.editable);
        let menus = (&menued, &menu as &dyn Fn(&mut Ui, &str) -> Option<ShopStep>);
        let lines = shown_lines(t, list);
        let rows: Vec<ShownLine> = typed
            .rows
            .iter()
            .map(|row| {
                let line = row
                    .id
                    .as_ref()
                    .and_then(|id| lines.iter().find(|l| &l.id == id));
                match line {
                    Some(line) => ShownLine {
                        text: row.typed(),
                        ..line.clone()
                    },
                    // New, or gone from the list a moment ago.
                    None => ShownLine {
                        id: row.shown_id(),
                        text: row.typed(),
                        in_group: false,
                        checked: false,
                        below: None,
                    },
                }
            })
            .collect();
        // A new Line, and one this Device may change, by anyone who may add.
        let in_place = |row: &TypedLine| {
            shown.checks && row.id.as_ref().is_none_or(|id| shown.editable.contains(id))
        };
        let fields: Vec<u64> = typed
            .rows
            .iter()
            .filter(|r| in_place(r))
            .map(|r| r.key)
            .collect();
        let mut typing = Vec::new();
        let mut field = |ui: &mut Ui, line: &ShownLine, width: f32| {
            let Some(key) = typed.shown(&line.id).filter(|r| in_place(r)).map(|r| r.key) else {
                return false;
            };
            let at = fields.iter().position(|k| *k == key);
            let above = at
                .and_then(|at| at.checked_sub(1))
                .and_then(|at| fields.get(at))
                .copied();
            let below = at.and_then(|at| fields.get(at + 1)).copied();
            line_editing::field(
                ui,
                typed,
                (key, FieldOf::Shopping),
                (line.checked, width),
                (above, below),
                &mut typing,
            );
            true
        };
        // A new Line has none yet; a click stores it first.
        let trailing = |ui: &mut Ui, line: &ShownLine| {
            let (reappear, active) = match list.line(&line.id) {
                Some(stored) => (stored.reappear, shown.editable.contains(&line.id)),
                None => (Reappear::Unset, shown.checks && menued.contains(&line.id)),
            };
            reappear_badge(ui, (t, tones), reappear, active)
        };
        let asked = checklist_view::line_rows(
            ui,
            (t, tones),
            &rows,
            shown.checks,
            (Some(menus), Some(&mut field), Some(&trailing)),
        );
        for asked in asked {
            actions.push(match asked {
                LineAsk::Check(id, checked) => ShoppingAction::Check(id, checked),
                LineAsk::Menu(id, step) => ShoppingAction::Step(id, step),
            });
        }
        // Back to a plain Note is the Note dialog's kind badge (#160).
        actions.extend(typing.into_iter().map(ShoppingAction::Typed));
    });
}

/// "Show on the Agenda" of a Shopping List in a card's menu as `menu`
/// offers it (#186): its three ways as radio buttons under their heading,
/// before "Back to a plain Note" as on the phone; the way clicked. A
/// click on the way it has already changes nothing.
pub fn card_on_agenda(ui: &mut Ui, t: &L10n, menu: &CardMenu) -> Option<ShoppingAction> {
    let current = menu.shopping_on_agenda?;
    agenda_radios(ui, t, current).map(|shows| ShoppingAction::OnAgenda(menu.note.clone(), shows))
}

/// "Show on the Agenda" of a Shopping List or a Checklist (#219) in a
/// card's menu: its three ways as radio buttons under their heading,
/// `current` marked; the other way clicked.
pub fn agenda_radios(ui: &mut Ui, t: &L10n, current: ShoppingOnAgenda) -> Option<ShoppingOnAgenda> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut picked = None;
    ui.separator();
    ui.label(
        RichText::new(t.checklist_show_on_agenda())
            .small()
            .color(weak),
    );
    for shows in ShoppingOnAgenda::ALL {
        let on = shows == current;
        let label = RichText::new(on_agenda_label(t, shows)).color(text);
        if ui.radio(on, label).clicked() && !on {
            picked = Some(shows);
        }
    }
    ui.separator();
    picked
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
