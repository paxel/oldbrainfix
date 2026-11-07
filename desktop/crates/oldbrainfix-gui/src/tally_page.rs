//! The New Tally page (#161), as the phone's `tally_form_screen.dart`
//! draws it, in a dialog over the window: its name, then a row per
//! field, one at first, with "Add field" for the next. A field's row is
//! its name and its settings as badges ([`crate::setting_badges`]): its
//! kind (#264), a number or a Check field, how several sets or entries
//! in a bar count and its unit, which a Check field has not, and its
//! target, each opening its small editor. The unit is typed with the known units it
//! may mean as suggestions under it (#159), and the rule follows its kind
//! until it is picked. Save files the Tally where the page was opened
//! from, Unsorted for none, with every row that has a name; rows left
//! without one are dropped. Leaving without Save makes nothing.

use egui::{Id, RichText, TextEdit, Ui};
use oldbrainfix_core::tally_units;
use oldbrainfix_core::{Notes, Store, Tallies, TallyRule, UnitSystem};

use crate::app::{chip_row, embossed};
use crate::icons;
use crate::l10n::L10n;
use crate::setting_badges::{BadgeValue, badge, badge_dialog, badge_row};
use crate::tally_unit_match::{self, UnitNames};
use crate::tally_view::{self, kind_name};
use crate::theme::{self, Tones};

/// What was asked of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageIntent {
    Save,
    Close,
}

/// The setting of a field whose small editor is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Editing {
    Kind,
    Rule,
    Unit,
    Target,
}

/// One field as it is typed: its name, its unit as typed and the known
/// one a suggestion picked, its rule, which follows the unit until it is
/// picked, and its target as typed.
#[derive(Debug, Clone)]
struct FieldDraft {
    /// Keeps its text fields' ids while rows are added.
    key: u64,
    name: String,
    unit_text: String,
    /// The code of the known unit a suggestion put into the unit, until
    /// it is typed in again.
    picked: Option<&'static str>,
    rule: TallyRule,
    rule_picked: bool,
    /// Whether it is a Check field (#264): no unit and no rule.
    check: bool,
    target: String,
    /// Set by a Save that found the target no number.
    target_invalid: bool,
}

impl FieldDraft {
    fn new(key: u64) -> FieldDraft {
        FieldDraft {
            key,
            name: String::new(),
            unit_text: String::new(),
            picked: None,
            rule: TallyRule::Sum,
            rule_picked: false,
            check: false,
            target: String::new(),
            target_invalid: false,
        }
    }

    /// The unit as it is stored; a Check field has none.
    fn unit(&self, system: UnitSystem) -> String {
        if self.check {
            return String::new();
        }
        match self.picked {
            Some(code) => code.to_string(),
            None => tally_unit_match::unit_of_typed(&self.unit_text, system),
        }
    }

    /// The target in the base unit; `None` for none or no number.
    fn target(&self, system: UnitSystem) -> Option<f64> {
        let text = self.target.trim();
        if text.is_empty() {
            return None;
        }
        tally_units::parse_shown(text, &self.unit(system), system)
    }

    /// The rule follows the kind of unit until it is picked.
    fn unit_changed(&mut self, system: UnitSystem) {
        if !self.rule_picked {
            self.rule = tally_units::default_rule(tally_units::kind_of(&self.unit(system)));
        }
    }
}

/// The New Tally page as it is filled in.
#[derive(Debug, Clone)]
pub struct TallyPage {
    /// The Area or Topic it goes into; `None` for none.
    pub place: Option<String>,
    /// Whether it was opened from the New Session page, which gets the
    /// Tally made, ticked in its pool.
    pub for_session: bool,
    name: String,
    fields: Vec<FieldDraft>,
    name_missing: bool,
    fields_missing: bool,
    system: UnitSystem,
    /// The names the known units are matched by, in every language.
    unit_names: UnitNames,
    /// The field, by its row's key, whose small editor is open.
    editing: Option<(u64, Editing)>,
    /// The text field to take the keyboard, until it has it.
    focus: Option<Id>,
    next: u64,
}

const NAME_FIELD: &str = "new tally name";

