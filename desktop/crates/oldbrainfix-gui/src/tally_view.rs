//! Tallies on the desktop's screen (#103, ADR 0028), as the phone's
//! `tally_view.dart`, `tally_field_screen.dart` and
//! `tally_entry_screen.dart` draw them: the Tally part of a Note's
//! editor, and the field page and the entry page, which take the right
//! pane as the Routine editor does, Save at their top.
//!
//! The Tally part lists the fields, each opening its field page for who
//! may change the Note, with "Add entry" and "Add field"; in a Shared
//! Area the choice of whose entries show, the user's own by default; the
//! length of a bar; one chart per field shown; the entries, newest first,
//! 30 at a time, each clicked to edit it and right-clicked for a menu;
//! and a folded "Removed entries" with Restore, closed until opened,
//! remembered on this Device.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{DateTime, Duration, Utc};
use egui::{Id, RichText, Sense, TextEdit, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::tally_units::{self, KNOWN};
use oldbrainfix_core::{
    Day, PersonFilter, Profile, Store, Tallies, Tally, TallyBucket, TallyEntry, TallyField,
    TallyPeriod, TallyRule, UnitKind, UnitSystem, WallClock,
};

use crate::app::{embossed, icon_button, radio_column, when};
use crate::icons;
use crate::l10n::L10n;
use crate::profile_view;
use crate::schedule_text::{day_text, month_short};
use crate::theme::Tones;

/// The local setting that keeps "Removed entries" open, the phone's.
pub const REMOVED_OPEN: &str = "tallyRemovedOpen";

/// How many entries show before "Show more".
pub const PAGE: usize = 30;

/// How wide a bar of a chart is at least, with the gap after it.
const SLOT: f32 = 10.0;

/// How tall the bars of a chart are at most.
const CHART_HEIGHT: f32 = 120.0;

/// Whose entries the table and the charts show in a Shared Area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Whose {
    Mine,
    Everyone,
    /// The person with this Profile id.
    Person(String),
}

impl Whose {
    pub fn filter(&self) -> PersonFilter {
        match self {
            Whose::Mine => PersonFilter {
                me: true,
                people: BTreeSet::new(),
            },
            Whose::Everyone => PersonFilter::default(),
            Whose::Person(id) => PersonFilter::person(id),
        }
    }
}

/// How the Tally part of the selected Note is looked at: whose entries,
/// the length of a bar and how many entries show. It starts again for
/// every Note opened.
#[derive(Debug, Clone, PartialEq)]
pub struct TallyLook {
    /// The Note it is for.
    pub of: Option<String>,
    /// `None` until picked: the user's own in a Shared Area, else
    /// everyone's, which outside one are the user's own too.
    pub whose: Option<Whose>,
    pub period: TallyPeriod,
    pub count: usize,
}

impl TallyLook {
    pub fn new(of: Option<String>) -> TallyLook {
        TallyLook {
            of,
            whose: None,
            period: TallyPeriod::Day,
            count: PAGE,
        }
    }
}

/// An entry with who wrote it, `None` for the user's own (ADR 0017),
/// and whether this Device may change it.
#[derive(Debug, Clone)]
pub struct EntryShown {
    pub entry: TallyEntry,
    pub author: Option<Profile>,
    pub changes: bool,
}

/// The Tally of the selected Note as its editor shows it.
#[derive(Debug, Clone)]
pub struct TallyShown {
    pub tally: Tally,
    /// Whether the Note lies in a Shared Area.
    pub shared: bool,
    /// Whose entries show.
    pub whose: Whose,
    pub entries: Vec<EntryShown>,
    /// The other people who wrote entries, for "Show entries".
    pub authors: Vec<Profile>,
    /// The chart of each field shown, by field id.
    pub charts: BTreeMap<String, Vec<TallyBucket>>,
    /// The removed entries, the last removed first.
    pub removed: Vec<EntryShown>,
    pub removed_open: bool,
    /// The length of a bar the charts were read with.
    pub period: TallyPeriod,
    /// Whether this Device may add entries and fields.
    pub adds: bool,
    pub defines: bool,
    /// The fields whose Note this Device may change, by id.
    pub changeable: HashSet<String>,
    pub system: UnitSystem,
}

