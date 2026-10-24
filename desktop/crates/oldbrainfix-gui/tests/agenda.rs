//! The Agenda, the Routine editor, deleting a Note with Routines and the
//! Settings, driven headless by egui_kittest over the Dart core's
//! `routines` fixture log, on a European wall clock.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    ClockTime, Day, DeviceSettings, Notes, Occurrence, Routines, SigningKey, Store, WireEntry,
    parse_time,
};
use oldbrainfix_gui::autostart::Autostart;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// The `routines` fixture log, its clock held at `now`.
fn store(now: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/routines");
    let now = parse_time(now)?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    for line in std::fs::read_to_string(dir.join("log.jsonl"))?.lines() {
        let entry: WireEntry = serde_json::from_str(line)?;
        store.insert(&entry)?;
    }
    Ok(store)
}

/// The window at `now`, in `locale`, on the Agenda.
fn harness(now: &str, locale: &str) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store(now)?, locale);
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
    Ok(harness)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The text field whose hint is `hint`.
fn field<'a>(harness: &'a Harness<'_, App>, hint: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(move |n| n.placeholder() == Some(hint))
}

/// The text field that has the keyboard.
fn typing<'a>(harness: &'a Harness<'_, App>) -> egui_kittest::Node<'a> {
    harness.get_by(|n| n.is_focused())
}

/// The one-line text field holding `value`.
fn holding<'a>(harness: &'a Harness<'_, App>, value: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(move |n| n.role() == Role::TextInput && n.value().as_deref() == Some(value))
}

/// Clicks the empty field whose hint is `hint` and types `text` into it.
fn type_into(harness: &mut Harness<'_, App>, hint: &str, text: &str) {
    field(harness, hint).click();
    harness.run();
    typing(harness).type_text(text);
    harness.run();
}

/// Clicks the one-line field holding `value` and types `text` over it.
fn type_over(harness: &mut Harness<'_, App>, value: &str, text: &str) {
    holding(harness, value).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    typing(harness).type_text(text);
    harness.run();
}

fn banner(harness: &Harness<'_, App>) -> f32 {
    harness
        .get_by_role_and_label(Role::Button, "2 Unsorted")
        .rect()
        .top()
}

/// The left pane's row `label`, left of everything else of that name.
fn left<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, label)
}

fn top(harness: &Harness<'_, App>, label: &str) -> f32 {
    harness.get_by_label(label).rect().top()
}

fn routine_of(
    app: &App,
    text: &str,
) -> Result<oldbrainfix_core::Routine, Box<dyn std::error::Error>> {
    let routines = Routines::new(app.store());
    for routine in routines.all()? {
        if routines.note(&routine)?.is_some_and(|n| n.text == text) && !routine.is_reminder() {
            return Ok(routine);
        }
    }
    Err(format!("no Routine of {text:?}").into())
}

fn at(day: Day, hour: u32) -> Occurrence {
    Occurrence::new(day, ClockTime::new(hour, 0))
}

