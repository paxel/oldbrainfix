//! Sessions in the window (#104, ADR 0029): the Agenda lists today's
//! picks of a Session with their targets under its title and how many it
//! picks on later days; a click on a pick opens its entry form, prefilled
//! with the targets and without a date, which records the entry for the
//! Occurrence, or the entry it has; "Another one" swaps a pick; once every
//! pick has an entry the box stays checked; each Routine of a Session
//! Note shows its own Occurrences, Rolling going on across them.
//! "Session" in a plain Note's "+" opens the Routine editor with the pool,
//! the count's − / + stepper, the weekdays with a checkbox each for an own
//! count, and the rule on top, which say what to fix in the pool; "New
//! Session" on All Routines and Reminders makes one with its Note from the
//! text typed. Without a Tally both still offer it, the editor says how to
//! make one, and Save makes nothing. A Session is a kind of Note (#191,
//! ADR 0039): its kind badge makes a plain Note one on the setup's page
//! and one a plain Note again; its page in the Note dialog writes the pool
//! and the count at once, its badges the rule and the weekdays' counts;
//! "+" there offers no Session, its Routine running it, and the Routine
//! editor of one only schedules. The pool filters by its Tallies' tags,
//! any picked, keeping the ticks it hides and counting them, and New
//! Tally from it carries the tags (#220).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    Day, DeviceSettings, Notes, Occurrence, OwnProfile, Recurrence, Routines, Schedule,
    SessionRule, Sessions, SigningKey, Store, Tallies,
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

/// Opens the menu of the Note dialog's "+" badge (#160).
fn add_menu(harness: &mut Harness<'_, App>, t: &L10n) {
    harness
        .get_by_label(t.routine_or_reminder())
        .click_accesskit();
    harness.run();
}

fn click_button(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.run();
}

fn holds(harness: &Harness<'_, App>, label: &str) -> Option<String> {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .accesskit_node()
        .value()
}