/// Reads the Tally of the Note `note_id` as `look` asks, or `None` while
/// it is no Tally.
pub fn read(
    store: &Store,
    note_id: &str,
    look: &TallyLook,
    system: UnitSystem,
    wall: &WallClock,
) -> oldbrainfix_core::Result<Option<TallyShown>> {
    let tallies = Tallies::new(store);
    let Some(tally) = tallies.by_note(note_id)? else {
        return Ok(None);
    };
    let shared = match store.area_of(note_id)? {
        Some(area) => store.is_shared(&area)?,
        None => false,
    };
    let shown = |entry: TallyEntry| -> oldbrainfix_core::Result<EntryShown> {
        Ok(EntryShown {
            author: store.author_of(&entry.id)?,
            changes: store.may_change(&entry.id)?,
            entry,
        })
    };
    let mut authors: Vec<Profile> = Vec::new();
    if shared {
        for entry in tallies.entries(note_id, &PersonFilter::default())? {
            if let Some(author) = store.author_of(&entry.id)?
                && !authors.iter().any(|a| a.id == author.id)
            {
                authors.push(author);
            }
        }
    }
    let mut whose = match &look.whose {
        Some(whose) => whose.clone(),
        None if shared => Whose::Mine,
        None => Whose::Everyone,
    };
    // The person picked may have no entries any more.
    if let Whose::Person(id) = &whose
        && !authors.iter().any(|a| &a.id == id)
    {
        whose = Whose::Mine;
    }
    let filter = whose.filter();
    let mut entries = Vec::new();
    for entry in tallies.entries(note_id, &filter)? {
        entries.push(shown(entry)?);
    }
    let mut charts = BTreeMap::new();
    for field in tally.shown() {
        let bars = tallies.chart(note_id, &field.id, look.period, wall, &filter)?;
        charts.insert(field.id.clone(), bars);
    }
    let mut removed = Vec::new();
    for entry in tallies.removed_entries(note_id)? {
        removed.push(shown(entry)?);
    }
    removed.sort_by_key(|s| std::cmp::Reverse(s.entry.trashed.unwrap_or(s.entry.at)));
    let mut changeable = HashSet::new();
    for field in &tally.fields {
        if store.may_change(&field.note_id)? {
            changeable.insert(field.id.clone());
        }
    }
    Ok(Some(TallyShown {
        shared,
        whose,
        entries,
        authors,
        charts,
        removed,
        removed_open: store.local_setting(REMOVED_OPEN)?.as_deref() == Some("true"),
        period: look.period,
        adds: store.may_write_in(note_id)?,
        defines: store.may_change(note_id)?,
        changeable,
        system,
        tally,
    }))
}

/// What the user asked of the Tally part of a Note's editor.
#[derive(Debug, Clone, PartialEq)]
pub enum TallyAction {
    /// Opens the field page for the first field of the Note with this
    /// id, which makes it a Tally.
    Make(String),
    /// Opens the entry page for a new entry.
    AddEntry,
    /// Opens the entry page of the entry with this id.
    EditEntry(String),
    /// Asks whether the entry with this id goes.
    AskDelete(String),
    Restore(String),
    /// Opens the field page of the field with this id, or of a new one.
    OpenField(Option<String>),
    Whose(Whose),
    Period(TallyPeriod),
    /// Shows the next entries.
    More,
    /// Keeps whether "Removed entries" is open.
    RemovedOpen(bool),
}

/// The decimal sign of the app's language.
pub fn decimal_sign(t: &L10n) -> &'static str {
    match t.locale() {
        "de" => ",",
        _ => ".",
    }
}

/// `base`, stored in the base unit of `unit`, as shown with its unit.
pub fn shown_value(base: f64, unit: &str, system: UnitSystem, decimal: &str) -> String {
    let number = tally_units::format_shown(base, unit, system, decimal);
    let shown = tally_units::shown_unit(unit, system);
    if shown.is_empty() {
        number
    } else {
        format!("{number} {shown}")
    }
}

/// `base` as typed into a field of `unit`: as shown, without the unit.
fn typed_value(base: Option<f64>, unit: &str, system: UnitSystem, decimal: &str) -> String {
    base.map(|b| tally_units::format_shown(b, unit, system, decimal))
        .unwrap_or_default()
}

/// The name of a kind of unit; "Another unit" for a typed one.
fn kind_name(t: &L10n, kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Mass => t.unit_kind_mass(),
        UnitKind::Distance => t.unit_kind_distance(),
        UnitKind::Speed => t.unit_kind_speed(),
        UnitKind::Temperature => t.unit_kind_temperature(),
        UnitKind::Angle => t.unit_kind_angle(),
        UnitKind::Time => t.unit_kind_time(),
        UnitKind::Typed => t.tally_unit_typed(),
    }
}

