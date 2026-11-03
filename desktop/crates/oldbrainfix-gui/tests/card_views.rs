//! Every kind's compact and detailed view (#189), driven headless by
//! egui_kittest: the list's switch sets the view of all cards, the view
//! button on a card switches that one, kept on this Device until the
//! switch changes again; a click still only selects (#148). A compact
//! card shows no Capture Context; Edit and ⋮ stay top right.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    Checklists, Day, Notes, Recurrence, Schedule, SessionRule, Sessions, ShoppingLists, SigningKey,
    Store, Tallies, parse_time,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PLAIN: &str = "Call the plumber\nabout the tap\nand the boiler\nnot the roof";

/// The wall clock on UTC.
fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// An empty store on Monday, 26 October 2026, its clock moving a second
/// on every look.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = parse_time("2026-10-26T09:00:00.000Z")?;
    let tick = AtomicI64::new(0);
    let clock = move || start + Duration::seconds(tick.fetch_add(1, Ordering::Relaxed));
    let store = Store::in_memory(Box::new(clock), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

/// The window over `store` on All Notes.
fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
        // A frame per event: two clicks this quick make a double-click.
        .with_step_dt(0.02)
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

/// All Notes, its cards compact.
fn compact(store: &Arc<Store>) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    store.set_local_setting("compactNotes:allNotes", "true")?;
    Ok(window(store))
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The card of the Note with `text`.
fn card<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, text)
}

/// The button `label` on the card of the Note with `text`.
fn on_card<'a>(
    harness: &'a Harness<'_, App>,
    text: &str,
    label: &'a str,
) -> Option<egui_kittest::Node<'a>> {
    let card = card(harness, text).rect();
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .find(|n| card.contains_rect(n.rect()))
}

/// Whether the card of the Note with `text` is in the compact view.
fn compact_card(harness: &Harness<'_, App>, text: &str) -> bool {
    on_card(harness, text, "Show detailed view").is_some()
}

/// Lets more than a double-click's time pass.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// Clicks the view button of the card of the Note with `text`.
fn switch(harness: &mut Harness<'_, App>, text: &str) {
    let label = if compact_card(harness, text) {
        "Show detailed view"
    } else {
        "Show compact view"
    };
    on_card(harness, text, label)
        .unwrap_or_else(|| panic!("no {label} on {text:?}"))
        .click();
    harness.run();
    pause(harness);
}

fn switched(store: &Store) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(store.local_setting("switchedNotes:allNotes")?)
}

#[test]
fn a_compact_plain_note_shows_its_first_line_and_two_more_and_no_capture_context() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture(PLAIN, None, None)?;
    let harness = compact(&store)?;
    assert!(shows(&harness, "Call the plumber"));
    assert!(shows(&harness, "about the tap\nand the boiler"));
    // Only the card's own label, its whole text, holds the fourth line.
    assert_eq!(
        harness.query_all_by_label_contains("not the roof").count(),
        1
    );
    // No time, place or meta line.
    assert!(
        !harness
            .query_all_by_label_contains("10/26/2026")
            .any(|_| true)
    );
    // Edit and ⋮ top right.
    let card = card(&harness, PLAIN).rect();
    let edit = on_card(&harness, PLAIN, "Edit").ok_or("Edit")?.rect();
    assert!(edit.right() > card.center().x && edit.top() < card.top() + 40.0);
    assert!(on_card(&harness, PLAIN, "More: Call the plumber").is_some());
    Ok(())
}