/// Clicks the `nth` button labelled `label`, the Session's steppers
/// first.
fn click_nth(harness: &mut Harness<'_, App>, label: &str, nth: usize) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .nth(nth)
        .ok_or_else(|| format!("{label} #{nth}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// The Session's count and weekdays' own counts in the Routine editor.
fn draft_counts(harness: &Harness<'_, App>) -> Option<(i64, [Option<i64>; 7])> {
    let draft = harness.state().routine_editor()?.session()?;
    Some((draft.count, draft.counts))
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
fn the_card_and_the_page_list_the_past_runs_each_pick_done_or_not() -> TestResult {
    use oldbrainfix_core::sessions::keys;
    let device = device()?;
    let store = &device.store;
    // Every day from yesterday, its pick of yesterday as a Device wrote it.
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let from = today().add_days(-1);
    let schedule = Schedule::new(from, &[], Some(rule)).ok_or("a schedule")?;
    let (pool, count) = (&device.tallies, 2);
    let made =
        Sessions::new(store).spawn(&device.sport, &schedule, pool, count, SessionRule::Rolling)?;
    let id = made.routine.id;
    let yesterday = Occurrence::new(from, None);
    let picked = format!("{},{}", device.tallies[0], device.tallies[1]);
    store.append(&id, &keys::pick(&yesterday), Some(&picked), None)?;
    let entry = Tallies::new(store).add(&device.tallies[0], &BTreeMap::new(), "Done", None)?;
    let mark = keys::entry(&yesterday, &entry.id);
    store.append(&id, &mark, Some(&device.tallies[0]), None)?;
    let t = L10n::new("en");
    let harness = window(store, Some(&device.sport));
    // On its card, and on its page in the Note dialog.
    assert_eq!(harness.query_all_by_label(t.session_runs()).count(), 2);
    let done = t.session_run_done("Push-ups");
    assert_eq!(harness.query_all_by_label(&done).count(), 2);
    let not_done = t.session_run_not_done("Wall sit");
    assert_eq!(harness.query_all_by_label(&not_done).count(), 2);
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
fn the_headings_box_only_shows_done_and_each_pick_has_a_box_that_opens_it() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let mut harness = window(&device.store, None);
    let t = L10n::new("en");
    // Not clickable, even before its picks have entries (#208).
    let heading = harness
        .query_all_by_role_and_label(Role::CheckBox, "Done: All day Sport")
        .next()
        .ok_or("today's box")?;
    assert!(heading.accesskit_node().is_disabled());
    let heading_box = heading.rect();
    heading.click_accesskit();
    harness.run();
    let today = Sessions::new(&device.store).on(&id, &occurrence())?;
    assert!(!today.complete());
    assert!(harness.state().routine_editor().is_none());
    // Each pick an entry with its box, indented under the heading.
    let pick = harness
        .query_all_by_role_and_label(Role::CheckBox, "Push-ups 20 times")
        .next()
        .ok_or("the pick's box")?;
    assert_ne!(pick.accesskit_node().toggled(), Some(Toggled::True));
    assert!(pick.rect().left() > heading_box.left());
    assert!(pick.rect().top() > heading_box.top());
    pick.click_accesskit();
    harness.run();
    assert!(shows(&harness, t.tally_new_entry()));
    click_button(&mut harness, t.save());
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(checked(&harness, Role::CheckBox, "Push-ups 20 times"));
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
    add_menu(&mut harness, &t);
    click_button(&mut harness, t.session());
    assert!(shows(&harness, t.new_session()));
    // Every day, All day, by default.
    assert!(shows(&harness, t.all_day()));
    // Rolling by default, one a time, the weekdays folded.
    assert!(checked(
        &harness,
        Role::RadioButton,
        t.session_rule_rolling()
    ));
    assert_eq!(draft_counts(&harness), Some((1, [None; 7])));
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
    // − is gone at 1; + counts up, and − back down.
    let fewer = harness
        .query_all_by_role_and_label(Role::Button, t.fewer())
        .count();
    click_nth(&mut harness, t.more(), 0)?;
    click_nth(&mut harness, t.more(), 0)?;
    click_nth(&mut harness, t.fewer(), 0)?;
    assert_eq!(draft_counts(&harness), Some((2, [None; 7])));
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, t.fewer())
            .count(),
        fewer + 1
    );

    // The fold opens and stays so on this Device.
    harness.get_by_label(t.session_weekday_counts()).click();
    harness.run();
    assert_eq!(
        device.store.local_setting(WEEKDAYS_OPEN)?.as_deref(),
        Some("true")
    );
    // Each weekday picks as many as every day until it gets its own
    // count, which starts from every day's.
    assert_eq!(
        harness
            .query_all_by_label(t.session_weekday_count_hint())
            .count(),
        7
    );
    harness
        .get_by_role_and_label(Role::CheckBox, "Saturday")
        .click_accesskit();
    harness.run();
    assert_eq!(
        harness
            .query_all_by_label(t.session_weekday_count_hint())
            .count(),
        6
    );
    let mut own = [None; 7];
    own[5] = Some(2);
    assert_eq!(draft_counts(&harness), Some((2, own)));
    // Saturday's stepper comes after every day's.
    click_nth(&mut harness, t.fewer(), 1)?;
    own[5] = Some(1);
    assert_eq!(draft_counts(&harness), Some((2, own)));
    click_button(&mut harness, t.save());

    let sessions = Sessions::new(&device.store).all()?;
    let [session] = sessions.as_slice() else {
        return Err(format!("one Session: {sessions:?}").into());
    };
    assert_eq!(session.routine.note_id, device.legs);
    assert_eq!(
        session.setup.pool,
        vec![device.tallies[0].clone(), device.tallies[1].clone()]
    );
    assert_eq!(session.setup.count, 2);
    assert_eq!(session.setup.counts, BTreeMap::from([(6, 1)]));
    assert_eq!(session.setup.rule, SessionRule::Rolling);
    assert!(session.routine.schedule.times.is_empty());
    assert!(harness.state().routine_editor().is_none());
    Ok(())
}

/// Clicks the badge of `setting` showing `value` in the Note dialog.
fn open_badge(harness: &mut Harness<'_, App>, t: &L10n, setting: &str, value: &str) {
    click_button(harness, &t.setting_badge_label(setting, value));
}

/// The setup of the Session Note `note`.
fn setup_of(
    device: &Device,
    note: &str,
) -> Result<oldbrainfix_core::SessionSetup, Box<dyn std::error::Error>> {
    Ok(Sessions::new(&device.store)
        .setup_of(note)?
        .ok_or("a Session")?)
}

/// How often `field` was written on `entity`.
fn written(
    device: &Device,
    entity: &str,
    field: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(device
        .store
        .history(entity)?
        .iter()
        .filter(|e| e.field == field)
        .count())
}

/// Whether the checkbox `label` may be clicked.
fn enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    !harness
        .get_by_role_and_label(Role::CheckBox, label)
        .accesskit_node()
        .is_disabled()
}

#[test]
fn the_kind_badge_makes_a_plain_note_a_session_and_back() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Some(&device.legs));
    open_badge(&mut harness, &t, t.badge_kind(), t.kind_plain());
    harness
        .get_by_role_and_label(Role::RadioButton, t.session())
        .click_accesskit();
    harness.run();
    // Its setup alone: the pool, the count, the weekdays and the rule,
    // no schedule.
    assert!(
        harness
            .state()
            .routine_editor()
            .is_some_and(|e| e.makes_session())
    );
    assert!(shows(&harness, t.session_pool()));
    assert!(shows(&harness, t.new_tally()));
    assert!(!shows(&harness, t.all_day()));
    assert!(!shows(&harness, t.next_occurrences()));
    // Without a Tally it says so and makes nothing.
    click_button(&mut harness, t.save());
    assert!(shows(&harness, t.session_pool_missing()));
    assert!(!Sessions::new(&device.store).is_session_note(&device.legs)?);

    harness
        .get_by_role_and_label(Role::CheckBox, "Wall sit")
        .click_accesskit();
    harness.run();
    click_nth(&mut harness, t.more(), 0)?;
    harness
        .get_by_role_and_label(Role::RadioButton, t.session_rule_least())
        .click_accesskit();
    harness.run();
    click_button(&mut harness, t.save());
    assert!(harness.state().routine_editor().is_none());
    let setup = setup_of(&device, &device.legs)?;
    assert_eq!(setup.pool, vec![device.tallies[1].clone()]);
    assert_eq!(setup.count, 2);
    assert_eq!(setup.rule, SessionRule::Least);
    // It runs nowhere yet: its Routines say when.
    assert!(
        Routines::new(&device.store)
            .of_note(&device.legs)?
            .is_empty()
    );
    // Its page shows the setup, its badges the kind and the rule.
    assert!(checked(&harness, Role::CheckBox, "Wall sit"));
    assert!(shows(
        &harness,
        &t.setting_badge_label(t.badge_kind(), t.session())
    ));
    assert!(shows(
        &harness,
        &t.setting_badge_label(t.session_rule(), t.session_rule_least())
    ));

    // Back to a plain Note: no pool on its page any more.
    open_badge(&mut harness, &t, t.badge_kind(), t.session());
    harness
        .get_by_role_and_label(Role::RadioButton, t.kind_plain())
        .click_accesskit();
    harness.run();
    assert!(!Sessions::new(&device.store).is_session_note(&device.legs)?);
    assert!(!shows(&harness, t.session_pool()));
    assert!(shows(
        &harness,
        &t.setting_badge_label(t.badge_kind(), t.kind_plain())
    ));
    Ok(())
}

