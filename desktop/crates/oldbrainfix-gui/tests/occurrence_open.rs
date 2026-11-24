//! A click on an Occurrence opens its Note (#270), driven headless by
//! egui_kittest: a click on an Agenda row's text, or on a Checklist
//! Routine's Line's words, opens the Note dialog of its Note with its
//! Due line, not the Routine editor; "Edit schedule" in the row's ⋮ and
//! right-click opens the Routine editor; a Note in the trash leaves the
//! Routine editor to open; Done in the row's menu checks it off. A click
//! on an Occurrence's notification opens the Agenda with the Occurrence
//! marked and scrolled into view where the Agenda lists it, today, a
//! later day or the Missed Summary, else with "Already done" or "Skipped"
//! at the top (obf-1039), keeping a Routine editor open over it; the
//! Catch-up Notification's opens the Agenda; each click goes into the
//! trace with where it went.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::schedule::Frequency;
use oldbrainfix_core::{
    Alarms, Checklists, ClockTime, Day, DeviceSettings, Notes, Occurrence, Recurrence, Routines,
    Schedule, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::alerts::{
    AgendaWanted, CATCH_UP_KEY, OPEN, OccurrenceWanted, Tap, occurrence_key,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// An empty store whose clock shows `at` until it is set again, a
/// millisecond on with every look.
fn store(at: &str) -> Result<(Arc<Store>, Arc<AtomicI64>), Box<dyn std::error::Error>> {
    let now = Arc::new(AtomicI64::new(parse_time(at)?.timestamp_millis()));
    let held = now.clone();
    let clock = move || {
        let ms = held.fetch_add(1, Ordering::Relaxed);
        DateTime::from_timestamp_millis(ms).unwrap_or_default()
    };
    let store = Store::in_memory(Box::new(clock), || Ok(SigningKey::from_seed([7; 32])))?;
    Ok((Arc::new(store), now))
}

/// The window over `store` on the Agenda, the click on a notification
/// coming through `wanted`.
fn window(store: &Arc<Store>, wanted: &OccurrenceWanted) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.set_occurrence_wanted(wanted.clone());
    app.select(Selection::Agenda);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1800.0))
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
    harness.query_all_by_label(label).next().is_some()
}

/// Clicks the first button labelled `label`, today's row.
fn click_first(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .next()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

/// Clicks the last button labelled `label`, a menu's over the window's.
fn click_last(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

const PLANTS: &str = "Water the plants";

/// "Water the plants" every Monday at 08:00, set up on Monday 26 October
/// 2026 at 07:00, the clock then at 08:30: due. The store, the Note's id
/// and the Routine's id.
fn plants() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture(PLANTS, None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=WEEKLY;BYDAY=MO").map_err(|e| format!("{e:?}"))?;
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(rule)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&note, &schedule)?.id;
    now.store(
        parse_time("2026-10-26T08:30:00.000Z")?.timestamp_millis(),
        Ordering::Relaxed,
    );
    Ok((store, note, routine))
}

fn slot() -> OccurrenceWanted {
    Arc::new(Mutex::new(None))
}

#[test]
fn a_click_on_an_agenda_row_opens_its_note_with_the_due_line() -> TestResult {
    let (store, note, _) = plants()?;
    let mut harness = window(&store, &slot());
    click_first(&mut harness, PLANTS)?;
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    assert!(harness.state().note_open());
    assert!(harness.state().routine_editor().is_none());
    // The Due line answers it at the top.
    assert!(shows(&harness, "Due 08:00"));
    Ok(())
}

#[test]
fn edit_schedule_in_the_rows_menu_opens_the_routine_editor() -> TestResult {
    let (store, _, routine) = plants()?;
    let mut harness = window(&store, &slot());
    click_last(&mut harness, &format!("More for {PLANTS}"))?;
    // After the answers, before History and Share.
    let labels: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    let at = |label: &str| labels.iter().rposition(|l| l == label);
    let schedule = at("Edit schedule").ok_or("Edit schedule")?;
    assert!(at("Tomorrow").is_some_and(|i| i < schedule));
    assert!(at("History").is_some_and(|i| schedule < i));
    click_last(&mut harness, "Edit schedule")?;
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert_eq!(editor.routine.as_deref(), Some(routine.as_str()));

    // The right-click offers it too.
    let mut harness = window(&store, &slot());
    harness
        .get_all_by_role_and_label(Role::Button, PLANTS)
        .next()
        .ok_or("the row")?
        .click_secondary();
    harness.run();
    click_last(&mut harness, "Edit schedule")?;
    assert!(harness.state().routine_editor().is_some());
    Ok(())
}

#[test]
fn a_row_whose_note_is_in_the_trash_opens_its_routine() -> TestResult {
    let (store, note, routine) = plants()?;
    Notes::new(&store).trash(&note)?;
    let mut harness = window(&store, &slot());
    click_first(&mut harness, PLANTS)?;
    assert!(!harness.state().note_open());
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert_eq!(editor.routine.as_deref(), Some(routine.as_str()));
    Ok(())
}

#[test]
fn a_checklist_routines_line_opens_its_note() -> TestResult {
    let (store, _) = store("2026-10-24T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Packing\nBath\nTent\nMap", None, None)?;
    Checklists::new(&store).make(&note.id)?;
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 24), &[], Some(daily)).ok_or("a schedule")?;
    Routines::new(&store).spawn(&note.id, &schedule)?;
    let mut harness = window(&store, &slot());
    click_first(&mut harness, "Tent")?;
    assert_eq!(harness.state().selected_note(), Some(note.id.as_str()));
    assert!(harness.state().routine_editor().is_none());
    Ok(())
}

