//! Checklists on the Agenda in the window (#143, #208, #219, ADR 0030):
//! today shows each Checklist without a Routine or Reminder that has open
//! Lines as a heading with its name, its open Lines indented under it,
//! each with its box, after the Shopping Lists and before the timed
//! Occurrences, oldest capture first; a click on a Line's words opens
//! it, on the heading with the keyboard in a new empty Line after the
//! open ones, which is dropped left unused (#276); a box checks its Line,
//! which stays struck for the day, its box unchecking it, unless this
//! Device hides what is checked (#276); once every Line is checked it is
//! gone the next day; "Show on the Agenda" in the card's ⋮ and in the editor offers
//! the three ways of a Shopping List for this person: off, its heading
//! with the count, or every open Line; a Checklist with a Routine has
//! neither the row nor the choice.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{
    Checklists, ClockTime, Day, DeviceSettings, ListOnAgenda, Notes, OwnProfile, Routines,
    Schedule, SigningKey, Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 07:00 on Saturday, 24 October 2026, the wall clock on UTC.
const NOW: &str = "2026-10-24T07:00:00Z";

/// The text of the Note every test packs with: three Lines, "Charger"
/// in the group of "Passport".
const PACKING: &str = "Packing\nPassport\n  Charger\nTicket";

/// The choice's words, and its three ways'.
const SETTING: &str = "Show on the Agenda";
const OFF: &str = "Off";
const COUNT: &str = "One row with the number of open Lines";
const LINES: &str = "Each open Line on its own row";

/// The Agenda's message for a day with nothing due.
const NOTHING_TODAY: &str = "Nothing due today. Click + to capture a thought.";

/// The seconds the clock of [`packing_on`] has moved from [`NOW`]; it
/// moves a second per read and can be set.
type Ticks = Arc<AtomicI64>;

/// A store on a clock that moves a second per read from [`NOW`], with
/// its own Profile, and the Checklist [`PACKING`]: the store and the
/// Note's id.
fn packing() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let (store, note, _) = packing_on()?;
    Ok((store, note))
}

/// As [`packing`], with the clock.
fn packing_on() -> Result<(Arc<Store>, String, Ticks), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339(NOW)?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let read = ticks.clone();
    let clock = Box::new(move || start + Duration::seconds(read.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([6; 32])))?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let note = Notes::new(&store).capture(PACKING, None, None)?;
    Checklists::new(&store).make(&note.id)?;
    Ok((Arc::new(store), note.id, ticks))
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