#[test]
fn a_session_made_plain_again_keeps_its_routines_running_plain() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Some(&device.sport));
    open_badge(&mut harness, &t, t.badge_kind(), t.session());
    harness
        .get_by_role_and_label(Role::RadioButton, t.kind_plain())
        .click_accesskit();
    harness.run();
    assert!(!Sessions::new(&device.store).is_session_note(&device.sport)?);
    let routines = Routines::new(&device.store).of_note(&device.sport)?;
    assert_eq!(routines.len(), 1);
    assert!(Sessions::new(&device.store).by_id(&id)?.is_none());
    // A plain Note may become one again.
    open_badge(&mut harness, &t, t.badge_kind(), t.kind_plain());
    assert!(
        harness
            .query_all_by_role_and_label(Role::RadioButton, t.session())
            .next()
            .is_some()
    );
    Ok(())
}

#[test]
fn the_session_page_writes_its_pool_and_count_at_once() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let t = L10n::new("en");
    let sport = device.sport.as_str();
    let before = [
        written(&device, sport, "session:pool")?,
        written(&device, sport, "session:count")?,
        written(&device, sport, "session:rule")?,
    ];
    let mut harness = window(&device.store, Some(sport));
    // After its name: the pool, every Tally ticked, and the count.
    for name in ["Push-ups", "Wall sit", "Half bridge"] {
        assert!(checked(&harness, Role::CheckBox, name), "{name}");
    }
    assert!(shows(&harness, t.session_count()));
    // A Session becomes no Tally.
    assert!(!shows(&harness, t.make_tally()));
    harness
        .get_by_role_and_label(Role::CheckBox, "Half bridge")
        .click_accesskit();
    harness.run();
    assert_eq!(setup_of(&device, sport)?.pool, device.tallies[..2].to_vec());
    assert_eq!(written(&device, sport, "session:pool")?, before[0] + 1);
    // Rolling's order: only those ticked move.
    assert!(!shows(&harness, &t.session_move_up("Half bridge")));
    click_button(&mut harness, &t.session_move_up("Wall sit"));
    assert_eq!(
        setup_of(&device, sport)?.pool,
        vec![device.tallies[1].clone(), device.tallies[0].clone()]
    );
    click_nth(&mut harness, t.more(), 0)?;
    let setup = setup_of(&device, sport)?;
    assert_eq!(setup.count, 3);
    assert_eq!(written(&device, sport, "session:count")?, before[1] + 1);
    assert_eq!(written(&device, sport, "session:rule")?, before[2]);
    // The last Tally ticked stays.
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    assert_eq!(
        setup_of(&device, sport)?.pool,
        vec![device.tallies[1].clone()]
    );
    assert!(!enabled(&harness, "Wall sit"));
    assert!(enabled(&harness, "Push-ups"));
    Ok(())
}

