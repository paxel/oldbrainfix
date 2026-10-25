//! Sent Items in the window, driven headless: a Note is sent from its
//! editor, encrypted to a person met or signed only with a hint; a Sent
//! Item dropped onto the window or handed over shows its preview, and
//! Accept takes it into Unsorted with its Routines while Decline keeps
//! nothing.

use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{
    ClockTime, Day, ImportReport, Notes, PairedDevice, Routines, Schedule, SigningKey, Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(seed: u8, author: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    store.set_local_setting("author", author)?;
    Ok(store)
}

/// `other` is another person's Device paired by invite with `store`, its
/// key met in person.
fn meet(store: &Store, other: &Store, name: &str) -> TestResult {
    let id = other.device_id()?;
    store.remember_device(&PairedDevice {
        id: id.clone(),
        name: name.into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: chrono::Utc::now(),
        shared: true,
    })?;
    store.learn_keys(
        &[other.own_key_record()?],
        Some(&id),
        &mut ImportReport::default(),
    )?;
    Ok(())
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

fn drop_file(harness: &mut Harness<'_, App>, path: &std::path::Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

#[test]
fn a_note_is_saved_to_send_encrypted_to_a_person_met_or_signed_only() -> TestResult {
    let desk = store(1, "Ada")?;
    let ben = store(2, "Ben")?;
    meet(&desk, &ben, "Ben")?;
    let note = Notes::new(&desk).capture("Dentist on Friday", None, None)?;
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("for-ben");
    let mut app = App::new(desk, "en");
    app.select(Selection::All);
    app.select_note(&note.id);
    {
        let save = saved.clone();
        app.set_item_saver(Box::new(move |title, name| {
            assert_eq!(title, "Save the Note to send");
            assert!(name.starts_with("oldbrainfix-note-") && name.ends_with(".oldbrain"));
            Some(save.clone())
        }));
    }
    let mut harness = window(app);

    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Ben, encrypted"));
    assert!(!shows(&harness, "Not encrypted: whoever"));
    harness.get_by_label("Send").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Saved. Send the file"));
    let file = saved.with_extension("oldbrain");
    let bytes = std::fs::read(&file)?;
    assert!(!String::from_utf8_lossy(&bytes).contains("Dentist"));
    let opened = ben.open_sent_item(&bytes)?.map_err(|_| "Ben opens it")?;
    assert!(opened.encrypted);
    assert_eq!(opened.item.text, "Dentist on Friday");

    // To anyone: signed only, and said so before.
    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    harness
        .get_by_label("Anyone, not encrypted")
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "Not encrypted: whoever"));
    harness.get_by_label("Send").click_accesskit();
    harness.run();
    let bytes = std::fs::read(&file)?;
    assert!(String::from_utf8_lossy(&bytes).contains("Dentist"));
    let again = ben.open_sent_item(&bytes)?.map_err(|_| "signed")?;
    assert_eq!(again.item.id, opened.item.id, "the same Sent Item");
    assert_eq!(again.item.version, 2);
    Ok(())
}

#[test]
fn a_sent_item_dropped_or_handed_over_is_previewed_and_accepted_or_declined() -> TestResult {
    let ann = store(3, "Ann")?;
    let note = Notes::new(&ann).capture("Blood test", None, None)?;
    let schedule = Schedule::new(
        Day::new(2026, 11, 2),
        &[ClockTime::new(8, 15).ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    Routines::new(&ann).spawn(&note.id, &schedule)?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("from-ann.oldbrain");
    std::fs::write(&file, ann.send_note(&note.id, None)?)?;

    let desk = store(1, "Ada")?;
    let mut app = App::new(desk, "en");
    app.select(Selection::Agenda);
    let handed = Arc::new(Mutex::new(Vec::new()));
    app.set_handed_files(handed.clone());
    let mut harness = window(app);

    drop_file(&mut harness, &file);
    assert!(harness.state().previewing());
    assert!(shows(&harness, "From Ann"));
    assert!(shows(
        &harness,
        &format!("Not verified: never met. Key {}", ann.signing_key()?.code())
    ));
    assert!(shows(&harness, "came unencrypted"));
    assert!(shows(&harness, "Blood test"));
    assert!(shows(&harness, "08:15"));
    harness.get_by_label("Decline").click_accesskit();
    harness.run();
    assert!(!harness.state().previewing());
    assert!(shows(&harness, "Declined: nothing was kept."));
    assert!(Notes::new(harness.state().store()).all()?.is_empty());

    // Handed over by `oldbrainfix open`: shown at the next frame.
    if let Ok(mut held) = handed.lock() {
        held.push(file.clone());
    }
    harness.run();
    assert!(harness.state().previewing());
    harness.get_by_label("Accept").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Added to Unsorted, with its alarms on."));
    assert!(shows(&harness, "1 Unsorted"), "the window reads it at once");
    let held = harness.state().store();
    let got = Notes::new(held).unsorted()?;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].text, "Blood test");
    let routines = Routines::new(held).of_note(&got[0].id)?;
    assert_eq!(routines.len(), 1);
    assert_eq!(routines[0].schedule, schedule);

    // Opened again: accepted before, nothing new.
    drop_file(&mut harness, &file);
    assert!(shows(&harness, "You accepted this before"));
    harness.get_by_label("Close").click_accesskit();
    harness.run();
    assert_eq!(Notes::new(harness.state().store()).all()?.len(), 1);

    // For someone else, or damaged.
    let ben = store(2, "Ben")?;
    meet(&ann, &ben, "Ben")?;
    let to = ann.sent_item_recipients()?;
    let other = dir.path().join("for-ben.oldbrain");
    std::fs::write(&other, ann.send_note(&note.id, to.first())?)?;
    drop_file(&mut harness, &other);
    assert!(shows(&harness, "encrypted for someone else"));
    let damaged = dir.path().join("damaged.oldbrain");
    std::fs::write(&damaged, b"{\"format\":1}")?;
    drop_file(&mut harness, &damaged);
    assert!(shows(&harness, "cannot be read"));
    Ok(())
}
