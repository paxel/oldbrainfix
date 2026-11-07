//! The Note dialog (#146) and the badges of a Note's settings in it
//! (#160), the quick note's dialog (#146) and what "+" at the top of a
//! list makes (#161), driven headless by egui_kittest: a card opens its
//! Note in a dialog, one at a time, which Esc, its cross and back close,
//! keeping what was typed, the menu bar at hand over it and Tab kept off
//! the list beneath; Ctrl+N, File ▸
//! New Note and "+" open the quick note, and no list has a capture
//! field; each setting is a badge read as "setting: value" whose small
//! editor changes it; "+" makes a Note, a Checklist, a Shopping List, a
//! Tally, a Session or a Frog where the list is, and the last card of a
//! list scrolls fully clear of the window's edge (#158); a captured
//! detail of the Note, in its Captured badge, or of a Section, in the
//! dialog its line under it opens (#170),
//! goes with its Remove after one question while the time stays, and
//! where this Device may change the Note only (#162).

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
use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::named_places::keys as place_keys;
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::{
    AreaRole, Checklists, Day, Frogs, Importance, Invite, ListOnAgenda, Notes, OwnProfile,
    PairedDevice, PersonSettings, Places, Recurrence, Routines, Schedule, Sessions, ShoppingLists,
    SigningKey, Store, Tallies, TallyRule,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a microsecond per read, with the Area
/// Household: the store and the Area's id.
fn store() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([4; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    Ok((Arc::new(store), household))
}

/// The window over `store` in English, showing `selection`, `height`
/// tall.
fn window_of(store: &Arc<Store>, selection: Selection, height: f32) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(selection);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, height))
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

fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    window_of(store, selection, 1200.0)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The last widget labelled `label`: a menu's entry or a dialog's,
/// drawn over the window's of the same name.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    last(harness, label).click_accesskit();
    harness.run();
}

/// The menu bar's `menu`, ahead of the window's buttons of the same name.
fn bar<'a>(harness: &'a Harness<'_, App>, menu: &'a str) -> Node<'a> {
    match harness.get_all_by_label(menu).next() {
        Some(node) => node,
        None => panic!("no menu {menu}"),
    }
}

fn typed(harness: &mut Harness<'_, App>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn text_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(id)?.ok_or("the Note")?.text)
}

/// The card whose text starts with `text`.
fn card<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> Node<'a> {
    match harness
        .query_all_by(move |n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with(text))
        })
        .next()
    {
        Some(node) => node,
        None => panic!("no card {text}"),
    }
}

/// Picks the radio button `label` of the open small editor (#188).
fn pick(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .click_accesskit();
    harness.run();
}

/// The Note dialog's text field.
fn note_field<'a>(harness: &'a Harness<'_, App>) -> Node<'a> {
    harness.get_by_label("Note text")
}

#[test]
fn a_card_opens_its_note_in_a_dialog_which_esc_closes_keeping_the_text() -> TestResult {
    let (store, _) = store()?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    Notes::new(&store).capture("Stamps", None, None)?;
    let mut harness = window(&store, Selection::All);
    assert!(!harness.state().note_open());
    // Edit on the card, not the menu bar's Edit.
    let milk_card = card(&harness, "Milk").rect();
    harness
        .get_all_by_role_and_label(Role::Button, "Edit")
        .rfind(|n| milk_card.contains(n.rect().center()))
        .ok_or("Milk's Edit")?
        .click();
    harness.run();
    assert!(harness.state().note_open());
    assert_eq!(harness.state().selected_note(), Some(milk.as_str()));
    note_field(&harness).click();
    harness.run();
    harness.key_press(Key::End);
    typed(&mut harness, " and bread");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().note_open());
    assert_eq!(text_of(&store, &milk)?, "Milk and bread");
    // One at a time: another one opens in its place.
    harness.state_mut().select_note(&milk);
    harness.run();
    assert_eq!(harness.query_all_by_label("Note text").count(), 1);
    // The cross closes it, and so does back.
    click(&mut harness, "Close");
    assert!(!harness.state().note_open());
    harness.state_mut().select_note(&milk);
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert!(!harness.state().note_open());
    Ok(())
}

