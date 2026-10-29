//! The capture popup and the Settings' hotkey, driven headless by
//! egui_kittest over an empty store with its clock held still.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use chrono::Duration;
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, Note, Notes, SigningKey, Store, dart_iso, parse_time};
use oldbrainfix_gui::hotkey::{Chord, Control, Status};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

struct Popup {
    popup: CapturePopup,
    closed: bool,
    saved: Rc<Cell<u32>>,
}

fn typing<'a, S>(harness: &'a Harness<'_, S>) -> egui_kittest::Node<'a> {
    harness.get_by(|n| n.is_focused())
}

fn notes(store: &Store) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).all()?)
}

/// The Notes kept, each with its Nudge.
fn kept(store: &Store) -> Vec<(String, Option<String>)> {
    notes(store)
        .unwrap_or_default()
        .into_iter()
        .map(|n| {
            let nudge = store.current(&n.id, "nudge").ok().flatten();
            (n.text, nudge)
        })
        .collect()
}

/// A popup over a store the test can look into.
fn popup_over(store: Arc<Store>, locale: &str) -> Harness<'static, Popup> {
    let mut popup = CapturePopup::new(store, locale);
    let saved = Rc::new(Cell::new(0));
    let count = saved.clone();
    popup.on_saved(Box::new(move || count.set(count.get() + 1)));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 340.0))
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
                saved,
            },
        );
    harness.run();
    harness
}

#[test]
fn enter_keeps_the_typed_thought_with_a_nudge_in_an_hour() -> TestResult {
    let store = store()?;
    let now = store.now();
    let mut harness = popup_over(store.clone(), "en");
    // The field has the keyboard at once; nothing typed, Enter waits.
    harness.key_press(Key::Enter);
    harness.run();
    assert!(!harness.state().closed);
    typing(&harness).type_text("  Buy milk ");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().closed);
    assert_eq!(harness.state().saved.get(), 1);
    assert_eq!(harness.state().popup.text(), "");
    let note = notes(&store)?;
    assert_eq!(note.len(), 1);
    assert!(note[0].is_unsorted());
    assert_eq!(
        kept(&store),
        [(
            "Buy milk".to_string(),
            Some(dart_iso(now + Duration::hours(1)))
        )]
    );
    Ok(())
}

#[test]
fn the_nudge_buttons_keep_it_with_their_time() -> TestResult {
    let store = store()?;
    let now = store.now();
    let mut harness = popup_over(store.clone(), "en");
    for label in ["Remind me to sort it in", "10 min", "30 min", "1 h"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    // All of it fits the popup's window.
    let hint = harness.get_by_label_contains("Enter saves it with a Nudge in 1 h.");
    assert!(hint.rect().bottom() <= 340.0, "{:?}", hint.rect());
    // Nothing typed: the buttons rest.
    harness.get_by_label("10 min").click();
    harness.run();
    assert!(!harness.state().closed);
    assert!(notes(&store)?.is_empty());
    typing(&harness).type_text("Call Anna");
    harness.run();
    harness.get_by_label("30 min").click();
    harness.run();
    assert!(harness.state().closed);
    assert_eq!(
        kept(&store),
        [(
            "Call Anna".to_string(),
            Some(dart_iso(now + Duration::minutes(30)))
        )]
    );
    Ok(())
}

#[test]
fn enter_on_a_nudge_button_takes_that_buttons_time() -> TestResult {
    let store = store()?;
    let now = store.now();
    let mut harness = popup_over(store.clone(), "en");
    typing(&harness).type_text("Water the plants");
    harness.run();
    // The Note card's buttons come first, above the Nudge times.
    for label in [
        "File into…",
        "Routine",
        "Reminder",
        "Frog",
        "Session",
        "More",
        "10 min",
    ] {
        harness.key_press(Key::Tab);
        harness.run();
        assert!(harness.get_by_label(label).is_focused(), "{label}");
    }
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().closed);
    assert_eq!(
        kept(&store),
        [(
            "Water the plants".to_string(),
            Some(dart_iso(now + Duration::minutes(10)))
        )]
    );
    Ok(())
}

