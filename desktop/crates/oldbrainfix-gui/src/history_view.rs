//! A Routine's History (Q400), as the phone's `RoutineHistoryScreen`:
//! its Note and schedule; its Streak, current and longest, with the last
//! seven days as dots, unless this Device's Settings switched Streaks
//! off; its Adherence this week, this month and over the last 30 days;
//! and the Log of every Occurrence, the latest first, Done (when, early
//! or late), Missed or still due, with the snoozes this Device recorded.
//! It only reads.

use chrono::{DateTime, Utc};
use egui::{Color32, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::schedule::WallClock;
use oldbrainfix_core::{
    Adherence, ClockTime, Day, DayTally, DoneTiming, HistoryEntry, OccurrenceStatus, RoutineHistory,
};

use crate::app::embossed;
use crate::icons;
use crate::l10n::L10n;
use crate::schedule_text::{clock_text, day_text, occurrence_time, weekday_text};
use crate::theme::Tones;

/// How many lines of its Note's text the page shows at most.
const TITLE_LINES: usize = 3;

/// A Routine's History as the page shows it.
#[derive(Debug, Clone)]
pub struct HistoryShown {
    /// Its Note's text, what the Note holds when it has no words, or
    /// that the Note is gone.
    pub title: String,
    /// Its schedule in words.
    pub schedule: String,
    pub history: RoutineHistory,
    /// Whether this Device shows Streaks.
    pub streaks: bool,
}

/// The week's first day in `t`'s language, 1 for Monday to 7 for
/// Sunday, as the phone takes it from its locale: Monday in German,
/// Sunday in English.
pub fn week_start(t: &L10n) -> u32 {
    match t.locale() {
        "de" => 1,
        _ => 7,
    }
}

/// `share` (0 to 1) as a whole percentage, as the phone's
/// `NumberFormat.percentPattern` writes it: `92%` or `92 %`.
pub fn percent_text(t: &L10n, share: f64) -> String {
    let percent = (share * 100.0).round();
    match t.locale() {
        "de" => format!("{percent}\u{a0}%"),
        _ => format!("{percent}%"),
    }
}

/// The page: its heading, and the History of `shown`, `wall` turning
/// the instants of the Log into this Device's days and times.
pub fn page(ui: &mut Ui, t: &L10n, tones: Tones, shown: Option<&HistoryShown>, wall: &WallClock) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.heading(RichText::new(t.history_title()).color(text));
    ui.add_space(8.0);
    let Some(shown) = shown else {
        return;
    };
    let history = &shown.history;
    egui::ScrollArea::vertical()
        .id_salt("routine history")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add_space(4.0);
            let title: Vec<&str> = shown.title.trim().lines().take(TITLE_LINES).collect();
            ui.label(
                RichText::new(title.join("\n"))
                    .size(16.0)
                    .strong()
                    .color(text),
            );
            ui.label(RichText::new(&shown.schedule).color(text));
            if shown.streaks {
                embossed(ui, tones, false, None, |ui| {
                    streak_panel(ui, t, tones, history)
                });
            }
            embossed(ui, tones, false, None, |ui| adherence_panel(ui, t, history));
            ui.label(RichText::new(t.history_log()).strong().color(text));
            ui.label(RichText::new(t.snoozes_here_device()).small().color(weak));
            if history.entries.is_empty() {
                ui.label(RichText::new(t.history_empty()).color(text));
            }
            for entry in history.entries.iter().rev() {
                log_row(ui, t, tones, entry, wall);
            }
        });
}

/// The current and longest Streak, and the last seven days as dots.
fn streak_panel(ui: &mut Ui, t: &L10n, tones: Tones, history: &RoutineHistory) {
    let text = ui.visuals().text_color();
    ui.label(RichText::new(t.streak_heading()).strong().color(text));
    ui.columns(2, |columns| {
        for (ui, (label, count)) in columns.iter_mut().zip([
            (t.streak_now(), history.current_streak),
            (t.streak_longest(), history.longest_streak),
        ]) {
            ui.label(RichText::new(count.to_string()).size(28.0).color(text));
            ui.label(RichText::new(label).color(text));
        }
    });
    ui.add_space(4.0);
    ui.label(RichText::new(t.last_seven_days()).color(text));
    ui.columns(history.last_seven_days.len().max(1), |columns| {
        for (ui, tally) in columns.iter_mut().zip(&history.last_seven_days) {
            day_dot(ui, t, tones, tally);
        }
    });
}

