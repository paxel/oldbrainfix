//! File into… on the desktop (#272), driven headless by egui_kittest: a
//! filter that has the keyboard, an Area that matches with all its
//! Topics, else the Topics that match, "Filed into recently" with the
//! last three places this Device filed into while nothing is typed,
//! Enter taking the first row shown, Esc leaving, New Area… with the
//! filter's text filled in and New Topic… in Advanced only, both making
//! the place and filing into it, and Cancel there making nothing. The
//! quick note and the capture popup pick their place the same way.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{
    DeviceSettings, Notes, NotesGo, OwnProfile, PersonSettings, Places, QuickNoteControl,
    SigningKey, Store,
};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MILK: &str = "Buy milk";

/// Household with its Topics Kitchen and Cellar, Garden with its Topic
/// Kitchen garden, and the Unsorted Note "Buy milk".
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let now = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    places.add_topic(&house.id, "Kitchen")?;
    places.add_topic(&house.id, "Cellar")?;
    let garden = places.add_area("Garden")?;
    places.add_topic(&garden.id, "Kitchen garden")?;
    Notes::new(&store).capture(MILK, None, None)?;
    Ok(store)
}

fn harness(store: Store) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(Selection::Unsorted);
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

/// The id of the Area or Topic named `name`.
fn place(store: &Store, name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let places = Places::new(store);
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

/// Where "Buy milk" is filed.
fn milk_place(harness: &Harness<'_, App>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(Notes::new(harness.state().store())
        .all()?
        .into_iter()
        .find(|n| n.text == MILK)
        .ok_or("no Buy milk")?
        .place)
}

/// File into… for "Buy milk": its card selected, then Ctrl+M.
fn open_file_into(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
    harness.get_by_role_and_label(Role::Button, MILK).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::M);
    harness.run_steps(8);
}

