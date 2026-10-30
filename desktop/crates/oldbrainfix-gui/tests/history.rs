//! A Routine's History on the desktop (Q400): opened from an
//! Occurrence's menu and from the Routine's editor, its Streak, its
//! Adherence with the week starting as the language has it, its Log with
//! early, late and snoozed Occurrences, and the Settings' switch that
//! hides the Streak; driven headless by egui_kittest on a European wall
//! clock.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::alarms::Alarms;
use oldbrainfix_core::{
    ClockTime, Day, DeviceSettings, Occurrence, Recurrence, Routine, Routines, Schedule,
    SigningKey, Store, parse_time,
};
use oldbrainfix_gui::history_view;
use oldbrainfix_gui::schedule_text::schedule_text;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// Saturday, 24 October 2026, 07:00 in Europe: the Routine is made.
const MADE: &str = "2026-10-24T05:00:00.000Z";
/// Wednesday, 28 October 2026, noon in Europe.
const NOW: &str = "2026-10-28T11:00:00.000Z";

fn at(day: u32, hour: u32, minute: u32) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let wall = chrono::NaiveDate::from_ymd_opt(2026, 10, day)
        .and_then(|d| d.and_hms_opt(hour, minute, 0))
        .ok_or("a day")?;
    let local = wall
        .and_local_timezone(chrono_tz::Europe::Berlin)
        .single()
        .ok_or("a time in Europe")?;
    Ok(local.with_timezone(&Utc))
}

fn pills_at(day: u32) -> Option<Occurrence> {
    Some(Occurrence::new(
        Day::new(2026, 10, i64::from(day)),
        Some(ClockTime::new(8, 0)?),
    ))
}

/// A store holding "Take the pills" every day at 08:00 since the 24th:
/// Done on time on the 24th, Missed on the 25th, Done early on the 26th,
/// Done late on the 27th and due today, snoozed until 08:30; and a
/// Reminder tomorrow. Its clock shows [`NOW`].
fn store() -> Result<(Store, Routine), Box<dyn std::error::Error>> {
    let now = Arc::new(Mutex::new(parse_time(MADE)?));
    let held = now.clone();
    let store = Store::in_memory(
        Box::new(move || held.lock().map(|t| *t).unwrap_or_default()),
        || Ok(SigningKey::from_seed([7; 32])),
    )?;
    let routines = Routines::new(&store);
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e:?}"))?;
    let times = [ClockTime::new(8, 0).ok_or("a time")?];
    let daily = Schedule::new(Day::new(2026, 10, 24), &times, Some(rule)).ok_or("a schedule")?;
    let pills = routines.create("Take the pills", &daily)?;
    let once = Schedule::new(Day::new(2026, 10, 29), &times, None).ok_or("a schedule")?;
    routines.create("Dentist", &once)?;
    *now.lock().map_err(|_| "the clock")? = parse_time(NOW)?;
    let day = |d| pills_at(d).ok_or("an Occurrence");
    routines.mark_done(&pills.id, &day(24)?, Some(at(24, 8, 5)?))?;
    routines.mark_done(&pills.id, &day(26)?, Some(at(26, 7, 30)?))?;
    routines.mark_done(&pills.id, &day(27)?, Some(at(28, 9, 0)?))?;
    Alarms::new(&store).apply_snooze(&pills.id, &day(28)?.key(), at(28, 8, 30)?)?;
    Ok((store, pills))
}

fn window(mut app: App) -> Harness<'static, App> {
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
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