#[test]
fn the_menu_bar_stays_at_hand_over_the_note_dialog() -> TestResult {
    let (store, household) = store()?;
    let tap = Notes::new(&store).capture("Fix the tap", None, None)?.id;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    bar(&harness, "Note").click();
    harness.run();
    let entry = last(&harness, "File into… Ctrl+M");
    assert!(!entry.accesskit_node().is_disabled());
    entry.click();
    harness.run();
    last(&harness, "Household").click();
    harness.run();
    let filed = Notes::new(&store).by_id(&tap)?.ok_or("the Note")?;
    assert_eq!(filed.place.as_deref(), Some(household.as_str()));
    // Ctrl+M too, over the dialog.
    assert!(harness.state().note_open());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run();
    assert!(shows(&harness, "Unsorted"));
    Ok(())
}

#[test]
fn ctrl_n_file_new_note_and_plus_open_the_quick_note_and_no_list_has_a_capture_field() -> TestResult
{
    let (store, household) = store()?;
    for selection in [
        Selection::Agenda,
        Selection::All,
        Selection::Unsorted,
        Selection::Place(household.clone()),
    ] {
        let harness = window(&store, selection.clone());
        assert!(!shows(&harness, "Capture"), "{selection:?}");
    }
    let mut harness = window(&store, Selection::Agenda);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    harness.run();
    assert!(harness.state().quick_note_open());
    harness.run();
    assert_eq!(
        harness
            .get_by(|n| n.is_focused())
            .accesskit_node()
            .label()
            .as_deref(),
        Some("Capture")
    );
    typed(&mut harness, "Call Bob");
    // Leaving keeps it, nudged in 1 h.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().quick_note_open());
    let bob = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Call Bob")
        .ok_or("the Note")?;
    assert!(bob.is_unsorted());
    assert!(store.current(&bob.id, "nudge")?.is_some());
    // The new Note's card shows; no Note dialog opens.
    assert!(!harness.state().note_open());
    // File ▸ New Note.
    bar(&harness, "File").click();
    harness.run();
    last(&harness, "New Note Ctrl+N").click();
    harness.run();
    assert!(harness.state().quick_note_open());
    click(&mut harness, "Close");
    assert!(!harness.state().quick_note_open());
    // The Agenda's "+".
    click(&mut harness, "New Note");
    assert!(harness.state().quick_note_open());
    Ok(())
}