#[test]
fn a_tick_right_after_typing_the_description_is_written_with_it() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let t = L10n::new("en");
    let sport = device.sport.as_str();
    let mut harness = window(&device.store, Some(sport));
    harness
        .get_by(|n| n.placeholder() == Some(t.note_description()))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Mornings");
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, "Half bridge")
        .click();
    harness.run();
    assert_eq!(setup_of(&device, sport)?.pool, device.tallies[..2].to_vec());
    harness.state_mut().close_note();
    harness.run();
    let note = Notes::new(&device.store).by_id(sport)?.ok_or("the Note")?;
    assert_eq!(note.text, "Sport\nMornings");
    Ok(())
}

#[test]
fn new_tally_on_the_session_page_comes_first_in_its_pool() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Some(&device.sport));
    click_button(&mut harness, t.new_tally());
    harness.get_by(|n| n.is_focused()).type_text("Plank");
    harness.run();
    harness
        .get_all_by_role_and_label(Role::TextInput, t.kind_name())
        .last()
        .ok_or("the field's name")?
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Seconds");
    harness.run();
    click_button(&mut harness, t.save());
    let plank = Notes::new(&device.store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Plank")
        .ok_or("the Tally's Note")?;
    let pool = setup_of(&device, &device.sport)?.pool;
    assert_eq!(pool.first(), Some(&plank.id));
    assert_eq!(pool.len(), 4);
    assert!(checked(&harness, Role::CheckBox, "Plank"));
    Ok(())
}

