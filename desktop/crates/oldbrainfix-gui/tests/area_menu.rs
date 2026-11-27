//! The sidebar's Areas in the window (#279): the ⋮ of the band of the
//! Area or Topic shown (obf-1065), its entries greyed where they may not
//! be done, none elsewhere, F2 renaming but not in a text field; Unsorted
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

use egui::accesskit::Toggled;
use egui::epaint::ClippedShape;
use egui::{Key, Pos2, Rect, Shape};
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

/// The ⋮ of the band of an Area.
const AREA: &str = "More for this Area";
/// The ⋮ of the band of a Topic.
const TOPIC: &str = "More for this Topic";

/// The band's ⋮ labelled `menu`.
fn bar<'a>(harness: &'a Harness<'_, App>, menu: &'a str) -> Node<'a> {
    harness.get_by_label(menu)
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

/// Picks `entry` of the band's ⋮ `menu` by keyboard or screen reader,
/// which leaves egui's menus open unless the entry closes it.
fn pick(harness: &mut Harness<'_, App>, menu: &str, entry: &str) {
    open(harness, menu);
    last(harness, entry).click_accesskit();
    harness.run();
}

/// The entries of an Area's band ⋮ (obf-1014), Name, colour and icon…
/// with its key, Hidden from AI being a switch, Export ▸ holding Export
/// as Pack… and Export as AXT file… (obf-1145).
const ENTRIES: [&str; 6] = [
    "New Topic",
    "Name, colour and icon… F2",
    "Share with people…",
    "Move ⏵",
    "Export ⏵",
    "Delete",
];

/// The entries of a Topic's band ⋮, Name and icon… (obf-1156) with its
/// key.
const TOPIC_ENTRIES: [&str; 4] = [
    "Name and icon… F2",
    "Export as Pack…",
    "Export as AXT file…",
    "Delete",
];

#[test]
fn no_area_menu_shows_where_no_area_or_topic_shows() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(App::new(store, "en"));
    // The menu bar has no Area and no Note menu (obf-1065).
    for menu in ["File", "Edit", "View", "Sync", "AI", "Help"] {
        assert!(shows(&harness, menu), "{menu}");
    }
    let bar_bottom = harness.get_by_label("File").rect().bottom();
    for gone in ["Area", "Note"] {
        let in_bar = harness
            .query_all_by_label(gone)
            .any(|n| n.rect().top() < bar_bottom);
        assert!(!in_bar, "{gone}");
    }
    for place in [Selection::Agenda, Selection::All, Selection::Unsorted] {
        harness.state_mut().select(place);
        harness.run();
        for gone in [AREA, TOPIC] {
            assert!(!shows(&harness, gone), "{gone}");
        }
        // F2 renames nothing there.
        harness.key_press(Key::F2);
        harness.run();
        assert!(!shows(&harness, "Rename"));
    }
    Ok(())
}

#[test]
fn the_bands_menu_acts_on_the_area_shown_and_closes() -> TestResult {
    let (store, household, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(household));
    let mut harness = window(app);
    // The one Area: Move is greyed, the rest may be done.
    assert_eq!(
        enabled(&mut harness, AREA, ENTRIES),
        [true, true, true, false, true, true]
    );
    assert!(!shows(&harness, TOPIC));
    pick(&mut harness, AREA, "Name, colour and icon… F2");
    assert!(
        shows(&harness, "Name, colour and icon"),
        "the one dialog of the three"
    );
    assert!(shows(&harness, "No colour"));
    assert!(
        !shows(&harness, "Name, colour and icon… F2"),
        "the menu closed"
    );
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shows(&harness, "Name, colour and icon"));

    pick(&mut harness, AREA, "Delete");
    assert!(shows(&harness, "Delete the Area “Household”?"));
    assert!(
        !shows(&harness, "Name, colour and icon… F2"),
        "the menu closed"
    );
    Ok(())
}

#[test]
fn the_bands_menu_stands_last_in_its_second_row() -> TestResult {
    let (store, household, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(household));
    let harness = window(app);
    let more = bar(&harness, AREA).rect();
    let add = harness.get_by_label("Add here").rect();
    assert!(more.left() >= add.right(), "{more:?} after {add:?}");
    assert!((more.center().y - add.center().y).abs() < 1.0);
    Ok(())
}

#[test]
fn a_topics_menu_renames_exports_and_deletes() -> TestResult {
    let (store, _, repairs) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Place(repairs));
    let mut harness = window(app);
    // As its right-click in the sidebar (obf-1014), none of the Area's.
    assert_eq!(
        enabled(&mut harness, TOPIC, TOPIC_ENTRIES),
        [true, true, true, true]
    );
    open(&mut harness, TOPIC);
    for gone in ["Name, colour and icon… F2", "Share with people…", "Move ⏵"] {
        assert!(!shows(&harness, gone), "{gone}");
    }
    bar(&harness, TOPIC).click();
    harness.run();
    assert!(!shows(&harness, AREA));
    pick(&mut harness, TOPIC, "Export as Pack…");
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
    // A Topic's name with its icon (obf-1156).
    assert!(shows(&harness, "Name and icon"), "the Topic's dialog");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shows(&harness, "Name and icon"));

    // In the search field F2 is the field's.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::F);
    harness.run();
    assert!(harness.ctx.text_edit_focused());
    harness.key_press(Key::F2);
    harness.run();
    assert!(!shows(&harness, "Name and icon"));
    Ok(())
}

