//! The Areas in the person's own order on the desktop (#280): the
//! sidebar lists them by the name it shows until one is moved; an Area's
//! row dragged up or down puts it there, a drop beside where it stands
//! writing nothing; Alt+Up and Alt+Down and Move up and Move down, in the
//! Area menu and the right-click, move the Area shown one place, also a
//! Shared one a Member is in; a card dropped on an Area's row still files
//! it; and File into…, in the window and the capture popup, and Link
//! to… list the Areas in that order.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, DeviceSettings, Invite, Notes, OwnProfile, PairedDevice, PersonSettings, Places,
    QuickNoteControl, SigningKey, Store,
};
use oldbrainfix_gui::app_links::{self, Group};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::popup::CapturePopup;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read, with its own
/// Profile named `name`.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

const MILK: &str = "Buy milk";

/// Work, Garden and Household, made in that order, Household with its
/// Topic Kitchen, and the Unsorted Note "Buy milk".
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let store = device(1, "Ann")?;
    let places = Places::new(&store);
    places.add_area("Work")?;
    places.add_area("Garden")?;
    let house = places.add_area("Household")?;
    places.add_topic(&house.id, "Kitchen")?;
    Notes::new(&store).capture(MILK, None, None)?;
    Ok(store)
}

fn window(store: Store, language: &str, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, language);
    app.select(selection);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
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

/// The id of the Area named `name`.
fn area(harness: &Harness<'_, App>, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .find(|a| a.name == name)
        .map(|a| a.id)
        .ok_or_else(|| format!("no Area {name:?}").into())
}

/// The sidebar's row of `name`, drawn first.
fn row<'a>(harness: &'a Harness<'_, App>, name: &'a str) -> Node<'a> {
    match harness.query_all_by_label(name).next() {
        Some(node) => node,
        None => panic!("no row {name}"),
    }
}

/// `names` top to bottom, as the sidebar shows them.
fn sidebar<const N: usize>(
    harness: &Harness<'_, App>,
    mut names: [&'static str; N],
) -> [&'static str; N] {
    names.sort_by(|a, b| {
        let (a, b) = (row(harness, a).rect().top(), row(harness, b).rect().top());
        a.total_cmp(&b)
    });
    names
}

/// The Areas' stored names in the person's own order, as the core keeps
/// it.
fn kept(harness: &Harness<'_, App>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(PersonSettings::new(harness.state().store())
        .areas_in_order()?
        .into_iter()
        .map(|a| a.name)
        .collect())
}

/// Lets egui's time for a double-click run out.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// Presses at `from`, moves a little, then to `to`, and lets go there,
/// as a hand drags.
fn drag(harness: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    pause(harness);
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(0.0, 8.0));
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

/// Just inside the top of `name`'s row: the gap above it.
fn above(harness: &Harness<'_, App>, name: &str) -> Pos2 {
    let rect = row(harness, name).rect();
    egui::pos2(rect.center().x, rect.top() + 4.0)
}

/// Just inside the bottom of `name`'s row: the gap below it.
fn below(harness: &Harness<'_, App>, name: &str) -> Pos2 {
    let rect = row(harness, name).rect();
    egui::pos2(rect.center().x, rect.bottom() - 4.0)
}

/// The last widget labelled `label`: a menu's entry, drawn over the
/// window's widgets of the same name.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

/// The menu bar's Area menu, opened.
fn open_area_menu(harness: &mut Harness<'_, App>) {
    match harness.get_all_by_label("Area").next() {
        Some(node) => node.click(),
        None => panic!("no menu Area"),
    }
    harness.run();
}

/// Whether Move up and Move down of the Area menu can be picked; the
/// menu is closed again.
fn moves(harness: &mut Harness<'_, App>) -> [bool; 2] {
    open_area_menu(harness);
    let on = ["Move up Alt+Up", "Move down Alt+Down"]
        .map(|entry| !last(harness, entry).accesskit_node().is_disabled());
    harness.key_press(Key::Escape);
    harness.run();
    on
}

#[test]
fn the_sidebar_lists_the_areas_by_the_name_shown_until_one_moves() -> TestResult {
    let harness = window(store()?, "en", Selection::All);
    assert_eq!(
        sidebar(&harness, ["Work", "Garden", "Household"]),
        ["Garden", "Household", "Work"]
    );
    // A starter Area by its translated name: Haushalt after Arbeit and
    // Auto, though Household is stored.
    let store = device(1, "Ann")?;
    Places::new(&store).plant_starter_set(&["household", "work", "car"])?;
    let harness = window(store, "de", Selection::All);
    assert_eq!(
        sidebar(&harness, ["Haushalt", "Arbeit", "Auto"]),
        ["Arbeit", "Auto", "Haushalt"]
    );
    Ok(())
}

