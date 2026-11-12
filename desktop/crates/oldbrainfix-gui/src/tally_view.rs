//! Tallies on the desktop's screen (#103, ADR 0028), as the phone's
//! `tally_view.dart`, `tally_field_screen.dart` and
//! `tally_entry_screen.dart` draw them: the setup in the Note dialog, the
//! Tally on its Note's detailed card, and the field page and the entry
//! page, each in a dialog over the window as the Routine editor is, Save
//! at their top.
//!
//! The setup (#190) shows the Tally's bar length and whether it counts in
//! sets as badges (#264), then lists the fields, each its name and its
//! settings as badges, opening its field page for who may change the
//! Note, and "Add field". The detailed card shows "Add entry"; in a Shared
//! Area the choice of whose entries show, the user's own by default; one
//! chart per field shown, a bar per the Tally's bar length; the
//! entries, newest first, the older folded under the latest five until a
//! click shows them, kept per Note on this Device, each clicked to edit
//! it and right-clicked for a menu; and a folded "Removed entries" with
//! Restore, closed until opened, remembered on this Device.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{DateTime, Duration, Utc};
use egui::{Id, RichText, Sense, TextEdit, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::tally_units::{self, KnownUnit};
use oldbrainfix_core::{
    Day, Error, Occurrence, PersonFilter, Profile, Sessions, Store, Tallies, Tally, TallyBucket,
    TallyEntry, TallyField, TallyPeriod, TallyRule, UnitKind, UnitSystem, WallClock,
};

use crate::app::{chip_row, embossed, icon_button, when};
use crate::capture_choices::TallyFieldChoice;
use crate::icons;
use crate::l10n::L10n;
use crate::leave_saves::Leaving;
use crate::profile_view;
use crate::schedule_text::{day_text, month_short};
use crate::setting_badges::BadgeValue::{self, Icon, Text};
use crate::setting_badges::{badge, badge_row, switch_icon};
use crate::tally_unit_match::{self, UnitNames};
use crate::theme::{self, Tones};

/// The local setting that keeps "Removed entries" open, the phone's.
pub const REMOVED_OPEN: &str = "tallyRemovedOpen";

/// How many entries a card shows before the older ones fold (#190).
pub const SHOWN_ENTRIES: usize = 5;

/// The setting of this Device that keeps all entries of the Tally of
/// `note` shown on its card (#190), the phone's; nothing kept means
/// folded after [`SHOWN_ENTRIES`].
pub fn entries_shown_setting(note: &str) -> String {
    format!("tallyEntriesShown:{note}")
}

/// The name of `period` as a bar's length.
pub fn period_label(t: &L10n, period: TallyPeriod) -> &'static str {
    match period {
        TallyPeriod::Day => t.tally_period_day(),
        TallyPeriod::Week => t.tally_period_week(),
        TallyPeriod::Month => t.tally_period_month(),
        TallyPeriod::Year => t.tally_period_year(),
    }
}

/// The person filter the Tally of `note_id` counts by unless one is
/// picked: the user's own entries in a Shared Area (ADR 0028), everyone's,
/// which are the user's own too, elsewhere. The compact view's totals and
/// the detailed view count alike (#190).
pub fn default_filter(store: &Store, note_id: &str) -> oldbrainfix_core::Result<PersonFilter> {
    let shared = match store.area_of(note_id)? {
        Some(area) => store.is_shared(&area)?,
        None => false,
    };
    Ok(if shared {
        Whose::Mine.filter()
    } else {
        Whose::Everyone.filter()
    })
}

/// How wide a bar of a chart is, with the gap after it (#264).
const SLOT: f32 = 12.0;

/// How wide the gap after a bar is.
const GAP: f32 = 4.0;

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

/// An entry with who wrote it, `None` for the user's own (ADR 0017),
/// and whether this Device may change it.
#[derive(Debug, Clone)]
pub struct EntryShown {
    pub entry: TallyEntry,
    pub author: Option<Profile>,
    pub changes: bool,
}

/// A Tally as its setup and its detailed card show it.
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
    /// Whether the older entries show, kept per Note.
    pub all_shown: bool,
    /// Whether this Device may add entries and fields.
    pub adds: bool,
    pub defines: bool,
    /// The fields whose Note this Device may change, by id.
    pub changeable: HashSet<String>,
    pub system: UnitSystem,
}

/// Reads the Tally of the Note `note_id`, with the entries of `whose`
/// when picked, or `None` while it is no Tally.
pub fn read(
    store: &Store,
    note_id: &str,
    whose: Option<&Whose>,
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
    let mut whose = match whose {
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
    for field in tally.counting() {
        let bars = tallies.chart(note_id, &field.id, None, wall, &filter)?;
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
        all_shown: store
            .local_setting(&entries_shown_setting(note_id))?
            .as_deref()
            == Some("true"),
        adds: store.may_write_in(note_id)?,
        defines: store.may_change(note_id)?,
        changeable,
        system,
        tally,
    }))
}

/// What the user asked of a Tally's setup or of its detailed card.
#[derive(Debug, Clone, PartialEq)]
pub enum TallyAction {
    /// Opens the field page for the first field of the Note with this
    /// id, which makes it a Tally.
    Make(String),
    /// Opens the entry page for a new entry.
    AddEntry,
    /// Opens the entry page of the entry with this id.
    EditEntry(String),
    /// Removes the entry with this id, without asking; Ctrl+Z and
    /// "Removed entries" bring it back (#268).
    Delete(String),
    Restore(String),
    /// Opens the field page of the field with this id, or of a new one.
    OpenField(Option<String>),
    Whose(Whose),
    /// Opens the small editor of one of the Tally's own settings (#264).
    Badge(TallyBadge),
    /// Sets the length of a bar of the Tally's charts (#264).
    Bar(TallyPeriod),
    /// Switches whether the Tally counts in sets (#264).
    Sets(bool),
    /// Keeps whether the older entries show.
    AllShown(bool),
    /// Keeps whether "Removed entries" is open.
    RemovedOpen(bool),
}

