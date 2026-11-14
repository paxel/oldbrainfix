//! The sidebar's Areas in the window (#279): the menu bar's Area menu
//! for the Area or Topic shown, its entries greyed where they may not be
//! done or none shows, F2 renaming but not in a text field; Unsorted
//! saying how many Notes it holds; and an Area folding its Topics with
//! its chevron, kept on this Device.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use egui::Key;
use egui::accesskit::Toggled;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store with the Area Household, its Topic Repairs holding "Fix the
/// tap", and two Unsorted Notes; the ids of Household and Repairs.
fn store() -> Result<(Arc<Store>, String, String), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([7; 32]))
    })?;
    let places = Places::new(&store);
    let household = places.add_area("Household")?;
    let repairs = places.add_topic(&household.id, "Repairs")?;
    let notes = Notes::new(&store);
    let tap = notes.capture("Fix the tap", None, None)?;
    notes.file(&tap.id, &repairs.id)?;
    notes.capture("Call the bank", None, None)?;
    notes.capture("Water the plants", None, None)?;
    Ok((Arc::new(store), household.id, repairs.id))
}

fn window(app: App) -> Harness<'static, App> {
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

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The last widget labelled `label`: a menu's entry, drawn over the
/// window's widgets of the same name.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

/// The menu bar's `menu`, ahead of the window's widgets of the same name.
fn bar<'a>(harness: &'a Harness<'_, App>, menu: &'a str) -> Node<'a> {
    match harness.get_all_by_label(menu).next() {
        Some(node) => node,
        None => panic!("no menu {menu}"),
    }
}

fn open(harness: &mut Harness<'_, App>, menu: &str) {
    bar(harness, menu).click();
    harness.run();
}

/// Which of `entries` of `menu` can be picked; the menu is closed again.
fn enabled<const N: usize>(
    harness: &mut Harness<'_, App>,
    menu: &str,
    entries: [&str; N],
) -> [bool; N] {
    open(harness, menu);
    let on = entries.map(|entry| !last(harness, entry).accesskit_node().is_disabled());
    bar(harness, menu).click();
    harness.run();
    on
}

/// Picks `entry` of the Area menu by keyboard or screen reader, which
/// leaves egui's menus open unless the entry closes it.
fn pick(harness: &mut Harness<'_, App>, entry: &str) {
    open(harness, "Area");
    last(harness, entry).click_accesskit();
    harness.run();
}

/// The six entries of the Area menu, Rename with its key.
const ENTRIES: [&str; 6] = [
    "Rename F2",
    "Colour and icon",
    "Share…",
    "People",
    "Export as Pack…",
    "Delete",
];

#[test]
fn the_area_menu_is_greyed_where_no_area_or_topic_shows() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(App::new(store, "en"));
    assert_eq!(enabled(&mut harness, "Area", ENTRIES), [false; 6]);
    // F2 renames nothing there.
    harness.key_press(Key::F2);
    harness.run();
    assert!(!shows(&harness, "Rename"));
    Ok(())
}

#[test]
fn the_area_menu_acts_on_the_area_shown_and_closes() -> TestResult {
    let (store, household, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(household));
    let mut harness = window(app);
    // Not shared: People is greyed, the rest may be done.
    assert_eq!(
        enabled(&mut harness, "Area", ENTRIES),
        [true, true, true, false, true, true]
    );
    pick(&mut harness, "Rename F2");
    assert!(shows(&harness, "Rename"), "the Rename dialog");
    assert!(!shows(&harness, "Rename F2"), "the menu closed");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shows(&harness, "Rename"));

    pick(&mut harness, "Delete");
    assert!(shows(&harness, "Delete the Area “Household”?"));
    assert!(!shows(&harness, "Rename F2"), "the menu closed");
    Ok(())
}