/// Whether the last radio button `label` shows chosen, or `None` while
/// none shows.
fn radio(harness: &Harness<'_, App>, label: &str) -> Option<bool> {
    harness
        .query_all_by_role_and_label(Role::RadioButton, label)
        .last()
        .map(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

/// The three ways as their radio buttons show them chosen: off, count,
/// lines; `None` for one not showing.
fn chosen(harness: &Harness<'_, App>) -> [Option<bool>; 3] {
    [OFF, COUNT, LINES].map(|way| radio(harness, way))
}

/// Clicks the last radio button `label`.
fn pick(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::RadioButton, label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Whether no switch "Show on the Agenda" shows, as #143 had.
fn no_switch(harness: &Harness<'_, App>) -> bool {
    harness
        .query_all_by_role_and_label(Role::CheckBox, SETTING)
        .next()
        .is_none()
}

/// Whether the Agenda shows the group of the Checklist `name`: its
/// heading, a button of its name with no box of a Routine's beside it.
fn group(harness: &Harness<'_, App>, name: &str) -> bool {
    let headings = harness
        .query_all_by_role_and_label(Role::Button, name)
        .count();
    let occurrences = harness
        .query_all_by_label_contains(&format!(" {name}"))
        .filter(|n| n.accesskit_node().role() == Role::CheckBox)
        .count();
    headings > occurrences
}

/// The box of the Line `text`.
fn line_box<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> Option<egui_kittest::Node<'a>> {
    harness
        .query_all_by_role_and_label(Role::CheckBox, text)
        .next()
}

/// Whether the box of the Line `text` shows checked, or `None` while it
/// does not show.
fn ticked(harness: &Harness<'_, App>, text: &str) -> Option<bool> {
    line_box(harness, text).map(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

/// Clicks the box of the Line `text` and reads the Agenda again.
fn tick(harness: &mut Harness<'_, App>, text: &str) -> TestResult {
    line_box(harness, text)
        .ok_or(format!("{text:?}"))?
        .click_accesskit();
    harness.run();
    reload_agenda(harness);
    Ok(())
}

/// The texts of the Lines of the Checklist of `note`, in their order,
/// each "x " in front while checked and "  " while in a group.
fn texts(store: &Store, note: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Checklists::new(store)
        .by_note(note)?
        .ok_or("a Checklist")?
        .lines
        .iter()
        .map(|l| {
            let group = if l.in_group() { "  " } else { "" };
            let checked = if l.checked { "x " } else { "" };
            format!("{group}{checked}{}", l.text)
        })
        .collect())
}

/// What the field with the keyboard holds.
fn focused(harness: &Harness<'_, App>) -> Option<String> {
    harness
        .query_all_by(|n| n.is_focused())
        .next()
        .and_then(|n| n.value())
}

/// The top of the Line field holding `value` in the Note dialog.
fn field_top(harness: &Harness<'_, App>, value: &str) -> Result<f32, Box<dyn std::error::Error>> {
    Ok(harness
        .query_all_by(|n| {
            n.role() == Role::TextInput
                && n.placeholder().unwrap_or_default().is_empty()
                && n.value().as_deref() == Some(value)
        })
        .next()
        .ok_or(format!("a field holding {value:?}"))?
        .rect()
        .top())
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
    assert!(group(&harness, "Packing"));
    assert!(!shows(&harness, NOTHING_TODAY));
    assert!(
        harness
            .query_all_by_label_contains("of 3 open")
            .next()
            .is_none()
    );
    // Only today: tomorrow has nothing planned.
    let headings = harness
        .query_all_by_role_and_label(Role::Button, "Packing")
        .count();
    assert_eq!(headings, 1);
    // Its open Lines under it, each with its box, indented; a Line in a
    // group one step further (#208).
    let heading = harness
        .get_by_role_and_label(Role::Button, "Packing")
        .rect();
    let passport = line_box(&harness, "Passport").ok_or("Passport")?.rect();
    let charger = line_box(&harness, "Charger").ok_or("Charger")?.rect();
    assert!(passport.top() > heading.top());
    assert!(passport.left() >= heading.left() - 0.5);
    assert!(charger.left() > passport.left());
    // The box checks its Line; the parent counts as it shows: checking
    // its group checks it. Both stay, struck for the day (#276).
    tick(&mut harness, "Charger")?;
    assert_eq!(
        texts(&store, &note)?,
        ["x Passport", "  x Charger", "Ticket"]
    );
    assert_eq!(ticked(&harness, "Charger"), Some(true));
    assert_eq!(ticked(&harness, "Passport"), Some(true));
    assert_eq!(ticked(&harness, "Ticket"), Some(false));
    harness
        .get_by_role_and_label(Role::Button, "Ticket")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Packing")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn a_line_checked_today_stays_struck_and_its_box_unchecks_it() -> TestResult {
    let (store, note) = packing()?;
    let mut harness = window(&store, Selection::Agenda, None);
    tick(&mut harness, "Ticket")?;
    assert_eq!(texts(&store, &note)?, ["Passport", "  Charger", "x Ticket"]);
    assert_eq!(ticked(&harness, "Ticket"), Some(true));
    // In its place, after the open ones above it.
    assert!(top(&harness, "Charger")? < top(&harness, "Ticket")?);
    // A wrong click is undone by one more.
    tick(&mut harness, "Ticket")?;
    assert_eq!(texts(&store, &note)?, ["Passport", "  Charger", "Ticket"]);
    assert_eq!(ticked(&harness, "Ticket"), Some(false));
    Ok(())
}

#[test]
fn every_line_checked_today_shows_struck_until_the_day_ends() -> TestResult {
    let (store, note, ticks) = packing_on()?;
    let lists = Checklists::new(&store);
    lists.check(&line_id(&store, &note, "Charger")?)?;
    lists.check(&line_id(&store, &note, "Ticket")?)?;
    let mut harness = window(&store, Selection::Agenda, None);
    assert!(group(&harness, "Packing"));
    for line in ["Passport", "Charger", "Ticket"] {
        assert_eq!(ticked(&harness, line), Some(true), "{line}");
    }
    // Shown as its count, none open, it does not show.
    lists.set_on_agenda(&note, ListOnAgenda::Count)?;
    reload_agenda(&mut harness);
    assert!(!shows(&harness, "Packing: 0 of 3 open"));
    assert!(shows(&harness, NOTHING_TODAY));
    lists.set_on_agenda(&note, ListOnAgenda::Lines)?;
    // The next day it is gone until a Line is unchecked.
    ticks.fetch_add(86_400, Ordering::SeqCst);
    reload_agenda(&mut harness);
    assert!(!group(&harness, "Packing"));
    assert!(shows(&harness, NOTHING_TODAY));
    lists.uncheck(&line_id(&store, &note, "Ticket")?)?;
    reload_agenda(&mut harness);
    assert!(group(&harness, "Packing"));
    assert_eq!(ticked(&harness, "Ticket"), Some(false));
    assert!(line_box(&harness, "Charger").is_none());
    Ok(())
}

#[test]
fn hiding_whats_checked_takes_a_checked_line_off_at_once() -> TestResult {
    let (store, note) = packing()?;
    DeviceSettings::new(&store).set_agenda_hides_checked(true)?;
    let mut harness = window(&store, Selection::Agenda, None);
    tick(&mut harness, "Ticket")?;
    assert_eq!(texts(&store, &note)?, ["Passport", "  Charger", "x Ticket"]);
    assert!(line_box(&harness, "Ticket").is_none());
    assert_eq!(ticked(&harness, "Passport"), Some(false));
    tick(&mut harness, "Charger")?;
    assert!(!group(&harness, "Packing"));
    assert!(shows(&harness, NOTHING_TODAY));
    Ok(())
}

#[test]
fn the_heading_opens_the_list_in_a_new_line_after_the_open_ones() -> TestResult {
    let (store, note) = packing()?;
    let other = Notes::new(&store).capture("Other", None, None)?.id;
    Checklists::new(&store).check(&line_id(&store, &note, "Ticket")?)?;
    let mut harness = window(&store, Selection::Agenda, None);
    harness
        .get_by_role_and_label(Role::Button, "Packing")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    // The keyboard in an empty Line after the open ones and their group,
    // before the checked one.
    assert_eq!(focused(&harness).as_deref(), Some(""));
    let new = field_top(&harness, "")?;
    assert!(field_top(&harness, "Charger")? < new);
    assert!(new < field_top(&harness, "Ticket")?);
    harness.get_by(|n| n.is_focused()).type_text("Socks");
    harness.run();
    harness.state_mut().select_note(&other);
    harness.run();
    assert_eq!(
        texts(&store, &note)?,
        ["Passport", "  Charger", "Socks", "x Ticket"]
    );
    // Left unused, by back too, it is dropped.
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Packing")
        .click();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some(""));
    // The mouse's back button; Alt+Left rests while a Line is typed in.
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: egui::Pos2::new(640.0, 700.0),
            button: egui::PointerButton::Extra1,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(harness.state().selected_note(), None);
    assert!(group(&harness, "Packing"));
    assert_eq!(
        texts(&store, &note)?,
        ["Passport", "  Charger", "Socks", "x Ticket"]
    );
    // A Line's words open the list as it is.
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Socks").click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    assert_eq!(focused(&harness), None);
    assert!(field_top(&harness, "").is_err());
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
    let packing = top(&harness, "Packing")?;
    let errands = top(&harness, "Errands")?;
    assert!(top(&harness, "Water the plants")? < packing);
    assert!(packing < errands);
    assert!(errands < top(&harness, "Call Dana")?);
    Ok(())
}

#[test]
fn shown_as_its_count_it_is_its_heading_with_how_many_are_open() -> TestResult {
    let (store, note) = packing()?;
    let lists = Checklists::new(&store);
    lists.set_on_agenda(&note, ListOnAgenda::Count)?;
    let mut harness = window(&store, Selection::Agenda, None);
    let heading = "Packing: 3 of 3 open";
    assert!(shows(&harness, heading));
    assert!(line_box(&harness, "Passport").is_none());
    assert!(line_box(&harness, "Ticket").is_none());
    harness.get_by_role_and_label(Role::Button, heading).click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    // Without an open Line it never shows.
    lists.check(&line_id(&store, &note, "Passport")?)?;
    lists.check(&line_id(&store, &note, "Ticket")?)?;
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    reload_agenda(&mut harness);
    assert!(shows(&harness, NOTHING_TODAY));
    Ok(())
}

#[test]
fn the_cards_menu_offers_the_three_ways() -> TestResult {
    let (store, note) = packing()?;
    // A Line checked, so "Uncheck all" shows after the choice.
    Checklists::new(&store).check(&line_id(&store, &note, "Ticket")?)?;
    let mut harness = window(&store, Selection::All, None);
    click(&mut harness, "More: Packing")?;
    assert!(no_switch(&harness));
    assert_eq!(chosen(&harness), [Some(false), Some(false), Some(true)]);
    let below = top(&harness, "Uncheck all")?;
    let lines_top = harness
        .query_all_by_role_and_label(Role::RadioButton, LINES)
        .last()
        .ok_or("the choice")?
        .rect()
        .top();
    assert!(lines_top < below);
    pick(&mut harness, COUNT)?;
    let lists = Checklists::new(&store);
    assert_eq!(lists.on_agenda(&note)?, ListOnAgenda::Count);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(shows(&harness, "Packing: 2 of 3 open"));
    assert!(line_box(&harness, "Passport").is_none());
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Packing")?;
    assert_eq!(chosen(&harness), [Some(false), Some(true), Some(false)]);
    pick(&mut harness, OFF)?;
    assert_eq!(lists.on_agenda(&note)?, ListOnAgenda::Off);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!group(&harness, "Packing"));
    assert!(shows(&harness, NOTHING_TODAY));
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Packing")?;
    pick(&mut harness, LINES)?;
    assert_eq!(lists.on_agenda(&note)?, ListOnAgenda::Lines);
    Ok(())
}

#[test]
fn the_editors_badge_names_it_and_chooses_it() -> TestResult {
    let (store, note) = packing()?;
    let mut harness = window(&store, Selection::All, Some(&note));
    // The Note dialog's badge for it (#160), its ways in its editor.
    let badge = format!("{SETTING}: {LINES}");
    assert!(harness.query_by_label(&badge).is_some());
    click(&mut harness, &badge)?;
    assert!(no_switch(&harness));
    assert_eq!(radio(&harness, LINES), Some(true));
    pick(&mut harness, OFF)?;
    assert_eq!(Checklists::new(&store).on_agenda(&note)?, ListOnAgenda::Off);
    assert!(
        harness
            .query_by_label(&format!("{SETTING}: {OFF}"))
            .is_some()
    );
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!group(&harness, "Packing"));
    Ok(())
}