/// Types `text` where the keyboard is: the filter, as it opens.
fn type_here<S>(harness: &mut Harness<'_, S>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// How many buttons named `label` show.
fn buttons<S>(harness: &Harness<'_, S>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

/// The last button named `label`, the one in File into… over the
/// sidebar's.
fn press<S>(harness: &mut Harness<'_, S>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next_back()
        .ok_or_else(|| format!("no {label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The text field holding `value`.
fn field_with<S>(harness: &Harness<'_, S>, value: &str) -> bool {
    harness
        .query_all_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some(value))
        .next()
        .is_some()
}

#[test]
fn the_filter_has_the_keyboard_and_narrows_the_areas_and_topics() -> TestResult {
    let mut harness = harness(store()?);
    // What the sidebar shows without File into….
    let names = ["Household", "Kitchen", "Cellar", "Garden", "Kitchen garden"];
    let before: Vec<usize> = names.iter().map(|n| buttons(&harness, n)).collect();
    let listed = |harness: &Harness<'_, App>| -> Vec<usize> {
        names
            .iter()
            .zip(&before)
            .map(|(n, b)| buttons(harness, n) - b)
            .collect()
    };
    open_file_into(&mut harness);
    assert!(shows(&harness, "File into…"));
    assert_eq!(listed(&harness), [1, 1, 1, 1, 1]);
    // Typed: an Area whose name matches, with all its Topics.
    type_here(&mut harness, "house");
    assert_eq!(listed(&harness), [1, 1, 1, 0, 0]);
    // Else the Topics whose names match, under their Area.
    for _ in 0.."house".len() {
        harness.key_press(Key::Backspace);
        harness.run();
    }
    type_here(&mut harness, "kitch");
    assert_eq!(listed(&harness), [1, 1, 0, 1, 1]);
    // Nothing matches: said, and New Area… is still there.
    type_here(&mut harness, "zzz");
    assert!(shows(&harness, "No Area or Topic matches."));
    assert_eq!(buttons(&harness, "New Area…"), 1);
    Ok(())
}

#[test]
fn enter_files_into_the_first_match_and_esc_files_nowhere() -> TestResult {
    let store = store()?;
    let kitchen = place(&store, "Kitchen garden")?;
    let household = place(&store, "Household")?;
    let mut harness = harness(store);
    // Esc leaves, nothing filed.
    open_file_into(&mut harness);
    type_here(&mut harness, "kit");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shows(&harness, "Filter Areas and Topics"));
    assert_eq!(milk_place(&harness)?, None);
    // Only Topics match "kit", Garden's Kitchen garden first, by the
    // Area's name: it, not Garden, is the first match.
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    open_file_into(&mut harness);
    type_here(&mut harness, "kit");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(milk_place(&harness)?, Some(kitchen.clone()));
    // An Area that matches is its own first match.
    harness
        .state_mut()
        .select(Selection::Place(kitchen.clone()));
    harness.run();
    open_file_into(&mut harness);
    type_here(&mut harness, "hou");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(milk_place(&harness)?, Some(household.clone()));
    // Nothing typed: Enter files into the place filed into last.
    let cellar = place(harness.state().store(), "Cellar")?;
    DeviceSettings::new(harness.state().store()).filed_into(&cellar)?;
    harness.state_mut().select(Selection::Place(household));
    harness.run();
    open_file_into(&mut harness);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(milk_place(&harness)?, Some(cellar));
    Ok(())
}

#[test]
fn the_places_filed_into_last_show_at_the_top_until_something_is_typed() -> TestResult {
    let store = store()?;
    let settings = DeviceSettings::new(&store);
    let garden = place(&store, "Garden")?;
    let cellar = place(&store, "Cellar")?;
    let gone = Places::new(&store).add_area("Attic")?.id;
    for id in [&garden, &gone, &cellar] {
        settings.filed_into(id)?;
    }
    Places::new(&store).delete_area(&gone, NotesGo::Unsorted)?;
    let mut harness = harness(store);
    let cellars = buttons(&harness, "Cellar");
    let gardens = buttons(&harness, "Garden");
    open_file_into(&mut harness);
    assert!(shows(&harness, "Filed into recently"));
    // Cellar and Garden twice each, at the top and in their Area; the
    // Attic deleted since is not offered.
    assert_eq!(buttons(&harness, "Cellar"), cellars + 2);
    assert_eq!(buttons(&harness, "Garden"), gardens + 2);
    assert!(!shows(&harness, "Attic"));
    // The latest first, a Topic with its Area beside it.
    let top = |harness: &Harness<'_, App>, name: &str| {
        harness
            .query_all_by_role_and_label(Role::Button, name)
            .map(|n| n.rect())
            // The dialog's, not the sidebar's.
            .filter(|r| r.left() > 300.0)
            .map(|r| r.top())
            .fold(f32::MAX, f32::min)
    };
    let filter = harness.get_by(|n| n.is_focused()).rect().bottom();
    assert!(filter < top(&harness, "Cellar"));
    assert!(top(&harness, "Cellar") < top(&harness, "Garden"));
    // Hidden while filtering.
    type_here(&mut harness, "cel");
    assert!(!shows(&harness, "Filed into recently"));
    assert_eq!(buttons(&harness, "Cellar"), cellars + 1);
    // A pick is filed into last.
    press(&mut harness, "Cellar")?;
    let recent = DeviceSettings::new(harness.state().store()).recently_filed_into()?;
    assert_eq!(recent, [cellar, gone, garden]);
    Ok(())
}

#[test]
fn new_area_takes_the_filters_text_makes_the_area_and_files_into_it() -> TestResult {
    let mut harness = harness(store()?);
    open_file_into(&mut harness);
    type_here(&mut harness, "Errands");
    // Nothing matches: Enter asks the name, as New Area… does.
    harness.key_press(Key::Enter);
    harness.run();
    assert!(shows(&harness, "New Area"));
    assert!(field_with(&harness, "Errands"));
    // Cancel makes nothing and stays in File into….
    press(&mut harness, "Cancel")?;
    assert!(shows(&harness, "Filter Areas and Topics"));
    assert!(field_with(&harness, "Errands"));
    assert!(place(harness.state().store(), "Errands").is_err());
    // Esc while naming goes back to the list, too.
    press(&mut harness, "New Area…")?;
    harness.key_press(Key::Escape);
    harness.run();
    assert!(shows(&harness, "Filter Areas and Topics"));
    assert!(place(harness.state().store(), "Errands").is_err());
    // Create makes it and files into it.
    press(&mut harness, "New Area…")?;
    press(&mut harness, "Create")?;
    let errands = place(harness.state().store(), "Errands")?;
    assert_eq!(milk_place(&harness)?, Some(errands.clone()));
    assert!(!shows(&harness, "Filter Areas and Topics"));
    let recent = DeviceSettings::new(harness.state().store()).recently_filed_into()?;
    assert_eq!(recent, [errands]);
    Ok(())
}

#[test]
fn new_topic_ends_each_area_in_advanced_only_and_files_into_the_topic() -> TestResult {
    let mut harness = harness(store()?);
    open_file_into(&mut harness);
    assert_eq!(buttons(&harness, "New Topic…"), 2);
    // Under Garden, the second Area by name: Garden, Household.
    harness
        .query_all_by_role_and_label(Role::Button, "New Topic…")
        .next()
        .ok_or("New Topic… under Garden")?
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "New Topic"));
    type_here(&mut harness, "Shed");
    harness.key_press(Key::Enter);
    harness.run();
    let store = harness.state().store();
    let shed = place(store, "Shed")?;
    assert_eq!(store.area_of(&shed)?, Some(place(store, "Garden")?));
    assert_eq!(milk_place(&harness)?, Some(shed));

    // Not in Basic.
    let store = self::store()?;
    PersonSettings::new(&store).set_advanced(false)?;
    let mut harness = self::harness(store);
    open_file_into(&mut harness);
    assert!(shows(&harness, "New Area…"));
    assert_eq!(buttons(&harness, "New Topic…"), 0);
    Ok(())
}

