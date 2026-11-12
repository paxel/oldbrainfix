//! Moving cards into Areas and Topics on the desktop (#148): a click
//! selects a card, Ctrl-click and Shift-click select more, Esc and
//! another place drop the selection; one card or the cards selected go
//! together by a drag onto the sidebar, by the right-click's File into…
//! and by Ctrl+M, and a card this Device may not move stays, the
//! announcement's details saying how many did.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui::{Event, Key, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, DeviceSettings, Invite, Notes, OwnProfile, PairedDevice, Places, SigningKey, Store,
};
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
const PLUMBER: &str = "Call the plumber";
const ROSES: &str = "Water the roses";

/// Household with its Topic Kitchen, Garden, and three Unsorted Notes.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let store = device(1, "Ann")?;
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    places.add_topic(&house.id, "Kitchen")?;
    places.add_area("Garden")?;
    let notes = Notes::new(&store);
    for text in [MILK, PLUMBER, ROSES] {
        notes.capture(text, None, None)?;
    }
    Ok(store)
}

fn harness(store: Store, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
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

/// Lets egui's time for a double-click run out, so the next click is a
/// click of its own.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// The card of the Note `text`.
fn card<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, text)
}

fn has_card(harness: &Harness<'_, App>, text: &str) -> bool {
    harness
        .query_by_role_and_label(Role::Button, text)
        .is_some()
}

/// Whether the card of `text` tells a screen reader it is selected.
fn marked(harness: &Harness<'_, App>, text: &str) -> bool {
    card(harness, text).accesskit_node().toggled() == Some(Toggled::True)
}

/// Clicks the card of `text` with `modifiers`, as Ctrl-click or
/// Shift-click, after a pause.
fn click(harness: &mut Harness<'_, App>, text: &str, modifiers: Modifiers) {
    pause(harness);
    card(harness, text).click_modifiers(modifiers);
    harness.run();
}

/// The id of the Note with `text`.
fn id(harness: &Harness<'_, App>, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Notes::new(harness.state().store())
        .all()?
        .into_iter()
        .find(|n| n.text == text)
        .map(|n| n.id)
        .ok_or_else(|| format!("no Note {text:?}").into())
}

/// The ids of the Notes with these texts, in the order given.
fn ids(
    harness: &Harness<'_, App>,
    texts: &[&str],
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    texts.iter().map(|t| id(harness, t)).collect()
}

/// Where the Note with `text` is filed.
fn place_of(
    harness: &Harness<'_, App>,
    text: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let id = id(harness, text)?;
    Ok(Notes::new(harness.state().store())
        .by_id(&id)?
        .and_then(|n| n.place))
}

/// The id of the Area or Topic named `name`.
fn place(harness: &Harness<'_, App>, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let places = Places::new(harness.state().store());
    for area in places.areas()? {
        if area.name == name {
            return Ok(area.id);
        }
        for topic in places.topics(&area.id)? {
            if topic.name == name {
                return Ok(topic.id);
            }
        }
    }
    Err(format!("no place {name:?}").into())
}