#[test]
fn an_area_dragged_by_its_row_goes_where_it_is_let_go() -> TestResult {
    let mut harness = window(store()?, "en", Selection::All);
    // Up: Work above Garden.
    let (from, to) = (
        row(&harness, "Work").rect().center(),
        above(&harness, "Garden"),
    );
    drag(&mut harness, from, to);
    assert_eq!(
        sidebar(&harness, ["Work", "Garden", "Household"]),
        ["Work", "Garden", "Household"]
    );
    assert_eq!(kept(&harness)?, ["Work", "Garden", "Household"]);
    // Down: Work below Household, over its Topic too.
    let (from, to) = (
        row(&harness, "Work").rect().center(),
        below(&harness, "Kitchen"),
    );
    drag(&mut harness, from, to);
    assert_eq!(kept(&harness)?, ["Garden", "Household", "Work"]);
    // One place down, between two Areas.
    let (from, to) = (
        row(&harness, "Garden").rect().center(),
        below(&harness, "Kitchen"),
    );
    drag(&mut harness, from, to);
    assert_eq!(kept(&harness)?, ["Household", "Garden", "Work"]);
    // The drag selects nothing; the list shows what it showed.
    assert_eq!(harness.state().selection(), &Selection::All);
    // Once moved, a new Area goes to the end.
    Places::new(harness.state().store()).add_area("Attic")?;
    assert_eq!(kept(&harness)?, ["Household", "Garden", "Work", "Attic"]);
    Ok(())
}

#[test]
fn a_drop_beside_where_the_area_stands_writes_nothing() -> TestResult {
    let mut harness = window(store()?, "en", Selection::All);
    let from = row(&harness, "Household").rect().center();
    for to in [above(&harness, "Household"), below(&harness, "Kitchen")] {
        drag(&mut harness, from, to);
    }
    // Dropped beside the sidebar, it stays too.
    let to = row(&harness, "Garden").rect().center() + egui::vec2(600.0, 0.0);
    drag(&mut harness, from, to);
    // No order was kept: a new Area still sorts in by name.
    Places::new(harness.state().store()).add_area("Attic")?;
    assert_eq!(kept(&harness)?, ["Attic", "Garden", "Household", "Work"]);
    Ok(())
}

#[test]
fn alt_up_and_alt_down_move_the_area_shown_but_not_past_an_end() -> TestResult {
    let mut harness = window(store()?, "en", Selection::All);
    let garden = area(&harness, "Garden")?;
    // A card in Garden, which the list's Up and Down would select.
    Notes::new(harness.state().store()).capture("Cut the hedge", Some(&garden), None)?;
    harness.state_mut().select(Selection::Place(garden.clone()));
    harness.run();
    // First already: it stays, and nothing is kept; no card is stepped
    // to.
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.run();
    assert!(harness.state().selected_cards().is_empty());
    Places::new(harness.state().store()).add_area("Attic")?;
    assert_eq!(kept(&harness)?, ["Attic", "Garden", "Household", "Work"]);
    // Read again, Attic first.
    harness.state_mut().select(Selection::Place(garden));
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run();
    assert_eq!(kept(&harness)?, ["Attic", "Household", "Garden", "Work"]);
    assert!(harness.state().selected_cards().is_empty());
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.run();
    assert_eq!(kept(&harness)?, ["Garden", "Attic", "Household", "Work"]);
    assert_eq!(
        sidebar(&harness, ["Work", "Garden", "Household", "Attic"]),
        ["Garden", "Attic", "Household", "Work"]
    );
    Ok(())
}

#[test]
fn alt_up_moves_nothing_in_a_text_field() -> TestResult {
    let mut harness = window(store()?, "en", Selection::All);
    let work = area(&harness, "Work")?;
    harness.state_mut().select(Selection::Place(work));
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.run();
    Places::new(harness.state().store()).add_area("Attic")?;
    assert_eq!(kept(&harness)?, ["Attic", "Garden", "Household", "Work"]);
    Ok(())
}

#[test]
fn the_area_menu_moves_the_area_shown_greyed_at_an_end_and_for_a_topic() -> TestResult {
    let mut harness = window(store()?, "en", Selection::All);
    // No Area shows.
    assert_eq!(moves(&mut harness), [false, false]);
    let garden = area(&harness, "Garden")?;
    harness.state_mut().select(Selection::Place(garden));
    harness.run();
    assert_eq!(moves(&mut harness), [false, true]);
    open_area_menu(&mut harness);
    last(&harness, "Move down Alt+Down").click_accesskit();
    harness.run();
    assert_eq!(kept(&harness)?, ["Household", "Garden", "Work"]);
    // The pick closed the menu.
    assert!(harness.query_by_label("Move down Alt+Down").is_none());
    let work = area(&harness, "Work")?;
    harness.state_mut().select(Selection::Place(work));
    harness.run();
    assert_eq!(moves(&mut harness), [true, false]);
    // A Topic stays by name.
    let house = area(&harness, "Household")?;
    let kitchen = Places::new(harness.state().store()).topics(&house)?;
    harness
        .state_mut()
        .select(Selection::Place(kitchen[0].id.clone()));
    harness.run();
    assert_eq!(moves(&mut harness), [false, false]);
    Ok(())
}