/// The quick note with "File into…" switched on, "Buy bread" typed.
fn quick_note() -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let store = store()?;
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    let mut harness = harness(store);
    harness.state_mut().new_note();
    harness.run();
    harness
        .get_by(|n| n.placeholder() == Some("What's on your mind?"))
        .click();
    harness.run();
    type_here(&mut harness, "Buy bread");
    Ok(harness)
}

#[test]
fn the_quick_note_picks_its_place_in_file_into_and_a_new_area_stays() -> TestResult {
    let mut harness = quick_note()?;
    press(&mut harness, "File into…")?;
    harness.run_steps(8);
    // Over the quick note, the filter has the keyboard.
    assert!(harness.state().quick_note_open());
    type_here(&mut harness, "cell");
    harness.key_press(Key::Enter);
    harness.run();
    assert!(shows(&harness, "Into: Household / Cellar"));
    let cellar = place(harness.state().store(), "Cellar")?;
    let recent = DeviceSettings::new(harness.state().store()).recently_filed_into()?;
    assert_eq!(recent, [cellar]);
    // A new Area is made at once and kept, the Note saved or not.
    press(&mut harness, "File into…")?;
    harness.run_steps(8);
    type_here(&mut harness, "Errands");
    press(&mut harness, "New Area…")?;
    press(&mut harness, "Create")?;
    assert!(shows(&harness, "Into: Errands"));
    assert!(harness.state().quick_note_open());
    assert!(place(harness.state().store(), "Errands").is_ok());
    let bread = Notes::new(harness.state().store())
        .all()?
        .into_iter()
        .any(|n| n.text == "Buy bread");
    assert!(!bread, "not saved yet");
    Ok(())
}

struct Popup {
    popup: CapturePopup,
    closed: bool,
}

#[test]
fn the_popup_picks_its_place_on_a_page_and_a_new_area_stays_unsaved() -> TestResult {
    let store = Arc::new(store()?);
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    let popup = CapturePopup::new(store.clone(), "en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 900.0))
        .build_ui_state(
            |ui, state: &mut Popup| {
                App::install(ui.ctx());
                if state.popup.show(ui) == Outcome::Closed {
                    state.closed = true;
                }
            },
            Popup {
                popup,
                closed: false,
            },
        );
    harness.run();
    type_here(&mut harness, "Buy bread");
    press(&mut harness, "File into…")?;
    assert!(harness.state().popup.page_open());
    // Esc on the page goes back to the capture, nothing picked.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().popup.page_open());
    assert!(!harness.state().closed);
    assert_eq!(harness.state().popup.choices().place, None);
    press(&mut harness, "File into…")?;
    type_here(&mut harness, "Errands");
    press(&mut harness, "New Area…")?;
    assert!(field_with(&harness, "Errands"));
    // Esc while naming goes back to the list, not to the capture.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().popup.page_open());
    assert!(shows(&harness, "Filter Areas and Topics"));
    press(&mut harness, "New Area…")?;
    press(&mut harness, "Create")?;
    assert!(!harness.state().popup.page_open());
    let errands = place(&store, "Errands")?;
    assert_eq!(harness.state().popup.choices().place, Some(errands));
    assert!(shows(&harness, "Into: Errands"));
    // The text taken away, nothing is saved: the Area stays.
    harness.state_mut().popup = CapturePopup::new(store.clone(), "en");
    harness.run();
    assert!(place(&store, "Errands").is_ok());
    assert!(
        !Notes::new(&store)
            .all()?
            .into_iter()
            .any(|n| n.text == "Buy bread")
    );
    Ok(())
}

#[test]
fn file_into_opens_at_the_top_of_its_list_however_far_it_was_scrolled() -> TestResult {
    let store = store()?;
    for i in 1..=20 {
        Places::new(&store).add_area(&format!("Zone {i:02}"))?;
    }
    let mut harness = harness(store);
    open_file_into(&mut harness);
    harness
        .query_all_by_role_and_label(Role::Button, "Zone 20")
        .next_back()
        .ok_or("File into…'s Zone 20")?
        .scroll_to_me();
    harness.run();
    // Scrolled down to it, Zone 20 is picked with the pointer.
    harness
        .query_all_by_role_and_label(Role::Button, "Zone 20")
        .next_back()
        .ok_or("File into…'s Zone 20")?
        .click();
    harness.run();
    let zone = place(harness.state().store(), "Zone 20")?;
    assert_eq!(milk_place(&harness)?, Some(zone.clone()));
    // Open again, the places filed into recently show first, not the
    // end of the list where it was left.
    harness.state_mut().select(Selection::Place(zone));
    harness.run();
    open_file_into(&mut harness);
    let filter = harness.get_by(|n| n.is_focused()).rect().bottom();
    let heading = harness.get_by_label("Filed into recently").rect().top();
    assert!(heading >= filter, "{heading} under {filter}");
    Ok(())
}