/// The texts of the three cards top to bottom, as the list shows them.
fn shown_order(harness: &Harness<'_, App>) -> Vec<&'static str> {
    let mut texts = [MILK, PLUMBER, ROSES];
    texts.sort_by(|a, b| {
        let (a, b) = (card(harness, a).rect().top(), card(harness, b).rect().top());
        a.total_cmp(&b)
    });
    texts.to_vec()
}

/// Drags the card of `text` onto the sidebar's row `onto`, as a hand
/// does: pressed, moved a little, over the row, let go.
fn drag_onto(harness: &mut Harness<'_, App>, text: &str, onto: &str) {
    pause(harness);
    let from = card(harness, text).rect().center();
    let onto = harness
        .get_by_role_and_label(Role::Button, onto)
        .rect()
        .center();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(20.0, 0.0));
    harness.run();
    harness.hover_at(onto);
    harness.run();
    harness.event(Event::PointerButton {
        pos: onto,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

/// Picks `name` in the File into… dialog, drawn after the sidebar's row
/// of the same name.
fn pick_place(harness: &mut Harness<'_, App>, name: &str) -> TestResult {
    harness
        .query_all_by_label(name)
        .next_back()
        .ok_or_else(|| format!("no {name:?} to pick"))?
        .click();
    harness.run();
    Ok(())
}

#[test]
fn a_click_selects_ctrl_and_shift_select_more_and_esc_clears() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    let order = shown_order(&harness);
    let [top, middle, bottom] = [order[0], order[1], order[2]];
    assert!(harness.state().selected_cards().is_empty());

    click(&mut harness, top, Modifiers::NONE);
    assert_eq!(harness.state().selected_cards(), ids(&harness, &[top])?);
    assert!(marked(&harness, top));
    assert!(!marked(&harness, bottom));
    // A click selects and opens nothing.
    assert_eq!(harness.state().selected_note(), None);

    // Ctrl-click adds a card, and takes one away again.
    click(&mut harness, bottom, Modifiers::COMMAND);
    assert_eq!(
        harness.state().selected_cards(),
        ids(&harness, &[top, bottom])?
    );
    assert!(marked(&harness, bottom));
    click(&mut harness, top, Modifiers::COMMAND);
    assert_eq!(harness.state().selected_cards(), ids(&harness, &[bottom])?);
    assert!(!marked(&harness, top));

    // Shift-click selects the run from the card clicked last, the one
    // just taken away, in the order the list shows.
    click(&mut harness, bottom, Modifiers::SHIFT);
    assert_eq!(
        harness.state().selected_cards(),
        ids(&harness, &[top, middle, bottom])?
    );
    // A plain click selects one card alone again.
    click(&mut harness, middle, Modifiers::NONE);
    assert_eq!(harness.state().selected_cards(), ids(&harness, &[middle])?);

    // Esc drops the selection.
    click(&mut harness, top, Modifiers::COMMAND);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().selected_cards().is_empty());
    assert!(!marked(&harness, top));

    // So does showing another place.
    click(&mut harness, top, Modifiers::NONE);
    assert!(!harness.state().selected_cards().is_empty());
    harness.get_by_label("All Notes").click();
    harness.run();
    assert!(harness.state().selected_cards().is_empty());
    Ok(())
}

#[test]
fn an_unsorted_card_dragged_onto_a_topic_is_filed_there() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    drag_onto(&mut harness, MILK, "Kitchen");
    let kitchen = place(&harness, "Kitchen")?;
    assert_eq!(place_of(&harness, MILK)?, Some(kitchen.clone()));
    // A drag files into a place as File into… does: it is filed into
    // last (#272).
    let recent = DeviceSettings::new(harness.state().store()).recently_filed_into()?;
    assert_eq!(recent, [kitchen]);
    // It left Unsorted; the others did not move.
    assert!(!has_card(&harness, MILK));
    assert_eq!(place_of(&harness, PLUMBER)?, None);
    assert_eq!(place_of(&harness, ROSES)?, None);
    Ok(())
}

#[test]
fn the_cards_selected_are_dragged_together() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    click(&mut harness, MILK, Modifiers::NONE);
    click(&mut harness, ROSES, Modifiers::COMMAND);
    // Held, the drag says how many go.
    pause(&mut harness);
    let from = card(&harness, ROSES).rect().center();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(20.0, 0.0));
    harness.run();
    assert!(harness.query_by_label("2 Notes").is_some());
    let onto = harness
        .get_by_role_and_label(Role::Button, "Garden")
        .rect()
        .center();
    harness.hover_at(onto);
    harness.run();
    harness.event(Event::PointerButton {
        pos: onto,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let garden = place(&harness, "Garden")?;
    assert_eq!(place_of(&harness, MILK)?, Some(garden.clone()));
    assert_eq!(place_of(&harness, ROSES)?, Some(garden));
    assert_eq!(place_of(&harness, PLUMBER)?, None);
    assert!(harness.state().selected_cards().is_empty());

    // A card outside the selection is dragged alone.
    let mut harness = self::harness(store()?, Selection::Unsorted);
    click(&mut harness, MILK, Modifiers::NONE);
    drag_onto(&mut harness, PLUMBER, "Garden");
    assert_eq!(
        place_of(&harness, PLUMBER)?,
        Some(place(&harness, "Garden")?)
    );
    assert_eq!(place_of(&harness, MILK)?, None);
    Ok(())
}

#[test]
fn file_into_from_the_right_click_files_the_cards_selected() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    click(&mut harness, MILK, Modifiers::NONE);
    click(&mut harness, PLUMBER, Modifiers::COMMAND);
    pause(&mut harness);
    card(&harness, PLUMBER).click_secondary();
    harness.run();
    harness.get_by_label("File into…").click();
    harness.run_steps(8);
    // The dialog names how many go.
    assert!(harness.query_by_label("File 2 Notes into…").is_some());
    pick_place(&mut harness, "Household")?;
    let house = place(&harness, "Household")?;
    assert_eq!(place_of(&harness, MILK)?, Some(house.clone()));
    assert_eq!(place_of(&harness, PLUMBER)?, Some(house));
    assert_eq!(place_of(&harness, ROSES)?, None);
    assert!(!has_card(&harness, MILK));
    assert!(has_card(&harness, ROSES));
    Ok(())
}