/// What `entry` holds for the fields shown, one after the other.
fn values_line(t: &L10n, tally: &Tally, entry: &TallyEntry, system: UnitSystem) -> String {
    let decimal = decimal_sign(t);
    tally
        .shown()
        .into_iter()
        .filter_map(|field| {
            let value = entry.values.get(&field.id)?;
            Some(format!(
                "{} {}",
                field.name,
                shown_value(*value, &field.unit, system, decimal)
            ))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The Tally part of a Note's editor: nothing to draw while the Note is
/// no Tally.
pub fn section(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (shown, count): (&TallyShown, usize),
    actions: &mut Vec<TallyAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let tally = &shown.tally;
    let system = shown.system;
    let decimal = decimal_sign(t);
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.tally_heading()).strong().color(text));
        for field in &tally.fields {
            let unit = tally_units::shown_unit(&field.unit, system);
            let mut parts = Vec::new();
            if !unit.is_empty() {
                parts.push(unit);
            }
            if let Some(target) = field.target {
                let value = shown_value(target, &field.unit, system, decimal);
                parts.push(t.tally_target_value(&value));
            }
            if field.hidden {
                parts.push(t.tally_field_hidden().to_string());
            }
            let body = ui.vertical(|ui| {
                ui.label(RichText::new(&field.name).color(text));
                ui.label(RichText::new(parts.join(" · ")).small().color(weak));
            });
            if shown.changeable.contains(&field.id) {
                let open = ui
                    .interact(
                        body.response.rect,
                        Id::new(("tally field", &field.id)),
                        Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &field.name));
                if open.clicked() {
                    actions.push(TallyAction::OpenField(Some(field.id.clone())));
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            if shown.adds
                && ui
                    .button(RichText::new(t.add_tally_entry()).color(text))
                    .clicked()
            {
                actions.push(TallyAction::AddEntry);
            }
            if shown.defines
                && ui
                    .button(RichText::new(t.tally_add_field()).color(text))
                    .clicked()
            {
                actions.push(TallyAction::OpenField(None));
            }
        });
    });
    if shown.shared {
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.tally_show_entries()).strong().color(text));
            let mut options = vec![
                (Whose::Mine, t.tally_entries_mine().to_string()),
                (Whose::Everyone, t.tally_entries_everyone().to_string()),
            ];
            for person in &shown.authors {
                let name = profile_view::display_name(t, &person.name).to_string();
                options.push((Whose::Person(person.id.clone()), name));
            }
            for (whose, label) in options {
                let on = shown.whose == whose;
                if ui.radio(on, RichText::new(label).color(text)).clicked() && !on {
                    actions.push(TallyAction::Whose(whose));
                }
            }
        });
    }
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.tally_period()).strong().color(text));
        let mut period = shown.period;
        let options = [
            (TallyPeriod::Day, t.tally_period_day().to_string()),
            (TallyPeriod::Week, t.tally_period_week().to_string()),
            (TallyPeriod::Month, t.tally_period_month().to_string()),
            (TallyPeriod::Year, t.tally_period_year().to_string()),
        ];
        if radio_column(ui, &mut period, options) {
            actions.push(TallyAction::Period(period));
        }
    });
    for field in tally.shown() {
        embossed(ui, tones, false, None, |ui| {
            let unit = tally_units::shown_unit(&field.unit, system);
            let heading = if unit.is_empty() {
                field.name.clone()
            } else {
                format!("{} ({unit})", field.name)
            };
            ui.label(RichText::new(heading).strong().color(text));
            let bars = shown.charts.get(&field.id).map_or(&[][..], Vec::as_slice);
            chart(ui, t, field, bars, (shown.period, system));
        });
    }
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.tally_entries()).strong().color(text));
        if shown.entries.is_empty() {
            ui.label(RichText::new(t.tally_no_entries()).color(weak));
        }
        for entry in shown.entries.iter().take(count) {
            entry_row(ui, t, (tally, system), entry, false, actions);
        }
        if shown.entries.len() > count
            && ui
                .button(RichText::new(t.tally_more_entries()).color(text))
                .clicked()
        {
            actions.push(TallyAction::More);
        }
    });
    if shown.removed.is_empty() {
        return;
    }
    embossed(ui, tones, false, None, |ui| {
        let open = shown.removed_open;
        let fold =
            egui::CollapsingHeader::new(RichText::new(t.tally_removed_entries()).color(text))
                .id_salt("tally removed")
                .open(Some(open))
                .show(ui, |ui| {
                    for entry in &shown.removed {
                        entry_row(ui, t, (tally, system), entry, true, actions);
                    }
                });
        if fold.header_response.clicked() {
            actions.push(TallyAction::RemovedOpen(!open));
        }
    });
}

