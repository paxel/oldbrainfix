//! What the cards selected offer on the desktop (#273): from a card's
//! right-click and the Note menu, Merge into…, Importance and Move to
//! trash act on the selection together, or on the card alone outside
//! it, and Restore in the trash; one Ctrl+Z takes back the trash or the
//! Merge of all of them, and a Note this Device may not change stays.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::Modifiers;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, ClockTime, Day, Importance, Invite, Notes, OwnProfile, PairedDevice, Places,
    Routines, Schedule, SigningKey, Store,
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

/// Three Unsorted Notes, the plumber's with a Reminder.
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let store = device(1, "Ann")?;
    let notes = Notes::new(&store);
    for text in [MILK, PLUMBER, ROSES] {
        let note = notes.capture(text, None, None)?;
        if text == PLUMBER {
            let times: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
            let once = Schedule::new(Day::new(2026, 3, 2), &times, None).ok_or("a schedule")?;
            Routines::new(&store).spawn(&note.id, &once)?;
        }
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

/// Clicks the card of `text` with `modifiers` after a pause.
fn click(harness: &mut Harness<'_, App>, text: &str, modifiers: Modifiers) {
    pause(harness);
    card(harness, text).click_modifiers(modifiers);
    harness.run();
}

/// Selects the cards of `texts`: the first clicked, the others
/// Ctrl-clicked.
fn select(harness: &mut Harness<'_, App>, texts: &[&str]) {
    for (i, text) in texts.iter().enumerate() {
        let modifiers = if i == 0 {
            Modifiers::NONE
        } else {
            Modifiers::COMMAND
        };
        click(harness, text, modifiers);
    }
}

/// Right-clicks the card of `text` and picks `entry` in its menu.
fn right_pick(harness: &mut Harness<'_, App>, text: &str, entry: &str) -> TestResult {
    pause(harness);
    card(harness, text).click_secondary();
    harness.run();
    harness
        .query_all_by_label(entry)
        .next_back()
        .ok_or_else(|| format!("no {entry:?} in the card's menu"))?
        .click();
    harness.run_steps(8);
    Ok(())
}

/// The ids of the Notes by their texts, in the lists or the trash, read
/// before they are merged.
type Ids = HashMap<String, String>;

fn known(harness: &Harness<'_, App>) -> Result<Ids, Box<dyn std::error::Error>> {
    let notes = Notes::new(harness.state().store());
    Ok(notes
        .all()?
        .into_iter()
        .chain(notes.trashed()?)
        .map(|n| (n.text, n.id))
        .collect())
}

/// The Note with `text`, as the store has it now.
fn note(
    harness: &Harness<'_, App>,
    ids: &Ids,
    text: &str,
) -> Result<oldbrainfix_core::Note, Box<dyn std::error::Error>> {
    let id = ids.get(text).ok_or_else(|| format!("no Note {text:?}"))?;
    Ok(Notes::new(harness.state().store())
        .by_id(id)?
        .ok_or_else(|| format!("no Note {text:?}"))?)
}

fn trashed(
    harness: &Harness<'_, App>,
    ids: &Ids,
    text: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(note(harness, ids, text)?.is_trashed())
}

/// Whether the Reminder of the plumber's Note is in the trash.
fn reminder_trashed(
    harness: &Harness<'_, App>,
    ids: &Ids,
) -> Result<bool, Box<dyn std::error::Error>> {
    let store = harness.state().store();
    let plumber = note(harness, ids, PLUMBER)?.id;
    let routines = Routines::new(store);
    // The only Routine there is, in the trash or not.
    for id in store.entities_of_type("routine")? {
        if let Some(routine) = routines.by_id(&id)?
            && routine.note_id == plumber
        {
            return Ok(routine.is_trashed());
        }
    }
    Err("the Reminder".into())
}

fn undo(harness: &mut Harness<'_, App>) {
    pause(harness);
    harness.key_press_modifiers(Modifiers::COMMAND, egui::Key::Z);
    harness.run();
}

#[test]
fn the_cards_selected_go_to_the_trash_together_and_one_undo_brings_all_back() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, PLUMBER]);
    right_pick(&mut harness, PLUMBER, "Move to trash")?;
    assert!(trashed(&harness, &ids, MILK)?);
    assert!(trashed(&harness, &ids, PLUMBER)?);
    // Its Reminder goes with it (#262).
    assert!(reminder_trashed(&harness, &ids)?);
    assert!(!trashed(&harness, &ids, ROSES)?);
    assert!(harness.state().selected_cards().is_empty());
    // Asked nothing.
    assert!(harness.query_by_label_contains("Delete this").is_none());

    undo(&mut harness);
    assert!(!trashed(&harness, &ids, MILK)?);
    assert!(!trashed(&harness, &ids, PLUMBER)?);
    assert!(!reminder_trashed(&harness, &ids)?);
    Ok(())
}