fn field_id(key: u64) -> Id {
    Id::new(("new tally field", key))
}

impl TallyPage {
    /// An empty page, filed into `place`, for units in `system`.
    pub fn new(place: Option<String>, for_session: bool, system: UnitSystem) -> TallyPage {
        TallyPage {
            place,
            for_session,
            name: String::new(),
            fields: vec![FieldDraft::new(1)],
            name_missing: false,
            fields_missing: false,
            system,
            unit_names: tally_unit_match::unit_names(system),
            editing: None,
            focus: Some(Id::new(NAME_FIELD)),
            next: 1,
        }
    }

    /// The page with its name starting as `name`, such as the tags a
    /// Session's pool is filtered by (#220).
    pub fn with_name(mut self, name: String) -> TallyPage {
        self.name = name;
        self
    }

    /// The name as typed, without the blanks around it.
    pub fn name(&self) -> &str {
        self.name.trim()
    }

    /// Writes the Tally when nothing needs fixing, its Note filed into
    /// [`TallyPage::place`] with every field that has a name, in their
    /// order; the new Note's id, or `None` while something needs fixing,
    /// which then says so and takes the keyboard.
    pub fn save(&mut self, store: &Store) -> oldbrainfix_core::Result<Option<String>> {
        let name = self.name.trim().to_string();
        let system = self.system;
        self.name_missing = name.is_empty();
        let kept: Vec<usize> = self
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.name.trim().is_empty())
            .map(|(i, _)| i)
            .collect();
        self.fields_missing = kept.is_empty();
        for (i, field) in self.fields.iter_mut().enumerate() {
            field.target_invalid = kept.contains(&i)
                && !field.target.trim().is_empty()
                && field.target(system).is_none();
        }
        if self.name_missing {
            self.focus = Some(Id::new(NAME_FIELD));
            return Ok(None);
        }
        if self.fields_missing {
            self.focus = self.fields.first().map(|f| field_id(f.key));
            return Ok(None);
        }
        if let Some(field) = self.fields.iter().find(|f| f.target_invalid) {
            self.focus = Some(field_id(field.key));
            return Ok(None);
        }
        let fields: Vec<&FieldDraft> = self
            .fields
            .iter()
            .filter(|f| !f.name.trim().is_empty())
            .collect();
        let place = self.place.as_deref();
        let id = store.batch(|| {
            let note = Notes::new(store).capture(&name, place, None)?;
            let tallies = Tallies::new(store);
            for field in &fields {
                if field.check {
                    tallies.add_check(&note.id, field.name.trim(), field.target(system))?;
                } else {
                    tallies.add_field(
                        &note.id,
                        field.name.trim(),
                        &field.unit(system),
                        field.target(system),
                        Some(field.rule),
                    )?;
                }
            }
            Ok(note.id)
        })?;
        Ok(Some(id))
    }

    /// Draws the page and the small editor open over it; what was asked.
    pub fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<PageIntent> {
        let text = ui.visuals().text_color();
        let error = ui.visuals().error_fg_color;
        let mut intent = None;
        let focus = self.focus;
        let system = self.system;
        let fields_missing = self.fields_missing;
        // Whether the field asked for took the keyboard, and the badge
        // clicked.
        let mut focused = false;
        let mut opened_badge = None;
        egui::ScrollArea::vertical()
            .id_salt("new tally")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                ui.label(RichText::new(t.new_tally()).size(18.0).strong().color(text));
                ui.horizontal_wrapped(|ui| {
                    if theme::primary(ui, true, t.save()).clicked() {
                        intent = Some(PageIntent::Save);
                    }
                    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
                        intent = Some(PageIntent::Close);
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    let label = ui.label(RichText::new(t.kind_name()).strong().color(text));
                    let field = ui
                        .add(
                            TextEdit::singleline(&mut self.name)
                                .id(Id::new(NAME_FIELD))
                                .text_color(text)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(label.id);
                    crate::text_menu::show(ui, t, &field, &self.name);
                    if focus == Some(field.id) {
                        field.request_focus();
                        focused |= field.has_focus();
                    }
                    if self.name_missing {
                        ui.label(RichText::new(t.kind_name_missing()).color(error));
                    }
                });
                ui.label(RichText::new(t.tally_fields()).strong().color(text));
                for (index, field) in self.fields.iter_mut().enumerate() {
                    embossed(ui, tones, false, None, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        let label =
                            ui.label(RichText::new(t.tally_field_name()).strong().color(text));
                        let id = field_id(field.key);
                        let typed = ui
                            .add(
                                TextEdit::singleline(&mut field.name)
                                    .id(id)
                                    .hint_text(t.tally_field_name_hint())
                                    .text_color(text)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(label.id);
                        crate::text_menu::show(ui, t, &typed, &field.name);
                        if focus == Some(id) {
                            typed.request_focus();
                            focused |= typed.has_focus();
                        }
                        if index == 0 && fields_missing {
                            ui.label(RichText::new(t.tally_fields_missing()).color(error));
                        }
                        let unit = tally_units::shown_unit(&field.unit(system), system);
                        let target = field.target.trim();
                        let rule = tally_view::rule_value(t, field.rule);
                        let unit_value = if unit.is_empty() {
                            t.tally_no_unit().to_string()
                        } else {
                            unit.clone()
                        };
                        let target_value = match (target.is_empty(), unit.is_empty()) {
                            (true, _) => t.tally_no_target().to_string(),
                            (false, true) => target.to_string(),
                            (false, false) => format!("{target} {unit}"),
                        };
                        let shown = (t, tones);
                        let mut opened = None;
                        let kind = BadgeValue::Text(if field.check {
                            t.tally_field_kind_check()
                        } else {
                            t.tally_field_kind_number()
                        });
                        badge_row(ui, |ui| {
                            let mut badges = vec![(
                                icons::TALLY_CHECK,
                                t.tally_field_kind(),
                                kind,
                                Editing::Kind,
                            )];
                            if !field.check {
                                badges.push((
                                    icons::TALLY_RULE,
                                    t.tally_rule(),
                                    rule,
                                    Editing::Rule,
                                ));
                                badges.push((
                                    icons::TALLY_UNIT,
                                    t.tally_unit(),
                                    BadgeValue::Text(&unit_value),
                                    Editing::Unit,
                                ));
                            }
                            badges.push((
                                icons::TALLY_TARGET,
                                t.tally_target_badge(),
                                BadgeValue::Text(&target_value),
                                Editing::Target,
                            ));
                            for (icon, setting, value, editing) in badges {
                                if badge(ui, shown, (icon, None), setting, Some(value), true)
                                    .clicked()
                                {
                                    opened = Some(editing);
                                }
                            }
                        });
                        if let Some(editing) = opened {
                            opened_badge = Some((field.key, editing));
                        }
                        if field.target_invalid {
                            ui.label(RichText::new(t.tally_not_a_number()).color(error));
                        }
                    });
                }
                let add = ui.horizontal(|ui| {
                    icons::show(ui, icons::ADD, 20.0, text);
                    ui.button(RichText::new(t.tally_add_field()).color(text))
                });
                if add.inner.clicked() {
                    self.next += 1;
                    let field = FieldDraft::new(self.next);
                    self.focus = Some(field_id(field.key));
                    self.fields.push(field);
                }
            });
        // Not in the dialog's first, sizing pass, which drops a focus.
        if focused && !ui.is_sizing_pass() {
            self.focus = None;
        }
        if opened_badge.is_some() {
            self.editing = opened_badge;
        }
        self.editor(ui.ctx(), t, tones);
        intent
    }

    /// The small editor of the badge clicked, while it is open.
    fn editor(&mut self, ctx: &egui::Context, t: &L10n, tones: Tones) {
        let Some((key, editing)) = self.editing else {
            return;
        };
        let system = self.system;
        let names = &self.unit_names;
        let Some(field) = self.fields.iter_mut().find(|f| f.key == key) else {
            self.editing = None;
            return;
        };
        let shown = (t, tones);
        let mut done = false;
        let closed = match editing {
            Editing::Kind => badge_dialog(
                ctx,
                shown,
                (icons::TALLY_CHECK, None),
                t.tally_field_kind(),
                |ui| {
                    let options = [
                        (false, t.tally_field_kind_number().to_string()),
                        (true, t.tally_field_kind_check().to_string()),
                    ];
                    if chip_row(ui, &mut field.check, options) {
                        done = true;
                    }
                    let weak = ui.visuals().weak_text_color();
                    let detail = t.tally_field_kind_check_detail();
                    ui.label(RichText::new(detail).small().color(weak));
                },
            ),
            Editing::Rule => badge_dialog(
                ctx,
                shown,
                (icons::TALLY_RULE, None),
                t.tally_rule(),
                |ui| {
                    let options = [
                        (TallyRule::Sum, t.tally_rule_sum().to_string()),
                        (TallyRule::Max, t.tally_rule_max().to_string()),
                    ];
                    if chip_row(ui, &mut field.rule, options) {
                        field.rule_picked = true;
                        done = true;
                    }
                },
            ),
            Editing::Unit => badge_dialog(
                ctx,
                shown,
                (icons::TALLY_UNIT, None),
                t.tally_unit(),
                |ui| {
                    let text = ui.visuals().text_color();
                    let weak = ui.visuals().weak_text_color();
                    let id = Id::new("new tally unit");
                    let typed = ui.add(
                        TextEdit::singleline(&mut field.unit_text)
                            .id(id)
                            .hint_text(t.tally_unit_hint())
                            .text_color(text)
                            .desired_width(f32::INFINITY),
                    );
                    crate::text_menu::show(ui, t, &typed, &field.unit_text);
                    if !typed.has_focus()
                        && !typed.lost_focus()
                        && ui.memory(|m| m.focused().is_none())
                    {
                        typed.request_focus();
                    }
                    let mut changed = typed.changed();
                    if changed {
                        field.picked = None;
                    }
                    if typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        done = true;
                    }
                    ui.label(RichText::new(t.tally_unit_helper()).small().color(weak));
                    let matches =
                        tally_unit_match::matching_units(&field.unit_text, system, names, None);
                    if !matches.is_empty() {
                        // Suggestions that fill in the field, not a choice of
                        // their own.
                        ui.horizontal_wrapped(|ui| {
                            for known in matches {
                                let unit = tally_units::shown_unit(known.code, system);
                                let chip = ui
                                    .small_button(RichText::new(&unit).color(text))
                                    .on_hover_text(kind_name(t, known.kind));
                                if chip.clicked() {
                                    field.picked = Some(known.code);
                                    field.unit_text = unit;
                                    changed = true;
                                }
                            }
                        });
                    }
                    if changed {
                        field.unit_changed(system);
                    }
                },
            ),
            Editing::Target => badge_dialog(
                ctx,
                shown,
                (icons::TALLY_TARGET, None),
                t.tally_target_badge(),
                |ui| {
                    let text = ui.visuals().text_color();
                    let weak = ui.visuals().weak_text_color();
                    let label = ui.label(RichText::new(t.tally_target()).color(text));
                    let unit = tally_units::shown_unit(&field.unit(system), system);
                    ui.horizontal(|ui| {
                        let typed = ui
                            .add(
                                TextEdit::singleline(&mut field.target)
                                    .id(Id::new("new tally target"))
                                    .text_color(text)
                                    .desired_width(120.0),
                            )
                            .labelled_by(label.id);
                        crate::text_menu::show(ui, t, &typed, &field.target);
                        if !typed.has_focus()
                            && !typed.lost_focus()
                            && ui.memory(|m| m.focused().is_none())
                        {
                            typed.request_focus();
                        }
                        if typed.changed() {
                            field.target_invalid = false;
                        }
                        if typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            done = true;
                        }
                        ui.label(RichText::new(unit).color(text));
                    });
                    let typed = field.target.trim();
                    if !typed.is_empty() && field.target(system).is_none() {
                        ui.label(
                            RichText::new(t.tally_not_a_number())
                                .color(ui.visuals().error_fg_color),
                        );
                    }
                    ui.label(RichText::new(t.tally_target_hint()).small().color(weak));
                },
            ),
        };
        if closed || done {
            self.editing = None;
        }
    }
}