/// One entry: its date, its author when it is another person's, its
/// values and its line. A click opens it for who may change it, a
/// right-click offers Edit and Delete; a removed one has Restore below.
fn entry_row(
    ui: &mut Ui,
    t: &L10n,
    (tally, system): (&Tally, UnitSystem),
    shown: &EntryShown,
    removed: bool,
    actions: &mut Vec<TallyAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let entry = &shown.entry;
    let date = when(t, entry.at);
    let mut lines = Vec::new();
    if let Some(author) = &shown.author {
        lines.push(profile_view::display_name(t, &author.name).to_string());
    }
    let values = values_line(t, tally, entry, system);
    if !values.is_empty() {
        lines.push(values);
    }
    if !entry.line.is_empty() {
        lines.push(entry.line.clone());
    }
    let body = ui.vertical(|ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(&date).color(text));
        for line in &lines {
            ui.label(RichText::new(line).small().color(weak));
        }
    });
    let label = std::iter::once(date)
        .chain(lines)
        .collect::<Vec<_>>()
        .join(" · ");
    if removed {
        if shown.changes
            && ui
                .button(RichText::new(t.restore()).color(text))
                .on_hover_text(&label)
                .clicked()
        {
            actions.push(TallyAction::Restore(entry.id.clone()));
        }
        return;
    }
    if !shown.changes {
        return;
    }
    let open = ui
        .interact(
            body.response.rect,
            Id::new(("tally entry", &entry.id)),
            Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
    if open.clicked() {
        actions.push(TallyAction::EditEntry(entry.id.clone()));
    }
    open.context_menu(|ui| {
        if ui.button(RichText::new(t.edit()).color(text)).clicked() {
            actions.push(TallyAction::EditEntry(entry.id.clone()));
        }
        if ui.button(RichText::new(t.delete()).color(text)).clicked() {
            actions.push(TallyAction::AskDelete(entry.id.clone()));
        }
    });
}

/// A day as the charts name it: a date for a day or a week, a month and
/// its year, or a year.
fn bar_date(t: &L10n, day: Day, period: TallyPeriod) -> String {
    match period {
        TallyPeriod::Day | TallyPeriod::Week => match t.locale() {
            "de" => format!("{}.{}.{}", day.day, day.month, day.year),
            _ => format!("{}/{}/{}", day.month, day.day, day.year),
        },
        TallyPeriod::Month => format!("{} {}", month_short(t, day.month), day.year),
        TallyPeriod::Year => day.year.to_string(),
    }
}

/// One field's chart: a bar per day, week, month or year, the latest that
/// fit the width, never scrolled sideways; the target as a line; under
/// it the days the bars cover and the highest bar's value.
fn chart(
    ui: &mut Ui,
    t: &L10n,
    field: &TallyField,
    buckets: &[TallyBucket],
    (period, system): (TallyPeriod, UnitSystem),
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    if buckets.is_empty() {
        ui.label(RichText::new(t.tally_chart_empty()).color(weak));
        return;
    }
    let width = ui.available_width().max(SLOT);
    let fit = ((width / SLOT).floor() as usize).max(1);
    let shown = &buckets[buckets.len().saturating_sub(fit)..];
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, CHART_HEIGHT), Sense::hover());
    let label = t.tally_chart_label(&field.name, shown.len() as i64);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, &label));
    let values: Vec<f64> = shown.iter().filter_map(|b| b.value).collect();
    let highest = values.iter().copied().reduce(f64::max);
    if ui.is_rect_visible(rect) {
        let all = values.iter().copied().chain(field.target);
        let top = all.clone().fold(0.0, f64::max);
        let bottom = all.fold(0.0, f64::min);
        let span = top - bottom;
        let y = |value: f64| -> f32 {
            if span == 0.0 {
                rect.bottom()
            } else {
                rect.top() + rect.height() * ((top - value) / span) as f32
            }
        };
        let painter = ui.painter();
        let zero = y(0.0);
        painter.line_segment(
            [
                egui::pos2(rect.left(), zero),
                egui::pos2(rect.right(), zero),
            ],
            egui::Stroke::new(1.0, weak),
        );
        let slots = shown.len().max(7) as f32;
        let bar = rect.width() / slots;
        let fill = ui.visuals().selection.bg_fill;
        for (i, bucket) in shown.iter().enumerate() {
            let Some(value) = bucket.value else { continue };
            let left = rect.left() + i as f32 * bar;
            let (a, b) = (zero, y(value));
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(left + 1.0, a.min(b)),
                    egui::pos2(left + bar - 1.0, a.max(b)),
                ),
                0.0,
                fill,
            );
        }
        if let Some(target) = field.target {
            let at = y(target);
            painter.line_segment(
                [egui::pos2(rect.left(), at), egui::pos2(rect.right(), at)],
                egui::Stroke::new(2.0, ui.visuals().warn_fg_color),
            );
        }
    }
    if let (Some(first), Some(last)) = (shown.first(), shown.last()) {
        let range = t.tally_chart_range(
            &bar_date(t, first.start, period),
            &bar_date(t, last.end, period),
        );
        ui.label(RichText::new(range).small().color(text));
    }
    if let Some(highest) = highest {
        let value = shown_value(highest, &field.unit, system, decimal_sign(t));
        ui.label(
            RichText::new(t.tally_chart_highest(&value))
                .small()
                .color(text),
        );
    }
}