#[test]
fn a_card_outside_the_selection_goes_alone() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, PLUMBER]);
    right_pick(&mut harness, ROSES, "Move to trash")?;
    assert!(trashed(&harness, &ids, ROSES)?);
    assert!(!trashed(&harness, &ids, MILK)?);
    assert!(!trashed(&harness, &ids, PLUMBER)?);
    Ok(())
}

#[test]
fn note_delete_with_no_note_open_trashes_the_cards_selected() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, ROSES]);
    harness
        .query_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    harness
        .query_all_by_label("Delete")
        .next_back()
        .ok_or("Delete in the Note menu")?
        .click();
    harness.run();
    assert!(trashed(&harness, &ids, MILK)?);
    assert!(trashed(&harness, &ids, ROSES)?);
    assert!(!trashed(&harness, &ids, PLUMBER)?);
    undo(&mut harness);
    assert!(!trashed(&harness, &ids, MILK)?);
    assert!(!trashed(&harness, &ids, ROSES)?);
    Ok(())
}

#[test]
fn the_cards_selected_in_the_trash_are_restored_together() -> TestResult {
    let store = store()?;
    let routines = Routines::new(&store);
    for note in Notes::new(&store).all()? {
        routines.trash_note(&note.id)?;
    }
    let mut harness = harness(store, Selection::Trash);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, PLUMBER]);
    // The trash offers no Move to trash, only Restore.
    pause(&mut harness);
    card(&harness, PLUMBER).click_secondary();
    harness.run();
    assert!(harness.query_by_label("Move to trash").is_none());
    assert!(harness.query_by_label("Merge into…").is_none());
    harness.key_press(egui::Key::Escape);
    harness.run();
    right_pick(&mut harness, PLUMBER, "Restore")?;
    assert!(!trashed(&harness, &ids, MILK)?);
    assert!(!trashed(&harness, &ids, PLUMBER)?);
    assert!(!reminder_trashed(&harness, &ids)?);
    assert!(trashed(&harness, &ids, ROSES)?);

    // Note ▸ Restore restores the one selected, no Note open.
    click(&mut harness, ROSES, Modifiers::NONE);
    harness
        .query_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    harness
        .query_all_by_label("Restore")
        .next_back()
        .ok_or("Restore in the Note menu")?
        .click();
    harness.run();
    assert!(!trashed(&harness, &ids, ROSES)?);
    Ok(())
}

#[test]
fn importance_sets_it_on_the_cards_selected_and_closes_the_menu() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, ROSES]);
    pause(&mut harness);
    card(&harness, ROSES).click_secondary();
    harness.run();
    harness.get_by_label("Importance ⏵").click();
    harness.run();
    // As a keyboard or screen reader picks it, which no pointer click
    // closes.
    harness.get_by_label("Important").click_accesskit();
    harness.run();
    assert_eq!(
        note(&harness, &ids, MILK)?.importance,
        Importance::Important
    );
    assert_eq!(
        note(&harness, &ids, ROSES)?.importance,
        Importance::Important
    );
    assert_eq!(
        note(&harness, &ids, PLUMBER)?.importance,
        Importance::Normal
    );
    // The whole menu closed with the pick.
    assert!(harness.query_by_label("Merge into…").is_none());
    assert!(harness.state().selected_cards().is_empty());
    Ok(())
}

#[test]
fn the_cards_selected_merge_into_another_and_one_undo_parts_them() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, ROSES]);
    right_pick(&mut harness, ROSES, "Merge into…")?;
    // Picked in the dialog, drawn after the cards.
    harness
        .query_all_by_label(PLUMBER)
        .next_back()
        .ok_or("the plumber's Note to pick")?
        .click();
    harness.run();
    let plumber = note(&harness, &ids, PLUMBER)?.id;
    assert_eq!(
        note(&harness, &ids, MILK)?.merged_into.as_deref(),
        Some(plumber.as_str())
    );
    assert_eq!(
        note(&harness, &ids, ROSES)?.merged_into.as_deref(),
        Some(plumber.as_str())
    );
    assert!(harness.state().selected_cards().is_empty());
    // The list stays, no Note opened.
    assert_eq!(harness.state().selected_note(), None);

    undo(&mut harness);
    assert!(!note(&harness, &ids, MILK)?.is_merged());
    assert!(!note(&harness, &ids, ROSES)?.is_merged());
    Ok(())
}

#[test]
fn a_card_selected_picked_in_merge_into_takes_the_others_in() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, ROSES]);
    right_pick(&mut harness, MILK, "Merge into…")?;
    harness
        .query_all_by_label(ROSES)
        .next_back()
        .ok_or("the roses' Note to pick")?
        .click();
    harness.run();
    let roses = note(&harness, &ids, ROSES)?;
    assert_eq!(
        note(&harness, &ids, MILK)?.merged_into.as_deref(),
        Some(roses.id.as_str())
    );
    assert!(!roses.is_merged());
    assert!(!note(&harness, &ids, PLUMBER)?.is_merged());
    Ok(())
}

/// Ann's Household with her Note in it, shared with Ben as a Member, and
/// Ben's own Unsorted Note: Ben's store.
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
    Notes::new(&ben).capture(ROSES, None, None)?;
    Notes::new(&ben).capture(PLUMBER, None, None)?;
    Ok(ben)
}

