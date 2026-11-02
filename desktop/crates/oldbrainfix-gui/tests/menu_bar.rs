//! The window's frame driven headless (#145): the menu bar's File, Edit,
//! View, Note, Sync and Help, each entry reaching what it names; their
//! keys; the dialogs that hold the Settings and everything under Sync,
//! closed by their button, Esc and back, and back to the one they were
//! opened from; and the sidebar marking the place the list shows.

use std::sync::{Arc, Mutex};

use egui::accesskit::Toggled;
use egui::{Key, Modifiers, ViewportCommand, ViewportId};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store with the Area Household, its Topic Repairs holding "Fix the
/// tap", and the Unsorted Note "Call the bank"; the ids of Repairs and
/// of the tap's Note.
fn store() -> Result<(Store, String, String), Box<dyn std::error::Error>> {
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
    Ok((store, repairs.id, tap.id))
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
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The last widget labelled `label`: a menu's entry, drawn over the
/// window's buttons of the same name.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

/// The menu bar's `menu`, ahead of the window's buttons of the same name.
fn bar<'a>(harness: &'a Harness<'_, App>, menu: &'a str) -> Node<'a> {
    match harness.get_all_by_label(menu).next() {
        Some(node) => node,
        None => panic!("no menu {menu}"),
    }
}

/// Opens the menu bar's `menu`.
fn open(harness: &mut Harness<'_, App>, menu: &str) {
    bar(harness, menu).click();
    harness.run();
}

/// Picks `entry` of the menu bar's `menu`, one step: what it asked is
/// done in that frame.
fn pick(harness: &mut Harness<'_, App>, menu: &str, entry: &str) {
    open(harness, menu);
    last(harness, entry).click();
    harness.step();
}

/// Whether `entry` of `menu` can be picked; the menu is closed again.
fn enabled(harness: &mut Harness<'_, App>, menu: &str, entry: &str) -> bool {
    open(harness, menu);
    let on = !last(harness, entry).accesskit_node().is_disabled();
    bar(harness, menu).click();
    harness.run();
    on
}

/// The label of the widget with the keyboard.
fn focused(harness: &Harness<'_, App>) -> Option<String> {
    let node = harness.query_by(|n| n.is_focused())?.accesskit_node();
    node.label()
        .or_else(|| node.placeholder().map(str::to_string))
}

#[test]
fn the_file_menu_reaches_new_note_quick_note_import_and_quit() -> TestResult {
    let (store, _, _) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Frogs);
    let mut harness = window(app);
    // New Note opens the quick note's dialog over the list (#146).
    pick(&mut harness, "File", "New Note Ctrl+N");
    harness.run();
    assert!(harness.state().quick_note_open());
    assert_eq!(harness.state().selection(), &Selection::Frogs);
    assert_eq!(focused(&harness).as_deref(), Some("Capture"));
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().quick_note_open());

    assert!(!harness.state_mut().take_quick_note());
    pick(&mut harness, "File", "Quick note");
    assert!(harness.state_mut().take_quick_note());
    assert!(!harness.state_mut().take_quick_note());

    pick(&mut harness, "File", "Import…");
    harness.run();
    assert!(shows(&harness, "A folder…"));
    assert!(shows(&harness, "Google Keep…"));
    assert!(shows(
        &harness,
        "Each Markdown or text file in it becomes a Note"
    ));
    harness.get_by_label("Close").click();
    harness.run();
    assert!(!shows(&harness, "A folder…"));

    // Save as Markdown… wants a Note.
    assert!(!enabled(&mut harness, "File", "Save as Markdown…"));
    assert!(!harness.state_mut().take_quit());
    pick(&mut harness, "File", "Quit");
    assert!(harness.state_mut().take_quit());
    Ok(())
}

#[test]
fn the_edit_menu_finds_opens_the_settings_and_hands_the_clipboard_to_the_field() -> TestResult {
    let (store, _, tap) = store()?;
    let mut harness = window(App::new(store, "en"));
    // Cut, Copy and Paste want a field the cursor was in: the Note
    // dialog's, under the menu bar at hand (#146).
    assert!(!enabled(&mut harness, "Edit", "Copy"));
    harness.state_mut().select_note(&tap);
    harness.run();
    harness.get_by_label("Note text").click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(" now");
    harness.run();
    pick(&mut harness, "Edit", "Copy");
    let asked = &harness.output().viewport_output[&ViewportId::ROOT].commands;
    assert!(asked.contains(&ViewportCommand::RequestCopy), "{asked:?}");
    harness.run();
    // The cursor is back in the field.
    assert_eq!(focused(&harness).as_deref(), Some("Note text"));

    // Find: All Notes' search on the Agenda, which has none.
    pick(&mut harness, "Edit", "Find Ctrl+F");
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(focused(&harness).as_deref(), Some("Search all Notes"));

    pick(&mut harness, "Edit", "Settings… Ctrl+,");
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::Settings));
    assert!(shows(&harness, "Notifications on this computer"));
    // The list stays where it was under it.
    assert_eq!(harness.state().selection(), &Selection::All);
    Ok(())
}