/// A Tally's own setting, a badge of its setup (#264).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyBadge {
    Bar,
    Sets,
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
pub(crate) fn typed_value(
    base: Option<f64>,
    unit: &str,
    system: UnitSystem,
    decimal: &str,
) -> String {
    base.map(|b| tally_units::format_shown(b, unit, system, decimal))
        .unwrap_or_default()
}

/// The rule `rule` as the value of a field's rule badge (#192): its icon
/// and its short words, the phone's `tallyRuleIcon`.
pub(crate) fn rule_value(t: &L10n, rule: TallyRule) -> BadgeValue<'static> {
    match rule {
        TallyRule::Sum => Icon(icons::TALLY_SUM, t.tally_rule_sum_short()),
        TallyRule::Max => Icon(icons::TALLY_MAX, t.tally_rule_max_short()),
    }
}

/// The name of a kind of unit; "Another unit" for a typed one.
pub(crate) fn kind_name(t: &L10n, kind: UnitKind) -> &'static str {
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

/// What `entry` holds for the fields shown (#264): the Check fields it
/// ticks, then its sets, each a line of its values one after the other,
/// numbered while there are several ("1. Weight 60 kg · Reps 10"), as the
/// phone's `_values`.
fn values_lines(t: &L10n, tally: &Tally, entry: &TallyEntry, system: UnitSystem) -> Vec<String> {
    let decimal = decimal_sign(t);
    let ticked: Vec<String> = tally
        .checks()
        .into_iter()
        .filter(|f| entry.values.contains_key(&f.id))
        .map(|f| format!("✓ {}", f.name))
        .collect();
    let numbers = tally.numbers();
    let sets: Vec<String> = tally
        .sets_of(entry)
        .iter()
        .map(|set| {
            numbers
                .iter()
                .filter_map(|field| {
                    let value = set.get(&field.id)?;
                    let shown = shown_value(*value, &field.unit, system, decimal);
                    Some(format!("{} {shown}", field.name))
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .filter(|line| !line.is_empty())
        .collect();
    let mut lines = Vec::new();
    if !ticked.is_empty() {
        lines.push(ticked.join(" · "));
    }
    if sets.len() == 1 {
        lines.extend(sets);
    } else {
        lines.extend(
            sets.into_iter()
                .enumerate()
                .map(|(i, line)| format!("{}. {line}", i + 1)),
        );
    }
    lines
}

/// The setup of a Tally in the Note dialog (#190): the Tally's own
/// settings as badges (#264), the length of a bar and whether it counts
/// in sets; then a row per field, its name and its settings as badges as
/// on the New Tally page (#161), how several sets or entries in a bar
/// count, its unit and its target, a Check field its kind and its
/// target, a field that counted sets as a multiplier did (#200) that it
/// counts the sets of older entries, "Hidden" while it is hidden; a
/// click on the name or a badge opens the field's
/// page for who may change the Note, and "Add field" the page of a new
/// one. The chart and the entries show on the card, not here.
pub fn setup(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    shown: &TallyShown,
    actions: &mut Vec<TallyAction>,
) {
    let text = ui.visuals().text_color();
    let tally = &shown.tally;
    let system = shown.system;
    let decimal = decimal_sign(t);
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.tally_fields()).strong().color(text));
        badge_row(ui, |ui| {
            let bar = Text(period_label(t, tally.bar));
            let look = (t, tones);
            let setting = t.tally_period();
            if badge(
                ui,
                look,
                (icons::TALLY_BAR, None),
                setting,
                Some(bar),
                shown.defines,
            )
            .clicked()
            {
                actions.push(TallyAction::Badge(TallyBadge::Bar));
            }
            let on = tally.counts_in_sets;
            let words = if on { t.setting_on() } else { t.setting_off() };
            let sets = Icon(switch_icon(on), words);
            let setting = t.tally_counts_in_sets();
            if badge(
                ui,
                look,
                (icons::TALLY_SETS, None),
                setting,
                Some(sets),
                shown.defines,
            )
            .clicked()
            {
                actions.push(TallyAction::Badge(TallyBadge::Sets));
            }
        });
        ui.add_space(8.0);
        for field in &tally.fields {
            let changes = shown.changeable.contains(&field.id);
            let name = RichText::new(&field.name).strong().color(text);
            if !changes {
                ui.label(name);
            } else if ui.add(egui::Button::new(name).frame(false)).clicked() {
                actions.push(TallyAction::OpenField(Some(field.id.clone())));
            }
            let unit = tally_units::shown_unit(&field.unit, system);
            let rule = rule_value(t, field.rule);
            let unit_value = if unit.is_empty() {
                t.tally_no_unit().to_string()
            } else {
                unit
            };
            let target_value = match field.target {
                Some(target) => shown_value(target, &field.unit, system, decimal),
                None => t.tally_no_target().to_string(),
            };
            let counts_sets = field.multiplies.is_some();
            let mut badges = Vec::new();
            if field.check {
                let kind = Text(t.tally_field_kind_check());
                badges.push((icons::TALLY_CHECK, t.tally_field_kind(), kind));
            }
            if counts_sets {
                let value = Text(t.tally_set_counter());
                badges.push((icons::TALLY_SETS, t.tally_counts_in_sets(), value));
            }
            // A field counting sets counts no value: no rule, no target.
            if !counts_sets && !field.check {
                badges.push((icons::TALLY_RULE, t.tally_rule(), rule));
            }
            if !field.check {
                badges.push((icons::TALLY_UNIT, t.tally_unit(), Text(&unit_value)));
            }
            if !counts_sets {
                let target = Text(&target_value);
                badges.push((icons::TALLY_TARGET, t.tally_target_badge(), target));
            }
            if field.hidden {
                // Only while on: the switch's on, as the other switches.
                let hidden = Icon(switch_icon(true), t.tally_field_hidden());
                badges.push((icons::TALLY_HIDDEN, t.tally_hide_field(), hidden));
            }
            badge_row(ui, |ui| {
                for (icon, setting, value) in badges {
                    if badge(ui, (t, tones), (icon, None), setting, Some(value), changes).clicked()
                    {
                        actions.push(TallyAction::OpenField(Some(field.id.clone())));
                    }
                }
            });
            ui.add_space(8.0);
        }
        if shown.defines {
            let add = ui.horizontal(|ui| {
                icons::show(ui, icons::ADD, 20.0, text);
                ui.button(RichText::new(t.tally_add_field()).color(text))
            });
            if add.inner.clicked() {
                actions.push(TallyAction::OpenField(None));
            }
        }
    });
}

