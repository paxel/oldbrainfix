//! The All Routines and Reminders page, its Export and Share in an
//! Occurrence's menu (#115), driven headless by egui_kittest on a
//! European wall clock. The desktop saves the calendar files where the
//! save dialog says; the tests hand in the path.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::ics::routines_ics;
use oldbrainfix_core::routines::routines_by_name;
use oldbrainfix_core::{
    ClockTime, Day, Recurrence, Routine, Routines, Schedule, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::schedule_text::schedule_text;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// Wednesday noon in Europe.
const NOW: &str = "2026-10-28T11:00:00.000Z";

fn empty_store() -> Result<Store, Box<dyn std::error::Error>> {
    let now = parse_time(NOW)?;
    Ok(Store::in_memory(Box::new(move || now), || {
        Ok(SigningKey::from_seed([9; 32]))
    })?)
}

fn schedule(start: Day, time: Option<(u32, u32)>, rule: Option<&str>) -> Result<Schedule, String> {
    let times: Vec<ClockTime> = time
        .and_then(|(h, m)| ClockTime::new(h, m))
        .into_iter()
        .collect();
    let rule = match rule {
        Some(rule) => Some(Recurrence::parse(rule).map_err(|e| format!("{e:?}"))?),
        None => None,
    };
    Schedule::new(start, &times, rule).ok_or_else(|| "a schedule".to_string())
}

/// A store with a daily Routine, a weekly one that ends, a Reminder
/// tomorrow and a yearly birthday months away, beyond the Agenda's week.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let store = empty_store()?;
    let routines = Routines::new(&store);
    let daily = schedule(Day::new(2026, 10, 1), Some((8, 0)), Some("FREQ=DAILY"))?;
    routines.create("Water the plants", &daily)?;
    let course = schedule(
        Day::new(2026, 10, 28),
        Some((18, 0)),
        Some("FREQ=WEEKLY;BYDAY=WE;UNTIL=20261216"),
    )?;
    routines.create("Course", &course)?;
    routines.create(
        "Dentist",
        &schedule(Day::new(2026, 10, 29), Some((9, 0)), None)?,
    )?;
    let yearly = schedule(Day::new(2027, 3, 14), None, Some("FREQ=YEARLY"))?;
    routines.create("birthday of Ada", &yearly)?;
    Ok(store)
}

/// The window over `app`, on the Agenda unless `app` says otherwise.
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

/// `app`, saving calendar files at `path` and checking the dialog.
fn saving_to(mut app: App, path: &Path) -> App {
    let path = path.to_path_buf();
    app.set_calendar_saver(Box::new(move |title, name| {
        assert_eq!(title, "Save the calendar file");
        assert_eq!(name, "oldbrainfix.ics");
        Some(path.clone())
    }));
    app
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The cards labelled `label`: a Routine on its page.
fn cards<'a>(
    harness: &'a Harness<'_, App>,
    label: &'a str,
) -> impl Iterator<Item = egui_kittest::Node<'a>> {
    harness.query_all_by(move |n| n.role() == Role::Button && n.label().as_deref() == Some(label))
}

fn top(harness: &Harness<'_, App>, label: &str) -> f32 {
    cards(harness, label)
        .next()
        .map_or(f32::NAN, |n| n.rect().top())
}

fn routine(store: &Store, text: &str) -> Result<Routine, Box<dyn std::error::Error>> {
    let routines = Routines::new(store);
    for routine in routines.all()? {
        if routines.note(&routine)?.is_some_and(|n| n.text == text) {
            return Ok(routine);
        }
    }
    Err(format!("no Routine {text}").into())
}

/// The calendar file the phone writes of `list` at the store's now.
fn expected_ics(store: &Store, list: &[Routine]) -> Result<String, Box<dyn std::error::Error>> {
    let routines = Routines::new(store);
    let mut texts = Vec::new();
    for routine in list {
        let note = routines.note(routine)?.ok_or("a Note")?;
        texts.push((routine.id.clone(), note.text));
    }
    let text_of = |r: &Routine| {
        texts
            .iter()
            .find(|(id, _)| *id == r.id)
            .map(|(_, text)| text.clone())
            .unwrap_or_default()
    };
    Ok(routines_ics(list, text_of, store.now()))
}

#[test]
fn the_agenda_bar_opens_every_routine_and_reminder_alphabetically_also_a_yearly_one() -> TestResult
{
    let store = store()?;
    let birthday = routine(&store, "birthday of Ada")?;
    let mut harness = window(App::new(store, "en"));
    // Months away, the birthday is not on the Agenda's week.
    assert!(!shows(&harness, "birthday of Ada"));
    harness
        .get_by_label("All Routines and Reminders")
        .click_accesskit();
    harness.run();
    assert_eq!(*harness.state().selection(), Selection::AllRoutines);
    let names = ["birthday of Ada", "Course", "Dentist", "Water the plants"];
    for pair in names.windows(2) {
        assert!(top(&harness, pair[0]) < top(&harness, pair[1]), "{pair:?}");
    }
    // Each once, with its schedule under it.
    for name in names {
        assert_eq!(cards(&harness, name).count(), 1, "{name}");
    }
    let yearly = schedule_text(harness.state().t(), &birthday.schedule);
    assert!(shows(&harness, &yearly), "{yearly}");

    // Back to the Agenda with Alt+Left.
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    Ok(())
}