/// One day as a dot under its weekday: all Done, partly, Missed, still
/// due, or nothing due; a screen reader says which.
fn day_dot(ui: &mut Ui, t: &L10n, tones: Tones, tally: &DayTally) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    let day = day_text(t, tally.day);
    let (icon, colour, label) = if tally.is_empty() {
        (icons::REMOVE, weak, t.day_nothing(&day))
    } else if tally.missed > 0 && tally.done > 0 {
        (icons::DAY_PARTLY, tertiary(ui), t.day_partly_done(&day))
    } else if tally.missed > 0 {
        (icons::STATUS_MISSED, error, t.day_missed(&day))
    } else if tally.open > 0 {
        (icons::PICK_OPEN, tones.accent, t.day_open(&day))
    } else {
        (icons::STATUS_DONE, tones.accent, t.day_all_done(&day))
    };
    let shown = ui.vertical_centered(|ui| {
        ui.label(RichText::new(weekday_text(t, tally.day.weekday())).color(text));
        icons::show(ui, icon, 28.0, colour);
    });
    shown
        .response
        .interact(Sense::hover())
        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &label));
}

/// The phone's `tertiary`: the colour of a day partly Done, partly
/// Missed, between the accent and the error colour.
fn tertiary(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(0xE0, 0xB0, 0x60)
    } else {
        Color32::from_rgb(0x8A, 0x5A, 0x00)
    }
}

/// The Adherence this week, this month and over the last 30 days.
fn adherence_panel(ui: &mut Ui, t: &L10n, history: &RoutineHistory) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.adherence_heading()).strong().color(text));
    for (label, adherence) in [
        (t.adherence_this_week(), history.this_week),
        (t.adherence_this_month(), history.this_month),
        (t.adherence_last30_days(), history.last_30_days),
    ] {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).color(text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match adherence_figures(t, adherence) {
                    None => drop(ui.label(RichText::new(t.adherence_none()).color(text))),
                    Some((of, percent)) => {
                        ui.label(RichText::new(percent).size(16.0).color(text));
                        ui.label(RichText::new(of).small().color(weak));
                    }
                }
            });
        });
    }
}

/// "9 of 10 Done" and its percentage; `None` while nothing counts yet.
fn adherence_figures(t: &L10n, adherence: Adherence) -> Option<(String, String)> {
    let share = adherence.share()?;
    let total = adherence.done + adherence.missed;
    Some((
        t.adherence_of(adherence.done as i64, total as i64),
        percent_text(t, share),
    ))
}

/// One Occurrence in the Log: when it was due, and what came of it.
fn log_row(ui: &mut Ui, t: &L10n, tones: Tones, entry: &HistoryEntry, wall: &WallClock) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let (icon, colour) = match entry.status {
        OccurrenceStatus::Done => (icons::STATUS_DONE, tones.accent),
        OccurrenceStatus::Missed => (icons::STATUS_MISSED, ui.visuals().error_fg_color),
        OccurrenceStatus::Open => (icons::PICK_OPEN, tones.accent),
        OccurrenceStatus::Coming => (icons::CONTEXT_WHEN, weak),
    };
    let occurrence = &entry.occurrence;
    ui.horizontal(|ui| {
        icons::show(ui, icon, 24.0, colour);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let due = format!(
                "{} · {}",
                day_text(t, occurrence.day),
                occurrence_time(t, occurrence)
            );
            ui.label(RichText::new(due).color(text));
            ui.label(RichText::new(outcome(t, entry, wall)).color(weak));
            if let Some(until) = entry.snoozed_until {
                let line = t.log_snoozed(&when(t, until, occurrence.day, wall));
                ui.label(RichText::new(line).color(weak));
            }
        });
    });
}

/// What came of `entry`: Done when (early or late), Missed, due or still
/// to come.
pub fn outcome(t: &L10n, entry: &HistoryEntry, wall: &WallClock) -> String {
    let Some(done_at) = entry.done_at else {
        return match entry.status {
            OccurrenceStatus::Missed => t.log_missed(),
            OccurrenceStatus::Coming => t.log_coming(),
            _ => t.log_open(),
        }
        .to_string();
    };
    let at = when(t, done_at, entry.occurrence.day, wall);
    match entry.timing() {
        Some(DoneTiming::Early) => t.log_done_early(&at),
        Some(DoneTiming::Late) => t.log_done_late(&at),
        _ => t.log_done(&at),
    }
}

/// `instant` on this Device's clock: its time, with the day when that is
/// not `day`.
pub fn when(t: &L10n, instant: DateTime<Utc>, day: Day, wall: &WallClock) -> String {
    let local = wall(instant);
    let time = chrono::Timelike::hour(&local);
    let minute = chrono::Timelike::minute(&local);
    let time = ClockTime::new(time, minute)
        .map(clock_text)
        .unwrap_or_default();
    let on = Day::of(local);
    if on == day {
        time
    } else {
        format!("{} {time}", day_text(t, on))
    }
}