/// `at`, moved to the day `day` on `wall` at the same time of day.
pub fn moved_to(at: DateTime<Utc>, day: Day, wall: &WallClock) -> DateTime<Utc> {
    let from = Day::of(wall(at));
    let moved = at + Duration::days(from.days_until(day));
    // Across a change of the clocks the time of day is off by the change.
    let mut drift = wall(at).time() - wall(moved).time();
    if drift > Duration::hours(12) {
        drift -= Duration::days(1);
    } else if drift < Duration::hours(-12) {
        drift += Duration::days(1);
    }
    moved + drift
}

/// What the user asked of the field page or the entry page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormIntent {
    Save,
    Close,
    /// Hides the field, or shows it again.
    Hide(bool),
    /// Deletes the entry, after asking.
    Delete,
}

/// The field page or the entry page in the right pane.
#[derive(Debug, Clone)]
pub enum TallyForm {
    Field(FieldForm),
    Entry(EntryForm),
}

impl TallyForm {
    /// The Note whose Tally it changes.
    pub fn note_id(&self) -> &str {
        match self {
            TallyForm::Field(form) => &form.note_id,
            TallyForm::Entry(form) => &form.note_id,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<FormIntent> {
        match self {
            TallyForm::Field(form) => form.show(ui, t, tones),
            TallyForm::Entry(form) => form.show(ui, t, tones),
        }
    }
}

/// The heading of a page and its bar: Save, Cancel and, with `delete`,
/// Delete, last.
fn page_bar(ui: &mut Ui, t: &L10n, heading: &str, delete: bool) -> Option<FormIntent> {
    let text = ui.visuals().text_color();
    let mut intent = None;
    ui.label(RichText::new(heading).size(18.0).strong().color(text));
    ui.horizontal_wrapped(|ui| {
        if ui.button(RichText::new(t.save()).color(text)).clicked() {
            intent = Some(FormIntent::Save);
        }
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            intent = Some(FormIntent::Close);
        }
        if delete
            && ui
                .button(RichText::new(t.delete()).color(ui.visuals().error_fg_color))
                .clicked()
        {
            intent = Some(FormIntent::Delete);
        }
    });
    intent
}

/// An error under its field, in red.
fn field_error(ui: &mut Ui, error: &str) {
    ui.label(RichText::new(error).color(ui.visuals().error_fg_color));
}

/// One field of a Tally: its name; its unit, a known one shown in this
/// Device's system with its kind under it, or one as typed, only of its
/// kind once entries hold values for it; its target, optional; and how a
/// day with several entries counts, which follows the kind of unit until
/// it is picked. A field that exists has "Hide field", or "Show field
/// again", at the bottom. Leaving without Save changes nothing.
#[derive(Debug, Clone)]
pub struct FieldForm {
    pub note_id: String,
    /// The field shown; `None` for a new one.
    field: Option<TallyField>,
    pub name: String,
    /// The known unit picked, or `None` for one as typed.
    pub unit: Option<&'static str>,
    pub typed: String,
    pub target: String,
    target_as_opened: String,
    pub rule: TallyRule,
    rule_picked: bool,
    system: UnitSystem,
    name_missing: bool,
    target_invalid: bool,
    unit_invalid: bool,
    /// The text field to take the keyboard on the next frame.
    focus: Option<&'static str>,
}

const NAME_FIELD: &str = "tally field name";
const TYPED_FIELD: &str = "tally unit text";
const TARGET_FIELD: &str = "tally field target";
const LINE_FIELD: &str = "tally entry line";
const DAY_FIELD: &str = "tally entry day";

impl FieldForm {
    /// The page of `field` of the Tally of `note_id`, or of a new one.
    pub fn new(
        note_id: &str,
        field: Option<TallyField>,
        system: UnitSystem,
        t: &L10n,
    ) -> FieldForm {
        let decimal = decimal_sign(t);
        let unit = field
            .as_ref()
            .and_then(|f| tally_units::by_code(&f.unit))
            .map(|k| k.code);
        let typed = match (&field, unit) {
            (Some(f), None) => f.unit.clone(),
            _ => String::new(),
        };
        let target = field
            .as_ref()
            .map(|f| typed_value(f.target, &f.unit, system, decimal))
            .unwrap_or_default();
        FieldForm {
            note_id: note_id.to_string(),
            name: field.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
            unit,
            typed,
            target_as_opened: target.clone(),
            target,
            rule: field.as_ref().map_or(TallyRule::Sum, |f| f.rule),
            rule_picked: false,
            system,
            name_missing: false,
            target_invalid: false,
            unit_invalid: false,
            focus: field.is_none().then_some(NAME_FIELD),
            field,
        }
    }

    fn chosen_unit(&self) -> String {
        match self.unit {
            Some(code) => code.to_string(),
            None => self.typed.trim().to_string(),
        }
    }

    /// The rule follows the kind of unit until it is picked, for a new
    /// field.
    fn unit_changed(&mut self) {
        if !self.rule_picked && self.field.is_none() {
            self.rule = tally_units::default_rule(tally_units::kind_of(&self.chosen_unit()));
        }
    }

