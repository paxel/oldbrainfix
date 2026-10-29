//! Sessions in the window (#104, ADR 0029): the Agenda lists today's
//! picks of a Session with their targets under its title and how many it
//! picks on later days; a click on a pick opens its entry form, prefilled
//! with the targets and without a date, which records the entry for the
//! Occurrence, or the entry it has; "Another one" swaps a pick; once every
//! pick has an entry the box stays checked. "Session" in a Note's editor
//! opens the Routine editor with the pool, the count, the weekdays' counts
//! and the rule on top, which say under each field what to fix and write
//! only what changed.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    Day, Notes, Occurrence, OwnProfile, Recurrence, Schedule, SessionRule, Sessions, SigningKey,
    Store, Tallies,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::session_editor::WEEKDAYS_OPEN;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The Tallies of the pool, in its order, with their targets.
const TALLIES: [(&str, f64); 3] = [
    ("Push-ups", 20.0),
    ("Wall sit", 30.0),
    ("Half bridge", 15.0),
];

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// A Device on a clock that ticks a second a read from Monday noon, with
/// a Tally of one field with a target for each of [`TALLIES`], and the
/// Notes "Sport" and "Leg day": the store, the Tallies' Notes in pool
/// order, and the two Notes.
struct Device {
    store: Arc<Store>,
    tallies: Vec<String>,
    sport: String,
    legs: String,
}

fn device() -> Result<Device, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T12:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let notes = Notes::new(&store);
    let mut tallies = Vec::new();
    for (name, target) in TALLIES {
        let note = notes.capture(name, None, None)?;
        Tallies::new(&store).add_field(&note.id, name, "times", Some(target), None)?;
        tallies.push(note.id);
    }
    let sport = notes.capture("Sport", None, None)?.id;
    let legs = notes.capture("Leg day", None, None)?.id;
    Ok(Device {
        store: Arc::new(store),
        tallies,
        sport,
        legs,
    })
}

fn today() -> Day {
    Day::new(2026, 3, 2)
}

/// Every day, all day, from today.
fn daily() -> Result<Schedule, Box<dyn std::error::Error>> {
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    Ok(Schedule::new(today(), &[], Some(rule)).ok_or("a schedule")?)
}

/// Spawns the Session "Sport" picking two of the pool by Rolling; its id.
fn spawn(device: &Device) -> Result<String, Box<dyn std::error::Error>> {
    let session = Sessions::new(&device.store).spawn(
        &device.sport,
        &daily()?,
        &device.tallies,
        2,
        SessionRule::Rolling,
    )?;
    Ok(session.routine.id)
}