#[test]
fn a_checklist_with_a_routine_has_no_row_and_no_choice() -> TestResult {
    let (store, note) = packing()?;
    let schedule = Schedule::new(Day::new(2026, 10, 24), &[], None).ok_or("a schedule")?;
    Routines::new(&store).spawn(&note, &schedule)?;
    let mut harness = window(&store, Selection::Agenda, None);
    assert!(!group(&harness, "Packing"));
    // It shows as its Routine's Occurrence instead.
    assert!(shows(&harness, "Packing"));
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Packing")?;
    assert_eq!(chosen(&harness), [None; 3]);
    harness.key_press(egui::Key::Escape);
    harness.run();
    harness.state_mut().select_note(&note);
    harness.run();
    assert_eq!(chosen(&harness), [None; 3]);
    assert!(
        harness
            .query_all_by_label_contains("Show on the Agenda")
            .next()
            .is_none()
    );
    Ok(())
}

#[test]
fn a_plain_note_has_no_choice() -> TestResult {
    let (store, _) = packing()?;
    let plain = Notes::new(&store).capture("Call the bank", None, None)?;
    let mut harness = window(&store, Selection::All, Some(&plain.id));
    assert_eq!(chosen(&harness), [None; 3]);
    click(&mut harness, "More: Call the bank")?;
    assert_eq!(chosen(&harness), [None; 3]);
    Ok(())
}
