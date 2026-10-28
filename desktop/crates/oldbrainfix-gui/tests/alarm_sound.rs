//! The Settings' alarm sound driven headless (#102): Old Brain Fix's own
//! chime until a sound file is chosen in the system's dialog, the file
//! named once chosen, one gone or not playable said to ring the own chime
//! instead, and the way back to the own chime, in English and German.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, SigningKey, Store};
use oldbrainfix_gui::ring::OWN_SOUND;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The Settings over `store`, the system's dialog handing back `picked`
/// in turn.
fn harness(
    store: &Arc<Store>,
    locale: &str,
    picked: Vec<PathBuf>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store.clone(), locale);
    let picked = Rc::new(RefCell::new(picked.into_iter()));
    app.set_sound_picker(Box::new(move |_title| picked.borrow_mut().next()));
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
    Ok(harness)
}

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    Ok(Arc::new(Store::in_memory(
        Box::new(chrono::Utc::now),
        || Ok(SigningKey::from_seed([4; 32])),
    )?))
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

#[test]
fn a_sound_file_chosen_rings_until_the_own_chime_is_back() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let bell = dir.path().join("bell.wav");
    std::fs::write(&bell, OWN_SOUND)?;
    let broken = dir.path().join("broken.mp3");
    std::fs::write(&broken, b"not a sound at all")?;
    let mut harness = harness(&store, "en", vec![bell.clone(), broken.clone()])?;
    assert!(shows(&harness, "Alarm sound"));
    assert!(shows(&harness, "Old Brain Fix's own chime."));
    assert!(!shows(&harness, "Use the own chime"));
    harness
        .get_by_label("Choose a sound file…")
        .click_accesskit();
    harness.run();
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.alarm_sound()?, bell.to_str().map(String::from));
    assert!(shows(&harness, "The sound file bell.wav."));
    assert!(!shows(&harness, "Old Brain Fix's own chime."));
    assert!(!shows(&harness, "cannot be read or played"));
    // The file deleted: the own chime rings, and the Settings say so.
    std::fs::remove_file(&bell)?;
    harness.run();
    assert!(shows(&harness, "cannot be read or played right now"));
    harness.get_by_label("Use the own chime").click_accesskit();
    harness.run();
    assert_eq!(settings.alarm_sound()?, None);
    assert!(shows(&harness, "Old Brain Fix's own chime."));
    assert!(!shows(&harness, "cannot be read or played"));
    // A file that is no sound is kept, and said to ring the own chime.
    harness
        .get_by_label("Choose a sound file…")
        .click_accesskit();
    harness.run();
    assert_eq!(settings.alarm_sound()?, broken.to_str().map(String::from));
    assert!(shows(&harness, "The sound file broken.mp3."));
    assert!(shows(&harness, "cannot be read or played right now"));
    // The dialog cancelled changes nothing.
    harness
        .get_by_label("Choose a sound file…")
        .click_accesskit();
    harness.run();
    assert_eq!(settings.alarm_sound()?, broken.to_str().map(String::from));
    Ok(())
}

#[test]
fn the_alarm_sound_speaks_german() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let bell = dir.path().join("glocke.wav");
    std::fs::write(&bell, OWN_SOUND)?;
    let mut harness = harness(&store, "de", vec![bell])?;
    assert!(shows(&harness, "Alarmton"));
    assert!(shows(&harness, "Der eigene Klang von Old Brain Fix."));
    harness.get_by_label("Tondatei wählen…").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Die Tondatei glocke.wav."));
    harness
        .get_by_label("Eigenen Klang verwenden")
        .click_accesskit();
    harness.run();
    assert_eq!(DeviceSettings::new(&store).alarm_sound()?, None);
    Ok(())
}