#[test]
fn the_bands_menu_rests_over_the_note_dialog() -> TestResult {
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
    // The band's ⋮ stays in its place under the dialog; opened by a
    // screen reader, every entry of it is greyed. The Note dialog has an
    // Export as Pack… of its own, for the Note.
    bar(&harness, AREA).click_accesskit();
    harness.run();
    for entry in ENTRIES {
        let greyed = harness
            .get_all_by_label(entry)
            .any(|n| n.accesskit_node().is_disabled());
        let picked = harness
            .get_all_by_label(entry)
            .all(|n| n.accesskit_node().is_disabled() || entry == "Export as Pack…");
        assert!(greyed && picked, "{entry}");
    }
    bar(&harness, AREA).click_accesskit();
    harness.run();
    assert!(harness.state().selected_note().is_some());
    harness.key_press(Key::F2);
    harness.run();
    assert!(!shows(&harness, "Name, colour and icon"));
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

#[test]
fn a_topics_dialog_picks_its_icon_and_no_colour() -> TestResult {
    let (store, household, repairs) = store()?;
    Places::new(&store).set_colour(&household, Some("1565C0"))?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Place(repairs.clone()));
    let mut harness = window(app);
    harness.key_press(Key::F2);
    harness.run();
    // Its name and icon (obf-1156): a Topic has no colour of its own.
    assert!(shows(&harness, "Name and icon"));
    assert!(!shows(&harness, "No colour"));
    assert!(shows(&harness, "No icon"));
    last(&harness, "mail").click_accesskit();
    harness.run();
    last(&harness, "Save").click_accesskit();
    harness.run();
    assert!(!shows(&harness, "Name and icon"));
    let places = Places::new(&store);
    let icon = places.topic_by_id(&repairs)?.and_then(|t| t.icon);
    assert_eq!(icon.as_deref(), Some("mail"));
    // The Area keeps its own.
    let area = places.area_by_id(&household)?.ok_or("the Area")?;
    assert_eq!(area.icon, None);
    assert_eq!(area.colour.as_deref(), Some("1565C0"));
    Ok(())
}

/// Where the left pane draws the name `label` in its row, in points, and
/// the size of its text: once for each time it is drawn.
fn drawn_names(harness: &Harness<'_, App>, label: &str) -> Vec<(Pos2, f32)> {
    fn flat(shape: &Shape, into: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flat(s, into)),
            other => into.push(other.clone()),
        }
    }
    let scale = harness.ctx.pixels_per_point();
    let rows: Vec<Rect> = harness
        .query_all_by_label(label)
        .map(|n| {
            let r = n.rect();
            Rect::from_min_max(
                (r.min.to_vec2() / scale).to_pos2(),
                (r.max.to_vec2() / scale).to_pos2(),
            )
        })
        .collect();
    let mut shapes = Vec::new();
    for ClippedShape { shape, .. } in &harness.output().shapes {
        flat(shape, &mut shapes);
    }
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == label => {
                let size = text.galley.job.sections[0].format.font_id.size;
                rows.iter()
                    .any(|r| r.contains(text.pos))
                    .then_some((text.pos, size))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_topic_stands_under_its_area_in_smaller_text() -> TestResult {
    let (store, household, _repairs) = store()?;
    let places = Places::new(&store);
    places.set_icon(&household, Some("home"))?;
    let bills = places.add_topic(&household, "Bills")?;
    places.set_topic_icon(&bills.id, Some("mail"))?;
    let harness = window(App::new(store, "en"));
    // The Area's name bold (obf-1157): drawn twice, a little apart.
    let area = drawn_names(&harness, "Household");
    assert_eq!(area.len(), 2, "{area:?}");
    let (area_at, area_size) = area[0];
    // A Topic one text size smaller, its name further right than the
    // Area's, with its icon or without.
    let [(repairs_at, repairs_size)] = drawn_names(&harness, "Repairs")[..] else {
        panic!("Repairs drawn once");
    };
    let [(bills_at, bills_size)] = drawn_names(&harness, "Bills")[..] else {
        panic!("Bills drawn once");
    };
    assert!(repairs_size < area_size, "{repairs_size} < {area_size}");
    assert_eq!(bills_size, repairs_size);
    assert!(repairs_at.x > area_at.x + 1.0, "{repairs_at:?} {area_at:?}");
    // A Topic without an icon keeps its room: the names in one column.
    assert!(
        (repairs_at.x - bills_at.x).abs() < 0.5,
        "{repairs_at:?} {bills_at:?}"
    );
    Ok(())
}