#[test]
fn each_setting_is_a_badge_read_as_setting_and_value() -> TestResult {
    let (store, household) = store()?;
    let notes = Notes::new(&store);
    let tap = notes.capture("Fix the tap ☐", Some(&household), None)?.id;
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let schedule = Schedule::new(Day::new(2026, 3, 2), &[], Some(rule)).ok_or("a schedule")?;
    Routines::new(&store).spawn(&tap, &schedule)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    for label in [
        "Importance: Normal",
        "Area and Topic: Household",
        "Show ticks: Off",
        "Routine, Reminder, Frog or Session",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(
        harness
            .query_all_by_label_contains("Captured: ")
            .next()
            .is_some()
    );
    assert!(
        harness
            .query_all_by_label_contains("Routine: ")
            .next()
            .is_some()
    );
    // A plain Note has its kind too (#188), no Checklist's Show on the
    // Agenda.
    assert!(shows(&harness, "Kind: Plain Note"));
    assert!(
        harness
            .query_all_by_label_contains("Show on the Agenda")
            .next()
            .is_none()
    );
    Ok(())
}

/// A badge whose setting has a fixed set of values shows its value as an
/// icon, never with its word (#192): it is as wide for a long value as for
/// a short one, and as wide as an icon; other values are text, wider.
#[test]
fn a_fixed_set_of_values_shows_as_an_icon_without_its_word() -> TestResult {
    let (store, household) = store()?;
    let notes = Notes::new(&store);
    let tap = notes.capture("Fix the tap ☐", Some(&household), None)?.id;
    notes.set_importance(&tap, Importance::Unimportant)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    let width = |label: &str| harness.get_by_label(label).rect().width();
    // The setting's icon part, 38 points, then an 18-point icon padded by
    // 10 on each side.
    for label in [
        "Kind: Plain Note",
        "Importance: Unimportant",
        "Show ticks: Off",
    ] {
        assert_eq!(width(label), 38.0 + 18.0 + 20.0, "{label}");
    }
    assert!(width("Area and Topic: Household") > 38.0 + 18.0 + 20.0);
    Ok(())
}

#[test]
fn each_badge_opens_its_small_editor_and_follows_the_change() -> TestResult {
    let (store, household) = store()?;
    let tap = Notes::new(&store).capture("Fix the tap ☐", None, None)?.id;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    // The Importance as radio buttons; a pick changes it and closes.
    click(&mut harness, "Importance: Normal");
    assert!(shows(&harness, "Unimportant"));
    click(&mut harness, "Important");
    let note = Notes::new(&store).by_id(&tap)?.ok_or("the Note")?;
    assert_eq!(note.importance, Importance::Important);
    assert!(shows(&harness, "Importance: Important"));
    // Area and Topic opens File into….
    click(&mut harness, "Area and Topic: Unsorted");
    click(&mut harness, "Household");
    assert!(shows(&harness, "Area and Topic: Household"));
    let note = Notes::new(&store).by_id(&tap)?.ok_or("the Note")?;
    assert_eq!(note.place.as_deref(), Some(household.as_str()));
    // Show ticks is a switch, written at once.
    click(&mut harness, "Show ticks: Off");
    assert!(shows(&harness, "Off"));
    harness
        .get_by_role_and_label(Role::CheckBox, "Show ticks")
        .click_accesskit();
    harness.run();
    assert_eq!(
        store.local_setting(&format!("showTicks:{tap}"))?.as_deref(),
        Some("true")
    );
    click(&mut harness, "Close");
    assert!(shows(&harness, "Show ticks: On"));
    // Captured only shows its details.
    let captured = harness
        .query_all_by_label_contains("Captured: ")
        .next()
        .ok_or("Captured")?
        .accesskit_node()
        .label()
        .unwrap_or_default();
    click(&mut harness, &captured);
    assert!(shows(&harness, "Captured"));
    harness.key_press(Key::Escape);
    harness.run();
    // Esc closed the small editor, not the Note.
    assert!(harness.state().note_open());
    Ok(())
}

#[test]
fn a_lists_kind_badge_goes_back_to_a_plain_note_and_its_agenda_badge_switches() -> TestResult {
    let (store, _) = store()?;
    let list = Notes::new(&store)
        .capture("Packing\nsocks\nshirts", None, None)?
        .id;
    Checklists::new(&store).make(&list)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&list);
    harness.run();
    assert!(shows(&harness, "Kind: Checklist"));
    // Not in the Checklist's part any more.
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::CheckBox, "Show on the Agenda")
            .count(),
        0
    );
    // Its three ways, as a Shopping List's (#219); a pick closes it.
    click(
        &mut harness,
        "Show on the Agenda: Each open Line on its own row",
    );
    pick(&mut harness, "Off");
    assert_eq!(Checklists::new(&store).on_agenda(&list)?, ListOnAgenda::Off);
    assert!(shows(&harness, "Show on the Agenda: Off"));
    click(&mut harness, "Kind: Checklist");
    pick(&mut harness, "Plain Note");
    assert!(Checklists::new(&store).by_note(&list)?.is_none());
    assert!(shows(&harness, "Kind: Plain Note"));
    // A Shopping List's too.
    ShoppingLists::new(&store).make(&list)?;
    harness.state_mut().close_note();
    harness.state_mut().select_note(&list);
    harness.run();
    click(&mut harness, "Kind: Shopping List");
    pick(&mut harness, "Plain Note");
    let wall = |at: DateTime<Utc>| at.naive_utc();
    assert!(ShoppingLists::new(&store).by_note(&list, &wall)?.is_none());
    Ok(())
}