fn shows_exactly(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

#[test]
fn an_occurrences_menu_opens_its_history_and_back_returns_to_the_agenda() -> TestResult {
    let (store, pills) = store()?;
    let mut harness = window(App::new(store, "en"));
    // Today's row, the first of the week's.
    harness
        .query_all_by_label("More for Take the pills")
        .next()
        .ok_or("today's row")?
        .click();
    harness.run();
    harness.get_by_label("History").click_accesskit();
    harness.run();
    assert_eq!(
        *harness.state().selection(),
        Selection::History(pills.id.clone())
    );
    assert!(harness.query_by_label("Share").is_none());
    assert!(shows_exactly(&harness, "Take the pills"));
    let schedule = schedule_text(harness.state().t(), &pills.schedule);
    assert!(shows_exactly(&harness, &schedule), "{schedule}");

    // The Streak counts back to the Missed 25th; the 24th alone is the
    // other run.
    assert!(shows_exactly(&harness, "Streak"));
    assert!(shows_exactly(&harness, "Now"));
    assert!(shows_exactly(&harness, "Longest"));
    assert!(shows_exactly(&harness, "2"));
    assert!(shows(&harness, "Thu, 10/22/2026: nothing due"));
    assert!(shows(&harness, "Sat, 10/24/2026: all Done"));
    assert!(shows(&harness, "Sun, 10/25/2026: Missed"));
    assert!(shows(&harness, "Wed, 10/28/2026: still due"));

    // The English week starts on Sunday: the 25th to the 27th count.
    assert!(shows_exactly(&harness, "This week"));
    assert!(shows_exactly(&harness, "2 of 3 Done"));
    assert!(shows_exactly(&harness, "67%"));
    assert!(shows_exactly(&harness, "3 of 4 Done"));
    assert!(shows_exactly(&harness, "75%"));

    // The Log, the latest first, with the times on this computer's clock.
    assert!(shows_exactly(&harness, "Wed, 10/28/2026 · 08:00"));
    assert!(shows_exactly(&harness, "Due, not Done yet"));
    assert!(shows_exactly(&harness, "Snoozed until 08:30"));
    assert!(shows_exactly(&harness, "Done late: Wed, 10/28/2026 09:00"));
    assert!(shows_exactly(&harness, "Done early: 07:30"));
    assert!(shows_exactly(&harness, "Missed"));
    assert!(shows_exactly(&harness, "Done: 08:05"));
    let today = harness.get_by_label("Wed, 10/28/2026 · 08:00").rect().top();
    let first = harness.get_by_label("Sat, 10/24/2026 · 08:00").rect().top();
    assert!(today < first, "the latest first");
    assert!(shows(&harness, "as this computer recorded them"));

    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    Ok(())
}

#[test]
fn a_reminder_has_no_history_in_its_menu() -> TestResult {
    let (store, _) = store()?;
    let mut harness = window(App::new(store, "en"));
    harness.get_by_label("More for Dentist").click();
    harness.run();
    assert!(harness.query_by_label("Share").is_some());
    assert!(harness.query_by_label("History").is_none());
    Ok(())
}

#[test]
fn the_german_week_starts_on_monday() -> TestResult {
    let (store, pills) = store()?;
    let mut app = App::new(store, "de");
    app.select(Selection::History(pills.id.clone()));
    let harness = window(app);
    let t = *harness.state().t();
    assert!(shows_exactly(&harness, t.history_title()));
    // The 26th and the 27th count this week; the 24th to the 27th this
    // month.
    assert!(shows_exactly(&harness, &t.adherence_of(2, 2)));
    assert!(shows_exactly(&harness, "100\u{a0}%"));
    assert!(shows_exactly(&harness, &t.adherence_of(3, 4)));
    assert!(shows_exactly(&harness, "75\u{a0}%"));
    assert_eq!(history_view::week_start(&t), 1);
    Ok(())
}

#[test]
fn the_routine_editor_opens_its_history_and_back_leads_where_it_came_from() -> TestResult {
    let (store, pills) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::AllRoutines);
    app.open_routine(&pills.id);
    let mut harness = window(app);
    assert!(harness.state().routine_editor().is_some());
    harness.get_by_label("History").click();
    harness.run();
    assert_eq!(
        *harness.state().selection(),
        Selection::History(pills.id.clone())
    );
    assert!(shows_exactly(&harness, "2 of 3 Done"));
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(*harness.state().selection(), Selection::AllRoutines);
    Ok(())
}

#[test]
fn a_reminders_editor_has_no_history() -> TestResult {
    let (store, _) = store()?;
    let reminder = Routines::new(&store)
        .all()?
        .into_iter()
        .find(Routine::is_reminder)
        .ok_or("the Reminder")?;
    let mut app = App::new(store, "en");
    app.open_routine(&reminder.id);
    let harness = window(app);
    assert!(harness.state().routine_editor().is_some());
    assert!(harness.query_by_label("History").is_none());
    Ok(())
}

#[test]
fn the_settings_switch_hides_the_streak_and_the_adherence_stays() -> TestResult {
    let (store, pills) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Settings);
    let mut harness = window(app);
    harness
        .get_by_label("Show Streaks on this computer")
        .click_accesskit();
    harness.run();
    assert!(!DeviceSettings::new(harness.state().store()).streaks_on()?);
    harness
        .state_mut()
        .select(Selection::History(pills.id.clone()));
    harness.run();
    assert!(!shows_exactly(&harness, "Streak"));
    assert!(!shows(&harness, "all Done"));
    assert!(shows_exactly(&harness, "Adherence"));
    assert!(shows_exactly(&harness, "2 of 3 Done"));
    Ok(())
}

#[test]
fn a_history_of_no_routine_goes_back() -> TestResult {
    let (store, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::AllRoutines);
    app.select(Selection::History("no such Routine".to_string()));
    let harness = window(app);
    assert_eq!(*harness.state().selection(), Selection::AllRoutines);
    Ok(())
}