    /// Writes the field when nothing needs fixing; whether it did.
    pub fn save(&mut self, store: &Store) -> oldbrainfix_core::Result<bool> {
        let name = self.name.trim().to_string();
        let unit = self.chosen_unit();
        let target_text = self.target.trim().to_string();
        let target = if target_text.is_empty() {
            None
        } else {
            tally_units::parse_shown(&target_text, &unit, self.system)
        };
        self.name_missing = name.is_empty();
        self.target_invalid = !target_text.is_empty() && target.is_none();
        self.unit_invalid = self
            .field
            .as_ref()
            .is_some_and(|f| f.has_entries && tally_units::kind_of(&unit) != f.kind());
        // The field to fix takes the keyboard.
        self.focus = if self.name_missing {
            Some(NAME_FIELD)
        } else if self.unit_invalid {
            Some(TYPED_FIELD)
        } else if self.target_invalid {
            Some(TARGET_FIELD)
        } else {
            None
        };
        if self.focus.is_some() {
            return Ok(false);
        }
        let tallies = Tallies::new(store);
        let note = &self.note_id;
        match &self.field {
            None => {
                tallies.add_field(note, &name, &unit, target, Some(self.rule))?;
            }
            Some(field) => {
                if name != field.name {
                    tallies.rename_field(note, &field.id, &name)?;
                }
                let unit_changed = unit != field.unit;
                if unit_changed {
                    tallies.set_unit(note, &field.id, &unit)?;
                }
                // Read again in the unit now shown next to it.
                if unit_changed || target_text != self.target_as_opened {
                    tallies.set_target(note, &field.id, target)?;
                }
                if self.rule != field.rule {
                    tallies.set_rule(note, &field.id, self.rule)?;
                }
            }
        }
        Ok(true)
    }

    /// Hides the field shown, or shows it again.
    pub fn set_hidden(&self, store: &Store, hidden: bool) -> oldbrainfix_core::Result<()> {
        let Some(field) = &self.field else {
            return Ok(());
        };
        let tallies = Tallies::new(store);
        if hidden {
            tallies.hide_field(&self.note_id, &field.id)
        } else {
            tallies.restore_field(&self.note_id, &field.id)
        }
    }

    fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<FormIntent> {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let focus = self.focus.take();
        let heading = if self.field.is_none() {
            t.tally_new_field()
        } else {
            t.tally_field()
        };
        let mut intent = None;
        // Once entries hold values, only units of the field's kind are
        // left.
        let fixed = self
            .field
            .as_ref()
            .filter(|f| f.has_entries)
            .map(TallyField::kind);
        egui::ScrollArea::vertical()
            .id_salt("tally field")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                intent = page_bar(ui, t, heading, false);
                embossed(ui, tones, false, None, |ui| {
                    let label = ui.label(RichText::new(t.tally_field_name()).strong().color(text));
                    let field = ui
                        .add(
                            TextEdit::singleline(&mut self.name)
                                .id(Id::new(NAME_FIELD))
                                .hint_text(t.tally_field_name_hint())
                                .text_color(text)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(label.id);
                    if focus == Some(NAME_FIELD) {
                        field.request_focus();
                    }
                    if self.name_missing {
                        field_error(ui, t.tally_field_name_missing());
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.label(RichText::new(t.tally_unit()).strong().color(text));
                    if fixed.is_some() {
                        ui.label(RichText::new(t.tally_unit_fixed()).small().color(weak));
                    }
                    let mut changed = false;
                    for known in KNOWN.iter().filter(|k| fixed.is_none_or(|f| f == k.kind)) {
                        let label = tally_units::shown_unit(known.code, self.system);
                        changed |= ui
                            .radio_value(
                                &mut self.unit,
                                Some(known.code),
                                RichText::new(label).color(text),
                            )
                            .clicked();
                        ui.horizontal(|ui| {
                            ui.add_space(24.0);
                            ui.label(RichText::new(kind_name(t, known.kind)).small().color(weak));
                        });
                    }
                    let typed = if fixed.is_none_or(|f| f == UnitKind::Typed) {
                        let typed = ui.radio_value(
                            &mut self.unit,
                            None,
                            RichText::new(t.tally_unit_typed()).color(text),
                        );
                        changed |= typed.clicked();
                        Some(typed)
                    } else {
                        None
                    };
                    if self.unit.is_none()
                        && let Some(typed) = typed
                    {
                        let field = ui
                            .add(
                                TextEdit::singleline(&mut self.typed)
                                    .id(Id::new(TYPED_FIELD))
                                    .hint_text(t.tally_unit_typed_hint())
                                    .text_color(text)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(typed.id);
                        if focus == Some(TYPED_FIELD) {
                            field.request_focus();
                        }
                        // A typed known code is that unit; its kind may
                        // change the rule.
                        changed |= field.changed();
                        if self.unit_invalid {
                            field_error(ui, t.tally_unit_fixed());
                        }
                    }
                    if changed {
                        self.unit_changed();
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    let label = ui.label(RichText::new(t.tally_target()).strong().color(text));
                    ui.horizontal(|ui| {
                        let unit = tally_units::shown_unit(&self.chosen_unit(), self.system);
                        let field = ui
                            .add(
                                TextEdit::singleline(&mut self.target)
                                    .id(Id::new(TARGET_FIELD))
                                    .text_color(text)
                                    .desired_width(120.0),
                            )
                            .labelled_by(label.id);
                        if focus == Some(TARGET_FIELD) {
                            field.request_focus();
                        }
                        ui.label(RichText::new(unit).color(text));
                    });
                    if self.target_invalid {
                        field_error(ui, t.tally_not_a_number());
                    }
                    ui.label(RichText::new(t.tally_target_hint()).small().color(weak));
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.tally_rule()).strong().color(text));
                    let options = [
                        (TallyRule::Sum, t.tally_rule_sum().to_string()),
                        (TallyRule::Max, t.tally_rule_max().to_string()),
                    ];
                    if radio_column(ui, &mut self.rule, options) {
                        self.rule_picked = true;
                    }
                });
                // A secondary state change: a row at the bottom.
                if let Some(field) = &self.field {
                    embossed(ui, tones, false, None, |ui| {
                        let label = if field.hidden {
                            t.tally_show_field()
                        } else {
                            t.tally_hide_field()
                        };
                        if ui.button(RichText::new(label).color(text)).clicked() {
                            intent = Some(FormIntent::Hide(!field.hidden));
                        }
                        ui.label(
                            RichText::new(t.tally_hide_field_detail())
                                .small()
                                .color(weak),
                        );
                    });
                }
            });
        intent
    }
}

/// One entry of a Tally: a value per field shown, a new entry's
/// prefilled with the field's target, so that saving the targets is one
/// click; the day it counts for, today for a new one, at the same time
/// of day; and one line of text. A value left empty is none. An entry
/// that exists has Delete, which asks once; it can be restored.
#[derive(Debug, Clone)]
pub struct EntryForm {
    pub note_id: String,
    /// The entry shown; `None` for a new one.
    pub entry: Option<TallyEntry>,
    /// Each field shown, with its value as typed and as the page opened.
    pub values: Vec<(TallyField, String, String)>,
    pub line: String,
    pub day: Day,
    day_as_opened: Day,
    pub day_text: String,
    /// Whether this Device may delete it.
    changes: bool,
    system: UnitSystem,
    invalid: HashSet<String>,
    empty: bool,
    focus: Option<String>,
}

impl EntryForm {
    /// The page of `entry` of `tally`, or of a new one, on `today`.
    pub fn new(
        tally: &Tally,
        entry: Option<(TallyEntry, bool)>,
        (system, today): (UnitSystem, Day),
        wall: &WallClock,
        t: &L10n,
    ) -> EntryForm {
        let decimal = decimal_sign(t);
        let values = tally
            .shown()
            .into_iter()
            .map(|field| {
                let base = match &entry {
                    None => field.target,
                    Some((entry, _)) => entry.values.get(&field.id).copied(),
                };
                let text = typed_value(base, &field.unit, system, decimal);
                (field.clone(), text.clone(), text)
            })
            .collect::<Vec<_>>();
        let day = entry
            .as_ref()
            .map_or(today, |(entry, _)| Day::of(wall(entry.at)));
        EntryForm {
            note_id: tally.note_id.clone(),
            line: entry
                .as_ref()
                .map(|(e, _)| e.line.clone())
                .unwrap_or_default(),
            changes: entry.as_ref().is_some_and(|(_, changes)| *changes),
            entry: entry.map(|(e, _)| e),
            focus: values.first().map(|(f, ..)| f.id.clone()),
            values,
            day,
            day_as_opened: day,
            day_text: day.to_string(),
            system,
            invalid: HashSet::new(),
            empty: false,
        }
    }