#[test]
fn a_compact_list_or_tally_shows_its_name_and_its_counts() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let wall: &oldbrainfix_core::WallClock = &utc;
    // A Checklist, 2 of 3 open, with a description.
    let trip = notes.capture("Before the trip", None, None)?.id;
    let checklists = Checklists::new(&store);
    checklists.make(&trip)?;
    let passport = checklists.add(&trip, "Passport", None, false)?;
    checklists.add(&trip, "Tickets", None, false)?;
    checklists.add(&trip, "Charger", None, false)?;
    checklists.check(&passport)?;
    notes.edit(&trip, "Before the trip\nPack early")?;
    // A Shopping List, one open.
    let groceries = notes.capture("Groceries", None, None)?.id;
    let shopping = ShoppingLists::new(&store);
    shopping.make(&groceries)?;
    let milk = shopping.add(&groceries, "Milk", wall)?;
    shopping.add(&groceries, "Bread", wall)?;
    shopping.check(&milk, wall)?;
    // A Tally, today's entries counted, yesterday's not.
    let reading = notes.capture("Reading", None, None)?.id;
    let tallies = Tallies::new(&store);
    let pages = tallies.add_field(&reading, "Pages", "pages", Some(50.0), None)?;
    let books = tallies.add_field(&reading, "Books", "books", None, None)?;
    let entry = |values: &[(&String, f64)], at| {
        let values: BTreeMap<String, f64> =
            values.iter().map(|(id, v)| ((*id).clone(), *v)).collect();
        tallies.add(&reading, &values, "", at)
    };
    entry(&[(&pages, 20.0), (&books, 1.0)], None)?;
    entry(&[(&pages, 10.0)], None)?;
    let yesterday = parse_time("2026-10-25T09:00:00.000Z")?;
    entry(&[(&pages, 99.0)], Some(yesterday))?;
    let harness = compact(&store)?;

    assert!(shows(&harness, "Before the trip"));
    assert!(shows(&harness, "2 of 3 open"));
    assert!(!shows(&harness, "Pack early"));
    assert!(!shows(&harness, "Passport"));
    assert!(shows(&harness, "Groceries"));
    assert!(shows(&harness, "1 open"));
    assert!(!shows(&harness, "Bread"));
    assert!(shows(&harness, "Reading"));
    assert!(shows(&harness, "Pages 30 / 50 pages"));
    assert!(shows(&harness, "Books 1 books"));
    Ok(())
}

/// Every day at `hour`:00 from `from`.
fn daily_at(from: Day, hour: u32) -> Result<Schedule, Box<dyn std::error::Error>> {
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let at = [oldbrainfix_core::ClockTime::new(hour, 0).ok_or("a time")?];
    Ok(Schedule::new(from, &at, Some(rule)).ok_or("a schedule")?)
}

/// The Tallies "Push-ups" (20 times) and "Wall sit" (30 times), in pool
/// order, and the plain Note "Sport" with a description.
fn sport(store: &Store) -> Result<(Vec<String>, String), Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let mut pool = Vec::new();
    for (name, target) in [("Push-ups", 20.0), ("Wall sit", 30.0)] {
        let note = notes.capture(name, None, None)?.id;
        Tallies::new(store).add_field(&note, name, "times", Some(target), None)?;
        pool.push(note);
    }
    let sport = notes.capture("Sport\nmorning moves", None, None)?.id;
    Ok((pool, sport))
}

/// Whether a plain text `label` shows, no card or button.
fn says(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::Label, label)
        .next()
        .is_some()
}

#[test]
fn a_compact_session_shows_the_next_run_of_its_routines_and_their_week() -> TestResult {
    let store = store()?;
    let (pool, sport) = sport(&store)?;
    let sessions = Sessions::new(&store);
    let today = Day::new(2026, 10, 26);
    // This morning's run Done; the other Routine starts tomorrow, later.
    let morning = daily_at(today, 7)?;
    let first = sessions.spawn(&sport, &morning, &pool, 1, SessionRule::Rolling)?;
    let at = oldbrainfix_core::ClockTime::new(7, 0).ok_or("a time")?;
    let done = oldbrainfix_core::Occurrence::new(today, Some(at));
    oldbrainfix_core::Routines::new(&store).mark_done(first.id(), &done, None)?;
    oldbrainfix_core::Routines::new(&store).spawn(&sport, &daily_at(today.add_days(1), 8)?)?;
    let harness = compact(&store)?;

    assert!(shows(&harness, "Sport"));
    assert!(shows(&harness, "Next: Tue, 10/27/2026 07:00"));
    assert!(shows(&harness, "done 1 / skipped 0 this week"));
    assert!(!shows(&harness, "morning moves"));
    Ok(())
}