/// The window over `store` in English, on the Agenda or with the Note
/// `note` open.
fn window(store: &Arc<Store>, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(berlin));
    app.set_region(Some("DE"));
    if let Some(note) = note {
        app.select(Selection::All);
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

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn click_button(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.run();
}

/// Types `text` over what the field labelled `label` holds.
fn type_into(harness: &mut Harness<'_, App>, label: &str, text: &str) {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .type_text(text);
    harness.run();
}

fn holds(harness: &Harness<'_, App>, label: &str) -> Option<String> {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .accesskit_node()
        .value()
}

/// Whether the first widget of `role` labelled `label` is checked: on
/// the Agenda today's.
fn checked(harness: &Harness<'_, App>, role: Role, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(role, label)
        .next()
        .is_some_and(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

/// Today's Occurrence of every-day, all-day Session.
fn occurrence() -> Occurrence {
    Occurrence::new(today(), None)
}

#[test]
fn the_agenda_lists_todays_picks_with_targets_and_later_days_how_many() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let harness = window(&device.store, None);
    let t = L10n::new("en");
    // Rolling starts with the first two of the pool, under the title.
    assert!(shows(&harness, "Push-ups 20 times"));
    assert!(shows(&harness, "Wall sit 30 times"));
    assert!(!shows(&harness, "Half bridge 15 times"));
    let title = harness
        .query_all_by_role_and_label(Role::Button, "Sport")
        .next()
        .ok_or("today's row")?
        .rect()
        .top();
    let first = harness
        .get_by_role_and_label(Role::Button, "Push-ups 20 times")
        .rect()
        .top();
    assert!(title < first);
    assert_eq!(harness.query_all_by_label(t.session_another()).count(), 2);
    // Tomorrow and the five days after say how many.
    assert_eq!(harness.query_all_by_label("Picks 2 Tallies").count(), 6);
    // Showing wrote the pick for the user's other Devices.
    let marks = oldbrainfix_core::SessionMarks::of(&device.store, &id)?;
    assert_eq!(
        marks.picks.get(&occurrence().key()),
        Some(&device.tallies[..2].to_vec())
    );
    Ok(())
}

#[test]
fn a_click_on_a_pick_records_an_entry_prefilled_with_its_targets() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let mut harness = window(&device.store, None);
    let t = L10n::new("en");
    click_button(&mut harness, "Push-ups 20 times");
    assert!(shows(&harness, t.tally_new_entry()));
    assert_eq!(holds(&harness, "Push-ups").as_deref(), Some("20"));
    // It counts for now: no date.
    assert!(!shows(&harness, t.tally_entry_date()));
    click_button(&mut harness, t.save());
    assert_eq!(
        harness.state().selected_note(),
        Some(device.tallies[0].as_str())
    );
    let today = Sessions::new(&device.store).on(&id, &occurrence())?;
    assert_eq!(today.picks[0].entries.len(), 1);
    assert!(!today.picks[1].done());

    // Back on the Agenda the pick has no "Another one" any more.
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert_eq!(harness.query_all_by_label(t.session_another()).count(), 1);
    assert!(!checked(&harness, Role::CheckBox, "Done: All day Sport"));
    Ok(())
}

#[test]
fn once_every_pick_has_an_entry_the_box_stays_checked() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let sessions = Sessions::new(&device.store);
    sessions.pick_today(&berlin)?;
    for tally in &device.tallies[..2] {
        sessions.record(&id, &occurrence(), tally, &BTreeMap::new(), "done")?;
    }
    let harness = window(&device.store, None);
    let t = L10n::new("en");
    // Today's comes first; tomorrow's is open.
    let check = harness
        .query_all_by_role_and_label(Role::CheckBox, "Done: All day Sport")
        .next()
        .ok_or("today's box")?;
    assert_eq!(check.accesskit_node().toggled(), Some(Toggled::True));
    assert!(check.accesskit_node().is_disabled());
    assert!(!shows(&harness, t.session_another()));
    Ok(())
}

#[test]
fn a_pick_with_an_entry_opens_that_entry() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let sessions = Sessions::new(&device.store);
    sessions.pick_today(&berlin)?;
    let field = Tallies::new(&device.store)
        .by_note(&device.tallies[0])?
        .ok_or("a Tally")?
        .fields[0]
        .id
        .clone();
    let values = BTreeMap::from([(field, 25.0)]);
    sessions.record(&id, &occurrence(), &device.tallies[0], &values, "")?;
    let mut harness = window(&device.store, None);
    let t = L10n::new("en");
    click_button(&mut harness, "Push-ups 20 times");
    assert!(shows(&harness, t.tally_entry()));
    assert_eq!(holds(&harness, "Push-ups").as_deref(), Some("25"));
    assert!(shows(&harness, t.tally_entry_date()));
    Ok(())
}

#[test]
fn another_one_swaps_a_pick_for_the_next() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let mut harness = window(&device.store, None);
    let t = L10n::new("en");
    harness
        .query_all_by_label(t.session_another())
        .next()
        .ok_or("Another one")?
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "Half bridge 15 times"));
    assert!(!shows(&harness, "Push-ups 20 times"));
    assert!(shows(&harness, "Wall sit 30 times"));
    Ok(())
}

#[test]
fn a_tally_gone_from_every_pick_says_none_is_left() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let notes = Notes::new(&device.store);
    for tally in &device.tallies {
        notes.trash(tally)?;
    }
    let harness = window(&device.store, None);
    assert!(shows(&harness, L10n::new("en").session_nothing_to_pick()));
    Ok(())
}

