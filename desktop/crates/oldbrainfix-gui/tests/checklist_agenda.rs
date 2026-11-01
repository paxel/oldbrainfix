//! Checklists on the Agenda in the window (#143, ADR 0030): today shows
//! each Checklist without a Routine or Reminder that has open Lines as
//! one row, "Packing: 3 of 3 open", after the Shopping Lists and before
//! the timed Occurrences, oldest capture first; a click opens it; once
//! every Line is checked it is gone; "Show on the Agenda" in the card's
//! ⋮ and in the editor takes it off for this person; a Checklist with a
//! Routine has neither the row nor the switch.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    Checklists, ClockTime, Day, Notes, OwnProfile, Routines, Schedule, SigningKey, Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 07:00 on Saturday, 24 October 2026, the wall clock on UTC.
const NOW: &str = "2026-10-24T07:00:00Z";

/// The text of the Note every test packs with: three Lines, "Charger"
/// in the group of "Passport".
const PACKING: &str = "Packing\nPassport\n  Charger\nTicket";

/// The switch's words.
const SWITCH: &str = "Show on the Agenda";

/// The Agenda's message for a day with nothing due.
const NOTHING_TODAY: &str = "Nothing due today. Click + to capture a thought.";

/// A store on a clock that moves a second per read from [`NOW`], with
/// its own Profile, and the Checklist [`PACKING`]: the store and the
/// Note's id.
fn packing() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339(NOW)?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([6; 32])))?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let note = Notes::new(&store).capture(PACKING, None, None)?;
    Checklists::new(&store).make(&note.id)?;
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English, its days on UTC, showing
/// `selection` with the Note `note` open.
fn window(store: &Arc<Store>, selection: Selection, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(|utc: DateTime<Utc>| utc.naive_utc()));
    app.select(selection);
    if let Some(note) = note {
        app.select_note(note);
    }
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