/// Monday's 08:00 of the plants, as its notification names it.
fn monday() -> Occurrence {
    Occurrence::new(Day::new(2026, 10, 26), ClockTime::new(8, 0))
}

/// A click on the notification of `occurrence` of `routine`, as the
/// alerts hand it to the window and note it in its trace.
fn click(
    harness: &mut Harness<'_, App>,
    wanted: &OccurrenceWanted,
    (routine, occurrence): (&str, Occurrence),
) -> TestResult {
    let key = occurrence_key(routine, &occurrence);
    let at = DateTime::from_timestamp(1_793_000_000, 0).unwrap_or_default();
    let tap = Tap {
        key,
        action: OPEN.into(),
        at,
    };
    harness.state().click_trace().arrived(&tap);
    let asked = AgendaWanted::Occurrence(routine.to_string(), occurrence);
    *wanted.lock().map_err(|e| e.to_string())? = Some(asked);
    harness.run();
    Ok(())
}

/// Whether the row titled `title` is marked, as a screen reader hears it.
fn marked(harness: &Harness<'_, App>, title: &str) -> bool {
    harness
        .get_all_by_role_and_label(Role::Button, title)
        .any(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

/// Where the last click went, as the trace says.
fn went(harness: &Harness<'_, App>) -> Option<String> {
    harness.state().click_trace().clicks().last()?.went.clone()
}

#[test]
fn a_notification_click_opens_the_agenda_with_its_occurrence_marked() -> TestResult {
    // obf-1039: until 0.8 it opened the Note dialog, here over All Notes.
    let (store, _, routine) = plants()?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(!marked(&harness, PLANTS));
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    assert!(!harness.state().note_open());
    assert!(marked(&harness, PLANTS));
    assert_eq!(went(&harness).as_deref(), Some("Agenda today"));
    assert!(wanted.lock().map_err(|e| e.to_string())?.is_none());
    // Gone once the Agenda is left.
    harness.state_mut().select(Selection::All);
    harness.run();
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!marked(&harness, PLANTS));
    Ok(())
}

#[test]
fn the_marked_occurrence_is_scrolled_into_view() -> TestResult {
    let (store, _, routine) = plants()?;
    // Twenty Reminders today before the plants' 08:00, all day long.
    let times: Vec<ClockTime> = ClockTime::new(7, 0).into_iter().collect();
    for i in 0..20 {
        let note = Notes::new(&store)
            .capture(&format!("Earlier {i}"), None, None)?
            .id;
        let schedule = Schedule::new(Day::new(2026, 10, 26), &times, None).ok_or("a schedule")?;
        Routines::new(&store).spawn(&note, &schedule)?;
    }
    let wanted = slot();
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.set_occurrence_wanted(wanted.clone());
    app.select(Selection::Agenda);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    let below = |harness: &Harness<'_, App>| -> Result<bool, Box<dyn std::error::Error>> {
        let row = harness
            .get_all_by_role_and_label(Role::Button, PLANTS)
            .next()
            .ok_or("the row")?;
        let rect = row.rect();
        Ok(rect.min.y > 600.0)
    };
    assert!(below(&harness)?, "the plants lie below the window at first");
    // The click comes while another list shows, as it would.
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert!(marked(&harness, PLANTS));
    assert!(!below(&harness)?, "scrolled into view");
    Ok(())
}