#[test]
fn for_a_topic_colour_share_and_people_are_greyed() -> TestResult {
    let (store, _, repairs) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(repairs));
    let mut harness = window(app);
    assert_eq!(
        enabled(&mut harness, "Area", ENTRIES),
        [true, false, false, false, true, true]
    );
    pick(&mut harness, "Export as Pack…");
    assert!(shows(&harness, "Export as Pack"), "the Pack dialog");
    Ok(())
}

#[test]
fn f2_renames_the_place_shown_but_not_in_a_text_field() -> TestResult {
    let (store, _, repairs) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(repairs));
    let mut harness = window(app);
    harness.key_press(Key::F2);
    harness.run();
    assert!(shows(&harness, "Rename"), "the Rename dialog");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shows(&harness, "Rename"));

    // In the search field F2 is the field's.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::F);
    harness.run();
    assert!(harness.ctx.text_edit_focused());
    harness.key_press(Key::F2);
    harness.run();
    assert!(!shows(&harness, "Rename"));
    Ok(())
}

#[test]
fn the_area_menu_rests_over_the_note_dialog() -> TestResult {
    let (store, household, _) = store()?;
    let tap = Notes::new(&store)
        .all()?
        .into_iter()
        .find(|n| n.text == "Fix the tap")
        .ok_or("the tap")?
        .id;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(household));
    app.select_note(&tap);
    let mut harness = window(app);
    assert!(harness.state().selected_note().is_some());
    // The Note dialog has an Export as Pack… of its own, for the Note:
    // each entry has a greyed one, the Area menu's.
    open(&mut harness, "Area");
    for entry in ENTRIES {
        let greyed = harness
            .get_all_by_label(entry)
            .any(|n| n.accesskit_node().is_disabled());
        assert!(greyed, "{entry}");
    }
    bar(&harness, "Area").click();
    harness.run();
    harness.key_press(Key::F2);
    harness.run();
    assert!(!shows(&harness, "Rename"));
    Ok(())
}

#[test]
fn unsorted_says_how_many_notes_it_holds() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(App::new(store.clone(), "en"));
    assert!(shows(&harness, "Unsorted 2"));
    harness.get_by_label("Unsorted 2").click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    // Nothing else says a count.
    for place in ["All Notes", "Household", "Repairs", "Trash"] {
        assert!(shows(&harness, place), "{place}");
    }
    // Emptied, it says none.
    let notes = Notes::new(&store);
    for note in notes.unsorted()? {
        notes.trash(&note.id)?;
    }
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(
        !harness
            .query_all_by_label_contains("Unsorted ")
            .any(|_| true)
    );
    assert!(shows(&harness, "Unsorted"));
    Ok(())
}

#[test]
fn an_area_folds_its_topics_and_this_device_keeps_the_fold() -> TestResult {
    let (store, household, repairs) = store()?;
    let mut harness = window(App::new(store.clone(), "en"));
    assert!(shows(&harness, "Repairs"));
    harness.get_by_label("Hide the Topics of Household").click();
    harness.run();
    assert!(!shows(&harness, "Repairs"));
    // The chevron's click selects nothing.
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    assert!(shows(&harness, "Show the Topics of Household"));
    drop(harness);

    // A new window on this Device keeps it folded.
    let mut harness = window(App::new(store.clone(), "en"));
    assert!(!shows(&harness, "Repairs"));
    // The Topic shown stays in its folded Area, marked.
    harness
        .state_mut()
        .select(Selection::Place(repairs.clone()));
    harness.run();
    let marked = harness
        .get_all_by_label("Repairs")
        .any(|n| n.accesskit_node().toggled() == Some(Toggled::True));
    assert!(marked, "the Topic shown is marked");
    harness.state_mut().select(Selection::Place(household));
    harness.run();
    assert!(!shows(&harness, "Repairs"));

    harness.get_by_label("Show the Topics of Household").click();
    harness.run();
    assert!(shows(&harness, "Repairs"));
    drop(harness);
    let harness = window(App::new(store, "en"));
    assert!(shows(&harness, "Repairs"));
    Ok(())
}