/// Whether the pool lists the Tally `name`.
fn listed(harness: &Harness<'_, App>, name: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::CheckBox, name)
        .next()
        .is_some()
}

/// What the text field with the keyboard holds.
fn focused_value(harness: &Harness<'_, App>) -> Option<String> {
    harness.get_by(|n| n.is_focused()).accesskit_node().value()
}

/// Adds a Tally `text` of one field "Done" to the device; its Note's id.
fn tagged(device: &Device, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let note = Notes::new(&device.store).capture(text, None, None)?;
    Tallies::new(&device.store).add_field(&note.id, "Done", "times", None, None)?;
    Ok(note.id)
}

#[test]
fn the_pools_tags_filter_it_by_any_picked_keeping_hidden_ticks() -> TestResult {
    let device = device()?;
    let mop = tagged(&device, "Mop #Cleaning")?;
    tagged(&device, "Dust #cleaning #home")?;
    spawn(&device)?;
    let t = L10n::new("en");
    let sport = device.sport.as_str();
    let mut harness = window(&device.store, Some(sport));
    // Those most Tallies carry first.
    let chips: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| l.starts_with('#'))
        .collect();
    assert_eq!(chips, ["#cleaning", "#home"]);
    assert!(shows(&harness, &t.session_move_up("Wall sit")));
    click_button(&mut harness, "#cleaning");
    assert!(checked(&harness, Role::Button, "#cleaning"));
    assert!(!listed(&harness, "Push-ups"));
    assert!(listed(&harness, "Mop #Cleaning"));
    assert!(listed(&harness, "Dust #cleaning #home"));
    assert!(shows(&harness, &t.session_pool_hidden(3)));
    // Rolling's order is set only in the whole list.
    assert!(!shows(&harness, &t.session_move_up("Wall sit")));
    // A tick while filtered is written, and the filter stays.
    harness
        .get_by_role_and_label(Role::CheckBox, "Mop #Cleaning")
        .click_accesskit();
    harness.run();
    let mut pool = device.tallies.clone();
    pool.push(mop.clone());
    assert_eq!(setup_of(&device, sport)?.pool, pool);
    assert!(!listed(&harness, "Push-ups"));
    assert!(!shows(&harness, &t.session_move_up("Mop #Cleaning")));
    // New Tally carries the tags picked.
    click_button(&mut harness, t.new_tally());
    assert_eq!(focused_value(&harness).as_deref(), Some("#cleaning "));
    harness.get_by(|n| n.is_focused()).type_text("Plank");
    harness.run();
    harness
        .get_all_by_role_and_label(Role::TextInput, t.kind_name())
        .last()
        .ok_or("the field's name")?
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Seconds");
    harness.run();
    click_button(&mut harness, t.save());
    let plank = Notes::new(&device.store)
        .all()?
        .into_iter()
        .find(|n| n.text == "#cleaning Plank")
        .ok_or("the Tally's Note")?;
    assert_eq!(setup_of(&device, sport)?.pool.first(), Some(&plank.id));
    assert!(checked(&harness, Role::CheckBox, "#cleaning Plank"));
    assert!(!listed(&harness, "Push-ups"));
    // A click again takes the tag back.
    click_button(&mut harness, "#cleaning");
    assert!(listed(&harness, "Push-ups"));
    assert!(!shows(&harness, &t.session_pool_hidden(3)));
    Ok(())
}

