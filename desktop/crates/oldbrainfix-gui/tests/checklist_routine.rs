//! A Checklist with a Routine in the window (#121, ADR 0030): each
//! Occurrence's row on the Agenda lists the Lines as checked for it,
//! checkable there; the last Line checked makes it Done and keeps its
//! box checked; it is still checked off by hand; the card and the editor
//! check for the Occurrence the Checklist is on now.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::schedule::Frequency;
use oldbrainfix_core::{
    Checklists, Day, Notes, Occurrence, OwnProfile, Recurrence, RoutineOccurrence, Routines,
    Schedule, SigningKey, Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 07:00 on Saturday, 24 October 2026, the wall clock on UTC.
const NOW: &str = "2026-10-24T07:00:00Z";

/// A store on a clock that moves a second per read from [`NOW`], with
/// its own Profile; the Note "Packing" with the Lines Bath, Tent and Map
/// and a daily all-day Routine from it starting today: the store, the
/// Note's id and the Routine's id.
fn packing() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339(NOW)?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([5; 32])))?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let note = Notes::new(&store).capture("Packing\nBath\nTent\nMap", None, None)?;
    Checklists::new(&store).make(&note.id)?;
    let daily = Recurrence::every(Frequency::Daily, 1);
    let schedule = Schedule::new(Day::new(2026, 10, 24), &[], Some(daily)).ok_or("a schedule")?;
    let routine = Routines::new(&store).spawn(&note.id, &schedule)?;
    Ok((Arc::new(store), note.id, routine.id))
}

/// The window over `store` in English on the Agenda, its days on UTC.
fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(|utc: DateTime<Utc>| utc.naive_utc()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 3200.0))
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

fn today() -> Occurrence {
    Occurrence::new(Day::new(2026, 10, 24), None)
}

fn tomorrow() -> Occurrence {
    Occurrence::new(Day::new(2026, 10, 25), None)
}

/// The checkboxes labelled `label` from the top of the window down.
fn boxes<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Vec<egui_kittest::Node<'a>> {
    let mut all: Vec<_> = harness
        .query_all_by_role_and_label(Role::CheckBox, label)
        .collect();
    all.sort_by(|a, b| a.rect().top().total_cmp(&b.rect().top()));
    all
}

/// The box of the Line `text` on the Agenda's `nth` row from the top:
/// 0 today's, 1 tomorrow's.
fn line_box<'a>(
    harness: &'a Harness<'_, App>,
    text: &'a str,
    nth: usize,
) -> Result<egui_kittest::Node<'a>, Box<dyn std::error::Error>> {
    Ok(boxes(harness, text)
        .into_iter()
        .nth(nth)
        .ok_or(format!("{text:?} on row {nth}"))?)
}

fn is_checked(node: &egui_kittest::Node<'_>) -> bool {
    node.accesskit_node().toggled() == Some(Toggled::True)
}

/// Whether the Line `text` on the `nth` row shows checked.
fn line_checked(
    harness: &Harness<'_, App>,
    text: &str,
    nth: usize,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(is_checked(&line_box(harness, text, nth)?))
}

/// Clicks the box of the Line `text` on the `nth` row.
fn tick(harness: &mut Harness<'_, App>, text: &str, nth: usize) -> TestResult {
    line_box(harness, text, nth)?.click_accesskit();
    harness.run();
    Ok(())
}

/// The box that checks today's Occurrence off.
fn row_box<'a>(
    harness: &'a Harness<'_, App>,
) -> Result<egui_kittest::Node<'a>, Box<dyn std::error::Error>> {
    let mut all: Vec<_> = harness
        .query_all_by(|n| {
            n.role() == Role::CheckBox
                && n.label()
                    .is_some_and(|l| l.starts_with("Done: ") && l.contains("Packing"))
        })
        .collect();
    all.sort_by(|a, b| a.rect().top().total_cmp(&b.rect().top()));
    Ok(all.into_iter().next().ok_or("the row's box")?)
}

fn done(harness: &Harness<'_, App>, routine: &str, occurrence: &Occurrence) -> bool {
    Routines::new(harness.state().store())
        .by_id(routine)
        .ok()
        .flatten()
        .is_some_and(|r| r.done_at(occurrence).is_some())
}

/// How many Lines are checked for `occurrence` of `routine`.
fn checked_for(
    harness: &Harness<'_, App>,
    routine: &str,
    occurrence: Occurrence,
) -> Result<usize, Box<dyn std::error::Error>> {
    let on = RoutineOccurrence {
        routine_id: routine.to_string(),
        occurrence,
    };
    Ok(Checklists::new(harness.state().store())
        .for_occurrence(&on)?
        .ok_or("a Checklist")?
        .checked_count())
}