#[test]
fn the_view_menu_shows_the_places_and_switches_the_cards() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(App::new(store, "en"));
    for (entry, place) in [
        ("Unsorted", Selection::Unsorted),
        ("All Notes", Selection::All),
        ("Frogs", Selection::Frogs),
        ("All Routines and Reminders", Selection::AllRoutines),
        ("Agenda", Selection::Agenda),
    ] {
        pick(&mut harness, "View", entry);
        harness.run();
        assert_eq!(harness.state().selection(), &place, "{entry}");
    }
    // The Agenda has no cards to switch.
    assert!(!enabled(&mut harness, "View", "Compact views"));
    pick(&mut harness, "View", "All Notes");
    harness.run();
    assert!(!harness.state().compact());
    pick(&mut harness, "View", "Compact views");
    harness.run();
    assert!(harness.state().compact());
    pick(&mut harness, "View", "Detailed views");
    harness.run();
    assert!(!harness.state().compact());
    // The theme: the system's, light or dark (#147).
    open(&mut harness, "View");
    last(&harness, "Theme ⏵").hover();
    harness.run();
    for entry in ["System", "Light", "Dark"] {
        assert!(shows(&harness, entry), "{entry}");
    }
    Ok(())
}

#[test]
fn the_note_menu_is_greyed_without_a_note_and_acts_on_the_one_shown() -> TestResult {
    let (store, repairs, tap) = store()?;
    let household = Places::new(&store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Household")
        .ok_or("Household")?
        .id;
    let mut harness = window(App::new(store, "en"));
    for entry in [
        "File into… Ctrl+M",
        "Routine…",
        "Reminder…",
        "Make it a Checklist",
        "Send to a person",
        "History",
        "Delete",
    ] {
        assert!(!enabled(&mut harness, "Note", entry), "{entry}");
    }

    harness.state_mut().select(Selection::Place(repairs));
    harness.state_mut().select_note(&tap);
    harness.run();
    for entry in [
        "File into… Ctrl+M",
        "Routine…",
        "Reminder…",
        "Frog",
        "Session",
        "Make it a Checklist",
        "Make it a Shopping List",
        "Make it a Tally",
        "Send to a person",
        "Copy as text",
        "History",
        "Delete",
    ] {
        assert!(enabled(&mut harness, "Note", entry), "{entry}");
    }
    assert!(enabled(&mut harness, "File", "Save as Markdown…"));

    // File into… asks where, and files it there.
    pick(&mut harness, "Note", "File into… Ctrl+M");
    harness.run();
    last(&harness, "Household").click();
    harness.run();
    let filed = Notes::new(harness.state().store())
        .by_id(&tap)?
        .ok_or("the Note")?;
    assert_eq!(filed.place.as_deref(), Some(household.as_str()));

    pick(&mut harness, "Note", "Routine…");
    harness.run();
    assert!(harness.state().routine_editor().is_some());
    // Its dialog over the Note's, back to it with Esc.
    harness.key_press(Key::Escape);
    harness.run();

    pick(&mut harness, "Note", "Send to a person");
    harness.run();
    assert!(shows(&harness, "Send this Note"));
    harness.key_press(Key::Escape);
    harness.run();

    pick(&mut harness, "Note", "Delete");
    harness.run();
    assert!(shows(&harness, "Delete this Note?"));
    Ok(())
}

#[test]
fn the_sync_menu_opens_its_dialogs() -> TestResult {
    let (store, _, _) = store()?;
    let mut app = App::new(store, "en");
    app.set_sync_host(
        Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        "Desk",
    );
    let mut harness = window(app);
    for (entry, page) in [
        ("Sync now", Page::Sync),
        ("Pair a Device", Page::Pair),
        ("People", Page::People),
        ("Profile", Page::Profile),
        ("Backups…", Page::Backups),
    ] {
        pick(&mut harness, "Sync", entry);
        harness.step();
        assert_eq!(harness.state().page(), Some(page), "{entry}");
        // The list stays as it was under it.
        assert_eq!(harness.state().selection(), &Selection::Agenda);
        harness.key_press(Key::Escape);
        harness.step();
        harness.step();
        assert_eq!(harness.state().page(), None, "{entry}");
    }
    // Pair a Device takes the other Device's code; Sync lists the paired.
    pick(&mut harness, "Sync", "Pair a Device");
    harness.step();
    harness.step();
    assert!(shows(&harness, "Sync with this code"));
    assert!(!shows(&harness, "Paired Devices"));
    assert!(harness.state().sync_host().hosting());
    harness.key_press(Key::Escape);
    harness.step();
    assert!(!harness.state().sync_host().hosting());
    pick(&mut harness, "Sync", "Sync now");
    harness.step();
    harness.step();
    assert!(shows(&harness, "Paired Devices"));
    assert!(!shows(&harness, "Sync with this code"));
    // The Import left the Sync dialog for the File menu.
    assert!(!shows(&harness, "A folder…"));
    Ok(())
}

#[test]
fn the_help_menu_opens_the_guide_in_the_browser_and_about() -> TestResult {
    let (store, _, _) = store()?;
    let mut app = App::new(store, "de");
    let opened = Arc::new(Mutex::new(Vec::new()));
    let kept = opened.clone();
    app.set_url_opener(Box::new(move |url| {
        if let Ok(mut kept) = kept.lock() {
            kept.push(url.to_string());
        }
    }));
    let mut harness = window(app);
    pick(&mut harness, "Hilfe", "Anleitung");
    assert_eq!(
        *opened.lock().map_err(|e| e.to_string())?,
        ["https://paxel.github.io/oldbrainfix/de/guide/desktop-app.html"]
    );
    pick(&mut harness, "Hilfe", "Über Old Brain Fix");
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::About));
    assert!(shows(&harness, "Open-Source-Lizenzen"));
    Ok(())
}