#[test]
fn ctrl_m_and_the_note_menu_file_the_cards_selected() -> TestResult {
    let mut harness = harness(store()?, Selection::Unsorted);
    let order = shown_order(&harness);
    click(&mut harness, order[0], Modifiers::NONE);
    click(&mut harness, order[1], Modifiers::SHIFT);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
    assert!(harness.query_by_label("File 2 Notes into…").is_some());
    pick_place(&mut harness, "Kitchen")?;
    let kitchen = place(&harness, "Kitchen")?;
    assert_eq!(place_of(&harness, order[0])?, Some(kitchen.clone()));
    assert_eq!(place_of(&harness, order[1])?, Some(kitchen));
    assert_eq!(place_of(&harness, order[2])?, None);

    // Note ▸ File into… acts on the card selected, no Note open.
    click(&mut harness, order[2], Modifiers::NONE);
    // The menu bar's Note is the first of that name.
    harness
        .query_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    harness
        .query_all_by_label_contains("File into…")
        .next_back()
        .ok_or("File into… in the Note menu")?
        .click();
    harness.run_steps(8);
    assert!(harness.query_by_label("File into…").is_some());
    pick_place(&mut harness, "Garden")?;
    assert_eq!(
        place_of(&harness, order[2])?,
        Some(place(&harness, "Garden")?)
    );
    Ok(())
}

/// Ann's Household with her Note in it, shared with Ben as a Member, and
/// Ben's own Area and Unsorted Note: Ben's store.
fn shared() -> Result<Store, Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    Notes::new(&ann).capture(MILK, Some(&house.id), None)?;
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
    Notes::new(&ben).capture(ROSES, None, None)?;
    Ok(ben)
}

#[test]
fn a_card_this_device_may_not_move_stays_and_the_details_say_how_many() -> TestResult {
    let mut harness = harness(shared()?, Selection::All);
    let house = place(&harness, "Household")?;
    click(&mut harness, MILK, Modifiers::NONE);
    click(&mut harness, ROSES, Modifiers::COMMAND);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
    assert!(harness.query_by_label("File 2 Notes into…").is_some());
    pick_place(&mut harness, "Garden")?;
    // Ben's own Note moved; Ann's stayed in her Shared Area.
    assert_eq!(place_of(&harness, ROSES)?, Some(place(&harness, "Garden")?));
    assert_eq!(place_of(&harness, MILK)?, Some(house));
    // Said without numbers; the click opens them.
    assert_eq!(harness.state().notice(), Some("Not every Note was filed."));
    harness.get_by_label("Not every Note was filed.").click();
    harness.run_steps(8);
    assert!(
        harness
            .query_by_label_contains("Filed: 1. Stayed where they were: 1.")
            .is_some()
    );
    Ok(())
}

#[test]
fn file_into_offers_new_topic_only_where_this_device_may_set_one_up() -> TestResult {
    let mut harness = harness(shared()?, Selection::All);
    click(&mut harness, ROSES, Modifiers::NONE);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
    // Under Ben's Garden, not under Ann's Household, where he is a
    // Member (#272).
    let new_topics: Vec<_> = harness
        .query_all_by_role_and_label(Role::Button, "New Topic…")
        .map(|n| n.rect().top())
        .collect();
    assert_eq!(new_topics.len(), 1);
    let garden = harness
        .query_all_by_role_and_label(Role::Button, "Garden")
        .last()
        .ok_or("the dialog's Garden")?
        .rect()
        .top();
    let household = harness
        .query_all_by_role_and_label(Role::Button, "Household")
        .last()
        .ok_or("the dialog's Household")?
        .rect()
        .top();
    assert!(garden < new_topics[0] && new_topics[0] < household);
    Ok(())
}