#[test]
fn leaving_with_text_keeps_it_and_empty_just_closes() -> TestResult {
    let store = store()?;
    let now = store.now();
    let mut harness = popup_over(store.clone(), "en");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    assert!(notes(&store)?.is_empty());
    assert_eq!(harness.state().saved.get(), 0);

    let mut harness = popup_over(store.clone(), "en");
    typing(&harness).type_text("Descaler");
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    assert_eq!(
        kept(&store),
        [(
            "Descaler".to_string(),
            Some(dart_iso(now + Duration::hours(1)))
        )]
    );
    // The window going away keeps it the same way.
    let mut harness = popup_over(store.clone(), "en");
    typing(&harness).type_text("Stamps");
    harness.run();
    assert_eq!(harness.state_mut().popup.leave(), Outcome::Closed);
    assert_eq!(notes(&store)?.len(), 2);
    Ok(())
}

#[test]
fn the_popup_speaks_the_apps_language() -> TestResult {
    let harness = popup_over(store()?, "de");
    assert!(
        harness
            .query_by(|n| n.placeholder() == Some("Was geht dir durch den Kopf?"))
            .is_some()
    );
    assert!(harness.query_by_label("1 Std.").is_some());
    Ok(())
}

/// A hotkey that records where it is moved.
struct FakeHotkey {
    status: Status,
    moved: Vec<Chord>,
}

impl Control for FakeHotkey {
    fn status(&self) -> Status {
        self.status.clone()
    }

    fn rebind(&mut self, chord: &Chord) {
        self.moved.push(*chord);
        self.status = Status::Taken;
    }
}

/// The Settings with a fake hotkey, and that hotkey.
type Settings = (Harness<'static, App>, Rc<RefCell<FakeHotkey>>);

fn settings(status: Status) -> Result<Settings, Box<dyn std::error::Error>> {
    let fake = Rc::new(RefCell::new(FakeHotkey {
        status,
        moved: Vec::new(),
    }));
    let mut app = App::new(store()?, "en");
    let control: Rc<RefCell<dyn Control>> = fake.clone();
    app.set_hotkey(Some(control));
    app.select(Selection::Settings);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    Ok((harness, fake))
}

fn holding<'a>(harness: &'a Harness<'_, App>, value: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(move |n| n.role() == Role::TextInput && n.value().as_deref() == Some(value))
}

#[test]
fn the_settings_move_the_hotkey_and_say_how_it_stands() -> TestResult {
    let (mut harness, fake) = settings(Status::Active)?;
    assert!(harness.query_by_label("Quick capture").is_some());
    assert!(
        harness
            .query_by_label("Works anywhere on this computer.")
            .is_some()
    );
    assert!(
        harness
            .query_by_label_contains("oldbrainfix capture")
            .is_some()
    );
    holding(&harness, "Ctrl+Alt+N").click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.run();
    typing(&harness).type_text("super+shift+");
    harness.run();
    // Half a chord is not kept.
    assert!(
        harness
            .query_by_label("Type a key with Ctrl, Alt or Super, like Ctrl+Alt+N.")
            .is_some()
    );
    assert!(fake.borrow().moved.is_empty());
    typing(&harness).type_text("k");
    harness.run();
    assert!(fake.borrow().moved.is_empty());
    // Enter, or leaving the field, keeps it.
    harness.key_press(Key::Enter);
    harness.run();
    let stored = DeviceSettings::new(harness.state().store()).capture_hotkey()?;
    assert_eq!(stored, "Shift+Super+K");
    assert_eq!(
        fake.borrow()
            .moved
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>(),
        ["Shift+Super+K"]
    );
    assert!(
        harness
            .query_by_label("Another program already uses this key. Pick another one.")
            .is_some()
    );
    Ok(())
}

#[test]
fn on_wayland_the_settings_say_the_desktop_holds_the_key() -> TestResult {
    let (harness, _) = settings(Status::Desktop(Some("Meta+Ctrl+N".into())))?;
    assert!(
        harness
            .query_by_label_contains("Your desktop manages this key")
            .is_some()
    );
    assert!(
        harness
            .query_by_label_contains("Your desktop put it on Meta+Ctrl+N.")
            .is_some()
    );
    let (harness, _) = settings(Status::Unavailable)?;
    assert!(
        harness
            .query_by_label_contains("No hotkey on this desktop.")
            .is_some()
    );
    Ok(())
}