#[test]
fn the_setup_page_of_a_new_session_filters_its_pool_by_tag() -> TestResult {
    let device = device()?;
    tagged(&device, "Mop #cleaning")?;
    tagged(&device, "Squats #sport")?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Some(&device.legs));
    open_badge(&mut harness, &t, t.badge_kind(), t.kind_plain());
    harness
        .get_by_role_and_label(Role::RadioButton, t.session())
        .click_accesskit();
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    click_button(&mut harness, "#cleaning");
    click_button(&mut harness, "#sport");
    // Any of the tags picked.
    assert!(listed(&harness, "Mop #cleaning"));
    assert!(listed(&harness, "Squats #sport"));
    assert!(!listed(&harness, "Wall sit"));
    assert!(shows(&harness, &t.session_pool_hidden(1)));
    click_button(&mut harness, t.new_tally());
    assert_eq!(
        focused_value(&harness).as_deref(),
        Some("#cleaning #sport ")
    );
    Ok(())
}

#[test]
fn the_rule_badge_picks_how_it_picks_at_once() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let t = L10n::new("en");
    let sport = device.sport.as_str();
    let before = written(&device, sport, "session:rule")?;
    let mut harness = window(&device.store, Some(sport));
    open_badge(&mut harness, &t, t.session_rule(), t.session_rule_rolling());
    assert!(shows(&harness, t.session_rule_least_detail()));
    harness
        .get_by_role_and_label(Role::RadioButton, t.session_rule_least())
        .click_accesskit();
    harness.run();
    let setup = setup_of(&device, sport)?;
    assert_eq!(setup.rule, SessionRule::Least);
    assert_eq!(written(&device, sport, "session:rule")?, before + 1);
    assert_eq!(setup.count, 2);
    // The pick closes it; without Rolling the pool has no arrows.
    assert!(!shows(&harness, t.session_rule_least_detail()));
    assert!(!shows(&harness, &t.session_move_down("Push-ups")));
    assert!(shows(
        &harness,
        &t.setting_badge_label(t.session_rule(), t.session_rule_least())
    ));
    Ok(())
}

#[test]
fn the_weekdays_badge_gives_and_takes_a_weekdays_own_count_at_once() -> TestResult {
    let device = device()?;
    spawn(&device)?;
    let sport = device.sport.as_str();
    Sessions::new(&device.store).set_count_on(sport, 3, Some(4))?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Some(sport));
    let summary = t.session_weekday_counts_summary(1);
    open_badge(&mut harness, &t, t.session_weekday_counts(), &summary);
    assert!(checked(&harness, Role::CheckBox, "Wednesday"));
    let hints = |harness: &Harness<'_, App>| {
        harness
            .query_all_by_label(t.session_weekday_count_hint())
            .count()
    };
    assert_eq!(hints(&harness), 6);
    harness
        .get_by_role_and_label(Role::CheckBox, "Wednesday")
        .click_accesskit();
    harness.run();
    assert_eq!(hints(&harness), 7);
    assert!(setup_of(&device, sport)?.counts.is_empty());
    // Saturday's own count starts from every day's, its stepper last.
    harness
        .get_by_role_and_label(Role::CheckBox, "Saturday")
        .click_accesskit();
    harness.run();
    assert_eq!(setup_of(&device, sport)?.counts, BTreeMap::from([(6, 2)]));
    harness
        .query_all_by_role_and_label(Role::Button, t.more())
        .last()
        .ok_or("Saturday's +")?
        .click_accesskit();
    harness.run();
    assert_eq!(setup_of(&device, sport)?.counts, BTreeMap::from([(6, 3)]));
    // The dialog's Close, over the Note dialog's.
    harness
        .query_all_by_role_and_label(Role::Button, t.close())
        .last()
        .ok_or("Close")?
        .click_accesskit();
    harness.run();
    assert!(!shows(&harness, t.session_weekday_counts_explain()));
    assert!(shows(
        &harness,
        &t.setting_badge_label(t.session_weekday_counts(), &summary)
    ));
    Ok(())
}