/// How many of the Note's own Lines are checked.
fn own_checked(
    harness: &Harness<'_, App>,
    note: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(Checklists::new(harness.state().store())
        .by_note(note)?
        .ok_or("a Checklist")?
        .checked_count())
}

#[test]
fn an_occurrence_row_lists_its_lines_and_a_check_counts_for_it_only() -> TestResult {
    let (store, note, routine) = packing()?;
    let mut harness = window(&store);
    for text in ["Bath", "Tent", "Map"] {
        // Today's row, tomorrow's and the later days' each list them.
        assert!(boxes(&harness, text).len() >= 2, "{text}");
    }
    tick(&mut harness, "Bath", 0)?;
    assert!(line_checked(&harness, "Bath", 0)?);
    assert!(!line_checked(&harness, "Bath", 1)?);
    assert_eq!(checked_for(&harness, &routine, today())?, 1);
    // The Note's own Lines stay as they were.
    assert_eq!(own_checked(&harness, &note)?, 0);

    // Tomorrow's row is checked from today, for tomorrow.
    tick(&mut harness, "Tent", 1)?;
    assert!(line_checked(&harness, "Tent", 1)?);
    assert!(!line_checked(&harness, "Tent", 0)?);
    assert_eq!(checked_for(&harness, &routine, tomorrow())?, 1);
    Ok(())
}

#[test]
fn the_last_line_checked_marks_the_row_done_and_keeps_its_box_checked() -> TestResult {
    let (store, _, routine) = packing()?;
    let mut harness = window(&store);
    tick(&mut harness, "Bath", 0)?;
    tick(&mut harness, "Tent", 0)?;
    assert!(!done(&harness, &routine, &today()));
    assert!(!is_checked(&row_box(&harness)?));

    tick(&mut harness, "Map", 0)?;
    assert!(done(&harness, &routine, &today()));
    let row = row_box(&harness)?;
    assert!(is_checked(&row));
    // Done from its Lines, it stays checked.
    assert!(row.accesskit_node().is_disabled());
    assert!(!done(&harness, &routine, &tomorrow()));

    tick(&mut harness, "Tent", 0)?;
    assert!(!done(&harness, &routine, &today()));
    let row = row_box(&harness)?;
    assert!(!is_checked(&row));
    assert!(!row.accesskit_node().is_disabled());
    Ok(())
}

#[test]
fn the_row_is_still_checked_off_by_hand_its_lines_unchecked() -> TestResult {
    let (store, _, routine) = packing()?;
    let mut harness = window(&store);
    row_box(&harness)?.click_accesskit();
    harness.run();
    assert!(done(&harness, &routine, &today()));
    assert!(is_checked(&row_box(&harness)?));
    assert!(!line_checked(&harness, "Bath", 0)?);

    row_box(&harness)?.click_accesskit();
    harness.run();
    assert!(!done(&harness, &routine, &today()));
    assert!(!is_checked(&row_box(&harness)?));
    Ok(())
}

#[test]
fn a_check_on_the_card_counts_for_the_occurrence_it_is_on() -> TestResult {
    let (store, note, routine) = packing()?;
    let mut harness = window(&store);
    harness.state_mut().select(Selection::All);
    harness.run();
    tick(&mut harness, "Bath", 0)?;
    assert!(line_checked(&harness, "Bath", 0)?);
    assert_eq!(checked_for(&harness, &routine, today())?, 1);
    assert_eq!(own_checked(&harness, &note)?, 0);

    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(line_checked(&harness, "Bath", 0)?);
    assert!(!line_checked(&harness, "Bath", 1)?);
    Ok(())
}

#[test]
fn the_editor_checks_and_unchecks_all_for_the_occurrence_it_is_on() -> TestResult {
    let (store, note, routine) = packing()?;
    let mut harness = window(&store);
    harness.state_mut().select(Selection::All);
    harness.state_mut().select_note(&note);
    harness.run();
    // The editor's box lies right of the card's.
    let editor = |harness: &Harness<'_, App>| -> Result<bool, Box<dyn std::error::Error>> {
        let node = boxes(harness, "Bath")
            .into_iter()
            .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
            .ok_or("the editor's Bath")?;
        Ok(is_checked(&node))
    };
    boxes(&harness, "Bath")
        .into_iter()
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .ok_or("the editor's Bath")?
        .click_accesskit();
    harness.run();
    assert!(editor(&harness)?);
    assert_eq!(checked_for(&harness, &routine, today())?, 1);
    assert_eq!(own_checked(&harness, &note)?, 0);

    harness
        .get_by_role_and_label(Role::Button, "Uncheck all")
        .click_accesskit();
    harness.run();
    assert!(!editor(&harness)?);
    assert_eq!(checked_for(&harness, &routine, today())?, 0);
    Ok(())
}