#[test]
fn one_still_open_that_no_day_shows_opens_its_note_dialog() -> TestResult {
    // The plants weekly on Monday, clicked on Tuesday: not Missed until
    // next Monday, on no day of the Agenda.
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture(PLANTS, None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=WEEKLY;BYDAY=MO").map_err(|e| format!("{e:?}"))?;
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(rule)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&note, &schedule)?.id;
    now.store(
        parse_time("2026-10-27T09:00:00.000Z")?.timestamp_millis(),
        Ordering::Relaxed,
    );
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    assert!(shows_contains(&harness, "Due"));
    assert_eq!(
        went(&harness).as_deref(),
        Some("Note dialog, not on the Agenda")
    );
    Ok(())
}

fn shows_contains(harness: &Harness<'_, App>, words: &str) -> bool {
    harness.query_all_by_label_contains(words).next().is_some()
}

#[test]
fn an_occurrence_of_tomorrow_is_marked_under_tomorrow() -> TestResult {
    let (store, _) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture("Feed the cat", None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(18, 0).into_iter().collect();
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(daily)).ok_or("a schedule")?;
    let cat = Routines::new(&store).spawn(&note, &schedule)?.id;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    let tuesday = Occurrence::new(Day::new(2026, 10, 27), ClockTime::new(18, 0));
    click(&mut harness, &wanted, (&cat, tuesday))?;
    assert_eq!(went(&harness).as_deref(), Some("Agenda tomorrow"));
    // Today's row of the cat is not the one marked: the last is tomorrow's.
    let marks: Vec<bool> = harness
        .get_all_by_role_and_label(Role::Button, "Feed the cat")
        .map(|n| n.accesskit_node().toggled() == Some(Toggled::True))
        .collect();
    assert_eq!(marks.first(), Some(&false));
    assert_eq!(marks.get(1), Some(&true));
    Ok(())
}

/// The plants every day at 08:00, on Tuesday at 09:00: Monday's Missed,
/// as Tuesday's began.
fn plants_missed() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let (store, now) = store("2026-10-26T07:00:00.000Z")?;
    let note = Notes::new(&store).capture(PLANTS, None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(daily)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&note, &schedule)?.id;
    now.store(
        parse_time("2026-10-27T09:00:00.000Z")?.timestamp_millis(),
        Ordering::Relaxed,
    );
    Ok((store, routine))
}

#[test]
fn a_missed_occurrence_is_marked_in_the_missed_summary() -> TestResult {
    let (store, routine) = plants_missed()?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert_eq!(went(&harness).as_deref(), Some("Agenda Missed"));
    assert!(!shows(&harness, "Already done"));
    Ok(())
}

#[test]
fn one_done_or_skipped_meanwhile_says_so_at_the_top() -> TestResult {
    let (store, routine) = plants_missed()?;
    let at = parse_time("2026-10-27T08:59:00.000Z")?;
    Alarms::new(&store).apply_done(&routine, &monday().key(), at)?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    assert!(shows(&harness, "Already done"));
    assert_eq!(went(&harness).as_deref(), Some("Agenda, already done"));

    let (store, routine) = plants_missed()?;
    Alarms::new(&store).apply_skip(&routine, &monday().key(), at)?;
    let mut harness = window(&store, &wanted);
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert!(shows(&harness, "Skipped"));
    assert_eq!(went(&harness).as_deref(), Some("Agenda, skipped"));
    Ok(())
}