#[test]
fn a_routine_of_a_session_note_only_schedules_it() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    let before = setup_of(&device, &device.sport)?;
    let mut harness = window(&device.store, None);
    harness.state_mut().open_routine(&id);
    harness.run();
    let t = L10n::new("en");
    assert!(harness.state().routine_editor().is_some());
    assert!(shows(&harness, t.routine()));
    assert!(!shows(&harness, t.session_pool()));
    assert!(!shows(&harness, t.session_rule()));
    assert!(!shows(&harness, t.session_count()));
    click_button(&mut harness, t.save());
    assert!(harness.state().routine_editor().is_none());
    assert_eq!(setup_of(&device, &device.sport)?, before);
    Ok(())
}

#[test]
fn plus_on_a_session_note_offers_no_session_and_its_routine_runs_it() -> TestResult {
    let device = device()?;
    let first = spawn(&device)?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, Some(&device.sport));
    add_menu(&mut harness, &t);
    for offered in [t.routine(), t.reminder(), t.frog()] {
        assert!(
            harness
                .query_all_by_role_and_label(Role::Button, offered)
                .next()
                .is_some(),
            "{offered}"
        );
    }
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, t.session())
            .next()
            .is_none()
    );
    click_button(&mut harness, t.routine());
    assert!(
        harness
            .state()
            .routine_editor()
            .is_some_and(|e| e.session().is_none())
    );
    click_button(&mut harness, t.save());
    let runs = Sessions::new(&device.store).of_note(&device.sport)?;
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().any(|r| r.id() == first));
    // Each Routine is a Routine badge on its page.
    let badges = harness
        .query_all_by_label_contains(&format!("{}: ", t.routine()))
        .count();
    assert_eq!(badges, 2);
    Ok(())
}

#[test]
fn the_agenda_shows_each_occurrence_of_each_routine_of_a_session() -> TestResult {
    let device = device()?;
    let all_day = spawn(&device)?;
    // A second Routine of the Note, this evening.
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let at = [oldbrainfix_core::ClockTime::new(18, 0).ok_or("a time")?];
    let evening = Schedule::new(today(), &at, Some(rule)).ok_or("a schedule")?;
    let later = Routines::new(&device.store)
        .spawn(&device.sport, &evening)?
        .id;
    let harness = window(&device.store, None);
    let t = L10n::new("en");
    // Rolling goes on from the morning's pick in the evening (ADR 0039).
    assert!(shows(&harness, "Push-ups 20 times"));
    assert!(shows(&harness, "Wall sit 30 times"));
    assert!(shows(&harness, "Half bridge 15 times"));
    assert_eq!(harness.query_all_by_label(t.session_another()).count(), 4);
    let six = oldbrainfix_core::ClockTime::new(18, 0).ok_or("a time")?;
    let picked = |id: &str, at: Occurrence| -> Result<_, Box<dyn std::error::Error>> {
        let marks = oldbrainfix_core::SessionMarks::of(&device.store, id)?;
        Ok(marks.picks.get(&at.key()).cloned())
    };
    assert_eq!(
        picked(&all_day, occurrence())?,
        Some(device.tallies[..2].to_vec())
    );
    assert_eq!(
        picked(&later, Occurrence::new(today(), Some(six)))?,
        Some(vec![device.tallies[2].clone(), device.tallies[0].clone()])
    );
    Ok(())
}

#[test]
fn new_session_on_all_routines_makes_one_with_its_note_from_the_text() -> TestResult {
    let device = device()?;
    let t = L10n::new("en");
    let mut harness = window(&device.store, None);
    harness
        .get_by_label(t.all_routines_title())
        .click_accesskit();
    harness.run();
    click_button(&mut harness, t.new_session());
    assert!(shows(&harness, t.new_session()));
    harness
        .get_by(|n| n.placeholder() == Some(t.routine_what_hint()))
        .click();
    harness.run();
    harness
        .get_by(|n| n.is_focused())
        .type_text("Morning sport");
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, "Push-ups")
        .click_accesskit();
    harness.run();
    click_button(&mut harness, t.save());

    let sessions = Sessions::new(&device.store).all()?;
    let [session] = sessions.as_slice() else {
        return Err(format!("one Session: {sessions:?}").into());
    };
    let note = Notes::new(&device.store)
        .by_id(&session.routine.note_id)?
        .ok_or("its Note")?;
    assert_eq!(note.text, "Morning sport");
    assert_eq!(session.setup.pool, vec![device.tallies[0].clone()]);
    assert!(session.routine.schedule.times.is_empty());
    // Its Note is no Unsorted one, as a new Routine's is not.
    let unsorted = Notes::new(&device.store).unsorted()?;
    assert!(unsorted.iter().all(|n| n.id != note.id));
    Ok(())
}