/// A Tally on its Note's detailed card (#190): "Add entry" for who may
/// add one; in a Shared Area the choice of whose entries show; one chart
/// per field shown but those counting sets, a bar per the Tally's bar
/// length (#264), a Check field's of its ticks; the entries, newest
/// first, each with its ticks and its sets, the
/// older folded under the latest [`SHOWN_ENTRIES`]; and the folded
/// "Removed entries" with Restore. The card leaves its own click out of
/// where this lies, so its buttons take theirs.
pub fn card(ui: &mut Ui, t: &L10n, shown: &TallyShown, actions: &mut Vec<TallyAction>) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let tally = &shown.tally;
    let system = shown.system;
    let heading = |ui: &mut Ui, words: &str| {
        ui.add_space(4.0);
        ui.label(RichText::new(words).strong().color(text));
    };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        if shown.adds {
            let add = ui.horizontal(|ui| {
                icons::show(ui, icons::ADD, 20.0, text);
                ui.button(RichText::new(t.add_tally_entry()).color(text))
            });
            if add.inner.clicked() {
                actions.push(TallyAction::AddEntry);
            }
        }
        if shown.shared {
            heading(ui, t.tally_show_entries());
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
        }
        for field in tally.counting() {
            let unit = tally_units::shown_unit(&field.unit, system);
            let words = if unit.is_empty() {
                field.name.clone()
            } else {
                format!("{} ({unit})", field.name)
            };
            heading(ui, &words);
            let bars = shown.charts.get(&field.id).map_or(&[][..], Vec::as_slice);
            chart(ui, t, field, bars, (tally.bar, system));
        }
        heading(ui, t.tally_entries());
        if shown.entries.is_empty() {
            ui.label(RichText::new(t.tally_no_entries()).color(weak));
        }
        let older = shown.entries.len().saturating_sub(SHOWN_ENTRIES);
        let count = if shown.all_shown || older == 0 {
            shown.entries.len()
        } else {
            SHOWN_ENTRIES
        };
        for entry in shown.entries.iter().take(count) {
            entry_row(ui, t, (tally, system), entry, false, actions);
        }
        // The older ones folded under the latest (#190).
        if older > 0 {
            let icon = if shown.all_shown {
                icons::EXPAND_LESS
            } else {
                icons::EXPAND_MORE
            };
            let said = t.tally_older_entries(older as i64);
            let fold = ui.horizontal(|ui| {
                icons::show(ui, icon, 20.0, weak);
                ui.add(
                    egui::Button::new(RichText::new(&said).color(weak))
                        .frame(false)
                        .selected(shown.all_shown),
                )
            });
            if fold.inner.clicked() {
                actions.push(TallyAction::AllShown(!shown.all_shown));
            }
        }
        if shown.removed.is_empty() {
            return;
        }
        let open = shown.removed_open;
        let fold =
            egui::CollapsingHeader::new(RichText::new(t.tally_removed_entries()).color(text))
                .id_salt(("tally removed", &tally.note_id))
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
    lines.extend(values_lines(t, tally, entry, system));
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
        // A pick closes the menu, also one by keyboard or screen reader,
        // which no pointer click closes.
        if ui.button(RichText::new(t.edit()).color(text)).clicked() {
            actions.push(TallyAction::EditEntry(entry.id.clone()));
            ui.close();
        }
        if ui.button(RichText::new(t.delete()).color(text)).clicked() {
            actions.push(TallyAction::Delete(entry.id.clone()));
            ui.close();
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

/// One field's chart (#264): a slim bar of a fixed width per day, week,
/// month or year, with gaps, the newest on the right and as many of the
/// latest as fit the width, never scrolled sideways (older values are in
/// the entries); the target per bar as a thin line; under it the days
/// the bars cover and the highest bar's value.
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
    let shown = buckets
        .get(buckets.len().saturating_sub(fit)..)
        .unwrap_or(buckets);
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
        let fill = ui.visuals().selection.bg_fill;
        let count = shown.len();
        for (i, bucket) in shown.iter().enumerate() {
            let Some(value) = bucket.value else { continue };
            let right = rect.right() - (count - 1 - i) as f32 * SLOT;
            let (a, b) = (zero, y(value));
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(right - SLOT + GAP, a.min(b)),
                    egui::pos2(right, a.max(b)),
                ),
                0.0,
                fill,
            );
        }
        if let Some(target) = field.target {
            let at = y(target);
            painter.line_segment(
                [egui::pos2(rect.left(), at), egui::pos2(rect.right(), at)],
                egui::Stroke::new(1.0, ui.visuals().warn_fg_color),
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
    /// Deletes the entry, without asking (#268).
    Delete,
}

/// The field page or the entry page in its dialog over the window.
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

    /// Whether it is the field page of the Note being captured.
    pub fn capturing(&self) -> bool {
        matches!(self, TallyForm::Field(form) if form.capturing)
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
        if theme::primary(ui, true, t.save()).clicked() {
            intent = Some(FormIntent::Save);
        }
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            intent = Some(FormIntent::Close);
        }
        if delete && theme::destructive(ui, true, t.delete()).clicked() {
            intent = Some(FormIntent::Delete);
        }
    });
    intent
}