#[test]
fn the_agenda_shows_missed_unsorted_today_and_tomorrow_in_that_order() -> TestResult {
    // Sunday noon in Europe, the day the clock went back.
    let mut harness = harness("2026-10-25T11:00:00.000Z", "en")?;
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    for label in [
        "Agenda",
        "Missed",
        "Seen",
        "2 Unsorted",
        "Today",
        "Tomorrow",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(top(&harness, "Missed") < banner(&harness));
    assert!(banner(&harness) < top(&harness, "Today"));
    assert!(top(&harness, "Today") < top(&harness, "Tomorrow"));
    // Today: the weekend's all-day row first, then the pills by time.
    let weekend = harness
        .get_by_role_and_label(Role::Button, "Water the plants")
        .rect();
    let pills: Vec<_> = harness
        .query_all_by_label("Done: 08:00 Take the pills")
        .map(|n| n.rect().top())
        .collect();
    assert_eq!(pills.len(), 2, "today and tomorrow");
    assert!(weekend.top() < pills[0]);
    assert!(pills[0] < top(&harness, "Tomorrow"));
    assert!(top(&harness, "Tomorrow") < pills[1]);

    // Tomorrow's can be checked off today.
    harness
        .query_all_by_label("Done: 08:00 Take the pills")
        .nth(1)
        .ok_or("tomorrow's pills")?
        .click();
    harness.run();
    let pills = routine_of(harness.state(), "Take the pills")?;
    let tomorrow = Day::new(2026, 10, 26);
    assert!(pills.done_at(&at(tomorrow, 8)).is_some());
    // And unchecked again.
    harness
        .query_all_by_label("Done: 08:00 Take the pills")
        .nth(1)
        .ok_or("tomorrow's pills")?
        .click();
    harness.run();
    assert!(
        routine_of(harness.state(), "Take the pills")?
            .done_at(&at(tomorrow, 8))
            .is_none()
    );

    // A Missed one is checked off late from the card, which stays.
    let missed = harness
        .query_all_by_label_contains("Done: Sat, 10/24/2026")
        .next()
        .ok_or("a Missed Occurrence")?;
    missed.click();
    harness.run();
    assert!(shows(&harness, "Missed"));

    // "Seen" clears it.
    harness.get_by_label("Seen").click();
    harness.run();
    assert!(!shows(&harness, "Missed"));
    assert!(
        DeviceSettings::new(harness.state().store())
            .missed_seen()?
            .is_some()
    );

    // The banner opens the Unsorted Notes.
    harness
        .get_by_role_and_label(Role::Button, "2 Unsorted")
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    assert!(shows(&harness, "Unsorted thought"));
    Ok(())
}

#[test]
fn the_agenda_speaks_german() -> TestResult {
    let harness = harness("2026-10-25T11:00:00.000Z", "de")?;
    for label in ["Heute", "Morgen", "Verpasst", "Gesehen", "2 unsortiert"] {
        assert!(shows(&harness, label), "{label}");
    }
    Ok(())
}

#[test]
fn a_routine_spawned_from_a_note_takes_several_times_its_renag_and_escalation() -> TestResult {
    let mut harness = harness("2026-10-25T11:00:00.000Z", "en")?;
    harness.get_by_label("All Notes").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Another thought")
        .click();
    harness.run();
    harness.get_by_label("Routine").click();
    harness.run();
    assert!(shows(&harness, "New Routine"));
    assert!(shows(&harness, "From the Note"));
    // 08:00 to start with, and 20:00 added.
    type_into(&mut harness, "HH:MM", "20:00");
    harness.get_by_label("Add a time").click();
    harness.run();
    assert!(shows(&harness, "Remove 20:00"));
    // Its own Re-nag, and Escalation.
    harness.get_by_value("Default (10 min)").click();
    harness.run();
    harness.get_by_label("5 min").click();
    harness.run();
    harness
        .get_by_label("Escalate to an urgent alarm with sound")
        .click();
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();
    assert!(harness.state().routine_editor().is_none());
    let routine = routine_of(harness.state(), "Another thought")?;
    let times: Vec<String> = routine
        .schedule
        .times
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(times, ["08:00", "20:00"]);
    assert_eq!(
        routine.schedule.rule.map(|r| r.format()).as_deref(),
        Some("FREQ=DAILY")
    );
    assert_eq!(routine.renag, Some(chrono::Duration::minutes(5)));
    assert_eq!(routine.escalation, Some(3));
    // The Note left Unsorted and lists its Routine.
    let notes = Notes::new(harness.state().store());
    assert!(
        notes
            .unsorted()?
            .iter()
            .all(|n| n.text != "Another thought")
    );
    assert!(shows(&harness, "Every day · 08:00, 20:00"));
    Ok(())
}

#[test]
fn the_advanced_editor_refuses_what_it_cannot_read() -> TestResult {
    let mut harness = harness("2026-10-25T11:00:00.000Z", "en")?;
    harness.get_by_label("New Routine or Reminder").click();
    harness.run();
    type_into(&mut harness, "What is to be done?", "Change the filter");
    harness.get_by_label("Advanced").click();
    harness.run();
    // The simple editor's daily rule carries over.
    type_over(&mut harness, "FREQ=DAILY", "FREQ=WEEKLY;BYDAY=1MO");
    assert!(shows(&harness, "Not a rule this app understands: 1MO"));
    assert!(harness.get_by_label("Save").accesskit_node().is_disabled());
    // Monthly, the first Monday reads.
    type_over(
        &mut harness,
        "FREQ=WEEKLY;BYDAY=1MO",
        "FREQ=MONTHLY;BYDAY=1MO",
    );
    assert!(!harness.get_by_label("Save").accesskit_node().is_disabled());
    harness.get_by_label("Save").click();
    harness.run();
    let routine = routine_of(harness.state(), "Change the filter")?;
    assert_eq!(
        routine.schedule.rule.map(|r| r.format()).as_deref(),
        Some("FREQ=MONTHLY;BYDAY=1MO")
    );
    Ok(())
}

#[test]
fn deleting_a_note_asks_about_its_routines_and_a_routine_goes_to_the_trash() -> TestResult {
    let mut harness = harness("2026-10-25T11:00:00.000Z", "en")?;
    harness.get_by_label("All Notes").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Walk the dog")
        .click();
    harness.run();
    harness.get_by_label("Move to trash").click();
    harness.run();
    assert!(shows(&harness, "Also delete the linked Routine?"));
    harness.get_by_label("Yes").click();
    harness.run();
    let walk = find_note(harness.state(), "Walk the dog")?;
    let routines = Routines::new(harness.state().store());
    assert!(routines.all()?.iter().all(|r| r.note_id != walk));
    // Restored, the Note brings its Routine back.
    left(&harness, "Trash").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Walk the dog")
        .click();
    harness.run();
    harness.get_by_label("Restore").click();
    harness.run();
    assert!(routine_of(harness.state(), "Walk the dog").is_ok());

    // A Routine on its own: deleted from its editor, after asking, and
    // restored from the trash.
    harness.get_by_label("Agenda").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Night meds")
        .click();
    harness.run();
    assert!(shows(&harness, "Routine"));
    harness.get_by_label("Delete").click();
    harness.run();
    assert!(shows(
        &harness,
        "Delete this Routine? It stays in the trash for 30 days. Its Note stays."
    ));
    harness
        .query_all_by_label("Delete")
        .last()
        .ok_or("the dialog's Delete")?
        .click();
    harness.run();
    assert!(routine_of(harness.state(), "Night meds").is_err());
    left(&harness, "Trash").click();
    harness.run();
    assert!(shows(&harness, "Routines and Reminders"));
    harness
        .query_all_by_label("Restore")
        .next()
        .ok_or("a Restore")?
        .click();
    harness.run();
    assert!(routine_of(harness.state(), "Night meds").is_ok());
    Ok(())
}

fn find_note(app: &App, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(app.store());
    for id in app.store().entities_of_type("note")? {
        if let Some(note) = notes.by_id(&id)?
            && note.text == text
        {
            return Ok(id);
        }
    }
    Err(format!("no Note {text:?}").into())
}

#[test]
fn the_settings_keep_this_devices_alerts_and_login_start() -> TestResult {
    let mut harness = harness("2026-10-25T11:00:00.000Z", "en")?;
    let dir = tempfile::tempdir()?;
    let start = Autostart {
        exe: PathBuf::from("/usr/bin/oldbrainfix"),
        file: dir
            .path()
            .join("autostart/io.github.paxel.oldbrainfix.desktop"),
    };
    start.set(true)?;
    harness.state_mut().set_autostart(Some(start.clone()));
    harness.get_by_label("Settings").click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Settings);
    assert!(shows(
        &harness,
        "These settings belong to this computer and are never synced."
    ));
    harness
        .get_by_label("Notifications on this computer")
        .click();
    harness.run();
    harness.get_by_label("Start at login").click();
    harness.run();
    let settings = DeviceSettings::new(harness.state().store());
    assert!(!settings.alerts_on()?);
    assert!(!settings.autostart_on()?);
    assert!(!start.is_set());
    harness.get_by_label("Start at login").click();
    harness.run();
    assert!(start.is_set());
    // Quiet Hours from 21:30.
    type_over(&mut harness, "22:00", "21:30");
    let quiet = DeviceSettings::new(harness.state().store()).quiet_hours()?;
    assert_eq!(quiet.to_string(), "21:30-08:00");
    // The Re-nag default.
    harness.get_by_value("10 min").click();
    harness.run();
    harness.get_by_label("15 min").click();
    harness.run();
    let renag = DeviceSettings::new(harness.state().store()).renag_interval()?;
    assert_eq!(renag, chrono::Duration::minutes(15));
    // Back to the Agenda.
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    Ok(())
}
