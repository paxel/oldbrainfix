//! The schedule as a sentence at the top of the Routine editor (#227),
//! driven headless by egui_kittest: written as the schedule changes,
//! typed to fill the chips and times, with suggestions and the words not
//! understood marked, read-only on an Advanced schedule.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    ClockTime, Day, Recurrence, Routines, Schedule, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::App;
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::schedule_text::{weekday_long, weekday_text};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// Sunday, 25 October 2026, noon in Berlin.
const NOW: &str = "2026-10-25T11:00:00.000Z";

/// A store at [`NOW`] with the Routine `pills` on `rule` from `start` at
/// `times`.
fn store(rule: &str, start: Day, times: &[ClockTime]) -> Result<Store, Box<dyn std::error::Error>> {
    let now = parse_time(NOW)?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    let schedule = Schedule::new(start, times, Some(Recurrence::parse(rule)?)).ok_or("schedule")?;
    Routines::new(&store).create("pills", &schedule)?;
    Ok(store)
}

/// The window over `store` in `locale`, the editor of `pills` open.
fn editor(store: Store, locale: &str) -> Harness<'static, App> {
    let mut app = App::new(store, locale);
    app.set_wall_clock(Arc::new(berlin));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness
        .query_all_by_label("pills")
        .next()
        .unwrap_or_else(|| panic!("no pills on the Agenda"))
        .click();
    harness.run();
    harness
}

/// What the sentence line holds.
fn sentence(harness: &Harness<'_, App>, label: &str) -> String {
    harness
        .get_by(|n| n.role() == Role::MultilineTextInput && n.label().as_deref() == Some(label))
        .value()
        .unwrap_or_default()
}

/// Clicks the sentence line and types `text` over it.
fn type_sentence(harness: &mut Harness<'_, App>, label: &str, text: &str) {
    harness
        .get_by(|n| n.role() == Role::MultilineTextInput && n.label().as_deref() == Some(label))
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn picked(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .accesskit_node()
        .toggled()
        == Some(Toggled::True)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn at(hour: u32) -> ClockTime {
    ClockTime::new(hour, 0).unwrap_or(ClockTime { hour: 0, minute: 0 })
}

#[test]
fn the_line_shows_the_schedule_and_follows_the_chips() -> TestResult {
    let start = Day::new(2026, 10, 1);
    let mut harness = editor(store("FREQ=DAILY", start, &[at(8)])?, "en");
    assert_eq!(sentence(&harness, "When"), "Every day at 08:00, from 10/1");
    harness
        .get_by_role_and_label(Role::RadioButton, "Weekdays")
        .click();
    harness.run();
    // Weekdays start with the start's, a Thursday.
    assert_eq!(
        sentence(&harness, "When"),
        "Weekdays Thu at 08:00, from 10/1"
    );
    Ok(())
}

#[test]
fn typing_fills_the_chips_and_times_and_save_keeps_the_schedule() -> TestResult {
    let today = Day::new(2026, 10, 25);
    let mut harness = editor(store("FREQ=DAILY", today, &[])?, "en");
    assert_eq!(sentence(&harness, "When"), "Every day");
    type_sentence(
        &mut harness,
        "When",
        "Weekdays Mon, Thu at 08:00, 15:00, until 12/31",
    );
    assert!(picked(&harness, "Weekdays"));
    assert!(shows(&harness, "08:00"));
    assert!(shows(&harness, "15:00"));
    assert!(picked(&harness, "On a day"));
    harness.get_by_label("Save").click();
    harness.run();
    let routines = Routines::new(harness.state().store());
    let routine = routines.all()?.into_iter().next().ok_or("routine")?;
    assert_eq!(
        routine.schedule.rule.map(|r| r.format()).as_deref(),
        Some("FREQ=WEEKLY;UNTIL=20261231;BYDAY=MO,TH")
    );
    assert_eq!(routine.schedule.times, [at(8), at(15)]);
    Ok(())
}

#[test]
fn unknown_words_are_marked_and_change_nothing() -> TestResult {
    let today = Day::new(2026, 10, 25);
    let mut harness = editor(store("FREQ=DAILY", today, &[])?, "en");
    type_sentence(&mut harness, "When", "Every 2 wekks at 08:00");
    assert!(shows(
        &harness,
        "Marked words are not understood and change nothing."
    ));
    assert!(picked(&harness, "Every …"));
    assert!(picked(&harness, "day"));
    assert!(!shows(&harness, "08:00"));
    type_sentence(&mut harness, "When", "Every day at 08:00 please");
    assert!(shows(&harness, "08:00"));
    // Left, the line shows the schedule again, without the word.
    harness.get_by_label("Times of day").click();
    harness.run();
    assert_eq!(sentence(&harness, "When"), "Every day at 08:00");
    assert!(!shows(
        &harness,
        "Marked words are not understood and change nothing."
    ));
    Ok(())
}

#[test]
fn suggestions_offer_the_next_word_and_put_it_in() -> TestResult {
    let today = Day::new(2026, 10, 25);
    let mut harness = editor(store("FREQ=DAILY", today, &[])?, "en");
    type_sentence(&mut harness, "When", "Every 2 w");
    harness.get_by_role_and_label(Role::Button, "weeks").click();
    harness.run();
    assert_eq!(sentence(&harness, "When"), "Every 2 weeks ");
    assert!(picked(&harness, "weeks"));
    // The line keeps the keyboard, and offers what follows the repeat.
    assert!(
        harness
            .query_all_by(|n| n.is_focused() && n.role() == Role::MultilineTextInput)
            .next()
            .is_some()
    );
    assert!(shows(&harness, "until"));
    Ok(())
}

#[test]
fn an_advanced_schedule_shows_a_read_only_summary() -> TestResult {
    let today = Day::new(2026, 10, 25);
    let harness = editor(store("FREQ=DAILY;BYHOUR=8,20", today, &[])?, "en");
    assert!(shows(&harness, "Switch to Simple to type."));
    assert_eq!(
        sentence(&harness, "When"),
        "Rule: FREQ=DAILY;BYHOUR=8,20 · 08:00, 20:00"
    );
    Ok(())
}

#[test]
fn in_german_with_the_apps_weekday_names() -> TestResult {
    let today = Day::new(2026, 10, 25);
    let mut harness = editor(store("FREQ=WEEKLY;BYDAY=MO,TH", today, &[at(8)])?, "de");
    let de = L10n::new("de");
    // The phone's short names: the desktop's without their dot.
    let mon = weekday_text(&de, 1).trim_end_matches('.');
    let thu = weekday_text(&de, 4).trim_end_matches('.');
    assert_eq!(
        sentence(&harness, "Wann"),
        format!("Wochentags {mon}, {thu} um 08:00")
    );
    type_sentence(&mut harness, "Wann", "Jeden Monat am letzten Freitag");
    assert!(picked(&harness, "Monatlich an einem Wochentag"));
    assert!(picked(&harness, weekday_long(&de, 5)));
    Ok(())
}