/// An error under its field, in red.
fn field_error(ui: &mut Ui, error: &str) {
    ui.label(RichText::new(error).color(ui.visuals().error_fg_color));
}

/// One field of a Tally: its name; a new one's kind (#264), a number or a
/// Check field, which has no unit and no rule, so they leave the page
/// while it is picked; its unit, typed into one field under which the
/// known units it may mean show as small buttons in this Device's system
/// (#159), a known one or one as typed, only of its kind once entries
/// hold values for it; its target, optional, per bar; and how several
/// sets or entries in a bar count, which follows the kind of unit until
/// it is picked. A field that counted sets as a multiplier did (#200) has
/// no target and no rule. A field that exists has "Hide field", or "Show field again", at the
/// bottom. Leaving without Save changes nothing. For the
/// Note being captured (#113) Save hands the field back instead
/// ([`FieldForm::choice`]), as the phone's `chooseTallyField`.
#[derive(Debug, Clone)]
pub struct FieldForm {
    pub note_id: String,
    /// Whether it is the first field of the Note being captured, which
    /// is not written yet.
    capturing: bool,
    /// The field shown; `None` for a new one.
    field: Option<TallyField>,
    pub name: String,
    /// The unit as typed.
    pub unit_text: String,
    /// The unit as the page opened, shown in this Device's system.
    unit_as_opened: String,
    /// The code of the known unit a suggestion put into the field, until
    /// it is typed in again.
    picked: Option<&'static str>,
    /// Once entries hold values, only units of the field's kind are left.
    fixed: Option<UnitKind>,
    /// The names the known units are matched by, in every language.
    unit_names: UnitNames,
    pub target: String,
    target_as_opened: String,
    pub rule: TallyRule,
    rule_picked: bool,
    /// Whether it is a Check field (#264): chosen for a new field only.
    pub check: bool,
    /// Whether it counts the sets of older entries (#264): no target, no
    /// rule.
    counts_sets: bool,
    system: UnitSystem,
    name_missing: bool,
    target_invalid: bool,
    unit_invalid: bool,
    /// The text field to take the keyboard on the next frame.
    focus: Option<&'static str>,
}