#[test]
fn a_detailed_session_shows_its_pool_todays_picks_of_each_routine_and_four_weeks() -> TestResult {
    let store = store()?;
    let (pool, sport) = sport(&store)?;
    let sessions = Sessions::new(&store);
    let today = Day::new(2026, 10, 26);
    let morning = sessions.spawn(&sport, &daily_at(today, 7)?, &pool, 1, SessionRule::Rolling)?;
    oldbrainfix_core::Routines::new(&store).spawn(&sport, &daily_at(today, 18)?)?;
    let mut harness = window(&store);

    // Its name and description, then its pool and setup.
    assert!(shows(&harness, "Sport"));
    assert!(says(&harness, "Pool"));
    assert!(says(&harness, "Push-ups"));
    assert!(says(&harness, "Wall sit"));
    assert!(says(&harness, "Picks 1 Tally · Rolling"));
    // Today's picks under each Routine's time, Rolling going on.
    assert!(says(&harness, "Today"));
    assert!(says(&harness, "07:00"));
    assert!(says(&harness, "18:00"));
    assert!(shows(&harness, "Push-ups 20 times"));
    assert!(shows(&harness, "Wall sit 30 times"));
    assert_eq!(harness.query_all_by_label("Another one").count(), 2);
    // The last four weeks, this week first, from Sunday.
    assert!(says(&harness, "Last 4 weeks"));
    let weeks: Vec<String> = harness
        .query_all_by_label_contains("Week of ")
        .filter_map(|n| {
            let node = n.accesskit_node();
            node.label().or_else(|| node.value())
        })
        .collect();
    assert_eq!(weeks.len(), 4, "{weeks:?}");
    assert_eq!(weeks[0], "Week of Sun, 10/25/2026: done 0 / skipped 0");

    // "Another one" swaps the morning's pick there, and selects no card.
    harness
        .query_all_by_role_and_label(Role::Button, "Another one")
        .next()
        .ok_or("Another one")?
        .click();
    harness.run();
    pause(&mut harness);
    let marks = oldbrainfix_core::SessionMarks::of(&store, morning.id())?;
    assert_eq!(marks.picks.values().next(), Some(&vec![pool[1].clone()]));
    assert!(harness.state().selected_cards().is_empty());
    // A pick opens its entry form, as on the Agenda.
    harness
        .query_all_by_role_and_label(Role::Button, "Wall sit 30 times")
        .next()
        .ok_or("the pick")?
        .click();
    harness.run();
    assert!(shows(&harness, "New entry"));
    Ok(())
}

#[test]
fn a_session_without_a_routine_shows_its_pool_alone() -> TestResult {
    let store = store()?;
    let (pool, sport) = sport(&store)?;
    Sessions::new(&store).make(&sport, &pool[..1], 1, SessionRule::Rolling)?;
    let harness = window(&store);
    assert!(says(&harness, "Pool"));
    assert!(says(&harness, "Picks 1 Tally · Rolling"));
    assert!(!says(&harness, "Today"));
    assert!(!says(&harness, "Last 4 weeks"));
    let harness = compact(&store)?;
    assert!(shows(&harness, "Picks 1 Tally"));
    Ok(())
}

#[test]
fn an_older_session_set_up_on_its_routine_shows_as_a_session() -> TestResult {
    let store = store()?;
    let (pool, sport) = sport(&store)?;
    // As an older version wrote it: the setup on the Routine (ADR 0039).
    let schedule = daily_at(Day::new(2026, 10, 26), 18)?;
    oldbrainfix_core::Routines::new(&store).spawn_with(&sport, &schedule, |id| {
        store.append(id, "session:pool", Some(&pool.join(",")), None)?;
        store.append(id, "session:count", Some("1"), None)?;
        store.append(id, "session:rule", Some(SessionRule::Rolling.name()), None)?;
        Ok(())
    })?;
    assert!(store.current(&sport, "session:pool")?.is_none());
    let harness = window(&store);
    assert!(says(&harness, "Pool"));
    assert!(says(&harness, "18:00"));
    assert!(shows(&harness, "Push-ups 20 times"));
    let harness = compact(&store)?;
    assert!(shows(&harness, "Next: Mon, 10/26/2026 18:00"));
    assert!(shows(&harness, "done 0 / skipped 0 this week"));
    assert!(!shows(&harness, "morning moves"));
    Ok(())
}