#[test]
fn the_right_click_moves_the_area_and_leaves_out_the_way_past_an_end() -> TestResult {
    let mut harness = window(store()?, "en", Selection::All);
    row(&harness, "Garden").click_secondary();
    harness.run();
    assert!(harness.query_by_label("Move up").is_none());
    last(&harness, "Move down").click();
    harness.run();
    assert_eq!(kept(&harness)?, ["Household", "Garden", "Work"]);
    row(&harness, "Work").click_secondary();
    harness.run();
    assert!(harness.query_by_label("Move down").is_none());
    last(&harness, "Move up").click();
    harness.run();
    assert_eq!(kept(&harness)?, ["Household", "Work", "Garden"]);
    Ok(())
}

/// Ann's Household shared with Ben as a Member, and Ben's own Area
/// Garden: Ben's store.
fn shared() -> Result<Store, Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let invite = Invite {
        area,
        role: AreaRole::Member,
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    ben.remember_device(&PairedDevice {
        id: ann.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: ben.now(),
        shared: true,
    })?;
    Places::new(&ben).add_area("Garden")?;
    Ok(ben)
}

#[test]
fn a_member_moves_a_shared_area_in_their_own_order() -> TestResult {
    let mut harness = window(shared()?, "en", Selection::All);
    let house = area(&harness, "Household")?;
    harness.state_mut().select(Selection::Place(house));
    harness.run();
    assert_eq!(moves(&mut harness), [true, false]);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.run();
    assert_eq!(kept(&harness)?, ["Household", "Garden"]);
    Ok(())
}

#[test]
fn a_card_dropped_on_an_areas_row_is_filed_and_moves_no_area() -> TestResult {
    let mut harness = window(store()?, "en", Selection::Unsorted);
    let from = harness
        .get_by_role_and_label(Role::Button, MILK)
        .rect()
        .center();
    let to = row(&harness, "Work").rect().center();
    drag(&mut harness, from, to);
    let work = area(&harness, "Work")?;
    let milk = Notes::new(harness.state().store())
        .all()?
        .into_iter()
        .find(|n| n.text == MILK)
        .ok_or("the Note")?;
    assert_eq!(milk.place, Some(work));
    Places::new(harness.state().store()).add_area("Attic")?;
    assert_eq!(kept(&harness)?, ["Attic", "Garden", "Household", "Work"]);
    Ok(())
}

#[test]
fn file_into_lists_the_areas_in_the_persons_order() -> TestResult {
    let store = store()?;
    let work = Places::new(&store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Work")
        .ok_or("Work")?;
    PersonSettings::new(&store).set_area_order(&[work.id])?;
    let mut harness = window(store, "en", Selection::Unsorted);
    let card = harness.get_by_role_and_label(Role::Button, MILK);
    card.click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
    assert!(harness.query_by_label("File into…").is_some());
    // The picker's rows, drawn after the sidebar's.
    let top = |name: &str| last(&harness, name).rect().top();
    assert!(top("Work") < top("Garden"));
    assert!(top("Garden") < top("Household"));
    Ok(())
}

#[test]
fn link_to_lists_the_areas_in_the_persons_order() -> TestResult {
    let store = store()?;
    let t = L10n::new("en");
    let work = Places::new(&store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Work")
        .ok_or("Work")?;
    PersonSettings::new(&store).set_area_order(&[work.id])?;
    let places: Vec<String> = app_links::targets(&store, &t, false)?
        .into_iter()
        .filter(|target| target.group == Group::Places)
        .map(|target| target.label)
        .collect();
    assert_eq!(
        places,
        ["Work", "Garden", "Household", "Household ▸ Kitchen"]
    );
    Ok(())
}

#[test]
fn the_capture_popups_file_into_lists_the_areas_in_the_persons_order() -> TestResult {
    let store = store()?;
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    let work = Places::new(&store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Work")
        .ok_or("Work")?;
    PersonSettings::new(&store).set_area_order(&[work.id])?;
    let popup = CapturePopup::new(Arc::new(store), "en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 900.0))
        .build_ui_state(
            |ui, popup: &mut CapturePopup| {
                App::install(ui.ctx());
                popup.show(ui);
            },
            popup,
        );
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Buy bread");
    harness.run();
    harness
        .query_all_by_role_and_label(Role::Button, "File into…")
        .next_back()
        .ok_or("no File into…")?
        .click_accesskit();
    harness.run();
    assert!(harness.state().page_open());
    let top = |name: &str| harness.get_by_label(name).rect().top();
    assert!(top("Work") < top("Garden"));
    assert!(top("Garden") < top("Household"));
    Ok(())
}