#[test]
fn the_note_editor_makes_a_session_saying_what_to_fix() -> TestResult {
    let device = device()?;
    let mut harness = window(&device.store, Some(&device.legs));
    let t = L10n::new("en");
    click_button(&mut harness, t.session());
    assert!(shows(&harness, t.new_session()));
    // Rolling by default, one a time, the weekdays folded.
    assert!(checked(
        &harness,
        Role::RadioButton,
        t.session_rule_rolling()
    ));
    assert_eq!(holds(&harness, t.session_count()).as_deref(), Some("1"));
    assert!(shows(&harness, &t.session_weekday_counts_summary(0)));

    // Nothing chosen: Save says so and writes nothing.
    click_button(&mut harness, t.save());
    assert!(shows(&harness, t.session_pool_missing()));
    assert!(Sessions::new(&device.store).all()?.is_empty());

    harness
        .get_by_role_and_label(Role::CheckBox, "Wall sit")
        .click_accesskit();
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    assert!(!shows(&harness, t.session_pool_missing()));
    // Newest capture first; Push-ups goes above Wall sit for Rolling.
    click_button(&mut harness, &t.session_move_up("Push-ups"));
    type_into(&mut harness, t.session_count(), "0");
    assert!(shows(&harness, t.session_count_invalid()));
    type_into(&mut harness, t.session_count(), "2");
    assert!(!shows(&harness, t.session_count_invalid()));

    // The fold opens and stays so on this Device.
    harness.get_by_label(t.session_weekday_counts()).click();
    harness.run();
    assert_eq!(
        device.store.local_setting(WEEKDAYS_OPEN)?.as_deref(),
        Some("true")
    );
    type_into(&mut harness, "Saturday", "1");
    click_button(&mut harness, t.save());

    let sessions = Sessions::new(&device.store).all()?;
    let [session] = sessions.as_slice() else {
        return Err(format!("one Session: {sessions:?}").into());
    };
    assert_eq!(session.routine.note_id, device.legs);
    assert_eq!(
        session.pool,
        vec![device.tallies[0].clone(), device.tallies[1].clone()]
    );
    assert_eq!(session.count, 2);
    assert_eq!(session.counts, BTreeMap::from([(6, 1)]));
    assert_eq!(session.rule, SessionRule::Rolling);
    assert!(harness.state().routine_editor().is_none());
    Ok(())
}

#[test]
fn a_wrong_weekday_count_opens_the_fold_on_save() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    Sessions::new(&device.store).set_count_on(&id, 3, Some(1))?;
    let mut harness = window(&device.store, None);
    harness.state_mut().open_routine(&id);
    harness.run();
    let t = L10n::new("en");
    assert!(shows(&harness, t.session()));
    // Folded, the weekday's own count is summed up.
    assert!(shows(&harness, &t.session_weekday_counts_summary(1)));
    harness.get_by_label(t.session_weekday_counts()).click();
    harness.run();
    assert_eq!(holds(&harness, "Wednesday").as_deref(), Some("1"));
    type_into(&mut harness, "Wednesday", "x");
    // Folded again, then saved: the fold opens on the field to fix.
    harness.get_by_label(t.session_weekday_counts()).click();
    harness.run();
    assert!(shows(&harness, t.session_count_invalid()));
    click_button(&mut harness, t.save());
    assert!(harness.state().routine_editor().is_some());
    assert_eq!(holds(&harness, "Wednesday").as_deref(), Some("x"));
    assert_eq!(
        device.store.local_setting(WEEKDAYS_OPEN)?.as_deref(),
        Some("true")
    );
    Ok(())
}

#[test]
fn saving_a_session_writes_only_what_changed() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let written = |field: &str| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(device
            .store
            .history(&id)?
            .iter()
            .filter(|e| e.field == field)
            .count())
    };
    let before: Vec<usize> = ["session:pool", "session:count", "session:rule"]
        .iter()
        .map(|f| written(f))
        .collect::<Result<_, _>>()?;
    let mut harness = window(&device.store, None);
    harness.state_mut().open_routine(&id);
    harness.run();
    let t = L10n::new("en");
    harness
        .get_by_role_and_label(Role::RadioButton, t.session_rule_least())
        .click_accesskit();
    harness.run();
    // Without Rolling the order has no arrows.
    assert!(!shows(&harness, &t.session_move_down("Push-ups")));
    click_button(&mut harness, t.save());
    let after: Vec<usize> = ["session:pool", "session:count", "session:rule"]
        .iter()
        .map(|f| written(f))
        .collect::<Result<_, _>>()?;
    assert_eq!(after, vec![before[0], before[1], before[2] + 1]);
    for day in 1..=7 {
        assert_eq!(written(&format!("session:count:{day}"))?, 0, "{day}");
    }
    let session = Sessions::new(&device.store)
        .by_id(&id)?
        .ok_or("a Session")?;
    assert_eq!(session.rule, SessionRule::Least);
    Ok(())
}