#[test]
fn a_detailed_shopping_list_folds_its_struck_lines_until_a_click() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let wall: &oldbrainfix_core::WallClock = &utc;
    let groceries = notes.capture("Groceries", None, None)?.id;
    let shopping = ShoppingLists::new(&store);
    shopping.make(&groceries)?;
    let milk = shopping.add(&groceries, "Milk", wall)?;
    shopping.add(&groceries, "Bread", wall)?;
    shopping.check(&milk, wall)?;
    let mut harness = window(&store);

    assert!(shows(&harness, "Bread"));
    assert!(!shows(&harness, "Milk"));
    harness
        .get_by_role_and_label(Role::Button, "1 struck through")
        .click();
    harness.run();
    assert!(shows(&harness, "Milk"));
    // Kept open on this Device, also once another list showed.
    let kept = format!("shoppingStruckShown:{groceries}");
    assert_eq!(store.local_setting(&kept)?.as_deref(), Some("true"));
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(shows(&harness, "Milk"));
    pause(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "1 struck through")
        .click();
    harness.run();
    assert!(!shows(&harness, "Milk"));
    assert_eq!(store.local_setting(&kept)?, None);
    Ok(())
}

#[test]
fn a_cards_description_is_kept_unfolded_on_this_device() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let trip = notes.capture("Before the trip", None, None)?.id;
    Checklists::new(&store).make(&trip)?;
    notes.edit(&trip, "Before the trip\nPack early\nand light")?;
    let mut harness = window(&store);
    harness
        .get_by_role_and_label(Role::Button, "Show the whole description")
        .click();
    harness.run();
    let kept = format!("descriptionUnfolded:{trip}");
    assert_eq!(store.local_setting(&kept)?.as_deref(), Some("true"));
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Fold the description")
            .next()
            .is_some()
    );
    Ok(())
}

#[test]
fn the_view_button_switches_one_card_kept_until_the_lists_switch_changes() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let first = notes.capture("First Note\nmore", None, None)?.id;
    let second = notes.capture("Second Note\nmore", None, None)?.id;
    let mut harness = compact(&store)?;
    assert!(compact_card(&harness, "First Note\nmore"));
    assert!(compact_card(&harness, "Second Note\nmore"));

    switch(&mut harness, "Second Note\nmore");
    assert!(!compact_card(&harness, "Second Note\nmore"));
    assert!(compact_card(&harness, "First Note\nmore"));
    assert_eq!(switched(&store)?.as_deref(), Some(second.as_str()));
    // It switches without selecting the card or opening it.
    assert_eq!(harness.state().selected_note(), None);
    switch(&mut harness, "First Note\nmore");
    let kept: std::collections::HashSet<String> = switched(&store)?
        .unwrap_or_default()
        .split(',')
        .map(str::to_string)
        .collect();
    assert_eq!(kept, [first.clone(), second.clone()].into());
    switch(&mut harness, "First Note\nmore");
    assert!(compact_card(&harness, "First Note\nmore"));
    assert_eq!(switched(&store)?.as_deref(), Some(second.as_str()));

    // Another list and back: still so.
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(!compact_card(&harness, "Second Note\nmore"));
    assert!(compact_card(&harness, "First Note\nmore"));

    // The switch sets every card and forgets the switched ones.
    harness
        .get_by_role_and_label(Role::Button, "Show detailed views")
        .click();
    harness.run();
    assert_eq!(switched(&store)?, None);
    assert!(!compact_card(&harness, "First Note\nmore"));
    assert!(!compact_card(&harness, "Second Note\nmore"));
    harness
        .get_by_role_and_label(Role::Button, "Show compact views")
        .click();
    harness.run();
    assert!(compact_card(&harness, "First Note\nmore"));
    assert!(compact_card(&harness, "Second Note\nmore"));
    Ok(())
}

#[test]
fn a_click_selects_a_card_without_switching_it_and_a_double_click_opens_it() -> TestResult {
    let store = store()?;
    let milk = Notes::new(&store)
        .capture("Buy milk\nand bread", None, None)?
        .id;
    let mut harness = compact(&store)?;
    card(&harness, "Buy milk\nand bread").click();
    harness.run();
    assert!(compact_card(&harness, "Buy milk\nand bread"));
    assert_eq!(harness.state().selected_note(), None);
    assert_eq!(switched(&store)?, None);
    pause(&mut harness);
    let node = card(&harness, "Buy milk\nand bread");
    node.click();
    node.click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(milk.as_str()));
    Ok(())
}