#[test]
fn plus_badge_offers_routine_reminder_frog_and_session() -> TestResult {
    let (store, _) = store()?;
    let tap = Notes::new(&store).capture("Fix the tap", None, None)?.id;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    harness
        .get_by_label("Routine, Reminder, Frog or Session")
        .click();
    harness.run();
    for label in ["Routine", "Reminder", "Frog", "Session"] {
        assert!(shows(&harness, label), "{label}");
    }
    click(&mut harness, "Frog");
    assert_eq!(Frogs::new(&store).of_note(&tap)?.len(), 1);
    // A Note has one open Frog at a time.
    harness
        .get_by_label("Routine, Reminder, Frog or Session")
        .click();
    harness.run();
    assert!(shows(&harness, "Reminder"));
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Frog")
            .count(),
        0
    );
    click(&mut harness, "Reminder");
    let editor = harness.state().routine_editor().ok_or("the editor")?;
    assert!(editor.once());
    // The editor over the Note goes back to it.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().routine_editor().is_none());
    assert!(harness.state().note_open());
    Ok(())
}

#[test]
fn plus_offers_the_six_kinds_where_the_list_is_and_a_topic_in_an_area() -> TestResult {
    let (store, household) = store()?;
    let kinds = [
        "Note",
        "Checklist",
        "Shopping List",
        "Tally",
        "Session",
        "Frog",
    ];
    let mut harness = window(&store, Selection::Place(household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    for label in kinds.iter().chain(&["Topic"]) {
        assert!(shows(&harness, label), "{label}");
    }
    let mut harness = window(&store, Selection::All);
    harness.get_by_label("Add here").click();
    harness.run();
    for label in kinds {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(!shows(&harness, "Topic"));
    // Note opens the quick note, filed where the list is.
    let mut harness = window(&store, Selection::Place(household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Note");
    assert!(harness.state().quick_note_open());
    typed(&mut harness, "Descale the kettle");
    click(&mut harness, "Save");
    let kettle = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Descale the kettle")
        .ok_or("the Note")?;
    assert_eq!(kettle.place.as_deref(), Some(household.as_str()));
    // Unsorted's "+" opens the quick note at once.
    let mut harness = window(&store, Selection::Unsorted);
    click(&mut harness, "New Note");
    assert!(harness.state().quick_note_open());
    Ok(())
}

#[test]
fn in_simple_plus_offers_note_checklist_and_shopping_list_and_a_tally_stays() -> TestResult {
    let (store, household) = store()?;
    let push_ups = Notes::new(&store).capture("Push-ups", Some(&household), None)?;
    Tallies::new(&store).add_field(&push_ups.id, "Push-ups", "times", None, None)?;
    PersonSettings::new(&store).set_advanced(false)?;
    let mut harness = window(&store, Selection::Place(household.clone()));
    assert!(shows(&harness, "Push-ups"));
    harness.get_by_label("Add here").click();
    harness.run();
    for label in ["Note", "Checklist", "Shopping List"] {
        assert!(shows(&harness, label), "{label}");
    }
    for label in ["Tally", "Session", "Frog", "Topic"] {
        assert!(!shows(&harness, label), "{label}");
    }
    Ok(())
}

#[test]
fn a_checklist_from_plus_is_named_then_typed_in_place_and_a_frog_only_named() -> TestResult {
    let (store, household) = store()?;
    let mut harness = window(&store, Selection::Place(household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Checklist");
    assert!(shows(&harness, "New Checklist"));
    typed(&mut harness, "Packing");
    click(&mut harness, "Create");
    let packing = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Packing")
        .ok_or("the Checklist")?;
    assert_eq!(packing.place.as_deref(), Some(household.as_str()));
    assert!(Checklists::new(&store).by_note(&packing.id)?.is_some());
    assert_eq!(harness.state().selected_note(), Some(packing.id.as_str()));
    // The cursor waits in its first Line.
    harness.run();
    let focused = harness.get_by(|n| n.is_focused());
    assert_eq!(focused.accesskit_node().role(), Role::TextInput);
    typed(&mut harness, "Socks");
    harness.state_mut().close_note();
    harness.run();
    let list = Checklists::new(&store)
        .by_note(&packing.id)?
        .ok_or("the Checklist")?;
    assert_eq!(list.lines.len(), 1);
    assert_eq!(list.lines[0].text, "Socks");
    // A Frog from All Notes: its name only, Unsorted no more.
    let mut harness = window(&store, Selection::All);
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Frog");
    assert!(shows(&harness, "New Frog"));
    typed(&mut harness, "Call the tax office");
    click(&mut harness, "Create");
    let tax = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Call the tax office")
        .ok_or("the Frog's Note")?;
    assert_eq!(tax.place, None);
    assert_eq!(Frogs::new(&store).of_note(&tax.id)?.len(), 1);
    assert!(!harness.state().note_open());
    Ok(())
}

#[test]
fn a_shopping_list_from_plus_starts_with_one_line_the_cursor_in_it() -> TestResult {
    let (store, household) = store()?;
    let mut harness = window(&store, Selection::Place(household));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Shopping List");
    typed(&mut harness, "Market");
    click(&mut harness, "Create");
    let market = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Market")
        .ok_or("the Shopping List")?;
    // One empty Line, the cursor in it; no field and no "Add" above it.
    harness.run();
    let focused = harness.get_by(|n| n.is_focused());
    assert_eq!(focused.accesskit_node().role(), Role::TextInput);
    assert_eq!(focused.value().as_deref(), Some(""));
    // Under its name (#188).
    assert_eq!(
        harness
            .get_by_role_and_label(Role::TextInput, "Name")
            .value()
            .as_deref(),
        Some("Market")
    );
    let lines = harness.query_all_by(|n| {
        n.role() == Role::TextInput
            && n.placeholder().unwrap_or_default().is_empty()
            && n.label().as_deref() != Some("Name")
    });
    assert_eq!(lines.count(), 1);
    assert!(!shows(&harness, "Add"));
    typed(&mut harness, "Leeks");
    harness.key_press(egui::Key::Enter);
    harness.run();
    typed(&mut harness, "Pears");
    harness.state_mut().close_note();
    harness.run();
    let list = ShoppingLists::new(&store)
        .by_note(&market.id, &|utc: DateTime<Utc>| utc.naive_utc())?
        .ok_or("the Shopping List")?;
    let texts: Vec<&str> = list.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, ["Leeks", "Pears"]);
    Ok(())
}

#[test]
fn a_tally_with_its_fields_takes_one_page_and_one_save_in_their_order() -> TestResult {
    let (store, household) = store()?;
    let mut harness = window(&store, Selection::Place(household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Tally");
    assert!(shows(&harness, "New Tally"));
    // Save without a name says so, the name taking the keyboard, and
    // makes nothing.
    click(&mut harness, "Save");
    assert!(shows(&harness, "Give it a name."));
    typed(&mut harness, "Morning");
    let names = ["Weight", "Waist", "Steps", "Arm", "Calf"];
    for (i, name) in names.iter().enumerate() {
        if i == 0 {
            // The page's name first, then each field's.
            harness
                .get_all_by_role_and_label(Role::TextInput, "Name")
                .nth(1)
                .ok_or("the first field")?
                .click();
        } else {
            click(&mut harness, "Add field");
        }
        harness.run();
        typed(&mut harness, name);
    }
    // A row left without a name is dropped.
    click(&mut harness, "Add field");
    // The first field's unit, typed with suggestions; its rule follows.
    harness
        .get_all_by_label("Unit: No unit")
        .next()
        .ok_or("the unit badge")?
        .click();
    harness.run();
    typed(&mut harness, "k");
    assert!(shows(&harness, "kg"));
    click(&mut harness, "kg");
    click(&mut harness, "Close");
    assert!(shows(&harness, "Unit: kg"));
    assert!(shows(
        &harness,
        "Several sets or entries in a bar count: Largest"
    ));
    // A row's kind (#264): a Check field has no unit and no rule.
    let units = harness.query_all_by_label("Unit: No unit").count();
    harness
        .get_all_by_label("Kind of field: Number")
        .last()
        .ok_or("the last row's kind")?
        .click();
    harness.run();
    // A pick closes the small editor.
    click(&mut harness, "Check field");
    assert!(shows(&harness, "Kind of field: Check field"));
    let left = harness.query_all_by_label("Unit: No unit").count();
    assert_eq!(left, units - 1);
    click(&mut harness, "Save");
    let morning = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Morning")
        .ok_or("the Tally's Note")?;
    assert_eq!(morning.place.as_deref(), Some(household.as_str()));
    let tally = Tallies::new(&store)
        .by_note(&morning.id)?
        .ok_or("the Tally")?;
    // Saved together, they keep the order they were typed in.
    let saved: Vec<&str> = tally.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(saved, names);
    assert_eq!(tally.fields[0].unit, "kg");
    assert_eq!(tally.fields[0].rule, TallyRule::Max);
    assert_eq!(tally.fields[1].rule, TallyRule::Sum);
    assert!(!shows(&harness, "New Tally"));
    Ok(())
}

#[test]
fn a_session_with_a_new_tally_made_inside_takes_one_page() -> TestResult {
    let (store, household) = store()?;
    let mut harness = window(&store, Selection::Place(household.clone()));
    harness.get_by_label("Add here").click();
    harness.run();
    click(&mut harness, "Session");
    assert!(shows(&harness, "New Session"));
    harness
        .get_by_role_and_label(Role::TextInput, "Name")
        .click();
    harness.run();
    typed(&mut harness, "Workout");
    click(&mut harness, "New Tally");
    assert!(shows(&harness, "New Tally"));
    typed(&mut harness, "Pushups");
    harness
        .get_all_by_role_and_label(Role::TextInput, "Name")
        .last()
        .ok_or("the field")?
        .click();
    harness.run();
    typed(&mut harness, "Count");
    click(&mut harness, "Save");
    // Back on the Session's page, the new Tally ticked.
    assert!(shows(&harness, "New Session"));
    let pushups = harness.get_by_role_and_label(Role::CheckBox, "Pushups");
    assert_eq!(
        pushups.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    // Every day, all day.
    assert!(shows(&harness, "All day"));
    click(&mut harness, "Save");
    let workout = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Workout")
        .ok_or("the Session's Note")?;
    assert_eq!(workout.place.as_deref(), Some(household.as_str()));
    let tally = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Pushups")
        .ok_or("the Tally's Note")?;
    let routine = Routines::new(&store)
        .of_note(&workout.id)?
        .into_iter()
        .next()
        .ok_or("the Session")?;
    let session = Sessions::new(&store)
        .by_id(&routine.id)?
        .ok_or("the Session")?;
    assert_eq!(session.setup.pool, [tally.id]);
    assert!(routine.schedule.times.is_empty());
    assert!(harness.state().routine_editor().is_none());
    Ok(())
}

#[test]
fn the_last_card_of_a_list_scrolls_fully_clear() -> TestResult {
    let (store, _) = store()?;
    let notes = Notes::new(&store);
    for i in 1..=12 {
        notes.capture(&format!("Alpha {i:02}\nwith\nmore\nlines"), None, None)?;
    }
    let height = 700.0;
    let mut harness = window_of(&store, Selection::All, height);
    // The oldest shows last.
    for _ in 0..40 {
        card(&harness, "Alpha 12").scroll_down();
        harness.run();
    }
    let last_card = card(&harness, "Alpha 01").rect();
    let add = harness.get_by_label("Add here").rect();
    // Fully in view, its shadow too, above the window's edge.
    assert!(
        last_card.bottom() <= height - 20.0,
        "{last_card:?} in {height}"
    );
    assert!(add.bottom() < last_card.top(), "{add:?} {last_card:?}");
    Ok(())
}

#[test]
fn tab_stays_in_the_note_dialog_and_off_the_list_beneath() -> TestResult {
    let (store, _) = store()?;
    let milk = Notes::new(&store).capture("Milk", None, None)?.id;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&milk);
    harness.run();
    let field = note_field(&harness).rect();
    for _ in 0..40 {
        harness.key_press(Key::Tab);
        harness.run();
        if let Some(focused) = harness.query_by(|n| n.is_focused()) {
            let at = focused.rect();
            // In the dialog, or the menu bar above it.
            assert!(
                at.left() >= field.left() - 40.0 || at.top() < 30.0,
                "{:?} at {at:?}",
                focused.accesskit_node().label()
            );
        }
    }
    assert!(harness.state().note_open());
    Ok(())
}

/// The Note "Fix the tap" in `area` with a Capture Context as a phone
/// keeps it, at Home, shared from WhatsApp during a call with two
/// Bluetooth devices, and an Amend captured elsewhere during another
/// event: the Note's id.
fn captured_in(store: &Store, area: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let note = notes.capture("Fix the tap", area, None)?;
    let home = "namedplace:home";
    let place = |field: &str, value: &str| store.append(home, field, Some(value), None);
    place(place_keys::TYPE, place_keys::TYPE_NAMED_PLACE)?;
    place(place_keys::NAME, "Home")?;
    place(place_keys::CENTRE, "52.52,13.405")?;
    place(place_keys::RADIUS, "100")?;
    let at = |field: &str, value: &str| store.append(&note.id, field, Some(value), None);
    at(note_keys::LOCATION, "52.52,13.405")?;
    at(note_keys::SOURCE_PACKAGE, "com.whatsapp")?;
    at(note_keys::SOURCE_LABEL, "WhatsApp")?;
    at(note_keys::CALENDAR, r#"["Team call"]"#)?;
    at(note_keys::BLUETOOTH, r#"["Car kit","Headphones"]"#)?;
    let amend = notes.amend(&note.id, "Washer bought")?;
    let at = |field: &str, value: &str| store.append(&amend.id, field, Some(value), None);
    at(note_keys::LOCATION, "48.13724,11.57561")?;
    at(note_keys::CALENDAR, r#"["Dentist"]"#)?;
    Ok(note.id)
}

/// The label of the first "Captured: …" button `joined` or not: a
/// Section's line, its details joined by " · " (#170), or the Note's
/// badge, its time alone.
fn captured_label(harness: &Harness<'_, App>, joined: bool) -> Option<String> {
    harness
        .query_all_by_label_contains("Captured: ")
        .filter_map(|n| n.accesskit_node().label())
        .find(|label| label.contains(" · ") == joined)
}

/// Opens the Captured badge of the Note open: the time it shows.
fn open_captured(harness: &mut Harness<'_, App>) -> Result<String, Box<dyn std::error::Error>> {
    let badge = captured_label(harness, false).ok_or("Captured")?;
    click(harness, &badge);
    harness.run_steps(8);
    assert!(shows(harness, "Captured"));
    Ok(badge.trim_start_matches("Captured: ").to_string())
}

/// Clicks `remove` and answers its question `question` with Remove.
fn remove(harness: &mut Harness<'_, App>, remove: &str, question: &str) {
    click(harness, remove);
    harness.run_steps(8);
    assert!(shows(harness, question), "{question}");
    click(harness, "Remove");
    harness.run_steps(8);
}

#[test]
fn each_captured_detail_but_the_time_goes_from_the_captured_badge() -> TestResult {
    let (store, _) = store()?;
    let tap = captured_in(&store, None)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    let time = open_captured(&mut harness)?;
    let steps = [
        ("place", "Home"),
        ("calendar events", "During: Team call"),
        ("Bluetooth devices", "Connected: Car kit, Headphones"),
        ("app", "Shared from WhatsApp"),
    ];
    for (detail, line) in steps {
        assert!(shows(&harness, line), "{line}");
        let question = format!("Remove the {detail} from this Note?");
        remove(&mut harness, &format!("Remove the {detail}"), &question);
        // Gone from the dialog, which stays open over the Note.
        assert!(!shows(&harness, line), "{line}");
        assert!(!shows(&harness, &question));
        assert!(shows(&harness, "Captured"));
        assert!(harness.state().note_open());
    }
    let context = Notes::new(&store).by_id(&tap)?.ok_or("the Note")?.context;
    assert!(context.location.is_none() && context.named_place.is_none());
    assert!(context.source_app.is_none());
    assert!(context.calendar_events.is_empty() && context.bluetooth_devices.is_empty());
    // The time stays, with no Remove of its own; the Amend's are behind
    // its own line (#170).
    assert!(shows(&harness, &time), "{time}");
    assert_eq!(harness.query_all_by_label_contains("Remove the").count(), 0);
    Ok(())
}

#[test]
fn cancel_keeps_the_captured_detail() -> TestResult {
    let (store, _) = store()?;
    let tap = captured_in(&store, None)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    open_captured(&mut harness)?;
    click(&mut harness, "Remove the calendar events");
    harness.run_steps(8);
    assert!(shows(
        &harness,
        "Remove the calendar events from this Note?"
    ));
    click(&mut harness, "Cancel");
    harness.run_steps(8);
    assert!(!shows(
        &harness,
        "Remove the calendar events from this Note?"
    ));
    assert!(shows(&harness, "During: Team call"));
    assert!(shows(&harness, "Remove the calendar events"));
    let context = Notes::new(&store).by_id(&tap)?.ok_or("the Note")?.context;
    assert_eq!(context.calendar_events, ["Team call"]);
    Ok(())
}

#[test]
fn a_sections_captured_detail_goes_from_the_dialog_its_line_opens() -> TestResult {
    let (store, _) = store()?;
    let tap = captured_in(&store, None)?;
    let mut harness = window(&store, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    // The Amend's own as one line under it in the Note dialog (#170),
    // with no Remove; its details in the dialog the line opens.
    let line = captured_label(&harness, true).ok_or("the Amend's line")?;
    assert!(line.contains(" · 48.1372, 11.5756 · Dentist"), "{line}");
    assert!(!shows(&harness, "48.1372, 11.5756"));
    assert!(!shows(&harness, "Remove the place"));
    click(&mut harness, &line);
    harness.run_steps(8);
    assert!(shows(&harness, "Captured"));
    assert!(shows(&harness, "48.1372, 11.5756"));
    remove(
        &mut harness,
        "Remove the place",
        "Remove the place from this Section?",
    );
    assert!(!shows(&harness, "48.1372, 11.5756"));
    // The dialog stays open over the Note and shows what is left.
    assert!(shows(&harness, "During: Dentist"));
    assert!(harness.state().note_open());
    let notes = Notes::new(&store);
    let amend = notes
        .sections(&tap)?
        .into_iter()
        .next()
        .ok_or("the Amend")?;
    assert!(amend.context.location.is_none());
    assert_eq!(amend.context.calendar_events, ["Dentist"]);
    // The Note's own place stays.
    let context = notes.by_id(&tap)?.ok_or("the Note")?.context;
    assert!(context.location.is_some());
    Ok(())
}

/// Ann's Household with her captured Note in it, shared with Ben as a
/// Member: Ben's store and the Note's id.
fn shared_captured() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let (ann, house) = store()?;
    let tap = captured_in(&ann, Some(&house))?;
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let ben = Store::in_memory(clock, || Ok(SigningKey::from_seed([5; 32])))?;
    OwnProfile::new(&ben).apply_defaults("Ben", None)?;
    let area = ann.share(&house)?;
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
    Ok((Arc::new(ben), tap))
}

#[test]
fn a_member_has_no_remove_on_another_persons_note() -> TestResult {
    let (ben, tap) = shared_captured()?;
    let mut harness = window(&ben, Selection::All);
    harness.state_mut().select_note(&tap);
    harness.run();
    // The Amend's details show in its line, without a Remove.
    let line = captured_label(&harness, true).ok_or("the Amend's line")?;
    assert!(line.contains("48.1372, 11.5756"), "{line}");
    open_captured(&mut harness)?;
    // Ann's Named Place stays hers: Ben sees the coordinates.
    assert!(shows(&harness, "52.5200, 13.4050"));
    assert!(shows(&harness, "During: Team call"));
    assert!(
        harness
            .query_all_by_label_contains("Remove the")
            .next()
            .is_none()
    );
    Ok(())
}