#[test]
fn one_done_today_is_marked_where_it_shows_struck_and_said_where_hidden() -> TestResult {
    let (store, _, routine) = plants()?;
    let at = parse_time("2026-10-26T08:10:00.000Z")?;
    Alarms::new(&store).apply_done(&routine, &monday().key(), at)?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert!(marked(&harness, PLANTS));
    assert!(!shows(&harness, "Already done"));
    // Where this Device hides what is checked (#217) it is listed nowhere.
    DeviceSettings::new(&store).set_agenda_hides_checked(true)?;
    let mut harness = window(&store, &wanted);
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert!(shows(&harness, "Already done"));
    Ok(())
}

#[test]
fn done_in_the_rows_menu_checks_it_off() -> TestResult {
    let (store, _, routine) = plants()?;
    let mut harness = window(&store, &slot());
    click_last(&mut harness, &format!("More for {PLANTS}"))?;
    let labels: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    let at = |label: &str| labels.iter().rposition(|l| l == label);
    let done = at("Done").ok_or("Done in the menu")?;
    assert!(at("10 min").is_some_and(|i| done < i));
    click_last(&mut harness, "Done")?;
    let held = Routines::new(&store).by_id(&routine)?.ok_or("the plants")?;
    assert!(held.done_at(&monday()).is_some());
    // Done, the menu offers it no more.
    click_last(&mut harness, &format!("More for {PLANTS}"))?;
    let labels: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert!(!labels.iter().any(|l| l == "Done"));
    Ok(())
}

#[test]
fn a_notification_keeps_an_open_routine_editor_over_the_agenda() -> TestResult {
    let (store, _, routine) = plants()?;
    let other = Notes::new(&store).capture("Feed the cat", None, None)?.id;
    let times: Vec<ClockTime> = ClockTime::new(18, 0).into_iter().collect();
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 1), &times, Some(daily)).ok_or("a schedule")?;
    let cat = Routines::new(&store).spawn(&other, &schedule)?.id;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    harness.state_mut().open_routine(&cat);
    harness.run();
    click(&mut harness, &wanted, (&routine, monday()))?;
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    let editor = harness.state().routine_editor().ok_or("the editor kept")?;
    assert_eq!(editor.routine.as_deref(), Some(cat.as_str()));
    Ok(())
}

#[test]
fn a_notification_of_a_routine_gone_opens_the_agenda_alone() -> TestResult {
    let (store, _, _) = plants()?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, &wanted, ("gone", monday()))?;
    assert!(!harness.state().note_open());
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    assert!(!marked(&harness, PLANTS));
    assert_eq!(went(&harness).as_deref(), Some("Agenda, Routine gone"));
    Ok(())
}

#[test]
fn the_catch_up_notifications_click_opens_the_agenda() -> TestResult {
    let (store, _, _) = plants()?;
    let wanted = slot();
    let mut harness = window(&store, &wanted);
    harness.state_mut().select(Selection::All);
    harness.run();
    let at = DateTime::from_timestamp(1_793_000_000, 0).unwrap_or_default();
    harness.state().click_trace().arrived(&Tap {
        key: CATCH_UP_KEY.into(),
        action: OPEN.into(),
        at,
    });
    *wanted.lock().map_err(|e| e.to_string())? = Some(AgendaWanted::Agenda);
    harness.run();
    assert_eq!(*harness.state().selection(), Selection::Agenda);
    assert_eq!(went(&harness).as_deref(), Some("Agenda"));
    Ok(())
}

#[test]
fn with_the_agenda_read_on_its_thread_the_occurrence_is_placed_once_read() -> TestResult {
    let (store, _, routine) = plants()?;
    let wanted = slot();
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.set_occurrence_wanted(wanted.clone());
    app.select(Selection::All);
    let aside = store.clone();
    app.set_agenda_store(move || aside.clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    click(&mut harness, &wanted, (&routine, monday()))?;
    for _ in 0..40 {
        if went(&harness).is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
        harness.run();
    }
    assert_eq!(went(&harness).as_deref(), Some("Agenda today"));
    assert!(marked(&harness, PLANTS));
    Ok(())
}