#[test]
fn a_click_on_the_page_opens_the_routine_editor() -> TestResult {
    let store = store()?;
    let birthday = routine(&store, "birthday of Ada")?;
    let mut app = App::new(store, "en");
    app.select(Selection::AllRoutines);
    let mut harness = window(app);
    assert!(harness.state().routine_editor().is_none());
    cards(&harness, "birthday of Ada")
        .next()
        .ok_or("its card")?
        .click();
    harness.run();
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert_eq!(editor.schedule(), Some(&birthday.schedule));
    assert_eq!(*harness.state().selection(), Selection::AllRoutines);
    Ok(())
}

#[test]
fn export_saves_one_calendar_file_of_all_of_them_in_the_pages_order() -> TestResult {
    let store = store()?;
    let routines = Routines::new(&store);
    let mut names = Vec::new();
    for r in routines.all()? {
        let note = routines.note(&r)?.ok_or("a Note")?;
        names.push((r.id.clone(), note.text));
    }
    let sorted = routines_by_name(routines.all()?, |r| {
        names
            .iter()
            .find(|(id, _)| *id == r.id)
            .map(|(_, t)| t.clone())
            .unwrap_or_default()
    });
    let expected = expected_ics(&store, &sorted)?;
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("all");
    let mut app = saving_to(App::new(store, "en"), &saved);
    app.select(Selection::AllRoutines);
    let mut harness = window(app);

    harness.get_by_label("Export").click_accesskit();
    harness.run();
    assert!(shows(&harness, "The calendar file is saved."));
    let written = std::fs::read_to_string(saved.with_extension("ics"))?;
    assert_eq!(written, expected);
    assert_eq!(written.matches("BEGIN:VEVENT").count(), 4);
    assert!(written.contains("RRULE:FREQ=YEARLY"));
    Ok(())
}

#[test]
fn a_calendar_file_that_could_not_be_saved_says_so_and_the_click_says_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    let nowhere = dir.path().join("gone").join("all.ics");
    let mut app = saving_to(App::new(store()?, "en"), &nowhere);
    app.select(Selection::AllRoutines);
    let mut harness = window(app);

    harness.get_by_label("Export").click_accesskit();
    harness.run();
    assert!(!nowhere.exists());
    // Only that it failed, in its own words: no error text, no path.
    assert_eq!(
        harness.state().notice(),
        Some("The calendar file was not saved.")
    );
    assert!(!shows(&harness, &dir.path().to_string_lossy()));
    harness
        .get_by_label("The calendar file was not saved.")
        .click();
    harness.run();
    assert_eq!(harness.state().notice(), None);
    assert!(shows(&harness, "Calendar file not saved"));
    assert!(shows(
        &harness,
        "The calendar file could not be written where you chose."
    ));
    assert!(shows(
        &harness,
        "Nothing was lost; choose another folder and save it again."
    ));
    assert!(shows(&harness, "calendar file:"));
    harness
        .get_all_by_label("Close")
        .last()
        .ok_or("Close")?
        .click_accesskit();
    harness.run();
    assert!(!shows(&harness, "Calendar file not saved"));
    Ok(())
}

#[test]
fn share_in_an_occurrences_menu_saves_its_routine_as_a_series_or_its_reminder() -> TestResult {
    let store = store()?;
    let course = routine(&store, "Course")?;
    let dentist = routine(&store, "Dentist")?;
    let series = expected_ics(&store, std::slice::from_ref(&course))?;
    let reminder = expected_ics(&store, std::slice::from_ref(&dentist))?;
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("shared.ics");
    let mut harness = window(saving_to(App::new(store, "en"), &saved));

    // The row's ⋮.
    harness.get_by_label("More for Course").click();
    harness.run();
    harness.get_by_label("Share").click();
    harness.run();
    assert!(shows(&harness, "The calendar file is saved."));
    let written = std::fs::read_to_string(&saved)?;
    assert_eq!(written, series);
    assert_eq!(written.matches("BEGIN:VEVENT").count(), 1);
    assert!(written.contains("UNTIL=20261216"), "{written}");
    assert!(!written.contains("Dentist"));

    // A right-click on the row opens the same menu.
    harness.get_by_label("Dentist").click_secondary();
    harness.run();
    harness.get_by_label("Share").click();
    harness.run();
    let written = std::fs::read_to_string(&saved)?;
    assert_eq!(written, reminder);
    assert_eq!(written.matches("BEGIN:VEVENT").count(), 1);
    assert!(!written.contains("RRULE"));
    assert!(written.contains("Dentist"));
    Ok(())
}

#[test]
fn backing_out_of_the_save_dialog_writes_nothing() -> TestResult {
    let mut app = App::new(store()?, "en");
    app.set_calendar_saver(Box::new(|_, _| None));
    let mut harness = window(app);
    harness.get_by_label("More for Course").click();
    harness.run();
    harness.get_by_label("Share").click();
    harness.run();
    assert!(!shows(&harness, "The calendar file is saved."));
    Ok(())
}

#[test]
fn without_any_the_page_says_so_and_has_nothing_to_export() -> TestResult {
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut app = App::new(empty_store()?, "de");
    {
        let called = called.clone();
        app.set_calendar_saver(Box::new(move |_, _| {
            called.store(true, std::sync::atomic::Ordering::SeqCst);
            None::<PathBuf>
        }));
    }
    app.select(Selection::AllRoutines);
    let mut harness = window(app);
    assert!(shows(&harness, "Alle Routinen und Erinnerungen"));
    let empty = harness.state().t().all_routines_empty().to_string();
    assert!(shows(&harness, &empty));
    harness.get_by_label("Exportieren").click_accesskit();
    harness.run();
    assert!(!called.load(std::sync::atomic::Ordering::SeqCst));
    Ok(())
}