#[test]
fn the_keys_open_what_their_entries_open() -> TestResult {
    let (store, repairs, tap) = store()?;
    let mut app = App::new(store, "en");
    app.select(Selection::Trash);
    let mut harness = window(app);
    let press = |harness: &mut Harness<'_, App>, key| {
        harness.key_press_modifiers(Modifiers::COMMAND, key);
        harness.run();
    };
    press(&mut harness, Key::N);
    assert!(harness.state().quick_note_open());
    assert_eq!(focused(&harness).as_deref(), Some("Capture"));
    // The keys rest over the quick note's dialog.
    press(&mut harness, Key::F);
    assert_eq!(harness.state().selection(), &Selection::Trash);
    harness.key_press(Key::Escape);
    harness.run();
    press(&mut harness, Key::F);
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(focused(&harness).as_deref(), Some("Search all Notes"));
    // Ctrl+M wants a Note.
    press(&mut harness, Key::M);
    assert!(!shows(&harness, "Cancel"));
    harness.state_mut().select(Selection::Place(repairs));
    harness.state_mut().select_note(&tap);
    harness.run();
    press(&mut harness, Key::M);
    assert!(shows(&harness, "Cancel"));
    assert_eq!(harness.get_all_by_label("Household").count(), 2);
    harness.get_by_label("Cancel").click();
    harness.run();
    press(&mut harness, Key::Comma);
    assert_eq!(harness.state().page(), Some(Page::Settings));
    Ok(())
}

#[test]
fn a_dialog_closes_by_its_button_esc_and_back_and_leads_back_to_the_one_it_came_from() -> TestResult
{
    let (store, _, _) = store()?;
    let mut app = App::new(store, "en");
    app.open(Page::Settings);
    let mut harness = window(app);
    harness.get_by_label("Close Settings").click();
    harness.run();
    assert_eq!(harness.state().page(), None);

    harness.state_mut().open(Page::Settings);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().page(), None);

    // About from the Settings, the licences from About; back leads
    // through them, by the button and by Alt+Left.
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert!(harness.query_by_label("Back to Settings").is_none());
    harness.get_by_label("About Old Brain Fix").click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::About));
    harness.get_by_label("Open-source licenses").click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::Licences));
    harness.get_by_label("Back to About Old Brain Fix").click();
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::About));
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().page(), Some(Page::Settings));
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().page(), None);
    // The list did not move meanwhile.
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    assert!(!harness.state().can_go_back());
    Ok(())
}

#[test]
fn the_sidebar_marks_the_place_shown() -> TestResult {
    let (store, _, _) = store()?;
    let mut harness = window(App::new(store, "en"));
    // The heading of a list names it too, unmarked.
    let marked = |harness: &Harness<'_, App>, label: &str| {
        harness
            .get_all_by_label(label)
            .any(|n| n.accesskit_node().toggled() == Some(Toggled::True))
    };
    assert!(marked(&harness, "Agenda"));
    for place in [
        "Unsorted",
        "All Notes",
        "Frogs",
        "Household",
        "Repairs",
        "Trash",
    ] {
        harness.get_by_label(place).click();
        harness.run();
        assert!(marked(&harness, place), "{place}");
        assert!(!marked(&harness, "Agenda"), "{place}");
    }
    // The sidebar's order (#145).
    let top = |label: &str| harness.get_by_label(label).rect().top();
    let order = [
        top("Agenda"),
        top("Unsorted"),
        top("All Notes"),
        top("Frogs"),
        top("Household"),
        top("Repairs"),
    ];
    let mut sorted = order;
    sorted.sort_by(f32::total_cmp);
    assert_eq!(order, sorted);
    Ok(())
}