#[test]
fn a_note_this_device_may_not_change_stays() -> TestResult {
    let mut harness = harness(shared()?, Selection::All);
    let ids = known(&harness)?;
    // Alone, Ann's card offers none of them.
    pause(&mut harness);
    card(&harness, MILK).click_secondary();
    harness.run();
    assert!(harness.query_by_label("Move to trash").is_none());
    assert!(harness.query_by_label("Importance ⏵").is_none());
    harness.key_press(egui::Key::Escape);
    harness.run();

    select(&mut harness, &[MILK, ROSES]);
    right_pick(&mut harness, ROSES, "Move to trash")?;
    assert!(trashed(&harness, &ids, ROSES)?);
    assert!(!trashed(&harness, &ids, MILK)?);
    Ok(())
}

#[test]
fn a_merge_leaves_out_a_note_this_device_may_not_change() -> TestResult {
    let mut harness = harness(shared()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, ROSES]);
    right_pick(&mut harness, ROSES, "Merge into…")?;
    pick_note(&mut harness, PLUMBER)?;
    // Ben's own Note merged; Ann's, in her Shared Area, did not stop it
    // and stayed.
    let plumber = note(&harness, &ids, PLUMBER)?.id;
    let roses = note(&harness, &ids, ROSES)?;
    assert_eq!(roses.merged_into.as_deref(), Some(plumber.as_str()));
    assert!(!note(&harness, &ids, MILK)?.is_merged());
    Ok(())
}

/// Picks the Note `text` in Merge into…, drawn after its card.
fn pick_note(harness: &mut Harness<'_, App>, text: &str) -> TestResult {
    harness
        .query_all_by_label(text)
        .next_back()
        .ok_or_else(|| format!("no {text:?} to pick"))?
        .click();
    harness.run();
    Ok(())
}

/// Opens the menu bar's Note menu, ahead of the window's own "Note".
fn note_menu(harness: &mut Harness<'_, App>) -> TestResult {
    harness
        .query_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    Ok(())
}

#[test]
fn the_note_menu_merges_and_weighs_the_cards_selected() -> TestResult {
    let mut harness = harness(store()?, Selection::All);
    let ids = known(&harness)?;
    select(&mut harness, &[MILK, ROSES]);
    note_menu(&mut harness)?;
    harness.get_by_label("Importance ⏵").click();
    harness.run();
    harness.get_by_label("Unimportant").click_accesskit();
    harness.run();
    assert_eq!(
        note(&harness, &ids, MILK)?.importance,
        Importance::Unimportant
    );
    assert_eq!(
        note(&harness, &ids, ROSES)?.importance,
        Importance::Unimportant
    );
    assert_eq!(
        note(&harness, &ids, PLUMBER)?.importance,
        Importance::Normal
    );
    // The menu closed with a pick by keyboard or screen reader too.
    assert!(harness.query_by_label("Importance ⏵").is_none());

    select(&mut harness, &[MILK, ROSES]);
    note_menu(&mut harness)?;
    harness
        .query_all_by_label("Merge into…")
        .next_back()
        .ok_or("Merge into… in the Note menu")?
        .click();
    harness.run_steps(8);
    pick_note(&mut harness, PLUMBER)?;
    let plumber = note(&harness, &ids, PLUMBER)?.id;
    for text in [MILK, ROSES] {
        let merged = note(&harness, &ids, text)?;
        assert_eq!(merged.merged_into.as_deref(), Some(plumber.as_str()));
    }
    undo(&mut harness);
    assert!(!note(&harness, &ids, MILK)?.is_merged());
    assert!(!note(&harness, &ids, ROSES)?.is_merged());
    Ok(())
}

#[test]
fn the_note_menu_weighs_and_restores_the_note_open() -> TestResult {
    let store = store()?;
    let mut harness = harness(store, Selection::All);
    let ids = known(&harness)?;
    let milk = note(&harness, &ids, MILK)?.id;
    harness.state_mut().select_note(&milk);
    harness.run();
    note_menu(&mut harness)?;
    harness.get_by_label("Importance ⏵").click();
    harness.run();
    harness.get_by_label("Important").click_accesskit();
    harness.run();
    assert_eq!(
        note(&harness, &ids, MILK)?.importance,
        Importance::Important
    );
    assert_eq!(note(&harness, &ids, ROSES)?.importance, Importance::Normal);

    // Open in the trash, it is restored from the Note menu as from the
    // Note dialog.
    Routines::new(harness.state().store()).trash_note(&milk)?;
    harness.state_mut().select(Selection::Trash);
    harness.run();
    harness.state_mut().select_note(&milk);
    harness.run();
    note_menu(&mut harness)?;
    harness
        .query_all_by_label("Restore")
        .next_back()
        .ok_or("Restore in the Note menu")?
        .click();
    harness.run();
    assert!(!trashed(&harness, &ids, MILK)?);
    Ok(())
}