/// Reads the Agenda again, as coming back to it does.
fn reload_agenda(harness: &mut Harness<'_, App>) {
    harness.state_mut().select(Selection::All);
    harness.run();
    harness.state_mut().select(Selection::Agenda);
    harness.run();
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Clicks the last widget labelled `label`: a menu's over the window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Whether the switch "Show on the Agenda" shows, and on.
fn switch(harness: &Harness<'_, App>) -> Option<bool> {
    harness
        .query_all_by_role_and_label(Role::CheckBox, SWITCH)
        .last()
        .map(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

/// The top of the first widget labelled `label`.
fn top(harness: &Harness<'_, App>, label: &str) -> Result<f32, Box<dyn std::error::Error>> {
    Ok(harness
        .query_all_by_label(label)
        .next()
        .ok_or(format!("{label:?}"))?
        .rect()
        .top())
}

/// The id of the Line `text` of the Checklist of `note`.
fn line_id(store: &Store, note: &str, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Checklists::new(store)
        .by_note(note)?
        .ok_or("a Checklist")?
        .lines
        .iter()
        .find(|l| l.text == text)
        .ok_or(format!("{text:?}"))?
        .id
        .clone())
}

#[test]
fn a_checklist_shows_today_with_its_open_lines_and_opens() -> TestResult {
    let (store, note) = packing()?;
    let mut harness = window(&store, Selection::Agenda, None);
    assert!(shows(&harness, "Packing: 3 of 3 open"));
    assert!(!shows(&harness, NOTHING_TODAY));
    // Only today: tomorrow has nothing planned.
    let rows = harness
        .query_all_by_label_contains("Packing:")
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .count();
    assert_eq!(rows, 1);
    // The parent counts as it shows: checking its group checks it.
    Checklists::new(&store).check(&line_id(&store, &note, "Charger")?)?;
    reload_agenda(&mut harness);
    assert!(shows(&harness, "Packing: 1 of 3 open"));
    harness
        .get_by_role_and_label(Role::Button, "Packing: 1 of 3 open")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn every_line_checked_takes_it_off_until_one_is_unchecked() -> TestResult {
    let (store, note) = packing()?;
    let lists = Checklists::new(&store);
    lists.check(&line_id(&store, &note, "Charger")?)?;
    lists.check(&line_id(&store, &note, "Ticket")?)?;
    let mut harness = window(&store, Selection::Agenda, None);
    assert!(!shows(&harness, "Packing: 0 of 3 open"));
    assert!(shows(&harness, NOTHING_TODAY));
    lists.uncheck(&line_id(&store, &note, "Ticket")?)?;
    reload_agenda(&mut harness);
    assert!(shows(&harness, "Packing: 1 of 3 open"));
    Ok(())
}

#[test]
fn the_rows_go_oldest_capture_first_between_all_day_and_timed() -> TestResult {
    let (store, _) = packing()?;
    let later = Notes::new(&store).capture("Errands\nPost office", None, None)?;
    Checklists::new(&store).make(&later.id)?;
    let day = Day::new(2026, 10, 24);
    let nine: Vec<ClockTime> = ClockTime::new(9, 0).into_iter().collect();
    let routines = Routines::new(&store);
    routines.create(
        "Water the plants",
        &Schedule::new(day, &[], None).ok_or("all day")?,
    )?;
    routines.create(
        "Call Dana",
        &Schedule::new(day, &nine, None).ok_or("at nine")?,
    )?;
    let harness = window(&store, Selection::Agenda, None);
    let packing = top(&harness, "Packing: 3 of 3 open")?;
    let errands = top(&harness, "Errands: 1 of 1 open")?;
    assert!(top(&harness, "Water the plants")? < packing);
    assert!(packing < errands);
    assert!(errands < top(&harness, "Call Dana")?);
    Ok(())
}

#[test]
fn the_cards_menu_switches_it_off_the_agenda_and_on_again() -> TestResult {
    let (store, note) = packing()?;
    // A Line checked, so "Uncheck all" shows after the switch.
    Checklists::new(&store).check(&line_id(&store, &note, "Ticket")?)?;
    let mut harness = window(&store, Selection::All, None);
    click(&mut harness, "More: Packing")?;
    assert_eq!(switch(&harness), Some(true));
    let below = top(&harness, "Uncheck all")?;
    let switch_top = harness
        .query_all_by_role_and_label(Role::CheckBox, SWITCH)
        .last()
        .ok_or("the switch")?
        .rect()
        .top();
    assert!(switch_top < below);
    click(&mut harness, SWITCH)?;
    assert!(!Checklists::new(&store).shows_on_agenda(&note)?);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!shows(&harness, "Packing: 2 of 3 open"));
    assert!(shows(&harness, NOTHING_TODAY));
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Packing")?;
    assert_eq!(switch(&harness), Some(false));
    click(&mut harness, SWITCH)?;
    assert!(Checklists::new(&store).shows_on_agenda(&note)?);
    Ok(())
}

#[test]
fn the_editor_switches_it_off_the_agenda() -> TestResult {
    let (store, note) = packing()?;
    let mut harness = window(&store, Selection::All, Some(&note));
    // The Note dialog's badge for it (#160), its switch in its editor.
    assert!(harness.query_by_label("Show on the Agenda: On").is_some());
    click(&mut harness, "Show on the Agenda: On")?;
    assert_eq!(switch(&harness), Some(true));
    click(&mut harness, SWITCH)?;
    assert!(!Checklists::new(&store).shows_on_agenda(&note)?);
    assert_eq!(switch(&harness), Some(false));
    click(&mut harness, "Close")?;
    assert!(harness.query_by_label("Show on the Agenda: Off").is_some());
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!shows(&harness, "Packing: 3 of 3 open"));
    Ok(())
}

#[test]
fn a_checklist_with_a_routine_has_no_row_and_no_switch() -> TestResult {
    let (store, note) = packing()?;
    let schedule = Schedule::new(Day::new(2026, 10, 24), &[], None).ok_or("a schedule")?;
    Routines::new(&store).spawn(&note, &schedule)?;
    let mut harness = window(&store, Selection::Agenda, None);
    assert!(!shows(&harness, "Packing: 3 of 3 open"));
    // It shows as its Routine's Occurrence instead.
    assert!(shows(&harness, "Packing"));
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Packing")?;
    assert_eq!(switch(&harness), None);
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.state_mut().select_note(&note);
    harness.run();
    assert_eq!(switch(&harness), None);
    assert!(
        harness
            .query_all_by_label_contains("Show on the Agenda")
            .next()
            .is_none()
    );
    Ok(())
}

#[test]
fn a_plain_note_has_no_switch() -> TestResult {
    let (store, _) = packing()?;
    let plain = Notes::new(&store).capture("Call the bank", None, None)?;
    let mut harness = window(&store, Selection::All, Some(&plain.id));
    assert_eq!(switch(&harness), None);
    click(&mut harness, "More: Call the bank")?;
    assert_eq!(switch(&harness), None);
    Ok(())
}