    /// Writes the entry when nothing needs fixing; whether it did.
    pub fn save(&mut self, store: &Store, wall: &WallClock) -> oldbrainfix_core::Result<bool> {
        let mut parsed: BTreeMap<String, Option<f64>> = BTreeMap::new();
        let mut invalid = HashSet::new();
        for (field, typed, _) in &self.values {
            let typed = typed.trim();
            if typed.is_empty() {
                parsed.insert(field.id.clone(), None);
                continue;
            }
            let value = tally_units::parse_shown(typed, &field.unit, self.system);
            if value.is_none() {
                invalid.insert(field.id.clone());
            }
            parsed.insert(field.id.clone(), value);
        }
        let line = self.line.trim().to_string();
        self.empty = self.entry.is_none()
            && invalid.is_empty()
            && line.is_empty()
            && parsed.values().all(Option::is_none);
        let day_wrong = Day::parse(self.day_text.trim()) != Some(self.day);
        // The field to fix takes the keyboard.
        self.focus = self
            .values
            .iter()
            .find(|(f, ..)| invalid.contains(&f.id))
            .map(|(f, ..)| f.id.clone())
            .or_else(|| day_wrong.then(|| DAY_FIELD.to_string()))
            .or_else(|| self.empty.then(|| LINE_FIELD.to_string()));
        self.invalid = invalid;
        if self.focus.is_some() {
            return Ok(false);
        }
        let tallies = Tallies::new(store);
        let base = self.entry.as_ref().map_or_else(|| store.now(), |e| e.at);
        let at = (self.day != self.day_as_opened).then(|| moved_to(base, self.day, wall));
        match &self.entry {
            None => {
                let values = parsed
                    .into_iter()
                    .filter_map(|(id, value)| Some((id, value?)))
                    .collect();
                tallies.add(&self.note_id, &values, &line, at)?;
            }
            Some(entry) => {
                // Only those typed anew: one shown rounded stays as it is.
                let retyped = self
                    .values
                    .iter()
                    .filter(|(_, typed, opened)| typed.trim() != opened)
                    .filter_map(|(field, ..)| Some((field.id.clone(), *parsed.get(&field.id)?)))
                    .collect();
                tallies.edit_entry(&entry.id, &retyped, Some(&line), at)?;
            }
        }
        Ok(true)
    }

    fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<FormIntent> {
        let text = ui.visuals().text_color();
        let focus = self.focus.take();
        let heading = if self.entry.is_none() {
            t.tally_new_entry()
        } else {
            t.tally_entry()
        };
        let mut intent = None;
        let system = self.system;
        egui::ScrollArea::vertical()
            .id_salt("tally entry")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                intent = page_bar(ui, t, heading, self.entry.is_some() && self.changes);
                for (field, typed, _) in &mut self.values {
                    embossed(ui, tones, false, None, |ui| {
                        let label = ui.label(RichText::new(&field.name).strong().color(text));
                        ui.horizontal(|ui| {
                            let edit = ui
                                .add(
                                    TextEdit::singleline(typed)
                                        .id(Id::new(("tally value", &field.id)))
                                        .text_color(text)
                                        .desired_width(120.0),
                                )
                                .labelled_by(label.id);
                            if focus.as_deref() == Some(field.id.as_str()) {
                                edit.request_focus();
                            }
                            let unit = tally_units::shown_unit(&field.unit, system);
                            ui.label(RichText::new(unit).color(text));
                        });
                        if self.invalid.contains(&field.id) {
                            field_error(ui, t.tally_not_a_number());
                        }
                    });
                }
                embossed(ui, tones, false, None, |ui| {
                    let label = ui.label(RichText::new(t.tally_entry_date()).strong().color(text));
                    self.day_editor(ui, t, (label.id, focus.as_deref() == Some(DAY_FIELD)));
                });
                embossed(ui, tones, false, None, |ui| {
                    let label = ui.label(RichText::new(t.tally_entry_line()).strong().color(text));
                    let field = ui
                        .add(
                            TextEdit::singleline(&mut self.line)
                                .id(Id::new(LINE_FIELD))
                                .text_color(text)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(label.id);
                    if focus.as_deref() == Some(LINE_FIELD) {
                        field.request_focus();
                    }
                    if self.empty {
                        field_error(ui, t.tally_entry_empty());
                    }
                });
            });
        intent
    }

    /// The day the entry counts for: a day back or on, or typed.
    fn day_editor(&mut self, ui: &mut Ui, t: &L10n, (label, focus): (Id, bool)) {
        let text = ui.visuals().text_color();
        ui.horizontal_wrapped(|ui| {
            let mut moved = false;
            if icon_button(ui, icons::PREVIOUS, t.day_before(), text).clicked() {
                self.day = self.day.add_days(-1);
                moved = true;
            }
            ui.label(RichText::new(day_text(t, self.day)).strong().color(text));
            if icon_button(ui, icons::NEXT, t.day_after(), text).clicked() {
                self.day = self.day.add_days(1);
                moved = true;
            }
            if moved {
                self.day_text = self.day.to_string();
            }
            let field = ui
                .add(
                    TextEdit::singleline(&mut self.day_text)
                        .id(Id::new(DAY_FIELD))
                        .hint_text(t.date_hint())
                        .text_color(text)
                        .desired_width(110.0),
                )
                .labelled_by(label);
            if focus {
                field.request_focus();
            }
            if field.changed()
                && let Some(day) = Day::parse(self.day_text.trim())
            {
                self.day = day;
            }
        });
        if Day::parse(self.day_text.trim()) != Some(self.day) {
            field_error(ui, t.date_invalid());
        }
    }
}