const NAME_FIELD: &str = "tally field name";
const UNIT_FIELD: &str = "tally unit text";
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
        let unit_as_opened = field
            .as_ref()
            .map(|f| tally_units::shown_unit(&f.unit, system))
            .unwrap_or_default();
        let target = field
            .as_ref()
            .map(|f| typed_value(f.target, &f.unit, system, decimal))
            .unwrap_or_default();
        FieldForm {
            note_id: note_id.to_string(),
            capturing: false,
            name: field.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
            unit_text: unit_as_opened.clone(),
            unit_as_opened,
            picked: None,
            fixed: field
                .as_ref()
                .filter(|f| f.has_entries)
                .map(TallyField::kind),
            unit_names: tally_unit_match::unit_names(system),
            target_as_opened: target.clone(),
            target,
            rule: field.as_ref().map_or(TallyRule::Sum, |f| f.rule),
            rule_picked: false,
            check: field.as_ref().is_some_and(|f| f.check),
            counts_sets: field.as_ref().is_some_and(|f| f.multiplies.is_some()),
            system,
            name_missing: false,
            target_invalid: false,
            unit_invalid: false,
            focus: field.is_none().then_some(NAME_FIELD),
            field,
        }
    }

    /// The unit a suggestion picked, else the unit typed: the field's own
    /// while the text is as it opened, so a unit of the user's own that
    /// reads like a known symbol stays theirs.
    fn chosen_unit(&self) -> String {
        match (self.picked, &self.field) {
            (Some(code), _) => code.to_string(),
            (None, Some(field)) if self.unit_text == self.unit_as_opened => field.unit.clone(),
            (None, _) => tally_unit_match::unit_of_typed(&self.unit_text, self.system),
        }
    }

    /// Every known unit left while the unit is as the page opened, so
    /// another one is a click away; then those the text typed may mean.
    fn matches(&self) -> Vec<&'static KnownUnit> {
        let typed = if self.unit_text == self.unit_as_opened {
            ""
        } else {
            &self.unit_text
        };
        tally_unit_match::matching_units(typed, self.system, &self.unit_names, self.fixed)
    }

    /// The rule follows the kind of unit until it is picked, for a new
    /// field.
    fn unit_changed(&mut self) {
        if !self.rule_picked && self.field.is_none() {
            self.rule = tally_units::default_rule(tally_units::kind_of(&self.chosen_unit()));
        }
    }

    /// The page of the first field of the Note being captured.
    pub fn choose(system: UnitSystem, t: &L10n) -> FieldForm {
        FieldForm {
            capturing: true,
            ..FieldForm::new("", None, system, t)
        }
    }

    /// The field as typed, written nowhere; `None` while something needs
    /// fixing, which then says so under its field.
    pub fn choice(&mut self) -> Option<TallyFieldChoice> {
        let (name, unit, target) = self.checked()?;
        Some(TallyFieldChoice {
            name,
            unit,
            target,
            rule: if self.check {
                TallyRule::Sum
            } else {
                self.rule
            },
            check: self.check,
        })
    }

    /// Writes the field when nothing needs fixing; whether it did.
    pub fn save(&mut self, store: &Store) -> oldbrainfix_core::Result<bool> {
        let Some((name, unit, target)) = self.checked() else {
            return Ok(false);
        };
        let target_text = self.target.trim().to_string();
        let tallies = Tallies::new(store);
        let note = &self.note_id;
        match &self.field {
            None if self.check => {
                tallies.add_check(note, &name, target)?;
            }
            None => {
                tallies.add_field(note, &name, &unit, target, Some(self.rule))?;
            }
            Some(field) => {
                if name != field.name {
                    tallies.rename_field(note, &field.id, &name)?;
                }
                let unit_changed = !field.check && unit != field.unit;
                if unit_changed {
                    tallies.set_unit(note, &field.id, &unit)?;
                }
                // A field counting sets keeps its target and rule.
                if !self.counts_sets {
                    // Read again in the unit now shown next to it.
                    if unit_changed || target_text != self.target_as_opened {
                        tallies.set_target(note, &field.id, target)?;
                    }
                    if !field.check && self.rule != field.rule {
                        tallies.set_rule(note, &field.id, self.rule)?;
                    }
                }
            }
        }
        Ok(true)
    }

    /// The name, unit and target as typed, the target in the base unit;
    /// `None` while one needs fixing, which then says so under its field
    /// and takes the keyboard.
    fn checked(&mut self) -> Option<(String, String, Option<f64>)> {
        let name = self.name.trim().to_string();
        let unit = if self.check {
            String::new()
        } else {
            self.chosen_unit()
        };
        let target_text = if self.counts_sets {
            String::new()
        } else {
            self.target.trim().to_string()
        };
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
            .is_some_and(|f| !f.check && f.has_entries && tally_units::kind_of(&unit) != f.kind());
        // The field to fix takes the keyboard.
        self.focus = if self.name_missing {
            Some(NAME_FIELD)
        } else if self.unit_invalid {
            Some(UNIT_FIELD)
        } else if self.target_invalid {
            Some(TARGET_FIELD)
        } else {
            None
        };
        if self.focus.is_some() {
            return None;
        }
        Some((name, unit, target))
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
        let fixed = self.fixed;
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
                    crate::text_menu::show(ui, t, &field, &self.name);
                    if focus == Some(NAME_FIELD) {
                        field.request_focus();
                    }
                    if self.name_missing {
                        field_error(ui, t.tally_field_name_missing());
                    }
                });
                let new = self.field.is_none();
                if new || self.check {
                    embossed(ui, tones, false, None, |ui| {
                        ui.label(RichText::new(t.tally_field_kind()).strong().color(text));
                        if new {
                            let options = [
                                (false, t.tally_field_kind_number().to_string()),
                                (true, t.tally_field_kind_check().to_string()),
                            ];
                            chip_row(ui, &mut self.check, options);
                            if self.check {
                                let detail = t.tally_field_kind_check_detail();
                                ui.label(RichText::new(detail).small().color(weak));
                            }
                        } else {
                            ui.label(RichText::new(t.tally_field_kind_check()).color(text));
                        }
                    });
                }
                if !self.check {
                    embossed(ui, tones, false, None, |ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        let label = ui.label(RichText::new(t.tally_unit()).strong().color(text));
                        if fixed.is_some() {
                            ui.label(RichText::new(t.tally_unit_fixed()).small().color(weak));
                        }
                        let field = ui
                            .add(
                                TextEdit::singleline(&mut self.unit_text)
                                    .id(Id::new(UNIT_FIELD))
                                    .hint_text(t.tally_unit_hint())
                                    .text_color(text)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(label.id);
                        crate::text_menu::show(ui, t, &field, &self.unit_text);
                        if focus == Some(UNIT_FIELD) {
                            field.request_focus();
                        }
                        // Typing again leaves the known unit picked; the
                        // kind typed may change the rule.
                        let mut changed = field.changed();
                        if changed {
                            self.picked = None;
                        }
                        if self.unit_invalid {
                            field_error(ui, t.tally_unit_fixed());
                        }
                        // A unit of the user's own takes no known one.
                        if fixed != Some(UnitKind::Typed) {
                            ui.label(RichText::new(t.tally_unit_helper()).small().color(weak));
                        }
                        let matches = self.matches();
                        if !matches.is_empty() {
                            // Suggestions that fill in the field, not a
                            // choice of their own.
                            ui.horizontal_wrapped(|ui| {
                                for known in matches {
                                    let shown = tally_units::shown_unit(known.code, self.system);
                                    let chip = ui
                                        .small_button(RichText::new(&shown).color(text))
                                        .on_hover_text(kind_name(t, known.kind));
                                    if chip.clicked() {
                                        self.picked = Some(known.code);
                                        self.unit_text = shown;
                                        changed = true;
                                    }
                                }
                            });
                        }
                        if changed {
                            self.unit_changed();
                        }
                    });
                }
                if !self.counts_sets {
                    embossed(ui, tones, false, None, |ui| {
                        let label = ui.label(RichText::new(t.tally_target()).strong().color(text));
                        ui.horizontal(|ui| {
                            let unit = if self.check {
                                String::new()
                            } else {
                                tally_units::shown_unit(&self.chosen_unit(), self.system)
                            };
                            let field = ui
                                .add(
                                    TextEdit::singleline(&mut self.target)
                                        .id(Id::new(TARGET_FIELD))
                                        .text_color(text)
                                        .desired_width(120.0),
                                )
                                .labelled_by(label.id);
                            crate::text_menu::show(ui, t, &field, &self.target);
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
                }
                if !self.counts_sets && !self.check {
                    embossed(ui, tones, false, None, |ui| {
                        ui.label(RichText::new(t.tally_rule()).strong().color(text));
                        let options = [
                            (TallyRule::Sum, t.tally_rule_sum().to_string()),
                            (TallyRule::Max, t.tally_rule_max().to_string()),
                        ];
                        if chip_row(ui, &mut self.rule, options) {
                            self.rule_picked = true;
                        }
                    });
                }
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

/// One entry of a Tally: a box per Check field (#264), ticked once per
/// entry above the sets; a value per number field shown, a new entry's
/// prefilled with the field's target, so that saving the targets is one
/// click, or for a field without one with the value of the user's latest
/// entry that has one, selected as the field takes the keyboard so typing
/// replaces it (#276). A Tally that counts in sets shows each set as a row of those
/// values with "Add set" under them, which adds one that starts with the
/// values of the set before; a set after the first can be removed. Then
/// the day it counts for, today for a new one, at the same time of day;
/// and one line of text. A value left empty is none. An entry that exists
/// has Delete, which asks nothing; Ctrl+Z restores it (#268). Leaving it
/// by Esc, a click beside it or back saves a change as Save does (#275);
/// one that can't be saved, a value that is no number or a new entry left
/// empty, asks once whether to discard it. Left as it opened, targets and
/// all, it writes nothing; Cancel writes nothing either.
///
/// For a Session's pick (ADR 0029) a new entry is recorded for its
/// Occurrence instead; it counts for now, so the day does not show.
#[derive(Debug, Clone)]
pub struct EntryForm {
    pub note_id: String,
    /// The entry shown; `None` for a new one.
    pub entry: Option<TallyEntry>,
    /// The number fields shown: a set is a row across them.
    fields: Vec<TallyField>,
    /// Each set, its values as typed, in the order of the fields.
    pub sets: Vec<Vec<String>>,
    /// The sets as the page opened, to write only what changed.
    opened: Vec<Vec<String>>,
    /// Each Check field shown, whether it is ticked.
    pub checks: Vec<(TallyField, bool)>,
    /// The ticks as the page opened, to tell on leaving whether anything
    /// changed (#275).
    checks_opened: Vec<bool>,
    /// Whether the Tally counts in sets: "Add set" shows then.
    counts_in_sets: bool,
    pub line: String,
    pub day: Day,
    day_as_opened: Day,
    pub day_text: String,
    /// Whether this Device may delete it.
    changes: bool,
    system: UnitSystem,
    /// The values that are no number, as `<set>:<field id>`.
    invalid: HashSet<String>,
    empty: bool,
    focus: Option<String>,
    /// The fields of a new entry that start with the value of the user's
    /// latest entry (#276), by id.
    from_latest: HashSet<String>,
    /// The value of one of them that took the keyboard holding that
    /// value, by its key: selected while the pointer that gave the
    /// keyboard is down, which would place the cursor.
    selecting: Option<String>,
    /// The Session and its Occurrence a new entry is recorded for.
    pick: Option<(String, Occurrence)>,
}

/// What a new entry of `tally` starts with (#276): each number field's
/// target, and for one of `fields` without, the value of the first set
/// of the latest of `mine`, the user's entries newest first, that holds
/// one; with the fields that start so.
fn start_values(
    tally: &Tally,
    fields: &[TallyField],
    mine: &[TallyEntry],
) -> (BTreeMap<String, f64>, HashSet<String>) {
    let mut values = tally.targets();
    let mut wanted: HashSet<String> = fields
        .iter()
        .filter(|f| f.target.is_none())
        .map(|f| f.id.clone())
        .collect();
    let mut latest = HashSet::new();
    for entry in mine {
        if wanted.is_empty() {
            break;
        }
        let Some(first) = tally.sets_of(entry).into_iter().next() else {
            continue;
        };
        for (id, value) in first {
            if wanted.remove(&id) {
                values.entry(id.clone()).or_insert(value);
                latest.insert(id);
            }
        }
    }
    (values, latest)
}

/// The key of the value of `field` in the set `set` of the entry page.
fn value_key(set: usize, field: &str) -> String {
    format!("{set}:{field}")
}

impl EntryForm {
    /// The page of `entry` of `tally`, or of a new one, on `today`,
    /// starting from `mine`, the user's entries newest first.
    pub fn new(
        tally: &Tally,
        (entry, mine): (Option<(TallyEntry, bool)>, &[TallyEntry]),
        (system, today): (UnitSystem, Day),
        wall: &WallClock,
        t: &L10n,
    ) -> EntryForm {
        let decimal = decimal_sign(t);
        let fields: Vec<TallyField> = tally.numbers().into_iter().cloned().collect();
        let mut from_latest = HashSet::new();
        let rows = match &entry {
            None => {
                let (values, latest) = start_values(tally, &fields, mine);
                from_latest = latest;
                vec![values]
            }
            Some((entry, _)) => tally.sets_of(entry),
        };
        let rows = if rows.is_empty() {
            vec![BTreeMap::new()]
        } else {
            rows
        };
        let sets: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                fields
                    .iter()
                    .map(|f| typed_value(row.get(&f.id).copied(), &f.unit, system, decimal))
                    .collect()
            })
            .collect();
        let checks: Vec<(TallyField, bool)> = tally
            .checks()
            .into_iter()
            .map(|f| {
                let on = entry
                    .as_ref()
                    .is_some_and(|(e, _)| e.values.contains_key(&f.id));
                (f.clone(), on)
            })
            .collect();
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
            focus: fields.first().map(|f| value_key(0, &f.id)),
            fields,
            opened: sets.clone(),
            sets,
            checks_opened: checks.iter().map(|(_, on)| *on).collect(),
            checks,
            counts_in_sets: tally.counts_in_sets,
            day,
            day_as_opened: day,
            day_text: day.to_string(),
            system,
            invalid: HashSet::new(),
            empty: false,
            from_latest,
            selecting: None,
            pick: None,
        }
    }

    /// Records a new entry for the pick of `occurrence` of the Session
    /// `session`.
    pub fn for_pick(mut self, session: &str, occurrence: Occurrence) -> EntryForm {
        if self.entry.is_none() {
            self.pick = Some((session.to_string(), occurrence));
        }
        self
    }

    /// "Add set": a set that starts with the values of the set before.
    pub fn add_set(&mut self) {
        if let Some(last) = self.sets.last().cloned()
            && self.sets.len() < oldbrainfix_core::tallies::keys::MAX_SETS
        {
            self.sets.push(last);
            self.focus = self
                .fields
                .first()
                .map(|f| value_key(self.sets.len() - 1, &f.id));
        }
    }

    /// The values of each set, `None` where empty or no number, and the
    /// values that are no number, as `<set>:<field id>`.
    fn parsed(&self) -> (Vec<BTreeMap<String, Option<f64>>>, HashSet<String>) {
        let mut parsed: Vec<BTreeMap<String, Option<f64>>> = Vec::new();
        let mut invalid = HashSet::new();
        for (i, set) in self.sets.iter().enumerate() {
            let mut row = BTreeMap::new();
            for (field, typed) in self.fields.iter().zip(set) {
                let typed = typed.trim();
                if typed.is_empty() {
                    row.insert(field.id.clone(), None);
                    continue;
                }
                let value = tally_units::parse_shown(typed, &field.unit, self.system);
                if value.is_none() {
                    invalid.insert(value_key(i, &field.id));
                }
                row.insert(field.id.clone(), value);
            }
            parsed.push(row);
        }
        (parsed, invalid)
    }

    /// A new entry with nothing in it, its values `parsed`.
    fn is_empty(&self, parsed: &[BTreeMap<String, Option<f64>>]) -> bool {
        self.entry.is_none()
            && self.line.trim().is_empty()
            && !self.checks.iter().any(|(_, on)| *on)
            && parsed.iter().all(|row| row.values().all(Option::is_none))
    }

    /// Whether the day typed is no day, or not the one shown.
    fn day_wrong(&self) -> bool {
        Day::parse(self.day_text.trim()) != Some(self.day)
    }

    /// Whether Save would save: every value a number or empty, the day a
    /// day, and a new entry not empty.
    pub fn valid(&self) -> bool {
        let (parsed, invalid) = self.parsed();
        invalid.is_empty() && !self.day_wrong() && !self.is_empty(&parsed)
    }

    /// Whether anything differs from the page as it opened (#275).
    pub fn changed(&self) -> bool {
        let typed = |sets: &[Vec<String>]| -> Vec<Vec<String>> {
            sets.iter()
                .map(|set| set.iter().map(|v| v.trim().to_string()).collect())
                .collect()
        };
        let line_opened = self.entry.as_ref().map_or("", |e| e.line.as_str());
        self.day != self.day_as_opened
            || self.day_text.trim() != self.day_as_opened.to_string()
            || self.line.trim() != line_opened.trim()
            || self
                .checks
                .iter()
                .map(|(_, on)| *on)
                .ne(self.checks_opened.iter().copied())
            || typed(&self.sets) != typed(&self.opened)
    }

    /// What leaving the page comes to (#275).
    pub fn leaving(&self) -> Leaving {
        Leaving::of(self.changed(), self.valid())
    }

    /// Says under each field what to fix, the first taking the keyboard,
    /// as Save and Keep editing do; whether anything needs fixing.
    pub fn show_what_to_fix(&mut self) -> bool {
        let (parsed, invalid) = self.parsed();
        self.empty = invalid.is_empty() && self.is_empty(&parsed);
        let day_wrong = self.day_wrong();
        // The field to fix takes the keyboard.
        let first_invalid = (0..self.sets.len()).find_map(|i| {
            self.fields
                .iter()
                .map(|f| value_key(i, &f.id))
                .find(|key| invalid.contains(key))
        });
        self.focus = first_invalid
            .or_else(|| day_wrong.then(|| DAY_FIELD.to_string()))
            .or_else(|| self.empty.then(|| LINE_FIELD.to_string()));
        self.invalid = invalid;
        self.focus.is_some()
    }

    /// Writes the entry when nothing needs fixing; whether it did.
    pub fn save(&mut self, store: &Store, wall: &WallClock) -> oldbrainfix_core::Result<bool> {
        if self.show_what_to_fix() {
            return Ok(false);
        }
        let (parsed, _) = self.parsed();
        let line = self.line.trim().to_string();
        let held = |row: &BTreeMap<String, Option<f64>>| -> BTreeMap<String, f64> {
            row.iter()
                .filter_map(|(id, value)| Some((id.clone(), (*value)?)))
                .collect()
        };
        let more: Vec<BTreeMap<String, f64>> = parsed.iter().skip(1).map(held).collect();
        let first = parsed.first().cloned().unwrap_or_default();
        let tallies = Tallies::new(store);
        let base = self.entry.as_ref().map_or_else(|| store.now(), |e| e.at);
        let at = (self.day != self.day_as_opened).then(|| moved_to(base, self.day, wall));
        match &self.entry {
            None => {
                let mut values = held(&first);
                for (field, on) in &self.checks {
                    if *on {
                        values.insert(field.id.clone(), 1.0);
                    }
                }
                let recorded = match &self.pick {
                    Some((session, occurrence)) => {
                        let sessions = Sessions::new(store);
                        let note = &self.note_id;
                        match sessions.record_sets(session, occurrence, note, &values, &more, &line)
                        {
                            // The pick changed meanwhile, such as by a
                            // sync: the entry still goes to its Tally, so
                            // nothing typed is lost.
                            Err(Error::Invalid(_)) => false,
                            done => done.map(|_| true)?,
                        }
                    }
                    None => false,
                };
                if !recorded {
                    tallies.add_with(&self.note_id, &values, &more, &line, at, |_| Ok(()))?;
                }
            }
            Some(entry) => {
                // Only those typed anew: one shown rounded stays as it is.
                let opened = self.opened.first();
                let typed = self.sets.first();
                let mut retyped: BTreeMap<String, Option<f64>> = self
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| {
                        let now = typed.and_then(|s| s.get(*i)).map(|s| s.trim());
                        now != opened.and_then(|s| s.get(*i)).map(String::as_str)
                    })
                    .filter_map(|(_, field)| Some((field.id.clone(), *first.get(&field.id)?)))
                    .collect();
                for (field, on) in &self.checks {
                    if *on != entry.values.contains_key(&field.id) {
                        retyped.insert(field.id.clone(), on.then_some(1.0));
                    }
                }
                // The further sets as typed, while there were or are
                // several.
                let sets = (self.sets.len() > 1 || self.opened.len() > 1).then_some(&more[..]);
                tallies.edit_entry_sets(&entry.id, &retyped, sets, Some(&line), at)?;
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
        let in_sets = self.counts_in_sets || self.sets.len() > 1;
        egui::ScrollArea::vertical()
            .id_salt("tally entry")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                intent = page_bar(ui, t, heading, self.entry.is_some() && self.changes);
                if !self.checks.is_empty() {
                    embossed(ui, tones, false, None, |ui| {
                        for (field, on) in &mut self.checks {
                            ui.checkbox(on, RichText::new(&field.name).color(text));
                        }
                    });
                }
                let mut removed = None;
                for (i, set) in self.sets.iter_mut().enumerate() {
                    if in_sets {
                        ui.horizontal(|ui| {
                            let words = t.tally_set(i as i64 + 1);
                            ui.label(RichText::new(words).strong().color(text));
                            if i > 0 {
                                let remove = t.tally_remove_set(i as i64 + 1);
                                if icon_button(ui, icons::REMOVE, &remove, text).clicked() {
                                    removed = Some(i);
                                }
                            }
                        });
                    }
                    for (field, typed) in self.fields.iter().zip(set.iter_mut()) {
                        let key = value_key(i, &field.id);
                        embossed(ui, tones, false, None, |ui| {
                            let label = ui.label(RichText::new(&field.name).strong().color(text));
                            ui.horizontal(|ui| {
                                let id = if i == 0 {
                                    Id::new(("tally value", &field.id))
                                } else {
                                    Id::new(("tally value", i, &field.id))
                                };
                                let edit = ui
                                    .add(
                                        TextEdit::singleline(typed)
                                            .id(id)
                                            .text_color(text)
                                            .desired_width(120.0),
                                    )
                                    .labelled_by(label.id);
                                crate::text_menu::show(ui, t, &edit, typed);
                                if focus.as_deref() == Some(key.as_str()) {
                                    edit.request_focus();
                                }
                                // The latest value, as it opened, is
                                // selected as it takes the keyboard
                                // (#276), also against a click's cursor.
                                let as_opened = self.opened.first().and_then(|s| {
                                    let at = self.fields.iter().position(|f| f.id == field.id)?;
                                    s.get(at)
                                });
                                if i == 0
                                    && edit.gained_focus()
                                    && self.from_latest.contains(&field.id)
                                    && as_opened == Some(&*typed)
                                {
                                    self.selecting = Some(key.clone());
                                }
                                if self.selecting.as_deref() == Some(key.as_str()) {
                                    if edit.has_focus() {
                                        crate::format_bar::select(ui, id, typed, (0, typed.len()));
                                    }
                                    if !edit.has_focus() || !ui.input(|i| i.pointer.any_down()) {
                                        self.selecting = None;
                                    }
                                }
                                let unit = tally_units::shown_unit(&field.unit, system);
                                ui.label(RichText::new(unit).color(text));
                            });
                            if self.invalid.contains(&key) {
                                field_error(ui, t.tally_not_a_number());
                            }
                        });
                    }
                }
                if let Some(i) = removed {
                    self.sets.remove(i);
                }
                if self.counts_in_sets && !self.fields.is_empty() {
                    let add = ui.horizontal(|ui| {
                        icons::show(ui, icons::ADD, 20.0, text);
                        ui.button(RichText::new(t.tally_add_set()).color(text))
                    });
                    if add.inner.clicked() {
                        self.add_set();
                    }
                }
                if self.pick.is_none() {
                    embossed(ui, tones, false, None, |ui| {
                        let label =
                            ui.label(RichText::new(t.tally_entry_date()).strong().color(text));
                        self.day_editor(ui, t, (label.id, focus.as_deref() == Some(DAY_FIELD)));
                    });
                }
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
                    crate::text_menu::show(ui, t, &field, &self.line);
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
            crate::text_menu::show(ui, t, &field, &self.day_text);
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