#[test]
fn with_no_tally_a_session_is_offered_shows_the_hint_and_save_makes_nothing() -> TestResult {
    let start = DateTime::parse_from_rfc3339("2026-03-02T12:00:00Z")?.with_timezone(&Utc);
    let store = Store::in_memory(Box::new(move || start), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let legs = Notes::new(&store).capture("Leg day", None, None)?.id;
    let store = Arc::new(store);
    let t = L10n::new("en");
    let mut harness = window(&store, Some(&legs));
    add_menu(&mut harness, &t);
    for offered in [t.routine(), t.reminder(), t.frog(), t.session()] {
        assert!(
            harness
                .query_all_by_role_and_label(Role::Button, offered)
                .next()
                .is_some(),
            "{offered}"
        );
    }
    click_button(&mut harness, t.session());
    assert!(shows(&harness, t.new_session()));
    assert!(shows(&harness, t.session_pool_none_desk()));
    click_button(&mut harness, t.save());
    assert!(shows(&harness, t.session_pool_missing()));
    assert!(Sessions::new(&store).all()?.is_empty());
    assert!(harness.state().routine_editor().is_some());

    let mut harness = window(&store, None);
    harness
        .get_by_label(t.all_routines_title())
        .click_accesskit();
    harness.run();
    click_button(&mut harness, t.new_session());
    assert!(shows(&harness, t.session_pool_none_desk()));
    harness
        .get_by(|n| n.placeholder() == Some(t.routine_what_hint()))
        .click();
    harness.run();
    harness
        .get_by(|n| n.is_focused())
        .type_text("Morning sport");
    harness.run();
    click_button(&mut harness, t.save());
    assert!(shows(&harness, t.session_pool_missing()));
    assert!(Sessions::new(&store).all()?.is_empty());
    let notes = Notes::new(&store);
    assert!(notes.all()?.iter().all(|n| n.text != "Morning sport"));
    Ok(())
}

#[test]
fn with_checked_items_hidden_a_pick_with_its_entry_leaves_and_then_the_heading() -> TestResult {
    let device = device()?;
    let id = spawn(&device)?;
    DeviceSettings::new(&device.store).set_agenda_hides_checked(true)?;
    let sessions = Sessions::new(&device.store);
    sessions.pick_today(&berlin)?;
    sessions.record(
        &id,
        &occurrence(),
        &device.tallies[0],
        &BTreeMap::new(),
        "done",
    )?;
    let mut harness = window(&device.store, None);
    // The pick with its entry is gone, the open one and the heading stay
    // (#217).
    assert!(!shows(&harness, "Push-ups 20 times"));
    assert!(shows(&harness, "Wall sit 30 times"));
    let headings = |harness: &Harness<'_, App>| {
        harness
            .query_all_by_role_and_label(Role::CheckBox, "Done: All day Sport")
            .count()
    };
    assert_eq!(headings(&harness), 7);
    sessions.record(
        &id,
        &occurrence(),
        &device.tallies[1],
        &BTreeMap::new(),
        "done",
    )?;
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    // Done, today's heading goes with its picks; the coming days stay.
    assert_eq!(headings(&harness), 6);
    assert!(!shows(&harness, "Wall sit 30 times"));
    Ok(())
}
